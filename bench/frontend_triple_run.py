"""前端基准三次进程级独立运行总控(契约 G-M1-3;形态对标 bench/triple_run.py)。

lexer_bench.py / parser_bench.py 各 × 3 次子进程运行(进程级隔离)→
聚合证据 JSON → 回填 m1_budget.json 为 measured_local
(阈值 = 实测 × 0.95,对齐 M0 先例)。

用法:py -3 bench/frontend_triple_run.py [--only lexer|parser]
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
THRESHOLD_TOLERANCE = 0.95  # 阈值 = 实测 × 0.95(M0 先例,5% Critical 门)

BENCHES = {
    # 前缀 → (harness 脚本, 预算条目 id, 阈值小数位)
    "frontend_lexer": ("bench/lexer_bench.py", "m1.bench.lexer.throughput_mbps", 2),
    "frontend_parser": ("bench/parser_bench.py", "m1.bench.parser.throughput_kloc_per_s", 2),
}


def run_subprocess(cmd: list[str]) -> None:
    print(f"  $ {' '.join(cmd)}")
    proc = subprocess.run(cmd, cwd=ROOT)
    if proc.returncode != 0:
        raise RuntimeError(f"子进程失败: {' '.join(cmd)}")


def selected_prefixes(only: str | None) -> list[str]:
    if only is None:
        return list(BENCHES)
    prefix = f"frontend_{only}"
    if prefix not in BENCHES:
        raise SystemExit(f"--only 取值仅支持 lexer|parser,得到 {only!r}")
    return [prefix]


def collect_runs(prefixes: list[str]) -> str:
    date = datetime.date.today().strftime("%Y%m%d")
    py = sys.executable
    for seq in range(1, RUNS + 1):
        print(f"[frontend_triple_run] === 第 {seq}/{RUNS} 次进程级独立运行 ===")
        for prefix in prefixes:
            script, _, _ = BENCHES[prefix]
            run_subprocess([py, script, "--emit", f"evidence/{prefix}_{date}_{seq}.json"])
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
    print(
        f"[frontend_triple_run] aggregate: {out.name} = "
        f"{agg_value:.4f} {agg['results']['unit']}"
    )
    return out, agg


def backfill_budget(aggregates: dict[str, tuple[Path, dict]]) -> None:
    budget_path = ROOT / "milestones/m1/m1_budget.json"
    budget = json.loads(budget_path.read_text(encoding="utf-8"))

    for entry in budget["entries"]:
        match = [p for p, (_, eid, _) in BENCHES.items() if eid == entry["id"]]
        if not match or match[0] not in aggregates:
            continue
        prefix = match[0]
        _, _, digits = BENCHES[prefix]
        path, agg = aggregates[prefix]
        value = agg["results"]["trimmed_mean"]
        entry["threshold"] = round(value * THRESHOLD_TOLERANCE, digits)
        entry["evidence"] = "measured_local"
        entry["measured_value"] = value
        entry["evidence_file"] = path.relative_to(ROOT).as_posix()
        entry["skip_reason"] = None

    budget["revision_log"].append({
        "version": f"v1.{len(budget['revision_log'])}",
        "date": datetime.date.today().isoformat(),
        "change": "M1.4 回填:frontend_triple_run 三次进程级独立运行 trimmed mean,"
                  f"阈值 = 实测 × {THRESHOLD_TOLERANCE}(契约 G-M1-3,M0 先例)",
    })
    budget_path.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n",
                           encoding="utf-8")
    print(f"[frontend_triple_run] budget backfilled: {budget_path.relative_to(ROOT)}")


def main() -> int:
    only = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None
    prefixes = selected_prefixes(only)
    date = collect_runs(prefixes)
    aggregates = {prefix: aggregate(prefix, date) for prefix in prefixes}
    backfill_budget(aggregates)
    return 0


if __name__ == "__main__":
    sys.exit(main())
