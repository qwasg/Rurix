#!/usr/bin/env python3
"""mb1 Android 交叉构建冒烟(RXS-0211;RFC-0011 §4)。两阶段,退出码判定:

阶段 1 — host 平台无关单测(**任意 runner 恒跑,无 NDK 依赖**):
  `cargo test -p rurix-rt --features vulkan --lib` 跑 RXS-0211 锚点
  `loader_seam_selects_platform_lib`(加载缝库名 per-OS 唯一)+ `entry_point_name`
  等平台无关单测(纯 host、不触设备)。失败 → **无条件硬红**(平台无关,无 dev-env 借口)。
  这是 CI_GATES §2.57 gate face「平台无关单测」的兑现,任意 runner(含无 NDK)恒执行。

阶段 2 — NDK 交叉构建:NDK + aarch64-linux-android target 在位 → cross-build
  `rurix-rt --features vulkan`(lib+bin)链接绿;缺 → SKIP(dev-env 降级,非 fake)。
  RURIX_REQUIRE_ANDROID=1(专用 android-build runner)时缺 NDK/target 翻硬红。

**不触设备**(on-device saxpy/present = G-MB1-7 open,不在本门)。
"""
import os, re, subprocess, sys, glob
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = "aarch64-linux-android"
API = "26"


def host_unit_tests():
    """阶段 1:平台无关单测(loader_seam + entry_point_name;无 NDK 依赖,恒可跑)。

    返回 True=绿 / False=红。vulkan feature 仅动态加载 OS loader 符号(LoadLibraryA /
    dlopen 恒在位),host 侧编译不需 Vulkan SDK/NDK;单测全 device-free。
    """
    r = subprocess.run(
        ["cargo", "test", "-p", "rurix-rt", "--features", "vulkan", "--lib"],
        cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        print(f"[vk_android] FAIL host 平台无关单测(vulkan --lib):\n{r.stdout}\n{r.stderr}",
              file=sys.stderr)
        return False
    m = re.search(r"(\d+) passed", r.stdout)
    n = m.group(1) if m else "?"
    print(f"[vk_android] host 平台无关单测 PASS ({n}) "
          f"(loader_seam_selects_platform_lib + entry_point_name;纯 host,无 NDK 依赖)")
    return True


def find_ndk():
    for k in ("ANDROID_NDK_HOME", "ANDROID_NDK_ROOT"):
        v = os.environ.get(k)
        if v and Path(v).is_dir():
            return Path(v)
    sdk = os.environ.get("ANDROID_HOME") or os.environ.get("ANDROID_SDK_ROOT")
    if sdk:
        cands = sorted(glob.glob(str(Path(sdk) / "ndk" / "*")))
        if cands:
            return Path(cands[-1])
    return None


def has_target():
    r = subprocess.run(["rustup", "target", "list", "--installed"],
                       capture_output=True, text=True)
    return TARGET in r.stdout


def clang_bin(ndk):
    host = "windows-x86_64" if sys.platform == "win32" else \
           ("darwin-x86_64" if sys.platform == "darwin" else "linux-x86_64")
    bindir = ndk / "toolchains" / "llvm" / "prebuilt" / host / "bin"
    ext = ".cmd" if sys.platform == "win32" else ""
    linker = bindir / f"{TARGET}{API}-clang{ext}"
    ar_ext = ".exe" if sys.platform == "win32" else ""
    ar = bindir / f"llvm-ar{ar_ext}"
    return bindir, linker, ar


def cross_build():
    """阶段 2:NDK 交叉构建(在位 → 链接绿;缺 → SKIP;REQUIRE 翻红)。退出码。"""
    require = os.environ.get("RURIX_REQUIRE_ANDROID") == "1"
    ndk, tgt = find_ndk(), has_target()
    if not ndk or not tgt:
        msg = f"NDK={'ok' if ndk else 'missing'} target={'ok' if tgt else 'missing'}"
        if require:
            print(f"[vk_android] FAIL 要求 android 构建但环境缺:{msg}", file=sys.stderr)
            return 1
        print(f"[vk_android] SKIP dev-env 降级(非 fake)cross-build:{msg}")
        return 0
    bindir, linker, ar = clang_bin(ndk)
    if not linker.is_file():
        print(f"[vk_android] {'FAIL' if require else 'SKIP'} NDK clang 缺: {linker}",
              file=sys.stderr)
        return 1 if require else 0
    env = dict(os.environ)
    env["PATH"] = str(bindir) + os.pathsep + env.get("PATH", "")
    env["CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"] = str(linker)
    env["CARGO_TARGET_AARCH64_LINUX_ANDROID_AR"] = str(ar)
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan",
         "--target", TARGET, "--quiet"],
        cwd=ROOT, env=env, capture_output=True, text=True)
    if r.returncode != 0:
        print(f"[vk_android] FAIL cross-build 交叉 build:\n{r.stdout}\n{r.stderr}",
              file=sys.stderr)
        return 1
    print(f"[vk_android] PASS cross-build {TARGET} 链接绿(lib+vk_saxpy;on-device=G-MB1-7 open)")
    return 0


def main():
    # 阶段 1:host 平台无关单测——任意 runner 恒跑,失败即硬红(无 dev-env 借口)。
    if not host_unit_tests():
        return 1
    # 阶段 2:NDK 交叉构建——在位则链接绿,缺则 SKIP dev-env 降级。
    return cross_build()


if __name__ == "__main__":
    sys.exit(main())
