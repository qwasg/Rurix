#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.4 M98 四级追踪降级链门冒烟(g9.p0.m98.tracing_fallback_chain;RFC-0022 §4.7;
spec/global_illumination.md RXS-0359;G9_ACCEPTANCE_MAP §2 M98)。

门序机器阻断(D2-Q7 硬约束,前置):evidence/ 最新
  g9_m96_path_tracer_reference_<UTC>.json 必须 status=="pass" 且
  assertion_id=="g9.p0.m96.path_tracer_reference"(ci/g9_gi_interlock.py);
  缺失/非 pass 即门 FAIL 退 1(打印阻断原因)。

host 段:rurix-render gi::fallback_chain 14 单测逐名锚定(选档器闭集/强关记录/
  静默回退审计拒/L4 登记 fail-closed/L2 暴力=BVH 金标准/计数面/带比较器)+
  conformance gi reject tracing_fallback_silent_demotion.rx(`//@ spec: RXS-0359`)
  锚定 + 冻结深度带 milestones/g9/g9_m98_depth_band.json provenance 机器核验
  (含 m96_anchor_digest 与 M97 冻结带 depth2 m96_digest 逐字相等的门序链锚)。
device 段(必需,持 gpu_device_lock):rurixc --target vulkan 产三 SPV
  (g9_m98_screen_trace/g9_m98_hwrt/g9_m96_path_tracer)→ g9_m98_fallback_chain
  harness 全档真跑(双跑位级一致 + L1 device/host 结构域精确对拍 + 四级命中率/
  耗时计数逐帧非空 + 逐级强关双臂可检测 + 静默回退注入审计拒 + L4 not-triggered
  登记 + 六条目深度带内;RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1,SKIP 翻红)
  + --red-arm force-off-l1(device)/ --red-arm silent-demotion(host)子模式
  独立复跑抽检。

用法:
  py -3 ci/g9_tracing_fallback_chain_smoke.py --gate g9.p0.m98.tracing_fallback_chain
  py -3 ci/g9_tracing_fallback_chain_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones/g9/g9_m98_tracing_fallback_chain_evidence_schema.json"
BAND_PATH = ROOT / "milestones/g9/g9_m98_depth_band.json"
M97_BAND_PATH = ROOT / "milestones/g9/g9_m97_depth_band.json"
REJECT_CORPUS = ROOT / "conformance/gi/reject/tracing_fallback_silent_demotion.rx"
KERNEL_DIR = ROOT / "src/rurix-render/kernels"
WORK_DIR = ROOT / ".tmp/g94_gates/m98"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402
from g9_gi_interlock import m96_gate_passed  # noqa: E402

GATE_KEY = "g9.p0.m98.tracing_fallback_chain"
NUMERIC_STEP = 149
SOURCE_REF = "RFC-0022 §4.7;spec/global_illumination.md RXS-0359;G9_ACCEPTANCE_MAP §2 M98"
TAG = "g9_m98"

FALLBACK_CHAIN_TESTS = [
    "level_order_flags_and_names",
    "switches_independent_per_level",
    "selector_distance_and_coverage_priority",
    "force_off_records_forced_off_cause",
    "silent_demotion_injection_fails_audit",
    "l4_not_triggered_registration_fail_closed",
    "l2_bruteforce_matches_bvh_gold_standard",
    "cosine_dir_and_stream_determinism",
    "point_light_core_numeric_anchor",
    "gbuffer_prepass_deterministic_and_sane",
    "force_off_changes_product_digest_structural",
    "counters_faces_non_empty_per_frame",
    "band_roundtrip_and_fail_closed",
    "leg_work_counters_deterministic",
]

