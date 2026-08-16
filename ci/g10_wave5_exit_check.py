#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.5b 波）
"""G10.5 波次聚合门 g10.wave.5.exit（步骤 191；milestones/g10/CI_GATES.md §5；
G10_CONTRACT G-G10-7；同构 ci/g10_wave4_exit_check.py + ci/g10_wave_exit_lib.py）。

只读汇总 G10.5 波四门最新 evidence——M139 A/B 对比门（步骤 188）/ M140
差距清单登记门（步骤 189）/ M141 性能对标基线门（步骤 190）/ M130 双端
核验腿（--phase g10.5，步骤 187，须 status==pass 且 phase_g10_5_pass==true，
MAP §3.3 双阶段口径）——+ spec 条款头在树（visual_comparison.md RXS-0391
+ RXS-0390/RXS-0384 回顾，共 3 枚）+ RFC-0026/0027 双 Agent Approved 字面
在树 + 门序三重绑定留痕（M139 最新 evidence 内嵌 three_binding ==
M130 g10.5 最新 evidence 登记面：param_digest/session_run_id/base_commit
逐字相等，只读复核）+ 差距清单场景全集零空行（M140 最新 evidence 登记
per_scene 双场景在列 + not_ready_scenes 显式在列 + 清单文件 digest ==
M140 登记 digest）+ 三门 RED 臂独立有效（M139/M140/M141 最新 evidence
各含 red_* checks 且全真）。不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。
聚合 PASS 不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE。

用法：
  py -3 ci/g10_wave5_exit_check.py --gate g10.wave.5.exit
  py -3 ci/g10_wave5_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import g10_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g10.wave.5.exit"
NUMERIC_STEP = 191
SUBJECT = "g10_wave5_exit"
WAVE = "G10.5"
SOURCE_REF = (
    "milestones/g10/CI_GATES.md §5;G10_CONTRACT G-G10-7;G10_ACCEPTANCE_MAP §1/§3.3;"
    "RFC-0026 §4.4~§4.6 + RFC-0027;RXS-0391 clause head on tree (+RXS-0390/RXS-0384);"
    "gate-order triple-binding trace;gap registry scene-set zero-empty;three gates red arms independently effective"
)
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_wave5_exit_evidence_schema.json"
RFC0026 = ROOT / "rfcs" / "0026-visual-comparison-metrics.md"
RFC0027 = ROOT / "rfcs" / "0027-external-reference-harness-license.md"
SPEC_VC = ROOT / "spec" / "visual_comparison.md"
REGISTRY_PATH = ROOT / "milestones" / "g10" / "g10_gap_registry.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g10.p0.m139.ab_comparison", "g10_m139_ab_comparison"),
    ("g10.p0.m140.gap_registry", "g10_m140_gap_registry"),
    ("g10.p0.m141.perf_baseline", "g10_m141_perf_baseline"),
]

_RXS_HEAD_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _latest_m130_g10_5() -> tuple[Path | None, dict | None]:
    """最新 M130 双端核验期 evidence（timestamp 主键 + 文件名次键，仍并列
    fail-closed——RFC-0026 §4.6「最新」排序字面）。"""
    cands = []
    for f in wel.EVIDENCE_DIR.glob("g10_m130_dual_determinism_contract_*.json"):
        try:
            doc = wel.load_json(f)
        except Exception:
            continue
        if doc.get("phase") == "g10.5":
            cands.append((doc.get("timestamp", ""), f.name, f, doc))
    if not cands:
        return None, None
    cands.sort(key=lambda t: (t[0], t[1]))
    top = [c for c in cands if (c[0], c[1]) == (cands[-1][0], cands[-1][1])]
    if len(top) != 1:
        return None, None
    return top[0][2], top[0][3]


def collect_rows(evidence_dir: Path | None = None) -> list[dict]:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    # M130 双端核验腿（--phase g10.5）：最新 g10.5 期 evidence 须 status==pass
    # 且 phase_g10_5_pass==true（骨架期绿不替双端核验期充绿，MAP §3.3）。
    if evidence_dir is None:
        m130_path, m130_doc = _latest_m130_g10_5()
    else:
        m130_path, m130_doc = None, None
    row = {
        "symbolic_gate_key": "g10.p0.m130.dual_determinism_contract --phase g10.5",
        "subject_prefix": "g10_m130_dual_determinism_contract (phase=g10.5)",
        "evidence_path": None if m130_path is None else str(m130_path.relative_to(ROOT)).replace("\\", "/"),
        "status": "FAIL",
        "detail": "",
    }
    if m130_doc is None:
        row["detail"] = "缺 M130 g10.5 期 evidence（或最新判定并列）"
    else:
        ok = m130_doc.get("status") == "pass" and m130_doc.get("phase_g10_5_pass") is True
        row["status"] = "PASS" if ok else "FAIL"
        row["detail"] = "phase_g10_5_pass==true" if ok else f"status={m130_doc.get('status')!r} phase_g10_5_pass={m130_doc.get('phase_g10_5_pass')!r}"
        row["timestamp"] = m130_doc.get("timestamp")
        row["device_section_state"] = m130_doc.get("device_section_state")
        row["host_section_pass"] = m130_doc.get("host_section_pass")
    rows.append(row)
    return rows


def collect_extra_facts() -> list[dict]:
    facts: list[dict] = []

    # ① spec 条款头在树（visual_comparison.md RXS-0391 新增 + RXS-0390/RXS-0384 回顾，共 3 枚）。
    heads: set[int] = set()
    if SPEC_VC.is_file():
        heads = {int(m) for m in _RXS_HEAD_RE.findall(SPEC_VC.read_text(encoding="utf-8"))}
    missing = sorted({391, 390, 384} - heads)
    facts.append(
        _fact(
            "rxs0391_0390_0384_clause_heads_on_tree",
            SPEC_VC.is_file() and not missing,
            (
                "spec/visual_comparison.md RXS-0391（差距清单 schema）+ RXS-0390/RXS-0384 条款头全在树（共 3 枚）"
                if SPEC_VC.is_file() and not missing
                else f"spec 缺失或缺条款头: {missing}"
            ),
        )
    )

    # ② RFC-0026/0027 双 Agent Approved 字面在树。
    ok26, detail26 = wel.rfc_agent_approved(RFC0026)
    ok27, detail27 = wel.rfc_agent_approved(RFC0027)
    facts.append(
        _fact(
            "rfc0026_0027_agent_approved",
            ok26 and ok27,
            f"RFC-0026: {detail26}; RFC-0027: {detail27}",
        )
    )

    # ③ 门序三重绑定留痕：M139 最新 evidence 内嵌 three_binding == M130 g10.5
    # 最新 evidence 登记面（param_digest/session_run_id/base_commit 逐字相等）。
    bind_bad: list[str] = []
    m139_path = wel.load_latest_evidence("g10_m139_ab_comparison")
    m130_path, m130_doc = _latest_m130_g10_5()
    if m139_path is None:
        bind_bad.append("缺 M139 最新 evidence")
    if m130_doc is None:
        bind_bad.append("缺 M130 g10.5 期 evidence")
    if not bind_bad:
        m139 = wel.load_json(m139_path)
        tb = m139.get("ab_report", {}).get("three_binding", {})
        rep = m130_doc.get("contract_report", {})
        for k, m130_val in (
            ("param_digest", rep.get("param_digest")),
            ("session_run_id", rep.get("session_run_id")),
            ("base_commit", m130_doc.get("base_commit")),
        ):
            if not m130_val or tb.get(k) != m130_val:
                bind_bad.append(f"three_binding.{k} 不等: M139={tb.get(k)!r} vs M130={m130_val!r}")
        m130_rel = str(m130_path.relative_to(ROOT)).replace("\\", "/")
        if tb.get("m130_evidence_path") != m130_rel:
            bind_bad.append(f"m130_evidence_path 漂移: {tb.get('m130_evidence_path')!r} vs {m130_rel!r}")
    facts.append(
        _fact(
            "gate_order_triple_binding_trace",
            not bind_bad,
            (
                "M139 内嵌 three_binding 与 M130 g10.5 最新 evidence 逐字相等"
                "（param_digest/session_run_id/base_commit/m130_evidence_path——门序硬约束留痕）"
                if not bind_bad
                else "; ".join(bind_bad[:3])
            ),
        )
    )

    # ④ 差距清单场景全集零空行 + not_ready 显式在列（M140 最新 evidence 登记面
    # + 清单文件 digest == M140 登记 digest）。
    gap_bad: list[str] = []
    m140_path = wel.load_latest_evidence("g10_m140_gap_registry")
    if m140_path is None:
        gap_bad.append("缺 M140 最新 evidence")
    else:
        m140 = wel.load_json(m140_path)
        rep = m140.get("gap_registry_report", {})
        per_scene = rep.get("per_scene", {})
        if sorted(per_scene) != ["bistro-interior", "cornell-box"]:
            gap_bad.append(f"per_scene 行集不全等: {sorted(per_scene)}")
        if any(not isinstance(v, int) or v < 0 for v in per_scene.values()):
            gap_bad.append(f"per_scene 计数异常: {per_scene}")
        if not isinstance(rep.get("not_ready_scenes"), list):
            gap_bad.append("not_ready_scenes 未显式在列")
        if not REGISTRY_PATH.is_file():
            gap_bad.append("清单文件不在树")
        else:
            digest = "sha256:" + hashlib.sha256(REGISTRY_PATH.read_bytes()).hexdigest()
            if rep.get("registry_digest") != digest:
                gap_bad.append("清单文件 digest ≠ M140 登记 digest（漂移）")
    facts.append(
        _fact(
            "gap_registry_scene_set_zero_empty_rows",
            not gap_bad,
            (
                "差距清单场景全集零空行（双场景 per_scene 在列 + not_ready_scenes 显式 + 清单 digest 无漂移）"
                if not gap_bad
                else "; ".join(gap_bad[:3])
            ),
        )
    )

    # ⑤ 三门 RED 臂独立有效：M139/M140/M141 最新 evidence 各含 red_* checks 且全真。
    red_bad: list[str] = []
    red_total = 0
    for _key, prefix in REQUIRED_GATES:
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
            "three_gates_red_arms_independently_effective",
            not red_bad,
            (
                f"M139/M140/M141 三门最新 evidence 各含 red_* checks 且全真（共 {red_total} 臂独立有效）"
                if not red_bad
                else "; ".join(red_bad[:3])
            ),
        )
    )
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = collect_rows(evidence_dir=evidence_dir)
    extras = collect_extra_facts()
    notes_parts = [
        "implemented: three G10.5 gates (P0 M139 ab_comparison step 188 / "
        "P0 M140 gap_registry step 189 / P0 M141 perf_baseline step 190) + "
        "M130 dual-end verification leg (--phase g10.5 step 187, phase_g10_5_pass==true)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: RXS-0391 clause head + RFC-0026/0027 Agent Approved literals + "
        "gate-order triple-binding trace + gap registry scene-set zero-empty + three gates red arms independently effective",
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

    with tempfile.TemporaryDirectory(prefix="g10_wave5_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G10.5 wave5.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
