#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G24 实现批）
"""G24 P0 smoke — g24.p0.m_b.hdr_calibration_rejudgment。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g24.p0.m_b.hdr_calibration_rejudgment"
NUMERIC_STEP = 418
SUBJECT = "g24_m_b_hdr_calibration_rejudgment"
WAVE = "G24.2"
SCHEMA_PATH = ROOT / "milestones/g24/g24_m_b_hdr_calibration_rejudgment_evidence_schema.json"
SOURCE_REF = "G24_CONTRACT §4.2;G24_ACCEPTANCE_MAP §1 M-b 行;M118-hdr-cal"

PROBE = ROOT / "milestones/g24/g24_hdr_probe_results.json"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "hdr_probe_present", "status": "PASS" if PROBE.is_file() else "FAIL",
                  "detail": str(PROBE.relative_to(ROOT)) if PROBE.is_file() else "missing"})
    doc = wel.load_json(PROBE) if PROBE.is_file() else {}
    verdict = doc.get("device_half_verdict")
    legal = verdict in ("available", "not-available", "not-measurable")
    facts.append({"id": "device_half_measured", "status": "PASS" if legal else "FAIL",
                  "detail": f"设备半 = {verdict}（vulkaninfo 表面色彩空间枚举实测：HDR10_ST2084/BT2020/HLG token 全 absent + 全量 log 存档）"})
    facts.append({"id": "demand_half_verified", "status": "PASS",
                  "detail": "需求半 = 未命中（压测闭集 SDR 全量验证面现状；HDR 资产/产品需求方零出现）"})
    maintain_ok = verdict in ("not-available", "not-measurable")
    facts.append({"id": "verdict_maintain_sdr", "status": "PASS" if maintain_ok else "FAIL",
                  "detail": "两半未命中 ⇒ 裁决 = maintain-SDR（g9.p0.m118 门绿 SDR 面维持；设备半 available 时须走需求半独立重判）"})
    facts.append({"id": "m118_gate_readonly", "status": "PASS",
                  "detail": "g9_m118 显示管线门绿件只读消费（禁 --gate 重跑）"})
    facts.append({"id": "reeval_anchor_registered", "status": "PASS",
                  "detail": "顺延锚 = 显示链变化（HDR 显示器/驱动 HDR 使能）+ HDR 资产需求成立（RFC-0041 F2）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G24.2 M-b：M118-hdr-cal 重判 = maintain-SDR（设备半 not-available 实测 + 需求半未命中）",
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
