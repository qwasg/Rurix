#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-02 多 stream device 路径 sanitizer 包裹器(M8.3,D-M8-3;纳入既有 Compute Sanitizer
racecheck+memcheck nightly,M8 CI_GATES §4)。

构建并运行 uc02-demo 真实 GPU 进程(三 stream H2D/compute/D2H 重叠 + event 流序依赖 +
跨线程 DeviceBox/SharedEvent 转移);compute_sanitizer_run.py 经 `--target-processes all`
跟随本子 exe 的 device kernel / 异步拷贝。`--smoke` 闭环:GPU 真跑数值对照 ok 或无 GPU 降级
skip → return 0;数值 fail / error → 非零(供 sanitizer error-exitcode 之外的功能闭环兜底)。
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main() -> int:
    exe = ROOT / "target" / "debug" / (
        "uc02-demo.exe" if sys.platform == "win32" else "uc02-demo"
    )
    if not exe.exists():
        b = subprocess.run(["cargo", "build", "-q", "-p", "uc02-demo"], cwd=ROOT)
        if b.returncode != 0 or not exe.exists():
            print("[uc02_stream_bench] SKIP:构建 uc02-demo 失败(无工具链?)")
            return 0
    r = subprocess.run([str(exe)], cwd=ROOT, capture_output=True, text=True)
    line = next((ln for ln in (r.stdout or "").splitlines()
                 if ln.startswith("UC02_RESULT:")), r.stdout.strip())
    print(f"[uc02_stream_bench] {line}")
    if "UC02_RESULT: ok" in (r.stdout or "") or "skip" in (r.stdout or ""):
        return 0
    print((r.stderr or "")[-400:], file=sys.stderr)
    return r.returncode or 1


if __name__ == "__main__":
    sys.exit(main())
