#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D2 平滑法线臂：改后自验矩阵（GPU 锁内：cargo build 双 bin + off×2 + on×2）。

机核判据：
  off 臂双跑 digest == 改前基线 digest（baseline_pre.json）——Stage A 锚零漂移；
  on  臂双跑 digest 互等（确定性）且 ≠ off 臂（接线真实生效）；
  记录 on/off scene_gpu_ns_mean 对照（性能增量 measured）。
产物：artifacts/night_0828/d2_smooth_nrm/verify_summary.json
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

OUT = ROOT / "artifacts" / "night_0828" / "d2_smooth_nrm"
BIN = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"

BENCH = [
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
    }


def bench(env: dict[str, str], tag: str, extra: list[str]) -> dict:
    out_root = f"artifacts/night_0828/d2_smooth_nrm/{tag}"
    t0 = time.monotonic()
    r = run([str(BIN)] + BENCH + extra + ["--out-root", out_root], env)
    rec = {
        "tag": tag,
        "rc": r.returncode,
        "wall_s": round(time.monotonic() - t0, 2),
        "stderr_tail": (r.stderr or "").strip().splitlines()[-6:],
    }
    if r.returncode == 0:
        rec.update(receipt_digest(out_root))
    return rec


def main() -> int:
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = "H:\\rurix\\target-night"
    # 改前基线 digest 直读收据（pre_off 收据 = 改前二进制双跑产物，两跑
    # digest 互等且与夜基线 128 帧 render 收据 frame_0009 交叉一致）。
    pre_rp = OUT / "pre_off" / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
    pre_rec = json.loads(pre_rp.read_text(encoding="utf-8"))
    pre_digest = pre_rec["last_frame_digest"]
    pre_scene_gpu_ns = pre_rec["stats_post_warmup"]["scene_gpu_ns_mean"]
    result: dict = {
        "pre_digest": pre_digest,
        "pre_scene_gpu_ns_mean": pre_scene_gpu_ns,
        "runs": [],
    }
    with gpu_device_lock(purpose="d2 smooth-nrm verify matrix"):
        t0 = time.monotonic()
        b = run(
            ["cargo", "build", "--release", "-p", "rurix-render", "--features",
             "vendor-upscale", "--bin", "g14_3_pipeline_perf", "--bin",
             "g31_window_present"],
            env,
        )
        result["build"] = {
            "rc": b.returncode,
            "wall_s": round(time.monotonic() - t0, 2),
            "stderr_tail": (b.stderr or "").strip().splitlines()[-10:],
        }
        if b.returncode != 0:
            OUT.joinpath("verify_summary.json").write_text(
                json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
            print("BUILD FAIL")
            return 1
        for tag, extra in [
            ("post_off_1", []),
            ("post_off_2", []),
            ("post_on_1", ["--smooth-normals", "on"]),
            ("post_on_2", ["--smooth-normals", "on"]),
        ]:
            result["runs"].append(bench(env, tag, extra))

    offs = [r for r in result["runs"] if r["tag"].startswith("post_off")]
    ons = [r for r in result["runs"] if r["tag"].startswith("post_on")]
    off_digests = {r.get("last_frame_digest") for r in offs}
    on_digests = {r.get("last_frame_digest") for r in ons}
    result["verdict"] = {
        "off_double_run_equal": len(off_digests) == 1,
        "off_matches_pre_baseline": off_digests == {pre_digest},
        "on_double_run_equal": len(on_digests) == 1,
        "on_differs_from_off": on_digests.isdisjoint(off_digests) and len(on_digests) == 1,
        "off_scene_gpu_ns_mean": [r.get("scene_gpu_ns_mean") for r in offs],
        "on_scene_gpu_ns_mean": [r.get("scene_gpu_ns_mean") for r in ons],
        "off_frame_ms_mean": [r.get("frame_ms_mean") for r in offs],
        "on_frame_ms_mean": [r.get("frame_ms_mean") for r in ons],
    }
    v = result["verdict"]
    result["all_pass"] = all(
        r["rc"] == 0 for r in result["runs"]
    ) and v["off_double_run_equal"] and v["off_matches_pre_baseline"] \
        and v["on_double_run_equal"] and v["on_differs_from_off"]
    OUT.joinpath("verify_summary.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=1))
    return 0 if result["all_pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
