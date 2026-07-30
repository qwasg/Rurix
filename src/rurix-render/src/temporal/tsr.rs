//! TSR 类时域超分主实现(报告7 P1 自研蓝本,§2.2 Epic 公开旋钮体系;
//! RFC-0016 §4.H2:与 TAA 同一 kernel、仅分辨率映射不同)。
//!
//! 与 TAA([`crate::temporal::taa`])同 kernel 思想、仅分辨率映射不同:
//! a. 历史缓冲在**输出分辨率**常驻(报告7 §5 资源布局:历史独立于渲染分辨率);
//! b. 当前帧低分辨率样本按 jitter 偏移散射到输出网格——**重采样核选定
//!    Catmull-Rom(a = -0.5 Keys 三次)**:依据——TSR 历史采样即 bicubic
//!    (报告7 §2.2),Catmull-Rom 保边锐度优于双线性、振铃弱于 Lanczos-2,
//!    4×4 tap 固定成本且三次精确(线性/二次/三次函数无损重建,单测钉死);
//!    样本位置 = 输入纹素中心 + jitter(jitter 对齐是时域超分与朴素上采样的
//!    分水岭);抗振铃 = 结果钳入 4×4 采集邻域 min/max;
//! c. **MV 上采样选定最近邻**:深度不连续处双线性会插出不存在的中值速度、
//!    跨边渗漏成鬼影,最近邻保持速度场硬边界(深度感知上采样归 P3 质量攻坚,
//!    报告7 §4);深度上采样同理最近邻;reactive 上采样取双线性(先验软权重,
//!    边缘过渡平滑,后验验证仍是硬 0/1);
//! d. 历史重投影经 [`reproject_sample`]、历史验证经 [`validate_history_with_mv`]
//!    (公共底座,G-G5-7 代码审计点,**禁私写**):冻结接口无法线输入,
//!    法线通道填均匀常量——验证实质由深度相对差 + 出屏检测承担(disocclusion
//!    主判据),文档留痕;YCoCg 邻域裁剪([`neighborhood_aabb`])作用于
//!    当前帧上采样图的 3×3 输出分辨率邻域;
//! e. **闪烁时域分析**(报告7 §2.2 机制 2,公开旋钮):逐像素亮度(YCoCg Y)
//!    帧间差分,符号翻转(带死区)计数的指数滑动统计;**按时长判定与目标
//!    帧率解耦**的语义(照 `r.TSR.ShadingRejection.Flickering.FrameRateCap/
//!    .Period`,报告7 §6 风险三条)——本 host 参考实现落地为**帧计数窗口**
//!    (EMA 速率 k = 2/(N+1),默认 N = 16 帧 ≈ 60Hz 下 0.27s),生产 device
//!    实现按目标帧率等比缩放窗口帧数,文档写明不再另造阈值;高闪烁区
//!    收紧混合权重(偏向历史)**并松弛邻域裁剪**(闪烁 = 着色不稳定,
//!    钳制会把振荡钉进输出,松弛让稳定历史存活——报告7 §2.2「拒绝着色
//!    变化」口径;代价是该区鬼影风险,闪烁区视觉可接受);
//! f. **reactive mask 双通道**(报告7 §2.3):输入 reactive(自动通道语义,
//!    透明/粒子 R8 输出;手工通道由调用方合并入槽)与深度/出屏后验验证
//!    取并集——reactive 高处历史权重压至 0(alpha → 1 取当前帧,宁可锯齿
//!    回归不拖影,同 [`crate::temporal::taa`] validity=0 原则);与闪烁收紧
//!    冲突时 **reactive 优先**(收紧按 (1 - reactive) 缩放);
//! g. reset/首帧/输出分辨率变化:直接上采样当前帧([`TsrUpscaler::resample_current_frame`]);
//! h. **不做锐化**(RFC-0016 §4.H2 明文;锐化归 tonemapper 后可选 pass,
//!    报告7 §2.2 机制 4,Fortnite 统一 `r.Tonemapper.Sharpen=0.5` 先例)。
//!
//! 历史状态内置双缓冲(接口契约见 [`crate::temporal::upscale`] 模块文档):
//! 第 N 帧输出即第 N+1 帧历史;历史颜色/深度/亮度/闪烁统计全部输出分辨率。
//! 收敛加速(Resurrection)/拒绝抗锯齿质量档分档(报告7 §2.2 机制 3)归后续
//! 波次,本期旋钮面以 [`TsrParams`] 为限。

use crate::temporal::common::{
    neighborhood_aabb, reproject_sample, rgb_image_to_ycocg, rgb_to_ycocg,
    validate_history_with_mv, ycocg_to_rgb,
};
use crate::temporal::image::ImageF32;
use crate::temporal::upscale::{UpscaleBackend, UpscaleInputs};

/// TSR 旋钮(报告7 §2.2 公开旋钮体系的 host 参考实现子集;默认值即验收口径)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TsrParams {
    /// 当前帧基础混合权重(历史 = 1 - alpha;与 TAA kernel 同语义,默认 0.1,
    /// 有效累积窗约 10 帧)。
    pub base_alpha: f32,
    /// alpha 下限(任何调节不得越过;防历史锁死导致永不收敛,默认 0.04)。
    pub min_alpha: f32,
    /// 闪烁判定 EMA 窗长(帧计数窗口,默认 16 帧 ≈ 60Hz 下 0.27s;按时长
    /// 判定的语义见模块文档 e,device 实现按目标帧率缩放)。
    pub flicker_window_frames: u32,
    /// 闪烁收紧强度 ∈ \[0,1\](高闪烁区 alpha × (1 - tighten·score·(1-reactive)),
    /// 默认 0.5)。
    pub flicker_tighten: f32,
    /// 翻转死区·绝对亮度(帧间亮度差低于此不记翻转,抗数值噪声,默认 0.02)。
    pub flicker_deadzone_abs: f32,
    /// 翻转死区·相对亮度(× max(|cur|, |prev|),抗亮部量化抖动,默认 0.1)。
    pub flicker_deadzone_rel: f32,
    /// 历史验证深度相对容差(公共底座判据,默认 0.1;法线判据因冻结接口
    /// 无法线输入而恒过,见模块文档 d)。
    pub depth_rel_tol: f32,
}

