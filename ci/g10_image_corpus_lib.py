#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.4a 波）
"""G10.4a M136 对拍图集生成器（spec/visual_comparison.md RXS-0387 L4 下界面）。

**图集下界（语义冻结）**：图集 ≥ 24 图对；内容类五类每类 ≥ 4——
`high_freq_edge`（高频边缘）/ `smooth_gradient`（平滑渐变）/ `noise`（噪声）/
`highlight_clip`（高亮截断）/ `color_isolation`（色彩孤立区）；每对 =
（A, B=A+类特征扰动）。本生成器产 **5 类 × 5 对 = 25 图对**（满足下界且
留一对余量），全部闭式确定性（xorshift 固定 seed，零随机量/零外部文件），
LDR 显示域 sRGB `[0,1]` 帧。

每图 digest = `sha256("G10IMGD-1\\0" ‖ w u32le ‖ h u32le ‖ channels=3 ‖
f32 LE 像素字节)`；图集清单与每图 digest 入 evidence（稀释通道封堵——
「一张平色图满足字面」不成立）。
"""
from __future__ import annotations

import hashlib
import struct

import numpy as np

CLASSES = ("high_freq_edge", "smooth_gradient", "noise", "highlight_clip", "color_isolation")
PAIRS_PER_CLASS = 5
IMG_W = 64
IMG_H = 64

MIN_PAIRS_TOTAL = 24
MIN_PAIRS_PER_CLASS = 4


def _xorshift32(state: int) -> int:
    state ^= (state << 13) & 0xFFFFFFFF
    state ^= state >> 17
    state ^= (state << 5) & 0xFFFFFFFF
    return state & 0xFFFFFFFF


def _noise_field(seed: int, w: int, h: int, channels: int = 3) -> np.ndarray:
    n = w * h * channels
    out = np.empty(n, dtype=np.float64)
    s = seed & 0xFFFFFFFF
    for i in range(n):
        s = _xorshift32(s)
        out[i] = (s >> 8) / float(1 << 24)
    return out.reshape(h, w, channels)


