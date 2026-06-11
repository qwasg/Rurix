"""三次进程级独立运行总控(BENCH_PROTOCOL.md §3 三次运行规则,H05 triple_run 思路重写)。

每个基准 × 3 次子进程运行(进程级隔离,每次重过 L0)→ 聚合证据 JSON →
回填 m0_budget.json 为 measured_local(契约 G-M0-1)。

用法:py -3 bench/triple_run.py [--only saxpy|bandwidth]
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
THRESHOLD_TOLERANCE = 0.95  # 阈值 = 实测 × 0.95,对应 5% Critical 门(r11 §7 / 08 §4)

BANDWIDTH_DIRS = ("h2d_pinned", "h2d_pageable", "d2h_pinned", "d2h_pageable", "d2d")

BUDGET_MAP = {
    # 预算条目 id → 证据 bench 文件前缀
    "m0.bench.saxpy.effective_bandwidth_gbps": "saxpy",
    "m0.bench.bandwidth.h2d_pinned_gbps": "bandwidth_h2d_pinned",
    "m0.bench.bandwidth.h2d_pageable_gbps": "bandwidth_h2d_pageable",
    "m0.bench.bandwidth.d2h_pinned_gbps": "bandwidth_d2h_pinned",
    "m0.bench.bandwidth.d2h_pageable_gbps": "bandwidth_d2h_pageable",
    "m0.bench.bandwidth.d2d_gbps": "bandwidth_d2d",
}


def run_subprocess(cmd: list[str]) -> None:
    print(f"  $ {' '.join(cmd)}")
    proc = subprocess.run(cmd, cwd=ROOT)
    if proc.returncode != 0:
        raise RuntimeError(f"子进程失败: {' '.join(cmd)}")


def collect_runs(only: str | None) -> str:
    date = datetime.date.today().strftime("%Y%m%d")
    py = sys.executable
    for seq in range(1, RUNS + 1):
        print(f"[triple_run] === 第 {seq}/{RUNS} 次进程级独立运行 ===")
        if only in (None, "saxpy"):
            run_subprocess([py, "bench/saxpy_bench.py", "--emit",
                            f"evidence/saxpy_{date}_{seq}.json"])
        if only in (None, "bandwidth"):
            for d in BANDWIDTH_DIRS:
                run_subprocess([py, "bench/bandwidth_bench.py", "--direction", d,
                                "--emit", f"evidence/bandwidth_{d}_{date}_{seq}.json"])
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
            f"{prefix}: 存在非 measured_local 运行({levels}),整组作废(BENCH_PROTOCOL §3)"
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
    print(f"[triple_run] aggregate: {out.name} = {agg_value:.4f} {agg['results']['unit']}")
    return out, agg


def backfill_budget(aggregates: dict[str, tuple[Path, dict]]) -> None:
    budget_path = ROOT / "milestones/m0/m0_budget.json"
    budget = json.loads(budget_path.read_text(encoding="utf-8"))

    for entry in budget["entries"]:
        prefix = BUDGET_MAP.get(entry["id"])
        if prefix is None or prefix not in aggregates:
            continue
        path, agg = aggregates[prefix]
        value = agg["results"]["trimmed_mean"]
        entry["threshold"] = round(value * THRESHOLD_TOLERANCE, 2)
        entry["evidence"] = "measured_local"
        entry["measured_value"] = value
        entry["evidence_file"] = path.relative_to(ROOT).as_posix()
        entry["skip_reason"] = None

    if "saxpy" in aggregates and "bandwidth_d2d" in aggregates:
        saxpy_v = aggregates["saxpy"][1]["results"]["trimmed_mean"]
        d2d_v = aggregates["bandwidth_d2d"][1]["results"]["trimmed_mean"]
        ratio = saxpy_v / d2d_v
        for r in budget["ratio_assertions"]:
            if r["id"] == "m0.ratio.saxpy_vs_d2d_bandwidth":
                r["threshold"] = round(ratio * THRESHOLD_TOLERANCE, 4)
                r["evidence"] = "measured_local"
                r["measured_value"] = round(ratio, 4)
                r["skip_reason"] = None

    budget["revision_log"].append({
        "version": f"v1.{len(budget['revision_log'])}",
        "date": datetime.date.today().isoformat(),
        "change": "M0.3 回填:triple_run 三次进程级独立运行 trimmed mean,"
                  f"阈值 = 实测 × {THRESHOLD_TOLERANCE}(5% Critical 门)",
    })
    budget_path.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n",
                           encoding="utf-8")
    print(f"[triple_run] budget backfilled: {budget_path.relative_to(ROOT)}")


def main() -> int:
    only = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None
    date = collect_runs(only)
    aggregates: dict[str, tuple[Path, dict]] = {}
    prefixes = []
    if only in (None, "saxpy"):
        prefixes.append("saxpy")
    if only in (None, "bandwidth"):
        prefixes.extend(f"bandwidth_{d}" for d in BANDWIDTH_DIRS)
    for prefix in prefixes:
        aggregates[prefix] = aggregate(prefix, date)
    backfill_budget(aggregates)
    return 0


if __name__ == "__main__":
    sys.exit(main())
