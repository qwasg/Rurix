#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""A2b 视觉对照：修复前后 presented 实测帧的吊扇/灯罩区裁剪并排图。

前 = artifacts/day_0828/recon/bluefan/a_default.raw（bug SPV 实测,digest==旧锚 5596a730）
后 = artifacts/day_0828/a2b_aces_fix/off.raw（v2 修复 SPV 实测,digest==新锚 55e4a92d）
裁剪窗与 recon fan_bug_vs_fixed.png 同（x1300-1720, y0-220, ×2 NEAREST）——
recon 那张右半为『仿真』修复预测，本图右半为『GPU 实测』修复结果。
"""
from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent


def load_raw(p: Path) -> np.ndarray:
    raw = np.frombuffer(p.read_bytes(), dtype=np.uint8)
    w, h = (int(v) for v in np.frombuffer(raw[:8].tobytes(), dtype="<u4"))
    return raw[8:].reshape(h, w, 4)[:, :, [2, 1, 0]]  # BGRA→RGB


def main() -> int:
    before = load_raw(HERE.parent / "recon" / "bluefan" / "a_default.raw")
    after = load_raw(HERE / "off.raw")
    Image.fromarray(after, "RGB").save(HERE / "off_full.png")
    x0, x1, y0, y1 = 1300, 1720, 0, 220
    gap = np.full((y1 - y0, 4, 3), 255, np.uint8)
    pair = np.concatenate([before[y0:y1, x0:x1], gap, after[y0:y1, x0:x1]], axis=1)
    img = Image.fromarray(pair, "RGB").resize((pair.shape[1] * 2, pair.shape[0] * 2), Image.NEAREST)
    img.save(HERE / "fan_ring_before_after.png")
    fp = (12, 1500)
    print(f"before fan(1500,12) = {tuple(int(v) for v in before[fp])}")
    print(f"after  fan(1500,12) = {tuple(int(v) for v in after[fp])}")
    # 灯罩渐变环带量化：同一竖直扫描线（灯罩中心 x=1655）G 通道非单调折返次数。
    xs = 1655
    for tag, img_ in (("before", before), ("after", after)):
        col = img_[10:200, xs, 1].astype(np.int16)
        dif = np.diff(col)
        reversals = int(np.sum((dif[1:] > 0) & (dif[:-1] < 0)) + np.sum((dif[1:] < 0) & (dif[:-1] > 0)))
        print(f"{tag}: lampshade scanline x={xs} G 非单调折返数 = {reversals}")
    print("wrote off_full.png + fan_ring_before_after.png (left=bug measured/right=fixed measured, x2)")
    return 0


if __name__ == "__main__":
    main()
