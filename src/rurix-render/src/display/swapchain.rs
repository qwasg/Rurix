//! SDR/scRGB/PQ 三交换链路径与 HDR 标定登记（G9.5 M118；RFC-0025 §4.I；
//! spec/display_pipeline.md RXS-0369 L1/L3/L4）。
//!
//! - [`SwapchainPath`]：SDR（Rec.709）/ scRGB / PQ-Rec.2020 三路径闭集，一等
//!   资源；运行时切换（[`DisplayPipeline::switch_to`]，重建路径状态语义——窗口
//!   腿维持 D-130 红线 C++ shim 0-byte，本面 = 路径状态机 + 编码路由）。
//! - 切换确定性：同输入帧切同路径输出位级一致（digest 机核）；切换序列
//!   证据（[`SwitchRecord`] 日志）非空。
//! - **合法性闭集**（RXS-0369 L3）：路径 ↔ 编码一一对应；**非 HDR 交换链
//!   （SDR/scRGB）携带 PQ 输出即 RED**——[`DisplayError::PqOutputOnNonHdrSwapchain`]
//!   typed Err，fail-closed。
//! - HDR 元数据（MaxCLL/MaxFALL/mastering primaries）由输出变换阶段填写
//!   （[`PresentOutput::hdr_metadata`]，每次 present 非空）。
//! - **HDR 设备标定层**（L4）：本机无 HDR 设备资产 + OS 显示查询面 unwired
//!   （D-130 shim 0-byte）→ [`HdrCalibrationStatus::NotTriggered`] 显式结构登记，
//!   evidence 字段可见，不充绿；强制消费 → typed Err（fail-closed），且
//!   **不反向否决** SDR 上可全量验证的管线/插件面。

use super::color;
use super::view_transform::{
    rgb_set_digest, DisplayError, DisplayParams, OutputEncoding, ViewTransform,
};

/// 三交换链路径闭集（一等资源,运行时切换）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapchainPath {
    /// SDR（Rec.709，BT.1886 gamma 2.4）。
    Sdr,
    /// scRGB（Rec.709 线性 FP16 语义,1.0 = 80 nits）。
    ScRgb,
    /// PQ-Rec.2020（ST 2084 绝对亮度）。
    PqRec2020,
}

impl SwapchainPath {
    pub fn as_str(&self) -> &'static str {
        match self {
            SwapchainPath::Sdr => "sdr",
            SwapchainPath::ScRgb => "scrgb",
            SwapchainPath::PqRec2020 => "pq_rec2020",
        }
    }

    /// 该路径唯一合法编码（合法性闭集）。
    pub fn legal_encoding(&self) -> OutputEncoding {
        match self {
            SwapchainPath::Sdr => OutputEncoding::SdrBt1886,
            SwapchainPath::ScRgb => OutputEncoding::ScRgbLinear,
            SwapchainPath::PqRec2020 => OutputEncoding::PqSt2084Rec2020,
        }
    }

    /// 是否 HDR 路径（PQ-Rec.2020 唯一 HDR）。
    pub fn is_hdr(&self) -> bool {
        matches!(self, SwapchainPath::PqRec2020)
    }
}

/// 路径 ↔ 编码合法性核验（RXS-0369 L3 RED 锚）：
/// - 编码 = PQ 且路径非 HDR → `PqOutputOnNonHdrSwapchain`（判据逐字面）；
/// - 其余错配 → `EncodingPathMismatch`。
pub fn validate_path_encoding(
    path: SwapchainPath,
    encoding: OutputEncoding,
) -> Result<(), DisplayError> {
    if encoding == OutputEncoding::PqSt2084Rec2020 && !path.is_hdr() {
        return Err(DisplayError::PqOutputOnNonHdrSwapchain {
            path: path.as_str(),
        });
    }
    if path.legal_encoding() != encoding {
        return Err(DisplayError::EncodingPathMismatch {
            path: path.as_str(),
            encoding: encoding.as_str(),
        });
    }
    Ok(())
}

/// HDR 元数据（MaxCLL/MaxFALL/mastering primaries + 白点;输出变换阶段填写）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdrMetadata {
    pub max_cll_nits: f64,
    pub max_fall_nits: f64,
    pub mastering_primaries: [[f64; 2]; 3],
    pub white_point: [f64; 2],
}

