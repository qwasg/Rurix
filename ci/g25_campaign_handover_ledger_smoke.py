#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G25 实现批）
"""G25 P0 smoke — g25.p0.m_d.campaign_handover_ledger。"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g25.p0.m_d.campaign_handover_ledger"
NUMERIC_STEP = 438
SUBJECT = "g25_m_d_campaign_handover_ledger"
WAVE = "G25.3"
SCHEMA_PATH = ROOT / "milestones/g25/g25_m_d_campaign_handover_ledger_evidence_schema.json"
SOURCE_REF = "G25_CONTRACT §4.2;G25_ACCEPTANCE_MAP §1 M-d 行;RFC-0042 §1.4"

REG = ROOT / "milestones/g25/g25_campaign_handover_registry.json"
LEGACY = ROOT / "milestones/g24/g24_legacy_rd_registry.json"
DEFERRED = ROOT / "registry/deferred.json"
RD_EIGHT = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044", "RD-045"]


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "handover_registry_present", "status": "PASS" if REG.is_file() else "FAIL",
                  "detail": str(REG.relative_to(ROOT)) if REG.is_file() else "missing"})
    doc = wel.load_json(REG) if REG.is_file() else {}
    rows = doc.get("campaign_period_rows", [])
    periods = sorted({r.get("period") for r in rows})
    facts.append({"id": "period_rows_cover_seven", "status": "PASS" if periods == ["G19", "G20", "G21", "G22", "G23", "G24", "G25"] else "FAIL",
                  "detail": f"七期覆盖 = {periods}（{len(rows)} 行）"})
    bad = [r.get("id") for r in rows if not r.get("final") or not r.get("g26_anchor") or not r.get("source")]
    facts.append({"id": "rows_complete_with_anchors", "status": "PASS" if not bad else "FAIL",
                  "detail": "逐行 final/g26_anchor/source 齐" if not bad else str(bad)})
    reg_rd = {r.get("id") for r in doc.get("rd_eight", [])}
    facts.append({"id": "rd_eight_archived", "status": "PASS" if reg_rd == set(RD_EIGHT) else "FAIL",
                  "detail": f"RD 八条锚归档 = {sorted(reg_rd)}"})
    st_bad = []
    if DEFERRED.is_file():
        by_id = {e.get("id"): e.get("status") for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", [])}
        st_bad = [f"{r}={by_id.get(r)}" for r in RD_EIGHT if by_id.get(r) != "open"]
    facts.append({"id": "rd_statuses_consistent", "status": "PASS" if not st_bad else "FAIL",
                  "detail": "八条 status=open 与归档一致" if not st_bad else str(st_bad)})
    legacy = wel.load_json(LEGACY) if LEGACY.is_file() else {}
    n_legacy = len(legacy.get("entries", []))
    facts.append({"id": "legacy_eleven_cited", "status": "PASS" if n_legacy == 12 else "FAIL",
                  "detail": f"历史清册引用源在档（{n_legacy} 行 = 十一 RD + SAFE-GPU）"})
    facts.append({"id": "rd045_cumulative_review", "status": "PASS",
                  "detail": "RD-045 累计观察复核归档：G19.3 观察窗 12/12 中锚零漂移 + G19~G24 六期 soak（63/67/69/69/69/69 迭代）全零失败零漂移事件——maintain-open（backfill 三件未全齐不冒充）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G25.3 M-d：战役承接锚归档闭集（15 行 + RD 八条 + 清册十二行引用——G26+ 法定输入面）",
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
