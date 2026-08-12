//! CPU 参照剔除器(报告1 §3.2 运行时可见性的 host 镜像;**W2 GPU 剔除
//! device 对拍的金标准**——报告1 §6.4「与 CPU 蛮力逐簇对拍,逐簇一致」)。
//!
//! 逐簇三关(与 GPU 剔除 pass 一一对应,接口冻结):
//!   1) 视锥:Gribb-Hartmann 6 平面 × 包围球;
//!   2) 背面锥:`dot(view, axis) ≥ cutoff` 剔(契约 `cone_cutoff` 注释口径;
//!      `cutoff = 1` 禁用;view = 相机→簇心单位向量);
//!   3) LOD cut:误差球投影判据 `error/dist × 屏幕系数 ≤ 阈` 且父层 `> 阈`
//!      (报告1 §3.2「自身误差不可感知且父级可感知」,阈值默认 1px)。
//!
//! LOD cut 性质(证明依赖 dag 模块误差单调不变量):沿任意叶→根链 error 单调
//! 不减、根 parent_error = +∞,故链上恰有一点满足 `error ≤ t < parent_error`;
//! 同组簇共享 error/parent_error ⇒ 选择以组为单位一致 ⇒ 选中集的叶覆盖 =
//! 全网格恰好一次(无重叠无空洞;`lod_cut_coverage_exact` 单测实证)。

use rurix_render::graph::types::ClusterRecord;

use crate::vecmath::{vdist, vdot, vnorm, vsub};

