#!/usr/bin/env python3
"""G38 GPU 批次 2 执行器(主 agent 锁内跑;T5 RIS/NEE A/B + lamp-k 阶梯)。

四步:run_ab(8 GPU 跑)→ judge_ab → run_kladder(6 档)→ judge_kladder2。
T5 脚本自身不管锁(设计约定),本包装器持 gpu_device_lock 全程。
任一步 rc≠0 即停(fail-closed),judge 产物路径打印供主 agent 判读。
"""

from __future__ import annotations

import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
T5 = ROOT / "artifacts" / "day_0830_g38" / "t5_risnee"
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

STEPS = [
    ("run_ab", [sys.executable, "-X", "utf8", str(T5 / "run_ab.py")]),
    ("judge_ab", [sys.executable, "-X", "utf8", str(T5 / "judge_ab.py")]),
    ("run_kladder", [sys.executable, "-X", "utf8", str(T5 / "run_kladder.py")]),
    ("judge_kladder2", [sys.executable, "-X", "utf8", str(T5 / "judge_kladder2.py")]),
]


def main() -> int:
    with gpu_device_lock(purpose="G38 批次2(RIS/NEE A/B + lamp-k 阶梯)"):
        for name, cmd in STEPS:
            t0 = time.time()
            print(f"[batch2] {name} BEGIN", flush=True)
            p = subprocess.run(cmd, cwd=str(ROOT))
            dt = round(time.time() - t0, 1)
            print(f"[batch2] {name} rc={p.returncode} wall_s={dt}", flush=True)
            if p.returncode != 0:
                print(f"BATCH2 FAIL at {name}")
                return 1
    print("BATCH2 PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
