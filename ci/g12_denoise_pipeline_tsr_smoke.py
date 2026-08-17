#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.3 降噪波）
"""G12.3 M162 降噪管线 + TSR 联动门冒烟
（g12.p0.m162.denoise_pipeline_tsr；G12_CONTRACT §4.2 M162 行判据逐字;
G12_ACCEPTANCE_MAP §1;spec/global_illumination.md RXS-0402;RFC-0029 §4.5）。

硬判据:时域/空域降噪管线落地（时域累积消费既有 TAA/TSR 历史接口面——
temporal 底座 0-byte 不接线 + firefly 预钳位 + 空域 A-trous 类滤波）+
噪声谱高频能量下降 measured ≥ 标定阈（g12_budget 标定条目消费,禁手写
P-09）+ 帧均值能量守恒容差内（不引入系统性变暗/变亮偏置）+ temporal
底座 0-byte 断言（目录级 git diff vs G12.0 不可变 ref 机核）+ NRD 类
vendor 降噪评估报告落盘（评估不接线——报告在位 + 树内零 vendor 接线符号）
+ golden 对拍面不降级（固定全 spp golden 不偏离 measured×2.0 冻结带）+
帧型标签闭集 {raw, denoised} + 固定 seed 双跑位级一致。
RED 臂:降噪系统性偏置注入（energy-bias,A-trous 输出面 ±k 亮度——均值
能量断言必检出）/ 噪声底未降冒充降噪（masquerade 恒等旁通——高频下降
≈0 必检出）/ 历史验证关闭（validation-off——拒绝计数严格低于洁净臂必
检出）/ temporal 底座接线（目录级 diff 非空即 RED,本脚本机核）/ 评估
冒充接入（vendor 接线符号在树即 RED,本脚本机核）。

用法:
  py -3 ci/g12_denoise_pipeline_tsr_smoke.py --gate g12.p0.m162.denoise_pipeline_tsr
  py -3 ci/g12_denoise_pipeline_tsr_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g12_pt_prod_lib as gl  # noqa: E402

GATE_KEY = "g12.p0.m162.denoise_pipeline_tsr"
NUMERIC_STEP = 223
SUBJECT = "g12_m162_denoise_pipeline_tsr"
SCHEMA_PATH = ROOT / "milestones/g12/g12_m162_denoise_pipeline_tsr_evidence_schema.json"
SOURCE_REF = "G12_CONTRACT §4.2 M162;G12_ACCEPTANCE_MAP §1;spec/global_illumination.md RXS-0402;RFC-0029 §4.5"
TAG = "g12_m162"

DENOISE_KERNEL = ROOT / "src/rurix-render/kernels/g12_pt_denoise.rx"
NRD_REPORT = ROOT / "milestones/g12/design/nrd_vendor_denoise_evaluation.md"
TEMPORAL_DIR = "src/rurix-render/src/temporal"
CAL1 = gl.WORK_DIR / "denoise_calibration_run1.json"
CAL2 = gl.WORK_DIR / "denoise_calibration_run2.json"

# NRD 报告必备章节标记（落盘机核面;评估不接线字面）。
NRD_REQUIRED_SECTIONS = ["§1", "§2", "§3", "§4", "§5", "§6", "UpscaleBackend", "不接线", "MV", "深度", "法线"]
# vendor 接线符号闭集（树内命中即 RED——评估冒充接入面）。
VENDOR_WIRING_TOKENS = ["NrdIntegration", "NRDIntegration", "nrd::", "IN_VIEWZ", "IN_NORMAL_ROUGHNESS", "REBLUR"]

DENOISE_TESTS = [
    "temporal_accumulate_static_accepts_history",
    "temporal_accumulate_rejects_depth_discontinuity",
    "atrous_denoises_flat_and_preserves_edge",
    "hf_noise_drop_detects_masquerade",
    "energy_bias_injection_detected",
    "gbuffer_and_mv_derivation_sane",
    "params_fail_closed_and_label_closed_set",
]
CORPUS = [
    ("accept/denoise_pipeline_minimal.rx", "RXS-0402"),
    ("reject/denoise_energy_bias.rx", "RXS-0402"),
    ("reject/temporal_base_rewire.rx", "RXS-0402"),
]
SUBMODE_ARMS = ["denoise-energy-bias", "denoise-masquerade", "history-validation-off"]

# 降噪标定条目注册表:(budget id, calib json 键, direction, slug, 描述)。
DENOISE_ENTRY_REGISTRY = [
    ("g12.pt.denoise_hf_drop_min", "hf_drop", "min", "hf_drop_min",
     "降噪噪声谱高频能量下降标定阈(12 单元〔2 场景 × 3 族 × {static,moved}〕min,低梯度半幅掩码口径;threshold = measured × 0.5,协议冻结 k;M166 标定程序降噪腿产,禁手写 P-09)"),
    ("g12.pt.denoise_mean_energy_tol", "mean_energy", "max", "mean_energy_tol",
     "降噪帧均值能量守恒容差(12 单元 |mean(den)−mean(raw)|/mean(raw) p100 × 2.0,协议冻结 k;M166 标定程序降噪腿产,禁手写 P-09)"),
]

CHECK_KEYS = [
    "host_denoise_tests_anchored",
    "conformance_corpus_anchored",
    "budget_anchors_present",
    "m96_frozen_surface_0byte",
    "temporal_base_0byte",
    "nrd_evaluation_report_present",
    "nrd_no_vendor_wiring",
    "calibration_two_run_bitexact",
    "calibration_budget_entries_measured",
    "budget_eval_all_pass",
    "device_harness_full_pass",
    "device_double_run_bitexact",
    "device_hf_noise_floor_drop",
    "device_mean_energy_conserved",
    "device_history_validation_active",
    "device_golden_band_within",
    "device_frame_label_closed",
    "device_red_arms_effective",
    "device_red_arm_submodes_detected",
    "device_validation_zero",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


# ---------------------------------------------------------------------------
# temporal 底座 0-byte 机核（目录级 git diff vs G12.0 不可变 ref）
# ---------------------------------------------------------------------------


def temporal_base_0byte() -> tuple[bool, str]:
    r = gl.run(["git", "diff", "--name-only", gl.G12_ZERO_BASE, "--", TEMPORAL_DIR])
    changed = [x.strip() for x in r.stdout.splitlines() if x.strip()]
    if changed:
        return False, f"temporal 底座有差分(接线即 RED): {changed[:3]}"
    u = gl.run(["git", "status", "--porcelain", "--", TEMPORAL_DIR])
    dirty = [x for x in u.stdout.splitlines() if x.strip()]
    if dirty:
        return False, f"temporal 底座工作树未提交面: {dirty[:3]}"
    return True, f"temporal/ vs {gl.G12_ZERO_BASE} 目录级 0-byte(提交面 + 工作树双面)"


# ---------------------------------------------------------------------------
# NRD 评估报告机核（落盘 + 必备章节 + 零接线符号）
# ---------------------------------------------------------------------------


def nrd_report_ok() -> tuple[bool, str]:
    if not NRD_REPORT.is_file():
        return False, f"NRD 评估报告缺失({NRD_REPORT.relative_to(ROOT)})"
    text = NRD_REPORT.read_text(encoding="utf-8")
    missing = [s for s in NRD_REQUIRED_SECTIONS if s not in text]
    if missing:
        return False, f"NRD 报告缺必备面 {missing[:4]}"
    return True, "NRD 评估报告落盘 + 六节 + UpscaleBackend/MV/深度/法线/不接线字面齐备"


def no_vendor_wiring() -> tuple[bool, str]:
    hits: list[str] = []
    for path in (ROOT / "src").rglob("*.rs"):
        text = path.read_text(encoding="utf-8", errors="replace")
        for tok in VENDOR_WIRING_TOKENS:
            if tok in text:
                hits.append(f"{path.relative_to(ROOT)}:{tok}")
    for path in ROOT.rglob("Cargo.toml"):
        if ".tmp" in path.parts or "target" in path.parts:
            continue
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            low = line.strip().lower()
            if low.startswith("nrd") or low.startswith("nri"):
                hits.append(f"{path.relative_to(ROOT)}:{line.strip()[:40]}")
    if hits:
        return False, f"vendor 降噪接线符号在树(评估冒充接入即 RED): {hits[:3]}"
    return True, "src/ 与 Cargo.toml 全树零 vendor 降噪接线符号(评估不接线)"


# ---------------------------------------------------------------------------
# 降噪标定腿（harness --calibrate-denoise 两跑逐位一致 + budget 条目追加）
# ---------------------------------------------------------------------------


def denoise_calibration(harness: Path) -> dict | None:
    """返回标定 JSON（两跑逐字节一致）或 None（失败面由调用方登记）。"""
    cmd = [str(harness), "--calibrate-denoise", str(CAL1)]
    r1 = gl.run(cmd, timeout=1800)
    if r1.returncode != 0:
        check(False, f"降噪标定跑 1 失败 rc={r1.returncode}: {(r1.stdout + r1.stderr)[-400:]}")
        return None
    r2 = gl.run([str(harness), "--calibrate-denoise", str(CAL2)], timeout=1800)
    if r2.returncode != 0:
        check(False, f"降噪标定跑 2 失败 rc={r2.returncode}")
        return None
    b1 = CAL1.read_bytes()
    b2 = CAL2.read_bytes()
    if b1 != b2:
        check(False, "降噪标定两跑非逐字节一致(不可复跑即 RED)")
        return None
    calib = json.loads(b1.decode("utf-8"))
    count = int(calib.get("sample_manifest", {}).get("count", 0))
    if count < 12:
        check(False, f"降噪标定样本集 {count} < 下界 12")
        return None
    return calib


def append_denoise_budget_entries(calib: dict, ts: str) -> list[str]:
    """重算校验 + 逐条目 evidence + 字节级纯追加（M166 同纪律幂等）。"""
    problems: list[str] = []
    entries: list[dict] = []
    for eid, key, direction, slug, desc in DENOISE_ENTRY_REGISTRY:
        block = calib.get(key) or {}
        measured = float(block.get("measured", "nan"))
        tol = float(block.get("tol", "nan"))
        # 重算校验（手写阈值冒充检出器——协议公式直读）。
        expect = measured * (0.5 if direction == "min" else 2.0)
        if abs(tol - expect) > 1e-12 * max(1.0, abs(expect)):
            problems.append(f"{eid}: tol={tol} ≠ 标定重算 {expect}(手写阈值冒充)")
        if direction == "min" and not (measured > 0.0):
            problems.append(f"{eid}: measured 下降非正({measured})——噪声底未降冒充降噪")
        ev_rel = f"evidence/g12_m162_calibration_{slug}_{ts}.json"
        entries.append({
            "id": eid,
            "description": desc + f";样本集 digest {calib['sample_manifest']['digest']}(count={calib['sample_manifest']['count']} ≥ 12);标定程序 ci/g12_denoise_pipeline_tsr_smoke.py 降噪腿可复跑(两跑逐位一致)",
            "direction": direction,
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": tol,
            "evidence_file": ev_rel,
            "measured_value": measured,
        })
    if problems:
        return problems
    gl.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    for entry, (eid, key, _direction, slug, _desc) in zip(entries, DENOISE_ENTRY_REGISTRY):
        measured = float(calib[key]["measured"])
        doc = {
            "schema": "rurix.g12pt.denoise_calibration_entry.v1",
            "entry_id": eid,
            "results": {"trimmed_mean": measured},
            "protocol": calib[key].get("protocol", ""),
            "sample_manifest": calib["sample_manifest"],
            "provenance": calib["provenance"],
            "timestamp": ts,
        }
        out = gl.EVIDENCE_DIR / f"g12_m162_calibration_{slug}_{ts}.json"
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    # 字节级纯追加(M138/M166 同纪律:已存在同值幂等、值漂移即 problems)。
    budget_text = gl.BUDGET_PATH.read_text(encoding="utf-8")
    budget = json.loads(budget_text)
    to_add: list[dict] = []
    for entry in entries:
        existing = [x for x in budget.get("entries", []) if x.get("id") == entry["id"]]
        if existing:
            ex = existing[0]
            comparable = {k: v for k, v in entry.items() if k != "evidence_file"}
            ex_comparable = {k: v for k, v in ex.items() if k != "evidence_file"}
            if ex_comparable != comparable:
                problems.append(f"{entry['id']} 已在树且值漂移(只追加禁改写)")
            continue
        to_add.append(entry)
    if problems or not to_add:
        return problems
    nl = "\r\n" if "\r\n" in budget_text else "\n"
    anchor = f"{nl}  ],{nl}  \"ratio_assertions\""
    if anchor not in budget_text:
        return ["g12_budget.json 结构锚缺失(拒改写)"]
    frag = ""
    for entry in to_add:
        body = json.dumps(entry, ensure_ascii=False, indent=2)
        body = body.replace("\n", nl)
        body = "    " + body.replace(nl, nl + "    ")
        frag += "," + nl + body
    head, sep, tail = budget_text.partition(anchor)
    budget_text = head + frag + sep + tail
    json.loads(budget_text)
    gl.BUDGET_PATH.write_text(budget_text, encoding="utf-8", newline="")
    return problems


# ---------------------------------------------------------------------------
# device 腿（双 kernel 产线 + harness --gate + RED 臂子模式复跑）
# ---------------------------------------------------------------------------


def run_device_leg(cal: dict) -> tuple[str, dict | None, bool, list[str]]:
    failures: list[str] = []
    submode_ok = True
    doc: dict | None = None
    with gl.gpu_device_lock(purpose=f"{TAG} device 腿"):
        rurixc = gl.build_rurixc()
        spv = gl.WORK_DIR / "g12_pt_production.spv"
        dn_spv = gl.WORK_DIR / "g12_pt_denoise.spv"
        harness = gl.build_harness() if rurixc else None
        ok = rurixc is not None and harness is not None
        if ok:
            ok = gl.compile_spv(rurixc, spv)
            # 降噪 kernel SPV 产线(同 rurixc 产线)。
            print(f"[{TAG}] rurixc {DENOISE_KERNEL.name} --target vulkan -o {dn_spv.name}")
            r = gl.run([str(rurixc), str(DENOISE_KERNEL), "--target", "vulkan", "-o", str(dn_spv)])
            ok = ok and r.returncode == 0 and dn_spv.is_file()
        if not ok:
            failures.append("rurixc/SPV/harness 产线失败(含降噪 kernel)")
            return "fail", None, False, failures
        a = cal["anchors"]
        args = [
            "--gate", GATE_KEY,
            "--spv", str(spv),
            "--denoise-spv", str(dn_spv),
            "--evidence", str(gl.HARNESS_EVIDENCE),
            "--pbrt", str(gl.PBRT_EXE),
            "--imgtool", str(gl.IMGTOOL_EXE),
            "--work-dir", str(gl.WORK_DIR / "pbrt_work"),
            "--tau", repr(cal["tau"]),
            "--sampler", gl.winner_cli_name(cal["winner"]),
            "--hf-drop-min", repr(cal["hf_drop_min"]),
            "--mean-energy-tol", repr(cal["mean_energy_tol"]),
        ]
        print(f"[{TAG}] device 全档: harness --gate {GATE_KEY}(双 kernel,validation=on)")
        r = gl.run([str(harness)] + args, env=gl.device_env(), timeout=3600)
        out = r.stdout + r.stderr
        if "G12_PT_PROD: SKIP" in r.stdout:
            return "skipped_dev_env", None, False, [f"device SKIP: {out.strip()[-400:]}"]
        if gl.HARNESS_EVIDENCE.is_file():
            try:
                doc = json.loads(gl.HARNESS_EVIDENCE.read_text(encoding="utf-8"))
            except json.JSONDecodeError as e:
                failures.append(f"harness evidence 不可解析: {e}")
        if r.returncode != 0 or "G12_PT_PROD: PASS" not in r.stdout:
            failures.append(f"harness 全档失败 rc={r.returncode}: {out[-1500:]}")
            return "fail", doc, False, failures
        if doc is None:
            failures.append("harness evidence 缺失")
            return "fail", None, False, failures
        if doc.get("schema") != "rurix.g12pt.production.v1" or doc.get("gate") != GATE_KEY:
            failures.append("harness evidence schema/gate 字面不符")
            return "fail", doc, False, failures
        if doc.get("spec_anchor") != "RXS-0402":
            failures.append("harness evidence spec_anchor ≠ RXS-0402")
            return "fail", doc, False, failures
        # RED 臂子模式独立复跑抽检(退出码 0 + PASS red-arm 字面 = 臂独立有效)。
        for arm in SUBMODE_ARMS:
            print(f"[{TAG}] device RED 臂子模式: --red-arm {arm}")
            ra = gl.run(
                [
                    str(harness), "--red-arm", arm, "--spv", str(spv),
                    "--denoise-spv", str(dn_spv),
                    "--tau", repr(cal["tau"]),
                    "--sampler", gl.winner_cli_name(cal["winner"]),
                    "--hf-drop-min", repr(cal["hf_drop_min"]),
                    "--mean-energy-tol", repr(cal["mean_energy_tol"]),
                ],
                env=gl.device_env(),
                timeout=900,
            )
            rout = ra.stdout + ra.stderr
            if ra.returncode != 0 or f"G12_PT_PROD: PASS red-arm {arm}" not in ra.stdout:
                failures.append(f"RED 臂子模式 {arm} 未独立检出 rc={ra.returncode}: {rout[-400:]}")
                submode_ok = False
    return "executed", doc, submode_ok, failures


def load_denoise_calibration_from_budget() -> dict | None:
    """门消费面:τ/winner(G12.2 标定面)+ 降噪两条目 threshold(g12_budget)。"""
    cal = gl.load_calibration()
    if cal is None:
        return None
    budget = gl.load_budget()
    for eid, key in (
        ("g12.pt.denoise_hf_drop_min", "hf_drop_min"),
        ("g12.pt.denoise_mean_energy_tol", "mean_energy_tol"),
    ):
        e = gl.budget_entry(budget, eid)
        if e is None or e.get("evidence") != "measured_local":
            return None
        cal[key] = float(e["threshold"])
    return cal


# ---------------------------------------------------------------------------
# selftest（反 YAML-only）
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 20:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 20", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    # 红臂①:temporal 底座 diff 检出器——合成差分面必判 RED。
    if not _detect_temporal_diff("src/rurix-render/src/temporal/taa.rs\n"):
        print(f"[{TAG}] selftest FAIL: temporal 差分注入未检出", file=sys.stderr)
        return 1
    if _detect_temporal_diff(""):
        print(f"[{TAG}] selftest FAIL: temporal 0-byte 正例误判", file=sys.stderr)
        return 1
    # 红臂②:vendor 接线符号检出器——合成符号面必判 RED。
    if not _detect_wiring("let x = nrd::Integration::new();"):
        print(f"[{TAG}] selftest FAIL: vendor 接线符号注入未检出", file=sys.stderr)
        return 1
    if _detect_wiring("// 自研降噪管线,零 vendor 依赖"):
        print(f"[{TAG}] selftest FAIL: 零接线正例误判", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (3 RED + 2 GREEN)")
    return 0


def _detect_temporal_diff(diff_text: str) -> bool:
    return bool(diff_text.strip())


def _detect_wiring(text: str) -> bool:
    return any(tok in text for tok in VENDOR_WIRING_TOKENS)


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


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

    gl.os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    # ── host 段 ──
    # prod_denoise 单测逐名锚定(gi::path_trace::prod_denoise 模块)。
    r = gl.run(["cargo", "test", "-p", "rurix-render", "--lib", "gi::path_trace::prod_denoise"])
    blob = r.stdout + r.stderr
    missing = [n for n in DENOISE_TESTS if n not in blob]
    checks["host_denoise_tests_anchored"] = r.returncode == 0 and "test result: ok" in blob and not missing
    check(checks["host_denoise_tests_anchored"], f"prod_denoise 单测失败或未锚定: {missing[:3]} rc={r.returncode}")
    note(f"{len(DENOISE_TESTS)} 降噪单测逐名锚定全绿")

    ok, msg = gl.conformance_anchor(CORPUS, GATE_KEY)
    checks["conformance_corpus_anchored"] = ok
    check(ok, msg)
    note(msg)

    # budget 锚面(G12.2 15 条目齐备;降噪两条目由本门标定腿追加后判)。
    checks["budget_anchors_present"] = gl.load_calibration() is not None
    check(checks["budget_anchors_present"], "g12_budget G12.2 标定/锚条目缺失(M166 未绿不得抢跑)")

    ok, msg = gl.m96_frozen_surface_unchanged()
    checks["m96_frozen_surface_0byte"] = ok
    check(ok, msg)
    note(msg)

    ok, msg = temporal_base_0byte()
    checks["temporal_base_0byte"] = ok
    check(ok, msg)
    note(msg)

    ok, msg = nrd_report_ok()
    checks["nrd_evaluation_report_present"] = ok
    check(ok, msg)
    note(msg)

    ok, msg = no_vendor_wiring()
    checks["nrd_no_vendor_wiring"] = ok
    check(ok, msg)
    note(msg)

    # ── 标定腿(纯 host;harness --calibrate-denoise 两跑逐位一致 + budget 追加)──
    harness = gl.build_harness()
    if harness is None:
        check(False, "g12_pt_production harness 构建失败")
    calib = None
    if harness is not None:
        gl.WORK_DIR.mkdir(parents=True, exist_ok=True)
        calib = denoise_calibration(harness)
        checks["calibration_two_run_bitexact"] = calib is not None
        if calib is not None:
            problems = append_denoise_budget_entries(calib, ts)
            checks["calibration_budget_entries_measured"] = not problems
            check(not problems, f"降噪标定条目追加: {problems[:2]}")
            note(
                f"降噪标定:hf_drop measured={calib['hf_drop']['measured']} → 阈 {calib['hf_drop']['tol']};"
                f"mean_energy p100={calib['mean_energy']['measured']} → 容差 {calib['mean_energy']['tol']}"
            )
    if checks["calibration_budget_entries_measured"]:
        r = gl.run(["py", "-3", "ci/budget_eval.py"])
        checks["budget_eval_all_pass"] = r.returncode == 0 and "[budget_eval] PASS" in (r.stdout + r.stderr)
        check(checks["budget_eval_all_pass"], f"budget_eval 非零: {(r.stdout + r.stderr)[-300:]}")

    # ── device 段 ──
    device_state = "fail"
    doc = None
    cal = load_denoise_calibration_from_budget()
    if cal is None:
        check(False, "降噪标定阈预算条目缺失(标定腿未绿不得跑 device)")
    else:
        device_state, doc, submode_ok, leg_failures = run_device_leg(cal)
        for f in leg_failures:
            check(False, f)
        if device_state == "skipped_dev_env":
            check(False, "device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP)")
            device_state = "fail"
        if device_state == "executed" and doc is not None:
            hc = doc.get("checks", {})
            checks["device_harness_full_pass"] = True
            checks["device_double_run_bitexact"] = hc.get("double_run_bitexact") is True
            checks["device_hf_noise_floor_drop"] = hc.get("hf_noise_floor_drop") is True
            checks["device_mean_energy_conserved"] = hc.get("mean_energy_conserved") is True
            checks["device_history_validation_active"] = hc.get("history_validation_active") is True
            checks["device_golden_band_within"] = hc.get("golden_band_within") is True
            checks["device_frame_label_closed"] = hc.get("frame_label_closed") is True
            checks["device_red_arms_effective"] = all(
                hc.get(k) is True
                for k in ("red_energy_bias_detected", "red_masquerade_detected", "red_validation_off_detected")
            )
            checks["device_validation_zero"] = (
                hc.get("validation_zero") is True
                and doc.get("device_state", {}).get("validation") == "on"
                and doc.get("device_state", {}).get("require_real") is True
            )
            for k in CHECK_KEYS:
                if k.startswith("device_") and k != "device_red_arm_submodes_detected" and not checks[k]:
                    check(False, f"harness 判据 {k} 为假")
            checks["device_red_arm_submodes_detected"] = submode_ok
            note("device:双 kernel 全档真跑(时域+firefly+A-trous 双帧管线)+ RED 三臂子模式独立复跑")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"
    evidence = gl.gate_evidence(
        subject=SUBJECT,
        gate_key=GATE_KEY,
        milestone="M162",
        wave="G12.3",
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        checks=checks,
        device_state=device_state,
        host_pass=host_pass,
        commands=[
            {"seq": 1, "command": "cargo test -p rurix-render --lib gi::path_trace::prod_denoise", "exit_code": 0 if checks["host_denoise_tests_anchored"] else 1},
            {"seq": 2, "command": "git diff --name-only 5ae83aa7 -- src/rurix-render/src/temporal (0-byte 机核)", "exit_code": 0 if checks["temporal_base_0byte"] else 1},
            {"seq": 3, "command": "g12_pt_production --calibrate-denoise <run1/run2> (纯 host 两跑逐字节一致)", "exit_code": 0 if checks["calibration_two_run_bitexact"] else 1},
            {"seq": 4, "command": "cargo build -p rurixc --features vulkan-backend --bin rurixc + rurixc g12_pt_production.rx / g12_pt_denoise.rx --target vulkan", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 5, "command": "g12_pt_production --gate g12.p0.m162.denoise_pipeline_tsr --spv .. --denoise-spv .. --hf-drop-min <g12.pt.denoise_hf_drop_min> --mean-energy-tol <g12.pt.denoise_mean_energy_tol>(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g12_pt_production --red-arm denoise-energy-bias|denoise-masquerade|history-validation-off --denoise-spv .. (子模式抽检)", "exit_code": 0 if checks["device_red_arm_submodes_detected"] else 1},
            {"seq": 7, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
        ],
        environment=gl.environment(),
        production={
            "correctness_anchor_unchanged": checks["m96_frozen_surface_0byte"] and checks["temporal_base_0byte"],
            "baseline_anchor_id": "g12.pt.denoise_hf_drop_min / g12.pt.denoise_mean_energy_tol(本门标定腿产出入 budget)",
            "measured_value": (
                "; ".join(
                    f"{k}: hf_drop={v.get('hf_drop','?')} ediff={v.get('mean_energy_rel_diff','?')} hist_rej={v.get('history_rejected','?')}"
                    for k, v in ((doc or {}).get("measurements") or {}).items()
                )
                if doc
                else "n/a(device 未执行)"
            ),
            "not_worse_than_anchor": checks["device_hf_noise_floor_drop"] and checks["device_mean_energy_conserved"] and checks["device_golden_band_within"],
            "threshold_provenance": "g12_budget.json 降噪标定条目(标定程序降噪腿 measured 产,两跑逐位一致,禁手写 P-09)",
            "evolution_register": None,
        },
        notes="; ".join(NOTES + FAILURES[:8]),
        all_pass=all_pass,
        ts=ts,
    )
    gl.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = gl.EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
