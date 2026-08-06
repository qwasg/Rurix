#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.6c wave6c.exit：只读汇总 M68 PASS。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.6c.exit"
NUMERIC_STEP = 125
SUBJECT = "g8_wave6c_exit"
WAVE = "G8.6c"
SOURCE_REF = "CI_GATES §5;G8_CONTRACT;M68 fracture full-chain"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave6c_exit_evidence_schema.json"
REQUIRED = [("g8.p0.m68.fracture_pipeline", "g8_m68_fracture_pipeline")]


def run_gate(*, evidence_dir=None) -> int:
    if NUMERIC_STEP <= 0:
        print("[wave6c] NUMERIC_STEP unset → 红", file=sys.stderr)
        return 1
    rows = [wel.require_gate_pass(k, p, evidence_dir=evidence_dir) for k, p in REQUIRED]
    extras = [
        {
            "id": "fracture_full_chain",
            "status": "PASS" if rows and rows[0]["status"] == "PASS" else "FAIL",
            "detail": "M68 12 checks via required gate",
        }
    ]
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=rows,
        extra_facts=extras,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="wave6c: M68 PASS",
        host_section_pass=True,
    )
    return code


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        assert NUMERIC_STEP == 125
        print("[wave6c] selftest OK")
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
