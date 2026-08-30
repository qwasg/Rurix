#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase C 补充：clamp 调参臂噪点指标（c001/c005/c015，conv 协议 + 暗部亮度）。
合并进 c_metrics.json（tuned_arms 键）。"""
from __future__ import annotations

import glob
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np

from g10_exr_lib import decode_exr

ARMS = {
    "gi2_on_c001": ROOT / "artifacts/day_0828/c_gi_r2/arms/gi2_on_c001/bistro-interior/tier100/tsr_device",
    "gi2_on_c005": ROOT / "artifacts/day_0828/c_gi_r2/arms/gi2_on_c005/bistro-interior/tier100/tsr_device",
    "gi2_on_c015": ROOT / "artifacts/day_0828/c_gi_r2/arms/gi2_on_c015/bistro-interior/tier100/tsr_device",
}
ROIS = {
    "wall": (1400, 150, 480, 270),
    "floor": (1100, 800, 480, 270),
    "dark_arch": (360, 0, 360, 180),
    "dark_table": (560, 560, 560, 200),
}


def load_luma_roi(path: str, roi) -> np.ndarray:
    f = decode_exr(Path(path).read_bytes(), expected_end="rurix")
    px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
    x, y, w, h = roi
    c = px[y : y + h, x : x + w, :]
    return c[..., 0] * 0.2126 + c[..., 1] * 0.7152 + c[..., 2] * 0.0722


def main() -> int:
    dst = ROOT / "artifacts/day_0828/c_gi_r2/c_metrics.json"
    out = json.loads(dst.read_text(encoding="utf-8"))
    out["tuned_arms"] = {}
    for arm, base in ARMS.items():
        rec: dict = {"temporal_conv": {}, "converged_rois": {}}
        f = decode_exr((base / "converged.exr").read_bytes(), expected_end="rurix")
        px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
        luma = px[..., 0] * 0.2126 + px[..., 1] * 0.7152 + px[..., 2] * 0.0722
        rec["converged_global_mean"] = float(luma.mean())
        for rn, (x, y, w, h) in ROIS.items():
            r = luma[y : y + h, x : x + w]
            rec["converged_rois"][rn] = {
                "mean": float(r.mean()),
                "p5": float(np.percentile(r, 5)),
                "p50": float(np.percentile(r, 50)),
            }
        files = sorted(glob.glob(str(base / "frames" / "frame_01*.exr")))[::2][:16]
        for rn, roi in ROIS.items():
            stack = np.stack([load_luma_roi(f2, roi) for f2 in files], axis=0)
            tstd = stack.std(axis=0)
            tmean = stack.mean(axis=0)
            rel = tstd / np.maximum(tmean, 1e-4)
            rec["temporal_conv"][rn] = {
                "frames_used": len(files),
                "temporal_std_p95": float(np.percentile(tstd, 95)),
                "temporal_rel_mean": float(rel.mean()),
                "temporal_rel_p95": float(np.percentile(rel, 95)),
                "mean_luma": float(tmean.mean()),
            }
        out["tuned_arms"][arm] = rec
        print(f"[{arm}] done")
    dst.write_text(json.dumps(out, indent=1, ensure_ascii=False), encoding="utf-8")
    print(f"-> {dst}")
    for arm, rec in out["tuned_arms"].items():
        for rn in ROIS:
            t = rec["temporal_conv"][rn]
            print(f"{arm} {rn}: rel_p95={t['temporal_rel_p95']:.4f} mean={rec['converged_rois'][rn]['mean']:.6f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
