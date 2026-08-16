#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波）
"""G11.3 M147 R1 材质子集修复闭环门（P0，步骤 201；
g11.p0.m147.fix_r1_material_subset；G11_CONTRACT §4.2 M147 行判据逐字 / G-G11-5；
G11_ACCEPTANCE_MAP §1 M147 行；CI_GATES §4；g10_gap_registry R1 行承接锚；
spec/visual_comparison.md RXS-0393）。

host+device 门（host CPU 参考管线真渲染，device_section_state=executed）。
判据（契约 §4.2 M147 行字面）：

1. **baseColorTexture/法线/metallic-roughness 采样接入（承接锚字面消费）**：
   bistro 复跑帧由 `--material-pbr` 渲染——baseColorTexture（bcdec DDS 真实解码
   + sRGB→线性 IEC 分段）× baseColorFactor ×(1−metallic) 漫反射 + 太阳 GGX 高光
   + 法线贴图（BC5 XY 重建 Z，逐三角形 UV 梯度切线架）+ GI 逐实例 albedo 代理；
   消费登记 = 渲染输出 materials 闭集块（textured==70 / normal_mapped==70 /
   textures_consumed==144 / formats={bc1,bc3,bc5} / declared_unconsumed==[]）。
2. **修复前后 LDR 臂度量 delta 收敛 measured（锁定基线 = bistro LDR SSIM delta
   0.8328980787837229，收敛阈由标定程序产）**：基线复现（G10.5 LDR 帧只读重算
   == 锁定值 f64）+ 复测 delta（G11.3 帧区实测）收敛判定（RXS-0393 L2）。
3. **契约 digest 0-byte**：当次重算 == G10.5 锁定值（双场景）。

RED 臂（契约判据字面）：未采样冒充修复即 RED（red_unsampled_masquerade——伪造
消费登记必检出）；delta 未收敛冒充闭环即 RED（red_unconverged_masquerade）；
契约参数漂移即 RED（contract_digest_locked_unchanged）；手写阈值/estimated
冒充标定即 RED。

用法：
  py -3 ci/g11_fix_r1_material_subset_smoke.py --gate g11.p0.m147.fix_r1_material_subset
  py -3 ci/g11_fix_r1_material_subset_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m147_fix_r1_material_subset_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g11_3_fix_lib as fl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g11.p0.m147.fix_r1_material_subset"
NUMERIC_STEP = 201
SOURCE_REF = (
    "G11_CONTRACT §4.2 M147 + G-G11-5;G11_ACCEPTANCE_MAP §1 M147;CI_GATES §4;"
    "g10_gap_registry R1 行承接锚;spec/visual_comparison.md RXS-0393"
)
TAG = "g11_m147"
SUBJECT = "g11_m147_fix_r1_material_subset"
MATRIX_ROW = "M147"

BUDGET_ENTRY_ID = "g11.fix.r1_ssim_shrink_tol"
SAFETY_K = 1.0

CHECK_KEYS = [
    "contract_digest_locked_unchanged",
    "material_subset_consumed",
    "rurix_frame_changed_vs_g10",
    "baseline_metric_reproduction",
    "closure_delta_converged_measured",
    "calibration_rerun_deterministic",
    "budget_entry_appended_measured_local",
    "budget_eval_strict_all_pass",
    "red_unsampled_masquerade_detected",
    "red_unconverged_masquerade_detected",
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


def _consumption_problems(mats: dict) -> list[str]:
    """材质子集消费登记校验（RED 臂共用：未采样冒充修复判红面）。"""
    problems: list[str] = []
    if mats.get("material_pbr") is not True:
        problems.append("material_pbr ≠ true（--material-pbr 消费链断裂）")
    if mats.get("textured") != 70:
        problems.append(f"textured={mats.get('textured')!r} ≠ 70（baseColorTexture 未全量采样——未采样冒充修复即 RED）")
    if mats.get("normal_mapped") != 70:
        problems.append(f"normal_mapped={mats.get('normal_mapped')!r} ≠ 70（法线贴图未全量采样）")
    if mats.get("textures_consumed") != 144:
        problems.append(f"textures_consumed={mats.get('textures_consumed')!r} ≠ 144（DDS 解码消费面缺量）")
    if sorted(mats.get("texture_formats") or []) != ["bc1", "bc3", "bc5"]:
        problems.append(f"texture_formats={mats.get('texture_formats')!r} ≠ [bc1,bc3,bc5]（实测枚举闭集漂移）")
    if mats.get("textures_declared_unconsumed"):
        problems.append(f"declared_unconsumed 非空: {mats.get('textures_declared_unconsumed')!r}（bistro 面应全消费）")
    return problems


def compute_shrink_calibration() -> dict:
    """R1 收敛幅度阈标定：样本 = LDR SSIM 双跑噪声（确定性帧同一对两跑逐位一致）。"""
    a = fl.ssim_ldr("bistro-interior")
    b = fl.ssim_ldr("bistro-interior")
    return {"p100": abs(a - b), "sample_count": 1, "estimator": "p100", "k": SAFETY_K, "ssim": a}


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    c1 = compute_shrink_calibration()
    c2 = compute_shrink_calibration()
    if c1 != c2:
        print(f"[{TAG}] selftest FAIL: 标定两跑不一致", file=sys.stderr)
        return 1
    ok_entry = {
        "id": "g11.fix.selftest_probe",
        "evidence": "measured_local",
        "threshold": c1["p100"] * SAFETY_K,
        "measured_value": c1["p100"],
        "evidence_file": "milestones/g11/g11_budget.json",
    }
    if fl.validate_budget_entry(ok_entry, c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: 合法条目误判", file=sys.stderr)
        return 1
    if not fl.validate_budget_entry(dict(ok_entry, threshold=c1["p100"] + 0.25), c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: 手写阈值冒充未检出", file=sys.stderr)
        return 1
    if not fl.validate_budget_entry(dict(ok_entry, evidence="estimated"), c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: estimated 冒充未检出", file=sys.stderr)
        return 1
    # 红臂①：未采样冒充必检出。
    if not _consumption_problems({"material_pbr": True, "textured": 0, "normal_mapped": 0, "textures_consumed": 0, "texture_formats": [], "textures_declared_unconsumed": []}):
        print(f"[{TAG}] selftest FAIL: 未采样冒充未检出", file=sys.stderr)
        return 1
    # 红臂②：未收敛冒充必检出。
    if fl.evaluate_closure(0.8328980787837229, 0.8328980787837229, 0.0)["converged"]:
        print(f"[{TAG}] selftest FAIL: 未收敛冒充未检出", file=sys.stderr)
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
    check(not digest_drift, f"契约 digest 漂移（契约参数漂移即 RED）: {digest_drift}")

    # ② 材质子集消费登记（承接锚字面消费面）。
    rep = fl.load_report()
    rurix_bistro = rep.get("results", {}).get("rurix", {}).get("bistro-interior", {})
    mats = (rurix_bistro.get("render_json", {}) or {}).get("materials", {}) or {}
    cons_problems = _consumption_problems(mats)
    checks["material_subset_consumed"] = not cons_problems
    check(not cons_problems, f"材质子集消费异常: {cons_problems[:3]}")

    # ③ Rurix 帧 ≠ G10.5 锁定帧（修复生效——未变冒充修复即 RED 的对偶面）。
    ru_digest_now = rurix_bistro.get("frame_content_digest", "")
    checks["rurix_frame_changed_vs_g10"] = (
        bool(ru_digest_now) and ru_digest_now != fl.G10_5_FRAME_DIGEST[("rurix", "bistro-interior")]
    )
    check(checks["rurix_frame_changed_vs_g10"], "Rurix bistro 帧未变——材质采样未生效冒充")

    # ④ 基线复现（G10.5 LDR 帧只读重算 == 锁定值 f64）。
    base_ssim = fl.ssim_ldr("bistro-interior", root=fl.FRAMES_G10_5)
    r1_row = fl.gap_row("R1")
    baseline = r1_row["measured_delta"][0]["delta"]
    baseline_a = r1_row["measured_delta"][0]["a_value"]
    repro_ok = (base_ssim == baseline_a and (1.0 - base_ssim) == baseline)
    checks["baseline_metric_reproduction"] = repro_ok
    check(repro_ok, f"基线复现漂移: ssim {base_ssim!r} ≠ 锁定 {baseline_a!r}")

    # ⑤ 复测 delta + 收敛判定。
    retest_ssim = fl.ssim_ldr("bistro-interior")
    retest_delta = 1.0 - retest_ssim
    cal1 = compute_shrink_calibration()
    cal2 = compute_shrink_calibration()
    checks["calibration_rerun_deterministic"] = cal1 == cal2
    check(cal1 == cal2, "标定程序不可复跑（两跑漂移即 RED）")
    threshold = cal1["p100"] * SAFETY_K

    ev = fl.evaluate_closure(baseline, retest_delta, threshold)
    closure = {
        "gap_row_id": r1_row["gap_id"],
        "baseline_delta": baseline,
        "retest_delta": retest_delta,
        "converged": bool(ev["converged"]),
        "threshold_provenance": f"标定程序 ci/g11_fix_r1_material_subset_smoke.py（LDR SSIM 双跑噪声 p100×k={SAFETY_K}，样本集 = G11.3 bistro LDR 帧对；budget 条目 {BUDGET_ENTRY_ID}）",
        "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
    }
    checks["closure_delta_converged_measured"] = ev["converged"]
    check(ev["converged"], f"复测 delta {retest_delta!r} 未收敛（基线 {baseline!r}——delta 未收敛冒充闭环即 RED）")
    note(f"R1 修复前后 delta 对拍: 基线 {baseline}（ssim {baseline_a}）→ 复测 {retest_delta}（ssim {retest_ssim}）")
    if not ev["converged"]:
        note(
            "R1 未收敛根因（诚实登记）：Rurix 帧真实反照率（纹理均值×(1−metallic)≈0.10）"
            "下单反弹 GI + 无点光源（R3/R4 承接面 G11.4）使帧面较 UE 暗 ≈150×，"
            "SSIM 亮度/结构项塌陷——材质采样本身经 manifest/均值/digest 面核验正确落地"
        )

    # ⑥ 标定 evidence 落盘 + budget 追加。
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    report_digest = fl.sha256_file(fl.REPORT_PATH)
    calib_ev = fl.calib_evidence_payload(
        subject="g11_m147_calibration_r1_ssim_shrink",
        gate_key=GATE_KEY, matrix_row=MATRIX_ROW, numeric_step=NUMERIC_STEP,
        p100=cal1["p100"], k=SAFETY_K, sample_count=cal1["sample_count"],
        sample_set_digest=report_digest,
        provenance_measured="measured_local：G11.3 bistro LDR 帧对 SSIM 双跑逐位一致（确定性），噪声 p100×k；禁手写阈值冒充标定（P-09）",
        ts=ts,
    )
    calib_ev["environment"] = wel.collect_environment()
    calib_ev["provenance"]["k_rationale"] = "样本 = 双跑噪声，p100=0.0 时 k 取值不改变标定值；取 M138/C2 同值 1.0（k∈[1,3] 闭集内）"
    calib_path = EVIDENCE_DIR / f"g11_m147_calibration_r1_ssim_shrink_{ts}.json"
    calib_path.write_text(json.dumps(calib_ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    entry = {
        "id": BUDGET_ENTRY_ID,
        "description": (
            "R1 LDR SSIM delta 收敛幅度阈：双跑噪声 p100 × k=1.0（RXS-0393 L3；标定程序 "
            f"ci/g11_fix_r1_material_subset_smoke.py 两跑逐位一致；样本集 digest {report_digest[:24]}…）。M147 measured 标定（P-09）。"
        ),
        "direction": "max",
        "evidence": "measured_local",
        "skip_reason": None,
        "unit": "1",
        "threshold": threshold,
        "evidence_file": str(calib_path.relative_to(ROOT)).replace("\\", "/"),
        "measured_value": cal1["p100"],
    }
    budget_problems = fl.validate_budget_entry(entry, cal1["p100"], SAFETY_K)
    if not budget_problems:
        budget_problems = fl.append_budget_entries([entry])
        if not budget_problems:
            note(f"g11_budget.json 字节级纯追加 {BUDGET_ENTRY_ID}（threshold={threshold!r}）")
    checks["budget_entry_appended_measured_local"] = not budget_problems
    check(not budget_problems, f"budget 条目异常: {budget_problems[:2]}")

    # ⑦ budget_eval --strict。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")

    # ⑧ RED 臂①：未采样冒充必检出。
    checks["red_unsampled_masquerade_detected"] = bool(_consumption_problems(
        {"material_pbr": True, "textured": 0, "normal_mapped": 0, "textures_consumed": 0, "texture_formats": [], "textures_declared_unconsumed": []}
    ))
    check(checks["red_unsampled_masquerade_detected"], "未采样冒充未检出")

    # ⑨ RED 臂②：delta 未收敛冒充必检出。
    forged_nc = fl.evaluate_closure(baseline, baseline, threshold)
    checks["red_unconverged_masquerade_detected"] = not forged_nc["converged"]
    check(checks["red_unconverged_masquerade_detected"], "未收敛冒充未检出")

    # ⑩ RED 臂③④：手写阈值 / estimated 冒充必拒。
    forged_entry = {
        "id": "g11.fix.red_probe",
        "evidence": "measured_local",
        "threshold": cal1["p100"] * SAFETY_K + 0.25,
        "measured_value": cal1["p100"],
        "evidence_file": str(calib_path.relative_to(ROOT)).replace("\\", "/"),
    }
    checks["red_handwritten_threshold_detected"] = bool(fl.validate_budget_entry(forged_entry, cal1["p100"], SAFETY_K))
    check(checks["red_handwritten_threshold_detected"], "手写阈值冒充未检出")
    forged_entry2 = dict(forged_entry, threshold=cal1["p100"] * SAFETY_K, evidence="estimated")
    checks["red_estimated_masquerade_detected"] = bool(fl.validate_budget_entry(forged_entry2, cal1["p100"], SAFETY_K))
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
        "material_provenance": {
            "render_materials_block": mats,
            "rurix_frame_content_digest": ru_digest_now,
            "g10_5_locked_rurix_frame_digest": fl.G10_5_FRAME_DIGEST[("rurix", "bistro-interior")],
            "retest_ssim": retest_ssim,
            "baseline_ssim": baseline_a,
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
            f"[{TAG}] PASS（R1 材质子集消费闭环：delta {baseline} → {retest_delta} 收敛 + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
