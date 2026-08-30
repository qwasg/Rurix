#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""夜间巡航工具：EXR → PNG 可视化（复用 ci/g10_exr_lib 独立解析器）。

用法:
  py -3 artifacts/night_0828/exr_view.py <in.exr> <out.png> [--exposure E] [--mode aces|reinhard|raw]
  --mode raw: 不做 tonemap,仅 exposure 乘法 + sRGB gamma(用于观察原始动态范围)
"""
from __future__ import annotations

import argparse
import math
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np
from PIL import Image

from g10_exr_lib import decode_exr


def aces_fitted(x: np.ndarray) -> np.ndarray:
    # Stephen Hill ACES fit (Krzysztof Narkowicz 简化式)
    a, b, c, d, e = 2.51, 0.03, 2.43, 0.59, 0.14
    return np.clip((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0)


def srgb_encode(x: np.ndarray) -> np.ndarray:
    x = np.clip(x, 0.0, 1.0)
    return np.where(x <= 0.0031308, x * 12.92, 1.055 * np.power(x, 1.0 / 2.4) - 0.055)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("src")
    ap.add_argument("dst")
    ap.add_argument("--exposure", type=float, default=1.0, help="乘性曝光(线性域)")
    ap.add_argument("--mode", choices=["aces", "reinhard", "raw"], default="aces")
    args = ap.parse_args()

    buf = Path(args.src).read_bytes()
    frame = decode_exr(buf, expected_end="rurix")
    w, h, layout = frame["width"], frame["height"], frame["layout"]
    px = np.array(frame["pixels"], dtype=np.float64)
    if layout == "rgb":
        img = px.reshape(h, w, 3)
    else:
        img = np.repeat(px.reshape(h, w, 1), 3, axis=2)

    stats = {
        "min": [float(img[..., c].min()) for c in range(3)],
        "max": [float(img[..., c].max()) for c in range(3)],
        "mean": [float(img[..., c].mean()) for c in range(3)],
        "p99": [float(np.percentile(img[..., c], 99)) for c in range(3)],
        "nan_count": int(np.isnan(img).sum()),
        "neg_count": int((img < 0).sum()),
    }

    x = img * args.exposure
    if args.mode == "aces":
        x = aces_fitted(x)
    elif args.mode == "reinhard":
        x = x / (1.0 + x)
    x8 = (srgb_encode(x) * 255.0 + 0.5).astype(np.uint8)
    Image.fromarray(x8, "RGB").save(args.dst)
    print(f"src={args.src} {w}x{h} layout={layout}")
    print(f"stats={stats}")
    print(f"dst={args.dst} mode={args.mode} exposure={args.exposure}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
