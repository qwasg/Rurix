#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.6 M124 解析浮力走 Field 通道门冒烟(g9.p1.m124.buoyancy_field_channel;
RFC-0024 §4.D;spec/physics.md RXS-0376;G9_ACCEPTANCE_MAP §3 M124;
G9_CONTRACT §8.1 裁决① P1 全进)。

host 纯 host 确定性门(device_section_state=not_applicable;harness evidence
实记 device_name=host-only〔Jolt 5.3 lockstep 单线程〕/validation=
not_applicable)。三段判据:

  host 段:rurix-physics buoyancy 5 单测 + buoyancy_capture 3 单测逐名锚定
    (解析 clip 闭集/旁路 API 拒/fail-closed 面/确定性符号/变帧率逐位一致)
    + conformance physics M124 双件语料锚定 + 冻结带
    g9_m124_buoyancy_freeze.json provenance 机器核验。
  harness 段:持锁(gpu_device_lock)真跑 g9_m124_buoyancy --evidence
    (直出件落 .tmp 工作区不覆盖 evidence/ harness 直出件;schema/spec_anchor/
    assertion_id/status==pass + 9 判据闭集全真)+ --red-arm
    bypass-api/field-unwired/framerate-drift 子模式独立复跑抽检。

用法:
  py -3 ci/g9_buoyancy_field_channel_smoke.py --gate g9.p1.m124.buoyancy_field_channel
  py -3 ci/g9_buoyancy_field_channel_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_m124_buoyancy_field_channel_evidence_schema.json"
BAND_PATH = ROOT / "milestones" / "g9" / "g9_m124_buoyancy_freeze.json"
CORPUS_DIR = ROOT / "conformance" / "physics"
WORK_DIR = ROOT / ".tmp" / "g96_gates" / "m124"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p1.m124.buoyancy_field_channel"
NUMERIC_STEP = 166
SOURCE_REF = "RFC-0024 §4.D;spec/physics.md RXS-0376;G9_ACCEPTANCE_MAP §3 M124"
TAG = "g9_m124"
SUBJECT = "g9_m124_buoyancy_field_channel"
MATRIX_ROW = "M124"

MODULE_TESTS = {
    "field::buoyancy": [
        "analytic_clip_known_values_and_closed_set",
        "bypass_api_rejected_typed_err_single_literal",
        "evaluator_deterministic_buoyancy_and_drag_signs",
        "medium_from_field_channel_params_and_fail_closed_faces",
        "voxel_table_versioned_fail_closed_and_scene_input_digest",
    ],
    "field::buoyancy_capture": [
        "framerate_sensitive_drift_injection_detected_fail_closed",
        "behavior_traits_slender_floats_tumbler_sinks",
        "record_replay_bitexact_and_variable_framerate_identical",
    ],
}
CORPUS_FILES = [
    ("accept/buoyancy_field_channel_minimal.rx", "RXS-0376"),
    ("reject/buoyancy_bypass_api_injection.rx", "RXS-0376"),
]
BAND_SCHEMA = "rurix.g9m124.buoyancy_freeze.v1"
BAND_SPEC_ANCHOR = "RXS-0376"
HARNESS_BIN = "g9_m124_buoyancy"
HARNESS_SCHEMA = "rurix.g9m124.buoyancy.v1"
HARNESS_ASSERTION = "g9.p1.m124.buoyancy_field_channel"
HARNESS_TAG = "G9_M124_BUOYANCY"
HARNESS_CHECKS = [
    "conformance_corpus_anchored",
    "field_channel_buoyancy_green",
    "bypass_api_injection_red",
    "field_channel_unwired_red",
    "framerate_drift_injection_red",
    "capture_replay_tick_hash_equal",
    "variable_framerate_bitwise_identical",
    "behavior_traits_and_corpus_fixture",
    "measured_freeze_digest_match",
]
RED_ARMS = ["bypass-api", "field-unwired", "framerate-drift"]

CORPUS_FIXTURE_SCENARIOS = ["slender_body", "tumbler_body"]
CORPUS_FIXTURE_FILES = ["header.json", "input.json", "state0.json", "state_final.json", "journal.jsonl", "expected.json"]

CHECK_KEYS = [
    "host_module_tests_anchored",
    "conformance_corpus_anchored",
    "corpus_fixture_provenance",
    "band_provenance_frozen",
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


# ═══════════════════════ host 段 ═══════════════════════


def host_module_tests() -> bool:
    ok_all = True
    for module, names in MODULE_TESTS.items():
        rc, blob = run_cmd(["cargo", "test", "-p", "rurix-physics", "--features", "physics-buoyancy", "--lib", module])
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
    need("禁手写" in str(band.get("provenance", "")), "provenance 缺『禁手写』纪律字面")
    for scenario in ("slender_body", "tumbler_body"):
        for field in ("world_digest", "journal_digest", "field_chain_digest", "input_digest"):
            key = f"{scenario}_{field}"
            val = band.get(key)
            need(is_hex(val, 64) if val else False, f"{key} 非 64-hex")
    return ok


def host_corpus_fixture() -> bool:
    """细长/翻滚 corpus fixture provenance(禁手写 golden:六件齐备 + header
    capture schema/scenario_id 锚 + expected 行为特征面非空)。"""
    ok = True
    for scenario in CORPUS_FIXTURE_SCENARIOS:
        base = CORPUS_DIR / "buoyancy" / scenario
        for name in CORPUS_FIXTURE_FILES:
            if not (base / name).is_file():
                check(False, f"缺 corpus fixture conformance/physics/buoyancy/{scenario}/{name}")
                ok = False
        if not ok:
            continue
        try:
            header = json.loads((base / "header.json").read_text(encoding="utf-8"))
            expected = json.loads((base / "expected.json").read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as e:
            check(False, f"corpus fixture {scenario} 不可读: {e}")
            ok = False
            continue
        if header.get("schema_id") != "rurix.physics.capture":
            check(False, f"corpus fixture {scenario} header.schema_id 不符")
            ok = False
        if scenario not in str(header.get("scenario_id", "")):
            check(False, f"corpus fixture {scenario} header.scenario_id 未锚定场景")
            ok = False
        if not expected:
            check(False, f"corpus fixture {scenario} expected 行为特征面空")
            ok = False
    return ok


# ═══════════════════════ harness 段(持锁真跑) ═══════════════════════


def build_harness() -> Path | None:
    rc, blob = run_cmd(["cargo", "build", "-p", "rurix-physics", "--features", "physics-buoyancy", "--bin", HARNESS_BIN])
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


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 7:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 7", file=sys.stderr)
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
    checks["corpus_fixture_provenance"] = host_corpus_fixture()
    checks["band_provenance_frozen"] = host_band_provenance()

    # harness 段(持锁串行:cargo 构建 + 全档真跑 + RED 臂子模式抽检)
    with gpu_device_lock(purpose="g9_m124 buoyancy_field_channel harness 腿"):
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
            note("harness:解析浮力走 Field 通道 + corpus fixture + capture/replay 逐 tick hash + 变帧率逐位一致 + 三 RED 臂")

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
        print(f"[{TAG}] PASS (host 纯 host 确定性门;harness 持锁真跑 + 冻结带 provenance + 三 RED 臂全绿)")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
