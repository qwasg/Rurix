#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.7 vendor 转换并行化延续波）
"""G14.7 波次聚合门 g14.wave.7.exit（步骤 260；G14_CONTRACT §7 裁决 7 延续波程序面/
§2.2 同律；G14_ACCEPTANCE_MAP 附录 A；同构 ci/g14_wave6_exit_check.py）。

只读汇总 G14.7 波 M-g(M177) 门最新 evidence——vendor 转换并行化位级零漂移 +
同码 A/B measured（步骤 259）——+ 六 facts:
① M-g 门 RED 臂独立有效（digest-drift/direction-masquerade 双臂）;
② M-c 回归面最新 evidence 绿（并行化后 M-c 门复跑 PASS——既有判据零降级）;
③ M-d v3 守护面绿（最新 M-d evidence checks production_caliber_v2 +
   stage_a_digest_drift_guard 双真 + digest 锚 18 格在树——并行化后 18 格 × 3 轮
   末帧 digest 全矩阵位级零漂移）;
④ g14_budget production 口径 bistro 双探针格条目 measured_local + budget_eval 全 PASS;
⑤ M-d 通过线诚实红面登记（最新 M-d evidence status==fail 时：gap registry 行数
   == unmet_count 一致性 + 不充绿叙述面——G-G14-6 绿面归后续延续波，本 fact
   只核「红被如实登记」机核面，不以红为绿）;
⑥ G5~G13 closed 面 0-byte（vs G14.0 ref committed diff 闭集 ⊆ 授权面）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g14_wave7_exit_check.py --gate g14.wave.7.exit
  py -3 ci/g14_wave7_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g13_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g14.wave.7.exit"
NUMERIC_STEP = 260  # 落盘前实测 registry/number_ledger.json CI_step.next_free=260 顺位领取
SUBJECT = "g14_wave7_exit"
WAVE = "G14.7"
SOURCE_REF = (
    "G14_CONTRACT §7 裁决 7/§2.2 同律;G14_ACCEPTANCE_MAP 附录 A;M-g gate red arms independently effective;"
    "M-c regression latest green;M-d v3 caliber+digest guards green;budget production bistro entries measured;"
    "M-d pass-line honest red registered;G5~G13 closed 0-byte"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_wave7_exit_evidence_schema.json"
BUDGET_PATH = ROOT / "milestones" / "g14" / "g14_budget.json"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
REGISTRY_PATH = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g14.p0.m_g.vendor_parallel_conversion", "g14_m_g_vendor_parallel_conversion"),
]
MD_PREFIX = "g14_m_d_dual_end_fps_parity"
MC_PREFIX = "g14_m_c_rurix_pipeline_perf"

G14_0_REF = "f4c8da0b"
ALLOWED_CLOSED_DIFF = {
    "ci/g10_gap_registry_lib.py",
    "ci/g13_ue_upscale_parity_smoke.py",
    "ci/g13_ue_lumen_gi_parity_smoke.py",
    "milestones/g13/g13_budget.json",
    "ci/budget_eval.py",
}


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

    # ① M-g RED 臂独立有效
    red_checks = {k: v for k, v in checks.items() if "red" in k}
    facts.append(_fact(
        "m_g_red_arms_independently_effective",
        bool(red_checks) and all(v is True for v in red_checks.values()),
        f"M-g 门 red 面 checks 全真（{len(red_checks)} 面：digest-drift/direction-masquerade）"
        if red_checks else "缺 red 面",
    ))

    # ② M-c 回归面最新绿
    mc_path = wel.load_latest_evidence(MC_PREFIX)
    mc_doc = wel.load_json(mc_path) if mc_path else {}
    mc_checks = mc_doc.get("checks") or {}
    mc_bad = [k for k, v in mc_checks.items() if v is not True]
    ok2 = bool(mc_doc) and mc_doc.get("status") == "pass" and not mc_bad
    facts.append(_fact(
        "m_c_regression_latest_green",
        ok2,
        f"M-c 最新 evidence PASS（{mc_path.name if mc_path else '缺'}，checks 全真）"
        if ok2 else f"M-c 最新面非绿: status={mc_doc.get('status')!r} bad={mc_bad[:3]}",
    ))

    # ③ M-d v3 守护面绿 + 锚 18 格在树
    md_path = wel.load_latest_evidence(MD_PREFIX)
    md_doc = wel.load_json(md_path) if md_path else {}
    md_checks = md_doc.get("checks") or {}
    anchors_doc = wel.load_json(ANCHOR_PATH) if ANCHOR_PATH.is_file() else {}
    anchor_n = len(anchors_doc.get("anchors") or {})
    ok3 = (md_checks.get("production_caliber_v2") is True
           and md_checks.get("stage_a_digest_drift_guard") is True
           and anchor_n == 18)
    facts.append(_fact(
        "m_d_v3_guards_green",
        ok3,
        f"M-d v3 守护双真 + digest 锚 {anchor_n}/18 格在树（并行化后全矩阵位级零漂移）"
        if ok3 else f"M-d v3 守护面缺: cal={md_checks.get('production_caliber_v2')} "
        f"dig={md_checks.get('stage_a_digest_drift_guard')} anchors={anchor_n}",
    ))

    # ④ budget production bistro 双探针格条目 + budget_eval 全 PASS
    bud_bad: list[str] = []
    if not BUDGET_PATH.is_file():
        bud_bad.append("g14_budget.json 缺失")
    else:
        budget = wel.load_json(BUDGET_PATH)
        got = {e.get("id"): e for e in (budget.get("entries") or [])}
        want = {f"g14.pipeline_perf.prod_frame_ms.bistro-interior_t67_{b}"
                for b in ("dlss_sr", "fsr_3_1_5")}
        for eid in sorted(want):
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
        "budget_production_bistro_entries_measured",
        not bud_bad,
        "g14_budget production 口径 bistro 双探针格条目齐备 measured_local + budget_eval 全 PASS"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ⑤ M-d 通过线诚实红面登记（红则核登记一致面；绿则核登记表空表面——两态机核）
    parity = md_doc.get("parity") or {}
    unmet = parity.get("unmet_count")
    reg_doc = wel.load_json(REGISTRY_PATH) if REGISTRY_PATH.is_file() else {}
    reg_rows = len(reg_doc.get("items") or []) if reg_doc else -1
    if md_doc.get("status") == "fail":
        ok5 = unmet is not None and reg_rows == unmet
        detail5 = (f"M-d 红面如实登记：unmet={unmet} == 登记表 {reg_rows} 行（不充绿）"
                   if ok5 else f"M-d 红面登记不一致: unmet={unmet} reg={reg_rows}")
    else:
        ok5 = md_doc.get("status") == "pass" and unmet == 0 and reg_rows == 0
        detail5 = ("M-d 绿面：0 未达标 + 空表显式登记" if ok5
                   else f"M-d 态面异常: status={md_doc.get('status')!r} unmet={unmet} reg={reg_rows}")
    facts.append(_fact("m_d_pass_line_honest_face_registered", ok5 and bool(md_doc), detail5))

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
            _fact("m_g_red_arms_independently_effective", False, "selftest 空目录"),
            _fact("m_c_regression_latest_green", False, "selftest 空目录"),
            _fact("m_d_v3_guards_green", False, "selftest 空目录"),
            _fact("budget_production_bistro_entries_measured", False, "selftest 空目录"),
            _fact("m_d_pass_line_honest_face_registered", False, "selftest 空目录"),
            _fact("legacy_criteria_0byte", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G14.7 M-g(M177) vendor parallel conversion gate (step 259)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: M-g red arms + M-c regression + M-d v3 guards + budget bistro entries + honest red face + legacy 0-byte",
        "aggregate PASS does not mask any child FAIL/SKIP/DEV_ENV_DEGRADE",
        "M-d pass line red stays registered in wave4 exit / future continuation waves (G-G14-6 green face pending)",
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

    with tempfile.TemporaryDirectory(prefix="g14_wave7_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G14.7 wave7.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
