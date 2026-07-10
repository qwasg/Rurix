// GRX-011 segment A: math-equivalent HLSL SSAO blur kernel (MODE_SMART
// subset, single pass).
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact under the owner-approved texture artifact
// provenance policy (GRX-009 texture_artifact_provenance_policy.json, which
// applies to every texture compute pass): HLSL compiled by DXC + Rurix-owned
// RTS0 + DXV validation may serve as the runtime-mappable canonical package.
// It does NOT imply real_gpu_pass=true.
//
// Math target: Godot's
// servers/rendering/renderer_rd/shaders/effects/ssao_blur.glsl, MODE_SMART
// variant (SSAO_BLUR_PASS_SMART), for the SUPPORTED SUBSET ONLY:
//   * one blur pass over one deinterleaved half-res slice
//   * edge-aware 3x3 cross (center + L/R/T/B neighbors)
//   * unpack_edges (ssao_blur.glsl L39-48): packed byte -> 4x 2-bit LRTB
//     edge values / 3.0, then clamp(edges + edge_sharpness, 0, 1)
//   * sample_blurred (L95-122): sum_weight starts at 0.5 with the center
//     value, each neighbor adds weight*value / weight, result = sum/sum_weight
//   * packed edges passthrough in the second output channel; z/w written 0
//     (main() L153: imageStore(dest_image, ssC, vec4(sampled, 0.0, 0.0)))
//
// Interior-texel equivalence note: Godot's MODE_SMART uses two textureGather
// calls at half-pixel offsets around the pixel center, which for interior
// texels select exactly the center/L/R/T/B texels this kernel reads via
// Load (gather component mapping: UL.y=center, UL.x=left, UL.z=top,
// BR.z=right, BR.x=bottom). This kernel clamps neighbor coordinates at the
// image border; Godot binds a mirror sampler (VERY_LOW quality uses a
// clamp/default sampler), so border texels can differ — recorded gap.
//
// EXPLICIT GAPS (recorded in pass_manifest.json known_gaps): MODE_WIDE and
// MODE_NON_SMART variants, multi-pass ping-pong chains (ssao_blur_passes
// default 2, wide passes for pass < blur_passes-2), the 4-slice
// deinterleaved loop, SSIL blur (rgba16 value + separate r8 edges image),
// mirror-sampler border addressing, rg8 unorm storage (this kernel reads
// float values; the deinterleaved slices store rg8), and the
// gather-vs-load addressing seam outside interior texels.
//
// Binding surface (matches artifacts/ssao_blur_descriptor_layout.json):
//   t0 space0 : Texture2D<float4>   src_ssao (x = ssao value, y = packed
//               edges; z/w unused)
//   u0 space0 : RWTexture2D<float4> dst_ssao (x = blurred value, y = packed
//               edges passthrough, z/w = 0)
//   b0 space0 : 28-byte constants matching the canonical Rurix
//               root-constant layout (root_parameter_index 0, 7 dwords):
//                 dwords 0-1 : source_width              (i64: low, high)
//                 dwords 2-3 : source_height             (i64: low, high)
//                 dword  4   : edge_sharpness            (f32)
//                 dword  5   : half_screen_pixel_size_x  (f32, unused)
//                 dword  6   : half_screen_pixel_size_y  (f32, unused)
//               The i64 dims are declared as uint2 (low, high dword) so the
//               DXIL stays plain cs_6_0 without the optional Int64 shader
//               capability; only the low dword is consumed (dims < 2^32)
//               and the high dword must be written as 0 by the runtime.
//               half_screen_pixel_size_x/y mirror Godot's
//               SSAOBlurPushConstant shape (edge_sharpness, pad,
//               half_screen_pixel_size[2]); this Load-based kernel does not
//               consume them (they exist for uv-space gather addressing).
//
// Thread mapping: [numthreads(8,8,1)], one thread per destination texel
// (dst extent == src extent; the blur is a 1:1 ping-pong pass at the
// half-res deinterleaved slice size). Dispatch with
// (ceil(width/8), ceil(height/8), 1) thread groups.

Texture2D<float4> src_ssao : register(t0, space0);
RWTexture2D<float4> dst_ssao : register(u0, space0);

cbuffer SsaoBlurConstants : register(b0, space0)
{
    uint2 source_width_u64;         // dwords 0-1: i64 source_width  (low, high)
    uint2 source_height_u64;        // dwords 2-3: i64 source_height (low, high)
    float edge_sharpness;           // dword 4
    float half_screen_pixel_size_x; // dword 5 (shape parity, unused)
    float half_screen_pixel_size_y; // dword 6 (shape parity, unused)
};

// ssao_blur.glsl unpack_edges (L39-48): 2-bit LRTB fields of the packed
// byte, each / 3.0, then clamp(edges + edge_sharpness, 0, 1).
float4 unpack_edges(float packed_val_f)
{
    uint packed_val = (uint)(packed_val_f * 255.5f);
    float4 edgesLRTB;
    edgesLRTB.x = float((packed_val >> 6) & 0x03u) / 3.0f;
    edgesLRTB.y = float((packed_val >> 4) & 0x03u) / 3.0f;
    edgesLRTB.z = float((packed_val >> 2) & 0x03u) / 3.0f;
    edgesLRTB.w = float((packed_val >> 0) & 0x03u) / 3.0f;
    return clamp(edgesLRTB + edge_sharpness, 0.0f, 1.0f);
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
        int xi = int(x);
        int yi = int(y);
        // Border addressing: clamp to the image extent (Godot's smart blur
        // binds a mirror sampler; interior texels are identical, border
        // texels are a recorded gap).
        int xl = max(xi - 1, 0);
        int xr = min(xi + 1, int(width) - 1);
        int yt = max(yi - 1, 0);
        int yb = min(yi + 1, int(height) - 1);

        float2 vC = src_ssao.Load(int3(xi, yi, 0)).xy;
        float ssao_valueL = src_ssao.Load(int3(xl, yi, 0)).x;
        float ssao_valueR = src_ssao.Load(int3(xr, yi, 0)).x;
        float ssao_valueT = src_ssao.Load(int3(xi, yt, 0)).x;
        float ssao_valueB = src_ssao.Load(int3(xi, yb, 0)).x;

        float packed_edges = vC.y;
        float4 edgesLRTB = unpack_edges(packed_edges);

        // sample_blurred (ssao_blur.glsl L95-122): center weight 0.5, then
        // add_sample for L, R, T, B in that order.
        float sum_weight = 0.5f;
        float sum = vC.x * sum_weight;
        sum += ssao_valueL * edgesLRTB.x;
        sum_weight += edgesLRTB.x;
        sum += ssao_valueR * edgesLRTB.y;
        sum_weight += edgesLRTB.y;
        sum += ssao_valueT * edgesLRTB.z;
        sum_weight += edgesLRTB.z;
        sum += ssao_valueB * edgesLRTB.w;
        sum_weight += edgesLRTB.w;

        float ssao_avg = sum / sum_weight;

        // main() L153: vec4(sampled, 0.0, 0.0) — blurred value, packed
        // edges passthrough, zeros.
        dst_ssao[uint2(x, y)] = float4(ssao_avg, packed_edges, 0.0f, 0.0f);
    }
}
