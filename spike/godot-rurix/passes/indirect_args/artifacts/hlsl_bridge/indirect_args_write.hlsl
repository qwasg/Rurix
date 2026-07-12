// GRX-018 segment S2: math-equivalent HLSL indirect_args WRITE kernel.
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact; the RTS0 root signature is Rurix-owned
// (rurixc::binding_layout) and SHARED with the validation kernel
// (indirect_args_validate.hlsl). See PASS_CONTRACT.md sec 5 for the route
// rationale (all raw-buffer u32 pass; the Rurix DXIL path has no u32 buffer
// views, no integer bit-op lowering, and no atomics). It does NOT imply
// real_gpu_pass=true.
//
// Math target: the Godot indirect-MultiMesh command block (there is NO native
// compute shader; the native producer is CPU code in mesh_storage.cpp):
//   * INDIRECT_MULTIMESH_COMMAND_STRIDE = 5 u32 dwords per surface
//     (mesh_storage.h L62-64);
//   * dword 0 (index_count) natively CPU-filled at _multimesh_set_mesh
//     L1674-1696 from mesh_surface_get_vertices_drawn_count;
//   * dword 1 (instance_count) natively a CPU buffer_update at
//     _multimesh_set_visible_instances L2210 — the write this kernel replaces,
//     sourcing the count from the GRX-015/016 survivor buffer instead;
//   * dwords 2-4 (first_index / vertex_offset / first_instance) natively
//     zero-initialized, backfilled here from the b0 template.
// All five dwords are written every dispatch so a stride/offset bug cannot
// hide in a stale dword; the resident validation kernel re-checks every one.
//
// SUPPORTED SUBSET (PASS_CONTRACT.md sec 5.1): one multimesh per dispatch,
// surface_count in [1, MAX_SURFACES=8], one shared survivor count for all
// surfaces (mirroring the native path, which writes the same p_visible into
// every surface block), out-of-range survivor counts clamped to
// max_instance_count.
//
// Binding surface (matches artifacts/indirect_args_descriptor_layout.json;
// shared verbatim with indirect_args_validate.hlsl — u1 is declared but never
// referenced here, and a root-signature superset is legal for both PSOs):
//   t0 space0 : StructuredBuffer<uint>   src_survivor_counts (GRX-015
//               count-only OR GRX-016 compacted-count producer buffer;
//               interface pinned in PASS_CONTRACT.md sec 4.1)
//   u0 space0 : RWStructuredBuffer<uint> dst_command_buffer (Godot
//               multimesh->command_buffer: 5 dwords per surface; the runtime
//               real-pass arm binds a Rurix-owned STAGING buffer here and only
//               copies over the live buffer after clean validation)
//   u1 space0 : RWStructuredBuffer<uint> dst_validation (validation kernel
//               output; unused by this kernel)
//   b0 space0 : 176-byte / 44-dword Rurix-owned parameter block (see
//               resource_mapping.md). The template array is declared uint4[10]
//               so its 40 dwords stay tightly packed (a uint[40] cbuffer array
//               would pad each element to 16 bytes).
//
// Thread mapping: [numthreads(64,1,1)], one thread per surface; dispatch
// (1,1,1) since surface_count <= 8 < 64.

#define MAX_SURFACES 8u

StructuredBuffer<uint> src_survivor_counts : register(t0, space0);
RWStructuredBuffer<uint> dst_command_buffer : register(u0, space0);
RWStructuredBuffer<uint> dst_validation : register(u1, space0); // unused here

cbuffer IndirectArgsParams : register(b0, space0) {
    uint surface_count;              // dword 0  (1..=MAX_SURFACES)
    uint max_instance_count;         // dword 1  (= multimesh->instances)
    uint survivor_count_word_offset; // dword 2  (word index into t0)
    uint pad0;                       // dword 3  (carried, 0)
    // dwords 4..43: MAX_SURFACES x 5-dword command templates, tightly packed:
    // {index_count, instance_count_reserved(0), first_index, vertex_offset,
    //  first_instance} per surface (indexed-draw naming; natively dwords
    // 2-4 are zero — see resource_mapping.md for the non-indexed reading).
    uint4 surface_template[10];
};

// Template word w (w in [0, 40)) of the tightly packed 40-dword region.
uint template_word(uint w) {
    return surface_template[w >> 2u][w & 3u];
}

[numthreads(64, 1, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID) {
    uint s = dispatch_id.x;
    if (s >= surface_count) {
        return; // discard (surface_count <= MAX_SURFACES enforced by the gate)
    }

    // One shared survivor count for every surface block (native parity:
    // _multimesh_set_visible_instances writes the same p_visible per surface).
    uint survivors = src_survivor_counts[survivor_count_word_offset];
    // Out-of-range clamp (defense in depth; a count above max_instance_count
    // is a producer-interface violation the validation kernel red-flags).
    uint clamped = min(survivors, max_instance_count);

    uint t = s * 5u;
    uint base = s * 5u;

    dst_command_buffer[base + 0u] = template_word(t + 0u); // index_count
    dst_command_buffer[base + 1u] = clamped;               // instance_count (GPU-dynamic)
    dst_command_buffer[base + 2u] = template_word(t + 2u); // first_index
    dst_command_buffer[base + 3u] = template_word(t + 3u); // vertex_offset
    dst_command_buffer[base + 4u] = template_word(t + 4u); // first_instance
}
