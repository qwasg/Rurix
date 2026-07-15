#!/usr/bin/env python3
"""mb1 Vulkan compute 设备真跑冒烟(RXS-0207;RFC-0011 §4.6)。

`rurixc --target vulkan saxpy.rx` 产 SPIR-V → `bin/vk_saxpy` 在本机 Vulkan 设备
(NVIDIA / AMD 桌面 / lavapipe)真跑 saxpy = a*x + out,回读数值精确校验 + validation 零报错。
fail-closed:无 Vulkan 设备 → SKIP(开发环境降级,非 fake pass);`RURIX_REQUIRE_REAL=1`
(GPU runner)时缺设备翻硬红。退出码判定(非 grep stdout 充绿——但校验数值正确性经 demo 的
`VK_SAXPY: ok` + exit 0 双判)。

第二 ICD(lavapipe / 软件光栅)跨厂商数值回归阶段(设计 artifact-gen-lavapipe.md §3):
primary NVIDIA(系统 ICD)pass 成功后,若经 `RURIX_VK_LAVAPIPE_ICD`(或已指向软件 ICD json 的
`VK_DRIVER_FILES`)发现 lavapipe ICD manifest,则**仅向该 subprocess env 副本**注入
`VK_DRIVER_FILES=<lvp_icd.json>` 再跑同一 `vk_saxpy` 同一 `.spv`,断言其数值(out[*]/max_err)
与 primary 一致(saxpy 确定性)且 validation 静默 → 证 SPIR-V 跨非-NVIDIA 驱动可消费且数值回归。
不一致/validation 报错 → 硬红。**绝不下载/抓取二进制**;缺 ICD → `SKIP: second ICD unavailable
(dev-env degrade)` 且不判红(承既有缺工具 SKIP 纪律,非 fake)。**honesty 关键:primary env 副本
显式剥除 `VK_DRIVER_FILES`/`VK_ICD_FILENAMES`**(loader 视其为 EXCLUSIVE 覆盖),故即便操作员用
`VK_DRIVER_FILES` 指向 lavapipe 触发第二 ICD,primary 仍跑系统/默认 ICD → 第二遍才是真跨厂商、非
软件-vs-软件假绿,NVIDIA 零回归。软件 ICD 跑通 ≠ AMD/Android 已验证(G-MB1-6/7 尾门不受影响)。
"""

import os
import re
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


def parse_saxpy_numbers(stdout):
    """从 vk_saxpy stdout 提取跨 ICD 数值回归比较键:out[i] 与 max_err。

    输出行形如 `VK_SAXPY: ok ... out[0]=<v> out[1]=<v> out[1023]=<v> max_err=<v>`。
    返回 {字段名: 字面量串},缺字段则不在 dict 中(调用方判 None)。
    """
    keys = {}
    for m in re.finditer(r"(out\[\d+\]|max_err)=([-+0-9.eE]+)", stdout):
        keys[m.group(1)] = m.group(2)
    return keys


def discover_lavapipe_icd():
    """定位软件(lavapipe/SwiftShader)ICD manifest —— 仅从环境读取,绝不下载/抓取(设计 §3.1/§3.2)。

    优先 `RURIX_VK_LAVAPIPE_ICD`(直指 `lvp_icd*.json` 绝对路径);其次若 `VK_DRIVER_FILES`
    已指向某个存在的软件 ICD json(名含 lvp/lavapipe/swiftshader)也接受。返回存在的 Path 或 None。
    """
    cand = os.environ.get("RURIX_VK_LAVAPIPE_ICD", "").strip()
    if cand and Path(cand).is_file():
        return Path(cand)
    drv = os.environ.get("VK_DRIVER_FILES", "").strip()
    if drv:
        # VK_DRIVER_FILES 可含多路径(os.pathsep 分隔);取首个存在且名指软件 ICD 的 manifest。
        for p in drv.split(os.pathsep):
            p = p.strip()
            if not p or not Path(p).is_file():
                continue
            name = Path(p).name.lower()
            if any(t in name for t in ("lvp", "lavapipe", "swiftshader")):
                return Path(p)
    return None


