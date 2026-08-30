#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase C GI2 视觉对照：暗部裁剪（gi2 off vs on）。

面 1 = 窗口 presented dump（display 域,ACES+BT.1886 后 BGRA8——真实显示面）：
  off = artifacts/day_0828/b_textures/png/combo_tex.raw（七臂锚 8b1c12f3 dump,同相机同组合）
  on  = artifacts/day_0828/c_gi_r2/png/combo8_gi2.raw（八臂 0e6ca110 dump）
面 2 = bench 收敛 EXR（scene-linear,crop_tool 同式 ACES fitted ×16 曝光）：
  off/on = arms/gi2_{off,on}_render converged.exr
输出 side-by-side（左 off | 右 on,×2 nearest 放大）→ png/。
"""
from __future__ import annotations

import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np
from PIL import Image

from g10_exr_lib import decode_exr

OUT = ROOT / "artifacts/day_0828/c_gi_r2/png"
ROIS = {
    "arch": (360, 0, 360, 180),
    "table": (560, 560, 560, 200),
    "bar": (1200, 650, 480, 240),
}


def load_raw(p: Path) -> np.ndarray:
    b = p.read_bytes()
    w, h = struct.unpack_from("<II", b, 0)
    px = np.frombuffer(b, dtype=np.uint8, count=w * h * 4, offset=8).reshape(h, w, 4)
    return px[..., [2, 1, 0]]  # BGRA -> RGB


def load_exr_rgb(p: Path) -> np.ndarray:
    f = decode_exr(p.read_bytes(), expected_end="rurix")
    return np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)


def aces_fitted(x: np.ndarray) -> np.ndarray:
    a, b, c, d, e = 2.51, 0.03, 2.43, 0.59, 0.14
    return np.clip((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0)


def srgb_encode(x: np.ndarray) -> np.ndarray:
    x = np.clip(x, 0.0, 1.0)
    return np.where(x <= 0.0031308, x * 12.92, 1.055 * np.power(x, 1.0 / 2.4) - 0.055)


def side_by_side(l8: np.ndarray, r8: np.ndarray, dst: Path, scale: int = 2) -> None:
    h, w, _ = l8.shape
    div = np.full((h, 4, 3), 255, dtype=np.uint8)
    combo = np.concatenate([l8, div, r8], axis=1)
    im = Image.fromarray(combo, "RGB")
    im = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
    im.save(dst)
    print(f"  {dst.name} ({im.width}x{im.height})")


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    # 面 1：presented 域
    off8 = load_raw(ROOT / "artifacts/day_0828/b_textures/png/combo_tex.raw")
    on8 = load_raw(ROOT / "artifacts/day_0828/c_gi_r2/png/combo8_gi2.raw")
    print("presented 域裁剪（左 off=8b1c12f3 dump | 右 on=0e6ca110 dump）:")
    for rn, (x, y, w, h) in ROIS.items():
        side_by_side(off8[y : y + h, x : x + w], on8[y : y + h, x : x + w], OUT / f"crop_{rn}_off_vs_on.png")
    Image.fromarray(off8, "RGB").save(OUT / "full_combo7_off.png")
    Image.fromarray(on8, "RGB").save(OUT / "full_combo8_gi2_on.png")
    print("  full_combo7_off.png / full_combo8_gi2_on.png")
    # 面 2：bench 收敛 EXR（ACES fitted ×16）
    exr_off = load_exr_rgb(ROOT / "artifacts/day_0828/c_gi_r2/arms/gi2_off_render/bistro-interior/tier100/tsr_device/converged.exr")
    exr_on = load_exr_rgb(ROOT / "artifacts/day_0828/c_gi_r2/arms/gi2_on_render/bistro-interior/tier100/tsr_device/converged.exr")
    to8 = lambda x: (srgb_encode(aces_fitted(x * 16.0)) * 255.0 + 0.5).astype(np.uint8)
    print("bench 收敛 EXR 裁剪（ACES ×16;左 off | 右 on）:")
    for rn, (x, y, w, h) in ROIS.items():
        side_by_side(to8(exr_off[y : y + h, x : x + w]), to8(exr_on[y : y + h, x : x + w]), OUT / f"exr_{rn}_off_vs_on.png")
    return 0


if __name__ == "__main__":
    sys.exit(main())
