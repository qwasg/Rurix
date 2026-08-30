#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""c_metrics.json 绝对值速览（conv 协议）。"""
import json

d = json.load(open(r"h:\rurix\artifacts\day_0828\c_gi_r2\c_metrics.json", encoding="utf-8"))
for arm in ["old_base", "old_gi_sinhash", "gi2_off", "gi2_on"]:
    t = d[arm]["temporal"]["conv"]
    print(arm)
    for rn in ["wall", "floor", "dark_arch", "dark_table"]:
        r = t[rn]
        print(
            f"  {rn:10s} rel_p95={r['temporal_rel_p95']:.6f} rel_mean={r['temporal_rel_mean']:.6f}"
            f" std_p95={r['temporal_std_p95']:.3e} mean_luma={r['mean_luma']:.6f}"
        )
