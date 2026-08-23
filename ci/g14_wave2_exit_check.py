#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.2 修订与测量波）
"""G14.2 波次聚合门 g14.wave.2.exit（步骤 252；G14_CONTRACT G-G14-4/§2.2；
G14_ACCEPTANCE_MAP §1；同构 ci/g13_wave2_exit_check.py）。

只读汇总 G14.2 波 M-a(M172)/M-b(M173) 双门最新 evidence——登记表 UE 方差带
结构化对账修订（步骤 250，G13 §8.7 承接锚兑现）+ UE benchmark 臂正式帧率测量
（步骤 251，G10-N11 承接锚兑现）——+ 六 facts:
① G13 锁定双差距登记表终态 0-byte（在树 digest == G13.5b 锁定面提交态，
   M-a 复跑前后逐字节一致字面承接）;
② M-a/M-b 门 RED 臂独立有效（最新 evidence red 面 checks 非空且全真）;
③ g14_budget M-a/M-b 条目齐备 measured_local 零 estimated + budget_eval
   全 PASS（P-09 禁手写）;
④ M-a 承接锚兑现面（修订后 M-c/M-d 复跑双绿 + 方差带程序产入 budget 字面）;
⑤ M-b 三轮进程级独立运行 + MRQ 开销剥离 measured 面（逐格 overhead 非空）;
⑥ G5~G13 closed 面 0-byte（vs G14.0 不可变 ref f4c8da0b committed diff 闭集
   ⊆ {ci/g10_gap_registry_lib.py, ci/g13_ue_upscale_parity_smoke.py,
   ci/g13_ue_lumen_gi_parity_smoke.py, milestones/g13/g13_budget.json}——G14 M-a
   修订授权面 + G14.12 RFC-0030 §4.7 测量派生冻结件重派生 + G13.5a 加性演进位，
   沿 G13.5a M-e git_closed_surface 同构口径）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g14_wave2_exit_check.py --gate g14.wave.2.exit
  py -3 ci/g14_wave2_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g14_registry_variance_band_reconciliation_smoke as ma  # noqa: E402
import g13_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g14.wave.2.exit"
NUMERIC_STEP = 252  # 落盘前实测 registry/number_ledger.json CI_step.next_free=252 顺位领取
SUBJECT = "g14_wave2_exit"
WAVE = "G14.2"
SOURCE_REF = (
    "G14_CONTRACT G-G14-4/§2.2;G14_ACCEPTANCE_MAP §1;M-a/M-b gate red arms independently effective;"
    "g13 locked registries 0-byte;g14_budget entries measured_local;G13 §8.7 anchor fulfilled;"
    "three-run independence + MRQ overhead measured;G5~G13 closed 0-byte closed-set diff"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_wave2_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g14.p0.m_a.registry_variance_band_reconciliation", "g14_m_a_registry_variance_band_reconciliation"),
    ("g14.p0.m_b.ue_benchmark_arm_measurement", "g14_m_b_ue_benchmark_arm_measurement"),
]

G14_0_REF = "f4c8da0b"
# G14 期 G5~G13 closed 面允许 diff 闭集（G14 M-a 修订授权面，契约 §8.7 承接锚字面）。
ALLOWED_CLOSED_DIFF = {
    "ci/g10_gap_registry_lib.py",
    "ci/g13_ue_upscale_parity_smoke.py",
    "ci/g13_ue_lumen_gi_parity_smoke.py",
    "milestones/g13/g13_budget.json",
}
REGISTRY_FILES = [
    ROOT / "milestones" / "g13" / "g13_ue_upscale_gap_registry.json",
    ROOT / "milestones" / "g13" / "g13_ue_lumen_gap_registry.json",
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _git(*args: str) -> str:
    r = subprocess.run(["git"] + list(args), cwd=ROOT, capture_output=True, text=True)
    return r.stdout or ""


def collect_facts() -> list[dict]:
    facts: list[dict] = []

    # ① G13 锁定双登记表终态 0-byte（在树 == HEAD 提交态逐字节）。
    reg_bad: list[str] = []
    for p in REGISTRY_FILES:
        rel = p.relative_to(ROOT).as_posix()
        if not p.is_file():
            reg_bad.append(f"{rel} 缺失")
            continue
        committed = _git("show", f"HEAD:{rel}")
        if committed.replace("\r\n", "\n") != p.read_text(encoding="utf-8"):
            reg_bad.append(f"{rel} 在树 ≠ HEAD 提交态")
    facts.append(_fact(
        "g13_locked_registries_0byte",
        not reg_bad,
        "G13 锁定双登记表在树 == HEAD 提交态逐字节（8+2 行终态 0-byte）"
        if not reg_bad else "; ".join(reg_bad[:3]),
    ))

    # ② 双门 RED 臂独立有效（red 面 checks 非空全真）。
    red_bad: list[str] = []
    red_total = 0
    for _key, prefix in REQUIRED_GATES:
        path = wel.load_latest_evidence(prefix)
        if path is None:
            red_bad.append(f"{prefix} 缺最新 evidence")
            continue
        doc = wel.load_json(path)
        red_checks = {k: v for k, v in (doc.get("checks") or {}).items() if "red_" in k}
        red_total += len(red_checks)
        if not red_checks or any(v is not True for v in red_checks.values()):
            red_bad.append(f"{prefix} red 面 checks 缺失或非真")
    facts.append(_fact(
        "m_a_m_b_red_arms_independently_effective",
        not red_bad,
        f"双门最新 evidence red 面 checks 全真（共 {red_total} 臂独立有效）"
        if not red_bad else "; ".join(red_bad[:3]),
    ))

    # ③ g14_budget 条目齐备 measured_local + budget_eval 全 PASS。
    bud_bad: list[str] = []
    if not ma.BUDGET_PATH.is_file():
        bud_bad.append("g14_budget.json 缺失")
    else:
        budget = wel.load_json(ma.BUDGET_PATH)
        want = {"g14.ue_variance_band.upscale_probe_rel", "g14.ue_variance_band.lumen_probe_rel"}
        want |= {f"g14.ue_benchmark.frame_ms.{s}_t{t}" for s in ("cornell-box", "bistro-interior") for t in (50, 67, 100)}
        got = {e.get("id"): e for e in (budget.get("entries") or [])}
        for eid in sorted(want):
            e = got.get(eid)
            if e is None:
                bud_bad.append(f"缺条目 {eid}")
            elif e.get("evidence") != "measured_local":
                bud_bad.append(f"{eid} 非 measured_local")
    r = subprocess.run([sys.executable, str(ROOT / "ci" / "budget_eval.py")], cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "budget_entries_measured",
        not bud_bad,
        "g14_budget M-a 两条目 + M-b 六条目齐备 measured_local 零 estimated + budget_eval 全 PASS（P-09）"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ④ M-a 承接锚兑现面（复跑双绿 + 方差带程序产）。
    a_path = wel.load_latest_evidence(REQUIRED_GATES[0][1])
    a_doc = wel.load_json(a_path) if a_path else {}
    a_checks = a_doc.get("checks") or {}
    a_ok = all(a_checks.get(k) is True for k in (
        "m_c_rerun_pass", "m_d_rerun_pass", "registries_0byte_post_rerun",
        "ue_variance_bands_measured_into_budget"))
    facts.append(_fact(
        "m_a_anchor_fulfilled",
        a_ok,
        "修订后 M-c/M-d 复跑双绿 + 双登记表 0-byte + 方差带程序产入 budget（G13 §8.7 承接锚兑现）"
        if a_ok else f"M-a 承接面缺: {[k for k in ('m_c_rerun_pass','m_d_rerun_pass','registries_0byte_post_rerun','ue_variance_bands_measured_into_budget') if a_checks.get(k) is not True]}",
    ))

    # ⑤ M-b 三轮独立性 + MRQ 开销剥离 measured 面。
    b_path = wel.load_latest_evidence(REQUIRED_GATES[1][1])
    b_doc = wel.load_json(b_path) if b_path else {}
    b_checks = b_doc.get("checks") or {}
    overhead = ((b_doc.get("parity") or {}).get("mrq_overhead")) or []
    b_ok = (
        b_checks.get("three_process_independent_runs") is True
        and b_checks.get("mrq_overhead_measured") is True
        and len(overhead) == 6
        and all("mrq_capture_overhead_ms" in r for r in overhead)
    )
    facts.append(_fact(
        "m_b_three_run_and_overhead",
        b_ok,
        f"三轮进程级独立运行 + MRQ 开销剥离 {len(overhead)} 格 measured（G10-N11 兑现）"
        if b_ok else "M-b 三轮独立性/开销剥离面缺",
    ))

    # ⑥ G5~G13 closed 面 0-byte（vs G14.0 ref committed diff 闭集）。
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
    # 异己登记面（G13 立项裁决 1 继承）：g12_pt_sampler_selection.json 工作树豁免位。
    working_allowed = {"milestones/g12/g12_pt_sampler_selection.json"}
    bad_committed = [f for f in committed if f not in ALLOWED_CLOSED_DIFF]
    bad_working = [f for f in working if f not in working_allowed]
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
            _fact("g13_locked_registries_0byte", False, "selftest 空目录"),
            _fact("m_a_m_b_red_arms_independently_effective", False, "selftest 空目录"),
            _fact("budget_entries_measured", False, "selftest 空目录"),
            _fact("m_a_anchor_fulfilled", False, "selftest 空目录"),
            _fact("m_b_three_run_and_overhead", False, "selftest 空目录"),
            _fact("legacy_criteria_0byte", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G14.2 M-a(M172) registry variance band reconciliation (step 250) + M-b(M173) UE benchmark arm measurement (step 251)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: g13 registries 0-byte + red arms + g14_budget measured + M-a anchor + M-b three-run/overhead + legacy 0-byte",
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
    """① 缺双门 evidence → 红;② 真树聚合 VERDICT == 子门实测态（遮蔽即自检红）。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g14_wave2_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G14.2 wave2.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
