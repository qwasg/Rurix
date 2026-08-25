#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G28.1 治理波）
"""G28 evidence schema 生成器：12 份 schema 单源产出。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g28"
S = base.schema

SLUGS = [
    ("m_a", "restir_device_kernel", "G28.2"),
    ("m_b", "restir_spatial_reuse_arm", "G28.2"),
    ("m_c", "m52_rd040_workload_rejudgment", "G28.3"),
    ("m_d", "rd034_upstream_recheck", "G28.3"),
    ("m_e", "closed_gate_no_regression", "G28.4"),
]

SPECS = [
    ("g28_acceptance_map_check_evidence_schema.json",
     S("g28_acceptance_map_check", "g28.wave.1.acceptance_map", "G28.1", 12, 12,
       "g28.1 acceptance map check evidence")),
    ("g28_candidate_decisions_check_evidence_schema.json",
     S("g28_candidate_decisions_check", "g28.wave.1.candidate_decisions", "G28.1", 10, 10,
       "g28.1 candidate decisions check evidence")),
    ("g28_interlock_check_evidence_schema.json",
     S("g28_interlock_check", "g28.gov.implementation_interlock", "G28.1", 8, 8,
       "g28.1 implementation interlock check evidence")),
] + [
    (f"g28_{m}_{slug}_evidence_schema.json",
     S(f"g28_{m}_{slug}", f"g28.p0.{m}.{slug}", wave, 6, None, f"g28_{m}_{slug} evidence",
       extra_states=(["skipped_dev_env"] if m in ("m_a", "m_b") else None)))
    for m, slug, wave in SLUGS
] + [
    ("g28_wave_exit_evidence_schema.json",
     S([f"g28_wave{n}_exit" for n in range(2, 7)],
       [f"g28.wave.{n}.exit" for n in range(2, 7)],
       [f"G28.{n}" for n in range(2, 7)],
       4, 4, "G28 wave exit evidence (parametrized waves 2..6)")),
    ("g28_p2_decisions_check_evidence_schema.json",
     S("g28_p2_decisions_check", "g28.wave.5a.decisions", "G28.5", 6, 6,
       "g28.5 P2 decisions check evidence")),
    ("g28_stabilization_soak_evidence_schema.json",
     S("g28_stabilization_soak", "g28.wave.5a.soak", "G28.5", 8, 8,
       "g28.5 stabilization soak evidence")),
    ("g28_wave6b_closeout_evidence_schema.json",
     S("g28_wave6b_closeout", "g28.wave.6b.closeout", "G28.6", 8, 8,
       "g28.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g28_acceptance_map_check_evidence_schema.json",
    "g28_candidate_decisions_check_evidence_schema.json",
    "g28_interlock_check_evidence_schema.json",
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
