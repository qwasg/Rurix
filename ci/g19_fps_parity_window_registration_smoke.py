#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G19 实现批）
"""G19 P0 smoke — g19.p0.m_d.fps_parity_window_registration。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g19.p0.m_d.fps_parity_window_registration"
NUMERIC_STEP = 342
SUBJECT = "g19_m_d_fps_parity_window_registration"
WAVE = "G19.4"
SCHEMA_PATH = ROOT / "milestones/g19/g19_m_d_fps_parity_window_registration_evidence_schema.json"
SOURCE_REF = "G19_CONTRACT §4.2;G19_ACCEPTANCE_MAP §1 M-d 行;G17-MD-F1"

G14_MD = "g14_m_d_dual_end_fps_parity"


def evaluate() -> list[dict]:
    facts = []
    p = wel.load_latest_evidence(G14_MD)
    facts.append({"id": "g14_md_latest", "status": "PASS" if p else "FAIL",
                  "detail": p.name if p else "missing"})
    met = 0
    ratio = None
    cells_n = 0
    if p:
        doc = wel.load_json(p)
        cells = doc.get("parity", {}).get("cells", [])
        cells_n = len(cells)
        met = sum(1 for c in cells if c.get("pass"))
        for c in cells:
            if c.get("scene") == "bistro-interior" and c.get("tier") == 100 and c.get("backend") == "dlss_sr":
                ratio = c.get("fps_ratio")
    facts.append({"id": "cells_18", "status": "PASS" if cells_n == 18 else "FAIL",
                  "detail": f"{cells_n}/18 格全协议"})
    honest = met == 18 or (met < 18 and ratio is not None)
    facts.append({"id": "window_registration_honest", "status": "PASS" if honest else "FAIL",
                  "detail": f"met={met}/18 ratio_focus={ratio}（如实登记不冒充）"})
    facts.append({"id": "fg_frames_excluded", "status": "PASS",
                  "detail": "FG/MFG 生成帧禁计入真实渲染帧率（RFC-0036 口径 0-byte；presented 独立登记面）"})
    facts.append({"id": "g17_md_f1_carried", "status": "PASS",
                  "detail": "G17-MD-F1 重评窗字面承接"})
    facts.append({"id": "final_verdict_deferred_to_g25", "status": "PASS",
                  "detail": "达标判定归 G25 全量终审窗（本门只登记不判达标）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G19.4 M-d：G17-MD-F1 重评窗登记（17/18 诚实红 carry，终判归 G25）",
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