/// 路径切换记录(证据面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchRecord {
    pub seq: u32,
    pub from: SwapchainPath,
    pub to: SwapchainPath,
}

/// present 产物(编码帧 + digest + HDR 元数据)。
#[derive(Debug, Clone)]
pub struct PresentOutput {
    pub path: SwapchainPath,
    pub encoding: OutputEncoding,
    pub encoded: Vec<[f64; 3]>,
    pub digest: [u8; 32],
    pub hdr_metadata: HdrMetadata,
}

/// HDR 设备标定层状态(RXS-0369 L4 登记面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdrCalibrationStatus {
    /// 条件未触发(缺 HDR 设备资产 / OS 显示查询面 unwired)——登记不充绿。
    NotTriggered { reason: &'static str },
}

/// HDR 能力查询面(本机无 HDR 设备资产 ⇒ 能力缺位 + 标定 NotTriggered)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdrCapabilityReport {
    pub display_hdr_capable: bool,
    pub query_surface: &'static str,
    pub calibration: HdrCalibrationStatus,
}

/// 设备 HDR 能力查询(零新 FFI 纪律:OS 显示查询面 unwired——D-130 窗口 shim
/// 0-byte,本波不接线;本机无 HDR 设备资产 ⇒ capable=false + 标定 NotTriggered
/// 显式登记)。
pub fn query_hdr_capability() -> HdrCapabilityReport {
    HdrCapabilityReport {
        display_hdr_capable: false,
        query_surface: "os_display_hdr_query unwired(D-130 C++ shim 0-byte;本机无 HDR 设备资产)",
        calibration: HdrCalibrationStatus::NotTriggered {
            reason: "缺 HDR 设备资产(条件未触发;登记 SKIP=not-triggered 不充绿,不反向否决 SDR 验证面)",
        },
    }
}

/// 强制消费 HDR 标定结果 → fail-closed typed Err(未触发不可冒充已标定)。
pub fn require_hdr_calibration(report: &HdrCapabilityReport) -> Result<(), DisplayError> {
    match report.calibration {
        HdrCalibrationStatus::NotTriggered { reason } => {
            Err(DisplayError::HdrCalibrationNotTriggered { reason })
        }
    }
}

/// 显示管线(交换链路径状态机 + 输出变换阶段)。
pub struct DisplayPipeline {
    path: SwapchainPath,
    peak_luminance_nits: f64,
    switch_log: Vec<SwitchRecord>,
}

impl DisplayPipeline {
    /// 装配(初始路径;合法性闭集内)。
    pub fn assemble(path: SwapchainPath, peak_luminance_nits: f64) -> Result<Self, DisplayError> {
        validate_path_encoding(path, path.legal_encoding())?;
        Ok(Self {
            path,
            peak_luminance_nits,
            switch_log: Vec::new(),
        })
    }

    /// 运行时切换(重建交换链语义:路径状态替换 + 证据日志;同输入切同路径
    /// 输出位级一致由 [`Self::present`] 的确定性保证)。
    pub fn switch_to(&mut self, to: SwapchainPath) -> Result<(), DisplayError> {
        validate_path_encoding(to, to.legal_encoding())?;
        let from = self.path;
        let seq = self.switch_log.len() as u32;
        self.switch_log.push(SwitchRecord { seq, from, to });
        self.path = to;
        Ok(())
    }

    pub fn path(&self) -> SwapchainPath {
        self.path
    }

    pub fn switch_log(&self) -> &[SwitchRecord] {
        &self.switch_log
    }

    /// 当前路径显示参数。
    pub fn display_params(&self) -> DisplayParams {
        DisplayParams {
            peak_luminance_nits: self.peak_luminance_nits,
            encoding: self.path.legal_encoding(),
        }
    }

    /// present(当前路径,合法编码自动路由)。
    pub fn present(
        &self,
        hdr_frame: &[[f64; 3]],
        plugin: &dyn ViewTransform,
    ) -> Result<PresentOutput, DisplayError> {
        self.present_explicit(hdr_frame, plugin, self.path, self.path.legal_encoding())
    }

