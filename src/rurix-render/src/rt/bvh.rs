//! 最小 BVH(rt 内部几何内核;报告4 §2.2 AS 底座,RFC-0016 章 F)。
//!
//! 三角形 BVH(中位数切分 SAH 简化版)+ 两级 TLAS 遍历(实例逆变换光线,t 参数
//! 仿射不变)。本文件是 W2 device ray query 效果的**对拍金标准几何内核**:host 与
//! device 必须在同一几何输入上产同一命中集合(容差 = 浮点结合序)。
//!
//! 确定性承诺:构建(稳定排序 median split)、遍历(固定左右序)、相交(Möller–Trumbore)
//! 全部单线程定序,同输入同输出(IEEE-754 f32);零 unsafe、零外部依赖。
//! 风格参照 `src/rurix-geometry/src/lib.rs`(RXS-0111~0113)但独立实现,不 import
//! rurix-geometry(rt 模块自含,避免新增 crate 依赖)。

use core::fmt;
use core::ops::{Add, Index, Mul, Neg, Sub};

// ---------------------------------------------------------------------------
// 基础数学(Vec3 / Aabb / Ray / Transform3x4)
// ---------------------------------------------------------------------------

/// 三维向量(host 参考数学;`[f32; 3]` 数组语义的命名载体)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    /// 零向量。
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);

    /// 构造向量(分量依序 x, y, z)。
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// 从数组构造(分量依序 x, y, z)。
    pub fn from_array(a: [f32; 3]) -> Self {
        Self::new(a[0], a[1], a[2])
    }

    /// 转为数组。
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    /// 点积。
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    /// 叉积(右手系)。
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    /// 欧氏范数。
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// 归一化。**边界**:零向量归一化产零向量(不除零,确定性)。
    pub fn normalize(self) -> Vec3 {
        let l = self.length();
        if l == 0.0 {
            Vec3::ZERO
        } else {
            self * (1.0 / l)
        }
    }

    /// 三分量全部有限(非 NaN 非 ±inf)。
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// 逐分量最小。
    pub fn min(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x.min(o.x), self.y.min(o.y), self.z.min(o.z))
    }

    /// 逐分量最大。
    pub fn max(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x.max(o.x), self.y.max(o.y), self.z.max(o.z))
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

impl Index<usize> for Vec3 {
    type Output = f32;
    fn index(&self, axis: usize) -> &f32 {
        match axis {
            0 => &self.x,
            1 => &self.y,
            _ => &self.z,
        }
    }
}

/// 轴对齐包围盒(下角 `min` / 上角 `max`)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// 原点退化盒(禁用实例占位用;永不产生真实命中)。
    pub const ZERO: Aabb = Aabb::new(Vec3::ZERO, Vec3::ZERO);

    /// 构造 AABB(不强制校验排序)。
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// 三角形包围盒。
    pub fn from_triangle(a: Vec3, b: Vec3, c: Vec3) -> Self {
        Aabb::new(a.min(b).min(c), a.max(b).max(c))
    }

    /// 两盒并(包住 self 与 other 的最小 AABB)。
    pub fn union(self, o: Aabb) -> Aabb {
        Aabb::new(self.min.min(o.min), self.max.max(o.max))
    }

    /// 形心(各轴中点)。
    pub fn centroid(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// 各轴尺寸。
    pub fn extent(self) -> Vec3 {
        self.max - self.min
    }

    /// 表面积(SAH 质量度量;退化盒产 0)。refit 膨胀比的计算基准。
    pub fn surface_area(self) -> f32 {
        let e = self.extent();
        2.0 * (e.x * e.y + e.y * e.z + e.z * e.x)
    }
}

/// 射线(起点 + 方向;方向**不**强制单位长——t 为参数距离,单位长时即世界距离)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

/// 3×4 仿射变换(行主序 `[r0c0 r0c1 r0c2 t0, r1c0 …, r2c0 …]`;实例对象空间 →
/// 世界空间)。法线变换经 [`Transform3x4::transpose_apply`](逆矩阵转置作用)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3x4 {
    pub m: [f32; 12],
}

impl Transform3x4 {
    /// 恒等变换。
    pub const IDENTITY: Transform3x4 = Transform3x4 {
        m: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
    };

    /// 一般构造(行主序 12 元素)。
    pub const fn from_rows(m: [f32; 12]) -> Self {
        Self { m }
    }

    /// 纯平移。
    pub fn from_translation(t: [f32; 3]) -> Self {
        Self {
            m: [
                1.0, 0.0, 0.0, t[0], 0.0, 1.0, 0.0, t[1], 0.0, 0.0, 1.0, t[2],
            ],
        }
    }

    /// 作用于点(含平移)。
    pub fn apply_point(&self, p: Vec3) -> Vec3 {
        let m = &self.m;
        Vec3::new(
            m[0] * p.x + m[1] * p.y + m[2] * p.z + m[3],
            m[4] * p.x + m[5] * p.y + m[6] * p.z + m[7],
            m[8] * p.x + m[9] * p.y + m[10] * p.z + m[11],
        )
    }

