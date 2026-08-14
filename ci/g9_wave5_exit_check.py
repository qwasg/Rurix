#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.5 波次聚合门 g9.wave.5.exit(步骤 165;milestones/g9/CI_GATES §6 v1.12)。

只读汇总 G9.5 波十一门(两 P0 M110/M118 + 九 P1 M111~M120)最新 evidence
+ RFC-0025 Agent Approved 字面维持 + RXS-0363~0373 条款头在树
(spec/world_partition.md 0363~0368 + spec/display_pipeline.md 0369~0373)
+ M115 MaterialClosure 32B 布局 digest 冻结面核验(milestones/g9/
g9_m115_skin_band.json 冻结值与 spec RXS-0373 修订行断言及 harness 直出件
material_closure_32b 机核面逐字一致)+ M114 strand 档 not-triggered 登记字面
+ M120 仅测量不定档字面(tier_selection.committed==false / NotMeasuredYet)。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g9_wave5_exit_check.py --gate g9.wave.5.exit
  py -3 ci/g9_wave5_exit_check.py --selftest
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
GATE_KEY = "g9.wave.5.exit"
NUMERIC_STEP = 165
SUBJECT = "g9_wave5_exit"
WAVE = "G9.5"
SOURCE_REF = (
    "milestones/g9/CI_GATES §6 v1.12;G9_CONTRACT §8.5;G9_ACCEPTANCE_MAP §2/§3;"
    "RFC-0025 Agent Approved;RXS-0363~0373 clause heads on tree;"
    "M115 32B layout digest frozen;M114 strand tier not-triggered registered;"
    "M120 tier not committed (NotMeasuredYet)"
)
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_wave5_exit_evidence_schema.json"
RFC0025 = ROOT / "rfcs" / "0025-world-and-specialty-renderers.md"
SPEC_WP = ROOT / "spec" / "world_partition.md"
SPEC_DP = ROOT / "spec" / "display_pipeline.md"
M115_BAND = ROOT / "milestones" / "g9" / "g9_m115_skin_band.json"

# 十一个 G9.5 门:(symbolic_key, evidence subject_prefix)——两 P0(§4)+ 九 P1
# (§4A,G9_CONTRACT §8.1 裁决① P1 全进)。门 key 按 G9_ACCEPTANCE_MAP §2/§3 实记
# (M111=hlod_baking / M119=post_processing_skeleton,harness 直出件 assertion_id
# 命名差 hlod_runtime/post_chain 已在 v1.11 如实登记)。聚合门只核各门最新一份
# evidence 的 PASS(host_section_pass=true + checks 全真 + device 非 FAIL/SKIP/
# degrade),不重跑 smoke、不代绿。
REQUIRED_GATES: list[tuple[str, str]] = [
    ("g9.p0.m110.world_partition", "g9_m110_world_partition"),
    ("g9.p0.m118.display_pipeline_view_transform", "g9_m118_display_pipeline_view_transform"),
    ("g9.p1.m111.hlod_baking", "g9_m111_hlod_baking"),
    ("g9.p1.m112.atmosphere_froxel", "g9_m112_atmosphere_froxel"),
    ("g9.p1.m113.water_dual_pipeline", "g9_m113_water_dual_pipeline"),
    ("g9.p1.m114.hair_marschner", "g9_m114_hair_marschner"),
    ("g9.p1.m115.skin_burley_diffusion", "g9_m115_skin_burley_diffusion"),
    ("g9.p1.m116.terrain_chunk_cell", "g9_m116_terrain_chunk_cell"),
    ("g9.p1.m117.decal_dbuffer", "g9_m117_decal_dbuffer"),
    ("g9.p1.m119.post_processing_skeleton", "g9_m119_post_processing_skeleton"),
    ("g9.p1.m120.oit_benchmark_harness", "g9_m120_oit_benchmark_harness"),
]

_RXS_HEAD_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)
# harness 直出件文件名:<prefix>_<UTC>.json 或 <prefix>_<UTC>_von/_voff.json;
# 门 evidence(g9_m114_hair_marschner_<UTC>.json 等)因 prefix 后非数字被排除。
_HARNESS_NAME_RE = re.compile(r"_(\d{8}T\d{6}Z)(?:_(?:von|voff))?\.json$")
_HEX64_RE = re.compile(r"^[0-9a-f]{64}$")


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _rxs_heads(path: Path) -> set[int]:
    if not path.is_file():
        return set()
    return {int(m) for m in _RXS_HEAD_RE.findall(path.read_text(encoding="utf-8"))}


