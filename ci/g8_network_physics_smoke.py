#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.6b M67 network_physics 硬门冒烟(g8.p0.m67.network_physics;
RFC-0021 §4.B1;design §3.5 十三 checks)。

host 恒跑 / device not_applicable。进程内双世界 NetTrace 真跑;
smoothing bound 经 --force-freeze-bound 采样后本地冻结(RFC §6.5.1 /
g8_budget 由 Gov materialize 同 PR 回填)。

用法:
  py -3 ci/g8_network_physics_smoke.py --gate g8.p0.m67.network_physics
  py -3 ci/g8_network_physics_smoke.py --selftest
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
TRACE = (
    ROOT
    / "conformance"
    / "physics"
    / "network"
    / "mispredict_impulse_delay"
    / "trace.json"
)
SCHEMA = ROOT / "milestones" / "g8" / "g8_m67_network_physics_evidence_schema.json"

GATE_KEY = "g8.p0.m67.network_physics"
NUMERIC_STEP = 122
SUBJECT = "g8_m67_network_physics"
SOURCE_REF = (
    "RFC-0021 §4.B1;G8.6_G8.8_PHYSICS_CLOSEOUT_DESIGN.md §3;"
    "G8_ACCEPTANCE_MAP M67"
)

CHECK_KEYS = [
    "trace_fixture_deterministic",
    "prediction_divergence_observed_at_golden",
    "correction_received_at_golden_frame",
    "rollback_start_and_input_sequence_match_expected",
    "resim_final_hash_equals_server",
    "contact_event_committed_exactly_once",
    "event_dedup_across_repeated_rollbacks",
    "smoothing_authoritative_state_untouched",
    "smoothing_within_frozen_bound_per_frame",
    "history_ring_overflow_hard_correction_explicit",
    "incompatible_schema_or_build_digest_rejected",
    "profile_match_recorded",
    "frame_domain_map_recorded",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def utc_stamp() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True, cwd=ROOT)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def gates_exe() -> Path:
    name = "g8-physics-gates.exe" if sys.platform == "win32" else "g8-physics-gates"
    return ROOT / "target" / "debug" / name


