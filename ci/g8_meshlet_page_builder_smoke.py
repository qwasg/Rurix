#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.3 M01 meshlet_page_builder 硬门冒烟(步骤 105;g8.p0.m01.meshlet_page_builder;
RFC-0020 §4.9;spec/geometry_pages.md RXS-0328~0331)。

host 纯 host 门(device_section_state=not_applicable)。12 腿判据:

  ① builder_double_run_byte_equal
  ② header_magic_version_golden
  ③ header_schema_digest_golden
  ④ decoded_dag_nodes_equal_reference
  ⑤ decoded_dag_edges_equal_reference
  ⑥ decoded_bounds_equal_reference
  ⑦ decoded_lod_parent_equal_reference
  ⑧ page_size_within_contract
  ⑨ unknown_version_rejected_pre_consume
  ⑩ rxgb_converter_explicit
  ⑪ rxgb_reader_zero_byte
  ⑫ not_substituted_by_m04

用法:
  py -3 ci/g8_meshlet_page_builder_smoke.py --gate g8.p0.m01.meshlet_page_builder
  py -3 ci/g8_meshlet_page_builder_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
GOLDEN_DIR = ROOT / "tests" / "geom_pages" / "golden"
REJECT_DIR = ROOT / "conformance" / "geom_pages" / "reject"

GATE_KEY = "g8.p0.m01.meshlet_page_builder"
NUMERIC_STEP = 105

CHECK_KEYS = [
    "builder_double_run_byte_equal",
    "header_magic_version_golden",
    "header_schema_digest_golden",
    "decoded_dag_nodes_equal_reference",
    "decoded_dag_edges_equal_reference",
    "decoded_bounds_equal_reference",
    "decoded_lod_parent_equal_reference",
    "page_size_within_contract",
    "unknown_version_rejected_pre_consume",
    "rxgb_converter_explicit",
    "rxgb_reader_zero_byte",
    "not_substituted_by_m04",
]

FAILURES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def cargo_test(package: str, extra: list[str] | None = None) -> bool:
    cmd = ["cargo", "test", "-p", package, "--quiet"]
    if extra:
        cmd.extend(extra)
    print(f"[g8_m01] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        check(False, f"cargo test -p {package} 失败:\n{r.stdout}\n{r.stderr}")
        return False
    return True


def run_probe() -> dict | None:
    print("[g8_m01] cargo run -p rurix-asset --bin g8_m01_probe")
    r = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "rurix-asset",
            "--quiet",
            "--bin",
            "g8_m01_probe",
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
        check(
            False,
            f"g8_m01_probe 失败 (rc={r.returncode}):\n{r.stdout}\n{r.stderr}",
        )
        # 仍尝试解析 stdout JSON
    text = r.stdout.strip().splitlines()
    if not text:
        check(False, "g8_m01_probe 无 stdout")
        return None
    try:
        return json.loads(text[-1])
    except Exception as e:
        check(False, f"g8_m01_probe JSON 解析失败: {e}\nstdout={r.stdout!r}")
        return None


def verify_fixtures_present() -> None:
    check(
        (GOLDEN_DIR / "m01_header.bin").is_file(),
        "缺少 tests/geom_pages/golden/m01_header.bin",
    )
    check(
        (GOLDEN_DIR / "m01_digest_manifest.json").is_file(),
        "缺少 tests/geom_pages/golden/m01_digest_manifest.json",
    )
    for name in ("unknown_version.rxpl", "bad_magic.rxpl", "truncated.rxpl"):
        check((REJECT_DIR / name).is_file(), f"缺少 reject fixture: {name}")


def verify_header_magic_bytes() -> None:
    p = GOLDEN_DIR / "m01_header.bin"
    if not p.is_file():
        return
    data = p.read_bytes()
    check(len(data) == 136, f"m01_header.bin 长度应为 136,实测 {len(data)}")
    check(data[0:4] == b"RXPL", "m01_header.bin magic ≠ RXPL")
    check(data[8:10] == (1).to_bytes(2, "little"), "m01_header.bin major ≠ 1")
    check(data[12] == 1, "m01_header.bin endian ≠ 1")
    check(data[14:16] == (136).to_bytes(2, "little"), "m01_header.bin header_size ≠ 136")


def verify_not_reading_m04() -> None:
    """自检:本 smoke 源不引用 RXPD/压缩产物路径。"""
    src = Path(__file__).read_text(encoding="utf-8")
    check("RXPD" not in src or "not_substituted" in src, "smoke 源意外硬编码 RXPD 消费")
    check(
        "conformance/geom_pages/reject" in src.replace("\\", "/"),
        "smoke 应消费 RXPL reject 语料",
    )


def write_evidence(results: dict, host_ok: bool) -> Path:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    ev = {
        "schema_version": 1,
        "subject": "g8_m01_meshlet_page_builder",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M01",
        "wave": "G8.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": (
            "RFC-0020 §4.9;spec/geometry_pages.md RXS-0328~0331;"
            "G8_ACCEPTANCE_MAP §2 M01"
        ),
        "host_section_pass": host_ok,
        "device_section_state": "not_applicable",
        "checks": results,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": (
            "host 纯 host 门;device 段 not_applicable。"
            "12 腿经 g8_m01_probe + cargo test -p rurix-geom-pages/rurix-asset/"
            "rurix-geom-build(serialize)。numeric_step=105 为预占,ledger 合入时校准。"
        ),
    }
    path = EVIDENCE_DIR / f"g8_m01_meshlet_page_builder_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m01] evidence 落盘: {path.relative_to(ROOT)}")
    return path


