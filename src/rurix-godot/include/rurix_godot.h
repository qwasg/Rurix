/*
 * rurix_godot.h — C ABI for the opt-in Godot D3D12 Forward+ acceleration bridge.
 *
 * The ABI records Godot D3D12 resources/passes and returns explicit fallback
 * status for unsupported paths. Godot keeps its original renderer path alive at
 * all times.
 */
#ifndef RURIX_GODOT_H
#define RURIX_GODOT_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define RXGD_ABI_VERSION 1u

#define RXGD_STATUS_OK 0
#define RXGD_STATUS_FALLBACK 1
#define RXGD_E_NULL -1
#define RXGD_E_ABI -2
#define RXGD_E_UNSUPPORTED -3
#define RXGD_E_INVALID_ARGUMENT -4

#define RXGD_BACKEND_D3D12 1u
#define RXGD_RENDER_METHOD_FORWARD_PLUS 1u

/*
 * Reserved capability flags carried in RxGdCaps.flags. These reuse the
 * existing flags field, so the ABI struct layout and RXGD_ABI_VERSION are
 * unchanged. RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP is the GRX-009 segment 4b
 * gated dispatch bring-up opt-in signal; setting it never on its own makes
 * the bridge return RXGD_STATUS_OK.
 */
#define RXGD_CAP_SHADER_INT64 (1u << 0)
#define RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP (1u << 1)
/*
 * RXGD_CAP_LUMINANCE_DISPATCH_RECORD is the GRX-009 segment 4d harness-only
 * "record arm" signal, also carried in RxGdCaps.flags (no ABI struct layout
 * change; RXGD_ABI_VERSION stays 1). It is set ONLY by the explicit, test-only
 * bridge D3D12 dispatch recording harness alongside real D3D12 device/queue and
 * resource handles. The Godot module never sets it. It only has effect when the
 * bridge was built with the `d3d12-recording-shim` feature; without that feature
 * the bit is ignored and the bridge still returns RXGD_STATUS_FALLBACK.
 */
#define RXGD_CAP_LUMINANCE_DISPATCH_RECORD (1u << 2)
/*
 * RXGD_CAP_LUMINANCE_REAL_PASS is the GRX-009 segment 4h opt-in "real pass"
 * arm, also carried in RxGdCaps.flags (no ABI struct layout change;
 * RXGD_ABI_VERSION stays 1). The Godot module sets it ONLY when the
 * default-false per-pass `.../dispatch_real_pass` bring-up opt-in setting is
 * enabled (patch 0009); the default Godot config never sets it. Setting it
 * never on its own makes the bridge return RXGD_STATUS_OK: the bridge runs
 * the runtime binding preflight, the dispatch eligibility gate, the segment
 * 4h kernel-binding-kind conformance check, and the segment 4i math-pyramid-
 * parity check, and falls back with a recorded reason plus a machine-readable
 * RXGD_REAL_PASS_BLOCKED diagnostic naming the first missing prerequisite.
 *
 * HONEST FAIL-CLOSED PATH: the tracked artifact is raw-buffer (segment 3a
 * src/lib.rx lowering View/ViewMut<global, f32> to target("dx.RawBuffer",
 * float, ...)). The texture-capable kernel source src/lib_texture.rx is in
 * place (declaring Texture2D<f32>/RWTexture2D<f32>), and the compiler
 * supports the RWTexture2D<F> lang item, MirResourceType::RWTexture2D,
 * texture_target_ty, and @llvm.dx.resource.load.texture.* /
 * @llvm.dx.resource.store.texture.* emit — but the patched llc at
 * H:\llvm-dxil\build\bin\llc.exe does NOT support the
 * llvm.dx.resource.load.texture.2d intrinsic, so the texture-capable compile
 * records status=compile_failed and the bridge tracked package stays
 * raw-buffer. The Godot runtime provides Texture2D ID3D12Resource* handles
 * (segment 4e), which mismatch the tracked raw-buffer kernel's declared
 * raw_buffer_view binding kind, so the kernel-binding-kind conformance check
 * FAILS CLOSED. The FIRST missing prerequisite is `kernel_binding_kind_mismatch`
 * (NOT math_pyramid_parity_not_proven, which is the next honest blocker above
 * binding-kind and becomes reachable only when a newer patched llc supports
 * texture intrinsics and the tracked package flips to texture-capable). The
 * runtime remains `fallback_only`, `default_enable_state=disabled`, and no
 * performance claim is made.
 */
