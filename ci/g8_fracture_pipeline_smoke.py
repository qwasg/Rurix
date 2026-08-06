#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.6c M68 fracture_pipeline 硬门(g8.p0.m68.fracture_pipeline;RFC-0021 §4.C)。

host 恒跑 / device not_applicable。12 checks 全链。

用法:
  py -3 ci/g8_fracture_pipeline_smoke.py --gate g8.p0.m68.fracture_pipeline
  py -3 ci/g8_fracture_pipeline_smoke.py --selftest
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
SOURCE = ROOT / "conformance" / "physics" / "fracture" / "pillar_prefracture" / "source.json"
GOLDEN = ROOT / "conformance" / "physics" / "fracture" / "pillar_prefracture" / "golden.json"
SCHEMA = ROOT / "milestones" / "g8" / "g8_m68_fracture_pipeline_evidence_schema.json"

GATE_KEY = "g8.p0.m68.fracture_pipeline"
NUMERIC_STEP = 124
SUBJECT = "g8_m68_fracture_pipeline"
SOURCE_REF = (
    "RFC-0021 §4.C;G8.6_G8.8_PHYSICS_CLOSEOUT_DESIGN.md §4;"
    "G8_ACCEPTANCE_MAP M68"
)

CHECK_KEYS = [
    "cook_deterministic_double_byte_equal",
    "cook_counts_and_digests_match_golden",
    "unknown_schema_fails_closed",
    "dangling_edge_or_nontree_cluster_fails_closed",
    "below_threshold_no_break",
    "above_threshold_breaks_specified_edge_at_tick",
    "cluster_activation_hierarchy_matches_golden",
    "activated_bodies_enter_journal_and_capture",
    "cache_roundtrip_event_sequence_identical",
    "cache_roundtrip_state_hash_identical",
    "vfx_exactly_once_per_fracture_event",
    "vfx_no_duplicate_across_rollback_or_cache_replay",
]


def utc_stamp() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True, cwd=ROOT)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def build_gates() -> Path:
    print("[g8_m68] cargo build -p g8-physics-gates")
    r = subprocess.run(
        ["cargo", "build", "-p", "g8-physics-gates", "--quiet"],
        cwd=ROOT,
    )
    if r.returncode != 0:
        raise SystemExit("cargo build failed")
    name = "g8-physics-gates.exe" if sys.platform == "win32" else "g8-physics-gates"
    return ROOT / "target" / "debug" / name


def run_gate() -> int:
    exe = build_gates()
    print("[g8_m68] fracture --source/--golden")
    r = subprocess.run(
        [str(exe), "fracture", "--source", str(SOURCE), "--golden", str(GOLDEN)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    line = (r.stdout or "").strip().splitlines()
    last = line[-1] if line else "{}"
    try:
        doc = json.loads(last)
    except json.JSONDecodeError:
        print(f"[g8_m68] bad JSON: {last!r}", file=sys.stderr)
        return 1
    checks = {k: bool(doc.get(k)) for k in CHECK_KEYS}
    host_pass = bool(doc.get("ok")) and all(checks.values()) and r.returncode == 0
    stamp = utc_stamp()
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M68",
        "wave": "G8.6c",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": checks,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": tool_version("cargo"),
            "rustc_version": tool_version("rustc"),
        },
        "notes": doc.get("detail") or "M68 fracture pipeline",
    }
    if SCHEMA.is_file():
        try:
            import jsonschema

            errs = sorted(
                jsonschema.Draft7Validator(
                    json.loads(SCHEMA.read_text(encoding="utf-8"))
                ).iter_errors(evidence),
                key=lambda e: list(e.path),
            )
            if errs:
                host_pass = False
                evidence["host_section_pass"] = False
                evidence["notes"] += "; schema " + errs[0].message
        except ImportError:
            pass
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    out.write_text(json.dumps(evidence, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m68] evidence → {out.relative_to(ROOT)}")
    for k, v in checks.items():
        print(f"  check {k}: {'PASS' if v else 'FAIL'}")
    print(f"[g8_m68] VERDICT = {'PASS' if host_pass else 'FAIL'}")
    return 0 if host_pass else 1


def run_selftest() -> int:
    exe = build_gates()
    missing = ROOT / "conformance" / "physics" / "fracture" / "__missing__" / "source.json"
    r = subprocess.run(
        [str(exe), "fracture", "--source", str(missing), "--golden", str(GOLDEN)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode == 0:
        print("[selftest] FAIL: missing source green", file=sys.stderr)
        return 1
    print("[selftest] PASS: missing source → red")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
