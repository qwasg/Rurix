# GRX Route B rd_native TERMINAL benchmark campaign (2026-07-13, measured_local, no performance claim)

The ceiling-quantification campaign closing the rd_native measurement arc: rb4
exe (full 0001-0029 + 0040-0048 revision stack) x workload v2.3 x per-pass GPU
timestamps. Delivers the three things the Wave 5 checkpoint said it could not:
(1) tonemap timed ONLY where its output is pixel-correct (the 0040 LINEAR mode
guard now fail-closes FILMIC scenes), (2) particles_copy's FIRST engaged
measurement (v2.3 no-userdata emitter subset), and (3) direct per-pass GPU-µs
attribution + an Amdahl ceiling for the whole five-pass program. Every number
is `measured_local` on one machine (RTX 4070 Ti, 1080p, template_debug); **no
performance claim of any kind is made** and no pass is proposed for
default-enable.

## Setup (all 7 legs = SAME exe, strictly serial, machine quiet/exclusive)

- exe: `target/grx/godot-scratch-rb4/bin/godot.windows.template_debug.x86_64.console.exe`,
  console-exe sha256 `fc41853b5a2c501a…`, patch stack `0001-0029+0040-0048`
  (identical tuple of exe sha / dll sha `47910fe7…` / patch id / run_mode=full
  across all 7 archived summaries; verified this session).
- workload v2.3 (project regenerated from the tracked generator this session):
  many_mesh_instances += RID-direct INDIRECT MultiMesh 40k; particles += 3x50k
  no-userdata custom-process emitters (stride 112); timestamp-collection
  autoload. v2.3 changes the scene load, so the v2.2 baseline is retired and
  baseline v2.3 is re-recorded here.
- containers: `target/grx/rd_containers/<pass>.rd_container.bin` (staged set;
  gpu_culling NOT armed — its enablement is mechanism-blocked with a device
  removal, so arming it in a bench leg would be dishonest).
