//! G12.3 降噪波 host 面（spec/global_illumination.md RXS-0402；RFC-0029 §4.5；
//! 门 `g12.p0.m162.denoise_pipeline_tsr`）。
//!
//! 本模块 = 降噪管线 host 数据面/host oracle，与 device 降噪 kernel
//! `kernels/g12_pt_denoise.rx` 公式面逐字同源（RXS-0357 host oracle 纪律继承：
//! 仅 host 输出不能充绿，门绿由 device 腿承载）。消费面纪律：
//! - **G12.2 生产化模块输出面**（[`super::prod`]：ProdImage/ProdScene/相机面）
//!   只消费不回写；M96 参照器冻结面 0-byte；
//! - **temporal 底座 0-byte 不接线**（RXS-0402 L4）：时域累积 host 腿直接
//!   调用 [`crate::temporal::common`] 的既有历史接口面（[`compute_camera_mv`]
//!   / [`reproject_sample`] / [`validate_history`] / YCoCg 邻域裁剪族）与
//!   历史验证参数面（TSR `depth_rel_tol=0.1` / TAA `blend_alpha=0.1` /
//!   法线判据 0.9）——只读消费，不改 TAA/TSR 任何语义面/代码面；
//! - G-buffer（主光线深度/法线）与 MV 为 host 预备输入面（G7.4「输入是
//!   输入」先例——与 RNG 流同纪律）；降噪语义本体（时域累积 + A-trous）
//!   在 device 腿承载。
//!
//! 管线形态（RXS-0402 L1）：PT 原生帧（raw）→ 时域累积（历史重投影 +
//! 三判据历史验证 + YCoCg 邻域裁剪 + α 混合）→ 空域 A-trous 类滤波（逐级
//! 步长 2^ℓ × 3 级，边缘停止函数消费亮度/深度/法线）→ 降噪帧
//! （denoised）。帧型标签闭集 `{raw, denoised}`。

use super::PtCamera;
use super::prod::{ProdScene, prod_scene_from_m96};
use crate::rt::bvh::{Ray, TriBvh, Vec3};
use crate::temporal::common::{self, Mat4, neighborhood_aabb, perspective_rh_zo, rgb_to_ycocg};
use crate::temporal::image::ImageF32;

// ---------------------------------------------------------------------------
// 冻结参数面（实现波定，登记进 evidence；RED 臂注入位单列）
// ---------------------------------------------------------------------------

/// 时域累积当前帧混合权重 α（TAA `blend_alpha=0.1` 参数面消费；有效累积窗
/// 约 10 帧）。
pub const G12_DENOISE_ALPHA: f32 = 0.1;
/// 历史验证深度相对容差（TSR `depth_rel_tol=0.1` 参数面消费）。
pub const G12_DENOISE_DEPTH_REL_TOL: f32 = 0.1;
/// 历史验证法线点积下界（TAA 历史验证法线判据消费面：0.9）。
pub const G12_DENOISE_NORMAL_DOT_MIN: f32 = 0.9;
/// A-trous 级数（小波域多尺度 ℓ=0/1/2，步长 1/2/4）。
pub const G12_DENOISE_ATROUS_LEVELS: u32 = 3;
/// 边缘停止亮度尺度 σ_l（相对亮度差，以 max(Y_p, Y_q, 0.05) 对称相对化）。
pub const G12_DENOISE_SIGMA_L: f32 = 0.2;
/// 边缘停止深度尺度 σ_z（相对深度差）。
pub const G12_DENOISE_SIGMA_Z: f32 = 0.1;
/// firefly 预钳位强度 γ（YCoCg 3x3 邻域 μ±γσ 方差裁剪——消费底座
/// `neighborhood_variance_bounds`/`clamp_to_bounds` 面；低 spp 亮噪尖峰
/// （firefly）预抑制，消除边缘停止滤波对尖峰的不对称保留所引入的系统性
/// 变暗偏置）。
pub const G12_DENOISE_FIREFLY_GAMMA: f32 = 2.0;
/// 帧间相机微移（沿相机 right 向；fixture 常量——m96 场景单位尺度（盒
/// [0,1]、相机距离 ~0.9、fov 50°），0.02 ≈ 1.5px 量级 MV，历史验证在帧
/// 边缘/遮挡揭开处真实触发；实测 MV 统计进 evidence）。
pub const G12_DENOISE_CAM_SHIFT: f32 = 0.02;
/// 帧 2 固定 seed 派生异或常量（固定 seed 确定性协议：帧 1 seed =
/// G12_PROD_SEED，帧 2 seed = G12_PROD_SEED ^ 本常量——两帧采样流独立
/// 且确定）。
pub const G12_DENOISE_FRAME2_SEED_XOR: u64 = 0x3C6E_0B8E_9A5A_5A5A;
/// 降噪 raw 帧 spp（低 spp 噪声面）。
pub const G12_DENOISE_RAW_SPP: u32 = 4;
/// 参照帧 spp（全 spp 参照 = 收敛上界面）。
pub const G12_DENOISE_REF_SPP: u32 = 64;
/// MV/深度派生的投影深度区间（ZO [0,1]；场景尺度容纳面）。
pub const G12_DENOISE_Z_NEAR: f32 = 0.05;
/// 投影远平面。
pub const G12_DENOISE_Z_FAR: f32 = 10000.0;

/// 帧型标签闭集（RXS-0402 L1：{raw, denoised}，混标即 RED）。
pub const G12_DENOISE_FRAME_LABELS: [&str; 2] = ["raw", "denoised"];

/// 帧型标签闭集校验。
pub fn frame_label_valid(label: &str) -> bool {
    G12_DENOISE_FRAME_LABELS.contains(&label)
}

