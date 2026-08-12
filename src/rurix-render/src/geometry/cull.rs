//! 两级剔除 host 参考实现(报告1 §3.2/§6.2 P1;RFC-0016 §4.C2)——未来 GPU 剔除
//! kernel 的**金标准**,语义逐条对应 GPU 线程模型:
//!
//! | host 函数 | GPU pass(报告1 §6.2) | 线程映射 |
//! |---|---|---|
//! | [`instance_cull`] | `instance_cull` | 1 线程/实例 |
//! | [`cluster_cull`] | `cluster_cull`(LOD cut 判定并入) | 1 线程/簇(subgroup 压缩) |
//! | [`compact_draw_args`] | `compact_draw_args` | 单线程组前缀和 |
//!
//! 三关语义(与离线 `rurix-geom-build::cull_ref` 同口径,逐条对拍锚定):
//! 1. **视锥**:Gribb-Hartmann 自 view_proj 提取 6 平面;实例级 AABB 正顶点测试、
//!    簇级包围球球心距测试(保守:只剔确定不可见,绝不漏可见)。
//! 2. **背面锥**:冻结契约 `ClusterRecord::cone_cutoff` 注释口径——
//!    `dot(view, axis) ≥ cutoff` 整簇剔除,`cutoff ≥ 1` 禁用;`view` = 相机→簇心
//!    单位向量(锥顶点取包围球心,P0 无独立锥顶点字段;相机距球心 <1e-6 不剔)。
//! 3. **LOD cut**:自身误差投影 < 阈值 且 父级误差投影 ≥ 阈值(报告1 §3.2「自身
//!    不可感知且父级可感知」,恰构成 DAG 上一个 cut,无需簇间通信)。离线不变量
//!    (叶 error = 0、根 parent_error = +∞、链上单调)保证每链恰一点入选,
//!    `lod_cut_coverage_exact_hand_dag` 单测实证。
//!
//! 遮挡剔除(HZB 两阶段)按 RFC-0016 §9 Q-B 裁决**不入本期硬门**:本参考只承载
//! 单阶段三关,HZB 输入面待 W3 device 接线时预留。device 对拍口径 = 集合 + 计数
//! 一致;**顺序不锚定**(device 原子散射序与 host 稳定输入序天然不同)。
//!
//! 实例变换约定:刚体 + 均匀缩放(球半径/误差按 3×3 列范数最大者缩放,非均匀
//! 缩放下保守放大——投影误差偏大偏向选细层,安全方向;锥轴按 3×3 变换再归一化,
//! 非均匀缩放为近似,`gpu_scene` 语义面不承载剪切)。

use crate::graph::types::ClusterRecord;

use super::gpu_scene::{InstanceRecord, transform_point};

/// SW/HW 分箱默认阈值(屏幕像素;报告1 §3.3:Epic profile 调定的边长 ~32px 参照,
/// 小三角形走 SW compute 软光栅、大三角形走 HW 间接绘制)。
pub const DEFAULT_BIN_THRESHOLD_PX: f32 = 32.0;

// ---------------------------------------------------------------------------
// 视锥(Gribb-Hartmann 六平面提取)
// ---------------------------------------------------------------------------

/// 视锥(6 平面,内向单位法线;点 p 在锥内 ⟺ 全部 `dot(n, p) + d ≥ 0`)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    /// 平面顺序冻结:left/right/bottom/top/near/far(单测锚定)。
    pub planes: [[f32; 4]; 6],
}

impl Frustum {
    /// Gribb-Hartmann 提取(行主 M、列向量 `clip = M·v`、ZO 深度 [0,1] 口径:
    /// near = row2,far = row3 − row2;法线单位化使点-平面距离可读)。
    pub fn from_view_proj(m: &[[f32; 4]; 4]) -> Self {
        let combine = |a: usize, b: usize, sign: f32| {
            let mut p = [0.0f32; 4];
            for (k, c) in p.iter_mut().enumerate() {
                *c = m[a][k] + sign * m[b][k];
            }
            normalize_plane(p)
        };
        Self {
            planes: [
                combine(3, 0, 1.0),    // left:   x + w ≥ 0
                combine(3, 0, -1.0),   // right:  w − x ≥ 0
                combine(3, 1, 1.0),    // bottom: y + w ≥ 0
                combine(3, 1, -1.0),   // top:    w − y ≥ 0
                normalize_plane(m[2]), // near(ZO): z ≥ 0
                combine(3, 2, -1.0),   // far:    w − z ≥ 0
            ],
        }
    }

