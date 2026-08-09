//! G8.5b M25 UpscalerInputAbi v1 —— 十项输入闭集 + canonical bytes + SHA-256。
//!
//! 设计锚:`G8.5_RENDERING_COMPLETION_DESIGN.md` §4;SHA-256 复用 `rurix-pkg`
//! (与 M31/RXS-0306 同源);canonical 编码沿 RXS-0305 CanonW 律。
//!
//! 装配期核验:缺任一 required 输入或调用方/backend ABI hash 不等 → 确定性
//! [`AssembleError`](fail-closed,禁静默补零)。

use rurix_pkg::sha256;

use crate::temporal::image::ImageF32;
use crate::temporal::upscale::{UpscaleBackend, UpscaleInputs, UpscaleInputsExt};

/// ABI 版本前缀(进 canonical bytes / hash)。
pub const ABI_V1_PREFIX: &[u8] = b"rurix.upscaler-input-abi.v1\0";

/// 十项闭集固定序(字段集合机验锚;改序/增删必变 hash)。
pub const ABI_SLOT_NAMES: [&str; 10] = [
    "color",
    "depth",
    "motion",
    "exposure",
    "jitter",
    "render_extent",
    "output_extent",
    "reset",
    "reactive",
    "transparent",
];

/// 单项 ABI 描述符字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbiSlotDesc {
    pub name: &'static str,
    /// 资源类别:`image_f32` / `scalar_f32` / `scalar_f32x2` / `extent_u32x2` / `bool`。
    pub resource_kind: &'static str,
    /// 通道数或标量类型标签。
    pub channels_or_type: &'static str,
    /// extent 域:`render` / `output` / `none`。
    pub extent_domain: &'static str,
    pub required: bool,
    pub semantic_ref: &'static str,
}

/// UpscalerInputAbi v1 描述符闭集。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpscalerInputAbi {
    pub version: u32,
    pub slots: [AbiSlotDesc; 10],
}

impl UpscalerInputAbi {
    /// 冻结 v1 十项描述符。
    pub fn v1() -> Self {
        Self {
            version: 1,
            slots: [
                AbiSlotDesc {
                    name: "color",
                    resource_kind: "image_f32",
                    channels_or_type: "3",
                    extent_domain: "render",
                    required: true,
                    semantic_ref: "RFC-0016 §4.0-3 color RGB pre-exposure",
                },
                AbiSlotDesc {
                    name: "depth",
                    resource_kind: "image_f32",
                    channels_or_type: "1",
                    extent_domain: "render",
                    required: true,
                    semantic_ref: "RFC-0016 §4.0-3 depth history validate",
                },
                AbiSlotDesc {
                    name: "motion",
                    resource_kind: "image_f32",
                    channels_or_type: "2",
                    extent_domain: "render",
                    required: true,
                    semantic_ref: "RFC-0016 §4.0-3 mv uv delta",
                },
                AbiSlotDesc {
                    name: "exposure",
                    resource_kind: "scalar_f32",
                    channels_or_type: "f32",
                    extent_domain: "none",
                    required: true,
                    semantic_ref: "RFC-0016 §4.0-3 exposure >0 display-domain",
                },
                AbiSlotDesc {
                    name: "jitter",
                    resource_kind: "scalar_f32x2",
                    channels_or_type: "f32x2",
                    extent_domain: "none",
                    required: true,
                    semantic_ref: "temporal::common::jitter_sequence input-px",
                },
                AbiSlotDesc {
                    name: "render_extent",
                    resource_kind: "extent_u32x2",
                    channels_or_type: "u32x2",
                    extent_domain: "render",
                    required: true,
                    semantic_ref: "G8.5b M25 explicit render extent(=color size)",
                },
                AbiSlotDesc {
                    name: "output_extent",
                    resource_kind: "extent_u32x2",
                    channels_or_type: "u32x2",
                    extent_domain: "output",
                    required: true,
                    semantic_ref: "RFC-0016 §4.0-3 UpscaleInputs.output_size",
                },
                AbiSlotDesc {
                    name: "reset",
                    resource_kind: "bool",
                    channels_or_type: "bool",
                    extent_domain: "none",
                    required: true,
                    semantic_ref: "RFC-0016 §4.0-3 history discard",
                },
                AbiSlotDesc {
                    name: "reactive",
                    resource_kind: "image_f32",
                    channels_or_type: "1",
                    extent_domain: "render",
                    required: false,
                    semantic_ref: "报告7 §2.3 reactive mask optional",
                },
                AbiSlotDesc {
                    name: "transparent",
                    resource_kind: "image_f32",
                    channels_or_type: "1",
                    extent_domain: "render",
                    required: false,
                    semantic_ref: "RFC-0019 §4.6.3 transparent provenance(UpscaleInputsExt)",
                },
            ],
        }
    }

