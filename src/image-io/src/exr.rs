//! EXR HDR 帧容器（spec/imageio.md §2A RXS-0385；RFC-0026 §4.1；G10.4 M134）。
//!
//! 自研最小子集（RFC-0026 §9 Q2 裁决执行面）：OpenEXR scanline 布局，
//! Rurix 侧 canonical = float32 每通道（RGB 三通道 / 单通道 Y 两形态）；
//! 压缩闭集 `{NONE, ZIP}`——v1 实现面 = **NONE 编码 + NONE 解码**，ZIP（及
//! 闭集外一切压缩值）fail-closed 显式 [`ImageError::UnsupportedCompression`]
//! （禁静默；harness 须将 UE 侧压缩配置收窄至自研可解子集并 evidence 登记）。
//!
//! 元数据闭集（RXS-0385 L3）：标准属性白名单（结构属性 + chromaticities /
//! pixelAspectRatio / screenWindowCenter / screenWindowWidth）+ `rurix:*`
//! 命名空间九字段；写侧闭集外禁写。读取侧分端策略（L4）：`rurix` 帧
//! strict（闭集外属性确定性拒绝）；`ue5` 帧 strip-and-log（闭集外属性剥离
//! 并逐属性登记属性名与值 digest；`rurix:*` 属性出现 = 命名空间冒充拒绝）。
//!
//! 纪律：全 safe（crate `unsafe_code = "deny"`）、零外部依赖、纯函数确定性
//! ——同一输入产逐字节一致字节流（canonical 编码不含路径 / mtime / 随机量）。

use crate::{ImageError, ImageResult};

/// EXR magic（`0x762f3101`，小端字节 `76 2f 31 01`）。
pub const EXR_MAGIC: [u8; 4] = [0x76, 0x2f, 0x31, 0x01];
/// EXR version 字段（version 2，scanline 无 flags）。
pub const EXR_VERSION: [u8; 4] = [0x02, 0x00, 0x00, 0x00];
/// version 字段 flags 字节合法掩码：仅 long names（0x04，UE 写出器长属性名
/// 面；tiled=0x02 / deep=0x08 / multi-part=0x10 均子集外拒绝）。
const EXR_FLAGS_ALLOWED: u8 = 0x04;

/// OpenEXR compression 枚举：NONE=0（v1 编解面）。
const COMPRESSION_NONE: u8 = 0;
/// OpenEXR pixel_type：HALF=1（fp16，UE 侧实测面，读取提升 f32）。
const PIXEL_TYPE_HALF: i32 = 1;
/// OpenEXR pixel_type：FLOAT=2（fp32，Rurix canonical）。
const PIXEL_TYPE_FLOAT: i32 = 2;

/// Rec.709 primaries + D65 白点位级闭集（RXS-0385 L2；与色彩空间闭集互证）。
/// 序：red(x,y) green(x,y) blue(x,y) white(x,y)。
pub const CHROMATICITIES_REC709_D65: [f32; 8] =
    [0.64, 0.33, 0.30, 0.60, 0.15, 0.06, 0.3127, 0.3290];

/// 域闭集（RXS-0385 L2 / RXS-0386 L1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExrDomain {
    /// HDR 臂：tonemap 前 scene-referred 线性帧（`"scene-linear-hdr"`）。
    SceneLinearHdr,
    /// LDR 臂：显示域 sRGB `[0,1]` 编码帧（`"display-referred-ldr"`）。
    DisplayReferredLdr,
}

impl ExrDomain {
    /// 域闭集字面（`"scene-linear-hdr"` / `"display-referred-ldr"`）——
    /// G10.5b 起 pub（M137 diff 报告器 domain 派生消费面，H1 修订；闭集字面不变）。
    pub fn as_str(self) -> &'static str {
        match self {
            ExrDomain::SceneLinearHdr => "scene-linear-hdr",
            ExrDomain::DisplayReferredLdr => "display-referred-ldr",
        }
    }
    fn parse(s: &str) -> ImageResult<Self> {
        match s {
            "scene-linear-hdr" => Ok(ExrDomain::SceneLinearHdr),
            "display-referred-ldr" => Ok(ExrDomain::DisplayReferredLdr),
            other => Err(ImageError::MetadataViolation(format!(
                "rurix:domain 闭集外取值: {other:?}"
            ))),
        }
    }
}

/// transfer 闭集（`"linear"` / `"srgb"`；HDR 臂必 linear，LDR 臂必 srgb）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExrTransfer {
    /// 线性（HDR 臂）。
    Linear,
    /// sRGB（LDR 臂）。
    Srgb,
}

impl ExrTransfer {
    fn as_str(self) -> &'static str {
        match self {
            ExrTransfer::Linear => "linear",
            ExrTransfer::Srgb => "srgb",
        }
    }
    fn parse(s: &str) -> ImageResult<Self> {
        match s {
            "linear" => Ok(ExrTransfer::Linear),
            "srgb" => Ok(ExrTransfer::Srgb),
            other => Err(ImageError::MetadataViolation(format!(
                "rurix:transfer 闭集外取值: {other:?}"
            ))),
        }
    }
}

/// 位深登记（`"float32"` canonical / `"float16"` UE 侧实测登记）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExrBitDepth {
    /// float32 每通道（Rurix canonical；写侧唯一形态）。
    Float32,
    /// float16 每通道（UE 侧实测；读取提升 f32，写侧不产）。
    Float16,
}

impl ExrBitDepth {
    fn as_str(self) -> &'static str {
        match self {
            ExrBitDepth::Float32 => "float32",
            ExrBitDepth::Float16 => "float16",
        }
    }
    fn parse(s: &str) -> ImageResult<Self> {
        match s {
            "float32" => Ok(ExrBitDepth::Float32),
            "float16" => Ok(ExrBitDepth::Float16),
            other => Err(ImageError::MetadataViolation(format!(
                "rurix:bit_depth 闭集外取值: {other:?}"
            ))),
        }
    }
}

/// 来源端闭集（分端读取策略分派键，RXS-0385 L4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExrSourceEnd {
    /// Rurix 端（strict：白名单与 `rurix:*` 闭集外属性确定性拒绝）。
    Rurix,
    /// UE5 端（strip-and-log：闭集外属性剥离登记，不拒真实帧）。
    Ue5,
}

impl ExrSourceEnd {
    fn as_str(self) -> &'static str {
        match self {
            ExrSourceEnd::Rurix => "rurix",
            ExrSourceEnd::Ue5 => "ue5",
        }
    }
}

/// view transform 枚举闭集（LDR 臂必；v1 契约字面仅 `"aces13"`，余演进位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExrViewTransform {
    /// ACES 1.3（v1 唯一契约值）。
    Aces13,
    /// ACES 2.0（演进位）。
    Aces20,
    /// AgX（演进位）。
    Agx,
    /// 中性（演进位）。
    Neutral,
    /// UE5 默认 ACES Filmic（caliber_diff 登记面）。
    Ue5DefaultAcesFilmic,
}

impl ExrViewTransform {
    fn as_str(self) -> &'static str {
        match self {
            ExrViewTransform::Aces13 => "aces13",
            ExrViewTransform::Aces20 => "aces20",
            ExrViewTransform::Agx => "agx",
            ExrViewTransform::Neutral => "neutral",
            ExrViewTransform::Ue5DefaultAcesFilmic => "ue5-default-aces-filmic",
        }
    }
    fn parse(s: &str) -> ImageResult<Self> {
        match s {
            "aces13" => Ok(ExrViewTransform::Aces13),
            "aces20" => Ok(ExrViewTransform::Aces20),
            "agx" => Ok(ExrViewTransform::Agx),
            "neutral" => Ok(ExrViewTransform::Neutral),
            "ue5-default-aces-filmic" => Ok(ExrViewTransform::Ue5DefaultAcesFilmic),
            other => Err(ImageError::MetadataViolation(format!(
                "rurix:view_transform 闭集外取值: {other:?}"
            ))),
        }
    }
}

