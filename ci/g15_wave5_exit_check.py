#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.5 性能零降级波）
"""G15.5 波次聚合门 g15.wave.5.exit（步骤 276；G15_CONTRACT G-G15-6/§2 G15.5；
G15_ACCEPTANCE_MAP §1；同构 ci/g15_wave4_exit_check.py）。

只读汇总 G15.5 波 M-d 门（g15.p0.m_d.perf_parity_zero_regression，步骤 275——
G14 M-d 门同口径复跑消费 + 逐格 ratio ≥ ×1.00 维持 + 画质锚带复核 + budget 零
estimated + digest 锚漂移守护）最新 evidence + 六 facts:
① M-d 门 fresh PASS + RED 臂独立有效（red_arm_ 面 checks 非空且全真，≥4 臂——
   本门四臂）;
② 复跑真跑面 + 18 格全达（M-d evidence parity 链锚对账：消费的 G14 M-d 复跑件
   timestamp ≥ 本波启动锚 + device executed + base_commit==HEAD；G14 M-d 复跑件
   本体经 M-d 门 validate_cells 纯函数面全量重算绿——18 格闭集/ratio f64 精确
   重算/三轮守护带/口径不变量；G14.12 soak 复跑定盘件同口径对照双件 18 格全达）;
③ digest 锚零漂移（g14_3_stage_a_digest_anchor 冻结锚 18 键与复跑件逐格三轮
   last_frame_digest 位级全等重算绿 + drift 零检出字面——RD-045 同型事件监控
   登记面）;
④ 画质锚带复核（G14 M-c 最新 evidence PASS + 在树 converged.exr 双件 SSIM
   deficit 重算 ≤ 0.010779849285388998 带内——M-d 门 recheck_quality_anchor
   函数面消费）;
⑤ G14 门产 budget 条目零 estimated 维持（g14_budget 全条目 measured_local）+
   g15_budget 九条目齐备 measured_local + budget_eval 全 PASS（P-09）;
⑥ G5~G14 closed 面 0-byte（vs G15.0 不可变 ref f061487efaf7816684de18a6ef86554e5c392a75
   committed diff 闭集 ⊆ G14 战后归档授权面；工作树闭集 ⊆
   {milestones/g14/g14_ue_variance_samples.json} 样本只追加面）+ 零 src 变更
   机核（src/ tracked diff 空 + untracked ⊆ 异己登记六件闭集——G15 全期零 src
   字面维持；本波 G14 M-d 复跑 = M-d 判据复跑义务真跑兑现面，非 src 触改面）+
   RFC/RXS 命名空间机核（RFC next_free=31 维持零消费；RXS next_free=408 维持）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g15_wave5_exit_check.py --gate g15.wave.5.exit
  py -3 ci/g15_wave5_exit_check.py --selftest
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
import g15_perf_parity_guard_smoke as md  # noqa: E402
import g15_gap_fix_closure_smoke as mb  # noqa: E402

GATE_KEY = "g15.wave.5.exit"
NUMERIC_STEP = 276  # 落盘前实测 registry/number_ledger.json CI_step.next_free=276 顺位领取
SUBJECT = "g15_wave5_exit"
WAVE = "G15.5"
SOURCE_REF = (
    "G15_CONTRACT G-G15-6/§2 G15.5;G15_ACCEPTANCE_MAP §1;"
    "M-d gate red arms independently effective;rerun real + 18 cells all met revalidate green;"
    "digest anchor zero drift + quality anchor band recheck;g14_budget zero estimated + budget_eval PASS;"
    "G5~G14 closed 0-byte + zero src change maintained + RFC/RXS namespace check"
)
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_wave5_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g15.p0.m_d.perf_parity_zero_regression", "g15_m_d_perf_parity_zero_regression"),
]

G15_0_REF = "f061487efaf7816684de18a6ef86554e5c392a75"  # G15.0 不可变 ref（G14 close-out flip commit，tag g14-closed）
# G15.0→G15.5 期 G5~G14 closed 面允许 diff 闭集（34f96ac3 G14 战后归档授权面在案）。
ALLOWED_CLOSED_DIFF = {
    "milestones/g14/g14_budget.json",
    "milestones/g14/g14_ue_variance_samples.json",
}
# 工作树允许面 = G14.5a 加性样本级联只追加面（G13 双门复跑门产追加，0-byte 回写禁）。
WORKING_ALLOWED = {
    "milestones/g14/g14_ue_variance_samples.json",
}
G15_BUDGET_IDS = {
    "g15.quality_guard.g14_anchor_ssim_deficit_band",
    "g15.quality_guard.ue_variance_band_upscale_probe_rel",
    "g15.quality_guard.ue_variance_band_lumen_probe_rel",
    "g15.m_a.ue_variance_band_upscale_probe_rel",
    "g15.m_a.ue_variance_band_lumen_probe_rel",
    "g15.m_c.absolute_pass_line_ssim_deficit_tol_cornell_box",
    "g15.m_c.absolute_pass_line_flip_deficit_tol_cornell_box",
    "g15.m_c.absolute_pass_line_ssim_deficit_tol_bistro_interior",
    "g15.m_c.absolute_pass_line_flip_deficit_tol_bistro_interior",
}


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _git(*args: str) -> str:
    r = subprocess.run(["git"] + list(args), cwd=ROOT, capture_output=True, text=True)
    return r.stdout or ""


def collect_facts() -> list[dict]:
    facts: list[dict] = []

    # ① M-d 门 fresh PASS + RED 臂独立有效（red_arm_ 面 checks 非空全真，≥4 臂）。
    d_path = wel.load_latest_evidence("g15_m_d_perf_parity_zero_regression")
    d_doc = wel.load_json(d_path) if d_path else {}
    d_row = wel.require_gate_pass(*REQUIRED_GATES[0])
    red_checks = {k: v for k, v in (d_doc.get("checks") or {}).items() if k.startswith("red_arm_")}
    red_ok = bool(red_checks) and len(red_checks) >= 4 and all(v is True for v in red_checks.values())
    facts.append(_fact(
        "m_d_gate_pass_red_arms_effective",
        d_row["status"] == "PASS" and red_ok,
        f"M-d 最新 evidence PASS + red_arm_ 面 checks 全真（{len(red_checks)} 臂独立有效）"
        if d_row["status"] == "PASS" and red_ok
        else f"M-d 行: {d_row['detail']}；red_arm_ 面臂数/真值异常",
    ))

    # ② 复跑真跑面 + 18 格全达（链锚对账 + G14 M-d 复跑件本体全量重算绿 + G14.12 对照）。
    rt_bad: list[str] = []
    parity = d_doc.get("parity") or {}
    consumed_path = ROOT / str(parity.get("g14_m_d_evidence") or "")
    consumed: dict = {}
    if not d_doc:
        rt_bad.append("M-d evidence 缺失")
    else:
        if not md.freshness_ok(str(parity.get("g14_m_d_evidence_timestamp") or ""), str(parity.get("wave_start") or "9")):
            rt_bad.append("消费的 G14 M-d 复跑件 timestamp < 本波启动锚（旧件冒充 fresh 面）")
        if parity.get("g14_m_d_base_commit") != (_git("rev-parse", "HEAD") or "").strip():
            rt_bad.append("消费的 G14 M-d 复跑件 base_commit ≠ HEAD（非同树复跑面）")
        if d_doc.get("device_section_state") != "executed":
            rt_bad.append("M-d device_section_state ≠ executed")
        if not consumed_path.is_file():
            rt_bad.append(f"消费的 G14 M-d 复跑件缺失: {parity.get('g14_m_d_evidence')!r}")
        else:
            consumed = wel.load_json(consumed_path)
            ok, detail = wel.gate_pass_reason(consumed, md.G14_MD_GATE)
            if not ok:
                rt_bad.append(f"G14 M-d 复跑件非全绿: {detail}")
            else:
                rt_bad += md.validate_cells(consumed, md.load_anchors())[:3]
        cmp_doc = parity.get("comparison_vs_g14_12") or {}
        if len(cmp_doc.get("cells") or []) != 18 or cmp_doc.get("all_ge_pass_line_both") is not True:
            rt_bad.append("G14.12 soak 复跑同口径对照面非 18 格双达")
    facts.append(_fact(
        "rerun_real_and_18_cells_all_met",
        not rt_bad,
        "复跑真跑面（fresh ≥ 启动锚 + 同树 + device executed）+ 18 格 ratio ≥ ×1.00 全量重算绿 + G14.12 对照双件全达"
        if not rt_bad else "; ".join(rt_bad[:3]),
    ))

    # ③ digest 锚零漂移（位级对账重算绿 + drift 零检出字面）。
    dg_bad: list[str] = []
    if consumed:
        anchors = md.load_anchors()
        if sorted(anchors) != sorted(md.expected_cell_keys()):
            dg_bad.append("冻结锚 18 键闭集不齐")
        drift_errs = [e for e in md.validate_cells(consumed, anchors)
                      if "digest 漂移" in e or "冻结锚缺" in e or "last_frame_digest" in e]
        dg_bad += drift_errs[:2]
    else:
        dg_bad.append("G14 M-d 复跑件未装载——锚对账跳过")
    dg = parity.get("digest_anchor") or {}
    dmon = parity.get("drift_monitoring") or {}
    if dg.get("drift_count") != 0 or dmon.get("rd_045_type_digest_drift_detected") != 0 or dg.get("cells_checked") != 18:
        dg_bad.append("M-d evidence drift 计数/覆盖面非零漂移字面")
    facts.append(_fact(
        "digest_anchor_zero_drift",
        not dg_bad,
        "g14_3_stage_a_digest_anchor 冻结锚 18 格 × 3 轮位级全等重算绿 + drift 零检出（RD-045 同型事件监控登记面维持 open-defer）"
        if not dg_bad else "; ".join(dg_bad[:3]),
    ))

    # ④ 画质锚带复核（M-d 门 recheck 函数面消费——G14 M-c 最新 PASS + SSIM deficit 重算带内）。
    qa_ok, qa_detail, qa_deficit = md.recheck_quality_anchor()
    stored_qa = parity.get("quality_anchor") or {}
    qa_ok = qa_ok and stored_qa.get("within_band") is True and stored_qa.get("deficit_recomputed") == qa_deficit
    facts.append(_fact(
        "quality_anchor_band_recheck",
        qa_ok,
        f"画质锚带复核重算绿：{qa_detail}（M-d evidence 存储面与聚合侧重算位级一致）"
        if qa_ok else f"画质锚带复核异常: {qa_detail}；stored={stored_qa.get('deficit_recomputed')!r} within={stored_qa.get('within_band')!r}",
    ))

    # ⑤ G14 门产 budget 零 estimated + g15_budget 九条目 measured_local + budget_eval 全 PASS。
    bud_bad: list[str] = []
    g14_bud = wel.load_json(md.G14_BUDGET_PATH) if md.G14_BUDGET_PATH.is_file() else {}
    g14_entries = g14_bud.get("entries") or []
    if not g14_entries:
        bud_bad.append("g14_budget 条目空")
    for e in g14_entries:
        if e.get("evidence") != "measured_local" or e.get("skip_reason") is not None:
            bud_bad.append(f"g14_budget {e.get('id')} 非 measured_local/skip 非 null")
    stored_g14 = parity.get("g14_budget") or {}
    if stored_g14.get("entries") != len(g14_entries) or stored_g14.get("zero_estimated") is not True:
        bud_bad.append("M-d evidence g14_budget 存储面与在树重算不符")
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
        if len(budget.get("entries") or []) != 9:
            bud_bad.append("g15_budget 条目数 ≠ 9")
    r = subprocess.run([sys.executable, str(ROOT / "ci" / "budget_eval.py")], cwd=ROOT,
                       capture_output=True, text=True)
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "budgets_zero_estimated_and_eval_pass",
        not bud_bad,
        f"g14_budget {len(g14_entries)} 条目零 estimated/skip + g15_budget 九条目 measured_local + budget_eval 全 PASS（P-09）"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ⑥ G5~G14 closed 面 0-byte + 零 src 变更机核 + RFC/RXS 命名空间机核。
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
        f"committed 闭集={committed or '空'}；工作树闭集={working or '空'}；src/ 零变更机核绿（G15 全期零 src 字面维持；本波 G14 M-d 复跑 = M-d 判据复跑义务真跑兑现，非 src 触改面）；RFC next_free=31 维持 / RXS next_free=408 维持"
        if ok6 else f"越界 committed={bad_committed} working={bad_working} src_diff={src_diff[:3]} src_porc={src_porc_bad[:3]} rfc={rfc_next_free!r} rxs={rxs_next_free!r}",
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        extras = [
            _fact("m_d_gate_pass_red_arms_effective", False, "selftest 空目录"),
            _fact("rerun_real_and_18_cells_all_met", False, "selftest 空目录"),
            _fact("digest_anchor_zero_drift", False, "selftest 空目录"),
            _fact("quality_anchor_band_recheck", False, "selftest 空目录"),
            _fact("budgets_zero_estimated_and_eval_pass", False, "selftest 空目录"),
            _fact("legacy_closed_zero_src_change_and_namespace", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G15.5 M-d perf parity zero regression guard (step 275) — G14 M-d gate same-caliber rerun consumed fresh + 18-cell ratio >= x1.00 maintained + quality anchor band recheck + budget zero estimated + digest anchor drift guard",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: M-d PASS + red arms + rerun-real/18-cell revalidate + anchor zero drift + quality band recheck + budgets/eval + closed 0-byte/namespace",
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
    """① 缺 M-d evidence → 红;② 真树聚合 VERDICT == 子门实测态（遮蔽即自检红）。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g15_wave5_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G15.5 wave5.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
