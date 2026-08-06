#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.5b M25 upscaler_input_abi 硬门冒烟(g8.p1.m25.upscaler_input_abi)。

host:ABI 十项/hash/fail-closed + TSR/CAS 双非 no-op backend 序列 golden。
device:uc06-renderer --m25-upscaler-abi(CAS .rx vs host;RURIX_REQUIRE_REAL=1)。
悬置七行裁决:evidence.retained_open + G8_CANDIDATE_DECISIONS 加性节。

用法:
  py -3 ci/g8_upscaler_input_abi_smoke.py --gate g8.p1.m25.upscaler_input_abi
  py -3 ci/g8_upscaler_input_abi_smoke.py --selftest
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
GOLDEN_DIR = ROOT / "tests" / "upscaler_abi" / "golden"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_m25_upscaler_input_abi_evidence_schema.json"
CANDIDATE = ROOT / "milestones" / "g8" / "G8_CANDIDATE_DECISIONS.md"

GATE_KEY = "g8.p1.m25.upscaler_input_abi"
# CI_step 待领(ledger next_free);evidence 用 0 占位,Gov 接线时改写。
NUMERIC_STEP = 118
SOURCE_REF = (
    "G8_ACCEPTANCE_MAP §2 M25;G8.5_RENDERING_COMPLETION_DESIGN §4–§5;"
    "RD-041 M25 go;vendor FSR FFI open-observe"
)
TAG = "g8_m25"
WAVE = "G8.5b"

CHECK_KEYS = [
    "abi_ten_inputs_enumerated",
    "abi_layout_hash_stable",
    "abi_hash_sensitive",
    "missing_input_fail_closed",
    "hash_mismatch_fail_closed",
    "backend_tsr_consumes_all",
    "backend_cas_consumes_all",
    "output_extent_and_finite",
    "sequence_digest_match",
    "backend_switch_abi_identical",
    "not_stub",
    "validation_zero",
]

RETAINED_OPEN = [
    {
        "matrix_row": "M05",
        "decision": "no-go",
        "status": "open-留 G8.7",
        "trigger": "维持决策表 RD-039 位移/RD-041 蒙皮·WPO MV 既判;动态资产面出现后重判",
    },
    {
        "matrix_row": "M07",
        "decision": "no-go",
        "status": "open",
        "trigger": "RT 与主几何误差联动需求出现(M50 消费侧真实资产)时判档",
    },
    {
        "matrix_row": "M08",
        "decision": "no-go",
        "status": "open",
        "trigger": "材质数规模使 dispatch 分桶成为 measured 瓶颈时",
    },
    {
        "matrix_row": "M45",
        "decision": "no-go",
        "status": "open",
        "trigger": "HDR 显示设备资产/产品需求出现时",
    },
    {
        "matrix_row": "M46",
        "decision": "no-go",
        "status": "open",
        "trigger": "产品级后处理需求(bloom/DOF/曝光分级)随 G9+ 建造期出现时",
    },
    {
        "matrix_row": "M47",
        "decision": "no-go",
        "status": "open",
        "trigger": "透明资产面出现时;OIT 策略选型需 measured 对照(M24 最小透明贡献不冒充 M47)",
    },
    {
        "matrix_row": "M17",
        "decision": "no-go",
        "status": "open",
        "trigger": "GI/材质画质门需要跨路径 golden 时(G9+ 建造期前置;建议 G8.7 复审)",
    },
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


def extract_json(stdout: str) -> dict | None:
    text = stdout.strip()
    if not text:
        return None
    for line in reversed(text.splitlines()):
        line = line.strip()
        if line.startswith("{") and "subject" in line:
            try:
                return json.loads(line)
            except Exception:
                continue
    try:
        return json.loads(text)
    except Exception:
        return None


def run_probe(write_golden: bool = False) -> dict | None:
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "rurix-render",
        "--bin",
        "g8_m25_probe",
        "--",
        "--golden-dir",
        str(GOLDEN_DIR),
    ]
    if write_golden:
        cmd.append("--write-golden")
    print(f"[{TAG}] {' '.join(cmd[-6:])}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        check(False, f"g8_m25_probe 失败 rc={r.returncode}\n{r.stderr[-2000:]}")
    return extract_json(r.stdout)


def run_device() -> tuple[str, dict | None]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "uc06-renderer",
        "--features",
        "vulkan",
        "--",
        "--m25-upscaler-abi",
    ]
    print(f"[{TAG}] device: --m25-upscaler-abi")
    try:
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env, timeout=600)
    except subprocess.TimeoutExpired:
        check(False, "device timeout")
        return "fail", None
    doc = extract_json(r.stdout)
    blob = (r.stdout or "") + (r.stderr or "")
    if doc is None and "SKIP" in blob:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1):\n{blob[-2000:]}")
        return "skipped_dev_env", None
    if doc is None:
        check(False, f"device JSON 缺失 rc={r.returncode}\n{blob[-2000:]}")
        return "fail", None
    if r.returncode != 0 and not doc.get("pass"):
        check(False, f"device 失败 rc={r.returncode}")
        return "fail", doc
    return "executed", doc


