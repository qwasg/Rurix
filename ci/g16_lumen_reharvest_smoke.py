#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16plus M-f）
"""G16plus M-f Lumen 差分重收割（g16.p0.m_f.lumen_reharvest，步骤 289）。"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g16_p0_lib as g16  # noqa: E402

GATE_KEY = "g16.p0.m_f.lumen_reharvest"
NUMERIC_STEP = 289
SUBJECT = "g16_m_f_lumen_reharvest"
WAVE = "G16.9"
SOURCE_REF = "G16_CONTRACT G-G16-9;G16_ACCEPTANCE_MAP 附录 A M-f"
SCHEMA = g16.ROOT / "milestones" / "g16" / "g16_m_f_lumen_reharvest_evidence_schema.json"
DISP = g16.ROOT / "milestones" / "g16" / "g16_quality_gap_disposition.json"


def run_gate() -> int:
    facts = []
    up_ok, up_d = g16.git_clean("milestones/g13/g13_ue_upscale_gap_registry.json")
    lm_ok, lm_d = g16.git_clean("milestones/g13/g13_ue_lumen_gap_registry.json")
    facts.append(g16.fact("g13_registries_0byte", up_ok and lm_ok, f"up={up_d} lm={lm_d}"))
    disp_ok = DISP.is_file()
    items = []
    if disp_ok:
        items = (json.loads(DISP.read_text(encoding="utf-8")) or {}).get("items") or []
    lumen = [i for i in items if "lumen" in str(i.get("title", "")).lower() or "lumen" in str(i.get("source_registry", ""))]
    facts.append(g16.fact("disposition_has_lumen_rows", len(lumen) >= 2, f"lumen_rows={len(lumen)}"))
    energy = None
    ssim = None
    for i in lumen:
        for m in i.get("fresh_measured_delta") or []:
            if "gi_energy_rel@cornell" in m.get("metric", ""):
                energy = m.get("a_value")
            if "indirect_ssim@cornell" in m.get("metric", ""):
                ssim = m.get("b_value")
    facts.append(g16.fact("cornell_energy_traceable", energy is not None, f"energy_ue={energy}"))
    facts.append(g16.fact("cornell_indirect_ssim_traceable", ssim is not None, f"indirect_ssim={ssim}"))
    facts.append(g16.fact("no_g13_write", up_ok and lm_ok, "不写 G13 两表"))
    facts.append(g16.fact("disposition_nonzero", len(items) >= 2, f"items={len(items)}"))
    return g16.emit(WAVE, SUBJECT, GATE_KEY, NUMERIC_STEP, SOURCE_REF, SCHEMA, facts, "G16plus M-f")


def run_selftest() -> int:
    if NUMERIC_STEP != 289 or not SCHEMA.is_file():
        print("[g16_m_f] SELFTEST FAIL")
        return 1
    print("[g16_m_f] SELFTEST PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return g16.verify_latest_wave(SUBJECT, 6)
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
