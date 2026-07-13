# GRX-019 fused_post_chain — real auto-exposure parameter pipeline (design)

Status: **design only, not implemented.** Goal: replace the five placeholder AE
scalars in patch 0045 with the real per-frame values, closing the measured
`rd_native` parity gap `max_abs=85 / mean_abs=66` (thresholds `max<=4 / mean<=1.0`,
`rd_native_enablement_evidence.json` line 906/1147). No container or kernel change
is required — this is a pure call-site + hook-signature change.

---

## 1. The gap and its cause

The fused `rd_native` kernel *engages cleanly* (it is a texture pass — no device
removal; contrast the gpu_culling diagnosis) but its LDR output diverges from the
native auto-exposure reference by `max_abs=85, mean_abs=66` (out of 255). The
divergence is because patch 0045 hardcodes five AE scalars as best-effort
placeholders (`patch 0045` lines 280-284, packed at b0 bytes 32/36/40/56/60):

```
max_luminance       = 1.0f      // b0 dword 8   (byte 32)
min_luminance       = 0.0f      // b0 dword 9   (byte 36)
exposure_adjust     = 1.0f      // b0 dword 10  (byte 40)
first_frame         = 0.0f      // b0 dword 14  (byte 56)
auto_exposure_scale = 1.0f      // b0 dword 15  (byte 60)
```

The kernel's Segment B exposure formula (mirrors `tonemap.glsl` L866-868) is:

```
auto_exposure_denominator = lum_current * luminance_multiplier / auto_exposure_scale
exposure_effective        = exposure * (1 / auto_exposure_denominator)
rgb = linear_to_srgb(rgb * luminance_multiplier * exposure_effective)
```

