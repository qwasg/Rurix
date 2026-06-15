# -*- coding: utf-8 -*-
"""LSP 10k 行交互延迟三次进程级独立运行总控(契约 G-M6-2;形态对标
bench/compile_triple_run.py)。

bench/lsp_bench.py × 3 次子进程运行(每次重起 rurixc --tooling-server,进程级隔离)
→ 逐交互跨 run trimmed mean 聚合证据 JSON → 回填 milestones/m6/m6_budget.json 的
m6.bench.lsp_interaction_latency_ms 由 estimated 占位为 measured_local。

阈值方向 = max(延迟越低越好,阈值为上界):每交互阈值 = 实测 trimmed mean ×
(1 + 余量),并保证 publishDiagnostics 阈值 < 5000ms(07 §6 增量 check < 5s 行业线
天花板)。余量(MARGIN)默认 0.5(50% 上界冗余,应对 Windows/桌面噪声),**数值经
人工批准**(契约 G-M6-2 / 硬规则 1),本脚本回填后于 revision_log 标注 agent 提案、
待人工终审。

用法:py -3 bench/lsp_latency_triple.py [--lines 10000] [--margin 0.5]
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
DIGITS = 2
ENTRY_ID = "m6.bench.lsp_interaction_latency_ms"
INTERACTIONS = ("completion", "definition", "publishDiagnostics")
LATENCY_CEILING_MS = 5000.0  # 07 §6 增量 check < 5s 行业线天花板


def collect_runs(lines: int) -> tuple[str, list[Path]]:
    date = datetime.date.today().strftime("%Y%m%d")
    py = sys.executable
    paths: list[Path] = []
    for seq in range(1, RUNS + 1):
        print(f"[lsp_latency_triple] === 第 {seq}/{RUNS} 次进程级独立运行 ===")
        out = f"evidence/lsp_latency_{date}_{seq}.json"
        proc = subprocess.run(
            [py, "bench/lsp_bench.py", "--lines", str(lines), "--emit", out], cwd=ROOT
        )
        if proc.returncode != 0:
            raise RuntimeError(f"子进程失败: lsp_bench.py 第 {seq} 次运行")
        paths.append(ROOT / out)
    return date, paths


def aggregate(date: str, run_paths: list[Path]) -> tuple[Path, dict]:
    docs = [json.loads(p.read_text(encoding="utf-8")) for p in run_paths]
    levels = {d["evidence_level"] for d in docs}
    if levels != {"measured_local"}:
        raise RuntimeError(
            f"存在非 measured_local 运行({levels}),整组作废(BENCH_PROTOCOL §3)"
        )

    agg = json.loads(json.dumps(docs[0]))  # 以第 1 次运行为模板(保留 schema 必填字段)
    agg["timestamp"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    per_agg: dict[str, float] = {}
    for name in INTERACTIONS:
        run_means = [d["results"]["per_interaction"][name]["trimmed_mean"] for d in docs]
        value = trimmed_mean(run_means, 0.2)
        ci_lo, ci_hi = bootstrap_ci(run_means, statistic="median")
        sub = agg["results"]["per_interaction"][name]
        sub["trimmed_mean"] = round(value, 4)
        sub["trial_medians"] = [round(v, 4) for v in run_means]
        sub["cv"] = round(cv(run_means), 6)
        sub["ci95"] = [round(ci_lo, 4), round(ci_hi, 4)]
        sub["min"] = round(min(run_means), 4)
        sub["max"] = round(max(run_means), 4)
        per_agg[name] = round(value, 4)

    worst_name = max(per_agg, key=per_agg.get)
    agg["results"]["trimmed_mean"] = per_agg[worst_name]
    agg["results"]["worst_interaction"] = worst_name
    agg["notes"] = (
        f"aggregate of {RUNS} process-level independent runs "
        f"(lsp_latency_{date}_1..{RUNS}.json); per_interaction.trial_medians = "
        f"per-run trimmed means. " + agg.get("notes", "")
    )
    out = ROOT / f"evidence/lsp_latency_{date}_agg.json"
    out.write_text(json.dumps(agg, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[lsp_latency_triple] aggregate: {out.name} "
          f"(worst {worst_name} {per_agg[worst_name]:.4f} ms)")
    return out, agg


def backfill_budget(agg_path: Path, agg: dict, margin: float) -> None:
    budget_path = ROOT / "milestones/m6/m6_budget.json"
    budget = json.loads(budget_path.read_text(encoding="utf-8"))

    measured = {n: agg["results"]["per_interaction"][n]["trimmed_mean"] for n in INTERACTIONS}
    thresholds = {n: round(measured[n] * (1 + margin), DIGITS) for n in INTERACTIONS}
    if thresholds["publishDiagnostics"] >= LATENCY_CEILING_MS:
        raise RuntimeError(
            f"publishDiagnostics 阈值 {thresholds['publishDiagnostics']}ms 触碰 "
            f"{LATENCY_CEILING_MS}ms 行业线天花板(07 §6),实测异常需排查而非放阈"
        )

    for entry in budget["entries"]:
        if entry["id"] != ENTRY_ID:
            continue
        entry["evidence"] = "measured_local"
        entry["evidence_file"] = agg_path.relative_to(ROOT).as_posix()
        # 标量 threshold(满足既有 check_schemas measured_local 校验)= 最坏交互上界;
        # 逐交互上界在 thresholds,由 ci/budget_eval.py 特例分支逐一对阈。
        entry["threshold"] = thresholds[agg["results"]["worst_interaction"]]
        entry["thresholds"] = thresholds
        entry["measured"] = measured
        entry["skip_reason"] = None

    budget["revision_log"].append({
        "version": f"v1.{len(budget['revision_log'])}",
        "date": datetime.date.today().isoformat(),
        "change": "M6.5 回填(G-M6-2):lsp_latency_triple 三次进程级独立运行 trimmed mean,"
                  f"m6.bench.lsp_interaction_latency_ms estimated → measured_local;逐交互"
                  f"(completion/definition/publishDiagnostics)阈值 = 实测 × (1 + {margin})"
                  "(max 方向上界,publishDiagnostics < 5000ms 07 §6 行业线天花板内);"
                  "标量 threshold = 最坏交互上界(满足 check_schemas);direction/unit/id 语义不改。"
                  "阈值为 agent 提案,待人工终审批准(硬规则 1)",
    })
    budget_path.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n",
                           encoding="utf-8")
    print(f"[lsp_latency_triple] budget backfilled: {budget_path.relative_to(ROOT)}")
    print(f"  measured(ms)={measured}")
    print(f"  thresholds(ms)={thresholds}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--lines", type=int, default=10_000)
    ap.add_argument("--margin", type=float, default=DEFAULT_MARGIN)
    args = ap.parse_args()
    date, run_paths = collect_runs(args.lines)
    agg_path, agg = aggregate(date, run_paths)
    backfill_budget(agg_path, agg, args.margin)
    return 0


if __name__ == "__main__":
    sys.exit(main())
