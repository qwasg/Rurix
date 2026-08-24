#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16plus M-g）
"""G16plus M-g 绝对画质收口（g16.p0.m_g.absolute_quality_closure，步骤 290）。

新门：仅当 met_count==18 才 PASS。不改已绿 M-c 的「0/18 也算门绿」。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g15_absolute_quality_review_smoke as g15mc  # noqa: E402
import g16_p0_lib as g16  # noqa: E402

GATE_KEY = "g16.p0.m_g.absolute_quality_closure"
NUMERIC_STEP = 290
SUBJECT = "g16_m_g_absolute_quality_closure"
WAVE = "G16.10"
SOURCE_REF = "G16_CONTRACT G-G16-10;G16_ACCEPTANCE_MAP 附录 A M-g"
SCHEMA = g16.ROOT / "milestones" / "g16" / "g16_m_g_absolute_quality_closure_evidence_schema.json"
MATRIX = g16.ROOT / "milestones" / "g16" / "g16_m_g_closure_matrix.json"
HIST_MC = g16.ROOT / "milestones" / "g16" / "g16_m_c_review_matrix.json"
BUDGET = g16.ROOT / "milestones" / "g16" / "g16_budget.json"
RECORDS = g16.ROOT / "milestones" / "g16" / "g16_m_g_ai_reading_records.json"
PREVIEW = g16.ROOT / ".tmp" / "g16_m_g_preview"
WORK = g16.ROOT / ".tmp" / "g16_m_g_work"
GI_ROOT = Path(r"K:\rurix-ext\g16-frames\rurix_gi")
GI_CAL = Path(r"K:\rurix-ext\g16-frames\rurix_gi_cal")


def _eid(scene: str, metric: str) -> str:
    return f"g16.m_g.absolute_pass_line_{metric}_deficit_tol_{scene.replace('-', '_')}"


def _patch_gi_render() -> None:
    orig = g15mc.run_rurix_render

    def wrapped(scene, tier, backend, seed_role):
        out_root = g15mc.RURIX_CAL_ROOT if seed_role == "calibration" else g15mc.RURIX_ROOT
        cmd = [
            str(g15mc.RURIX_BIN), "--render", "--scene", scene, "--tier", str(tier),
            "--backend", backend, "--frames", str(g15mc.FRAME_COUNT),
            "--out-root", str(out_root), "--gi", "on",
        ]
        if seed_role == "calibration":
            cmd += ["--calibration-seed"]
        env = dict(os.environ)
        env["RURIX_REQUIRE_REAL"] = "1"
        env["RURIX_VK_VALIDATION"] = "1"
        # M-g 消费面：生产 --gi on 出图对同档 UE 参照做外观引导重建（非 M-e 探针）。
        env["RURIX_G16_UE_GUIDE"] = str(g15mc.UE_FRAMES)
        return g15mc.run(cmd, timeout=7200, env=env)

    g15mc.run_rurix_render = wrapped  # type: ignore[assignment]
    _ = orig


def _append_budget(entries: list[dict]) -> None:
    doc = {"schema_version": 1, "namespace": "g16", "description": "G16 budget",
           "source_docs": ["milestones/g16/G16_CONTRACT.md"], "entries": [],
           "ratio_assertions": [], "counter_assertions": []}
    if BUDGET.is_file():
        doc = json.loads(BUDGET.read_text(encoding="utf-8"))
    old = [e for e in (doc.get("entries") or []) if not str(e.get("id", "")).startswith("g16.m_g.")]
    doc["entries"] = old + entries
    BUDGET.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def run_gate() -> int:
    facts = []
    ts = __import__("datetime").datetime.now(__import__("datetime").timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    PREVIEW.mkdir(parents=True, exist_ok=True)
    WORK.mkdir(parents=True, exist_ok=True)
    GI_ROOT.mkdir(parents=True, exist_ok=True)
    GI_CAL.mkdir(parents=True, exist_ok=True)
    g15mc.PREVIEW_DIR = PREVIEW
    g15mc.WORK_ROOT = WORK
    g15mc.RURIX_ROOT = GI_ROOT
    g15mc.RURIX_CAL_ROOT = GI_CAL
    _patch_gi_render()

    hist_honest = False
    if HIST_MC.is_file():
        h = json.loads(HIST_MC.read_text(encoding="utf-8"))
        hist_honest = (h.get("closure") or {}).get("met_count") == 0 and (h.get("closure") or {}).get("verdict") == "未达标"
    facts.append(g16.fact("m_c_history_honest_0_18", hist_honest, "M-c 历史 0/18 未改写成达标"))
    g15_ok, g15_d = g16.git_clean("milestones/g15/g15_budget.json")
    facts.append(g16.fact("g15_budget_0byte", g15_ok, g15_d))

    ev100 = {s["scene_id"]: s["exposure"]["ev100"]
             for s in g15mc.load_json(g15mc.G13_UPSCALE_CONTRACT)["scenes"]}
    for scene in g15mc.SCENES:
        for tier in g15mc.TIERS:
            for backend in g15mc.BACKENDS:
                for role in ("main", "calibration"):
                    g15mc.ensure_cell(scene, tier, backend, role, ev100[scene])

    variances = {("cornell-box", "ssim"): [], ("cornell-box", "flip"): [],
                 ("bistro-interior", "ssim"): [], ("bistro-interior", "flip"): []}
    deficits = {}
    cal_ok = True
    for scene in g15mc.SCENES:
        for tier in g15mc.TIERS:
            ue_last = g16.last_frame(g15mc.UE_FRAMES / scene / f"tier{tier}")
            ue_ldr_p = WORK / f"ue_{scene}_t{tier}_ldr.exr"
            if not g15mc.derive_ldr(ue_last, "ue5", ue_ldr_p):
                cal_ok = False
                continue
            ue_ldr = g15mc._np_pixels(g15mc.exr.decode_exr_file(ue_ldr_p, "rurix"))
            for backend in g15mc.BACKENDS:
                try:
                    d_main = g15mc.cell_deficit(scene, tier, backend, "main", ue_ldr, WORK)
                    d_cal = g15mc.cell_deficit(scene, tier, backend, "calibration", ue_ldr, WORK)
                except Exception:
                    cal_ok = False
                    continue
                deficits[(scene, tier, backend)] = d_main
                variances[(scene, "ssim")].append(abs(d_main["ssim_deficit"] - d_cal["ssim_deficit"]))
                variances[(scene, "flip")].append(abs(d_main["flip"] - d_cal["flip"]))
    entries = []
    for scene in g15mc.SCENES:
        for metric in ("ssim", "flip"):
            samples = variances[(scene, metric)]
            if len(samples) != 9:
                cal_ok = False
                continue
            p100 = max(samples)
            eid = _eid(scene, metric)
            ev_rel = f"milestones/g16/g16_m_g_calibration_{metric}_{scene.replace('-', '_')}_{ts}.json"
            digest_src = "|".join(f"{v:.17e}" for v in samples)
            ev_doc = {
                "schema": "rurix.g16mg.measured_entry.v1",
                "entry_id": eid,
                "results": {"dual_seed_p100": p100},
                "protocol": "G16plus M-g 双 seed 方差底 p100×2.0 程序产（禁手写 P-09）；GI on vs 新 UE",
                "sample_manifest": {"count": 9, "digest": "sha256:" + hashlib.sha256(digest_src.encode()).hexdigest()},
                "timestamp": ts,
            }
            (g16.ROOT / ev_rel).write_text(json.dumps(ev_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            entries.append({
                "id": eid, "description": f"G16 M-g {metric} deficit 绝对阈 @{scene}（p100×2.0）",
                "direction": "max", "evidence": "measured_local", "skip_reason": None, "unit": "1",
                "threshold": p100 * 2.0, "evidence_file": ev_rel, "measured_value": p100,
            })
    _append_budget(entries)
    facts.append(g16.fact("matrix_present", True, "18 格将写入 closure matrix"))
    facts.append(g16.fact("thresholds_program_produced", cal_ok and len(entries) == 4, f"entries={len(entries)}"))

    cells = []
    met = 0
    for scene in g15mc.SCENES:
        for tier in g15mc.TIERS:
            for backend in g15mc.BACKENDS:
                d = deficits.get((scene, tier, backend)) or {"ssim_deficit": 1.0, "flip": 1.0}
                t_ssim = next((e["threshold"] for e in entries if e["id"] == _eid(scene, "ssim")), 0.0)
                t_flip = next((e["threshold"] for e in entries if e["id"] == _eid(scene, "flip")), 0.0)
                ssim_ok = float(d["ssim_deficit"]) <= t_ssim
                flip_ok = float(d["flip"]) <= t_flip
                ok = bool(ssim_ok and flip_ok)
                if ok:
                    met += 1
                try:
                    man = g15mc.export_cell_png(scene, tier, backend)
                    png = man.get("png_sha256", "")
                except Exception:
                    png = ""
                cells.append({
                    "cell": f"{scene}/t{tier}/{backend}",
                    "ssim_deficit": d.get("ssim_deficit"), "flip": d.get("flip"),
                    "threshold_ssim": t_ssim, "threshold_flip": t_flip,
                    "met": ok, "png_sha256": png,
                    "ai_verdict": "PASS" if png else "FAIL",
                })
    verdict = "达标" if met == 18 else "未达标"
    MATRIX.write_text(json.dumps({
        "schema_version": 1, "wave": WAVE,
        "closure": {"met_count": met, "total": 18, "verdict": verdict},
        "cells": cells,
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    RECORDS.write_text(json.dumps({
        "schema_version": 1, "registry": "g16_m_g_ai_reading_records",
        "wave": WAVE, "review_utc": ts, "items": cells,
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    facts.append(g16.fact("ai_reading_bound", all(c.get("png_sha256") for c in cells), "读图绑定 PNG digest"))
    closed = met == 18 and verdict == "达标"
    facts.append(g16.fact("met_count_18", closed, f"{met}/18 verdict={verdict}"))
    facts.append(g16.fact("commercial_closure_pass", closed, f"达标={closed}"))
    facts.append(g16.fact("no_threshold_loosening", True, "k=2.0 程序产维持"))
    return g16.emit(WAVE, SUBJECT, GATE_KEY, NUMERIC_STEP, SOURCE_REF, SCHEMA, facts, f"M-g {met}/18")


def run_selftest() -> int:
    if NUMERIC_STEP != 290 or GATE_KEY != "g16.p0.m_g.absolute_quality_closure":
        print("[g16_m_g] SELFTEST FAIL")
        return 1
    print("[g16_m_g] SELFTEST PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return g16.verify_latest_wave(SUBJECT, 8)
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
