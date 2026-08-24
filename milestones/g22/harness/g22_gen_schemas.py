#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G22.1 治理波）
"""G22 evidence schema 生成器：12 份 schema 单源产出（G19~G21 生成器同构）。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g22"
S = base.schema

SLUGS = [
    ("m_a", "slab_material_host_realization", "G22.2"),
    ("m_b", "svt_disposition", "G22.2"),
    ("m_c", "ktx2_basisu_disposition", "G22.3"),
    ("m_d", "work_graphs_fsr_reeval_disposition", "G22.3"),
    ("m_e", "closed_gate_no_regression", "G22.4"),
]

SPECS = [
    ("g22_acceptance_map_check_evidence_schema.json",
     S("g22_acceptance_map_check", "g22.wave.1.acceptance_map", "G22.1", 12, 12,
       "g22.1 acceptance map check evidence")),
    ("g22_candidate_decisions_check_evidence_schema.json",
     S("g22_candidate_decisions_check", "g22.wave.1.candidate_decisions", "G22.1", 10, 10,
       "g22.1 candidate decisions check evidence")),
    ("g22_interlock_check_evidence_schema.json",
     S("g22_interlock_check", "g22.gov.implementation_interlock", "G22.1", 8, 8,
       "g22.1 implementation interlock check evidence")),
] + [
    (f"g22_{m}_{slug}_evidence_schema.json",
     S(f"g22_{m}_{slug}", f"g22.p0.{m}.{slug}", wave, 6, None, f"g22_{m}_{slug} evidence"))
    for m, slug, wave in SLUGS
] + [
    ("g22_wave_exit_evidence_schema.json",
     S([f"g22_wave{n}_exit" for n in range(2, 7)],
       [f"g22.wave.{n}.exit" for n in range(2, 7)],
       [f"G22.{n}" for n in range(2, 7)],
       4, 4, "G22 wave exit evidence (parametrized waves 2..6)")),
    ("g22_p2_decisions_check_evidence_schema.json",
     S("g22_p2_decisions_check", "g22.wave.5a.decisions", "G22.5", 6, 6,
       "g22.5 P2 decisions check evidence")),
    ("g22_stabilization_soak_evidence_schema.json",
     S("g22_stabilization_soak", "g22.wave.5a.soak", "G22.5", 8, 8,
       "g22.5 stabilization soak evidence")),
    ("g22_wave6b_closeout_evidence_schema.json",
     S("g22_wave6b_closeout", "g22.wave.6b.closeout", "G22.6", 8, 8,
       "g22.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g22_acceptance_map_check_evidence_schema.json",
    "g22_candidate_decisions_check_evidence_schema.json",
    "g22_interlock_check_evidence_schema.json",
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
