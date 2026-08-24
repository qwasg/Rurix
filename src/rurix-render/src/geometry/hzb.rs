//! HZB 层级深度金字塔遮挡剔除 host 参考臂（RFC-0037；G20.2 M-a；
//! 本模块兑现 mod.rs 头注「HZB 两阶段 P3 预留」的第一阶段 host 面）。
//!
//! ## 约定
//!
//! 深度域 = \[0,1\] ZO；默认 **reverse-Z**（值越大越近，与
//! [`crate::geometry::visbuffer`] 的 reverse-Z 30 位量化同向），同时提供
//! standard-Z 变体（值越小越近）。金字塔每级纹素保存其 footprint 内**最远**
//! 深度（reverse-Z 取 min / standard-Z 取 max）——保守遮挡语义的唯一合法
//! 归约方向：被测物 rect 的最近点比 rect 覆盖域内「最远遮挡深度」还远 ⇔
//! rect 内每个像素的场景深度都比被测物近 ⇔ 完全遮挡。
//!
//! ## 保守性硬不变量（M-a 程序产判据）
//!
//! `test_rect` 判 [`Occlusion::Occluded`] ⇒ 逐像素精确真值必同判遮挡
//! （**零假阳性**：不得剔除任何可见物；漏剔合法——保守性只损效率不损正确）。
//!
//! 纯 f32/f64 host 确定性；device kernel 车道 = RFC-0037 out-of-scope 登记。

use crate::temporal::image::ImageF32;

/// 深度约定（reverse-Z 为渲染器默认，与 visbuffer 30 位量化同向）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthConvention {
    /// 值越大越近（1.0 = 近平面；farther-of = min）。
    ReverseZ,
    /// 值越小越近（0.0 = 近平面；farther-of = max）。
    StandardZ,
}

impl DepthConvention {
    /// 归约二值：取「更远」者。
    fn farther(self, a: f32, b: f32) -> f32 {
        match self {
            DepthConvention::ReverseZ => a.min(b),
            DepthConvention::StandardZ => a.max(b),
        }
    }

    /// a 是否严格比 b 更远。
    fn is_farther(self, a: f32, b: f32) -> bool {
        match self {
            DepthConvention::ReverseZ => a < b,
            DepthConvention::StandardZ => a > b,
        }
    }
}

/// 遮挡判定两态（保守：Visible = 「不能证明被遮」，非「必可见」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Occlusion {
    Occluded,
    Visible,
}

/// HZB 金字塔（mip0 = 全分辨率拷贝；mip k 纹素 = 下级 2×2 最远深度，
/// 非 2 幂边按 ceil 减半、越界 clamp 复采边纹素——保守方向不变）。
#[derive(Debug, Clone)]
pub struct HzbPyramid {
    pub conv: DepthConvention,
    /// mips\[0\] 为全分辨率；每级 c=1。
    pub mips: Vec<ImageF32>,
}

impl HzbPyramid {
    /// 从单通道深度图构建全金字塔（直至 1×1）。
    pub fn build(depth: &ImageF32, conv: DepthConvention) -> Self {
        assert!(depth.c == 1, "HZB 输入必须单通道深度");
        let mut mips = vec![depth.clone()];
        while mips.last().unwrap().w > 1 || mips.last().unwrap().h > 1 {
            let prev = mips.last().unwrap();
            let (pw, ph) = (prev.w, prev.h);
            let nw = pw.div_ceil(2).max(1);
            let nh = ph.div_ceil(2).max(1);
            let next = ImageF32::from_fn(nw, nh, 1, |x, y, _| {
                let x0 = (x * 2).min(pw - 1);
                let y0 = (y * 2).min(ph - 1);
                let x1 = (x * 2 + 1).min(pw - 1);
                let y1 = (y * 2 + 1).min(ph - 1);
                let a = conv.farther(prev.get(x0, y0, 0), prev.get(x1, y0, 0));
                let b = conv.farther(prev.get(x0, y1, 0), prev.get(x1, y1, 0));
                conv.farther(a, b)
            });
            mips.push(next);
        }
        Self { conv, mips }
    }