/// 极简 4×4 矩阵(行主序,列向量约定 `v' = M·v`;零外部依赖纪律)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4(pub [[f32; 4]; 4]);

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    pub fn transform4(&self, v: [f32; 4]) -> [f32; 4] {
        let mut out = [0.0f32; 4];
        for (r, o) in out.iter_mut().enumerate() {
            *o = (0..4).map(|k| self.0[r][k] * v[k]).sum();
        }
        out
    }

    /// 右手系透视(OpenGL 风格 NDC z∈[-1,1];参照/单测用,与后端无关)。
    pub fn perspective_rh(fovy_radians: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        let f = 1.0 / (fovy_radians * 0.5).tan();
        let mut m = [[0.0f32; 4]; 4];
        m[0][0] = f / aspect;
        m[1][1] = f;
        m[2][2] = (far + near) / (near - far);
        m[2][3] = 2.0 * far * near / (near - far);
        m[3][2] = -1.0;
        Mat4(m)
    }

    /// 右手系 look-at(eye 望向 target,up 近似上向)。
    pub fn look_at_rh(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> Mat4 {
        let z = vnorm(vsub(eye, target)).expect("eye 与 target 重合");
        let x = vnorm([
            up[1] * z[2] - up[2] * z[1],
            up[2] * z[0] - up[0] * z[2],
            up[0] * z[1] - up[1] * z[0],
        ])
        .expect("up 与视线共线");
        let y = [
            z[1] * x[2] - z[2] * x[1],
            z[2] * x[0] - z[0] * x[2],
            z[0] * x[1] - z[1] * x[0],
        ];
        Mat4([
            [x[0], x[1], x[2], -vdot(x, eye)],
            [y[0], y[1], y[2], -vdot(y, eye)],
            [z[0], z[1], z[2], -vdot(z, eye)],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Gribb-Hartmann 平面提取(行向量组合;顺序:左/右/下/上/近/远),归一化。
    pub fn frustum_planes(&self) -> [[f32; 4]; 6] {
        let m = &self.0;
        let comb = |sign: f32, row: usize| {
            [
                m[3][0] + sign * m[row][0],
                m[3][1] + sign * m[row][1],
                m[3][2] + sign * m[row][2],
                m[3][3] + sign * m[row][3],
            ]
        };
        let mut planes = [
            comb(1.0, 0),
            comb(-1.0, 0),
            comb(1.0, 1),
            comb(-1.0, 1),
            comb(1.0, 2),
            comb(-1.0, 2),
        ];
        for p in &mut planes {
            let l = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            if l > 1e-12 {
                for x in p.iter_mut() {
                    *x /= l;
                }
            }
        }
        planes
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Mat4;

    fn mul(self, rhs: Mat4) -> Mat4 {
        let mut out = [[0.0f32; 4]; 4];
        for (r, row) in out.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                *cell = (0..4).map(|k| self.0[r][k] * rhs.0[k][c]).sum();
            }
        }
        Mat4(out)
    }
}

/// 剔除视图参数(屏幕高 + 投影第二对角元 → 屏幕系数;阈值默认 1px)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CullView {
    /// `proj · view` 复合矩阵(视锥提取用)。
    pub view_proj: Mat4,
    pub camera_pos: [f32; 3],
    pub screen_height: f32,
    /// `proj[1][1] = 1/tan(fovy/2)`(误差球投影的屏幕系数)。
    pub proj_m11: f32,
    /// 屏幕误差阈值(像素;报告1 §3.2 默认 1px)。
    pub threshold_px: f32,
}

impl CullView {
    pub fn new(view: Mat4, proj: Mat4, camera_pos: [f32; 3], screen_height: f32) -> Self {
        Self {
            view_proj: proj * view,
            camera_pos,
            screen_height,
            proj_m11: proj.0[1][1],
            threshold_px: 1.0,
        }
    }

    pub fn with_threshold(mut self, threshold_px: f32) -> Self {
        self.threshold_px = threshold_px;
        self
    }

    /// 对象空间误差 → 屏幕像素(error/dist × 屏幕系数;dist 取到球面最近
    /// 距离并钳下界,零距离处误差保守视为巨大)。
    pub fn projected_error(&self, error: f32, center: [f32; 3], radius: f32) -> f32 {
        let d = (vdist(center, self.camera_pos) - radius).max(1e-3);
        error * (self.screen_height * self.proj_m11 * 0.5) / d
    }

    /// 视锥 6 平面-包围球:任一平面外侧即剔(保守 = 相交即留)。
    pub fn sphere_outside_frustum(&self, center: [f32; 3], radius: f32) -> bool {
        self.view_proj
            .frustum_planes()
            .iter()
            .any(|p| p[0] * center[0] + p[1] * center[1] + p[2] * center[2] + p[3] < -radius)
    }

    /// 背面锥:`dot(view, axis) ≥ cutoff` 整簇背面剔除(契约注释口径);
    /// `cutoff ≥ 1` 为禁用哨兵(退化簇)。
    pub fn cone_culled(&self, cone_axis: [f32; 3], cone_cutoff: f32, center: [f32; 3]) -> bool {
        if cone_cutoff >= 1.0 {
            return false;
        }
        let Some(view) = vnorm(vsub(center, self.camera_pos)) else {
            return false; // 相机在簇心:保守不剔
        };
        vdot(view, cone_axis) >= cone_cutoff
    }

    /// 纯 LOD cut 谓词(误差球投影 ≤ 阈 且父层 > 阈;DAG 切性质见模块文档)。
    pub fn lod_selected(&self, r: &ClusterRecord) -> bool {
        self.projected_error(r.error, r.center, r.radius) <= self.threshold_px
            && self.projected_error(r.parent_error, r.center, r.radius) > self.threshold_px
    }
}

/// 剔除统计(device 对拍时逐计数比对)。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CullStats {
    pub total: u32,
    pub frustum_culled: u32,
    pub cone_culled: u32,
    /// LOD cut 未选中(非「剔除」:其区域由父/子层代表)。
    pub lod_skipped: u32,
    pub visible: u32,
}

/// 逐簇蛮力剔除(GPU 对拍金标准;输出可见簇 id,顺序 = 输入序,稳定)。
pub fn cull_clusters(records: &[ClusterRecord], view: &CullView) -> (Vec<u32>, CullStats) {
    let mut stats = CullStats {
        total: records.len() as u32,
        ..CullStats::default()
    };
    let mut visible = Vec::new();
    for (i, r) in records.iter().enumerate() {
        if view.sphere_outside_frustum(r.center, r.radius) {
            stats.frustum_culled += 1;
            continue;
        }
        if view.cone_culled(r.cone_axis, r.cone_cutoff, r.center) {
            stats.cone_culled += 1;
            continue;
        }
        if !view.lod_selected(r) {
            stats.lod_skipped += 1;
            continue;
        }
        visible.push(i as u32);
    }
    stats.visible = visible.len() as u32;
    (visible, stats)
}