    /// CanonW 规范字节(RXS-0305 律:u32 LE + length-prefix 字符串)。
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut w = CanonW::new();
        w.bytes(ABI_V1_PREFIX);
        w.u32v(self.version);
        w.u32v(self.slots.len() as u32);
        for s in &self.slots {
            w.strv(s.name);
            w.strv(s.resource_kind);
            w.strv(s.channels_or_type);
            w.strv(s.extent_domain);
            w.u32v(u32::from(s.required));
            w.strv(s.semantic_ref);
        }
        w.buf
    }

    pub fn hash(&self) -> [u8; 32] {
        sha256::digest(&self.canonical_bytes())
    }

    pub fn hash_hex(&self) -> String {
        sha256::hex(&self.hash())
    }

    /// 白盒扰动:改某一槽 required → 新 hash(敏感度测试用)。
    pub fn with_required_flipped(&self, slot: &str) -> Self {
        let mut out = self.clone();
        for s in &mut out.slots {
            if s.name == slot {
                s.required = !s.required;
                break;
            }
        }
        out
    }

    /// 白盒扰动:改某一槽 channels_or_type 标签。
    pub fn with_channels_tampered(&self, slot: &str, new_tag: &'static str) -> Self {
        let mut out = self.clone();
        for s in &mut out.slots {
            if s.name == slot {
                s.channels_or_type = new_tag;
                break;
            }
        }
        out
    }
}

struct CanonW {
    buf: Vec<u8>,
}

impl CanonW {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }
    fn u32v(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn strv(&mut self, s: &str) {
        self.u32v(s.len() as u32);
        self.buf.extend_from_slice(s.as_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }
}

/// 装配失败(fail-closed)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssembleError {
    MissingRequired(&'static str),
    HashMismatch { caller: String, backend: String },
    Shape(&'static str),
}

impl std::fmt::Display for AssembleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequired(s) => write!(f, "missing required ABI input: {s}"),
            Self::HashMismatch { caller, backend } => {
                write!(f, "ABI hash mismatch caller={caller} backend={backend}")
            }
            Self::Shape(s) => write!(f, "ABI shape: {s}"),
        }
    }
}

/// 调用侧绑定集(Option = 可摘除以测 fail-closed)。
#[derive(Debug, Clone, Copy)]
pub struct UpscaleBindSet<'a> {
    pub caller_abi_hash: [u8; 32],
    pub color: Option<&'a ImageF32>,
    pub depth: Option<&'a ImageF32>,
    pub motion: Option<&'a ImageF32>,
    pub exposure: Option<f32>,
    pub jitter: Option<[f32; 2]>,
    pub render_extent: Option<(u32, u32)>,
    pub output_extent: Option<(u32, u32)>,
    pub reset: Option<bool>,
    pub reactive: Option<&'a ImageF32>,
    pub transparent: Option<&'a ImageF32>,
    pub frame_index: u32,
}

/// 装配后的资源身份清单(evidence 逐项消费见证)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumeReport {
    pub slots: Vec<&'static str>,
}

