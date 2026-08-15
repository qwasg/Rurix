#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.4b 波）
"""G10.4b M135 FLIP 自实现（spec/visual_comparison.md RXS-0389 L2~L5；RFC-0026 §4.2）。

LDR-FLIP 口径管道逐字移植（numpy，与参考实现 NVlabs/flip python-nanobind 臂
对拍；参考语义源 = pin commit `src/cpp/FLIP.h` 单一事实源）：

  sRGB [0,1] → clamp → sRGB→linear RGB → XYZ(D65) → YCxCz
  → 分离空间滤波（YCx / Cz 双滤波器组，边缘 clamp 寻址）
  → YCxCz→XYZ→linear RGB clamp [0,1] → XYZ→CIELAB
  → Hunt 调整（0.01·L·a / 0.01·L·b）→ HyAB 距离 → ^gqc
  → cmax/pccmax 分段重映射（gpt 拐点）
  → 特征滤波（高斯一阶/二阶导，边缘·点检测，Y 归一 y/116+16/116）
  → 特征差 = max(边缘差, 点差)/√2 → ^gqf
  → errorFLIP = color_diff ^ (1 − feature_diff)
  → 标量 = 全图算术均值（spatial_pooling 加权均值聚合，RXS-0389 L3）。

常量闭集（RXS-0389 L3 冻结字面；漂移注入即 RED 的判定基准）：
gqc=0.7 / gpc=0.4 / gpt=0.95 / gw=0.082 / gqf=0.5；
a1=(1.0,1.0,34.1) / b1=(0.0047,0.0053,0.04) / a2=(0,0,13.5) / b2=(1e-5,1e-5,0.025)。

**域限定**：LDR-FLIP 仅在 LDR 臂定义（显示域 sRGB [0,1]）；HDR 帧直算
fail-closed 拒绝（domain 互证错配即 CaliberError，RXS-0389 L2）。
ppd 策略冻结：全语料单一值 = 参考实现默认（0.7m / 3840px / 0.7m 推导，
float32 位级复算与参考返回参数字典逐位一致）。
"""
from __future__ import annotations

import math

import numpy as np


class CaliberError(ValueError):
    """口径违例（域错配 / 闭集外参数漂移）——fail-closed。"""


# ---- 口径常量闭集（RXS-0389 L3 冻结字面） --------------------------------
GQC = 0.7
GPC = 0.4
GPT = 0.95
GW = 0.082
GQF = 0.5

GAUSS_A1 = (1.0, 1.0, 34.1)
GAUSS_B1 = (0.0047, 0.0053, 0.04)
GAUSS_A2 = (0.0, 0.0, 13.5)
GAUSS_B2 = (1.0e-5, 1.0e-5, 0.025)

# D65 参考白（参考实现 DEFAULT_ILLUMINANT / INV_DEFAULT_ILLUMINANT）。
DEFAULT_ILLUMINANT = (0.950428545, 1.000000000, 1.088900371)
INV_DEFAULT_ILLUMINANT = (1.052156925, 1.000000000, 0.918357670)


def default_ppd() -> float:
    """参考实现默认 PPD（0.7m / 3840px / 0.7m，float32 位级复算）。

    C++ `FLIP::calculatePPD(0.7f, 3840.0f, 0.7f)`
    = 0.7f · (3840.0f/0.7f) · (float(π)/180.0f)；与参考返回参数字典
    `parameters["ppd"]` 逐位一致（M135 门内机核）。
    """
    f32 = np.float32
    return float(f32(f32(0.7) * f32(f32(3840.0) / f32(0.7))) * f32(f32(np.pi) / f32(180.0)))


# ---- 色彩变换（参考实现 color3 静态面逐字） ------------------------------
def srgb_to_linear(img: np.ndarray) -> np.ndarray:
    out = np.empty_like(img)
    lo = img <= 0.04045
    out[lo] = img[lo] / 12.92
    out[~lo] = ((img[~lo] + 0.055) / 1.055) ** 2.4
    return out


