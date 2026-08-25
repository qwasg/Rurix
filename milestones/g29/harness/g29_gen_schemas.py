#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G29.1 治理波）
"""G29 evidence schema 生成器：12 份 schema 单源产出。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g29"
S = base.schema

SLUGS = [
    ("m_a", "slab_device_kernel", "G29.2"),
    ("m_b", "slab_side_table_arm", "G29.2"),
    ("m_c", "svt_ktx2_gap_rejudgment", "G29.3"),
    ("m_d", "wg_dgc_capability_recheck", "G29.3"),
    ("m_e", "closed_gate_no_regression", "G29.4"),
]

SPECS = [
    ("g29_acceptance_map_check_evidence_schema.json",
     S("g29_acceptance_map_check", "g29.wave.1.acceptance_map", "G29.1", 12, 12,
       "g29.1 acceptance map check evidence")),
    ("g29_candidate_decisions_check_evidence_schema.json",
     S("g29_candidate_decisions_check", "g29.wave.1.candidate_decisions", "G29.1", 10, 10,
       "g29.1 candidate decisions check evidence")),
    ("g29_interlock_check_evidence_schema.json",
     S("g29_interlock_check", "g29.gov.implementation_interlock", "G29.1", 8, 8,
       "g29.1 implementation interlock check evidence")),
] + [
    (f"g29_{m}_{slug}_evidence_schema.json",
     S(f"g29_{m}_{slug}", f"g29.p0.{m}.{slug}", wave, 6, None, f"g29_{m}_{slug} evidence",
       extra_states=(["skipped_dev_env"] if m in ("m_a", "m_b") else None)))
    for m, slug, wave in SLUGS
] + [
    ("g29_wave_exit_evidence_schema.json",
     S([f"g29_wave{n}_exit" for n in range(2, 7)],
       [f"g29.wave.{n}.exit" for n in range(2, 7)],
       [f"G29.{n}" for n in range(2, 7)],
       4, 4, "G29 wave exit evidence (parametrized waves 2..6)")),
    ("g29_p2_decisions_check_evidence_schema.json",
     S("g29_p2_decisions_check", "g29.wave.5a.decisions", "G29.5", 6, 6,
       "g29.5 P2 decisions check evidence")),
    ("g29_stabilization_soak_evidence_schema.json",
     S("g29_stabilization_soak", "g29.wave.5a.soak", "G29.5", 8, 8,
       "g29.5 stabilization soak evidence")),
    ("g29_wave6b_closeout_evidence_schema.json",
     S("g29_wave6b_closeout", "g29.wave.6b.closeout", "G29.6", 8, 8,
       "g29.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g29_acceptance_map_check_evidence_schema.json",
    "g29_candidate_decisions_check_evidence_schema.json",
    "g29_interlock_check_evidence_schema.json",
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
