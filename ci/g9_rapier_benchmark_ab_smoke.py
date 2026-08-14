#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.6 M126 Rapier 深造对标基准 A/B 门冒烟(g9.p1.m126.rapier_benchmark_ab;
RFC-0024 §4.E2;spec/physics.md RXS-0378;G9_ACCEPTANCE_MAP §3 M126;
G9_CONTRACT §8.1 裁决① P1 全进;RD-044 字面不变——本门只产基准报告,
不升格深造、不作验收依赖与生产默认)。

host 纯 host 确定性门(device_section_state=not_applicable;harness evidence
实记 device_name=host-only〔Jolt 5.3 + rapier3d 0.33 双臂〕/validation=
not_applicable;rapier feature 默认 off 纪律维持——本门仅 feature on 构建档
产绿)。三段判据:

  host 段:rurix-physics benchmark 2 单测逐名锚定(A/B 同输入双跑位级一致 +
    偏差统计/基准冒充 replay oracle 拒 + 无 measured 数据 RD-044 申请拒)
    + conformance physics M126 双件语料锚定 + measured 报告
    g9_m126_rapier_benchmark.json provenance 机器核验(双臂 digest/计时
    measured + 跨 solver 偏差统计 + RD-044 verdict 诚实登记)。
  harness 段:持锁(gpu_device_lock)真跑 g9_m126_rapier_benchmark --evidence
    (直出件落 .tmp 工作区不覆盖 evidence/ harness 直出件;schema/spec_anchor/
    assertion_id/status==pass + 10 判据闭集全真)+ --red-arm
    replay-oracle/rd044-without-measured 子模式独立复跑抽检。

用法:
  py -3 ci/g9_rapier_benchmark_ab_smoke.py --gate g9.p1.m126.rapier_benchmark_ab
  py -3 ci/g9_rapier_benchmark_ab_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_m126_rapier_benchmark_ab_evidence_schema.json"
REPORT_PATH = ROOT / "milestones" / "g9" / "g9_m126_rapier_benchmark.json"
CORPUS_DIR = ROOT / "conformance" / "physics"
WORK_DIR = ROOT / ".tmp" / "g96_gates" / "m126"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p1.m126.rapier_benchmark_ab"
NUMERIC_STEP = 167
SOURCE_REF = "RFC-0024 §4.E2;spec/physics.md RXS-0378;G9_ACCEPTANCE_MAP §3 M126"
TAG = "g9_m126"
SUBJECT = "g9_m126_rapier_benchmark_ab"
MATRIX_ROW = "M126"

MODULE_TESTS = {
    "benchmark": [
        "ab_fixture_same_input_double_run_bitwise_and_deviation_recorded",
        "benchmark_as_replay_oracle_fail_closed_and_rd044_requires_measured",
    ],
}
CORPUS_FILES = [
    ("accept/rapier_benchmark_ab_fixture_minimal.rx", "RXS-0378"),
    ("reject/rapier_benchmark_as_replay_oracle.rx", "RXS-0378"),
]
REPORT_SCHEMA = "rurix.g9m126.rapier_benchmark.report.v1"
REPORT_SPEC_ANCHOR = "RXS-0378"
HARNESS_BIN = "g9_m126_rapier_benchmark"
HARNESS_FEATURES = "physics-capture,rapier"
HARNESS_SCHEMA = "rurix.g9m126.rapier_benchmark.v1"
HARNESS_ASSERTION = "g9.p1.m126.rapier_benchmark_ab"
HARNESS_TAG = "G9_M126_RAPIER_BENCH"
HARNESS_CHECKS = [
    "conformance_corpus_anchored",
    "same_scene_same_input_same_profile",
    "jolt_double_run_bitwise",
    "rapier_double_run_bitwise",
    "cross_solver_deviation_recorded",
    "rest_above_ground_invariant",
    "benchmark_as_replay_oracle_red",
    "rd044_application_without_measured_red",
    "rd044_condition_literal_unchanged",
    "measured_report_written",
]
RED_ARMS = ["replay-oracle", "rd044-without-measured"]

