#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0831 网站出图战役:flight.raw.fNNNN 帧序列 → PNG(跳 warmup,连续重编号)→ H.264 mp4。

输入布局 = g31_window_present --dump-present-raw(w/h u32 LE 头 + BGRA8);
raw2png.py 同律 BGRA→RGB。帧号含 warmup,前 WARMUP_SKIP 帧(TSR 收敛段)跳过。
"""
from __future__ import annotations

import re
import struct
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image
import imageio_ffmpeg

HERE = Path(__file__).resolve().parent
WARMUP_SKIP = 10          # 帧号 < 10 跳过(warmup 收敛段)
FPS = 30
CRF = "21"
SRC = HERE / "flight.raw"
FRAMES_DIR = HERE / "flight_frames"
OUT = HERE / "bistro_flight.mp4"


def main() -> int:
    raws = sorted(
        HERE.glob("flight.raw.f*"),
        key=lambda p: int(re.search(r"f(\d{4})$", p.name).group(1)),
    )
    keep = [p for p in raws if int(re.search(r"f(\d{4})$", p.name).group(1)) >= WARMUP_SKIP]
    if not keep:
        print("FAIL: 无帧可转")
        return 1
    FRAMES_DIR.mkdir(exist_ok=True)
    for n, p in enumerate(keep):
        b = p.read_bytes()
        w, h = struct.unpack_from("<II", b, 0)
        a = np.frombuffer(b, dtype=np.uint8, offset=8, count=w * h * 4).reshape(h, w, 4)
        rgb = a[:, :, [2, 1, 0]]
        Image.fromarray(np.ascontiguousarray(rgb), "RGB").save(FRAMES_DIR / f"f{n:04d}.png")
    print(f"frames: {len(keep)} → {FRAMES_DIR}")
    ffmpeg = imageio_ffmpeg.get_ffmpeg_exe()
    cmd = [
        ffmpeg, "-y", "-framerate", str(FPS),
        "-i", str(FRAMES_DIR / "f%04d.png"),
        "-c:v", "libx264", "-crf", CRF, "-preset", "slow",
        "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an", str(OUT),
    ]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0:
        print(r.stderr[-1600:])
        return 1
    print(f"{OUT.name}  {OUT.stat().st_size // 1024} KB  {len(keep)} 帧 @{FPS}fps")
    return 0


if __name__ == "__main__":
    sys.exit(main())
