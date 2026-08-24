#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G25.1 治理波）
"""从 ci/g24_*_check.py 参数化派生 G25 治理三门脚本（同构转换，判定逻辑 0-byte）。"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("hair_strand_oit_rejudgment", "quality_final_state_verification"),
    ("hdr_calibration_rejudgment", "fps_parity_final_verdict"),
    ("bistro_exterior_conversion_rejudgment", "campaign_full_chain_no_regression"),
    ("safe_gpu_and_legacy_rd_disposition", "campaign_handover_ledger"),
]

SEC1_OLD = '''SEC1_IDS = [
    "M114-strand",
    "M118-hdr-cal",
    "G10-N6",
    "SAFE-GPU",
]'''
SEC1_NEW = '''SEC1_IDS = [
    "SAFE-GPU",
    "G17-MD-F1",
]'''


def transform(text: str, ref: str) -> str:
    text = text.replace("defer-to-G25+", "\x03N\x03").replace("defer-to-G24+", "\x03C\x03")
    text = (text.replace("G24", "\x01C\x01").replace("G23", "\x01P\x01")
                .replace("g24", "\x02c\x02").replace("g23", "\x02p\x02"))
    text = (text.replace("\x01C\x01", "G25").replace("\x01P\x01", "G24")
                .replace("\x02c\x02", "g25").replace("\x02p\x02", "g24"))
    text = text.replace("\x03N\x03", "defer-to-G26+").replace("\x03C\x03", "defer-to-G25+")
    text = text.replace("帮我一次性完成G19-G25".replace("G19", "G19"), "帮我一次性完成G19-G25")
    # 治理步骤号（413/414/415 → 429/430/431；夹具 next_free 416 → 432）
    text = text.replace("416", "432").replace("415", "431").replace("414", "430").replace("413", "429")
    for old, new in SLUGS:
        text = text.replace(old, new)
    # 候选闭集 §1（4 行 → 2 行）与 go 判定（G25 §1 全 go）
    text = text.replace(SEC1_OLD, SEC1_NEW)
    text = text.replace('"go" if rid in ("M114-strand", "M118-hdr-cal", "G10-N6", "SAFE-GPU")',
                        '"go" if rid in ("SAFE-GPU", "G17-MD-F1")')
    text = text.replace("/4 行", "/2 行")
    text = text.replace("候选决策表 9 行零空行", "候选决策表 7 行零空行")
    text = text.replace("冻结 9 行候选闭集全等", "冻结 7 行候选闭集全等")
    # 自检红臂：删行目标 G10-N6 不在 G25 §1 → 换 SAFE-GPU
    text = text.replace('not l.startswith("| G10-N6 ")', 'not l.startswith("| SAFE-GPU ")')
    text = text.replace("删除 §1 G10-N6 行 → 闭集红", "删除 §1 SAFE-GPU 行 → 闭集红")
    text = text.replace("候选决策表缺行 G10-N6", "候选决策表缺行 SAFE-GPU")
    # 不可变 ref（G24 flip 后实测回填）
    text = re.sub(r'G25_0_IMMUTABLE_REF = "[0-9a-fA-F]{40}"',
                  f'G25_0_IMMUTABLE_REF = "{ref}"', text)
    text = text.replace("G24REFPENDING", ref).replace("G25REFPENDING", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G25REFPENDING"
    pairs = [
        ("ci/g24_acceptance_map_check.py", "ci/g25_acceptance_map_check.py"),
        ("ci/g24_candidate_decisions_check.py", "ci/g25_candidate_decisions_check.py"),
        ("ci/g24_interlock_check.py", "ci/g25_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