#define RXGD_CAP_LUMINANCE_REAL_PASS (1u << 3)
/*
 * RXGD_CAP_TONEMAP_REAL_PASS is the GRX-010 opt-in "real pass" arm for the
 * tonemap pass, also carried in RxGdCaps.flags (no ABI struct layout change;
 * RXGD_ABI_VERSION stays 1). The default Godot config (and tracked patch
 * 0011, whose module gate carries no resource bindings) never sets it.
 * Setting it arms the fail-closed TonemapGate real-pass attempt; the
 * shipping feature-off bridge still returns RXGD_STATUS_FALLBACK with
 * real_dispatch_path_not_linked even when every software gate passes.
 */
#define RXGD_CAP_TONEMAP_REAL_PASS (1u << 4)
/*
 * RXGD_CAP_SSAO_BLUR_REAL_PASS is the GRX-011 opt-in "real pass" arm for the
 * SSAO blur pass, also carried in RxGdCaps.flags (no ABI struct layout
 * change; RXGD_ABI_VERSION stays 1). The default Godot config (and tracked
 * patch 0012, whose module gate carries no resource bindings) never sets it.
 * Setting it arms the fail-closed SsaoBlurGate real-pass attempt; the
 * shipping feature-off bridge still returns RXGD_STATUS_FALLBACK with
 * real_dispatch_path_not_linked even when every software gate passes.
 * RXGD_PASS_SSIL_BLUR is not wired to this gate (kept on its historical
 * placeholder path).
 */
#define RXGD_CAP_SSAO_BLUR_REAL_PASS (1u << 5)

#define RXGD_RESOURCE_TEXTURE 1u
#define RXGD_RESOURCE_BUFFER 2u

#define RXGD_PASS_CLUSTER_STORE 1u
#define RXGD_PASS_SSAO_BLUR 2u
#define RXGD_PASS_SSIL_BLUR 3u
#define RXGD_PASS_LUMINANCE_REDUCTION 4u
#define RXGD_PASS_TONEMAP 5u
#define RXGD_PASS_TAA_RESOLVE 6u
#define RXGD_PASS_PARTICLES_COPY 7u
#define RXGD_PASS_GPU_CULLING 8u
#define RXGD_PASS_INDIRECT_ARGS 9u
#define RXGD_PASS_FUSED_POST_CHAIN 10u

typedef struct RxGdCaps {
	uint32_t abi_version;
	uint32_t struct_size;
	uint32_t backend;
	uint32_t render_method;
	uint32_t flags;
	uint32_t vendor_id;
	uint32_t device_id;
	uint8_t adapter_luid[8];
} RxGdCaps;

typedef struct RxGdResource {
	uint32_t abi_version;
	uint32_t struct_size;
	uint32_t resource_type;
	uint32_t format;
	uint32_t width;
	uint32_t height;
	uint32_t depth;
	uint32_t mip_levels;
	uint64_t usage_flags;
	uint64_t native_handle;
} RxGdResource;

typedef struct RxGdFrameStats {
	uint32_t abi_version;
	uint32_t struct_size;
	uint64_t frame_id;
	uint64_t recorded_passes;
	uint64_t fallback_passes;
	uint64_t registered_resources;
	uint64_t gpu_time_ns;
	uint64_t cpu_record_ns;
	int32_t last_error;
} RxGdFrameStats;

typedef struct RxGdSession RxGdSession;

uint32_t rxgd_abi_version(void);
/*
 * Returns 1 when the bridge was built with the GRX-009 segment 4d
 * `d3d12-recording-shim` feature (the test-only real D3D12 dispatch recording
 * path is linked), 0 otherwise. The default shipping bridge returns 0.
 */
int32_t rxgd_dispatch_recording_shim_available(void);
int32_t rxgd_create_d3d12_session(void *device, void *queue, RxGdCaps caps, RxGdSession **out_session);
int32_t rxgd_register_texture(RxGdSession *session, RxGdResource resource);
int32_t rxgd_register_buffer(RxGdSession *session, RxGdResource resource);
int32_t rxgd_record_pass(
	RxGdSession *session,
	uint32_t pass_id,
	const RxGdResource *resources,
	uint64_t resource_count,
	const uint8_t *push_constants,
	uint64_t push_constant_size);
int32_t rxgd_collect_timestamps(RxGdSession *session, uint64_t frame_id, RxGdFrameStats *out_stats);
void rxgd_destroy_session(RxGdSession *session);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* RURIX_GODOT_H */
