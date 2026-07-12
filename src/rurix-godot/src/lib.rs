//! `rurix-godot` exposes the opt-in C ABI used by the Godot D3D12 Forward+
//! acceleration experiment.
//!
//! The first implementation deliberately records resources and pass telemetry
//! without taking ownership of Godot's D3D12 objects. Concrete Rurix DXIL pass
//! replacement can plug into this ABI while the original Godot path remains the
//! fallback whenever validation or device support fails.

use core::ffi::c_void;
use core::slice;

pub const RXGD_ABI_VERSION: u32 = 1;

pub const RXGD_STATUS_OK: i32 = 0;
pub const RXGD_STATUS_FALLBACK: i32 = 1;
pub const RXGD_E_NULL: i32 = -1;
pub const RXGD_E_ABI: i32 = -2;
pub const RXGD_E_UNSUPPORTED: i32 = -3;
pub const RXGD_E_INVALID_ARGUMENT: i32 = -4;

pub const RXGD_BACKEND_D3D12: u32 = 1;
pub const RXGD_RENDER_METHOD_FORWARD_PLUS: u32 = 1;

pub const RXGD_RESOURCE_TEXTURE: u32 = 1;
pub const RXGD_RESOURCE_BUFFER: u32 = 2;

pub const RXGD_PASS_CLUSTER_STORE: u32 = 1;
pub const RXGD_PASS_SSAO_BLUR: u32 = 2;
pub const RXGD_PASS_SSIL_BLUR: u32 = 3;
pub const RXGD_PASS_LUMINANCE_REDUCTION: u32 = 4;
pub const RXGD_PASS_TONEMAP: u32 = 5;
pub const RXGD_PASS_TAA_RESOLVE: u32 = 6;
pub const RXGD_PASS_PARTICLES_COPY: u32 = 7;
pub const RXGD_PASS_GPU_CULLING: u32 = 8;
pub const RXGD_PASS_INDIRECT_ARGS: u32 = 9;
pub const RXGD_PASS_FUSED_POST_CHAIN: u32 = 10;

const MAX_RESOURCES_PER_PASS: u64 = 64;
const MAX_PUSH_CONSTANT_BYTES: u64 = 4096;
const RXGD_CAP_SHADER_INT64: u32 = 1 << 0;
/// GRX-009 segment 4b reserved capability flag carried in `RxGdCaps.flags`
/// (ABI v1, no struct layout change). The Godot side sets it only when the
/// per-pass `.../dispatch_bringup` opt-in setting is enabled. It advertises
/// that the caller opted into the gated dispatch bring-up path; it never by
/// itself makes the bridge return `RXGD_STATUS_OK`.
pub const RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP: u32 = 1 << 1;
/// GRX-009 segment 4d harness-only "record arm" capability flag carried in
/// `RxGdCaps.flags` (ABI v1, no struct layout change). It is set ONLY by the
/// explicit, test-only bridge D3D12 dispatch recording harness alongside real
/// D3D12 device/queue and resource handles; the Godot module never sets it.
///
/// It only has effect when the bridge is built with the `d3d12-recording-shim`
/// feature. Without that feature the bit is ignored and the luminance pass keeps
/// returning `RXGD_STATUS_FALLBACK`. Even with the feature, recording still
/// requires the full dispatch eligibility gate (opt-in flag, 64-bit integer
/// capability, non-null native handles, and a compiled package whose
/// layout/digests match the offline evidence) plus the tracked artifact bytes
/// hashing to the offline digests before `rxgd_record_pass` may return OK.
pub const RXGD_CAP_LUMINANCE_DISPATCH_RECORD: u32 = 1 << 2;
/// GRX-009 segment 4h opt-in "real pass" capability flag carried in
/// `RxGdCaps.flags` (ABI v1, no struct layout change). The Godot side sets it
/// only when the default-false per-pass `.../dispatch_real_pass` bring-up
/// opt-in setting is enabled (patch 0009); the default Godot config never
/// sets it. It arms the gated REAL luminance pass attempt: the bridge runs
/// the full runtime binding preflight, the dispatch eligibility gate, the
/// segment 4h kernel-binding-kind conformance check, and the segment 4i
/// math-pyramid-parity check, in that order, and returns
/// `RXGD_STATUS_FALLBACK` with a recorded fallback reason (plus a
/// once-per-session machine-readable `RXGD_REAL_PASS_BLOCKED` diagnostic
/// naming the FIRST missing prerequisite) unless every check passes AND a
/// runtime-mappable real dispatch path is linked. With the tracked stage
/// A2/A3 texture-capable hlsl_bridge artifact the kernel declares per-slot
/// `texture2d`/`rwtexture2d` binding kinds matching the Texture2D handles
/// the Godot runtime provides, and level-0 math parity is CPU-proven
/// (`math_parity_evidence.json`), so all software gates can pass; the real
/// dispatch itself is only linked under the `d3d12-recording-shim` feature
/// (stage A5) — the shipping feature-off bridge still returns
/// `RXGD_STATUS_FALLBACK` with `real_dispatch_path_not_linked`. When both
/// this flag and `RXGD_CAP_LUMINANCE_DISPATCH_RECORD` are set, the
/// real-pass arm takes precedence (the 4d recording harness never sets this
/// flag, so the combination does not occur in tracked harnesses).
pub const RXGD_CAP_LUMINANCE_REAL_PASS: u32 = 1 << 3;
/// GRX-010 opt-in "real pass" capability flag for the tonemap pass, carried
/// in `RxGdCaps.flags` (ABI v1, no struct layout change). The Godot side
/// sets it only when the default-false per-pass
/// `rendering/rurix_accel/passes/tonemap/dispatch_real_pass` opt-in setting
/// is enabled (patch 0013); the default Godot config never sets it. It
/// arms the gated REAL tonemap pass attempt: the bridge runs the full
/// runtime binding preflight, the dispatch eligibility gate, the per-slot
/// kernel-binding-kind conformance check, and the math-parity check, in
/// that order, and returns `RXGD_STATUS_FALLBACK` with a recorded fallback
/// reason (plus a once-per-session machine-readable
/// `RXGD_TONEMAP_REAL_PASS_BLOCKED` diagnostic naming the FIRST missing
/// prerequisite) unless every check passes AND a runtime-mappable real
/// dispatch path is linked. The real dispatch itself is only linked under
/// the `d3d12-recording-shim` feature — the shipping feature-off bridge
/// always fails closed with `real_dispatch_path_not_linked`.
pub const RXGD_CAP_TONEMAP_REAL_PASS: u32 = 1 << 4;
/// GRX-011 opt-in "real pass" capability flag for the SSAO blur pass,
/// carried in `RxGdCaps.flags` (ABI v1, no struct layout change). The Godot
/// side would set it only when a default-false per-pass ssao_blur real-pass
/// opt-in setting is enabled; the default Godot config (and a future
/// ssao_blur gate patch, a 0002-level gate without resource bindings that
/// is not yet tracked — patches 0012/0013 are the GRX-010 tonemap runtime
/// binding and real-pass opt-in slices) never sets it. It arms the gated
/// REAL SSAO blur pass attempt: the bridge runs the
/// full runtime binding preflight, the dispatch eligibility gate, the
/// per-slot kernel-binding-kind conformance check, and the math-parity
/// check, in that order, and returns `RXGD_STATUS_FALLBACK` with a recorded
/// fallback reason (plus a once-per-session machine-readable
/// `RXGD_SSAO_BLUR_REAL_PASS_BLOCKED` diagnostic naming the FIRST missing
/// prerequisite) unless every check passes AND a runtime-mappable real
/// dispatch path is linked. The real dispatch itself is only linked under
/// the `d3d12-recording-shim` feature — the shipping feature-off bridge
/// always fails closed with `real_dispatch_path_not_linked`.
pub const RXGD_CAP_SSAO_BLUR_REAL_PASS: u32 = 1 << 5;
/// GRX-012 opt-in "real pass" capability flag for the TAA resolve pass,
/// carried in `RxGdCaps.flags` (ABI v1, no struct layout change). The Godot
/// side would set it only when a default-false per-pass taa_resolve real-pass
/// opt-in setting is enabled (deferred patch 0019); the default Godot config
/// never sets it. It arms the gated REAL TAA resolve pass attempt: the bridge
/// runs the full runtime binding preflight, the dispatch eligibility gate, the
/// per-slot kernel-binding-kind conformance check, and the math-parity check,
/// in that order, and returns `RXGD_STATUS_FALLBACK` with a recorded fallback
/// reason (plus a once-per-session machine-readable
/// `RXGD_TAA_REAL_PASS_BLOCKED` diagnostic naming the FIRST missing
/// prerequisite) unless every check passes AND a runtime-mappable real dispatch
/// path is linked. The real dispatch itself is only linked under the
/// `d3d12-recording-shim` feature — the shipping feature-off bridge always
/// fails closed with `real_dispatch_path_not_linked`.
pub const RXGD_CAP_TAA_RESOLVE_REAL_PASS: u32 = 1 << 6;
const LUMINANCE_RESOURCE_COUNT: u64 = 2;
const LUMINANCE_ROOT_CONSTANT_BYTES: u64 = 28;
/// GRX-009 stage A3: the per-slot resource binding kinds the tracked
/// luminance kernel declares. The tracked canonical package is the
/// texture-capable hlsl_bridge workaround artifact
/// (`artifacts/hlsl_bridge/luminance_reduce_level.{hlsl,dxil}` compiled by
/// DXC `cs_6_0` and validated by `dxv`, copied to the canonical
/// `artifacts/luminance_reduction.{dxil,rts0.bin,_descriptor_layout.json}`
/// paths under the owner-approved `hlsl_bridge_workaround` provenance
/// policy). The kernel binds `src_luminance` as `Texture2D<float>` (SRV t0,
/// slot 0 → `"texture2d"`) and `dst_luminance` as `RWTexture2D<float>`
/// (UAV u0, slot 1 → `"rwtexture2d"`), matching the Texture2D
/// `ID3D12Resource*` handles the Godot runtime provides (segment 4e).
///
/// Buffer resources (`RXGD_RESOURCE_BUFFER` → `"raw_buffer_view"`) no
/// longer conform to the tracked kernel at any slot; the historical
/// raw-buffer fixture is retained at `artifacts/raw_buffer_historical/`
/// only as evidence.
pub const LUMINANCE_KERNEL_RESOURCE_BINDING_KINDS: [&str; 2] = ["texture2d", "rwtexture2d"];

/// GRX-009 stage A3: the binding kind of the kernel's slot-0 SRV
/// (`src_luminance = Texture2D<float>` at t0). Binding-kind conformance is
/// checked per slot against [`LUMINANCE_KERNEL_RESOURCE_BINDING_KINDS`]
/// (slot 1, `dst_luminance`, is `"rwtexture2d"`); this scalar constant is
/// retained for the `RXGD_REAL_PASS_BLOCKED` diagnostic line and external
/// probes.
pub const LUMINANCE_KERNEL_RESOURCE_BINDING_KIND: &str = "texture2d";

/// GRX-009 stage A4: math-parity status of the tracked hlsl_bridge
/// luminance kernel. The level-reduction math (8×8 tile, arithmetic mean,
/// partial-tile divisor matching Godot level 0) is proven equivalent to the
/// CPU reference in `math_parity_evidence.json`
/// (`status=pending_gpu_dispatch`: every case has a CPU-expected grid; the
/// GPU-observed side is measured by a real dispatch). Level-0 parity is
/// therefore CPU-proven and the real-pass math gate no longer blocks the
/// level-0 dispatch; full pyramid/EMA/WRITE_LUMINANCE parity is still
/// gated on measured GPU results and the multi-level continuation round.
pub const LUMINANCE_KERNEL_MATH_PARITY_STATUS: &str =
    "level0_cpu_reference_proven_pending_gpu_dispatch";

/// SHA-256 digests of the GRX-009 canonical offline luminance package. The
/// canonical `artifacts/` paths carry the texture-capable hlsl_bridge
/// workaround artifact (DXC-compiled `cs_6_0` container validated by `dxv`,
/// Rurix-owned RTS0 root signature from
/// `rurixc::binding_layout::serialize_rts0`, and the rurixc-emitted
/// `src/lib_texture.rx` descriptor layout declaring per-slot
/// `texture2d`/`rwtexture2d` binding kinds), per
/// the owner-approved `hlsl_bridge_workaround` provenance policy
/// (`texture_artifact_provenance_policy.json`). The dispatch bring-up
/// eligibility check matches the compiled package identity against these
/// baked evidence digests; a mismatch means the runtime binding does not
/// correspond to the compiled package and must fall back.
const LUMINANCE_OFFLINE_DXIL_SHA256: &str =
    "14761af20456557a019086d51185cdc5375acf89bf9ac208927e703d265c3d2e";
const LUMINANCE_OFFLINE_ROOT_SIGNATURE_SHA256: &str =
    "f08794f9886e1ebc4c905e3006732e572ec913a75255c5b488cf4877a1391f03";
const LUMINANCE_OFFLINE_DESCRIPTOR_LAYOUT_SHA256: &str =
    "0067fcb119fe1c364263839e529112965af300a45583152225502391f9512749";

/// GRX-010: the per-slot resource binding kinds the tracked tonemap kernel
/// declares. The tracked canonical package is the texture-capable
/// hlsl_bridge workaround artifact
/// (`spike/godot-rurix/passes/tonemap/artifacts/hlsl_bridge/tonemap_apply.hlsl`
/// compiled by DXC `cs_6_0` and validated by `dxv`, published to the
/// canonical `artifacts/tonemap.{dxil,rts0.bin}` /
/// `tonemap_descriptor_layout.json` paths under the owner-approved
/// `hlsl_bridge_workaround` provenance policy). The kernel binds
/// `src_color` as `Texture2D<float4>` (SRV t0, slot 0 → `"texture2d"`) and
/// `dst_color` as `RWTexture2D<float4>` (UAV u0, slot 1 →
/// `"rwtexture2d"`).
pub const TONEMAP_KERNEL_RESOURCE_BINDING_KINDS: [&str; 2] = ["texture2d", "rwtexture2d"];

/// GRX-010: the binding kind of the tonemap kernel's slot-0 SRV
/// (`src_color = Texture2D<float4>` at t0); retained for the
/// `RXGD_TONEMAP_REAL_PASS_BLOCKED` diagnostic line and external probes.
pub const TONEMAP_KERNEL_RESOURCE_BINDING_KIND: &str = "texture2d";

/// GRX-010: math-parity status of the tracked hlsl_bridge tonemap kernel.
/// The LINEAR + linear_to_srgb math subset is proven equivalent to the CPU
/// reference in the pass `math_parity_evidence.json`
/// (`status=pending_gpu_dispatch`: every case has a CPU-expected grid; the
/// GPU-observed side is measured by a real dispatch). Anything other than
/// this exact status fails the real-pass math gate closed.
pub const TONEMAP_KERNEL_MATH_PARITY_STATUS: &str =
    "linear_srgb_cpu_reference_proven_pending_gpu_dispatch";

/// SHA-256 digests of the GRX-010 canonical offline tonemap package
/// (`spike/godot-rurix/passes/tonemap/offline_compile_evidence.json`). The
/// dispatch eligibility check matches the compiled package identity
/// against these baked evidence digests; a mismatch means the runtime
/// binding does not correspond to the compiled package and must fall back.
const TONEMAP_OFFLINE_DXIL_SHA256: &str =
    "4b3d60118523b746622ba3ec01f192f820b9c67b8644a71992a490a13e8aa392";
const TONEMAP_OFFLINE_ROOT_SIGNATURE_SHA256: &str =
    "f08794f9886e1ebc4c905e3006732e572ec913a75255c5b488cf4877a1391f03";
const TONEMAP_OFFLINE_DESCRIPTOR_LAYOUT_SHA256: &str =
    "1777b8b16b1e713c0ca55ce4c3e8fdc607a1e55bd15e6e7555e9fe20bc72a7ae";
const TONEMAP_RESOURCE_COUNT: u64 = 2;
const TONEMAP_ROOT_CONSTANT_BYTES: u64 = 28;

/// GRX-011: the per-slot resource binding kinds the tracked ssao_blur
/// kernel declares. The tracked canonical package is the texture-capable
/// hlsl_bridge workaround artifact
/// (`spike/godot-rurix/passes/ssao_blur/artifacts/hlsl_bridge/ssao_blur_smart.hlsl`
/// compiled by DXC `cs_6_0` and validated by `dxv`, published to the
/// canonical `artifacts/ssao_blur.{dxil,rts0.bin}` /
/// `ssao_blur_descriptor_layout.json` paths under the owner-approved
/// `hlsl_bridge_workaround` provenance policy). The kernel binds
/// `src_ssao` as `Texture2D<float4>` (SRV t0, slot 0 → `"texture2d"`) and
/// `dst_ssao` as `RWTexture2D<float4>` (UAV u0, slot 1 →
/// `"rwtexture2d"`).
pub const SSAO_BLUR_KERNEL_RESOURCE_BINDING_KINDS: [&str; 2] = ["texture2d", "rwtexture2d"];

/// GRX-011: the binding kind of the ssao_blur kernel's slot-0 SRV
/// (`src_ssao = Texture2D<float4>` at t0); retained for the
/// `RXGD_SSAO_BLUR_REAL_PASS_BLOCKED` diagnostic line and external probes.
pub const SSAO_BLUR_KERNEL_RESOURCE_BINDING_KIND: &str = "texture2d";

/// GRX-011: math-parity status of the tracked hlsl_bridge ssao_blur
/// kernel. The MODE_SMART edge-aware cross blur subset (single pass,
/// single slice) is proven equivalent to the CPU reference in the pass
/// `math_parity_evidence.json` (`status=pending_gpu_dispatch`: every case
/// has a CPU-expected grid; the GPU-observed side is measured by a real
/// dispatch). Anything other than this exact status fails the real-pass
/// math gate closed.
pub const SSAO_BLUR_KERNEL_MATH_PARITY_STATUS: &str =
    "smart_blur_cpu_reference_proven_pending_gpu_dispatch";

/// SHA-256 digests of the GRX-011 canonical offline ssao_blur package
/// (`spike/godot-rurix/passes/ssao_blur/offline_compile_evidence.json`).
/// The dispatch eligibility check matches the compiled package identity
/// against these baked evidence digests; a mismatch means the runtime
/// binding does not correspond to the compiled package and must fall back.
const SSAO_BLUR_OFFLINE_DXIL_SHA256: &str =
    "0a730561d08a2b37d26a8bc3f29df893febedb32d55e2e5b5c3d7a84b369ce32";
const SSAO_BLUR_OFFLINE_ROOT_SIGNATURE_SHA256: &str =
    "f08794f9886e1ebc4c905e3006732e572ec913a75255c5b488cf4877a1391f03";
const SSAO_BLUR_OFFLINE_DESCRIPTOR_LAYOUT_SHA256: &str =
    "ba09771d5cc65511972eaa6f5810b9f614756a4ab323cc3d9ea2afdf7b525e2c";
const SSAO_BLUR_RESOURCE_COUNT: u64 = 2;
const SSAO_BLUR_ROOT_CONSTANT_BYTES: u64 = 28;

/// GRX-012: the per-slot resource binding kinds the tracked taa_resolve kernel
/// declares. The tracked canonical package is the texture-capable hlsl_bridge
/// workaround artifact
/// (`spike/godot-rurix/passes/taa_resolve/artifacts/hlsl_bridge/taa_resolve.hlsl`
/// compiled by DXC `cs_6_0` and validated by `dxv`, published to the canonical
/// `artifacts/taa_resolve.{dxil,rts0.bin}` / `taa_resolve_descriptor_layout.json`
/// paths under the owner-approved `hlsl_bridge_workaround` provenance policy).
/// The kernel binds `color_buffer`/`depth_buffer`/`velocity_buffer`/
/// `last_velocity_buffer`/`history_buffer` as `Texture2D` SRVs (t0..t4, each
/// slot → `"texture2d"`) and `output_buffer` as `RWTexture2D<float4>` (UAV u0,
/// slot 5 → `"rwtexture2d"`).
pub const TAA_RESOLVE_KERNEL_RESOURCE_BINDING_KINDS: [&str; 6] = [
    "texture2d",
    "texture2d",
    "texture2d",
    "texture2d",
    "texture2d",
    "rwtexture2d",
];

/// GRX-012: the binding kind of the taa_resolve kernel's slot-0 SRV
/// (`color_buffer = Texture2D<float4>` at t0); retained for the
/// `RXGD_TAA_REAL_PASS_BLOCKED` diagnostic line and external probes.
pub const TAA_RESOLVE_KERNEL_RESOURCE_BINDING_KIND: &str = "texture2d";

/// GRX-012: math-parity status of the tracked hlsl_bridge taa_resolve kernel.
/// The single full-resolution TAA resolve (Spartan-derived; groupshared tile,
/// 3x3 closest-depth velocity, 9-tap Catmull-Rom history, clip_aabb variance
/// clipping, Reinhard-domain blend) is proven equivalent to the CPU reference
/// in the pass `math_parity_evidence.json` (`status=pending_gpu_dispatch`:
/// every case has a CPU-expected grid; the GPU-observed side is measured by a
/// real dispatch). Anything other than this exact status fails the real-pass
/// math gate closed.
pub const TAA_RESOLVE_KERNEL_MATH_PARITY_STATUS: &str =
    "taa_resolve_cpu_reference_proven_pending_gpu_dispatch";

/// SHA-256 digests of the GRX-012 canonical offline taa_resolve package
/// (`spike/godot-rurix/passes/taa_resolve/offline_compile_evidence.json`). The
/// dispatch eligibility check matches the compiled package identity against
/// these baked evidence digests; a mismatch means the runtime binding does not
/// correspond to the compiled package and must fall back.
const TAA_RESOLVE_OFFLINE_DXIL_SHA256: &str =
    "1081b3362153746bd3e6f7407a4093ef13d1e65bc26ad9add55bec465720b5df";
const TAA_RESOLVE_OFFLINE_ROOT_SIGNATURE_SHA256: &str =
    "18fb877cf7a9880adcd23fab2ca78f107b6a16ec63d30a818976473e1ef4a301";
const TAA_RESOLVE_OFFLINE_DESCRIPTOR_LAYOUT_SHA256: &str =
    "8959d2996331655a63697efd55f3361420e177cbd8b6094129b54805b72c4c1d";
const TAA_RESOLVE_RESOURCE_COUNT: u64 = 6;
const TAA_RESOLVE_ROOT_CONSTANT_BYTES: u64 = 28;

/// Fallback reasons for gated passes, mirroring the five-value enum used by
/// the GRX-008 fallback telemetry schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallbackReason {
    CompileFailed,
    ValidationFailed,
    UnsupportedDevice,
    VisualDiffFailed,
    ManualDisabled,
}

impl FallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            FallbackReason::CompileFailed => "compile_failed",
            FallbackReason::ValidationFailed => "validation_failed",
            FallbackReason::UnsupportedDevice => "unsupported_device",
            FallbackReason::VisualDiffFailed => "visual_diff_failed",
            FallbackReason::ManualDisabled => "manual_disabled",
        }
    }
}

/// Identity of the GRX-009 compiled luminance package (DXIL container,
/// root signature, descriptor layout) as seen by the bridge. The tracked
/// canonical package is the **texture-capable hlsl_bridge workaround**
/// artifact (DXC-compiled `cs_6_0` container validated by `dxv`, per-slot
/// `texture2d`/`rwtexture2d` binding kinds, owner-approved
/// `hlsl_bridge_workaround` provenance).
///
/// The bridge does not read the compiled artifacts from disk at runtime; the
/// expected descriptor layout and the offline compile evidence digests are
/// baked in via [`LuminanceDispatchPackage::verified_offline_package`]. The
/// dispatch bring-up eligibility check confirms the package is available and
/// that its layout and digests still match the offline evidence before it may
/// enter the (still closed) dispatch bring-up gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LuminanceDispatchPackage {
    pub available: bool,
    pub resource_count: u64,
    pub root_constant_bytes: u64,
    pub srv_register: u32,
    pub uav_register: u32,
    pub requires_shader_int64: bool,
    pub dxil_sha256: &'static str,
    pub root_signature_sha256: &'static str,
    pub descriptor_layout_sha256: &'static str,
}

impl LuminanceDispatchPackage {
    /// The compiled package identity that matches the tracked GRX-009
    /// offline compile evidence (texture-capable hlsl_bridge workaround
    /// artifact at the canonical `artifacts/` paths).
    pub fn verified_offline_package() -> LuminanceDispatchPackage {
        LuminanceDispatchPackage {
            available: true,
            resource_count: LUMINANCE_RESOURCE_COUNT,
            root_constant_bytes: LUMINANCE_ROOT_CONSTANT_BYTES,
            srv_register: 0,
            uav_register: 0,
            requires_shader_int64: true,
            dxil_sha256: LUMINANCE_OFFLINE_DXIL_SHA256,
            root_signature_sha256: LUMINANCE_OFFLINE_ROOT_SIGNATURE_SHA256,
            descriptor_layout_sha256: LUMINANCE_OFFLINE_DESCRIPTOR_LAYOUT_SHA256,
        }
    }

    /// Verifies the compiled package is present and that its descriptor layout
    /// and artifact digests match the tracked offline compile evidence. An
    /// unavailable package maps to `compile_failed`; any layout or digest
    /// mismatch maps to `validation_failed`.
    fn verify_matches_offline_evidence(&self) -> Result<(), FallbackReason> {
        if !self.available {
            return Err(FallbackReason::CompileFailed);
        }
        if self.resource_count != LUMINANCE_RESOURCE_COUNT
            || self.root_constant_bytes != LUMINANCE_ROOT_CONSTANT_BYTES
            || self.srv_register != 0
            || self.uav_register != 0
            || !self.requires_shader_int64
        {
            return Err(FallbackReason::ValidationFailed);
        }
        if self.dxil_sha256 != LUMINANCE_OFFLINE_DXIL_SHA256
            || self.root_signature_sha256 != LUMINANCE_OFFLINE_ROOT_SIGNATURE_SHA256
            || self.descriptor_layout_sha256 != LUMINANCE_OFFLINE_DESCRIPTOR_LAYOUT_SHA256
        {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }
}

/// Floor-divide by 8 with Godot's `MAX(dim / 8, 1)` degenerate rule: any
/// dimension below 8 floors to 0 and clamps back to 1. Mirrors Godot's native
/// luminance cascade verbatim (`w = MAX(w / 8, 1)` in both
/// `Luminance::LuminanceBuffers::configure` and `Luminance::luminance_reduction`,
/// `servers/rendering/renderer_rd/effects/luminance.cpp`), so the planned
/// `reduce[]` extents are byte-identical to the native Godot luminance buffers
/// this pyramid rebinds. Native's own edge-drop is preserved: `dispatch_threads`
/// launches `ceil(source/8)` groups (one dest texel each) but the destination
/// buffer is only `floor(source/8)`, so the trailing partial 8×8 tile writes out
/// of bounds and is discarded by the hardware. The tracked reduce kernel's
/// internal `ceil`-based write guard drops that same partial texel against the
/// floor-sized buffer, so the in-bounds result stays bit-exact with native.
fn floor_div8(dim: u32) -> u32 {
    (dim / 8).max(1)
}

/// GRX-009 Wave 2: one dispatch level of the luminance reduction pyramid — the
/// source extent it reduces and the destination (`MAX(src/8, 1)`, native floor)
/// extent it writes. `is_final` marks the 1×1 WRITE_LUMINANCE level (clamp +
/// EMA).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyramidLevel {
    pub src_width: u32,
    pub src_height: u32,
    pub dst_width: u32,
    pub dst_height: u32,
    pub is_final: bool,
}

/// GRX-009 Wave 2: plan the luminance reduction pyramid for a source extent,
/// mirroring the native Godot cascade byte-for-byte — each level reduces by 8×8
/// tiles (`MAX(dim/8, 1)`, native floor) until the destination is 1×1; the last
/// level is the final WRITE_LUMINANCE level. Each level's source equals the
/// previous level's floor destination, so `reduce[]` extents match Godot's
/// native luminance buffers exactly (e.g. 256×144 → 32×18 → 4×2 → 1×1, three
/// dispatches). Always returns at least one level (a source already ≤ 8×8 yields
/// a single final level). Every step strictly shrinks (or clamps to 1), so it
/// always terminates.
pub fn plan_luminance_pyramid_levels(source_width: u32, source_height: u32) -> Vec<PyramidLevel> {
    let mut levels = Vec::new();
    let (mut w, mut h) = (source_width.max(1), source_height.max(1));
    loop {
        let dst_width = floor_div8(w);
        let dst_height = floor_div8(h);
        let is_final = dst_width == 1 && dst_height == 1;
        levels.push(PyramidLevel {
            src_width: w,
            src_height: h,
            dst_width,
            dst_height,
            is_final,
        });
        if is_final {
            break;
        }
        w = dst_width;
        h = dst_height;
    }
    levels
}

