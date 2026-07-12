// GRX-012 segment A: math-equivalent HLSL TAA resolve kernel (single
// full-resolution resolve, Spartan-derived).
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact under the owner-approved texture artifact
// provenance policy (GRX-009 texture_artifact_provenance_policy.json, which
// applies to every texture compute pass): HLSL compiled by DXC + Rurix-owned
// RTS0 + DXV validation may serve as the runtime-mappable canonical package.
// It does NOT imply real_gpu_pass=true.
//
// Math target: Godot's
// servers/rendering/renderer_rd/shaders/effects/taa_resolve.glsl (single
// full-resolution TAA resolve), faithfully ported for the SUPPORTED SUBSET:
//   * groupshared 10x10 tile (8x8 group + 1 border) caching clamped
//     color+depth loads (populate_group_shared_memory)
//   * get_closest_pixel_velocity_3x3 (3x3 min-depth velocity poll, native
//     border-offset quirk preserved)
//   * 9-tap Catmull-Rom history sampling (textureLod bilinear reproduced as
//     explicit float 4-tap Load bilinear with clamp addressing)
//   * clip_history_3x3 (clip_aabb variance clipping, adaptive box size)
//   * get_factor_disocclusion + out-of-screen reset + luminance-diff flicker
//     suppression
//   * Reinhard-domain blend (base 1/16) then inverse Reinhard
//
// EXPLICIT GAPS (recorded in pass_manifest.json known_gaps): hardware bilinear
// sub-texel fixed-point rounding (this kernel does float bilinear), rgba16f /
// rg16f half storage quantization (this kernel reads/writes float), the history
// physical maintenance chain, one-frame latency (draw_graph replacement), and
// multiview.
//
// Binding surface (matches artifacts/taa_resolve_descriptor_layout.json):
//   t0 space0 : Texture2D<float4>   color_buffer         (current-frame HDR)
//   t1 space0 : Texture2D<float>    depth_buffer         (texelFetch -> Load)
//   t2 space0 : Texture2D<float2>   velocity_buffer      (current motion)
//   t3 space0 : Texture2D<float2>   last_velocity_buffer (previous motion)
//   t4 space0 : Texture2D<float4>   history_buffer       (previous resolve)
//   u0 space0 : RWTexture2D<float4> output_buffer        (resolve output)
//   b0 space0 : 28-byte constants matching the canonical Rurix root-constant
//               layout (root_parameter_index 0, 7 dwords):
//                 dwords 0-1 : source_width            (i64: low, high)
//                 dwords 2-3 : source_height           (i64: low, high)
//                 dword  4   : disocclusion_threshold  (f32)
//                 dword  5   : variance_dynamic        (f32)
//                 dword  6   : reserved0               (f32, unused)
//               The i64 dims are declared as uint2 (low, high dword) so the
//               DXIL stays plain cs_6_0 without the optional Int64 shader
//               capability; only the low dword is consumed (dims < 2^32) and
//               the high dword must be written as 0 by the runtime.
//               resolution = float2(source_width, source_height).
//
// Thread mapping: [numthreads(8,8,1)], one thread per output texel (output
// extent == color extent). Dispatch with (ceil(width/8), ceil(height/8), 1).

Texture2D<float4> color_buffer : register(t0, space0);
Texture2D<float> depth_buffer : register(t1, space0);
Texture2D<float2> velocity_buffer : register(t2, space0);
Texture2D<float2> last_velocity_buffer : register(t3, space0);
Texture2D<float4> history_buffer : register(t4, space0);
RWTexture2D<float4> output_buffer : register(u0, space0);

cbuffer TaaResolveConstants : register(b0, space0)
{
    uint2 source_width_u64;         // dwords 0-1: i64 source_width  (low, high)
    uint2 source_height_u64;        // dwords 2-3: i64 source_height (low, high)
    float disocclusion_threshold;   // dword 4
    float variance_dynamic;         // dword 5
    float reserved0;                // dword 6 (unused, canonical shape parity)
};

#define GROUP_SIZE 8
#define FLT_MIN_TAA 0.00000001f
#define FLT_MAX_TAA 32767.0f
#define RPC_9 0.11111111111f
#define RPC_16 0.0625f
#define DISOCCLUSION_SCALE 0.01f
#define BORDER 1
#define TILE_DIM 10   // GROUP_SIZE + BORDER * 2

