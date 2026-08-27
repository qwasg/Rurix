//! ViewTransform 插件面（G9.5 M118；RFC-0025 §4.I；spec/display_pipeline.md
//! RXS-0369 L2）。
//!
//! - [`ViewTransform`] trait：输入 HDR 线性（scene-linear Rec.709）+ 显示参数
//!   [`DisplayParams`] → 输出编码（SDR BT.1886 / scRGB 线性 / PQ-Rec.2020 三种
//!   [`OutputEncoding`]）。插件本体只负责 scene-linear → **显示线性**（Rec.709
//!   基色度、1.0 = 100 nits SDR 白点参考电平），路径编码由共享
//!   [`encode_display_linear`] 承担（输出变换阶段职责单一事实源）。
//! - 四内置实现并列（D4 D13：锁死单一 tonemapper 是 2026 架构错误）：
//!   [`aces13`](crate::display::aces13) / [`aces20`](crate::display::aces20) /
//!   [`agx`](crate::display::agx) / [`neutral`](crate::display::neutral)，第三方
//!   可经 [`ViewTransformRegistry::register`] 注册；**未注册插件名调用 → 拒录
//!   RED**（typed `Err`，[`DisplayError::UnregisteredPlugin`]）。
//! - golden 输入集 [`golden_input_set`]：闭式确定性生成（灰阶×6 色相×淡色×超
//!   100 nits 高光），逐插件对冻结 golden digest（host 参考公式逐字实现 +
//!   measured 冻结 + provenance，禁手写）。

use super::color;

/// 输出编码闭集（三交换链路径一一对应；[`super::swapchain::SwapchainPath`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputEncoding {
    /// SDR（Rec.709，BT.1886 gamma 2.4，全范围）。
    SdrBt1886,
    /// scRGB（Rec.709 基色度线性，1.0 = 80 nits；100-nits SDR 白 → 1.25）。
    ScRgbLinear,
    /// PQ-Rec.2020（ST 2084 绝对亮度编码，Rec.2020 基色度）。
    PqSt2084Rec2020,
}

impl OutputEncoding {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputEncoding::SdrBt1886 => "sdr_bt1886",
            OutputEncoding::ScRgbLinear => "scrgb_linear",
            OutputEncoding::PqSt2084Rec2020 => "pq_st2084_rec2020",
        }
    }
}

/// 显示参数（view transform 输入面之一；HDR 元数据在输出变换阶段由
/// [`super::swapchain::DisplayPipeline`] 汇总填写）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayParams {
    /// 目标峰值亮度（nits；SDR 参考 = 100.0）。
    pub peak_luminance_nits: f64,
    /// 输出编码。
    pub encoding: OutputEncoding,
}

/// SDR 参考白电平（nits）：显示线性 1.0 ≡ 100 nits。
pub const SDR_REFERENCE_WHITE_NITS: f64 = 100.0;
/// scRGB 标度：1.0 ≡ 80 nits（D3D scRGB 约定）。
pub const SCRGB_WHITE_NITS: f64 = 80.0;
/// scRGB 输出钳制上界（nits 等效 10000/80 = 125.0 scRGB 单位；冻结常量）。
pub const SCRGB_MAX_UNITS: f64 = 125.0;

