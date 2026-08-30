#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""诊断:屏幕中央"两个三角形模糊区"消融定位(GPU 锁内)。

三臂对照(全显式十六臂形态,静态契约相机 = 交互窗口初始位姿):
  d_full16   全开(基线,现行 full 展开等价)
  d_no_refl  关 --rt-reflect(反射单样本噪声嫌疑)
  d_no_nrm   关 --normal-maps(逐三角切线不连续嫌疑)
各 96f + 末帧 presented raw dump。离线:中央 crop 逐对 diff 统计 + 帧间
噪声口径(f0080 vs 末帧)——定位模糊区归属臂。
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
from gpu_device_lock import gpu_device_lock  # noqa: E402

WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
F = ROOT / "artifacts" / "day_0829_realism" / "final"
PNG = F / "png_triage"

BASE = [
    "--smooth-normals", "on", "--ggx", "on",
    "--lamp-lights", "on", "--lamp-gain", "4",
    "--textures", "on", "--bloom", "on", "--dither", "on",
    "--auto-exposure", "on", "--tsr-quality", "on",
    "--gi2", "on", "--gi2-clamp", "0.01", "--emissive-tex", "on",
]
SIX = ["--metal-f0", "on", "--rt-ao", "on", "--soft-shadows", "on",
       "--soft-shadow-samples", "1", "--rt-reflect", "on", "--gi2-tex", "on",
       "--normal-maps", "on"]


def drop(flags: list[str], name: str) -> list[str]:
    out = []
    i = 0
    while i < len(flags):
        if flags[i] == name:
            i += 2
            continue
        out.append(flags[i])
        i += 1
    return out


ARMS = [
    ("d_full16", SIX),
    ("d_no_refl", drop(SIX, "--rt-reflect")),
    ("d_no_nrm", drop(SIX, "--normal-maps")),
]


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_G18_AMBIENT"] = "0.004"
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def main() -> int:
    PNG.mkdir(parents=True, exist_ok=True)
    rows = []
    with gpu_device_lock(purpose="day0829 blur triage dumps", timeout_s=7200.0):
        for tag, six in ARMS:
            ev = PNG / f"{tag}.json"
            cmd = [str(WIN), "--frames", "96", "--warmup", "2", "--hidden",
                   *BASE, *six,
                   "--dump-present-raw", str(PNG / f"{tag}.raw"),
                   "--dump-present-every", "80",
                   "--evidence", str(ev)]
            t0 = time.time()
            r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                               timeout=1800, env=env_of())
            row = {"step": tag, "rc": r.returncode,
                   "vuid": (r.stderr or "").count("VUID-"),
                   "wall_s": round(time.time() - t0, 1)}
            if r.returncode != 0:
                row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-6:]
            rows.append(row)
            print(json.dumps(row, ensure_ascii=False), flush=True)
    (PNG / "TRIAGE_RUNS.json").write_text(
        json.dumps(rows, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("TRIAGE done")
    return 0


if __name__ == "__main__":
    sys.exit(main())
