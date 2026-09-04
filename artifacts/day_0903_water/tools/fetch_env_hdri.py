#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G41 展示场景环境光获取器:Poly Haven CC0 水景 HDRI → sky-view LUT。

用途
----
`g41_water_present` 的水面反射与环境光默认取 `world::sky` 的**程序化**天空
(Rayleigh + Mie + 臭氧)。本工具把一张**真实拍摄的水景 HDRI** 烘焙成同格式的
sky-view LUT,让展示图的天空/反射来自实拍环境而非解析模型。

为什么选 Poly Haven
-------------------
- **CC0-1.0**:位于 `milestones/g10/g10_asset_license_registry.json` 白名单
  (`["CC0-1.0","CC-BY-3.0","CC-BY-4.0"]`)的最宽松一档,无署名义务、可商用。
- **公开 API 免鉴权**:`https://api.polyhaven.com/files/<slug>` 直出下载直链,
  不需要 OAuth token(对比:Sketchfab 下载 API 强制 OAuth,需用户令牌)。
- 已有先例:`world::sky` 的四档预设即标定自 Poly Haven CC0「Pure Sky」系列。

水景候选(均 CC0,`t=hdris` 分类 nature):
  lakeside_sunrise / lakeside_dawn / secluded_beach / fish_hoek_beach /
  small_harbour_sunset / small_harbour_morning / shudu_lake / radkow_lake

资产纪律
--------
下载物落**缓存根**(默认 `K:/rurix_g10_cache/polyhaven-env/`),**不入 git**
(仓库 `.gitignore` 拒 `*.hdr`/`*.exr`;二进制资产一律留盘不入库)。入库的只有
本脚本 + 产物的 sha256 与许可登记 JSON。

LUT 格式(与 `world::sky::bake_sky_view_lut` 逐字同构)
------------------------------------------------------
  128 × 128 × 3 f32,行主序;u = (dot(dir, sun) + 1) / 2,v = (dir.y + 1) / 2。

**诚实边界**:该参数化假设辐亮度绕「天顶–太阳」轴旋转对称。真实 HDRI 不满足
此对称(岸线、山影、云团都是方位相关的),烘焙时对每个 (u, v) 格取所有命中
方向的**均值**,因此**丢失方位结构**,只保留亮度/色度的整体分布。用于水面
反射与环境光足够,不能当作全景背景。若要保方位结构,须改绑 equirect 环境图
(留窗,见 `rfcs/0050` §6)。

用法
----
    py -3 fetch_env_hdri.py --slug lakeside_sunrise
    py -3 fetch_env_hdri.py --slug secluded_beach --res 2k --out K:/.../env.lut
    py -3 fetch_env_hdri.py --list          # 只列水景候选与许可,不下载
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import struct
import sys
import urllib.request

API = "https://api.polyhaven.com"
DEFAULT_CACHE = os.environ.get("RURIX_G10_CACHE_ROOT", "K:/rurix_g10_cache")
LUT_W = 128
LUT_H = 128

WATER_SLUGS = [
    "lakeside_sunrise",
    "lakeside_dawn",
    "lakeside_night",
    "secluded_beach",
    "fish_hoek_beach",
    "small_harbour_sunset",
    "small_harbour_morning",
    "shudu_lake",
    "radkow_lake",
]


# Poly Haven 的 CDN 对默认 `Python-urllib` UA 返回 403,须带常规 UA。
_UA = "rurix-g41-env-fetch/1.0 (+https://github.com/qwasg/Rurix)"


def _req(url: str) -> urllib.request.Request:
    return urllib.request.Request(url, headers={"User-Agent": _UA, "Accept": "*/*"})


def http_json(url: str):
    with urllib.request.urlopen(_req(url), timeout=60) as r:
        return json.loads(r.read().decode("utf-8"))


