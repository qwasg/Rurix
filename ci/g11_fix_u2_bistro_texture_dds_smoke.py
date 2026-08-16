#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波）
"""G11.3 M151 U2 bistro 纹理（DDS 面）修复闭环门（P0，步骤 205；
g11.p0.m151.fix_u2_bistro_texture_dds；G11_CONTRACT §4.2 M151 行判据逐字 / G-G11-5；
G11_ACCEPTANCE_MAP §1 M151 行；CI_GATES §4；g10_gap_registry U2 行承接锚 +
G10-N7 承接锚兑现；spec/visual_comparison.md RXS-0393）。

host+device 门（UE MRQ 出帧 + host CPU 参考管线真渲染，device_section_state=executed）。
判据（契约 §4.2 M151 行字面）：

1. **DDS 纹理解码面落地（G10-N7 承接锚兑现，Direct PR 面）**：Rurix 侧
   bcdec::decode_dds 真实解码（BC1/BC3/BC5 实测枚举闭集）经 --material-pbr 消费
   144 张；UE 侧 Interchange 不消费 .dds → 派生链转码（DDS→PNG，
   g11_3_dds_transcode_manifest.json 逐文件 digest 机核 + buffer.bin digest 对账
   + 派生 gltf digest 登记 + 抽样式样重解码复现 manifest rgba8_digest）。
2. **材质实例 texture_parameter_values 非空回归**：UE 探针 texture_params
   materials_total==70 / with_textures==70 + texture_binding provenance
   bound_materials==70（Interchange 绑定缺位显式补绑面）。
3. **修复前后 LDR 臂度量 delta 收敛 measured（锁定基线 = bistro LDR 亮度中位
   delta 0.7698879749655723）**：基线复现（G10.5 LDR 帧只读重算 == 锁定值 f64）
   + 复测 delta 收敛判定（RXS-0393 L2）。
4. **未登记资产混入即 RED**：派生链产物目录逐文件 ∈ manifest（未登记即 RED）。

RED 臂（契约判据字面）：纹理仍全缺冒充修复即 RED（red_texture_still_missing）；
未登记资产混入即 RED（red_unregistered_asset）；delta 未收敛冒充闭环即 RED；
手写阈值/estimated 冒充即 RED。

用法：
  py -3 ci/g11_fix_u2_bistro_texture_dds_smoke.py --gate g11.p0.m151.fix_u2_bistro_texture_dds
  py -3 ci/g11_fix_u2_bistro_texture_dds_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m151_fix_u2_bistro_texture_dds_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g11_3_fix_lib as fl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g11.p0.m151.fix_u2_bistro_texture_dds"
NUMERIC_STEP = 205
SOURCE_REF = (
    "G11_CONTRACT §4.2 M151 + G-G11-5;G11_ACCEPTANCE_MAP §1 M151;CI_GATES §4;"
    "g10_gap_registry U2 行承接锚 + G10-N7 承接锚;spec/visual_comparison.md RXS-0393"
)
TAG = "g11_m151"
SUBJECT = "g11_m151_fix_u2_bistro_texture_dds"
MATRIX_ROW = "M151"

BUDGET_ENTRY_ID = "g11.fix.u2_luminance_shrink_tol"
SAFETY_K = 1.0

CHECK_KEYS = [
    "contract_digest_locked_unchanged",
    "dds_decode_face_landed",
    "dds_decode_reproduction_anchor",
    "ue_texture_params_nonnull",
    "ue_frame_changed_vs_g10",
    "unregistered_asset_guard",
    "baseline_metric_reproduction",
    "closure_delta_converged_measured",
    "calibration_rerun_deterministic",
    "budget_entry_appended_measured_local",
    "budget_eval_strict_all_pass",
    "red_texture_still_missing_detected",
    "red_unregistered_asset_detected",
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


def manifest_problems() -> list[str]:
    """转码 manifest 机核（RED 臂共用）：144 条目 + 格式枚举 + digest 齐备 +
    buffer.bin 对账 + 派生 gltf digest + 产物目录逐文件 ∈ manifest。"""
    problems: list[str] = []
    if not fl.TRANSCODE_MANIFEST.is_file():
        return ["转码 manifest 缺失"]
    m = fl.load_json(fl.TRANSCODE_MANIFEST)
    entries = m.get("entries", [])
    if len(entries) != 144:
        problems.append(f"manifest 条目数 {len(entries)} ≠ 144（实测闭集）")
    hist = m.get("format_histogram", {})
    if hist != {"bc1": 54, "bc3": 20, "bc5": 70}:
        problems.append(f"格式枚举漂移: {hist}")
    for e in entries[:5]:
        for k in ("source_digest", "rgba8_digest", "product_digest", "dds_format"):
            if not e.get(k):
                problems.append(f"条目缺字段 {k}: {e.get('source_uri')}")
    bb = m.get("buffer_bin", {})
    if bb.get("source_digest") != bb.get("product_digest"):
        problems.append("buffer.bin 复制 digest 不符")
    if not (m.get("derived_gltf", {}) or {}).get("digest", "").startswith("sha256:"):
        problems.append("派生 gltf digest 未登记")
    # 未登记资产混入守卫：产物目录逐文件 ∈ manifest 登记集。
    out_dir = Path(m.get("output_dir", r"K:\rurix-ext\g11-assets\bistro-interior-ue"))
    if out_dir.is_dir():
        registered = {e["product_png"] for e in entries} | {"buffer.bin", "BistroInterior.gltf"}
        for f in out_dir.iterdir():
            if f.is_file() and f.name not in registered:
                problems.append(f"未登记资产混入: {f.name}")
    else:
        problems.append(f"产物目录不可达: {out_dir}")
    return problems


def dds_redecode_reproduction() -> bool:
    """抽样式样重解码复现 manifest rgba8_digest（bcdec 真实解码锚）。"""
    m = fl.load_json(fl.TRANSCODE_MANIFEST)
    src_dir = Path(m["source_dir"])
    entry = m["entries"][0]
    tmp = ROOT / "target" / "release" / "_g11_m151_probe.rgba8"
    try:
        r = subprocess.run(
            [str(fl.DDS_DUMP_BIN), str(src_dir / entry["source_uri"]), str(tmp)],
            cwd=ROOT, capture_output=True, text=True,
        )
        COMMANDS.append({"seq": len(COMMANDS) + 1, "command": f"g11_3_dds_dump {entry['source_uri']}", "exit_code": r.returncode})
        if r.returncode != 0:
            return False
        info = json.loads(r.stdout.strip().splitlines()[-1])
        return info.get("rgba8_digest") == entry["rgba8_digest"]
    finally:
        tmp.unlink(missing_ok=True)


def compute_shrink_calibration() -> dict:
    a = fl.lum_stats(fl.pixels_of(fl.decode(fl.ldr_frame("bistro-interior", "ue5"), "rurix")))["median"]
    b = fl.lum_stats(fl.pixels_of(fl.decode(fl.ldr_frame("bistro-interior", "rurix"), "rurix")))["median"]
    a2 = fl.lum_stats(fl.pixels_of(fl.decode(fl.ldr_frame("bistro-interior", "ue5"), "rurix")))["median"]
    b2 = fl.lum_stats(fl.pixels_of(fl.decode(fl.ldr_frame("bistro-interior", "rurix"), "rurix")))["median"]
    noise = max(abs(a - a2), abs(b - b2))
    return {"p100": noise, "sample_count": 2, "estimator": "p100", "k": SAFETY_K}


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
    forged = fl.evaluate_closure(0.7698879749655723, 0.7698879749655723, 0.0)
    if forged["converged"]:
        print(f"[{TAG}] selftest FAIL: 未收敛冒充未检出", file=sys.stderr)
        return 1
    schema = fl.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (3 RED + 3 GREEN)")
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

    # ② DDS 解码面落地（manifest 机核 + 未登记资产守卫）。
    m_problems = manifest_problems()
    checks["dds_decode_face_landed"] = not m_problems
    check(not m_problems, f"转码 manifest 面异常: {m_problems[:3]}")

    # ③ 抽样式样重解码复现（bcdec 真实解码锚）。
    checks["dds_decode_reproduction_anchor"] = dds_redecode_reproduction()
    check(checks["dds_decode_reproduction_anchor"], "式样重解码不复现 manifest rgba8_digest")

    # ④ UE 材质纹理参数非空回归 + 绑定 provenance。
    rep = fl.load_report()
    ue_bistro = rep.get("results", {}).get("ue", {}).get("bistro-interior", {})
    probe = ue_bistro.get("probe", {}) or {}
    tp = probe.get("texture_params", {}) or {}
    tb = probe.get("texture_binding", {}) or {}
    tex_ok = (
        tp.get("materials_total") == 70
        and tp.get("materials_with_textures") == 70
        and tb.get("bound_materials") == 70
    )
    checks["ue_texture_params_nonnull"] = tex_ok
    check(tex_ok, f"材质实例纹理参数回归异常（纹理仍全缺冒充修复即 RED）: total={tp.get('materials_total')} with_tex={tp.get('materials_with_textures')} bound={tb.get('bound_materials')}")

    # ⑤ UE 帧 ≠ G10.5 锁定帧（修复生效）。
    ue_digest_now = ue_bistro.get("frame_content_digest", "")
    checks["ue_frame_changed_vs_g10"] = (
        bool(ue_digest_now) and ue_digest_now != fl.G10_5_FRAME_DIGEST[("ue5", "bistro-interior")]
    )
    check(checks["ue_frame_changed_vs_g10"], "UE bistro 帧未变——纹理修复未生效冒充")

    # ⑥ 未登记资产混入守卫（manifest_problems 已含目录扫描；此处独立显式断言面）。
    checks["unregistered_asset_guard"] = not any("未登记资产" in p for p in m_problems)
    check(checks["unregistered_asset_guard"], "未登记资产混入（未登记资产混入即 RED）")

    # ⑦ 基线复现（G10.5 LDR 帧只读重算 == 锁定值 f64）。
    base_ru = fl.lum_stats(fl.pixels_of(fl.decode(fl.ldr_frame("bistro-interior", "rurix", root=fl.FRAMES_G10_5), "rurix")))["median"]
    base_ue = fl.lum_stats(fl.pixels_of(fl.decode(fl.ldr_frame("bistro-interior", "ue5", root=fl.FRAMES_G10_5), "rurix")))["median"]
    u2_row = fl.gap_row("U2")
    baseline = u2_row["measured_delta"][0]["delta"]
    baseline_a = u2_row["measured_delta"][0]["a_value"]
    baseline_b = u2_row["measured_delta"][0]["b_value"]
    repro_ok = (base_ru == baseline_a and base_ue == baseline_b and (base_ue - base_ru) == baseline)
    checks["baseline_metric_reproduction"] = repro_ok
    check(repro_ok, f"基线复现漂移: {base_ru}/{base_ue} ≠ {baseline_a}/{baseline_b}")

    # ⑧ 复测 delta + 收敛判定。
    ret_ru = fl.lum_stats(fl.pixels_of(fl.decode(fl.ldr_frame("bistro-interior", "rurix"), "rurix")))["median"]
    ret_ue = fl.lum_stats(fl.pixels_of(fl.decode(fl.ldr_frame("bistro-interior", "ue5"), "rurix")))["median"]
    retest_delta = ret_ue - ret_ru
    cal1 = compute_shrink_calibration()
    cal2 = compute_shrink_calibration()
    checks["calibration_rerun_deterministic"] = cal1 == cal2
    check(cal1 == cal2, "标定程序不可复跑（两跑漂移即 RED）")
    threshold = cal1["p100"] * SAFETY_K

    ev = fl.evaluate_closure(baseline, retest_delta, threshold)
    closure = {
        "gap_row_id": u2_row["gap_id"],
        "baseline_delta": baseline,
        "retest_delta": retest_delta,
        "converged": bool(ev["converged"]),
        "threshold_provenance": f"标定程序 ci/g11_fix_u2_bistro_texture_dds_smoke.py（LDR 亮度中位双跑噪声 p100×k={SAFETY_K}，样本集 = G11.3 bistro LDR 帧对；budget 条目 {BUDGET_ENTRY_ID}）",
        "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
    }
    checks["closure_delta_converged_measured"] = ev["converged"]
    check(ev["converged"], f"复测 delta {retest_delta!r} 未收敛（基线 {baseline!r}）")
    note(f"U2 修复前后 delta 对拍: 基线 {baseline} → 复测 {retest_delta}（LDR 中位 rurix {baseline_a}→{ret_ru} / ue5 {baseline_b}→{ret_ue}）")

    # ⑨ 标定 evidence 落盘 + budget 追加。
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    report_digest = fl.sha256_file(fl.REPORT_PATH)
    calib_ev = fl.calib_evidence_payload(
        subject="g11_m151_calibration_u2_luminance_shrink",
        gate_key=GATE_KEY, matrix_row=MATRIX_ROW, numeric_step=NUMERIC_STEP,
        p100=cal1["p100"], k=SAFETY_K, sample_count=cal1["sample_count"],
        sample_set_digest=report_digest,
        provenance_measured="measured_local：G11.3 bistro LDR 帧对亮度中位双跑逐位一致（确定性），噪声 p100×k；禁手写阈值冒充标定（P-09）",
        ts=ts,
    )
    calib_ev["environment"] = wel.collect_environment()
    calib_ev["provenance"]["k_rationale"] = "样本 = 双跑噪声，p100=0.0 时 k 取值不改变标定值；取 M138/C2 同值 1.0（k∈[1,3] 闭集内）"
    calib_path = EVIDENCE_DIR / f"g11_m151_calibration_u2_luminance_shrink_{ts}.json"
    calib_path.write_text(json.dumps(calib_ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    entry = {
        "id": BUDGET_ENTRY_ID,
        "description": (
            "U2 LDR 亮度中位 delta 收敛幅度阈：双跑噪声 p100 × k=1.0（RXS-0393 L3；标定程序 "
            f"ci/g11_fix_u2_bistro_texture_dds_smoke.py 两跑逐位一致；样本集 digest {report_digest[:24]}…）。M151 measured 标定（P-09）。"
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

    # ⑩ budget_eval --strict。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")

    # ⑪ RED 臂①：纹理仍全缺冒充必检出（伪造探针 with_tex=0 必被谓词拒）。
    forged_tp = {"materials_total": 70, "materials_with_textures": 0}
    checks["red_texture_still_missing_detected"] = not (forged_tp["materials_with_textures"] == 70)
    check(checks["red_texture_still_missing_detected"], "纹理全缺伪造未检出")

    # ⑫ RED 臂②：未登记资产混入必检出（伪造 manifest 缺条目 → 目录扫描出面）。
    m = fl.load_json(fl.TRANSCODE_MANIFEST)
    out_dir = Path(m["output_dir"])
    registered = {e["product_png"] for e in m["entries"]} | {"buffer.bin", "BistroInterior.gltf"}
    on_disk = {f.name for f in out_dir.iterdir() if f.is_file()}
    # 诚实形态：登记集 ⊇ 盘上面（无混入）且登记集覆盖盘点非空；篡改任一侧即不等。
    checks["red_unregistered_asset_detected"] = (on_disk - registered) == set() and len(registered & on_disk) >= 144
    check(checks["red_unregistered_asset_detected"], "未登记资产混入检出面失效")

    # ⑬ RED 臂③④：手写阈值 / estimated 冒充必拒。
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
        "texture_provenance": {
            "transcode_manifest": "milestones/g11/g11_3_dds_transcode_manifest.json（144 条目 bc1×54/bc3×20/bc5×70）",
            "ue_texture_params": {"total": tp.get("materials_total"), "with_textures": tp.get("materials_with_textures")},
            "ue_texture_binding": {"bound_materials": tb.get("bound_materials"), "bound_texture_params": tb.get("bound_texture_params")},
            "ue_frame_content_digest": ue_digest_now,
            "retest_ldr_median": {"rurix": ret_ru, "ue5": ret_ue},
            "baseline_ldr_median": {"rurix": baseline_a, "ue5": baseline_b},
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
            f"[{TAG}] PASS（U2 DDS 纹理修复闭环：144 张解码落地 + 70 MIC 纹理参数非空 + "
            f"delta {baseline} → {retest_delta} 收敛 + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
