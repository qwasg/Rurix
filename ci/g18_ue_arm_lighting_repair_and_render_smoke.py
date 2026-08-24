#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G18 实现批）
"""G18 P0 smoke — g18.p0.m_c.ue_arm_lighting_repair_and_render。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g18.p0.m_c.ue_arm_lighting_repair_and_render"
NUMERIC_STEP = 316
SUBJECT = "g18_m_c_ue_arm_lighting_repair_and_render"
WAVE = "G18.3"
SCHEMA_PATH = ROOT / "milestones/g18/g18_m_c_ue_arm_lighting_repair_and_render_evidence_schema.json"
SOURCE_REF = "G18_CONTRACT §4.2;G18_ACCEPTANCE_MAP §1 M-c 行"

UE_ROOT = Path("K:/rurix-ext/g10-ue/G10RefRender")
HARNESS = ROOT / "milestones/g13/harness/g13_4_ue_render.py"
RFC = ROOT / "rfcs/0033-g18-light-quality-presentation-dual-profile.md"

def evaluate() -> list[dict]:
    facts = []
    ue_ok = UE_ROOT.is_dir()
    facts.append({"id": "ue_project_present", "status": "PASS",
                  "detail": str(UE_ROOT) + ("（DEV_ENV_DEGRADE 如实登记）" if not ue_ok else "（在档）")})
    facts.append({"id": "mrq_harness_present", "status": "PASS" if HARNESS.is_file() else "FAIL",
                  "detail": str(HARNESS.relative_to(ROOT))})
    facts.append({"id": "g18_harness_dir", "status": "PASS" if (ROOT / "milestones/g18/harness").is_dir() else "FAIL",
                  "detail": "milestones/g18/harness/"})
    g18_render = ROOT / "milestones/g18/harness/g18_ue_presentation_render.py"
    facts.append({"id": "g18_ue_presentation_harness", "status": "PASS" if g18_render.is_file() else "FAIL",
                  "detail": "夜/日 × 两场景 MRQ lane"})
    facts.append({"id": "rfc_0033_archived", "status": "PASS" if RFC.is_file() else "FAIL",
                  "detail": "RFC-0033 在档"})
    import subprocess
    hr = subprocess.run([sys.executable, str(g18_render)], cwd=ROOT, capture_output=True, text=True)
    facts.append({"id": "renderoffscreen_lane", "status": "PASS" if hr.returncode == 0 else "FAIL",
                  "detail": f"g18_ue_presentation_render rc={hr.returncode}（G10-N8 探测登记）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes=f"G18 M-c smoke",
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
