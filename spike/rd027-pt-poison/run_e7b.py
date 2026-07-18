#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# SPIKE(RD-027):E7b 循环封顶插桩——定位跑飞循环 + PPM 对照判「真越界 vs 扰动溶解」。
"""E7b:三分法定位(G-G3-1 ②;E5 报告候选机理 M1~M4 判别)。

封顶设计(全部 cap 远高于合法上界,语义 0 影响):
  dda    两处 `while st < max_steps` 体首注入 `if st >= 1000u32 { break; }`(合法 ≤195)
  cell   两处 `while j < je` 体首注入 `if j >= c0 as usize + 4096usize { break; }`
  bounce `while b < bounces` 体首注入 `if b >= 8u32 { break; }`(合法 ≤4)
  spp    `while sl < spp_batch` 体首注入 `if sl >= 64u32 { break; }`(合法 ≤32)

判读(毒径 poison_b3,默认 O3 工具链):
  cap_all 挂       → 自旋不在源循环层(SASS 级 M1/M4)
  cap_all 完成:
    PPM == O0 参考 → cap 从未触发,溶解=注入分支扰动 codegen(heisen;M1/M4 加权)
    PPM != O0 参考 → cap 真触发 = 循环迭代越过合法界(M2 跑飞实证)→ 逐 cap 二分定位
用法:py -3 spike/rd027-pt-poison/run_e7b.py [--variant cap_all|cap_dda|cap_cell|cap_bounce|cap_spp]
"""
from __future__ import annotations

import glob
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from run_e0a import patches_for  # noqa: E402
from spike_common import (  # noqa: E402
    WORK, append_jsonl, build_variant, campaign_header, fail, log, run_variant, canary,
)

CAPS = {
    "dda": ("while st < max_steps {",
            "while st < max_steps {\n        if st >= 1000u32 { break; }"),
    "cell": ("while j < je {",
             "while j < je {\n            if j >= c0 as usize + 4096usize { break; }"),
    "bounce": ("while b < bounces {",
               "while b < bounces {\n            if b >= 8u32 { break; }"),
    "spp": ("while sl < spp_batch {",
            "while sl < spp_batch {\n        if sl >= 64u32 { break; }"),
}
VARIANTS = {
    "cap_all": ["dda", "cell", "bounce", "spp"],
    "cap_dda": ["dda"], "cap_cell": ["cell"], "cap_bounce": ["bounce"], "cap_spp": ["spp"],
}


def find_reference_ppm() -> Path:
    hits = sorted(glob.glob(str(WORK / "runs" / "aotO0_poison_b3_*" / "frame_0000.ppm")))
    if not hits:
        fail("缺 O0 参考图(先跑 run_e1.py 的 aotO0_poison_b3)")
    return Path(hits[-1])


def main() -> int:
    argv = sys.argv[1:]
    which = [argv[argv.index("--variant") + 1]] if "--variant" in argv else ["cap_all"]
    ref = find_reference_ppm()
    ref_bytes = ref.read_bytes()
    campaign_header("E7b", f"循环封顶插桩(参考图={ref})")

    for vname in which:
        caps = VARIANTS[vname]
        patches = patches_for(8, 8, 3) + [("render_pt.rx",) + CAPS[c] for c in caps]
        b = build_variant(f"e7b_{vname}", patches)
        append_jsonl({"kind": "build", "name": f"e7b_{vname}", "caps": caps,
                      "ptx_sha256": b["ptx_sha256"]})
        rec = run_variant(f"e7b_{vname}", b["exe"], expect="discriminator")
        verdict = rec["classification"]
        ppm_match = None
        if verdict == "completed":
            rundirs = sorted(glob.glob(str(WORK / "runs" / f"e7b_{vname}_*")))
            ppm = Path(rundirs[-1]) / "frame_0000.ppm"
            if ppm.is_file():
                ppm_match = ppm.read_bytes() == ref_bytes
            append_jsonl({"kind": "ppm_compare", "name": f"e7b_{vname}",
                          "match_o0_reference": ppm_match})
            log(f"e7b_{vname}: PPM == O0 参考 → {ppm_match}")
        elif verdict == "hang_timeout":
            canary_exe = WORK / "bin" / "ctrl_b2.exe"
            if canary_exe.is_file():
                canary(canary_exe)
        append_jsonl({"kind": "e7b_verdict", "name": f"e7b_{vname}",
                      "classification": verdict, "ppm_match_o0": ppm_match})
    return 0


if __name__ == "__main__":
    sys.exit(main())