/// 显示管线失败类别（typed Err，fail-closed；严禁 UB）。
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayError {
    /// 未注册插件名调用（RXS-0369 L2 RED 锚）。
    UnregisteredPlugin(String),
    /// 重复注册同名插件。
    DuplicatePlugin(String),
    /// 非 HDR 交换链携带 PQ 输出（RXS-0369 L3 RED 锚）。
    PqOutputOnNonHdrSwapchain { path: &'static str },
    /// 交换链路径与输出编码不匹配（合法性闭集外组合）。
    EncodingPathMismatch {
        path: &'static str,
        encoding: &'static str,
    },
    /// 输入/输出含非有限值（NaN/Inf）。
    NonFiniteValue { stage: &'static str },
    /// HDR 设备标定层条件未触发却被强制消费（不充绿、fail-closed）。
    HdrCalibrationNotTriggered { reason: &'static str },
}

impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayError::UnregisteredPlugin(n) => write!(f, "未注册 view transform 插件: {n}"),
            DisplayError::DuplicatePlugin(n) => write!(f, "重复注册插件: {n}"),
            DisplayError::PqOutputOnNonHdrSwapchain { path } => {
                write!(f, "非 HDR 交换链({path})携带 PQ 输出(RED)")
            }
            DisplayError::EncodingPathMismatch { path, encoding } => {
                write!(
                    f,
                    "交换链路径 {path} 与输出编码 {encoding} 不匹配(合法性闭集外)"
                )
            }
            DisplayError::NonFiniteValue { stage } => write!(f, "{stage} 阶段出现非有限值"),
            DisplayError::HdrCalibrationNotTriggered { reason } => {
                write!(f, "HDR 设备标定层未触发({reason})不可消费")
            }
        }
    }
}

impl std::error::Error for DisplayError {}

/// ViewTransform 插件 trait（输入 HDR 线性 + 显示参数 → 输出编码）。
pub trait ViewTransform {
    /// 注册名（注册表键，如 `aces13`）。
    fn id(&self) -> &'static str;
    /// 展示名（含版本/参考公式来源，进 golden provenance）。
    fn display_name(&self) -> &'static str;
    /// scene-linear Rec.709 HDR → 显示线性 Rec.709（1.0 = 100 nits 参考电平）。
    /// 实现 = 参考公式 host 逐字移植（golden 事实源）。
    fn to_display_linear(&self, hdr_linear: [f64; 3]) -> [f64; 3];
    /// 完整变换（HDR 线性 → 输出编码）：默认 = `to_display_linear` +
    /// 共享路径编码；插件不重写编码段（输出变换阶段职责单一）。
    fn transform(&self, hdr_linear: [f64; 3], params: &DisplayParams) -> [f64; 3] {
        encode_display_linear(self.to_display_linear(hdr_linear), params)
    }
}

/// 共享路径编码（输出变换阶段；三编码闭集）。
///
/// - SDR：显示线性 [0,1] 钳制 → BT.1886 gamma 2.4 逆 EOTF（CTL `bt1886_r`，
///   Lw=1/Lb=0）；
/// - scRGB：×(100/80) 线性（SDR 白 → 1.25 scRGB 单位），钳 [0, 125.0]；
/// - PQ-Rec.2020：显示线性 ×100 nits → Rec.709→Rec.2020 基色度转换（同 D65
///   白，仅基色度旋转）→ ST 2084 编码。
pub fn encode_display_linear(rgb: [f64; 3], params: &DisplayParams) -> [f64; 3] {
    match params.encoding {
        OutputEncoding::SdrBt1886 => {
            let mut out = [0.0f64; 3];
            for i in 0..3 {
                let v = rgb[i].clamp(0.0, 1.0);
                out[i] = color::bt1886_inv(v, 2.4, 1.0, 0.0);
            }
            out
        }
        OutputEncoding::ScRgbLinear => {
            let scale = SDR_REFERENCE_WHITE_NITS / SCRGB_WHITE_NITS;
            let mut out = [0.0f64; 3];
            for i in 0..3 {
                out[i] = (rgb[i] * scale).clamp(0.0, SCRGB_MAX_UNITS);
            }
            out
        }
        OutputEncoding::PqSt2084Rec2020 => {
            let rec709_to_2020 = color::rgb_to_rgb(&color::REC709, &color::REC2020);
            let nits = color::svmul(params.peak_luminance_nits, rgb);
            let wide = color::vmul(nits, &rec709_to_2020);
            let mut out = [0.0f64; 3];
            for i in 0..3 {
                out[i] = color::y_to_st2084(wide[i].max(0.0));
            }
            out
        }
    }
}