def _latest_harness_docs(
    subject_prefix: str, evidence_dir: Path | None = None
) -> list[dict] | None:
    """evidence/ 内取该 harness 前缀最新一戳直出件(同戳 von/voff 全组)。

    subject_prefix 例如 ``g9_m114_hair``(匹配 ``g9_m114_hair_<UTC>_von.json``,
    不匹配门 evidence ``g9_m114_hair_marschner_<UTC>.json``)。
    """
    base = evidence_dir if evidence_dir is not None else wel.EVIDENCE_DIR
    if not base.is_dir():
        return None
    candidates: list[tuple[str, Path]] = []
    for p in base.glob(f"{subject_prefix}_*.json"):
        m = _HARNESS_NAME_RE.search(p.name)
        if m is None:
            continue
        # prefix 后必须紧跟 UTC 戳(排除门 evidence 等同前缀更长名)。
        if not p.name[len(subject_prefix) + 1 :].startswith(m.group(1)):
            continue
        candidates.append((m.group(1), p))
    if not candidates:
        return None
    newest = max(stamp for stamp, _p in candidates)
    docs: list[dict] = []
    for stamp, p in candidates:
        if stamp != newest:
            continue
        try:
            docs.append(wel.load_json(p))
        except (OSError, ValueError):
            return None
    return docs


def _gate_checks_true(subject_prefix: str, check_key: str, evidence_dir: Path | None) -> bool:
    path = wel.load_latest_evidence(subject_prefix, evidence_dir=evidence_dir)
    if path is None:
        return False
    try:
        doc = wel.load_json(path)
    except (OSError, ValueError):
        return False
    checks = doc.get("checks")
    return isinstance(checks, dict) and checks.get(check_key) is True


