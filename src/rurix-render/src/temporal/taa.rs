//! TAA 参考实现(报告7 P0「公共底座 + 自研 TAA」;RFC-0016 章 H 前半)。
//!
//! 底座之上的薄 pass:历史按 MV 双线性重采样 → YCoCg 3x3 邻域裁剪 → 按历史
//! 验证 mask 与 alpha 混合。静态场景收敛对拍超采样参考是硬验收(报告7 §5
//! 验证方法一,门 G-G5-7);device shader 以本实现为对拍金标准。

use crate::temporal::common::{
    neighborhood_aabb, neighborhood_variance_bounds, rgb_image_to_ycocg, rgb_to_ycocg, ycocg_to_rgb,
};
use crate::temporal::image::ImageF32;

/// 历史邻域裁剪模式(报告7 §2.1:邻域裁剪是鬼影的直接克星)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClampMode {
    /// 3x3 邻域 AABB(YCoCg 空间)——报告7 标准做法。
    #[default]
    Aabb,
    /// 方差裁剪(μ ± γσ,边界见 [`neighborhood_variance_bounds`])。
    Variance,
}

/// TAA 参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaaParams {
    /// 当前帧混合权重(历史权重 = 1 - alpha;默认 0.1,有效累积窗约 10 帧)。
    pub blend_alpha: f32,
    /// 邻域裁剪模式(默认 Aabb)。
    pub clamp_mode: ClampMode,
    /// 方差裁剪 γ(仅 Variance 模式生效,默认 1.0)。
    pub variance_gamma: f32,
}

impl Default for TaaParams {
    fn default() -> Self {
        Self {
            blend_alpha: 0.1,
            clamp_mode: ClampMode::Aabb,
            variance_gamma: 1.0,
        }
    }
}

