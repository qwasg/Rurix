#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D2 窗口车道视觉证据：presented BGRA8 raw dump（w/h u32 LE 头）→ PNG。

输入：
  off 参照 = artifacts/night_0828/d3_bloom/bloom_off.raw（D3 既有纯 off 臂
             presented dump，digest == 5596a730 锚面）
  on 面   = artifacts/night_0828/d2_window/on_smooth_amb.raw（d2w_verify.py 视觉臂）
输出：artifacts/night_0828/d2_window/{off_ref,on_smooth_amb}.png
"""
from __future__ import annotations

import struct
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent.parent.parent
OUT = ROOT / "artifacts" / "night_0828" / "d2_window"


def raw_to_png(raw_path: Path, png_path: Path) -> dict:
    data = raw_path.read_bytes()
    w, h = struct.unpack_from("<II", data, 0)
    px = data[8:]
    assert len(px) == w * h * 4, f"{raw_path.name}: 字节数 {len(px)} ≠ {w}×{h}×4"
    # BGRA8 打包 → RGBA（通道重排）。
    img = Image.frombytes("RGBA", (w, h), px, "raw", "BGRA")
    img.save(png_path)
    return {"src": raw_path.name, "png": png_path.name, "size": [w, h]}


def main() -> int:
    outs = []
    off_raw = ROOT / "artifacts" / "night_0828" / "d3_bloom" / "bloom_off.raw"
    on_raw = OUT / "on_smooth_amb.raw"
    if off_raw.exists():
        outs.append(raw_to_png(off_raw, OUT / "off_ref.png"))
    if on_raw.exists():
        outs.append(raw_to_png(on_raw, OUT / "on_smooth_amb.png"))
    for o in outs:
        print(o)
    return 0 if len(outs) == 2 else 1


if __name__ == "__main__":
    sys.exit(main())
