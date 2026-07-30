//! 探针放置与追踪(报告2 §3.1「均匀网格屏幕探针 ~1/16 分辨率、每探针少量
//! 光线」;RFC-0016 章 E1)。
//!
//! - [`place_probes`]:每 `cell×cell`(默认 4×4 = 1/16)像素块一枚探针,锚在
//!   块中心像素,经相机逆投影得世界位置 + 法线;无几何像素(深度为远平面/非
//!   有限/零法线)标 `valid = false`;
//! - [`trace_probes`]:每探针 N 条(默认 16,可配)法线半球**余弦加权**方向
//!   (BRDF PDF 单因子重要性采样,RFC 章 E1),经 [`RadianceTracer`] 统一契约
//!   收辐射度;RNG = [`Pcg32`] 固定种子 + 探针索引去相关,全管线确定性。

use crate::gi::tracer::RadianceTracer;
use crate::rt::bvh::Vec3;
use crate::rt::ref_tracer::{Pcg32, RAY_EPS};
use crate::temporal::common::Mat4;
use crate::temporal::image::ImageF32;

/// 默认探针块边长(每 4×4 像素一探针 = 1/16,报告2 §3.1)。
pub const DEFAULT_PROBE_CELL: u32 = 4;
/// 默认每探针光线数(Lumen/GI-1.0 屏幕探针同量级;可配)。
pub const DEFAULT_RAYS_PER_PROBE: u32 = 16;

/// GI 相机(view_proj 及其逆;深度约定 = NDC z ∈ [0,1] ZO,与
/// [`crate::temporal::common::perspective_rh_zo`] / `compute_camera_mv` 同口径)。
#[derive(Debug, Clone, Copy)]
pub struct GiCamera {
    /// 当前帧 view_proj。
    pub view_proj: Mat4,
    /// 其逆(反投影用)。
    pub inv_view_proj: Mat4,
}

impl GiCamera {
    /// 由 view_proj 构造(奇异矩阵 panic——相机矩阵必须可逆,调用契约)。
    pub fn new(view_proj: Mat4) -> GiCamera {
        let inv = view_proj.inverse().expect("GiCamera: view_proj 必须可逆");
        GiCamera {
            view_proj,
            inv_view_proj: inv,
        }
    }
}

/// 像素反投影:`(px, py)` 像素中心 + NDC 深度 → 世界位置。
///
/// `depth` 为 NDC z ∈ [0,1](ZO);齐次 w 失效(非有限/≈0)产 `None`。
pub fn back_project(
    camera: &GiCamera,
    px: u32,
    py: u32,
    w: u32,
    h: u32,
    depth: f32,
) -> Option<Vec3> {
    let u = (px as f32 + 0.5) / w as f32;
    let v = (py as f32 + 0.5) / h as f32;
    let ndc = [2.0 * u - 1.0, 1.0 - 2.0 * v, depth, 1.0];
    let world4 = camera.inv_view_proj.transform_vec4(ndc);
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

/// 单个探针(屏幕锚点 + 世界锚点;`valid = false` = 无几何像素占位)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Probe {
    /// 锚定像素坐标(cell 中心像素,边缘截断到屏内)。
    pub anchor: [u32; 2],
    /// 世界位置(反投影;invalid 探针为零向量)。
    pub pos: Vec3,
    /// 世界法线(单位长,自 GBuffer 归一化;invalid 探针为零向量)。
    pub normal: Vec3,
    /// 锚点 NDC 深度(探针空间滤波深度相似性度量基准)。
    pub depth: f32,
    /// 有效旗标(false ⇒ 追踪/滤波/插值全部跳过)。
    pub valid: bool,
}

/// 探针网格(行主序 `w×h`;覆盖整块屏幕,边缘不足一格仍有一探针)。
#[derive(Debug, Clone)]
pub struct ProbeGrid {
    /// 探针网格宽(每 `cell` 像素一枚,向上取整)。
    pub w: u32,
    /// 探针网格高。
    pub h: u32,
    /// 探针块边长(像素)。
    pub cell: u32,
    /// 全屏分辨率 `[w, h]`。
    pub screen: [u32; 2],
    /// 探针数组(行主序,长度 `w*h`)。
    pub probes: Vec<Probe>,
}

