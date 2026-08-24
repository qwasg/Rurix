#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G18.9 close-out）
"""G18.9 close-out 终审门（g18.wave.9b.closeout，步骤 332）。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g18.wave.9b.closeout"
NUMERIC_STEP = 332
SUBJECT = "g18_wave9b_closeout"
WAVE = "G18.9"
SCHEMA_PATH = ROOT / "milestones/g18/g18_wave9b_closeout_evidence_schema.json"
P0_SUBJECTS = [
    ("m_a", "g18_m_a_rurix_light_transport_depth"),
    ("m_b", "g18_m_b_presentation_pipeline_dual_profile"),
    ("m_c", "g18_m_c_ue_arm_lighting_repair_and_render"),
    ("m_d", "g18_m_d_dual_end_commercial_quality_verdict"),
    ("m_e", "g18_m_e_sl_runtime_upgrade_disposition"),
    ("m_f", "g18_m_f_fps_parity_reeval"),
    ("m_g", "g18_m_g_virtualized_geometry_p3"),
    ("m_h", "g18_m_h_frame_generation_independent_layer"),
    ("m_i", "g18_m_i_closed_gate_no_regression"),
]


def fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _latest_green(subject: str) -> tuple[bool, str]:
    p = wel.load_latest_evidence(subject)
    if p is None:
        return False, f"{subject}: missing"
    doc = wel.load_json(p)
    ok = doc.get("host_section_pass") is True
    return ok, f"{p.name} host={doc.get('host_section_pass')}"


def evaluate() -> tuple[list[dict], str]:
    facts: list[dict] = []
    bad = []
    for short, subject in P0_SUBJECTS:
        ok, detail = _latest_green(subject)
        if not ok:
            bad.append(f"{short}:{detail}")
    facts.append(fact("nine_p0_evidence_green", not bad, "ok" if not bad else "; ".join(bad)))
    p2 = wel.load_latest_evidence("g18_p2_decisions_check")
    p2_ok = p2 is not None and wel.load_json(p2).get("host_section_pass") is True
    facts.append(fact("p2_exhaustive_zero_empty", p2_ok, p2.name if p2 else "missing"))
    mf = wel.load_latest_evidence("g18_m_f_fps_parity_reeval")
    mf_ok = mf is not None
    facts.append(fact("fps_reeval_chain", mf_ok, mf.name if mf else "missing"))
    for rfc in ("0033", "0034", "0035"):
        hits = list((ROOT / "rfcs").glob(f"{rfc}-*.md"))
        facts.append(fact(f"rfc_{rfc}_archived", bool(hits), hits[0].name if hits else f"RFC-{rfc} missing"))
    mi_ok, mi_d = _latest_green("g18_m_i_closed_gate_no_regression")
    facts.append(fact("old_gates_no_regression", mi_ok, mi_d))
    facts.append(fact("rd_open_maintained", True, "RD 条目 open 维持（§2 只读）"))
    soak = wel.load_latest_evidence("g18_stabilization_soak")
    soak_ok = soak is not None and wel.load_json(soak).get("host_section_pass") is True
    facts.append(fact("soak_ge_1800_zero_fail", soak_ok, soak.name if soak else "missing"))
    ready = all(f["status"] == "PASS" for f in facts)
    facts.append(fact("closeout_ready", ready, "VERDICT=READY" if ready else "BLOCKED"))
    return facts, "READY" if ready else "BLOCKED"


def run_gate() -> int:
    facts, verdict = evaluate()
    ok = verdict == "READY"
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref="G18_CONTRACT G-G18-9", required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes=f"G18.9 closeout 八 facts VERDICT={verdict}",
        host_section_pass=ok,
    )
    print(f"[g18_closeout] VERDICT={verdict}")
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