/// 插件注册表（trait 对象；四内置实现并列 + 第三方可注册）。
pub struct ViewTransformRegistry {
    plugins: std::collections::BTreeMap<&'static str, Box<dyn ViewTransform>>,
}

impl ViewTransformRegistry {
    /// 空注册表。
    pub fn empty() -> Self {
        Self {
            plugins: std::collections::BTreeMap::new(),
        }
    }

    /// 含四内置插件的注册表（ACES 1.3 / ACES 2.0 / AgX / 中性并列）。
    pub fn with_builtins() -> Self {
        let mut r = Self::empty();
        r.register(Box::new(super::aces13::Aces13::new()))
            .expect("builtin aces13");
        r.register(Box::new(super::aces20::Aces20::new()))
            .expect("builtin aces20");
        r.register(Box::new(super::agx::AgX::canonical()))
            .expect("builtin agx");
        r.register(Box::new(super::neutral::Neutral))
            .expect("builtin neutral");
        r
    }

    /// 注册插件（同名重复 → typed Err，fail-closed）。
    pub fn register(&mut self, plugin: Box<dyn ViewTransform>) -> Result<(), DisplayError> {
        let id = plugin.id();
        if self.plugins.contains_key(id) {
            return Err(DisplayError::DuplicatePlugin(id.to_string()));
        }
        self.plugins.insert(id, plugin);
        Ok(())
    }

    /// 按名取用（未注册 → typed Err，RXS-0369 L2「未注册插件名调用 → 拒录 RED」）。
    pub fn get(&self, name: &str) -> Result<&dyn ViewTransform, DisplayError> {
        self.plugins
            .get(name)
            .map(|b| b.as_ref())
            .ok_or_else(|| DisplayError::UnregisteredPlugin(name.to_string()))
    }

    /// 已注册插件名（确定序，BTreeMap 迭代序）。
    pub fn registered_names(&self) -> Vec<&'static str> {
        self.plugins.keys().copied().collect()
    }
}

/// golden 输入集（闭式确定性生成；灰阶 × 六色相 × 淡色 × 高光簇，共 155 条）。
///
/// 布局（冻结，改动即 golden 分叉）：
/// - 17 条中性灰阶：2^k，k ∈ -8..=8；
/// - 6 色相（R/Y/G/C/B/M 纯 Rec.709 基色度及两两混合）× 13 强度：2^k，k ∈ -6..=6；
/// - 6 色相淡色（与 0.5 中性灰等量混合）× 8 强度：2^k，k ∈ -5..=2；
/// - 6 色相超 SDR 高光（×8/×64 两档）。
pub fn golden_input_set() -> Vec<[f64; 3]> {
    let mut set = Vec::with_capacity(155);
    for k in -8..=8i32 {
        let v = 2.0f64.powi(k);
        set.push([v, v, v]);
    }
    const HUES: [[f64; 3]; 6] = [
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
    ];
    for h in HUES {
        for k in -6..=6i32 {
            let v = 2.0f64.powi(k);
            set.push([h[0] * v, h[1] * v, h[2] * v]);
        }
    }
    for h in HUES {
        for k in -5..=2i32 {
            let v = 2.0f64.powi(k);
            set.push([
                h[0] * v * 0.5 + 0.5 * v,
                h[1] * v * 0.5 + 0.5 * v,
                h[2] * v * 0.5 + 0.5 * v,
            ]);
        }
    }
    for h in HUES {
        for m in [8.0f64, 64.0] {
            set.push([h[0] * m, h[1] * m, h[2] * m]);
        }
    }
    set
}

