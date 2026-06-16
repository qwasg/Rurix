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

# include_bytes!/include_str! 把 NVIDIA runtime DLL / 导入库打包进产物的形态
# (cublas runtime DLL 按需附带须经 Attachment A 白名单 + M8.4 发布链路分离打包,
# M8.2 仅动态加载系统 DLL,不入产物;许可红线 r6,RXS-0129)。
EMBED_NV_LIB_RE = re.compile(
    r"include_(?:bytes|str)!\s*\(\s*[^)]*\.(?:dll|lib)",
    re.IGNORECASE,
)

# cublas runtime DLL 动态加载候选名 Attachment A 白名单形态(cublas64_<ver>.dll /
# cublasLt64_<ver>.dll 运行期库;完整 Toolkit/驱动/Nsight/静态库永不捆绑)。
CUBLAS_WHITELIST_RE = re.compile(r"^cublas(?:Lt)?64_\d+\.dll$")
# sys.rs 中 c"...dll" 字面量(动态加载候选)。
DLL_LITERAL_RE = re.compile(r'c"([^"]+\.dll)"', re.IGNORECASE)


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


def check_cublas_runtime_not_bundled() -> list[str]:
    """断言 3(M8.2,RXS-0129):cublas runtime DLL 不入产物 + 动态加载候选名限
    Attachment A 白名单(cublas64_*.dll / cublasLt64_*.dll;完整 Toolkit/驱动/Nsight
    /静态库永不捆绑,许可红线 r6)。M8.2 仅动态加载系统 DLL,物理捆绑/再分发承接 M8.4。"""
    failures: list[str] = []
    cublas_dir = ROOT / "src" / "rurix-cublas"
    if not cublas_dir.is_dir():
        return failures
    # 3a:rurix-cublas 源不得把 DLL/导入库经 include_* 打包进产物。
    for rs in sorted(cublas_dir.rglob("*.rs")):
        text = rs.read_text(encoding="utf-8")
        for ln, line in enumerate(text.splitlines(), start=1):
            if EMBED_NV_LIB_RE.search(line):
                failures.append(
                    f"{rs.relative_to(ROOT)}:{ln}: 检测到把 .dll/.lib 打包进产物的形态"
                    "(cublas runtime DLL 须运行期动态加载,禁打包;r6,RXS-0129):\n"
                    f"  {line.strip()}"
                )
    # 3b:仓库不得提交 cublas runtime DLL / 导入库二进制(应来自系统 CUDA Toolkit)。
    for pat in ("**/cublas*.dll", "**/cublas*.lib"):
        for f in sorted(ROOT.glob(pat)):
            if "target" in f.parts:
                continue  # 构建产物目录不计入源树审计
            failures.append(
                f"{f.relative_to(ROOT)}: 仓库提交了 cublas runtime DLL/导入库"
                "(完整 Toolkit/再分发组件永不入源树,r6;M8.2 链接系统 DLL)"
            )
    # 3c:sys.rs 动态加载候选名全部匹配 Attachment A 白名单形态。
    sys_rs = cublas_dir / "src" / "sys.rs"
    if sys_rs.is_file():
        text = sys_rs.read_text(encoding="utf-8")
        # 仅核对 CUBLAS_DLL_CANDIDATES 数组区域(动态加载候选),避免误伤注释中的 nvcuda.dll 引用。
        start = text.find("CUBLAS_DLL_CANDIDATES")
        region = text[start : text.find("];", start) + 2] if start != -1 else ""
        cands = DLL_LITERAL_RE.findall(region)
        if not cands:
            failures.append(
                "src/rurix-cublas/src/sys.rs: 未定位 CUBLAS_DLL_CANDIDATES 动态加载候选名"
                "(无法核验 Attachment A 白名单,RXS-0129)"
            )
        for name in cands:
            if not CUBLAS_WHITELIST_RE.match(name):
                failures.append(
                    f"src/rurix-cublas/src/sys.rs: 动态加载候选 {name} 不在 Attachment A "
                    "白名单形态(仅 cublas64_*.dll / cublasLt64_*.dll 运行期库,r6,RXS-0129)"
                )
    return failures


def main() -> int:
    failures: list[str] = []
    failures += check_embedded_ptx()
    failures += check_bc_not_packaged()
    failures += check_cublas_runtime_not_bundled()
    if failures:
        print("[check_redistribution] FAIL — NVIDIA 再分发面非空/libdevice 被打包:")
        for f in failures:
            print(f"  - {f}")
        return 1
    print(
        "[check_redistribution] PASS — 版本化嵌入 PTX 无 __nv_* 符号、"
        "源无 libdevice .bc 打包、cublas runtime DLL 不入产物且动态加载候选限 "
        "Attachment A 白名单(再分发面为空)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
