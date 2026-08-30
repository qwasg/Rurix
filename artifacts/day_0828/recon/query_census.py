#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""查询 census 关键材质行。"""
import json
from pathlib import Path

d = json.loads((Path(__file__).parent / "material_census.json").read_text(encoding="utf-8"))
KEYS = (24, 25, 27, 38, 39, 40, 50, 51, 59, 13, 14, 63, 41)
for r in d["materials"]:
    if r["material_index"] in KEYS:
        e = r["effective_tri_albedo"]
        m = r["tex_mean_linear_rgb"]
        f = r["base_color_factor"]
        dd = r["dds"]
        print(f"[{r['material_index']:3d}] {r['name']:42s} tris={r['tris']:7d} top12={str(r['top12']):5s} "
              f"met={r['metallic']:.2f} mean=({m[0]:.4f},{m[1]:.4f},{m[2]:.4f}) fac=({f[0]:.3f},{f[1]:.3f},{f[2]:.3f}) "
              f"eff=({e[0]:.4f},{e[1]:.4f},{e[2]:.4f}) dds={dd['w']}x{dd['h']} mips={dd['mips']} {dd['fourcc']}")
