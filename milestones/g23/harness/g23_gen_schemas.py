#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G23.1 治理波）
"""G23 evidence schema 生成器：12 份 schema 单源产出。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g23"
S = base.schema

SLUGS = [
    ("m_a", "jolt_56_adoption_rejudgment", "G23.2"),
    ("m_b", "neural_deform_rejudgment", "G23.2"),
    ("m_c", "research_track_disposition", "G23.3"),
    ("m_d", "physics_p3_subitem_disposition", "G23.3"),
    ("m_e", "closed_gate_no_regression", "G23.4"),
]

SPECS = [
    ("g23_acceptance_map_check_evidence_schema.json",
     S("g23_acceptance_map_check", "g23.wave.1.acceptance_map", "G23.1", 12, 12,
       "g23.1 acceptance map check evidence")),
    ("g23_candidate_decisions_check_evidence_schema.json",
     S("g23_candidate_decisions_check", "g23.wave.1.candidate_decisions", "G23.1", 10, 10,
       "g23.1 candidate decisions check evidence")),
    ("g23_interlock_check_evidence_schema.json",
     S("g23_interlock_check", "g23.gov.implementation_interlock", "G23.1", 8, 8,
       "g23.1 implementation interlock check evidence")),
] + [
    (f"g23_{m}_{slug}_evidence_schema.json",
     S(f"g23_{m}_{slug}", f"g23.p0.{m}.{slug}", wave, 6, None, f"g23_{m}_{slug} evidence"))
    for m, slug, wave in SLUGS
] + [
    ("g23_wave_exit_evidence_schema.json",
     S([f"g23_wave{n}_exit" for n in range(2, 7)],
       [f"g23.wave.{n}.exit" for n in range(2, 7)],
       [f"G23.{n}" for n in range(2, 7)],
       4, 4, "G23 wave exit evidence (parametrized waves 2..6)")),
    ("g23_p2_decisions_check_evidence_schema.json",
     S("g23_p2_decisions_check", "g23.wave.5a.decisions", "G23.5", 6, 6,
       "g23.5 P2 decisions check evidence")),
    ("g23_stabilization_soak_evidence_schema.json",
     S("g23_stabilization_soak", "g23.wave.5a.soak", "G23.5", 8, 8,
       "g23.5 stabilization soak evidence")),
    ("g23_wave6b_closeout_evidence_schema.json",
     S("g23_wave6b_closeout", "g23.wave.6b.closeout", "G23.6", 8, 8,
       "g23.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g23_acceptance_map_check_evidence_schema.json",
    "g23_candidate_decisions_check_evidence_schema.json",
    "g23_interlock_check_evidence_schema.json",
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
