# GRX-019 Fused Post Chain Resource Mapping

## Scope

This file records the GRX-019 fused_post_chain resource mapping. It maps the real Godot resources of the two fusion members — the final WRITE_LUMINANCE luminance-reduction level and the LINEAR+sRGB tonemap subset — into a single Rurix fused-dispatch design while keeping the runtime fallback-first. This is not a real GPU runtime pass, does not skip the native Godot luminance_reduction or tonemap, and does not provide visual, telemetry, or performance evidence. No dispatch/barrier/VRAM-traffic or FPS improvement is claimed.

## Godot Native Flow

- Upstream call site: `renderer_scene_render_rd.cpp` `_render_buffers_post_process_and_tonemap` (`L459+`): the auto-exposure branch runs `luminance->luminance_reduction(...)` (`L555-588` region, `set_immediate` = native `p_set`, `step` = adjust speed x time_step), then the tonemap block sets `tonemap.exposure_texture = luminance->get_current_luminance_buffer(rb)` (`L697`) and `use_auto_exposure` / `auto_exposure_scale` (`L698-700`; invalid RID or auto exposure off falls back to `DEFAULT_RD_TEXTURE_WHITE`, `L702`).
- Member A (luminance final level): `Luminance::luminance_reduction` (`luminance.cpp` `L159-256`) reduces 8x per level; the FINAL level (`i == reduce.size()-1 && !p_set`, `L228-231`) uses the `WRITE_LUMINANCE` shader variant with `current` bound as prev (set 2) and writes `reduce[last]` (1x1); the chain ends with `SWAP(current, reduce[last])` (`L255`). Shader math (`luminance_reduce.glsl` `L76-79`): `avg = clamp(prev + (avg - prev) * exposure_adjust, min_luminance, max_luminance)` (EMA inside the clamp).
- Member B (tonemap): fullscreen fragment pass (`tone_mapper.cpp`); `tonemap.glsl` `L860` (`*= luminance_multiplier`), `L864-870` exposure with the auto-exposure leg `L866-868` (`exposure *= 1.0 / (texelFetch(source_auto_exposure, ivec2(0,0), 0).r * params.luminance_multiplier / params.auto_exposure_scale)`), `L893` `apply_tonemapping` (LINEAR = identity), `L942-943` `linear_to_srgb` under `FLAG_CONVERT_TO_SRGB`.
- **Key gap being fixed by the fusion**: the native tonemap reads the luminance current value through the `exposure_texture` sampler (`renderer_scene_render_rd.cpp:697`), but patch 0012 forwards only the scalar `exposure/white/luminance_multiplier` to the bridge — no `exposure_texture` handle — so the existing Rurix tonemap pass is semantically correct only with auto exposure OFF. The fused kernel makes the in-kernel register-carried luminance current the tonemap exposure input per the native `L866-868` formula, covering the auto-exposure-ON scenario.
- Pass shape: ONE fused compute dispatch, full-resolution thread grid (tonemap shape); the luminance final level rides along as a per-thread-group register-resident prologue (redundantly recomputed per group from the <=8x8 `lum_source`, broadcast via groupshared; group (0,0) thread 0 writes the 1x1 `dst_luminance`).

## Godot Resources

| Godot resource | Role | Native binding shape | GRX-019 Rurix mapping |
| --- | --- | --- | --- |
| full-res HDR internal texture (rgba16f) | tonemap input | `sampler2D source_color` (tonemap set 0) | `src_color` SRV `t0 space0`, `binding_kind = texture2d` (`Texture2D<float4>`; textureLod at texel centers -> Load) |
| last intermediate luminance reduce level (r32f, <=8x8) | final-level reduction source | `image2D source_luminance` (luminance_reduce set 0) | `lum_source` SRV `t1 space0`, `binding_kind = texture2d` (`Texture2D<float>`) |
| previous-frame 1x1 luminance `current` | EMA prev | `sampler2D prev_luminance` (luminance_reduce set 2) | `prev_luminance` SRV `t2 space0`, `binding_kind = texture2d` (`Texture2D<float>`) |
| LDR output | tonemap output | native: fullscreen fragment framebuffer write | `dst_color` UAV `u0 space0`, `binding_kind = rwtexture2d` (1:1, extent == src_color extent; recorded raster-vs-compute seam) |
| this-frame 1x1 luminance `current` | EMA output | `image2D dest_luminance` (luminance_reduce set 1) then `SWAP(current, reduce[last])` | `dst_luminance` UAV `u1 space0`, `binding_kind = rwtexture2d` (1x1; MUST be a different resource than `prev_luminance` — double buffering mirrors the native SWAP) |

