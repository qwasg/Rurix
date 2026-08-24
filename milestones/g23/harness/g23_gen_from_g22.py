#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G23.1 治理波）
"""从 ci/g22_*_check.py 参数化派生 G23 治理三门脚本（同构转换，判定逻辑 0-byte）。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SLUGS = [
    ("slab_material_host_realization", "jolt_56_adoption_rejudgment"),
    ("svt_disposition", "neural_deform_rejudgment"),
    ("ktx2_basisu_disposition", "research_track_disposition"),
    ("work_graphs_fsr_reeval_disposition", "physics_p3_subitem_disposition"),
]

SEC1_OLD = '''SEC1_IDS = [
    "SAFE-GPU",
    "M127",
    "M114-strand",
    "M118-hdr-cal",
    "M125-adopt3",
    "G10-N6",
]'''
SEC1_NEW = '''SEC1_IDS = [
    "M125-adopt3",
    "M127",
    "SAFE-GPU",
    "M114-strand",
    "M118-hdr-cal",
    "G10-N6",
]'''


def transform(text: str, ref: str) -> str:
    # defer 枚举先滚：合法值 G23+→G24+、红臂注入 G22+→G23+
    text = text.replace("defer-to-G23+", "\x03N\x03").replace("defer-to-G22+", "\x03C\x03")
    text = (text.replace("G22", "\x01C\x01").replace("G21", "\x01P\x01")
                .replace("g22", "\x02c\x02").replace("g21", "\x02p\x02"))
    text = (text.replace("\x01C\x01", "G23").replace("\x01P\x01", "G22")
                .replace("\x02c\x02", "g23").replace("\x02p\x02", "g22"))
    text = text.replace("\x03N\x03", "defer-to-G24+").replace("\x03C\x03", "defer-to-G23+")
    text = text.replace("帮我一次性完成G23-G25", "帮我一次性完成G19-G25")
    # 治理步骤号（381/382/383 → 397/398/399；夹具 next_free 384 → 400）
    text = text.replace("384", "400").replace("383", "399").replace("382", "398").replace("381", "397")
    for old, new in SLUGS:
        text = text.replace(old, new)
    # 候选闭集 §1（6 行不变，行序换 + go 判定）
    text = text.replace(SEC1_OLD, SEC1_NEW)
    text = text.replace('"go" if rid in ()', '"go" if rid in ("M125-adopt3", "M127")')
    # 不可变 ref（G22 flip 后实测回填）
    text = text.replace("0a4b1df397ed79ab30380fe0b12a822027a18d78", ref)
    return text


def main() -> int:
    ref = sys.argv[sys.argv.index("--ref") + 1] if "--ref" in sys.argv else "G23REFPENDING"
    pairs = [
        ("ci/g22_acceptance_map_check.py", "ci/g23_acceptance_map_check.py"),
        ("ci/g22_candidate_decisions_check.py", "ci/g23_candidate_decisions_check.py"),
        ("ci/g22_interlock_check.py", "ci/g23_interlock_check.py"),
    ]
    for src, dst in pairs:
        t = (ROOT / src).read_text(encoding="utf-8")
        out = transform(t, ref)
        (ROOT / dst).write_text(out, encoding="utf-8", newline="\n")
        print(f"[gen_gov] {src} → {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