impl Default for TsrParams {
    fn default() -> Self {
        Self {
            base_alpha: 0.1,
            min_alpha: 0.04,
            flicker_window_frames: 16,
            flicker_tighten: 0.5,
            flicker_deadzone_abs: 0.02,
            flicker_deadzone_rel: 0.1,
            depth_rel_tol: 0.1,
        }
    }
}

/// Catmull-Rom 三次核(a = -0.5 Keys;支撑 |t| < 2,四 tap 权和恒 = 1)。
fn catmull_rom(t: f32) -> f32 {
    let t = t.abs();
    if t <= 1.0 {
        1.5 * t * t * t - 2.5 * t * t + 1.0
    } else if t < 2.0 {
        -0.5 * t * t * t + 2.5 * t * t - 4.0 * t + 2.0
    } else {
        0.0
    }
}

/// 逐通道最近邻上采样(MV/深度用;保持硬边界,见模块文档 c)。
fn upsample_nearest(src: &ImageF32, ow: u32, oh: u32) -> ImageF32 {
    let mut out = ImageF32::new(ow, oh, src.c);
    let (fw, fh) = (ow as f32, oh as f32);
    for y in 0..oh {
        for x in 0..ow {
            let u = (x as f32 + 0.5) / fw;
            let v = (y as f32 + 0.5) / fh;
            for ch in 0..src.c {
                out.set(x, y, ch, src.sample_nearest(u, v, ch));
            }
        }
    }
    out
}

/// 逐通道双线性上采样(reactive 先验软权重用,见模块文档 c)。
fn upsample_bilinear(src: &ImageF32, ow: u32, oh: u32) -> ImageF32 {
    let mut out = ImageF32::new(ow, oh, src.c);
    let (fw, fh) = (ow as f32, oh as f32);
    for y in 0..oh {
        for x in 0..ow {
            let u = (x as f32 + 0.5) / fw;
            let v = (y as f32 + 0.5) / fh;
            for ch in 0..src.c {
                out.set(x, y, ch, src.sample_bilinear(u, v, ch));
            }
        }
    }
    out
}

/// 整图亮度(YCoCg Y 通道;闪烁统计与邻域裁剪同空间)。
fn luma_image(img: &ImageF32) -> ImageF32 {
    let mut out = ImageF32::new(img.w, img.h, 1);
    for y in 0..img.h {
        for x in 0..img.w {
            out.set(x, y, 0, rgb_to_ycocg(img.pixel3(x, y))[0]);
        }
    }
    out
}

/// 自研 TSR 类时域超分器(报告7 P1 主实现;任何平台保底,vendor SDK
/// 不可用平台的兜底层,报告7 §6 风险二)。
///
/// 历史状态(颜色/深度/亮度/翻转符号/闪烁分数,全部输出分辨率)内置于本
/// 结构体,双缓冲语义见 [`crate::temporal::upscale`] 模块文档。
#[derive(Debug, Clone)]
pub struct TsrUpscaler {
    params: TsrParams,
    output_size: Option<(u32, u32)>,
    history: Option<ImageF32>,
    history_depth: Option<ImageF32>,
    prev_luma: Option<ImageF32>,
    prev_sign: Option<ImageF32>,
    flicker_score: Option<ImageF32>,
}

impl TsrUpscaler {
    pub fn new(params: TsrParams) -> Self {
        Self {
            params,
            output_size: None,
            history: None,
            history_depth: None,
            prev_luma: None,
            prev_sign: None,
            flicker_score: None,
        }
    }

    /// jitter 对齐 Catmull-Rom 重采样:当前帧(输入分辨率)→ `output_size`
    /// 显示域图像(× exposure;抗振铃钳入 4×4 采集邻域 min/max)。
    ///
    /// 即 reset/首帧路径的输出,也是每帧混合的当前帧贡献(模块文档 b/g);
    /// 独立公开以便 vendor 后端/单测复用同一核(G-G5-7 禁私写重采样)。
    pub fn resample_current_frame(inputs: &UpscaleInputs) -> ImageF32 {
        let (iw, ih, ow, oh) = inputs.validated();
        let (sx, sy) = (iw as f32 / ow as f32, ih as f32 / oh as f32);
        // 核参缩放(模块文档 b):以输出像素为核宽单位 × 0.75 系数——
        // Catmull-Rom 在 t = 1 有零点,恰好 2× 整数比下 frac = 0.5 相位两
        // 最近 tap 同时归零(权和坍缩);0.75 把核宽放到 2.67 输出像素,
        // 任相位权和 ≥ 0.45,同时保持窄核的时域锐度(收敛门禁实测裁决);
        // 1:1 保持单位核(CR 插值性 = NativeAA 零抖动逐像素透传)。
        let kernel_scale = |r: f32| if r > 1.0 { r * 0.75 } else { 1.0 };
        let (rx, ry) = (
            kernel_scale(ow as f32 / iw as f32),
            kernel_scale(oh as f32 / ih as f32),
        );
        let (jx, jy) = (inputs.jitter[0], inputs.jitter[1]);
        let mut out = ImageF32::new(ow, oh, 3);
        for oy in 0..oh {
            for ox in 0..ow {
                // 输出像素中心 → 输入纹素空间连续坐标(纹素中心 = 整数);
                // 样本实际位置 = 纹素中心 + jitter → 绕 (px - j) 的 4×4 窗;
                // 核参 × rx/ry(见上 kernel_scale:窄核散射保输入锐度,jitter
                // 相位平均在输出网格上交织出全密度采样;以输入像素为单位的
                // 宽核会在时域固定点上留下系统性模糊,收敛门禁实测裁决)
                let px = (ox as f32 + 0.5) * sx - 0.5;
                let py = (oy as f32 + 0.5) * sy - 0.5;
                let gx = px - jx;
                let gy = py - jy;
                let bx = gx.floor() as i32;
                let by = gy.floor() as i32;
                let mut acc = [0.0f32; 3];
                let mut wsum = 0.0f32;
                let mut mn = [f32::INFINITY; 3];
                let mut mx = [f32::NEG_INFINITY; 3];
                for dy in -1i32..=2 {
                    for dx in -1i32..=2 {
                        let tx = (bx + dx).clamp(0, iw as i32 - 1) as u32;
                        let ty = (by + dy).clamp(0, ih as i32 - 1) as u32;
                        let w = catmull_rom((gx - (bx + dx) as f32) * rx)
                            * catmull_rom((gy - (by + dy) as f32) * ry);
                        let p = inputs.color.pixel3(tx, ty);
                        for ch in 0..3 {
                            acc[ch] += w * p[ch];
                            mn[ch] = mn[ch].min(p[ch]);
                            mx[ch] = mx[ch].max(p[ch]);
                        }
                        wsum += w;
                    }
                }
                let mut px_out = [0.0f32; 3];
                for ch in 0..3 {
                    // 抗振铃:钳入采集邻域色域后转显示域
                    let v = (acc[ch] / wsum).clamp(mn[ch], mx[ch]);
                    px_out[ch] = (v * inputs.exposure).max(0.0);
                }
                out.set_pixel3(ox, oy, px_out);
            }
        }
        out
    }