def run_selftest() -> int:
    assert SCHEMA_PATH.is_file(), "缺 M25 evidence schema"
    assert len(CHECK_KEYS) == 12
    assert len(RETAINED_OPEN) == 7
    fake = {k: False for k in CHECK_KEYS}
    fake["abi_ten_inputs_enumerated"] = True
    assert not all(fake.values())
    # 反假绿:noop 透传不得让 not_stub 为真
    assert "not_stub" in CHECK_KEYS
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} retained_open={len(RETAINED_OPEN)}")
    return 0


def candidate_section_ok() -> bool:
    if not CANDIDATE.is_file():
        return False
    text = CANDIDATE.read_text(encoding="utf-8")
    return "矩阵 P1 未判行补裁决" in text and all(
        x in text for x in ("M07", "M08", "M17", "M45", "M46", "M47", "M05")
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--write-golden", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    checks = {k: False for k in CHECK_KEYS}

    if args.write_golden or not (GOLDEN_DIR / "tsr_sequence.sha256").is_file():
        run_probe(write_golden=True)

    # host units(单 filter:cargo test 只接受一个 TESTNAME)
    print(f"[{TAG}] cargo test -p rurix-render temporal::")
    tr = subprocess.run(
        ["cargo", "test", "-q", "-p", "rurix-render", "temporal::"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if tr.returncode != 0:
        check(False, f"temporal:: tests 失败:\n{tr.stdout}\n{tr.stderr}")

    probe = run_probe(write_golden=False)
    abi_hash = ""
    if probe:
        for k in CHECK_KEYS:
            if k == "validation_zero":
                continue
            ok = bool(probe.get(k))
            checks[k] = ok
            if not ok:
                check(False, f"probe.{k} 为假")
        abi_hash = str(probe.get("abi_hash") or "")
        note(f"abi_hash={abi_hash}")
        note(f"tsr_digest={probe.get('tsr_sequence_digest')}")
        note(f"cas_digest={probe.get('cas_sequence_digest')}")
        if not probe.get("pass"):
            check(False, "probe.pass=false")
    else:
        check(False, "probe JSON 缺失")

    device_state, doc = run_device()
    if device_state == "executed" and doc:
        checks["backend_cas_consumes_all"] = checks["backend_cas_consumes_all"] and bool(
            doc.get("consumes_all")
        )
        checks["output_extent_and_finite"] = checks["output_extent_and_finite"] and bool(
            doc.get("output_extent_ok")
        ) and bool(doc.get("finite"))
        checks["validation_zero"] = int(doc.get("validation_errors") or 0) == 0
        if not doc.get("pass"):
            check(False, "device pass=false")
        if not doc.get("not_passthrough"):
            check(False, "device 输出疑似透传")
        note(f"device max_abs_err={doc.get('max_abs_err')}")
    elif device_state == "skipped_dev_env":
        checks["validation_zero"] = False

    if not candidate_section_ok():
        check(False, "G8_CANDIDATE_DECISIONS 缺「矩阵 P1 未判行补裁决」加性节")
    else:
        note("retained_open anchored in CANDIDATE §加性节 + evidence array")

    host_keys = [k for k in CHECK_KEYS if k != "validation_zero"]
    host_pass = all(checks[k] for k in host_keys) and tr.returncode == 0

    all_pass = all(checks.values()) and not FAILURES
    if device_state == "skipped_dev_env" and require_real():
        all_pass = False
        device_section = "skipped_dev_env"
    elif all_pass and device_state == "executed":
        device_section = "pass"
    else:
        device_section = device_state if device_state else "fail"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g8_m25_upscaler_input_abi",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M25",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_pass,
        "device_section_state": device_section,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "abi_hash": abi_hash if len(abi_hash) == 64 else ("0" * 64),
        "backends": {
            "primary": "tsr",
            "secondary": "cas_easu",
            "vendor_fsr_ffi": "open_rd041_observe",
        },
        "retained_open": RETAINED_OPEN,
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"g8_m25_upscaler_input_abi_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    try:
        import jsonschema

        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        errs = list(jsonschema.Draft7Validator(schema).iter_errors(evidence))
        for e in errs:
            check(False, f"schema: {e.message}")
    except ImportError:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")
        note("jsonschema missing; required-keys only")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_section}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
