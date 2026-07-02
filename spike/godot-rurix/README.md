# Godot Rurix Acceleration Spike

> Status (2026-07-02): GRX-009 preparation is complete, gated implementation
> segment 1 (disabled/fallback scaffold) is delivered, and segment 2 (Godot
> core call-site fallback wiring) is now landed via patch 0003. Segment 3a
> offline compile evidence has now started with a real kernel/package draft and
> an explicit compile-failure blocker record: the current DXIL minimal compute
> path still rejects the parameterized luminance reduction kernel draft, so no
> DXIL/root signature/descriptor layout artifact set exists yet. The bridge
> still requests fallback for `luminance_reduction`, the
> per-pass module setting still defaults to disabled, and the wired core call
> site still falls back to native Godot in practice. Full baseline measurement,
> any runtime-usable Rurix acceleration pass, real visual diff evidence,
> measured fallback telemetry, and any performance improvement remain
> unfinished. Runner evidence is still quick-smoke only and is not eligible
> for strict close-out.

This directory holds tracked assets for the ignored Godot 4.7-dev D3D12
Forward+ snapshot in `external/godot-master`.

- `patches/` contains Godot-side module patches, applied as a stack:
  `0001-rurix-accel-module-scaffold.patch` (module scaffold) and
  `0002-rurix-accel-luminance-pass-gate.patch` (GRX-009 default-disabled
  luminance gate; module-scoped) plus
  `0003-rurix-accel-luminance-core-callsite-wiring.patch` (GRX-009 segment 2
  Auto Exposure core call-site fallback wiring).
- `bench/` contains benchmark manifests and gate scripts.
- `passes/` contains per-pass contracts and manifests (GRX-009+).

The Godot source tree itself is intentionally ignored so the Rurix repository
does not vendor a large external engine checkout.

## GRX-003 Load Smoke Design

The first GRX.1 runtime validation is load/fallback behavior, not acceleration.

- Present DLL path:
  - Build `rurix_godot.dll` from `src/rurix-godot`.
  - Build Godot with `module_rurix_accel_enabled=yes`.
  - Launch a minimal D3D12 + Forward+ Godot project with `rendering/rurix_accel/enabled=true`.
  - Expect a log line equivalent to `RurixAccel: D3D12 Forward+ bridge session ready.`

- Missing DLL fallback:
  - Point `rendering/rurix_accel/dll_path` at a missing file, or remove the DLL from the Godot run directory.
  - Launch the same minimal D3D12 + Forward+ project.
  - Expect `RurixAccel: rurix_godot.dll not found; Godot fallback path remains active.`
  - The process must not crash.

- ABI mismatch fallback:
  - Launch the same minimal D3D12 + Forward+ project with an incompatible DLL.
  - Expect `RurixAccel: ABI version mismatch; disabling acceleration.` or `RurixAccel: invalid rurix_godot.dll ABI; disabling acceleration.`
  - The process must not crash and must keep fallback active.

Fresh local evidence now exists for both the present-DLL and missing-DLL load/fallback paths, and the current toolchain probe already reports `build_artifacts_ready=true`, `load_smoke_ready=true`. `GRX-004` is complete with fresh per-scene smoke evidence under `target/grx/`. `GRX-005` (benchmark runner), `GRX-006` (baseline schema / perf gate), `GRX-007` (visual diff scaffold), and `GRX-008` (fallback telemetry scaffold) are all delivered and hardened. `GRX-009` preparation is complete, segment 1 of its gated implementation is delivered, and segment 2 core call-site fallback wiring is now landed (see the GRX-009 section below). The next step is segment 3a offline compile evidence only; the actual accelerated pass, real visual diff evidence, measured fallback telemetry, and any performance improvement still do not exist and are not claimed.

## GRX-004 Skeleton Hardening

`GRX-004` only covers the generated 7-scene benchmark project skeleton plus
per-scene Godot load smoke under `target/grx/`.

- `GRX-004` is only complete when `target/grx/godot_bench_project_smoke_summary.json`
  reports `scene_count=7` and all seven scene results are `pass`.
