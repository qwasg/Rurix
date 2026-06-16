"""cublas SGEMM 绑定基准 harness(M8.2,契约 G-M8-2;BENCH_PROTOCOL.md §3)。

measure cublas GEMM kernel 吞吐(2*M*N*K/t,GFLOPS),作为 rurix-cublas 绑定 kernel
的性能事实(对照手写 CUDA C++ tiled GEMM,≥90%,01 §6 UC-01 判据)。1024³ 与
bench/gemm_tile_cuda_bench.py 同档(共享 denominator m8.bench.gemm_cuda)。

cublas 句柄一次创建于 timed loop 外(仅计 cublasSgemm kernel 时间);行主序 ↔ 列主序
适配同 src/rurix-cublas(RXS-0128)。cublas runtime DLL = Attachment A 白名单(RXS-0129)。

用法:
  py -3 bench/cublas_gemm_bench.py --smoke
  py -3 bench/cublas_gemm_bench.py --emit evidence/x.json
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import cublas_driver as cb
from bench import cuda_driver as cu
from bench.protocol import L2_CLEAR_MB, ROOT, run_protocol, write_evidence

M_MAIN = N_MAIN = K_MAIN = 1024
M_SMOKE, N_SMOKE, K_SMOKE = 100, 80, 70
REL_TOL = 1e-3


def main() -> int:
    smoke = "--smoke" in sys.argv
    emit = ROOT / sys.argv[sys.argv.index("--emit") + 1] if "--emit" in sys.argv else None
    m, n, k = (M_SMOKE, N_SMOKE, K_SMOKE) if smoke else (M_MAIN, N_MAIN, K_MAIN)

    with cu.Context():
        with cb.Handle() as h:
            print(f"[cublas-gemm] cublas handle created ({cb.loaded_dll_name()})")
            rng = np.random.default_rng(20260616)
            a = (rng.integers(0, 7, size=m * k).astype(np.float32)) * np.float32(0.1) + np.float32(0.05)
            b = (rng.integers(0, 5, size=k * n).astype(np.float32)) * np.float32(0.2) + np.float32(0.1)
            d_a = cu.mem_alloc(m * k * 4)
            d_b = cu.mem_alloc(k * n * 4)
            d_c = cu.mem_alloc(m * n * 4)
            cu.memcpy_htod(d_a, a.ctypes.data, m * k * 4)
            cu.memcpy_htod(d_b, b.ctypes.data, k * n * 4)

            st = cb.sgemm_row_major(h, d_c.value, d_a.value, d_b.value, m, n, k)
            cu.stream_sync()
            if st != cb.CUBLAS_STATUS_SUCCESS:
                raise RuntimeError(f"cublasSgemm 返回 status {st}")
            got = np.empty(m * n, dtype=np.float32)
            cu.memcpy_dtoh(got.ctypes.data, d_c, m * n * 4)
            expect = (a.reshape(m, k).astype(np.float64) @ b.reshape(k, n).astype(np.float64)).reshape(m * n)
            denom = np.maximum(np.abs(expect), 1.0)
            rel = np.abs(got.astype(np.float64) - expect) / denom
            if rel.max() > REL_TOL:
                raise AssertionError(f"correctness FAIL: max rel err {rel.max():.2e}")
            print(f"[cublas-gemm] correctness PASS ({m}x{k} * {k}x{n}, max rel err {rel.max():.2e})")
            if smoke:
                return 0

            l2_buf = cu.mem_alloc(L2_CLEAR_MB * 1024 * 1024)
            events = cu.EventPair()

            def pre_timed() -> None:
                cu.memset_d8(l2_buf, 0, L2_CLEAR_MB * 1024 * 1024)

            def iter_ms() -> float:
                cu.stream_sync()
                events.record_start()
                cb.sgemm_row_major(h, d_c.value, d_a.value, d_b.value, m, n, k)
                events.record_stop()
                cu.stream_sync()
                return events.elapsed_ms()

            gflop = 2.0 * m * n * k / 1e9
            doc = run_protocol(
                bench_id="cublas_gemm_f32",
                problem_size=f"{M_MAIN}x{K_MAIN} * {K_MAIN}x{N_MAIN} f32",
                metric="throughput",
                unit="GFLOPS",
                iter_ms=iter_ms,
                ms_to_metric=lambda ms: gflop / (ms / 1e3),
                pre_timed=pre_timed,
                notes=f"cublas SGEMM binding ({cb.loaded_dll_name()}, Attachment A whitelist); "
                      "row-major via cublasSgemm(OP_N,OP_N,n,m,k,B,n,A,k,C,n); "
                      "metric id m8.bench.cublas_gemm.throughput_gflops",
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
