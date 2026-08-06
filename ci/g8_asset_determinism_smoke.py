#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.3 M79 asset_determinism 硬门冒烟(步骤合入时领取;g8.p0.m79.asset_determinism;
RFC-0020 §4.2/§4.6;spec/asset_pipeline.md RXS-0335~0337)。

host 纯 host 门。checks.* 12 项(设计案 §3.1)。

用法:
  py -3 ci/g8_asset_determinism_smoke.py --gate g8.p0.m79.asset_determinism
  py -3 ci/g8_asset_determinism_smoke.py --selftest
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
ACCEPT = ROOT / "conformance" / "asset" / "canon" / "accept"
REJECT = ROOT / "conformance" / "asset" / "canon" / "reject"
GRAPH_REJECT = ROOT / "conformance" / "asset" / "graph" / "reject"

GATE_KEY = "g8.p0.m79.asset_determinism"
NUMERIC_STEP = 108  # ledger next_free at materialize; Gov 校准

CHECK_KEYS = [
    "canon_golden_byte_equal",
    "canon_reject_corpus_fail_closed",
    "double_build_isolated_roots",
    "double_build_dag_byte_equal",
    "double_build_artifacts_byte_equal",
    "double_build_manifest_digest_equal",
    "mutate_dependency_flips_key",
    "mutate_recipe_flips_key",
    "mutate_profile_flips_key",
    "mutate_tool_version_flips_key",
    "unrelated_nodes_keys_stable",
    "no_env_time_path_in_signed_bytes",
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


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)


def ensure_fixtures() -> None:
    print("[g8_m79] cargo run -p rurix-asset --bin g8_m79_write_fixtures")
    r = run(["cargo", "run", "-p", "rurix-asset", "--bin", "g8_m79_write_fixtures", "--quiet"])
    if r.returncode != 0:
        print(r.stdout + r.stderr, file=sys.stderr)
        sys.exit(1)


def build_rxcook() -> Path:
    print("[g8_m79] cargo build -p rurix-asset --bin rxcook")
    r = run(["cargo", "build", "-p", "rurix-asset", "--bin", "rxcook", "--quiet"])
    if r.returncode != 0:
        print(r.stdout + r.stderr, file=sys.stderr)
        sys.exit(1)
    exe = ROOT / "target" / "debug" / ("rxcook.exe" if sys.platform == "win32" else "rxcook")
    if not exe.is_file():
        sys.exit(1)
    return exe


def parse_kv(text: str) -> dict[str, str]:
    out: dict[str, str] = {}
    for line in text.splitlines():
        if "=" in line and not line.strip().startswith("{"):
            k, _, v = line.partition("=")
            out[k.strip()] = v.strip()
    return out


def write_evidence(results: dict[str, bool], host_ok: bool) -> Path:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    path = EVIDENCE_DIR / f"g8_m79_asset_determinism_{stamp}.json"
    doc = {
        "schema_version": 1,
        "subject": "g8_m79_asset_determinism",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "G8.3",
        "wave": "G8.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": "RFC-0020 §4.2/§4.6; design §3.1; RXS-0335~0337",
        "host_section_pass": host_ok,
        "device_section_state": "not_applicable",
        "checks": {k: bool(results.get(k)) for k in CHECK_KEYS},
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "M79 host: canon corpus + double-build + mutations; device N/A",
    }
    path.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m79] evidence 落盘: {path}")
    return path


def run_gate() -> int:
    ensure_fixtures()
    exe = build_rxcook()

    r = run(["cargo", "test", "-p", "rurix-asset", "--lib", "canon::", "--quiet"])
    # filter may fail on windows quoting; fallback
    if r.returncode != 0:
        r = run(["cargo", "test", "-p", "rurix-asset", "--lib", "canon", "--quiet"])
    check(r.returncode == 0, "cargo test canon failed")

    r = run(["cargo", "test", "-p", "rurix-asset", "--lib", "graph", "--quiet"])
    check(r.returncode == 0, "cargo test graph failed")
    check(GRAPH_REJECT.is_dir() and any(GRAPH_REJECT.iterdir()), "graph reject corpus missing")

    results = {k: False for k in CHECK_KEYS}

    # canon corpus
    r = run(
        [
            str(exe),
            "canon-check",
            "--accept",
            str(ACCEPT),
            "--reject",
            str(REJECT),
        ]
    )
    kv = parse_kv(r.stdout)
    canon_ok = r.returncode == 0 and kv.get("ok") == "true"
    results["canon_golden_byte_equal"] = canon_ok
    results["canon_reject_corpus_fail_closed"] = canon_ok
    if not canon_ok:
        check(False, f"canon-check failed:\n{r.stdout}\n{r.stderr}")

    # double-build + mutations
    r = run([str(exe), "verify", "--double-build", "--workspace", str(ROOT)])
    checks: dict[str, bool] = {}
    for line in r.stdout.splitlines():
        line = line.strip().rstrip(",")
        for k in CHECK_KEYS:
            prefix = f'"{k}":'
            if line.startswith(prefix):
                checks[k] = "true" in line.split(":", 1)[1]
    if r.returncode != 0:
        check(False, f"verify exit {r.returncode}:\n{r.stdout}\n{r.stderr}")
    for k in CHECK_KEYS:
        if k.startswith("double_") or k.startswith("mutate_") or k in (
            "unrelated_nodes_keys_stable",
            "no_env_time_path_in_signed_bytes",
        ):
            results[k] = bool(checks.get(k, False))
            if not results[k]:
                check(False, f"verify leg red: {k}")

    host_ok = len(FAILURES) == 0 and all(results.values())
    write_evidence(results, host_ok)

    print("[g8_m79] checks:")
    for k in CHECK_KEYS:
        print(f"  {'PASS' if results[k] else 'FAIL'}: {k}")

    if not host_ok:
        print(f"[g8_m79] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        print("[g8_m79] VERDICT=FAIL")
        return 1
    print("[g8_m79] VERDICT=PASS")
    return 0


def selftest() -> int:
    if len(CHECK_KEYS) != 12:
        print("CHECK_KEYS != 12", file=sys.stderr)
        return 1
    print("[g8_m79] selftest PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
