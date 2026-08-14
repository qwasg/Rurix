#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.5 M120 OIT benchmark harness 门冒烟(g9.p1.m120.oit_benchmark_harness;
RFC-0025 §4.K;spec/display_pipeline.md RXS-0371;G9_ACCEPTANCE_MAP §3 M120;
G9_CONTRACT §8.1 裁决① P1 全进——仅测量不定档,D4 D15)。

host 纯 host 确定性门(device_section_state=not_applicable;harness evidence
实记 device_name=host-only〔4070 Ti device 帧时腿归后续波〕/validation=
not_applicable)。三段判据 + 不定档字段机器核验:

  host 段:rurix-render oit:: 10 单测逐名锚定(选型 fail-closed NotMeasuredYet/
    无数据提交拒/排序 fallback 可达+精确档范围闭集/精确档内存无界拒/场景
    确定性非平凡/benchmark evidence 非空/精确档 vs 排序真值 diff=0/排序
    fallback 恒可用/近似算法误差 measured/内存模型逐算法公式)+ conformance
    display_pipeline M120 双件语料锚定(//@ spec: RXS-0371 + 门 key/脚本名
    留痕)+ 冻结带 milestones/g9/g9_m120_oit_measurements.json provenance
    机器核验(七算法 × 4 overdraw 档 + scene/truth digest)。
  harness 段:持锁(gpu_device_lock)真跑 g9_m120_oit_benchmark --evidence
    (直出件落 .tmp 工作区不覆盖 evidence/ harness 直出件;schema/spec_anchor/
    assertion_id/status==pass + 9 判据闭集全真)+ --red-arm
    selection-without-data/unbounded-memory 子模式独立复跑抽检。
  不定档字段机器核验:tier_selection.committed==false(仅测量不定档——本门
    只产 benchmark 数据不定默认档,选型入口 fail-closed)。

用法:
  py -3 ci/g9_oit_benchmark_harness_smoke.py --gate g9.p1.m120.oit_benchmark_harness
  py -3 ci/g9_oit_benchmark_harness_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_m120_oit_benchmark_harness_evidence_schema.json"
BAND_PATH = ROOT / "milestones" / "g9" / "g9_m120_oit_measurements.json"
CORPUS_DIR = ROOT / "conformance" / "display_pipeline"
WORK_DIR = ROOT / ".tmp" / "g95_gates" / "m120"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p1.m120.oit_benchmark_harness"
NUMERIC_STEP = 164
SOURCE_REF = "RFC-0025 §4.K;spec/display_pipeline.md RXS-0371;G9_ACCEPTANCE_MAP §3 M120"
TAG = "g9_m120"
SUBJECT = "g9_m120_oit_benchmark_harness"
MATRIX_ROW = "M120"

MODULE_TESTS = {
    "oit::selection": [
        "selection_fail_closed_not_measured_yet",
        "selection_commit_without_data_rejected",
        "sorted_fallback_reachable_and_exact_scope_closed",
        "exact_tier_unbounded_memory_rejected",
    ],
    "oit::scene": [
        "scene_deterministic_and_nontrivial",
    ],
    "oit::measure": [
        "benchmark_evidence_nonempty",
    ],
    "oit::algorithms": [
        "linked_list_exact_vs_sorted_truth_diff_zero",
        "sorted_fallback_always_available",
        "approximate_algorithms_show_measured_error",
        "memory_models_per_algorithm_formula",
    ],
}
CORPUS_FILES = [
    ("accept/oit_benchmark_harness_minimal.rx", "RXS-0371"),
    ("reject/oit_default_tier_without_benchmark_data.rx", "RXS-0371"),
]
BAND_SCHEMA = "rurix.g9m120.oit_measurements.v1"
BAND_SPEC_ANCHOR = "RXS-0371"
BAND_ALGORITHMS = ["simple", "linked_list", "loop32", "loop64", "spinlock", "interlock", "weighted_blended"]
HARNESS_BIN = "g9_m120_oit_benchmark"
HARNESS_SCHEMA = "rurix.g9m120.oit_benchmark.v1"
HARNESS_ASSERTION = "g9.p1.m120.oit_benchmark_harness"
HARNESS_TAG = "G9_M120_OIT"
HARNESS_CHECKS = [
    "conformance_corpus_anchored",
    "benchmark_evidence_nonempty",
    "double_run_deterministic_bit_equal",
    "linked_list_exact_tier_diff_zero",
    "sorted_fallback_always_reachable",
    "quality_measurement_sensitive",
    "selection_without_data_red_arm",
    "exact_tier_unbounded_memory_red_arm",
    "measurements_frozen_equal",
]
RED_ARMS = ["selection-without-data", "unbounded-memory"]

