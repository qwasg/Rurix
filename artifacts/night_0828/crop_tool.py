#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""夜间巡航工具：EXR 区域裁剪放大（问题确认用）。

用法:
  py -3 crop_tool.py <in.exr> <out.png> --x 0 --y 0 --w 640 --h 360 [--scale 2] [--exposure 1.0] [--mode aces|raw]
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np
from PIL import Image

from g10_exr_lib import decode_exr


def aces_fitted(x: np.ndarray) -> np.ndarray:
    a, b, c, d, e = 2.51, 0.03, 2.43, 0.59, 0.14
    return np.clip((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0)


def srgb_encode(x: np.ndarray) -> np.ndarray:
    x = np.clip(x, 0.0, 1.0)
    return np.where(x <= 0.0031308, x * 12.92, 1.055 * np.power(x, 1.0 / 2.4) - 0.055)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("src")
    ap.add_argument("dst")
    ap.add_argument("--x", type=int, required=True)
    ap.add_argument("--y", type=int, required=True)
    ap.add_argument("--w", type=int, default=640)
    ap.add_argument("--h", type=int, default=360)
    ap.add_argument("--scale", type=int, default=2)
    ap.add_argument("--exposure", type=float, default=1.0)
    ap.add_argument("--mode", choices=["aces", "raw"], default="aces")
    args = ap.parse_args()

    frame = decode_exr(Path(args.src).read_bytes(), expected_end="rurix")
    w, h = frame["width"], frame["height"]
    px = np.array(frame["pixels"], dtype=np.float64).reshape(h, w, 3)
    x0, y0 = args.x, args.y
    crop = px[y0:y0 + args.h, x0:x0 + args.w, :]
    x = crop * args.exposure
    if args.mode == "aces":
        x = aces_fitted(x)
    x8 = (srgb_encode(x) * 255.0 + 0.5).astype(np.uint8)
    im = Image.fromarray(x8, "RGB")
    if args.scale != 1:
        im = im.resize((im.width * args.scale, im.height * args.scale), Image.NEAREST)
    im.save(args.dst)
    print(f"crop ({x0},{y0}) {args.w}x{args.h} scale={args.scale} -> {args.dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