// ---------------------------------------------------------------------------
// 参数面（fail-closed 校验；RED 注入位正常态恒 0/false）
// ---------------------------------------------------------------------------

/// 降噪管线参数（device params 缓冲与 host oracle 共用同一值面）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DenoiseParams {
    /// 当前帧混合权重 α。
    pub alpha: f32,
    /// 历史验证深度相对容差。
    pub depth_rel_tol: f32,
    /// 历史验证法线点积下界。
    pub normal_dot_min: f32,
    /// A-trous 级数。
    pub atrous_levels: u32,
    /// 边缘停止亮度尺度。
    pub sigma_l: f32,
    /// 边缘停止深度尺度。
    pub sigma_z: f32,
    /// 能量偏置注入（≠0 = RED 臂；正常 = 0）。
    pub energy_bias: f32,
    /// 历史验证关闭注入（true = RED 臂；正常 = false）。
    pub validation_off: bool,
    /// 旁路旁通注入（true = 噪声底未降冒充降噪 RED 臂；正常 = false）。
    pub denoise_off: bool,
}

impl DenoiseParams {
    /// 生产化基线（参数面 = 冻结常量集）。
    pub fn production() -> DenoiseParams {
        DenoiseParams {
            alpha: G12_DENOISE_ALPHA,
            depth_rel_tol: G12_DENOISE_DEPTH_REL_TOL,
            normal_dot_min: G12_DENOISE_NORMAL_DOT_MIN,
            atrous_levels: G12_DENOISE_ATROUS_LEVELS,
            sigma_l: G12_DENOISE_SIGMA_L,
            sigma_z: G12_DENOISE_SIGMA_Z,
            energy_bias: 0.0,
            validation_off: false,
            denoise_off: false,
        }
    }

