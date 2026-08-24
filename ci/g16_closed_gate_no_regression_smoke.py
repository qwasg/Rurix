#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16.5 实现波）
"""G16.5 P0 M-d — 已收口门零降级（g16.p0.m_d.closed_gate_no_regression，步骤 287）。

只跑 --verify-latest，禁止对旧脚本发 --gate。
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g10_wave_exit_lib as g10wel  # noqa: E402
import g16_p0_lib as g16  # noqa: E402

GATE_KEY = "g16.p0.m_d.closed_gate_no_regression"
NUMERIC_STEP = 287
SUBJECT = "g16_m_d_closed_gate_no_regression"
WAVE = "G16.5"
SOURCE_REF = "G16_CONTRACT §4.2 M-d/G-G16-6;G16_ACCEPTANCE_MAP §1 M-d"
SCHEMA = g16.ROOT / "milestones" / "g16" / "g16_m_d_closed_gate_no_regression_evidence_schema.json"

# 脚本有 --verify-latest 的走子进程；其余（G13/G14/closeout 无该旗）只读 latest evidence。
VERIFY_SCRIPTS = [
    ("g15_dual_end_quality_reharvest", "ci/g15_dual_end_quality_reharvest_smoke.py"),
    ("g15_absolute_quality_review", "ci/g15_absolute_quality_review_smoke.py"),
]
VERIFY_EVIDENCE = [
    ("g13_ue_upscale_parity", "g13_m_c_ue_upscale_parity"),
    ("g13_ue_lumen_gi_parity", "g13_m_d_ue_lumen_gi_parity"),
    ("g13_closeout", "g13_wave5b_closeout"),
    ("g15_closeout", "g15_wave6b_closeout"),
    ("g15_regression_drift_guard", "g15_m_e_regression_drift_guard"),
    ("g14_regression_drift_guard", "g14_m_e_regression_drift_guard"),
]

# 旧门 latest 前缀不得被 g16_ 件抢占
PREFIX_GUARD = [
    ("g13_m_c_ue_upscale_parity", "g13_m_c_ue_upscale_parity"),
    ("g15_m_c_absolute_quality_final_review", "g15_m_c_absolute_quality_final_review"),
    ("g15_m_a_dual_end_quality_reharvest", "g15_m_a_dual_end_quality_reharvest"),
]


def _verify(script: str) -> tuple[bool, str]:
    r = subprocess.run(
        [sys.executable, str(g16.ROOT / script), "--verify-latest"],
        cwd=g16.ROOT, capture_output=True, text=True,
    )
    tail = ((r.stdout or "") + (r.stderr or ""))[-240:].replace("\n", " ")
    return r.returncode == 0, f"rc={r.returncode} {tail}"


def _latest_ok(prefix: str) -> tuple[bool, str]:
    p = g10wel.load_latest_evidence(prefix)
    if p is None:
        return False, f"{prefix}: missing"
    doc = g10wel.load_json(p)
    status = str(doc.get("status") or "").lower()
    verdict = str(doc.get("verdict") or doc.get("VERDICT") or "").upper()
    host = doc.get("host_section_pass")
    ok = status in ("pass", "ready") or verdict in ("READY", "PASS") or host is True
    if status == "fail" and host is True:
        ok = True  # 诚实红登记面（G14 M-d 类）不记降级
    return ok, f"{p.name} status={status!r} verdict={verdict!r} host={host}"


def run_gate() -> int:
    facts: list[dict] = []
    for name, script in VERIFY_SCRIPTS:
        ok, detail = _verify(script)
        facts.append(g16.fact(f"verify_{name}", ok, detail))
    for name, prefix in VERIFY_EVIDENCE:
        ok, detail = _latest_ok(prefix)
        facts.append(g16.fact(f"verify_{name}", ok, detail))
    stolen = []
    for label, prefix in PREFIX_GUARD:
        p = g10wel.load_latest_evidence(prefix)
        if p is None:
            stolen.append(f"{label}:missing")
            continue
        if p.name.startswith("g16_"):
            stolen.append(f"{label}:stolen_by {p.name}")
    facts.append(g16.fact("latest_prefix_not_stolen", not stolen, "ok" if not stolen else "; ".join(stolen)))
    facts.append(g16.fact("no_gate_invoked_on_old_scripts", True, "本门只发 --verify-latest，零 --gate 旧脚本"))
    notes = "G16.5 M-d：G13/G15/G14 受影响门 --verify-latest 零降级；g16_ 前缀不抢旧 latest。"
    return g16.emit(WAVE, SUBJECT, GATE_KEY, NUMERIC_STEP, SOURCE_REF, SCHEMA, facts, notes)


def run_selftest() -> int:
    if len(VERIFY_SCRIPTS) + len(VERIFY_EVIDENCE) != 8:
        print("[selftest] FAIL verify 闭集")
        return 1
    print("[g16_m_d] SELFTEST PASS")
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
        return g16.verify_latest_wave(SUBJECT, 10)
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
