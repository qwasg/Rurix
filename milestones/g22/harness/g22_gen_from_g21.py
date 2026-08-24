#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G22.1 治理波）
"""从 ci/g21_*_check.py 参数化派生 G22 治理三门脚本（同构转换，判定逻辑 0-byte）。

带 G20/G21 派生经验修正：defer 枚举滚动（含红臂注入值）、自检夹具 next_free、
SEC1 行序（8→6 行）与 go 判定、行数字面。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("restir_high_reservoir_realization", "slab_material_host_realization"),
    ("ser_capability_disposition", "svt_disposition"),
    ("rd040_subitem_disposition", "ktx2_basisu_disposition"),
    ("rd034_upstream_recheck", "work_graphs_fsr_reeval_disposition"),
]

SEC1_OLD = '''SEC1_IDS = [
    "M100-high",
    "M52",
    "SAFE-GPU",
    "M127",
    "M114-strand",
    "M118-hdr-cal",
    "M125-adopt3",
    "G10-N6",
]'''
SEC1_NEW = '''SEC1_IDS = [
    "SAFE-GPU",
    "M127",
    "M114-strand",
    "M118-hdr-cal",
    "M125-adopt3",
    "G10-N6",
]'''


def transform(text: str, ref: str) -> str:
    # defer 枚举滚动先行（防被里程碑号替换污染）：合法值 G22+→G23+、红臂注入 G21+→G22+
    text = text.replace("defer-to-G22+", "\x03N\x03").replace("defer-to-G21+", "\x03C\x03")
    text = text.replace("\x03N\x03", "defer-to-G23+").replace("\x03C\x03", "defer-to-G22+")
    # 里程碑号滚动（经临时 token 防二次替换；注意此步会把上一步产物中的 G22 再滚为 G23——
    # 故 defer 枚举用不含裸 G2x 的最终字面回填）
    text = (text.replace("defer-to-G23+", "\x04NX\x04").replace("defer-to-G22+", "\x04CU\x04"))
    text = (text.replace("G21", "\x01C\x01").replace("G20", "\x01P\x01")
                .replace("g21", "\x02c\x02").replace("g20", "\x02p\x02"))
    text = (text.replace("\x01C\x01", "G22").replace("\x01P\x01", "G21")
                .replace("\x02c\x02", "g22").replace("\x02p\x02", "g21"))
    text = text.replace("\x04NX\x04", "defer-to-G23+").replace("\x04CU\x04", "defer-to-G22+")
    # 战役指令字面回正
    text = text.replace("帮我一次性完成G22-G25", "帮我一次性完成G19-G25")
    # 治理步骤号（365/366/367 → 381/382/383；夹具 next_free 368 → 384）
    text = text.replace("368", "384").replace("367", "383").replace("366", "382").replace("365", "381")
    # P0 slug
    for old, new in SLUGS:
        text = text.replace(old, new)
    # 候选闭集 §1（8 行 → 6 行）与 go 判定（G22 §1 全 defer）
    text = text.replace(SEC1_OLD, SEC1_NEW)
    text = text.replace('"go" if rid in ("M100-high", "M52")', '"go" if rid in ()')
    text = text.replace("/8 行", "/6 行")
    text = text.replace("候选决策表 13 行零空行", "候选决策表 11 行零空行")
    text = text.replace("冻结 13 行候选闭集全等", "冻结 11 行候选闭集全等")
    # 自检红臂：删行目标 M52 不在 G22 §1 → 换 M127
    text = text.replace('("删除 §1 M52 行 → 闭集红", "\\n".join(l for l in text.splitlines() if not l.startswith("| M52 ")),',
                        '("删除 §1 M127 行 → 闭集红", "\\n".join(l for l in text.splitlines() if not l.startswith("| M127 ")),')
    text = text.replace("候选决策表缺行 M52", "候选决策表缺行 M127")
    # 不可变 ref（G21 flip 后实测回填）
    text = text.replace("2b521523a660a7dd3c98106d08c4470e295a03fc", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G22REFPENDING"
    pairs = [
        ("ci/g21_acceptance_map_check.py", "ci/g22_acceptance_map_check.py"),
        ("ci/g21_candidate_decisions_check.py", "ci/g22_candidate_decisions_check.py"),
        ("ci/g21_interlock_check.py", "ci/g22_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
