# GRX-011 SSAO Blur Resource Mapping

## Scope

This file records the GRX-011 ssao_blur resource mapping. It maps the real Godot SSAO edge-aware blur resources and parameters into the Rurix bridge design while keeping the runtime fallback-first. This is not a real GPU runtime pass, does not skip the native Godot SSAO blur, and does not provide visual, telemetry, or performance evidence.

## Godot Native Flow

- Entry point: `RendererRD::SSEffects::generate_ssao(...)` in `external/godot-master/servers/rendering/renderer_rd/effects/ss_effects.cpp` (`L1130`); the "Edge-Aware Blur" block is `L1320-1378`. This is a **compute** pass chain (Intel ASSAO-derived `ssao_blur.glsl`, `local_size 8x8x1`).
- Upstream call site: `render_forward_clustered.cpp` `_process_ssao` (`L1402`) → `ss_effects->generate_ssao(...)` (`L1424`).
- Pass shape: `blur_passes = ssao_quality > VERY_LOW ? ssao_blur_passes : 1` (`L1326`; `ssao_blur_passes` defaults to 2, `ss_effects.h:168`). Each pass loops 4 deinterleaved slices and ping-pongs between `ao_deinterleaved_slices[i]` and `ao_pong_slices[i]` (`L1340-1373`), dispatching `compute_list_dispatch_threads(buffer_width, buffer_height, 1)` per slice (`L1372`).
- Variant selection (`L1330-1338`): `SSAO_BLUR_PASS_WIDE` for `pass < blur_passes - 2`, otherwise `SSAO_BLUR_PASS_SMART`; `SSAO_BLUR_PASS` (non-smart) only at VERY_LOW quality. **This slice's kernel covers ONE SMART blur pass on ONE slice.**
- Parameters: `SSEffects::SSAOBlurPushConstant` (`ss_effects.h:383-387`, 16 bytes: `edge_sharpness`, `pad`, `half_screen_pixel_size[2]`), assembled at `L1322-1324`.

## Godot Resources

| Godot resource | Role | Native binding shape | GRX-011 Rurix mapping |
| --- | --- | --- | --- |
| `ao_deinterleaved_slices[i]` / `ao_pong_slices[i]` (ping-pong source; `RB_SCOPE_SSAO` `RB_DEINTERLEAVED`/`RB_DEINTERLEAVED_PONG`, `R8G8_UNORM`, `ss_effects.cpp:1123-1124`; x = ssao value, y = packed edges) | blur input | `sampler2D source_ssao` at set 0 binding 0 of `ssao_blur.glsl` (mirror sampler; default sampler at VERY_LOW) | `src_ssao` SRV `t0 space0`, `binding_kind = texture2d` |
| opposite ping-pong slice | blur output | `image2D dest_image` (rg8) at set 1 binding 0 | `dst_ssao` UAV `u0 space0`, `binding_kind = rwtexture2d` (1:1, dst extent == src extent; rg8 unorm quantization is a recorded gap) |
| SSIL `deinterleaved_slices` (rgba16) + `edges_slices` (r8) | SSIL blur variant | `ssil_blur.glsl` sets 0-2 | NOT mapped in this slice (known gap; `RXGD_PASS_SSIL_BLUR` not wired) |

Slice extent: `buffer_width/height = (full_screen + 1) / 2` (half_size: `(full_screen + 3) / 4`), `ss_effects.cpp:1104-1114`.

## Parameters

| Rurix root constant | Godot source | Type in current artifact | Notes |
| --- | --- | --- | --- |
| `source_width` | deinterleaved slice width (`p_ssao_buffers.buffer_width`) | `i64` | Carried as uint2 (low, high dword) in the HLSL cbuffer; high dword must be 0. Requires the 64-bit integer capability gate per the GRX-009 canonical template. |
| `source_height` | deinterleaved slice height (`p_ssao_buffers.buffer_height`) | `i64` | Same packing as `source_width`. |
| `edge_sharpness` | `1.0 - p_settings.sharpness` (`ss_effects.cpp:1322`) | `f32` | Added to unpacked LRTB edge values before clamp (`ssao_blur.glsl` L47). |
| `half_screen_pixel_size_x` | `1.0 / p_ssao_buffers.buffer_width` (`L1323`) | `f32` | Carried for `SSAOBlurPushConstant` shape parity; the Load-based kernel does not consume it (native uses it for uv-space gather addressing). |
| `half_screen_pixel_size_y` | `1.0 / p_ssao_buffers.buffer_height` (`L1324`) | `f32` | Same as `half_screen_pixel_size_x`. |

