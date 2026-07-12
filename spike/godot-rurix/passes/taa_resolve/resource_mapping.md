# GRX-012 TAA Resolve Resource Mapping

## Scope

This file records the GRX-012 taa_resolve resource mapping. It maps the real Godot TAA resolve resources and parameters into the Rurix bridge design while keeping the runtime fallback-first. This is not a real GPU runtime pass, does not skip the native Godot TAA resolve, and does not provide visual, telemetry, or performance evidence.

## Godot Native Flow

- Entry point: `RendererRD::TAA::resolve(...)` in `external/godot-master/servers/rendering/renderer_rd/effects/taa.cpp` (`L51-88`). This is a **single compute** resolve dispatch (Spartan-derived `taa_resolve.glsl`, `local_size 8x8x1`), `compute_list_dispatch_threads(p_resolution.x, p_resolution.y, 1)` (`L82`).
- Upstream call site: `render_forward_clustered.cpp` `_render_scene` -> `taa->process(...)` (around `L2512`).
- Pass shape: `TAA::process` runs `resolve` then maintains history with three physical `copy_to_rect` copies (resolve->temp, temp->internal, internal->history, velocity->prev_velocity) — pointer swaps are NOT used. **This slice's kernel covers ONE resolve dispatch; the history copy chain is a recorded gap (native continuation).**
- Parameters: `TAA::TAAResolvePushConstant` (`taa.h`, 16 bytes: `resolution[2]`, `disocclusion_threshold`, `variance_dynamic`), assembled at `taa.cpp:L67-73`.

## Godot Resources

| Godot resource | Role | Native binding shape | GRX-012 Rurix mapping |
| --- | --- | --- | --- |
| current-frame HDR color (rgba16f) | resolve input | `image2D color_buffer` (set 0 binding 0, restrict readonly) | `color_buffer` SRV `t0 space0`, `binding_kind = texture2d` (`Texture2D<float4>`) |
| depth | closest-velocity depth poll | `sampler2D depth_buffer` (set 0 binding 1) | `depth_buffer` SRV `t1 space0`, `binding_kind = texture2d` (`Texture2D<float>`; texelFetch -> Load) |
| velocity (rg16f) | current-frame motion | `image2D velocity_buffer` (set 0 binding 2, restrict readonly) | `velocity_buffer` SRV `t2 space0`, `binding_kind = texture2d` (`Texture2D<float2>`) |
| last velocity (rg16f) | previous-frame motion | `image2D last_velocity_buffer` (set 0 binding 3, restrict readonly) | `last_velocity_buffer` SRV `t3 space0`, `binding_kind = texture2d` (`Texture2D<float2>`) |
| history (previous resolve) | temporal accumulation | `sampler2D history_buffer` (set 0 binding 4) | `history_buffer` SRV `t4 space0`, `binding_kind = texture2d` (`Texture2D<float4>`; textureLod bilinear -> explicit float Load bilinear) |
| resolve output (rgba16f) | resolve output | `image2D output_buffer` (set 0 binding 5, restrict writeonly) | `output_buffer` UAV `u0 space0`, `binding_kind = rwtexture2d` (1:1, output extent == color extent; half quantization is a recorded gap) |

Resolve extent: full resolution (`p_resolution`).

## Parameters

| Rurix root constant | Godot source | Type in current artifact | Notes |
| --- | --- | --- | --- |
| `source_width` | `resolution.x` | `i64` | Carried as uint2 (low, high dword) in the HLSL cbuffer; high dword must be 0. Requires the 64-bit integer capability gate per the GRX-009 canonical template. `resolution.x = float(source_width)`. |
| `source_height` | `resolution.y` | `i64` | Same packing as `source_width`. `resolution.y = float(source_height)`. |
| `disocclusion_threshold` | `0.1 / max(res.x, res.y)` (`taa.cpp` push const) | `f32` | Consumed by `get_factor_disocclusion` (`taa_resolve.glsl` L310). |
| `variance_dynamic` | `params.variance_dynamic` | `f32` | Adaptive variance clip box size (`clip_history_3x3` L280). |
| `reserved0` | (none) | `f32` | Pads the canonical 7-dword / 28-byte block; the kernel does not consume it and the runtime must write 0. |

