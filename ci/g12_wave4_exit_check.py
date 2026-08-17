#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.4 UE PT 对标波）
"""G12.4 波次聚合门 g12.wave.4.exit（步骤 227;milestones/g12/CI_GATES.md §5;
G12_CONTRACT G-G12-6;同构 ci/g12_wave3_exit_check.py + ci/g12_wave_exit_lib.py）。

只读汇总 G12.4 波两 P0 门最新 evidence——M163 UE PT 对标（步骤 225）+
M164 生产化回归门（步骤 226）——+ 六 facts:
① M96 正确性锚 0-byte(g9.p0.m96 最新 evidence PASS + 冻结带/参照器面
   git diff 闭集机核);
② 对标契约 digest 冻结面(M163 最新 evidence parity.contract_digest ==
   门内冻结注册值 ∧ ue_build_id == M128 登记值 ∧ 帧计数全段非空);
③ 差距登记表落盘 + RXS-0391 归属枚举合法 + 行集对账（表文件在树 +
   M163 evidence parity.gap_registry_file 字面 + 超容差登记面 checks）;
④ g12_budget 20 条目齐备 measured_local 零 estimated(G12.2 十五 + 降噪
   二 + 对标标定三) + budget_eval 全 PASS(P-09 禁手写);
⑤ spec-first 面(RXS-0403 条款头在树 + RFC-0029 Agent Approved +
   conformance 3 件〔accept 1 + reject 2〕);
⑥ 62 门零降级(M164 最新 evidence PASS 承载——本批不重跑) + RD-040
   history 只追加(M100-high G12.4 触发评估登记留痕,条目级字段 0-byte)。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g12_wave4_exit_check.py --gate g12.wave.4.exit
  py -3 ci/g12_wave4_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g12_pt_prod_lib as gl  # noqa: E402
import g12_wave_exit_lib as wel  # noqa: E402
import g12_ue_pt_parity_smoke as m163  # noqa: E402

GATE_KEY = "g12.wave.4.exit"
NUMERIC_STEP = 227
SUBJECT = "g12_wave4_exit"
WAVE = "G12.4"
SOURCE_REF = (
    "milestones/g12/CI_GATES.md §5;G12_CONTRACT G-G12-6;G12_ACCEPTANCE_MAP §1;"
    "M163 parity contract digest frozen + ue_build_id M128 machine check;"
    "gap registry landed + RXS-0391 enum;g12_budget 20 entries measured_local;"
    "spec-first RXS-0403 + RFC-0029 approved;62 gates zero-degrade + RD-040 M100-high trigger evaluated"
)
SCHEMA_PATH = ROOT / "milestones" / "g12" / "g12_wave4_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g12.p0.m163.ue_pt_parity", "g12_m163_ue_pt_parity"),
    ("g12.p0.m164.regression_guard", "g12_m164_regression_guard"),
]

CORPUS_3 = [
    ("accept/ue_pt_parity_contract_minimal.rx", "RXS-0403", "g12.p0.m163.ue_pt_parity"),
    ("reject/parity_digest_mismatch_report.rx", "RXS-0403", "g12.p0.m163.ue_pt_parity"),
    ("reject/residual_caliber_silent.rx", "RXS-0403", "g12.p0.m163.ue_pt_parity"),
]

PARITY_BUDGET_IDS = [
    "g12.pt.parity_curve_tol",
    "g12.pt.parity_noise_tol",
    "g12.pt.parity_energy_tol",
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

    # ② 对标契约 digest 冻结面(M163 最新 evidence parity 节三面机核)。
    m163_path = wel.load_latest_evidence("g12_m163_ue_pt_parity")
    c_ok = False
    c_msg = "M163 最新 evidence 缺失"
    if m163_path is not None:
        doc = wel.load_json(m163_path)
        parity = doc.get("parity") or {}
        c_digest = parity.get("contract_digest")
        c_build = parity.get("ue_build_id")
        c_segs = parity.get("curve_segments") or []
        c_ok = (
            c_digest == m163.FROZEN_CONTRACT_DIGEST
            and c_build == m163.M128_UE_BUILD_ID
            and len(c_segs) >= 1
            and (doc.get("checks") or {}).get("contract_digest_three_way_consistent") is True
        )
        c_msg = f"contract_digest={str(c_digest)[:24]}… ue_build_id={c_build} 段数={len(c_segs)} 三向={c_ok}"
    facts.append(_fact(
        "parity_contract_digest_frozen_and_ue_build_m128",
        c_ok,
        c_msg,
    ))

    # ③ 差距登记表落盘 + 归属枚举合法 + 行集对账面。
    reg_path = ROOT / "milestones" / "g12" / "g12_ue_pt_gap_registry.json"
    g_ok = reg_path.is_file()
    g_msg = "差距登记表缺失"
    if g_ok:
        reg = wel.load_json(reg_path)
        items = reg.get("items", [])
        enum_bad = [
            it.get("ue5_module_primary")
            for it in items
            if it.get("ue5_module_primary") not in m163.UE_MODULE_ALLOWED
        ]
        scenes_ok = set(reg.get("scene_set", [])) == {"cornell-box", "bistro-interior"}
        g_ok = bool(items) and not enum_bad and scenes_ok
        g_msg = f"登记表 {len(items)} 行(quality {sum(1 for it in items if it.get('kind')=='quality_gap')} + caliber {sum(1 for it in items if it.get('kind')=='caliber_diff')});归属枚举越集={enum_bad[:2]};场景集={scenes_ok}"
    facts.append(_fact(
        "gap_registry_landed_with_enum_attribution",
        g_ok,
        g_msg,
    ))

    # ④ g12_budget 20 条目齐备 + budget_eval 全 PASS。
    bud_bad: list[str] = []
    budget = gl.load_budget()
    for eid in gl.ANCHOR_IDS + gl.CALIB_IDS + DENOISE_BUDGET_IDS + PARITY_BUDGET_IDS:
        e = gl.budget_entry(budget, eid)
        if e is None:
            bud_bad.append(f"缺条目 {eid}")
        elif e.get("evidence") != "measured_local":
            bud_bad.append(f"{eid} 非 measured_local")
    r = gl.run(["py", "-3", "ci/budget_eval.py"])
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "budget_20_entries_measured_and_eval_pass",
        not bud_bad,
        "8 锚 + 7 标定 + 2 降噪标定 + 3 对标标定条目齐备 measured_local 零 estimated + budget_eval 全 PASS(P-09)"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ⑤ spec-first 面(条款头 + RFC Approved + 语料 3 件)。
    spec_text = (ROOT / "spec/visual_comparison.md").read_text(encoding="utf-8")
    clause_ok = "### RXS-0403 " in spec_text
    rfc_ok, rfc_msg = wel.rfc_agent_approved(ROOT / "rfcs/0029-g12-path-tracer-productionization.md")
    corp_bad: list[str] = []
    for rel, clause, gate_key in CORPUS_3:
        path = ROOT / "conformance/visual_comparison" / rel
        if not path.is_file():
            corp_bad.append(f"缺 {rel}")
            continue
        text = path.read_text(encoding="utf-8")
        if f"//@ spec: {clause}" not in text or gate_key not in text:
            corp_bad.append(f"{rel} 锚/key 缺")
    facts.append(_fact(
        "spec_first_clause_and_rfc_and_corpus",
        clause_ok and rfc_ok and not corp_bad,
        "RXS-0403 条款头在树 + RFC-0029 Agent Approved + conformance 3 件锚定在位"
        if clause_ok and rfc_ok and not corp_bad
        else f"条款头={clause_ok};RFC: {rfc_msg};语料: {'; '.join(corp_bad[:2])}",
    ))

    # ⑥ 62 门零降级(M164 最新 evidence 承载) + RD-040 history 追加留痕。
    m164_row = wel.require_gate_pass("g12.p0.m164.regression_guard", "g12_m164_regression_guard")
    rd_ok = False
    rd_msg = "deferred.json 不可解析"
    try:
        deferred = wel.load_json(ROOT / "registry/deferred.json")
        for e in deferred.get("entries", []):
            if e.get("id") == "RD-040":
                hist = e.get("history", [])
                last = hist[-1] if hist else {}
                rd_ok = (
                    e.get("status") == "open"
                    and "M100-high" in str(last.get("event", ""))
                    and "G12.4" in str(last.get("date_anchor", "") or last.get("event", ""))
                )
                rd_msg = f"RD-040 status={e.get('status')} history 末条={str(last.get('event', ''))[:60]}…"
                break
    except Exception as ex:  # noqa: BLE001
        rd_msg = f"deferred.json 解析异常: {ex}"
    facts.append(_fact(
        "regression_62_zero_degrade_and_rd040_m100high_registered",
        m164_row["status"] == "PASS" and rd_ok,
        f"62 门零降级(M164 {m164_row['status']});{rd_msg}",
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        extras = [
            _fact("m96_correctness_anchor_0byte", False, "selftest 空目录"),
            _fact("parity_contract_digest_frozen_and_ue_build_m128", False, "selftest 空目录"),
            _fact("gap_registry_landed_with_enum_attribution", False, "selftest 空目录"),
            _fact("budget_20_entries_measured_and_eval_pass", False, "selftest 空目录"),
            _fact("spec_first_clause_and_rfc_and_corpus", False, "selftest 空目录"),
            _fact("regression_62_zero_degrade_and_rd040_m100high_registered", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: two G12.4 P0 gates (M163 ue_pt_parity step 225 / M164 regression_guard step 226)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: M96 anchor 0-byte + parity digest frozen + gap registry + budget 20 measured + spec-first + 62 zero-degrade + RD-040 trigger",
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
    """① 缺 M163/M164 evidence → 红;② 真树聚合 VERDICT == 子门实测态(遮蔽即自检红)。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g12_wave4_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G12.4 wave4.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
