#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""A2 自动曝光适应曲线分析：luma_on/luma_off 序列 → 验收统计 JSON。

判据（A2 验收协议第 6 步）：
- 无 >±10% 逐帧振荡（相邻帧 mean 相对变化 |Δ|/prev ≤ 0.10）
- 终态 mean ∈ [0.25, 0.55]（8bit 归一）
"""
from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def stats(path: Path) -> dict:
    doc = json.loads(path.read_text(encoding="utf-8"))
    seq = [(e["frame"], e["mean"]) for e in doc["seq"]]
    means = [m for _, m in seq]
    warmup = doc["warmup"]
    post = [m for f, m in seq if f >= warmup]
    deltas = []
    for a, b in zip(means, means[1:]):
        deltas.append(abs(b - a) / max(a, 1e-9))
    max_step = max(deltas) if deltas else 0.0
    return {
        "file": path.name,
        "auto_exposure": doc["auto_exposure"],
        "frames_total": len(seq),
        "first_mean": means[0],
        "final_mean": means[-1],
        "post_warmup_min": min(post),
        "post_warmup_max": max(post),
        "max_frame_step_rel": max_step,
        "oscillation_gt_10pct_frames": sum(1 for d in deltas if d > 0.10),
    }


def main() -> None:
    on = stats(HERE / "luma_on.json")
    off = stats(HERE / "luma_off.json")
    verdict = {
        "schema": "rurix.a2.autoexp.curve_analysis.v1",
        "on": on,
        "off": off,
        "checks": {
            "no_oscillation_gt_10pct": on["oscillation_gt_10pct_frames"] == 0,
            "final_mean_in_band_0p25_0p55": 0.25 <= on["final_mean"] <= 0.55,
            "band_full_curve": 0.25 <= on["post_warmup_min"] and on["post_warmup_max"] <= 0.55,
            "brighter_than_off": on["final_mean"] > off["final_mean"],
        },
    }
    verdict["verdict"] = "PASS" if all(
        verdict["checks"][k] for k in ("no_oscillation_gt_10pct", "final_mean_in_band_0p25_0p55")
    ) else "FAIL"
    out = HERE / "curve_analysis.json"
    out.write_text(json.dumps(verdict, indent=1, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(verdict, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
