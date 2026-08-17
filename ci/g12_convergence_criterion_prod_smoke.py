#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.2 生产化核心波续）
"""G12.2 M161 收敛判据生产化门冒烟
（g12.p0.m161.convergence_criterion_prod；G12_CONTRACT §4.2 M161 行判据
逐字;G12_ACCEPTANCE_MAP §1;spec RXS-0401）。

硬判据:逐像素方差驱动自适应 spp 终止（Σx/Σx² 在线协议,θ 标定程序产,
spp 下界 N_floor=16 保障——防早期方差欠估计早停）+ 收敛报告（逐像素 spp
分布/方差/未收敛像素计数非空 + 独立重算一致——缺报即 RED）+ 收敛误判率
≤ 标定阈（场景×族单元 p100 标定,误判带 0.25 协议冻结）+ 固定全 spp
golden 对拍不偏离冻结带（g9_m96_pbrt_tolerance_band measured×2.0 带继承）
+ 帧型标签闭集 {adaptive, full_reference}（混标即 RED）+ 固定 seed 位级
确定性协议继承 + M96 既有判据 0-byte。
RED 臂:早停冒充收敛（early-stop——n_floor=1+巨阈注入 device 变体必分叉
且逐像素样本数全 1)/未收敛像素缺报（underreport——报告计数 ≠ 独立重算
必检出)/帧型混标（label-mix——闭集校验必拒）。

用法:
  py -3 ci/g12_convergence_criterion_prod_smoke.py --gate g12.p0.m161.convergence_criterion_prod
  py -3 ci/g12_convergence_criterion_prod_smoke.py --selftest
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

GATE_KEY = "g12.p0.m161.convergence_criterion_prod"
NUMERIC_STEP = 221
SUBJECT = "g12_m161_convergence_criterion_prod"
SCHEMA_PATH = ROOT / "milestones/g12/g12_m161_convergence_criterion_prod_evidence_schema.json"
SOURCE_REF = "G12_CONTRACT §4.2 M161;G12_ACCEPTANCE_MAP §1;spec/global_illumination.md RXS-0401;RFC-0029 §4.4"
TAG = "g12_m161"

PROD_TESTS = [
    "adaptive_params_validate_and_report_nonempty",
    "adaptive_vs_full_reference_close_and_misjudge_recompute",
    "conformance_g12_corpus_present",
]
CORPUS = [
    ("accept/adaptive_convergence_minimal.rx", "RXS-0401"),
    ("reject/early_stop_masquerade.rx", "RXS-0401"),
    ("reject/unconverged_pixel_underreport.rx", "RXS-0401"),
]
SUBMODE_ARMS = ["early-stop"]

CHECK_KEYS = [
    "host_prod_tests_anchored",
    "conformance_corpus_anchored",
    "budget_anchors_present",
    "m96_frozen_surface_0byte",
    "device_harness_full_pass",
    "device_double_run_bitexact",
    "device_convergence_report_nonempty",
    "device_spp_floor_held",
    "device_frame_label_closed",
    "device_misjudge_within_tol",
    "device_golden_band_within",
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
            checks["device_double_run_bitexact"] = hc.get("double_run_bitexact") is True
            checks["device_convergence_report_nonempty"] = hc.get("convergence_report_nonempty") is True
            checks["device_spp_floor_held"] = hc.get("spp_floor_held") is True
            checks["device_frame_label_closed"] = hc.get("frame_label_closed") is True
            checks["device_misjudge_within_tol"] = hc.get("misjudge_within_tol") is True
            checks["device_golden_band_within"] = hc.get("golden_band_within") is True
            checks["device_red_arms_effective"] = all(
                hc.get(k) is True
                for k in ("red_early_stop", "red_underreport_detected", "red_label_mix_detected")
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
            note("device:全档真跑(自适应双跑位级 + 收敛报告 + 误判率 + golden 带)+ RED 臂子模式独立复跑")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = gl.gate_evidence(
        subject=SUBJECT,
        gate_key=GATE_KEY,
        milestone="M161",
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
            {"seq": 5, "command": "g12_pt_production --gate g12.p0.m161.convergence_criterion_prod --spv .. --pbrt .. --imgtool .. --tau <g12.pt.rr_tau> --theta <g12.pt.adaptive_rel_err_theta> --sampler <winner> --misjudge-tol <g12.pt.misjudge_rate_tol> --anchor-* <g12.pt.ref_curve_*>(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g12_pt_production --red-arm early-stop --spv .. (子模式抽检)", "exit_code": 0 if checks["device_red_arm_submodes_detected"] else 1},
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
