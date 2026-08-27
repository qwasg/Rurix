//! 中性（neutral）view transform 插件（G9.5 M118；RFC-0025 §4.I；RXS-0369 L2）。
//!
//! 参考公式 = Khronos PBR Neutral tone mapping（KhronosGroup/ToneMapping
//! `PBR_Neutral/pbrNeutral.glsl`）host 逐字移植：输入 Linear Rec.709 → 输出
//! Linear Rec.709 [0,1]（起始压缩点 0.76、desaturation 0.15、toe 偏移段逐字）;
//! 显示编码由共享输出编码承担。

/// Khronos PBR Neutral 逐字参数。
const START_COMPRESSION: f64 = 0.8 - 0.04;
const DESATURATION: f64 = 0.15;

/// 中性插件（Khronos PBR Neutral）。
pub struct Neutral;

impl Neutral {
    /// `PBRNeutralToneMapping` 逐字。
    fn pbr_neutral(color: [f64; 3]) -> [f64; 3] {
        let x = color[0].min(color[1]).min(color[2]);
        let offset = if x < 0.08 { x - 6.25 * x * x } else { 0.04 };
        let mut c = [color[0] - offset, color[1] - offset, color[2] - offset];
        let peak = c[0].max(c[1]).max(c[2]);
        if peak < START_COMPRESSION {
            return c;
        }
        let d = 1.0 - START_COMPRESSION;
        let new_peak = 1.0 - d * d / (peak + d - START_COMPRESSION);
        let ratio = new_peak / peak;
        for v in c.iter_mut() {
            *v *= ratio;
        }
        let g = 1.0 - 1.0 / (DESATURATION * (peak - new_peak) + 1.0);
        [
            c[0] + (new_peak - c[0]) * g,
            c[1] + (new_peak - c[1]) * g,
            c[2] + (new_peak - c[2]) * g,
        ]
    }
}

impl super::view_transform::ViewTransform for Neutral {
    fn id(&self) -> &'static str {
        "neutral"
    }

    fn display_name(&self) -> &'static str {
        "Neutral (Khronos PBR Neutral host 逐字)"
    }

    fn to_display_linear(&self, hdr_linear: [f64; 3]) -> [f64; 3] {
        let v = Self::pbr_neutral(hdr_linear);
        [
            v[0].clamp(0.0, 1.0),
            v[1].clamp(0.0, 1.0),
            v[2].clamp(0.0, 1.0),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::view_transform::ViewTransform;

    //@ spec: RXS-0369
    #[test]
    fn neutral_known_output_landmarks() {
        // 起始压缩点以下恒等(减去 offset 后):0.18 → offset=0.04 ⇒ 0.14(逐字)。
        let p = Neutral;
        let g = p.to_display_linear([0.18, 0.18, 0.18]);
        assert!((g[0] - 0.14).abs() < 1e-12, "0.18 → {g:?}(逐字 0.14)");
        // 低端 toe:x<0.08 段。
        let d = p.to_display_linear([0.05, 0.05, 0.05]);
        assert!((d[0] - 6.25 * 0.05 * 0.05).abs() < 1e-12, "toe 段: {d:?}");
        // 高光压缩:1.0 白 → ≈0.869(PBR Neutral 公布行为:渐近 1 不达);
        // 超白 → 更近 1 但 <1。
        let w = p.to_display_linear([1.0, 1.0, 1.0]);
        assert!(w[0] > 0.8 && w[0] < 0.95, "白压缩: {w:?}");
        let sw = p.to_display_linear([64.0, 64.0, 64.0]);
        assert!(sw[0] > 0.99 && sw[0] < 1.0, "超白渐近: {sw:?}");
        // 确定性。
        assert_eq!(
            p.to_display_linear([1.0, 0.5, 0.25]),
            p.to_display_linear([1.0, 0.5, 0.25])
        );
    }
}