- Each benchmark scene must have independent Godot load evidence and an
  independent log under `target/grx/godot-bench-project/logs/`.
- `ERROR: Could not load global script cache.` may be recorded as an allowlisted
  warning, but it must remain visible in the smoke summary and logs.
- Any other Godot `ERROR`, any `SCRIPT ERROR`, parser/parse failure, or failed
  script/resource load keeps `GRX-004` incomplete.
- `GRX-005` runner work, baseline/perf JSON, visual diff, and any actual
  benchmark or performance claims remain unfinished and must not be stated as
  complete here.

## GRX-005 Runner

`GRX-005` was delivered and hardened after the fresh `GRX-004` per-scene smoke
close-out (see the completed history below).

- The runner must use the fixed full-mode manifest parameters:
  - `warmup_frames=300`
  - `sample_frames=2000`
  - `vsync=false`
  - `resolution=1920x1080`
- The runner must execute the same seven benchmark scenes in manifest order and
  emit raw frame sample JSON per scene under `target/grx/godot-bench-runs/`.
- Raw JSON must include CPU frame samples plus derived `avg_fps` and
  `p95_frame_time_ms`.
- GPU timestamps are not required for `GRX-005`, but the artifacts must
  explicitly record `gpu_timestamps_available=false`; no placeholder values may
  be presented as measured timestamps.
- The runner scans Godot log output for failure markers, aligned with
  `bench_project_smoke.py`: the known `ERROR: Could not load global script
  cache.` (with its `at: ProjectSettings::get_global_class_list` context line)
  is recorded as an allowlisted warning, while any other `ERROR:`,
  `SCRIPT ERROR:`, `Parser Error:`, `Parse Error:`, `Failed loading resource:`,
  or `Failed loading script` marks that scene as failed. Each scene result
  records `failure_markers` / `warnings`, and the runner summary adds an
  aggregate `warning_count`. The runner does not pass `--verbose`, so it does
  not reuse the smoke "missing Loading resource evidence" rule.
- `GRX-005` only delivers raw runner evidence and a runner summary. It does not
  complete baseline schema/perf gate work, visual diff, any actual acceleration
  pass, or any performance improvement claim.

## GRX-006 Baseline Schema / Perf Gate

`GRX-006` defines the baseline/perf evidence JSON formats and the strict perf
gate input format. It delivers format/gate infrastructure only; no full baseline
measurement, acceleration pass, visual diff, or performance improvement is
claimed.

- `bench/schemas/baseline_evidence.schema.json` describes a single measured_local
  run (baseline or rurix): seven fixed scenes in order, `target_backend`
  `Godot 4.7-dev Windows D3D12 Forward+`, resolution `1920x1080`, `vsync=false`,
  `evidence_level=measured_local`, and a traceable `raw_artifact_path` per scene.
  `quick_smoke` documents are smoke evidence only and are NOT eligible as strict
  perf gate input.
- `bench/schemas/perf_gate_input.schema.json` describes strict close-out input:
  full mode with `warmup_frames=300` / `sample_frames=2000`, plus per-scene
  baseline/rurix FPS and p95 and both raw artifact paths.
- `bench/perf_gate.py` supports `--kind {perf_gate,baseline}`, `--strict`, and
  `--validate-only`. It separates `FORMAT FAIL` (format/schema) from `PERF FAIL`
  (thresholds). Strict mode rejects any SKIP/estimated markers, `quick_smoke` or
  non-full run modes, missing scenes, and missing raw artifact paths. The three
  close-out thresholds remain geomean FPS ratio >= 1.5, mean p95 reduction >=
  0.30, and single-scene FPS ratio >= 0.95.
- `bench/samples/baseline_smoke_example.json` is a smoke-only baseline document
  used to exercise the schema reader; it must not be used for strict perf gate.
- `bench/samples/perf_gate_failing_example.json` is a format-correct strict perf
  gate input that intentionally FAILS the thresholds (rurix FPS near baseline);
  it does not fabricate any achieved 1.5x speedup.

### GRX-006 Hardening

The GRX-006 format/gate infrastructure was hardened to close three gaps. It is
still infrastructure only; no full baseline, acceleration pass, real visual
diff, or performance improvement is claimed.