    /// rect（uv 闭区间）× 被测物最近深度的保守遮挡测试。
    ///
    /// 选级：rect 像素跨度 ≤ 2 纹素的最低 mip，采样覆盖 rect 的 ≤2×2 纹素窗，
    /// 取其中最远深度作保守遮挡深度；被测物最近点更远 ⇒ Occluded。
    pub fn test_rect(&self, uv_min: [f32; 2], uv_max: [f32; 2], nearest_depth: f32) -> Occlusion {
        let base = &self.mips[0];
        let (w0, h0) = (base.w as f32, base.h as f32);
        let x0 = (uv_min[0].clamp(0.0, 1.0) * w0).floor().clamp(0.0, w0 - 1.0) as u32;
        let y0 = (uv_min[1].clamp(0.0, 1.0) * h0).floor().clamp(0.0, h0 - 1.0) as u32;
        let x1 = (uv_max[0].clamp(0.0, 1.0) * w0).ceil().clamp(1.0, w0) as u32 - 1;
        let y1 = (uv_max[1].clamp(0.0, 1.0) * h0).ceil().clamp(1.0, h0) as u32 - 1;
        let span = (x1 - x0 + 1).max(y1 - y0 + 1);
        // 最低满足「跨度 ≤ 2 纹素」的 mip：2^mip ≥ span/2 ⇔ mip = ceil(log2(span)) − 1（span>2 时）。
        let mut mip = 0u32;
        while (span >> mip) > 2 {
            mip += 1;
        }
        let mip = (mip as usize).min(self.mips.len() - 1);
        let img = &self.mips[mip];
        let mx0 = x0 >> mip as u32;
        let my0 = y0 >> mip as u32;
        let mx1 = (x1 >> mip as u32).min(img.w - 1);
        let my1 = (y1 >> mip as u32).min(img.h - 1);
        let mut farthest = img.get(mx0, my0, 0);
        for my in my0..=my1 {
            for mx in mx0..=mx1 {
                farthest = self.conv.farther(farthest, img.get(mx, my, 0));
            }
        }
        if self.conv.is_farther(nearest_depth, farthest) {
            Occlusion::Occluded
        } else {
            Occlusion::Visible
        }
    }
}

