//! RT 效果 pass(RTAO / 硬阴影)host 完整实现——GBuffer 驱动逐像素生产者
//! (报告4 §3.2 P0 最小闭环「TLAS + ray query 效果 + 时域降噪」与 §5 机制清单
//! 「效果缓冲与降噪链」;RFC-0016 §4.F F2 效果 pass,验收门 G-G5-6)。
//!
//! 与 [`crate::rt::ref_tracer`] 的分工(对拍契约,RFC §4.F F3「同结构对拍」):
//! ref_tracer 是**顶点数组驱动**的逐点金标准(调用方直喂世界位置/法线),本模块是
//! **GBuffer 驱动**的逐像素生产者(depth+normal 反投影世界位置)。两者几何同源
//! (同一 [`Tlas`] + [`BlasSet`])、采样定律同源(法线半球余弦加权 + [`RAY_EPS`]
//! 自交纪律 + 同一 [`Pcg32`] 调度),高 spp 下效果 pass 必须收敛到参考——单测以
//! 「frame 0 同 seed 位级对拍 + 独立流高 spp 收敛界」双层锚定。
//!
//! ## GBuffer 重建约定(世界位置反投影,与 temporal 底座同口径)
//!
//! - `depth`:单通道 ZO 深度 ∈ \[0,1\]([`crate::temporal::common::perspective_rh_zo`]
//!   口径);**depth ≥ 1.0 / NaN / ±inf = 天空(无几何)无效像素**,效果输出 1.0
//!   (与 ref_tracer 无效像素约定一致);远平面真实表面与天空不可区分(标准局限,
//!   写明在此)。
//! - `normals`:3 通道**世界空间**法线(无需归一,内部归一化);零长/非有限 =
//!   无效像素(RTAO 判据;硬阴影只依赖位置,不消费法线,与 ref_tracer 判据对齐)。
//! - 世界位置:像素 (x,y) 纹素中心 uv = ((x+0.5)/w, (y+0.5)/h),ndc =
//!   (2u−1, 1−2v, depth),world = inverse(view_proj)·(ndc,1) 透视除——与
//!   [`crate::temporal::common::compute_camera_mv`] 同一反投影约定(跨模块单口径);
//!   齐次 w 失效(|w| < 1e-8 或产物非有限)= 无效像素。
//!
//! ## RNG 调度(确定性契约;device 腿对齐锚点)
//!
//! RTAO 与 [`crate::rt::ref_tracer::rtao_reference`] **同一消费调度**:单一
//! [`Pcg32`] 流,行主序逐像素、逐采样 (r1, r2) 顺序消费,无效像素不消费。
//! 帧间去相关 = 种子混合 [`frame_seed`](frame_index 乘异或入种,frame 0 恒等
//! = 对拍对齐臂):同 (seed, frame_index) 逐像素逐位可复现,异 frame_index
//! 采样流去相关。时域滤波(低 spp 累积收敛)依赖此去相关,见 [`crate::rt::denoise`]。

use crate::rt::bvh::{BlasSet, Ray, Tlas, Vec3};
use crate::rt::ref_tracer::{Pcg32, RAY_EPS};
use crate::temporal::common::Mat4;
use crate::temporal::image::ImageF32;

// ---------------------------------------------------------------------------
// 效果输入契约与度量埋点
// ---------------------------------------------------------------------------

/// 效果 pass 输入契约(GBuffer 驱动;RFC-0016 §4.F F2「效果缓冲抽象」)。
///
/// 字段语义与重建约定见模块文档「GBuffer 重建约定」。`view_proj` = 相机
/// 视图投影矩阵(生成 depth 所用的那一组;世界位置经其逆矩阵重建)。
#[derive(Debug, Clone, Copy)]
pub struct EffectInputs<'a, B: BlasSet + ?Sized> {
    /// 单通道 ZO 深度图(≥1.0/非有限 = 天空无效像素)。
    pub depth: &'a ImageF32,
    /// 3 通道世界空间法线图(零长/非有限 = RTAO 无效像素)。
    pub normals: &'a ImageF32,
    /// 相机视图投影矩阵(必须可逆;逆矩阵用于逐像素世界位置重建)。
    pub view_proj: Mat4,
    /// 场景实例级加速结构(与参考效果同一份几何输入)。
    pub tlas: &'a Tlas,
    /// BLAS 集合(TLAS 实例引用经 [`BlasSet`] 解析)。
    pub blases: &'a B,
}

impl<'a, B: BlasSet + ?Sized> EffectInputs<'a, B> {
    /// 构造并校验通道/尺寸契约。
    ///
    /// # Panics
    /// `depth` 非单通道、`normals` 非 3 通道或两者尺寸不一致即 panic(调用契约)。
    pub fn new(
        depth: &'a ImageF32,
        normals: &'a ImageF32,
        view_proj: Mat4,
        tlas: &'a Tlas,
        blases: &'a B,
    ) -> Self {
        assert_eq!(depth.c, 1, "EffectInputs: depth 必须单通道");
        assert_eq!(normals.c, 3, "EffectInputs: normals 必须 3 通道(世界空间)");
        assert!(
            depth.w == normals.w && depth.h == normals.h,
            "EffectInputs: depth/normals 尺寸必须一致"
        );
        Self {
            depth,
            normals,
            view_proj,
            tlas,
            blases,
        }
    }

