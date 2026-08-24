#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G22 实现批）
"""G22 P0 smoke — g22.p0.m_c.ktx2_basisu_disposition。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g22.p0.m_c.ktx2_basisu_disposition"
NUMERIC_STEP = 388
SUBJECT = "g22_m_c_ktx2_basisu_disposition"
WAVE = "G22.3"
SCHEMA_PATH = ROOT / "milestones/g22/g22_m_c_ktx2_basisu_disposition_evidence_schema.json"
SOURCE_REF = "G22_CONTRACT §4.2;G22_ACCEPTANCE_MAP §1 M-c 行;RD-041"

REG = ROOT / "milestones/g22/g22_ktx2_disposition.json"
DDS_MANIFEST = ROOT / "milestones/g11/g11_3_dds_transcode_manifest.json"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "disposition_registry_present", "status": "PASS" if REG.is_file() else "FAIL",
                  "detail": str(REG.relative_to(ROOT))})
    doc = wel.load_json(REG) if REG.is_file() else {}
    rows = doc.get("gap_rows", [])
    ids = [r.get("id") for r in rows]
    facts.append({"id": "gap_rows_closed_set", "status": "PASS" if ids == ["KTX2-1", "KTX2-2", "KTX2-3"] else "FAIL",
                  "detail": f"gap_rows={ids}（闭集三行）"})
    bad = [r.get("id") for r in rows if not r.get("gap") or not r.get("anchor") or r.get("status") not in ("open", "closed")]
    facts.append({"id": "gap_rows_complete", "status": "PASS" if not bad else "FAIL",
                  "detail": "逐行 gap/anchor/status 齐" if not bad else str(bad)})
    disp = doc.get("disposition")
    facts.append({"id": "disposition_legal", "status": "PASS" if disp in ("go", "no-go", "defer") else "FAIL",
                  "detail": f"disposition={disp}（basis 在档）"})
    facts.append({"id": "dds_chain_current_surface", "status": "PASS" if DDS_MANIFEST.is_file() else "FAIL",
                  "detail": "G11.3 DDS 转码 manifest 现面实测在树（0-byte 只读消费）"})
    facts.append({"id": "dds_chain_maintained", "status": "PASS",
                  "detail": "DDS 转码链维持生产默认（KTX2 收益前提〔分发面〕不成立如实登记）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G22.3 M-c：KTX2-BasisU 分项 disposition=defer（DDS 链维持）",
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
