#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""F 相微调诊断：吧台小吊灯仍全白的根因——材质归因 + UV + emissive 贴图逐 mip 采样。

复用 recon/raycast_probe.py 的装配/相机口径，追加 TEXCOORD_0 插值与 kernel 同式
lod 估计（lod = log2(th · k_pix · k_tri · w_base)，k_pix = 2·tan(fovy/2)/H）。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
ASSET_DIR = Path("K:/rurix-ext/g11-assets/bistro-interior-ue")
CONTRACT = ROOT / "milestones/g13/g13_ue_upscale_parity_contract.json"
W, H = 1920, 1080

EMISSIVE_MATS = {
    38: "MASTER_Interior_01_Paris_Lantern_Emissive.png",
    39: "Paris_Ceiling_Lamp_Emissive.png",
    40: "Paris_CeilingFan_Emissive.png",
    59: "Paris_Wall_Light_Interior_Emissive.png",
}
# 契约 Le（scale ≡ 1.0，故 Le_px = em_tex_linear）
LE = {38: 0.09250, 39: 0.02230, 40: 0.02230, 59: 0.22172}

CT = {5120: ("b", 1), 5121: ("B", 1), 5122: ("h", 2), 5123: ("H", 2), 5125: ("I", 4), 5126: ("f", 4)}
NC = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def read_accessor(gltf, buf, ai):
    a = gltf["accessors"][ai]
    bv = gltf["bufferViews"][a["bufferView"]]
    off = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
    fmt, isz = CT[a["componentType"]]
    n = NC[a["type"]]
    cnt = a["count"]
    stride = bv.get("byteStride") or isz * n
    if stride == isz * n:
        return np.frombuffer(buf, dtype=np.dtype(fmt), count=cnt * n, offset=off).reshape(cnt, n)
    arr = np.zeros((cnt, n), dtype=np.dtype(fmt))
    for i in range(cnt):
        arr[i] = np.frombuffer(buf, dtype=np.dtype(fmt), count=n, offset=off + i * stride)
    return arr


def node_local(n):
    if "matrix" in n:
        return np.array(n["matrix"], dtype=np.float64).reshape(4, 4).T
    m = np.eye(4)
    t = n.get("translation", [0, 0, 0])
    r = n.get("rotation", [0, 0, 0, 1])
    s = n.get("scale", [1, 1, 1])
    x, y, z, w = r
    rot = np.array([
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ])
    m[:3, :3] = rot @ np.diag(s)
    m[:3, 3] = t
    return m


