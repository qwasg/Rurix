#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0902 雨夜街景战役:`<base>.raw.fNNNN` 逐帧序列 → PNG(跳 warmup,连续重编号)→ H.264 mp4。

输入布局 = g35_particle_lane --dump-present-raw <base> --dump-present-every n 的逐帧转储
(w/h u32 LE 头 + BGRA8;g31_window_present 逐字同布局),帧号含 warmup;
帧号 < --warmup-skip 的帧(TSR 收敛段)跳过。与 artifacts/day_0831_site/make_flight_video.py 同形,
仅把 SRC / FRAMES_DIR / OUT / WARMUP_SKIP / FPS 参数化;ffmpeg 取 imageio_ffmpeg 自带二进制
(系统 PATH 无 ffmpeg)。PNG 帧目录与 mp4 均在 .gitignore(clip_frames/、*.mp4)。

用法:
  py -3 make_rain_clip.py --src clip.raw [--frames-dir clip_frames] [--out bistro_rain_night.mp4]
                          [--warmup-skip 10] [--fps 30] [--crf 21]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
FRAME_RE = re.compile(r"\.f(\d{4})$")


def frame_no(p: Path) -> int:
    m = FRAME_RE.search(p.name)
    if m is None:
        raise ValueError(f"非帧文件名: {p.name}")
    return int(m.group(1))


def main() -> int:
    ap = argparse.ArgumentParser(description="雨夜推轨帧序列 → mp4")
    ap.add_argument("--src", required=True, help="--dump-present-raw 的基路径(逐帧件 = <src>.fNNNN)")
    ap.add_argument("--frames-dir", default=None, help="PNG 帧输出目录(缺省 <src 同目录>/clip_frames)")
    ap.add_argument("--out", default=None, help="mp4 输出路径(缺省 <src 同目录>/bistro_rain_night.mp4)")
    ap.add_argument("--warmup-skip", type=int, default=10, help="帧号 < 该值的帧跳过(warmup 收敛段)")
    ap.add_argument("--fps", type=int, default=30)
    ap.add_argument("--crf", default="21")
    ap.add_argument("--report", default=None, help="登记 JSON(缺省 <out>.json)")
    args = ap.parse_args()

    src = Path(args.src).resolve()
    frames_dir = Path(args.frames_dir).resolve() if args.frames_dir else src.parent / "clip_frames"
    out = Path(args.out).resolve() if args.out else src.parent / "bistro_rain_night.mp4"
    report = Path(args.report).resolve() if args.report else out.with_suffix(out.suffix + ".json")

    raws = sorted(
        (p for p in src.parent.glob(src.name + ".f*") if FRAME_RE.search(p.name)),
        key=frame_no,
    )
    keep = [p for p in raws if frame_no(p) >= args.warmup_skip]
    if not keep:
        print(f"FAIL: 无帧可转({src}.f* 共 {len(raws)} 件,warmup_skip={args.warmup_skip})")
        return 1
    frames_dir.mkdir(parents=True, exist_ok=True)
    w0 = h0 = None
    for n, p in enumerate(keep):
        b = p.read_bytes()
        w, h = struct.unpack_from("<II", b, 0)
        if len(b) != 8 + w * h * 4:
            print(f"FAIL: 帧 {p.name} 字节数 {len(b)} ≠ 8 + {w}×{h}×4")
            return 1
        if w0 is None:
            w0, h0 = w, h
        elif (w, h) != (w0, h0):
            print(f"FAIL: 帧 {p.name} 分辨率 {w}×{h} 与首帧 {w0}×{h0} 不一致")
            return 1
        a = np.frombuffer(b, dtype=np.uint8, offset=8, count=w * h * 4).reshape(h, w, 4)
        rgb = a[:, :, [2, 1, 0]]
        Image.fromarray(np.ascontiguousarray(rgb), "RGB").save(frames_dir / f"f{n:04d}.png")
    print(f"frames: {len(keep)}(跳过 {len(raws) - len(keep)} 帧 warmup)→ {frames_dir}  {w0}x{h0}")

    import imageio_ffmpeg  # noqa: E402  仓内约定:系统无 ffmpeg,用 imageio_ffmpeg 自带件

    ffmpeg = imageio_ffmpeg.get_ffmpeg_exe()
    cmd = [
        ffmpeg, "-y", "-framerate", str(args.fps),
        "-i", str(frames_dir / "f%04d.png"),
        "-c:v", "libx264", "-crf", str(args.crf), "-preset", "slow",
        "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an", str(out),
    ]
    r = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if r.returncode != 0:
        print(r.stderr[-1600:])
        return 1
    sha = hashlib.sha256(out.read_bytes()).hexdigest()
    rec = {
        "src": str(src).replace("\\", "/"),
        "frames_total": len(raws),
        "frames_kept": len(keep),
        "warmup_skip": args.warmup_skip,
        "fps": args.fps,
        "crf": str(args.crf),
        "resolution": [w0, h0],
        "duration_s": round(len(keep) / args.fps, 3),
        "out": str(out).replace("\\", "/"),
        "out_bytes": out.stat().st_size,
        "out_sha256": f"sha256:{sha}",
        "ffmpeg": ffmpeg.replace("\\", "/"),
        "note": "帧号含 warmup;PNG 帧目录与 mp4 不入库(.gitignore),本 JSON 为登记件",
    }
    report.write_text(json.dumps(rec, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"{out.name}  {out.stat().st_size // 1024} KB  {len(keep)} 帧 @{args.fps}fps  {rec['duration_s']} s  → 登记 {report.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
