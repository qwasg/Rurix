// GRX-016 segment S2: instance_compaction dispatch 3/3 — survivor scatter.
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned); Rurix-owned RTS0.
// See instance_compaction_scan_local.hlsl / PASS_CONTRACT.md sec 5.5 for the
// route rationale and the full dispatch-chain contract. It does NOT imply
// real_gpu_pass=true.
//
// This kernel runs after a UAV barrier on group_offsets + survivor_count.
// Thread p re-reads its visibility bit; if instance p survives, its global
// compacted rank is
//   rank = group_offsets[p / 256] + local_prefix[p]
// and the kernel moves the instance's transform payload to the front of the
// staging buffer:
//   dst_transforms[rank*S .. rank*S+S-1] = src_transforms[p*S .. p*S+S-1]
// with S = transform_stride_vec4 (== 3 in scope: the 12-float 3D
// transform-only MultiMesh stride, 3 float4 rows of basis-row+origin, see
// mesh_storage.cpp _multimesh_instance_set_transform L1895-1912).
//
// The move is BIT-PRESERVING: buffers are declared uint4 and no arithmetic
// touches the payload, so the compacted output must be byte-exact against
// the CPU reference (zero tolerance). Non-survivors write nothing: dst
// elements at rank >= survivor_count are left untouched (don't-care under
// the Godot "draw the first N instances" consumption contract; the parity
// fixtures zero-initialize dst and compare the WHOLE buffer byte-exactly).
//
// ORDER CONTRACT (PASS_CONTRACT.md sec 5.2): the exclusive-prefix rank is
// monotone in p, so compaction is STABLE — survivors keep their relative
// index order — but absolute instance indices change (instance p lands in
// slot rank(p)). Opaque-only applicability; alpha-blended materials and any
// absolute-instance-index-keyed consumer are out of scope.
//
// Binding surface (matches artifacts/instance_compaction_descriptor_layout.json,
// variant scatter):
//   t0 space0 : StructuredBuffer<uint>    visibility_mask (GRX-015 output)
//   t1 space0 : StructuredBuffer<uint4>   src_transforms  (MultiMesh buffer,
//               12 floats = 3 uint4 per instance, read bit-preserving)
//   t2 space0 : StructuredBuffer<uint>    local_prefix    (scan_local output)
//   t3 space0 : StructuredBuffer<uint>    group_offsets   (scan_groups output)
//   u0 space0 : RWStructuredBuffer<uint4> dst_transforms  (compacted staging)
//   b0 space0 : 32-byte / 8-dword Rurix-defined CompactionParams (shared by
//               all three variants; see resource_mapping.md).

#define GROUP_SIZE 256u

StructuredBuffer<uint> visibility_mask : register(t0, space0);
StructuredBuffer<uint4> src_transforms : register(t1, space0);
StructuredBuffer<uint> local_prefix : register(t2, space0);
StructuredBuffer<uint> group_offsets : register(t3, space0);
RWStructuredBuffer<uint4> dst_transforms : register(u0, space0);

cbuffer CompactionParams : register(b0, space0) {
    uint total_instances;       // dword 0: N
    uint bitmask_words;         // dword 1: ceil(N / 32) (unused here)
    uint num_groups;            // dword 2: ceil(N / 256) (unused here)
    uint transform_stride_vec4; // dword 3: float4 per instance (== 3 in scope,
                                //          enforced by the later S4 gate)
    uint pad0;                  // dword 4
    uint pad1;                  // dword 5
    uint pad2;                  // dword 6
    uint pad3;                  // dword 7
};

[numthreads(256, 1, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID,
          uint3 group_id : SV_GroupID) {
    uint p = dispatch_id.x;
    if (p >= total_instances) {
        return; // discard (also shields garbage tail bits of the last mask word)
    }

    uint vis = (visibility_mask[p >> 5u] >> (p & 31u)) & 1u;
    if (vis == 0u) {
        return; // culled instance: writes nothing anywhere
    }

    uint rank = group_offsets[group_id.x] + local_prefix[p];
    uint stride = transform_stride_vec4;
    uint src_base = p * stride;
    uint dst_base = rank * stride;
    for (uint k = 0u; k < stride; ++k) {
        dst_transforms[dst_base + k] = src_transforms[src_base + k];
    }
}