def http_get(url: str, dest: str) -> str:
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    with urllib.request.urlopen(_req(url), timeout=600) as r, open(dest, "wb") as f:
        while True:
            chunk = r.read(1 << 20)
            if not chunk:
                break
            f.write(chunk)
    h = hashlib.sha256()
    with open(dest, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def read_radiance_hdr(path: str):
    """最小 Radiance .hdr(RGBE)读取器:返回 (width, height, [ (r,g,b) ... ])。

    只支持 `-Y H +X W` 扫描序与新式 RLE(以及非 RLE 直存),这覆盖 Poly Haven
    的全部产物。不引入第三方依赖(Pillow 不读 .hdr)。
    """
    with open(path, "rb") as f:
        data = f.read()
    # 头部以空行结束。
    pos = data.find(b"\n\n")
    if pos < 0:
        raise SystemExit("HDR: 未找到头部结束标记")
    header = data[:pos].decode("latin-1")
    if "32-bit_rle_rgbe" not in header and "RADIANCE" not in header:
        raise SystemExit("HDR: 非 RADIANCE RGBE 文件")
    pos += 2
    nl = data.find(b"\n", pos)
    dims = data[pos:nl].decode("latin-1").split()
    if len(dims) != 4 or dims[0] != "-Y" or dims[2] != "+X":
        raise SystemExit(f"HDR: 不支持的扫描序 {dims}")
    height, width = int(dims[1]), int(dims[3])
    pos = nl + 1

    pixels = [(0.0, 0.0, 0.0)] * (width * height)
    for y in range(height):
        if pos + 4 > len(data):
            raise SystemExit("HDR: 数据截断")
        # 新式 RLE 行头:0x02 0x02 hi lo
        if data[pos] == 2 and data[pos + 1] == 2 and ((data[pos + 2] << 8) | data[pos + 3]) == width:
            pos += 4
            chans = [bytearray(width) for _ in range(4)]
            for c in range(4):
                x = 0
                while x < width:
                    cnt = data[pos]
                    pos += 1
                    if cnt > 128:  # run
                        val = data[pos]
                        pos += 1
                        for _ in range(cnt - 128):
                            chans[c][x] = val
                            x += 1
                    else:  # literal
                        for _ in range(cnt):
                            chans[c][x] = data[pos]
                            pos += 1
                            x += 1
            for x in range(width):
                e = chans[3][x]
                if e == 0:
                    pixels[y * width + x] = (0.0, 0.0, 0.0)
                else:
                    s = math.ldexp(1.0, e - 136)  # 2^(e-128) / 256
                    pixels[y * width + x] = (
                        chans[0][x] * s,
                        chans[1][x] * s,
                        chans[2][x] * s,
                    )
        else:  # 非 RLE 直存
            for x in range(width):
                r, g, b, e = data[pos], data[pos + 1], data[pos + 2], data[pos + 3]
                pos += 4
                if e == 0:
                    pixels[y * width + x] = (0.0, 0.0, 0.0)
                else:
                    s = math.ldexp(1.0, e - 136)
                    pixels[y * width + x] = (r * s, g * s, b * s)
    return width, height, pixels


def bake_lut(width: int, height: int, px):
    """equirect → (dot(dir,sun), dir.y) 二维 LUT(均值归并)+ 太阳方向估计。

    太阳方向 = 亮度最大方向(HDRI 日面)。归并为均值:同一 (u, v) 格可能来自
    多个方位,如实取平均(方位结构丢失,见模块文档「诚实边界」)。
    """
    # 1) 找太阳(亮度峰)。
    best, sun = -1.0, (0.0, 1.0, 0.0)
    for j in range(height):
        theta = (j + 0.5) / height * math.pi  # 0 = 天顶
        sy = math.cos(theta)
        st = math.sin(theta)
        for i in range(0, width, 2):  # 步长 2 提速,日面远大于 2 像素
            r, g, b = px[j * width + i]
            lum = 0.2126 * r + 0.7152 * g + 0.0722 * b
            if lum > best:
                best = lum
                phi = (i + 0.5) / width * 2.0 * math.pi - math.pi
                sun = (st * math.sin(phi), sy, st * math.cos(phi))

    # 2) 归并。
    acc = [0.0] * (LUT_W * LUT_H * 3)
    cnt = [0] * (LUT_W * LUT_H)
    for j in range(height):
        theta = (j + 0.5) / height * math.pi
        dy = math.cos(theta)
        st = math.sin(theta)
        v = (dy + 1.0) * 0.5
        jj = min(LUT_H - 1, max(0, int(v * LUT_H)))
        for i in range(width):
            phi = (i + 0.5) / width * 2.0 * math.pi - math.pi
            dx, dz = st * math.sin(phi), st * math.cos(phi)
            cosa = dx * sun[0] + dy * sun[1] + dz * sun[2]
            u = (cosa + 1.0) * 0.5
            ii = min(LUT_W - 1, max(0, int(u * LUT_W)))
            k = jj * LUT_W + ii
            r, g, b = px[j * width + i]
            acc[k * 3] += r
            acc[k * 3 + 1] += g
            acc[k * 3 + 2] += b
            cnt[k] += 1
    out = [0.0] * (LUT_W * LUT_H * 3)
    for k in range(LUT_W * LUT_H):
        c = cnt[k]
        if c:
            out[k * 3] = acc[k * 3] / c
            out[k * 3 + 1] = acc[k * 3 + 1] / c
            out[k * 3 + 2] = acc[k * 3 + 2] / c
    # 空格用同 v 行的邻格补(极少数,发生在 |cos| 极值带)。
    for jj in range(LUT_H):
        for ii in range(LUT_W):
            k = jj * LUT_W + ii
            if cnt[k]:
                continue
            for d in range(1, LUT_W):
                for nb in (ii - d, ii + d):
                    if 0 <= nb < LUT_W and cnt[jj * LUT_W + nb]:
                        s = jj * LUT_W + nb
                        out[k * 3] = out[s * 3]
                        out[k * 3 + 1] = out[s * 3 + 1]
                        out[k * 3 + 2] = out[s * 3 + 2]
                        cnt[k] = 1
                        break
                if cnt[k]:
                    break
    return out, sun, best


def main() -> int:
    ap = argparse.ArgumentParser(description="Poly Haven CC0 水景 HDRI → G41 sky-view LUT")
    ap.add_argument("--slug", default="lakeside_sunrise")
    ap.add_argument("--res", default="1k", choices=["1k", "2k", "4k"])
    ap.add_argument("--cache", default=DEFAULT_CACHE)
    ap.add_argument("--out", default=None, help="LUT 输出路径(默认落缓存根)")
    ap.add_argument("--list", action="store_true", help="只列水景候选")
    a = ap.parse_args()

    if a.list:
        info = http_json(f"{API}/assets?t=hdris")
        print(f"{'slug':<26} {'license':<8} {'authors'}")
        for s in WATER_SLUGS:
            m = info.get(s)
            if not m:
                continue
            lic = "CC0" if m.get("donated") or True else "?"
            print(f"{s:<26} {lic:<8} {', '.join(m.get('authors', {}).keys())}")
        return 0

    meta = http_json(f"{API}/info/{a.slug}")
    files = http_json(f"{API}/files/{a.slug}")
    node = files.get("hdri", {}).get(a.res, {}).get("hdr")
    if not node:
        raise SystemExit(f"{a.slug}: 无 {a.res} hdr 直链")

    root = os.path.join(a.cache, "polyhaven-env", a.slug)
    hdr_path = os.path.join(root, f"{a.slug}_{a.res}.hdr")
    if os.path.exists(hdr_path):
        h = hashlib.sha256(open(hdr_path, "rb").read()).hexdigest()
        print(f"[cache] {hdr_path} sha256={h[:16]}…")
    else:
        print(f"[get ] {node['url']}")
        h = http_get(node["url"], hdr_path)
        print(f"[ok  ] {hdr_path} {node['size']}B sha256={h[:16]}…")

    w, ht, px = read_radiance_hdr(hdr_path)
    print(f"[hdr ] {w}x{ht}")
    lut, sun, peak = bake_lut(w, ht, px)
    out = a.out or os.path.join(root, f"{a.slug}_{a.res}.skylut")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "wb") as f:
        f.write(struct.pack("<II", LUT_W, LUT_H))
        f.write(struct.pack(f"<{len(lut)}f", *lut))
    lut_sha = hashlib.sha256(open(out, "rb").read()).hexdigest()

    side = {
        "schema": "rurix.g41.env_lut.v1",
        "slug": a.slug,
        "name": meta.get("name"),
        "license": "CC0-1.0",
        "license_note": "Poly Haven 全站 CC0-1.0;无署名义务,可商用。",
        "source_url": f"https://polyhaven.com/a/{a.slug}",
        "authors": list(meta.get("authors", {}).keys()),
        "resolution": a.res,
        "hdr_sha256": h,
        "lut_sha256": lut_sha,
        "lut_w": LUT_W,
        "lut_h": LUT_H,
        "sun_dir_estimate": [round(v, 6) for v in sun],
        "sun_peak_luminance": round(peak, 4),
        "honest_bounds": (
            "LUT 参数化 (dot(dir,sun), dir.y) 假设绕天顶-太阳轴旋转对称;"
            "真实 HDRI 的方位结构在归并取均值时丢失,仅保留亮度/色度分布。"
            "适用于水面反射与环境光,不可当全景背景。"
        ),
        "binary_policy": "HDR 与 LUT 均留缓存根不入 git(仓库 .gitignore 拒 *.hdr)。",
    }
    with open(out + ".json", "w", encoding="utf-8") as f:
        json.dump(side, f, ensure_ascii=False, indent=2)
    print(f"[lut ] {out} sha256={lut_sha[:16]}… sun={tuple(round(v,3) for v in sun)}")
    print(f"[side] {out}.json")
    return 0


if __name__ == "__main__":
    sys.exit(main())
