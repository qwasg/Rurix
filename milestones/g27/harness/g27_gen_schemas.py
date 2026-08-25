#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G27.1 治理波）
"""G27 evidence schema 生成器：12 份 schema 单源产出。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g27"
S = base.schema

SLUGS = [
    ("m_a", "hzb_device_kernel", "G27.2"),
    ("m_b", "m61_mesh_shader_rejudgment", "G27.2"),
    ("m_c", "cluster_p4_gap_rejudgment", "G27.3"),
    ("m_d", "hlod_l4_counter_rejudgment", "G27.3"),
    ("m_e", "closed_gate_no_regression", "G27.4"),
]

SPECS = [
    ("g27_acceptance_map_check_evidence_schema.json",
     S("g27_acceptance_map_check", "g27.wave.1.acceptance_map", "G27.1", 12, 12,
       "g27.1 acceptance map check evidence")),
    ("g27_candidate_decisions_check_evidence_schema.json",
     S("g27_candidate_decisions_check", "g27.wave.1.candidate_decisions", "G27.1", 10, 10,
       "g27.1 candidate decisions check evidence")),
    ("g27_interlock_check_evidence_schema.json",
     S("g27_interlock_check", "g27.gov.implementation_interlock", "G27.1", 8, 8,
       "g27.1 implementation interlock check evidence")),
] + [
    (f"g27_{m}_{slug}_evidence_schema.json",
     S(f"g27_{m}_{slug}", f"g27.p0.{m}.{slug}", wave, 6, None, f"g27_{m}_{slug} evidence",
       extra_states=(["skipped_dev_env"] if m in ("m_a", "m_b") else None)))
    for m, slug, wave in SLUGS
] + [
    ("g27_wave_exit_evidence_schema.json",
     S([f"g27_wave{n}_exit" for n in range(2, 7)],
       [f"g27.wave.{n}.exit" for n in range(2, 7)],
       [f"G27.{n}" for n in range(2, 7)],
       4, 4, "G27 wave exit evidence (parametrized waves 2..6)")),
    ("g27_p2_decisions_check_evidence_schema.json",
     S("g27_p2_decisions_check", "g27.wave.5a.decisions", "G27.5", 6, 6,
       "g27.5 P2 decisions check evidence")),
    ("g27_stabilization_soak_evidence_schema.json",
     S("g27_stabilization_soak", "g27.wave.5a.soak", "G27.5", 8, 8,
       "g27.5 stabilization soak evidence")),
    ("g27_wave6b_closeout_evidence_schema.json",
     S("g27_wave6b_closeout", "g27.wave.6b.closeout", "G27.6", 8, 8,
       "g27.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g27_acceptance_map_check_evidence_schema.json",
    "g27_candidate_decisions_check_evidence_schema.json",
    "g27_interlock_check_evidence_schema.json",
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
