"""Sanitizer 红绿验证夹具运行器(M5.4 G-M5-4,非生产 kernel)。

装载 race / clean 共享内存夹具 PTX 并 launch 单 block,供 compute-sanitizer
--tool racecheck 包裹:
  - race  变体(缺 bar.sync):racecheck 应检出竞争 → 红态(clean=false);
  - clean 变体(补 bar.sync):racecheck 应转绿 → 绿态(clean=true)。

本脚本本身不判定红绿(由外层 compute_sanitizer_run.py 解析 sanitizer 报告),
只负责真实把夹具 kernel 跑起来,使 racecheck 有可观测的运行期访存。

用法(通常由 compute-sanitizer 包裹):
  py -3 bench/sanitizer_fixtures/fixture_runner.py --variant race
  py -3 bench/sanitizer_fixtures/fixture_runner.py --variant clean
"""
from __future__ import annotations

import argparse
import ctypes
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from bench import cuda_driver as cu

HERE = Path(__file__).resolve().parent
BLOCK = 64  # 2 个 warp,确保跨 warp 竞争对 racecheck 可见


def parse_entry(ptx: str, path: Path) -> str:
    m = re.search(r"\.entry\s+([A-Za-z_$][A-Za-z0-9_$]*)", ptx)
    if not m:
        raise RuntimeError(f"无法从 {path} 解析 .entry kernel 名")
    return m.group(1)


def main() -> int:
    ap = argparse.ArgumentParser(description="Sanitizer 红绿夹具运行器(G-M5-4)")
    ap.add_argument("--variant", choices=["race", "clean"], required=True)
    args = ap.parse_args()

    ptx_path = HERE / ("race_shared.ptx" if args.variant == "race" else "clean_shared.ptx")
    ptx = ptx_path.read_text(encoding="utf-8")
    entry = parse_entry(ptx, ptx_path)

    with cu.Context():
        module, version, jit_log = cu.load_ptx(ptx)
        fn = cu.get_function(module, entry)
        d_out = cu.mem_alloc(BLOCK * 4)
        cu.memset_d8(d_out, 0, BLOCK * 4)
        cu.launch(fn, (1, 1, 1), (BLOCK, 1, 1), [ctypes.c_uint64(d_out.value)])
        cu.stream_sync()
        out = (ctypes.c_uint32 * BLOCK)()
        cu.memcpy_dtoh(ctypes.addressof(out), d_out, BLOCK * 4)
        print(f"[fixture:{args.variant}] launched (.version {version}, entry {entry}, "
              f"block={BLOCK})" + (f" jit: {jit_log}" if jit_log.strip() else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
