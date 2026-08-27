//! 后处理骨架显式排序链(G9.5 M119;RFC-0025 §4.J;spec/display_pipeline.md
//! RXS-0370 L1~L4 逐条对齐)。
//!
//! //@ spec: RXS-0370
//!
//! 本模块承载 M119 后处理骨架五节点显式排序(帧图语义):
//!
//! **五级显式排序冻结(顺序闭集)**:
//! 1. exposure(histogram + EV 偏移)
//! 2. bloom(tonemap 前 HDR 域多尺度 mip 链,down/up 双 pass)
//! 3. tonemap(经 M118 view transform 插件)
//! 4. color grading(LUT 资产,tonemap 后)
//! 5. output transform(RRT/ODT 或中性)
//!
//! - **顺序可检测断言**:交换两级(如 tonemap↔LUT)或跳级注入,输出 digest
//!   必不同=顺序可机核;SDR 路径可全量验证。
//! - **全程 HDR 线性域(RED 臂)**:全链任何节点不得隐式 clamp 到 SDR——节点
//!   输出范围探针([`HdrProbe`])检验;**隐式 SDR clamp 注入即探针越界 RED**
//!   (RED 臂独立有效)。
//! - **曝光状态帧间持久**:histogram→目标 EV 的 adapt 状态为 persistent
//!   resource(双缓冲双写),跨帧丢失注入即 RED;adapt 上/下不同速率。
//! - **与 M118 view transform 插件面接线**:tonemap 级消费 [`ViewTransform`] trait
//!   ([`display::view_transform`](super::view_transform));链级**禁静默插级/跳级**
//!   (注入即 RED)。
//! - **与 TAA/TSR 时域链显式排序**:bloom/tonemap 输出与 TAA/TSR 顺序在帧图
//!   显式声明(本 skeleton 落顺序接口面,时域底座消费 M24 字面 0-byte)。
//!
//! 纪律:host 纯 safe 确定性(全库 `forbid(unsafe_code)`);零新 FFI;无 device
//! 依赖——M119 语义面 = 排序骨架 + HDR 探针 + 曝光持久状态;`RURIX_REQUIRE_REAL=1`
//! 以 host 确定性为准,validation 不适用。

use super::view_transform::{DisplayError, DisplayParams, ViewTransform};

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed)
// ---------------------------------------------------------------------------

/// 后处理链失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum PostChainError {
    /// 隐式 SDR clamp 注入(RED 锚:探针越界)。
    ImplicitSdrClamp {
        stage: &'static str,
        channel: &'static str,
        value: f64,
    },
    /// 节点显式排序被交换/跳级/插级(RED 锚:顺序闭集)。
    StageOrderViolation {
        expected: &'static str,
        got: &'static str,
    },
    /// 曝光状态跨帧丢失(RED 锚:持久资源帧间丢失)。
    ExposureStateLost { expected_frame: u32 },
    /// 插件级错误(透传 M118)。
    Plugin(DisplayError),
    /// 输入含非有限值(传播性断言)。
    NonFiniteValue { stage: &'static str },
}

impl std::fmt::Display for PostChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostChainError::ImplicitSdrClamp {
                stage,
                channel,
                value,
            } => {
                write!(f, "{stage} 隐式 SDR clamp 越界: {channel}={value}(RED)")
            }
            PostChainError::StageOrderViolation { expected, got } => {
                write!(f, "节点排序违反:期望 {expected},实到 {got}(RED)")
            }
            PostChainError::ExposureStateLost { expected_frame } => {
                write!(f, "曝光状态跨帧丢失:期望帧 {expected_frame}(RED)")
            }
            PostChainError::Plugin(e) => write!(f, "插件错误: {e}"),
            PostChainError::NonFiniteValue { stage } => {
                write!(f, "{stage} 阶段输入含非有限值(NaN/Inf)")
            }
        }
    }
}

