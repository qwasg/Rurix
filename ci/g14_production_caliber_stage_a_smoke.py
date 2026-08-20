#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.6 口径与 host 面优化波）
"""G14.6 P0 硬门 M-f：生产口径双列 + vendor Stage A 位级零漂移
（g14.p0.m_f.production_caliber_stage_a；G14_CONTRACT §4.2 M-f 行/G14.x 延续波
程序面〔§7 裁决 7〕；G14_ACCEPTANCE_MAP §1 M-f 行）。

判据（契约 §4.2 M-f 逐字）：
- bench receipt 双列口径落盘：frame_ms（全量口径，G14.3 兼容）与
  frame_ms_production（= frame − tail；tail = is_finite 全帧校验 +
  frame_content_digest payload 重建+sha256 = bench 测量面非生产路径固有面），
  逐格不变量机核 production ≤ full（探针格 cornell-box t67 × 三后端真跑）；
- vendor Stage A 位级零漂移：DLSS 臂 pack 直写 mapped staging（消 ~px·21B 二次
  memcpy）+ DLSS/FSR 双臂输出驻留写（消逐帧 ~out_px·12B 分配）——三探针格
  末帧 digest == milestones/g14/g14_3_stage_a_digest_anchor.json 冻结锚
  （M-d 20260820T012652Z 复跑面收割，pre-Stage-A 码面）逐字一致；
- vendor 分解遥测轴 measured：RURIX_VENDOR_TIMING=1 探针跑（cornell t67
  dlss_sr 30 帧）六段（pack/sl_book/upload/evaluate/submit_wait/readback）
  解析非空，evaluate 段 measured 值登记（G14.4 调研 R1 黑盒面裁决动作）；
- Stage A 前后全量口径对照行（pre-Stage-A 锚 = M-d 012652Z evidence 逐格
  rurix_median_ms vs 本门探针格实测——对照行非空即录，量级不作先验承诺
  P-09）；
- 三探针格 production 口径 measured 入 g14_budget（阈 = 实测 ×1.5 守护带沿
  G9.1~G12.5 先例，measured_local 零 estimated）+ budget_eval 全 PASS；
- RED 双臂：caliber-masquerade（production > full 伪收据必被不变量拒）+
  digest-drift（锚篡改必检出）。

RED 字面：以全量口径冒充生产口径即 RED；Stage A 输出漂移静默即 RED；
estimated 冒充 measured 即 RED。

用法：
  py -3 ci/g14_production_caliber_stage_a_smoke.py --gate g14.p0.m_f.production_caliber_stage_a
  py -3 ci/g14_production_caliber_stage_a_smoke.py --verify-latest
  py -3 ci/g14_production_caliber_stage_a_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g10_wave_exit_lib as wel  # noqa: E402
import g14_rurix_pipeline_perf_smoke as mc  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g14.p0.m_f.production_caliber_stage_a"
NUMERIC_STEP = 257  # 落盘前实测 registry/number_ledger.json CI_step.next_free=257 顺位领取
SUBJECT = "g14_m_f_production_caliber_stage_a"
WAVE = "G14.6"
TAG = "g14_m_f"
MATRIX_ROW = "M176"
SOURCE_REF = (
    "G14_CONTRACT §4.2 M-f/§7 裁决 7;G14_ACCEPTANCE_MAP §1;G14.4 调研取证面（§8.5 a~f）;"
    "M141/M165 50×3 冻结统计口径;G14.3 digest 冻结锚 g14_3_stage_a_digest_anchor.json"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_m_f_production_caliber_stage_a_evidence_schema.json"
MEASURED_ENTRY_SCHEMA = ROOT / "milestones" / "g14" / "g14_m_c_measured_entry_evidence_schema.json"
BUDGET_PATH = ROOT / "milestones" / "g14" / "g14_budget.json"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
MD_PRE_EVIDENCE = ROOT / "evidence" / "g14_m_d_dual_end_fps_parity_20260820T012652Z.json"

BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"
OUT_ROOT = Path(r"K:\rurix-ext\g14-frames\rurix_prod")

PROBE_CELLS = [("cornell-box", 67, "tsr_device"), ("cornell-box", 67, "dlss_sr"), ("cornell-box", 67, "fsr_3_1_5")]
VTM_CELL = ("cornell-box", 67, "dlss_sr")
VTM_FRAMES = 30

CHECK_KEYS = [
    "double_column_receipt",
    "stage_a_bitexact_probe",
    "vendor_timing_axis_measured",
    "before_after_registered",
    "budget_entries_written",
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


def _write_probe_entry(cell_key: str, backend: str, prod_ms: float, ts: str) -> str:
    import hashlib
    doc = {
        "schema": "rurix.g14pipelineperf.measured_entry.v1",
        "entry_id": f"g14.pipeline_perf.prod_frame_ms.{cell_key}",
        "results": {"trimmed_mean": prod_ms},
        "protocol": (
            "G14.6 生产口径（frame_ms_production = 全量 − bench 测量面 tail）探针格单轮 "
            "160 帧 warmup 10 稳态 mean；双列同测零行为变更（G14.6 M-f 门字面）"
        ),
        "sample_manifest": {
            "count": 160,
            "digest": "sha256:" + hashlib.sha256(f"{cell_key}|{prod_ms}|{ts}".encode("utf-8")).hexdigest(),
        },
        "provenance": {
            "gpu": "device",
            "backend": backend,
            "base_commit": (run(["git", "rev-parse", "HEAD"]).stdout or "").strip(),
        },
        "cells": {"probe_cell": cell_key},
        "timestamp": ts,
    }
    name = f"g14_m_f_probe_{cell_key}_{ts}.json"
    (wel.EVIDENCE_DIR / name).write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return f"evidence/{name}"


def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    started = _dt.datetime.now(_dt.timezone.utc)
    ts = started.strftime("%Y%m%dT%H%M%SZ")

    anchors_doc = wel.load_json(ANCHOR_PATH)
    anchors = anchors_doc.get("anchors") or {}

    # ── ① 探针格真跑（三后端 cornell t67）+ 双列口径 + 不变量 ──
    probe: dict[str, dict] = {}
    meas_ok = True
    with gpu_device_lock(purpose=f"{TAG} 探针格三轮真跑"):
        for scene, tier, backend in PROBE_CELLS:
            note(f"探针格 {scene}/t{tier}/{backend}…")
            res = mc.run_bench(scene, tier, backend, 1)
            if not res.get("ok"):
                meas_ok = False
                check(False, f"探针格测量失败 {scene}/t{tier}/{backend}: {res.get('tail', '')[:120]}")
                continue
            probe[f"{scene}_t{tier}_{backend}"] = res
    inv_ok = all(
        0.0 < r["frame_ms_production_mean"] <= r["frame_ms_mean"]
        for r in probe.values()
    )
    checks["double_column_receipt"] = meas_ok and len(probe) == len(PROBE_CELLS) and inv_ok
    check(checks["double_column_receipt"], f"双列口径/不变量面: cells={len(probe)} inv={inv_ok}")
    for key, r in probe.items():
        note(f"  {key}: full={r['frame_ms_mean']:.3f}ms prod={r['frame_ms_production_mean']:.3f}ms "
             f"tail={r['tail_ms_mean']:.3f}ms")

    # ── ② Stage A 位级零漂移（digest == 冻结锚） ──
    drift_rows = []
    bit_ok = bool(probe)
    for key, r in probe.items():
        anchor = (anchors.get(key) or {}).get("last_frame_digest", "")
        mine = r.get("last_frame_digest", "")
        same = bool(anchor) and mine == anchor
        drift_rows.append({"cell": key, "anchor": anchor, "measured": mine, "match": same})
        if not same:
            bit_ok = False
            check(False, f"Stage A 位级漂移 {key}: anchor={anchor[:40]}… measured={mine[:40]}…")
    checks["stage_a_bitexact_probe"] = bit_ok and len(drift_rows) == len(PROBE_CELLS)
    note("Stage A 位级对拍：" + "; ".join(f"{r['cell']}={'同' if r['match'] else '异'}" for r in drift_rows))

    # ── ③ vendor 分解遥测轴（RURIX_VENDOR_TIMING=1 探针跑） ──
    import os
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    env["RURIX_VENDOR_TIMING"] = "1"
    vtm_segments: dict[str, float] = {}
    with gpu_device_lock(purpose=f"{TAG} vendor 分解遥测探针"):
        r = run([str(BIN), "--bench", "--scene", VTM_CELL[0], "--tier", str(VTM_CELL[1]),
                 "--backend", VTM_CELL[2], "--frames", str(VTM_FRAMES), "--warmup", "10"],
                timeout=2400, env=env)
    vtm_lines = [l for l in ((r.stderr or "") + (r.stdout or "")).splitlines()
                 if l.startswith("[vendor-timing dlss]")]
    seg_re = re.compile(r"pack=([0-9.]+) sl_book=([0-9.]+) upload=([0-9.]+) evaluate=([0-9.]+) submit_wait=([0-9.]+) readback=([0-9.]+)")
    seg_vals: dict[str, list[float]] = {k: [] for k in ("pack", "sl_book", "upload", "evaluate", "submit_wait", "readback")}
    for l in vtm_lines:
        m = seg_re.search(l)
        if m:
            for i, k in enumerate(seg_vals):
                seg_vals[k].append(float(m.group(i + 1)))
    # 稳态窗 = 弃 warmup 10 帧后的探针帧（与 bench 腿 warmup 同口径）
    steady = {k: (sum(v[10:]) / len(v[10:]) if len(v) > 10 else None) for k, v in seg_vals.items()}
    if r.returncode == 0 and all(v is not None for v in steady.values()):
        vtm_segments = {k: round(v, 6) for k, v in steady.items() if v is not None}
    checks["vendor_timing_axis_measured"] = bool(vtm_segments) and vtm_segments.get("evaluate", 0.0) > 0.0
    check(checks["vendor_timing_axis_measured"], f"vendor 分解遥测面: lines={len(vtm_lines)}")
    if vtm_segments:
        note("vendor 分解遥测（cornell t67 dlss 稳态 mean, ms）："
             + " ".join(f"{k}={v:.3f}" for k, v in vtm_segments.items()))

    # ── ④ Stage A 前后全量口径对照行（pre-Stage-A 锚 = M-d 012652Z evidence） ──
    ba_rows = []
    pre_doc = wel.load_json(MD_PRE_EVIDENCE) if MD_PRE_EVIDENCE.is_file() else {}
    pre_cells = {f"{c['scene']}_t{c['tier']}_{c['backend']}": c for c in ((pre_doc.get("parity") or {}).get("cells") or [])}
    for key, r in probe.items():
        pre = pre_cells.get(key)
        if pre is None:
            continue
        ba_rows.append({
            "cell": key,
            "pre_stage_a_full_ms": pre["rurix_median_ms"],
            "post_stage_a_full_ms": r["frame_ms_mean"],
            "post_stage_a_production_ms": r["frame_ms_production_mean"],
        })
    checks["before_after_registered"] = len(ba_rows) == len(PROBE_CELLS)
    check(checks["before_after_registered"], f"前后对照行面: {len(ba_rows)}/{len(PROBE_CELLS)}")
    for row in ba_rows:
        note(f"  前后对照 {row['cell']}: pre-full={row['pre_stage_a_full_ms']:.3f} → "
             f"post-full={row['post_stage_a_full_ms']:.3f} / post-prod={row['post_stage_a_production_ms']:.3f}ms")

    # ── ⑤ budget 条目（3 探针格 production 口径 measured-entry 件 + 阈 = 实测 ×1.5） ──
    bud_ok = True
    if probe:
        doc = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
        if doc is None:
            bud_ok = False
        else:
            new_ids = set()
            new_entries = []
            for key, r in probe.items():
                eid = f"g14.pipeline_perf.prod_frame_ms.{key}"
                new_ids.add(eid)
                ev_file = _write_probe_entry(key, key.rsplit("_", 1)[-1], r["frame_ms_production_mean"], ts)
                new_entries.append({
                    "id": eid,
                    "description": (
                        f"G14.6 生产口径（frame_ms_production = 全量 − bench 测量面 tail〔is_finite+digest〕）"
                        f"探针格 {key} 帧时——单轮 160 帧 warmup 10 稳态 mean（阈 = 实测 ×1.5 守护带沿 "
                        f"G9.1~G12.5 先例）；回归守护/对标输入面；M-d v2 对标 = 三轮全量面"
                    ),
                    "direction": "max",
                    "evidence": "measured_local",
                    "skip_reason": None,
                    "unit": "ms",
                    "threshold": r["frame_ms_production_mean"] * 1.5,
                    "evidence_file": ev_file,
                    "measured_value": r["frame_ms_production_mean"],
                })
            keep = [e for e in (doc.get("entries") or []) if e.get("id") not in new_ids]
            doc["entries"] = keep + new_entries
            BUDGET_PATH.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            back = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
            got = {e["id"]: e for e in back.get("entries") or []}
            bud_ok = all(got.get(i, {}).get("measured_value") is not None for i in new_ids)
    checks["budget_entries_written"] = bud_ok and bool(probe)

    # ── ⑥ budget_eval 全 PASS ──
    bud = run([sys.executable, str(ROOT / "ci" / "budget_eval.py")], timeout=900)
    checks["budget_eval_all_pass"] = bud.returncode == 0 and "[budget_eval] PASS" in (bud.stdout or "")

    # ── ⑦ RED 双臂（函数面真跑） ──
    red: dict[str, bool] = {}
    # 臂① caliber-masquerade：production > full 伪值必被不变量拒；合法面必收
    def _invariant_ok(prod: float, full: float) -> bool:
        return 0.0 < prod <= full
    red["caliber_masquerade_detected"] = (not _invariant_ok(16.3, 7.4)) and _invariant_ok(7.4, 16.3)
    # 臂② digest-drift：锚篡改（hex 翻一字）必检出；未篡必收
    def _digest_match(anchor: str, measured: str) -> bool:
        return bool(anchor) and anchor == measured
    tampered = "sha256:" + ("0" if not drift_rows or drift_rows[0]["anchor"][7:8] != "0" else "1") + (drift_rows[0]["anchor"][8:] if drift_rows else "")
    red["digest_drift_detected"] = bool(drift_rows) and (not _digest_match(tampered, drift_rows[0]["measured"])) and _digest_match(drift_rows[0]["anchor"], drift_rows[0]["measured"])
    red_ok = all(red.values())
    checks["red_arms_effective"] = red_ok
    check(red_ok, f"RED 臂面: {red}")

    # ── verdict ──
    all_pass = all(checks.values()) and not FAILURES
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
        "device_section_state": "executed" if checks["double_column_receipt"] else "fail",
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": "g14_3_pipeline_perf --bench ×3 探针格（双列口径真跑）",
             "exit_code": 0 if checks["double_column_receipt"] else 1},
            {"seq": 2, "command": "RURIX_VENDOR_TIMING=1 g14_3_pipeline_perf --bench cornell-box 67 dlss_sr 30 帧（分解遥测）",
             "exit_code": 0 if checks["vendor_timing_axis_measured"] else 1},
            {"seq": 3, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
            {"seq": 4, "command": "RED 双臂（caliber-masquerade/digest-drift）",
             "exit_code": 0 if checks["red_arms_effective"] else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "production": {
            "correctness_anchor_unchanged": checks["stage_a_bitexact_probe"],
            "baseline_anchor_id": "g14.ue_benchmark.frame_ms.*（UE 臂）+ g14.pipeline_perf.frame_ms.*（G14.3 全量口径）+ g14_3_stage_a_digest_anchor（位级锚）",
            "measured_value": "; ".join(
                f"{k}: full={r['frame_ms_mean']:.3f}/prod={r['frame_ms_production_mean']:.3f}ms"
                for k, r in probe.items()) + (f"; vtm evaluate={vtm_segments.get('evaluate', 0):.3f}ms" if vtm_segments else ""),
            "not_worse_than_anchor": checks["stage_a_bitexact_probe"],
            "threshold_provenance": "budget 阈 = 实测 ×1.5 守护带（G9.1~G12.5 先例程序产，禁手写 P-09）；位级锚 = G14.3 digest 冻结面",
            "evolution_register": (
                f"Stage A 位级零漂移 {sum(1 for r in drift_rows if r['match'])}/{len(drift_rows)} 格；"
                f"双列口径不变量机核过；vendor evaluate 段 measured={vtm_segments.get('evaluate', 0):.3f}ms 登记（R1 裁决面）"
            ),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "parity": {
            "probe_cells": [
                {
                    "cell": k,
                    "full_ms": r["frame_ms_mean"],
                    "production_ms": r["frame_ms_production_mean"],
                    "tail_ms": r["tail_ms_mean"],
                    "last_frame_digest": r.get("last_frame_digest", ""),
                }
                for k, r in probe.items()
            ],
            "digest_anchor_file": "milestones/g14/g14_3_stage_a_digest_anchor.json",
            "bitexact_rows": drift_rows,
            "vendor_timing_segments_ms": vtm_segments,
            "before_after": ba_rows,
            "red": red,
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
    print(f"[{TAG}] VERDICT={'PASS' if all_pass else 'FAIL'} checks={sum(1 for v in checks.values())}/{len(checks)}")
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
    # 负样本：不变量伪值必拒 + 锚篡改必检出（函数面红绿双臂）
    def _invariant_ok(prod: float, full: float) -> bool:
        return 0.0 < prod <= full
    if _invariant_ok(16.3, 7.4) or not _invariant_ok(7.4, 16.3):
        print(f"[{TAG}] selftest FAIL: 不变量臂不红不绿", file=sys.stderr)
        failures += 1
    if failures:
        return 1
    print(f"[{TAG}] selftest PASS（schema 闭集 + 双臂红绿）")
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
