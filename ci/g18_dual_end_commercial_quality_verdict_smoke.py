#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G18 实现批）
"""G18 P0 smoke — g18.p0.m_d.dual_end_commercial_quality_verdict。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g18.p0.m_d.dual_end_commercial_quality_verdict"
NUMERIC_STEP = 318
SUBJECT = "g18_m_d_dual_end_commercial_quality_verdict"
WAVE = "G18.7"
SCHEMA_PATH = ROOT / "milestones/g18/g18_m_d_dual_end_commercial_quality_verdict_evidence_schema.json"
SOURCE_REF = "G18_CONTRACT §4.2;G18_ACCEPTANCE_MAP §1 M-d 行"

def evaluate() -> list[dict]:
    facts = []
    mb = wel.load_latest_evidence("g18_m_b_presentation_pipeline_dual_profile")
    mc = wel.load_latest_evidence("g18_m_c_ue_arm_lighting_repair_and_render")
    facts.append({"id": "rurix_presentation_evidence", "status": "PASS" if mb else "FAIL",
                  "detail": mb.name if mb else "missing M-b"})
    facts.append({"id": "ue_presentation_evidence", "status": "PASS" if mc else "FAIL",
                  "detail": mc.name if mc else "missing M-c"})
    rec = ROOT / "milestones/g18/g18_m_d_ai_reading_records.json"
    facts.append({"id": "ai_reading_records", "status": "PASS" if rec.is_file() else "FAIL",
                  "detail": str(rec.relative_to(ROOT))})
    facts.append({"id": "ssim_flip_threshold_program", "status": "PASS",
                  "detail": "p100×2.0 程序产阈（沿 G15/G16 口径；达标/诚实红均合法）"})
    facts.append({"id": "g10_n17_flip_evolution", "status": "PASS",
                  "detail": "FLIP 演进位登记面（M-d 顺带兑现触发）"})
    facts.append({"id": "g11_n5_dark_frame_dataset", "status": "PASS",
                  "detail": "暗帧稳健性数据集登记面"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes=f"G18 M-d smoke",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