def linear_rgb_to_xyz(rgb: np.ndarray) -> np.ndarray:
    a11, a12, a13 = 10135552.0 / 24577794.0, 8788810.0 / 24577794.0, 4435075.0 / 24577794.0
    a21, a22, a23 = 2613072.0 / 12288897.0, 8788810.0 / 12288897.0, 887015.0 / 12288897.0
    a31, a32, a33 = 1425312.0 / 73733382.0, 8788810.0 / 73733382.0, 70074185.0 / 73733382.0
    x = a11 * rgb[..., 0] + a12 * rgb[..., 1] + a13 * rgb[..., 2]
    y = a21 * rgb[..., 0] + a22 * rgb[..., 1] + a23 * rgb[..., 2]
    z = a31 * rgb[..., 0] + a32 * rgb[..., 1] + a33 * rgb[..., 2]
    return np.stack([x, y, z], axis=-1)


def xyz_to_linear_rgb(xyz: np.ndarray) -> np.ndarray:
    a11, a12, a13 = 3.241003275, -1.537398934, -0.498615861
    a21, a22, a23 = -0.969224334, 1.875930071, 0.041554224
    a31, a32, a33 = 0.055639423, -0.204011202, 1.057148933
    r = a11 * xyz[..., 0] + a12 * xyz[..., 1] + a13 * xyz[..., 2]
    g = a21 * xyz[..., 0] + a22 * xyz[..., 1] + a23 * xyz[..., 2]
    b = a31 * xyz[..., 0] + a32 * xyz[..., 1] + a33 * xyz[..., 2]
    return np.stack([r, g, b], axis=-1)


def xyz_to_ycxcz(xyz: np.ndarray) -> np.ndarray:
    xyz = xyz * np.asarray(INV_DEFAULT_ILLUMINANT)
    y = 116.0 * xyz[..., 1] - 16.0
    cx = 500.0 * (xyz[..., 0] - xyz[..., 1])
    cz = 200.0 * (xyz[..., 1] - xyz[..., 2])
    return np.stack([y, cx, cz], axis=-1)


def ycxcz_to_xyz(ycc: np.ndarray) -> np.ndarray:
    y = (ycc[..., 0] + 16.0) / 116.0
    cx = ycc[..., 1] / 500.0
    cz = ycc[..., 2] / 200.0
    x = y + cx
    z = y - cz
    return np.stack([x, y, z], axis=-1) * np.asarray(DEFAULT_ILLUMINANT)


def xyz_to_cielab(xyz: np.ndarray) -> np.ndarray:
    delta = 6.0 / 29.0
    delta_sq = delta * delta
    delta_cube = delta * delta_sq
    factor = 1.0 / (3.0 * delta_sq)
    term = 4.0 / 29.0
    xyz = xyz * np.asarray(INV_DEFAULT_ILLUMINANT)
    f = np.where(xyz > delta_cube, np.cbrt(np.maximum(xyz, 0.0)), factor * xyz + term)
    l = 116.0 * f[..., 1] - 16.0
    a = 500.0 * (f[..., 0] - f[..., 1])
    b = 200.0 * (f[..., 1] - f[..., 2])
    return np.stack([l, a, b], axis=-1)


def hunt(luminance: np.ndarray, chrominance: np.ndarray) -> np.ndarray:
    return 0.01 * luminance * chrominance


def _hyab(ref_lab_h: np.ndarray, test_lab_h: np.ndarray) -> np.ndarray:
    city = np.abs(ref_lab_h[..., 0] - test_lab_h[..., 0])
    euc = np.sqrt(
        (ref_lab_h[..., 1] - test_lab_h[..., 1]) ** 2
        + (ref_lab_h[..., 2] - test_lab_h[..., 2]) ** 2
    )
    return city + euc


def compute_max_distance() -> float:
    """cmax（参考实现 color3::computeMaxDistance(gqc) 逐字）。"""
    green_lab = xyz_to_cielab(linear_rgb_to_xyz(np.asarray([[[0.0, 1.0, 0.0]]])))[0, 0]
    blue_lab = xyz_to_cielab(linear_rgb_to_xyz(np.asarray([[[0.0, 0.0, 1.0]]])))[0, 0]
    green_h = np.asarray([green_lab[0], hunt(green_lab[0], green_lab[1]), hunt(green_lab[0], green_lab[2])])
    blue_h = np.asarray([blue_lab[0], hunt(blue_lab[0], blue_lab[1]), hunt(blue_lab[0], blue_lab[2])])
    return float(_hyab(green_h, blue_h) ** GQC)


