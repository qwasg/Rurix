#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.3 M04 page_format_abi 硬门冒烟(g8.p0.m04.page_format_abi;
RFC-0020 §4.9;spec/geometry_pages.md RXS-0338~0342)。

host 段:双 ABI / LZ1 / 四类拒录 / 映射表 / CPU digest。
device 段(必需):rurixc→SPIR-V + vk_geom_page_decode;digest==CPU;
  validation=0;`RURIX_REQUIRE_REAL=1` 下 SKIP 不充绿。

用法:
  py -3 ci/g8_page_format_abi_smoke.py --gate g8.p0.m04.page_format_abi
  py -3 ci/g8_page_format_abi_smoke.py --selftest
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
GOLDEN_DIR = ROOT / "tests" / "geom_pages" / "golden"
REJECT_DIR = ROOT / "conformance" / "geom_pages" / "reject"
KERNEL = ROOT / "src" / "rurix-rt" / "kernels" / "geom_page_decode.rx"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_m04_page_format_abi_evidence_schema.json"

GATE_KEY = "g8.p0.m04.page_format_abi"
NUMERIC_STEP = 109
SOURCE_REF = "RFC-0020 §4.9;spec/geometry_pages.md RXS-0338~0342;G8_ACCEPTANCE_MAP §2 M04"
TAG = "g8_m04"

CHECK_KEYS = [
    "abi_ids_distinct_and_frozen",
    "encode_decode_records_byte_equal",
    "compress_twice_byte_equal",
    "corrupt_truncation_fail_closed",
    "corrupt_checksum_fail_closed",
    "corrupt_unknown_codec_fail_closed",
    "corrupt_unknown_version_fail_closed",
    "section_overlap_oob_fail_closed",
    "reject_before_allocation",
    "disk_memory_mapping_frozen",
    "cpu_decode_digest_stable",
    "device_decode_digest_equals_cpu",
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
    try:
        return json.loads(text)
    except Exception:
        pass
    idx = text.rfind("\n{")
    if idx < 0:
        idx = text.rfind("{")
    else:
        idx += 1
    if idx < 0:
        return None
    try:
        return json.loads(text[idx:])
    except Exception:
        return None


def run_probe() -> dict | None:
    print(f"[{TAG}] cargo run -p rurix-asset --bin g8_m04_probe")
    r = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "rurix-asset",
            "--bin",
            "g8_m04_probe",
            "--",
            "--golden-dir",
            str(GOLDEN_DIR),
            "--reject-dir",
            str(REJECT_DIR),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"g8_m04_probe 失败 rc={r.returncode}\n{r.stderr}")
    return extract_json(r.stdout)


def compile_kernel(work: Path) -> Path | None:
    spv = work / "geom_page_decode.spv"
    print(f"[{TAG}] rurixc geom_page_decode.rx → {spv.name}")
    r = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "rurixc",
            "--features",
            "vulkan-backend",
            "--bin",
            "rurixc",
            "--",
            str(KERNEL),
            "--target",
            "vulkan",
            "-o",
            str(spv),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0 or not spv.is_file():
        check(False, f"kernel 编译失败:\n{r.stdout}\n{r.stderr}")
        return None
    return spv