/// TAA resolve:当前帧(带 jitter 采样)+ 历史 + MV + 验证 mask → 抗锯齿输出。
///
/// `cur`/`history` 为 3 通道 RGB,`mv` 2 通道 uv 偏移(历史采样位置 = uv+mv,
/// 见 [`crate::temporal::common::compute_camera_mv`]),`validity` 1 通道 0/1
/// (见 [`crate::temporal::common::validate_history_with_mv`])。validity = 0 的
/// 像素(disocclusion/深度/法线突变)直接取当前帧——宁可锯齿回归也不拖影
/// (报告7 §2.3 reactive mask 同原则)。
pub fn taa_resolve(
    cur: &ImageF32,
    history: &ImageF32,
    mv: &ImageF32,
    validity: &ImageF32,
    params: &TaaParams,
) -> ImageF32 {
    assert!(
        cur.c == 3 && history.same_shape(cur) && mv.c == 2 && validity.c == 1,
        "输入通道/形状不符"
    );
    assert!(
        mv.w == cur.w && mv.h == cur.h && validity.w == cur.w && validity.h == cur.h,
        "输入尺寸不符"
    );
    let (w, h) = (cur.w, cur.h);
    let (fw, fh) = (w as f32, h as f32);
    // 邻域统计在 YCoCg 空间(报告7 标准做法);cur/history 皆转换
    let cur_ycc = rgb_image_to_ycocg(cur);
    let (lo, hi) = match params.clamp_mode {
        ClampMode::Aabb => neighborhood_aabb(&cur_ycc),
        ClampMode::Variance => neighborhood_variance_bounds(&cur_ycc, params.variance_gamma),
    };
    let alpha = params.blend_alpha;
    let mut out = ImageF32::new(w, h, 3);
    for y in 0..h {
        for x in 0..w {
            if validity.get(x, y, 0) < 0.5 {
                // 历史不可信:直接取当前帧
                out.set_pixel3(x, y, cur.pixel3(x, y));
                continue;
            }
            let u = (x as f32 + 0.5) / fw;
            let v = (y as f32 + 0.5) / fh;
            let hist_rgb = history.sample_bilinear3(u + mv.get(x, y, 0), v + mv.get(x, y, 1));
            let hist_ycc = rgb_to_ycocg(hist_rgb);
            // 历史色钳入当前帧 3x3 邻域色域(鬼影抑制)
            let hc = [
                hist_ycc[0].clamp(lo.get(x, y, 0), hi.get(x, y, 0)),
                hist_ycc[1].clamp(lo.get(x, y, 1), hi.get(x, y, 1)),
                hist_ycc[2].clamp(lo.get(x, y, 2), hi.get(x, y, 2)),
            ];
            let cc = cur_ycc.pixel3(x, y);
            let blended = [
                alpha * cc[0] + (1.0 - alpha) * hc[0],
                alpha * cc[1] + (1.0 - alpha) * hc[1],
                alpha * cc[2] + (1.0 - alpha) * hc[2],
            ];
            out.set_pixel3(x, y, ycocg_to_rgb(blended));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::common::{jitter_sequence, validate_history_with_mv};

    // -----------------------------------------------------------------------
    // 合成静态场景:棋盘格(相位偏移 3.7px,格子边界切割像素=抗锯齿主战场)
    // + 低频渐变 + 高频正弦细节。shade 定义在连续坐标上,jitter/超采样都
    // 是它的采样器。
    // -----------------------------------------------------------------------

    fn shade(fx: f32, fy: f32) -> [f32; 3] {
        let check = (((fx + 3.7) / 8.0).floor() as i32 + ((fy + 3.7) / 8.0).floor() as i32) & 1;
        let base = 0.15 + 0.7 * check as f32;
        let grad = 0.15 * (fx * 0.05).sin() * (fy * 0.07).cos();
        let detail = 0.05 * (fx * 0.9).sin() * (fy * 1.1).sin();
        [
            (base + grad + detail).clamp(0.0, 1.0),
            (0.8 * base + 0.5 * grad + detail).clamp(0.0, 1.0),
            (0.6 * base - grad + detail).clamp(0.0, 1.0),
        ]
    }

    /// 参考:每像素 4x4 超采样(收敛对拍金标准,报告7 §5 验证方法一)。
    fn render_reference(w: u32, h: u32) -> ImageF32 {
        ImageF32::from_fn(w, h, 3, |x, y, ch| {
            let mut acc = 0.0f32;
            for sy in 0..4 {
                for sx in 0..4 {
                    acc += shade(
                        x as f32 + (sx as f32 + 0.5) / 4.0,
                        y as f32 + (sy as f32 + 0.5) / 4.0,
                    )[ch as usize];
                }
            }
            acc / 16.0
        })
    }

    /// 每帧:jitter 后单采样 shade(等价相机投影矩阵 jitter 的采样口径)。
    fn render_jittered(w: u32, h: u32, jitter: [f32; 2]) -> ImageF32 {
        ImageF32::from_fn(w, h, 3, |x, y, ch| {
            shade(x as f32 + 0.5 + jitter[0], y as f32 + 0.5 + jitter[1])[ch as usize]
        })
    }

    fn full_validity(w: u32, h: u32) -> ImageF32 {
        ImageF32::from_fn(w, h, 1, |_, _, _| 1.0)
    }

    /// 跑 N 帧静态场景 TAA,返回逐帧输出。
    fn run_static_taa(w: u32, h: u32, frames: usize, params: &TaaParams) -> Vec<ImageF32> {
        let jitters = jitter_sequence(frames as u32);
        let mv = ImageF32::new(w, h, 2);
        let validity = full_validity(w, h);
        let mut history: Option<ImageF32> = None;
        let mut outs = Vec::with_capacity(frames);
        for &j in &jitters {
            let cur = render_jittered(w, h, j);
            let out = match &history {
                None => cur.clone(),
                Some(hist) => taa_resolve(&cur, hist, &mv, &validity, params),
            };
            history = Some(out.clone());
            outs.push(out);
        }
        outs
    }

    #[test]
    fn taa_converges_static_scene() {
        // 门 G-G5-7 核心验收:静态场景 32 帧 TAA,终帧与 4x4 超采样参考的
        // MSE < 未抗锯齿单帧 MSE 的 25%,且逐帧 MSE 呈下降趋势。
        let (w, h) = (64u32, 64u32);
        let reference = render_reference(w, h);
        for clamp_mode in [ClampMode::Aabb, ClampMode::Variance] {
            let params = TaaParams {
                clamp_mode,
                ..TaaParams::default()
            };
            let outs = run_static_taa(w, h, 32, &params);
            let no_aa_mse =
                ImageF32::mse(&render_jittered(w, h, jitter_sequence(1)[0]), &reference);
            let mses: Vec<f64> = outs.iter().map(|o| ImageF32::mse(o, &reference)).collect();
            let final_mse = mses[31];
            assert!(
                final_mse < 0.25 * no_aa_mse,
                "{clamp_mode:?}: final={final_mse:.6} 应 < 25% × no_aa={no_aa_mse:.6}"
            );
            // 单调下降趋势(允许小抖动):末 8 帧均值 < 首 8 帧均值
            let first_avg = mses[..8].iter().sum::<f64>() / 8.0;
            let last_avg = mses[24..].iter().sum::<f64>() / 8.0;
            assert!(
                last_avg < first_avg,
                "{clamp_mode:?}: 首段 {first_avg:.6} 应 > 末段 {last_avg:.6}"
            );
            eprintln!(
                "[{clamp_mode:?}] no_aa_mse={no_aa_mse:.6} final_mse={final_mse:.6} \
                 ratio={:.4} first8={first_avg:.6} last8={last_avg:.6}",
                final_mse / no_aa_mse
            );
        }
    }

    #[test]
    fn taa_disocclusion_uses_current_frame() {
        // 移动遮挡物合成场景:揭露区深度突变 → validity 置 0 → 直接用当前帧,
        // 无鬼影(报告7 §2.1 三不信场景一:遮挡揭开)。
        let (w, h) = (32u32, 16u32);
        let bg = [0.1f32, 0.1, 0.1];
        let fg = [0.9f32, 0.9, 0.9];
        // 8px 亮方块遮挡物,帧 t 占据 x ∈ [t, t+8)
        let render = |t: u32| {
            ImageF32::from_fn(w, h, 3, |x, _, ch| {
                if x >= t && x < t + 8 {
                    fg[ch as usize]
                } else {
                    bg[ch as usize]
                }
            })
        };
        let depth_of = |t: u32| {
            ImageF32::from_fn(
                w,
                h,
                1,
                |x, _, _| if x >= t && x < t + 8 { 0.3 } else { 0.9 },
            )
        };
        let normal = ImageF32::from_fn(w, h, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        // 帧 0 → 帧 1:遮挡物右移 1px;MV = 0(物体 MV 缺失的最恶劣情形,
        // 深度验证必须兜住)
        let cur = render(1);
        let history = render(0);
        let mv = ImageF32::new(w, h, 2);
        let validity =
            validate_history_with_mv(&depth_of(1), &depth_of(0), &normal, &normal, &mv, 0.1, 0.9);
        let out = taa_resolve(&cur, &history, &mv, &validity, &TaaParams::default());
        // 验证 mask 语义:x=0 揭露(0.3→0.9 深度突变),x=8 新遮挡(0.9→0.3)
        assert!(validity.get(0, 4, 0) < 0.5, "揭露列应 invalid");
        assert!(validity.get(8, 4, 0) < 0.5, "新遮挡列应 invalid");
        for x in 1..8 {
            assert!(validity.get(x, 4, 0) > 0.5, "({x}) 遮挡物内部应 valid");
        }
        for x in 9..w {
            assert!(validity.get(x, 4, 0) > 0.5, "({x}) 背景应 valid");
        }
        // 无鬼影:揭露区输出 == 当前帧(亮历史残影 = 0.82,若混入必超差)
        for ch in 0..3 {
            assert!((out.pixel3(0, 4)[ch] - bg[ch]).abs() < 1e-6, "揭露列鬼影");
            assert!(
                (out.pixel3(8, 4)[ch] - fg[ch]).abs() < 1e-6,
                "新遮挡列应取当前帧"
            );
        }
        // 不变区域保持混合结果:遮挡物内部 ≈ 亮,远处背景 ≈ 暗
        for ch in 0..3 {
            assert!(out.pixel3(4, 4)[ch] > 0.8, "遮挡物内部应为亮");
            assert!(out.pixel3(20, 4)[ch] < 0.2, "远处背景应为暗");
        }
    }

    #[test]
    fn taa_jitter_residual_far_below_unfiltered() {
        // 静态场景 TAA 后帧间差(第 31 vs 32 帧)远小于无 TAA 抖动帧间差
        // (时域稳定性;报告7 §2.2 闪烁时域分析的对立面)。
        let (w, h) = (64u32, 64u32);
        let outs = run_static_taa(w, h, 32, &TaaParams::default());
        let taa_diff = ImageF32::mse(&outs[30], &outs[31]);
        let jitters = jitter_sequence(32);
        let raw_diff = ImageF32::mse(
            &render_jittered(w, h, jitters[30]),
            &render_jittered(w, h, jitters[31]),
        );
        assert!(
            taa_diff < 0.05 * raw_diff,
            "taa_diff={taa_diff:.8} 应 << raw_diff={raw_diff:.8}"
        );
        eprintln!("[jitter_residual] taa_diff={taa_diff:.8} raw_diff={raw_diff:.8}");
    }
}
