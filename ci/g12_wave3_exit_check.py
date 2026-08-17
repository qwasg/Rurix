#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.3 降噪波）
"""G12.3 波次聚合门 g12.wave.3.exit（步骤 224;milestones/g12/CI_GATES.md §5;
G12_CONTRACT G-G12-5;同构 ci/g12_wave2_exit_check.py + ci/g12_wave_exit_lib.py）。

只读汇总 G12.3 波唯一 P0 门最新 evidence——M162 降噪管线 + TSR 联动
(步骤 223)——+ 六 facts:
① M96 正确性锚 0-byte(g9.p0.m96 最新 evidence PASS + 冻结带/参照器面
   git diff 闭集机核);
② temporal 底座 0-byte(src/rurix-render/src/temporal/ vs G12.0 不可变
   ref 目录级 diff 空 + 工作树零未提交面——底座接线即 RED 的聚合复核面);
③ M162 RED 臂独立有效(最新 evidence red 面 checks 非空且全真);
④ g12_budget 锚+标定 17 条目齐备 measured_local 零 estimated(G12.2 十五
   + 降噪二) + budget_eval 全 PASS(P-09 禁手写);
⑤ spec-first 面(RXS-0402 条款头在树 + RFC-0029 Agent Approved 字面 +
   conformance 锚定语料 3 件〔accept 1 + reject 2〕);
⑥ NRD 评估报告落盘 + 评估不接线(报告必备章节齐备 + src//Cargo.toml 零
   vendor 接线符号 + deferred.json RD-040 status=open 维持且
   backfill_condition nrd 分项字面在树 0-byte)。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g12_wave3_exit_check.py --gate g12.wave.3.exit
  py -3 ci/g12_wave3_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g12_pt_prod_lib as gl  # noqa: E402
import g12_wave_exit_lib as wel  # noqa: E402
import g12_denoise_pipeline_tsr_smoke as m162  # noqa: E402

GATE_KEY = "g12.wave.3.exit"
NUMERIC_STEP = 224
SUBJECT = "g12_wave3_exit"
WAVE = "G12.3"
SOURCE_REF = (
    "milestones/g12/CI_GATES.md §5;G12_CONTRACT G-G12-5;G12_ACCEPTANCE_MAP §1;"
    "M162 gate red arms independently effective;M96 correctness anchor 0-byte;"
    "temporal base 0-byte;g12_budget 17 anchors/entries measured_local;"
    "spec-first RXS-0402 + RFC-0029 approved;conformance 3 corpus;"
    "NRD evaluation report present + no vendor wiring + RD-040 open"
)
SCHEMA_PATH = ROOT / "milestones/g12/g12_wave3_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g12.p0.m162.denoise_pipeline_tsr", "g12_m162_denoise_pipeline_tsr"),
]

CORPUS_3 = [
    ("accept/denoise_pipeline_minimal.rx", "RXS-0402", "g12.p0.m162.denoise_pipeline_tsr"),
    ("reject/denoise_energy_bias.rx", "RXS-0402", "g12.p0.m162.denoise_pipeline_tsr"),
    ("reject/temporal_base_rewire.rx", "RXS-0402", "g12.p0.m162.denoise_pipeline_tsr"),
]

DENOISE_BUDGET_IDS = [
    "g12.pt.denoise_hf_drop_min",
    "g12.pt.denoise_mean_energy_tol",
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

    # ② temporal 底座 0-byte(目录级 diff + 工作树双面机核)。
    t_ok, t_msg = m162.temporal_base_0byte()
    facts.append(_fact(
        "temporal_base_0byte",
        t_ok,
        t_msg,
    ))

    # ③ M162 RED 臂独立有效(red 面 checks 非空全真)。
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
        "m162_red_arms_independently_effective",
        not red_bad,
        f"M162 最新 evidence red 面 checks 全真(共 {red_total} 臂独立有效)"
        if not red_bad else "; ".join(red_bad[:3]),
    ))

    # ④ g12_budget 17 条目齐备 + budget_eval 全 PASS。
    bud_bad: list[str] = []
    budget = gl.load_budget()
    for eid in gl.ANCHOR_IDS + gl.CALIB_IDS + DENOISE_BUDGET_IDS:
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
        "8 锚 + 7 标定 + 2 降噪标定条目齐备 measured_local 零 estimated + budget_eval 全 PASS(P-09)"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ⑤ spec-first 面(条款头 + RFC Approved + 语料 3 件)。
    spec_text = (ROOT / "spec/global_illumination.md").read_text(encoding="utf-8")
    clause_ok = "### RXS-0402 " in spec_text
    rfc_ok, rfc_msg = wel.rfc_agent_approved(ROOT / "rfcs/0029-g12-path-tracer-productionization.md")
    corp_bad: list[str] = []
    for rel, clause, gate_key in CORPUS_3:
        path = ROOT / "conformance/gi" / rel
        if not path.is_file():
            corp_bad.append(f"缺 {rel}")
            continue
        text = path.read_text(encoding="utf-8")
        if f"//@ spec: {clause}" not in text or gate_key not in text:
            corp_bad.append(f"{rel} 锚/key 缺")
    facts.append(_fact(
        "spec_first_clause_and_rfc_and_corpus",
        clause_ok and rfc_ok and not corp_bad,
        "RXS-0402 条款头在树 + RFC-0029 Agent Approved + conformance 3 件锚定在位"
        if clause_ok and rfc_ok and not corp_bad
        else f"条款头={clause_ok};RFC: {rfc_msg};语料: {'; '.join(corp_bad[:2])}",
    ))

    # ⑥ NRD 评估报告落盘 + 评估不接线 + RD-040 open 维持。
    rep_ok, rep_msg = m162.nrd_report_ok()
    wire_ok, wire_msg = m162.no_vendor_wiring()
    rd_ok = False
    rd_msg = "deferred.json 不可解析"
    try:
        deferred = wel.load_json(ROOT / "registry/deferred.json")
        for e in deferred.get("entries", []):
            if e.get("id") == "RD-040":
                bf = e.get("backfill_condition", "")
                rd_ok = e.get("status") == "open" and "NRD" in bf and "UpscaleBackend" in bf
                rd_msg = f"RD-040 status={e.get('status')} backfill nrd 分项字面在树={rd_ok}"
                break
    except Exception as e:  # noqa: BLE001
        rd_msg = f"deferred.json 解析异常: {e}"
    facts.append(_fact(
        "nrd_evaluation_present_no_wiring_rd040_open",
        rep_ok and wire_ok and rd_ok,
        f"报告: {rep_msg};接线: {wire_msg};{rd_msg}"
        if rep_ok and wire_ok and rd_ok
        else f"报告: {rep_msg};接线: {wire_msg};{rd_msg}",
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        # selftest 负样本面:facts 全 FAIL(空树无参照面)。
        extras = [
            _fact("m96_correctness_anchor_0byte", False, "selftest 空目录"),
            _fact("temporal_base_0byte", False, "selftest 空目录"),
            _fact("m162_red_arms_independently_effective", False, "selftest 空目录"),
            _fact("budget_anchors_and_calibration_measured", False, "selftest 空目录"),
            _fact("spec_first_clause_and_rfc_and_corpus", False, "selftest 空目录"),
            _fact("nrd_evaluation_present_no_wiring_rd040_open", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: one G12.3 P0 gate (M162 denoise pipeline + TSR, step 223)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: M96 anchor 0-byte + temporal base 0-byte + red arms + budget 17 entries measured + spec-first + NRD evaluation no-wiring",
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
    """① 缺 M162 evidence → 红;② 真树聚合 VERDICT == 子门实测态(遮蔽即自检红)。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g12_wave3_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G12.3 wave3.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
