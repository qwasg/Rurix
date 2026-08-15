#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.2 波 materialize）
"""G10.2 波次聚合门 g10.wave.2.exit（步骤 180；milestones/g10/CI_GATES.md §5；
G10_CONTRACT G-G10-4；同构 ci/g10_wave3_exit_check.py + ci/g10_wave_exit_lib.py）。

只读汇总 G10.2 波三门最新 evidence——M128 UE5 出图环境（步骤 177）/ M129
UE5 参考帧（步骤 178）/ M130 双端确定性契约骨架期（步骤 179）——+
spec/external_reference.md RXS-0380 与 spec/visual_comparison.md RXS-0384
条款头在树 + RFC-0026/0027 Agent Approved 字面在树 + 暂定场景集登记与偏差
如实登记 + M130 双 phase 纪律（最新 evidence phase_g10_2_pass=true 且
phase_g10_5_pass=false，骨架期绿不替双端核验期充绿）。不重跑 smoke、不代绿、
不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE。

用法：
  py -3 ci/g10_wave2_exit_check.py --gate g10.wave.2.exit
  py -3 ci/g10_wave2_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import g10_ue5_lib as uelib  # noqa: E402
import g10_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g10.wave.2.exit"
NUMERIC_STEP = 180
SUBJECT = "g10_wave2_exit"
WAVE = "G10.2"
SOURCE_REF = (
    "milestones/g10/CI_GATES.md §5;G10_CONTRACT G-G10-4;G10_ACCEPTANCE_MAP §1/§3.3;"
    "RFC-0026 §4.6/RFC-0027 §4.1;RXS-0380/RXS-0384 clause heads on tree;"
    "provisional scene set registered;M130 phase discipline"
)
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_wave2_exit_evidence_schema.json"
RFC0026 = ROOT / "rfcs" / "0026-visual-comparison-metrics.md"
RFC0027 = ROOT / "rfcs" / "0027-external-reference-harness-license.md"
SPEC_XR = ROOT / "spec" / "external_reference.md"
SPEC_VC = ROOT / "spec" / "visual_comparison.md"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g10.p0.m128.ue5_capture_environment", "g10_m128_ue5_capture_environment"),
    ("g10.p0.m129.ue5_reference_frames", "g10_m129_ue5_reference_frames"),
    ("g10.p0.m130.dual_determinism_contract", "g10_m130_dual_determinism_contract"),
]

_RXS_HEAD_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_extra_facts() -> list[dict]:
    facts: list[dict] = []

    # ① spec 条款头在树：RXS-0380（external_reference）+ RXS-0384（visual_comparison）。
    heads_xr: set[int] = set()
    if SPEC_XR.is_file():
        heads_xr = {int(m) for m in _RXS_HEAD_RE.findall(SPEC_XR.read_text(encoding="utf-8"))}
    heads_vc: set[int] = set()
    if SPEC_VC.is_file():
        heads_vc = {int(m) for m in _RXS_HEAD_RE.findall(SPEC_VC.read_text(encoding="utf-8"))}
    ok_heads = 380 in heads_xr and 384 in heads_vc
    facts.append(
        _fact(
            "rxs0380_rxs0384_clause_heads_on_tree",
            ok_heads,
            (
                "RXS-0380（external_reference.md）与 RXS-0384（visual_comparison.md）条款头全在树"
                if ok_heads
                else f"缺条款头: external_reference 有 0380={380 in heads_xr}, visual_comparison 有 0384={384 in heads_vc}"
            ),
        )
    )

    # ② RFC-0026 / RFC-0027 Agent Approved 字面在树。
    ok26, d26 = wel.rfc_agent_approved(RFC0026)
    ok27, d27 = wel.rfc_agent_approved(RFC0027)
    facts.append(
        _fact(
            "rfc0026_rfc0027_agent_approved",
            ok26 and ok27,
            f"RFC-0026: {d26}; RFC-0027: {d27}" if ok26 and ok27 else f"0026={d26} | 0027={d27}",
        )
    )

    # ③ 暂定场景集登记 + 偏差如实登记（RFC-0027 §4.4 F8 形态）。
    ss_ok = True
    ss_detail = "g10_2_provisional_scene_set.json 在树：单场景闭集 + deviation_note 非空（CornellBox/Bistro 缺口如实登记）"
    if not uelib.PROVISIONAL_SCENE_SET_PATH.is_file():
        ss_ok, ss_detail = False, "暂定场景集登记件缺失"
    else:
        try:
            doc = uelib.load_json(uelib.PROVISIONAL_SCENE_SET_PATH)
            scenes = doc.get("scenes") or []
            ready = [s for s in scenes if s.get("status") == "ready"]
            if doc.get("kind") != "g10_2_provisional_scene_set" or not scenes or len(ready) != len(scenes):
                ss_ok, ss_detail = False, "场景集结构/ready 闭集异常"
            elif not str(doc.get("deviation_note", "")).strip():
                ss_ok, ss_detail = False, "deviation_note 缺失（缺口未如实登记）"
        except (OSError, ValueError) as e:
            ss_ok, ss_detail = False, f"场景集不可读: {e}"
    facts.append(_fact("provisional_scene_set_registered", ss_ok, ss_detail))

    # ④ M130 双 phase 纪律：最新 evidence 骨架期绿且不替双端核验期充绿。
    m130_path = wel.load_latest_evidence("g10_m130_dual_determinism_contract")
    phase_ok = False
    phase_detail = "M130 最新 evidence 缺失"
    if m130_path is not None:
        try:
            ev = wel.load_json(m130_path)
            if (
                ev.get("status") == "pass"
                and ev.get("phase") == "g10.2"
                and ev.get("phase_g10_2_pass") is True
                and ev.get("phase_g10_5_pass") is False
            ):
                phase_ok = True
                phase_detail = (
                    f"M130 骨架期绿 phase_g10_2_pass=true 且 phase_g10_5_pass=false"
                    f"（{m130_path.name}；双端核验腿归 G10.5）"
                )
            else:
                phase_detail = (
                    f"phase 纪律不符: status={ev.get('status')} phase={ev.get('phase')} "
                    f"g10_2={ev.get('phase_g10_2_pass')} g10_5={ev.get('phase_g10_5_pass')}"
                )
        except (OSError, ValueError) as e:
            phase_detail = f"M130 evidence 不可读: {e}"
    facts.append(_fact("m130_phase_discipline", phase_ok, phase_detail))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_extra_facts()
    notes_parts = [
        "implemented: three G10.2 gates (P0 M128 ue5_capture_environment step 177 / "
        "P0 M129 ue5_reference_frames step 178 / P0 M130 dual_determinism_contract "
        "skeleton phase g10.2 step 179)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: RXS-0380/RXS-0384 clause heads + RFC-0026/0027 Agent Approved literal + "
        "provisional scene set registered + M130 phase discipline",
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

    with tempfile.TemporaryDirectory(prefix="g10_wave2_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G10.2 wave2.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
