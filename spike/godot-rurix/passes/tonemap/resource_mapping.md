# GRX-010 Tonemap Resource Mapping

## Scope

This file records the GRX-010 tonemap resource mapping. It maps the real Godot tonemap resources and parameters into the Rurix bridge design while keeping the runtime fallback-first. This is not a real GPU runtime pass, does not skip the native Godot tonemapper, and does not provide visual, telemetry, or performance evidence.

## Godot Native Flow

- Entry point: `RendererRD::ToneMapper::tonemapper(RID p_source_color, RID p_dst_framebuffer, const TonemapSettings &p_settings)` in `external/godot-master/servers/rendering/renderer_rd/effects/tone_mapper.cpp` (`L117`). This is a **raster fullscreen fragment pass** (`tonemap.glsl`), not a compute dispatch.
- Call site: `external/godot-master/servers/rendering/renderer_rd/renderer_scene_render_rd.cpp` `_render_buffers_post_process_and_tonemap` (`L459`), "Tonemap" block `L689-832`. The `can_use_storage` leg calls `tone_mapper->tonemapper(color_texture, dest_fb, tonemap)` at `L826`.
- Source: `color_texture = use_upscaled_texture ? rb->get_upscaled_texture() : rb->get_internal_texture()` (`L492`) — the HDR linear scene color.
- Destination: `dest_fb` — the render target framebuffer, or an intermediate `Tonemapper/destination` texture framebuffer when a spatial upscaler / SMAA runs afterwards (`L786-807`).
- Parameters: `ToneMapper::TonemapSettings` assembled at `L693-823` (tonemap_mode, exposure, white, max_value, luminance_multiplier, auto exposure, glow, BCS, color correction, FXAA, debanding, convert_to_srgb).

## Godot Resources

| Godot resource | Role | Native binding shape | GRX-010 Rurix mapping |
| --- | --- | --- | --- |
| `color_texture` (`rb->get_internal_texture()` / upscaled) | HDR linear scene color input | sampled texture `source_color` at set 0 binding 0 of `tonemap.glsl` | `src_color` SRV `t0 space0`, `binding_kind = texture2d` |
| `dest_fb` color attachment | LDR output | raster framebuffer color attachment | `dst_color` UAV `u0 space0`, `binding_kind = rwtexture2d` (full-res, dst extent == src extent; raster-vs-compute output seam is a recorded gap) |
| `source_auto_exposure` (1x1 luminance, GRX-009 output) | auto exposure divisor | sampled texture at set 1 binding 0 | NOT mapped in this slice (known gap; auto exposure unsupported) |
| glow mips / glow map / color correction LUT | glow / grading | sampled textures at sets 2-3 | NOT mapped in this slice (known gaps) |

## Parameters

| Rurix root constant | Godot source | Type in current artifact | Notes |
| --- | --- | --- | --- |
| `source_width` | `color_texture` width (`tonemap.texture_size.x`) | `i64` | Carried as uint2 (low, high dword) in the HLSL cbuffer; high dword must be 0. Requires the 64-bit integer capability gate per the GRX-009 canonical template. |
| `source_height` | `color_texture` height (`tonemap.texture_size.y`) | `i64` | Same packing as `source_width`. |
| `exposure` | `TonemapSettings::exposure` (`environment_get_exposure`) | `f32` | Applied before tonemapping (`tonemap.glsl` L864/L870). |
| `white` | `TonemapSettings::white` (`environment_get_white`) | `f32` | Unused by `TONEMAPPER_LINEAR`; kept for push-constant shape parity with Godot's `TonemapPushConstant`. |
| `luminance_multiplier` | `rb->get_luminance_multiplier()` | `f32` | Applied first (`tonemap.glsl` L860). |

Root constants occupy 7 DWORDs (28 bytes) at root_parameter_index 0: `source_width` at DWORD 0..1, `source_height` at DWORD 2..3, `exposure` at DWORD 4, `white` at DWORD 5, `luminance_multiplier` at DWORD 6 — the same `[i64, i64, f32, f32, f32]` packing shape as the GRX-009 canonical layout.

## Descriptor Layout

- Root constants / root-cbuffer mapping: `b0 space0` for `source_width`, `source_height`, `exposure`, `white`, `luminance_multiplier`.
- SRV: `src_color = t0 space0` (`binding_kind = texture2d`).
- UAV: `dst_color = u0 space0` (`binding_kind = rwtexture2d`).
- Required resource count for the bridge gate: 2 resources, source then destination.
- Required push constant size for the bridge gate: 28 bytes.
- Required dst shape: `dst_color` extent == `src_color` extent (1:1 full-resolution pass; contrast with GRX-009's `max(source / 8, 1)` reduce shape).
- Required target device gate: 64-bit integer shader capability must be confirmed on the D3D12 device before any runtime attempt may proceed (b0 carries i64 dims; template parity with GRX-009).

## Supported Math Subset

The tracked kernel (`artifacts/hlsl_bridge/tonemap_apply.hlsl`) implements ONLY:

1. `color.rgb *= luminance_multiplier` (tonemap.glsl L860)
2. `color.rgb *= exposure` (L864/L870; no auto exposure)
3. `apply_tonemapping` with `TONEMAPPER_LINEAR` (identity, L247-249)
4. `linear_to_srgb` (L230-233, via the `FLAG_CONVERT_TO_SRGB` leg L942-943)
5. alpha passthrough

Everything else in `tonemap.glsl` `main()` is a recorded gap (see `pass_manifest.json` `known_gaps`).

## Fallback Rules

- The pass remains disabled by default and runtime remains `fallback_only`.
- Missing source or destination resource returns fallback.
- Descriptor layout mismatch returns fallback.
- ABI mismatch returns fallback through existing ABI validation paths.
- Missing 64-bit integer shader capability returns fallback.
- Missing `RXGD_CAP_TONEMAP_REAL_PASS` opt-in returns fallback (`manual_disabled`).
- Buffer (non-texture) resources fail the per-slot kernel-binding-kind conformance check.
- The shipping (feature-off) bridge fails closed with `real_dispatch_path_not_linked` even when every software gate passes.
- The native Godot tonemapper remains the active path whenever the bridge does not return OK.

## Explicit Non-Goals (this slice)

- No real runtime tonemap GPU pass is enabled by default.
- No Godot runtime native-handle resource binding is wired (0005/0007-level work, later slice).
- No in-engine visual diff evidence is produced (4g-level, later slice).
- No measured fallback telemetry is produced (4g-level, later slice).
- No gated real-pass enablement measured success (4h-level, later slice).
- No performance number or acceleration claim is made.
