#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase E1 工作项 4：--quality full 窗口风暴（--window-storm 3）。

判据：干净退出 rc=0 + resize_eras ≥ 1 + validation 静默（VUID 零命中）。
AE 状态 resize 复位 ~12 帧半衰属预期行为，登记不判红（A2 在案）。
口径照夜巡 robust_storm：30f/warmup4/dolly 轨迹 + 爆发 resize 3 次。
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
E = ROOT / "artifacts" / "day_0828" / "e_final"


def main() -> int:
    ev = E / "ev" / "e_storm.json"
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    with gpu_device_lock(purpose="day0828 Phase E1 --quality full 窗口风暴", timeout_s=7200.0):
        t0 = time.time()
        r = subprocess.run(
            [str(WIN), "--frames", "30", "--warmup", "4", "--hidden",
             "--auto-move", "dolly", "--quality", "full", "--window-storm", "3",
             "--evidence", str(ev)],
            cwd=ROOT, capture_output=True, text=True, timeout=1800, env=env)
        wall = time.time() - t0
    eras = None
    exit_reason = None
    if ev.is_file():
        d = json.loads(ev.read_text(encoding="utf-8"))
        eras = d.get("resize_eras")
        exit_reason = d.get("exit_reason")
    vuid = (r.stderr or "").count("VUID-")
    ok = r.returncode == 0 and (eras or 0) >= 1 and vuid == 0
    summary = {"schema": "rurix.day0828.e1.quality_full_storm.v1",
               "arm": "--quality full --window-storm 3（dolly 30f）",
               "rc": r.returncode, "wall_s": round(wall, 1),
               "resize_eras": eras, "exit_reason": exit_reason,
               "vuid_hits": vuid, "pass": ok,
               "note": "AE 状态 resize 复位 ~12 帧半衰属预期（A2 在案）,登记不判红",
               "stderr_tail": (r.stderr or "").strip().splitlines()[-3:]}
    (E / "e4_storm_summary.json").write_text(
        json.dumps(summary, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps(summary, ensure_ascii=False, indent=1))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
