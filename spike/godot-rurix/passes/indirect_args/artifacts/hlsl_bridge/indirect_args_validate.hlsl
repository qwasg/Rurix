// GRX-018 segment S2: RESIDENT validation red-leg kernel for indirect_args.
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact; the RTS0 root signature is Rurix-owned
// (rurixc::binding_layout) and SHARED with the write kernel
// (indirect_args_write.hlsl). It does NOT imply real_gpu_pass=true.
//
// MANDATE (GRX_PLAN.md GRX-018: "any validation mismatch -> immediate
// fallback"; PASS_CONTRACT.md sec 5.4): wrong args or a surface-stride
// mismatch is a GPU-hang-class risk, because draw_list_draw_indirect
// (render_forward_clustered.cpp L610) consumes whatever 5 dwords sit at
// surface_index * INDIRECT_MULTIMESH_COMMAND_STRIDE. This kernel is therefore
// a RESIDENT part of the pass, not a test-only artifact: the real-pass arm
// always runs write -> UAV barrier -> validate -> readback, and only copies
// the staging blocks over the live command buffer when both counters below
// read zero. On any nonzero counter the copy is skipped and the native CPU
// contents stay live (fallback_reason = validation_failed).
//
// Checks per surface s (bitmask written to dst_validation[2 + s]):
//   bit 0..4 : generated dword c != expected dword c, where the expected
//              block is the b0 template with dword 1 recomputed as
//              min(survivors, max_instance_count);
//   bit 5    : in-buffer instance_count > max_instance_count (an unclamped
//              or foreign writer — the exact class of bug that hangs GPUs);
//   bit 6    : survivors > max_instance_count (producer-interface violation;
//              the write kernel's clamp fired — input cannot be trusted).
// Counters (InterlockedAdd; buffer zeroed before this dispatch):
//   dst_validation[0] = mismatch_count      (surfaces with any of bits 0-5)
//   dst_validation[1] = clamp_trigger_count (surfaces with bit 6)
//
// Binding surface: identical to indirect_args_write.hlsl (t0 survivor counts,
// u0 command buffer — read through the UAV, u1 validation output, b0 shared
// 176-byte / 44-dword parameter block); one Rurix-owned RTS0 serves both PSOs.
//
// Thread mapping: [numthreads(64,1,1)], one thread per surface; dispatch
// (1,1,1) since surface_count <= 8 < 64. Each thread owns its per-surface
// mask word; only the two counter words are contended (atomics).

#define MAX_SURFACES 8u

StructuredBuffer<uint> src_survivor_counts : register(t0, space0);
RWStructuredBuffer<uint> dst_command_buffer : register(u0, space0);
RWStructuredBuffer<uint> dst_validation : register(u1, space0);

cbuffer IndirectArgsParams : register(b0, space0) {
    uint surface_count;              // dword 0  (1..=MAX_SURFACES)
    uint max_instance_count;         // dword 1  (= multimesh->instances)
    uint survivor_count_word_offset; // dword 2  (word index into t0)
    uint pad0;                       // dword 3  (carried, 0)
    uint4 surface_template[10];      // dwords 4..43 (see write kernel)
};

uint template_word(uint w) {
    return surface_template[w >> 2u][w & 3u];
}

[numthreads(64, 1, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID) {
    uint s = dispatch_id.x;
    if (s >= surface_count) {
        return; // discard
    }

    uint survivors = src_survivor_counts[survivor_count_word_offset];
    uint expected_instance_count = min(survivors, max_instance_count);

    uint t = s * 5u;
    uint base = s * 5u;
    uint mask = 0u;

    if (dst_command_buffer[base + 0u] != template_word(t + 0u)) {
        mask |= 1u << 0u; // index_count mismatch
    }
    if (dst_command_buffer[base + 1u] != expected_instance_count) {
        mask |= 1u << 1u; // instance_count mismatch
    }
    if (dst_command_buffer[base + 2u] != template_word(t + 2u)) {
        mask |= 1u << 2u; // first_index mismatch
    }
    if (dst_command_buffer[base + 3u] != template_word(t + 3u)) {
        mask |= 1u << 3u; // vertex_offset mismatch
    }
    if (dst_command_buffer[base + 4u] != template_word(t + 4u)) {
        mask |= 1u << 4u; // first_instance mismatch
    }
    if (dst_command_buffer[base + 1u] > max_instance_count) {
        mask |= 1u << 5u; // in-buffer clamp violation (unclamped writer)
    }
    if (survivors > max_instance_count) {
        mask |= 1u << 6u; // producer violation: clamp fired (red flag)
    }

    dst_validation[2u + s] = mask;

    if ((mask & 0x3Fu) != 0u) {
        uint prev_mismatch;
        InterlockedAdd(dst_validation[0], 1u, prev_mismatch);
    }
    if ((mask & 0x40u) != 0u) {
        uint prev_clamp;
        InterlockedAdd(dst_validation[1], 1u, prev_clamp);
    }
}