/// 逐像素精确遮挡真值（测试金标准；rect 内每像素场景深度都严格比被测物近
/// ⇔ 完全遮挡）。
pub fn exact_rect_occluded(
    depth: &ImageF32,
    conv: DepthConvention,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    nearest_depth: f32,
) -> bool {
    let (w0, h0) = (depth.w as f32, depth.h as f32);
    let x0 = (uv_min[0].clamp(0.0, 1.0) * w0).floor().clamp(0.0, w0 - 1.0) as u32;
    let y0 = (uv_min[1].clamp(0.0, 1.0) * h0).floor().clamp(0.0, h0 - 1.0) as u32;
    let x1 = (uv_max[0].clamp(0.0, 1.0) * w0).ceil().clamp(1.0, w0) as u32 - 1;
    let y1 = (uv_max[1].clamp(0.0, 1.0) * h0).ceil().clamp(1.0, h0) as u32 - 1;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if !conv.is_farther(nearest_depth, depth.get(x, y, 0)) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 确定性合成深度场：近墙(左半屏) + 远背景 + 中带渐变（reverse-Z 域）。
    fn scene_depth_reverse_z(w: u32, h: u32) -> ImageF32 {
        ImageF32::from_fn(w, h, 1, |x, y, _| {
            let fx = (x as f32 + 0.5) / w as f32;
            let fy = (y as f32 + 0.5) / h as f32;
            if fx < 0.5 {
                0.9 // 近墙
            } else {
                0.1 + 0.05 * ((fx * 7.0 + fy * 3.0).sin() * 0.5 + 0.5) // 远背景带扰动
            }
        })
    }

    fn det_rects(n: u32) -> Vec<([f32; 2], [f32; 2], f32)> {
        // 确定性伪随机 rect + 深度（位混合;无 rand 依赖）
        let mut out = Vec::new();
        for i in 0..n {
            let mut v = i.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
            let mut next = || {
                v ^= v >> 15;
                v = v.wrapping_mul(0x7FEB_352D);
                v ^= v >> 13;
                (v % 1000) as f32 / 1000.0
            };
            let cx = next();
            let cy = next();
            let hw = 0.02 + 0.2 * next();
            let hh = 0.02 + 0.2 * next();
            let d = next();
            out.push((
                [(cx - hw).clamp(0.0, 1.0), (cy - hh).clamp(0.0, 1.0)],
                [(cx + hw).clamp(0.0, 1.0), (cy + hh).clamp(0.0, 1.0)],
                d,
            ));
        }
        out
    }

    #[test]
    fn zero_false_positive_vs_exact_truth_reverse_z() {
        let depth = scene_depth_reverse_z(97, 61); // 非 2 幂尺寸
        let hzb = HzbPyramid::build(&depth, DepthConvention::ReverseZ);
        let mut occluded_n = 0u32;
        for (mn, mx, d) in det_rects(400) {
            if hzb.test_rect(mn, mx, d) == Occlusion::Occluded {
                occluded_n += 1;
                assert!(
                    exact_rect_occluded(&depth, DepthConvention::ReverseZ, mn, mx, d),
                    "HZB 假阳性：判遮挡但精确真值可见（rect={mn:?}..{mx:?} d={d}）"
                );
            }
        }
        assert!(occluded_n > 0, "剔除率为零：HZB 无效（400 rect 无一判遮挡）");
    }

    #[test]
    fn zero_false_positive_vs_exact_truth_standard_z() {
        let rz = scene_depth_reverse_z(64, 64);
        let depth = ImageF32::from_fn(64, 64, 1, |x, y, _| 1.0 - rz.get(x, y, 0));
        let hzb = HzbPyramid::build(&depth, DepthConvention::StandardZ);
        let mut occluded_n = 0u32;
        for (mn, mx, d) in det_rects(400) {
            let d = 1.0 - d;
            if hzb.test_rect(mn, mx, d) == Occlusion::Occluded {
                occluded_n += 1;
                assert!(
                    exact_rect_occluded(&depth, DepthConvention::StandardZ, mn, mx, d),
                    "standard-Z 假阳性（rect={mn:?}..{mx:?} d={d}）"
                );
            }
        }
        assert!(occluded_n > 0, "standard-Z 剔除率为零");
    }

    #[test]
    fn behind_near_wall_is_occluded_front_is_visible() {
        let depth = scene_depth_reverse_z(128, 128);
        let hzb = HzbPyramid::build(&depth, DepthConvention::ReverseZ);
        // 左半屏近墙(0.9)后方物体(0.5 更远) → 必遮挡
        assert_eq!(
            hzb.test_rect([0.05, 0.2], [0.4, 0.8], 0.5),
            Occlusion::Occluded,
            "近墙后物体必须被剔"
        );
        // 同 rect 但物体在墙前(0.95 更近) → 可见
        assert_eq!(
            hzb.test_rect([0.05, 0.2], [0.4, 0.8], 0.95),
            Occlusion::Visible,
            "墙前物体不得被剔"
        );
        // 跨越远背景区(右半屏)的更近物体 → 可见
        assert_eq!(
            hzb.test_rect([0.6, 0.1], [0.9, 0.9], 0.5),
            Occlusion::Visible,
            "远背景前的物体不得被剔"
        );
    }

    #[test]
    fn pyramid_shape_and_farthest_reduction() {
        let depth = scene_depth_reverse_z(33, 17);
        let hzb = HzbPyramid::build(&depth, DepthConvention::ReverseZ);
        assert_eq!(hzb.mips.last().unwrap().w, 1);
        assert_eq!(hzb.mips.last().unwrap().h, 1);
        // 顶层 1×1 = 全图最远（reverse-Z min）
        let global_min = depth.min_per_channel()[0];
        assert!((hzb.mips.last().unwrap().get(0, 0, 0) - global_min).abs() < 1e-7);
    }

    #[test]
    fn double_run_bitexact() {
        let depth = scene_depth_reverse_z(80, 45);
        let a = HzbPyramid::build(&depth, DepthConvention::ReverseZ);
        let b = HzbPyramid::build(&depth, DepthConvention::ReverseZ);
        assert_eq!(a.mips.len(), b.mips.len());
        for (ma, mb) in a.mips.iter().zip(b.mips.iter()) {
            assert_eq!(ma, mb, "金字塔双跑必须位级一致");
        }
    }
}
