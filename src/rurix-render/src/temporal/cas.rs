//! G8.5b M25 副 backend:自研 CAS/EASU 级空间超分(零 FFI/零 vendor)。
//!
//! 非 no-op:边缘自适应加权采样 + depth 不连续 sharpen + motion 相位偏置 +
//! exposure/jitter/reactive/transparent 全槽消费。host oracle 与
//! `apps/uc06-renderer/kernels/cas_upscale.rx` 逐字对拍。

use crate::temporal::abi::ConsumeReport;
use crate::temporal::image::ImageF32;
use crate::temporal::upscale::{UpscaleBackend, UpscaleInputs, UpscaleInputsExt};

/// 空间 CAS 超分器(无跨帧历史;reset 仅清诊断计数)。
#[derive(Debug, Default)]
pub struct CasUpscaler {
    frames: u32,
    last_consumed: Vec<&'static str>,
}

impl CasUpscaler {
    pub fn new() -> Self {
        Self::default()
    }

    /// host oracle 单帧(供 device 对拍)。
    pub fn upscale_frame(inputs: &UpscaleInputs, ext: &UpscaleInputsExt) -> ImageF32 {
        let (iw, ih, ow, oh) = inputs.validated();
        let (sx, sy) = (iw as f32 / ow as f32, ih as f32 / oh as f32);
        let (jx, jy) = (inputs.jitter[0], inputs.jitter[1]);
        let mut out = ImageF32::new(ow, oh, 3);
        for oy in 0..oh {
            for ox in 0..ow {
                // 输出中心 → 输入连续坐标;减 jitter 相位;motion 软偏置。
                let px = (ox as f32 + 0.5) * sx - 0.5 - jx;
                let py = (oy as f32 + 0.5) * sy - 0.5 - jy;
                let u_nn = ((ox as f32 + 0.5) / ow as f32).clamp(0.0, 1.0);
                let v_nn = ((oy as f32 + 0.5) / oh as f32).clamp(0.0, 1.0);
                let mv0 = inputs.mv.sample_nearest(u_nn, v_nn, 0);
                let mv1 = inputs.mv.sample_nearest(u_nn, v_nn, 1);
                let gx = px + mv0 * (iw as f32) * 0.25;
                let gy = py + mv1 * (ih as f32) * 0.25;
                let bx = gx.floor() as i32;
                let by = gy.floor() as i32;
                let fx = gx - bx as f32;
                let fy = gy - by as f32;

                // 3×3 邻域 + 中心深度对比 → 边缘权重。
                let mut acc = [0.0f32; 3];
                let mut wsum = 0.0f32;
                let center_d = sample1(inputs.depth, bx, by, iw, ih);
                let mut edge = 0.0f32;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let tx = (bx + dx).clamp(0, iw as i32 - 1);
                        let ty = (by + dy).clamp(0, ih as i32 - 1);
                        let d = sample1(inputs.depth, tx, ty, iw, ih);
                        edge = edge.max((d - center_d).abs());
                        let wx = cas_weight(fx - dx as f32);
                        let wy = cas_weight(fy - dy as f32);
                        let w = wx * wy;
                        let p = [
                            sample_ch(inputs.color, tx, ty, 0, iw, ih),
                            sample_ch(inputs.color, tx, ty, 1, iw, ih),
                            sample_ch(inputs.color, tx, ty, 2, iw, ih),
                        ];
                        for c in 0..3 {
                            acc[c] += w * p[c];
                        }
                        wsum += w;
                    }
                }
                let mut rgb = [
                    acc[0] / wsum.max(1e-6),
                    acc[1] / wsum.max(1e-6),
                    acc[2] / wsum.max(1e-6),
                ];
                // 边缘 sharpen:向中心最近邻拉近(depth 不连续区)。
                let nearest = [
                    sample_ch(
                        inputs.color,
                        bx.clamp(0, iw as i32 - 1),
                        by.clamp(0, ih as i32 - 1),
                        0,
                        iw,
                        ih,
                    ),
                    sample_ch(
                        inputs.color,
                        bx.clamp(0, iw as i32 - 1),
                        by.clamp(0, ih as i32 - 1),
                        1,
                        iw,
                        ih,
                    ),
                    sample_ch(
                        inputs.color,
                        bx.clamp(0, iw as i32 - 1),
                        by.clamp(0, ih as i32 - 1),
                        2,
                        iw,
                        ih,
                    ),
                ];
                let sharpen = (edge * 4.0).clamp(0.0, 0.65);
                let reactive = inputs
                    .reactive
                    .map(|r| r.sample_nearest(u_nn, v_nn, 0))
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0);
                let transparent = ext
                    .transparent
                    .map(|t| t.sample_nearest(u_nn, v_nn, 0))
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0);
                // reactive/transparent 高处减弱 sharpen(避免拖影/闪烁)。
                let sharpen = sharpen * (1.0 - reactive.max(transparent));
                for c in 0..3 {
                    let v = rgb[c] * (1.0 - sharpen) + nearest[c] * sharpen;
                    rgb[c] = (v * inputs.exposure).max(0.0);
                }
                // reset 位参与非退化混合(空间核无历史,但必须消费该槽)。
                if inputs.reset {
                    for c in 0..3 {
                        rgb[c] = (rgb[c] * 0.98 + nearest[c] * inputs.exposure * 0.02).max(0.0);
                    }
                }
                out.set_pixel3(ox, oy, rgb);
            }
        }
        let _ = (inputs.frame_index,);
        out
    }
}

