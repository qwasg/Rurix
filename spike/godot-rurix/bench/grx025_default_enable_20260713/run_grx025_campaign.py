#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-025 default-enable per-pass bisection campaign driver (2026-07-13).

Runs the five rd_native replaceable passes ONE AT A TIME (single-pass backend=2
legs) against the SAME rb4 exe + v2.3 workload as the terminal rd_native
campaign (`bench/rd_native_final_20260713/`), so each single-pass leg is
directly comparable to that campaign's archived baseline_run{1,2,3}.json
(exe sha256 fc41853b..., dll sha256 47910fe7..., patch 0001-0029+0040-0048).

The GRX-025 default-enable gate asks: on the scenes where a pass ENGAGES, does
its single-pass avg_fps ratio (vs baseline median) sit at or above 0.95x? This
driver produces the per-pass legs; `analyze_grx025.py` computes the verdicts.

measured_local, single machine (RTX 4070 Ti, template_debug, D3D12 Forward+,
1080p), strictly serial, machine quiet. NO performance claim is made and no
number here is a benefit claim; the aggregate is a default-enable DECISION
INPUT (GRX-025), not a speedup.

Usage:
  py -3 run_grx025_campaign.py                 # all 5 passes, 1 full leg each
  py -3 run_grx025_campaign.py --passes tonemap,cluster_store
  py -3 run_grx025_campaign.py --passes tonemap --tag r2   # noise re-run
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH_DIR = HERE.parents[0]                      # spike/godot-rurix/bench
ROOT = HERE.parents[3]                           # repo root H:/rurix
RUNNER = BENCH_DIR / "run_benchmark_scenes.py"
RUNNER_SUMMARY = ROOT / "target" / "grx" / "godot_bench_runner_summary.json"
GODOT_EXE = (
    ROOT / "target" / "grx" / "godot-scratch-rb4" / "bin"
    / "godot.windows.template_debug.x86_64.console.exe"
)
PATCH_STACK_ID = "0001-0029+0040-0048"
DXC_DIR = r"H:\dxc-round7\extracted\bin\x64"
PROGRESS_LOG = HERE / "campaign_progress.log"
MATRIX_DIR = HERE / "matrices"

PASSES = ["tonemap", "ssao_blur", "taa_resolve", "particles_copy", "cluster_store"]


def now() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def log(msg: str) -> None:
    line = f"[{now()}] {msg}"
    print(line, flush=True)
    with PROGRESS_LOG.open("a", encoding="utf-8", newline="\n") as fh:
        fh.write(line + "\n")


def run_leg(pass_name: str, tag: str | None) -> int:
    matrix = MATRIX_DIR / f"rd_native_{pass_name}.json"
    if not matrix.is_file():
        log(f"leg {pass_name} ABORT: missing matrix {matrix}")
        return 1
    suffix = f"_{tag}" if tag else ""
    archive = HERE / f"rurix_{pass_name}{suffix}.json"
    log(f"leg {pass_name}{suffix} START -> {archive.name}: single-pass backend=2")

    env = dict(os.environ)
    env["RURIX_DXC_DIR"] = DXC_DIR
    cmd = [
        sys.executable, str(RUNNER),
        "--profile", "full",
        "--leg", "rurix",
        "--pass-matrix", str(matrix),
        "--godot-exe", str(GODOT_EXE),
        "--patch-stack-id", PATCH_STACK_ID,
    ]
    started = _dt.datetime.now(_dt.timezone.utc)
    proc = subprocess.run(cmd, cwd=str(ROOT), env=env)
    elapsed = int((_dt.datetime.now(_dt.timezone.utc) - started).total_seconds())

    if not RUNNER_SUMMARY.is_file():
        log(f"leg {pass_name}{suffix} FAIL: runner produced no summary (exit={proc.returncode})")
        return 1
    summary = json.loads(RUNNER_SUMMARY.read_text(encoding="utf-8"))
    # Archive the summary byte-for-byte as LF (mirror the source's own LF write).
    shutil.copyfile(RUNNER_SUMMARY, archive)
    status = summary.get("status")
    fc = summary.get("failure_count")
    wc = summary.get("warning_count")
    rid = summary.get("run_id")
    log(
        f"leg {pass_name}{suffix} {str(status).upper()} in {elapsed}s run_id={rid} "
        f"failure_count={fc} warning_count={wc} -> {archive.name}"
    )
    return 0 if (proc.returncode == 0 and status == "success" and fc == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--passes", type=str, default=",".join(PASSES))
    ap.add_argument("--tag", type=str, default=None,
                    help="archive suffix for a noise re-run, e.g. r2")
    args = ap.parse_args()

    passes = [p.strip() for p in args.passes.split(",") if p.strip()]
    bad = [p for p in passes if p not in PASSES]
    if bad:
        print(f"unknown passes: {bad}; valid={PASSES}", file=sys.stderr)
        return 2

    for req in (RUNNER, GODOT_EXE):
        if not Path(req).exists():
            print(f"required asset missing: {req}", file=sys.stderr)
            return 2

    log(f"campaign start: exe={GODOT_EXE} patch_stack_id={PATCH_STACK_ID} "
        f"passes={','.join(passes)} tag={args.tag or '-'}")
    rc = 0
    for i, p in enumerate(passes, 1):
        log(f"--- {i}/{len(passes)} : {p} ---")
        if run_leg(p, args.tag) != 0:
            rc = 1
            log(f"leg {p} non-clean; continuing serial campaign")
    log(f"campaign COMPLETE: passes={','.join(passes)} rc={rc}")
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