impl ProbeGrid {
    /// 按网格坐标取探针。
    pub fn at(&self, i: u32, j: u32) -> &Probe {
        &self.probes[(j * self.w + i) as usize]
    }

    /// 探针 `(i, j)` 锚点的全屏 uv(时域 MV/历史采集与放置共用同一锚点定义)。
    pub fn anchor_uv(&self, i: u32, j: u32) -> [f32; 2] {
        let p = self.at(i, j);
        [
            (p.anchor[0] as f32 + 0.5) / self.screen[0] as f32,
            (p.anchor[1] as f32 + 0.5) / self.screen[1] as f32,
        ]
    }

    /// 有效探针数。
    pub fn valid_count(&self) -> usize {
        self.probes.iter().filter(|p| p.valid).count()
    }
}

/// 探针放置:均匀网格锚在块中心像素,反投影得世界位置 + 法线。
///
/// 无效判据(与 [`crate::rt::ref_tracer`] 无几何像素口径一致):深度非有限或
/// ≥ 1.0(远平面 = 天空)、法线非有限或零长、反投影齐次 w 失效。
pub fn place_probes(
    depth: &ImageF32,
    normals: &ImageF32,
    camera: &GiCamera,
    cell: u32,
) -> ProbeGrid {
    assert!(cell >= 1, "place_probes: cell 必须 ≥1");
    assert_eq!(depth.c, 1, "place_probes: 深度图必须单通道");
    assert!(
        normals.c == 3 && normals.w == depth.w && normals.h == depth.h,
        "place_probes: 法线图形状与深度图不符"
    );
    let (w, h) = (depth.w, depth.h);
    let gw = w.div_ceil(cell);
    let gh = h.div_ceil(cell);
    let mut probes = Vec::with_capacity((gw * gh) as usize);
    for j in 0..gh {
        for i in 0..gw {
            let ax = (i * cell + cell / 2).min(w - 1);
            let ay = (j * cell + cell / 2).min(h - 1);
            let d = depth.get(ax, ay, 0);
            let n = Vec3::from_array(normals.pixel3(ax, ay));
            let valid = d.is_finite()
                && d < 1.0
                && n.is_finite()
                && n.length() > 0.0
                && back_project(camera, ax, ay, w, h, d).is_some();
            let (pos, normal) = if valid {
                let p = back_project(camera, ax, ay, w, h, d).expect("已校验");
                (p, n.normalize())
            } else {
                (Vec3::ZERO, Vec3::ZERO)
            };
            probes.push(Probe {
                anchor: [ax, ay],
                pos,
                normal,
                depth: if valid { d } else { 1.0 },
                valid,
            });
        }
    }
    ProbeGrid {
        w: gw,
        h: gh,
        cell,
        screen: [w, h],
        probes,
    }
}

/// 单探针采样结果(方向与辐射度一一对应;无效探针/零采样 = 空)。
#[derive(Debug, Clone, Default)]
pub struct ProbeSamples {
    /// 采样方向(世界空间,单位长,法线半球余弦加权)。
    pub dirs: Vec<Vec3>,
    /// 对应方向收得的辐射度。
    pub radiance: Vec<[f32; 3]>,
}

