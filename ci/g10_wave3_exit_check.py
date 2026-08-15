#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.3 波）
"""G10.3 波次聚合门 g10.wave.3.exit（步骤 176；milestones/g10/CI_GATES.md §5；
G10_CONTRACT G-G10-5；同构 ci/g9_wave6_exit_check.py + ci/g10_wave_exit_lib.py）。

只读汇总 G10.3 波三门最新 evidence——M131 许可登记（步骤 173）/ M132 语料
加载（步骤 174）/ M133 清单冻结（步骤 175）——+ spec/external_reference.md
在树且 RXS-0380~0383 条款头齐 + RFC-0027 Agent Approved 字面在树 + 许可注册
表零缺行（结构闭集复算）+ 清单 digest 注册在树字面（最新修订行 digest ==
全量行复算）。不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮
蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE。

用法：
  py -3 ci/g10_wave3_exit_check.py --gate g10.wave.3.exit
  py -3 ci/g10_wave3_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import g10_corpus_lib as lib  # noqa: E402
import g10_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g10.wave.3.exit"
NUMERIC_STEP = 176
SUBJECT = "g10_wave3_exit"
WAVE = "G10.3"
SOURCE_REF = (
    "milestones/g10/CI_GATES.md §5;G10_CONTRACT G-G10-5;G10_ACCEPTANCE_MAP §1/§2;"
    "RFC-0027 §4.2/§4.3/§4.4;RXS-0380~0383 clause heads on tree;"
    "license registry zero missing rows;manifest digest registered in tree"
)
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_wave3_exit_evidence_schema.json"
RFC0027 = ROOT / "rfcs" / "0027-external-reference-harness-license.md"
SPEC_XR = ROOT / "spec" / "external_reference.md"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g10.p0.m131.asset_license_registry", "g10_m131_asset_license_registry"),
    ("g10.p0.m132.corpus_loading", "g10_m132_corpus_loading"),
    ("g10.p1.m133.corpus_list_freeze", "g10_m133_corpus_list_freeze"),
]

_RXS_HEAD_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_extra_facts() -> list[dict]:
    facts: list[dict] = []

    # ① spec/external_reference.md 在树且 RXS-0380~0383 条款头齐。
    heads: set[int] = set()
    if SPEC_XR.is_file():
        heads = {int(m) for m in _RXS_HEAD_RE.findall(SPEC_XR.read_text(encoding="utf-8"))}
    missing = sorted({380, 381, 382, 383} - heads)
    facts.append(
        _fact(
            "rxs0380_0383_clause_heads_on_tree",
            SPEC_XR.is_file() and not missing,
            (
                "spec/external_reference.md 在树，RXS-0380~0383 条款头全在树（共 4 枚）"
                if SPEC_XR.is_file() and not missing
                else f"spec 缺失或缺条款头: {missing}"
            ),
        )
    )

    # ② RFC-0027 Agent Approved 字面在树。
    ok_rfc, detail_rfc = wel.rfc_agent_approved(RFC0027)
    facts.append(_fact("rfc0027_agent_approved", ok_rfc, f"RFC-0027: {detail_rfc}"))

    # ③ 许可注册表零缺行（结构闭集复算；不触缓存——缓存核验在 M131 门内）。
    reg_fails: list[str] = []
    if not lib.REGISTRY_PATH.is_file():
        reg_fails = ["许可注册表缺失"]
    else:
        try:
            reg_fails = lib.validate_registry(lib.load_json(lib.REGISTRY_PATH))
        except (OSError, ValueError) as e:
            reg_fails = [f"注册表不可读: {e}"]
    facts.append(
        _fact(
            "license_registry_zero_missing_rows",
            not reg_fails,
            "g10_asset_license_registry.json 按类登记闭集零缺行（白名单/五元组/六字段/attribution 复算全过）"
            if not reg_fails
            else "; ".join(reg_fails[:3]),
        )
    )

    # ④ 清单 digest 注册在树字面：最新修订行 manifest_digest == 全量行复算。
    bad_4: list[str] = []
    if not lib.MANIFEST_PATH.is_file():
        bad_4.append("场景清单缺失")
    else:
        try:
            man = lib.load_json(lib.MANIFEST_PATH)
        except (OSError, ValueError):
            man = {}
            bad_4.append("场景清单不可读")
        revisions = man.get("revisions") or []
        if not revisions:
            bad_4.append("revisions 空（未注册 digest）")
        else:
            want = lib.manifest_scenes_digest(man.get("scenes") or [])
            if revisions[-1].get("manifest_digest") != want:
                bad_4.append("最新修订 digest ≠ 全量行复算")
    facts.append(
        _fact(
            "manifest_digest_registered_in_tree",
            not bad_4,
            (
                "g10_corpus_scene_manifest.json 最新修订行 manifest_digest == 全量行 canonical 复算（清单 digest 注册在树）"
                if not bad_4
                else "; ".join(bad_4)
            ),
        )
    )
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_extra_facts()
    notes_parts = [
        "implemented: three G10.3 gates (P0 M131 asset_license_registry step 173 / "
        "P0 M132 corpus_loading step 174 / P1 M133 corpus_list_freeze step 175)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: RXS-0380~0383 clause heads + RFC-0027 Agent Approved literal + "
        "license registry zero missing rows + manifest digest registered in tree",
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
        host_section_pass=True,
    )
    return code


def run_selftest() -> int:
    """① 缺三门 evidence → 红；② 真树三门绿 + 事实核验 → 绿。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g10_wave3_selftest_") as td:
        code = run_gate(evidence_dir=Path(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 正样本:仓库最新三门 evidence")
    code = run_gate(evidence_dir=None)
    if code != 0:
        print("[selftest] FAIL: 真树聚合未绿（前置三门/事实核验未满足）", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真树聚合绿")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G10.3 wave3.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
