#!/usr/bin/env python3
"""mb1 W6 Vulkan win32 swapchain present 真跑冒烟(RXS-0210 L4 present 落地;RFC-0011 §4.6)。

`rurixc --target vulkan vk_tri_{vs,fs}.rx` 产 SPIR-V(方案 B:去 UserSemantic/SPV_GOOGLE)→
`bin/vk_present` 在本机 Vulkan 设备(NVIDIA / AMD 桌面)创建隐藏 win32 窗口 + `VkSurfaceKHR`
(`VK_KHR_win32_surface`)+ `VkSwapchainKHR`(`VK_KHR_swapchain`)→ 渲染数帧居中三角形到
swapchain image → `vkCmdCopyImageToBuffer` 回读(反证 present 可数值校验)→ 转 `PRESENT_SRC_KHR`
→ `vkQueuePresentKHR` → 逐像素断言(背景角==clear / 中心覆盖非背景 / covered>0)+ present 逐帧
`VK_SUCCESS`/`SUBOPTIMAL` + `VK_LAYER_KHRONOS_validation` 零报错。

fail-closed 纪律(反 Godot 退出码/grep 教训):
  - rurixc(--features vulkan-backend)+ rurix-rt(--features vulkan --bin vk_present)build 必绿。
  - device 真跑:vk_present 退出 0 + `VK_PRESENT: ok` + stderr 无 `Validation Error`/`VUID-`
    → PASS。无 Vulkan 设备 → SKIP(dev-env degrade);`RURIX_REQUIRE_REAL=1` 缺设备翻硬红。
  - **非 Windows**:win32 surface 不可用(`run_graphics_present` 确定性 `Err` windows-only;
    Android present = 尾门 G-MB1-7)→ SKIP(`RURIX_REQUIRE_REAL=1` 亦 SKIP,非 fake、非硬红——
    present 硬门只在 Windows GPU runner;非 Windows 无 win32 surface 对象)。
  - **内建 red_self_test(退出码判定,自反证非工作树改)**:经 `examples/emit_spirv_provenance`
    (provenance=true,DXIL 路,带 SPV_GOOGLE)对同源产**带保名** `.spv` → 喂同 `vk_present`
    present 管线 → `VK_LAYER_KHRONOS_validation` 报 VUID-VkShaderModuleCreateInfo-pCode-08742,
    `run_graphics_present` 的 debug messenger fail-closed 翻 `Err` → **vk_present 退出非 0**
    (证 present 路径**同样** fail-closed,validation 消息真被捕获判红)。
退出码判定(非 grep stdout 充绿——校验正确性经 demo 的 `VK_PRESENT: ok` + exit 0 双判)。

窗口/swapchain present 完成 RXS-0210 的 L4 present-defer(RD-032 的 code-deferral 已 discharge);
AMD 真卡 present 像素校验 = 尾门 G-MB1-6,Android surface present = 尾门 G-MB1-7(均 RD-032 open)。
"""

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ACCEPT_DIR = ROOT / "conformance" / "vulkan" / "accept"
VS_RX = ACCEPT_DIR / "vk_tri_vs.rx"
FS_RX = ACCEPT_DIR / "vk_tri_fs.rx"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""

FAILURES: list[str] = []
NOTES: list[str] = []

# 无设备 / 非-win32(SKIP)信号:run_graphics_present 缺 Vulkan 运行时 / 非 Windows 的确定性 Err 串。
NO_DEVICE_KEYS = (
    "vulkan-1.dll",
    "libvulkan",
    "vkGetInstanceProcAddr",
    "物理设备",
    "graphics queue",
    "present-capable",
    "vkCreateInstance",
    "vkCreateWin32SurfaceKHR",
    "windows-only",
)


def check(cond, msg):
    if not cond:
        FAILURES.append(msg)


def note(msg):
    NOTES.append(msg)


