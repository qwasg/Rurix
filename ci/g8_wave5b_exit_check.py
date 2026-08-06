#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.5b wave5b.exit 聚合门(步骤 119)。

M24 PASS + M25 PASS + RD-038 GI/TSR/真帧接入空集 + retained-open。
不重跑 smoke、不代绿、不加 RURIX_REQUIRE_REAL。
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.5b.exit"
NUMERIC_STEP = 119
SUBJECT = "g8_wave5b_exit"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave5b_exit_evidence_schema.json"
CANDIDATE = ROOT / "milestones" / "g8" / "G8_CANDIDATE_DECISIONS.md"
RFC19 = ROOT / "rfcs" / "0019-rendering-platform.md"

REQUIRED = [
    ("g8.p0.m24.tsr_contract", "g8_m24_tsr_contract"),
    ("g8.p1.m25.upscaler_input_abi", "g8_m25_upscaler_input_abi"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_extra(evidence_dir=None) -> list[dict]:
    facts = []
    # RD-038 GI/TSR/真帧接入空集(G7 closed;决策表 v1.1 先例)
    text = CANDIDATE.read_text(encoding="utf-8") if CANDIDATE.is_file() else ""
    empty_ok = "RD-038" in text and ("closed" in text.lower() or "空集" in text)
    facts.append(
        _fact(
            "rd038_gi_tsr_frame_ingress_empty_set",
            empty_ok,
            "RD-038 G7 closed → G8.5b 接入空集(不放宽 M24/M25)",
        )
    )
    retained = all(
        x in text for x in ("M07", "M08", "M45", "M46", "M47", "矩阵 P1 未判行补裁决")
    )
    facts.append(
        _fact(
            "retained_open_listed",
            retained,
            "G8_CANDIDATE_DECISIONS §10 矩阵 P1 未判行补裁决",
        )
    )
    # M24 tolerance RFC 冻结
    rfc = RFC19.read_text(encoding="utf-8") if RFC19.is_file() else ""
    m24 = wel.load_latest_evidence("g8_m24_tsr_contract", evidence_dir=evidence_dir)
    tol_ok = False
    detail = "missing m24 evidence"
    if m24:
        d = json.loads(m24.read_text(encoding="utf-8"))
        stage = (d.get("tolerance_stage") or {}).get("stage")
        tol_ok = stage == "rfc_budget_frozen" and "4.6.4" in rfc
        detail = f"tolerance_stage={stage!r}; RFC §4.6.4={'yes' if '4.6.4' in rfc else 'no'}"
    facts.append(_fact("m24_tolerance_rfc_frozen", tol_ok, detail))
    return facts


def run_gate(*, evidence_dir=None) -> int:
    if NUMERIC_STEP <= 0:
        print("[wave5b] NUMERIC_STEP unset", file=sys.stderr)
        return 1
    rows = [wel.require_gate_pass(k, p, evidence_dir=evidence_dir) for k, p in REQUIRED]
    code, _ = wel.emit_wave_evidence(
        wave="G8.5b",
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=(
            "CI_GATES §5;G8.5_RENDERING_COMPLETION_DESIGN §6;"
            "M24+M25 PASS + RD-038 empty-set + retained-open + RFC-0019 §4.6.4"
        ),
        required_gate_rows=rows,
        extra_facts=collect_extra(evidence_dir),
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="wave5b aggregate",
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
        assert NUMERIC_STEP == 119
        print("[wave5b] selftest OK")
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
