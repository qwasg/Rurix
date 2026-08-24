#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G21.1 治理波）
"""G21.0 baseline 快检：Stage A digest 锚在档计数 + G20 HZB 零假阳性绿计数。

真实命令输出产两件 baseline evidence（禁手写数字）：
  evidence/g21_baseline_stage_a_digest_guard.json
  evidence/g21_baseline_g20_hzb_zero_fp.json
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
import g11_wave_exit_lib as wel  # noqa: E402

ANCHOR = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
OUT_DIGEST = ROOT / "evidence/g21_baseline_stage_a_digest_guard.json"
OUT_HZB = ROOT / "evidence/g21_baseline_g20_hzb_zero_fp.json"


def main() -> int:
    anchors = json.loads(ANCHOR.read_text(encoding="utf-8"))["anchors"]
    n_anchor = sum(1 for v in anchors.values() if str(v.get("last_frame_digest", "")).startswith("sha256:"))
    OUT_DIGEST.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(n_anchor)},
            "notes": f"G21.0 Stage A digest 锚在档格数实测（anchors={n_anchor}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    p = wel.load_latest_evidence("g20_hzb_probe")
    arms_zero_fp = 0
    src = "missing"
    if p is not None:
        doc = wel.load_json(p)
        arms_zero_fp = sum(1 for a in doc.get("arms", []) if a.get("false_positives") == 0)
        src = p.name
    OUT_HZB.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(arms_zero_fp)},
            "notes": f"G21.0 G20 HZB 双约定零假阳性臂计数实测（{src}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    print(f"[g21_baseline] anchors={n_anchor} hzb_zero_fp_arms={arms_zero_fp} ({src})")
    return 0 if (n_anchor == 18 and arms_zero_fp == 2) else 1


if __name__ == "__main__":
    sys.exit(main())