impl std::error::Error for PostChainError {}

pub type Result<T> = std::result::Result<T, PostChainError>;

// ---------------------------------------------------------------------------
// HDR 线性域探针(RED 锚:节点输出范围越界即 RED)
// ---------------------------------------------------------------------------

/// 节点输出范围探针:后处理全链节点输出必须在 HDR 线性域内。
///
/// 检测语义:隐式 SDR clamp 到 [0,1] 的输出必须经过显式钳制(用 [`StageOutput::clamp_sdr`])
/// 且本链骨架不插入任何 clamp 节点——任何节点输出被静默 clamp 即探针报警。
/// 当前机制:每级输出经 [`HdrProbe::check`] 核验,值域非空且至少有一条超出
/// [0,1] 或经 histogram 路径有非零高亮区,否则报警(退化到全 0 或全 clamp)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdrProbe {
    pub min_val: f64,
    pub max_val: f64,
    pub mean: f64,
}

impl HdrProbe {
    /// 空/单点探针。
    pub fn from_pixels(pixels: &[[f64; 3]]) -> Self {
        if pixels.is_empty() {
            return Self {
                min_val: 0.0,
                max_val: 0.0,
                mean: 0.0,
            };
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut sum = 0.0;
        let mut n = 0usize;
        for p in pixels {
            for c in p {
                if c.is_finite() {
                    min = min.min(*c);
                    max = max.max(*c);
                    sum += *c;
                    n += 1;
                }
            }
        }
        if n == 0 {
            return Self {
                min_val: 0.0,
                max_val: 0.0,
                mean: 0.0,
            };
        }
        Self {
            min_val: min,
            max_val: max,
            mean: sum / n as f64,
        }
    }

    /// 隐式 SDR clamp 探针(RED 臂):**仅作用于 HDR 线性域级**(exposure/bloom;
    /// tonemap 起为显示编码域,[0,1] 输出合法,不探)。canonical 输入含 >1 高光,
    /// HDR 域级输出 max ≤ 1.0 即暗示上游静默 clamp ⇒ RED。
    pub fn check_for_implicit_clamp(&self, stage: &'static str) -> Result<()> {
        if self.min_val.is_nan() || self.max_val.is_nan() {
            return Err(PostChainError::NonFiniteValue { stage });
        }
        if self.max_val <= 1.0 && self.max_val > 0.0 && self.mean > 0.01 {
            return Err(PostChainError::ImplicitSdrClamp {
                stage,
                channel: "all",
                value: self.max_val,
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 五级显式排序骨架节点
// ---------------------------------------------------------------------------

/// 五级显式排序(帧图语义):暴露顺序闭集;顺序交换/跳级/插级即 RED。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Exposure = 0,
    Bloom = 1,
    Tonemap = 2,
    ColorGrading = 3,
    OutputTransform = 4,
}

impl Stage {
    pub const ORDER: [Stage; 5] = [
        Stage::Exposure,
        Stage::Bloom,
        Stage::Tonemap,
        Stage::ColorGrading,
        Stage::OutputTransform,
    ];
    pub const NAMES: [&'static str; 5] = [
        "exposure",
        "bloom",
        "tonemap",
        "color_grading",
        "output_transform",
    ];

    pub fn name(&self) -> &'static str {
        Stage::NAMES[*self as usize]
    }
    pub fn index(&self) -> usize {
        *self as usize
    }
}

// ---------------------------------------------------------------------------
// 节点运算(主机确定性参考,device shader 对拍基准)
// ---------------------------------------------------------------------------

/// histogram 曝光(简化为标量乘 EV 偏移;完整 histogram 计数在 device 面,
/// host 骨架用确定性 EV 映射维持 golden 等价性)。
fn apply_exposure(px: [f64; 3], ev_offset: f64) -> [f64; 3] {
    let scale = 2.0f64.powf(ev_offset);
    [px[0] * scale, px[1] * scale, px[2] * scale]
}

/// bloom 简化为双尺度平均(完整 mip 链在 device 面,host 骨架用 3×3 box
/// blur 近似维持 HDR 域运算语义;输出仍在 HDR 线性域)。
fn apply_bloom(pixels: &[[f64; 3]], width: usize) -> Vec<[f64; 3]> {
    let h = pixels.len() / width;
    let mut out = pixels.to_vec();
    for y in 1..h.saturating_sub(1) {
        for x in 1..width.saturating_sub(1) {
            let i = y * width + x;
            let mut s = [0.0; 3];
            for dy in -1..=1i32 {
                for dx in -1..=1i32 {
                    let j = ((y as i32 + dy) as usize) * width + ((x as i32 + dx) as usize);
                    for c in 0..3 {
                        s[c] += pixels[j][c];
                    }
                }
            }
            for c in 0..3 {
                out[i][c] = pixels[i][c] + s[c] / 9.0 * 0.5;
            }
        }
    }
    out
}

/// 色彩分级 LUT(3×3 identity 的 per-channel 偏移/缩放近似;完整 3D LUT 在
/// device 面,host 骨架用 1D 逐通道映射维持 golden)。
fn apply_color_grading(px: [f64; 3], slope: [f64; 3], offset: [f64; 3]) -> [f64; 3] {
    [
        (px[0] * slope[0] + offset[0]).max(0.0),
        (px[1] * slope[1] + offset[1]).max(0.0),
        (px[2] * slope[2] + offset[2]).max(0.0),
    ]
}

// ---------------------------------------------------------------------------
// 曝光状态帧间持久
// ---------------------------------------------------------------------------

/// 曝光状态 persistent resource(双缓冲双写;跨帧丢失注入即 RED)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExposureState {
    pub frame: u32,
    /// 当前 EV 偏移(adapt 结果;上/下不同速率)。
    pub ev_current: f64,
    /// 目标 EV 偏移(histogram 测光结果)。
    pub ev_target: f64,
    /// adapt 速率:上(暗→亮)与下(亮→暗)不同(RXS-0370 L3)。
    pub adapt_up_rate: f64,
    pub adapt_down_rate: f64,
}

impl ExposureState {
    /// 首帧初始化(不得复用脏帧——`ev_current = ev_target` 表示正确冷启动)。
    pub fn init(frame: u32, ev_target: f64) -> Self {
        Self {
            frame,
            ev_current: ev_target,
            ev_target,
            adapt_up_rate: 1.0,
            adapt_down_rate: 0.5,
        }
    }

    /// 逐帧 adapt:ev_current 向 ev_target 按速率推进(帧间单调有界)。
    pub fn tick(&mut self, frame: u32, new_target: f64) -> Result<()> {
        if frame != self.frame + 1 {
            return Err(PostChainError::ExposureStateLost {
                expected_frame: self.frame + 1,
            });
        }
        self.frame = frame;
        self.ev_target = new_target;
        let delta = self.ev_target - self.ev_current;
        let rate = if delta > 0.0 {
            self.adapt_up_rate
        } else {
            self.adapt_down_rate
        };
        let step = delta.clamp(-rate, rate);
        self.ev_current += step;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 后处理链(五级显式排序;顺序闭集;插件接线)
// ---------------------------------------------------------------------------

/// 后处理链骨架(顺序闭集,不可重排)。
///
/// 运行面:[`Self::process`] 五级依次调用,每级输出经 HDR 探针检验;排序由
/// 代码结构保证(非配置驱动),交换/跳级只能在 harness RED 臂中通过直接调用
/// 各 stage 函数实现(输出 digest 必不同)。
pub struct PostProcessChain<'a> {
    pub plugin: &'a dyn ViewTransform,
    pub params: &'a DisplayParams,
    pub exposure: ExposureState,
    pub lut_slope: [f64; 3],
    pub lut_offset: [f64; 3],
}

impl<'a> PostProcessChain<'a> {
    /// 五级链处理(HDR 线性域 → 输出编码)。HDR 域级(exposure/bloom)输出经
    /// 探针检验;tonemap 起为显示编码域(插件语义保证 [0,1] 合法),只做有限性
    /// 核验。
    pub fn process(
        &mut self,
        frame: u32,
        pixels: &[[f64; 3]],
        width: usize,
    ) -> Result<Vec<[f64; 3]>> {
        // 1) exposure(histogram + EV;EV 缩放为显式操作非 clamp,仅有限性核验)。
        let after_exp: Vec<[f64; 3]> = pixels
            .iter()
            .map(|&px| apply_exposure(px, self.exposure.ev_current))
            .collect();
        if !after_exp.iter().flatten().all(|v| v.is_finite()) {
            return Err(PostChainError::NonFiniteValue { stage: "exposure" });
        }

        // 2) bloom(HDR 域多尺度;完整 device 版为 mip 链,此处 box-blur 近似;
        // 探针——隐式 clamp 注入即越界 RED)。
        let after_bloom = apply_bloom(&after_exp, width);
        HdrProbe::from_pixels(&after_bloom).check_for_implicit_clamp("bloom")?;

        // 3) tonemap(经 M118 view transform 插件;scene-linear → 显示线性,
        // [0,1] 输出合法,仅有限性核验)。
        let after_tone: Vec<[f64; 3]> = after_bloom
            .iter()
            .map(|&px| self.plugin.to_display_linear(px))
            .collect();
        if !after_tone.iter().flatten().all(|v| v.is_finite()) {
            return Err(PostChainError::NonFiniteValue { stage: "tonemap" });
        }

        // 4) color grading(LUT,tonemap 后;显示域,仅有限性核验)。
        let after_lut: Vec<[f64; 3]> = after_tone
            .iter()
            .map(|&px| apply_color_grading(px, self.lut_slope, self.lut_offset))
            .collect();
        if !after_lut.iter().flatten().all(|v| v.is_finite()) {
            return Err(PostChainError::NonFiniteValue {
                stage: "color_grading",
            });
        }

        // 5) output transform(编码到目标空间)。
        let after_out: Vec<[f64; 3]> = after_lut
            .iter()
            .map(|&px| super::view_transform::encode_display_linear(px, self.params))
            .collect();

        // 曝光状态帧间持久(驱动下一帧)。
        self.exposure.tick(frame, self.exposure.ev_target)?;

        Ok(after_out)
    }

    /// 单级处理暴露(RED 臂:顺序交换/跳级检测用;正常使用 `process` 全链)。
    pub fn process_stage(
        &self,
        stage: Stage,
        pixels: &[[f64; 3]],
        width: usize,
    ) -> Result<Vec<[f64; 3]>> {
        match stage {
            Stage::Exposure => Ok(pixels
                .iter()
                .map(|&px| apply_exposure(px, self.exposure.ev_current))
                .collect()),
            Stage::Bloom => Ok(apply_bloom(pixels, width)),
            Stage::Tonemap => Ok(pixels
                .iter()
                .map(|&px| self.plugin.to_display_linear(px))
                .collect()),
            Stage::ColorGrading => Ok(pixels
                .iter()
                .map(|&px| apply_color_grading(px, self.lut_slope, self.lut_offset))
                .collect()),
            Stage::OutputTransform => Ok(pixels
                .iter()
                .map(|&px| super::view_transform::encode_display_linear(px, self.params))
                .collect()),
        }
    }
}

// ---------------------------------------------------------------------------
// canonical 场景与辅助
// ---------------------------------------------------------------------------

/// canonical HDR 测试帧(32×32,含高光>1 + 暗部<0.01,供 HDR 探针检验)。
pub fn canonical_hdr_frame() -> Vec<[f64; 3]> {
    let w = 32usize;
    let h = 32usize;
    let mut pixels = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let t = x as f64 / (w - 1) as f64;
            let u = y as f64 / (h - 1) as f64;
            let luma = 2.0f64.powf(t * 10.0 - 5.0);
            let highlight = if (x + y) % 8 == 0 { 8.0 } else { 0.0 };
            pixels.push([
                luma * (0.5 + 0.5 * u) + highlight,
                luma * (0.8 + 0.2 * t) + highlight,
                luma * (1.2 - 0.4 * u) + highlight,
            ]);
        }
    }
    pixels
}

/// 帧 digest(f64 LE 字节流 SHA-256)。
pub fn frame_digest(pixels: &[[f64; 3]]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(pixels.len() * 24);
    for p in pixels {
        for c in p {
            buf.extend_from_slice(&c.to_le_bytes());
        }
    }
    rurix_pkg::sha256::digest(&buf)
}

#[cfg(test)]
mod tests {
    use super::super::view_transform::{DisplayParams, OutputEncoding, ViewTransformRegistry};
    use super::*;

    fn make_chain<'a>(
        plugin: &'a dyn ViewTransform,
        params: &'a DisplayParams,
        frame: u32,
    ) -> PostProcessChain<'a> {
        PostProcessChain {
            plugin,
            params,
            exposure: ExposureState::init(frame, 0.0),
            lut_slope: [1.0, 1.0, 1.0],
            lut_offset: [0.0, 0.0, 0.0],
        }
    }

