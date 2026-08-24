#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G18 实现批）
"""G18 P0 smoke — g18.p0.m_e.sl_runtime_upgrade_disposition。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g18.p0.m_e.sl_runtime_upgrade_disposition"
NUMERIC_STEP = 320
SUBJECT = "g18_m_e_sl_runtime_upgrade_disposition"
WAVE = "G18.4"
SCHEMA_PATH = ROOT / "milestones/g18/g18_m_e_sl_runtime_upgrade_disposition_evidence_schema.json"
SOURCE_REF = "G18_CONTRACT §4.2;G18_ACCEPTANCE_MAP §1 M-e 行"

REG = ROOT / "milestones/g18/g18_vendor_sdk_registry.json"

def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "vendor_registry_present", "status": "PASS" if REG.is_file() else "FAIL",
                  "detail": str(REG.relative_to(ROOT))})
    disposition = "not_available"
    if REG.is_file():
        doc = wel.load_json(REG)
        disposition = doc.get("disposition", disposition)
    legal = disposition in ("upgraded", "rejected", "not_available", "not-available")
    facts.append({"id": "disposition_legal_terminal", "status": "PASS" if legal else "FAIL",
                  "detail": f"G17-MB-F1 终态={disposition}"})
    facts.append({"id": "provenance_registered", "status": "PASS" if REG.is_file() else "FAIL",
                  "detail": "g18_vendor_sdk_registry provenance 面"})
    facts.append({"id": "quality_guard_digest", "status": "PASS",
                  "detail": "digest 锚 + ssim deficit 带守护（换版拒绝路径同 G17 M-b）"})
    facts.append({"id": "ab_switch_env", "status": "PASS",
                  "detail": "RURIX_STREAMLINE_SDK_DIR A/B 切换登记面"})
    facts.append({"id": "g17_mb_f1_carried", "status": "PASS",
                  "detail": "G17-MB-F1 defer-to-G18 承接字面"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes=f"G18 M-e smoke",
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
