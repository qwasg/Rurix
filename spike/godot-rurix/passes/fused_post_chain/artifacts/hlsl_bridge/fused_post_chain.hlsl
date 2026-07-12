// GRX-019 S2: math-equivalent HLSL fused post-chain kernel (luminance final
// WRITE_LUMINANCE level + tonemap LINEAR/sRGB subset in ONE full-resolution
// dispatch).
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact under the owner-approved texture artifact
// provenance policy (GRX-009 texture_artifact_provenance_policy.json, which
// applies to every texture compute pass): HLSL compiled by DXC + Rurix-owned
// RTS0 + DXV validation may serve as the runtime-mappable canonical package.
// It does NOT imply real_gpu_pass=true.
//
// Fusion members (both members recorded strict measured success in their own
// slices; referencing them makes no new claim here):
//   Segment A: the final WRITE_LUMINANCE luminance-reduction level, math
//     equivalent to ../../../luminance_reduction/artifacts/hlsl_bridge/
//     luminance_reduce_level.hlsl (-D RX_WRITE_LUMINANCE) evaluated at
//     destination texel (0,0): partial-tile-correct mean over the <= 8x8
//     lum_source, cur = clamp(avg, min_luminance, max_luminance), then
//     ema = prev + (cur - prev) * exposure_adjust. first_frame != 0 selects
//     cur (skips the EMA).
//   Segment B: the tonemap TONEMAPPER_LINEAR + linear_to_srgb subset, math
//     equivalent to ../../../tonemap/artifacts/hlsl_bridge/tonemap_apply.hlsl,
//     with the exposure input upgraded to the segment-A register-carried
//     luminance current per the native auto-exposure formula
//     (tonemap.glsl L866-868):
//       exposure *= 1.0 / (lum_current * luminance_multiplier
//                          / auto_exposure_scale);
//     This closes the recorded gap where patch 0012 forwards only scalar
//     exposure/white/luminance_multiplier and never the exposure_texture
//     handle (renderer_scene_render_rd.cpp:697), which made the standalone
//     Rurix tonemap pass correct only with auto exposure OFF.
//
// EXPLICIT GAPS (recorded in pass_manifest.json known_gaps): glow composite
// (all modes; gaussian_glow's own luminance consumption), non-LINEAR
// tonemappers (Reinhard/FILMIC/ACES/AgX), the rest of the auto-exposure
// texture chain (upper reduce pyramid levels stay native), the inherited
// member clamp-order divergence vs native WRITE_LUMINANCE (EMA inside the
// clamp), the first_frame clamp-bounded divergence vs the native p_set plain
// reduce (raw avg), rgba16f/r32f storage quantization, one-frame latency
// (the tonemap segment natively consumes the CURRENT frame's color), sRGB
// output not clamped to [0,1], the raster-vs-compute output seam, FXAA/BCS/
// color correction/debanding/multiview/HDR output, and lum_source > 8x8.
//
// Binding surface (matches artifacts/fused_post_chain_descriptor_layout.json):
//   t0 space0 : Texture2D<float4>   src_color      (full-res HDR color)
//   t1 space0 : Texture2D<float>    lum_source     (final-level luminance
//                                                   source, extent <= 8x8)
//   t2 space0 : Texture2D<float>    prev_luminance (1x1 previous EMA result;
//                                                   MUST differ from u1)
//   u0 space0 : RWTexture2D<float4> dst_color      (LDR output, full res)
//   u1 space0 : RWTexture2D<float>  dst_luminance  (1x1 current EMA output)
//   b0 space0 : 64-byte constants (root_parameter_index 0, 16 dwords) merging
//               the two member canonical layouts + fusion controls:
//                 dwords 0-1  : source_width         (i64: low, high)
//                 dwords 2-3  : source_height        (i64: low, high)
//                 dwords 4-5  : lum_source_width     (i64: low, high; <= 8)
//                 dwords 6-7  : lum_source_height    (i64: low, high; <= 8)
//                 dword  8    : max_luminance        (f32)
//                 dword  9    : min_luminance        (f32)
//                 dword  10   : exposure_adjust      (f32)
//                 dword  11   : exposure             (f32)
//                 dword  12   : white                (f32, unused for LINEAR)
//                 dword  13   : luminance_multiplier (f32)
//                 dword  14   : first_frame          (f32, 0.0/1.0)
//                 dword  15   : auto_exposure_scale  (f32)
//               The i64 dims are declared as uint2 (low, high dword) so the
//               DXIL stays plain cs_6_0 without the optional Int64 shader
//               capability; only the low dword is consumed (dims < 2^32) and
//               the high dword must be written as 0 by the runtime.
//
// Thread mapping: [numthreads(8,8,1)], one thread per destination LDR texel
// (dst_color extent == src_color extent; the tonemap segment defines the
// grid). Dispatch with (ceil(width/8), ceil(height/8), 1) thread groups.
// Segment A is recomputed redundantly by thread 0 of EVERY group from the
// <= 8x8 lum_source (at most 64 loads) and broadcast through groupshared
// memory; only dispatch group (0,0) writes the 1x1 dst_luminance. All groups
// read identical SRV data, so every group computes the identical value.

Texture2D<float4> src_color : register(t0, space0);
Texture2D<float> lum_source : register(t1, space0);
Texture2D<float> prev_luminance : register(t2, space0);
RWTexture2D<float4> dst_color : register(u0, space0);
RWTexture2D<float> dst_luminance : register(u1, space0);