    fn clear_state(&mut self) {
        self.output_size = None;
        self.history = None;
        self.history_depth = None;
        self.prev_luma = None;
        self.prev_sign = None;
        self.flicker_score = None;
    }
}

impl Default for TsrUpscaler {
    fn default() -> Self {
        Self::new(TsrParams::default())
    }
}

impl UpscaleBackend for TsrUpscaler {
    fn name(&self) -> &str {
        "tsr"
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        let (_, _, ow, oh) = inputs.validated();
        // 输出分辨率变化 → 自动丢弃历史(接口契约,模块文档)
        if self.output_size != Some((ow, oh)) {
            self.clear_state();
            self.output_size = Some((ow, oh));
        }
        let cur = Self::resample_current_frame(inputs);
        let depth_hi = upsample_nearest(inputs.depth, ow, oh);
        let mv_hi = upsample_nearest(inputs.mv, ow, oh);
        let reactive_hi = inputs.reactive.map(|r| upsample_bilinear(r, ow, oh));
        let cur_luma = luma_image(&cur);

        let out = if inputs.reset || self.history.is_none() {
            // reset/首帧:直接上采样当前帧;闪烁统计从干净态起步
            self.prev_sign = Some(ImageF32::new(ow, oh, 1));
            self.flicker_score = Some(ImageF32::new(ow, oh, 1));
            cur.clone()
        } else {
            let history = self.history.take().expect("历史存在");
            let history_depth = self.history_depth.take().expect("历史深度存在");
            let prev_luma = self.prev_luma.take().expect("历史亮度存在");
            let mut score = self.flicker_score.take().expect("闪烁分数存在");
            let mut sign = self.prev_sign.take().expect("翻转符号存在");

            // 闪烁时域分析(模块文档 e):帧间亮度差分 → 死区符号 → 翻转 EMA
            let ema_k = 2.0 / (self.params.flicker_window_frames as f32 + 1.0);
            for y in 0..oh {
                for x in 0..ow {
                    let lc = cur_luma.get(x, y, 0);
                    let lp = prev_luma.get(x, y, 0);
                    let d = lc - lp;
                    let dead = self
                        .params
                        .flicker_deadzone_abs
                        .max(self.params.flicker_deadzone_rel * lc.abs().max(lp.abs()));
                    let s = if d > dead {
                        1.0
                    } else if d < -dead {
                        -1.0
                    } else {
                        0.0
                    };
                    let ps = sign.get(x, y, 0);
                    let flip = if s != 0.0 && ps != 0.0 && s != ps {
                        1.0
                    } else {
                        0.0
                    };
                    score.set(x, y, 0, score.get(x, y, 0) * (1.0 - ema_k) + flip * ema_k);
                    if s != 0.0 {
                        sign.set(x, y, 0, s);
                    }
                }
            }

            // 重投影 + 历史验证(全走公共底座,模块文档 d;法线填均匀常量,
            // 验证实质 = 深度相对差 + 出屏检测)
            let (hist_reproj, _inside) = reproject_sample(&history, &mv_hi);
            let normals = ImageF32::from_fn(ow, oh, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
            let validity = validate_history_with_mv(
                &depth_hi,
                &history_depth,
                &normals,
                &normals,
                &mv_hi,
                self.params.depth_rel_tol,
                0.9,
            );
            let cur_ycc = rgb_image_to_ycocg(&cur);
            let (lo, hi) = neighborhood_aabb(&cur_ycc);

            let mut out = ImageF32::new(ow, oh, 3);
            for y in 0..oh {
                for x in 0..ow {
                    if validity.get(x, y, 0) < 0.5 {
                        // 历史不可信(disocclusion/深度突变):取当前帧上采样,
                        // 宁可锯齿回归不拖影
                        out.set_pixel3(x, y, cur.pixel3(x, y));
                        continue;
                    }
                    let reactive = reactive_hi.as_ref().map_or(0.0, |r| r.get(x, y, 0));
                    let flick = score.get(x, y, 0);
                    let hist_ycc = rgb_to_ycocg(hist_reproj.pixel3(x, y));
                    // 闪烁松弛:高闪烁(且非 reactive)区让稳定历史存活(模块文档 e)
                    let relax = (flick * (1.0 - reactive)).clamp(0.0, 1.0);
                    let hist_used = [
                        hist_ycc[0].clamp(lo.get(x, y, 0), hi.get(x, y, 0)) * (1.0 - relax)
                            + hist_ycc[0] * relax,
                        hist_ycc[1].clamp(lo.get(x, y, 1), hi.get(x, y, 1)) * (1.0 - relax)
                            + hist_ycc[1] * relax,
                        hist_ycc[2].clamp(lo.get(x, y, 2), hi.get(x, y, 2)) * (1.0 - relax)
                            + hist_ycc[2] * relax,
                    ];
                    // reactive 高 → alpha → 1(压历史);闪烁高 → alpha 收紧(偏历史);
                    // 冲突时 reactive 优先(收紧按 1 - reactive 缩放)
                    let alpha = (self.params.base_alpha
                        * (1.0 - self.params.flicker_tighten * flick * (1.0 - reactive)))
                        .max(reactive)
                        .clamp(self.params.min_alpha, 1.0);
                    let cc = cur_ycc.pixel3(x, y);
                    let blended = [
                        alpha * cc[0] + (1.0 - alpha) * hist_used[0],
                        alpha * cc[1] + (1.0 - alpha) * hist_used[1],
                        alpha * cc[2] + (1.0 - alpha) * hist_used[2],
                    ];
                    let rgb = ycocg_to_rgb(blended);
                    out.set_pixel3(x, y, [rgb[0].max(0.0), rgb[1].max(0.0), rgb[2].max(0.0)]);
                }
            }
            self.flicker_score = Some(score);
            self.prev_sign = Some(sign);
            out
        };

        // 双缓冲:本帧输出即下帧历史
        self.history = Some(out.clone());
        self.history_depth = Some(depth_hi);
        self.prev_luma = Some(cur_luma);
        out
    }

    fn reset_history(&mut self) {
        self.clear_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::common::jitter_sequence;
    use crate::temporal::ssim::{SsimGate, ssim};

    // -----------------------------------------------------------------------
    // 合成测试场景生成器(报告7 §5 验证方法一:解析式着色,无三角形管线):
    // 棋盘 8px + 斜边硬界 + 1px 细线 + 低频渐变,定义在参考(输出)分辨率
    // 像素单位上;jitter/超采样/降采样都是它的采样器。
    // -----------------------------------------------------------------------

    fn shade(fx: f32, fy: f32) -> [f32; 3] {
        let check = (((fx + 3.7) / 8.0).floor() as i32 + ((fy + 3.7) / 8.0).floor() as i32) & 1;
        let mut base = 0.2 + 0.55 * check as f32;
        if fx + fy > 84.0 {
            base = 1.0 - base; // 斜边(硬边,锯齿主战场)
        }
        let line = (fx + 0.3) % 6.0 < 1.0; // 细线(1 ref px,半分辨率输入必锯齿)
        let v = if line { base * 0.35 } else { base };
        let grad = 0.08 * (fx * 0.05).sin() * (fy * 0.07).cos();
        [
            (v + grad).clamp(0.0, 1.0),
            (0.85 * v + 0.6 * grad).clamp(0.0, 1.0),
            (0.7 * v - grad).clamp(0.0, 1.0),
        ]
    }

    /// 输入分辨率渲染:像素 (x,y) 采样场景 ((x+0.5+j)·scale)(scale = ref/in)。
    fn render_input(w: u32, h: u32, scale: f32, jitter: [f32; 2]) -> ImageF32 {
        ImageF32::from_fn(w, h, 3, |x, y, ch| {
            shade(
                (x as f32 + 0.5 + jitter[0]) * scale,
                (y as f32 + 0.5 + jitter[1]) * scale,
            )[ch as usize]
        })
    }

    /// 参考:输出分辨率 4×4 超采样(收敛对拍金标准,报告7 §5)。
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

    /// 单帧双线性上采样(对照组:无 jitter 对齐、无时域累积)。
    fn bilinear_up(src: &ImageF32, ow: u32, oh: u32) -> ImageF32 {
        ImageF32::from_fn(ow, oh, 3, |x, y, ch| {
            src.sample_bilinear(
                (x as f32 + 0.5) / ow as f32,
                (y as f32 + 0.5) / oh as f32,
                ch,
            )
        })
    }

    fn const_depth(w: u32, h: u32) -> ImageF32 {
        ImageF32::from_fn(w, h, 1, |_, _, _| 0.5)
    }

    fn inputs_for<'a>(
        color: &'a ImageF32,
        depth: &'a ImageF32,
        mv: &'a ImageF32,
        out_size: (u32, u32),
        jitter: [f32; 2],
        frame_index: u32,
        reset: bool,
    ) -> UpscaleInputs<'a> {
        UpscaleInputs {
            color,
            depth,
            mv,
            reactive: None,
            exposure: 1.0,
            jitter,
            output_size: out_size,
            frame_index,
            reset,
        }
    }