static const int2 kOffsets3x3[9] = {
    int2(-1, -1), int2(0, -1), int2(1, -1),
    int2(-1, 0),  int2(0, 0),  int2(1, 0),
    int2(-1, 1),  int2(0, 1),  int2(1, 1),
};

static const float3 lumCoeff = float3(0.299f, 0.587f, 0.114f);

groupshared float3 tile_color[TILE_DIM][TILE_DIM];
groupshared float tile_depth[TILE_DIM][TILE_DIM];

float3 reinhard(float3 hdr) { return hdr / (hdr + 1.0f); }
float3 reinhard_inverse(float3 sdr) { return sdr / (1.0f - sdr); }

// depth_buffer texelFetch (integer coords, mip 0).
float get_depth(int2 tid) { return depth_buffer.Load(int3(tid, 0)); }

void store_color_depth(uint2 group_thread_id, int2 tid, uint w, uint h)
{
    // Out of bounds clamp (native clamps thread_id to [0, resolution-1]).
    tid = clamp(tid, int2(0, 0), int2(int(w) - 1, int(h) - 1));
    tile_color[group_thread_id.x][group_thread_id.y] = color_buffer.Load(int3(tid, 0)).rgb;
    tile_depth[group_thread_id.x][group_thread_id.y] = get_depth(tid);
}

void populate_group_shared_memory(uint2 group_id, uint group_index, uint w, uint h)
{
    int2 group_top_left = int2(group_id) * GROUP_SIZE - BORDER;
    if (group_index < (TILE_DIM * TILE_DIM / 4)) { // 100/4 = 25
        uint idx[4];
        idx[0] = group_index;
        idx[1] = group_index + (TILE_DIM * TILE_DIM / 4);       // +25
        idx[2] = group_index + (TILE_DIM * TILE_DIM / 2);       // +50
        idx[3] = group_index + (TILE_DIM * TILE_DIM * 3 / 4);   // +75
        [unroll]
        for (int k = 0; k < 4; ++k) {
            int2 gt = int2(int(idx[k] % TILE_DIM), int(idx[k] / TILE_DIM));
            store_color_depth(uint2(gt), group_top_left + gt, w, h);
        }
    }
    GroupMemoryBarrierWithGroupSync();
}

float3 load_color(uint2 group_thread_id)
{
    group_thread_id += BORDER;
    return tile_color[group_thread_id.x][group_thread_id.y];
}

float load_depth(uint2 group_thread_id)
{
    group_thread_id += BORDER;
    return tile_depth[group_thread_id.x][group_thread_id.y];
}

void depth_test_min(uint2 pos, inout float min_depth, inout uint2 min_pos)
{
    float depth = load_depth(pos);
    if (depth < min_depth) {
        min_depth = depth;
        min_pos = pos;
    }
}

// Velocity with the closest depth in the 3x3 neighbourhood. The velocity is
// fetched at (group_top_left + min_pos) exactly as the Spartan-derived native
// shader does (a one-texel border offset relative to the depth query); Load
// returns 0 out of bounds, matching native imageLoad.
float2 get_closest_pixel_velocity_3x3(uint2 group_pos, uint2 group_top_left)
{
    float min_depth = 1.0f;
    uint2 min_pos = group_pos;
    [unroll]
    for (int i = 0; i < 9; ++i) {
        depth_test_min(uint2(int2(group_pos) + kOffsets3x3[i]), min_depth, min_pos);
    }
    return velocity_buffer.Load(int3(int2(group_top_left) + int2(min_pos), 0));
}

// Explicit float 4-tap bilinear with clamp addressing, reproducing textureLod
// on history_buffer with a linear+clamp sampler. Interior UVs are texel-exact;
// hardware sub-texel fixed-point rounding is a recorded gap.
float3 sample_history_bilinear(float2 uv, float2 resolution)
{
    float2 s = uv * resolution - 0.5f;
    float2 fs = floor(s);
    float2 f = s - fs;
    int2 i0 = int2(fs);
    int2 i1 = i0 + 1;
    int2 maxc = int2(int(resolution.x) - 1, int(resolution.y) - 1);
    i0 = clamp(i0, int2(0, 0), maxc);
    i1 = clamp(i1, int2(0, 0), maxc);
    float3 c00 = history_buffer.Load(int3(i0.x, i0.y, 0)).rgb;
    float3 c10 = history_buffer.Load(int3(i1.x, i0.y, 0)).rgb;
    float3 c01 = history_buffer.Load(int3(i0.x, i1.y, 0)).rgb;
    float3 c11 = history_buffer.Load(int3(i1.x, i1.y, 0)).rgb;
    float3 top = lerp(c00, c10, f.x);
    float3 bot = lerp(c01, c11, f.x);
    return lerp(top, bot, f.y);
}

