#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.4b 波）
"""G10.4 波次聚合门 g10.wave.4.exit（步骤 186；milestones/g10/CI_GATES.md §5；
G10_CONTRACT G-G10-5；同构 ci/g10_wave3_exit_check.py + ci/g10_wave_exit_lib.py）。

只读汇总 G10.4 波五门最新 evidence——M134 帧捕获管线（步骤 181）/ M135
FLIP 度量（步骤 184）/ M136 SSIM/PSNR 度量（步骤 182）/ M137 逐像素 diff
报告（步骤 183）/ M138 阈值标定（步骤 185）——+ spec 条款头在树
（imageio.md RXS-0385 + visual_comparison.md RXS-0386~0389，共 5 枚）+
RFC-0026 Agent Approved 字面在树 + 标定值入 g10_budget 且 provenance 齐备
（五条 g10.metric.* 条目 measured_local + evidence_file 在树可解
results.trimmed_mean + threshold == trimmed_mean × k 重算口径，P-09）+
四门 RED 臂独立有效（M134/M135/M136/M137 最新 evidence 各含 red_* checks
且全真）。不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽
任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE。

用法：
  py -3 ci/g10_wave4_exit_check.py --gate g10.wave.4.exit
  py -3 ci/g10_wave4_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import g10_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g10.wave.4.exit"
NUMERIC_STEP = 186
SUBJECT = "g10_wave4_exit"
WAVE = "G10.4"
SOURCE_REF = (
    "milestones/g10/CI_GATES.md §5;G10_CONTRACT G-G10-5;G10_ACCEPTANCE_MAP §1/§2;"
    "RFC-0026 §4.1~§4.4;RXS-0385~0389 clause heads on tree;"
    "calibrated thresholds in g10_budget with provenance (P-09);four gates red arms independently effective"
)
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_wave4_exit_evidence_schema.json"
RFC0026 = ROOT / "rfcs" / "0026-visual-comparison-metrics.md"
SPEC_IMAGEIO = ROOT / "spec" / "imageio.md"
SPEC_VC = ROOT / "spec" / "visual_comparison.md"
BUDGET_PATH = ROOT / "milestones" / "g10" / "g10_budget.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g10.p0.m134.frame_capture_pipeline", "g10_m134_frame_capture_pipeline"),
    ("g10.p0.m135.flip_metric", "g10_m135_flip_metric"),
    ("g10.p0.m136.ssim_psnr_metric", "g10_m136_ssim_psnr_metric"),
    ("g10.p0.m137.pixel_diff_report", "g10_m137_pixel_diff_report"),
    ("g10.p1.m138.metric_threshold_calibration", "g10_m138_metric_threshold_calibration"),
]

# §5 wave4 行「四条 RED 臂独立有效」面：四门最新 evidence 的 red_* checks 闭集。
RED_ARM_GATES: list[tuple[str, str]] = REQUIRED_GATES[:4]

# M138 标定值入 budget 的五条条目闭集（id → k）。
CALIB_BUDGET_ENTRIES: list[tuple[str, float]] = [
    ("g10.metric.flip_pairwise_scalar_tol", 2.0),
    ("g10.metric.flip_pairwise_error_map_tol", 2.0),
    ("g10.metric.ssim_pairwise_tol", 2.0),
    ("g10.metric.psnr_pairwise_tol", 2.0),
    ("g10.metric.diff_report_over_threshold", 1.0),
]

_RXS_HEAD_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_extra_facts() -> list[dict]:
    facts: list[dict] = []

    # ① spec 条款头在树（imageio.md RXS-0385 + visual_comparison.md RXS-0386~0389，共 5 枚）。
    heads_io: set[int] = set()
    heads_vc: set[int] = set()
    if SPEC_IMAGEIO.is_file():
        heads_io = {int(m) for m in _RXS_HEAD_RE.findall(SPEC_IMAGEIO.read_text(encoding="utf-8"))}
    if SPEC_VC.is_file():
        heads_vc = {int(m) for m in _RXS_HEAD_RE.findall(SPEC_VC.read_text(encoding="utf-8"))}
    missing = sorted(({385} - heads_io) | ({386, 387, 388, 389} - heads_vc))
    facts.append(
        _fact(
            "rxs0385_0389_clause_heads_on_tree",
            SPEC_IMAGEIO.is_file() and SPEC_VC.is_file() and not missing,
            (
                "spec/imageio.md RXS-0385 + spec/visual_comparison.md RXS-0386~0389 条款头全在树（共 5 枚）"
                if SPEC_IMAGEIO.is_file() and SPEC_VC.is_file() and not missing
                else f"spec 缺失或缺条款头: {missing}"
            ),
        )
    )

    # ② RFC-0026 Agent Approved 字面在树。
    ok_rfc, detail_rfc = wel.rfc_agent_approved(RFC0026)
    facts.append(_fact("rfc0026_agent_approved", ok_rfc, f"RFC-0026: {detail_rfc}"))

    # ③ 标定值入 g10_budget 且 provenance 齐备（P-09）：五条 g10.metric.* 条目
    # measured_local + evidence_file 在树可解 results.trimmed_mean +
    # threshold == trimmed_mean × k（重算口径）+ 条目 description 含样本集 digest 引用。
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
                "g10_budget.json 五条 g10.metric.* 标定条目 measured_local + evidence_file 在树可解 "
                "trimmed_mean + threshold == trimmed_mean × k + 样本集 digest 引用齐备（P-09）"
                if not budget_bad
                else "; ".join(budget_bad[:3])
            ),
        )
    )

    # ④ 四门 RED 臂独立有效：M134/M135/M136/M137 最新 evidence 各含 red_* checks 且全真。
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
                f"M134/M135/M136/M137 四门最新 evidence 各含 red_* checks 且全真（共 {red_total} 臂独立有效）"
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
        "implemented: five G10.4 gates (P0 M134 frame_capture_pipeline step 181 / "
        "P0 M135 flip_metric step 184 / P0 M136 ssim_psnr_metric step 182 / "
        "P0 M137 pixel_diff_report step 183 / P1 M138 metric_threshold_calibration step 185)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: RXS-0385~0389 clause heads + RFC-0026 Agent Approved literal + "
        "calibrated thresholds in g10_budget with provenance (P-09) + four gates red arms independently effective",
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
    """① 缺五门 evidence → 红；② 真树五门绿 + 事实核验 → 绿。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g10_wave4_selftest_") as td:
        code = run_gate(evidence_dir=Path(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 正样本:仓库最新五门 evidence")
    # 负/正样本 evidence 文件名按 UTC 秒戳命名——隔开 1.1s 防同秒同名覆写
    # （evidence/ 只增不删不改，负样本 FAIL 件诚实留痕）。
    import time

    time.sleep(1.1)
    code = run_gate(evidence_dir=None)
    if code != 0:
        print("[selftest] FAIL: 真树聚合未绿（前置五门/事实核验未满足）", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真树聚合绿")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G10.4 wave4.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
