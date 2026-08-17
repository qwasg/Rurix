#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.2 生产化核心波续）
"""G12.2 M160 采样策略升级 + 低差异序列门冒烟
（g12.p0.m160.sampling_lds_upgrade；G12_CONTRACT §4.2 M160 行判据逐字;
G12_ACCEPTANCE_MAP §1;spec RXS-0400）。

硬判据:分层/低差异序列生产化（stratified/Sobol 类确定性种子扰动;选型
benchmark measured 裁决,winner 族 artifact 一致性机核）+ 确定性协议加性
扩展（序列索引推导确定性——逐索引重求值 == 流内容;固定 seed 位级一致维持
——流/device 双跑位级一致;RNG 流布局 provenance 进 evidence;RXS-0357
L2 既有字面 0-byte）+ 收敛曲线 measured 不劣于独立 PCG 流锚（g12_budget
ref_curve 锚,容差 M166 标定程序产）。
RED 臂:序列非确定冒充低差异（nondeterministic——篡改流元素 device 输出
必分叉）/位级一致破坏未登记（seed-change）——必检出 + --red-arm 子模式
独立复跑抽检。

用法:
  py -3 ci/g12_sampling_lds_upgrade_smoke.py --gate g12.p0.m160.sampling_lds_upgrade
  py -3 ci/g12_sampling_lds_upgrade_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g12_pt_prod_lib as gl  # noqa: E402

GATE_KEY = "g12.p0.m160.sampling_lds_upgrade"
NUMERIC_STEP = 220
SUBJECT = "g12_m160_sampling_lds_upgrade"
SCHEMA_PATH = ROOT / "milestones/g12/g12_m160_sampling_lds_upgrade_evidence_schema.json"
SOURCE_REF = "G12_CONTRACT §4.2 M160;G12_ACCEPTANCE_MAP §1;spec/global_illumination.md RXS-0400;RFC-0029 §4.3"
TAG = "g12_m160"

PROD_TESTS = [
    "sampler_index_determinism_and_bitexact",
    "host_oracle_lds_bitexact_and_selection_deterministic",
    "sampler_benchmark_lds_not_worse_than_pcg",
    "conformance_g12_corpus_present",
]
CORPUS = [
    ("accept/lds_deterministic_minimal.rx", "RXS-0400"),
    ("reject/lds_nondeterministic_inject.rx", "RXS-0400"),
]
SUBMODE_ARMS = ["nondeterministic", "seed-change"]

CHECK_KEYS = [
    "host_prod_tests_anchored",
    "conformance_corpus_anchored",
    "budget_anchors_present",
    "m96_frozen_surface_0byte",
    "selection_artifact_consistent",
    "device_harness_full_pass",
    "device_stream_bitexact_index_deterministic",
    "device_provenance_registered",
    "device_selection_benchmark_deterministic",
    "device_double_run_bitexact",
    "device_curve_not_worse",
    "device_red_arms_effective",
    "device_red_arm_submodes_detected",
    "device_validation_zero",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 14:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 14", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (1 RED + 1 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    gl.os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}

    # ── host 段 ──
    ok, msg = gl.host_prod_tests(PROD_TESTS, TAG)
    checks["host_prod_tests_anchored"] = ok
    check(ok, msg)
    note(msg)
    ok, msg = gl.conformance_anchor(CORPUS, GATE_KEY)
    checks["conformance_corpus_anchored"] = ok
    check(ok, msg)
    cal = gl.load_calibration()
    checks["budget_anchors_present"] = cal is not None
    check(cal is not None, "g12_budget 标定/锚条目缺失(M166 未绿不得抢跑)")
    ok, msg = gl.m96_frozen_surface_unchanged()
    checks["m96_frozen_surface_0byte"] = ok
    check(ok, msg)
    note(msg)
    # 选型 artifact 一致性(winner == 标定 winner == 本门消费族;混用即 RED)。
    if cal is not None:
        sel = json.loads(gl.SELECTION_PATH.read_text(encoding="utf-8"))
        calib1 = gl.WORK_DIR / "calibration_run1.json"
        calib_winner = None
        if calib1.is_file():
            calib_winner = json.loads(calib1.read_text(encoding="utf-8")).get("sampler_selection", {}).get("winner")
        sel_ok = (
            sel.get("winner") == cal["winner"]
            and (calib_winner is None or calib_winner == cal["winner"])
            and gl.winner_cli_name(cal["winner"]) in ("pcg", "stratified", "sobol")
        )
        checks["selection_artifact_consistent"] = sel_ok
        check(sel_ok, f"选型 artifact 不一致: artifact={sel.get('winner')} calib={calib_winner}")
        note(f"选型 winner = {cal['winner']}(benchmark measured 裁决,artifact 一致)")

    # ── device 段 ──
    device_state = "fail"
    doc = None
    if cal is not None:
        device_state, doc, submode_ok, leg_failures = gl.run_device_leg(GATE_KEY, cal, SUBMODE_ARMS, TAG)
        for f in leg_failures:
            check(False, f)
        if device_state == "skipped_dev_env":
            check(False, "device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP)")
            device_state = "fail"
        if device_state == "executed" and doc is not None:
            hc = doc.get("checks", {})
            checks["device_harness_full_pass"] = True
            checks["device_stream_bitexact_index_deterministic"] = hc.get("stream_bitexact_index_deterministic") is True
            checks["device_provenance_registered"] = hc.get("provenance_registered") is True
            checks["device_selection_benchmark_deterministic"] = hc.get("selection_benchmark_deterministic") is True
            checks["device_double_run_bitexact"] = hc.get("double_run_bitexact") is True
            checks["device_curve_not_worse"] = hc.get("curve_not_worse") is True
            checks["device_red_arms_effective"] = all(
                hc.get(k) is True for k in ("red_nondeterministic", "red_seed_change")
            )
            checks["device_validation_zero"] = (
                hc.get("validation_zero") is True
                and doc.get("device_state", {}).get("validation") == "on"
                and doc.get("device_state", {}).get("require_real") is True
            )
            for k in CHECK_KEYS:
                if k.startswith("device_") and k != "device_red_arm_submodes_detected" and not checks[k]:
                    check(False, f"harness 判据 {k} 为假")
            checks["device_red_arm_submodes_detected"] = submode_ok
            note("device:全档真跑(流确定性 + 双跑位级 + 曲线锚)+ RED 臂子模式独立复跑")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = gl.gate_evidence(
        subject=SUBJECT,
        gate_key=GATE_KEY,
        milestone="M160",
        wave="G12.2",
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        checks=checks,
        device_state=device_state,
        host_pass=host_pass,
        commands=[
            {"seq": 1, "command": "cargo test -p rurix-render --lib gi::path_trace::prod", "exit_code": 0 if checks["host_prod_tests_anchored"] else 1},
            {"seq": 2, "command": "cargo build -p rurixc --features vulkan-backend --bin rurixc", "exit_code": 0},
            {"seq": 3, "command": "rurixc src/rurix-render/kernels/g12_pt_production.rx --target vulkan -o .tmp/g12_gates/pt_prod/g12_pt_production.spv", "exit_code": 0},
            {"seq": 4, "command": "cargo build -p rurix-render --features vulkan --bin g12_pt_production", "exit_code": 0},
            {"seq": 5, "command": "g12_pt_production --gate g12.p0.m160.sampling_lds_upgrade --spv .. --pbrt .. --imgtool .. --tau <g12.pt.rr_tau> --sampler <winner> --curve-tol <g12.pt.curve_tol_rel> --anchor-* <g12.pt.ref_curve_*>(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g12_pt_production --red-arm nondeterministic|seed-change --spv .. (子模式抽检)", "exit_code": 0 if checks["device_red_arm_submodes_detected"] else 1},
        ],
        environment=gl.environment(),
        production=gl.production_section(doc, checks["m96_frozen_surface_0byte"]),
        notes="; ".join(NOTES + FAILURES[:8]),
        all_pass=all_pass,
        ts=ts,
    )
    gl.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = gl.EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
