#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""render graph 面 smoke（步骤 65;G3.5 / RFC-0013 §4.D;RXS-0236~0241;验收门 G-G3-5）。

本 smoke 证 **G3.5 render graph 面**:声明式宿主库 Graph/PassBuilder/GraphResource 类型面 +
纯 host 自动资源状态推导（`graph.rs`）+ 🔒 pass 边界 happens-before 语义本体 + 双后端执行器 +
uc04 手动 `plan_barriers` 永续独立复核门（D6 互证金标准）。

  host 段（**恒跑**,反 YAML-only;步骤 65 核心 = 本面最强验收):
    1. **D6 互证金标准**（`src/uc04-demo/tests/d6_crosscheck.rs`,恒跑纯 host 无 GPU):uc04
       deferred 三 pass 图（`deferred::plan_deferred_passes` / RXS-0168）经 `graph.rs` 推导的
       barrier 集，与 uc04 手动 `barrier::plan_barriers` RXS-0169 手动锚点集**集合相等双向断言**
       （两独立实现互证;`graph.rs` 禁 import `barrier.rs`,oracle 独立性,RXS-0239/0241）;
    2. `graph.rs` **图合法性 reject 四族 RX6029 + 声明-反射失配 RX6030**（读未写 / 写写冲突 /
       读写同 pass / 生命周期误用 + 反射双向失配,装配期 strict,RXS-0237）;
    3. `graph.rs` **推导 golden + 确定性双跑**（deferred 图恰 5 条 barrier 逐条锚 + 同图逐字节
       相同 + depth/UAV 独立路由 + AccessKind 双后端映射单一事实源,RXS-0238/0240）;
    4. cabi `rxrt_graph_*` **符号面 + handle-0/未知句柄失败路 + 增量建面 → execute 装配核验**
       （`src/rurix-rt-cabi/src/lib.rs`,RXS-0241）;
    5. **conformance graph 语料**（accept graph_deferred_three_pass 0 诊断 lowering 落
       `rxrt_graph_*` + reject graph_in_kernel RX3015 全拦截,RXS-0236）。

  device 段（**gate GPU + 显示环境 + opt-in**;auto barrier 真跑 = 交互 GPU 链路,**不进
  pr-smoke 硬门**,镜像 bindless 双态先例):
    6. uc04 deferred 三 pass 图迁 Graph API 经 `run_graph` 自动状态推导重跑步骤 48 同判据
       （auto_barrier_deferred_match）+ 漏声明 read → 装配期 strict 拒 RED
       （undeclared_read_strict_reject）+ Vulkan 同图 `run_graph` 出图对照（vulkan_run_graph_match）。
       **D3D12 shim 执行器诚实边界**:shim C++ pass/barrier 数组下发入口大改留后续,device 首跑
       先经 Vulkan `run_graph`（本机活驱动）;判据阈值 = owner 本机迭代校准 TODO。PASS 写
       evidence/graph_<epoch>.json(hazard_ok=true → g3.counter.auto_barrier_hazard_redgreen)。

