#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# SPIKE(RD-027):E4 删减阶梯 GPU 判定——d1~d7 毒径参数重建+守护运行。
"""E4:对累积删减阶梯逐级判定 hang/complete,收敛最小触发构造。

变体源树在 build/spike-rd027/e4/d<N>/src/(E4 制作 agent 产出,params 为切片档);
本脚本对每级:params.rx 打毒径补丁(SPP 8/batch 8/bounces 3/frames 1)→ rx build
→ proc_guard 120s 判定 → 挂起后金丝雀。转折点(dN 绿/dN-1 挂)= 触发构造 delta。

用法:py -3 spike/rd027-pt-poison/run_e4.py [--steps d1,d2,...](默认 d1~d7 全跑)
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from spike_common import (  # noqa: E402
    RX, WORK, append_jsonl, campaign_header, canary, fail, log, run_variant,
)
from bench.proc_guard import guarded_run  # noqa: E402

E4 = WORK / "e4"
POISON_PATCHES = [
    ("pub const SPP: usize = 32;", "pub const SPP: usize = 8;"),
    ("pub const SPP_BATCH: usize = 32;", "pub const SPP_BATCH: usize = 8;"),
    ("pub const PT_BOUNCES: u32 = 4;", "pub const PT_BOUNCES: u32 = 3;"),
    ("pub const PT_BOUNCES: u32 = 2;", "pub const PT_BOUNCES: u32 = 3;"),
    ("pub const REND_FRAMES: usize = 8;", "pub const REND_FRAMES: usize = 1;"),
]


def patch_params(src: Path) -> None:
    p = src / "params.rx"
    text = p.read_bytes().decode("utf-8")
    hit = 0
    for anchor, repl in POISON_PATCHES:
        if anchor in text:
            text = text.replace(anchor, repl)
            hit += 1
        elif repl in text:
            hit += 1
    if hit < 4:
        fail(f"params 毒径补丁锚点不足({hit}/4+): {p}")
    with open(p, "wb") as f:
        f.write(text.encode("utf-8"))


def main() -> int:
    argv = sys.argv[1:]
    steps = (argv[argv.index("--steps") + 1].split(",") if "--steps" in argv
             else [f"d{i}" for i in range(1, 8)])
    campaign_header("E4", f"删减阶梯 GPU 判定(毒径参数 8spp/b3/1帧;steps={steps})")
    ctrl = WORK / "bin" / "ctrl_b2.exe"

    results = {}
    for step in steps:
        src = E4 / step / "src"
        if not src.is_dir():
            fail(f"变体源树缺失: {src}")
        patch_params(src)
        exe = E4 / step / "poison.exe"
        r = guarded_run([RX, "build", src / "offline.rx", "-o", exe],
                        timeout=1800, label=f"rx-build:e4_{step}")
        if r.returncode != 0:
            append_jsonl({"kind": "e4_build_fail", "name": step,
                          "tail": (r.stdout + r.stderr)[-300:]})
            log(f"{step}: BUILD FAIL(毒径参数下)——记录后跳过")
            results[step] = "build_fail"
            continue
        rec = run_variant(f"e4_{step}", exe, expect="ladder")
        results[step] = rec["classification"]
        if rec["classification"] == "hang_timeout" and ctrl.is_file():
            if not canary(ctrl):
                append_jsonl({"kind": "abort", "reason": f"canary_failed_after_e4_{step}"})
                return 3
    append_jsonl({"kind": "e4_summary", "results": results})
    log(f"E4 阶梯结果:{results}")
    hangs = [s for s in steps if results.get(s) == "hang_timeout"]
    greens = [s for s in steps if results.get(s) == "completed"]
    if hangs and greens:
        log(f"转折点:最深挂起={hangs[-1]},最浅完成={greens[0]}")
    elif hangs and not greens:
        log("d7 仍挂——三层循环+多级 break 骨架已是最小面,进入手动内联细分")
    elif greens and not hangs:
        log("d1 即绿——NEE/shadow_walk 面即触发 delta,进入 d1a~d1d 细分")
    return 0


if __name__ == "__main__":
    sys.exit(main())
