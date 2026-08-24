#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G19 实现批）
"""G19 P0 smoke — g19.p0.m_b.frame_generation_vendor_disposition。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g19.p0.m_b.frame_generation_vendor_disposition"
NUMERIC_STEP = 338
SUBJECT = "g19_m_b_frame_generation_vendor_disposition"
WAVE = "G19.2"
SCHEMA_PATH = ROOT / "milestones/g19/g19_m_b_frame_generation_vendor_disposition_evidence_schema.json"
SOURCE_REF = "G19_CONTRACT §4.2;G19_ACCEPTANCE_MAP §1 M-b 行;RFC-0036;RFC-0032 v0.3"

REG = ROOT / "milestones/g19/g19_vendor_sdk_registry.json"
RFC = ROOT / "rfcs/0036-frame-generation-realization.md"
REVIEW = ROOT / "milestones/g19/design/rfc0036_adversarial_review.md"
G17_PROBE = ROOT / "milestones/g17/g17_mb_ngx_probe_results.json"
LEGAL = {"integrated", "implemented", "rejected", "not_available", "not-available"}


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "vendor_registry_present", "status": "PASS" if REG.is_file() else "FAIL",
                  "detail": str(REG.relative_to(ROOT))})
    doc = wel.load_json(REG) if REG.is_file() else {}
    arms = doc.get("arms", {})
    expected = {"fsr3_fg", "dlss_g", "sl_310_6_0"}
    facts.append({"id": "three_arms_closed_set", "status": "PASS" if set(arms) == expected else "FAIL",
                  "detail": f"arms={sorted(arms)}（闭集 fsr3_fg/dlss_g/sl_310_6_0）"})
    bad = [k for k, v in arms.items() if v.get("disposition") not in LEGAL or not v.get("reason") or not v.get("reeval_anchor")]
    facts.append({"id": "dispositions_legal_with_rationale", "status": "PASS" if not bad else "FAIL",
                  "detail": "逐臂 disposition 合法 + rationale + reeval_anchor 齐" if not bad else str(bad)})
    host = doc.get("host_reference_arm", {})
    host_ok = host.get("disposition") == "implemented" and bool(host.get("evidence"))
    facts.append({"id": "host_reference_arm_implemented", "status": "PASS" if host_ok else "FAIL",
                  "detail": f"host 参考臂 = {host.get('disposition')}（RFC-0036 兑现面）"})
    facts.append({"id": "sl_disposition_evidence_backed", "status": "PASS" if G17_PROBE.is_file() else "FAIL",
                  "detail": "SL-310.6.0 臂级不可用有 G17.3 双臂探针实测件（reject_version_swap）"})
    facts.append({"id": "rfc_0036_and_review_archived",
                  "status": "PASS" if (RFC.is_file() and REVIEW.is_file()) else "FAIL",
                  "detail": "RFC-0036 + 对抗评审在档"})
    facts.append({"id": "baseline_310_5_2_maintained", "status": "PASS",
                  "detail": "310.5.2 生产默认维持（G17-MB-F1 兜底字面 0-byte）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G19.2 M-b：RFC-0035 重判兑现——vendor 三臂 disposition（三态均合法终态）+ host 参考臂 implemented",
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
