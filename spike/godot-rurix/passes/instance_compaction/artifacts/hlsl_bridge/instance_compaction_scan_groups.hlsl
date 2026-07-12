// GRX-016 segment S2: instance_compaction dispatch 2/3 — group-offset scan.
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned); Rurix-owned RTS0.
// See instance_compaction_scan_local.hlsl / PASS_CONTRACT.md sec 5.5 for the
// route rationale and the full dispatch-chain contract. It does NOT imply
// real_gpu_pass=true.
//
// This kernel runs as a SINGLE thread group (dispatch(1,1,1)) after a UAV
// barrier on group_totals. Thread t scans the per-group survivor totals
// written by scan_local and produces
//   group_offsets[t]   = exclusive prefix over group_totals[0..t)
//                        (the global rank of group t's first survivor)
//   survivor_count[0]  = total number of surviving instances
// Same groupshared Hillis-Steele scan as scan_local; u32 addition only, so
// the CPU reference must match with ZERO tolerance.
//
// CAPACITY CONTRACT (fail-closed at the later S4 gate, PASS_CONTRACT.md sec
// 5.1): this single-group second level requires num_groups <= GROUP_SIZE
// (256), i.e. total_instances <= 256 * 256 = 65536. Larger N needs a third
// scan level and is out of scope for this slice; the gate must reject it
// BEFORE any dispatch (never dispatch a chain that would read unscanned
// group totals).
//
// Binding surface (matches artifacts/instance_compaction_descriptor_layout.json,
// variant scan_groups):
//   t0 space0 : StructuredBuffer<uint>    group_totals   (u32[num_groups])
//   u0 space0 : RWStructuredBuffer<uint>  group_offsets  (u32[num_groups])
//   u1 space0 : RWStructuredBuffer<uint>  survivor_count (u32[1])
//   b0 space0 : 32-byte / 8-dword Rurix-defined CompactionParams (shared by
//               all three variants; see resource_mapping.md).

#define GROUP_SIZE 256u

StructuredBuffer<uint> group_totals : register(t0, space0);
RWStructuredBuffer<uint> group_offsets : register(u0, space0);
RWStructuredBuffer<uint> survivor_count : register(u1, space0);

cbuffer CompactionParams : register(b0, space0) {
    uint total_instances;       // dword 0: N (unused here; carried for the shared b0 shape)
    uint bitmask_words;         // dword 1: ceil(N / 32) (unused here)
    uint num_groups;            // dword 2: ceil(N / 256); MUST be <= 256
    uint transform_stride_vec4; // dword 3: float4 per instance (== 3 in scope; unused here)
    uint pad0;                  // dword 4
    uint pad1;                  // dword 5
    uint pad2;                  // dword 6
    uint pad3;                  // dword 7
};

groupshared uint scan_temp[GROUP_SIZE];

[numthreads(256, 1, 1)]
void main(uint3 group_thread_id : SV_GroupThreadID) {
    uint tid = group_thread_id.x;

    uint total = 0u;
    if (tid < num_groups) {
        total = group_totals[tid];
    }

    scan_temp[tid] = total;
    GroupMemoryBarrierWithGroupSync();

    // Hillis-Steele inclusive scan (8 fixed steps for GROUP_SIZE=256);
    // deterministic dataflow, u32 adds bounded by total_instances <= 65536.
    for (uint offset = 1u; offset < GROUP_SIZE; offset <<= 1u) {
        uint addend = 0u;
        if (tid >= offset) {
            addend = scan_temp[tid - offset];
        }
        GroupMemoryBarrierWithGroupSync();
        scan_temp[tid] += addend;
        GroupMemoryBarrierWithGroupSync();
    }

    // Exclusive prefix = inclusive prefix - own value.
    if (tid < num_groups) {
        group_offsets[tid] = scan_temp[tid] - total;
    }
    // Last lane's inclusive prefix covers every group (num_groups <= 256):
    // the global survivor count.
    if (tid == GROUP_SIZE - 1u) {
        survivor_count[0] = scan_temp[tid];
    }
}