    /// 包围球对视锥(簇级;球心到任一平面距离 < −r 则整体在外)。
    pub fn contains_sphere(&self, center: [f32; 3], radius: f32) -> bool {
        self.planes
            .iter()
            .all(|p| p[0] * center[0] + p[1] * center[1] + p[2] * center[2] + p[3] >= -radius)
    }

    /// AABB 对视锥(实例级;逐平面取正顶点——沿法线最远的角,正顶点在外侧则
    /// 整体在外。保守:相交/跨界一律保留,不漏可见)。
    pub fn intersects_aabb(&self, lo: [f32; 3], hi: [f32; 3]) -> bool {
        self.planes.iter().all(|p| {
            let px = if p[0] >= 0.0 { hi[0] } else { lo[0] };
            let py = if p[1] >= 0.0 { hi[1] } else { lo[1] };
            let pz = if p[2] >= 0.0 { hi[2] } else { lo[2] };
            p[0] * px + p[1] * py + p[2] * pz + p[3] >= 0.0
        })
    }
}

fn normalize_plane(p: [f32; 4]) -> [f32; 4] {
    let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-12);
    [p[0] / len, p[1] / len, p[2] / len, p[3] / len]
}

// ---------------------------------------------------------------------------
// 剔除相机与误差投影
// ---------------------------------------------------------------------------

/// 剔除相机(剔除 pass 的 uniform 面;`view_proj` 行主、列向量约定,
/// 与 `temporal::common::perspective_rh_zo` 的 ZO [0,1] 深度口径一致)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CullCamera {
    /// 视图 × 投影(clip = M·(world, 1))。
    pub view_proj: [[f32; 4]; 4],
    /// 相机世界位置(锥剔 view 向量与误差径向距离源)。
    pub cam_pos: [f32; 3],
    /// 视口高(像素;误差投影屏幕系数源)。
    pub screen_height_px: f32,
    /// LOD 误差阈值(屏幕像素;报告1 默认 1px,`error ≤1px` 取本层)。
    pub error_threshold_px: f32,
}

impl CullCamera {
    /// 提取视锥。
    pub fn frustum(&self) -> Frustum {
        Frustum::from_view_proj(&self.view_proj)
    }

    /// 屏幕投影系数:m11·H/2(对称透视 m11 = cot(fov_y/2))。
    fn projection_factor(&self) -> f32 {
        self.view_proj[1][1] * self.screen_height_px * 0.5
    }

    /// 误差球屏幕像素投影(报告1 §3.2 标准公式)。
    ///
    /// 推导:与视线垂直的世界长度 ℓ 在径向距离 d 处,clip_y = m11·ℓ、w ≈ d
    /// (小角近似 w = −z_view ≈ d),ndc = m11·ℓ/d,像素 = ndc·H/2
    /// ⇒ `px = ℓ·m11·H/(2d)`。d 取相机到球心**径向距离**(与 Bevy/meshopt
    /// 同口径;非视深,广角下投影偏大、偏向选细层,保守一致)。
    ///
    /// 边界:d ≤ ℓ 时视角 ≥1 rad、小角近似失效,保守返回 +∞(必选细层方向,
    /// 配合叶簇 error = 0 恒过自检,cut 无洞);ℓ ≤ 0 返回 0(叶簇恒过);
    /// ℓ = +∞ 直通(根 parent_error 恒过父检)。
    pub fn projected_error_px(&self, error_world: f32, dist: f32) -> f32 {
        if error_world <= 0.0 {
            return 0.0;
        }
        if error_world.is_infinite() {
            return f32::INFINITY;
        }
        if dist > error_world {
            error_world * self.projection_factor() / dist
        } else {
            f32::INFINITY
        }
    }

