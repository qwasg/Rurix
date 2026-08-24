//! FG/MFG 帧生成独立层 host 参考臂（RFC-0036；G19.2 M-a；G13-N7 兑现）。
//!
//! ## 口径纪律（G13-N7 字面 0-byte）
//!
//! 真实渲染帧率口径不变：FG/MFG 生成帧**禁计入**真实渲染帧率与 upscale
//! ratio。[`FgAccounting`] 以类型面分离两口径——[`FgAccounting::real_render_fps`]
//! 只由真渲帧构成；[`FgAccounting::presented_fps`]（真渲 + 生成）为独立新登记
//! 面，两者并列输出、永不混算。
//!
//! ## 算法（host 纯 f32 确定性；device kernel 车道 = RFC-0036 §1.5 out-of-scope）
//!
//! mv 约定与时域底座一致（[`super::common`]）：对 cur 帧像素 x，
//! `prev_uv = cur_uv − mv(x)`（mv = prev→cur 的 uv 位移场，2 通道）。
//! 生成 t ∈ (0,1) 处中间帧（prev = t0、cur = t1）：
//!
//! 1. **双向 warp**：`a = prev.sample(uv − t·mv)`、`b = cur.sample(uv + (1−t)·mv)`
//!    （mv 场在 uv 处取样，局部平滑运动假设——实践 FG 的标准近似）；
//! 2. **一致性权重**：`w = exp(−‖a−b‖² / σ²)`——正确追踪的点双向样本一致
//!    （w→1），遮挡/揭示区双向样本失配（w→0）；
//! 3. **遮挡感知混合**：一致区间线性混合 `lin = a·(1−t) + b·t`；失配区兜底
//!    时域最近真渲帧样本 `near = (t < 0.5 ? a : b)`；输出
//!    `out = lin·w + near·(1−w)`。

use crate::temporal::image::ImageF32;

/// FG/MFG 参数（默认 = ×2 档单帧插值）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameGenParams {
    /// 每对真渲帧之间生成的中间帧数（MFG 档：1 = ×2，2 = ×3，3 = ×4）。
    pub inserted_per_pair: u32,
    /// 一致性权重带宽 σ（颜色空间 L2 距离；越小遮挡判定越敏感）。
    pub consistency_sigma: f32,
}

impl Default for FrameGenParams {
    fn default() -> Self {
        Self {
            inserted_per_pair: 1,
            consistency_sigma: 0.1,
        }
    }
}

/// MFG 档位 → 每对插入帧数（×2/×3/×4 三档闭集，RFC-0036 §1.1）。
pub fn mfg_inserted_frames(mode_x: u32) -> u32 {
    assert!((2..=4).contains(&mode_x), "MFG 档位闭集 ×2/×3/×4");
    mode_x - 1
}

/// 单帧插值：在 prev（t=0）与 cur（t=1）之间生成 t ∈ (0,1) 处中间帧。
///
/// `mv` 为 2 通道 uv 位移场（prev→cur，按 cur 帧像素栅格）；prev/cur 须为
/// 3 通道同尺寸。纯 f32 确定性（同平台双跑位级一致）。
pub fn interpolate(
    prev: &ImageF32,
    cur: &ImageF32,
    mv: &ImageF32,
    t: f32,
    params: &FrameGenParams,
) -> ImageF32 {
    assert!(prev.c == 3 && prev.same_shape(cur), "prev/cur 须 3 通道同尺寸");
    assert!(mv.c == 2 && mv.w == cur.w && mv.h == cur.h, "mv 须 2 通道同栅格");
    assert!(t > 0.0 && t < 1.0, "t 必须 ∈ (0,1)（端点即真渲帧本身）");
    let (w, h) = (cur.w, cur.h);
    let inv_sigma2 = 1.0 / (params.consistency_sigma * params.consistency_sigma);
    let mut out = ImageF32::new(w, h, 3);
    for y in 0..h {
        for x in 0..w {
            let u = (x as f32 + 0.5) / w as f32;
            let v = (y as f32 + 0.5) / h as f32;
            let mvx = mv.get(x, y, 0);
            let mvy = mv.get(x, y, 1);
            let a = prev.sample_bilinear3(u - t * mvx, v - t * mvy);
            let b = cur.sample_bilinear3(u + (1.0 - t) * mvx, v + (1.0 - t) * mvy);
            let d2 = (a[0] - b[0]) * (a[0] - b[0])
                + (a[1] - b[1]) * (a[1] - b[1])
                + (a[2] - b[2]) * (a[2] - b[2]);
            let w_cons = (-d2 * inv_sigma2).exp();
            let near = if t < 0.5 { a } else { b };
            let mut px = [0.0f32; 3];
            for ch in 0..3 {
                let lin = a[ch] * (1.0 - t) + b[ch] * t;
                px[ch] = lin * w_cons + near[ch] * (1.0 - w_cons);
            }
            out.set_pixel3(x, y, px);
        }
    }
    out
}

