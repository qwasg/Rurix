"""比对 bench/kernels 与 rurix-rt build 嵌入 PTX 哈希(M5.3 review fix)。

用法: py -3 ci/check_bench_ptx_sync.py
需先 `cargo build -p rurix-rt`(有 clang 时产出非空 PTX)。
"""
from __future__ import annotations

import hashlib
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from bench.resolve_ptx import resolve_ptx

KERNELS = ("reduce", "scan", "transpose", "gemm_tile")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    mismatches: list[str] = []
    for name in KERNELS:
        try:
            build_ptx = resolve_ptx(name)
        except FileNotFoundError as e:
            print(f"SKIP {name}: {e}")
            continue
        if build_ptx.read_text(encoding="utf-8").strip() == "":
            print(f"SKIP {name}: build PTX 为空(降级)")
            continue
        bench_ptx = ROOT / "bench/kernels" / f"rurix_{name}.ptx"
        if not bench_ptx.is_file():
            mismatches.append(f"{name}: 缺 {bench_ptx.relative_to(ROOT)}")
            continue
        if sha256(build_ptx) != sha256(bench_ptx):
            mismatches.append(
                f"{name}: bench/kernels 与 build 产物不一致 "
                f"(运行 py -3 bench/sync_bench_ptx.py)"
            )
            continue
        # cubin 形态结构核对(G1.5,RXS-0150):cubin 由**已 bless 的 PTX** 经 `ptxas -arch=sm_89`
        # 预编(build.rs)。cubin 字节随 ptxas 版号绑定不确定(G1_PLAN §7),故**不设字节级 golden**
        # ——PTX `.nvptx` 文本 golden 维持唯一确定性 bless 门(M4.2);cubin 以**结构核对**纳入:
        # 存在的预编 cubin 须为有效 ELF(捕获 cubin↔PTX 形态漂移 / 损坏)。
        cubin = build_ptx.parent / f"{name}.sm_89.cubin"
        if cubin.is_file() and cubin.stat().st_size > 0 and cubin.read_bytes()[:4] != b"\x7fELF":
            mismatches.append(
                f"{name}: 预编 cubin 非有效 ELF(cubin↔PTX 形态漂移 / 损坏,RXS-0150)"
            )
    if mismatches:
        print("bench PTX 漂移:")
        for m in mismatches:
            print(f"  - {m}")
        return 1
    print("bench PTX 与 build 产物一致(SKIP 项已忽略)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
