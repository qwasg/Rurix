#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.4 波次聚合门 g9.wave.4.exit(步骤 153;milestones/g9/CI_GATES §6 v1.9)。

只读汇总 G9.4 波六门(三 P0 M96/M97/M98 + 三 P1 M99/M100/M101)最新
evidence + RFC-0022 Agent Approved 字面维持 + RXS-0357~0362 条款头在树
(spec/global_illumination.md)+ 门序机器阻断在树且 M97~M101 五门最新
evidence 均含 checks.gate_order_m96_passed=true(D2-Q7;ci/g9_gi_interlock.py)
+ 六冻结带在树。不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS
不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g9_wave4_exit_check.py --gate g9.wave.4.exit
  py -3 ci/g9_wave4_exit_check.py --selftest
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
GATE_KEY = "g9.wave.4.exit"
NUMERIC_STEP = 153
SUBJECT = "g9_wave4_exit"
WAVE = "G9.4"
SOURCE_REF = (
    "milestones/g9/CI_GATES §6 v1.9;G9_CONTRACT §8.4;G9_ACCEPTANCE_MAP §2/§3;"
    "RFC-0022 Agent Approved;RXS-0357~0362 clause heads on tree;"
    "gate-order interlock enforced (D2-Q7)"
)
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_wave4_exit_evidence_schema.json"
RFC0022 = ROOT / "rfcs" / "0022-virtual-geometry-gi-semantics.md"
SPEC_GI = ROOT / "spec" / "global_illumination.md"
INTERLOCK = ROOT / "ci" / "g9_gi_interlock.py"
BAND_FILES = [
    "g9_m96_pbrt_tolerance_band.json",
    "g9_m97_depth_band.json",
    "g9_m98_depth_band.json",
    "g9_m99_spg_rc_band.json",
    "g9_m100_multi_light_band.json",
    "g9_m101_if_tier_band.json",
]

# 六个 G9.4 门:(symbolic_key, evidence subject_prefix)——三 P0(§4)+ 三 P1
# (§4A,G9_CONTRACT §8.1 裁决① P1 全进)。聚合门只核各门最新一份 evidence 的
# PASS(host_section_pass=true + checks 全真 + device 非 FAIL/SKIP/degrade),
# 不重跑 smoke、不代绿。
REQUIRED_GATES: list[tuple[str, str]] = [
    ("g9.p0.m96.path_tracer_reference", "g9_m96_path_tracer_reference"),
    ("g9.p0.m97.surface_cache", "g9_m97_surface_cache"),
    ("g9.p0.m98.tracing_fallback_chain", "g9_m98_tracing_fallback_chain"),
    ("g9.p1.m99.spg_radiance_cache", "g9_m99_spg_radiance_cache"),
    ("g9.p1.m100.multi_light_low", "g9_m100_multi_light_low"),
    ("g9.p1.m101.if_tier_ladder", "g9_m101_if_tier_ladder"),
]

# 门序机器阻断(D2-Q7)核验面:M97~M101 五门最新 evidence 须含
# checks.gate_order_m96_passed=true(M96 为门序源头,自身无此前置)。
INTERLOCKED_PREFIXES = [prefix for _key, prefix in REQUIRED_GATES[1:]]

_RXS_HEAD_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _rxs_heads(path: Path) -> set[int]:
    if not path.is_file():
        return set()
    return {int(m) for m in _RXS_HEAD_RE.findall(path.read_text(encoding="utf-8"))}


def collect_extra_facts(evidence_dir: Path | None = None) -> list[dict]:
    facts: list[dict] = []

    ok, detail = wel.rfc_agent_approved(RFC0022)
    facts.append(_fact("rfc0022_approved", ok, detail))

    heads = _rxs_heads(SPEC_GI)
    want = {357, 358, 359, 360, 361, 362}
    missing = sorted(want - heads)
    facts.append(
        _fact(
            "rxs0357_0362_clause_heads_on_tree",
            not missing,
            (
                "spec/global_illumination.md RXS-0357~0362 条款头全在树"
                if not missing
                else f"缺条款头: {missing}"
            ),
        )
    )

    if not INTERLOCK.is_file():
        facts.append(_fact("gate_order_interlock_enforced", False, f"缺 {INTERLOCK.name}"))
    else:
        bad: list[str] = []
        for prefix in INTERLOCKED_PREFIXES:
            path = wel.load_latest_evidence(prefix, evidence_dir=evidence_dir)
            if path is None:
                bad.append(f"{prefix}: 缺 evidence")
                continue
            try:
                doc = wel.load_json(path)
            except (OSError, ValueError):
                bad.append(f"{prefix}: 不可读")
                continue
            checks = doc.get("checks")
            if not isinstance(checks, dict) or checks.get("gate_order_m96_passed") is not True:
                bad.append(f"{prefix}: gate_order_m96_passed≠true")
        facts.append(
            _fact(
                "gate_order_interlock_enforced",
                not bad,
                (
                    "ci/g9_gi_interlock.py 在树;M97~M101 五门最新 evidence 均含 "
                    "checks.gate_order_m96_passed=true(D2-Q7 门序前置机器核验留痕)"
                    if not bad
                    else "; ".join(bad)
                ),
            )
        )

    missing_bands = [b for b in BAND_FILES if not (ROOT / "milestones" / "g9" / b).is_file()]
    facts.append(
        _fact(
            "frozen_bands_on_tree",
            not missing_bands,
            (
                "六冻结带(m96 pbrt 容差/m97 depth/m98 depth/m99 spg_rc/m100 multi_light/"
                "m101 if_tier)全在树"
                if not missing_bands
                else f"缺冻结带: {missing_bands}"
            ),
        )
    )
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [
        wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir)
        for key, prefix in REQUIRED_GATES
    ]
    extras = collect_extra_facts(evidence_dir=evidence_dir)
    notes_parts = [
        "implemented: six G9.4 gates (P0 M96/M97/M98 + P1 M99/M100/M101)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: RFC-0022 Approved literal + RXS-0357~0362 clause heads + "
        "gate-order interlock (D2-Q7) + six frozen bands on tree",
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
    """① 缺一门 evidence → 红;② 真树六门绿 + 事实核验 → 绿。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile
    from pathlib import Path as P

    with tempfile.TemporaryDirectory(prefix="g9_wave4_selftest_") as td:
        code = run_gate(evidence_dir=P(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 正样本:仓库最新六门 evidence")
    code = run_gate(evidence_dir=None)
    if code != 0:
        print("[selftest] FAIL: 真树聚合未绿(前置六门/事实核验未满足)", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真树聚合绿")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G9.4 wave4.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
