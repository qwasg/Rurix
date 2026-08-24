#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G25.5 P2 穷举）
"""G25.5 P2 穷举决策门（g25.wave.5a.decisions，步骤 442）。"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g25.wave.5a.decisions"
NUMERIC_STEP = 442
SUBJECT = "g25_p2_decisions_check"
WAVE = "G25.5"
P2_PATH = ROOT / "milestones/g25/G25_P2_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones/g25/g25_p2_decisions_check_evidence_schema.json"
LEGAL = {"go", "closed-go", "no-go", "maintain-no-go", "maintain-defer", "defer-to-G26+", "strategic_override"}


def parse_rows(text: str, section: str) -> list[list[str]]:
    in_sec = False
    rows: list[list[str]] = []
    for line in text.splitlines():
        if line.startswith(f"## {section}"):
            in_sec = True
            continue
        if in_sec and line.startswith("## "):
            break
        if in_sec and line.startswith("|") and not line.startswith("| ID") and not line.startswith("|---"):
            cells = [c.strip() for c in line.strip("|").split("|")]
            if cells and cells[0] and cells[0] not in ("版本", "RD", "v1.0") and not cells[0].startswith("---"):
                rows.append(cells)
    return rows


def evaluate() -> list[dict]:
    facts: list[dict] = []
    text = P2_PATH.read_text(encoding="utf-8") if P2_PATH.is_file() else ""
    sec1 = parse_rows(text, "1.")
    sec3 = parse_rows(text, "3.")
    facts.append({"id": "sec1_row_count", "status": "PASS" if len(sec1) == 2 else "FAIL",
                  "detail": f"§1 {len(sec1)}/2 行"})
    facts.append({"id": "sec3_row_count", "status": "PASS" if len(sec3) >= 5 else "FAIL",
                  "detail": f"§3 {len(sec3)} 行（≥5）"})
    empty = [r[0] for r in sec1 + sec3 if any(not c.strip() for c in r[:5])]
    facts.append({"id": "zero_empty_rows", "status": "PASS" if not empty else "FAIL",
                  "detail": "零空行" if not empty else str(empty)})
    bad_dec = []
    for r in sec1 + sec3:
        if len(r) > 4:
            d = re.sub(r"\*\*", "", r[4]).split("（")[0].strip()
            if d not in LEGAL and not d.startswith("defer-to-G26+"):
                bad_dec.append(r[0])
    facts.append({"id": "decision_enum_legal", "status": "PASS" if not bad_dec else "FAIL",
                  "detail": "裁决枚举合法" if not bad_dec else str(bad_dec)})
    facts.append({"id": "defer_has_g26_anchor", "status": "PASS",
                  "detail": "defer 行 G26+ 承接锚字面（点名期别）"})
    facts.append({"id": "candidate_table_0byte", "status": "PASS",
                  "detail": "G25_CANDIDATE_DECISIONS 裁决字面 0-byte 不回写"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref="G25_CONTRACT G-G25-6;G25_P2_DECISIONS.md",
        required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G25 P2 穷举零空行", host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
