//! SSIM 结构相似度门禁(报告7 §5 验证方法一「SSIM ≥ 阈值」的可执行版本;
//! RFC-0016 §4.H3 验收:SSIM 门禁,host 参考实现;门 G-G5-7)。
//!
//! 标准 SSIM(Wang et al. 2004):滑动窗内比较亮度(均值)、对比度(方差)、
//! 结构(协方差)三项,全图取均值(MSSIM)。本实现取 **8×8 均值窗简化版**
//! (均匀权重的盒式窗;依据:盒式窗与 11×11 高斯窗在门禁口径下结论一致,
//! 且盒式窗无高斯系数表,确定性最直白——门禁只需要稳定可复现的相对度量,
//! 不追求与某一下游实现逐位一致)。
//!
//! 常数依据(照 Wang 原文):C1 = (K1·L)²,C2 = (K2·L)²,K1 = 0.01,K2 = 0.03;
//! L = 动态范围——host 参考实现输入为 \[0,1\] f32 显示域颜色,故 L = 1.0
//! (原文 8-bit L = 255 的等价归一化)。

use crate::temporal::image::ImageF32;

/// SSIM 参数(默认即门禁口径;调参只影响单测自定场景,不动门禁默认值)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SsimParams {
    /// 窗边长(默认 8;图像任一边不足窗长时窗收缩到 min(w, h))。
    pub window: u32,
    /// 亮度稳定常数系数 K1(默认 0.01,Wang 2004)。
    pub k1: f64,
    /// 对比度/结构稳定常数系数 K2(默认 0.03,Wang 2004)。
    pub k2: f64,
    /// 动态范围 L(默认 1.0:\[0,1\] f32 显示域)。
    pub data_range: f64,
}

impl Default for SsimParams {
    fn default() -> Self {
        Self {
            window: 8,
            k1: 0.01,
            k2: 0.03,
            data_range: 1.0,
        }
    }
}

/// 默认参数 SSIM(3 通道 RGB,同尺寸;返回全图 MSSIM,∈ \[-1, 1\])。
pub fn ssim(a: &ImageF32, b: &ImageF32) -> f64 {
    ssim_with(a, b, &SsimParams::default())
}

/// 带参 SSIM。逐锚点逐通道求 SSIM 后取全均值(f64 累加,确定性)。
pub fn ssim_with(a: &ImageF32, b: &ImageF32, params: &SsimParams) -> f64 {
    assert!(a.c == 3 && a.same_shape(b), "SSIM 输入必须 3 通道同尺寸");
    assert!(params.window >= 2, "SSIM 窗必须 ≥2");
    let (w, h) = (a.w, a.h);
    let win = params.window.min(w).min(h);
    let c1 = (params.k1 * params.data_range).powi(2);
    let c2 = (params.k2 * params.data_range).powi(2);
    let n = f64::from(win * win);
    let anchors_x = w - win + 1;
    let anchors_y = h - win + 1;
    let mut acc = 0.0f64;
    let mut count = 0u64;
    for ay in 0..anchors_y {
        for ax in 0..anchors_x {
            for ch in 0..3 {
                let mut sum_a = 0.0f64;
                let mut sum_b = 0.0f64;
                let mut sq_a = 0.0f64;
                let mut sq_b = 0.0f64;
                let mut cross = 0.0f64;
                for dy in 0..win {
                    for dx in 0..win {
                        let va = f64::from(a.get(ax + dx, ay + dy, ch));
                        let vb = f64::from(b.get(ax + dx, ay + dy, ch));
                        sum_a += va;
                        sum_b += vb;
                        sq_a += va * va;
                        sq_b += vb * vb;
                        cross += va * vb;
                    }
                }
                let mu_a = sum_a / n;
                let mu_b = sum_b / n;
                let var_a = (sq_a / n - mu_a * mu_a).max(0.0);
                let var_b = (sq_b / n - mu_b * mu_b).max(0.0);
                let cov = cross / n - mu_a * mu_b;
                let num = (2.0 * mu_a * mu_b + c1) * (2.0 * cov + c2);
                let den = (mu_a * mu_a + mu_b * mu_b + c1) * (var_a + var_b + c2);
                acc += num / den;
                count += 1;
            }
        }
    }
    acc / count as f64
}

/// G-G5-7 门禁封装:阈值 + 参数;`passes` 为判定入口(CI/单测共用口径)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SsimGate {
    /// 通过阈值(如静态收敛门 0.9,报告7 §5 收敛正确性口径)。
    pub threshold: f64,
    /// 计算参数(默认 [`SsimParams::default`])。
    pub params: SsimParams,
}

