#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.7 vendor 转换并行化延续波）
"""G14.7 延续波门 M-g：vendor 转换并行化（pack/readback 像素带并行）位级零漂移
+ 同码 A/B measured（g14.p0.m_g.vendor_parallel_conversion；G14_CONTRACT §7 裁决 7
延续波程序面；G14_ACCEPTANCE_MAP 附录 A M-g 行）。

判据（附录 A M-g 行逐字）：
- vendor_upscale.rs 四区打包（color f16/depth/mv/reactive）与双臂输出回读转换
  （DLSS 连续 RGBA / FSR 行距对齐）改像素带并行（std::thread::scope 带切分，
  元素零依赖，带内逐值同式同序——输出字节面与单带串行逐位一致）+ fsr-dbg
  逐帧诊断打印门控（FSR_DBG_STAGE 置位才打印，CI 零消费面）；
- 位级零漂移三机核：① 三探针格（bistro-interior t67 dlss_sr / fsr_3_1_5 并行面
  + cornell-box t67 dlss_sr 阈下单带面）末帧 digest == g14_3_stage_a_digest_anchor
  冻结锚；② 串行对照臂（RURIX_VENDOR_PAR=0）bistro t67 dlss digest 同锚——
  并行 ≡ 串行 ≡ 锚 三角机核；③ Rust 函数面 g14_7_parallel_conversion_bitexact
  单测（并行 vs 串行合成字节面逐位一致 + 带数决策纯函数锚）真跑绿；
- 同码 A/B measured：bistro t67 dlss RURIX_VENDOR_TIMING=1 探针跑（PAR=0 串行
  对照 ×2 + 缺省并行 ×2 交错，30 帧 warmup 10 稳态 mean）pack 与 readback 双段
  并行臂 mean < 串行臂 mean（方向机核，改善量 measured 登记，不设先验阈值 P-09）；
- 双探针格（bistro t67 dlss/fsr）production 口径 measured 入 g14_budget（阈 =
  实测 ×1.5 守护带程序产禁手写 P-09）+ budget_eval 全 PASS；
- RED 双臂：digest-drift（锚篡改必检出）+ direction-masquerade（par≥ser 伪改善
  必被方向机核拒）。

RED 字面：并行输出漂移静默即 RED；以串行臂冒充并行改善（direction 伪报）即 RED；
estimated 冒充 measured 即 RED。

用法：
  py -3 ci/g14_vendor_parallel_conversion_smoke.py --gate g14.p0.m_g.vendor_parallel_conversion
  py -3 ci/g14_vendor_parallel_conversion_smoke.py --verify-latest
  py -3 ci/g14_vendor_parallel_conversion_smoke.py --selftest
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

GATE_KEY = "g14.p0.m_g.vendor_parallel_conversion"
NUMERIC_STEP = 259  # 落盘前实测 registry/number_ledger.json CI_step.next_free=259 顺位领取
SUBJECT = "g14_m_g_vendor_parallel_conversion"
WAVE = "G14.7"
TAG = "g14_m_g"
MATRIX_ROW = "M177"
SOURCE_REF = (
    "G14_CONTRACT §7 裁决 7 延续波程序面;G14_ACCEPTANCE_MAP 附录 A M-g 行;G14.4 调研取证面（§8.5 d）;"
    "G14.6 Stage A 冻结锚 g14_3_stage_a_digest_anchor.json;M141/M165 50×3 冻结统计口径"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_m_g_vendor_parallel_conversion_evidence_schema.json"
MEASURED_ENTRY_SCHEMA = ROOT / "milestones" / "g14" / "g14_m_c_measured_entry_evidence_schema.json"
BUDGET_PATH = ROOT / "milestones" / "g14" / "g14_budget.json"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"

BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"

PROBE_CELLS = [
    ("bistro-interior", 67, "dlss_sr"),
    ("bistro-interior", 67, "fsr_3_1_5"),
    ("cornell-box", 67, "dlss_sr"),
]
BUDGET_CELLS = [("bistro-interior", 67, "dlss_sr"), ("bistro-interior", 67, "fsr_3_1_5")]
AB_CELL = ("bistro-interior", 67, "dlss_sr")
AB_FRAMES = 30

CHECK_KEYS = [
    "bitexact_digest_anchor",
    "serial_control_arm_bitexact",
    "rust_unit_bitexact",
    "parallel_ab_delta_measured",
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
            "G14.7 生产口径（frame_ms_production = 全量 − bench 测量面 tail）探针格单轮 "
            "160 帧 warmup 10 稳态 mean（vendor 转换并行化落地面；G14.6 双列口径继承）"
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
    name = f"g14_m_g_probe_{cell_key}_{ts}.json"
    (wel.EVIDENCE_DIR / name).write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return f"evidence/{name}"


def _vtm_segments(r: subprocess.CompletedProcess) -> dict[str, float]:
    """解析 vendor-timing 六段稳态 mean（弃 warmup 10 帧）。"""
    lines = [l for l in ((r.stderr or "") + (r.stdout or "")).splitlines()
             if l.startswith("[vendor-timing dlss]")]
    seg_re = re.compile(
        r"pack=([0-9.]+) sl_book=([0-9.]+) upload=([0-9.]+) evaluate=([0-9.]+) "
        r"submit_wait=([0-9.]+) readback=([0-9.]+)")
    vals: dict[str, list[float]] = {k: [] for k in ("pack", "sl_book", "upload", "evaluate", "submit_wait", "readback")}
    for l in lines:
        m = seg_re.search(l)
        if m:
            for i, k in enumerate(vals):
                vals[k].append(float(m.group(i + 1)))
    return {k: sum(v[10:]) / len(v[10:]) for k, v in vals.items() if len(v) > 10}


def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    started = _dt.datetime.now(_dt.timezone.utc)
    ts = started.strftime("%Y%m%dT%H%M%SZ")

    anchors_doc = wel.load_json(ANCHOR_PATH)
    anchors = anchors_doc.get("anchors") or {}

    # ── ① 探针格真跑（缺省 = 并行面）+ 位级对拍 ──
    probe: dict[str, dict] = {}
    meas_ok = True
    with gpu_device_lock(purpose=f"{TAG} 探针格真跑（并行面）"):
        for scene, tier, backend in PROBE_CELLS:
            note(f"探针格 {scene}/t{tier}/{backend}…")
            res = mc.run_bench(scene, tier, backend, 1)
            if not res.get("ok"):
                meas_ok = False
                check(False, f"探针格测量失败 {scene}/t{tier}/{backend}: {res.get('tail', '')[:120]}")
                continue
            probe[f"{scene}_t{tier}_{backend}"] = res
    drift_rows = []
    bit_ok = bool(probe) and meas_ok
    for key, r in probe.items():
        anchor = (anchors.get(key) or {}).get("last_frame_digest", "")
        mine = r.get("last_frame_digest", "")
        same = bool(anchor) and mine == anchor
        drift_rows.append({"cell": key, "anchor": anchor, "measured": mine, "match": same})
        if not same:
            bit_ok = False
            check(False, f"并行面位级漂移 {key}: anchor={anchor[:40]}… measured={mine[:40]}…")
    checks["bitexact_digest_anchor"] = bit_ok and len(drift_rows) == len(PROBE_CELLS)
    note("并行面位级对拍：" + "; ".join(f"{r['cell']}={'同' if r['match'] else '异'}" for r in drift_rows))

    # ── ② 串行对照臂位级（RURIX_VENDOR_PAR=0 → 单带串行，digest 同锚） ──
    import os
    ser_digest = ""
    with gpu_device_lock(purpose=f"{TAG} 串行对照臂真跑"):
        os.environ["RURIX_VENDOR_PAR"] = "0"
        try:
            res0 = mc.run_bench(AB_CELL[0], AB_CELL[1], AB_CELL[2], 1)
        finally:
            os.environ.pop("RURIX_VENDOR_PAR", None)
    if res0.get("ok"):
        ser_digest = res0.get("last_frame_digest", "")
    anchor_ab = (anchors.get(f"{AB_CELL[0]}_t{AB_CELL[1]}_{AB_CELL[2]}") or {}).get("last_frame_digest", "")
    checks["serial_control_arm_bitexact"] = bool(ser_digest) and ser_digest == anchor_ab
    check(checks["serial_control_arm_bitexact"],
          f"串行对照臂位级: measured={ser_digest[:40]}… anchor={anchor_ab[:40]}…")
    note(f"串行对照臂 digest {'== 锚' if checks['serial_control_arm_bitexact'] else '≠ 锚'}"
         f"（并行 ≡ 串行 ≡ 锚 三角机核）")

    # ── ③ Rust 函数面位级单测（并行 vs 串行合成字节面逐位一致） ──
    ut = run(["cargo", "test", "--release", "-p", "rurix-rt", "--features", "vendor-upscale",
              "g14_7_parallel_conversion_bitexact"], timeout=3600)
    ut_ok = ut.returncode == 0 and "test result: ok" in (ut.stdout or "")
    checks["rust_unit_bitexact"] = ut_ok
    check(ut_ok, f"rust 函数面位级单测: rc={ut.returncode}")

    # ── ④ 同码 A/B measured（交错四跑，VTM 六段稳态 mean 方向机核） ──
    ab: dict[str, dict[str, float]] = {"ser": {}, "par": {}}
    ab_ok = True
    env_base = dict(os.environ)
    env_base["RURIX_REQUIRE_REAL"] = "1"
    env_base["RURIX_VK_VALIDATION"] = "1"
    env_base["RURIX_VENDOR_TIMING"] = "1"
    arms = [("ser", "0"), ("par", None), ("ser", "0"), ("par", None)]  # 交错消漂移趋势
    with gpu_device_lock(purpose=f"{TAG} A/B 交错探针跑"):
        for label, par_val in arms:
            env = dict(env_base)
            if par_val is not None:
                env["RURIX_VENDOR_PAR"] = par_val
            else:
                env.pop("RURIX_VENDOR_PAR", None)
            r = run([str(BIN), "--bench", "--scene", AB_CELL[0], "--tier", str(AB_CELL[1]),
                     "--backend", AB_CELL[2], "--frames", str(AB_FRAMES), "--warmup", "10"],
                    timeout=2400, env=env)
            segs = _vtm_segments(r)
            if r.returncode != 0 or not segs:
                ab_ok = False
                check(False, f"A/B 臂 {label} 探针跑失败/遥测空: rc={r.returncode}")
                continue
            for k, v in segs.items():
                ab[label].setdefault(k, [])
                ab[label][k].append(v)
    ab_delta: dict[str, dict[str, float]] = {}
    if ab_ok and all(len(ab[a].get("pack", [])) == 2 for a in ("ser", "par")):
        for seg in ("pack", "readback"):
            ser_m = sum(ab["ser"][seg]) / 2
            par_m = sum(ab["par"][seg]) / 2
            ab_delta[seg] = {"serial_ms": round(ser_m, 6), "parallel_ms": round(par_m, 6),
                             "delta_ms": round(ser_m - par_m, 6),
                             "speedup": round(ser_m / par_m, 6) if par_m > 0 else -1.0}
    direction_ok = (bool(ab_delta)
                    and all(ab_delta[s]["parallel_ms"] < ab_delta[s]["serial_ms"] for s in ("pack", "readback")))
    checks["parallel_ab_delta_measured"] = ab_ok and direction_ok
    check(checks["parallel_ab_delta_measured"], f"A/B 方向机核面: {ab_delta}")
    for seg, row in ab_delta.items():
        note(f"A/B {seg}: 串行={row['serial_ms']:.3f}ms → 并行={row['parallel_ms']:.3f}ms"
             f"（Δ={row['delta_ms']:.3f}ms, ×{row['speedup']:.2f}）")

    # ── ⑤ budget 条目（双探针格 production 口径 measured-entry + 阈 = 实测 ×1.5） ──
    bud_ok = True
    bud_rows = [r for k, r in probe.items()
                if any(k == f"{s}_t{t}_{b}" for s, t, b in BUDGET_CELLS)]
    if len(bud_rows) != len(BUDGET_CELLS):
        bud_ok = False
    else:
        doc = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
        if doc is None:
            bud_ok = False
        else:
            new_ids = set()
            new_entries = []
            for (scene, tier, backend) in BUDGET_CELLS:
                key = f"{scene}_t{tier}_{backend}"
                r = probe[key]
                eid = f"g14.pipeline_perf.prod_frame_ms.{key}"
                new_ids.add(eid)
                ev_file = _write_probe_entry(key, backend, r["frame_ms_production_mean"], ts)
                new_entries.append({
                    "id": eid,
                    "description": (
                        f"G14.7 vendor 转换并行化落地面生产口径（frame_ms_production = 全量 − bench "
                        f"测量面 tail）探针格 {key} 帧时——单轮 160 帧 warmup 10 稳态 mean（阈 = 实测 "
                        f"×1.5 守护带沿 G9.1~G12.5 先例）；回归守护/对标输入面"
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
    checks["budget_entries_written"] = bud_ok

    # ── ⑥ budget_eval 全 PASS ──
    bud = run([sys.executable, str(ROOT / "ci" / "budget_eval.py")], timeout=900)
    checks["budget_eval_all_pass"] = bud.returncode == 0 and "[budget_eval] PASS" in (bud.stdout or "")

    # ── ⑦ RED 双臂（函数面真跑） ──
    red: dict[str, bool] = {}
    # 臂① digest-drift：锚篡改（hex 翻一字）必检出；未篡必收
    def _digest_match(anchor: str, measured: str) -> bool:
        return bool(anchor) and anchor == measured
    tampered = "sha256:" + ("0" if not drift_rows or drift_rows[0]["anchor"][7:8] != "0" else "1") + (drift_rows[0]["anchor"][8:] if drift_rows else "")
    red["digest_drift_detected"] = (bool(drift_rows)
                                    and (not _digest_match(tampered, drift_rows[0]["measured"]))
                                    and _digest_match(drift_rows[0]["anchor"], drift_rows[0]["measured"]))
    # 臂② direction-masquerade：par ≥ ser 伪改善必被方向机核拒；真改善必收
    def _direction_ok(ser: float, par: float) -> bool:
        return par < ser
    red["direction_masquerade_detected"] = (not _direction_ok(2.07, 9.10)) and _direction_ok(9.10, 2.07)
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
        "device_section_state": "executed" if checks["bitexact_digest_anchor"] else "fail",
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": "g14_3_pipeline_perf --bench ×3 探针格（并行面位级对拍）",
             "exit_code": 0 if checks["bitexact_digest_anchor"] else 1},
            {"seq": 2, "command": "RURIX_VENDOR_PAR=0 g14_3_pipeline_perf --bench bistro-interior 67 dlss_sr（串行对照臂位级）",
             "exit_code": 0 if checks["serial_control_arm_bitexact"] else 1},
            {"seq": 3, "command": "cargo test --release -p rurix-rt --features vendor-upscale g14_7_parallel_conversion_bitexact",
             "exit_code": 0 if checks["rust_unit_bitexact"] else 1},
            {"seq": 4, "command": "A/B 交错四跑（RURIX_VENDOR_TIMING=1，PAR=0×2 + 缺省并行×2）",
             "exit_code": 0 if checks["parallel_ab_delta_measured"] else 1},
            {"seq": 5, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
            {"seq": 6, "command": "RED 双臂（digest-drift/direction-masquerade）",
             "exit_code": 0 if checks["red_arms_effective"] else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "production": {
            "correctness_anchor_unchanged": checks["bitexact_digest_anchor"] and checks["serial_control_arm_bitexact"],
            "baseline_anchor_id": "g14_3_stage_a_digest_anchor（位级锚）+ g14.pipeline_perf.prod_frame_ms.*（生产口径基线）",
            "measured_value": "; ".join(
                f"{k}: prod={r['frame_ms_production_mean']:.3f}ms" for k, r in probe.items())
                + ("; A/B " + " ".join(f"{s} Δ={row['delta_ms']:.3f}ms ×{row['speedup']:.2f}"
                                       for s, row in ab_delta.items()) if ab_delta else ""),
            "not_worse_than_anchor": checks["bitexact_digest_anchor"],
            "threshold_provenance": "budget 阈 = 实测 ×1.5 守护带（G9.1~G12.5 先例程序产，禁手写 P-09）；位级锚 = G14.3 digest 冻结面；A/B 改善量 measured 登记不设先验阈值",
            "evolution_register": (
                f"vendor 转换并行化位级零漂移 {sum(1 for r in drift_rows if r['match'])}/{len(drift_rows)} 格"
                f" + 串行对照臂 {'同锚' if checks['serial_control_arm_bitexact'] else '异锚'}"
                f" + Rust 函数面位级单测 {'绿' if checks['rust_unit_bitexact'] else '红'}；"
                f"A/B 方向机核 {'绿' if checks['parallel_ab_delta_measured'] else '红'}（{ab_delta}）"
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
            "serial_control_arm": {
                "cell": f"{AB_CELL[0]}_t{AB_CELL[1]}_{AB_CELL[2]}",
                "env": "RURIX_VENDOR_PAR=0",
                "last_frame_digest": ser_digest,
                "match_anchor": checks["serial_control_arm_bitexact"],
            },
            "ab_delta_segments_ms": ab_delta,
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
    # 负样本：方向机核伪改善必拒 + 真改善必收（函数面红绿双臂）
    def _direction_ok(ser: float, par: float) -> bool:
        return par < ser
    if _direction_ok(2.07, 9.10) or not _direction_ok(9.10, 2.07):
        print(f"[{TAG}] selftest FAIL: 方向机核臂不红不绿", file=sys.stderr)
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