    /// 跑 N 帧静态合成场景 TSR(1/2 分辨率输入,Halton jitter),返回逐帧输出
    /// 与末帧输入。
    fn run_static_tsr(in_w: u32, out_w: u32, frames: u32) -> (Vec<ImageF32>, Vec<ImageF32>) {
        let in_h = in_w;
        let out_h = out_w;
        let scale = out_w as f32 / in_w as f32;
        let depth = const_depth(in_w, in_h);
        let mv = ImageF32::new(in_w, in_h, 2);
        let mut tsr = TsrUpscaler::default();
        let jitters = jitter_sequence(frames);
        let mut outs = Vec::new();
        let mut raws = Vec::new();
        for (i, &j) in jitters.iter().enumerate() {
            let cur = render_input(in_w, in_h, scale, j);
            let inp = inputs_for(&cur, &depth, &mv, (out_w, out_h), j, i as u32, i == 0);
            outs.push(tsr.upscale(&inp));
            raws.push(cur);
        }
        (outs, raws)
    }

    // -----------------------------------------------------------------------
    // trait / reset / 重采样核
    // -----------------------------------------------------------------------

    #[test]
    fn trait_object_dispatch_complete() {
        // TsrUpscaler 完整实现 UpscaleBackend:经 dyn 调用三方法
        let mut backend: Box<dyn UpscaleBackend> = Box::new(TsrUpscaler::default());
        assert_eq!(backend.name(), "tsr");
        let cur = render_input(16, 16, 2.0, [0.1, -0.2]);
        let depth = const_depth(16, 16);
        let mv = ImageF32::new(16, 16, 2);
        let inp = inputs_for(&cur, &depth, &mv, (32, 32), [0.1, -0.2], 0, true);
        let out = backend.upscale(&inp);
        assert_eq!((out.w, out.h, out.c), (32, 32, 3));
        backend.reset_history();
        let out2 = backend.upscale(&inp);
        assert_eq!((out2.w, out2.h), (32, 32));
    }

