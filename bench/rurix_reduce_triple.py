"""Rurix reduce 三次进程级独立运行 + m5_budget.json 回填(契约 G-M5-1,M5.4)。

**操作者工具**(需锁频 L0 在位,BENCH_PROTOCOL.md §2/§3):每次重过 L0、重装载,
三次 trimmed mean 再 trimmed mean 聚合;**任一次非 measured_local(锁频降级
unlocked)→ 整组作废,拒绝回填**(契约 G-M5-1,unlocked 不得回填)。

回填 [milestones/m5/m5_budget.json](../milestones/m5/m5_budget.json):
  - entries[] 追加/更新 numerator `m5.bench.reduce.effective_bandwidth_gbps`
    (measured_local + evidence_file + threshold = 实测 × 0.95);
  - ratio_assertions `m5.ratio.reduce_vs_cuda` 翻 estimated→measured_local
    + measured_value = numerator / denominator(CUDA C++ 对照锚点);
  - revision_log 追加。

denominator 须已 measured_local(M5.4 先跑 CUDA 对照三次运行回填
`m5.bench.reduce_cuda.effective_bandwidth_gbps`)。

回填后核验:`py -3 ci/budget_eval.py --strict`(比值 ≥0.90 通过,契约 G-M5-1)。

用法(锁频后):py -3 bench/rurix_reduce_triple.py
"""
from __future__ import annotations

import datetime
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import lock_clocks
from bench.stats import bootstrap_ci, cv, trimmed_mean

ROOT = Path(__file__).resolve().parent.parent
RUNS = 3
THRESHOLD_TOLERANCE = 0.95
PREFIX = "rurix_reduce"
NUMERATOR_ID = "m5.bench.reduce.effective_bandwidth_gbps"
DENOMINATOR_ID = "m5.bench.reduce_cuda.effective_bandwidth_gbps"
RATIO_ID = "m5.ratio.reduce_vs_cuda"
BENCH_SCRIPT = "bench/reduce_bench.py"


def run_subprocess(cmd: list[str]) -> None:
    print(f"  $ {' '.join(cmd)}")
    if subprocess.run(cmd, cwd=ROOT).returncode != 0:
        raise RuntimeError(f"子进程失败: {' '.join(cmd)}")


def collect_runs() -> str:
    date = datetime.date.today().strftime("%Y%m%d")
    py = sys.executable
    for seq in range(1, RUNS + 1):
        print(f"[rurix_reduce_triple] === 第 {seq}/{RUNS} 次进程级独立运行 ===")
        run_subprocess([py, BENCH_SCRIPT, "--emit",
                        f"evidence/{PREFIX}_{date}_{seq}.json"])
    return date


def aggregate(date: str) -> tuple[Path, dict]:
    docs = []
    for seq in range(1, RUNS + 1):
        path = ROOT / f"evidence/{PREFIX}_{date}_{seq}.json"
        docs.append(json.loads(path.read_text(encoding="utf-8")))
    levels = {d["evidence_level"] for d in docs}
    if levels != {"measured_local"}:
        raise RuntimeError(
            f"{PREFIX}: 存在非 measured_local 运行({levels}),整组作废(契约 G-M5-1,"
            "unlocked 证据不得回填)"
        )
    run_means = [d["results"]["trimmed_mean"] for d in docs]
    all_trial_medians = [m for d in docs for m in d["results"]["trial_medians"]]
    agg_value = trimmed_mean(run_means, 0.2)
    ci_lo, ci_hi = bootstrap_ci(all_trial_medians, statistic="median")

    agg = json.loads(json.dumps(docs[0]))
    agg["timestamp"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    agg["results"]["trial_medians"] = [round(v, 4) for v in run_means]
    agg["results"]["trimmed_mean"] = round(agg_value, 4)
    agg["results"]["cv"] = round(cv(run_means), 6)
    agg["results"]["ci95"] = [round(ci_lo, 4), round(ci_hi, 4)]
    agg["results"]["min"] = round(min(all_trial_medians), 4)
    agg["results"]["max"] = round(max(all_trial_medians), 4)
    agg["notes"] = (
        f"aggregate of {RUNS} process-level independent runs "
        f"({PREFIX}_{date}_1..{RUNS}.json); trial_medians = per-run trimmed means. "
        + agg.get("notes", "")
    )
    out = ROOT / f"evidence/{PREFIX}_{date}_agg.json"
    out.write_text(json.dumps(agg, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[rurix_reduce_triple] aggregate: {out.name} = {agg_value:.4f} {agg['results']['unit']}")
    return out, agg


def denominator_value() -> float:
    budget_path = ROOT / "milestones/m5/m5_budget.json"
    budget = json.loads(budget_path.read_text(encoding="utf-8"))
    for e in budget["entries"]:
        if e["id"] == DENOMINATOR_ID:
            if e.get("evidence") != "measured_local":
                raise RuntimeError(
                    f"{DENOMINATOR_ID} 尚未 measured_local;"
                    "M5.4 先跑 CUDA 对照三次运行回填 denominator"
                )
            return float(e["measured_value"])
    raise RuntimeError(f"{DENOMINATOR_ID} 缺失(denominator)")


def backfill_budget(agg_path: Path, agg: dict) -> None:
    value = agg["results"]["trimmed_mean"]
    rel = agg_path.relative_to(ROOT).as_posix()
    budget_path = ROOT / "milestones/m5/m5_budget.json"
    budget = json.loads(budget_path.read_text(encoding="utf-8"))

    entry = {
        "id": NUMERATOR_ID,
        "direction": "min",
        "threshold": round(value * THRESHOLD_TOLERANCE, 2),
        "evidence": "measured_local",
        "unit": "GB/s",
        "evidence_file": rel,
        "measured_value": value,
    }
    budget["entries"] = [e for e in budget.get("entries", []) if e["id"] != NUMERATOR_ID]
    budget["entries"].append(entry)

    den = denominator_value()
    ratio = value / den
    for r in budget["ratio_assertions"]:
        if r["id"] == RATIO_ID:
            r["evidence"] = "measured_local"
            r["measured_value"] = round(ratio, 4)
            r["skip_reason"] = None

    budget["revision_log"].append({
        "version": f"v1.{len(budget['revision_log'])}",
        "date": datetime.date.today().isoformat(),
        "change": "M5.4 回填:Rurix reduce measured_local(三次进程级独立运行 trimmed mean "
                  f"= {value:.4f} GB/s)→ numerator entry + ratio vs CUDA C++ 对照 "
                  f"({den:.4f} GB/s)= {ratio:.4f}(契约 G-M5-1,阈值 0.90)",
    })
    budget_path.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n",
                           encoding="utf-8")
    print(f"[rurix_reduce_triple] budget backfilled: ratio = {ratio:.4f} "
          f"(numerator {value:.4f} / denominator {den:.4f}; 阈值 0.90 "
          f"{'PASS' if ratio >= 0.90 else 'FAIL'})")
    print("[rurix_reduce_triple] 下一步核验:py -3 ci/budget_eval.py --strict")


def main() -> int:
    lock_clocks.require_locked()  # 采样前置闸门:未锁频拒绝采样(契约 G-M5-1)
    date = collect_runs()
    agg_path, agg = aggregate(date)
    backfill_budget(agg_path, agg)
    return 0


if __name__ == "__main__":
    sys.exit(main())
