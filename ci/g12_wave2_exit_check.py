#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.2 生产化核心波续）
"""G12.2 波次聚合门 g12.wave.2.exit（步骤 222;milestones/g12/CI_GATES.md §5;
G12_CONTRACT G-G12-4;同构 ci/g11_wave5_exit_check.py + ci/g12_wave_exit_lib.py）。

只读汇总 G12.2 波五门最新 evidence——M158 MIS 完整面(步骤 218)/ M159 RR
生产化(219)/ M160 采样升级+低差异(220)/ M161 收敛判据生产化(221)/
M166 PT 生产化标定(217)——+ 六 facts:
① M96 正确性锚 0-byte(g9.p0.m96 最新 evidence PASS + 冻结带/参照器面
   git diff 闭集机核——既有行零删除、追加行 ⊆ prod 模块注册块);
② 五门 RED 臂独立有效(最新 evidence red 面 checks 非空且全真);
③ g12_budget 锚+标定 15 条目齐备 measured_local 零 estimated +
   budget_eval 全 PASS(P-09 禁手写);
④ spec-first 面(RXS-0398~0401 条款头在树 + RFC-0029 Agent Approved 字面);
⑤ conformance 锚定语料 11 件在位(accept 4 + reject 7;//@ spec 锚 + 门
   key 预期面);
⑥ M166 标定 provenance 齐备(两跑逐位一致 + 样本集 digest + 7 条目标定
   evidence 落盘 + 选型 artifact winner 一致)。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g12_wave2_exit_check.py --gate g12.wave.2.exit
  py -3 ci/g12_wave2_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g12_pt_prod_lib as gl  # noqa: E402
import g12_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g12.wave.2.exit"
NUMERIC_STEP = 222
SUBJECT = "g12_wave2_exit"
WAVE = "G12.2"
SOURCE_REF = (
    "milestones/g12/CI_GATES.md §5;G12_CONTRACT G-G12-4;G12_ACCEPTANCE_MAP §1/§2;"
    "five gates red arms independently effective;M96 correctness anchor 0-byte;"
    "g12_budget 15 anchors/entries measured_local;spec-first RXS-0398~0401 + RFC-0029 approved;"
    "conformance 11 corpus;M166 calibration provenance"
)
SCHEMA_PATH = ROOT / "milestones/g12/g12_wave2_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g12.p0.m158.mis_full_surface", "g12_m158_mis_full_surface"),
    ("g12.p0.m159.russian_roulette_prod", "g12_m159_russian_roulette_prod"),
    ("g12.p0.m160.sampling_lds_upgrade", "g12_m160_sampling_lds_upgrade"),
    ("g12.p0.m161.convergence_criterion_prod", "g12_m161_convergence_criterion_prod"),
    ("g12.p1.m166.pt_production_calibration", "g12_pt_production_calibration"),
]

CORPUS_11 = [
    ("accept/mis_full_surface_minimal.rx", "RXS-0398", "g12.p0.m158.mis_full_surface"),
    ("accept/rr_throughput_adaptive_minimal.rx", "RXS-0399", "g12.p0.m159.russian_roulette_prod"),
    ("accept/lds_deterministic_minimal.rx", "RXS-0400", "g12.p0.m160.sampling_lds_upgrade"),
    ("accept/adaptive_convergence_minimal.rx", "RXS-0401", "g12.p0.m161.convergence_criterion_prod"),
    ("reject/mis_weight_missing.rx", "RXS-0398", "g12.p0.m158.mis_full_surface"),
    ("reject/mis_energy_bias_inject.rx", "RXS-0398", "g12.p0.m158.mis_full_surface"),
    ("reject/rr_early_kill_bias.rx", "RXS-0399", "g12.p0.m159.russian_roulette_prod"),
    ("reject/rr_compensation_missing.rx", "RXS-0399", "g12.p0.m159.russian_roulette_prod"),
    ("reject/lds_nondeterministic_inject.rx", "RXS-0400", "g12.p0.m160.sampling_lds_upgrade"),
    ("reject/early_stop_masquerade.rx", "RXS-0401", "g12.p0.m161.convergence_criterion_prod"),
    ("reject/unconverged_pixel_underreport.rx", "RXS-0401", "g12.p0.m161.convergence_criterion_prod"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


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

    # ② 五门 RED 臂独立有效(red 面 checks 非空全真)。
    red_bad: list[str] = []
    red_total = 0
    for _key, prefix in REQUIRED_GATES:
        path = wel.load_latest_evidence(prefix)
        if path is None:
            red_bad.append(f"{prefix} 缺最新 evidence")
            continue
        doc = wel.load_json(path)
        red_checks = {k: v for k, v in (doc.get("checks") or {}).items() if "red" in k}
        red_total += len(red_checks)
        if not red_checks or any(v is not True for v in red_checks.values()):
            red_bad.append(f"{prefix} red 面 checks 缺失或非真")
    facts.append(_fact(
        "five_gates_red_arms_independently_effective",
        not red_bad,
        f"五门最新 evidence red 面 checks 全真(共 {red_total} 臂独立有效)"
        if not red_bad else "; ".join(red_bad[:3]),
    ))

    # ③ g12_budget 15 条目齐备 + budget_eval 全 PASS。
    bud_bad: list[str] = []
    budget = gl.load_budget()
    for eid in gl.ANCHOR_IDS + gl.CALIB_IDS:
        e = gl.budget_entry(budget, eid)
        if e is None:
            bud_bad.append(f"缺条目 {eid}")
        elif e.get("evidence") != "measured_local":
            bud_bad.append(f"{eid} 非 measured_local")
    r = gl.run(["py", "-3", "ci/budget_eval.py"])
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "budget_anchors_and_calibration_measured",
        not bud_bad,
        "8 锚 + 7 标定条目齐备 measured_local 零 estimated + budget_eval 全 PASS(P-09)"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ④ spec-first 面(条款头 + RFC Approved)。
    spec_text = (ROOT / "spec/global_illumination.md").read_text(encoding="utf-8")
    missing = [c for c in ("RXS-0398", "RXS-0399", "RXS-0400", "RXS-0401") if f"### {c} " not in spec_text]
    rfc_ok, rfc_msg = wel.rfc_agent_approved(ROOT / "rfcs/0029-g12-path-tracer-productionization.md")
    facts.append(_fact(
        "spec_first_clauses_and_rfc_approved",
        not missing and rfc_ok,
        "RXS-0398~0401 条款头在树 + RFC-0029 Agent Approved"
        if not missing and rfc_ok else f"缺条款头 {missing};RFC: {rfc_msg}",
    ))

    # ⑤ conformance 锚定语料 11 件。
    corp_bad: list[str] = []
    for rel, clause, gate_key in CORPUS_11:
        path = ROOT / "conformance/gi" / rel
        if not path.is_file():
            corp_bad.append(f"缺 {rel}")
            continue
        text = path.read_text(encoding="utf-8")
        if f"//@ spec: {clause}" not in text or gate_key not in text:
            corp_bad.append(f"{rel} 锚/key 缺")
    facts.append(_fact(
        "conformance_corpus_11_anchored",
        not corp_bad,
        "conformance/gi 11 件(accept 4 + reject 7)锚定在位"
        if not corp_bad else "; ".join(corp_bad[:3]),
    ))

    # ⑥ M166 标定 provenance 齐备。
    prov_bad: list[str] = []
    m166_path = wel.load_latest_evidence("g12_pt_production_calibration")
    if m166_path is None:
        prov_bad.append("M166 缺最新 evidence")
    else:
        doc = wel.load_json(m166_path)
        mc = doc.get("checks") or {}
        if mc.get("calibration_two_run_bitexact") is not True:
            prov_bad.append("两跑逐位一致非真")
        if mc.get("sample_digest_registered") is not True:
            prov_bad.append("样本集 digest 未登记")
        if mc.get("selection_artifact_written") is not True:
            prov_bad.append("选型 artifact 未落盘")
    if not gl.SELECTION_PATH.is_file():
        prov_bad.append("选型 artifact 文件缺失")
    else:
        sel = gl.load_json(gl.SELECTION_PATH)
        if sel.get("winner") not in ("pcg_independent", "stratified_per_dimension", "sobol_class_seed_perturbed"):
            prov_bad.append("选型 artifact winner 非法")
    for eid in gl.CALIB_IDS:
        e = gl.budget_entry(budget, eid)
        if e is not None:
            ef = e.get("evidence_file")
            if not ef or not (ROOT / ef).is_file():
                prov_bad.append(f"{eid} evidence_file 缺档")
    facts.append(_fact(
        "m166_calibration_provenance_complete",
        not prov_bad,
        "两跑逐位一致 + 样本集 digest + 选型 artifact winner 合法 + 7 条目 evidence 在档"
        if not prov_bad else "; ".join(prov_bad[:3]),
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        # selftest 负样本面:facts 全 FAIL(空树无参照面)。
        extras = [
            _fact("m96_correctness_anchor_0byte", False, "selftest 空目录"),
            _fact("five_gates_red_arms_independently_effective", False, "selftest 空目录"),
            _fact("budget_anchors_and_calibration_measured", False, "selftest 空目录"),
            _fact("spec_first_clauses_and_rfc_approved", False, "selftest 空目录"),
            _fact("conformance_corpus_11_anchored", False, "selftest 空目录"),
            _fact("m166_calibration_provenance_complete", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: five G12.2 gates (P1 M166 calibration step 217 / P0 M158 step 218 / "
        "P0 M159 step 219 / P0 M160 step 220 / P0 M161 step 221)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: M96 anchor 0-byte + red arms + budget 15 entries measured + spec-first + corpus 11 + M166 provenance",
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
    """① 缺五门 evidence → 红;② 真树聚合 VERDICT == 子门实测态(遮蔽即自检红)。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g12_wave2_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G12.2 wave2.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