def srgb_to_linear(c):
    c = c / 255.0
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def main() -> int:
    gltf = json.loads(GLTF.read_text(encoding="utf-8"))
    buf = (GLTF.parent / gltf["buffers"][0]["uri"]).read_bytes()
    nodes = gltf["nodes"]
    parent = {}
    for i, n in enumerate(nodes):
        for c in n.get("children", []):
            parent[c] = i
    world = {}

    def compose(i):
        if i in world:
            return world[i]
        m = node_local(nodes[i])
        if i in parent:
            m = compose(parent[i]) @ m
        world[i] = m
        return m

    v0l, e1l, e2l, matl, uvl = [], [], [], [], []
    for ni, n in enumerate(nodes):
        if "mesh" not in n:
            continue
        wm = compose(ni)
        for prim in gltf["meshes"][n["mesh"]]["primitives"]:
            pos = read_accessor(gltf, buf, prim["attributes"]["POSITION"]).astype(np.float64)
            idx = read_accessor(gltf, buf, prim["indices"]).astype(np.int64).reshape(-1, 3)
            uv = read_accessor(gltf, buf, prim["attributes"]["TEXCOORD_0"]).astype(np.float64)
            wp = pos @ wm[:3, :3].T + wm[:3, 3]
            tv = wp[idx]
            v0l.append(tv[:, 0]); e1l.append(tv[:, 1] - tv[:, 0]); e2l.append(tv[:, 2] - tv[:, 0])
            matl.append(np.full(len(tv), prim.get("material", -1), dtype=np.int32))
            uvl.append(uv[idx])  # (t,3,2)
    v0 = np.concatenate(v0l); e1 = np.concatenate(e1l); e2 = np.concatenate(e2l)
    tmat = np.concatenate(matl); tuv = np.concatenate(uvl)
    print(f"tris baked: {len(v0)}")

    contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
    srow = [s for s in contract["scenes"] if s["scene_id"] == "bistro-interior"][0]
    cam = srow["camera"]
    eye = np.array(cam["position"])
    qw, qx, qy, qz = cam["orientation_quat"]

    def qrot(v):
        u = np.array([qx, qy, qz])
        return v + 2 * np.cross(u, np.cross(u, v) + qw * v)

    f = qrot(np.array([0.0, 0.0, -1.0])); f /= np.linalg.norm(f)
    s = np.cross(f, qrot(np.array([0.0, 1.0, 0.0]))); s /= np.linalg.norm(s)
    u = np.cross(s, f)
    fovy = np.deg2rad(cam["fov_y_deg"])
    ty = np.tan(fovy / 2)
    aspect = W / H
    k_pix = 2.0 * ty / H

    # emissive 贴图 mip 金字塔（linear 域 box 半采样，与烘焙同语义）
    pyr = {}
    for mi, png in EMISSIVE_MATS.items():
        im = np.asarray(Image.open(ASSET_DIR / png).convert("RGB"), dtype=np.float64)
        lin = srgb_to_linear(im)
        levels = [lin]
        cur = lin
        while cur.shape[0] > 1:
            h2, w2 = cur.shape[0] // 2, cur.shape[1] // 2
            cur = cur[: h2 * 2, : w2 * 2].reshape(h2, 2, w2, 2, 3).mean(axis=(1, 3))
            levels.append(cur)
        pyr[mi] = levels

    def cast(px, py):
        ndx = 2.0 * ((px + 0.5) / W) - 1.0
        ndy = 1.0 - 2.0 * ((py + 0.5) / H)
        d = f + ty * (ndx * aspect * s + ndy * u)
        d /= np.linalg.norm(d)
        pv = np.cross(np.broadcast_to(d, e2.shape), e2)
        det = (e1 * pv).sum(1)
        ok = np.abs(det) > 1e-12
        inv = np.where(ok, 1.0 / np.where(ok, det, 1.0), 0.0)
        tvv = eye - v0
        uu = (tvv * pv).sum(1) * inv
        qv = np.cross(tvv, e1)
        vv = (qv * np.broadcast_to(d, e1.shape)).sum(1) * inv
        tt = (e2 * qv).sum(1) * inv
        hit = ok & (uu >= 0) & (vv >= 0) & (uu + vv <= 1) & (tt > 1e-4)
        if not hit.any():
            return None
        k = np.where(hit)[0][np.argmin(tt[hit])]
        return k, float(tt[k]), float(uu[k]), float(vv[k])

    probes = [
        ("bell_1_c", (557, 255)), ("bell_1_up", (557, 240)), ("bell_1_dn", (557, 272)), ("bell_1_L", (540, 255)),
        ("bell_2_c", (860, 269)), ("bell_2_up", (860, 252)), ("bell_2_dn", (860, 285)),
        ("bell_3_c", (1040, 306)), ("bell_3_up", (1040, 292)),
        ("bell_4_c", (1168, 332)), ("bell_4_up", (1168, 320)),
        ("disc_bulb", (1562, 187)), ("disc_shade", (1385, 15)),
        ("edge_L", (80, 4)), ("edge_R", (1649, 5)),
    ]
    print(f"{'zone':14s} {'px':>5s},{'py':>4s} {'mat':>4s} {'name':34s} {'t':>7s} {'lod':>5s}  em_tex_linear(L) mip0/lodN → ×16×3.3 后")
    for zone, (px, py) in probes:
        r = cast(px, py)
        if r is None:
            print(f"{zone:14s} {px:5d},{py:4d} MISS")
            continue
        k, t, bu, bv = r
        mi = int(tmat[k])
        name = gltf["materials"][mi].get("name", "?")
        if mi not in EMISSIVE_MATS:
            print(f"{zone:14s} {px:5d},{py:4d} {mi:4d} {name:34s} {t:7.3f}  （非自发光材质）")
            continue
        uv = tuv[k][0] * (1 - bu - bv) + tuv[k][1] * bu + tuv[k][2] * bv
        uvw = uv - np.floor(uv)
        # k_tri = sqrt(uv_area / world_area)
        uv_e1 = tuv[k][1] - tuv[k][0]; uv_e2 = tuv[k][2] - tuv[k][0]
        uv_area = abs(uv_e1[0] * uv_e2[1] - uv_e1[1] * uv_e2[0]) * 0.5
        w_area = 0.5 * np.linalg.norm(np.cross(e1[k], e2[k]))
        k_tri = np.sqrt(uv_area / max(w_area, 1e-12))
        lod = np.log2(max(t * k_pix * k_tri * 2048.0, 1e-6))
        lod_c = int(np.clip(np.floor(lod), 0, len(pyr[mi]) - 1))
        levels = pyr[mi]
        def samp(lv):
            a = levels[lv]
            hh, ww = a.shape[0], a.shape[1]
            x = min(int(uvw[0] * ww), ww - 1); y = min(int(uvw[1] * hh), hh - 1)
            return a[y, x]
        l0 = samp(0); ln = samp(lod_c)
        lum0 = (0.2126 * l0[0] + 0.7152 * l0[1] + 0.0722 * l0[2])
        lumn = (0.2126 * ln[0] + 0.7152 * ln[1] + 0.0722 * ln[2])
        print(f"{zone:14s} {px:5d},{py:4d} {mi:4d} {name:34s} {t:7.3f} {lod:5.2f}  "
              f"uv=({uvw[0]:.3f},{uvw[1]:.3f}) L0={lum0:.4f} L{lod_c}={lumn:.4f} → disp0={lum0*16*3.3:.2f} dispN={lumn*16*3.3:.2f}")
    # 全图岛统计：emissive 贴图的亮度分布（判断"整岛亮"还是"亮斑集中"）
    print("\n== emissive 贴图 mip0 线性亮度分布 ==")
    for mi, png in EMISSIVE_MATS.items():
        a = pyr[mi][0]
        lum = 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]
        nz = lum[lum > 0.001]
        print(f"mat {mi:2d} {png:44s} 非零占比={len(nz)/lum.size*100:5.1f}%  nz_mean={nz.mean() if len(nz) else 0:.4f}  nz_p50={np.percentile(nz,50) if len(nz) else 0:.4f}  nz_p95={np.percentile(nz,95) if len(nz) else 0:.4f}  max={lum.max():.4f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
