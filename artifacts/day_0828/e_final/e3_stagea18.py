#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase E1 工作项 3：Stage A 全 18 格锚检（canonical 160 帧口径）。

测序纪律（夜巡教训）：vendor（dlss/fsr）格之后同批跑 tsr 格会 rc=1 ——
6 个 tsr 格先行隔离单跑（每格独立子进程,先于任何 vendor 格），再 12 个
vendor 格批跑。锚 = milestones/g14/g14_3_stage_a_digest_anchor.json 逐格。
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

BIN = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"
OUT = ROOT / "artifacts" / "day_0828" / "e_final" / "regression" / "stagea18"
ANCHORS = json.loads(
    (ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json").read_text(encoding="utf-8")
)["anchors"]

SCENES = ["cornell-box", "bistro-interior"]
TIERS = ["50", "67", "100"]
TSR = [(s, t, "tsr_device") for s in SCENES for t in TIERS]
VENDOR = [(s, t, b) for s in SCENES for t in TIERS for b in ("dlss_sr", "fsr_3_1_5")]


def run_cell(scene: str, tier: str, backend: str) -> dict:
    key = f"{scene}_t{tier}_{backend}"
    anchor = ANCHORS[key]["last_frame_digest"]
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    argv = [str(BIN), "--bench", "--scene", scene, "--tier", tier,
            "--backend", backend, "--frames", "160", "--warmup", "10",
            "--out-root", str(OUT)]
    t0 = time.time()
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=1200, env=env)
    wall = time.time() - t0
    receipt = OUT / scene / f"tier{tier}" / backend / "bench_receipt.json"
    got = None
    if r.returncode == 0 and receipt.is_file():
        got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
    ok = got == anchor
    vuid = (r.stderr or "").count("VUID-")
    print(f"[{key}] rc={r.returncode} {'MATCH' if ok else 'DRIFT/ERR'} vuid={vuid} ({wall:.0f}s)",
          flush=True)
    return {"cell": key, "fresh": got, "anchor": anchor, "match": ok,
            "rc": r.returncode, "vuid_hits": vuid, "wall_s": round(wall, 1)}


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    results = []
    with gpu_device_lock(purpose="day0828 Phase E1 Stage A 全 18 格锚检", timeout_s=7200.0):
        print("== 阶段 1/2：6 tsr 格隔离单跑（先于任何 vendor 格）==", flush=True)
        for cell in TSR:
            results.append(run_cell(*cell))
        print("== 阶段 2/2：12 vendor 格批跑 ==", flush=True)
        for cell in VENDOR:
            results.append(run_cell(*cell))
    matched = sum(1 for x in results if x["match"])
    summary = {"schema": "rurix.day0828.e1.stagea18_anchor_check.v1",
               "caliber": "canonical 160f/warmup10;6 tsr 隔离先行 + 12 vendor 批跑",
               "matched": matched, "total": len(results),
               "zero_drift": matched == len(results), "cells": results}
    out = ROOT / "artifacts" / "day_0828" / "e_final" / "e3_stagea18_summary.json"
    out.write_text(json.dumps(summary, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"STAGE-A-18 {matched}/{len(results)}")
    return 0 if matched == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
