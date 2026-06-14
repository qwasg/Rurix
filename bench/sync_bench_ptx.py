"""从 rurix-rt build 产物同步 PTX 到 bench/kernels/(M5.3 review fix)。

用法: py -3 bench/sync_bench_ptx.py
"""
from __future__ import annotations

import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench.protocol import ROOT
from bench.resolve_ptx import resolve_ptx

KERNELS = ("saxpy", "reduce", "scan", "transpose", "gemm_tile")
OUT = ROOT / "bench/kernels"


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for name in KERNELS:
        src = resolve_ptx(name)
        dst_name = f"rurix_{name}.ptx" if name != "saxpy" else "rurix_saxpy.ptx"
        if name == "saxpy":
            dst_name = "rurix_saxpy.ptx"
        else:
            dst_name = f"rurix_{name}.ptx"
        dst = OUT / dst_name
        shutil.copy2(src, dst)
        print(f"[sync] {src} -> {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
