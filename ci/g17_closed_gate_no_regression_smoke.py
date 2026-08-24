#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Claude Fable 5（G17.6 M-e 旧门零降级波）
"""G17.6 P0 硬门 M-e：已收口门零降级
（g17.p0.m_e.closed_gate_no_regression；G17_CONTRACT §4.2 M-e/G-G17-7；
G17_ACCEPTANCE_MAP §1 M-e 行）。

判据（契约 §4.2 M-e 逐字）：G13/G14/G15/G16 受影响门 `--verify-latest` 全绿零降级；
`g17_` 前缀不抢旧门 latest；禁对旧脚本发 `--gate`。

用法：
  py -3 ci/g17_closed_gate_no_regression_smoke.py --gate g17.p0.m_e.closed_gate_no_regression
  py -3 ci/g17_closed_gate_no_regression_smoke.py --verify-latest
  py -3 ci/g17_closed_gate_no_regression_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g17.p0.m_e.closed_gate_no_regression"
NUMERIC_STEP = 304  # post-interlock 实测顺位领取
SUBJECT = "g17_m_e_closed_gate_no_regression"
WAVE = "G17.6"
SCHEMA_PATH = ROOT / "milestones/g17/g17_m_e_closed_gate_no_regression_evidence_schema.json"
SOURCE_REF = "G17_CONTRACT §4.2 M-e/G-G17-7;G17_ACCEPTANCE_MAP §1 M-e 行"

# 有 --verify-latest 旗的受影响门走子进程（G16 全套——G16.5 M-d 先例扩 G16 门）。
VERIFY_SCRIPTS = [
    ("g16_ue_reference_arm_repair", "ci/g16_ue_reference_arm_repair_smoke.py"),
    ("g16_dual_end_reharvest", "ci/g16_dual_end_reharvest_smoke.py"),
    ("g16_absolute_quality_rereview", "ci/g16_absolute_quality_rereview_smoke.py"),
    ("g16_closed_gate_no_regression", "ci/g16_closed_gate_no_regression_smoke.py"),
    ("g16_gi_expression", "ci/g16_gi_expression_smoke.py"),
    ("g16_lumen_reharvest", "ci/g16_lumen_reharvest_smoke.py"),
    ("g16_absolute_quality_closure", "ci/g16_absolute_quality_closure_smoke.py"),
    ("g16_stabilization_soak", "ci/g16_stabilization_soak.py"),
    ("g16_closeout", "ci/g16_closeout_check.py"),
]
# 无该旗/诚实红登记面的旧门只读 latest evidence（G16.5 M-d _latest_ok 口径）。
VERIFY_EVIDENCE = [
    ("g13_ue_upscale_parity", "g13_m_c_ue_upscale_parity"),
    ("g13_ue_lumen_gi_parity", "g13_m_d_ue_lumen_gi_parity"),
    ("g13_closeout", "g13_wave5b_closeout"),
    ("g15_closeout", "g15_wave6b_closeout"),
    ("g15_regression_drift_guard", "g15_m_e_regression_drift_guard"),
    ("g14_regression_drift_guard", "g14_m_e_regression_drift_guard"),
]
# 诚实红终态维持面：G15 M-d 门（g15.p0.m_d.perf_parity_zero_regression）历史终态 =
# 诚实红定盘件（G15 §8.7 六红键，G15 closeout 在此红面下 READY）——零降级判定 =
# latest 件名维持该定盘件字面（G17 期零新件抢占，红终态 0-byte 不遮蔽不代绿）。
HONEST_RED_TERMINAL = [
    ("g15_perf_parity_guard_terminal", "g15_m_d_perf_parity_zero_regression",
     "g15_m_d_perf_parity_zero_regression_20260823T195859Z.json"),
]
# 旧门 latest 前缀不得被 g17_ 件抢占。
PREFIX_GUARD = [
    ("g14_m_d_dual_end_fps_parity", "g14_m_d_dual_end_fps_parity"),
    ("g16_m_g_absolute_quality_closure", "g16_m_g_absolute_quality_closure"),
    ("g16_wave6b_closeout", "g16_wave6b_closeout"),
    ("g15_m_c_absolute_quality_final_review", "g15_m_c_absolute_quality_final_review"),
]


def _verify(script: str) -> tuple[bool, str]:
    r = subprocess.run(
        [sys.executable, str(ROOT / script), "--verify-latest"],
        cwd=ROOT, capture_output=True, text=True,
    )
    tail = ((r.stdout or "") + (r.stderr or ""))[-200:].replace("\n", " ")
    return r.returncode == 0, f"rc={r.returncode} {tail}"


def _latest_ok(prefix: str) -> tuple[bool, str]:
    p = wel.load_latest_evidence(prefix)
    if p is None:
        return False, f"{prefix}: missing"
    doc = wel.load_json(p)
    status = str(doc.get("status") or "").lower()
    verdict = str(doc.get("verdict") or doc.get("VERDICT") or "").upper()
    host = doc.get("host_section_pass")
    ok = status in ("pass", "ready") or verdict in ("READY", "PASS") or host is True
    if status == "fail" and host is True:
        ok = True  # 诚实红登记面（G14 M-d 类）不记降级
    return ok, f"{p.name} status={status!r} verdict={verdict!r} host={host}"


def evaluate() -> list[dict]:
    facts: list[dict] = []
    for name, script in VERIFY_SCRIPTS:
        ok, detail = _verify(script)
        facts.append({"id": f"verify_{name}", "status": "PASS" if ok else "FAIL", "detail": detail})
    for name, prefix in VERIFY_EVIDENCE:
        ok, detail = _latest_ok(prefix)
        facts.append({"id": f"verify_{name}", "status": "PASS" if ok else "FAIL", "detail": detail})
    for name, prefix, terminal_file in HONEST_RED_TERMINAL:
        p = wel.load_latest_evidence(prefix)
        ok = p is not None and p.name == terminal_file
        facts.append({
            "id": f"verify_{name}",
            "status": "PASS" if ok else "FAIL",
            "detail": f"latest = {p.name if p else 'missing'}（诚实红定盘件 {terminal_file} 维持 latest"
                      f" = 零降级；红终态 0-byte 不遮蔽不代绿——G15 §8.7 定盘字面）"
            if ok else f"latest = {p.name if p else 'missing'} ≠ 定盘件 {terminal_file}（终态被抢占/缺失）",
        })
    stolen = []
    for label, prefix in PREFIX_GUARD:
        p = wel.load_latest_evidence(prefix)
        if p is None:
            stolen.append(f"{label}:missing")
            continue
        if p.name.startswith("g17_"):
            stolen.append(f"{label}:stolen_by {p.name}")
    facts.append({"id": "latest_prefix_not_stolen", "status": "PASS" if not stolen else "FAIL",
                  "detail": "ok（g17_ 前缀零抢占）" if not stolen else "; ".join(stolen)})
    facts.append({"id": "no_gate_invoked_on_old_scripts", "status": "PASS",
                  "detail": "本门只发 --verify-latest，零 --gate 旧脚本（禁 --gate 字面）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    overall = all(f["status"] == "PASS" for f in facts)
    if not SCHEMA_PATH.is_file():
        print(f"[g17_m_e] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=[],
        extra_facts=facts,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G17.6 M-e：G13/G14/G15/G16 受影响门 --verify-latest 全绿零降级（9 子进程 + "
              "6 latest 只读 + 1 诚实红终态维持〔G15 M-d 定盘件 latest 字面 = 零降级，红终态"
              "不遮蔽不代绿〕+ 前缀守护 + 禁 --gate 声明 = 18 facts）；g17_ 前缀不抢旧 latest",
        host_section_pass=overall,
    )
    return 0 if (overall and code == 0) else 1


def verify_latest() -> int:
    p = wel.load_latest_evidence(SUBJECT)
    if p is None:
        print(f"[g17_m_e] verify-latest FAIL: 无 {SUBJECT} evidence", file=sys.stderr)
        return 1
    doc = wel.load_json(p)
    ok = doc.get("host_section_pass") is True and all(
        f.get("status") == "PASS" for f in doc.get("extra_facts", [])
    )
    print(f"[g17_m_e] verify-latest {'PASS' if ok else 'FAIL'}: {p.name}")
    return 0 if ok else 1


def run_selftest() -> int:
    n = len(VERIFY_SCRIPTS) + len(VERIFY_EVIDENCE) + len(HONEST_RED_TERMINAL) + 2
    ok = n == 18
    missing = [s for _, s in VERIFY_SCRIPTS if not (ROOT / s).is_file()]
    if missing:
        print(f"  SCRIPT MISS — {missing}")
        ok = False
    print(f"  {'CLOSURE ok' if ok else 'CLOSURE FAIL'} — facts 闭集 {n}（期望 18）+ 子进程脚本全在树")
    print(f"[g17_m_e] SELFTEST {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


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
        return verify_latest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
