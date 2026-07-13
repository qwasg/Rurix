#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX Route B rd_native Wave 5 checkpoint analysis (measured_local, no perf claim).

Reads the archived runner summaries in this directory:
  * baseline_run1.json / _run2.json / _run3.json  (leg=baseline, backend=0, rb2 exe)
  * rurix_all5.json                               (leg=rurix, all 5 rd_native passes)
  * rurix_<pass>.json                             (leg=rurix, single rd_native pass)

and emits the checkpoint tables:
  * baseline v2.2 = per-scene median avg_fps / median p95 across the 3 baseline
    runs, plus the median-geomean run id (the aggregation precedent from v2.1);
  * per-pass avg_fps ratio (rurix / baseline_median), per scene + geomean over
    ALL scenes and over ENGAGED scenes only;
  * per-pass p95 frame-time ratio (rurix / baseline_median; <1 == faster);
  * per-scene rd_native engagement (the one-shot active marker) so an engaged
    leg is never conflated with a failed-closed one.

BOTH legs are the SAME rb2 exe (0001-0029 + 0040-0045); backend=0 (baseline) vs
backend=2 (rurix). This is a checkpoint, not the strict close-out perf gate: it
makes NO performance claim and every number is measured_local. Numbers for a
pass on a scene where the pass did NOT engage (active=false) are pure
native-vs-native noise and are excluded from the engaged-only geomean.
"""
from __future__ import annotations

import json
import math
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
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


def load(name: str) -> dict:
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def scene_map(summary: dict) -> dict[str, dict]:
    out = {}
    for r in summary.get("per_scene_results", []):
        out[r["scene_name"]] = r
    return out


def geomean(vals: list[float]) -> float:
    vals = [v for v in vals if v and v > 0]
    if not vals:
        return float("nan")
    return math.exp(sum(math.log(v) for v in vals) / len(vals))


def main() -> None:
    # ---- baseline v2.2: 3 runs -> per-scene median ----
    b_runs = []
    for i in (1, 2, 3):
        p = HERE / f"baseline_run{i}.json"
        if p.is_file():
            b_runs.append(load(p.name))
    if not b_runs:
        print("no baseline_run*.json yet")
        return
    b_maps = [scene_map(s) for s in b_runs]
    base_med = {}
    for sc in SCENES:
        fps = [m[sc]["avg_fps"] for m in b_maps if sc in m]
        p95 = [m[sc]["p95_frame_time_ms"] for m in b_maps if sc in m]
        if fps:
            base_med[sc] = {
                "avg_fps": statistics.median(fps),
                "p95": statistics.median(p95),
                "fps_runs": [round(x, 2) for x in fps],
            }
    run_geos = [(s.get("run_id"), geomean([m[sc]["avg_fps"] for sc in SCENES if sc in m]))
                for s, m in zip(b_runs, b_maps)]
    run_geos_sorted = sorted(run_geos, key=lambda t: t[1])
    median_run = run_geos_sorted[len(run_geos_sorted) // 2]

    print("=" * 78)
    print("BASELINE v2.2 (rb2 exe, backend=0, %d full runs; per-scene MEDIAN)" % len(b_runs))
    print("=" * 78)
    print(f"{'scene':22} {'median_fps':>11} {'median_p95':>11}   runs")
    for sc in SCENES:
        if sc in base_med:
            b = base_med[sc]
            print(f"{sc:22} {b['avg_fps']:>11.2f} {b['p95']:>11.4f}   {b['fps_runs']}")
    print(f"\nper-run geomean(avg_fps): " +
          ", ".join(f"{rid}={g:.2f}" for rid, g in run_geos))
    print(f"median-geomean run (v2.1-style aggregate pick): {median_run[0]} "
          f"(geomean {median_run[1]:.2f})")
    base_geo = geomean([base_med[sc]["avg_fps"] for sc in SCENES if sc in base_med])
    print(f"baseline median-of-3 geomean: {base_geo:.2f}")

    # ---- rurix legs ----
    legs = {}
    for name in ["all5"] + PASSES:
        p = HERE / f"rurix_{name}.json"
        if p.is_file():
            legs[name] = load(p.name)

    def active(res: dict, pass_name: str) -> bool | None:
        eng = res.get("pass_engagement") or {}
        e = eng.get(pass_name)
        if isinstance(e, dict) and "rd_native_active" in e:
            return bool(e["rd_native_active"])
        return None

    # engagement table
    print("\n" + "=" * 78)
    print("rd_native ENGAGEMENT (one-shot active marker) per scene x pass (all5 leg)")
    print("=" * 78)
    if "all5" in legs:
        amap = scene_map(legs["all5"])
        hdr = f"{'scene':22}" + "".join(f"{p[:11]:>13}" for p in PASSES)
        print(hdr)
        for sc in SCENES:
            if sc in amap:
                cells = "".join(f"{str(active(amap[sc], p)):>13}" for p in PASSES)
                print(f"{sc:22}{cells}")

    # ratio tables
    for metric, label, better in [("avg_fps", "avg_fps ratio (rurix/baseline; >1 faster)", "hi"),
                                  ("p95_frame_time_ms", "p95 frametime ratio (rurix/baseline; <1 faster)", "lo")]:
        print("\n" + "=" * 78)
        print(label)
        print("=" * 78)
        cols = [n for n in (["all5"] + PASSES) if n in legs]
        print(f"{'scene':22}" + "".join(f"{c[:11]:>13}" for c in cols))
        ratios = {c: [] for c in cols}
        eng_ratios = {c: [] for c in cols}
        for sc in SCENES:
            if sc not in base_med:
                continue
            bval = base_med[sc]["avg_fps"] if metric == "avg_fps" else base_med[sc]["p95"]
            row = f"{sc:22}"
            for c in cols:
                m = scene_map(legs[c])
                if sc not in m or m[sc].get(metric) is None:
                    row += f"{'-':>13}"
                    continue
                rv = m[sc][metric]
                ratio = rv / bval if bval else float("nan")
                # engaged? for single-pass leg, that pass; for all5, ANY pass active
                if c == "all5":
                    eng = any(active(m[sc], p) for p in PASSES)
                else:
                    eng = active(m[sc], c)
                ratios[c].append(ratio)
                if eng:
                    eng_ratios[c].append(ratio)
                mark = "*" if eng else " "
                row += f"{ratio:>12.3f}{mark}"
            print(row)
        print(f"{'geomean(ALL)':22}" + "".join(f"{geomean(ratios[c]):>13.4f}" for c in cols))
        print(f"{'geomean(ENGAGED*)':22}" +
              "".join((f"{geomean(eng_ratios[c]):>13.4f}" if eng_ratios[c] else f"{'n/a':>13}") for c in cols))

    print("\n(* = the pass engaged rd_native on that scene; non-engaged cells are "
          "native-vs-native noise. No performance claim; measured_local.)")


if __name__ == "__main__":
    main()
