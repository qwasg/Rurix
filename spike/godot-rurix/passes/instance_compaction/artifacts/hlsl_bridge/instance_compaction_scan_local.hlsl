// GRX-016 segment S2: instance_compaction dispatch 1/3 — per-group local scan.
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact; the RTS0 root signature is Rurix-owned
// (rurixc::binding_layout). See PASS_CONTRACT.md sec 5.5 for the route
// rationale (all raw-buffer SSBO pass; the workaround is used because the
// DXIL compute-body lowering has f32-only buffer views, no integer bit-op
// lowering, and rejects `shared let` / group barriers, all three of which
// this scan needs). It does NOT imply real_gpu_pass=true.
//
// Math target: unlike GRX-009..014 there is NO native Godot compaction
// shader to mirror — Godot's only visibility lever for a MultiMesh is the
// CPU-side "draw the first N instances" contract
// (MeshStorage::multimesh_get_instances_to_draw, mesh_storage.h L721-728).
// The correctness reference is the CPU stable-stream-compaction reference in
// generate_math_parity_evidence.py (integer-exact, zero tolerance).
//
// Dispatch chain (see PASS_CONTRACT.md sec 5.1):
//   D1 scan_local   dispatch(ceil(N/256), 1, 1)   <- THIS KERNEL
//      UAV barrier on local_prefix + group_totals
//   D2 scan_groups  dispatch(1, 1, 1)
//      UAV barrier on group_offsets + survivor_count
//   D3 scatter      dispatch(ceil(N/256), 1, 1)
//
// This kernel: thread p reads its visibility bit from the GRX-015 gpu_culling
// bitmask (u32[ceil(N/32)], bit p = word p>>5, bit p&31), runs a groupshared
// Hillis-Steele inclusive scan over the group's 256 bits, and writes
//   local_prefix[p]    = exclusive prefix of the bit within its group
//   group_totals[gid]  = number of surviving instances in group gid
// All math is u32 addition on 0/1 values (max sum 256): integer-exact and
// order-deterministic, so the CPU reference must match with ZERO tolerance.
//
// Bits of the last mask word beyond total_instances-1 are IGNORED via the
// p < total_instances bound (GRX-015 is not required to zero-pad the tail).
//
// Binding surface (matches artifacts/instance_compaction_descriptor_layout.json,
// variant scan_local):
//   t0 space0 : StructuredBuffer<uint>    visibility_mask (GRX-015 output)
//   u0 space0 : RWStructuredBuffer<uint>  local_prefix    (u32[N])
//   u1 space0 : RWStructuredBuffer<uint>  group_totals    (u32[num_groups])
//   b0 space0 : 32-byte / 8-dword Rurix-defined CompactionParams (shared by
//               all three variants; see resource_mapping.md).

#define GROUP_SIZE 256u

StructuredBuffer<uint> visibility_mask : register(t0, space0);
RWStructuredBuffer<uint> local_prefix : register(u0, space0);
RWStructuredBuffer<uint> group_totals : register(u1, space0);

cbuffer CompactionParams : register(b0, space0) {
    uint total_instances;       // dword 0: N
    uint bitmask_words;         // dword 1: ceil(N / 32)
    uint num_groups;            // dword 2: ceil(N / 256)
    uint transform_stride_vec4; // dword 3: float4 per instance (== 3 in scope; unused here)
    uint pad0;                  // dword 4
    uint pad1;                  // dword 5
    uint pad2;                  // dword 6
    uint pad3;                  // dword 7
};

groupshared uint scan_temp[GROUP_SIZE];

[numthreads(256, 1, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID,
          uint3 group_thread_id : SV_GroupThreadID,
          uint3 group_id : SV_GroupID) {
    uint p = dispatch_id.x;
    uint tid = group_thread_id.x;

    // Out-of-range threads contribute 0 (also makes garbage bits in the last
    // mask word beyond total_instances-1 unreachable).
    uint vis = 0u;
    if (p < total_instances) {
        vis = (visibility_mask[p >> 5u] >> (p & 31u)) & 1u;
    }

    scan_temp[tid] = vis;
    GroupMemoryBarrierWithGroupSync();

    // Hillis-Steele inclusive scan (8 fixed steps for GROUP_SIZE=256).
    // Deterministic dataflow: each slot is written only by its own thread,
    // reads are separated from writes by barriers, and u32 adds of 0/1
    // values cannot overflow, so the result is bit-exact.
    for (uint offset = 1u; offset < GROUP_SIZE; offset <<= 1u) {
        uint addend = 0u;
        if (tid >= offset) {
            addend = scan_temp[tid - offset];
        }
        GroupMemoryBarrierWithGroupSync();
        scan_temp[tid] += addend;
        GroupMemoryBarrierWithGroupSync();
    }

    // Exclusive prefix = inclusive prefix - own value (no extra barrier).
    if (p < total_instances) {
        local_prefix[p] = scan_temp[tid] - vis;
    }
    // Last lane's inclusive prefix is the group's survivor total.
    if (tid == GROUP_SIZE - 1u) {
        group_totals[group_id.x] = scan_temp[tid];
    }
}
