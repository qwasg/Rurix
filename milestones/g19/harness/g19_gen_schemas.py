#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G19.1 治理波）
"""G19 evidence schema 生成器：12 份 schema 单源产出（治理 3 + P0 5 + wave_exit/p2/soak/closeout）。

单源模板与 g11_wave_exit_lib.emit_wave_evidence payload 形状逐字段对齐（G18 同构）。
"""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "milestones/g19"

COMMON_REQUIRED = [
    "schema_version", "subject", "symbolic_gate_key", "matrix_row", "wave",
    "numeric_step", "source_ref", "host_section_pass", "device_section_state",
    "required_gates", "extra_facts", "subjects", "checks",
    "evidence_level", "run_url", "timestamp", "environment", "notes",
]


def schema(subject, key, wave, facts_min, facts_max, title, extra_states=None):
    states = ["not_applicable"] + (extra_states or [])
    sub = {"type": "string", "const": subject} if isinstance(subject, str) else {"type": "string", "enum": subject}
    k = {"type": "string", "const": key} if isinstance(key, str) else {"type": "string", "enum": key}
    w = {"type": "string", "const": wave} if isinstance(wave, str) else {"type": "string", "enum": wave}
    facts = {"type": "array", "minItems": facts_min,
             "items": {"type": "object", "required": ["id", "status"],
                        "properties": {"id": {"type": "string"},
                                       "status": {"type": "string", "enum": ["PASS", "FAIL"]},
                                       "detail": {"type": "string"}}}}
    if facts_max is not None:
        facts["maxItems"] = facts_max
    return {
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": title,
        "type": "object",
        "additionalProperties": False,
        "required": COMMON_REQUIRED,
        "properties": {
            "schema_version": {"type": "integer", "const": 1},
            "subject": sub,
            "symbolic_gate_key": k,
            "matrix_row": w,
            "wave": w,
            "numeric_step": {"type": "integer"},
            "source_ref": {"type": "string"},
            "host_section_pass": {"type": "boolean"},
            "device_section_state": {"type": "string", "enum": states},
            "required_gates": {"type": "array", "maxItems": 0, "items": {"type": "object"}},
            "extra_facts": facts,
            "subjects": {"type": "array", "maxItems": 0, "items": {"type": "object"}},
            "checks": {"type": "object"},
            "evidence_level": {"type": "string", "const": "measured_local"},
            "run_url": {"type": "string"},
            "timestamp": {"type": "string"},
            "environment": {"type": "object"},
            "notes": {"type": "string"},
        },
    }


SPECS = [
    ("g19_acceptance_map_check_evidence_schema.json",
     schema("g19_acceptance_map_check", "g19.wave.1.acceptance_map", "G19.1", 12, 12,
            "g19.1 acceptance map check evidence")),
    ("g19_candidate_decisions_check_evidence_schema.json",
     schema("g19_candidate_decisions_check", "g19.wave.1.candidate_decisions", "G19.1", 10, 10,
            "g19.1 candidate decisions check evidence")),
    ("g19_interlock_check_evidence_schema.json",
     schema("g19_interlock_check", "g19.gov.implementation_interlock", "G19.1", 8, 8,
            "g19.1 implementation interlock check evidence")),
    ("g19_m_a_frame_generation_host_realization_evidence_schema.json",
     schema("g19_m_a_frame_generation_host_realization", "g19.p0.m_a.frame_generation_host_realization",
            "G19.2", 6, None, "g19_m_a_frame_generation_host_realization evidence")),
    ("g19_m_b_frame_generation_vendor_disposition_evidence_schema.json",
     schema("g19_m_b_frame_generation_vendor_disposition", "g19.p0.m_b.frame_generation_vendor_disposition",
            "G19.2", 6, None, "g19_m_b_frame_generation_vendor_disposition evidence")),
    ("g19_m_c_rd045_drift_observation_window_evidence_schema.json",
     schema("g19_m_c_rd045_drift_observation_window", "g19.p0.m_c.rd045_drift_observation_window",
            "G19.3", 6, None, "g19_m_c_rd045_drift_observation_window evidence",
            extra_states=["skipped_dev_env"])),
    ("g19_m_d_fps_parity_window_registration_evidence_schema.json",
     schema("g19_m_d_fps_parity_window_registration", "g19.p0.m_d.fps_parity_window_registration",
            "G19.4", 6, None, "g19_m_d_fps_parity_window_registration evidence")),
    ("g19_m_e_closed_gate_no_regression_evidence_schema.json",
     schema("g19_m_e_closed_gate_no_regression", "g19.p0.m_e.closed_gate_no_regression",
            "G19.4", 6, None, "g19_m_e_closed_gate_no_regression evidence")),
    ("g19_wave_exit_evidence_schema.json",
     schema([f"g19_wave{n}_exit" for n in range(2, 7)],
            [f"g19.wave.{n}.exit" for n in range(2, 7)],
            [f"G19.{n}" for n in range(2, 7)],
            4, 4, "G19 wave exit evidence (parametrized waves 2..6)")),
    ("g19_p2_decisions_check_evidence_schema.json",
     schema("g19_p2_decisions_check", "g19.wave.5a.decisions", "G19.5", 6, 6,
            "g19.5 P2 decisions check evidence")),
    ("g19_stabilization_soak_evidence_schema.json",
     schema("g19_stabilization_soak", "g19.wave.5a.soak", "G19.5", 8, 8,
            "g19.5 stabilization soak evidence")),
    ("g19_wave6b_closeout_evidence_schema.json",
     schema("g19_wave6b_closeout", "g19.wave.6b.closeout", "G19.6", 8, 8,
            "g19.6 close-out check evidence")),
]

GOVERNANCE_ONLY = {
    "g19_acceptance_map_check_evidence_schema.json",
    "g19_candidate_decisions_check_evidence_schema.json",
    "g19_interlock_check_evidence_schema.json",
}


def main(governance_only: bool = False) -> int:
    for name, doc in SPECS:
        if governance_only and name not in GOVERNANCE_ONLY:
            continue
        p = OUT / name
        p.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"[gen_schemas] → {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    import sys
    sys.exit(main(governance_only="--governance-only" in sys.argv))
