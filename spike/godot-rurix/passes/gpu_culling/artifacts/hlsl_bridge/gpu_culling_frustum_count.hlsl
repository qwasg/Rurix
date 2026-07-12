// GRX-015 segment S2: math-equivalent HLSL gpu_culling kernel (count-only
// conservative frustum cull over an indirect MultiMesh's instance transforms).
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact; the RTS0 root signature is Rurix-owned
// (rurixc::binding_layout). See PASS_CONTRACT.md sec 5.3 for the route
// rationale (gpu_culling is an all raw-buffer SSBO pass; the workaround is
// used because the Rurix DXIL lowering has no u32 buffer views, no atomic
// intrinsics on any backend, no integer bit-operation lowering, and no sqrt
// lowering on the DXIL path). It does NOT imply real_gpu_pass=true.
//
// Math target: Rurix-defined (ADDITIVE pass — no native Godot compute shader
// is replaced; the native path is the CPU write of the command block's
// instance-count dword at mesh_storage.cpp _multimesh_set_visible_instances
// L2210, byte offset (surface_index*5+1)*4):
//   * one thread per instance; [numthreads(64,1,1)]; dispatch
//     (ceil(instance_count/64), 1, 1)
//   * conservative bounding sphere: world_center = rows * (center_local, 1);
//     world_radius = radius_local * frobenius_norm(basis). The Frobenius norm
//     is an upper bound on the basis spectral norm, so the test may keep a
//     truly-invisible instance visible but can never cull a visible one.
//   * 6 normalized inward-facing planes: dist = dot(n, world_center) + d;
//     the instance is culled iff ANY plane has dist < -world_radius.
//   * visible instance: InterlockedOr its bit into the dst_visibility
//     bitmask word (bit (i & 31) of word (i >> 5); the GRX-016/018 shared
//     interface), and InterlockedAdd(+1) into EACH surface's instance-count
//     dword of dst_commands (mirroring the CPU loop over surface_count at
//     mesh_storage.cpp L2205-2212). Count-only: no transform remap or
//     compaction (GRX-016 territory).
//   * the instance-count dwords and the bitmask buffer are assumed ZEROED
//     before the dispatch (runtime responsibility; GRX-014 zeroed-destination
//     convention); all other command dwords are never touched, so the
//     CPU-initialized dword 0 (vertices-drawn count) survives.
//
// EXPLICIT GAPS (recorded in pass_manifest.json known_gaps): precise OBB /
// transformed-AABB test, occlusion culling, LOD, 2D transform format,
// color/custom stride variants (transform_stride_floats carried, exercised at
// 12), motion_vectors_current_offset != 0 (carried, exercised at 0),
// per-surface differing visibility, compaction (GRX-016), indirect args
// beyond the count dword (GRX-018).
//
// Binding surface (matches artifacts/gpu_culling_descriptor_layout.json):
//   t0 space0 : StructuredBuffer<float>     src_transforms (Godot
//               multimesh->buffer; 12-float row-major 3x4 lanes per 3D
//               instance at (motion_vectors_current_offset + i) *
//               transform_stride_floats; mesh_storage.cpp L1880-1915)
//   u0 space0 : RWStructuredBuffer<uint>    dst_commands (Godot
//               multimesh->command_buffer; INDIRECT_MULTIMESH_COMMAND_STRIDE
//               = 5 dwords per surface; ONLY dword instance_count_dword_index
//               of each block is atomically accumulated)
//   u1 space0 : RWStructuredBuffer<uint>    dst_visibility (Rurix-allocated
//               bitmask u32[ceil(N/32)]; the GRX-016/018 input interface)
//   b0 space0 : 144-byte / 36-dword Rurix-defined constants (no Godot push
//               constant exists for this additive pass; see
//               resource_mapping.md for the field-by-field table).
//
// Parity note: the float intermediates feed ONLY comparisons; the outputs are
// pure u32 (bitmask words + counts) and are compared at ZERO tolerance. The
// S3 fixtures assert a classification-margin floor (|dist + world_radius| >=
// 1e-3 for every instance x plane), so ULP-level GPU reassociation / FMA /
// sqrt differences cannot flip any classification.

