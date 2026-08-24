#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G24.1 治理波）
"""G24 evidence schema 生成器：12 份 schema 单源产出。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g24"
S = base.schema

SLUGS = [
    ("m_a", "hair_strand_oit_rejudgment", "G24.2"),
    ("m_b", "hdr_calibration_rejudgment", "G24.2"),
    ("m_c", "bistro_exterior_conversion_rejudgment", "G24.3"),
    ("m_d", "safe_gpu_and_legacy_rd_disposition", "G24.3"),
    ("m_e", "closed_gate_no_regression", "G24.4"),
]

SPECS = [
    ("g24_acceptance_map_check_evidence_schema.json",
     S("g24_acceptance_map_check", "g24.wave.1.acceptance_map", "G24.1", 12, 12,
       "g24.1 acceptance map check evidence")),
    ("g24_candidate_decisions_check_evidence_schema.json",
     S("g24_candidate_decisions_check", "g24.wave.1.candidate_decisions", "G24.1", 10, 10,
       "g24.1 candidate decisions check evidence")),
    ("g24_interlock_check_evidence_schema.json",
     S("g24_interlock_check", "g24.gov.implementation_interlock", "G24.1", 8, 8,
       "g24.1 implementation interlock check evidence")),
] + [
    (f"g24_{m}_{slug}_evidence_schema.json",
     S(f"g24_{m}_{slug}", f"g24.p0.{m}.{slug}", wave, 6, None, f"g24_{m}_{slug} evidence"))
    for m, slug, wave in SLUGS
] + [
    ("g24_wave_exit_evidence_schema.json",
     S([f"g24_wave{n}_exit" for n in range(2, 7)],
       [f"g24.wave.{n}.exit" for n in range(2, 7)],
       [f"G24.{n}" for n in range(2, 7)],
       4, 4, "G24 wave exit evidence (parametrized waves 2..6)")),
    ("g24_p2_decisions_check_evidence_schema.json",
     S("g24_p2_decisions_check", "g24.wave.5a.decisions", "G24.5", 6, 6,
       "g24.5 P2 decisions check evidence")),
    ("g24_stabilization_soak_evidence_schema.json",
     S("g24_stabilization_soak", "g24.wave.5a.soak", "G24.5", 8, 8,
       "g24.5 stabilization soak evidence")),
    ("g24_wave6b_closeout_evidence_schema.json",
     S("g24_wave6b_closeout", "g24.wave.6b.closeout", "G24.6", 8, 8,
       "g24.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g24_acceptance_map_check_evidence_schema.json",
    "g24_candidate_decisions_check_evidence_schema.json",
    "g24_interlock_check_evidence_schema.json",
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