Root constants occupy 7 DWORDs (28 bytes) at root_parameter_index 0: `source_width` at DWORD 0..1, `source_height` at DWORD 2..3, `edge_sharpness` at DWORD 4, `half_screen_pixel_size_x` at DWORD 5, `half_screen_pixel_size_y` at DWORD 6 — the same `[i64, i64, f32, f32, f32]` packing shape as the GRX-009/GRX-010 canonical layouts.

## Descriptor Layout

- Root constants / root-cbuffer mapping: `b0 space0` for `source_width`, `source_height`, `edge_sharpness`, `half_screen_pixel_size_x`, `half_screen_pixel_size_y`.
- SRV: `src_ssao = t0 space0` (`binding_kind = texture2d`).
- UAV: `dst_ssao = u0 space0` (`binding_kind = rwtexture2d`).
- Required resource count for the bridge gate: 2 resources, source then destination.
- Required push constant size for the bridge gate: 28 bytes.
- Required dst shape: `dst_ssao` extent == `src_ssao` extent (1:1 ping-pong pass at the deinterleaved slice size; same shape rule as GRX-010's full-res tonemap, contrast with GRX-009's `max(source / 8, 1)` reduce shape).
- Required target device gate: 64-bit integer shader capability must be confirmed on the D3D12 device before any runtime attempt may proceed (b0 carries i64 dims; template parity with GRX-009/GRX-010).

## Supported Math Subset

The tracked kernel (`artifacts/hlsl_bridge/ssao_blur_smart.hlsl`) implements ONLY one SMART blur pass on one slice:

1. `unpack_edges(center.y)` (ssao_blur.glsl L39-48): packed byte → 4×2-bit LRTB / 3.0, `clamp(edges + edge_sharpness, 0, 1)`
2. edge-aware cross average (`sample_blurred`, L95-122): `sum_weight = 0.5` with the center value, `add_sample(L/R/T/B value, edge weight)` in L, R, T, B order, result `sum / sum_weight`
3. packed edges passthrough in the second channel; z/w written 0 (`main()` L153)
4. border texels use clamp addressing (interior texels are texel-exact vs Godot's half-pixel gather addressing)

Everything else in the native blur chain is a recorded gap (see `pass_manifest.json` `known_gaps`).

## Fallback Rules

- The pass remains disabled by default and runtime remains `fallback_only`.
- Missing source or destination resource returns fallback.
- Descriptor layout mismatch returns fallback.
- ABI mismatch returns fallback through existing ABI validation paths.
- Missing 64-bit integer shader capability returns fallback.
- Missing `RXGD_CAP_SSAO_BLUR_REAL_PASS` opt-in returns fallback (`manual_disabled`).
- Buffer (non-texture) resources fail the per-slot kernel-binding-kind conformance check.
- The shipping (feature-off) bridge fails closed with `real_dispatch_path_not_linked` even when every software gate passes.
- The native Godot SSAO blur loop remains the active path whenever the bridge does not return OK.

## Explicit Non-Goals (this slice)

- No real runtime SSAO blur GPU pass is enabled by default.
- No Godot runtime native-handle resource binding is wired (0005/0007-level work, later slice; includes the 4-slice/ping-pong scheduling design).
- No SSIL blur wiring (`RXGD_PASS_SSIL_BLUR` keeps its historical placeholder path).
- No in-engine visual diff evidence is produced (4g-level, later slice; includes temporal/noise stability).
- No measured fallback telemetry is produced (4g-level, later slice).
- No gated real-pass enablement measured success (4h-level, later slice).
- No performance number or acceleration claim is made.
