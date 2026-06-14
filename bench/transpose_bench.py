"""Rurix transpose 基准 harness(M5.3 交付物 D-M5-5;RD-002 承接,BENCH_PROTOCOL.md §3)。

kernel:`src/rurix-rt/kernels/transpose.rx`(16x16 shared-tile 转置,2D ThreadCtx,
atomics-free)→ `bench/kernels/rurix_transpose.ptx`。有效带宽 = 2 * W * H *
sizeof(f32) / t(读 + 写)。

PTX 再生成:
  cargo run -q -p rurixc --bin rurixc -- src/rurix-rt/kernels/transpose.rx \
      --emit=ptx -o bench/kernels/rurix_transpose.ptx

用法:
  py -3 bench/transpose_bench.py --smoke
  py -3 bench/transpose_bench.py --emit evidence/x.json
"""
from __future__ import annotations

import ctypes
import re
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import cuda_driver as cu
from bench.protocol import L2_CLEAR_MB, ROOT, run_protocol, write_evidence
from bench.resolve_ptx import read_ptx

# 主档 4096x4096(16 整除,measured 主档);smoke 取非整除小档验证边界。
W_MAIN, H_MAIN = 4096, 4096
W_SMOKE, H_SMOKE = 200, 150
TILE = 16
KERNEL = "transpose"


def parse_entry(ptx: str, path: Path) -> str:
    m = re.search(r"\.entry\s+([A-Za-z_$][A-Za-z0-9_$]*)", ptx)
    if not m:
        raise RuntimeError(f"无法从 {path} 解析 .entry kernel 名")
    return m.group(1)


def main() -> int:
    smoke = "--smoke" in sys.argv
    emit = ROOT / sys.argv[sys.argv.index("--emit") + 1] if "--emit" in sys.argv else None
    w, h = (W_SMOKE, H_SMOKE) if smoke else (W_MAIN, H_MAIN)

    ptx_path, ptx = read_ptx(KERNEL)
    entry = parse_entry(ptx, ptx_path)
    with cu.Context():
        module, version, jit_log = cu.load_ptx(ptx)
        print(f"[rurix-transpose] PTX loaded (.version {version}, entry {entry})")
        fn = cu.get_function(module, entry)

        src = np.arange(h * w, dtype=np.float32) * np.float32(0.5)  # h 行 × w 列
        d_src = cu.mem_alloc(h * w * 4)
        d_dst = cu.mem_alloc(w * h * 4)
        cu.memcpy_htod(d_src, src.ctypes.data, h * w * 4)

        grid = ((w + TILE - 1) // TILE, (h + TILE - 1) // TILE, 1)
        args = [ctypes.c_uint64(d_src.value), ctypes.c_uint64(d_dst.value),
                ctypes.c_uint64(w), ctypes.c_uint64(h)]
        cu.launch(fn, grid, (TILE, TILE, 1), args)
        cu.stream_sync()
        got = np.empty(w * h, dtype=np.float32)
        cu.memcpy_dtoh(got.ctypes.data, d_dst, w * h * 4)
        # 参考:dst (w 行 × h 列) = src.T
        expect = src.reshape(h, w).T.reshape(w * h)
        if not np.array_equal(got, expect):
            bad = int(np.sum(got != expect))
            raise AssertionError(f"correctness FAIL: {bad}/{w * h} 元素不一致")
        print(f"[rurix-transpose] correctness PASS ({h}x{w} -> {w}x{h}, bitwise equal)")
        if smoke:
            return 0

        l2_buf = cu.mem_alloc(L2_CLEAR_MB * 1024 * 1024)
        events = cu.EventPair()

        def pre_timed() -> None:
            cu.memset_d8(l2_buf, 0, L2_CLEAR_MB * 1024 * 1024)

        def iter_ms() -> float:
            cu.stream_sync()
            events.record_start()
            cu.launch(fn, grid, (TILE, TILE, 1), args)
            events.record_stop()
            cu.stream_sync()
            return events.elapsed_ms()

        gb = 2 * w * h * 4 / 1e9
        doc = run_protocol(
            bench_id="rurix_transpose_f32",
            problem_size=f"{H_MAIN}x{W_MAIN} f32 ({h * w * 4 // (1024 * 1024)} MiB)",
            metric="effective_bandwidth",
            unit="GB/s",
            iter_ms=iter_ms,
            ms_to_metric=lambda ms: gb / (ms / 1e3),
            pre_timed=pre_timed,
            notes=f"rurixc device codegen PTX (.version {version}, entry {entry}); 16x16 "
                  "shared-tile transpose, 2D ThreadCtx, atomics-free",
        )
        doc["results"]["correctness_check"] = "pass"
        if emit:
            write_evidence(doc, emit)
        else:
            import json
            print(json.dumps(doc["results"], ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
