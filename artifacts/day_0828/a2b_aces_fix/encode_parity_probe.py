#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""A2b device-vs-host encode parity 探针（防复发核心交付；CI 门候选）。

口径：同帧 GPU 切层对拍——
  输入 A = RURIX_G31_DUMP_F32 落的 TSR 输出 f32（encode kernel 唯一像素输入）；
  输入 B = --dump-present-raw 落的同帧 presented BGRA8 字节（w,h u32 LE 头）；
  host 预测 = 输入 A 逐像素经 display::aces13 f64 金标准（to_display_linear 逐字，
  矩阵/样条全 f64 现推）→ BT.1886 γ2.4 逆 EOTF v^(1/2.4) → floor(v·255+0.5) 量化
  （dither off / autoexp off 口径，aeg=1.0）；
  比对 = 预测 8bit vs presented 8bit 逐像素逐通道。
报告：exact-match %、LSB 差分位（p50/p99/p100）、>1/>2 LSB 像素数（期望 ~0）、
  worst 像素座标与值、fan(1500,12) 地标、0.18 灰设计点（host f64 与 kernel f32
  仿真双口径 → 8bit 99 = ACES 显示线性 0.104 设计点）。
依赖：artifacts/day_0828/recon/bluefan_encode_sim.py（本日 recon 已交叉验证的
  host 矩阵推导/样条参数/kernel f32 仿真单源）。

用法：py -3 artifacts/day_0828/a2b_aces_fix/encode_parity_probe.py
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
RECON = HERE.parent / "recon"
sys.path.insert(0, str(RECON))
from bluefan_encode_sim import (  # noqa: E402
    C5,
    C9,
    HALF_MIN,
    SPLINE_M,
    HostAces,
    kernel_sim,
    pack_params,
    vmul,
)

D = np.float64


def _spline_fwd_vec(x: np.ndarray, c: dict, n_seg: float) -> np.ndarray:
    """segmented_spline_c5/c9_fwd 逐字向量化（f64；basis = vmul(cf, SPLINE_M) 正形）。"""
    logx = np.log10(np.maximum(x, HALF_MIN))
    lminx = math.log10(c["min_point"][0])
    lmidx = math.log10(c["mid_point"][0])
    lmaxx = math.log10(c["max_point"][0])
    lminy = math.log10(c["min_point"][1])
    lmaxy = math.log10(c["max_point"][1])
    m_lo = logx <= lminx
    m_s1 = (~m_lo) & (logx < lmidx)
    m_s2 = (~m_lo) & (~m_s1) & (logx < lmaxx)

    lo = np.asarray(c["coefs_low"], dtype=D)
    hi = np.asarray(c["coefs_high"], dtype=D)
    kc1 = n_seg * (logx - lminx) / (lmidx - lminx)
    j1 = np.clip(np.floor(kc1), 0, n_seg - 1).astype(np.int64)
    t1 = kc1 - j1
    kc2 = n_seg * (logx - lmidx) / (lmaxx - lmidx)
    j2 = np.clip(np.floor(kc2), 0, n_seg - 1).astype(np.int64)
    t2 = kc2 - j2
    cf0 = np.where(m_s1, lo[j1], hi[j2])
    cf1 = np.where(m_s1, lo[np.minimum(j1 + 1, len(lo) - 1)], hi[np.minimum(j2 + 1, len(hi) - 1)])
    cf2 = np.where(m_s1, lo[np.minimum(j1 + 2, len(lo) - 1)], hi[np.minimum(j2 + 2, len(hi) - 1)])
    t = np.where(m_s1, t1, t2)
    # vmul(cf, SPLINE_M) 行向量正形（host 单源；M[2][1]=M[2][2]=0）。
    b0 = 0.5 * cf0 - cf1 + 0.5 * cf2
    b1 = -cf0 + cf1
    b2 = 0.5 * cf0 + 0.5 * cf1
    sp = t * t * b0 + t * b1 + b2
    lin_lo = logx * c["slope_low"] + (lminy - c["slope_low"] * lminx)
    lin_hi = logx * c["slope_high"] + (lmaxy - c["slope_high"] * lmaxx)
    logy = np.where(m_lo, lin_lo, np.where(m_s1 | m_s2, sp, lin_hi))
    return np.power(10.0, logy)


