#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.5a wave5a.exit 聚合门草稿(NUMERIC_STEP=0 留给主 agent 领号)。

M19 PASS + RD-038 raster/VSM 接入空集登记 + retained-open 清单(设计 §5)。
不重跑 smoke、不代绿。

用法:
  py -3 ci/g8_wave5a_exit_check.py --gate g8.wave.5a.exit
  py -3 ci/g8_wave5a_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.5a.exit"
NUMERIC_STEP = 116
SUBJECT = "g8_wave5a_exit"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave5a_exit_evidence_schema.json"
CANDIDATE = ROOT / "milestones" / "g8" / "G8_CANDIDATE_DECISIONS.md"

REQUIRED = [
    ("g8.p0.m19.vsm_page_cache", "g8_m19_vsm_page_cache"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_extra(evidence_dir=None) -> list[dict]:
    facts = []
    # RD-038 接入空集:M19 evidence 内登记或设计裁决
    m19 = wel.load_latest_evidence("g8_m19_vsm_page_cache", evidence_dir=evidence_dir)
    empty_ok = False
    detail = "missing m19 evidence"
    if m19:
        d = json.loads(m19.read_text(encoding="utf-8"))
        ing = d.get("rd038_raster_vsm_ingress") or {}
        empty_ok = ing.get("status") == "empty_set"
        detail = f"rd038_ingress={ing.get('status')!r}"
    facts.append(_fact("rd038_ingress_empty_set", empty_ok, detail))

    # retained-open:决策表或设计 §5 悬置行(M07/M08/M17/M45/M46/M47)
    text = CANDIDATE.read_text(encoding="utf-8") if CANDIDATE.is_file() else ""
    retained = all(x in text for x in ("M07", "M08", "M45", "M46", "M47"))
    # 诚实:若决策表尚未补裁决节,记 FAIL 留给主 agent 治理 PR
    facts.append(
        _fact(
            "retained_open_listed",
            retained,
            "G8_CANDIDATE_DECISIONS 含 M07/M08/M45/M46/M47 锚(设计 §5;主 agent 补裁决节)",
        )
    )
    return facts


def run_gate(*, evidence_dir=None) -> int:
    if NUMERIC_STEP <= 0:
        print("[wave5a] NUMERIC_STEP unset (Gov 领号后回填)", file=sys.stderr)
        return 1
    rows = [wel.require_gate_pass(k, p, evidence_dir=evidence_dir) for k, p in REQUIRED]
    code, _ = wel.emit_wave_evidence(
        wave="G8.5a",
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=(
            "CI_GATES §5;G8.5_RENDERING_COMPLETION_DESIGN §6;"
            "M19 PASS + RD-038 empty-set + retained-open"
        ),
        required_gate_rows=rows,
        extra_facts=collect_extra(evidence_dir),
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="wave5a aggregate draft; NUMERIC_STEP pending",
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
        assert NUMERIC_STEP == 116
        print("[wave5a] selftest OK")
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
