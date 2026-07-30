//! GI 单反弹闭环管线组装 + host 对拍工具(报告2 §3.1 MVP 定义;RFC-0016 章
//! E1/E3)。
//!
//! 管线顺序 = [`place_probes`] → [`trace_probes`] → [`project_all`] →(可关)
//! [`filter_probes_3x3`] → [`interpolate`] →(可关)[`temporal_accumulate`],
//! 输出全屏 `ImageF32`(3ch)间接漫反射 irradiance。**关滤波关累积仅单反弹**
//! 即 RFC 章 E3 能量守恒检查口径(白炉单测锚定)。
//!
//! host 对拍工具(G-G5-6「device 真跑与 CPU 参考追踪器方向一致性对拍」的
//! host 侧口径,亦为 W3 device 腿的逐字语义蓝本):
//! - [`render_gbuffer_pinhole`]:针孔相机 GBuffer 合成(深度/法线);
//! - [`irradiance_bruteforce_reference`]:逐像素蛮力 irradiance 参考(与探针
//!   管线同一 [`RadianceTracer`],采样预算独立)。

use crate::gi::filter::{FilterParams, filter_probes_3x3};
use crate::gi::interpolate::{DEFAULT_PLANE_SCALE, interpolate};
use crate::gi::probe::{
    DEFAULT_PROBE_CELL, DEFAULT_RAYS_PER_PROBE, GiCamera, ProbeGrid, back_project,
    cosine_sample_hemisphere, place_probes, probe_seed, trace_probes,
};
use crate::gi::sh::{ShL1Rgb, project_all};
use crate::gi::temporal::{GiFrame, GiHistory, TemporalParams, temporal_accumulate};
use crate::gi::tracer::{GiScene, RadianceTracer};
use crate::rt::bvh::{Ray, Vec3};
use crate::rt::ref_tracer::{Pcg32, RAY_EPS};
use crate::temporal::image::ImageF32;

/// GI 管线参数(全部确定性:固定种子 ⇒ 同输入同输出)。
#[derive(Debug, Clone)]
pub struct GiParams {
    /// 探针块边长(像素;4 = 1/16,报告2 §3.1)。
    pub cell: u32,
    /// 每探针光线数。
    pub rays_per_probe: u32,
    /// 采样种子(Pcg32 固定种子 + 探针索引去相关)。
    pub seed: u64,
    /// 探针空间 3×3 滤波开关。
    pub filter: bool,
    /// 滤波参数。
    pub filter_params: FilterParams,
    /// 时域累积开关(需调用方同时提供历史与 MV 才生效)。
    pub temporal: bool,
    /// 时域累积参数。
    pub temporal_params: TemporalParams,
    /// 平面权重尺度(世界单位)。
    pub plane_scale: f32,
}

impl Default for GiParams {
    fn default() -> Self {
        GiParams {
            cell: DEFAULT_PROBE_CELL,
            rays_per_probe: DEFAULT_RAYS_PER_PROBE,
            seed: 0x5255_5258_4749_0001, // "RURXGI"+1:固定默认种子
            filter: true,
            filter_params: FilterParams::default(),
            temporal: true,
            temporal_params: TemporalParams::default(),
            plane_scale: DEFAULT_PLANE_SCALE,
        }
    }
}

/// GI 单帧输出。
#[derive(Debug, Clone)]
pub struct GiOutput {
    /// 全屏间接漫反射 irradiance(3ch;无效像素 = 0)。
    pub irradiance: ImageF32,
    /// 探针网格(放置结果)。
    pub probes: ProbeGrid,
    /// 探针 SH(滤波/累积后,与 probes 对齐)。
    pub probe_sh: Vec<ShL1Rgb>,
    /// 本帧历史输出(调用方图外持有,双缓冲轮换;下一帧经 `history` 回喂)。
    pub history: GiHistory,
}

