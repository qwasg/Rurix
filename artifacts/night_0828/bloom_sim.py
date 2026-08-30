#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D3 bloom 算法仿真验证（与 g31_bloom_*.rx 三 kernel 同式）在真实收敛帧上。

链：HDR → 软膝阈值+2×降采样(g31_bloom_bright) → 半分辨率高斯 H/V(g31_bloom_blur)
→ 双线性上采样加性合成(g31_bloom_composite) → ACES → sRGB → PNG 对照。

用法: py -3 bloom_sim.py <converged.exr> <out_prefix> [--threshold 1.0] [--strength 0.3] [--exposure 1.0]
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

W9 = np.array([0.028532, 0.067234, 0.124009, 0.179044, 0.202360,
               0.179044, 0.124009, 0.067234, 0.028532])


def aces_fitted(x):
    a, b, c, d, e = 2.51, 0.03, 2.43, 0.59, 0.14
    return np.clip((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0)


def srgb_encode(x):
    x = np.clip(x, 0.0, 1.0)
    return np.where(x <= 0.0031308, x * 12.92, 1.055 * np.power(x, 1.0 / 2.4) - 0.055)


def bright_down(img: np.ndarray, threshold: float, knee: float) -> np.ndarray:
    h, w, _ = img.shape
    oh, ow = (h + 1) // 2, (w + 1) // 2
    out = np.zeros((oh, ow, 3))
    for oy in range(oh):
        for ox in range(ow):
            sx0, sy0 = min(ox * 2, w - 1), min(oy * 2, h - 1)
            sx1, sy1 = min(ox * 2 + 1, w - 1), min(oy * 2 + 1, h - 1)
            block = img[[sy0, sy0, sy1, sy1], [sx0, sx1, sx0, sx1], :]
            c = block.mean(axis=0)
            luma = c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
            soft = max(0.0, luma - threshold + knee)
            soft_w = soft * soft / max(4.0 * knee, 1e-6)
            hard_w = luma - threshold
            if luma < threshold - knee:
                wgt = 0.0
            elif luma > threshold + knee:
                wgt = hard_w
            else:
                wgt = soft_w
            out[oy, ox] = c * (wgt / max(luma, 1e-6))
    return out


def gauss_blur(img: np.ndarray) -> np.ndarray:
    # 可分离 9-tap，edge replicate
    p = np.pad(img, ((0, 0), (4, 4), (0, 0)), mode="edge")
    tmp = np.zeros_like(img)
    for k in range(9):
        tmp += p[:, k:k + img.shape[1], :] * W9[k]
    p2 = np.pad(tmp, ((4, 4), (0, 0), (0, 0)), mode="edge")
    out = np.zeros_like(tmp)
    for k in range(9):
        out += p2[k:k + tmp.shape[0], :, :] * W9[k]
    return out


def upsample_add(img: np.ndarray, bloom: np.ndarray, strength: float) -> np.ndarray:
    h, w, _ = img.shape
    bh, bw, _ = bloom.shape
    fy = (np.arange(h) + 0.5) * 0.5 - 0.5
    fx = (np.arange(w) + 0.5) * 0.5 - 0.5
    yy, xx = np.meshgrid(fy, fx, indexing="ij")  # (h, w)
    y0 = np.clip(np.floor(yy).astype(int), 0, bh - 2)
    x0 = np.clip(np.floor(xx).astype(int), 0, bw - 2)
    ty = (yy - np.floor(yy))[..., None]
    tx = (xx - np.floor(xx))[..., None]
    b00 = bloom[y0, x0]
    b10 = bloom[y0, x0 + 1]
    b01 = bloom[y0 + 1, x0]
    b11 = bloom[y0 + 1, x0 + 1]
    bl = (b00 * (1 - tx) * (1 - ty) + b10 * tx * (1 - ty)
          + b01 * (1 - tx) * ty + b11 * tx * ty)
    return img + bl * strength


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("src")
    ap.add_argument("out_prefix")
    ap.add_argument("--threshold", type=float, default=1.0)
    ap.add_argument("--strength", type=float, default=0.3)
    ap.add_argument("--knee", type=float, default=0.5)
    ap.add_argument("--exposure", type=float, default=1.0)
    args = ap.parse_args()

    f = decode_exr(Path(args.src).read_bytes(), expected_end="rurix")
    w, h = f["width"], f["height"]
    img = np.array(f["pixels"], dtype=np.float64).reshape(h, w, 3) * args.exposure

    bright = bright_down(img, args.threshold, args.knee)
    bloom = gauss_blur(bright)
    comp = upsample_add(img, bloom, args.strength)

    for name, data in [("nobloom", img), ("bloom", comp)]:
        x8 = (srgb_encode(aces_fitted(data)) * 255.0 + 0.5).astype(np.uint8)
        Image.fromarray(x8, "RGB").save(f"{args.out_prefix}_{name}.png")
    # bloom 贡献量统计
    diff = (comp - img)
    print(f"bloom 像素平均增量 = {diff.mean():.6f}，>0.01 像素占比 = {(diff.max(axis=2) > 0.01).mean()*100:.2f}%")
    print(f"亮部像素(阈值上)占比 = {(bright.max(axis=2) > 0).mean()*100:.3f}%")
    return 0


if __name__ == "__main__":
    sys.exit(main())
