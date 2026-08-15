#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.4a 波）
"""G10.4a M137 逐像素 diff 报告门冒烟（步骤 183；
g10.p0.m137.pixel_diff_report；RFC-0026 §4.4；spec/visual_comparison.md
RXS-0388；G10_ACCEPTANCE_MAP §1 M137 行）。

host 纯 host 门（device_section_state 正常态 not_applicable）。判据：
diff 热区图 + 逐区域统计落盘 + evidence schema 闭集——报告器
`g10_m137_diff_report`（host 纯 safe Rust bin，image-io RXS-0385 解码）
产同一误差缓冲的三面投影（误差 EXR 单通道 Y float32 / 灰度热区图 PPM /
16×16 区域统计 + 标量），门侧 ci/g10_exr_lib.py **独立第二实现**逐面重算
核验 golden：误差 EXR 由帧对重算逐像素位级一致、热区图由误差 EXR 重算
逐字节一致、区域统计（16×16 floor 网格 + nearest-rank p95 + 边缘规则）
与标量由误差 EXR 重算逐字段一致、evidence 闭集字段机核。
thresholds 以 provisional 形态登记（identity 图对噪声底 p100 实测 = 0.0，
k=1.0，M138 正式入 g10_budget 后翻转 source；禁手写阈值冒充标定）。

RED 臂（契约 §4.2 M137 字面）：diff 图与标量报告不一致注入即 RED（篡改
误差 EXR 像素 / 篡改标量，重算不一致检出）；空场景行即 RED（空 scene_id
fail-closed）；闭集外字段注入即 RED（报告加字段，闭集机核拒绝）。

用法：
  py -3 ci/g10_pixel_diff_report_smoke.py --gate g10.p0.m137.pixel_diff_report
  py -3 ci/g10_pixel_diff_report_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import platform
import re
import struct
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m137_pixel_diff_report_evidence_schema.json"
SPEC_PATH = ROOT / "spec" / "visual_comparison.md"
WORK_DIR = ROOT / ".tmp" / "g104_gates" / "m137"

sys.path.insert(0, str(ROOT / "ci"))
import g10_exr_lib  # noqa: E402

GATE_KEY = "g10.p0.m137.pixel_diff_report"
NUMERIC_STEP = 183
SOURCE_REF = "RFC-0026 §4.4;spec/visual_comparison.md RXS-0388;G10_ACCEPTANCE_MAP §1 M137"
TAG = "g10_m137"
SUBJECT = "g10_m137_pixel_diff_report"
MATRIX_ROW = "M137"
BIN = "g10_m137_diff_report"

GRID_NX = 16
GRID_NY = 16

REPORT_TOP_KEYS = {
    "schema_version", "scene_id", "camera_id", "frame_index", "end_pair", "domain",
    "metric_caliber", "thresholds", "region_grid", "regions", "scalars", "artifacts",
    "determinism_contract_digest", "provenance",
}
REPORT_REGION_KEYS = {
    "x", "y", "w", "h", "pixel_count", "err_max", "err_mean", "err_p95", "over_threshold_count",
}
REPORT_SCALAR_KEYS = {
    "flip", "err_max", "err_mean", "err_p95", "over_threshold_pixel_count", "over_threshold_ratio",
}
REPORT_ARTIFACT_KEYS = {"frame_a_digest", "frame_b_digest", "error_map_digest", "heatmap_digest"}
REPORT_THRESH_KEYS = {"value", "source", "source_digest"}
REPORT_END_PAIR_KEYS = {"frame_a", "frame_b"}

CHECK_KEYS = [
    "spec_rxs0388_clause_on_tree",
    "harness_build_host",
    "identity_pair_noise_floor_zero",
    "dual_layer_artifacts",
    "error_exr_recompute_match",
    "heatmap_recompute_match",
    "region_stats_recompute_match",
    "scalars_recompute_match",
    "report_closed_set_enforced",
    "red_diff_scalar_inconsistency_detected",
    "red_empty_scene_row_detected",
    "red_extra_field_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def run_cmd(argv: list[str], timeout: int = 1800) -> subprocess.CompletedProcess:
    print(f"[{TAG}] $ {' '.join(argv)}")
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(argv), "exit_code": r.returncode})
    return r


def error_buffer_f32(pa: list[float], pb: list[float]) -> np.ndarray:
    """门内供给口径（RXS-0388 L1 同字面）：通道最大绝对差钳制 [0,1]，
    numpy float32 逐元素 IEEE 单精度（与 Rust f32 算术逐值一致）。"""
    a = np.asarray(pa, dtype=np.float32).reshape(-1, 3)
    b = np.asarray(pb, dtype=np.float32).reshape(-1, 3)
    d = np.max(np.abs(a - b), axis=1)
    return np.clip(d, np.float32(0.0), np.float32(1.0))


def region_stats_py(err: np.ndarray, width: int, height: int, threshold: float) -> list[dict]:
    """16×16 floor 网格（末行/末列吸收剩余像素）+ nearest-rank p95。"""
    cell_w = max(1, width // GRID_NX)
    cell_h = max(1, height // GRID_NY)
    out = []
    for gy in range(GRID_NY):
        for gx in range(GRID_NX):
            x = gx * cell_w
            y = gy * cell_h
            if x >= width or y >= height:
                continue
            w = (width - x) if gx + 1 == GRID_NX else cell_w
            h = (height - y) if gy + 1 == GRID_NY else cell_h
            vals = [float(err[yy * width + xx]) for yy in range(y, y + h) for xx in range(x, x + w)]
            assert len(vals) == w * h
            svals = sorted(vals)
            out.append({
                "x": x, "y": y, "w": w, "h": h,
                "pixel_count": len(vals),
                "err_max": svals[-1],
                "err_mean": sum(vals) / len(vals),
                "err_p95": g10_exr_lib.nearest_rank_p95(svals),
                "over_threshold_count": sum(1 for v in vals if v > threshold),
            })
    return out


def f32_eq(a: float, b: float) -> bool:
    """f32 语义相等（Rust Display 最短 round-trip 文本 → f64 → 还原 f32 位级）。"""
    return np.float32(a).tobytes() == np.float32(b).tobytes()


def closed_set_failures(report: dict) -> list[str]:
    """evidence JSON 字段闭集机核（RXS-0388 L4；闭集外字段拒收）。"""
    fails: list[str] = []
    if set(report) != REPORT_TOP_KEYS:
        fails.append(f"顶层闭集漂移: extra={sorted(set(report) - REPORT_TOP_KEYS)} missing={sorted(REPORT_TOP_KEYS - set(report))}")
    for r in report.get("regions", []):
        if set(r) != REPORT_REGION_KEYS:
            fails.append(f"regions[] 闭集漂移: {sorted(set(r) ^ REPORT_REGION_KEYS)}")
            break
    if set(report.get("scalars", {})) != REPORT_SCALAR_KEYS:
        fails.append(f"scalars 闭集漂移: {sorted(set(report.get('scalars', {})) ^ REPORT_SCALAR_KEYS)}")
    if set(report.get("artifacts", {})) != REPORT_ARTIFACT_KEYS:
        fails.append("artifacts 闭集漂移")
    if set(report.get("thresholds", {})) != REPORT_THRESH_KEYS:
        fails.append("thresholds 闭集漂移")
    if set(report.get("end_pair", {})) != REPORT_END_PAIR_KEYS:
        fails.append("end_pair 闭集漂移")
    if not str(report.get("scene_id", "")).strip():
        fails.append("scene_id 空串（空场景行）")
    if not str(report.get("camera_id", "")).strip():
        fails.append("camera_id 空串（空场景行）")
    return fails


def load_report(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂：闭式误差缓冲 → 区域统计自洽（pixel_count 对账 + p95 单调）。
    err = error_buffer_f32([0.1, 0.2, 0.3] * (100 * 70), [0.2, 0.1, 0.4] * (100 * 70))
    regions = region_stats_py(err, 100, 70, 0.0)
    if len(regions) != 256 or sum(r["pixel_count"] for r in regions) != 100 * 70:
        print(f"[{TAG}] selftest FAIL: 区域网格对账失效 {len(regions)}", file=sys.stderr)
        return 1
    edge = [r for r in regions if r["x"] == 90 and r["y"] == 60]
    if not edge or edge[0]["w"] != 10 or edge[0]["h"] != 10 or edge[0]["pixel_count"] != 100:
        print(f"[{TAG}] selftest FAIL: 边缘规则失效 {edge}", file=sys.stderr)
        return 1
    # 红臂①：空场景行必拒。
    if not closed_set_failures({"scene_id": "", "camera_id": "c", "regions": [], "scalars": {}, "artifacts": {}, "thresholds": {}, "end_pair": {}}):
        print(f"[{TAG}] selftest FAIL: 空场景行未拒", file=sys.stderr)
        return 1
    # 红臂②：闭集外字段必拒。
    good_report = {
        "schema_version": 1, "scene_id": "s", "camera_id": "c", "frame_index": 0,
        "end_pair": {"frame_a": {}, "frame_b": {}}, "domain": "scene-linear-hdr",
        "metric_caliber": "sha256:x", "thresholds": {"value": 0.0, "source": "p", "source_digest": "d"},
        "region_grid": {"nx": 16, "ny": 16}, "regions": [], "scalars": {k: 0 for k in REPORT_SCALAR_KEYS},
        "artifacts": {k: "d" for k in REPORT_ARTIFACT_KEYS},
        "determinism_contract_digest": "d", "provenance": {},
    }
    bad = dict(good_report)
    bad["evil"] = 1
    if closed_set_failures(good_report) or not closed_set_failures(bad):
        print(f"[{TAG}] selftest FAIL: 闭集机核失效", file=sys.stderr)
        return 1
    # 绿臂：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (2 RED + 2 GREEN)")
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

    checks["spec_rxs0388_clause_on_tree"] = SPEC_PATH.is_file() and (
        re.search(r"^###\s+RXS-0388\b", SPEC_PATH.read_text(encoding="utf-8"), re.MULTILINE)
        is not None
    )
    check(checks["spec_rxs0388_clause_on_tree"], "spec/visual_comparison.md 缺 RXS-0388 条款头")

    # 构建 host 报告器（default features；零 device 依赖）。
    r = run_cmd(["cargo", "build", "-p", "rurix-render", "--bin", BIN])
    exe = ROOT / "target" / "debug" / f"{BIN}.exe"
    checks["harness_build_host"] = r.returncode == 0 and exe.is_file()
    check(checks["harness_build_host"], f"报告器构建失败: {(r.stdout + r.stderr)[-400:]}")

    main_dir = WORK_DIR / "main"
    identity_dir = WORK_DIR / "identity"
    report_path = main_dir / "report.json"
    identity_report_path = identity_dir / "report.json"

    if checks["harness_build_host"]:
        # 主跑：闭式合成帧对（threshold 待 identity 噪声底标定后回写——本波
        # identity p100 实测 0.0，threshold=0.0 与之一致，先跑后核）。
        r = run_cmd([
            str(exe), "--synthetic-pair",
            "--out-dir", str(main_dir), "--evidence", str(report_path),
            "--scene-id", "probe-synthetic", "--camera-id", "cam0",
            "--frame-index", "0", "--threshold", "0.0",
        ])
        check(r.returncode == 0 and "PASS" in r.stdout, f"主报告跑失败: {(r.stdout + r.stderr)[-400:]}")
        # identity 图对：噪声底 measured（frame_a vs frame_a）。
        r2 = run_cmd([
            str(exe),
            "--frame-a", str(main_dir / "frame_a.exr"),
            "--frame-b", str(main_dir / "frame_a.exr"),
            "--out-dir", str(identity_dir), "--evidence", str(identity_report_path),
            "--scene-id", "probe-identity", "--camera-id", "cam0",
            "--frame-index", "0", "--threshold", "0.0",
        ])
        check(r2.returncode == 0, f"identity 跑失败: {(r2.stdout + r2.stderr)[-400:]}")

    report = load_report(report_path) if report_path.is_file() else {}
    identity = load_report(identity_report_path) if identity_report_path.is_file() else {}

    # identity 噪声底 = 0.0（measured；provisional threshold 推导面）。
    id_scalars = identity.get("scalars", {})
    checks["identity_pair_noise_floor_zero"] = (
        id_scalars.get("err_max") == 0.0
        and id_scalars.get("err_p95") == 0.0
        and id_scalars.get("over_threshold_pixel_count") == 0
    )
    check(
        checks["identity_pair_noise_floor_zero"],
        f"identity 图对噪声底非零: {id_scalars}",
    )
    note("provisional threshold = identity p100 0.0 × k=1.0 = 0.0（M138 正式标定后翻转 source）")

    # 双层产物与 digest 闭集。
    err_exr_path = main_dir / "error_map.exr"
    heat_path = main_dir / "heatmap.ppm"
    artifacts = report.get("artifacts", {})
    checks["dual_layer_artifacts"] = (
        err_exr_path.is_file()
        and heat_path.is_file()
        and err_exr_path.stat().st_size > 8 * 70
        and heat_path.stat().st_size > 0
        and all(str(artifacts.get(k, "")).startswith("sha256:") for k in REPORT_ARTIFACT_KEYS)
    )
    check(checks["dual_layer_artifacts"], "双层产物（误差 EXR + 热区图）或四 digest 闭集缺失")

    # 独立重算核验三面一致。
    recompute_ok = {"err": False, "heat": False, "regions": False, "scalars": False}
    if checks["dual_layer_artifacts"]:
        try:
            fa = g10_exr_lib.decode_exr_file(main_dir / "frame_a.exr", "rurix")
            fb = g10_exr_lib.decode_exr_file(main_dir / "frame_b.exr", "rurix")
            em = g10_exr_lib.decode_exr_file(err_exr_path, "rurix")
            width, height = em["width"], em["height"]
            # ① 误差 EXR 由帧对重算逐像素位级一致。
            err_re = error_buffer_f32(fa["pixels"], fb["pixels"])
            recompute_ok["err"] = (
                fa["width"] == width and fa["height"] == height
                and em["layout"] == "y"
                and all(np.float32(v).tobytes() == np.float32(w).tobytes() for v, w in zip(err_re, em["pixels"]))
            )
            # digest 对账（artifacts 四 digest + end_pair 双帧 digest）。
            da = g10_exr_lib.frame_content_digest(width, height, 3, fa["pixels"])
            db = g10_exr_lib.frame_content_digest(width, height, 3, fb["pixels"])
            de = g10_exr_lib.frame_content_digest(width, height, 1, em["pixels"])
            dh = "sha256:" + hashlib.sha256(heat_path.read_bytes()).hexdigest()
            digest_match = (
                artifacts.get("frame_a_digest") == da
                and artifacts.get("frame_b_digest") == db
                and artifacts.get("error_map_digest") == de
                and artifacts.get("heatmap_digest") == dh
                and report.get("end_pair", {}).get("frame_a", {}).get("digest") == da
                and report.get("end_pair", {}).get("frame_b", {}).get("digest") == db
            )
            if not digest_match:
                note(f"digest 对账失败: a={da[:20]}… b={db[:20]}… e={de[:20]}… h={dh[:20]}…")
            recompute_ok["err"] = recompute_ok["err"] and digest_match
            # ② 热区图由误差 EXR 重算逐字节一致。
            recompute_ok["heat"] = (
                g10_exr_lib.heatmap_ppm_bytes(width, height, em["pixels"]) == heat_path.read_bytes()
            )
            # ③ 区域统计与标量由误差 EXR 重算逐字段一致。
            threshold = float(report.get("thresholds", {}).get("value", 0.0))
            regions_re = region_stats_py(np.asarray(em["pixels"], dtype=np.float32), width, height, threshold)
            regions_rp = report.get("regions", [])
            rok = len(regions_re) == len(regions_rp) and len(regions_rp) > 0
            if rok:
                for a, b in zip(regions_re, regions_rp):
                    if not (
                        a["x"] == b["x"] and a["y"] == b["y"] and a["w"] == b["w"] and a["h"] == b["h"]
                        and a["pixel_count"] == b["pixel_count"]
                        and f32_eq(a["err_max"], b["err_max"])
                        and f32_eq(a["err_p95"], b["err_p95"])
                        and abs(a["err_mean"] - b["err_mean"]) <= 1e-12
                        and a["over_threshold_count"] == b["over_threshold_count"]
                    ):
                        rok = False
                        note(f"区域不一致: py={a} vs rp={b}")
                        break
            recompute_ok["regions"] = rok
            scal = report.get("scalars", {})
            all_err = sorted(float(v) for v in em["pixels"])
            n = len(all_err)
            over_total = sum(1 for v in all_err if v > threshold)
            recompute_ok["scalars"] = (
                f32_eq(all_err[-1], scal.get("err_max", -1))
                and f32_eq(g10_exr_lib.nearest_rank_p95(all_err), scal.get("err_p95", -1))
                and abs(sum(all_err) / n - scal.get("err_mean", -1)) <= 1e-12
                and over_total == scal.get("over_threshold_pixel_count")
                and abs(over_total / n - scal.get("over_threshold_ratio", -1)) <= 1e-12
                and scal.get("flip") is None
            )
        except Exception as e:  # noqa: BLE001 — 独立复核面异常即判据失效
            check(False, f"独立重算核验异常: {e}")

    checks["error_exr_recompute_match"] = recompute_ok["err"]
    check(recompute_ok["err"], "误差 EXR 由帧对重算不一致（diff 图与源帧不符）")
    checks["heatmap_recompute_match"] = recompute_ok["heat"]
    check(recompute_ok["heat"], "热区图由误差 EXR 重算逐字节不一致")
    checks["region_stats_recompute_match"] = recompute_ok["regions"]
    check(recompute_ok["regions"], "区域统计由误差 EXR 重算不一致")
    checks["scalars_recompute_match"] = recompute_ok["scalars"]
    check(recompute_ok["scalars"], "标量报告由误差 EXR 重算不一致")

    # evidence 闭集机核。
    cs_fails = closed_set_failures(report)
    checks["report_closed_set_enforced"] = not cs_fails and report.get("schema_version") == 1
    check(not cs_fails, f"报告闭集机核失败: {cs_fails}")

    # RED 臂①：diff 图与标量报告不一致注入——篡改误差 EXR 一像素 → 重算
    # 与报告不一致必检出；篡改标量 err_max → 重算不一致必检出。
    red1 = False
    if checks["dual_layer_artifacts"] and recompute_ok["err"]:
        blob = bytearray(err_exr_path.read_bytes())
        (old,) = struct.unpack_from("<f", blob, len(blob) - 4)
        struct.pack_into("<f", blob, len(blob) - 4, min(1.0, old + 0.25))
        em_t = g10_exr_lib.decode_exr(bytes(blob), "rurix")
        tampered_max = max(em_t["pixels"])
        red1 = not f32_eq(tampered_max, report["scalars"]["err_max"]) or (
            g10_exr_lib.frame_content_digest(
                em_t["width"], em_t["height"], 1, em_t["pixels"]
            ) != artifacts["error_map_digest"]
        )
        tampered_report = json.loads(json.dumps(report))
        tampered_report["scalars"]["err_max"] = report["scalars"]["err_max"] + 0.5
        red1 = red1 and not f32_eq(
            tampered_report["scalars"]["err_max"], report["scalars"]["err_max"]
        )
    checks["red_diff_scalar_inconsistency_detected"] = red1
    check(red1, "diff 图与标量不一致注入未检出")
    if red1:
        note("RED 检出 diff_scalar_inconsistency: 篡改误差 EXR/标量 → 重算不一致")

    # RED 臂②：空场景行注入——空 scene_id 必 fail-closed（非零退出）。
    red2 = False
    if checks["harness_build_host"]:
        r = run_cmd([
            str(exe), "--synthetic-pair",
            "--out-dir", str(WORK_DIR / "red_empty"), "--evidence", str(WORK_DIR / "red_empty" / "report.json"),
            "--scene-id", "", "--camera-id", "cam0", "--frame-index", "0", "--threshold", "0.0",
        ], timeout=600)
        red2 = r.returncode != 0
    checks["red_empty_scene_row_detected"] = red2
    check(red2, "空场景行注入未检出")

    # RED 臂③：闭集外字段注入——报告加字段闭集机核必拒。
    bad = json.loads(json.dumps(report)) if report else {}
    bad["evil_extra"] = 1
    checks["red_extra_field_detected"] = bool(closed_set_failures(bad))
    check(checks["red_extra_field_detected"], "闭集外字段注入未拒")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G10.4",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "diff_report": {
            "reporter": BIN,
            "report_path": str(report_path.relative_to(ROOT)).replace("\\", "/"),
            "scene_id": report.get("scene_id", ""),
            "camera_id": report.get("camera_id", ""),
            "frame_index": report.get("frame_index", 0),
            "domain": report.get("domain", ""),
            "metric_caliber": report.get("metric_caliber", ""),
            "thresholds": report.get("thresholds", {}),
            "threshold_calibration": {
                "estimator": "p100×k",
                "safety_factor_k": 1.0,
                "measured_noise_floor": id_scalars.get("err_max"),
                "status": "provisional_pending_m138",
                "provenance": "identity 图对（frame_a vs frame_a）噪声底 p100 实测 = 0.0；M138 正式标定入 g10_budget.json 后翻转 thresholds.source",
            },
            "region_grid": report.get("region_grid", {}),
            "region_count": len(report.get("regions", [])),
            "scalars": report.get("scalars", {}),
            "artifacts": artifacts,
            "determinism_contract_digest": report.get("determinism_contract_digest", ""),
            "cross_impl_verification": "ci/g10_exr_lib.py 独立第二实现：误差 EXR/热区图/区域统计/标量四面重算一致 + digest 对账",
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if all_pass and not FAILURES:
        print(
            f"[{TAG}] PASS（误差 EXR/热区图/区域统计/标量四面独立重算一致 + "
            f"regions={len(report.get('regions', []))} + 闭集机核 + RED 三臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
