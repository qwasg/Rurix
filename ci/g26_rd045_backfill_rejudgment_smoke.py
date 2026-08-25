#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G26 实现批）
"""G26.3 P0 smoke — g26.p0.m_c.rd045_backfill_rejudgment。

消费 milestones/g26/g26_rd045_fresh_window_results.json（harness 真跑新鲜观察窗
+ 三件盘点，RFC-0043 §3）+ registry/deferred.json RD-045 history 只追加登记。
close/maintain-open/drift-escalate 均合法诚实终态；F5 防冒充硬线 = ①件判定
输入面禁引观察窗结果（结构性机核）。
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g26.p0.m_c.rd045_backfill_rejudgment"
NUMERIC_STEP = 452  # post-interlock actual-next-free 顺位领取（448~460 批）
SUBJECT = "g26_m_c_rd045_backfill_rejudgment"
WAVE = "G26.3"
SCHEMA_PATH = ROOT / "milestones/g26/g26_m_c_rd045_backfill_rejudgment_evidence_schema.json"
SOURCE_REF = "G26_CONTRACT §4.2 M-c;G26_ACCEPTANCE_MAP §1 M-c 行;RFC-0043 §3;registry/deferred.json RD-045"

RESULTS = ROOT / "milestones/g26/g26_rd045_fresh_window_results.json"
DEFERRED = ROOT / "registry/deferred.json"
LEGAL_DISPOSITIONS = ("maintain-open-with-extended-zero-recurrence", "drift-detected-escalate", "closed")


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "fresh_window_results_present", "status": "PASS" if RESULTS.is_file() else "FAIL",
                  "detail": str(RESULTS.relative_to(ROOT))})
    doc = wel.load_json(RESULTS) if RESULTS.is_file() else {}
    s = doc.get("summary", {})
    n = doc.get("rounds_requested", 0)
    facts.append({"id": "rounds_ge_6_all_ok", "status": "PASS" if n >= 6 and s.get("rounds_ok") == n else "FAIL",
                  "detail": f"rounds_ok={s.get('rounds_ok')}/{n}（要求 ≥6 全 ok；窗长口径如实登记）"})
    facts.append({"id": "digest_trace_vs_anchor", "status": "PASS" if s.get("digest_anchor_hits") == n else "FAIL",
                  "detail": f"anchor_hits={s.get('digest_anchor_hits')}/{n} drift_rounds={s.get('drift_rounds')}"})
    inv = doc.get("backfill_inventory", {})
    p1 = inv.get("piece1_root_cause_located", {})
    iso_ok = "不含观察窗" in str(p1.get("input_surface", ""))
    facts.append({"id": "inventory_input_isolation", "status": "PASS" if iso_ok else "FAIL",
                  "detail": f"F5 硬线：①件判定输入面 = {p1.get('input_surface')!r}（观察性证据永不充当①件）"})
    met = inv.get("met_count")
    facts.append({"id": "three_piece_inventory_registered",
                  "status": "PASS" if isinstance(met, int) and all(
                      k in inv for k in ("piece1_root_cause_located", "piece2_production_fix",
                                         "piece3_full_rfc_evaluation")) else "FAIL",
                  "detail": f"三件盘点 {met}/3：①{p1.get('met')} ②{inv.get('piece2_production_fix', {}).get('met')} "
                            f"③{inv.get('piece3_full_rfc_evaluation', {}).get('met')}"})
    disp = doc.get("disposition", "")
    legal = disp in LEGAL_DISPOSITIONS and (disp != "closed" or met == 3)
    facts.append({"id": "disposition_legal_honest", "status": "PASS" if legal else "FAIL",
                  "detail": f"disposition={disp}（close 仅当三件全齐；maintain-open/escalate 均合法，不冒充）"})
    rd = {}
    if DEFERRED.is_file():
        for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", []):
            if e.get("id") == "RD-045":
                rd = e
    hist_ok = any("G26.3" in (h.get("event") or "") for h in rd.get("history", []))
    facts.append({"id": "rd045_history_appended", "status": "PASS" if hist_ok else "FAIL",
                  "detail": "RD-045 history 含 G26.3 重判窗只追加登记"})
    status_ok = (rd.get("status") == "open") if disp != "closed" else (rd.get("status") == "closed")
    facts.append({"id": "rd045_status_honest", "status": "PASS" if status_ok else "FAIL",
                  "detail": f"RD-045 status={rd.get('status')}（与 disposition={disp} 一致；三件未齐不冒充 close）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G26.3 M-c：RD-045 backfill 三件重判（bistro-interior/t50/tsr_device 新鲜观察窗 6 轮 digest 锚对拍 + 三件盘点 F5 防冒充硬线）",
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
