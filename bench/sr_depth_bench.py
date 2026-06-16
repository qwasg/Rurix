"""Rurix 软光栅 sr_depth 基准 / 冒烟 harness(M7.3,D-M7-3;spec/softraster.md RXS-0120)。

kernel 来自 rurixc device codegen:`src/rurix-rt/kernels/sr_depth.rx`(z-buffer 写入 +
less 深度测试,每像素 owner 按固定片元序串行合成,相等不覆盖,atomics-free、确定性)。

用法:
  py -3 bench/sr_depth_bench.py --smoke   # 装载 + 执行 + 正确性(Compute Sanitizer nightly 包裹)

L3 规模协议化采样(m7.bench.soft_raster_l3_frame_ms,G-M7-2)由 M7.5 回填。
"""
from __future__ import annotations

import ctypes
import re
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import cuda_driver as cu
from bench.resolve_ptx import read_ptx

NPIX = 1024
FRAGS = 4
BLOCK = 256
KERNEL = "sr_depth"
Z_FAR = 1.0e30


def parse_entry(ptx: str, path: Path) -> str:
    m = re.search(r"\.entry\s+([A-Za-z_$][A-Za-z0-9_$]*)", ptx)
    if not m:
        raise RuntimeError(f"无法从 {path} 解析 .entry kernel 名")
    return m.group(1)


def main() -> int:
    if "--smoke" not in sys.argv:
        print("[rurix-sr_depth] 仅 --smoke 已实现;L3 协议化采样(G-M7-2)由 M7.5 回填")
        return 0
    npix = NPIX
    frags = FRAGS
    nblocks = (npix + BLOCK - 1) // BLOCK

    ptx_path, ptx = read_ptx(KERNEL)
    entry = parse_entry(ptx, ptx_path)
    with cu.Context():
        module, version, _ = cu.load_ptx(ptx)
        print(f"[rurix-sr_depth] PTX loaded (.version {version}, entry {entry})")
        fn = cu.get_function(module, entry)

        rng = np.random.default_rng(20260616)
        cand_z = rng.random((npix, frags), dtype=np.float32)
        cand_c = rng.random((npix, frags), dtype=np.float32)
        cand_z_flat = cand_z.reshape(-1).copy()
        cand_c_flat = cand_c.reshape(-1).copy()
        zbuf = np.zeros(npix, dtype=np.float32)
        color = np.zeros(npix, dtype=np.float32)

        d_cz = cu.mem_alloc(npix * frags * 4)
        d_cc = cu.mem_alloc(npix * frags * 4)
        d_zb = cu.mem_alloc(npix * 4)
        d_co = cu.mem_alloc(npix * 4)
        cu.memcpy_htod(d_cz, cand_z_flat.ctypes.data, npix * frags * 4)
        cu.memcpy_htod(d_cc, cand_c_flat.ctypes.data, npix * frags * 4)

        args = [
            ctypes.c_uint64(d_cz.value),
            ctypes.c_uint64(d_cc.value),
            ctypes.c_uint64(d_zb.value),
            ctypes.c_uint64(d_co.value),
            ctypes.c_uint64(npix),
            ctypes.c_uint64(frags),
            ctypes.c_float(Z_FAR),
        ]
        cu.launch(fn, (nblocks, 1, 1), (BLOCK, 1, 1), args)
        cu.stream_sync()
        cu.memcpy_dtoh(zbuf.ctypes.data, d_zb, npix * 4)
        cu.memcpy_dtoh(color.ctypes.data, d_co, npix * 4)

        # 参考:固定片元序 less 测试(相等不覆盖,first wins)。
        ref_z = np.full(npix, Z_FAR, dtype=np.float32)
        ref_c = np.zeros(npix, dtype=np.float32)
        for i in range(npix):
            for f in range(frags):
                z = cand_z[i, f]
                if z < ref_z[i]:
                    ref_z[i] = z
                    ref_c[i] = cand_c[i, f]
        if not (np.array_equal(zbuf, ref_z) and np.array_equal(color, ref_c)):
            bad = int(np.argmax(zbuf != ref_z))
            raise AssertionError(
                f"correctness FAIL: pixel {bad} zbuf {zbuf[bad]} expect {ref_z[bad]}"
            )
        print(f"[rurix-sr_depth] correctness PASS (npix={npix}, frags={frags}, less 合成逐字节一致)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
