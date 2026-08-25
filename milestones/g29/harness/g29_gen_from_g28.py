#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G29.1 治理波）
"""从 ci/g28_*_check.py 参数化派生 G29 治理三门脚本（同构转换，判定逻辑 0-byte）。"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("restir_device_kernel", "slab_device_kernel"),
    ("restir_spatial_reuse_arm", "slab_side_table_arm"),
    ("m52_rd040_workload_rejudgment", "svt_ktx2_gap_rejudgment"),
    ("rd034_upstream_recheck", "wg_dgc_capability_recheck"),
]

SEC1_OLD = '''SEC1_IDS = [
    "M100-high",
    "M52",
    "RD-034",
]'''
SEC1_NEW = '''SEC1_IDS = [
    "RD-041-slab",
    "RD-041-svt-ktx2-wg",
]'''


def transform(text: str, ref: str) -> str:
    text = text.replace("帮我一次性完成G26-G30", "\x04I\x04")
    text = text.replace("defer-to-G29+", "\x03N\x03").replace("defer-to-G28+", "\x03C\x03")
    text = (text.replace("G28", "\x01C\x01").replace("G27", "\x01P\x01")
                .replace("g28", "\x02c\x02").replace("g27", "\x02p\x02"))
    text = (text.replace("\x01C\x01", "G29").replace("\x01P\x01", "G28")
                .replace("\x02c\x02", "g29").replace("\x02p\x02", "g28"))
    text = text.replace("\x03N\x03", "defer-to-G30+").replace("\x03C\x03", "defer-to-G29+")
    text = text.replace("\x04I\x04", "帮我一次性完成G26-G30")
    # 治理步骤号（477/478/479 → 493/494/495；夹具 next_free 480 → 496）
    text = text.replace("480", "496").replace("479", "495").replace("478", "494").replace("477", "493")
    for old, new in SLUGS:
        text = text.replace(old, new)
    # 候选闭集 §1（3 行 → 2 行）
    text = text.replace(SEC1_OLD, SEC1_NEW)
    text = text.replace('"go" if rid in ("M100-high", "M52", "RD-034")',
                        '"go" if rid in ("RD-041-slab", "RD-041-svt-ktx2-wg")')
    text = text.replace('f"§1 {len(got1)}/3 行"', 'f"§1 {len(got1)}/2 行"')
    text = text.replace("候选决策表 8 行零空行", "候选决策表 7 行零空行")
    text = text.replace("冻结 8 行候选闭集全等", "冻结 7 行候选闭集全等")
    text = text.replace("8 行全列非空", "7 行全列非空")
    text = text.replace("候选决策表 8 行闭集（§1 三行 + §3 五行）", "候选决策表 7 行闭集（§1 两行 + §3 五行）")
    text = text.replace("承接 3 行", "承接 2 行")
    text = text.replace('not l.startswith("| M100-high ")', 'not l.startswith("| RD-041-slab ")')
    text = text.replace("删除 §1 M100-high 行 → 闭集红", "删除 §1 RD-041-slab 行 → 闭集红")
    text = text.replace("候选决策表缺行 M100-high", "候选决策表缺行 RD-041-slab")
    text = text.replace('"| M52 | 名 |", "| M52 |  |"', '"| RD-041-svt-ktx2-wg | 名 |", "| RD-041-svt-ktx2-wg |  |"')
    text = re.sub(r'G29_0_IMMUTABLE_REF = "[0-9a-fA-F]{40}"',
                  f'G29_0_IMMUTABLE_REF = "{ref}"', text)
    text = text.replace("G28REFPENDING", ref).replace("G29REFPENDING", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G29REFPENDING"
    pairs = [
        ("ci/g28_acceptance_map_check.py", "ci/g29_acceptance_map_check.py"),
        ("ci/g28_candidate_decisions_check.py", "ci/g29_candidate_decisions_check.py"),
        ("ci/g28_interlock_check.py", "ci/g29_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
