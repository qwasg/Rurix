#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase D alpha 稳态档 ladder 快检：conv 协议四 ROI std_p95（vs arm1 基线）。"""
from __future__ import annotations

import glob
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np

from g10_exr_lib import decode_exr

BASE = ROOT / "artifacts/day_0828/d_tsr/arms"
ARMS = {
    "arm1_off": BASE / "arm1_snrm/bistro-interior/tier100/tsr_device",
    "a004": BASE / "arm2_snrm_tsrq/bistro-interior/tier100/tsr_device",
    "a002": BASE / "ladder_a002/bistro-interior/tier100/tsr_device",
}
ROIS = {
    "wall": (1400, 150, 480, 270),
    "floor": (1100, 800, 480, 270),
    "dark_arch": (360, 0, 360, 180),
    "dark_table": (560, 560, 560, 200),
}


def main() -> int:
    out: dict = {}
    for arm, base in ARMS.items():
        files = sorted(glob.glob(str(base / "frames/frame_01*.exr")))[::2][:16]
        stacks = {rn: [] for rn in ROIS}
        for fp in files:
            f = decode_exr(Path(fp).read_bytes(), expected_end="rurix")
            px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
            luma = px[..., 0] * 0.2126 + px[..., 1] * 0.7152 + px[..., 2] * 0.0722
            for rn, (x, y, w, h) in ROIS.items():
                stacks[rn].append(luma[y : y + h, x : x + w])
        out[arm] = {
            rn: float(np.percentile(np.stack(s, axis=0).std(axis=0), 95))
            for rn, s in stacks.items()
        }
    for arm in ("a004", "a002"):
        for rn in ROIS:
            d = (1.0 - out[arm][rn] / max(out["arm1_off"][rn], 1e-30)) * 100.0
            print(f"{arm} {rn}: std_p95={out[arm][rn]:.4e} drop_vs_off={d:+.1f}%")
    print(json.dumps(out, indent=1))
    Path(ROOT / "artifacts/day_0828/d_tsr/d_ladder.json").write_text(
        json.dumps(out, indent=1) + "\n", encoding="utf-8"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