/// GRX-009 gate for the `luminance_reduction` pass (segment 4b).
///
/// The gate starts disabled and stays disabled in this segment. Segment 4a
/// established the runtime binding preflight; segment 4b adds an explicit,
/// opt-in gated dispatch bring-up path in front of it. Even when every
/// dispatch eligibility precondition is satisfied (Godot opt-in flag set,
/// 64-bit integer capability, non-null native D3D12 device/queue handles,
/// non-null resource handles, and a compiled package whose layout/digests
/// match the offline evidence), the explicit dispatch gate stays closed.
/// Segment 4c has since produced a standalone measured D3D12 dispatch smoke
/// (real device/queue, Rurix RTS0 accepted, compute PSO from the tracked
/// DXIL, dispatch + fence completion + UAV readback), but the gate remains
/// closed because there is no bridge-linked runtime dispatch recording path,
/// no measured bridge telemetry, and `rxgd_record_pass` must not return OK.
/// So `request_dispatch_bringup` always fails and the pass falls back to the
/// native Godot luminance path. While the gate is closed no estimated GPU/CPU
/// time may be attributed to this pass.
/// GRX-009 segment 4d measured telemetry for one real bridge-recorded D3D12
/// luminance dispatch. Populated only under the `d3d12-recording-shim` feature
/// when the recording shim completes a real dispatch. No GPU timestamp is
/// implemented yet, so no GPU time is attributed here (see
/// `gpu_timestamp_status=not_yet` in the segment 4d evidence).
#[cfg(feature = "d3d12-recording-shim")]
#[derive(Clone, Copy, Debug)]
pub struct DispatchRecord {
    pub fence_completed_value: u64,
    pub dispatch: (u32, u32, u32),
    pub dst_width: u32,
    pub dst_height: u32,
    pub readback_checksum: u32,
    pub dst_first_value: f32,
    pub dxil_signed: bool,
    /// Measured wall-clock CPU time of the record + submit + fence-wait, in ns.
    pub cpu_record_ns: u64,
}

#[derive(Debug)]
pub struct LuminanceReductionGate {
    enabled: bool,
    last_fallback_reason: Option<FallbackReason>,
    dispatch_package: LuminanceDispatchPackage,
    /// GRX-009 segment 4h: the FIRST missing real-pass prerequisite recorded
    /// by the last opt-in real-pass attempt (the identity carried by the
    /// `RXGD_REAL_PASS_BLOCKED` diagnostic), if any.
    last_real_pass_blocked: Option<&'static str>,
    /// The `RXGD_REAL_PASS_BLOCKED` diagnostic is printed once per session:
    /// the luminance call site runs every frame and one machine-readable line
    /// is enough for the segment 4h evidence.
    real_pass_blocked_emitted: bool,
    #[cfg(feature = "d3d12-recording-shim")]
    last_dispatch_record: Option<DispatchRecord>,
}

impl LuminanceReductionGate {
    pub fn new() -> LuminanceReductionGate {
        LuminanceReductionGate {
            enabled: false,
            last_fallback_reason: None,
            dispatch_package: LuminanceDispatchPackage::verified_offline_package(),
            last_real_pass_blocked: None,
            real_pass_blocked_emitted: false,
            #[cfg(feature = "d3d12-recording-shim")]
            last_dispatch_record: None,
        }
    }

    /// GRX-009 segment 4d: take the last measured dispatch record (if any). Only
    /// available under the `d3d12-recording-shim` feature.
    #[cfg(feature = "d3d12-recording-shim")]
    pub fn take_last_dispatch_record(&mut self) -> Option<DispatchRecord> {
        self.last_dispatch_record.take()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn last_fallback_reason(&self) -> Option<FallbackReason> {
        self.last_fallback_reason
    }

    /// GRX-009 segment 4h: the FIRST missing prerequisite recorded by the
    /// last opt-in real-pass attempt, or None when no real-pass attempt was
    /// made (or a future attempt actually dispatched).
    pub fn last_real_pass_blocked(&self) -> Option<&'static str> {
        self.last_real_pass_blocked
    }

    /// Segment 4a: enabling still always fails. The offline DXIL artifact
    /// exists (segment 4i, texture-capable tracked package), but the bridge
    /// links no runtime kernel and has no dispatch path, so the compiled-kernel
    /// precondition remains unmet.
    pub fn request_enable(&mut self) -> Result<(), FallbackReason> {
        self.enabled = false;
        self.last_fallback_reason = Some(FallbackReason::CompileFailed);
        Err(FallbackReason::CompileFailed)
    }

    /// Segment 4b: request entry into the gated dispatch bring-up path.
    ///
    /// Even after every dispatch eligibility precondition passes, this still
    /// fails. Segment 4c has a standalone measured D3D12 dispatch smoke, but
    /// the explicit dispatch gate stays closed because no bridge-linked
    /// runtime dispatch recording path is linked, no measured bridge
    /// telemetry exists, and `rxgd_record_pass` must not return OK. The caller
    /// must keep the native Godot luminance path. It never returns `Ok` in
    /// this segment, so no estimated GPU/CPU time is ever attributed to the
    /// pass.
    pub fn request_dispatch_bringup(&mut self) -> Result<(), FallbackReason> {
        self.enabled = false;
        self.last_fallback_reason = Some(FallbackReason::CompileFailed);
        Err(FallbackReason::CompileFailed)
    }

    /// Gate decision for one record attempt: a closed gate requests fallback
    /// to the native Godot luminance path.
    ///
    /// Retained for the segment 4a preflight contract and its regression
    /// tests; the segment 4b record path goes through
    /// [`LuminanceReductionGate::record_gated_dispatch_bringup`].
    #[cfg_attr(not(test), allow(dead_code))]
    fn record_outcome(&mut self) -> i32 {
        if !self.enabled {
            if self.last_fallback_reason.is_none() {
                self.last_fallback_reason = Some(FallbackReason::ManualDisabled);
            }
            return RXGD_STATUS_FALLBACK;
        }
        RXGD_STATUS_OK
    }

    /// GRX-009 segment 4a runtime binding preflight for one record attempt.
    ///
    /// Validates the luminance binding contract before any future gated
    /// dispatch slice may exist: the 64-bit integer shader capability, the
    /// b0/t0/u0 descriptor shape (two textures in src-then-dst order plus the
    /// 28-byte root constant block), the source dimensions carried in the b0
    /// push constants against the bound `src_luminance = t0` resource, and
    /// the `max(source / 8, 1)` level-0 reduce shape of `dst_luminance = u0`.
    /// A successful preflight still requests fallback: no real D3D12 dispatch
    /// path exists, the gate stays disabled, and no estimated GPU/CPU time
    /// may be attributed.
    ///
    /// Retained for the segment 4a contract and its regression tests; the
    /// segment 4b record path is
    /// [`LuminanceReductionGate::record_gated_dispatch_bringup`], which reuses
    /// [`LuminanceReductionGate::check_runtime_binding_preflight`].
    #[cfg_attr(not(test), allow(dead_code))]
    fn record_runtime_binding_preflight(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
    ) -> i32 {
        if let Err(reason) = Self::check_runtime_binding_preflight(caps, resources, push_constants)
        {
            self.last_fallback_reason = Some(reason);
            return RXGD_STATUS_FALLBACK;
        }
        self.record_outcome()
    }