// Catmull-Rom 9-tap (TheRealMJP), textureLod replaced by sample_history_bilinear.
float3 sample_catmull_rom_9(float2 uv, float2 resolution)
{
    float2 sample_pos = uv * resolution;
    float2 texPos1 = floor(sample_pos - 0.5f) + 0.5f;
    float2 f = sample_pos - texPos1;

    float2 w0 = f * (-0.5f + f * (1.0f - 0.5f * f));
    float2 w1 = 1.0f + f * f * (-2.5f + 1.5f * f);
    float2 w2 = f * (0.5f + f * (2.0f - 1.5f * f));
    float2 w3 = f * f * (-0.5f + 0.5f * f);

    float2 w12 = w1 + w2;
    float2 offset12 = w2 / (w1 + w2);

    float2 texPos0 = texPos1 - 1.0f;
    float2 texPos3 = texPos1 + 2.0f;
    float2 texPos12 = texPos1 + offset12;

    texPos0 /= resolution;
    texPos3 /= resolution;
    texPos12 /= resolution;

    float3 result = float3(0.0f, 0.0f, 0.0f);
    result += sample_history_bilinear(float2(texPos0.x, texPos0.y), resolution) * w0.x * w0.y;
    result += sample_history_bilinear(float2(texPos12.x, texPos0.y), resolution) * w12.x * w0.y;
    result += sample_history_bilinear(float2(texPos3.x, texPos0.y), resolution) * w3.x * w0.y;

    result += sample_history_bilinear(float2(texPos0.x, texPos12.y), resolution) * w0.x * w12.y;
    result += sample_history_bilinear(float2(texPos12.x, texPos12.y), resolution) * w12.x * w12.y;
    result += sample_history_bilinear(float2(texPos3.x, texPos12.y), resolution) * w3.x * w12.y;

    result += sample_history_bilinear(float2(texPos0.x, texPos3.y), resolution) * w0.x * w3.y;
    result += sample_history_bilinear(float2(texPos12.x, texPos3.y), resolution) * w12.x * w3.y;
    result += sample_history_bilinear(float2(texPos3.x, texPos3.y), resolution) * w3.x * w3.y;

    return max(result, 0.0f);
}

// Playdead "Temporal Reprojection Anti-Aliasing" clip_aabb.
float3 clip_aabb(float3 aabb_min, float3 aabb_max, float3 p, float3 q)
{
    float3 r = q - p;
    float3 rmax = aabb_max - p;
    float3 rmin = aabb_min - p;

    if (r.x > rmax.x + FLT_MIN_TAA) { r *= (rmax.x / r.x); }
    if (r.y > rmax.y + FLT_MIN_TAA) { r *= (rmax.y / r.y); }
    if (r.z > rmax.z + FLT_MIN_TAA) { r *= (rmax.z / r.z); }

    if (r.x < rmin.x - FLT_MIN_TAA) { r *= (rmin.x / r.x); }
    if (r.y < rmin.y - FLT_MIN_TAA) { r *= (rmin.y / r.y); }
    if (r.z < rmin.z - FLT_MIN_TAA) { r *= (rmin.z / r.z); }

    return p + r;
}

