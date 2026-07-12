# GRX-017 material_sorting — telemetry-only slice (patch 0026)

## What this is

GRX-017's FIRST slice per `milestones/grx/GRX_PLAN.md` ("若 telemetry 不足,先补
telemetry"): a **measured sampling** of how Godot's existing Forward+
opaque-list material/shader key sort behaves on the `material_variants` bench
scene, collected through the patch 0026 telemetry instrumentation. It is NOT
an evidence-gate document: it feeds no CI gate, flips no manifest field,
builds no `ci/grx_gates/` module, and makes **no FPS, p95, GPU-timestamp,
cache-miss, or performance claim**.

## Instrumentation (patch `0026-rurix-accel-material-sorting-telemetry.patch`)

- Default-false setting `rendering/rurix_accel/telemetry/material_sorting/enabled`
  (the shipping config never enables it; enabled here via a local
  `override.cfg`).
- `RenderForwardClustered::_render_scene`: `render_list[RENDER_LIST_OPAQUE]
  .sort_by_key()` timed with `OS::get_ticks_usec` (the sort itself is
  byte-identical either way).
- `_render_list_template`: O(1) counters on the EXISTING PSO-rebind branch and
  the EXISTING `material_uniform_set` switch branch, compile-time gated to
  `PASS_MODE_COLOR` (no new traversal; depth/shadow/SDF passes uncounted).
- One verbose-only line every 30 frames (`RXGD_MATERIAL_SORTING_TELEMETRY
  frame=.. sort_usec=.. element_count=.. pso_switches=..
  material_uniform_set_switches=.. interval_frames=30`), switch counters reset
  at emission. The switch counts cover the interval window of COLOR render
  lists that FOLLOW each sort (up to one frame of trailing skew — declared).
  Requires `--verbose`; zero stdout otherwise.

## Sample (`telemetry_sample.json`, measured 2026-07-12)

`material_variants` scene, 1280x720 quick-smoke run (30 warmup + 120 sample
frames) on the `0001..0026` scratch Godot (Windows D3D12 Forward+, NVIDIA
GeForce RTX 4070 Ti; exe sha256 pinned in the JSON):

| metric | value |
| --- | --- |
| opaque `element_count` at sort | 17049 |
| `sort_usec` min / mean / max (5 samples) | 961 / 1005 / 1062 |
| PSO rebinds per frame (mean, COLOR lists) | ~32 |
| material_uniform_set switches per frame (mean, COLOR lists) | ~2049 |

Reading (observation only, no claim): the existing sort already amortizes PSO
rebinds to ~32/frame over 17k opaque elements, while material uniform-set
switches remain ~2k/frame — that switch pressure is the quantity a future
GRX-017 sorting/batching slice would target, and this sample is the measured
baseline for judging whether such a slice is worth pursuing.

## Status

- GRX-017 decision: **telemetry-only for now** (`keep_disabled` /
  telemetry-first per GRX_PLAN); no gate module, no kernel, no bridge call, no
  real-pass triple, no default enablement.
- Patch 0026 defaults keep every tracked smoke leg byte-identical (the setting
  is false everywhere except this sampling run's local `override.cfg`).