/// MFG 多帧生成：在 prev/cur 之间按 `params.inserted_per_pair` 生成
/// t_i = i/(n+1)（i = 1..=n）的中间帧序列（时序递增）。
pub fn mfg_between(
    prev: &ImageF32,
    cur: &ImageF32,
    mv: &ImageF32,
    params: &FrameGenParams,
) -> Vec<ImageF32> {
    let n = params.inserted_per_pair;
    assert!((1..=3).contains(&n), "inserted_per_pair 闭集 1..=3（×2/×3/×4）");
    (1..=n)
        .map(|i| interpolate(prev, cur, mv, i as f32 / (n + 1) as f32, params))
        .collect()
}

/// 帧率账目：真渲与生成两口径类型面分离（禁混算）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FgAccounting {
    /// 真渲帧数（唯一允许进入真实渲染帧率口径的计数）。
    pub real_frames: u64,
    /// FG/MFG 生成帧数（独立登记面，禁计入真实渲染帧率）。
    pub generated_frames: u64,
    /// 真渲总耗时（秒）。
    pub real_render_seconds: f64,
    /// 生成总耗时（秒）。
    pub generation_seconds: f64,
}

impl FgAccounting {
    /// 真实渲染帧率（口径 0-byte：只含真渲帧/真渲耗时，生成帧禁入）。
    pub fn real_render_fps(&self) -> f64 {
        if self.real_render_seconds <= 0.0 {
            return 0.0;
        }
        self.real_frames as f64 / self.real_render_seconds
    }

    /// presented 帧率（独立新登记面：真渲 + 生成 ÷ 全部耗时；与真实渲染
    /// 帧率并列输出、永不混算）。
    pub fn presented_fps(&self) -> f64 {
        let total = self.real_render_seconds + self.generation_seconds;
        if total <= 0.0 {
            return 0.0;
        }
        (self.real_frames + self.generated_frames) as f64 / total
    }

