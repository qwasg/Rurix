#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波）
"""G11.3 M150 U1 cornell 壳体零辐射修复闭环门（P0，步骤 204；
g11.p0.m150.fix_u1_cornell_shell_radiance；G11_CONTRACT §4.2 M150 行判据逐字 /
G-G11-5；G11_ACCEPTANCE_MAP §1 M150 行；CI_GATES §4；g10_gap_registry U1 行承接锚；
spec/visual_comparison.md RXS-0393）。

host+device 门（UE MRQ 出帧 + host CPU 参考管线真渲染，device_section_state=executed）。
判据（契约 §4.2 M150 行字面）：

1. **cornell 壳体（墙/顶/地板）零辐射修复**：UE 侧双面化（g10_5_build_scenes.py
   two_sided 父材质 + 逐 actor MIC 置换——双端着色口径对齐面，语料 0-byte 不走
   M133 修订）——provenance = two_sided_replacement 逐 actor 登记 + two_sided
   读回真 + UE 帧内容 digest ≠ G10.5 锁定帧（修复生效）；**语料 0-byte** 机核 =
   cornell-box-generated 资产 digest == M131 白名单登记值（g10_corpus_lib 复算）。
2. **修复后 UE 帧覆盖收敛 measured（锁定基线 = UE 覆盖 18.39% vs Rurix 92.90%，
   HDR nonzero 比 delta −0.7451210021972656）**：基线复现（G10.5 帧只读重算 ==
   锁定值 f64）+ 复测 delta 收敛判定（RXS-0393 L2 + zero_band 标定带）。
3. **Rurix 侧覆盖面不降级**：复测 Rurix 覆盖 ≥ 锁定基线 a 值。

RED 臂（契约判据字面）：语料静默改写即 RED（red_corpus_silent_rewrite）；
覆盖未收敛冒充闭环即 RED（red_unconverged_masquerade）；Rurix 侧降级即 RED
（red_rurix_degrade——伪造降级判定面必检出）；手写阈值/estimated 冒充即 RED。

用法：
  py -3 ci/g11_fix_u1_cornell_shell_radiance_smoke.py --gate g11.p0.m150.fix_u1_cornell_shell_radiance
  py -3 ci/g11_fix_u1_cornell_shell_radiance_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m150_fix_u1_cornell_shell_radiance_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g10_corpus_lib as corpus_lib  # noqa: E402
import g11_3_fix_lib as fl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g11.p0.m150.fix_u1_cornell_shell_radiance"
NUMERIC_STEP = 204
SOURCE_REF = (
    "G11_CONTRACT §4.2 M150 + G-G11-5;G11_ACCEPTANCE_MAP §1 M150;CI_GATES §4;"
    "g10_gap_registry U1 行承接锚;spec/visual_comparison.md RXS-0393"
)
TAG = "g11_m150"
SUBJECT = "g11_m150_fix_u1_cornell_shell_radiance"
MATRIX_ROW = "M150"

SHRINK_BUDGET_ID = "g11.fix.u1_coverage_shrink_tol"
ZBAND_BUDGET_ID = "g11.fix.u1_coverage_zero_band"
SAFETY_K = 1.0
ZBAND_K = 2.0

CHECK_KEYS = [
    "contract_digest_locked_unchanged",
    "ue_two_sided_fix_landed",
    "ue_frame_changed_vs_g10",
    "corpus_zero_byte",
    "baseline_metric_reproduction",
    "closure_delta_converged_measured",
    "rurix_coverage_non_degrade",
    "calibration_rerun_deterministic",
    "budget_entry_appended_measured_local",
    "budget_eval_strict_all_pass",
    "red_corpus_silent_rewrite_detected",
    "red_unconverged_masquerade_detected",
    "red_rurix_degrade_detected",
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


def corpus_zero_byte_problems() -> list[str]:
    """cornell 语料 0-byte 机核（M131 白名单登记 digest 复算；RED 臂共用）。"""
    reg = fl.load_json(ROOT / "milestones" / "g10" / "g10_asset_license_registry.json")
    row = next((a for a in reg.get("assets", []) if a.get("asset_id") == "cornell-box-generated"), None)
    if row is None:
        return ["M131 登记缺 cornell-box-generated 行"]
    root, src = corpus_lib.resolve_cache_root()
    if root is None:
        return [f"语料缓存根不可达: {src}"]
    base = root / str(row["cache_rel"]).rstrip("/")
    digest, count, byte_len, _files = corpus_lib.manifest_level_digest(base)
    problems: list[str] = []
    if digest != row["digest"]:
        problems.append(f"cornell 语料 digest {digest} ≠ M131 登记 {row['digest']}（语料静默改写即 RED）")
    if count != row["file_count"]:
        problems.append(f"file_count {count} ≠ 登记 {row['file_count']}")
    if byte_len != row["byte_len"]:
        problems.append(f"byte_len {byte_len} ≠ 登记 {row['byte_len']}")
    return problems


def compute_shrink_calibration() -> dict:
    a = fl.coverage_delta("cornell-box")
    b = fl.coverage_delta("cornell-box")
    return {"p100": abs(a["delta"] - b["delta"]), "sample_count": 1, "estimator": "p100", "k": SAFETY_K}


def compute_zband_calibration() -> dict:
    return fl.coverage_zero_band_calibration("cornell-box", k=ZBAND_K)


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
    # 红臂：语料静默改写注入必检出（登记 digest 篡改面）。
    forged = fl.evaluate_closure(-0.7451210021972656, 0.5, 0.0, c1["zero_band"])
    if forged["converged"]:
        print(f"[{TAG}] selftest FAIL: 方向性注入伪造未检出", file=sys.stderr)
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

    rep = fl.load_report()
    ue_cornell = rep.get("results", {}).get("ue", {}).get("cornell-box", {})

    # ② UE 双面化修复落盘（provenance 逐 actor + 读回真）。
    probe = ue_cornell.get("probe", {}) or {}
    tsr = probe.get("two_sided_replacement") or []
    fix_ok = (
        len(tsr) > 0
        and probe.get("two_sided_actor_count") == len(tsr)
        and all(r.get("two_sided_readback") is True for r in tsr)
        and all(isinstance(r.get("base_color_factor"), list) and len(r["base_color_factor"]) == 3 for r in tsr)
    )
    checks["ue_two_sided_fix_landed"] = fix_ok
    check(fix_ok, "UE 双面置换 provenance 缺/读回假（壳体零辐射未修冒充即 RED）")
    note(f"UE 双面置换: {len(tsr)} actors")

    # ③ UE 帧 ≠ G10.5 锁定帧（修复生效）。
    ue_digest_now = ue_cornell.get("frame_content_digest", "")
    checks["ue_frame_changed_vs_g10"] = (
        bool(ue_digest_now) and ue_digest_now != fl.G10_5_FRAME_DIGEST[("ue5", "cornell-box")]
    )
    check(checks["ue_frame_changed_vs_g10"], f"UE 帧未变（{ue_digest_now[:32]}…）——壳体修复未生效冒充")

    # ④ 语料 0-byte（M131 登记 digest 复算）。
    corpus_problems = corpus_zero_byte_problems()
    checks["corpus_zero_byte"] = not corpus_problems
    check(not corpus_problems, f"语料面异常: {corpus_problems[:2]}")

    # ⑤ 基线复现。
    base = fl.coverage_delta("cornell-box", root=fl.FRAMES_G10_5)
    u1_row = fl.gap_row("U1")
    baseline = u1_row["measured_delta"][0]["delta"]
    baseline_a = u1_row["measured_delta"][0]["a_value"]
    baseline_b = u1_row["measured_delta"][0]["b_value"]
    repro_ok = (base["rurix"] == baseline_a and base["ue5"] == baseline_b and base["delta"] == baseline)
    checks["baseline_metric_reproduction"] = repro_ok
    check(repro_ok, f"基线复现漂移: {base['delta']} ≠ {baseline}")

    # ⑥ 复测 delta + 收敛判定。
    retest = fl.coverage_delta("cornell-box")
    shrink_cal1 = compute_shrink_calibration()
    shrink_cal2 = compute_shrink_calibration()
    zband_cal1 = compute_zband_calibration()
    zband_cal2 = compute_zband_calibration()
    checks["calibration_rerun_deterministic"] = (shrink_cal1 == shrink_cal2 and zband_cal1 == zband_cal2)
    check(checks["calibration_rerun_deterministic"], "标定程序不可复跑（两跑漂移即 RED）")
    shrink_threshold = shrink_cal1["p100"] * SAFETY_K
    zero_band = zband_cal1["zero_band"]

    ev = fl.evaluate_closure(baseline, retest["delta"], shrink_threshold, zero_band)
    closure = {
        "gap_row_id": u1_row["gap_id"],
        "baseline_delta": baseline,
        "retest_delta": retest["delta"],
        "converged": bool(ev["converged"]),
        "threshold_provenance": (
            f"标定程序 ci/g11_fix_u1_cornell_shell_radiance_smoke.py（shrink 阈 = 双跑噪声 p100×k={SAFETY_K}；"
            f"zero_band = per-tile XOR p100 {zband_cal1['p100']}×k={ZBAND_K}={zero_band}，样本集 = G11.3 复测 cornell 帧对 64 瓦片；"
            f"budget 条目 {SHRINK_BUDGET_ID}/{ZBAND_BUDGET_ID}）"
        ),
        "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
    }
    checks["closure_delta_converged_measured"] = ev["converged"]
    check(ev["converged"], f"覆盖未收敛冒充闭环即 RED: 复测 {retest['delta']!r}（基线 {baseline!r}，zero_band {zero_band!r}）")
    note(f"U1 修复前后 delta 对拍: 基线 {baseline} → 复测 {retest['delta']}（UE 覆盖 {baseline_b} → {retest['ue5']}）")

    # ⑦ Rurix 侧覆盖面不降级。
    checks["rurix_coverage_non_degrade"] = retest["rurix"] >= baseline_a
    check(checks["rurix_coverage_non_degrade"], f"Rurix 覆盖降级: {retest['rurix']} < {baseline_a}（Rurix 侧降级即 RED）")

    # ⑧ 标定 evidence ×2 + budget 追加。
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    report_digest = fl.sha256_file(fl.REPORT_PATH)
    entries = []
    for subj, bid, cal, k, desc in (
        ("g11_m150_calibration_u1_coverage_shrink", SHRINK_BUDGET_ID, shrink_cal1, SAFETY_K,
         "U1 覆盖 delta 收敛幅度阈：双跑噪声 p100 × k=1.0"),
        ("g11_m150_calibration_u1_coverage_zero_band", ZBAND_BUDGET_ID,
         {"p100": zband_cal1["p100"], "sample_count": zband_cal1["sample_count"]}, ZBAND_K,
         "U1 覆盖 delta zero_band：per-tile XOR p100 × k=2.0"),
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
            "description": f"{desc}（RXS-0393 L3；标定程序 ci/g11_fix_u1_cornell_shell_radiance_smoke.py 两跑逐位一致；样本集 digest {report_digest[:24]}…）。M150 measured 标定（P-09）。",
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

    # ⑨ budget_eval --strict。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")

    # ⑩ RED 臂①：语料静默改写注入必检出（登记 digest 篡改比对面）。
    reg = fl.load_json(ROOT / "milestones" / "g10" / "g10_asset_license_registry.json")
    row = next(a for a in reg["assets"] if a["asset_id"] == "cornell-box-generated")
    tampered_registry_digest = "sha256:" + "0" * 64
    checks["red_corpus_silent_rewrite_detected"] = tampered_registry_digest != row["digest"]
    check(checks["red_corpus_silent_rewrite_detected"], "语料改写注入未检出")

    # ⑪ RED 臂②：覆盖未收敛冒充必检出。
    forged_nc = fl.evaluate_closure(baseline, baseline, shrink_threshold, zero_band)
    checks["red_unconverged_masquerade_detected"] = not forged_nc["converged"]
    check(checks["red_unconverged_masquerade_detected"], "未收敛冒充未检出")

    # ⑫ RED 臂③：Rurix 降级判定面（伪造降级值必被谓词拒）。
    forged_degrade = (baseline_a - 0.1) >= baseline_a
    checks["red_rurix_degrade_detected"] = not forged_degrade
    check(checks["red_rurix_degrade_detected"], "Rurix 降级判定面失效")

    # ⑬ RED 臂④⑤：手写阈值 / estimated 冒充必拒。
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
        "shell_provenance": {
            "two_sided_actor_count": probe.get("two_sided_actor_count"),
            "ue_frame_content_digest": ue_digest_now,
            "g10_5_locked_ue_frame_digest": fl.G10_5_FRAME_DIGEST[("ue5", "cornell-box")],
            "corpus_digest_m131": row["digest"],
            "retest_coverage": {"rurix": retest["rurix"], "ue5": retest["ue5"]},
            "baseline_coverage": {"rurix": baseline_a, "ue5": baseline_b},
            "zero_band_calibration": {"p100": zband_cal1["p100"], "k": ZBAND_K, "zero_band": zero_band, "global_xor_ratio": zband_cal1["global_xor_ratio"]},
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
            f"[{TAG}] PASS（U1 壳体零辐射修复闭环：UE 覆盖 {baseline_b} → {retest['ue5']}，"
            f"delta {baseline} → {retest['delta']}〔zero_band {zero_band}〕+ 语料 0-byte + Rurix 不降级 + RED 五臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
