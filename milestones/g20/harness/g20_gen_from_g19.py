#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G20.1 治理波）
"""从 ci/g19_*_check.py 参数化派生 G20 治理三门脚本（同构转换，判定逻辑 0-byte）。

转换面：里程碑号（G19→G20 / g19→g20 / 前期 G18→G19 / g18→g19）、治理步骤
（333/334/335→349/350/351）、P0 slug 五元组、候选闭集 §1 行序、不可变 ref。
战役指令字面「帮我一次性完成G19-G25」为跨期常量，转换后回正。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("frame_generation_host_realization", "hzb_occlusion_host_realization"),
    ("frame_generation_vendor_disposition", "cluster_streaming_p4_disposition"),
    ("rd045_drift_observation_window", "mesh_shader_rejudgment"),
    ("fps_parity_window_registration", "far_field_l4_disposition"),
    # closed_gate_no_regression 同名保持
]

SEC1_OLD = '''SEC1_IDS = [
    "G13-N7",
    "M52",
    "SAFE-GPU",
    "M127",
    "M98-l4",
    "M114-strand",
    "M118-hdr-cal",
    "M125-adopt3",
    "G10-N6",
]'''
SEC1_NEW = '''SEC1_IDS = [
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


def transform(text: str, ref: str) -> str:
    # 里程碑号滚动（经临时 token 防二次替换）
    text = (text.replace("G19", "\x01CUR\x01").replace("G18", "\x01PRV\x01")
                .replace("g19", "\x02cur\x02").replace("g18", "\x02prv\x02"))
    text = (text.replace("\x01CUR\x01", "G20").replace("\x01PRV\x01", "G19")
                .replace("\x02cur\x02", "g20").replace("\x02prv\x02", "g19"))
    # 战役指令字面回正（跨期常量）
    text = text.replace("帮我一次性完成G20-G25", "帮我一次性完成G19-G25")
    # 治理步骤号
    text = text.replace("335", "351").replace("334", "350").replace("333", "349")
    # P0 slug 五元组
    for old, new in SLUGS:
        text = text.replace(old, new)
    # 候选闭集 §1 行序 + go 行判定
    text = text.replace(SEC1_OLD.replace("G13-N7", "G13-N7"), SEC1_NEW) if SEC1_OLD in text else text
    text = text.replace('"go" if rid == "G13-N7"', '"go" if rid in ("M61", "M98-l4")')
    text = text.replace('go_bad = ["G13-N7"]', 'go_bad = ["M61"]')
    text = text.replace("（['G13-N7']）", "（['M61']）")
    text = text.replace("'go_rows_map_anchor'),\n", "'go_rows_map_anchor'),\n")
    # 不可变 ref（G19 flip 后实测回填）
    text = text.replace("9dda737bca0b2026f1e9672c5e70f6b807c172b9", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G20REFPENDING"
    pairs = [
        ("ci/g19_acceptance_map_check.py", "ci/g20_acceptance_map_check.py"),
        ("ci/g19_candidate_decisions_check.py", "ci/g20_candidate_decisions_check.py"),
        ("ci/g19_interlock_check.py", "ci/g20_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