Tonemap extent: full resolution (`src_color` extent). Luminance source extent: <= 8x8 (single tile; mirrors the native final-level shape where the destination is 1x1 after one 8x reduction).

## Parameters

Merged b0 (member A + member B canonical layouts + fusion controls). Root constants occupy **16 DWORDs (64 bytes)** at root_parameter_index 0 with the `[i64, i64, i64, i64, f32 x8]` packing shape (i64 dims first, f32 scalars after — the same packing discipline as the GRX-009..014 canonical layouts; the 64-byte size intentionally departs from the members' 28-byte shape and the S4 gate must validate 64 bytes).

| Rurix root constant | Godot source | Type | Dwords | Notes |
| --- | --- | --- | --- | --- |
| `source_width` | HDR color width | `i64` | 0-1 | Carried as uint2 (low, high dword) in the HLSL cbuffer; high dword must be 0. Requires the 64-bit integer capability gate per the GRX-009 canonical template. |
| `source_height` | HDR color height | `i64` | 2-3 | Same packing. Tonemap-segment bounds check. |
| `lum_source_width` | final-level luminance source width | `i64` | 4-5 | Must be <= 8 (single-tile subset gate). Partial-tile divisor input. |
| `lum_source_height` | final-level luminance source height | `i64` | 6-7 | Must be <= 8. |
| `max_luminance` | `LuminanceReducePushConstant.max_luminance` | `f32` | 8 | Member A clamp upper bound. |
| `min_luminance` | `LuminanceReducePushConstant.min_luminance` | `f32` | 9 | Member A clamp lower bound. Parity fixtures use `min_luminance > 0` (see divide-by-zero gap). |
| `exposure_adjust` | `LuminanceReducePushConstant.exposure_adjust` (= adjust speed x time_step) | `f32` | 10 | EMA blend factor. |
| `exposure` | `TonemapPushConstant.exposure` | `f32` | 11 | Base exposure before the auto-exposure factor. |
| `white` | `TonemapPushConstant.white` | `f32` | 12 | Unused by TONEMAPPER_LINEAR; carried for member-B shape parity. |
| `luminance_multiplier` | `TonemapPushConstant.luminance_multiplier` | `f32` | 13 | Consumed twice per the native formula: the color pre-scale (`L860`) and the auto-exposure denominator (`L867`). |
| `first_frame` | native `p_set` (`set_immediate`) | `f32` | 14 | 0.0/1.0. When nonzero the kernel outputs `cur = clamp(avg)` and skips the EMA (bounded documented divergence from the native p_set raw-avg plain reduce; see known gaps). |
| `auto_exposure_scale` | `TonemapPushConstant.auto_exposure_scale` | `f32` | 15 | Auto-exposure formula factor (`L867`). |

## Descriptor Layout

- Root constants / root-cbuffer mapping: `b0 space0`, 16 dwords / 64 bytes as tabled above, root_parameter_index 0.
- SRV descriptor range: `src_color = t0`, `lum_source = t1`, `prev_luminance = t2` (all `binding_kind = texture2d`).
- UAV descriptor range: `dst_color = u0`, `dst_luminance = u1` (both `binding_kind = rwtexture2d`).
- Single descriptor table: SRV range (t0..t2, 3 descriptors) precedes UAV range (u0..u1, 2 descriptors), matching `rurixc::binding_layout::infer_root_signature` (SRV+UAV in one table, SRV range first).
- Required resource count for the (deferred S4) bridge gate: 5 resources, in order src_color/lum_source/prev_luminance/dst_color/dst_luminance.
- Required push constant size for the bridge gate: **64 bytes**.
- Required shapes: `dst_color` extent == `src_color` extent (1:1 full-resolution); `dst_luminance` and `prev_luminance` extents == 1x1; `lum_source` extent <= 8x8 and equal to the b0 lum dims; `prev_luminance` and `dst_luminance` must be different resources.
- Required target device gate: 64-bit integer shader capability must be confirmed on the D3D12 device before any runtime attempt may proceed (b0 carries i64 dims; template parity with GRX-009..014).

## Supported Math Subset

The tracked kernel (`artifacts/hlsl_bridge/fused_post_chain.hlsl`) implements ONE fused full-resolution dispatch:

1. **Segment A (luminance final level, per-group register prologue)**: partial-tile-correct mean over `lum_source` (<=8x8, same accumulation order as the member kernel `../luminance_reduction/artifacts/hlsl_bridge/luminance_reduce_level.hlsl -D RX_WRITE_LUMINANCE` evaluated at destination texel (0,0)); `cur = clamp(avg, min_luminance, max_luminance)`; `ema = prev + (cur - prev) * exposure_adjust`; `first_frame != 0` selects `cur`. Redundantly recomputed per thread group, broadcast via groupshared; dispatch group (0,0) thread 0 writes `dst_luminance[0,0]`.
2. **Segment B (tonemap LINEAR+sRGB)**: `color.rgb *= luminance_multiplier`; `exposure_effective = exposure * (1.0 / (lum_current * luminance_multiplier / auto_exposure_scale))` with `lum_current` register-carried from segment A (native `tonemap.glsl L866-868` operation order); `color.rgb *= exposure_effective`; TONEMAPPER_LINEAR identity; `linear_to_srgb` (`tonemap.glsl L230-233` coefficients); alpha passthrough.

Everything else in the native post chain is a recorded gap (see `pass_manifest.json` `known_gaps`), notably: glow composite (all modes, including gaussian_glow's own luminance consumption), non-LINEAR tonemappers (Reinhard/FILMIC/ACES/AgX), the rest of the auto-exposure texture chain (upper reduce levels, other exposure_texture consumers), FXAA/BCS/color correction/debanding/multiview/HDR output, half storage quantization, and the one-frame-latency constraint on the tonemap segment.

## Fallback Rules (two-level contract)

- The pass remains disabled by default and runtime remains `fallback_only`.
- **Level 1 (fusion -> members)**: any fused-gate failure falls back to the per-member single-pass gated paths (the existing `LuminanceReductionGate` / `TonemapGate` opt-in arms, themselves fallback by default). The fusion-level marker must be distinguishable from the member-level markers (patch slice 0036-0038, deferred).
- **Level 2 (members -> native)**: any member-gate failure falls back to the native Godot `luminance_reduction` + `tonemap` path.
- Any missing source or output resource returns fallback.
- Descriptor layout mismatch (including a b0 size other than 64 bytes) returns fallback.
- `lum_source` extent > 8x8, non-1x1 `prev_luminance`/`dst_luminance`, `dst_color` extent != `src_color` extent, or `prev_luminance` aliasing `dst_luminance` returns fallback.
- ABI mismatch returns fallback through existing ABI validation paths.
- Missing 64-bit integer shader capability returns fallback.
- Missing `RXGD_CAP_FUSED_POST_CHAIN_REAL_PASS` (1 << 13) opt-in returns fallback (`manual_disabled`).
- Buffer (non-texture) resources fail the per-slot kernel-binding-kind conformance check.
- The shipping (feature-off) bridge fails closed with `real_dispatch_path_not_linked` even when every software gate passes.
- The native Godot post chain remains the active path whenever the bridge does not return OK.

## Explicit Non-Goals (this slice)

- No real runtime fused GPU pass is enabled by default.
- No bridge gate is wired (S4 is a later slice; this slice is S1-S3 only).
- No Godot patch (0036-0038 are deferred to a later serial slice).
- No standalone D3D12 dispatch smoke (S6, later slice).
- No Godot runtime native-handle resource binding is wired.
- No in-engine visual diff evidence is produced (4g-level, later slice).
- No measured fallback telemetry is produced (4g-level, later slice).
- No gated real-pass enablement measured success (4h-level, later slice).
- No dispatch/barrier/VRAM-traffic proxy, FPS number, or acceleration claim is made.