    /// fail-closed 校验（门侧消费前必经）。
    pub fn validate(&self) -> Result<(), String> {
        if !(self.alpha > 0.0 && self.alpha <= 1.0) {
            return Err(format!("alpha 越域 (0,1]: {}", self.alpha));
        }
        if !(self.depth_rel_tol > 0.0 && self.depth_rel_tol < 1.0) {
            return Err(format!("depth_rel_tol 越域 (0,1): {}", self.depth_rel_tol));
        }
        if !(self.normal_dot_min > 0.0 && self.normal_dot_min <= 1.0) {
            return Err(format!(
                "normal_dot_min 越域 (0,1]: {}",
                self.normal_dot_min
            ));
        }
        if self.atrous_levels == 0 || self.atrous_levels > 5 {
            return Err(format!("atrous_levels 越域 [1,5]: {}", self.atrous_levels));
        }
        if !(self.sigma_l > 0.0 && self.sigma_l.is_finite()) {
            return Err(format!("sigma_l 非正/非有限: {}", self.sigma_l));
        }
        if !(self.sigma_z > 0.0 && self.sigma_z.is_finite()) {
            return Err(format!("sigma_z 非正/非有限: {}", self.sigma_z));
        }
        if !(self.energy_bias.is_finite() && self.energy_bias.abs() < 1.0) {
            return Err(format!("energy_bias 越域 (−1,1): {}", self.energy_bias));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// G-buffer 派生（主光线深度/法线；输入面纪律承 G7.4「输入是输入」先例）
// ---------------------------------------------------------------------------

/// G-buffer（逐像素 NDC 深度 [0,1] + 世界几何法线；miss = 深度 1 + 零法线，
/// 历史验证对零法线恒拒）。
#[derive(Debug, Clone)]
pub struct GBuffer {
    /// NDC 深度（1 通道，ZO [0,1]）。
    pub depth: ImageF32,
    /// 世界空间几何法线（3 通道，绕向原样；miss = 零向量）。
    pub normal: ImageF32,
}

/// PT 相机 → view_proj（`temporal::common::perspective_rh_zo` × 逐 PT 相机
/// 基组装的 view——行 = [right; up; −forward]，与 PT 相机基逐字一致；
/// 方形画幅断言 fail-closed〔PT 相机水平 = 垂直半角〕）。
pub fn pt_camera_view_proj(cam: &PtCamera) -> Mat4 {
    assert_eq!(
        cam.width, cam.height,
        "PT 相机非方形画幅（水平=垂直半角约定）"
    );
    let s = Vec3::from_array(cam.right);
    let u = Vec3::from_array(cam.up);
    let f = Vec3::from_array(cam.forward);
    let e = Vec3::from_array(cam.origin);
    let view = Mat4 {
        m: [
            [s.x, s.y, s.z, -s.dot(e)],
            [u.x, u.y, u.z, -u.dot(e)],
            [-f.x, -f.y, -f.z, f.dot(e)],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };
    let fov_y = 2.0 * cam.tan_half_fov.atan();
    let proj = perspective_rh_zo(fov_y, 1.0, G12_DENOISE_Z_NEAR, G12_DENOISE_Z_FAR);
    proj.mul(&view)
}

/// 主光线 G-buffer 派生：逐像素中心光线（无 jitter——G-buffer 为像素中心
/// 主可见性输入面）经 host BVH 求交，深度转 NDC（ZO），法线取绕向几何
/// 法线（朝向一致性由历史验证零法线拒判兜住 miss）。
pub fn gbuffer_host(scene: &ProdScene) -> GBuffer {
    let bvh = TriBvh::build(&scene.positions, &scene.indices);
    let cam = &scene.camera;
    let (w, h) = (cam.width, cam.height);
    let co = Vec3::from_array(cam.origin);
    let cf = Vec3::from_array(cam.forward);
    let cr = Vec3::from_array(cam.right);
    let cu = Vec3::from_array(cam.up);
    let tan = cam.tan_half_fov;
    let mut depth = ImageF32::new(w, h, 1);
    let mut normal = ImageF32::new(w, h, 3);
    let zf = G12_DENOISE_Z_FAR;
    let zn = G12_DENOISE_Z_NEAR;
    for y in 0..h {
        for x in 0..w {
            let u = (x as f32 + 0.5) / w as f32;
            let v = (y as f32 + 0.5) / h as f32;
            let sx = (2.0 * u - 1.0) * tan;
            let sy = (1.0 - 2.0 * v) * tan;
            let d0 = cf + cr * sx + cu * sy;
            let len = d0.length();
            let dir = d0 * (1.0 / len);
            let ray = Ray { origin: co, dir };
            if let Some(hit) = bvh.intersect(&ray) {
                // 前向距离 d = t·(dir·forward) = t/|d0|（dir·cf = 1/|d0|）。
                let d_fwd = hit.t / len;
                let ndc = zf * (d_fwd - zn) / ((zf - zn) * d_fwd);
                depth.set(x, y, 0, ndc.clamp(0.0, 1.0));
                normal.set_pixel3(x, y, hit.normal);
            } else {
                depth.set(x, y, 0, 1.0);
                normal.set_pixel3(x, y, [0.0, 0.0, 0.0]);
            }
        }
    }
    GBuffer { depth, normal }
}

/// 相机 MV 派生（`temporal::common::compute_camera_mv` 0-byte 只读消费；
/// 输入 = 当前帧 NDC 深度 + 当前/上一帧 view_proj）。
pub fn camera_mv_host(gbuf_cur: &GBuffer, cam_cur: &PtCamera, cam_prev: &PtCamera) -> ImageF32 {
    common::compute_camera_mv(
        &gbuf_cur.depth,
        &pt_camera_view_proj(cam_cur),
        &pt_camera_view_proj(cam_prev),
    )
}

// ---------------------------------------------------------------------------
// 时域累积 host oracle（与 device kernel 时域模式公式面逐字同源；host 腿直接
// 消费 temporal::common 历史接口面——0-byte 只读）
// ---------------------------------------------------------------------------

/// 时域累积：当前帧 + 历史帧（经 MV 重投影 + 三判据验证 + YCoCg 邻域裁剪）
/// α 混合。返回 (累积帧, 有效历史 mask)。history = None（首帧）→ 输出 =
/// 当前帧（α=1 兜底），mask 全 0。
///
/// 公式面（RXS-0402 L4；与 `temporal::common::validate_history` /
/// `reproject_sample` / `neighborhood_aabb` / `clamp_to_bounds` 逐字同源）：
/// valid = 屏内（重投影 uv ∈ [0,1]）∧ |d_cur − d_prev| ≤ tol·max(d_cur,
/// d_prev, 1e-6) ∧ dot(n_cur, n_prev) ≥ normal_dot_min；out = valid ?
/// α·cur + (1−α)·clamp(hist) : cur。validation_off（RED 臂）= 跳过深度/
/// 法线双判据（仅屏内判）；denoise_off（RED 臂）= 输出恒 = 当前帧。
pub fn temporal_accumulate_host(
    cur: &ImageF32,
    history: Option<&ImageF32>,
    gbuf_cur: &GBuffer,
    gbuf_prev: Option<&GBuffer>,
    mv: &ImageF32,
    params: &DenoiseParams,
) -> (ImageF32, ImageF32) {
    assert!(cur.c >= 3 && mv.c == 2, "当前帧/MV 通道面不符");
    let (w, h) = (cur.w, cur.h);
    let mut out = ImageF32::new(w, h, 3);
    let mut valid_mask = ImageF32::new(w, h, 1);
    for y in 0..h {
        for x in 0..w {
            out.set_pixel3(x, y, cur.pixel3(x, y));
        }
    }
    let (Some(hist), Some(gp)) = (history, gbuf_prev) else {
        return (out, valid_mask);
    };
    // 历史重投影（消费底座 reproject_sample；出屏 = disocclusion 置 0）。
    let (hist_reproj, inside) = common::reproject_sample(hist, mv);
    let (prev_depth_reproj, _) = common::reproject_sample(&gp.depth, mv);
    let (prev_normal_reproj, _) = common::reproject_sample(&gp.normal, mv);
    // 历史验证（消费底座 validate_history；深度相对差 + 法线点积双判据）。
    let mut valid = common::validate_history(
        &gbuf_cur.depth,
        &prev_depth_reproj,
        &gbuf_cur.normal,
        &prev_normal_reproj,
        params.depth_rel_tol,
        params.normal_dot_min,
    );
    if params.validation_off {
        // RED 臂：历史验证关闭（仅屏内判）——深度/法线判据跳过。
        valid = ImageF32::from_fn(w, h, 1, |_, _, _| 1.0);
    }
    // YCoCg 邻域裁剪（消费底座 neighborhood_aabb/clamp_to_bounds）。
    let cur_ycc = crate::temporal::common::rgb_image_to_ycocg(cur);
    let (lo, hi) = neighborhood_aabb(&cur_ycc);
    let hist_ycc = crate::temporal::common::rgb_image_to_ycocg(&hist_reproj);
    let hist_clamped = crate::temporal::common::ycocg_image_to_rgb(
        &crate::temporal::common::clamp_to_bounds(&hist_ycc, &lo, &hi),
    );
    let a = params.alpha;
    for y in 0..h {
        for x in 0..w {
            let v = if inside.get(x, y, 0) > 0.5 && valid.get(x, y, 0) > 0.5 {
                1.0
            } else {
                0.0
            };
            valid_mask.set(x, y, 0, v);
            let cc = cur.pixel3(x, y);
            let hc = hist_clamped.pixel3(x, y);
            let blended = [
                a * cc[0] + (1.0 - a) * hc[0],
                a * cc[1] + (1.0 - a) * hc[1],
                a * cc[2] + (1.0 - a) * hc[2],
            ];
            let o = if v > 0.5 { blended } else { cc };
            out.set_pixel3(x, y, o);
        }
    }
    if params.denoise_off {
        // RED 臂：旁路旁通（输出恒 = 当前帧，冒充已降噪）。
        for y in 0..h {
            for x in 0..w {
                out.set_pixel3(x, y, cur.pixel3(x, y));
            }
        }
    }
    (out, valid_mask)
}

// ---------------------------------------------------------------------------
// firefly 预钳位 host oracle（与 device kernel mode 2 公式面逐字同源；消费
// 底座 neighborhood_variance_bounds/clamp_to_bounds 面——YCoCg 3x3 邻域
// μ±γσ 方差裁剪）
// ---------------------------------------------------------------------------

/// firefly 预钳位：逐像素 YCoCg 钳入 3x3 邻域 μ±γσ（亮噪尖峰预抑制——
/// 低 spp firefly 是边缘停止滤波系统性变暗偏置的主源，先钳位再滤波）。
pub fn firefly_clamp_host(img: &ImageF32, gamma: f32) -> ImageF32 {
    assert!(img.c >= 3, "firefly 钳位输入必须 ≥3 通道");
    let ycc = crate::temporal::common::rgb_image_to_ycocg(img);
    let (lo, hi) = crate::temporal::common::neighborhood_variance_bounds(&ycc, gamma);
    crate::temporal::common::ycocg_image_to_rgb(&crate::temporal::common::clamp_to_bounds(
        &ycc, &lo, &hi,
    ))
}

/// 单帧降噪 host 全管线（标定/单测消费面）：firefly 预钳位 → 时域累积 →
/// A-trous 逐级。返回 (降噪帧, 有效历史 mask)。
pub fn denoise_frame_host(
    cur: &ImageF32,
    history: Option<&ImageF32>,
    gbuf_cur: &GBuffer,
    gbuf_prev: Option<&GBuffer>,
    mv: &ImageF32,
    params: &DenoiseParams,
) -> (ImageF32, ImageF32) {
    let pre = if params.denoise_off {
        cur.clone()
    } else {
        firefly_clamp_host(cur, G12_DENOISE_FIREFLY_GAMMA)
    };
    let (acc, valid) = temporal_accumulate_host(&pre, history, gbuf_cur, gbuf_prev, mv, params);
    (
        atrous_host(&acc, &gbuf_cur.depth, &gbuf_cur.normal, params),
        valid,
    )
}

// ---------------------------------------------------------------------------
// 空域 A-trous 类滤波 host oracle（与 device kernel A-trous 模式公式面逐字
// 同源：逐级步长 2^ℓ × 3x3 [1,2,1]²/16 核 × 边缘停止函数）
// ---------------------------------------------------------------------------

/// A-trous 单级：3x3 核（[1,2,1]²/16）按步长 step=2^ℓ 抽稀，边缘停止
/// w = w_h · exp(−dL²/(2σ_l²)) · exp(−dZ²/(2σ_z²)) · max(dot(n,n′),0)^8
/// （dL = |Y_p − Y_q|/max(Y_p, 0.05) 相对亮度差——中心像素归一化优先保护
/// 暗侧防边缘光晕;亮噪尖峰不对称保留引入的偏置由 firefly 预钳位腿抑制；
/// dZ = |z_p − z_q|/max(z_p, z_q, 1e-6) 相对深度差）。denoise_off（RED
/// 臂）= 恒等输出。
pub fn atrous_level_host(
    img: &ImageF32,
    depth: &ImageF32,
    normal: &ImageF32,
    level: u32,
    params: &DenoiseParams,
) -> ImageF32 {
    let (w, h) = (img.w, img.h);
    let mut out = ImageF32::new(w, h, 3);
    if params.denoise_off {
        for y in 0..h {
            for x in 0..w {
                out.set_pixel3(x, y, img.pixel3(x, y));
            }
        }
        return out;
    }
    let step = 1i32 << level;
    let sig_l2 = 2.0 * params.sigma_l * params.sigma_l;
    let sig_z2 = 2.0 * params.sigma_z * params.sigma_z;
    for y in 0..h {
        for x in 0..w {
            let yc_p = rgb_to_ycocg(img.pixel3(x, y));
            let z_p = depth.get(x, y, 0);
            let n_p = normal.pixel3(x, y);
            let mut acc = [0.0f32; 3];
            let mut wsum = 0.0f32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let xx = (x as i32 + dx * step).clamp(0, w as i32 - 1) as u32;
                    let yy = (y as i32 + dy * step).clamp(0, h as i32 - 1) as u32;
                    let w_h =
                        (1.0 + (1.0 - dx.abs() as f32)) * (1.0 + (1.0 - dy.abs() as f32)) / 16.0;
                    let yc_q = rgb_to_ycocg(img.pixel3(xx, yy));
                    let dl = (yc_p[0] - yc_q[0]).abs() / yc_p[0].max(yc_q[0]).max(0.05);
                    let w_lum = (-dl * dl / sig_l2).exp();
                    let z_q = depth.get(xx, yy, 0);
                    let dz = (z_p - z_q).abs() / z_p.max(z_q).max(1e-6);
                    let w_z = (-dz * dz / sig_z2).exp();
                    let n_q = normal.pixel3(xx, yy);
                    let lp = (n_p[0] * n_p[0] + n_p[1] * n_p[1] + n_p[2] * n_p[2]).sqrt();
                    let lq = (n_q[0] * n_q[0] + n_q[1] * n_q[1] + n_q[2] * n_q[2]).sqrt();
                    let dot = if lp > 1e-12 && lq > 1e-12 {
                        ((n_p[0] * n_q[0] + n_p[1] * n_q[1] + n_p[2] * n_q[2]) / (lp * lq)).max(0.0)
                    } else {
                        0.0
                    };
                    let w_n = dot.powf(8.0);
                    let wtap = w_h * w_lum * w_z * w_n;
                    let q = img.pixel3(xx, yy);
                    acc[0] += wtap * q[0];
                    acc[1] += wtap * q[1];
                    acc[2] += wtap * q[2];
                    wsum += wtap;
                }
            }
            // 中心 tap 恒有 w_h=4/16、w_lum=w_z=w_n=1（q==p）→ wsum ≥ 1/4，
            // 无需零除兜底分支（min/max 算术门纪律）。
            let inv = 1.0 / wsum.max(1e-6);
            out.set_pixel3(x, y, [acc[0] * inv, acc[1] * inv, acc[2] * inv]);
        }
    }
    out
}

/// A-trous 全管线（逐级 ℓ=0..levels−1；逐级消费上级输出）。
pub fn atrous_host(
    img: &ImageF32,
    depth: &ImageF32,
    normal: &ImageF32,
    params: &DenoiseParams,
) -> ImageF32 {
    let mut cur = img.clone();
    for level in 0..params.atrous_levels {
        cur = atrous_level_host(&cur, depth, normal, level, params);
    }
    cur
}

// ---------------------------------------------------------------------------
// 噪声谱 / 能量守恒测量面（门消费 = device 输出帧的 host 确定函数聚合，
// G12.2 既有门同律；测量公式确定函数，双跑位级一致由 digest 面承载）
// ---------------------------------------------------------------------------

/// 帧亮度面（YCoCg Y 通道：0.25r + 0.5g + 0.25b——与邻域裁剪同一亮度空间）。
pub fn frame_luminance(rgb: &[f32]) -> Vec<f32> {
    rgb.chunks_exact(3)
        .map(|p| rgb_to_ycocg([p[0], p[1], p[2]])[0])
        .collect()
}

/// 3x3 高斯模糊（[1,2,1]²/16，边界复制；纯线性——噪声谱高通基准面）。
fn blur3x3(lum: &[f32], w: u32, h: u32) -> Vec<f32> {
    let mut out = vec![0.0f32; lum.len()];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0f32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let xx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                    let yy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                    let wgt =
                        (1.0 + (1.0 - dx.abs() as f32)) * (1.0 + (1.0 - dy.abs() as f32)) / 16.0;
                    acc += wgt * lum[(yy * w + xx) as usize];
                }
            }
            out[(y * w + x) as usize] = acc;
        }
    }
    out
}

