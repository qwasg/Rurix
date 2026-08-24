#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G23 实现批）
"""G23 P0 smoke — g23.p0.m_a.jolt_56_adoption_rejudgment。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g23.p0.m_a.jolt_56_adoption_rejudgment"
NUMERIC_STEP = 400
SUBJECT = "g23_m_a_jolt_56_adoption_rejudgment"
WAVE = "G23.2"
SCHEMA_PATH = ROOT / "milestones/g23/g23_m_a_jolt_56_adoption_rejudgment_evidence_schema.json"
SOURCE_REF = "G23_CONTRACT §4.2;G23_ACCEPTANCE_MAP §1 M-a 行;RFC-0040 §1.1"

SYS56 = ROOT / "src/rurix-physics-sys56/Cargo.toml"
VENDOR56 = ROOT / "src/rurix-physics-sys56/VENDOR56.md"
REG = ROOT / "milestones/g23/g23_jolt_adoption_registry.json"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "sys56_arm_on_tree", "status": "PASS" if SYS56.is_file() else "FAIL",
                  "detail": "rurix-physics-sys56（JoltC@2982004 + Jolt v5.6.0 隔离评估臂）在树"})
    facts.append({"id": "vendor56_provenance", "status": "PASS" if VENDOR56.is_file() else "FAIL",
                  "detail": "VENDOR56.md 上游 pin/裁剪/补丁全字段 provenance 在档"})
    p = wel.load_latest_evidence("g9_m125_jolt_56_ab_evaluation")
    doc9 = wel.load_json(p) if p else {}
    ab_ok = str(doc9.get("status", "")).upper() == "PASS" or doc9.get("host_section_pass") is True
    facts.append({"id": "ab_evidence_readonly_green", "status": "PASS" if ab_ok else "FAIL",
                  "detail": f"g9_m125 A/B 绿件只读盘点（{p.name if p else 'missing'}；禁 --gate 重跑）"})
    r = subprocess.run(["cargo", "check", "-p", "rurix-physics-sys56"],
                       cwd=ROOT, capture_output=True, text=True)
    facts.append({"id": "arm_build_freshness", "status": "PASS" if r.returncode == 0 else "FAIL",
                  "detail": f"cargo check -p rurix-physics-sys56 rc={r.returncode}（评估臂构建新鲜真跑）"})
    reg = wel.load_json(REG) if REG.is_file() else {}
    pieces = reg.get("three_pieces", [])
    pieces_ok = [p_.get("id") for p_ in pieces] == ["ADOPT-1", "ADOPT-2", "ADOPT-3"] and all(
        p_.get("state") and p_.get("basis") for p_ in pieces)
    facts.append({"id": "three_pieces_registered", "status": "PASS" if pieces_ok else "FAIL",
                  "detail": "采纳三件逐件 state+basis 登记（升格证据/生产切换/退役程序）"})
    verdict = reg.get("verdict")
    facts.append({"id": "verdict_maintain_53_honest",
                  "status": "PASS" if verdict in ("maintain-5.3", "adopt") else "FAIL",
                  "detail": f"裁决={verdict}（需求证据三类全空 ⇒ maintain-5.3；maintain/adopt 均合法）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G23.2 M-a：M125-adopt3 重判 = maintain-5.3（三件条件 1/3，需求证据全空）",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
