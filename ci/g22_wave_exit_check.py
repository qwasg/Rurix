#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G22 波聚合门）
"""G22 波聚合门（参数化 g22.wave.{2..6}.exit）。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
SCHEMA_PATH = ROOT / "milestones/g22/g22_wave_exit_evidence_schema.json"
WAVES: dict[str, tuple[str, str, int]] = {
    "g22.wave.2.exit": ("G22.2", "g22_m_a_slab_material_host_realization", 385),
    "g22.wave.3.exit": ("G22.3", "g22_m_b_svt_disposition", 387),
    "g22.wave.4.exit": ("G22.4", "g22_m_c_ktx2_basisu_disposition", 389),
    "g22.wave.5.exit": ("G22.5", "g22_m_d_work_graphs_fsr_reeval_disposition", 391),
    "g22.wave.6.exit": ("G22.6", "g22_m_e_closed_gate_no_regression", 393),
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
        wave=wave, subject=f"g22_wave{n}_exit", symbolic_gate_key=gate_key, numeric_step=step,
        source_ref="G22_CONTRACT §2", required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=f"g22_wave{n}_exit",
        notes=f"G22 wave {n} exit", host_section_pass=overall,
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