/// 噪声谱高频能量（RXS-0402 L2 口径：帧误差〔对同场景同 spp 上界参照帧〕
/// 亮度面的高通段能量 = mean((err − blur3x3(err))²)，**在低梯度半幅掩码
/// 上取均值**——掩码 = 参照帧亮度 3x3 极差 ≤ 全幅中位数的像素。边缘位移/
/// 光晕偏置（高梯度区）与噪声底（平滑区）分离：降噪有效性在噪声所在的
/// 平滑区 measured；口径进 evidence）。
pub fn high_freq_error_energy(frame: &[f32], reference: &[f32], w: u32, h: u32) -> f64 {
    assert_eq!(frame.len(), reference.len(), "误差面形状不符");
    let lf = frame_luminance(frame);
    let lr = frame_luminance(reference);
    let err: Vec<f32> = lf.iter().zip(lr.iter()).map(|(a, b)| a - b).collect();
    let blur = blur3x3(&err, w, h);
    // 参照帧 3x3 极差梯度 + 中位数掩码。
    let mut grad = vec![0.0f32; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let mut mn = f32::INFINITY;
            let mut mx = f32::NEG_INFINITY;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let xx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                    let yy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                    let v = lr[(yy * w + xx) as usize];
                    mn = mn.min(v);
                    mx = mx.max(v);
                }
            }
            grad[(y * w + x) as usize] = mx - mn;
        }
    }
    let mut sorted = grad.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = sorted[sorted.len() / 2];
    let mut acc = 0.0f64;
    let mut n = 0u64;
    for (i, g) in grad.iter().enumerate() {
        if *g <= median {
            let hp = f64::from(err[i] - blur[i]);
            acc += hp * hp;
            n += 1;
        }
    }
    acc / (n as f64).max(1.0)
}

