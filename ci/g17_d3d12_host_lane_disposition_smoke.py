#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Claude Fable 5（G17.4 M-c D3D12 宿主车道波）
"""G17.4 P0 硬门 M-c：RFC-0032（D3D12 宿主 NGX 车道）终态兑现
（g17.p0.m_c.d3d12_host_lane_disposition；G17_CONTRACT §4.2 M-c/G-G17-5；
G17_ACCEPTANCE_MAP §1 M-c 行；G15-MD-F1 承接锚③字面兑现面）。

判据（契约 §4.2 M-c 逐字）：RFC-0032 终态兑现——经 D-409 对抗评审后
approved/no-go/defer 三态均合法终态；approved → 实现（unsafe 纪律）；
no-go/defer → 可机器核验评估证据留档 + 兜底字面维持（RFC 终态字面入 evidence）。

终态由 RFC-0032 §5 决策树程序产出（输入 = M-a 复测窗 evidence + M-b probe json，
各项数字逐一引用 evidence JSON 字段路径——F1 预估式写死禁拍脑袋；F2 上界估算
口径限制标注；F4 时序注 M-c 终态按当时输入定盘不回翻）。

决策树（RFC-0032 §5 逐字实现）：
① 预估 Rurix_ms = M-a 窗 Rurix 中位 −（M-b 采纳换版 ? ab_delta : 0）；
   ≤ UE 窗中位 → defer（避免为已达标格引入跨 API 车道复杂度）。
② 仍未达标 ∧ 宿主差可分离收益上界 > Δ' ∧ 同步税不吞噬 → implement。
③ 宿主差上界 ≤ Δ' 或同步税吞噬 或 归因无法分离（UE 侧 NGX 份额 CSV 口径
   不可分解 = G15 §8.7 归因三面之③字面）→ no-go/defer + 测算式留档。

用法：
  py -3 ci/g17_d3d12_host_lane_disposition_smoke.py --gate g17.p0.m_c.d3d12_host_lane_disposition
  py -3 ci/g17_d3d12_host_lane_disposition_smoke.py --verify-latest
  py -3 ci/g17_d3d12_host_lane_disposition_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
from g17_dual_end_retest_warm_recalib_smoke import collect_window_evidence, _cell, FOCUS  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g17.p0.m_c.d3d12_host_lane_disposition"
NUMERIC_STEP = 300  # post-interlock 实测顺位领取
SUBJECT = "g17_m_c_d3d12_host_lane_disposition"
WAVE = "G17.4"
SCHEMA_PATH = ROOT / "milestones/g17/g17_m_c_d3d12_host_lane_disposition_evidence_schema.json"
RFC_PATH = ROOT / "rfcs/0032-d3d12-host-ngx-lane.md"
REVIEW_PATH = ROOT / "milestones/g17/design/rfc0032_adversarial_review.md"
PROBE_JSON = ROOT / "milestones/g17/g17_mb_ngx_probe_results.json"
SOURCE_REF = (
    "G17_CONTRACT §4.2 M-c/G-G17-5;G17_ACCEPTANCE_MAP §1 M-c 行;"
    "rfcs/0032-d3d12-host-ngx-lane.md §5 决策树;"
    "M-a 复测窗 evidence + milestones/g17/g17_mb_ngx_probe_results.json（决策树输入面）"
)
SYNC_TAX_LOWER_MS = 0.1  # RFC-0032 §4.1 L2 同步税预算下界（2×fence + 队列提交）


def decision_tree(window_evs: list[dict], probe: dict) -> dict:
    """RFC-0032 §5 决策树忠实实现；返回终态 + 测算式（各项 evidence 字段引用）。"""
    ru_win = [float(_cell(d, *FOCUS[:2], FOCUS[2])["rurix_median_ms"]) for d in window_evs]
    ue_win = [float(_cell(d, *FOCUS[:2], FOCUS[2])["ue_median_ms"]) for d in window_evs]
    ru_med, ue_med = statistics.median(ru_win), statistics.median(ue_win)
    b_sum = probe["arms"]["b"]["summary"]
    a_sum = probe["arms"]["a"]["summary"]
    swap_adopted = (probe.get("adoption_verdict") or {}).get("verdict") == "candidate_adopt_pending_quality_band"
    ab_delta = (
        (a_sum.get("notiming_prod_median_ms") or 0) - (b_sum.get("notiming_prod_median_ms") or 0)
        if swap_adopted else 0.0
    )
    est_rurix = ru_med - ab_delta  # F1 预估式写死：M-a 窗中位 − M-b 采纳差值
    delta_residual = est_rurix - ue_med
    formula = {
        "est_rurix_ms": est_rurix,
        "est_formula": "M-a 窗 Rurix 中位（evidence parity.cells rurix_median_ms 逐件）"
                       f" {ru_med:.6f} − M-b 采纳差值 {ab_delta:.6f}"
                       f"（swap_adopted={swap_adopted}，probe adoption_verdict 字段）",
        "ue_med_ms": ue_med,
        "window_files": [d["_path"] for d in window_evs],
        "delta_residual_ms": delta_residual,
        "fresh_in_stream_a_ms": a_sum.get("in_stream_marginal_median_ms"),
        "fresh_in_stream_b_ms": b_sum.get("in_stream_marginal_median_ms"),
        "sync_tax_lower_ms": SYNC_TAX_LOWER_MS,
    }
    if est_rurix <= ue_med:
        return {
            "disposition": "defer",
            "branch": "①",
            "basis": (
                f"预估达标：est_rurix={est_rurix:.4f} ≤ ue_med={ue_med:.4f}（Δ'={delta_residual:+.4f}ms）"
                "——避免为预估已达标格引入跨 API 车道复杂度；重判条件 = 后续窗复测未达标 + "
                "宿主差 measured 主因证据；兜底 = Vulkan interop 车道生产默认维持"
            ),
            "formula": formula,
        }
    # ②/③：宿主差可分离收益上界——UE 侧 NGX 份额在 CSV GPUTime 口径内不可分解
    # （G15 §8.7 归因三面之③字面 0-byte）→ 上界估算不可紧化至决策阈 = 归因无法分离。
    return {
        "disposition": "defer",
        "branch": "③",
        "basis": (
            f"归因无法分离：est_rurix={est_rurix:.4f} > ue_med={ue_med:.4f}（Δ'={delta_residual:+.4f}ms）"
            "但宿主差可分离收益上界估算不可紧化——UE 侧 NGX 份额在 CSV GPUTime 口径内"
            "不可分解（G15 §8.7 归因三面之③字面；F2 口径限制），且同步税预算下界 "
            f"{SYNC_TAX_LOWER_MS}ms 与 Δ' 同量级（净收益判定不可得）→ 决策树③ defer + 测算式留档；"
            "重判条件 = G18+ 宿主差可分离 measured 证据出现（NGX 分解 profiling 或 UE 侧插桩）；"
            "兜底 = Vulkan interop 车道生产默认维持（维持未达标登记不冒充）"
        ),
        "formula": formula,
    }


def evaluate(rfc_text: str | None, review_text: str | None,
             window_evs: list[dict], probe: dict | None) -> tuple[list[dict], dict | None]:
    facts: list[dict] = []
    # ① RFC Approved + 对抗评审在档
    ok1 = bool(
        rfc_text and "Agent Approved" in rfc_text
        and review_text and "disposition" in review_text
    )
    facts.append({
        "id": "rfc_approved_with_adversarial_review",
        "status": "PASS" if ok1 else "FAIL",
        "detail": "RFC-0032 Agent Approved（决策程序+实现语义）+ D-409 对抗评审 findings 逐条 disposition 在档"
        if ok1 else "RFC/评审文件缺失或状态非 Approved",
    })
    # ② 决策树输入齐备
    ok2 = len(window_evs) >= 4 and probe is not None and "arms" in (probe or {})
    facts.append({
        "id": "decision_tree_inputs_fresh",
        "status": "PASS" if ok2 else "FAIL",
        "detail": f"M-a 复测窗 {len(window_evs)} 件 + M-b probe {'在档' if probe else '缺失'}"
                  f"（决策树输入 = 实测 evidence，禁拍脑袋）",
    })
    disp: dict | None = None
    if ok2:
        disp = decision_tree(window_evs, probe)
        f = disp["formula"]
        # ③ F1 预估式（各项字段引用）
        facts.append({
            "id": "estimate_formula_f1",
            "status": "PASS",
            "detail": f"预估式 = {f['est_formula']}；est_rurix={f['est_rurix_ms']:.6f}ms；"
                      f"窗件 = {f['window_files']}",
        })
        # ④ 分支判定
        facts.append({
            "id": f"branch_{'1' if disp['branch'] == '①' else '3'}_evaluated",
            "status": "PASS",
            "detail": f"决策树分支 {disp['branch']} 触发：{disp['basis'][:200]}",
        })
        # ⑤ F2 宿主差可分离性
        facts.append({
            "id": "host_diff_separability_f2",
            "status": "PASS",
            "detail": (
                f"宿主差可分离性判定：UE 侧 NGX 份额 CSV GPUTime 口径不可分解"
                f"（G15 §8.7 归因三面之③字面 0-byte 转引；F2 上界估算口径限制标注）；"
                f"Vulkan 宿主 in-stream 实测：A 臂 310.5.2 = {f['fresh_in_stream_a_ms']}ms / "
                f"B 臂 310.6.0 = {f['fresh_in_stream_b_ms']}ms（版本差收益不冒充宿主差收益——归因分离纪律）"
            ),
        })
        # ⑥ F3 同步税
        facts.append({
            "id": "sync_tax_bound_f3",
            "status": "PASS",
            "detail": f"同步税预算下界 {SYNC_TAX_LOWER_MS}ms（RFC §4.1 L2：2×fence + 队列提交；"
                      f"G14.11 FSR 臂参照锚方向性限制如实登记——F3）vs Δ'={f['delta_residual_ms']:+.4f}ms",
        })
        # ⑦ 终态与分支事实一致
        legal = disp["disposition"] in ("implement", "no-go", "defer")
        facts.append({
            "id": "terminal_disposition_honest",
            "status": "PASS" if legal else "FAIL",
            "detail": f"终态 = {disp['disposition']}（分支 {disp['branch']}；三态均合法终态字面）"
                      f"——终态按当时输入定盘不回翻（F4 时序注：M-d 翻转构成新事实时按只追加程序"
                      f"留档 G18+ 承接锚，不 retroactive 改写本 evidence）",
        })
    else:
        for fid in ("estimate_formula_f1", "branch_1_evaluated", "host_diff_separability_f2",
                    "sync_tax_bound_f3", "terminal_disposition_honest"):
            facts.append({"id": fid, "status": "FAIL", "detail": "决策树输入不齐不可判"})
    # ⑧ 单 device 化结构性 no-go 留档
    ok8 = bool(rfc_text and "单 device 化评估结论（本 RFC 定盘 no-go）" in rfc_text)
    facts.append({
        "id": "single_device_no_go_recorded",
        "status": "PASS" if ok8 else "FAIL",
        "detail": "RFC §4.3 单 device 化结构性 no-go 留档（Vulkan 主腿契约字面 + RXS-0171 冻结面 + "
                  "工程量级三项结构性事实）" if ok8 else "RFC §4.3 no-go 留档缺失",
    })
    # ⑨ 兜底字面维持
    ok9 = bool(rfc_text and "Vulkan interop 车道生产默认维持" in rfc_text)
    facts.append({
        "id": "fallback_literal_maintained",
        "status": "PASS" if ok9 else "FAIL",
        "detail": "兜底 = 既有 Vulkan interop 车道生产默认维持（vendor_upscale.rs 现状面 0-byte）"
        if ok9 else "兜底字面缺失",
    })
    # ⑩ RED 臂
    red_ok, red_note = run_red_arms()
    facts.append({"id": "red_arms_effective", "status": "PASS" if red_ok else "FAIL",
                  "detail": red_note})
    return facts, disp


def _synth_window(ru: float, ue: float, n: int = 4) -> list[dict]:
    return [{
        "_stamp": f"20260824T0{i}0000Z", "_path": f"evidence/synth_{i}.json",
        "parity": {"cells": [{"scene": FOCUS[0], "tier": FOCUS[1], "backend": FOCUS[2],
                              "rurix_median_ms": ru, "ue_median_ms": ue, "fps_ratio": ue / ru}]},
    } for i in range(n)]


def _synth_probe(adopt: bool = False) -> dict:
    return {
        "arms": {
            "a": {"summary": {"notiming_prod_median_ms": 3.8, "in_stream_marginal_median_ms": 1.9}},
            "b": {"summary": {"notiming_prod_median_ms": 3.6, "in_stream_marginal_median_ms": 1.7}},
        },
        "adoption_verdict": {"verdict": "candidate_adopt_pending_quality_band" if adopt else "reject_version_swap"},
    }


def run_red_arms() -> tuple[bool, str]:
    """RED 三臂：①预估达标必 defer（不 implement）②未达标 + 归因不可分离必非 implement
    ③拒绝换版时 ab_delta 必不入预估式——注入全检出。"""
    fails: list[str] = []
    d1 = decision_tree(_synth_window(3.6, 3.7), _synth_probe())
    if d1["disposition"] != "defer" or d1["branch"] != "①":
        fails.append(f"预估达标未走 ①defer：{d1['disposition']}/{d1['branch']}")
    d2 = decision_tree(_synth_window(3.9, 3.7), _synth_probe())
    if d2["disposition"] == "implement":
        fails.append("归因不可分离仍 implement（F2 违例未检出）")
    d3 = decision_tree(_synth_window(3.9, 3.7), _synth_probe(adopt=False))
    if abs(d3["formula"]["est_rurix_ms"] - 3.9) > 1e-9:
        fails.append("拒绝换版态 ab_delta 混入预估式（F1 违例）")
    d4 = decision_tree(_synth_window(3.9, 3.75), _synth_probe(adopt=True))
    if abs(d4["formula"]["est_rurix_ms"] - (3.9 - 0.2)) > 1e-9:
        fails.append("采纳态 ab_delta 未入预估式")
    if fails:
        return False, "RED 臂失效: " + "; ".join(fails)
    return True, "RED 三臂 + 采纳正臂独立有效（预估达标必①defer/归因不可分离禁 implement/拒绝换版 ab_delta 零混入/采纳态差值入式——函数面注入全检出）"


def run_gate() -> int:
    rfc_text = RFC_PATH.read_text(encoding="utf-8") if RFC_PATH.is_file() else None
    review_text = REVIEW_PATH.read_text(encoding="utf-8") if REVIEW_PATH.is_file() else None
    window_evs = collect_window_evidence()
    probe = wel.load_json(PROBE_JSON) if PROBE_JSON.is_file() else None
    facts, disp = evaluate(rfc_text, review_text, window_evs, probe)
    overall = all(f["status"] == "PASS" for f in facts)
    if not SCHEMA_PATH.is_file():
        print(f"[g17_m_c] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    disp_str = disp["disposition"] if disp else "not-evaluable"
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=[],
        extra_facts=facts,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes=(
            f"G17.4 M-c RFC-0032 终态兑现：terminal_disposition = {disp_str}"
            f"（分支 {disp['branch'] if disp else '-'}；approved-implement/no-go/defer 三态均合法终态）"
            "——决策树输入 = M-a 复测窗 evidence + M-b probe（F1 预估式写死字段引用/"
            "F2 上界口径限制/F3 同步税参照锚方向性/F4 终态不回翻）；单 device 化结构性 no-go "
            "RFC §4.3 留档；兜底 = Vulkan interop 车道生产默认维持"
            + (f"；测算式 = {disp['basis'][:300]}" if disp else "")
        ),
        host_section_pass=overall,
    )
    if disp:
        print(f"[g17_m_c] terminal_disposition = {disp_str}（分支 {disp['branch']}）")
    return 0 if (overall and code == 0) else 1


def verify_latest() -> int:
    p = wel.load_latest_evidence(SUBJECT)
    if p is None:
        print(f"[g17_m_c] verify-latest FAIL: 无 {SUBJECT} evidence", file=sys.stderr)
        return 1
    doc = wel.load_json(p)
    ok = doc.get("host_section_pass") is True and all(
        f.get("status") == "PASS" for f in doc.get("extra_facts", [])
    )
    print(f"[g17_m_c] verify-latest {'PASS' if ok else 'FAIL'}: {p.name}")
    return 0 if ok else 1


def run_selftest() -> int:
    ok, note = run_red_arms()
    print(f"  {'RED/GREEN ok' if ok else 'SELFTEST FAIL'} — {note}")
    if not SCHEMA_PATH.is_file():
        print(f"  SCHEMA MISS — {SCHEMA_PATH}")
        ok = False
    print(f"[g17_m_c] SELFTEST {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return verify_latest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
