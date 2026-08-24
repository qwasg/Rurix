#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G22.1 治理波）
"""G22.0 baseline 快检：Stage A digest 锚在档计数 + G21 ReSTIR 方差收益实测。"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
import g11_wave_exit_lib as wel  # noqa: E402

ANCHOR = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
OUT_DIGEST = ROOT / "evidence/g22_baseline_stage_a_digest_guard.json"
OUT_RESTIR = ROOT / "evidence/g22_baseline_g21_restir_gain.json"


def main() -> int:
    anchors = json.loads(ANCHOR.read_text(encoding="utf-8"))["anchors"]
    n_anchor = sum(1 for v in anchors.values() if str(v.get("last_frame_digest", "")).startswith("sha256:"))
    OUT_DIGEST.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(n_anchor)},
            "notes": f"G22.0 Stage A digest 锚在档格数实测（anchors={n_anchor}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    p = wel.load_latest_evidence("g21_restir_probe")
    gain = 0.0
    src = "missing"
    if p is not None:
        doc = wel.load_json(p)
        gain = float(doc.get("variance_reduction", 0.0))
        src = p.name
    OUT_RESTIR.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": gain},
            "notes": f"G22.0 G21 ReSTIR 方差收益实测 carry（{src}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    print(f"[g22_baseline] anchors={n_anchor} restir_gain={gain} ({src})")
    return 0 if (n_anchor == 18 and gain > 2.0) else 1


if __name__ == "__main__":
    sys.exit(main())
