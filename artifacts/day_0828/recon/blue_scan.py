#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0828 recon: bloom_off.raw（静态契约相机窗口 dump）饱和蓝像素全扫 +
射线投射归因（复用 raycast_probe 的装配/相机逻辑）。"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]

# 复用 raycast_probe 的函数（import 同目录模块）
sys.path.insert(0, str(HERE))
from raycast_probe import GLTF, CONTRACT, read_accessor, node_local  # noqa: E402

W, H = 1920, 1080
RAW = ROOT / "artifacts/night_0828/d3_bloom/bloom_off.raw"


def main() -> int:
    rb = RAW.read_bytes()
    raw = np.frombuffer(rb, dtype=np.uint8, offset=len(rb) - W * H * 4).reshape(H, W, 4)
    b = raw[:, :, 0].astype(np.int32)
    g = raw[:, :, 1].astype(np.int32)
    r = raw[:, :, 2].astype(np.int32)
    mask = (b > 90) & (b > 2 * r) & (b > g + 40)
    ys, xs = np.nonzero(mask)
    print(f"saturated-blue pixels: {len(xs)}")
    if len(xs):
        print(f"bbox x=[{xs.min()},{xs.max()}] y=[{ys.min()},{ys.max()}]")
        # 连通块粗分（按 x 聚类分段）
        order = np.argsort(xs)
        sx, sy = xs[order], ys[order]
        splits = np.nonzero(np.diff(sx) > 30)[0]
        segs = np.split(np.arange(len(sx)), splits + 1)
        for k, seg in enumerate(segs):
            print(f"  cluster{k}: n={len(seg)} x=[{sx[seg].min()},{sx[seg].max()}] y=[{sy[seg].min()},{sy[seg].max()}]")

    # ── 装配三角汤（同 raycast_probe）──
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
            tv = wp[idx]
            v0l.append(tv[:, 0])
            e1l.append(tv[:, 1] - tv[:, 0])
            e2l.append(tv[:, 2] - tv[:, 0])
            matl.append(np.full(len(tv), prim.get("material", -1), dtype=np.int32))
    v0 = np.concatenate(v0l)
    e1 = np.concatenate(e1l)
    e2 = np.concatenate(e2l)
    tmat = np.concatenate(matl)

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
    ty = np.tan(np.deg2rad(cam["fov_y_deg"]) / 2)
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
            return None
        return int(tmat[np.where(hit)[0][np.argmin(tt[hit])]])

    # 均匀抽样 40 个蓝像素做归因
    from collections import Counter
    cnt = Counter()
    if len(xs):
        step = max(1, len(xs) // 40)
        for j in range(0, len(xs), step):
            mi = cast(int(xs[j]), int(ys[j]))
            name = mats_json[mi].get("name", "?") if mi is not None and mi >= 0 else "MISS"
            cnt[f"{mi}:{name}"] += 1
    print("\nblue-pixel material attribution (sampled):")
    for k, v in cnt.most_common():
        print(f"  {k}: {v}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
