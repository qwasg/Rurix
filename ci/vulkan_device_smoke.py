#!/usr/bin/env python3
"""mb1 Vulkan compute 设备真跑冒烟(RXS-0207;RFC-0011 §4.6)。

`rurixc --target vulkan saxpy.rx` 产 SPIR-V → `bin/vk_saxpy` 在本机 Vulkan 设备
(NVIDIA / AMD 桌面 / lavapipe)真跑 saxpy = a*x + out,回读数值精确校验 + validation 零报错。
fail-closed:无 Vulkan 设备 → SKIP(开发环境降级,非 fake pass);`RURIX_REQUIRE_REAL=1`
(GPU runner)时缺设备翻硬红。退出码判定(非 grep stdout 充绿——但校验数值正确性经 demo 的
`VK_SAXPY: ok` + exit 0 双判)。lavapipe/SwiftShader 第二 ICD 经 `VK_ICD_FILENAMES` 指定同脚本跑。
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SAXPY_RX = ROOT / "conformance" / "vulkan" / "accept" / "vk_saxpy.rx"


def build(pkg, *args):
    r = subprocess.run(
        ["cargo", "build", "-p", pkg, *args, "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"[vulkan_device] FAIL cargo build -p {pkg}:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        sys.exit(1)


def main():
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
    build("rurixc", "--features", "vulkan-backend")
    build("rurix-rt", "--features", "vulkan", "--bin", "vk_saxpy")
    exe_suffix = ".exe" if sys.platform == "win32" else ""
    rurixc = ROOT / "target" / "debug" / f"rurixc{exe_suffix}"
    vk_saxpy = ROOT / "target" / "debug" / f"vk_saxpy{exe_suffix}"
    if not vk_saxpy.is_file():
        print(f"[vulkan_device] FAIL vk_saxpy 产物缺失: {vk_saxpy}", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as d:
        spv = Path(d) / "saxpy.spv"
        r = subprocess.run(
            [str(rurixc), "--target", "vulkan", str(SAXPY_RX), "-o", str(spv)],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if r.returncode != 0 or not spv.is_file():
            print(f"[vulkan_device] FAIL saxpy codegen:\n{r.stderr}", file=sys.stderr)
            return 1

        env = dict(os.environ, RURIX_VK_VALIDATION="1")
        run = subprocess.run(
            [str(vk_saxpy), str(spv)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            env=env,
        )
        ok = run.returncode == 0 and "VK_SAXPY: ok" in run.stdout
        if ok:
            # validation layer 报错(stderr 含 "Validation Error" / VUID)即整体 FAIL。
            if "Validation Error" in run.stderr or "VUID-" in run.stderr:
                print(
                    f"[vulkan_device] FAIL validation layer 报错:\n{run.stderr}",
                    file=sys.stderr,
                )
                return 1
            print(f"[vulkan_device] PASS 本机 Vulkan 真跑: {run.stdout.strip()}")
            return 0

        # 失败:区分"无设备(SKIP)"与"真错(FAIL)"。
        no_device = any(
            k in run.stderr
            for k in ("vulkan-1.dll", "libvulkan", "物理设备", "compute queue", "vkCreateInstance")
        )
        if no_device and not require_real:
            print(
                f"[vulkan_device] SKIP 无 Vulkan 设备(开发环境降级,非 fake):{run.stderr.strip()}"
            )
            return 0
        print(
            f"[vulkan_device] FAIL vk_saxpy 真跑失败(exit={run.returncode}):\n"
            f"stdout={run.stdout}\nstderr={run.stderr}",
            file=sys.stderr,
        )
        return 1


if __name__ == "__main__":
    sys.exit(main())