    /// 作用于方向向量(不含平移)。**不归一化**——保持 t 参数仿射不变
    /// (`M⁻¹(o + t·d) = M⁻¹o + t·M⁻¹d`,对象空间命中 t 即世界空间 t)。
    pub fn apply_vector(&self, v: Vec3) -> Vec3 {
        let m = &self.m;
        Vec3::new(
            m[0] * v.x + m[1] * v.y + m[2] * v.z,
            m[4] * v.x + m[5] * v.y + m[6] * v.z,
            m[8] * v.x + m[9] * v.y + m[10] * v.z,
        )
    }

    /// 转置作用(`M³ˣ³ᵀ·v`;法线世界化 = `inverse.transpose_apply(n).normalize()`)。
    pub fn transpose_apply(&self, v: Vec3) -> Vec3 {
        let m = &self.m;
        Vec3::new(
            m[0] * v.x + m[4] * v.y + m[8] * v.z,
            m[1] * v.x + m[5] * v.y + m[9] * v.z,
            m[2] * v.x + m[6] * v.y + m[10] * v.z,
        )
    }

    /// 一般仿射逆(3×3 伴随矩阵 + 平移回代)。`|det| ≤ 1e-12` 判退化产 `None`
    /// (host 参考模型的保守阈值;此类实例在 [`Tlas::build`] 中确定性禁用)。
    pub fn inverse(&self) -> Option<Transform3x4> {
        let m = &self.m;
        let (a, b, c) = (m[0], m[1], m[2]);
        let (d, e, f) = (m[4], m[5], m[6]);
        let (g, h, i) = (m[8], m[9], m[10]);
        let ca = e * i - f * h;
        let cb = -(d * i - f * g);
        let cc = d * h - e * g;
        let det = a * ca + b * cb + c * cc;
        if det.abs() <= 1e-12 {
            return None;
        }
        let inv_det = 1.0 / det;
        let r = [
            ca * inv_det,
            (c * h - b * i) * inv_det,
            (b * f - c * e) * inv_det,
            cb * inv_det,
            (a * i - c * g) * inv_det,
            (c * d - a * f) * inv_det,
            cc * inv_det,
            (b * g - a * h) * inv_det,
            (a * e - b * d) * inv_det,
        ];
        let t = Vec3::new(m[3], m[7], m[11]);
        let it = Vec3::new(
            -(r[0] * t.x + r[1] * t.y + r[2] * t.z),
            -(r[3] * t.x + r[4] * t.y + r[5] * t.z),
            -(r[6] * t.x + r[7] * t.y + r[8] * t.z),
        );
        Some(Transform3x4 {
            m: [
                r[0], r[1], r[2], it.x, r[3], r[4], r[5], it.y, r[6], r[7], r[8], it.z,
            ],
        })
    }
}

/// 变换后的 AABB(8 角点变换再取并)。
pub fn transform_aabb(t: &Transform3x4, bb: Aabb) -> Aabb {
    let mut out: Option<Aabb> = None;
    for i in 0..8 {
        let p = Vec3::new(
            if i & 1 == 0 { bb.min.x } else { bb.max.x },
            if i & 2 == 0 { bb.min.y } else { bb.max.y },
            if i & 4 == 0 { bb.min.z } else { bb.max.z },
        );
        let wp = t.apply_point(p);
        let wb = Aabb::new(wp, wp);
        out = Some(match out {
            None => wb,
            Some(u) => u.union(wb),
        });
    }
    out.unwrap_or(Aabb::ZERO)
}

// ---------------------------------------------------------------------------
// 命中记录与三角形相交(Möller–Trumbore)
// ---------------------------------------------------------------------------

/// `TriBvh::intersect` 直接命中时 [`Hit::instance`] 的哨兵值(非 TLAS 上下文无实例)。
pub const NO_INSTANCE: u32 = u32::MAX;

/// 命中记录(closest-hit 查询返回值;**对拍契约字段**)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// 世界空间参数距离(`p(t) = origin + t·dir`;实例空间共享同一 t——仿射不变)。
    pub t: f32,
    /// 命中三角形在所属 BLAS 内的索引。
    pub tri: u32,
    /// 命中实例在 [`Tlas`] 实例列表的下标;`TriBvh` 直接查询为 [`NO_INSTANCE`]。
    pub instance: u32,
    /// 重心坐标 `(u, v)`:`p = a·(1−u−v) + b·u + c·v`。
    pub bary: [f32; 2],
    /// 世界空间几何法线(单位长;朝向依三角形 winding,不做 front-face 翻转)。
    pub normal: [f32; 3],
}

/// 命中接受区间下界(严格大于;配合调用方 origin bias 排除自交)。
const T_MIN: f32 = 0.0;

/// Ray–AABB slab 测试(区间 `[t_min, t_max]`;轴平行退化显式分支,无 `0·∞` NaN)。
fn slab_intersect(bounds: &Aabb, ray: &Ray, t_min: f32, t_max: f32) -> bool {
    let mut enter = t_min;
    let mut exit = t_max;
    for axis in 0..3 {
        let o = ray.origin[axis];
        let d = ray.dir[axis];
        let lo = bounds.min[axis];
        let hi = bounds.max[axis];
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
            enter = enter.max(t1);
            exit = exit.min(t2);
            if enter > exit {
                return false;
            }
        }
    }
    true
}

