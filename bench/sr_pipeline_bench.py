"""Rurix 软光栅 G0 端到端 L3 管线基准 / 冒烟 harness(M7.5,D-M7-5;契约 G-M7-2)。

口径(反 YAML-only 真跑):在单个 CUDA Event 计时区内,把 G0 软光栅四 stage device
kernel(binning → tile 光栅 → 深度 → tonemap;`src/rurix-rt/kernels/sr_*.rx`,
spec/softraster.md RXS-0118~0121)按 L3 规模背靠背 launch,`frame_ms` = 四 stage
串行 GPU 墙钟(默认流序列化)。四 kernel 签名不改(M7.3 既有 PTX golden 冻结,改签名
属禁区);各 stage 按 L3 独立放大,代表一帧 1920x1080 软光栅的各阶段工作量。

L3 = "大三角形 / 大分辨率帧"(契约 §4 G-M7-2):
  binning  : 多图元(PRIM_COUNT)分桶到大 tile 网格(W/TILE x H/TILE)
  raster   : 大三角形铺满大帧(W x H 覆盖/重心插值)
  depth    : 大像素数(W*H)x 固定片元 less 深度合成
  tonemap  : 大分量数(W*H*3)HDR->LDR 量化

采样遵守 BENCH_PROTOCOL.md §3(warmup/稳态 → 50x3 timed → trimmed mean;CUDA Event
计时;L2 清理);measured_local 判定取锁频起止双探测(protocol.run_protocol)。三次
进程级独立运行 + 回填经 bench/sr_pipeline_triple.py(BENCH_PROTOCOL §3 三次运行规则)。

用法:
  py -3 bench/sr_pipeline_bench.py --smoke                # 缩比装载 + 四 stage 正确性
  py -3 bench/sr_pipeline_bench.py --emit evidence/x.json # L3 协议采样并产证据 JSON

经 rx bench 收编(RD-003,RXS-0088):`rx bench sr_pipeline --smoke`(src/rx/src/main.rs)
透传退出码,协议口径与证据格式不变。
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
from bench.protocol import L2_CLEAR_MB, ROOT, run_protocol, write_evidence
from bench.resolve_ptx import read_ptx

# --- L3 规模(大分辨率帧 / 大三角形;真跑显存约束下的工程裁定值) ---
L3_W = 1920
L3_H = 1080
L3_PRIM_COUNT = 4096
# --- smoke 缩比(快速正确性闸门,管线语义与 L3 同) ---
SMOKE_W = 128
SMOKE_H = 72
SMOKE_PRIM_COUNT = 256

TILE_SIZE = 8
CAP = 32
FRAGS = 4
Z_FAR = 1.0e30

BLOCK_1D = 256
BLOCK_BIN = 64
BLOCK_RASTER = 16

KERNELS = ("sr_binning", "sr_raster_tile", "sr_depth", "sr_tonemap")


def parse_entry(ptx: str, path: Path) -> str:
    m = re.search(r"\.entry\s+([A-Za-z_$][A-Za-z0-9_$]*)", ptx)
    if not m:
        raise RuntimeError(f"无法从 {path} 解析 .entry kernel 名")
    return m.group(1)


def load_kernels() -> dict:
    """装载四 stage PTX,返回 {name: function}(+ 首个 .version 供留痕)。"""
    fns = {}
    version = None
    for name in KERNELS:
        path, ptx = read_ptx(name)
        module, ver, _ = cu.load_ptx(ptx)
        version = version or ver
        fns[name] = cu.get_function(module, parse_entry(ptx, path))
    return fns, version


class Frame:
    """一帧 L3 软光栅的 device 缓冲 + host 输入(确定性 rng),供串联 launch 与正确性比对。"""

    def __init__(self, w: int, h: int, prim_count: int):
        self.w = w
        self.h = h
        self.prim_count = prim_count
        self.tiles_x = w // TILE_SIZE
        self.tiles_y = h // TILE_SIZE
        self.ntiles = self.tiles_x * self.tiles_y
        self.npix = w * h
        self.ncomp = w * h * 3
        rng = np.random.default_rng(20260616)

        # binning 输入:prim_count 个三角形(x 列按 W、y 列按 H 缩放,覆盖 tile 网格像素域)。
        prim = rng.random((prim_count, 6), dtype=np.float32)
        prim[:, 0::2] *= np.float32(w)   # x0,x1,x2
        prim[:, 1::2] *= np.float32(h)   # y0,y1,y2
        self.prim = prim
        self.prim_flat = prim.reshape(-1).copy()

        # raster 输入:大三角形铺满大帧(CCW,镜像 smoke 朝向缩放),属性顶点值。
        self.tri = np.array([
            w * 0.125, h * 0.125,
            w * 0.875, h * 0.1875,
            w * 0.1875, h * 0.8125,
            1.0, 2.0, 3.0,
        ], dtype=np.float32)

        # depth 输入:每像素 FRAGS 个候选 (z, color)。具名持有 flat 拷贝避免临时数组在
        # memcpy 前被 GC 回收(CPython:.ctypes.data 取址后不持引用)。
        self.cand_z = rng.random((self.npix, FRAGS), dtype=np.float32)
        self.cand_c = rng.random((self.npix, FRAGS), dtype=np.float32)
        self.cand_z_flat = self.cand_z.reshape(-1).copy()
        self.cand_c_flat = self.cand_c.reshape(-1).copy()

        # tonemap 输入:HDR 分量。
        self.hdr = (rng.standard_normal(self.ncomp).astype(np.float32) * np.float32(0.6)
                    + np.float32(0.5))

        # device 分配 + 输入上行。
        f4 = 4
        self.d_prim = cu.mem_alloc(prim_count * 6 * f4)
        self.d_count = cu.mem_alloc(self.ntiles * f4)
        self.d_list = cu.mem_alloc(self.ntiles * CAP * f4)
        self.d_tri = cu.mem_alloc(9 * f4)
        self.d_cov = cu.mem_alloc(self.npix * f4)
        self.d_cz = cu.mem_alloc(self.npix * FRAGS * f4)
        self.d_cc = cu.mem_alloc(self.npix * FRAGS * f4)
        self.d_zb = cu.mem_alloc(self.npix * f4)
        self.d_co = cu.mem_alloc(self.npix * f4)
        self.d_hdr = cu.mem_alloc(self.ncomp * f4)
        self.d_ldr = cu.mem_alloc(self.ncomp * f4)

        cu.memcpy_htod(self.d_prim, self.prim_flat.ctypes.data, prim_count * 6 * f4)
        cu.memcpy_htod(self.d_tri, self.tri.ctypes.data, 9 * f4)
        cu.memcpy_htod(self.d_cz, self.cand_z_flat.ctypes.data, self.npix * FRAGS * f4)
        cu.memcpy_htod(self.d_cc, self.cand_c_flat.ctypes.data, self.npix * FRAGS * f4)
        cu.memcpy_htod(self.d_hdr, self.hdr.ctypes.data, self.ncomp * f4)

        # launch 参数(输入只读、输出每帧覆盖 → 跨迭代输入恒定)。
        self.bin_args = [
            ctypes.c_uint64(self.d_prim.value), ctypes.c_uint64(self.d_count.value),
            ctypes.c_uint64(self.d_list.value), ctypes.c_uint64(self.tiles_x),
            ctypes.c_uint64(self.tiles_y), ctypes.c_uint64(TILE_SIZE),
            ctypes.c_uint64(prim_count), ctypes.c_uint64(CAP),
        ]
        self.raster_args = [
            ctypes.c_uint64(self.d_tri.value), ctypes.c_uint64(self.d_cov.value),
            ctypes.c_uint64(w), ctypes.c_uint64(h),
        ]
        self.depth_args = [
            ctypes.c_uint64(self.d_cz.value), ctypes.c_uint64(self.d_cc.value),
            ctypes.c_uint64(self.d_zb.value), ctypes.c_uint64(self.d_co.value),
            ctypes.c_uint64(self.npix), ctypes.c_uint64(FRAGS), ctypes.c_float(Z_FAR),
        ]
        self.tonemap_args = [
            ctypes.c_uint64(self.d_hdr.value), ctypes.c_uint64(self.d_ldr.value),
            ctypes.c_uint64(self.ncomp),
        ]

    def launch_frame(self, fns: dict) -> None:
        """四 stage 背靠背 launch(默认流序列化;不在 stage 间同步,测整帧 GPU 墙钟)。"""
        bin_blocks = (self.ntiles + BLOCK_BIN - 1) // BLOCK_BIN
        cu.launch(fns["sr_binning"], (bin_blocks, 1, 1), (BLOCK_BIN, 1, 1), self.bin_args)
        gx = (self.w + BLOCK_RASTER - 1) // BLOCK_RASTER
        gy = (self.h + BLOCK_RASTER - 1) // BLOCK_RASTER
        cu.launch(fns["sr_raster_tile"], (gx, gy, 1), (BLOCK_RASTER, BLOCK_RASTER, 1),
                  self.raster_args)
        dep_blocks = (self.npix + BLOCK_1D - 1) // BLOCK_1D
        cu.launch(fns["sr_depth"], (dep_blocks, 1, 1), (BLOCK_1D, 1, 1), self.depth_args)
        tm_blocks = (self.ncomp + BLOCK_1D - 1) // BLOCK_1D
        cu.launch(fns["sr_tonemap"], (tm_blocks, 1, 1), (BLOCK_1D, 1, 1), self.tonemap_args)

    def verify(self, fns: dict) -> None:
        """逐 stage GPU 输出 vs 向量化 host 参考(与 sr_*_bench.py smoke oracle 同义)。"""
        self.launch_frame(fns)
        cu.stream_sync()
        T = float(TILE_SIZE)

        # --- binning:半开区间相交分桶计数(cap 截断);向量化逐图元累加 ---
        cover_count = np.zeros(self.ntiles, dtype=np.float32)
        cu.memcpy_dtoh(cover_count.ctypes.data, self.d_count, self.ntiles * 4)
        xs = self.prim[:, 0::2]
        ys = self.prim[:, 1::2]
        bx0, bx1 = xs.min(axis=1), xs.max(axis=1)
        by0, by1 = ys.min(axis=1), ys.max(axis=1)
        ref_grid = np.zeros((self.tiles_y, self.tiles_x), dtype=np.int64)
        for k in range(self.prim_count):
            ax = max(0, int(math.floor(bx0[k] / T)))
            bxe = min(self.tiles_x - 1, int(math.ceil(bx1[k] / T)) - 1)
            ay = max(0, int(math.floor(by0[k] / T)))
            bye = min(self.tiles_y - 1, int(math.ceil(by1[k] / T)) - 1)
            if bxe >= ax and bye >= ay:
                ref_grid[ay:bye + 1, ax:bxe + 1] += 1
        ref_count = np.minimum(ref_grid, CAP).reshape(-1).astype(np.float32)
        if not np.array_equal(cover_count, ref_count):
            bad = int(np.argmax(cover_count != ref_count))
            raise AssertionError(
                f"binning FAIL: tile {bad} count {cover_count[bad]} expect {ref_count[bad]}")

        # --- raster:逐像素边函数同号覆盖 + 重心插值 ---
        cov = np.zeros(self.npix, dtype=np.float32)
        cu.memcpy_dtoh(cov.ctypes.data, self.d_cov, self.npix * 4)
        x0, y0, x1, y1, x2, y2, a0, a1, a2 = (float(v) for v in self.tri)
        xx, yy = np.meshgrid(np.arange(self.w, dtype=np.float64) + 0.5,
                             np.arange(self.h, dtype=np.float64) + 0.5)
        area2 = (x1 - x0) * (y2 - y0) - (y1 - y0) * (x2 - x0)
        e0 = (x2 - x1) * (yy - y1) - (y2 - y1) * (xx - x1)
        e1 = (x0 - x2) * (yy - y2) - (y0 - y2) * (xx - x2)
        e2 = (x1 - x0) * (yy - y0) - (y1 - y0) * (xx - x0)
        covered = (area2 != 0.0) & (e0 >= 0.0) & (e1 >= 0.0) & (e2 >= 0.0)
        w0 = np.where(covered, e0 / area2, 0.0)
        w1 = np.where(covered, e1 / area2, 0.0)
        w2 = np.where(covered, e2 / area2, 0.0)
        ref_cov = np.where(covered, w0 * a0 + w1 * a1 + w2 * a2, 0.0).astype(np.float32).reshape(-1)
        if int(covered.sum()) == 0:
            raise AssertionError("raster FAIL: 无覆盖像素(大三角形/光栅异常)")
        if not np.allclose(cov, ref_cov, atol=1e-3):
            bad = int(np.argmax(np.abs(cov - ref_cov)))
            raise AssertionError(
                f"raster FAIL: pixel {bad} got {cov[bad]} expect {ref_cov[bad]}")

        # --- depth:固定片元序 less 合成(相等不覆盖,first wins = argmin 首现) ---
        zbuf = np.zeros(self.npix, dtype=np.float32)
        color = np.zeros(self.npix, dtype=np.float32)
        cu.memcpy_dtoh(zbuf.ctypes.data, self.d_zb, self.npix * 4)
        cu.memcpy_dtoh(color.ctypes.data, self.d_co, self.npix * 4)
        win = self.cand_z.argmin(axis=1)
        rows = np.arange(self.npix)
        ref_z = self.cand_z[rows, win]
        ref_c = self.cand_c[rows, win]
        if not (np.array_equal(zbuf, ref_z) and np.array_equal(color, ref_c)):
            bad = int(np.argmax(zbuf != ref_z))
            raise AssertionError(
                f"depth FAIL: pixel {bad} zbuf {zbuf[bad]} expect {ref_z[bad]}")

        # --- tonemap:clamp[0,1]+NaN->0+floor(c*255+0.5) 半值向上 ---
        ldr = np.zeros(self.ncomp, dtype=np.float32)
        cu.memcpy_dtoh(ldr.ctypes.data, self.d_ldr, self.ncomp * 4)
        clamped = np.where(np.isnan(self.hdr), 0.0, np.clip(self.hdr, 0.0, 1.0))
        ref_ldr = np.floor(clamped * 255.0 + 0.5).astype(np.float32)
        if not np.array_equal(ldr, ref_ldr):
            bad = int(np.argmax(ldr != ref_ldr))
            raise AssertionError(
                f"tonemap FAIL: idx {bad} got {ldr[bad]} expect {ref_ldr[bad]}")


def main() -> int:
    smoke = "--smoke" in sys.argv
    emit = None
    if "--emit" in sys.argv:
        emit = ROOT / sys.argv[sys.argv.index("--emit") + 1]
    if not smoke and emit is None:
        print("[sr_pipeline] 用法:--smoke 或 --emit <证据路径>")
        return 2

    w, h, prims = (SMOKE_W, SMOKE_H, SMOKE_PRIM_COUNT) if smoke else (L3_W, L3_H, L3_PRIM_COUNT)

    with cu.Context():
        fns, version = load_kernels()
        print(f"[sr_pipeline] 四 stage PTX loaded (.version {version}): {', '.join(KERNELS)}")
        frame = Frame(w, h, prims)
        frame.verify(fns)
        print(f"[sr_pipeline] correctness PASS (frame {w}x{h}, prims {prims}, "
              f"四 stage 逐 stage 参考一致)")
        if smoke:
            return 0

        l2_buf = cu.mem_alloc(L2_CLEAR_MB * 1024 * 1024)
        events = cu.EventPair()

        def pre_timed() -> None:
            cu.memset_d8(l2_buf, 0, L2_CLEAR_MB * 1024 * 1024)

        def iter_ms() -> float:
            cu.stream_sync()              # 测量区前刷 WDDM batch(r11 §1.4)
            events.record_start()
            frame.launch_frame(fns)       # 四 stage 串行(默认流)= 一帧
            events.record_stop()
            cu.stream_sync()
            return events.elapsed_ms()

        doc = run_protocol(
            bench_id="soft_raster_l3",
            problem_size=(f"{w}x{h} frame; binning {prims} prims/"
                          f"{frame.tiles_x}x{frame.tiles_y} tiles cap{CAP}, "
                          f"raster 1 big-tri, depth {frame.npix}px x{FRAGS} frags, "
                          f"tonemap {frame.ncomp} comp"),
            metric="frame_time",
            unit="ms",
            iter_ms=iter_ms,
            ms_to_metric=lambda ms: ms,   # 帧时间本身即指标(ms)
            pre_timed=pre_timed,
            notes="G0 软光栅四 stage(binning/raster/depth/tonemap)device kernel "
                  "RXS-0118~0121 背靠背 launch 单帧 GPU 墙钟;四 stage 各按 L3 独立放大,"
                  "代表一帧大分辨率软光栅;kernel 签名不改(M7.3 PTX golden 冻结); "
                  "desktop WDDM,其他 GPU 进程概况记入 isolation_check",
        )
        doc["bench"]["level"] = "L3"       # protocol 默认 L1,L3 规模在此覆写
        doc["results"]["correctness_check"] = "pass"
        if emit:
            write_evidence(doc, emit)
    return 0


if __name__ == "__main__":
    sys.exit(main())
