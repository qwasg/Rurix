#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.2 波次聚合门 g9.wave.2.exit(步骤 138;milestones/g9/CI_GATES §5)。

只读汇总七个 G9.2 P0 最新 evidence + RFC-0022/0023/0024 Agent Approved
+ G-G9-3 interlock READY + RD-039/040 总体维持 open(分项已承接)。不重跑
smoke、不代绿、不设 RURIX_REQUIRE_REAL。M121/M122 骨架期 phase_g9_6_pass
恒 false 不充绿(聚合门只核 --phase g9.2 evidence 的 PASS)。

用法:
  py -3 ci/g9_wave2_exit_check.py --gate g9.wave.2.exit
  py -3 ci/g9_wave2_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

# 允许同目录 import
sys.path.insert(0, str(Path(__file__).resolve().parent))

import g9_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g9.wave.2.exit"
NUMERIC_STEP = 138
SUBJECT = "g9_wave2_exit"
WAVE = "G9.2"
SOURCE_REF = (
    "milestones/g9/CI_GATES §5;G9_CONTRACT G-G9-4;G9_CANDIDATE_DECISIONS v1.1;"
    "RFC-0022/0023/0024 Agent Approved;RD-039/040 overall open retained"
)
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_wave2_exit_evidence_schema.json"
RFC0022 = ROOT / "rfcs" / "0022-virtual-geometry-gi-semantics.md"
RFC0023 = ROOT / "rfcs" / "0023-gpu-driven-submission-shading.md"
RFC0024 = ROOT / "rfcs" / "0024-physics-platform-revision.md"

# 七个 G9.2 P0:(symbolic_key, evidence subject_prefix)——M121/M122 骨架期
# --phase g9.2 evidence subject 与完整期同 subject,聚合门只核最新一份的
# PASS 与 phase_g9_2_pass=true;phase_g9_6_pass 恒 false 不充绿(双 phase 纪律)。
REQUIRED_GATES: list[tuple[str, str]] = [
    ("g9.p0.m90.cluster_dag_deepening", "g9_m90_cluster_dag_deepening"),
    ("g9.p0.m91.page_format_v2_abi", "g9_m91_page_format_v2_abi"),
    ("g9.p0.m102.dgc_abstraction", "g9_m102_dgc_abstraction"),
    ("g9.p0.m103.descriptor_global_table", "g9_m103_descriptor_global_table"),
    ("g9.p0.m104.accesskind_indirect_edge", "g9_m104_accesskind_indirect_edge"),
    ("g9.p0.m121.physics_particle_view", "g9_m121_physics_particle_view"),
    ("g9.p0.m122.gameplay_field", "g9_m122_gameplay_field"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_extra_facts() -> list[dict]:
    facts: list[dict] = []

    for rfc, fid in ((RFC0022, "rfc0022_approved"), (RFC0023, "rfc0023_approved"), (RFC0024, "rfc0024_approved")):
        ok, detail = wel.rfc_agent_approved(rfc)
        facts.append(_fact(fid, ok, detail))

    rd039 = wel.load_rd_status("RD-039")
    facts.append(
        _fact(
            "rd039_retained_open",
            rd039 == "open",
            f"RD-039.status={rd039!r} (wave2 要求维持 open;M06/M09/M61 分项已承接不代关)",
        )
    )
    rd040 = wel.load_rd_status("RD-040")
    facts.append(
        _fact(
            "rd040_retained_open",
            rd040 == "open",
            f"RD-040.status={rd040!r} (wave2 要求维持 open;M52 分项已承接不代关)",
        )
    )
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [
        wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir)
        for key, prefix in REQUIRED_GATES
    ]
    extras = collect_extra_facts()
    notes_parts = [
        "implemented: seven G9.2 P0 keys (M90/M91/M102/M103/M104/M121/M122 skeleton)",
        "dual-phase discipline: M121/M122 skeleton --phase g9.2 PASS; phase_g9_6_pass=false 不充绿",
        "retained-open: RD-039/RD-040 overall open (分项承接不代关)",
        "aggregate read-only: no smoke re-run, no substitute green",
    ]
    code, _path = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=rows,
        extra_facts=extras,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="; ".join(notes_parts),
        host_section_pass=True,  # 由 emit 内 overall 覆盖
    )
    return code


def run_selftest() -> int:
    """① 缺一门 evidence → 红;② 真树七门绿 + RFC/RD → 绿。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile
    from pathlib import Path as P

    with tempfile.TemporaryDirectory(prefix="g9_wave2_selftest_") as td:
        code = run_gate(evidence_dir=P(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 正样本:仓库最新七门 evidence")
    code = run_gate(evidence_dir=None)
    if code != 0:
        print("[selftest] FAIL: 真树聚合未绿(前置七门/RFC/RD 未满足)", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真树聚合绿")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G9.2 wave2.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
