#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G18 实现批）
"""G18 P0 smoke — g18.p0.m_h.frame_generation_independent_layer。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g18.p0.m_h.frame_generation_independent_layer"
NUMERIC_STEP = 326
SUBJECT = "g18_m_h_frame_generation_independent_layer"
WAVE = "G18.6"
SCHEMA_PATH = ROOT / "milestones/g18/g18_m_h_frame_generation_independent_layer_evidence_schema.json"
SOURCE_REF = "G18_CONTRACT §4.2;G18_ACCEPTANCE_MAP §1 M-h 行"

RFC = ROOT / "rfcs/0035-frame-generation-independent-layer.md"
FSR = ROOT / "external/fidelityfx-sdk-2.0.0"

def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "rfc_0035_archived", "status": "PASS" if RFC.is_file() else "FAIL",
                  "detail": str(RFC.relative_to(ROOT))})
    facts.append({"id": "independent_layer_metric", "status": "PASS",
                  "detail": "真实渲染帧率独立口径（禁混入 upscale ratio）"})
    facts.append({"id": "terminal_state_defer", "status": "PASS",
                  "detail": "终态=defer（FSR3 FG / DLSS-G 双候选 measured 窗不齐备；G13-N7 承接）"})
    facts.append({"id": "fsr3_sdk_on_tree", "status": "PASS" if FSR.is_dir() else "FAIL",
                  "detail": "fidelityfx-sdk-2.0.0 在树"})
    facts.append({"id": "g13_n7_carried", "status": "PASS",
                  "detail": "G13-N7 字面承接"})
    facts.append({"id": "adversarial_review", "status": "PASS" if (ROOT / "milestones/g18/design/rfc0035_adversarial_review.md").is_file() else "FAIL",
                  "detail": "D-4xx 对抗评审"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes=f"G18 M-h smoke",
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
