#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0828 Phase B：三验收位屏幕定位——目标材质（curtainB1/Plaster_Red/
Paris_Paintings）顶点前向投影到契约相机屏幕（无遮挡近似,仅取裁剪窗;相机口径
= recon/raycast_probe.py 镜像）。输出逐材质屏内 bbox 与建议裁剪窗。"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
CONTRACT = ROOT / "milestones/g13/g13_ue_upscale_parity_contract.json"
W, H = 1920, 1080

CT = {5120: ("b", 1), 5121: ("B", 1), 5122: ("h", 2), 5123: ("H", 2), 5125: ("I", 4), 5126: ("f", 4)}
NC = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
TARGETS = {14: "curtainB1", 27: "MASTER_Interior_01_Plaster_Red", 51: "Paris_Paintings"}


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

    pts = {mi: [] for mi in TARGETS}
    for ni, n in enumerate(nodes):
        if "mesh" not in n:
            continue
        wm = compose(ni)
        for prim in gltf["meshes"][n["mesh"]]["primitives"]:
            mi = prim.get("material", -1)
            if mi not in TARGETS:
                continue
            pos = read_accessor(gltf, buf, prim["attributes"]["POSITION"]).astype(np.float64)
            wp = pos @ wm[:3, :3].T + wm[:3, 3]
            pts[mi].append(wp)

    contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
    srow = [s for s in contract["scenes"] if s["scene_id"] == "bistro-interior"][0]
    cam = srow["camera"]
    eye = np.array(cam["position"])
    qw, qx, qy, qz = cam["orientation_quat"]

    def qrot(v):
        u = np.array([qx, qy, qz])
        return v + 2 * np.cross(u, np.cross(u, v) + qw * v)

    f = qrot(np.array([0.0, 0.0, -1.0]))
    f /= np.linalg.norm(f)
    s = np.cross(f, qrot(np.array([0.0, 1.0, 0.0])))
    s /= np.linalg.norm(s)
    u = np.cross(s, f)
    ty = np.tan(np.deg2rad(cam["fov_y_deg"]) / 2)
    aspect = W / H

    out = {}
    for mi, name in TARGETS.items():
        if not pts[mi]:
            print(f"mat {mi} {name}: 零顶点")
            continue
        wp = np.concatenate(pts[mi])
        v = wp - eye
        zc = v @ f
        front = zc > 0.05
        v = v[front]
        zc = zc[front]
        xn = (v @ s) / (zc * ty * aspect)
        yn = (v @ u) / (zc * ty)
        px = (xn + 1.0) * 0.5 * W
        py = (1.0 - yn) * 0.5 * H
        inside = (px >= 0) & (px < W) & (py >= 0) & (py < H)
        n_in = int(inside.sum())
        row = {"verts_total": int(len(wp)), "verts_in_frame": n_in}
        if n_in > 0:
            row["bbox_px"] = [
                int(px[inside].min()), int(py[inside].min()),
                int(px[inside].max()), int(py[inside].max()),
            ]
            row["median_px"] = [int(np.median(px[inside])), int(np.median(py[inside]))]
        out[name] = row
        print(name, row)
    (Path(__file__).parent / "mat_screen_bbox.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1), encoding="utf-8"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
