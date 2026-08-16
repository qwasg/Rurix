#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波；G11.3 收口双 phase 校准；G11.5 波 g11.5 phase 腿落地）
"""G11.3/G11.5 M147 R1 材质子集修复闭环门·双 phase（P0，步骤 201；
g11.p0.m147.fix_r1_material_subset --phase g11.3|g11.5；G11_CONTRACT §4.2 M147
行判据逐字（正文 0-byte 冻结）+ §8.3a 双 phase 修订句 / G-G11-5 / G-G11-7；
G11_ACCEPTANCE_MAP §1 M147 行 + §3.4 双 phase 口径；CI_GATES §4；
g10_gap_registry R1 行承接锚；spec/visual_comparison.md RXS-0393；
G10 M130 单 key 双 phase 先例同构）。

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
   == 锁定值 f64）+ 复测 delta（definitive 测量面 = G11.5 同契约复跑帧区实测）
   收敛判定（RXS-0393 L2 quality_gap 款字面）。
3. **契约 digest 0-byte**：当次重算 == G10.5 锁定值（双场景）。

双 phase 口径（§8.3a 修订句；判据语义 0-byte——收敛断言一字不弱只后移）：

- `--phase g11.3`（修复落盘 + 局部度量登记面）：判据 1/3 与判据 2 的
  基线复现/标定/复测 measured 面全绿；收敛检改写为 verdict 显式登记形态——
  实测收敛（verdict=converged ∧ convergence_pending=false）或
  deferred_to_g11_5 显式登记（verdict=deferred_to_g11_5 ∧
  convergence_pending=true）皆合法；evidence 标 phase=g11.3 +
  g11_3_phase_pass（当且仅当 12 检全绿）+ convergence_pending——
  **不是 SKIP 充绿**：convergence_pending 缺登记冒充全闭环即 RED
  （selftest 红臂机核）。G11.3 实测未收敛（复测 0.9903435577002249 > 基线
  0.8328980787837229——R1 局部度量被 R3/R4 光照残余结构性主导，证据链
  milestones/g11/g11_2_residual_caliber_registry.json + 反向激励旁证
  ssim(ue_修,rurix_未修白帧) > ssim(ue_修,rurix_修) measured 登记）。
- `--phase g11.5`（收敛断言面，definitive 测量面 = G11.5 同契约复跑，
  RXS-0393 L2 quality_gap 款字面；G11.5 波落地）：对 R1 行给出修复前后 SSIM
  delta 收敛断言——复测 delta 自 G11.5 帧区当次独立重算（与复测差距清单 R1 行
  登记逐位互核，拿 G11.3/G11.4 旧帧区值冒充 definitive 面即 RED），收敛阈消费
  g11_budget g11.fix.r1_ssim_shrink_tol 标定条目（标定程序产 p100×k 禁手写）；
  **不收敛则本门 FAIL、整波 FAIL**（§8.3a 不弱化声明 + G11_PLAN §2 G11.5 节
  M155 门预备注记——禁改判据充绿，未收敛如实 FAIL 停线报主会话裁决）。

RED 臂（契约判据字面）：未采样冒充修复即 RED（red_unsampled_masquerade——伪造
消费登记必检出）；delta 未收敛冒充闭环即 RED（red_unconverged_masquerade——
evaluate_closure 收敛断言语义 0-byte）；契约参数漂移即 RED
（contract_digest_locked_unchanged）；手写阈值/estimated 冒充标定即 RED；
g11.5 phase 增 red_stale_definitive_face（旧帧区复测值冒充 definitive 面必检出）。

用法：
  py -3 ci/g11_fix_r1_material_subset_smoke.py --gate g11.p0.m147.fix_r1_material_subset --phase g11.3
  py -3 ci/g11_fix_r1_material_subset_smoke.py --gate g11.p0.m147.fix_r1_material_subset --phase g11.5
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
import g11_5_retest_lib as rl  # noqa: E402
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

# g11.3 phase 检集（双 phase 校准后：11 检维持 + 收敛检改写为 verdict 显式登记形态）。
CHECK_KEYS = [
    "contract_digest_locked_unchanged",
    "material_subset_consumed",
    "rurix_frame_changed_vs_g10",
    "baseline_metric_reproduction",
    "closure_convergence_verdict_registered",
    "calibration_rerun_deterministic",
    "budget_entry_appended_measured_local",
    "budget_eval_strict_all_pass",
    "red_unsampled_masquerade_detected",
    "red_unconverged_masquerade_detected",
    "red_handwritten_threshold_detected",
    "red_estimated_masquerade_detected",
]

# 双 phase 校准前 legacy 检集（schema anyOf v1 支互核用——既有 evidence 形态 0-byte）。
CHECK_KEYS_LEGACY = [
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

# g11.5 phase 检集（收敛断言面——definitive 测量面 = G11.5 同契约复跑帧区；
# 契约 §8.3a 不弱化声明：不收敛则本门 FAIL、整波 FAIL，禁改判据充绿）。
CHECK_KEYS_G11_5 = [
    "contract_digest_locked_unchanged",
    "material_subset_consumed",
    "baseline_metric_reproduction",
    "retest_delta_g11_5_definitive_recomputed",
    "closure_delta_converged_measured",
    "closure_threshold_from_budget_calibrated",
    "budget_eval_strict_all_pass",
    "red_unconverged_masquerade_detected",
    "red_handwritten_threshold_detected",
    "red_estimated_masquerade_detected",
    "red_stale_definitive_face_detected",
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


def evaluate_g11_3_closure_registration(converged: bool, verdict: str | None, convergence_pending) -> bool:
    """g11.3 phase 收敛检 verdict 显式登记判定（§8.3a 双 phase 修订；不是 SKIP 充绿）：

    - 实测收敛：verdict == "converged" 且 convergence_pending == false；
    - 未收敛：verdict == "deferred_to_g11_5" 且 convergence_pending == true
      （收敛断言后移 G11.5 definitive 测量面，断言语义 0-byte 不弱化）；
    - convergence_pending 缺登记冒充全闭环（未收敛而 pending≠true / verdict 缺）
      或自相矛盾形态（收敛而 pending=true）一律红。
    """
    if converged:
        return verdict == "converged" and convergence_pending is False
    return verdict == "deferred_to_g11_5" and convergence_pending is True


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
    # 红臂②：未收敛冒充必检出（evaluate_closure 收敛断言语义 0-byte——g11.5 definitive 面同一判定层）。
    if fl.evaluate_closure(0.8328980787837229, 0.8328980787837229, 0.0)["converged"]:
        print(f"[{TAG}] selftest FAIL: 未收敛冒充未检出", file=sys.stderr)
        return 1
    # 绿臂（双 phase 登记形态）：g11.3 phase 下收敛红但其余全绿 → 门 PASS 且
    # convergence_pending 如实登记（verdict=deferred_to_g11_5 ∧ pending=true）。
    if not evaluate_g11_3_closure_registration(False, "deferred_to_g11_5", True):
        print(f"[{TAG}] selftest FAIL: deferred 如实登记形态误判（绿臂失效）", file=sys.stderr)
        return 1
    if not evaluate_g11_3_closure_registration(True, "converged", False):
        print(f"[{TAG}] selftest FAIL: 实测收敛登记形态误判（绿臂失效）", file=sys.stderr)
        return 1
    # 红臂③（双 phase 新增）：convergence_pending 缺登记冒充全闭环必检出。
    if evaluate_g11_3_closure_registration(False, None, None):
        print(f"[{TAG}] selftest FAIL: verdict 缺登记冒充全闭环未检出", file=sys.stderr)
        return 1
    if evaluate_g11_3_closure_registration(False, "converged", False):
        print(f"[{TAG}] selftest FAIL: 未收敛冒充实测收敛（pending 缺登记）未检出", file=sys.stderr)
        return 1
    if evaluate_g11_3_closure_registration(True, "deferred_to_g11_5", True):
        print(f"[{TAG}] selftest FAIL: 收敛而 pending=true 自相矛盾形态未检出", file=sys.stderr)
        return 1
    # schema anyOf 三支 checks.required 与三形态 CHECK_KEYS 闭集精确互核（M130 同构扩支）。
    schema = fl.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    branches = schema.get("anyOf", [])
    if len(branches) != 3:
        print(f"[{TAG}] selftest FAIL: schema 非 anyOf 三支形态", file=sys.stderr)
        return 1
    req_v1 = set(branches[0].get("properties", {}).get("checks", {}).get("required", []))
    req_v2 = set(branches[1].get("properties", {}).get("checks", {}).get("required", []))
    req_v3 = set(branches[2].get("properties", {}).get("checks", {}).get("required", []))
    if req_v1 != set(CHECK_KEYS_LEGACY):
        print(f"[{TAG}] selftest FAIL: v1 支 required 与 CHECK_KEYS_LEGACY 不等 {req_v1 ^ set(CHECK_KEYS_LEGACY)}", file=sys.stderr)
        return 1
    if req_v2 != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: v2 支 required 与 CHECK_KEYS 不等 {req_v2 ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    if req_v3 != set(CHECK_KEYS_G11_5):
        print(f"[{TAG}] selftest FAIL: v3 支 required 与 CHECK_KEYS_G11_5 不等 {req_v3 ^ set(CHECK_KEYS_G11_5)}", file=sys.stderr)
        return 1
    # g11.5 phase 红臂单元：未收敛冒充必检出（evaluate_closure 同一判定层语义 0-byte）。
    if fl.evaluate_closure(0.8328980787837229, 0.9903435577002249, 0.0)["converged"]:
        print(f"[{TAG}] selftest FAIL: g11.5 未收敛冒充（反向增大）未检出", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (8 RED + 5 GREEN)")
    return 0


def run_g11_5_gate() -> int:
    """--phase g11.5 收敛断言面（definitive 测量面 = G11.5 同契约复跑帧区；
    契约 §8.3a 不弱化声明：不收敛则本门 FAIL、整波 FAIL，禁改判据充绿）。"""
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS_G11_5}
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

    # ② 材质子集消费登记（G11.5 复跑报告全修复面——承接锚字面消费维持）。
    rep = rl.load_report()
    rurix_bistro = rep.get("results", {}).get("rurix", {}).get("bistro-interior", {})
    mats = (rurix_bistro.get("render_json", {}) or {}).get("materials", {}) or {}
    cons_problems = _consumption_problems(mats)
    checks["material_subset_consumed"] = not cons_problems
    check(not cons_problems, f"材质子集消费异常: {cons_problems[:3]}")

    # ③ 基线复现（G10.5 LDR 帧只读重算 == 锁定值 f64）。
    base_ssim = fl.ssim_ldr("bistro-interior", root=fl.FRAMES_G10_5)
    r1_row = fl.gap_row("R1")
    baseline = r1_row["measured_delta"][0]["delta"]
    baseline_a = r1_row["measured_delta"][0]["a_value"]
    repro_ok = (base_ssim == baseline_a and (1.0 - base_ssim) == baseline)
    checks["baseline_metric_reproduction"] = repro_ok
    check(repro_ok, f"基线复现漂移: ssim {base_ssim!r} ≠ 锁定 {baseline_a!r}")

    # ④ 复测 delta（definitive 面 = G11.5 帧区当次独立重算）与复测差距清单 R1 行
    # 登记逐位互核（拿 G11.3/G11.4 旧帧区值冒充 definitive 面即 RED）。
    retest_ssim = rl.ssim_ldr("bistro-interior")
    retest_delta = 1.0 - retest_ssim
    registry_r1 = {}
    if rl.RETEST_REGISTRY_PATH.is_file():
        for it in rl.load_retest_registry().get("items", []):
            if it.get("title", "").startswith("R1 "):
                registry_r1 = it
                break
    rep_face = rep.get("results", {}).get("metrics", {}).get("closure_faces", {}).get("r1", {})
    definitive_ok = (
        registry_r1.get("retest_delta") == retest_delta
        and rep_face.get("retest_delta") == retest_delta
        and rep_face.get("retest_ssim") == retest_ssim
    )
    checks["retest_delta_g11_5_definitive_recomputed"] = definitive_ok
    check(definitive_ok, f"definitive 面互核断裂: 门侧重算 {retest_delta!r} vs 清单 {registry_r1.get('retest_delta')!r} vs 报告 {rep_face.get('retest_delta')!r}")
    note(f"R1 修复前后 delta 对拍（g11.5 definitive 面）: 基线 {baseline}（ssim {baseline_a}）→ 复测 {retest_delta}（ssim {retest_ssim}）")

    # ⑤ 收敛阈消费 g11_budget 标定条目（标定程序产 p100×k 禁手写；evidence_file
    # 在树可解 results.trimmed_mean 且 threshold == trimmed_mean×k）。
    thr = rl.row_thresholds("R1")
    ent = thr["shrink_entry"]
    threshold = thr["shrink_tol"]
    thr_ok = ent.get("evidence") == "measured_local"
    ep = ROOT / (ent.get("evidence_file") or "")
    if ep.is_file():
        cal_doc = fl.load_json(ep)
        tm = cal_doc.get("results", {}).get("trimmed_mean")
        k_val = cal_doc.get("results", {}).get("safety_factor_k")
        thr_ok = thr_ok and ent.get("measured_value") == tm and ent.get("threshold") == tm * k_val
    else:
        thr_ok = False
    checks["closure_threshold_from_budget_calibrated"] = bool(thr_ok)
    check(thr_ok, "收敛阈 provenance 异常（手写阈值冒充标定即 RED）")

    # ⑥ 收敛断言（RXS-0393 L2 quality_gap 款字面；同一判定层 0-byte）——不收敛则
    # 本门 FAIL、整波 FAIL（§8.3a 不弱化声明；禁改判据充绿）。
    ev = fl.evaluate_closure(baseline, retest_delta, threshold)
    converged = bool(ev["converged"])
    verdict = "converged" if converged else "not_converged"
    checks["closure_delta_converged_measured"] = converged
    check(
        converged,
        f"R1 行 g11.5 phase 收敛断言不成立（不收敛则整波 FAIL——§8.3a 不弱化声明，禁改判据充绿）: "
        f"基线 {baseline!r} → 复测 {retest_delta!r}（反向增大 |复测| > |基线|，shrink {ev['shrink']!r} < 阈 {threshold!r}）",
    )
    if not converged:
        note(
            f"R1 g11.5 definitive 面实测未收敛如实 FAIL（不冒充闭环）：复测 delta {retest_delta!r} 反向增大"
            f"（基线 {baseline!r}）——锁定 SSIM 度量对正确修复结构性不友好（真实反照率 ≈0.10 下帧面较 UE "
            "暗 ≈150×，亮度/结构项塌陷；反向激励旁证 ssim(ue_修,rurix_未修白帧)=0.1624318277352612 > "
            "ssim(ue_修,rurix_修)=0.009656442299775102 入证据链）——整波 FAIL 停线报主会话裁决，"
            "G11.6 P2 候选行承接（契约 §8.3a 登记面）"
        )

    # ⑦ budget_eval --strict。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")

    # ⑧ RED 臂①：delta 未收敛冒充必检出（复测 == 基线 ⇒ 不收敛；同一判定层语义 0-byte）。
    forged_nc = fl.evaluate_closure(baseline, baseline, threshold)
    checks["red_unconverged_masquerade_detected"] = not forged_nc["converged"]
    check(checks["red_unconverged_masquerade_detected"], "未收敛冒充未检出")

    # ⑨ RED 臂②③：手写阈值 / estimated 冒充必拒。
    forged_entry = {
        "id": "g11.fix.red_probe",
        "evidence": "measured_local",
        "threshold": threshold + 0.25,
        "measured_value": threshold,
        "evidence_file": str(ep.relative_to(ROOT)).replace("\\", "/") if ep.is_file() else "milestones/g11/g11_budget.json",
    }
    checks["red_handwritten_threshold_detected"] = bool(fl.validate_budget_entry(forged_entry, threshold, 1.0))
    check(checks["red_handwritten_threshold_detected"], "手写阈值冒充未检出")
    forged_entry2 = dict(forged_entry, threshold=threshold, evidence="estimated")
    checks["red_estimated_masquerade_detected"] = bool(fl.validate_budget_entry(forged_entry2, threshold, 1.0))
    check(checks["red_estimated_masquerade_detected"], "estimated 冒充未检出")

    # ⑩ RED 臂④：旧帧区复测值冒充 definitive 面必检出（G11.3 帧区值 ≠ G11.5
    # definitive 重算——互核面对陈旧值断裂）。
    stale_ssim = fl.ssim_ldr("bistro-interior")  # G11.3 帧区（fl 默认 root）
    stale_delta = 1.0 - stale_ssim
    checks["red_stale_definitive_face_detected"] = stale_delta != retest_delta
    check(checks["red_stale_definitive_face_detected"], "旧帧区值与 definitive 面不可区分（陈旧冒充检出臂失效）")
    note(f"definitive 面陈旧检出旁证: G11.3 帧区 delta {stale_delta!r} ≠ G11.5 definitive {retest_delta!r}")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    closure = {
        "gap_row_id": r1_row["gap_id"],
        "baseline_delta": baseline,
        "retest_delta": retest_delta,
        "converged": converged,
        "verdict": verdict,
        "threshold_provenance": f"g11_budget 标定条目 {BUDGET_ENTRY_ID}（标定程序 ci/g11_fix_r1_material_subset_smoke.py 双跑噪声 p100×k={SAFETY_K}，P-09 禁手写；definitive 测量面 = G11.5 同契约复跑帧区）",
        "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
    }
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G11.5",
        "phase": "g11.5",
        "g11_5_phase_pass": all_pass,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "executed",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS_G11_5},
        "commands": COMMANDS,
        "closure": closure,
        "material_provenance": {
            "render_materials_block": mats,
            "rurix_frame_content_digest": rurix_bistro.get("frame_content_digest"),
            "retest_ssim_g11_5_definitive": retest_ssim,
            "baseline_ssim": baseline_a,
            "registry_r1_row_digest": registry_r1.get("gap_id"),
            "stale_g11_3_face_delta": stale_delta,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS_G11_5)} device=executed phase=g11.5")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（g11.5 phase：R1 修复前后 SSIM delta 收敛断言成立——基线 {baseline} → 复测 {retest_delta}）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--phase", choices=["g11.3", "g11.5"])
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    if args.phase is None:
        print(f"[{TAG}] FAIL: 缺 --phase（g11.3 修复落盘+局部度量登记期 / g11.5 收敛断言期）", file=sys.stderr)
        return 2
    if args.phase == "g11.5":
        return run_g11_5_gate()

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

    # ⑤ 复测 delta + 收敛判定 verdict 显式登记（g11.3 phase 登记面；收敛断言后移 g11.5）。
    retest_ssim = fl.ssim_ldr("bistro-interior")
    retest_delta = 1.0 - retest_ssim
    cal1 = compute_shrink_calibration()
    cal2 = compute_shrink_calibration()
    checks["calibration_rerun_deterministic"] = cal1 == cal2
    check(cal1 == cal2, "标定程序不可复跑（两跑漂移即 RED）")
    threshold = cal1["p100"] * SAFETY_K

    ev = fl.evaluate_closure(baseline, retest_delta, threshold)
    converged = bool(ev["converged"])
    verdict = "converged" if converged else "deferred_to_g11_5"
    convergence_pending = not converged
    checks["closure_convergence_verdict_registered"] = evaluate_g11_3_closure_registration(
        converged, verdict, convergence_pending
    )
    check(
        checks["closure_convergence_verdict_registered"],
        "收敛判定 verdict 登记异常（convergence_pending 缺登记冒充全闭环即 RED）",
    )
    note(f"R1 修复前后 delta 对拍: 基线 {baseline}（ssim {baseline_a}）→ 复测 {retest_delta}（ssim {retest_ssim}）")
    if converged:
        note("R1 复测 delta 实测收敛（verdict=converged，g11.3 phase 登记面提前闭环）")
    else:
        note(
            f"R1 未收敛如实登记（verdict=deferred_to_g11_5，convergence_pending=true——不是 SKIP 充绿）："
            f"复测 delta {retest_delta!r} 反向增大（基线 {baseline!r}），收敛断言后移 --phase g11.5 "
            "definitive 测量面（RXS-0393 L2；判据语义 0-byte 不弱化）。根因（measured 登记）：Rurix 帧"
            "真实反照率（纹理均值×(1−metallic)≈0.10）下单反弹 GI + 无点光源（R3/R4 承接面 G11.4）"
            "使帧面较 UE 暗 ≈150×，SSIM 亮度/结构项塌陷——材质采样本身经 manifest/均值/digest 面"
            "核验正确落地；耦合证据链 milestones/g11/g11_2_residual_caliber_registry.json"
        )
    # 反向激励旁证（measured 入证据链，G11.6 P2 候选行消费）：ssim(ue_修, rurix_未修白帧) vs
    # ssim(ue_修, rurix_修)——锁定度量对「未修复白帧」评分反高于「正确采样纹理的暗帧」。
    ssim_ue_fixed_vs_rurix_unfixed = fl.ssim_ldr_cross(
        "bistro-interior", "ue5", fl.FRAMES_G11_3, "rurix", fl.FRAMES_G10_5
    )
    note(
        f"反向激励旁证（measured）：ssim(ue_修, rurix_未修复 G10.5 帧)={ssim_ue_fixed_vs_rurix_unfixed} "
        f"> ssim(ue_修, rurix_修)={retest_ssim}——锁定度量对正确修复结构性不友好（G11.6 P2 候选行登记）"
    )

    closure = {
        "gap_row_id": r1_row["gap_id"],
        "baseline_delta": baseline,
        "retest_delta": retest_delta,
        "converged": converged,
        "verdict": verdict,
        "convergence_pending": convergence_pending,
        "threshold_provenance": f"标定程序 ci/g11_fix_r1_material_subset_smoke.py（LDR SSIM 双跑噪声 p100×k={SAFETY_K}，样本集 = G11.3 bistro LDR 帧对；budget 条目 {BUDGET_ENTRY_ID}）",
        "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
    }
    if convergence_pending:
        closure["deferred_to"] = "g11.5"
        closure["defer_reason"] = (
            "R1 局部 SSIM 度量被 R3（点光源子集）/R4（多反弹 GI）光照残余结构性主导——耦合解消归 "
            "G11.4 承接面，definitive 收敛测量面 = G11.5 同契约复跑（RXS-0393 L2 quality_gap 款字面；"
            "契约 §8.3a 修订句：断言不弱化只后移，G11.5 不收敛则整波 FAIL）"
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

    # ⑨ RED 臂②：delta 未收敛冒充必检出（收敛判定层语义 0-byte——g11.5 definitive 面同一判定）。
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
        "phase": "g11.3",
        "g11_3_phase_pass": all_pass,
        "convergence_pending": convergence_pending,
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
            "anti_incentive_ssim_ue_fixed_vs_rurix_unfixed_g10_5": ssim_ue_fixed_vs_rurix_unfixed,
            "anti_incentive_ssim_ue_fixed_vs_rurix_fixed": retest_ssim,
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
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=executed phase=g11.3")
    if all_pass and not FAILURES:
        print(
            f"[{TAG}] PASS（g11.3 phase：R1 材质子集消费闭环 + 局部度量 measured 登记——"
            f"delta {baseline} → {retest_delta} verdict={verdict} convergence_pending={str(convergence_pending).lower()}；"
            "收敛断言归 --phase g11.5 definitive 面 + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
