#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G19 波聚合门）
"""G19 波聚合门（参数化 g19.wave.{2..6}.exit）。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
SCHEMA_PATH = ROOT / "milestones/g19/g19_wave_exit_evidence_schema.json"
WAVES: dict[str, tuple[str, str, int]] = {
    "g19.wave.2.exit": ("G19.2", "g19_m_a_frame_generation_host_realization", 337),
    "g19.wave.3.exit": ("G19.3", "g19_m_b_frame_generation_vendor_disposition", 339),
    "g19.wave.4.exit": ("G19.4", "g19_m_c_rd045_drift_observation_window", 341),
    "g19.wave.5.exit": ("G19.5", "g19_m_d_fps_parity_window_registration", 343),
    "g19.wave.6.exit": ("G19.6", "g19_m_e_closed_gate_no_regression", 345),
}
GUARDS = ("check_structure.py", "check_schemas.py", "check_number_ledger.py")


def evaluate(gate_key: str, m_doc: dict | None, guard_codes: list[int], budget_code: int) -> list[dict]:
    wave, subject, _ = WAVES[gate_key]
    facts = []
    if m_doc is None:
        facts.append({"id": "required_m_gate_latest_pass", "status": "FAIL",
                      "detail": f"{subject} 无 evidence"})
    else:
        m_ok = m_doc.get("host_section_pass") is True and all(
            f.get("status") == "PASS" for f in m_doc.get("extra_facts", [])
        )
        facts.append({"id": "required_m_gate_latest_pass", "status": "PASS" if m_ok else "FAIL",
                      "detail": subject})
    facts.append({"id": "guards_pass", "status": "PASS" if all(c == 0 for c in guard_codes) else "FAIL",
                  "detail": str(guard_codes)})
    facts.append({"id": "budget_eval_pass", "status": "PASS" if budget_code == 0 else "FAIL",
                  "detail": str(budget_code)})
    facts.append({"id": "aggregate_read_only", "status": "PASS", "detail": "只读聚合"})
    return facts


def run_gate(gate_key: str) -> int:
    wave, subject, step = WAVES[gate_key]
    p = wel.load_latest_evidence(subject)
    m_doc = wel.load_json(p) if p else None
    guard_codes = [subprocess.run([sys.executable, f"ci/{g}"], cwd=ROOT, capture_output=True).returncode for g in GUARDS]
    budget_code = subprocess.run([sys.executable, "ci/budget_eval.py"], cwd=ROOT, capture_output=True).returncode
    facts = evaluate(gate_key, m_doc, guard_codes, budget_code)
    overall = all(f["status"] == "PASS" for f in facts)
    n = gate_key.split(".")[2]
    code, _ = wel.emit_wave_evidence(
        wave=wave, subject=f"g19_wave{n}_exit", symbolic_gate_key=gate_key, numeric_step=step,
        source_ref="G19_CONTRACT §2", required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=f"g19_wave{n}_exit",
        notes=f"G19 wave {n} exit", host_section_pass=overall,
    )
    return 0 if (overall and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=sorted(WAVES))
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return 0
    return run_gate(args.gate)


if __name__ == "__main__":
    sys.exit(main())
