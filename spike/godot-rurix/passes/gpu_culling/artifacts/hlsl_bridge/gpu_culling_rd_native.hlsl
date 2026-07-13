// GRX-015 Route B rd_native: math-equivalent HLSL gpu_culling kernel, RD-native
// container variant (count-only conservative frustum cull over an indirect
// MultiMesh's instance transforms).
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). Same math as the shim
// bridge kernel gpu_culling_frustum_count.hlsl; the ONLY differences are the
// binding surface, tuned so a Godot RenderingShaderContainerD3D12 container can
// be emitted for the RD-native (main RenderingDevice) path:
//
//   * The 6 frustum planes MOVE OUT of the b0 root constants into a
//     StructuredBuffer<float4> at register t1 (a second SRV, aggregated with the
//     transform SRV t0 into one descriptor table range — the taa_resolve 5-SRV
//     precedent). This shrinks b0 from 144 bytes to 48 bytes so it fits the
//     RD/D3D12 128-byte 32-bit-root-constant window (rendering_device.cpp:6101);
//     the 144-byte b0 is rejected by generate_rd_container.py as
//     push_constant_too_large and CANNOT drive an RD-native container. The CBV
//     route is a dead path here (parse_rts0 rejects root-descriptor CBVs and the
//     Rurix binding_layout always emits CBVs as root descriptors).
//   * Everything else — the conservative bounding-sphere test, the 6-plane
//     inward-facing cull, the InterlockedOr bitmask write, and the per-surface
//     InterlockedAdd(+1) count write — is byte-for-byte the same math as the
//     shim kernel (see PASS_CONTRACT.md sec 5.1 and gpu_culling_frustum_count.hlsl).
//
// Runtime binding (matches artifacts/gpu_culling_rd_native_descriptor_layout.json):
//   t0 space0 : StructuredBuffer<float>   src_transforms (Godot multimesh->buffer;
//               12-float row-major 3x4 lanes per 3D instance)
//   t1 space0 : StructuredBuffer<float4>  frustum_planes (6 normalized
//               inward-facing planes (nx, ny, nz, d); Rurix-owned per-frame
//               buffer_update'd from the render camera)
//   u0 space0 : RWStructuredBuffer<uint>  dst_commands (Godot
//               multimesh->command_buffer; only instance_count_dword_index of
//               each command_stride_dwords-sized surface block is accumulated)
//   u1 space0 : RWStructuredBuffer<uint>  dst_visibility (Rurix-allocated bitmask
//               u32[ceil(N/32)]; the GRX-016/018 interface)
//   b0 space0 : 48-byte / 12-dword Rurix-defined constants (see below)
//
// Pre-dispatch runtime responsibility (patch 0046, main RenderingDevice): each
// surface's instance-count dword and the whole visibility bitmask are zeroed via
// RD::buffer_clear BEFORE compute_list_begin (buffer_clear is forbidden while a
// compute list is active), so the InterlockedAdd/InterlockedOr accumulate from 0.
// All other command dwords (dword 0 = vertices-drawn count) are never touched.
//
// Thread mapping: [numthreads(64,1,1)], one thread per instance; dispatch
// (ceil(instance_count/64), 1, 1).

StructuredBuffer<float> src_transforms : register(t0, space0);
StructuredBuffer<float4> frustum_planes : register(t1, space0);
RWStructuredBuffer<uint> dst_commands : register(u0, space0);
RWStructuredBuffer<uint> dst_visibility : register(u1, space0);

cbuffer CullParams : register(b0, space0) {
    // Rurix-defined 48-byte / 12-dword layout (frustum planes moved to t1).
    uint instance_count;                   // dword 0
    uint motion_vectors_current_offset;    // dword 1 (exercised at 0)
    uint transform_stride_floats;          // dword 2 (== 12 in scope)
    uint surface_count;                    // dword 3

    uint command_stride_dwords;            // dword 4 (== 5, mirrors
                                           //   INDIRECT_MULTIMESH_COMMAND_STRIDE)
    uint instance_count_dword_index;       // dword 5 (== 1, mirrors the
                                           //   +sizeof(uint32_t) of L2210)
    float mesh_bound_center_local_x;       // dword 6
    float mesh_bound_center_local_y;       // dword 7

    float mesh_bound_center_local_z;       // dword 8
    float mesh_bound_radius_local;         // dword 9 (half local AABB diagonal)
    uint pad1;                             // dword 10
    uint pad2;                             // dword 11
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
        // Count-only command write: +1 into EACH surface's instance-count dword
        // (the same dword the CPU writes at mesh_storage.cpp L2210); assumes the
        // dword was zeroed before this dispatch.
        for (uint s = 0u; s < surface_count; s++) {
            InterlockedAdd(
                dst_commands[s * command_stride_dwords + instance_count_dword_index],
                1u);
        }
    }
}
