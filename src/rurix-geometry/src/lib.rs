//! rurix-geometry — Rurix 生态包第二梯队几何库(G1.4 / MR-0004，09 §5「网格/BVH 基础」)。
//!
//! **host 纯 safe 参考实现**，复用 [`spec/stdlib.md`](../../../spec/stdlib.md) 既有几何原语 /
//! 谓词语义 **0-byte**（RXS-0104~0113），并在其上做**纯编排**的 BVH / triangle mesh 基础——
//! 不新增任何公共语义面 / spec 条款（镜像 `uc03-demo`「工程编排不新增语义面」先例）：
//!
//! - [`Point3`] / [`Vector3`] / [`Normal3`] 语义区分与互转（RXS-0110）
//! - [`Aabb`] / [`Ray`] 类型与构造（RXS-0111）
//! - 几何谓词 [`Aabb::contains`] / [`Aabb::distance`]（RXS-0112）
//! - Ray–AABB slab 相交 [`Aabb::intersects`]（RXS-0113）
//! - [`Bvh`] 包围体层级 + [`Triangle`] 网格图元（纯 orchestration，复用上述谓词）
//!
//! 全 safe（`unsafe_code = "deny"`）、纯函数、确定性（IEEE-754 f32）；零外部依赖。

#![forbid(unsafe_code)]

// ———————————————————————— 几何向量类（RXS-0110） ————————————————————————

/// 空间点（位置）。与 [`Vector3`] / [`Normal3`] **语义互异**（RXS-0110）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// 自由向量（位移 / 方向）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// 法向量（归一化后的方向，承载法线语义）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Normal3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3 {
    /// 构造点（分量依序 x, y, z）。
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// `Point3 − Point3 → Vector3`（两点位移，RXS-0110）。
    ///
    /// 方法名 `sub` 刻意镜像 RXS-0110 谱面方法（`Point3.sub(Point3)`）；语义为
    /// **点减点产向量**（非 `std::ops::Sub` 的同型相减），故不实现 `Sub` trait。
    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, other: Point3) -> Vector3 {
        Vector3::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    /// `Point3 + Vector3 → Point3`（平移，RXS-0110）。
    pub fn offset(self, v: Vector3) -> Point3 {
        Point3::new(self.x + v.x, self.y + v.y, self.z + v.z)
    }

    /// `Point3 → Vector3`（位置向量，RXS-0110）。
    pub fn as_vector(self) -> Vector3 {
        Vector3::new(self.x, self.y, self.z)
    }
}

impl Vector3 {
    /// 构造向量（分量依序 x, y, z）。
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// 点积。
    pub fn dot(self, other: Vector3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// 叉积（右手系）。
    pub fn cross(self, other: Vector3) -> Vector3 {
        Vector3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// 欧氏范数 `sqrt(x²+y²+z²)`。
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// `Vector3 → Normal3`（归一化，RXS-0110）。
    ///
    /// **边界（RXS-0110）**：当 `length == 0.0` 时产**零** `Normal3`（各分量 `+0.0`），
    /// 不除零、不产生未定义行为；否则产 `Normal3(x/L, y/L, z/L)`。确定性返回值。
    pub fn to_normal(self) -> Normal3 {
        let l = self.length();
        if l == 0.0 {
            Normal3::new(0.0, 0.0, 0.0)
        } else {
            Normal3::new(self.x / l, self.y / l, self.z / l)
        }
    }
}

impl Normal3 {
    /// 构造法向量（不强制归一化；归一化经 [`Vector3::to_normal`]）。
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// `Normal3 → Vector3`（RXS-0110）。
    pub fn to_vector(self) -> Vector3 {
        Vector3::new(self.x, self.y, self.z)
    }
}

// ———————————————————————— Aabb / Ray（RXS-0111） ————————————————————————

/// 轴对齐包围盒（下角 `min` / 上角 `max`，RXS-0111）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Point3,
    pub max: Point3,
}

/// 射线（起点 `origin` + 方向 `dir`，RXS-0111）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Point3,
    pub dir: Vector3,
}

impl Aabb {
    /// 构造 AABB（`min` 为下角、`max` 为上角；不强制校验排序，RXS-0111）。
    pub fn new(min: Point3, max: Point3) -> Self {
        Self { min, max }
    }

