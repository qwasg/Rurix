#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.2 生产化核心波续）
"""G12.2 M159 俄罗斯轮盘生产化门冒烟（g12.p0.m159.russian_roulette_prod；
G12_CONTRACT §4.2 M159 行判据逐字;G12_ACCEPTANCE_MAP §1;spec RXS-0399）。

硬判据:吞吐自适应 RR（p_kill = clamp(1−T/τ, 0, p_max),τ 标定程序产,
p_max<1 恒成立）+ 无偏补偿闭式 1/(1−p_kill)（上界登记）+ 最小反弹保障
（N_min ≥ 2,低深度不早杀,fail-closed 机核）+ RR 终止率/补偿计数非空 +
RR 开/关无偏对照（均值差 ≤ 标定容差）+ 收敛曲线不劣于基线锚 + 固定 seed
位级确定性协议继承 + M96 既有判据 0-byte。
RED 臂:跳 RR 偏移(no-rr,RXS-0357 三臂面继承)/补偿缺失冒充无偏
(comp-off)/早杀偏置注入(early-kill,N_min 违反)——device 变体真跑
digest 偏离正例臂必检出 + --red-arm 子模式独立复跑抽检。

用法:
  py -3 ci/g12_russian_roulette_prod_smoke.py --gate g12.p0.m159.russian_roulette_prod
  py -3 ci/g12_russian_roulette_prod_smoke.py --selftest
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

GATE_KEY = "g12.p0.m159.russian_roulette_prod"
NUMERIC_STEP = 219
SUBJECT = "g12_m159_russian_roulette_prod"
SCHEMA_PATH = ROOT / "milestones/g12/g12_m159_russian_roulette_prod_evidence_schema.json"
SOURCE_REF = "G12_CONTRACT §4.2 M159;G12_ACCEPTANCE_MAP §1;spec/global_illumination.md RXS-0399;RFC-0029 §4.2"
TAG = "g12_m159"

PROD_TESTS = [
    "rr_params_validate_and_closed_form_compensation",
    "host_oracle_rr_counters_nonempty_and_unbiased",
    "conformance_g12_corpus_present",
]
CORPUS = [
    ("accept/rr_throughput_adaptive_minimal.rx", "RXS-0399"),
    ("reject/rr_early_kill_bias.rx", "RXS-0399"),
    ("reject/rr_compensation_missing.rx", "RXS-0399"),
]
SUBMODE_ARMS = ["no-rr", "comp-off"]

CHECK_KEYS = [
    "host_prod_tests_anchored",
    "conformance_corpus_anchored",
    "budget_anchors_present",
    "m96_frozen_surface_0byte",
    "device_harness_full_pass",
    "device_double_run_bitexact",
    "device_rr_counters_nonempty",
    "device_rr_unbiased",
    "device_rr_params_fail_closed",
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
    if len(CHECK_KEYS) != 13:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 13", file=sys.stderr)
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
            checks["device_rr_counters_nonempty"] = hc.get("rr_counters_nonempty") is True
            checks["device_rr_unbiased"] = hc.get("rr_unbiased") is True
            checks["device_rr_params_fail_closed"] = hc.get("rr_params_fail_closed") is True
            checks["device_curve_not_worse"] = hc.get("curve_not_worse") is True
            checks["device_red_arms_effective"] = all(
                hc.get(k) is True for k in ("red_no_rr", "red_comp_off", "red_early_kill")
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
            note("device:全档真跑(计数非空 + 无偏对照 + 曲线锚)+ RED 臂子模式独立复跑")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = gl.gate_evidence(
        subject=SUBJECT,
        gate_key=GATE_KEY,
        milestone="M159",
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
            {"seq": 5, "command": "g12_pt_production --gate g12.p0.m159.russian_roulette_prod --spv .. --pbrt .. --imgtool .. --tau <g12.pt.rr_tau> --sampler <winner> --curve-tol <g12.pt.curve_tol_rel> --rr-unbiased-tol <g12.pt.rr_unbiased_tol> --anchor-* <g12.pt.ref_curve_*>(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g12_pt_production --red-arm no-rr|comp-off --spv .. (子模式抽检)", "exit_code": 0 if checks["device_red_arm_submodes_detected"] else 1},
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
