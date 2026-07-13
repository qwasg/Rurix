#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX Route B rd_native TERMINAL benchmark campaign analysis (2026-07-13).

measured_local, single machine (RTX 4070 Ti, template_debug), NO performance
claim. Reads the archived runner summaries in this directory:
  * baseline_run1.json / _run2.json / _run3.json  (leg=baseline, backend=0, rb4 exe)
  * rurix_all5.json         (leg=rurix, 5 rd_native passes backend=2)
  * rurix_all5_fused.json   (leg=rurix, 5 passes + fused_post_chain backend=2)
  * baseline_ts.json        (leg=baseline, --gpu-timestamps)
  * rurix_all5_ts.json      (leg=rurix, 5 passes, --gpu-timestamps)

Emits:
  1. baseline v2.3 (3 full runs -> per-scene median avg_fps/p95, geomean spread);
  2. all5 (+ all5_fused control) avg_fps & p95 ratio vs baseline_median, per
     scene + geomean(ALL) + geomean(ENGAGED);
  3. rd_native engagement matrix (one-shot active marker) per scene x pass;
  4. per-pass GPU budget table (baseline_ts vs all5_ts bucket medians, us);
  5. Amdahl ceiling: per-scene Sigma(engaged replaceable-bucket GPU us)/frame
     GPU total -> theoretical FPS-ratio ceiling if those passes were zero-cost,
     vs measured all5 ratio.

BOTH legs are the SAME rb4 exe (0001-0029 + 0040-0048); backend=0 (baseline) vs
backend=2 (rurix). A pass on a scene where it did NOT engage (active=false) is
pure native-vs-native noise; such cells are excluded from the engaged-only
geomean. fused_post_chain engages on NO bench scene (LINEAR-tonemap subset and
auto-exposure are mutually exclusive across the 7 scenes) so all5_fused is an
honest duplicate control of all5 and the fused per-pass column is n/a.
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
# rd_native replaceable passes and the native Godot RENDER_TIMESTAMP bucket each
# one replaces (verified against the probe capture 2026-07-13).
PASS_BUCKET = {
    "tonemap": "Tonemap",
    "ssao_blur": "Process SSAO",
    "taa_resolve": "TAA",
    "particles_copy": "Particles View-Axis Copy",
    "cluster_store": "Pack 3D Cluster Elements",
}
PASSES = list(PASS_BUCKET.keys())
FRAME_TOTAL = "__frame_total__"


def load(name: str) -> dict:
    return json.loads((HERE / name).read_text(encoding="utf-8"))


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


def bucket_median(res: dict, bucket: str) -> float | None:
    gts = res.get("gpu_timestamps")
    if not isinstance(gts, dict):
        return None
    b = gts.get("buckets", {}).get(bucket)
    if not isinstance(b, dict):
        return None
    return float(b["median_us"])