/// GI 单反弹闭环(host 完整实现;device 腿 W3 接线,语义以本函数为蓝本)。
///
/// `history` = 上一帧 [`GiOutput::history`](跨帧外部资源双缓冲);`mv` = 相机
/// motion vectors([`crate::temporal::common::compute_camera_mv`] 产出)。
/// 二者齐备且 `params.temporal` 开才走累积,否则历史仅做当前帧快照。
///
/// # Panics
/// 深度/法线图形状不符(c1/c3、同分辨率)即 panic,调用契约。
pub fn render_gi(
    depth: &ImageF32,
    normals: &ImageF32,
    camera: &GiCamera,
    tracer: &dyn RadianceTracer,
    history: Option<&GiHistory>,
    mv: Option<&ImageF32>,
    params: &GiParams,
) -> GiOutput {
    assert_eq!(depth.c, 1, "render_gi: 深度图必须单通道");
    assert!(
        normals.c == 3 && normals.w == depth.w && normals.h == depth.h,
        "render_gi: 法线图形状与深度图不符"
    );
    let grid = place_probes(depth, normals, camera, params.cell);
    let samples = trace_probes(&grid, tracer, params.rays_per_probe, params.seed);
    let mut shs = project_all(&grid, &samples);
    if params.filter {
        shs = filter_probes_3x3(&grid, &shs, &params.filter_params);
    }
    let mut irradiance = interpolate(&grid, &shs, depth, normals, camera, params.plane_scale);
    let out_history = match (history, mv) {
        (Some(h), Some(m)) if params.temporal => {
            let frame = GiFrame {
                grid: &grid,
                shs: &shs,
                irradiance: &irradiance,
                depth,
                normals,
            };
            let (s, i, nh) = temporal_accumulate(&frame, h, m, &params.temporal_params);
            shs = s;
            irradiance = i;
            nh
        }
        _ => GiHistory::from_frame(&grid, &shs, &irradiance, depth, normals),
    };
    GiOutput {
        irradiance,
        probes: grid,
        probe_sh: shs,
        history: out_history,
    }
}

/// 针孔相机 GBuffer 合成(host 对拍/单测工具)。
///
/// 每像素自近平面发一条光线:命中 ⇒ 深度 = 命中点 NDC z ∈ \[0,1\],法线 =
/// 朝向相机的双面着色几何法线;未命中 ⇒ 深度 1.0 + 零法线(与
/// [`place_probes`] 无效口径一致)。
pub fn render_gbuffer_pinhole(
    scene: &GiScene,
    camera: &GiCamera,
    w: u32,
    h: u32,
) -> (ImageF32, ImageF32) {
    let mut depth = ImageF32::new(w, h, 1);
    let mut normals = ImageF32::new(w, h, 3);
    let unproject = |nx: f32, ny: f32, z: f32| -> Option<Vec3> {
        let v4 = camera.inv_view_proj.transform_vec4([nx, ny, z, 1.0]);
        if !v4[3].is_finite() || v4[3].abs() < 1e-8 {
            return None;
        }
        Some(Vec3::new(v4[0] / v4[3], v4[1] / v4[3], v4[2] / v4[3]))
    };
    for y in 0..h {
        for x in 0..w {
            let u = (x as f32 + 0.5) / w as f32;
            let v = (y as f32 + 0.5) / h as f32;
            let (nx, ny) = (2.0 * u - 1.0, 1.0 - 2.0 * v);
            let (Some(p0), Some(p1)) = (unproject(nx, ny, 0.0), unproject(nx, ny, 1.0)) else {
                depth.set(x, y, 0, 1.0);
                continue;
            };
            let dir = (p1 - p0).normalize();
            if dir.length() == 0.0 {
                depth.set(x, y, 0, 1.0);
                continue;
            }
            let Some(hit) = scene
                .tlas
                .intersect(&scene.blases, &Ray { origin: p0, dir })
            else {
                depth.set(x, y, 0, 1.0);
                continue;
            };
            let p = p0 + dir * hit.t;
            let clip = camera.view_proj.transform_vec4([p.x, p.y, p.z, 1.0]);
            if clip[3] <= 1e-8 {
                depth.set(x, y, 0, 1.0);
                continue;
            }
            depth.set(x, y, 0, clip[2] / clip[3]);
            let mut n = Vec3::from_array(hit.normal);
            if n.dot(dir) > 0.0 {
                n = -n;
            }
            normals.set_pixel3(x, y, n.normalize().to_array());
        }
    }
    (depth, normals)
}

