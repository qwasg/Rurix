"""手写 CUDA C++ tiled GEMM 对照基准 harness(M5.3 WP8;RD-002 承接,BENCH_PROTOCOL.md §3)。

kernel:`bench/cuda_ref/gemm_tile.cu`(经典 16x16 shared-memory tiling,不触 Tensor Core)
→ `bench/kernels/cuda_gemm_tile.ptx`。吞吐 = 2*M*N*K / t(GFLOPS)。

PTX 再生成:
  py -3 bench/compile_cuda_ref.py

用法:
  py -3 bench/gemm_tile_cuda_bench.py --smoke
  py -3 bench/gemm_tile_cuda_bench.py --emit evidence/x.json
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

M_MAIN = N_MAIN = K_MAIN = 1024
M_SMOKE, N_SMOKE, K_SMOKE = 100, 80, 70
TILE = 16
REL_TOL = 1e-3
KERNEL = "gemm_tile"


def read_cuda_ptx(name: str) -> tuple[Path, str]:
    path = ROOT / f"bench/kernels/cuda_{name}.ptx"
    if not path.is_file():
        raise FileNotFoundError(
            f"找不到 CUDA 对照 PTX {path.relative_to(ROOT)}; "
            "运行 py -3 bench/compile_cuda_ref.py"
        )
    return path, path.read_text(encoding="utf-8")


def parse_entry(ptx: str, path: Path) -> str:
    m = re.search(r"\.entry\s+([A-Za-z_$][A-Za-z0-9_$]*)", ptx)
    if not m:
        raise RuntimeError(f"无法从 {path} 解析 .entry kernel 名")
    return m.group(1)


def main() -> int:
    smoke = "--smoke" in sys.argv
    emit = ROOT / sys.argv[sys.argv.index("--emit") + 1] if "--emit" in sys.argv else None
    m, n, k = (M_SMOKE, N_SMOKE, K_SMOKE) if smoke else (M_MAIN, N_MAIN, K_MAIN)

    ptx_path, ptx = read_cuda_ptx(KERNEL)
    entry = parse_entry(ptx, ptx_path)
    with cu.Context():
        module, version, jit_log = cu.load_ptx(ptx)
        print(f"[cuda-gemm] PTX loaded (.version {version}, entry {entry})")
        fn = cu.get_function(module, entry)

        rng = np.random.default_rng(20260614)
        a = (rng.integers(0, 7, size=m * k).astype(np.float32)) * np.float32(0.1) + np.float32(0.05)
        b = (rng.integers(0, 5, size=k * n).astype(np.float32)) * np.float32(0.2) + np.float32(0.1)
        d_a = cu.mem_alloc(m * k * 4)
        d_b = cu.mem_alloc(k * n * 4)
        d_c = cu.mem_alloc(m * n * 4)
        cu.memcpy_htod(d_a, a.ctypes.data, m * k * 4)
        cu.memcpy_htod(d_b, b.ctypes.data, k * n * 4)

        grid = ((n + TILE - 1) // TILE, (m + TILE - 1) // TILE, 1)
        args = [ctypes.c_uint64(d_a.value), ctypes.c_uint64(d_b.value), ctypes.c_uint64(d_c.value),
                ctypes.c_uint64(m), ctypes.c_uint64(n), ctypes.c_uint64(k)]
        cu.launch(fn, grid, (TILE, TILE, 1), args)
        cu.stream_sync()
        got = np.empty(m * n, dtype=np.float32)
        cu.memcpy_dtoh(got.ctypes.data, d_c, m * n * 4)
        expect = (a.reshape(m, k).astype(np.float64) @ b.reshape(k, n).astype(np.float64)).reshape(m * n)
        denom = np.maximum(np.abs(expect), 1.0)
        rel = np.abs(got.astype(np.float64) - expect) / denom
        if rel.max() > REL_TOL:
            raise AssertionError(f"correctness FAIL: max rel err {rel.max():.2e}")
        print(f"[cuda-gemm] correctness PASS ({m}x{k} * {k}x{n}, max rel err {rel.max():.2e})")
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

        gflop = 2.0 * m * n * k / 1e9
        doc = run_protocol(
            bench_id="cuda_gemm_tile_f32",
            problem_size=f"{M_MAIN}x{K_MAIN} * {K_MAIN}x{N_MAIN} f32",
            metric="throughput",
            unit="GFLOPS",
            iter_ms=iter_ms,
            ms_to_metric=lambda ms: gflop / (ms / 1e3),
            pre_timed=pre_timed,
            notes=f"handwritten CUDA C++ reference PTX (.version {version}, entry {entry}); "
                  "classic 16x16 shared-memory tiling, NO Tensor Core, atomics-free; "
                  "metric id m5.bench.gemm_tile_cuda.throughput_gflops",
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
