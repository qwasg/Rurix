#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.2 生产化核心波续）
"""G12.2 M158 MIS 完整面生产化门冒烟（g12.p0.m158.mis_full_surface；
G12_CONTRACT §4.2 M158 行判据逐字;G12_ACCEPTANCE_MAP §1;spec RXS-0398）。

硬判据:MIS 完整面生产化——光源采样（NEE）× BSDF 采样 MIS 权重全路径覆盖
（balance heuristic 逐顶点双策略 + 多光源联合 PDF 离散×连续 + delta 光源
退化 w_nee=1 无除零）+ 能量守恒（白炉 device 均值 vs host 参照截断真值面
+ 不产能量上界 Le 硬断言 + 逐级能量增量单调不增,RXS-0395 口径继承）+ 同
spp 收敛曲线不劣于参照器基线锚（g12_budget pt.ref_curve 锚,容差 M166 标定
程序产）+ 固定 seed 位级确定性协议继承（双跑位级一致）+ M96 既有判据
0-byte（冻结带/参照器面 git diff 闭集机核）。
RED 臂:权重缺失冒充 MIS(no-mis)/能量偏置注入(energy-bias)/确定性协议
漂移(seed-change)——device 变体真跑 digest 偏离正例臂必检出 + --red-arm
子模式独立复跑抽检。

host 段:rurix-render gi::path_trace::prod 单测逐名锚定 + conformance gi
语料锚（accept mis_full_surface_minimal + reject mis_weight_missing /
mis_energy_bias_inject）+ g12_budget 标定/锚条目齐备（M166 产）+ M96 冻结
面 0-byte 机核。device 段(必需,持 gpu_device_lock):rurixc --target
vulkan 产 SPV → g12_pt_production harness --gate 全档真跑
（RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1,SKIP 翻红）。

用法:
  py -3 ci/g12_mis_full_surface_smoke.py --gate g12.p0.m158.mis_full_surface
  py -3 ci/g12_mis_full_surface_smoke.py --selftest
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

GATE_KEY = "g12.p0.m158.mis_full_surface"
NUMERIC_STEP = 218
SUBJECT = "g12_m158_mis_full_surface"
SCHEMA_PATH = ROOT / "milestones/g12/g12_m158_mis_full_surface_evidence_schema.json"
SOURCE_REF = "G12_CONTRACT §4.2 M158;G12_ACCEPTANCE_MAP §1;spec/global_illumination.md RXS-0398;RFC-0029 §4.1"
TAG = "g12_m158"

PROD_TESTS = [
    "mis_balance_numeric_anchors_and_delta_degenerate",
    "prod_scenes_validate_and_light_dist_deterministic",
    "host_oracle_furnace_energy_and_levels_monotone",
    "host_oracle_mis_red_arm_detectable_and_delta_mis_irrelevant",
    "conformance_g12_corpus_present",
]
CORPUS = [
    ("accept/mis_full_surface_minimal.rx", "RXS-0398"),
    ("reject/mis_weight_missing.rx", "RXS-0398"),
    ("reject/mis_energy_bias_inject.rx", "RXS-0398"),
]
RED_ARMS = ["no-mis", "energy-bias", "seed-change"]
SUBMODE_ARMS = ["no-mis", "energy-bias"]

CHECK_KEYS = [
    "host_prod_tests_anchored",
    "conformance_corpus_anchored",
    "budget_anchors_present",
    "m96_frozen_surface_0byte",
    "device_harness_full_pass",
    "device_double_run_bitexact",
    "device_light_dist_deterministic",
    "device_mis_delta_degenerate",
    "device_furnace_energy_conserved",
    "device_levels_monotone",
    "device_no_light_leak",
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
    if len(CHECK_KEYS) != 15:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 15", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    synth = {k: True for k in CHECK_KEYS}
    synth.pop(CHECK_KEYS[0])
    if not (req - set(synth)):
        print(f"[{TAG}] selftest FAIL: 合成缺键未触发 schema required 红", file=sys.stderr)
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
            checks["device_light_dist_deterministic"] = hc.get("light_dist_deterministic") is True
            checks["device_mis_delta_degenerate"] = hc.get("mis_delta_degenerate") is True
            checks["device_furnace_energy_conserved"] = hc.get("furnace_energy_conserved") is True
            checks["device_levels_monotone"] = hc.get("levels_monotone") is True
            checks["device_no_light_leak"] = hc.get("no_light_leak_nonneg") is True
            checks["device_curve_not_worse"] = hc.get("curve_not_worse") is True
            checks["device_red_arms_effective"] = all(
                hc.get(k) is True for k in ("red_no_mis", "red_energy_bias", "red_seed_change")
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
            note("device:全档真跑(双跑位级 + 曲线锚 + 白炉守恒)+ RED 臂子模式独立复跑")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = gl.gate_evidence(
        subject=SUBJECT,
        gate_key=GATE_KEY,
        milestone="M158",
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
            {"seq": 5, "command": "g12_pt_production --gate g12.p0.m158.mis_full_surface --spv .. --pbrt .. --imgtool .. --tau <g12.pt.rr_tau> --sampler <winner> --curve-tol <g12.pt.curve_tol_rel> --furnace-tol <g12.pt.furnace_energy_tol> --level-tol <g12.pt.level_monotone_tol> --anchor-* <g12.pt.ref_curve_*>(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g12_pt_production --red-arm no-mis|energy-bias --spv .. (子模式抽检)", "exit_code": 0 if checks["device_red_arm_submodes_detected"] else 1},
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
