"""Resolve Rurix kernel PTX for bench harness(M5.3 review fix:单源化)。

优先级:
1. 环境变量 `RURIX_BENCH_PTX_DIR/{name}.ptx`
2. `target/**/build/rurix-rt-*/out/{name}.ptx`(最新 mtime,`cargo build -p rurix-rt`)
3. 仓内 `bench/kernels/rurix_{name}.ptx` 或 `bench/kernels/{name}.ptx`
"""
from __future__ import annotations

import os
from pathlib import Path

from bench.protocol import ROOT


def resolve_ptx(name: str) -> Path:
    """`name` = reduce / scan / transpose / gemm_tile / saxpy 等干名。"""
    env_dir = os.environ.get("RURIX_BENCH_PTX_DIR")
    if env_dir:
        p = Path(env_dir) / f"{name}.ptx"
        if p.is_file():
            return p

    target_root = ROOT / "target"
    if target_root.is_dir():
        candidates = list(target_root.glob(f"**/build/rurix-rt-*/out/{name}.ptx"))
        if candidates:
            return max(candidates, key=lambda p: p.stat().st_mtime)

    for rel in (
        f"bench/kernels/rurix_{name}.ptx",
        f"bench/kernels/{name}.ptx",
    ):
        p = ROOT / rel
        if p.is_file():
            return p

    raise FileNotFoundError(
        f"找不到 kernel PTX '{name}': 设置 RURIX_BENCH_PTX_DIR 或运行 "
        f"'cargo build -p rurix-rt' / 'py -3 bench/sync_bench_ptx.py'"
    )


def read_ptx(name: str) -> tuple[Path, str]:
    path = resolve_ptx(name)
    return path, path.read_text(encoding="utf-8")
