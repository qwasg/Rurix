"""bandwidthTest 等价 harness(M0 交付物 D-M0-3,BENCH_PROTOCOL.md §4.2)。

方向:h2d_pinned / h2d_pageable / d2h_pinned / d2h_pageable / d2d,256MB 主档。

用法:
  py -3 bench/bandwidth_bench.py --direction d2d --emit evidence/x.json
  py -3 bench/bandwidth_bench.py --direction all --emit-dir evidence --seq 1
"""
from __future__ import annotations

import ctypes
import datetime
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import cuda_driver as cu
from bench.protocol import ROOT, run_protocol, write_evidence

SIZE_MB = 256
NBYTES = SIZE_MB * 1024 * 1024
DIRECTIONS = ("h2d_pinned", "h2d_pageable", "d2h_pinned", "d2h_pageable", "d2d")


def bench_direction(direction: str) -> dict:
    keep_alive = []
    d_src = cu.mem_alloc(NBYTES)
    cu.memset_d8(d_src, 7, NBYTES)

    if direction == "d2d":
        d_dst = cu.mem_alloc(NBYTES)

        def copy() -> None:
            cu.memcpy_dtod(d_dst, d_src, NBYTES)
    else:
        pinned = direction.endswith("pinned")
        if pinned:
            h_ptr_obj = cu.mem_alloc_host(NBYTES)
            h_ptr = h_ptr_obj.value
        else:
            h_buf = ctypes.create_string_buffer(NBYTES)
            keep_alive.append(h_buf)
            h_ptr = ctypes.addressof(h_buf)
        if direction.startswith("h2d"):
            def copy() -> None:
                cu.memcpy_htod(d_src, h_ptr, NBYTES)
        else:
            def copy() -> None:
                cu.memcpy_dtoh(h_ptr, d_src, NBYTES)

    events = cu.EventPair()

    def iter_ms() -> float:
        cu.stream_sync()
        events.record_start()
        copy()
        events.record_stop()
        cu.stream_sync()
        return events.elapsed_ms()

    # D2D 计双向字节(读 + 写),与 NVIDIA bandwidthTest 参考工具的计量约定一致
    gb = (2 * NBYTES if direction == "d2d" else NBYTES) / 1e9
    return run_protocol(
        bench_id=f"bandwidth_{direction}",
        problem_size=f"{SIZE_MB}MB",
        metric="copy_bandwidth",
        unit="GB/s",
        iter_ms=iter_ms,
        ms_to_metric=lambda ms: gb / (ms / 1e3),
        notes=f"bandwidthTest-equivalent, direction={direction}, Driver API memcpy, "
              "CUDA Event timing with pre/post stream sync (WDDM batch flush)",
    )


def main() -> int:
    args = sys.argv[1:]
    direction = args[args.index("--direction") + 1] if "--direction" in args else "all"
    seq = args[args.index("--seq") + 1] if "--seq" in args else "1"
    emit = args[args.index("--emit") + 1] if "--emit" in args else None
    emit_dir = args[args.index("--emit-dir") + 1] if "--emit-dir" in args else None

    targets = DIRECTIONS if direction == "all" else (direction,)
    date = datetime.date.today().strftime("%Y%m%d")
    with cu.Context():
        for d in targets:
            doc = bench_direction(d)
            if emit and len(targets) == 1:
                write_evidence(doc, ROOT / emit)
            elif emit_dir:
                write_evidence(doc, ROOT / emit_dir / f"bandwidth_{d}_{date}_{seq}.json")
            else:
                print(f"{d}: {doc['results']['trimmed_mean']} GB/s "
                      f"(ci95 {doc['results']['ci95']}, level={doc['evidence_level']})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
