#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase C 噪声空间特征：时域 std 图的 lag-1 空间自相关（旧 sin hash 的
屏空间相关色块 vs R2 白噪特征的量化面）+ 绝对幅值对比 + 老新臂视觉裁剪。

自相关口径：ROI 时域 std 图去均值后 corr(x, x+1) / corr(y, y+1)——
sin hash 屏空间相关 ⇒ 噪声成块（正相关高）；R2 ⇒ 接近白噪（低/负）。
"""
from __future__ import annotations

import glob
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np
from PIL import Image

from g10_exr_lib import decode_exr

ARMS = {
    "old_gi_sinhash": ROOT / "artifacts/night_0828/arms/gi_on/bistro-interior/tier100/tsr_device",
    "gi2_on_c001": ROOT / "artifacts/day_0828/c_gi_r2/arms/gi2_on_c001/bistro-interior/tier100/tsr_device",
    "gi2_on_c015": ROOT / "artifacts/day_0828/c_gi_r2/arms/gi2_on_c015/bistro-interior/tier100/tsr_device",
}
ROIS = {
    "wall": (1400, 150, 480, 270),
    "dark_arch": (360, 0, 360, 180),
    "dark_table": (560, 560, 560, 200),
}


def load_luma_roi(path, roi) -> np.ndarray:
    f = decode_exr(Path(path).read_bytes(), expected_end="rurix")
    px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
    x, y, w, h = roi
    c = px[y : y + h, x : x + w, :]
    return c[..., 0] * 0.2126 + c[..., 1] * 0.7152 + c[..., 2] * 0.0722


def lag1(a: np.ndarray) -> tuple[float, float]:
    d = a - a.mean()
    sx = float((d[:, :-1] * d[:, 1:]).mean() / max(d.var(), 1e-30))
    sy = float((d[:-1, :] * d[1:, :]).mean() / max(d.var(), 1e-30))
    return sx, sy


def main() -> int:
    out: dict = {"schema": "rurix.day0828.c_gi_r2.spatial_char.v1", "law": "temporal-std map lag-1 spatial autocorr (conv frames 100..127 stride2)"}
    for arm, base in ARMS.items():
        files = sorted(glob.glob(str(base / "frames" / "frame_01*.exr")))[::2][:16]
        rec = {}
        for rn, roi in ROIS.items():
            stack = np.stack([load_luma_roi(f, roi) for f in files], axis=0)
            tstd = stack.std(axis=0)
            cx, cy = lag1(tstd)
            rec[rn] = {
                "std_map_lag1_x": cx,
                "std_map_lag1_y": cy,
                "std_p95_abs": float(np.percentile(tstd, 95)),
                "std_mean_abs": float(tstd.mean()),
            }
            print(f"[{arm}] {rn}: lag1=({cx:+.3f},{cy:+.3f}) std_p95={np.percentile(tstd,95):.3e}")
        out[arm] = rec
    dst = ROOT / "artifacts/day_0828/c_gi_r2/c_spatial_char.json"
    dst.write_text(json.dumps(out, indent=1, ensure_ascii=False), encoding="utf-8")
    print(f"-> {dst}")

    # 老新臂视觉：dark_table 收敛帧裁剪（ACES ×16;左 old_gi | 右 gi2_c001）。
    def to8(x):
        a, b, c, d, e = 2.51, 0.03, 2.43, 0.59, 0.14
        t = np.clip((x * 16.0 * (a * x * 16.0 + b)) / (x * 16.0 * (c * x * 16.0 + d) + e), 0.0, 1.0)
        s = np.where(t <= 0.0031308, t * 12.92, 1.055 * np.power(t, 1 / 2.4) - 0.055)
        return (np.clip(s, 0, 1) * 255.0 + 0.5).astype(np.uint8)

    for rn, (x, y, w, h) in ROIS.items():
        def conv_rgb(base):
            f = decode_exr((base / "converged.exr").read_bytes(), expected_end="rurix")
            px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
            return px[y : y + h, x : x + w, :]

        l8 = to8(conv_rgb(ARMS["old_gi_sinhash"]))
        r8 = to8(conv_rgb(ARMS["gi2_on_c001"]))
        div = np.full((h, 4, 3), 255, dtype=np.uint8)
        im = Image.fromarray(np.concatenate([l8, div, r8], axis=1), "RGB")
        im = im.resize((im.width * 2, im.height * 2), Image.NEAREST)
        im.save(ROOT / "artifacts/day_0828/c_gi_r2/png" / f"oldgi_vs_gi2_{rn}.png")
        print(f"  -> png/oldgi_vs_gi2_{rn}.png")
    return 0


if __name__ == "__main__":
    sys.exit(main())
