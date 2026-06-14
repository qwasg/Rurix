"""Rurix SAXPY 三次进程级独立运行 + m4_budget.json 回填(契约 G-M4-1,M4.4)。

**操作者工具**(需锁频 L0 在位,BENCH_PROTOCOL.md §2/§3):每次重过 L0、重装载,
三次 trimmed mean 再 trimmed mean 聚合;**任一次非 measured_local(锁频降级
unlocked)→ 整组作废,拒绝回填**(契约 G-M4-1,unlocked 不得回填)。

回填 [milestones/m4/m4_budget.json](../milestones/m4/m4_budget.json):
  - entries[] 追加/更新 numerator `m4.bench.saxpy.effective_bandwidth_gbps`
    (measured_local + evidence_file + threshold = 实测 × 0.95);
  - ratio_assertions `m4.ratio.saxpy_vs_m0_baseline` 翻 estimated→measured_local
    + measured_value = numerator / denominator(M0 手写基线);
  - revision_log 追加。

回填后核验:`py -3 ci/budget_eval.py --strict`(比值 ≥0.95 通过,契约 G-M4-1)。

用法(锁频后):py -3 bench/rurix_saxpy_triple.py
"""
from __future__ import annotations

import datetime
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench.stats import bootstrap_ci, cv, trimmed_mean

ROOT = Path(__file__).resolve().parent.parent
RUNS = 3
THRESHOLD_TOLERANCE = 0.95  # 阈值 = 实测 × 0.95(5% Critical 门,r11 §7 / 08 §4)
PREFIX = "rurix_saxpy"
NUMERATOR_ID = "m4.bench.saxpy.effective_bandwidth_gbps"
RATIO_ID = "m4.ratio.saxpy_vs_m0_baseline"


def run_subprocess(cmd: list[str]) -> None:
    print(f"  $ {' '.join(cmd)}")
    if subprocess.run(cmd, cwd=ROOT).returncode != 0:
        raise RuntimeError(f"子进程失败: {' '.join(cmd)}")


def collect_runs() -> str:
    date = datetime.date.today().strftime("%Y%m%d")
    py = sys.executable
    for seq in range(1, RUNS + 1):
        print(f"[rurix_saxpy_triple] === 第 {seq}/{RUNS} 次进程级独立运行 ===")
        run_subprocess([py, "bench/rurix_saxpy_bench.py", "--emit",
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
            f"{PREFIX}: 存在非 measured_local 运行({levels}),整组作废(契约 G-M4-1,"
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
        f"({PREFIX}_{date}_1..{RUNS}.json); trial_medians = per-run trimmed means. "
        + agg.get("notes", "")
    )
    out = ROOT / f"evidence/{PREFIX}_{date}_agg.json"
    out.write_text(json.dumps(agg, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[rurix_saxpy_triple] aggregate: {out.name} = {agg_value:.4f} {agg['results']['unit']}")
    return out, agg


def baseline_value() -> float:
    """M0 手写 PTX 基线(denominator,measured_local 锚点)。"""
    m0 = json.loads((ROOT / "milestones/m0/m0_budget.json").read_text(encoding="utf-8"))
    for e in m0["entries"]:
        if e["id"] == "m0.bench.saxpy.effective_bandwidth_gbps":
            return float(e["measured_value"])
    raise RuntimeError("m0.bench.saxpy.effective_bandwidth_gbps 缺失(denominator)")


def backfill_budget(agg_path: Path, agg: dict) -> None:
    value = agg["results"]["trimmed_mean"]
    rel = agg_path.relative_to(ROOT).as_posix()
    budget_path = ROOT / "milestones/m4/m4_budget.json"
    budget = json.loads(budget_path.read_text(encoding="utf-8"))

    # numerator entry(自参照回归地板 threshold = 实测 × 0.95;ratio 才是 G-M4-1 硬门)
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

    den = baseline_value()
    ratio = value / den
    for r in budget["ratio_assertions"]:
        if r["id"] == RATIO_ID:
            r["evidence"] = "measured_local"
            r["measured_value"] = round(ratio, 4)
            r["skip_reason"] = None

    budget["revision_log"].append({
        "version": f"v1.{len(budget['revision_log'])}",
        "date": datetime.date.today().isoformat(),
        "change": "M4.4 回填:Rurix SAXPY measured_local(三次进程级独立运行 trimmed mean "
                  f"= {value:.4f} GB/s)→ numerator entry + ratio vs M0 基线 "
                  f"({den:.4f} GB/s)= {ratio:.4f}(契约 G-M4-1,阈值 0.95)",
    })
    budget_path.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n",
                           encoding="utf-8")
    print(f"[rurix_saxpy_triple] budget backfilled: ratio = {ratio:.4f} "
          f"(numerator {value:.4f} / denominator {den:.4f}; 阈值 0.95 "
          f"{'PASS' if ratio >= 0.95 else 'FAIL'})")
    print("[rurix_saxpy_triple] 下一步核验:py -3 ci/budget_eval.py --strict")


def main() -> int:
    date = collect_runs()
    agg_path, agg = aggregate(date)
    backfill_budget(agg_path, agg)
    return 0


if __name__ == "__main__":
    sys.exit(main())
