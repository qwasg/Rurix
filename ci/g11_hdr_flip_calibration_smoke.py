#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.2 波）
"""G11.2 M157 HDR-FLIP 独立标定门（P1，步骤 199；
g11.p1.m157.hdr_flip_calibration；G11_ACCEPTANCE_MAP §2 M157 行判据 /
G-G11-4；CI_GATES §4A；G10-N10 承接锚兑现——G10.4b HDR-FLIP 仅探针臂遗留面；
spec/visual_comparison.md RXS-0389 L2/L5 + RXS-0393；RFC-0026 §4.2 F10 估计器语义）。

host 纯 host 门（device_section_state=not_applicable）。判据（MAP §2 M157 行字面）：

1. **HDR 域正式对拍样本集（真实 HDR 帧双臂）**：G11.2 复跑真实 HDR 帧
   （cornell-box + bistro-interior 双场景，UE5 臂为参考端 / Rurix 臂为测试端）
   确定性切分瓦片图对（4×4 网格 × 双场景 = 32 对 ≥ 下界 24——RXS-0389 图集
   下界口径继承）；样本清单 + 每对 digest + 样本集 manifest digest 入
   evidence；低于下界冒充有效标定即 RED。
2. **标定程序可复跑（两跑逐位一致）**：同一帧集/同一切分上 p100 估计器两跑
   逐位一致（自实现 flip_hdr vs 参考实现 flip_evaluator〔NVlabs/flip 1.7
   python-nanobind，M135 pin 五元组同源〕逐对 |标量差| 与误差图逐像素差
   分列——RXS-0389 L5 两面分列口径）。
3. **标定值按 M138 同程序（p100×k measured）入 g11_budget.json**
   （measured_local，字节级纯追加 + provenance 齐备，P-09 禁手写阈值）：
   `g11.metric.hdr_flip_pairwise_scalar_tol` /
   `g11.metric.hdr_flip_pairwise_error_map_tol`（k=2.0，M138 同值先例）+
   budget_eval --strict 全 PASS。
4. **恒等图对极值断言**：HDR-FLIP 恒等图对标量恰为 0（RXS-0389 L2 极值口径）。

RED 臂（MAP §2 行字面）：手写阈值冒充标定即 RED；estimated 冒充 measured
即 RED；标定程序不可复跑即 RED；样本集低于下界冒充有效标定即 RED。

用法：
  py -3 ci/g11_hdr_flip_calibration_smoke.py --gate g11.p1.m157.hdr_flip_calibration
  py -3 ci/g11_hdr_flip_calibration_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m157_hdr_flip_calibration_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g11_2_caliber_lib as cl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402
from g10_flip_lib import default_ppd, flip_hdr  # noqa: E402

GATE_KEY = "g11.p1.m157.hdr_flip_calibration"
NUMERIC_STEP = 199
SOURCE_REF = (
    "G11_ACCEPTANCE_MAP §2 M157;G11_CONTRACT G-G11-4;CI_GATES §4A;"
    "G10-N10 承接锚;spec/visual_comparison.md RXS-0389/RXS-0393;RFC-0026 §4.2 F10"
)
TAG = "g11_m157"
SUBJECT = "g11_m157_hdr_flip_calibration"
MATRIX_ROW = "M157"

SAFETY_K = 2.0  # k∈[1.0,3.0]；实现差噪声底上方双倍余量（M138 同值先例）
SAMPLE_GRID = 4  # 4×4 瓦片 × 双场景 = 32 对
SAMPLE_LOWER_BOUND = 24  # RXS-0389 图集下界口径继承（≥24 图对）

ENTRY_SCALAR = "g11.metric.hdr_flip_pairwise_scalar_tol"
ENTRY_ERROR_MAP = "g11.metric.hdr_flip_pairwise_error_map_tol"

CHECK_KEYS = [
    "hdr_sample_set_real_frames",
    "estimator_semantics_frozen",
    "calibration_rerun_deterministic",
    "hdr_flip_identity_zero",
    "budget_entries_appended_measured_local",
    "budget_eval_strict_all_pass",
    "red_handwritten_threshold_detected",
    "red_estimated_masquerade_detected",
    "red_nonrerunnable_detected",
    "red_below_bound_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def build_sample_set() -> dict:
    """真实 HDR 帧双臂样本集（确定性切分）：双场景 × 4×4 瓦片 = 32 对。
    参考端 = UE5（外部参照端），测试端 = Rurix。manifest + 逐对 digest。"""
    pairs: list[dict] = []
    for scene_id in cl.SCENES:
        ref = cl.pixels_of(cl.decode(cl.hdr_frame(scene_id, "ue5"), "ue5"))
        tst = cl.pixels_of(cl.decode(cl.hdr_frame(scene_id, "rurix"), "rurix"))
        h, w, _ = ref.shape
        for ty in range(SAMPLE_GRID):
            for tx in range(SAMPLE_GRID):
                y0, y1 = ty * h // SAMPLE_GRID, (ty + 1) * h // SAMPLE_GRID
                x0, x1 = tx * w // SAMPLE_GRID, (tx + 1) * w // SAMPLE_GRID
                a = ref[y0:y1, x0:x1].copy()
                b = tst[y0:y1, x0:x1].copy()
                ad = hashlib.sha256(a.astype(np.float64).tobytes()).hexdigest()
                bd = hashlib.sha256(b.astype(np.float64).tobytes()).hexdigest()
                pairs.append({
                    "pair_id": f"{scene_id}:tile({tx},{ty})",
                    "scene_id": scene_id,
                    "tile": [tx, ty, x0, y0, x1 - x0, y1 - y0],
                    "reference_digest": "sha256:" + ad,
                    "test_digest": "sha256:" + bd,
                    "a": a, "b": b,
                })
    manifest_src = json.dumps(
        [{k: p[k] for k in ("pair_id", "scene_id", "tile", "reference_digest", "test_digest")} for p in pairs],
        ensure_ascii=False, sort_keys=True,
    )
    manifest = {
        "pair_count": len(pairs),
        "grid": SAMPLE_GRID,
        "scenes": sorted(cl.SCENES),
        "manifest_digest": "sha256:" + hashlib.sha256(manifest_src.encode("utf-8")).hexdigest(),
        "frames_root": str(cl.FRAMES_G11),
    }
    return {"pairs": pairs, "manifest": manifest}


def compute_calibration(sample: dict) -> dict:
    """HDR-FLIP 标定估计器（可复跑）：逐对 |自实现 − 参考实现|（标量差与误差图
    逐像素差分列）的样本最大值 p100。auto-from-reference 曝光（RXS-0389 L2）；
    参考瓦片全黑（y_max=0，cornell UE 帧 18.39% 覆盖面黑区）时双端同退
    fixed(0,0,2) 曝光——退化输入双端同参数面，确定性可复跑。"""
    from flip_evaluator import evaluate

    ppd = default_ppd()
    scalar_diffs: list[float] = []
    map_diffs: list[float] = []
    degenerate = 0
    for p in sample["pairs"]:
        a = p["a"].astype(np.float64)
        b = p["b"].astype(np.float64)
        lum = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
        if float(lum.max()) <= 0.0:
            degenerate += 1
            em_self, m_self, _used = flip_hdr(
                a, b, ppd, hdr_exposure_mode="fixed",
                hdr_exposure_start=0.0, hdr_exposure_stop=0.0, hdr_num_exposures=2,
            )
            em_ref, m_ref, _prm = evaluate(
                a.astype(np.float32), b.astype(np.float32),
                "HDR", inputsRGB=False, applyMagma=False, computeMeanError=True,
                parameters={"startExposure": 0.0, "stopExposure": 0.0, "numExposures": 2},
            )
        else:
            em_self, m_self, _used = flip_hdr(a, b, ppd)
            em_ref, m_ref, _prm = evaluate(
                a.astype(np.float32), b.astype(np.float32),
                "HDR", inputsRGB=False, applyMagma=False, computeMeanError=True, parameters={},
            )
        scalar_diffs.append(abs(m_self - float(m_ref)))
        map_diffs.append(float(np.abs(em_self - em_ref[..., 0]).max()))
    return {
        "flip_scalar": max(scalar_diffs),
        "flip_error_map": max(map_diffs),
        "degenerate_tiles": degenerate,
        "sample_set_digest": sample["manifest"]["manifest_digest"],
        "sample_pair_count": sample["manifest"]["pair_count"],
        "estimator": "p100",
    }


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    sample = build_sample_set()
    # 绿臂①：样本集下界 + manifest digest。
    if sample["manifest"]["pair_count"] < SAMPLE_LOWER_BOUND:
        print(f"[{TAG}] selftest FAIL: 样本集低于下界误判", file=sys.stderr)
        return 1
    # 绿臂②：恒等图对 HDR-FLIP == 0（取非退化瓦片——全黑瓦片 auto 曝光无定）。
    a0 = _first_nondegenerate_tile(sample)
    _em, m_id, _u = flip_hdr(a0, a0, default_ppd())
    if m_id != 0.0:
        print(f"[{TAG}] selftest FAIL: 恒等图对非零 {m_id}", file=sys.stderr)
        return 1
    # 绿臂③：标定两跑逐位一致。
    c1 = compute_calibration(sample)
    c2 = compute_calibration(sample)
    if c1 != c2:
        print(f"[{TAG}] selftest FAIL: 标定两跑不一致", file=sys.stderr)
        return 1
    # 红臂①：手写阈值冒充必拒。
    ok_entry = {
        "id": "g11.metric.selftest_probe",
        "evidence": "measured_local",
        "threshold": c1["flip_scalar"] * SAFETY_K,
        "measured_value": c1["flip_scalar"],
        "evidence_file": "milestones/g11/g11_budget.json",
    }
    if cl.validate_budget_entry(ok_entry, c1["flip_scalar"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: 合法条目误判", file=sys.stderr)
        return 1
    if not cl.validate_budget_entry(dict(ok_entry, threshold=c1["flip_scalar"] * SAFETY_K * 1.5), c1["flip_scalar"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: 手写阈值冒充未检出", file=sys.stderr)
        return 1
    # 红臂②：estimated 冒充必拒。
    if not cl.validate_budget_entry(dict(ok_entry, evidence="estimated"), c1["flip_scalar"], SAFETY_K):
        print(f"[{TAG}] selftest FAIL: estimated 冒充未检出", file=sys.stderr)
        return 1
    # 红臂③：低于下界样本集冒充必检出。
    small = {"manifest": {"pair_count": SAMPLE_LOWER_BOUND - 1}}
    if not validate_lower_bound(small):
        print(f"[{TAG}] selftest FAIL: 低于下界冒充未检出", file=sys.stderr)
        return 1
    schema = cl.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (3 RED + 3 GREEN)")
    return 0


def _first_nondegenerate_tile(sample: dict) -> np.ndarray:
    """首个亮度最大值 > 0 的参考瓦片（cornell UE 帧 18.39% 覆盖面，全黑瓦片
    auto-from-reference 曝光无定——恒等断言取有内容瓦片）。"""
    for p in sample["pairs"]:
        a = p["a"]
        lum = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
        if float(lum.max()) > 0.0:
            return a.astype(np.float64)
    raise RuntimeError("样本集无非退化瓦片（全黑——样本集判别力不足即 RED 面）")


def validate_lower_bound(sample: dict) -> list[str]:
    problems: list[str] = []
    if sample.get("manifest", {}).get("pair_count", 0) < SAMPLE_LOWER_BOUND:
        problems.append(
            f"样本集 {sample.get('manifest', {}).get('pair_count')} < 下界 {SAMPLE_LOWER_BOUND}（低于下界冒充有效标定即 RED）"
        )
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

    # ① 真实 HDR 帧双臂样本集（下界 + digest 入 evidence）。
    sample = build_sample_set()
    lb_problems = validate_lower_bound(sample)
    checks["hdr_sample_set_real_frames"] = not lb_problems
    check(not lb_problems, f"样本集异常: {lb_problems}")
    note(f"样本集: {sample['manifest']['pair_count']} 对（双场景 4×4 瓦片，真实 HDR 帧双臂）digest={sample['manifest']['manifest_digest'][:24]}…")

    # ② 标定两跑（可复跑判据）。
    cal1 = compute_calibration(sample)
    cal2 = compute_calibration(sample)
    checks["calibration_rerun_deterministic"] = cal1 == cal2
    check(cal1 == cal2, "标定程序不可复跑（两跑漂移即 RED）")
    note(f"HDR-FLIP 标定两跑逐位一致: scalar={cal1['flip_scalar']:.6e} error_map={cal1['flip_error_map']:.6e}")

    # ③ 估计器语义冻结（p100 × k，k∈[1,3]，样本集 digest 引用，下界）。
    checks["estimator_semantics_frozen"] = (
        cal1["estimator"] == "p100"
        and cal1["sample_set_digest"].startswith("sha256:")
        and cal1["sample_pair_count"] >= SAMPLE_LOWER_BOUND
        and 1.0 <= SAFETY_K <= 3.0
    )
    check(checks["estimator_semantics_frozen"], "估计器语义漂移（p100×k / k 边界 / 样本集 digest / 下界）")

    # ④ 恒等图对极值断言（HDR-FLIP == 0；取非退化瓦片）。
    a0 = _first_nondegenerate_tile(sample)
    _em_id, m_id, _u = flip_hdr(a0, a0, default_ppd())
    checks["hdr_flip_identity_zero"] = m_id == 0.0
    check(m_id == 0.0, f"恒等图对 HDR-FLIP 非零: {m_id}")

    # ⑤ 标定 evidence 两件落盘 + 标定值入 g11_budget（字节级纯追加）。
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    calib_specs = [
        ("hdr_flip_scalar", ENTRY_SCALAR, cal1["flip_scalar"], "HDR-FLIP 自实现 vs 参考实现逐对标量差 p100 × k=2.0（RXS-0389 L5 标量对拍容差面）"),
        ("hdr_flip_error_map", ENTRY_ERROR_MAP, cal1["flip_error_map"], "HDR-FLIP 自实现 vs 参考实现误差图逐像素差 p100 × k=2.0（RXS-0389 L5 误差图对拍容差面）"),
    ]
    new_entries: list[dict] = []
    budget_problems: list[str] = []
    for slug, eid, p100, desc in calib_specs:
        calib_ev = {
            "schema_version": 1,
            "subject": f"g11_m157_calibration_{slug}",
            "symbolic_gate_key": GATE_KEY,
            "milestone": MATRIX_ROW,
            "wave": "G11.2",
            "numeric_step": NUMERIC_STEP,
            "results": {
                "trimmed_mean": p100,
                "estimator": "p100",
                "sample_pair_count": cal1["sample_pair_count"],
                "safety_factor_k": SAFETY_K,
                "threshold": p100 * SAFETY_K,
            },
            "provenance": {
                "estimator_semantics": "p100 × k（RFC-0026 §4.2 F10；M138 同程序纪律）",
                "k_rationale": "实现差噪声底上方双倍余量；k∈[1,3] 闭集内（M138 同值先例）",
                "sample_set_digest": cal1["sample_set_digest"],
                "sample_set": "G11.2 复跑真实 HDR 帧双臂（cornell-box + bistro-interior × UE5/Rurix 臂）4×4 瓦片 32 对 ≥ 下界 24",
                "reference_impl": "NVlabs/flip 1.7 python-nanobind（M135 pin 五元组同源登记）",
                "measured": "measured_local：真实 HDR 帧样本集逐对差 p100 × k 复跑两跑逐位一致；禁手写阈值冒充标定（P-09）",
            },
            "environment": wel.collect_environment(),
            "timestamp": ts,
        }
        calib_path = EVIDENCE_DIR / f"g11_m157_calibration_{slug}_{ts}.json"
        calib_path.write_text(json.dumps(calib_ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        entry = {
            "id": eid,
            "description": (
                f"{desc}。标定程序 ci/g11_hdr_flip_calibration_smoke.py 可复跑（两跑逐位一致）；"
                f"样本集 = 真实 HDR 帧双臂 4×4 瓦片 32 对 digest {cal1['sample_set_digest'][:24]}…；"
                "M157 measured 标定（P-09 禁手写阈值；G10-N10 承接锚兑现——HDR 域正式对拍样本集 + 标定值入 budget）。"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": p100 * SAFETY_K,
            "evidence_file": str(calib_path.relative_to(ROOT)).replace("\\", "/"),
            "measured_value": p100,
        }
        budget_problems.extend(cl.validate_budget_entry(entry, p100, SAFETY_K))
        new_entries.append(entry)
    if not budget_problems:
        budget_problems = cl.append_budget_entries(new_entries)
        if not budget_problems:
            note(f"g11_budget.json 字节级纯追加 2 条（{ENTRY_SCALAR} / {ENTRY_ERROR_MAP}）")
    checks["budget_entries_appended_measured_local"] = not budget_problems
    check(not budget_problems, f"budget 条目异常: {budget_problems[:2]}")

    # ⑥ budget_eval --strict 全 PASS。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")
    note(f"budget_eval --strict: exit {r.returncode}（{tail[-100:]}）")

    # ⑦ RED 臂①②：手写阈值 / estimated 冒充必拒。
    forged = {
        "id": "g11.metric.red_probe",
        "evidence": "measured_local",
        "threshold": cal1["flip_scalar"] * SAFETY_K * 1.5,
        "measured_value": cal1["flip_scalar"],
        "evidence_file": str((EVIDENCE_DIR / f"g11_m157_calibration_hdr_flip_scalar_{ts}.json").relative_to(ROOT)).replace("\\", "/"),
    }
    checks["red_handwritten_threshold_detected"] = bool(cl.validate_budget_entry(forged, cal1["flip_scalar"], SAFETY_K))
    check(checks["red_handwritten_threshold_detected"], "手写阈值冒充未检出")
    forged2 = dict(forged, threshold=cal1["flip_scalar"] * SAFETY_K, evidence="estimated")
    checks["red_estimated_masquerade_detected"] = bool(cl.validate_budget_entry(forged2, cal1["flip_scalar"], SAFETY_K))
    check(checks["red_estimated_masquerade_detected"], "estimated 冒充未检出")

    # ⑧ RED 臂③：不可复跑注入必检出（两跑漂移 → deterministic 判据翻红）。
    drift = dict(cal2)
    drift["flip_scalar"] = drift["flip_scalar"] + 1e-12
    checks["red_nonrerunnable_detected"] = (cal1 != drift) and (cal1 == cal2)
    check(checks["red_nonrerunnable_detected"], "复跑漂移注入未检出")

    # ⑨ RED 臂④：低于下界样本集冒充必检出。
    small = {"manifest": {"pair_count": SAMPLE_LOWER_BOUND - 1}}
    checks["red_below_bound_detected"] = bool(validate_lower_bound(small))
    check(checks["red_below_bound_detected"], "低于下界冒充未检出")

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
        "calibration_report": {
            "estimator": "p100",
            "estimator_semantics": "统计量 = 全样本集逐对 |自实现 − 参考实现| 差样本最大值（p100）；容差 = p100 × k（RFC-0026 §4.2 F10；标量差与误差图逐像素差分列 RXS-0389 L5）",
            "sample_set_digest": cal1["sample_set_digest"],
            "sample_pair_count": cal1["sample_pair_count"],
            "sample_grid": SAMPLE_GRID,
            "sample_lower_bound": SAMPLE_LOWER_BOUND,
            "sample_manifest": sample["manifest"],
            "rerun_deterministic": cal1 == cal2,
            "entries": [
                {"budget_entry_id": ENTRY_SCALAR, "calibrated_p100": cal1["flip_scalar"], "safety_factor_k": SAFETY_K, "threshold": cal1["flip_scalar"] * SAFETY_K},
                {"budget_entry_id": ENTRY_ERROR_MAP, "calibrated_p100": cal1["flip_error_map"], "safety_factor_k": SAFETY_K, "threshold": cal1["flip_error_map"] * SAFETY_K},
            ],
            "budget_path": "milestones/g11/g11_budget.json",
            "budget_append_only": True,
            "budget_eval_strict_exit": r.returncode,
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
            f"[{TAG}] PASS（HDR-FLIP 独立标定：真实 HDR 帧 {cal1['sample_pair_count']} 对 + 两跑逐位一致 + "
            f"scalar={cal1['flip_scalar']:.6e} / error_map={cal1['flip_error_map']:.6e} × k={SAFETY_K} 入 g11_budget + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
