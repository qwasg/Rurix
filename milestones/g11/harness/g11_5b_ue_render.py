#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.5b 波）
"""G11.5b 诊断变体 MRQ Phase B 出图执行器（host 侧；gpu_device_lock 串行）。

命令形态沿 G10.2 实证（同 g10_5_ue_render.py 字面；诊断面 = 参数化 map/seq/cfg）：
  UnrealEditor-Cmd.exe <proj> <map> -game -LevelSequence=<seq> -MoviePipelineConfig=<cfg>
      -windowed -resx=<w> -resy=<h> -log -notexturestreaming -Unattended -FixedSeed

用法：py -3 milestones/g11/harness/g11_5b_ue_render.py <map> <seq> <cfg> [--timeout N]
示例（G11.5b 诊断两臂）：
  py -3 milestones/g11/harness/g11_5b_ue_render.py /Game/Maps/G11_DiagBistroSky0 \
      /Game/Cinematics/G11_DiagBistroSky0Seq /Game/Cinematics/G11_DiagBistroSky0Config
  py -3 milestones/g11/harness/g11_5b_ue_render.py /Game/Maps/G10_BistroInterior \
      /Game/Cinematics/G10_bistro_interiorSeq /Game/Cinematics/G11_DiagBistroNoSpecConfig
"""
from __future__ import annotations

import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

UE_EXE = r"F:\UE_5.8\Engine\Binaries\Win64\UnrealEditor-Cmd.exe"
UPROJECT = r"K:\rurix-ext\g10-ue\G10RefRender\G10RefRender.uproject"
RES = (1920, 1080)


def main() -> int:
    if len(sys.argv) < 4:
        print("usage: g11_5b_ue_render.py <map> <seq> <cfg> [--timeout N]", file=sys.stderr)
        return 2
    map_path, seq_path, cfg_path = sys.argv[1], sys.argv[2], sys.argv[3]
    timeout = 1800
    if "--timeout" in sys.argv:
        timeout = int(sys.argv[sys.argv.index("--timeout") + 1])
    argv = [
        UE_EXE,
        UPROJECT,
        map_path,
        "-game",
        f"-LevelSequence={seq_path}",
        f"-MoviePipelineConfig={cfg_path}",
        "-windowed",
        f"-resx={RES[0]}",
        f"-resy={RES[1]}",
        "-log",
        "-notexturestreaming",
        "-Unattended",
        "-FixedSeed",
    ]
    started = time.time()
    with gpu_device_lock(purpose=f"g11.5b UE MRQ 诊断出帧 {map_path}"):
        r = subprocess.run(argv, capture_output=True, timeout=timeout)
    dur = time.time() - started
    out = (r.stdout + r.stderr).decode("utf-8", "replace")
    tail = [l for l in out.splitlines() if "MoviePipeline" in l or "Error" in l or "LogMovieRenderPipeline" in l]
    print("\n".join(tail[-40:]))
    print(f"[g11_5b_ue_render] map={map_path} exit={r.returncode} duration_s={dur:.1f} started_epoch={started:.3f}")
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
