"""Rurix 软光栅 sr_raster_tile 基准 / 冒烟 harness(M7.3,D-M7-3;spec/softraster.md RXS-0119)。

kernel 来自 rurixc device codegen:`src/rurix-rt/kernels/sr_raster_tile.rx`(2D 线程映射
像素,边函数符号同号判定覆盖 + 重心坐标插值,退化三角形不覆盖;每像素独立、确定性)。

用法:
  py -3 bench/sr_raster_tile_bench.py --smoke   # 装载 + 执行 + 正确性(Compute Sanitizer nightly 包裹)

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

W = 64
H = 64
BLOCK = 16
KERNEL = "sr_raster_tile"
# 单三角形 (x0,y0,x1,y1,x2,y2,a0,a1,a2)(逆时针,属性顶点值)。
TRI = np.array([8.0, 8.0, 56.0, 12.0, 12.0, 52.0, 1.0, 2.0, 3.0], dtype=np.float32)


def parse_entry(ptx: str, path: Path) -> str:
    m = re.search(r"\.entry\s+([A-Za-z_$][A-Za-z0-9_$]*)", ptx)
    if not m:
        raise RuntimeError(f"无法从 {path} 解析 .entry kernel 名")
    return m.group(1)


def edge(ax, ay, bx, by, px, py):
    return (bx - ax) * (py - ay) - (by - ay) * (px - ax)


def main() -> int:
    if "--smoke" not in sys.argv:
        print("[rurix-sr_raster_tile] 仅 --smoke 已实现;L3 协议化采样(G-M7-2)由 M7.5 回填")
        return 0
    gx = (W + BLOCK - 1) // BLOCK
    gy = (H + BLOCK - 1) // BLOCK

    ptx_path, ptx = read_ptx(KERNEL)
    entry = parse_entry(ptx, ptx_path)
    with cu.Context():
        module, version, _ = cu.load_ptx(ptx)
        print(f"[rurix-sr_raster_tile] PTX loaded (.version {version}, entry {entry})")
        fn = cu.get_function(module, entry)

        cover = np.zeros(W * H, dtype=np.float32)
        d_tri = cu.mem_alloc(9 * 4)
        d_cov = cu.mem_alloc(W * H * 4)
        cu.memcpy_htod(d_tri, TRI.ctypes.data, 9 * 4)

        args = [
            ctypes.c_uint64(d_tri.value),
            ctypes.c_uint64(d_cov.value),
            ctypes.c_uint64(W),
            ctypes.c_uint64(H),
        ]
        cu.launch(fn, (gx, gy, 1), (BLOCK, BLOCK, 1), args)
        cu.stream_sync()
        cu.memcpy_dtoh(cover.ctypes.data, d_cov, W * H * 4)

        # 参考:逐像素边函数同号覆盖 + 重心插值。
        x0, y0, x1, y1, x2, y2, a0, a1, a2 = (float(v) for v in TRI)
        area2 = edge(x0, y0, x1, y1, x2, y2)
        ref = np.zeros(W * H, dtype=np.float32)
        for y in range(H):
            for x in range(W):
                px = x + 0.5
                py = y + 0.5
                e0 = edge(x1, y1, x2, y2, px, py)
                e1 = edge(x2, y2, x0, y0, px, py)
                e2 = edge(x0, y0, x1, y1, px, py)
                if area2 != 0.0 and e0 >= 0.0 and e1 >= 0.0 and e2 >= 0.0:
                    w0 = e0 / area2
                    w1 = e1 / area2
                    w2 = e2 / area2
                    ref[y * W + x] = np.float32(w0 * a0 + w1 * a1 + w2 * a2)
        covered = int((cover != 0.0).sum())
        # 覆盖集合一致(浮点插值容差比对)。
        if covered == 0:
            raise AssertionError("correctness FAIL: 无覆盖像素(三角形/光栅异常)")
        if not np.allclose(cover, ref, atol=1e-4):
            bad = int(np.argmax(np.abs(cover - ref)))
            raise AssertionError(
                f"correctness FAIL: pixel {bad} got {cover[bad]} expect {ref[bad]}"
            )
        print(f"[rurix-sr_raster_tile] correctness PASS ({W}x{H}, 覆盖像素 {covered}, 重心插值一致)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