def collect_extra_facts(evidence_dir: Path | None = None) -> list[dict]:
    facts: list[dict] = []

    ok, detail = wel.rfc_agent_approved(RFC0025)
    facts.append(_fact("rfc0025_approved", ok, detail))

    heads_wp = _rxs_heads(SPEC_WP)
    heads_dp = _rxs_heads(SPEC_DP)
    want_wp = {363, 364, 365, 366, 367, 368}
    want_dp = {369, 370, 371, 372, 373}
    missing_wp = sorted(want_wp - heads_wp)
    missing_dp = sorted(want_dp - heads_dp)
    missing = missing_wp + missing_dp
    facts.append(
        _fact(
            "rxs0363_0373_clause_heads_on_tree",
            not missing,
            (
                "spec/world_partition.md RXS-0363~0368 + spec/display_pipeline.md "
                "RXS-0369~0373 条款头全在树(共 11 枚)"
                if not missing
                else f"缺条款头: {missing}"
            ),
        )
    )

    # M115 MaterialClosure 32B 布局 digest 冻结面:冻结带在树且 spec_anchor=
    # RXS-0373、digest 64-hex;spec RXS-0373 条款头含「触 MaterialClosure 32B 经
    # RFC-0025 §4.L 修订行」断言字面;harness 最新直出件 material_closure_32b
    # 机核面(size_bytes=32/reserved_zero/flags_unassigned_zero/revision_line)
    # 与 golden/冻结带 digest 三者逐字一致。
    bad_32b: list[str] = []
    band_digest: str | None = None
    if not M115_BAND.is_file():
        bad_32b.append(f"缺 {M115_BAND.name}")
    else:
        try:
            band = wel.load_json(M115_BAND)
        except (OSError, ValueError):
            band = {}
            bad_32b.append(f"{M115_BAND.name} 不可读")
        if band.get("schema") != "rurix.g9m115.skin_band.v1":
            bad_32b.append("band.schema ≠ rurix.g9m115.skin_band.v1")
        if band.get("spec_anchor") != "RXS-0373":
            bad_32b.append("band.spec_anchor ≠ RXS-0373")
        digest = band.get("closure_32b_layout_digest")
        if not (isinstance(digest, str) and _HEX64_RE.match(digest)):
            bad_32b.append("closure_32b_layout_digest 非 64-hex")
        else:
            band_digest = digest
    spec_dp_text = SPEC_DP.read_text(encoding="utf-8") if SPEC_DP.is_file() else ""
    if "触 MaterialClosure 32B 经 RFC-0025 §4.L 修订行" not in spec_dp_text:
        bad_32b.append("spec/display_pipeline.md 缺 RXS-0373 32B 修订行断言字面")
    if band_digest is not None:
        docs = _latest_harness_docs("g9_m115_skin", evidence_dir=evidence_dir)
        if not docs:
            bad_32b.append("缺 g9_m115_skin harness 最新直出件")
        else:
            for doc in docs:
                mc = doc.get("material_closure_32b")
                if not isinstance(mc, dict):
                    bad_32b.append("harness 缺 material_closure_32b 机核面")
                    continue
                if mc.get("size_bytes") != 32:
                    bad_32b.append("material_closure_32b.size_bytes ≠ 32")
                if mc.get("layout_digest") != band_digest:
                    bad_32b.append("harness layout_digest ≠ 冻结带值")
                if mc.get("reserved_zero") is not True:
                    bad_32b.append("reserved_zero ≠ true")
                if mc.get("flags_unassigned_zero") is not True:
                    bad_32b.append("flags_unassigned_zero ≠ true")
                if mc.get("revision_line") != "RFC-0025 §4.L":
                    bad_32b.append("revision_line ≠ RFC-0025 §4.L")
                golden = doc.get("golden")
                if not isinstance(golden, dict) or golden.get("closure_32b_layout_digest") != band_digest:
                    bad_32b.append("harness golden.closure_32b_layout_digest ≠ 冻结带值")
    facts.append(
        _fact(
            "m115_closure_32b_layout_digest_frozen",
            not bad_32b,
            (
                "g9_m115_skin_band.json 冻结值 = harness 最新直出件 "
                "material_closure_32b.layout_digest/golden 逐字一致(size=32B、"
                "reserved/flags 未分配位全零、revision_line=RFC-0025 §4.L);"
                "spec RXS-0373 修订行断言字面在树"
                if not bad_32b
                else "; ".join(bad_32b)
            ),
        )
    )

    # M114 strand 档强制精确 OIT 分项 not-triggered 登记字面:spec RXS-0372 条款头
    # 字面 + 门最新 evidence not_triggered_field_verified + harness 最新直出件
    # strand_tier_not_triggered_registered / m120_measurements_availability_recorded。
    bad_114: list[str] = []
    if "strand 档强制精确 OIT 分项 not-triggered 登记" not in spec_dp_text:
        bad_114.append("spec/display_pipeline.md 缺 RXS-0372 strand 档 not-triggered 登记字面")
    if not _gate_checks_true("g9_m114_hair_marschner", "not_triggered_field_verified", evidence_dir):
        bad_114.append("M114 门最新 evidence checks.not_triggered_field_verified ≠ true")
    docs_114 = _latest_harness_docs("g9_m114_hair", evidence_dir=evidence_dir)
    if not docs_114:
        bad_114.append("缺 g9_m114_hair harness 最新直出件")
    else:
        for doc in docs_114:
            checks = doc.get("checks")
            if not isinstance(checks, dict):
                bad_114.append("harness 缺 checks")
                continue
            if checks.get("strand_tier_not_triggered_registered") is not True:
                bad_114.append("harness checks.strand_tier_not_triggered_registered ≠ true")
            if checks.get("m120_measurements_availability_recorded") is not True:
                bad_114.append("harness checks.m120_measurements_availability_recorded ≠ true")
    facts.append(
        _fact(
            "m114_strand_tier_not_triggered_registered",
            not bad_114,
            (
                "spec RXS-0372 条款头 + M114 门/harness 最新 evidence 三处登记面一致:"
                "strand 档强制精确 OIT 分项 not-triggered 不充绿(消费 M120 测量带,"
                "承接锚重判兜底 G9.7)"
                if not bad_114
                else "; ".join(bad_114)
            ),
        )
    )

    # M120 仅测量不定档字面:spec RXS-0371 条款头字面 + 门最新 evidence
    # not_triggered_field_verified + harness 最新直出件 tier_selection.committed==
    # false 且 policy 含 NotMeasuredYet 字面。
    bad_120: list[str] = []
    if "仅测量不定档" not in spec_dp_text:
        bad_120.append("spec/display_pipeline.md 缺 RXS-0371 仅测量不定档字面")
    if not _gate_checks_true("g9_m120_oit_benchmark_harness", "not_triggered_field_verified", evidence_dir):
        bad_120.append("M120 门最新 evidence checks.not_triggered_field_verified ≠ true")
    docs_120 = _latest_harness_docs("g9_m120_oit_benchmark", evidence_dir=evidence_dir)
    if not docs_120:
        bad_120.append("缺 g9_m120_oit_benchmark harness 最新直出件")
    else:
        for doc in docs_120:
            tier = doc.get("tier_selection")
            if not isinstance(tier, dict):
                bad_120.append("harness 缺 tier_selection 面")
                continue
            if tier.get("committed") is not False:
                bad_120.append("tier_selection.committed ≠ false")
            if "NotMeasuredYet" not in str(tier.get("policy", "")):
                bad_120.append("tier_selection.policy 缺 NotMeasuredYet 字面")
    facts.append(
        _fact(
            "m120_tier_not_committed_not_measured_yet",
            not bad_120,
            (
                "spec RXS-0371 条款头 + M120 门/harness 最新 evidence 三处登记面一致:"
                "仅测量不定档(tier_selection.committed==false,select_default_tier "
                "fail-closed NotMeasuredYet)"
                if not bad_120
                else "; ".join(bad_120)
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
        "implemented: eleven G9.5 gates (P0 M110/M118 + P1 M111~M120)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: RFC-0025 Approved literal + RXS-0363~0373 clause heads + "
        "M115 32B layout digest frozen face + M114 strand not-triggered + "
        "M120 tier not committed (NotMeasuredYet)",
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
    """① 缺十一门 evidence → 红;② 真树十一门绿 + 事实核验 → 绿。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile
    from pathlib import Path as P

    with tempfile.TemporaryDirectory(prefix="g9_wave5_selftest_") as td:
        code = run_gate(evidence_dir=P(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 正样本:仓库最新十一门 evidence")
    code = run_gate(evidence_dir=None)
    if code != 0:
        print("[selftest] FAIL: 真树聚合未绿(前置十一门/事实核验未满足)", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真树聚合绿")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G9.5 wave5.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