    /// 字段直构造后的防御性校验(各 pass 入口复查)。
    fn assert_shapes(&self) {
        assert_eq!(self.depth.c, 1, "EffectInputs: depth 必须单通道");
        assert_eq!(self.normals.c, 3, "EffectInputs: normals 必须 3 通道");
        assert!(
            self.depth.w == self.normals.w && self.depth.h == self.normals.h,
            "EffectInputs: depth/normals 尺寸必须一致"
        );
    }
}

/// 效果 pass 度量埋点(报告4 §5「验证与画像:逐效果计数」;evidence 计数源)。
///
/// 各 pass 以**覆写语义**写入:一次调用 = 一帧的独立计数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectStats {
    /// 本帧发射的追踪射线总数(RTAO = 有效像素 × spp;硬阴影 = 有效像素 × 1)。
    pub rays: u64,
    /// 本帧无效(天空/无几何/坏法线)像素数,输出 1.0 且不发射射线。
    pub invalid_pixels: u32,
}

// ---------------------------------------------------------------------------
// 采样与反投影工具(与 ref_tracer 同定律;确定性)
// ---------------------------------------------------------------------------

/// 帧间去相关种子混合:frame_index 乘异或入种(奇数黄金比例乘子)。
///
/// **对拍对齐臂**:`frame_seed(seed, 0) == seed`——frame 0 的效果 pass 与同
/// seed 的参考效果位级一致(同一单流调度)。frame ≥ 1 经 [`Pcg32::new`] 播种
/// 序列充分打散,采样流逐帧去相关。同 (seed, frame_index) 跨平台位级可复现。
pub fn frame_seed(seed: u64, frame_index: u32) -> u64 {
    seed ^ u64::from(frame_index).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// 有效 GBuffer 深度判据:有限且 ∈ \[0,1)(1.0 = 远平面/天空,无效)。
fn valid_gbuffer_depth(depth: f32) -> bool {
    depth.is_finite() && (0.0..1.0).contains(&depth)
}

/// 逐像素世界位置重建(模块文档「GBuffer 重建约定」;齐次 w 失效产 `None`)。
fn world_position_from_gbuffer(
    inv_view_proj: &Mat4,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    depth: f32,
) -> Option<Vec3> {
    let u = (x as f32 + 0.5) / w as f32;
    let v = (y as f32 + 0.5) / h as f32;
    let ndc = [2.0 * u - 1.0, 1.0 - 2.0 * v, depth, 1.0];
    let world4 = inv_view_proj.transform_vec4(ndc);
    if !world4[3].is_finite() || world4[3].abs() < 1e-8 {
        return None;
    }
    let p = Vec3::new(
        world4[0] / world4[3],
        world4[1] / world4[3],
        world4[2] / world4[3],
    );
    if p.is_finite() { Some(p) } else { None }
}

/// 法线半球余弦加权采样方向(单位长)。**与 ref_tracer 同一定律逐字对齐**
/// (对拍契约:同 (r1, r2) 入 → 同方向出);`r1, r2 ∈ [0,1)` 均匀输入。
fn cosine_sample_hemisphere(n: Vec3, r1: f32, r2: f32) -> Vec3 {
    let (t, b) = orthonormal_basis(n);
    let phi = 2.0 * core::f32::consts::PI * r1;
    let r = r2.sqrt();
    let x = r * phi.cos();
    let y = r * phi.sin();
    let z = (1.0 - r2).max(0.0).sqrt();
    (t * x + b * y + n * z).normalize()
}

/// 由法线构造正交基(与 ref_tracer 同一确定性分支)。
fn orthonormal_basis(n: Vec3) -> (Vec3, Vec3) {
    let up = if n.y.abs() < 0.999 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let t = up.cross(n).normalize();
    let b = n.cross(t);
    (t, b)
}

// ---------------------------------------------------------------------------
// 效果 pass
// ---------------------------------------------------------------------------

/// RTAO pass:逐像素法线半球余弦加权 `samples_per_pixel` 条 any_hit 遮蔽率,
/// 输出单通道 AO ∈ \[0,1\](1 = 无遮蔽;报告4 §3.2 推荐首效果之一,Vulkan
/// 官方教程「GBuffer + compute ray query」形态)。
///
/// - 无效像素(天空/坏法线)输出 1.0 且不计射线(模块文档约定);
/// - `samples_per_pixel == 0` 退化全 1(与 ref_tracer 零采样约定一致),不计射线;
/// - `radius` = 遮蔽判定开区间 `(0, radius)` 上界;≤ 0 自然退化为无遮蔽;
/// - 确定性:同 (seed, frame_index) 逐像素逐位可复现;**frame 0 与同 seed 的
///   [`crate::rt::ref_tracer::rtao_reference`] 位级一致**(同一单流调度,
///   [`frame_seed`] 对齐臂),异 frame 去相关。
///
/// # Panics
/// 输入形状违约或 `view_proj` 不可逆即 panic。
pub fn rtao_pass<B: BlasSet + ?Sized>(
    inputs: &EffectInputs<'_, B>,
    samples_per_pixel: u32,
    radius: f32,
    frame_index: u32,
    seed: u64,
    stats: &mut EffectStats,
) -> ImageF32 {
    inputs.assert_shapes();
    let (w, h) = (inputs.depth.w, inputs.depth.h);
    let inv_vp = inputs
        .view_proj
        .inverse()
        .expect("rtao_pass: view_proj 必须可逆");
    let mut rng = Pcg32::new(frame_seed(seed, frame_index));
    let mut out = ImageF32::new(w, h, 1);
    let mut rays = 0u64;
    let mut invalid = 0u32;
    for y in 0..h {
        for x in 0..w {
            let depth = inputs.depth.get(x, y, 0);
            let pos = if valid_gbuffer_depth(depth) {
                world_position_from_gbuffer(&inv_vp, x, y, w, h, depth)
            } else {
                None
            };
            let n = Vec3::from_array(inputs.normals.pixel3(x, y));
            let (p, n) = match (pos, n.is_finite() && n.length() > 0.0) {
                (Some(p), true) => (p, n.normalize()),
                _ => {
                    invalid += 1;
                    out.set(x, y, 0, 1.0);
                    continue;
                }
            };
            if samples_per_pixel == 0 {
                // 零采样退化:全 1(不消费 RNG,与参考调度对齐)。
                out.set(x, y, 0, 1.0);
                continue;
            }
            let origin = p + n * RAY_EPS;
            let mut occluded = 0u32;
            for _ in 0..samples_per_pixel {
                let r1 = rng.next_f32();
                let r2 = rng.next_f32();
                let dir = cosine_sample_hemisphere(n, r1, r2);
                if inputs
                    .tlas
                    .any_hit(inputs.blases, &Ray { origin, dir }, radius)
                {
                    occluded += 1;
                }
            }
            rays += u64::from(samples_per_pixel);
            out.set(x, y, 0, 1.0 - occluded as f32 / samples_per_pixel as f32);
        }
    }
    *stats = EffectStats {
        rays,
        invalid_pixels: invalid,
    };
    out
}

/// RT 硬阴影 pass:逐像素向光源一根 any_hit 阴影光线,输出单通道 0/1 可见性
/// (报告4 §3.2 推荐首效果之一;与 VSM 对照验证面,RFC §4.F F2)。
///
/// - `light_dir`:指向光源的方向(无需归一,内部归一化);零长/非有限退化全 1
///   (无方向即无遮蔽,与 ref_tracer 约定一致),不计射线;
/// - 无效像素(天空)输出 1.0;命中区间 `(0, +∞)`,原点沿光方向偏移 [`RAY_EPS`]
///   (自交纪律沿 ref_tracer,接收面全亮区无噪点);
/// - 无 RNG:同输入逐位可复现;与同几何的
///   [`crate::rt::ref_tracer::hard_shadow_reference`] 逐像素位级一致。
///
/// # Panics
/// 输入形状违约或 `view_proj` 不可逆即 panic。
pub fn hard_shadow_pass<B: BlasSet + ?Sized>(
    inputs: &EffectInputs<'_, B>,
    light_dir: [f32; 3],
    stats: &mut EffectStats,
) -> ImageF32 {
    inputs.assert_shapes();
    let (w, h) = (inputs.depth.w, inputs.depth.h);
    let inv_vp = inputs
        .view_proj
        .inverse()
        .expect("hard_shadow_pass: view_proj 必须可逆");
    let l = Vec3::from_array(light_dir);
    let degenerate = !l.is_finite() || l.length() == 0.0;
    let dir = l.normalize();
    let mut out = ImageF32::new(w, h, 1);
    let mut rays = 0u64;
    let mut invalid = 0u32;
    for y in 0..h {
        for x in 0..w {
            let depth = inputs.depth.get(x, y, 0);
            let pos = if valid_gbuffer_depth(depth) {
                world_position_from_gbuffer(&inv_vp, x, y, w, h, depth)
            } else {
                None
            };
            let Some(p) = pos else {
                invalid += 1;
                out.set(x, y, 0, 1.0);
                continue;
            };
            if degenerate {
                out.set(x, y, 0, 1.0);
                continue;
            }
            let origin = p + dir * RAY_EPS;
            let lit = if inputs
                .tlas
                .any_hit(inputs.blases, &Ray { origin, dir }, f32::INFINITY)
            {
                0.0
            } else {
                1.0
            };
            rays += 1;
            out.set(x, y, 0, lit);
        }
    }
    *stats = EffectStats {
        rays,
        invalid_pixels: invalid,
    };
    out
}

// ---------------------------------------------------------------------------
// GBuffer 生成辅助(同源对拍基础设施;报告4 §5「与离线路径追踪器的自动对拍」)
// ---------------------------------------------------------------------------

/// 用 TLAS 主射线生成 depth/normal GBuffer(**几何同源保证**:效果 pass 的
/// GBuffer 与参考效果所见的 TLAS 是同一份几何,对拍不把几何错误与采样错误
/// 互相甩锅,RFC §4.F F3)。
///
/// 主射线:纹素中心反投影近平面点(depth=0)与远平面点(depth=1),方向归一;
/// `tlas.intersect` 最近命中 → 命中点经 `view_proj` 正投影回深度(与重建约定
/// 精确往返),法线取世界空间几何法线(单位长);未命中 = 天空(depth=1.0,
/// 法线零向量 = 无效像素)。
///
/// 返回 `(depth, normals)`:`depth` 单通道,`normals` 3 通道世界空间。
///
/// # Panics
/// `view_proj` 不可逆即 panic。
pub fn gbuffer_from_scene<B: BlasSet + ?Sized>(
    view_proj: &Mat4,
    width: u32,
    height: u32,
    tlas: &Tlas,
    blases: &B,
) -> (ImageF32, ImageF32) {
    let inv_vp = view_proj
        .inverse()
        .expect("gbuffer_from_scene: view_proj 必须可逆");
    let mut depth = ImageF32::new(width, height, 1);
    let mut normals = ImageF32::new(width, height, 3);
    for y in 0..height {
        for x in 0..width {
            // 天空默认值:depth=1.0、法线零向量(无效像素约定)。
            depth.set(x, y, 0, 1.0);
            let near = world_position_from_gbuffer(&inv_vp, x, y, width, height, 0.0)
                .expect("gbuffer_from_scene: 近平面反投影必须有限");
            let far = world_position_from_gbuffer(&inv_vp, x, y, width, height, 1.0)
                .expect("gbuffer_from_scene: 远平面反投影必须有限");
            let dir = (far - near).normalize();
            if dir.length() == 0.0 {
                continue;
            }
            if let Some(hit) = tlas.intersect(blases, &Ray { origin: near, dir }) {
                let wp = near + dir * hit.t;
                let clip = view_proj.transform_vec4([wp.x, wp.y, wp.z, 1.0]);
                if clip[3].abs() < 1e-8 || !clip[2].is_finite() {
                    continue;
                }
                depth.set(x, y, 0, clip[2] / clip[3]);
                normals.set_pixel3(x, y, Vec3::from_array(hit.normal).normalize().to_array());
            }
        }
    }
    (depth, normals)
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::bvh::{InstanceDesc, Transform3x4, TriBvh};
    use crate::rt::ref_tracer::{hard_shadow_reference, rtao_reference};
    use crate::temporal::common::{look_at_rh, perspective_rh_zo};

    /// 网格(位置 + 索引)。
    type Mesh = (Vec<[f32; 3]>, Vec<[u32; 3]>);

    /// 地板四边形:x,z ∈ [x0,x1]×[z0,z1],高度 y,**法线 +y**(winding 已校验)。
    fn quad_y_up(x0: f32, x1: f32, z0: f32, z1: f32, y: f32) -> Mesh {
        (
            vec![[x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1]],
            vec![[0, 2, 1], [0, 3, 2]],
        )
    }

    /// x=0 墙面四边形:y,z ∈ [y0,y1]×[z0,z1],**法线 +x**(朝向墙角内侧)。
    fn quad_x_up(y0: f32, y1: f32, z0: f32, z1: f32, x: f32) -> Mesh {
        (
            vec![[x, y0, z0], [x, y1, z0], [x, y1, z1], [x, y0, z1]],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    /// z=0 墙面四边形:x,y ∈ [x0,x1]×[y0,y1],**法线 +z**(朝向墙角内侧)。
    fn quad_z_up(x0: f32, x1: f32, y0: f32, y1: f32, z: f32) -> Mesh {
        (
            vec![[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    /// 把多份网格合并为单一 BLAS(共享顶点数组)。
    fn merge(meshes: &[Mesh]) -> Mesh {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut indices: Vec<[u32; 3]> = Vec::new();
        for (pos, idx) in meshes {
            let base = positions.len() as u32;
            positions.extend(pos.iter().copied());
            indices.extend(idx.iter().map(|t| [t[0] + base, t[1] + base, t[2] + base]));
        }
        (positions, indices)
    }

    /// 单 BLAS 单实例(恒等变换)场景。
    fn scene_of(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> (Vec<TriBvh>, Tlas) {
        let blases = vec![TriBvh::build(positions, indices)];
        let tlas = Tlas::build(
            &[InstanceDesc {
                blas: 0,
                transform: Transform3x4::IDENTITY,
                mask: 0xFF,
                flags: 0,
            }],
            &blases,
        );
        (blases, tlas)
    }

    /// 对拍主场景(规格:三面墙角 + 开阔平面):大地板 y=0(x,z ∈ [−4,4]),
    /// 墙 x=0 与 z=0(y,z / x,y ∈ [0,2]);墙角遮蔽显著,远处开阔平面遮蔽精确为零。
    fn corner_open_scene() -> (Vec<TriBvh>, Tlas) {
        let floor = quad_y_up(-4.0, 4.0, -4.0, 4.0, 0.0);
        let wall_x = quad_x_up(0.0, 2.0, 0.0, 2.0, 0.0);
        let wall_z = quad_z_up(0.0, 2.0, 0.0, 2.0, 0.0);
        let (pos, idx) = merge(&[floor, wall_x, wall_z]);
        scene_of(&pos, &idx)
    }

    /// 墙角相机:俯视墙角与开阔地板,视线略抬使上缘越墙顶含天空像素
    /// (无效像素路径在位级对拍同步覆盖)。
    fn corner_camera() -> Mat4 {
        let proj = perspective_rh_zo(0.9, 1.0, 0.1, 50.0);
        let view = look_at_rh([2.4, 2.0, 2.4], [0.0, 0.7, 0.0], [0.0, 1.0, 0.0]);
        proj.mul(&view)
    }

    /// 正俯视相机(硬阴影对拍用):像素格规则落在地板上,画面 x → 世界 x。
    fn top_down_camera(height: f32) -> Mat4 {
        let proj = perspective_rh_zo(0.9, 1.0, 0.1, 50.0);
        let view = look_at_rh([0.0, height, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, -1.0]);
        proj.mul(&view)
    }

    /// 像素 ↔ 顶点采样点对齐(GBuffer 驱动 ↔ 参考顶点驱动的桥):
    /// 逐像素按「GBuffer 重建约定」反投影世界位置(与 pass 内部同一函数,
    /// 位级同源),无效像素喂 NaN 位置 + 零法线(参考效果同判无效、同不消费
    /// RNG)——两侧跳过集合逐像素一致,单流调度逐位对齐。
    fn gbuffer_point_arrays<B: BlasSet + ?Sized>(
        inputs: &EffectInputs<'_, B>,
    ) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
        let inv_vp = inputs.view_proj.inverse().expect("view_proj 可逆");
        let (w, h) = (inputs.depth.w, inputs.depth.h);
        let mut positions = Vec::with_capacity((w * h) as usize);
        let mut normals = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let d = inputs.depth.get(x, y, 0);
                let n = Vec3::from_array(inputs.normals.pixel3(x, y));
                let pos = if valid_gbuffer_depth(d) {
                    world_position_from_gbuffer(&inv_vp, x, y, w, h, d)
                } else {
                    None
                };
                match (pos, n.is_finite() && n.length() > 0.0) {
                    (Some(p), true) => {
                        positions.push(p.to_array());
                        normals.push(n.to_array());
                    }
                    _ => {
                        positions.push([f32::NAN; 3]);
                        normals.push([0.0; 3]);
                    }
                }
            }
        }
        (positions, normals)
    }

    const W: u32 = 24;
    const H: u32 = 24;
    const RADIUS: f32 = 1.0;
    const SEED: u64 = 0x5EED_0001;

    fn corner_inputs<'a>(
        depth: &'a ImageF32,
        normals: &'a ImageF32,
        vp: Mat4,
        tlas: &'a Tlas,
        blases: &'a Vec<TriBvh>,
    ) -> EffectInputs<'a, Vec<TriBvh>> {
        EffectInputs::new(depth, normals, vp, tlas, blases)
    }

    #[test]
    fn rtao_open_plane_exactly_lit() {
        // 开阔平面 AO=1 精确:正俯视纯地板,半球采样恒有 +y 分量,不可能命中
        // 地板(RAY_EPS 沿法线偏移后地板命中 t < 0 不被接受)。
        let (pos, idx) = quad_y_up(-4.0, 4.0, -4.0, 4.0, 0.0);
        let (blases, tlas) = scene_of(&pos, &idx);
        let vp = top_down_camera(5.0);
        let (depth, normals) = gbuffer_from_scene(&vp, 16, 16, &tlas, &blases);
        let inputs = corner_inputs(&depth, &normals, vp, &tlas, &blases);
        let mut stats = EffectStats::default();
        let ao = rtao_pass(&inputs, 16, RADIUS, 0, SEED, &mut stats);
        assert!(
            ao.data.iter().all(|&v| v == 1.0),
            "开阔平面 AO 必须精确 1.0"
        );
        assert_eq!(stats.rays, 16 * 16 * 16, "射线数 = 有效像素 × spp");
        assert_eq!(stats.invalid_pixels, 0);
    }

    #[test]
    fn rtao_pointwise_bitwise_matches_reference() {
        // **RTAO 对拍金标准(位级臂)**:同几何(三面墙角 + 开阔平面)同 spp=64。
        // 像素↔顶点采样点对齐方法:gbuffer_point_arrays 逐像素反投影世界位置
        // (与 pass 内部同一函数),无效像素喂 NaN;参考与 pass 同一单流调度、
        // frame 0 种子恒等(frame_seed 对齐臂)→ 输出必须逐位相等(远严于
        // 逐点差 < 0.05 的规格,因为 RNG 流按契约对齐)。
        let (blases, tlas) = corner_open_scene();
        let vp = corner_camera();
        let (depth, normals) = gbuffer_from_scene(&vp, W, H, &tlas, &blases);
        let inputs = corner_inputs(&depth, &normals, vp, &tlas, &blases);
        let (positions, ref_normals) = gbuffer_point_arrays(&inputs);
        let reference = rtao_reference(&positions, &ref_normals, &tlas, &blases, 64, RADIUS, SEED);
        let mut stats = EffectStats::default();
        let out = rtao_pass(&inputs, 64, RADIUS, 0, SEED, &mut stats);
        assert_eq!(
            out.data, reference,
            "frame 0 同 seed:GBuffer 驱动必须与顶点驱动参考逐位一致"
        );
        // 度量埋点:射线 = 有效像素 × 64;无效像素 + 有效像素 = 全像素。
        let valid = W * H - stats.invalid_pixels;
        assert_eq!(stats.rays, u64::from(valid) * 64);
        assert!(valid > 100, "墙角画面有效像素应充足,valid={valid}");
        // 墙角区应存在显著遮蔽像素(AO < 0.9),开阔区存在精确 1.0。
        let mut min_ao = 1.0f32;
        let mut any_exact_one = false;
        for (i, &v) in out.data.iter().enumerate() {
            if positions[i].iter().all(|c| c.is_finite()) {
                min_ao = min_ao.min(v);
                any_exact_one |= v == 1.0;
            }
        }
        assert!(min_ao < 0.9, "墙角应显著遮蔽,min_ao={min_ao}");
        assert!(any_exact_one, "开阔区应存在精确 1.0 像素");
        eprintln!(
            "[rtao_pointwise] valid={valid} invalid={} min_ao={min_ao}",
            stats.invalid_pixels
        );
    }

    #[test]
    fn rtao_converges_toward_high_spp_reference() {
        // **RTAO 对拍金标准(收敛臂)**:独立 RNG 流下,spp=64 的 GBuffer 驱动
        // 输出必须收敛到高 spp(2048)参考均值 μ̂。统计口径(写明):逐像素随机
        // 误差 ~N(0, p(1−p)/spp),逐点硬阈在 p≈0.5 像素上统计不可靠,故以
        // 「全图均值绝对误差」为收敛度量(大数平均后稳健):spp 2→64 采样数
        // 32×,误差应 ~1/√32 ≈ 0.18,阈值取 0.35(约 2× 余量)。
        let (blases, tlas) = corner_open_scene();
        let vp = corner_camera();
        let (depth, normals) = gbuffer_from_scene(&vp, W, H, &tlas, &blases);
        let inputs = corner_inputs(&depth, &normals, vp, &tlas, &blases);
        let (positions, ref_normals) = gbuffer_point_arrays(&inputs);
        let mu = rtao_reference(
            &positions,
            &ref_normals,
            &tlas,
            &blases,
            2048,
            RADIUS,
            SEED + 1,
        );
        let mean_abs_err = |img: &ImageF32| {
            let (mut sum, mut cnt) = (0.0f64, 0u32);
            for (i, &v) in img.data.iter().enumerate() {
                if positions[i].iter().all(|c| c.is_finite()) {
                    sum += f64::from((v - mu[i]).abs());
                    cnt += 1;
                }
            }
            sum / f64::from(cnt)
        };
        let mut stats = EffectStats::default();
        let out64 = rtao_pass(&inputs, 64, RADIUS, 0, SEED + 2, &mut stats);
        let out2 = rtao_pass(&inputs, 2, RADIUS, 0, SEED + 3, &mut stats);
        let (e64, e2) = (mean_abs_err(&out64), mean_abs_err(&out2));
        assert!(e64 < 0.05, "spp=64 均值绝对误差应 < 0.05,e64={e64}");
        assert!(
            e64 < 0.35 * e2,
            "spp 2→64 应显著收敛:e64={e64} 应 < 0.35 × e2={e2}"
        );
        eprintln!(
            "[rtao_converge] e2={e2:.5} e64={e64:.5} ratio={:.3}",
            e64 / e2
        );
    }

    #[test]
    fn rtao_radius_bound_excludes_far_occluders() {
        // 半径界断言(远遮挡不计):专设已知距离场景——地板 + 单墙 x=2
        // (y∈[0,2], z∈[−2,2]),正俯视。radius=0.5:地板像素距墙 2−px ≥ 0.6
        // (死带 [0.4,0.6] 避开像素量化)→ 遮蔽不可能发生,AO 精确 1;距墙
        // ≤ 0.4 且 |pz| ≤ 1.5(墙 z 界内)→ 墙在 radius 内,必须遮蔽(AO<1)。
        let floor = quad_y_up(-4.0, 4.0, -4.0, 4.0, 0.0);
        let wall = quad_x_up(0.0, 2.0, -2.0, 2.0, 2.0);
        let (pos, idx) = merge(&[floor, wall]);
        let (blases, tlas) = scene_of(&pos, &idx);
        let vp = top_down_camera(5.0);
        let (depth, normals) = gbuffer_from_scene(&vp, 16, 16, &tlas, &blases);
        let inputs = corner_inputs(&depth, &normals, vp, &tlas, &blases);
        let inv_vp = vp.inverse().expect("可逆");
        let mut stats = EffectStats::default();
        let out = rtao_pass(&inputs, 16, 0.5, 0, SEED, &mut stats);
        let (mut far, mut near) = (0u32, 0u32);
        let mut near_ao_sum = 0.0f64;
        for y in 0..16 {
            for x in 0..16 {
                let d = depth.get(x, y, 0);
                if !valid_gbuffer_depth(d) {
                    continue;
                }
                let p = world_position_from_gbuffer(&inv_vp, x, y, 16, 16, d).expect("有限");
                if p.y.abs() > 0.1 {
                    continue; // 墙顶像素不参与
                }
                let dist = 2.0 - p.x;
                if dist >= 0.6 {
                    // 定理断言(非统计):任何命中需 t ≥ dist > radius + RAY_EPS,
                    // 遮蔽不可能发生,逐像素精确 1。
                    assert_eq!(
                        out.get(x, y, 0),
                        1.0,
                        "({x},{y}) 距墙 {dist} 超 radius=0.5,遮蔽不计"
                    );
                    far += 1;
                } else if dist <= 0.4 && p.z.abs() <= 1.5 {
                    // 墙在 radius 内:遮蔽率 ~10%(立体角小),逐像素全不命中
                    // 概率不可忽略 → 近带取均值统计断言(期望 ≈0.9,阈 0.98)。
                    near_ao_sum += f64::from(out.get(x, y, 0));
                    near += 1;
                }
            }
        }
        assert!(far > 0 && near > 0, "双向断言区必须有像素");
        let near_mean = near_ao_sum / f64::from(near);
        assert!(
            near_mean < 0.98,
            "近墙带均值 AO={near_mean} 应显著 < 1(radius 内遮蔽生效)"
        );
        eprintln!("[radius_bound] far={far} near={near} near_mean={near_mean:.4}");
        // 零采样退化:全 1 且零射线(与参考零采样约定一致)。
        let out0 = rtao_pass(&inputs, 0, RADIUS, 0, SEED, &mut stats);
        assert!(out0.data.iter().all(|&v| v == 1.0));
        assert_eq!(stats.rays, 0);
    }

    #[test]
    fn rtao_deterministic_and_frame_decorrelated() {
        // 同 seed 同 frame_index:逐像素逐位可复现。
        let (blases, tlas) = corner_open_scene();
        let vp = corner_camera();
        let (depth, normals) = gbuffer_from_scene(&vp, W, H, &tlas, &blases);
        let inputs = corner_inputs(&depth, &normals, vp, &tlas, &blases);
        let mut stats = EffectStats::default();
        let a = rtao_pass(&inputs, 8, RADIUS, 3, SEED, &mut stats);
        let b = rtao_pass(&inputs, 8, RADIUS, 3, SEED, &mut stats);
        assert_eq!(a.data, b.data, "同 (seed, frame_index) 必须逐位复现");
        // 异 frame_index:采样流去相关(时域滤波收敛前提)。
        let c = rtao_pass(&inputs, 8, RADIUS, 4, SEED, &mut stats);
        assert_ne!(a.data, c.data, "异 frame_index 采样流必须去相关");
        // frame_seed 对齐臂:frame 0 恒等。
        assert_eq!(frame_seed(SEED, 0), SEED);
        assert_ne!(frame_seed(SEED, 1), SEED);
    }

    #[test]
    fn gbuffer_roundtrip_world_position() {
        // GBuffer 重建约定锚定:depth 反投影世界位置,再经 view_proj 正投影,
        // 必须回到原 uv(±1e-4)与原 depth(±1e-4);天空像素 depth=1.0 且
        // 法线零长(无效像素约定)。
        let (blases, tlas) = corner_open_scene();
        let vp = corner_camera();
        let (depth, normals) = gbuffer_from_scene(&vp, W, H, &tlas, &blases);
        let inv_vp = vp.inverse().expect("可逆");
        let mut valid = 0u32;
        for y in 0..H {
            for x in 0..W {
                let d = depth.get(x, y, 0);
                let n = Vec3::from_array(normals.pixel3(x, y));
                if !valid_gbuffer_depth(d) {
                    assert_eq!(d, 1.0, "天空 depth 必须为 1.0");
                    assert_eq!(n.length(), 0.0, "天空法线必须为零向量");
                    continue;
                }
                valid += 1;
                let p = world_position_from_gbuffer(&inv_vp, x, y, W, H, d).expect("有限");
                let clip = vp.transform_vec4([p.x, p.y, p.z, 1.0]);
                let (cu, cv) = (clip[0] / clip[3], clip[1] / clip[3]);
                let u = (x as f32 + 0.5) / W as f32;
                let v = (y as f32 + 0.5) / H as f32;
                assert!((cu - (2.0 * u - 1.0)).abs() < 1e-4, "({x},{y}) ndc.x 往返");
                assert!((cv - (1.0 - 2.0 * v)).abs() < 1e-4, "({x},{y}) ndc.y 往返");
                assert!((clip[2] / clip[3] - d).abs() < 1e-4, "({x},{y}) depth 往返");
                assert!((n.length() - 1.0).abs() < 1e-4, "({x},{y}) 法线单位长");
            }
        }
        assert!(valid > 100, "valid={valid}");
    }

    #[test]
    fn rtao_sky_pixels_invalid() {
        // 手工 GBuffer:首行天空(depth=1.0、零法线)→ 输出 1.0、计无效、
        // 不发射射线;其余行有效。
        let (pos, idx) = quad_y_up(-4.0, 4.0, -4.0, 4.0, 0.0);
        let (blases, tlas) = scene_of(&pos, &idx);
        let vp = top_down_camera(5.0);
        let depth = ImageF32::from_fn(4, 4, 1, |_, y, _| if y == 0 { 1.0 } else { 0.5 });
        let normals = ImageF32::from_fn(4, 4, 3, |_, y, ch| {
            if y == 0 {
                0.0
            } else if ch == 1 {
                1.0
            } else {
                0.0
            }
        });
        let inputs = corner_inputs(&depth, &normals, vp, &tlas, &blases);
        let mut stats = EffectStats::default();
        let ao = rtao_pass(&inputs, 4, RADIUS, 0, SEED, &mut stats);
        assert_eq!(stats.invalid_pixels, 4, "首行天空 4 像素无效");
        assert_eq!(stats.rays, 12 * 4, "射线仅来自有效像素");
        for x in 0..4 {
            assert_eq!(ao.get(x, 0, 0), 1.0, "天空像素输出 1.0");
        }
        // 硬阴影同约定:天空 → 1.0 且不计射线。
        let vis = hard_shadow_pass(&inputs, [0.0, 1.0, 0.0], &mut stats);
        assert_eq!(stats.invalid_pixels, 4);
        assert_eq!(stats.rays, 12);
        for x in 0..4 {
            assert_eq!(vis.get(x, 0, 0), 1.0);
        }
    }

    const LIGHT: [f32; 3] = [-0.5, 1.0, 0.0];

    /// 遮挡板场景:地板 y=0(x,z ∈ [−4,4])接收面 + 板 y=1(x,z ∈ [0,1])。
    fn plate_scene() -> (Vec<TriBvh>, Tlas) {
        let floor = quad_y_up(-4.0, 4.0, -4.0, 4.0, 0.0);
        let plate = quad_y_up(0.0, 1.0, 0.0, 1.0, 1.0);
        let (pos, idx) = merge(&[floor, plate]);
        scene_of(&pos, &idx)
    }

    #[test]
    fn hard_shadow_pointwise_matches_reference() {
        // **硬阴影对拍金标准**:无 RNG,同一 GBuffer 反投影位置 + 同一
        // RAY_EPS 纪律 → 与 hard_shadow_reference 逐像素位级一致。
        let (blases, tlas) = plate_scene();
        let vp = top_down_camera(6.0);
        let (depth, normals) = gbuffer_from_scene(&vp, 16, 16, &tlas, &blases);
        let inputs = corner_inputs(&depth, &normals, vp, &tlas, &blases);
        let (positions, _) = gbuffer_point_arrays(&inputs);
        let reference = hard_shadow_reference(&positions, LIGHT, &tlas, &blases);
        let mut stats = EffectStats::default();
        let vis = hard_shadow_pass(&inputs, LIGHT, &mut stats);
        assert_eq!(vis.data, reference, "硬阴影必须与参考逐像素一致");
        assert_eq!(stats.rays, 16 * 16, "全画面有效(正俯视无天空)");
        assert_eq!(stats.invalid_pixels, 0);
    }

    #[test]
    fn hard_shadow_analytic_boundary() {
        // 遮挡板投影边界:光方向 normalize(−0.5,1,0),地板点 (px,0,pz) 的阴影
        // 光线在板高度处 x_at = px − 0.5;x_at ∈ [0,1] 且 pz ∈ [0,1] → 影。
        // 相机偏置 +x 上方(正俯视正中时阴影区会被板自身遮挡,断言区无像素):
        // 阴影区 px > 0.8 段对相机可见。边界带 ±0.4(> 像素世界足印 0.36)内
        // 不断言,带外必须精确匹配;断言域只含地板像素(p.y≈0,板顶像素 lit
        // 属边界带内,不参与)。
        let (blases, tlas) = plate_scene();
        let proj = perspective_rh_zo(0.9, 1.0, 0.1, 50.0);
        let view = look_at_rh([2.0, 6.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, -1.0]);
        let vp = proj.mul(&view);
        let (depth, normals) = gbuffer_from_scene(&vp, 16, 16, &tlas, &blases);
        let inputs = corner_inputs(&depth, &normals, vp, &tlas, &blases);
        let mut stats = EffectStats::default();
        let vis = hard_shadow_pass(&inputs, LIGHT, &mut stats);
        let inv_vp = vp.inverse().expect("可逆");
        let (mut shadowed, mut lit_outside) = (0u32, 0u32);
        for y in 0..16 {
            for x in 0..16 {
                let p = world_position_from_gbuffer(&inv_vp, x, y, 16, 16, depth.get(x, y, 0))
                    .expect("有限");
                if p.y.abs() > 0.1 {
                    continue; // 板顶像素:lit 且落在边界带内,不参与断言
                }
                let v = vis.get(x, y, 0);
                let in_x = (0.5 + 0.4..=1.5 - 0.4).contains(&p.x);
                let in_z = (0.0 + 0.4..=1.0 - 0.4).contains(&p.z);
                let out_x = !(0.5 - 0.4..=1.5 + 0.4).contains(&p.x);
                let out_z = !(0.0 - 0.4..=1.0 + 0.4).contains(&p.z);
                if in_x && in_z {
                    assert_eq!(v, 0.0, "({x},{y}) p={p:?} 投影板内必须影");
                    shadowed += 1;
                } else if out_x || out_z {
                    assert_eq!(v, 1.0, "({x},{y}) p={p:?} 投影板外必须亮");
                    lit_outside += 1;
                }
            }
        }
        assert!(shadowed > 0 && lit_outside > 0, "断言区必须双向有像素");
        eprintln!("[shadow_boundary] shadowed={shadowed} lit={lit_outside}");
    }

    #[test]
    fn hard_shadow_no_self_shadow_on_receiver() {
        // 无遮挡物接收面:全亮无噪点(RAY_EPS 自交纪律:沿光方向偏移后接收面
        // 自身命中 t < 0 不被接受);零长光方向退化全 1 且零射线。
        let (pos, idx) = quad_y_up(-4.0, 4.0, -4.0, 4.0, 0.0);
        let (blases, tlas) = scene_of(&pos, &idx);
        let vp = top_down_camera(6.0);
        let (depth, normals) = gbuffer_from_scene(&vp, 16, 16, &tlas, &blases);
        let inputs = corner_inputs(&depth, &normals, vp, &tlas, &blases);
        let mut stats = EffectStats::default();
        let vis = hard_shadow_pass(&inputs, LIGHT, &mut stats);
        assert!(vis.data.iter().all(|&v| v == 1.0), "接收面全亮无噪点");
        assert_eq!(stats.rays, 16 * 16);
        let vis0 = hard_shadow_pass(&inputs, [0.0, 0.0, 0.0], &mut stats);
        assert!(vis0.data.iter().all(|&v| v == 1.0), "零长光方向退化全 1");
        assert_eq!(stats.rays, 0, "退化方向不发射射线");
    }

    #[test]
    #[should_panic(expected = "EffectInputs")]
    fn effect_inputs_shape_validation_panics() {
        let (pos, idx) = quad_y_up(-1.0, 1.0, -1.0, 1.0, 0.0);
        let (blases, tlas) = scene_of(&pos, &idx);
        let vp = top_down_camera(2.0);
        let depth = ImageF32::new(4, 4, 1);
        let bad_normals = ImageF32::new(4, 4, 1); // 通道违约:必须 3 通道
        let _ = EffectInputs::new(&depth, &bad_normals, vp, &tlas, &blases);
    }
}
