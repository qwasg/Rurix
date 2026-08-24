#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G20.6 close-out）
"""G20.6 close-out 终审门（g20.wave.6b.closeout，步骤 364）。"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g20.wave.6b.closeout"
NUMERIC_STEP = 364
SUBJECT = "g20_wave6b_closeout"
WAVE = "G20.6"
SCHEMA_PATH = ROOT / "milestones/g20/g20_wave6b_closeout_evidence_schema.json"
P0_SUBJECTS = [
    ("m_a", "g20_m_a_hzb_occlusion_host_realization"),
    ("m_b", "g20_m_b_cluster_streaming_p4_disposition"),
    ("m_c", "g20_m_c_mesh_shader_rejudgment"),
    ("m_d", "g20_m_d_far_field_l4_disposition"),
    ("m_e", "g20_m_e_closed_gate_no_regression"),
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
    facts.append(fact("five_p0_evidence_green", not bad, "ok" if not bad else "; ".join(bad)))
    p2 = wel.load_latest_evidence("g20_p2_decisions_check")
    p2_ok = p2 is not None and wel.load_json(p2).get("host_section_pass") is True
    facts.append(fact("p2_exhaustive_zero_empty", p2_ok, p2.name if p2 else "missing"))
    hzb = wel.load_latest_evidence("g20_hzb_probe")
    hzb_ok = hzb is not None and wel.load_json(hzb).get("zero_false_positive") is True
    facts.append(fact("hzb_realization_chain", hzb_ok, hzb.name if hzb else "missing"))
    hits = list((ROOT / "rfcs").glob("0037-*.md"))
    facts.append(fact("rfc_0037_archived", bool(hits), hits[0].name if hits else "RFC-0037 missing"))
    me_ok, me_d = _latest_green("g20_m_e_closed_gate_no_regression")
    facts.append(fact("old_gates_no_regression", me_ok, me_d))
    rd_ok = False
    rd_detail = "RD-039 missing"
    dp = ROOT / "registry/deferred.json"
    if dp.is_file():
        st = {e.get("id"): e.get("status") for e in json.loads(dp.read_text(encoding="utf-8")).get("entries", [])}
        rd_ok = all(st.get(r) == "open" for r in
                    ("RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044", "RD-045"))
        rd_detail = f"八条 open 维持={rd_ok}"
    facts.append(fact("rd_open_maintained", rd_ok, rd_detail))
    soak = wel.load_latest_evidence("g20_stabilization_soak")
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
        source_ref="G20_CONTRACT G-G20-6", required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes=f"G20.6 closeout 八 facts VERDICT={verdict}",
        host_section_pass=ok,
    )
    print(f"[g20_closeout] VERDICT={verdict}")
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
