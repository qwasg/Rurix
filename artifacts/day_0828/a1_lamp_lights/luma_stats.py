#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""A1 灯光提取臂：converged.exr 全图亮度统计（mean/p5/p50/p99）。

EXR 读法照 artifacts/night_0828/grain_metric.py（ci/g10_exr_lib.decode_exr）。

用法:
  py -3 luma_stats.py <converged.exr> [--label x] [--out m.json]
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np

from g10_exr_lib import decode_exr


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("exr")
    ap.add_argument("--label", default="")
    ap.add_argument("--out", default=None)
    args = ap.parse_args()

    f = decode_exr(Path(args.exr).read_bytes(), expected_end="rurix")
    px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
    luma = px[..., 0] * 0.2126 + px[..., 1] * 0.7152 + px[..., 2] * 0.0722

    res = {
        "label": args.label,
        "exr": args.exr,
        "width": f["width"],
        "height": f["height"],
        "mean": float(luma.mean()),
        "p5": float(np.percentile(luma, 5)),
        "p50": float(np.percentile(luma, 50)),
        "p99": float(np.percentile(luma, 99)),
        "max": float(luma.max()),
    }
    txt = json.dumps(res, indent=2, ensure_ascii=False)
    print(txt)
    if args.out:
        Path(args.out).write_text(txt + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
