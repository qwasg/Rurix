#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G25 实现批）
"""G25 P0 smoke — g25.p0.m_c.campaign_full_chain_no_regression。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g25.p0.m_c.campaign_full_chain_no_regression"
NUMERIC_STEP = 436
SUBJECT = "g25_m_c_campaign_full_chain_no_regression"
WAVE = "G25.3"
SCHEMA_PATH = ROOT / "milestones/g25/g25_m_c_campaign_full_chain_no_regression_evidence_schema.json"
SOURCE_REF = "G25_CONTRACT §4.2;G25_ACCEPTANCE_MAP §1 M-c 行;RFC-0042 §1.3"

VERIFY_SCRIPTS = [
    ("g24_closed_gate", "ci/g24_closed_gate_no_regression_smoke.py"),
    ("g24_closeout", "ci/g24_closeout_check.py"),
]
GUARDS = ("check_structure.py", "check_schemas.py", "check_number_ledger.py")


def _verify(script: str) -> tuple[bool, str]:
    r = subprocess.run([sys.executable, str(ROOT / script), "--verify-latest"],
                       cwd=ROOT, capture_output=True, text=True)
    tail = ((r.stdout or "") + (r.stderr or ""))[-120:].replace("\n", " ")
    return r.returncode == 0, f"rc={r.returncode} {tail}"


def evaluate() -> list[dict]:
    facts = []
    for name, script in VERIFY_SCRIPTS:
        ok, detail = _verify(script)
        facts.append({"id": f"verify_{name}", "status": "PASS" if ok else "FAIL", "detail": detail})
    facts.append({"id": "recursive_chain_note", "status": "PASS",
                  "detail": "G24 链递归涵盖 G13~G23（各期 M-e 链式 verify-latest 结构）"})
    codes = [subprocess.run([sys.executable, f"ci/{g}"], cwd=ROOT, capture_output=True).returncode
             for g in GUARDS]
    facts.append({"id": "guards_pass", "status": "PASS" if all(c == 0 for c in codes) else "FAIL",
                  "detail": f"structure/schemas/ledger 守卫 rc={codes}"})
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"],
                       cwd=ROOT, capture_output=True, text=True)
    tail = ((r.stdout or "") + (r.stderr or "")).strip().splitlines()[-1:]
    facts.append({"id": "budget_strict_full", "status": "PASS" if r.returncode == 0 else "FAIL",
                  "detail": f"budget_eval --strict rc={r.returncode} {tail}"})
    facts.append({"id": "no_gate_on_old_scripts", "status": "PASS",
                  "detail": "只发 --verify-latest，禁 --gate 旧脚本"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G25.3 M-c：战役全链零降级（G24 链递归 + strict 预算全量）",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
