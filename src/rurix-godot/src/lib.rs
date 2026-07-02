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

/// GRX-009 gate for the `luminance_reduction` pass (segment 1).
///
/// The gate starts disabled and stays disabled in this segment: no compiled
/// Rurix DXIL luminance kernel exists, so `request_enable` always fails with
/// `compile_failed` and every record attempt falls back to the native Godot
/// luminance path. While the gate is closed no estimated GPU/CPU time may be
/// attributed to this pass.
#[derive(Debug)]
pub struct LuminanceReductionGate {
    enabled: bool,
    last_fallback_reason: Option<FallbackReason>,
}

impl LuminanceReductionGate {
    pub fn new() -> LuminanceReductionGate {
        LuminanceReductionGate {
            enabled: false,
            last_fallback_reason: None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn last_fallback_reason(&self) -> Option<FallbackReason> {
        self.last_fallback_reason
    }

    /// Segment 1: enabling always fails because no compiled Rurix DXIL
    /// luminance kernel exists; the gate stays disabled.
    pub fn request_enable(&mut self) -> Result<(), FallbackReason> {
        self.enabled = false;
        self.last_fallback_reason = Some(FallbackReason::CompileFailed);
        Err(FallbackReason::CompileFailed)
    }

    /// Gate decision for one record attempt: a closed gate requests fallback
    /// to the native Godot luminance path.
    fn record_outcome(&mut self) -> i32 {
        if !self.enabled {
            if self.last_fallback_reason.is_none() {
                self.last_fallback_reason = Some(FallbackReason::ManualDisabled);
            }
            return RXGD_STATUS_FALLBACK;
        }
        RXGD_STATUS_OK
    }
}

impl Default for LuminanceReductionGate {
    fn default() -> LuminanceReductionGate {
        LuminanceReductionGate::new()
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
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rxgd_abi_version() -> u32 {
    RXGD_ABI_VERSION
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

        if resource_count > 0 {
            // SAFETY: Pointer/count were validated above. We only read the
            // fixed-size records for ABI validation and never retain pointers.
            let resource_slice =
                unsafe { slice::from_raw_parts(resources, resource_count as usize) };
            for resource in resource_slice {
                let rc = validate_resource(*resource);
                if rc != RXGD_STATUS_OK {
                    inner.last_error = rc;
                    return rc;
                }
            }
        }

        if !pass_supported(pass_id) {
            inner.fallback_passes += 1;
            inner.last_error = RXGD_STATUS_FALLBACK;
            return RXGD_STATUS_FALLBACK;
        }

        if pass_id == RXGD_PASS_LUMINANCE_REDUCTION {
            // GRX-009 segment 1: the luminance_reduction gate is closed
            // (default disabled, no compiled kernel), so the pass must fall
            // back to the native Godot luminance path and no estimated
            // timing may be recorded for it.
            let rc = inner.luminance_gate.record_outcome();
            if rc != RXGD_STATUS_OK {
                inner.fallback_passes += 1;
                inner.last_error = rc;
                return rc;
            }
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
    unsafe {
        drop(Box::from_raw(session as *mut SessionInner));
    }
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
        RXGD_PASS_SSAO_BLUR | RXGD_PASS_SSIL_BLUR => 120_000,
        // RXGD_PASS_LUMINANCE_REDUCTION is gated (GRX-009) and never reaches
        // the estimated-timing path while its gate is closed.
        RXGD_PASS_TONEMAP => 70_000,
        RXGD_PASS_TAA_RESOLVE => 160_000,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_ptr(n: usize) -> *mut c_void {
        n as *mut c_void
    }

    fn create_session() -> *mut RxGdSession {
        let mut session = core::ptr::null_mut();
        let rc = rxgd_create_d3d12_session(
            fake_ptr(1),
            fake_ptr(2),
            RxGdCaps::d3d12_forward_plus(),
            &mut session,
        );
        assert_eq!(rc, RXGD_STATUS_OK);
        assert!(!session.is_null());
        session
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
        let session = create_session();
        let resource = RxGdResource::texture(123, 1920, 1080, 87);
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
}
