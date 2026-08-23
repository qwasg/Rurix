#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.2 测量重收割波）
"""G15.2 波次聚合门 g15.wave.2.exit（步骤 270；G15_CONTRACT G-G15-3/§2.2；
G15_ACCEPTANCE_MAP §1；同构 ci/g14_wave2_exit_check.py）。

只读汇总 G15.2 波 M-a 门（g15.p0.m_a.dual_end_quality_reharvest，步骤 269——
双端画质对拍链路全量复跑消费 + 20 行登记表逐项重评 + G15 差距处置表落盘 +
UE 方差带程序产 + AI 读图基线臂）最新 evidence + 六 facts:
① 上游三门（G13 M-c ue_upscale_parity + G13 M-d ue_lumen_gi_parity + G12 M163
   ue_pt_parity）fresh evidence 全 PASS（timestamp ≥ M-a evidence 登记的本波
   启动锚；红面诚实登记不充绿）;
② 三 parity 契约 + 三冻结登记表终态 0-byte（在树 == HEAD 提交态逐字节 git
   机核；G13 upscale 8 行/Lumen 2 行/G12 PT 10 行只消费不回写）;
③ M-a 门 RED 臂独立有效（最新 evidence red 面 checks 非空且全真，≥4 臂）;
④ G15 差距处置表 20 行零空行（gap_id 闭集逐字对账 + 方向判定交叉核验面重算
   绿——经 M-a 门同族校验器/交叉核验器函数面消费）;
⑤ g15_budget 五条目齐备 measured_local 零 estimated + budget_eval 全 PASS
   （P-09 禁手写）;
⑥ G5~G14 closed 面 0-byte（vs G15.0 不可变 ref f061487efaf7816684de18a6ef86554e5c392a75
   committed diff 闭集 ⊆ G14 战后归档授权面 {milestones/g14/g14_budget.json,
   milestones/g14/g14_ue_variance_samples.json}——34f96ac3 归档在案；工作树
   闭集 ⊆ {milestones/g14/g14_ue_variance_samples.json} 样本只追加面）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g15_wave2_exit_check.py --gate g15.wave.2.exit
  py -3 ci/g15_wave2_exit_check.py --selftest
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

GATE_KEY = "g15.wave.2.exit"
NUMERIC_STEP = 270  # 落盘前实测 registry/number_ledger.json CI_step.next_free=270 顺位领取
SUBJECT = "g15_wave2_exit"
WAVE = "G15.2"
SOURCE_REF = (
    "G15_CONTRACT G-G15-3/§2.2;G15_ACCEPTANCE_MAP §1;M-a gate red arms independently effective;"
    "upstream three gates fresh PASS;three frozen registries 0-byte;disposition table 20 rows "
    "zero-empty;g15_budget entries measured_local;G5~G14 closed 0-byte closed-set diff vs G15.0 ref"
)
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_wave2_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g15.p0.m_a.dual_end_quality_reharvest", "g15_m_a_dual_end_quality_reharvest"),
]

G15_0_REF = "f061487efaf7816684de18a6ef86554e5c392a75"  # G15.0 不可变 ref（G14 close-out flip commit，tag g14-closed）
# G15.0→G15.2 期 G5~G14 closed 面允许 diff 闭集（34f96ac3 G14 战后归档授权面在案）。
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

    # ① 上游三门 fresh PASS（timestamp ≥ M-a evidence 登记的本波启动锚）。
    a_path = wel.load_latest_evidence(REQUIRED_GATES[0][1])
    a_doc = wel.load_json(a_path) if a_path else {}
    wave_start = str((a_doc.get("parity") or {}).get("wave_start") or "")
    up_bad: list[str] = []
    up_rows = []
    for key, prefix in ((ma.MC_GATE, ma.MC_PREFIX), (ma.MD_GATE, ma.MD_PREFIX), (ma.G12_GATE, ma.G12_PREFIX)):
        row = ma.upstream_gate_row(key, prefix, wave_start) if wave_start else {
            "symbolic_gate_key": key, "status": "FAIL", "detail": "M-a evidence 缺 wave_start 锚"}
        up_rows.append(row)
        if row["status"] != "PASS":
            up_bad.append(f"{prefix}: {row['detail']}")
    facts.append(_fact(
        "upstream_three_gates_fresh_pass",
        not up_bad,
        f"上游三门 fresh 全 PASS（wave_start={wave_start}；复跑件 timestamp ≥ 本波启动锚）"
        if not up_bad else "; ".join(up_bad[:3]),
    ))

    # ② 三 parity 契约 + 三冻结表 0-byte（在树 == HEAD 提交态逐字节）。
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

    # ③ M-a 门 RED 臂独立有效（red 面 checks 非空全真，≥4 臂）。
    red_checks = {k: v for k, v in (a_doc.get("checks") or {}).items() if k.startswith("red_arm_")}
    red_ok = bool(red_checks) and len(red_checks) >= 4 and all(v is True for v in red_checks.values())
    facts.append(_fact(
        "m_a_red_arms_independently_effective",
        red_ok,
        f"M-a 最新 evidence red 面 checks 全真（{len(red_checks)} 臂独立有效）"
        if red_ok else "M-a red 面 checks 缺失/非真/臂数不足",
    ))

    # ④ 处置表 20 行零空行（gap_id 闭集逐字对账 + 方向交叉核验重算绿）。
    disp_bad: list[str] = []
    disp_doc = None
    if not ma.DISPOSITION_PATH.is_file():
        disp_bad.append("g15_quality_gap_disposition.json 缺失")
    else:
        try:
            disp_doc = wel.load_json(ma.DISPOSITION_PATH)
        except (OSError, json.JSONDecodeError) as e:
            disp_bad.append(f"处置表不可读: {e}")
    if disp_doc is not None:
        frozen_union: list[tuple[str, str]] = []
        for path, src in ((ma.G13_UPSCALE_REGISTRY, "g13_ue_upscale_gap_registry"),
                          (ma.G13_LUMEN_REGISTRY, "g13_ue_lumen_gap_registry"),
                          (ma.G12_PT_REGISTRY, "g12_ue_pt_gap_registry")):
            rdoc = wel.load_json(path)
            for it in rdoc.get("items") or []:
                frozen_union.append((it.get("gap_id"), src))
        verrs = ma.validate_disposition(disp_doc, frozen_union)
        if verrs:
            disp_bad += verrs[:2]
        xerrs = ma.crosscheck_directions(disp_doc)
        if xerrs:
            disp_bad += xerrs[:2]
    facts.append(_fact(
        "disposition_table_20_rows_zero_empty",
        not disp_bad,
        "处置表 20 行零空行 + gap_id 闭集逐字对账 + 方向判定交叉核验重算全绿"
        if not disp_bad else "; ".join(disp_bad[:3]),
    ))

    # ⑤ g15_budget 五条目齐备 measured_local + budget_eval 全 PASS。
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
    r = subprocess.run([sys.executable, str(ROOT / "ci" / "budget_eval.py")], cwd=ROOT,
                       capture_output=True, text=True)
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "budget_entries_measured",
        not bud_bad,
        "g15_budget 五条目齐备 measured_local 零 estimated + budget_eval 全 PASS（P-09）"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ⑥ G5~G14 closed 面 0-byte（vs G15.0 ref committed diff 闭集 ⊆ 归档授权面）。
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
    ok6 = not bad_committed and not bad_working
    facts.append(_fact(
        "legacy_criteria_0byte",
        ok6,
        f"committed 闭集={committed or '空'}（允许面={sorted(ALLOWED_CLOSED_DIFF)}）；工作树闭集={working or '空'}"
        if ok6 else f"越界 committed={bad_committed} working={bad_working}",
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        extras = [
            _fact("upstream_three_gates_fresh_pass", False, "selftest 空目录"),
            _fact("frozen_contracts_and_registries_0byte", False, "selftest 空目录"),
            _fact("m_a_red_arms_independently_effective", False, "selftest 空目录"),
            _fact("disposition_table_20_rows_zero_empty", False, "selftest 空目录"),
            _fact("budget_entries_measured", False, "selftest 空目录"),
            _fact("legacy_criteria_0byte", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G15.2 M-a dual-end quality reharvest (step 269) — upstream three gates fresh rerun consumed + 20-row disposition + UE variance bands + AI reading baseline arm",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: upstream fresh PASS + frozen 0-byte + M-a red arms + disposition 20 rows + g15_budget measured + legacy 0-byte",
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
    """① 缺 M-a evidence → 红;② 真树聚合 VERDICT == 子门实测态（遮蔽即自检红）。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g15_wave2_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G15.2 wave2.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
