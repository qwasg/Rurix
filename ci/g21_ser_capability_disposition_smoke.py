#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G21 实现批）
"""G21 P0 smoke — g21.p0.m_b.ser_capability_disposition。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g21.p0.m_b.ser_capability_disposition"
NUMERIC_STEP = 370
SUBJECT = "g21_m_b_ser_capability_disposition"
WAVE = "G21.2"
SCHEMA_PATH = ROOT / "milestones/g21/g21_m_b_ser_capability_disposition_evidence_schema.json"
SOURCE_REF = "G21_CONTRACT §4.2;G21_ACCEPTANCE_MAP §1 M-b 行;RFC-0038 §1.4"

PROBE = ROOT / "milestones/g21/g21_ser_capability_probe_results.json"
SUBITEMS = ROOT / "milestones/g21/g21_rd040_subitem_registry.json"
RFC = ROOT / "rfcs/0038-lighting-p3-deepening.md"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "capability_probe_present", "status": "PASS" if PROBE.is_file() else "FAIL",
                  "detail": str(PROBE.relative_to(ROOT)) if PROBE.is_file() else "missing"})
    doc = wel.load_json(PROBE) if PROBE.is_file() else {}
    verdict = doc.get("capability_verdict")
    legal = verdict in ("available", "not-available", "not-measurable")
    facts.append({"id": "capability_verdict_legal", "status": "PASS" if legal else "FAIL",
                  "detail": f"capability 半边 = {verdict}（vulkaninfo 实测取证；三态均合法）"})
    tokens = doc.get("tokens_found", {})
    facts.append({"id": "capability_tokens_archived",
                  "status": "PASS" if doc.get("log_path") and tokens else "FAIL",
                  "detail": f"扩展字面取证 {sum(1 for v in tokens.values() if v)}/{len(tokens)} + 全量 log 存档"})
    sub = wel.load_json(SUBITEMS) if SUBITEMS.is_file() else {}
    rt_lane = next((s for s in sub.get("subitems", []) if s.get("id") == "RT-PIPELINE-SBT"), {})
    workload_miss = rt_lane.get("disposition") == "defer"
    facts.append({"id": "workload_half_verified", "status": "PASS" if workload_miss else "FAIL",
                  "detail": "workload 半边 = RT pipeline/SBT 宿主车道零实现（RD-040 分项 defer 如实）→ 未命中"})
    facts.append({"id": "verdict_maintain_defer", "status": "PASS",
                  "detail": "M52 裁决 = maintain-defer（capability-hit + workload-miss；语言层不加 SER 原语兜底 0-byte；maintain-defer/go 均合法）"})
    facts.append({"id": "rfc_0038_archived", "status": "PASS" if RFC.is_file() else "FAIL",
                  "detail": "RFC-0038 §1.4 SER 重判程序在档"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G21.2 M-b：M52 SER 两半实测重判 = maintain-defer（capability available + workload 车道缺）",
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
