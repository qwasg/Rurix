#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.5 吞吐基线波）
"""G12.5 波次聚合门 g12.wave.5.exit（步骤 229;milestones/g12/CI_GATES.md §5;
G12_CONTRACT G-G12-7;同构 ci/g12_wave4_exit_check.py + ci/g12_wave_exit_lib.py）。

只读汇总 G12.5 波 P0 门最新 evidence——M165 PT 吞吐优化基线（步骤 228）——
+ 六 facts:
① M96 正确性锚 0-byte(g9.p0.m96 最新 evidence PASS + 冻结带/参照器面
   git diff 闭集机核);
② 吞吐基线 8 条目入 g12_budget measured_local 零 estimated + 逐条目
   evidence 在档 + budget_eval 全 PASS(P-09 禁手写);
③ 不设通过线登记机核(M165 最新 evidence baseline.zero_pass_line 字面 +
   8 条目描述逐个携带「不构成帧率对标通过线」字面 + 零帧率对标通过线声明
   ——以基线冒充帧率对标即 RED 的判定面);
④ 正确性锚断言(M165 evidence 4 cell digest_match 全真 + distinct==1 +
   evolution_register null + 冻结锚集 == 门内注册字面——digest 漂移未登记
   即 RED 的判定面);
⑤ M165 RED 臂独立有效(三臂 checks 全真——基线冒充帧率对标/digest 漂移
   未登记/estimated 冒充);
⑥ 62 门零降级(M164 最新 evidence PASS 承载)+ 本波触改二进制面零降级
   机核(M163 全档复跑最新 evidence PASS 且 base_commit == M165 同值——
   g12_4_ue_pt_parity_render 加性 --benchmark 扩展面共享二进制回归锚)。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g12_wave5_exit_check.py --gate g12.wave.5.exit
  py -3 ci/g12_wave5_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g12_pt_prod_lib as gl  # noqa: E402
import g12_wave_exit_lib as wel  # noqa: E402
import g12_pt_throughput_baseline_smoke as m165  # noqa: E402

GATE_KEY = "g12.wave.5.exit"
NUMERIC_STEP = 229
SUBJECT = "g12_wave5_exit"
WAVE = "G12.5"
SOURCE_REF = (
    "milestones/g12/CI_GATES.md §5;G12_CONTRACT G-G12-7;G12_ACCEPTANCE_MAP §1;"
    "M165 throughput baseline 8 budget entries measured_local + zero pass-line registration;"
    "correctness anchor fixed-seed digest 0-byte (M163 receipt frozen);"
    "62 gates zero-degrade + M163 rerun same-base_commit binary regression anchor"
)
SCHEMA_PATH = ROOT / "milestones" / "g12" / "g12_wave5_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g12.p0.m165.pt_throughput_baseline", "g12_m165_pt_throughput_baseline"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _m165_baseline_doc() -> tuple[Path | None, dict]:
    path = wel.load_latest_evidence("g12_m165_pt_throughput_baseline")
    if path is None:
        return None, {}
    try:
        return path, wel.load_json(path)
    except Exception:  # noqa: BLE001
        return path, {}


def collect_facts() -> list[dict]:
    facts: list[dict] = []

    # ① M96 正确性锚 0-byte(门最新 PASS + 冻结面 diff 闭集)。
    m96_row = wel.require_gate_pass("g9.p0.m96.path_tracer_reference", "g9_m96_path_tracer_reference")
    surf_ok, surf_msg = gl.m96_frozen_surface_unchanged()
    facts.append(_fact(
        "m96_correctness_anchor_0byte",
        m96_row["status"] == "PASS" and surf_ok,
        f"M96 门最新 {m96_row['status']}({m96_row.get('detail', '')[:60]});冻结面: {surf_msg}",
    ))

    # ② 吞吐基线 8 条目入 budget measured_local 零 estimated + 逐条目 evidence
    #    在档 + budget_eval 全 PASS。
    bud_bad: list[str] = []
    budget = gl.load_budget()
    for eid in m165.budget_entry_ids():
        e = gl.budget_entry(budget, eid)
        if e is None:
            bud_bad.append(f"缺条目 {eid}")
            continue
        if e.get("evidence") != "measured_local":
            bud_bad.append(f"{eid} 非 measured_local")
            continue
        ef = e.get("evidence_file", "")
        if not (ROOT / ef).is_file():
            bud_bad.append(f"{eid} evidence_file 缺失")
    r = gl.run(["py", "-3", "ci/budget_eval.py"])
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "throughput_baseline_8_entries_measured_and_eval_pass",
        not bud_bad,
        "8 基线条目(frame_ms×4 + primary_rays_sec×4)齐备 measured_local 零 estimated + 逐条目 evidence 在档 + budget_eval 全 PASS(P-09)"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ③ 不设通过线登记机核(evidence 字面 + 8 条目描述字面 + 零冒充声明)。
    _p, m165_doc = _m165_baseline_doc()
    baseline = m165_doc.get("baseline") or {}
    zpl = baseline.get("zero_pass_line") or ""
    entries_ok = True
    for eid in m165.budget_entry_ids():
        e = gl.budget_entry(budget, eid) or {}
        if m165.baseline_registration_problems(e):
            entries_ok = False
    z_ok = (
        m165.NO_PASS_LINE_LITERAL in zpl
        and "G14" in zpl
        and entries_ok
        and (m165_doc.get("checks") or {}).get("no_pass_line_registered") is True
    )
    facts.append(_fact(
        "no_pass_line_registration_machine_check",
        z_ok,
        f"zero_pass_line 字面={'构成帧率对标通过线' in zpl and 'G14' in zpl};8 条目登记校验={entries_ok};门 checks 登记={(m165_doc.get('checks') or {}).get('no_pass_line_registered')}",
    ))

    # ④ 正确性锚断言(4 cell digest_match + distinct==1 + 演进位 null + 冻结锚
    #    集 == 门内注册字面)。
    cells = baseline.get("cells") or []
    anchor = baseline.get("correctness_anchor") or {}
    frozen = anchor.get("frozen_digests") or {}
    frozen_expected = {f"{s}|spp{n}": d for (s, n), d in m165.FROZEN_FRAME_DIGESTS.items()}
    a_ok = (
        len(cells) == 4
        and all(c.get("digest_match") is True for c in cells)
        and all(c.get("distinct_frame_digests") == 1 for c in cells)
        and anchor.get("evolution_register", "x") is None
        and frozen == frozen_expected
        and anchor.get("kind") == "固定 seed digest 0-byte"
    )
    facts.append(_fact(
        "correctness_anchor_fixed_seed_digest_0byte",
        a_ok,
        f"cell 数={len(cells)} digest_match 全真={all(c.get('digest_match') is True for c in cells)} distinct 单值={all(c.get('distinct_frame_digests') == 1 for c in cells)} 演进位 null={anchor.get('evolution_register', 'x') is None} 冻结锚集字面一致={frozen == frozen_expected}",
    ))

    # ⑤ M165 RED 臂独立有效(三臂 checks 全真)。
    mchecks = m165_doc.get("checks") or {}
    red_keys = [k for k in m165.CHECK_KEYS if k.startswith("red_")]
    red_ok = bool(m165_doc) and all(mchecks.get(k) is True for k in red_keys) and len(red_keys) == 3
    facts.append(_fact(
        "m165_red_arms_independently_valid",
        red_ok,
        f"RED 臂 {len(red_keys)} 条全真={all(mchecks.get(k) is True for k in red_keys)}(基线冒充帧率对标/digest 漂移未登记/estimated 冒充)",
    ))

    # ⑥ 62 门零降级(M164 最新 PASS 承载) + 本波触改二进制面零降级机核
    #    (M163 全档复跑最新 PASS ∧ base_commit == M165 同值——共享 harness 二
    #    进制回归锚:任一重新基线/复跑必须同窗,漂移即红)。
    m164_row = wel.require_gate_pass("g12.p0.m164.regression_guard", "g12_m164_regression_guard")
    m163_path = wel.load_latest_evidence("g12_m163_ue_pt_parity")
    m163_doc = wel.load_json(m163_path) if m163_path is not None else {}
    same_base = bool(m163_doc) and bool(m165_doc) and m163_doc.get("base_commit") == m165_doc.get("base_commit")
    m163_pass = (m163_doc.get("status") == "pass") and all((m163_doc.get("checks") or {}).values())
    facts.append(_fact(
        "regression_62_zero_degrade_and_m163_rerun_same_base",
        m164_row["status"] == "PASS" and m163_pass and same_base,
        f"62 门零降级(M164 {m164_row['status']});M163 复跑 {'PASS' if m163_pass else 'FAIL/缺失'};base_commit 同窗={same_base}(M163={str(m163_doc.get('base_commit'))[:8]} vs M165={str(m165_doc.get('base_commit'))[:8]})",
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        extras = [
            _fact("m96_correctness_anchor_0byte", False, "selftest 空目录"),
            _fact("throughput_baseline_8_entries_measured_and_eval_pass", False, "selftest 空目录"),
            _fact("no_pass_line_registration_machine_check", False, "selftest 空目录"),
            _fact("correctness_anchor_fixed_seed_digest_0byte", False, "selftest 空目录"),
            _fact("m165_red_arms_independently_valid", False, "selftest 空目录"),
            _fact("regression_62_zero_degrade_and_m163_rerun_same_base", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G12.5 P0 gate (M165 pt_throughput_baseline step 228)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: M96 anchor 0-byte + 8 baseline entries measured + zero pass-line + digest 0-byte anchor + RED arms + 62 zero-degrade/M163 rerun same-base",
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
    """① 缺 M165 evidence → 红;② 真树聚合 VERDICT == 子门实测态(遮蔽即自检红)。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g12_wave5_selftest_") as td:
        code = run_gate(evidence_dir=Path(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 真树一致性:聚合 VERDICT == 子门实测态(不遮蔽机核)")
    rows = [wel.require_gate_pass(key, prefix) for key, prefix in REQUIRED_GATES]
    extras = collect_facts()
    expected_pass = all(r["status"] == "PASS" for r in rows) and all(f["status"] == "PASS" for f in extras)
    code = run_gate(evidence_dir=None)
    if (code == 0) != expected_pass:
        print(
            f"[selftest] FAIL: 聚合 VERDICT 与子门实测态不一致(遮蔽/代绿面)——expected_pass={expected_pass} exit={code}",
            file=sys.stderr,
        )
        return 1
    print(f"[selftest] PASS: 真树聚合 VERDICT={'PASS' if code == 0 else 'FAIL'} == 子门实测态(不遮蔽)")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G12.5 wave5.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
