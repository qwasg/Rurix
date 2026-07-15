"""编译性能基准三次进程级独立运行总控(契约 G-M3-3;形态对标
bench/frontend_triple_run.py)。

compile_bench.py(cold_compile / check_latency)各 × 3 次子进程运行(进程级隔离)
→ 聚合证据 JSON → 回填 milestones/m2/m2_budget.json 两条 estimated 占位为
measured_local(m2.bench.cold_compile_hello_world_ms / m2.bench.check_latency_ms)。

阈值方向 = max(延迟越低越好,阈值为上界):阈值 = 实测 trimmed mean × (1 + 余量)。
余量(MARGIN)默认 0.5(50% 上界冗余,应对 Windows 文件系统/杀软扫描噪声,
BENCH_PROTOCOL 环境画像纪律);**数值经自主批准**(契约 G-M3-3 / 硬规则 1),
本脚本回填后于 revision_log 标注 agent 提案、待人工终审。

用法:py -3 bench/compile_triple_run.py [--only cold|check] [--margin 0.5]
"""
from __future__ import annotations

import argparse
import datetime
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench.stats import bootstrap_ci, cv, trimmed_mean

ROOT = Path(__file__).resolve().parent.parent
RUNS = 3
DEFAULT_MARGIN = 0.5  # 阈值 = 实测 × (1 + margin);max 方向上界冗余

BENCHES = {
    # 前缀 → (compile_bench --bench 值, 预算条目 id, 阈值小数位)
    "compile_cold": ("cold_compile", "m2.bench.cold_compile_hello_world_ms", 2),
    "compile_check": ("check_latency", "m2.bench.check_latency_ms", 2),
}


def run_subprocess(cmd: list[str]) -> None:
    print(f"  $ {' '.join(cmd)}")
    proc = subprocess.run(cmd, cwd=ROOT)
    if proc.returncode != 0:
        raise RuntimeError(f"子进程失败: {' '.join(cmd)}")


def selected_prefixes(only: str | None) -> list[str]:
    if only is None:
        return list(BENCHES)
    prefix = f"compile_{only}"
    if prefix not in BENCHES:
        raise SystemExit(f"--only 取值仅支持 cold|check,得到 {only!r}")
    return [prefix]


def collect_runs(prefixes: list[str]) -> str:
    date = datetime.date.today().strftime("%Y%m%d")
    py = sys.executable
    for seq in range(1, RUNS + 1):
        print(f"[compile_triple_run] === 第 {seq}/{RUNS} 次进程级独立运行 ===")
        for prefix in prefixes:
            bench, _, _ = BENCHES[prefix]
            run_subprocess([py, "bench/compile_bench.py", "--bench", bench,
                            "--emit", f"evidence/{prefix}_{date}_{seq}.json"])
    return date


def aggregate(prefix: str, date: str) -> tuple[Path, dict]:
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
    print(f"[compile_triple_run] aggregate: {out.name} = {agg_value:.4f} ms")
    return out, agg


def backfill_budget(aggregates: dict[str, tuple[Path, dict]], margin: float) -> None:
    budget_path = ROOT / "milestones/m2/m2_budget.json"
    budget = json.loads(budget_path.read_text(encoding="utf-8"))

    for entry in budget["entries"]:
        match = [p for p, (_, eid, _) in BENCHES.items() if eid == entry["id"]]
        if not match or match[0] not in aggregates:
            continue
        prefix = match[0]
        _, _, digits = BENCHES[prefix]
        path, agg = aggregates[prefix]
        value = agg["results"]["trimmed_mean"]
        # max 方向:阈值为上界 = 实测 × (1 + 余量)
        entry["threshold"] = round(value * (1 + margin), digits)
        entry["evidence"] = "measured_local"
        entry["measured_value"] = value
        entry["evidence_file"] = path.relative_to(ROOT).as_posix()
        entry["skip_reason"] = None

    budget["revision_log"].append({
        "version": f"v1.{len(budget['revision_log'])}",
        "date": datetime.date.today().isoformat(),
        "change": "M3.4 回填(G-M3-3):compile_triple_run 三次进程级独立运行 trimmed mean,"
                  f"阈值 = 实测 × (1 + {margin})(max 方向上界);"
                  "阈值余量为 agent 提案,待人工终审批准(硬规则 1)",
    })
    budget_path.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n",
                           encoding="utf-8")
    print(f"[compile_triple_run] budget backfilled: {budget_path.relative_to(ROOT)}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", choices=["cold", "check"])
    ap.add_argument("--margin", type=float, default=DEFAULT_MARGIN)
    args = ap.parse_args()
    prefixes = selected_prefixes(args.only)
    date = collect_runs(prefixes)
    aggregates = {prefix: aggregate(prefix, date) for prefix in prefixes}
    backfill_budget(aggregates, args.margin)
    return 0


if __name__ == "__main__":
    sys.exit(main())
