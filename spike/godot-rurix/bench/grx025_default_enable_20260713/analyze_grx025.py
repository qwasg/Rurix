#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-025 per-pass default-enable bisection analysis (2026-07-13).

measured_local, single machine (RTX 4070 Ti, template_debug, D3D12 Forward+,
1080p). NO performance claim. Produces the GRX-025 DECISION INPUT: for each of
the five rd_native replaceable passes, the single-pass avg_fps ratio (vs the
terminal v2.3 baseline median) on the scenes where the pass ENGAGES, an
engaged-only geomean, the worst engaged scene, and a >=0.95 verdict
(pass | fail | inconclusive) that folds in this round's per-scene baseline
noise floor.

Baseline is the SINGLE SOURCE OF TRUTH from the terminal rd_native campaign:
`../rd_native_final_20260713/baseline_run{1,2,3}.json` (same rb4 exe / v2.3
workload / patch stack as these single-pass legs; verified by identical
godot_exe_sha256). Baselines are NOT re-run here (task constraint); they are
read from that directory so there is exactly one v2.3 baseline of record.

Rurix single-pass legs are `rurix_<pass>[_<tag>].json` in THIS directory; when
a pass carries noise re-runs (extra tags) their per-scene avg_fps/p95 are
MEDIANED before the ratio is taken.
"""
from __future__ import annotations

import json
import math
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
FINAL_DIR = HERE.parent / "rd_native_final_20260713"

SCENES = [
    "clustered_lights",
    "many_mesh_instances",
    "material_variants",
    "post_fx_chain",
    "volumetric_fog",
    "particles",
    "mixed_forward_plus",
]
PASSES = ["tonemap", "ssao_blur", "taa_resolve", "particles_copy", "cluster_store"]

# Per-scene baseline run-to-run avg_fps spread for THIS v2.3 round (computed
# from the three baseline runs; printed live in main() and used as the noise
# band that a single-run rurix cell inherits as +/- half-spread).
GATE = 0.95


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def scene_map(summary: dict) -> dict[str, dict]:
    return {r["scene_name"]: r for r in summary.get("per_scene_results", [])}


def geomean(vals: list[float]) -> float:
    vals = [v for v in vals if v and v > 0]
    if not vals:
        return float("nan")
    return math.exp(sum(math.log(v) for v in vals) / len(vals))


def active(res: dict, pass_name: str) -> bool | None:
    eng = res.get("pass_engagement") or {}
    e = eng.get(pass_name)
    if isinstance(e, dict) and "rd_native_active" in e:
        return bool(e["rd_native_active"])
    return None


def baseline_medians() -> tuple[dict[str, dict], dict[str, float]]:
    """Return per-scene {avg_fps median, p95 median, fps runs} and per-scene
    run-to-run avg_fps spread fraction (max-min)/min = this round's noise band."""
    maps = []
    for i in (1, 2, 3):
        p = FINAL_DIR / f"baseline_run{i}.json"
        if p.is_file():
            maps.append(scene_map(load(p)))
    if not maps:
        raise SystemExit(f"no baseline_run*.json under {FINAL_DIR}")
    med, spread = {}, {}
    for sc in SCENES:
        fps = [m[sc]["avg_fps"] for m in maps if sc in m]
        p95 = [m[sc]["p95_frame_time_ms"] for m in maps if sc in m]
        if fps:
            med[sc] = {
                "avg_fps": statistics.median(fps),
                "p95": statistics.median(p95),
                "fps_runs": [round(x, 2) for x in fps],
            }
            spread[sc] = (max(fps) - min(fps)) / min(fps) if min(fps) else 0.0
    return med, spread


def pass_legs(pass_name: str) -> list[dict]:
    """All archived legs for a pass: rurix_<pass>.json + rurix_<pass>_*.json."""
    legs = []
    base = HERE / f"rurix_{pass_name}.json"
    if base.is_file():
        legs.append(load(base))
    for extra in sorted(HERE.glob(f"rurix_{pass_name}_*.json")):
        legs.append(load(extra))
    return legs


def median_metric(legs: list[dict], scene: str, metric: str):
    vals = []
    for s in legs:
        m = scene_map(s)
        if scene in m and m[scene].get(metric) is not None:
            vals.append(m[scene][metric])
    if not vals:
        return None
    return statistics.median(vals)


def verdict_for(eng_geo: float, worst_ratio: float, worst_scene: str,
                worst_noise: float) -> str:
    """>=0.95 default-enable verdict, noise-aware.

    pass         : engaged geomean >= GATE and worst engaged scene is either
                   >= GATE or its shortfall is inside that scene's noise band.
    fail         : engaged geomean < GATE by more than the worst scene's noise.
    inconclusive : geomean straddles GATE within noise, i.e. the decision is
                   dominated by a scene whose baseline noise is wider than the
                   distance to GATE -> re-run recommended.
    """
    if eng_geo is None or math.isnan(eng_geo):
        return "invalid (no engaged coverage)"
    worst_inside_noise = worst_ratio >= GATE or (GATE - worst_ratio) <= worst_noise
    if eng_geo >= GATE and worst_inside_noise:
        return "pass"
    if eng_geo < GATE and (GATE - eng_geo) > worst_noise:
        return "fail"
    return "inconclusive (noise-edge; re-run)"


