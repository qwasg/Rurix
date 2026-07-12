// GRX-013 segment S2: math-equivalent HLSL particles_copy kernel
// (COPY_MODE_FILL_INSTANCES subset, 3D, ALIGN_DISABLED + ALIGN_BILLBOARD).
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact; the RTS0 root signature is Rurix-owned
// (rurixc::binding_layout). See PASS_CONTRACT.md sec 5 for the route rationale
// (particles_copy is an all raw-buffer SSBO pass; the workaround is used
// because Rurix's lang subset has no aggregate SSBO element types and the DXIL
// backend does not lower the sin/cos/sqrt device-math intrinsics the
// ALIGN_BILLBOARD path needs). It does NOT imply real_gpu_pass=true.
//
// Math target: Godot's
// servers/rendering/renderer_rd/shaders/particles_copy.glsl, MODE_FILL_INSTANCES
// path, for the SUPPORTED SUBSET ONLY:
//   * 3D mode only (no PARAMS_FLAG_COPY_MODE_2D; 5 vec4 per instance)
//   * align_mode in {ALIGN_DISABLED (0), ALIGN_BILLBOARD (1)}
//   * no trail (trail_size == 1): no trail interpolation, no trail_bind_poses
//   * no sort (no USE_SORT_BUFFER) and no ORDER_BY_LIFETIME reindex
//   * no userdata (USERDATA_COUNT undefined)
//   * active/inactive branch: inactive -> zero basis + (-inf,-inf,-inf,0)
//     translation column (particles_copy.glsl L324-328)
//
// Column-major note: Godot uses column-major mat4 (txform[i] is column i). To
// avoid HLSL matrix-convention ambiguity this kernel loads the mat4 as four
// explicit float4 columns and does all matrix algebra as explicit scalar/vector
// ops in the same order as the GLSL, so it is bit-faithful to a column-major
// reference.
//
// EXPLICIT GAPS (recorded in pass_manifest.json known_gaps): 2D copy mode
// (PARAMS_FLAG_COPY_MODE_2D + inv_emission_transform), MODE_FILL_SORT_BUFFER /
// COPY_MODE_FILL_INSTANCES_WITH_SORT_BUFFER (VIEW_DEPTH sort), ORDER_BY_LIFETIME
// draw-order reindex, trail interpolation + trail_bind_poses (trail_size > 1),
// userdata channels, and the align_mode variants ALIGN_Y_TO_VELOCITY (2),
// ALIGN_Z_BILLBOARD_Y_TO_VELOCITY (3), ALIGN_LOCAL_BILLBOARD (4).
//
// Binding surface (matches artifacts/particles_copy_descriptor_layout.json):
//   t0 space0 : StructuredBuffer<ParticleData>   src_particles (set0 binding1
//               of particles_copy.glsl; simulation result)
//   u0 space0 : RWStructuredBuffer<float4>        dst_instances (set0 binding2
//               of particles_copy.glsl; render instance transforms)
//   b0 space0 : 128-byte constants matching Godot's
//               ParticlesShader::CopyPushConstant exactly (32 dwords; see
//               resource_mapping.md for the field-by-field mapping). Fields for
//               out-of-scope features (inv_emission_transform, trail_*,
//               lifetime_*, align_axis) are carried for byte-exact push-constant
//               shape parity and are unused by this kernel.
//
// Thread mapping: [numthreads(64,1,1)] mirrors particles_copy.glsl
// local_size_x = 64; one thread per instance. Dispatch (ceil(N/64), 1, 1).

struct ParticleData {
    float4 xform_c0; // mat4 column 0
    float4 xform_c1; // mat4 column 1
    float4 xform_c2; // mat4 column 2
    float4 xform_c3; // mat4 column 3 (translation in .xyz)
    float3 velocity;
    uint flags;
    float4 color;
    float4 custom;
};

StructuredBuffer<ParticleData> src_particles : register(t0, space0);
RWStructuredBuffer<float4> dst_instances : register(u0, space0);

cbuffer CopyParams : register(b0, space0) {
    // Mirrors ParticlesShader::CopyPushConstant / particles_copy.glsl Params.
    float3 sort_direction;                 // dwords 0-2
    uint total_particles;                  // dword  3

    uint trail_size;                       // dword  4  (== 1 in scope)
    uint trail_total;                      // dword  5  (unused in scope)
    float frame_delta;                     // dword  6  (unused in scope)
    float frame_remainder;                 // dword  7

    float3 align_up;                       // dwords 8-10
    uint align_mode;                       // dword  11

    uint lifetime_split;                   // dword  12 (unused in scope)
    uint lifetime_reverse;                 // dword  13 (unused in scope)
    uint motion_vectors_current_offset;    // dword  14
    uint flags_bits;                       // dword  15 (order_by_lifetime|copy_mode_2d)

    float4 inv_emission_transform_0;       // dwords 16-19 (2D only; unused)
    float4 inv_emission_transform_1;       // dwords 20-23 (2D only; unused)
    float4 inv_emission_transform_2;       // dwords 24-27 (2D only; unused)

    uint align_channel_filter;             // dword  28 (CopyPushConstant.align_src)
    uint align_axis;                       // dword  29 (unused in scope)
    uint pad1;                             // dword  30
    uint pad2;                             // dword  31
};