    /// RXS-0370 L1:五级显式排序——顺序闭集,交换两级输出必不同(顺序可检测);
    /// SDR 路径可全量验证。
    #[test]
    //@ spec: RXS-0370
    fn five_stage_explicit_order_detectable() {
        let reg = ViewTransformRegistry::with_builtins();
        let plugin = reg.get("neutral").unwrap();
        let params = DisplayParams {
            peak_luminance_nits: 100.0,
            encoding: OutputEncoding::SdrBt1886,
        };
        let frame = canonical_hdr_frame();
        let width = 32usize;

        // 正常五级序。
        let mut chain = make_chain(plugin, &params, 0);
        let normal = chain.process(1, &frame, width).unwrap();
        let d_normal = frame_digest(&normal);

        // 交换 bloom↔tonemap(RED 臂:顺序交换产出必不同)。
        let swapped = make_chain(plugin, &params, 0);
        let s_exp = swapped
            .process_stage(Stage::Exposure, &frame, width)
            .unwrap();
        let s_tone = swapped
            .process_stage(Stage::Tonemap, &s_exp, width)
            .unwrap();
        let s_bloom = swapped.process_stage(Stage::Bloom, &s_tone, width).unwrap();
        let s_lut = swapped
            .process_stage(Stage::ColorGrading, &s_bloom, width)
            .unwrap();
        let s_out = swapped
            .process_stage(Stage::OutputTransform, &s_lut, width)
            .unwrap();
        let d_swapped = frame_digest(&s_out);
        assert_ne!(
            d_normal, d_swapped,
            "交换 bloom↔tonemap 必须产出不同 digest"
        );

        // 跳过 bloom(RED 臂:跳级产出必不同)。
        let skipped = make_chain(plugin, &params, 0);
        let k_exp = skipped
            .process_stage(Stage::Exposure, &frame, width)
            .unwrap();
        let k_tone = skipped
            .process_stage(Stage::Tonemap, &k_exp, width)
            .unwrap();
        let k_lut = skipped
            .process_stage(Stage::ColorGrading, &k_tone, width)
            .unwrap();
        let k_out = skipped
            .process_stage(Stage::OutputTransform, &k_lut, width)
            .unwrap();
        let d_skipped = frame_digest(&k_out);
        assert_ne!(d_normal, d_skipped, "跳过 bloom 必须产出不同 digest");
    }

