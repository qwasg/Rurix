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