/// 噪声谱高频能量下降比 = 1 − hf(denoised)/hf(raw)（≥ 标定阈判据由门侧
/// 消费 g12_budget 标定条目）。
pub fn hf_noise_drop(raw: &[f32], denoised: &[f32], reference: &[f32], w: u32, h: u32) -> f64 {
    let e_raw = high_freq_error_energy(raw, reference, w, h);
    let e_den = high_freq_error_energy(denoised, reference, w, h);
    1.0 - e_den / e_raw.max(1e-30)
}

/// 帧均值能量（全通道算术均值，f64 聚合）。
pub fn frame_mean_energy(rgb: &[f32]) -> f64 {
    let mut acc = 0.0f64;
    for &v in rgb {
        acc += f64::from(v);
    }
    acc / (rgb.len() as f64).max(1.0)
}

/// 帧均值能量相对差（RXS-0402 L3：|mean(den) − mean(raw)| /
/// max(mean(raw), tiny)——不引入系统性变暗/变亮偏置）。
pub fn frame_mean_rel_diff(a: &[f32], b: &[f32]) -> f64 {
    let ma = frame_mean_energy(a);
    let mb = frame_mean_energy(b);
    (ma - mb).abs() / mb.abs().max(1e-12)
}

/// 区域均值能量差分布 p90（RXS-0402 L3：8×8 分块逐块 |mean_den −
/// mean_raw|/max(mean_raw, 1e-12)，nearest-rank p90 进 evidence）。
pub fn region_energy_diff_p90(denoised: &[f32], raw: &[f32], w: u32, h: u32) -> f64 {
    let bw = 8u32;
    let bh = 8u32;
    let mut diffs: Vec<f64> = Vec::new();
    let mut by = 0;
    while by < h {
        let mut bx = 0;
        while bx < w {
            let xe = (bx + bw).min(w);
            let ye = (by + bh).min(h);
            let mut sa = 0.0f64;
            let mut sb = 0.0f64;
            let mut n = 0u64;
            for y in by..ye {
                for x in bx..xe {
                    let i = (y * w + x) as usize * 3;
                    sa += f64::from(denoised[i])
                        + f64::from(denoised[i + 1])
                        + f64::from(denoised[i + 2]);
                    sb += f64::from(raw[i]) + f64::from(raw[i + 1]) + f64::from(raw[i + 2]);
                    n += 3;
                }
            }
            let ma = sa / n as f64;
            let mb = sb / n as f64;
            diffs.push((ma - mb).abs() / mb.abs().max(1e-12));
            bx += bw;
        }
        by += bh;
    }
    diffs.sort_by(|a, b| a.partial_cmp(b).expect("NaN 差分"));
    if diffs.is_empty() {
        return 0.0;
    }
    diffs[((diffs.len() as f64 - 1.0) * 0.9).round() as usize]
}