cbuffer FusedPostChainConstants : register(b0, space0)
{
    uint2 source_width_u64;      // dwords 0-1:  i64 source_width      (low, high)
    uint2 source_height_u64;     // dwords 2-3:  i64 source_height     (low, high)
    uint2 lum_source_width_u64;  // dwords 4-5:  i64 lum_source_width  (low, high)
    uint2 lum_source_height_u64; // dwords 6-7:  i64 lum_source_height (low, high)
    float max_luminance;         // dword 8
    float min_luminance;         // dword 9
    float exposure_adjust;       // dword 10
    float exposure;              // dword 11
    float white;                 // dword 12 (unused for TONEMAPPER_LINEAR)
    float luminance_multiplier;  // dword 13
    float first_frame;           // dword 14 (0.0 / 1.0, mirrors native p_set)
    float auto_exposure_scale;   // dword 15
};

// Per-group broadcast slot for the segment-A luminance current value.
groupshared float rx_lum_current_broadcast;

// tonemap.glsl linear_to_srgb, componentwise (identical to the member kernel
// tonemap_apply.hlsl):
//   mix((1+a)*pow(c, 1/2.4) - a, 12.92*c, c < 0.0031308) with a = 0.055.
float3 linear_to_srgb(float3 color)
{
    const float a = 0.055f;
    float3 lo = 12.92f * color;
    float3 hi = (1.0f + a) * pow(max(color, 0.0f), 1.0f / 2.4f) - a;
    return float3(
        color.x < 0.0031308f ? lo.x : hi.x,
        color.y < 0.0031308f ? lo.y : hi.y,
        color.z < 0.0031308f ? lo.z : hi.z);
}

// Segment A: the final WRITE_LUMINANCE level evaluated at destination texel
// (0,0). Accumulation order (sy outer, sx inner, bounds-guarded) matches the
// member kernel luminance_reduce_level.hlsl exactly, so the result is
// binary32-identical to the member for every lum_source extent <= 8x8.
float compute_luminance_current()
{
    // Low dwords only; the canonical i64 dims must have zero high dwords.
    uint lum_width = lum_source_width_u64.x;
    uint lum_height = lum_source_height_u64.x;

    float accum = 0.0f;
    float count = 0.0f;
    for (uint dy = 0u; dy < 8u; dy = dy + 1u) {
        uint sy = dy;
        if (sy < lum_height) {
            for (uint dx = 0u; dx < 8u; dx = dx + 1u) {
                uint sx = dx;
                if (sx < lum_width) {
                    float src_lum = lum_source.Load(int3(int(sx), int(sy), 0));
                    accum = accum + src_lum;
                    count = count + 1.0f;
                }
            }
        }
    }

    // Partial-tile-correct mean (count = valid texel count, same as the
    // member kernel).
    float avg = (count > 0.0f) ? (accum / count) : 0.0f;

    // Member-kernel order: clamp then EMA (the native WRITE_LUMINANCE order,
    // EMA inside the clamp, is a recorded inherited gap).
    float cur = clamp(avg, min_luminance, max_luminance);
    float prev = prev_luminance.Load(int3(0, 0, 0));
    float ema = prev + (cur - prev) * exposure_adjust;

    // first_frame mirrors the native p_set: no previous luminance exists, so
    // the EMA is skipped and the clamped current value is used directly (the
    // native p_set plain reduce writes the RAW avg; the clamp-bounded
    // divergence is a recorded gap).
    return (first_frame != 0.0f) ? cur : ema;
}

[numthreads(8, 8, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID,
          uint3 group_id : SV_GroupID,
          uint group_index : SV_GroupIndex)
{
    // ── Segment A: luminance final level (per-group register prologue) ──
    if (group_index == 0u) {
        float lum = compute_luminance_current();
        rx_lum_current_broadcast = lum;
        if (group_id.x == 0u && group_id.y == 0u) {
            // Written exactly once per dispatch; every group computes the
            // identical value from identical read-only inputs.
            dst_luminance[uint2(0u, 0u)] = lum;
        }
    }
    GroupMemoryBarrierWithGroupSync();
    float lum_current = rx_lum_current_broadcast;

    // ── Segment B: tonemap (TONEMAPPER_LINEAR + linear_to_srgb subset) ──
    // Low dwords only; the canonical i64 dims must have zero high dwords.
    uint width = source_width_u64.x;
    uint height = source_height_u64.x;

    uint x = dispatch_id.x;
    uint y = dispatch_id.y;
    if (x < width && y < height) {
        float4 color = src_color.Load(int3(int(x), int(y), 0));

        // tonemap.glsl L860.
        color.rgb *= luminance_multiplier;

        // tonemap.glsl L864-870 with the FLAG_USE_AUTO_EXPOSURE leg
        // (L866-868) always taken: the fused pass targets the
        // auto-exposure-ON scenario (natively luminance_reduction only runs
        // with auto exposure on), and lum_current replaces the
        // source_auto_exposure texel fetch. Operation order mirrors the GLSL:
        //   exposure *= 1.0 / (lum * luminance_multiplier
        //                      / auto_exposure_scale);
        // lum_current == 0 divides to inf exactly like the native GLSL
        // (recorded gap; the deferred gate keeps min_luminance > 0 fixtures).
        float auto_exposure_denominator =
            lum_current * luminance_multiplier / auto_exposure_scale;
        float exposure_effective =
            exposure * (1.0f / auto_exposure_denominator);
        color.rgb *= exposure_effective;

        // apply_tonemapping, TONEMAPPER_LINEAR: identity (tonemap.glsl
        // L247-249; `white` intentionally unused for this mode).

        // FLAG_CONVERT_TO_SRGB leg (tonemap.glsl L942-943).
        color.rgb = linear_to_srgb(color.rgb);

        dst_color[uint2(x, y)] = color;
    }
}
