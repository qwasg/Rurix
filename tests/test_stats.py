"""bench/stats.py 合成数据复算单测(PR Smoke 步骤 4,CI_GATES.md §3.4)。"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench.stats import bootstrap_ci, cv, iqr_filter, steady_state_reached, trimmed_mean


def test_trimmed_mean_drops_tails():
    # 10 个值,trim=0.2 → 去头 2 尾 2,剩 [3..8] 均值 5.5
    data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 100]
    assert trimmed_mean(data, trim=0.2) == 5.5


def test_trimmed_mean_zero_trim_is_mean():
    assert trimmed_mean([1.0, 2.0, 3.0], trim=0.0) == 2.0


def test_cv_known_value():
    # [9, 11]:mean=10,stdev=sqrt(2)≈1.41421 → cv≈0.141421
    assert abs(cv([9.0, 11.0]) - 0.1414213562) < 1e-9


def test_iqr_rejects_outlier():
    data = [10.0] * 20 + [10.1] * 20 + [1000.0]
    kept, rejected = iqr_filter(data)
    assert rejected == [1000.0]
    assert len(kept) == 40


def test_bootstrap_ci_brackets_mean_and_reproducible():
    data = [10.0, 10.5, 9.5, 10.2, 9.8, 10.1, 9.9, 10.3, 9.7, 10.0]
    lo, hi = bootstrap_ci(data, seed=42)
    assert lo <= 10.0 <= hi
    assert (lo, hi) == bootstrap_ci(data, seed=42)  # seed 固定可复现


def test_steady_state():
    assert not steady_state_reached([10, 20, 30, 40, 50])          # CV 远超 5%
    assert steady_state_reached([10.0, 10.01, 9.99, 10.0, 10.02])  # CV < 5%
    assert not steady_state_reached([10.0, 10.0], window=5)        # 样本不足
