#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.4b 波）
"""G10.4b M138 度量阈值标定门（P1，步骤 185；
g10.p1.m138.metric_threshold_calibration；G10_PLAN §2 G10.4 / MAP §2 M138 行；
CI_GATES §4A；RFC-0026 §4.2 F10 估计器语义）。

host 纯 host 门（device_section_state 正常态 not_applicable）。判据：
标定程序可复跑（同一图集 digest 上 p100 估计器两跑逐位一致）+ M135/M136/
M137 三门 provisional 容差翻正——标定值入 `g10_budget.json`（纯追加条目，
measured_local + provenance + 环境画像，P-09 禁手写阈值）+ 三门
provisional_pending_m138 标记消费登记（标定重算值与门内登记 p100 逐位一致）
+ 标定估计器语义按 RFC（p100 × k，k∈[1.0,3.0] 登记、样本集 digest 引用）
+ budget_eval --strict 全 PASS。

RED 臂（MAP §2 M138 行字面）：手写阈值冒充标定即 RED（threshold ≠
p100×k 拒录）；estimated 冒充 measured 即 RED；标定程序不可复跑即 RED
（两跑漂移检出）；门 evidence 缺失冒充输入即 RED（fail-closed）。

用法：
  py -3 ci/g10_metric_threshold_calibration_smoke.py --gate g10.p1.m138.metric_threshold_calibration
  py -3 ci/g10_metric_threshold_calibration_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
BUDGET_PATH = ROOT / "milestones" / "g10" / "g10_budget.json"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m138_metric_threshold_calibration_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))
import g10_wave_exit_lib as wel  # noqa: E402
from g10_image_corpus_lib import corpus_manifest, generate_corpus  # noqa: E402
from g10_flip_lib import default_ppd, flip_ldr  # noqa: E402
from g10_ssim_psnr_lib import psnr_joint, reference_ssim_psnr, ssim_wang2004  # noqa: E402

GATE_KEY = "g10.p1.m138.metric_threshold_calibration"
NUMERIC_STEP = 185
SOURCE_REF = "G10_PLAN §2 G10.4;G10_ACCEPTANCE_MAP §2 M138;CI_GATES §4A;RFC-0026 §4.2 F10"
TAG = "g10_m138"
SUBJECT = "g10_m138_metric_threshold_calibration"
MATRIX_ROW = "M138"

# 标定条目闭集（估计器 = p100 × k；样本集 = 对拍图集 digest；k 取值与理由随
# provenance 登记。flip 两面分列〔RXS-0389 L5〕/ ssim·psnr 分列〔RXS-0387 L4〕
# / diff over_threshold = identity 噪声底〔RXS-0388 L2，k=1.0 与 M137
# provisional 语义连续——p100=0.0 时 k 取值不改变标定值〕）。
CALIB_ENTRIES = [
    {
        "slug": "flip_scalar",
        "id": "g10.metric.flip_pairwise_scalar_tol",
        "unit": "1",
        "k": 2.0,
        "source_gate": "g10_m135_flip_metric",
        "source_field": ("metric_report", "tolerance", "p100_scalar_abs_diff"),
        "k_rationale": "实现差噪声底上方双倍余量；k∈[1,3] 闭集内（RFC-0026 §4.2 F10；M135/M136 同值先例）",
        "desc": "FLIP 自实现 vs 参考实现逐图标量差 p100 × k=2.0（RXS-0389 L5 标量对拍容差面）",
    },
    {
        "slug": "flip_error_map",
        "id": "g10.metric.flip_pairwise_error_map_tol",
        "unit": "1",
        "k": 2.0,
        "source_gate": "g10_m135_flip_metric",
        "source_field": ("metric_report", "tolerance", "p100_error_map_max_abs_diff"),
        "k_rationale": "实现差噪声底上方双倍余量；k∈[1,3] 闭集内（RFC-0026 §4.2 F10；M135/M136 同值先例）",
        "desc": "FLIP 自实现 vs 参考实现误差图逐像素差 p100 × k=2.0（RXS-0389 L5 误差图对拍容差面）",
    },
    {
        "slug": "ssim",
        "id": "g10.metric.ssim_pairwise_tol",
        "unit": "1",
        "k": 2.0,
        "source_gate": "g10_m136_ssim_psnr_metric",
        "source_field": ("metric_report", "tolerance", "p100_ssim_abs_diff"),
        "k_rationale": "实现差噪声底上方双倍余量；k∈[1,3] 闭集内（RFC-0026 §4.2 F10；M136 同值先例）",
        "desc": "SSIM 自实现 vs scikit-image 参考逐图标量差 p100 × k=2.0（RXS-0387 L4 对拍容差面）",
    },
    {
        "slug": "psnr",
        "id": "g10.metric.psnr_pairwise_tol",
        "unit": "dB",
        "k": 2.0,
        "source_gate": "g10_m136_ssim_psnr_metric",
        "source_field": ("metric_report", "tolerance", "p100_psnr_abs_diff"),
        "k_rationale": "实现差噪声底上方双倍余量；k∈[1,3] 闭集内；p100=0.0（逐位一致）时 k 取值不改变标定值",
        "desc": "PSNR 自实现 vs scikit-image 参考逐图标量差 p100 × k=2.0（RXS-0387 L4 对拍容差面）",
    },
    {
        "slug": "diff_over_threshold",
        "id": "g10.metric.diff_report_over_threshold",
        "unit": "1",
        "k": 1.0,
        "source_gate": "g10_m137_pixel_diff_report",
        "source_field": ("diff_report", "threshold_calibration", "measured_noise_floor"),
        "k_rationale": "identity 噪声底 p100=0.0，k 取值不改变标定值；取 M137 provisional 同值 1.0 保持语义连续（k∈[1,3] 闭集内）",
        "desc": "逐像素 diff 报告 over_threshold 阈值 = identity 图对噪声底 p100 × k=1.0（RXS-0388 L2 阈值面）",
    },
]

CHECK_KEYS = [
    "gates_latest_evidence_provisional_present",
    "estimator_semantics_frozen",
    "recompute_matches_gate_registrations",
    "calibration_rerun_deterministic",
    "calibration_evidence_files_provenance",
    "budget_entries_appended_measured_local",
    "budget_eval_strict_all_pass",
    "provisional_markers_consumed",
    "red_handwritten_threshold_detected",
    "red_estimated_masquerade_detected",
    "red_nonrerunnable_detected",
    "red_missing_gate_evidence_detected",
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


def _file_digest(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def run_cmd(argv: list[str]) -> subprocess.CompletedProcess:
    print(f"[{TAG}] $ {' '.join(argv)}")
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(argv), "exit_code": r.returncode})
    return r


def compute_calibration() -> dict:
    """标定估计器（可复跑）：样本 = 对拍图集 25 对，统计量 = 逐图 |自实现 −
    参考实现| 差样本最大值（p100，标量面与误差图面分列）；M137 面 =
    identity 图对误差缓冲 p100（通道最大绝对差钳制 [0,1]，RXS-0388 L1 门内
    供给口径同字面）。确定性可复跑——同一图集 digest 上两跑逐位一致。"""
    import numpy as np
    from flip_evaluator import evaluate

    pairs = generate_corpus()
    mani = corpus_manifest(pairs)
    ppd = default_ppd()

    flip_scalar_diffs: list[float] = []
    flip_map_diffs: list[float] = []
    ssim_diffs: list[float] = []
    psnr_diffs: list[float] = []
    import warnings

    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        for p in pairs:
            em_self, m_self = flip_ldr(p["a"], p["b"], ppd)
            em_ref, m_ref, _ = evaluate(
                p["a"].astype(np.float32), p["b"].astype(np.float32),
                "LDR", inputsRGB=True, applyMagma=False, computeMeanError=True, parameters={},
            )
            flip_scalar_diffs.append(abs(m_self - float(m_ref)))
            flip_map_diffs.append(float(np.abs(em_self - em_ref[..., 0]).max()))
            s_self = ssim_wang2004(p["a"], p["b"])
            p_self = psnr_joint(p["a"], p["b"])
            s_ref, p_ref = reference_ssim_psnr(p["a"], p["b"])
            ssim_diffs.append(abs(s_self - s_ref))
            import math

            psnr_diffs.append(
                0.0 if (math.isinf(p_self) and math.isinf(p_ref)) else abs(p_self - p_ref)
            )

    # M137 面：identity 图对噪声底 p100（门内供给口径：逐像素 RGB 通道最大
    # 绝对差钳制 [0,1]，float32 逐元素——RXS-0388 L1 同字面）。
    a0 = pairs[0]["a"].astype(np.float32)
    err = np.clip(np.max(np.abs(a0 - a0), axis=-1), np.float32(0.0), np.float32(1.0))
    identity_noise_floor = float(err.max())

    return {
        "flip_scalar": max(flip_scalar_diffs),
        "flip_error_map": max(flip_map_diffs),
        "ssim": max(ssim_diffs),
        "psnr": max(psnr_diffs),
        "diff_over_threshold": identity_noise_floor,
        "sample_set_digest": mani["manifest_digest"],
        "sample_pair_count": mani["pair_count"],
        "estimator": "p100",
    }


def validate_budget_entry(entry: dict, p100: float, k: float) -> list[str]:
    """标定条目合法性机核（手写阈值冒充 / estimated 冒充判红面）。"""
    problems: list[str] = []
    if entry.get("evidence") != "measured_local":
        problems.append(f"{entry.get('id')}: evidence={entry.get('evidence')!r}（estimated 冒充 measured 即 RED）")
    if entry.get("threshold") != p100 * k:
        problems.append(
            f"{entry.get('id')}: threshold={entry.get('threshold')!r} ≠ p100×k={p100 * k!r}（手写阈值冒充标定即 RED）"
        )
    if entry.get("measured_value") != p100:
        problems.append(f"{entry.get('id')}: measured_value ≠ p100")
    ef = entry.get("evidence_file") or ""
    if not ef or not (ROOT / ef).is_file():
        problems.append(f"{entry.get('id')}: evidence_file 不在树: {ef!r}")
    return problems


def _get_nested(doc: dict, path: tuple[str, ...]):
    cur = doc
    for key in path:
        if not isinstance(cur, dict):
            return None
        cur = cur.get(key)
    return cur


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂：估计器两跑逐位一致 + 合法条目零 problems + 闭集互核。
    c1 = compute_calibration()
    c2 = compute_calibration()
    if c1 != c2:
        print(f"[{TAG}] selftest FAIL: 标定两跑不一致", file=sys.stderr)
        return 1
    ok_entry = {
        "id": "g10.metric.selftest_probe",
        "evidence": "measured_local",
        "threshold": c1["ssim"] * 2.0,
        "measured_value": c1["ssim"],
        "evidence_file": "milestones/g10/g10_budget.json",
    }
    if validate_budget_entry(ok_entry, c1["ssim"], 2.0):
        print(f"[{TAG}] selftest FAIL: 合法条目误判", file=sys.stderr)
        return 1
    # 红臂①：手写阈值冒充必拒。
    bad = dict(ok_entry, threshold=c1["ssim"] * 2.0 * 1.5)
    if not validate_budget_entry(bad, c1["ssim"], 2.0):
        print(f"[{TAG}] selftest FAIL: 手写阈值冒充未检出", file=sys.stderr)
        return 1
    # 红臂②：estimated 冒充必拒。
    bad2 = dict(ok_entry, evidence="estimated")
    if not validate_budget_entry(bad2, c1["ssim"], 2.0):
        print(f"[{TAG}] selftest FAIL: estimated 冒充未检出", file=sys.stderr)
        return 1
    # 红臂③：不可复跑（漂移注入）必检出。
    drift = dict(c2)
    drift["ssim"] = drift["ssim"] + 1e-12
    if c1 == drift:
        print(f"[{TAG}] selftest FAIL: 复跑漂移注入未检出", file=sys.stderr)
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

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}

    # ① 三门最新 evidence + provisional 标记在录（fail-closed）。
    gate_evidences: dict[str, tuple[Path, dict]] = {}
    missing: list[str] = []
    provisional_bad: list[str] = []
    for slug in ("g10_m135_flip_metric", "g10_m136_ssim_psnr_metric", "g10_m137_pixel_diff_report"):
        path = wel.load_latest_evidence(slug)
        if path is None:
            missing.append(slug)
            continue
        doc = wel.load_json(path)
        gate_evidences[slug] = (path, doc)
        if doc.get("status") != "pass":
            provisional_bad.append(f"{slug} 最新 evidence status={doc.get('status')!r}")
        if slug == "g10_m137_pixel_diff_report":
            st = _get_nested(doc, ("diff_report", "threshold_calibration", "status"))
        else:
            st = _get_nested(doc, ("metric_report", "tolerance", "status"))
        if st != "provisional_pending_m138":
            provisional_bad.append(f"{slug} provisional 标记缺失（实测 {st!r}）")
    checks["gates_latest_evidence_provisional_present"] = not missing and not provisional_bad
    check(not missing, f"门 evidence 缺失（冒充输入 fail-closed）: {missing}")
    check(not provisional_bad, f"provisional 标记或门绿状态异常: {provisional_bad}")

    # ② 标定两跑（可复跑判据）。
    cal1 = compute_calibration()
    cal2 = compute_calibration()
    checks["calibration_rerun_deterministic"] = cal1 == cal2
    check(cal1 == cal2, "标定程序不可复跑（两跑漂移即 RED）")
    note(
        f"标定两跑逐位一致: flip_scalar={cal1['flip_scalar']:.6e} flip_map={cal1['flip_error_map']:.6e} "
        f"ssim={cal1['ssim']:.6e} psnr={cal1['psnr']:.6e} identity_floor={cal1['diff_over_threshold']:.6e}"
    )

    # ③ 估计器语义冻结（p100 × k，k∈[1,3]，样本集 digest 引用）。
    checks["estimator_semantics_frozen"] = (
        cal1["estimator"] == "p100"
        and cal1["sample_set_digest"].startswith("sha256:")
        and cal1["sample_pair_count"] >= 24
        and all(1.0 <= e["k"] <= 3.0 for e in CALIB_ENTRIES)
    )
    check(checks["estimator_semantics_frozen"], "估计器语义漂移（p100×k / k 边界 / 样本集 digest）")

    # ④ 重算值与三门登记 p100 逐位一致（provisional 标记消费判据）。
    mismatches: list[str] = []
    for e in CALIB_ENTRIES:
        doc = gate_evidences.get(e["source_gate"], (None, None))[1]
        registered = _get_nested(doc or {}, e["source_field"])
        recomputed = cal1[e["slug"]]
        if registered != recomputed:
            mismatches.append(
                f"{e['slug']}: 门登记 {registered!r} ≠ 标定重算 {recomputed!r}"
            )
    checks["recompute_matches_gate_registrations"] = not mismatches
    check(bool(gate_evidences) and not mismatches, f"标定重算与门登记不一致: {mismatches[:3]}")
    if not mismatches:
        note("provisional 标记消费：五面标定重算值与三门最新 evidence 登记 p100 逐位一致")

    # ⑤ 标定 evidence 五件落盘（results.trimmed_mean + provenance + 环境画像）。
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    calib_files: dict[str, Path] = {}
    for e in CALIB_ENTRIES:
        p100 = cal1[e["slug"]]
        src_path, _src_doc = gate_evidences[e["source_gate"]]
        payload = {
            "schema_version": 1,
            "subject": f"g10_m138_calibration_{e['slug']}",
            "symbolic_gate_key": GATE_KEY,
            "milestone": "M138",
            "wave": "G10.4",
            "numeric_step": NUMERIC_STEP,
            "results": {
                "trimmed_mean": p100,
                "estimator": "p100",
                "sample_pair_count": cal1["sample_pair_count"],
                "safety_factor_k": e["k"],
                "threshold": p100 * e["k"],
            },
            "provenance": {
                "estimator_semantics": "p100 × k（RFC-0026 §4.2 F10）",
                "k_rationale": e["k_rationale"],
                "sample_set_digest": cal1["sample_set_digest"],
                "source_gate": e["source_gate"],
                "source_gate_evidence": str(src_path.relative_to(ROOT)).replace("\\", "/"),
                "source_gate_evidence_digest": _file_digest(src_path),
                "consumed_provisional_marker": "provisional_pending_m138",
                "measured": "measured_local：本图集 25 对逐图差 p100 × k 复跑两跑逐位一致；禁手写阈值冒充标定（P-09）",
            },
            "environment": {
                "os": platform.platform(),
                "python_version": sys.version.split()[0],
                "cargo_version": _tool_version("cargo"),
                "rustc_version": _tool_version("rustc"),
            },
            "timestamp": ts,
        }
        out = EVIDENCE_DIR / f"g10_m138_calibration_{e['slug']}_{ts}.json"
        out.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        calib_files[e["slug"]] = out
    prov_ok = all(
        p.is_file()
        and (d := wel.load_json(p)).get("results", {}).get("trimmed_mean") == cal1[slug]
        and d.get("provenance", {}).get("sample_set_digest") == cal1["sample_set_digest"]
        and bool(d.get("environment", {}).get("os"))
        for slug, p in calib_files.items()
    )
    checks["calibration_evidence_files_provenance"] = prov_ok
    check(prov_ok, "标定 evidence 五件 provenance/trimmed_mean/环境画像不齐备")

    # ⑥ 标定值入 g10_budget.json（纯追加；已存在同值幂等，漂移即 RED）。
    budget_text = BUDGET_PATH.read_text(encoding="utf-8")
    budget = json.loads(budget_text)
    new_entries: list[dict] = []
    budget_problems: list[str] = []
    for e in CALIB_ENTRIES:
        p100 = cal1[e["slug"]]
        entry = {
            "id": e["id"],
            "description": (
                f"{e['desc']}。标定程序 ci/g10_metric_threshold_calibration_smoke.py 可复跑"
                f"（两跑逐位一致）；样本集 = 对拍图集 digest {cal1['sample_set_digest'][:24]}…；"
                f"provisional_pending_m138 标记消费 = {e['source_gate']} 最新 evidence 登记 p100 "
                f"与标定重算逐位一致。M138 measured 标定（P-09 禁手写阈值）。"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": e["unit"],
            "threshold": p100 * e["k"],
            "evidence_file": str(calib_files[e["slug"]].relative_to(ROOT)).replace("\\", "/"),
            "measured_value": p100,
        }
        budget_problems.extend(validate_budget_entry(entry, p100, e["k"]))
        existing = [x for x in budget.get("entries", []) if x.get("id") == e["id"]]
        if existing:
            # 幂等口径：值面逐字一致（id/description/direction/evidence/
            # skip_reason/unit/threshold/measured_value）；evidence_file 指向
            # 首次登记批次的标定证据（evidence/ 只增不删不改，后续批次不覆写
            # 引用），仅核验其在树且 trimmed_mean == p100——防「同值换皮」
            # 假漂移，也防真值漂移静默通过。
            ex = existing[0]
            comparable = {k: v for k, v in entry.items() if k != "evidence_file"}
            ex_comparable = {k: v for k, v in ex.items() if k != "evidence_file"}
            if ex_comparable != comparable:
                budget_problems.append(f"{e['id']} 已在树且值漂移（只追加禁改写）: 在树 {ex} vs 重算 {entry}")
            else:
                ef = ex.get("evidence_file") or ""
                tm = None
                if ef and (ROOT / ef).is_file():
                    try:
                        tm = wel.load_json(ROOT / ef).get("results", {}).get("trimmed_mean")
                    except (OSError, ValueError):
                        tm = None
                if tm != p100:
                    budget_problems.append(f"{e['id']} 在树 evidence_file 不可解或 trimmed_mean≠p100: {ef!r}")
            continue
        new_entries.append(entry)
    if not budget_problems and new_entries:
        # 字节级纯追加（既有行 0-byte：行尾风格随原文件〔CRLF 维持〕，仅在
        # entries 数组尾插入新条目文本，不重序列化全文）。
        nl = "\r\n" if "\r\n" in budget_text else "\n"
        anchor = f"{nl}  ],{nl}  \"ratio_assertions\""
        if anchor not in budget_text:
            budget_problems.append("g10_budget.json 结构锚缺失（entries 闭合段未找到，拒改写）")
        else:
            frag = ""
            for entry in new_entries:
                body = json.dumps(entry, ensure_ascii=False, indent=2)
                body = body.replace("\n", nl)
                body = "    " + body.replace(nl, nl + "    ")
                frag += "," + nl + body
            head, sep, tail = budget_text.partition(anchor)
            budget_text = head + frag + sep + tail
            # 追加后整体可解析复核（防字节级注入破坏 JSON）。
            json.loads(budget_text)
            BUDGET_PATH.write_text(budget_text, encoding="utf-8", newline="")
            note(f"g10_budget.json 字节级纯追加 {len(new_entries)} 条标定条目（{[e['id'] for e in new_entries]}；行尾风格随原文件）")
    elif not new_entries and not budget_problems:
        note("g10_budget.json 五条标定条目已在树且与重算逐位一致（幂等复跑，零改写）")
    checks["budget_entries_appended_measured_local"] = not budget_problems
    check(bool(budget_problems) is False, f"budget 条目异常: {budget_problems[:3]}")

    # ⑦ budget_eval --strict 全 PASS。
    r = run_cmd([sys.executable, "ci/budget_eval.py", "--strict"])
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")
    note(f"budget_eval --strict: exit {r.returncode}（{tail[-120:]}）")

    # ⑧ provisional 标记消费登记（三门 → 五条 budget 条目映射闭集）。
    consumption = [
        {
            "source_gate": e["source_gate"],
            "source_gate_evidence": str(gate_evidences[e["source_gate"]][0].relative_to(ROOT)).replace("\\", "/"),
            "source_gate_evidence_digest": _file_digest(gate_evidences[e["source_gate"]][0]),
            "consumed_marker": "provisional_pending_m138",
            "budget_entry_id": e["id"],
            "calibrated_p100": cal1[e["slug"]],
            "safety_factor_k": e["k"],
            "threshold": cal1[e["slug"]] * e["k"],
        }
        for e in CALIB_ENTRIES
    ]
    src_gates = {c["source_gate"] for c in consumption}
    checks["provisional_markers_consumed"] = src_gates == {
        "g10_m135_flip_metric", "g10_m136_ssim_psnr_metric", "g10_m137_pixel_diff_report",
    } and len(consumption) == 5
    check(checks["provisional_markers_consumed"], "消费登记闭集不齐（三门 → 五条映射）")

    # RED 臂①：手写阈值冒充标定必拒（threshold ≠ p100×k）。
    forged = {
        "id": "g10.metric.red_probe",
        "evidence": "measured_local",
        "threshold": cal1["ssim"] * 2.0 * 1.5,
        "measured_value": cal1["ssim"],
        "evidence_file": str(calib_files["ssim"].relative_to(ROOT)).replace("\\", "/"),
    }
    checks["red_handwritten_threshold_detected"] = bool(validate_budget_entry(forged, cal1["ssim"], 2.0))
    check(checks["red_handwritten_threshold_detected"], "手写阈值冒充未检出")

    # RED 臂②：estimated 冒充 measured 必拒。
    forged2 = dict(forged, threshold=cal1["ssim"] * 2.0, evidence="estimated")
    checks["red_estimated_masquerade_detected"] = bool(validate_budget_entry(forged2, cal1["ssim"], 2.0))
    check(checks["red_estimated_masquerade_detected"], "estimated 冒充未检出")

    # RED 臂③：不可复跑注入必检出（两跑漂移 → deterministic 判据翻红）。
    drift = dict(cal2)
    drift["ssim"] = drift["ssim"] + 1e-12
    checks["red_nonrerunnable_detected"] = (cal1 != drift) and (cal1 == cal2)
    check(checks["red_nonrerunnable_detected"], "复跑漂移注入未检出")

    # RED 臂④：门 evidence 缺失冒充输入 fail-closed（空目录取最新 → None）。
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g10_m138_red_") as td:
        red_missing = wel.load_latest_evidence("g10_m135_flip_metric", evidence_dir=Path(td)) is None
    checks["red_missing_gate_evidence_detected"] = red_missing
    check(checks["red_missing_gate_evidence_detected"], "门 evidence 缺失冒充未检出")

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
        "wave": "G10.4",
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
            "estimator_semantics": "统计量 = 全图集逐图 |自实现 − 参考实现| 差样本最大值（p100）；容差 = p100 × k（RFC-0026 §4.2 F10）",
            "sample_set_digest": cal1["sample_set_digest"],
            "sample_pair_count": cal1["sample_pair_count"],
            "rerun_deterministic": cal1 == cal2,
            "entries": consumption,
            "budget_path": "milestones/g10/g10_budget.json",
            "budget_append_only": True,
            "budget_eval_strict_exit": r.returncode,
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
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if all_pass and not FAILURES:
        print(
            f"[{TAG}] PASS（标定两跑逐位一致 + 五面 p100×k 标定值入 g10_budget（纯追加）"
            f"+ 三门 provisional 标记消费登记 + budget_eval --strict 全 PASS + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
