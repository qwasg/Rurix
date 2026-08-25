#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G27.1 治理波）
"""从 ci/g26_*_check.py 参数化派生 G27 治理三门脚本（同构转换，判定逻辑 0-byte）。"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("framegen_device_kernel", "hzb_device_kernel"),
    ("framegen_device_bench_accounting", "m61_mesh_shader_rejudgment"),
    ("rd045_backfill_rejudgment", "cluster_p4_gap_rejudgment"),
    ("g17_md_f1_rejudgment_window", "hlod_l4_counter_rejudgment"),
]

SEC1_OLD = '''SEC1_IDS = [
    "G13-N7",
    "RD-045-window",
    "G17-MD-F1",
]'''
SEC1_NEW = '''SEC1_IDS = [
    "M61",
    "M98-l4",
    "RD-039-mesh",
]'''


def transform(text: str, ref: str) -> str:
    # 战役指令字面先入哨兵（防被期别替换误伤——G26-G30 字面全战役不变）。
    text = text.replace("帮我一次性完成G26-G30", "\x04I\x04")
    text = text.replace("defer-to-G27+", "\x03N\x03").replace("defer-to-G26+", "\x03C\x03")
    text = (text.replace("G26", "\x01C\x01").replace("G25", "\x01P\x01")
                .replace("g26", "\x02c\x02").replace("g25", "\x02p\x02"))
    text = (text.replace("\x01C\x01", "G27").replace("\x01P\x01", "G26")
                .replace("\x02c\x02", "g27").replace("\x02p\x02", "g26"))
    text = text.replace("\x03N\x03", "defer-to-G28+").replace("\x03C\x03", "defer-to-G27+")
    text = text.replace("\x04I\x04", "帮我一次性完成G26-G30")
    # 治理步骤号（445/446/447 → 461/462/463；夹具 next_free 448 → 464）
    text = text.replace("448", "464").replace("447", "463").replace("446", "462").replace("445", "461")
    for old, new in SLUGS:
        text = text.replace(old, new)
    # 候选闭集 §1（G27 上游三行）
    text = text.replace(SEC1_OLD, SEC1_NEW)
    text = text.replace('"go" if rid in ("G13-N7", "RD-045-window", "G17-MD-F1")',
                        '"go" if rid in ("M61", "M98-l4", "RD-039-mesh")')
    # 自检红臂：删行目标换 §1 首行 M61；空单元格注入行换 M98-l4
    text = text.replace('not l.startswith("| G13-N7 ")', 'not l.startswith("| M61 ")')
    text = text.replace("删除 §1 G13-N7 行 → 闭集红", "删除 §1 M61 行 → 闭集红")
    text = text.replace("候选决策表缺行 G13-N7", "候选决策表缺行 M61")
    text = text.replace('"| G17-MD-F1 | 名 |", "| G17-MD-F1 |  |"', '"| M98-l4 | 名 |", "| M98-l4 |  |"')
    # 不可变 ref（G26 flip 后实测回填）
    text = re.sub(r'G27_0_IMMUTABLE_REF = "[0-9a-fA-F]{40}"',
                  f'G27_0_IMMUTABLE_REF = "{ref}"', text)
    text = text.replace("G26REFPENDING", ref).replace("G27REFPENDING", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G27REFPENDING"
    pairs = [
        ("ci/g26_acceptance_map_check.py", "ci/g27_acceptance_map_check.py"),
        ("ci/g26_candidate_decisions_check.py", "ci/g27_candidate_decisions_check.py"),
        ("ci/g26_interlock_check.py", "ci/g27_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
