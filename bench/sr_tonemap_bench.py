"""Rurix 软光栅 sr_tonemap 基准 / 冒烟 harness(M7.3,D-M7-3;spec/softraster.md RXS-0121)。

kernel 来自 rurixc device codegen:`src/rurix-rt/kernels/sr_tonemap.rx`(HDR→LDR 像素
量化,clamp[0,1]+NaN→0+floor(c*255+0.5) 半值向上,as usize 截断 floor,不依赖 libdevice)。
全 safe、atomics-free。每分量 owner 线程独写。

用法:
  py -3 bench/sr_tonemap_bench.py --smoke   # 装载 + 执行 + 正确性(Compute Sanitizer nightly 包裹)

L3 规模协议化采样(m7.bench.soft_raster_l3_frame_ms,G-M7-2)由 M7.5 回填。
"""
from __future__ import annotations

import ctypes
import math
import re
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import cuda_driver as cu
from bench.resolve_ptx import read_ptx

N_SMOKE = 4096
BLOCK = 256
KERNEL = "sr_tonemap"


def parse_entry(ptx: str, path: Path) -> str:
    m = re.search(r"\.entry\s+([A-Za-z_$][A-Za-z0-9_$]*)", ptx)
    if not m:
        raise RuntimeError(f"无法从 {path} 解析 .entry kernel 名")
    return m.group(1)


def quantize_ref(c: float) -> float:
    x = 0.0 if math.isnan(c) else min(max(c, 0.0), 1.0)
    return float(int(x * 255.0 + 0.5))  # as usize 截断 = floor(非负)


def main() -> int:
    if "--smoke" not in sys.argv:
        print("[rurix-sr_tonemap] 仅 --smoke 已实现;L3 协议化采样(G-M7-2)由 M7.5 回填")
        return 0
    n = N_SMOKE
    nblocks = (n + BLOCK - 1) // BLOCK

    ptx_path, ptx = read_ptx(KERNEL)
    entry = parse_entry(ptx, ptx_path)
    with cu.Context():
        module, version, _ = cu.load_ptx(ptx)
        print(f"[rurix-sr_tonemap] PTX loaded (.version {version}, entry {entry})")
        fn = cu.get_function(module, entry)

        rng = np.random.default_rng(20260616)
        hdr = (rng.standard_normal(n).astype(np.float32) * np.float32(0.6) + np.float32(0.5))
        ldr = np.zeros(n, dtype=np.float32)
        d_hdr = cu.mem_alloc(n * 4)
        d_ldr = cu.mem_alloc(n * 4)
        cu.memcpy_htod(d_hdr, hdr.ctypes.data, n * 4)

        args = [
            ctypes.c_uint64(d_hdr.value),
            ctypes.c_uint64(d_ldr.value),
            ctypes.c_uint64(n),
        ]
        cu.launch(fn, (nblocks, 1, 1), (BLOCK, 1, 1), args)
        cu.stream_sync()
        cu.memcpy_dtoh(ldr.ctypes.data, d_ldr, n * 4)

        ref = np.array([quantize_ref(float(v)) for v in hdr], dtype=np.float32)
        if not np.array_equal(ldr, ref):
            bad = int(np.argmax(ldr != ref))
            raise AssertionError(
                f"correctness FAIL: idx {bad} got {ldr[bad]} expect {ref[bad]} (hdr={hdr[bad]})"
            )
        print(f"[rurix-sr_tonemap] correctness PASS (n={n}, 量化逐分量逐字节一致)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
