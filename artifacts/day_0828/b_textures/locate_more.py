#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0828 Phase B：备选可见材质屏幕定位（curtainA/floor/basket + curtainB1 复核）。"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
import locate_mats as lm  # noqa: E402


def main() -> int:
    gltf = json.loads(lm.GLTF.read_text(encoding="utf-8"))
    buf = (lm.GLTF.parent / gltf["buffers"][0]["uri"]).read_bytes()
    nodes = gltf["nodes"]
    parent = {}
    for i, n in enumerate(nodes):
        for c in n.get("children", []):
            parent[c] = i
    world = {}

    def compose(i):
        if i in world:
            return world[i]
        m = lm.node_local(nodes[i])
        if i in parent:
            m = compose(parent[i]) @ m
        world[i] = m
        return m

    contract = json.loads(lm.CONTRACT.read_text(encoding="utf-8"))
    cam = [s for s in contract["scenes"] if s["scene_id"] == "bistro-interior"][0]["camera"]
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
    aspect = 1920 / 1080

    for mi, name in [(13, "curtainA"), (33, "floor_tile_hex"), (63, "WickerBasket"), (14, "curtainB1")]:
        pts = []
        for ni, n in enumerate(nodes):
            if "mesh" not in n:
                continue
            wm = compose(ni)
            for prim in gltf["meshes"][n["mesh"]]["primitives"]:
                if prim.get("material", -1) != mi:
                    continue
                pos = lm.read_accessor(gltf, buf, prim["attributes"]["POSITION"]).astype(np.float64)
                pts.append(pos @ wm[:3, :3].T + wm[:3, 3])
        if not pts:
            print(mi, name, "no verts")
            continue
        wp = np.concatenate(pts)
        v = wp - eye
        zc = v @ f
        front = zc > 0.05
        if not front.any():
            print(mi, name, "all behind")
            continue
        v2 = v[front]
        zc2 = zc[front]
        px = ((v2 @ s) / (zc2 * ty * aspect) + 1) * 0.5 * 1920
        py = (1 - (v2 @ u) / (zc2 * ty)) * 0.5 * 1080
        inside = (px >= 0) & (px < 1920) & (py >= 0) & (py < 1080)
        if inside.sum() == 0:
            print(mi, name, "in-frame 0 /", len(wp))
            continue
        print(
            mi, name, "in-frame", int(inside.sum()), "/", len(wp),
            "bbox", [int(px[inside].min()), int(py[inside].min()), int(px[inside].max()), int(py[inside].max())],
            "median", [int(np.median(px[inside])), int(np.median(py[inside]))],
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
