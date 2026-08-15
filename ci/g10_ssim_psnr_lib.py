#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.4a 波）
"""G10.4a M136 SSIM/PSNR 自实现（spec/visual_comparison.md RXS-0387 L1~L3）。

Wang et al. 2004 标准参数化逐字实现（口径闭集，闭集外参数禁调）：

- 窗 = 11×11 高斯窗 σ = 1.5（`gaussian_filter(mode='reflect')` 承载，与
  scikit-image `structural_similarity(gaussian_weights=True)` 内部同滤波面）；
- 常数 K1 = 0.01、K2 = 0.03，C1 = (K1·L)²、C2 = (K2·L)²，L = data_range = 1.0；
- 协方差 = 总体协方差（不采样校正；`use_sample_covariance=False` 同口径）；
- 聚合 = 逐通道 SSIM → RGB 三通道均值（mean-SSIM，非 multi-scale MS-SSIM）；
- PSNR：MSE = RGB 三通道联合均方误差；`PSNR = 10·log10(L²/MSE)`，MSE = 0
  → +inf（evidence 序列化字符串 `"inf"`，RXS-0387 L3 字面）。

**LDR 域限定（L1 防口径混用）**：`compute_*` 族仅接受 display-referred-ldr
域 `[0,1]` 帧——任何值越界（HDR 内容）或域标签非 LDR 即 fail-closed 显式
`CaliberError`（HDR 直算即口径混用 RED）。

与 G5 既有 SSIM 门禁 helper（`src/rurix-render/src/temporal/ssim.rs`，8×8
盒式窗）**不同属一套口径**——字面 0-byte，两口径并存、各自登记、互不冒充
（RFC-0026 §4.3 0-byte 声明）。
"""
from __future__ import annotations

import math

import numpy as np
from scipy.ndimage import gaussian_filter

# Wang 2004 参数闭集（RXS-0387 L2 冻结字面；漂移注入即 RED 的判定基准）。
SSIM_WIN_SIZE = 11
SSIM_SIGMA = 1.5
SSIM_K1 = 0.01
SSIM_K2 = 0.03
DATA_RANGE = 1.0

LDR_DOMAIN = "display-referred-ldr"


class CaliberError(ValueError):
    """fail-closed：域限定/口径违例即抛出。"""


def _assert_ldr(a: np.ndarray, b: np.ndarray, domain: str) -> None:
    if domain != LDR_DOMAIN:
        raise CaliberError(f"SSIM/PSNR 仅 LDR 臂定义（domain={domain!r}；HDR 直算即口径混用）")
    if a.shape != b.shape or a.ndim != 3 or a.shape[2] != 3:
        raise CaliberError(f"帧形态非法（须同形 H×W×3）: {a.shape} vs {b.shape}")
    for name, img in (("a", a), ("b", b)):
        if not np.all(np.isfinite(img)):
            raise CaliberError(f"帧 {name} 含 NaN/Inf")
        if float(img.min()) < 0.0 or float(img.max()) > DATA_RANGE:
            raise CaliberError(
                f"帧 {name} 值域越出 LDR [0,{DATA_RANGE}]（HDR 直算即口径混用 RED）"
            )


# 参考实现对齐面（skimage 0.26 `structural_similarity` 内部同字面）：
# gaussian_filter truncate=3.5（skimage gaussian_weights 路径硬编码）+
# 均值前按滤波半径裁剪边缘 pad=(win_size-1)//2=5（"to avoid edge effects"）。
_GAUSS_TRUNCATE = 3.5
_EDGE_PAD = (SSIM_WIN_SIZE - 1) // 2


def ssim_wang2004(a: np.ndarray, b: np.ndarray, domain: str = LDR_DOMAIN) -> float:
    """mean-SSIM（Wang 2004；逐通道 11×11 高斯 σ=1.5 总体协方差 → 边缘裁剪
    均值 → RGB 均值）。返回值域 [-1, 1]；恒等图对恰为 1.0。"""
    _assert_ldr(a, b, domain)
    c1 = (SSIM_K1 * DATA_RANGE) ** 2
    c2 = (SSIM_K2 * DATA_RANGE) ** 2
    per_channel: list[float] = []
    for ch in range(3):
        x = a[..., ch].astype(np.float64)
        y = b[..., ch].astype(np.float64)
        ux = gaussian_filter(x, SSIM_SIGMA, mode="reflect", truncate=_GAUSS_TRUNCATE)
        uy = gaussian_filter(y, SSIM_SIGMA, mode="reflect", truncate=_GAUSS_TRUNCATE)
        uxx = gaussian_filter(x * x, SSIM_SIGMA, mode="reflect", truncate=_GAUSS_TRUNCATE)
        uyy = gaussian_filter(y * y, SSIM_SIGMA, mode="reflect", truncate=_GAUSS_TRUNCATE)
        uxy = gaussian_filter(x * y, SSIM_SIGMA, mode="reflect", truncate=_GAUSS_TRUNCATE)
        vx = uxx - ux * ux
        vy = uyy - uy * uy
        cxy = uxy - ux * uy
        ssim_map = ((2.0 * ux * uy + c1) * (2.0 * cxy + c2)) / ((ux**2 + uy**2 + c1) * (vx + vy + c2))
        cropped = ssim_map[_EDGE_PAD:-_EDGE_PAD, _EDGE_PAD:-_EDGE_PAD]
        per_channel.append(float(np.mean(cropped, dtype=np.float64)))
    return sum(per_channel) / 3.0


def psnr_joint(a: np.ndarray, b: np.ndarray, domain: str = LDR_DOMAIN) -> float:
    """PSNR（RGB 联合 MSE；MSE=0 → +inf）。"""
    _assert_ldr(a, b, domain)
    diff = a.astype(np.float64) - b.astype(np.float64)
    mse = float(np.mean(diff * diff))
    if mse == 0.0:
        return float("inf")
    return 10.0 * math.log10(DATA_RANGE * DATA_RANGE / mse)


def psnr_json_value(v: float) -> float | str:
    """PSNR evidence 序列化约定（RXS-0387 L3：number 或字符串字面 "inf"）。"""
    return "inf" if math.isinf(v) else v


def parse_psnr_json_value(v: float | str) -> float:
    """解析器双形态接受（"inf" 与有限值；其余字符串拒绝）。"""
    if isinstance(v, str):
        if v == "inf":
            return float("inf")
        raise CaliberError(f"PSNR 字符串闭集外取值: {v!r}")
    return float(v)


def reference_ssim_psnr(a: np.ndarray, b: np.ndarray) -> tuple[float, float]:
    """scikit-image 参考实现（显式 Wang 参数化；版本 pin 随 evidence 登记）。"""
    from skimage.metrics import peak_signal_noise_ratio, structural_similarity

    s = structural_similarity(
        a,
        b,
        gaussian_weights=True,
        sigma=SSIM_SIGMA,
        win_size=SSIM_WIN_SIZE,
        use_sample_covariance=False,
        data_range=DATA_RANGE,
        channel_axis=-1,
    )
    p = peak_signal_noise_ratio(a, b, data_range=DATA_RANGE)
    return float(s), float(p)
