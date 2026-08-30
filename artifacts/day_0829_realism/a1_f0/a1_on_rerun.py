#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""臂① on 双跑重验(红修 #2 后:realism AE 下标族修正——full+f0 组合 AE 真正
生效,digest 语义更新;off 锚与无 AE A/B 不受影响不重跑)。

判据:双跑位级一致 + VUID=0 + digest ≠ 87d7139f(错位时代 digest——AE 生效
后必变,不变即修正未生效判红)+ off 锚复验(all-off 8f,快证)。
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
A = ROOT / "artifacts" / "day_0829_realism" / "a1_f0"
EV = A / "ev"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_FULL = "sha256:de342586e452b903a2df7b744b9f67ad5b95b6bc5e3c17e0257def516ffc7211"
STALE_ON = "sha256:87d7139ffba304001fe1038e43b0a04e5315823d87f60c5104da1b66dea7db41"


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


LOG = open(A / "a1_log.jsonl", "a", encoding="utf-8")
RESULTS: list[dict] = []


def rec(row: dict) -> None:
    row["t"] = time.strftime("%H:%M:%S")
    LOG.write(json.dumps(row, ensure_ascii=False) + "\n")
    LOG.flush()
    RESULTS.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)


def run_win(tag: str, extra: list[str], frames: int) -> tuple[bool, str | None, dict]:
    ev = EV / f"{tag}.json"
    cmd = [str(WIN), "--frames", str(frames), "--warmup", "2", "--hidden",
           *extra, "--evidence", str(ev)]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       timeout=1800, env=env_of())
    wall = time.time() - t0
    got = None
    evd: dict = {}
    if r.returncode == 0 and ev.is_file():
        evd = json.loads(ev.read_text(encoding="utf-8"))
        got = evd.get("digest")
    vuid = (r.stderr or "").count("VUID-")
    ok = r.returncode == 0 and vuid == 0 and got is not None
    row = {"step": tag, "rc": r.returncode, "digest": got, "vuid": vuid,
           "wall_s": round(wall, 1),
           "real_render_frame_ms": evd.get("real_render_frame_ms")}
    if not ok:
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-8:]
    return ok, got, row


def main() -> int:
    EV.mkdir(parents=True, exist_ok=True)
    fails = 0
    with gpu_device_lock(purpose="day0829 realism a1 on-rerun (redfix2 AE idx)", timeout_s=7200.0):
        # off 锚快证(新 exe 含红修 #2 代码,off 面零漂移)。
        ok, got, row = run_win("a1_rf2_off_alloff_8f", [], 8)
        row["expect"] = ANCHOR_ALLOFF
        row["pass"] = ok and got == ANCHOR_ALLOFF
        fails += 0 if row["pass"] else 1
        rec(row)
        if fails == 0:
            ok, got, row = run_win("a1_rf2_off_full_96f", ["--quality", "full"], 96)
            row["expect"] = ANCHOR_FULL
            row["pass"] = ok and got == ANCHOR_FULL
            fails += 0 if row["pass"] else 1
            rec(row)
        d1 = None
        if fails == 0:
            ok, d1, row = run_win("a1_rf2_on_run1", ["--quality", "full", "--metal-f0", "on"], 96)
            row["pass"] = ok and d1 != STALE_ON
            row["stale_binding_digest_gone"] = d1 != STALE_ON
            fails += 0 if row["pass"] else 1
            rec(row)
        if fails == 0:
            ok, d2, row = run_win("a1_rf2_on_run2", ["--quality", "full", "--metal-f0", "on"], 96)
            row["pass"] = ok and d2 == d1
            row["double_run_bitexact"] = d2 == d1
            fails += 0 if row["pass"] else 1
            rec(row)
    summ = json.loads((A / "A1_RUNS.json").read_text(encoding="utf-8"))
    summ["rows"] = summ.get("rows", []) + RESULTS
    summ["on_rerun_redfix2_fails"] = fails
    summ["red_fix_2"] = "realism AE 下标族修正(tri_base 尾挂后 AE 三件顺延 +1;首版 _EM 下标错位 = tri_base 被 reduce 覆写/AE 链失效,full+f0 digest 与无 AE 组合位级相同即症状);既有 em+AE set_autoexp 无 _EM 分支为 Phase F 遗留缺口,de342586 锚内冻结不动,登记 HANDOVER"
    summ["fails"] = fails
    (A / "A1_RUNS.json").write_text(json.dumps(summ, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("A1-RF2", json.dumps({"fails": fails}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