#define PARTICLE_FLAG_ACTIVE  1u
#define PARTICLE_FLAG_TRAILED 4u

#define ALIGN_DISABLED  0u
#define ALIGN_BILLBOARD 1u

#define CHANNEL_FILTER_X 1u
#define CHANNEL_FILTER_Y 2u
#define CHANNEL_FILTER_Z 3u
#define CHANNEL_FILTER_W 4u

[numthreads(64, 1, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID) {
    uint particle = dispatch_id.x;
    if (particle >= total_particles) {
        return; // discard
    }

    ParticleData pd = src_particles[particle];

    // mat4 as four columns.
    float4 c0;
    float4 c1;
    float4 c2;
    float4 c3;

    bool active = ((pd.flags & PARTICLE_FLAG_ACTIVE) != 0u) ||
                  ((pd.flags & PARTICLE_FLAG_TRAILED) != 0u);

    if (active) {
        c0 = pd.xform_c0;
        c1 = pd.xform_c1;
        c2 = pd.xform_c2;
        c3 = pd.xform_c3;

        // trail_size == 1 in scope: skip the trail interpolation of txform[3].

        if (align_mode == ALIGN_BILLBOARD) {
            // particles_copy.glsl L176-207.
            float angle = 0.0f;
            if (align_channel_filter == CHANNEL_FILTER_X) {
                angle = pd.custom.x;
            } else if (align_channel_filter == CHANNEL_FILTER_Y) {
                angle = pd.custom.y;
            } else if (align_channel_filter == CHANNEL_FILTER_Z) {
                angle = pd.custom.z;
            } else if (align_channel_filter == CHANNEL_FILTER_W) {
                angle = pd.custom.w;
            }

            float3 axis = normalize(sort_direction);
            float s = sin(angle);
            float cN = cos(angle);
            float oc = 1.0f - cN;

            // Rodrigues rotation, GLSL mat3 constructor is column-major:
            // rotated columns rc0, rc1, rc2.
            float3 rc0 = float3(
                oc * axis.x * axis.x + cN,
                oc * axis.x * axis.y - axis.z * s,
                oc * axis.z * axis.x + axis.y * s);
            float3 rc1 = float3(
                oc * axis.x * axis.y + axis.z * s,
                oc * axis.y * axis.y + cN,
                oc * axis.y * axis.z - axis.x * s);
            float3 rc2 = float3(
                oc * axis.z * axis.x - axis.y * s,
                oc * axis.y * axis.z + axis.x * s,
                oc * axis.z * axis.z + cN);

            // new_up = rotated * align_up (column-major mat3 * vec3).
            float3 new_up = rc0 * align_up.x + rc1 * align_up.y + rc2 * align_up.z;

            // local columns.
            float3 L0 = normalize(cross(new_up, sort_direction));
            float3 L1 = new_up;
            float3 L2 = sort_direction;

            // local = local * mat3(txform): result column j = L0*tx[j].x +
            // L1*tx[j].y + L2*tx[j].z, using the xyz of the txform columns.
            float3 nc0 = L0 * c0.x + L1 * c0.y + L2 * c0.z;
            float3 nc1 = L0 * c1.x + L1 * c1.y + L2 * c1.z;
            float3 nc2 = L0 * c2.x + L1 * c2.y + L2 * c2.z;

            c0.xyz = nc0;
            c1.xyz = nc1;
            c2.xyz = nc2;
        }
        // ALIGN_DISABLED: basis columns unchanged.

        // particles_copy.glsl L306.
        c3.xyz += pd.velocity * frame_remainder;

        // trail_size == 1: skip trail_bind_poses multiply.
        // 3D: skip the PARAMS_FLAG_COPY_MODE_2D inv_emission_transform path.
    } else {
        // particles_copy.glsl L324-328: zero basis, translation to -INF so the
        // particle is invisible.
        float neg_inf = asfloat(0xFF800000u);
        c0 = float4(0.0f, 0.0f, 0.0f, 0.0f);
        c1 = float4(0.0f, 0.0f, 0.0f, 0.0f);
        c2 = float4(0.0f, 0.0f, 0.0f, 0.0f);
        c3 = float4(neg_inf, neg_inf, neg_inf, 0.0f);
    }

    // txform = transpose(txform); write rows 0..2 (particles_copy.glsl L329,
    // L340-346). Row i of the transpose = the i-th component of each column.
    float4 r0 = float4(c0.x, c1.x, c2.x, c3.x);
    float4 r1 = float4(c0.y, c1.y, c2.y, c3.y);
    float4 r2 = float4(c0.z, c1.z, c2.z, c3.z);

    uint instance_index = dispatch_id.x + motion_vectors_current_offset;
    uint write_offset = instance_index * 5u; // 3D: 3 xform rows + color + custom

    dst_instances[write_offset + 0u] = r0;
    dst_instances[write_offset + 1u] = r1;
    dst_instances[write_offset + 2u] = r2;
    dst_instances[write_offset + 3u] = pd.color;
    dst_instances[write_offset + 4u] = pd.custom;
}
