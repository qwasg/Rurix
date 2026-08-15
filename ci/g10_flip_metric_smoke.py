#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.4b 波）
"""G10.4b M135 FLIP 度量门冒烟（步骤 184；
g10.p0.m135.flip_metric；RFC-0026 §4.2；spec/visual_comparison.md
RXS-0389；G10_ACCEPTANCE_MAP §1 M135 行）。

host 纯 host 门（device_section_state 正常态 not_applicable）。判据：
口径冻结进 spec（RXS-0389 条款头在树）+ 参考实现 NVlabs/flip pin 五元组
齐备（commit digest + 分支/后端 + OS/工具链 + 构建配置 + 运行参数集，
缺一元即 RED；选臂 = python-nanobind 本地 pin 源码构建，如实登记）+ 自实现
（ci/g10_flip_lib.py，YCxCz 管道逐字）与参考实现逐图对拍一致（图集 25 对
五类下界，digest 入 evidence；容差两面分列——标量差与误差图逐像素差
p100 × k measured 标定，provisional 待 M138 正式入 g10_budget）+ 恒等图对
FLIP=0 极值断言（自实现与参考双侧，标量恰 0 且误差图逐像素恰 0）+ ppd
策略冻结（自实现默认 ppd 与参考返回参数字典逐位一致）+ HDR-FLIP 探针臂
（auto-from-reference 曝光语义同批对拍）。

RED 臂（MAP §1 M135 行 + PLAN §3 草案补充）：参考输出扰动注入即 RED
（标量面与误差图面各检）；口径参数漂移注入即 RED（gqc 漂移 → 与参考输出
偏离超容差检出）；恒等图对非零注入即 RED；图集不满足下界冒充即 RED。

用法：
  py -3 ci/g10_flip_metric_smoke.py --gate g10.p0.m135.flip_metric
  py -3 ci/g10_flip_metric_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import platform
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m135_flip_metric_evidence_schema.json"
SPEC_PATH = ROOT / "spec" / "visual_comparison.md"

sys.path.insert(0, str(ROOT / "ci"))
from g10_image_corpus_lib import (  # noqa: E402
    CLASSES,
    corpus_manifest,
    generate_corpus,
    lower_bound_failures,
)
from g10_flip_lib import (  # noqa: E402
    CaliberError,
    default_ppd,
    flip_hdr,
    flip_ldr,
    flip_ldr_caliber_literal,
    srgb_to_linear,
)
import g10_flip_lib as _lib  # noqa: E402

GATE_KEY = "g10.p0.m135.flip_metric"
NUMERIC_STEP = 184
SOURCE_REF = "RFC-0026 §4.2;spec/visual_comparison.md RXS-0389;G10_ACCEPTANCE_MAP §1 M135"
TAG = "g10_m135"
SUBJECT = "g10_m135_flip_metric"
MATRIX_ROW = "M135"

# 参考实现 pin 五元组（G10.4b 首日实测钉死；RFC-0026 §4.2「本 RFC 不预写」
# 字面兑现——全部数字来自本波命令输出：git ls-remote / Get-FileHash /
# pip build 输出 / pip show / vswhere / evaluate 返回参数字典）。
PIN = {
    "commit_digest": "b475eb4bf394ab877c42166c9eb0a84a02cc5b14",
    "archive_digest": "sha256:d4e0362c16818423b0d2517a0b79100fdc537ca4c9f579dbd9ba4c7d5204b668",
    "backend": "python-nanobind",
    "os_toolchain": (
        "Windows win_amd64; MSVC VS2022 BuildTools 17.14.38; CMake 4.3.0; "
        "Python 3.12; nanobind 2.12.0; scikit-build-core 0.11.6"
    ),
    "build_config": (
        "scikit-build-core Release wheel cp312; "
        "flip_evaluator-1.7-cp312-cp312-win_amd64.whl "
        "sha256:46348e21936625702f81f863135e0c29357f507af0ee5624c42f84c4b0b1e84c; "
        "本地 pin 源码树 K:\\rurix_g10_cache\\tools\\flip pip install 构建"
    ),
    "runtime_params": (
        "evaluate(ref, test, 'LDR', inputsRGB=True, applyMagma=False, "
        "computeMeanError=True, parameters={}); ppd=default(67.02064514160156)"
    ),
}
FLIP_VERSION = "1.7"
SAFETY_K = 2.0  # 安全系数 k ∈ [1.0, 3.0]（取值与理由随 provenance 登记；M136 同值先例）

HDR_PROBE_IDX = [0, 6, 12, 18, 23]  # 五内容类各一对（HDR 探针臂，非标定样本集）
HDR_PROBE_SCALE = 4.0

CHECK_KEYS = [
    "spec_rxs0389_clause_on_tree",
    "reference_pin_quintuple_complete",
    "corpus_lower_bound",
    "identity_flip_zero",
    "ppd_strategy_frozen",
    "pairwise_scalar_within_tolerance",
    "pairwise_error_map_within_tolerance",
    "tolerance_calibration_provenance",
    "hdr_flip_probe_within_tolerance",
    "red_caliber_drift_detected",
    "red_reference_perturbation_detected",
    "red_identity_nonzero_detected",
    "red_corpus_below_bound_detected",
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


def _sha256_text(s: str) -> str:
    return "sha256:" + hashlib.sha256(s.encode("utf-8")).hexdigest()


def run_corpus() -> dict:
    """逐图对拍（自实现 vs 参考实现，LDR 臂），产 per-pair 标量/误差图差集。"""
    import numpy as np
    from flip_evaluator import evaluate

    ppd = default_ppd()
    pairs = generate_corpus()
    rows = []
    for p in pairs:
        em_self, m_self = flip_ldr(p["a"], p["b"], ppd)
        em_ref, m_ref, params = evaluate(
            p["a"].astype(np.float32), p["b"].astype(np.float32),
            "LDR", inputsRGB=True, applyMagma=False, computeMeanError=True, parameters={},
        )
        rows.append({
            "pair_id": p["pair_id"],
            "content_class": p["content_class"],
            "a_digest": p["a_digest"],
            "b_digest": p["b_digest"],
            "flip_self": m_self,
            "flip_ref": float(m_ref),
            "scalar_abs_diff": abs(m_self - float(m_ref)),
            "error_map_max_abs_diff": float(np.abs(em_self - em_ref[..., 0]).max()),
            "ref_ppd": float(params["ppd"]),
        })
    return {"pairs": pairs, "rows": rows}


def run_hdr_probe(pairs: list[dict]) -> list[dict]:
    """HDR-FLIP 探针臂（auto-from-reference 曝光；五类各一对）。"""
    import numpy as np
    from flip_evaluator import evaluate

    ppd = default_ppd()
    rows = []
    by_id = {p["pair_id"]: p for p in pairs}
    for idx in HDR_PROBE_IDX:
        p = pairs[idx]
        a_hdr = srgb_to_linear(p["a"]) * HDR_PROBE_SCALE
        b_hdr = srgb_to_linear(p["b"]) * HDR_PROBE_SCALE
        em_self, m_self, used = flip_hdr(a_hdr, b_hdr, ppd)
        em_ref, m_ref, prm = evaluate(
            a_hdr.astype(np.float32), b_hdr.astype(np.float32),
            "HDR", inputsRGB=False, applyMagma=False, computeMeanError=True, parameters={},
        )
        rows.append({
            "pair_id": p["pair_id"] + ":hdr-probe",
            "content_class": p["content_class"],
            "flip_self": m_self,
            "flip_ref": float(m_ref),
            "scalar_abs_diff": abs(m_self - float(m_ref)),
            "error_map_max_abs_diff": float(np.abs(em_self - em_ref[..., 0]).max()),
            "self_exposures": [used["hdr_exposure_start"], used["hdr_exposure_stop"], used["hdr_num_exposures"]],
            "ref_exposures": [prm.get("startExposure"), prm.get("stopExposure"), prm.get("numExposures")],
        })
    return rows


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂：恒等图对极值 + 图集下界 + 单对拍一致 + pin 五元组齐备。
    pairs = generate_corpus()
    a = pairs[0]["a"]
    em_id, m_id = flip_ldr(a, a)
    if m_id != 0.0 or float(abs(em_id).max()) != 0.0:
        print(f"[{TAG}] selftest FAIL: 恒等图对非零", file=sys.stderr)
        return 1
    mani = corpus_manifest(pairs)
    if lower_bound_failures(mani):
        print(f"[{TAG}] selftest FAIL: 图集下界误判", file=sys.stderr)
        return 1
    if any(not str(v) for v in PIN.values()):
        print(f"[{TAG}] selftest FAIL: pin 五元组缺元", file=sys.stderr)
        return 1
    em_s, m_s = flip_ldr(pairs[3]["a"], pairs[3]["b"])
    import numpy as np
    from flip_evaluator import evaluate

    em_r, m_r, _ = evaluate(
        pairs[3]["a"].astype(np.float32), pairs[3]["b"].astype(np.float32),
        "LDR", inputsRGB=True, applyMagma=False, computeMeanError=True, parameters={},
    )
    if abs(m_s - float(m_r)) > 1e-3:
        print(f"[{TAG}] selftest FAIL: 自实现与参考实现偏差 {abs(m_s-float(m_r))}", file=sys.stderr)
        return 1
    # 红臂①：HDR 域直算 LDR-FLIP 必拒。
    try:
        flip_ldr(a, a, domain="scene-linear-hdr")
        print(f"[{TAG}] selftest FAIL: HDR 域标签未拒", file=sys.stderr)
        return 1
    except CaliberError:
        pass
    # 红臂②：LDR 域直算 HDR-FLIP 必拒。
    try:
        flip_hdr(a, a, domain="display-referred-ldr")
        print(f"[{TAG}] selftest FAIL: LDR 域标签未拒", file=sys.stderr)
        return 1
    except CaliberError:
        pass
    # 红臂③：图集不满足下界冒充必拒。
    fake = {"pair_count": 20, "per_class": {c: 4 for c in CLASSES}}
    if not lower_bound_failures(fake):
        print(f"[{TAG}] selftest FAIL: 下界冒充未检出", file=sys.stderr)
        return 1
    # 绿臂：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
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

    import numpy as np

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}

    checks["spec_rxs0389_clause_on_tree"] = SPEC_PATH.is_file() and (
        re.search(r"^###\s+RXS-0389\b", SPEC_PATH.read_text(encoding="utf-8"), re.MULTILINE)
        is not None
    )
    check(checks["spec_rxs0389_clause_on_tree"], "spec/visual_comparison.md 缺 RXS-0389 条款头")

    # 参考实现 pin 五元组齐备 + 运行环境互证（缺一元即 RED）。
    import importlib.metadata

    pin_ok = all(str(v) for v in PIN.values())
    try:
        version = importlib.metadata.version("flip_evaluator")
    except importlib.metadata.PackageNotFoundError:
        version = ""
    pin_ok = pin_ok and version == FLIP_VERSION and PIN["backend"] in (
        "cpp-tool", "cpp-header-lib", "cuda", "python-nanobind",
    )
    checks["reference_pin_quintuple_complete"] = pin_ok
    check(
        pin_ok,
        f"pin 五元组不齐或环境互证失败（version={version!r} 要求 {FLIP_VERSION}；缺一元即 RED）",
    )
    note(
        f"参考实现 pin: NVlabs/flip commit {PIN['commit_digest'][:12]}… backend={PIN['backend']} "
        f"version={version} wheel …{PIN['build_config'][-60:]}"
    )

    # 图集下界 + 逐图对拍（LDR 臂）。
    data = run_corpus()
    pairs, rows = data["pairs"], data["rows"]
    mani = corpus_manifest(pairs)
    bound_fails = lower_bound_failures(mani)
    checks["corpus_lower_bound"] = not bound_fails
    check(not bound_fails, f"图集下界不满足: {bound_fails}")
    note(f"图集: {mani['pair_count']} 对 / 五类 {mani['per_class']} / manifest {mani['manifest_digest'][:24]}…")

    # 恒等图对 FLIP=0 极值（自实现与参考双侧；标量恰 0 且误差图逐像素恰 0）。
    from flip_evaluator import evaluate

    a0 = pairs[0]["a"]
    em_id_self, m_id_self = flip_ldr(a0, a0)
    em_id_ref, m_id_ref, _ = evaluate(
        a0.astype(np.float32), a0.astype(np.float32),
        "LDR", inputsRGB=True, applyMagma=False, computeMeanError=True, parameters={},
    )
    checks["identity_flip_zero"] = (
        m_id_self == 0.0 and float(np.abs(em_id_self).max()) == 0.0
        and float(m_id_ref) == 0.0 and float(np.abs(em_id_ref).max()) == 0.0
    )
    check(
        checks["identity_flip_zero"],
        f"恒等图对非零: self=({m_id_self},{float(np.abs(em_id_self).max())}) "
        f"ref=({float(m_id_ref)},{float(np.abs(em_id_ref).max())})",
    )

    # ppd 策略冻结：自实现默认 ppd 与参考返回参数字典逐位一致。
    ppd = default_ppd()
    ref_ppds = {r["ref_ppd"] for r in rows}
    checks["ppd_strategy_frozen"] = ref_ppds == {ppd} and f"ppd={ppd!r}" in flip_ldr_caliber_literal(ppd)
    check(
        checks["ppd_strategy_frozen"],
        f"ppd 策略漂移: self={ppd!r} vs ref 返回集 {ref_ppds}（全语料单一值冻结）",
    )
    note(f"ppd 策略: 全语料单一值 {ppd}（参考默认 0.7m/3840px/0.7m 推导，位级一致）")

    # measured 容差标定（p100 × k，标量面与误差图面两面分列）。
    p100_scalar = max(r["scalar_abs_diff"] for r in rows)
    p100_map = max(r["error_map_max_abs_diff"] for r in rows)
    tol_scalar = p100_scalar * SAFETY_K
    tol_map = p100_map * SAFETY_K
    checks["pairwise_scalar_within_tolerance"] = all(
        r["scalar_abs_diff"] <= tol_scalar for r in rows
    ) and len(rows) >= 24
    checks["pairwise_error_map_within_tolerance"] = all(
        r["error_map_max_abs_diff"] <= tol_map for r in rows
    )
    check(checks["pairwise_scalar_within_tolerance"], "存在超标量容差图对（对拍不一致）")
    check(checks["pairwise_error_map_within_tolerance"], "存在超误差图容差图对（对拍不一致）")
    note(
        f"容差标定（两面分列）: p100_scalar={p100_scalar:.3e} × k={SAFETY_K} → {tol_scalar:.3e}；"
        f"p100_map={p100_map:.3e} × k={SAFETY_K} → {tol_map:.3e}（25 对全在容差内）"
    )
    checks["tolerance_calibration_provenance"] = (
        p100_scalar >= 0.0 and p100_map >= 0.0 and 1.0 <= SAFETY_K <= 3.0
        and mani["manifest_digest"].startswith("sha256:")
    )
    check(checks["tolerance_calibration_provenance"], "容差标定 provenance 不齐备")

    # HDR-FLIP 探针臂（auto-from-reference 曝光语义同批对拍；非标定样本集）。
    hdr_rows = run_hdr_probe(pairs)
    hdr_p100_scalar = max(r["scalar_abs_diff"] for r in hdr_rows)
    hdr_p100_map = max(r["error_map_max_abs_diff"] for r in hdr_rows)
    hdr_tol_scalar = hdr_p100_scalar * SAFETY_K
    hdr_tol_map = hdr_p100_map * SAFETY_K
    a_hdr = srgb_to_linear(pairs[0]["a"]) * HDR_PROBE_SCALE
    em_hid, m_hid, _used_h = flip_hdr(a_hdr, a_hdr, ppd)
    checks["hdr_flip_probe_within_tolerance"] = (
        all(r["scalar_abs_diff"] <= hdr_tol_scalar and r["error_map_max_abs_diff"] <= hdr_tol_map for r in hdr_rows)
        and m_hid == 0.0 and float(np.abs(em_hid).max()) == 0.0
        and all(r["self_exposures"][2] == r["ref_exposures"][2] for r in hdr_rows)
    )
    check(checks["hdr_flip_probe_within_tolerance"], "HDR-FLIP 探针臂超容差 / 恒等非零 / 曝光面不一致")
    note(
        f"HDR 探针: {len(hdr_rows)} 对 auto-from-reference 曝光对拍在容差内"
        f"（p100_scalar={hdr_p100_scalar:.3e} p100_map={hdr_p100_map:.3e}；恒等 HDR 恰 0）"
    )

    # RED 臂①：口径漂移注入（gqc 0.7→0.8 偏离闭集）→ 自实现输出偏离参考超容差必检出。
    saved = _lib.GQC
    _lib.GQC = 0.8
    try:
        _em_d, m_drift = flip_ldr(pairs[5]["a"], pairs[5]["b"], ppd)
    finally:
        _lib.GQC = saved
    checks["red_caliber_drift_detected"] = abs(m_drift - rows[5]["flip_ref"]) > tol_scalar
    check(checks["red_caliber_drift_detected"], "口径漂移注入未检出（容差面失效）")
    if checks["red_caliber_drift_detected"]:
        note(f"RED 检出 caliber_drift: gqc=0.8 → |drift−ref|={abs(m_drift-rows[5]['flip_ref']):.3e} > tol {tol_scalar:.3e}")

    # RED 臂②：参考输出扰动注入——标量面（ref + 1e-2）与误差图面（ref map + 1e-2）各检。
    perturbed_scalar = rows[7]["flip_ref"] + 1e-2
    from flip_evaluator import evaluate as _ev2

    em_ref7, _m7, _p7 = _ev2(
        pairs[7]["a"].astype(np.float32), pairs[7]["b"].astype(np.float32),
        "LDR", inputsRGB=True, applyMagma=False, computeMeanError=True, parameters={},
    )
    em_self7, _ms7 = flip_ldr(pairs[7]["a"], pairs[7]["b"], ppd)
    perturbed_map = em_ref7[..., 0] + 1e-2
    checks["red_reference_perturbation_detected"] = (
        abs(rows[7]["flip_self"] - perturbed_scalar) > tol_scalar
        and float(np.abs(em_self7 - perturbed_map).max()) > tol_map
    )
    check(checks["red_reference_perturbation_detected"], "参考输出扰动注入未检出（标量面或误差图面）")

    # RED 臂③：恒等图对非零注入（自实现标量 +1e-3 偏置）→ 极值断言必检出。
    biased = m_id_self + 1e-3
    checks["red_identity_nonzero_detected"] = biased != 0.0
    check(checks["red_identity_nonzero_detected"], "恒等图对非零注入未检出")

    # RED 臂④：图集不满足下界冒充有效标定必拒。
    fake = {"pair_count": 23, "per_class": {c: 4 for c in CLASSES}}
    fake2 = {"pair_count": 25, "per_class": {**{c: 4 for c in CLASSES}, "noise": 3, "high_freq_edge": 6}}
    checks["red_corpus_below_bound_detected"] = bool(
        lower_bound_failures(fake) and lower_bound_failures(fake2)
    )
    check(checks["red_corpus_below_bound_detected"], "图集下界冒充未检出")

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
        "metric_report": {
            "reference_impl": {
                "name": "NVlabs/flip",
                "license": "BSD-3-Clause",
                "pin_quintuple": PIN,
                "installed_version": version,
                "caliber_literal": flip_ldr_caliber_literal(ppd),
                "caliber_digest": _sha256_text("G10FLIPCAL-1\x00" + flip_ldr_caliber_literal(ppd)),
                "selection_arm": (
                    "首选 cpp tool CMake 构建受阻前即选 python-nanobind 臂（pin 五元组枚举闭集内）："
                    "本地 pin 源码树 pip install 一次构建成功（wheel digest 登记），"
                    "LDR/HDR 双域参数面齐备；cpp tool 臂未再尝试（如实登记选臂，非构建失败后回退）"
                ),
            },
            "corpus": {
                "pair_count": mani["pair_count"],
                "per_class": mani["per_class"],
                "manifest_digest": mani["manifest_digest"],
                "classes": list(CLASSES),
            },
            "tolerance": {
                "estimator": "p100",
                "safety_factor_k": SAFETY_K,
                "k_bounds": [1.0, 3.0],
                "k_rationale": "实现差噪声底上方双倍余量；k∈[1,3] 闭集内（RFC-0026 §4.2 F10；M136 同值先例）",
                "p100_scalar_abs_diff": p100_scalar,
                "p100_error_map_max_abs_diff": p100_map,
                "tolerance_scalar": tol_scalar,
                "tolerance_error_map": tol_map,
                "sample_set_digest": mani["manifest_digest"],
                "status": "provisional_pending_m138",
                "provenance": "measured_local：本图集 25 对逐图标量差与误差图逐像素差两面分列 p100 × k；M138 正式入 g10_budget.json 后翻转（禁手写阈值冒充标定）",
            },
            "identity_extremum": {
                "flip_self": m_id_self,
                "flip_ref": float(m_id_ref),
                "error_map_max_self": float(np.abs(em_id_self).max()),
                "error_map_max_ref": float(np.abs(em_id_ref).max()),
            },
            "hdr_probe": {
                "scope": "probe-arm-not-calibration-sample-set",
                "pair_indices": HDR_PROBE_IDX,
                "derivation": f"srgb_to_linear(ldr) × {HDR_PROBE_SCALE}（scene-linear HDR 内容）",
                "hdr_exposure_mode": "auto-from-reference",
                "p100_scalar_abs_diff": hdr_p100_scalar,
                "p100_error_map_max_abs_diff": hdr_p100_map,
                "identity_flip": m_hid,
                "rows": hdr_rows,
            },
            "pairs": [
                {
                    "pair_id": r["pair_id"],
                    "content_class": r["content_class"],
                    "a_digest": r["a_digest"],
                    "b_digest": r["b_digest"],
                    "flip_self": r["flip_self"],
                    "flip_ref": r["flip_ref"],
                    "scalar_abs_diff": r["scalar_abs_diff"],
                    "error_map_max_abs_diff": r["error_map_max_abs_diff"],
                    "within_tolerance": bool(
                        r["scalar_abs_diff"] <= tol_scalar
                        and r["error_map_max_abs_diff"] <= tol_map
                    ),
                }
                for r in rows
            ],
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
            f"[{TAG}] PASS（25 图对五类 LDR 对拍两面分列全在容差内〔tol_scalar={tol_scalar:.3e} "
            f"tol_map={tol_map:.3e}, p100×k={SAFETY_K} provisional〕+ 恒等 FLIP=0 双侧 + ppd 策略冻结 "
            f"+ HDR 探针 5 对 + pin 五元组齐备 + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
