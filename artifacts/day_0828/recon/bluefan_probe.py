#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""bluefan recon: 产物×fan 区颜色对账 + 夜巡 receipt channel_order 取证。

fan 探针 (1500,12)；wall 探针 (1700,430)（右墙红灰泥参照）。
raw 文件按两种解释各报一次（file-order bytes vs BGRA→RGB），钉死通道序疑点。
"""
from __future__ import annotations

import json
import struct
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
N8 = ROOT / "artifacts" / "night_0828"
W, H = 1920, 1080
FAN = (1500, 12)
WALL = (1700, 430)

report: dict = {"receipts": {}, "raw": {}, "png": {}}

# ── 1) 夜巡 receipt：channel_order / spv_scene / visible ──
for tag, p in {
    "bloom_off": N8 / "d3_bloom" / "bloom_off_ev.json",
    "bloom_on": N8 / "d3_bloom" / "bloom_on_ev.json",
    "hero_base": N8 / "hero" / "hero_base_ev.json",
    "hero_full": N8 / "hero" / "hero_full_ev.json",
    "hero_ultimate": N8 / "hero" / "hero_ultimate_ev.json",
}.items():
    if not p.exists():
        continue
    try:
        j = json.loads(p.read_text(encoding="utf-8"))
    except Exception as e:  # noqa: BLE001
        report["receipts"][tag] = {"error": str(e)}
        continue

    def walk(o, keys=("channel_order", "visible", "spv_scene", "spv_encode")):
        found = {}

        def rec(x):
            if isinstance(x, dict):
                for k, v in x.items():
                    if k in keys and k not in found:
                        found[k] = v
                    rec(v)
            elif isinstance(x, list):
                for v in x:
                    rec(v)

        rec(o)
        return found

    report["receipts"][tag] = walk(j)

# ── 2) raw dump：两种通道序解释 ──
def probe_raw(p: Path) -> dict:
    b = p.read_bytes()
    w, h = struct.unpack_from("<II", b, 0)
    a = np.frombuffer(b, dtype=np.uint8, offset=8).reshape(h, w, 4)
    out = {"size": [int(w), int(h)]}
    for name, (x, y) in {"fan(1500,12)": FAN, "wall(1700,430)": WALL}.items():
        px = a[y, x]
        out[name] = {
            "file_bytes[b0,b1,b2,b3]": [int(v) for v in px],
            "if_BGRA→RGB": [int(px[2]), int(px[1]), int(px[0])],
            "if_RGBA→RGB": [int(px[0]), int(px[1]), int(px[2])],
        }
    # fan 区均值（主簇 x[1348,1686] y[0,197]）
    reg = a[0:198, 1348:1687].reshape(-1, 4).astype(np.float64)
    out["fan_region_mean_file_bytes"] = [round(v, 1) for v in reg.mean(0)]
    return out

for tag, p in {
    "bloom_off.raw": N8 / "d3_bloom" / "bloom_off.raw",
    "bloom_on.raw": N8 / "d3_bloom" / "bloom_on.raw",
    "hero_base.raw": N8 / "hero" / "hero_base.raw",
    "hero_full.raw": N8 / "hero" / "hero_full.raw",
    "hero_ultimate.raw": N8 / "hero" / "hero_ultimate.raw",
}.items():
    if p.exists():
        report["raw"][tag] = probe_raw(p)

# ── 3) PNG：直接读像素（PIL 已解到 RGB(A)）──
def probe_png(p: Path, fan=FAN, wall=WALL) -> dict:
    img = Image.open(p)
    out = {"mode": img.mode, "size": list(img.size)}
    im = img.convert("RGB")
    a = np.asarray(im)
    fx, fy = fan
    wx, wy = wall
    if fy < a.shape[0] and fx < a.shape[1]:
        out[f"fan({fx},{fy})_RGB"] = [int(v) for v in a[fy, fx]]
    if wy < a.shape[0] and wx < a.shape[1]:
        out[f"wall({wx},{wy})_RGB"] = [int(v) for v in a[wy, wx]]
    return out

png_list = {
    "bloom_off.png": N8 / "d3_bloom" / "bloom_off.png",
    "bloom_on.png": N8 / "d3_bloom" / "bloom_on.png",
    "hero_base.png": N8 / "hero" / "hero_base.png",
    "hero_full.png": N8 / "hero" / "hero_full.png",
    "hero_ultimate.png": N8 / "hero" / "hero_ultimate.png",
    "d2w_off_ref.png": N8 / "d2_window" / "off_ref.png",
}
for tag, p in png_list.items():
    if p.exists():
        report["png"][tag] = probe_png(p)

# before/after 拼图：探针位置未知拼法，先只记录尺寸；fan 探针按上下两半各测一次
for tag, p in {
    "ultimate_before_after.png": N8 / "hero" / "ultimate_before_after.png",
    "window_before_after.png": N8 / "hero" / "window_before_after.png",
}.items():
    if not p.exists():
        continue
    img = Image.open(p).convert("RGB")
    a = np.asarray(img)
    h2 = a.shape[0]
    entry = {"size": [img.size[0], img.size[1]]}
    # 猜测上下拼：fan 探针在上半 (1500,12) 与下半 (1500, h/2+12)
    if a.shape[1] > 1500:
        entry["top_fan(1500,12)_RGB"] = [int(v) for v in a[12, 1500]]
        if h2 >= 1080 * 2:
            entry["bottom_fan(1500,h/2+12)_RGB"] = [int(v) for v in a[h2 // 2 + 12, 1500]]
    report["png"][tag] = entry

# ── 4) bench 车道 16-bit PNG（presentation_night.png）：PIL 读不了，手工解码 ──
def decode_png16(p: Path) -> tuple[np.ndarray, dict]:
    """非交错 8/16-bit RGB(A)/灰 PNG 手工解码（zlib + 逐行 filter 还原）。"""
    import zlib

    data = p.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    off = 8
    idat = b""
    meta = {}
    while off < len(data):
        (ln,) = struct.unpack_from(">I", data, off)
        typ = data[off + 4 : off + 8]
        chunk = data[off + 8 : off + 8 + ln]
        if typ == b"IHDR":
            w, h, bd, ct, comp, filt, inter = struct.unpack(">IIBBBBB", chunk)
            meta = {"w": w, "h": h, "bit_depth": bd, "color_type": ct, "interlace": inter}
        elif typ == b"IDAT":
            idat += chunk
        elif typ == b"IEND":
            break
        off += 12 + ln
    assert meta.get("interlace") == 0, "交错 PNG 不支持"
    w, h, bd, ct = meta["w"], meta["h"], meta["bit_depth"], meta["color_type"]
    nch = {0: 1, 2: 3, 4: 2, 6: 4}[ct]
    bpp = nch * (bd // 8)
    raw = zlib.decompress(idat)
    stride = w * bpp
    out = np.zeros((h, stride), dtype=np.uint8)
    pos = 0
    prev = np.zeros(stride, dtype=np.uint8)
    for y in range(h):
        f = raw[pos]
        pos += 1
        line = np.frombuffer(raw, dtype=np.uint8, count=stride, offset=pos).copy()
        pos += stride
        if f == 0:
            pass
        elif f == 1:
            for i in range(bpp, stride):
                line[i] = (int(line[i]) + int(line[i - bpp])) & 0xFF
        elif f == 2:
            line[:] = (line.astype(np.int32) + prev.astype(np.int32)) & 0xFF
        elif f == 3:
            for i in range(stride):
                a_ = int(line[i - bpp]) if i >= bpp else 0
                line[i] = (int(line[i]) + ((a_ + int(prev[i])) >> 1)) & 0xFF
        elif f == 4:
            for i in range(stride):
                a_ = int(line[i - bpp]) if i >= bpp else 0
                b_ = int(prev[i])
                c_ = int(prev[i - bpp]) if i >= bpp else 0
                pa, pb, pc = abs(b_ - c_), abs(a_ - c_), abs(a_ + b_ - 2 * c_)
                pr = a_ if (pa <= pb and pa <= pc) else (b_ if pb <= pc else c_)
                line[i] = (int(line[i]) + pr) & 0xFF
        else:
            raise ValueError(f"filter {f}")
        out[y] = line
        prev = line
    if bd == 16:
        arr = out.reshape(h, w, nch, 2)
        px = (arr[..., 0].astype(np.uint16) << 8) | arr[..., 1]
    else:
        px = out.reshape(h, w, nch)
    return px, meta


for tag, p in {
    "presentation_night.png(quality_base)": N8
    / "arms"
    / "quality_base"
    / "bistro-interior"
    / "tier100"
    / "tsr_device"
    / "presentation_night.png",
}.items():
    if p.exists():
        try:
            a, meta = decode_png16(p)
            entry = {"meta": meta, "dtype": str(a.dtype)}
            if a.shape[0] > 430 and a.shape[1] > 1700:
                entry["fan(1500,12)_raw16"] = [int(v) for v in a[12, 1500][:3]]
                entry["wall(1700,430)_raw16"] = [int(v) for v in a[430, 1700][:3]]
                entry["fan(1500,12)_as8bit"] = [int(v) >> 8 for v in a[12, 1500][:3]]
                entry["wall(1700,430)_as8bit"] = [int(v) >> 8 for v in a[430, 1700][:3]]
        except Exception as e:  # noqa: BLE001
            entry = {"error": str(e)}
        report["png"][tag] = entry

# exr_view 产的 baseline pngs（搜 night_0828 顶层 *.png 含 aces/baseline 名）
for p in sorted(N8.glob("*.png")):
    if any(k in p.name.lower() for k in ("aces", "baseline")):
        report["png"][f"exr_view:{p.name}"] = probe_png(p)

out_p = HERE / "bluefan_probe_report.json"
out_p.write_text(json.dumps(report, ensure_ascii=False, indent=1), encoding="utf-8")
print(json.dumps(report, ensure_ascii=False, indent=1))