/// 探针种子去相关:base seed XOR 探针索引 × 黄金比例乘子(2⁶⁴ 奇数乘子,
/// 相邻探针序列位级不同;同 `(seed, index)` 恒同序列,跨平台确定性)。
pub fn probe_seed(seed: u64, probe_index: u32) -> u64 {
    seed ^ (u64::from(probe_index).wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// 法线半球余弦加权采样方向(单位长;`r1, r2 ∈ [0,1)` 均匀输入)。
/// 算法与 [`crate::rt::ref_tracer`] 内部采样同型(该处为私有实现,本处为探针
/// 管线自带;`pdf(ω) = (n·ω)/π`,z = √(1−r2))。
pub fn cosine_sample_hemisphere(n: Vec3, r1: f32, r2: f32) -> Vec3 {
    let (t, b) = orthonormal_basis(n);
    let phi = 2.0 * core::f32::consts::PI * r1;
    let r = r2.sqrt();
    let x = r * phi.cos();
    let y = r * phi.sin();
    let z = (1.0 - r2).max(0.0).sqrt();
    (t * x + b * y + n * z).normalize()
}

/// 由法线构造正交基(确定性分支;`t ⟂ n`,`b = n × t` 已单位长)。
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

/// 全探针追踪:每有效探针 `rays_per_probe` 条余弦加权方向经 [`RadianceTracer`]
/// 收辐射度;射线原点沿探针法线偏移 [`RAY_EPS`] 防自交。
///
/// 确定性承诺:探针按网格行主序处理,每探针独立 [`Pcg32`] 流
/// ([`probe_seed`] 去相关),同 `(seed, 输入)` 产逐元素相同输出。
pub fn trace_probes(
    grid: &ProbeGrid,
    tracer: &dyn RadianceTracer,
    rays_per_probe: u32,
    seed: u64,
) -> Vec<ProbeSamples> {
    grid.probes
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            if !p.valid || rays_per_probe == 0 {
                return ProbeSamples::default();
            }
            let mut rng = Pcg32::new(probe_seed(seed, idx as u32));
            let origin = p.pos + p.normal * RAY_EPS;
            let mut dirs = Vec::with_capacity(rays_per_probe as usize);
            let mut radiance = Vec::with_capacity(rays_per_probe as usize);
            for _ in 0..rays_per_probe {
                let r1 = rng.next_f32();
                let r2 = rng.next_f32();
                let dir = cosine_sample_hemisphere(p.normal, r1, r2);
                dirs.push(dir);
                radiance.push(tracer.trace(origin, dir));
            }
            ProbeSamples { dirs, radiance }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::common::{look_at_rh, perspective_rh_zo};

    /// 测试相机:原点看向 −z(视图 = 恒等),fov 90°,ZO 深度 [0.1, 100]。
    fn test_camera() -> GiCamera {
        let proj = perspective_rh_zo(core::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let view = look_at_rh([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]);
        GiCamera::new(proj.mul(&view))
    }

    /// 独立路径锚定:fov 90° 透视下 NDC (nx, ny, d) 的视空间位置(f64 手算,
    /// 不经 Mat4 逆,与实现路径互验)。
    fn analytic_view_pos(nx: f64, ny: f64, d: f64) -> [f64; 3] {
        let (n, f) = (0.1f64, 100.0f64);
        let m22 = f / (n - f);
        let m23 = n * f / (n - f);
        // ndc.z = (m22·z + m23)/(−z) ⇒ z = m23/(−ndc − m22)
        let z = m23 / (-d - m22);
        // fov 90° ⇒ m00 = m11 = 1:x_v = ndc.x·(−z),y_v = ndc.y·(−z)
        [nx * (-z), ny * (-z), z]
    }

    #[test]
    fn place_probes_grid_and_backprojection_anchored() {
        let cam = test_camera();
        let depth = ImageF32::from_fn(64, 64, 1, |_, _, _| 0.5);
        let normals = ImageF32::from_fn(64, 64, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        let grid = place_probes(&depth, &normals, &cam, 4);
        // 64×64 / 4×4 ⇒ 16×16 探针;锚定像素 = 块中心 (i·4+2, j·4+2)。
        assert_eq!((grid.w, grid.h), (16, 16));
        assert_eq!(grid.at(0, 0).anchor, [2, 2]);
        assert_eq!(grid.at(1, 0).anchor, [6, 2]);
        assert_eq!(grid.at(15, 15).anchor, [62, 62]);
        assert_eq!(grid.valid_count(), 256);
        // 反投影锚定:uv = (2.5/64, 2.5/64) ⇒ ndc = (−0.921875, 0.921875, 0.5)。
        let expect = analytic_view_pos(-0.921875, 0.921875, 0.5);
        let p = grid.at(0, 0).pos;
        for (k, (got, exp)) in [p.x, p.y, p.z].iter().zip(expect.iter()).enumerate() {
            assert!(
                (f64::from(*got) - exp).abs() < 1e-3,
                "probe(0,0) 分量{k}: {got} vs 手算 {exp}"
            );
        }
        // 对角锚定:uv = (62.5/64, 62.5/64) ⇒ ndc = (0.953125, −0.953125, 0.5)。
        let expect = analytic_view_pos(0.953125, -0.953125, 0.5);
        let p = grid.at(15, 15).pos;
        for (k, (got, exp)) in [p.x, p.y, p.z].iter().zip(expect.iter()).enumerate() {
            assert!(
                (f64::from(*got) - exp).abs() < 1e-3,
                "probe(15,15) 分量{k}: {got} vs 手算 {exp}"
            );
        }
        assert_eq!(grid.at(0, 0).normal, Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn place_probes_invalid_depth_marked() {
        let cam = test_camera();
        let mut depth = ImageF32::from_fn(16, 16, 1, |_, _, _| 0.5);
        let mut normals = ImageF32::from_fn(16, 16, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        // cell=16 ⇒ 单探针,依次注入四类无效。
        let grid = place_probes(&depth, &normals, &cam, 16);
        assert!(grid.at(0, 0).valid, "基准应有效");
        depth.set(8, 8, 0, 1.0); // 远平面 = 天空
        let grid = place_probes(&depth, &normals, &cam, 16);
        assert!(!grid.at(0, 0).valid, "深度 1.0(远平面天空)应无效");
        depth.set(8, 8, 0, f32::NAN);
        let grid = place_probes(&depth, &normals, &cam, 16);
        assert!(!grid.at(0, 0).valid, "NaN 深度应无效");
        depth.set(8, 8, 0, 0.5);
        normals.set_pixel3(8, 8, [0.0, 0.0, 0.0]);
        let grid = place_probes(&depth, &normals, &cam, 16);
        assert!(!grid.at(0, 0).valid, "零长法线应无效");
        // 无效探针占位语义:零位置/零法线,不污染后续阶段。
        let p = *grid.at(0, 0);
        assert_eq!(p.pos, Vec3::ZERO);
        assert_eq!(p.normal, Vec3::ZERO);
    }

    #[test]
    fn trace_probes_deterministic_hemisphere_decorrelated() {
        // 单四边形场景(命中/未命中都存在),固定种子。
        use crate::gi::tracer::{GiMeshInstance, GiScene, RayTracedRadiance};
        use crate::rt::bvh::Transform3x4;
        let mesh = GiMeshInstance {
            positions: vec![
                [-2.0, -2.0, -3.0],
                [2.0, -2.0, -3.0],
                [2.0, 2.0, -3.0],
                [-2.0, 2.0, -3.0],
            ],
            indices: vec![[0, 1, 2], [0, 2, 3]],
            transform: Transform3x4::IDENTITY,
            albedo: [0.7, 0.7, 0.7],
        };
        let tracer =
            RayTracedRadiance::new(GiScene::build(&[mesh], [0.0, 0.0, 1.0], [2.0; 3], [0.3; 3]));
        let cam = test_camera();
        let depth = ImageF32::from_fn(16, 16, 1, |x, _, _| if x < 8 { 0.99 } else { 1.0 });
        let normals = ImageF32::from_fn(16, 16, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        let grid = place_probes(&depth, &normals, &cam, 4);
        assert_eq!(grid.valid_count(), 8, "右半深度 1.0 应全部无效");
        let a = trace_probes(&grid, &tracer, 8, 42);
        let b = trace_probes(&grid, &tracer, 8, 42);
        for (sa, sb) in a.iter().zip(b.iter()) {
            assert_eq!(sa.dirs, sb.dirs, "同种子方向序列应位级一致");
            assert_eq!(sa.radiance, sb.radiance, "同种子辐射度应位级一致");
        }
        // 有效探针:8 条方向全部落在法线半球内(dot(n, ω) > 0)。
        let p0 = grid.at(0, 0);
        for d in &a[0].dirs {
            assert!(d.dot(p0.normal) > 0.0, "方向应在探针法线半球内");
            assert!((d.length() - 1.0).abs() < 1e-5, "方向应单位长");
        }
        // 探针间去相关:相邻有效探针方向序列位级不同。
        let idx1 = grid
            .probes
            .iter()
            .position(|p| p.valid)
            .expect("有有效探针");
        let idx2 = idx1 + 1;
        assert_ne!(a[idx1].dirs, a[idx2].dirs, "相邻探针采样序列应去相关");
        // 无效探针与零采样:空样本。
        let invalid = grid
            .probes
            .iter()
            .position(|p| !p.valid)
            .expect("有无效探针");
        assert!(a[invalid].dirs.is_empty() && a[invalid].radiance.is_empty());
        let zero = trace_probes(&grid, &tracer, 0, 42);
        assert!(zero.iter().all(|s| s.dirs.is_empty()));
    }
}
