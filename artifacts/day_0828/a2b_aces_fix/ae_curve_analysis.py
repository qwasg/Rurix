#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""A2b AE 适应曲线复测分析（修复改变 presented 亮度映射 → 登记新曲线数值）。

口径 = A2 交付的验证面（六臂组合 + dolly 240 帧 present-luma-out）；
不做硬带断言（定档归 A3）——终态 mean 掉出 [0.2,0.6] 大带才标注。
"""
from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def main() -> None:
    doc = json.loads((HERE / "luma_combo_dolly240.json").read_text(encoding="utf-8"))
    means = [e["mean"] for e in doc["seq"]]
    deltas = [abs(b - a) / max(a, 1e-9) for a, b in zip(means, means[1:])]
    out = {
        "schema": "rurix.a2b.ae_curve_retest.v1",
        "caliber": "六臂组合 + --auto-move dolly --frames 240 --present-luma-out（A2 验证面同口径,v2 修复 SPV）",
        "frames_total": len(means),
        "warmup": doc.get("warmup"),
        "first_mean": means[0],
        "final_mean": means[-1],
        "min_mean": min(means),
        "max_mean": max(means),
        "max_frame_step_rel": max(deltas) if deltas else 0.0,
        "oscillation_gt_10pct_frames": sum(1 for d in deltas if d > 0.10),
        "final_in_wide_band_0p2_0p6": 0.2 <= means[-1] <= 0.6,
        "a2_prefix_reference": {
            "on": {"first": 0.3968, "final": 0.4005, "min": 0.3755, "max": 0.4006, "max_step": 0.0042},
            "note": "A2 在 bug SPV 下的在案曲线（bug 系统性提亮暗部 ⇒ 修复后同 gain 下 presented mean 预期下移,如实登记非回归）",
        },
    }
    (HERE / "ae_curve_retest.json").write_text(json.dumps(out, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(out, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
