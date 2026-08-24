#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G21.1 治理波）
"""G21 evidence schema 生成器：12 份 schema 单源产出（G19/G20 生成器同构）。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g21"
S = base.schema

SPECS = [
    ("g21_acceptance_map_check_evidence_schema.json",
     S("g21_acceptance_map_check", "g21.wave.1.acceptance_map", "G21.1", 12, 12,
       "g21.1 acceptance map check evidence")),
    ("g21_candidate_decisions_check_evidence_schema.json",
     S("g21_candidate_decisions_check", "g21.wave.1.candidate_decisions", "G21.1", 10, 10,
       "g21.1 candidate decisions check evidence")),
    ("g21_interlock_check_evidence_schema.json",
     S("g21_interlock_check", "g21.gov.implementation_interlock", "G21.1", 8, 8,
       "g21.1 implementation interlock check evidence")),
    ("g21_m_a_restir_high_reservoir_realization_evidence_schema.json",
     S("g21_m_a_restir_high_reservoir_realization", "g21.p0.m_a.restir_high_reservoir_realization",
       "G21.2", 6, None, "g21_m_a_restir_high_reservoir_realization evidence")),
    ("g21_m_b_ser_capability_disposition_evidence_schema.json",
     S("g21_m_b_ser_capability_disposition", "g21.p0.m_b.ser_capability_disposition",
       "G21.2", 6, None, "g21_m_b_ser_capability_disposition evidence")),
    ("g21_m_c_rd040_subitem_disposition_evidence_schema.json",
     S("g21_m_c_rd040_subitem_disposition", "g21.p0.m_c.rd040_subitem_disposition",
       "G21.3", 6, None, "g21_m_c_rd040_subitem_disposition evidence")),
    ("g21_m_d_rd034_upstream_recheck_evidence_schema.json",
     S("g21_m_d_rd034_upstream_recheck", "g21.p0.m_d.rd034_upstream_recheck",
       "G21.3", 6, None, "g21_m_d_rd034_upstream_recheck evidence")),
    ("g21_m_e_closed_gate_no_regression_evidence_schema.json",
     S("g21_m_e_closed_gate_no_regression", "g21.p0.m_e.closed_gate_no_regression",
       "G21.4", 6, None, "g21_m_e_closed_gate_no_regression evidence")),
    ("g21_wave_exit_evidence_schema.json",
     S([f"g21_wave{n}_exit" for n in range(2, 7)],
       [f"g21.wave.{n}.exit" for n in range(2, 7)],
       [f"G21.{n}" for n in range(2, 7)],
       4, 4, "G21 wave exit evidence (parametrized waves 2..6)")),
    ("g21_p2_decisions_check_evidence_schema.json",
     S("g21_p2_decisions_check", "g21.wave.5a.decisions", "G21.5", 6, 6,
       "g21.5 P2 decisions check evidence")),
    ("g21_stabilization_soak_evidence_schema.json",
     S("g21_stabilization_soak", "g21.wave.5a.soak", "G21.5", 8, 8,
       "g21.5 stabilization soak evidence")),
    ("g21_wave6b_closeout_evidence_schema.json",
     S("g21_wave6b_closeout", "g21.wave.6b.closeout", "G21.6", 8, 8,
       "g21.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g21_acceptance_map_check_evidence_schema.json",
    "g21_candidate_decisions_check_evidence_schema.json",
    "g21_interlock_check_evidence_schema.json",
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
