#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.3 M80 ddc_content_address 硬门冒烟(步骤 110;g8.p0.m80.ddc_content_address)。"""
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
CORPUS = ROOT / "conformance" / "asset" / "ddc"

GATE_KEY = "g8.p0.m80.ddc_content_address"
NUMERIC_STEP = 110

CHECK_KEYS = [
    "preimage_covers_source_digest",
    "preimage_covers_dependency_keys",
    "preimage_covers_tool_version",
    "preimage_covers_cook_profile",
    "same_preimage_same_key",
    "put_get_byte_equal",
    "mutation_source_flips_key",
    "mutation_dependency_flips_key",
    "mutation_recipe_flips_key",
    "mutation_profile_flips_key",
    "mutation_toolchain_flips_key",
    "mutation_schema_set_flips_key",
    "mutation_abi_set_flips_key",
    "mutation_artifact_kind_flips_key",
    "mutation_output_id_flips_key",
    "bitflip_rejected_as_corruption",
    "truncation_rejected",
    "concurrent_same_key_put_safe",
    "evict_then_rebuild_key_stable",
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


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print("[g8_m80] selftest PASS" if len(CHECK_KEYS) == 19 else "FAIL")
        return 0 if len(CHECK_KEYS) == 19 else 1

    CORPUS.mkdir(parents=True, exist_ok=True)
    # checked-in mutation note
    note = CORPUS / "mutation_vectors.json"
    if not note.is_file():
        note.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "segments": [
                        "source_set",
                        "dependency_keys",
                        "import_recipe",
                        "cook_profile",
                        "tool_chain",
                        "schema_set",
                        "abi_set",
                        "artifact_kind",
                        "output_id",
                    ],
                    "expectation": "each single-segment mutation must flip DDC key",
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )

    r = run(["cargo", "test", "-p", "rurix-asset", "--lib", "ddc", "--quiet"])
    check(r.returncode == 0, f"cargo test ddc failed:\n{r.stdout}\n{r.stderr}")

    r = run(["cargo", "build", "-p", "rurix-asset", "--bin", "rxcook", "--quiet"])
    if r.returncode != 0:
        print(r.stderr, file=sys.stderr)
        return 1
    exe = ROOT / "target" / "debug" / ("rxcook.exe" if sys.platform == "win32" else "rxcook")
    r = run([str(exe), "ddc-selftest"])
    results = {k: False for k in CHECK_KEYS}
    for line in r.stdout.splitlines():
        line = line.strip().rstrip(",")
        for k in CHECK_KEYS:
            if line.startswith(f'"{k}":'):
                results[k] = "true" in line.split(":", 1)[1]
    if r.returncode != 0:
        check(False, f"ddc-selftest exit {r.returncode}\n{r.stdout}\n{r.stderr}")
    for k in CHECK_KEYS:
        if not results[k]:
            check(False, f"leg red: {k}")

    host_ok = len(FAILURES) == 0 and all(results.values())
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    path = EVIDENCE_DIR / f"g8_m80_ddc_content_address_{stamp}.json"
    doc = {
        "schema_version": 1,
        "subject": "g8_m80_ddc_content_address",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "G8.3",
        "wave": "G8.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": "RFC-0020 §4.3; design §3.2; RXS-0343",
        "host_section_pass": host_ok,
        "device_section_state": "not_applicable",
        "checks": results,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "M80 host DDC CAS; 19 legs",
    }
    path.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m80] evidence 落盘: {path}")
    print("[g8_m80] checks:")
    for k in CHECK_KEYS:
        print(f"  {'PASS' if results[k] else 'FAIL'}: {k}")
    if not host_ok:
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        print("[g8_m80] VERDICT=FAIL")
        return 1
    print("[g8_m80] VERDICT=PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
