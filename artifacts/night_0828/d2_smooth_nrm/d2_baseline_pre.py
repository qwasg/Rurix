#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D2 平滑法线臂：改前基线（GPU 锁内：cargo build +  stash 改前二进制 + off 臂 bench 取 digest）。

改前基线 = 当前工作树（含 G36 会话未提交面，本任务未动任何源文件）构建产物。
产物：artifacts/night_0828/d2_smooth_nrm/baseline_pre.json
     artifacts/night_0828/d2_smooth_nrm/g14_3_pipeline_perf_pre_d2.exe（改前二进制封存）
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

OUT = ROOT / "artifacts" / "night_0828" / "d2_smooth_nrm"
BIN = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"
STASH = OUT / "g14_3_pipeline_perf_pre_d2.exe"

BENCH_ARGS = [
    "--bench", "--scene", "bistro-interior", "--tier", "100",
    "--backend", "tsr_device", "--frames", "8", "--warmup", "2",
    "--presentation-profile", "night",
]


def run(cmd: list[str], env: dict[str, str]) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True)


def receipt_digest(out_root: str) -> dict:
    rp = ROOT / out_root / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
    rec = json.loads(rp.read_text(encoding="utf-8"))
    stats = rec.get("stats_post_warmup", {})
    return {
        "receipt": str(rp.relative_to(ROOT)),
        "last_frame_digest": rec.get("last_frame_digest"),
        "scene_gpu_ns_mean": stats.get("scene_gpu_ns_mean"),
        "frame_ms_mean": stats.get("frame_ms_mean"),
        "seed": rec.get("seed"),
        "jitter_base": rec.get("jitter_base"),
    }


def main() -> int:
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = "H:\\rurix\\target-night"
    result: dict = {"steps": []}
    with gpu_device_lock(purpose="d2 smooth-nrm baseline build+off-arm"):
        t0 = time.monotonic()
        b = run(
            ["cargo", "build", "--release", "-p", "rurix-render", "--features",
             "vendor-upscale", "--bin", "g14_3_pipeline_perf"],
            env,
        )
        result["steps"].append({
            "step": "cargo_build_pre",
            "rc": b.returncode,
            "wall_s": round(time.monotonic() - t0, 2),
            "stderr_tail": (b.stderr or "").strip().splitlines()[-8:],
        })
        if b.returncode != 0:
            OUT.joinpath("baseline_pre.json").write_text(
                json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
            print("BUILD FAIL")
            return 1

        shutil.copy2(BIN, STASH)
        result["steps"].append({"step": "stash_pre_exe", "rc": 0,
                                "stash": str(STASH.relative_to(ROOT)),
                                "bytes": STASH.stat().st_size})

        t0 = time.monotonic()
        out_root = "artifacts/night_0828/d2_smooth_nrm/pre_off"
        r = run([str(STASH)] + BENCH_ARGS + ["--out-root", out_root], env)
        rec = {
            "step": "bench_pre_off",
            "rc": r.returncode,
            "wall_s": round(time.monotonic() - t0, 2),
            "stdout_tail": (r.stdout or "").strip().splitlines()[-4:],
            "stderr_tail": (r.stderr or "").strip().splitlines()[-6:],
        }
        if r.returncode == 0:
            rec.update(receipt_digest(out_root))
        result["steps"].append(rec)
    OUT.joinpath("baseline_pre.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=1))
    return 0 if all(s["rc"] == 0 for s in result["steps"]) else 1


if __name__ == "__main__":
    sys.exit(main())