/// Ray–三角形相交(Möller–Trumbore,双面)。命中区间 `(T_MIN, t_max)` 开区间;
/// 返回 `(t, u, v)`。`det ≈ 0`(平行/退化三角形)判 miss,确定性无 NaN。
fn intersect_triangle(ray: &Ray, a: Vec3, b: Vec3, c: Vec3, t_max: f32) -> Option<(f32, f32, f32)> {
    let e1 = b - a;
    let e2 = c - a;
    let p = ray.dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-8 {
        return None;
    }
    let inv_det = 1.0 / det;
    let s = ray.origin - a;
    let u = s.dot(p) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = ray.dir.dot(q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv_det;
    if t > T_MIN && t < t_max {
        Some((t, u, v))
    } else {
        None
    }
}

/// 三角形几何法线(依 winding,归一化;零面积产零向量,确定性)。
fn face_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    (b - a).cross(c - a).normalize()
}

// ---------------------------------------------------------------------------
// BVH 节点与 median-split 构建(中位数切分 SAH 简化版)
// ---------------------------------------------------------------------------

/// 叶节点最大图元数(host 参考取值;遍历深度 ~log₂(n/4))。
const MAX_LEAF_PRIMS: usize = 4;

#[derive(Debug, Clone, Copy)]
enum NodeKind {
    Leaf { start: u32, count: u32 },
    Internal { left: u32, right: u32 },
}

#[derive(Debug, Clone, Copy)]
struct Node {
    bounds: Aabb,
    kind: NodeKind,
}

/// 在一组图元 AABB 上做确定性 median-split 构建(最长形心轴 + 稳定排序中点切分,
/// SAH 简化版)。返回(节点数组, 图元排列);空输入产空节点数组。
/// 不变量:子孙节点下标恒大于祖先(先序推入),refit 逆序遍历即自底向上。
fn build_median_bvh(prims: &[Aabb]) -> (Vec<Node>, Vec<u32>) {
    let mut order: Vec<u32> = (0..prims.len() as u32).collect();
    let mut nodes: Vec<Node> = Vec::new();
    if !prims.is_empty() {
        build_recursive(prims, &mut order, 0, prims.len(), &mut nodes);
    }
    (nodes, order)
}

fn build_recursive(
    prims: &[Aabb],
    order: &mut [u32],
    start: usize,
    end: usize,
    nodes: &mut Vec<Node>,
) -> u32 {
    let mut bounds = prims[order[start] as usize];
    for &pi in &order[start + 1..end] {
        bounds = bounds.union(prims[pi as usize]);
    }
    let idx = nodes.len() as u32;
    nodes.push(Node {
        bounds,
        kind: NodeKind::Leaf {
            start: start as u32,
            count: (end - start) as u32,
        },
    });

    if end - start <= MAX_LEAF_PRIMS {
        return idx;
    }

    // 划分轴 = 形心包围盒最长轴;稳定排序保证确定性(等值键保持原序)。
    let mut cmin = prims[order[start] as usize].centroid();
    let mut cmax = cmin;
    for &pi in &order[start + 1..end] {
        let c = prims[pi as usize].centroid();
        cmin = cmin.min(c);
        cmax = cmax.max(c);
    }
    let ext = cmax - cmin;
    let axis = if ext.x >= ext.y && ext.x >= ext.z {
        0
    } else if ext.y >= ext.z {
        1
    } else {
        2
    };
    let key = |pi: u32| prims[pi as usize].centroid()[axis];
    order[start..end].sort_by(|&a, &b| {
        key(a)
            .partial_cmp(&key(b))
            .unwrap_or(core::cmp::Ordering::Equal)
    });

    let mid = start + (end - start) / 2;
    let left = build_recursive(prims, order, start, mid, nodes);
    let right = build_recursive(prims, order, mid, end, nodes);
    nodes[idx as usize].kind = NodeKind::Internal { left, right };
    idx
}

// ---------------------------------------------------------------------------
// TriBvh:三角形 BVH(单网格 BLAS 几何内核)
// ---------------------------------------------------------------------------

/// refit 错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefitError {
    /// 新顶点数与构建时不符(拓扑变化必须走全量重建,不得 refit)。
    VertexCountMismatch { expected: usize, got: usize },
}

impl fmt::Display for RefitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            RefitError::VertexCountMismatch { expected, got } => {
                write!(f, "refit 顶点数失配:期望 {expected},实得 {got}")
            }
        }
    }
}

impl std::error::Error for RefitError {}

/// refit 重建建议阈值(报告4 §2.2 refit 分级口径):任一叶 AABB 表面积膨胀比
/// **严格大于**该值时建议全量重建(`RefitReport::needs_rebuild`)。
pub const REFIT_INFLATION_REBUILD_THRESHOLD: f32 = 2.0;

/// refit 报告(变形质量度量;as_manager 决策树的输入)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefitReport {
    /// 全部叶节点中「新表面积 / 初建表面积」的最大值(收缩不计入;基准 1.0 =
    /// 无膨胀;初建退化零面积叶膨胀后按 +∞ 计)。
    pub max_inflation: f32,
}

impl RefitReport {
    /// 变形超阈判据:任一叶 AABB 膨胀比 > [`REFIT_INFLATION_REBUILD_THRESHOLD`]。
    pub fn needs_rebuild(&self) -> bool {
        self.max_inflation > REFIT_INFLATION_REBUILD_THRESHOLD
    }
}

