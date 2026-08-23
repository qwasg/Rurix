#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.3 修复闭环波）
"""G15.3 波次聚合门 g15.wave.3.exit（步骤 272；G15_CONTRACT G-G15-4/§2 G15.3；
G15_ACCEPTANCE_MAP §1；同构 ci/g15_wave2_exit_check.py）。

只读汇总 G15.3 波 M-b 门（g15.p0.m_b.gap_fix_closure_loop，步骤 271——measured
主差修复闭环：处置表 20 行逐行终态处置三态零空行 + 修复项 RED 先行 + 触冻结面
独立 Full RFC 留痕 + 材质链表达面立项评估结论登记 + G15-MA-F1 评估定论）最新
evidence + 六 facts:
① M-b 门 fresh PASS + RED 臂独立有效（最新 evidence red 面 checks 非空且全真，
   ≥4 臂——本门五臂）;
② 闭环登记表 20 行零空行重算绿（gap_id 闭集与 M-a 处置表逐字对账 + 三态逐态
   义务字面 + 汇总 tally 重算一致——经 M-b 门同族校验器函数面消费）;
③ 三 parity 契约 + 三冻结登记表终态 0-byte（在树 == HEAD 提交态逐字节 git
   机核；M-a 处置表只消费不回写——结构有效面归 M-b 门本体）;
④ 材质链表达面立项评估结论 + G15-MA-F1 定论登记（not-triggered 未命中/
   closed-caliber-registered 字面；triggered/fix-project 须 Full RFC Agent
   Approved 面——本波未触发如实登记不充绿）;
⑤ g15_budget 五条目齐备 measured_local 零 estimated + budget_eval 全 PASS
   （P-09 禁手写；本波零修复零追加维持字面）;
⑥ G5~G14 closed 面 0-byte（vs G15.0 不可变 ref f061487efaf7816684de18a6ef86554e5c392a75
   committed diff 闭集 ⊆ G14 战后归档授权面 {milestones/g14/g14_budget.json,
   milestones/g14/g14_ue_variance_samples.json}；工作树闭集 ⊆
   {milestones/g14/g14_ue_variance_samples.json} 样本只追加面）+ RFC 命名空间
   0-byte（ledger next_free=31 维持——触冻结面零发生）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g15_wave3_exit_check.py --gate g15.wave.3.exit
  py -3 ci/g15_wave3_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402
import g15_dual_end_quality_reharvest_smoke as ma  # noqa: E402
import g15_gap_fix_closure_smoke as mb  # noqa: E402

GATE_KEY = "g15.wave.3.exit"
NUMERIC_STEP = 272  # 落盘前实测 registry/number_ledger.json CI_step.next_free=272 顺位领取
SUBJECT = "g15_wave3_exit"
WAVE = "G15.3"
SOURCE_REF = (
    "G15_CONTRACT G-G15-4/§2 G15.3;G15_ACCEPTANCE_MAP §1;M-b gate red arms independently effective;"
    "closure registry 20 rows zero-empty recompute;three contracts and frozen registries 0-byte;"
    "material chain and G15-MA-F1 verdicts registered;g15_budget entries measured_local maintained;"
    "G5~G14 closed 0-byte closed-set diff vs G15.0 ref + RFC namespace 0-byte"
)
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_wave3_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g15.p0.m_b.gap_fix_closure_loop", "g15_m_b_gap_fix_closure_loop"),
]

G15_0_REF = "f061487efaf7816684de18a6ef86554e5c392a75"  # G15.0 不可变 ref（G14 close-out flip commit，tag g14-closed）
# G15.0→G15.3 期 G5~G14 closed 面允许 diff 闭集（34f96ac3 G14 战后归档授权面在案）。
ALLOWED_CLOSED_DIFF = {
    "milestones/g14/g14_budget.json",
    "milestones/g14/g14_ue_variance_samples.json",
}
# 工作树允许面 = G14.5a 加性样本级联只追加面（G13 双门复跑门产追加，0-byte 回写禁）。
WORKING_ALLOWED = {
    "milestones/g14/g14_ue_variance_samples.json",
}
FROZEN_FILES = [
    "milestones/g13/g13_ue_upscale_parity_contract.json",
    "milestones/g13/g13_ue_lumen_gi_parity_contract.json",
    "milestones/g12/g12_ue_pt_parity_contract.json",
    "milestones/g13/g13_ue_upscale_gap_registry.json",
    "milestones/g13/g13_ue_lumen_gap_registry.json",
    "milestones/g12/g12_ue_pt_gap_registry.json",
]
G15_BUDGET_IDS = {
    "g15.quality_guard.g14_anchor_ssim_deficit_band",
    "g15.quality_guard.ue_variance_band_upscale_probe_rel",
    "g15.quality_guard.ue_variance_band_lumen_probe_rel",
    "g15.m_a.ue_variance_band_upscale_probe_rel",
    "g15.m_a.ue_variance_band_lumen_probe_rel",
}


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _git(*args: str) -> str:
    r = subprocess.run(["git"] + list(args), cwd=ROOT, capture_output=True, text=True)
    return r.stdout or ""


def collect_facts() -> list[dict]:
    facts: list[dict] = []

    # ① M-b 门 fresh PASS + RED 臂独立有效（red 面 checks 非空全真，≥4 臂）。
    b_path = wel.load_latest_evidence("g15_m_b_gap_fix_closure_loop")
    b_doc = wel.load_json(b_path) if b_path else {}
    b_row = wel.require_gate_pass(*REQUIRED_GATES[0])
    red_checks = {k: v for k, v in (b_doc.get("checks") or {}).items() if k.startswith("red_arm_")}
    red_ok = bool(red_checks) and len(red_checks) >= 4 and all(v is True for v in red_checks.values())
    facts.append(_fact(
        "m_b_gate_pass_red_arms_effective",
        b_row["status"] == "PASS" and red_ok,
        f"M-b 最新 evidence PASS + red 面 checks 全真（{len(red_checks)} 臂独立有效）"
        if b_row["status"] == "PASS" and red_ok
        else f"M-b 行: {b_row['detail']}；red 面臂数/真值异常",
    ))

    # ② 闭环登记表 20 行零空行重算绿（M-b 门同族校验器函数面消费）。
    reg_bad: list[str] = []
    if not mb.CLOSURE_PATH.is_file():
        reg_bad.append("g15_gap_fix_closure_registry.json 缺失")
    elif not ma.DISPOSITION_PATH.is_file():
        reg_bad.append("g15_quality_gap_disposition.json 缺失")
    else:
        try:
            closure_doc = wel.load_json(mb.CLOSURE_PATH)
            disp_doc = wel.load_json(ma.DISPOSITION_PATH)
            verrs = mb.validate_closure(closure_doc, disp_doc.get("items") or [])
            if verrs:
                reg_bad += verrs[:2]
        except (OSError, json.JSONDecodeError) as e:
            reg_bad.append(f"登记表/处置表不可读: {e}")
    facts.append(_fact(
        "closure_registry_20_rows_revalidate_green",
        not reg_bad,
        "闭环登记表 20 行零空行 + gap_id 闭集逐字对账 + 三态逐态义务 + 汇总 tally 重算全绿"
        if not reg_bad else "; ".join(reg_bad[:3]),
    ))

    # ③ 三 parity 契约 + 三冻结表 0-byte（在树 == HEAD 提交态逐字节）。
    zero_bad: list[str] = []
    for rel in FROZEN_FILES:
        p = ROOT / rel
        if not p.is_file():
            zero_bad.append(f"{rel} 缺失")
            continue
        committed = _git("show", f"HEAD:{rel}")
        if committed.replace("\r\n", "\n") != p.read_text(encoding="utf-8").replace("\r\n", "\n"):
            zero_bad.append(f"{rel} 在树 ≠ HEAD 提交态")
    facts.append(_fact(
        "frozen_contracts_and_registries_0byte",
        not zero_bad,
        "三 parity 契约 + 三冻结登记表在树 == HEAD 逐字节（8+2+10 行终态 0-byte 只消费不回写）"
        if not zero_bad else "; ".join(zero_bad[:3]),
    ))

    # ④ 材质链评估 + G15-MA-F1 定论登记（triggered/fix-project 须 RFC Approved 面）。
    v_bad: list[str] = []
    if not mb.CLOSURE_PATH.is_file():
        v_bad.append("闭环登记表缺失")
    else:
        closure_doc = wel.load_json(mb.CLOSURE_PATH)
        mc = closure_doc.get("material_chain_assessment") or {}
        mc_verdict = mc.get("verdict")
        if mc_verdict == "not-triggered":
            if "未命中" not in str(mc.get("verdict_verbatim") or "") or mc.get("full_rfc_required") is not False:
                v_bad.append("材质链 not-triggered 字面/Full RFC 面异常")
        elif mc_verdict == "triggered":
            v_bad.append("材质链 triggered——须 Full RFC Agent Approved 留痕（本波未触发为异常面）")
        else:
            v_bad.append(f"材质链 verdict 闭集外: {mc_verdict!r}")
        f1 = next((f for f in (closure_doc.get("findings_adjudication") or [])
                   if isinstance(f, dict) and f.get("id") == "G15-MA-F1"), None)
        if f1 is None:
            v_bad.append("G15-MA-F1 定论缺行")
        elif f1.get("verdict") == "fix-project":
            v_bad.append("G15-MA-F1 判 fix-project——须 RED 先行 + digest 锚重收割同型程序留痕（本波未触发为异常面）")
        elif f1.get("verdict") not in ("closed-caliber-registered", "open-defer-G16+"):
            v_bad.append(f"G15-MA-F1 verdict 闭集外: {f1.get('verdict')!r}")
    facts.append(_fact(
        "material_chain_and_f1_verdicts_registered",
        not v_bad,
        "材质链评估 not-triggered 未命中 + G15-MA-F1 closed-caliber-registered 定论登记（未触发如实登记不充绿）"
        if not v_bad else "; ".join(v_bad[:3]),
    ))

    # ⑤ g15_budget 五条目齐备 measured_local + budget_eval 全 PASS（本波零追加维持）。
    bud_bad: list[str] = []
    if not ma.BUDGET_PATH.is_file():
        bud_bad.append("g15_budget.json 缺失")
    else:
        budget = wel.load_json(ma.BUDGET_PATH)
        got = {e.get("id"): e for e in (budget.get("entries") or [])}
        for eid in sorted(G15_BUDGET_IDS):
            e = got.get(eid)
            if e is None:
                bud_bad.append(f"缺条目 {eid}")
            elif e.get("evidence") != "measured_local":
                bud_bad.append(f"{eid} 非 measured_local")
        if len(budget.get("entries") or []) != 5:
            bud_bad.append("g15_budget 条目数 ≠ 5（本波零修复零追加维持字面）")
    r = subprocess.run([sys.executable, str(ROOT / "ci" / "budget_eval.py")], cwd=ROOT,
                       capture_output=True, text=True)
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "budget_entries_measured",
        not bud_bad,
        "g15_budget 五条目齐备 measured_local 零 estimated + budget_eval 全 PASS（P-09；本波零追加维持）"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ⑥ G5~G14 closed 面 0-byte + RFC 命名空间 0-byte（next_free=31 维持）。
    globs = [
        "ci/g5_*.py", "ci/g6_*.py", "ci/g7_*.py", "ci/g8_*.py", "ci/g9_*.py",
        "ci/g10_*.py", "ci/g11_*.py", "ci/g12_*.py", "ci/g13_*.py", "ci/g14_*.py",
        "milestones/g5", "milestones/g6", "milestones/g7", "milestones/g8",
        "milestones/g9", "milestones/g10", "milestones/g11", "milestones/g12",
        "milestones/g13", "milestones/g14",
    ]
    diff = _git("diff", "--name-only", f"{G15_0_REF}..HEAD", "--", *globs)
    committed = sorted(x for x in diff.splitlines() if x.strip())
    porc = _git("status", "--porcelain", "--", *globs)
    working = sorted(ln[3:].strip() for ln in porc.splitlines() if ln.strip())
    bad_committed = [f for f in committed if f not in ALLOWED_CLOSED_DIFF]
    bad_working = [f for f in working if f not in WORKING_ALLOWED]
    ledger = wel.load_json(ROOT / "registry" / "number_ledger.json")
    rfc_next_free = ((ledger.get("namespaces") or {}).get("RFC") or {}).get("next_free")
    rfc_ok = rfc_next_free == 31
    ok6 = not bad_committed and not bad_working and rfc_ok
    facts.append(_fact(
        "legacy_closed_and_rfc_0byte",
        ok6,
        f"committed 闭集={committed or '空'}（允许面={sorted(ALLOWED_CLOSED_DIFF)}）；工作树闭集={working or '空'}；RFC next_free={rfc_next_free}（=31 维持）"
        if ok6 else f"越界 committed={bad_committed} working={bad_working} rfc_next_free={rfc_next_free!r}",
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        extras = [
            _fact("m_b_gate_pass_red_arms_effective", False, "selftest 空目录"),
            _fact("closure_registry_20_rows_revalidate_green", False, "selftest 空目录"),
            _fact("frozen_contracts_and_registries_0byte", False, "selftest 空目录"),
            _fact("material_chain_and_f1_verdicts_registered", False, "selftest 空目录"),
            _fact("budget_entries_measured", False, "selftest 空目录"),
            _fact("legacy_closed_and_rfc_0byte", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G15.3 M-b gap fix closure loop (step 271) — 20-row final dispositions three-state zero-empty + zero fix projects (evaluation-complete legal exit) + material chain not-triggered + G15-MA-F1 closed-caliber-registered",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: M-b PASS + red arms + closure registry revalidate + frozen 0-byte + verdicts registered + g15_budget measured + legacy/RFC 0-byte",
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
    """① 缺 M-b evidence → 红;② 真树聚合 VERDICT == 子门实测态（遮蔽即自检红）。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g15_wave3_selftest_") as td:
        code = run_gate(evidence_dir=Path(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 真树一致性:聚合 VERDICT == 子门实测态（不遮蔽机核）")
    rows = [wel.require_gate_pass(key, prefix) for key, prefix in REQUIRED_GATES]
    extras = collect_facts()
    expected_pass = all(r["status"] == "PASS" for r in rows) and all(f["status"] == "PASS" for f in extras)
    code = run_gate(evidence_dir=None)
    if (code == 0) != expected_pass:
        print(
            f"[selftest] FAIL: 聚合 VERDICT 与子门实测态不一致——expected_pass={expected_pass} exit={code}",
            file=sys.stderr,
        )
        return 1
    print(f"[selftest] PASS: 真树聚合 VERDICT={'PASS' if code == 0 else 'FAIL'} == 子门实测态（不遮蔽）")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G15.3 wave3.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
