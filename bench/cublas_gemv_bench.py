"""cublas SGEMV 绑定基准 harness(M8.2,契约 G-M8-2;BENCH_PROTOCOL.md §3)。

measure cublas GEMV kernel 有效带宽(GEMV 访存受限;读 A(M*N)+x(N)、写 y(M),
带宽 = (M*N + N + M)*4 / t,GB/s),作为 rurix-cublas 绑定 kernel 的性能事实
(对照手写 CUDA C++ gemv,≥90%,01 §6 UC-01 判据)。8192² 与 bench/gemv_cuda_bench.py
同档(共享 denominator m8.bench.gemv_cuda)。

cublas 句柄一次创建于 timed loop 外;行主序经 cublasSgemv(OP_T) 适配(RXS-0128)。

用法:
  py -3 bench/cublas_gemv_bench.py --smoke
  py -3 bench/cublas_gemv_bench.py --emit evidence/x.json
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import cublas_driver as cb
from bench import cuda_driver as cu
from bench.protocol import L2_CLEAR_MB, ROOT, run_protocol, write_evidence

M_MAIN = N_MAIN = 8192
M_SMOKE, N_SMOKE = 160, 128
REL_TOL = 1e-3


def main() -> int:
    smoke = "--smoke" in sys.argv
    emit = ROOT / sys.argv[sys.argv.index("--emit") + 1] if "--emit" in sys.argv else None
    m, n = (M_SMOKE, N_SMOKE) if smoke else (M_MAIN, N_MAIN)

    with cu.Context():
        with cb.Handle() as h:
            print(f"[cublas-gemv] cublas handle created ({cb.loaded_dll_name()})")
            rng = np.random.default_rng(20260616)
            a = (rng.integers(0, 7, size=m * n).astype(np.float32)) * np.float32(0.1) + np.float32(0.05)
            x = (rng.integers(0, 5, size=n).astype(np.float32)) * np.float32(0.2) + np.float32(0.1)
            d_a = cu.mem_alloc(m * n * 4)
            d_x = cu.mem_alloc(n * 4)
            d_y = cu.mem_alloc(m * 4)
            cu.memcpy_htod(d_a, a.ctypes.data, m * n * 4)
            cu.memcpy_htod(d_x, x.ctypes.data, n * 4)

            st = cb.sgemv_row_major(h, d_y.value, d_a.value, d_x.value, m, n)
            cu.stream_sync()
            if st != cb.CUBLAS_STATUS_SUCCESS:
                raise RuntimeError(f"cublasSgemv 返回 status {st}")
            got = np.empty(m, dtype=np.float32)
            cu.memcpy_dtoh(got.ctypes.data, d_y, m * 4)
            expect = a.reshape(m, n).astype(np.float64) @ x.astype(np.float64)
            denom = np.maximum(np.abs(expect), 1.0)
            rel = np.abs(got.astype(np.float64) - expect) / denom
            if rel.max() > REL_TOL:
                raise AssertionError(f"correctness FAIL: max rel err {rel.max():.2e}")
            print(f"[cublas-gemv] correctness PASS ({m}x{n} · {n}, max rel err {rel.max():.2e})")
            if smoke:
                return 0

            l2_buf = cu.mem_alloc(L2_CLEAR_MB * 1024 * 1024)
            events = cu.EventPair()

            def pre_timed() -> None:
                cu.memset_d8(l2_buf, 0, L2_CLEAR_MB * 1024 * 1024)

            def iter_ms() -> float:
                cu.stream_sync()
                events.record_start()
                cb.sgemv_row_major(h, d_y.value, d_a.value, d_x.value, m, n)
                events.record_stop()
                cu.stream_sync()
                return events.elapsed_ms()

            gb = (m * n + n + m) * 4 / 1e9  # 读 A(M*N)+x(N)、写 y(M)
            doc = run_protocol(
                bench_id="cublas_gemv_f32",
                problem_size=f"{M_MAIN}x{N_MAIN} f32 · {N_MAIN}",
                metric="effective_bandwidth",
                unit="GB/s",
                iter_ms=iter_ms,
                ms_to_metric=lambda ms: gb / (ms / 1e3),
                pre_timed=pre_timed,
                notes=f"cublas SGEMV binding ({cb.loaded_dll_name()}, Attachment A whitelist); "
                      "row-major via cublasSgemv(OP_T,n,m,A,n,x,1,y,1); memory-bound; "
                      "metric id m8.bench.cublas_gemv.effective_bandwidth_gbps",
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