def build(pkg, *args):
    r = subprocess.run(
        ["cargo", "build", "-p", pkg, *args, "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"[vulkan_present] FAIL cargo build -p {pkg} {args}:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        sys.exit(1)


def locate(tool, env_var):
    env = os.environ.get(env_var)
    if env and Path(env).is_file():
        return env
    return shutil.which(tool)


def compile_vulkan(rurixc, rx_path, spv_path):
    r = subprocess.run(
        [str(rurixc), "--target", "vulkan", str(rx_path), "-o", str(spv_path)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return r.returncode, r.stderr


def emit_provenance(example_exe, rx_path, spv_path):
    """DXIL 路(provenance=true)产带 SPV_GOOGLE/UserSemantic 的 `.spv`(red_self_test 反证)。"""
    r = subprocess.run(
        [str(example_exe), str(rx_path), str(spv_path)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return r.returncode, r.stderr


def spirv_val_accepts(tool, spv_path):
    r = subprocess.run(
        [tool, "--target-env", "vulkan1.0", str(spv_path)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return r.returncode == 0


def run_present(vk_present, vs_spv, fs_spv):
    env = dict(os.environ, RURIX_VK_VALIDATION="1")
    return subprocess.run(
        [str(vk_present), str(vs_spv), str(fs_spv)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
    )


def is_no_device(text):
    return any(k in text for k in NO_DEVICE_KEYS)


def main():
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"

    build("rurixc", "--features", "vulkan-backend")
    build("rurixc", "--features", "vulkan-backend", "--example", "emit_spirv_provenance")
    build("rurix-rt", "--features", "vulkan", "--bin", "vk_present")

    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    vk_present = ROOT / "target" / "debug" / f"vk_present{EXE_SUFFIX}"
    example_exe = ROOT / "target" / "debug" / "examples" / f"emit_spirv_provenance{EXE_SUFFIX}"
    for p, name in ((rurixc, "rurixc"), (vk_present, "vk_present"), (example_exe, "emit_spirv_provenance")):
        check(p.is_file(), f"产物缺失: {name} ({p})")
    check(VS_RX.is_file() and FS_RX.is_file(), f"conformance 语料缺失: {VS_RX} / {FS_RX}")
    if FAILURES:
        return _report(False, False)

    spirv_val = locate("spirv-val", "RURIX_SPIRV_VAL")
    if spirv_val is None and require_real:
        print("[vulkan_present] FAIL RURIX_REQUIRE_REAL=1 但 spirv-val 缺失(GPU runner 应有 Vulkan SDK)", file=sys.stderr)
        return 1
    if spirv_val is None:
        note("spirv-val 不可用:校验段 SKIP(开发环境降级)")

    with tempfile.TemporaryDirectory() as d:
        d = Path(d)
        vs = d / "vs.spv"
        fs = d / "fs.spv"
        vs_prov = d / "vs_prov.spv"
        fs_prov = d / "fs_prov.spv"

        # ── Phase 1 codegen(方案 B,vulkan 原生路)──
        for rx, spv in ((VS_RX, vs), (FS_RX, fs)):
            code, stderr = compile_vulkan(rurixc, rx, spv)
            check(code == 0, f"{rx.name}: --target vulkan 应绿(0)却退出 {code}\n{stderr}")
            check(spv.is_file(), f"{rx.name}: .spv 未产出")

        # 带保名变体(DXIL 路,red_self_test 反证源)。
        for rx, spv in ((VS_RX, vs_prov), (FS_RX, fs_prov)):
            code, stderr = emit_provenance(example_exe, rx, spv)
            check(code == 0, f"{rx.name}: emit_spirv_provenance 应绿(0)却退出 {code}\n{stderr}")

        # ── spirv-val:vulkan 变体接受(可选,工具在位即真跑)──
        if spirv_val is not None:
            for spv, label in ((vs, "vk_tri_vs"), (fs, "vk_tri_fs")):
                if spv.is_file():
                    check(spirv_val_accepts(spirv_val, spv), f"{label}: spirv-val vulkan1.0 拒绝合法 vulkan 变体")

        if FAILURES:
            return _report(spirv_val is not None, False)

        # ── device 真跑(good 路)+ red_self_test(provenance 路)──
        device_ran = False
        good = run_present(vk_present, vs, fs)
        good_out = good.stdout + good.stderr
        if good.returncode != 0 and is_no_device(good_out):
            if require_real and "windows-only" not in good_out:
                # win32 present 硬门只在 Windows GPU runner;缺 Vulkan 设备(非平台缺失)翻硬红。
                print(f"[vulkan_present] FAIL RURIX_REQUIRE_REAL=1 但无 Vulkan 设备:\n{good.stderr}", file=sys.stderr)
                return 1
            reason = "非 Windows(win32 surface 不可用;Android present = G-MB1-7 尾门)" if "windows-only" in good_out else "无 Vulkan 设备"
            note(f"{reason}:present 真跑段 + red_self_test SKIP(dev-env degrade):{good.stderr.strip()}")
        else:
            device_ran = True
            # good 路:exit 0 + VK_PRESENT: ok + validation 静默。
            check(good.returncode == 0, f"good 路 vk_present 应退出 0 却 {good.returncode}\nstdout={good.stdout}\nstderr={good.stderr}")
            check("VK_PRESENT: ok" in good.stdout, f"good 路应见 'VK_PRESENT: ok':\n{good.stdout}")
            check(
                "Validation Error" not in good.stderr and "VUID-" not in good.stderr,
                f"good 路 validation 应静默却报错:\n{good.stderr}",
            )

            # red_self_test:provenance 变体 → VUID-08742 → present 路径退出码判红(fail-closed)。
            red = run_present(vk_present, vs_prov, fs_prov)
            red_out = red.stdout + red.stderr
            check(
                red.returncode != 0,
                f"red_self_test: provenance 变体应退出非 0(present 路 fail-closed)却 {red.returncode}\nstdout={red.stdout}\nstderr={red.stderr}",
            )
            check(
                "08742" in red_out,
                f"red_self_test: provenance 变体应触 VUID-...-08742 却未见:\nstdout={red.stdout}\nstderr={red.stderr}",
            )
            check(
                "VK_PRESENT: ok" not in red.stdout,
                f"red_self_test: provenance 变体不应打印 'VK_PRESENT: ok'(应 fail-closed 前退):\n{red.stdout}",
            )

    return _report(spirv_val is not None, device_ran)


def _report(had_val, device_ran):
    for m in NOTES:
        print(f"[vulkan_present] NOTE {m}")
    if FAILURES:
        print(f"[vulkan_present] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    val_seg = "spirv-val✓" if had_val else "spirv-val SKIP"
    dev_seg = "win32 present✓ + red_self_test(VUID-08742)✓" if device_ran else "present SKIP"
    print(f"[vulkan_present] PASS (vk_tri_{{vs,fs}} → SPIR-V,{val_seg},{dev_seg})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
