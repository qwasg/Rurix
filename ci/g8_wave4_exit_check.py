#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.4 wave4.exit 聚合门(步骤合入时领取)。

M37 PASS + GeomPage PASS + VT not-triggered + queue_mode=single。
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.4.exit"
NUMERIC_STEP = 114
SUBJECT = "g8_wave4_exit"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave4_exit_evidence_schema.json"
CANDIDATE = ROOT / "milestones" / "g8" / "G8_CANDIDATE_DECISIONS.md"

REQUIRED = [
    ("g8.p0.m37.streaming_io", "g8_m37_streaming_io"),
    ("g8.gate.geom_page", "g8_gate_geom_page"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_extra(evidence_dir=None) -> list[dict]:
    facts = []
    # VT not-triggered from candidate M40 no-go
    text = CANDIDATE.read_text(encoding="utf-8") if CANDIDATE.is_file() else ""
    # RD-041/M40 行：no-go + 门-VT SKIP=not-triggered（设计 §4.4）
    vt_ok = (
        "M40" in text
        and "no-go" in text
        and ("SKIP=not-triggered" in text or "not-triggered" in text.lower())
    )
    facts.append(
        _fact(
            "vt_not_triggered",
            vt_ok,
            "M40/SVT no-go → vt_gate=SKIP=not-triggered（零实现）",
        )
    )
    # queue_mode from M37 latest evidence
    m37 = wel.load_latest_evidence("g8_m37_streaming_io", evidence_dir=evidence_dir)
    qm_ok = False
    detail = "missing m37 evidence"
    if m37:
        d = json.loads(m37.read_text(encoding="utf-8"))
        qm = d.get("queue_mode")
        qm_ok = qm == "single"
        detail = f"queue_mode={qm!r}"
    facts.append(_fact("queue_mode_single", qm_ok, detail))
    return facts


def run_gate(*, evidence_dir=None) -> int:
    if NUMERIC_STEP <= 0:
        print("[wave4] NUMERIC_STEP unset", file=sys.stderr)
        return 1
    rows = [wel.require_gate_pass(k, p, evidence_dir=evidence_dir) for k, p in REQUIRED]
    code, _ = wel.emit_wave_evidence(
        wave="G8.4",
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="CI_GATES §5; design §4.5; queue_mode=single; VT not-triggered",
        required_gate_rows=rows,
        extra_facts=collect_extra(evidence_dir),
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="wave4 aggregate",
        host_section_pass=True,
    )
    return code


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
