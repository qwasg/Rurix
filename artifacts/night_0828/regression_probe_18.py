#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""夜间巡航回归探针：target-night 二进制（含全部本巡航改动）跑代表性 Stage A
格，digest 对在案锚——证明全部加性改动默认 off 零降级（多车道/档/后端覆盖）。"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

BIN = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"
OUT = ROOT / "artifacts" / "night_0828" / "regression_full"
# (scene, tier, backend, anchor_digest)
CELLS = [
    ("bistro-interior", "100", "dlss_sr", "sha256:55ea0c2ba68011727b4136ecb32c627e36d539bb38a2aadad617bb17cb578d4a"),
    ("bistro-interior", "100", "fsr_3_1_5", "sha256:4cf67f08944c8d1faa6f96573910783c5f06e614d11ae37f928fc95f34f2d504"),
    ("bistro-interior_t100", "sr", "device", "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"),
    ("bistro-interior", "50", "dlss_sr", "sha256:9636cef0cb8775bed7c16d077816cf5bb8267c0894686b6a7fadaf8ae93feb53"),
    ("bistro-interior", "50", "fsr_3_1_5", "sha256:fed6420fe368303f09f591421c2abf529305ef10d890bbea074b1a9145570272"),
    ("bistro-interior_t50", "sr", "device", "sha256:8baadc1410ab0ab3e960d8a0d9649026b51a9eb0c6cfb966211d948476b138ba"),
    ("bistro-interior", "67", "dlss_sr", "sha256:d09fa898711eb6b617db7a4cd1440bcac980abce14c139cb618606f4386c6f89"),
    ("bistro-interior", "67", "fsr_3_1_5", "sha256:296edfbc3075980dd1adcebc42c9f401d9121e51bdd42ef9e1664407866da421"),
    ("bistro-interior_t67", "sr", "device", "sha256:9138c48dc63f34c9f8552e0228316be879b58c08f5f960347b5ac73156fea4a6"),
    ("cornell-box", "100", "dlss_sr", "sha256:a26130a190b6c174f40aedf839f3b7ae40d2f97c0bf015e91eebdadddaea8d4d"),
    ("cornell-box", "100", "fsr_3_1_5", "sha256:dd5e0841b172c31b1a82c56732c23b576cd6edcb476e614c6b06c20cca04f616"),
    ("cornell-box_t100", "sr", "device", "sha256:4a9f9637e37759f3a4b3b822e9b4174486f9331f69741eacb8299e8e8b71d190"),
    ("cornell-box", "50", "dlss_sr", "sha256:3eb3794d40cdfb59954fcf93607fdff42cc03c6010f90b5db08a49908b745851"),
    ("cornell-box", "50", "fsr_3_1_5", "sha256:db280e555c7fc47d72719997aa8f144ab5579fdf423b773772f94db86cea2041"),
    ("cornell-box_t50", "sr", "device", "sha256:c1b9ca686accf0c62b65174ed24ee3e9110047eb4256b081345a9dd45b7f6a45"),
    ("cornell-box", "67", "dlss_sr", "sha256:96f83ba4574ac102cbb3007fcef0d3819a76beae909102e7c0f3357c978dcc43"),
    ("cornell-box", "67", "fsr_3_1_5", "sha256:d7bf3ca87024347472a9877866bc9d7d0923332474ba7201fdf18808616c5b23"),
    ("cornell-box_t67", "sr", "device", "sha256:de56f41ec78bfeeafb028ac33408587ca70e706287871bf0fa8f90a00c74e297"),
]


def main() -> int:
    results = []
    with gpu_device_lock(purpose="night0828 Stage A 回归探针"):
        for scene, tier_n, backend, anchor in CELLS:
            key = f"{scene}_t{tier_n}_{backend}"
            env = dict(os.environ)
            env["RURIX_REQUIRE_REAL"] = "1"
            env["RURIX_VK_VALIDATION"] = "1"
            argv = [str(BIN), "--bench", "--scene", scene, "--tier", tier_n,
                    "--backend", backend, "--frames", "160", "--warmup", "10",
                    "--out-root", str(OUT)]
            t0 = time.time()
            r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=1200, env=env)
            wall = time.time() - t0
            receipt = OUT / scene / f"tier{tier_n}" / backend / "bench_receipt.json"
            got = None
            if r.returncode == 0 and receipt.is_file():
                got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
            ok = got == anchor
            results.append({"cell": key, "fresh": got, "anchor": anchor, "match": ok,
                            "rc": r.returncode, "wall_s": round(wall, 1)})
            print(f"[{key}] rc={r.returncode} {'MATCH' if ok else 'DRIFT/ERR'} ({wall:.0f}s)", flush=True)
    matched = sum(1 for x in results if x["match"])
    summary = {"matched": matched, "total": len(results), "zero_drift": matched == len(results), "cells": results}
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "regression_summary.json").write_text(json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"ZERO-DRIFT {matched}/{len(results)}")
    return 0 if matched == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