    /// 退化 AABB：单点（min == max == p），用于 BVH 叶或点图元。
    pub fn point(p: Point3) -> Self {
        Self { min: p, max: p }
    }

    /// 两盒并（包住 self 与 other 的最小 AABB）。BVH 内部节点 bounds 用。
    pub fn union(self, other: Aabb) -> Aabb {
        Aabb::new(
            Point3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            Point3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        )
    }

    /// 形心（各轴中点）。BVH 划分轴选择用。
    pub fn centroid(self) -> Point3 {
        Point3::new(
            0.5 * (self.min.x + self.max.x),
            0.5 * (self.min.y + self.max.y),
            0.5 * (self.min.z + self.max.z),
        )
    }

    /// `Aabb.contains(Point3) → bool`（逐轴 `min ≤ p ≤ max` 合取，RXS-0112）。
    pub fn contains(self, p: Point3) -> bool {
        self.min.x <= p.x
            && p.x <= self.max.x
            && self.min.y <= p.y
            && p.y <= self.max.y
            && self.min.z <= p.z
            && p.z <= self.max.z
    }

    /// `Aabb.distance(Point3) → f32`（点到盒最短距离；点在盒内 → `+0.0`，RXS-0112）。
    pub fn distance(self, p: Point3) -> f32 {
        let dx = (self.min.x - p.x).max(0.0) + (p.x - self.max.x).max(0.0);
        let dy = (self.min.y - p.y).max(0.0) + (p.y - self.max.y).max(0.0);
        let dz = (self.min.z - p.z).max(0.0) + (p.z - self.max.z).max(0.0);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// `Aabb.intersects(Ray) → bool`（slab 法，RXS-0113）。
    ///
    /// 命中当且仅当 `t_exit ≥ t_enter` 且 `t_exit ≥ 0`（射线非负参数区间命中盒）。
    /// 轴平行退化（`dir_axis == 0`）：原点在该轴 slab 内则该轴不约束、在外则判否——
    /// 显式分支保证确定性、无 `0·∞` NaN，结果与 RXS-0113 IEEE 语义一致。
    pub fn intersects(self, ray: Ray) -> bool {
        let mut t_enter = 0.0f32; // 射线非负参数起点（蕴含 t_exit ≥ 0 约束）
        let mut t_exit = f32::INFINITY;
        let axes = [
            (ray.origin.x, ray.dir.x, self.min.x, self.max.x),
            (ray.origin.y, ray.dir.y, self.min.y, self.max.y),
            (ray.origin.z, ray.dir.z, self.min.z, self.max.z),
        ];
        for (o, d, lo, hi) in axes {
            if d == 0.0 {
                if o < lo || o > hi {
                    return false;
                }
            } else {
                let inv = 1.0 / d;
                let mut t1 = (lo - o) * inv;
                let mut t2 = (hi - o) * inv;
                if t1 > t2 {
                    core::mem::swap(&mut t1, &mut t2);
                }
                t_enter = t_enter.max(t1);
                t_exit = t_exit.min(t2);
            }
        }
        t_exit >= t_enter
    }
}

// ———————————————————————— Triangle 网格图元（编排，复用 RXS-0111） ————————————————————————

/// 三角形网格图元（三顶点）。包围盒经 [`Triangle::aabb`] 投影到 [`Aabb`]（RXS-0111），
/// 供 [`Bvh`] 加速结构编排（09 §5「网格基础」）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub a: Point3,
    pub b: Point3,
    pub c: Point3,
}

impl Triangle {
    /// 构造三角形。
    pub fn new(a: Point3, b: Point3, c: Point3) -> Self {
        Self { a, b, c }
    }

    /// 三角形的轴对齐包围盒（RXS-0111）。
    pub fn aabb(self) -> Aabb {
        Aabb::new(
            Point3::new(
                self.a.x.min(self.b.x).min(self.c.x),
                self.a.y.min(self.b.y).min(self.c.y),
                self.a.z.min(self.b.z).min(self.c.z),
            ),
            Point3::new(
                self.a.x.max(self.b.x).max(self.c.x),
                self.a.y.max(self.b.y).max(self.c.y),
                self.a.z.max(self.b.z).max(self.c.z),
            ),
        )
    }
}

