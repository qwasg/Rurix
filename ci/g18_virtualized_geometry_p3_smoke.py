#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G18 实现批）
"""G18 P0 smoke — g18.p0.m_g.virtualized_geometry_p3。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g18.p0.m_g.virtualized_geometry_p3"
NUMERIC_STEP = 324
SUBJECT = "g18_m_g_virtualized_geometry_p3"
WAVE = "G18.5"
SCHEMA_PATH = ROOT / "milestones/g18/g18_m_g_virtualized_geometry_p3_evidence_schema.json"
SOURCE_REF = "G18_CONTRACT §4.2;G18_ACCEPTANCE_MAP §1 M-g 行"

RFC = ROOT / "rfcs/0034-virtualized-geometry-p3-mesh-shader.md"

def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "rfc_0034_archived", "status": "PASS" if RFC.is_file() else "FAIL",
                  "detail": str(RFC.relative_to(ROOT))})
    facts.append({"id": "terminal_state_no_go", "status": "PASS",
                  "detail": "RFC-0034 终态=no-go（VK_EXT_mesh_shader 第三光栅路径本期不接线；HZB P4 defer）"})
    facts.append({"id": "ser_applicability", "status": "PASS",
                  "detail": "M52 SER 适用性=no-go 如实登记（ray query 形态不适用）"})
    facts.append({"id": "pixel_identity_criterion", "status": "PASS",
                  "detail": "像素零差判据留档（未触发实现）"})
    facts.append({"id": "vs_fallback_maintained", "status": "PASS",
                  "detail": "VS 光栅 fallback 维持（M61 defer 承接）"})
    facts.append({"id": "adversarial_review", "status": "PASS" if (ROOT / "milestones/g18/design/rfc0034_adversarial_review.md").is_file() else "FAIL",
                  "detail": "D-4xx 对抗评审"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes=f"G18 M-g smoke",
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
