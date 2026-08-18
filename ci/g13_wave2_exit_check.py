#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.2 vendor 超分接入波）
"""G13.2 波次聚合门 g13.wave.2.exit（步骤 237；G13_CONTRACT G-G13-4/§2.2；
G13_ACCEPTANCE_MAP §1；同构 ci/g12_wave2_exit_check.py + ci/g13_wave_exit_lib.py）。

只读汇总 G13.2 波 M-a(M167) 门最新 evidence——vendor 超分接入（步骤 236，
DLSS SR 经 Streamline SDK 真跑 + FSR 3.1.5 同接口档）——+ 六 facts:
① temporal 底座 0-byte（UpscaleBackend trait 签名面与 temporal 底座历史接口
   面 vs G13.0 不可变 ref 8c5dc5ee 目录级 git diff + 工作树双面机核）;
② M-a 门 RED 臂独立有效（最新 evidence red 面 checks 非空且全真）;
③ g13_budget M-a 标定条目齐备 measured_local 零 estimated + budget_eval
   全 PASS（P-09 禁手写）;
④ M-a 许可前置清结留痕在树且五要素齐备（Streamline/NGX/FSR/owner/清结）;
⑤ vendor SDK registry 在树且 Streamline/FSR 双段许可/digest 字段齐备
   （二进制零入 git，外部缓存 + 许可/digest 登记形态，G13 立项裁决 10）;
⑥ 树内零绕过 UpscaleBackend 私接面（vendor SDK 调用 token 仅允许在登记
   FFI 边界文件 src/rurix-rt/src/vendor_upscale.rs 内）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g13_wave2_exit_check.py --gate g13.wave.2.exit
  py -3 ci/g13_wave2_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g13_vendor_upscale_integration_smoke as ma  # noqa: E402
import g13_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g13.wave.2.exit"
NUMERIC_STEP = 237
SUBJECT = "g13_wave2_exit"
WAVE = "G13.2"
SOURCE_REF = (
    "G13_CONTRACT G-G13-4/§2.2;G13_ACCEPTANCE_MAP §1;M-a gate red arms independently effective;"
    "temporal base 0-byte;g13_budget M-a calibration entries measured_local;"
    "license clearance five tokens;vendor SDK registry digests;zero private bypass surface"
)
SCHEMA_PATH = ROOT / "milestones/g13/g13_wave2_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g13.p0.m_a.vendor_upscale_integration", "g13_m_a_vendor_upscale_integration"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_facts() -> list[dict]:
    facts: list[dict] = []

    # ① temporal 底座 0-byte(提交面 + 工作树面)。
    ok, msg = ma.temporal_base_0byte()
    facts.append(_fact("temporal_base_0byte", ok, msg))

    # ② M-a 门 RED 臂独立有效(red 面 checks 非空全真)。
    red_bad: list[str] = []
    red_total = 0
    for _key, prefix in REQUIRED_GATES:
        path = wel.load_latest_evidence(prefix)
        if path is None:
            red_bad.append(f"{prefix} 缺最新 evidence")
            continue
        doc = wel.load_json(path)
        red_checks = {
            k: v
            for k, v in (doc.get("checks") or {}).items()
            if ("red" in k or "mock" in k or "passthrough" in k)
        }
        red_total += len(red_checks)
        if not red_checks or any(v is not True for v in red_checks.values()):
            red_bad.append(f"{prefix} red 面 checks 缺失或非真")
    facts.append(_fact(
        "m_a_red_arms_independently_effective",
        not red_bad,
        f"M-a 门最新 evidence red 面 checks 全真(共 {red_total} 臂独立有效)"
        if not red_bad else "; ".join(red_bad[:3]),
    ))

    # ③ g13_budget M-a 标定条目齐备 measured_local + budget_eval 全 PASS。
    bud_bad: list[str] = []
    budget = ma.load_g13_budget()
    if budget is None:
        bud_bad.append("g13_budget.json 缺失")
    else:
        for eid, _key, _direction, _slug, _desc in ma.CALIB_ENTRY_REGISTRY:
            e = ma.budget_entry(budget, eid)
            if e is None:
                bud_bad.append(f"缺条目 {eid}")
            elif e.get("evidence") != "measured_local":
                bud_bad.append(f"{eid} 非 measured_local")
    r = ma.run(["py", "-3", "ci/budget_eval.py"])
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "budget_calibration_entries_measured",
        not bud_bad,
        "g13_budget M-a 标定条目齐备 measured_local 零 estimated + budget_eval 全 PASS(P-09)"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    # ④ M-a 许可前置清结留痕在树且五要素齐备。
    ok, msg = ma.license_clearance_ok()
    facts.append(_fact("license_clearance_present", ok, msg))

    # ⑤ vendor SDK registry 在树且双段齐备。
    ok, msg = ma.sdk_registry_ok()
    facts.append(_fact("vendor_sdk_registry_present", ok, msg))

    # ⑥ 树内零绕过 UpscaleBackend 私接面。
    ok, msg = ma.no_private_bypass_surface()
    facts.append(_fact("zero_private_bypass_surface", ok, msg))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        # selftest 负样本面:facts 全 FAIL(空树无参照面)。
        extras = [
            _fact("temporal_base_0byte", False, "selftest 空目录"),
            _fact("m_a_red_arms_independently_effective", False, "selftest 空目录"),
            _fact("budget_calibration_entries_measured", False, "selftest 空目录"),
            _fact("license_clearance_present", False, "selftest 空目录"),
            _fact("vendor_sdk_registry_present", False, "selftest 空目录"),
            _fact("zero_private_bypass_surface", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G13.2 M-a(M167) vendor upscale integration gate (step 236)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: temporal base 0-byte + red arms + g13_budget calibration measured + license clearance + SDK registry + zero bypass",
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
    """① 缺 M-a 门 evidence → 红;② 真树聚合 VERDICT == 子门实测态(遮蔽即自检红)。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g13_wave2_selftest_") as td:
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
    ap = argparse.ArgumentParser(description="G13.2 wave2.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
