#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# SPIKE(RD-027):E0a 基线重立——工具链 13.2→13.3 漂移后复测毒径矩阵。
"""E0a:当前工具链下毒径基线判定(G-G3-1 ①)。

变体矩阵(REND_FRAMES 8→1 缩短;bounces/spp 为宿主编译期常数但 kernel 侧全经
launch 标量实参下发——PTX/cubin 跨变体应逐字节同一,本脚本以 ptx_sha256 见证):

  ctrl_b2   SPP=8/batch 8,  bounces=2, 1 帧   → 预期 绿(秒级)
  ctrl_32   SPP=32/batch 32, bounces=2, 1 帧  → 预期 绿(生产切片档)
  poison_b3 SPP=8/batch 8,  bounces=3, 1 帧   → MS1.4 实录:挂起 >300s
  poison_b4 SPP=8/batch 8,  bounces=4, 1 帧   → MS1.4 实录:挂起 >15min
  poison_256 SPP=256/batch 32, bounces=2, 1 帧 → MS1.4 实录:挂起 >590s 零帧

纪律:benign-first/poison-last;每次毒径判定后金丝雀门(ctrl_b2 复绿 + nvidia-smi
响应),失败即中止;全部经 proc_guard 硬超时(120s 判定线,诚实红 124);
增量 JSONL 落 build/spike-rd027/campaign.jsonl。

用法:py -3 spike/rd027-pt-poison/run_e0a.py [--skip-poison] [--build-only|--run-only]
(--build-only:仅构建变体,纯 CPU 不触 GPU,可与 runner CI 并存;
 --run-only:复用已构建 bin/ 直接跑 GPU 阶段,须 runner 空闲)
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from spike_common import (  # noqa: E402
    WORK, append_jsonl, build_variant, campaign_header, canary, fail, log, run_variant,
)

# (name, SPP, SPP_BATCH, PT_BOUNCES, expect)
MATRIX = [
    ("ctrl_b2", 8, 8, 2, "green"),
    ("ctrl_32", 32, 32, 2, "green"),
    ("poison_b3", 8, 8, 3, "hang_per_ms14"),
    ("poison_b4", 8, 8, 4, "hang_per_ms14"),
    ("poison_256", 256, 32, 2, "hang_per_ms14"),
]


def patches_for(spp: int, batch: int, bounces: int) -> list[tuple[str, str, str]]:
    return [
        ("params.rx", "pub const SPP: usize = 32;", f"pub const SPP: usize = {spp};"),
        ("params.rx", "pub const SPP_BATCH: usize = 32;",
         f"pub const SPP_BATCH: usize = {batch};"),
        ("params.rx", "pub const PT_BOUNCES: u32 = 2;",
         f"pub const PT_BOUNCES: u32 = {bounces};"),
        ("params.rx", "pub const REND_FRAMES: usize = 8;",
         "pub const REND_FRAMES: usize = 1;"),
    ]


def main() -> int:
    argv = sys.argv[1:]
    skip_poison = "--skip-poison" in argv
    build_only = "--build-only" in argv
    run_only = "--run-only" in argv
    manifest = WORK / "e0a_built.json"

    if not run_only:
        campaign_header("E0a", "基线重立:工具链 13.2→13.3 漂移复测(REND_FRAMES=1)")
        # 1) 构建全部变体(纯 CPU+ptxas AOT,不触 GPU);记录 ptx digest 验单 artifact 事实
        built: dict[str, dict] = {}
        for name, spp, batch, bounces, expect in MATRIX:
            b = build_variant(name, patches_for(spp, batch, bounces))
            built[name] = b
            append_jsonl({"kind": "build", "name": name,
                          "config": {"spp": spp, "batch": batch, "bounces": bounces,
                                     "frames": 1},
                          "ptx_sha256": b["ptx_sha256"], "ll_sha256": b["ll_sha256"],
                          "build_wall_s": b["build_wall_s"]})
            log(f"built {name}: ptx={str(b['ptx_sha256'])[:16]}…")
        digests = {b["ptx_sha256"] for b in built.values()}
        append_jsonl({"kind": "single_artifact_check",
                      "distinct_ptx_digests": len(digests),
                      "confirmed_single_artifact": len(digests) == 1})
        log(f"single-artifact 核验:distinct PTX digests = {len(digests)} "
            f"({'同一 artifact ✓' if len(digests) == 1 else '多 artifact —— 假设被推翻!'})")
        with open(manifest, "wb") as f:
            f.write(json.dumps({k: str(v["exe"]) for k, v in built.items()},
                               ensure_ascii=False).encode("utf-8"))
        if build_only:
            log("--build-only:构建完成,GPU 阶段待 runner 空闲后 --run-only 执行")
            return 0
    else:
        if not manifest.is_file():
            fail("--run-only 但无 e0a_built.json;先跑 --build-only")
        built = {k: {"exe": Path(v)} for k, v in
                 json.loads(manifest.read_bytes().decode("utf-8")).items()}
        for k, v in built.items():
            if not v["exe"].is_file():
                fail(f"--run-only:变体 exe 缺失 {v['exe']}")

    # 2) benign-first:两个对照档
    results = {}
    for name in ("ctrl_b2", "ctrl_32"):
        rec = run_variant(name, built[name]["exe"], expect="green")
        results[name] = rec["classification"]
        if rec["classification"] != "completed":
            fail(f"对照档 {name} 未复绿({rec['classification']})——复现前提破裂,先排环境")

    # 3) poison-last + 每毒径后金丝雀
    if skip_poison:
        log("--skip-poison:跳过毒径档(仅对照基线)")
    else:
        for name in ("poison_b3", "poison_b4", "poison_256"):
            rec = run_variant(name, built[name]["exe"], expect="hang_per_ms14")
            results[name] = rec["classification"]
            if rec["classification"] == "hang_timeout":
                if not canary(built["ctrl_b2"]["exe"]):
                    append_jsonl({"kind": "abort", "reason": "canary_failed_after_" + name})
                    return 3

    append_jsonl({"kind": "e0a_summary", "results": results,
                  "reproduced": any(v == "hang_timeout" for v in results.values())})
    log(f"E0a 结果:{results}")
    if any(v == "hang_timeout" for v in results.values()):
        log("→ 毒径在 13.3 工具链下复现,进入判别矩阵(E7a/E0b/E1/E2)")
    elif not skip_poison:
        log("→ 毒径未复现:走 toolchain-fixed 归因路(锁定差异组件 + 升档回填链)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