// ———————————————————————— BVH 包围体层级（编排，复用 RXS-0113） ————————————————————————

const MAX_LEAF_PRIMS: usize = 2;

#[derive(Debug, Clone, Copy)]
enum NodeKind {
    Leaf { start: usize, count: usize },
    Internal { left: usize, right: usize },
}

/// BVH 节点（包围盒 + 叶/内部分支）。叶节点 ray 测试委托 [`Aabb::intersects`]（RXS-0113）。
#[derive(Debug, Clone, Copy)]
pub struct BvhNode {
    pub bounds: Aabb,
    kind: NodeKind,
}

/// 包围体层级（BVH）——09 §5「网格/BVH 基础」。
///
/// 在一组图元 AABB（[`Aabb`]，RXS-0111）上做**确定性** median-split 构建，ray 查询经
/// [`Aabb::intersects`] slab 谓词（RXS-0113）剪枝。纯编排、零新语义面。
#[derive(Debug, Clone)]
pub struct Bvh {
    nodes: Vec<BvhNode>,
    order: Vec<usize>,
    prims: Vec<Aabb>,
}

impl Bvh {
    /// 在图元 AABB 列表上构建 BVH（确定性：按形心在最长轴 median 划分）。
    pub fn build(prims: Vec<Aabb>) -> Self {
        let mut order: Vec<usize> = (0..prims.len()).collect();
        let mut nodes: Vec<BvhNode> = Vec::new();
        if !prims.is_empty() {
            let n = prims.len();
            build_recursive(&prims, &mut order, 0, n, &mut nodes);
        }
        Self {
            nodes,
            order,
            prims,
        }
    }

    /// 从三角形网格构建 BVH（图元 = 各三角形包围盒，RXS-0111）。
    pub fn from_triangles(tris: &[Triangle]) -> Self {
        Self::build(tris.iter().map(|t| t.aabb()).collect())
    }

    /// 图元数。
    pub fn len(&self) -> usize {
        self.prims.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.prims.is_empty()
    }

    /// 整棵树的根包围盒（空树为 `None`）。
    pub fn bounds(&self) -> Option<Aabb> {
        self.nodes.first().map(|n| n.bounds)
    }

    /// 返回被射线命中的图元下标（升序去重；命中测试 = [`Aabb::intersects`]，RXS-0113）。
    pub fn query_ray(&self, ray: Ray) -> Vec<usize> {
        let mut hits: Vec<usize> = Vec::new();
        if !self.nodes.is_empty() {
            self.query_node(0, ray, &mut hits);
        }
        hits.sort_unstable();
        hits
    }

    /// 射线是否命中任一图元（短路）。
    pub fn intersects_ray(&self, ray: Ray) -> bool {
        !self.nodes.is_empty() && self.any_node(0, ray)
    }

    fn query_node(&self, idx: usize, ray: Ray, hits: &mut Vec<usize>) {
        let node = self.nodes[idx];
        if !node.bounds.intersects(ray) {
            return;
        }
        match node.kind {
            NodeKind::Leaf { start, count } => {
                for &p in &self.order[start..start + count] {
                    if self.prims[p].intersects(ray) {
                        hits.push(p);
                    }
                }
            }
            NodeKind::Internal { left, right } => {
                self.query_node(left, ray, hits);
                self.query_node(right, ray, hits);
            }
        }
    }

    fn any_node(&self, idx: usize, ray: Ray) -> bool {
        let node = self.nodes[idx];
        if !node.bounds.intersects(ray) {
            return false;
        }
        match node.kind {
            NodeKind::Leaf { start, count } => self.order[start..start + count]
                .iter()
                .any(|&p| self.prims[p].intersects(ray)),
            NodeKind::Internal { left, right } => {
                self.any_node(left, ray) || self.any_node(right, ray)
            }
        }
    }
}

