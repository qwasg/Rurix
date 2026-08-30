#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""bluefan 全帧对拍：向量化 kernel 仿真（bug 臂/fixed 臂）× 实测 dump。

输入：a_default_tsr_f32.bin（TSR 输出 f32，encode 唯一输入）+ a_default.raw
      （device 实测 presented BGRA8）。
产出：
  1) bug 臂 vs 实测逐像素一致率（sanity：仿真忠实度 + 全帧无第二异源）
  2) fixed 臂 PNG（修复后全帧预览）+ 扇区 bug/fixed 并排对照图
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from bluefan_encode_sim import pack_params  # noqa: E402

F = np.float32
W, H = 1920, 1080


def vec_encode(rgb: np.ndarray, p: np.ndarray, transposed: bool) -> np.ndarray:
    """g31_display_encode.rx 向量化逐字仿真（f32；dither off）。rgb: (N,3) f32。"""
    c = lambda v: F(v)
    r0, g0, b0 = rgb[:, 0], rgb[:, 1], rgb[:, 2]
    ar = r0 * p[4] + g0 * p[7] + b0 * p[10]
    ag = r0 * p[5] + g0 * p[8] + b0 * p[11]
    ab = r0 * p[6] + g0 * p[9] + b0 * p[12]
    maxc = np.maximum(np.maximum(ar, ag), ab)
    minc = np.minimum(np.minimum(ar, ag), ab)
    sat = (np.maximum(maxc, c(1e-10)) - np.maximum(minc, c(1e-10))) / np.maximum(maxc, c(0.01))
    chroma = np.sqrt(np.maximum(ab * (ab - ag) + ag * (ag - ar) + ar * (ar - ab), c(0.0)))
    yc = (ab + ag + ar + c(1.75) * chroma) / c(3.0)
    sx = (sat - c(0.4)) / c(0.2)
    tsig = np.maximum(c(1.0) - np.abs(sx / c(2.0)), c(0.0))
    sgn = np.sign(sx).astype(F)
    sig = (c(1.0) + sgn * (c(1.0) - tsig * tsig)) / c(2.0)
    gg = c(0.05) * sig
    glow = np.where(
        yc <= c(2.0 / 3.0 * 0.08),
        gg,
        np.where(yc >= c(2.0 * 0.08), c(0.0), gg * (c(0.08) / np.maximum(yc, c(1e-30)) - c(0.5))),
    ).astype(F)
    added = c(1.0) + glow
    ar2, ag2, ab2 = ar * added, ag * added, ab * added
    # red modifier（中性色 hw=0）
    yy = c(1.7320508) * (ag2 - ab2)
    xx = c(2.0) * ar2 - ag2 - ab2
    ax, ay = np.abs(xx), np.abs(yy)
    mx = np.maximum(ax, ay)
    z = np.where(mx > 0, np.minimum(ax, ay) / np.maximum(mx, c(1e-30)), c(0.0)).astype(F)
    at = c(0.78539816) * z - z * (z - c(1.0)) * (c(0.2447) + c(0.0663) * z)
    ang = np.where(ay > ax, c(1.5707963) - at, at).astype(F)
    ang = np.where(xx < 0, c(3.1415927) - ang, ang).astype(F)
    ang = np.where(yy < 0, -ang, ang).astype(F)
    hue = ang * c(57.29578)
    hue = np.where(hue < 0, hue + c(360.0), hue).astype(F)
    hue = np.where(mx > 0, hue, c(0.0)).astype(F)
    centered = hue.copy()
    centered = np.where(centered < c(-180.0), centered + c(360.0), centered).astype(F)
    centered = np.where(centered > c(180.0), centered - c(360.0), centered).astype(F)
    in_win = (centered > c(-67.5)) & (centered < c(67.5))
    kc = (centered + c(67.5)) * c(4.0) / c(135.0)
    jf = np.floor(kc)
    tf = kc - jf
    y0 = tf * tf * tf * c(1.0 / 6.0)
    y1_ = tf * tf * tf * c(-3.0 / 6.0) + tf * tf * c(3.0 / 6.0) + tf * c(3.0 / 6.0) + c(1.0 / 6.0)
    y2_ = tf * tf * tf * c(3.0 / 6.0) + tf * tf * c(-1.0) + c(4.0 / 6.0)
    y3_ = tf * tf * tf * c(-1.0 / 6.0) + tf * tf * c(3.0 / 6.0) + tf * c(-3.0 / 6.0) + c(1.0 / 6.0)
    ybs = np.where(jf < 0.5, y0, np.where(jf < 1.5, y1_, np.where(jf < 2.5, y2_, y3_))).astype(F)
    hw = np.where(in_win, ybs * c(1.5), c(0.0)).astype(F)
    neutral = (ar2 == ag2) & (ag2 == ab2)
    hw = np.where(neutral, c(0.0), hw).astype(F)
    ar2 = ar2 + hw * sat * (c(0.03) - ar2) * c(1.0 - 0.82)
    cr = np.maximum(ar2, c(0.0))
    cg = np.maximum(ag2, c(0.0))
    cb = np.maximum(ab2, c(0.0))
    pr = np.clip(cr * p[13] + cg * p[16] + cb * p[19], c(0.0), c(65504.0))
    pg = np.clip(cr * p[14] + cg * p[17] + cb * p[20], c(0.0), c(65504.0))
    pb = np.clip(cr * p[15] + cg * p[18] + cb * p[21], c(0.0), c(65504.0))
    dr = pr * p[31] + pg * p[34] + pb * p[37]
    dg = pr * p[32] + pg * p[35] + pb * p[38]
    db = pr * p[33] + pg * p[36] + pb * p[39]

    def log10f(x):
        return (np.log2(x) * c(0.30103)).astype(F)

    def pow10f(x):
        return np.exp2(x * c(3.3219281)).astype(F)

    def spline(x, lo_at, hi_at, kminx, kmidx, kmaxx, kminy, kmaxy, slow, shigh, nseg):
        lminx, lmidx, lmaxx = log10f(p[kminx]), log10f(p[kmidx]), log10f(p[kmaxx])
        lminy, lmaxy = log10f(p[kminy]), log10f(p[kmaxy])
        lx = log10f(np.maximum(x, c(0.00006103515625)))
        # 段选择
        m_lo = lx <= lminx
        m_sp1 = (~m_lo) & (lx < lmidx)
        m_sp2 = (~m_lo) & (~m_sp1) & (lx < lmaxx)
        # 样条段1
        kc1 = c(nseg) * (lx - lminx) / (lmidx - lminx)
        j1 = np.clip(np.floor(kc1), 0, nseg - 1).astype(np.int64)
        t1 = (kc1 - j1.astype(F)).astype(F)
        cf0_1 = p[lo_at + j1]
        cf1_1 = p[lo_at + j1 + 1]
        cf2_1 = p[lo_at + j1 + 2]
        # 样条段2
        kc2 = c(nseg) * (lx - lmidx) / (lmaxx - lmidx)
        j2 = np.clip(np.floor(kc2), 0, nseg - 1).astype(np.int64)
        t2 = (kc2 - j2.astype(F)).astype(F)
        cf0_2 = p[hi_at + j2]
        cf1_2 = p[hi_at + j2 + 1]
        cf2_2 = p[hi_at + j2 + 2]
        cf0 = np.where(m_sp1, cf0_1, cf0_2).astype(F)
        cf1 = np.where(m_sp1, cf1_1, cf1_2).astype(F)
        cf2 = np.where(m_sp1, cf2_1, cf2_2).astype(F)
        t = np.where(m_sp1, t1, t2).astype(F)
        b0 = c(0.5) * cf0 - cf1 + c(0.5) * cf2
        if transposed:
            b1 = -cf0 + cf1 + c(0.5) * cf2
            b2 = c(0.5) * cf0
        else:
            b1 = -cf0 + cf1
            b2 = c(0.5) * cf0 + c(0.5) * cf1
        sp = t * t * b0 + t * b1 + b2
        lin_lo = lx * p[slow] + (lminy - p[slow] * lminx)
        lin_hi = lx * p[shigh] + (lmaxy - p[shigh] * lmaxx)
        return np.where(m_lo, lin_lo, np.where(m_sp1 | m_sp2, sp, lin_hi)).astype(F)

    o_r = pow10f(spline(dr, 85, 91, 97, 99, 101, 98, 102, 103, 104, 3.0))
    o_g = pow10f(spline(dg, 85, 91, 97, 99, 101, 98, 102, 103, 104, 3.0))
    o_b = pow10f(spline(db, 85, 91, 97, 99, 101, 98, 102, 103, 104, 3.0))
    xr = o_r * p[22] + o_g * p[25] + o_b * p[28]
    xg = o_r * p[23] + o_g * p[26] + o_b * p[29]
    xb = o_r * p[24] + o_g * p[27] + o_b * p[30]
    qr = xr * p[13] + xg * p[16] + xb * p[19]
    qg = xr * p[14] + xg * p[17] + xb * p[20]
    qb = xr * p[15] + xg * p[18] + xb * p[21]
    sr = pow10f(spline(qr, 105, 115, 125, 127, 129, 126, 130, 131, 132, 7.0))
    sg = pow10f(spline(qg, 105, 115, 125, 127, 129, 126, 130, 131, 132, 7.0))
    sb = pow10f(spline(qb, 105, 115, 125, 127, 129, 126, 130, 131, 132, 7.0))
    lr = (sr - c(0.02)) / c(47.98)
    lg = (sg - c(0.02)) / c(47.98)
    lb = (sb - c(0.02)) / c(47.98)
    x1 = lr * p[49] + lg * p[52] + lb * p[55]
    y1v = lr * p[50] + lg * p[53] + lb * p[56]
    z1 = lr * p[51] + lg * p[54] + lb * p[57]
    div = x1 + y1v + z1
    div = np.where(div == 0.0, c(1e-10), div).astype(F)
    xyx = x1 / div
    xyy = y1v / div
    y2v = np.power(np.maximum(y1v, c(0.0)), c(0.9811)).astype(F)
    yden = np.maximum(xyy, c(1e-10))
    x2 = xyx * y2v / yden
    z2 = (c(1.0) - xyx - xyy) * y2v / yden
    dr2 = x2 * p[58] + y2v * p[61] + z2 * p[64]
    dg2 = x2 * p[59] + y2v * p[62] + z2 * p[65]
    db2 = x2 * p[60] + y2v * p[63] + z2 * p[66]
    er = dr2 * p[40] + dg2 * p[43] + db2 * p[46]
    eg = dr2 * p[41] + dg2 * p[44] + db2 * p[47]
    eb = dr2 * p[42] + dg2 * p[45] + db2 * p[48]
    x3 = er * p[49] + eg * p[52] + eb * p[55]
    y3 = er * p[50] + eg * p[53] + eb * p[56]
    z3 = er * p[51] + eg * p[54] + eb * p[57]
    x4 = x3 * p[67] + y3 * p[70] + z3 * p[73]
    y4 = x3 * p[68] + y3 * p[71] + z3 * p[74]
    z4 = x3 * p[69] + y3 * p[72] + z3 * p[75]
    fr = np.clip(x4 * p[76] + y4 * p[79] + z4 * p[82], c(0.0), c(1.0))
    fg = np.clip(x4 * p[77] + y4 * p[80] + z4 * p[83], c(0.0), c(1.0))
    fb = np.clip(x4 * p[78] + y4 * p[81] + z4 * p[84], c(0.0), c(1.0))
    q = lambda v: np.clip(np.floor(np.power(v, c(0.41666666)) * c(255.0) + c(0.5)), 0.0, 255.0).astype(np.uint8)
    return np.stack([q(fr), q(fg), q(fb)], axis=1)


