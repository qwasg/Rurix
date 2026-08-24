#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G20.1 治理波）
"""G20 evidence schema 生成器：12 份 schema 单源产出（G19 生成器同构）。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g20"
S = base.schema

SPECS = [
    ("g20_acceptance_map_check_evidence_schema.json",
     S("g20_acceptance_map_check", "g20.wave.1.acceptance_map", "G20.1", 12, 12,
       "g20.1 acceptance map check evidence")),
    ("g20_candidate_decisions_check_evidence_schema.json",
     S("g20_candidate_decisions_check", "g20.wave.1.candidate_decisions", "G20.1", 10, 10,
       "g20.1 candidate decisions check evidence")),
    ("g20_interlock_check_evidence_schema.json",
     S("g20_interlock_check", "g20.gov.implementation_interlock", "G20.1", 8, 8,
       "g20.1 implementation interlock check evidence")),
    ("g20_m_a_hzb_occlusion_host_realization_evidence_schema.json",
     S("g20_m_a_hzb_occlusion_host_realization", "g20.p0.m_a.hzb_occlusion_host_realization",
       "G20.2", 6, None, "g20_m_a_hzb_occlusion_host_realization evidence")),
    ("g20_m_b_cluster_streaming_p4_disposition_evidence_schema.json",
     S("g20_m_b_cluster_streaming_p4_disposition", "g20.p0.m_b.cluster_streaming_p4_disposition",
       "G20.2", 6, None, "g20_m_b_cluster_streaming_p4_disposition evidence")),
    ("g20_m_c_mesh_shader_rejudgment_evidence_schema.json",
     S("g20_m_c_mesh_shader_rejudgment", "g20.p0.m_c.mesh_shader_rejudgment",
       "G20.3", 6, None, "g20_m_c_mesh_shader_rejudgment evidence")),
    ("g20_m_d_far_field_l4_disposition_evidence_schema.json",
     S("g20_m_d_far_field_l4_disposition", "g20.p0.m_d.far_field_l4_disposition",
       "G20.3", 6, None, "g20_m_d_far_field_l4_disposition evidence")),
    ("g20_m_e_closed_gate_no_regression_evidence_schema.json",
     S("g20_m_e_closed_gate_no_regression", "g20.p0.m_e.closed_gate_no_regression",
       "G20.4", 6, None, "g20_m_e_closed_gate_no_regression evidence")),
    ("g20_wave_exit_evidence_schema.json",
     S([f"g20_wave{n}_exit" for n in range(2, 7)],
       [f"g20.wave.{n}.exit" for n in range(2, 7)],
       [f"G20.{n}" for n in range(2, 7)],
       4, 4, "G20 wave exit evidence (parametrized waves 2..6)")),
    ("g20_p2_decisions_check_evidence_schema.json",
     S("g20_p2_decisions_check", "g20.wave.5a.decisions", "G20.5", 6, 6,
       "g20.5 P2 decisions check evidence")),
    ("g20_stabilization_soak_evidence_schema.json",
     S("g20_stabilization_soak", "g20.wave.5a.soak", "G20.5", 8, 8,
       "g20.5 stabilization soak evidence")),
    ("g20_wave6b_closeout_evidence_schema.json",
     S("g20_wave6b_closeout", "g20.wave.6b.closeout", "G20.6", 8, 8,
       "g20.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g20_acceptance_map_check_evidence_schema.json",
    "g20_candidate_decisions_check_evidence_schema.json",
    "g20_interlock_check_evidence_schema.json",
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
