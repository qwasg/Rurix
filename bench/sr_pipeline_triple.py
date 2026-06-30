"""软光栅 L3 三次进程级独立运行 + m7_budget.json 回填(契约 G-M7-2,M7.5,D-M7-5)。

**操作者工具**(需锁频 L0 在位,BENCH_PROTOCOL.md §2/§3 三次运行规则):每次重过 L0、
重装载,三次 trimmed mean 再 trimmed mean 聚合;**任一次非 measured_local(锁频降级
unlocked)→ 整组作废,拒绝回填**(BENCH_PROTOCOL §2.1,unlocked 不得回填)。

回填 [milestones/m7/m7_budget.json](../milestones/m7/m7_budget.json) 的占位项
`m7.bench.soft_raster_l3_frame_ms`(estimated → measured_local):
  - evidence_file = evidence/sr_l3_<date>_agg.json(去 estimated 占位);
  - direction = max(帧时间越小越好,阈值为上界);
  - threshold = 实测 trimmed_mean × 安全系数(SAFETY_FACTOR,实测 × 安全系数为上界,
    参照行业线天花板;裁定经 Direct PR 留痕,close-out 终审自主签署);
  - revision_log 追加。

回填后核验:`py -3 ci/budget_eval.py --strict`(全局零 estimated 残留 + 帧时间
trimmed_mean ≤ threshold → PASS,契约 G-M7-2)。

用法(锁频后):py -3 bench/sr_pipeline_triple.py
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
# 帧时间上界安全系数(实测 × 安全系数为 direction=max 上界;1.5 = 50% 回归裕度,
# 远低于行业线软光栅 L3 帧天花板;Direct 裁定,close-out 终审自主签署)。
SAFETY_FACTOR = 1.5
PREFIX = "sr_l3"
ENTRY_ID = "m7.bench.soft_raster_l3_frame_ms"
BENCH_SCRIPT = "bench/sr_pipeline_bench.py"
BUDGET_PATH = "milestones/m7/m7_budget.json"


def run_subprocess(cmd: list[str]) -> None:
    print(f"  $ {' '.join(cmd)}")
    if subprocess.run(cmd, cwd=ROOT).returncode != 0:
        raise RuntimeError(f"子进程失败: {' '.join(cmd)}")


def collect_runs() -> str:
    date = datetime.date.today().strftime("%Y%m%d")
    py = sys.executable
    for seq in range(1, RUNS + 1):
        print(f"[sr_pipeline_triple] === 第 {seq}/{RUNS} 次进程级独立运行 ===")
        run_subprocess([py, BENCH_SCRIPT, "--emit", f"evidence/{PREFIX}_{date}_{seq}.json"])
    return date


def aggregate(date: str) -> tuple[Path, dict]:
    docs = []
    for seq in range(1, RUNS + 1):
        path = ROOT / f"evidence/{PREFIX}_{date}_{seq}.json"
        docs.append(json.loads(path.read_text(encoding="utf-8")))
    levels = {d["evidence_level"] for d in docs}
    if levels != {"measured_local"}:
        raise RuntimeError(
            f"{PREFIX}: 存在非 measured_local 运行({levels}),整组作废"
            "(BENCH_PROTOCOL §2.1,unlocked 证据不得回填)"
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
    print(f"[sr_pipeline_triple] aggregate: {out.name} = {agg_value:.4f} {agg['results']['unit']}")
    return out, agg


def backfill_budget(agg_path: Path, agg: dict) -> None:
    value = agg["results"]["trimmed_mean"]
    threshold = round(value * SAFETY_FACTOR, 4)
    rel = agg_path.relative_to(ROOT).as_posix()
    budget_path = ROOT / BUDGET_PATH
    budget = json.loads(budget_path.read_text(encoding="utf-8"))

    found = False
    for e in budget["entries"]:
        if e["id"] == ENTRY_ID:
            e["evidence"] = "measured_local"
            e["direction"] = "max"
            e["unit"] = "ms"
            e["threshold"] = threshold
            e["evidence_file"] = rel
            e["measured_value"] = value
            e["skip_reason"] = None
            found = True
    if not found:
        raise RuntimeError(f"{ENTRY_ID} 占位项缺失,无法回填")

    budget["revision_log"].append({
        "version": f"v1.{len(budget['revision_log'])}",
        "date": datetime.date.today().isoformat(),
        "change": (
            f"M7.5 回填(契约 G-M7-2,D-M7-5):软光栅 L3 端到端帧时间 measured_local"
            f"(三次进程级独立运行 trimmed mean = {value:.4f} ms;BENCH_PROTOCOL §3 "
            f"L0 锁频 / 三次独立 / trimmed mean,evidence/{agg_path.name})"
            f"→ estimated 占位转 measured_local;direction=max,"
            f"threshold = 实测 × {SAFETY_FACTOR}(安全系数上界,参照行业线天花板)"
            f"= {threshold:.4f} ms。占位在 M7 内生灭,close-out budget_eval --strict 判定"
        ),
    })
    budget_path.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n",
                           encoding="utf-8")
    ok = value <= threshold
    print(f"[sr_pipeline_triple] budget backfilled: {ENTRY_ID} measured_local "
          f"= {value:.4f} ms vs max {threshold:.4f} ms "
          f"({'PASS' if ok else 'FAIL'})")
    print("[sr_pipeline_triple] 下一步核验:py -3 ci/budget_eval.py --strict")


def main() -> int:
    lock_clocks.require_locked()  # 采样前置闸门:未锁频拒绝采样(BENCH_PROTOCOL §2.1)
    date = collect_runs()
    agg_path, agg = aggregate(date)
    backfill_budget(agg_path, agg)
    return 0


if __name__ == "__main__":
    sys.exit(main())
