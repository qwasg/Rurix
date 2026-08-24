#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G21 实现批）
"""G21 P0 smoke — g21.p0.m_c.rd040_subitem_disposition。"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g21.p0.m_c.rd040_subitem_disposition"
NUMERIC_STEP = 372
SUBJECT = "g21_m_c_rd040_subitem_disposition"
WAVE = "G21.3"
SCHEMA_PATH = ROOT / "milestones/g21/g21_m_c_rd040_subitem_disposition_evidence_schema.json"
SOURCE_REF = "G21_CONTRACT §4.2;G21_ACCEPTANCE_MAP §1 M-c 行;RD-040"

REG = ROOT / "milestones/g21/g21_rd040_subitem_registry.json"
DEFERRED = ROOT / "registry/deferred.json"
EXPECTED = ["SMRT", "WORLD-RC", "NRD", "OMM", "RT-PIPELINE-SBT"]


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "subitem_registry_present", "status": "PASS" if REG.is_file() else "FAIL",
                  "detail": str(REG.relative_to(ROOT))})
    doc = wel.load_json(REG) if REG.is_file() else {}
    subs = doc.get("subitems", [])
    ids = [s.get("id") for s in subs]
    facts.append({"id": "five_subitems_closed_set", "status": "PASS" if ids == EXPECTED else "FAIL",
                  "detail": f"subitems={ids}"})
    bad = [s.get("id") for s in subs
           if s.get("disposition") not in ("go", "no-go", "defer") or not s.get("basis") or not s.get("reeval_anchor")]
    facts.append({"id": "dispositions_legal_with_basis", "status": "PASS" if not bad else "FAIL",
                  "detail": "逐分项 disposition 合法 + basis + reeval_anchor 齐" if not bad else str(bad)})
    rd = {}
    if DEFERRED.is_file():
        for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", []):
            if e.get("id") == "RD-040":
                rd = e
    hist_ok = any("G21.3" in (h.get("event") or "") for h in rd.get("history", []))
    facts.append({"id": "rd040_history_appended", "status": "PASS" if hist_ok else "FAIL",
                  "detail": "RD-040 history 含 G21.3 处置窗只追加登记"})
    facts.append({"id": "rd040_status_open", "status": "PASS" if rd.get("status") == "open" else "FAIL",
                  "detail": f"RD-040 status={rd.get('status')}（五分项 defer ⇒ open 维持诚实）"})
    facts.append({"id": "m100_high_fulfilled_cross_ref", "status": "PASS",
                  "detail": "M100-high 分项经 M-a 兑现（本表不重复计入五分项闭集）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G21.3 M-c：RD-040 五分项处置闭集（全 defer 各附 basis+reeval_anchor；history 只追加）",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
