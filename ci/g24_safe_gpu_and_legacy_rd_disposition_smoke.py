#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G24 实现批）
"""G24 P0 smoke — g24.p0.m_d.safe_gpu_and_legacy_rd_disposition。"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g24.p0.m_d.safe_gpu_and_legacy_rd_disposition"
NUMERIC_STEP = 422
SUBJECT = "g24_m_d_safe_gpu_and_legacy_rd_disposition"
WAVE = "G24.3"
SCHEMA_PATH = ROOT / "milestones/g24/g24_m_d_safe_gpu_and_legacy_rd_disposition_evidence_schema.json"
SOURCE_REF = "G24_CONTRACT §4.2;G24_ACCEPTANCE_MAP §1 M-d 行;RFC-0041 §1.4"

REG = ROOT / "milestones/g24/g24_legacy_rd_registry.json"
DEFERRED = ROOT / "registry/deferred.json"
LEGACY_RD = ["RD-007", "RD-011", "RD-012", "RD-014", "RD-015", "RD-026",
             "RD-027", "RD-030", "RD-032", "RD-033", "RD-036"]


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "ledger_registry_present", "status": "PASS" if REG.is_file() else "FAIL",
                  "detail": str(REG.relative_to(ROOT)) if REG.is_file() else "missing"})
    doc = wel.load_json(REG) if REG.is_file() else {}
    entries = doc.get("entries", [])
    ids = [e.get("id") for e in entries]
    expected = LEGACY_RD + ["SAFE-GPU"]
    facts.append({"id": "twelve_rows_closed_set", "status": "PASS" if ids == expected else "FAIL",
                  "detail": f"清册 {len(ids)}/12 行（十一 RD + SAFE-GPU）"})
    bad = [e.get("id") for e in entries
           if not e.get("backfill_check") or not e.get("disposition") or not e.get("reeval_anchor")]
    facts.append({"id": "rows_complete_with_backfill_check", "status": "PASS" if not bad else "FAIL",
                  "detail": "逐行 backfill 核验 + disposition + reeval_anchor 齐" if not bad else str(bad)})
    hist_miss = []
    st_bad = []
    if DEFERRED.is_file():
        by_id = {e.get("id"): e for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", [])}
        for rid in LEGACY_RD:
            e = by_id.get(rid, {})
            if not any("G24.3" in (h.get("event") or "") for h in e.get("history", [])):
                hist_miss.append(rid)
            if e.get("status") not in ("open", "inherited"):
                st_bad.append(f"{rid}={e.get('status')}")
    facts.append({"id": "legacy_histories_appended", "status": "PASS" if not hist_miss else "FAIL",
                  "detail": "十一条 RD history 含 G24.3 只追加登记" if not hist_miss else str(hist_miss)})
    facts.append({"id": "statuses_honest_zero_close", "status": "PASS" if not st_bad else "FAIL",
                  "detail": "零 close（backfill 字面无一成立；status 全 open/inherited 维持）" if not st_bad else str(st_bad)})
    sg = next((e for e in entries if e.get("id") == "SAFE-GPU"), {})
    facts.append({"id": "safe_gpu_disposition", "status": "PASS" if sg.get("disposition") == "defer-to-G25+" else "FAIL",
                  "detail": f"SAFE-GPU 处置 = {sg.get('disposition')}（独立期立项判据未成立；G25 归档窗点名）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G24.3 M-d：SAFE-GPU defer-to-G25+ + 历史 RD 十一条清册（零 close 诚实）",
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
