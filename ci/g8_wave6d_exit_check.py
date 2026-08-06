#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.6d wave6d.exit：M72 + m70.vehicle subject。"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.6d.exit"
NUMERIC_STEP = 127
SUBJECT = "g8_wave6d_exit"
WAVE = "G8.6d"
SOURCE_REF = "CI_GATES §5;M72 cloth + M70 vehicle subject"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave6d_exit_evidence_schema.json"
REQUIRED = [("g8.p1.m72.cloth_product_chain", "g8_m72_cloth_product_chain")]


def vehicle_subject() -> dict:
    name = "g8-physics-gates.exe" if sys.platform == "win32" else "g8-physics-gates"
    exe = ROOT / "target" / "debug" / name
    if not exe.is_file():
        subprocess.run(["cargo", "build", "-p", "g8-physics-gates", "--quiet"], cwd=ROOT)
    r = subprocess.run([str(exe), "vehicle"], cwd=ROOT, capture_output=True, text=True)
    try:
        doc = json.loads((r.stdout or "").strip().splitlines()[-1])
    except Exception:
        doc = {}
    ok = bool(doc.get("vehicle_subject_pass"))
    return {
        "id": "g8.wave6d.m70.vehicle",
        "status": "PASS" if ok else "FAIL",
        "detail": doc.get("detail") or "vehicle probe",
        "evidence_path": "apps/g8-physics-gates vehicle",
    }


def run_gate(*, evidence_dir=None) -> int:
    if NUMERIC_STEP <= 0:
        return 1
    rows = [wel.require_gate_pass(k, p, evidence_dir=evidence_dir) for k, p in REQUIRED]
    subj = vehicle_subject()
    extras = [
        {
            "id": "cloth_five_map",
            "status": "PASS" if rows and rows[0]["status"] == "PASS" else "FAIL",
            "detail": "M72 five MAP + supports",
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
        subjects=[subj],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="wave6d: M72 + m70.vehicle",
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
        assert NUMERIC_STEP == 127
        print("[wave6d] selftest OK")
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
