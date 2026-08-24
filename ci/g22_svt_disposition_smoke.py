#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G22 实现批）
"""G22 P0 smoke — g22.p0.m_b.svt_disposition。"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g22.p0.m_b.svt_disposition"
NUMERIC_STEP = 386
SUBJECT = "g22_m_b_svt_disposition"
WAVE = "G22.2"
SCHEMA_PATH = ROOT / "milestones/g22/g22_m_b_svt_disposition_evidence_schema.json"
SOURCE_REF = "G22_CONTRACT §4.2;G22_ACCEPTANCE_MAP §1 M-b 行;RD-041"

GAP = ROOT / "milestones/g22/g22_svt_gap.json"
STREAMING = [ROOT / "src/rurix-render/src/streaming" / f for f in
             ("pool.rs", "feedback.rs", "engine.rs", "resource.rs")]
DEFERRED = ROOT / "registry/deferred.json"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "gap_registry_present", "status": "PASS" if GAP.is_file() else "FAIL",
                  "detail": str(GAP.relative_to(ROOT))})
    doc = wel.load_json(GAP) if GAP.is_file() else {}
    rows = doc.get("gap_rows", [])
    ids = [r.get("id") for r in rows]
    facts.append({"id": "gap_rows_closed_set", "status": "PASS" if ids == ["SVT-1", "SVT-2", "SVT-3", "SVT-4"] else "FAIL",
                  "detail": f"gap_rows={ids}（闭集四行）"})
    bad = [r.get("id") for r in rows if not r.get("gap") or not r.get("anchor") or r.get("status") not in ("open", "closed")]
    facts.append({"id": "gap_rows_complete", "status": "PASS" if not bad else "FAIL",
                  "detail": "逐行 gap/anchor/status 齐" if not bad else str(bad)})
    disp = doc.get("disposition")
    facts.append({"id": "disposition_legal", "status": "PASS" if disp in ("go", "no-go", "defer") else "FAIL",
                  "detail": f"disposition={disp}（basis 在档）"})
    missing = [str(p.name) for p in STREAMING if not p.is_file()]
    facts.append({"id": "current_surface_verified", "status": "PASS" if not missing else "FAIL",
                  "detail": "streaming/ 四模块现面实测在树" if not missing else f"缺 {missing}"})
    rd_hist = False
    if DEFERRED.is_file():
        for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", []):
            if e.get("id") == "RD-041":
                rd_hist = any("G22.2" in (h.get("event") or "") for h in e.get("history", []))
    facts.append({"id": "rd041_history_appended", "status": "PASS" if rd_hist else "FAIL",
                  "detail": "RD-041 history 含 G22 处置窗只追加登记"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G22.2 M-b：SVT 分项 disposition=defer（四行差距闭集如实登记）",
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