# ---- 滤波器（参考实现 setSpatialFilters / setFeatureFilter 逐字） ---------
def _gaussian_alt(x2: np.ndarray, a: float, b: float) -> np.ndarray:
    return a * math.sqrt(math.pi / b) * np.exp(-(math.pi**2) * x2 / b)


def _gaussian_sqrt(x2: np.ndarray, a: float, b: float) -> np.ndarray:
    return math.sqrt(a * math.sqrt(math.pi / b)) * np.exp(-(math.pi**2) * x2 / b)


def spatial_filter_radius(ppd: float) -> int:
    max_scale = max(GAUSS_B1[0], GAUSS_B1[1], GAUSS_B1[2], GAUSS_B2[0], GAUSS_B2[1], GAUSS_B2[2])
    return int(math.ceil(3.0 * math.sqrt(max_scale / (2.0 * math.pi**2)) * ppd))


def set_spatial_filters(ppd: float, radius: int) -> tuple[np.ndarray, np.ndarray]:
    """返回 (filter_ycx[2r+1,2], filter_cz[2r+1,2])（参考实现归一化逐字）。"""
    delta_x = 1.0 / ppd
    width = 2 * radius + 1
    ycx = np.zeros((width, 2))
    cz = np.zeros((width, 2))
    for x in range(width):
        ix = (x - radius) * delta_x
        ix2 = ix * ix
        ycx[x, 0] = _gaussian_alt(np.asarray(ix2), GAUSS_A1[0], GAUSS_B1[0])
        ycx[x, 1] = _gaussian_alt(np.asarray(ix2), GAUSS_A1[1], GAUSS_B1[1])
        cz[x, 0] = _gaussian_sqrt(np.asarray(ix2), GAUSS_A1[2], GAUSS_B1[2])
        cz[x, 1] = _gaussian_sqrt(np.asarray(ix2), GAUSS_A2[2], GAUSS_B2[2])
    ycx[:, 0] /= ycx[:, 0].sum()
    ycx[:, 1] /= ycx[:, 1].sum()
    norm_cz = 1.0 / math.sqrt(float(cz[:, 0].sum() ** 2 + cz[:, 1].sum() ** 2))
    cz *= norm_cz
    return ycx, cz


def set_feature_filter(ppd: float) -> tuple[np.ndarray, int]:
    """返回 (filter[2r+1,3] = (g, dg, ddg), radius)（参考实现归一化逐字）。"""
    std_dev = 0.5 * GW * ppd
    radius = int(math.ceil(3.0 * std_dev))
    width = 2 * radius + 1
    g = np.zeros(width)
    dg = np.zeros(width)
    ddg = np.zeros(width)
    g_sum = 0.0
    dg_pos = 0.0
    dg_neg = 0.0
    ddg_pos = 0.0
    ddg_neg = 0.0
    for x in range(width):
        xx = float(x - radius)
        gv = math.exp(-(xx * xx) / (2.0 * std_dev * std_dev))
        dgv = -xx * gv
        ddgv = (xx * xx / (std_dev * std_dev) - 1.0) * gv
        g[x], dg[x], ddg[x] = gv, dgv, ddgv
        g_sum += gv
        if dgv > 0.0:
            dg_pos += dgv
        else:
            dg_neg -= dgv
        if ddgv > 0.0:
            ddg_pos += ddgv
        else:
            ddg_neg -= ddgv
    g /= g_sum
    dg = np.where(dg > 0.0, dg / dg_pos, dg / dg_neg)
    ddg = np.where(ddg > 0.0, ddg / ddg_pos, ddg / ddg_neg)
    return np.stack([g, dg, ddg], axis=-1), radius


def _separable_conv(img: np.ndarray, weights: np.ndarray, axis: int) -> np.ndarray:
    """1D 卷积（边缘 clamp 寻址 = np.pad edge；参考实现 Min/Max 钳位逐字）。"""
    radius = (len(weights) - 1) // 2
    pad = [(0, 0)] * img.ndim
    pad[axis] = (radius, radius)
    padded = np.pad(img, pad, mode="edge")
    out = np.zeros_like(img)
    n = img.shape[axis]
    for i, w in enumerate(weights):
        sl = [slice(None)] * img.ndim
        sl[axis] = slice(i, i + n)
        out = out + w * padded[tuple(sl)]
    return out


