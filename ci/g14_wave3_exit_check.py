#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.3 Rurix 管线性能波）
"""G14.3 波次聚合门 g14.wave.3.exit（步骤 255；G14_CONTRACT G-G14-5/§2.2；
G14_ACCEPTANCE_MAP §1；同构 ci/g14_wave2_exit_check.py）。

只读汇总 G14.3 波 M-c(M174) 门最新 evidence——Rurix 生产管线性能面（步骤
253，DeviceFrameSession 持久车道 + 三轮进程级独立运行 measured + 倒挂消除 +
优化前后对照）——+ 六 facts:
① M-c 门 RED 臂独立有效（最新 evidence red 面 checks 非空且全真——
   kernel-tamper/seed-change/one-shot-masquerade）;
② g14_budget M-c 18 条目 + 画质锚守护位齐备 measured_local 零 estimated +
   budget_eval 全 PASS（P-09 禁手写）;
③ 倒挂消除 + 优化前后对照 measured 面（t67≤t100 全序正常 + G13.3 基线逐档
   对照行非空）;
④ 固定 seed 双跑位级一致 + temporal 底座 0-byte（vs G14.0 ref f4c8da0b）;
⑤ G13.4 车道画质锚带守护复核（锚定 budget 位在树且当次复核 PASS）;
⑥ G5~G13 closed 面 0-byte（vs G14.0 ref committed diff 闭集 ⊆ 授权面）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g14_wave3_exit_check.py --gate g14.wave.3.exit
  py -3 ci/g14_wave3_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g13_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g14.wave.3.exit"
NUMERIC_STEP = 255  # 落盘前实测 registry/number_ledger.json CI_step.next_free=255 顺位领取
SUBJECT = "g14_wave3_exit"
WAVE = "G14.3"
SOURCE_REF = (
    "G14_CONTRACT G-G14-5/§2.2;G14_ACCEPTANCE_MAP §1;M-c gate red arms independently effective;"
    "g14_budget M-c entries measured_local;inversion eliminated + before/after measured;"
    "double-run bitexact + temporal base 0-byte;G13.4 lane quality anchor;G5~G13 closed 0-byte"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_wave3_exit_evidence_schema.json"
BUDGET_PATH = ROOT / "milestones" / "g14" / "g14_budget.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g14.p0.m_c.rurix_pipeline_perf", "g14_m_c_rurix_pipeline_perf"),
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

    # ① RED 臂独立有效
    red_checks = {k: v for k, v in checks.items() if "red" in k}
    facts.append(_fact(
        "m_c_red_arms_independently_effective",
        bool(red_checks) and all(v is True for v in red_checks.values()),
        f"M-c 门 red 面 checks 全真（{len(red_checks)} 臂）" if red_checks else "缺 red 面",
    ))

    # ② budget 条目齐备 + budget_eval 全 PASS
    bud_bad: list[str] = []
    if not BUDGET_PATH.is_file():
        bud_bad.append("g14_budget.json 缺失")
    else:
        budget = wel.load_json(BUDGET_PATH)
        got = {e.get("id"): e for e in (budget.get("entries") or [])}
        want = {f"g14.pipeline_perf.frame_ms.{s}_t{t}_{b}"
                for s in ("cornell-box", "bistro-interior") for t in (50, 67, 100)
                for b in ("tsr_device", "dlss_sr", "fsr_3_1_5")}
        want.add("g14.pipeline_perf.quality_anchor_ssim_deficit")
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
        "budget_entries_measured",
        not bud_bad,
        "g14_budget M-c 18 条目 + 画质锚守护位齐备 measured_local + budget_eval 全 PASS"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ③ 倒挂消除 + 优化前后对照
    ba = parity.get("before_after") or []
    ok3 = checks.get("inversion_eliminated") is True and bool(ba) and all(
        r.get("improvement_rel") is not None for r in ba)
    ba_txt = "; ".join(
        "t{}: {:.0f}→{:.1f}ms".format(r["tier"], r["g13_3_baseline_ms"], r["g14_3_lane_ms"])
        for r in ba)
    facts.append(_fact(
        "inversion_eliminated_and_before_after",
        ok3,
        f"倒挂消除 + 优化前后对照 {len(ba)} 档（{ba_txt}）"
        if ok3 else "倒挂/对照面缺",
    ))

    # ④ 双跑位级 + temporal 底座 0-byte
    ok4 = checks.get("double_run_bitexact") is True and checks.get("temporal_base_0byte") is True
    anchor = parity.get("bitexact_anchor") or {}
    facts.append(_fact(
        "bitexact_and_temporal_0byte",
        ok4,
        f"双跑位级 digest={str(anchor.get('digest'))[:32]}… + temporal 底座 0-byte"
        if ok4 else "位级/底座面缺",
    ))

    # ⑤ G13.4 车道画质锚带守护复核
    ok5 = checks.get("quality_parity_anchor") is True
    facts.append(_fact(
        "quality_anchor_guard",
        ok5,
        f"画质锚带守护复核 PASS（{parity.get('quality_anchor', '')[:80]}）"
        if ok5 else "画质锚带复核非绿",
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
            _fact("m_c_red_arms_independently_effective", False, "selftest 空目录"),
            _fact("budget_entries_measured", False, "selftest 空目录"),
            _fact("inversion_eliminated_and_before_after", False, "selftest 空目录"),
            _fact("bitexact_and_temporal_0byte", False, "selftest 空目录"),
            _fact("quality_anchor_guard", False, "selftest 空目录"),
            _fact("legacy_criteria_0byte", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G14.3 M-c(M174) rurix pipeline perf gate (step 253)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: red arms + budget measured + inversion/before-after + bitexact/temporal + quality anchor + legacy 0-byte",
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
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g14_wave3_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G14.3 wave3.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