StructuredBuffer<float> src_transforms : register(t0, space0);
RWStructuredBuffer<uint> dst_commands : register(u0, space0);
RWStructuredBuffer<uint> dst_visibility : register(u1, space0);

cbuffer CullParams : register(b0, space0) {
    // Rurix-defined layout (resource_mapping.md); planes normalized,
    // inward-facing (dist = dot(n, p) + d >= 0 on the visible side).
    float4 frustum_planes[6];              // dwords 0-23: (nx, ny, nz, d) x 6

    uint instance_count;                   // dword 24
    uint motion_vectors_current_offset;    // dword 25 (exercised at 0)
    uint transform_stride_floats;          // dword 26 (== 12 in scope)
    uint surface_count;                    // dword 27

    uint command_stride_dwords;            // dword 28 (== 5, mirrors
                                           //   INDIRECT_MULTIMESH_COMMAND_STRIDE)
    uint instance_count_dword_index;       // dword 29 (== 1, mirrors the
                                           //   +sizeof(uint32_t) of L2210)
    float mesh_bound_center_local_x;       // dword 30
    float mesh_bound_center_local_y;       // dword 31

    float mesh_bound_center_local_z;       // dword 32
    float mesh_bound_radius_local;         // dword 33 (half local AABB diagonal)
    uint pad1;                             // dword 34
    uint pad2;                             // dword 35
};

[numthreads(64, 1, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID) {
    uint instance = dispatch_id.x;
    if (instance >= instance_count) {
        return; // discard
    }

    uint base = (motion_vectors_current_offset + instance) * transform_stride_floats;

    // Row-major 3x4 lanes (mesh_storage.cpp _multimesh_instance_set_transform):
    // r0 = (basis.rows[0], origin.x), r1 = (.., origin.y), r2 = (.., origin.z).
    float4 r0 = float4(src_transforms[base + 0u], src_transforms[base + 1u],
                       src_transforms[base + 2u], src_transforms[base + 3u]);
    float4 r1 = float4(src_transforms[base + 4u], src_transforms[base + 5u],
                       src_transforms[base + 6u], src_transforms[base + 7u]);
    float4 r2 = float4(src_transforms[base + 8u], src_transforms[base + 9u],
                       src_transforms[base + 10u], src_transforms[base + 11u]);

    // World-space bound center: rows * (center_local, 1).
    float cx = mesh_bound_center_local_x;
    float cy = mesh_bound_center_local_y;
    float cz = mesh_bound_center_local_z;
    float wx = ((r0.x * cx + r0.y * cy) + r0.z * cz) + r0.w;
    float wy = ((r1.x * cx + r1.y * cy) + r1.z * cz) + r1.w;
    float wz = ((r2.x * cx + r2.y * cy) + r2.z * cz) + r2.w;

    // Conservative world radius: radius_local * frobenius_norm(basis).
    // Frobenius >= spectral norm, so the sphere never shrinks below the true
    // transformed bound (never over-culls). Left-to-right accumulation.
    float fro2 = r0.x * r0.x + r0.y * r0.y + r0.z * r0.z
               + r1.x * r1.x + r1.y * r1.y + r1.z * r1.z
               + r2.x * r2.x + r2.y * r2.y + r2.z * r2.z;
    float world_radius = mesh_bound_radius_local * sqrt(fro2);

    // 6-plane test: culled iff fully outside any plane.
    bool visible = true;
    [unroll]
    for (uint p = 0u; p < 6u; p++) {
        float4 pl = frustum_planes[p];
        float dist = ((pl.x * wx + pl.y * wy) + pl.z * wz) + pl.w;
        if (dist < -world_radius) {
            visible = false;
        }
    }

    if (visible) {
        // GRX-016/018 shared interface: bit (i & 31) of word (i >> 5).
        InterlockedOr(dst_visibility[instance >> 5u], 1u << (instance & 31u));
        // Count-only command write: +1 into EACH surface's instance-count
        // dword (the same dword the CPU writes at mesh_storage.cpp L2210);
        // assumes the dword was zeroed before this dispatch.
        for (uint s = 0u; s < surface_count; s++) {
            InterlockedAdd(
                dst_commands[s * command_stride_dwords + instance_count_dword_index],
                1u);
        }
    }
}
