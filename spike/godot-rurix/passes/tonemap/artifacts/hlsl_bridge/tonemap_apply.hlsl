// GRX-010 stage A: math-equivalent HLSL tonemap kernel (LDR output).
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact under the owner-approved texture artifact
// provenance policy (GRX-009
// texture_artifact_provenance_policy.json, which applies to every texture
// compute pass): HLSL compiled by DXC + Rurix-owned RTS0 + DXV validation
// may serve as the runtime-mappable canonical package. The Rurix toolchain
// still cannot lower texture kernels to a runtime-mappable DXIL container
// (patched llc lacks `llvm.dx.resource.load.texture.2d`; float4 texture
// element types are additionally unsupported by the Rurix Texture2D<F>
// lang item). It does NOT imply real_gpu_pass=true.
//
// Math target: the scalar core of Godot's
// servers/rendering/renderer_rd/shaders/effects/tonemap.glsl fragment path
// for the SUPPORTED SUBSET ONLY:
//   * tonemapper = TONEMAPPER_LINEAR (0, Godot default ENV_TONE_MAPPER_LINEAR)
//   * FLAG_CONVERT_TO_SRGB set (SDR output, using_hdr=false default)
//   * no auto exposure / glow / FXAA / BCS / color correction / debanding
//
// Per-texel math (tonemap.glsl main(), reduced to that subset):
//   color.rgb *= luminance_multiplier;             // L860
//   color.rgb *= exposure;                         // L864/L870
//   color.rgb = apply_tonemapping(color.rgb);      // L893: LINEAR = identity
//   color.rgb = linear_to_srgb(color.rgb);         // L942-943
//   alpha passes through unchanged.
//
// linear_to_srgb (tonemap.glsl L230-233):
//   a = 0.055; c < 0.0031308 ? 12.92*c : (1+a)*pow(c, 1/2.4) - a
//
// EXPLICIT GAPS (recorded in pass_manifest.json known_gaps): other
// tonemappers (Reinhard/Filmic/ACES/AgX), auto exposure, glow, FXAA,
// BCS, color correction, debanding, multiview, HDR output
// (convert_to_srgb=false), bicubic glow upscale, and the raster-vs-compute
// output shape difference (Godot writes the LDR result to a framebuffer via
// a fullscreen fragment shader; this kernel writes a full-res UAV).
// `white` is carried in b0 for push-constant shape parity with Godot's
// TonemapPushConstant but is unused by the linear tonemapper.
//
// Binding surface (matches artifacts/tonemap_descriptor_layout.json):
//   t0 space0 : Texture2D<float4>   src_color  (HDR linear scene color)
//   u0 space0 : RWTexture2D<float4> dst_color  (LDR output, full res)
//   b0 space0 : 28-byte constants matching the canonical Rurix
//               root-constant layout (root_parameter_index 0, 7 dwords):
//                 dwords 0-1 : source_width         (i64: low, high)
//                 dwords 2-3 : source_height        (i64: low, high)
//                 dword  4   : exposure             (f32)
//                 dword  5   : white                (f32, unused for LINEAR)
//                 dword  6   : luminance_multiplier (f32)
//               The i64 dims are declared as uint2 (low, high dword) so the
//               DXIL stays plain cs_6_0 without the optional Int64 shader
//               capability; only the low dword is consumed (dims < 2^32)
//               and the high dword must be written as 0 by the runtime.
//
// Thread mapping: [numthreads(8,8,1)], one thread per destination texel
// (dst extent == src extent; tonemap is a 1:1 full-resolution pass).
// Dispatch with (ceil(width/8), ceil(height/8), 1) thread groups.

Texture2D<float4> src_color : register(t0, space0);
RWTexture2D<float4> dst_color : register(u0, space0);

cbuffer TonemapConstants : register(b0, space0)
{
    uint2 source_width_u64;      // dwords 0-1: i64 source_width  (low, high)
    uint2 source_height_u64;     // dwords 2-3: i64 source_height (low, high)
    float exposure;              // dword 4
    float white;                 // dword 5 (unused for TONEMAPPER_LINEAR)
    float luminance_multiplier;  // dword 6
};

// tonemap.glsl linear_to_srgb, componentwise:
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

[numthreads(8, 8, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID)
{
    // Low dword only; the canonical i64 dims must have a zero high dword.
    uint width = source_width_u64.x;
    uint height = source_height_u64.x;

    uint x = dispatch_id.x;
    uint y = dispatch_id.y;
    if (x < width && y < height) {
        float4 color = src_color.Load(int3(int(x), int(y), 0));

        // tonemap.glsl L860 + L864/L870 (no auto exposure in this subset).
        color.rgb *= luminance_multiplier;
        color.rgb *= exposure;

        // apply_tonemapping, TONEMAPPER_LINEAR: identity (tonemap.glsl
        // L247-249; `white` intentionally unused for this mode).

        // FLAG_CONVERT_TO_SRGB leg (tonemap.glsl L942-943).
        color.rgb = linear_to_srgb(color.rgb);

        dst_color[uint2(x, y)] = color;
    }
}