def image_digest(img: np.ndarray) -> str:
    """图 digest（图集清单登记面；f32 LE 像素字节 canonical）。"""
    h, w, c = img.shape
    payload = b"G10IMGD-1\x00" + struct.pack("<IIB", w, h, c) + img.astype("<f4").tobytes()
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def _pair_high_freq_edge(idx: int) -> tuple[np.ndarray, np.ndarray, str]:
    """高频边缘：1px 棋盘格 + 竖条带；扰动 = 1px 位移 + 相位翻转。"""
    y, x = np.mgrid[0:IMG_H, 0:IMG_W]
    checker = ((x + y) % 2).astype(np.float64)
    stripes = ((x // (1 + idx % 3)) % 2).astype(np.float64)
    a = np.stack([checker, stripes, 1.0 - checker], axis=-1) * 0.9 + 0.05
    shift = 1 + idx % 2
    b = np.roll(a, shift=shift, axis=1)
    if idx % 2 == 1:
        b = 1.0 - b
    return a.astype(np.float64), b.astype(np.float64), f"checker{1 + idx % 3}px-shift{shift}"


def _pair_smooth_gradient(idx: int) -> tuple[np.ndarray, np.ndarray, str]:
    """平滑渐变：双线性 / 径向渐变；扰动 = 增益 + 缓偏移。"""
    y, x = np.mgrid[0:IMG_H, 0:IMG_W]
    u = x / (IMG_W - 1)
    v = y / (IMG_H - 1)
    if idx % 2 == 0:
        base = np.stack([u, v, 0.5 * (u + v)], axis=-1)
    else:
        r = np.sqrt((u - 0.5) ** 2 + (v - 0.5) ** 2)
        base = np.stack([r, 1.0 - r, u * v], axis=-1)
    a = np.clip(base, 0.0, 1.0)
    gain = 1.0 - 0.05 * (idx + 1)
    b = np.clip(a * gain + 0.01 * (idx + 1), 0.0, 1.0)
    return a, b, f"grad-gain{gain:.2f}"


def _pair_noise(idx: int) -> tuple[np.ndarray, np.ndarray, str]:
    """噪声：xorshift 均匀噪声场；扰动 = 小幅加性噪声 delta。"""
    a = _noise_field(0xC0FFEE + idx * 7919, IMG_W, IMG_H)
    delta = (_noise_field(0xBADC0DE + idx * 104729, IMG_W, IMG_H) - 0.5) * 0.06
    b = np.clip(a + delta, 0.0, 1.0)
    return a, b, f"noise-delta0.06-seed{idx}"


def _pair_highlight_clip(idx: int) -> tuple[np.ndarray, np.ndarray, str]:
    """高亮截断：A 含近饱和高亮带；B = 低截断点 clip（clip 差）。"""
    y, x = np.mgrid[0:IMG_H, 0:IMG_W]
    band = np.exp(-(((x - IMG_W / 2) / (4.0 + idx)) ** 2))
    a = np.stack(
        [0.3 + 0.7 * band, 0.2 + 0.75 * band, 0.4 + 0.6 * band], axis=-1
    )
    a = np.clip(a, 0.0, 1.0)
    clip = 0.95 - 0.03 * idx
    b = np.minimum(a, clip)
    return a, b, f"clip{clip:.2f}"


def _pair_color_isolation(idx: int) -> tuple[np.ndarray, np.ndarray, str]:
    """色彩孤立区：平坦灰底 + 孤立饱和色块；扰动 = 色块色相/饱和度变化。"""
    a = np.full((IMG_H, IMG_W, 3), 0.5, dtype=np.float64)
    rng_pos = [(8 + idx * 3, 10), (40, 30 + idx * 2), (20 + idx, 50)]
    colors = [(0.9, 0.1, 0.1), (0.1, 0.9, 0.2), (0.15, 0.3, 0.95)]
    for (cx, cy), col in zip(rng_pos, colors):
        a[cy:cy + 6, cx:cx + 6] = col
    b = a.copy()
    # 扰动：孤立色块变色（面积小、色差大）。
    for (cx, cy), col in zip(rng_pos, colors):
        b[cy:cy + 6, cx:cx + 6] = (col[2], col[0], col[1])
    return a, b, f"iso-block3-idx{idx}"


_GENERATORS = {
    "high_freq_edge": _pair_high_freq_edge,
    "smooth_gradient": _pair_smooth_gradient,
    "noise": _pair_noise,
    "highlight_clip": _pair_highlight_clip,
    "color_isolation": _pair_color_isolation,
}


def generate_corpus() -> list[dict]:
    """产 25 图对清单（含每图 digest 与类标签；确定性可复跑）。"""
    pairs: list[dict] = []
    for cls in CLASSES:
        gen = _GENERATORS[cls]
        for idx in range(PAIRS_PER_CLASS):
            a, b, variant = gen(idx)
            assert a.shape == (IMG_H, IMG_W, 3) and b.shape == a.shape
            assert 0.0 <= float(a.min()) and float(a.max()) <= 1.0
            assert 0.0 <= float(b.min()) and float(b.max()) <= 1.0
            pairs.append({
                "pair_id": f"{cls}-{idx}",
                "content_class": cls,
                "variant": variant,
                "a": a,
                "b": b,
                "a_digest": image_digest(a),
                "b_digest": image_digest(b),
            })
    return pairs


def corpus_manifest(pairs: list[dict]) -> dict:
    """图集清单（类 × 计数对账 + 每图 digest + 清单级 digest）。"""
    per_class: dict[str, int] = {}
    for p in pairs:
        per_class[p["content_class"]] = per_class.get(p["content_class"], 0) + 1
    listing = [
        {
            "pair_id": p["pair_id"],
            "content_class": p["content_class"],
            "a_digest": p["a_digest"],
            "b_digest": p["b_digest"],
        }
        for p in pairs
    ]
    canonical = repr(sorted((d["pair_id"], d["a_digest"], d["b_digest"]) for d in listing))
    return {
        "pair_count": len(pairs),
        "per_class": per_class,
        "pairs": listing,
        "manifest_digest": "sha256:" + hashlib.sha256(canonical.encode("utf-8")).hexdigest(),
    }


def lower_bound_failures(manifest: dict) -> list[str]:
    """图集下界机核（≥24 图对、五类每类 ≥4；不满足即标定无效）。"""
    fails: list[str] = []
    if manifest["pair_count"] < MIN_PAIRS_TOTAL:
        fails.append(f"图集 {manifest['pair_count']} 对 < 下界 {MIN_PAIRS_TOTAL}")
    for cls in CLASSES:
        n = manifest["per_class"].get(cls, 0)
        if n < MIN_PAIRS_PER_CLASS:
            fails.append(f"内容类 {cls} {n} 对 < 下界 {MIN_PAIRS_PER_CLASS}")
    return fails
