# GRX bench workload v2 — calibration notes

> **NOT EVIDENCE.** This document records developer calibration only. The
> numbers below were captured with the **iter** profile (120 warmup / 600
> sample), **not** the evidence-grade **full** profile (300 warmup / 2000
> sample), and are **not** eligible for any perf gate. They exist solely to show
> that each v2 scene lands in a measurable frame-time band on this machine so the
> candidate pass set has room to move the numbers. No performance improvement is
> claimed or implied. The strict close-out baseline (`--profile full`, 3 runs on
> a quiet machine) is recorded separately and is not part of this file.

## What changed and why (GRX-004b)

The v1 scenes were a minimal 3D skeleton — a handful of placeholder nodes per
scene (e.g. 8 omni lights, one 256-instance MultiMesh, 5 materials) and
`auto_exposure` never enabled. Baselines sat at ~1700–2700 FPS (0.36–0.58 ms),
i.e. CPU-bound on the engine's fixed per-frame overhead and **insensitive to the
subsystem each scene is named for**. That makes the benchmark unable to measure
what it claims to measure. v2 rewrites each `_populate_scene()` to actually load
its named subsystem, keeping layouts deterministic (fixed per-scene RNG seed).
The perf gate math is unchanged (same seven scenes, same 300/2000 sampling,
same-scene baseline-vs-rurix comparison); only the workload inside each scene
changed. The v1 baseline (`baseline/baseline_full_20260708.json`) is retained
unmodified as a historical artifact.

## Machine state during calibration

- GPU: NVIDIA GeForce RTX 4070 Ti; 1920x1080; D3D12 Forward+; vsync off.
- CPU load ~6–7% idle; GPU idle before each run; **no** LLVM/ninja/cargo build
  running concurrently at capture time. If a heavy build is running, treat these
  numbers as directional only.
- Pipeline cache: warm (the D3D12 pipeline/shader cache in the project's user dir
  is populated after the first run of each scene). The first cold run of
  `material_variants` showed the expected PSO-compile stutter (avg ~317 FPS,
  p95 ~15 ms); warm steady-state is what the table reports. The full-profile 300
  warmup frames absorb PSO compilation; for evidence runs, doing a throwaway warm
  -up run first is recommended so run 1 is not penalized by cold-cache compiles.

## Calibration landing table (iter profile, warm cache)

Target band: ~30–300 FPS (≈3–30 ms/frame). All seven scenes land inside it.

| scene | v2 workload (key knobs) | avg FPS | frame ms | p95 ms |
|---|---|---|---|---|
| clustered_lights | 512 omni + 384 spot overlapping lights, 625 lit receiver boxes | 236.7 | 4.23 | 4.500 |
| many_mesh_instances | 200 000 independent MeshInstance3D + 60 000 MultiMesh | 223.2 | 4.48 | 4.545 |
| material_variants | 2048 distinct material variants × 45 000 shuffled instances | 260.3 | 3.84 | 4.167 |
| post_fx_chain | auto-exposure + 7-level glow + FILMIC + 2.0x supersample + 400 emissive spheres + 48 omni | 193.3 | 5.17 | 5.368 |
| volumetric_fog | fog density 0.1 + 400 overlapping omni lights + 500 pillars | 212.5 | 4.71 | 4.762 |
| particles | 600 000 GPU particles across 12 emitters | 219.0 | 4.57 | 4.762 |
| mixed_forward_plus | 15 000 mesh + 512 material-variant instances + 160 omni + 180 000 particles + glow/fog/auto-exposure | 239.6 | 4.17 | 4.545 |

(iter run_id `20260711T113204Z_iter` / `20260711T112707Z_iter` series; artifacts
under `target/grx/godot-bench-runs/`, gitignored.)

## Scene knob → FPS relationship observed

- **clustered_lights**: FPS is roughly inverse to (light count × per-cluster
  overlap). 896 lights with ranges 9–20 units over a ±28 unit spread ≈ 237 FPS.
  Per-type cluster capacity is 512 (silent drop above that, no error), so omni
  and spot counts are kept at/under 512 each.
- **many_mesh_instances**: frame time scales ~linearly with independent-instance
  count (CPU cull + draw-list build). 50 000 → ~1.4 ms; 200 000 → ~4.5 ms.
