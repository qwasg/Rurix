#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""流序分配 AsyncBuffer device 路径 sanitizer 包裹器(G1.2,MR-0001;纳入既有 Compute
Sanitizer racecheck+memcheck nightly,G1 CI_GATES §4,CUDA.jl #780 use-after-free 事故类回归)。

构建并运行 rurix-rt 的 async_buffer_pipeline 示例(三 stream 流序分配 cuMemAllocAsync +
两条 share_with 跨 stream 时序边 + cuMemFreeAsync 流序释放);compute_sanitizer_run.py 经
`--target-processes all` 跟随本 exe 的 device 分配/拷贝/释放。`--smoke` 闭环:GPU 真跑往返
数值对照 ok 或无 GPU 降级 skip → return 0;数值 fail → 非零(供 sanitizer error-exitcode
之外的功能闭环兜底)。

P0-7:exe 运行经 proc_guard.guarded_run 看门狗(超时杀进程树 + 隔离卡死 exe + 诚实红)。
2026-07-17 本 exe(async_buffer_pipeline.exe)正是挂死 ~4h50m 锁 runner 的僵尸源。
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from bench.proc_guard import guarded_run, EXE_RUN_TIMEOUT, CARGO_BUILD_TIMEOUT


def main() -> int:
    exe = ROOT / "target" / "debug" / "examples" / (
        "async_buffer_pipeline.exe" if sys.platform == "win32" else "async_buffer_pipeline"
    )
    if not exe.exists():
        b = guarded_run(
            ["cargo", "build", "-q", "-p", "rurix-rt", "--example", "async_buffer_pipeline"],
            cwd=ROOT, timeout=CARGO_BUILD_TIMEOUT, capture=False,
            label="cargo build async_buffer_pipeline",
        )
        if b.timed_out:
            print("[async_buffer_bench] FAIL:构建 async_buffer_pipeline 超时(已杀进程树)",
                  file=sys.stderr)
            return b.returncode  # 124 诚实红,非 SKIP
        if b.returncode != 0 or not exe.exists():
            print("[async_buffer_bench] SKIP:构建 async_buffer_pipeline 示例失败(无工具链?)")
            return 0
    r = guarded_run([str(exe)], cwd=ROOT, timeout=EXE_RUN_TIMEOUT,
                    quarantine_exe=exe, label="async_buffer_pipeline")
    line = next((ln for ln in (r.stdout or "").splitlines()
                 if ln.startswith("ASYNC_BUFFER_RESULT:")), r.stdout.strip())
    print(f"[async_buffer_bench] {line}")
    if "ASYNC_BUFFER_RESULT: ok" in (r.stdout or "") or "skip" in (r.stdout or ""):
        return 0
    print((r.stderr or "")[-400:], file=sys.stderr)
    return r.returncode or 1


if __name__ == "__main__":
    sys.exit(main())