def host_encode_vec(rgb: np.ndarray, hm: HostAces) -> np.ndarray:
    """aces13.rs to_display_linear f64 逐字向量化 + BT.1886 γ + 量化。

    rgb: (N,3) f64 scene-linear Rec.709（TSR 输出）。返回 (N,3) uint8 RGB。
    """
    m = lambda mat: np.asarray(mat, dtype=D)
    r709_ap0 = m(hm.rec709_to_ap0)
    ap0_ap1 = m(hm.ap0_to_ap1)
    ap1_ap0 = m(hm.ap1_to_ap0)
    rrt_sat = m(hm.rrt_sat)
    odt_sat = m(hm.odt_sat)
    ap1_xyz = m(hm.ap1_to_xyz)
    xyz_ap1 = m(hm.xyz_to_ap1)
    d60_d65 = m(hm.d60_to_d65)
    xyz_709 = m(hm.xyz_to_rec709)
    vm = lambda v, mat: v @ mat  # vmul 行向量约定 out[i]=Σ_j v[j]·m[j][i]

    aces = vm(rgb, r709_ap0)
    r, g, b = aces[:, 0], aces[:, 1], aces[:, 2]
    # --- glow（rgb_2_saturation / rgb_2_yc / sigmoid_shaper / glow_fwd 逐字）---
    tiny = 1e-10
    maxc = np.max(aces, axis=1)
    minc = np.min(aces, axis=1)
    sat = (np.maximum(maxc, tiny) - np.maximum(minc, tiny)) / np.maximum(maxc, 1e-2)
    chroma = np.sqrt(np.maximum(b * (b - g) + g * (g - r) + r * (r - b), 0.0))
    yc = (b + g + r + 1.75 * chroma) / 3.0
    x = (sat - 0.4) / 0.2
    t = np.maximum(1.0 - np.abs(x / 2.0), 0.0)
    s = (1.0 + np.sign(x) * (1.0 - t * t)) / 2.0
    gg = 0.05 * s
    glow = np.where(
        yc <= 2.0 / 3.0 * 0.08,
        gg,
        np.where(yc >= 2.0 * 0.08, 0.0, gg * (0.08 / np.maximum(yc, 1e-300) - 0.5)),
    )
    aces = aces * (1.0 + glow)[:, None]
    r, g, b = aces[:, 0], aces[:, 1], aces[:, 2]
    # --- red modifier（rgb_2_hue 精确 atan2 + center_hue + cubic_basis_shaper）---
    neutral = (r == g) & (g == b)
    hue = np.degrees(np.arctan2(math.sqrt(3.0) * (g - b), 2.0 * r - g - b))
    hue = np.where(hue < 0.0, hue + 360.0, hue)
    centered = np.where(hue > 180.0, hue - 360.0, hue)  # center_hue(hue, 0)
    w = 135.0
    in_win = (centered > -w / 2.0) & (centered < w / 2.0)
    kc = (centered + w / 2.0) * 4.0 / w
    jf = np.floor(kc)
    tf = kc - jf
    y0 = tf**3 * (1.0 / 6.0)
    y1 = tf**3 * (-3.0 / 6.0) + tf**2 * (3.0 / 6.0) + tf * (3.0 / 6.0) + 1.0 / 6.0
    y2 = tf**3 * (3.0 / 6.0) + tf**2 * (-1.0) + 4.0 / 6.0
    y3 = tf**3 * (-1.0 / 6.0) + tf**2 * (3.0 / 6.0) + tf * (-3.0 / 6.0) + 1.0 / 6.0
    ybs = np.where(jf < 0.5, y0, np.where(jf < 1.5, y1, np.where(jf < 2.5, y2, y3)))
    hw = np.where(in_win & ~neutral, ybs * 1.5, 0.0)
    aces[:, 0] = r + hw * sat * (0.03 - r) * (1.0 - 0.82)
    # --- ACES → 渲染空间 → desat → c5 → OCES ---
    aces = np.maximum(aces, 0.0)
    pre = np.clip(vm(aces, ap0_ap1), 0.0, 65504.0)
    pre = vm(pre, rrt_sat)
    post = np.stack([_spline_fwd_vec(pre[:, i], C5, 3.0) for i in range(3)], axis=1)
    oces = vm(post, ap1_ap0)
    # --- ODT ---
    pre2 = vm(oces, ap0_ap1)
    post2 = np.stack([_spline_fwd_vec(pre2[:, i], C9, 7.0) for i in range(3)], axis=1)
    cb = 10.0 ** math.log10(0.02)
    lin = (post2 - cb) / (48.0 - cb)
    xyz = vm(lin, ap1_xyz)
    div = xyz.sum(axis=1)
    div = np.where(div == 0.0, 1e-10, div)
    xyx = xyz[:, 0] / div
    xyy = xyz[:, 1] / div
    yv = np.power(np.maximum(xyz[:, 1], 0.0), 0.9811)
    yden = np.maximum(xyy, 1e-10)
    xyz2 = np.stack([xyx * yv / yden, yv, (1.0 - xyx - xyy) * yv / yden], axis=1)
    lin = vm(vm(xyz2, xyz_ap1), odt_sat)
    disp = vm(vm(vm(lin, ap1_xyz), d60_d65), xyz_709)
    disp = np.clip(disp, 0.0, 1.0)
    # --- BT.1886 γ2.4 逆 EOTF + 8-bit 量化（dither off）---
    return np.clip(np.floor(np.power(disp, 1.0 / 2.4) * 255.0 + 0.5), 0.0, 255.0).astype(np.uint8)


