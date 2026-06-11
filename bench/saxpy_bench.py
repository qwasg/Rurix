"""手写 PTX SAXPY 基线 harness(M0 交付物 D-M0-3,BENCH_PROTOCOL.md §4.1)。

用法:
  py -3 bench/saxpy_bench.py --smoke                 # 装载 + 单次执行 + 正确性(CI 步骤 5)
  py -3 bench/saxpy_bench.py --emit evidence/x.json  # 完整协议采样并产出证据 JSON
"""
from __future__ import annotations

import ctypes
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import cuda_driver as cu
from bench.protocol import L2_CLEAR_MB, ROOT, run_protocol, write_evidence

N_MAIN = 2 ** 24          # 主档(BENCH_PROTOCOL.md §4.1)
N_SMOKE = 2 ** 20
BLOCK = 256
A = np.float32(2.5)


def reference(a: np.float32, x: np.ndarray, y: np.ndarray) -> np.ndarray:
    # 与 PTX 的 mul.rn + add.rn 逐位一致:f32 两次舍入
    return (a * x).astype(np.float32) + y


def setup(n: int):
    rng = np.random.default_rng(20260611)
    x = rng.standard_normal(n, dtype=np.float32)
    y = rng.standard_normal(n, dtype=np.float32)
    nbytes = n * 4
    d_x, d_y = cu.mem_alloc(nbytes), cu.mem_alloc(nbytes)
    cu.memcpy_htod(d_x, x.ctypes.data, nbytes)
    cu.memcpy_htod(d_y, y.ctypes.data, nbytes)
    return x, y, d_x, d_y, nbytes


def check_correctness(fn, n: int, x, y, d_x, d_y, nbytes) -> None:
    grid = ((n + BLOCK - 1) // BLOCK, 1, 1)
    cu.launch(fn, grid, (BLOCK, 1, 1),
              [ctypes.c_uint64(d_x.value), ctypes.c_uint64(d_y.value),
               ctypes.c_float(A), ctypes.c_uint32(n)])
    cu.stream_sync()
    out = np.empty(n, dtype=np.float32)
    cu.memcpy_dtoh(out.ctypes.data, d_y, nbytes)
    expected = reference(A, x, y)
    if not np.array_equal(out, expected):
        bad = int(np.sum(out != expected))
        raise AssertionError(f"correctness FAIL: {bad}/{n} 元素不一致")
    # 恢复 y 原值,保证后续迭代输入恒定
    cu.memcpy_htod(d_y, y.ctypes.data, nbytes)


def main() -> int:
    smoke = "--smoke" in sys.argv
    emit = None
    if "--emit" in sys.argv:
        emit = ROOT / sys.argv[sys.argv.index("--emit") + 1]
    n = N_SMOKE if smoke else N_MAIN

    ptx = (ROOT / "bench/kernels/saxpy.ptx").read_text(encoding="utf-8")
    with cu.Context():
        module, version, jit_log = cu.load_ptx(ptx)
        print(f"[saxpy] PTX loaded (.version {version})" + (f" jit: {jit_log}" if jit_log.strip() else ""))
        fn = cu.get_function(module, "saxpy")
        x, y, d_x, d_y, nbytes = setup(n)
        check_correctness(fn, n, x, y, d_x, d_y, nbytes)
        print(f"[saxpy] correctness PASS (n=2^{n.bit_length() - 1}, bitwise equal)")
        if smoke:
            return 0

        # L2 清理 buffer(r11 §1.1)
        l2_buf = cu.mem_alloc(L2_CLEAR_MB * 1024 * 1024)
        events = cu.EventPair()
        grid = ((n + BLOCK - 1) // BLOCK, 1, 1)
        args = [ctypes.c_uint64(d_x.value), ctypes.c_uint64(d_y.value),
                ctypes.c_float(A), ctypes.c_uint32(n)]

        def pre_timed() -> None:
            cu.memset_d8(l2_buf, 0, L2_CLEAR_MB * 1024 * 1024)

        def iter_ms() -> float:
            cu.stream_sync()              # 测量区前刷 WDDM batch(r11 §1.4)
            events.record_start()
            cu.launch(fn, grid, (BLOCK, 1, 1), args)
            events.record_stop()
            cu.stream_sync()              # 测量区后同步
            return events.elapsed_ms()

        gb = 3 * nbytes / 1e9             # 读 x、读 y、写 y

        doc = run_protocol(
            bench_id="saxpy_f32",
            problem_size=f"2^24 elements f32 ({nbytes // (1024 * 1024)} MiB/array)",
            metric="effective_bandwidth",
            unit="GB/s",
            iter_ms=iter_ms,
            ms_to_metric=lambda ms: gb / (ms / 1e3),
            pre_timed=pre_timed,
            notes=f"handwritten PTX .version {version}, mul.rn+add.rn (no FMA, bitwise host comparison); "
                  "desktop WDDM environment: other GPU-using processes recorded in isolation_check",
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
