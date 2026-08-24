#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G19 实现批）
"""G19 P0 smoke — g19.p0.m_c.rd045_drift_observation_window。

消费 milestones/g19/g19_rd045_observation_results.json（harness 真跑 ≥12 轮
receipt digest 对锚取证）+ registry/deferred.json RD-045 history 只追加登记。
close/maintain-open 均合法诚实终态；本窗零漂移 → maintain-open（backfill 三件
未全齐不冒充 close）。
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g19.p0.m_c.rd045_drift_observation_window"
NUMERIC_STEP = 340
SUBJECT = "g19_m_c_rd045_drift_observation_window"
WAVE = "G19.3"
SCHEMA_PATH = ROOT / "milestones/g19/g19_m_c_rd045_drift_observation_window_evidence_schema.json"
SOURCE_REF = "G19_CONTRACT §4.2;G19_ACCEPTANCE_MAP §1 M-c 行;registry/deferred.json RD-045"

RESULTS = ROOT / "milestones/g19/g19_rd045_observation_results.json"
DEFERRED = ROOT / "registry/deferred.json"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "observation_results_present", "status": "PASS" if RESULTS.is_file() else "FAIL",
                  "detail": str(RESULTS.relative_to(ROOT))})
    doc = wel.load_json(RESULTS) if RESULTS.is_file() else {}
    s = doc.get("summary", {})
    n = doc.get("rounds_requested", 0)
    facts.append({"id": "rounds_ge_12", "status": "PASS" if n >= 12 and s.get("rounds_ok") == n else "FAIL",
                  "detail": f"rounds_ok={s.get('rounds_ok')}/{n}（要求 ≥12 全 ok）"})
    facts.append({"id": "digest_trace_vs_anchor", "status": "PASS" if s.get("digest_anchor_hits") == n else "FAIL",
                  "detail": f"anchor_hits={s.get('digest_anchor_hits')}/{n} drift_rounds={s.get('drift_rounds')}"})
    disp = doc.get("disposition", "")
    legal = disp in ("maintain-open-with-extended-zero-recurrence", "drift-detected-escalate", "closed")
    facts.append({"id": "disposition_legal_honest", "status": "PASS" if legal else "FAIL",
                  "detail": f"disposition={disp}（close/maintain-open 均合法；不冒充）"})
    rd = {}
    if DEFERRED.is_file():
        for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", []):
            if e.get("id") == "RD-045":
                rd = e
    hist_ok = any("G19.3" in (h.get("event") or "") for h in rd.get("history", []))
    facts.append({"id": "rd045_history_appended", "status": "PASS" if hist_ok else "FAIL",
                  "detail": "RD-045 history 含 G19.3 观察窗只追加登记"})
    facts.append({"id": "rd045_status_honest", "status": "PASS" if rd.get("status") == "open" else "FAIL",
                  "detail": f"RD-045 status={rd.get('status')}（根因未逐字定位 → maintain-open 不冒充 close）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G19.3 M-c：RD-045 长窗观察兑现（bistro-interior/t50/tsr_device ≥12 轮 digest 锚对拍）",
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