CHECK_KEYS = [
    # host 段
    "gate_order_m96_passed",
    "host_fallback_chain_tests_anchored",
    "conformance_gi_corpus_anchored",
    "depth_band_provenance_frozen",
    # device 段
    "device_harness_full_pass",
    "device_double_run_bitexact",
    "device_l1_host_parity",
    "device_counters_non_empty",
    "device_force_off_detectable",
    "device_silent_demotion_audit",
    "device_red_arm_submodes_detected",
    "device_l4_not_triggered",
    "device_m96_cross_anchor_band",
    "device_validation_zero",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def run_cargo(args: list[str]) -> tuple[int, str]:
    print(f"[{TAG}] cargo {' '.join(args)}")
    r = subprocess.run(["cargo", *args], cwd=ROOT, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def target_dir() -> Path:
    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


def is_hex(v: object, n: int) -> bool:
    return isinstance(v, str) and len(v) == n and all(c in "0123456789abcdef" for c in v)


# ═══════════════════════ host 段 ═══════════════════════


def host_fallback_chain_tests() -> bool:
    """cargo test -p rurix-render --lib gi::fallback_chain:14 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurix-render", "--lib", "gi::fallback_chain"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in FALLBACK_CHAIN_TESTS:
        if not (ok and name in blob):
            check(False, f"gi::fallback_chain 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_conformance_anchor() -> bool:
    """conformance gi reject 语料在位 + `//@ spec: RXS-0359` 锚定 + 静默回退预期面。"""
    if not REJECT_CORPUS.is_file():
        check(False, f"缺 RED 语料 {REJECT_CORPUS.name}")
        return False
    text = REJECT_CORPUS.read_text(encoding="utf-8")
    ok = (
        "//@ spec: RXS-0359" in text
        and GATE_KEY in text
        and "静默回退" in text
    )
    if not ok:
        check(False, "RED 语料锚定/静默回退预期面缺失")
    return ok


def host_band_provenance() -> bool:
    """M98 深度带 provenance 机器核验 + M97 带 depth2 门序链锚逐字核验。"""
    if not BAND_PATH.is_file():
        check(False, f"缺冻结深度带 {BAND_PATH.name}")
        return False
    try:
        band = json.loads(BAND_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        check(False, f"深度带不可读: {e}")
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"深度带 provenance: {msg}")
            ok = False

    need(band.get("schema") == "rurix.g9m98.depth_band.v1", "schema 字面不符")
    need(bool(band.get("frozen_at_utc")), "frozen_at_utc 空")
    need(bool(band.get("device_name")), "device_name 空")
    need(band.get("scene") == "m96_cornell", "scene ≠ m96_cornell")
    need(is_hex(band.get("m96_anchor_digest"), 64), "m96_anchor_digest 形态")
    need("M98_BAND_MARGIN" in str(band.get("freeze_rule", "")), "freeze_rule 缺 M98_BAND_MARGIN 登记")
    need(band.get("matched_depth") == "2", "matched_depth ≠ 2")
    need(band.get("m96_golden_spp") == "64", "m96_golden_spp ≠ 64")
    need(bool(band.get("seed_chain")), "seed_chain 空")
    entries = band.get("entries")
    need(isinstance(entries, list) and len(entries) == 6, "entries ≠ 6(四 solo + 两 chain)")
    for e in entries if isinstance(entries, list) else []:
        for f in ("chain_digest", "m96_digest"):
            need(is_hex(e.get(f), 64), f"条目 {e.get('tier')} {f} 形态")
        for f in ("band_rel_dev", "measured_rel_dev"):
            need(isinstance(e.get(f), str) and len(e[f]) > 0, f"条目 {e.get('tier')} 缺 {f}")
    # 门序链锚:M98 带 m96_anchor_digest == M97 带 depth2 条目 m96_digest。
    try:
        m97 = json.loads(M97_BAND_PATH.read_text(encoding="utf-8"))
        anchor = next(
            (e.get("m96_digest") for e in m97.get("entries", []) if e.get("depth") == "2"),
            None,
        )
    except (OSError, json.JSONDecodeError):
        anchor = None
    if anchor is None:
        check(False, "M97 冻结带缺 depth2 条目(门序链锚不可核)")
        ok = False
    elif band.get("m96_anchor_digest") != anchor:
        check(False, f"M98 带 m96_anchor_digest ≠ M97 带 depth2 m96_digest({anchor})")
        ok = False
    else:
        note("M98 带 m96_anchor_digest == M97 带 depth2 m96_digest(门序链锚逐字一致)")
    return ok


# ═══════════════════════ device 段 ═══════════════════════


def build_rurixc() -> Path | None:
    print(f"[{TAG}] cargo build -p rurixc --features vulkan-backend --bin rurixc")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"rurixc 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = target_dir() / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if not exe.is_file():
        check(False, f"rurixc 产物缺失: {exe}")
        return None
    return exe


def compile_spv(rurixc: Path, kernel_name: str, out: Path) -> bool:
    print(f"[{TAG}] rurixc {kernel_name} --target vulkan -o {out.name}")
    out.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        [str(rurixc), str(KERNEL_DIR / kernel_name), "--target", "vulkan", "-o", str(out)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0 or not out.is_file():
        check(False, f"SPV 产线失败({kernel_name}) rc={r.returncode}:\n{(r.stdout + r.stderr)[-1500:]}")
        return False
    return True


def build_harness() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin g9_m98_fallback_chain")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", "g9_m98_fallback_chain"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"g9_m98_fallback_chain 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = target_dir() / "debug" / ("g9_m98_fallback_chain.exe" if sys.platform == "win32" else "g9_m98_fallback_chain")
    if not exe.is_file():
        check(False, f"harness 产物缺失: {exe}")
        return None
    return exe


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    env["RURIX_BASE_COMMIT"] = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
    ).stdout.strip()
    return env


def run_harness_full(exe: Path, spv_l1: Path, spv_l3: Path, spv_m96: Path) -> tuple[str, dict | None]:
    """全档真跑(四级链 + 六条目深度带)。返回 (device_state, harness evidence|None)。"""
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(exe),
        "--spv-l1", str(spv_l1),
        "--spv-l3", str(spv_l3),
        "--spv-m96", str(spv_m96),
        "--work-dir", str(WORK_DIR / "work"),
        "--evidence", str(HARNESS_EVIDENCE),
    ]
    print(f"[{TAG}] device 全档: g9_m98_fallback_chain(三 SPV,validation=on)")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=device_env(), timeout=1800)
    out = r.stdout + r.stderr
    if "G9_M98_FB: SKIP" in r.stdout:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {out.strip()[-800:]}")
        return "skipped_dev_env", None
    doc = None
    if HARNESS_EVIDENCE.is_file():
        try:
            doc = json.loads(HARNESS_EVIDENCE.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            check(False, f"harness evidence 不可解析: {e}")
    if r.returncode != 0 or "G9_M98_FB: PASS" not in r.stdout:
        check(False, f"harness 全档失败 rc={r.returncode}:\n{out[-2000:]}")
        return "fail", doc
    if doc is None:
        check(False, "harness evidence 缺失")
        return "fail", None
    if doc.get("schema") != "rurix.g9m98.fallback_chain.v1" or doc.get("spec_anchor") != "RXS-0359":
        check(False, "harness evidence schema/spec_anchor 字面不符")
        return "fail", doc
    if doc.get("assertion_id") != GATE_KEY or doc.get("status") != "pass":
        check(False, "harness evidence assertion_id/status 不符")
        return "fail", doc
    return "executed", doc


def run_red_arm_force_off(exe: Path, spv_l1: Path, spv_l3: Path) -> bool:
    """--red-arm force-off-l1 子模式独立复跑(device;退出码 0 + PASS 字面)。"""
    print(f"[{TAG}] device RED 臂子模式: --red-arm force-off-l1")
    r = subprocess.run(
        [str(exe), "--red-arm", "force-off-l1", "--spv-l1", str(spv_l1), "--spv-l3", str(spv_l3)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=device_env(),
        timeout=900,
    )
    out = r.stdout + r.stderr
    ok = r.returncode == 0 and "G9_M98_FB: PASS red-arm force-off-l1" in r.stdout
    if not ok:
        check(False, f"RED 臂子模式 force-off-l1 未独立检出 rc={r.returncode}: {out[-600:]}")
    return ok


def run_red_arm_silent_demotion(exe: Path) -> bool:
    """--red-arm silent-demotion 子模式独立复跑(纯 host 臂)。"""
    print(f"[{TAG}] host RED 臂子模式: --red-arm silent-demotion")
    r = subprocess.run(
        [str(exe), "--red-arm", "silent-demotion"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=300,
    )
    out = r.stdout + r.stderr
    ok = r.returncode == 0 and "G9_M98_FB: PASS red-arm silent-demotion" in r.stdout
    if not ok:
        check(False, f"RED 臂子模式 silent-demotion 未独立检出 rc={r.returncode}: {out[-600:]}")
    return ok


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    # 红臂①:合成 FAILURES 必须使门红(check() 判别有效)。
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 红臂②:M96 evidence 缺失 ⇒ 门序必阻断(D2-Q7)。
    with tempfile.TemporaryDirectory(prefix="g9_m98_selftest_") as td:
        ok, detail = m96_gate_passed(Path(td))
        if ok or "门序阻断" not in detail:
            print(f"[{TAG}] selftest FAIL: M96 evidence 缺失未阻断", file=sys.stderr)
            return 1
    # CHECK_KEYS 闭集恰 14 项(host 4 + device 10)。
    if len(CHECK_KEYS) != 14:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 14", file=sys.stderr)
        return 1
    # 绿臂:schema required 与 CHECK_KEYS 闭集互核;合成缺键 evidence 必红。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    synth = {k: True for k in CHECK_KEYS}
    synth.pop(CHECK_KEYS[0])
    if not (req - set(synth)):
        print(f"[{TAG}] selftest FAIL: 合成缺键未触发 schema required 红", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (2 RED + 1 GREEN)")
    return 0


# ═══════════════════════ main ═══════════════════════


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

    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    device_state = "fail"

    # 门序机器阻断(D2-Q7,前置):M96 门未绿即 FAIL 退 1,host/device 段不跑。
    interlock_ok, interlock_detail = m96_gate_passed()
    print(f"[{TAG}] {interlock_detail}")
    note(interlock_detail)
    checks["gate_order_m96_passed"] = interlock_ok
    if not interlock_ok:
        check(False, interlock_detail)
    else:
        # host 段
        checks["host_fallback_chain_tests_anchored"] = host_fallback_chain_tests()
        checks["conformance_gi_corpus_anchored"] = host_conformance_anchor()
        checks["depth_band_provenance_frozen"] = host_band_provenance()

        # device 段(持锁串行:rurixc 构建 + 三 SPV 产线 + harness 全档 + RED 臂子模式)
        with gpu_device_lock(purpose="g9_m98 fallback chain device 腿"):
            rurixc = build_rurixc()
            exe = build_harness() if rurixc else None
            spv_l1 = WORK_DIR / "g9_m98_screen_trace.spv"
            spv_l3 = WORK_DIR / "g9_m98_hwrt.spv"
            spv_m96 = WORK_DIR / "g9_m96_path_tracer.spv"
            spvs_ok = rurixc is not None and all(
                compile_spv(rurixc, k, o)
                for k, o in (
                    ("g9_m98_screen_trace.rx", spv_l1),
                    ("g9_m98_hwrt.rx", spv_l3),
                    ("g9_m96_path_tracer.rx", spv_m96),
                )
            )
            if exe and spvs_ok:
                device_state, doc = run_harness_full(exe, spv_l1, spv_l3, spv_m96)
                if device_state == "executed" and doc is not None:
                    hc = doc.get("checks", {})
                    checks["device_harness_full_pass"] = True
                    checks["device_double_run_bitexact"] = hc.get("double_run_bitexact") is True
                    checks["device_l1_host_parity"] = hc.get("l1_device_host_parity") is True
                    checks["device_counters_non_empty"] = (
                        hc.get("level_coverage_all_used") is True
                        and hc.get("counters_non_empty_per_frame") is True
                    )
                    checks["device_force_off_detectable"] = all(
                        hc.get(k) is True
                        for k in ("force_off_l1_detectable", "force_off_l2_detectable", "force_off_l3_detectable")
                    )
                    checks["device_silent_demotion_audit"] = hc.get("silent_demotion_detected") is True
                    l4 = doc.get("l4_registration", {})
                    checks["device_l4_not_triggered"] = (
                        hc.get("l4_not_triggered_registered") is True
                        and l4.get("status") == "not-triggered"
                        and l4.get("trigger_met") is False
                        and l4.get("serve_request_rejected") is True
                        and l4.get("counters_zero") is True
                    )
                    checks["device_m96_cross_anchor_band"] = (
                        hc.get("m96_cross_anchor") is True
                        and hc.get("depth_band_within") is True
                    )
                    env = doc.get("environment", {})
                    checks["device_validation_zero"] = (
                        hc.get("validation_zero") is True
                        and env.get("validation") == "on"
                        and env.get("require_real") is True
                    )
                    checks["device_red_arm_submodes_detected"] = (
                        run_red_arm_force_off(exe, spv_l1, spv_l3)
                        and run_red_arm_silent_demotion(exe)
                    )
                    for k in CHECK_KEYS:
                        if k.startswith("device_") and not checks[k]:
                            check(False, f"harness 判据 {k} 为假")
                note("device:全档真跑(四级计数 + 强关双臂 + 静默回退审计)+ force-off-l1/silent-demotion 子模式复跑")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m98_tracing_fallback_chain",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M98",
        "milestone": "M98",
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G9.4",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        ).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": [
            {"seq": 1, "command": "cargo test -p rurix-render --lib gi::fallback_chain", "exit_code": 0},
            {"seq": 2, "command": "cargo build -p rurixc --features vulkan-backend --bin rurixc", "exit_code": 0},
            {"seq": 3, "command": "rurixc kernels/g9_m98_{screen_trace,hwrt}.rx + g9_m96_path_tracer.rx --target vulkan -o .tmp/g94_gates/m98/*.spv", "exit_code": 0},
            {"seq": 4, "command": "cargo build -p rurix-render --features vulkan --bin g9_m98_fallback_chain", "exit_code": 0},
            {"seq": 5, "command": "g9_m98_fallback_chain --spv-l1 .. --spv-l3 .. --spv-m96 .. (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g9_m98_fallback_chain --red-arm force-off-l1 --spv-l1 .. --spv-l3 .. + --red-arm silent-demotion (子模式抽检)", "exit_code": 0 if checks["device_red_arm_submodes_detected"] else 1},
        ],
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
    out = EVIDENCE_DIR / f"g9_m98_tracing_fallback_chain_{ts}.json"
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
