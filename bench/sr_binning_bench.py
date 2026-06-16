"""Rurix 软光栅 sr_binning 基准 / 冒烟 harness(M7.3,D-M7-3;spec/softraster.md RXS-0118)。

kernel 来自 rurixc device codegen:`src/rurix-rt/kernels/sr_binning.rx`(图元分桶到 tile,
每 tile owner 线程按图元下标升序遍历,包围盒与 tile 像素矩形相交则入桶,达 cap 截断;
atomics-free、确定性遍历序)。

用法:
  py -3 bench/sr_binning_bench.py --smoke   # 装载 + 执行 + 正确性(Compute Sanitizer nightly 包裹)

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

TILES_X = 8
TILES_Y = 8
TILE_SIZE = 8
PRIM_COUNT = 64
CAP = 32
BLOCK = 64
KERNEL = "sr_binning"


def parse_entry(ptx: str, path: Path) -> str:
    m = re.search(r"\.entry\s+([A-Za-z_$][A-Za-z0-9_$]*)", ptx)
    if not m:
        raise RuntimeError(f"无法从 {path} 解析 .entry kernel 名")
    return m.group(1)


def main() -> int:
    if "--smoke" not in sys.argv:
        print("[rurix-sr_binning] 仅 --smoke 已实现;L3 协议化采样(G-M7-2)由 M7.5 回填")
        return 0
    ntiles = TILES_X * TILES_Y
    nblocks = (ntiles + BLOCK - 1) // BLOCK

    ptx_path, ptx = read_ptx(KERNEL)
    entry = parse_entry(ptx, ptx_path)
    with cu.Context():
        module, version, _ = cu.load_ptx(ptx)
        print(f"[rurix-sr_binning] PTX loaded (.version {version}, entry {entry})")
        fn = cu.get_function(module, entry)

        rng = np.random.default_rng(20260616)
        # 每图元 6 个 f32:x0,y0,x1,y1,x2,y2(屏幕坐标,范围覆盖 tile 网格像素域)。
        extent = float(TILES_X * TILE_SIZE)
        prim = (rng.random((PRIM_COUNT, 6), dtype=np.float32) * np.float32(extent))
        prim_flat = prim.reshape(-1).copy()
        bin_count = np.zeros(ntiles, dtype=np.float32)
        bin_list = np.zeros(ntiles * CAP, dtype=np.float32)

        d_prim = cu.mem_alloc(PRIM_COUNT * 6 * 4)
        d_count = cu.mem_alloc(ntiles * 4)
        d_list = cu.mem_alloc(ntiles * CAP * 4)
        cu.memcpy_htod(d_prim, prim_flat.ctypes.data, PRIM_COUNT * 6 * 4)

        args = [
            ctypes.c_uint64(d_prim.value),
            ctypes.c_uint64(d_count.value),
            ctypes.c_uint64(d_list.value),
            ctypes.c_uint64(TILES_X),
            ctypes.c_uint64(TILES_Y),
            ctypes.c_uint64(TILE_SIZE),
            ctypes.c_uint64(PRIM_COUNT),
            ctypes.c_uint64(CAP),
        ]
        cu.launch(fn, (nblocks, 1, 1), (BLOCK, 1, 1), args)
        cu.stream_sync()
        cu.memcpy_dtoh(bin_count.ctypes.data, d_count, ntiles * 4)
        cu.memcpy_dtoh(bin_list.ctypes.data, d_list, ntiles * CAP * 4)

        # 参考:半开区间相交分桶,升序遍历,cap 截断。
        bx0 = prim[:, 0::2].min(axis=1)
        bx1 = prim[:, 0::2].max(axis=1)
        by0 = prim[:, 1::2].min(axis=1)
        by1 = prim[:, 1::2].max(axis=1)
        ref_count = np.zeros(ntiles, dtype=np.float32)
        for tile in range(ntiles):
            tx = tile % TILES_X
            ty = tile // TILES_X
            tx0 = float(tx * TILE_SIZE)
            tx1 = float((tx + 1) * TILE_SIZE)
            ty0 = float(ty * TILE_SIZE)
            ty1 = float((ty + 1) * TILE_SIZE)
            cnt = 0
            for k in range(PRIM_COUNT):
                if bx0[k] < tx1 and tx0 < bx1[k] and by0[k] < ty1 and ty0 < by1[k]:
                    if cnt < CAP:
                        cnt += 1
            ref_count[tile] = float(cnt)
        if not np.array_equal(bin_count, ref_count):
            bad = int(np.argmax(bin_count != ref_count))
            raise AssertionError(
                f"correctness FAIL: tile {bad} count {bin_count[bad]} expect {ref_count[bad]}"
            )
        print(f"[rurix-sr_binning] correctness PASS (tiles={ntiles}, prims={PRIM_COUNT}, 分桶计数一致)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
