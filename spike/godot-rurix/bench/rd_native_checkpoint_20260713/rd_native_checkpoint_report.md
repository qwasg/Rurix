# GRX Route B rd_native Wave 5 checkpoint (2026-07-13, measured_local, no performance claim)

First performance look at the rd_native TRUE-replacement legs. Unlike the
2026-07-12 Tier-1 exit (shim/scaffold path, whose numbers were declared invalid
for pass cost because per-dispatch stdout+readback dominated the measurement),
rd_native is bridge-independent and its hot path is stdout-clean, so this is the
first checkpoint where a leg's frame time can plausibly reflect the pass itself.
It is still a CHECKPOINT, not the strict close-out perf gate: every number is
`measured_local`, no pass is proposed for default-enable, and **no performance
claim of any kind is made**.

## Setup (both legs = SAME exe)

- exe: `target/grx/godot-scratch-rb2/bin/godot.windows.template_debug.x86_64.console.exe`
  (patch stack `0001-0029 + 0040-0045`; console-exe sha256 `22dfab424f71…`
  pinned identically in every one of the 9 summaries).
- baseline leg = all rurix settings at defaults (backend `0`, fail-closed native
  behavior); rurix leg = per-pass `backend=2` + `rd_container_path` via a
  transient `override.cfg`. NOTE: the aggregated baseline evidence carries the
  aggregator's stock "unmodified tracked Godot build" note; for v2.2 the binary
  is the rb2 patched exe with an unmodified (all-defaults) configuration — the
  summary's `godot_exe`/`godot_exe_sha256` fields carry the true provenance.
- containers: `target/grx/rd_containers/<pass>.rd_container.bin`, byte-identical
  to `spike/godot-rurix/rd-native-pipeline/out/` (the S2-proven ~1-ULP set;
  verified this session).
