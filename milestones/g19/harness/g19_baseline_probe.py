#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G19.1 治理波）
"""G19.0 baseline 快检：Stage A digest 锚 18 格在档计数 + G14 M-d 最新 18 格 met 计数。

真实命令输出产两件 baseline evidence（禁手写数字）：
  evidence/g19_baseline_stage_a_digest_guard.json   （期望 18 锚全在档）
  evidence/g19_baseline_fps_parity_met.json         （最新 g14_m_d evidence met 计数如实）
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
import g11_wave_exit_lib as wel  # noqa: E402

ANCHOR = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
OUT_DIGEST = ROOT / "evidence/g19_baseline_stage_a_digest_guard.json"
OUT_FPS = ROOT / "evidence/g19_baseline_fps_parity_met.json"


def main() -> int:
    anchors = json.loads(ANCHOR.read_text(encoding="utf-8"))["anchors"]
    n_anchor = sum(1 for v in anchors.values() if str(v.get("last_frame_digest", "")).startswith("sha256:"))
    OUT_DIGEST.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(n_anchor)},
            "notes": f"G19.0 Stage A digest 锚在档格数实测（milestones/g14/g14_3_stage_a_digest_anchor.json anchors={n_anchor}）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    p = wel.load_latest_evidence("g14_m_d_dual_end_fps_parity")
    met = 0
    src = "missing"
    if p is not None:
        doc = wel.load_json(p)
        cells = doc.get("parity", {}).get("cells", [])
        met = sum(1 for c in cells if c.get("pass"))
        src = p.name
    OUT_FPS.write_text(
        json.dumps({
            "schema_version": 1,
            "results": {"trimmed_mean": float(met)},
            "notes": f"G19.0 fps parity met 计数实测（{src}；17/18 诚实红 carry，终判归 G25）",
        }, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"[g19_baseline] anchors={n_anchor} → {OUT_DIGEST.relative_to(ROOT)}")
    print(f"[g19_baseline] fps_met={met} ({src}) → {OUT_FPS.relative_to(ROOT)}")
    return 0 if (n_anchor == 18 and met >= 17) else 1


if __name__ == "__main__":
    sys.exit(main())
