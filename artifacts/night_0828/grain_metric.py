#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""夜间巡航工具：颗粒噪点量化 = 逐像素时间方差 + 空间高频能量。

对一组 EXR 帧（同一渲染臂的逐帧输出）:
  temporal: 逐像素跨帧 std(亮度) 的区域内均值 —— TSR 未收敛/逐帧抖动噪声量
  spatial : 收敛帧(或指定帧)高通能量 = |x - box3x3(x)| 区域内均值 —— 静态颗粒/面片感量

用法:
  py -3 grain_metric.py <frames_glob> --roi x y w h [--converged <exr>] [--stride 8] [--out m.json]
"""
from __future__ import annotations

import argparse
import glob
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np

from g10_exr_lib import decode_exr


def load_roi(path: str, x: int, y: int, w: int, h: int) -> np.ndarray:
    f = decode_exr(Path(path).read_bytes(), expected_end="rurix")
    px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
    return px[y:y + h, x:x + w, :]


def luma(img: np.ndarray) -> np.ndarray:
    return img[..., 0] * 0.2126 + img[..., 1] * 0.7152 + img[..., 2] * 0.0722


def box3(a: np.ndarray) -> np.ndarray:
    p = np.pad(a, 1, mode="edge")
    s = np.zeros_like(a)
    for dy in range(3):
        for dx in range(3):
            s += p[dy:dy + a.shape[0], dx:dx + a.shape[1]]
    return s / 9.0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("frames_glob")
    ap.add_argument("--roi", type=int, nargs=4, required=True, metavar=("X", "Y", "W", "H"))
    ap.add_argument("--converged", default=None)
    ap.add_argument("--stride", type=int, default=8)
    ap.add_argument("--max-frames", type=int, default=16)
    ap.add_argument("--out", default=None)
    args = ap.parse_args()

    x, y, w, h = args.roi
    files = sorted(glob.glob(args.frames_glob))[:: args.stride][: args.max_frames]
    if len(files) < 2:
        print("FAIL: 帧数不足", files)
        return 1

    stack = np.stack([luma(load_roi(f, x, y, w, h)) for f in files], axis=0)
    tstd = stack.std(axis=0)
    tmean = stack.mean(axis=0)
    # 相对噪声 = std / max(mean, eps)：墙面等暗区需要相对口径
    rel = tstd / np.maximum(tmean, 1e-4)

    res: dict = {
        "frames_used": len(files),
        "roi": args.roi,
        "temporal_std_mean": float(tstd.mean()),
        "temporal_std_p95": float(np.percentile(tstd, 95)),
        "temporal_rel_mean": float(rel.mean()),
        "temporal_rel_p95": float(np.percentile(rel, 95)),
        "mean_luma": float(tmean.mean()),
    }

    if args.converged:
        conv = luma(load_roi(args.converged, x, y, w, h))
        hp = np.abs(conv - box3(conv))
        res["converged_highpass_mean"] = float(hp.mean())
        res["converged_highpass_p95"] = float(np.percentile(hp, 95))
        res["converged_highpass_rel"] = float((hp / np.maximum(conv, 1e-4)).mean())

    txt = json.dumps(res, indent=2, ensure_ascii=False)
    print(txt)
    if args.out:
        Path(args.out).write_text(txt + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
