#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G23.1 治理波）
"""G23.0 baseline 快检：Stage A digest 锚在档计数 + G22 slab 白炉审计样本数实测。"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
import g11_wave_exit_lib as wel  # noqa: E402

ANCHOR = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
OUT_DIGEST = ROOT / "evidence/g23_baseline_stage_a_digest_guard.json"
OUT_SLAB = ROOT / "evidence/g23_baseline_g22_slab_samples.json"


def main() -> int:
    anchors = json.loads(ANCHOR.read_text(encoding="utf-8"))["anchors"]
    n_anchor = sum(1 for v in anchors.values() if str(v.get("last_frame_digest", "")).startswith("sha256:"))
    OUT_DIGEST.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(n_anchor)},
            "notes": f"G23.0 Stage A digest 锚在档格数实测（anchors={n_anchor}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    p = wel.load_latest_evidence("g22_slab_probe")
    samples = 0
    src = "missing"
    if p is not None:
        doc = wel.load_json(p)
        samples = int(doc.get("samples", 0)) if doc.get("white_furnace_identity") else 0
        src = p.name
    OUT_SLAB.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(samples)},
            "notes": f"G23.0 G22 slab 白炉审计绿样本数实测 carry（{src}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    print(f"[g23_baseline] anchors={n_anchor} slab_samples={samples} ({src})")
    return 0 if (n_anchor == 18 and samples >= 16641) else 1


if __name__ == "__main__":
    sys.exit(main())
