#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase E1 工作项 6：hero 三段对照图 + 四问题特写。

三段：①战前生产默认（night hero_base.raw——蓝扇叶+死黑原始状态）
②Phase A 后（a3_tuning/g4.raw：灯光+AE+ACES 修复+g4 定档）
③终态 --quality full（e_final/png/full.raw：九臂预设新 dump）。
特写：曝光/死黑区（餐桌下）、色块区（画作墙）、噪点区（拱下）、蓝扇叶区
——各出一张 ①vs③ 放大对照（③ 含中段可读注记）。
raw 格式 = w/h u32 LE 头 + BGRA8（raw2png.py 同款）。
"""
from __future__ import annotations

import struct
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[3]
E = ROOT / "artifacts" / "day_0828" / "e_final"
PNG = E / "png"

SRC = {
    "base": ROOT / "artifacts" / "night_0828" / "hero" / "hero_base.raw",
    "phase_a": ROOT / "artifacts" / "day_0828" / "a3_tuning" / "g4.raw",
    "final": PNG / "full.raw",
}
LABELS = {
    "base": "① 战前生产默认（死黑 / ACES 转置 bug 蓝扇叶 / 均值色块 / 噪点）",
    "phase_a": "② Phase A 后（灯光提取 + 自动曝光 + ACES 修复,g4 定档）",
    "final": "③ 终态 --quality full（九臂:+纹理全覆盖 +GI2 +TSR 质量档）",
}
# (名称, 左上 x, y, w, h) —— B/C/D 相 ROI 同位。
CLOSEUPS = [
    ("exposure_dead_black", "曝光/死黑区（餐桌下）", 560, 560, 560, 200),
    ("color_blocks_paintings", "色块区（画作墙）", 1400, 150, 480, 270),
    ("grain_dark_arch", "噪点区（拱下）", 360, 0, 360, 180),
    ("blue_fan", "蓝扇叶区（吊扇）", 1330, 0, 400, 220),
]


def load_raw(p: Path) -> Image.Image:
    b = p.read_bytes()
    w, h = struct.unpack_from("<II", b, 0)
    return Image.frombytes("RGBA", (w, h), bytes(b[8: 8 + w * h * 4]), "raw", "BGRA").convert("RGB")


def font(size: int):
    for name in ("msyh.ttc", "msyhbd.ttc", "simhei.ttf"):
        try:
            return ImageFont.truetype(f"C:/Windows/Fonts/{name}", size)
        except OSError:
            continue
    return ImageFont.load_default()


def banner(img: Image.Image, text: str, size: int = 30) -> Image.Image:
    bar = 46
    out = Image.new("RGB", (img.width, img.height + bar), (12, 12, 12))
    out.paste(img, (0, bar))
    ImageDraw.Draw(out).text((14, 8), text, fill=(235, 235, 235), font=font(size))
    return out


def main() -> int:
    PNG.mkdir(parents=True, exist_ok=True)
    imgs = {k: load_raw(p) for k, p in SRC.items()}
    # 三段纵拼主图。
    panels = [banner(imgs[k], LABELS[k]) for k in ("base", "phase_a", "final")]
    w = max(p.width for p in panels)
    hero = Image.new("RGB", (w, sum(p.height for p in panels) + 2 * 4), (0, 0, 0))
    y = 0
    for p in panels:
        hero.paste(p, (0, y))
        y += p.height + 4
    hero_path = E / "hero_campaign_before_after.png"
    hero.save(hero_path)
    imgs["final"].save(PNG / "full.png")
    print(f"hero -> {hero_path} ({hero.width}x{hero.height})")
    # 四特写：①before vs ③after 横拼 2× 放大。
    for key, label, x, yy, cw, ch in CLOSEUPS:
        crops = []
        for tag, name in (("base", "战前默认"), ("final", "--quality full")):
            c = imgs[tag].crop((x, yy, x + cw, yy + ch)).resize((cw * 2, ch * 2), Image.NEAREST)
            crops.append(banner(c, f"{label}  {name}", size=24))
        pair = Image.new("RGB", (sum(c.width for c in crops) + 4, crops[0].height), (0, 0, 0))
        pair.paste(crops[0], (0, 0))
        pair.paste(crops[1], (crops[0].width + 4, 0))
        out = E / f"closeup_{key}.png"
        pair.save(out)
        print(f"closeup -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
