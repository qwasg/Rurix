#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.5 M110 世界分区门冒烟(g9.p0.m110.world_partition;RFC-0025 §4.A;
spec/world_partition.md RXS-0363;G9_ACCEPTANCE_MAP §2 M110)。

host 纯 host 确定性门(device_section_state=not_applicable;harness evidence
实记 device_name=host-only/validation=not_applicable)。三段判据:

  host 段:rurix-render world::partition 12 单测逐名锚定(schema 往返/cell
    资产属性/fail-closed/Data Layer 预留/流送预算逐帧/违约排队双臂/事件乱序
    校验/digest 序敏/一致性机核/目标 cell 对拍/soak 确定性)+ conformance
    world_partition M110 三件语料锚定(//@ spec: RXS-0363 + 门 key/脚本名留痕)
    + 冻结带 milestones/g9/g9_m110_world_partition_band.json provenance
    机器核验(schema/spec_anchor/frozen_at_utc/provenance/freeze_rule 禁手写
    + 三 digest 形态 + hlod_tool 字面)。
  harness 段:持锁(gpu_device_lock)真跑 g9_m110_world_partition --evidence
    (直出件落 .tmp 工作区不覆盖 evidence/ harness 直出件;schema/spec_anchor/
    assertion_id/status==pass + 10 判据闭集全真)+ --red-arm
    budget-overrun/event-order 子模式独立复跑抽检。

用法:
  py -3 ci/g9_world_partition_smoke.py --gate g9.p0.m110.world_partition
  py -3 ci/g9_world_partition_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_m110_world_partition_evidence_schema.json"
BAND_PATH = ROOT / "milestones" / "g9" / "g9_m110_world_partition_band.json"
CORPUS_DIR = ROOT / "conformance" / "world_partition"
WORK_DIR = ROOT / ".tmp" / "g95_gates" / "m110"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p0.m110.world_partition"
NUMERIC_STEP = 154
SOURCE_REF = "RFC-0025 §4.A;spec/world_partition.md RXS-0363;G9_ACCEPTANCE_MAP §2 M110"
TAG = "g9_m110"
SUBJECT = "g9_m110_world_partition"
MATRIX_ROW = "M110"

MODULE_TESTS = {
    "world::partition": [
        "schema_roundtrip_byte_equal",
        "cell_size_is_asset_property_and_bounds_derived",
        "decode_fail_closed_variants",
        "data_layer_reserved_not_wired",
        "streaming_diff_and_per_frame_budget_evidence",
        "budget_violation_injection_queues_and_alarms",
        "memory_budget_violation_queues",
        "event_order_validator_rejects_out_of_order",
        "event_log_digest_order_sensitive",
        "frame_budget_consistency_checker_catches_silent_overrun",
        "target_cells_matches_brute_force",
        "soak_world_scale_and_path_deterministic",
    ],
}
CORPUS_FILES = [
    ("accept/cell_event_sequence_minimal.rx", "RXS-0363"),
    ("reject/cell_event_sequence_out_of_order.rx", "RXS-0363"),
    ("reject/partition_budget_overrun_no_demote.rx", "RXS-0363"),
]
BAND_SCHEMA = "rurix.g9m110.world_partition_band.v1"
BAND_SPEC_ANCHOR = "RXS-0363"
BAND_DIGEST_FIELDS = ["world_digest", "event_log_digest", "hlod_digest"]
HARNESS_BIN = "g9_m110_world_partition"
HARNESS_SCHEMA = "rurix.g9m110.world_partition.v1"
HARNESS_ASSERTION = "g9.p0.m110.world_partition"
HARNESS_TAG = "G9_M110_WP"
HARNESS_CHECKS = [
    "conformance_corpus_anchored",
    "schema_roundtrip_and_cell_frozen",
    "data_layer_reserved_unwired",
    "budget_counters_per_frame_nonempty",
    "budget_violation_queued_demote_red_arm",
    "cell_event_sequence_golden_equal",
    "event_out_of_order_red_arm",
    "hlod_double_build_hash_equal",
    "soak_hitch_p99_within_measured_threshold",
    "budget_threshold_provenance",
]
RED_ARMS = ["budget-overrun", "event-order"]

CHECK_KEYS = [
    "host_module_tests_anchored",
    "conformance_corpus_anchored",
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
            check(False, f"缺语料 conformance/world_partition/{rel}")
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
    for f in BAND_DIGEST_FIELDS:
        need(is_hex(band.get(f), 64), f"{f} 非 64-hex")
    need(band.get("hlod_tool") == "rurix.hlod.bake.v1", "hlod_tool ≠ rurix.hlod.bake.v1")
    return ok


# ═══════════════════════ harness 段(持锁真跑) ═══════════════════════


def build_harness() -> Path | None:
    rc, blob = run_cmd(["cargo", "build", "-p", "rurix-render", "--bin", HARNESS_BIN])
    if rc != 0:
        check(False, f"{HARNESS_BIN} 构建失败:\n{blob[-2000:]}")
        return None
    # harness HLOD 双构建腿要求 rxhlod 与 harness 同构建目录(rurix.hlod.bake.v1)。
    rc2, blob2 = run_cmd(["cargo", "build", "-p", "rurix-asset", "--bin", "rxhlod"])
    if rc2 != 0:
        check(False, f"rxhlod 构建失败:\n{blob2[-2000:]}")
        return None
    exe = target_dir() / "debug" / (HARNESS_BIN + (".exe" if sys.platform == "win32" else ""))
    if not exe.is_file():
        check(False, f"harness 产物缺失: {exe}")
        return None
    if not (target_dir() / "debug" / ("rxhlod.exe" if sys.platform == "win32" else "rxhlod")).is_file():
        check(False, "rxhlod 产物缺失(未与 harness 同构建)")
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
    rc, out = run_cmd(
        [str(exe), "--evidence", str(HARNESS_EVIDENCE), "--work-dir", str(WORK_DIR / "work")],
        timeout=3600,
        env=harness_env(),
    )
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
    # 红臂①:合成 FAILURES 必须使门红(check() 判别有效)。
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 6:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 6", file=sys.stderr)
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

    # harness 段(持锁串行:cargo 构建 + 全档真跑 + RED 臂子模式抽检)
    with gpu_device_lock(purpose="g9_m110 world_partition harness 腿"):
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
            note("harness:全档真跑(soak hitch p99 ≤ g9_budget 冻结阈值)+ RED 双臂子模式复跑")

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
        print(f"[{TAG}] PASS (host 纯 host 确定性门;harness 持锁真跑 + RED 双臂子模式全绿)")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