def run_second_icd(vk_saxpy, spv, primary_stdout, icd):
    """第二 ICD(lavapipe/软件)跨厂商数值回归:同一 `vk_saxpy` 跑同一 `.spv`,
    **仅**向本 subprocess env 副本注入 `VK_DRIVER_FILES=<icd>`(不污染 primary NVIDIA pass 的
    系统-ICD env),断言其数值(out[*] 与 max_err)与 primary 一致(saxpy 确定性;a/x/out 皆精确
    可表 → IEEE-754 float32 逐位一致,容差极小 ≤1e-3 承 demo 自校)且 validation 静默。返回退出码。
    """
    env = dict(os.environ, RURIX_VK_VALIDATION="1")
    env["VK_DRIVER_FILES"] = str(icd)  # loader ≥1.3.207 首选;覆盖系统 ICD 发现(§3.1)
    env["VK_ICD_FILENAMES"] = str(icd)  # 旧 loader 同义兜底
    run = subprocess.run(
        [str(vk_saxpy), str(spv)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
    )
    ok = run.returncode == 0 and "VK_SAXPY: ok" in run.stdout
    if not ok:
        print(
            f"[vulkan_device] FAIL 2nd ICD (lavapipe) 真跑失败(exit={run.returncode}):\n"
            f"stdout={run.stdout}\nstderr={run.stderr}",
            file=sys.stderr,
        )
        return 1
    if "Validation Error" in run.stderr or "VUID-" in run.stderr:
        print(
            f"[vulkan_device] FAIL 2nd ICD (lavapipe) validation layer 报错:\n{run.stderr}",
            file=sys.stderr,
        )
        return 1
    primary = parse_saxpy_numbers(primary_stdout)
    second = parse_saxpy_numbers(run.stdout)
    for k in ("out[0]", "out[1]", "out[1023]", "max_err"):
        pv, sv = primary.get(k), second.get(k)
        if pv is None or sv is None or abs(float(pv) - float(sv)) > 1e-3:
            print(
                f"[vulkan_device] FAIL 2nd ICD (lavapipe) 跨厂商数值不符 {k}: "
                f"primary={pv} second={sv}",
                file=sys.stderr,
            )
            return 1
    print(f"[vulkan_device] 2nd ICD (lavapipe) PASS cross-vendor numeric match: {run.stdout.strip()}")
    return 0


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
        # honesty:primary pass 必须跑 loader-default(系统)ICD,与 ambient 配置无关。Vulkan loader
        # 把 VK_DRIVER_FILES/VK_ICD_FILENAMES 当 EXCLUSIVE 覆盖 —— 若操作员用其指向 lavapipe 触发第二
        # ICD(见 discover_lavapipe_icd),不剥离则 primary 也会跑软件 ICD → run_second_icd 变
        # lavapipe-vs-lavapipe 假绿。故 primary env 副本剥掉这两个覆盖,保证 primary=系统/默认 ICD。
        env.pop("VK_DRIVER_FILES", None)
        env.pop("VK_ICD_FILENAMES", None)
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
            # 第二 ICD(lavapipe/软件)跨厂商数值回归阶段 —— 加性,primary NVIDIA pass 已定绿。
            icd = discover_lavapipe_icd()
            if icd is None:
                # 本机无软件 ICD(获取留 follow-up,绝不下载)→ 诚实 SKIP,不判红;primary 结果作数。
                print("[vulkan_device] SKIP: second ICD unavailable (dev-env degrade)")
                return 0
            print(f"[vulkan_device] 2nd ICD (lavapipe) manifest: {icd}")
            return run_second_icd(vk_saxpy, spv, run.stdout, icd)

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