/// 降噪帧 canonical digest（SHA-256(out_rgb ‖ out_valid 字节)——固定 seed
/// 确定性协议面；双跑位级一致判据载体）。
pub fn denoise_frame_digest(rgb: &[f32], valid: &[f32]) -> [u8; 32] {
    let mut pre = Vec::with_capacity(rgb.len() * 4 + valid.len() * 4);
    for v in rgb.iter().chain(valid.iter()) {
        pre.extend_from_slice(&v.to_le_bytes());
    }
    rurix_pkg::sha256::digest(&pre)
}

// ---------------------------------------------------------------------------
// device 输入打包（kernel 头注参数面逐字同源）
// ---------------------------------------------------------------------------

/// 降噪参数打包（16 f32；与 `kernels/g12_pt_denoise.rx` 头注逐字同源）：
/// [0]=pixel_count [1]=width [2]=height [3]=mode(0=时域/1=A-trous/
/// 2=firefly 预钳位) [4]=atrous_step(2^ℓ) [5]=alpha [6]=depth_rel_tol
/// [7]=normal_dot_min [8]=sigma_l [9]=sigma_z [10]=energy_bias(RED 臂,
/// 正常=0) [11]=validation_off(RED 臂,正常=0) [12]=has_history
/// [13]=denoise_off(RED 臂,正常=0) [14]=firefly_gamma [15]=reserved(恒 0)。
pub fn pack_denoise_params(
    width: u32,
    height: u32,
    mode: u32,
    atrous_step: u32,
    has_history: bool,
    params: &DenoiseParams,
) -> Vec<f32> {
    let mut out = vec![0.0f32; 16];
    out[0] = (width * height) as f32;
    out[1] = width as f32;
    out[2] = height as f32;
    out[3] = mode as f32;
    out[4] = atrous_step as f32;
    out[5] = params.alpha;
    out[6] = params.depth_rel_tol;
    out[7] = params.normal_dot_min;
    out[8] = params.sigma_l;
    out[9] = params.sigma_z;
    out[10] = params.energy_bias;
    out[11] = if params.validation_off { 1.0 } else { 0.0 };
    out[12] = if has_history { 1.0 } else { 0.0 };
    out[13] = if params.denoise_off { 1.0 } else { 0.0 };
    out[14] = G12_DENOISE_FIREFLY_GAMMA;
    out
}

/// 帧间相机微移（fixture 常量 G12_DENOISE_CAM_SHIFT 沿 right 向；场景
/// 几何 0-byte——返回克隆场景仅相机面不同）。
pub fn moved_camera_scene(scene: &ProdScene) -> ProdScene {
    let mut s = scene.clone();
    let r = s.camera.right;
    s.camera.origin = [
        s.camera.origin[0] + r[0] * G12_DENOISE_CAM_SHIFT,
        s.camera.origin[1] + r[1] * G12_DENOISE_CAM_SHIFT,
        s.camera.origin[2] + r[2] * G12_DENOISE_CAM_SHIFT,
    ];
    s.name = Box::leak(format!("{}_moved", scene.name).into_boxed_str());
    s
}

/// m96 场景 → 生产化场景（降噪门消费面 = prod 模块同源转换；冻结 fixtures
/// 不回写）。
pub fn denoise_scene_from_m96(s: &super::PtScene) -> ProdScene {
    prod_scene_from_m96(s)
}

