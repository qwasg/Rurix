#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0829 真实感战役渲染执行器:GPU 锁内跑一个渲染臂并登记(night_0828 同形,账本落本目录)。

用法:
  py -3 run_render.py --tag base -- <bin> [args...]
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", required=True)
    ap.add_argument("cmd", nargs=argparse.REMAINDER)
    args = ap.parse_args()
    cmd = args.cmd
    if cmd and cmd[0] == "--":
        cmd = cmd[1:]
    if not cmd:
        print("FAIL: 空命令")
        return 2
    t0 = time.monotonic()
    with gpu_device_lock(purpose=f"day0829 realism {args.tag}"):
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    dt = time.monotonic() - t0
    tail = (r.stdout or "").strip().splitlines()[-3:]
    rec = {
        "tag": args.tag,
        "cmd": cmd,
        "rc": r.returncode,
        "wall_s": round(dt, 2),
        "stdout_tail": tail,
        "stderr_tail": (r.stderr or "").strip().splitlines()[-3:],
    }
    logp = ROOT / "artifacts" / "day_0829_realism" / "render_runs.jsonl"
    with open(logp, "a", encoding="utf-8") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(json.dumps(rec, ensure_ascii=False, indent=1))
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