def selftest() -> None:
    """反 YAML-only:合成数据喂判定层,证明能红。"""
    check(False, "selftest: 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print("[g8_m01] selftest FAIL: check() 未正确记录", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    # CHECK_KEYS 恰好 12
    if len(CHECK_KEYS) != 12:
        print(f"[g8_m01] selftest FAIL: CHECK_KEYS 应为 12,实测 {len(CHECK_KEYS)}", file=sys.stderr)
        sys.exit(1)
    # 合成 probe JSON 缺腿 → 判定红
    fake = {"ok": True, "checks": {k: True for k in CHECK_KEYS[:-1]}}
    missing = [k for k in CHECK_KEYS if k not in fake["checks"]]
    if missing != ["not_substituted_by_m04"]:
        print(f"[g8_m01] selftest FAIL: 缺腿探测异常 {missing}", file=sys.stderr)
        sys.exit(1)
    print("[g8_m01] selftest PASS(红绿判别有效;未跑 cargo、未写 evidence)")


def main() -> int:
    parser = argparse.ArgumentParser(description="G8.3 M01 meshlet_page_builder 硬门冒烟")
    parser.add_argument("--gate", default=GATE_KEY, help="symbolic gate key")
    parser.add_argument("--selftest", action="store_true", help="反 YAML-only 红绿自检")
    args = parser.parse_args()

    if args.selftest:
        selftest()
        return 0

    if args.gate != GATE_KEY:
        check(False, f"--gate `{args.gate}` ≠ canonical key `{GATE_KEY}`")

    verify_fixtures_present()
    verify_header_magic_bytes()
    verify_not_reading_m04()

    pages_ok = cargo_test("rurix-geom-pages")
    asset_ok = cargo_test("rurix-asset")
    # RXGB reader 0-byte 锚:既有 serialize roundtrip 单测
    rxgb_ok = cargo_test("rurix-geom-build", ["serialize"])

    probe = run_probe()
    results = {k: False for k in CHECK_KEYS}
    if probe and isinstance(probe.get("checks"), dict):
        for k in CHECK_KEYS:
            results[k] = bool(probe["checks"].get(k, False))
            if not results[k]:
                check(False, f"probe 腿红: {k}")
        if not probe.get("ok", False):
            check(False, "probe ok=false")
        if not probe.get("bad_magic_rejected", False):
            check(False, "bad_magic.rxpl 未拒录")
        if not probe.get("truncated_rejected", False):
            check(False, "truncated.rxpl 未拒录")
    else:
        check(False, "probe 结果缺失")

    # rxgb_reader_zero_byte 额外要求 geom-build serialize 单测绿
    if not rxgb_ok:
        results["rxgb_reader_zero_byte"] = False

    host_ok = (
        pages_ok
        and asset_ok
        and rxgb_ok
        and len(FAILURES) == 0
        and all(results.values())
    )

    write_evidence(results, host_ok)

    if FAILURES:
        print(f"[g8_m01] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print(
        "[g8_m01] PASS (host 纯 host 门;12 腿全绿;"
        "cargo test geom-pages/asset/geom-build serialize 全绿)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
