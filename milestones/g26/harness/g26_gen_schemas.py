#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G26.1 治理波）
"""G26 evidence schema 生成器：12 份 schema 单源产出。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "milestones/g19/harness"))
import g19_gen_schemas as base  # noqa: E402

OUT = ROOT / "milestones/g26"
S = base.schema

SLUGS = [
    ("m_a", "framegen_device_kernel", "G26.2"),
    ("m_b", "framegen_device_bench_accounting", "G26.2"),
    ("m_c", "rd045_backfill_rejudgment", "G26.3"),
    ("m_d", "g17_md_f1_rejudgment_window", "G26.3"),
    ("m_e", "closed_gate_no_regression", "G26.4"),
]

SPECS = [
    ("g26_acceptance_map_check_evidence_schema.json",
     S("g26_acceptance_map_check", "g26.wave.1.acceptance_map", "G26.1", 12, 12,
       "g26.1 acceptance map check evidence")),
    ("g26_candidate_decisions_check_evidence_schema.json",
     S("g26_candidate_decisions_check", "g26.wave.1.candidate_decisions", "G26.1", 10, 10,
       "g26.1 candidate decisions check evidence")),
    ("g26_interlock_check_evidence_schema.json",
     S("g26_interlock_check", "g26.gov.implementation_interlock", "G26.1", 8, 8,
       "g26.1 implementation interlock check evidence")),
] + [
    (f"g26_{m}_{slug}_evidence_schema.json",
     S(f"g26_{m}_{slug}", f"g26.p0.{m}.{slug}", wave, 6, None, f"g26_{m}_{slug} evidence",
       extra_states=(["skipped_dev_env"] if m in ("m_a", "m_b") else None)))
    for m, slug, wave in SLUGS
] + [
    ("g26_wave_exit_evidence_schema.json",
     S([f"g26_wave{n}_exit" for n in range(2, 7)],
       [f"g26.wave.{n}.exit" for n in range(2, 7)],
       [f"G26.{n}" for n in range(2, 7)],
       4, 4, "G26 wave exit evidence (parametrized waves 2..6)")),
    ("g26_p2_decisions_check_evidence_schema.json",
     S("g26_p2_decisions_check", "g26.wave.5a.decisions", "G26.5", 6, 6,
       "g26.5 P2 decisions check evidence")),
    ("g26_stabilization_soak_evidence_schema.json",
     S("g26_stabilization_soak", "g26.wave.5a.soak", "G26.5", 8, 8,
       "g26.5 stabilization soak evidence")),
    ("g26_wave6b_closeout_evidence_schema.json",
     S("g26_wave6b_closeout", "g26.wave.6b.closeout", "G26.6", 8, 8,
       "g26.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g26_acceptance_map_check_evidence_schema.json",
    "g26_candidate_decisions_check_evidence_schema.json",
    "g26_interlock_check_evidence_schema.json",
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