- Strict forbidden-marker detection now uses a word-boundary regex (underscore
  treated as a word character) instead of prefix matching. Standalone
  `skip` / `skipped` / `estimated` markers embedded anywhere in a strict
  document now FORMAT FAIL, covering `SKIP: missing`, `skip-reason`,
  `status=SKIP`, `estimated:true`, and `estimated local`, while fragments like
  `spike`, `quick_smoke`, and ordinary path separators are not flagged.
- The baseline reader now validates each scene `sample_count` as a positive
  integer and requires `sample_count == sample_frames`, matching
  `baseline_evidence.schema.json`.
- Strict perf gate input now validates that, when `thresholds` is present, all
  three values equal the fixed close-out thresholds
  (`geomean_fps_ratio_min=1.5`, `p95_frame_time_reduction_min=0.3`,
  `single_scene_fps_ratio_min=0.95`); any override FORMAT FAILs.
- Two reproducible red-test samples were added:
  `bench/samples/perf_gate_forbidden_skip_example.json` (contains `SKIP: missing`
  and must FORMAT FAIL under `--strict`) and
  `bench/samples/baseline_missing_sample_count_example.json` (missing one scene
  `sample_count` and must FORMAT FAIL under `--kind baseline --validate-only`).
- `ci/godot_rurix_toolchain_probe.py` now reports `grx006_schema_ready` and, when
  the GRX-006 schemas/samples parse, advances `next_action` to
  `start_grx007_visual_diff_scaffold`.

## GRX-007 Visual Diff Scaffold

`GRX-007` delivers visual capture/diff scaffold only. No frames are captured, no
diff is produced, and no visual verification is claimed at this stage.

- Scaffold assets: `bench/capture_reference_frames.py`, `bench/visual_diff.py`,
  `bench/schemas/visual_diff_evidence.schema.json`, and
  `bench/samples/visual_diff_placeholder.json`.
- The schema covers the seven fixed scenes in order, each with at least one
  capture frame. Each frame has `status` in `{pass, skip}`.
- LDR support: `visual_diff.py` computes a per-channel absolute diff (per-channel
  max/mean) only when both `reference_frame_path` and `candidate_frame_path`
  point at real frame files on disk. Otherwise it reports SKIP.
- HDR/temporal fields (`ssim`, `psnr`, `temporal_stability`, ...) are declared in
  the schema but left null; no HDR/temporal result is produced or fabricated.
- Build-stage SKIP is allowed, but every skipped capture frame must record a
  concrete reason: `missing capture backend`, `missing Godot full run`, or
  `missing frame artifact`.
- `bench/samples/visual_diff_placeholder.json` marks all seven scenes as SKIP.
  Do not state that visual verification passed unless real reference AND
  candidate frame files and a computed diff JSON exist. `visual_diff.py` prints a
  `SCAFFOLD ... visual verification is NOT done` line whenever no real diff is
  computed.

### GRX-007 Hardening

The GRX-007 visual diff scaffold was hardened to close four gaps. It is still
scaffold/infrastructure only; no real frames are captured and no visual
verification is claimed.

- A `status=pass` capture frame now requires an `ldr_diff` object that carries
  both `per_channel_max_abs` and `per_channel_mean_abs`, each a length-3 array
  of non-negative numbers `[r, g, b]`. Missing fields or wrong shapes FORMAT
  FAIL (`visual_diff.py` and `visual_diff_evidence.schema.json` enforce this).
- A `status=skip` capture frame now must not carry fabricated frame paths or
  diff numbers: `reference_frame_path`, `candidate_frame_path`, `ldr_diff`,
  `hdr_diff`, and `temporal_diff` must be null or absent. A skip frame carrying
  any non-null value FORMAT FAILs, so a skip cannot smuggle in a fake diff
  (`visual_diff.py` and the schema `else` branch enforce this).
- Non `--validate-only` runs now read the real reference/candidate frame files
  and compare the computed LDR per-channel diff against the recorded `ldr_diff`.
  Any mismatch prints `DIFF FAIL` and exits non-zero, so fabricated diff numbers
  cannot pass silently.
