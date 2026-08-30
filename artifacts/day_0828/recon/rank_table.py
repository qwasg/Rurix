#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""top-N 律法排位表（13..24 名 + 关键材质排位）。"""
import json
from pathlib import Path

d = json.loads((Path(__file__).parent / "material_census.json").read_text(encoding="utf-8"))
rows = sorted(d["materials"], key=lambda r: (-r["tris"], r["material_index"]))
csum = 0
for i, r in enumerate(rows):
    csum += r["tris"]
    if i < 24 or r["material_index"] in (51, 27, 50, 40):
        e = r["effective_tri_albedo"]
        print(f"rank{i+1:3d} [{r['material_index']:3d}] {r['name']:44s} tris={r['tris']:7d} "
              f"cum={csum/d['tris_total']*100:5.1f}% eff=({e[0]:.3f},{e[1]:.3f},{e[2]:.3f})")
print(f"\ntop-24 覆盖 = {sum(r['tris'] for r in rows[:24])/d['tris_total']*100:.1f}%")
print(f"top-32 覆盖 = {sum(r['tris'] for r in rows[:32])/d['tris_total']*100:.1f}%")
