#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.4 M37 streaming_io 硬门冒烟(g8.p0.m37.streaming_io)。

device:uc06-renderer --stream-io(真实磁盘 async read → 冻结 decoder →
host-visible upload → GPU FNV；迟到页 fallback/恢复；queue_mode=single)。

用法:
  py -3 ci/g8_streaming_io_smoke.py --gate g8.p0.m37.streaming_io
  py -3 ci/g8_streaming_io_smoke.py --selftest
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
GOLDEN_DIR = ROOT / "tests" / "geom_pages" / "golden"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_m37_streaming_io_evidence_schema.json"
KERNEL = ROOT / "apps" / "uc06-renderer" / "kernels" / "stream_consume_digest.rx"

GATE_KEY = "g8.p0.m37.streaming_io"
NUMERIC_STEP = 112
SOURCE_REF = (
    "G8_ACCEPTANCE_MAP §2 M37;G8.3_G8.4_ASSET_STREAMING_DESIGN §4.1;"
    "RFC-0019 §4.8.3 queue_mode=single"
)
TAG = "g8_m37"
WAVE = "G8.4"

CHECK_KEYS = [
    "real_disk_file_read",
    "per_page_stage_order_monotonic",
    "decompress_via_frozen_decoder",
    "final_device_digest_equals_golden",
    "late_page_fallback_frame_present",
    "late_page_recovers_correct",
    "fault_injection_deterministic",
    "budgets_metered",
    "queue_mode_single_registered",
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


def extract_json(stdout: str) -> dict | None:
    text = stdout.strip()
    if not text:
        return None
    for line in reversed(text.splitlines()):
        line = line.strip()
        if line.startswith("{") and ("subject" in line or "checks" in line):
            try:
                return json.loads(line)
            except Exception:
                continue
    try:
        return json.loads(text)
    except Exception:
        return None


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
        "--stream-io",
        "--golden-dir",
        str(GOLDEN_DIR),
    ]
    print(f"[{TAG}] device: --stream-io")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env, timeout=900)
    doc = extract_json(r.stdout)
    merged = (r.stdout or "") + (r.stderr or "")
    if doc is None and "SKIP" in merged:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1):\n{merged[-2000:]}")
        return "skipped_dev_env", None
    if doc is None:
        check(
            False,
            f"device JSON 缺失 rc={r.returncode}\n{r.stderr[-1500:]}\n{r.stdout[-1500:]}",
        )
        return "fail", None
    if r.returncode != 0 and not doc.get("pass"):
        check(False, f"device 失败 rc={r.returncode} pass=false")
        return "fail", doc
    return "executed", doc


def run_selftest() -> int:
    assert SCHEMA_PATH.is_file(), "缺 M37 evidence schema"
    assert KERNEL.is_file(), "缺 stream_consume_digest.rx"
    assert (GOLDEN_DIR / "m04_page0.rxpd").is_file(), "缺 M04 golden RXPD"
    assert len(CHECK_KEYS) == 10
    # 反假绿：预载内存替身不得单独充绿
    fake = {k: False for k in CHECK_KEYS}
    fake["queue_mode_single_registered"] = True
    assert not all(fake.values())
    src = KERNEL.read_text(encoding="utf-8")
    assert "2166136261" in src and "16777619" in src
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)}")
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

    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    checks = {k: False for k in CHECK_KEYS}

    # host 锚：golden + kernel + 冻结 decoder crate 存在
    host_ok = (
        (GOLDEN_DIR / "m04_page0.rxpd").is_file()
        and KERNEL.is_file()
        and (ROOT / "src" / "rurix-geom-pages" / "src" / "disk.rs").is_file()
    )
    check(host_ok, "host 锚(golden/kernel/decoder)缺失")
    note(f"golden={GOLDEN_DIR / 'm04_page0.rxpd'}")

    device_state, doc = run_device()
    if device_state == "executed" and doc:
        c = doc.get("checks") or {}
        for k in CHECK_KEYS:
            ok = bool(c.get(k))
            checks[k] = ok
            if not ok:
                check(False, f"device.checks.{k}=false")
        qm = doc.get("queue_mode")
        checks["queue_mode_single_registered"] = qm == "single" and checks.get(
            "queue_mode_single_registered", False
        )
        if qm != "single":
            check(False, f"queue_mode={qm!r} 期望 single")
        if int(doc.get("validation_errors") or 0) != 0:
            checks["device_validation_zero"] = False
            check(False, "validation_errors!=0")
        if not doc.get("pass"):
            check(False, "device pass=false")
        note(
            f"fallback={doc.get('fallback_frames')} recovered={doc.get('recovered')} "
            f"digest={doc.get('device_digest')} io={doc.get('bytes_io')}"
        )
    elif device_state == "skipped_dev_env":
        for k in CHECK_KEYS:
            checks[k] = False

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
        "subject": "g8_m37_streaming_io",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M37",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_ok,
        "device_section_state": device_section,
        "queue_mode": "single",
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
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"g8_m37_streaming_io_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    for k in schema.get("required", []):
        check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_section}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
