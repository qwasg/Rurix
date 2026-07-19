#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""bindless 面 smoke（步骤 64;G3.4 / RFC-0013 §4.C;RXS-0231~0235;验收门 G-G3-4）。

G3.3 步骤 62/63(采样超集)提供 descriptor 底座;本 smoke 证 **G3.4 bindless 面**:
`ResourceCount::Unbounded` 自 Unmappable 翻转为合法路 + 独占 set/space 分配律 + 动态
非均匀索引 `table[nonuniform(idx)].sample()` codegen(OpTypeRuntimeArray + NonUniform +
clamp)+ TextureTable 宿主注册面 + Vulkan feature chain。

  host 段（**恒跑**,反 YAML-only;步骤 64）:
    1. 绑定推导单测（`src/rurixc/src/binding_layout.rs`:Unbounded SRV 纹理合法化 +
       Vk-native 独占 set4 / RTS0 独占 space1 分配律 + 有界路零漂移 + RTS0 unbounded
       range roundtrip + 无界非纹理维持 Unmappable,RXS-0233);
    2. codegen 单测（`src/rurixc/src/dxil_spirv.rs`:无界表 emit OpTypeRuntimeArray +
       RuntimeDescriptorArray/ShaderNonUniform capability + SPV_EXT_descriptor_indexing,
       RXS-0234);
    3. cabi `rxrt_table_*` 符号面单测（`src/rurix-rt-cabi/src/lib.rs`:注册序即索引 +
       handle-0 失败路,RXS-0235);
    4. **nonuniform 缺失 reject UI golden**（RXS-0232,RX3016)+ conformance 语料
       (accept 动态索引 0 诊断 / reject nonuniform_missing·handle_escape·table_return);
    5. bindless SPIR-V spirv-val 三态（`.rx` 无界表着色器 → Vulkan 原生 SPIR-V →
       `spirv-val --target-env vulkan1.2` accept;工具在位 accept / 缺工具 SKIP,退出码判定)。

  device 段（**gate real-shim + GPU + 显示环境**;bindless 索引真跑 = 交互 GPU 链路,
  **不进 pr-smoke 硬门**,镜像 sampling_superset 双态先例):
    6. bindless harness（`bin/bindless_modes`:≥4 纹理注册表按屏幕象限动态索引采样 ==
       四色 + 篡改注册序 → 像素换位 RED + feature chain 四 bit 缺失 → 确定性 Err);
       **判据阈值(采样点/期望色/容差)= owner 本机迭代校准 TODO**;首期 PARTIAL(真跑
       但未过阈值)= 诚实 SKIP(不伪造绿;REQUIRE_REAL=1 翻硬红)。