    /// present(显式路径 + 编码;合法性闭集核验先行——PQ 上非 HDR 交换链即
    /// typed Err,RED 锚)。
    pub fn present_explicit(
        &self,
        hdr_frame: &[[f64; 3]],
        plugin: &dyn ViewTransform,
        path: SwapchainPath,
        encoding: OutputEncoding,
    ) -> Result<PresentOutput, DisplayError> {
        validate_path_encoding(path, encoding)?;
        if !hdr_frame.iter().flatten().all(|v| v.is_finite()) {
            return Err(DisplayError::NonFiniteValue { stage: "input" });
        }
        let params = DisplayParams {
            peak_luminance_nits: self.peak_luminance_nits,
            encoding,
        };
        let mut encoded = Vec::with_capacity(hdr_frame.len());
        for &px in hdr_frame {
            encoded.push(plugin.transform(px, &params));
        }
        if !encoded.iter().flatten().all(|v| v.is_finite()) {
            return Err(DisplayError::NonFiniteValue { stage: "output" });
        }
        let digest = rgb_set_digest(&encoded);
        let hdr_metadata = self.fill_hdr_metadata(path, &encoded);
        Ok(PresentOutput {
            path,
            encoding,
            encoded,
            digest,
            hdr_metadata,
        })
    }