    /// Pure runtime binding preflight (segment 4a): validates the luminance
    /// binding contract and returns the fallback reason on the first failure.
    /// It does not mutate the gate, so both the segment 4a preflight path and
    /// the segment 4b gated dispatch bring-up path can reuse it.
    fn check_runtime_binding_preflight(
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
    ) -> Result<(), FallbackReason> {
        if caps.flags & RXGD_CAP_SHADER_INT64 == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        if resources.len() as u64 != LUMINANCE_RESOURCE_COUNT {
            return Err(FallbackReason::ValidationFailed);
        }
        if push_constants.len() as u64 != LUMINANCE_ROOT_CONSTANT_BYTES {
            return Err(FallbackReason::ValidationFailed);
        }
        if resources[0].resource_type != RXGD_RESOURCE_TEXTURE
            || resources[1].resource_type != RXGD_RESOURCE_TEXTURE
        {
            return Err(FallbackReason::ValidationFailed);
        }
        // b0 root constants: source_width/source_height are lowered as i64
        // (2 DWORDs each) followed by three f32 scalars; only the dimensions
        // participate in binding preflight.
        let source_width = le_u64(&push_constants[0..8]);
        let source_height = le_u64(&push_constants[8..16]);
        if source_width == 0 || source_height == 0 {
            return Err(FallbackReason::ValidationFailed);
        }
        if source_width != u64::from(resources[0].width)
            || source_height != u64::from(resources[0].height)
        {
            return Err(FallbackReason::ValidationFailed);
        }
        if u64::from(resources[1].width) != (source_width / 8).max(1)
            || u64::from(resources[1].height) != (source_height / 8).max(1)
        {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }

    /// GRX-009 segment 4b gated dispatch bring-up for one record attempt.
    ///
    /// Runs the full segment 4a runtime binding preflight, then the dispatch
    /// eligibility checks, and only then consults the explicit dispatch gate.
    /// Any preflight or eligibility failure returns `RXGD_STATUS_FALLBACK`
    /// with a recorded fallback reason. Even when everything is eligible the
    /// explicit dispatch gate stays closed: segment 4c has a standalone
    /// measured D3D12 dispatch smoke, but there is still no bridge-linked
    /// runtime dispatch recording path, no measured bridge telemetry, and
    /// `rxgd_record_pass` must not return OK. So this still returns
    /// `RXGD_STATUS_FALLBACK` and never attributes estimated GPU/CPU time.
    fn record_gated_dispatch_bringup(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
        device: usize,
        queue: usize,
    ) -> i32 {
        if let Err(reason) = Self::check_runtime_binding_preflight(caps, resources, push_constants)
        {
            self.last_fallback_reason = Some(reason);
            return RXGD_STATUS_FALLBACK;
        }
        if let Err(reason) = self.check_dispatch_eligibility(caps, resources, device, queue) {
            self.last_fallback_reason = Some(reason);
            return RXGD_STATUS_FALLBACK;
        }
        // GRX-009 segment 4d: explicit, test-only bridge dispatch recording arm.
        // Only reachable when the crate is built with the `d3d12-recording-shim`
        // feature AND the caller advertises the harness-only record-arm flag.
        // The Godot module never sets that flag and the shipping bridge is built
        // without the feature, so the default runtime path never enters here.
        #[cfg(feature = "d3d12-recording-shim")]
        {
            if caps.flags & RXGD_CAP_LUMINANCE_DISPATCH_RECORD != 0 {
                return match self.attempt_real_dispatch_recording(
                    resources,
                    push_constants,
                    device,
                    queue,
                ) {
                    Ok(()) => RXGD_STATUS_OK,
                    Err(reason) => {
                        self.last_fallback_reason = Some(reason);
                        RXGD_STATUS_FALLBACK
                    }
                };
            }
        }

        // Explicit dispatch bring-up gate. All eligibility preconditions are
        // satisfied and segment 4c has a standalone measured D3D12 dispatch
        // smoke, but with no recording arm (or without the recording-shim
        // feature) there is no bridge-linked runtime dispatch recording path,
        // no measured bridge telemetry, and rxgd_record_pass must not return OK,
        // so entering the gate still fails and the caller keeps the native Godot
        // luminance path.
        match self.request_dispatch_bringup() {
            Ok(()) => RXGD_STATUS_OK,
            Err(reason) => {
                self.last_fallback_reason = Some(reason);
                RXGD_STATUS_FALLBACK
            }
        }
    }

    /// GRX-009 segment 4d: record one real D3D12 luminance dispatch through the
    /// linked recording shim. Requires the full dispatch eligibility gate to
    /// have already passed. Verifies the tracked artifact bytes hash to the
    /// offline digests, then hands the real device/queue/resource handles to the
    /// shim. On success the gate is marked enabled and the measured telemetry is
    /// retained for the caller; any shim failure returns a fallback reason.
    #[cfg(feature = "d3d12-recording-shim")]
    fn attempt_real_dispatch_recording(
        &mut self,
        resources: &[RxGdResource],
        push_constants: &[u8],
        device: usize,
        queue: usize,
    ) -> Result<(), FallbackReason> {
        let src = resources[0];
        let dst = resources[1];
        let record = d3d12_recording_shim::record_luminance_dispatch(
            device,
            queue,
            src.native_handle as usize,
            dst.native_handle as usize,
            push_constants,
            src.width,
            src.height,
            dst.width,
            dst.height,
        )?;
        self.enabled = true;
        self.last_dispatch_record = Some(record);
        Ok(())
    }

    /// GRX-009 stage A3 kernel-binding-kind conformance check (per slot).
    ///
    /// The tracked texture-capable hlsl_bridge kernel declares
    /// [`LUMINANCE_KERNEL_RESOURCE_BINDING_KINDS`] = `["texture2d",
    /// "rwtexture2d"]`: slot 0 is the `src_luminance` SRV
    /// (`Texture2D<float>` at t0) and slot 1 is the `dst_luminance` UAV
    /// (`RWTexture2D<float>` at u0). A real dispatch is only well-defined
    /// when every bound runtime resource provides the binding kind its slot
    /// declares. The Godot runtime hands the bridge real Texture2D
    /// `ID3D12Resource*` handles for both slots (segment 4e), which map to
    /// `"texture2d"`/`"rwtexture2d"` respectively, so the Godot runtime
    /// binding conforms. Buffer resources (`RXGD_RESOURCE_BUFFER` →
    /// `"raw_buffer_view"`) no longer conform at any slot and fail closed
    /// with `kernel_binding_kind_mismatch`.
    fn check_real_pass_binding_kind(resources: &[RxGdResource]) -> Result<(), FallbackReason> {
        if resources.len() != LUMINANCE_KERNEL_RESOURCE_BINDING_KINDS.len() {
            return Err(FallbackReason::ValidationFailed);
        }
        if resources.iter().enumerate().any(|(slot, resource)| {
            runtime_resource_binding_kind(resource, slot)
                != LUMINANCE_KERNEL_RESOURCE_BINDING_KINDS[slot]
        }) {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }

    /// GRX-009 stage A4 math-parity check.
    ///
    /// The tracked hlsl_bridge kernel implements the Godot level-0
    /// reduction (single 8×8 tile, arithmetic mean, partial-tile divisor
    /// matching Godot level 0). Its level-0 math is CPU-proven equivalent
    /// to the reference implementation in `math_parity_evidence.json`
    /// ([`LUMINANCE_KERNEL_MATH_PARITY_STATUS`] =
    /// `"level0_cpu_reference_proven_pending_gpu_dispatch"`: every case carries a
    /// CPU-expected grid; GPU observation is pending a real dispatch),
    /// so the level-0 real-pass attempt is no longer blocked on math
    /// parity. Full pyramid/EMA/WRITE_LUMINANCE parity remains gated on
    /// measured GPU results and the multi-level continuation round; any
    /// other status fails closed.
    fn check_real_pass_math_parity() -> Result<(), FallbackReason> {
        if LUMINANCE_KERNEL_MATH_PARITY_STATUS == "level0_cpu_reference_proven_pending_gpu_dispatch"
        {
            return Ok(());
        }
        Err(FallbackReason::ValidationFailed)
    }

    /// GRX-009 segment 4h: one opt-in gated REAL luminance pass attempt.
    ///
    /// Order: segment 4a runtime binding preflight → segment 4b dispatch
    /// eligibility → segment 4h kernel-binding-kind conformance → segment
    /// 4i math-pyramid-parity → linked real dispatch path. Every failure
    /// returns `RXGD_STATUS_FALLBACK` with a recorded fallback reason and a
    /// once-per-session machine-readable `RXGD_REAL_PASS_BLOCKED
    /// first_missing_prerequisite=...` diagnostic the segment 4h harness
    /// parses for evidence. No estimated GPU/CPU time is ever attributed on
    /// the fallback path and `enabled` stays false.
    ///
    /// GRX-009 stage A5: with the tracked texture-capable hlsl_bridge
    /// package every software gate can pass for the real Godot texture
    /// handles. The real dispatch invocation itself is linked only under
    /// the `d3d12-recording-shim` feature: there the attempt routes through
    /// [`Self::attempt_real_dispatch_recording`] and may return
    /// `RXGD_STATUS_OK` after a real recorded dispatch. The shipping
    /// feature-off bridge still fails closed with
    /// `real_dispatch_path_not_linked` (no dispatch path is compiled in),
    /// so the default runtime path never returns OK.
    fn record_real_pass_attempt(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
        device: usize,
        queue: usize,
    ) -> i32 {
        if let Err(reason) = Self::check_runtime_binding_preflight(caps, resources, push_constants)
        {
            return self.real_pass_blocked("runtime_binding_preflight_failed", reason);
        }
        if let Err(reason) = self.check_dispatch_eligibility(caps, resources, device, queue) {
            return self.real_pass_blocked("dispatch_eligibility_failed", reason);
        }
        if let Err(reason) = Self::check_real_pass_binding_kind(resources) {
            return self.real_pass_blocked("kernel_binding_kind_mismatch", reason);
        }
        if let Err(reason) = Self::check_real_pass_math_parity() {
            return self.real_pass_blocked("math_pyramid_parity_not_proven", reason);
        }
        // All software gates passed: the runtime binding conforms to the
        // tracked texture-capable kernel per slot and level-0 math parity is
        // CPU-proven. Under the d3d12-recording-shim feature the linked real
        // dispatch path records one real D3D12 dispatch and may return OK;
        // without the feature no dispatch path is compiled in and the
        // attempt fails closed.
        #[cfg(feature = "d3d12-recording-shim")]
        {
            return match self.attempt_real_dispatch_recording(
                resources,
                push_constants,
                device,
                queue,
            ) {
                Ok(()) => {
                    self.last_real_pass_blocked = None;
                    // GRX-009 stage A5 real-pass marker (patch 0009 harness
                    // contract): printed ONLY after a real recorded dispatch
                    // completed. Deliberately NOT an `ERROR:` line and NOT an
                    // `RXGD_DIAG` line, so the segment 4g/4h runtime log
                    // audits stay clean.
                    println!("RXGD_GODOT_RUNTIME_LUMINANCE_REAL_PASS recorded=1");
                    RXGD_STATUS_OK
                }
                Err(reason) => self.real_pass_blocked("real_dispatch_recording_failed", reason),
            };
        }
        #[cfg(not(feature = "d3d12-recording-shim"))]
        self.real_pass_blocked(
            "real_dispatch_path_not_linked",
            FallbackReason::CompileFailed,
        )
    }

    /// Records one blocked real-pass attempt: fallback status, reason, the
    /// first-missing-prerequisite identity, and (once per session) the
    /// machine-readable diagnostic line.
    fn real_pass_blocked(&mut self, prerequisite: &'static str, reason: FallbackReason) -> i32 {
        self.enabled = false;
        self.last_fallback_reason = Some(reason);
        self.last_real_pass_blocked = Some(prerequisite);
        if !self.real_pass_blocked_emitted {
            self.real_pass_blocked_emitted = true;
            // Deliberately NOT an `ERROR:` line and NOT an `RXGD_DIAG` line —
            // either would fail the segment 4g/4h runtime log audits.
            println!(
                "RXGD_REAL_PASS_BLOCKED first_missing_prerequisite={} fallback_reason={} \
                 kernel_binding={} default_enable_state=disabled",
                prerequisite,
                reason.as_str(),
                LUMINANCE_KERNEL_RESOURCE_BINDING_KIND,
            );
        }
        RXGD_STATUS_FALLBACK
    }

    /// GRX-009 segment 4b dispatch eligibility gate.
    ///
    /// Confirms that the caller opted into gated dispatch bring-up, that the
    /// device advertises the 64-bit integer shader capability, that the
    /// native D3D12 device/queue handles and both resource handles are
    /// non-null, and that the compiled package layout/digests still match the
    /// offline compile evidence. Returns the first failing precondition as a
    /// fallback reason. Passing eligibility never on its own records a
    /// dispatch: the explicit dispatch gate is consulted separately.
    fn check_dispatch_eligibility(
        &self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        device: usize,
        queue: usize,
    ) -> Result<(), FallbackReason> {
        // Godot opt-in gate: the reserved dispatch bring-up capability flag is
        // set by the Godot side only when the per-pass opt-in setting is on.
        if caps.flags & RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP == 0 {
            return Err(FallbackReason::ManualDisabled);
        }
        // 64-bit integer shader capability (also checked in preflight; the
        // eligibility gate re-affirms it as a device precondition).
        if caps.flags & RXGD_CAP_SHADER_INT64 == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        // Native D3D12 device/queue handles must be non-null.
        if device == 0 || queue == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        // Resource native handles must be non-null.
        if resources.iter().any(|resource| resource.native_handle == 0) {
            return Err(FallbackReason::ValidationFailed);
        }
        // Compiled package must be available and match the offline evidence.
        self.dispatch_package.verify_matches_offline_evidence()
    }

    /// GRX-009 Wave 2 hook-contract validation for the multi-level pyramid
    /// binding (fail-closed). The resource array is
    /// `[source, reduce[0..L-1], current, prev]` where `L = level_count - 1`,
    /// so its length is `level_count + 2`. Every entry must be a texture with a
    /// non-zero native handle: any zero handle (a missing Godot buffer) fails
    /// the WHOLE pyramid closed to the native path — no partial dispatch. See
    /// `hook_contract_v2.md`.
    pub fn check_pyramid_resource_binding(
        resources: &[RxGdResource],
        level_count: usize,
    ) -> Result<(), FallbackReason> {
        if level_count == 0 {
            return Err(FallbackReason::ValidationFailed);
        }
        // [source] + [reduce; L] + [current] + [prev], L = level_count - 1.
        if resources.len() != level_count + 2 {
            return Err(FallbackReason::ValidationFailed);
        }
        // Fail closed on any missing (zero) native handle: the whole pyramid
        // falls back rather than dispatching against an undefined binding.
        if resources.iter().any(|resource| resource.native_handle == 0) {
            return Err(FallbackReason::ValidationFailed);
        }
        // The reduce/write kernels bind Texture2D / RWTexture2D at every slot;
        // a buffer resource never conforms.
        if resources
            .iter()
            .any(|resource| resource.resource_type != RXGD_RESOURCE_TEXTURE)
        {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }

    /// GRX-009 Wave 2: one opt-in gated REAL multi-level luminance pyramid
    /// attempt (fail-closed). Plans the pyramid for `source_width`×
    /// `source_height`, checks the device precondition + the hook-contract
    /// resource array (`[source, reduce[0..L-1], current, prev]`) + the compiled
    /// package identity, then records the full reduce chain + final
    /// WRITE_LUMINANCE in ONE submit through the linked shim. Every failure
    /// returns `RXGD_STATUS_FALLBACK` with a recorded reason + the
    /// once-per-session `RXGD_REAL_PASS_BLOCKED` diagnostic naming the first
    /// missing prerequisite.
    ///
    /// The real dispatch is linked only under the `d3d12-recording-shim`
    /// feature; the shipping feature-off bridge fails closed with
    /// `real_dispatch_path_not_linked`. `rxgd_record_pass` routes the
    /// multi-resource luminance array (resource_count >= 3) here through
    /// [`Self::record_pyramid_from_push_constants`], which parses the b0 root
    /// constants first (hook_contract_v2 §3); the two-resource level-0 arm is
    /// unchanged.
    ///
    /// `first_frame` mirrors the native `p_set`: on the first frame there is no
    /// previous luminance, so the caller supplies a zero-cleared `prev` (the EMA
    /// then degenerates to `cur * exposure_adjust`). This method never fabricates
    /// a `prev`; it records exactly the binding it is handed. ONE-FRAME LATENCY:
    /// hooked from a Godot frame, `source`/`prev` carry the previous frame's
    /// content — documented, not hidden.
    #[allow(clippy::too_many_arguments)]
    fn record_pyramid_attempt(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        source_width: u32,
        source_height: u32,
        max_luminance: f32,
        min_luminance: f32,
        exposure_adjust: f32,
        first_frame: bool,
        device: usize,
        queue: usize,
    ) -> i32 {
        let _ = first_frame; // caller-owned prev buffer selection; no code path here.
        if caps.flags & RXGD_CAP_SHADER_INT64 == 0 {
            return self.real_pass_blocked(
                "dispatch_eligibility_failed",
                FallbackReason::UnsupportedDevice,
            );
        }
        if device == 0 || queue == 0 {
            return self.real_pass_blocked(
                "dispatch_eligibility_failed",
                FallbackReason::UnsupportedDevice,
            );
        }
        let levels = plan_luminance_pyramid_levels(source_width, source_height);
        if let Err(reason) = Self::check_pyramid_resource_binding(resources, levels.len()) {
            return self.real_pass_blocked("pyramid_binding_invalid", reason);
        }
        if let Err(reason) = self.dispatch_package.verify_matches_offline_evidence() {
            return self.real_pass_blocked("dispatch_eligibility_failed", reason);
        }
        #[cfg(feature = "d3d12-recording-shim")]
        {
            let handles: Vec<usize> = resources
                .iter()
                .map(|resource| resource.native_handle as usize)
                .collect();
            match d3d12_recording_shim::record_luminance_pyramid_dispatch(
                device,
                queue,
                &handles,
                &levels,
                max_luminance,
                min_luminance,
                exposure_adjust,
                /*readback=*/ false,
            ) {
                Ok(record) => {
                    self.enabled = true;
                    self.last_dispatch_record = Some(record);
                    self.last_real_pass_blocked = None;
                    RXGD_STATUS_OK
                }
                Err(reason) => self.real_pass_blocked("real_dispatch_recording_failed", reason),
            }
        }
        #[cfg(not(feature = "d3d12-recording-shim"))]
        {
            let _ = (max_luminance, min_luminance, exposure_adjust);
            self.real_pass_blocked(
                "real_dispatch_path_not_linked",
                FallbackReason::CompileFailed,
            )
        }
    }

    /// GRX-009 Wave 2: `rxgd_record_pass` entry for the multi-resource
    /// luminance pyramid arm (hook_contract_v2). Parses the 28-byte b0 root
    /// constant block (§3: `source_width`/`source_height` as little-endian i64
    /// followed by `max`/`min`/`exposure_adjust` f32), failing the whole
    /// pyramid closed with `pyramid_push_constants_invalid` on any length
    /// mismatch or out-of-range dimension, then routes into
    /// [`Self::record_pyramid_attempt`]. `first_frame` (the native `p_set`) is
    /// caller-owned prev-buffer selection and is NOT carried on the wire (the
    /// 28-byte block has no slot for it, matching Godot patch 0010), so it is
    /// recorded as `false` here; the bridge never fabricates a `prev`.
    fn record_pyramid_from_push_constants(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
        device: usize,
        queue: usize,
    ) -> i32 {
        let Some((source_width, source_height, max_luminance, min_luminance, exposure_adjust)) =
            parse_luminance_root_constants(push_constants)
        else {
            return self.real_pass_blocked(
                "pyramid_push_constants_invalid",
                FallbackReason::ValidationFailed,
            );
        };
        self.record_pyramid_attempt(
            caps,
            resources,
            source_width,
            source_height,
            max_luminance,
            min_luminance,
            exposure_adjust,
            /*first_frame=*/ false,
            device,
            queue,
        )
    }
}

impl Default for LuminanceReductionGate {
    fn default() -> LuminanceReductionGate {
        LuminanceReductionGate::new()
    }
}

/// Identity of the GRX-010 compiled tonemap package (DXIL container, root
/// signature, descriptor layout) as seen by the bridge. Template copy of
/// [`LuminanceDispatchPackage`] with the constants and digests pointing at
/// the tonemap artifacts (texture-capable hlsl_bridge workaround package,
/// owner-approved `hlsl_bridge_workaround` provenance).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TonemapDispatchPackage {
    pub available: bool,
    pub resource_count: u64,
    pub root_constant_bytes: u64,
    pub srv_register: u32,
    pub uav_register: u32,
    pub requires_shader_int64: bool,
    pub dxil_sha256: &'static str,
    pub root_signature_sha256: &'static str,
    pub descriptor_layout_sha256: &'static str,
}

impl TonemapDispatchPackage {
    /// The compiled package identity that matches the tracked GRX-010
    /// offline compile evidence.
    pub fn verified_offline_package() -> TonemapDispatchPackage {
        TonemapDispatchPackage {
            available: true,
            resource_count: TONEMAP_RESOURCE_COUNT,
            root_constant_bytes: TONEMAP_ROOT_CONSTANT_BYTES,
            srv_register: 0,
            uav_register: 0,
            requires_shader_int64: true,
            dxil_sha256: TONEMAP_OFFLINE_DXIL_SHA256,
            root_signature_sha256: TONEMAP_OFFLINE_ROOT_SIGNATURE_SHA256,
            descriptor_layout_sha256: TONEMAP_OFFLINE_DESCRIPTOR_LAYOUT_SHA256,
        }
    }

    /// Verifies the compiled package is present and that its descriptor
    /// layout and artifact digests match the tracked offline compile
    /// evidence. An unavailable package maps to `compile_failed`; any
    /// layout or digest mismatch maps to `validation_failed`.
    fn verify_matches_offline_evidence(&self) -> Result<(), FallbackReason> {
        if !self.available {
            return Err(FallbackReason::CompileFailed);
        }
        if self.resource_count != TONEMAP_RESOURCE_COUNT
            || self.root_constant_bytes != TONEMAP_ROOT_CONSTANT_BYTES
            || self.srv_register != 0
            || self.uav_register != 0
            || !self.requires_shader_int64
        {
            return Err(FallbackReason::ValidationFailed);
        }
        if self.dxil_sha256 != TONEMAP_OFFLINE_DXIL_SHA256
            || self.root_signature_sha256 != TONEMAP_OFFLINE_ROOT_SIGNATURE_SHA256
            || self.descriptor_layout_sha256 != TONEMAP_OFFLINE_DESCRIPTOR_LAYOUT_SHA256
        {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }
}

/// GRX-010 gate for the `tonemap` pass. Template copy of the GRX-009
/// [`LuminanceReductionGate`] preflight → eligibility → binding-kind →
/// math-parity → linked-dispatch chain, with every constant and digest
/// pointing at the tonemap package.
///
/// The gate starts disabled and stays disabled: the default record path
/// (no [`RXGD_CAP_TONEMAP_REAL_PASS`] arm) always returns
/// `RXGD_STATUS_FALLBACK`, and the opt-in real-pass arm fails closed with
/// `real_dispatch_path_not_linked` on the shipping feature-off bridge. No
/// estimated GPU/CPU time is ever attributed to this pass while the gate
/// is closed. Note this REMOVES the historical placeholder behaviour where
/// `RXGD_PASS_TONEMAP` was recorded with estimated timings.
#[derive(Debug)]
pub struct TonemapGate {
    enabled: bool,
    last_fallback_reason: Option<FallbackReason>,
    dispatch_package: TonemapDispatchPackage,
    /// The FIRST missing real-pass prerequisite recorded by the last
    /// opt-in real-pass attempt (the identity carried by the
    /// `RXGD_TONEMAP_REAL_PASS_BLOCKED` diagnostic), if any.
    last_real_pass_blocked: Option<&'static str>,
    /// The diagnostic is printed once per session: the tonemap call site
    /// runs every frame and one machine-readable line is enough.
    real_pass_blocked_emitted: bool,
    #[cfg(feature = "d3d12-recording-shim")]
    last_dispatch_record: Option<DispatchRecord>,
}

impl TonemapGate {
    pub fn new() -> TonemapGate {
        TonemapGate {
            enabled: false,
            last_fallback_reason: None,
            dispatch_package: TonemapDispatchPackage::verified_offline_package(),
            last_real_pass_blocked: None,
            real_pass_blocked_emitted: false,
            #[cfg(feature = "d3d12-recording-shim")]
            last_dispatch_record: None,
        }
    }

    /// Take the last measured dispatch record (if any). Only available
    /// under the `d3d12-recording-shim` feature.
    #[cfg(feature = "d3d12-recording-shim")]
    pub fn take_last_dispatch_record(&mut self) -> Option<DispatchRecord> {
        self.last_dispatch_record.take()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn last_fallback_reason(&self) -> Option<FallbackReason> {
        self.last_fallback_reason
    }

    /// The FIRST missing prerequisite recorded by the last opt-in
    /// real-pass attempt, or None when no real-pass attempt was made (or a
    /// future attempt actually dispatched).
    pub fn last_real_pass_blocked(&self) -> Option<&'static str> {
        self.last_real_pass_blocked
    }

    /// Pure runtime binding preflight: validates the tonemap binding
    /// contract and returns the fallback reason on the first failure.
    ///
    /// Mirrors the GRX-009 segment 4a preflight with the tonemap dst
    /// shape: the 64-bit integer shader capability (the b0 block carries
    /// i64 dims per the canonical template), exactly two texture resources
    /// in src-then-dst order, the 28-byte b0 root constant block, nonzero
    /// source dimensions matching the bound `src_color` resource, and a
    /// `dst_color` extent equal to the source extent (tonemap is a 1:1
    /// full-resolution pass, unlike luminance's `max(source / 8, 1)`).
    fn check_runtime_binding_preflight(
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
    ) -> Result<(), FallbackReason> {
        if caps.flags & RXGD_CAP_SHADER_INT64 == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        if resources.len() as u64 != TONEMAP_RESOURCE_COUNT {
            return Err(FallbackReason::ValidationFailed);
        }
        if push_constants.len() as u64 != TONEMAP_ROOT_CONSTANT_BYTES {
            return Err(FallbackReason::ValidationFailed);
        }
        if resources[0].resource_type != RXGD_RESOURCE_TEXTURE
            || resources[1].resource_type != RXGD_RESOURCE_TEXTURE
        {
            return Err(FallbackReason::ValidationFailed);
        }
        // b0 root constants: source_width/source_height are lowered as i64
        // (2 DWORDs each) followed by exposure/white/luminance_multiplier;
        // only the dimensions participate in binding preflight.
        let source_width = le_u64(&push_constants[0..8]);
        let source_height = le_u64(&push_constants[8..16]);
        if source_width == 0 || source_height == 0 {
            return Err(FallbackReason::ValidationFailed);
        }
        if source_width != u64::from(resources[0].width)
            || source_height != u64::from(resources[0].height)
        {
            return Err(FallbackReason::ValidationFailed);
        }
        if u64::from(resources[1].width) != source_width
            || u64::from(resources[1].height) != source_height
        {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }

    /// GRX-010 dispatch eligibility gate (template copy of the GRX-009
    /// segment 4b gate): the caller must arm the tonemap real-pass opt-in
    /// flag, the device must advertise the 64-bit integer capability, the
    /// native D3D12 device/queue handles and both resource handles must be
    /// non-null, and the compiled package layout/digests must still match
    /// the offline compile evidence.
    fn check_dispatch_eligibility(
        &self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        device: usize,
        queue: usize,
    ) -> Result<(), FallbackReason> {
        if caps.flags & RXGD_CAP_TONEMAP_REAL_PASS == 0 {
            return Err(FallbackReason::ManualDisabled);
        }
        if caps.flags & RXGD_CAP_SHADER_INT64 == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        if device == 0 || queue == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        if resources.iter().any(|resource| resource.native_handle == 0) {
            return Err(FallbackReason::ValidationFailed);
        }
        self.dispatch_package.verify_matches_offline_evidence()
    }

    /// GRX-010 kernel-binding-kind conformance check (per slot). The
    /// tracked tonemap kernel declares
    /// [`TONEMAP_KERNEL_RESOURCE_BINDING_KINDS`] = `["texture2d",
    /// "rwtexture2d"]`; buffer resources (`raw_buffer_view`) fail closed
    /// at any slot.
    fn check_real_pass_binding_kind(resources: &[RxGdResource]) -> Result<(), FallbackReason> {
        if resources.len() != TONEMAP_KERNEL_RESOURCE_BINDING_KINDS.len() {
            return Err(FallbackReason::ValidationFailed);
        }
        if resources.iter().enumerate().any(|(slot, resource)| {
            runtime_resource_binding_kind(resource, slot)
                != TONEMAP_KERNEL_RESOURCE_BINDING_KINDS[slot]
        }) {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }

    /// GRX-010 math-parity check. The tracked hlsl_bridge tonemap kernel's
    /// LINEAR + linear_to_srgb subset is CPU-proven equivalent to the
    /// reference implementation in the pass `math_parity_evidence.json`
    /// (GPU observation pending a real dispatch); any other status fails
    /// closed.
    fn check_real_pass_math_parity() -> Result<(), FallbackReason> {
        if TONEMAP_KERNEL_MATH_PARITY_STATUS
            == "linear_srgb_cpu_reference_proven_pending_gpu_dispatch"
        {
            return Ok(());
        }
        Err(FallbackReason::ValidationFailed)
    }

    /// Default record path (no real-pass arm): runs the runtime binding
    /// preflight for an honest fallback reason and then keeps the gate
    /// closed with `manual_disabled`. Never returns OK and never
    /// attributes estimated GPU/CPU time. With patch 0012's runtime
    /// resource binding the Godot module gate now passes a real, fully
    /// valid tonemap binding, so in practice this records
    /// `manual_disabled` (the pre-0012 0002-level gate carried no
    /// bindings and recorded `validation_failed` from the preflight).
    fn record_default_fallback(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
    ) -> i32 {
        if let Err(reason) = Self::check_runtime_binding_preflight(caps, resources, push_constants)
        {
            self.last_fallback_reason = Some(reason);
            return RXGD_STATUS_FALLBACK;
        }
        self.enabled = false;
        self.last_fallback_reason = Some(FallbackReason::ManualDisabled);
        RXGD_STATUS_FALLBACK
    }

    /// GRX-010 opt-in gated REAL tonemap pass attempt. Order: runtime
    /// binding preflight → dispatch eligibility → per-slot
    /// kernel-binding-kind conformance → math parity → linked real
    /// dispatch path. Every failure returns `RXGD_STATUS_FALLBACK` with a
    /// recorded fallback reason and a once-per-session machine-readable
    /// `RXGD_TONEMAP_REAL_PASS_BLOCKED first_missing_prerequisite=...`
    /// diagnostic. The real dispatch invocation is linked only under the
    /// `d3d12-recording-shim` feature; the shipping feature-off bridge
    /// fails closed with `real_dispatch_path_not_linked`.
    fn record_real_pass_attempt(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
        device: usize,
        queue: usize,
    ) -> i32 {
        if let Err(reason) = Self::check_runtime_binding_preflight(caps, resources, push_constants)
        {
            return self.real_pass_blocked("runtime_binding_preflight_failed", reason);
        }
        if let Err(reason) = self.check_dispatch_eligibility(caps, resources, device, queue) {
            return self.real_pass_blocked("dispatch_eligibility_failed", reason);
        }
        if let Err(reason) = Self::check_real_pass_binding_kind(resources) {
            return self.real_pass_blocked("kernel_binding_kind_mismatch", reason);
        }
        if let Err(reason) = Self::check_real_pass_math_parity() {
            return self.real_pass_blocked("math_parity_not_proven", reason);
        }
        #[cfg(feature = "d3d12-recording-shim")]
        {
            return match self.attempt_real_dispatch_recording(
                resources,
                push_constants,
                device,
                queue,
            ) {
                Ok(()) => {
                    self.last_real_pass_blocked = None;
                    // Printed ONLY after a real recorded dispatch completed.
                    // Deliberately NOT an `ERROR:` line and NOT an
                    // `RXGD_DIAG` line, so runtime log audits stay clean.
                    println!("RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS recorded=1");
                    RXGD_STATUS_OK
                }
                Err(reason) => self.real_pass_blocked("real_dispatch_recording_failed", reason),
            };
        }
        #[cfg(not(feature = "d3d12-recording-shim"))]
        self.real_pass_blocked(
            "real_dispatch_path_not_linked",
            FallbackReason::CompileFailed,
        )
    }

    /// Record one real D3D12 tonemap dispatch through the linked recording
    /// shim (the generic texture-pass shim shared with GRX-009; the SRV
    /// t0 + UAV u0 + 28-byte b0 + `ceil(dims / 8)` dispatch shape matches
    /// the tonemap kernel exactly, and the view formats are derived from
    /// the real resource formats).
    #[cfg(feature = "d3d12-recording-shim")]
    fn attempt_real_dispatch_recording(
        &mut self,
        resources: &[RxGdResource],
        push_constants: &[u8],
        device: usize,
        queue: usize,
    ) -> Result<(), FallbackReason> {
        let src = resources[0];
        let dst = resources[1];
        let record = d3d12_recording_shim::record_tonemap_dispatch(
            device,
            queue,
            src.native_handle as usize,
            dst.native_handle as usize,
            push_constants,
            src.width,
            src.height,
            dst.width,
            dst.height,
        )?;
        self.enabled = true;
        self.last_dispatch_record = Some(record);
        Ok(())
    }

    /// Records one blocked real-pass attempt: fallback status, reason, the
    /// first-missing-prerequisite identity, and (once per session) the
    /// machine-readable diagnostic line.
    fn real_pass_blocked(&mut self, prerequisite: &'static str, reason: FallbackReason) -> i32 {
        self.enabled = false;
        self.last_fallback_reason = Some(reason);
        self.last_real_pass_blocked = Some(prerequisite);
        if !self.real_pass_blocked_emitted {
            self.real_pass_blocked_emitted = true;
            // Deliberately NOT an `ERROR:` line and NOT an `RXGD_DIAG` line —
            // either would fail the runtime log audits.
            println!(
                "RXGD_TONEMAP_REAL_PASS_BLOCKED first_missing_prerequisite={} \
                 fallback_reason={} kernel_binding={} default_enable_state=disabled",
                prerequisite,
                reason.as_str(),
                TONEMAP_KERNEL_RESOURCE_BINDING_KIND,
            );
        }
        RXGD_STATUS_FALLBACK
    }
}

impl Default for TonemapGate {
    fn default() -> TonemapGate {
        TonemapGate::new()
    }
}

/// Identity of the GRX-011 compiled ssao_blur package (DXIL container, root
/// signature, descriptor layout) as seen by the bridge. Template copy of
/// [`TonemapDispatchPackage`] with the constants and digests pointing at
/// the ssao_blur artifacts (texture-capable hlsl_bridge workaround package,
/// owner-approved `hlsl_bridge_workaround` provenance).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SsaoBlurDispatchPackage {
    pub available: bool,
    pub resource_count: u64,
    pub root_constant_bytes: u64,
    pub srv_register: u32,
    pub uav_register: u32,
    pub requires_shader_int64: bool,
    pub dxil_sha256: &'static str,
    pub root_signature_sha256: &'static str,
    pub descriptor_layout_sha256: &'static str,
}

impl SsaoBlurDispatchPackage {
    /// The compiled package identity that matches the tracked GRX-011
    /// offline compile evidence.
    pub fn verified_offline_package() -> SsaoBlurDispatchPackage {
        SsaoBlurDispatchPackage {
            available: true,
            resource_count: SSAO_BLUR_RESOURCE_COUNT,
            root_constant_bytes: SSAO_BLUR_ROOT_CONSTANT_BYTES,
            srv_register: 0,
            uav_register: 0,
            requires_shader_int64: true,
            dxil_sha256: SSAO_BLUR_OFFLINE_DXIL_SHA256,
            root_signature_sha256: SSAO_BLUR_OFFLINE_ROOT_SIGNATURE_SHA256,
            descriptor_layout_sha256: SSAO_BLUR_OFFLINE_DESCRIPTOR_LAYOUT_SHA256,
        }
    }

    /// Verifies the compiled package is present and that its descriptor
    /// layout and artifact digests match the tracked offline compile
    /// evidence. An unavailable package maps to `compile_failed`; any
    /// layout or digest mismatch maps to `validation_failed`.
    fn verify_matches_offline_evidence(&self) -> Result<(), FallbackReason> {
        if !self.available {
            return Err(FallbackReason::CompileFailed);
        }
        if self.resource_count != SSAO_BLUR_RESOURCE_COUNT
            || self.root_constant_bytes != SSAO_BLUR_ROOT_CONSTANT_BYTES
            || self.srv_register != 0
            || self.uav_register != 0
            || !self.requires_shader_int64
        {
            return Err(FallbackReason::ValidationFailed);
        }
        if self.dxil_sha256 != SSAO_BLUR_OFFLINE_DXIL_SHA256
            || self.root_signature_sha256 != SSAO_BLUR_OFFLINE_ROOT_SIGNATURE_SHA256
            || self.descriptor_layout_sha256 != SSAO_BLUR_OFFLINE_DESCRIPTOR_LAYOUT_SHA256
        {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }
}

/// GRX-011 gate for the `ssao_blur` pass. Template copy of the GRX-010
/// [`TonemapGate`] preflight → eligibility → binding-kind → math-parity →
/// linked-dispatch chain, with every constant and digest pointing at the
/// ssao_blur package.
///
/// The gate starts disabled and stays disabled: the default record path
/// (no [`RXGD_CAP_SSAO_BLUR_REAL_PASS`] arm) always returns
/// `RXGD_STATUS_FALLBACK`, and the opt-in real-pass arm fails closed with
/// `real_dispatch_path_not_linked` on the shipping feature-off bridge. No
/// estimated GPU/CPU time is ever attributed to this pass while the gate
/// is closed. Note this REMOVES the historical placeholder behaviour where
/// `RXGD_PASS_SSAO_BLUR` was recorded with estimated timings
/// (`RXGD_PASS_SSIL_BLUR` keeps that placeholder path; it is not wired in
/// this slice).
#[derive(Debug)]
pub struct SsaoBlurGate {
    enabled: bool,
    last_fallback_reason: Option<FallbackReason>,
    dispatch_package: SsaoBlurDispatchPackage,
    /// The FIRST missing real-pass prerequisite recorded by the last
    /// opt-in real-pass attempt (the identity carried by the
    /// `RXGD_SSAO_BLUR_REAL_PASS_BLOCKED` diagnostic), if any.
    last_real_pass_blocked: Option<&'static str>,
    /// The diagnostic is printed once per session: the SSAO blur call site
    /// runs every frame and one machine-readable line is enough.
    real_pass_blocked_emitted: bool,
    #[cfg(feature = "d3d12-recording-shim")]
    last_dispatch_record: Option<DispatchRecord>,
}

impl SsaoBlurGate {
    pub fn new() -> SsaoBlurGate {
        SsaoBlurGate {
            enabled: false,
            last_fallback_reason: None,
            dispatch_package: SsaoBlurDispatchPackage::verified_offline_package(),
            last_real_pass_blocked: None,
            real_pass_blocked_emitted: false,
            #[cfg(feature = "d3d12-recording-shim")]
            last_dispatch_record: None,
        }
    }

    /// Take the last measured dispatch record (if any). Only available
    /// under the `d3d12-recording-shim` feature.
    #[cfg(feature = "d3d12-recording-shim")]
    pub fn take_last_dispatch_record(&mut self) -> Option<DispatchRecord> {
        self.last_dispatch_record.take()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn last_fallback_reason(&self) -> Option<FallbackReason> {
        self.last_fallback_reason
    }

    /// The FIRST missing prerequisite recorded by the last opt-in
    /// real-pass attempt, or None when no real-pass attempt was made (or a
    /// future attempt actually dispatched).
    pub fn last_real_pass_blocked(&self) -> Option<&'static str> {
        self.last_real_pass_blocked
    }

    /// Pure runtime binding preflight: validates the ssao_blur binding
    /// contract and returns the fallback reason on the first failure.
    ///
    /// Mirrors the GRX-010 preflight with the ssao_blur dst shape: the
    /// 64-bit integer shader capability (the b0 block carries i64 dims per
    /// the canonical template), exactly two texture resources in
    /// src-then-dst order, the 28-byte b0 root constant block, nonzero
    /// source dimensions matching the bound `src_ssao` resource, and a
    /// `dst_ssao` extent equal to the source extent (the blur is a 1:1
    /// ping-pong pass at the deinterleaved slice size).
    fn check_runtime_binding_preflight(
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
    ) -> Result<(), FallbackReason> {
        if caps.flags & RXGD_CAP_SHADER_INT64 == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        if resources.len() as u64 != SSAO_BLUR_RESOURCE_COUNT {
            return Err(FallbackReason::ValidationFailed);
        }
        if push_constants.len() as u64 != SSAO_BLUR_ROOT_CONSTANT_BYTES {
            return Err(FallbackReason::ValidationFailed);
        }
        if resources[0].resource_type != RXGD_RESOURCE_TEXTURE
            || resources[1].resource_type != RXGD_RESOURCE_TEXTURE
        {
            return Err(FallbackReason::ValidationFailed);
        }
        // b0 root constants: source_width/source_height are lowered as i64
        // (2 DWORDs each) followed by edge_sharpness and the two
        // half_screen_pixel_size floats; only the dimensions participate in
        // binding preflight.
        let source_width = le_u64(&push_constants[0..8]);
        let source_height = le_u64(&push_constants[8..16]);
        if source_width == 0 || source_height == 0 {
            return Err(FallbackReason::ValidationFailed);
        }
        if source_width != u64::from(resources[0].width)
            || source_height != u64::from(resources[0].height)
        {
            return Err(FallbackReason::ValidationFailed);
        }
        if u64::from(resources[1].width) != source_width
            || u64::from(resources[1].height) != source_height
        {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }

    /// GRX-011 dispatch eligibility gate (template copy of the GRX-010
    /// gate): the caller must arm the ssao_blur real-pass opt-in flag, the
    /// device must advertise the 64-bit integer capability, the native
    /// D3D12 device/queue handles and both resource handles must be
    /// non-null, and the compiled package layout/digests must still match
    /// the offline compile evidence.
    fn check_dispatch_eligibility(
        &self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        device: usize,
        queue: usize,
    ) -> Result<(), FallbackReason> {
        if caps.flags & RXGD_CAP_SSAO_BLUR_REAL_PASS == 0 {
            return Err(FallbackReason::ManualDisabled);
        }
        if caps.flags & RXGD_CAP_SHADER_INT64 == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        if device == 0 || queue == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        if resources.iter().any(|resource| resource.native_handle == 0) {
            return Err(FallbackReason::ValidationFailed);
        }
        self.dispatch_package.verify_matches_offline_evidence()
    }

    /// GRX-011 kernel-binding-kind conformance check (per slot). The
    /// tracked ssao_blur kernel declares
    /// [`SSAO_BLUR_KERNEL_RESOURCE_BINDING_KINDS`] = `["texture2d",
    /// "rwtexture2d"]`; buffer resources (`raw_buffer_view`) fail closed
    /// at any slot.
    fn check_real_pass_binding_kind(resources: &[RxGdResource]) -> Result<(), FallbackReason> {
        if resources.len() != SSAO_BLUR_KERNEL_RESOURCE_BINDING_KINDS.len() {
            return Err(FallbackReason::ValidationFailed);
        }
        if resources.iter().enumerate().any(|(slot, resource)| {
            runtime_resource_binding_kind(resource, slot)
                != SSAO_BLUR_KERNEL_RESOURCE_BINDING_KINDS[slot]
        }) {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }

    /// GRX-011 math-parity check. The tracked hlsl_bridge ssao_blur
    /// kernel's MODE_SMART edge-aware cross blur subset is CPU-proven
    /// equivalent to the reference implementation in the pass
    /// `math_parity_evidence.json` (GPU observation pending a real
    /// dispatch); any other status fails closed.
    fn check_real_pass_math_parity() -> Result<(), FallbackReason> {
        if SSAO_BLUR_KERNEL_MATH_PARITY_STATUS
            == "smart_blur_cpu_reference_proven_pending_gpu_dispatch"
        {
            return Ok(());
        }
        Err(FallbackReason::ValidationFailed)
    }

    /// Default record path (no real-pass arm): runs the runtime binding
    /// preflight for an honest fallback reason and then keeps the gate
    /// closed with `manual_disabled`. Never returns OK and never
    /// attributes estimated GPU/CPU time. Patch 0012's module gate calls
    /// the bridge without resource bindings (0002-level wiring), so in
    /// practice this records `validation_failed` from the preflight.
    fn record_default_fallback(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
    ) -> i32 {
        if let Err(reason) = Self::check_runtime_binding_preflight(caps, resources, push_constants)
        {
            self.last_fallback_reason = Some(reason);
            return RXGD_STATUS_FALLBACK;
        }
        self.enabled = false;
        self.last_fallback_reason = Some(FallbackReason::ManualDisabled);
        RXGD_STATUS_FALLBACK
    }

    /// GRX-011 opt-in gated REAL SSAO blur pass attempt. Order: runtime
    /// binding preflight → dispatch eligibility → per-slot
    /// kernel-binding-kind conformance → math parity → linked real
    /// dispatch path. Every failure returns `RXGD_STATUS_FALLBACK` with a
    /// recorded fallback reason and a once-per-session machine-readable
    /// `RXGD_SSAO_BLUR_REAL_PASS_BLOCKED first_missing_prerequisite=...`
    /// diagnostic. The real dispatch invocation is linked only under the
    /// `d3d12-recording-shim` feature; the shipping feature-off bridge
    /// fails closed with `real_dispatch_path_not_linked`.
    fn record_real_pass_attempt(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
        device: usize,
        queue: usize,
    ) -> i32 {
        if let Err(reason) = Self::check_runtime_binding_preflight(caps, resources, push_constants)
        {
            return self.real_pass_blocked("runtime_binding_preflight_failed", reason);
        }
        if let Err(reason) = self.check_dispatch_eligibility(caps, resources, device, queue) {
            return self.real_pass_blocked("dispatch_eligibility_failed", reason);
        }
        if let Err(reason) = Self::check_real_pass_binding_kind(resources) {
            return self.real_pass_blocked("kernel_binding_kind_mismatch", reason);
        }
        if let Err(reason) = Self::check_real_pass_math_parity() {
            return self.real_pass_blocked("math_parity_not_proven", reason);
        }
        #[cfg(feature = "d3d12-recording-shim")]
        {
            return match self.attempt_real_dispatch_recording(
                resources,
                push_constants,
                device,
                queue,
            ) {
                Ok(()) => {
                    self.last_real_pass_blocked = None;
                    // Printed ONLY after a real recorded dispatch completed.
                    // Deliberately NOT an `ERROR:` line and NOT an
                    // `RXGD_DIAG` line, so runtime log audits stay clean.
                    println!("RXGD_GODOT_RUNTIME_SSAO_BLUR_REAL_PASS recorded=1");
                    RXGD_STATUS_OK
                }
                Err(reason) => self.real_pass_blocked("real_dispatch_recording_failed", reason),
            };
        }
        #[cfg(not(feature = "d3d12-recording-shim"))]
        self.real_pass_blocked(
            "real_dispatch_path_not_linked",
            FallbackReason::CompileFailed,
        )
    }

    /// Record one real D3D12 SSAO blur dispatch through the linked
    /// recording shim (the generic texture-pass shim shared with
    /// GRX-009/GRX-010; the SRV t0 + UAV u0 + 28-byte b0 +
    /// `ceil(dims / 8)` dispatch shape matches the ssao_blur kernel
    /// exactly, and the view formats are derived from the real resource
    /// formats).
    #[cfg(feature = "d3d12-recording-shim")]
    fn attempt_real_dispatch_recording(
        &mut self,
        resources: &[RxGdResource],
        push_constants: &[u8],
        device: usize,
        queue: usize,
    ) -> Result<(), FallbackReason> {
        let src = resources[0];
        let dst = resources[1];
        let record = d3d12_recording_shim::record_ssao_blur_dispatch(
            device,
            queue,
            src.native_handle as usize,
            dst.native_handle as usize,
            push_constants,
            src.width,
            src.height,
            dst.width,
            dst.height,
        )?;
        self.enabled = true;
        self.last_dispatch_record = Some(record);
        Ok(())
    }

    /// Records one blocked real-pass attempt: fallback status, reason, the
    /// first-missing-prerequisite identity, and (once per session) the
    /// machine-readable diagnostic line.
    fn real_pass_blocked(&mut self, prerequisite: &'static str, reason: FallbackReason) -> i32 {
        self.enabled = false;
        self.last_fallback_reason = Some(reason);
        self.last_real_pass_blocked = Some(prerequisite);
        if !self.real_pass_blocked_emitted {
            self.real_pass_blocked_emitted = true;
            // Deliberately NOT an `ERROR:` line and NOT an `RXGD_DIAG` line —
            // either would fail the runtime log audits.
            println!(
                "RXGD_SSAO_BLUR_REAL_PASS_BLOCKED first_missing_prerequisite={} \
                 fallback_reason={} kernel_binding={} default_enable_state=disabled",
                prerequisite,
                reason.as_str(),
                SSAO_BLUR_KERNEL_RESOURCE_BINDING_KIND,
            );
        }
        RXGD_STATUS_FALLBACK
    }
}

impl Default for SsaoBlurGate {
    fn default() -> SsaoBlurGate {
        SsaoBlurGate::new()
    }
}

/// Identity of the GRX-012 compiled taa_resolve package (DXIL container, root
/// signature, descriptor layout) as seen by the bridge. Template copy of
/// [`SsaoBlurDispatchPackage`] with the constants and digests pointing at the
/// taa_resolve artifacts (texture-capable hlsl_bridge workaround package,
/// owner-approved `hlsl_bridge_workaround` provenance). Unlike the 2-resource
/// passes, taa_resolve binds SIX texture resources (5 SRVs t0..t4 + 1 UAV u0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaaResolveDispatchPackage {
    pub available: bool,
    pub resource_count: u64,
    pub root_constant_bytes: u64,
    pub srv_count: u32,
    pub uav_register: u32,
    pub requires_shader_int64: bool,
    pub dxil_sha256: &'static str,
    pub root_signature_sha256: &'static str,
    pub descriptor_layout_sha256: &'static str,
}

impl TaaResolveDispatchPackage {
    /// The compiled package identity that matches the tracked GRX-012 offline
    /// compile evidence.
    pub fn verified_offline_package() -> TaaResolveDispatchPackage {
        TaaResolveDispatchPackage {
            available: true,
            resource_count: TAA_RESOLVE_RESOURCE_COUNT,
            root_constant_bytes: TAA_RESOLVE_ROOT_CONSTANT_BYTES,
            srv_count: 5,
            uav_register: 0,
            requires_shader_int64: true,
            dxil_sha256: TAA_RESOLVE_OFFLINE_DXIL_SHA256,
            root_signature_sha256: TAA_RESOLVE_OFFLINE_ROOT_SIGNATURE_SHA256,
            descriptor_layout_sha256: TAA_RESOLVE_OFFLINE_DESCRIPTOR_LAYOUT_SHA256,
        }
    }

    /// Verifies the compiled package is present and that its descriptor layout
    /// and artifact digests match the tracked offline compile evidence. An
    /// unavailable package maps to `compile_failed`; any layout or digest
    /// mismatch maps to `validation_failed`.
    fn verify_matches_offline_evidence(&self) -> Result<(), FallbackReason> {
        if !self.available {
            return Err(FallbackReason::CompileFailed);
        }
        if self.resource_count != TAA_RESOLVE_RESOURCE_COUNT
            || self.root_constant_bytes != TAA_RESOLVE_ROOT_CONSTANT_BYTES
            || self.srv_count != 5
            || self.uav_register != 0
            || !self.requires_shader_int64
        {
            return Err(FallbackReason::ValidationFailed);
        }
        if self.dxil_sha256 != TAA_RESOLVE_OFFLINE_DXIL_SHA256
            || self.root_signature_sha256 != TAA_RESOLVE_OFFLINE_ROOT_SIGNATURE_SHA256
            || self.descriptor_layout_sha256 != TAA_RESOLVE_OFFLINE_DESCRIPTOR_LAYOUT_SHA256
        {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }
}

/// GRX-012 gate for the `taa_resolve` pass. Template copy of the GRX-011
/// [`SsaoBlurGate`] preflight → eligibility → binding-kind → math-parity →
/// linked-dispatch chain, with every constant and digest pointing at the
/// taa_resolve package and a SIX-resource binding surface (5 SRVs + 1 UAV).
///
/// The gate starts disabled and stays disabled: the default record path (no
/// [`RXGD_CAP_TAA_RESOLVE_REAL_PASS`] arm) always returns
/// `RXGD_STATUS_FALLBACK`, and the opt-in real-pass arm fails closed with
/// `real_dispatch_path_not_linked` on the shipping feature-off bridge. No
/// estimated GPU/CPU time is ever attributed to this pass while the gate is
/// closed. Note this REMOVES the historical placeholder behaviour where
/// `RXGD_PASS_TAA_RESOLVE` was recorded with estimated timings.
#[derive(Debug)]
pub struct TaaResolveGate {
    enabled: bool,
    last_fallback_reason: Option<FallbackReason>,
    dispatch_package: TaaResolveDispatchPackage,
    /// The FIRST missing real-pass prerequisite recorded by the last opt-in
    /// real-pass attempt (the identity carried by the
    /// `RXGD_TAA_REAL_PASS_BLOCKED` diagnostic), if any.
    last_real_pass_blocked: Option<&'static str>,
    /// The diagnostic is printed once per session: the TAA resolve call site
    /// runs every frame and one machine-readable line is enough.
    real_pass_blocked_emitted: bool,
    #[cfg(feature = "d3d12-recording-shim")]
    last_dispatch_record: Option<DispatchRecord>,
}

impl TaaResolveGate {
    pub fn new() -> TaaResolveGate {
        TaaResolveGate {
            enabled: false,
            last_fallback_reason: None,
            dispatch_package: TaaResolveDispatchPackage::verified_offline_package(),
            last_real_pass_blocked: None,
            real_pass_blocked_emitted: false,
            #[cfg(feature = "d3d12-recording-shim")]
            last_dispatch_record: None,
        }
    }

    /// Take the last measured dispatch record (if any). Only available under
    /// the `d3d12-recording-shim` feature.
    #[cfg(feature = "d3d12-recording-shim")]
    pub fn take_last_dispatch_record(&mut self) -> Option<DispatchRecord> {
        self.last_dispatch_record.take()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn last_fallback_reason(&self) -> Option<FallbackReason> {
        self.last_fallback_reason
    }

    /// The FIRST missing prerequisite recorded by the last opt-in real-pass
    /// attempt, or None when no real-pass attempt was made (or a future attempt
    /// actually dispatched).
    pub fn last_real_pass_blocked(&self) -> Option<&'static str> {
        self.last_real_pass_blocked
    }

    /// Pure runtime binding preflight: validates the taa_resolve binding
    /// contract and returns the fallback reason on the first failure.
    ///
    /// The 64-bit integer shader capability (the b0 block carries i64 dims per
    /// the canonical template), exactly six texture resources in
    /// color/depth/velocity/last_velocity/history/output order, the 28-byte b0
    /// root constant block, nonzero source dimensions matching the bound
    /// `color_buffer` resource, and an `output_buffer` extent equal to the
    /// color extent (the resolve is a 1:1 full-resolution pass).
    fn check_runtime_binding_preflight(
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
    ) -> Result<(), FallbackReason> {
        if caps.flags & RXGD_CAP_SHADER_INT64 == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        if resources.len() as u64 != TAA_RESOLVE_RESOURCE_COUNT {
            return Err(FallbackReason::ValidationFailed);
        }
        if push_constants.len() as u64 != TAA_RESOLVE_ROOT_CONSTANT_BYTES {
            return Err(FallbackReason::ValidationFailed);
        }
        // All six slots (5 SRVs + 1 UAV) bind textures; a buffer never conforms.
        if resources
            .iter()
            .any(|resource| resource.resource_type != RXGD_RESOURCE_TEXTURE)
        {
            return Err(FallbackReason::ValidationFailed);
        }
        // b0 root constants: source_width/source_height are lowered as i64
        // (2 DWORDs each) followed by disocclusion_threshold/variance_dynamic/
        // reserved0; only the dimensions participate in binding preflight.
        let source_width = le_u64(&push_constants[0..8]);
        let source_height = le_u64(&push_constants[8..16]);
        if source_width == 0 || source_height == 0 {
            return Err(FallbackReason::ValidationFailed);
        }
        // color_buffer (slot 0) is the resolution reference.
        if source_width != u64::from(resources[0].width)
            || source_height != u64::from(resources[0].height)
        {
            return Err(FallbackReason::ValidationFailed);
        }
        // output_buffer (slot 5) extent == color extent (1:1 full-res resolve).
        if u64::from(resources[5].width) != source_width
            || u64::from(resources[5].height) != source_height
        {
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(())
    }

    /// GRX-012 dispatch eligibility gate: the caller must arm the taa_resolve
    /// real-pass opt-in flag, the device must advertise the 64-bit integer
    /// capability, the native D3D12 device/queue handles and all six resource
    /// handles must be non-null, and the compiled package layout/digests must
    /// still match the offline compile evidence.
    fn check_dispatch_eligibility(
        &self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        device: usize,
        queue: usize,
    ) -> Result<(), FallbackReason> {
        if caps.flags & RXGD_CAP_TAA_RESOLVE_REAL_PASS == 0 {
            return Err(FallbackReason::ManualDisabled);
        }
        if caps.flags & RXGD_CAP_SHADER_INT64 == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        if device == 0 || queue == 0 {
            return Err(FallbackReason::UnsupportedDevice);
        }
        if resources.iter().any(|resource| resource.native_handle == 0) {
            return Err(FallbackReason::ValidationFailed);
        }
        self.dispatch_package.verify_matches_offline_evidence()
    }

    /// GRX-012 kernel-binding-kind conformance check (per slot). The tracked
    /// taa_resolve kernel declares [`TAA_RESOLVE_KERNEL_RESOURCE_BINDING_KINDS`]
    /// = `["texture2d" x5, "rwtexture2d"]`; every slot binds a texture (SRV or
    /// UAV) and a buffer (`raw_buffer_view`) fails closed at any slot.
    fn check_real_pass_binding_kind(resources: &[RxGdResource]) -> Result<(), FallbackReason> {
        if resources.len() != TAA_RESOLVE_KERNEL_RESOURCE_BINDING_KINDS.len() {
            return Err(FallbackReason::ValidationFailed);
        }
        for (slot, resource) in resources.iter().enumerate() {
            // A texture provides the kind its slot declares (texture2d for the
            // SRV slots t0..t4, rwtexture2d for the UAV slot u0); a buffer
            // provides "raw_buffer_view" and never conforms.
            let provided = match resource.resource_type {
                RXGD_RESOURCE_TEXTURE => TAA_RESOLVE_KERNEL_RESOURCE_BINDING_KINDS[slot],
                _ => "raw_buffer_view",
            };
            if provided != TAA_RESOLVE_KERNEL_RESOURCE_BINDING_KINDS[slot] {
                return Err(FallbackReason::ValidationFailed);
            }
        }
        Ok(())
    }

    /// GRX-012 math-parity check. The tracked hlsl_bridge taa_resolve kernel's
    /// single full-resolution resolve subset is CPU-proven equivalent to the
    /// reference implementation in the pass `math_parity_evidence.json` (GPU
    /// observation pending a real dispatch); any other status fails closed.
    fn check_real_pass_math_parity() -> Result<(), FallbackReason> {
        if TAA_RESOLVE_KERNEL_MATH_PARITY_STATUS
            == "taa_resolve_cpu_reference_proven_pending_gpu_dispatch"
        {
            return Ok(());
        }
        Err(FallbackReason::ValidationFailed)
    }

    /// Default record path (no real-pass arm): runs the runtime binding
    /// preflight for an honest fallback reason and then keeps the gate closed
    /// with `manual_disabled`. Never returns OK and never attributes estimated
    /// GPU/CPU time. A future patch 0017 module gate calls the bridge without
    /// resource bindings (0002-level wiring), so in practice this records
    /// `validation_failed` from the preflight.
    fn record_default_fallback(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
    ) -> i32 {
        if let Err(reason) = Self::check_runtime_binding_preflight(caps, resources, push_constants)
        {
            self.last_fallback_reason = Some(reason);
            return RXGD_STATUS_FALLBACK;
        }
        self.enabled = false;
        self.last_fallback_reason = Some(FallbackReason::ManualDisabled);
        RXGD_STATUS_FALLBACK
    }

    /// GRX-012 opt-in gated REAL TAA resolve pass attempt. Order: runtime
    /// binding preflight → dispatch eligibility → per-slot kernel-binding-kind
    /// conformance → math parity → linked real dispatch path. Every failure
    /// returns `RXGD_STATUS_FALLBACK` with a recorded fallback reason and a
    /// once-per-session machine-readable `RXGD_TAA_REAL_PASS_BLOCKED
    /// first_missing_prerequisite=...` diagnostic. The real dispatch invocation
    /// is linked only under the `d3d12-recording-shim` feature; the shipping
    /// feature-off bridge fails closed with `real_dispatch_path_not_linked`.
    fn record_real_pass_attempt(
        &mut self,
        caps: RxGdCaps,
        resources: &[RxGdResource],
        push_constants: &[u8],
        device: usize,
        queue: usize,
    ) -> i32 {
        if let Err(reason) = Self::check_runtime_binding_preflight(caps, resources, push_constants)
        {
            return self.real_pass_blocked("runtime_binding_preflight_failed", reason);
        }
        if let Err(reason) = self.check_dispatch_eligibility(caps, resources, device, queue) {
            return self.real_pass_blocked("dispatch_eligibility_failed", reason);
        }
        if let Err(reason) = Self::check_real_pass_binding_kind(resources) {
            return self.real_pass_blocked("kernel_binding_kind_mismatch", reason);
        }
        if let Err(reason) = Self::check_real_pass_math_parity() {
            return self.real_pass_blocked("math_parity_not_proven", reason);
        }
        #[cfg(feature = "d3d12-recording-shim")]
        {
            return match self.attempt_real_dispatch_recording(
                resources,
                push_constants,
                device,
                queue,
            ) {
                Ok(()) => {
                    self.last_real_pass_blocked = None;
                    // Printed ONLY after a real recorded dispatch completed.
                    // Deliberately NOT an `ERROR:` line and NOT an `RXGD_DIAG`
                    // line, so runtime log audits stay clean.
                    println!("RXGD_GODOT_RUNTIME_TAA_RESOLVE_REAL_PASS recorded=1");
                    RXGD_STATUS_OK
                }
                Err(reason) => self.real_pass_blocked("real_dispatch_recording_failed", reason),
            };
        }
        #[cfg(not(feature = "d3d12-recording-shim"))]
        self.real_pass_blocked(
            "real_dispatch_path_not_linked",
            FallbackReason::CompileFailed,
        )
    }

    /// Record one real D3D12 TAA resolve dispatch through the linked recording
    /// shim's 6-resource entry (5 SRVs t0..t4 + UAV u0 + 28-byte b0 +
    /// `ceil(dims / 8)` dispatch shape; the view formats are derived from the
    /// real resource formats).
    #[cfg(feature = "d3d12-recording-shim")]
    fn attempt_real_dispatch_recording(
        &mut self,
        resources: &[RxGdResource],
        push_constants: &[u8],
        device: usize,
        queue: usize,
    ) -> Result<(), FallbackReason> {
        let color = resources[0];
        let record = d3d12_recording_shim::record_taa_resolve_dispatch(
            device,
            queue,
            resources[0].native_handle as usize, // color_buffer   (t0)
            resources[1].native_handle as usize, // depth_buffer   (t1)
            resources[2].native_handle as usize, // velocity       (t2)
            resources[3].native_handle as usize, // last_velocity  (t3)
            resources[4].native_handle as usize, // history        (t4)
            resources[5].native_handle as usize, // output         (u0)
            push_constants,
            color.width,
            color.height,
        )?;
        self.enabled = true;
        self.last_dispatch_record = Some(record);
        Ok(())
    }

    /// Records one blocked real-pass attempt: fallback status, reason, the
    /// first-missing-prerequisite identity, and (once per session) the
    /// machine-readable diagnostic line.
    fn real_pass_blocked(&mut self, prerequisite: &'static str, reason: FallbackReason) -> i32 {
        self.enabled = false;
        self.last_fallback_reason = Some(reason);
        self.last_real_pass_blocked = Some(prerequisite);
        if !self.real_pass_blocked_emitted {
            self.real_pass_blocked_emitted = true;
            // Deliberately NOT an `ERROR:` line and NOT an `RXGD_DIAG` line —
            // either would fail the runtime log audits.
            println!(
                "RXGD_TAA_REAL_PASS_BLOCKED first_missing_prerequisite={} \
                 fallback_reason={} kernel_binding={} default_enable_state=disabled",
                prerequisite,
                reason.as_str(),
                TAA_RESOLVE_KERNEL_RESOURCE_BINDING_KIND,
            );
        }
        RXGD_STATUS_FALLBACK
    }
}

impl Default for TaaResolveGate {
    fn default() -> TaaResolveGate {
        TaaResolveGate::new()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RxGdCaps {
    pub abi_version: u32,
    pub struct_size: u32,
    pub backend: u32,
    pub render_method: u32,
    pub flags: u32,
    pub vendor_id: u32,
    pub device_id: u32,
    pub adapter_luid: [u8; 8],
}

impl RxGdCaps {
    pub fn d3d12_forward_plus() -> RxGdCaps {
        RxGdCaps {
            abi_version: RXGD_ABI_VERSION,
            struct_size: size_of::<RxGdCaps>() as u32,
            backend: RXGD_BACKEND_D3D12,
            render_method: RXGD_RENDER_METHOD_FORWARD_PLUS,
            flags: 0,
            vendor_id: 0,
            device_id: 0,
            adapter_luid: [0; 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RxGdResource {
    pub abi_version: u32,
    pub struct_size: u32,
    pub resource_type: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_levels: u32,
    pub usage_flags: u64,
    pub native_handle: u64,
}

impl RxGdResource {
    pub fn texture(native_handle: u64, width: u32, height: u32, format: u32) -> RxGdResource {
        RxGdResource {
            abi_version: RXGD_ABI_VERSION,
            struct_size: size_of::<RxGdResource>() as u32,
            resource_type: RXGD_RESOURCE_TEXTURE,
            format,
            width,
            height,
            depth: 1,
            mip_levels: 1,
            usage_flags: 0,
            native_handle,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RxGdFrameStats {
    pub abi_version: u32,
    pub struct_size: u32,
    pub frame_id: u64,
    pub recorded_passes: u64,
    pub fallback_passes: u64,
    pub registered_resources: u64,
    pub gpu_time_ns: u64,
    pub cpu_record_ns: u64,
    pub last_error: i32,
}

impl RxGdFrameStats {
    fn new(session: &SessionInner, frame_id: u64) -> RxGdFrameStats {
        RxGdFrameStats {
            abi_version: RXGD_ABI_VERSION,
            struct_size: size_of::<RxGdFrameStats>() as u32,
            frame_id,
            recorded_passes: session.recorded_passes,
            fallback_passes: session.fallback_passes,
            registered_resources: session.registered_resources,
            gpu_time_ns: session.estimated_gpu_time_ns,
            cpu_record_ns: session.estimated_cpu_record_ns,
            last_error: session.last_error,
        }
    }
}

#[repr(C)]
pub struct RxGdSession {
    _private: [u8; 0],
}

#[derive(Debug)]
struct SessionInner {
    device: usize,
    queue: usize,
    caps: RxGdCaps,
    registered_resources: u64,
    recorded_passes: u64,
    fallback_passes: u64,
    estimated_gpu_time_ns: u64,
    estimated_cpu_record_ns: u64,
    last_error: i32,
    luminance_gate: LuminanceReductionGate,
    tonemap_gate: TonemapGate,
    ssao_blur_gate: SsaoBlurGate,
    taa_resolve_gate: TaaResolveGate,
}

impl SessionInner {
    fn new(device: *mut c_void, queue: *mut c_void, caps: RxGdCaps) -> SessionInner {
        SessionInner {
            device: device as usize,
            queue: queue as usize,
            caps,
            registered_resources: 0,
            recorded_passes: 0,
            fallback_passes: 0,
            estimated_gpu_time_ns: 0,
            estimated_cpu_record_ns: 0,
            last_error: RXGD_STATUS_OK,
            luminance_gate: LuminanceReductionGate::new(),
            tonemap_gate: TonemapGate::new(),
            ssao_blur_gate: SsaoBlurGate::new(),
            taa_resolve_gate: TaaResolveGate::new(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rxgd_abi_version() -> u32 {
    RXGD_ABI_VERSION
}

/// GRX-009 segment 4d: returns 1 when the bridge was built with the
/// `d3d12-recording-shim` feature (the test-only real D3D12 dispatch recording
/// path is linked), 0 otherwise. The default shipping bridge returns 0.
#[unsafe(no_mangle)]
pub extern "C" fn rxgd_dispatch_recording_shim_available() -> i32 {
    if cfg!(feature = "d3d12-recording-shim") {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rxgd_create_d3d12_session(
    device: *mut c_void,
    queue: *mut c_void,
    caps: RxGdCaps,
    out_session: *mut *mut RxGdSession,
) -> i32 {
    if out_session.is_null() || device.is_null() || queue.is_null() {
        return RXGD_E_NULL;
    }
    if !caps_supported(caps) {
        return RXGD_E_UNSUPPORTED;
    }

    let boxed = Box::new(SessionInner::new(device, queue, caps));
    // SAFETY: `out_session` was checked non-null and is owned by the caller for
    // this call. The boxed allocation is intentionally transferred as an opaque
    // handle and reclaimed by `rxgd_destroy_session`.
    unsafe {
        *out_session = Box::into_raw(boxed) as *mut RxGdSession;
    }
    RXGD_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn rxgd_register_texture(session: *mut RxGdSession, resource: RxGdResource) -> i32 {
    with_session_mut(session, |inner| {
        let rc = validate_resource(resource);
        if rc != RXGD_STATUS_OK {
            inner.last_error = rc;
            return rc;
        }
        inner.registered_resources += 1;
        RXGD_STATUS_OK
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rxgd_register_buffer(session: *mut RxGdSession, resource: RxGdResource) -> i32 {
    with_session_mut(session, |inner| {
        let rc = validate_resource(resource);
        if rc != RXGD_STATUS_OK || resource.resource_type != RXGD_RESOURCE_BUFFER {
            inner.last_error = RXGD_E_INVALID_ARGUMENT;
            return RXGD_E_INVALID_ARGUMENT;
        }
        inner.registered_resources += 1;
        RXGD_STATUS_OK
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rxgd_record_pass(
    session: *mut RxGdSession,
    pass_id: u32,
    resources: *const RxGdResource,
    resource_count: u64,
    push_constants: *const u8,
    push_constant_size: u64,
) -> i32 {
    with_session_mut(session, |inner| {
        if resource_count > MAX_RESOURCES_PER_PASS
            || push_constant_size > MAX_PUSH_CONSTANT_BYTES
            || (resource_count > 0 && resources.is_null())
            || (push_constant_size > 0 && push_constants.is_null())
        {
            inner.last_error = RXGD_E_INVALID_ARGUMENT;
            return RXGD_E_INVALID_ARGUMENT;
        }

        let mut resource_slice: &[RxGdResource] = &[];
        if resource_count > 0 {
            // SAFETY: Pointer/count were validated above. We only read the
            // fixed-size records for ABI validation and never retain pointers.
            resource_slice = unsafe { slice::from_raw_parts(resources, resource_count as usize) };
            // GRX-009 Wave 2 (hook_contract_v2 §1): the luminance pyramid arm
            // (RXGD_PASS_LUMINANCE_REDUCTION with resource_count >= 3) hands the
            // bridge a handle-only `[source, reduce[0..L-1], current, prev]`
            // texture array. Each level's extent is derived from the b0 source
            // dimensions by the planner, not carried per-resource, so those
            // entries legitimately have zero width/height. Validate them with
            // the ABI-header contract only; the pyramid binding check
            // (`check_pyramid_resource_binding`) fails the whole pyramid closed
            // (FALLBACK) on any zero handle or non-texture slot. Every other
            // pass keeps the strict width/height-bearing front-door check.
            let is_luminance_pyramid =
                pass_id == RXGD_PASS_LUMINANCE_REDUCTION && resource_count >= 3;
            for resource in resource_slice {
                let rc = if is_luminance_pyramid {
                    validate_pyramid_resource(*resource)
                } else {
                    validate_resource(*resource)
                };
                if rc != RXGD_STATUS_OK {
                    inner.last_error = rc;
                    return rc;
                }
            }
        }

        let mut push_constant_slice: &[u8] = &[];
        if push_constant_size > 0 {
            // SAFETY: Pointer/size were validated above. Only the fixed-size
            // root-constant block is read for preflight validation and no
            // pointer is retained.
            push_constant_slice =
                unsafe { slice::from_raw_parts(push_constants, push_constant_size as usize) };
        }

        if !pass_supported(pass_id) {
            inner.fallback_passes += 1;
            inner.last_error = RXGD_STATUS_FALLBACK;
            return RXGD_STATUS_FALLBACK;
        }

        if pass_id == RXGD_PASS_LUMINANCE_REDUCTION {
            let caps = inner.caps;
            let device = inner.device;
            let queue = inner.queue;
            // GRX-009 Wave 2 (hook_contract_v2): a multi-resource array
            // (resource_count >= 3) is the luminance reduction pyramid
            // `[source, reduce[0..L-1], current, prev]` (length num_levels + 2);
            // it routes into the fail-closed pyramid attempt, which parses the
            // b0 root constants and records the full reduce chain + final
            // WRITE_LUMINANCE in one submit under the recording-shim feature.
            // Exactly two resources keep the segment 4e/4h level-0 arm: the
            // opt-in real-pass flag routes to the gated real-pass attempt and
            // every other call keeps the segment 4b gated dispatch bring-up
            // path, so the 4d recording smoke and level-0 semantics are
            // unchanged.
            let rc = if resource_count >= 3 {
                inner.luminance_gate.record_pyramid_from_push_constants(
                    caps,
                    resource_slice,
                    push_constant_slice,
                    device,
                    queue,
                )
            } else if caps.flags & RXGD_CAP_LUMINANCE_REAL_PASS != 0 {
                inner.luminance_gate.record_real_pass_attempt(
                    caps,
                    resource_slice,
                    push_constant_slice,
                    device,
                    queue,
                )
            } else {
                inner.luminance_gate.record_gated_dispatch_bringup(
                    caps,
                    resource_slice,
                    push_constant_slice,
                    device,
                    queue,
                )
            };
            if rc == RXGD_STATUS_OK {
                // GRX-009 segment 4d: a real bridge dispatch was recorded through
                // the recording shim (only reachable under the
                // `d3d12-recording-shim` feature). Attribute measured CPU record
                // time; GPU timestamp is not implemented yet so no GPU time is
                // attributed (gpu_timestamp_status=not_yet).
                inner.recorded_passes += 1;
                #[cfg(feature = "d3d12-recording-shim")]
                {
                    if let Some(record) = inner.luminance_gate.take_last_dispatch_record() {
                        inner.estimated_cpu_record_ns += record.cpu_record_ns;
                    }
                }
                inner.last_error = RXGD_STATUS_OK;
                return RXGD_STATUS_OK;
            }
            inner.fallback_passes += 1;
            inner.last_error = rc;
            return rc;
        }

        if pass_id == RXGD_PASS_TONEMAP {
            let caps = inner.caps;
            let device = inner.device;
            let queue = inner.queue;
            // GRX-010: the opt-in real-pass arm routes through the gated
            // real-pass attempt (fail-closed; see RXGD_CAP_TONEMAP_REAL_PASS).
            // Every other call keeps the default fail-closed fallback path.
            // This replaces the historical placeholder estimated-timing path
            // for RXGD_PASS_TONEMAP: no estimated tonemap GPU time is
            // attributed any more.
            let rc = if caps.flags & RXGD_CAP_TONEMAP_REAL_PASS != 0 {
                inner.tonemap_gate.record_real_pass_attempt(
                    caps,
                    resource_slice,
                    push_constant_slice,
                    device,
                    queue,
                )
            } else {
                inner.tonemap_gate.record_default_fallback(
                    caps,
                    resource_slice,
                    push_constant_slice,
                )
            };
            if rc == RXGD_STATUS_OK {
                // A real bridge dispatch was recorded through the recording
                // shim (only reachable under the `d3d12-recording-shim`
                // feature). Attribute measured CPU record time; no GPU
                // timestamp is implemented (gpu_timestamp_status=not_yet).
                inner.recorded_passes += 1;
                #[cfg(feature = "d3d12-recording-shim")]
                {
                    if let Some(record) = inner.tonemap_gate.take_last_dispatch_record() {
                        inner.estimated_cpu_record_ns += record.cpu_record_ns;
                    }
                }
                inner.last_error = RXGD_STATUS_OK;
                return RXGD_STATUS_OK;
            }
            inner.fallback_passes += 1;
            inner.last_error = rc;
            return rc;
        }

        if pass_id == RXGD_PASS_SSAO_BLUR {
            let caps = inner.caps;
            let device = inner.device;
            let queue = inner.queue;
            // GRX-011: the opt-in real-pass arm routes through the gated
            // real-pass attempt (fail-closed; see
            // RXGD_CAP_SSAO_BLUR_REAL_PASS). Every other call keeps the
            // default fail-closed fallback path. This replaces the
            // historical placeholder estimated-timing path for
            // RXGD_PASS_SSAO_BLUR: no estimated ssao_blur GPU time is
            // attributed any more (RXGD_PASS_SSIL_BLUR keeps its
            // placeholder path; it is not wired in this slice).
            let rc = if caps.flags & RXGD_CAP_SSAO_BLUR_REAL_PASS != 0 {
                inner.ssao_blur_gate.record_real_pass_attempt(
                    caps,
                    resource_slice,
                    push_constant_slice,
                    device,
                    queue,
                )
            } else {
                inner.ssao_blur_gate.record_default_fallback(
                    caps,
                    resource_slice,
                    push_constant_slice,
                )
            };
            if rc == RXGD_STATUS_OK {
                // A real bridge dispatch was recorded through the recording
                // shim (only reachable under the `d3d12-recording-shim`
                // feature). Attribute measured CPU record time; no GPU
                // timestamp is implemented (gpu_timestamp_status=not_yet).
                inner.recorded_passes += 1;
                #[cfg(feature = "d3d12-recording-shim")]
                {
                    if let Some(record) = inner.ssao_blur_gate.take_last_dispatch_record() {
                        inner.estimated_cpu_record_ns += record.cpu_record_ns;
                    }
                }
                inner.last_error = RXGD_STATUS_OK;
                return RXGD_STATUS_OK;
            }
            inner.fallback_passes += 1;
            inner.last_error = rc;
            return rc;
        }

        if pass_id == RXGD_PASS_TAA_RESOLVE {
            let caps = inner.caps;
            let device = inner.device;
            let queue = inner.queue;
            // GRX-012: the opt-in real-pass arm routes through the gated
            // real-pass attempt (fail-closed; see
            // RXGD_CAP_TAA_RESOLVE_REAL_PASS). Every other call keeps the
            // default fail-closed fallback path. This replaces the historical
            // placeholder estimated-timing path for RXGD_PASS_TAA_RESOLVE: no
            // estimated taa_resolve GPU time is attributed any more.
            let rc = if caps.flags & RXGD_CAP_TAA_RESOLVE_REAL_PASS != 0 {
                inner.taa_resolve_gate.record_real_pass_attempt(
                    caps,
                    resource_slice,
                    push_constant_slice,
                    device,
                    queue,
                )
            } else {
                inner.taa_resolve_gate.record_default_fallback(
                    caps,
                    resource_slice,
                    push_constant_slice,
                )
            };
            if rc == RXGD_STATUS_OK {
                // A real bridge dispatch was recorded through the recording
                // shim (only reachable under the `d3d12-recording-shim`
                // feature). Attribute measured CPU record time; no GPU
                // timestamp is implemented (gpu_timestamp_status=not_yet).
                inner.recorded_passes += 1;
                #[cfg(feature = "d3d12-recording-shim")]
                {
                    if let Some(record) = inner.taa_resolve_gate.take_last_dispatch_record() {
                        inner.estimated_cpu_record_ns += record.cpu_record_ns;
                    }
                }
                inner.last_error = RXGD_STATUS_OK;
                return RXGD_STATUS_OK;
            }
            inner.fallback_passes += 1;
            inner.last_error = rc;
            return rc;
        }

        inner.recorded_passes += 1;
        inner.estimated_cpu_record_ns += 25_000 + resource_count * 1_000;
        inner.estimated_gpu_time_ns += estimated_pass_gpu_time(pass_id);
        inner.last_error = RXGD_STATUS_OK;
        RXGD_STATUS_OK
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rxgd_collect_timestamps(
    session: *mut RxGdSession,
    frame_id: u64,
    out_stats: *mut RxGdFrameStats,
) -> i32 {
    if out_stats.is_null() {
        return RXGD_E_NULL;
    }

    with_session_mut(session, |inner| {
        let stats = RxGdFrameStats::new(inner, frame_id);
        // SAFETY: `out_stats` is checked non-null and points to caller-owned
        // writable storage for one `RxGdFrameStats`.
        unsafe {
            *out_stats = stats;
        }
        RXGD_STATUS_OK
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rxgd_destroy_session(session: *mut RxGdSession) {
    if session.is_null() {
        return;
    }
    // SAFETY: Handles are created only by `rxgd_create_d3d12_session` using
    // `Box::into_raw`; this function is the single ownership-reclaiming entry.
    let inner = unsafe { Box::from_raw(session as *mut SessionInner) };
    // GRX-009 Wave 2: flush + tear down the cached shim session for this
    // (device, queue), printing one machine-readable `RXGD_SUMMARY pass=<id>
    // recorded=<n> fallback=<n>` line per pass the shim recorded. No-op when the
    // shim was never entered (default shipping path). Only linked under the
    // recording-shim feature.
    #[cfg(feature = "d3d12-recording-shim")]
    d3d12_recording_shim::close_shim_session(inner.device, inner.queue);
    drop(inner);
}

fn caps_supported(caps: RxGdCaps) -> bool {
    caps.abi_version == RXGD_ABI_VERSION
        && caps.struct_size == size_of::<RxGdCaps>() as u32
        && caps.backend == RXGD_BACKEND_D3D12
        && caps.render_method == RXGD_RENDER_METHOD_FORWARD_PLUS
}

fn validate_resource(resource: RxGdResource) -> i32 {
    if resource.abi_version != RXGD_ABI_VERSION
        || resource.struct_size != size_of::<RxGdResource>() as u32
        || resource.native_handle == 0
    {
        return RXGD_E_INVALID_ARGUMENT;
    }
    match resource.resource_type {
        RXGD_RESOURCE_TEXTURE => {
            if resource.width == 0 || resource.height == 0 || resource.depth == 0 {
                RXGD_E_INVALID_ARGUMENT
            } else {
                RXGD_STATUS_OK
            }
        }
        RXGD_RESOURCE_BUFFER => {
            if resource.width == 0 {
                RXGD_E_INVALID_ARGUMENT
            } else {
                RXGD_STATUS_OK
            }
        }
        _ => RXGD_E_INVALID_ARGUMENT,
    }
}

/// GRX-009 Wave 2 front-door validation for a luminance pyramid resource
/// (hook_contract_v2 §1). The pyramid array carries handle-only textures —
/// each level's extent is derived from the b0 source dimensions by the
/// planner, not carried per-resource — so the width/height/depth extents are
/// intentionally absent (zero). Only the ABI header (`abi_version`,
/// `struct_size`) is enforced at the front door; the pyramid binding contract
/// [`LuminanceReductionGate::check_pyramid_resource_binding`] re-checks the
/// non-null native handle and the texture resource type per slot and fails the
/// WHOLE pyramid closed (FALLBACK) — never a hard `RXGD_E_INVALID_ARGUMENT` —
/// on any zero handle or non-texture slot.
fn validate_pyramid_resource(resource: RxGdResource) -> i32 {
    if resource.abi_version != RXGD_ABI_VERSION
        || resource.struct_size != size_of::<RxGdResource>() as u32
    {
        return RXGD_E_INVALID_ARGUMENT;
    }
    RXGD_STATUS_OK
}

/// GRX-009 stage A3: the binding kind a runtime `RxGdResource` provides to
/// the shader at a given kernel slot, for the per-slot kernel-binding-kind
/// conformance check. Texture resources map to the SRV kind (`"texture2d"`)
/// at slot 0 (`src_luminance` = t0) and the UAV kind (`"rwtexture2d"`) at
/// slot 1 (`dst_luminance` = u0); buffer resources provide
/// `"raw_buffer_view"` regardless of slot.
fn runtime_resource_binding_kind(resource: &RxGdResource, slot: usize) -> &'static str {
    match resource.resource_type {
        RXGD_RESOURCE_TEXTURE => match slot {
            0 => "texture2d",
            1 => "rwtexture2d",
            _ => "unknown",
        },
        RXGD_RESOURCE_BUFFER => "raw_buffer_view",
        _ => "unknown",
    }
}

/// Reads a little-endian `u64` from an exactly 8-byte slice; callers must
/// have validated the surrounding block length first.
fn le_u64(bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(bytes);
    u64::from_le_bytes(buf)
}

/// Reads a little-endian `f32` from an exactly 4-byte slice; callers must
/// have validated the surrounding block length first.
fn le_f32(bytes: &[u8]) -> f32 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(bytes);
    f32::from_le_bytes(buf)
}

/// Parse the 28-byte luminance b0 root constant block shared by the level-0
/// and pyramid arms (hook_contract_v2 §3, matching Godot patch 0010's
/// marshalling): `source_width`/`source_height` as little-endian i64 (low
/// dword then high dword; the high dword must be 0 because Godot passes the
/// extent as `uint32_t`) followed by `max_luminance`/`min_luminance`/
/// `exposure_adjust` as little-endian f32. Returns `None` on any length
/// mismatch or an out-of-`u32`-range dimension so the caller can fail the
/// whole pyramid closed.
fn parse_luminance_root_constants(push_constants: &[u8]) -> Option<(u32, u32, f32, f32, f32)> {
    if push_constants.len() as u64 != LUMINANCE_ROOT_CONSTANT_BYTES {
        return None;
    }
    let source_width = le_u64(&push_constants[0..8]);
    let source_height = le_u64(&push_constants[8..16]);
    if source_width > u64::from(u32::MAX) || source_height > u64::from(u32::MAX) {
        return None;
    }
    let max_luminance = le_f32(&push_constants[16..20]);
    let min_luminance = le_f32(&push_constants[20..24]);
    let exposure_adjust = le_f32(&push_constants[24..28]);
    Some((
        source_width as u32,
        source_height as u32,
        max_luminance,
        min_luminance,
        exposure_adjust,
    ))
}

fn pass_supported(pass_id: u32) -> bool {
    matches!(
        pass_id,
        RXGD_PASS_CLUSTER_STORE
            | RXGD_PASS_SSAO_BLUR
            | RXGD_PASS_SSIL_BLUR
            | RXGD_PASS_LUMINANCE_REDUCTION
            | RXGD_PASS_TONEMAP
            | RXGD_PASS_TAA_RESOLVE
            | RXGD_PASS_PARTICLES_COPY
            | RXGD_PASS_GPU_CULLING
            | RXGD_PASS_INDIRECT_ARGS
            | RXGD_PASS_FUSED_POST_CHAIN
    )
}

fn estimated_pass_gpu_time(pass_id: u32) -> u64 {
    match pass_id {
        RXGD_PASS_CLUSTER_STORE => 80_000,
        RXGD_PASS_SSIL_BLUR => 120_000,
        // RXGD_PASS_LUMINANCE_REDUCTION is gated (GRX-009),
        // RXGD_PASS_TONEMAP is gated (GRX-010), RXGD_PASS_SSAO_BLUR is gated
        // (GRX-011), and RXGD_PASS_TAA_RESOLVE is gated (GRX-012); none of them
        // reaches the estimated-timing path while its gate is closed.
        RXGD_PASS_PARTICLES_COPY => 55_000,
        RXGD_PASS_GPU_CULLING => 140_000,
        RXGD_PASS_INDIRECT_ARGS => 60_000,
        RXGD_PASS_FUSED_POST_CHAIN => 180_000,
        _ => 0,
    }
}

fn with_session_mut<F>(session: *mut RxGdSession, f: F) -> i32
where
    F: FnOnce(&mut SessionInner) -> i32,
{
    if session.is_null() {
        return RXGD_E_NULL;
    }
    // SAFETY: Public ABI creates session handles from `Box<SessionInner>` and
    // treats `RxGdSession` as an opaque alias. Callers must not pass destroyed
    // handles; this mirrors normal C ABI ownership contracts.
    let inner = unsafe { &mut *(session as *mut SessionInner) };
    let _ = (inner.device, inner.queue, inner.caps);
    f(inner)
}

/// GRX-009 segment 4d bridge D3D12 dispatch recording shim FFI (Windows-only,
/// `d3d12-recording-shim` feature). The tracked offline luminance DXIL container
/// and RTS0 root signature are embedded here and their SHA-256 digests are
/// re-verified against the baked offline compile evidence before any recording,
/// so a stale/tampered artifact can never drive a real dispatch. The shim is
/// compiled and linked by `build.rs`.
#[cfg(feature = "d3d12-recording-shim")]
mod d3d12_recording_shim {
    use super::{
        DispatchRecord, FallbackReason, LUMINANCE_OFFLINE_DXIL_SHA256,
        LUMINANCE_OFFLINE_ROOT_SIGNATURE_SHA256, PyramidLevel, RXGD_PASS_LUMINANCE_REDUCTION,
        RXGD_PASS_SSAO_BLUR, RXGD_PASS_TAA_RESOLVE, RXGD_PASS_TONEMAP,
        SSAO_BLUR_OFFLINE_DXIL_SHA256, SSAO_BLUR_OFFLINE_ROOT_SIGNATURE_SHA256,
        TAA_RESOLVE_OFFLINE_DXIL_SHA256, TAA_RESOLVE_OFFLINE_ROOT_SIGNATURE_SHA256,
        TONEMAP_OFFLINE_DXIL_SHA256, TONEMAP_OFFLINE_ROOT_SIGNATURE_SHA256,
    };
    use core::ffi::c_void;

    /// Shim <-> Rust ABI version (kept in sync with `rxgd_luminance_record.cpp`).
    /// Bumped 1 -> 2 for the Wave 2 v2 execution model (session cache + ring
    /// allocators + descriptor-heap ring + multi-level record entry + summary).
    pub(super) const SHIM_ABI_VERSION: u32 = 2;

    /// Tracked offline luminance artifacts (segment 4i, texture-capable).
    /// Embedded so the bridge never reads them from disk at runtime; their
    /// digests are verified against the baked offline evidence before recording.
    const LUMINANCE_DXIL: &[u8] = include_bytes!(
        "../../../spike/godot-rurix/passes/luminance_reduction/artifacts/luminance_reduction.dxil"
    );
    const LUMINANCE_RTS0: &[u8] = include_bytes!(
        "../../../spike/godot-rurix/passes/luminance_reduction/artifacts/luminance_reduction.rts0.bin"
    );

    /// Wave 2 multi-level pyramid: the tracked texture-capable final-level
    /// WRITE_LUMINANCE kernel (`-D RX_WRITE_LUMINANCE=1` variant of
    /// `luminance_reduce_level.hlsl`). Binds SRV t0 (src) + SRV t1 (prev) + UAV
    /// u0 (dst) and applies `clamp(avg,min,max)` then the EMA
    /// `prev + (cur - prev) * exposure_adjust`. Embedded from the hlsl_bridge
    /// workaround artifact; its digests are re-verified before any dispatch.
    const LUMINANCE_WRITE_DXIL: &[u8] = include_bytes!(
        "../../../spike/godot-rurix/passes/luminance_reduction/artifacts/hlsl_bridge/luminance_reduce_level_write_luminance.dxil"
    );
    const LUMINANCE_WRITE_RTS0: &[u8] = include_bytes!(
        "../../../spike/godot-rurix/passes/luminance_reduction/artifacts/hlsl_bridge/root_signature_write_luminance.rts0.bin"
    );
    /// SHA-256 of the embedded WRITE_LUMINANCE artifacts (computed from the
    /// on-disk files). Re-verified before any pyramid dispatch so a stale or
    /// tampered artifact can never drive a real dispatch.
    const LUMINANCE_WRITE_DXIL_SHA256: &str =
        "ce2f247835dec6794de1f6a2b2af64b6e8c0c66017715914889f4e6b49ffca4d";
    const LUMINANCE_WRITE_RTS0_SHA256: &str =
        "436948d47664266999e8a4489fa1b406bc25982fce3b18a1e13d1a6c81714eea";

    /// Tracked offline tonemap artifacts (GRX-010, texture-capable
    /// hlsl_bridge workaround package). Same embedding + digest discipline
    /// as the luminance artifacts.
    const TONEMAP_DXIL: &[u8] =
        include_bytes!("../../../spike/godot-rurix/passes/tonemap/artifacts/tonemap.dxil");
    const TONEMAP_RTS0: &[u8] =
        include_bytes!("../../../spike/godot-rurix/passes/tonemap/artifacts/tonemap.rts0.bin");

    /// Tracked offline ssao_blur artifacts (GRX-011, texture-capable
    /// hlsl_bridge workaround package). Same embedding + digest discipline
    /// as the luminance/tonemap artifacts.
    const SSAO_BLUR_DXIL: &[u8] =
        include_bytes!("../../../spike/godot-rurix/passes/ssao_blur/artifacts/ssao_blur.dxil");
    const SSAO_BLUR_RTS0: &[u8] =
        include_bytes!("../../../spike/godot-rurix/passes/ssao_blur/artifacts/ssao_blur.rts0.bin");

    /// Tracked offline taa_resolve artifacts (GRX-012, texture-capable
    /// hlsl_bridge workaround package; 5 SRVs t0..t4 + UAV u0). Same embedding +
    /// digest discipline as the luminance/tonemap/ssao_blur artifacts.
    const TAA_RESOLVE_DXIL: &[u8] =
        include_bytes!("../../../spike/godot-rurix/passes/taa_resolve/artifacts/taa_resolve.dxil");
    const TAA_RESOLVE_RTS0: &[u8] = include_bytes!(
        "../../../spike/godot-rurix/passes/taa_resolve/artifacts/taa_resolve.rts0.bin"
    );

    #[repr(C)]
    struct RxgdRecordResult {
        fence_completed_value: u64,
        dispatch_x: u32,
        dispatch_y: u32,
        dispatch_z: u32,
        dst_width: u32,
        dst_height: u32,
        readback_checksum: u32,
        dst_first_value: f32,
        dxil_signed: i32,
        error_detail: [u8; 256],
    }

    /// One bound resource for a multi-level job (mirrors the C++ `RxgdShimResource`).
    #[repr(C)]
    struct RxgdShimResource {
        resource: *mut c_void,
        reserved0: u32,
        reserved1: u32,
    }

    /// One kernel's tracked bytes for a multi-level job (mirrors the C++
    /// `RxgdShimKernel`). `binding_count` is 2 (reduce: SRV+UAV) or 3 (write:
    /// SRV+SRV+UAV).
    #[repr(C)]
    struct RxgdShimKernel {
        dxil: *const u8,
        dxil_len: usize,
        rts0: *const u8,
        rts0_len: usize,
        binding_count: u32,
        reserved0: u32,
    }

    /// One dispatch level in a multi-level sequence (mirrors the C++
    /// `RxgdShimLevel`).
    #[repr(C)]
    struct RxgdShimLevel {
        kernel_index: u32,
        srv_index: u32,
        uav_index: u32,
        prev_index: u32,
        dispatch_x: u32,
        dispatch_y: u32,
        dispatch_z: u32,
        dst_width: u32,
        dst_height: u32,
        push_constants: [u8; 28],
    }

    unsafe extern "C" {
        fn rxgd_luminance_record_shim_abi_version() -> u32;

        fn rxgd_luminance_record_shim_session_close(
            abi_version: u32,
            device: *mut c_void,
            queue: *mut c_void,
        );

        #[allow(clippy::too_many_arguments)]
        fn rxgd_luminance_record_dispatch(
            abi_version: u32,
            pass_id: u32,
            device: *mut c_void,
            queue: *mut c_void,
            dxil: *const u8,
            dxil_len: usize,
            rts0: *const u8,
            rts0_len: usize,
            src: *mut c_void,
            dst: *mut c_void,
            push_constants: *const u8,
            push_constant_len: usize,
            src_w: u32,
            src_h: u32,
            dst_w: u32,
            dst_h: u32,
            out: *mut RxgdRecordResult,
        ) -> i32;

        #[allow(clippy::too_many_arguments)]
        fn rxgd_luminance_record_levels(
            abi_version: u32,
            pass_id: u32,
            device: *mut c_void,
            queue: *mut c_void,
            kernels: *const RxgdShimKernel,
            kernel_count: u32,
            resources: *const RxgdShimResource,
            resource_count: u32,
            levels: *const RxgdShimLevel,
            level_count: u32,
            readback: u32,
            out: *mut RxgdRecordResult,
        ) -> i32;

        // GRX-012: 5-SRV (t0..t4) + 1-UAV (u0) single-dispatch entry for the
        // taa_resolve kernel (test-only readback mode). Additive; the existing
        // 2-resource / multi-level entries are unchanged.
        #[allow(clippy::too_many_arguments)]
        fn rxgd_taa_resolve_record_dispatch(
            abi_version: u32,
            pass_id: u32,
            device: *mut c_void,
            queue: *mut c_void,
            dxil: *const u8,
            dxil_len: usize,
            rts0: *const u8,
            rts0_len: usize,
            color: *mut c_void,
            depth: *mut c_void,
            velocity: *mut c_void,
            last_velocity: *mut c_void,
            history: *mut c_void,
            output: *mut c_void,
            push_constants: *const u8,
            push_constant_len: usize,
            width: u32,
            height: u32,
            out: *mut RxgdRecordResult,
        ) -> i32;
    }

    /// Flush + tear down the cached shim session for `(device, queue)`, printing
    /// one `RXGD_SUMMARY pass=<id> recorded=<n> fallback=<n>` line per pass the
    /// session recorded. Called from `rxgd_destroy_session` under the feature.
    /// Safe to call for a `(device, queue)` that never recorded (no-op).
    pub fn close_shim_session(device: usize, queue: usize) {
        // SAFETY: the close entry only frees the shim-owned cache for the
        // `(device, queue)` key and prints the summary; it dereferences neither
        // pointer as a D3D12 object and retains nothing past the call.
        unsafe {
            rxgd_luminance_record_shim_session_close(
                SHIM_ABI_VERSION,
                device as *mut c_void,
                queue as *mut c_void,
            );
        }
    }

    /// Record one real luminance compute dispatch through the linked shim.
    ///
    /// `device`/`queue`/`src`/`dst` are real D3D12 handle pointer values passed
    /// as `usize` (validated non-null by the dispatch eligibility gate). Returns
    /// the measured [`DispatchRecord`] on success, or a fallback reason if the
    /// embedded artifacts do not hash to the offline digests, the shim ABI does
    /// not match, or the shim reports any D3D12 failure.
    #[allow(clippy::too_many_arguments)]
    pub fn record_luminance_dispatch(
        device: usize,
        queue: usize,
        src: usize,
        dst: usize,
        push_constants: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
    ) -> Result<DispatchRecord, FallbackReason> {
        record_texture_pass_dispatch(
            RXGD_PASS_LUMINANCE_REDUCTION,
            LUMINANCE_DXIL,
            LUMINANCE_RTS0,
            LUMINANCE_OFFLINE_DXIL_SHA256,
            LUMINANCE_OFFLINE_ROOT_SIGNATURE_SHA256,
            device,
            queue,
            src,
            dst,
            push_constants,
            src_w,
            src_h,
            dst_w,
            dst_h,
        )
    }

    /// GRX-010: record one real tonemap compute dispatch through the same
    /// parameterized texture-pass shim entry point. The shim contract (SRV
    /// t0 + UAV u0 descriptor table, 28-byte b0 root constants,
    /// `ceil(src_dims / 8)` thread groups, view formats derived from the
    /// real resource formats) matches the tonemap kernel exactly (dst
    /// extent == src extent for the 1:1 full-resolution pass).
    #[allow(clippy::too_many_arguments)]
    pub fn record_tonemap_dispatch(
        device: usize,
        queue: usize,
        src: usize,
        dst: usize,
        push_constants: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
    ) -> Result<DispatchRecord, FallbackReason> {
        record_texture_pass_dispatch(
            RXGD_PASS_TONEMAP,
            TONEMAP_DXIL,
            TONEMAP_RTS0,
            TONEMAP_OFFLINE_DXIL_SHA256,
            TONEMAP_OFFLINE_ROOT_SIGNATURE_SHA256,
            device,
            queue,
            src,
            dst,
            push_constants,
            src_w,
            src_h,
            dst_w,
            dst_h,
        )
    }

    /// GRX-011: record one real SSAO blur compute dispatch through the same
    /// parameterized texture-pass shim entry point. The shim contract (SRV
    /// t0 + UAV u0 descriptor table, 28-byte b0 root constants,
    /// `ceil(src_dims / 8)` thread groups, view formats derived from the
    /// real resource formats) matches the ssao_blur kernel exactly (dst
    /// extent == src extent for the 1:1 ping-pong blur pass).
    #[allow(clippy::too_many_arguments)]
    pub fn record_ssao_blur_dispatch(
        device: usize,
        queue: usize,
        src: usize,
        dst: usize,
        push_constants: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
    ) -> Result<DispatchRecord, FallbackReason> {
        record_texture_pass_dispatch(
            RXGD_PASS_SSAO_BLUR,
            SSAO_BLUR_DXIL,
            SSAO_BLUR_RTS0,
            SSAO_BLUR_OFFLINE_DXIL_SHA256,
            SSAO_BLUR_OFFLINE_ROOT_SIGNATURE_SHA256,
            device,
            queue,
            src,
            dst,
            push_constants,
            src_w,
            src_h,
            dst_w,
            dst_h,
        )
    }

    /// GRX-012: record one real TAA resolve compute dispatch through the shim's
    /// 6-resource entry (5 SRVs t0..t4 + UAV u0). Verifies the embedded
    /// taa_resolve artifact digests against the baked offline evidence, then
    /// hands the six real `ID3D12Resource*` handles + the pass's DXIL/RTS0 bytes
    /// to the shim. Returns the measured [`DispatchRecord`] on success, or a
    /// fallback reason if the embedded artifacts do not hash to the offline
    /// digests, the shim ABI does not match, or the shim reports a D3D12
    /// failure.
    #[allow(clippy::too_many_arguments)]
    pub fn record_taa_resolve_dispatch(
        device: usize,
        queue: usize,
        color: usize,
        depth: usize,
        velocity: usize,
        last_velocity: usize,
        history: usize,
        output: usize,
        push_constants: &[u8],
        width: u32,
        height: u32,
    ) -> Result<DispatchRecord, FallbackReason> {
        // Artifact integrity: the embedded bytes must hash to the baked offline
        // compile evidence digests, or the runtime binding does not correspond
        // to the tracked compiled package and must not drive a real dispatch.
        if sha256_hex(TAA_RESOLVE_DXIL) != TAA_RESOLVE_OFFLINE_DXIL_SHA256
            || sha256_hex(TAA_RESOLVE_RTS0) != TAA_RESOLVE_OFFLINE_ROOT_SIGNATURE_SHA256
        {
            return Err(FallbackReason::ValidationFailed);
        }
        // SAFETY: the shim ABI query takes no arguments and only returns a
        // compile-time constant; it dereferences no pointer.
        let shim_abi = unsafe { rxgd_luminance_record_shim_abi_version() };
        if shim_abi != SHIM_ABI_VERSION {
            return Err(FallbackReason::ValidationFailed);
        }

        let mut out = RxgdRecordResult {
            fence_completed_value: 0,
            dispatch_x: 0,
            dispatch_y: 0,
            dispatch_z: 0,
            dst_width: 0,
            dst_height: 0,
            readback_checksum: 0,
            dst_first_value: 0.0,
            dxil_signed: 0,
            error_detail: [0u8; 256],
        };
        let start = std::time::Instant::now();
        // SAFETY: the DXIL/RTS0 slices are embedded read-only data with their
        // true `len()`, `push_constants` is a caller-owned read-only slice with
        // its true `len()`, and `out` is an exclusive local. The
        // device/queue/resource pointer values were validated non-null by the
        // dispatch eligibility gate and originate from real D3D12 objects. The
        // shim honours the versioned ABI (first arg checked against
        // `SHIM_ABI_VERSION`), only reads the byte inputs, records one dispatch,
        // writes `out`, and retains no pointer past the call.
        let rc = unsafe {
            rxgd_taa_resolve_record_dispatch(
                SHIM_ABI_VERSION,
                RXGD_PASS_TAA_RESOLVE,
                device as *mut c_void,
                queue as *mut c_void,
                TAA_RESOLVE_DXIL.as_ptr(),
                TAA_RESOLVE_DXIL.len(),
                TAA_RESOLVE_RTS0.as_ptr(),
                TAA_RESOLVE_RTS0.len(),
                color as *mut c_void,
                depth as *mut c_void,
                velocity as *mut c_void,
                last_velocity as *mut c_void,
                history as *mut c_void,
                output as *mut c_void,
                push_constants.as_ptr(),
                push_constants.len(),
                width,
                height,
                &mut out,
            )
        };
        let cpu_record_ns = start.elapsed().as_nanos() as u64;
        if rc != 0 {
            let detail_len = out
                .error_detail
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(out.error_detail.len());
            let detail = String::from_utf8_lossy(&out.error_detail[..detail_len]);
            eprintln!(
                "RXGD_SHIM_DIAG rxgd_taa_resolve_record_dispatch rc={rc} dxil_signed={} detail=\"{detail}\"",
                out.dxil_signed
            );
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(DispatchRecord {
            fence_completed_value: out.fence_completed_value,
            dispatch: (out.dispatch_x, out.dispatch_y, out.dispatch_z),
            dst_width: out.dst_width,
            dst_height: out.dst_height,
            readback_checksum: out.readback_checksum,
            dst_first_value: out.dst_first_value,
            dxil_signed: out.dxil_signed != 0,
            cpu_record_ns,
        })
    }

    /// Wave 2 multi-level pyramid: record the full reduce chain + final
    /// WRITE_LUMINANCE level in ONE submit through the shim's multi-level entry.
    ///
    /// `resource_handles` is the hook-contract array `[source, reduce[0..L-1],
    /// current, prev]` (`L = levels.len() - 1`) as `ID3D12Resource*` pointer
    /// values (as `usize`), all validated non-zero by the caller. `levels` is
    /// the planned pyramid ([`super::plan_luminance_pyramid_levels`]). Each
    /// reduce level uses the SRV+UAV kernel; the final level uses the SRV+SRV+UAV
    /// WRITE_LUMINANCE kernel and reads `prev` for the EMA. `readback` selects
    /// the test-only readback (fence wait + marker) vs the production no-wait
    /// path.
    ///
    /// ONE-FRAME LATENCY: when driven from a Godot runtime hook, `source`/`prev`
    /// carry the previous frame's content (Godot has not yet submitted this
    /// frame); the EMA feedback makes a 1-frame delay defensible. This function
    /// records exactly what it is handed (see hook_contract_v2.md).
    #[allow(clippy::too_many_arguments)]
    pub fn record_luminance_pyramid_dispatch(
        device: usize,
        queue: usize,
        resource_handles: &[usize],
        levels: &[PyramidLevel],
        max_luminance: f32,
        min_luminance: f32,
        exposure_adjust: f32,
        readback: bool,
    ) -> Result<DispatchRecord, FallbackReason> {
        // Artifact integrity for BOTH kernels (reduce + WRITE_LUMINANCE): the
        // embedded bytes must hash to the baked digests, or the runtime binding
        // does not correspond to the tracked compiled package.
        if sha256_hex(LUMINANCE_DXIL) != LUMINANCE_OFFLINE_DXIL_SHA256
            || sha256_hex(LUMINANCE_RTS0) != LUMINANCE_OFFLINE_ROOT_SIGNATURE_SHA256
            || sha256_hex(LUMINANCE_WRITE_DXIL) != LUMINANCE_WRITE_DXIL_SHA256
            || sha256_hex(LUMINANCE_WRITE_RTS0) != LUMINANCE_WRITE_RTS0_SHA256
        {
            return Err(FallbackReason::ValidationFailed);
        }
        // SAFETY: the shim ABI query takes no arguments and only returns a
        // compile-time constant; it dereferences no pointer.
        let shim_abi = unsafe { rxgd_luminance_record_shim_abi_version() };
        if shim_abi != SHIM_ABI_VERSION {
            return Err(FallbackReason::ValidationFailed);
        }
        let num_levels = levels.len();
        if num_levels == 0 || resource_handles.len() != num_levels + 2 {
            return Err(FallbackReason::ValidationFailed);
        }

        // Kernel table: index 0 = reduce (SRV t0 + UAV u0), index 1 =
        // WRITE_LUMINANCE (SRV t0 + SRV t1 + UAV u0).
        let kernels = [
            RxgdShimKernel {
                dxil: LUMINANCE_DXIL.as_ptr(),
                dxil_len: LUMINANCE_DXIL.len(),
                rts0: LUMINANCE_RTS0.as_ptr(),
                rts0_len: LUMINANCE_RTS0.len(),
                binding_count: 2,
                reserved0: 0,
            },
            RxgdShimKernel {
                dxil: LUMINANCE_WRITE_DXIL.as_ptr(),
                dxil_len: LUMINANCE_WRITE_DXIL.len(),
                rts0: LUMINANCE_WRITE_RTS0.as_ptr(),
                rts0_len: LUMINANCE_WRITE_RTS0.len(),
                binding_count: 3,
                reserved0: 0,
            },
        ];
        let resources: Vec<RxgdShimResource> = resource_handles
            .iter()
            .map(|&h| RxgdShimResource {
                resource: h as *mut c_void,
                reserved0: 0,
                reserved1: 0,
            })
            .collect();
        let mut shim_levels: Vec<RxgdShimLevel> = Vec::with_capacity(num_levels);
        for (i, lvl) in levels.iter().enumerate() {
            let is_final = i + 1 == num_levels;
            let kernel_index = if is_final { 1 } else { 0 };
            // Resource array layout: [source(0), reduce[0..L-1](1..=L),
            // current(L+1), prev(L+2)]. Level i reads index i (source or the
            // previous level's output) and writes index i+1, except the final
            // level writes `current` (num_levels) and reads `prev` (num_levels+1).
            let srv_index = i as u32;
            let uav_index = if is_final {
                num_levels as u32
            } else {
                (i + 1) as u32
            };
            let prev_index = (num_levels + 1) as u32;
            // [numthreads(8,8,1)], one thread per dest texel: dispatch
            // ceil(dst / 8) thread groups over the native floor-sized
            // destination (`dst = MAX(src/8, 1)`). This launches exactly enough
            // threads to cover every in-bounds floor texel; the kernel's own
            // ceil-based write guard drops the trailing partial tile against the
            // floor buffer, matching native's `dispatch_threads(source)` +
            // floor-buffer edge-drop bit-for-bit.
            let dispatch_x = ((lvl.dst_width + 7) / 8).max(1);
            let dispatch_y = ((lvl.dst_height + 7) / 8).max(1);
            let mut pc = [0u8; 28];
            pc[0..8].copy_from_slice(&(u64::from(lvl.src_width)).to_le_bytes());
            pc[8..16].copy_from_slice(&(u64::from(lvl.src_height)).to_le_bytes());
            pc[16..20].copy_from_slice(&max_luminance.to_le_bytes());
            pc[20..24].copy_from_slice(&min_luminance.to_le_bytes());
            pc[24..28].copy_from_slice(&exposure_adjust.to_le_bytes());
            shim_levels.push(RxgdShimLevel {
                kernel_index,
                srv_index,
                uav_index,
                prev_index,
                dispatch_x,
                dispatch_y,
                dispatch_z: 1,
                dst_width: lvl.dst_width,
                dst_height: lvl.dst_height,
                push_constants: pc,
            });
        }

        let mut out = RxgdRecordResult {
            fence_completed_value: 0,
            dispatch_x: 0,
            dispatch_y: 0,
            dispatch_z: 0,
            dst_width: 0,
            dst_height: 0,
            readback_checksum: 0,
            dst_first_value: 0.0,
            dxil_signed: 0,
            error_detail: [0u8; 256],
        };
        let start = std::time::Instant::now();
        // SAFETY: `kernels`/`resources`/`shim_levels` are exclusive local arrays
        // whose element pointers reference embedded read-only artifact bytes and
        // caller-validated non-null resource handles; each pointer/length pair is
        // valid for the call. `out` is an exclusive local. The device/queue
        // pointer values were validated non-null by the caller. The shim honours
        // the versioned ABI (first arg checked against `SHIM_ABI_VERSION`), only
        // reads the byte inputs, records the dispatches, writes `out`, and
        // retains no pointer past the call.
        let rc = unsafe {
            rxgd_luminance_record_levels(
                SHIM_ABI_VERSION,
                RXGD_PASS_LUMINANCE_REDUCTION,
                device as *mut c_void,
                queue as *mut c_void,
                kernels.as_ptr(),
                kernels.len() as u32,
                resources.as_ptr(),
                resources.len() as u32,
                shim_levels.as_ptr(),
                shim_levels.len() as u32,
                if readback { 1 } else { 0 },
                &mut out,
            )
        };
        let cpu_record_ns = start.elapsed().as_nanos() as u64;
        if rc != 0 {
            let detail_len = out
                .error_detail
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(out.error_detail.len());
            let detail = String::from_utf8_lossy(&out.error_detail[..detail_len]);
            eprintln!(
                "RXGD_SHIM_DIAG rxgd_luminance_record_levels rc={rc} dxil_signed={} detail=\"{detail}\"",
                out.dxil_signed
            );
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(DispatchRecord {
            fence_completed_value: out.fence_completed_value,
            dispatch: (out.dispatch_x, out.dispatch_y, out.dispatch_z),
            dst_width: out.dst_width,
            dst_height: out.dst_height,
            readback_checksum: out.readback_checksum,
            dst_first_value: out.dst_first_value,
            dxil_signed: out.dxil_signed != 0,
            cpu_record_ns,
        })
    }

    /// Shared parameterized texture-pass recording path: verifies the
    /// embedded artifact digests for the requested pass package, then hands
    /// the real handles plus the pass's DXIL/RTS0 bytes to the shim.
    #[allow(clippy::too_many_arguments)]
    fn record_texture_pass_dispatch(
        pass_id: u32,
        dxil_bytes: &'static [u8],
        rts0_bytes: &'static [u8],
        expected_dxil_sha256: &str,
        expected_rts0_sha256: &str,
        device: usize,
        queue: usize,
        src: usize,
        dst: usize,
        push_constants: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
    ) -> Result<DispatchRecord, FallbackReason> {
        // Artifact integrity: the embedded bytes must hash to the baked offline
        // compile evidence digests, or the runtime binding does not correspond
        // to the tracked compiled package and must not drive a real dispatch.
        if sha256_hex(dxil_bytes) != expected_dxil_sha256
            || sha256_hex(rts0_bytes) != expected_rts0_sha256
        {
            return Err(FallbackReason::ValidationFailed);
        }

        // SAFETY: the shim ABI query takes no arguments and only returns a
        // compile-time constant; it dereferences no pointer.
        let shim_abi = unsafe { rxgd_luminance_record_shim_abi_version() };
        if shim_abi != SHIM_ABI_VERSION {
            return Err(FallbackReason::ValidationFailed);
        }

        let mut out = RxgdRecordResult {
            fence_completed_value: 0,
            dispatch_x: 0,
            dispatch_y: 0,
            dispatch_z: 0,
            dst_width: 0,
            dst_height: 0,
            readback_checksum: 0,
            dst_first_value: 0.0,
            dxil_signed: 0,
            error_detail: [0u8; 256],
        };
        let start = std::time::Instant::now();
        // SAFETY: all pointer/length pairs are valid for this call — the DXIL and
        // RTS0 slices are embedded read-only data with their true `len()`,
        // `push_constants` is a caller-owned read-only slice with its true
        // `len()`, and `out` is an exclusive local `RxgdRecordResult`. The
        // device/queue/src/dst pointer values were validated non-null by the
        // dispatch eligibility gate and originate from the harness's real D3D12
        // objects. The shim honours the versioned ABI (first arg checked
        // against `SHIM_ABI_VERSION`), only reads the byte inputs, records one
        // dispatch, writes `out`, and retains no pointer past the call.
        let rc = unsafe {
            rxgd_luminance_record_dispatch(
                SHIM_ABI_VERSION,
                pass_id,
                device as *mut c_void,
                queue as *mut c_void,
                dxil_bytes.as_ptr(),
                dxil_bytes.len(),
                rts0_bytes.as_ptr(),
                rts0_bytes.len(),
                src as *mut c_void,
                dst as *mut c_void,
                push_constants.as_ptr(),
                push_constants.len(),
                src_w,
                src_h,
                dst_w,
                dst_h,
                &mut out,
            )
        };
        let cpu_record_ns = start.elapsed().as_nanos() as u64;
        if rc != 0 {
            // Surface the shim's D3D12 failure detail so the Godot-runtime
            // recording smoke can diagnose why a real dispatch fell back
            // instead of silently mapping every failure to ValidationFailed.
            let detail_len = out
                .error_detail
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(out.error_detail.len());
            let detail = String::from_utf8_lossy(&out.error_detail[..detail_len]);
            eprintln!(
                "RXGD_SHIM_DIAG rxgd_luminance_record_dispatch rc={rc} dxil_signed={} detail=\"{detail}\"",
                out.dxil_signed
            );
            return Err(FallbackReason::ValidationFailed);
        }
        Ok(DispatchRecord {
            fence_completed_value: out.fence_completed_value,
            dispatch: (out.dispatch_x, out.dispatch_y, out.dispatch_z),
            dst_width: out.dst_width,
            dst_height: out.dst_height,
            readback_checksum: out.readback_checksum,
            dst_first_value: out.dst_first_value,
            dxil_signed: out.dxil_signed != 0,
            cpu_record_ns,
        })
    }

    /// Compact SHA-256 (FIPS 180-4) over `data`, returned as lowercase hex. Used
    /// only to re-verify the embedded artifact digests; no external dependency.
    fn sha256_hex(data: &[u8]) -> String {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let bit_len = (data.len() as u64).wrapping_mul(8);
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());
        for chunk in msg.chunks_exact(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    chunk[i * 4],
                    chunk[i * 4 + 1],
                    chunk[i * 4 + 2],
                    chunk[i * 4 + 3],
                ]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
                (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let t1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(hh);
        }
        let mut out = String::with_capacity(64);
        for v in h {
            out.push_str(&format!("{v:08x}"));
        }
        out
    }

    #[cfg(test)]
    mod tests {
        use super::LUMINANCE_OFFLINE_ROOT_SIGNATURE_SHA256;
        use super::{LUMINANCE_DXIL, LUMINANCE_OFFLINE_DXIL_SHA256, LUMINANCE_RTS0, sha256_hex};

        #[test]
        fn sha256_matches_known_vectors() {
            assert_eq!(
                sha256_hex(b""),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            );
            assert_eq!(
                sha256_hex(b"abc"),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            );
        }

        #[test]
        fn embedded_artifacts_match_offline_digests() {
            assert_eq!(sha256_hex(LUMINANCE_DXIL), LUMINANCE_OFFLINE_DXIL_SHA256);
            assert_eq!(
                sha256_hex(LUMINANCE_RTS0),
                LUMINANCE_OFFLINE_ROOT_SIGNATURE_SHA256
            );
        }

        #[test]
        fn embedded_tonemap_artifacts_match_offline_digests() {
            use super::super::{
                TONEMAP_OFFLINE_DXIL_SHA256, TONEMAP_OFFLINE_ROOT_SIGNATURE_SHA256,
            };
            use super::{TONEMAP_DXIL, TONEMAP_RTS0};
            assert_eq!(sha256_hex(TONEMAP_DXIL), TONEMAP_OFFLINE_DXIL_SHA256);
            assert_eq!(
                sha256_hex(TONEMAP_RTS0),
                TONEMAP_OFFLINE_ROOT_SIGNATURE_SHA256
            );
        }

        #[test]
        fn embedded_ssao_blur_artifacts_match_offline_digests() {
            use super::super::{
                SSAO_BLUR_OFFLINE_DXIL_SHA256, SSAO_BLUR_OFFLINE_ROOT_SIGNATURE_SHA256,
            };
            use super::{SSAO_BLUR_DXIL, SSAO_BLUR_RTS0};
            assert_eq!(sha256_hex(SSAO_BLUR_DXIL), SSAO_BLUR_OFFLINE_DXIL_SHA256);
            assert_eq!(
                sha256_hex(SSAO_BLUR_RTS0),
                SSAO_BLUR_OFFLINE_ROOT_SIGNATURE_SHA256
            );
        }

        #[test]
        fn embedded_taa_resolve_artifacts_match_offline_digests() {
            use super::super::{
                TAA_RESOLVE_OFFLINE_DXIL_SHA256, TAA_RESOLVE_OFFLINE_ROOT_SIGNATURE_SHA256,
            };
            use super::{TAA_RESOLVE_DXIL, TAA_RESOLVE_RTS0};
            assert_eq!(
                sha256_hex(TAA_RESOLVE_DXIL),
                TAA_RESOLVE_OFFLINE_DXIL_SHA256
            );
            assert_eq!(
                sha256_hex(TAA_RESOLVE_RTS0),
                TAA_RESOLVE_OFFLINE_ROOT_SIGNATURE_SHA256
            );
        }

        #[test]
        fn embedded_write_luminance_artifacts_match_digests() {
            use super::{
                LUMINANCE_WRITE_DXIL, LUMINANCE_WRITE_DXIL_SHA256, LUMINANCE_WRITE_RTS0,
                LUMINANCE_WRITE_RTS0_SHA256,
            };
            // Digests computed from the on-disk hlsl_bridge WRITE_LUMINANCE
            // artifacts; guards against embedding a stale/tampered kernel.
            assert_eq!(
                sha256_hex(LUMINANCE_WRITE_DXIL),
                LUMINANCE_WRITE_DXIL_SHA256
            );
            assert_eq!(
                sha256_hex(LUMINANCE_WRITE_RTS0),
                LUMINANCE_WRITE_RTS0_SHA256
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_ptr(n: usize) -> *mut c_void {
        n as *mut c_void
    }

    fn create_session() -> *mut RxGdSession {
        create_session_with_caps(RxGdCaps::d3d12_forward_plus())
    }

    fn create_session_with_caps(caps: RxGdCaps) -> *mut RxGdSession {
        let mut session = core::ptr::null_mut();
        let rc = rxgd_create_d3d12_session(fake_ptr(1), fake_ptr(2), caps, &mut session);
        assert_eq!(rc, RXGD_STATUS_OK);
        assert!(!session.is_null());
        session
    }

    fn luminance_push_constants(
        source_width: u64,
        source_height: u64,
        max_luminance: f32,
        min_luminance: f32,
        exposure_adjust: f32,
    ) -> [u8; LUMINANCE_ROOT_CONSTANT_BYTES as usize] {
        let mut bytes = [0u8; LUMINANCE_ROOT_CONSTANT_BYTES as usize];
        bytes[0..8].copy_from_slice(&source_width.to_le_bytes());
        bytes[8..16].copy_from_slice(&source_height.to_le_bytes());
        bytes[16..20].copy_from_slice(&max_luminance.to_le_bytes());
        bytes[20..24].copy_from_slice(&min_luminance.to_le_bytes());
        bytes[24..28].copy_from_slice(&exposure_adjust.to_le_bytes());
        bytes
    }

    fn zeroed_stats() -> RxGdFrameStats {
        RxGdFrameStats {
            abi_version: 0,
            struct_size: 0,
            frame_id: 0,
            recorded_passes: 0,
            fallback_passes: 0,
            registered_resources: 0,
            gpu_time_ns: 0,
            cpu_record_ns: 0,
            last_error: 0,
        }
    }

    /// Caps advertising both the 64-bit integer capability and the segment 4b
    /// gated dispatch bring-up opt-in flag.
    fn luminance_dispatch_optin_caps() -> RxGdCaps {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64 | RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP;
        caps
    }

    /// Caps additionally arming the segment 4h opt-in real-pass attempt on top
    /// of the bring-up opt-in (the segment 4h harness project enables both
    /// default-false settings explicitly).
    fn luminance_real_pass_optin_caps() -> RxGdCaps {
        let mut caps = luminance_dispatch_optin_caps();
        caps.flags |= RXGD_CAP_LUMINANCE_REAL_PASS;
        caps
    }

    /// A fully valid segment 4e-shaped luminance binding: two texture
    /// resources with non-null native handles plus coherent b0 constants.
    fn valid_luminance_binding() -> (
        [RxGdResource; 2],
        [u8; LUMINANCE_ROOT_CONSTANT_BYTES as usize],
    ) {
        let resources = [
            RxGdResource::texture(71, 1920, 1080, 87),
            RxGdResource::texture(72, 240, 135, 114),
        ];
        let push_constants = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        (resources, push_constants)
    }

    #[test]
    fn real_pass_cap_flag_is_reserved_bit_three() {
        assert_eq!(RXGD_CAP_LUMINANCE_REAL_PASS, 1 << 3);
        // ABI stays v1: the flag reuses the existing RxGdCaps.flags field.
        assert_eq!(rxgd_abi_version(), RXGD_ABI_VERSION);
    }

    #[test]
    fn real_pass_binding_kind_check_accepts_texture_slots_only() {
        // Stage A3: the tracked hlsl_bridge kernel declares per-slot
        // ["texture2d", "rwtexture2d"] (src_luminance = SRV t0,
        // dst_luminance = UAV u0). Two texture resources in slot order
        // conform; buffer resources (raw_buffer_view) no longer conform at
        // any slot, and a mixed or wrong-arity binding fails closed.
        let texture = RxGdResource::texture(81, 64, 64, 87);
        let buffer = RxGdResource {
            resource_type: RXGD_RESOURCE_BUFFER,
            ..texture
        };
        // Two texture resources (slot 0 SRV, slot 1 UAV) conform.
        assert_eq!(
            LuminanceReductionGate::check_real_pass_binding_kind(&[texture, texture]),
            Ok(())
        );
        // Wrong arity fails closed (the kernel declares exactly 2 slots).
        assert_eq!(
            LuminanceReductionGate::check_real_pass_binding_kind(&[texture]),
            Err(FallbackReason::ValidationFailed)
        );
        // Buffer resources no longer conform to the texture-capable kernel.
        assert_eq!(
            LuminanceReductionGate::check_real_pass_binding_kind(&[buffer, buffer]),
            Err(FallbackReason::ValidationFailed)
        );
        // Mixed bindings fail closed.
        assert_eq!(
            LuminanceReductionGate::check_real_pass_binding_kind(&[texture, buffer]),
            Err(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            LuminanceReductionGate::check_real_pass_binding_kind(&[buffer, texture]),
            Err(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn real_pass_math_parity_gate_allows_cpu_proven_level_0() {
        // Stage A4: level-0 math parity is CPU-proven
        // (math_parity_evidence.json, pending GPU dispatch), so the math
        // gate no longer blocks the level-0 real-pass attempt.
        assert_eq!(
            LUMINANCE_KERNEL_MATH_PARITY_STATUS,
            "level0_cpu_reference_proven_pending_gpu_dispatch"
        );
        assert_eq!(
            LuminanceReductionGate::check_real_pass_math_parity(),
            Ok(())
        );
    }

    /// Stage A5: with the tracked texture-capable hlsl_bridge package a
    /// fully valid, fully opted-in real-pass attempt passes every software
    /// gate (preflight, eligibility, per-slot binding kind, CPU-proven
    /// level-0 math parity) and reaches the linked-dispatch boundary. In
    /// the shipping feature-off build no dispatch path is compiled in, so
    /// the attempt still fails closed with `real_dispatch_path_not_linked`
    /// and compile_failed telemetry. (Under the d3d12-recording-shim
    /// feature the same attempt would invoke the real recording shim, which
    /// requires real D3D12 handles, so this test is feature-off only.)
    #[cfg(not(feature = "d3d12-recording-shim"))]
    #[test]
    fn luminance_real_pass_optin_blocks_on_missing_dispatch_path() {
        let (resources, push_constants) = valid_luminance_binding();
        let session = create_session_with_caps(luminance_real_pass_optin_caps());
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                resources.as_ptr(),
                resources.len() as u64,
                push_constants.as_ptr(),
                push_constants.len() as u64,
            ),
            RXGD_STATUS_FALLBACK
        );
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 12, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        rxgd_destroy_session(session);

        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_real_pass_attempt(
            luminance_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::CompileFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("real_dispatch_path_not_linked")
        );
    }

    #[test]
    fn luminance_real_pass_with_buffer_resources_blocks_on_binding_kind() {
        // Stage A3: buffer resources (RXGD_RESOURCE_BUFFER →
        // raw_buffer_view) no longer conform to the texture-capable tracked
        // kernel at any slot. Via `record_real_pass_attempt` a buffer
        // resource fails at the segment 4a preflight first (the Godot
        // runtime binding contract requires textures), so the
        // first-missing-prerequisite is `runtime_binding_preflight_failed`;
        // the direct binding-kind check documents the per-slot rejection.
        let texture = RxGdResource::texture(91, 1920, 1080, 87);
        let buffer = RxGdResource {
            resource_type: RXGD_RESOURCE_BUFFER,
            ..texture
        };

        // Direct: buffer resources are rejected by the per-slot check.
        assert_eq!(
            LuminanceReductionGate::check_real_pass_binding_kind(&[buffer, buffer]),
            Err(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            LuminanceReductionGate::check_real_pass_binding_kind(&[texture, buffer]),
            Err(FallbackReason::ValidationFailed)
        );

        // End-to-end: with buffer resources the preflight catches them first
        // (the Godot runtime binding contract requires textures), so the
        // first-missing-prerequisite is `runtime_binding_preflight_failed`.
        // This documents the actual order: preflight → eligibility →
        // binding_kind → math_parity → linked dispatch path.
        let (_, push_constants) = valid_luminance_binding();
        let buffer_resources = [buffer, buffer];
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_real_pass_attempt(
            luminance_real_pass_optin_caps(),
            &buffer_resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("runtime_binding_preflight_failed")
        );
    }

    #[test]
    fn luminance_real_pass_hash_mismatch_blocks_on_validation() {
        // Segment 4i: a LuminanceDispatchPackage whose dxil_sha256 digest does
        // not match the baked constant must be rejected with
        // `Err(FallbackReason::ValidationFailed)` (the runtime binding does
        // not correspond to the compiled package and must fall back). This
        // complements `luminance_dispatch_package_matches_offline_evidence`,
        // which covers the unavailable-package and tampered-layout paths; this
        // test pins the dxil digest specifically.
        let package = LuminanceDispatchPackage::verified_offline_package();
        assert!(package.verify_matches_offline_evidence().is_ok());

        // Tampered dxil digest → validation_failed.
        let mut tampered = package;
        tampered.dxil_sha256 =
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef0000";
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );

        // Tampered root_signature digest → validation_failed.
        let mut tampered = package;
        tampered.root_signature_sha256 =
            "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );

        // Tampered descriptor_layout digest → validation_failed.
        let mut tampered = package;
        tampered.descriptor_layout_sha256 =
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn luminance_real_pass_without_bringup_optin_is_manual_disabled() {
        // The real-pass arm alone (without the segment 4b bring-up opt-in)
        // fails dispatch eligibility as manual_disabled: both default-false
        // settings must be explicitly enabled.
        let (resources, push_constants) = valid_luminance_binding();
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64 | RXGD_CAP_LUMINANCE_REAL_PASS;
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_real_pass_attempt(caps, &resources, &push_constants, 1, 2);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ManualDisabled)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("dispatch_eligibility_failed")
        );
    }

    #[test]
    fn luminance_real_pass_capability_downgrade_is_unsupported_device() {
        // Forced-failure knob (segment 4h red leg): clearing the 64-bit
        // integer capability makes the preflight fail closed first with
        // unsupported_device.
        let (resources, push_constants) = valid_luminance_binding();
        let mut caps = luminance_real_pass_optin_caps();
        caps.flags &= !RXGD_CAP_SHADER_INT64;
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_real_pass_attempt(caps, &resources, &push_constants, 1, 2);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::UnsupportedDevice)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("runtime_binding_preflight_failed")
        );
    }

    #[test]
    fn luminance_real_pass_null_resource_handle_is_validation_failed() {
        // Eligibility fires before the binding-kind conformance check: a null
        // native handle is validation_failed at dispatch_eligibility.
        let (mut resources, push_constants) = valid_luminance_binding();
        resources[1].native_handle = 0;
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_real_pass_attempt(
            luminance_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("dispatch_eligibility_failed")
        );
    }

    #[test]
    fn luminance_real_pass_flag_does_not_change_default_path() {
        // Without the real-pass arm the segment 4b bring-up path is byte-for-
        // byte unchanged: valid, fully eligible bindings still fall back with
        // compile_failed (the explicit dispatch gate stays closed).
        let (resources, push_constants) = valid_luminance_binding();
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_gated_dispatch_bringup(
            luminance_dispatch_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::CompileFailed)
        );
        assert_eq!(gate.last_real_pass_blocked(), None);
    }

    #[test]
    fn abi_version_is_stable() {
        assert_eq!(rxgd_abi_version(), RXGD_ABI_VERSION);
        assert_eq!(size_of::<RxGdCaps>(), 36);
        assert_eq!(size_of::<RxGdResource>(), 48);
        assert_eq!(size_of::<RxGdFrameStats>(), 64);
    }

    #[test]
    fn create_rejects_wrong_backend() {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.backend = 99;
        let mut session = core::ptr::null_mut();
        let rc = rxgd_create_d3d12_session(fake_ptr(1), fake_ptr(2), caps, &mut session);
        assert_eq!(rc, RXGD_E_UNSUPPORTED);
        assert!(session.is_null());
    }

    #[test]
    fn record_supported_pass_and_stats() {
        // GRX-010 gated RXGD_PASS_TONEMAP and GRX-011 gated
        // RXGD_PASS_SSAO_BLUR (both fail-closed), so this estimated-timing
        // regression uses SSIL blur (still on the placeholder path).
        let session = create_session();
        let resource = RxGdResource::texture(123, 1920, 1080, 87);
        assert_eq!(rxgd_register_texture(session, resource), RXGD_STATUS_OK);
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_SSIL_BLUR,
                &resource,
                1,
                core::ptr::null(),
                0
            ),
            RXGD_STATUS_OK
        );

        let mut stats = RxGdFrameStats {
            abi_version: 0,
            struct_size: 0,
            frame_id: 0,
            recorded_passes: 0,
            fallback_passes: 0,
            registered_resources: 0,
            gpu_time_ns: 0,
            cpu_record_ns: 0,
            last_error: 0,
        };
        assert_eq!(
            rxgd_collect_timestamps(session, 42, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.frame_id, 42);
        assert_eq!(stats.recorded_passes, 1);
        assert_eq!(stats.fallback_passes, 0);
        assert_eq!(stats.registered_resources, 1);
        assert!(stats.gpu_time_ns > 0);
        rxgd_destroy_session(session);
    }

    #[test]
    fn luminance_reduction_defaults_disabled_and_falls_back() {
        let session = create_session();
        let resource = RxGdResource::texture(321, 1920, 1080, 87);
        assert_eq!(rxgd_register_texture(session, resource), RXGD_STATUS_OK);
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                &resource,
                1,
                core::ptr::null(),
                0
            ),
            RXGD_STATUS_FALLBACK
        );

        let mut stats = RxGdFrameStats {
            abi_version: 0,
            struct_size: 0,
            frame_id: 0,
            recorded_passes: 0,
            fallback_passes: 0,
            registered_resources: 0,
            gpu_time_ns: 0,
            cpu_record_ns: 0,
            last_error: 0,
        };
        assert_eq!(
            rxgd_collect_timestamps(session, 9, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        rxgd_destroy_session(session);
    }

    #[test]
    fn luminance_runtime_binding_preflight_falls_back_without_resources() {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64;
        let session = create_session_with_caps(caps);
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                core::ptr::null(),
                0,
                [0u8; LUMINANCE_ROOT_CONSTANT_BYTES as usize].as_ptr(),
                LUMINANCE_ROOT_CONSTANT_BYTES,
            ),
            RXGD_STATUS_FALLBACK
        );

        let mut stats = RxGdFrameStats {
            abi_version: 0,
            struct_size: 0,
            frame_id: 0,
            recorded_passes: 0,
            fallback_passes: 0,
            registered_resources: 0,
            gpu_time_ns: 0,
            cpu_record_ns: 0,
            last_error: 0,
        };
        assert_eq!(
            rxgd_collect_timestamps(session, 10, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        rxgd_destroy_session(session);
    }

    #[test]
    fn luminance_runtime_binding_preflight_rejects_descriptor_mismatch() {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64;
        let session = create_session_with_caps(caps);
        let resources = [
            RxGdResource::texture(11, 128, 128, 87),
            RxGdResource {
                abi_version: RXGD_ABI_VERSION,
                struct_size: size_of::<RxGdResource>() as u32,
                resource_type: RXGD_RESOURCE_BUFFER,
                format: 0,
                width: 4,
                height: 1,
                depth: 1,
                mip_levels: 1,
                usage_flags: 0,
                native_handle: 12,
            },
        ];
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                resources.as_ptr(),
                resources.len() as u64,
                [0u8; LUMINANCE_ROOT_CONSTANT_BYTES as usize].as_ptr(),
                LUMINANCE_ROOT_CONSTANT_BYTES - 4,
            ),
            RXGD_STATUS_FALLBACK
        );
        rxgd_destroy_session(session);
    }

    #[test]
    fn luminance_requires_64bit_integer_capability_gate() {
        let session = create_session();
        let resources = [
            RxGdResource::texture(21, 128, 128, 87),
            RxGdResource::texture(22, 16, 16, 114),
        ];
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                resources.as_ptr(),
                resources.len() as u64,
                [0u8; LUMINANCE_ROOT_CONSTANT_BYTES as usize].as_ptr(),
                LUMINANCE_ROOT_CONSTANT_BYTES,
            ),
            RXGD_STATUS_FALLBACK
        );
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_runtime_binding_preflight(
            RxGdCaps::d3d12_forward_plus(),
            &resources,
            &[0u8; LUMINANCE_ROOT_CONSTANT_BYTES as usize],
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::UnsupportedDevice)
        );
        rxgd_destroy_session(session);
    }

    #[test]
    fn luminance_runtime_binding_preflight_valid_bindings_still_fall_back() {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64;
        let session = create_session_with_caps(caps);
        let resources = [
            RxGdResource::texture(31, 1920, 1080, 87),
            RxGdResource::texture(32, 240, 135, 114),
        ];
        let push_constants = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        // Segment 4a: even a fully valid runtime binding preflight must still
        // request fallback; no real D3D12 dispatch is recorded.
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                resources.as_ptr(),
                resources.len() as u64,
                push_constants.as_ptr(),
                push_constants.len() as u64,
            ),
            RXGD_STATUS_FALLBACK
        );

        let mut stats = RxGdFrameStats {
            abi_version: 0,
            struct_size: 0,
            frame_id: 0,
            recorded_passes: 0,
            fallback_passes: 0,
            registered_resources: 0,
            gpu_time_ns: 0,
            cpu_record_ns: 0,
            last_error: 0,
        };
        assert_eq!(
            rxgd_collect_timestamps(session, 11, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_runtime_binding_preflight(caps, &resources, &push_constants);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ManualDisabled)
        );
        rxgd_destroy_session(session);
    }

    #[test]
    fn luminance_preflight_rejects_source_dimension_mismatch() {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64;
        let session = create_session_with_caps(caps);
        let resources = [
            RxGdResource::texture(41, 1280, 720, 87),
            RxGdResource::texture(42, 160, 90, 114),
        ];
        // Push constants claim 1920x1080 while the bound source is 1280x720.
        let push_constants = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                resources.as_ptr(),
                resources.len() as u64,
                push_constants.as_ptr(),
                push_constants.len() as u64,
            ),
            RXGD_STATUS_FALLBACK
        );
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_runtime_binding_preflight(caps, &resources, &push_constants);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
        rxgd_destroy_session(session);
    }

    #[test]
    fn luminance_preflight_rejects_zero_source_dimensions() {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64;
        let session = create_session_with_caps(caps);
        let resources = [
            RxGdResource::texture(51, 1920, 1080, 87),
            RxGdResource::texture(52, 240, 135, 114),
        ];
        let push_constants = [0u8; LUMINANCE_ROOT_CONSTANT_BYTES as usize];
        // A zeroed b0 block carries source dimensions of 0x0.
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                resources.as_ptr(),
                resources.len() as u64,
                push_constants.as_ptr(),
                LUMINANCE_ROOT_CONSTANT_BYTES,
            ),
            RXGD_STATUS_FALLBACK
        );
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_runtime_binding_preflight(caps, &resources, &push_constants);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
        rxgd_destroy_session(session);
    }

    #[test]
    fn luminance_preflight_rejects_reduce_level_shape_mismatch() {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64;
        let session = create_session_with_caps(caps);
        // Destination must be max(source / 8, 1) per axis; 128x128 is not the
        // level-0 reduce shape for a 1920x1080 source.
        let resources = [
            RxGdResource::texture(61, 1920, 1080, 87),
            RxGdResource::texture(62, 128, 128, 114),
        ];
        let push_constants = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                resources.as_ptr(),
                resources.len() as u64,
                push_constants.as_ptr(),
                push_constants.len() as u64,
            ),
            RXGD_STATUS_FALLBACK
        );
        rxgd_destroy_session(session);
    }

    #[test]
    fn luminance_gate_enable_fails_without_compiled_kernel() {
        let mut gate = LuminanceReductionGate::new();
        assert!(!gate.is_enabled());
        assert_eq!(gate.last_fallback_reason(), None);

        assert_eq!(gate.request_enable(), Err(FallbackReason::CompileFailed));
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::CompileFailed)
        );
        assert_eq!(gate.record_outcome(), RXGD_STATUS_FALLBACK);

        let mut untouched_gate = LuminanceReductionGate::default();
        assert_eq!(untouched_gate.record_outcome(), RXGD_STATUS_FALLBACK);
        assert_eq!(
            untouched_gate.last_fallback_reason(),
            Some(FallbackReason::ManualDisabled)
        );
        assert_eq!(FallbackReason::CompileFailed.as_str(), "compile_failed");
        assert_eq!(FallbackReason::ManualDisabled.as_str(), "manual_disabled");
    }

    #[test]
    fn luminance_dispatch_package_matches_offline_evidence() {
        let package = LuminanceDispatchPackage::verified_offline_package();
        assert!(package.verify_matches_offline_evidence().is_ok());
        assert_eq!(RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP, 1 << 1);

        // Unavailable compiled package -> compile_failed.
        let mut tampered = package;
        tampered.available = false;
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::CompileFailed)
        );

        // Tampered artifact digest -> validation_failed.
        let mut tampered = package;
        tampered.descriptor_layout_sha256 = "deadbeef";
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );

        // Tampered descriptor layout -> validation_failed.
        let mut tampered = package;
        tampered.root_constant_bytes = LUMINANCE_ROOT_CONSTANT_BYTES - 4;
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn luminance_gated_dispatch_defaults_disabled_and_falls_back() {
        // Godot dispatch bring-up opt-in flag is NOT set (default). Even with a
        // fully valid runtime binding preflight the pass must fall back.
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64;
        let session = create_session_with_caps(caps);
        let resources = [
            RxGdResource::texture(31, 1920, 1080, 87),
            RxGdResource::texture(32, 240, 135, 114),
        ];
        let push_constants = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                resources.as_ptr(),
                resources.len() as u64,
                push_constants.as_ptr(),
                push_constants.len() as u64,
            ),
            RXGD_STATUS_FALLBACK
        );
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 20, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        rxgd_destroy_session(session);

        // Gate level: opt-in flag missing maps to manual_disabled.
        let mut gate = LuminanceReductionGate::new();
        assert_eq!(
            gate.record_gated_dispatch_bringup(caps, &resources, &push_constants, 1, 2),
            RXGD_STATUS_FALLBACK
        );
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ManualDisabled)
        );
    }

    #[test]
    fn luminance_gated_dispatch_missing_native_handles_falls_back() {
        let caps = luminance_dispatch_optin_caps();
        let resources = [
            RxGdResource::texture(41, 1920, 1080, 87),
            RxGdResource::texture(42, 240, 135, 114),
        ];
        let push_constants = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);

        // Null native D3D12 device handle.
        let mut gate = LuminanceReductionGate::new();
        assert_eq!(
            gate.record_gated_dispatch_bringup(caps, &resources, &push_constants, 0, 2),
            RXGD_STATUS_FALLBACK
        );
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::UnsupportedDevice)
        );

        // Null native D3D12 command queue handle.
        let mut gate = LuminanceReductionGate::new();
        assert_eq!(
            gate.record_gated_dispatch_bringup(caps, &resources, &push_constants, 1, 0),
            RXGD_STATUS_FALLBACK
        );
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::UnsupportedDevice)
        );

        // Null resource native handle.
        let bad_resources = [
            RxGdResource::texture(0, 1920, 1080, 87),
            RxGdResource::texture(42, 240, 135, 114),
        ];
        let mut gate = LuminanceReductionGate::new();
        assert_eq!(
            gate.record_gated_dispatch_bringup(caps, &bad_resources, &push_constants, 1, 2),
            RXGD_STATUS_FALLBACK
        );
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn luminance_gated_dispatch_layout_or_hash_mismatch_falls_back() {
        let caps = luminance_dispatch_optin_caps();
        let resources = [
            RxGdResource::texture(51, 1920, 1080, 87),
            RxGdResource::texture(52, 240, 135, 114),
        ];
        let push_constants = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);

        // Compiled package unavailable -> compile_failed.
        let mut gate = LuminanceReductionGate::new();
        gate.dispatch_package.available = false;
        assert_eq!(
            gate.record_gated_dispatch_bringup(caps, &resources, &push_constants, 1, 2),
            RXGD_STATUS_FALLBACK
        );
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::CompileFailed)
        );

        // Tampered artifact digest (hash mismatch) -> validation_failed.
        let mut gate = LuminanceReductionGate::new();
        gate.dispatch_package.dxil_sha256 =
            "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(
            gate.record_gated_dispatch_bringup(caps, &resources, &push_constants, 1, 2),
            RXGD_STATUS_FALLBACK
        );
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );

        // Tampered descriptor layout (wrong SRV register) -> validation_failed.
        let mut gate = LuminanceReductionGate::new();
        gate.dispatch_package.srv_register = 3;
        assert_eq!(
            gate.record_gated_dispatch_bringup(caps, &resources, &push_constants, 1, 2),
            RXGD_STATUS_FALLBACK
        );
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn luminance_gated_dispatch_valid_eligibility_still_gated() {
        let caps = luminance_dispatch_optin_caps();
        let session = create_session_with_caps(caps);
        let resources = [
            RxGdResource::texture(61, 1920, 1080, 87),
            RxGdResource::texture(62, 240, 135, 114),
        ];
        let push_constants = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        // Fully eligible via the ABI (opt-in flag set, int64 cap, non-null fake
        // device/queue and resource handles, package matches evidence). The
        // explicit dispatch gate must still force fallback and never OK, with
        // no estimated GPU/CPU time attributed.
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_LUMINANCE_REDUCTION,
                resources.as_ptr(),
                resources.len() as u64,
                push_constants.as_ptr(),
                push_constants.len() as u64,
            ),
            RXGD_STATUS_FALLBACK
        );
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 21, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        rxgd_destroy_session(session);

        // Gate level: eligibility passes but the explicit dispatch gate stays
        // closed (compile_failed: no linked runtime dispatch kernel).
        let mut gate = LuminanceReductionGate::new();
        assert!(
            gate.check_dispatch_eligibility(caps, &resources, 1, 2)
                .is_ok()
        );
        assert_eq!(
            gate.record_gated_dispatch_bringup(caps, &resources, &push_constants, 1, 2),
            RXGD_STATUS_FALLBACK
        );
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::CompileFailed)
        );
        assert_eq!(
            gate.request_dispatch_bringup(),
            Err(FallbackReason::CompileFailed)
        );
    }

    /// Caps arming the GRX-010 tonemap real-pass opt-in (plus the 64-bit
    /// integer capability the canonical b0 template requires).
    fn tonemap_real_pass_optin_caps() -> RxGdCaps {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64 | RXGD_CAP_TONEMAP_REAL_PASS;
        caps
    }

    /// A fully valid tonemap binding: two full-resolution texture resources
    /// with non-null native handles plus coherent b0 constants.
    fn valid_tonemap_binding() -> (
        [RxGdResource; 2],
        [u8; TONEMAP_ROOT_CONSTANT_BYTES as usize],
    ) {
        let resources = [
            RxGdResource::texture(81, 1920, 1080, 10),
            RxGdResource::texture(82, 1920, 1080, 28),
        ];
        let push_constants = tonemap_push_constants(1920, 1080, 1.0, 1.0, 1.0);
        (resources, push_constants)
    }

    fn tonemap_push_constants(
        source_width: u64,
        source_height: u64,
        exposure: f32,
        white: f32,
        luminance_multiplier: f32,
    ) -> [u8; TONEMAP_ROOT_CONSTANT_BYTES as usize] {
        let mut bytes = [0u8; TONEMAP_ROOT_CONSTANT_BYTES as usize];
        bytes[0..8].copy_from_slice(&source_width.to_le_bytes());
        bytes[8..16].copy_from_slice(&source_height.to_le_bytes());
        bytes[16..20].copy_from_slice(&exposure.to_le_bytes());
        bytes[20..24].copy_from_slice(&white.to_le_bytes());
        bytes[24..28].copy_from_slice(&luminance_multiplier.to_le_bytes());
        bytes
    }

    #[test]
    fn tonemap_cap_flag_is_reserved_bit_four() {
        assert_eq!(RXGD_CAP_TONEMAP_REAL_PASS, 1 << 4);
        // ABI stays v1: the flag reuses the existing RxGdCaps.flags field.
        assert_eq!(rxgd_abi_version(), RXGD_ABI_VERSION);
    }

    #[test]
    fn tonemap_defaults_disabled_and_falls_back() {
        // GRX-010: the default path (no opt-in flag) must fall back and must
        // not attribute the historical placeholder estimated GPU time. This
        // mirrors the patch 0011 module gate, which calls the bridge without
        // resource bindings (0002-level wiring).
        let session = create_session();
        let resource = RxGdResource::texture(321, 1920, 1080, 87);
        assert_eq!(rxgd_register_texture(session, resource), RXGD_STATUS_OK);
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_TONEMAP,
                &resource,
                1,
                core::ptr::null(),
                0
            ),
            RXGD_STATUS_FALLBACK
        );

        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 30, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        rxgd_destroy_session(session);
    }

