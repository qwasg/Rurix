#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""仅 tsr_device 6 格回归（隔离 vendor 臂干扰，验证 tsr 车道零漂移）。"""
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
OUT = ROOT / "artifacts" / "night_0828" / "regression_tsr"
_A = json.loads((ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json").read_text(encoding="utf-8"))["anchors"]
CELLS = [
    (s, t, "tsr_device", _A[f"{s}_t{t}_tsr_device"]["last_frame_digest"])
    for s in ("bistro-interior", "cornell-box") for t in ("50", "67", "100")
]


def main() -> int:
    results = []
    with gpu_device_lock(purpose="night0828 tsr 回归"):
        for scene, tier, backend, anchor in CELLS:
            env = dict(os.environ)
            env["RURIX_REQUIRE_REAL"] = "1"
            env["RURIX_VK_VALIDATION"] = "1"
            argv = [str(BIN), "--bench", "--scene", scene, "--tier", tier,
                    "--backend", backend, "--frames", "160", "--warmup", "10",
                    "--out-root", str(OUT)]
            r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=1200, env=env)
            receipt = OUT / scene / f"tier{tier}" / backend / "bench_receipt.json"
            got = None
            if r.returncode == 0 and receipt.is_file():
                got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
            ok = got == anchor
            results.append({"cell": f"{scene}_t{tier}", "match": ok, "rc": r.returncode, "fresh": got})
            print(f"{scene}_t{tier} rc={r.returncode} {'MATCH' if ok else 'ERR/DRIFT'}", flush=True)
    m = sum(1 for x in results if x["match"])
    print(f"TSR ZERO-DRIFT {m}/{len(results)}")
    (OUT / "summary.json").parent.mkdir(parents=True, exist_ok=True)
    (OUT / "summary.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