CHECK_KEYS = [
    "host_module_tests_anchored",
    "conformance_corpus_anchored",
    "band_provenance_frozen",
    "harness_full_pass",
    "harness_checks_closed_set_green",
    "harness_red_arm_submode_detected",
    "not_triggered_field_verified",
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


def run_cmd(cmd: list[str], *, record: bool = True, timeout: int = 1800, env: dict | None = None) -> tuple[int, str]:
    print(f"[{TAG}] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    if record:
        COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(cmd), "exit_code": r.returncode})
    return r.returncode, r.stdout + r.stderr


def target_dir() -> Path:
    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


def is_hex(v: object, n: int) -> bool:
    return isinstance(v, str) and len(v) == n and all(c in "0123456789abcdef" for c in v)


# ═══════════════════════ host 段 ═══════════════════════


def host_module_tests() -> bool:
    ok_all = True
    for module, names in MODULE_TESTS.items():
        rc, blob = run_cmd(["cargo", "test", "-p", "rurix-render", "--lib", module])
        ok = rc == 0 and "test result: ok" in blob
        for name in names:
            if not (ok and name in blob):
                check(False, f"{module} 单测 {name} 未锚定/失败")
                ok_all = False
        if not ok:
            check(False, f"cargo test -p rurix-render --lib {module} 失败")
            ok_all = False
    return ok_all


def host_conformance() -> bool:
    ok = True
    for rel, anchor in CORPUS_FILES:
        path = CORPUS_DIR / rel
        if not path.is_file():
            check(False, f"缺语料 conformance/display_pipeline/{rel}")
            ok = False
            continue
        text = path.read_text(encoding="utf-8")
        if f"//@ spec: {anchor}" not in text or GATE_KEY not in text:
            check(False, f"语料 {rel} 缺 `//@ spec: {anchor}` 锚或门 key 留痕")
            ok = False
    return ok


def host_band_provenance() -> bool:
    if not BAND_PATH.is_file():
        check(False, f"缺冻结带 {BAND_PATH.name}")
        return False
    try:
        band = json.loads(BAND_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        check(False, f"冻结带不可读: {e}")
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"冻结带 provenance: {msg}")
            ok = False

    need(band.get("schema") == BAND_SCHEMA, f"schema ≠ {BAND_SCHEMA}")
    need(band.get("spec_anchor") == BAND_SPEC_ANCHOR, f"spec_anchor ≠ {BAND_SPEC_ANCHOR}")
    need(bool(band.get("frozen_at_utc")), "frozen_at_utc 空")
    need(str(band.get("provenance", "")).startswith("Assisted-by:"), "provenance 空/形态不符")
    need("禁手写" in str(band.get("freeze_rule", "")), "freeze_rule 缺『禁手写』纪律字面")
    need("仅测量不定档" in str(band.get("freeze_rule", "")), "freeze_rule 缺『仅测量不定档』字面")
    need(band.get("algorithms") == BAND_ALGORITHMS, "algorithms ≠ nvpro 七算法闭集")
    need(isinstance(band.get("overdraw_levels"), list) and len(band["overdraw_levels"]) == 4, "overdraw_levels ≠ 4 档")
    for field in ("scene_digests", "truth_digests"):
        dig = band.get(field)
        need(isinstance(dig, dict) and len(dig) == 4, f"{field} ≠ 4 档")
        for k, v in (dig or {}).items():
            need(is_hex(v, 64), f"{field}.{k} 非 64-hex")
    need(isinstance(band.get("measurements"), dict) and bool(band["measurements"]), "measurements 空(evidence 非空判据)")
    return ok


# ═══════════════════════ harness 段(持锁真跑) ═══════════════════════


