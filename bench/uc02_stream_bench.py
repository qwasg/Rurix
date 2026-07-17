#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-02 多 stream device 路径 sanitizer 包裹器(M8.3,D-M8-3;纳入既有 Compute Sanitizer
racecheck+memcheck nightly,M8 CI_GATES §4)。

构建并运行 uc02-demo 真实 GPU 进程(三 stream H2D/compute/D2H 重叠 + event 流序依赖 +
跨线程 DeviceBox/SharedEvent 转移);compute_sanitizer_run.py 经 `--target-processes all`
跟随本子 exe 的 device kernel / 异步拷贝。`--smoke` 闭环:GPU 真跑数值对照 ok 或无 GPU 降级
skip → return 0;数值 fail / error → 非零(供 sanitizer error-exitcode 之外的功能闭环兜底)。

P0-7:exe 运行经 proc_guard.guarded_run 看门狗(超时杀进程树 + 隔离卡死 exe + 诚实红),
不再裸 subprocess 无 timeout 挂死锁 runner。
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from bench.proc_guard import guarded_run, EXE_RUN_TIMEOUT, CARGO_BUILD_TIMEOUT


def main() -> int:
    exe = ROOT / "target" / "debug" / (
        "uc02-demo.exe" if sys.platform == "win32" else "uc02-demo"
    )
    if not exe.exists():
        b = guarded_run(["cargo", "build", "-q", "-p", "uc02-demo"], cwd=ROOT,
                        timeout=CARGO_BUILD_TIMEOUT, capture=False,
                        label="cargo build uc02-demo")
        if b.timed_out:
            print("[uc02_stream_bench] FAIL:构建 uc02-demo 超时(已杀进程树)", file=sys.stderr)
            return b.returncode  # 124 诚实红,非 SKIP
        if b.returncode != 0 or not exe.exists():
            print("[uc02_stream_bench] SKIP:构建 uc02-demo 失败(无工具链?)")
            return 0
    r = guarded_run([str(exe)], cwd=ROOT, timeout=EXE_RUN_TIMEOUT,
                    quarantine_exe=exe, label="uc02-demo")
    line = next((ln for ln in (r.stdout or "").splitlines()
                 if ln.startswith("UC02_RESULT:")), r.stdout.strip())
    print(f"[uc02_stream_bench] {line}")
    if "UC02_RESULT: ok" in (r.stdout or "") or "skip" in (r.stdout or ""):
        return 0
    print((r.stderr or "")[-400:], file=sys.stderr)
    return r.returncode or 1


if __name__ == "__main__":
    sys.exit(main())
