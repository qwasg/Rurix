//! meshlet 化(报告1 §3.1;簇生成两原则引 JCGT 2023:顶点局部性优先、簇形
//! 紧致利于剔除)。
//!
//! 贪心邻接生长:从种子三角形出发,每步在候选前沿中挑「与簇共享顶点最多、
//! 质心离簇中心最近」的面加入,直至触及契约上限(≤128 三角形 / ≤64 顶点,
//! 上限单源 = rurix-render graph::types)。簇内顶点重映射为 u8 局部索引。

use rurix_render::graph::types::{MAX_TRIS_PER_CLUSTER, MAX_VERTS_PER_CLUSTER};
use std::collections::HashMap;

use crate::mesh::{TriMesh, build_face_adjacency};
use crate::vecmath::{vcross, vdist, vdot, vlen, vsub};

/// 每簇三角形上限(冻结契约转引,见 [`crate::dag`] 导出)。
pub const MAX_TRIS: usize = MAX_TRIS_PER_CLUSTER as usize;
/// 每簇顶点上限(冻结契约转引;顶点 u8 局部化的依据)。
pub const MAX_VERTS: usize = MAX_VERTS_PER_CLUSTER as usize;

/// 簇(meshlet)构建期表示;DAG 完成后由 [`crate::dag`] 汇总导出为契约记录。
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// 局部顶点位置(≤64)。
    pub verts: Vec<[f32; 3]>,
    /// 局部三角形(3×u8 局部索引,≤128)。
    pub tris: Vec<[u8; 3]>,
    /// 源网格三角形 id(叶覆盖不变量用)。
    pub source_tris: Vec<u32>,
    /// 包围球中心(对象空间)。
    pub center: [f32; 3],
    /// 包围球半径(Ritter 近似,保守方向 = 宁大勿小)。
    pub radius: f32,
    /// 背面锥轴(单位向量;面积加权平均法线)。
    pub cone_axis: [f32; 3],
    /// 背面锥剔除阈值(meshopt 口径;1.0 = 退化簇禁用锥剔除)。
    pub cone_cutoff: f32,
}

/// 簇化中间结果(保留全局顶点 id,供 DAG 层内合并/焊接)。
pub(crate) struct RawCluster {
    /// 输入三角形 id 列表(本层三角形集合的划分元素)。
    pub tris: Vec<u32>,
    /// 局部 → 全局顶点。
    pub verts: Vec<u32>,
    /// 局部三角形(3×u8)。
    pub local: Vec<[u8; 3]>,
}

/// 单簇生长状态(贪心主循环)。
struct Grow<'a> {
    tris: &'a [[u32; 3]],
    adj: &'a [Vec<u32>],
    used: &'a mut [bool],
    map: HashMap<u32, u8>,
    verts: Vec<u32>,
    local: Vec<[u8; 3]>,
    faces: Vec<u32>,
    in_front: Vec<bool>,
    front: Vec<u32>,
}

impl<'a> Grow<'a> {
    /// 面 f 加入簇是否会越上限。
    fn fits(&self, f: u32) -> bool {
        if self.faces.len() >= MAX_TRIS {
            return false;
        }
        let new_verts = self.tris[f as usize]
            .iter()
            .filter(|v| !self.map.contains_key(*v))
            .count();
        self.verts.len() + new_verts <= MAX_VERTS
    }

    fn add(&mut self, f: u32) {
        let t = self.tris[f as usize];
        let mut lt = [0u8; 3];
        for (k, &v) in t.iter().enumerate() {
            let idx = match self.map.get(&v) {
                Some(&i) => i,
                None => {
                    let i = self.verts.len() as u8;
                    self.verts.push(v);
                    self.map.insert(v, i);
                    i
                }
            };
            lt[k] = idx;
        }
        self.local.push(lt);
        self.faces.push(f);
        self.used[f as usize] = true;
        for &nb in &self.adj[f as usize] {
            if !self.used[nb as usize] && !self.in_front[nb as usize] {
                self.in_front[nb as usize] = true;
                self.front.push(nb);
            }
        }
    }
}