/// 派生链标记（RXS-0385 L3 / RXS-0386 L2 / imageio.md v1.2 修订行加性登记）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExrDerivation {
    /// 直接捕获（HDR 臂）。
    Capture,
    /// LDR 臂派生链（`"derived:host-srgb-encoder-v1"`）。
    DerivedHostSrgbEncoderV1,
    /// M137 diff 报告误差标量场（`"derived:diff-report-v1"`；v1.2 加性登记，
    /// 双源 digest 归 evidence artifacts 闭集，`source_frame_digest` 缺省合法）。
    DerivedDiffReportV1,
}

impl ExrDerivation {
    fn as_str(self) -> &'static str {
        match self {
            ExrDerivation::Capture => "capture",
            ExrDerivation::DerivedHostSrgbEncoderV1 => "derived:host-srgb-encoder-v1",
            ExrDerivation::DerivedDiffReportV1 => "derived:diff-report-v1",
        }
    }
    fn parse(s: &str) -> ImageResult<Self> {
        match s {
            "capture" => Ok(ExrDerivation::Capture),
            "derived:host-srgb-encoder-v1" => Ok(ExrDerivation::DerivedHostSrgbEncoderV1),
            "derived:diff-report-v1" => Ok(ExrDerivation::DerivedDiffReportV1),
            other => Err(ImageError::MetadataViolation(format!(
                "rurix:derivation 闭集外取值: {other:?}"
            ))),
        }
    }
}

/// chromaticities 来源登记（UE 帧缺失补写时必填）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaticitiesOrigin {
    /// 写出器原生（Rurix 帧）。
    Writer,
    /// harness 补写（UE 帧缺失时；`"harness-backfill"`）。
    HarnessBackfill,
}

impl ChromaticitiesOrigin {
    fn as_str(self) -> &'static str {
        match self {
            ChromaticitiesOrigin::Writer => "writer",
            ChromaticitiesOrigin::HarnessBackfill => "harness-backfill",
        }
    }
    fn parse(s: &str) -> ImageResult<Self> {
        match s {
            "writer" => Ok(ChromaticitiesOrigin::Writer),
            "harness-backfill" => Ok(ChromaticitiesOrigin::HarnessBackfill),
            other => Err(ImageError::MetadataViolation(format!(
                "rurix:chromaticities_origin 闭集外取值: {other:?}"
            ))),
        }
    }
}

/// 通道布局（canonical 两形态：RGB 三通道 / 单通道 Y 误差标量场）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExrChannelLayout {
    /// RGB 三通道（EXR 字母序存储 B,G,R）。
    Rgb,
    /// 单通道 Y（M137 误差标量场）。
    Y,
}

impl ExrChannelLayout {
    /// 通道数。
    pub fn channels(self) -> usize {
        match self {
            ExrChannelLayout::Rgb => 3,
            ExrChannelLayout::Y => 1,
        }
    }
    /// EXR 存储序通道名（字母序）。
    fn storage_names(self) -> &'static [&'static str] {
        match self {
            ExrChannelLayout::Rgb => &["B", "G", "R"],
            ExrChannelLayout::Y => &["Y"],
        }
    }
}

/// 帧元数据闭集（RXS-0385 L3；写侧校验齐备）。
#[derive(Debug, Clone, PartialEq)]
pub struct ExrMetadata {
    /// `rurix:schema_version`（`"1"` 起，加性演进）。
    pub schema_version: String,
    /// `rurix:domain`。
    pub domain: ExrDomain,
    /// `rurix:transfer`。
    pub transfer: ExrTransfer,
    /// `rurix:bit_depth`（Rurix canonical = Float32）。
    pub bit_depth: ExrBitDepth,
    /// `rurix:source_end`。
    pub source_end: ExrSourceEnd,
    /// `rurix:view_transform`（LDR 臂必）。
    pub view_transform: Option<ExrViewTransform>,
    /// `rurix:capture_params_digest`（帧 ↔ 参数互证；`"sha256:<64hex>"`）。
    pub capture_params_digest: String,
    /// `rurix:derivation`。
    pub derivation: ExrDerivation,
    /// `rurix:source_frame_digest`（派生帧必；capture 帧缺省合法）。
    pub source_frame_digest: Option<String>,
    /// `rurix:chromaticities_origin`（条件必）。
    pub chromaticities_origin: Option<ChromaticitiesOrigin>,
}

impl ExrMetadata {
    /// 写侧闭集校验（fail-closed；任一违例 → [`ImageError::MetadataViolation`]）。
    pub fn validate(&self) -> ImageResult<()> {
        if self.schema_version != "1" {
            return Err(ImageError::MetadataViolation(format!(
                "rurix:schema_version 须为 \"1\": {:?}",
                self.schema_version
            )));
        }
        // 域 ↔ transfer 互证（sRGB/线性混标即 RED）。
        match (self.domain, self.transfer) {
            (ExrDomain::SceneLinearHdr, ExrTransfer::Linear)
            | (ExrDomain::DisplayReferredLdr, ExrTransfer::Srgb) => {}
            _ => {
                return Err(ImageError::MetadataViolation(format!(
                    "域/transfer 混标: domain={:?} transfer={:?}（sRGB/线性混标）",
                    self.domain.as_str(),
                    self.transfer.as_str()
                )));
            }
        }
        // LDR 臂 view_transform 必填。
        if self.domain == ExrDomain::DisplayReferredLdr && self.view_transform.is_none() {
            return Err(ImageError::MetadataViolation(
                "LDR 臂 rurix:view_transform 必填".to_owned(),
            ));
        }
        // digest 形态闭集（"sha256:<64hex>"）。
        if !is_sha256_digest(&self.capture_params_digest) {
            return Err(ImageError::MetadataViolation(format!(
                "rurix:capture_params_digest 形态非法: {:?}",
                self.capture_params_digest
            )));
        }
        // 派生链互证：LDR 派生帧 source_frame_digest 必填；diff 报告误差帧
        // 双源 digest 归 evidence artifacts（v1.2 修订行），本字段缺省合法。
        if self.derivation == ExrDerivation::DerivedHostSrgbEncoderV1 {
            match &self.source_frame_digest {
                Some(d) if is_sha256_digest(d) => {}
                _ => {
                    return Err(ImageError::MetadataViolation(
                        "派生帧 rurix:source_frame_digest 必填且须 sha256 形态".to_owned(),
                    ));
                }
            }
        }
        if let Some(d) = &self.source_frame_digest
            && !is_sha256_digest(d)
        {
            return Err(ImageError::MetadataViolation(format!(
                "rurix:source_frame_digest 形态非法: {d:?}"
            )));
        }
        Ok(())
    }
}

