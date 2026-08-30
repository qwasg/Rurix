#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G37 W6:终验门矩阵(CPU 守卫 + GPU 门串行;soak 另跑)。

范围 = 本役改动面(补扫 A 类门 + 翻转敏感门)+ 核心守卫:
  cpu: check_schemas / budget_eval / 关键 selftest 束
  gpu: g31 波 A 五门 + 波 B/C 改动过的门(12 门,gate key 从各脚本 GATE_KEY 自省)
判据:全绿 = rc 全 0;任一红即整体 FAIL(如实登记,不静默)。
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
W6 = ROOT / "artifacts" / "day_0830_delivery" / "w6_final"

CPU_STEPS: list[list[str]] = [
    ["py", "-3", "ci/check_schemas.py"],
    ["py", "-3", "ci/budget_eval.py"],
    ["py", "-3", "ci/gpu_device_lock.py", "--selftest"],
    ["py", "-3", "ci/g31_encode_parity_smoke.py", "--selftest"],
    ["py", "-3", "ci/g31_texture_sampling_smoke.py", "--selftest"],
    ["py", "-3", "ci/g31_blocked_probes_smoke.py", "--selftest"],
    ["py", "-3", "ci/g31_vendor_license_smoke.py", "--selftest"],
]

GPU_GATES = [
    "ci/g31_window_present_smoke.py",
    "ci/g31_frame_pipelining_smoke.py",
    "ci/g31_game_loop_smoke.py",
    "ci/g31_dynamic_scene_smoke.py",
    "ci/g31_framegen_present_smoke.py",
    "ci/g31_hzb_wiring_smoke.py",
    "ci/g31_slab_wiring_smoke.py",
    "ci/g31_svt_smoke.py",
    "ci/g31_cluster_lod_smoke.py",
    "ci/g31_wp_hlod_smoke.py",
    "ci/g31_profiling_smoke.py",
    "ci/g31_robustness_smoke.py",
]

RESULTS: list[dict] = []


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def gate_key_of(script: Path) -> str | None:
    m = re.search(r'GATE_KEY\s*=\s*"([^"]+)"', script.read_text(encoding="utf-8"))
    return m.group(1) if m else None


def run_step(tag: str, argv: list[str], timeout: int = 10800) -> bool:
    t0 = time.time()
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True,
                       timeout=timeout, env=env_of())
    row = {"step": tag, "argv": " ".join(argv), "rc": r.returncode,
           "wall_s": round(time.time() - t0, 1), "pass": r.returncode == 0,
           "t": time.strftime("%H:%M:%S")}
    if r.returncode != 0:
        row["tail"] = ((r.stdout or "") + "\n" + (r.stderr or "")).strip().splitlines()[-12:]
    RESULTS.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)
    return r.returncode == 0


def main() -> int:
    fails = 0
    for argv in CPU_STEPS:
        if not run_step("cpu:" + argv[2] if len(argv) > 2 else "cpu", argv, timeout=3600):
            fails += 1
    for g in GPU_GATES:
        script = ROOT / g
        key = gate_key_of(script)
        # robustness 门的 --gate 为布尔 flag(store_true),不吃 key 值。
        if "robustness" in g:
            argv = ["py", "-3", g, "--gate"]
        else:
            argv = ["py", "-3", g] + (["--gate", key] if key else [])
        if not run_step("gpu:" + Path(g).stem, argv):
            fails += 1
    doc = {"schema": "rurix.day0830.delivery.w6_gates.v1",
           "fails": fails, "verdict": "PASS" if fails == 0 else "FAIL",
           "rows": RESULTS}
    (W6 / "W6_GATES.json").write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n",
                                      encoding="utf-8")
    print("W6GATES", json.dumps({"fails": fails}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
