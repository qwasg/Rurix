#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.5a 波）
"""G10.5 harness — UE 编辑器脚本执行器（host 侧；gpu_device_lock 串行）。

用法：py -3 milestones/g10/harness/g10_5_ue_run.py <ue_python_script.py> [extra args...]
脚本路径透传 -ExecutePythonScript；UE 日志 tail 打印；exit code 透传。
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

UE_EXE = r"F:\UE_5.8\Engine\Binaries\Win64\UnrealEditor-Cmd.exe"
UPROJECT = r"K:\rurix-ext\g10-ue\G10RefRender\G10RefRender.uproject"


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: g10_5_ue_run.py <script> [args...]", file=sys.stderr)
        return 2
    script = sys.argv[1]
    extra = sys.argv[2:]
    argv = [
        UE_EXE,
        UPROJECT,
        f"-ExecutePythonScript={script}",
        "-unattended",
        "-log",
        "-nopause",
    ] + extra
    with gpu_device_lock(purpose="g10.5 UE editor script run"):
        r = subprocess.run(argv, capture_output=True, timeout=1800)
    out = (r.stdout + r.stderr).decode("utf-8", "replace")
    interesting = [
        l
        for l in out.splitlines()
        if "G10_5" in l or "PROBE" in l or "Error" in l or "Traceback" in l or "Exception" in l
    ]
    print("\n".join(interesting[-120:]))
    print(f"[g10_5_ue_run] exit={r.returncode}")
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