CHECK_KEYS = [
    "host_module_tests_anchored",
    "conformance_corpus_anchored",
    "measured_report_provenance",
    "cross_solver_deviation_recorded",
    "rd044_verdict_honest",
    "harness_full_pass",
    "harness_checks_closed_set_green",
    "harness_red_arm_submode_detected",
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


def _load_report() -> dict | None:
    if not REPORT_PATH.is_file():
        check(False, f"缺 measured 报告 {REPORT_PATH.name}")
        return None
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        check(False, f"measured 报告不可读: {e}")
        return None


# ═══════════════════════ host 段 ═══════════════════════


def host_module_tests() -> bool:
    ok_all = True
    for module, names in MODULE_TESTS.items():
        rc, blob = run_cmd(["cargo", "test", "-p", "rurix-physics", "--features", HARNESS_FEATURES, "--lib", module])
        ok = rc == 0 and "test result: ok" in blob
        for name in names:
            if not (ok and name in blob):
                check(False, f"{module} 单测 {name} 未锚定/失败")
                ok_all = False
        if not ok:
            check(False, f"cargo test -p rurix-physics --lib {module} 失败")
            ok_all = False
    return ok_all


def host_conformance() -> bool:
    ok = True
    for rel, anchor in CORPUS_FILES:
        path = CORPUS_DIR / rel
        if not path.is_file():
            check(False, f"缺语料 conformance/physics/{rel}")
            ok = False
            continue
        text = path.read_text(encoding="utf-8")
        if f"//@ spec: {anchor}" not in text or GATE_KEY not in text:
            check(False, f"语料 {rel} 缺 `//@ spec: {anchor}` 锚或门 key 留痕")
            ok = False
    return ok


def host_report_provenance() -> bool:
    """measured 报告 provenance(schema/双臂 digest/计时 measured/画像字面)。"""
    doc = _load_report()
    if doc is None:
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"measured 报告 provenance: {msg}")
            ok = False

    need(doc.get("schema") == REPORT_SCHEMA, f"schema ≠ {REPORT_SCHEMA}")
    need(bool(doc.get("generated_at_utc")), "generated_at_utc 空")
    prov = doc.get("provenance", {})
    need("measured_local" in str(prov.get("evidence_level", "")), "evidence_level 非 measured_local(禁 estimated)")
    need(isinstance(prov.get("backends"), dict) and "rapier3d" in str(prov["backends"].get("rapier", "")),
         "provenance.backends.rapier 版本面缺失")
    scenario = doc.get("scenario", {})
    need(scenario.get("same_scene_same_input") is True, "scenario.same_scene_same_input ≠ true")
    need(is_hex(scenario.get("input_digest"), 64), "scenario.input_digest 非 64-hex")
    profile = scenario.get("determinism_profile", {})
    need("1/60" in str(profile.get("dt_fixed", "")), "determinism_profile.dt_fixed 未锁死")
    arms = doc.get("arms", {})
    for arm in ("jolt", "rapier"):
        a = arms.get(arm, {})
        need(a.get("double_run_bitwise") is True, f"arms.{arm}.double_run_bitwise ≠ true(同后端双跑位级一致硬断言)")
        need(is_hex(a.get("world_digest"), 64), f"arms.{arm}.world_digest 非 64-hex")
        need(isinstance(a.get("step_ns_median"), int) and a["step_ns_median"] > 0,
             f"arms.{arm}.step_ns_median 非正(measured 计时缺失)")
        need(isinstance(a.get("contact_events_total"), int), f"arms.{arm}.contact_events_total 缺失")
    need(isinstance(doc.get("glam_migration"), str) and "glam" in doc["glam_migration"],
         "glam 迁移兼容留档缺失")
    need("replay" in str(doc.get("benchmark_not_replay_oracle", "")), "基准不作 replay oracle 字面缺失")
    return ok


