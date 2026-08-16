#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.2 波）
"""G11.2 波次聚合门 g11.wave.2.exit（步骤 200；milestones/g11/CI_GATES.md §5；
G11_CONTRACT G-G11-4；同构 ci/g10_wave4_exit_check.py + ci/g11_wave_exit_lib.py）。

只读汇总 G11.2 波四门最新 evidence——M144 C1 亮度口径对齐（步骤 196）/
M145 C2 曝光链对齐（步骤 197）/ M146 C3 位深对齐（步骤 198）/ M157 HDR-FLIP
独立标定（步骤 199）——+ spec 条款头在树（visual_comparison.md RXS-0392/0393，
共 2 枚）+ RFC-0028 Agent Approved 字面在树 + 残余口径差登记在树且完备
（g11_2_residual_caliber_registry 校验器零 problems——R3/R4 承接锚非空）+
标定值入 g11_budget 且 provenance 齐备（四条 g11.caliber.*/g11.metric.* 条目
measured_local + evidence_file 在树可解 results.trimmed_mean + threshold ==
trimmed_mean × k 重算口径，P-09）+ 四门 RED 臂独立有效（最新 evidence 各含
red_* checks 且全真）。不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合
PASS 不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE。

用法：
  py -3 ci/g11_wave2_exit_check.py --gate g11.wave.2.exit
  py -3 ci/g11_wave2_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import g11_2_caliber_lib as cl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g11.wave.2.exit"
NUMERIC_STEP = 200
SUBJECT = "g11_wave2_exit"
WAVE = "G11.2"
SOURCE_REF = (
    "milestones/g11/CI_GATES.md §5;G11_CONTRACT G-G11-4;G11_ACCEPTANCE_MAP §1/§2;"
    "RFC-0028 §4.5/§4.6;RXS-0392/0393 clause heads on tree;"
    "residual caliber registry complete;calibrated thresholds in g11_budget with provenance (P-09);"
    "four gates red arms independently effective"
)
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_wave2_exit_evidence_schema.json"
RFC0028 = ROOT / "rfcs" / "0028-g11-gi-quality-closure.md"
SPEC_VC = ROOT / "spec" / "visual_comparison.md"
BUDGET_PATH = ROOT / "milestones" / "g11" / "g11_budget.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g11.p0.m144.caliber_c1_indoor_luminance", "g11_m144_caliber_c1_indoor_luminance"),
    ("g11.p0.m145.caliber_c2_exposure_chain", "g11_m145_caliber_c2_exposure_chain"),
    ("g11.p0.m146.caliber_c3_exr_bit_depth", "g11_m146_caliber_c3_exr_bit_depth"),
    ("g11.p1.m157.hdr_flip_calibration", "g11_m157_hdr_flip_calibration"),
]

# 四门 RED 臂独立有效面（四门最新 evidence 的 red_* checks 闭集）。
RED_ARM_GATES: list[tuple[str, str]] = REQUIRED_GATES

# 标定值入 budget 的四条条目闭集（id → k）。
CALIB_BUDGET_ENTRIES: list[tuple[str, float]] = [
    ("g11.caliber.c2_exposure_scale_tol", 1.0),
    ("g11.caliber.c3_bitdepth_domain_tol", 1.0),
    ("g11.metric.hdr_flip_pairwise_scalar_tol", 2.0),
    ("g11.metric.hdr_flip_pairwise_error_map_tol", 2.0),
]

_RXS_HEAD_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_extra_facts() -> list[dict]:
    facts: list[dict] = []

    # ① spec 条款头在树（visual_comparison.md RXS-0392/0393，共 2 枚）。
    heads: set[int] = set()
    if SPEC_VC.is_file():
        heads = {int(m) for m in _RXS_HEAD_RE.findall(SPEC_VC.read_text(encoding="utf-8"))}
    missing = sorted({392, 393} - heads)
    facts.append(
        _fact(
            "rxs0392_0393_clause_heads_on_tree",
            SPEC_VC.is_file() and not missing,
            (
                "spec/visual_comparison.md RXS-0392/0393 条款头全在树（共 2 枚）"
                if SPEC_VC.is_file() and not missing
                else f"spec 缺失或缺条款头: {missing}"
            ),
        )
    )

    # ② RFC-0028 Agent Approved 字面在树。
    ok_rfc, detail_rfc = wel.rfc_agent_approved(RFC0028)
    facts.append(_fact("rfc0028_agent_approved", ok_rfc, f"RFC-0028: {detail_rfc}"))

    # ③ 残余口径差登记在树且完备（R3/R4 承接锚非空——RXS-0392 L4 机核面）。
    reg_problems: list[str] = []
    try:
        registry = cl.load_residual_registry()
        reg_problems = cl.validate_residual_registry(registry)
    except (OSError, ValueError) as e:
        reg_problems = [f"残余登记不可读: {e}"]
    facts.append(
        _fact(
            "residual_caliber_registry_complete",
            not reg_problems,
            (
                "g11_2_residual_caliber_registry 逐环节非空 + R3（m153）/R4（m154）承接锚非空（RXS-0392 L4）"
                if not reg_problems
                else "; ".join(reg_problems[:3])
            ),
        )
    )

    # ④ 标定值入 g11_budget 且 provenance 齐备（P-09）：四条 g11.caliber.*/g11.metric.*
    # 条目 measured_local + evidence_file 在树可解 results.trimmed_mean +
    # threshold == trimmed_mean × k（重算口径）。
    budget_bad: list[str] = []
    try:
        budget = wel.load_json(BUDGET_PATH)
    except (OSError, ValueError) as e:
        budget = {"entries": []}
        budget_bad.append(f"budget 不可读: {e}")
    entries = {e.get("id"): e for e in budget.get("entries", [])}
    for eid, k in CALIB_BUDGET_ENTRIES:
        entry = entries.get(eid)
        if entry is None:
            budget_bad.append(f"{eid} 缺条目")
            continue
        if entry.get("evidence") != "measured_local":
            budget_bad.append(f"{eid} evidence={entry.get('evidence')!r}")
        ef = entry.get("evidence_file") or ""
        ep = ROOT / ef
        if not ep.is_file():
            budget_bad.append(f"{eid} evidence_file 不在树")
            continue
        try:
            tm = wel.load_json(ep).get("results", {}).get("trimmed_mean")
        except (OSError, ValueError):
            tm = None
        if not isinstance(tm, (int, float)):
            budget_bad.append(f"{eid} evidence 缺 results.trimmed_mean")
            continue
        if entry.get("measured_value") != tm or entry.get("threshold") != tm * k:
            budget_bad.append(f"{eid} threshold/measured ≠ trimmed_mean×k（{tm}×{k}）")
        if "sha256:" not in str(entry.get("description", "")):
            budget_bad.append(f"{eid} description 缺样本集 digest 引用")
    facts.append(
        _fact(
            "calibrated_thresholds_in_budget_with_provenance",
            not budget_bad,
            (
                "g11_budget.json 四条 g11.caliber.*/g11.metric.* 标定条目 measured_local + evidence_file 在树可解 "
                "trimmed_mean + threshold == trimmed_mean × k + 样本集 digest 引用齐备（P-09）"
                if not budget_bad
                else "; ".join(budget_bad[:3])
            ),
        )
    )

    # ⑤ 四门 RED 臂独立有效：最新 evidence 各含 red_* checks 且全真。
    red_bad: list[str] = []
    red_total = 0
    for _key, prefix in RED_ARM_GATES:
        path = wel.load_latest_evidence(prefix)
        if path is None:
            red_bad.append(f"{prefix} 缺最新 evidence")
            continue
        doc = wel.load_json(path)
        red_checks = {k: v for k, v in (doc.get("checks") or {}).items() if k.startswith("red_")}
        red_total += len(red_checks)
        if not red_checks or any(v is not True for v in red_checks.values()):
            red_bad.append(f"{prefix} red_* checks 缺失或非真 {red_checks}")
    facts.append(
        _fact(
            "four_gates_red_arms_independently_effective",
            not red_bad,
            (
                f"M144/M145/M146/M157 四门最新 evidence 各含 red_* checks 且全真（共 {red_total} 臂独立有效）"
                if not red_bad
                else "; ".join(red_bad[:3])
            ),
        )
    )
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_extra_facts()
    notes_parts = [
        "implemented: four G11.2 gates (P0 M144 caliber_c1_indoor_luminance step 196 / "
        "P0 M145 caliber_c2_exposure_chain step 197 / P0 M146 caliber_c3_exr_bit_depth step 198 / "
        "P1 M157 hdr_flip_calibration step 199)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: RXS-0392/0393 clause heads + RFC-0028 Agent Approved literal + "
        "residual caliber registry complete + calibrated thresholds in g11_budget with provenance (P-09) + "
        "four gates red arms independently effective",
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
    """① 缺四门 evidence → 红；② 真树四门绿 + 事实核验 → 绿。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g11_wave2_selftest_") as td:
        code = run_gate(evidence_dir=Path(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 正样本:仓库最新四门 evidence")
    # 负/正样本 evidence 文件名按 UTC 秒戳命名——隔开 1.1s 防同秒同名覆写
    # （evidence/ 只增不删不改，负样本 FAIL 件诚实留痕）。
    import time

    time.sleep(1.1)
    code = run_gate(evidence_dir=None)
    if code != 0:
        print("[selftest] FAIL: 真树聚合未绿（前置四门/事实核验未满足）", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真树聚合绿")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G11.2 wave2.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