def main() -> None:
    # ---- baseline v2.3: 3 runs -> per-scene median ----
    b_runs, b_maps = [], []
    for i in (1, 2, 3):
        p = HERE / f"baseline_run{i}.json"
        if p.is_file():
            s = load(p.name)
            b_runs.append(s)
            b_maps.append(scene_map(s))
    if not b_runs:
        print("no baseline_run*.json yet")
        return
    base_med = {}
    for sc in SCENES:
        fps = [m[sc]["avg_fps"] for m in b_maps if sc in m]
        p95 = [m[sc]["p95_frame_time_ms"] for m in b_maps if sc in m]
        if fps:
            base_med[sc] = {
                "avg_fps": statistics.median(fps),
                "p95": statistics.median(p95),
                "fps_runs": [round(x, 2) for x in fps],
                "p95_runs": [round(x, 4) for x in p95],
            }
    run_geos = [(s.get("run_id"), geomean([m[sc]["avg_fps"] for sc in SCENES if sc in m]))
                for s, m in zip(b_runs, b_maps)]
    rg_sorted = sorted(run_geos, key=lambda t: t[1])
    median_run = rg_sorted[len(rg_sorted) // 2]

    print("=" * 92)
    print("1. BASELINE v2.3 (rb4 exe, backend=0, %d full runs; per-scene MEDIAN)" % len(b_runs))
    print("=" * 92)
    print(f"{'scene':22} {'med_fps':>9} {'med_p95':>9}   fps_runs")
    for sc in SCENES:
        if sc in base_med:
            b = base_med[sc]
            print(f"{sc:22} {b['avg_fps']:>9.2f} {b['p95']:>9.4f}   {b['fps_runs']}")
    gspread = [g for _, g in run_geos]
    print("\nper-run geomean(avg_fps): " + ", ".join(f"{rid}={g:.2f}" for rid, g in run_geos))
    if len(gspread) > 1:
        print("geomean spread: %.2f..%.2f (%.2f%%)" %
              (min(gspread), max(gspread), 100.0 * (max(gspread) - min(gspread)) / min(gspread)))
    print("median-geomean run pick: %s (geomean %.2f)" % (median_run[0], median_run[1]))
    base_geo = geomean([base_med[sc]["avg_fps"] for sc in SCENES if sc in base_med])
    print("baseline median-of-3 geomean: %.2f" % base_geo)

    # ---- rurix ratio legs ----
    legs = {}
    for name in ("all5", "all5_fused"):
        p = HERE / f"rurix_{name}.json"
        if p.is_file():
            legs[name] = load(p.name)

    # engagement matrix (5 passes from all5; fused column from the all5_fused
    # leg, the only leg where fused_post_chain was armed).
    print("\n" + "=" * 92)
    print("3. rd_native ENGAGEMENT (one-shot active marker) per scene x pass")
    print("   [5 passes: all5 leg; fused_post_chain: all5_fused leg]")
    print("=" * 92)
    if "all5" in legs:
        amap = scene_map(legs["all5"])
        fmap = scene_map(legs["all5_fused"]) if "all5_fused" in legs else {}
        hdr = f"{'scene':22}" + "".join(f"{p[:13]:>15}" for p in PASSES + ["fused_post_chain"])
        print(hdr)
        for sc in SCENES:
            if sc in amap:
                cells = "".join(f"{str(active(amap[sc], p)):>15}" for p in PASSES)
                fused = active(fmap[sc], "fused_post_chain") if sc in fmap else None
                cells += f"{str(fused):>15}"
                print(f"{sc:22}{cells}")

    # ratio tables
    for metric, label in [("avg_fps", "2a. avg_fps ratio (rurix/baseline_median; >1 faster)"),
                          ("p95_frame_time_ms", "2b. p95 frametime ratio (rurix/baseline_median; <1 faster)")]:
        print("\n" + "=" * 92)
        print(label)
        print("=" * 92)
        cols = [n for n in ("all5", "all5_fused") if n in legs]
        print(f"{'scene':22}" + "".join(f"{c:>16}" for c in cols))
        allr = {c: [] for c in cols}
        engr = {c: [] for c in cols}
        for sc in SCENES:
            if sc not in base_med:
                continue
            bval = base_med[sc]["avg_fps"] if metric == "avg_fps" else base_med[sc]["p95"]
            row = f"{sc:22}"
            for c in cols:
                m = scene_map(legs[c])
                if sc not in m or m[sc].get(metric) is None:
                    row += f"{'-':>16}"
                    continue
                ratio = m[sc][metric] / bval if bval else float("nan")
                eng = any(active(m[sc], p) for p in PASSES)
                allr[c].append(ratio)
                if eng:
                    engr[c].append(ratio)
                row += f"{ratio:>14.4f}{'*' if eng else ' '} "
            print(row)
        print(f"{'geomean(ALL 7)':22}" + "".join(f"{geomean(allr[c]):>16.4f}" for c in cols))
        print(f"{'geomean(ENGAGED*)':22}" +
              "".join((f"{geomean(engr[c]):>16.4f}" if engr[c] else f"{'n/a':>16}") for c in cols))

    # ---- GPU budget + Amdahl ----
    bts = load("baseline_ts.json") if (HERE / "baseline_ts.json").is_file() else None
    ats = load("rurix_all5_ts.json") if (HERE / "rurix_all5_ts.json").is_file() else None
    if bts and ats:
        bmap, amap = scene_map(bts), scene_map(ats)
        print("\n" + "=" * 92)
        print("4. PER-PASS GPU BUDGET (median us): baseline_ts (B) vs all5_ts (R); "
              "delta=B-R; * engaged")
        print("=" * 92)
        buckets_order = [PASS_BUCKET[p] for p in PASSES] + ["Auto exposure", FRAME_TOTAL]
        for sc in SCENES:
            if sc not in bmap or sc not in amap:
                continue
            print(f"\n-- {sc} --")
            print(f"   {'bucket':30} {'baseline':>10} {'all5':>10} {'delta':>9}  eng")
            for p in PASSES:
                bk = PASS_BUCKET[p]
                bv = bucket_median(bmap[sc], bk)
                rv = bucket_median(amap[sc], bk)
                eng = active(amap[sc], p)
                bs = f"{bv:>10.2f}" if bv is not None else f"{'-':>10}"
                rs = f"{rv:>10.2f}" if rv is not None else f"{'-':>10}"
                ds = f"{bv - rv:>9.2f}" if (bv is not None and rv is not None) else f"{'-':>9}"
                print(f"   {bk:30} {bs} {rs} {ds}  {'*' if eng else ''}")
            for bk in ("Auto exposure", FRAME_TOTAL):
                bv = bucket_median(bmap[sc], bk)
                rv = bucket_median(amap[sc], bk)
                bs = f"{bv:>10.2f}" if bv is not None else f"{'-':>10}"
                rs = f"{rv:>10.2f}" if rv is not None else f"{'-':>10}"
                ds = f"{bv - rv:>9.2f}" if (bv is not None and rv is not None) else f"{'-':>9}"
                print(f"   {bk:30} {bs} {rs} {ds}")

        # Amdahl ceiling
        print("\n" + "=" * 92)
        print("5. AMDAHL CEILING (GPU-bound upper bound): engaged replaceable buckets / "
              "frame GPU total")
        print("=" * 92)
        print(f"{'scene':22} {'engaged buckets':>16} {'frameGPU us':>13} {'repl frac':>10} "
              f"{'ceil x':>8}  {'meas all5':>10}")
        ceils, meas = [], []
        amap_ratio = scene_map(legs["all5"]) if "all5" in legs else {}
        for sc in SCENES:
            if sc not in bmap:
                continue
            ftot = bucket_median(bmap[sc], FRAME_TOTAL)
            if not ftot:
                continue
            repl = 0.0
            for p in PASSES:
                if active(amap[sc], p):  # engaged on the rurix ts run
                    bv = bucket_median(bmap[sc], PASS_BUCKET[p])
                    if bv is not None:
                        repl += bv
            frac = repl / ftot
            ceil = 1.0 / (1.0 - frac) if frac < 1.0 else float("inf")
            ceils.append(ceil)
            mr = None
            if sc in amap_ratio and amap_ratio[sc].get("avg_fps") and sc in base_med:
                mr = amap_ratio[sc]["avg_fps"] / base_med[sc]["avg_fps"]
                meas.append(mr)
            print(f"{sc:22} {repl:>16.2f} {ftot:>13.2f} {frac:>10.4f} {ceil:>8.4f}  "
                  f"{(f'{mr:.4f}' if mr else 'n/a'):>10}")
        print(f"\n{'geomean ceiling (x)':22} {geomean(ceils):.4f}   "
              f"vs measured all5 avg_fps geomean {geomean(meas):.4f}")

    print("\n(measured_local; no performance claim; * = pass engaged rd_native on that "
          "scene. fused engages on no scene -> all5_fused == all5 control.)")


if __name__ == "__main__":
    main()