def host_cross_solver_deviation() -> bool:
    """跨 solver 确定性偏差统计如实记录(非逐位;不变量/容差对拍)。"""
    doc = _load_report()
    if doc is None:
        return False
    dev = doc.get("cross_solver_deviation", {})
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"跨 solver 偏差统计: {msg}")
            ok = False

    need(isinstance(dev.get("world_chain_bitwise_equal"), bool), "world_chain_bitwise_equal 非 bool(须如实记录)")
    need(isinstance(dev.get("max_translation_abs_diff"), (int, float)), "max_translation_abs_diff 缺失")
    need(isinstance(dev.get("mean_translation_abs_diff"), (int, float)), "mean_translation_abs_diff 缺失")
    need(isinstance(dev.get("max_linvel_abs_diff"), (int, float)), "max_linvel_abs_diff 缺失")
    need(isinstance(dev.get("contact_events_abs_diff"), int), "contact_events_abs_diff 缺失")
    need(dev.get("rest_above_ground_invariant") is True, "rest_above_ground_invariant ≠ true")
    need(dev.get("within_tolerance_0.05m") is True, "within_tolerance_0.05m ≠ true")
    return ok


def host_rd044_verdict() -> bool:
    """RD-044 verdict 诚实登记(条件字面不变 + verdict_basis 可追溯 + 不升格深造)。"""
    doc = _load_report()
    if doc is None:
        return False
    rd = doc.get("rd044", {})
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"RD-044 登记: {msg}")
            ok = False

    need(rd.get("condition_literal_unchanged") is True, "condition_literal_unchanged ≠ true(RD-044 字面不变判据)")
    need(isinstance(rd.get("condition_literal"), str) and "真实 workload" in rd["condition_literal"],
         "condition_literal 字面缺失/漂移")
    need(rd.get("verdict") in ("maintain_no_go", "eligible_to_apply"), f"verdict 非法: {rd.get('verdict')}")
    need(bool(rd.get("verdict_basis")), "verdict_basis 空(measured 依据缺失)")
    need("不升格深造" in str(rd.get("scope", "")), "scope 缺『不升格深造』字面")
    return ok


# ═══════════════════════ harness 段(持锁真跑) ═══════════════════════


def build_harness() -> Path | None:
    rc, blob = run_cmd(["cargo", "build", "-p", "rurix-physics", "--features", HARNESS_FEATURES, "--bin", HARNESS_BIN])
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
    rc, out = run_cmd([str(exe), "--evidence", str(HARNESS_EVIDENCE)], timeout=1800, env=harness_env())
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
    if doc.get("schema") != HARNESS_SCHEMA or doc.get("spec_anchor") != REPORT_SPEC_ANCHOR:
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


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 8:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 8", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    synth = {k: True for k in CHECK_KEYS}
    synth.pop(CHECK_KEYS[0])
    if not (req - set(synth)):
        print(f"[{TAG}] selftest FAIL: 合成缺键未触发 schema required 红", file=sys.stderr)
        return 1
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
    checks["measured_report_provenance"] = host_report_provenance()
    checks["cross_solver_deviation_recorded"] = host_cross_solver_deviation()
    checks["rd044_verdict_honest"] = host_rd044_verdict()

    # harness 段(持锁串行:cargo 构建 + 全档真跑 + RED 臂子模式抽检)
    with gpu_device_lock(purpose="g9_m126 rapier_benchmark_ab harness 腿"):
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
            checks["harness_red_arm_submode_detected"] = run_red_arms(exe)
            note("harness:A/B 同场景同输入同画像 + 双臂双跑位级一致 + measured 报告 + 双 RED 臂 + RD-044 字面不变")

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
        "wave": "G9.6",
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
        print(f"[{TAG}] PASS (host 纯 host 确定性门;harness 持锁真跑 + measured 报告 + RD-044 诚实登记 + 双 RED 臂全绿)")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
