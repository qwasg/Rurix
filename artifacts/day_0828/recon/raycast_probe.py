#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0828 recon: 契约相机逐像素射线投射 → 材质归因（CPU Möller–Trumbore）。

镜像 g14_3_lane_body.rs 装配（节点树世界变换 L1673-1703 + 三角汤 L1730-1849）
与相机口径（L1963-1971: forward=q·(0,0,-1), up0=q·(0,1,0); look_at_rh 正交基）。
同时取 bloom_off.raw（静态契约相机窗口 BGRA dump）与 quality_base converged.exr
（g14_3 lane 线性 EXR）在同像素的颜色 → 双路径对照。
"""
from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
from g10_exr_lib import decode_exr  # noqa: E402

GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
CONTRACT = ROOT / "milestones/g13/g13_ue_upscale_parity_contract.json"
BLOOM_OFF_RAW = ROOT / "artifacts/night_0828/d3_bloom/bloom_off.raw"
CONV_EXR = ROOT / "artifacts/night_0828/arms/quality_base/bistro-interior/tier100/tsr_device/converged.exr"
W, H = 1920, 1080

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
        arr = np.frombuffer(buf, dtype=np.dtype(fmt), count=cnt * n, offset=off).reshape(cnt, n)
    else:
        arr = np.zeros((cnt, n), dtype=np.dtype(fmt))
        for i in range(cnt):
            arr[i] = np.frombuffer(buf, dtype=np.dtype(fmt), count=n, offset=off + i * stride)
    return arr


def node_local(n):
    if "matrix" in n:
        return np.array(n["matrix"], dtype=np.float64).reshape(4, 4).T  # glTF 列主序
    m = np.eye(4)
    t = n.get("translation", [0, 0, 0])
    r = n.get("rotation", [0, 0, 0, 1])  # xyzw
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

    v0l, e1l, e2l, matl = [], [], [], []
    for ni, n in enumerate(nodes):
        if "mesh" not in n:
            continue
        wm = compose(ni)
        for prim in gltf["meshes"][n["mesh"]]["primitives"]:
            pos = read_accessor(gltf, buf, prim["attributes"]["POSITION"]).astype(np.float64)
            idx = read_accessor(gltf, buf, prim["indices"]).astype(np.int64).reshape(-1, 3)
            wp = pos @ wm[:3, :3].T + wm[:3, 3]
            tv = wp[idx]  # (t,3,3)
            v0l.append(tv[:, 0])
            e1l.append(tv[:, 1] - tv[:, 0])
            e2l.append(tv[:, 2] - tv[:, 0])
            matl.append(np.full(len(tv), prim.get("material", -1), dtype=np.int32))
    v0 = np.concatenate(v0l)
    e1 = np.concatenate(e1l)
    e2 = np.concatenate(e2l)
    tmat = np.concatenate(matl)
    print(f"tris baked: {len(v0)}")

    # 契约相机（bistro-interior 行）
    contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
    srow = [s for s in contract["scenes"] if s["scene_id"] == "bistro-interior"][0]
    cam = srow["camera"]
    eye = np.array(cam["position"])
    qw, qx, qy, qz = cam["orientation_quat"]

    def qrot(v):
        u = np.array([qx, qy, qz])
        return v + 2 * np.cross(u, np.cross(u, v) + qw * v)

    fwd = qrot(np.array([0.0, 0.0, -1.0]))
    up0 = qrot(np.array([0.0, 1.0, 0.0]))
    f = fwd / np.linalg.norm(fwd)
    s = np.cross(f, up0)
    s /= np.linalg.norm(s)
    u = np.cross(s, f)
    fovy = np.deg2rad(cam["fov_y_deg"])
    ty = np.tan(fovy / 2)
    aspect = W / H

    mats_json = gltf["materials"]

    def cast(px, py):
        ndx = 2.0 * ((px + 0.5) / W) - 1.0
        ndy = 1.0 - 2.0 * ((py + 0.5) / H)
        d = f + ty * (ndx * aspect * s + ndy * u)
        d /= np.linalg.norm(d)
        pv = np.cross(np.broadcast_to(d, e2.shape), e2)
        det = (e1 * pv).sum(1)
        ok = np.abs(det) > 1e-12
        inv = np.where(ok, 1.0 / np.where(ok, det, 1.0), 0.0)
        tv = eye - v0
        uu = (tv * pv).sum(1) * inv
        qv = np.cross(tv, e1)
        vv = (qv * np.broadcast_to(d, e1.shape)).sum(1) * inv
        tt = (e2 * qv).sum(1) * inv
        hit = ok & (uu >= 0) & (vv >= 0) & (uu + vv <= 1) & (tt > 1e-4)
        if not hit.any():
            return None, None
        ti = np.where(hit)[0][np.argmin(tt[hit])]
        return int(tmat[ti]), float(tt[hit].min())

    # BGRA raw（窗口静态契约相机）与 EXR（g14_3 lane 线性）
    rb = BLOOM_OFF_RAW.read_bytes()
    raw = np.frombuffer(rb, dtype=np.uint8, offset=len(rb) - W * H * 4).reshape(H, W, 4)
    frame = decode_exr(CONV_EXR.read_bytes(), expected_end="rurix")
    exr = np.array(frame["pixels"], dtype=np.float64).reshape(frame["height"], frame["width"], 3)

    probes = [
        ("fan_zone", [(1500, 12), (1530, 20), (1560, 12), (1590, 25), (1620, 15), (1650, 30), (1680, 12), (1560, 40), (1610, 45)]),
        ("disc_lamp", [(1378, 9), (1400, 15)]),
        ("pendant", [(1490, 141), (1500, 150)]),
        ("right_wall", [(1650, 356), (1700, 320), (1800, 300)]),
        ("paintings_R", [(1700, 380), (1750, 350), (1820, 340), (1870, 330), (1650, 200), (1750, 190), (1900, 200)]),
    ]
    print(f"{'zone':12s} {'px':>5s},{'py':>4s}  {'mat':>4s} {'name':40s} {'t':>7s}  raw(BGRA→RGB)     exr_linear")
    for zone, pts in probes:
        for (px, py) in pts:
            mi, t = cast(px, py)
            name = mats_json[mi].get("name", "?") if mi is not None and mi >= 0 else "MISS"
            r = raw[py, px]
            rgb = (int(r[2]), int(r[1]), int(r[0]))
            ex = exr[py, px]
            print(f"{zone:12s} {px:5d},{py:4d}  {mi if mi is not None else -2:4d} {name:40s} {t if t else 0:7.3f}  "
                  f"rgb={rgb!s:16s} exr=({ex[0]:.4f},{ex[1]:.4f},{ex[2]:.4f})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