def build_harness() -> Path | None:
    rc, blob = run_cmd(["cargo", "build", "-p", "rurix-render", "--bin", HARNESS_BIN])
    if rc != 0:
        check(False, f"{HARNESS_BIN} 构建失败:\n{blob[-2000:]}")
        return None
    exe = target_dir() / "debug" / (HARNESS_BIN + (".exe" if sys.platform == "win32" else ""))
    if not exe.is_file():
        check(False, f"harness 产物缺失: {exe}")
        return None
    return exe


def harness_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_BASE_COMMIT"] = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
    ).stdout.strip()
    return env


def run_harness_full(exe: Path) -> dict | None:
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    rc, out = run_cmd([str(exe), "--evidence", str(HARNESS_EVIDENCE)], timeout=3600, env=harness_env())
    doc = None
    if HARNESS_EVIDENCE.is_file():
        try:
            doc = json.loads(HARNESS_EVIDENCE.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            check(False, f"harness evidence 不可解析: {e}")
    if rc != 0 or f"{HARNESS_TAG}: PASS" not in out:
        check(False, f"harness 全档失败 rc={rc}:\n{out[-2000:]}")
        return None
    if doc is None:
        check(False, "harness evidence 缺失")
        return None
    if doc.get("schema") != HARNESS_SCHEMA or doc.get("spec_anchor") != BAND_SPEC_ANCHOR:
        check(False, "harness evidence schema/spec_anchor 字面不符")
    if doc.get("assertion_id") != HARNESS_ASSERTION or doc.get("status") != "pass":
        check(False, "harness evidence assertion_id/status 不符")
    if doc.get("failures") != []:
        check(False, f"harness evidence failures 非空: {doc.get('failures')}")
    return doc


def run_red_arms(exe: Path) -> bool:
    ok_all = True
    for arm in RED_ARMS:
        rc, out = run_cmd([str(exe), "--red-arm", arm], timeout=1800, env=harness_env())
        ok = rc == 0 and f"{HARNESS_TAG}: PASS red-arm {arm}" in out
        if not ok:
            check(False, f"RED 臂子模式 {arm} 未独立检出 rc={rc}: {out[-600:]}")
            ok_all = False
    return ok_all


def verify_not_triggered(doc: dict) -> bool:
    """仅测量不定档字段机器核验(tier_selection.committed==false)。"""
    ts = doc.get("tier_selection", {})
    ok = ts.get("committed") is False and "仅测量不定档" in str(ts.get("policy", ""))
    if not ok:
        check(False, f"tier_selection 不定档字段核验失败: {ts}")
    return ok


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    # 红臂①:合成 FAILURES 必须使门红(check() 判别有效)。
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 7:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 7", file=sys.stderr)
        return 1
    # 红臂②:合成缺键 evidence 必触发 schema checks.required 红。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    synth = {k: True for k in CHECK_KEYS}
    synth.pop(CHECK_KEYS[0])
    if not (req - set(synth)):
        print(f"[{TAG}] selftest FAIL: 合成缺键未触发 schema required 红", file=sys.stderr)
        return 1
    # 绿臂:schema checks.required 与 CHECK_KEYS 闭集精确互核。
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
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

    # host 段
    checks["host_module_tests_anchored"] = host_module_tests()
    checks["conformance_corpus_anchored"] = host_conformance()
    checks["band_provenance_frozen"] = host_band_provenance()

    # harness 段(持锁串行:cargo 构建 + 全档真跑 + 不定档核验 + RED 臂子模式抽检)
    with gpu_device_lock(purpose="g9_m120 oit_benchmark harness 腿"):
        exe = build_harness()
        if exe:
            doc = run_harness_full(exe)
            if doc is not None and not FAILURES:
                checks["harness_full_pass"] = True
                hc = doc.get("checks", {})
                green = True
                for k in HARNESS_CHECKS:
                    if hc.get(k) is not True:
                        check(False, f"harness 判据 {k} 非 true")
                        green = False
                checks["harness_checks_closed_set_green"] = green
                checks["not_triggered_field_verified"] = verify_not_triggered(doc)
            checks["harness_red_arm_submode_detected"] = run_red_arms(exe)
            note("harness:七算法 × 4 档 evidence 非空 + 仅测量不定档 + 精确档 diff=0 + RED 双臂子模式复跑")

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
        "wave": "G9.5",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        ).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
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

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS (host 纯 host 确定性门;harness 持锁真跑 + 仅测量不定档 + RED 双臂子模式全绿)")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