impl ConsumeReport {
    pub fn contains_all_required(&self) -> bool {
        let abi = UpscalerInputAbi::v1();
        abi.slots
            .iter()
            .filter(|s| s.required)
            .all(|s| self.slots.iter().any(|&n| n == s.name))
    }

    pub fn contains_named(&self, names: &[&str]) -> bool {
        names.iter().all(|n| self.slots.iter().any(|s| s == n))
    }
}

/// 装配 UpscaleInputs(+Ext);缺 required 或 hash 不等 → Err。
pub fn assemble<'a>(
    bind: &'a UpscaleBindSet<'a>,
    backend_abi_hash: [u8; 32],
) -> Result<(UpscaleInputs<'a>, UpscaleInputsExt<'a>), AssembleError> {
    if bind.caller_abi_hash != backend_abi_hash {
        return Err(AssembleError::HashMismatch {
            caller: sha256::hex(&bind.caller_abi_hash),
            backend: sha256::hex(&backend_abi_hash),
        });
    }
    let color = bind.color.ok_or(AssembleError::MissingRequired("color"))?;
    let depth = bind.depth.ok_or(AssembleError::MissingRequired("depth"))?;
    let motion = bind
        .motion
        .ok_or(AssembleError::MissingRequired("motion"))?;
    let exposure = bind
        .exposure
        .ok_or(AssembleError::MissingRequired("exposure"))?;
    let jitter = bind
        .jitter
        .ok_or(AssembleError::MissingRequired("jitter"))?;
    let render_extent = bind
        .render_extent
        .ok_or(AssembleError::MissingRequired("render_extent"))?;
    let output_extent = bind
        .output_extent
        .ok_or(AssembleError::MissingRequired("output_extent"))?;
    let reset = bind.reset.ok_or(AssembleError::MissingRequired("reset"))?;

    if (color.w, color.h) != render_extent {
        return Err(AssembleError::Shape("render_extent != color size"));
    }
    if depth.c != 1 || (depth.w, depth.h) != render_extent {
        return Err(AssembleError::Shape("depth must be 1ch @ render_extent"));
    }
    if motion.c != 2 || (motion.w, motion.h) != render_extent {
        return Err(AssembleError::Shape("motion must be 2ch @ render_extent"));
    }
    if let Some(r) = bind.reactive {
        if r.c != 1 || (r.w, r.h) != render_extent {
            return Err(AssembleError::Shape("reactive must be 1ch @ render_extent"));
        }
    }
    if let Some(t) = bind.transparent {
        if t.c != 1 || (t.w, t.h) != render_extent {
            return Err(AssembleError::Shape(
                "transparent must be 1ch @ render_extent",
            ));
        }
    }

    let inputs = UpscaleInputs {
        color,
        depth,
        mv: motion,
        reactive: bind.reactive,
        exposure,
        jitter,
        output_size: output_extent,
        frame_index: bind.frame_index,
        reset,
    };
    let ext = UpscaleInputsExt {
        transparent: bind.transparent,
    };
    // 触发冻结形状校验(违例 panic = 装配契约违例)。
    let _ = inputs.validated();
    Ok((inputs, ext))
}

/// 经 ABI 装配跑 backend;`upscale_ext` 路径;返回输出 + 消费报告。
pub fn run_via_abi<B: UpscaleBackend>(
    backend: &mut B,
    bind: &UpscaleBindSet<'_>,
) -> Result<(ImageF32, ConsumeReport), AssembleError> {
    let (inputs, ext) = assemble(bind, backend.abi_hash())?;
    let out = backend.upscale_ext(&inputs, &ext);
    let report = backend.consumed_slots();
    Ok((out, report))
}