- `visual_diff.py --write-output <path>` generates an evidence JSON with the
  computed `ldr_diff` written back, instead of only printing. This is the
  generation mode; the default mode remains compare-and-DIFF-FAIL.
- Reproducible red/green samples were added:
  `bench/samples/visual_diff_pass_missing_ldr_example.json` (status=pass with a
  missing `ldr_diff` field; FORMAT FAIL under `--validate-only`),
  `bench/samples/visual_diff_skip_with_fake_ldr_example.json` (status=skip with a
  fabricated `ldr_diff`; FORMAT FAIL under `--validate-only`),
  `bench/samples/visual_diff_mismatch_example.json` (status=pass with real frame
  fixtures but wrong recorded `ldr_diff`; DIFF FAIL), and
  `bench/samples/visual_diff_ldr_pass_example.json` (status=pass whose recorded
  `ldr_diff` matches the computed per-channel absolute diff of the tiny
  fixtures under `bench/samples/frames/`). These fixtures exercise the LDR diff
  comparison path only and are not real captured-scene visual verification.

## GRX-008 Fallback Telemetry Scaffold

`GRX-008` delivers fallback telemetry scaffold only. No acceleration pass is
implemented, no telemetry is measured, and the presence of a pass entry does NOT
mean that pass has been wired up or that a real fallback occurred.

- Scaffold assets: `bench/schemas/fallback_telemetry.schema.json`,
  `bench/samples/fallback_telemetry_placeholder.json`, and
  `bench/fallback_telemetry.py`.
- The `fallback_reason` enum covers exactly five values: `compile_failed`,
  `validation_failed`, `unsupported_device`, `visual_diff_failed`, and
  `manual_disabled`.
- Each pass entry must record `pass_id`, `enable_state` (`enabled`/`disabled`),
  `fallback_reason`, `godot_fallback_active` (boolean), plus telemetry
  `telemetry_timestamp` and `telemetry_frame`. At the scaffold stage the
  timestamp/frame may be `null` placeholders, but the fields must be present.
- The placeholder document is entirely scaffold/SKIP: every pass is disabled,
  covers one reason enum value, and its `note` states that it does not represent
  any pass being wired up or any real fallback having occurred.
  `fallback_telemetry.py` prints a `SCAFFOLD ... does NOT mean any pass has been
  wired up` line on non `--validate-only` runs.

### GRX-008 Hardening

The GRX-008 fallback telemetry scaffold was hardened so scaffold and
full/measured_local documents are validated differently. It is still
scaffold/infrastructure only; no acceleration pass is implemented, no telemetry
is measured, and the placeholder document remains scaffold-only.

- Scaffold documents (`run_mode=scaffold` / `evidence_level=scaffold`) may leave
  `telemetry_timestamp` and `telemetry_frame` null, but every pass must be
  `enable_state=disabled` and `godot_fallback_active=true`.
- Full runs (`run_mode=full` or `evidence_level=measured_local`) require a
  non-empty `telemetry_timestamp` and a non-negative integer `telemetry_frame`;
  a null timestamp/frame FORMAT FAILs.
- `measured_local` documents may not use a `pass_id` starting with
  `placeholder_`.
- Reproducible red samples were added:
  `bench/samples/fallback_telemetry_full_null_timestamp_example.json`
  (`run_mode=full` / `measured_local` but null timestamp/frame; FORMAT FAIL under
  `--validate-only`) and
  `bench/samples/fallback_telemetry_scaffold_fallback_inactive_example.json`
  (scaffold but `godot_fallback_active=false`; FORMAT FAIL under
  `--validate-only`). The existing
  `bench/samples/fallback_telemetry_placeholder.json` still FORMAT PASSes and is
  explicitly not actual fallback telemetry.

## GRX-009 Luminance Reduction Pass — Gated Implementation Segment 2 / Segment 3a Blocked

`GRX-009` preparation (`passes/luminance_reduction/PASS_CONTRACT.md` +
`pass_manifest.json`) is complete, segment 1 delivered verifiable
disabled/fallback wiring, and segment 2 now wires the Godot core Auto Exposure
call site into the same opt-in fallback gate. The pass itself is still NOT
implemented.

