"""手写 CUDA C++ 对照(分母)三次进程级独立运行 + m5_budget.json 回填(契约 G-M5-1,M5.4)。

**操作者工具**(需锁频 L0 在位,BENCH_PROTOCOL.md §2/§3):对 reduce/scan/gemm_tile 三个
手写 CUDA C++ 对照基准,每个重过 L0、重装载跑三次,三次 trimmed mean 再 trimmed mean 聚合;
**任一次非 measured_local(锁频降级 unlocked)→ 整组作废,拒绝回填**(契约 G-M5-1)。

分母先行:rurix_*_triple.py 的 denominator_value() 要求 m5.bench.*_cuda.* 已 measured_local,
本脚本回填这些分母 entry(estimated → measured_local + evidence_file + threshold = 实测 × 0.95)。
回填后再跑 rurix_*_triple.py 回填分子 + ratio,最后 py -3 ci/budget_eval.py --strict 核验。

用法(锁频后):
  py -3 bench/cuda_ref_triple.py                       # 三分母全跑
  py -3 bench/cuda_ref_triple.py --only reduce|scan|gemm_tile
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
THRESHOLD_TOLERANCE = 0.95  # 阈值 = 实测 × 0.95,对应 5% Critical 门(r11 §7 / 08 §4)

# 分母配置表(沿用 triple_run.py 的表驱动风格):kind → (证据前缀, bench 脚本, 预算 entry id, 单位)
DENOMINATORS: dict[str, tuple[str, str, str, str]] = {
    "reduce": (
        "cuda_reduce", "bench/reduce_cuda_bench.py",
        "m5.bench.reduce_cuda.effective_bandwidth_gbps", "GB/s",
    ),
    "scan": (
        "cuda_scan", "bench/scan_cuda_bench.py",
        "m5.bench.scan_cuda.effective_bandwidth_gbps", "GB/s",
    ),
    "gemm_tile": (
        "cuda_gemm_tile", "bench/gemm_tile_cuda_bench.py",
        "m5.bench.gemm_tile_cuda.throughput_gflops", "GFLOPS",
    ),
}


def run_subprocess(cmd: list[str]) -> None:
    print(f"  $ {' '.join(cmd)}")
    if subprocess.run(cmd, cwd=ROOT).returncode != 0:
        raise RuntimeError(f"子进程失败: {' '.join(cmd)}")


def collect_runs(kinds: list[str]) -> str:
    date = datetime.date.today().strftime("%Y%m%d")
    py = sys.executable
    for seq in range(1, RUNS + 1):
        print(f"[cuda_ref_triple] === 第 {seq}/{RUNS} 次进程级独立运行 ===")
        for kind in kinds:
            prefix, script, _id, _unit = DENOMINATORS[kind]
            run_subprocess([py, script, "--emit",
                            f"evidence/{prefix}_{date}_{seq}.json"])
    return date


def aggregate(prefix: str, date: str) -> tuple[Path, dict]:
    """三份 run 证据 → 聚合证据文件(trial_medians = 三次运行各自的 trimmed mean)。"""
    docs = []
    for seq in range(1, RUNS + 1):
        path = ROOT / f"evidence/{prefix}_{date}_{seq}.json"
        docs.append(json.loads(path.read_text(encoding="utf-8")))
    levels = {d["evidence_level"] for d in docs}
    if levels != {"measured_local"}:
        raise RuntimeError(
            f"{prefix}: 存在非 measured_local 运行({levels}),整组作废(契约 G-M5-1,"
            "unlocked 证据不得回填)"
        )
    run_means = [d["results"]["trimmed_mean"] for d in docs]
    all_trial_medians = [m for d in docs for m in d["results"]["trial_medians"]]
    agg_value = trimmed_mean(run_means, 0.2)
    ci_lo, ci_hi = bootstrap_ci(all_trial_medians, statistic="median")

    agg = json.loads(json.dumps(docs[0]))  # 以第 1 次运行为模板
    agg["timestamp"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    agg["results"]["trial_medians"] = [round(v, 4) for v in run_means]
    agg["results"]["trimmed_mean"] = round(agg_value, 4)
    agg["results"]["cv"] = round(cv(run_means), 6)
    agg["results"]["ci95"] = [round(ci_lo, 4), round(ci_hi, 4)]
    agg["results"]["min"] = round(min(all_trial_medians), 4)
    agg["results"]["max"] = round(max(all_trial_medians), 4)
    agg["notes"] = (
        f"aggregate of {RUNS} process-level independent runs "
        f"({prefix}_{date}_1..{RUNS}.json); trial_medians = per-run trimmed means. "
        + agg.get("notes", "")
    )
    out = ROOT / f"evidence/{prefix}_{date}_agg.json"
    out.write_text(json.dumps(agg, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[cuda_ref_triple] aggregate: {out.name} = {agg_value:.4f} {agg['results']['unit']}")
    return out, agg


def backfill_budget(results: dict[str, tuple[Path, dict, str, str]]) -> None:
    """results: kind → (agg_path, agg, denominator_id, unit)。翻 estimated → measured_local。"""
    budget_path = ROOT / "milestones/m5/m5_budget.json"
    budget = json.loads(budget_path.read_text(encoding="utf-8"))
    by_id = {e["id"]: e for e in budget["entries"]}

    summary = []
    for kind, (agg_path, agg, den_id, unit) in results.items():
        value = agg["results"]["trimmed_mean"]
        rel = agg_path.relative_to(ROOT).as_posix()
        entry = by_id.get(den_id)
        if entry is None:
            raise RuntimeError(f"{den_id} 缺失(分母占位);m5_budget.json entries 应有 estimated 占位")
        entry["threshold"] = round(value * THRESHOLD_TOLERANCE, 2)
        entry["evidence"] = "measured_local"
        entry["unit"] = unit
        entry["evidence_file"] = rel
        entry["measured_value"] = value
        entry["skip_reason"] = None
        summary.append(f"{den_id}={value:.4f} {unit}")

    budget["revision_log"].append({
        "version": f"v1.{len(budget['revision_log'])}",
        "date": datetime.date.today().isoformat(),
        "change": "M5.4 回填:手写 CUDA C++ 对照分母 measured_local(三次进程级独立运行 trimmed "
                  f"mean;{'; '.join(summary)})→ 分母 entry estimated→measured_local,"
                  f"阈值 = 实测 × {THRESHOLD_TOLERANCE}(契约 G-M5-1)",
    })
    budget_path.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n",
                           encoding="utf-8")
    print(f"[cuda_ref_triple] budget backfilled (分母): {'; '.join(summary)}")
    print("[cuda_ref_triple] 下一步:py -3 bench/rurix_{reduce,scan,gemm_tile}_triple.py "
          "回填分子 + ratio,再 py -3 ci/budget_eval.py --strict")


def main() -> int:
    lock_clocks.require_locked()  # 采样前置闸门:未锁频拒绝采样(契约 G-M5-1)
    only = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None
    if only is not None and only not in DENOMINATORS:
        print(f"[cuda_ref_triple] 未知 --only {only!r};可选 {list(DENOMINATORS)}", file=sys.stderr)
        return 2
    kinds = [only] if only else list(DENOMINATORS)

    date = collect_runs(kinds)
    results: dict[str, tuple[Path, dict, str, str]] = {}
    for kind in kinds:
        prefix, _script, den_id, unit = DENOMINATORS[kind]
        agg_path, agg = aggregate(prefix, date)
        results[kind] = (agg_path, agg, den_id, unit)
    backfill_budget(results)
    return 0


if __name__ == "__main__":
    sys.exit(main())
