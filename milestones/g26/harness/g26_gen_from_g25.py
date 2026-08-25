#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G26.1 治理波）
"""从 ci/g25_*_check.py 参数化派生 G26 治理三门脚本（同构转换，判定逻辑 0-byte）。"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("quality_final_state_verification", "framegen_device_kernel"),
    ("fps_parity_final_verdict", "framegen_device_bench_accounting"),
    ("campaign_full_chain_no_regression", "rd045_backfill_rejudgment"),
    ("campaign_handover_ledger", "g17_md_f1_rejudgment_window"),
]

SEC1_OLD = '''SEC1_IDS = [
    "SAFE-GPU",
    "G17-MD-F1",
]'''
SEC1_NEW = '''SEC1_IDS = [
    "G13-N7",
    "RD-045-window",
    "G17-MD-F1",
]'''


def transform(text: str, ref: str) -> str:
    # 用户战役指令字面先入哨兵（防被期别替换误伤）。
    text = text.replace("帮我一次性完成G19-G25", "\x04I\x04")
    text = text.replace("defer-to-G26+", "\x03N\x03").replace("defer-to-G25+", "\x03C\x03")
    text = (text.replace("G25", "\x01C\x01").replace("G24", "\x01P\x01")
                .replace("g25", "\x02c\x02").replace("g24", "\x02p\x02"))
    text = (text.replace("\x01C\x01", "G26").replace("\x01P\x01", "G25")
                .replace("\x02c\x02", "g26").replace("\x02p\x02", "g25"))
    text = text.replace("\x03N\x03", "defer-to-G27+").replace("\x03C\x03", "defer-to-G26+")
    text = text.replace("\x04I\x04", "帮我一次性完成G26-G30")
    # 治理步骤号（429/430/431 → 445/446/447；夹具 next_free 432 → 448）
    text = text.replace("432", "448").replace("431", "447").replace("430", "446").replace("429", "445")
    for old, new in SLUGS:
        text = text.replace(old, new)
    # 候选闭集 §1（2 行 → 3 行）与 go 判定（G26 §1 全 go）
    text = text.replace(SEC1_OLD, SEC1_NEW)
    text = text.replace('"go" if rid in ("SAFE-GPU", "G17-MD-F1")',
                        '"go" if rid in ("G13-N7", "RD-045-window", "G17-MD-F1")')
    text = text.replace("/2 行\" + (\"\" if ok1", "/3 行\" + (\"\" if ok1")
    text = text.replace("候选决策表 7 行零空行", "候选决策表 8 行零空行")
    text = text.replace("冻结 7 行候选闭集全等", "冻结 8 行候选闭集全等")
    text = text.replace("14 行全列非空", "8 行全列非空")
    text = text.replace("候选决策表 14 行闭集（§1 九行 + §3 五行）", "候选决策表 8 行闭集（§1 三行 + §3 五行）")
    text = text.replace("承接 9 行", "承接 3 行")
    text = text.replace("G26_P2_DECISIONS.md §1 defer-to-G27+ 九行",
                        "g25_campaign_handover_registry.json 承接三行（G13-N7/RD-045-window/G17-MD-F1）")
    # defer 期别窗正则放宽（G26 defer 合法值 = defer-to-G27+）
    text = text.replace('re.compile(r"G2[0-5]")', 're.compile(r"G2[0-9]")')
    # 自检红臂：删行目标换 §1 首行 G13-N7
    text = text.replace('not l.startswith("| SAFE-GPU ")', 'not l.startswith("| G13-N7 ")')
    text = text.replace("删除 §1 SAFE-GPU 行 → 闭集红", "删除 §1 G13-N7 行 → 闭集红")
    text = text.replace("候选决策表缺行 SAFE-GPU", "候选决策表缺行 G13-N7")
    # 不可变 ref（G25 flip 后实测回填）
    text = re.sub(r'G26_0_IMMUTABLE_REF = "[0-9a-fA-F]{40}"',
                  f'G26_0_IMMUTABLE_REF = "{ref}"', text)
    text = text.replace("G25REFPENDING", ref).replace("G26REFPENDING", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G26REFPENDING"
    pairs = [
        ("ci/g25_acceptance_map_check.py", "ci/g26_acceptance_map_check.py"),
        ("ci/g25_candidate_decisions_check.py", "ci/g26_candidate_decisions_check.py"),
        ("ci/g25_interlock_check.py", "ci/g26_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
