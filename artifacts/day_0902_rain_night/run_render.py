#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0902 雨夜街景展示战役渲染执行器:GPU 锁内跑一个渲染臂并登记
(day_0831_site/run_render.py 同形,账本落本目录 render_runs.jsonl)。

用法:
  py -3 run_render.py --tag <tag> -- <bin> [args...]

登记字段:tag / cmd / rc / wall_s / stdout_tail / stderr_tail(各末 3 行)/ log(全量 stdout+stderr 落 logs/<tag>.log,
加性字段,供首红查判读器口径;logs/ 不入库)。
GPU 真跑全程 ci/gpu_device_lock.py 排他锁(fail-closed:超时不得锁即 RuntimeError)。
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

LEDGER = HERE / "render_runs.jsonl"


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
    with gpu_device_lock(purpose=f"day0902 rain_night {args.tag}"):
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
    dt = time.monotonic() - t0
    log_dir = HERE / "logs"
    log_dir.mkdir(exist_ok=True)
    safe_tag = "".join(ch if (ch.isalnum() or ch in "-_.") else "_" for ch in args.tag)
    log_path = log_dir / f"{safe_tag}.log"
    with open(log_path, "a", encoding="utf-8") as lf:
        lf.write(f"===== {time.strftime('%Y-%m-%dT%H:%M:%S')} tag={args.tag} rc={r.returncode} wall_s={dt:.2f}\n")
        lf.write("----- cmd -----\n" + " ".join(cmd) + "\n")
        lf.write("----- stdout -----\n" + (r.stdout or "") + "\n")
        lf.write("----- stderr -----\n" + (r.stderr or "") + "\n")
    rec = {
        "tag": args.tag,
        "cmd": cmd,
        "rc": r.returncode,
        "wall_s": round(dt, 2),
        "stdout_tail": (r.stdout or "").strip().splitlines()[-3:],
        "stderr_tail": (r.stderr or "").strip().splitlines()[-3:],
        "log": str(log_path.relative_to(HERE)).replace("\\", "/"),
    }
    with open(LEDGER, "a", encoding="utf-8") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(json.dumps(rec, ensure_ascii=False, indent=1))
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
