#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G41 水面环绕短片:`g41_water_present --dump-raw <base> --dump-raw-every 1` 的逐帧
raw 件 → PNG 帧 → mp4(libx264)。

raw 件布局 = `<base>.fNNNN`,w/h u32 LE 头 + BGRA8(与 g31_window_present /
day_0902_rain_night `--dump-present-every` 逐字同布局);帧号含 warmup,帧号 <
`--warmup-skip` 的帧(波场/相机收敛段)跳过。与 `artifacts/day_0902_rain_night/
make_rain_clip.py` 同形,仅缺省路径与命名参数化;ffmpeg 取 `imageio_ffmpeg` 自带
二进制(系统 PATH 无 ffmpeg,仓内约定)。

PNG 帧目录与 mp4 均在 `.gitignore`(`clip_frames/`、`*.mp4`),本脚本产出的
`<out>.json` 登记件(帧数 / 分辩率 / sha256 / ffmpeg 路径)为入库面。

用法:
    py -3 make_water_clip.py --src artifacts/day_0903_water/clip_orbit.raw ^
        --out artifacts/day_0903_water/lagoon_orbit.mp4 --warmup-skip 60 --fps 30
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

FRAME_RE = re.compile(r"\.f(\d{4})$")


def frame_no(p: Path) -> int:
    m = FRAME_RE.search(p.name)
    if m is None:
        raise ValueError(f"非帧文件名: {p.name}")
    return int(m.group(1))


def main() -> int:
    ap = argparse.ArgumentParser(description="G41 水面环绕帧序列 → mp4")
    ap.add_argument("--src", required=True, help="--dump-raw 的基路径(逐帧件 = <src>.fNNNN)")
    ap.add_argument("--frames-dir", default=None, help="PNG 帧输出目录(缺省 <src 同目录>/clip_frames)")
    ap.add_argument("--out", default=None, help="mp4 输出路径(缺省 <src 同目录>/lagoon_orbit.mp4)")
    ap.add_argument("--warmup-skip", type=int, default=60, help="帧号 < 该值的帧跳过(收敛段)")
    ap.add_argument("--fps", type=int, default=30)
    ap.add_argument("--crf", default="21")
    ap.add_argument("--report", default=None, help="登记 JSON(缺省 <out>.json)")
    args = ap.parse_args()

    src = Path(args.src).resolve()
    frames_dir = Path(args.frames_dir).resolve() if args.frames_dir else src.parent / "clip_frames"
    out = Path(args.out).resolve() if args.out else src.parent / "lagoon_orbit.mp4"
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
        print(f"FAIL: ffmpeg rc={r.returncode}")
        return 1
    sha = hashlib.sha256(out.read_bytes()).hexdigest()
    info = {
        "schema": "rurix.g41.water_clip.v1",
        "src_base": str(src).replace("\\", "/"),
        "raw_frames_total": len(raws),
        "frames_kept": len(keep),
        "warmup_skip": args.warmup_skip,
        "width": w0,
        "height": h0,
        "fps": args.fps,
        "duration_s": round(len(keep) / args.fps, 3),
        "crf": args.crf,
        "out": str(out).replace("\\", "/"),
        "out_bytes": out.stat().st_size,
        "out_sha256": f"sha256:{sha}",
        "ffmpeg": ffmpeg.replace("\\", "/"),
        "note": "帧号含 warmup;raw 帧、PNG 帧目录与 mp4 不入库(.gitignore),本 JSON 为登记件",
    }
    report.write_text(json.dumps(info, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"mp4: {out}  {info['out_bytes']} B  {info['duration_s']} s  sha256:{sha[:16]}…")
    print(f"report: {report}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
