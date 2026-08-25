#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G30.1 治理波）
"""从 ci/g29_*_check.py 参数化派生 G30 治理三门脚本（同构转换，判定逻辑 0-byte）。"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("slab_device_kernel", "tail_anchor_rejudgment_closure"),
    ("slab_side_table_arm", "commercial_final_review"),
    ("svt_ktx2_gap_rejudgment", "campaign_full_chain_no_regression"),
    ("wg_dgc_capability_recheck", "campaign_handover_ledger"),
]

SEC1_OLD = '''SEC1_IDS = [
    "RD-041-slab",
    "RD-041-svt-ktx2-wg",
]'''
SEC1_NEW = '''SEC1_IDS = [
    "M125-adopt3",
    "M127",
    "M114-strand",
    "M118-hdr-cal",
    "G10-N6",
    "SAFE-GPU",
    "G17-MD-F1",
]'''


def transform(text: str, ref: str) -> str:
    text = text.replace("帮我一次性完成G26-G30", "\x04I\x04")
    text = text.replace("defer-to-G30+", "\x03N\x03").replace("defer-to-G29+", "\x03C\x03")
    text = (text.replace("G29", "\x01C\x01").replace("G28", "\x01P\x01")
                .replace("g29", "\x02c\x02").replace("g28", "\x02p\x02"))
    text = (text.replace("\x01C\x01", "G30").replace("\x01P\x01", "G29")
                .replace("\x02c\x02", "g30").replace("\x02p\x02", "g29"))
    text = text.replace("\x03N\x03", "defer-to-G31+").replace("\x03C\x03", "defer-to-G30+")
    text = text.replace("\x04I\x04", "帮我一次性完成G26-G30")
    # 治理步骤号（493/494/495 → 509/510/511；夹具 next_free 496 → 512）
    text = text.replace("496", "512").replace("495", "511").replace("494", "510").replace("493", "509")
    for old, new in SLUGS:
        text = text.replace(old, new)
    # 候选闭集 §1（2 行 → 7 行）
    text = text.replace(SEC1_OLD, SEC1_NEW)
    text = text.replace('"go" if rid in ("RD-041-slab", "RD-041-svt-ktx2-wg")',
                        '"go" if rid in ("M125-adopt3", "M127", "M114-strand", "M118-hdr-cal", "G10-N6", "SAFE-GPU", "G17-MD-F1")')
    text = text.replace('f"§1 {len(got1)}/2 行"', 'f"§1 {len(got1)}/7 行"')
    text = text.replace("候选决策表 7 行零空行", "候选决策表 12 行零空行")
    text = text.replace("冻结 7 行候选闭集全等", "冻结 12 行候选闭集全等")
    text = text.replace("7 行全列非空", "12 行全列非空")
    text = text.replace("候选决策表 7 行闭集（§1 两行 + §3 五行）", "候选决策表 12 行闭集（§1 七行 + §3 五行）")
    text = text.replace("承接 2 行", "承接 7 行")
    text = text.replace('not l.startswith("| RD-041-slab ")', 'not l.startswith("| M125-adopt3 ")')
    text = text.replace("删除 §1 RD-041-slab 行 → 闭集红", "删除 §1 M125-adopt3 行 → 闭集红")
    text = text.replace("候选决策表缺行 RD-041-slab", "候选决策表缺行 M125-adopt3")
    text = text.replace('"| RD-041-svt-ktx2-wg | 名 |", "| RD-041-svt-ktx2-wg |  |"', '"| M127 | 名 |", "| M127 |  |"')
    text = re.sub(r'G30_0_IMMUTABLE_REF = "[0-9a-fA-F]{40}"',
                  f'G30_0_IMMUTABLE_REF = "{ref}"', text)
    text = text.replace("G29REFPENDING", ref).replace("G30REFPENDING", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G30REFPENDING"
    pairs = [
        ("ci/g29_acceptance_map_check.py", "ci/g30_acceptance_map_check.py"),
        ("ci/g29_candidate_decisions_check.py", "ci/g30_candidate_decisions_check.py"),
        ("ci/g29_interlock_check.py", "ci/g30_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