/// 三角形 BVH(单网格的 BLAS 几何内核)。持有顶点/索引拷贝:refit 只更新顶点
/// 位置与叶包围盒并向上传播,**不重构拓扑**(节点数组/排列不变)。
#[derive(Debug, Clone)]
pub struct TriBvh {
    positions: Vec<Vec3>,
    indices: Vec<[u32; 3]>,
    nodes: Vec<Node>,
    order: Vec<u32>,
    /// 各叶节点初建时的表面积(膨胀比基准;内部节点槽位为 0)。
    leaf_base_area: Vec<f32>,
}

impl TriBvh {
    /// 从位置/索引构建 BVH。
    ///
    /// # Panics
    /// 索引越界(≥ 顶点数)即 panic——构建期调用契约,网格数据必须先校验。
    pub fn build(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> Self {
        for t in indices {
            for &vi in t {
                assert!(
                    (vi as usize) < positions.len(),
                    "TriBvh::build: 索引 {vi} 越界(顶点数 {})",
                    positions.len()
                );
            }
        }
        let positions: Vec<Vec3> = positions.iter().map(|&p| Vec3::from_array(p)).collect();
        let prims: Vec<Aabb> = indices
            .iter()
            .map(|t| {
                let (a, b, c) = tri_corners(&positions, *t);
                Aabb::from_triangle(a, b, c)
            })
            .collect();
        let (nodes, order) = build_median_bvh(&prims);
        let leaf_base_area = nodes
            .iter()
            .map(|n| match n.kind {
                NodeKind::Leaf { .. } => n.bounds.surface_area(),
                NodeKind::Internal { .. } => 0.0,
            })
            .collect();
        Self {
            positions,
            indices: indices.to_vec(),
            nodes,
            order,
            leaf_base_area,
        }
    }

    /// 三角形数。
    pub fn triangle_count(&self) -> usize {
        self.indices.len()
    }

    /// 顶点数。
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// 是否为空(零三角形)。
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// 根包围盒(空树产 `None`)。
    pub fn bounds(&self) -> Option<Aabb> {
        self.nodes.first().map(|n| n.bounds)
    }

    /// 最近命中查询(全距离范围;等价 `intersect_within(ray, +∞)`)。
    pub fn intersect(&self, ray: &Ray) -> Option<Hit> {
        self.intersect_within(ray, f32::INFINITY)
    }

    /// 最近命中查询(限 `t < t_max`;TLAS 全局剪枝共用)。
    pub fn intersect_within(&self, ray: &Ray, t_max: f32) -> Option<Hit> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut best: Option<Hit> = None;
        let mut t_best = t_max;
        self.intersect_node(0, ray, &mut t_best, &mut best);
        best
    }

    /// 阴影/遮蔽查询:`(0, t_max)` 区间任一命中即早出(true)。
    pub fn any_hit(&self, ray: &Ray, t_max: f32) -> bool {
        !self.nodes.is_empty() && self.any_node(0, ray, t_max)
    }

    /// 动态 refit(报告4 §2.2:变形用 refit、拓扑变化用 rebuild)。以新顶点位置
    /// 更新各叶包围盒并自底向上传播,节点拓扑/排列不变;返回膨胀比报告。
    ///
    /// # Errors
    /// 顶点数与构建时不符(拓扑变化)产 [`RefitError::VertexCountMismatch`],
    /// 调用方必须改走全量重建。
    pub fn refit(&mut self, new_positions: &[[f32; 3]]) -> Result<RefitReport, RefitError> {
        if new_positions.len() != self.positions.len() {
            return Err(RefitError::VertexCountMismatch {
                expected: self.positions.len(),
                got: new_positions.len(),
            });
        }
        for (dst, &src) in self.positions.iter_mut().zip(new_positions) {
            *dst = Vec3::from_array(src);
        }
        let mut max_inflation = 1.0f32;
        // 子孙下标恒大于祖先(构建先序):逆序遍历即自底向上传播。
        for idx in (0..self.nodes.len()).rev() {
            let kind = self.nodes[idx].kind;
            let bounds = match kind {
                NodeKind::Leaf { start, count } => {
                    let mut bb: Option<Aabb> = None;
                    for k in start..start + count {
                        let (a, b, c) = self.tri_corners(self.order[k as usize]);
                        let tb = Aabb::from_triangle(a, b, c);
                        bb = Some(match bb {
                            None => tb,
                            Some(u) => u.union(tb),
                        });
                    }
                    let bb = bb.expect("叶节点图元数 ≥ 1(构建不变量)");
                    let area = bb.surface_area();
                    let base = self.leaf_base_area[idx];
                    let ratio = if base == 0.0 {
                        if area == 0.0 { 1.0 } else { f32::INFINITY }
                    } else {
                        area / base
                    };
                    max_inflation = max_inflation.max(ratio);
                    bb
                }
                NodeKind::Internal { left, right } => self.nodes[left as usize]
                    .bounds
                    .union(self.nodes[right as usize].bounds),
            };
            self.nodes[idx].bounds = bounds;
        }
        Ok(RefitReport { max_inflation })
    }

    fn tri_corners(&self, tri: u32) -> (Vec3, Vec3, Vec3) {
        tri_corners(&self.positions, self.indices[tri as usize])
    }