/// digest 形态谓词（`"sha256:"` + 64 小写 hex）。
fn is_sha256_digest(s: &str) -> bool {
    let Some(hex) = s.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// EXR 帧（行主序像素 + 元数据闭集；像素 f32，RGB 序 R,G,B 或单通道 Y）。
#[derive(Debug, Clone, PartialEq)]
pub struct ExrImage {
    /// 宽（列数）。
    pub width: u32,
    /// 高（行数）。
    pub height: u32,
    /// 通道布局。
    pub layout: ExrChannelLayout,
    /// 行主序像素（长度 = width × height × channels；RGB 序 R,G,B）。
    pub pixels: Vec<f32>,
    /// 元数据闭集。
    pub metadata: ExrMetadata,
}

impl ExrImage {
    /// 构造并校验（尺寸/像素长度/元数据闭集，fail-closed）。
    pub fn new(
        width: u32,
        height: u32,
        layout: ExrChannelLayout,
        pixels: Vec<f32>,
        metadata: ExrMetadata,
    ) -> ImageResult<Self> {
        let img = Self {
            width,
            height,
            layout,
            pixels,
            metadata,
        };
        img.validate_shape()?;
        Ok(img)
    }

    /// 尺寸/像素长度/值域/元数据闭集校验（构造与编码共用单一事实源）。
    pub fn validate_shape(&self) -> ImageResult<()> {
        if self.width == 0 || self.height == 0 {
            return Err(ImageError::InvalidExr(
                "EXR 帧宽/高须为正（空帧禁入）".to_owned(),
            ));
        }
        let want = self.width as usize * self.height as usize * self.layout.channels();
        if self.pixels.len() != want {
            return Err(ImageError::InvalidExr(format!(
                "像素长度 {} ≠ {}×{}×{}（={want}）",
                self.pixels.len(),
                self.width,
                self.height,
                self.layout.channels()
            )));
        }
        if self.pixels.iter().any(|v| v.is_nan()) {
            return Err(ImageError::InvalidExr(
                "NaN 禁入 canonical 帧值域（RXS-0384 L1 同口径）".to_owned(),
            ));
        }
        self.metadata.validate()
    }
}

/// strip-and-log 登记条目（剥离属性名 + 值 digest 输入面）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrippedAttribute {
    /// 属性名（如 `unreal/frameRenderStartTimeUTC`）。
    pub name: String,
    /// 属性类型（如 `string` / `int` / `tiledesc`）。
    pub attr_type: String,
    /// 值字节长度。
    pub value_len: usize,
    /// 剥离事由（`"ue5-strip-and-log"` / `"alpha-channel-strip"`）。
    pub reason: String,
}

/// 解码产物（帧数据 + 分端策略登记面）。
#[derive(Debug, Clone)]
pub struct DecodedExr {
    /// 宽（列数）。
    pub width: u32,
    /// 高（行数）。
    pub height: u32,
    /// 通道布局（alpha 已剥离；RGB 序 R,G,B 或单通道 Y）。
    pub layout: ExrChannelLayout,
    /// 行主序像素 f32（fp16 源已精确提升）。
    pub pixels: Vec<f32>,
    /// 落盘源位深（fp16/fp32，provenance 登记面）。
    pub source_bit_depth: ExrBitDepth,
    /// rurix 帧元数据闭集（ue5 帧无 `rurix:*` 属性 → None，登记面在 stripped）。
    pub metadata: Option<ExrMetadata>,
    /// strip-and-log 登记（ue5 帧闭集外属性 / alpha 通道剥离，逐条在录）。
    pub stripped: Vec<StrippedAttribute>,
}

// ─────────────────────────── 编码（NONE，确定性字节流） ───────────────────────────

fn push_str(bytes: &mut Vec<u8>, s: &str) {
    bytes.extend_from_slice(s.as_bytes());
    bytes.push(0);
}

