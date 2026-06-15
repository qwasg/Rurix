"""L1 采样协议执行器(BENCH_PROTOCOL.md §3,数字来源 r11 §1.3)。

职责:warmup/稳态 → 50×3 timed → 统计聚合 → 组装证据 JSON。
被 saxpy_bench / bandwidth_bench 复用;计时回调返回"该次迭代耗时 ms"。

M6.1 收编(RD-003,RXS-0088):本模块及 bench/*.py 协议脚本降级为"被 rx bench
统一入口编排的协议库"——`rx bench [<name>] [--smoke]`(src/rx/src/main.rs)以本
协议为单一事实源,口径(L0 锁频前置 / 三次进程级独立运行 / trimmed mean)与
evidence/*.json 证据格式完全不变(evidence/ 只增不删不改)。formal close 待 M6
close-out 终审(契约 §8)。
"""
from __future__ import annotations

import datetime
import json
import subprocess
import time
from pathlib import Path
from typing import Callable

from bench import env_probe
from bench.stats import bootstrap_ci, cv, iqr_filter, steady_state_reached, trimmed_mean

ROOT = Path(__file__).resolve().parent.parent

WARMUP_MIN = 10            # r11 §1.1
WARMUP_MAX = 50            # 稳态判定不收敛时的上限保护
WARMUP_TIMEOUT_S = 300     # r11 §1.1
STEADY_WINDOW = 5          # r11 §1.1
STEADY_CV = 0.05           # r11 §1.1
TIMED_ITERATIONS = 50      # r11 §1.3.1
TRIALS = 3                 # r11 §1.3.1
TRIM = 0.2                 # r11 §1.3.1
L2_CLEAR_MB = 256          # r11 §1.1


def git_commit() -> str:
    out = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT,
                         capture_output=True, text=True, check=False)
    return out.stdout.strip() or "unknown"


def run_protocol(
    bench_id: str,
    problem_size: str,
    metric: str,
    unit: str,
    iter_ms: Callable[[], float],
    ms_to_metric: Callable[[float], float],
    pre_timed: Callable[[], None] | None = None,
    notes: str = "",
) -> dict:
    """执行完整协议并返回证据 JSON dict。

    iter_ms:执行一次测量区并返回耗时 ms(内部须自带 CUDA Event 与同步);
    ms_to_metric:耗时 → 指标(如 GB/s);
    pre_timed:每次 timed 迭代前的准备(L2 清理由调用方注入)。
    """
    env_start = env_probe.collect_environment()
    temp_start = env_start["thermal"]["temp_start_c"]

    # --- warmup + 稳态判定 ---
    warmup_times: list[float] = []
    t0 = time.monotonic()
    while len(warmup_times) < WARMUP_MAX:
        if time.monotonic() - t0 > WARMUP_TIMEOUT_S:
            raise TimeoutError("warmup 超时(协议保护,r11 §1.1)")
        warmup_times.append(iter_ms())
        if len(warmup_times) >= WARMUP_MIN and steady_state_reached(
            warmup_times, STEADY_WINDOW, STEADY_CV
        ):
            break
    steady = steady_state_reached(warmup_times, STEADY_WINDOW, STEADY_CV)
    steady_cv_value = cv(warmup_times[-STEADY_WINDOW:])

    # --- 50 × 3 timed ---
    trial_metric_medians: list[float] = []
    all_metrics: list[float] = []
    for _trial in range(TRIALS):
        samples_ms: list[float] = []
        for _i in range(TIMED_ITERATIONS):
            if pre_timed:
                pre_timed()
            samples_ms.append(iter_ms())
        metrics = sorted(ms_to_metric(ms) for ms in samples_ms)
        all_metrics.extend(metrics)
        trial_metric_medians.append(metrics[len(metrics) // 2])

    kept, rejected = iqr_filter(all_metrics)
    result_mean = trimmed_mean(trial_metric_medians, TRIM)
    ci_lo, ci_hi = bootstrap_ci(kept, statistic="median")  # 与中位数中心趋势同统计量

    env_end = env_probe.collect_environment()
    environment = env_start
    environment["thermal"] = {
        "temp_start_c": temp_start,
        "temp_end_c": env_end["thermal"]["temp_end_c"],
        "steady_state": steady,
    }
    # locked 判定取采样起止双探测(单瞬时 NVML 读数存在 P-state 瞬态误判)
    locked = env_start["clocks"]["locked"] and env_end["clocks"]["locked"]
    environment["clocks"]["locked"] = locked

    evidence_level = "measured_local" if locked else "unlocked"

    return {
        "schema_version": 1,
        "evidence_level": evidence_level,
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "bench": {
            "id": bench_id,
            "level": "L1",
            "problem_size": problem_size,
            "harness_commit": git_commit(),
        },
        "environment": environment,
        "sampling": {
            "warmup_iterations": len(warmup_times),
            "steady_state_cv": round(steady_cv_value, 6),
            "timed_iterations": TIMED_ITERATIONS,
            "trials": TRIALS,
            "trimmed_pct": TRIM,
            "l2_clear_mb": L2_CLEAR_MB,
            "timer": "cuda_event",
        },
        "results": {
            "metric": metric,
            "unit": unit,
            "trial_medians": [round(v, 4) for v in trial_metric_medians],
            "trimmed_mean": round(result_mean, 4),
            "cv": round(cv(kept), 6),
            "ci95": [round(ci_lo, 4), round(ci_hi, 4)],
            "min": round(min(all_metrics), 4),
            "max": round(max(all_metrics), 4),
            "outliers_rejected_iqr": len(rejected),
            "correctness_check": "n/a",
        },
        "notes": notes,
    }


def write_evidence(doc: dict, out_path: Path) -> None:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[protocol] evidence written: {out_path.relative_to(ROOT)} "
          f"(level={doc['evidence_level']}, {doc['results']['trimmed_mean']} {doc['results']['unit']})")