    /// 账目不变量：presented 帧数恒等式（真渲 + 生成）。
    pub fn presented_frames(&self) -> u64 {
        self.real_frames + self.generated_frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::ssim::ssim;

    /// 平滑确定性合成场景（双线性友好：低频正弦混合）。
    fn smooth_scene(w: u32, h: u32, phase: f32) -> ImageF32 {
        ImageF32::from_fn(w, h, 3, |x, y, ch| {
            let fx = (x as f32 + 0.5) / w as f32;
            let fy = (y as f32 + 0.5) / h as f32;
            let base = 0.5
                + 0.35 * ((fx * 6.0 + phase) * std::f32::consts::PI).sin()
                    * ((fy * 4.0) * std::f32::consts::PI).cos();
            (base + 0.05 * ch as f32).clamp(0.0, 1.0)
        })
    }

    fn zero_mv(w: u32, h: u32) -> ImageF32 {
        ImageF32::new(w, h, 2)
    }

    #[test]
    fn identical_frames_zero_motion_is_identity() {
        let f = smooth_scene(32, 24, 0.3);
        let mv = zero_mv(32, 24);
        let mid = interpolate(&f, &f, &mv, 0.5, &FrameGenParams::default());
        // mv=0 + 双帧相同：纹素中心双线性取样退化为精确读取 → 位级同帧。
        assert_eq!(mid, f, "静止场景插帧必须位级等于真渲帧");
    }

    #[test]
    fn pure_translation_matches_analytic_midframe() {
        // 场景以恒定 uv 速度平移：GT 中帧 = 相位平移一半的解析渲染。
        let (w, h) = (64, 48);
        let shift_u = 2.0 / w as f32; // 每帧 2 像素水平平移
        let render = |k: f32| {
            ImageF32::from_fn(w, h, 3, |x, y, ch| {
                let fx = (x as f32 + 0.5) / w as f32 - k * shift_u;
                let fy = (y as f32 + 0.5) / h as f32;
                let base = 0.5
                    + 0.35 * ((fx * 6.0) * std::f32::consts::PI).sin()
                        * ((fy * 4.0) * std::f32::consts::PI).cos();
                (base + 0.05 * ch as f32).clamp(0.0, 1.0)
            })
        };
        let prev = render(0.0);
        let cur = render(1.0);
        let gt_mid = render(0.5);
        let mv = ImageF32::from_fn(w, h, 2, |_, _, ch| if ch == 0 { shift_u } else { 0.0 });
        let interp = interpolate(&prev, &cur, &mv, 0.5, &FrameGenParams::default());
        let s = ssim(&interp, &gt_mid);
        assert!(s > 0.99, "纯平移插帧 vs 解析 GT 中帧 SSIM={s} 应 > 0.99");
        // 对照下界：frame-hold（复制 prev）必须显著更差。
        let s_hold = ssim(&prev, &gt_mid);
        assert!(s > s_hold, "插帧 {s} 必须优于 frame-hold {s_hold}");
    }

    #[test]
    fn mfg_lane_counts_and_order() {
        let prev = smooth_scene(16, 12, 0.0);
        let cur = smooth_scene(16, 12, 0.5);
        let mv = zero_mv(16, 12);
        for mode_x in 2..=4u32 {
            let params = FrameGenParams {
                inserted_per_pair: mfg_inserted_frames(mode_x),
                ..Default::default()
            };
            let frames = mfg_between(&prev, &cur, &mv, &params);
            assert_eq!(frames.len() as u32, mode_x - 1, "MFG ×{mode_x} 帧数");
        }
    }

    #[test]
    fn occlusion_falls_back_to_nearest_real_frame() {
        // 构造双向失配：prev 全黑、cur 全白，mv=0 → w→0，
        // t=0.25 兜底 prev 样本（黑）、t=0.75 兜底 cur 样本（白）。
        let (w, h) = (8, 8);
        let prev = ImageF32::from_fn(w, h, 3, |_, _, _| 0.0);
        let cur = ImageF32::from_fn(w, h, 3, |_, _, _| 1.0);
        let mv = zero_mv(w, h);
        let p = FrameGenParams {
            consistency_sigma: 0.05,
            ..Default::default()
        };
        let early = interpolate(&prev, &cur, &mv, 0.25, &p);
        let late = interpolate(&prev, &cur, &mv, 0.75, &p);
        assert!(early.get(4, 4, 0) < 0.05, "t<0.5 失配区应兜底 prev（暗）");
        assert!(late.get(4, 4, 0) > 0.95, "t>0.5 失配区应兜底 cur（亮）");
    }

    #[test]
    fn accounting_real_fps_excludes_generated() {
        let acc = FgAccounting {
            real_frames: 10,
            generated_frames: 10,
            real_render_seconds: 1.0,
            generation_seconds: 0.1,
        };
        assert!((acc.real_render_fps() - 10.0).abs() < 1e-12, "真实渲染帧率禁计生成帧");
        assert!((acc.presented_fps() - 20.0 / 1.1).abs() < 1e-9, "presented 独立口径");
        assert_eq!(acc.presented_frames(), 20);
    }

    #[test]
    fn double_run_bitexact() {
        let prev = smooth_scene(48, 32, 0.1);
        let cur = smooth_scene(48, 32, 0.9);
        let mv = ImageF32::from_fn(48, 32, 2, |x, y, ch| {
            0.01 * ((x + y + ch) % 5) as f32 - 0.02
        });
        let p = FrameGenParams {
            inserted_per_pair: 3,
            ..Default::default()
        };
        let run1 = mfg_between(&prev, &cur, &mv, &p);
        let run2 = mfg_between(&prev, &cur, &mv, &p);
        assert_eq!(run1, run2, "双跑必须位级一致");
    }
}
