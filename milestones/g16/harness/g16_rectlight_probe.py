#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16.1 第一波）
"""G16 只读探针 host 编排：加载现采 G13_CornellBox，读回 G13_QuadLight_0。"""
from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

UE_EXE = r"F:\UE_5.8\Engine\Binaries\Win64\UnrealEditor-Cmd.exe"
UPROJECT = r"K:\rurix-ext\g10-ue\G10RefRender\G10RefRender.uproject"
SCRIPT = ROOT / "milestones" / "g16" / "harness" / "ue_python" / "g16_rectlight_probe.py"
OUT = Path(r"K:\rurix-ext\g13-frames\cornell-box\rectlight_probe.json")


def main() -> int:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env["G16_PROBE_OUT"] = str(OUT).replace("\\", "/")
    argv = [
        UE_EXE,
        UPROJECT,
        f"-ExecutePythonScript={SCRIPT}",
        "-unattended",
        "-log",
        "-nopause",
    ]
    print(f"[g16_probe] start out={OUT}")
    with gpu_device_lock(purpose="g16 rectlight readonly probe"):
        started = time.time()
        r = subprocess.run(argv, capture_output=True, timeout=1800, env=env)
    out = (r.stdout + r.stderr).decode("utf-8", "replace")
    hits = [ln for ln in out.splitlines() if "G16_PROBE" in ln or "Error" in ln or "LogPython" in ln]
    print("\n".join(hits[-60:]))
    print(f"[g16_probe] exit={r.returncode} duration_s={time.time() - started:.1f} probe_exists={OUT.is_file()}")
    if r.returncode != 0:
        return r.returncode
    return 0 if OUT.is_file() else 2


if __name__ == "__main__":
    sys.exit(main())
