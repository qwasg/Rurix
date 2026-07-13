# GRX-025 per-pass default-enable bisection (2026-07-13, measured_local, no performance claim)

Phase-1 of the honest-ceiling close-out. The terminal rd_native campaign
(`../rd_native_final_20260713/`) quantified the Amdahl hard ceiling (1.0669x
geomean) and made NO default-enable proposal. GRX-025 asks the narrower,
gate-shaped question the close-out actually needs: **on the scenes where each
rd_native pass ENGAGES, is its single-pass avg_fps ratio (vs the v2.3 baseline
median) at or above 0.95x?** — i.e. does turning the pass on by default cost
less than 5% where it fires. This is a DEFAULT-ENABLE decision input, not a
speedup; every number is `measured_local` on one machine and **no performance
claim of any kind is made**.

## Setup

- exe: `target/grx/godot-scratch-rb4/bin/godot.windows.template_debug.x86_64.console.exe`,
  console-exe sha256 `fc41853b…`, dll sha256 `47910fe7…`, patch stack
  `0001-0029+0040-0048` — **byte-identical to the terminal campaign's baseline**
  (verified this session), so each single-pass leg is directly comparable to
  that campaign's archived `baseline_run{1,2,3}.json`.
- baseline: reused verbatim from `../rd_native_final_20260713/baseline_run{1,2,3}.json`
  (v2.3 workload, 3 full runs, per-scene median). **Not re-run here** — one
  v2.3 baseline of record; `analyze_grx025.py` reads it from that directory.
- rurix legs: five single-pass matrices (`matrices/rd_native_<pass>.json`, one
  pass at `backend=2` each, byte-identical to the Wave-5 checkpoint templates),
  one `full` run each (300 warmup + 2000 sample, 1920x1080, vsync off, D3D12
  Forward+), strictly serial, machine quiet/exclusive. `RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64`.
- confirmatory re-runs: `tonemap` and `taa_resolve` were re-run once (`_r2`) —
  each engages on the round's noisiest scenes (tonemap on clustered 5.08% +
  material 6.00%; taa on many_mesh 2.17%), so their per-scene ratios are
  MEDIANED over 2 legs to blunt single-run noise. The other three passes engage
  only on low-noise scenes (post_fx 0.65%, mixed 0.69%, volumetric 0.26%,
  particles 1.03%) and stay single-leg.
- driver: `run_grx025_campaign.py`; analysis: `analyze_grx025.py` (reproduces
  every table below; raw stdout archived as `analysis_output.txt`). Progress:
  `campaign_progress.log`.
- every leg: `failure_count=0`, `warning_count=14` (the known allowlisted
  global-script-cache pair, 2 lines × 7 scenes), zero failure markers.

## 1. Baseline v2.3 noise floor (this-round run-to-run avg_fps spread)

Computed from the three reused baseline runs as `(max-min)/min`:

| scene | median avg_fps | run-to-run spread | note |
|---|---|---|---|
| clustered_lights | 239.33 | **5.08%** | noisy |
| many_mesh_instances | 220.63 | **2.17%** | noisy (understated in the terminal report) |
| material_variants | 249.49 | **6.00%** | noisy |
| post_fx_chain | 178.10 | 0.65% | |
| volumetric_fog | 215.16 | 0.26% | |
| particles | 175.45 | 1.03% | |
| mixed_forward_plus | 222.63 | 0.69% | |

**Honest-record correction:** the terminal report §7 said "other five scenes
≤0.7%". Recomputing `(max-min)/min` on its own archived runs, **many_mesh is
2.17% and particles 1.03%** — the "≤0.7%" applies only to post_fx / volumetric /
mixed. This matters because two passes' worst engaged cells land on many_mesh
(2.17%); the verdicts below fold that band in.

## 2. Engagement (one-shot rd_native active marker) — matches pre-registration EXACTLY

`pass_engagement_source = rd_native_active_marker` on every rurix-leg scene.
Observed engagement is identical to the terminal campaign's all5 leg and to the
pre-registered expectation:

| pass | engaged scenes | count | pre-registered | match |
|---|---|---|---|---|
| tonemap | clustered_lights, many_mesh_instances, material_variants, particles | 4/7 (LINEAR only) | 4/7 | yes |
| ssao_blur | post_fx_chain, mixed_forward_plus | 2/7 | 2/7 | yes |
| taa_resolve | many_mesh_instances, mixed_forward_plus | 2/7 | 2/7 | yes |
| particles_copy | particles | 1/7 | 1/7 | yes |
| cluster_store | clustered_lights, post_fx_chain, volumetric_fog, mixed_forward_plus | 4/7 | 4/7 | yes |

The 0040 LINEAR mode guard fail-closes tonemap on the three FILMIC scenes
(post_fx, volumetric, mixed); those cells are honestly non-engaged (native
vs native).

## 3. Per-pass avg_fps ratio (single-pass rurix / baseline median; * = engaged)

Median over available legs (tonemap/taa = median-of-2; others single). `noise%`
= that scene's baseline spread from §1.

| scene | tonemap | ssao_blur | taa_resolve | particles_copy | cluster_store | noise% |
|---|---|---|---|---|---|---|
| clustered_lights | **1.0080** | 1.0228 | 1.0220 | 1.0213 | **1.0156** | 5.08 |
| many_mesh_instances | **0.9888** | 0.9912 | **0.9923** | 0.9877 | 0.9858 | 2.17 |
| material_variants | **0.9912** | 0.9791 | 0.9890 | 0.9741 | 1.0136 | 6.00 |
| post_fx_chain | 0.9965 | **0.9996** | 0.9966 | 0.9918 | **0.9901** | 0.65 |
| volumetric_fog | 0.9959 | 1.0202 | 0.9967 | 1.0170 | **1.0030** | 0.26 |
| particles | **0.9958** | 0.9978 | 0.9981 | **0.9945** | 0.9953 | 1.03 |
| mixed_forward_plus | 0.9904 | **0.9904** | **0.9890** | 0.9881 | **0.9887** | 0.69 |