float3 clip_history_3x3(uint2 group_pos, float3 color_history, float2 velocity_closest)
{
    float3 s1 = load_color(uint2(int2(group_pos) + kOffsets3x3[0]));
    float3 s2 = load_color(uint2(int2(group_pos) + kOffsets3x3[1]));
    float3 s3 = load_color(uint2(int2(group_pos) + kOffsets3x3[2]));
    float3 s4 = load_color(uint2(int2(group_pos) + kOffsets3x3[3]));
    float3 s5 = load_color(uint2(int2(group_pos) + kOffsets3x3[4]));
    float3 s6 = load_color(uint2(int2(group_pos) + kOffsets3x3[5]));
    float3 s7 = load_color(uint2(int2(group_pos) + kOffsets3x3[6]));
    float3 s8 = load_color(uint2(int2(group_pos) + kOffsets3x3[7]));
    float3 s9 = load_color(uint2(int2(group_pos) + kOffsets3x3[8]));

    float3 color_avg = (s1 + s2 + s3 + s4 + s5 + s6 + s7 + s8 + s9) * RPC_9;
    float3 color_avg2 = ((s1 * s1) + (s2 * s2) + (s3 * s3) + (s4 * s4) + (s5 * s5) +
                         (s6 * s6) + (s7 * s7) + (s8 * s8) + (s9 * s9)) * RPC_9;

    float box_size = lerp(0.0f, variance_dynamic,
                          smoothstep(0.02f, 0.0f, length(velocity_closest)));
    float3 dev = sqrt(abs(color_avg2 - (color_avg * color_avg))) * box_size;
    float3 color_min = color_avg - dev;
    float3 color_max = color_avg + dev;

    float3 color = clip_aabb(color_min, color_max, clamp(color_avg, color_min, color_max), color_history);
    color = clamp(color, FLT_MIN_TAA, FLT_MAX_TAA);
    return color;
}

float luminance_taa(float3 color) { return max(dot(color, lumCoeff), 0.0001f); }

float get_factor_disocclusion(float2 uv_reprojected, float2 velocity, float2 resolution)
{
    float2 velocity_previous = last_velocity_buffer.Load(int3(int2(uv_reprojected * resolution), 0));
    float2 velocity_texels = velocity * resolution;
    float2 prev_velocity_texels = velocity_previous * resolution;
    float disocclusion = length(prev_velocity_texels - velocity_texels) - disocclusion_threshold;
    return clamp(disocclusion * DISOCCLUSION_SCALE, 0.0f, 1.0f);
}

float3 temporal_antialiasing(uint2 pos_group_top_left, uint2 pos_group,
                             uint2 pos_screen, float2 uv, float2 resolution)
{
    float2 velocity = velocity_buffer.Load(int3(int2(pos_screen), 0));
    float2 uv_reprojected = uv + velocity;

    float3 color_input = load_color(pos_group);
    float3 color_history = sample_catmull_rom_9(uv_reprojected, resolution);

    float2 velocity_closest = get_closest_pixel_velocity_3x3(pos_group, pos_group_top_left);
    color_history = clip_history_3x3(pos_group, color_history, velocity_closest);

    float blend_factor = RPC_16;
    {
        float factor_screen = (any(uv_reprojected < float2(0.0f, 0.0f)) ||
                               any(uv_reprojected > float2(1.0f, 1.0f))) ? 1.0f : 0.0f;
        float factor_disocclusion = get_factor_disocclusion(uv_reprojected, velocity, resolution);
        blend_factor = clamp(blend_factor + factor_screen + factor_disocclusion, 0.0f, 1.0f);
    }

    float3 color_resolved = float3(0.0f, 0.0f, 0.0f);
    {
        color_history = reinhard(color_history);
        color_input = reinhard(color_input);

        float lum_color = luminance_taa(color_input);
        float lum_history = luminance_taa(color_history);
        float diff = abs(lum_color - lum_history) / max(lum_color, max(lum_history, 1.001f));
        diff = 1.0f - diff;
        diff = diff * diff;
        blend_factor = lerp(0.0f, blend_factor, diff);

        color_resolved = lerp(color_history, color_input, blend_factor);
        color_resolved = reinhard_inverse(color_resolved);
    }

    return color_resolved;
}

[numthreads(GROUP_SIZE, GROUP_SIZE, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID,
          uint3 group_thread_id : SV_GroupThreadID,
          uint3 group_id : SV_GroupID,
          uint group_index : SV_GroupIndex)
{
    uint width = source_width_u64.x;
    uint height = source_height_u64.x;
    float2 resolution = float2(float(width), float(height));

    populate_group_shared_memory(group_id.xy, group_index, width, height);

    // Out of bounds check (after populate, matching native ordering).
    if (dispatch_id.x >= width || dispatch_id.y >= height) {
        return;
    }

    uint2 pos_group = group_thread_id.xy;
    uint2 pos_group_top_left = group_id.xy * GROUP_SIZE - BORDER;
    uint2 pos_screen = dispatch_id.xy;
    float2 uv = (float2(dispatch_id.xy) + 0.5f) / resolution;

    float3 result = temporal_antialiasing(pos_group_top_left, pos_group, pos_screen, uv, resolution);
    output_buffer[dispatch_id.xy] = float4(result, 1.0f);
}