**SKIP 纪律**:无显示/无 GPU/未 opt-in → device 段 SKIP = dev-env degrade（**非 fake pass**,退
0）;`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。device 真跑须显式 opt-in `RURIX_GRAPH_DEVICE=1`
（或 REQUIRE_REAL=1）。**AMD 真卡见证 = G-MB1-6 硬件尾门独立存续**(本机 RTX 4070 Ti measured
不充作 AMD);run URL 不伪造。
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""

# 无设备(SKIP)信号(镜像 ci/bindless_smoke.py NO_DEVICE_KEYS)。
NO_DEVICE_KEYS = (
    "vulkan-1.dll",
    "libvulkan",
    "vkGetInstanceProcAddr",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
)

# host 段恒跑的互证/推导/cabi/conformance 结构性单测(RXS-0236~0241;工具无关,不依赖 GPU)。
HOST_TESTS = [
    # D6 互证金标准(graph.rs 推导集 == uc04 RXS-0169 手动锚点集,集合相等双向;本面最强验收)。
    (
        ["cargo", "test", "-p", "uc04-demo", "--test", "d6_crosscheck"],
        "D6 互证金标准(graph.rs 推导 == uc04 plan_barriers RXS-0169 手动锚点,双向集合相等)",
    ),
    # graph.rs 图合法性 reject 四族 RX6029 + 声明-反射失配 RX6030 + 推导 golden + 确定性双跑 +
    # depth/UAV 独立路由 + AccessKind 双后端映射单一事实源(RXS-0237/0238/0240)。
    (
        ["cargo", "test", "-p", "rurix-rt", "--lib", "graph::"],
        "graph.rs 推导 golden + 确定性双跑 + reject 四族 RX6029/RX6030 + 双后端映射单一事实源",
    ),
    # cabi rxrt_graph_* 符号面 + handle-0/未知句柄失败路 + 增量建面 → execute 装配核验(RXS-0241)。
    (
        ["cargo", "test", "-p", "rurix-rt-cabi", "--lib",
         "tests::graph_symbols_failure_path_and_incremental_build"],
        "rurix-rt-cabi rxrt_graph_* 符号面 + 增量建面 execute 装配核验",
    ),
    # conformance graph 语料(accept 三 pass 图 lowering 落 rxrt_graph_* + reject graph_in_kernel
    # RX3015 全拦截,RXS-0236)。
    (
        ["cargo", "test", "-p", "rurixc", "--test", "host_orch_corpus", "--",
         "accept_graph_deferred_three_pass_lowers_to_rxrt_graph",
         "reject_corpus_all_intercepted"],
        "rurixc render graph 宿主接线语料(accept lowering + reject RX3015)",
    ),
]


def fail(msg: str) -> int:
    print(f"[render_graph_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg)
    print(f"[render_graph_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


# ─────────────────────────── host 段（恒跑，步骤 65 核心） ───────────────────────────


def host_section() -> bool:
    """host 段恒跑:D6 互证 set-equality + graph.rs 图合法性 reject + 推导 golden 确定性双跑 +
    cabi 符号面 + conformance accept/reject。全绿返回 True。"""
    for cmd, label, *_ in HOST_TESTS:
        p = run(cmd)
        if p.returncode != 0 or "test result: ok" not in (p.stdout + p.stderr):
            print((p.stdout + p.stderr)[-2500:], file=sys.stderr)
            print(f"[render_graph_smoke] host 段 FAIL: {label} 未过", file=sys.stderr)
            return False
        print(f"[render_graph_smoke] host 段 OK: {label}")
    print("[render_graph_smoke] host 段全绿(D6 互证金标准 set-equality + 图合法性 reject 四族 "
          "RX6029/RX6030 + 推导 golden 确定性双跑 + rxrt_graph_* 符号面 + conformance accept/reject RX3015)")
    return True


# ─────────────────────────── device 段（步骤 65，SKIP 三态） ───────────────────────────


def device_opt_in() -> bool:
    return (
        os.environ.get("RURIX_GRAPH_DEVICE") == "1"
        or os.environ.get("RURIX_REQUIRE_REAL") == "1"
    )


def device_section() -> int:
    """device 段:uc04 deferred 三 pass 图迁 Graph API 经 run_graph 自动状态推导重跑步骤 48 同判据
    + 漏声明 read → 装配期 strict 拒 RED + Vulkan 同图 run_graph 对照。

    device 真跑 = 交互 GPU 链路(活驱动),归主循环 owner 本机错峰见证。**D3D12 shim 执行器诚实
    边界**:shim C++ pass/barrier 数组下发入口大改留后续,device 首跑先经 Vulkan run_graph;判据
    阈值(采样点/期望色/容差)= owner 本机迭代校准 TODO。PASS 写 evidence/graph_<epoch>.json
    (hazard_ok=true → g3.counter.auto_barrier_hazard_redgreen PASS)。**AMD 真卡见证 = G-MB1-6
    硬件尾门独立存续**;host 段(D6 互证金标准)已为本面核心恒跑验收。
    """
    if not device_opt_in():
        return skip(
            "device 段未 opt-in(auto barrier 真跑 = 交互 GPU 链路;设 RURIX_GRAPH_DEVICE=1 或 "
            "RURIX_REQUIRE_REAL=1 启用)——uc04→Graph run_graph 重跑步骤 48 + 漏声明 strict 拒 RED "
            "+ Vulkan 同图对照归 owner 本机活驱动错峰见证(D3D12 shim 执行器诚实边界,判据阈值 TODO)"
        )
    # device 执行器 harness(uc04 迁 Graph + Vulkan run_graph)归主循环活驱动落地;本 smoke 阶段
    # 无独立 graph device harness bin(run_graph 设备段真跑归主循环,D3D12 shim 入口大改留后续),
    # 故 opt-in 亦诚实 SKIP(不伪造 device 绿,G-G3-5 防降级硬门;REQUIRE_REAL=1 翻硬红提醒)。
    return skip(
        "device 段 render graph 执行器 harness 未就位(Vulkan run_graph device 首跑归主循环活驱动;"
        "D3D12 shim pass/barrier 数组下发入口大改留后续 = 诚实边界)——host 段 D6 互证金标准为本面"
        "核心恒跑验收,device 首跑判据(uc04→Graph run_graph 重跑步骤 48 + 漏声明 strict 拒 RED)归 "
        "owner 本机错峰;不伪造 device 绿(G-G3-5 防降级硬门)"
    )


def main() -> int:
    print("[render_graph_smoke] 步骤 65(G3.5 render graph 面,RFC-0013 §4.D,RXS-0236~0241)")
    if not host_section():
        return 1
    rc = device_section()
    # host 恒跑绿 + device SKIP/PASS;evidence 仅 device 真跑写(此处不伪造 hazard_ok)。
    _ = (EVIDENCE_DIR, _dt, json, github_run_url)  # device 真跑回填时消费。
    return rc


if __name__ == "__main__":
    sys.exit(main())
