#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.4 双端对标波）
"""G14.4 波次聚合门 g14.wave.4.exit（步骤 256；G14_CONTRACT G-G14-6/§2.2/§7 裁决 7；
G14_ACCEPTANCE_MAP §1；同构 ci/g14_wave3_exit_check.py）。

只读汇总 G14.4 波 M-d(M175) 门最新 evidence——双端帧率正式对标（步骤 254，UE
benchmark 臂三轮进程级独立运行 × Rurix 生产管线三轮进程级独立运行 + 通过线
×1.00 逐格判定 + 画质零降级守护 + 差距登记表落盘）——+ 六 facts:
① M-d 门 RED 臂独立有效（最新 evidence red_arms_effective 真——
   single-round/mixed-caliber/unmet-masquerade 三臂）;
② 双端三轮进程级独立运行 measured 面（dual_end_measurement_fresh /
   three_run_independence / sampling_protocol_50x3 三 checks 全真 + parity
   cells 18 格齐备）;
③ 通过线逐格判定 + 差距登记表落盘一致（pass_line_evaluated 真 +
   g14_fps_gap_registry.json 在树且 items 行数 == parity.unmet_count——全达标
   时 0 行空表显式登记面；gaplib 校验面只读复核）;
④ 画质零降级守护（quality_guard_green 真 + parity.quality_guard 登记非空——
   G13 锁定双门最新 evidence PASS 消费面 + G14.3 车道锚带在树）;
⑤ 逐轮守护带登记（逐格 per_run_ratios 三件套非空）;
⑥ G5~G13 closed 面 0-byte（vs G14.0 ref committed diff 闭集 ⊆ 授权面）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。M-d 通过线未达标时本门 verdict 红 = 诚实面（差距
登记表已落盘；G14.x 延续波/G16+ 承接继续优化——契约 §7 裁决 7 字面，不充绿
叙述）。

用法:
  py -3 ci/g14_wave4_exit_check.py --gate g14.wave.4.exit
  py -3 ci/g14_wave4_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g10_gap_registry_lib as gaplib  # noqa: E402
import g13_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g14.wave.4.exit"
NUMERIC_STEP = 256  # 落盘前实测 registry/number_ledger.json CI_step.next_free=256 顺位领取
SUBJECT = "g14_wave4_exit"
WAVE = "G14.4"
SOURCE_REF = (
    "G14_CONTRACT G-G14-6/§2.2/§7 裁决 7;G14_ACCEPTANCE_MAP §1;M-d gate red arms independently effective;"
    "dual-end three-run measured 18 cells;pass line per-cell evaluated + gap registry written;"
    "quality guard green;per-run guard band registered;G5~G13 closed 0-byte"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_wave4_exit_evidence_schema.json"
REGISTRY_PATH = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g14.p0.m_d.dual_end_fps_parity", "g14_m_d_dual_end_fps_parity"),
]

G14_0_REF = "f4c8da0b"
ALLOWED_CLOSED_DIFF = {
    "ci/g10_gap_registry_lib.py",
    "ci/g13_ue_upscale_parity_smoke.py",
    "ci/g13_ue_lumen_gi_parity_smoke.py",
    "milestones/g13/g13_budget.json",
    "milestones/g13/g13_ue_upscale_gap_registry.json",
    "ci/budget_eval.py",
}
SCENES = ("cornell-box", "bistro-interior")


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _git(*args: str) -> str:
    r = subprocess.run(["git"] + list(args), cwd=ROOT, capture_output=True, text=True)
    return r.stdout or ""


def collect_facts() -> list[dict]:
    facts: list[dict] = []
    path = wel.load_latest_evidence(REQUIRED_GATES[0][1])
    doc = wel.load_json(path) if path else {}
    checks = doc.get("checks") or {}
    parity = doc.get("parity") or {}
    cells = parity.get("cells") or []

    # ① RED 臂独立有效
    red_checks = {k: v for k, v in checks.items() if "red" in k}
    facts.append(_fact(
        "m_d_red_arms_independently_effective",
        bool(red_checks) and all(v is True for v in red_checks.values()),
        f"M-d 门 red 面 checks 全真（{len(red_checks)} 面：single-round/mixed-caliber/unmet-masquerade）"
        if red_checks else "缺 red 面",
    ))

    # ② 双端三轮进程级独立运行 measured 面
    meas_keys = ("dual_end_measurement_fresh", "three_run_independence", "sampling_protocol_50x3")
    ok2 = all(checks.get(k) is True for k in meas_keys) and len(cells) == 18
    facts.append(_fact(
        "dual_end_three_run_measured",
        ok2,
        f"双端测量三 checks 全真 + cells {len(cells)}/18 格齐备（三轮进程级独立运行 50×3 口径）"
        if ok2 else f"测量面缺：checks={[k for k in meas_keys if checks.get(k) is not True]} cells={len(cells)}",
    ))

    # ③ 通过线逐格判定 + 差距登记表落盘一致
    unmet_count = parity.get("unmet_count")
    reg_items: list = []
    reg_ok = REGISTRY_PATH.is_file()
    if reg_ok:
        reg_doc = wel.load_json(REGISTRY_PATH)
        reg_items = (reg_doc.get("items") or []) if isinstance(reg_doc, dict) else []
        verrs = gaplib.validate_registry(reg_doc, scene_set=list(SCENES),
                                         registry_name="g14_fps_gap_registry")
        reg_ok = reg_ok and not verrs
    ok3 = (checks.get("pass_line_evaluated") is True and reg_ok
           and unmet_count is not None and len(reg_items) == unmet_count)
    facts.append(_fact(
        "pass_line_evaluated_and_gap_registry_consistent",
        ok3,
        f"通过线逐格判定面真 + 登记表 {len(reg_items)} 行 == unmet_count {unmet_count}（gaplib 校验绿）"
        if ok3 else f"判定/登记表面缺：pass_line_evaluated={checks.get('pass_line_evaluated')} "
        f"reg_rows={len(reg_items)} unmet={unmet_count} reg_ok={reg_ok}",
    ))

    # ④ 画质零降级守护
    guard = parity.get("quality_guard") or []
    ok4 = checks.get("quality_guard_green") is True and bool(guard)
    facts.append(_fact(
        "quality_guard_green",
        ok4,
        f"画质零降级守护绿（{'; '.join(str(g)[:60] for g in guard[:3])}）"
        if ok4 else "画质守护面非绿",
    ))

    # ⑤ 逐轮守护带登记
    ok5 = bool(cells) and all(
        isinstance(c.get("per_run_ratios"), list) and len(c["per_run_ratios"]) == 3
        for c in cells)
    facts.append(_fact(
        "per_run_guard_band_registered",
        ok5,
        f"逐格逐轮守护带登记齐备（{len(cells)} 格 × 3 轮比值）"
        if ok5 else "逐轮守护带面缺",
    ))

    # ⑥ G5~G13 closed 面 0-byte
    globs = [
        "ci/g5_*.py", "ci/g6_*.py", "ci/g7_*.py", "ci/g8_*.py", "ci/g9_*.py",
        "ci/g10_*.py", "ci/g11_*.py", "ci/g12_*.py", "ci/g13_*.py",
        "milestones/g5", "milestones/g6", "milestones/g7", "milestones/g8",
        "milestones/g9", "milestones/g10", "milestones/g11", "milestones/g12",
        "milestones/g13",
    ]
    diff = _git("diff", "--name-only", f"{G14_0_REF}..HEAD", "--", *globs)
    committed = sorted(x for x in diff.splitlines() if x.strip())
    porc = _git("status", "--porcelain", "--", *globs)
    working = sorted(ln[3:].strip() for ln in porc.splitlines() if ln.strip())
    working_allowed = {"milestones/g12/g12_pt_sampler_selection.json"}
    bad_committed = [f for f in committed if f not in ALLOWED_CLOSED_DIFF]
    bad_working = [f for f in working if f not in working_allowed]
    ok6 = not bad_committed and not bad_working
    facts.append(_fact(
        "legacy_criteria_0byte",
        ok6,
        f"committed 闭集={committed or '空'}；工作树闭集={working or '空'}"
        if ok6 else f"越界 committed={bad_committed} working={bad_working}",
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        extras = [
            _fact("m_d_red_arms_independently_effective", False, "selftest 空目录"),
            _fact("dual_end_three_run_measured", False, "selftest 空目录"),
            _fact("pass_line_evaluated_and_gap_registry_consistent", False, "selftest 空目录"),
            _fact("quality_guard_green", False, "selftest 空目录"),
            _fact("per_run_guard_band_registered", False, "selftest 空目录"),
            _fact("legacy_criteria_0byte", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G14.4 M-d(M175) dual-end fps parity gate (step 254)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: red arms + dual-end measured + pass line/gap registry + quality guard + per-run band + legacy 0-byte",
        "aggregate PASS does not mask any child FAIL/SKIP/DEV_ENV_DEGRADE",
        "M-d pass-line unmet => wave verdict red = honest face (gap registry written; G14.x continuation/G16+ carries optimization per CONTRACT §7 ruling 7)",
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
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g14_wave4_selftest_") as td:
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
        print(f"[selftest] FAIL: 聚合 VERDICT 与子门实测态不一致——expected={expected_pass} exit={code}",
              file=sys.stderr)
        return 1
    print(f"[selftest] PASS: 真树聚合 VERDICT={'PASS' if code == 0 else 'FAIL'} == 子门实测态（不遮蔽）")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G14.4 wave4.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
