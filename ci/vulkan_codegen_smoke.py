#!/usr/bin/env python3
"""mb1 Vulkan/SPIR-V codegen 冒烟(RXS-0200~0205;RFC-0011)。

`rx build --target vulkan` 对 conformance/vulkan/accept/*.rx 产 SPIR-V,经 spirv-val
校验接受(compute + graphics)。fail-closed 纪律(对齐 RXS-0073 ptxas 干验证 SKIP、
反 Godot 退出码/grep 教训):
  - rurixc(--features vulkan-backend)build 必绿;--target vulkan 对合法语料退出 0 产 .spv。
  - spirv-val 在位 → 每 .spv 经 `spirv-val --target-env vulkan1.0` 接受(严格 Vulkan)。
    spirv-val 缺失 → 校验段 SKIP(开发环境降级,真实红绿在带 Vulkan SDK 环境),非 fake pass。
  - red_self_test(反 YAML-only):F64 子集外 kernel → RX6026(编译期红,恒跑);篡改 .spv
    字节 → spirv-val 拒(需 spirv-val)。
  - 确定性:同源 ×N 编译产 .spv 字节全等。
退出码判定(非 grep stdout)。任一应绿却红 / 应红却绿 → 整体 FAIL(非零退出);无 spirv-val
→ 校验 SKIP exit 0。CI(GPU runner,有 Vulkan SDK)`RURIX_REQUIRE_REAL=1` 时缺工具翻硬红。
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ACCEPT_DIR = ROOT / "conformance" / "vulkan" / "accept"

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond, msg):
    if not cond:
        FAILURES.append(msg)


def note(msg):
    NOTES.append(msg)


def build_rurixc():
    print("[vulkan_codegen] cargo build -p rurixc --features vulkan-backend")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"[vulkan_codegen] FAIL cargo build:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if not exe.is_file():
        print(f"[vulkan_codegen] FAIL rurixc 产物缺失: {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def locate_spirv_val():
    """env RURIX_SPIRV_VAL(.is_file)→ PATH `spirv-val`;缺失 → None(校验 SKIP)。"""
    env = os.environ.get("RURIX_SPIRV_VAL")
    if env and Path(env).is_file():
        return env
    import shutil

    return shutil.which("spirv-val")


def compile_vulkan(exe, rx_path, spv_path):
    """rurixc --target vulkan <rx> -o <spv>;返回 (returncode, stderr)。退出码判定。"""
    r = subprocess.run(
        [str(exe), "--target", "vulkan", str(rx_path), "-o", str(spv_path)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return r.returncode, r.stderr


def spirv_val_accepts(tool, spv_path):
    """spirv-val --target-env vulkan1.0 <spv>;退出码 0 = accept(非 grep stdout)。"""
    r = subprocess.run(
        [tool, "--target-env", "vulkan1.0", str(spv_path)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return r.returncode == 0


def red_self_test(exe, spirv_val):
    """反 YAML-only:合成红输入必红。F64 子集外恒跑;篡改需 spirv-val。"""
    # (1) F64 子集外 → 编译期 RX6026(退出码非 0)。
    f64_src = (
        "kernel fn k(out: ViewMut<global, f64>, t: ThreadCtx<1>) {\n"
        "    let i = t.global_id();\n"
        "    out[i] = 1.0;\n"
        "}\n"
    )
    with tempfile.TemporaryDirectory() as d:
        rx = Path(d) / "f64.rx"
        rx.write_text(f64_src, encoding="utf-8")
        spv = Path(d) / "f64.spv"
        code, stderr = compile_vulkan(exe, rx, spv)
        check(code != 0, "red_self_test: F64 子集外 kernel 应红(RX6026)却退出 0(反 YAML-only)")
        check("RX6026" in stderr, f"red_self_test: F64 应发 RX6026,stderr 未见:\n{stderr}")

    # (2) 篡改 .spv 字节 → spirv-val 拒(需 spirv-val)。
    if spirv_val is None:
        note("red_self_test: spirv-val 缺失,篡改红绿段 SKIP(开发环境降级)")
        return
    noop = ACCEPT_DIR / "vk_noop.rx"
    with tempfile.TemporaryDirectory() as d:
        spv = Path(d) / "noop.spv"
        code, _ = compile_vulkan(exe, noop, spv)
        check(code == 0, "red_self_test: vk_noop 应绿却红")
        if code == 0 and spv.is_file():
            check(
                spirv_val_accepts(spirv_val, spv),
                "red_self_test: 未篡改 vk_noop.spv 应 spirv-val 接受",
            )
            # 篡改指令流一字节。
            b = bytearray(spv.read_bytes())
            if len(b) > 24:
                b[20] ^= 0xFF
                tampered = Path(d) / "tampered.spv"
                tampered.write_bytes(bytes(b))
                check(
                    not spirv_val_accepts(spirv_val, tampered),
                    "red_self_test: 篡改 .spv 应被 spirv-val 拒却接受(反 YAML-only)",
                )


def main():
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
    exe = build_rurixc()
    spirv_val = locate_spirv_val()
    if spirv_val is None and require_real:
        print(
            "[vulkan_codegen] FAIL RURIX_REQUIRE_REAL=1 但 spirv-val 缺失(GPU runner 应有 Vulkan SDK)",
            file=sys.stderr,
        )
        return 1
    if spirv_val is None:
        note("spirv-val 不可用:校验段 SKIP(开发环境降级,真实红绿在带 Vulkan SDK 环境)")

    red_self_test(exe, spirv_val)

    cases = sorted(ACCEPT_DIR.glob("*.rx"))
    check(len(cases) > 0, f"conformance/vulkan/accept 无语料: {ACCEPT_DIR}")
    validated = 0
    for rx in cases:
        with tempfile.TemporaryDirectory() as d:
            spv1 = Path(d) / (rx.stem + "_1.spv")
            spv2 = Path(d) / (rx.stem + "_2.spv")
            code1, stderr1 = compile_vulkan(exe, rx, spv1)
            check(code1 == 0, f"{rx.name}: --target vulkan 应绿(0)却退出 {code1}\n{stderr1}")
            if code1 != 0:
                continue
            check(spv1.is_file(), f"{rx.name}: .spv 未产出")
            # 确定性:同源 ×2 字节全等。
            code2, _ = compile_vulkan(exe, rx, spv2)
            if code2 == 0 and spv1.is_file() and spv2.is_file():
                check(
                    spv1.read_bytes() == spv2.read_bytes(),
                    f"{rx.name}: 同源 ×2 编译 .spv 字节不等(非确定性)",
                )
            # spirv-val 严格校验(在位时)。
            if spirv_val is not None and spv1.is_file():
                check(
                    spirv_val_accepts(spirv_val, spv1),
                    f"{rx.name}: spirv-val --target-env vulkan1.0 拒绝合法产物",
                )
                validated += 1

    return _report(len(cases), validated, spirv_val is not None)


def _report(n_cases, validated, had_val):
    for m in NOTES:
        print(f"[vulkan_codegen] NOTE {m}")
    if FAILURES:
        print(f"[vulkan_codegen] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    seg = f"{validated}/{n_cases} spirv-val vulkan1.0" if had_val else "spirv-val SKIP"
    print(f"[vulkan_codegen] PASS ({n_cases} 语料 --target vulkan 产 SPIR-V,{seg})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
