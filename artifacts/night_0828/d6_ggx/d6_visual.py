#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D6 GGX 视觉臂：128 帧收敛渲染（GPU 锁内）off/on 双臂 + EXR→PNG 转换。

判据：
  off 臂 converged_digest == snrm_on 在案锚 778f1dfc…（128 帧收敛态零漂移）；
  on 臂 digest ≠ off（GGX 接线进收敛帧）；
  双臂 EXR → ACES PNG 落盘供人工复核（金属/釉面高光、无腐败/无全屏噪）。
产物：artifacts/night_0828/d6_ggx/visual_summary.json + ggx_off/on_aces.png
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
ANCHOR_SNRM_128 = "sha256:778f1dfcd2e2c163af79879f8e0e804000674a2db291283b9f42569c03eaac76"

RENDER = [
    "--render", "--scene", "bistro-interior", "--tier", "100",
    "--backend", "tsr_device", "--frames", "128", "--warmup", "10",
    "--presentation-profile", "night", "--smooth-normals", "on",
]


def render(env: dict[str, str], tag: str, extra: list[str]) -> dict:
    out_root = f"artifacts/night_0828/d6_ggx/{tag}"
    t0 = time.monotonic()
    r = subprocess.run(
        [str(BIN)] + RENDER + extra + ["--out-root", out_root],
        cwd=ROOT, env=env, capture_output=True, text=True)
    rec = {
        "tag": tag,
        "rc": r.returncode,
        "wall_s": round(time.monotonic() - t0, 2),
        "stderr_tail": (r.stderr or "").strip().splitlines()[-4:],
    }
    rp = ROOT / out_root / "bistro-interior" / "tier100" / "tsr_device" / "render_receipt.json"
    if r.returncode == 0 and rp.exists():
        j = json.loads(rp.read_text(encoding="utf-8"))
        rec["converged_digest"] = j.get("converged_digest")
        rec["exr"] = str((ROOT / out_root / "bistro-interior" / "tier100" / "tsr_device" / "converged.exr").relative_to(ROOT))
    return rec


def to_png(exr: str, png: Path) -> bool:
    r = subprocess.run(
        ["py", "-3", "artifacts/night_0828/exr_view.py", exr, str(png),
         "--mode", "aces", "--exposure", "1.0"],
        cwd=ROOT, capture_output=True, text=True)
    return r.returncode == 0 and png.exists()


def main() -> int:
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = "H:\\rurix\\target-night"
    result: dict = {"anchor_snrm_128f": ANCHOR_SNRM_128, "runs": []}
    with gpu_device_lock(purpose="d6 ggx visual 128f renders"):
        result["runs"].append(render(env, "render_off", []))
        result["runs"].append(render(env, "render_on", ["--ggx", "on"]))
    runs = {r["tag"]: r for r in result["runs"]}
    off, on = runs["render_off"], runs["render_on"]
    pngs = {}
    for tag, r in runs.items():
        if r.get("exr"):
            png = OUT / f"{'ggx_off' if tag == 'render_off' else 'ggx_on'}_aces.png"
            pngs[tag] = str(png.relative_to(ROOT)) if to_png(r["exr"], png) else None
    result["verdict"] = {
        "off_matches_snrm128_anchor": off.get("converged_digest") == ANCHOR_SNRM_128,
        "on_differs_from_off": on.get("converged_digest") not in (None, off.get("converged_digest")),
        "on_converged_digest": on.get("converged_digest"),
        "off_converged_digest": off.get("converged_digest"),
        "pngs": pngs,
    }
    result["all_pass"] = (
        all(r["rc"] == 0 for r in result["runs"])
        and result["verdict"]["off_matches_snrm128_anchor"]
        and result["verdict"]["on_differs_from_off"]
        and all(pngs.values())
    )
    OUT.joinpath("visual_summary.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(result["verdict"], ensure_ascii=False, indent=1))
    print("ALL_PASS" if result["all_pass"] else "FAIL")
    return 0 if result["all_pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