(Bold = engaged cell. Non-engaged cells are native-vs-native and drift inside
their scene noise — e.g. cluster_store's non-engaged material 1.0136 and
clustered's engaged 1.0156 both sit inside 5-6% baseline noise, not savings.)

## 4. GRX-025 DECISION TABLE (>=0.95 engaged-geomean gate, noise-aware)

| pass | legs | engaged# | engaged geomean | worst engaged scene | worst ratio | worst noise | verdict |
|---|---|---|---|---|---|---|---|
| tonemap | 2 | 4 | **0.9959** | many_mesh_instances | 0.9888 | 2.17% | **pass** |
| ssao_blur | 1 | 2 | **0.9950** | mixed_forward_plus | 0.9904 | 0.69% | **pass** |
| taa_resolve | 2 | 2 | **0.9906** | mixed_forward_plus | 0.9890 | 0.69% | **pass** |
| particles_copy | 1 | 1 | **0.9945** | particles | 0.9945 | 1.03% | **pass** |
| cluster_store | 1 | 4 | **0.9993** | mixed_forward_plus | 0.9887 | 0.69% | **pass** |

Verdict rule (`analyze_grx025.py:verdict_for`): **pass** = engaged geomean
>= 0.95 AND the worst engaged scene is either >= 0.95 or its shortfall is
inside that scene's noise band; **fail** = geomean below 0.95 by more than the
worst scene's noise; **inconclusive** = geomean straddles 0.95 within noise
(re-run). No pass is on the edge — the smallest engaged geomean is
taa_resolve 0.9906 and the lowest engaged cell across all passes is
mixed_forward_plus taa 0.9890, both comfortably above 0.95 even after
subtracting their (small, 0.69%) scene noise.

**All five rd_native passes clear the >=0.95 default-enable gate on their
engaged scenes.** Every engaged geomean sits in [0.9906, 0.9993]; the cost of
enabling any single pass where it fires is <1.5% geomean, well inside the 5%
budget the gate encodes.

### Notes per pass

- **tonemap** (worst risk going in): single-run geomean was 0.9911, pulled down
  by the two 5-6%-noise scenes it uniquely straddles (clustered, material). The
  median-of-2 lifts it to **0.9959** — the single run was noise, not cost. Every
  timed cell is the LINEAR-subset only (FILMIC fail-closed), so this is a
  pixel-correct-subset verdict.
- **taa_resolve** (the terminal report's dominant per-pass GPU regression,
  +100-114 µs): median-of-2 confirms **0.9906**, exactly the 0.97-0.98+ band the
  campaign predicted. The +100 µs intrinsic cost is real (final report §4) but
  is <1.1% of avg_fps on its two engaged scenes, so it still clears 0.95. Data,
  not hand-waving: TAA passes.
- **particles_copy** — FIRST default-enable read: **0.9945** on the one scene
  (particles) where the v2.3 no-userdata emitter subset engages; 0-delta GPU µs
  in the final report §4. Single engaged scene → the geomean IS that cell;
  low-noise (1.03%) so the single leg is adequate.
- **cluster_store** highest at **0.9993** (near-A/A); **ssao_blur 0.9950**.

## 5. p95 (engaged geomean, quantization-caveated)

tonemap 1.0142 / ssao_blur 1.0048 / taa_resolve 1.0170 / particles_copy 1.0000 /
cluster_store 1.0254. p95 raw frame times sit on discrete 1/fps buckets, so a
1.02-1.03 engaged-p95 geomean is one-to-two bucket steps, not a measured tail
regression of that magnitude (same caveat as the terminal report §3). p95 is
reported for completeness; the gate is on avg_fps.

## 6. Honest-record / caliber limitations

1. `measured_local`, single machine (RTX 4070 Ti, template_debug, D3D12
   Forward+, 1080p). **No performance claim; no speedup asserted.** The 0.95
   gate measures default-enable COST, not benefit.
2. Ratios are single-full-run (tonemap/taa median-of-2) vs a median-of-3
   baseline. Engaged cells on clustered (5.08%) and material (6.00%) carry that
   baseline noise; the verdict folds each scene's band in, and the two passes
   touching those scenes were re-run to median.
3. `particles_copy` has a single engaged scene by design (userdata subset); its
   verdict rests on one low-noise cell + the terminal §4 0-delta attribution +
   the S2 ~1-ULP container parity, not on a multi-scene geomean.
4. p95 quantization (§5). Engagement is the one-shot `--verbose` marker (the
   rd_native hot path is stdout-clean; carried over from Wave 5).
5. `fused_post_chain` and `gpu_culling` are NOT part of this bisection: fused
   engages on no bench scene (LINEAR∩AE=∅, final report §2) and gpu_culling is
   mechanism-blocked (device removal at enablement) — arming either would be
   dishonest. GRX-025 covers the five demonstrably-engageable passes.

## 7. CR self-check

Every file authored this session under `bench/grx025_default_enable_20260713/`
(5 matrices, 5 base + 2 re-run summaries, `run_grx025_campaign.py`,
`analyze_grx025.py`, `analysis_output.txt`, `campaign_progress.log`, this
report) verified `CR=0` byte-level; archived summaries are the runner's own LF
writes copied byte-for-byte. Nothing in this campaign is committed to git.
