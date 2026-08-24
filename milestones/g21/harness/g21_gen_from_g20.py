#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G21.1 治理波）
"""从 ci/g20_*_check.py 参数化派生 G21 治理三门脚本（同构转换，判定逻辑 0-byte）。

带 G20 派生经验修正：defer 枚举滚动、自检夹具 next_free、红臂非法枚举注入值、
SEC1 行序与 go 判定、§1 行数字面。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("hzb_occlusion_host_realization", "restir_high_reservoir_realization"),
    ("cluster_streaming_p4_disposition", "ser_capability_disposition"),
    ("mesh_shader_rejudgment", "rd040_subitem_disposition"),
    ("far_field_l4_disposition", "rd034_upstream_recheck"),
]

SEC1_OLD = '''SEC1_IDS = [
    "M61",
    "M98-l4",
    "M52",
    "SAFE-GPU",
    "M127",
    "M114-strand",
    "M118-hdr-cal",
    "M125-adopt3",
    "G10-N6",
]'''
SEC1_NEW = '''SEC1_IDS = [
    "M100-high",
    "M52",
    "SAFE-GPU",
    "M127",
    "M114-strand",
    "M118-hdr-cal",
    "M125-adopt3",
    "G10-N6",
]'''


def transform(text: str, ref: str) -> str:
    # 里程碑号滚动（经临时 token 防二次替换）
    text = (text.replace("G20", "\x01CUR\x01").replace("G19", "\x01PRV\x01")
                .replace("g20", "\x02cur\x02").replace("g19", "\x02prv\x02"))
    text = (text.replace("\x01CUR\x01", "G21").replace("\x01PRV\x01", "G20")
                .replace("\x02cur\x02", "g21").replace("\x02prv\x02", "g20"))
    # 战役指令字面回正（跨期常量；上一步把 G19-G25 → G20-G25）
    text = text.replace("帮我一次性完成G20-G25", "帮我一次性完成G19-G25")
    # defer 枚举滚动：先滚合法值 G21+→G22+，再滚红臂注入值 G20+→G21+（顺序敏感）
    text = text.replace("defer-to-G21+", "\x03NEXT\x03").replace("defer-to-G20+", "\x03CUR\x03")
    text = text.replace("\x03NEXT\x03", "defer-to-G22+").replace("\x03CUR\x03", "defer-to-G21+")
    # 治理步骤号（349/350/351 → 365/366/367；夹具 next_free 352 → 368）
    text = text.replace("352", "368").replace("351", "367").replace("350", "366").replace("349", "365")
    # P0 slug
    for old, new in SLUGS:
        text = text.replace(old, new)
    # 候选闭集 §1（9 行 → 8 行）与 go 判定
    text = text.replace(SEC1_OLD, SEC1_NEW)
    text = text.replace('"go" if rid in ("M61", "M98-l4")', '"go" if rid in ("M100-high", "M52")')
    text = text.replace("/9 行", "/8 行")
    text = text.replace("候选决策表 14 行零空行", "候选决策表 13 行零空行")
    text = text.replace("冻结 14 行候选闭集全等", "冻结 13 行候选闭集全等")
    text = text.replace("承接 16 行", "承接 8 行")
    text = text.replace("删除 §1 M52 行 → 闭集红", "删除 §1 M52 行 → 闭集红")
    # 不可变 ref（G20 flip 后实测回填）
    text = text.replace("3c138867f94af31101591b8b2103bb1622175d4c", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G21REFPENDING"
    pairs = [
        ("ci/g20_acceptance_map_check.py", "ci/g21_acceptance_map_check.py"),
        ("ci/g20_candidate_decisions_check.py", "ci/g21_candidate_decisions_check.py"),
        ("ci/g20_interlock_check.py", "ci/g21_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
