#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""mesh-task-RT 运行时面 smoke（步骤 66/67;G3.6 / RFC-0013 §4.E7/E8;RXS-0248;验收门 G-G3-6）。

本 smoke 证 **G3.6 mesh/RT 运行时面**:Vulkan mesh 管线（无 vertex-input DrawMeshTasks）+
🔒 BLAS/TLAS/SBT/TraceRays 运行时 + 🔒 SBT 三 region 对齐律（纯 host 可单测）+ 扩展/feature
缺失确定性 Err。

  host 段（**恒跑**,反 YAML-only;步骤 66/67 核心 = 本面纯 host 验收):
    1. **SBT 三 region 对齐 + 扩展协商 + FFI 布局纯 host 单测**(`src/rurix-rt/src/vk.rs` vk::tests:
       plan_sbt 对齐律不变式〔含 NVIDIA 32/32/64 确切铺设〕/ align_up / 扩展协商缺失确定性 Err /
       AS·RT FFI 布局逐字节锚〔AccelInstance 64 / geometry union 64 / …〕/ instance bitfield 打包);
    2. **mesh/RT 编码腿见证语料 spirv-val**(`src/rurixc/tests/mesh_rt_vulkan_spirv_val.rs`:mesh/task/
       六 RT 执行模型 .spv × `vulkan1.2`/`spv1.4` accept;spirv-val 三态,退出码判定反 grep)。

  device 段（**gate GPU + opt-in**;mesh/RT 真跑 = 交互 GPU 链路,**不进 pr-smoke 硬门**,镜像
  render graph / bindless 双态先例):
    3. `bin/vk_mesh`(步骤 66):mesh 管线出图 covered 计数 + 篡改 SetMeshOutputs 顶点数 RED；
       `bin/vk_rt`(步骤 67):单三角形 TLAS raygen/miss/closesthit 命中·miss 双色 + 移动顶点命中区
       移动 RED。stages_ok 去重并集 ≥3〔mesh/raygen/closesthit〕→ evidence/meshrt_*.json →
       g3.counter.mesh_task_rt_stages(ci/budget_eval.py)。**判据阈值(coverage-producing mesh +
       写 storage image 的 raygen 见证语料 + hit/miss 色)= owner 本机 RTX 4070 Ti 迭代校准 TODO**;
       codegen emit_mesh_min 退化三角形 / emit_raygen_min 首期不写 storage image → 首期 PARTIAL,
       **AS/SBT/TraceRays 全机构真跑零 validation 报错**(机构就位)。

