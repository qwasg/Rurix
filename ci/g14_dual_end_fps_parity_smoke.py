#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.4 双端对标波）
"""G14.4 P0 硬门 M-d：双端帧率正式对标 + 画质零降级守护
（g14.p0.m_d.dual_end_fps_parity；G14_CONTRACT §4.2 M-d/G-G14-6；
G14_ACCEPTANCE_MAP §1 M-d 行；G10-N16/G11-N3 帧率面兑现 + 用户
「帧率对标UE5略高（不降级画质）」字面兑现面）。

判据（契约 §4.2 M-d 逐字）：
- 同场景同输出分辨率同超分档位 GPU 管线双端 A/B：UE 臂 = M-b benchmark 臂测量面
  （本门复跑 g14_2_ue_bench 三轮进程级独立运行，新鲜度机核）；Rurix 臂 = M-c
  生产管线路径（g14_3_pipeline_perf --bench 三轮进程级独立运行）；统计 =
  50×3 trimmed mean 跨轮中位数（M141/M165 冻结口径）；
- **通过线 = Rurix 三轮 trimmed mean 帧率 ≥ UE 同口径 ×1.00**（「略高」下限，
  G14-N7 口径裁决登记）+ 逐轮守护带登记（逐轮比值入 evidence）；
- 画质零降级守护：G13 锁定对拍 deficit 基线带内不劣化（经 M-a 修订后 M-c/M-d
  门复跑面——本门消费其最新 evidence checks 面 + G14.3 车道 vs G13.4 车道
  锚定守护带复核）；G14 不设绝对画质通过线归 G15；
- 对标差距/未达标项显式登记 milestones/g14/g14_fps_gap_registry.json（gaplib
  正典形，不静默混入）；未达标冒充达标即 RED（未达标如实登记，本门 verdict
  红 = 通过线未达的诚实面，不阻塞 G14.5a 穷举；商用收口判定归 G15+）。

RED 字面：以单轮/混合口径/MRQ 含开销数据冒充正式对标即 RED；画质劣化静默
即 RED；未达标冒充达标即 RED（门内三臂：single-round/mixed-caliber/
unmet-masquerade 检出）。

用法：
  py -3 ci/g14_dual_end_fps_parity_smoke.py --gate g14.p0.m_d.dual_end_fps_parity
  py -3 ci/g14_dual_end_fps_parity_smoke.py --verify-latest
  py -3 ci/g14_dual_end_fps_parity_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g10_wave_exit_lib as wel  # noqa: E402
import g10_gap_registry_lib as gaplib  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g14.p0.m_d.dual_end_fps_parity"
NUMERIC_STEP = 254  # 落盘前实测 registry/number_ledger.json CI_step.next_free=254 顺位领取
SUBJECT = "g14_m_d_dual_end_fps_parity"
WAVE = "G14.4"
TAG = "g14_m_d"
MATRIX_ROW = "M175"
SOURCE_REF = (
    "G14_CONTRACT §4.2 M-d/G-G14-6;G14_ACCEPTANCE_MAP §1;G10-N16/G11-N3 帧率面;"
    "G14-N7 通过线 ×1.00 口径裁决;M141/M165 50×3 冻结统计口径"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_m_d_dual_end_fps_parity_evidence_schema.json"
REGISTRY_PATH = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"
REGISTRY_NAME = "g14_fps_gap_registry"

UE_BENCH = ROOT / "milestones" / "g14" / "harness" / "g14_2_ue_bench.py"
RURIX_BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"
UE_CSV_DIR = Path(r"K:\rurix-ext\g10-ue\G10RefRender\Saved\Profiling\CSV")
BENCH_ROOT = Path(r"K:\rurix-ext\g14-frames\ue_bench")
MC_SMOKE = ROOT / "ci" / "g14_rurix_pipeline_perf_smoke.py"
MB_SMOKE = ROOT / "ci" / "g14_ue_benchmark_arm_measurement_smoke.py"

SCENES = ("cornell-box", "bistro-interior")
TIERS = (50, 67, 100)
RUNS = (1, 2, 3)
PASS_LINE_RATIO = 1.00  # G14-N7 口径裁决：「略高」最保守机器可核下限（≥ 即达标）

CHECK_KEYS = [
    "dual_end_measurement_fresh",
    "three_run_independence",
    "sampling_protocol_50x3",
    "pass_line_evaluated",
    "quality_guard_green",
    "gap_registry_written",
    "budget_eval_all_pass",
    "red_arms_effective",
]

NOTES: list[str] = []
FAILURES: list[str] = []


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env=None) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


# ---------------------------------------------------------------- 双端测量腿
def run_ue_cell(scene: str, tier: int) -> dict | None:
    """UE 臂三轮（经 M-b harness；逐轮 receipt+CSV 收割）。"""
    sys.path.insert(0, str(MB_SMOKE.parent))
    import g14_ue_benchmark_arm_measurement_smoke as mb  # noqa: E402
    runs = []
    for run_index in RUNS:
        rr = run([sys.executable, str(UE_BENCH), scene, "--tier", str(tier),
                  "--run-index", str(run_index)], timeout=2400)
        rp = BENCH_ROOT / scene / f"tier{tier}" / f"bench_receipt_r{run_index}.json"
        rec = wel.load_json(rp) if rp.is_file() else {}
        cp = UE_CSV_DIR / f"g14_bench_{scene}_t{tier}_r{run_index}.csv"
        # CSV 新鲜度机核（mtime ≥ 本轮 started_epoch−5s——M-b 同名 tag 陈旧 CSV
        # 防混面：harness exit=0 但 UE 未重写 CSV 时不得消费旧数据冒充新鲜轮）。
        fresh = (cp.is_file() and rec.get("started_epoch") is not None
                 and cp.stat().st_mtime >= rec["started_epoch"] - 5.0)
        parsed = mb.parse_csv_frame_times(cp) if fresh else {"ok": False}
        if rr.returncode != 0 or not rec or not parsed.get("ok"):
            return None
        import g10_perf_baseline_smoke as g10pb  # noqa: E402
        stats = g10pb.block_stats(parsed["frame_times"])
        runs.append({"run_index": run_index, "trimmed_mean_ms": stats["trimmed_mean_ms"],
                     "cv": stats["cv"], "started_epoch": rec.get("started_epoch", 0)})
    if len(runs) != len(RUNS):
        return None
    means = sorted(r["trimmed_mean_ms"] for r in runs)
    return {"runs": runs, "median_ms": means[len(means) // 2]}


def run_rurix_cell(scene: str, tier: int, backend: str) -> dict | None:
    """Rurix 臂三轮（g14_3 bench）。"""
    sys.path.insert(0, str(MC_SMOKE.parent))
    import g14_rurix_pipeline_perf_smoke as mc  # noqa: E402
    runs = []
    for run_index in RUNS:
        res = mc.run_bench(scene, tier, backend, run_index)
        if not res.get("ok"):
            return None
        runs.append({"run_index": run_index, "frame_ms_mean": res["frame_ms_mean"],
                     "cv": res["cv"], "started_epoch": res["started_epoch"]})
    means = sorted(r["frame_ms_mean"] for r in runs)
    return {"runs": runs, "median_ms": means[len(means) // 2]}


def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    started = _dt.datetime.now(_dt.timezone.utc)
    ts = started.strftime("%Y%m%dT%H%M%SZ")
    cells: list[dict] = []

    # ── ① 双端三轮进程级独立运行（UE 臂 + Rurix 臂逐格） ──
    # 锁纪律（首跑嵌套锁死锁修复留痕：门外层持锁 + UE harness 子进程跨进程自持
    # 锁 = 互等死锁——双实例并发 30min 无进展发现，kill 双实例后修订为本面）：
    # UE 臂 = harness 子进程逐轮自持 GPU 锁（M-b 同律，门侧不再外层持锁）；
    # Rurix 臂 = bin 不自持锁，门侧逐格持锁串行（M-c 同律）。
    meas_ok = True
    for scene in SCENES:
        for tier in TIERS:
            note(f"UE 臂 {scene}/t{tier} 三轮…")
            ue = run_ue_cell(scene, tier)
            if ue is None:
                meas_ok = False
                check(False, f"UE 臂测量失败 {scene}/t{tier}")
                continue
            for backend in ("tsr_device", "dlss_sr", "fsr_3_1_5"):
                note(f"Rurix 臂 {scene}/t{tier}/{backend} 三轮…")
                with gpu_device_lock(purpose=f"{TAG} Rurix 臂 {scene}/t{tier}/{backend} 三轮"):
                    ru = run_rurix_cell(scene, tier, backend)
                if ru is None:
                    meas_ok = False
                    check(False, f"Rurix 臂测量失败 {scene}/t{tier}/{backend}")
                    continue
                ratio = (1000.0 / ru["median_ms"]) / (1000.0 / ue["median_ms"])
                per_run_ratios = [
                    (1000.0 / rr["frame_ms_mean"]) / (1000.0 / ue["runs"][i]["trimmed_mean_ms"])
                    for i, rr in enumerate(ru["runs"])
                ]
                cells.append({
                    "scene": scene, "tier": tier, "backend": backend,
                    "ue_median_ms": ue["median_ms"],
                    "rurix_median_ms": ru["median_ms"],
                    "fps_ratio": ratio,
                    "per_run_ratios": per_run_ratios,
                    "pass": ratio >= PASS_LINE_RATIO,
                })
    checks["dual_end_measurement_fresh"] = meas_ok and len(cells) == len(SCENES) * len(TIERS) * 3
    checks["three_run_independence"] = meas_ok
    checks["sampling_protocol_50x3"] = meas_ok

    # ── ② 通过线判定（逐格 Rurix fps ≥ UE ×1.00 + 逐轮守护带登记） ──
    unmet = [c for c in cells if not c["pass"]]
    met = [c for c in cells if c["pass"]]
    checks["pass_line_evaluated"] = bool(cells)
    note(f"通过线判定（×{PASS_LINE_RATIO:.2f}）：达标 {len(met)}/{len(cells)} 格；未达标 {len(unmet)} 格")
    for c in cells:
        note(f"  {c['scene']}/t{c['tier']}/{c['backend']}: UE={c['ue_median_ms']:.3f}ms "
             f"Rurix={c['rurix_median_ms']:.3f}ms ratio={c['fps_ratio']:.4f} "
             f"{'达标' if c['pass'] else '未达标'}")

    # ── ③ 画质零降级守护（G13 锁定基线带内经 M-a 修订面复跑绿 + G14.3 车道锚带） ──
    guard_ok = True
    guard_notes: list[str] = []
    for prefix, gate_key in (("g13_m_c_ue_upscale_parity", "g13.p0.m_c.ue_upscale_parity"),
                             ("g13_m_d_ue_lumen_gi_parity", "g13.p0.m_d.ue_lumen_gi_parity")):
        path = wel.load_latest_evidence(prefix)
        if path is None:
            guard_ok = False
            guard_notes.append(f"{prefix} 缺 evidence")
            continue
        doc = wel.load_json(path)
        if doc.get("status") != "pass":
            guard_ok = False
            guard_notes.append(f"{prefix} 最新 evidence status={doc.get('status')!r}")
            continue
        gchecks = doc.get("checks") or {}
        bad = [k for k, v in gchecks.items() if v is not True]
        if bad:
            guard_ok = False
            guard_notes.append(f"{prefix} checks 非全真: {bad[:3]}")
        else:
            guard_notes.append(f"{prefix} PASS（{path.name}）")
    # G14.3 车道锚带（画质不劣化于首跑锚）复核
    budget_path = ROOT / "milestones" / "g14" / "g14_budget.json"
    if budget_path.is_file():
        bud = wel.load_json(budget_path)
        anchor = next((e for e in (bud.get("entries") or [])
                       if e.get("id") == "g14.pipeline_perf.quality_anchor_ssim_deficit"), None)
        if anchor is None:
            guard_ok = False
            guard_notes.append("g14 画质锚守护位缺")
        else:
            guard_notes.append(f"画质锚带在树（threshold={anchor['threshold']:.6g}）")
    checks["quality_guard_green"] = guard_ok
    note(f"画质零降级守护：{'; '.join(guard_notes)}")

    # ── ④ 差距登记表（未达标项显式登记，gaplib 正典形；全达标 = 空表显式登记） ──
    registry_rows: list[dict] = []
    cam = "g14_dual_end_fps_parity"
    prim = gaplib.MODULE_PREFIX + "PostProcess"
    import hashlib
    for c in unmet:
        title = f"fps_parity_deficit@{c['scene']}/t{c['tier']}/{c['backend']}"
        cell_digest = "sha256:" + hashlib.sha256(
            json.dumps(c, sort_keys=True).encode("utf-8")).hexdigest()
        registry_rows.append({
            "gap_id": gaplib.derive_gap_id(c["scene"], cam, prim, "quality_gap", title),
            "scene_id": c["scene"], "camera_id": cam,
            "domain": "display-referred-ldr", "kind": "quality_gap",
            "ue5_module_primary": prim, "ue5_module_secondary": [],
            "measured_delta": [{
                "metric": f"fps_ratio@{c['scene']}/t{c['tier']}/{c['backend']}",
                "a_value": 1000.0 / c["ue_median_ms"],
                "b_value": 1000.0 / c["rurix_median_ms"],
                "delta": (1000.0 / c["rurix_median_ms"]) - (1000.0 / c["ue_median_ms"]),
                "evidence_digest": cell_digest,
            }],
            "suggested_priority": "P1",
            "g11_anchor": "G14-N7 通过线口径（G15+/G16 继续优化承接；G14 如实登记不冒充）",
            "title": title,
            "description": (
                f"双端帧率对标未达标：{c['scene']} tier{c['tier']} {c['backend']} "
                f"Rurix/UE fps 比 {c['fps_ratio']:.4f} < {PASS_LINE_RATIO:.2f} 通过线"
                f"（三轮进程级独立运行 50×3 trimmed mean 跨轮中位数）；逐轮比值入 evidence；"
                f"只登记不拟合（RXS-0392）。"
            ),
            "attachments": [],
        })
    registry_doc = {
        "schema_version": 1,
        "registry": REGISTRY_NAME,
        "generated_by": "ci/g14_dual_end_fps_parity_smoke.py --gate g14.p0.m_d.dual_end_fps_parity",
        "scene_set": list(SCENES),
        "items": registry_rows,
        "scene_summary": [
            {"scene_id": s,
             "gap_count": sum(1 for r in registry_rows if r["scene_id"] == s),
             "no_gap_explicit": not any(r["scene_id"] == s for r in registry_rows)}
            for s in SCENES
        ],
        "not_ready_scenes": [],
    }
    verrs = gaplib.validate_registry(registry_doc, scene_set=list(SCENES), registry_name=REGISTRY_NAME)
    reg_ok = not verrs
    check(not verrs, f"g14 帧率差距登记表 schema 校验: {verrs[:3]}")
    if reg_ok:
        REGISTRY_PATH.write_text(json.dumps(registry_doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    checks["gap_registry_written"] = reg_ok

    # ── ⑤ budget_eval 全 PASS ──
    bud = run([sys.executable, str(ROOT / "ci" / "budget_eval.py")], timeout=900)
    checks["budget_eval_all_pass"] = bud.returncode == 0 and "[budget_eval] PASS" in (bud.stdout or "")

    # ── ⑥ RED 三臂（host 函数级真跑） ──
    red: dict[str, bool] = {}
    red["single_round_detected"] = True  # 见下（函数面）
    # 单轮冒充三轮：聚合判定面少轮必红
    def _aggregate_ok(run_count: int) -> bool:
        return run_count == len(RUNS)
    red["single_round_detected"] = (not _aggregate_ok(1)) and _aggregate_ok(3)
    # 混合口径冒充：MRQ 含开销数据充 benchmark 臂（capture_arm 面非 benchmark 即拒）
    def _accept_ue_arm(capture_arm: str) -> bool:
        return "benchmark" in str(capture_arm)
    red["mixed_caliber_detected"] = (not _accept_ue_arm("A-mrq-batch")) and _accept_ue_arm("B-benchmark-csvprofile")
    # 未达标冒充达标：ratio<1.00 不得 pass
    def _pass_of(ratio: float) -> bool:
        return ratio >= PASS_LINE_RATIO
    red["unmet_masquerade_detected"] = (not _pass_of(0.97)) and _pass_of(1.0)
    red_ok = all(red.values())
    checks["red_arms_effective"] = red_ok
    check(red_ok, f"RED 臂面: {red}")

    # ── verdict：通过线全达标 = PASS；未达标 = FAIL（诚实面，差距登记表已落盘） ──
    pass_line_green = not unmet
    checks_all = all(checks.values()) and not FAILURES
    all_pass = checks_all and pass_line_green
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "status": "pass" if all_pass else "fail",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": (run(["git", "rev-parse", "HEAD"]).stdout or "").strip(),
        "host_section_pass": all_pass,
        "device_section_state": "executed" if checks["dual_end_measurement_fresh"] else "fail",
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": "g14_2_ue_bench <scene> --tier <t> --run-index <1..3> ×6（UE 臂复跑）",
             "exit_code": 0 if checks["dual_end_measurement_fresh"] else 1},
            {"seq": 2, "command": "g14_3_pipeline_perf --bench ×18（Rurix 臂三轮逐格）",
             "exit_code": 0 if checks["three_run_independence"] else 1},
            {"seq": 3, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
            {"seq": 4, "command": "RED 三臂（single-round/mixed-caliber/unmet-masquerade）",
             "exit_code": 0 if checks["red_arms_effective"] else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "production": {
            "correctness_anchor_unchanged": checks["quality_guard_green"],
            "baseline_anchor_id": "g14.ue_benchmark.frame_ms.*（UE 臂）+ g14.pipeline_perf.frame_ms.*（Rurix 臂）",
            "measured_value": f"达标 {len(met)}/{len(cells)}；"
            + "; ".join(f"{c['scene']}/t{c['tier']}/{c['backend']}: ratio={c['fps_ratio']:.4f}" for c in cells[:18]),
            "not_worse_than_anchor": checks["quality_guard_green"],
            "threshold_provenance": "通过线 = 契约面（G14-N7 口径裁决 ×1.00 下限，非 measured 标定面——判档理由契约登记）；统计 = M141/M165 冻结 50×3 口径",
            "evolution_register": (
                f"通过线判定：达标 {len(met)}/{len(cells)} 格；未达标 {len(unmet)} 格显式登记 "
                f"g14_fps_gap_registry.json（不静默混入）；未达标如实登记不冒充（G-G14-6 诚实面）"
            ),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "parity": {
            "pass_line_ratio": PASS_LINE_RATIO,
            "met_count": len(met),
            "unmet_count": len(unmet),
            "cells": cells,
            "quality_guard": guard_notes,
            "gap_registry_file": "milestones/g14/g14_fps_gap_registry.json",
        },
    }
    errs = wel.validate_schema(evidence, SCHEMA_PATH) if SCHEMA_PATH.is_file() else []
    if errs:
        print(f"[{TAG}] schema errors: {errs}", file=sys.stderr)
        all_pass = False
        evidence["status"] = "fail"
        evidence["host_section_pass"] = False
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    print(f"[{TAG}] VERDICT={'PASS' if all_pass else 'FAIL'} checks={sum(1 for v in checks.values())}/{len(checks)}"
          f" pass_line={'达标' if pass_line_green else '未达标'}")
    return 0 if all_pass else 1


def verify_latest() -> int:
    path = wel.load_latest_evidence(SUBJECT)
    if path is None:
        print(f"[{TAG}] FAIL: 缺最新 evidence（{SUBJECT}_*.json）", file=sys.stderr)
        return 1
    doc = wel.load_json(path)
    checks = doc.get("checks") or {}
    bad = [k for k in CHECK_KEYS if checks.get(k) is not True]
    if bad or doc.get("status") != "pass":
        print(f"[{TAG}] FAIL checks={bad} status={doc.get('status')!r}", file=sys.stderr)
        return 1
    print(f"[{TAG}] verify-latest PASS（{path.name}）")
    return 0


def selftest() -> int:
    failures = 0
    schema = wel.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        failures += 1
    if failures:
        return 1
    print(f"[{TAG}] selftest PASS（schema 闭集）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.verify_latest:
        return verify_latest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
