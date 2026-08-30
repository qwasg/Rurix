#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase F F4 视觉主证 + ROI 指标。

输入：png/off_96.raw / png/on_96.raw（1080p BGRA8 w/h u32 LE 头）+
png/on_4k.raw（4K）。输出：
- 全帧 PNG（off/on/4K）;
- 三灯具裁剪对照（off|on 并排 ×3 nearest,标注）：吊灯（罩+灯泡）/灯笼/吊扇;
  壁灯 mat59 契约相机 0 顶点在框（locate_lamps.py 在案）→ 探针替补登记;
- 4K 吊灯原生裁剪;
- f4_visual_metrics.json：逐灯具「OFF 饱和血块」像素集（off luma ≥250 的
  bbox 内像素——同像素集 off/on 对比,判据不自证）：mean_off / mean_on /
  p99_on / frac≥250_on;吊灯判据 = 罩区（blob 均值）<250 离开饱和 且 bulb
  亮点（blob p99）≥250;全图 mean off/on 差（AE 补偿稳定性）。
"""
from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

F = Path(__file__).resolve().parent
PNG = F / "png"

# 1080p 契约相机灯具窗（locate_lamps.py 簇 bbox + 边距;x0,y0,x1,y1）。
ROIS = {
    "ceiling_lamp": (1496, 84, 1652, 228),
    "lantern": (800, 204, 900, 324),
    "lantern_2": (996, 252, 1092, 348),
    "fan": (1452, 0, 1716, 132),
}


def read_raw(p: Path) -> np.ndarray:
    b = p.read_bytes()
    w, h = struct.unpack_from("<II", b, 0)
    a = np.frombuffer(b, dtype=np.uint8, count=w * h * 4, offset=8).reshape(h, w, 4)
    return a[..., [2, 1, 0]].copy()  # BGRA → RGB


def luma(rgb: np.ndarray) -> np.ndarray:
    f = rgb.astype(np.float64)
    return 0.2126 * f[..., 0] + 0.7152 * f[..., 1] + 0.0722 * f[..., 2]


def side_by_side(off: np.ndarray, on: np.ndarray, box, scale: int, label: str, out: Path):
    x0, y0, x1, y1 = box
    a = off[y0:y1, x0:x1]
    b = on[y0:y1, x0:x1]
    h, w = a.shape[:2]
    div = 4
    canvas = np.full((h, w * 2 + div, 3), 32, dtype=np.uint8)
    canvas[:, :w] = a
    canvas[:, w + div:] = b
    img = Image.fromarray(canvas).resize(((w * 2 + div) * scale, h * scale), Image.NEAREST)
    d = ImageDraw.Draw(img)
    d.text((4, 2), f"{label}  em OFF", fill=(255, 220, 40))
    d.text(((w + div) * scale + 4, 2), "em ON", fill=(80, 255, 120))
    img.save(out)
    return out


def main() -> int:
    off = read_raw(PNG / "off_96.raw")
    on = read_raw(PNG / "on_96.raw")
    Image.fromarray(off).save(PNG / "off_96.png")
    Image.fromarray(on).save(PNG / "on_96.png")
    lo, ln = luma(off), luma(on)

    metrics: dict = {
        "schema": "rurix.day0828.f_emissive.f4_visual_metrics.v1",
        "protocol": "逐灯具像素集 = 1080p 契约相机窗内 off-luma≥250 饱和血块（同像素集 off/on 对比,判据不自证）;luma = BT.709 8bit",
        "full_frame": {
            "mean_off": round(float(lo.mean()), 4),
            "mean_on": round(float(ln.mean()), 4),
            "abs_delta": round(abs(float(lo.mean()) - float(ln.mean())), 4),
        },
        "rois": {},
    }
    for name, box in ROIS.items():
        x0, y0, x1, y1 = box
        wl_o = lo[y0:y1, x0:x1]
        wl_n = ln[y0:y1, x0:x1]
        blob = wl_o >= 250.0
        npx = int(blob.sum())
        row: dict = {"bbox_px": list(box), "blob_px": npx}
        if npx >= 16:
            bo = wl_o[blob]
            bn = wl_n[blob]
            row.update({
                "blob_mean_off": round(float(bo.mean()), 3),
                "blob_mean_on": round(float(bn.mean()), 3),
                "blob_p99_on": round(float(np.percentile(bn, 99)), 3),
                "blob_frac_ge250_on": round(float((bn >= 250).mean()), 5),
                "blob_frac_lt250_on": round(float((bn < 250).mean()), 5),
            })
        metrics["rois"][name] = row
        side_by_side(off, on, box, 3, name, PNG / f"crop_{name}.png")

    # 判据（任务口径:罩区〔shade〕与 bulb 亮点区分开——几何切分:吊灯 bbox 上
    # 62% 行 = 罩壳 dome,下 38% 行 = bulb 发光底盘〔crop 目检定界〕;像素集
    # 仍 = off 饱和血块 ∩ 行带,同像素集 off/on 对比）。
    x0, y0, x1, y1 = ROIS["ceiling_lamp"]
    ys = y0 + int((y1 - y0) * 0.62)
    wl_o = lo[y0:y1, x0:x1]
    wl_n = ln[y0:y1, x0:x1]
    blob = wl_o >= 250.0
    rows = np.arange(y0, y1)[:, None] * np.ones((1, x1 - x0))
    shade_m = blob & (rows < ys)
    bulb_m = blob & (rows >= ys)
    sub = {}
    for nm, m in (("shade", shade_m), ("bulb", bulb_m)):
        bo = wl_o[m]
        bn = wl_n[m]
        sub[nm] = {
            "rows": [y0, ys] if nm == "shade" else [ys, y1],
            "px": int(m.sum()),
            "mean_off": round(float(bo.mean()), 3),
            "mean_on": round(float(bn.mean()), 3),
            "p99_on": round(float(np.percentile(bn, 99)), 3),
            "frac_lt250_on": round(float((bn < 250).mean()), 5),
        }
    metrics["rois"]["ceiling_lamp"]["split"] = sub
    metrics["verdict"] = {
        "ceiling_lamp_shade_mean_on_lt_250": sub["shade"]["mean_on"] < 250.0,
        "ceiling_lamp_bulb_region_ge_250": sub["bulb"]["mean_on"] >= 250.0
        or sub["bulb"]["p99_on"] >= 250.0,
        "full_mean_stable_lt_2": metrics["full_frame"]["abs_delta"] < 2.0,
        "wall_light_note": "mat 59 契约相机 0 顶点在框（lamp_screen_rois.json）——视觉替补 = 探针双臂覆盖槽 73（SSBO p100=0 位级 + sampler ≤1LSB）+ heap 入位登记,Phase B curtainB1 同律",
    }

    # 4K：全帧 + 吊灯原生裁剪（同相机 ⇒ 1080p 坐标 ×2）。
    p4 = PNG / "on_4k.raw"
    if p4.is_file():
        on4 = read_raw(p4)
        Image.fromarray(on4).save(PNG / "on_4k.png")
        x0, y0, x1, y1 = [v * 2 for v in ROIS["ceiling_lamp"]]
        Image.fromarray(on4[y0:y1, x0:x1]).resize(((x1 - x0) * 2, (y1 - y0) * 2), Image.NEAREST).save(
            PNG / "crop_4k_ceiling_lamp_native.png")
        l4 = luma(on4)
        wl4 = l4[y0:y1, x0:x1]
        metrics["uhd_4k"] = {
            "full_mean": round(float(l4.mean()), 4),
            "ceiling_lamp_crop_mean": round(float(wl4.mean()), 3),
            "ceiling_lamp_crop_p99": round(float(np.percentile(wl4, 99)), 3),
        }

    (F / "f4_visual_metrics.json").write_text(
        json.dumps(metrics, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(metrics["verdict"], ensure_ascii=False, indent=1))
    print(json.dumps(metrics["rois"]["ceiling_lamp"], ensure_ascii=False))
    print("full", metrics["full_frame"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