// ---------------------------------------------------------------------------
// 单测（host oracle 面；纯 host 确定性，无 device 依赖）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gi::path_trace::m96_cornell_scene;

    /// 合成帧：低频渐变 + 格子边界（边缘保持判据载体）。
    fn synth_frame(w: u32, h: u32, noise_amp: f32, seed: u64) -> Vec<f32> {
        let mut out = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                let base = 0.2 + 0.5 * (x as f32 / w as f32);
                let edge = if x >= w / 2 { 0.3 } else { 0.0 };
                // 确定性伪噪声（splitmix 混合;逐像素确定函数）。
                let mut z = (x as u64) * 0x9E37 + (y as u64) * 0xC2B2 + seed;
                z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
                z ^= z >> 29;
                let n = (z % 1000) as f32 / 1000.0 - 0.5;
                let v = (base + edge + noise_amp * n).max(0.0);
                out.extend_from_slice(&[v, v * 0.8, v * 0.6]);
            }
        }
        out
    }

    fn flat_depth(w: u32, h: u32, d: f32) -> ImageF32 {
        ImageF32::from_fn(w, h, 1, |_, _, _| d)
    }

    fn flat_normal(w: u32, h: u32) -> ImageF32 {
        ImageF32::from_fn(w, h, 3, |_, _, c| if c == 2 { 1.0 } else { 0.0 })
    }

    // 时域累积：静态场景（MV=0、同深度/法线）历史全接受且 α 混合；
    // 双跑确定；无历史兜底 = 当前帧。
    #[test]
    fn temporal_accumulate_static_accepts_history() {
        let (w, h) = (16u32, 16u32);
        let cur_rgb = synth_frame(w, h, 0.0, 1);
        let hist_rgb = synth_frame(w, h, 0.0, 2);
        let cur = ImageF32::from_fn(w, h, 3, |x, y, c| cur_rgb[((y * w + x) * 3 + c) as usize]);
        let hist = ImageF32::from_fn(w, h, 3, |x, y, c| hist_rgb[((y * w + x) * 3 + c) as usize]);
        let gb = GBuffer {
            depth: flat_depth(w, h, 0.5),
            normal: flat_normal(w, h),
        };
        let mv = ImageF32::new(w, h, 2);
        let p = DenoiseParams::production();
        let (out, valid) = temporal_accumulate_host(&cur, Some(&hist), &gb, Some(&gb), &mv, &p);
        // 全接受（静态 + 屏内）。
        assert!(valid.data.iter().all(|&v| v == 1.0), "静态面历史应全接受");
        // α 混合：out = α·cur + (1−α)·hist（同布局零 MV → 裁剪面不改值域
        // 内历史；边界像素邻域含不同亮度格子时裁剪可收紧——断言在平坦
        // 左上块内）。
        let o = out.pixel3(2, 2);
        let cc = cur.pixel3(2, 2);
        let hc = hist.pixel3(2, 2);
        let expect = G12_DENOISE_ALPHA * cc[0] + (1.0 - G12_DENOISE_ALPHA) * hc[0];
        assert!(
            (o[0] - expect).abs() < 1e-4,
            "α 混合不符: {o:?} vs {expect}"
        );
        // 无历史兜底 = 当前帧 + mask 全 0。
        let (out0, valid0) = temporal_accumulate_host(&cur, None, &gb, None, &mv, &p);
        assert!(out0.data == cur.data, "无历史兜底必须 = 当前帧");
        assert!(valid0.data.iter().all(|&v| v == 0.0));
    }

    // 时域累积：深度突变处历史被拒（去鬼影面）；validation_off 注入臂全
    // 接受（RED 检出器语义面）。
    #[test]
    fn temporal_accumulate_rejects_depth_discontinuity() {
        let (w, h) = (16u32, 16u32);
        let cur = ImageF32::from_fn(w, h, 3, |_, _, _| 0.5);
        let hist = ImageF32::from_fn(w, h, 3, |_, _, _| 0.9);
        let gb_cur = GBuffer {
            depth: flat_depth(w, h, 0.5),
            normal: flat_normal(w, h),
        };
        let mut gb_prev = GBuffer {
            depth: flat_depth(w, h, 0.5),
            normal: flat_normal(w, h),
        };
        // 左半深度突变（遮挡揭开面）。
        for y in 0..h {
            for x in 0..(w / 2) {
                gb_prev.depth.set(x, y, 0, 0.1);
            }
        }
        let mv = ImageF32::new(w, h, 2);
        let p = DenoiseParams::production();
        let (_out, valid) =
            temporal_accumulate_host(&cur, Some(&hist), &gb_cur, Some(&gb_prev), &mv, &p);
        assert!(
            (0..h).all(|y| (0..w / 2).all(|x| valid.get(x, y, 0) == 0.0)),
            "深度突变处历史必须被拒"
        );
        assert!(
            (0..h).all(|y| ((w / 2)..w).all(|x| valid.get(x, y, 0) == 1.0)),
            "一致处历史必须接受"
        );
        // RED 臂：validation_off → 全接受（检出器语义面）。
        let mut p_off = p;
        p_off.validation_off = true;
        let (_o2, valid2) =
            temporal_accumulate_host(&cur, Some(&hist), &gb_cur, Some(&gb_prev), &mv, &p_off);
        assert!(
            valid2.data.iter().all(|&v| v == 1.0),
            "validation_off 臂必须全接受（冒充有效历史注入面）"
        );
    }

    // A-trous：噪声平坦块方差下降 + 阶跃边缘保持 + 均值守恒 + denoise_off
    // 恒等（冒充臂检出器语义面）。
    #[test]
    fn atrous_denoises_flat_and_preserves_edge() {
        let (w, h) = (32u32, 32u32);
        let noisy = synth_frame(w, h, 0.15, 7);
        let img = ImageF32::from_fn(w, h, 3, |x, y, c| noisy[((y * w + x) * 3 + c) as usize]);
        let depth = flat_depth(w, h, 0.5);
        let normal = flat_normal(w, h);
        let p = DenoiseParams::production();
        let out = atrous_host(&img, &depth, &normal, &p);
        // 平坦块（右半内部 3..28 行/20..28 列）逐像素标准差下降。
        let sd = |img: &ImageF32| {
            let mut vals = Vec::new();
            for y in 3..28u32 {
                for x in 20..28u32 {
                    vals.push(f64::from(img.get(x, y, 0)));
                }
            }
            let mean = vals.iter().sum::<f64>() / vals.len() as f64;
            (vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / vals.len() as f64).sqrt()
        };
        let sd_before = sd(&img);
        let sd_after = sd(&out);
        assert!(
            sd_after < sd_before * 0.8,
            "平坦块噪声未降: {sd_before} → {sd_after}"
        );
        // 阶跃边缘保持：中线两侧均值差保留 ≥ 80%。
        let col_mean = |img: &ImageF32, x0: u32, x1: u32| {
            let mut acc = 0.0f64;
            for y in 4..28u32 {
                for x in x0..x1 {
                    acc += f64::from(img.get(x, y, 0));
                }
            }
            acc / ((28 - 4) * (x1 - x0)) as f64
        };
        let edge_before = col_mean(&img, 18, 20) - col_mean(&img, 12, 14);
        let edge_after = col_mean(&out, 18, 20) - col_mean(&out, 12, 14);
        assert!(
            edge_after > edge_before * 0.8,
            "阶跃边缘被抹平: {edge_before} → {edge_after}"
        );
        // 帧均值能量守恒（归一化滤波不引入系统性偏置）。
        let ediff = frame_mean_rel_diff(&out.data, &img.data);
        assert!(ediff < 0.02, "均值能量守恒越界: {ediff}");
        // denoise_off = 恒等（冒充降噪臂）。
        let mut p_off = p;
        p_off.denoise_off = true;
        let out_off = atrous_host(&img, &depth, &normal, &p_off);
        assert_eq!(out_off.data, img.data, "denoise_off 必须恒等输出");
    }

    // 噪声谱：降噪帧高频误差能量 < 原生帧；旁路旁通（恒等）下降比 = 0
    // 必被检出（噪声底未降冒充降噪 RED 臂语义面）。
    #[test]
    fn hf_noise_drop_detects_masquerade() {
        let (w, h) = (32u32, 32u32);
        let reference = synth_frame(w, h, 0.0, 3);
        let raw = synth_frame(w, h, 0.4, 3);
        let img = ImageF32::from_fn(w, h, 3, |x, y, c| raw[((y * w + x) * 3 + c) as usize]);
        let depth = flat_depth(w, h, 0.5);
        let normal = flat_normal(w, h);
        let p = DenoiseParams::production();
        let den = atrous_host(&img, &depth, &normal, &p);
        let drop = hf_noise_drop(&raw, &den.data, &reference, w, h);
        assert!(drop > 0.3, "降噪高频能量下降不足: {drop}");
        // 冒充面：恒等输出下降比 ≈ 0。
        let drop_id = hf_noise_drop(&raw, &raw, &reference, w, h);
        assert!(drop_id.abs() < 1e-9, "恒等冒充面下降比非零: {drop_id}");
    }

    // 能量偏置注入臂：+k 亮度注入必使均值能量差越容差（检出器语义面）。
    #[test]
    fn energy_bias_injection_detected() {
        let (w, h) = (16u32, 16u32);
        let raw = synth_frame(w, h, 0.05, 11);
        let biased: Vec<f32> = raw.iter().map(|v| v + 0.05).collect();
        let diff = frame_mean_rel_diff(&biased, &raw);
        assert!(diff > 0.01, "偏置注入未放大均值能量差: {diff}");
        let diff_self = frame_mean_rel_diff(&raw, &raw);
        assert_eq!(diff_self, 0.0, "恒等面均值差非零");
    }

    // G-buffer/MV 派生：cornell 中心像素命中几何（深度 ∈ (0,1)、法线非
    // 零）；微移相机 MV 中位量级非零且有限（历史验证真实触发面）。
    #[test]
    fn gbuffer_and_mv_derivation_sane() {
        let m96 = m96_cornell_scene();
        let scene = prod_scene_from_m96(&m96);
        let gb = gbuffer_host(&scene);
        let (w, h) = (scene.camera.width, scene.camera.height);
        let d = gb.depth.get(w / 2, h / 2, 0);
        assert!(d > 0.0 && d < 1.0, "cornell 中心像素深度越界: {d}");
        let n = gb.normal.pixel3(w / 2, h / 2);
        assert!(
            n[0] * n[0] + n[1] * n[1] + n[2] * n[2] > 0.5,
            "cornell 中心像素法线零向量"
        );
        let moved = moved_camera_scene(&scene);
        let gb2 = gbuffer_host(&moved);
        let mv = camera_mv_host(&gb2, &moved.camera, &scene.camera);
        let mut mags: Vec<f64> = mv
            .data
            .chunks_exact(2)
            .map(|m| f64::from(m[0] * m[0] + m[1] * m[1]).sqrt())
            .collect();
        mags.sort_by(|a, b| a.partial_cmp(b).expect("NaN MV"));
        let med = mags[mags.len() / 2] * w as f64;
        assert!(
            med > 0.05 && med < w as f64 * 0.5,
            "微移相机 MV 中位量级越界: {med}px"
        );
    }

    // 参数 fail-closed + 帧型标签闭集 + digest 确定性。
    #[test]
    fn params_fail_closed_and_label_closed_set() {
        let p = DenoiseParams::production();
        assert!(p.validate().is_ok());
        let mut bad = p;
        bad.alpha = 0.0;
        assert!(bad.validate().is_err(), "alpha=0 未拒");
        let mut bad2 = p;
        bad2.atrous_levels = 0;
        assert!(bad2.validate().is_err(), "levels=0 未拒");
        assert!(frame_label_valid("raw") && frame_label_valid("denoised"));
        assert!(!frame_label_valid("adaptive") && !frame_label_valid("full_reference"));
        let a = vec![0.1f32, 0.2, 0.3, 0.4];
        let v = vec![1.0f32, 0.0];
        assert_eq!(denoise_frame_digest(&a, &v), denoise_frame_digest(&a, &v));
        let mut b = a.clone();
        b[0] += 1e-6;
        assert_ne!(denoise_frame_digest(&a, &v), denoise_frame_digest(&b, &v));
    }
}
