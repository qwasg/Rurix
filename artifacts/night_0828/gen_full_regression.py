#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""生成全 18 格 Stage A 回归探针（从锚 JSON 程序产 CELLS，防手抄错）。"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
a = json.loads((ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json").read_text(encoding="utf-8"))["anchors"]
lines = ["CELLS = ["]
for k in sorted(a):
    scene, rest = k.rsplit("_t", 1)
    tier, backend = rest.split("_", 1)
    lines.append(f'    ("{scene}", "{tier}", "{backend}", "{a[k]["last_frame_digest"]}"),')
lines.append("]")
cells_src = "\n".join(lines)

# 改写 regression_probe.py 的 CELLS 块 + 输出目录
probe = ROOT / "artifacts" / "night_0828" / "regression_probe.py"
src = probe.read_text(encoding="utf-8")
import re
src = re.sub(r"CELLS = \[.*?\]", cells_src, src, flags=re.S)
src = src.replace('OUT = ROOT / "artifacts" / "night_0828" / "regression"',
                  'OUT = ROOT / "artifacts" / "night_0828" / "regression_full"')
probe18 = ROOT / "artifacts" / "night_0828" / "regression_probe_18.py"
probe18.write_text(src, encoding="utf-8")
print(f"written {probe18} with {len(a)} cells")