/// 贪心邻接生长簇化(报告1 §3.1 meshopt 分簇思路的 Rust 实现)。
///
/// 评分:`共享顶点数 - 面质心到簇中心距离 / (簇半径 + ε)`——共享顶点主导
/// (顶点局部性),距离惩罚促簇形紧致。同分取前沿先入者,全确定性。
pub(crate) fn clusterize_tris(positions: &[[f32; 3]], tris: &[[u32; 3]]) -> Vec<RawCluster> {
    let n = tris.len();
    let adj = build_face_adjacency(tris);
    let centroids: Vec<[f32; 3]> = tris
        .iter()
        .map(|t| {
            let (a, b, c) = (
                positions[t[0] as usize],
                positions[t[1] as usize],
                positions[t[2] as usize],
            );
            [
                (a[0] + b[0] + c[0]) / 3.0,
                (a[1] + b[1] + c[1]) / 3.0,
                (a[2] + b[2] + c[2]) / 3.0,
            ]
        })
        .collect();
    let mut used = vec![false; n];
    let mut out = Vec::new();
    for seed in 0..n {
        if used[seed] {
            continue;
        }
        let mut g = Grow {
            tris,
            adj: &adj,
            used: &mut used,
            map: HashMap::new(),
            verts: Vec::new(),
            local: Vec::new(),
            faces: Vec::new(),
            in_front: vec![false; n],
            front: Vec::new(),
        };
        g.add(seed as u32);
        loop {
            // 当前簇均值中心与半径(评分参考系;≤64 顶点,逐次重算开销可忽略)。
            let mut center = [0.0f32; 3];
            for &v in &g.verts {
                let p = positions[v as usize];
                center = [center[0] + p[0], center[1] + p[1], center[2] + p[2]];
            }
            let inv = 1.0 / g.verts.len() as f32;
            center = [center[0] * inv, center[1] * inv, center[2] * inv];
            let radius = g
                .verts
                .iter()
                .map(|&v| vdist(positions[v as usize], center))
                .fold(0.0f32, f32::max);
            let mut best: Option<(usize, f32)> = None; // (前沿下标, 评分)
            for (fi, &f) in g.front.iter().enumerate() {
                if g.used[f as usize] || !g.fits(f) {
                    continue;
                }
                let shared = tris[f as usize]
                    .iter()
                    .filter(|v| g.map.contains_key(*v))
                    .count() as f32;
                let score = shared - vdist(centroids[f as usize], center) / (radius + 1e-6);
                if best.is_none_or(|(_, s)| score > s) {
                    best = Some((fi, score));
                }
            }
            let Some((fi, _)) = best else { break };
            let f = g.front[fi];
            g.front.swap_remove(fi);
            g.add(f);
        }
        out.push(RawCluster {
            tris: g.faces,
            verts: g.verts,
            local: g.local,
        });
    }
    out
}

/// 网格 → 带包围球/背面锥的完整簇集(报告1 §5 P0 验收:100% 输入可转换)。
pub fn clusterize(mesh: &TriMesh) -> Vec<Cluster> {
    let tris = mesh.triangles();
    clusterize_tris(&mesh.positions, &tris)
        .into_iter()
        .map(|rc| {
            let verts: Vec<[f32; 3]> = rc
                .verts
                .iter()
                .map(|&v| mesh.positions[v as usize])
                .collect();
            let (center, radius) = bounding_sphere(&verts);
            let cluster_tris: Vec<[u32; 3]> = rc.tris.iter().map(|&f| tris[f as usize]).collect();
            let (cone_axis, cone_cutoff) = backface_cone(&mesh.positions, &cluster_tris);
            Cluster {
                verts,
                tris: rc.local,
                source_tris: rc.tris,
                center,
                radius,
                cone_axis,
                cone_cutoff,
            }
        })
        .collect()
}

/// Ritter 近似包围球:三轴极值点对选最大距者定初始球,再逐点扩张。
/// 比最优球大 ~20% 以内;剔除用途取保守方向(宁大勿小),满足
/// 「球包含全部簇顶点」不变量。
pub(crate) fn bounding_sphere(points: &[[f32; 3]]) -> ([f32; 3], f32) {
    if points.is_empty() {
        return ([0.0; 3], 0.0);
    }
    let mut lo = [0usize; 3];
    let mut hi = [0usize; 3];
    for (i, p) in points.iter().enumerate() {
        for k in 0..3 {
            if p[k] < points[lo[k]][k] {
                lo[k] = i;
            }
            if p[k] > points[hi[k]][k] {
                hi[k] = i;
            }
        }
    }
    let mut pair = (lo[0], hi[0]);
    let mut best = 0.0f32;
    for k in 0..3 {
        let d = vdist(points[lo[k]], points[hi[k]]);
        if d > best {
            best = d;
            pair = (lo[k], hi[k]);
        }
    }
    let (a, b) = (points[pair.0], points[pair.1]);
    let mut center = [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
    ];
    let mut radius = best * 0.5;
    for &p in points {
        let d = vdist(p, center);
        if d > radius {
            let nr = (radius + d) * 0.5;
            let dir = vsub(p, center);
            center = [
                center[0] + dir[0] * (nr - radius) / d,
                center[1] + dir[1] * (nr - radius) / d,
                center[2] + dir[2] * (nr - radius) / d,
            ];
            radius = nr;
        }
    }
    (center, radius)
}

