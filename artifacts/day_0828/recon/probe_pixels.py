#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0828 recon: tex_on/tex_off raw (BGRA8 1920x1080, 8B 前缀) 区域像素取证 + 裁剪图。"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
TEX = HERE.parent.parent / "night_0828" / "tex"
W, H = 1920, 1080


def load_raw(p: Path) -> np.ndarray:
    b = p.read_bytes()
    off = len(b) - W * H * 4
    a = np.frombuffer(b, dtype=np.uint8, offset=off).reshape(H, W, 4)
    return a[:, :, [2, 1, 0]]  # BGRA -> RGB


def region_stats(img: np.ndarray, x0, y0, w, h, label):
    c = img[y0:y0 + h, x0:x0 + w].reshape(-1, 3).astype(np.float64)
    mean = c.mean(axis=0)
    mx = c.max(axis=0)
    print(f"  {label:26s} ({x0},{y0},{w}x{h}) mean RGB=({mean[0]:6.1f},{mean[1]:6.1f},{mean[2]:6.1f}) max=({mx[0]:.0f},{mx[1]:.0f},{mx[2]:.0f})")


def main() -> int:
    on = load_raw(TEX / "tex_on.raw")
    off = load_raw(TEX / "tex_off.raw")
    print(f"prefix bytes: {(TEX / 'tex_on.raw').stat().st_size - W * H * 4}")

    regions = [
        ("blue_rect", 1300, 0, 260, 70),
        ("blue_rect_core", 1360, 4, 120, 40),
        ("right_wall_red", 1560, 380, 260, 200),
        ("right_wall_red2", 1700, 430, 160, 140),
    ]
    for tag, img in (("ON ", on), ("OFF", off)):
        print(f"[{tag}]")
        for label, x, y, w, h in regions:
            region_stats(img, x, y, w, h, label)

    # 裁剪放大输出（×3 nearest）+ 提亮版（×6 增益看暗部结构）
    for label, x, y, w, h in [("blue", 1240, 0, 400, 120), ("red", 1500, 340, 420, 300)]:
        for tag, img in (("on", on), ("off", off)):
            crop = img[y:y + h, x:x + w]
            Image.fromarray(crop, "RGB").resize((w * 3, h * 3), Image.NEAREST).save(HERE / f"crop_{label}_{tag}.png")
            boost = np.clip(crop.astype(np.float64) * 6.0, 0, 255).astype(np.uint8)
            Image.fromarray(boost, "RGB").resize((w * 3, h * 3), Image.NEAREST).save(HERE / f"crop_{label}_{tag}_x6.png")
    # 差分图（on-off 放大 8x）
    d = np.clip(np.abs(on.astype(np.int16) - off.astype(np.int16)) * 8, 0, 255).astype(np.uint8)
    Image.fromarray(d, "RGB").save(HERE / "diff_on_off_x8.png")
    print("crops + diff written to", HERE)
    return 0


if __name__ == "__main__":
    sys.exit(main())
