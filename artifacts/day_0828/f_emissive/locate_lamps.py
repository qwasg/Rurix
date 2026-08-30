#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase F：四灯具材质（38 灯笼/39 吊灯/40 吊扇/59 壁灯）顶点前向投影到契约
相机 1080p 屏幕 + 网格聚类 → 逐灯具 ROI 候选窗（b_textures/locate_mats.py
投影口径镜像 + 24px 网格 union-find 聚类）。输出 lamp_screen_rois.json。"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
CONTRACT = ROOT / "milestones/g13/g13_ue_upscale_parity_contract.json"
W, H = 1920, 1080
CELL = 24

CT = {5120: ("b", 1), 5121: ("B", 1), 5122: ("h", 2), 5123: ("H", 2), 5125: ("I", 4), 5126: ("f", 4)}
NC = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
TARGETS = {
    38: "MASTER_Interior_01_Paris_Lantern",
    39: "Paris_Ceiling_Lamp",
    40: "Paris_CeilingFan",
    59: "Paris_Wall_Light_Interior",
}


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


def clusters(px: np.ndarray, py: np.ndarray) -> list[dict]:
    """24px 网格 union-find 8 邻接聚类;返回按顶点数降序簇表。"""
    cells: dict[tuple[int, int], int] = {}
    for x, y in zip(px, py):
        c = (int(x) // CELL, int(y) // CELL)
        cells[c] = cells.get(c, 0) + 1
    parent = {c: c for c in cells}

    def find(c):
        while parent[c] != c:
            parent[c] = parent[parent[c]]
            c = parent[c]
        return c

    for (cx, cy) in list(cells):
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                nb = (cx + dx, cy + dy)
                if nb in cells:
                    ra, rb = find((cx, cy)), find(nb)
                    if ra != rb:
                        parent[ra] = rb
    groups: dict[tuple[int, int], list[tuple[int, int]]] = {}
    for c in cells:
        groups.setdefault(find(c), []).append(c)
    out = []
    for cs in groups.values():
        n = sum(cells[c] for c in cs)
        xs = [c[0] for c in cs]
        ys = [c[1] for c in cs]
        out.append({
            "verts": n,
            "bbox_px": [min(xs) * CELL, min(ys) * CELL,
                        min((max(xs) + 1) * CELL, W), min((max(ys) + 1) * CELL, H)],
        })
    out.sort(key=lambda r: -r["verts"])
    return out[:6]


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
        wp = np.concatenate(pts[mi]) if pts[mi] else np.zeros((0, 3))
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
        row = {
            "material_index": mi,
            "verts_total": int(len(wp)),
            "verts_in_frame": int(inside.sum()),
            "clusters": clusters(px[inside], py[inside]) if inside.sum() else [],
        }
        out[name] = row
        print(name, json.dumps(row, ensure_ascii=False))
    (Path(__file__).parent / "lamp_screen_rois.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