    #[test]
    fn reset_first_frame_is_plain_upsample() {
        // reset 后首帧 = 上采样当前帧(逐像素断言重采样核输出)
        let cur = render_input(16, 16, 2.0, [0.25, -0.4]);
        let depth = const_depth(16, 16);
        let mv = ImageF32::new(16, 16, 2);
        let mut tsr = TsrUpscaler::default();
        let inp0 = inputs_for(&cur, &depth, &mv, (32, 32), [0.25, -0.4], 0, true);
        let expected = TsrUpscaler::resample_current_frame(&inp0);
        let out0 = tsr.upscale(&inp0);
        for i in 0..out0.data.len() {
            assert!(
                (out0.data[i] - expected.data[i]).abs() < 1e-7,
                "首帧必须为纯重采样输出"
            );
        }
        // 跑若干帧后 reset_history → 再次首帧 = 纯重采样
        for i in 1..4u32 {
            let j = jitter_sequence(8)[i as usize];
            let c = render_input(16, 16, 2.0, j);
            let inp = inputs_for(&c, &depth, &mv, (32, 32), j, i, false);
            tsr.upscale(&inp);
        }
        tsr.reset_history();
        let j = [0.4, 0.1];
        let c = render_input(16, 16, 2.0, j);
        let inp = inputs_for(&c, &depth, &mv, (32, 32), j, 4, false);
        let expected2 = TsrUpscaler::resample_current_frame(&inp);
        let out = tsr.upscale(&inp);
        for i in 0..out.data.len() {
            assert!((out.data[i] - expected2.data[i]).abs() < 1e-7);
        }
    }

    #[test]
    fn kernel_constant_and_linear_ramp_exact() {
        // 常数场:任意 jitter/对齐下逐像素精确(权和归一化);
        // 线性斜坡:单帧重建不追求逐帧精确(窄核散射语义),但**相位平均**
        // 必须无偏——对称核卷积线性函数 = 中心值,这是时域合成正确性的
        // 数学内核(Halton 16 相位平均,内部区域断言)
        let depth = const_depth(8, 8);
        let mv = ImageF32::new(8, 8, 2);
        let j = [0.3, -0.45];
        let flat = ImageF32::from_fn(8, 8, 3, |_, _, ch| 0.3 + 0.1 * ch as f32);
        let inp = inputs_for(&flat, &depth, &mv, (16, 16), j, 0, true);
        let out = TsrUpscaler::resample_current_frame(&inp);
        for y in 0..16 {
            for x in 0..16 {
                for ch in 0..3 {
                    let expect = 0.3 + 0.1 * ch as f32;
                    assert!((out.get(x, y, ch) - expect).abs() < 1e-5, "常数场必须精确");
                }
            }
        }
        // 线性斜坡的相位平均无偏性
        let f = |px: f32, py: f32, ch: u32| {
            (0.02 * px + 0.03 * py + 0.1 * ch as f32 + 0.2).clamp(0.0, 1.0)
        };
        let jitters = jitter_sequence(16);
        let mut acc = ImageF32::new(16, 16, 3);
        for &jj in &jitters {
            let ramp = ImageF32::from_fn(8, 8, 3, |x, y, ch| {
                f(x as f32 + jj[0], y as f32 + jj[1], ch)
            });
            let inp = inputs_for(&ramp, &depth, &mv, (16, 16), jj, 0, true);
            let out = TsrUpscaler::resample_current_frame(&inp);
            for (a, &v) in acc.data.iter_mut().zip(out.data.iter()) {
                *a += v / 16.0;
            }
        }
        // 边界像素采集窗越界钳制,相位平均同样有偏 → 仅断内部区域;
        // 判据 = 相位平均偏差远小于单相位误差(平均向真值收敛,而非有偏点)
        let mut max_bias = 0.0f32;
        let mut mean_phase_err = 0.0f32;
        let mut count = 0u32;
        for oy in 4..=11u32 {
            for ox in 4..=11u32 {
                let px = (ox as f32 + 0.5) * 0.5 - 0.5;
                let py = (oy as f32 + 0.5) * 0.5 - 0.5;
                for ch in 0..3 {
                    let truth = f(px, py, ch);
                    max_bias = max_bias.max((acc.get(ox, oy, ch) - truth).abs());
                    for &jj in &jitters {
                        let ramp = ImageF32::from_fn(8, 8, 3, |x, y, c| {
                            f(x as f32 + jj[0], y as f32 + jj[1], c)
                        });
                        let inp = inputs_for(&ramp, &depth, &mv, (16, 16), jj, 0, true);
                        let out = TsrUpscaler::resample_current_frame(&inp);
                        mean_phase_err += (out.get(ox, oy, ch) - truth).abs();
                        count += 1;
                    }
                }
            }
        }
        mean_phase_err /= count as f32;
        eprintln!("[tsr_kernel_bias] max_bias={max_bias:.6} mean_phase_err={mean_phase_err:.6}");
        assert!(
            max_bias < 0.25 * mean_phase_err && max_bias < 0.01,
            "相位平均偏差 {max_bias:.5} 应 << 单相位误差 {mean_phase_err:.5}"
        );
    }

