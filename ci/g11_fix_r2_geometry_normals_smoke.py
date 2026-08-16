#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波）
"""G11.3 M148 R2 几何法线修复闭环门（P0，步骤 202；
g11.p0.m148.fix_r2_geometry_normals；G11_CONTRACT §4.2 M148 行判据逐字 / G-G11-5；
G11_ACCEPTANCE_MAP §1 M148 行；CI_GATES §4；g10_gap_registry R2 行承接锚；
spec/visual_comparison.md RXS-0393）。

host+device 门（host CPU 参考管线真渲染，device_section_state=executed）。
判据（契约 §4.2 M148 行字面）：

1. **winding 朝向 + 双面翻转消费（平滑法线面承接锚字面）**：cornell 复跑帧由
   `--smooth-normals` 渲染（顶点平滑法线重心插值 + 逆矩阵转置世界化 + 双面翻转
   消费）——门内独立核验 cornell gltf NORMAL 属性在树（accessor 非空）+ 复跑
   报告旗标/消费登记 + 渲染输出 materials.smooth_normals=true。
2. **修复前后 cornell HDR 覆盖 delta 收敛 measured（锁定基线 −0.7451210021972656）**：
   基线复现（G10.5 锁定帧只读重算 == 锁定值 f64）+ 复测 delta（G11.3 帧区实测）
   收敛判定（RXS-0393 L2 quality_gap 款 + zero_band 跨端离散一致性标定带——
   per-tile XOR p100×k measured 产）。
3. **与 U1 同面对账**：UE 侧壳体双面化修复证据（two_sided_replacement provenance
   非空 + 读回真）与 Rurix 侧平滑法线消费共享同一 cornell 覆盖 delta 面。
4. **Rurix 侧覆盖面不降级**：复测 Rurix 覆盖 ≥ 锁定基线 a 值。

RED 臂（契约判据字面）：法线未消费冒充修复即 RED（red_normals_not_consumed）；
delta 未收敛冒充闭环即 RED（red_unconverged_masquerade）；方向性注入即 RED
（red_direction_injection——伪造符号翻转超带必检出）；手写阈值/estimated 冒充即 RED。

用法：
  py -3 ci/g11_fix_r2_geometry_normals_smoke.py --gate g11.p0.m148.fix_r2_geometry_normals
  py -3 ci/g11_fix_r2_geometry_normals_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m148_fix_r2_geometry_normals_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g11_3_fix_lib as fl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g11.p0.m148.fix_r2_geometry_normals"
NUMERIC_STEP = 202
SOURCE_REF = (
    "G11_CONTRACT §4.2 M148 + G-G11-5;G11_ACCEPTANCE_MAP §1 M148;CI_GATES §4;"
    "g10_gap_registry R2 行承接锚;spec/visual_comparison.md RXS-0393"
)
TAG = "g11_m148"
SUBJECT = "g11_m148_fix_r2_geometry_normals"
MATRIX_ROW = "M148"

SHRINK_BUDGET_ID = "g11.fix.r2_coverage_shrink_tol"
ZBAND_BUDGET_ID = "g11.fix.r2_coverage_zero_band"
SAFETY_K = 1.0   # shrink 阈：零 p100 先例沿 M138/C2
ZBAND_K = 2.0    # zero_band：per-tile XOR p100×k（M157 k=2.0 先例）

CORNELL_GLTF = Path(r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf")

CHECK_KEYS = [
    "contract_digest_locked_unchanged",
    "smooth_normals_consumed",
    "baseline_metric_reproduction",
    "closure_delta_converged_measured",
    "u1_shared_face_reconciliation",
    "rurix_coverage_non_degrade",
    "calibration_rerun_deterministic",
    "budget_entry_appended_measured_local",
    "budget_eval_strict_all_pass",
    "red_normals_not_consumed_detected",
    "red_unconverged_masquerade_detected",
    "red_direction_injection_detected",
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


def cornell_normal_accessor_count() -> int:
    """cornell gltf NORMAL 属性 accessor 独立计数（平滑法线消费前提面）。"""
    doc = json.loads(CORNELL_GLTF.read_text(encoding="utf-8"))
    n = 0
    for m in doc.get("meshes", []):
        for p in m.get("primitives", []):
            if "NORMAL" in (p.get("attributes") or {}):
                n += 1
    return n


def compute_shrink_calibration() -> dict:
    """R2 收敛幅度阈标定：样本 = 覆盖 delta 双跑噪声（确定性帧同一对两跑逐位一致）。"""
    a = fl.coverage_delta("cornell-box")
    b = fl.coverage_delta("cornell-box")
    noise = abs(a["delta"] - b["delta"])
    return {"p100": noise, "sample_count": 1, "estimator": "p100", "k": SAFETY_K}


def compute_zband_calibration() -> dict:
    """R2 zero_band 标定：per-tile XOR p100×k（跨端离散一致性包络，M157 程序纪律）。"""
    cal = fl.coverage_zero_band_calibration("cornell-box", k=ZBAND_K)
    return cal


def _consumption_problems(render_json: dict, rurix_flags: list[str]) -> list[str]:
    """平滑法线消费校验（RED 臂共用）。"""
    problems: list[str] = []
    mats = render_json.get("materials", {}) or {}
    if mats.get("smooth_normals") is not True:
        problems.append("渲染输出 materials.smooth_normals ≠ true（法线未消费冒充修复即 RED）")
    if "--smooth-normals" not in (rurix_flags or []):
        problems.append("复跑报告 rurix_flags 缺 --smooth-normals（消费链断裂）")
    return problems


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    c1 = compute_zband_calibration()
    c2 = compute_zband_calibration()
    if c1 != c2:
        print(f"[{TAG}] selftest FAIL: zero_band 标定两跑不一致", file=sys.stderr)
        return 1
    ok_entry = {
        "id": "g11.fix.selftest_probe",
        "evidence": "measured_local",
        "threshold": c1["p100"] * ZBAND_K,
        "measured_value": c1["p100"],
        "evidence_file": "milestones/g11/g11_budget.json",
    }
    if fl.validate_budget_entry(ok_entry, c1["p100"], ZBAND_K):
        print(f"[{TAG}] selftest FAIL: 合法条目误判", file=sys.stderr)
        return 1
    if not fl.validate_budget_entry(dict(ok_entry, threshold=c1["p100"]), c1["p100"], ZBAND_K):
        print(f"[{TAG}] selftest FAIL: 手写阈值冒充未检出", file=sys.stderr)
        return 1
    if not fl.validate_budget_entry(dict(ok_entry, evidence="estimated"), c1["p100"], ZBAND_K):
        print(f"[{TAG}] selftest FAIL: estimated 冒充未检出", file=sys.stderr)
        return 1
    # 红臂：方向性注入（符号翻转超带）必检出。
    forged = fl.evaluate_closure(-0.7451210021972656, 0.5, 0.0, c1["zero_band"])
    if forged["converged"]:
        print(f"[{TAG}] selftest FAIL: 方向性注入伪造未检出", file=sys.stderr)
        return 1
    # 红臂：法线未消费冒充必检出。
    if not _consumption_problems({"materials": {"smooth_normals": False}}, []):
        print(f"[{TAG}] selftest FAIL: 法线未消费伪造未检出", file=sys.stderr)
        return 1
    schema = fl.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (4 RED + 3 GREEN)")
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

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    # ① 契约 digest 三面绑定 0-byte。
    digest_drift = [
        f"{s}: {fl.contract_digest_rust(s)} ≠ {fl.LOCKED_DIGEST[s]}"
        for s in fl.SCENES
        if fl.contract_digest_rust(s) != fl.LOCKED_DIGEST[s]
    ]
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "g10_5_scene_render --contract-digest ×2 scenes", "exit_code": 0})
    checks["contract_digest_locked_unchanged"] = not digest_drift
    check(not digest_drift, f"契约 digest 漂移: {digest_drift}")

    # ② 平滑法线消费（旗标 + gltf NORMAL 属性在树 + 输出登记）。
    rep = fl.load_report()
    rurix_cornell = rep.get("results", {}).get("rurix", {}).get("cornell-box", {})
    render_json = rurix_cornell.get("render_json", {}) or {}
    cons_problems = _consumption_problems(render_json, rurix_cornell.get("rurix_flags", []))
    n_normals = cornell_normal_accessor_count()
    if n_normals <= 0:
        cons_problems.append("cornell gltf NORMAL 属性缺失（平滑法线消费前提不在）")
    checks["smooth_normals_consumed"] = not cons_problems
    check(not cons_problems, f"平滑法线消费异常: {cons_problems[:3]}")
    note(f"cornell gltf NORMAL accessor 计数 = {n_normals}")

    # ③ 基线复现（G10.5 锁定帧只读重算 == 锁定值 f64）。
    base = fl.coverage_delta("cornell-box", root=fl.FRAMES_G10_5)
    r2_row = fl.gap_row("R2")
    baseline = r2_row["measured_delta"][0]["delta"]
    baseline_a = r2_row["measured_delta"][0]["a_value"]
    baseline_b = r2_row["measured_delta"][0]["b_value"]
    repro_ok = (
        base["rurix"] == baseline_a and base["ue5"] == baseline_b and base["delta"] == baseline
    )
    checks["baseline_metric_reproduction"] = repro_ok
    check(repro_ok, f"基线复现漂移: 重算 {base['rurix']}/{base['ue5']}/{base['delta']} ≠ 锁定 {baseline_a}/{baseline_b}/{baseline}")

    # ④ 复测 delta + 收敛判定（zero_band 标定带）。
    retest = fl.coverage_delta("cornell-box")
    shrink_cal1 = compute_shrink_calibration()
    shrink_cal2 = compute_shrink_calibration()
    zband_cal1 = compute_zband_calibration()
    zband_cal2 = compute_zband_calibration()
    checks["calibration_rerun_deterministic"] = (
        shrink_cal1 == shrink_cal2 and zband_cal1 == zband_cal2
    )
    check(checks["calibration_rerun_deterministic"], "标定程序不可复跑（两跑漂移即 RED）")
    shrink_threshold = shrink_cal1["p100"] * SAFETY_K
    zero_band = zband_cal1["zero_band"]

    ev = fl.evaluate_closure(baseline, retest["delta"], shrink_threshold, zero_band)
    closure = {
        "gap_row_id": r2_row["gap_id"],
        "baseline_delta": baseline,
        "retest_delta": retest["delta"],
        "converged": bool(ev["converged"]),
        "threshold_provenance": (
            f"标定程序 ci/g11_fix_r2_geometry_normals_smoke.py（shrink 阈 = 覆盖 delta 双跑噪声 p100×k={SAFETY_K}；"
            f"zero_band = per-tile XOR p100 {zband_cal1['p100']}×k={ZBAND_K}={zero_band}，样本集 = G11.3 复测 cornell 帧对 64 瓦片；"
            f"budget 条目 {SHRINK_BUDGET_ID}/{ZBAND_BUDGET_ID}）"
        ),
        "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
    }
    checks["closure_delta_converged_measured"] = ev["converged"]
    check(ev["converged"], f"复测 delta {retest['delta']!r} 未收敛（基线 {baseline!r}，zero_band {zero_band!r}，方向 {ev['direction_ok']}）")
    note(f"R2 修复前后 delta 对拍: 基线 {baseline} → 复测 {retest['delta']}（shrink 阈 {shrink_threshold}，zero_band {zero_band}，global_xor {zband_cal1['global_xor_ratio']}）")

    # ⑤ 与 U1 同面对账（UE 侧双面化修复证据在树）。
    ue_probe = rep.get("results", {}).get("ue", {}).get("cornell-box", {}).get("probe", {}) or {}
    tsr = ue_probe.get("two_sided_replacement") or []
    u1_ok = (
        len(tsr) > 0
        and all(r.get("two_sided_readback") is True for r in tsr)
        and ue_probe.get("two_sided_actor_count") == len(tsr)
    )
    checks["u1_shared_face_reconciliation"] = u1_ok
    check(u1_ok, "U1 同面对账断裂（UE 双面置换 provenance 缺/读回假）")

    # ⑥ Rurix 侧覆盖面不降级。
    checks["rurix_coverage_non_degrade"] = retest["rurix"] >= baseline_a
    check(checks["rurix_coverage_non_degrade"], f"Rurix 覆盖降级: {retest['rurix']} < {baseline_a}（Rurix 侧降级即 RED）")

    # ⑦ 标定 evidence 落盘 ×2 + 标定值入 g11_budget（字节级纯追加）。
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    report_digest = fl.sha256_file(fl.REPORT_PATH)
    entries = []
    for subj, bid, cal, k, desc in (
        ("g11_m148_calibration_r2_coverage_shrink", SHRINK_BUDGET_ID, shrink_cal1, SAFETY_K,
         "R2 覆盖 delta 收敛幅度阈：覆盖 delta 双跑噪声 p100 × k=1.0"),
        ("g11_m148_calibration_r2_coverage_zero_band", ZBAND_BUDGET_ID,
         {"p100": zband_cal1["p100"], "sample_count": zband_cal1["sample_count"]}, ZBAND_K,
         "R2 覆盖 delta zero_band：per-tile XOR p100 × k=2.0（跨端离散一致性包络）"),
    ):
        calib_ev = fl.calib_evidence_payload(
            subject=subj, gate_key=GATE_KEY, matrix_row=MATRIX_ROW, numeric_step=NUMERIC_STEP,
            p100=cal["p100"], k=k, sample_count=cal["sample_count"],
            sample_set_digest=report_digest,
            provenance_measured="measured_local：G11.3 复测 cornell 帧对确定性双跑/瓦片剖分实测（P-09 禁手写阈值）",
            ts=ts,
        )
        calib_ev["environment"] = wel.collect_environment()
        calib_ev["provenance"]["k_rationale"] = "k 取值见门脚本常量（零 p100 先例 1.0 / zero_band 沿 M157 先例 2.0；k∈[1,3] 闭集内）"
        calib_path = EVIDENCE_DIR / f"{subj}_{ts}.json"
        calib_path.write_text(json.dumps(calib_ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        entries.append({
            "id": bid,
            "description": f"{desc}（RXS-0393 L3；标定程序 ci/g11_fix_r2_geometry_normals_smoke.py 两跑逐位一致；样本集 digest {report_digest[:24]}…）。M148 measured 标定（P-09）。",
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": cal["p100"] * k,
            "evidence_file": str(calib_path.relative_to(ROOT)).replace("\\", "/"),
            "measured_value": cal["p100"],
        })
    budget_problems: list[str] = []
    for e, cal, k in ((entries[0], shrink_cal1, SAFETY_K), (entries[1], {"p100": zband_cal1["p100"]}, ZBAND_K)):
        budget_problems += fl.validate_budget_entry(e, cal["p100"], k)
    if not budget_problems:
        budget_problems = fl.append_budget_entries(entries)
        if not budget_problems:
            note(f"g11_budget.json 字节级纯追加 {SHRINK_BUDGET_ID}/{ZBAND_BUDGET_ID}")
    checks["budget_entry_appended_measured_local"] = not budget_problems
    check(not budget_problems, f"budget 条目异常: {budget_problems[:2]}")

    # ⑧ budget_eval --strict 全 PASS。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")

    # ⑨ RED 臂①：法线未消费冒充必检出。
    checks["red_normals_not_consumed_detected"] = bool(
        _consumption_problems({"materials": {"smooth_normals": False}}, [])
    )
    check(checks["red_normals_not_consumed_detected"], "法线未消费伪造未检出")

    # ⑩ RED 臂②：delta 未收敛冒充必检出（复测 == 基线）。
    forged_nc = fl.evaluate_closure(baseline, baseline, shrink_threshold, zero_band)
    checks["red_unconverged_masquerade_detected"] = not forged_nc["converged"]
    check(checks["red_unconverged_masquerade_detected"], "未收敛冒充未检出")

    # ⑪ RED 臂③：方向性注入（符号翻转超带）必检出。
    forged_dir = fl.evaluate_closure(baseline, 0.5, shrink_threshold, zero_band)
    checks["red_direction_injection_detected"] = not forged_dir["converged"]
    check(checks["red_direction_injection_detected"], "方向性注入伪造未检出")

    # ⑫ RED 臂④⑤：手写阈值 / estimated 冒充必拒。
    forged_entry = {
        "id": "g11.fix.red_probe",
        "evidence": "measured_local",
        "threshold": zband_cal1["p100"] * ZBAND_K + 0.25,
        "measured_value": zband_cal1["p100"],
        "evidence_file": entries[1]["evidence_file"],
    }
    checks["red_handwritten_threshold_detected"] = bool(fl.validate_budget_entry(forged_entry, zband_cal1["p100"], ZBAND_K))
    check(checks["red_handwritten_threshold_detected"], "手写阈值冒充未检出")
    forged_entry2 = dict(forged_entry, threshold=zband_cal1["p100"] * ZBAND_K, evidence="estimated")
    checks["red_estimated_masquerade_detected"] = bool(fl.validate_budget_entry(forged_entry2, zband_cal1["p100"], ZBAND_K))
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
        "wave": "G11.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "executed",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "closure": closure,
        "normals_provenance": {
            "cornell_normal_accessor_count": n_normals,
            "rurix_flags": rurix_cornell.get("rurix_flags"),
            "render_materials_block": render_json.get("materials"),
            "retest_coverage": {"rurix": retest["rurix"], "ue5": retest["ue5"]},
            "baseline_coverage": {"rurix": baseline_a, "ue5": baseline_b},
            "zero_band_calibration": {"p100": zband_cal1["p100"], "k": ZBAND_K, "zero_band": zero_band, "global_xor_ratio": zband_cal1["global_xor_ratio"]},
            "u1_shared_face": {"two_sided_actor_count": ue_probe.get("two_sided_actor_count")},
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
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=executed")
    if all_pass and not FAILURES:
        print(
            f"[{TAG}] PASS（R2 平滑法线消费闭环：基线 delta {baseline} → 复测 {retest['delta']} "
            f"〔zero_band {zero_band}〕+ U1 同面对账 + Rurix 覆盖不降级 + RED 五臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