def build_gates() -> Path:
    print("[g8_m67] cargo build -p g8-physics-gates")
    r = subprocess.run(
        ["cargo", "build", "-p", "g8-physics-gates", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(r.stdout + r.stderr, file=sys.stderr)
        sys.exit(1)
    exe = gates_exe()
    if not exe.is_file():
        print(f"[g8_m67] missing {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def run_gates(exe: Path, args: list[str]) -> tuple[int, dict | None, str]:
    r = subprocess.run(
        [str(exe), *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    text = (r.stdout or "").strip().splitlines()
    last = text[-1] if text else ""
    doc = None
    try:
        doc = json.loads(last)
    except Exception:
        pass
    return r.returncode, doc, r.stdout + r.stderr


def run_gate() -> int:
    checks = {k: False for k in CHECK_KEYS}
    if not TRACE.is_file():
        print(f"[g8_m67] missing trace {TRACE}", file=sys.stderr)
        return 1

    exe = build_gates()
    print("[g8_m67] net --trace (force-freeze-bound measured)")
    code, doc, out = run_gates(
        exe,
        ["net", "--trace", str(TRACE), "--force-freeze-bound"],
    )
    check(code == 0 and doc is not None and doc.get("ok") is True, f"net failed: {out[-800:]}")
    if not doc:
        doc = {}

    def take(key: str, doc_key: str | None = None) -> bool:
        dk = doc_key or key
        v = bool(doc.get(dk))
        checks[key] = v
        check(v, f"{key} failed")
        return v

    take("trace_fixture_deterministic")
    take("prediction_divergence_observed_at_golden")
    take("correction_received_at_golden_frame")
    take("rollback_start_and_input_sequence_match_expected")
    take("resim_final_hash_equals_server")
    take("contact_event_committed_exactly_once")
    take("event_dedup_across_repeated_rollbacks")
    take("smoothing_authoritative_state_untouched")
    take("smoothing_within_frozen_bound_per_frame")
    take("history_ring_overflow_hard_correction_explicit")
    take(
        "incompatible_schema_or_build_digest_rejected",
        "incompatible_schema_or_build_digest_rejected",
    )
    take("profile_match_recorded")
    take("frame_domain_map_recorded")

    character_state_canonical_ok = bool(doc.get("character_state_canonical_ok"))
    physics_asset_cook_deterministic = bool(doc.get("physics_asset_cook_deterministic"))
    check(character_state_canonical_ok, "character state canonical failed")
    check(physics_asset_cook_deterministic, "physics asset cook failed")

    map_five = {
        "correction_received": checks["correction_received_at_golden_frame"],
        "rollback_resim_sequence": checks[
            "rollback_start_and_input_sequence_match_expected"
        ],
        "resim_hash_equals_server": checks["resim_final_hash_equals_server"],
        "event_commit_exactly_once": checks["contact_event_committed_exactly_once"]
        and checks["event_dedup_across_repeated_rollbacks"],
        "smoothing_bound_and_auth_untouched": checks[
            "smoothing_authoritative_state_untouched"
        ]
        and checks["smoothing_within_frozen_bound_per_frame"],
    }
    for k, v in map_five.items():
        check(v, f"map_five.{k} failed")

    smoothing_bound = {
        "frozen": bool(doc.get("smoothing_bound_frozen")),
        "reference": "RFC-0021 §6.5.1",
        "max_position_m": float(doc.get("max_position_offset_m", 0.0)) * 1.25
        if doc.get("smoothing_bound_frozen")
        else 0.0,
        "max_angle_rad": float(doc.get("max_angle_offset_rad", 0.0)) * 1.25
        if doc.get("smoothing_bound_frozen")
        else 0.0,
        "measured_max_position_m": float(doc.get("max_position_offset_m", 0.0)),
        "measured_max_angle_rad": float(doc.get("max_angle_offset_rad", 0.0)),
    }
    note(
        f"correction_frame={doc.get('correction_frame')} "
        f"rollback_start={doc.get('rollback_start')} "
        f"committed={doc.get('contact_events_committed')}"
    )
    host_pass = all(checks.values()) and all(map_five.values()) and not FAILURES
    stamp = utc_stamp()
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M67",
        "wave": "G8.6b",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": checks,
        "map_five": map_five,
        "smoothing_bound": smoothing_bound,
        # wave6b subject 取证字段(可选;不改 NUMERIC_STEP / 不升格 P0)
        "character_state_canonical_ok": character_state_canonical_ok,
        "physics_asset_cook_deterministic": physics_asset_cook_deterministic,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": tool_version("cargo"),
            "rustc_version": tool_version("rustc"),
        },
        "notes": "; ".join(NOTES) if NOTES else "M67 network physics measured_local",
    }

    try:
        import jsonschema

        errs = sorted(
            jsonschema.Draft7Validator(json.loads(SCHEMA.read_text(encoding="utf-8"))).iter_errors(
                evidence
            ),
            key=lambda e: list(e.path),
        )
        if errs:
            for e in errs:
                FAILURES.append(f"schema: {e.message}")
            host_pass = False
            evidence["host_section_pass"] = False
    except ImportError:
        note("jsonschema missing; skipped local schema validate")

    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out_path = EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    out_path.write_text(json.dumps(evidence, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m67] evidence → {out_path.relative_to(ROOT)}")
    for k, v in checks.items():
        print(f"  check {k}: {'PASS' if v else 'FAIL'}")
    for k, v in map_five.items():
        print(f"  map_five.{k}: {'PASS' if v else 'FAIL'}")
    if FAILURES:
        print("[g8_m67] FAILURES:", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
    print(f"[g8_m67] VERDICT = {'PASS' if host_pass else 'FAIL'}")
    return 0 if host_pass else 1


def run_selftest() -> int:
    exe = build_gates()
    missing = ROOT / "conformance" / "physics" / "network" / "__missing__" / "trace.json"
    code, doc, _ = run_gates(exe, ["net", "--trace", str(missing)])
    if code == 0 and doc and doc.get("ok"):
        print("[selftest] FAIL: missing trace still green", file=sys.stderr)
        return 1
    print("[selftest] PASS: missing trace → red")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G8.6b M67 network_physics smoke")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if NUMERIC_STEP <= 0:
        note("NUMERIC_STEP=0 (Gov materialize 回填前草稿)")
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