/// 合成固定序列 fixture(host/device 共用种子;golden 冻结锚)。
pub fn synthetic_frame(frame: u32, iw: u32, ih: u32) -> SyntheticFrame {
    let color = ImageF32::from_fn(iw, ih, 3, |x, y, ch| {
        let t = (frame as f32) * 0.07;
        let u = x as f32 / iw as f32;
        let v = y as f32 / ih as f32;
        match ch {
            0 => (0.35 + 0.45 * (u * 6.0 + t).sin()).clamp(0.0, 1.0),
            1 => (0.30 + 0.40 * (v * 5.0 - t).cos()).clamp(0.0, 1.0),
            _ => (0.25 + 0.35 * ((u + v) * 4.0 + t * 0.5).sin()).clamp(0.0, 1.0),
        }
    });
    let depth = ImageF32::from_fn(iw, ih, 1, |x, y, _| {
        0.2 + 0.6 * ((x + y + frame) as f32 % 17.0) / 16.0
    });
    let motion = ImageF32::from_fn(iw, ih, 2, |x, y, ch| {
        let s = if ch == 0 { 0.002 } else { -0.0015 };
        s * (((x + 3 * y + frame) % 11) as f32 - 5.0)
    });
    let reactive = ImageF32::from_fn(iw, ih, 1, |x, y, _| {
        if (x + y + frame) % 13 == 0 { 0.85 } else { 0.0 }
    });
    let transparent = ImageF32::from_fn(iw, ih, 1, |x, y, _| {
        if x > iw / 2 && (y + frame) % 9 == 0 {
            0.6
        } else {
            0.0
        }
    });
    let jitter = [
        ((frame % 8) as f32) * 0.125 - 0.4375,
        (((frame * 3) % 8) as f32) * 0.125 - 0.4375,
    ];
    SyntheticFrame {
        color,
        depth,
        motion,
        reactive,
        transparent,
        exposure: 1.0 + (frame % 3) as f32 * 0.05,
        jitter,
        reset: frame == 0,
        frame_index: frame,
    }
}

#[derive(Debug, Clone)]
pub struct SyntheticFrame {
    pub color: ImageF32,
    pub depth: ImageF32,
    pub motion: ImageF32,
    pub reactive: ImageF32,
    pub transparent: ImageF32,
    pub exposure: f32,
    pub jitter: [f32; 2],
    pub reset: bool,
    pub frame_index: u32,
}

impl SyntheticFrame {
    pub fn bind_set<'a>(&'a self, ow: u32, oh: u32, abi_hash: [u8; 32]) -> UpscaleBindSet<'a> {
        UpscaleBindSet {
            caller_abi_hash: abi_hash,
            color: Some(&self.color),
            depth: Some(&self.depth),
            motion: Some(&self.motion),
            exposure: Some(self.exposure),
            jitter: Some(self.jitter),
            render_extent: Some((self.color.w, self.color.h)),
            output_extent: Some((ow, oh)),
            reset: Some(self.reset),
            reactive: Some(&self.reactive),
            transparent: Some(&self.transparent),
            frame_index: self.frame_index,
        }
    }
}

/// 输出序列 digest(f32 位型拼接 SHA-256)。
pub fn sequence_digest(frames: &[ImageF32]) -> String {
    let mut bytes = Vec::new();
    for img in frames {
        bytes.extend_from_slice(&(img.w).to_le_bytes());
        bytes.extend_from_slice(&(img.h).to_le_bytes());
        bytes.extend_from_slice(&(img.c).to_le_bytes());
        for v in &img.data {
            bytes.extend_from_slice(&v.to_bits().to_le_bytes());
        }
    }
    sha256::hex_digest(&bytes)
}

/// 反假绿:透传 color 最近邻上采样(故意 ignore 其它槽)——判据必须判红。
#[derive(Debug, Default)]
pub struct NoOpPassthroughUpscaler {
    consumed: Vec<&'static str>,
}

