"""M5.4 基准采样编排入口(契约 G-M5-1,BENCH_PROTOCOL.md §3)。

把 M5.4 安全并行硬证据的完整采样链串成一个操作者入口:

  1. 锁频前置闸门(require_locked;未锁频 fail-fast,unlocked 证据不得回填,§2.1);
  2. CUDA C++ 对照分母三次运行(cuda_ref_triple)→ 回填 m5.bench.*_cuda.*;
  3. 自研 kernel 分子三次运行(rurix_{reduce,scan,gemm_tile}_triple)→ 回填分子 + ratio;
  4. 严格预算核验(budget_eval --strict;三条 ratio ≥0.90)。

**前置**:管理员已 py -3 bench/lock_clocks.py --lock。本脚本不替操作者锁频(锁频需仰角),
只做 fail-fast 闸门;子步骤各自再过 require_locked() 二道防线。

用法(锁频后):
  py -3 bench/m5_bench_all.py            # 跑分母→分子→strict 核验
  py -3 bench/m5_bench_all.py --no-eval  # 跳过最后的 budget_eval --strict
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import lock_clocks

ROOT = Path(__file__).resolve().parent.parent

# 顺序固定:分母先于分子(rurix_*_triple 依赖分母已 measured_local)
NUMERATOR_SCRIPTS = (
    "bench/rurix_reduce_triple.py",
    "bench/rurix_scan_triple.py",
    "bench/rurix_gemm_tile_triple.py",
)


def run_step(cmd: list[str]) -> None:
    print(f"\n[m5_bench_all] >>> {' '.join(cmd)}")
    if subprocess.run(cmd, cwd=ROOT).returncode != 0:
        raise RuntimeError(f"步骤失败: {' '.join(cmd)}")


def main() -> int:
    lock_clocks.require_locked()  # fail-fast:未锁频不开跑(契约 G-M5-1)
    py = sys.executable

    run_step([py, "bench/cuda_ref_triple.py"])           # 分母
    for script in NUMERATOR_SCRIPTS:                       # 分子 + ratio
        run_step([py, script])

    if "--no-eval" not in sys.argv:
        run_step([py, "ci/budget_eval.py", "--strict"])   # 三条 ratio ≥0.90
    print("\n[m5_bench_all] 完成:M5.4 基准采样 + 回填 + 核验链全过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