    /// 包围球投影直径(分箱度量:簇屏幕尺寸上界;d ≤ r 时相机在球内,
    /// 屏幕填满 ⇒ +∞ ⇒ HW 箱)。
    pub fn projected_diameter_px(&self, radius_world: f32, dist: f32) -> f32 {
        if dist > radius_world {
            2.0 * radius_world * self.projection_factor() / dist
        } else {
            f32::INFINITY
        }
    }
}

// ---------------------------------------------------------------------------
// 第一级:实例剔除(1 线程/实例语义)
// ---------------------------------------------------------------------------

/// 实例级视锥剔除:逐实例世界 AABB(`InstanceRecord::aabb_min/max`,装配期已由
/// 对象 AABB 经变换得出)对视锥。输出可见实例下标(**稳定输入序**;GPU 语义 =
/// 1 线程/实例 + 原子追加,host 参考序即确定性金标准)。
pub fn instance_cull(instances: &[InstanceRecord], cam: &CullCamera) -> Vec<u32> {
    let frustum = cam.frustum();
    let mut out = Vec::new();
    for (i, inst) in instances.iter().enumerate() {
        if frustum.intersects_aabb(inst.aabb_min, inst.aabb_max) {
            out.push(i as u32);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 第二级:簇剔除 + LOD cut(1 线程/簇语义)
// ---------------------------------------------------------------------------

/// 可见簇(两级剔除产物)。
///
/// `cluster` = **全局簇表下标**(扁平 `ClusterRecord` 池);`instance` = 实例表
/// 下标。W3 device 接线注记:VisBuffer cluster27 写的是**本帧可见簇列表的下标**
/// 而非本字段(报告1 §3.4 Nanite 口径——实例 × 簇展开后的帧内列表,材质经
/// 实例反查,同 mesh 多实例多材质才可区分;见 `visbuffer`/`material_pass`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VisibleCluster {
    pub instance: u32,
    pub cluster: u32,
}

/// 簇级剔除:对可见实例的簇段逐簇三关(视锥球 → 背面锥 → LOD cut)。
///
/// 输出稳定序(可见实例序 × 簇段升序);device 端 1 线程/簇 + subgroup 压缩的
/// 顺序不锚定,对拍比集合与计数。
pub fn cluster_cull(
    instances: &[InstanceRecord],
    visible_instances: &[u32],
    clusters: &[ClusterRecord],
    cam: &CullCamera,
) -> Vec<VisibleCluster> {
    let frustum = cam.frustum();
    let mut out = Vec::new();
    for &vi in visible_instances {
        let inst = &instances[vi as usize];
        for local in 0..inst.cluster_count {
            let gidx = (inst.cluster_offset + local) as usize;
            let c = &clusters[gidx];
            let (center_w, radius_w, scale) = world_sphere(&inst.transform, c);
            // 关 1:视锥(包围球)。
            if !frustum.contains_sphere(center_w, radius_w) {
                continue;
            }
            // 关 2:背面锥(cutoff ≥ 1 禁用;锥顶点 = 球心,近距退化不剔)。
            if c.cone_cutoff < 1.0 {
                let to_center = sub3(center_w, cam.cam_pos);
                let dist = dot3(to_center, to_center).sqrt();
                if dist > 1e-6 {
                    let view = [
                        to_center[0] / dist,
                        to_center[1] / dist,
                        to_center[2] / dist,
                    ];
                    if let Some(axis_w) = transform_dir_normalized(&inst.transform, c.cone_axis)
                        && dot3(view, axis_w) >= c.cone_cutoff
                    {
                        continue;
                    }
                }
            }
            // 关 3:LOD cut(自身 < 阈 且 父级 ≥ 阈,互补边界恰成 cut)。
            let dist = dist3(center_w, cam.cam_pos);
            let self_px = cam.projected_error_px(c.error * scale, dist);
            let parent_px = cam.projected_error_px(c.parent_error * scale, dist);
            if self_px < cam.error_threshold_px && parent_px >= cam.error_threshold_px {
                out.push(VisibleCluster {
                    instance: vi,
                    cluster: gidx as u32,
                });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 分箱与间接参数(单线程组前缀和语义的确定性镜像)
// ---------------------------------------------------------------------------

/// 光栅分箱与间接参数(host 参考;GPU 单线程组「计数 → 前缀和 → 散射」三趟的
/// 确定性等价物——稳定分割保持输入序;device 原子散射序不锚定,对拍比集合与计数)。
#[derive(Debug, Clone, PartialEq)]
pub struct DrawArgsCpu {
    /// SW(小三角形 compute 软光栅)路簇数 ⇒ `DispatchIndirect` 簇维组数语义。
    pub sw_cluster_count: u32,
    /// HW(大三角形硬件间接绘制)路簇数 ⇒ `DrawIndirect` 图元数语义。
    pub hw_cluster_count: u32,
    /// SW 路三角形总数(sw_raster 1 线程/三角形的 dispatch 宽度审计面)。
    pub sw_triangle_count: u32,
    /// HW 路三角形总数(间接绘制 indexCount 汇总审计面)。
    pub hw_triangle_count: u32,
    /// SW 箱簇列表(稳定输入序)。
    pub sw_clusters: Vec<VisibleCluster>,
    /// HW 箱簇列表(稳定输入序)。
    pub hw_clusters: Vec<VisibleCluster>,
}

/// 分箱:按簇包围球投影直径与阈值的比较切 SW/HW 两路(报告1 §3.3 分箱思想;
/// 阈值默认 [`DEFAULT_BIN_THRESHOLD_PX`] = 32px 边长档)。
pub fn compact_draw_args(
    visible: &[VisibleCluster],
    instances: &[InstanceRecord],
    clusters: &[ClusterRecord],
    cam: &CullCamera,
    bin_threshold_px: f32,
) -> DrawArgsCpu {
    let mut sw_clusters = Vec::new();
    let mut hw_clusters = Vec::new();
    let (mut sw_triangle_count, mut hw_triangle_count) = (0u32, 0u32);
    // 单趟稳定分割 = 计数趟 + 前缀和 + 散射趟的确定性结果(偏移即运行计数)。
    for vc in visible {
        let inst = &instances[vc.instance as usize];
        let c = &clusters[vc.cluster as usize];
        let (center_w, radius_w, _) = world_sphere(&inst.transform, c);
        let size_px = cam.projected_diameter_px(radius_w, dist3(center_w, cam.cam_pos));
        if size_px >= bin_threshold_px {
            hw_clusters.push(*vc);
            hw_triangle_count += c.triangle_count;
        } else {
            sw_clusters.push(*vc);
            sw_triangle_count += c.triangle_count;
        }
    }
    DrawArgsCpu {
        sw_cluster_count: sw_clusters.len() as u32,
        hw_cluster_count: hw_clusters.len() as u32,
        sw_triangle_count,
        hw_triangle_count,
        sw_clusters,
        hw_clusters,
    }
}

// ---------------------------------------------------------------------------
// 内部工具
// ---------------------------------------------------------------------------

/// 簇包围球经实例变换到世界:球心仿射变换精确;半径按 3×3 列范数最大者缩放
/// (刚体 + 均匀缩放精确,非均匀缩放保守放大——安全方向)。
/// `pub(crate)`:G9.3 `visible_cluster_set` 复用同一世界化口径(单源,禁重算)。
pub(crate) fn world_sphere(m: &[[f32; 4]; 3], c: &ClusterRecord) -> ([f32; 3], f32, f32) {
    let scale = linear_scale(m);
    (transform_point(m, c.center), c.radius * scale, scale)
}

/// 3×3 线性部的最大列范数(缩放因子)。
fn linear_scale(m: &[[f32; 4]; 3]) -> f32 {
    (0..3)
        .map(|j| m[0][j] * m[0][j] + m[1][j] * m[1][j] + m[2][j] * m[2][j])
        .fold(0.0f32, f32::max)
        .sqrt()
}

/// 方向向量经 3×3 线性部变换并归一化(锥轴;刚体/均匀缩放精确)。
/// 退化(零缩放/零轴)返回 None——调用方按「不做锥剔」保守保留。
/// `pub(crate)`:G9.3 `visible_cluster_set` 可见性标记复用同一锥轴变换口径。
pub(crate) fn transform_dir_normalized(m: &[[f32; 4]; 3], v: [f32; 3]) -> Option<[f32; 3]> {
    let mut out = [0.0f32; 3];
    for (i, o) in out.iter_mut().enumerate() {
        *o = m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2];
    }
    let len = dot3(out, out).sqrt();
    if len < 1e-12 {
        return None;
    }
    Some([out[0] / len, out[1] / len, out[2] / len])
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    dot3(sub3(a, b), sub3(a, b)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::gpu_scene::NO_PARENT;

    /// 精确系数相机(90° 视锥 |x|≤|z|、m11 = 1.0 精确 ⇒ 投影系数 = H/2 精确;
    /// near = 0.2、far = +∞,剔除测试不被远面干扰)。
    fn exact_cam(screen_h: f32, threshold: f32) -> CullCamera {
        CullCamera {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, -1.0, -0.2],
                [0.0, 0.0, -1.0, 0.0],
            ],
            cam_pos: [0.0, 0.0, 0.0],
            screen_height_px: screen_h,
            error_threshold_px: threshold,
        }
    }

    fn inst_box(min: [f32; 3], max: [f32; 3]) -> InstanceRecord {
        InstanceRecord {
            transform: super::super::gpu_scene::IDENTITY_3X4,
            cluster_offset: 0,
            cluster_count: 0,
            material_id: 0,
            flags: 0,
            aabb_min: min,
            mesh_id: 0,
            aabb_max: max,
            reserved: NO_PARENT,
        }
    }

    fn inst_at(t: [f32; 3], cluster_offset: u32, cluster_count: u32) -> InstanceRecord {
        InstanceRecord {
            transform: [
                [1.0, 0.0, 0.0, t[0]],
                [0.0, 1.0, 0.0, t[1]],
                [0.0, 0.0, 1.0, t[2]],
            ],
            cluster_offset,
            cluster_count,
            material_id: 0,
            flags: 0,
            aabb_min: [t[0] - 2.0, t[1] - 2.0, t[2] - 2.0],
            mesh_id: 0,
            aabb_max: [t[0] + 2.0, t[1] + 2.0, t[2] + 2.0],
            reserved: NO_PARENT,
        }
    }

    fn cluster(
        center: [f32; 3],
        radius: f32,
        axis: [f32; 3],
        cutoff: f32,
        err: f32,
        perr: f32,
    ) -> ClusterRecord {
        ClusterRecord {
            center,
            radius,
            cone_axis: axis,
            cone_cutoff: cutoff,
            error: err,
            parent_error: perr,
            vertex_offset: 0,
            triangle_offset: 0,
            vertex_count: 0,
            triangle_count: 0,
            page_id: 0,
            reserved: 0,
        }
    }

    #[test]
    fn frustum_instance_inside_outside_straddle() {
        let cam = exact_cam(1000.0, 1.0);
        let inside = inst_box([-1.0, -1.0, -11.0], [1.0, 1.0, -9.0]);
        let lateral_out = inst_box([98.0, -1.0, -11.0], [100.0, 1.0, -9.0]);
        let behind = inst_box([-1.0, -1.0, 9.0], [1.0, 1.0, 11.0]);
        let left_out = inst_box([-5.0, -1.0, -2.0], [-3.0, 1.0, -1.0]);
        // 跨界:z ∈ [-2,-0.5] 段上 x ∈ [-2,0] 与左平面 x = z 相交 ⇒ 保留(保守)。
        let straddle = inst_box([-2.0, -1.0, -2.0], [0.0, 1.0, -0.5]);
        // 近平面跨界(部分在 near 内)⇒ 保留。
        let near_straddle = inst_box([-1.0, -1.0, -0.3], [1.0, 1.0, -0.05]);
        let visible = instance_cull(
            &[
                inside,
                lateral_out,
                behind,
                left_out,
                straddle,
                near_straddle,
            ],
            &cam,
        );
        assert_eq!(visible, vec![0, 4, 5]);
    }

    #[test]
    fn cone_cull_front_back_disabled() {
        // 簇心世界 (0,0,-10):view = (0,0,-1)。LOD 恒过(error 0、parent ∞)隔离锥关。
        let cam = exact_cam(1000.0, 1.0);
        let inst = [inst_at([0.0, 0.0, -10.0], 0, 3)];
        let back = cluster([0.0; 3], 1.0, [0.0, 0.0, -1.0], 0.5, 0.0, f32::INFINITY);
        let front = cluster([0.0; 3], 1.0, [0.0, 0.0, 1.0], 0.5, 0.0, f32::INFINITY);
        let disabled = cluster([0.0; 3], 1.0, [0.0, 0.0, -1.0], 1.0, 0.0, f32::INFINITY);
        let clusters = [back, front, disabled];
        let out = cluster_cull(&inst, &[0], &clusters, &cam);
        // 背对(axis 与 view 同向,dot = 1 ≥ 0.5)剔;正对(dot = −1)留;cutoff = 1 禁用留。
        assert_eq!(
            out,
            vec![
                VisibleCluster {
                    instance: 0,
                    cluster: 1
                },
                VisibleCluster {
                    instance: 0,
                    cluster: 2
                }
            ]
        );
    }

    /// 手工 DAG:叶 0..3(error 0,parent 0.5)/ 中 4..5(0.5,2.0)/ 根 6(2.0,+∞);
    /// 全部对象空间同心(原点),世界位置由实例平移 (0,0,-d) 承载。
    fn hand_dag() -> Vec<ClusterRecord> {
        let mut v = Vec::new();
        for _ in 0..4 {
            v.push(cluster([0.0; 3], 0.5, [0.0; 3], 2.0, 0.0, 0.5));
        }
        for _ in 0..2 {
            v.push(cluster([0.0; 3], 0.5, [0.0; 3], 2.0, 0.5, 2.0));
        }
        v.push(cluster([0.0; 3], 0.5, [0.0; 3], 2.0, 2.0, f32::INFINITY));
        v
    }

    fn hand_dag_children(id: u32) -> &'static [u32] {
        match id {
            6 => &[4, 5],
            4 => &[0, 1],
            5 => &[2, 3],
            _ => &[],
        }
    }

    fn expand_to_leaves(selected: &[u32]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut stack: Vec<u32> = selected.to_vec();
        while let Some(id) = stack.pop() {
            let ch = hand_dag_children(id);
            if ch.is_empty() {
                out.push(id);
            } else {
                stack.extend_from_slice(ch);
            }
        }
        out.sort_unstable();
        out
    }

    fn dag_visible(d: f32, threshold: f32) -> Vec<u32> {
        let cam = exact_cam(1000.0, threshold);
        let clusters = hand_dag();
        let inst = [inst_at([0.0, 0.0, -d], 0, 7)];
        cluster_cull(&inst, &[0], &clusters, &cam)
            .iter()
            .map(|vc| vc.cluster)
            .collect()
    }

    #[test]
    fn lod_cut_near_selects_leaves_far_selects_root() {
        // 投影系数 500(H = 1000/2):中误差 0.5·500/d = 250/d,根 2·500/d = 1000/d。
        // 近 d = 100:250/100 = 2.5 ≥ 1 ⇒ 父可感知 ⇒ 全叶。
        assert_eq!(dag_visible(100.0, 1.0), vec![0, 1, 2, 3]);
        // 远 d = 100000:1000/d = 0.01 < 1 ⇒ 仅根。
        assert_eq!(dag_visible(100000.0, 1.0), vec![6]);
    }

    #[test]
    fn lod_cut_boundary_complementary() {
        // 边界精确值(f32 精确算术):self < t 与 parent ≥ t 互补,恰一侧入选。
        // d = 250:中误差恰 1.0 ⇒ 中自检失败(1.0 ≮ 1)、叶父检过(1.0 ≥ 1)⇒ 选细层。
        assert_eq!(dag_visible(250.0, 1.0), vec![0, 1, 2, 3]);
        // d = 1000:根误差恰 1.0 ⇒ 根自检失败、中父检过 ⇒ 选中层。
        assert_eq!(dag_visible(1000.0, 1.0), vec![4, 5]);
    }

    #[test]
    fn lod_cut_coverage_exact_hand_dag() {
        // DAG cut 性质(思想同 rurix-geom-build `lod_cut_coverage_exact`,数据自建
        // 不依赖该 crate):任意相机/阈值下,选中集叶覆盖 = 全叶集 {0,1,2,3} 恰好一次。
        for &(d, t) in &[
            (100.0f32, 1.0f32),
            (250.0, 1.0),
            (400.0, 1.0),
            (1000.0, 1.0),
            (2000.0, 1.0),
            (200.0, 4.0),
            (50.0, 0.25),
        ] {
            let selected = dag_visible(d, t);
            assert!(!selected.is_empty(), "d={d} t={t} cut 为空");
            let mut expanded = expand_to_leaves(&selected);
            expanded.dedup();
            assert_eq!(expanded, vec![0, 1, 2, 3], "d={d} t={t} 叶覆盖有洞或重叠");
        }
    }

    #[test]
    fn two_level_compose_instance_culled_clusters_skipped() {
        // 实例 1 视锥外:其簇段(即使簇本身位置可达)不进入簇剔除输出。
        let cam = exact_cam(1000.0, 1.0);
        let clusters = vec![
            cluster([0.0; 3], 0.5, [0.0; 3], 2.0, 0.0, f32::INFINITY),
            cluster([0.0; 3], 0.5, [0.0; 3], 2.0, 0.0, f32::INFINITY),
        ];
        let inst = [
            inst_at([0.0, 0.0, -10.0], 0, 1),
            inst_at([100.0, 0.0, -10.0], 1, 1),
        ];
        let visible_instances = instance_cull(&inst, &cam);
        assert_eq!(visible_instances, vec![0]);
        let out = cluster_cull(&inst, &visible_instances, &clusters, &cam);
        assert_eq!(
            out,
            vec![VisibleCluster {
                instance: 0,
                cluster: 0
            }]
        );
    }

    #[test]
    fn compact_draw_args_bins_and_counts() {
        let cam = exact_cam(1000.0, 1.0);
        // 近簇(0,0,-5) r = 1:投影直径 2·1·500/5 = 200 ≥ 32 ⇒ HW;tri 3。
        // 远簇(0,0,-500) r = 1:2·500/500 = 2 < 32 ⇒ SW;tri 5。
        let mut near = cluster([0.0; 3], 1.0, [0.0; 3], 2.0, 0.0, f32::INFINITY);
        near.triangle_count = 3;
        let mut far = cluster([0.0; 3], 1.0, [0.0; 3], 2.0, 0.0, f32::INFINITY);
        far.triangle_count = 5;
        let clusters = [near, far];
        let inst = [
            inst_at([0.0, 0.0, -5.0], 0, 1),
            inst_at([0.0, 0.0, -500.0], 1, 1),
        ];
        let visible = vec![
            VisibleCluster {
                instance: 0,
                cluster: 0,
            },
            VisibleCluster {
                instance: 1,
                cluster: 1,
            },
        ];
        let args = compact_draw_args(&visible, &inst, &clusters, &cam, DEFAULT_BIN_THRESHOLD_PX);
        assert_eq!(args.hw_cluster_count, 1);
        assert_eq!(args.sw_cluster_count, 1);
        assert_eq!(args.hw_triangle_count, 3);
        assert_eq!(args.sw_triangle_count, 5);
        assert_eq!(args.hw_clusters, vec![visible[0]]);
        assert_eq!(args.sw_clusters, vec![visible[1]]);
        // 稳定序:输入序 [近, 远] 保持到各自箱内。
        assert_eq!(args.hw_clusters[0].instance, 0);
    }
}
