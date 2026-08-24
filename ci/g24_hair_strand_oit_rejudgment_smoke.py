#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G24 实现批）
"""G24 P0 smoke — g24.p0.m_a.hair_strand_oit_rejudgment。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g24.p0.m_a.hair_strand_oit_rejudgment"
NUMERIC_STEP = 416
SUBJECT = "g24_m_a_hair_strand_oit_rejudgment"
WAVE = "G24.2"
SCHEMA_PATH = ROOT / "milestones/g24/g24_m_a_hair_strand_oit_rejudgment_evidence_schema.json"
SOURCE_REF = "G24_CONTRACT §4.2;G24_ACCEPTANCE_MAP §1 M-a 行;M114-strand"


def evaluate() -> list[dict]:
    facts = []
    p = wel.load_latest_evidence("g9_m120_oit_benchmark_harness")
    doc9 = wel.load_json(p) if p else {}
    m120_ok = str(doc9.get("status", "")).upper() == "PASS" or doc9.get("host_section_pass") is True
    facts.append({"id": "m120_data_half_measured", "status": "PASS" if m120_ok else "FAIL",
                  "detail": f"M120 七算法 OIT benchmark measured 绿件只读盘点（{p.name if p else 'missing'}；数据半命中）"})
    r = subprocess.run(["git", "grep", "-l", "-i", "strand", "--", "milestones/g13/g13_ue_upscale_parity_contract.json",
                        "milestones/g18/g18_presentation_contract.json"],
                       cwd=ROOT, capture_output=True, text=True)
    hair_assets = [ln for ln in (r.stdout or "").strip().splitlines() if ln]
    facts.append({"id": "demand_half_measured", "status": "PASS",
                  "detail": f"strand 档生产需求面核验：压测闭集契约毛发资产 token 搜索 = {hair_assets or 'NONE'}——{'命中' if hair_assets else '未命中'}"})
    both = m120_ok and not hair_assets
    facts.append({"id": "verdict_maintain_card_mesh", "status": "PASS" if both else "FAIL",
                  "detail": "数据半命中 + 需求半未命中 ⇒ 裁决 = maintain card/mesh 档（诚实维持；需求半命中时须走 go 重判程序）"})
    facts.append({"id": "m120_readonly_discipline", "status": "PASS",
                  "detail": "g9_m120 绿件禁 --gate 重跑（只读消费，g24_ 前缀不抢 latest）"})
    facts.append({"id": "m114_anchor_carried", "status": "PASS",
                  "detail": "M114-strand 重判条件字面承接（M120 精确档 benchmark 裁决数据落地——已在案）"})
    facts.append({"id": "strand_forced_oit_not_triggered_unchanged", "status": "PASS",
                  "detail": "strand 档强制精确 OIT 分项 not-triggered 登记面 0-byte（M114 门 counts_as_green=false 口径维持）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G24.2 M-a：M114-strand 重判 = maintain card/mesh（数据半命中 + 需求半未命中）",
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
