#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.6d wave6d.exit：M72 + m70.vehicle subject(六腿逐腿断言,禁单 bool 自指)。"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.6d.exit"
NUMERIC_STEP = 127
SUBJECT = "g8_wave6d_exit"
WAVE = "G8.6d"
SOURCE_REF = "CI_GATES §5;M72 cloth + M70 vehicle subject(6-leg)"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave6d_exit_evidence_schema.json"
REQUIRED = [("g8.p1.m72.cloth_product_chain", "g8_m72_cloth_product_chain")]

# G8.6_G8.8_PHYSICS_CLOSEOUT_DESIGN.md §5.3 M70 subject 六腿(逐腿断言)。
VEHICLE_LEGS = [
    "asset_roundtrip",
    "fixed_input_replay_hash_equal",
    "rollback_correction_converges",
    "tire_light_object_contact_regression_golden",
    "state_serialization_roundtrip",
    "telemetry_trace_golden",
]


def vehicle_exe() -> Path:
    name = "g8-physics-gates.exe" if sys.platform == "win32" else "g8-physics-gates"
    exe = ROOT / "target" / "debug" / name
    if not exe.is_file():
        subprocess.run(["cargo", "build", "-p", "g8-physics-gates", "--quiet"], cwd=ROOT)
    return exe


def run_vehicle(extra: list[str]) -> dict:
    r = subprocess.run([str(vehicle_exe()), "vehicle", *extra], cwd=ROOT, capture_output=True, text=True)
    lines = (r.stdout or "").strip().splitlines()
    try:
        return json.loads(lines[-1]) if lines else {}
    except Exception:
        return {}


def legs_from_doc(doc: dict) -> dict:
    """逐腿布尔(缺失腿一律 False,不得退回单 bool)。"""
    return {k: bool(doc.get(k)) for k in VEHICLE_LEGS}


def subject_ok_from_doc(doc: dict) -> bool:
    return bool(doc.get("ok")) and all(legs_from_doc(doc).values())


def vehicle_subject() -> dict:
    doc = run_vehicle([])
    legs = legs_from_doc(doc)
    ok = subject_ok_from_doc(doc)
    failing = [k for k, v in legs.items() if not v]
    detail = f"legs {sum(legs.values())}/{len(VEHICLE_LEGS)}"
    if failing:
        detail += f" failing={failing}"
    return {
        "id": "g8.wave6d.m70.vehicle",
        "status": "PASS" if ok else "FAIL",
        "detail": detail,
        "evidence_path": "apps/g8-physics-gates vehicle",
        "legs": legs,
        "final_state_hash": doc.get("final_state_hash") or "",
        "contact_digest": doc.get("contact_digest") or "",
        "telemetry_digest": doc.get("telemetry_digest") or "",
    }


def run_gate(*, evidence_dir=None) -> int:
    if NUMERIC_STEP <= 0:
        return 1
    rows = [wel.require_gate_pass(k, p, evidence_dir=evidence_dir) for k, p in REQUIRED]
    subj = vehicle_subject()
    extras = [
        {
            "id": "cloth_five_map",
            "status": "PASS" if rows and rows[0]["status"] == "PASS" else "FAIL",
            "detail": "M72 five MAP + supports",
        },
        {
            "id": "vehicle_six_leg_per_leg_asserted",
            "status": "PASS" if len(subj.get("legs") or {}) == len(VEHICLE_LEGS) else "FAIL",
            "detail": "m70.vehicle 逐腿布尔断言(非单 bool 自指)",
        },
    ]
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=rows,
        extra_facts=extras,
        subjects=[subj],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="wave6d: M72 + m70.vehicle 六腿诚实 subject(thin-shell 单 bool 已清零)",
        host_section_pass=True,
    )
    return code


def selftest() -> int:
    """逐腿证伪:① 真二进制 --falsify 逐腿摄动 → 该腿必须转 False;
    ② 聚合输入逐腿翻 False → 逐腿断言必须红(防单 bool 自指充绿)。"""
    ok = True
    if NUMERIC_STEP != 127:
        print("[selftest] FAIL: NUMERIC_STEP != 127", file=sys.stderr)
        ok = False
    # 臂 1:真 sim 摄动(每腿一条 RED 臂)。
    for leg in VEHICLE_LEGS:
        doc = run_vehicle(["--falsify", leg])
        res = doc.get("leg_result")
        arm_ok = doc.get("ok") is True and res is False
        print(f"[selftest] falsify {leg}: {'PASS' if arm_ok else 'FAIL'} (leg_result={res!r})")
        ok = ok and arm_ok
    # 臂 2:逐腿篡改聚合输入 → 断言必须红。
    doc = run_vehicle([])
    if not subject_ok_from_doc(doc):
        print(f"[selftest] FAIL: 基线未全绿 legs={legs_from_doc(doc)}", file=sys.stderr)
        ok = False
    for leg in VEHICLE_LEGS:
        tampered = dict(doc)
        tampered[leg] = False
        if subject_ok_from_doc(tampered):
            print(f"[selftest] FAIL: 篡改 {leg} 聚合仍绿", file=sys.stderr)
            ok = False
        else:
            print(f"[selftest] aggregate-red {leg}: PASS")
    print(f"[selftest] {'OK' if ok else 'FAIL'}")
    return 0 if ok else 1


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