- Rust bridge gate (`src/rurix-godot`, C ABI v1 unchanged): the new
  `LuminanceReductionGate` defaults to disabled, `request_enable` always fails
  with `compile_failed` (no compiled Rurix DXIL luminance kernel exists), and
  `rxgd_record_pass` always returns `RXGD_STATUS_FALLBACK` for
  `RXGD_PASS_LUMINANCE_REDUCTION`, incrementing `fallback_passes` and no longer
  recording the previous placeholder estimated GPU time. Red/green coverage
  lives in `cargo test -p rurix-godot`.
- Godot module patch: `patches/0002-rurix-accel-luminance-pass-gate.patch` is
  stacked on 0001 and only touches `modules/rurix_accel/*`. It adds the
  default-false project setting
  `rendering/rurix_accel/passes/luminance_reduction/enabled` and
  `try_record_luminance_reduction()`, which returns false (keep the native
  Godot luminance path) when the setting is off, the bridge session or record
  symbol is missing, or the bridge returns any non-OK status.
- Godot core call-site patch: `patches/0003-rurix-accel-luminance-core-callsite-wiring.patch`
  is stacked on 0001+0002 and touches the Auto Exposure producer call site in
  `renderer_scene_render_rd.cpp` plus the required hook declarations. It calls
  `D3D12Hooks::get_singleton()->try_record_luminance_reduction()` before the
  native `luminance_reduction(...)` invocation and skips the native path only
  when the full module/session/setting/bridge gate returns OK. Because the
  bridge still returns `RXGD_STATUS_FALLBACK` for
  `RXGD_PASS_LUMINANCE_REDUCTION`, the native Godot luminance path remains
  active in practice.
- Patch stack checking: `ci/godot_rurix_bridge_smoke.py` now validates the
  four legal tree states (base, 0001-only, 0001+0002, 0001+0002+0003) and
  fails on any drift.
- Disabled-state telemetry record:
  `bench/samples/fallback_telemetry_luminance_disabled_example.json` is a
  scaffold-level document (`manual_disabled`, `godot_fallback_active=true`)
  that records the disabled/fallback wiring state. It is NOT measured
  telemetry and no real in-engine fallback event was captured.
- Callsite-wired disabled-state telemetry record:
  `bench/samples/fallback_telemetry_luminance_callsite_wired_disabled_example.json`
  is a scaffold-level document for the segment 2 wired-but-still-disabled
  state. It is NOT measured telemetry and no real in-engine fallback event was
  captured.
- Probe hardening: `grx009_prep_ready` in `ci/godot_rurix_toolchain_probe.py`
  remains preparation-only, while `grx009_segment1_ready` and
  `grx009_segment2_ready` now distinguish the segment 1 and segment 2 delivery
  states and advance `next_action` accordingly. `grx009_segment2_ready` now
  also requires the callsite-wired scaffold telemetry sample to pass
  `fallback_telemetry.py --validate-only` and the shared patch-stack check to
  report the real `0001+0002+0003` state.
- Segment 3a compile evidence: `passes/luminance_reduction/rurix.toml`,
  `src/lib.rx`, `compile_offline.py`, `compile_evidence.schema.json`, and
  `offline_compile_evidence.json` exist, but segment 3a is blocked. Current
  artifacts describe only files produced by the latest compile attempt; any
  `dxil_ir_text` / `entry_shell_only` IR is debug evidence, not a validated DXIL
  container or compile-ready artifact. The current DXIL compute path rejects the
  non-trivial luminance body because real body lowering is not implemented.
  Runtime behavior stays fallback-only until real DXIL container emission, real
  body lowering, visual diff, fallback gate, and measured telemetry are all
  complete.

Still unfinished for GRX-009 (strict evidence gaps): a real DXIL container,
real compute body lowering, a real Rurix GPU luminance pass, real visual diff
evidence (all visual evidence remains SKIP/placeholder), measured_local
fallback telemetry, full baseline/Rurix comparison data, and any performance
data or improvement claim.