/// 逐像素蛮力 irradiance 参考(G-G5-6 方向一致性对拍 host 侧口径)。
///
/// 估计量推导:`E(x) = ∫_hemi L(ω)·(n·ω) dω`,ω ~ 余弦 pdf `(n·ω)/π`
/// ⇒ `E ≈ (π/N)Σᵢ L(ωᵢ)`(cos 项与 pdf 精确相消,无 pdf 护栏偏差)——与探针
/// 管线共用同一 [`RadianceTracer`],采样预算独立(对拍取 4× 探针预算)。
/// 无效像素/零采样 ⇒ 输出 0。
pub fn irradiance_bruteforce_reference(
    depth: &ImageF32,
    normals: &ImageF32,
    camera: &GiCamera,
    tracer: &dyn RadianceTracer,
    spp: u32,
    seed: u64,
) -> ImageF32 {
    assert_eq!(depth.c, 1, "bruteforce: 深度图必须单通道");
    assert!(
        normals.c == 3 && normals.w == depth.w && normals.h == depth.h,
        "bruteforce: 法线图形状与深度图不符"
    );
    let (w, h) = (depth.w, depth.h);
    let mut out = ImageF32::new(w, h, 3);
    if spp == 0 {
        return out;
    }
    for y in 0..h {
        for x in 0..w {
            let d = depth.get(x, y, 0);
            let n = Vec3::from_array(normals.pixel3(x, y));
            if !d.is_finite() || d >= 1.0 || !n.is_finite() || n.length() == 0.0 {
                continue;
            }
            let n = n.normalize();
            let Some(pos) = back_project(camera, x, y, w, h, d) else {
                continue;
            };
            let idx = y * w + x;
            let mut rng = Pcg32::new(probe_seed(seed, idx));
            let origin = pos + n * RAY_EPS;
            let mut acc = [0.0f64; 3];
            for _ in 0..spp {
                let r1 = rng.next_f32();
                let r2 = rng.next_f32();
                let dir = cosine_sample_hemisphere(n, r1, r2);
                let rad = tracer.trace(origin, dir);
                for ch in 0..3 {
                    acc[ch] += f64::from(rad[ch]);
                }
            }
            let scale = core::f64::consts::PI / f64::from(spp);
            out.set_pixel3(
                x,
                y,
                [
                    (acc[0] * scale) as f32,
                    (acc[1] * scale) as f32,
                    (acc[2] * scale) as f32,
                ],
            );
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gi::tracer::{GiMeshInstance, RayTracedRadiance};
    use crate::rt::bvh::Transform3x4;
    use crate::temporal::common::{
        compute_camera_mv, look_at_rh, perspective_rh_zo, validate_history_with_mv,
    };

    /// 以法线方向参考自动修正 winding 的四边形压入辅助(顶点顺序不定时用)。
    fn push_quad(
        positions: &mut Vec<[f32; 3]>,
        indices: &mut Vec<[u32; 3]>,
        corners: [[f32; 3]; 4],
        normal_hint: [f32; 3],
    ) {
        let base = positions.len() as u32;
        let a = Vec3::from_array(corners[0]);
        let b = Vec3::from_array(corners[1]);
        let c = Vec3::from_array(corners[2]);
        let n = (b - a).cross(c - a);
        let hint = Vec3::from_array(normal_hint);
        positions.extend_from_slice(&corners);
        if n.dot(hint) >= 0.0 {
            indices.push([base, base + 1, base + 2]);
            indices.push([base, base + 2, base + 3]);
        } else {
            indices.push([base, base + 2, base + 1]);
            indices.push([base, base + 3, base + 2]);
        }
    }

    fn mesh_of(
        positions: Vec<[f32; 3]>,
        indices: Vec<[u32; 3]>,
        albedo: [f32; 3],
    ) -> GiMeshInstance {
        GiMeshInstance {
            positions,
            indices,
            transform: Transform3x4::IDENTITY,
            albedo,
        }
    }

    /// 开放白炉场景:巨大白地板(albedo 1)+ 单位天光,方向光关闭。
    fn open_furnace_scene() -> GiScene {
        let mut pos = Vec::new();
        let mut idx = Vec::new();
        push_quad(
            &mut pos,
            &mut idx,
            [
                [-50.0, 0.0, -50.0],
                [50.0, 0.0, -50.0],
                [50.0, 0.0, 50.0],
                [-50.0, 0.0, 50.0],
            ],
            [0.0, 1.0, 0.0],
        );
        GiScene::build(
            &[mesh_of(pos, idx, [1.0, 1.0, 1.0])],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        )
    }

    /// 封闭白盒:六面内壁(albedo 1,边缘 0.05 重叠防接缝泄漏)+ 方向光开启
    /// (内壁全部阴影遮蔽)+ 单位天光(几何不可达)。
    fn closed_box_scene() -> GiScene {
        let (mut pos, mut idx) = (Vec::new(), Vec::new());
        let e = 2.05f32; // 壁面延伸 0.05 重叠
        push_quad(
            &mut pos,
            &mut idx,
            [[-e, -2.0, -e], [e, -2.0, -e], [e, -2.0, e], [-e, -2.0, e]],
            [0.0, 1.0, 0.0],
        );
        push_quad(
            &mut pos,
            &mut idx,
            [[-e, 2.0, -e], [e, 2.0, -e], [e, 2.0, e], [-e, 2.0, e]],
            [0.0, -1.0, 0.0],
        );
        push_quad(
            &mut pos,
            &mut idx,
            [[-2.0, -e, -e], [-2.0, -e, e], [-2.0, e, e], [-2.0, e, -e]],
            [1.0, 0.0, 0.0],
        );
        push_quad(
            &mut pos,
            &mut idx,
            [[2.0, -e, -e], [2.0, -e, e], [2.0, e, e], [2.0, e, -e]],
            [-1.0, 0.0, 0.0],
        );
        push_quad(
            &mut pos,
            &mut idx,
            [[-e, -e, -2.0], [e, -e, -2.0], [e, e, -2.0], [-e, e, -2.0]],
            [0.0, 0.0, 1.0],
        );
        push_quad(
            &mut pos,
            &mut idx,
            [[-e, -e, 2.0], [e, -e, 2.0], [e, e, 2.0], [-e, e, 2.0]],
            [0.0, 0.0, -1.0],
        );
        GiScene::build(
            &[mesh_of(pos, idx, [1.0, 1.0, 1.0])],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        )
    }

    /// 墙角场景:灰地板 + 红墙(x=−8)+ 蓝墙(z=−8),斜向方向光 + 淡蓝天光。
    fn corner_scene() -> GiScene {
        let mut floor_p = Vec::new();
        let mut floor_i = Vec::new();
        push_quad(
            &mut floor_p,
            &mut floor_i,
            [
                [-8.0, 0.0, -8.0],
                [8.0, 0.0, -8.0],
                [8.0, 0.0, 8.0],
                [-8.0, 0.0, 8.0],
            ],
            [0.0, 1.0, 0.0],
        );
        let mut red_p = Vec::new();
        let mut red_i = Vec::new();
        push_quad(
            &mut red_p,
            &mut red_i,
            [
                [-8.0, 0.0, -8.0],
                [-8.0, 0.0, 8.0],
                [-8.0, 6.0, 8.0],
                [-8.0, 6.0, -8.0],
            ],
            [1.0, 0.0, 0.0],
        );
        let mut blue_p = Vec::new();
        let mut blue_i = Vec::new();
        push_quad(
            &mut blue_p,
            &mut blue_i,
            [
                [-8.0, 0.0, -8.0],
                [8.0, 0.0, -8.0],
                [8.0, 6.0, -8.0],
                [-8.0, 6.0, -8.0],
            ],
            [0.0, 0.0, 1.0],
        );
        GiScene::build(
            &[
                mesh_of(floor_p, floor_i, [0.7, 0.7, 0.7]),
                mesh_of(red_p, red_i, [0.75, 0.35, 0.3]),
                mesh_of(blue_p, blue_i, [0.3, 0.45, 0.8]),
            ],
            [0.4, 1.0, 0.35],
            [2.5, 2.5, 2.5],
            [0.25, 0.3, 0.4],
        )
    }

    /// 俯视相机(白炉用):eye (0,3,0) 看向原点。
    fn furnace_camera() -> GiCamera {
        let proj = perspective_rh_zo(0.9, 1.0, 0.1, 100.0);
        let view = look_at_rh([0.0, 3.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, -1.0]);
        GiCamera::new(proj.mul(&view))
    }

    /// 盒内相机(封闭白盒用):原点看向 −z。
    fn box_camera() -> GiCamera {
        let proj = perspective_rh_zo(1.0, 1.0, 0.1, 100.0);
        let view = look_at_rh([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]);
        GiCamera::new(proj.mul(&view))
    }

    /// 墙角相机:`dx` 为纯 +x 平移量(disocclusion 测试用)。
    fn corner_camera(dx: f32) -> GiCamera {
        let proj = perspective_rh_zo(1.0, 1.0, 0.1, 100.0);
        let view = look_at_rh(
            [4.0 + dx, 4.5, 4.0],
            [-2.0 + dx, 0.0, -3.0],
            [0.0, 1.0, 0.0],
        );
        GiCamera::new(proj.mul(&view))
    }

    /// SH 系数组 MSE(f64 累加;收敛计量)。
    fn sh_mse(a: &[ShL1Rgb], b: &[ShL1Rgb]) -> f64 {
        assert_eq!(a.len(), b.len());
        let mut acc = 0.0f64;
        let mut cnt = 0usize;
        for (sa, sb) in a.iter().zip(b.iter()) {
            for k in 0..4 {
                for ch in 0..3 {
                    let d = f64::from(sa.c[k][ch]) - f64::from(sb.c[k][ch]);
                    acc += d * d;
                    cnt += 1;
                }
            }
        }
        acc / cnt.max(1) as f64
    }

    #[test]
    fn gbuffer_pinhole_depth_normal_anchored() {
        // z=−2 单位四边形([−1,1]²,法线 +z):中心像素深度 = ndc(−2) 手算锚定,
        // 角像素未命中 ⇒ 深度 1.0(无效)。
        let mut pos = Vec::new();
        let mut idx = Vec::new();
        push_quad(
            &mut pos,
            &mut idx,
            [
                [-1.0, -1.0, -2.0],
                [1.0, -1.0, -2.0],
                [1.0, 1.0, -2.0],
                [-1.0, 1.0, -2.0],
            ],
            [0.0, 0.0, 1.0],
        );
        let scene = GiScene::build(
            &[mesh_of(pos, idx, [1.0; 3])],
            [0.0, 0.0, 1.0],
            [1.0; 3],
            [0.0; 3],
        );
        let cam = box_camera();
        let (depth, normals) = render_gbuffer_pinhole(&scene, &cam, 64, 64);
        // 手算:ndc = (m22·z + m23)/(−z),z = −2 ⇒ (2.002…− 0.1001…)/2 ≈ 0.95095。
        let (n_, f_) = (0.1f64, 100.0f64);
        let m22 = f_ / (n_ - f_);
        let m23 = n_ * f_ / (n_ - f_);
        let expect = ((m22 * -2.0 + m23) / 2.0) as f32;
        let got = depth.get(32, 32, 0);
        assert!(
            (got - expect).abs() < 1e-4,
            "中心深度锚定: {got} vs {expect}"
        );
        assert_eq!(normals.pixel3(32, 32), [0.0, 0.0, 1.0], "中心法线 +z");
        assert_eq!(depth.get(0, 0, 0), 1.0, "角像素未命中 ⇒ 深度 1.0");
        assert_eq!(normals.pixel3(0, 0), [0.0, 0.0, 0.0], "角像素零法线");
    }

    #[test]
    fn white_furnace_energy_conservation() {
        // ===== 理论推导(RFC-0016 章 E3 能量守恒检查口径)=====
        // 【开放白炉 · 不丢能】无限大白地板 + 单位天光(L_sky=1),方向光关,
        // 关滤波关累积单反弹:探针法线半球(朝上)内所有光线自地板上方出发即离
        // 面,永不命中 ⇒ L(ω) ≡ 1。SH(L1)+ 余弦卷积对常量半球场在任意方向
        // **精确**重建(推导:半球常量场 c₀ = √π·L ⇒ 直流贡献 (√π/2)·√π·L =
        // πL/2;c₁(法向)= √(3/4π)·πL ⇒ 线性贡献 (2π/3)·(3/4π)·πL = πL/2;
        // 合计 πL——见 gi::sh 模块文档)。∴ 理论值 E = π·L_sky = π/通道。
        // MC 无偏估计 + pdf 护栏(≤5% 欠估计偏差,gi::sh COS_PDF_MIN)+ 16 样本
        // /探针噪声 ⇒ 验收:全屏均值 ∈ π·[0.9, 1.1](容差 ≤10%)。
        let scene = open_furnace_scene();
        let cam = furnace_camera();
        let (depth, normals) = render_gbuffer_pinhole(&scene, &cam, 64, 64);
        let tracer = RayTracedRadiance::new(scene);
        let params = GiParams {
            filter: false,
            temporal: false,
            seed: 0xF00D,
            ..GiParams::default()
        };
        let out = render_gi(&depth, &normals, &cam, &tracer, None, None, &params);
        // 确定性复核:同参数再跑一遍,逐位一致。
        let out2 = render_gi(&depth, &normals, &cam, &tracer, None, None, &params);
        assert_eq!(
            out.irradiance.data, out2.irradiance.data,
            "同种子同输入应位级一致"
        );
        let mut mean = [0.0f64; 3];
        let n_px = (64 * 64) as f64;
        for v in out.irradiance.data.chunks_exact(3) {
            for ch in 0..3 {
                mean[ch] += f64::from(v[ch]) / n_px;
            }
        }
        let pi = core::f64::consts::PI;
        eprintln!(
            "[white_furnace] 开放白炉均值 = {mean:.6?}(理论 π = {pi:.6},相对偏差 {:.2?}%)",
            (mean[0] / pi - 1.0) * 100.0
        );
        for (ch, &m) in mean.iter().enumerate() {
            assert!(
                (m - pi).abs() <= 0.1 * pi,
                "开放白炉 ch{ch}: 均值 {m} 应 ∈ π±10%(不丢能)"
            );
        }
        // ===== 【封闭白盒 · 不凭空造能】=====
        // 六面内壁(albedo 1)+ 方向光开启但内壁全部阴影遮蔽 + 天光几何不可达
        // ⇒ 任意命中辐射度 ≡ 0 ⇒ E ≡ 0;任何非零均值即凭空造能。
        let scene = closed_box_scene();
        let cam = box_camera();
        let (depth, normals) = render_gbuffer_pinhole(&scene, &cam, 64, 64);
        let tracer = RayTracedRadiance::new(scene);
        let out = render_gi(&depth, &normals, &cam, &tracer, None, None, &params);
        let total: f64 = out.irradiance.data.iter().map(|&v| f64::from(v)).sum();
        assert_eq!(total, 0.0, "封闭白盒: irradiance 应恒为 0(不凭空造能)");
    }

    #[test]
    fn direction_consistency_vs_bruteforce() {
        // G-G5-6 对拍口径 host 侧:同一 RadianceTracer,探针管线(16 光线/探针
        // + SH + 平面插值,关滤波关累积)vs 独立逐像素蛮力参考(64 spp = 4×
        // 采样,不同种子)——逐像素余弦相似度均值 > 0.9(固定种子)。
        let scene = corner_scene();
        let cam = corner_camera(0.0);
        let (depth, normals) = render_gbuffer_pinhole(&scene, &cam, 64, 64);
        let tracer = RayTracedRadiance::new(scene);
        let params = GiParams {
            filter: false,
            temporal: false,
            seed: 0xC0DE,
            ..GiParams::default()
        };
        let gi = render_gi(&depth, &normals, &cam, &tracer, None, None, &params);
        let reference =
            irradiance_bruteforce_reference(&depth, &normals, &cam, &tracer, 64, 0xBEEF);
        // 参考亮度均值(过滤近黑像素:余弦相似度在近零矢量上无意义)。
        let mut lum_sum = 0.0f64;
        let mut lum_cnt = 0usize;
        for v in reference.data.chunks_exact(3) {
            let l = f64::from(v[0] + v[1] + v[2]) / 3.0;
            if l > 0.0 {
                lum_sum += l;
                lum_cnt += 1;
            }
        }
        let lum_mean = lum_sum / lum_cnt.max(1) as f64;
        let mut cos_sum = 0.0f64;
        let mut cos_cnt = 0usize;
        for y in 0..64u32 {
            for x in 0..64u32 {
                let r = reference.pixel3(x, y);
                let lum = (r[0] + r[1] + r[2]) / 3.0;
                if f64::from(lum) < 0.05 * lum_mean {
                    continue;
                }
                let g = gi.irradiance.pixel3(x, y);
                let dot = f64::from(r[0] * g[0] + r[1] * g[1] + r[2] * g[2]);
                let lr = f64::from(r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
                let lg = f64::from(g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
                if lr < 1e-9 || lg < 1e-9 {
                    continue;
                }
                cos_sum += dot / (lr * lg);
                cos_cnt += 1;
            }
        }
        assert!(cos_cnt >= 2000, "有效对拍像素应过半数(64×64): {cos_cnt}");
        let cos_mean = cos_sum / cos_cnt as f64;
        eprintln!(
            "[direction_consistency] 余弦相似度均值 = {cos_mean:.6}(有效像素 {cos_cnt}/4096)"
        );
        assert!(
            cos_mean > 0.9,
            "方向一致性:余弦相似度均值 {cos_mean} 应 > 0.9(有效像素 {cos_cnt})"
        );
    }

    #[test]
    fn temporal_accumulate_static_converges() {
        // 语义:静态场景(几何/相机不动 ⇒ 当前帧逐帧位级一致)+ 陈旧历史(光照
        // 突变,模拟开关门瞬间的过期历史——深度/法线不变,公共底座验证通过,
        // 正如真实光照突变)。指数混合 ⇒ 帧间差几何衰减:out_1 − cur = −(1−α)·
        // (cur − 陈旧) 起步,Δ_k = α·(1−α)^{k−2}·Δ_1(k ≥ 2,Δ_1 = out_1 − cur)。
        // α = 0.4 ⇒ MSE 比 Δ_8/Δ_1 = α²(1−α)¹² ≈ 0.035% ≪ 5%(收敛)。
        let scene = corner_scene();
        let cam = corner_camera(0.0);
        let (depth, normals) = render_gbuffer_pinhole(&scene, &cam, 32, 32);
        let tracer = RayTracedRadiance::new(scene);
        let mut params = GiParams {
            seed: 0xCAFE,
            ..GiParams::default()
        };
        params.temporal_params.alpha = 0.4;
        let mv = compute_camera_mv(&depth, &cam.view_proj, &cam.view_proj);
        let frame0 = render_gi(&depth, &normals, &cam, &tracer, None, None, &params);
        // 注入陈旧历史:亮度减半(深度/法线不动)。
        let mut history = frame0.history.clone();
        for sh in history.probe_sh.iter_mut() {
            *sh = sh.scale(0.5);
        }
        for v in history.irradiance.data.iter_mut() {
            *v *= 0.5;
        }
        let mut prev_irr = frame0.irradiance.clone();
        let mut prev_sh = frame0.probe_sh.clone();
        let mut diffs = Vec::new();
        let mut diffs_sh = Vec::new();
        for _ in 0..8 {
            let out = render_gi(
                &depth,
                &normals,
                &cam,
                &tracer,
                Some(&history),
                Some(&mv),
                &params,
            );
            diffs.push(ImageF32::mse(&out.irradiance, &prev_irr));
            diffs_sh.push(sh_mse(&out.probe_sh, &prev_sh));
            prev_irr = out.irradiance.clone();
            prev_sh = out.probe_sh.clone();
            history = out.history;
        }
        assert!(diffs[0] > 0.0, "首帧差应非零(陈旧历史注入)");
        eprintln!(
            "[temporal_converge] 帧间差比 Δ_8/Δ_1 = {:.6}(irradiance), {:.6}(探针 SH);理论 α²(1−α)¹² = {:.6}",
            diffs[7] / diffs[0],
            diffs_sh[7] / diffs_sh[0],
            (0.4f64 * 0.4) * 0.6f64.powi(12)
        );
        assert!(
            diffs[7] < 0.05 * diffs[0],
            "8 帧后帧间差应 < 首帧差 5%(收敛): {} vs {}",
            diffs[7],
            diffs[0]
        );
        assert!(
            diffs_sh[7] < 0.05 * diffs_sh[0],
            "探针 SH 同步收敛: {} vs {}",
            diffs_sh[7],
            diffs_sh[0]
        );
    }

    #[test]
    fn temporal_disocclusion_uses_current_frame() {
        // 相机 +x 纯平移 ⇒ 右缘 su>1 出屏 + 视差深度失配 ⇒ 公共底座
        // `validate_history_with_mv` mask = 0;mask = 0 处输出必须逐位等于纯当前
        // 帧(拒绝历史 ⇒ 无鬼影),mask = 1 处为指数混合(有限非负)。
        let scene = corner_scene();
        let cam_a = corner_camera(0.0);
        let cam_b = corner_camera(0.5);
        let (depth_a, normals_a) = render_gbuffer_pinhole(&scene, &cam_a, 64, 64);
        let (depth_b, normals_b) = render_gbuffer_pinhole(&scene, &cam_b, 64, 64);
        let tracer = RayTracedRadiance::new(scene);
        let params = GiParams {
            seed: 0xD15C,
            ..GiParams::default()
        };
        let frame_a = render_gi(&depth_a, &normals_a, &cam_a, &tracer, None, None, &params);
        let mv = compute_camera_mv(&depth_b, &cam_b.view_proj, &cam_a.view_proj);
        let blended = render_gi(
            &depth_b,
            &normals_b,
            &cam_b,
            &tracer,
            Some(&frame_a.history),
            Some(&mv),
            &params,
        );
        let current = render_gi(&depth_b, &normals_b, &cam_b, &tracer, None, None, &params);
        let tp = &params.temporal_params;
        let mask = validate_history_with_mv(
            &depth_b,
            &frame_a.history.depth,
            &normals_b,
            &frame_a.history.normal,
            &mv,
            tp.depth_rel_tol,
            tp.normal_dot_min,
        );
        let (mut zeros, mut ones) = (0usize, 0usize);
        for y in 0..64u32 {
            for x in 0..64u32 {
                let m = mask.get(x, y, 0);
                let b = blended.irradiance.pixel3(x, y);
                let c = current.irradiance.pixel3(x, y);
                if m < 0.5 {
                    zeros += 1;
                    assert_eq!(b, c, "({x},{y}) disocclusion 处必须逐位等于当前帧(无鬼影)");
                } else {
                    ones += 1;
                    for &bv in &b {
                        assert!(bv.is_finite() && bv >= 0.0, "({x},{y}) 混合输出有限非负");
                    }
                }
            }
        }
        eprintln!(
            "[temporal_disocclusion] mask=0(disocclusion)= {zeros} 像素, mask=1 = {ones} 像素"
        );
        assert!(
            zeros >= 32,
            "相机平移应产生 disocclusion(mask=0 ≥ 32 像素): {zeros}"
        );
        assert!(ones >= 2000, "内部区域应大量保持有效: {ones}");
    }
}
