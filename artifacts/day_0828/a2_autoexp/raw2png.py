#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""A2 验证工具：presented raw dump（w/h u32 LE 头 + BGRA8 打包）→ PNG。

用法: py -3 raw2png.py <in.raw> [<in2.raw> ...]（同名 .png 落同目录）
"""
from __future__ import annotations

import struct
import sys
from pathlib import Path

from PIL import Image


def convert(p: Path) -> Path:
    b = p.read_bytes()
    w, h = struct.unpack_from("<II", b, 0)
    px = b[8 : 8 + w * h * 4]
    # 窗口 swapchain 为 bgra8_unorm ⇒ 字节序 [b,g,r,a]。
    img = Image.frombytes("RGBA", (w, h), bytes(px), "raw", "BGRA")
    out = p.with_suffix(p.suffix + ".png") if not p.name.endswith(".raw") else Path(str(p)[:-4] + ".png")
    img.convert("RGB").save(out)
    return out


def main() -> None:
    for arg in sys.argv[1:]:
        p = Path(arg)
        out = convert(p)
        print(f"{p.name} -> {out.name}")


if __name__ == "__main__":
    main()
