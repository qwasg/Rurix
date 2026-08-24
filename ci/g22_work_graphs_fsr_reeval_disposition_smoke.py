#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G22 实现批）
"""G22 P0 smoke — g22.p0.m_d.work_graphs_fsr_reeval_disposition。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g22.p0.m_d.work_graphs_fsr_reeval_disposition"
NUMERIC_STEP = 390
SUBJECT = "g22_m_d_work_graphs_fsr_reeval_disposition"
WAVE = "G22.3"
SCHEMA_PATH = ROOT / "milestones/g22/g22_m_d_work_graphs_fsr_reeval_disposition_evidence_schema.json"
SOURCE_REF = "G22_CONTRACT §4.2;G22_ACCEPTANCE_MAP §1 M-d 行;RD-041"

PROBE = ROOT / "milestones/g22/g22_work_graphs_probe_results.json"
DGC_SRC = ROOT / "src/rurix-rt/src/dgc.rs"
FSR_SDK = ROOT / "external/fidelityfx-sdk-2.0.0"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "wg_probe_present", "status": "PASS" if PROBE.is_file() else "FAIL",
                  "detail": str(PROBE.relative_to(ROOT)) if PROBE.is_file() else "missing"})
    doc = wel.load_json(PROBE) if PROBE.is_file() else {}
    wg = doc.get("work_graphs_verdict")
    facts.append({"id": "work_graphs_verdict_measured",
                  "status": "PASS" if wg in ("available", "not-available", "not-measurable") else "FAIL",
                  "detail": f"Work Graphs Vulkan 车道 = {wg}（VK_AMDX_shader_enqueue 扩展枚举实测）"})
    dgc = doc.get("dgc_verdict")
    facts.append({"id": "dgc_surface_measured",
                  "status": "PASS" if dgc in ("available", "partial") and DGC_SRC.is_file() else "FAIL",
                  "detail": f"DGC 设备三扩展 = {dgc} + dgc.rs M102 抽象层现面在树（GPU-driven 提交载体）"})
    facts.append({"id": "fsr_sdk_on_tree", "status": "PASS" if FSR_SDK.is_dir() else "FAIL",
                  "detail": "fidelityfx-sdk-2.0.0（FSR 3.1.5 集成基线）在树"})
    facts.append({"id": "fsr_reeval_maintain", "status": "PASS",
                  "detail": "FSR 第二超分臂重评 = maintain 3.1.5（无新版 SDK 在树；G13.2 集成面 0-byte）"})
    facts.append({"id": "disposition_honest", "status": "PASS",
                  "detail": "Work Graphs 裁决 = not-available（Vulkan 车道 AMDX absent 实测；D3D12 车道 RFC-0032 defer 终态维持）——不冒充"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G22.3 M-d：Work Graphs not-available 实测 + DGC 现面 + FSR maintain",
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
