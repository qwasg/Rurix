#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.4 UE PT 对标波）
"""G12.4 harness — UE PT 对标 UE 臂 host 编排器（RXS-0403 L3 双端出图 UE 腿；
RXS-0380 L2 臂 A 命令面 + Phase B 形态沿 G10.5 实证；gpu_device_lock 串行）。

职责闭集：
  1. Phase A（建设）：逐场景 UnrealEditor-Cmd -ExecutePythonScript=
     g12_4_build_pt_scenes.py（地图/MRQ 资产配置幂等重建 + 探针 JSON 收割）；
  2. Phase B（渲染）：逐 （场景 × spp） MRQ job 真跑（-game -LevelSequence
     -MoviePipelineConfig -windowed -log -notexturestreaming -Unattended
     -FixedSeed 命令面闭集）；
  3. 收割：output_dir 内 mtime ≥ run_start 的 .exr 新鲜度机核 + EXR magic +
     体积下限 + canonical digest（g10_determinism 单源）→ 帧库落盘
     K:/rurix-ext/g12-frames/ue_pt/<scene>/spp<n>/ + 收割 receipt JSON。

用法：
  py -3 milestones/g12/harness/g12_4_ue_render.py build [scene_id|--all] [--skip-import]
  py -3 milestones/g12/harness/g12_4_ue_render.py render [scene_id|--all] [--spp N]
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
sys.path.insert(0, str(ROOT / "milestones" / "g10" / "harness" / "ue_python"))

from gpu_device_lock import gpu_device_lock  # noqa: E402
import g10_determinism as _det  # noqa: E402

UE_EXE = r"F:\UE_5.8\Engine\Binaries\Win64\UnrealEditor-Cmd.exe"
UPROJECT = r"K:\rurix-ext\g10-ue\G10RefRender\G10RefRender.uproject"
CONTRACT = ROOT / "milestones" / "g12" / "g12_ue_pt_parity_contract.json"
BUILD_SCRIPT = ROOT / "milestones" / "g12" / "harness" / "ue_python" / "g12_4_build_pt_scenes.py"
OUT_ROOT = Path(r"K:\rurix-ext\g12-frames\ue_pt")
EXR_MAGIC = b"\x76\x2f\x31\x01"
MIN_EXR_BYTES = 10_000  # 128×128 f16 NONE 实测 ≈130 KB 级;下限防合成小文件冒充

SCENES = {
    "cornell-box": {"map": "/Game/Maps/G12_PTCornellBox", "tag": "cornell_box"},
    "bistro-interior": {"map": "/Game/Maps/G12_PTBistroInterior", "tag": "bistro_interior"},
}
SPP_SEQ = [1, 4, 16, 64, 256, 1024]


def run_editor(argv, timeout_s, env_extra=None, purpose="g12.4 ue"):
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    with gpu_device_lock(purpose=purpose):
        started = time.time()
        r = subprocess.run(argv, capture_output=True, timeout=timeout_s, env=env)
    out = (r.stdout + r.stderr).decode("utf-8", "replace")
    return started, r.returncode, out


def build(scene_id, skip_import=False):
    probe_out = OUT_ROOT / scene_id / "build_probe.json"
    probe_out.parent.mkdir(parents=True, exist_ok=True)
    env = {
        "G12_4_SCENE": scene_id,
        "G12_4_CONTRACT": str(CONTRACT),
        "G12_4_PROBE_OUT": str(probe_out).replace("\\", "/"),
        "G12_4_OUT_ROOT": str(OUT_ROOT).replace("\\", "/"),
    }
    if skip_import:
        env["G12_4_SKIP_IMPORT"] = "1"
    argv = [UE_EXE, UPROJECT, f"-ExecutePythonScript={BUILD_SCRIPT}", "-unattended", "-log", "-nopause"]
    started, rc, out = run_editor(argv, 5400, env, purpose=f"g12.4 UE PT 建设 {scene_id}")
    lines = [l for l in out.splitlines() if "G12_4_BUILD" in l or "Error" in l or "LogPython" in l]
    print("\n".join(lines[-30:]))
    print(f"[g12_4_ue_render] build scene={scene_id} exit={rc} duration_s={time.time()-started:.1f}")
    if rc != 0:
        return False
    if not probe_out.is_file():
        print(f"[g12_4_ue_render] build probe 缺失: {probe_out}")
        return False
    return True


def render(scene_id, spp):
    s = SCENES[scene_id]
    cfg = f"/Game/Cinematics/G12_{s['tag']}_spp{spp}_Config"
    seq = f"/Game/Cinematics/G12_{s['tag']}_Seq"
    argv = [
        UE_EXE,
        UPROJECT,
        s["map"],
        "-game",
        f"-LevelSequence={seq}",
        f"-MoviePipelineConfig={cfg}",
        "-windowed",
        "-resx=128",
        "-resy=128",
        "-log",
        "-notexturestreaming",
        "-Unattended",
        "-FixedSeed",
    ]
    started, rc, out = run_editor(argv, 7200, purpose=f"g12.4 UE PT MRQ {scene_id} spp{spp}")
    tail = [l for l in out.splitlines() if "MoviePipeline" in l or "PathTrac" in l or "Error" in l]
    print("\n".join(tail[-25:]))
    dur = time.time() - started
    # 收割（新鲜度机核）
    out_dir = OUT_ROOT / scene_id / f"spp{spp}"
    frames = []
    if out_dir.is_dir():
        for f in sorted(out_dir.rglob("*.exr")):
            if f.stat().st_mtime < started:
                continue
            blob = f.read_bytes()
            frames.append(
                {
                    "name": f.name,
                    "bytes": len(blob),
                    "exr_magic_ok": blob[:4] == EXR_MAGIC,
                    "canonical_digest": _det.exr_canonical_digest(str(f)),
                }
            )
    receipt = {
        "scene_id": scene_id,
        "spp": spp,
        "config": cfg,
        "exit_code": rc,
        "duration_s": round(dur, 3),
        "started_epoch": started,
        "frames": frames,
        "output_tail": out[-1500:],
    }
    rp = OUT_ROOT / scene_id / f"spp{spp}" / "render_receipt.json"
    rp.parent.mkdir(parents=True, exist_ok=True)
    rp.write_text(json.dumps(receipt, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    ok = rc == 0 and frames and all(
        f["exr_magic_ok"] and f["bytes"] >= MIN_EXR_BYTES for f in frames
    )
    print(
        f"[g12_4_ue_render] render scene={scene_id} spp={spp} exit={rc} "
        f"frames={len(frames)} ok={bool(ok)} duration_s={dur:.1f}"
    )
    return bool(ok)


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    mode = sys.argv[1]
    target = sys.argv[2]
    skip_import = "--skip-import" in sys.argv
    spp_only = None
    if "--spp" in sys.argv:
        spp_only = int(sys.argv[sys.argv.index("--spp") + 1])
    scenes = list(SCENES) if target == "--all" else [target]
    if mode == "build":
        ok = all(build(s, skip_import) for s in scenes)
        return 0 if ok else 1
    if mode == "render":
        ok = True
        for s in scenes:
            for spp in SPP_SEQ:
                if spp_only is not None and spp != spp_only:
                    continue
                ok = render(s, spp) and ok
        return 0 if ok else 1
    print("未知模式: " + mode)
    return 2


if __name__ == "__main__":
    sys.exit(main())
