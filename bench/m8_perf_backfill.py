"""M8.2 UC-01/cublas L1/L2 性能 measured_local 回填(契约 G-M8-2;BENCH_PROTOCOL.md §3)。

**操作者工具**(需锁频 L0 在位,需管理员 `py -3 bench/lock_clocks.py --lock`):对 9 个
基准各跑三次进程级独立运行,trimmed-mean-of-trimmed-means 聚合到 `_agg.json`;**任一次
非 measured_local(锁频降级 unlocked)→ 整组作废,拒绝回填**(契约 G-M8-2,零 estimated)。

回填 milestones/m8/m8_budget.json:
  - 9 entries(measured_local + evidence_file + threshold = 实测 × 0.95):
      自研 saxpy/reduce/gemm + cublas gemm/gemv + 手写 CUDA C++ 对照 saxpy/reduce/gemm/gemv;
  - 5 ratio_assertions(measured_value = numerator/denominator,阈值 0.90,UC-01 判据 01 §6):
      saxpy/reduce/gemm 自研 vs CUDA C++,cublas gemm/gemv vs CUDA C++;
  - revision_log 追加。

回填后核验:py -3 ci/budget_eval.py --strict(比值 ≥0.90 通过,零 estimated)。

用法(锁频后):py -3 bench/m8_perf_backfill.py
                py -3 bench/m8_perf_backfill.py --only cublas_gemm,cublas_gemv  # 仅跑子集
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
RATIO_THRESHOLD = 0.90
BUDGET = ROOT / "milestones" / "m8" / "m8_budget.json"

# (key, entry_id, script, unit, evidence_prefix)
BENCHES = [
    ("saxpy", "m8.bench.saxpy.effective_bandwidth_gbps", "bench/rurix_saxpy_bench.py", "GB/s", "m8_rurix_saxpy"),
    ("saxpy_cuda", "m8.bench.saxpy_cuda.effective_bandwidth_gbps", "bench/saxpy_cuda_bench.py", "GB/s", "m8_cuda_saxpy"),
    ("reduce", "m8.bench.reduce.effective_bandwidth_gbps", "bench/reduce_bench.py", "GB/s", "m8_rurix_reduce"),
    ("reduce_cuda", "m8.bench.reduce_cuda.effective_bandwidth_gbps", "bench/reduce_cuda_bench.py", "GB/s", "m8_cuda_reduce"),
    ("gemm", "m8.bench.gemm.throughput_gflops", "bench/gemm_tile_bench.py", "GFLOPS", "m8_rurix_gemm"),
    ("gemm_cuda", "m8.bench.gemm_cuda.throughput_gflops", "bench/gemm_tile_cuda_bench.py", "GFLOPS", "m8_cuda_gemm"),
    ("cublas_gemm", "m8.bench.cublas_gemm.throughput_gflops", "bench/cublas_gemm_bench.py", "GFLOPS", "m8_cublas_gemm"),
    ("cublas_gemv", "m8.bench.cublas_gemv.effective_bandwidth_gbps", "bench/cublas_gemv_bench.py", "GB/s", "m8_cublas_gemv"),
    ("gemv_cuda", "m8.bench.gemv_cuda.effective_bandwidth_gbps", "bench/gemv_cuda_bench.py", "GB/s", "m8_cuda_gemv"),
]

# (ratio_id, numerator_key, denominator_key)
RATIOS = [
    ("m8.ratio.saxpy_vs_cuda", "saxpy", "saxpy_cuda"),
    ("m8.ratio.reduce_vs_cuda", "reduce", "reduce_cuda"),
    ("m8.ratio.gemm_vs_cuda", "gemm", "gemm_cuda"),
    ("m8.ratio.cublas_gemm_vs_cuda", "cublas_gemm", "gemm_cuda"),
    ("m8.ratio.cublas_gemv_vs_cuda", "cublas_gemv", "gemv_cuda"),
]


def run_subprocess(cmd: list[str]) -> None:
    print(f"  $ {' '.join(cmd)}")
    if subprocess.run(cmd, cwd=ROOT).returncode != 0:
        raise RuntimeError(f"子进程失败: {' '.join(cmd)}")


def collect_and_aggregate(key: str, script: str, prefix: str, date: str) -> tuple[str, float, str]:
    """三次进程级独立运行 → 聚合 _agg.json。返回(evidence_file 相对路径, 聚合值, unit)。"""
    py = sys.executable
    for seq in range(1, RUNS + 1):
        print(f"[m8_perf_backfill] === {key} 第 {seq}/{RUNS} 次进程级独立运行 ===")
        run_subprocess([py, script, "--emit", f"evidence/{prefix}_{date}_{seq}.json"])
    docs = [json.loads((ROOT / f"evidence/{prefix}_{date}_{seq}.json").read_text(encoding="utf-8"))
            for seq in range(1, RUNS + 1)]
    levels = {d["evidence_level"] for d in docs}
    if levels != {"measured_local"}:
        raise RuntimeError(
            f"{key}: 存在非 measured_local 运行({levels}),整组作废(契约 G-M8-2,unlocked 不得回填)"
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
    agg["notes"] = (f"aggregate of {RUNS} process-level independent runs "
                    f"({prefix}_{date}_1..{RUNS}.json); trial_medians = per-run trimmed means. "
                    + agg.get("notes", ""))
    out = ROOT / f"evidence/{prefix}_{date}_agg.json"
    out.write_text(json.dumps(agg, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    unit = agg["results"]["unit"]
    print(f"[m8_perf_backfill] {key} aggregate: {out.name} = {agg_value:.4f} {unit}")
    return out.relative_to(ROOT).as_posix(), round(agg_value, 4), unit


def main() -> int:
    only = None
    if "--only" in sys.argv:
        only = set(sys.argv[sys.argv.index("--only") + 1].split(","))
    lock_clocks.require_locked()  # 采样前置闸门:未锁频拒绝采样(契约 G-M8-2,零 estimated)
    date = datetime.date.today().strftime("%Y%m%d")

    selected = [b for b in BENCHES if only is None or b[0] in only]
    results: dict[str, tuple[str, float, str]] = {}
    for key, _eid, script, _unit, prefix in selected:
        results[key] = collect_and_aggregate(key, script, prefix, date)

    budget = json.loads(BUDGET.read_text(encoding="utf-8"))
    entries = {e["id"]: e for e in budget.get("entries", [])}
    for key, eid, _script, unit, _prefix in selected:
        ef, value, _u = results[key]
        direction = "min"
        entries[eid] = {
            "id": eid,
            "direction": direction,
            "threshold": round(value * THRESHOLD_TOLERANCE, 2),
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": unit,
            "evidence_file": ef,
            "measured_value": value,
        }
    budget["entries"] = list(entries.values())

    ratios = {r["id"]: r for r in budget.get("ratio_assertions", [])}
    for rid, num_key, den_key in RATIOS:
        if num_key not in results or den_key not in results:
            continue  # --only 子集未覆盖该比值,跳过
        num_eid = next(e for k, e, *_ in BENCHES if k == num_key)
        den_eid = next(e for k, e, *_ in BENCHES if k == den_key)
        nv = results[num_key][1]
        dv = results[den_key][1]
        ratios[rid] = {
            "id": rid,
            "numerator": num_eid,
            "denominator": den_eid,
            "direction": "min",
            "threshold": RATIO_THRESHOLD,
            "evidence": "measured_local",
            "skip_reason": None,
            "measured_value": round(nv / dv, 4),
        }
    budget["ratio_assertions"] = list(ratios.values())

    summary = ", ".join(
        f"{rid.split('.')[-1]}={ratios[rid]['measured_value']}"
        for rid, *_ in RATIOS if rid in ratios
    )
    budget["revision_log"].append({
        "version": f"v1.{len(budget['revision_log'])}",
        "date": datetime.date.today().isoformat(),
        "change": "M8.2 回填:UC-01 自研(saxpy/reduce/gemm)+ cublas(gemm/gemv)L1/L2 性能 "
                  "measured_local(三次进程级独立运行 trimmed mean,BENCH_PROTOCOL §3)→ "
                  "9 entries(numerator + 手写 CUDA C++ 对照 denominator,threshold=实测×0.95)+ "
                  f"5 ratio_assertions vs 手写 CUDA C++(阈值 0.90,契约 G-M8-2 / 01 §6):{summary}",
    })
    BUDGET.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[m8_perf_backfill] budget backfilled: {summary}")
    print("[m8_perf_backfill] 下一步核验:py -3 ci/budget_eval.py --strict")
    return 0


if __name__ == "__main__":
    sys.exit(main())