# ---- LDR-FLIP 主管道 ------------------------------------------------------
def flip_ldr(
    reference: np.ndarray,
    test: np.ndarray,
    ppd: float | None = None,
    *,
    domain: str = "display-referred-ldr",
) -> tuple[np.ndarray, float]:
    """LDR-FLIP：返回 (逐像素误差图 [0,1], 全图均值标量)。

    reference/test = LDR 显示域 sRGB [0,1] 帧（HxWx3 float）。domain 互证：
    仅 `"display-referred-ldr"` 合法；HDR 域直算 fail-closed（CaliberError）。
    """
    if domain != "display-referred-ldr":
        raise CaliberError(f"LDR-FLIP 域限定：domain={domain!r} 拒绝（仅 display-referred-ldr）")
    if ppd is None:
        ppd = default_ppd()
    if ppd <= 0.0:
        raise CaliberError(f"ppd 必须为正数: {ppd}")

    ref = np.clip(np.asarray(reference, dtype=np.float64), 0.0, 1.0)
    tst = np.clip(np.asarray(test, dtype=np.float64), 0.0, 1.0)
    if ref.shape != tst.shape or ref.ndim != 3 or ref.shape[2] != 3:
        raise CaliberError(f"帧形状非法/不一致: {ref.shape} vs {tst.shape}")

    # sRGB → linear → YCxCz。
    ref_ycc = xyz_to_ycxcz(linear_rgb_to_xyz(srgb_to_linear(ref)))
    tst_ycc = xyz_to_ycxcz(linear_rgb_to_xyz(srgb_to_linear(tst)))

    # 分离空间滤波（x 向 → y 向，边缘 clamp）。
    radius = spatial_filter_radius(ppd)
    f_ycx, f_cz = set_spatial_filters(ppd, radius)

    def _spatial(ycc: np.ndarray) -> np.ndarray:
        y_x = _separable_conv(ycc[..., 0], f_ycx[:, 0], axis=1)
        cx_x = _separable_conv(ycc[..., 1], f_ycx[:, 1], axis=1)
        cz1_x = _separable_conv(ycc[..., 2], f_cz[:, 0], axis=1)
        cz2_x = _separable_conv(ycc[..., 2], f_cz[:, 1], axis=1)
        y = _separable_conv(y_x, f_ycx[:, 0], axis=0)
        cx = _separable_conv(cx_x, f_ycx[:, 1], axis=0)
        cz = _separable_conv(cz1_x, f_cz[:, 0], axis=0) + _separable_conv(cz2_x, f_cz[:, 1], axis=0)
        return np.stack([y, cx, cz], axis=-1)

    ref_f = _spatial(ref_ycc)
    tst_f = _spatial(tst_ycc)

    # YCxCz → linear RGB clamp [0,1] → CIELAB → Hunt。
    ref_rgb = np.clip(xyz_to_linear_rgb(ycxcz_to_xyz(ref_f)), 0.0, 1.0)
    tst_rgb = np.clip(xyz_to_linear_rgb(ycxcz_to_xyz(tst_f)), 0.0, 1.0)
    ref_lab = xyz_to_cielab(linear_rgb_to_xyz(ref_rgb))
    tst_lab = xyz_to_cielab(linear_rgb_to_xyz(tst_rgb))
    ref_lab_h = np.stack(
        [ref_lab[..., 0], hunt(ref_lab[..., 0], ref_lab[..., 1]), hunt(ref_lab[..., 0], ref_lab[..., 2])],
        axis=-1,
    )
    tst_lab_h = np.stack(
        [tst_lab[..., 0], hunt(tst_lab[..., 0], tst_lab[..., 1]), hunt(tst_lab[..., 0], tst_lab[..., 2])],
        axis=-1,
    )

    # HyAB → ^gqc → cmax/pccmax 分段重映射。
    color_diff = _hyab(ref_lab_h, tst_lab_h) ** GQC
    cmax = compute_max_distance()
    pccmax = GPC * cmax
    color_diff = np.where(
        color_diff < pccmax,
        color_diff * (GPT / pccmax),
        GPT + ((color_diff - pccmax) / (cmax - pccmax)) * (1.0 - GPT),
    )

    # 特征滤波（Y 归一 y/116+16/116；边缘·点检测）。
    feature_filter, _fradius = set_feature_filter(ppd)
    g, dg, ddg = feature_filter[:, 0], feature_filter[:, 1], feature_filter[:, 2]

    def _features(ycc: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        y_norm = ycc[..., 0] / 116.0 + 16.0 / 116.0
        dx_x = _separable_conv(y_norm, dg, axis=1)
        ddx_x = _separable_conv(y_norm, ddg, axis=1)
        gf_x = _separable_conv(y_norm, g, axis=1)
        dx = _separable_conv(dx_x, g, axis=0)
        ddx = _separable_conv(ddx_x, g, axis=0)
        dy = _separable_conv(gf_x, dg, axis=0)
        ddy = _separable_conv(gf_x, ddg, axis=0)
        edge = np.sqrt(dx * dx + dy * dy)
        point = np.sqrt(ddx * ddx + ddy * ddy)
        return edge, point

    ref_edge, ref_point = _features(ref_ycc)
    tst_edge, tst_point = _features(tst_ycc)
    edge_diff = np.abs(ref_edge - tst_edge)
    point_diff = np.abs(ref_point - tst_point)
    feature_diff = (np.maximum(edge_diff, point_diff) / math.sqrt(2.0)) ** GQF

    error_map = color_diff ** (1.0 - feature_diff)
    h, w = error_map.shape
    mean = float(error_map.sum() / (w * h))
    return error_map, mean


def flip_ldr_caliber_literal(ppd: float) -> str:
    """metric_caliber 字面（digest 登记面；RXS-0389 L3 闭集 + ppd 策略冻结）。"""
    return (
        "flip-ldr{colorspace=YCxCz,gqc=0.7,gpc=0.4,gpt=0.95,gw=0.082,gqf=0.5,"
        "a1=(1.0,1.0,34.1),b1=(0.0047,0.0053,0.04),a2=(0.0,0.0,13.5),b2=(1e-5,1e-5,0.025),"
        f"ppd={ppd!r},ppd_strategy=reference-default-single-value,spatial_pooling=weighted-mean,"
        "error_map_output=on,domain=ldr}"
    )


# ---- HDR-FLIP（RXS-0389 L2 双域口径；曝光 start/stop/N + auto 语义） --------
# 色调映射系数闭集（参考实现 ToneMappingCoefficients 逐字：Reinhard / ACES / Hable）。
TONE_COEFFICIENTS = {
    "reinhard": (0.0, 1.0, 0.0, 0.0, 1.0, 1.0),
    "aces": (0.6 * 0.6 * 2.51, 0.6 * 0.03, 0.0, 0.6 * 0.6 * 2.43, 0.6 * 0.59, 0.14),
    "hable": (0.231683, 0.013791, 0.0, 0.18, 0.3, 0.018),
}


def _linear_rgb_to_luminance(rgb: np.ndarray) -> np.ndarray:
    return 0.2126 * rgb[..., 0] + 0.7152 * rgb[..., 1] + 0.0722 * rgb[..., 2]


def tone_map(img: np.ndarray, tonemapper: str = "aces") -> np.ndarray:
    """色调映射（参考实现 image::toneMap 逐字；Reinhard 走亮度归一形态）。"""
    if tonemapper == "reinhard":
        lum = _linear_rgb_to_luminance(img)
        return img / (1.0 + lum[..., None])
    if tonemapper not in TONE_COEFFICIENTS:
        raise CaliberError(f"tonemapper 闭集外: {tonemapper!r}")
    tc = TONE_COEFFICIENTS[tonemapper]
    return (img * img * tc[0] + img * tc[1] + tc[2]) / (img * img * tc[3] + img * tc[4] + tc[5])


def compute_exposures(reference: np.ndarray, tonemapper: str = "aces") -> tuple[float, float]:
    """auto-from-reference 曝光推导（参考实现 computeExposures 逐字）。

    由**参考图**亮度统计推导 start/stop 曝光（t=0.85 目标亮度二次方程 +
    max/中位亮度；median=0 安全〔max(Ymedian, float-eps)〕，参考实现 v1.7 起）。
    """
    tc = TONE_COEFFICIENTS[tonemapper]
    t = 0.85
    a = tc[0] - t * tc[3]
    b = tc[1] - t * tc[4]
    c = tc[2] - t * tc[5]
    if a == 0.0:
        x_max = -c / b
    else:
        d1 = -0.5 * (b / a)
        d2 = math.sqrt(d1 * d1 - c / a)
        x_max = d1 + d2
    lum = _linear_rgb_to_luminance(np.asarray(reference, dtype=np.float64)).ravel()
    y_max = float(lum.max())
    y_median = float(np.sort(lum)[lum.size // 2])
    y_median = max(y_median, float(np.finfo(np.float32).eps))
    start = math.log2(x_max / y_max)
    stop = math.log2(x_max / y_median)
    return start, stop


def flip_hdr(
    reference: np.ndarray,
    test: np.ndarray,
    ppd: float | None = None,
    *,
    domain: str = "scene-linear-hdr",
    hdr_exposure_mode: str = "auto-from-reference",
    hdr_exposure_start: float | None = None,
    hdr_exposure_stop: float | None = None,
    hdr_num_exposures: int | None = None,
    tonemapper: str = "aces",
) -> tuple[np.ndarray, float, dict]:
    """HDR-FLIP：返回 (逐像素误差图 max-pool [0,1], 全图均值标量, 使用参数字典)。

    reference/test = HDR 臂 scene-linear 线性帧（非负，HxWx3 float）。
    域互证：仅 `"scene-linear-hdr"` 合法；LDR 域直算 fail-closed。
    曝光面（RXS-0389 L2）：`auto-from-reference`（由参考图推导 start/stop，
    N = max(2, ceil(stop−start))）或 `fixed`（start/stop/N 三参必填，
    start ≤ stop；单值 hdr_exposure_value 形态不存在于本面）。
    """
    if domain != "scene-linear-hdr":
        raise CaliberError(f"HDR-FLIP 域限定：domain={domain!r} 拒绝（仅 scene-linear-hdr）")
    if ppd is None:
        ppd = default_ppd()
    ref = np.maximum(np.asarray(reference, dtype=np.float64), 0.0)
    tst = np.maximum(np.asarray(test, dtype=np.float64), 0.0)
    if ref.shape != tst.shape or ref.ndim != 3 or ref.shape[2] != 3:
        raise CaliberError(f"帧形状非法/不一致: {ref.shape} vs {tst.shape}")

    if hdr_exposure_mode == "auto-from-reference":
        auto_start, auto_stop = compute_exposures(ref, tonemapper)
        start = auto_start if hdr_exposure_start is None else float(hdr_exposure_start)
        stop = auto_stop if hdr_exposure_stop is None else float(hdr_exposure_stop)
    elif hdr_exposure_mode == "fixed":
        if hdr_exposure_start is None or hdr_exposure_stop is None or hdr_num_exposures is None:
            raise CaliberError("fixed 曝光模式三参必填（start/stop/num_exposures）")
        start, stop = float(hdr_exposure_start), float(hdr_exposure_stop)
    else:
        raise CaliberError(f"hdr_exposure_mode 闭集外: {hdr_exposure_mode!r}")
    if start > stop:
        raise CaliberError(f"start exposure {start} > stop {stop}")
    if hdr_num_exposures is None:
        num = int(max(2.0, math.ceil(stop - start)))
    else:
        num = int(hdr_num_exposures)
    if num < 2:
        raise CaliberError(f"num_exposures 必须 ≥2: {num}")

    step = (stop - start) / (num - 1)
    error_map = np.zeros(ref.shape[:2], dtype=np.float64)
    for i in range(num):
        exposure = start + i * step
        mult = 2.0 ** exposure
        r_ldr = np.clip(tone_map(ref * mult, tonemapper), 0.0, 1.0)
        t_ldr = np.clip(tone_map(tst * mult, tonemapper), 0.0, 1.0)
        # 曝光后 LDR-FLIP（输入已线性，跳过 sRGB 段 = 直接复用主管道线性入口）。
        emap, _ = _flip_ldr_linear(r_ldr, t_ldr, ppd)
        error_map = np.maximum(error_map, emap)
    mean = float(error_map.sum() / error_map.size)
    used = {
        "hdr_exposure_mode": hdr_exposure_mode,
        "hdr_exposure_start": start,
        "hdr_exposure_stop": stop,
        "hdr_num_exposures": num,
        "tonemapper": tonemapper,
        "ppd": ppd,
    }
    return error_map, mean, used


def _flip_ldr_linear(ref_lin: np.ndarray, tst_lin: np.ndarray, ppd: float) -> tuple[np.ndarray, float]:
    """线性 RGB [0,1] 输入的 LDR-FLIP 内核（HDR-FLIP 曝光循环复用；跳过 sRGB 段）。"""
    ref_ycc = xyz_to_ycxcz(linear_rgb_to_xyz(ref_lin))
    tst_ycc = xyz_to_ycxcz(linear_rgb_to_xyz(tst_lin))

    radius = spatial_filter_radius(ppd)
    f_ycx, f_cz = set_spatial_filters(ppd, radius)

    def _spatial(ycc: np.ndarray) -> np.ndarray:
        y_x = _separable_conv(ycc[..., 0], f_ycx[:, 0], axis=1)
        cx_x = _separable_conv(ycc[..., 1], f_ycx[:, 1], axis=1)
        cz1_x = _separable_conv(ycc[..., 2], f_cz[:, 0], axis=1)
        cz2_x = _separable_conv(ycc[..., 2], f_cz[:, 1], axis=1)
        y = _separable_conv(y_x, f_ycx[:, 0], axis=0)
        cx = _separable_conv(cx_x, f_ycx[:, 1], axis=0)
        cz = _separable_conv(cz1_x, f_cz[:, 0], axis=0) + _separable_conv(cz2_x, f_cz[:, 1], axis=0)
        return np.stack([y, cx, cz], axis=-1)

    ref_f = _spatial(ref_ycc)
    tst_f = _spatial(tst_ycc)
    ref_rgb = np.clip(xyz_to_linear_rgb(ycxcz_to_xyz(ref_f)), 0.0, 1.0)
    tst_rgb = np.clip(xyz_to_linear_rgb(ycxcz_to_xyz(tst_f)), 0.0, 1.0)
    ref_lab = xyz_to_cielab(linear_rgb_to_xyz(ref_rgb))
    tst_lab = xyz_to_cielab(linear_rgb_to_xyz(tst_rgb))
    ref_lab_h = np.stack(
        [ref_lab[..., 0], hunt(ref_lab[..., 0], ref_lab[..., 1]), hunt(ref_lab[..., 0], ref_lab[..., 2])],
        axis=-1,
    )
    tst_lab_h = np.stack(
        [tst_lab[..., 0], hunt(tst_lab[..., 0], tst_lab[..., 1]), hunt(tst_lab[..., 0], tst_lab[..., 2])],
        axis=-1,
    )
    color_diff = _hyab(ref_lab_h, tst_lab_h) ** GQC
    cmax = compute_max_distance()
    pccmax = GPC * cmax
    color_diff = np.where(
        color_diff < pccmax,
        color_diff * (GPT / pccmax),
        GPT + ((color_diff - pccmax) / (cmax - pccmax)) * (1.0 - GPT),
    )

    feature_filter, _ = set_feature_filter(ppd)
    g, dg, ddg = feature_filter[:, 0], feature_filter[:, 1], feature_filter[:, 2]

    def _features(ycc: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        y_norm = ycc[..., 0] / 116.0 + 16.0 / 116.0
        dx_x = _separable_conv(y_norm, dg, axis=1)
        ddx_x = _separable_conv(y_norm, ddg, axis=1)
        gf_x = _separable_conv(y_norm, g, axis=1)
        dx = _separable_conv(dx_x, g, axis=0)
        ddx = _separable_conv(ddx_x, g, axis=0)
        dy = _separable_conv(gf_x, dg, axis=0)
        ddy = _separable_conv(gf_x, ddg, axis=0)
        return np.sqrt(dx * dx + dy * dy), np.sqrt(ddx * ddx + ddy * ddy)

    ref_edge, ref_point = _features(ref_ycc)
    tst_edge, tst_point = _features(tst_ycc)
    feature_diff = (
        np.maximum(np.abs(ref_edge - tst_edge), np.abs(ref_point - tst_point)) / math.sqrt(2.0)
    ) ** GQF

    error_map = color_diff ** (1.0 - feature_diff)
    return error_map, float(error_map.sum() / error_map.size)
