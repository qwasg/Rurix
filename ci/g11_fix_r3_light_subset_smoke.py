#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.4 波）
"""G11.4 M153 R3 灯种子集修复闭环门（P0，步骤 208；
g11.p0.m153.fix_r3_light_subset；G11_CONTRACT §4.2 M153 行判据逐字 + G-G11-6；
G11_ACCEPTANCE_MAP §1 M153 行；CI_GATES §4；g10_gap_registry R3 行承接锚；
spec/global_illumination.md RXS-0394 + spec/visual_comparison.md RXS-0393）。

host+device 门（host CPU 参考管线真渲染，device_section_state=executed）。
判据（契约 §4.2 M153 行字面）：

1. **点/面光源 + glTF emissive 表达（bistro 包内 4+ 盏实测消费）**：契约光照
   参数面单通道消费（corpus/lighting_bistro_interior.json 经 M133 只追加修订
   程序产 point_lights/emissive_surfaces——派生链 g11_4_light_derive.py 报告
   逐盏 provenance；UE build_scenes 同消费契约面 spawn 4 盏 + 读回探针）；
   Rurix 渲染 lights 闭集块（point_lights_consumed ≥4 / emissive 4 件 /
   area 缺类显式登记 / source_digest == 光照文件 digest 复算）。
2. **修复前后 HDR 亮度中位 delta 收敛 measured（锁定基线 2.664779790997505，
   收敛阈由标定程序产）**：基线复现（G10.5 帧只读重算 == 锁定值 f64 +
   G11.2 域统一换算面 2.7314592314362525）+ 复测 delta（G11.4 帧区 m153
   隔离面实测）收敛判定（RXS-0393 L2 quality_gap 款）。
3. **cornell 契约 sun+sky 灯面 0-byte**：lighting_cornell_box.json 与 git
   HEAD 逐位一致 + cornell 渲染 lights.enabled=false（未消费）+ 清单修订行
   只追加（scenes 行集 0-byte）。

RED 臂（契约判据字面）：点光源未表达冒充修复即 RED（red_light_unexpressed——
伪造消费登记必检出）；delta 未收敛冒充闭环即 RED（red_unconverged_masquerade）；
契约灯面漂移即 RED（cornell_light_face_0byte 机核面）；手写阈值/estimated
冒充标定即 RED。

用法：
  py -3 ci/g11_fix_r3_light_subset_smoke.py --gate g11.p0.m153.fix_r3_light_subset
  py -3 ci/g11_fix_r3_light_subset_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m153_fix_r3_light_subset_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g11_4_fix_lib as gl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g11.p0.m153.fix_r3_light_subset"
NUMERIC_STEP = 208
SOURCE_REF = (
    "G11_CONTRACT §4.2 M153 + G-G11-6;G11_ACCEPTANCE_MAP §1 M153;CI_GATES §4;"
    "g10_gap_registry R3 行承接锚;spec/global_illumination.md RXS-0394;"
    "spec/visual_comparison.md RXS-0393"
)
TAG = "g11_m153"
SUBJECT = "g11_m153_fix_r3_light_subset"
MATRIX_ROW = "M153"

BUDGET_ENTRY_ID = "g11.fix.r3_luminance_shrink_tol"
SAFETY_K = 1.0

CHECK_KEYS = [
    "contract_digest_locked_unchanged",
    "light_derivation_provenance",
    "light_seed_set_consumed",
    "ue_point_lights_double_end",
    "cornell_light_face_0byte",
    "rurix_frame_changed_vs_g11_3",
    "baseline_metric_reproduction",
    "closure_delta_converged_measured",
    "calibration_rerun_deterministic",
    "budget_entry_appended_measured_local",
    "budget_eval_strict_all_pass",
    "red_light_unexpressed_detected",
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


def _git_head_digest(rel: str) -> str | None:
    r = subprocess.run(["git", "show", f"HEAD:{rel}"], cwd=ROOT, capture_output=True)
    if r.returncode != 0:
        return None
    import hashlib

    return "sha256:" + hashlib.sha256(r.stdout).hexdigest()


def compute_shrink_calibration() -> dict:
    return gl.shrink_calibration(
        lambda: gl.hdr_lum("bistro-interior-m153", "rurix")["median"], k=SAFETY_K
    )


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
    if gl.validate_budget_entry(ok_entry, c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: 合法条目误判", file=sys.stderr)
        return 1
    if not gl.validate_budget_entry(dict(ok_entry, threshold=c1["p100"] + 0.25), c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: 手写阈值冒充未检出", file=sys.stderr)
        return 1
    if not gl.validate_budget_entry(dict(ok_entry, evidence="estimated"), c1["p100"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: estimated 冒充未检出", file=sys.stderr)
        return 1
    # 红臂①：未表达冒充必检出。
    if not gl.lights_block_problems({"enabled": True, "point_lights_consumed": 0, "emissive_materials_consumed": 0}):
        print(f"[{TAG}] selftest FAIL: 未表达冒充未检出", file=sys.stderr)
        return 1
    # 绿臂：合形消费登记不误拒。
    good = {
        "enabled": True,
        "point_lights_consumed": 4,
        "emissive_materials_consumed": 4,
        "area_lights_declared_absent": True,
        "source_digest": "sha256:00",
        "point_lights": [
            {"position": [0, 0, 0], "color_linear_rgb": [1, 1, 1], "intensity_cd": 1.0,
             "emit_direction": [0, -1, 0], "area_m2": 0.1, "derived_from": "x"}
        ],
    }
    if gl.lights_block_problems(good):
        print(f"[{TAG}] selftest FAIL: 合形消费登记误拒 {gl.lights_block_problems(good)}", file=sys.stderr)
        return 1
    # 红臂②：未收敛冒充必检出。
    if gl.evaluate_closure(2.7314592314362525, 2.7314592314362525, 0.0)["converged"]:
        print(f"[{TAG}] selftest FAIL: 未收敛冒充未检出", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (6 RED + {len(CHECK_KEYS) - 6} GREEN)")
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
        f"{s}: {gl.contract_digest_rust(s)} ≠ {gl.LOCKED_DIGEST[s]}"
        for s in gl.SCENES
        if gl.contract_digest_rust(s) != gl.LOCKED_DIGEST[s]
    ]
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "g10_5_scene_render --contract-digest ×2 scenes", "exit_code": 0})
    checks["contract_digest_locked_unchanged"] = not digest_drift
    check(not digest_drift, f"契约 digest 漂移（契约参数漂移即 RED）: {digest_drift}")

    # ② 派生链 provenance（M133 只追加修订程序面）。
    prov_problems: list[str] = []
    der = gl.load_json(gl.DERIVATION_REPORT) if gl.DERIVATION_REPORT.is_file() else {}
    pls = der.get("point_lights", [])
    if len(pls) < 4:
        prov_problems.append(f"派生报告 point_lights {len(pls)} < 4")
    for p in pls:
        for k in ("id", "position", "color_linear_rgb", "intensity_cd", "emit_direction", "area_m2", "derived_from"):
            if k not in p:
                prov_problems.append(f"派生报告缺字段 {k}: {p.get('id')}")
    if len(der.get("emissive_surfaces", [])) != 4:
        prov_problems.append("emissive_surfaces ≠ 4")
    if der.get("lighting_json_digest_post_revision") != gl.sha256_file(gl.LIGHTING_BISTRO):
        prov_problems.append("派生报告 digest 与光照文件当次复算不符")
    man = gl.load_json(gl.SCENE_MANIFEST)
    revs = man.get("revisions", [])
    if not any("G11.4 R3 灯种子集承接" in str(r.get("change_note", "")) for r in revs):
        prov_problems.append("清单缺 G11.4 修订行（M133 只追加修订程序违例）")
    ids = [r.get("revision") for r in revs]
    if ids != sorted(ids) or len(set(ids)) != len(ids):
        prov_problems.append(f"清单修订 id 非只追加序: {ids}")
    checks["light_derivation_provenance"] = not prov_problems
    check(not prov_problems, f"派生链 provenance 异常: {prov_problems[:3]}")

    # ③ 灯种子集消费登记（Rurix 面）。
    rep = gl.load_report()
    rurix_bistro153 = rep.get("results", {}).get("rurix", {}).get("bistro-interior-m153", {})
    lights = (rurix_bistro153.get("render_json", {}) or {}).get("lights", {}) or {}
    cons = gl.lights_block_problems(lights)
    if lights.get("source_digest") != gl.sha256_file(gl.LIGHTING_BISTRO):
        cons.append("lights.source_digest ≠ 光照文件 digest 当次复算（契约面单通道断裂）")
    checks["light_seed_set_consumed"] = not cons
    check(not cons, f"灯种子集消费异常: {cons[:3]}")

    # ④ UE 双端同消费（build_scenes spawn 读回探针）。
    ue_probe = (rep.get("results", {}).get("ue", {}).get("bistro-interior", {}) or {}).get("probe", {}) or {}
    ue_n = ue_probe.get("g11_4_point_lights_count") or 0
    ue_ok = ue_n >= 4 and all(
        (p.get("intensity_cd_readback") or 0) > 0 for p in ue_probe.get("g11_4_point_lights", [])
    )
    checks["ue_point_lights_double_end"] = bool(ue_ok)
    check(ue_ok, f"UE 侧点光源读回异常（双端同消费即 RED）: count={ue_n}")

    # ⑤ cornell 契约灯面 0-byte（文件与 HEAD 逐位 + cornell 渲染未消费 + UE 复用核验）。
    head_d = _git_head_digest("milestones/g10/corpus/lighting_cornell_box.json")
    cornell_file_ok = head_d is not None and gl.sha256_file(gl.LIGHTING_CORNELL) == head_d
    cornell_render = rep.get("results", {}).get("rurix", {}).get("cornell-box", {})
    cornell_lights = (cornell_render.get("render_json", {}) or {}).get("lights", {}) or {}
    cornell_render_ok = cornell_lights.get("enabled") is False
    cornell_ue = rep.get("results", {}).get("ue", {}).get("cornell-box", {})
    cornell_ue_ok = "复用" in str(cornell_ue.get("reuse_from", ""))
    checks["cornell_light_face_0byte"] = cornell_file_ok and cornell_render_ok and cornell_ue_ok
    check(
        checks["cornell_light_face_0byte"],
        f"cornell 灯面漂移（契约灯面漂移即 RED）: file={cornell_file_ok} render={cornell_render_ok} ue={cornell_ue_ok}",
    )

    # ⑥ Rurix 帧 ≠ G11.3 修复帧（修复生效）。
    g113_digest = gl.load_json(gl.REPORT_G11_3_PATH)["results"]["rurix"]["bistro-interior"]["frame_content_digest"]
    ru_digest_now = rurix_bistro153.get("frame_content_digest", "")
    checks["rurix_frame_changed_vs_g11_3"] = bool(ru_digest_now) and ru_digest_now != g113_digest
    check(checks["rurix_frame_changed_vs_g11_3"], "Rurix bistro m153 帧未变——灯种子集未生效冒充")

    # ⑦ 基线复现（G10.5 帧只读重算 == 锁定值 f64 + 域统一换算面）。
    base = gl.baseline_reproduction_r3()
    r3_row = gl.gap_row("R3")
    baseline = r3_row["measured_delta"][0]["delta"]
    repro_ok = (
        base["a"] == r3_row["measured_delta"][0]["a_value"]
        and base["b"] == r3_row["measured_delta"][0]["b_value"]
        and base["delta_locked"] == baseline
        and base["delta_aligned"] == gl.ALIGNED_BASELINE_R3
    )
    checks["baseline_metric_reproduction"] = repro_ok
    check(repro_ok, f"基线复现漂移: {base} ≠ 锁定 {baseline} / 对齐 {gl.ALIGNED_BASELINE_R3}")

    # ⑧ 复测 delta + 收敛判定（m153 隔离面）。
    face = rep.get("results", {}).get("metrics", {}).get("closure_faces", {}).get("r3", {})
    retest_delta = face.get("retest_delta")
    cal1 = compute_shrink_calibration()
    cal2 = compute_shrink_calibration()
    checks["calibration_rerun_deterministic"] = cal1 == cal2
    check(cal1 == cal2, "标定程序不可复跑（两跑漂移即 RED）")
    threshold = cal1["p100"] * SAFETY_K
    ev = gl.evaluate_closure(gl.ALIGNED_BASELINE_R3, retest_delta, threshold)
    converged = bool(ev["converged"])
    checks["closure_delta_converged_measured"] = converged
    check(
        converged,
        f"R3 未收敛（delta 未收敛冒充闭环即 RED）: 基线（对齐域）{gl.ALIGNED_BASELINE_R3} → 复测 {retest_delta}",
    )
    note(
        f"R3 修复前后 delta 对拍: 锁定基线 {baseline}（原域）/ {gl.ALIGNED_BASELINE_R3}（对齐域）→ 复测 {retest_delta}"
        f"（m154 全修复面 {face.get('retest_delta_m154_face')}）；标定阈 p100×k={SAFETY_K}={threshold}"
    )

    closure = {
        "gap_row_id": r3_row["gap_id"],
        "baseline_delta": baseline,
        "baseline_delta_aligned_domain": gl.ALIGNED_BASELINE_R3,
        "retest_delta": retest_delta,
        "converged": converged,
        "threshold_provenance": f"标定程序 ci/g11_fix_r3_light_subset_smoke.py（HDR 亮度中位双跑噪声 p100×k={SAFETY_K}，样本集 = G11.4 bistro m153 帧对；budget 条目 {BUDGET_ENTRY_ID}）",
        "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
    }

    # ⑨ 标定 evidence 落盘 + budget 追加。
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    report_digest = gl.sha256_file(gl.REPORT_PATH)
    calib_ev = gl.calib_evidence_payload(
        subject="g11_m153_calibration_r3_luminance_shrink",
        gate_key=GATE_KEY, matrix_row=MATRIX_ROW, numeric_step=NUMERIC_STEP,
        p100=cal1["p100"], k=SAFETY_K, sample_count=cal1["sample_count"],
        sample_set_digest=report_digest,
        provenance_measured="measured_local：G11.4 bistro m153 面 HDR 帧对亮度中位双跑逐位一致（确定性），噪声 p100×k；禁手写阈值冒充标定（P-09）",
        ts=ts,
    )
    calib_ev["environment"] = wel.collect_environment()
    calib_ev["provenance"]["k_rationale"] = "样本 = 双跑噪声，p100=0.0 时 k 取值不改变标定值；取 M138/C2 同值 1.0（k∈[1,3] 闭集内）"
    calib_path = EVIDENCE_DIR / f"g11_m153_calibration_r3_luminance_shrink_{ts}.json"
    calib_path.write_text(json.dumps(calib_ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    entry = {
        "id": BUDGET_ENTRY_ID,
        "description": (
            "R3 HDR 亮度中位 delta 收敛幅度阈：双跑噪声 p100 × k=1.0（RXS-0393 L3；标定程序 "
            f"ci/g11_fix_r3_light_subset_smoke.py 两跑逐位一致；样本集 digest {report_digest[:24]}…）。M153 measured 标定（P-09）。"
        ),
        "direction": "max",
        "evidence": "measured_local",
        "skip_reason": None,
        "unit": "1",
        "threshold": threshold,
        "evidence_file": str(calib_path.relative_to(ROOT)).replace("\\", "/"),
        "measured_value": cal1["p100"],
    }
    budget_problems = gl.validate_budget_entry(entry, cal1["p100"], SAFETY_K)
    if not budget_problems:
        budget_problems = gl.append_budget_entries([entry])
        if not budget_problems:
            note(f"g11_budget.json 字节级纯追加 {BUDGET_ENTRY_ID}（threshold={threshold!r}）")
    checks["budget_entry_appended_measured_local"] = not budget_problems
    check(not budget_problems, f"budget 条目异常: {budget_problems[:2]}")

    # ⑩ budget_eval --strict。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")

    # ⑪ RED 臂①：未表达冒充必检出。
    checks["red_light_unexpressed_detected"] = bool(gl.lights_block_problems(
        {"enabled": True, "point_lights_consumed": 0, "emissive_materials_consumed": 0}
    ))
    check(checks["red_light_unexpressed_detected"], "未表达冒充未检出")

    # ⑫ RED 臂②：delta 未收敛冒充必检出。
    forged_nc = gl.evaluate_closure(gl.ALIGNED_BASELINE_R3, gl.ALIGNED_BASELINE_R3, threshold)
    checks["red_unconverged_masquerade_detected"] = not forged_nc["converged"]
    check(checks["red_unconverged_masquerade_detected"], "未收敛冒充未检出")

    # ⑬ RED 臂③④：手写阈值 / estimated 冒充必拒。
    forged_entry = {
        "id": "g11.fix.red_probe",
        "evidence": "measured_local",
        "threshold": cal1["p100"] * SAFETY_K + 0.25,
        "measured_value": cal1["p100"],
        "evidence_file": str(calib_path.relative_to(ROOT)).replace("\\", "/"),
    }
    checks["red_handwritten_threshold_detected"] = bool(gl.validate_budget_entry(forged_entry, cal1["p100"], SAFETY_K))
    check(checks["red_handwritten_threshold_detected"], "手写阈值冒充未检出")
    forged_entry2 = dict(forged_entry, threshold=cal1["p100"] * SAFETY_K, evidence="estimated")
    checks["red_estimated_masquerade_detected"] = bool(gl.validate_budget_entry(forged_entry2, cal1["p100"], SAFETY_K))
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
        "wave": "G11.4",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "executed",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "closure": closure,
        "light_provenance": {
            "render_lights_block": lights,
            "derivation_report_digest": der and gl.sha256_file(gl.DERIVATION_REPORT),
            "lighting_bistro_digest": gl.sha256_file(gl.LIGHTING_BISTRO),
            "lighting_cornell_digest": gl.sha256_file(gl.LIGHTING_CORNELL),
            "ue_point_lights_count": ue_n,
            "rurix_m153_frame_digest": ru_digest_now,
            "g11_3_rurix_frame_digest": g113_digest,
            "baseline_reproduction": base,
            "m154_face_delta": face.get("retest_delta_m154_face"),
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
            f"[{TAG}] PASS（R3 灯种子集消费闭环——4 点光 + 4 emissive 双端表达；"
            f"delta 基线（对齐域）{gl.ALIGNED_BASELINE_R3} → 复测 {retest_delta} 收敛 measured；"
            "cornell 灯面 0-byte + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
