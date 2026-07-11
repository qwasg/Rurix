// GRX-009 stage A2: math-equivalent HLSL texture luminance reduction kernel.
//
// Provenance: hlsl_bridge_workaround (NOT rurix_owned). This kernel is a
// DXC-compiled bridge artifact that mirrors, step for step, the Rurix
// texture kernel `src/lib_texture.rx` (`luminance_reduce_level_texture`),
// which the Rurix toolchain cannot yet lower to a runtime-mappable DXIL
// container (the patched llc lacks `llvm.dx.resource.load.texture.2d`
// support; see texture_intrinsic_toolchain_blocker.json). It exists so
// math parity and runtime mapping can be proven ahead of the Rurix-owned
// compile path. It does NOT replace the canonical artifacts and does NOT
// imply real_gpu_pass=true.
//
// Binding surface (matches artifacts/hlsl_bridge/descriptor_layout.json):
//   t0 space0 : Texture2D<float>    src_luminance
//   u0 space0 : RWTexture2D<float>  dst_luminance
//   b0 space0 : 28-byte constants matching the canonical Rurix
//               root-constant layout (root_parameter_index 0, 7 dwords):
//                 dwords 0-1 : source_width   (i64: low, high)
//                 dwords 2-3 : source_height  (i64: low, high)
//                 dword  4   : max_luminance   (f32)
//                 dword  5   : min_luminance   (f32)
//                 dword  6   : exposure_adjust (f32)
//               The i64 dims are declared as uint2 (low, high dword) so the
//               DXIL stays plain cs_6_0 without the optional Int64 shader
//               capability; only the low dword is consumed (dims < 2^32)
//               and the high dword must be written as 0 by the runtime.
//   t1 space0 (RX_WRITE_LUMINANCE only) : Texture2D<float> prev_luminance
//
// Variants (compile-time, mirroring Godot's WRITE_LUMINANCE shader version
// in luminance_reduce.glsl; a runtime cbuffer flag would grow the constants
// to 32 bytes and break the 28-byte parity with the canonical Rurix
// root-constant layout, so the final-level path is a define instead):
//   (no defines)
//       Level-N reduction: write the partial-tile-correct 8x8 tile mean.
//       No clamp and no exposure_adjust here — per Godot, those belong
//       only to the final WRITE_LUMINANCE level. Math target:
//       src/lib_texture.rx.
//   -D RX_WRITE_LUMINANCE=1
//       Final level: cur = clamp(avg, min_luminance, max_luminance), then
//       EMA against the previous frame's luminance (t1):
//           out = prev + (cur - prev) * exposure_adjust
//       With prev == 0 this degenerates to cur * exposure_adjust, which is
//       exactly the src/lib.rx final math (clamp * exposure_adjust).
//
// Thread mapping: lib_texture.rx is 1D (dst_index = t.global_id();
// x = dst_index % dst_width; y = dst_index / dst_width). This kernel is
// [numthreads(8,8,1)] with one thread per destination texel
// (x, y) = dispatch_id.xy — exactly the decomposition of the same linear
// dst_index — so the per-texel math is identical. Dispatch with
// (ceil(dst_width/8), ceil(dst_height/8), 1) thread groups.
//
// Source addressing: lib_texture.rx computes src_index = sy * source_width
// + sx and reads src_luminance[src_index]; that linear index decomposes
// back to texel (sx, sy), so Load(int3(sx, sy, 0)) below is the same read.

Texture2D<float> src_luminance : register(t0, space0);
#if defined(RX_WRITE_LUMINANCE)
Texture2D<float> prev_luminance : register(t1, space0);
#endif
RWTexture2D<float> dst_luminance : register(u0, space0);

cbuffer LuminanceReduceConstants : register(b0, space0)
{
    uint2 source_width_u64;  // dwords 0-1: i64 source_width  (low, high)
    uint2 source_height_u64; // dwords 2-3: i64 source_height (low, high)
    float max_luminance;     // dword 4
    float min_luminance;     // dword 5
    float exposure_adjust;   // dword 6
};

[numthreads(8, 8, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID)
{
    // Low dword only; the canonical i64 dims must have a zero high dword.
    uint source_width = source_width_u64.x;
    uint source_height = source_height_u64.x;

    // Ceil-div 8x8 destination extent, mirroring lib_texture.rx exactly.
    uint dst_width = (source_width > 1u) ? ((source_width + 7u) / 8u) : 1u;
    uint dst_height = (source_height > 1u) ? ((source_height + 7u) / 8u) : 1u;

    uint x = dispatch_id.x;
    uint y = dispatch_id.y;
    // Equivalent to lib_texture.rx's `dst_index < dst_len` guard with
    // dst_index = y * dst_width + x.
    if (x < dst_width && y < dst_height) {
        uint src_x = x * 8u;
        uint src_y = y * 8u;

        float accum = 0.0f;
        float count = 0.0f;
        for (uint dy = 0u; dy < 8u; dy = dy + 1u) {
            uint sy = src_y + dy;
            if (sy < source_height) {
                for (uint dx = 0u; dx < 8u; dx = dx + 1u) {
                    uint sx = src_x + dx;
                    if (sx < source_width) {
                        // Single-channel R32F luminance source: the loaded
                        // value IS the luminance (same rationale as
                        // lib_texture.rx).
                        float src_lum = src_luminance.Load(int3(int(sx), int(sy), 0));
                        accum = accum + src_lum;
                        count = count + 1.0f;
                    }
                }
            }
        }

        // Per-tile arithmetic mean with a partial-tile-correct divisor
        // (count = valid pixel count, same as lib_texture.rx).
        float avg = (count > 0.0f) ? (accum / count) : 0.0f;

#if defined(RX_WRITE_LUMINANCE)
        // Final WRITE_LUMINANCE level (dst is 1x1): clamp, then EMA against
        // the previous frame's 1x1 luminance. prev == 0 degenerates to the
        // src/lib.rx final math (clamp * exposure_adjust).
        float cur = clamp(avg, min_luminance, max_luminance);
        float prev = prev_luminance.Load(int3(0, 0, 0));
        dst_luminance[uint2(x, y)] = prev + (cur - prev) * exposure_adjust;
#else
        // Level-N: mean only; clamp/exposure belong to the final level.
        dst_luminance[uint2(x, y)] = avg;
#endif
    }
}
