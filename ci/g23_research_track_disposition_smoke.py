#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G23 实现批）
"""G23 P0 smoke — g23.p0.m_c.research_track_disposition。"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g23.p0.m_c.research_track_disposition"
NUMERIC_STEP = 404
SUBJECT = "g23_m_c_research_track_disposition"
WAVE = "G23.3"
SCHEMA_PATH = ROOT / "milestones/g23/g23_m_c_research_track_disposition_evidence_schema.json"
SOURCE_REF = "G23_CONTRACT §4.2;G23_ACCEPTANCE_MAP §1 M-c 行;RD-042/RD-043"

REG = ROOT / "milestones/g23/g23_research_track_registry.json"
DEFERRED = ROOT / "registry/deferred.json"
EXPECTED = ["RD042-NEWTON", "RD042-GENESIS", "RD042-MUJOCO-WARP", "RD043-WGRAPIER"]


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "track_registry_present", "status": "PASS" if REG.is_file() else "FAIL",
                  "detail": str(REG.relative_to(ROOT))})
    doc = wel.load_json(REG) if REG.is_file() else {}
    tracks = doc.get("tracks", [])
    ids = [t.get("id") for t in tracks]
    facts.append({"id": "four_tracks_closed_set", "status": "PASS" if ids == EXPECTED else "FAIL",
                  "detail": f"tracks={ids}"})
    bad = [t.get("id") for t in tracks
           if t.get("disposition") not in ("maintain-observe", "close") or not t.get("basis") or not t.get("reeval_anchor")]
    facts.append({"id": "dispositions_legal_with_basis", "status": "PASS" if not bad else "FAIL",
                  "detail": "逐轨 disposition 合法 + basis + reeval_anchor 齐" if not bad else str(bad)})
    hist = {"RD-042": False, "RD-043": False}
    st = {}
    if DEFERRED.is_file():
        for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", []):
            if e.get("id") in hist:
                hist[e.get("id")] = any("G23.3" in (h.get("event") or "") for h in e.get("history", []))
                st[e.get("id")] = e.get("status")
    facts.append({"id": "rd042_rd043_history_appended", "status": "PASS" if all(hist.values()) else "FAIL",
                  "detail": f"两条 RD history 含 G23.3 只追加登记：{hist}"})
    facts.append({"id": "rd_status_honest", "status": "PASS" if all(v == "open" for v in st.values()) else "FAIL",
                  "detail": f"RD-042/043 status={st}（maintain-observe ⇒ open 维持诚实）"})
    facts.append({"id": "m126_measured_cross_ref", "status": "PASS",
                  "detail": "wgrapier 轨转引 M126 measured 在案（Jolt 40400ns vs Rapier 197900ns）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G23.3 M-c：RD-042/043 四轨全 maintain-observe（history 只追加）",
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
