#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# SPIKE(RD-027):E0b 双装载路判别——同源构型 cubin AOT vs 驱动 PTX JIT。
"""E0b:一次实验切走一半嫌疑(G-G3-1 ②核心)。

同一毒径/对照源码各构建两遍:
  <cfg>          正常构建(ptxas 13.3 AOT cubin 嵌入,运行期 cuModuleLoadData 免 JIT)
  jit_<cfg>      强制 PTX-only 构建(locate_ptxas 三候选全灭 → 驱动 620.02 JIT)

信号矩阵:仅 cubin 挂→ptxas(L3);仅 JIT 挂→驱动 JIT(L4);双挂→PTX 文本上游(L1/L2);
双绿→复现前提破裂回 E0a。

E0a 已给出 cubin 腿结果,本脚本默认只补 JIT 腿(jit_ctrl_b2 绿证 JIT 路可用 +
三毒径 JIT 腿);--with-cubin 时连 cubin 腿一起重跑。

用法:py -3 spike/rd027-pt-poison/run_e0b.py [--build-only|--run-only]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from run_e0a import MATRIX, patches_for  # noqa: E402
from spike_common import (  # noqa: E402
    WORK, append_jsonl, build_variant, campaign_header, canary, fail, log,
    no_ptxas_env, run_variant,
)

JIT_SET = [("jit_" + name, spp, batch, bounces, expect)
           for name, spp, batch, bounces, expect in MATRIX
           if name in ("ctrl_b2", "poison_b3", "poison_b4", "poison_256")]


def main() -> int:
    argv = sys.argv[1:]
    build_only = "--build-only" in argv
    run_only = "--run-only" in argv
    manifest = WORK / "e0b_built.json"

    if not run_only:
        campaign_header("E0b", "双装载路判别:JIT 腿构建(PTX-only,locate_ptxas 全灭)")
        built: dict[str, dict] = {}
        env = no_ptxas_env()
        for name, spp, batch, bounces, expect in JIT_SET:
            b = build_variant(name, patches_for(spp, batch, bounces), env=env)
            built[name] = b
            note_seen = "PTX-only" in b["build_stderr_tail"] or "embedding PTX-only" in b["build_stderr_tail"]
            append_jsonl({"kind": "build", "name": name, "loader_path": "jit",
                          "config": {"spp": spp, "batch": batch, "bounces": bounces,
                                     "frames": 1},
                          "ptx_sha256": b["ptx_sha256"],
                          "ptx_only_note_seen": note_seen,
                          "build_tail": b["build_stderr_tail"][-200:]})
            log(f"built {name}: ptx={str(b['ptx_sha256'])[:16]}… ptx_only_note={note_seen}")
        with open(manifest, "wb") as f:
            f.write(json.dumps({k: str(v["exe"]) for k, v in built.items()},
                               ensure_ascii=False).encode("utf-8"))
        if build_only:
            log("--build-only:JIT 腿构建完成")
            return 0
    else:
        if not manifest.is_file():
            fail("--run-only 但无 e0b_built.json;先跑 --build-only")
        built = {k: {"exe": Path(v)} for k, v in
                 json.loads(manifest.read_bytes().decode("utf-8")).items()}

    # GPU 阶段:jit 对照先行(证 JIT 路可用),毒径殿后 + 金丝雀
    results = {}
    rec = run_variant("jit_ctrl_b2", built["jit_ctrl_b2"]["exe"], expect="green")
    results["jit_ctrl_b2"] = rec["classification"]
    if rec["classification"] != "completed":
        fail(f"jit_ctrl_b2 未复绿({rec['classification']})——JIT 路本身不可用,矩阵无效")
    for name in ("jit_poison_b3", "jit_poison_b4", "jit_poison_256"):
        rec = run_variant(name, built[name]["exe"], expect="discriminator")
        results[name] = rec["classification"]
        if rec["classification"] == "hang_timeout":
            if not canary(built["jit_ctrl_b2"]["exe"]):
                append_jsonl({"kind": "abort", "reason": "canary_failed_after_" + name})
                return 3
    append_jsonl({"kind": "e0b_summary", "results": results})
    log(f"E0b 结果:{results}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