- `RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64` (signed DXC dxil.dll on PATH
  for the rd_native legs' pipeline builds).
- profile `full` = 300 warmup + 2000 sample frames, 1920x1080, vsync off, D3D12
  Forward+, machine quiet (exclusive; legs strictly sequential). Baseline v2.2 =
  3 full runs -> per-scene median; rurix = all5 + single-pass x5, 1 full run each.
- runner: `spike/godot-rurix/bench/run_benchmark_scenes.py` (this session's
  rd_native extension, see §1); matrices under `matrices/`; raw summaries in
  this directory (`baseline_run{1,2,3}.json`, `rurix_*.json`); analysis =
  `analyze_checkpoint.py` (reproduces every table below from the summaries).

## 1. Runner extension (run_benchmark_scenes.py)

- `VALID_PASS_MATRIX_KEYS` += the 12 GRX Route B keys (6 passes x `backend` +
  `rd_container_path`), names verified byte-for-byte against the
  `GLOBAL_DEF_BASIC` calls in patches 0040-0045.
- `load_pass_matrix` accepts an int ONLY for a `.../backend` key and ONLY in
  {0,1,2} (fail-closed otherwise); `render_override_cfg` emits the backend int
  BARE (`backend=2`) because a quoted `"2"` would parse as a Godot string and
  the `((int)GLOBAL_GET(...)) == 2` renderer gate would never match.
- **rd_native engagement semantics (designed + landed).** rd_native is
  bridge-independent: no shim engagement counter file, no `RXGD_SUMMARY` line —
  none of the four shim engagement sources can fire. Its only signal is the
  ONE-SHOT `RXGD_RD_NATIVE_<PASS> active` marker (module `print_verbose`, once
  at pipeline build). The runner therefore auto-adds `--verbose` for an
  rd_native leg, prepends `RURIX_DXC_DIR` to PATH, and records
  `pass_engagement[<pass>] = {"rd_native_active": bool, "mechanism":
  "one_shot_active_marker"}` with `pass_engagement_source =
  "rd_native_active_marker"`. Marker present == engaged (native pass skipped);
  absent == honestly failed closed to native. The engagement gate therefore
  cannot mis-kill an rd_native leg for lacking recorded/fallback counts — the
  mechanism difference is recorded, not papered over.

## 2. Subset-scene alignment (renderer-gate audit + marker probe, then full runs)

Gate conditions read from the landed patches; engagement confirmed twice
(quick-smoke probe, then the full runs — identical pattern):

| pass | backend==2 renderer gate (beyond the selector) | scenes engaged (full-run markers) | subset alignment verdict |
|---|---|---|---|
| tonemap (0040) | `rurix_tonemap_dest_texture.is_valid()` — **NO tonemap_mode check** | ALL 7 | Kernel = LINEAR + linear_to_srgb only. Subset-correct on the 4 no-WorldEnvironment scenes (clustered_lights, many_mesh_instances, material_variants, particles — Godot defaults to LINEAR there). **MISALIGNED on post_fx_chain / volumetric_fog / mixed_forward_plus (FILMIC): rd_native engages and writes LINEAR output — wrong pixels, no fail-closed mode guard.** Timing recorded; those cells flagged not-representative-of-correct-output. |
| ssao_blur (0041) | `blur_pipeline == SSAO_BLUR_PASS_SMART` | post_fx_chain, mixed_forward_plus | Aligned. SSAO on in exactly those 2 scenes; bench SSAO uses the SMART blur pipeline, inside the kernel subset. Other 5 scenes: SSAO off, honest non-engagement. |
| taa_resolve (0042) | (hook called when the TAA resolve dispatch runs) | many_mesh_instances, mixed_forward_plus | Aligned. `use_taa` on in exactly those 2 scenes. |
| particles_copy (0043) | `!do_sort && copy_mode_2d==0 && particles->userdata_count==0` | **NONE (0/7)** | **ZERO bench coverage.** Every bench emitter uses `ParticleProcessMaterial`, which always drives USERDATA1 (velocity accumulation) => `userdata_count==1` => fail-closed to native on every scene. Confirmed empirically (marker absent all 7 scenes) and by the patch-0043 in-source comment. The GRX-013 Z_BILLBOARD alignment fixed the SHIM-path subset (do_sort/FILL_INSTANCES) but the rd_native gate is stricter (userdata). |
| cluster_store (0044) | (hook at the cluster-store dispatch; upstream early-out when `render_element_count == 0`) | clustered_lights, post_fx_chain, volumetric_fog, mixed_forward_plus | Aligned. Engages exactly where clustered omni/spot lights exist; the other 3 scenes have only a DirectionalLight (0 cluster elements => native early-out before the hook). |
| fused_post_chain (0045) | excluded from this checkpoint | n/a | Not one of the 5 strict-success passes: its enablement gate is measured-blocked (luminance double-buffer aliasing guard; no strict success in the library). Runner keys added for completeness; no leg run. |

**Honest scene-generator decision:** no scene changes were made this round.
Honest per-scene recording was chosen over scene surgery: adding LINEAR-tonemap
variants or a no-userdata particle ShaderMaterial would change the measured
workload mid-checkpoint. Both gaps are recorded as next-batch scene/kernel
investment (GRX-004b precedent: gate math unchanged, motive documented).

## 3. Baseline v2.2 (rb2 exe, backend=0, 3 full runs, per-scene median)

| scene | median avg_fps | median p95 (ms) | run1 / run2 / run3 (fps) |
|---|---|---|---|
| clustered_lights | 235.20 | 4.5455 | 235.48 / 234.64 / 235.20 |
| many_mesh_instances | 223.73 | 4.5455 | 223.51 / 224.57 / 223.73 |
| material_variants | 262.23 | 4.1667 | 262.23 / 262.42 / 260.93 |
| post_fx_chain | 177.35 | 6.0606 | 177.70 / 177.14 / 177.35 |
| volumetric_fog | 212.56 | 5.0000 | 219.01 / 212.56 / 211.63 |
| particles | 215.35 | 4.9841 | 216.51 / 215.35 / 215.08 |
| mixed_forward_plus | 224.33 | 4.7619 | 224.09 / 224.33 / 224.33 |

Per-run geomean 221.38 / 220.26 / 219.90 (spread 0.7%); median-of-3 geomean
**220.23**. Aggregated evidence (v2.1-style median-geomean run pick = run2,
`20260713T050232Z_full`): `spike/godot-rurix/bench/baseline/baseline_full_workload_v2_2_20260713.json`
(perf_gate `--kind baseline --strict --validate-only` PASS). Noise floor note:
most scenes repeat within ±0.5%, but volumetric_fog spread 3.4% (219.0 vs
211.6) — single-run rurix deltas inside ~±3-4% on that scene are not
distinguishable from noise.

## 4. rd_native engagement per scene (all5 leg, one-shot active marker)

| scene | tonemap | ssao_blur | taa_resolve | particles_copy | cluster_store |
|---|---|---|---|---|---|
| clustered_lights | ENGAGED | — | — | — | ENGAGED |
| many_mesh_instances | ENGAGED | — | ENGAGED | — | — |
| material_variants | ENGAGED | — | — | — | — |
| post_fx_chain | ENGAGED | ENGAGED | — | — | ENGAGED |
| volumetric_fog | ENGAGED | — | — | — | ENGAGED |
| particles | ENGAGED | — | — | — | — |
| mixed_forward_plus | ENGAGED | ENGAGED | ENGAGED | — | ENGAGED |

Single-pass legs show the identical per-pass pattern (`pass_engagement_source =
rd_native_active_marker` on all 7 scenes of all 6 rurix legs; baseline legs
carry no marker — source null, correct). **Engagement validity verdict:
tonemap / ssao_blur / taa_resolve / cluster_store legs VALID (engaged on their
subset scenes); particles_copy leg INVALID for savings comparison — engagement
= none on all 7 scenes (userdata subset gap), its ratios are pure
native-vs-native noise and are excluded (`n/a`) from the engaged-only geomean.**

## 5. Per-pass avg_fps ratio (rurix / baseline_median; >1 == faster; * = engaged)

| scene | all5 | tonemap | ssao_blur | taa_resolve | particles_copy | cluster_store |
|---|---|---|---|---|---|---|
| clustered_lights | 1.027* | 0.990* | 1.032 | 1.033 | 1.031 | 1.031* |
| many_mesh_instances | 0.997* | 0.984* | 0.988 | 0.989* | 0.995 | 0.994 |
| material_variants | 0.943* | 0.948* | 1.014 | 0.991 | 0.991 | 0.991 |
| post_fx_chain | 1.009* | 1.010* | 0.995* | 0.994 | 0.995 | 0.994* |
| volumetric_fog | 0.980* | 0.924* | 0.994 | 0.994 | 1.001 | 1.019* |
| particles | 0.958* | 0.907* | 0.998 | 0.998 | 0.996 | 0.994 |
| mixed_forward_plus | 0.958* | 1.017* | 1.002* | 0.978* | 1.000 | 1.003* |
| **geomean (ALL 7)** | **0.9816** | **0.9676** | **1.0032** | **0.9968** | **1.0012** | **1.0036** |
| **geomean (ENGAGED only)** | **0.9816** | **0.9676** | **0.9986** | **0.9836** | **n/a (0 coverage)** | **1.0116** |

## 6. Per-pass p95 frame-time ratio (rurix / baseline_median; <1 == faster; * = engaged)

| scene | all5 | tonemap | ssao_blur | taa_resolve | particles_copy | cluster_store |
|---|---|---|---|---|---|---|
| clustered_lights | 0.971* | 1.000* | 0.960 | 0.955 | 0.957 | 0.969* |
| many_mesh_instances | 1.000* | 1.048* | 1.000 | 1.000* | 1.000 | 1.000 |
| material_variants | 1.000* | 1.091* | 1.000 | 1.000 | 1.000 | 1.000 |
| post_fx_chain | 0.967* | 0.973* | 1.000* | 1.000 | 1.000 | 1.000* |
| volumetric_fog | 1.000* | 1.000* | 1.000 | 1.000 | 1.000 | 0.975* |
| particles | 1.003* | 1.003* | 1.003 | 1.003 | 1.003 | 1.003 |
| mixed_forward_plus | 1.000* | 0.955* | 1.000* | 1.000* | 1.000 | 1.000* |
| **geomean (ENGAGED only)** | **0.9915** | **1.0091** | **1.0000** | **1.0000** | **n/a** | **0.9859** |

p95 caveat: the raw frame times sit on discrete values (e.g. 4.5455 = 1/220s,
4.1667 = 1/240s), so p95 ratios move in quantized steps; a 1.048/1.091 cell is
one bucket step, not a measured tail regression of that magnitude.

## 7. First real per-pass numbers vs the 2026-07-12 shim/scaffold Tier-1 exit

| leg | scaffold-era geomean (2026-07-12, shim + instrumentation) | rd_native geomean, engaged-only (this checkpoint) |
|---|---|---|
| all5 | 0.2542 | 0.9816 |
| tonemap | 0.3866 | 0.9676 |
| ssao_blur | 0.7180 (engagement then mis-parsed) | 0.9986 |
| taa_resolve | 0.7124 (engagement then mis-parsed) | 0.9836 |
| particles_copy | 0.9982 (zero engagement = noise) | n/a (zero engagement, now attributed: userdata gate) |
| cluster_store | (no scaffold leg) | 1.0116 |

The scaffold numbers were instrumentation-dominated slowdowns the Tier-1 report
itself ruled "不可作为 pass 本征开销的证据" (per-dispatch `RXGD_BRIDGE_REC`
stdout + readback + checksum on every dispatch, plus native re-render as
backstop = double work). rd_native removes that whole path AND the native
dispatch (true replacement): tonemap moved 0.3866 -> 0.9676, all5 0.2542 ->
0.9816. The remaining distance from 1.0 is the honest current picture of the
replacement itself, no longer harness artifact — with per-scene deltas at the
±1-4% level partly inside the single-run noise floor (tonemap's worst cells
0.907/0.924 sit on the noisiest scenes; repeats required before reading them as
pass cost).

## 8. Distance to the 1.5x ambition — attribution and next-batch investment

Measured: best engaged-only geomean = cluster_store 1.0116; all5 = 0.9816.
Distance to 1.5x: everything (~0.98-1.01 vs 1.50).

Attribution (in causal order):

1. **The replaced passes are micro-passes at this workload.** At ~220 FPS the
   whole frame is ~4.5 ms; tonemap/ssao-blur/taa-resolve/cluster-store are each
   on the order of tens of microseconds of GPU time at 1080p on this GPU. Even
   a ZERO-COST replacement of all five is bounded (Amdahl) at a few percent of
   whole-frame FPS — the 1.5x target is structurally unreachable from these
   five passes on these scenes, independent of kernel quality.
2. **The kernels are 1:1 ports.** Same algorithm, same dispatch dimensions,
   ~1-ULP parity by design (S2/S5 evidence) — the expected intrinsic delta is
   ~0 minus pipeline-switch overhead (extra PSO bind + uniform-set cache lookup
   per frame). The measured ~0.97-1.01 band is exactly that expectation.
3. **The bench is CPU/submission-bound in several scenes** (200k node cull in
   many_mesh_instances, template_debug engine overhead everywhere), so GPU-side
   micro-savings cannot move avg_fps there at all.
4. **Whole-frame FPS is the wrong instrument for micro-pass deltas.** The
   runner records `gpu_timestamps_available=false`; a 50 µs pass delta is ~1%
   of frame time — at or below the per-scene noise floor (±0.5-3.4%).

Next-batch investment (ranked by expected information per effort):

1. **Pass-level GPU timing** (D3D12 timestamp queries around the replaced vs
   native dispatch): turns invisible micro-deltas into direct pass-cost
   evidence; also decides whether pipeline-switch overhead is real.
   Prerequisite for ANY per-pass efficiency statement.
2. **Fused_post_chain unblock** (double-buffer luminance aliasing guard):
   fusing N post passes eliminates whole dispatch+barrier boundaries — the
   first structural (not 1:1) win candidate, and the only in-flight path to a
   measurable whole-frame delta on post-heavy scenes.
3. **Subset-gap closure for coverage, not speed:** (a) tonemap FILMIC — add the
   missing `tonemap_mode` fail-closed check to the 0040 gate (correctness
   hardening; currently WRONG PIXELS on 3 scenes), then extend the kernel
   subset or add a LINEAR bench variant scene; (b) particles_copy — either a
   no-userdata particle shader scene variant or kernel userdata support;
   current bench coverage is zero, so the pass has no perf evidence at all.
4. **A GPU-bound heavy scene tier** (lower FPS, larger post/cluster share) so
   pass savings are a measurable fraction of the frame.
5. **Bigger-pass targets** (gpu_culling 0027-0029 on the 200k-instance scene,
   light-assignment ahead of cluster_store): passes whose frame share is
   percent-scale rather than sub-percent.

## Measurement validity (no performance claim)

1. Everything here is `measured_local`, single-machine (RTX 4070 Ti,
   template_debug build), single full run per rurix leg; per-scene deltas under
   ~±4% on volumetric_fog/particles-class scenes are within or near the
   observed baseline noise floor. The geomeans, not individual cells, are the
   signal; none of it constitutes a performance claim or a default-enable case.
2. rd_native legs run with `--verbose` (required for the one-shot engagement
   marker). Verified non-polluting: log line counts scale with scene
   complexity, not frame count (2300-frame full runs produce ~the same 420-670
   lines as 90-frame smokes; the render loop emits nothing per-frame).
3. tonemap's FILMIC-scene cells time a PIXEL-INCORRECT output (subset
   misalignment, §2); they measure the kernel's dispatch cost honestly but must
   never be cited as a validated replacement on those scenes.
4. particles_copy: engagement=none on all scenes -> leg INVALID for
   comparison; recorded as subset-coverage-gap data.
5. Baseline v2.2 and all 6 rurix legs ran strictly sequentially on a quiet
   machine, same exe sha, failure_count=0, zero failure markers, the only
   warnings being the known allowlisted global-script-cache pair (14/leg =
   2 lines x 7 scenes).
