"""parser 吞吐 harness 单测(M1_PLAN §3 任务 5:合成数据复算)。"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench.parser_bench import build_corpus, count_loc, kloc_per_s
from bench.stats import trimmed_mean


def test_count_loc_physical_lines():
    assert count_loc("") == 0
    assert count_loc("fn f() {}") == 1
    assert count_loc("a\nb\nc") == 3
    assert count_loc("a\nb\n") == 2  # 与 Rust src.lines().count() 同口径


def test_kloc_per_s_synthetic_recompute():
    # 1000 行 / 1ms = 1_000_000 行/s = 1000 kloc/s
    assert abs(kloc_per_s(1_000_000, 1000) - 1000.0) < 1e-9
    # 500 行 / 2s = 250 行/s = 0.25 kloc/s
    assert abs(kloc_per_s(2_000_000_000, 500) - 0.25) < 1e-9


def test_trial_median_pipeline_synthetic():
    # 复算 harness 的 trial 内中位数 → 跨 trial trimmed mean 链路
    loc = 2000
    trials_ns = [
        [1_000_000, 2_000_000, 4_000_000],   # 中位 2ms → 1000 kloc/s
        [2_000_000, 2_000_000, 2_000_000],   # 中位 2ms → 1000 kloc/s
        [1_000_000, 4_000_000, 8_000_000],   # 中位 4ms → 500 kloc/s
    ]
    medians = []
    for chunk in trials_ns:
        metrics = sorted(kloc_per_s(ns, loc) for ns in chunk)
        medians.append(metrics[len(metrics) // 2])
    assert medians == [1000.0, 1000.0, 500.0]
    # trimmed_mean(20%) 对 3 个样本不剔除(int(3*0.2)=0),等于算术平均
    assert abs(trimmed_mean(medians, 0.2) - (2500.0 / 3)) < 1e-9


def test_build_corpus_excludes_inner_attr_samples():
    corpus, desc = build_corpus(1)  # 最小放大倍数
    assert "#![" not in corpus  # 拼接后内部属性不再处于文件顶部(RXS-0011)
    assert "fn " in corpus
    assert "kloc corpus" in desc