    #[test]
    fn tonemap_valid_default_binding_still_falls_back_manual_disabled() {
        // A fully valid binding without the real-pass opt-in flag keeps the
        // gate closed with manual_disabled and never returns OK.
        let (resources, push_constants) = valid_tonemap_binding();
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64;
        let mut gate = TonemapGate::new();
        assert_eq!(
            gate.record_default_fallback(caps, &resources, &push_constants),
            RXGD_STATUS_FALLBACK
        );
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ManualDisabled)
        );
        assert_eq!(gate.last_real_pass_blocked(), None);
    }

    /// GRX-010: a fully valid, fully opted-in real-pass attempt passes every
    /// software gate (preflight, eligibility, per-slot binding kind,
    /// CPU-proven math parity) and reaches the linked-dispatch boundary. In
    /// the shipping feature-off build no dispatch path is compiled in, so
    /// the attempt still fails closed with `real_dispatch_path_not_linked`.
    #[cfg(not(feature = "d3d12-recording-shim"))]
    #[test]
    fn tonemap_real_pass_optin_blocks_on_missing_dispatch_path() {
        let (resources, push_constants) = valid_tonemap_binding();
        let session = create_session_with_caps(tonemap_real_pass_optin_caps());
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_TONEMAP,
                resources.as_ptr(),
                resources.len() as u64,
                push_constants.as_ptr(),
                push_constants.len() as u64,
            ),
            RXGD_STATUS_FALLBACK
        );
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 31, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        rxgd_destroy_session(session);

        let mut gate = TonemapGate::new();
        let rc = gate.record_real_pass_attempt(
            tonemap_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::CompileFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("real_dispatch_path_not_linked")
        );
    }

    #[test]
    fn tonemap_real_pass_capability_downgrade_is_unsupported_device() {
        // Forced-failure red leg: clearing the 64-bit integer capability
        // makes the preflight fail closed first with unsupported_device.
        let (resources, push_constants) = valid_tonemap_binding();
        let mut caps = tonemap_real_pass_optin_caps();
        caps.flags &= !RXGD_CAP_SHADER_INT64;
        let mut gate = TonemapGate::new();
        let rc = gate.record_real_pass_attempt(caps, &resources, &push_constants, 1, 2);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::UnsupportedDevice)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("runtime_binding_preflight_failed")
        );
    }

    #[test]
    fn tonemap_real_pass_null_resource_handle_is_validation_failed() {
        let (mut resources, push_constants) = valid_tonemap_binding();
        resources[1].native_handle = 0;
        let mut gate = TonemapGate::new();
        let rc = gate.record_real_pass_attempt(
            tonemap_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("dispatch_eligibility_failed")
        );
    }

    #[test]
    fn tonemap_preflight_rejects_shape_and_dimension_mismatches() {
        let caps = tonemap_real_pass_optin_caps();

        // dst extent must equal src extent (1:1 full-resolution pass).
        let resources = [
            RxGdResource::texture(91, 1920, 1080, 10),
            RxGdResource::texture(92, 960, 540, 28),
        ];
        let push_constants = tonemap_push_constants(1920, 1080, 1.0, 1.0, 1.0);
        assert_eq!(
            TonemapGate::check_runtime_binding_preflight(caps, &resources, &push_constants),
            Err(FallbackReason::ValidationFailed)
        );

        // b0 dims must match the bound src resource.
        let resources = [
            RxGdResource::texture(93, 1280, 720, 10),
            RxGdResource::texture(94, 1280, 720, 28),
        ];
        assert_eq!(
            TonemapGate::check_runtime_binding_preflight(caps, &resources, &push_constants),
            Err(FallbackReason::ValidationFailed)
        );

        // Zeroed dims fail.
        let zeroed = [0u8; TONEMAP_ROOT_CONSTANT_BYTES as usize];
        assert_eq!(
            TonemapGate::check_runtime_binding_preflight(caps, &resources, &zeroed),
            Err(FallbackReason::ValidationFailed)
        );

        // A coherent 1:1 binding passes the pure preflight.
        let (resources, push_constants) = valid_tonemap_binding();
        assert_eq!(
            TonemapGate::check_runtime_binding_preflight(caps, &resources, &push_constants),
            Ok(())
        );
    }

    #[test]
    fn tonemap_real_pass_binding_kind_check_accepts_texture_slots_only() {
        let texture = RxGdResource::texture(95, 64, 64, 10);
        let buffer = RxGdResource {
            resource_type: RXGD_RESOURCE_BUFFER,
            ..texture
        };
        assert_eq!(
            TonemapGate::check_real_pass_binding_kind(&[texture, texture]),
            Ok(())
        );
        assert_eq!(
            TonemapGate::check_real_pass_binding_kind(&[texture]),
            Err(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            TonemapGate::check_real_pass_binding_kind(&[buffer, buffer]),
            Err(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            TonemapGate::check_real_pass_binding_kind(&[texture, buffer]),
            Err(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn tonemap_dispatch_package_matches_offline_evidence() {
        let package = TonemapDispatchPackage::verified_offline_package();
        assert!(package.verify_matches_offline_evidence().is_ok());

        // Unavailable compiled package -> compile_failed.
        let mut tampered = package;
        tampered.available = false;
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::CompileFailed)
        );

        // Tampered artifact digest -> validation_failed.
        let mut tampered = package;
        tampered.dxil_sha256 = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );

        // Tampered descriptor layout -> validation_failed.
        let mut tampered = package;
        tampered.root_constant_bytes = TONEMAP_ROOT_CONSTANT_BYTES - 4;
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn tonemap_real_pass_hash_mismatch_blocks_on_validation() {
        // A tampered package digest must block the fully opted-in real-pass
        // attempt at dispatch eligibility with validation_failed.
        let (resources, push_constants) = valid_tonemap_binding();
        let mut gate = TonemapGate::new();
        gate.dispatch_package.dxil_sha256 =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let rc = gate.record_real_pass_attempt(
            tonemap_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("dispatch_eligibility_failed")
        );
    }

    #[test]
    fn tonemap_math_parity_gate_allows_cpu_proven_linear_srgb() {
        assert_eq!(
            TONEMAP_KERNEL_MATH_PARITY_STATUS,
            "linear_srgb_cpu_reference_proven_pending_gpu_dispatch"
        );
        assert_eq!(TonemapGate::check_real_pass_math_parity(), Ok(()));
    }

    /// Caps arming the GRX-011 ssao_blur real-pass opt-in (plus the 64-bit
    /// integer capability the canonical b0 template requires).
    fn ssao_blur_real_pass_optin_caps() -> RxGdCaps {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64 | RXGD_CAP_SSAO_BLUR_REAL_PASS;
        caps
    }

    /// A fully valid ssao_blur binding: two half-res-slice-sized texture
    /// resources with non-null native handles plus coherent b0 constants.
    fn valid_ssao_blur_binding() -> (
        [RxGdResource; 2],
        [u8; SSAO_BLUR_ROOT_CONSTANT_BYTES as usize],
    ) {
        let resources = [
            RxGdResource::texture(101, 960, 540, 17),
            RxGdResource::texture(102, 960, 540, 17),
        ];
        let push_constants = ssao_blur_push_constants(960, 540, 0.02, 1.0 / 960.0, 1.0 / 540.0);
        (resources, push_constants)
    }

    fn ssao_blur_push_constants(
        source_width: u64,
        source_height: u64,
        edge_sharpness: f32,
        half_screen_pixel_size_x: f32,
        half_screen_pixel_size_y: f32,
    ) -> [u8; SSAO_BLUR_ROOT_CONSTANT_BYTES as usize] {
        let mut bytes = [0u8; SSAO_BLUR_ROOT_CONSTANT_BYTES as usize];
        bytes[0..8].copy_from_slice(&source_width.to_le_bytes());
        bytes[8..16].copy_from_slice(&source_height.to_le_bytes());
        bytes[16..20].copy_from_slice(&edge_sharpness.to_le_bytes());
        bytes[20..24].copy_from_slice(&half_screen_pixel_size_x.to_le_bytes());
        bytes[24..28].copy_from_slice(&half_screen_pixel_size_y.to_le_bytes());
        bytes
    }

    #[test]
    fn ssao_blur_cap_flag_is_reserved_bit_five() {
        assert_eq!(RXGD_CAP_SSAO_BLUR_REAL_PASS, 1 << 5);
        // ABI stays v1: the flag reuses the existing RxGdCaps.flags field.
        assert_eq!(rxgd_abi_version(), RXGD_ABI_VERSION);
    }

    #[test]
    fn ssao_blur_defaults_disabled_and_falls_back() {
        // GRX-011: the default path (no opt-in flag) must fall back and must
        // not attribute the historical placeholder estimated GPU time. This
        // mirrors the patch 0012 module gate, which calls the bridge without
        // resource bindings (0002-level wiring).
        let session = create_session();
        let resource = RxGdResource::texture(321, 960, 540, 17);
        assert_eq!(rxgd_register_texture(session, resource), RXGD_STATUS_OK);
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_SSAO_BLUR,
                &resource,
                1,
                core::ptr::null(),
                0
            ),
            RXGD_STATUS_FALLBACK
        );

        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 40, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        rxgd_destroy_session(session);
    }

    #[test]
    fn ssao_blur_valid_default_binding_still_falls_back_manual_disabled() {
        // A fully valid binding without the real-pass opt-in flag keeps the
        // gate closed with manual_disabled and never returns OK.
        let (resources, push_constants) = valid_ssao_blur_binding();
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64;
        let mut gate = SsaoBlurGate::new();
        assert_eq!(
            gate.record_default_fallback(caps, &resources, &push_constants),
            RXGD_STATUS_FALLBACK
        );
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ManualDisabled)
        );
        assert_eq!(gate.last_real_pass_blocked(), None);
    }

    /// GRX-011: a fully valid, fully opted-in real-pass attempt passes every
    /// software gate (preflight, eligibility, per-slot binding kind,
    /// CPU-proven math parity) and reaches the linked-dispatch boundary. In
    /// the shipping feature-off build no dispatch path is compiled in, so
    /// the attempt still fails closed with `real_dispatch_path_not_linked`.
    #[cfg(not(feature = "d3d12-recording-shim"))]
    #[test]
    fn ssao_blur_real_pass_optin_blocks_on_missing_dispatch_path() {
        let (resources, push_constants) = valid_ssao_blur_binding();
        let session = create_session_with_caps(ssao_blur_real_pass_optin_caps());
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_SSAO_BLUR,
                resources.as_ptr(),
                resources.len() as u64,
                push_constants.as_ptr(),
                push_constants.len() as u64,
            ),
            RXGD_STATUS_FALLBACK
        );
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 41, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        rxgd_destroy_session(session);

        let mut gate = SsaoBlurGate::new();
        let rc = gate.record_real_pass_attempt(
            ssao_blur_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::CompileFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("real_dispatch_path_not_linked")
        );
    }

    #[test]
    fn ssao_blur_real_pass_capability_downgrade_is_unsupported_device() {
        // Forced-failure red leg: clearing the 64-bit integer capability
        // makes the preflight fail closed first with unsupported_device.
        let (resources, push_constants) = valid_ssao_blur_binding();
        let mut caps = ssao_blur_real_pass_optin_caps();
        caps.flags &= !RXGD_CAP_SHADER_INT64;
        let mut gate = SsaoBlurGate::new();
        let rc = gate.record_real_pass_attempt(caps, &resources, &push_constants, 1, 2);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::UnsupportedDevice)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("runtime_binding_preflight_failed")
        );
    }

    #[test]
    fn ssao_blur_real_pass_null_resource_handle_is_validation_failed() {
        let (mut resources, push_constants) = valid_ssao_blur_binding();
        resources[1].native_handle = 0;
        let mut gate = SsaoBlurGate::new();
        let rc = gate.record_real_pass_attempt(
            ssao_blur_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("dispatch_eligibility_failed")
        );
    }

    #[test]
    fn ssao_blur_preflight_rejects_shape_and_dimension_mismatches() {
        let caps = ssao_blur_real_pass_optin_caps();

        // dst extent must equal src extent (1:1 ping-pong blur pass).
        let resources = [
            RxGdResource::texture(111, 960, 540, 17),
            RxGdResource::texture(112, 480, 270, 17),
        ];
        let push_constants = ssao_blur_push_constants(960, 540, 0.02, 1.0 / 960.0, 1.0 / 540.0);
        assert_eq!(
            SsaoBlurGate::check_runtime_binding_preflight(caps, &resources, &push_constants),
            Err(FallbackReason::ValidationFailed)
        );

        // b0 dims must match the bound src resource.
        let resources = [
            RxGdResource::texture(113, 640, 360, 17),
            RxGdResource::texture(114, 640, 360, 17),
        ];
        assert_eq!(
            SsaoBlurGate::check_runtime_binding_preflight(caps, &resources, &push_constants),
            Err(FallbackReason::ValidationFailed)
        );

        // Zeroed dims fail.
        let zeroed = [0u8; SSAO_BLUR_ROOT_CONSTANT_BYTES as usize];
        assert_eq!(
            SsaoBlurGate::check_runtime_binding_preflight(caps, &resources, &zeroed),
            Err(FallbackReason::ValidationFailed)
        );

        // A coherent 1:1 binding passes the pure preflight.
        let (resources, push_constants) = valid_ssao_blur_binding();
        assert_eq!(
            SsaoBlurGate::check_runtime_binding_preflight(caps, &resources, &push_constants),
            Ok(())
        );
    }

    #[test]
    fn ssao_blur_real_pass_binding_kind_check_accepts_texture_slots_only() {
        let texture = RxGdResource::texture(115, 64, 64, 17);
        let buffer = RxGdResource {
            resource_type: RXGD_RESOURCE_BUFFER,
            ..texture
        };
        assert_eq!(
            SsaoBlurGate::check_real_pass_binding_kind(&[texture, texture]),
            Ok(())
        );
        assert_eq!(
            SsaoBlurGate::check_real_pass_binding_kind(&[texture]),
            Err(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            SsaoBlurGate::check_real_pass_binding_kind(&[buffer, buffer]),
            Err(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            SsaoBlurGate::check_real_pass_binding_kind(&[texture, buffer]),
            Err(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn ssao_blur_dispatch_package_matches_offline_evidence() {
        let package = SsaoBlurDispatchPackage::verified_offline_package();
        assert!(package.verify_matches_offline_evidence().is_ok());

        // Unavailable compiled package -> compile_failed.
        let mut tampered = package;
        tampered.available = false;
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::CompileFailed)
        );

        // Tampered artifact digest -> validation_failed.
        let mut tampered = package;
        tampered.dxil_sha256 = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );

        // Tampered descriptor layout -> validation_failed.
        let mut tampered = package;
        tampered.root_constant_bytes = SSAO_BLUR_ROOT_CONSTANT_BYTES - 4;
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn ssao_blur_real_pass_hash_mismatch_blocks_on_validation() {
        // A tampered package digest must block the fully opted-in real-pass
        // attempt at dispatch eligibility with validation_failed.
        let (resources, push_constants) = valid_ssao_blur_binding();
        let mut gate = SsaoBlurGate::new();
        gate.dispatch_package.dxil_sha256 =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let rc = gate.record_real_pass_attempt(
            ssao_blur_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("dispatch_eligibility_failed")
        );
    }

    #[test]
    fn ssao_blur_math_parity_gate_allows_cpu_proven_smart_blur() {
        assert_eq!(
            SSAO_BLUR_KERNEL_MATH_PARITY_STATUS,
            "smart_blur_cpu_reference_proven_pending_gpu_dispatch"
        );
        assert_eq!(SsaoBlurGate::check_real_pass_math_parity(), Ok(()));
    }

    // ── GRX-012 taa_resolve gate tests ───────────────────────────────────────

    /// Caps arming the GRX-012 taa_resolve real-pass opt-in (plus the 64-bit
    /// integer capability the canonical b0 template requires).
    fn taa_resolve_real_pass_optin_caps() -> RxGdCaps {
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64 | RXGD_CAP_TAA_RESOLVE_REAL_PASS;
        caps
    }

    fn taa_resolve_push_constants(
        source_width: u64,
        source_height: u64,
        disocclusion_threshold: f32,
        variance_dynamic: f32,
        reserved0: f32,
    ) -> [u8; TAA_RESOLVE_ROOT_CONSTANT_BYTES as usize] {
        let mut bytes = [0u8; TAA_RESOLVE_ROOT_CONSTANT_BYTES as usize];
        bytes[0..8].copy_from_slice(&source_width.to_le_bytes());
        bytes[8..16].copy_from_slice(&source_height.to_le_bytes());
        bytes[16..20].copy_from_slice(&disocclusion_threshold.to_le_bytes());
        bytes[20..24].copy_from_slice(&variance_dynamic.to_le_bytes());
        bytes[24..28].copy_from_slice(&reserved0.to_le_bytes());
        bytes
    }

    /// A fully valid taa_resolve binding: six full-resolution texture resources
    /// (color/depth/velocity/last_velocity/history/output) with non-null native
    /// handles plus coherent b0 constants.
    fn valid_taa_resolve_binding() -> (
        [RxGdResource; 6],
        [u8; TAA_RESOLVE_ROOT_CONSTANT_BYTES as usize],
    ) {
        let resources = [
            RxGdResource::texture(201, 960, 540, 2), // color_buffer   (t0)
            RxGdResource::texture(202, 960, 540, 35), // depth_buffer   (t1)
            RxGdResource::texture(203, 960, 540, 34), // velocity       (t2)
            RxGdResource::texture(204, 960, 540, 34), // last_velocity  (t3)
            RxGdResource::texture(205, 960, 540, 2), // history        (t4)
            RxGdResource::texture(206, 960, 540, 2), // output         (u0)
        ];
        let push_constants = taa_resolve_push_constants(960, 540, 0.1 / 960.0, 1.0, 0.0);
        (resources, push_constants)
    }

    #[test]
    fn taa_resolve_cap_flag_is_reserved_bit_six() {
        assert_eq!(RXGD_CAP_TAA_RESOLVE_REAL_PASS, 1 << 6);
        // ABI stays v1: the flag reuses the existing RxGdCaps.flags field.
        assert_eq!(rxgd_abi_version(), RXGD_ABI_VERSION);
    }

    #[test]
    fn taa_resolve_defaults_disabled_and_falls_back() {
        // GRX-012: the default path (no opt-in flag) must fall back and must
        // not attribute the historical placeholder estimated GPU time.
        let session = create_session();
        let resource = RxGdResource::texture(721, 960, 540, 2);
        assert_eq!(rxgd_register_texture(session, resource), RXGD_STATUS_OK);
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_TAA_RESOLVE,
                &resource,
                1,
                core::ptr::null(),
                0
            ),
            RXGD_STATUS_FALLBACK
        );
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 60, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        rxgd_destroy_session(session);
    }

    #[test]
    fn taa_resolve_valid_default_binding_still_falls_back_manual_disabled() {
        let (resources, push_constants) = valid_taa_resolve_binding();
        let mut caps = RxGdCaps::d3d12_forward_plus();
        caps.flags = RXGD_CAP_SHADER_INT64;
        let mut gate = TaaResolveGate::new();
        assert_eq!(
            gate.record_default_fallback(caps, &resources, &push_constants),
            RXGD_STATUS_FALLBACK
        );
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ManualDisabled)
        );
        assert_eq!(gate.last_real_pass_blocked(), None);
    }

    /// GRX-012: a fully valid, fully opted-in real-pass attempt passes every
    /// software gate and reaches the linked-dispatch boundary. In the shipping
    /// feature-off build no dispatch path is compiled in, so the attempt fails
    /// closed with `real_dispatch_path_not_linked`.
    #[cfg(not(feature = "d3d12-recording-shim"))]
    #[test]
    fn taa_resolve_real_pass_optin_blocks_on_missing_dispatch_path() {
        let (resources, push_constants) = valid_taa_resolve_binding();
        let session = create_session_with_caps(taa_resolve_real_pass_optin_caps());
        assert_eq!(
            rxgd_record_pass(
                session,
                RXGD_PASS_TAA_RESOLVE,
                resources.as_ptr(),
                resources.len() as u64,
                push_constants.as_ptr(),
                push_constants.len() as u64,
            ),
            RXGD_STATUS_FALLBACK
        );
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 61, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.gpu_time_ns, 0);
        assert_eq!(stats.cpu_record_ns, 0);
        rxgd_destroy_session(session);

        let mut gate = TaaResolveGate::new();
        let rc = gate.record_real_pass_attempt(
            taa_resolve_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::CompileFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("real_dispatch_path_not_linked")
        );
    }

    #[test]
    fn taa_resolve_real_pass_capability_downgrade_is_unsupported_device() {
        let (resources, push_constants) = valid_taa_resolve_binding();
        let mut caps = taa_resolve_real_pass_optin_caps();
        caps.flags &= !RXGD_CAP_SHADER_INT64;
        let mut gate = TaaResolveGate::new();
        let rc = gate.record_real_pass_attempt(caps, &resources, &push_constants, 1, 2);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::UnsupportedDevice)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("runtime_binding_preflight_failed")
        );
    }

    #[test]
    fn taa_resolve_real_pass_null_resource_handle_is_validation_failed() {
        let (mut resources, push_constants) = valid_taa_resolve_binding();
        resources[5].native_handle = 0;
        let mut gate = TaaResolveGate::new();
        let rc = gate.record_real_pass_attempt(
            taa_resolve_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("dispatch_eligibility_failed")
        );
    }

    #[test]
    fn taa_resolve_preflight_rejects_shape_and_dimension_mismatches() {
        let caps = taa_resolve_real_pass_optin_caps();

        // output extent must equal color extent (1:1 full-res resolve).
        let mut resources = valid_taa_resolve_binding().0;
        resources[5] = RxGdResource::texture(206, 480, 270, 2);
        let push_constants = taa_resolve_push_constants(960, 540, 0.1 / 960.0, 1.0, 0.0);
        assert_eq!(
            TaaResolveGate::check_runtime_binding_preflight(caps, &resources, &push_constants),
            Err(FallbackReason::ValidationFailed)
        );

        // b0 dims must match the bound color resource.
        let resources = valid_taa_resolve_binding().0;
        let bad_dims = taa_resolve_push_constants(640, 360, 0.1 / 640.0, 1.0, 0.0);
        assert_eq!(
            TaaResolveGate::check_runtime_binding_preflight(caps, &resources, &bad_dims),
            Err(FallbackReason::ValidationFailed)
        );

        // Wrong resource count fails.
        let five: [RxGdResource; 5] = [
            resources[0],
            resources[1],
            resources[2],
            resources[3],
            resources[4],
        ];
        assert_eq!(
            TaaResolveGate::check_runtime_binding_preflight(caps, &five, &push_constants),
            Err(FallbackReason::ValidationFailed)
        );

        // A coherent 6-resource binding passes the pure preflight.
        let (resources, push_constants) = valid_taa_resolve_binding();
        assert_eq!(
            TaaResolveGate::check_runtime_binding_preflight(caps, &resources, &push_constants),
            Ok(())
        );
    }

    #[test]
    fn taa_resolve_real_pass_binding_kind_check_accepts_texture_slots_only() {
        let (resources, _) = valid_taa_resolve_binding();
        assert_eq!(
            TaaResolveGate::check_real_pass_binding_kind(&resources),
            Ok(())
        );
        // Too few resources fails.
        assert_eq!(
            TaaResolveGate::check_real_pass_binding_kind(&resources[..5]),
            Err(FallbackReason::ValidationFailed)
        );
        // A buffer at any slot fails closed (raw_buffer_view never conforms).
        let mut with_buffer = resources;
        with_buffer[2] = RxGdResource {
            resource_type: RXGD_RESOURCE_BUFFER,
            ..with_buffer[2]
        };
        assert_eq!(
            TaaResolveGate::check_real_pass_binding_kind(&with_buffer),
            Err(FallbackReason::ValidationFailed)
        );
        let mut buffer_uav = resources;
        buffer_uav[5] = RxGdResource {
            resource_type: RXGD_RESOURCE_BUFFER,
            ..buffer_uav[5]
        };
        assert_eq!(
            TaaResolveGate::check_real_pass_binding_kind(&buffer_uav),
            Err(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn taa_resolve_dispatch_package_matches_offline_evidence() {
        let package = TaaResolveDispatchPackage::verified_offline_package();
        assert!(package.verify_matches_offline_evidence().is_ok());
        assert_eq!(package.resource_count, 6);
        assert_eq!(package.srv_count, 5);

        let mut tampered = package;
        tampered.available = false;
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::CompileFailed)
        );

        let mut tampered = package;
        tampered.dxil_sha256 = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );

        let mut tampered = package;
        tampered.srv_count = 4;
        assert_eq!(
            tampered.verify_matches_offline_evidence(),
            Err(FallbackReason::ValidationFailed)
        );
    }

    #[test]
    fn taa_resolve_real_pass_hash_mismatch_blocks_on_validation() {
        let (resources, push_constants) = valid_taa_resolve_binding();
        let mut gate = TaaResolveGate::new();
        gate.dispatch_package.dxil_sha256 =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let rc = gate.record_real_pass_attempt(
            taa_resolve_real_pass_optin_caps(),
            &resources,
            &push_constants,
            1,
            2,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("dispatch_eligibility_failed")
        );
    }

    #[test]
    fn taa_resolve_math_parity_gate_allows_cpu_proven_resolve() {
        assert_eq!(
            TAA_RESOLVE_KERNEL_MATH_PARITY_STATUS,
            "taa_resolve_cpu_reference_proven_pending_gpu_dispatch"
        );
        assert_eq!(TaaResolveGate::check_real_pass_math_parity(), Ok(()));
        assert_eq!(
            TAA_RESOLVE_KERNEL_RESOURCE_BINDING_KINDS,
            [
                "texture2d",
                "texture2d",
                "texture2d",
                "texture2d",
                "texture2d",
                "rwtexture2d"
            ]
        );
    }

    #[test]
    fn unsupported_pass_requests_fallback() {
        let session = create_session();
        let rc = rxgd_record_pass(session, 999, core::ptr::null(), 0, core::ptr::null(), 0);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        let mut stats = RxGdFrameStats {
            abi_version: 0,
            struct_size: 0,
            frame_id: 0,
            recorded_passes: 0,
            fallback_passes: 0,
            registered_resources: 0,
            gpu_time_ns: 0,
            cpu_record_ns: 0,
            last_error: 0,
        };
        assert_eq!(
            rxgd_collect_timestamps(session, 7, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        rxgd_destroy_session(session);
    }

    #[test]
    fn dispatch_recording_shim_availability_matches_feature() {
        let expected = if cfg!(feature = "d3d12-recording-shim") {
            1
        } else {
            0
        };
        assert_eq!(rxgd_dispatch_recording_shim_available(), expected);
    }

    /// GRX-009 segment 4d: even under the `d3d12-recording-shim` feature, the
    /// record arm requires the harness-only RXGD_CAP_LUMINANCE_DISPATCH_RECORD
    /// flag. With only the dispatch bring-up opt-in set (and no record flag) the
    /// explicit gate stays closed and the pass falls back — the recording shim
    /// is never invoked, so no fake/null handle is ever dereferenced.
    #[test]
    fn luminance_record_arm_requires_record_flag() {
        let caps = luminance_dispatch_optin_caps();
        assert_eq!(caps.flags & RXGD_CAP_LUMINANCE_DISPATCH_RECORD, 0);
        let resources = [
            RxGdResource::texture(71, 1920, 1080, 87),
            RxGdResource::texture(72, 240, 135, 114),
        ];
        let push_constants = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        let mut gate = LuminanceReductionGate::new();
        assert_eq!(
            gate.record_gated_dispatch_bringup(caps, &resources, &push_constants, 1, 2),
            RXGD_STATUS_FALLBACK
        );
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::CompileFailed)
        );
    }

    // ── GRX-009 Wave 2 multi-level pyramid (level chain + hook contract) ──────

    /// A source already ≤ 8×8 (or 1×1) yields a single final WRITE_LUMINANCE
    /// level whose destination is 1×1.
    #[test]
    fn pyramid_plan_single_level_for_small_source() {
        for (w, h) in [(8u32, 8u32), (1, 1), (5, 7), (3, 8)] {
            let levels = plan_luminance_pyramid_levels(w, h);
            assert_eq!(levels.len(), 1, "source {w}x{h} should be one level");
            assert!(levels[0].is_final);
            assert_eq!((levels[0].dst_width, levels[0].dst_height), (1, 1));
            assert_eq!(
                (levels[0].src_width, levels[0].src_height),
                (w.max(1), h.max(1))
            );
        }
    }

    /// A 16×16 source cascades 16×16 → 2×2 → 1×1 (two dispatches); only the last
    /// level is the final WRITE_LUMINANCE level.
    #[test]
    fn pyramid_plan_two_level_cascade() {
        let levels = plan_luminance_pyramid_levels(16, 16);
        assert_eq!(levels.len(), 2);
        assert_eq!(
            levels[0],
            PyramidLevel {
                src_width: 16,
                src_height: 16,
                dst_width: 2,
                dst_height: 2,
                is_final: false
            }
        );
        assert_eq!(
            levels[1],
            PyramidLevel {
                src_width: 2,
                src_height: 2,
                dst_width: 1,
                dst_height: 1,
                is_final: true
            }
        );
    }

    /// A 1920×1080 source cascades in four dispatches down to 1×1. Dims follow
    /// Godot's native floor cascade (`MAX(dim/8, 1)`): 240×135 → 30×16 → 3×2 →
    /// 1×1; each level's source equals the previous level's floor destination,
    /// so the reduce[] extents are byte-identical to Godot's native luminance
    /// buffers.
    #[test]
    fn pyramid_plan_1080p_four_levels_chain_is_contiguous() {
        let levels = plan_luminance_pyramid_levels(1920, 1080);
        assert_eq!(levels.len(), 4);
        // Native floor chain, level by level.
        assert_eq!((levels[0].src_width, levels[0].src_height), (1920, 1080));
        assert_eq!((levels[0].dst_width, levels[0].dst_height), (240, 135));
        assert_eq!((levels[1].dst_width, levels[1].dst_height), (30, 16));
        assert_eq!((levels[2].dst_width, levels[2].dst_height), (3, 2));
        assert!(levels.last().unwrap().is_final);
        assert_eq!(
            (
                levels.last().unwrap().dst_width,
                levels.last().unwrap().dst_height
            ),
            (1, 1)
        );
        // Exactly one final level; the chain is contiguous.
        assert_eq!(levels.iter().filter(|l| l.is_final).count(), 1);
        for pair in levels.windows(2) {
            assert_eq!(
                (pair[0].dst_width, pair[0].dst_height),
                (pair[1].src_width, pair[1].src_height)
            );
        }
    }

    /// The GRX-009 segment 4h smoke viewport (256×144) cascades in exactly three
    /// dispatches following Godot's native floor chain: 256×144 → 32×18 → 4×2 →
    /// 1×1. Locks the reduce[] extents to the native luminance buffers this
    /// pyramid rebinds (`Luminance::LuminanceBuffers::configure`), including the
    /// floor edge-drop at the 18→2 level (native `ceil(18/8)=3` groups write
    /// into a floor `18/8=2` buffer; the trailing partial tile is discarded).
    #[test]
    fn pyramid_plan_256x144_matches_native_floor_chain() {
        let levels = plan_luminance_pyramid_levels(256, 144);
        assert_eq!(levels.len(), 3);
        assert_eq!(
            levels[0],
            PyramidLevel {
                src_width: 256,
                src_height: 144,
                dst_width: 32,
                dst_height: 18,
                is_final: false
            }
        );
        assert_eq!(
            levels[1],
            PyramidLevel {
                src_width: 32,
                src_height: 18,
                dst_width: 4,
                dst_height: 2,
                is_final: false
            }
        );
        assert_eq!(
            levels[2],
            PyramidLevel {
                src_width: 4,
                src_height: 2,
                dst_width: 1,
                dst_height: 1,
                is_final: true
            }
        );
        // Array length the patch marshals: [source, reduce[0..L-1], current,
        // prev] = num_levels + 2 = 5 for the smoke viewport.
        assert_eq!(levels.len() + 2, 5);
    }

    /// Hook-contract resource array `[source, reduce[0..L-1], current, prev]` is
    /// `level_count + 2` textures; a correct binding validates.
    #[test]
    fn pyramid_resource_binding_accepts_full_contract_array() {
        let levels = plan_luminance_pyramid_levels(1920, 1080); // 4 levels -> 6 resources
        let resources: Vec<RxGdResource> = (0..levels.len() + 2)
            .map(|i| RxGdResource::texture((i as u64) + 100, 64, 64, 41))
            .collect();
        assert_eq!(resources.len(), 6);
        assert_eq!(
            LuminanceReductionGate::check_pyramid_resource_binding(&resources, levels.len()),
            Ok(())
        );
    }

    /// Fail-closed: any single zero (missing) native handle fails the WHOLE
    /// pyramid binding.
    #[test]
    fn pyramid_resource_binding_any_zero_handle_fails_closed() {
        let level_count = 4;
        for zero_slot in 0..level_count + 2 {
            let mut resources: Vec<RxGdResource> = (0..level_count + 2)
                .map(|i| RxGdResource::texture((i as u64) + 100, 64, 64, 41))
                .collect();
            resources[zero_slot].native_handle = 0;
            assert_eq!(
                LuminanceReductionGate::check_pyramid_resource_binding(&resources, level_count),
                Err(FallbackReason::ValidationFailed),
                "zero handle at slot {zero_slot} must fail closed"
            );
        }
    }

    /// Fail-closed: wrong array length, a buffer resource, and a zero level
    /// count are all rejected.
    #[test]
    fn pyramid_resource_binding_shape_violations_fail_closed() {
        let ok: Vec<RxGdResource> = (0..6)
            .map(|i| RxGdResource::texture((i as u64) + 1, 64, 64, 41))
            .collect();
        // level_count 4 expects 6 resources; 5 or 7 must fail.
        assert_eq!(
            LuminanceReductionGate::check_pyramid_resource_binding(&ok[..5], 4),
            Err(FallbackReason::ValidationFailed)
        );
        assert_eq!(
            LuminanceReductionGate::check_pyramid_resource_binding(&ok, 3),
            Err(FallbackReason::ValidationFailed)
        );
        // A buffer resource never conforms at any slot (kernels bind
        // Texture2D / RWTexture2D).
        let mut with_buffer = ok.clone();
        with_buffer[2].resource_type = RXGD_RESOURCE_BUFFER;
        assert_eq!(
            LuminanceReductionGate::check_pyramid_resource_binding(&with_buffer, 4),
            Err(FallbackReason::ValidationFailed)
        );
        // Zero level count is invalid.
        assert_eq!(
            LuminanceReductionGate::check_pyramid_resource_binding(&ok, 0),
            Err(FallbackReason::ValidationFailed)
        );
    }

    /// The gated pyramid attempt fails closed (and records the first missing
    /// prerequisite) when a resource handle is missing, before any dispatch.
    #[test]
    fn pyramid_attempt_missing_handle_blocks_before_dispatch() {
        let caps = luminance_dispatch_optin_caps(); // carries RXGD_CAP_SHADER_INT64
        let mut resources: Vec<RxGdResource> = (0..6)
            .map(|i| RxGdResource::texture((i as u64) + 100, 64, 64, 41))
            .collect();
        resources[4].native_handle = 0; // missing `current`
        let mut gate = LuminanceReductionGate::new();
        assert_eq!(
            gate.record_pyramid_attempt(caps, &resources, 1920, 1080, 8.0, 0.05, 0.5, false, 1, 2),
            RXGD_STATUS_FALLBACK
        );
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("pyramid_binding_invalid")
        );
    }

    /// The gated pyramid attempt fails closed on a null device handle
    /// (eligibility) before it can reach the binding check or any dispatch.
    #[test]
    fn pyramid_attempt_null_device_blocks_on_eligibility() {
        let caps = luminance_dispatch_optin_caps();
        let resources: Vec<RxGdResource> = (0..6)
            .map(|i| RxGdResource::texture((i as u64) + 100, 64, 64, 41))
            .collect();
        let mut gate = LuminanceReductionGate::new();
        assert_eq!(
            gate.record_pyramid_attempt(caps, &resources, 1920, 1080, 8.0, 0.05, 0.5, false, 0, 2),
            RXGD_STATUS_FALLBACK
        );
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("dispatch_eligibility_failed")
        );
    }

    /// Feature-off shipping bridge: even a fully valid pyramid binding with real
    /// non-null device/queue handles fails closed with
    /// `real_dispatch_path_not_linked` — no dispatch path is compiled in.
    #[cfg(not(feature = "d3d12-recording-shim"))]
    #[test]
    fn pyramid_attempt_feature_off_is_not_linked() {
        let caps = luminance_dispatch_optin_caps();
        let resources: Vec<RxGdResource> = (0..6)
            .map(|i| RxGdResource::texture((i as u64) + 100, 64, 64, 41))
            .collect();
        let mut gate = LuminanceReductionGate::new();
        assert_eq!(
            gate.record_pyramid_attempt(caps, &resources, 1920, 1080, 8.0, 0.05, 0.5, true, 1, 2),
            RXGD_STATUS_FALLBACK
        );
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("real_dispatch_path_not_linked")
        );
    }

    /// GRX-009 Wave 2: a handle-only luminance pyramid resource array exactly as
    /// Godot patch 0010 marshals it — every slot is an `RXGD_RESOURCE_TEXTURE`
    /// with the ABI header + a non-null native handle set but zero width/height
    /// (each level's extent comes from the b0 planner, not the resource struct).
    fn pyramid_handle_only_resources(count: usize) -> Vec<RxGdResource> {
        (0..count)
            .map(|i| RxGdResource {
                abi_version: RXGD_ABI_VERSION,
                struct_size: size_of::<RxGdResource>() as u32,
                resource_type: RXGD_RESOURCE_TEXTURE,
                format: 0,
                width: 0,
                height: 0,
                depth: 1,
                mip_levels: 1,
                usage_flags: 0,
                native_handle: (i as u64) + 100,
            })
            .collect()
    }

    /// Push-constant parse (hook_contract_v2 §3): the shared 28-byte b0 block
    /// round-trips through `parse_luminance_root_constants`, and any length
    /// other than exactly 28 bytes — or an out-of-`u32`-range dimension — is
    /// rejected (`None`) so the caller fails the whole pyramid closed.
    #[test]
    fn parse_luminance_root_constants_roundtrips_and_rejects_bad_input() {
        let pc = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        assert_eq!(
            parse_luminance_root_constants(&pc),
            Some((1920u32, 1080u32, 8.0f32, 0.05f32, 0.5f32))
        );
        // A source ≤ 8×8 (single final level) round-trips just the same.
        let pc_small = luminance_push_constants(8, 8, 4.0, 0.1, 1.0);
        assert_eq!(
            parse_luminance_root_constants(&pc_small),
            Some((8u32, 8u32, 4.0f32, 0.1f32, 1.0f32))
        );
        // Any length != 28 is rejected.
        for bad_len in [0usize, 16, 24, 27, 29, 32] {
            let bytes = vec![0u8; bad_len];
            assert_eq!(
                parse_luminance_root_constants(&bytes),
                None,
                "push-constant length {bad_len} must be rejected"
            );
        }
        // A source dimension whose high i64 dword is set overflows u32 → None.
        let mut overflow = pc;
        overflow[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(parse_luminance_root_constants(&overflow), None);
    }

    /// `record_pyramid_from_push_constants` fails the whole pyramid closed with
    /// `pyramid_push_constants_invalid` when the b0 block is not exactly 28
    /// bytes — before any eligibility or dispatch — under both feature legs.
    #[test]
    fn pyramid_from_push_constants_bad_length_fails_closed() {
        let caps = luminance_real_pass_optin_caps();
        let resources = pyramid_handle_only_resources(6);
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_pyramid_from_push_constants(caps, &resources, &[0u8; 16], 1, 2);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("pyramid_push_constants_invalid")
        );
        assert_eq!(
            gate.last_fallback_reason(),
            Some(FallbackReason::ValidationFailed)
        );
    }

    /// `record_pyramid_from_push_constants` parses the b0 dims and routes into
    /// the pyramid attempt: a resource array whose length does not match the
    /// planned `level_count + 2` fails closed with `pyramid_binding_invalid`
    /// (both feature legs, before any dispatch).
    #[test]
    fn pyramid_from_push_constants_shape_mismatch_fails_closed() {
        let caps = luminance_real_pass_optin_caps();
        // 1920×1080 plans four levels -> needs 6 resources; supply only 3.
        let resources = pyramid_handle_only_resources(3);
        let pc = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_pyramid_from_push_constants(caps, &resources, &pc, 1, 2);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("pyramid_binding_invalid")
        );
    }

    /// Feature-off shipping bridge: a fully valid handle-only pyramid array with
    /// a correct 28-byte b0 block parses, routes through every software gate
    /// (eligibility, binding, package identity) and fails closed only at the
    /// linked-dispatch boundary with `real_dispatch_path_not_linked`.
    #[cfg(not(feature = "d3d12-recording-shim"))]
    #[test]
    fn pyramid_from_push_constants_feature_off_reaches_dispatch_boundary() {
        let caps = luminance_real_pass_optin_caps();
        let resources = pyramid_handle_only_resources(6); // 1920×1080 -> 4 levels -> 6
        let pc = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        let mut gate = LuminanceReductionGate::new();
        let rc = gate.record_pyramid_from_push_constants(caps, &resources, &pc, 1, 2);
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        assert!(!gate.is_enabled());
        assert_eq!(
            gate.last_real_pass_blocked(),
            Some("real_dispatch_path_not_linked")
        );
    }

    /// FFI routing: exactly two resources keep the level-0 arm (backward
    /// compatible) and fall back gracefully — never mis-routed to the pyramid
    /// arm and never a hard argument error. Both feature legs.
    #[test]
    fn rxgd_record_pass_two_resources_keep_level0_arm() {
        let (resources, push_constants) = valid_luminance_binding();
        // Default caps (no bring-up / real-pass opt-in): the level-0 arm's
        // preflight fails closed on the missing 64-bit integer capability.
        let session = create_session();
        let rc = rxgd_record_pass(
            session,
            RXGD_PASS_LUMINANCE_REDUCTION,
            resources.as_ptr(),
            resources.len() as u64,
            push_constants.as_ptr(),
            push_constants.len() as u64,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 1, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        rxgd_destroy_session(session);
    }

    /// FFI routing: the front door accepts the handle-only pyramid array (zero
    /// width/height, as Godot patch 0010 marshals it) — it is NOT rejected as
    /// an invalid argument — and routes it into the pyramid arm, which fails
    /// closed with FALLBACK on the feature-off shipping bridge.
    #[cfg(not(feature = "d3d12-recording-shim"))]
    #[test]
    fn rxgd_record_pass_pyramid_front_door_accepts_handle_only_array() {
        let resources = pyramid_handle_only_resources(6);
        let pc = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        let session = create_session_with_caps(luminance_real_pass_optin_caps());
        let rc = rxgd_record_pass(
            session,
            RXGD_PASS_LUMINANCE_REDUCTION,
            resources.as_ptr(),
            resources.len() as u64,
            pc.as_ptr(),
            pc.len() as u64,
        );
        // Not RXGD_E_INVALID_ARGUMENT (the strict width/height front door would
        // have rejected the zero-extent textures) — a graceful fail-closed.
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 2, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        assert_eq!(stats.last_error, RXGD_STATUS_FALLBACK);
        rxgd_destroy_session(session);
    }

    /// FFI routing: a multi-resource array (resource_count >= 3) whose length
    /// does not match the planned pyramid falls back gracefully (not an argument
    /// error) under both feature legs — it never reaches a dispatch.
    #[test]
    fn rxgd_record_pass_pyramid_shape_mismatch_falls_back() {
        let resources = pyramid_handle_only_resources(3); // 1920×1080 needs 6
        let pc = luminance_push_constants(1920, 1080, 8.0, 0.05, 0.5);
        let session = create_session_with_caps(luminance_real_pass_optin_caps());
        let rc = rxgd_record_pass(
            session,
            RXGD_PASS_LUMINANCE_REDUCTION,
            resources.as_ptr(),
            resources.len() as u64,
            pc.as_ptr(),
            pc.len() as u64,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 3, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        rxgd_destroy_session(session);
    }

    /// FFI routing: a multi-resource pyramid array with a b0 block that is not
    /// exactly 28 bytes falls back gracefully under both feature legs.
    #[test]
    fn rxgd_record_pass_pyramid_bad_push_constants_falls_back() {
        let resources = pyramid_handle_only_resources(6);
        let pc = [0u8; 16];
        let session = create_session_with_caps(luminance_real_pass_optin_caps());
        let rc = rxgd_record_pass(
            session,
            RXGD_PASS_LUMINANCE_REDUCTION,
            resources.as_ptr(),
            resources.len() as u64,
            pc.as_ptr(),
            pc.len() as u64,
        );
        assert_eq!(rc, RXGD_STATUS_FALLBACK);
        let mut stats = zeroed_stats();
        assert_eq!(
            rxgd_collect_timestamps(session, 4, &mut stats),
            RXGD_STATUS_OK
        );
        assert_eq!(stats.recorded_passes, 0);
        assert_eq!(stats.fallback_passes, 1);
        rxgd_destroy_session(session);
    }
}
