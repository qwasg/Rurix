#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Day 0829 视觉对照工具:presented raw dump → PNG(可选增益/伽马提亮暗部)。

来源:artifacts/day_0828/a2_autoexp/raw2png.py 适配拷贝,新增 --gain/--gamma。
输入格式(g31_window_present.rs 写盘段确认):w/h u32 LE 头(8B)+ BGRA8 打包。
注意 raw 已是 display-encode 后的 8bit 字节;--gain/--gamma 仅为查看辅助
(暗部检查提亮),默认 1.0/1.0 时与 a2 原版输出像素级一致。

用法:
  py -3 raw2png.py <in.raw> [<in2.raw> ...] [--gain 4] [--gamma 2.2]
  （同名 .png 落同目录;.raw.fNNNN 多帧后缀 → _fNNNN.png）
"""
from __future__ import annotations

import argparse
import re
import struct
from pathlib import Path

import numpy as np
from PIL import Image


def out_path(p: Path) -> Path:
    """off.raw → off.png;off.raw.f0080 → off_f0080.png;其余加 .png。"""
    m = re.match(r"^(.*)\.raw\.(f\d{4})$", p.name)
    if m:
        return p.with_name(f"{m.group(1)}_{m.group(2)}.png")
    if p.name.endswith(".raw"):
        return p.with_name(p.name[:-4] + ".png")
    return p.with_name(p.name + ".png")


def convert(p: Path, gain: float, gamma: float) -> Path:
    b = p.read_bytes()
    w, h = struct.unpack_from("<II", b, 0)
    a = np.frombuffer(b, dtype=np.uint8, offset=8, count=w * h * 4).reshape(h, w, 4)
    rgb = a[:, :, [2, 1, 0]]  # 窗口 swapchain bgra8_unorm ⇒ [b,g,r,a] → RGB
    if gain != 1.0 or gamma != 1.0:
        f = np.clip(rgb.astype(np.float64) / 255.0 * gain, 0.0, 1.0)
        rgb = (np.power(f, 1.0 / gamma) * 255.0 + 0.5).astype(np.uint8)
    out = out_path(p)
    Image.fromarray(np.ascontiguousarray(rgb), "RGB").save(out)
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("raws", nargs="+")
    ap.add_argument("--gain", type=float, default=1.0, help="线性增益(默认 1.0)")
    ap.add_argument("--gamma", type=float, default=1.0, help="显示伽马(默认 1.0)")
    args = ap.parse_args()
    for arg in args.raws:
        p = Path(arg)
        out = convert(p, args.gain, args.gamma)
        print(f"{p.name} -> {out.name}")


if __name__ == "__main__":
    main()
