#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0902 雨夜战役:g35_particle_lane 真跑薄封装(拼五件 SPV + RURIX_REQUIRE_REAL=1 → run_render.py 登记)。

用法:
  py -3 g35_run.py --tag <tag> [--exe target\\release\\g35_particle_lane.exe] -- <g35 其余旗标...>

展开为:
  RURIX_REQUIRE_REAL=1 RURIX_VK_VALIDATION=1
  py -3 run_render.py --tag <tag> -- <exe> --spv-scene ... --spv-mv ... --spv-resample ...
                        --spv-resolve ... --spv-encode ... --headless <其余旗标>

五件 SPV 一律显式指向 .tmp/g35_gates/render/(9/3 现编、spirv-val 绿),不依赖 bin 默认的 .tmp/g14_gates/m_c/
旧 encode 件(过期 encode 会复现彩色噪点,见 g35_particle_lane.rs 头注)。RURIX_REQUIRE_REAL=1 把
dev_env_or_fail 的三态 SKIP 翻硬红,避免场景资产装载失败时「假绿退 0」;它要求 RURIX_VK_VALIDATION=1
(validation ERROR count 不可 unavailable,VUID=0 为门 fact),二者成对(day_0831_g39 纪律同律)。
账本仍是本目录 render_runs.jsonl(run_render.py 落账,cmd 字段 = 展开后的完整命令)。
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SPV_DIR = Path(".tmp") / "g35_gates" / "render"
SPV_ARGS = [
    "--spv-scene", str(SPV_DIR / "g14_3_direct_gi.spv"),
    "--spv-mv", str(SPV_DIR / "g14_mv.spv"),
    "--spv-resample", str(SPV_DIR / "g14_8_tsr_resample.spv"),
    "--spv-resolve", str(SPV_DIR / "g14_8_tsr_resolve.spv"),
    "--spv-encode", str(SPV_DIR / "g31_display_encode.spv"),
]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", required=True)
    ap.add_argument("--exe", default=str(Path("target") / "release" / "g35_particle_lane.exe"))
    ap.add_argument("rest", nargs=argparse.REMAINDER)
    args = ap.parse_args()
    rest = args.rest
    if rest and rest[0] == "--":
        rest = rest[1:]
    for p in [Path(args.exe)] + [Path(a) for a in SPV_ARGS[1::2]]:
        if not (ROOT / p).is_file():
            print(f"FAIL: 缺件 {p}")
            return 2
    cmd = [sys.executable, str(HERE / "run_render.py"), "--tag", args.tag, "--", args.exe, *SPV_ARGS, "--headless", *rest]
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    # RURIX_REQUIRE_REAL=1 要求 RURIX_VK_VALIDATION=1(ERROR count 不可 unavailable;VUID=0 为门 fact)
    env["RURIX_VK_VALIDATION"] = "1"
    r = subprocess.run(cmd, cwd=ROOT, env=env)
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
