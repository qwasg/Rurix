#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G24.1 治理波）
"""G24.0 baseline 快检：Stage A digest 锚在档计数 + 历史 open RD 清册行数实测。"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

ANCHOR = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
DEFERRED = ROOT / "registry/deferred.json"
OUT_DIGEST = ROOT / "evidence/g24_baseline_stage_a_digest_guard.json"
OUT_RD = ROOT / "evidence/g24_baseline_legacy_rd_count.json"
LEGACY = {"RD-007", "RD-011", "RD-012", "RD-014", "RD-015", "RD-026",
          "RD-027", "RD-030", "RD-032", "RD-033", "RD-036"}


def main() -> int:
    anchors = json.loads(ANCHOR.read_text(encoding="utf-8"))["anchors"]
    n_anchor = sum(1 for v in anchors.values() if str(v.get("last_frame_digest", "")).startswith("sha256:"))
    OUT_DIGEST.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(n_anchor)},
            "notes": f"G24.0 Stage A digest 锚在档格数实测（anchors={n_anchor}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    d = json.loads(DEFERRED.read_text(encoding="utf-8"))
    n_legacy = sum(1 for e in d.get("entries", [])
                   if e.get("id") in LEGACY and e.get("status") in ("open", "inherited"))
    OUT_RD.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(n_legacy)},
            "notes": f"G24.0 历史 open/inherited RD 清册行数实测（{n_legacy}/11）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8", newline="\n",
    )
    print(f"[g24_baseline] anchors={n_anchor} legacy_rd={n_legacy}")
    return 0 if (n_anchor == 18 and n_legacy == 11) else 1


if __name__ == "__main__":
    sys.exit(main())
