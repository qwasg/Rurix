#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G24.1 治理波）
"""从 ci/g23_*_check.py 参数化派生 G24 治理三门脚本（同构转换，判定逻辑 0-byte）。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("jolt_56_adoption_rejudgment", "hair_strand_oit_rejudgment"),
    ("neural_deform_rejudgment", "hdr_calibration_rejudgment"),
    ("research_track_disposition", "bistro_exterior_conversion_rejudgment"),
    ("physics_p3_subitem_disposition", "safe_gpu_and_legacy_rd_disposition"),
]

SEC1_OLD = '''SEC1_IDS = [
    "M125-adopt3",
    "M127",
    "SAFE-GPU",
    "M114-strand",
    "M118-hdr-cal",
    "G10-N6",
]'''
SEC1_NEW = '''SEC1_IDS = [
    "M114-strand",
    "M118-hdr-cal",
    "G10-N6",
    "SAFE-GPU",
]'''


def transform(text: str, ref: str) -> str:
    text = text.replace("defer-to-G24+", "\x03N\x03").replace("defer-to-G23+", "\x03C\x03")
    text = (text.replace("G23", "\x01C\x01").replace("G22", "\x01P\x01")
                .replace("g23", "\x02c\x02").replace("g22", "\x02p\x02"))
    text = (text.replace("\x01C\x01", "G24").replace("\x01P\x01", "G23")
                .replace("\x02c\x02", "g24").replace("\x02p\x02", "g23"))
    text = text.replace("\x03N\x03", "defer-to-G25+").replace("\x03C\x03", "defer-to-G24+")
    text = text.replace("帮我一次性完成G24-G25", "帮我一次性完成G19-G25")
    # 治理步骤号（397/398/399 → 413/414/415；夹具 next_free 400 → 416）
    text = text.replace("400", "416").replace("399", "415").replace("398", "414").replace("397", "413")
    for old, new in SLUGS:
        text = text.replace(old, new)
    # 候选闭集 §1（6 行 → 4 行）与 go 判定（G24 §1 全 go）
    text = text.replace(SEC1_OLD, SEC1_NEW)
    text = text.replace('"go" if rid in ("M125-adopt3", "M127")',
                        '"go" if rid in ("M114-strand", "M118-hdr-cal", "G10-N6", "SAFE-GPU")')
    text = text.replace("/6 行", "/4 行")
    text = text.replace("候选决策表 11 行零空行", "候选决策表 9 行零空行")
    text = text.replace("冻结 11 行候选闭集全等", "冻结 9 行候选闭集全等")
    # 自检红臂：删行目标 M127 不在 G24 §1 → 换 G10-N6
    text = text.replace('not l.startswith("| M127 ")', 'not l.startswith("| G10-N6 ")')
    text = text.replace("删除 §1 M127 行 → 闭集红", "删除 §1 G10-N6 行 → 闭集红")
    text = text.replace("候选决策表缺行 M127", "候选决策表缺行 G10-N6")
    # 不可变 ref（G23 flip 后实测回填；源脚本中为 G23 期的真实 40-hex，按正则整体替换）
    import re
    text = re.sub(r'G24_0_IMMUTABLE_REF = "[0-9a-f]{40}"',
                  f'G24_0_IMMUTABLE_REF = "{ref}"', text)
    text = text.replace("G23REFPENDING", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G24REFPENDING"
    pairs = [
        ("ci/g23_acceptance_map_check.py", "ci/g24_acceptance_map_check.py"),
        ("ci/g23_candidate_decisions_check.py", "ci/g24_candidate_decisions_check.py"),
        ("ci/g23_interlock_check.py", "ci/g24_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
