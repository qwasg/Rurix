#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D1 验证：我的 IGN TPDF 公式（与 g31_display_encode.rx 逐字同式）在真实
收敛帧上的色带消除效果 + 与 kernel 的公式一致性。

指标（沿前序会话 dither_metrics 同口径）：渐变区 8-bit 唯一色阶数 ↑ /
恒定段平均长度 ↓ ⇒ 色带消除。

用法: py -3 verify_dither.py <converged.exr> [--roi x y w h]
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np

from g10_exr_lib import decode_exr


def aces_fitted(x):
    a, b, c, d, e = 2.51, 0.03, 2.43, 0.59, 0.14
    return np.clip((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0)


def srgb_encode(x):
    x = np.clip(x, 0.0, 1.0)
    return np.where(x <= 0.0031308, x * 12.92, 1.055 * np.power(x, 1.0 / 2.4) - 0.055)


def ign_tpdf(w: int, h: int) -> np.ndarray:
    """与 kernel 逐字同式（f64 复算 f32 公式；空间结构一致）。"""
    fy, fx = np.mgrid[0:h, 0:w].astype(np.float64)[0], np.mgrid[0:h, 0:w].astype(np.float64)[1]
    t1 = fx * 0.06711056 + fy * 0.00583715
    t2 = fx * 0.00583715 + fy * 0.06711056 + 0.37045599
    f1 = t1 - np.floor(t1)
    f2 = t2 - np.floor(t2)
    g1 = 52.9829189 * f1
    g2 = 52.9829189 * f2
    r1 = g1 - np.floor(g1)
    r2 = g2 - np.floor(g2)
    return r1 - r2  # (−1,1) TPDF


def quant8(v: np.ndarray, dn: np.ndarray | None) -> np.ndarray:
    x = v * 255.0 + 0.5
    if dn is not None:
        x = x + dn
    return np.clip(np.floor(x), 0.0, 255.0)


def banding_metrics(q: np.ndarray, blocks: list[tuple[int, int]]) -> dict:
    levels, runlens = [], []
    for by, bx in blocks:
        blk = q[by:by + 32, bx:bx + 32]
        levels.append(float(len(np.unique(blk))))
        rl = []
        for row in blk:
            run = 1
            for i in range(1, len(row)):
                if row[i] == row[i - 1]:
                    run += 1
                else:
                    rl.append(run)
                    run = 1
            rl.append(run)
        runlens.append(float(np.mean(rl)))
    return {"unique_levels_mean": float(np.mean(levels)), "const_runlen_mean_px": float(np.mean(runlens))}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("src")
    ap.add_argument("--roi", type=int, nargs=4, default=[1400, 150, 480, 270])
    ap.add_argument("--out", default=None)
    args = ap.parse_args()

    f = decode_exr(Path(args.src).read_bytes(), expected_end="rurix")
    w, h = f["width"], f["height"]
    px = np.array(f["pixels"], dtype=np.float64).reshape(h, w, 3)
    x, y, rw, rh = args.roi
    roi = px[y:y + rh, x:x + rw, :]
    disp = srgb_encode(aces_fitted(roi * 1.0))

    dn = ign_tpdf(rw, rh)
    blocks = [(by, bx) for by in range(0, rh - 32, 64) for bx in range(0, rw - 32, 64)]
    res = {"roi": args.roi, "blocks": len(blocks)}
    for ch, name in enumerate("rgb"):
        q_no = quant8(disp[..., ch], None)
        q_tp = quant8(disp[..., ch], dn)
        res[f"ch_{name}"] = {
            "no_dither": banding_metrics(q_no, blocks),
            "tpdf": banding_metrics(q_tp, blocks),
        }
    print(json.dumps(res, indent=2, ensure_ascii=False))
    if args.out:
        Path(args.out).write_text(json.dumps(res, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