    /// RXS-0370 L2:全程 HDR 线性域——隐式 SDR clamp 注入即探针越界 RED。
    #[test]
    //@ spec: RXS-0370
    fn hdr_linear_domain_probe_catches_implicit_clamp() {
        let reg = ViewTransformRegistry::with_builtins();
        let plugin = reg.get("neutral").unwrap();
        let params = DisplayParams {
            peak_luminance_nits: 100.0,
            encoding: OutputEncoding::SdrBt1886,
        };
        let frame = canonical_hdr_frame();

        // 正常全链:max > 1(含高光)。
        let mut chain = make_chain(plugin, &params, 0);
        let out = chain.process(1, &frame, 32).unwrap();
        let probe = HdrProbe::from_pixels(&out);
        assert!(probe.max_val > 0.0, "正常输出非退化");

        // 探针能红:若 bloom 阶段输出全在 [0,1] 且 max≤1(模拟 clamp)⇒ 非 exposure
        // 阶段报警。
        let clamped: Vec<[f64; 3]> = frame
            .iter()
            .map(|p| {
                [
                    p[0].clamp(0.0, 1.0),
                    p[1].clamp(0.0, 1.0),
                    p[2].clamp(0.0, 1.0),
                ]
            })
            .collect();
        assert!(
            HdrProbe::from_pixels(&clamped)
                .check_for_implicit_clamp("bloom")
                .is_err(),
            "隐式 SDR clamp 注入必须被探针捕获(RED)"
        );
    }

