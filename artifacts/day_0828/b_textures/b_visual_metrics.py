#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0828 Phase B 验收工具：三验收位裁剪对照 + 恒定色连通域指标。

输入 = presented raw dump（8B w/h 头 + BGRA8）：combo_off.raw（六臂无纹理）
vs combo_tex.raw（七臂含纹理）。
- 裁剪位：右墙 Paris_Paintings（画作）/ 红墙 Plaster_Red（灰泥）/ 地板
  floor_tile_hex（替补——curtainB1/curtainA 契约相机全帧外,前向投影 0 顶点
  在框,如实登记）;off|on 并排 ×2 放大 PNG。
- 色块指标：RGB>>3 量化 → 恒定色 8 连通域标注 → ≥阈值面积连通域计数/总面
  积/最大域,on vs off 对照（大色块显著缩减 = 均值马赛克修复的量化证据）。
"""
from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

HERE = Path(__file__).resolve().parent
W, H = 1920, 1080


def load_raw(p: Path) -> np.ndarray:
    b = p.read_bytes()
    w, h = struct.unpack_from("<II", b, 0)
    assert (w, h) == (W, H), (w, h)
    a = np.frombuffer(b, dtype=np.uint8, offset=8, count=W * H * 4).reshape(H, W, 4)
    return a[:, :, [2, 1, 0]].copy()  # BGRA -> RGB


def crop_pair(off: np.ndarray, on: np.ndarray, label: str, x: int, y: int, w: int, h: int) -> dict:
    co = off[y : y + h, x : x + w]
    cn = on[y : y + h, x : x + w]
    gap = np.full((h, 6, 3), 255, dtype=np.uint8)
    pair = np.concatenate([co, gap, cn], axis=1)
    img = Image.fromarray(pair, "RGB").resize(((w * 2 + 6) * 2, h * 2), Image.NEAREST)
    out = HERE / "png" / f"crop_{label}_off_vs_on.png"
    img.save(out)
    stats = {}
    for tag, c in (("off", co), ("on", cn)):
        f = c.reshape(-1, 3).astype(np.float64)
        stats[tag] = {
            "mean_rgb": [round(v, 2) for v in f.mean(0)],
            "std_rgb": [round(v, 2) for v in f.std(0)],
        }
    stats["window"] = [x, y, w, h]
    stats["png"] = str(out.relative_to(HERE.parents[2])).replace("\\", "/")
    return stats


def flat_components(img: np.ndarray, min_area: int) -> dict:
    """恒定色连通域：RGB>>3 量化后逐量化色 8 连通标注,统计 ≥min_area 域。"""
    q = (img >> 3).astype(np.uint16)
    key = (q[:, :, 0] << 10) | (q[:, :, 1] << 5) | q[:, :, 2]
    big_count = 0
    big_area = 0
    largest: list[int] = []
    for color in np.unique(key):
        mask = key == color
        if mask.sum() < min_area:
            continue
        lab, n = ndimage.label(mask, structure=np.ones((3, 3), dtype=np.int8))
        if n == 0:
            continue
        sizes = np.bincount(lab.ravel())[1:]
        sel = sizes[sizes >= min_area]
        big_count += int(sel.size)
        big_area += int(sel.sum())
        largest.extend(int(v) for v in sel)
    largest.sort(reverse=True)
    return {
        "min_area_px": min_area,
        "components_ge_min": big_count,
        "area_ge_min_px": big_area,
        "area_ge_min_frac": round(big_area / (W * H), 6),
        "top8_components_px": largest[:8],
    }


def main() -> int:
    off = load_raw(HERE / "png" / "combo_off.raw")
    on = load_raw(HERE / "png" / "combo_tex.raw")
    report: dict = {"schema": "rurix.day0828.b_textures.visual_metrics.v1"}
    # 三验收位（curtainB1/curtainA 契约相机全帧外——floor_tile_hex 替补,
    # mat_screen_bbox.json 前向投影定位在案）。
    report["crops"] = {
        "paris_paintings_right_wall": crop_pair(off, on, "paintings", 1330, 270, 590, 230),
        "plaster_red_wall": crop_pair(off, on, "plaster_red", 1620, 100, 280, 170),
        "floor_tile_hex_substitute": crop_pair(off, on, "floor_tile", 700, 700, 520, 300),
    }
    report["curtain_note"] = (
        "curtainB1(mat14)/curtainA(mat13) 契约相机前向投影 0 顶点在框（centroid 距相机 ~4.3m,方位角 −82°）;"
        "orbit/dolly 轨迹摆头 ±17°/±11° 不可达 ⇒ 三验收位以 floor_tile_hex 替补,curtainB1 槽位覆盖走 heap 装载"
        "（slot 映射 + 探针 p100=0.0 三 mip 级）登记"
    )
    # 色块指标（≥5000 px 恒定量化色连通域;全帧）。
    report["color_blocks"] = {
        "off": flat_components(off, 5000),
        "on": flat_components(on, 5000),
    }
    ob = report["color_blocks"]["off"]
    nb = report["color_blocks"]["on"]
    report["color_blocks"]["reduction"] = {
        "area_frac_off": ob["area_ge_min_frac"],
        "area_frac_on": nb["area_ge_min_frac"],
        "area_reduction_pct": round(
            100.0 * (1.0 - nb["area_ge_min_px"] / max(ob["area_ge_min_px"], 1)), 2
        ),
        "count_off": ob["components_ge_min"],
        "count_on": nb["components_ge_min"],
    }
    outp = HERE / "visual_metrics.json"
    outp.write_text(json.dumps(report, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(report["color_blocks"], ensure_ascii=False, indent=1))
    print("crops + metrics ->", outp)
    return 0


if __name__ == "__main__":
    sys.exit(main())