/// canonical 测试帧（64×64 HDR 线性，闭式生成：水平灰阶扫描 × 垂直色相带 ×
/// 对角高光斜带；供三交换链路径切换确定性证据）。
pub fn canonical_hdr_frame() -> Vec<[f64; 3]> {
    const W: usize = 64;
    const H: usize = 64;
    let mut frame = Vec::with_capacity(W * H);
    for y in 0..H {
        for x in 0..W {
            let t = x as f64 / (W - 1) as f64;
            let band = (y / 8) as f64;
            let hue = band / 7.0 * 2.0 * std::f64::consts::PI;
            let luma = 2.0f64.powf(t * 12.0 - 9.0);
            let chroma = 0.5 + 0.5 * (y as f64 / (H - 1) as f64);
            let r = luma * (1.0 + chroma * hue.cos().max(0.0));
            let g = luma * (1.0 + chroma * (hue - 2.0 * std::f64::consts::PI / 3.0).cos().max(0.0));
            let b = luma * (1.0 + chroma * (hue + 2.0 * std::f64::consts::PI / 3.0).cos().max(0.0));
            let diag = if (x + y) % 16 == 0 { 32.0 } else { 0.0 };
            frame.push([r + diag, g + diag, b + diag]);
        }
    }
    frame
}

/// 帧/输出集 digest（f64 LE 字节流 SHA-256，经 rurix-pkg 同源面）。
pub fn rgb_set_digest(rgb: &[[f64; 3]]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(rgb.len() * 24);
    for p in rgb {
        for c in p {
            buf.extend_from_slice(&c.to_le_bytes());
        }
    }
    rurix_pkg::sha256::digest(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0369
    #[test]
    fn registry_four_builtins_and_unregistered_rejected() {
        let r = ViewTransformRegistry::with_builtins();
        assert_eq!(
            r.registered_names(),
            vec!["aces13", "aces20", "agx", "neutral"]
        );
        // 未注册插件名调用 → 拒录(typed Err,RED 锚)。
        match r.get("filmic-x") {
            Err(DisplayError::UnregisteredPlugin(n)) => assert_eq!(n, "filmic-x"),
            Ok(_) => panic!("未注册名未拒录"),
            Err(e) => panic!("意外错误类别: {e}"),
        }
        // 重复注册 → typed Err。
        let mut r2 = ViewTransformRegistry::empty();
        r2.register(Box::new(super::super::neutral::Neutral))
            .unwrap();
        assert!(matches!(
            r2.register(Box::new(super::super::neutral::Neutral)),
            Err(DisplayError::DuplicatePlugin(_))
        ));
    }

    //@ spec: RXS-0369
    #[test]
    fn golden_input_set_deterministic_and_nontrivial() {
        let a = golden_input_set();
        let b = golden_input_set();
        assert_eq!(a.len(), 155);
        assert_eq!(a, b);
        assert!(a.iter().any(|p| p[0] > 1.0));
        assert!(a.iter().any(|p| p[0] == 0.0 && p[1] > 0.0));
    }

    //@ spec: RXS-0369
    #[test]
    fn encodings_closed_set_behaviors() {
        let lin = [0.18, 0.5, 1.5];
        let sdr = encode_display_linear(
            lin,
            &DisplayParams {
                peak_luminance_nits: 100.0,
                encoding: OutputEncoding::SdrBt1886,
            },
        );
        // SDR:钳 [0,1] 后 gamma 2.4 逆 EOTF;1.5 → 钳 1.0 → 1.0。
        assert!(sdr.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!((sdr[2] - 1.0).abs() < 1e-12);
        assert!((sdr[0] - 0.18f64.powf(1.0 / 2.4)).abs() < 1e-12);
        let scrgb = encode_display_linear(
            lin,
            &DisplayParams {
                peak_luminance_nits: 100.0,
                encoding: OutputEncoding::ScRgbLinear,
            },
        );
        // scRGB:×1.25 线性,超白保留余量。
        assert!((scrgb[2] - 1.875).abs() < 1e-12);
        let pq = encode_display_linear(
            lin,
            &DisplayParams {
                peak_luminance_nits: 100.0,
                encoding: OutputEncoding::PqSt2084Rec2020,
            },
        );
        // PQ:[0,1] 码值;18% 灰 100nits 参考下 18 nits → 已知 PQ 码值邻域。
        assert!(pq.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!(pq[0] > 0.3 && pq[0] < 0.6);
    }
}