impl UpscaleBackend for NoOpPassthroughUpscaler {
    fn name(&self) -> &str {
        "noop_passthrough"
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        let (iw, ih, ow, oh) = inputs.validated();
        let _ = (iw, ih);
        // 故意只读 color,忽略 depth/mv/reactive/exposure/jitter。
        let mut out = ImageF32::new(ow, oh, 3);
        for y in 0..oh {
            for x in 0..ow {
                let u = (x as f32 + 0.5) / ow as f32;
                let v = (y as f32 + 0.5) / oh as f32;
                out.set_pixel3(
                    x,
                    y,
                    [
                        inputs.color.sample_nearest(u, v, 0),
                        inputs.color.sample_nearest(u, v, 1),
                        inputs.color.sample_nearest(u, v, 2),
                    ],
                );
            }
        }
        self.consumed = vec!["color", "render_extent", "output_extent"];
        out
    }

    fn reset_history(&mut self) {}

    fn consumed_slots(&self) -> ConsumeReport {
        ConsumeReport {
            slots: self.consumed.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::cas::CasUpscaler;
    use crate::temporal::tsr::TsrUpscaler;

    #[test]
    fn ten_slots_closed_set() {
        let abi = UpscalerInputAbi::v1();
        let names: Vec<_> = abi.slots.iter().map(|s| s.name).collect();
        assert_eq!(names, ABI_SLOT_NAMES);
    }

    #[test]
    fn hash_stable_twice() {
        let a = UpscalerInputAbi::v1();
        let b = UpscalerInputAbi::v1();
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn hash_sensitive_to_required_and_layout() {
        let base = UpscalerInputAbi::v1();
        let h0 = base.hash();
        assert_ne!(base.with_required_flipped("reactive").hash(), h0);
        assert_ne!(base.with_channels_tampered("color", "4").hash(), h0);
    }

    #[test]
    fn missing_required_fail_closed() {
        let abi = UpscalerInputAbi::v1();
        let h = abi.hash();
        let frame = synthetic_frame(0, 8, 8);
        let mut bind = frame.bind_set(16, 16, h);
        bind.depth = None;
        let err = assemble(&bind, h).unwrap_err();
        assert!(matches!(err, AssembleError::MissingRequired("depth")));
    }

    #[test]
    fn hash_mismatch_fail_closed() {
        let abi = UpscalerInputAbi::v1();
        let frame = synthetic_frame(0, 8, 8);
        let mut bad = [0u8; 32];
        bad[0] = 0xab;
        let bind = frame.bind_set(16, 16, bad);
        let err = assemble(&bind, abi.hash()).unwrap_err();
        assert!(matches!(err, AssembleError::HashMismatch { .. }));
    }

    #[test]
    fn dual_backend_same_caller_abi() {
        let abi = UpscalerInputAbi::v1();
        let h = abi.hash();
        let mut tsr = TsrUpscaler::default();
        let mut cas = CasUpscaler::default();
        assert_eq!(tsr.abi_hash(), h);
        assert_eq!(cas.abi_hash(), h);
        let frame = synthetic_frame(1, 8, 8);
        let bind = frame.bind_set(16, 16, h);
        let (o1, r1) = run_via_abi(&mut tsr, &bind).unwrap();
        let (o2, r2) = run_via_abi(&mut cas, &bind).unwrap();
        assert_eq!((o1.w, o1.h, o1.c), (16, 16, 3));
        assert_eq!((o2.w, o2.h, o2.c), (16, 16, 3));
        assert!(r1.contains_all_required());
        assert!(r2.contains_all_required());
        assert!(o1.data.iter().all(|v| v.is_finite()));
        assert!(o2.data.iter().all(|v| v.is_finite()));
        // 非 no-op:输出不得等于任一输入透传(尺寸不同已保证;再验≠最近邻 color)
        let mut noop = NoOpPassthroughUpscaler::default();
        let (onoop, rn) = run_via_abi(&mut noop, &bind).unwrap();
        assert!(!rn.contains_all_required());
        assert_ne!(sequence_digest(&[o1]), sequence_digest(&[onoop.clone()]));
        assert_ne!(sequence_digest(&[o2]), sequence_digest(&[onoop]));
    }
}