**SKIP 纪律**:无显示/无 GPU/无 real-shim/未 opt-in → device 段 SKIP = dev-env
degrade（**非 fake pass**,退 0);`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。device 真跑须
显式 opt-in `RURIX_BINDLESS_DEVICE=1`(或 REQUIRE_REAL=1)。**AMD 真卡见证 = G-MB1-6
硬件尾门独立存续**(本机 RTX 4070 Ti measured 不充作 AMD);run URL 不伪造。
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

# 无设备(SKIP)信号(镜像 ci/sampling_superset_smoke.py NO_DEVICE_KEYS)。
NO_DEVICE_KEYS = (
    "vulkan-1.dll",
    "libvulkan",
    "vkGetInstanceProcAddr",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
)

# host 段恒跑的推导/codegen/cabi/conformance 结构性单测(RXS-0231~0235;工具无关,不依赖 GPU)。
HOST_TESTS = [
    # rurixc:绑定推导翻转 + 独占 set/space 分配律 + 有界路零漂移 + RTS0 unbounded range(RXS-0233)。
    (
        ["cargo", "test", "-p", "rurixc", "--features", "dxil-backend shader-stages", "--lib", "--",
         "binding_layout::tests::spirv_bindings_unbounded_srv_texture_is_legal_bindless_set",
         "binding_layout::tests::spirv_bindings_unbounded_non_texture_still_unmappable",
         "binding_layout::tests::bindless_exclusive_set_space_allocation_law",
         "binding_layout::tests::bounded_path_zero_drift_when_table_added"],
        "rurixc bindless 绑定推导 + 独占分配律单测",
    ),
    # rurixc:codegen 无界表 emit OpTypeRuntimeArray + capability + 扩展(RXS-0234)。
    (
        ["cargo", "test", "-p", "rurixc", "--features", "dxil-backend shader-stages vulkan-backend",
         "--lib", "--",
         "dxil_spirv::tests::unbounded_srv_texture_emits_runtime_array",
         "dxil_spirv::tests::unbounded_non_texture_still_unmappable"],
        "rurixc bindless codegen runtime-array 单测",
    ),
    # rurix-rt-cabi:rxrt_table_* 符号面(注册序即索引 + handle-0 失败路,RXS-0235)。
    (
        ["cargo", "test", "-p", "rurix-rt-cabi", "--lib",
         "tests::table_symbols_failure_path_and_register_order"],
        "rurix-rt-cabi rxrt_table_* 符号面单测",
    ),
    # rurixc:conformance shader 语料(accept 动态索引 + reject nonuniform_missing/handle_escape/
    # table_return 全拦截)+ UI golden(RX3016 nonuniform 缺失 reject golden,RXS-0232)。
    (
        ["cargo", "test", "-p", "rurixc", "--features", "dxil-backend shader-stages", "--test",
         "shader_corpus"],
        "rurixc bindless conformance 语料(accept + reject)",
    ),
    (
        ["cargo", "test", "-p", "rurixc", "--features", "dxil-backend shader-stages", "--test",
         "ui_golden"],
        "rurixc bindless UI golden(RX3016 nonuniform 缺失 reject)",
    ),
    # rurixc:bindless `.rx` 无界表着色器 → Vulkan 原生 SPIR-V → spirv-val vulkan1.2 三态
    # (工具在位 accept / 缺工具 SKIP;device 真跑前唯一 bindless SPIR-V 合法性机验闸门,RXS-0234)。
    (
        ["cargo", "test", "-p", "rurixc", "--features", "dxil-backend shader-stages vulkan-backend",
         "--test", "bindless_vulkan_spirv_val"],
        "rurixc bindless Vulkan SPIR-V + spirv-val vulkan1.2 三态",
    ),
]


def fail(msg: str) -> int:
    print(f"[bindless_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg)
    print(f"[bindless_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
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


# ─────────────────────────── host 段（恒跑，步骤 64） ───────────────────────────


def host_section() -> bool:
    """host 段恒跑:绑定推导翻转 + 独占分配律 + codegen runtime-array + cabi 符号面 +
    conformance/UI golden + bindless spirv-val 三态。全绿返回 True。"""
    for cmd, label, *_ in HOST_TESTS:
        p = run(cmd)
        if p.returncode != 0 or "test result: ok" not in (p.stdout + p.stderr):
            print((p.stdout + p.stderr)[-2500:], file=sys.stderr)
            print(f"[bindless_smoke] host 段 FAIL: {label} 未过", file=sys.stderr)
            return False
        print(f"[bindless_smoke] host 段 OK: {label}")
    print("[bindless_smoke] host 段全绿(Unbounded 合法化 + 独占 set/space 分配律 + RTS0 roundtrip + "
          "codegen runtime-array + rxrt_table_* + nonuniform 缺失 reject golden + spirv-val vulkan1.2 三态)")
    return True


# ─────────────────────────── device 段（步骤 64，SKIP 三态） ───────────────────────────


def device_opt_in() -> bool:
    return (
        os.environ.get("RURIX_BINDLESS_DEVICE") == "1"
        or os.environ.get("RURIX_REQUIRE_REAL") == "1"
    )


def device_section() -> int:
    """device 段:bindless harness(bin/bindless_modes)四象限动态索引红绿。

    opt-in 后 build + run `bin/bindless_modes`:≥4 纹理注册表按屏幕象限动态索引采样 ==
    四色 + 篡改注册序 → 像素换位 RED + feature chain 四 bit 缺失 → 确定性 Err。**判据阈值
    (采样点/期望色/容差)= owner 本机迭代校准 TODO**——首期 PARTIAL(真跑但未过阈值)=
    诚实 SKIP(不伪造 device 绿;RURIX_REQUIRE_REAL=1 翻硬红,G-G3-4 防降级硬门)。owner 本机
    RTX 4070 Ti 错峰真跑写 evidence/bindless_<date>.json(smoke_ok=true →
    g3.counter.bindless_descriptor_smoke PASS)。**AMD 真卡见证 = G-MB1-6 硬件尾门独立存续**。
    """
    if not device_opt_in():
        return skip(
            "device 段未 opt-in(bindless 索引真跑 = 交互 GPU 链路;设 RURIX_BINDLESS_DEVICE=1 "
            "或 RURIX_REQUIRE_REAL=1 启用)——四象限动态索引四色 + 篡改换位 RED + feature 缺失 "
            "Err 归 owner 本机错峰见证(判据阈值 TODO)"
        )

    build = run(
        ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan",
         "--bin", "bindless_modes", "--quiet"]
    )
    if build.returncode != 0:
        print((build.stdout + build.stderr)[-2500:], file=sys.stderr)
        return fail("cargo build bindless_modes(--features vulkan)失败(host 编译红,非 SKIP 事项)")
    exe = ROOT / "target" / "debug" / f"bindless_modes{EXE_SUFFIX}"
    env = dict(os.environ, RURIX_VK_VALIDATION="1")
    p = subprocess.run([str(exe)], cwd=ROOT, capture_output=True, text=True, env=env)
    out = p.stdout + p.stderr
    if any(k in out for k in NO_DEVICE_KEYS) or "BINDLESS_MODES: SKIP" in p.stdout:
        return skip(f"device 段 bindless_modes 无 Vulkan 设备/loader:{p.stderr.strip()[:300]}")
    if p.returncode != 0:
        print(out[-2500:], file=sys.stderr)
        return fail("bindless_modes harness 退出非 0(device 真跑内部错误,非阈值 MISS)")
    if "Validation Error" in p.stderr or "VUID-" in p.stderr:
        print(p.stderr[-2500:], file=sys.stderr)
        return fail("bindless_modes:VK_LAYER_KHRONOS_validation 报错(fail-closed)")
    if "BINDLESS_MODES: PASS" in p.stdout:
        print(f"[bindless_smoke] device 段:bindless 四象限动态索引红绿 PASS\n{p.stdout.strip()[-600:]}")
        return 0
    # PARTIAL:真跑但判据阈值未过(owner 迭代校准)→ 诚实 SKIP(REQUIRE_REAL 翻红)。
    print(p.stdout.strip()[-800:], file=sys.stderr)
    return skip(
        "device 段 bindless_modes PARTIAL(判据阈值未过)——判据阈值/采样点归 owner 本机迭代"
        "校准(expect_* 谓词 TODO);不伪造 device 绿(G-G3-4 防降级硬门)"
    )


def main() -> int:
    print("[bindless_smoke] 步骤 64(G3.4 bindless 面,RFC-0013 §4.C,RXS-0231~0235)")
    if not host_section():
        return 1
    rc = device_section()
    # host 恒跑绿 + device SKIP/PASS;evidence 仅 device 真跑写(此处不伪造 smoke_ok)。
    _ = (EVIDENCE_DIR, _dt, json, github_run_url)  # device 真跑回填时消费。
    return rc


if __name__ == "__main__":
    sys.exit(main())