def main() -> None:
    base_med, spread = baseline_medians()

    print("=" * 96)
    print("GRX-025 per-pass DEFAULT-ENABLE bisection (measured_local; NO perf claim)")
    print("baseline = ../rd_native_final_20260713/baseline_run{1,2,3}.json (v2.3, rb4)")
    print("=" * 96)
    print("\nBaseline v2.3 per-scene median avg_fps + this-round run-to-run spread (noise band):")
    for sc in SCENES:
        b = base_med[sc]
        flag = "  <-- NOISY" if spread[sc] >= 0.02 else ""
        print(f"  {sc:22} med={b['avg_fps']:8.2f}  runs={b['fps_runs']}  "
              f"spread={spread[sc]*100:5.2f}%{flag}")
    base_geo = geomean([base_med[sc]["avg_fps"] for sc in SCENES])
    print(f"  baseline median-of-3 geomean = {base_geo:.2f}")

    rows = []
    for p in PASSES:
        legs = pass_legs(p)
        if not legs:
            print(f"\n[{p}] no legs archived yet")
            continue
        nlegs = len(legs)
        # engagement pattern from the first leg (identical across legs by design)
        amap0 = scene_map(legs[0])
        engaged_scenes = [sc for sc in SCENES if sc in amap0 and active(amap0[sc], p)]

        print("\n" + "-" * 96)
        print(f"[{p}]  legs={nlegs}  engaged_scenes={engaged_scenes or 'NONE (leg invalid)'}")
        print("-" * 96)
        print(f"  {'scene':22} {'avg_fps ratio':>14} {'p95 ratio':>11} "
              f"{'noise%':>8}  engaged")
        eng_fps, eng_p95 = [], []
        worst = (None, 2.0, 0.0)
        for sc in SCENES:
            rf = median_metric(legs, sc, "avg_fps")
            rp = median_metric(legs, sc, "p95_frame_time_ms")
            if rf is None:
                continue
            fps_ratio = rf / base_med[sc]["avg_fps"]
            p95_ratio = rp / base_med[sc]["p95"] if rp is not None else float("nan")
            eng = active(amap0[sc], p) if sc in amap0 else None
            mark = "*" if eng else " "
            print(f"  {sc:22} {fps_ratio:>13.4f}{mark} {p95_ratio:>11.4f} "
                  f"{spread[sc]*100:>7.2f}%  {'ENGAGED' if eng else '-'}")
            if eng:
                eng_fps.append(fps_ratio)
                eng_p95.append(p95_ratio)
                if fps_ratio < worst[1]:
                    worst = (sc, fps_ratio, spread[sc])
        if eng_fps:
            eng_geo = geomean(eng_fps)
            eng_p95_geo = geomean([v for v in eng_p95 if not math.isnan(v)])
            v = verdict_for(eng_geo, worst[1], worst[0], worst[2])
            print(f"  --> engaged avg_fps geomean = {eng_geo:.4f} | "
                  f"engaged p95 geomean = {eng_p95_geo:.4f}")
            print(f"  --> worst engaged scene = {worst[0]} @ {worst[1]:.4f} "
                  f"(scene noise {worst[2]*100:.2f}%)")
            print(f"  --> >=0.95 VERDICT: {v}")
            rows.append((p, nlegs, len(eng_fps), eng_geo, worst[0], worst[1],
                         worst[2], v))
        else:
            print("  --> NO engaged scenes: leg INVALID for default-enable "
                  "(native-vs-native noise); verdict = invalid")
            rows.append((p, nlegs, 0, float("nan"), "-", float("nan"), 0.0,
                         "invalid (no engaged coverage)"))

    print("\n" + "=" * 96)
    print("GRX-025 DECISION TABLE (>=0.95 engaged-geomean gate; noise-aware)")
    print("=" * 96)
    print(f"{'pass':16} {'legs':>4} {'eng#':>4} {'eng_geo':>9} "
          f"{'worst scene':>20} {'worst':>8} {'noise':>7}  verdict")
    for (p, nl, ne, eg, ws, wr, wn, v) in rows:
        eg_s = f"{eg:.4f}" if not math.isnan(eg) else "n/a"
        wr_s = f"{wr:.4f}" if not math.isnan(wr) else "n/a"
        print(f"{p:16} {nl:>4} {ne:>4} {eg_s:>9} {ws:>20} {wr_s:>8} "
              f"{wn*100:>6.2f}%  {v}")
    print("\n(measured_local; no performance claim; * = pass engaged rd_native on "
          "that scene. Gate is a DEFAULT-ENABLE decision input, not a speedup.)")


if __name__ == "__main__":
    main()