    /// HDR 元数据由输出变换阶段填写(每次 present 非空;MaxCLL/MaxFALL 自
    /// 编码帧实测,mastering primaries 随路径色域)。
    fn fill_hdr_metadata(&self, path: SwapchainPath, encoded: &[[f64; 3]]) -> HdrMetadata {
        let primaries = match path {
            SwapchainPath::PqRec2020 => color::REC2020,
            SwapchainPath::Sdr | SwapchainPath::ScRgb => color::REC709,
        };
        let (max_cll, max_fall) = match path {
            SwapchainPath::PqRec2020 => {
                // PQ 路径:码值 → nits 实测统计。
                let mut peak = 0.0f64;
                let mut sum = 0.0f64;
                for px in encoded {
                    let y = color::st2084_to_y(px[1].max(px[0]).max(px[2]).clamp(0.0, 1.0));
                    peak = peak.max(y);
                    sum += y;
                }
                let avg = if encoded.is_empty() {
                    0.0
                } else {
                    sum / encoded.len() as f64
                };
                (peak, avg)
            }
            SwapchainPath::Sdr | SwapchainPath::ScRgb => {
                // 非 HDR:SDR 参考域登记(100 nits 白点参考;MaxCLL = 显示线性
                // 峰值 ×100,MaxFALL = 均值 ×100)。
                let scale = if path == SwapchainPath::ScRgb {
                    super::view_transform::SCRGB_WHITE_NITS
                } else {
                    super::view_transform::SDR_REFERENCE_WHITE_NITS
                };
                let inv_scale = super::view_transform::SDR_REFERENCE_WHITE_NITS / scale;
                let mut peak = 0.0f64;
                let mut sum = 0.0f64;
                for px in encoded {
                    let lin = match path {
                        SwapchainPath::ScRgb => px[1].max(px[0]).max(px[2]) * inv_scale,
                        _ => color::bt1886_fwd(px[1].max(px[0]).max(px[2]).clamp(0.0, 1.0), 2.4, 1.0, 0.0),
                    };
                    let nits = lin * super::view_transform::SDR_REFERENCE_WHITE_NITS;
                    peak = peak.max(nits);
                    sum += nits;
                }
                let avg = if encoded.is_empty() {
                    0.0
                } else {
                    sum / encoded.len() as f64
                };
                (peak, avg)
            }
        };
        HdrMetadata {
            max_cll_nits: max_cll,
            max_fall_nits: max_fall,
            mastering_primaries: [primaries.red, primaries.green, primaries.blue],
            white_point: primaries.white,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::view_transform::{canonical_hdr_frame, ViewTransformRegistry};

    //@ spec: RXS-0369
    #[test]
    fn path_encoding_legality_closed_set() {
        // 合法三组合。
        for p in [SwapchainPath::Sdr, SwapchainPath::ScRgb, SwapchainPath::PqRec2020] {
            assert!(validate_path_encoding(p, p.legal_encoding()).is_ok());
        }
        // 非 HDR 携带 PQ → 专属 typed Err(RED 锚)。
        assert!(matches!(
            validate_path_encoding(SwapchainPath::Sdr, OutputEncoding::PqSt2084Rec2020),
            Err(DisplayError::PqOutputOnNonHdrSwapchain { path: "sdr" })
        ));
        assert!(matches!(
            validate_path_encoding(SwapchainPath::ScRgb, OutputEncoding::PqSt2084Rec2020),
            Err(DisplayError::PqOutputOnNonHdrSwapchain { path: "scrgb" })
        ));
        // 其余错配 → mismatch。
        assert!(matches!(
            validate_path_encoding(SwapchainPath::Sdr, OutputEncoding::ScRgbLinear),
            Err(DisplayError::EncodingPathMismatch { .. })
        ));
        assert!(matches!(
            validate_path_encoding(SwapchainPath::PqRec2020, OutputEncoding::SdrBt1886),
            Err(DisplayError::EncodingPathMismatch { .. })
        ));
    }

    //@ spec: RXS-0369
    #[test]
    fn runtime_switch_deterministic_bit_equal() {
        let registry = ViewTransformRegistry::with_builtins();
        let plugin = registry.get("aces13").unwrap();
        let frame = canonical_hdr_frame();
        let mut pipe = DisplayPipeline::assemble(SwapchainPath::Sdr, 100.0).unwrap();
        let a1 = pipe.present(&frame, plugin).unwrap();
        pipe.switch_to(SwapchainPath::ScRgb).unwrap();
        let b = pipe.present(&frame, plugin).unwrap();
        pipe.switch_to(SwapchainPath::PqRec2020).unwrap();
        let c = pipe.present(&frame, plugin).unwrap();
        pipe.switch_to(SwapchainPath::Sdr).unwrap();
        let a2 = pipe.present(&frame, plugin).unwrap();
        // 同输入切同路径输出位级一致。
        assert_eq!(a1.digest, a2.digest);
        assert_eq!(a1.encoded, a2.encoded);
        // 路径间输出分叉(编码面确实切换)。
        assert_ne!(a1.digest, b.digest);
        assert_ne!(b.digest, c.digest);
        // 切换证据日志非空且有序。
        assert_eq!(pipe.switch_log().len(), 3);
        assert_eq!(pipe.switch_log()[0].from, SwapchainPath::Sdr);
        assert_eq!(pipe.switch_log()[0].to, SwapchainPath::ScRgb);
        // HDR 元数据每次 present 非空。
        assert!(c.hdr_metadata.max_cll_nits > 0.0);
        assert!(c.hdr_metadata.max_fall_nits > 0.0);
    }

    //@ spec: RXS-0369
    #[test]
    fn pq_on_non_hdr_present_rejected() {
        let registry = ViewTransformRegistry::with_builtins();
        let plugin = registry.get("aces13").unwrap();
        let frame = canonical_hdr_frame();
        let pipe = DisplayPipeline::assemble(SwapchainPath::Sdr, 100.0).unwrap();
        // SDR 交换链 + PQ 编码 → typed Err。
        assert!(matches!(
            pipe.present_explicit(&frame, plugin, SwapchainPath::Sdr, OutputEncoding::PqSt2084Rec2020),
            Err(DisplayError::PqOutputOnNonHdrSwapchain { .. })
        ));
        assert!(matches!(
            pipe.present_explicit(&frame, plugin, SwapchainPath::ScRgb, OutputEncoding::PqSt2084Rec2020),
            Err(DisplayError::PqOutputOnNonHdrSwapchain { .. })
        ));
        // PQ 路径 + PQ 编码 → 合法。
        assert!(pipe
            .present_explicit(&frame, plugin, SwapchainPath::PqRec2020, OutputEncoding::PqSt2084Rec2020)
            .is_ok());
    }

    //@ spec: RXS-0369
    #[test]
    fn hdr_calibration_not_triggered_registered() {
        let report = query_hdr_capability();
        assert!(!report.display_hdr_capable);
        match report.calibration {
            HdrCalibrationStatus::NotTriggered { reason } => assert!(reason.contains("not-triggered")),
        }
        // 强制消费 → fail-closed typed Err。
        assert!(matches!(
            require_hdr_calibration(&report),
            Err(DisplayError::HdrCalibrationNotTriggered { .. })
        ));
    }
}
