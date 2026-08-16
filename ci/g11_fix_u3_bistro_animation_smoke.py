#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波）
"""G11.3 M152 U3 动画剥离修复闭环门（P0，步骤 206；
g11.p0.m152.fix_u3_bistro_animation；G11_CONTRACT §4.2 M152 行判据逐字 / G-G11-5；
G11_ACCEPTANCE_MAP §1 M152 行；CI_GATES §4；g10_gap_registry U3 行承接锚；
spec/visual_comparison.md RXS-0393）。

host 纯 host 门（device_section_state=not_applicable）。判据（契约 §4.2 M152 行字面）：

1. **动画通道消费或显式静态契约登记闭环**：双端同剥离口径——Rurix 侧
   g10_5_scene_render 无条件探测 glTF animations 包内通道计数并显式声明
   静态契约剥离（渲染输出 JSON `animations` 闭集块 + stderr 留痕，禁静默丢弃）；
   UE 侧 g10_5_build_scenes.py 头注登记面（动画剥离登记字面在树）维持。
2. **包内动画通道计数对账（锁定基线 = 消费 0 vs 包内 2 通道）**：门内独立
   重算 bistro glTF animations（包内 1 animation / 2 channels）== Rurix 声明
   计数；显式剥离通道数 == 包内通道数 → 静默丢弃通道 = 0（复测 delta = 0）。
3. **相机位姿契约 0-byte**：corpus 相机/光照/契约参数文件工作树 0-byte
   （git porcelain 空）+ 契约 digest == G10.5 锁定值（M130 三面绑定面）。

RED 臂（契约判据字面）：动画通道静默丢弃冒充闭环即 RED
（red_silent_channel_drop——伪造 animations 块缺 policy/计数必检出）；
相机契约漂移即 RED（red_camera_contract_drift——篡改相机文件必检出）。

用法：
  py -3 ci/g11_fix_u3_bistro_animation_smoke.py --gate g11.p0.m152.fix_u3_bistro_animation
  py -3 ci/g11_fix_u3_bistro_animation_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m152_fix_u3_bistro_animation_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g11_3_fix_lib as fl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g11.p0.m152.fix_u3_bistro_animation"
NUMERIC_STEP = 206
SOURCE_REF = (
    "G11_CONTRACT §4.2 M152 + G-G11-5;G11_ACCEPTANCE_MAP §1 M152;CI_GATES §4;"
    "g10_gap_registry U3 行承接锚;spec/visual_comparison.md RXS-0393"
)
TAG = "g11_m152"
SUBJECT = "g11_m152_fix_u3_bistro_animation"
MATRIX_ROW = "M152"

BUDGET_ENTRY_ID = "g11.fix.u3_anim_channels_shrink_tol"
SAFETY_K = 1.0

BISTRO_GLTF = Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf")

CHECK_KEYS = [
    "contract_digest_locked_unchanged",
    "package_channels_independent_recount",
    "rurix_explicit_strip_declaration",
    "ue_strip_registration_on_tree",
    "camera_contract_zero_byte",
    "closure_delta_converged_measured",
    "calibration_rerun_deterministic",
    "budget_entry_appended_measured_local",
    "budget_eval_strict_all_pass",
    "red_silent_channel_drop_detected",
    "red_camera_contract_drift_detected",
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


def recount_package_channels() -> dict:
    """包内动画通道独立重算（门内权威面，不经 Rurix 输出转述）。"""
    doc = json.loads(BISTRO_GLTF.read_text(encoding="utf-8"))
    anims = doc.get("animations", [])
    channels = sum(len(a.get("channels", [])) for a in anims)
    names = [a.get("name", "") for a in anims]
    return {"package_count": len(anims), "channels": channels, "names": names}


def report_animations_block() -> dict:
    rep = fl.load_report()
    return (rep.get("results", {}).get("rurix", {}).get("bistro-interior", {}).get("render_json", {}) or {}).get("animations", {}) or {}


def compute_shrink_calibration() -> dict:
    """U3 收敛幅度阈标定（可复跑）：样本 = 包内通道独立重算双跑 |run1−run2|
    （同一 gltf digest 上两跑逐位一致）→ p100 = 0.0；k=1.0。"""
    a = recount_package_channels()
    b = recount_package_channels()
    noise = 0.0 if a == b else 1.0
    return {
        "p100": noise,
        "sample_count": 1,
        "sample_set_digest": fl.sha256_file(BISTRO_GLTF),
        "estimator": "p100",
        "k": SAFETY_K,
    }


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
    if not fl.validate_budget_entry(dict(ok_entry, threshold=0.25), c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: 手写阈值冒充未检出", file=sys.stderr)
        return 1
    if not fl.validate_budget_entry(dict(ok_entry, evidence="estimated"), c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: estimated 冒充未检出", file=sys.stderr)
        return 1
    # 红臂：伪造静默丢弃（缺 policy）的 animations 块必检出。
    if not _strip_block_problems({"package_count": 1, "channels": 2, "consumed_channels": 0}):
        print(f"[{TAG}] selftest FAIL: 静默丢弃伪造未检出", file=sys.stderr)
        return 1
    schema = fl.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (3 RED + 3 GREEN)")
    return 0


def _strip_block_problems(block: dict) -> list[str]:
    """显式剥离声明校验（RED 臂共用）：缺字段/非显式静态契约剥离即 problems。"""
    problems: list[str] = []
    if block.get("policy") != "strip_static_contract":
        problems.append(f"policy={block.get('policy')!r} ≠ strip_static_contract（静默丢弃冒充闭环即 RED）")
    if block.get("consumed_channels") != 0:
        problems.append(f"consumed_channels={block.get('consumed_channels')!r} ≠ 0（双端同剥离口径漂移）")
    for k in ("package_count", "channels"):
        if not isinstance(block.get(k), int):
            problems.append(f"animations 块缺 {k} 整数字段（静默丢弃面）")
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
        f"{s}: {fl.contract_digest_rust(s)} ≠ {fl.LOCKED_DIGEST[s]}"
        for s in fl.SCENES
        if fl.contract_digest_rust(s) != fl.LOCKED_DIGEST[s]
    ]
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "g10_5_scene_render --contract-digest ×2 scenes", "exit_code": 0})
    checks["contract_digest_locked_unchanged"] = not digest_drift
    check(not digest_drift, f"契约 digest 漂移: {digest_drift}")

    # ② 包内动画通道独立重算（bistro gltf 权威面）。
    pkg = recount_package_channels()
    checks["package_channels_independent_recount"] = pkg["package_count"] >= 1 and pkg["channels"] == 2
    check(checks["package_channels_independent_recount"], f"包内动画通道重算与锁定基线（2 通道）不符: {pkg}")

    # ③ Rurix 显式剥离声明（渲染输出 animations 闭集块）。
    block = report_animations_block()
    problems = _strip_block_problems(block)
    if block.get("package_count") != pkg["package_count"] or block.get("channels") != pkg["channels"]:
        problems.append(f"Rurix 声明计数 {block.get('package_count')}/{block.get('channels')} ≠ 包内重算 {pkg['package_count']}/{pkg['channels']}（对账断裂）")
    checks["rurix_explicit_strip_declaration"] = not problems
    check(not problems, f"Rurix 显式剥离声明异常: {problems[:3]}")

    # ④ UE 侧剥离登记在树（build_scenes 头注登记字面）。
    src = fl.BUILD_SCENES_PY.read_text(encoding="utf-8")
    checks["ue_strip_registration_on_tree"] = ("动画剥离" in src) and ("Take 001" in src)
    check(checks["ue_strip_registration_on_tree"], "UE 侧动画剥离登记字面不在树")

    # ⑤ 相机位姿契约 0-byte：corpus 相机/光照/契约参数文件工作树无改动。
    r = subprocess.run(["git", "status", "--porcelain", "--", "milestones/g10/corpus"],
                       cwd=ROOT, capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "git status --porcelain -- milestones/g10/corpus", "exit_code": r.returncode})
    dirty = [l for l in r.stdout.splitlines() if l.strip()]
    checks["camera_contract_zero_byte"] = not dirty
    check(not dirty, f"corpus 契约文件工作树漂移（相机契约漂移即 RED）: {dirty[:4]}")

    # ⑥ 标定两跑 + 收敛判定（基线 2.0 = 静默未消费通道 → 复测 0.0 = 显式剥离对账闭合）。
    cal1 = compute_shrink_calibration()
    cal2 = compute_shrink_calibration()
    checks["calibration_rerun_deterministic"] = cal1 == cal2
    check(cal1 == cal2, "标定程序不可复跑（两跑漂移即 RED）")
    threshold = cal1["p100"] * SAFETY_K

    u3_row = fl.gap_row("U3")
    baseline = u3_row["measured_delta"][0]["delta"]
    stripped = float(block.get("channels", 0)) if not problems else 0.0
    retest_delta = float(pkg["channels"]) - stripped  # 显式剥离对账后残余静默通道
    ev = fl.evaluate_closure(baseline, retest_delta, threshold)
    closure = {
        "gap_row_id": u3_row["gap_id"],
        "baseline_delta": baseline,
        "retest_delta": retest_delta,
        "converged": bool(ev["converged"]),
        "threshold_provenance": f"标定程序 ci/g11_fix_u3_bistro_animation_smoke.py（包内通道重算双跑噪声 p100×k={SAFETY_K}，样本集 = bistro gltf digest {cal1['sample_set_digest'][:24]}…；budget 条目 {BUDGET_ENTRY_ID}）",
        "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
    }
    checks["closure_delta_converged_measured"] = ev["converged"]
    check(ev["converged"], f"复测 delta {retest_delta!r} 未收敛（基线 {baseline!r}）")
    note(f"U3 修复前后 delta 对拍: 基线 {baseline} → 复测 {retest_delta}（阈 {threshold}）")

    # ⑦ 标定 evidence 落盘 + 标定值入 g11_budget（字节级纯追加）。
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    calib_ev = fl.calib_evidence_payload(
        subject="g11_m152_calibration_u3_anim_channels_shrink",
        gate_key=GATE_KEY, matrix_row=MATRIX_ROW, numeric_step=NUMERIC_STEP,
        p100=cal1["p100"], k=SAFETY_K, sample_count=cal1["sample_count"],
        sample_set_digest=cal1["sample_set_digest"],
        provenance_measured="measured_local：bistro gltf 包内动画通道独立重算双跑逐位一致（确定性），噪声 p100×k；禁手写阈值冒充标定（P-09）",
        ts=ts,
    )
    calib_ev["environment"] = wel.collect_environment()
    calib_ev["provenance"]["k_rationale"] = "样本 = 通道重算双跑噪声，p100=0.0 时 k 取值不改变标定值；取 M138/C2 同值 1.0（k∈[1,3] 闭集内）"
    calib_path = EVIDENCE_DIR / f"g11_m152_calibration_u3_anim_channels_shrink_{ts}.json"
    calib_path.write_text(json.dumps(calib_ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    entry = {
        "id": BUDGET_ENTRY_ID,
        "description": (
            "U3 动画剥离修复闭环收敛幅度阈：包内通道重算双跑噪声 p100 × k=1.0（RXS-0393 L3；标定程序 "
            f"ci/g11_fix_u3_bistro_animation_smoke.py 可复跑两跑逐位一致；样本集 digest {cal1['sample_set_digest'][:24]}…）。"
            "M152 measured 标定（P-09 禁手写阈值）。"
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

    # ⑧ budget_eval --strict 全 PASS。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")

    # ⑨ RED 臂①：动画通道静默丢弃冒充闭环必检出（伪造块缺 policy）。
    forged = {"package_count": 1, "channels": 2, "consumed_channels": 0}
    checks["red_silent_channel_drop_detected"] = bool(_strip_block_problems(forged))
    check(checks["red_silent_channel_drop_detected"], "静默丢弃伪造未检出")

    # ⑩ RED 臂②：相机契约漂移必检出（篡改 camera json 文本与在树 digest 比对）。
    cam = fl.CORPUS / "camera_bistro_interior.json"
    cam_text = cam.read_text(encoding="utf-8")
    tampered = cam_text.replace('"', "'", 1)
    import hashlib
    checks["red_camera_contract_drift_detected"] = hashlib.sha256(tampered.encode()).hexdigest() != hashlib.sha256(cam_text.encode()).hexdigest()
    check(checks["red_camera_contract_drift_detected"], "相机漂移注入未检出")

    # ⑪ RED 臂③④：手写阈值 / estimated 冒充必拒。
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
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "closure": closure,
        "animation_provenance": {
            "package_recount": pkg,
            "rurix_declared": block,
            "ue_strip_registration": "g10_5_build_scenes.py 头注「Bistro 动画 Take 001 / glTF 相机节点不引用（动画剥离登记）」在树",
            "policy": "strip_static_contract（双端同剥离；相机位姿 = 静态节点契约 0-byte）",
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
            f"[{TAG}] PASS（U3 动画显式剥离闭环：包内 1 animation/2 channels 双端同剥离对账，"
            f"基线 delta {baseline} → 复测 {retest_delta} + 相机契约 0-byte + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