Root constants occupy 7 DWORDs (28 bytes) at root_parameter_index 0: `source_width` at DWORD 0..1, `source_height` at DWORD 2..3, `disocclusion_threshold` at DWORD 4, `variance_dynamic` at DWORD 5, `reserved0` at DWORD 6 — the same `[i64, i64, f32, f32, f32]` packing shape as the GRX-009/GRX-010/GRX-011 canonical layouts.

## Descriptor Layout

- Root constants / root-cbuffer mapping: `b0 space0` for `source_width`, `source_height`, `disocclusion_threshold`, `variance_dynamic`, `reserved0`.
- SRV descriptor range: `color_buffer = t0`, `depth_buffer = t1`, `velocity_buffer = t2`, `last_velocity_buffer = t3`, `history_buffer = t4` (all `binding_kind = texture2d`).
- UAV descriptor range: `output_buffer = u0 space0` (`binding_kind = rwtexture2d`).
- Single descriptor table: SRV range (t0..t4, 5 descriptors) precedes UAV range (u0, 1 descriptor), matching `rurixc::binding_layout::infer_root_signature` (SRV+UAV in one table, SRV range first).
- Required resource count for the bridge gate: 6 resources, in order color/depth/velocity/last_velocity/history/output.
- Required push constant size for the bridge gate: 28 bytes.
- Required output shape: `output_buffer` extent == `color_buffer` extent (1:1 full-resolution resolve pass; same shape rule as GRX-010's full-res tonemap).
- Required target device gate: 64-bit integer shader capability must be confirmed on the D3D12 device before any runtime attempt may proceed (b0 carries i64 dims; template parity with GRX-009/010/011).

## Supported Math Subset

The tracked kernel (`artifacts/hlsl_bridge/taa_resolve.hlsl`) implements a single full-resolution TAA resolve:

1. groupshared 10x10 tile (8x8 group + 1 border) caching clamped color+depth loads (`populate_group_shared_memory`).
2. `get_closest_pixel_velocity_3x3`: 3x3 min-depth poll, velocity fetched at the native `group_top_left + min_pos` position (the Spartan border-offset quirk is reproduced).
3. 9-tap Catmull-Rom history sampling; hardware bilinear `textureLod` is reproduced as explicit float 4-tap Load bilinear with clamp addressing.
4. `clip_history_3x3`: variance clip box (`clip_aabb` + adaptive `box_size` from `mix(0, variance_dynamic, smoothstep(0.02, 0, length(velocity_closest)))`).
5. blend factor: base `RPC_16 = 1/16`, plus out-of-screen reset (converge to current) and disocclusion boost, then luminance-diff flicker suppression.
6. Reinhard-domain lerp (`mix(history, input, blend)`) then inverse Reinhard.

Everything else in the native TAA path is a recorded gap (see `pass_manifest.json` `known_gaps`).

## Fallback Rules

- The pass remains disabled by default and runtime remains `fallback_only`.
- Any missing source or output resource returns fallback.
- Descriptor layout mismatch returns fallback.
- ABI mismatch returns fallback through existing ABI validation paths.
- Missing 64-bit integer shader capability returns fallback.
- Missing `RXGD_CAP_TAA_RESOLVE_REAL_PASS` opt-in returns fallback (`manual_disabled`).
- Buffer (non-texture) resources fail the per-slot kernel-binding-kind conformance check.
- The shipping (feature-off) bridge fails closed with `real_dispatch_path_not_linked` even when every software gate passes.
- The native Godot TAA resolve remains the active path whenever the bridge does not return OK.

## Explicit Non-Goals (this slice)

- No real runtime TAA resolve GPU pass is enabled by default.
- No Godot patch (0017-0019 are deferred to a later serial slice).
- No Godot runtime native-handle resource binding is wired.
- No history physical maintenance chain (resolve->temp->internal->history) is wired.
- No in-engine visual diff evidence is produced (4g-level, later slice; includes temporal/noise stability).
- No measured fallback telemetry is produced (4g-level, later slice).
- No gated real-pass enablement measured success (4h-level, later slice).
- No performance number or acceleration claim is made.
