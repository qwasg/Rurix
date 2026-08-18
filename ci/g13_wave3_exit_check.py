#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.3 TSR device 化波）
"""G13.3 波次聚合门 g13.wave.3.exit（步骤 239；G13_CONTRACT G-G13-5/§2.2；
G13_ACCEPTANCE_MAP §1；spec/display_pipeline.md RXS-0404；同构
ci/g13_wave2_exit_check.py + ci/g13_wave_exit_lib.py）。

只读汇总 G13.3 波 M-b(M168) 门最新 evidence——自研 TSR device 化（步骤 238，
resample/resolve 双 .rx kernel SPV + spirv-val + device vs host 逐帧对拍 +
三档质量/帧时 measured 对照）——+ 六 facts:
① temporal 底座 0-byte（UpscaleBackend trait 签名面与 temporal 底座历史接口
   面 vs G13.0 不可变 ref 8c5dc5ee 目录级 git diff + 工作树双面机核）;
② M-b 门 RED 臂独立有效（最新 evidence red 面 checks 全真——kernel-bias /
   seed-change 双臂）;
③ g13_budget M-b 标定/帧时七条目齐备 measured_local 零 estimated +
   budget_eval 全 PASS（P-09 禁手写;帧时条目携带不设通过线字面）;
④ 双 kernel 源 `//@ spec: RXS-0404` 锚定 + conformance accept/reject 语料
   齐备 + trace_matrix 全锚定 PASS;
⑤ M-b 门最新 evidence 三档 device 判据全真（device vs host 对拍在容差内 /
   三档 deficit 在冻结带 / 收敛单调 / 双跑位级 / validation 零命中）;
⑥ 帧时条目 zero_pass_line 登记（三档帧时条目描述逐个携带「不构成帧率对标
   通过线」字面——正式帧率对标锚定 G14，以基线冒充帧率对标即 RED）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g13_wave3_exit_check.py --gate g13.wave.3.exit
  py -3 ci/g13_wave3_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g13_tsr_device_kernel_smoke as mb  # noqa: E402
import g13_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g13.wave.3.exit"
NUMERIC_STEP = 239
SUBJECT = "g13_wave3_exit"
WAVE = "G13.3"
SOURCE_REF = (
    "G13_CONTRACT G-G13-5/§2.2;G13_ACCEPTANCE_MAP §1;spec/display_pipeline.md RXS-0404;"
    "M-b gate red arms independently effective;temporal base 0-byte;g13_budget M-b seven entries "
    "measured_local;kernel RXS-0404 anchors + conformance corpus;three-tier device criteria all true;"
    "frame-time entries zero_pass_line registered"
)
SCHEMA_PATH = ROOT / "milestones/g13/g13_wave3_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g13.p0.m_b.tsr_device_kernel", "g13_m_b_tsr_device_kernel"),
]

NO_PASS_LINE = "不构成帧率对标通过线"


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_facts() -> list[dict]:
    facts: list[dict] = []

    # ① temporal 底座 0-byte(提交面 + 工作树面)。
    ok, msg = mb.temporal_base_0byte()
    facts.append(_fact("temporal_base_0byte", ok, msg))

    # ② M-b 门 RED 臂独立有效(red 面 checks 全真)。
    red_bad: list[str] = []
    red_total = 0
    latest_doc: dict | None = None
    for _key, prefix in REQUIRED_GATES:
        path = wel.load_latest_evidence(prefix)
        if path is None:
            red_bad.append(f"{prefix} 缺最新 evidence")
            continue
        latest_doc = wel.load_json(path)
        red_checks = {
            k: v
            for k, v in (latest_doc.get("checks") or {}).items()
            if k.startswith("device_red_")
        }
        red_total += len(red_checks)
        if not red_checks or any(v is not True for v in red_checks.values()):
            red_bad.append(f"{prefix} red 面 checks 缺失或非真")
    facts.append(_fact(
        "m_b_red_arms_independently_effective",
        not red_bad,
        f"M-b 门最新 evidence red 面 checks 全真(共 {red_total} 臂独立有效)"
        if not red_bad else "; ".join(red_bad[:3]),
    ))

    # ③ g13_budget M-b 七条目齐备 measured_local + budget_eval 全 PASS。
    bud_bad: list[str] = []
    budget = mb.load_g13_budget()
    entry_ids = [eid for eid, *_ in mb.CALIB_ENTRY_REGISTRY] + [eid for eid, _t, _d in mb.BENCH_ENTRY_REGISTRY]
    if budget is None:
        bud_bad.append("g13_budget.json 缺失")
    else:
        for eid in entry_ids:
            e = mb.budget_entry(budget, eid)
            if e is None:
                bud_bad.append(f"缺条目 {eid}")
            elif e.get("evidence") != "measured_local":
                bud_bad.append(f"{eid} 非 measured_local")
    r = mb.run(["py", "-3", "ci/budget_eval.py"])
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "budget_calibration_entries_measured",
        not bud_bad,
        f"g13_budget M-b 七条目齐备 measured_local 零 estimated + budget_eval 全 PASS(P-09;共 {len(entry_ids)} 条目)"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ④ 双 kernel RXS-0404 锚定 + conformance 语料 + trace_matrix。
    ok_k, msg_k = mb.kernel_sources_anchored()
    ok_c, msg_c = mb.conformance_corpus_anchored()
    facts.append(_fact(
        "kernel_and_corpus_anchored",
        ok_k and ok_c,
        f"{msg_k};{msg_c}",
    ))

    # ⑤ M-b 门最新 evidence 三档 device 判据全真。
    dev_bad: list[str] = []
    if latest_doc is None:
        dev_bad.append("缺最新 evidence")
    else:
        for k in (
            "device_host_device_maxdiff_within_tol",
            "device_tier_deficit_band_within",
            "device_converge_monotonic",
            "device_double_run_bitexact",
            "device_validation_zero",
        ):
            if (latest_doc.get("checks") or {}).get(k) is not True:
                dev_bad.append(f"checks.{k} 非真")
    facts.append(_fact(
        "three_tier_device_criteria_true",
        not dev_bad,
        "M-b 门最新 evidence 三档 device 判据全真(对拍容差/冻结带/收敛单调/双跑位级/validation 零命中)"
        if not dev_bad else "; ".join(dev_bad[:3]),
    ))

    # ⑥ 帧时条目 zero_pass_line 登记(描述逐个携带不设通过线字面)。
    zpl_bad: list[str] = []
    if budget is None:
        zpl_bad.append("g13_budget.json 缺失")
    else:
        for eid, _tier, _desc in mb.BENCH_ENTRY_REGISTRY:
            e = mb.budget_entry(budget, eid)
            if e is None:
                zpl_bad.append(f"缺条目 {eid}")
            elif NO_PASS_LINE not in str(e.get("description", "")):
                zpl_bad.append(f"{eid} 描述缺不设通过线字面")
    facts.append(_fact(
        "frame_time_zero_pass_line_registered",
        not zpl_bad,
        "三档帧时条目描述逐个携带「不构成帧率对标通过线」字面(正式帧率对标锚定 G14)"
        if not zpl_bad else "; ".join(zpl_bad[:3]),
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        # selftest 负样本面:facts 全 FAIL(空树无参照面)。
        extras = [
            _fact("temporal_base_0byte", False, "selftest 空目录"),
            _fact("m_b_red_arms_independently_effective", False, "selftest 空目录"),
            _fact("budget_calibration_entries_measured", False, "selftest 空目录"),
            _fact("kernel_and_corpus_anchored", False, "selftest 空目录"),
            _fact("three_tier_device_criteria_true", False, "selftest 空目录"),
            _fact("frame_time_zero_pass_line_registered", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G13.3 M-b(M168) TSR device kernel gate (step 238)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: temporal base 0-byte + red arms + g13_budget seven entries measured + RXS-0404 anchors + three-tier device criteria + zero_pass_line",
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
    """① 缺 M-b 门 evidence → 红;② 真树聚合 VERDICT == 子门实测态(遮蔽即自检红)。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g13_wave3_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G13.3 wave3.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
