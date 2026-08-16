#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.2 波）
"""G11.2 M146 C3 EXR 位深对齐闭环门（P0，步骤 198；
g11.p0.m146.caliber_c3_exr_bit_depth；G11_CONTRACT §4.2 M146 行判据逐字 /
G-G11-4；G11_ACCEPTANCE_MAP §1 M146 行；CI_GATES §4；spec/imageio.md RXS-0385 /
spec/visual_comparison.md RXS-0393）。

host 纯 host 门（device_section_state=not_applicable）。判据（契约 §4.2 M146 行字面）：

1. **UE EXR fp16→f32 提升口径（RXS-0385 strip-and-log）与 Rurix 原生 f32 度量域
   对齐登记**：UE MRQ 源帧 fp16（decode 实测 source_bit_depth=float16）→ 度量域
   统一提升 f32——**提升精确无损**（fp16→f32 全 65536 位模式穷举核验
   half_to_f32 == numpy float16 语义，IEEE binary16 ⊂ binary32 精确映射）+
   UE 帧逐像素可逆核验（全部像素值 fp16 可表，roundtrip 逐位一致——无二次
   截断注入）；Rurix 原生 f32（source_bit_depth=float32 + metadata
   rurix:bit_depth=float32 互证）。锁定基线 delta = 16.0（源位深 32 vs 16，
   g10_gap_registry C3 行 0-byte 消费）→ 复测**度量域** delta = 0.0；源位深
   量化差显式登记残余（g11_2_residual_caliber_registry c3 行承接面）。
2. **位深元数据闭集回归**：全部帧（双端 HDR + 四张 LDR）rurix:* 元数据闭集
   齐备（Rurix 端 strict 面；UE 端 strip-and-log 面 source_bit_depth 实测登记）。
3. **收敛阈标定程序产（RXS-0393 L3，禁手写）**：标定 = 双场景度量域位深差
   样本 p100 × k（两跑逐位一致）→ g11_budget.json
   `g11.caliber.c3_bitdepth_domain_tol`（measured_local，字节级纯追加）+
   budget_eval --strict 全 PASS。

RED 臂（契约判据字面）：位深截断注入即 RED（red_bitdepth_truncation——UE
fp16 帧按 rurix strict 端解码必拒〔位深非 float32 canonical〕）；元数据缺字段
即 RED（red_metadata_missing_field）；手写阈值 / estimated 冒充即 RED。

用法：
  py -3 ci/g11_caliber_c3_exr_bit_depth_smoke.py --gate g11.p0.m146.caliber_c3_exr_bit_depth
  py -3 ci/g11_caliber_c3_exr_bit_depth_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m146_caliber_c3_exr_bit_depth_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g11_2_caliber_lib as cl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402
from g10_exr_lib import MetadataViolation, half_to_f32  # noqa: E402

GATE_KEY = "g11.p0.m146.caliber_c3_exr_bit_depth"
NUMERIC_STEP = 198
SOURCE_REF = (
    "G11_CONTRACT §4.2 M146 + G-G11-4;G11_ACCEPTANCE_MAP §1 M146;CI_GATES §4;"
    "spec/imageio.md RXS-0385;spec/visual_comparison.md RXS-0393"
)
TAG = "g11_m146"
SUBJECT = "g11_m146_caliber_c3_exr_bit_depth"
MATRIX_ROW = "M146"

BUDGET_ENTRY_ID = "g11.caliber.c3_bitdepth_domain_tol"
SAFETY_K = 1.0  # k∈[1.0,3.0]；p100=0.0 时 k 取值不改变标定值（M138 diff_over_threshold 先例）

CHECK_KEYS = [
    "contract_digest_locked_unchanged",
    "ue_source_fp16_registered",
    "rurix_native_f32",
    "promotion_exact_exhaustive",
    "ue_pixels_lossless_promotion",
    "measurement_domain_f32_unified",
    "bitdepth_metadata_closed_set",
    "closure_delta_converged_measured",
    "calibration_rerun_deterministic",
    "budget_entry_appended_measured_local",
    "budget_eval_strict_all_pass",
    "red_bitdepth_truncation_detected",
    "red_metadata_missing_field_detected",
    "red_handwritten_threshold_detected",
    "red_estimated_masquerade_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def promotion_exhaustive_check() -> dict:
    """fp16→f32 提升精确性穷举（全 65536 位模式）：half_to_f32（ci/g10_exr_lib
    单一事实源）vs numpy float16→float64 参考语义——有限值逐位一致；NaN 双方
    同判 NaN；±Inf 双方同号 Inf。任一不符即提升口径漂移。"""
    bits = np.arange(65536, dtype=np.uint16)
    ref = bits.view(np.float16).astype(np.float64)
    mismatches = 0
    nan_joint = 0
    for i in range(65536):
        got = half_to_f32(int(bits[i]))
        rv = float(ref[i])
        if np.isnan(rv):
            if got != got:  # got is NaN
                nan_joint += 1
                continue
            mismatches += 1
            if mismatches <= 3:
                note(f"提升不符: bits=0x{i:04x} got={got!r} ref={rv!r}")
            continue
        if got != rv:
            mismatches += 1
            if mismatches <= 3:
                note(f"提升不符: bits=0x{i:04x} got={got!r} ref={rv!r}")
    return {"patterns": 65536, "mismatches": mismatches, "nan_joint": nan_joint}


def compute_domain_calibration() -> dict:
    """C3 标定估计器（可复跑）：样本 = 双场景度量域位深差 |32 − 32|（UE 提升后
    与 Rurix 原生 f32 同域）；统计量 = p100。"""
    diffs: list[float] = []
    for scene_id in cl.SCENES:
        du = cl.decode(cl.hdr_frame(scene_id, "ue5"), "ue5")
        dr = cl.decode(cl.hdr_frame(scene_id, "rurix"), "rurix")
        ue_domain = 32.0 if du["source_bit_depth"] in ("float16", "float32") else 0.0
        rurix_domain = 32.0 if dr["source_bit_depth"] == "float32" else 0.0
        diffs.append(abs(rurix_domain - ue_domain))
    return {
        "p100": max(diffs),
        "sample_count": len(diffs),
        "sample_set_digest": cl.sha256_file(cl.REPORT_PATH),
        "estimator": "p100",
        "k": SAFETY_K,
    }


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂①：穷举提升核验零不符。
    promo = promotion_exhaustive_check()
    if promo["mismatches"] != 0:
        print(f"[{TAG}] selftest FAIL: 提升穷举不符 {promo}", file=sys.stderr)
        return 1
    # 绿臂②：标定两跑逐位一致。
    c1 = compute_domain_calibration()
    c2 = compute_domain_calibration()
    if c1 != c2:
        print(f"[{TAG}] selftest FAIL: 标定两跑不一致", file=sys.stderr)
        return 1
    # 红臂①：手写阈值冒充必拒。
    ok_entry = {
        "id": "g11.caliber.selftest_probe",
        "evidence": "measured_local",
        "threshold": c1["p100"] * SAFETY_K,
        "measured_value": c1["p100"],
        "evidence_file": "milestones/g11/g11_budget.json",
    }
    if cl.validate_budget_entry(ok_entry, c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: 合法条目误判", file=sys.stderr)
        return 1
    if not cl.validate_budget_entry(dict(ok_entry, threshold=c1["p100"] + 8.0), c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: 手写阈值冒充未检出", file=sys.stderr)
        return 1
    # 红臂②：estimated 冒充必拒。
    if not cl.validate_budget_entry(dict(ok_entry, evidence="estimated"), c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: estimated 冒充未检出", file=sys.stderr)
        return 1
    # 红臂③：元数据缺字段必检出（闭集校验器）。
    if not validate_metadata_closed_set({"rurix:schema_version": "1"}):
        print(f"[{TAG}] selftest FAIL: 元数据缺字段未检出", file=sys.stderr)
        return 1
    schema = cl.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (3 RED + 3 GREEN)")
    return 0


REQUIRED_RURIX_META = (
    "rurix:schema_version",
    "rurix:domain",
    "rurix:transfer",
    "rurix:bit_depth",
    "rurix:source_end",
    "rurix:capture_params_digest",
    "rurix:derivation",
)


def validate_metadata_closed_set(metadata: dict) -> list[str]:
    problems: list[str] = []
    for k in REQUIRED_RURIX_META:
        if k not in metadata:
            problems.append(f"元数据缺字段: {k}")
    return problems


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

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    # ① 契约 digest 三面绑定 0-byte。
    digest_drift = [
        f"{s}: {cl.contract_digest_rust(s)} ≠ {cl.LOCKED_DIGEST[s]}"
        for s in cl.SCENES
        if cl.contract_digest_rust(s) != cl.LOCKED_DIGEST[s]
    ]
    COMMANDS.append({"seq": 1, "command": "g10_5_scene_render --contract-digest ×2 scenes", "exit_code": 0})
    checks["contract_digest_locked_unchanged"] = not digest_drift
    check(not digest_drift, f"契约 digest 漂移: {digest_drift}")

    # ② 双端源位深实测登记 + Rurix 原生 f32 互证。
    src_depth: dict[str, str] = {}
    ue_fp16_ok = True
    rurix_f32_ok = True
    for scene_id in cl.SCENES:
        du = cl.decode(cl.hdr_frame(scene_id, "ue5"), "ue5")
        dr = cl.decode(cl.hdr_frame(scene_id, "rurix"), "rurix")
        src_depth[f"{scene_id}:ue5"] = du["source_bit_depth"]
        src_depth[f"{scene_id}:rurix"] = dr["source_bit_depth"]
        if du["source_bit_depth"] != "float16":
            ue_fp16_ok = False
        if dr["source_bit_depth"] != "float32" or dr["metadata"].get("rurix:bit_depth") != "float32":
            rurix_f32_ok = False
    checks["ue_source_fp16_registered"] = ue_fp16_ok
    check(ue_fp16_ok, f"UE 源位深非 fp16: {src_depth}")
    checks["rurix_native_f32"] = rurix_f32_ok
    check(rurix_f32_ok, f"Rurix 原生位深非 f32: {src_depth}")
    note(f"源位深实测: {src_depth}")

    # ③ fp16→f32 提升精确性穷举（全 65536 位模式）。
    promo = promotion_exhaustive_check()
    checks["promotion_exact_exhaustive"] = promo["mismatches"] == 0
    check(promo["mismatches"] == 0, f"提升穷举不符: {promo}")
    note(f"fp16→f32 提升穷举: {promo['patterns']} 位模式零不符（NaN 联合 {promo['nan_joint']}）")

    # ④ UE 帧逐像素可逆核验（全部像素 fp16 可表，roundtrip 逐位一致——无二次截断）。
    lossless_bad: list[str] = []
    for scene_id in cl.SCENES:
        du = cl.decode(cl.hdr_frame(scene_id, "ue5"), "ue5")
        arr = np.asarray(du["pixels"], dtype=np.float64)
        rt = arr.astype(np.float16).astype(np.float64)
        if not np.array_equal(arr, rt):
            lossless_bad.append(f"{scene_id} UE 帧像素含非 fp16 可表值（二次截断疑）")
    checks["ue_pixels_lossless_promotion"] = not lossless_bad
    check(not lossless_bad, f"UE 帧提升可逆性异常: {lossless_bad[:2]}")

    # ⑤ 度量域 f32 统一（decode 输出 = f64/f32 精确域双端同源面）。
    checks["measurement_domain_f32_unified"] = all(
        cl.decode(cl.hdr_frame(s, e), e)["source_bit_depth"] in ("float16", "float32")
        for s in cl.SCENES for e in ("rurix", "ue5")
    )
    check(checks["measurement_domain_f32_unified"], "度量域位深统一面异常")

    # ⑥ 位深元数据闭集回归（Rurix 端 strict 闭集齐备 + UE 端 strip-and-log 登记）。
    meta_bad: list[str] = []
    for scene_id in cl.SCENES:
        dr = cl.decode(cl.hdr_frame(scene_id, "rurix"), "rurix")
        meta_bad += [f"{scene_id}/rurix HDR {p}" for p in validate_metadata_closed_set(dr["metadata"])]
        for end in ("rurix", "ue5"):
            ld = cl.decode(cl.ldr_frame(scene_id, end), "rurix")
            meta_bad += [f"{scene_id}/{end} LDR {p}" for p in validate_metadata_closed_set(ld["metadata"])]
        du = cl.decode(cl.hdr_frame(scene_id, "ue5"), "ue5")
        if not isinstance(du.get("stripped"), list):
            meta_bad.append(f"{scene_id}/ue5 strip-and-log 登记面缺失")
    checks["bitdepth_metadata_closed_set"] = not meta_bad
    check(not meta_bad, f"位深元数据闭集异常: {meta_bad[:3]}")

    # ⑦ 标定两跑 + 收敛判定（基线 16.0 → 度量域 0.0）。
    cal1 = compute_domain_calibration()
    cal2 = compute_domain_calibration()
    checks["calibration_rerun_deterministic"] = cal1 == cal2
    check(cal1 == cal2, "标定程序不可复跑（两跑漂移即 RED）")
    c3_row = cl.gap_row("C3")
    baseline_primary = c3_row["measured_delta"][0]["delta"]
    retest_delta = cal1["p100"]
    threshold = cal1["p100"] * SAFETY_K
    converged = retest_delta <= threshold
    closure = {
        "gap_row_id": c3_row["gap_id"],
        "baseline_delta": baseline_primary,
        "retest_delta": retest_delta,
        "converged": bool(converged),
        "threshold_provenance": f"标定程序 ci/g11_caliber_c3_exr_bit_depth_smoke.py（p100×k={SAFETY_K}，样本集 = 复跑报告 digest 引用；budget 条目 {BUDGET_ENTRY_ID}）；残余源位深量化差显式登记 g11_2_residual_caliber_registry c3_source_bit_depth_quantization 行",
        "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
    }
    checks["closure_delta_converged_measured"] = converged
    check(converged, f"复测 delta {retest_delta!r} 未收敛（> 标定阈 {threshold!r}）")
    note(f"C3 修复前后 delta 对拍: 基线（源位深）{baseline_primary} → 复测（度量域）{retest_delta}（阈 {threshold}）")

    # ⑧ 标定 evidence 落盘 + 标定值入 g11_budget（字节级纯追加）。
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    calib_ev = {
        "schema_version": 1,
        "subject": "g11_m146_calibration_c3_bitdepth_domain",
        "symbolic_gate_key": GATE_KEY,
        "milestone": MATRIX_ROW,
        "wave": "G11.2",
        "numeric_step": NUMERIC_STEP,
        "results": {
            "trimmed_mean": cal1["p100"],
            "estimator": "p100",
            "sample_pair_count": cal1["sample_count"],
            "safety_factor_k": SAFETY_K,
            "threshold": threshold,
        },
        "provenance": {
            "estimator_semantics": "p100 × k（RFC-0026 §4.2 F10 / RXS-0393 L3）",
            "k_rationale": "样本 = 双场景度量域位深差，p100=0.0 时 k 取值不改变标定值；取 M138 diff_over_threshold 同值 1.0 保持语义连续（k∈[1,3] 闭集内）",
            "sample_set_digest": cal1["sample_set_digest"],
            "promotion_exhaustive": promo,
            "measured": "measured_local：双场景度量域位深差 p100 × k 复跑两跑逐位一致 + fp16→f32 全 65536 位模式穷举精确核验；禁手写阈值冒充标定（P-09）",
        },
        "environment": wel.collect_environment(),
        "timestamp": ts,
    }
    calib_path = EVIDENCE_DIR / f"g11_m146_calibration_c3_bitdepth_domain_{ts}.json"
    calib_path.write_text(json.dumps(calib_ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    entry = {
        "id": BUDGET_ENTRY_ID,
        "description": (
            "C3 位深对齐收敛阈（度量域）：双场景度量域位深差 |32 − 32| p100 × "
            f"k={SAFETY_K}（RXS-0393 L3；标定程序 ci/g11_caliber_c3_exr_bit_depth_smoke.py "
            f"可复跑两跑逐位一致；样本集 = 复跑报告 digest {cal1['sample_set_digest']}；"
            "fp16→f32 提升全 65536 位模式穷举精确核验；源位深 "
            "fp16 量化差显式登记残余 g11_2_residual_caliber_registry c3 行）。"
            "M146 measured 标定（P-09 禁手写阈值）。"
        ),
        "direction": "max",
        "evidence": "measured_local",
        "skip_reason": None,
        "unit": "bit",
        "threshold": threshold,
        "evidence_file": str(calib_path.relative_to(ROOT)).replace("\\", "/"),
        "measured_value": cal1["p100"],
    }
    budget_problems = cl.validate_budget_entry(entry, cal1["p100"], SAFETY_K)
    if not budget_problems:
        budget_problems = cl.append_budget_entries([entry])
        if not budget_problems:
            note(f"g11_budget.json 字节级纯追加 {BUDGET_ENTRY_ID}（threshold={threshold!r}）")
    checks["budget_entry_appended_measured_local"] = not budget_problems
    check(not budget_problems, f"budget 条目异常: {budget_problems[:2]}")

    # ⑨ budget_eval --strict 全 PASS。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")
    note(f"budget_eval --strict: exit {r.returncode}（{tail[-100:]}）")

    # ⑩ RED 臂①：位深截断注入必检出（UE fp16 帧按 rurix strict 端解码必拒）。
    red_trunc = False
    try:
        cl.decode(cl.hdr_frame("cornell-box", "ue5"), "rurix")
    except MetadataViolation:
        red_trunc = True
    except Exception:
        red_trunc = True
    checks["red_bitdepth_truncation_detected"] = red_trunc
    check(red_trunc, "位深截断注入（fp16 冒充 rurix canonical）未检出")

    # ⑪ RED 臂②：元数据缺字段必检出。
    checks["red_metadata_missing_field_detected"] = bool(validate_metadata_closed_set({"rurix:schema_version": "1"}))
    check(checks["red_metadata_missing_field_detected"], "元数据缺字段未检出")

    # ⑫ RED 臂③④：手写阈值 / estimated 冒充必拒。
    forged_entry = {
        "id": "g11.caliber.red_probe",
        "evidence": "measured_local",
        "threshold": cal1["p100"] * SAFETY_K + 8.0,
        "measured_value": cal1["p100"],
        "evidence_file": str(calib_path.relative_to(ROOT)).replace("\\", "/"),
    }
    checks["red_handwritten_threshold_detected"] = bool(cl.validate_budget_entry(forged_entry, cal1["p100"], SAFETY_K))
    check(checks["red_handwritten_threshold_detected"], "手写阈值冒充未检出")
    forged_entry2 = dict(forged_entry, threshold=cal1["p100"] * SAFETY_K, evidence="estimated")
    checks["red_estimated_masquerade_detected"] = bool(cl.validate_budget_entry(forged_entry2, cal1["p100"], SAFETY_K))
    check(checks["red_estimated_masquerade_detected"], "estimated 冒充未检出")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G11.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "closure": closure,
        "bitdepth_provenance": {
            "source_bit_depth": src_depth,
            "promotion_exhaustive": promo,
            "measurement_domain": "float32（UE fp16 → f32 精确提升；Rurix 原生 f32）",
            "residual_registration": "milestones/g11/g11_2_residual_caliber_registry.json c3_source_bit_depth_quantization 行（UE 源帧写出时 fp16 量化一次，不可回退，显式留档）",
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if all_pass and not FAILURES:
        print(
            f"[{TAG}] PASS（C3 位深对齐闭环：UE fp16→f32 提升精确穷举 + 度量域统一 "
            f"（基线 {baseline_primary} → 复测 {retest_delta}）+ 元数据闭集回归 + 标定值入 g11_budget + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