/// 背面锥(meshopt `meshopt_buildMeshletsBound` 口径;契约 `cone_cutoff` 注释):
/// 轴 = 面积加权平均法线;`cutoff = sin(轴与各面法线最大夹角)`(几何推导:
/// `dot(view, axis) ≥ sin(α)` 时所有面法线与 view 夹角 ≤90°,整簇背面)。
/// 保守收紧:mindp 减 1e-4 使 cutoff 略增(宁留勿错剔)。
/// `mindp ≤ 0`(开放/包围形簇,法线四散)→ cutoff = 1 禁用锥剔除。
pub(crate) fn backface_cone(positions: &[[f32; 3]], tris: &[[u32; 3]]) -> ([f32; 3], f32) {
    let mut axis = [0.0f32; 3];
    for t in tris {
        let (a, b, c) = (
            positions[t[0] as usize],
            positions[t[1] as usize],
            positions[t[2] as usize],
        );
        let n = vcross(vsub(b, a), vsub(c, a)); // 未归一化:模长 = 2×面积 → 面积加权
        axis = [axis[0] + n[0], axis[1] + n[1], axis[2] + n[2]];
    }
    let alen = vlen(axis);
    if alen <= 1e-12 {
        return ([0.0, 0.0, 1.0], 1.0); // 法线抵消(包围形簇)→ 禁用
    }
    let axis = [axis[0] / alen, axis[1] / alen, axis[2] / alen];
    let mut mindp = f32::MAX;
    let mut any = false;
    for t in tris {
        let (a, b, c) = (
            positions[t[0] as usize],
            positions[t[1] as usize],
            positions[t[2] as usize],
        );
        let n = vcross(vsub(b, a), vsub(c, a));
        let l = vlen(n);
        if l <= 1e-12 {
            continue; // 退化三角形不参与锥
        }
        mindp = mindp.min(vdot([n[0] / l, n[1] / l, n[2] / l], axis));
        any = true;
    }
    if !any || mindp <= 1e-6 {
        return (axis, 1.0);
    }
    let mc = (mindp - 1e-4).max(0.0);
    // 钳 < 1:cutoff 恰为 1 是「禁用」哨兵,合法锥须可区分。
    let cutoff = (1.0 - mc * mc).sqrt().min(1.0 - 1e-6);
    (axis, cutoff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vecmath::vdot as dot;

    #[test]
    fn covers_every_triangle_once() {
        for m in [TriMesh::plane_grid(8, 1.0), TriMesh::uv_sphere(1.0, 16, 16)] {
            let clusters = clusterize(&m);
            let mut seen = vec![false; m.triangle_count()];
            for c in &clusters {
                for &f in &c.source_tris {
                    assert!(!seen[f as usize], "三角形 {f} 被覆盖两次");
                    seen[f as usize] = true;
                }
            }
            assert!(seen.iter().all(|&s| s), "存在未覆盖三角形");
        }
    }

    #[test]
    fn limits_respected() {
        for m in [
            TriMesh::uv_sphere(1.0, 32, 32),
            TriMesh::plane_grid(16, 1.0),
        ] {
            for c in clusterize(&m) {
                assert!(c.tris.len() <= MAX_TRIS, "簇三角形数越限");
                assert!(c.verts.len() <= MAX_VERTS, "簇顶点数越限");
                for t in &c.tris {
                    assert!(
                        t.iter()
                            .all(|&i| usize::from(i) < c.verts.len() && usize::from(i) < MAX_VERTS)
                    );
                }
                assert_eq!(c.tris.len(), c.source_tris.len());
            }
        }
    }

    #[test]
    fn sphere_contains_all_vertices() {
        let m = TriMesh::uv_sphere(1.0, 16, 16);
        for c in clusterize(&m) {
            for v in &c.verts {
                let d = vdist(*v, c.center);
                assert!(d <= c.radius + 1e-4, "顶点越出包围球:d={d} r={}", c.radius);
            }
        }
    }

    #[test]
    fn cone_plane_cluster_tight() {
        // 平面整体一簇:轴 ≈ +z,cutoff ≈ 0(任何非背面视角可见)。
        let m = TriMesh::plane_grid(4, 1.0);
        let clusters = clusterize(&m);
        assert_eq!(clusters.len(), 1);
        let c = &clusters[0];
        assert!(dot(c.cone_axis, [0.0, 0.0, 1.0]) > 0.99, "锥轴偏离 +z");
        assert!(c.cone_cutoff < 0.05, "平面簇 cutoff 应接近 0");
    }

    #[test]
    fn cone_degenerate_cube_disabled() {
        // 立方体整体一簇(12 三角形 8 顶点):法线六向抵消 → 开放簇禁用锥剔除。
        let m = TriMesh::cube(1.0);
        let clusters = clusterize(&m);
        assert_eq!(clusters.len(), 1);
        assert_eq!(
            clusters[0].cone_cutoff.to_bits(),
            1.0f32.to_bits(),
            "退化簇 cutoff 须为 1(禁用)"
        );
    }
}
