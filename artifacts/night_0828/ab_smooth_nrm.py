#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D2 平滑法线 A/B 验收驱动：off（默认）vs on（--smooth-normals on）。

产出：两臂 converged EXR 的颗粒指标（墙面/地板 ROI 时间方差 + 收敛高频）、
scene GPU 帧时对照、视觉裁剪 PNG、Stage A 锚零漂移核验（off 臂 digest
须 == milestones/g14 锚或本巡航 base 臂 digest）。

用法: py -3 ab_smooth_nrm.py [--frames 128] [--skip-render]
"""
from __future__ import annotations

import argparse
import json
import statistics as st
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

BIN = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"
OUT = ROOT / "artifacts" / "night_0828"
NIGHT_SPV = ROOT / ".tmp" / "night_0828" / "spv" / "g18_smooth_nrm.spv"
BASE_DIGEST = "sha256:cde1b2551e5b2f7777e76adcb72a2d47e851bc737e49f9f526e82c5444f9ba7d"

ROIS = {"wall": (1400, 150, 480, 270), "floor": (1100, 800, 480, 270)}


def render(tag: str, extra: list[str], frames: int) -> Path:
    out_root = OUT / "arms" / tag
    cmd = [str(BIN), "--render", "--scene", "bistro-interior", "--tier", "100",
           "--backend", "tsr_device", "--frames", str(frames), "--warmup", "10",
           "--presentation-profile", "night", "--out-root", str(out_root)] + extra
    t0 = time.monotonic()
    with gpu_device_lock(purpose=f"night0828 D2 {tag}"):
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    dt = time.monotonic() - t0
    print(f"[{tag}] rc={r.returncode} wall={dt:.1f}s")
    if r.returncode != 0:
        print("STDERR:", (r.stderr or "")[-800:])
        raise SystemExit(f"render {tag} 失败")
    return out_root / "bistro-interior" / "tier100" / "tsr_device"


def receipt_stats(d: Path) -> dict:
    r = json.loads((d / "render_receipt.json").read_text(encoding="utf-8"))
    return {
        "converged_digest": r["converged_digest"],
        "scene_gpu_ms": st.mean(r["scene_gpu_ns"][10:]) / 1e6,
        "frame_ms_mean": st.mean(r["frame_ms"][10:]),
    }


def grain(d: Path, roi_name: str) -> dict:
    x, y, w, h = ROIS[roi_name]
    r = subprocess.run(
        ["py", "-3", str(OUT / "grain_metric.py"), str(d / "frames" / "frame_*.exr"),
         "--roi", str(x), str(y), str(w), str(h), "--converged", str(d / "converged.exr")],
        capture_output=True, text=True, cwd=ROOT)
    return json.loads(r.stdout)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--frames", type=int, default=128)
    ap.add_argument("--skip-render", action="store_true")
    args = ap.parse_args()

    if not args.skip_render:
        d_off = render("snrm_off", [], args.frames)
        d_on = render("snrm_on", ["--smooth-normals", "on"], args.frames)
    else:
        d_off = OUT / "arms" / "snrm_off" / "bistro-interior" / "tier100" / "tsr_device"
        d_on = OUT / "arms" / "snrm_on" / "bistro-interior" / "tier100" / "tsr_device"

    s_off, s_on = receipt_stats(d_off), receipt_stats(d_on)
    report: dict = {"off": s_off, "on": s_on, "grain": {}}
    report["stage_a_zero_drift"] = s_off["converged_digest"] == BASE_DIGEST
    report["wiring_live"] = s_off["converged_digest"] != s_on["converged_digest"]
    report["perf_delta_ms"] = round(s_on["scene_gpu_ms"] - s_off["scene_gpu_ms"], 4)
    report["perf_ratio"] = round(s_on["scene_gpu_ms"] / s_off["scene_gpu_ms"], 4)
    for roi in ROIS:
        report["grain"][roi] = {"off": grain(d_off, roi), "on": grain(d_on, roi)}

    txt = json.dumps(report, indent=2, ensure_ascii=False)
    (OUT / "d2_ab_report.json").write_text(txt + "\n", encoding="utf-8")
    print(txt)
    return 0


if __name__ == "__main__":
    sys.exit(main())