- `RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64` for rd_native legs.
- profile `full` = 300 warmup + 2000 sample frames, 1920x1080, vsync off, D3D12
  Forward+. Runner: `spike/godot-rurix/bench/run_benchmark_scenes.py`
  (unchanged from the Wave 5 session's rd_native + --gpu-timestamps extension).
- leg matrix (7 serial full runs, ~90 s each):
  | # | leg | matrix | --gpu-timestamps | archived as |
  |---|---|---|---|---|
  | 1-3 | baseline x3 | none (backend all 0) | no | `baseline_run{1,2,3}.json` |
  | 4 | all5 | 5 passes backend=2 | no | `rurix_all5.json` |
  | 5 | all5+fused | 5 passes + fused_post_chain backend=2 | no | `rurix_all5_fused.json` |
  | 6 | baseline-ts | none | yes | `baseline_ts.json` |
  | 7 | all5-ts | 5 passes backend=2 | yes | `rurix_all5_ts.json` |
- ratio legs (1-5) are timestamp-free by design: `--gpu-profile` measurably
  costs FPS (baseline-ts vs baseline median: clustered 231.4 vs 239.3), so the
  ts legs are attribution-only and never enter a ratio.
- every leg: failure_count=0, zero failure markers; the only warnings are the
  known allowlisted global-script-cache pair (14/leg = 2 lines x 7 scenes).
- analysis: `analyze_final.py` (reproduces every table below from the archived
  summaries; raw stdout archived as `analysis_output.txt`).
- campaign interruption (honest record): the first driver process silently died
  after legs 1-2 (archived summaries intact, no partial data used); legs 3-7
  were re-run to completion in two synchronous chunks
  (`campaign_progress.log` carries both segments). Leg 3's aborted first
  attempt left only an empty run dir; its archived summary comes entirely from
  the completed re-run `20260713T101545Z_full`.

## 1. Baseline v2.3 (rb4 exe, backend=0, 3 full runs, per-scene median)

| scene | median avg_fps | median p95 (ms) | run1 / run2 / run3 (fps) |
|---|---|---|---|
| clustered_lights | 239.33 | 4.1830 | 239.33 / 245.39 / 233.52 |
| many_mesh_instances | 220.63 | 4.6044 | 221.86 / 220.63 / 217.15 |
| material_variants | 249.49 | 4.2200 | 254.86 / 249.49 / 240.44 |
| post_fx_chain | 178.10 | 5.9678 | 178.10 / 178.49 / 177.33 |
| volumetric_fog | 215.16 | 4.7619 | 215.16 / 214.80 / 215.36 |
| particles | 175.45 | 6.0606 | 176.08 / 175.45 / 174.28 |
| mixed_forward_plus | 222.63 | 4.7619 | 222.63 / 221.32 / 222.84 |

Per-run geomean 213.66 / 213.33 / 210.12 (spread 1.69%); **baseline
median-of-3 geomean 212.74**; v2.1-style median-geomean run pick = run2
(`20260713T100751Z_full`, geomean 213.33). Noise floor: post_fx_chain /
volumetric_fog / mixed repeat within ±0.4%, but clustered_lights spreads 4.9%
(233.5-245.4) and material_variants 5.7% (240.4-254.9) this round — single-run
rurix deltas inside ~±2-3% on those two scenes are not distinguishable from
noise; geomeans, not cells, are the signal.

## 2. Engagement matrix (one-shot active marker; expected vs observed, per leg)

Observed pattern is IDENTICAL across all four rurix legs (all5, all5_fused,
all5_ts, and the pre-campaign quick-smoke probe); `pass_engagement_source =
rd_native_active_marker` on every rurix-leg scene; all baseline legs carry no
marker (source null, correct). fused column from the all5_fused leg (the only
leg where fused was armed).

| scene | tonemap | ssao_blur | taa_resolve | particles_copy | cluster_store | fused_post_chain |
|---|---|---|---|---|---|---|
| clustered_lights | **ENGAGED** | — | — | — | **ENGAGED** | — |
| many_mesh_instances | **ENGAGED** | — | **ENGAGED** | — | — | — |
| material_variants | **ENGAGED** | — | — | — | — | — |
| post_fx_chain | — (FILMIC) | **ENGAGED** | — | — | **ENGAGED** | — (FILMIC) |
| volumetric_fog | — (FILMIC) | — | — | — | **ENGAGED** | — (FILMIC) |
| particles | **ENGAGED** | — | — | **ENGAGED** | — | — |
| mixed_forward_plus | — (FILMIC) | **ENGAGED** | **ENGAGED** | — | **ENGAGED** | — (FILMIC) |

Verdict against the pre-registered expectations — ALL SIX MATCH:

1. **tonemap 4/7 (LINEAR subset), as expected.** The 0040 mode guard now
   fail-closes the three FILMIC scenes that Wave 5 flagged as WRONG PIXELS;
   every timed tonemap cell in this campaign is pixel-correct-subset only.
2. **ssao_blur 2/7** (SSAO on + SMART blur: post_fx_chain, mixed) — aligned.
3. **taa_resolve 2/7** (use_taa: many_mesh, mixed) — aligned.
4. **particles_copy 1/7 — FIRST-EVER engaged coverage** on the particles
   scene's v2.3 no-userdata emitter subset (Wave 5: 0/7, leg invalid).
   mixed_forward_plus stays honestly False: its 3 emitters are all standard
   `ParticleProcessMaterial` (userdata_count==1, out of subset by design).
5. **cluster_store 4/7** (scenes with clustered omni/spot lights) — aligned.
6. **fused_post_chain 0/7 — expected and confirmed.** The 0045 gate requires
   the LINEAR-tonemap subset AND auto-exposure-produced distinct luminance
   buffers; across the 7 bench scenes those are mutually exclusive (the 4
   LINEAR scenes have no CameraAttributes AE; the 2 AE scenes are FILMIC).
   No scene was invented to force it (honest-record precedent, Wave 5 §2).
   **The all5_fused leg is therefore INVALID as a fused measurement and is
   recorded as an all5 duplicate control** (see §3).

## 3. Ratio tables (rurix / baseline_median; single full run per rurix leg)

avg_fps ratio (>1 == faster; * = >=1 pass engaged on that scene — true of
every scene in both legs, so geomean(ALL 7) == geomean(ENGAGED)):

| scene | all5 | all5_fused (control) |
|---|---|---|
| clustered_lights | 1.0155* | 1.0130* |
| many_mesh_instances | 0.9868* | 0.9916* |
| material_variants | 0.9850* | 0.9859* |
| post_fx_chain | 0.9982* | 0.9927* |
| volumetric_fog | 0.9921* | 0.9943* |
| particles | 0.9927* | 0.9928* |
| mixed_forward_plus | 0.9893* | 0.9800* |
| **geomean (engaged = ALL 7)** | **0.9942** | **0.9929** |

p95 frame-time ratio (<1 == faster; same engagement marking):

| scene | all5 | all5_fused (control) |
|---|---|---|
| clustered_lights | 1.0435* | 1.0439* |
| many_mesh_instances | 1.0342* | 1.0342* |
| material_variants | 0.9874* | 0.9874* |
| post_fx_chain | 1.0156* | 1.0156* |
| volumetric_fog | 1.0454* | 1.0412* |
| particles | 1.0000* | 1.0000* |
| mixed_forward_plus | 1.0000* | 1.0000* |
| **geomean (engaged = ALL 7)** | **1.0178** | **1.0172** |

- p95 caveat (unchanged from Wave 5): raw frame times sit on discrete
  1/fps-bucket values, so p95 ratios move in quantized steps; a 1.03-1.05 cell
  is one bucket step, not a measured tail regression of that magnitude.
- **all5_fused vs all5 geomean delta = 0.13% — inside single-run noise.**
  Since fused never engaged, this pair is a genuine A/A control: arming the
  fused backend selector (one extra GLOBAL_GET int compare per frame plus a
  container stat at startup) has no measurable cost. It is NOT evidence about
  the fused kernel itself, which remains unmeasured on this bench (no
  engaging scene exists).
- The shadow-recompute net-cost question the leg was designed to answer is
  therefore **unanswerable on this workload** (the gate never opened);
  answering it requires a LINEAR+AE scene variant — recorded as scene-tier
  investment, not hacked in mid-campaign.

## 4. Per-pass GPU budget (µs, bucket medians, baseline-ts vs all5-ts)

Buckets are Godot RENDER_TIMESTAMP brackets; the rd_native dispatch replaces
the native dispatch INSIDE the same bracket, so `delta = B - R` is the
intrinsic cost difference of the replacement (negative == the Rurix pass costs
more GPU time than the native pass it replaces). Timestamp tick granularity is
~1 µs (medians snap to ticks; equal medians == indistinguishable at tick
resolution, cross-checked against means). 2000 frames sampled per scene per
leg, 0 discarded samples. Only engaged cells (bold) attribute the replacement;
non-engaged cells are native-vs-native and sit at delta≈0 as expected.

| scene | bucket | baseline (µs) | all5 (µs) | delta (µs) | engaged |
|---|---|---|---|---|---|
| clustered_lights | Tonemap | 17.41 | 30.72 | **-13.31** | yes |
| clustered_lights | Pack 3D Cluster Elements | 110.59 | 117.76 | **-7.17** | yes |
| clustered_lights | frame GPU total | 3978.24 | 4005.89 | -27.65 | |
| many_mesh_instances | Tonemap | 28.67 | 45.06 | **-16.38** | yes |
| many_mesh_instances | TAA | 197.63 | 311.30 | **-113.66** | yes |
| many_mesh_instances | frame GPU total | 1511.42 | 1610.75 | -99.33 | |
| material_variants | Tonemap | 34.82 | 65.54 | **-30.72** | yes |
| material_variants | frame GPU total | 818.18 | 871.42 | -53.25 | |
| post_fx_chain | Process SSAO | 238.59 | 238.59 | **0.00** | yes |
| post_fx_chain | Pack 3D Cluster Elements | 31.74 | 31.74 | **0.00** | yes |
| post_fx_chain | Tonemap (not engaged, FILMIC) | 106.50 | 106.50 | 0.00 | no |
| post_fx_chain | Auto exposure (not replaced) | 246.78 | 246.78 | 0.00 | no |
| post_fx_chain | frame GPU total | 5468.16 | 5467.14 | +1.02 | |
| volumetric_fog | Pack 3D Cluster Elements | 97.28 | 107.52 | **-10.24** | yes |
| volumetric_fog | frame GPU total | 4489.22 | 4492.29 | -3.07 | |
| particles | Tonemap | 18.43 | 32.77 | **-14.34** | yes |
| particles | Particles View-Axis Copy | 456.70 | 457.73 | **-1.02** | yes |
| particles | frame GPU total | 5514.24 | 5530.62 | -16.38 | |
| mixed_forward_plus | Process SSAO | 64.51 | 64.51 | **0.00** | yes |
| mixed_forward_plus | TAA | 132.10 | 231.42 | **-99.33** | yes |
| mixed_forward_plus | Pack 3D Cluster Elements | 35.84 | 39.94 | **-4.10** | yes |
| mixed_forward_plus | frame GPU total | 4578.30 | 4537.86 | +40.45 | |

(`Particles View-Axis Copy` is the patch-0047 bracket. The full per-bucket
dump, including means and p95s, is in the archived ts summaries; the table
keeps medians. mixed's frame-total +40 µs "gain" despite the slower TAA is
frame-total run-to-run noise, not a saving.)

**First direct per-pass intrinsic-cost attribution (the Wave 5 top-ranked
investment, delivered):**

1. **taa_resolve is the dominant intrinsic regression: ≈ +100-114 µs/frame**
   (many_mesh 198→311, mixed 132→231; means agree: 158→258). The Rurix TAA
   kernel costs ~1.6-1.8x the native resolve inside its bracket.
2. **tonemap costs ≈ +13-31 µs/frame, ~1.8-1.9x its (tiny) native bucket**
   consistently across all four engaged LINEAR scenes.
3. **cluster_store costs ≈ +4-10 µs/frame (~6-11%)** across its three engaged
   ts-leg scenes.
4. **ssao_blur and particles_copy are ≈ 0-delta at tick resolution.** For
   ssao_blur the replaced blur is a small slice of a gather-dominated bucket;
   for particles_copy the engaged no-userdata subset is 3 of 15 emitters in a
   bandwidth-bound copy — a 1:1-port cost profile in both cases.
5. Wave 5's hypothesis "the ~0.97-1.01 band is pipeline-switch overhead" is
   now REFINED: the overhead is real but pass-specific — concentrated in TAA
   and tonemap, near-zero in ssao_blur/particles_copy.

## 5. Amdahl ceiling (report core: the quantified upper bound)

Per scene: `replaceable fraction = Σ(engaged replaceable-bucket median µs) /
frame GPU total µs` from the BASELINE-ts leg; ceiling = 1/(1-fraction) — the
avg-FPS ratio that would result if every engaged pass became literally
ZERO-COST (kernels + dispatch + barriers all free), keeping the GPU-bound
approximation (CPU-bound scenes cap even lower, so this is generous).

| scene | engaged replaceable (µs) | frame GPU total (µs) | fraction | ceiling (x) | measured all5 |
|---|---|---|---|---|---|
| clustered_lights | 128.00 | 3978.24 | 0.0322 | 1.0332 | 1.0155 |
| many_mesh_instances | 226.30 | 1511.42 | 0.1497 | 1.1761 | 0.9868 |
| material_variants | 34.82 | 818.18 | 0.0426 | 1.0444 | 0.9850 |
| post_fx_chain | 270.34 | 5468.16 | 0.0494 | 1.0520 | 0.9982 |
| volumetric_fog | 97.28 | 4489.22 | 0.0217 | 1.0221 | 0.9921 |
| particles | 475.14 | 5514.24 | 0.0862 | 1.0943 | 0.9927 |
| mixed_forward_plus | 232.45 | 4578.30 | 0.0508 | 1.0535 | 0.9893 |
| **geomean** | | | | **1.0669** | **0.9942** |

**The ceiling numbers (measured_local, this workload/GPU/build):**

- **Theoretical zero-cost geomean ceiling = 1.0669x.** Even if all five
  engaged passes cost nothing at all, this workload cannot show more than
  ~6.7% geomean FPS. The best single-scene ceiling is many_mesh 1.176x —
  and that scene is additionally CPU/submission-bound (200k node cull), so
  its GPU-side ceiling is not reachable in avg_fps anyway.
- **Measured all5 = 0.9942 — the program currently sits ~7.3 points BELOW its
  own zero-cost ceiling**, of which the per-pass table attributes the largest
  identified slice to TAA (+100-114 µs) and tonemap (+13-31 µs) intrinsic
  costs; the rest is inside the noise floor.
- **The 1.5x ambition is structurally unreachable from these five passes on
  this workload — now with numbers**: 1.50 vs a 1.0669 hard upper bound.
  Reaching 1.5x via pass replacement requires replacing buckets worth ≥33% of
  frame GPU time (e.g. Render Opaque Pass at 930-4155 µs, Render Depth
  Pre-Pass, Glow), or a workload whose replaceable share is that large.
  Wave 5's qualitative Amdahl attribution (§8.1) is hereby quantified.

## 6. Comparison vs the Wave 5 checkpoint (2026-07-13, v2.2/rb2)

Caliber differences, stated up front: scenes v2.3 (heavier particles +150k,
many_mesh +40k indirect) vs v2.2; exe rb4 (0040-0048 revisions incl. the 0040
LINEAR mode guard + 0047 timestamp bracket + 0048 getters) vs rb2 (0040-0045);
so cells are NOT directly comparable — trends only.

| quantity | W5 checkpoint (v2.2/rb2) | this campaign (v2.3/rb4) |
|---|---|---|
| baseline geomean | 220.23 | 212.74 (v2.3 heavier: particles 215.35→175.45, many_mesh 223.73→220.63) |
| tonemap engagement | 7/7, of which 3 FILMIC scenes WRONG PIXELS | 4/7 LINEAR-only (mode guard fail-closed; every timed cell pixel-correct-subset) |
| particles_copy engagement | 0/7 — leg INVALID | 1/7 — first engaged coverage (v2.3 no-userdata subset) |
| fused_post_chain | excluded (enablement blocked) | armed + honest 0/7 non-engagement (LINEAR∩AE=∅); A/A control leg |
| all5 geomean ratio | 0.9816 (incl. wrong-pixel tonemap cells) | 0.9942 (pixel-correct-subset only) |
| per-pass GPU µs | none (gpu_timestamps_available=false) | full bucket attribution, 2000 frames/scene |
| Amdahl | qualitative ("a few percent, structurally unreachable") | quantified: ceiling geomean 1.0669x |

The all5 movement 0.9816→0.9942 should NOT be read as an improvement claim:
the wrong-pixel FILMIC tonemap cells (which were W5's worst: 0.907/0.924 on
the noisiest scenes) are simply no longer part of the engaged set, and the
workload changed. It is reported as the honest consequence of the correctness
hardening.

## 7. Anomalies and honest-record items

1. **Driver interruption** (§ Setup): first campaign driver died silently after
   legs 1-2; legs 3-7 re-run to completion. All archived summaries are from
   completed runs only; leg ordering (3 baselines before rurix legs) preserved.
2. **fused_post_chain leg invalid as a fused measurement** (0/7 engagement,
   structural LINEAR∩AE=∅) — recorded as an all5 A/A control instead; the
   fused kernel remains perf-unmeasured on this bench.
3. **particles_copy engaged but its bucket covers all 15 emitters**, of which
   only the 3 no-userdata emitters are replaced; the 0-delta is the honest
   composite (replaced slice indistinguishable at tick resolution).
4. **Baseline noise this round**: clustered_lights 4.9% and material_variants
   5.7% run-to-run fps spread (other five scenes ≤0.7%); single-run rurix
   cells on those two scenes carry that uncertainty. clustered's 1.0155 all5
   cell is inside its own noise band.
5. **ts legs cost FPS** (--gpu-profile: e.g. clustered baseline-ts 231.4 vs
   clean 239.3): by design they never enter a ratio; their role is bucket
   attribution only, and both ts legs carry the identical overhead.
6. **gpu_culling never armed** (mechanism blocked with device removal at
   enablement — arming it in a perf leg would be dishonest); the v2.3 indirect
   MultiMesh target idles in many_mesh until that pass is unblocked.
7. p95 ratio quantization (§3) and GPU-timestamp tick snapping (§4) both mean
   small deltas in those tables are step artifacts, not measurements.

## 8. Measurement validity + CR self-check

1. Everything `measured_local`, one machine (RTX 4070 Ti, template_debug,
   D3D12 Forward+, 1080p), single full run per rurix leg, 3 runs baseline.
   **No performance claim; no default-enable proposal.** The numbers quantify
   a ceiling, not a benefit.
2. Engagement identical across all rurix legs and sourced from the one-shot
   `--verbose` marker (Wave 5's non-pollution verification carries over: the
   rd_native hot path is stdout-clean; verbose output is init/shutdown-bound).
3. All 7 legs: failure_count=0, zero failure markers, allowlisted warnings
   only; single provenance tuple (exe sha `fc41853b…`, dll sha `47910fe7…`,
   patch `0001-0029+0040-0048`, run_mode full) across every archived summary.
4. CR self-check: every file in `bench/rd_native_final_20260713/` (7 archived
   summaries, 2 matrices, analyze_final.py, analysis_output.txt,
   campaign_progress.log, this report) verified CR=0 byte-level after the one
   normalization noted below; nothing in this campaign is committed to git.
   (analysis_output.txt was initially written CRLF by Windows-Python stdout
   redirection and normalized to LF; re-verified 0 CR bytes.)
