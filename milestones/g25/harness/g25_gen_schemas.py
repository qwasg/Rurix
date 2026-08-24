#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G25.1 治理波）
"""G25 evidence schema 生成器：12 份 schema 单源产出。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g25"
S = base.schema

SLUGS = [
    ("m_a", "quality_final_state_verification", "G25.2"),
    ("m_b", "fps_parity_final_verdict", "G25.2"),
    ("m_c", "campaign_full_chain_no_regression", "G25.3"),
    ("m_d", "campaign_handover_ledger", "G25.3"),
    ("m_e", "closed_gate_no_regression", "G25.4"),
]

SPECS = [
    ("g25_acceptance_map_check_evidence_schema.json",
     S("g25_acceptance_map_check", "g25.wave.1.acceptance_map", "G25.1", 12, 12,
       "g25.1 acceptance map check evidence")),
    ("g25_candidate_decisions_check_evidence_schema.json",
     S("g25_candidate_decisions_check", "g25.wave.1.candidate_decisions", "G25.1", 10, 10,
       "g25.1 candidate decisions check evidence")),
    ("g25_interlock_check_evidence_schema.json",
     S("g25_interlock_check", "g25.gov.implementation_interlock", "G25.1", 8, 8,
       "g25.1 implementation interlock check evidence")),
] + [
    (f"g25_{m}_{slug}_evidence_schema.json",
     S(f"g25_{m}_{slug}", f"g25.p0.{m}.{slug}", wave, 6, None, f"g25_{m}_{slug} evidence"))
    for m, slug, wave in SLUGS
] + [
    ("g25_wave_exit_evidence_schema.json",
     S([f"g25_wave{n}_exit" for n in range(2, 7)],
       [f"g25.wave.{n}.exit" for n in range(2, 7)],
       [f"G25.{n}" for n in range(2, 7)],
       4, 4, "G25 wave exit evidence (parametrized waves 2..6)")),
    ("g25_p2_decisions_check_evidence_schema.json",
     S("g25_p2_decisions_check", "g25.wave.5a.decisions", "G25.5", 6, 6,
       "g25.5 P2 decisions check evidence")),
    ("g25_stabilization_soak_evidence_schema.json",
     S("g25_stabilization_soak", "g25.wave.5a.soak", "G25.5", 8, 8,
       "g25.5 stabilization soak evidence")),
    ("g25_wave6b_closeout_evidence_schema.json",
     S("g25_wave6b_closeout", "g25.wave.6b.closeout", "G25.6", 8, 8,
       "g25.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g25_acceptance_map_check_evidence_schema.json",
    "g25_candidate_decisions_check_evidence_schema.json",
    "g25_interlock_check_evidence_schema.json",
}


def main(governance_only: bool = False) -> int:
    import json
    for name, doc in SPECS:
        if governance_only and name not in GOVERNANCE_ONLY:
            continue
        p = OUT / name
        p.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
        print(f"[gen_schemas] → {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(governance_only="--governance-only" in sys.argv))
