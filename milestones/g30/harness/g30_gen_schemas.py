#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G30.1 治理波）
"""G30 evidence schema 生成器：12 份 schema 单源产出。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g30"
S = base.schema

SLUGS = [
    ("m_a", "tail_anchor_rejudgment_closure", "G30.2"),
    ("m_b", "commercial_final_review", "G30.2"),
    ("m_c", "campaign_full_chain_no_regression", "G30.3"),
    ("m_d", "campaign_handover_ledger", "G30.3"),
    ("m_e", "closed_gate_no_regression", "G30.4"),
]

SPECS = [
    ("g30_acceptance_map_check_evidence_schema.json",
     S("g30_acceptance_map_check", "g30.wave.1.acceptance_map", "G30.1", 12, 12,
       "g30.1 acceptance map check evidence")),
    ("g30_candidate_decisions_check_evidence_schema.json",
     S("g30_candidate_decisions_check", "g30.wave.1.candidate_decisions", "G30.1", 10, 10,
       "g30.1 candidate decisions check evidence")),
    ("g30_interlock_check_evidence_schema.json",
     S("g30_interlock_check", "g30.gov.implementation_interlock", "G30.1", 8, 8,
       "g30.1 implementation interlock check evidence")),
] + [
    (f"g30_{m}_{slug}_evidence_schema.json",
     S(f"g30_{m}_{slug}", f"g30.p0.{m}.{slug}", wave, 6, None, f"g30_{m}_{slug} evidence",
       extra_states=(["skipped_dev_env"] if m in ("m_a", "m_b") else None)))
    for m, slug, wave in SLUGS
] + [
    ("g30_wave_exit_evidence_schema.json",
     S([f"g30_wave{n}_exit" for n in range(2, 7)],
       [f"g30.wave.{n}.exit" for n in range(2, 7)],
       [f"G30.{n}" for n in range(2, 7)],
       4, 4, "G30 wave exit evidence (parametrized waves 2..6)")),
    ("g30_p2_decisions_check_evidence_schema.json",
     S("g30_p2_decisions_check", "g30.wave.5a.decisions", "G30.5", 6, 6,
       "g30.5 P2 decisions check evidence")),
    ("g30_stabilization_soak_evidence_schema.json",
     S("g30_stabilization_soak", "g30.wave.5a.soak", "G30.5", 8, 8,
       "g30.5 stabilization soak evidence")),
    ("g30_wave6b_closeout_evidence_schema.json",
     S("g30_wave6b_closeout", "g30.wave.6b.closeout", "G30.6", 8, 8,
       "g30.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g30_acceptance_map_check_evidence_schema.json",
    "g30_candidate_decisions_check_evidence_schema.json",
    "g30_interlock_check_evidence_schema.json",
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
