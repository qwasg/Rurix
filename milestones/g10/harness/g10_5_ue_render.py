#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.5a 波）
"""G10.5 harness — UE 侧 Phase B MRQ 出图执行器（host 侧；gpu_device_lock 串行）。

命令形态沿 G10.2 实证（spec/external_reference.md RXS-0380 L2 臂 A Phase B）：
  UnrealEditor-Cmd.exe <proj> <map> -game -LevelSequence=<seq> -MoviePipelineConfig=<cfg>
      -windowed -resx=<w> -resy=<h> -log -notexturestreaming -Unattended -FixedSeed

用法：py -3 milestones/g10/harness/g10_5_ue_render.py <scene_id> [--timeout N]
scene_id ∈ {cornell-box, bistro-interior}；出帧目录 K:/rurix-ext/g10-frames/g10_5/ue/<scene_id>/。
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

SCENES = {
    "cornell-box": {
        "map": "/Game/Maps/G10_CornellBox",
        "seq": "/Game/Cinematics/G10_cornell_boxSeq",
        "cfg": "/Game/Cinematics/G10_cornell_boxConfig",
        "res": (512, 512),
    },
    "bistro-interior": {
        "map": "/Game/Maps/G10_BistroInterior",
        "seq": "/Game/Cinematics/G10_bistro_interiorSeq",
        "cfg": "/Game/Cinematics/G10_bistro_interiorConfig",
        "res": (1920, 1080),
    },
}


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] not in SCENES:
        print("usage: g10_5_ue_render.py <cornell-box|bistro-interior> [--timeout N]", file=sys.stderr)
        return 2
    scene_id = sys.argv[1]
    timeout = 1800
    if "--timeout" in sys.argv:
        timeout = int(sys.argv[sys.argv.index("--timeout") + 1])
    s = SCENES[scene_id]
    argv = [
        UE_EXE,
        UPROJECT,
        s["map"],
        "-game",
        f"-LevelSequence={s['seq']}",
        f"-MoviePipelineConfig={s['cfg']}",
        "-windowed",
        f"-resx={s['res'][0]}",
        f"-resy={s['res'][1]}",
        "-log",
        "-notexturestreaming",
        "-Unattended",
        "-FixedSeed",
    ]
    started = time.time()
    with gpu_device_lock(purpose=f"g10.5 UE MRQ Phase B 出帧 {scene_id}"):
        r = subprocess.run(argv, capture_output=True, timeout=timeout)
    dur = time.time() - started
    out = (r.stdout + r.stderr).decode("utf-8", "replace")
    tail = [l for l in out.splitlines() if "MoviePipeline" in l or "Error" in l or "LogMovieRenderPipeline" in l]
    print("\n".join(tail[-40:]))
    print(f"[g10_5_ue_render] scene={scene_id} exit={r.returncode} duration_s={dur:.1f} started_epoch={started:.3f}")
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
