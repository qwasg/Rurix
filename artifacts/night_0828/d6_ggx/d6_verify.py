#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D6 GGX 高光臂：改后自验矩阵（GPU 锁内：off×2 + on×2 + 默认臂×2）。

机核判据：
  snrm_off 臂双跑 digest == 改前基线（baseline_pre.json snrm_digest）——
    g18_smooth_nrm kernel D6 扩面 + tri_mr 哑表 + PARAMS_LEN 56 零漂移；
  default 臂双跑 digest == 改前基线（baseline_pre.json default_digest）——
    PARAMS_LEN 48→56 对 Stage A 默认臂零影响；
  ggx_on 臂双跑 digest 互等（确定性）且 ≠ snrm_off 臂（接线真实生效）；
  记录 off/on scene_gpu_ns_mean 对照（GGX 增量 measured）。
产物：artifacts/night_0828/d6_ggx/verify_summary.json
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

OUT = ROOT / "artifacts" / "night_0828" / "d6_ggx"
BIN = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"

BENCH = [
    "--bench", "--scene", "bistro-interior", "--tier", "100",
    "--backend", "tsr_device", "--frames", "8", "--warmup", "2",
]
SNRM = ["--presentation-profile", "night", "--smooth-normals", "on"]


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
    out_root = f"artifacts/night_0828/d6_ggx/{tag}"
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
    pre = json.loads(OUT.joinpath("baseline_pre.json").read_text(encoding="utf-8"))
    pre_default = pre["verdict"]["default_digest"][0]
    pre_snrm = pre["verdict"]["snrm_digest"][0]
    result: dict = {
        "pre_default_digest": pre_default,
        "pre_snrm_digest": pre_snrm,
        "runs": [],
    }
    with gpu_device_lock(purpose="d6 ggx verify matrix"):
        for tag, extra in [
            ("post_default_1", []),
            ("post_default_2", []),
            ("post_snrm_off_1", SNRM),
            ("post_snrm_off_2", SNRM),
            ("post_ggx_on_1", SNRM + ["--ggx", "on"]),
            ("post_ggx_on_2", SNRM + ["--ggx", "on"]),
        ]:
            result["runs"].append(bench(env, tag, extra))

    defs = [r for r in result["runs"] if r["tag"].startswith("post_default")]
    offs = [r for r in result["runs"] if r["tag"].startswith("post_snrm_off")]
    ons = [r for r in result["runs"] if r["tag"].startswith("post_ggx_on")]
    def_digests = {r.get("last_frame_digest") for r in defs}
    off_digests = {r.get("last_frame_digest") for r in offs}
    on_digests = {r.get("last_frame_digest") for r in ons}
    result["verdict"] = {
        "default_double_run_equal": len(def_digests) == 1,
        "default_matches_pre": def_digests == {pre_default},
        "off_double_run_equal": len(off_digests) == 1,
        "off_matches_pre_snrm": off_digests == {pre_snrm},
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
    ) and v["default_double_run_equal"] and v["default_matches_pre"] \
        and v["off_double_run_equal"] and v["off_matches_pre_snrm"] \
        and v["on_double_run_equal"] and v["on_differs_from_off"]
    OUT.joinpath("verify_summary.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(result["verdict"], ensure_ascii=False, indent=1))
    print("ALL_PASS" if result["all_pass"] else "FAIL")
    return 0 if result["all_pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
