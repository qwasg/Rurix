#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# SPIKE(RD-027):E1 ptxas 优化档扫描——经转发 wrapper 注入 -O{0,1,2},AOT cubin 腿。
"""E1:定位「毒径是否依赖优化器重构」(E5 报告附加二分:O3 独有 latch/CALL 协议)。

经 build/spike-rd027/ptxas_wrap(RURIXC_PTXAS 指向它,RD027_REAL_PTXAS 指真 ptxas,
RD027_PTXAS_O 注入档位)构建 aotO{0,1,2}_{ctrl_b2,poison_b3};O3 = E0a 默认档已测挂。
信号:某档绿 → ptxas 优化器重构面定罪候选 + 护栏(-O pin);全档挂 → 构造在任意
翻译档下毒,PTX 构造/语义层嫌疑再加权(与 E0b 双路挂互证)。

用法:py -3 spike/rd027-pt-poison/run_e1.py [--build-only|--run-only]
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from run_e0a import patches_for  # noqa: E402
from spike_common import (  # noqa: E402
    WORK, append_jsonl, build_variant, campaign_header, canary, fail, log, run_variant,
)

WRAPPER = WORK / "ptxas_wrap" / "target" / "release" / "ptxas_wrap.exe"
LEVELS = ["0", "1", "2"]
CONFIGS = {"ctrl_b2": (8, 8, 2), "poison_b3": (8, 8, 3)}


def real_ptxas() -> str:
    cuda = os.environ.get("CUDA_PATH")
    p = Path(cuda) / "bin" / "ptxas.exe" if cuda else None
    if p and p.is_file():
        return str(p)
    fail("CUDA_PATH/bin/ptxas.exe 不可定位")


def main() -> int:
    argv = sys.argv[1:]
    build_only = "--build-only" in argv
    run_only = "--run-only" in argv
    manifest = WORK / "e1_built.json"

    if not run_only:
        if not WRAPPER.is_file():
            fail(f"wrapper 缺失: {WRAPPER}(先 cargo build --release)")
        campaign_header("E1", "ptxas 优化档扫描(wrapper 注入 -O0/1/2;O3=E0a 默认档)")
        built: dict[str, dict] = {}
        for lvl in LEVELS:
            env = os.environ.copy()
            env["RURIXC_PTXAS"] = str(WRAPPER)
            env["RD027_REAL_PTXAS"] = real_ptxas()
            env["RD027_PTXAS_O"] = lvl
            for cfg, (spp, batch, bounces) in CONFIGS.items():
                name = f"aotO{lvl}_{cfg}"
                b = build_variant(name, patches_for(spp, batch, bounces), env=env)
                built[name] = b
                append_jsonl({"kind": "build", "name": name, "loader_path": f"aot_O{lvl}",
                              "ptx_sha256": b["ptx_sha256"]})
                log(f"built {name}")
        with open(manifest, "wb") as f:
            f.write(json.dumps({k: str(v["exe"]) for k, v in built.items()},
                               ensure_ascii=False).encode("utf-8"))
        if build_only:
            return 0
    else:
        if not manifest.is_file():
            fail("--run-only 但无 e1_built.json")
        built = {k: {"exe": Path(v)} for k, v in
                 json.loads(manifest.read_bytes().decode("utf-8")).items()}
        for k, v in built.items():
            if not v["exe"].is_file():
                fail(f"变体 exe 缺失(隔离区?): {v['exe']};重跑 --build-only")

    results = {}
    for lvl in LEVELS:
        ctrl = f"aotO{lvl}_ctrl_b2"
        rec = run_variant(ctrl, built[ctrl]["exe"], expect="green")
        results[ctrl] = rec["classification"]
        if rec["classification"] != "completed":
            log(f"WARN {ctrl} 未绿——O{lvl} 档本身可疑,毒径结果仅供参考")
        poison = f"aotO{lvl}_poison_b3"
        rec = run_variant(poison, built[poison]["exe"], expect="discriminator")
        results[poison] = rec["classification"]
        if rec["classification"] == "hang_timeout":
            if not canary(built[f"aotO{lvl}_ctrl_b2"]["exe"]):
                append_jsonl({"kind": "abort", "reason": "canary_failed_after_" + poison})
                return 3
    append_jsonl({"kind": "e1_summary", "results": results})
    log(f"E1 结果:{results}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
