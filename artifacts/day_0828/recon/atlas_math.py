#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0828 recon: 全 70 材质唯一贴图清单 + 图集/显存方案数学。"""
from __future__ import annotations

import json
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
d = json.loads((HERE / "material_census.json").read_text(encoding="utf-8"))

uniq: dict[str, dict] = {}
mat_per_uri = Counter()
for r in d["materials"]:
    if r["texture_uri"]:
        mat_per_uri[r["texture_uri"]] += 1
        uniq.setdefault(r["texture_uri"], r["dds"])

sizes = Counter((v["w"], v["h"], v["fourcc"], v["mips"]) for v in uniq.values())
print(f"materials={len(d['materials'])} unique_base_color_dds={len(uniq)}")
for (w, h, fmt, mips), n in sorted(sizes.items(), key=lambda kv: -kv[0][0]):
    print(f"  {w:5d}x{h:<5d} {fmt} mips={mips:2d}: {n} 张")
ge1024 = sum(1 for v in uniq.values() if v["w"] >= 1024 and v["h"] >= 1024)
print(f">=1024^2: {ge1024} 张")

mip0_texels = sum(v["w"] * v["h"] for v in uniq.values())
mipchain_texels = 0
for v in uniq.values():
    w, h = v["w"], v["h"]
    while True:
        mipchain_texels += w * h
        if w == 1 and h == 1:
            break
        w = max(1, w // 2)
        h = max(1, h // 2)

MB = 1024 * 1024
print(f"\nmip0 texels total = {mip0_texels:,} ({mip0_texels/1e6:.1f} MTex)")
print(f"mip-chain texels total = {mipchain_texels:,} (×{mipchain_texels/mip0_texels:.4f})")
print(f"\n== 存储形态 ==")
print(f"u32 packed RGBA8 (现形态 4B/texel): mip0 {mip0_texels*4/MB:.0f} MiB | 全链 {mipchain_texels*4/MB:.0f} MiB")
print(f"f32 RGB (12B/texel):               mip0 {mip0_texels*12/MB:.0f} MiB | 全链 {mipchain_texels*12/MB:.0f} MiB")

print(f"\n== 方案 A: 2048 瓦片网格扩容（现律法直扩）==")
import math
n = len(uniq)
cols = 8
rows = math.ceil(n / cols)
aw, ah = cols * 2048, rows * 2048
print(f"unique={n} → {cols}×{rows} 瓦 = {aw}×{ah} texel = {aw*ah*4/MB:.0f} MiB (mip0 only, u32)")
print(f"  padding 浪费 = {(aw*ah - mip0_texels)/ (aw*ah) *100:.1f}%")

print(f"\n== 方案 A': 1024 降采样瓦片 ==")
t1024 = sum(min(v['w'],1024) * min(v['h'],1024) for v in uniq.values())
rows1 = math.ceil(n / 8)
print(f"8×{rows1} 瓦 1024² = {8*1024}×{rows1*1024} = {8*1024*rows1*1024*4/MB:.0f} MiB (mip0, u32)")

print(f"\n== 方案 C: 线性 texel heap（无瓦片,逐槽偏移,mip 链直存）==")
print(f"mip0 only: {mip0_texels*4/MB:.0f} MiB | 全 mip 链: {mipchain_texels*4/MB:.0f} MiB (u32 packed, 零 padding)")
cap = 1024
heap_capped = 0
for v in uniq.values():
    w, h = min(v["w"], cap), min(v["h"], cap)
    while True:
        heap_capped += w * h
        if w == 1 and h == 1:
            break
        w = max(1, w // 2)
        h = max(1, h // 2)
print(f"cap {cap}²+全链: {heap_capped*4/MB:.0f} MiB")

shared = {u: c for u, c in mat_per_uri.items() if c > 1}
print(f"\nshared textures (multi-material): {shared}")
