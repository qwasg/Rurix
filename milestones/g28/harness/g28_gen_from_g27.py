#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G28.1 治理波）
"""从 ci/g27_*_check.py 参数化派生 G28 治理三门脚本（同构转换，判定逻辑 0-byte）。"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("hzb_device_kernel", "restir_device_kernel"),
    ("m61_mesh_shader_rejudgment", "restir_spatial_reuse_arm"),
    ("cluster_p4_gap_rejudgment", "m52_rd040_workload_rejudgment"),
    ("hlod_l4_counter_rejudgment", "rd034_upstream_recheck"),
]

SEC1_OLD = '''SEC1_IDS = [
    "M61",
    "M98-l4",
    "RD-039-mesh",
]'''
SEC1_NEW = '''SEC1_IDS = [
    "M100-high",
    "M52",
    "RD-034",
]'''


def transform(text: str, ref: str) -> str:
    text = text.replace("帮我一次性完成G26-G30", "\x04I\x04")
    text = text.replace("defer-to-G28+", "\x03N\x03").replace("defer-to-G27+", "\x03C\x03")
    text = (text.replace("G27", "\x01C\x01").replace("G26", "\x01P\x01")
                .replace("g27", "\x02c\x02").replace("g26", "\x02p\x02"))
    text = (text.replace("\x01C\x01", "G28").replace("\x01P\x01", "G27")
                .replace("\x02c\x02", "g28").replace("\x02p\x02", "g27"))
    text = text.replace("\x03N\x03", "defer-to-G29+").replace("\x03C\x03", "defer-to-G28+")
    text = text.replace("\x04I\x04", "帮我一次性完成G26-G30")
    # 治理步骤号（461/462/463 → 477/478/479；夹具 next_free 464 → 480）
    text = text.replace("464", "480").replace("463", "479").replace("462", "478").replace("461", "477")
    for old, new in SLUGS:
        text = text.replace(old, new)
    text = text.replace(SEC1_OLD, SEC1_NEW)
    text = text.replace('"go" if rid in ("M61", "M98-l4", "RD-039-mesh")',
                        '"go" if rid in ("M100-high", "M52", "RD-034")')
    text = text.replace('not l.startswith("| M61 ")', 'not l.startswith("| M100-high ")')
    text = text.replace("删除 §1 M61 行 → 闭集红", "删除 §1 M100-high 行 → 闭集红")
    text = text.replace("候选决策表缺行 M61", "候选决策表缺行 M100-high")
    text = text.replace('"| M98-l4 | 名 |", "| M98-l4 |  |"', '"| M52 | 名 |", "| M52 |  |"')
    text = re.sub(r'G28_0_IMMUTABLE_REF = "[0-9a-fA-F]{40}"',
                  f'G28_0_IMMUTABLE_REF = "{ref}"', text)
    text = text.replace("G27REFPENDING", ref).replace("G28REFPENDING", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G28REFPENDING"
    pairs = [
        ("ci/g27_acceptance_map_check.py", "ci/g28_acceptance_map_check.py"),
        ("ci/g27_candidate_decisions_check.py", "ci/g28_candidate_decisions_check.py"),
        ("ci/g27_interlock_check.py", "ci/g28_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
