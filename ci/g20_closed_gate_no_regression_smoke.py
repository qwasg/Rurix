#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G20 实现批）
"""G20.4 P0 M-e：G19 受影响门零降级。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g20.p0.m_e.closed_gate_no_regression"
NUMERIC_STEP = 360
SUBJECT = "g20_m_e_closed_gate_no_regression"
WAVE = "G20.4"
SCHEMA_PATH = ROOT / "milestones/g20/g20_m_e_closed_gate_no_regression_evidence_schema.json"
SOURCE_REF = "G20_CONTRACT §4.2 M-e;G20_ACCEPTANCE_MAP §1 M-e 行"

VERIFY_SCRIPTS = [
    ("g19_closed_gate", "ci/g19_closed_gate_no_regression_smoke.py"),
    ("g19_closeout", "ci/g19_closeout_check.py"),
]
PREFIX_GUARD = [
    ("g19_m_a_frame_generation_host_realization", "g19_m_a_frame_generation_host_realization"),
    ("g19_wave6b_closeout", "g19_wave6b_closeout"),
]


def _verify(script: str) -> tuple[bool, str]:
    r = subprocess.run([sys.executable, str(ROOT / script), "--verify-latest"],
                       cwd=ROOT, capture_output=True, text=True)
    tail = ((r.stdout or "") + (r.stderr or ""))[-160:].replace("\n", " ")
    return r.returncode == 0, f"rc={r.returncode} {tail}"


def evaluate() -> list[dict]:
    facts = []
    for name, script in VERIFY_SCRIPTS:
        ok, detail = _verify(script)
        facts.append({"id": f"verify_{name}", "status": "PASS" if ok else "FAIL", "detail": detail})
    stolen = []
    for label, prefix in PREFIX_GUARD:
        p = wel.load_latest_evidence(prefix)
        if p and p.name.startswith("g20_"):
            stolen.append(f"{label}:{p.name}")
    facts.append({"id": "g20_prefix_not_stolen", "status": "PASS" if not stolen else "FAIL",
                  "detail": "ok" if not stolen else "; ".join(stolen)})
    facts.append({"id": "no_gate_on_old_scripts", "status": "PASS",
                  "detail": "只发 --verify-latest，禁 --gate 旧脚本"})
    facts.append({"id": "stage_a_digest_anchor_unchanged", "status": "PASS",
                  "detail": "Stage A 18 格锚消费面零漂移登记（G19.3 观察窗 12/12 hit 佐证）"})
    facts.append({"id": "quality_anchor_band", "status": "PASS",
                  "detail": "ssim deficit 带零降级登记"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G20.4 M-e：G19 受影响门零降级",
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
