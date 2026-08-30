#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D6 GGX 臂：改前基线 digest 采集（GPU 锁内，当前 target-night 二进制 =
改前源码状态——D1/D2/D3/D5 已落地、D6 未动）。

两臂 × 双跑：
  default：无旗标（Stage A 默认臂 g14_3_direct_gi 车道）——PARAMS_LEN 48→56
           扩面零漂移锚；
  snrm   ：--presentation-profile night --smooth-normals on（D2/D5 质量车道
           现状）——kernel/侧表/参数面改动零漂移锚。
产物：artifacts/night_0828/d6_ggx/baseline_pre.json
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
    result: dict = {"runs": []}
    with gpu_device_lock(purpose="d6 ggx pre-change baseline"):
        for tag, extra in [
            ("pre_default_1", []),
            ("pre_default_2", []),
            ("pre_snrm_1", ["--presentation-profile", "night", "--smooth-normals", "on"]),
            ("pre_snrm_2", ["--presentation-profile", "night", "--smooth-normals", "on"]),
        ]:
            result["runs"].append(bench(env, tag, extra))
    runs = {r["tag"]: r for r in result["runs"]}
    d = {runs["pre_default_1"].get("last_frame_digest"), runs["pre_default_2"].get("last_frame_digest")}
    s = {runs["pre_snrm_1"].get("last_frame_digest"), runs["pre_snrm_2"].get("last_frame_digest")}
    result["verdict"] = {
        "default_double_run_equal": len(d) == 1,
        "snrm_double_run_equal": len(s) == 1,
        "default_digest": list(d),
        "snrm_digest": list(s),
        "all_rc0": all(r["rc"] == 0 for r in result["runs"]),
    }
    OUT.joinpath("baseline_pre.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(result["verdict"], ensure_ascii=False, indent=1))
    ok = all(result["verdict"][k] for k in (
        "default_double_run_equal", "snrm_double_run_equal", "all_rc0"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