/// 在 `order[start..end]` 上递归构建子树，返回节点下标。
fn build_recursive(
    prims: &[Aabb],
    order: &mut [usize],
    start: usize,
    end: usize,
    nodes: &mut Vec<BvhNode>,
) -> usize {
    let bounds = order[start..end]
        .iter()
        .map(|&i| prims[i])
        .reduce(Aabb::union)
        .expect("非空范围");
    let node_idx = nodes.len();
    nodes.push(BvhNode {
        bounds,
        kind: NodeKind::Leaf {
            start,
            count: end - start,
        },
    });

    if end - start <= MAX_LEAF_PRIMS {
        return node_idx;
    }

    // 划分轴 = 形心包围盒最长轴。
    let centroid_bounds = order[start..end]
        .iter()
        .map(|&i| Aabb::point(prims[i].centroid()))
        .reduce(Aabb::union)
        .expect("非空范围");
    let ext = centroid_bounds.max.sub(centroid_bounds.min);
    let axis = if ext.x >= ext.y && ext.x >= ext.z {
        0
    } else if ext.y >= ext.z {
        1
    } else {
        2
    };

    let key = |i: usize| -> f32 {
        let c = prims[i].centroid();
        match axis {
            0 => c.x,
            1 => c.y,
            _ => c.z,
        }
    };
    order[start..end].sort_by(|&a, &b| {
        key(a)
            .partial_cmp(&key(b))
            .unwrap_or(core::cmp::Ordering::Equal)
    });

    let mid = start + (end - start) / 2;
    let left = build_recursive(prims, order, start, mid, nodes);
    let right = build_recursive(prims, order, mid, end, nodes);
    nodes[node_idx].kind = NodeKind::Internal { left, right };
    node_idx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5
    }

    //@ spec: RXS-0110 几何向量类语义区分与互转(Point/Vector/Normal)
    #[test]
    fn geom_vec_conversions() {
        let p = Point3::new(1.0, 2.0, 3.0);
        let q = Point3::new(4.0, 6.0, 3.0);
        // Point3 − Point3 → Vector3
        let d = q.sub(p);
        assert_eq!(d, Vector3::new(3.0, 4.0, 0.0));
        // Point3 + Vector3 → Point3
        assert_eq!(p.offset(d), Point3::new(4.0, 6.0, 3.0));
        // Point3 → Vector3
        assert_eq!(p.as_vector(), Vector3::new(1.0, 2.0, 3.0));
        // Vector3 → Normal3（归一化）
        let n = d.to_normal();
        assert!(approx(n.x, 0.6) && approx(n.y, 0.8) && approx(n.z, 0.0));
        // Normal3 → Vector3
        assert_eq!(n.to_vector(), Vector3::new(n.x, n.y, n.z));
    }

    //@ spec: RXS-0110 零向量归一化确定性边界(产零 Normal3,不除零)
    #[test]
    fn zero_vector_normalizes_to_zero_normal() {
        let z = Vector3::new(0.0, 0.0, 0.0).to_normal();
        assert_eq!(z, Normal3::new(0.0, 0.0, 0.0));
    }

    //@ spec: RXS-0111 AABB / Ray 类型与构造
    #[test]
    fn aabb_ray_construction() {
        let bb = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        assert_eq!(bb.min, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(bb.max, Point3::new(2.0, 2.0, 2.0));
        let r = Ray {
            origin: Point3::new(-1.0, 1.0, 1.0),
            dir: Vector3::new(1.0, 0.0, 0.0),
        };
        assert_eq!(r.origin.x, -1.0);
        assert_eq!(r.dir, Vector3::new(1.0, 0.0, 0.0));
    }

    //@ spec: RXS-0112 几何谓词:Point∈AABB 包含与点到 AABB 距离
    #[test]
    fn aabb_contains_and_distance() {
        let bb = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        // 内部点:contains 真,distance == +0.0
        assert!(bb.contains(Point3::new(1.0, 1.0, 1.0)));
        assert!(approx(bb.distance(Point3::new(1.0, 1.0, 1.0)), 0.0));
        // 外部点:contains 假,distance = 逐轴溢出欧氏范数
        let outside = Point3::new(5.0, 2.0, 2.0);
        assert!(!bb.contains(outside));
        assert!(approx(bb.distance(outside), 3.0));
        // 角外点:sqrt(3²+3²+0²)
        assert!(approx(
            bb.distance(Point3::new(5.0, 5.0, 2.0)),
            (18.0f32).sqrt()
        ));
    }

    //@ spec: RXS-0113 几何谓词:Ray–AABB 相交(slab 法)
    #[test]
    fn ray_aabb_intersection() {
        let bb = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        // 命中:从盒外沿 +x 穿过
        let hit = Ray {
            origin: Point3::new(-1.0, 1.0, 1.0),
            dir: Vector3::new(1.0, 0.0, 0.0),
        };
        assert!(bb.intersects(hit));
        // 不命中:平行错过(y 轴在盒外,dir 沿 x)
        let miss = Ray {
            origin: Point3::new(-1.0, 5.0, 1.0),
            dir: Vector3::new(1.0, 0.0, 0.0),
        };
        assert!(!bb.intersects(miss));
        // 背向:盒在射线反方向 → 不命中(t_exit < 0)
        let behind = Ray {
            origin: Point3::new(5.0, 1.0, 1.0),
            dir: Vector3::new(1.0, 0.0, 0.0),
        };
        assert!(!bb.intersects(behind));
        // 原点在盒内 → 命中
        let inside = Ray {
            origin: Point3::new(1.0, 1.0, 1.0),
            dir: Vector3::new(0.0, 1.0, 0.0),
        };
        assert!(bb.intersects(inside));
    }

    //@ spec: RXS-0111 Triangle 包围盒投影(网格图元编排)
    #[test]
    fn triangle_aabb() {
        let t = Triangle::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(0.0, 4.0, 1.0),
        );
        let bb = t.aabb();
        assert_eq!(bb.min, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(bb.max, Point3::new(3.0, 4.0, 1.0));
    }

    //@ spec: RXS-0113 BVH ray 查询经 Aabb::intersects 谓词(纯编排,确定性)
    #[test]
    fn bvh_query_ray_hits_expected_prims() {
        // 三个分离的单位盒沿 x 轴排开。
        let prims = vec![
            Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0)),
            Aabb::new(Point3::new(5.0, 0.0, 0.0), Point3::new(6.0, 1.0, 1.0)),
            Aabb::new(Point3::new(10.0, 0.0, 0.0), Point3::new(11.0, 1.0, 1.0)),
        ];
        let bvh = Bvh::build(prims);
        assert_eq!(bvh.len(), 3);
        // 沿 +x 穿过全部三盒(y=z=0.5)。
        let through = Ray {
            origin: Point3::new(-1.0, 0.5, 0.5),
            dir: Vector3::new(1.0, 0.0, 0.0),
        };
        assert_eq!(bvh.query_ray(through), vec![0, 1, 2]);
        assert!(bvh.intersects_ray(through));
        // 高于全部盒(y=5)→ 无命中。
        let over = Ray {
            origin: Point3::new(-1.0, 5.0, 0.5),
            dir: Vector3::new(1.0, 0.0, 0.0),
        };
        assert!(bvh.query_ray(over).is_empty());
        assert!(!bvh.intersects_ray(over));
    }

    //@ spec: RXS-0113 BVH 确定性:构建顺序无关于查询结果
    #[test]
    fn bvh_query_is_order_independent() {
        let a = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Point3::new(5.0, 0.0, 0.0), Point3::new(6.0, 1.0, 1.0));
        let ray = Ray {
            origin: Point3::new(-1.0, 0.5, 0.5),
            dir: Vector3::new(1.0, 0.0, 0.0),
        };
        let bvh1 = Bvh::build(vec![a, b]);
        let bvh2 = Bvh::build(vec![b, a]);
        // query_ray 返回排序后的全局图元下标，结果对插入顺序稳定。
        assert_eq!(bvh1.query_ray(ray), vec![0, 1]);
        assert_eq!(bvh2.query_ray(ray), vec![0, 1]);
    }

    #[test]
    fn empty_bvh_is_inert() {
        let bvh = Bvh::build(Vec::new());
        assert!(bvh.is_empty());
        assert_eq!(bvh.bounds(), None);
        let ray = Ray {
            origin: Point3::new(0.0, 0.0, 0.0),
            dir: Vector3::new(1.0, 0.0, 0.0),
        };
        assert!(bvh.query_ray(ray).is_empty());
        assert!(!bvh.intersects_ray(ray));
    }
}
