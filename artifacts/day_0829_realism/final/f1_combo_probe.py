#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""终局 F1:全臂组合帧时实测定档(GPU 锁内)。

臂⑤单臂 +3.9ms 踩 90fps 线 ⇒ 组合档位实测:
  combo_s2   full + 六臂默认(soft samples 2)
  combo_s1   full + 六臂 + --soft-shadow-samples 1
  combo_ao1  full + 六臂 + soft 1 + --rt-ao-samples 1(再降档备选)
判定:real_render_frame_ms ≤ 11.11 的最高档 = 终局默认档;全超 = 裁臂决策。
双跑位级留给 F2 重锚腿;本腿单跑定档 + VUID=0。
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
EV = F / "ev"

SIX = ["--metal-f0", "on", "--rt-ao", "on", "--soft-shadows", "on",
       "--rt-reflect", "on", "--gi2-tex", "on", "--normal-maps", "on"]

ARMS = [
    ("combo_s2", ["--quality", "full", *SIX]),
    ("combo_s1", ["--quality", "full", *SIX, "--soft-shadow-samples", "1"]),
    ("combo_ao1", ["--quality", "full", *SIX, "--soft-shadow-samples", "1", "--rt-ao-samples", "1"]),
]


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def main() -> int:
    EV.mkdir(parents=True, exist_ok=True)
    rows: list[dict] = []
    fails = 0
    with gpu_device_lock(purpose="day0829 realism final combo probe", timeout_s=7200.0):
        for tag, extra in ARMS:
            ev = EV / f"{tag}.json"
            cmd = [str(WIN), "--frames", "96", "--warmup", "2", "--hidden",
                   *extra, "--evidence", str(ev)]
            t0 = time.time()
            r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                               timeout=1800, env=env_of())
            wall = time.time() - t0
            evd = json.loads(ev.read_text(encoding="utf-8")) if r.returncode == 0 and ev.is_file() else {}
            vuid = (r.stderr or "").count("VUID-")
            ms = evd.get("real_render_frame_ms")
            row = {"step": tag, "rc": r.returncode, "vuid": vuid,
                   "digest": evd.get("digest"), "real_render_frame_ms": ms,
                   "within_90fps": (ms is not None and ms <= 11.11),
                   "wall_s": round(wall, 1)}
            if r.returncode != 0 or vuid != 0:
                row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-8:]
                fails += 1
            rows.append(row)
            print(json.dumps(row, ensure_ascii=False), flush=True)
    (F / "F1_COMBO.json").write_text(
        json.dumps({"schema": "rurix.day0829.realism.f1_combo.v1", "fails": fails,
                    "budget_ms": 11.11, "rows": rows}, ensure_ascii=False, indent=1) + "\n",
        encoding="utf-8")
    print("F1", json.dumps({"fails": fails}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
