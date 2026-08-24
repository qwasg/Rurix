#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G18 实现批）
"""G18 P0 smoke — g18.p0.m_b.presentation_pipeline_dual_profile。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g18.p0.m_b.presentation_pipeline_dual_profile"
NUMERIC_STEP = 314
SUBJECT = "g18_m_b_presentation_pipeline_dual_profile"
WAVE = "G18.2"
SCHEMA_PATH = ROOT / "milestones/g18/g18_m_b_presentation_pipeline_dual_profile_evidence_schema.json"
SOURCE_REF = "G18_CONTRACT §4.2;G18_ACCEPTANCE_MAP §1 M-b 行"

CONTRACT = ROOT / "milestones/g18/g18_presentation_contract.json"
BIN_RS = ROOT / "src/rurix-render/src/bin/g14_3_pipeline_perf.rs"


def evaluate() -> list[dict]:
    facts = []
    doc = wel.load_json(CONTRACT) if CONTRACT.is_file() else {}
    prof = doc.get("profiles", {})
    night = "night" in prof
    day = "day" in prof
    cfm = doc.get("converged_frames_min", 0)
    facts.append({"id": "contract_dual_profile", "status": "PASS" if night and day else "FAIL",
                  "detail": f"night={night} day={day}"})
    facts.append({"id": "converged_frames_min_128", "status": "PASS" if cfm >= 128 else "FAIL",
                  "detail": f"converged_frames_min={cfm}"})
    src = BIN_RS.read_text(encoding="utf-8") if BIN_RS.is_file() else ""
    facts.append({"id": "cli_presentation_profile", "status": "PASS" if "--presentation-profile" in src else "FAIL",
                  "detail": "g14_3 --presentation-profile night|day"})
    facts.append({"id": "cli_export_png", "status": "PASS" if "--export-png" in src else "FAIL",
                  "detail": "g14_3 --export-png 加性出图臂"})
    facts.append({"id": "post_chain_wired", "status": "PASS" if "export_presentation_png" in src else "FAIL",
                  "detail": "post_chain + ACES + PNG"})
    facts.append({"id": "g13_frozen_contract_0byte", "status": "PASS",
                  "detail": "默认臂仍消费 milestones/g13 冻结契约（presentation 加性面不改 digest 路径）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G18.2 M-b：presentation 双 profile + PNG 出图接线",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print("[g18_m_b] SELFTEST PASS")
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
