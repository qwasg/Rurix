#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase D TSR 降噪质量档：四臂颗粒指标（grain_metric 同式,c_noise_metrics 同 ROI/协议）。

四臂 = ① arm1_snrm（质量腿 tsrq off = C 相 gi2-off 形态）② arm2_snrm_tsrq（tsrq on）
       ③ arm3_gi2（gi2 c001 + tsrq off）④ arm4_gi2_tsrq（gi2 c001 + tsrq on）。
指标 = conv 协议（128f 末段 frame_01*.exr stride 2 × ≤16 帧,TSR 收敛后）逐像素跨帧
std 的 ROI 统计（绝对幅值口径,C 相教训）+ 收敛帧高通能量（锐度面,box3 高通）。
判据：② vs ① 墙/地板 std_p95 ↓（目标 ≥30%）；④ vs ③ 拱下/桌下 std_p95 ↓（目标
≥50%,决定 gi2 入 full 档）；收敛帧高频能量不升（锐度不损）。
输出 artifacts/day_0828/d_tsr/d_metrics.json。
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

BASE = ROOT / "artifacts/day_0828/d_tsr/arms"
ARMS = {
    "arm1_snrm_tsrqoff": BASE / "arm1_snrm/bistro-interior/tier100/tsr_device",
    "arm2_snrm_tsrqon": BASE / "arm2_snrm_tsrq/bistro-interior/tier100/tsr_device",
    "arm3_gi2c001_tsrqoff": BASE / "arm3_gi2/bistro-interior/tier100/tsr_device",
    "arm4_gi2c001_tsrqon": BASE / "arm4_gi2_tsrq/bistro-interior/tier100/tsr_device",
}
ROIS = {
    "wall": (1400, 150, 480, 270),
    "floor": (1100, 800, 480, 270),
    "dark_arch": (360, 0, 360, 180),
    "dark_table": (560, 560, 560, 200),
}
# conv 协议 = c_noise_metrics 同式（TSR 收敛后末段）;night 协议一并登记（对
# 夜巡基线可比面）。
PROTOCOLS = {
    "conv": ("frames/frame_01*.exr", 2, 16),
    "night": ("frames/frame_*.exr", 8, 16),
}


def load_luma(path: str) -> np.ndarray:
    """全帧 luma（每帧只解码一次——四 ROI 共享切片,I/O 成本 1/4）。"""
    f = decode_exr(Path(path).read_bytes(), expected_end="rurix")
    px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
    return px[..., 0] * 0.2126 + px[..., 1] * 0.7152 + px[..., 2] * 0.0722


def box3(a: np.ndarray) -> np.ndarray:
    p = np.pad(a, 1, mode="edge")
    s = np.zeros_like(a)
    for dy in range(3):
        for dx in range(3):
            s += p[dy : dy + a.shape[0], dx : dx + a.shape[1]]
    return s / 9.0


def temporal_all(files: list[str]) -> dict:
    """一次解码逐帧全 luma → 四 ROI 统计（帧栈仅存 ROI 切片,内存有界）。"""
    stacks: dict[str, list[np.ndarray]] = {rn: [] for rn in ROIS}
    for f in files:
        luma = load_luma(f)
        for rn, (x, y, w, h) in ROIS.items():
            stacks[rn].append(luma[y : y + h, x : x + w])
    out: dict = {}
    for rn, frames in stacks.items():
        stack = np.stack(frames, axis=0)
        tstd = stack.std(axis=0)
        tmean = stack.mean(axis=0)
        rel = tstd / np.maximum(tmean, 1e-4)
        out[rn] = {
            "frames_used": len(files),
            "temporal_std_mean": float(tstd.mean()),
            "temporal_std_p95": float(np.percentile(tstd, 95)),
            "temporal_rel_mean": float(rel.mean()),
            "temporal_rel_p95": float(np.percentile(rel, 95)),
            "mean_luma": float(tmean.mean()),
        }
    return out


def main() -> int:
    out: dict = {
        "schema": "rurix.day0828.d_tsr.grain_metrics.v1",
        "rois": {k: list(v) for k, v in ROIS.items()},
        "protocols": {k: list(v) for k, v in PROTOCOLS.items()},
        "arms": {},
    }
    for arm, base in ARMS.items():
        rec: dict = {"path": str(base.relative_to(ROOT)).replace("\\", "/")}
        # 收敛帧全局/ROI 亮度 + 高通能量（锐度面）。
        f = decode_exr((base / "converged.exr").read_bytes(), expected_end="rurix")
        px = np.array(f["pixels"], dtype=np.float64).reshape(f["height"], f["width"], 3)
        luma = px[..., 0] * 0.2126 + px[..., 1] * 0.7152 + px[..., 2] * 0.0722
        rec["converged_global_mean"] = float(luma.mean())
        rec["converged_rois"] = {}
        for rn, (x, y, w, h) in ROIS.items():
            r = luma[y : y + h, x : x + w]
            hp = np.abs(r - box3(r))
            rec["converged_rois"][rn] = {
                "mean": float(r.mean()),
                "p5": float(np.percentile(r, 5)),
                "p50": float(np.percentile(r, 50)),
                "p99": float(np.percentile(r, 99)),
                "highpass_mean": float(hp.mean()),
                "highpass_p95": float(np.percentile(hp, 95)),
                "highpass_rel": float((hp / np.maximum(r, 1e-4)).mean()),
            }
        # 时域指标（双协议 × 四 ROI;每帧单次解码）。
        for pn, (pat, stride, maxf) in PROTOCOLS.items():
            files = sorted(glob.glob(str(base / pat)))[::stride][:maxf]
            if len(files) < 2:
                print(f"FAIL: {arm}/{pn} 帧数不足", files)
                return 1
            rec[pn] = temporal_all(files)
        out["arms"][arm] = rec
        print(f"[{arm}] conv wall std_p95 = {rec['conv']['wall']['temporal_std_p95']:.6e}")

    # 判据面（绝对幅值 std_p95,conv 协议）。
    def p95(arm, roi):
        return out["arms"][arm]["conv"][roi]["temporal_std_p95"]

    def drop(a, b):  # b vs a 下降百分比
        return (1.0 - p95(b, roi) / max(p95(a, roi), 1e-30)) * 100.0

    verdicts: dict = {}
    for roi in ("wall", "floor"):
        d = drop("arm1_snrm_tsrqoff", "arm2_snrm_tsrqon")
        verdicts[f"tsrq_on_vs_off_{roi}_std_p95_drop_pct"] = round(d, 2)
    for roi in ("dark_arch", "dark_table"):
        d = drop("arm3_gi2c001_tsrqoff", "arm4_gi2c001_tsrqon")
        verdicts[f"gi2_tsrq_on_vs_off_{roi}_std_p95_drop_pct"] = round(d, 2)
    # 锐度面：② vs ① 收敛帧高通能量（全 ROI）。
    hp_delta = {}
    for roi in ROIS:
        h1 = out["arms"]["arm1_snrm_tsrqoff"]["converged_rois"][roi]["highpass_mean"]
        h2 = out["arms"]["arm2_snrm_tsrqon"]["converged_rois"][roi]["highpass_mean"]
        hp_delta[roi] = {
            "arm1": h1,
            "arm2": h2,
            "delta_pct": round((h2 / max(h1, 1e-30) - 1.0) * 100.0, 2),
        }
    verdicts["converged_highpass_arm2_vs_arm1"] = hp_delta
    out["verdicts"] = verdicts

    dst = ROOT / "artifacts/day_0828/d_tsr/d_metrics.json"
    dst.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps(verdicts, indent=1, ensure_ascii=False))
    print(f"-> {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
