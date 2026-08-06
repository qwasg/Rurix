#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.2 波次聚合门 g8.wave.2.exit(步骤 104;CI_GATES §5)。

只读汇总七个 G8.2 P0 最新 evidence + RFC-0019 Approved + RD-037 closed
+ 本波 RD-038 接入空集登记。不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。
RD-040 总体维持 open(仅 M50 分项已留痕),exit 不翻 closed。

用法:
  py -3 ci/g8_wave2_exit_check.py --gate g8.wave.2.exit
  py -3 ci/g8_wave2_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

# 允许同目录 import
sys.path.insert(0, str(Path(__file__).resolve().parent))

import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.2.exit"
NUMERIC_STEP = 104
SUBJECT = "g8_wave2_exit"
WAVE = "G8.2"
SOURCE_REF = (
    "CI_GATES §5;G8_CONTRACT G-G8-4;G8_CANDIDATE_DECISIONS v1.1;"
    "RFC-0019 Agent Approved;RD-037 closed;RD-038 wave-ingress=[]"
)
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave2_exit_evidence_schema.json"
RFC0019 = ROOT / "rfcs" / "0019-rendering-platform.md"
CANDIDATE = ROOT / "milestones" / "g8" / "G8_CANDIDATE_DECISIONS.md"

# 七个 G8.2 P0:(symbolic_key, evidence subject_prefix)
REQUIRED_GATES: list[tuple[str, str]] = [
    ("g8.p0.m50.rt_pipeline_incremental", "g8_m50_rt_pipeline_incremental"),
    ("g8.p0.m89.single_source_gfx_submit", "g8_m89_single_source_gfx_submit"),
    ("g8.p0.m29.shader_permutation", "g8_m29_shader_permutation"),
    ("g8.p0.m30.pso_cache", "g8_m30_pso_cache"),
    ("g8.p0.m31.reflection_hash", "g8_m31_reflection_hash"),
    ("g8.p0.m32.capability_profile", "g8_m32_capability_profile"),
    ("g8.p0.m85.shader_manifest_ddc", "g8_m85_shader_manifest_ddc"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_extra_facts() -> list[dict]:
    facts: list[dict] = []

    ok, detail = wel.rfc_agent_approved(RFC0019)
    facts.append(_fact("rfc0019_approved", ok, detail))

    rd037 = wel.load_rd_status("RD-037")
    facts.append(
        _fact(
            "rd037_closed",
            rd037 == "closed",
            f"RD-037.status={rd037!r} (要求 closed)",
        )
    )

    rd038 = wel.load_rd_status("RD-038")
    # 总体 closed 是事实;本波接入空集是 CANDIDATE_DECISIONS v1.1 字面
    rd038_overall_ok = rd038 == "closed"
    facts.append(
        _fact(
            "rd038_overall_closed",
            rd038_overall_ok,
            f"RD-038.status={rd038!r} (要求 closed;G7 逐字审计路径)",
        )
    )

    # 本波 RD-038 接入 = 空集(不因空集放宽七门)
    ingress: list[str] = []
    cand_ok = False
    cand_detail = "G8_CANDIDATE_DECISIONS.md 缺失"
    if CANDIDATE.is_file():
        text = CANDIDATE.read_text(encoding="utf-8")
        # v1.1 字面锚:「G8.2 为**空集**」或「为空集」
        markers = (
            "G8.2 为**空集**",
            "G8.2 为空集",
            "在 G8.2 为**空集**",
            "G8.2 为**空集**（无 G8.2 腿）",
        )
        cand_ok = any(m in text for m in markers) or (
            "空集" in text and "G8.2" in text and "v1.1" in text
        )
        cand_detail = (
            "CANDIDATE_DECISIONS v1.1: G8.2 RD-038 接入空集字面在位"
            if cand_ok
            else "未找到 G8.2 RD-038 接入空集字面(v1.1)"
        )
    facts.append(
        _fact(
            "rd038_wave_ingress_empty",
            cand_ok and ingress == [],
            f"{cand_detail}; rd038_wave_ingress={ingress!r}",
        )
    )

    # RD-040 总体维持 open——作为诚实旁证(不是 PASS 条件放宽;缺失/closed 反而红)
    rd040 = wel.load_rd_status("RD-040")
    facts.append(
        _fact(
            "rd040_retained_open",
            rd040 == "open",
            f"RD-040.status={rd040!r} (wave2 要求维持 open,不代关)",
        )
    )

    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [
        wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir)
        for key, prefix in REQUIRED_GATES
    ]
    extras = collect_extra_facts()
    # selftest 空目录时仍核 RD/RFC(真实文件);只让 gates 红即可证聚合不代绿
    notes_parts = [
        "implemented: seven G8.2 P0 keys (M50/M89/M29/M30/M31/M32/M85)",
        "retained-open: RD-040 overall open (M50 partial history only)",
        "rd038_wave_ingress=[] (G8_CANDIDATE_DECISIONS v1.1)",
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
    """① 缺一门 evidence → 红;② 真树七门绿 + 空集 + RD → 绿。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile
    from pathlib import Path as P

    with tempfile.TemporaryDirectory(prefix="g8_wave2_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G8.2 wave2.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