def main() -> int:
    p = pack_params()
    tsr = np.fromfile(HERE / "bluefan" / "a_default_tsr_f32.bin", dtype="<f4").reshape(-1, 3)
    raw = np.frombuffer((HERE / "bluefan" / "a_default.raw").read_bytes(), dtype=np.uint8, offset=8).reshape(-1, 4)
    meas = raw[:, [2, 1, 0]]  # BGRA→RGB
    bug = vec_encode(tsr.astype(F), p, transposed=True)
    fix = vec_encode(tsr.astype(F), p, transposed=False)
    d = np.abs(bug.astype(np.int16) - meas.astype(np.int16)).max(axis=1)
    n = len(d)
    print(f"bug-arm vs measured: exact={np.sum(d == 0)}/{n} ({100.0 * np.sum(d == 0) / n:.4f}%)  |diff|<=1: {100.0 * np.sum(d <= 1) / n:.4f}%  max_diff={d.max()}")
    ys, xs = np.divmod(np.argmax(d), W)
    print(f"  worst px=({xs},{ys}) meas={meas[np.argmax(d)]} sim={bug[np.argmax(d)]}")
    # fixed 全帧 + 扇区并排对照
    fiximg = fix.reshape(H, W, 3)
    bugimg = bug.reshape(H, W, 3)
    Image.fromarray(fiximg, "RGB").save(HERE / "bluefan" / "fixed_encode_full.png")
    x0, x1_, y0, y1_ = 1300, 1720, 0, 220
    pair = np.concatenate([bugimg[y0:y1_, x0:x1_], np.full((y1_ - y0, 4, 3), 255, np.uint8), fiximg[y0:y1_, x0:x1_]], axis=1)
    Image.fromarray(pair, "RGB").resize((pair.shape[1] * 2, pair.shape[0] * 2), Image.NEAREST).save(HERE / "bluefan" / "fan_bug_vs_fixed.png")
    print("wrote fixed_encode_full.png + fan_bug_vs_fixed.png (left=bug/right=fixed, x2)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
