#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.3 波次聚合门 g9.wave.3.exit(步骤 146;milestones/g9/CI_GATES §6 v1.6)。

只读汇总 G9.3 波七门(三 P0 M93/M94/M95 + 四 P1 M92/M105/M106/M107)最新
evidence + RFC-0022/0023 Agent Approved 字面维持 + RXS-0350~0356 条款头
在树 + U56/U57 unsafe 登记在树。不重跑 smoke、不代绿、不设
RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g9_wave3_exit_check.py --gate g9.wave.3.exit
  py -3 ci/g9_wave3_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# 允许同目录 import
sys.path.insert(0, str(Path(__file__).resolve().parent))

import g9_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g9.wave.3.exit"
NUMERIC_STEP = 146
SUBJECT = "g9_wave3_exit"
WAVE = "G9.3"
SOURCE_REF = (
    "milestones/g9/CI_GATES §6 v1.6;G9_CONTRACT §8.3;G9_ACCEPTANCE_MAP §2/§3;"
    "RFC-0022/0023 Agent Approved;RXS-0350~0356 clause heads on tree;"
    "U56/U57 unsafe-audit registered"
)
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_wave3_exit_evidence_schema.json"
RFC0022 = ROOT / "rfcs" / "0022-virtual-geometry-gi-semantics.md"
RFC0023 = ROOT / "rfcs" / "0023-gpu-driven-submission-shading.md"
SPEC_VG = ROOT / "spec" / "virtual_geometry.md"
SPEC_GDS = ROOT / "spec" / "gpu_driven_submit.md"
UNSAFE_AUDIT = ROOT / "unsafe-audit" / "rurix-rt.md"
NUMBER_LEDGER = ROOT / "registry" / "number_ledger.json"

# 七个 G9.3 门:(symbolic_key, evidence subject_prefix)——三 P0(§4)+ 四 P1
# (§4A,G9_CONTRACT §8.1 裁决① P1 全进)。聚合门只核各门最新一份 evidence 的
# PASS(host_section_pass=true + checks 全真 + device 非 FAIL/SKIP/degrade),
# 不重跑 smoke、不代绿。
REQUIRED_GATES: list[tuple[str, str]] = [
    ("g9.p0.m93.visible_cluster_set", "g9_m93_visible_cluster_set"),
    ("g9.p0.m94.clas_rt_convergence", "g9_m94_clas_rt_convergence"),
    ("g9.p0.m95.single_source_truth", "g9_m95_single_source_truth"),
    ("g9.p1.m92.gpu_skinning_lod_update", "g9_m92_gpu_skinning_lod_update"),
    ("g9.p1.m105.command_build_node", "g9_m105_command_build_node"),
    ("g9.p1.m106.execution_set_pso", "g9_m106_execution_set_pso"),
    ("g9.p1.m107.shader_library_ir_link", "g9_m107_shader_library_ir_link"),
]

_RXS_HEAD_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _rxs_heads(path: Path) -> set[int]:
    if not path.is_file():
        return set()
    return {int(m) for m in _RXS_HEAD_RE.findall(path.read_text(encoding="utf-8"))}


def collect_extra_facts() -> list[dict]:
    facts: list[dict] = []

    for rfc, fid in ((RFC0022, "rfc0022_approved"), (RFC0023, "rfc0023_approved")):
        ok, detail = wel.rfc_agent_approved(rfc)
        facts.append(_fact(fid, ok, detail))

    vg = _rxs_heads(SPEC_VG)
    gds = _rxs_heads(SPEC_GDS)
    want_vg = {350, 351, 352, 353}
    want_gds = {354, 355, 356}
    missing = sorted((want_vg - vg) | (want_gds - gds))
    facts.append(
        _fact(
            "rxs0350_0356_clause_heads_on_tree",
            not missing,
            (
                "spec/virtual_geometry.md RXS-0350~0353 + spec/gpu_driven_submit.md "
                "RXS-0354~0356 条款头全在树"
                if not missing
                else f"缺条款头: {missing}"
            ),
        )
    )

    if not UNSAFE_AUDIT.is_file():
        facts.append(_fact("u56_u57_registered", False, f"unsafe-audit 缺失: {UNSAFE_AUDIT}"))
    else:
        text = UNSAFE_AUDIT.read_text(encoding="utf-8")
        has_rows = "| U56 |" in text and "| U57 |" in text
        try:
            ledger = wel.load_json(NUMBER_LEDGER)
            u_ns = ledger.get("namespaces", {}).get("U", {})
            u_otm = u_ns.get("on_tree_max")
            u_nf = u_ns.get("next_free")
        except (OSError, ValueError):
            u_otm = u_nf = None
        ledger_ok = isinstance(u_otm, int) and u_otm >= 57 and isinstance(u_nf, int) and u_nf >= 58
        ok = has_rows and ledger_ok
        facts.append(
            _fact(
                "u56_u57_registered",
                ok,
                f"unsafe-audit/rurix-rt.md U56/U57 行={'在树' if has_rows else '缺失'};"
                f"ledger U on_tree_max={u_otm} next_free={u_nf}(要求 ≥57/≥58)",
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
        "implemented: seven G9.3 gates (P0 M93/M94/M95 + P1 M92/M105/M106/M107)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: RFC-0022/0023 Approved literal + RXS-0350~0356 clause heads + U56/U57 registered",
        "aggregate PASS does not mask any child FAIL/SKIP/DEV_ENV_DEGRADE",
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
    """① 缺一门 evidence → 红;② 真树七门绿 + 事实核验 → 绿。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile
    from pathlib import Path as P

    with tempfile.TemporaryDirectory(prefix="g9_wave3_selftest_") as td:
        code = run_gate(evidence_dir=P(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 正样本:仓库最新七门 evidence")
    code = run_gate(evidence_dir=None)
    if code != 0:
        print("[selftest] FAIL: 真树聚合未绿(前置七门/事实核验未满足)", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真树聚合绿")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G9.3 wave3.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
