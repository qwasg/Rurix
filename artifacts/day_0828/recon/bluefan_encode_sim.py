#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""bluefan 根因证明：g31_display_encode.rx 逐字 f32 仿真 vs host aces13 f64 参考。

三臂对拍（同一 TSR 输出实测输入，来自 RURIX_G31_DUMP_F32 dump）：
  A) kernel-as-written：样条基 b1=−cf0+cf1+0.5·cf2 / b2=0.5·cf0（M·cf 转置形）
  B) kernel-fixed     ：b1=cf1−cf0 / b2=0.5·(cf0+cf1)（host vmul(cf,M)=cf·M 正形）
  C) host-f64         ：aces13.rs 逐字移植（correct 金标准）
判定：A == 实测 presented 字节（钉死根因层位）；B ≈ C（钉死最小修复）。
"""
from __future__ import annotations

import json
import math
from pathlib import Path

import numpy as np

F = np.float32

# ── color.rs 逐字（f64）────────────────────────────────────────────────
AP0 = {"red": [0.7347, 0.2653], "green": [0.0, 1.0], "blue": [0.0001, -0.077], "white": [0.32168, 0.33767]}
AP1 = {"red": [0.713, 0.293], "green": [0.165, 0.83], "blue": [0.128, 0.044], "white": [0.32168, 0.33767]}
REC709 = {"red": [0.64, 0.33], "green": [0.30, 0.60], "blue": [0.15, 0.06], "white": [0.3127, 0.3290]}
BRADFORD = [[0.89510, -0.75020, 0.03890], [0.26640, 1.71350, -0.06850], [-0.16140, 0.03670, 1.02960]]


def vmul(v, m):
    return [
        v[0] * m[0][0] + v[1] * m[1][0] + v[2] * m[2][0],
        v[0] * m[0][1] + v[1] * m[1][1] + v[2] * m[2][1],
        v[0] * m[0][2] + v[1] * m[1][2] + v[2] * m[2][2],
    ]


def mmul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def minv(m):
    det = (
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    )
    i = 1.0 / det
    return [
        [(m[1][1] * m[2][2] - m[1][2] * m[2][1]) * i, (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * i, (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * i],
        [(m[1][2] * m[2][0] - m[1][0] * m[2][2]) * i, (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * i, (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * i],
        [(m[1][0] * m[2][1] - m[1][1] * m[2][0]) * i, (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * i, (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * i],
    ]


def xy_y_to_xyz(x, y, yy):
    return [x * yy / max(y, 1e-10), yy, (1.0 - x - y) * yy / max(y, 1e-10)]


def rgb_to_xyz(c):
    y = 1.0
    x = c["white"][0] * y / c["white"][1]
    z = (1.0 - c["white"][0] - c["white"][1]) * y / c["white"][1]
    r, g, b = c["red"], c["green"], c["blue"]
    d = r[0] * (b[1] - g[1]) + b[0] * (g[1] - r[1]) + g[0] * (r[1] - b[1])
    sr = (x * (b[1] - g[1]) - g[0] * (y * (b[1] - 1.0) + b[1] * (x + z)) + b[0] * (y * (g[1] - 1.0) + g[1] * (x + z))) / d
    sg = (x * (r[1] - b[1]) + r[0] * (y * (b[1] - 1.0) + b[1] * (x + z)) - b[0] * (y * (r[1] - 1.0) + r[1] * (x + z))) / d
    sb = (x * (g[1] - r[1]) - r[0] * (y * (g[1] - 1.0) + g[1] * (x + z)) + g[0] * (y * (r[1] - 1.0) + r[1] * (x + z))) / d
    return [
        [sr * r[0], sr * r[1], sr * (1.0 - r[0] - r[1])],
        [sg * g[0], sg * g[1], sg * (1.0 - g[0] - g[1])],
        [sb * b[0], sb * b[1], sb * (1.0 - b[0] - b[1])],
    ]


def cat_bradford(sw, dw):
    s = vmul(xy_y_to_xyz(sw[0], sw[1], 1.0), BRADFORD)
    dd = vmul(xy_y_to_xyz(dw[0], dw[1], 1.0), BRADFORD)
    vk = [[dd[0] / s[0], 0, 0], [0, dd[1] / s[1], 0], [0, 0, dd[2] / s[2]]]
    return mmul(BRADFORD, mmul(vk, minv(BRADFORD)))


def rgb_to_rgb(src, dst):
    return mmul(rgb_to_xyz(src), mmul(cat_bradford(src["white"], dst["white"]), minv(rgb_to_xyz(dst))))


def sat_adjust(sat, rgb2y):
    return [[(1.0 - sat) * rgb2y[i] + (sat if i == j else 0.0) for j in range(3)] for i in range(3)]


# ── aces13.rs 样条参数（逐字）──────────────────────────────────────────
SPLINE_M = [[0.5, -1.0, 0.5], [-1.0, 1.0, 0.5], [0.5, 0.0, 0.0]]
C5 = {
    "coefs_low": [-4.0, -4.0, -3.1573765773, -0.4852499958, 1.8477324706, 1.8477324706],
    "coefs_high": [-0.7185482425, 2.0810307172, 3.6681241237, 4.0, 4.0, 4.0],
    "min_point": [0.18 * 3.0517578125e-05, 0.0001],
    "mid_point": [0.18, 4.8],
    "max_point": [0.18 * 262144.0, 10000.0],
    "slope_low": 0.0,
    "slope_high": 0.0,
}
HALF_MIN = 6.103515625e-05


def c5_fwd(x, c=C5):
    logx = math.log10(max(x, HALF_MIN))
    lminx, lmidx, lmaxx = (math.log10(c["min_point"][0]), math.log10(c["mid_point"][0]), math.log10(c["max_point"][0]))
    if logx <= lminx:
        logy = logx * c["slope_low"] + (math.log10(c["min_point"][1]) - c["slope_low"] * lminx)
    elif logx < lmidx:
        kc = 3.0 * (logx - lminx) / (lmidx - lminx)
        j = int(kc)
        t = kc - j
        cf = c["coefs_low"][j : j + 3]
        b = vmul(cf, SPLINE_M)
        logy = t * t * b[0] + t * b[1] + b[2]
    elif logx < lmaxx:
        kc = 3.0 * (logx - lmidx) / (lmaxx - lmidx)
        j = int(kc)
        t = kc - j
        cf = c["coefs_high"][j : j + 3]
        b = vmul(cf, SPLINE_M)
        logy = t * t * b[0] + t * b[1] + b[2]
    else:
        logy = logx * c["slope_high"] + (math.log10(c["max_point"][1]) - c["slope_high"] * lmaxx)
    return 10.0**logy


C9 = {
    "coefs_low": [-1.6989700043, -1.6989700043, -1.4779, -1.2291, -0.8648, -0.448, 0.00518, 0.4511080334, 0.9113744414, 0.9113744414],
    "coefs_high": [0.5154386965, 0.8470437783, 1.1358, 1.3802, 1.5197, 1.5985, 1.6467, 1.6746091357, 1.687873339, 1.687873339],
    "min_point": [c5_fwd(0.18 * 2.0**-6.5), 0.02],
    "mid_point": [c5_fwd(0.18), 4.8],
    "max_point": [c5_fwd(0.18 * 2.0**6.5), 48.0],
    "slope_low": 0.0,
    "slope_high": 0.04,
}


def c9_fwd(x, c=C9):
    logx = math.log10(max(x, HALF_MIN))
    lminx, lmidx, lmaxx = (math.log10(c["min_point"][0]), math.log10(c["mid_point"][0]), math.log10(c["max_point"][0]))
    if logx <= lminx:
        logy = logx * c["slope_low"] + (math.log10(c["min_point"][1]) - c["slope_low"] * lminx)
    elif logx < lmidx:
        kc = 7.0 * (logx - lminx) / (lmidx - lminx)
        j = int(kc)
        t = kc - j
        cf = c["coefs_low"][j : j + 3]
        b = vmul(cf, SPLINE_M)
        logy = t * t * b[0] + t * b[1] + b[2]
    elif logx < lmaxx:
        kc = 7.0 * (logx - lmidx) / (lmaxx - lmidx)
        j = int(kc)
        t = kc - j
        cf = c["coefs_high"][j : j + 3]
        b = vmul(cf, SPLINE_M)
        logy = t * t * b[0] + t * b[1] + b[2]
    else:
        logy = logx * c["slope_high"] + (math.log10(c["max_point"][1]) - c["slope_high"] * lmaxx)
    return 10.0**logy


# ── host aces13 f64 逐字（金标准）───────────────────────────────────────
class HostAces:
    def __init__(self):
        self.rec709_to_ap0 = rgb_to_rgb(REC709, AP0)
        self.ap0_to_ap1 = mmul(rgb_to_xyz(AP0), minv(rgb_to_xyz(AP1)))
        self.ap1_to_ap0 = mmul(rgb_to_xyz(AP1), minv(rgb_to_xyz(AP0)))
        self.ap1_to_xyz = rgb_to_xyz(AP1)
        y = [self.ap1_to_xyz[0][1], self.ap1_to_xyz[1][1], self.ap1_to_xyz[2][1]]
        self.rrt_sat = sat_adjust(0.96, y)
        self.odt_sat = sat_adjust(0.93, y)
        self.xyz_to_ap1 = minv(self.ap1_to_xyz)
        self.d60_to_d65 = cat_bradford(AP0["white"], REC709["white"])
        self.xyz_to_rec709 = minv(rgb_to_xyz(REC709))

    def run(self, rgb):
        aces = vmul(list(rgb), self.rec709_to_ap0)
        # glow
        tiny = 1e-10
        maxc, minc = max(aces), min(aces)
        sat = (max(maxc, tiny) - max(minc, tiny)) / max(maxc, 1e-2)
        r, g, b = aces
        chroma = math.sqrt(max(b * (b - g) + g * (g - r) + r * (r - b), 0.0))
        yc = (b + g + r + 1.75 * chroma) / 3.0
        x = (sat - 0.4) / 0.2
        t = max(1.0 - abs(x / 2.0), 0.0)
        sign = -1.0 if x < 0 else (1.0 if x > 0 else 0.0)
        s = (1.0 + sign * (1.0 - t * t)) / 2.0
        gg = 0.05 * s
        if yc <= 2.0 / 3.0 * 0.08:
            glow = gg
        elif yc >= 2.0 * 0.08:
            glow = 0.0
        else:
            glow = gg * (0.08 / yc - 0.5)
        aces = [c * (1.0 + glow) for c in aces]
        # red modifier
        if not (aces[0] == aces[1] == aces[2]):
            hue = math.degrees(math.atan2(math.sqrt(3.0) * (aces[1] - aces[2]), 2.0 * aces[0] - aces[1] - aces[2]))
            if hue < 0:
                hue += 360.0
            centered = hue
            if centered < -180.0:
                centered += 360.0
            elif centered > 180.0:
                centered -= 360.0
            w = 135.0
            M4 = [
                [-1 / 6, 3 / 6, -3 / 6, 1 / 6],
                [3 / 6, -6 / 6, 3 / 6, 0.0],
                [-3 / 6, 0.0, 3 / 6, 0.0],
                [1 / 6, 4 / 6, 1 / 6, 0.0],
            ]
            knots = [-w / 2, -w / 4, 0.0, w / 4, w / 2]
            hw = 0.0
            if knots[0] < centered < knots[4]:
                kc = (centered - knots[0]) * 4.0 / w
                j = int(kc)
                tt = kc - j
                mono = [tt**3, tt**2, tt, 1.0]
                if j < 4:
                    hw = sum(mono[i] * M4[i][3 - j] for i in range(4))
            hw *= 1.5
            aces[0] += hw * sat * (0.03 - aces[0]) * (1.0 - 0.82)
        aces = [max(c, 0.0) for c in aces]
        pre = vmul(aces, self.ap0_to_ap1)
        pre = [min(max(c, 0.0), 65504.0) for c in pre]
        pre = vmul(pre, self.rrt_sat)
        post = [c5_fwd(c) for c in pre]
        oces = vmul(post, self.ap1_to_ap0)
        # ODT
        pre2 = vmul(oces, self.ap0_to_ap1)
        post2 = [c9_fwd(c) for c in pre2]
        cb = 10.0 ** math.log10(0.02)
        lin = [(c - cb) / (48.0 - cb) for c in post2]
        xyz = vmul(lin, self.ap1_to_xyz)
        div = xyz[0] + xyz[1] + xyz[2]
        if div == 0.0:
            div = 1e-10
        xyx, xyy = xyz[0] / div, xyz[1] / div
        y2 = max(xyz[1], 0.0) ** 0.9811
        xyz2 = xy_y_to_xyz(xyx, xyy, y2)
        lin = vmul(xyz2, self.xyz_to_ap1)
        lin = vmul(lin, self.odt_sat)
        xyz = vmul(lin, self.ap1_to_xyz)
        xyz = vmul(xyz, self.d60_to_d65)
        disp = vmul(xyz, self.xyz_to_rec709)
        return [min(max(c, 0.0), 1.0) for c in disp]


# ── device params 打包（aces13_device_encode_params 逐字 → f32）─────────
def pack_params(w=1920, h=1080, bgra=True):
    hm = HostAces()
    v = [float(w), float(h), 1.0 if bgra else 0.0, 0.0]
    for m in (hm.rec709_to_ap0, hm.ap0_to_ap1, hm.ap1_to_ap0, hm.rrt_sat, hm.odt_sat, hm.ap1_to_xyz, hm.xyz_to_ap1, hm.d60_to_d65, hm.xyz_to_rec709):
        for row in m:
            v.extend(row)
    v.extend(C5["coefs_low"])
    v.extend(C5["coefs_high"])
    v.extend([C5["min_point"][0], C5["min_point"][1], C5["mid_point"][0], C5["mid_point"][1], C5["max_point"][0], C5["max_point"][1], C5["slope_low"], C5["slope_high"]])
    v.extend(C9["coefs_low"])
    v.extend(C9["coefs_high"])
    v.extend([C9["min_point"][0], C9["min_point"][1], C9["mid_point"][0], C9["mid_point"][1], C9["max_point"][0], C9["max_point"][1], C9["slope_low"], C9["slope_high"]])
    v.extend([0.0, 0.0, 0.0])
    assert len(v) == 136
    return np.array(v, dtype=F)


# ── kernel f32 逐字仿真（transposed=True 复刻 bug；False = 正形修复）────
def kernel_sim(rgb_in, p, transposed: bool):
    def f(x):
        return F(x)

    def log10f(x):
        return F(F(np.log2(f(x))) * F(0.30103))

    def pow10f(y):
        return F(np.exp2(F(f(y) * F(3.3219281))))

    def basis(cf0, cf1, cf2):
        b0 = F(F(F(0.5) * cf0) - cf1 + F(F(0.5) * cf2))
        if transposed:
            b1 = F(-cf0 + cf1 + F(F(0.5) * cf2))
            b2 = F(F(0.5) * cf0)
        else:
            b1 = F(-cf0 + cf1)
            b2 = F(F(0.5) * cf0 + F(0.5) * cf1)
        return b0, b1, b2

    def spline(x, lminx, lmidx, lmaxx, lminy, lmaxy, clow_at, chigh_at, slow, shigh, nseg):
        lx = log10f(max(f(x), F(0.00006103515625)))
        if lx <= lminx:
            return F(lx * slow + (lminy - slow * lminx))
        if lx < lmidx:
            kc = F(F(nseg) * (lx - lminx) / (lmidx - lminx))
            j = int(np.floor(kc))
            t = F(kc - F(j))
            cf0, cf1, cf2 = p[clow_at + j], p[clow_at + j + 1], p[clow_at + j + 2]
        elif lx < lmaxx:
            kc = F(F(nseg) * (lx - lmidx) / (lmaxx - lmidx))
            j = int(np.floor(kc))
            t = F(kc - F(j))
            cf0, cf1, cf2 = p[chigh_at + j], p[chigh_at + j + 1], p[chigh_at + j + 2]
        else:
            return F(lx * shigh + (lmaxy - shigh * lmaxx))
        b0, b1, b2 = basis(cf0, cf1, cf2)
        return F(F(t * t) * b0 + t * b1 + b2)

    r0, g0, b0_ = f(rgb_in[0]), f(rgb_in[1]), f(rgb_in[2])
    ar = F(r0 * p[4] + g0 * p[7] + b0_ * p[10])
    ag = F(r0 * p[5] + g0 * p[8] + b0_ * p[11])
    ab = F(r0 * p[6] + g0 * p[9] + b0_ * p[12])
    maxc = F(max(ar, ag, ab))
    minc = F(min(ar, ag, ab))
    sat = F((max(maxc, F(1e-10)) - max(minc, F(1e-10))) / max(maxc, F(0.01)))
    chroma = F(np.sqrt(max(F(ab * (ab - ag) + ag * (ag - ar) + ar * (ar - ab)), F(0.0))))
    yc = F((ab + ag + ar + F(1.75) * chroma) / F(3.0))
    sx = F((sat - F(0.4)) / F(0.2))
    tsig = F(max(F(1.0) - abs(F(sx / F(2.0))), F(0.0)))
    sgn = F(-1.0) if sx < 0.0 else (F(1.0) if sx > 0.0 else F(0.0))
    sig = F((F(1.0) + sgn * (F(1.0) - tsig * tsig)) / F(2.0))
    gg = F(F(0.05) * sig)
    if yc <= F(2.0 / 3.0 * 0.08):
        glow = gg
    elif yc >= F(2.0 * 0.08):
        glow = F(0.0)
    else:
        glow = F(gg * (F(0.08) / yc - F(0.5)))
    added = F(F(1.0) + glow)
    ar2, ag2, ab2 = F(ar * added), F(ag * added), F(ab * added)
    hw = F(0.0)
    if not (ar2 == ag2 and ag2 == ab2):
        yy = F(F(1.7320508) * (ag2 - ab2))
        xx = F(F(2.0) * ar2 - ag2 - ab2)
        ax, ay = abs(xx), abs(yy)
        mx = F(max(ax, ay))
        hue = F(0.0)
        if mx > 0.0:
            z = F(min(ax, ay) / mx)
            at = F(F(0.78539816) * z - z * (z - F(1.0)) * (F(0.2447) + F(0.0663) * z))
            ang = at
            if ay > ax:
                ang = F(F(1.5707963) - at)
            if xx < 0.0:
                ang = F(F(3.1415927) - ang)
            if yy < 0.0:
                ang = F(-ang)
            hue = F(ang * F(57.29578))
            if hue < 0.0:
                hue = F(hue + F(360.0))
        centered = hue
        if centered < F(-180.0):
            centered = F(centered + F(360.0))
        if centered > F(180.0):
            centered = F(centered - F(360.0))
        if F(-67.5) < centered < F(67.5):
            kc = F((centered + F(67.5)) * F(4.0) / F(135.0))
            jf = F(np.floor(kc))
            tf = F(kc - jf)
            if jf < 0.5:
                y = F(tf * tf * tf * F(1.0 / 6.0))
            elif jf < 1.5:
                y = F(tf * tf * tf * F(-3.0 / 6.0) + tf * tf * F(3.0 / 6.0) + tf * F(3.0 / 6.0) + F(1.0 / 6.0))
            elif jf < 2.5:
                y = F(tf * tf * tf * F(3.0 / 6.0) + tf * tf * F(-1.0) + F(4.0 / 6.0))
            else:
                y = F(tf * tf * tf * F(-1.0 / 6.0) + tf * tf * F(3.0 / 6.0) + tf * F(-3.0 / 6.0) + F(1.0 / 6.0))
            hw = F(y * F(1.5))
    ar2 = F(ar2 + hw * sat * (F(0.03) - ar2) * F(1.0 - 0.82))
    cr, cg, cb_ = F(max(ar2, F(0.0))), F(max(ag2, F(0.0))), F(max(ab2, F(0.0)))
    pr = F(min(max(F(cr * p[13] + cg * p[16] + cb_ * p[19]), F(0.0)), F(65504.0)))
    pg = F(min(max(F(cr * p[14] + cg * p[17] + cb_ * p[20]), F(0.0)), F(65504.0)))
    pb = F(min(max(F(cr * p[15] + cg * p[18] + cb_ * p[21]), F(0.0)), F(65504.0)))
    dr = F(pr * p[31] + pg * p[34] + pb * p[37])
    dg = F(pr * p[32] + pg * p[35] + pb * p[38])
    db = F(pr * p[33] + pg * p[36] + pb * p[39])
    c5a = (log10f(p[97]), log10f(p[99]), log10f(p[101]), log10f(p[98]), log10f(p[102]))
    o_r = pow10f(spline(dr, *c5a, 85, 91, p[103], p[104], 3.0))
    o_g = pow10f(spline(dg, *c5a, 85, 91, p[103], p[104], 3.0))
    o_b = pow10f(spline(db, *c5a, 85, 91, p[103], p[104], 3.0))
    xr = F(o_r * p[22] + o_g * p[25] + o_b * p[28])
    xg = F(o_r * p[23] + o_g * p[26] + o_b * p[29])
    xb = F(o_r * p[24] + o_g * p[27] + o_b * p[30])
    qr = F(xr * p[13] + xg * p[16] + xb * p[19])
    qg = F(xr * p[14] + xg * p[17] + xb * p[20])
    qb = F(xr * p[15] + xg * p[18] + xb * p[21])
    c9a = (log10f(p[125]), log10f(p[127]), log10f(p[129]), log10f(p[126]), log10f(p[130]))
    sr = pow10f(spline(qr, *c9a, 105, 115, p[131], p[132], 7.0))
    sg = pow10f(spline(qg, *c9a, 105, 115, p[131], p[132], 7.0))
    sb = pow10f(spline(qb, *c9a, 105, 115, p[131], p[132], 7.0))
    lr = F((sr - F(0.02)) / F(47.98))
    lg = F((sg - F(0.02)) / F(47.98))
    lb = F((sb - F(0.02)) / F(47.98))
    x1 = F(lr * p[49] + lg * p[52] + lb * p[55])
    y1 = F(lr * p[50] + lg * p[53] + lb * p[56])
    z1 = F(lr * p[51] + lg * p[54] + lb * p[57])
    div = F(x1 + y1 + z1)
    if div == 0.0:
        div = F(1e-10)
    xyx = F(x1 / div)
    xyy = F(y1 / div)
    y2 = F(np.power(max(y1, F(0.0)), F(0.9811)))
    yden = F(max(xyy, F(1e-10)))
    x2 = F(xyx * y2 / yden)
    z2 = F((F(1.0) - xyx - xyy) * y2 / yden)
    dr2 = F(x2 * p[58] + y2 * p[61] + z2 * p[64])
    dg2 = F(x2 * p[59] + y2 * p[62] + z2 * p[65])
    db2 = F(x2 * p[60] + y2 * p[63] + z2 * p[66])
    er = F(dr2 * p[40] + dg2 * p[43] + db2 * p[46])
    eg = F(dr2 * p[41] + dg2 * p[44] + db2 * p[47])
    eb = F(dr2 * p[42] + dg2 * p[45] + db2 * p[48])
    x3 = F(er * p[49] + eg * p[52] + eb * p[55])
    y3 = F(er * p[50] + eg * p[53] + eb * p[56])
    z3 = F(er * p[51] + eg * p[54] + eb * p[57])
    x4 = F(x3 * p[67] + y3 * p[70] + z3 * p[73])
    y4 = F(x3 * p[68] + y3 * p[71] + z3 * p[74])
    z4 = F(x3 * p[69] + y3 * p[72] + z3 * p[75])
    fr = F(min(max(F(x4 * p[76] + y4 * p[79] + z4 * p[82]), F(0.0)), F(1.0)))
    fg = F(min(max(F(x4 * p[77] + y4 * p[80] + z4 * p[83]), F(0.0)), F(1.0)))
    fb = F(min(max(F(x4 * p[78] + y4 * p[81] + z4 * p[84]), F(0.0)), F(1.0)))
    q = lambda v: int(min(max(np.floor(F(np.power(v, F(0.41666666)) * F(255.0) + F(0.5))), F(0.0)), F(255.0)))
    return (q(fr), q(fg), q(fb)), (float(fr), float(fg), float(fb))


def q8_host(disp):
    return tuple(int(min(max(math.floor(c ** (1.0 / 2.4) * 255.0 + 0.5), 0), 255)) for c in disp)


def main():
    p = pack_params()
    host = HostAces()
    probes = {
        "fan(1500,12)  TSR-out": (0.35686296, 0.25368458, 0.10629952),
        "wall(1700,430) TSR-out": (0.00186373, 0.00055717, 0.00041163),
        "gray 0.18": (0.18, 0.18, 0.18),
    }
    measured = {"fan(1500,12)  TSR-out": (0, 62, 170), "wall(1700,430) TSR-out": (25, 13, 16)}
    out = {}
    for name, rgb in probes.items():
        bug8, bugf = kernel_sim(rgb, p, transposed=True)
        fix8, fixf = kernel_sim(rgb, p, transposed=False)
        hostd = host.run(rgb)
        host8 = q8_host(hostd)
        out[name] = {
            "input_linear": [round(v, 7) for v in rgb],
            "A_kernel_as_written_RGB8": list(bug8),
            "B_kernel_fixed_basis_RGB8": list(fix8),
            "C_host_f64_reference_RGB8": list(host8),
            "measured_presented_RGB8": list(measured.get(name, ())) or None,
        }
        print(f"{name}: in={rgb}")
        print(f"  A kernel-as-written (transposed basis) -> {bug8}  display_lin={tuple(round(v,5) for v in bugf)}")
        print(f"  B kernel-fixed      (cf x M correct)   -> {fix8}  display_lin={tuple(round(v,5) for v in fixf)}")
        print(f"  C host f64 aces13 reference            -> {host8}  display_lin={tuple(round(v,5) for v in hostd)}")
        if name in measured:
            print(f"  MEASURED presented (run bluefan_a)      -> {measured[name]}")
    Path(__file__).with_name("bluefan_sim_report.json").write_text(json.dumps(out, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
