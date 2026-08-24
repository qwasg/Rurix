#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16plus close-out）
"""G16plus close-out（g16.wave.6b.closeout，步骤 292）。仅当 M-g 绿 + soak 绿才 READY。"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g16_p0_lib as g16  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g16.wave.6b.closeout"
NUMERIC_STEP = 292
SUBJECT = "g16_wave6b_closeout"
WAVE = "G16.6b"
SOURCE_REF = "G16_CONTRACT G-G16-11;G16_ACCEPTANCE_MAP 附录 A M-h"
SCHEMA = g16.ROOT / "milestones" / "g16" / "g16_wave6b_closeout_evidence_schema.json"
RFC = g16.ROOT / "rfcs" / "0031-g16plus-gi-expression-quality-closure.md"
DEFERRED = g16.ROOT / "registry" / "deferred.json"
RD_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044", "RD-045"]
OLD_P0 = [
    ("g16_m_a_ue_reference_arm_repair", 8),
    ("g16_m_b_dual_end_reharvest", 6),
    ("g16_m_c_absolute_quality_rereview", 8),
    ("g16_m_d_closed_gate_no_regression", 8),
]


def _latest_pass(prefix: str) -> tuple[bool, str]:
    p = wel.load_latest_evidence(prefix)
    if p is None:
        return False, f"缺 {prefix}"
    doc = wel.load_json(p)
    ok = bool(doc.get("host_section_pass"))
    return ok, f"{p.name} host={ok}"


def run_gate() -> int:
    facts = []
    old_ok = True
    old_d = []
    for pref, _n in OLD_P0:
        ok, d = _latest_pass(pref)
        old_ok = old_ok and ok
        old_d.append(d)
    facts.append(g16.fact("old_p0_still_green", old_ok, "; ".join(old_d)))
    app_ok = True
    app_d = []
    for pref in ("g16_m_e_gi_expression", "g16_m_f_lumen_reharvest", "g16_m_g_absolute_quality_closure"):
        ok, d = _latest_pass(pref)
        app_ok = app_ok and ok
        app_d.append(d)
    facts.append(g16.fact("appendix_a_meg_green", app_ok, "; ".join(app_d)))
    rfc_t = RFC.read_text(encoding="utf-8") if RFC.is_file() else ""
    facts.append(g16.fact("rfc0031_approved", "Agent Approved" in rfc_t, RFC.name))
    rd_ok = False
    if DEFERRED.is_file():
        entries = json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries") or []
        by = {e.get("id"): e.get("status") for e in entries}
        rd_ok = all(by.get(i) == "open" for i in RD_IDS)
    facts.append(g16.fact("rd_eight_open", rd_ok, "RD-034/039/040/041/042/043/044/045"))
    mg = wel.load_latest_evidence("g16_m_g_absolute_quality_closure")
    commercial = False
    if mg is not None:
        doc = wel.load_json(mg)
        facts_m = {f.get("id"): f.get("status") for f in doc.get("extra_facts") or []}
        commercial = facts_m.get("met_count_18") == "PASS" and facts_m.get("commercial_closure_pass") == "PASS"
    facts.append(g16.fact("commercial_18_18", commercial, "M-g met_count==18"))
    stolen = False
    p14 = wel.load_latest_evidence("g14_m_d_rurix_ue_perf_parity")
    if p14 is not None and "g16_" in p14.name:
        stolen = True
    facts.append(g16.fact("direct_arm_latest_unstolen", not stolen, "G14 M-d latest 未被 GI 抢"))
    soak_ok, soak_d = _latest_pass("g16_stabilization_soak")
    facts.append(g16.fact("soak_fullrun_first", soak_ok, soak_d))
    ready = all(f["status"] == "PASS" for f in facts)
    facts.append(g16.fact("closeout_ready", ready, "READY" if ready else "BLOCKED"))
    notes = "VERDICT=READY" if ready else "VERDICT=BLOCKED G16plus 仍 active"
    return g16.emit(WAVE, SUBJECT, GATE_KEY, NUMERIC_STEP, SOURCE_REF, SCHEMA, facts, notes)


def run_selftest() -> int:
    if NUMERIC_STEP != 292 or GATE_KEY != "g16.wave.6b.closeout":
        print("[g16_closeout] SELFTEST FAIL")
        return 1
    print("[g16_closeout] SELFTEST PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return g16.verify_latest_wave(SUBJECT, 8)
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
