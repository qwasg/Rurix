"""手写 CUDA C++ GEMV 对照基准 harness(M8.2,契约 G-M8-2;BENCH_PROTOCOL.md §3)。

kernel:`bench/cuda_ref/gemv.cu`(y=A·x,行主序,每线程一输出行)→
`bench/kernels/cuda_gemv.ptx`。GEMV 访存受限,有效带宽 = (M*N + N + M)*sizeof(f32)/t
(读 A、读 x、写 y;与 bench/cublas_gemv_bench.py 同口径与同档 8192²)。
cublas GEMV 绑定 m8.bench.cublas_gemv 的 ≥90% 对照分母(m8.bench.gemv_cuda)。

PTX 再生成:py -3 bench/compile_cuda_ref.py(需 MSVC cl.exe;vcvars64 环境)

用法:
  py -3 bench/gemv_cuda_bench.py --smoke
  py -3 bench/gemv_cuda_bench.py --emit evidence/x.json
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

M_MAIN = N_MAIN = 8192
M_SMOKE, N_SMOKE = 160, 128
BLOCK = 256
REL_TOL = 1e-3
KERNEL = "gemv"


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
    m, n = (M_SMOKE, N_SMOKE) if smoke else (M_MAIN, N_MAIN)

    ptx_path, ptx = read_cuda_ptx(KERNEL)
    entry = parse_entry(ptx, ptx_path)
    with cu.Context():
        module, version, _ = cu.load_ptx(ptx)
        print(f"[cuda-gemv] PTX loaded (.version {version}, entry {entry})")
        fn = cu.get_function(module, entry)

        rng = np.random.default_rng(20260616)
        a = (rng.integers(0, 7, size=m * n).astype(np.float32)) * np.float32(0.1) + np.float32(0.05)
        x = (rng.integers(0, 5, size=n).astype(np.float32)) * np.float32(0.2) + np.float32(0.1)
        d_a = cu.mem_alloc(m * n * 4)
        d_x = cu.mem_alloc(n * 4)
        d_y = cu.mem_alloc(m * 4)
        cu.memcpy_htod(d_a, a.ctypes.data, m * n * 4)
        cu.memcpy_htod(d_x, x.ctypes.data, n * 4)
        grid = ((m + BLOCK - 1) // BLOCK, 1, 1)
        args = [ctypes.c_uint64(d_y.value), ctypes.c_uint64(d_a.value),
                ctypes.c_uint64(d_x.value), ctypes.c_uint64(m), ctypes.c_uint64(n)]
        cu.launch(fn, grid, (BLOCK, 1, 1), args)
        cu.stream_sync()
        got = np.empty(m, dtype=np.float32)
        cu.memcpy_dtoh(got.ctypes.data, d_y, m * 4)
        expect = a.reshape(m, n).astype(np.float64) @ x.astype(np.float64)
        denom = np.maximum(np.abs(expect), 1.0)
        rel = float((np.abs(got.astype(np.float64) - expect) / denom).max())
        if rel > REL_TOL:
            raise AssertionError(f"correctness FAIL: max rel err {rel:.2e}")
        print(f"[cuda-gemv] correctness PASS ({m}x{n} · {n}, max rel err {rel:.2e})")
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

        gb = (m * n + n + m) * 4 / 1e9
        doc = run_protocol(
            bench_id="cuda_gemv_f32",
            problem_size=f"{M_MAIN}x{N_MAIN} f32 · {N_MAIN}",
            metric="effective_bandwidth",
            unit="GB/s",
            iter_ms=iter_ms,
            ms_to_metric=lambda ms: gb / (ms / 1e3),
            pre_timed=pre_timed,
            notes=f"handwritten CUDA C++ reference PTX (.version {version}, entry {entry}); "
                  "y=A·x row-major, per-thread row, memory-bound; "
                  "metric id m8.bench.gemv_cuda.effective_bandwidth_gbps",
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
