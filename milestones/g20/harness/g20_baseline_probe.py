#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G20.1 治理波）
"""G20.0 baseline 快检：Stage A digest 锚在档计数 + G19 FG 三档质量绿计数。

真实命令输出产两件 baseline evidence（禁手写数字）：
  evidence/g20_baseline_stage_a_digest_guard.json
  evidence/g20_baseline_g19_fg_lanes_pass.json
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
import g11_wave_exit_lib as wel  # noqa: E402

ANCHOR = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
OUT_DIGEST = ROOT / "evidence/g20_baseline_stage_a_digest_guard.json"
OUT_FG = ROOT / "evidence/g20_baseline_g19_fg_lanes_pass.json"


def main() -> int:
    anchors = json.loads(ANCHOR.read_text(encoding="utf-8"))["anchors"]
    n_anchor = sum(1 for v in anchors.values() if str(v.get("last_frame_digest", "")).startswith("sha256:"))
    OUT_DIGEST.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(n_anchor)},
            "notes": f"G20.0 Stage A digest 锚在档格数实测（anchors={n_anchor}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    p = wel.load_latest_evidence("g19_frame_gen_probe")
    lanes_pass = 0
    src = "missing"
    if p is not None:
        doc = wel.load_json(p)
        lanes_pass = sum(1 for l in doc.get("lanes", []) if l.get("all_frames_interp_gt_hold"))
        src = p.name
    OUT_FG.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(lanes_pass)},
            "notes": f"G20.0 G19 FG 三档质量绿计数实测（{src}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    print(f"[g20_baseline] anchors={n_anchor} fg_lanes_pass={lanes_pass} ({src})")
    return 0 if (n_anchor == 18 and lanes_pass == 3) else 1


if __name__ == "__main__":
    sys.exit(main())
