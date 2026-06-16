"""手写 CUDA C++ SAXPY 对照基准 harness(M8.2,契约 G-M8-2;BENCH_PROTOCOL.md §3)。

kernel:`bench/cuda_ref/saxpy.cu`(out=a*x+y)→ `bench/kernels/cuda_saxpy.ptx`。
有效带宽 = 3 * N * sizeof(f32) / t(读 x、读 y、写 out;与 rurix_saxpy_bench 同口径)。
2^24 元素与 bench/rurix_saxpy_bench.py 同档(numerator m8.bench.saxpy 的对照分母)。

PTX 再生成:py -3 bench/compile_cuda_ref.py(需 MSVC cl.exe;vcvars64 环境)

用法:
  py -3 bench/saxpy_cuda_bench.py --smoke
  py -3 bench/saxpy_cuda_bench.py --emit evidence/x.json
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

N_MAIN = 2 ** 24
N_SMOKE = 2 ** 20
BLOCK = 256
A_SCALAR = np.float32(2.5)
ABS_TOL = 1e-4
KERNEL = "saxpy"


def read_cuda_ptx(name: str) -> tuple[Path, str]:
    path = ROOT / f"bench/kernels/cuda_{name}.ptx"
    if not path.is_file():
        raise FileNotFoundError(
            f"找不到 CUDA 对照 PTX {path.relative_to(ROOT)}; 运行 py -3 bench/compile_cuda_ref.py"
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
    n = N_SMOKE if smoke else N_MAIN
    nbytes = n * 4

    ptx_path, ptx = read_cuda_ptx(KERNEL)
    entry = parse_entry(ptx, ptx_path)
    with cu.Context():
        module, version, _ = cu.load_ptx(ptx)
        print(f"[cuda-saxpy] PTX loaded (.version {version}, entry {entry})")
        fn = cu.get_function(module, entry)

        rng = np.random.default_rng(20260616)
        x = rng.standard_normal(n).astype(np.float32)
        y = rng.standard_normal(n).astype(np.float32)
        d_out, d_x, d_y = cu.mem_alloc(nbytes), cu.mem_alloc(nbytes), cu.mem_alloc(nbytes)
        cu.memcpy_htod(d_x, x.ctypes.data, nbytes)
        cu.memcpy_htod(d_y, y.ctypes.data, nbytes)
        grid = ((n + BLOCK - 1) // BLOCK, 1, 1)
        args = [ctypes.c_uint64(d_out.value), ctypes.c_uint64(d_x.value),
                ctypes.c_uint64(d_y.value), ctypes.c_float(float(A_SCALAR)), ctypes.c_uint64(n)]
        cu.launch(fn, grid, (BLOCK, 1, 1), args)
        cu.stream_sync()
        got = np.empty(n, dtype=np.float32)
        cu.memcpy_dtoh(got.ctypes.data, d_out, nbytes)
        expect = A_SCALAR * x + y
        err = float(np.abs(got.astype(np.float64) - expect.astype(np.float64)).max())
        if err > ABS_TOL:
            raise AssertionError(f"correctness FAIL: max abs err {err:.2e}")
        print(f"[cuda-saxpy] correctness PASS (N={n}, max abs err {err:.2e})")
        if smoke:
            return 0

        l2_buf = cu.mem_alloc(L2_CLEAR_MB * 1024 * 1024)
        events = cu.EventPair()

        def pre_timed() -> None:
            cu.memset_d8(l2_buf, 0, L2_CLEAR_MB * 1024 * 1024)

        def iter_ms() -> float:
            cu.stream_sync()
            events.record_start()
            cu.launch(fn, grid, (BLOCK, 1, 1), args)
            events.record_stop()
            cu.stream_sync()
            return events.elapsed_ms()

        gb = 3 * nbytes / 1e9
        doc = run_protocol(
            bench_id="cuda_saxpy_f32",
            problem_size=f"2^24 elements f32 ({nbytes // (1024 * 1024)} MiB/array)",
            metric="effective_bandwidth",
            unit="GB/s",
            iter_ms=iter_ms,
            ms_to_metric=lambda ms: gb / (ms / 1e3),
            pre_timed=pre_timed,
            notes=f"handwritten CUDA C++ reference PTX (.version {version}, entry {entry}); "
                  "out=a*x+y, read x/y write out; metric id m8.bench.saxpy_cuda.effective_bandwidth_gbps",
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