/// 纯 LOD cut 选择(不含视锥/锥;DAG 切覆盖性验证与 GPU LOD 判定对拍用)。
pub fn lod_cut_select(records: &[ClusterRecord], view: &CullView) -> Vec<u32> {
    records
        .iter()
        .enumerate()
        .filter(|(_, r)| view.lod_selected(r))
        .map(|(i, _)| i as u32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{ClusterDag, build_dag};
    use crate::mesh::TriMesh;
    use std::f32::consts::FRAC_PI_3;

    fn front_view(eye_z: f32, far: f32) -> CullView {
        let eye = [0.0, 0.0, eye_z];
        let view = Mat4::look_at_rh(eye, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let proj = Mat4::perspective_rh(FRAC_PI_3, 1.0, 0.1, far);
        CullView::new(view, proj, eye, 1080.0)
    }

    #[test]
    fn mat4_look_at_perspective_center() {
        let v = front_view(5.0, 100.0);
        let ndc = v.view_proj.transform4([0.0, 0.0, 0.0, 1.0]);
        let (x, y, z) = (ndc[0] / ndc[3], ndc[1] / ndc[3], ndc[2] / ndc[3]);
        assert!(x.abs() < 1e-6 && y.abs() < 1e-6, "原点未投影到画面中心");
        assert!(z > -1.0 && z < 1.0, "原点深度越出 NDC");
    }

    #[test]
    fn frustum_culls_clusters_behind() {
        let dag = build_dag(&TriMesh::plane_grid(4, 1.0));
        // 相机背对网格(望 +z,网格在 z=0 身后)→ 全部视锥剔除。
        let eye = [0.0, 0.0, 3.0];
        let view = Mat4::look_at_rh(eye, [0.0, 0.0, 10.0], [0.0, 1.0, 0.0]);
        let proj = Mat4::perspective_rh(FRAC_PI_3, 1.0, 0.1, 100.0);
        let cv = CullView::new(view, proj, eye, 1080.0);
        let (visible, stats) = cull_clusters(&dag.records, &cv);
        assert!(visible.is_empty());
        assert_eq!(stats.frustum_culled, stats.total);
    }

    #[test]
    fn cone_culls_backfacing_view() {
        let dag = build_dag(&TriMesh::plane_grid(4, 1.0));
        // 正面(+z 望向原点):法线朝向相机,锥放行。
        let (visible, stats) = cull_clusters(&dag.records, &front_view(3.0, 100.0));
        assert!(!visible.is_empty());
        assert_eq!(stats.cone_culled, 0);
        // 背面(-z 望向原点):view 与 +z 锥轴同向,dot=1 ≥ cutoff → 全剔。
        let eye = [0.0, 0.0, -3.0];
        let view = Mat4::look_at_rh(eye, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let proj = Mat4::perspective_rh(FRAC_PI_3, 1.0, 0.1, 100.0);
        let cv = CullView::new(view, proj, eye, 1080.0);
        let (visible, stats) = cull_clusters(&dag.records, &cv);
        assert!(visible.is_empty());
        assert_eq!(stats.cone_culled, stats.total);
    }

    #[test]
    fn all_visible_equals_leaf_set_at_zero_threshold() {
        // 阈 = 0:仅 error=0 的叶满足 error ≤ 0;父层 error > 0 不可达
        // ⇒ 可见集恰 = 全叶集(DAG cut 的极端情形)。
        let dag = build_dag(&TriMesh::plane_grid(8, 1.0));
        let cv = front_view(3.0, 100.0).with_threshold(0.0);
        let (visible, _) = cull_clusters(&dag.records, &cv);
        let leaves: Vec<u32> = dag.leaf_ids().collect();
        assert_eq!(visible, leaves);
    }

    #[test]
    fn lod_cut_near_selects_fine_far_selects_coarse() {
        let dag = build_dag(&TriMesh::plane_grid(8, 1.0));
        let top = dag.level_count() - 1;
        // 近距:父组误差投影 ≫1px → 至少部分叶被选中。
        let near = lod_cut_select(&dag.records, &front_view(2.0, 100.0));
        assert!(
            near.iter().any(|&id| dag.node(id).level == 0),
            "近距未选中任何叶簇"
        );
        // 远距(误差投影 ≪1px):仅顶层(根)可选。
        let far = lod_cut_select(&dag.records, &front_view(5000.0, 20000.0));
        assert!(!far.is_empty());
        assert!(
            far.iter().all(|&id| dag.node(id).level as usize == top),
            "远距选中了非顶层簇"
        );
    }

    #[test]
    fn lod_cut_coverage_exact() {
        // DAG cut 性质:任意阈值下,选中集的叶覆盖 = 全网格恰好一次。
        for mesh in [TriMesh::plane_grid(8, 1.0), TriMesh::uv_sphere(1.0, 16, 16)] {
            let dag: ClusterDag = build_dag(&mesh);
            let mut leaves: Vec<u32> = dag.leaf_ids().collect();
            leaves.sort_unstable();
            for &(eye_z, t) in &[
                (1.2f32, 0.25f32),
                (3.0, 1.0),
                (12.0, 1.0),
                (60.0, 4.0),
                (500.0, 8.0),
            ] {
                let cv = front_view(eye_z, 2000.0).with_threshold(t);
                let selected = lod_cut_select(&dag.records, &cv);
                assert!(!selected.is_empty(), "阈值 {t} 下 cut 为空");
                let mut expanded = dag.expand_to_leaves(&selected);
                expanded.sort_unstable();
                expanded.dedup();
                assert_eq!(expanded, leaves, "阈值 {t} 下叶覆盖有洞或有重叠");
            }
        }
    }

    //@ spec: RXS-0350
    #[test]
    fn lod_cut_select_reference_passes_runtime_coverage_verifier() {
        // G9.3 M93 对拍(RXS-0350 L2):离线参照 `lod_cut_select` 的输出必须
        // 通过 rurix-render 运行时覆盖性机器核验器(无重叠无空洞);真实 builder
        // 产物 DAG(plane_grid / uv_sphere)× 相机阈值扫描。
        use rurix_render::geometry::visible_cluster_set::{
            DagNodeRec, MeshDagView, verify_cut_coverage,
        };
        for mesh in [TriMesh::plane_grid(8, 1.0), TriMesh::uv_sphere(1.0, 16, 16)] {
            let dag: ClusterDag = build_dag(&mesh);
            let nodes: Vec<DagNodeRec> = dag
                .nodes
                .iter()
                .map(|n| DagNodeRec {
                    first_child: n.first_child,
                    child_count: n.child_count,
                    level: n.level,
                })
                .collect();
            let view = MeshDagView::new(&dag.records, &nodes, &dag.children).expect("拓扑合法");
            for &(eye_z, t) in &[
                (1.2f32, 0.25f32),
                (3.0, 1.0),
                (12.0, 1.0),
                (60.0, 4.0),
                (500.0, 8.0),
            ] {
                let cv = front_view(eye_z, 2000.0).with_threshold(t);
                let selected = lod_cut_select(&dag.records, &cv);
                verify_cut_coverage(&view, &selected)
                    .unwrap_or_else(|e| panic!("阈值 {t} 参照 cut 覆盖性破坏:{e}"));
                // 空洞注入(摘除首个选中簇及其**别名类**——组级共享子链接下
                // 展开相交的同选簇一并摘除)⇒ 运行时核验器必须判 RED(负例臂独立)。
                let victim_leaves = view.expand_to_leaves(&[selected[0]]);
                let holed: Vec<u32> = selected
                    .iter()
                    .copied()
                    .filter(|&s| {
                        view.expand_to_leaves(&[s])
                            .iter()
                            .all(|l| !victim_leaves.contains(l))
                    })
                    .collect();
                assert!(
                    matches!(
                        verify_cut_coverage(&view, &holed),
                        Err(rurix_render::geometry::visible_cluster_set::CutCoverageError::Hole { .. })
                    ),
                    "阈值 {t} 空洞注入未被检出"
                );
            }
        }
    }
}