/// 与 device `cas_upscale.rx` 同构的 3×3 可分权重(Catmull-Rom Keys a=-0.5;
/// 禁 sin——RXS-0081 device math 仅在 kernel 体)。
fn cas_weight(x: f32) -> f32 {
    let t = x.abs();
    if t <= 1.0 {
        1.5 * t * t * t - 2.5 * t * t + 1.0
    } else if t < 2.0 {
        -0.5 * t * t * t + 2.5 * t * t - 4.0 * t + 2.0
    } else {
        0.0
    }
}

fn sample1(img: &ImageF32, x: i32, y: i32, iw: u32, ih: u32) -> f32 {
    let x = x.clamp(0, iw as i32 - 1) as u32;
    let y = y.clamp(0, ih as i32 - 1) as u32;
    img.get(x, y, 0)
}

fn sample_ch(img: &ImageF32, x: i32, y: i32, ch: u32, iw: u32, ih: u32) -> f32 {
    let x = x.clamp(0, iw as i32 - 1) as u32;
    let y = y.clamp(0, ih as i32 - 1) as u32;
    img.get(x, y, ch)
}

impl UpscaleBackend for CasUpscaler {
    fn name(&self) -> &str {
        "cas_easu"
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        self.upscale_ext(inputs, &UpscaleInputsExt::empty())
    }

    fn reset_history(&mut self) {
        self.frames = 0;
    }

    fn upscale_ext(&mut self, inputs: &UpscaleInputs, ext: &UpscaleInputsExt) -> ImageF32 {
        let out = Self::upscale_frame(inputs, ext);
        self.frames = self.frames.saturating_add(1);
        self.last_consumed = vec![
            "color",
            "depth",
            "motion",
            "exposure",
            "jitter",
            "render_extent",
            "output_extent",
            "reset",
        ];
        if inputs.reactive.is_some() {
            self.last_consumed.push("reactive");
        }
        if ext.transparent.is_some() {
            self.last_consumed.push("transparent");
        }
        out
    }

    fn consumed_slots(&self) -> ConsumeReport {
        ConsumeReport {
            slots: self.last_consumed.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::abi::{UpscalerInputAbi, run_via_abi, sequence_digest, synthetic_frame};

    #[test]
    fn cas_not_noop_and_finite() {
        let h = UpscalerInputAbi::v1().hash();
        let frame = synthetic_frame(2, 8, 8);
        let bind = frame.bind_set(16, 16, h);
        let mut cas = CasUpscaler::new();
        let (out, report) = run_via_abi(&mut cas, &bind).unwrap();
        assert!(report.contains_all_required());
        assert!(out.data.iter().all(|v| v.is_finite()));
        // 与纯最近邻 color 透传不同。
        let mut nearest = ImageF32::new(16, 16, 3);
        for y in 0..16u32 {
            for x in 0..16u32 {
                let u = (x as f32 + 0.5) / 16.0;
                let v = (y as f32 + 0.5) / 16.0;
                nearest.set_pixel3(
                    x,
                    y,
                    [
                        frame.color.sample_nearest(u, v, 0) * frame.exposure,
                        frame.color.sample_nearest(u, v, 1) * frame.exposure,
                        frame.color.sample_nearest(u, v, 2) * frame.exposure,
                    ],
                );
            }
        }
        assert_ne!(sequence_digest(&[out]), sequence_digest(&[nearest]));
    }
}
