#!/usr/bin/env python3
"""mb1 Phase 3 Vulkan graphics offscreen 真跑冒烟(RXS-0210;RFC-0011 §4.6)。

`rurixc --target vulkan vk_tri_{vs,fs}.rx` 产 SPIR-V(方案 B:去 UserSemantic/SPV_GOOGLE
provenance)→ `bin/vk_triangle` 在本机 Vulkan 设备(NVIDIA / AMD 桌面 / lavapipe)offscreen
渲染居中三角形 → `vkCmdCopyImageToBuffer` 回读 → 逐像素断言(背景角==clear / 中心覆盖非背景 /
covered>0)+ `VK_LAYER_KHRONOS_validation` 零报错。

fail-closed 纪律(反 Godot 退出码/grep 教训):
  - rurixc(--features vulkan-backend)+ rurix-rt(--features vulkan --bin vk_triangle)build 必绿。
  - spirv-val 在位 → 两 `.spv` 经 `spirv-val --target-env vulkan1.0` 接受;spirv-dis 在位 →
    grep 断言 vulkan 变体**无** `SPV_GOOGLE`/`UserSemantic`(方案 B 生效反证)。工具缺失 → 该段
    SKIP(开发环境降级,非 fake pass);`RURIX_REQUIRE_REAL=1` 缺工具翻硬红。
  - device 真跑:vk_triangle 退出 0 + `VK_TRIANGLE: ok` + stderr 无 `Validation Error`/`VUID-`
    → PASS。无 Vulkan 设备 → SKIP(dev-env degrade);`RURIX_REQUIRE_REAL=1` 缺设备翻硬红。
  - **内建 red_self_test(退出码判定,自反证非工作树改)**:经 `examples/emit_spirv_provenance`
    (provenance=true,DXIL 路,带 SPV_GOOGLE)对同源产**带保名** `.spv` → 喂同 `vk_triangle`
    管线 → `VK_LAYER_KHRONOS_validation` 报 VUID-VkShaderModuleCreateInfo-pCode-08742,
    `run_graphics_offscreen` 的 debug messenger fail-closed 翻 `Err` → **vk_triangle 退出非 0**
    (证方案 B 前坑真实);且 `spirv-val` 仍**接受**带保名变体(证修复是「去装饰」非「产非法
    SPIR-V」——validation-vs-runtime 诚实性)。
退出码判定(非 grep stdout 充绿——校验正确性经 demo 的 `VK_TRIANGLE: ok` + exit 0 双判)。
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

# 无设备(SKIP)信号:vk_triangle / run_graphics_offscreen 缺 Vulkan 运行时的确定性 Err 串。
NO_DEVICE_KEYS = (
    "vulkan-1.dll",
    "libvulkan",
    "vkGetInstanceProcAddr",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
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
        print(f"[vulkan_graphics] FAIL cargo build -p {pkg} {args}:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
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


def spirv_google_count(tool, spv_path):
    """spirv-dis 反汇编中 SPV_GOOGLE / UserSemantic 出现次数(方案 B:vulkan 变体应 0)。"""
    r = subprocess.run([tool, str(spv_path)], cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        return None
    text = r.stdout
    return text.count("SPV_GOOGLE") + text.count("UserSemantic")


def run_triangle(vk_triangle, vs_spv, fs_spv):
    env = dict(os.environ, RURIX_VK_VALIDATION="1")
    r = subprocess.run(
        [str(vk_triangle), str(vs_spv), str(fs_spv)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
    )
    return r


def is_no_device(text):
    return any(k in text for k in NO_DEVICE_KEYS)


def main():
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"

    build("rurixc", "--features", "vulkan-backend")
    # emit_spirv_provenance 走 emit_spirv_body(DXIL 路 provenance=true)→ required-features
    # = dxil-backend + shader-stages(G3.3 typeck 接线补齐 example required-features 后,
    # 仅传 vulkan-backend 不满足);vulkan-backend 一并带上(red_self_test 语义不变)。
    build(
        "rurixc",
        "--features",
        "vulkan-backend dxil-backend shader-stages",
        "--example",
        "emit_spirv_provenance",
    )
    build("rurix-rt", "--features", "vulkan", "--bin", "vk_triangle")

    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    vk_triangle = ROOT / "target" / "debug" / f"vk_triangle{EXE_SUFFIX}"
    example_exe = ROOT / "target" / "debug" / "examples" / f"emit_spirv_provenance{EXE_SUFFIX}"
    for p, name in ((rurixc, "rurixc"), (vk_triangle, "vk_triangle"), (example_exe, "emit_spirv_provenance")):
        check(p.is_file(), f"产物缺失: {name} ({p})")
    check(VS_RX.is_file() and FS_RX.is_file(), f"conformance 语料缺失: {VS_RX} / {FS_RX}")
    if FAILURES:
        return _report(False, False, False)

    spirv_val = locate("spirv-val", "RURIX_SPIRV_VAL")
    spirv_dis = locate("spirv-dis", "RURIX_SPIRV_DIS")
    if (spirv_val is None or spirv_dis is None) and require_real:
        print("[vulkan_graphics] FAIL RURIX_REQUIRE_REAL=1 但 spirv-val/spirv-dis 缺失(GPU runner 应有 Vulkan SDK)", file=sys.stderr)
        return 1
    if spirv_val is None:
        note("spirv-val 不可用:校验段 SKIP(开发环境降级)")
    if spirv_dis is None:
        note("spirv-dis 不可用:SPV_GOOGLE 反证段 SKIP(开发环境降级)")

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

        # ── spirv-val:两 vulkan 变体接受 ──
        if spirv_val is not None:
            for spv, label in ((vs, "vk_tri_vs"), (fs, "vk_tri_fs")):
                if spv.is_file():
                    check(spirv_val_accepts(spirv_val, spv), f"{label}: spirv-val vulkan1.0 拒绝合法 vulkan 变体")
            # 反证 provenance 变体**也**被 spirv-val 接受(修复=去装饰,非产非法 SPIR-V)。
            for spv, label in ((vs_prov, "vk_tri_vs"), (fs_prov, "vk_tri_fs")):
                if spv.is_file():
                    check(
                        spirv_val_accepts(spirv_val, spv),
                        f"{label}(provenance): spirv-val 应接受带保名变体(证修复非产非法 SPIR-V)",
                    )

        # ── spirv-dis:vulkan 变体无 SPV_GOOGLE;provenance 变体有(方案 B 生效反证)──
        if spirv_dis is not None:
            for spv, label in ((vs, "vk_tri_vs"), (fs, "vk_tri_fs")):
                if spv.is_file():
                    n = spirv_google_count(spirv_dis, spv)
                    check(n == 0, f"{label}: vulkan 变体应无 SPV_GOOGLE/UserSemantic 却见 {n}(方案 B 未生效)")
            n_prov = spirv_google_count(spirv_dis, vs_prov) if vs_prov.is_file() else None
            check(
                n_prov is not None and n_prov > 0,
                f"provenance 变体应含 SPV_GOOGLE/UserSemantic(red_self_test 源无效),实得 {n_prov}",
            )

        # ── device 真跑(good 路)+ red_self_test(provenance 路)──
        device_ran = False
        good = run_triangle(vk_triangle, vs, fs)
        good_out = good.stdout + good.stderr
        if is_no_device(good_out):
            if require_real:
                print(f"[vulkan_graphics] FAIL RURIX_REQUIRE_REAL=1 但无 Vulkan 设备:\n{good.stderr}", file=sys.stderr)
                return 1
            note(f"无 Vulkan 设备:device 真跑段 + red_self_test SKIP(dev-env degrade):{good.stderr.strip()}")
        else:
            device_ran = True
            # good 路:exit 0 + VK_TRIANGLE: ok + validation 静默。
            check(good.returncode == 0, f"good 路 vk_triangle 应退出 0 却 {good.returncode}\nstdout={good.stdout}\nstderr={good.stderr}")
            check("VK_TRIANGLE: ok" in good.stdout, f"good 路应见 'VK_TRIANGLE: ok':\n{good.stdout}")
            check(
                "Validation Error" not in good.stderr and "VUID-" not in good.stderr,
                f"good 路 validation 应静默却报错:\n{good.stderr}",
            )

            # red_self_test:provenance 变体 → VUID-08742 → 退出码判红(fail-closed）。
            red = run_triangle(vk_triangle, vs_prov, fs_prov)
            red_out = red.stdout + red.stderr
            check(
                red.returncode != 0,
                f"red_self_test: provenance 变体应退出非 0(fail-closed)却 {red.returncode}\nstdout={red.stdout}\nstderr={red.stderr}",
            )
            check(
                "08742" in red_out,
                f"red_self_test: provenance 变体应触 VUID-...-08742 却未见:\nstdout={red.stdout}\nstderr={red.stderr}",
            )
            check(
                "VK_TRIANGLE: ok" not in red.stdout,
                f"red_self_test: provenance 变体不应打印 'VK_TRIANGLE: ok'(应 fail-closed 前退):\n{red.stdout}",
            )

    return _report(spirv_val is not None, spirv_dis is not None, device_ran)


def _report(had_val, had_dis, device_ran):
    for m in NOTES:
        print(f"[vulkan_graphics] NOTE {m}")
    if FAILURES:
        print(f"[vulkan_graphics] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    val_seg = "spirv-val✓" if had_val else "spirv-val SKIP"
    dis_seg = "no-SPV_GOOGLE✓" if had_dis else "spirv-dis SKIP"
    dev_seg = "device offscreen✓ + red_self_test(VUID-08742)✓" if device_ran else "device SKIP"
    print(f"[vulkan_graphics] PASS (vk_tri_{{vs,fs}} → SPIR-V,{val_seg},{dis_seg},{dev_seg})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