fn attr_string(bytes: &mut Vec<u8>, name: &str, value: &str) {
    push_str(bytes, name);
    push_str(bytes, "string");
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn attr_box2i(bytes: &mut Vec<u8>, name: &str, x_max: i32, y_max: i32) {
    push_str(bytes, name);
    push_str(bytes, "box2i");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    for v in [0i32, 0, x_max, y_max] {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
}

fn attr_float(bytes: &mut Vec<u8>, name: &str, value: f32) {
    push_str(bytes, name);
    push_str(bytes, "float");
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn attr_v2f(bytes: &mut Vec<u8>, name: &str, x: f32, y: f32) {
    push_str(bytes, name);
    push_str(bytes, "v2f");
    bytes.extend_from_slice(&8u32.to_le_bytes());
    bytes.extend_from_slice(&x.to_le_bytes());
    bytes.extend_from_slice(&y.to_le_bytes());
}

fn attr_byte(bytes: &mut Vec<u8>, name: &str, ty: &str, value: u8) {
    push_str(bytes, name);
    push_str(bytes, ty);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.push(value);
}

fn encode_header(img: &ExrImage) -> Vec<u8> {
    let md = &img.metadata;
    let mut h = Vec::new();
    // channels（chlist：name\0 pixel_type i32 pLinear u8 reserved[3] xS i32 yS i32，\0 终止）。
    push_str(&mut h, "channels");
    push_str(&mut h, "chlist");
    let mut chlist = Vec::new();
    for name in img.layout.storage_names() {
        push_str(&mut chlist, name);
        chlist.extend_from_slice(&PIXEL_TYPE_FLOAT.to_le_bytes());
        chlist.push(0); // pLinear
        chlist.extend_from_slice(&[0, 0, 0]); // reserved
        chlist.extend_from_slice(&1i32.to_le_bytes()); // xSampling
        chlist.extend_from_slice(&1i32.to_le_bytes()); // ySampling
    }
    chlist.push(0);
    h.extend_from_slice(&(chlist.len() as u32).to_le_bytes());
    h.extend_from_slice(&chlist);
    // chromaticities（Rec.709/D65 位级闭集，必填）。
    push_str(&mut h, "chromaticities");
    push_str(&mut h, "chromaticities");
    h.extend_from_slice(&32u32.to_le_bytes());
    for v in CHROMATICITIES_REC709_D65 {
        h.extend_from_slice(&v.to_le_bytes());
    }
    // compression = NONE（v1 唯一编码面）。
    attr_byte(&mut h, "compression", "compression", COMPRESSION_NONE);
    // dataWindow / displayWindow（(0,0)-(w-1,h-1)）。
    attr_box2i(
        &mut h,
        "dataWindow",
        img.width as i32 - 1,
        img.height as i32 - 1,
    );
    attr_box2i(
        &mut h,
        "displayWindow",
        img.width as i32 - 1,
        img.height as i32 - 1,
    );
    // lineOrder = INCREASING_Y（0）。
    attr_byte(&mut h, "lineOrder", "lineOrder", 0);
    // 可选标准属性闭集。
    attr_float(&mut h, "pixelAspectRatio", 1.0);
    // rurix:* 命名空间闭集（名称字典序落位："rurix:"（0x3A）< "screenWindow*"）。
    attr_string(&mut h, "rurix:bit_depth", md.bit_depth.as_str());
    attr_string(
        &mut h,
        "rurix:capture_params_digest",
        &md.capture_params_digest,
    );
    if let Some(origin) = md.chromaticities_origin {
        attr_string(&mut h, "rurix:chromaticities_origin", origin.as_str());
    }
    attr_string(&mut h, "rurix:derivation", md.derivation.as_str());
    attr_string(&mut h, "rurix:domain", md.domain.as_str());
    attr_string(&mut h, "rurix:schema_version", &md.schema_version);
    attr_string(&mut h, "rurix:source_end", md.source_end.as_str());
    if let Some(digest) = &md.source_frame_digest {
        attr_string(&mut h, "rurix:source_frame_digest", digest);
    }
    attr_string(&mut h, "rurix:transfer", md.transfer.as_str());
    if let Some(vt) = md.view_transform {
        attr_string(&mut h, "rurix:view_transform", vt.as_str());
    }
    attr_v2f(&mut h, "screenWindowCenter", 0.0, 0.0);
    attr_float(&mut h, "screenWindowWidth", 1.0);
    h.push(0); // header 终止
    h
}

/// 编码 EXR 帧为确定字节流（RXS-0385 IR2；NONE 压缩 scanline）。
///
/// 同一输入（宽高 / 像素 / 元数据）产**逐字节一致**字节流；元数据闭集 /
/// 像素长度 / 值域任一违例 → 库层错误值（fail-closed，不产部分字节流）。
pub fn encode_exr(img: &ExrImage) -> ImageResult<Vec<u8>> {
    // 构造与编码共用同一校验（调用方可能绕过 ExrImage::new 直接拼装）。
    img.validate_shape()?;
    let header = encode_header(img);
    let height = img.height as usize;
    let width = img.width as usize;
    let channels = img.layout.channels();
    let mut out =
        Vec::with_capacity(8 + header.len() + height * 8 + img.pixels.len() * 4 + height * 8);
    out.extend_from_slice(&EXR_MAGIC);
    out.extend_from_slice(&EXR_VERSION);
    out.extend_from_slice(&header);
    let table_pos = out.len();
    out.resize(out.len() + height * 8, 0); // 偏移表占位
    let mut offsets = Vec::with_capacity(height);
    for y in 0..height {
        offsets.push(out.len() as u64);
        out.extend_from_slice(&(y as i32).to_le_bytes());
        let row_bytes = width * channels * 4;
        out.extend_from_slice(&(row_bytes as u32).to_le_bytes());
        // 逐扫描线逐通道平面（存储序 B,G,R / Y；源像素 RGB 序 R,G,B）。
        for (ci, _name) in img.layout.storage_names().iter().enumerate() {
            // 存储序下标 → 源 RGB 序下标（B=2,G=1,R=0；Y=0）。
            let src_ci = match img.layout {
                ExrChannelLayout::Rgb => 2 - ci,
                ExrChannelLayout::Y => 0,
            };
            let base = y * width * channels + src_ci;
            for x in 0..width {
                out.extend_from_slice(&img.pixels[base + x * channels].to_le_bytes());
            }
        }
    }
    for (i, off) in offsets.iter().enumerate() {
        out[table_pos + i * 8..table_pos + i * 8 + 8].copy_from_slice(&off.to_le_bytes());
    }
    Ok(out)
}

// ─────────────────────────── 解码（NONE；分端策略） ───────────────────────────

fn read_cstr(bytes: &[u8], pos: &mut usize) -> ImageResult<String> {
    let start = *pos;
    let rel = bytes
        .get(start..)
        .ok_or_else(|| ImageError::InvalidExr("header 截断（cstr）".to_owned()))?
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| ImageError::InvalidExr("header 截断（cstr 无 NUL）".to_owned()))?;
    let s = std::str::from_utf8(&bytes[start..start + rel])
        .map_err(|_| ImageError::InvalidExr("属性名非 UTF-8".to_owned()))?
        .to_owned();
    *pos = start + rel + 1;
    Ok(s)
}

fn read_u32le(bytes: &[u8], pos: &mut usize) -> ImageResult<u32> {
    let b = bytes
        .get(*pos..*pos + 4)
        .ok_or_else(|| ImageError::InvalidExr("截断（u32）".to_owned()))?;
    *pos += 4;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn read_i32le(bytes: &[u8], pos: &mut usize) -> ImageResult<i32> {
    Ok(read_u32le(bytes, pos)? as i32)
}

fn read_f32le(bytes: &[u8], pos: &mut usize) -> ImageResult<f32> {
    Ok(f32::from_bits(read_u32le(bytes, pos)?))
}

struct RawAttr {
    name: String,
    attr_type: String,
    value: Vec<u8>,
}

fn parse_header(bytes: &[u8]) -> ImageResult<(Vec<RawAttr>, usize)> {
    if bytes.len() < 9 || bytes[..4] != EXR_MAGIC {
        return Err(ImageError::InvalidExr(
            "EXR magic 不符（非真 EXR 帧）".to_owned(),
        ));
    }
    if bytes[4] != 2 || bytes[5] & !EXR_FLAGS_ALLOWED != 0 || bytes[6] != 0 || bytes[7] != 0 {
        return Err(ImageError::InvalidExr(format!(
            "EXR version 字段非 v2 scanline（tiled/deep/multi-part 子集外拒绝）: {:02x?}",
            &bytes[4..8]
        )));
    }
    let mut pos = 8usize;
    let mut attrs = Vec::new();
    loop {
        if bytes[pos] == 0 {
            pos += 1;
            break;
        }
        let name = read_cstr(bytes, &mut pos)?;
        let attr_type = read_cstr(bytes, &mut pos)?;
        let size = read_u32le(bytes, &mut pos)? as usize;
        let value = bytes
            .get(pos..pos + size)
            .ok_or_else(|| ImageError::InvalidExr(format!("属性 {name} 值截断")))?
            .to_vec();
        pos += size;
        attrs.push(RawAttr {
            name,
            attr_type,
            value,
        });
    }
    Ok((attrs, pos))
}

/// 标准属性白名单（结构属性 + 可选标准属性闭集，RXS-0385 L3）。
const STD_WHITELIST: [&str; 9] = [
    "channels",
    "chromaticities",
    "compression",
    "dataWindow",
    "displayWindow",
    "lineOrder",
    "pixelAspectRatio",
    "screenWindowCenter",
    "screenWindowWidth",
];

/// rurix:* 命名空间闭集（九字段）。
const RURIX_ATTRS: [&str; 10] = [
    "rurix:bit_depth",
    "rurix:capture_params_digest",
    "rurix:chromaticities_origin",
    "rurix:derivation",
    "rurix:domain",
    "rurix:schema_version",
    "rurix:source_end",
    "rurix:source_frame_digest",
    "rurix:transfer",
    "rurix:view_transform",
];

fn is_std_whitelisted(name: &str) -> bool {
    STD_WHITELIST.contains(&name)
}

fn is_rurix_attr(name: &str) -> bool {
    name.starts_with("rurix:")
}

fn is_rurix_closed_set(name: &str) -> bool {
    RURIX_ATTRS.contains(&name)
}

fn attr_as_string(attr: &RawAttr) -> ImageResult<String> {
    if attr.attr_type != "string" {
        return Err(ImageError::MetadataViolation(format!(
            "属性 {} 类型须为 string，实得 {:?}",
            attr.name, attr.attr_type
        )));
    }
    std::str::from_utf8(&attr.value)
        .map(|s| s.to_owned())
        .map_err(|_| ImageError::InvalidExr(format!("属性 {} 值非 UTF-8", attr.name)))
}

fn attr_as_box2i(attr: &RawAttr) -> ImageResult<(i32, i32, i32, i32)> {
    if attr.attr_type != "box2i" || attr.value.len() != 16 {
        return Err(ImageError::InvalidExr(format!(
            "属性 {} 须为 box2i[16]",
            attr.name
        )));
    }
    let mut pos = 0usize;
    let v: Vec<i32> = (0..4)
        .map(|_| read_i32le(&attr.value, &mut pos))
        .collect::<ImageResult<Vec<i32>>>()?;
    Ok((v[0], v[1], v[2], v[3]))
}

fn find_attr<'a>(attrs: &'a [RawAttr], name: &str) -> Option<&'a RawAttr> {
    attrs.iter().find(|a| a.name == name)
}

/// fp16 位模式 → f32 精确提升（IEEE-754 binary16 → binary32，逐值位级可逆；
/// NaN/±Inf 映射保持类别）。
pub fn half_to_f32(bits: u16) -> f32 {
    let sign = (bits >> 15) & 1;
    let exp = (bits >> 10) & 0x1f;
    let frac = bits & 0x3ff;
    let mag = match exp {
        0 => (frac as f32) * f32::from_bits(0x3380_0000), // 2^-24
        31 => {
            if frac == 0 {
                f32::INFINITY
            } else {
                f32::NAN
            }
        }
        e => {
            // (1 + frac/1024) × 2^(e-15)，全链路 2 的幂运算，f32 精确。
            let mant = 1.0f32 + (frac as f32) / 1024.0;
            let scale = f32::from_bits(((e as u32) + 112) << 23); // 2^(e-15)
            mant * scale
        }
    };
    if sign == 1 { -mag } else { mag }
}

struct ChannelInfo {
    name: String,
    pixel_type: i32,
    x_sampling: i32,
    y_sampling: i32,
}

fn parse_chlist(attr: &RawAttr) -> ImageResult<Vec<ChannelInfo>> {
    if attr.attr_type != "chlist" {
        return Err(ImageError::InvalidExr(
            "channels 属性须为 chlist".to_owned(),
        ));
    }
    let mut pos = 0usize;
    let mut out = Vec::new();
    loop {
        if pos >= attr.value.len() {
            return Err(ImageError::InvalidExr("chlist 截断".to_owned()));
        }
        if attr.value[pos] == 0 {
            break;
        }
        let name = read_cstr(&attr.value, &mut pos)?;
        let pixel_type = read_i32le(&attr.value, &mut pos)?;
        let _p_linear = attr
            .value
            .get(pos)
            .ok_or_else(|| ImageError::InvalidExr("chlist pLinear 截断".to_owned()))?;
        pos += 4; // pLinear + reserved[3]
        let x_sampling = read_i32le(&attr.value, &mut pos)?;
        let y_sampling = read_i32le(&attr.value, &mut pos)?;
        out.push(ChannelInfo {
            name,
            pixel_type,
            x_sampling,
            y_sampling,
        });
    }
    Ok(out)
}

fn metadata_from_attrs(attrs: &[RawAttr]) -> ImageResult<ExrMetadata> {
    let get = |name: &str| -> ImageResult<String> {
        let attr = find_attr(attrs, name)
            .ok_or_else(|| ImageError::MetadataViolation(format!("元数据缺字段: {name}")))?;
        attr_as_string(attr)
    };
    let md = ExrMetadata {
        schema_version: get("rurix:schema_version")?,
        domain: ExrDomain::parse(&get("rurix:domain")?)?,
        transfer: ExrTransfer::parse(&get("rurix:transfer")?)?,
        bit_depth: ExrBitDepth::parse(&get("rurix:bit_depth")?)?,
        source_end: ExrSourceEnd::Rurix,
        view_transform: match find_attr(attrs, "rurix:view_transform") {
            Some(a) => Some(ExrViewTransform::parse(&attr_as_string(a)?)?),
            None => None,
        },
        capture_params_digest: get("rurix:capture_params_digest")?,
        derivation: ExrDerivation::parse(&get("rurix:derivation")?)?,
        source_frame_digest: match find_attr(attrs, "rurix:source_frame_digest") {
            Some(a) => Some(attr_as_string(a)?),
            None => None,
        },
        chromaticities_origin: match find_attr(attrs, "rurix:chromaticities_origin") {
            Some(a) => Some(ChromaticitiesOrigin::parse(&attr_as_string(a)?)?),
            None => None,
        },
    };
    // source_end 字面互证（strict 端须自报 rurix）。
    let se = get("rurix:source_end")?;
    if se != "rurix" {
        return Err(ImageError::MetadataViolation(format!(
            "rurix:source_end 须为 \"rurix\": {se:?}"
        )));
    }
    md.validate()?;
    Ok(md)
}

/// 解码 EXR 帧（RXS-0385 IR2/L4；NONE 解码 + 分端读取策略）。
///
/// - `expected_end = Rurix`：strict——白名单与 `rurix:*` 闭集外属性确定性
///   拒绝；`rurix:*` 必填字段缺失 / 值闭集外 / chromaticities 非位级闭集值
///   均拒绝；
/// - `expected_end = Ue5`：strip-and-log——白名单外属性剥离并逐条登记
///   （`rurix:*` 属性出现 = 命名空间冒充，拒绝）；chromaticities 缺失或值
///   非闭集 → fail-closed（harness backfill 路径须先行，本读取面不静默）。
/// - 压缩：NONE 解码；ZIP（=3）及闭集外一切压缩值 →
///   [`ImageError::UnsupportedCompression`]（fail-closed 显式，禁静默）。
/// - 位深：FLOAT 直读；HALF 精确提升 f32（`source_bit_depth` 登记 fp16）。
pub fn decode_exr(bytes: &[u8], expected_end: ExrSourceEnd) -> ImageResult<DecodedExr> {
    let (attrs, body_pos) = parse_header(bytes)?;
    let mut stripped: Vec<StrippedAttribute> = Vec::new();

    // 分端策略：闭集外属性处置（RXS-0385 L4）。
    for a in &attrs {
        let standard = is_std_whitelisted(&a.name);
        let rurix = is_rurix_attr(&a.name);
        match expected_end {
            ExrSourceEnd::Rurix => {
                if rurix {
                    if !is_rurix_closed_set(&a.name) {
                        return Err(ImageError::MetadataViolation(format!(
                            "rurix 帧 strict：rurix:* 闭集外属性 {:?}",
                            a.name
                        )));
                    }
                } else if !standard {
                    return Err(ImageError::MetadataViolation(format!(
                        "rurix 帧 strict：白名单外属性 {:?}",
                        a.name
                    )));
                }
            }
            ExrSourceEnd::Ue5 => {
                if rurix {
                    return Err(ImageError::MetadataViolation(format!(
                        "ue5 帧出现 rurix:* 属性 {:?}（命名空间冒充）",
                        a.name
                    )));
                }
                if !standard {
                    stripped.push(StrippedAttribute {
                        name: a.name.clone(),
                        attr_type: a.attr_type.clone(),
                        value_len: a.value.len(),
                        reason: "ue5-strip-and-log".to_owned(),
                    });
                }
            }
        }
    }

    // 结构属性核验。
    let compression = find_attr(&attrs, "compression")
        .ok_or_else(|| ImageError::InvalidExr("缺 compression 属性".to_owned()))?;
    if compression.value.len() != 1 {
        return Err(ImageError::InvalidExr(
            "compression 属性长度非法".to_owned(),
        ));
    }
    if compression.value[0] != COMPRESSION_NONE {
        return Err(ImageError::UnsupportedCompression(format!(
            "compression={}（闭集 {{NONE, ZIP}} 内 ZIP 解码 v1 未接通；其余压缩禁入）",
            compression.value[0]
        )));
    }
    let line_order = find_attr(&attrs, "lineOrder")
        .ok_or_else(|| ImageError::InvalidExr("缺 lineOrder 属性".to_owned()))?;
    if line_order.value.first().copied() != Some(0) {
        return Err(ImageError::InvalidExr(
            "lineOrder 非 INCREASING_Y（子集外）".to_owned(),
        ));
    }
    let dw = find_attr(&attrs, "dataWindow")
        .ok_or_else(|| ImageError::InvalidExr("缺 dataWindow 属性".to_owned()))
        .and_then(attr_as_box2i)?;
    let disp = find_attr(&attrs, "displayWindow")
        .ok_or_else(|| ImageError::InvalidExr("缺 displayWindow 属性".to_owned()))
        .and_then(attr_as_box2i)?;
    if dw.0 != 0 || dw.1 != 0 {
        return Err(ImageError::InvalidExr(
            "dataWindow 非零原点（子集外）".to_owned(),
        ));
    }
    let width = (dw.2 + 1) as u32;
    let height = (dw.3 + 1) as u32;
    if width == 0
        || height == 0
        || (dw.2 + 1) as i64 != width as i64
        || disp.2 != dw.2
        || disp.3 != dw.3
    {
        return Err(ImageError::InvalidExr(
            "dataWindow/displayWindow 尺寸非法或不一致".to_owned(),
        ));
    }

    // chromaticities 位级闭集互证（两端同判）。
    let chroma = find_attr(&attrs, "chromaticities").ok_or_else(|| {
        ImageError::MetadataViolation(
            "chromaticities 缺失（ue5 帧须经 harness backfill 先行，本面 fail-closed）".to_owned(),
        )
    })?;
    if chroma.attr_type != "chromaticities" || chroma.value.len() != 32 {
        return Err(ImageError::InvalidExr("chromaticities 形态非法".to_owned()));
    }
    {
        let mut pos = 0usize;
        for want in CHROMATICITIES_REC709_D65 {
            let got = read_f32le(&chroma.value, &mut pos)?;
            if got.to_bits() != want.to_bits() {
                return Err(ImageError::MetadataViolation(format!(
                    "chromaticities 值 ≠ Rec.709/D65 闭集（{got} ≠ {want}）"
                )));
            }
        }
    }

    // 通道表解析与布局判定（alpha 剥离登记）。
    let ch_attr = find_attr(&attrs, "channels")
        .ok_or_else(|| ImageError::InvalidExr("缺 channels 属性".to_owned()))?;
    let channels = parse_chlist(ch_attr)?;
    for c in &channels {
        if c.x_sampling != 1 || c.y_sampling != 1 {
            return Err(ImageError::InvalidExr(format!(
                "通道 {} 采样率非 1（子集外）",
                c.name
            )));
        }
        if c.pixel_type != PIXEL_TYPE_HALF && c.pixel_type != PIXEL_TYPE_FLOAT {
            return Err(ImageError::InvalidExr(format!(
                "通道 {} pixel_type={}（子集外，仅 HALF/FLOAT）",
                c.name, c.pixel_type
            )));
        }
    }
    let mut names: Vec<&str> = channels.iter().map(|c| c.name.as_str()).collect();
    let has_alpha = names.contains(&"A");
    if has_alpha {
        names.retain(|n| *n != "A");
        stripped.push(StrippedAttribute {
            name: "A".to_owned(),
            attr_type: "channel".to_owned(),
            value_len: 0,
            reason: "alpha-channel-strip".to_owned(),
        });
    }
    let layout = match names.as_slice() {
        ["B", "G", "R"] => ExrChannelLayout::Rgb,
        ["Y"] => ExrChannelLayout::Y,
        other => {
            return Err(ImageError::InvalidExr(format!(
                "通道集 {other:?} 非 canonical（B,G,R / 单通道 Y）"
            )));
        }
    };
    let bytes_per: Vec<usize> = channels
        .iter()
        .map(|c| {
            if c.pixel_type == PIXEL_TYPE_HALF {
                2
            } else {
                4
            }
        })
        .collect();
    let source_bit_depth = if channels.iter().all(|c| c.pixel_type == PIXEL_TYPE_HALF) {
        ExrBitDepth::Float16
    } else if channels.iter().all(|c| c.pixel_type == PIXEL_TYPE_FLOAT) {
        ExrBitDepth::Float32
    } else {
        return Err(ImageError::InvalidExr("通道位深混合（子集外）".to_owned()));
    };

    // rurix 帧元数据闭集重构（strict 端齐备校验）+ 位深互证（Rurix canonical
    // = float32 存储与 rurix:bit_depth="float32" 字面一致；fp16 存储的"rurix"
    // 帧非 canonical，拒绝）。
    let metadata = match expected_end {
        ExrSourceEnd::Rurix => {
            let md = metadata_from_attrs(&attrs)?;
            if source_bit_depth != ExrBitDepth::Float32 || md.bit_depth != ExrBitDepth::Float32 {
                return Err(ImageError::MetadataViolation(format!(
                    "rurix 帧位深非 float32 canonical（存储 {source_bit_depth:?} / 元数据 {:?}）",
                    md.bit_depth
                )));
            }
            Some(md)
        }
        ExrSourceEnd::Ue5 => None,
    };

    // 像素体解码（NONE scanline：偏移表 + 逐扫描线块）。
    let height_us = height as usize;
    let width_us = width as usize;
    let mut pos = body_pos;
    let mut offsets = Vec::with_capacity(height_us);
    for _ in 0..height_us {
        let b = bytes
            .get(pos..pos + 8)
            .ok_or_else(|| ImageError::InvalidExr("偏移表截断".to_owned()))?;
        offsets.push(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]));
        pos += 8;
    }
    let out_ch = layout.channels();
    let mut pixels = vec![0.0f32; width_us * height_us * out_ch];
    for (y, &off) in offsets.iter().enumerate() {
        let mut p = off as usize;
        let sy = read_i32le(bytes, &mut p)?;
        if sy != y as i32 {
            return Err(ImageError::InvalidExr(format!(
                "扫描线 y={sy} 与偏移表序 {y} 不符"
            )));
        }
        let packed = read_u32le(bytes, &mut p)? as usize;
        let want_packed = width_us * bytes_per.iter().sum::<usize>();
        if packed != want_packed {
            return Err(ImageError::InvalidExr(format!(
                "扫描线 {y} packed_size={packed} ≠ {want_packed}"
            )));
        }
        for (ci, c) in channels.iter().enumerate() {
            let bpc = bytes_per[ci];
            for x in 0..width_us {
                let v = if bpc == 2 {
                    let b = bytes
                        .get(p..p + 2)
                        .ok_or_else(|| ImageError::InvalidExr("像素体截断".to_owned()))?;
                    half_to_f32(u16::from_le_bytes([b[0], b[1]]))
                } else {
                    read_f32le(bytes, &mut p)?
                };
                if bpc == 2 {
                    p += 2;
                }
                if c.name == "A" {
                    continue; // alpha 剥离（不进入 canonical 缓冲）
                }
                let out_ci = match layout {
                    ExrChannelLayout::Rgb => match c.name.as_str() {
                        "R" => 0,
                        "G" => 1,
                        "B" => 2,
                        _ => unreachable!("通道集已闭集校验"),
                    },
                    ExrChannelLayout::Y => 0,
                };
                pixels[(y * width_us + x) * out_ch + out_ci] = v;
            }
        }
    }
    if pixels.iter().any(|v| v.is_nan()) {
        return Err(ImageError::InvalidExr(
            "NaN 帧值禁入 canonical 面".to_owned(),
        ));
    }
    Ok(DecodedExr {
        width,
        height,
        layout,
        pixels,
        source_bit_depth,
        metadata,
        stripped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe_metadata(domain: ExrDomain, transfer: ExrTransfer) -> ExrMetadata {
        ExrMetadata {
            schema_version: "1".to_owned(),
            domain,
            transfer,
            bit_depth: ExrBitDepth::Float32,
            source_end: ExrSourceEnd::Rurix,
            view_transform: None,
            capture_params_digest: format!("sha256:{}", "a".repeat(64)),
            derivation: ExrDerivation::Capture,
            source_frame_digest: None,
            chromaticities_origin: Some(ChromaticitiesOrigin::Writer),
        }
    }

    fn probe_frame(w: u32, h: u32) -> ExrImage {
        let md = probe_metadata(ExrDomain::SceneLinearHdr, ExrTransfer::Linear);
        let mut pixels = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                // 闭式探针：含 HDR>1 值、分数、负值（非 8-bit 精确表示面）。
                pixels.push(0.1 * x as f32 + 1.25);
                pixels.push(0.01 * y as f32 - 0.5);
                pixels.push((x + y) as f32 * 0.333_333_34);
            }
        }
        ExrImage::new(w, h, ExrChannelLayout::Rgb, pixels, md).unwrap()
    }

    //@ spec: RXS-0385
    // 往返无损 golden：capture→encode→decode 逐像素 float32 位级相等 +
    // 同输入两次编码逐字节一致（确定性字节流）。
    #[test]
    fn rgb_roundtrip_bit_exact_and_deterministic() {
        let img = probe_frame(7, 5);
        let bytes = encode_exr(&img).unwrap();
        let bytes2 = encode_exr(&img).unwrap();
        assert_eq!(bytes, bytes2, "同输入两次编码须逐字节一致");
        assert_eq!(&bytes[..4], &EXR_MAGIC);
        let dec = decode_exr(&bytes, ExrSourceEnd::Rurix).unwrap();
        assert_eq!(dec.width, 7);
        assert_eq!(dec.height, 5);
        assert_eq!(dec.layout, ExrChannelLayout::Rgb);
        assert_eq!(dec.source_bit_depth, ExrBitDepth::Float32);
        assert_eq!(dec.pixels.len(), img.pixels.len());
        for (a, b) in dec.pixels.iter().zip(img.pixels.iter()) {
            assert_eq!(a.to_bits(), b.to_bits(), "逐像素 float32 位级相等");
        }
        let md = dec.metadata.expect("rurix 帧元数据齐备");
        assert_eq!(md, img.metadata);
        assert!(dec.stripped.is_empty());
    }

    //@ spec: RXS-0385
    // 单通道 Y（误差标量场形态）往返无损。
    #[test]
    fn y_roundtrip_bit_exact() {
        let md = probe_metadata(ExrDomain::SceneLinearHdr, ExrTransfer::Linear);
        let pixels: Vec<f32> = (0..12).map(|i| i as f32 * 0.0625).collect();
        let img = ExrImage::new(4, 3, ExrChannelLayout::Y, pixels, md).unwrap();
        let bytes = encode_exr(&img).unwrap();
        let dec = decode_exr(&bytes, ExrSourceEnd::Rurix).unwrap();
        assert_eq!(dec.layout, ExrChannelLayout::Y);
        for (a, b) in dec.pixels.iter().zip(img.pixels.iter()) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
    }

    //@ spec: RXS-0385
    // 元数据闭集：sRGB/线性混标 fail-closed；缺字段 fail-closed；派生帧缺
    // source_frame_digest fail-closed；digest 形态非法 fail-closed。
    #[test]
    fn metadata_closed_set_enforced() {
        // 混标：HDR 域 + srgb。
        let mut md = probe_metadata(ExrDomain::SceneLinearHdr, ExrTransfer::Srgb);
        assert!(matches!(
            ExrImage::new(1, 1, ExrChannelLayout::Rgb, vec![0.0; 3], md.clone()),
            Err(ImageError::MetadataViolation(_))
        ));
        // 混标：LDR 域 + linear。
        md = probe_metadata(ExrDomain::DisplayReferredLdr, ExrTransfer::Linear);
        assert!(matches!(
            ExrImage::new(1, 1, ExrChannelLayout::Rgb, vec![0.0; 3], md),
            Err(ImageError::MetadataViolation(_))
        ));
        // LDR 臂缺 view_transform。
        md = probe_metadata(ExrDomain::DisplayReferredLdr, ExrTransfer::Srgb);
        assert!(matches!(
            ExrImage::new(1, 1, ExrChannelLayout::Rgb, vec![0.0; 3], md),
            Err(ImageError::MetadataViolation(_))
        ));
        // digest 形态非法。
        md = probe_metadata(ExrDomain::SceneLinearHdr, ExrTransfer::Linear);
        md.capture_params_digest = "deadbeef".to_owned();
        assert!(matches!(
            ExrImage::new(1, 1, ExrChannelLayout::Rgb, vec![0.0; 3], md),
            Err(ImageError::MetadataViolation(_))
        ));
        // 派生帧缺 source_frame_digest。
        md = probe_metadata(ExrDomain::DisplayReferredLdr, ExrTransfer::Srgb);
        md.view_transform = Some(ExrViewTransform::Aces13);
        md.derivation = ExrDerivation::DerivedHostSrgbEncoderV1;
        md.source_frame_digest = None;
        assert!(matches!(
            ExrImage::new(1, 1, ExrChannelLayout::Rgb, vec![0.0; 3], md.clone()),
            Err(ImageError::MetadataViolation(_))
        ));
        // 合法 LDR 派生帧放行。
        md.source_frame_digest = Some(format!("sha256:{}", "b".repeat(64)));
        assert!(ExrImage::new(1, 1, ExrChannelLayout::Rgb, vec![0.0; 3], md).is_ok());
    }

    //@ spec: RXS-0385
    // ZIP（及闭集外压缩值）fail-closed 显式 UnsupportedCompression（禁静默）。
    #[test]
    fn zip_compression_fail_closed() {
        let img = probe_frame(2, 2);
        let mut bytes = encode_exr(&img).unwrap();
        // 定位 compression 属性值字节（header 内唯一 compression=0x00 处）并改写为 3（ZIP）。
        let needle = b"compression\x00compression\x00\x01\x00\x00\x00\x00";
        let pos = bytes
            .windows(needle.len())
            .position(|w| w == needle)
            .expect("compression 属性在树");
        bytes[pos + needle.len() - 1] = 3;
        match decode_exr(&bytes, ExrSourceEnd::Rurix) {
            Err(ImageError::UnsupportedCompression(_)) => {}
            other => panic!("ZIP 须显式 UnsupportedCompression，实得 {other:?}"),
        }
        // 闭集外压缩值（PIZ=4）同判。
        bytes[pos + needle.len() - 1] = 4;
        assert!(matches!(
            decode_exr(&bytes, ExrSourceEnd::Rurix),
            Err(ImageError::UnsupportedCompression(_))
        ));
    }

    //@ spec: RXS-0385
    // fp16 → f32 精确提升：常规值 / 次正规 / 最大值 / 负值逐位核验。
    #[test]
    fn half_to_f32_exact() {
        assert_eq!(half_to_f32(0x0000), 0.0f32);
        assert_eq!(half_to_f32(0x8000), -0.0f32);
        assert_eq!(half_to_f32(0x3c00), 1.0f32);
        assert_eq!(half_to_f32(0x3800), 0.5f32);
        assert_eq!(half_to_f32(0xc000), -2.0f32);
        assert_eq!(half_to_f32(0x7bff), 65504.0f32); // half 最大正规
        assert_eq!(half_to_f32(0x0001), 5.960_464_5e-8_f32); // half 最小次正规 2^-24
        assert_eq!(half_to_f32(0x0400), 6.103_515_6e-5_f32); // half 最小正规 2^-14
        assert!(half_to_f32(0x7c00).is_infinite());
        assert!(half_to_f32(0x7e00).is_nan());
        // 全 16 位空间可穷举核验：有限值提升后与 f64 参考一致。
        for bits in 0u32..=0xffff {
            let h = bits as u16;
            let got = half_to_f32(h);
            let sign = if h >> 15 == 1 { -1.0f64 } else { 1.0f64 };
            let exp = ((h >> 10) & 0x1f) as i32;
            let frac = (h & 0x3ff) as f64;
            let want = match exp {
                0 => sign * frac * 2f64.powi(-24),
                31 => continue,
                e => sign * (1.0 + frac / 1024.0) * 2f64.powi(e - 15),
            };
            assert_eq!(got as f64, want, "bits={h:04x}");
        }
    }

    /// 手造 ue5 形态帧（fp16 RGBA + 闭集外 unreal/* 属性 + 白名单齐），
    /// 供 strip-and-log 读取测试。
    fn make_ue5_like_frame(w: u32, h: u32, with_chroma: bool) -> Vec<u8> {
        let mut hdr = Vec::new();
        // channels: A,B,G,R 各 HALF。
        push_str(&mut hdr, "channels");
        push_str(&mut hdr, "chlist");
        let mut chl = Vec::new();
        for name in ["A", "B", "G", "R"] {
            push_str(&mut chl, name);
            chl.extend_from_slice(&PIXEL_TYPE_HALF.to_le_bytes());
            chl.push(0);
            chl.extend_from_slice(&[0, 0, 0]);
            chl.extend_from_slice(&1i32.to_le_bytes());
            chl.extend_from_slice(&1i32.to_le_bytes());
        }
        chl.push(0);
        hdr.extend_from_slice(&(chl.len() as u32).to_le_bytes());
        hdr.extend_from_slice(&chl);
        if with_chroma {
            push_str(&mut hdr, "chromaticities");
            push_str(&mut hdr, "chromaticities");
            hdr.extend_from_slice(&32u32.to_le_bytes());
            for v in CHROMATICITIES_REC709_D65 {
                hdr.extend_from_slice(&v.to_le_bytes());
            }
        }
        attr_byte(&mut hdr, "compression", "compression", COMPRESSION_NONE);
        attr_box2i(&mut hdr, "dataWindow", w as i32 - 1, h as i32 - 1);
        attr_box2i(&mut hdr, "displayWindow", w as i32 - 1, h as i32 - 1);
        attr_byte(&mut hdr, "lineOrder", "lineOrder", 0);
        attr_float(&mut hdr, "pixelAspectRatio", 1.0);
        attr_v2f(&mut hdr, "screenWindowCenter", 0.0, 0.0);
        attr_float(&mut hdr, "screenWindowWidth", 1.0);
        // 闭集外 UE 属性（strip-and-log 对象）。
        attr_string(&mut hdr, "unreal/jobDate", "2026-08-15");
        push_str(&mut hdr, "unreal/stats/memory/peakUsedPhysicalMB");
        push_str(&mut hdr, "int");
        hdr.extend_from_slice(&4u32.to_le_bytes());
        hdr.extend_from_slice(&12345i32.to_le_bytes());
        hdr.push(0);
        let mut out = Vec::new();
        out.extend_from_slice(&EXR_MAGIC);
        out.extend_from_slice(&EXR_VERSION);
        out.extend_from_slice(&hdr);
        let table_pos = out.len();
        out.resize(out.len() + h as usize * 8, 0);
        let mut offsets = Vec::new();
        for y in 0..h {
            offsets.push(out.len() as u64);
            out.extend_from_slice(&(y as i32).to_le_bytes());
            out.extend_from_slice(&(w * 4 * 2).to_le_bytes());
            for (ci, _name) in ["A", "B", "G", "R"].iter().enumerate() {
                for x in 0..w {
                    // fp16 位模式：A=1.0，B/G/R = 闭式探针值（精确可表示面）。
                    let half_bits = match ci {
                        0 => 0x3c00u16,                          // A = 1.0
                        1 => 0x3800u16,                          // B = 0.5
                        2 => 0x3400u16,                          // G = 0.25
                        _ => (0x3c00u16).wrapping_add(x as u16), // R = 1.0 + x ulp
                    };
                    out.extend_from_slice(&half_bits.to_le_bytes());
                }
            }
        }
        for (i, off) in offsets.iter().enumerate() {
            out[table_pos + i * 8..table_pos + i * 8 + 8].copy_from_slice(&off.to_le_bytes());
        }
        out
    }

    //@ spec: RXS-0385
    // ue5 帧 strip-and-log：闭集外属性剥离登记（属性名/类型/长度在录）、alpha
    // 通道剥离登记、fp16 → f32 精确提升、chromaticities 位级闭集互证。
    #[test]
    fn ue5_strip_and_log_read() {
        let bytes = make_ue5_like_frame(4, 2, true);
        let dec = decode_exr(&bytes, ExrSourceEnd::Ue5).unwrap();
        assert_eq!(dec.width, 4);
        assert_eq!(dec.height, 2);
        assert_eq!(dec.layout, ExrChannelLayout::Rgb);
        assert_eq!(dec.source_bit_depth, ExrBitDepth::Float16);
        assert!(dec.metadata.is_none(), "ue5 帧无 rurix:* 元数据");
        // 剥离登记：unreal/* 两件 + alpha 通道一件。
        assert_eq!(dec.stripped.len(), 3);
        assert!(
            dec.stripped
                .iter()
                .any(|s| s.name == "unreal/jobDate" && s.reason == "ue5-strip-and-log")
        );
        assert!(
            dec.stripped
                .iter()
                .any(|s| s.name == "unreal/stats/memory/peakUsedPhysicalMB" && s.value_len == 4)
        );
        assert!(
            dec.stripped
                .iter()
                .any(|s| s.name == "A" && s.reason == "alpha-channel-strip")
        );
        // fp16 提升值核验（B=0.5 / G=0.25 / R=1.0+x ulp）。
        for x in 0..4usize {
            let (r, g, b) = (
                dec.pixels[x * 3],
                dec.pixels[x * 3 + 1],
                dec.pixels[x * 3 + 2],
            );
            assert_eq!(g, 0.25);
            assert_eq!(b, 0.5);
            assert_eq!(r, half_to_f32(0x3c00u16.wrapping_add(x as u16)));
        }
    }

    //@ spec: RXS-0385
    // chromaticities 缺失（ue5 帧）fail-closed；rurix strict 白名单外属性拒绝；
    // ue5 帧 rurix:* 冒充拒绝；截断帧 InvalidExr。
    #[test]
    fn per_end_policy_enforced() {
        // ue5 帧 chromaticities 缺失 → fail-closed。
        let bytes = make_ue5_like_frame(2, 2, false);
        assert!(matches!(
            decode_exr(&bytes, ExrSourceEnd::Ue5),
            Err(ImageError::MetadataViolation(_))
        ));
        // rurix strict：ue5 形态帧（含 unreal/* 白名单外属性）按 rurix 读 → 拒。
        let bytes = make_ue5_like_frame(2, 2, true);
        assert!(matches!(
            decode_exr(&bytes, ExrSourceEnd::Rurix),
            Err(ImageError::MetadataViolation(_))
        ));
        // ue5 帧内嵌 rurix:* 属性 → 冒充拒。
        let img = probe_frame(2, 2);
        let bytes = encode_exr(&img).unwrap();
        assert!(matches!(
            decode_exr(&bytes, ExrSourceEnd::Ue5),
            Err(ImageError::MetadataViolation(_))
        ));
        // 截断（像素体少一字节）→ InvalidExr。
        let mut short = encode_exr(&img).unwrap();
        short.truncate(short.len() - 1);
        assert!(matches!(
            decode_exr(&short, ExrSourceEnd::Rurix),
            Err(ImageError::InvalidExr(_))
        ));
    }
}