- **material_variants**: warm steady-state scales with draw count more than with
  variant count (Godot sorts opaque draws by material). 6 000 → ~2–3 ms warm;
  45 000 → ~3.8 ms. Variant count (2048) drives the cold-cache PSO-compile p95
  spike, which is the descriptor/PSO-churn signal GRX-017 targets.
- **post_fx_chain**: dominated by the 3D+post framebuffer size. 1.5x supersample
  ≈ 2 ms; 2.0x supersample ≈ 5.2 ms. auto-exposure is enabled via
  `CameraAttributesPractical` so Godot's luminance-reduction pass (the GRX-009
  target) actually runs.
- **volumetric_fog**: default froxel resolution makes the fog itself cheap; the
  in-band cost is deliberately driven by 400 overlapping lights + 500 pillars, so
  the savings must come from lighting/geometry passes (this scene has no
  dedicated Rurix pass).
- **particles**: frame time scales ~linearly with total particle count. 600 000
  particles (12 × 50 000) ≈ 4.6 ms.
- **mixed_forward_plus**: a proportional blend; lands near the middle of the band
  (~240 FPS) with all sub-workloads at roughly one-third scale.

## use_indirect status

`many_mesh_instances` also builds a standard `MultiMesh` component intended to
feed GRX-015/016/018 (GPU culling / compaction / indirect draw). The tracked
Godot build (`external/godot-master/scene/resources/multimesh.h`) exposes **no**
`use_indirect` property, so the indirect variant is **not** implemented; a
`TODO(GRX-015/016/018)` marks where to switch it once an indirect MultiMesh API
is confirmed for this build.

## v2.1 recalibration — scene-consumer FX added (NOT EVIDENCE)

The v2 scenes exercised each named subsystem but gave several candidate passes no
in-scene consumer. v2.1 adds those consumers as **scene semantics** (enabled for
both legs, not a rurix pass opt-in), so the perf gate can attribute savings:

- `post_fx_chain` and `mixed_forward_plus`: SSAO (`Environment.ssao_enabled`) —
  the `ssao_blur` (GRX-011) consumer.
- `mixed_forward_plus` and `many_mesh_instances`: temporal AA
  (`Viewport.use_taa`) — the `taa_resolve` (GRX-012) consumer.
- `particles`: one of the twelve emitters uses
  `GPUParticles3D.DRAW_ORDER_VIEW_DEPTH` — the `particles_copy` (GRX-013) depth
  -sort consumer.

The perf gate math is unchanged (same seven scenes, same 300/2000 sampling,
same-scene baseline-vs-rurix comparison, same 1.5/0.3/0.95 thresholds). Because
the per-scene workload changed, the evidence-grade `--profile full` baseline must
be re-recorded (that re-record is out of scope for this file / belongs to the
main session); the v2.20 table above is retained as the pre-FX v2 reference.

### Machine state during v2.1 recalibration

- GPU: NVIDIA GeForce RTX 4070 Ti; 1920x1080; D3D12 Forward+; vsync off.
- At capture time `nvidia-smi` reported ~3% GPU utilization / ~1.25 GiB used
  (desktop/overlay only); no Godot / dxc / llc / ninja / cargo build was running.
  Other agents on this machine may run dispatch smokes; none overlapped this
  short iter capture, but treat any single-run number as directional only.

### v2.1 landing table (iter profile, warm cache)

Target band: ~30–300 FPS. All four re-measured scenes stay inside it; SSAO/TAA
move FPS in the expected (lower) direction versus the v2.20 pre-FX numbers,
confirming the added effects actually run rather than being no-ops.

| scene | v2.1 FX added | avg FPS | frame ms | p95 ms | v2.20 avg FPS |
|---|---|---|---|---|---|
| post_fx_chain | + SSAO (`Environment.ssao_enabled`) | 180.8 | 5.53 | 5.689 | 193.3 |
| many_mesh_instances | + TAA (`Viewport.use_taa`) | 218.5 | 4.58 | 4.762 | 223.2 |
| particles | + one emitter `DRAW_ORDER_VIEW_DEPTH` | 226.0 | 4.43 | 4.545 | 219.0 |
| mixed_forward_plus | + SSAO + TAA | 225.8 | 4.43 | 4.545 | 239.6 |

(iter run_id `20260712T050832Z_iter`; artifacts under
`target/grx/godot-bench-runs/`, gitignored. The other three scenes
—`clustered_lights`, `material_variants`, `volumetric_fog`— are unchanged in
v2.1 and keep their v2.20 numbers.)