    fn intersect_node(&self, node: u32, ray: &Ray, t_best: &mut f32, best: &mut Option<Hit>) {
        let n = &self.nodes[node as usize];
        if !slab_intersect(&n.bounds, ray, T_MIN, *t_best) {
            return;
        }
        match n.kind {
            NodeKind::Leaf { start, count } => {
                for k in start..start + count {
                    let ti = self.order[k as usize];
                    let (a, b, c) = self.tri_corners(ti);
                    if let Some((t, u, v)) = intersect_triangle(ray, a, b, c, *t_best) {
                        *t_best = t;
                        *best = Some(Hit {
                            t,
                            tri: ti,
                            instance: NO_INSTANCE,
                            bary: [u, v],
                            normal: face_normal(a, b, c).to_array(),
                        });
                    }
                }
            }
            NodeKind::Internal { left, right } => {
                self.intersect_node(left, ray, t_best, best);
                self.intersect_node(right, ray, t_best, best);
            }
        }
    }

    fn any_node(&self, node: u32, ray: &Ray, t_max: f32) -> bool {
        let n = &self.nodes[node as usize];
        if !slab_intersect(&n.bounds, ray, T_MIN, t_max) {
            return false;
        }
        match n.kind {
            NodeKind::Leaf { start, count } => (start..start + count).any(|k| {
                let (a, b, c) = self.tri_corners(self.order[k as usize]);
                intersect_triangle(ray, a, b, c, t_max).is_some()
            }),
            NodeKind::Internal { left, right } => {
                self.any_node(left, ray, t_max) || self.any_node(right, ray, t_max)
            }
        }
    }
}

fn tri_corners(positions: &[Vec3], tri: [u32; 3]) -> (Vec3, Vec3, Vec3) {
    (
        positions[tri[0] as usize],
        positions[tri[1] as usize],
        positions[tri[2] as usize],
    )
}

// ---------------------------------------------------------------------------
// BlasSet:BLAS 集合抽象(TLAS 遍历对 BLAS 存储的解耦面)
// ---------------------------------------------------------------------------

/// BLAS 集合抽象:TLAS 实例以 `u32` 标识引用 BLAS,遍历经本 trait 取几何。
/// `&[TriBvh]` / `Vec<TriBvh>` 直接可用;as_manager 的 `BlasCache` 亦实现之
/// (失效/evict 槽位产 `None`,遍历确定性跳过)。
pub trait BlasSet {
    /// 按标识取 BLAS;不存在/已失效产 `None`。
    fn blas(&self, id: u32) -> Option<&TriBvh>;
}

impl BlasSet for [TriBvh] {
    fn blas(&self, id: u32) -> Option<&TriBvh> {
        self.get(id as usize)
    }
}

impl BlasSet for Vec<TriBvh> {
    fn blas(&self, id: u32) -> Option<&TriBvh> {
        self.as_slice().blas(id)
    }
}

// ---------------------------------------------------------------------------
// Tlas:实例级 BVH(两级遍历:实例逆变换光线 → BLAS 查询)
// ---------------------------------------------------------------------------

/// TLAS 实例描述(对象空间 BLAS + 仿射变换 + 掩码/旗标)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstanceDesc {
    /// BLAS 标识([`BlasSet`] 下标;as_manager 层为 `BlasId` 裸值)。
    pub blas: u32,
    /// 对象空间 → 世界空间仿射变换。
    pub transform: Transform3x4,
    /// 光线掩码(Vulkan `mask` 语义:`instance.mask & ray_mask ≠ 0` 才参与遍历;
    /// 0 = 实例对光线恒不可见)。
    pub mask: u8,
    /// 实例旗标(保留字段,与 device 腿 `VkGeometryInstanceFlagsKHR` 对齐留口;
    /// host 遍历不消费)。
    pub flags: u32,
}

#[derive(Debug, Clone, Copy)]
struct InstanceState {
    blas: u32,
    inverse: Transform3x4,
    mask: u8,
    /// 不可逆变换 / 失效 BLAS / 空 BLAS → false(永不命中,确定性跳过)。
    enabled: bool,
}

/// 实例级 BVH(TLAS)。实例数 <10k 时每帧全量快速重建即够(报告4 §2.2:
/// 亚毫秒级),故本 host 模型只提供 `build` 全量重建;增量标脏语义在
/// as_manager 的 `TlasBuilder`。
#[derive(Debug, Clone)]
pub struct Tlas {
    instances: Vec<InstanceState>,
    nodes: Vec<Node>,
    order: Vec<u32>,
}

impl Tlas {
    /// 以实例列表 + BLAS 集合构建 TLAS。禁用实例(不可逆变换/失效或空 BLAS)
    /// 保留槽位但永不命中;其世界 AABB 退化到原点(仅轻微放宽根盒,不影响正确性)。
    pub fn build<B: BlasSet + ?Sized>(instances: &[InstanceDesc], blases: &B) -> Tlas {
        let mut states = Vec::with_capacity(instances.len());
        let mut prims: Vec<Aabb> = Vec::with_capacity(instances.len());
        for desc in instances {
            let inverse = desc.transform.inverse();
            let root = blases.blas(desc.blas).and_then(|b| b.bounds());
            let (world, enabled) = match (inverse, root) {
                (Some(_), Some(root)) => (transform_aabb(&desc.transform, root), true),
                _ => (Aabb::ZERO, false),
            };
            states.push(InstanceState {
                blas: desc.blas,
                inverse: inverse.unwrap_or(Transform3x4::IDENTITY),
                mask: desc.mask,
                enabled,
            });
            prims.push(world);
        }
        let (nodes, order) = build_median_bvh(&prims);
        Tlas {
            instances: states,
            nodes,
            order,
        }
    }