def main() -> int:
    tsr_p = HERE / "off_tsr_f32.bin"
    raw_p = HERE / "off.raw"
    raw = np.frombuffer(raw_p.read_bytes(), dtype=np.uint8)
    w, h = (int(v) for v in np.frombuffer(raw[:8].tobytes(), dtype="<u4"))
    meas = raw[8:].reshape(-1, 4)[:, [2, 1, 0]]  # BGRA8 → RGB（receipt bgra8_unorm）
    tsr = np.fromfile(tsr_p, dtype="<f4").astype(D).reshape(-1, 3)
    assert tsr.shape[0] == w * h == meas.shape[0], (tsr.shape, w, h, meas.shape)

    pred = host_encode_vec(tsr.copy(), HostAces())
    dch = np.abs(pred.astype(np.int16) - meas.astype(np.int16))
    d = dch.max(axis=1)
    n = int(d.shape[0])
    exact = int(np.sum(d == 0))
    gt1 = int(np.sum(d > 1))
    gt2 = int(np.sum(d > 2))
    wi = int(np.argmax(d))
    wy, wx = divmod(wi, w)

    # fan(1500,12) 地标（契约相机静态帧）。
    fi = 12 * w + 1500
    fan_meas = [int(v) for v in meas[fi]]
    fan_pred = [int(v) for v in pred[fi]]
    fan_expect = [144, 122, 77]
    fan_ok = all(abs(a - b) <= 2 for a, b in zip(fan_meas, fan_expect))

    # 0.18 灰设计点（host f64 + kernel f32 正形仿真双口径）。
    hm = HostAces()
    g18_disp = hm.run((0.18, 0.18, 0.18))
    g18_host8 = [int(min(max(math.floor(c ** (1.0 / 2.4) * 255.0 + 0.5), 0), 255)) for c in g18_disp]
    g18_dev8, _ = kernel_sim((0.18, 0.18, 0.18), pack_params(), transposed=False)

    report = {
        "schema": "rurix.a2b.encode_parity_probe.v1",
        "date": "2026-08-28",
        "inputs": {
            "tsr_f32": str(tsr_p.name),
            "presented_raw": str(raw_p.name),
            "frame": "all-off 臂末帧（契约相机静态；RURIX_G31_DUMP_F32 + --dump-present-raw 同帧）",
            "width": w,
            "height": h,
            "caliber": "dither off / autoexp off（aeg=1.0）；host = display::aces13 f64 金标准 + BT.1886 v^(1/2.4) + floor(v·255+0.5)",
        },
        "parity": {
            "pixels": n,
            "exact_match": exact,
            "exact_match_pct": round(100.0 * exact / n, 4),
            "diff_p50_lsb": float(np.percentile(d, 50)),
            "diff_p99_lsb": float(np.percentile(d, 99)),
            "diff_p100_lsb": int(d.max()),
            "pixels_gt_1lsb": gt1,
            "pixels_gt_2lsb": gt2,
            "worst_pixel": {"x": wx, "y": wy, "measured_rgb": [int(v) for v in meas[wi]], "predicted_rgb": [int(v) for v in pred[wi]]},
        },
        "fan_landmark": {
            "xy": [1500, 12],
            "measured_rgb": fan_meas,
            "host_predicted_rgb": fan_pred,
            "expected_rgb": fan_expect,
            "within_2lsb": fan_ok,
        },
        "gray_018_design_point": {
            "host_f64_rgb8": g18_host8,
            "kernel_f32_sim_rgb8": list(g18_dev8),
            "expected": 99,
            "display_linear": [round(c, 6) for c in g18_disp],
            "ok": g18_host8 == [99, 99, 99] and list(g18_dev8) == [99, 99, 99],
        },
        "verdict": "PASS" if (gt2 == 0 and fan_ok and g18_host8 == [99, 99, 99]) else "FAIL",
    }
    out = HERE / "encode_parity_report.json"
    out.write_text(json.dumps(report, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=1))
    return 0 if report["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
