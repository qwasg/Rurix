"""手写 CUDA C++ reduce 对照基准 harness(M5.3 WP8;RD-002 承接,BENCH_PROTOCOL.md §3)。

kernel:`bench/cuda_ref/reduce.cu`(block 级 shared 树形归约,atomics-free)
→ `bench/kernels/cuda_reduce.ptx`。有效带宽 = N * sizeof(f32) / t。

PTX 再生成:
  py -3 bench/compile_cuda_ref.py

用法:
  py -3 bench/reduce_cuda_bench.py --smoke
  py -3 bench/reduce_cuda_bench.py --emit evidence/x.json
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
REL_TOL = 1e-3
KERNEL = "reduce"


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
    n = N_SMOKE if smoke else N_MAIN
    nblocks = (n + BLOCK - 1) // BLOCK

    ptx_path, ptx = read_cuda_ptx(KERNEL)
    entry = parse_entry(ptx, ptx_path)
    with cu.Context():
        module, version, jit_log = cu.load_ptx(ptx)
        print(f"[cuda-reduce] PTX loaded (.version {version}, entry {entry})")
        fn = cu.get_function(module, entry)

        rng = np.random.default_rng(20260614)
        src = (rng.integers(0, 13, size=n).astype(np.float32)) * np.float32(0.25)
        d_src = cu.mem_alloc(n * 4)
        d_part = cu.mem_alloc(nblocks * 4)
        cu.memcpy_htod(d_src, src.ctypes.data, n * 4)

        grid = (nblocks, 1, 1)
        args = [ctypes.c_uint64(d_src.value), ctypes.c_uint64(d_part.value), ctypes.c_uint64(n)]
        cu.launch(fn, grid, (BLOCK, 1, 1), args)
        cu.stream_sync()
        partials = np.empty(nblocks, dtype=np.float32)
        cu.memcpy_dtoh(partials.ctypes.data, d_part, nblocks * 4)
        got = float(partials.astype(np.float64).sum())
        expect = float(src.astype(np.float64).sum())
        denom = max(abs(expect), 1.0)
        if abs(got - expect) / denom > REL_TOL:
            raise AssertionError(f"correctness FAIL: got {got} expect {expect}")
        print(f"[cuda-reduce] correctness PASS (n=2^{n.bit_length() - 1}, sum={got:.1f} ref={expect:.1f})")
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

        gb = n * 4 / 1e9
        doc = run_protocol(
            bench_id="cuda_reduce_f32",
            problem_size=f"2^24 elements f32 ({n * 4 // (1024 * 1024)} MiB)",
            metric="effective_bandwidth",
            unit="GB/s",
            iter_ms=iter_ms,
            ms_to_metric=lambda ms: gb / (ms / 1e3),
            pre_timed=pre_timed,
            notes=f"handwritten CUDA C++ reference PTX (.version {version}, entry {entry}); "
                  "block-level shared tree reduction, atomics-free, host-combined partials; "
                  "metric id m5.bench.reduce_cuda.effective_bandwidth_gbps",
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