    /// 实例槽位数(含禁用实例)。
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// 是否无实例。
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// 根包围盒(空 TLAS 产 `None`)。
    pub fn bounds(&self) -> Option<Aabb> {
        self.nodes.first().map(|n| n.bounds)
    }

    /// 最近命中查询(ray_mask = 0xFF,即实例 mask 非 0 即可见)。
    pub fn intersect<B: BlasSet + ?Sized>(&self, blases: &B, ray: &Ray) -> Option<Hit> {
        self.intersect_with_mask(blases, ray, 0xFF)
    }

    /// 最近命中查询(显式 ray_mask;`instance.mask & ray_mask ≠ 0` 的实例参与)。
    pub fn intersect_with_mask<B: BlasSet + ?Sized>(
        &self,
        blases: &B,
        ray: &Ray,
        ray_mask: u8,
    ) -> Option<Hit> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut best: Option<Hit> = None;
        let mut t_best = f32::INFINITY;
        self.intersect_node(blases, 0, ray, ray_mask, &mut t_best, &mut best);
        best
    }

    /// 阴影/遮蔽查询(ray_mask = 0xFF;`(0, t_max)` 任一命中即早出)。
    pub fn any_hit<B: BlasSet + ?Sized>(&self, blases: &B, ray: &Ray, t_max: f32) -> bool {
        self.any_hit_with_mask(blases, ray, 0xFF, t_max)
    }

    /// 阴影/遮蔽查询(显式 ray_mask)。
    pub fn any_hit_with_mask<B: BlasSet + ?Sized>(
        &self,
        blases: &B,
        ray: &Ray,
        ray_mask: u8,
        t_max: f32,
    ) -> bool {
        !self.nodes.is_empty() && self.any_node(blases, 0, ray, ray_mask, t_max)
    }

    fn intersect_node<B: BlasSet + ?Sized>(
        &self,
        blases: &B,
        node: u32,
        ray: &Ray,
        ray_mask: u8,
        t_best: &mut f32,
        best: &mut Option<Hit>,
    ) {
        let n = &self.nodes[node as usize];
        if !slab_intersect(&n.bounds, ray, T_MIN, *t_best) {
            return;
        }
        match n.kind {
            NodeKind::Leaf { start, count } => {
                for k in start..start + count {
                    let si = self.order[k as usize] as usize;
                    let st = &self.instances[si];
                    if !st.enabled || st.mask & ray_mask == 0 {
                        continue;
                    }
                    let Some(blas) = blases.blas(st.blas) else {
                        continue;
                    };
                    // 实例逆变换光线:方向不归一化,t 参数仿射不变(对象空间 t 即世界 t)。
                    let local = Ray {
                        origin: st.inverse.apply_point(ray.origin),
                        dir: st.inverse.apply_vector(ray.dir),
                    };
                    if let Some(mut hit) = blas.intersect_within(&local, *t_best) {
                        *t_best = hit.t;
                        hit.instance = si as u32;
                        // 法线经逆矩阵转置世界化并归一。
                        hit.normal = st
                            .inverse
                            .transpose_apply(Vec3::from_array(hit.normal))
                            .normalize()
                            .to_array();
                        *best = Some(hit);
                    }
                }
            }
            NodeKind::Internal { left, right } => {
                self.intersect_node(blases, left, ray, ray_mask, t_best, best);
                self.intersect_node(blases, right, ray, ray_mask, t_best, best);
            }
        }
    }

    fn any_node<B: BlasSet + ?Sized>(
        &self,
        blases: &B,
        node: u32,
        ray: &Ray,
        ray_mask: u8,
        t_max: f32,
    ) -> bool {
        let n = &self.nodes[node as usize];
        if !slab_intersect(&n.bounds, ray, T_MIN, t_max) {
            return false;
        }
        match n.kind {
            NodeKind::Leaf { start, count } => (start..start + count).any(|k| {
                let st = &self.instances[self.order[k as usize] as usize];
                if !st.enabled || st.mask & ray_mask == 0 {
                    return false;
                }
                let Some(blas) = blases.blas(st.blas) else {
                    return false;
                };
                let local = Ray {
                    origin: st.inverse.apply_point(ray.origin),
                    dir: st.inverse.apply_vector(ray.dir),
                };
                blas.any_hit(&local, t_max)
            }),
            NodeKind::Internal { left, right } => {
                self.any_node(blases, left, ray, ray_mask, t_max)
                    || self.any_node(blases, right, ray, ray_mask, t_max)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5
    }

    /// z=0 平面单三角形(x,y ∈ [0,1] 半区)。
    fn unit_triangle() -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        (
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0, 1, 2]],
        )
    }

    /// UV 球网格(顶点精确在球面上;弦面内陷 ≈ r·(1−cos(π/slices)))。
    fn uv_sphere(radius: f32, stacks: u32, slices: u32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        positions.push([0.0, radius, 0.0]); // 顶极点 idx 0
        for i in 1..stacks {
            let theta = core::f32::consts::PI * i as f32 / stacks as f32;
            let y = radius * theta.cos();
            let rr = radius * theta.sin();
            for j in 0..slices {
                let phi = 2.0 * core::f32::consts::PI * j as f32 / slices as f32;
                positions.push([rr * phi.cos(), y, rr * phi.sin()]);
            }
        }
        let bottom = positions.len() as u32;
        positions.push([0.0, -radius, 0.0]); // 底极点
        let ring = |i: u32, j: u32| 1 + (i - 1) * slices + (j % slices);
        let mut indices: Vec<[u32; 3]> = Vec::new();
        for j in 0..slices {
            indices.push([0, ring(1, j + 1), ring(1, j)]);
        }
        for i in 1..stacks - 1 {
            for j in 0..slices {
                let (a, b) = (ring(i, j), ring(i, j + 1));
                let (c, d) = (ring(i + 1, j), ring(i + 1, j + 1));
                indices.push([a, d, b]);
                indices.push([a, c, d]);
            }
        }
        for j in 0..slices {
            indices.push([bottom, ring(stacks - 1, j), ring(stacks - 1, j + 1)]);
        }
        (positions, indices)
    }

    #[test]
    fn single_triangle_hit_and_miss() {
        let (pos, idx) = unit_triangle();
        let bvh = TriBvh::build(&pos, &idx);
        assert_eq!(bvh.triangle_count(), 1);
        // 命中:自上而下穿过三角形内部。
        let ray = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        let hit = bvh.intersect(&ray).expect("应命中");
        assert!(approx(hit.t, 1.0));
        assert_eq!(hit.tri, 0);
        assert_eq!(hit.instance, NO_INSTANCE);
        assert!(approx(hit.bary[0], 0.25) && approx(hit.bary[1], 0.25));
        assert_eq!(hit.normal, [0.0, 0.0, 1.0]);
        // miss:射线在三角形外。
        let miss = Ray {
            origin: Vec3::new(2.0, 2.0, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert_eq!(bvh.intersect(&miss), None);
        assert!(!bvh.any_hit(&miss, f32::INFINITY));
        // miss:三角形在射线背后(t < 0 不接受)。
        let behind = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, 1.0),
        };
        assert_eq!(bvh.intersect(&behind), None);
    }

    #[test]
    fn any_hit_respects_t_max() {
        let (pos, idx) = unit_triangle();
        let bvh = TriBvh::build(&pos, &idx);
        let ray = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        // 命中 t = 1:t_max 开区间语义——t_max = 2 命中,t_max = 0.5 不命中。
        assert!(bvh.any_hit(&ray, 2.0));
        assert!(!bvh.any_hit(&ray, 0.5));
        assert_eq!(bvh.intersect_within(&ray, 0.5), None);
    }

    #[test]
    fn sphere_mesh_inside_out_hits_radius() {
        let (pos, idx) = uv_sphere(1.0, 16, 32);
        let bvh = TriBvh::build(&pos, &idx);
        assert!(!bvh.is_empty());
        // 自内向外:球心出发 8 个方向,命中距离 ≈ 半径(弦面内陷 < 2%)。
        let dirs = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0).normalize(),
            Vec3::new(-1.0, 1.0, -1.0).normalize(),
        ];
        for dir in dirs {
            let ray = Ray {
                origin: Vec3::ZERO,
                dir,
            };
            let hit = bvh.intersect(&ray).expect("球内向外必命中");
            assert!((hit.t - 1.0).abs() < 2e-2, "dir={dir:?} t={}", hit.t);
            assert!(approx(Vec3::from_array(hit.normal).length(), 1.0));
        }
    }

    #[test]
    fn empty_bvh_is_inert() {
        let bvh = TriBvh::build(&[], &[]);
        assert!(bvh.is_empty());
        assert_eq!(bvh.bounds(), None);
        let ray = Ray {
            origin: Vec3::ZERO,
            dir: Vec3::new(1.0, 0.0, 0.0),
        };
        assert_eq!(bvh.intersect(&ray), None);
        assert!(!bvh.any_hit(&ray, f32::INFINITY));
        // 空树 refit(零顶点)合法且平凡。
        let mut bvh = bvh;
        let report = bvh.refit(&[]).expect("空树 refit 合法");
        assert!(approx(report.max_inflation, 1.0));
        assert!(!report.needs_rebuild());
    }

    #[test]
    fn tlas_instance_translation_preserves_t() {
        let (pos, idx) = unit_triangle();
        let blas = TriBvh::build(&pos, &idx);
        let blases = vec![blas];
        // 实例平移 (+10, 0, 0):平移后的命中 t 与未平移相对位置一致。
        let tlas = Tlas::build(
            &[InstanceDesc {
                blas: 0,
                transform: Transform3x4::from_translation([10.0, 0.0, 0.0]),
                mask: 0xFF,
                flags: 0,
            }],
            &blases,
        );
        assert_eq!(tlas.instance_count(), 1);
        let hit_ray = Ray {
            origin: Vec3::new(10.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        let hit = tlas.intersect(&blases, &hit_ray).expect("平移后应命中");
        assert!(approx(hit.t, 1.0));
        assert_eq!(hit.instance, 0);
        assert_eq!(hit.tri, 0);
        assert_eq!(hit.normal, [0.0, 0.0, 1.0]);
        // 原对象空间位置不再命中。
        let miss_ray = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert_eq!(tlas.intersect(&blases, &miss_ray), None);
        assert!(tlas.any_hit(&blases, &hit_ray, 2.0));
        assert!(!tlas.any_hit(&blases, &miss_ray, f32::INFINITY));
    }

    #[test]
    fn tlas_mask_culls_instance() {
        let (pos, idx) = unit_triangle();
        let blases = vec![TriBvh::build(&pos, &idx)];
        let tlas = Tlas::build(
            &[InstanceDesc {
                blas: 0,
                transform: Transform3x4::IDENTITY,
                mask: 0,
                flags: 0,
            }],
            &blases,
        );
        let ray = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        // mask = 0:默认(0xFF)与显式 ray mask 均不可见。
        assert_eq!(tlas.intersect(&blases, &ray), None);
        assert!(!tlas.any_hit(&blases, &ray, f32::INFINITY));
        // mask = 0x01:仅 ray_mask 含 bit0 可见(Vulkan mask 语义)。
        let tlas = Tlas::build(
            &[InstanceDesc {
                blas: 0,
                transform: Transform3x4::IDENTITY,
                mask: 0x01,
                flags: 0,
            }],
            &blases,
        );
        assert!(tlas.intersect_with_mask(&blases, &ray, 0x01).is_some());
        assert_eq!(tlas.intersect_with_mask(&blases, &ray, 0x02), None);
    }

    #[test]
    fn tlas_two_instances_closest_hit() {
        let (pos, idx) = unit_triangle();
        let blases = vec![TriBvh::build(&pos, &idx)];
        // 实例 0 平移 (0,0,−2)(t=3),实例 1 平移 (0,0,−5)(t=6)。
        let tlas = Tlas::build(
            &[
                InstanceDesc {
                    blas: 0,
                    transform: Transform3x4::from_translation([0.0, 0.0, -2.0]),
                    mask: 0xFF,
                    flags: 0,
                },
                InstanceDesc {
                    blas: 0,
                    transform: Transform3x4::from_translation([0.0, 0.0, -5.0]),
                    mask: 0xFF,
                    flags: 0,
                },
            ],
            &blases,
        );
        let ray = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        let hit = tlas.intersect(&blases, &ray).expect("应命中近处实例");
        assert!(approx(hit.t, 3.0));
        assert_eq!(hit.instance, 0);
        // 反向:近处实例在背后(t<0),应命中远处实例 t=6 的反面?——反向两侧均无命中。
        let back = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, 1.0),
        };
        assert_eq!(tlas.intersect(&blases, &back), None);
    }

    #[test]
    fn degenerate_transform_instance_disabled() {
        let (pos, idx) = unit_triangle();
        let blases = vec![TriBvh::build(&pos, &idx)];
        // 零缩放变换:det = 0 → 不可逆 → 实例确定性禁用。
        let tlas = Tlas::build(
            &[InstanceDesc {
                blas: 0,
                transform: Transform3x4::from_rows([0.0; 12]),
                mask: 0xFF,
                flags: 0,
            }],
            &blases,
        );
        assert_eq!(tlas.instance_count(), 1);
        let ray = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert_eq!(tlas.intersect(&blases, &ray), None);
        assert!(!tlas.any_hit(&blases, &ray, f32::INFINITY));
    }

    #[test]
    fn refit_small_deformation_keeps_topology() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let indices = vec![[0, 1, 2], [0, 2, 3]];
        let mut bvh = TriBvh::build(&positions, &indices);
        // 小变形:v2 沿 +x 移 0.1(叶面积 2 → 2.2,膨胀比 1.1 < 2)。
        let deformed = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.1, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let report = bvh.refit(&deformed).expect("同顶点数 refit 合法");
        assert!(approx(report.max_inflation, 1.1));
        assert!(!report.needs_rebuild());
        // 新包围盒生效:旧位置外、新位置内的点(x=1.02, refit 前超界)现在命中。
        let ray = Ray {
            origin: Vec3::new(1.02, 0.5, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        let hit = bvh.intersect(&ray).expect("refit 后新位置应命中");
        assert!(approx(hit.t, 1.0));
    }

    #[test]
    fn refit_large_deformation_suggests_rebuild() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let indices = vec![[0, 1, 2], [0, 2, 3]];
        let mut bvh = TriBvh::build(&positions, &indices);
        // 大变形:整体 ×2(叶面积 ×4,膨胀比 4 > 2 → 建议全量重建)。
        let deformed = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
            [0.0, 2.0, 0.0],
        ];
        let report = bvh.refit(&deformed).expect("同顶点数 refit 合法");
        assert!(report.max_inflation > REFIT_INFLATION_REBUILD_THRESHOLD);
        assert!(report.needs_rebuild());
        // refit 本身仍正确(几何跟随新位置),只是 BVH 质量劣化。
        let ray = Ray {
            origin: Vec3::new(1.4, 0.5, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert!(bvh.intersect(&ray).is_some());
    }

    #[test]
    fn refit_rejects_vertex_count_mismatch() {
        let (pos, idx) = unit_triangle();
        let mut bvh = TriBvh::build(&pos, &idx);
        let err = bvh
            .refit(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]])
            .expect_err("顶点数失配必须拒绝");
        assert_eq!(
            err,
            RefitError::VertexCountMismatch {
                expected: 3,
                got: 2
            }
        );
    }
}
