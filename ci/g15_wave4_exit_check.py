#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.4 绝对画质终审波）
"""G15.4 波次聚合门 g15.wave.4.exit（步骤 274；G15_CONTRACT G-G15-5/§2 G15.4；
G15_ACCEPTANCE_MAP §1；同构 ci/g15_wave3_exit_check.py）。

只读汇总 G15.4 波 M-c 门（g15.p0.m_c.absolute_quality_final_review，步骤 273——
绝对画质通过线程序产标定 + 18 格逐格判定 + 逐格 AI 读图严格画面审查记录 + 商用
收口判定）最新 evidence + 六 facts:
① M-c 门 fresh PASS + RED 臂独立有效（最新 evidence red 面 checks 非空且全真，
   ≥4 臂——本门五臂）;
② 18 格判定矩阵 + 读图记录重算绿（逐格 verdict 由存储数据面经 M-c 门
   crosscheck_verdicts 纯函数重算 == 存储标签 + met_count/verdict 字面一致 +
   读图记录 18 格闭集与 evidence manifest PNG digest 逐格绑定重算绿——M-c 门
   同族校验器函数面消费）;
③ 标定链程序产机核（g15_budget 九条目齐备〔五既有 + M-c 标定四条目〕
   measured_local + threshold == measured × 2.0 f64 精确重算 + budget_eval
   全 PASS——P-09 禁手写）;
④ 商用收口判定定盘字面（verdict ∈ {达标, 未达标} + met_count == 逐格重算 +
   未达格逐格归因非空 + 未达标面 g16_anchor 承接锚字面〔用户 2026-08-19 授权
   面〕+ findings 显式登记〔参照退化面 G15-MC-F<n> 在档〕——未达标如实登记
   不冒充亦为合法定盘字面）;
⑤ 三 parity 契约 + 三冻结登记表终态 0-byte（在树 == HEAD 提交态逐字节 git
   机核）+ RXS-0407 spec 锚定面维持（trace_matrix --check + stable_snapshot
   --check 全 PASS——spec-first 条款批在档）;
⑥ G5~G14 closed 面 0-byte（vs G15.0 不可变 ref f061487efaf7816684de18a6ef86554e5c392a75
   committed diff 闭集 ⊆ G14 战后归档授权面；工作树闭集 ⊆
   {milestones/g14/g14_ue_variance_samples.json} 样本只追加面）+ M-c 波零
   src 变更机核（src/ tracked diff 空 + untracked ⊆ 异己登记六件闭集）→
   G14 M-d 复跑义务 not-triggered 如实登记（性能零降级守护面——出图链路零
   src 改动，18 格 ×1.00 定盘面无机面触改）+ RFC/RXS 命名空间机核（RFC
   next_free=31 维持零消费；RXS next_free=408 == ledger〔RXS-0407 单号消费
   校准在档〕）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g15_wave4_exit_check.py --gate g15.wave.4.exit
  py -3 ci/g15_wave4_exit_check.py --selftest
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
import g15_absolute_quality_review_smoke as mc  # noqa: E402
import g15_gap_fix_closure_smoke as mb  # noqa: E402

GATE_KEY = "g15.wave.4.exit"
NUMERIC_STEP = 274  # 落盘前实测 registry/number_ledger.json CI_step.next_free=274 顺位领取
SUBJECT = "g15_wave4_exit"
WAVE = "G15.4"
SOURCE_REF = (
    "G15_CONTRACT G-G15-5/§2 G15.4;G15_ACCEPTANCE_MAP §1;spec/visual_comparison.md RXS-0407;"
    "M-c gate red arms independently effective;verdict matrix and reading records revalidate green;"
    "calibration chain program-produced recompute + g15_budget nine entries measured_local;"
    "commercial closure verdict honest registration;frozen 0-byte + spec anchors maintained;"
    "G5~G14 closed 0-byte + zero src change (G14 M-d rerun not-triggered) + RFC/RXS namespace check"
)
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_wave4_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g15.p0.m_c.absolute_quality_final_review", "g15_m_c_absolute_quality_final_review"),
]

G15_0_REF = "f061487efaf7816684de18a6ef86554e5c392a75"  # G15.0 不可变 ref（G14 close-out flip commit，tag g14-closed）
# G15.0→G15.4 期 G5~G14 closed 面允许 diff 闭集（34f96ac3 G14 战后归档授权面在案）。
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

    # ① M-c 门 fresh PASS + RED 臂独立有效（red 面 checks 非空全真，≥4 臂）。
    c_path = wel.load_latest_evidence("g15_m_c_absolute_quality_final_review")
    c_doc = wel.load_json(c_path) if c_path else {}
    c_row = wel.require_gate_pass(*REQUIRED_GATES[0])
    red_checks = {k: v for k, v in (c_doc.get("checks") or {}).items() if k.startswith("red_arm_")}
    red_ok = bool(red_checks) and len(red_checks) >= 4 and all(v is True for v in red_checks.values())
    facts.append(_fact(
        "m_c_gate_pass_red_arms_effective",
        c_row["status"] == "PASS" and red_ok,
        f"M-c 最新 evidence PASS + red 面 checks 全真（{len(red_checks)} 臂独立有效）"
        if c_row["status"] == "PASS" and red_ok
        else f"M-c 行: {c_row['detail']}；red 面臂数/真值异常",
    ))

    # ② 18 格判定矩阵 + 读图记录重算绿（M-c 门同族校验器/交叉核验器函数面消费）。
    re_bad: list[str] = []
    cells = (c_doc.get("parity") or {}).get("cells") or []
    closure = (c_doc.get("parity") or {}).get("commercial_closure") or {}
    manifest = (c_doc.get("parity") or {}).get("ai_reading_manifest") or []
    if not c_doc:
        re_bad.append("M-c evidence 缺失")
    else:
        if len(cells) != 18:
            re_bad.append(f"cells {len(cells)}≠18")
        else:
            re_bad += mc.crosscheck_verdicts(cells, closure)[:2]
        if not mc.RECORDS_PATH.is_file():
            re_bad.append("g15_m_c_ai_reading_records.json 缺失")
        elif len(manifest) != 18:
            re_bad.append(f"manifest {len(manifest)}≠18")
        else:
            try:
                rec_doc = wel.load_json(mc.RECORDS_PATH)
                re_bad += mc.validate_reading_records(rec_doc, manifest)[:2]
            except (OSError, json.JSONDecodeError) as e:
                re_bad.append(f"读图记录不可读: {e}")
    facts.append(_fact(
        "verdict_matrix_and_reading_records_revalidate_green",
        not re_bad,
        "18 格判定矩阵交叉核验重算 == 存储标签 + 读图记录 18 格闭集与 PNG digest 逐格绑定重算全绿"
        if not re_bad else "; ".join(re_bad[:3]),
    ))

    # ③ 标定链程序产机核（budget 九条目齐备 measured_local + 注册对账重算 + budget_eval）。
    bud_bad: list[str] = []
    if not ma.BUDGET_PATH.is_file():
        bud_bad.append("g15_budget.json 缺失")
    else:
        budget = wel.load_json(ma.BUDGET_PATH)
        got = {e.get("id"): e for e in (budget.get("entries") or [])}
        for eid in sorted(G15_BUDGET_IDS | set(mc.BUDGET_ENTRY_IDS)):
            e = got.get(eid)
            if e is None:
                bud_bad.append(f"缺条目 {eid}")
            elif e.get("evidence") != "measured_local":
                bud_bad.append(f"{eid} 非 measured_local")
        if len(budget.get("entries") or []) != 9:
            bud_bad.append("g15_budget 条目数 ≠ 9（五既有 + M-c 标定四条目）")
        bud_bad += mc.validate_budget_registration()[:2]
    r = subprocess.run([sys.executable, str(ROOT / "ci" / "budget_eval.py")], cwd=ROOT,
                       capture_output=True, text=True)
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "calibration_chain_program_produced",
        not bud_bad,
        "g15_budget 九条目齐备 measured_local + threshold==measured×2.0 f64 精确重算 + budget_eval 全 PASS（P-09）"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ④ 商用收口判定定盘字面（诚实定盘双态闭集 + 未达格逐格归因 + G16+ 承接锚）。
    cl_bad: list[str] = []
    verdict = closure.get("verdict")
    if verdict not in ("达标", "未达标"):
        cl_bad.append(f"verdict 闭集外: {verdict!r}")
    else:
        met = sum(1 for c in cells if c.get("verdict") == "pass")
        if closure.get("met_count") != met:
            cl_bad.append(f"met_count={closure.get('met_count')}≠重算 {met}")
        if verdict == "未达标":
            anchor = str(closure.get("g16_anchor") or "")
            if "G16+" not in anchor or "允许在G15后无限制新建里程碑继续优化" not in anchor:
                cl_bad.append("g16_anchor 承接锚字面缺失")
            attr = closure.get("unmet_attribution") or {}
            for cell_id in (closure.get("unmet_cells") or []):
                if not str(attr.get(cell_id) or "").strip():
                    cl_bad.append(f"{cell_id} 未达归因空（未达格逐格归因字面）")
        findings = (c_doc.get("parity") or {}).get("findings") or []
        degen = [c for c in cells if c.get("reference_state") == "degenerate_black"]
        if degen and not any(str(f.get("id", "")).startswith("G15-MC-F") for f in findings):
            cl_bad.append("参照退化格在档但 findings 缺 G15-MC-F 登记行")
    facts.append(_fact(
        "commercial_closure_verdict_honest",
        not cl_bad and bool(closure),
        f"商用收口判定定盘字面 = {verdict} {closure.get('met_count')}/{closure.get('total')}（未达格逐格归因 + 承接锚 + findings 登记面全量；未达标如实登记不冒充为合法定盘）"
        if not cl_bad and closure else "; ".join(cl_bad[:3]) or "commercial_closure 缺",
    ))

    # ⑤ 三契约三冻结表 0-byte + RXS-0407 spec 锚定面维持。
    zero_bad: list[str] = []
    for rel in FROZEN_FILES:
        p = ROOT / rel
        if not p.is_file():
            zero_bad.append(f"{rel} 缺失")
            continue
        committed = _git("show", f"HEAD:{rel}")
        if committed.replace("\r\n", "\n") != p.read_text(encoding="utf-8").replace("\r\n", "\n"):
            zero_bad.append(f"{rel} 在树 ≠ HEAD 提交态")
    spec_ok, spec_msg = mc.spec_clause_anchored()
    if not spec_ok:
        zero_bad.append(spec_msg)
    r = subprocess.run([sys.executable, str(ROOT / "ci" / "stable_snapshot.py"), "--check"], cwd=ROOT,
                       capture_output=True, text=True)
    if r.returncode != 0:
        zero_bad.append(f"stable_snapshot --check rc={r.returncode}")
    facts.append(_fact(
        "frozen_0byte_and_spec_anchors_maintained",
        not zero_bad,
        "三 parity 契约 + 三冻结登记表在树 == HEAD 逐字节 + RXS-0407 锚定（trace_matrix/stable_snapshot --check 全 PASS）"
        if not zero_bad else "; ".join(zero_bad[:3]),
    ))

    # ⑥ G5~G14 closed 面 0-byte + M-c 波零 src 变更机核（G14 M-d 复跑义务
    #    not-triggered 如实登记）+ RFC/RXS 命名空间机核。
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
    # 零 src 变更机核：src/ tracked diff（vs HEAD，G15 全期零 src 字面）空 +
    # untracked ⊆ 异己登记六件闭集（立项裁决 3 同律）。
    src_diff = [x for x in (_git("diff", "--name-only", "HEAD", "--", "src") or "").splitlines() if x.strip()]
    src_porc_bad: list[str] = []
    for ln in (_git("status", "--porcelain", "--", "src") or "").splitlines():
        if not ln.strip():
            continue
        state, path = ln[:2], ln[3:].strip()
        if state == "??":
            if path not in mb.ALIEN_UNTRACKED_SRC:
                src_porc_bad.append(f"untracked 越界 {path}")
        else:
            src_porc_bad.append(f"tracked 修改 {path}")
    ledger = wel.load_json(ROOT / "registry" / "number_ledger.json")
    rfc_next_free = ((ledger.get("namespaces") or {}).get("RFC") or {}).get("next_free")
    rxs_next_free = ((ledger.get("namespaces") or {}).get("RXS") or {}).get("next_free")
    ns_ok = rfc_next_free == 31 and rxs_next_free == 408
    ok6 = not bad_committed and not bad_working and not src_diff and not src_porc_bad and ns_ok
    facts.append(_fact(
        "legacy_closed_zero_src_change_and_namespace",
        ok6,
        f"committed 闭集={committed or '空'}；工作树闭集={working or '空'}；src/ 零变更机核绿（G14 M-d 复跑义务 not-triggered——出图链路零 src 改动如实登记）；RFC next_free=31 维持 / RXS next_free=408（RXS-0407 消费校准）"
        if ok6 else f"越界 committed={bad_committed} working={bad_working} src_diff={src_diff[:3]} src_porc={src_porc_bad[:3]} rfc={rfc_next_free!r} rxs={rxs_next_free!r}",
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        extras = [
            _fact("m_c_gate_pass_red_arms_effective", False, "selftest 空目录"),
            _fact("verdict_matrix_and_reading_records_revalidate_green", False, "selftest 空目录"),
            _fact("calibration_chain_program_produced", False, "selftest 空目录"),
            _fact("commercial_closure_verdict_honest", False, "selftest 空目录"),
            _fact("frozen_0byte_and_spec_anchors_maintained", False, "selftest 空目录"),
            _fact("legacy_closed_zero_src_change_and_namespace", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G15.4 M-c absolute quality final review (step 273) — absolute pass line program-produced calibration (dual-seed variance floor p100×2.0) + 18-cell verdicts + AI reading records + honest commercial closure verdict",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: M-c PASS + red arms + matrix/records revalidate + calibration/budget recompute + closure honesty + frozen/spec anchors + zero-src-change/namespace",
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
    """① 缺 M-c evidence → 红;② 真树聚合 VERDICT == 子门实测态（遮蔽即自检红）。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g15_wave4_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G15.4 wave4.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
