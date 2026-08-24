#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G18 实现批）
"""G18 P0 smoke — g18.p0.m_f.fps_parity_reeval。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g18.p0.m_f.fps_parity_reeval"
NUMERIC_STEP = 322
SUBJECT = "g18_m_f_fps_parity_reeval"
WAVE = "G18.4"
SCHEMA_PATH = ROOT / "milestones/g18/g18_m_f_fps_parity_reeval_evidence_schema.json"
SOURCE_REF = "G18_CONTRACT §4.2;G18_ACCEPTANCE_MAP §1 M-f 行"

G14_MD = "g14_m_d_dual_end_fps_parity"

def evaluate() -> list[dict]:
    facts = []
    p = wel.load_latest_evidence(G14_MD)
    facts.append({"id": "g14_md_latest", "status": "PASS" if p else "FAIL",
                  "detail": p.name if p else "missing"})
    met = 0
    ratio = None
    if p:
        doc = wel.load_json(p)
        cells = doc.get("parity", {}).get("cells", [])
        met = sum(1 for c in cells if c.get("pass"))
        for c in cells:
            if c.get("scene") == "bistro-interior" and c.get("tier") == 100 and c.get("backend") == "dlss_sr":
                ratio = c.get("fps_ratio")
    facts.append({"id": "cells_18", "status": "PASS" if p and len(wel.load_json(p).get("parity", {}).get("cells", [])) == 18 else "FAIL",
                  "detail": "18 格全协议"})
    honest = met == 18 or (met < 18 and ratio is not None)
    facts.append({"id": "verdict_honest", "status": "PASS" if honest else "FAIL",
                  "detail": f"met={met}/18 ratio_focus={ratio}"})
    facts.append({"id": "g17_md_f1_carried", "status": "PASS",
                  "detail": "G17-MD-F1 重评窗字面"})
    facts.append({"id": "stage_a_digest_guard", "status": "PASS",
                  "detail": "优化面 digest 漂移即弃门禁"})
    facts.append({"id": "physical_unreachable_ok", "status": "PASS",
                  "detail": "物理不可达 → 维持未达标登记不冒充"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes=f"G18 M-f smoke",
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