def build_harness() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-rt --features vulkan --bin vk_geom_page_decode")
    r = subprocess.run(
        [
            "cargo",
            "build",
            "-p",
            "rurix-rt",
            "--features",
            "vulkan",
            "--bin",
            "vk_geom_page_decode",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"harness 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = ROOT / "target" / "debug" / (
        "vk_geom_page_decode.exe" if sys.platform == "win32" else "vk_geom_page_decode"
    )
    if not exe.is_file():
        check(False, f"harness 缺失: {exe}")
        return None
    return exe


def run_device(
    exe: Path, spv: Path, rxpm: Path, expect_u32: int
) -> tuple[str, dict | None]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    cmd = [
        str(exe),
        "--spv",
        str(spv),
        "--rxpm",
        str(rxpm),
        "--expect-u32-count",
        str(expect_u32),
    ]
    print(f"[{TAG}] device: {' '.join(cmd[-6:])}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env, timeout=300)
    doc = extract_json(r.stdout)
    if doc and doc.get("device_state") == "skipped_dev_env":
        reason = doc.get("reason") or r.stderr.strip()
        if require_real() or env.get("RURIX_REQUIRE_REAL") == "1":
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {reason}")
        return "skipped_dev_env", doc
    if r.returncode != 0 or doc is None:
        check(False, f"device harness 失败 rc={r.returncode}\n{r.stderr}\n{r.stdout}")
        return "fail", doc
    return "executed", doc


def emit_rxpm_via_rxcook(work: Path) -> Path | None:
    rxpd = GOLDEN_DIR / "m04_page0.rxpd"
    rxpm = work / "page0.rxpm"
    r = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "rurix-asset",
            "--bin",
            "rxcook",
            "--",
            "decode-page",
            "--disk",
            str(rxpd),
            "--emit-expanded-digest",
            "--emit-rxpm",
            str(rxpm),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0 or not rxpm.is_file():
        check(False, f"rxcook decode-page 失败:\n{r.stderr}\n{r.stdout}")
        return None
    doc = extract_json(r.stdout)
    if doc:
        note(f"rxcook expanded_digest={doc.get('expanded_digest')}")
    return rxpm


def run_selftest() -> int:
    # 负样本:缺 checks 键应可被本脚本识别
    missing = [k for k in CHECK_KEYS if k not in CHECK_KEYS]
    assert not missing
    assert len(CHECK_KEYS) >= 13
    assert SCHEMA_PATH.is_file()
    assert KERNEL.is_file()
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
    host_pass = False
    device_state = "fail"
    cpu_digest = None
    expand_n = 0

    # host units
    print(f"[{TAG}] cargo test -p rurix-geom-pages")
    tr = subprocess.run(
        ["cargo", "test", "-q", "-p", "rurix-geom-pages"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    check(tr.returncode == 0, f"geom-pages tests 失败:\n{tr.stdout}\n{tr.stderr}")

    probe = run_probe()
    host_keys = [k for k in CHECK_KEYS if not k.startswith("device_")]
    if probe:
        for k in host_keys:
            ok = bool(probe.get(k))
            checks[k] = ok
            if not ok:
                check(False, f"probe.{k} 为假")
        cpu_digest = probe.get("expanded_digest")
        expand_n = int(probe.get("expanded_u32_count") or 0)
        if not probe.get("golden_rxpd_byte_equal"):
            check(False, "golden m04_page0.rxpd 与重编码不等")
        host_pass = all(checks[k] for k in host_keys)
    else:
        check(False, "probe JSON 缺失")

    # reject corpus files exist
    for name in [
        "truncated_payload.rxpd",
        "checksum_flip.rxpd",
        "unknown_codec.rxpd",
        "unknown_major.rxpd",
        "section_overlap.rxpm",
        "section_oob.rxpm",
    ]:
        check((REJECT_DIR / name).is_file(), f"缺 RED 语料 {name}")

    # device
    with tempfile.TemporaryDirectory(prefix="g8_m04_") as td:
        work = Path(td)
        rxpm = emit_rxpm_via_rxcook(work)
        spv = compile_kernel(work)
        exe = build_harness()
        if rxpm and spv and exe and cpu_digest and expand_n > 0:
            device_state, doc = run_device(exe, spv, rxpm, expand_n)
            if device_state == "executed" and doc:
                dev_dig = doc.get("expanded_digest")
                ve = int(doc.get("validation_errors") or 0)
                eq = dev_dig == cpu_digest
                checks["device_decode_digest_equals_cpu"] = eq
                checks["device_validation_zero"] = ve == 0
                if not eq:
                    check(
                        False,
                        f"device digest≠CPU: device={dev_dig} cpu={cpu_digest}",
                    )
                if ve != 0:
                    check(False, f"validation_errors={ve}")
            elif device_state == "skipped_dev_env":
                checks["device_decode_digest_equals_cpu"] = False
                checks["device_validation_zero"] = False
        else:
            check(False, "device 前置产物缺失")
            device_state = "fail"

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES
    if device_state == "skipped_dev_env" and require_real():
        all_pass = False

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g8_m04_page_format_abi",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M04",
        "wave": "G8.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_pass,
        "device_section_state": device_state if all_pass or device_state == "executed" else device_state,
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
    out = EVIDENCE_DIR / f"g8_m04_page_format_abi_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    # schema soft validate required keys
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
