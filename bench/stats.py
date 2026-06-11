"""采样协议统计函数(BENCH_PROTOCOL.md §3,数字来源 r11 §1.3)。

纯 Python 实现,便于无 numpy 环境(CI 降级分支)下复算。
"""
from __future__ import annotations

import random
import statistics
from typing import Sequence


def trimmed_mean(values: Sequence[float], trim: float = 0.2) -> float:
    """去头尾各 trim 比例后取均值(协议值 0.2,r11 §1.3.1)。"""
    if not values:
        raise ValueError("empty sample")
    if not 0 <= trim < 0.5:
        raise ValueError("trim must be in [0, 0.5)")
    data = sorted(values)
    k = int(len(data) * trim)
    kept = data[k: len(data) - k] if k else data
    return sum(kept) / len(kept)


def cv(values: Sequence[float]) -> float:
    """变异系数 std/mean(稳态判定阈值 < 0.05,r11 §1.1)。"""
    m = statistics.fmean(values)
    if m == 0:
        raise ValueError("mean is zero")
    return statistics.stdev(values) / m


def iqr_filter(values: Sequence[float], k: float = 1.5) -> tuple[list[float], list[float]]:
    """IQR 异常剔除(r11 §1.3.2)。返回 (保留, 剔除)。"""
    data = sorted(values)
    n = len(data)
    if n < 4:
        return list(values), []
    q1 = statistics.quantiles(data, n=4)[0]
    q3 = statistics.quantiles(data, n=4)[2]
    iqr = q3 - q1
    lo, hi = q1 - k * iqr, q3 + k * iqr
    kept = [v for v in values if lo <= v <= hi]
    rejected = [v for v in values if not (lo <= v <= hi)]
    return kept, rejected


def bootstrap_ci(
    values: Sequence[float],
    confidence: float = 0.95,
    n_resamples: int = 10_000,
    seed: int = 0,
    statistic: str = "mean",
) -> tuple[float, float]:
    """bootstrap 置信区间(r11 §1.3;seed 固定保证可复现)。

    statistic="median" 时与协议的中位数中心趋势保持同一统计量。
    """
    if not values:
        raise ValueError("empty sample")
    rng = random.Random(seed)
    n = len(values)
    stat = statistics.median if statistic == "median" else statistics.fmean
    estimates = sorted(
        stat([rng.choice(values) for _ in range(n)]) for _ in range(n_resamples)
    )
    alpha = (1 - confidence) / 2
    lo_idx = int(alpha * n_resamples)
    hi_idx = min(n_resamples - 1, int((1 - alpha) * n_resamples))
    return estimates[lo_idx], estimates[hi_idx]


def steady_state_reached(history: Sequence[float], window: int = 5, threshold: float = 0.05) -> bool:
    """连续 window 次迭代 CV < threshold 即判定稳态(r11 §1.1)。"""
    if len(history) < window:
        return False
    return cv(list(history)[-window:]) < threshold