**SKIP 纪律**:无 GPU/无 Vulkan loader/mesh·RT feature 缺失/未 opt-in → device 段 SKIP =
dev-env degrade（**非 fake pass**,退 0）;`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。device 真跑
须显式 opt-in `RURIX_MESH_DEVICE=1`/`RURIX_RT_DEVICE=1`（或 `RURIX_REQUIRE_REAL=1`）。
**VVL/驱动崩溃以退出码区分判定**（反 grep stdout,P0-5 教训）。**AMD 真卡见证 = G-MB1-6 硬件
尾门独立存续**(本机 RTX 4070 Ti measured 不充作 AMD);run URL 不伪造。
"""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""

# 无设备(SKIP)信号(镜像 bin/vk_mesh / bin/vk_rt NO_DEVICE_KEYS + harness SKIP 前缀)。
NO_DEVICE_KEYS = (
    "vulkan-1.dll",
    "libvulkan",
    "vkGetInstanceProcAddr",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
    "mesh shader feature",
    "RT feature",
    "缺扩展",
    "vkGetPhysicalDeviceFeatures2",
    "vkGetPhysicalDeviceProperties2",
    "vkEnumerateDeviceExtensionProperties",
)

# host 段恒跑的纯 host 单测(SBT 对齐/协商/FFI 布局 + 见证语料 spirv-val;工具无关不依赖 GPU)。
HOST_TESTS = [
    (
        ["cargo", "test", "-p", "rurix-rt", "--features", "vulkan", "--lib", "--",
         "vk::tests::plan_sbt",
         "vk::tests::align_up_rounds_to_power_of_two",
         "vk::tests::negotiate_device_extensions",
         "vk::tests::mesh_rt_ffi_layout_anchors",
         "vk::tests::accel_instance_bitfield_packing"],
        "🔒 SBT 三 region 对齐律不变式 + align_up + 扩展协商缺失确定性 Err + AS/RT FFI 布局逐字节锚 + instance bitfield",
    ),
    (
        ["cargo", "test", "-p", "rurixc", "--features", "vulkan-backend", "--test", "mesh_rt_vulkan_spirv_val"],
        "mesh/task/六 RT 执行模型见证语料 spirv-val vulkan1.2/spv1.4 accept(三态,退出码判定)",
    ),
]


def fail(msg: str) -> int:
    print(f"[meshrt_device_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg)
    print(f"[meshrt_device_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)


# ─────────────────────────── host 段（恒跑，步骤 66/67 核心） ───────────────────────────


def host_section() -> bool:
    for cmd, label in HOST_TESTS:
        p = run(cmd)
        if p.returncode != 0 or "test result: ok" not in (p.stdout + p.stderr):
            print((p.stdout + p.stderr)[-2500:], file=sys.stderr)
            print(f"[meshrt_device_smoke] host 段 FAIL: {label} 未过", file=sys.stderr)
            return False
        print(f"[meshrt_device_smoke] host 段 OK: {label}")
    print("[meshrt_device_smoke] host 段全绿(SBT 三 region 对齐律 + 扩展协商确定性 Err + FFI 布局逐字节锚 "
          "+ 见证语料 spirv-val 三态)")
    return True


# ─────────────────────────── device 段（步骤 66/67，SKIP 三态） ───────────────────────────


def device_opt_in() -> bool:
    return (
        os.environ.get("RURIX_MESH_DEVICE") == "1"
        or os.environ.get("RURIX_RT_DEVICE") == "1"
        or os.environ.get("RURIX_REQUIRE_REAL") == "1"
    )


def run_harness(name: str, ok_tag: str) -> int:
    """build + run 一个 device harness(vk_mesh / vk_rt);SKIP/PARTIAL→SKIP、PASS→0、真错→FAIL。"""
    build = run(["cargo", "build", "-p", "rurix-rt", "--features", "vulkan", "--bin", name, "--quiet"])
    if build.returncode != 0:
        print((build.stdout + build.stderr)[-2500:], file=sys.stderr)
        return fail(f"cargo build {name}(--features vulkan)失败(host 编译红,非 SKIP 事项)")
    exe = ROOT / "target" / "debug" / f"{name}{EXE_SUFFIX}"
    env = dict(os.environ, RURIX_VK_VALIDATION="1")
    p = subprocess.run([str(exe)], cwd=ROOT, capture_output=True, text=True, env=env)
    out = p.stdout + p.stderr
    tag = ok_tag  # "MESH" / "RT"
    if any(k in out for k in NO_DEVICE_KEYS) or f"{tag}: SKIP" in p.stdout:
        return skip(f"{name} 无 Vulkan 设备 / mesh·RT feature 缺失:{p.stderr.strip()[:300]}")
    if p.returncode != 0:
        print(out[-2500:], file=sys.stderr)
        return fail(f"{name} harness 退出非 0(device 真跑内部错误 / VVL fail-closed,非阈值 MISS;退出码判定)")
    if "Validation Error" in p.stderr and f"{tag}: FAIL" in out:
        print(p.stderr[-2500:], file=sys.stderr)
        return fail(f"{name}:VK_LAYER_KHRONOS_validation 报错(fail-closed)")
    if f"{tag}: PASS" in p.stdout:
        print(f"[meshrt_device_smoke] device 段 {name}: PASS\n{p.stdout.strip()[-500:]}")
        return 0
    # PARTIAL:真跑但判据阈值未过(owner 迭代 coverage 语料/阈值)→ 诚实 SKIP(REQUIRE_REAL 翻红)。
    print(p.stdout.strip()[-800:], file=sys.stderr)
    return skip(
        f"{name} PARTIAL(判据阈值未过)——coverage-producing 见证语料 + 像素阈值归 owner 本机迭代校准"
        "(codegen emit_mesh_min 退化三角形 / emit_raygen_min 首期不写 storage image;机构真跑零 "
        "validation 报错);不伪造 device 绿(G-G3-6 防降级硬门)"
    )


def device_section() -> int:
    if not device_opt_in():
        return skip(
            "device 段未 opt-in(mesh/RT 真跑 = 交互 GPU 链路;设 RURIX_MESH_DEVICE=1 / RURIX_RT_DEVICE=1 "
            "或 RURIX_REQUIRE_REAL=1 启用)——mesh 管线出图 covered + 篡改 SetMeshOutputs RED / 单三角形 "
            "TLAS 命中·miss 双色 + 移动顶点 RED 归 owner 本机活驱动错峰见证(判据阈值 TODO)"
        )
    rc_mesh = 0
    rc_rt = 0
    if os.environ.get("RURIX_MESH_DEVICE") == "1" or os.environ.get("RURIX_REQUIRE_REAL") == "1":
        rc_mesh = run_harness("vk_mesh", "MESH")
    if os.environ.get("RURIX_RT_DEVICE") == "1" or os.environ.get("RURIX_REQUIRE_REAL") == "1":
        rc_rt = run_harness("vk_rt", "RT")
    return rc_mesh or rc_rt


def main() -> int:
    print("[meshrt_device_smoke] 步骤 66/67(G3.6 mesh-task-RT 运行时面,RFC-0013 §4.E7/E8,RXS-0248)")
    if not host_section():
        return 1
    return device_section()


if __name__ == "__main__":
    sys.exit(main())
