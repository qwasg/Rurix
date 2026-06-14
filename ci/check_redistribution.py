"""NVIDIA 再分发面守卫(M5.4 第 5 步;CI_GATES.md §4 第 2 项 formal 激活)。

check_* 守卫风格(不分配错误码,07 §5):机器复核 rurix 分发产物的
"NVIDIA libdevice 再分发面为空"这一事实,作为 CI_GATES §4 第 2 项白名单审计
结论(M5.3 草拟 → formal 激活)的可机器复核闸门。CPU-only,不依赖 GPU/clang。

两项断言:
1. 版本化嵌入 PTX(bench/kernels/rurix_*.ptx,经 check_bench_ptx_sync.py 与
   rurix-rt build 产物哈希同步)不得含 `__nv_*` libdevice 派生符号 —— internalize+DCE
   后无残留外部 libdevice 调用,再分发面为空;
2. rurixc/rurix-rt 源不得把 libdevice bitcode(libdevice*.bc)经 include_bytes!/
   include_str! 打包进任何产物 —— libdevice.10.bc 运行期经 CUDA_PATH 定位
   (toolchain::locate_libdevice),不入产物。

用法: py -3 ci/check_redistribution.py
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# libdevice 派生符号前缀(NVPTX 后端为未解析的 libdevice 数学函数发出 __nv_* 调用)。
NV_SYMBOL = "__nv_"

# include_bytes!/include_str! 把 .bc(libdevice bitcode)打包进产物的形态。
# 运行期定位(locate_libdevice 拼路径)、注释、诊断提示中的字面量不算打包。
EMBED_BC_RE = re.compile(
    r"include_(?:bytes|str)!\s*\(\s*[^)]*\.bc",
    re.IGNORECASE,
)


def check_embedded_ptx() -> list[str]:
    """断言 1:版本化嵌入 PTX 无 __nv_* 符号。"""
    failures: list[str] = []
    ptx_dir = ROOT / "bench" / "kernels"
    ptx_files = sorted(ptx_dir.glob("rurix_*.ptx"))
    if not ptx_files:
        failures.append(
            f"未发现 {ptx_dir.relative_to(ROOT)}/rurix_*.ptx(版本化嵌入 PTX 缺失,"
            "无法核验再分发面)"
        )
        return failures
    for f in ptx_files:
        text = f.read_text(encoding="utf-8")
        hits = [
            f"  {f.relative_to(ROOT)}:{ln}: {line.strip()}"
            for ln, line in enumerate(text.splitlines(), start=1)
            if NV_SYMBOL in line
        ]
        if hits:
            failures.append(
                f"{f.relative_to(ROOT)}: 嵌入 PTX 含 {NV_SYMBOL}* libdevice 派生符号"
                "(再分发面非空,需白名单逐项核对):\n" + "\n".join(hits)
            )
    return failures


def check_bc_not_packaged() -> list[str]:
    """断言 2:源不把 libdevice .bc 经 include_* 打包进产物。"""
    failures: list[str] = []
    for crate in ("src/rurixc", "src/rurix-rt"):
        crate_dir = ROOT / crate
        if not crate_dir.is_dir():
            continue
        for rs in sorted(crate_dir.rglob("*.rs")):
            text = rs.read_text(encoding="utf-8")
            for ln, line in enumerate(text.splitlines(), start=1):
                if EMBED_BC_RE.search(line):
                    failures.append(
                        f"{rs.relative_to(ROOT)}:{ln}: 检测到把 .bc 打包进产物的形态"
                        "(libdevice bitcode 必须运行期定位,禁打包):\n"
                        f"  {line.strip()}"
                    )
    return failures


def main() -> int:
    failures: list[str] = []
    failures += check_embedded_ptx()
    failures += check_bc_not_packaged()
    if failures:
        print("[check_redistribution] FAIL — NVIDIA 再分发面非空/libdevice 被打包:")
        for f in failures:
            print(f"  - {f}")
        return 1
    print(
        "[check_redistribution] PASS — 版本化嵌入 PTX 无 __nv_* 符号、"
        "源无 libdevice .bc 打包(再分发面为空)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