impl SsimGate {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            params: SsimParams::default(),
        }
    }

    /// 计算得分(不自判,便于打点记录实际值)。
    pub fn score(&self, a: &ImageF32, b: &ImageF32) -> f64 {
        ssim_with(a, b, &self.params)
    }

    /// 门禁判定:score ≥ threshold。
    pub fn passes(&self, a: &ImageF32, b: &ImageF32) -> bool {
        self.score(a, b) >= self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(w: u32, h: u32) -> ImageF32 {
        // 棋盘 + 渐变 + 高频细节(确定性合成场景,与 tsr 收敛测试同族)
        ImageF32::from_fn(w, h, 3, |x, y, ch| {
            let check = ((x / 8) + (y / 8)) % 2;
            let base = 0.15 + 0.6 * check as f32;
            let grad = 0.1 * (x as f32 * 0.08).sin() * (y as f32 * 0.06).cos();
            (base * (0.8 + 0.1 * ch as f32) + grad).clamp(0.0, 1.0)
        })
    }

    /// 确定性伪噪声(分裂 base 的位混合;无 rand 依赖,全平台一致)。
    fn det_noise(x: u32, y: u32, ch: u32, seed: u32) -> f32 {
        let mut v = x.wrapping_mul(0x9E37_79B9)
            ^ y.wrapping_mul(0x85EB_CA6B)
            ^ ch.wrapping_mul(0xC2B2_AE35)
            ^ seed;
        v ^= v >> 16;
        v = v.wrapping_mul(0x7FEB_352D);
        v ^= v >> 15;
        (v % 2001) as f32 / 1000.0 - 1.0 // ∈ [-1, 1]
    }

    #[test]
    fn ssim_identical_is_one() {
        let a = scene(32, 24);
        assert!((ssim(&a, &a) - 1.0).abs() < 1e-12, "相同图 SSIM 必须 =1");
    }

    #[test]
    fn ssim_small_image_window_shrinks() {
        // 4×4 小于默认 8×8 窗 → 窗收缩,仍正常工作
        let a = scene(4, 4);
        assert!((ssim(&a, &a) - 1.0).abs() < 1e-12);
        let b = ImageF32::from_fn(4, 4, 3, |x, y, ch| {
            (a.get(x, y, ch) + 0.3 * det_noise(x, y, ch, 7)).clamp(0.0, 1.0)
        });
        let s = ssim(&a, &b);
        assert!(s < 1.0 && s > -1.0, "扰动图 SSIM 应 ∈ (-1,1):{s}");
    }

    #[test]
    fn ssim_noise_below_half() {
        let a = scene(32, 32);
        let noisy = ImageF32::from_fn(32, 32, 3, |x, y, ch| {
            (a.get(x, y, ch) + det_noise(x, y, ch, 42)).clamp(0.0, 1.0)
        });
        let s = ssim(&a, &noisy);
        assert!(s < 0.5, "重噪声图 SSIM={s} 应 < 0.5");
    }

    #[test]
    fn ssim_monotonic_in_noise_amplitude() {
        // 噪声幅度递增 → SSIM 严格递减(sanity 单调性)
        let a = scene(32, 32);
        let mut prev = 2.0f64;
        for &amp in &[0.02f32, 0.1, 0.3, 0.6] {
            let noisy = ImageF32::from_fn(32, 32, 3, |x, y, ch| {
                (a.get(x, y, ch) + amp * det_noise(x, y, ch, 11)).clamp(0.0, 1.0)
            });
            let s = ssim(&a, &noisy);
            assert!(s < prev, "amp={amp}: {s} 应 < 前一级 {prev}");
            prev = s;
        }
    }

    #[test]
    fn ssim_blur_close_noise_far() {
        // 排序 sanity:轻微平滑(保结构)得分应远高于加噪(坏结构)
        let a = scene(32, 32);
        let blurred = ImageF32::from_fn(32, 32, 3, |x, y, ch| {
            let xm = x.saturating_sub(1);
            let xp = (x + 1).min(31);
            let ym = y.saturating_sub(1);
            let yp = (y + 1).min(31);
            (a.get(x, y, ch) * 4.0
                + a.get(xm, y, ch)
                + a.get(xp, y, ch)
                + a.get(x, ym, ch)
                + a.get(x, yp, ch))
                / 8.0
        });
        let noisy = ImageF32::from_fn(32, 32, 3, |x, y, ch| {
            (a.get(x, y, ch) + 0.15 * det_noise(x, y, ch, 3)).clamp(0.0, 1.0)
        });
        let (sb, sn) = (ssim(&a, &blurred), ssim(&a, &noisy));
        assert!(sb > sn, "模糊 {sb} 应 > 噪声 {sn}");
    }

    #[test]
    fn gate_threshold_semantics() {
        let a = scene(24, 24);
        let gate = SsimGate::new(0.99);
        assert!(gate.passes(&a, &a));
        let other = scene(24, 24).clone();
        let flipped = ImageF32::from_fn(24, 24, 3, |x, y, ch| 1.0 - other.get(x, y, ch));
        let s = gate.score(&a, &flipped);
        assert!(s < 0.99 && !gate.passes(&a, &flipped), "反相图不得过门");
    }
}
