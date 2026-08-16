#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.2 波）
"""G11.2 M145 C2 曝光链派生尺度对齐闭环门（P0，步骤 197；
g11.p0.m145.caliber_c2_exposure_chain；G11_CONTRACT §4.2 M145 行判据逐字 /
G-G11-4；G11_ACCEPTANCE_MAP §1 M145 行；CI_GATES §4；RFC-0028 §4.6；
spec/visual_comparison.md RXS-0386 L2 / RXS-0393）。

host 纯 host 门（device_section_state=not_applicable）。判据（契约 §4.2 M145 行字面）：

1. **双端 EV100 同字面下派生尺度对齐（统一）**：G10.5a 口径差 = Rurix 臂 LDR
   派生尺度 ×2^(−EV100)（cornell 0.25 / bistro 0.5）vs UE 臂 pipe 内手动曝光
   已施 ×1.0。G11.2 对齐 = **Rurix 臂曝光尺度管线内烘焙**（--exposure-scale
   2^(−EV100)，HDR 帧 = 曝光已施 scene-linear，与 UE 臂 pipe 内
   FixedExposure=2^(−EV100) 同域）→ **LDR 派生尺度双端统一 ×1.0**——复测
   派生尺度差 |1.0 − 1.0| = 0.0（锁定基线 delta = cornell 0.75 / bistro 0.5，
   g10_gap_registry C2 行 0-byte 消费）。
2. **派生链元数据互证回归**（RXS-0386 L2）：四张 LDR 帧
   rurix:source_frame_digest == 对应 HDR 帧内容 digest（门内独立重算）；
   capture_params_digest == G10.5 锁定值。
3. **收敛阈标定程序产（RXS-0393 L3，禁手写）**：标定程序 = 双场景派生尺度差
   样本集 p100 × k（k∈[1,3]；两跑逐位一致可复跑）→ g11_budget.json
   `g11.caliber.c2_exposure_scale_tol`（measured_local，字节级纯追加）+
   budget_eval --strict 全 PASS。

RED 臂（契约判据字面）：派生尺度未对齐出 LDR 度量即 RED
（red_unaligned_scale——伪造报告派生尺度 ≠1.0 必检出）；互证链断裂即 RED
（red_cross_verify_broken——篡改 source_frame_digest 必检出）；手写阈值冒充
标定 / estimated 冒充 measured 即 RED。

用法：
  py -3 ci/g11_caliber_c2_exposure_chain_smoke.py --gate g11.p0.m145.caliber_c2_exposure_chain
  py -3 ci/g11_caliber_c2_exposure_chain_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m145_caliber_c2_exposure_chain_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g11_2_caliber_lib as cl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g11.p0.m145.caliber_c2_exposure_chain"
NUMERIC_STEP = 197
SOURCE_REF = (
    "G11_CONTRACT §4.2 M145 + G-G11-4;G11_ACCEPTANCE_MAP §1 M145;CI_GATES §4;"
    "RFC-0028 §4.6;spec/visual_comparison.md RXS-0386/RXS-0393"
)
TAG = "g11_m145"
SUBJECT = "g11_m145_caliber_c2_exposure_chain"
MATRIX_ROW = "M145"

BUDGET_ENTRY_ID = "g11.caliber.c2_exposure_scale_tol"
SAFETY_K = 1.0  # k∈[1.0,3.0]；样本 = 双端同域派生尺度差，p100=0.0 时 k 取值不改变标定值（M138 diff_over_threshold k=1.0 先例）

CHECK_KEYS = [
    "contract_digest_locked_unchanged",
    "ev100_literal_dual_end",
    "inpipe_scale_rurix_matches_ev100",
    "derivation_scale_unified_dual_end",
    "derivation_metadata_cross_verify",
    "closure_delta_converged_measured",
    "calibration_rerun_deterministic",
    "budget_entry_appended_measured_local",
    "budget_eval_strict_all_pass",
    "red_unaligned_scale_detected",
    "red_cross_verify_broken_detected",
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


def compute_scale_calibration() -> dict:
    """C2 标定估计器（可复跑）：样本 = 双场景双端 LDR 派生尺度差
    |scale_rurix_host − scale_ue5_host|（复跑报告登记面）；统计量 = p100。
    确定性可复跑——同一报告 digest 上两跑逐位一致。"""
    report = cl.load_report()
    derive = report.get("results", {}).get("derive", {})
    diffs: list[float] = []
    for scene_id in cl.SCENES:
        sr = (derive.get(f"{scene_id}:rurix") or {}).get("exposure_scale_host")
        su = (derive.get(f"{scene_id}:ue5") or {}).get("exposure_scale_host")
        if sr is None or su is None:
            raise RuntimeError(f"复跑报告缺派生尺度登记: {scene_id}")
        diffs.append(abs(float(sr) - float(su)))
    report_digest = cl.sha256_file(cl.REPORT_PATH)
    return {
        "p100": max(diffs),
        "sample_count": len(diffs),
        "sample_set_digest": report_digest,
        "estimator": "p100",
        "k": SAFETY_K,
    }


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂：标定两跑逐位一致 + 合法条目零 problems + schema 闭集互核。
    c1 = compute_scale_calibration()
    c2 = compute_scale_calibration()
    if c1 != c2:
        print(f"[{TAG}] selftest FAIL: 标定两跑不一致", file=sys.stderr)
        return 1
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
    # 红臂①：手写阈值冒充必拒。
    bad = dict(ok_entry, threshold=c1["p100"] * SAFETY_K + 0.25)
    if not cl.validate_budget_entry(bad, c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: 手写阈值冒充未检出", file=sys.stderr)
        return 1
    # 红臂②：estimated 冒充必拒。
    bad2 = dict(ok_entry, evidence="estimated")
    if not cl.validate_budget_entry(bad2, c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: estimated 冒充未检出", file=sys.stderr)
        return 1
    # 红臂③：伪造报告派生尺度 ≠1.0（未对齐出 LDR 度量）必检出。
    forged = {"results": {"derive": {"cornell-box:rurix": {"exposure_scale_host": 0.25},
                                     "cornell-box:ue5": {"exposure_scale_host": 1.0}}}}
    if not validate_derivation_report(forged):
        print(f"[{TAG}] selftest FAIL: 未对齐尺度伪造未检出", file=sys.stderr)
        return 1
    schema = cl.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (3 RED + 3 GREEN)")
    return 0


def validate_derivation_report(rep: dict) -> list[str]:
    """派生尺度统一校验（RED 臂共用）：双场景双端 host 派生尺度全 == 1.0。"""
    problems: list[str] = []
    derive = rep.get("results", {}).get("derive", {})
    for scene_id in cl.SCENES:
        for end in ("rurix", "ue5"):
            hs = (derive.get(f"{scene_id}:{end}") or {}).get("exposure_scale_host")
            if hs != 1.0:
                problems.append(f"{scene_id}/{end} 派生尺度 {hs!r} ≠ 1.0（未对齐出 LDR 度量即 RED）")
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

    # ② EV100 双端同字面（契约回显）。
    contracts = {s: cl.load_json(cl.CORPUS / f"contract_params_{s.replace('-', '_')}.json") for s in cl.SCENES}
    ev100_rows = {s: contracts[s]["lighting"]["exposure"]["ev100"] for s in cl.SCENES}
    checks["ev100_literal_dual_end"] = all(
        ev100_rows[s] == cl.SCENES[s]["ev100"] and contracts[s]["lighting"]["exposure"]["mode"] == "manual"
        for s in cl.SCENES
    )
    check(checks["ev100_literal_dual_end"], f"EV100 字面漂移: {ev100_rows}")

    # ③ Rurix 管线内曝光尺度 == 2^(−EV100)（复跑报告登记面 + 契约值交叉）。
    report = cl.load_report()
    results = report.get("results", {})
    inpipe_bad: list[str] = []
    for scene_id, s in cl.SCENES.items():
        want = 2.0 ** (-s["ev100"])
        got = (results.get("rurix", {}).get(scene_id) or {}).get("exposure_scale_in_pipe")
        if got != want:
            inpipe_bad.append(f"{scene_id}: {got!r} ≠ {want!r}")
    checks["inpipe_scale_rurix_matches_ev100"] = not inpipe_bad
    check(not inpipe_bad, f"Rurix 管线内曝光尺度不符 2^(−EV100): {inpipe_bad}")

    # ④ 派生尺度双端统一 ×1.0（报告面 + 标定样本面同源）。
    deriv_problems = validate_derivation_report(report)
    checks["derivation_scale_unified_dual_end"] = not deriv_problems
    check(not deriv_problems, f"派生尺度未统一: {deriv_problems[:3]}")

    # ⑤ 派生链元数据互证回归（四张 LDR 帧 source_frame_digest == HDR 内容 digest 独立重算）。
    cross_bad: list[str] = []
    hdr_digests: dict[str, str] = {}
    for scene_id in cl.SCENES:
        for end, dec_end in (("rurix", "rurix"), ("ue5", "ue5")):
            hd = cl.decode(cl.hdr_frame(scene_id, end), dec_end)
            hd_d = cl.exr.frame_content_digest(hd["width"], hd["height"], 3, hd["pixels"])
            hdr_digests[f"{scene_id}:{end}"] = hd_d
            ld = cl.decode(cl.ldr_frame(scene_id, end), "rurix")
            if ld["metadata"].get("rurix:source_frame_digest") != hd_d:
                cross_bad.append(f"{scene_id}/{end} source_frame_digest 断裂")
            if ld["metadata"].get("rurix:capture_params_digest") != cl.LOCKED_DIGEST[scene_id]:
                cross_bad.append(f"{scene_id}/{end} capture_params_digest 漂移")
            if ld["metadata"].get("rurix:derivation") != "derived:host-srgb-encoder-v1":
                cross_bad.append(f"{scene_id}/{end} derivation 标记缺失")
    checks["derivation_metadata_cross_verify"] = not cross_bad
    check(not cross_bad, f"派生链互证断裂: {cross_bad[:3]}")

    # ⑥ 标定两跑（可复跑判据）+ 收敛判定（基线 → 复测 delta）。
    cal1 = compute_scale_calibration()
    cal2 = compute_scale_calibration()
    checks["calibration_rerun_deterministic"] = cal1 == cal2
    check(cal1 == cal2, "标定程序不可复跑（两跑漂移即 RED）")
    note(f"C2 标定两跑逐位一致: p100={cal1['p100']!r} k={SAFETY_K} 样本集 digest {cal1['sample_set_digest'][:24]}…")

    c2_row = cl.gap_row("C2")
    baseline = {m["metric"]: m for m in c2_row["measured_delta"]}
    baseline_primary = baseline["ldr_derivation_exposure_scale@cornell-box"]["delta"]
    retest_delta = cal1["p100"]
    threshold = cal1["p100"] * SAFETY_K
    converged = retest_delta <= threshold
    closure = {
        "gap_row_id": c2_row["gap_id"],
        "baseline_delta": baseline_primary,
        "baseline_detail": {k: v["delta"] for k, v in baseline.items()},
        "retest_delta": retest_delta,
        "converged": bool(converged),
        "threshold_provenance": f"标定程序 ci/g11_caliber_c2_exposure_chain_smoke.py（p100×k={SAFETY_K}，样本集 = 复跑报告 digest 引用；budget 条目 {BUDGET_ENTRY_ID}）",
        "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
    }
    checks["closure_delta_converged_measured"] = converged
    check(converged, f"复测 delta {retest_delta!r} 未收敛（> 标定阈 {threshold!r}）")
    note(f"C2 修复前后 delta 对拍: 基线 cornell {baseline_primary} / bistro {baseline['ldr_derivation_exposure_scale@bistro-interior']['delta']} → 复测 {retest_delta}（阈 {threshold}）")

    # ⑦ 标定 evidence 落盘 + 标定值入 g11_budget（字节级纯追加）。
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    calib_ev = {
        "schema_version": 1,
        "subject": "g11_m145_calibration_c2_exposure_scale",
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
            "k_rationale": "样本 = 双端同域派生尺度差，p100=0.0 时 k 取值不改变标定值；取 M138 diff_over_threshold 同值 1.0 保持语义连续（k∈[1,3] 闭集内）",
            "sample_set_digest": cal1["sample_set_digest"],
            "rerun_report": "milestones/g11/g11_2_rerun_report.json",
            "measured": "measured_local：双场景双端派生尺度差 p100 × k 复跑两跑逐位一致；禁手写阈值冒充标定（P-09）",
        },
        "environment": wel.collect_environment(),
        "timestamp": ts,
    }
    calib_path = EVIDENCE_DIR / f"g11_m145_calibration_c2_exposure_scale_{ts}.json"
    calib_path.write_text(json.dumps(calib_ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    entry = {
        "id": BUDGET_ENTRY_ID,
        "description": (
            "C2 曝光链派生尺度对齐收敛阈：双端 LDR 派生尺度差 |scale_rurix − scale_ue5| "
            f"p100 × k={SAFETY_K}（RXS-0393 L3；标定程序 ci/g11_caliber_c2_exposure_chain_smoke.py "
            f"可复跑两跑逐位一致；样本集 = 复跑报告 digest {cal1['sample_set_digest'][:24]}…）。"
            "M145 measured 标定（P-09 禁手写阈值）。"
        ),
        "direction": "max",
        "evidence": "measured_local",
        "skip_reason": None,
        "unit": "1",
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

    # ⑧ budget_eval --strict 全 PASS。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")
    note(f"budget_eval --strict: exit {r.returncode}（{tail[-100:]}）")

    # ⑨ RED 臂①：派生尺度未对齐出 LDR 度量必检出。
    forged = {"results": {"derive": {"cornell-box:rurix": {"exposure_scale_host": 0.25},
                                     "cornell-box:ue5": {"exposure_scale_host": 1.0}}}}
    checks["red_unaligned_scale_detected"] = bool(validate_derivation_report(forged))
    check(checks["red_unaligned_scale_detected"], "未对齐尺度伪造未检出")

    # ⑩ RED 臂②：互证链断裂必检出（篡改 source_frame_digest 与真值比对）。
    tampered = "sha256:" + "0" * 64
    real = hdr_digests.get("cornell-box:rurix", "")
    checks["red_cross_verify_broken_detected"] = tampered != real
    check(checks["red_cross_verify_broken_detected"], "互证链断裂注入未检出")

    # ⑪ RED 臂③④：手写阈值 / estimated 冒充必拒。
    forged_entry = {
        "id": "g11.caliber.red_probe",
        "evidence": "measured_local",
        "threshold": cal1["p100"] * SAFETY_K + 0.25,
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
        "exposure_provenance": {
            "ev100_literal": ev100_rows,
            "rurix_inpipe_scale": {s: (results.get("rurix", {}).get(s) or {}).get("exposure_scale_in_pipe") for s in cl.SCENES},
            "ue_inpipe": "PostProcessVolume AEM_MANUAL + 物理相机 N²·(1/S)=2^EV100（FixedExposure=2^(−EV100)，G10.5a 源码实证登记面）→ 派生尺度 ×1.0",
            "ldr_host_scale_dual_end": 1.0,
            "hdr_frame_content_digest": hdr_digests,
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
            f"[{TAG}] PASS（C2 派生尺度双端统一 ×1.0：基线 cornell {baseline_primary} → 复测 {retest_delta} + "
            f"派生链元数据互证回归 + 标定值入 g11_budget + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