    #[test]
    fn kernel_passthrough_native_resolution() {
        // 1:1(NativeAA 档)+ 零 jitter:输出 = 输入 × exposure(逐像素)
        let cur = render_input(16, 16, 1.0, [0.0, 0.0]);
        let depth = const_depth(16, 16);
        let mv = ImageF32::new(16, 16, 2);
        let mut inp = inputs_for(&cur, &depth, &mv, (16, 16), [0.0, 0.0], 0, true);
        inp.exposure = 1.25;
        let out = TsrUpscaler::resample_current_frame(&inp);
        for i in 0..cur.data.len() {
            let expect = (cur.data[i] * 1.25).max(0.0);
            assert!(
                (out.data[i] - expect).abs() < 1e-6,
                "1:1 零抖动必须逐像素透传"
            );
        }
    }

    #[test]
    fn resolution_decoupling_output_nontrivial() {
        // 960→1920 类比例的小图版:32→64 输出尺寸正确、内容非平凡
        // (与朴素双线性上采样存在实质差异 = jitter 对齐核在工作)
        let j = [0.35, 0.15];
        let cur = render_input(32, 32, 2.0, j);
        let depth = const_depth(32, 32);
        let mv = ImageF32::new(32, 32, 2);
        let inp = inputs_for(&cur, &depth, &mv, (64, 64), j, 0, true);
        let out = TsrUpscaler::resample_current_frame(&inp);
        assert_eq!((out.w, out.h, out.c), (64, 64, 3));
        let (mn, mx) = (
            out.data.iter().fold(f32::INFINITY, |a, &v| a.min(v)),
            out.data.iter().fold(f32::NEG_INFINITY, |a, &v| a.max(v)),
        );
        assert!(mx - mn > 0.3, "输出必须有实质对比度");
        let naive = bilinear_up(&cur, 64, 64);
        let max_diff = out
            .data
            .iter()
            .zip(naive.data.iter())
            .map(|(&a, &b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(max_diff > 5e-3, "jitter 对齐核与朴素双线性必须有实质差异");
    }

    // -----------------------------------------------------------------------
    // 静态收敛 SSIM 门禁(G-G5-7 核心)与抖动残留
    // -----------------------------------------------------------------------

    #[test]
    fn static_convergence_ssim_gate() {
        // 静态合成场景 1/2 分辨率输入(32→64)× 32 帧 Halton jitter:
        // TSR 输出 vs 全分辨率 4×4 超采样参考 SSIM > 0.9,且显著高于
        // 单帧双线性上采样;MSE 逐帧段单调下降(首 8 帧均值 > 末 8 帧均值)。
        let reference = render_reference(64, 64);
        let (outs, raws) = run_static_tsr(32, 64, 32);
        let gate = SsimGate::new(0.9);
        let tsr_ssim = gate.score(&outs[31], &reference);
        let bilinear_ssim = gate.score(&bilinear_up(&raws[31], 64, 64), &reference);
        let mses: Vec<f64> = outs.iter().map(|o| ImageF32::mse(o, &reference)).collect();
        eprintln!(
            "[tsr_convergence] tsr_ssim={tsr_ssim:.4} bilinear_ssim={bilinear_ssim:.4} mses={mses:.6?}"
        );
        assert!(
            gate.passes(&outs[31], &reference),
            "TSR 终帧 SSIM={tsr_ssim:.4} 必须 > 0.9(参考对照双线性={bilinear_ssim:.4})"
        );
        assert!(
            tsr_ssim > bilinear_ssim + 0.03,
            "TSR {tsr_ssim:.4} 应显著高于双线性 {bilinear_ssim:.4}"
        );
        let first_avg = mses[..8].iter().sum::<f64>() / 8.0;
        let last_avg = mses[24..].iter().sum::<f64>() / 8.0;
        assert!(
            last_avg < first_avg,
            "MSE 应逐段下降:首段 {first_avg:.6} > 末段 {last_avg:.6}"
        );
    }

    #[test]
    fn jitter_residual_below_direct_upsample() {
        // 静态场景末段:TSR 帧间差 < 无 TSR 直接上采样帧间差的 5%
        let (outs, raws) = run_static_tsr(32, 64, 32);
        let tsr_diff = ImageF32::mse(&outs[30], &outs[31]);
        let raw_diff = ImageF32::mse(
            &bilinear_up(&raws[30], 64, 64),
            &bilinear_up(&raws[31], 64, 64),
        );
        assert!(
            tsr_diff < 0.05 * raw_diff,
            "tsr_diff={tsr_diff:.8} 应 < 5% × raw_diff={raw_diff:.8}"
        );
        eprintln!("[tsr_jitter_residual] tsr_diff={tsr_diff:.8} raw_diff={raw_diff:.8}");
    }

    #[test]
    fn history_double_buffer_engaged_and_stable() {
        // 双缓冲语义:第 N 帧输出参与第 N+1 帧混合——相邻帧输出既不同
        // (历史/当前在混合),又比无 TSR 直接上采样更时域稳定(混合在起效)
        let (outs, raws) = run_static_tsr(32, 64, 8);
        let max_diff = outs[0]
            .data
            .iter()
            .zip(outs[1].data.iter())
            .map(|(&a, &b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(max_diff > 1e-6, "第 0/1 帧输出应不同(历史参与混合)");
        let tsr_diff = ImageF32::mse(&outs[0], &outs[1]);
        let raw_diff = ImageF32::mse(
            &bilinear_up(&raws[0], 64, 64),
            &bilinear_up(&raws[1], 64, 64),
        );
        assert!(
            tsr_diff < raw_diff,
            "首帧对即应比朴素上采样稳定:tsr={tsr_diff:.6} raw={raw_diff:.6}"
        );
    }

    // -----------------------------------------------------------------------
    // disocclusion / 闪烁 / reactive
    // -----------------------------------------------------------------------

    #[test]
    fn disocclusion_takes_current_no_ghost() {
        // 移动遮挡物(1/2 分辨率):揭露区与新遮挡区历史被拒,取当前帧
        // 上采样——无鬼影(报告7 §2.1 三不信场景一;深度判据兜底 MV 说谎)。
        let (iw, ow) = (16u32, 32u32);
        let (bg, fg) = ([0.1f32; 3], [0.9f32; 3]);
        let render = |t: u32| {
            ImageF32::from_fn(iw, iw, 3, |x, _, ch| {
                let v = if x >= t && x < t + 4 { fg } else { bg };
                v[ch as usize]
            })
        };
        let depth_of = |t: u32| {
            ImageF32::from_fn(
                iw,
                iw,
                1,
                |x, _, _| if x >= t && x < t + 4 { 0.3 } else { 0.9 },
            )
        };
        let mv = ImageF32::new(iw, iw, 2);
        let mut tsr = TsrUpscaler::default();
        // 帧 0:方块 [2,6)
        let c0 = render(2);
        let d0 = depth_of(2);
        let inp0 = inputs_for(&c0, &d0, &mv, (ow, ow), [0.0, 0.0], 0, true);
        let out0 = tsr.upscale(&inp0);
        // 鬼影源确认:帧 0 揭露区(输入列 2 = 输出列 4,5)为亮
        assert!(out0.pixel3(5, 16)[0] > 0.8, "帧 0 该处应为亮(鬼影源存在)");
        // 帧 1:方块右移 1px → [3,7);MV = 0(最恶劣情形,深度验证兜底)
        let c1 = render(3);
        let d1 = depth_of(3);
        let inp1 = inputs_for(&c1, &d1, &mv, (ow, ow), [0.0, 0.0], 1, false);
        let out1 = tsr.upscale(&inp1);
        // 揭露区(输入列 2 → 输出列 4,5):历史亮 0.9,当前暗 0.1 → 必须取当前
        for ox in [4u32, 5] {
            assert!(
                out1.pixel3(ox, 16)[0] < 0.45,
                "({ox}) 揭露区鬼影:历史亮值必须被拒(当前={:.3})",
                out1.pixel3(ox, 16)[0]
            );
        }
        // 新遮挡区(输入列 6 → 输出列 12,13):历史暗 0.1,当前亮 0.9 → 取当前
        for ox in [12u32, 13] {
            assert!(
                out1.pixel3(ox, 16)[0] > 0.55,
                "({ox}) 新遮挡区应取当前帧亮值(当前={:.3})",
                out1.pixel3(ox, 16)[0]
            );
        }
        // 不变区域保持混合结果:远处背景仍暗
        assert!(out1.pixel3(28, 16)[0] < 0.2, "不变背景应保持暗");
    }

    #[test]
    fn flicker_suppressed_and_static_unharmed() {
        // 人工 2 帧周期翻转纹理区(左半)→ 输出振幅 < 输入振幅 30%;
        // 静态区(右半)不受误伤:闪烁分数 ≈ 0、帧间稳定、SSIM 不降。
        let (iw, ow) = (16u32, 32u32);
        let half_in = 8u32;
        let half_out = 16u32;
        let flip_amp = 0.6f32; // 输入振幅(0.2 ↔ 0.8)
        let static_px = |x: u32, y: u32, ch: u32| {
            0.4 + 0.15 * (x as f32 * 0.4).sin() * (y as f32 * 0.3).cos() + 0.05 * ch as f32
        };
        let render = |t: u32| {
            ImageF32::from_fn(iw, iw, 3, |x, y, ch| {
                if x < half_in {
                    if t.is_multiple_of(2) { 0.2 } else { 0.8 }
                } else {
                    static_px(x, y, ch)
                }
            })
        };
        let depth = const_depth(iw, iw);
        let mv = ImageF32::new(iw, iw, 2);
        let mut tsr = TsrUpscaler::default();
        let mut outs = Vec::new();
        for t in 0..16u32 {
            let cur = render(t);
            let inp = inputs_for(&cur, &depth, &mv, (ow, ow), [0.0, 0.0], t, t == 0);
            outs.push(tsr.upscale(&inp));
        }
        // 左半(翻转区):末 4 帧逐像素振幅 < 30% 输入振幅
        let mut worst = 0.0f32;
        for y in 4..28u32 {
            for x in 2..(half_out - 2) {
                let (mut mn, mut mx) = (f32::INFINITY, f32::NEG_INFINITY);
                for o in &outs[12..16] {
                    let v = o.get(x, y, 0);
                    mn = mn.min(v);
                    mx = mx.max(v);
                }
                worst = worst.max(mx - mn);
            }
        }
        assert!(
            worst < 0.3 * flip_amp,
            "翻转区输出振幅 {worst:.4} 应 < 30% × {flip_amp}"
        );
        // 右半静态区(避开重采样核 2-tap 支撑域跨边 + AABB 1 列,共 4 列):不受误伤
        let margin = half_out + 4;
        let score = tsr.flicker_score.as_ref().expect("闪烁分数存在");
        for y in 0..ow {
            for x in margin..ow {
                assert!(
                    score.get(x, y, 0) < 0.05,
                    "({x},{y}) 静态区闪烁分数应 ≈0,实际 {:.4}",
                    score.get(x, y, 0)
                );
            }
        }
        let mut static_wobble = 0.0f32;
        for y in 0..ow {
            for x in margin..ow {
                for ch in 0..3 {
                    let d = (outs[15].get(x, y, ch) - outs[14].get(x, y, ch)).abs();
                    static_wobble = static_wobble.max(d);
                }
            }
        }
        assert!(static_wobble < 1e-3, "静态区帧间应稳定:{static_wobble:.6}");
        // SSIM 不降:静态区末帧 vs 静态区首帧(纯重采样)同分位
        let crop = |img: &ImageF32| {
            ImageF32::from_fn(ow - margin, ow, 3, |x, y, ch| img.get(x + margin, y, ch))
        };
        let s_final = ssim(&crop(&outs[15]), &crop(&outs[0]));
        assert!(
            s_final > 0.99,
            "静态区末帧应与首帧几乎一致(SSIM={s_final:.4} > 0.99)"
        );
        eprintln!(
            "[tsr_flicker] out_amp={worst:.4} in_amp={flip_amp:.2} static_wobble={static_wobble:.6}"
        );
    }

    #[test]
    fn reactive_mask_suppresses_history() {
        // reactive = 1 区历史权重被压:亮度阶跃后 reactive 区当帧即达新值
        // (拖影长度 0),非 reactive 区拖影 ≥ 1 帧(报告7 §2.3 验收口径)。
        let n = 16u32;
        let depth = const_depth(n, n);
        let mv = ImageF32::new(n, n, 2);
        let reactive = ImageF32::from_fn(n, n, 1, |x, _, _| if x >= 8 { 1.0 } else { 0.0 });
        let base = |x: u32, t: u32| 0.3 + 0.005 * x as f32 + if t >= 3 { 0.4 } else { 0.0 };
        let render = |t: u32| ImageF32::from_fn(n, n, 3, |x, _, _| base(x, t));
        let mut tsr = TsrUpscaler::default();
        let mut outs = Vec::new();
        for t in 0..8u32 {
            let cur = render(t);
            let mut inp = inputs_for(&cur, &depth, &mv, (n, n), [0.0, 0.0], t, t == 0);
            inp.reactive = Some(&reactive);
            outs.push(tsr.upscale(&inp));
        }
        // 拖影长度:帧 3 起 |out - cur| ≥ 1e-3 的帧数(取内部列避开 AABB 边)
        let trail = |x: u32| {
            (3..8usize)
                .filter(|&t| (outs[t].get(x, 8, 0) - base(x, t as u32)).abs() >= 1e-3)
                .count()
        };
        let trail_left = trail(4); // reactive = 0
        let trail_right = trail(12); // reactive = 1
        assert!(
            trail_right == 0 && trail_left > trail_right,
            "拖影长度:reactive 区 {trail_right} 应 < 非 reactive 区 {trail_left}"
        );
        // reactive 区当帧精确取当前帧(历史权重 = 0)
        let err_right = (outs[3].get(12, 8, 0) - base(12, 3)).abs();
        assert!(
            err_right < 1e-5,
            "reactive 区输出必须 = 当前帧:{err_right:.6}"
        );
        eprintln!("[tsr_reactive] trail_normal={trail_left} trail_reactive={trail_right}");
    }

    #[test]
    fn flicker_requires_repeated_flips() {
        // 单次阶跃 ≠ 闪烁:闪烁分数保持低位,历史正常跟踪新值(收敛不误伤)
        let n = 16u32;
        let depth = const_depth(n, n);
        let mv = ImageF32::new(n, n, 2);
        let render = |t: u32| {
            ImageF32::from_fn(n, n, 3, |x, _, _| {
                0.3 + 0.005 * x as f32 + if t >= 3 { 0.4 } else { 0.0 }
            })
        };
        let mut tsr = TsrUpscaler::default();
        let mut outs = Vec::new();
        for t in 0..8u32 {
            let cur = render(t);
            let inp = inputs_for(&cur, &depth, &mv, (n, n), [0.0, 0.0], t, t == 0);
            outs.push(tsr.upscale(&inp));
        }
        let score = tsr.flicker_score.as_ref().expect("闪烁分数存在");
        let max_score = score.data.iter().fold(0.0f32, |a, &v| a.max(v));
        assert!(max_score < 0.3, "单次阶跃不得触发闪烁:{max_score:.3}");
        // 正常收敛不误伤:阶跃后误差按基础速率持续收缩(EMA 0.9/帧),
        // 帧 7 误差 < 帧 3 误差的 75%(非锁死、非发散)
        let target = |x: u32, t: u32| 0.3 + 0.005 * x as f32 + if t >= 3 { 0.4 } else { 0.0 };
        let err3 = (outs[3].get(4, 8, 0) - target(4, 3)).abs();
        let err7 = (outs[7].get(4, 8, 0) - target(4, 7)).abs();
        assert!(err3 > 1e-3, "阶跃当帧应有可见误差:{err3:.6}");
        assert!(
            err7 < 0.75 * err3,
            "单次阶跃后应持续收敛:err3={err3:.6} err7={err7:.6}"
        );
    }

    #[test]
    fn output_size_change_auto_resets() {
        // 输出分辨率变化 → 自动丢弃历史,首帧 = 纯重采样(接口契约)
        let depth = const_depth(16, 16);
        let mv = ImageF32::new(16, 16, 2);
        let mut tsr = TsrUpscaler::default();
        for (i, &j) in jitter_sequence(4).iter().enumerate() {
            let cur = render_input(16, 16, 2.0, j);
            let inp = inputs_for(&cur, &depth, &mv, (32, 32), j, i as u32, i == 0);
            tsr.upscale(&inp);
        }
        assert!(tsr.history.is_some(), "历史应已建立");
        // 换输出尺寸(非 2× 比例也应工作)
        let cur = render_input(16, 16, 2.0, [0.0, 0.0]);
        let inp = inputs_for(&cur, &depth, &mv, (48, 48), [0.0, 0.0], 4, false);
        let expected = TsrUpscaler::resample_current_frame(&inp);
        let out = tsr.upscale(&inp);
        assert_eq!((out.w, out.h), (48, 48));
        for i in 0..out.data.len() {
            assert!(
                (out.data[i] - expected.data[i]).abs() < 1e-7,
                "尺寸变化后首帧必须为纯重采样输出"
            );
        }
    }
}
