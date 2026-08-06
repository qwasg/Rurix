#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.4 门-GeomPage 独立硬门(g8.gate.geom_page)。

device:uc06-renderer --geom-page(冻结 M04 ABI 消费 + 按需驻留 + root 钉住 +
LRU 压力 + 独立迟到页证据)。独立 evidence key，不并入 M37。

用法:
  py -3 ci/g8_geom_page_gate.py --gate g8.gate.geom_page
  py -3 ci/g8_geom_page_gate.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
GOLDEN_DIR = ROOT / "tests" / "geom_pages" / "golden"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_gate_geom_page_evidence_schema.json"
HARNESS = ROOT / "apps" / "uc06-renderer" / "src" / "device_m37.rs"

GATE_KEY = "g8.gate.geom_page"
NUMERIC_STEP = 113
SOURCE_REF = (
    "G8_ACCEPTANCE_MAP G-G8-6;G8.3_G8.4_ASSET_STREAMING_DESIGN §4.2;"
    "consumes frozen M04 ABI only"
)
TAG = "g8_geom_page"
WAVE = "G8.4"

CHECK_KEYS = [
    "consumes_frozen_m04_abi",
    "on_demand_residency",
    "unreferenced_pages_not_loaded",
    "root_pages_pinned",
    "late_page_independent_evidence",
    "lru_eviction_under_pressure",
    "device_digest_matches_cpu",
    "validation_zero",
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


def scan_no_local_abi() -> bool:
    """源扫描：harness 不得重定 RXPD/RXPM magic/major（只 import 冻结 decoder）。"""
    if not HARNESS.is_file():
        return False
    src = HARNESS.read_text(encoding="utf-8")
    # 禁止本地重定魔数/ABI 常量
    if re.search(r'b"RXPD"|b"RXPM"', src):
        return False
    if "const DISK_MAJOR" in src or "const MEMORY_MAJOR" in src:
        return False
    return "decode_disk_page" in src and "rurix_geom_pages" in src


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
        "--geom-page",
        "--golden-dir",
        str(GOLDEN_DIR),
    ]
    print(f"[{TAG}] device: --geom-page")
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
    assert SCHEMA_PATH.is_file()
    assert (GOLDEN_DIR / "m04_page0.rxpd").is_file()
    assert len(CHECK_KEYS) == 8
    assert scan_no_local_abi()
    fake = {k: False for k in CHECK_KEYS}
    fake["consumes_frozen_m04_abi"] = True
    assert not all(fake.values())
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

    abi_scan = scan_no_local_abi()
    check(abi_scan, "源扫描：发现本地 ABI 重定或未 import 冻结 decoder")
    note(f"abi_scan={abi_scan}")

    device_state, doc = run_device()
    if device_state == "executed" and doc:
        c = doc.get("checks") or {}
        for k in CHECK_KEYS:
            ok = bool(c.get(k))
            checks[k] = ok
            if not ok:
                check(False, f"device.checks.{k}=false")
        # 源扫描与 runtime mapping 双锚
        checks["consumes_frozen_m04_abi"] = bool(
            checks.get("consumes_frozen_m04_abi") and abi_scan and doc.get("mapping_ok")
        )
        if doc.get("queue_mode") != "single":
            check(False, f"queue_mode={doc.get('queue_mode')!r}")
        if int(doc.get("validation_errors") or 0) != 0:
            checks["validation_zero"] = False
            check(False, "validation_errors!=0")
        if not doc.get("pass"):
            check(False, "device pass=false")
        note(
            f"fallback={doc.get('fallback_frames')} recovered={doc.get('recovered')} "
            f"root_pinned={doc.get('root_pinned')} after={doc.get('resident_after')}"
        )
    elif device_state == "skipped_dev_env":
        for k in CHECK_KEYS:
            checks[k] = False

    # 与 M37 独立：注入参数不同(late=2,delay=4)须在 notes/device 可见
    if doc and int(doc.get("fallback_frames") or 0) >= 1:
        note("late_page_independent: delay/page 与 M37 分离")

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
        "subject": "g8_gate_geom_page",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "GeomPage",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": abi_scan,
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
    out = EVIDENCE_DIR / f"g8_gate_geom_page_{ts}.json"
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