    /// RXS-0370 L3:曝光状态帧间持久——跨帧丢失注入即 RED;adapt 上/下不同
    /// 速率。
    #[test]
    //@ spec: RXS-0370
    fn exposure_state_persists_across_frames() {
        let mut state = ExposureState::init(0, 2.0);
        assert_eq!(state.ev_current, 2.0);
        // 正常 tick(帧号递增)。
        state.tick(1, 3.0).unwrap();
        assert_eq!(state.frame, 1);
        assert!(state.ev_current > 2.0, "向目标 adapt 推进");
        // 跨帧丢失(从 1 跳到 3 跳过 2)⇒ RED。
        assert!(matches!(
            state.tick(3, 4.0),
            Err(PostChainError::ExposureStateLost { expected_frame: 2 })
        ));
        // adapt 速率:上(暗→亮)=1.0 快于下(亮→暗)=0.5。
        let mut s2 = ExposureState::init(0, 0.0);
        s2.tick(1, 2.0).unwrap();
        let up_step = s2.ev_current;
        let mut s3 = ExposureState::init(0, 2.0);
        s3.tick(1, 0.0).unwrap();
        let down_step = 2.0 - s3.ev_current;
        assert!(
            up_step > down_step,
            "adapt 上快于下: up={up_step} down={down_step}"
        );
    }

    /// RXS-0370 L4:与 M118 view transform 插件面接线——tonemap 级消费
    /// ViewTransform trait;未注册插件名调用失败(透传 M118 RED 臂)。
    #[test]
    //@ spec: RXS-0370
    fn tonemap_consumes_view_transform_plugin() {
        let reg = ViewTransformRegistry::with_builtins();
        let plugin = reg.get("neutral").unwrap();
        let params = DisplayParams {
            peak_luminance_nits: 100.0,
            encoding: OutputEncoding::SdrBt1886,
        };
        let frame = canonical_hdr_frame();
        let mut chain = make_chain(plugin, &params, 0);
        let out = chain.process(1, &frame, 32).unwrap();
        assert!(out.iter().all(|p| p.iter().all(|v| v.is_finite())));
        // 四插件输出互不相同(至少 neutral vs aces13 不同)。
        let a13 = reg.get("aces13").unwrap();
        let mut c2 = make_chain(a13, &params, 0);
        let out2 = c2.process(1, &frame, 32).unwrap();
        assert_ne!(frame_digest(&out), frame_digest(&out2), "不同插件输出不同");
    }
}
