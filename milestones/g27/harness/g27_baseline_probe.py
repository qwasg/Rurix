#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G27.1 治理波）
"""G27.0 baseline 快检：Stage A digest 锚在档计数 + 上游八期收口 tag 计数实测。"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

ANCHOR = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
OUT_DIGEST = ROOT / "evidence/g27_baseline_stage_a_digest_guard.json"
OUT_TAGS = ROOT / "evidence/g27_baseline_campaign_tags.json"
CAMPAIGN_TAGS = ["g19-closed", "g20-closed", "g21-closed", "g22-closed",
                 "g23-closed", "g24-closed", "g25-closed", "g26-closed"]


def main() -> int:
    anchors = json.loads(ANCHOR.read_text(encoding="utf-8"))["anchors"]
    n_anchor = sum(1 for v in anchors.values() if str(v.get("last_frame_digest", "")).startswith("sha256:"))
    OUT_DIGEST.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(n_anchor)},
            "notes": f"G27.0 Stage A digest 锚在档格数实测（anchors={n_anchor}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    n_tags = 0
    for t in CAMPAIGN_TAGS:
        r = subprocess.run(["git", "rev-parse", "--verify", "--quiet", t],
                           cwd=ROOT, capture_output=True)
        if r.returncode == 0:
            n_tags += 1
    OUT_TAGS.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(n_tags)},
            "notes": f"G27.0 上游八期收口 tag 实测（{n_tags}/8：{CAMPAIGN_TAGS}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    print(f"[g27_baseline] anchors={n_anchor} campaign_tags={n_tags}/8")
    return 0 if (n_anchor == 18 and n_tags == 8) else 1


if __name__ == "__main__":
    sys.exit(main())
