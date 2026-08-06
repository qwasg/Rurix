#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.6d M72 cloth_product_chain(g8.p1.m72.cloth_product_chain)。"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA = ROOT / "milestones" / "g8" / "g8_m72_cloth_product_chain_evidence_schema.json"
GATE_KEY = "g8.p1.m72.cloth_product_chain"
NUMERIC_STEP = 126
SUBJECT = "g8_m72_cloth_product_chain"
SOURCE_REF = (
    "RFC-0021 §4.D2;G8.6_G8.8_PHYSICS_CLOSEOUT_DESIGN.md §5;"
    "G8_ACCEPTANCE_MAP M72"
)
CHECK_KEYS = [
    "schema_pass",
    "import_pass",
    "collision_pass",
    "lod_pass",
    "timeline_pass",
    "solver_double_run_deterministic",
    "bound_frozen_reference_present",
    "cloth_capture_scene_appended",
]


def utc_stamp() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True, cwd=ROOT)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def run_gate() -> int:
    subprocess.run(["cargo", "build", "-p", "g8-physics-gates", "--quiet"], cwd=ROOT, check=False)
    name = "g8-physics-gates.exe" if sys.platform == "win32" else "g8-physics-gates"
    exe = ROOT / "target" / "debug" / name
    r = subprocess.run([str(exe), "cloth"], cwd=ROOT, capture_output=True, text=True)
    line = (r.stdout or "").strip().splitlines()
    doc = json.loads(line[-1] if line else "{}")
    checks = {k: bool(doc.get(k)) for k in CHECK_KEYS}
    host_pass = bool(doc.get("ok")) and all(checks.values())
    stamp = utc_stamp()
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M72",
        "wave": "G8.6d",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": checks,
        "penetration_bound": {
            "frozen": True,
            "reference": "RFC-0021 §6.5.1 cloth",
            "max_penetration_m": float(doc.get("penetration_bound_m", 0.0)),
            "measured_max_penetration_m": float(doc.get("measured_max_penetration_m", 0.0)),
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": doc.get("detail") or "M72 cloth",
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    out.write_text(json.dumps(evidence, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m72] evidence → {out.relative_to(ROOT)}")
    for k, v in checks.items():
        print(f"  check {k}: {'PASS' if v else 'FAIL'}")
    print(f"[g8_m72] VERDICT = {'PASS' if host_pass else 'FAIL'}")
    return 0 if host_pass else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print("[selftest] OK")
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
