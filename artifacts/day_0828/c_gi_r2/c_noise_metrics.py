#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase C GI2 噪点/暗部指标驱动：三臂对比（旧 GI sin hash / gi2 R2 / gi2 off）。

指标 = grain_metric 同式时域指标（逐像素跨帧 std 的 ROI 统计）双协议：
  night 协议 = 全 128 帧 stride 8 × ≤16 帧（夜巡 D2/D4 同口径，含未收敛段）；
  conv  协议 = 末段帧 frame_01*.exr（100..127）stride 2 × ≤14 帧（TSR 收敛后口径）。
暗部亮度 = converged.exr ROI luma mean/p5/p50/p99。
输出 artifacts/day_0828/c_gi_r2/c_metrics.json。
"""
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
    "old_base": ROOT / "artifacts/night_0828/arms/base_render/bistro-interior/tier100/tsr_device",
    "old_gi_sinhash": ROOT / "artifacts/night_0828/arms/gi_on/bistro-interior/tier100/tsr_device",
    "gi2_off": ROOT / "artifacts/day_0828/c_gi_r2/arms/gi2_off_render/bistro-interior/tier100/tsr_device",
    "gi2_on": ROOT / "artifacts/day_0828/c_gi_r2/arms/gi2_on_render/bistro-interior/tier100/tsr_device",
}
ROIS = {
    "wall": (1400, 150, 480, 270),
    "floor": (1100, 800, 480, 270),
    "dark_arch": (360, 0, 360, 180),
    "dark_table": (560, 560, 560, 200),
}
PROTOCOLS = {
    "night": ("frame_*.exr", 8, 16),
    "conv": ("frame_01*.exr", 2, 16),
}


def load_luma_roi(path: str, roi: tuple[int, int, int, int]) -> np.ndarray:
    f = decode_exr(Path(path).read_bytes(), expected_end="rurix")
    px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
    x, y, w, h = roi
    c = px[y : y + h, x : x + w, :]
    return c[..., 0] * 0.2126 + c[..., 1] * 0.7152 + c[..., 2] * 0.0722


def temporal(files: list[str], roi) -> dict:
    stack = np.stack([load_luma_roi(f, roi) for f in files], axis=0)
    tstd = stack.std(axis=0)
    tmean = stack.mean(axis=0)
    rel = tstd / np.maximum(tmean, 1e-4)
    return {
        "frames_used": len(files),
        "temporal_std_mean": float(tstd.mean()),
        "temporal_std_p95": float(np.percentile(tstd, 95)),
        "temporal_rel_mean": float(rel.mean()),
        "temporal_rel_p95": float(np.percentile(rel, 95)),
        "mean_luma": float(tmean.mean()),
    }


def main() -> int:
    out: dict = {"schema": "rurix.day0828.c_gi_r2.noise_metrics.v1", "rois": {k: list(v) for k, v in ROIS.items()}}
    for arm, base in ARMS.items():
        arm_rec: dict = {}
        conv_path = base / "converged.exr"
        f = decode_exr(conv_path.read_bytes(), expected_end="rurix")
        px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
        luma = px[..., 0] * 0.2126 + px[..., 1] * 0.7152 + px[..., 2] * 0.0722
        arm_rec["converged_global_mean"] = float(luma.mean())
        arm_rec["converged_rois"] = {}
        for rn, (x, y, w, h) in ROIS.items():
            r = luma[y : y + h, x : x + w]
            arm_rec["converged_rois"][rn] = {
                "mean": float(r.mean()),
                "p5": float(np.percentile(r, 5)),
                "p50": float(np.percentile(r, 50)),
                "p99": float(np.percentile(r, 99)),
            }
        arm_rec["temporal"] = {}
        for pn, (pat, stride, maxf) in PROTOCOLS.items():
            files = sorted(glob.glob(str(base / "frames" / pat)))[::stride][:maxf]
            if len(files) < 2:
                arm_rec["temporal"][pn] = {"error": f"帧数不足 {len(files)}"}
                continue
            arm_rec["temporal"][pn] = {rn: temporal(files, roi) for rn, roi in ROIS.items()}
        out[arm] = arm_rec
        print(f"[{arm}] global_mean={arm_rec['converged_global_mean']:.6f}")
    # 汇总比率（conv 协议 temporal_rel_p95 为主指标）
    summary = {}
    for rn in ROIS:
        try:
            g_on = out["gi2_on"]["temporal"]["conv"][rn]["temporal_rel_p95"]
            g_off = out["gi2_off"]["temporal"]["conv"][rn]["temporal_rel_p95"]
            o_on = out["old_gi_sinhash"]["temporal"]["conv"][rn]["temporal_rel_p95"]
            o_off = out["old_base"]["temporal"]["conv"][rn]["temporal_rel_p95"]
            summary[rn] = {
                "gi2_on_over_off": g_on / max(g_off, 1e-12),
                "oldgi_on_over_off": o_on / max(o_off, 1e-12),
                "gi2_on_vs_oldgi_abs": g_on / max(o_on, 1e-12),
            }
        except (KeyError, TypeError):
            pass
    out["summary_conv_rel_p95_ratios"] = summary
    dst = ROOT / "artifacts/day_0828/c_gi_r2/c_metrics.json"
    dst.write_text(json.dumps(out, indent=1, ensure_ascii=False), encoding="utf-8")
    print(f"-> {dst}")
    print(json.dumps(summary, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