`exposure_effective` is directly proportional to `auto_exposure_scale`. The
placeholder `auto_exposure_scale = 1.0` vs the native default `≈0.4`
(`camera_attributes_get_auto_exposure_scale`) is a ~**2.5× multiplicative exposure
error** — a scene-wide, roughly-uniform over-brightening. Segment A's placeholder
clamp `[min=0, max=1]` compounds it: if the scene's average luminance exceeds 1.0,
`lum_current` is clamped to 1.0 (native clamps to ~8.0), pushing
`exposure_effective` even higher (same direction). A 2.5× linear exposure maps
through sRGB (`^(1/2.4)`) to ≈1.47× in display space, which for mid-tones is a
+60…+85 shift that clips at 255 for bright texels — exactly the observed
`mean 66 / max 85` signature (`mean/max = 0.78` ⇒ a fairly uniform offset plus the
scene's albedo/sRGB spread).

## 2. The values already exist at the call site — no new plumbing to compute

Everything needed is already computed in the SAME function scope as the fused hook
call (`renderer_scene_render_rd.cpp`, hook at ~L991):

| b0 field (kernel) | real source at the call site | line |
|---|---|---|
| `auto_exposure_scale` (d15) | `auto_exposure_scale` = `camera_attributes_get_auto_exposure_scale(...)`; also stashed as `tonemap.auto_exposure_scale` | L678 / L790 |
| `min_luminance` (d9) | `auto_exposure_min_sensitivity` = `camera_attributes_get_auto_exposure_min_sensitivity(...)` | L569 |
| `max_luminance` (d8) | `auto_exposure_max_sensitivity` = `camera_attributes_get_auto_exposure_max_sensitivity(...)` | L570 |
| `exposure_adjust` (d10) | `step` = `camera_attributes_get_auto_exposure_adjust_speed(...) * time_step` | L568 |
| `first_frame` (d14) | `set_immediate` (bool) ⇒ `set_immediate ? 1.0f : 0.0f` | ~L598/L630/L673 |
| `exposure` (d11) | `tonemap.exposure` (already forwarded) | L852 |
| `white` (d12) | `tonemap.white` (already forwarded) | L851 |
| `luminance_multiplier` (d13) | `tonemap.luminance_multiplier` (already forwarded) | L873 |

These are the exact arguments the native `luminance->luminance_reduction(...,
auto_exposure_min_sensitivity, auto_exposure_max_sensitivity, step, set_immediate)`
consumes (L673), so mirroring them makes Segment A follow the native reduce, and
`auto_exposure_scale` makes Segment B follow the native tonemap.

## 3. Minimal change set (design — 3 edits, all in the existing 0045 slice)

**No b0 layout change, no DXIL/RTS0/container recompile.** All eight scalars
already exist in the 64-byte b0 (dwords 8-15) and the kernel already reads them;
only the runtime *values* the module packs change.

1. `drivers/d3d12/d3d12_hooks.h` + `modules/rurix_accel/rurix_accel.{h,cpp}`:
   extend `try_record_fused_post_chain_rd_native(...)` from the current 3 trailing
   scalars `(p_exposure, p_white, p_luminance_multiplier)` to 8:
   add `p_min_luminance, p_max_luminance, p_exposure_adjust, p_first_frame,
   p_auto_exposure_scale`. In the module body, **delete** the five placeholder
   constants and `memcpy` the passed values into b0 bytes 32/36/40/56/60 (the
   existing offsets are unchanged).

2. `servers/rendering/renderer_rd/forward_clustered/render_forward_clustered.cpp`
   (or `renderer_scene_render_rd.cpp`, wherever the fused hook is invoked): pass
   the §2 values. Scope caveat: `auto_exposure_scale` is already hoisted to the
   function scope (declared L555, `tonemap.auto_exposure_scale` at L790), but
   `auto_exposure_min_sensitivity`, `auto_exposure_max_sensitivity`, `step`, and
   `set_immediate` are declared inside the `if (auto_exposure...)` block (L568-570)
   and are likely NOT in scope at the fused call. **Hoist** them: declare four
   function-scope locals next to the existing `float auto_exposure_scale = 1.0;`
   (L555) with safe defaults (`min=0, max=large, step=1.0, first_frame=0`) and
   assign them inside the auto-exposure block. Then pass them to the hook.

3. Pass `set_immediate ? 1.0f : 0.0f` for `p_first_frame`.

That is the entire pipeline: it reuses values the native path already produces one
scope up, and it does not touch the container.

## 4. Will real params close 85/66? (magnitude analysis)

**The dominant, near-uniform component collapses; a small residual remains.**

- `auto_exposure_scale`: 1.0 → ~0.4 removes the ~2.5× exposure error — the single
  largest term and the source of most of the 66 mean offset.
- `max_luminance`: 1.0 → ~8.0 stops the spurious `lum_current` clamp for scenes
  whose average luminance exceeds 1.0, removing the second same-direction term.
- `min_luminance`, `exposure_adjust`, `first_frame`: at the capture frame (frame
  40, a static/flat scene at EMA steady-state) these have ~0 effect — the EMA has
  converged, so the adjust rate and first-frame reset no longer move `lum_current`.

Order of magnitude: 85/66 ≈ 33%/26% of 255 is fully explained by a ~2.5× uniform
exposure error; removing it should drop the diff by roughly an order of magnitude,
to the residual-only level. Residual sources that real b0 params do NOT fix (each
already a recorded known gap):

- **Segment-A source aliasing (the largest residual):** the fused scaffold aliases
  `lum_source`/`prev_luminance` to the single public 1×1 current-luminance buffer
  rather than the true ≤8×8 final reduce level. Segment A then averages a 1×1
  "tile" = the native luminance directly, so it is likely *close* to native — but
  this must be verified, and if the aliased buffer is not exactly the native EMA
  output the residual will be non-zero. Closing the last few LSB may require
  feeding Segment A the real ≤8×8 final-level source (a binding change, out of
  scope of the b0 param pipeline).
- **clamp-order gap** (member kernel clamp-then-EMA vs native EMA-inside-clamp):
  ~0 at steady-state, small during transients.
- **rgba16f/r32f storage quantization**, the **raster-vs-compute output seam**, and
  **sRGB not clamped to [0,1]**: a few LSB each.

**Conclusion:** the real-parameter pipeline is **necessary and high-leverage** — it
should bring `max_abs` from 85 down toward single digits by killing the uniform
exposure error. Whether it reaches the strict `max<=4 / mean<=1.0` gate depends on
the Segment-A aliasing residual; the pipeline should therefore be paired with a
check that the aliased 1×1 `lum_source` equals the native EMA output (or a switch
to the true final-level source). Landing the pipeline and re-running
`ci/grx_rb_fused_post_chain_rd_native_enablement_smoke.py` will measure the actual
residual; expect a large drop, with strict-tolerance closure contingent on the
aliasing.

## 5. Not implied

No container/kernel/RTS0 change; no `real_gpu_pass=true`; no default enablement; no
performance claim. This design only wires already-computed native AE values into
the existing b0 fields and analyses the expected parity improvement.
