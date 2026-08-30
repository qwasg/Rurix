#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0828 recon: bistro-interior 70 材质普查（只读侦察）。

镜像 g14_3_lane_body.rs 的内容模型：
  - dds_mean_linear_rgb (L1303): DDS BC1/BC3 mip0 逐 texel sRGB→线性均值
  - 逐三角 albedo = tex_mean × baseColorFactor × (1−metallic) (L1750-1765)
  - top-12 映射律：三角数降序、并列 material_index 升序 (tex_on_ev.json mapping_law)

输出 material_census.json：材质名/index/三角数/均值色/有效 albedo/top-12 与否/
DDS 头（w/h/mips/fourcc/字节数）+ 蓝/红饱和候选排序。
"""
from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

import numpy as np

GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
OUT = Path(__file__).resolve().parent / "material_census.json"

# 镜像 srgb_to_linear (g14_3_lane_body.rs L1292)
def srgb_to_linear(c: np.ndarray) -> np.ndarray:
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)

LIN_LUT = srgb_to_linear(np.arange(256, dtype=np.float64) / 255.0)


def unpack565(c: np.ndarray) -> np.ndarray:
    """u16 -> (N,3) u8，镜像 Rust unpack (L1337-1346)。"""
    r = ((c >> 11) & 31).astype(np.uint16)
    g = ((c >> 5) & 63).astype(np.uint16)
    b = (c & 31).astype(np.uint16)
    return np.stack([(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)], axis=-1).astype(np.uint8)


def dds_header(bytes_: bytes) -> dict:
    if len(bytes_) < 128 or bytes_[:4] != b"DDS ":
        raise ValueError("DDS magic mismatch")
    h = struct.unpack_from("<I", bytes_, 12)[0]
    w = struct.unpack_from("<I", bytes_, 16)[0]
    mips = struct.unpack_from("<I", bytes_, 28)[0]
    fourcc = bytes_[84:88].decode("ascii", "replace")
    return {"w": w, "h": h, "mips": mips, "fourcc": fourcc}


def dds_mean_linear_rgb(bytes_: bytes) -> list[float]:
    """镜像 dds_mean_linear_rgb (L1303-1387)：mip0 BC1/BC3 → 线性均值。"""
    hd = dds_header(bytes_)
    w, h, fourcc = hd["w"], hd["h"], hd["fourcc"]
    block_bytes = {"DXT1": 8, "DXT5": 16}.get(fourcc)
    if block_bytes is None:
        raise ValueError(f"fourCC {fourcc} not BC1/BC3")
    bw, bh = (w + 3) // 4, (h + 3) // 4
    n = bw * bh
    raw = np.frombuffer(bytes_, dtype=np.uint8, count=n * block_bytes, offset=128).reshape(n, block_bytes)
    cb = raw[:, block_bytes - 8:]  # 颜色块恒末 8B (L1333)
    c0 = cb[:, 0].astype(np.uint16) | (cb[:, 1].astype(np.uint16) << 8)
    c1 = cb[:, 2].astype(np.uint16) | (cb[:, 3].astype(np.uint16) << 8)
    lut = (cb[:, 4].astype(np.uint32) | (cb[:, 5].astype(np.uint32) << 8)
           | (cb[:, 6].astype(np.uint32) << 16) | (cb[:, 7].astype(np.uint32) << 24))
    p0 = unpack565(c0).astype(np.uint32)  # (n,3)
    p1 = unpack565(c1).astype(np.uint32)
    four = c0 > c1
    pal = np.zeros((n, 4, 3), dtype=np.uint8)
    pal[:, 0] = p0
    pal[:, 1] = p1
    pal[:, 2] = np.where(four[:, None], (2 * p0 + p1) // 3, (p0 + p1) // 2).astype(np.uint8)
    pal[:, 3] = np.where(four[:, None], (p0 + 2 * p1) // 3, 0).astype(np.uint8)  # 透明槽 RGB=0 (L1360)
    # LUT 2-bit ×16 texel；无部分块（w,h 为 4 的倍数时精确；bistro 全 2^k≥16）
    if w % 4 or h % 4:
        raise ValueError("non-multiple-of-4 dims not handled")
    idx = np.stack([(lut >> (2 * k)) & 3 for k in range(16)], axis=1)  # (n,16)
    counts = np.zeros((n, 4), dtype=np.uint32)
    for k in range(4):
        counts[:, k] = (idx == k).sum(axis=1)
    lin_pal = LIN_LUT[pal]  # (n,4,3)
    acc = (lin_pal * counts[:, :, None]).sum(axis=(0, 1))
    npx = w * h
    return [float(acc[0] / npx), float(acc[1] / npx), float(acc[2] / npx)]


def main() -> int:
    gltf = json.loads(GLTF.read_text(encoding="utf-8"))
    base = GLTF.parent
    images = gltf.get("images", [])
    textures = gltf.get("textures", [])
    materials = gltf.get("materials", [])
    meshes = gltf.get("meshes", [])
    nodes = gltf.get("nodes", [])
    accessors = gltf.get("accessors", [])

    # 逐材质三角数（镜像装配循环：逐 node→mesh→primitive，indices count/3）
    tris = [0] * len(materials)
    for n in nodes:
        mi = n.get("mesh")
        if mi is None:
            continue
        for prim in meshes[mi].get("primitives", []):
            mat = prim.get("material")
            acc = accessors[prim["indices"]]
            t = acc["count"] // 3
            if mat is not None:
                tris[mat] += t

    # top-12 映射律：三角数降序、并列 index 升序（tex_on_ev.json mapping_law）
    order = sorted(range(len(materials)), key=lambda i: (-tris[i], i))
    top12 = set(order[:12])

    rows = []
    dds_cache: dict[str, dict] = {}
    for mi, m in enumerate(materials):
        pbr = m.get("pbrMetallicRoughness", {})
        f4 = pbr.get("baseColorFactor", [1.0, 1.0, 1.0, 1.0])
        factor = f4[:3]
        metallic = pbr.get("metallicFactor", 1.0)
        emissive = m.get("emissiveFactor", [0.0, 0.0, 0.0])
        uri = None
        bct = pbr.get("baseColorTexture")
        if bct is not None:
            src = textures[bct["index"]].get("source")
            if src is not None:
                uri = images[src].get("uri")
        mean = None
        hdr = None
        if uri:
            if uri not in dds_cache:
                raw = (base / uri).read_bytes()
                hd = dds_header(raw)
                hd["bytes"] = len(raw)
                try:
                    hd["mean_linear_rgb"] = dds_mean_linear_rgb(raw)
                except ValueError as e:
                    hd["mean_linear_rgb"] = None
                    hd["mean_err"] = str(e)
                dds_cache[uri] = hd
            hdr = dds_cache[uri]
            mean = hdr["mean_linear_rgb"]
        k = 1.0 - metallic
        b = ([mean[c] * factor[c] for c in range(3)] if mean else list(factor))
        eff = [b[c] * k for c in range(3)]
        rows.append({
            "material_index": mi,
            "name": m.get("name", ""),
            "tris": tris[mi],
            "top12": mi in top12,
            "base_color_factor": factor,
            "metallic": metallic,
            "gltf_emissive_factor": emissive,
            "texture_uri": uri,
            "dds": ({k2: hdr[k2] for k2 in ("w", "h", "mips", "fourcc", "bytes")} if hdr else None),
            "tex_mean_linear_rgb": mean,
            "effective_tri_albedo": eff,  # = 渲染器逐三角常量色 (L1750-1765)
        })

    # 蓝/红饱和候选（有效 albedo 域）
    def blue_score(r):
        e = r["effective_tri_albedo"]
        return e[2] - max(e[0], e[1])

    def red_score(r):
        e = r["effective_tri_albedo"]
        return e[0] - max(e[1], e[2])

    blues = sorted(rows, key=blue_score, reverse=True)[:8]
    reds = sorted(rows, key=red_score, reverse=True)[:8]

    out = {
        "schema": "rurix.day0828.recon.material_census.v1",
        "gltf": str(GLTF),
        "materials_total": len(materials),
        "tris_total": int(sum(tris)),
        "top12_mapping_law": "tris desc, ties material_index asc (tex_on_ev.json)",
        "top12_indices": sorted(top12),
        "top12_tris": int(sum(tris[i] for i in top12)),
        "materials": rows,
        "blue_candidates": [
            {"material_index": r["material_index"], "name": r["name"], "tris": r["tris"],
             "top12": r["top12"], "effective": r["effective_tri_albedo"],
             "factor": r["base_color_factor"], "mean": r["tex_mean_linear_rgb"],
             "score": blue_score(r)} for r in blues],
        "red_candidates": [
            {"material_index": r["material_index"], "name": r["name"], "tris": r["tris"],
             "top12": r["top12"], "effective": r["effective_tri_albedo"],
             "factor": r["base_color_factor"], "mean": r["tex_mean_linear_rgb"],
             "score": red_score(r)} for r in reds],
    }
    OUT.write_text(json.dumps(out, indent=1, ensure_ascii=False), encoding="utf-8")

    print(f"materials={len(materials)} tris_total={sum(tris)} top12_tris={sum(tris[i] for i in top12)}")
    print("\n== BLUE candidates (effective albedo, B - max(R,G)) ==")
    for r in blues:
        e = r["effective_tri_albedo"]
        print(f"  [{r['material_index']:3d}] {r['name']:44s} tris={r['tris']:7d} top12={str(r['top12']):5s} "
              f"eff=({e[0]:.4f},{e[1]:.4f},{e[2]:.4f}) score={blue_score(r):+.4f}")
    print("\n== RED candidates (effective albedo, R - max(G,B)) ==")
    for r in reds:
        e = r["effective_tri_albedo"]
        print(f"  [{r['material_index']:3d}] {r['name']:44s} tris={r['tris']:7d} top12={str(r['top12']):5s} "
              f"eff=({e[0]:.4f},{e[1]:.4f},{e[2]:.4f}) score={red_score(r):+.4f}")
    print(f"\nwrote {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
