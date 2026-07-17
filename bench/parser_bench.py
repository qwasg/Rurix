"""parser 吞吐基准 harness(M1 契约 D-M1-6;M1_PLAN §3 任务 5)。

用法:
  py -3 bench/parser_bench.py --smoke                 # 小语料 + 5 次迭代,正确性哨兵(Nightly 冒烟)
  py -3 bench/parser_bench.py --emit evidence/frontend_parser_x.json  # 完整协议采样

协议:BENCH_PROTOCOL.md §3 的统计形态(warmup ≥10 + 稳态 CV<5% → 50×3 →
trial 内中位数 → 跨 trial trimmed mean;统计函数复用 bench/stats.py);
计时为 parse_bench 进程内 std::time::Instant(逐迭代 ns,仅覆盖 parse 本体,
token 流预 lex 并在计时区外 clone)。
口径:kloc/s(m1.bench.parser.throughput_kloc_per_s;loc = 物理行数)。
预算回填(G-M1-3)须三次进程级独立运行,见 milestones/m1/M1_PLAN.md §4。
"""
from __future__ import annotations

import datetime
import json
import os
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench.stats import bootstrap_ci, cv, iqr_filter, steady_state_reached, trimmed_mean
from bench.proc_guard import guarded_run, EXE_RUN_TIMEOUT, CARGO_BUILD_TIMEOUT

ROOT = Path(__file__).resolve().parent.parent
CORPUS_DIR = ROOT / "conformance" / "syntax"
BIN = ROOT / "target" / "release" / ("parse_bench.exe" if os.name == "nt" else "parse_bench")

# 协议参数(BENCH_PROTOCOL §3 同源;与 lexer_bench.py 一致)
WARMUP_MIN = 10
WARMUP_MAX = 50
STEADY_WINDOW = 5
STEADY_CV = 0.05
TIMED_ITERATIONS = 50
TRIALS = 3
TRIM = 0.2

TARGET_CORPUS_BYTES = 4 * 1024 * 1024  # ~4 MiB 放大语料
SMOKE_ITERS = 5


def count_loc(text: str) -> int:
    """物理行数(与 parse_bench.rs 的 src.lines().count() 同口径)。"""
    return len(text.splitlines())


def kloc_per_s(ns: int, loc: int) -> float:
    """单次迭代吞吐:kloc/s。"""
    return (loc / 1000.0) / (ns / 1e9)


def git_commit() -> str:
    out = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT,
                         capture_output=True, text=True, check=False)
    return out.stdout.strip() or "unknown"


def build_corpus(target_bytes: int) -> tuple[str, str]:
    """拼接 conformance/syntax 全量样例并放大到目标体量。返回 (语料文本, 规模描述)。

    含 `#![feature(...)]` 内部属性的样例剔除(拼接后不再处于文件顶部,RXS-0011)。
    """
    files = sorted(CORPUS_DIR.glob("**/*.rx"))
    if not files:
        raise FileNotFoundError("conformance/syntax 为空,无基准语料")
    texts = [t for f in files if "#![" not in (t := f.read_text(encoding="utf-8"))]
    unit = "\n".join(texts)
    repeats = max(1, target_bytes // len(unit.encode("utf-8")))
    corpus = "\n".join([unit] * repeats)
    size_kloc = count_loc(corpus) / 1000.0
    return corpus, f"{size_kloc:.1f} kloc corpus ({len(texts)} files x {repeats})"


def ensure_binary() -> None:
    print("[parser_bench] cargo build --release --bin parse_bench ...")
    r = guarded_run(["cargo", "build", "--release", "--bin", "parse_bench"],
                    cwd=ROOT, timeout=CARGO_BUILD_TIMEOUT, capture=False,
                    label="cargo build parse_bench")
    if r.returncode != 0:
        raise RuntimeError(f"cargo build parse_bench 退出码 {r.returncode}"
                           + ("(超时,已杀进程树)" if r.timed_out else ""))
    if not BIN.is_file():
        raise FileNotFoundError(f"构建产物不存在: {BIN}")


def run_binary(corpus_path: Path, iters: int) -> tuple[int, list[int]]:
    """执行 parse_bench,返回 (语料 loc, 逐迭代 ns 列表)。"""
    proc = guarded_run(
        [str(BIN), str(corpus_path), str(iters)],
        timeout=EXE_RUN_TIMEOUT, quarantine_exe=BIN, label="parse_bench",
    )
    if proc.returncode != 0:
        raise RuntimeError(f"parse_bench 退出码 {proc.returncode}: {proc.stderr.strip()}")
    lines = [json.loads(line) for line in proc.stdout.splitlines() if line.strip()]
    header, samples = lines[0], lines[1:]
    if header.get("items", 0) <= 0:
        raise RuntimeError("parse_bench 哨兵行异常(items<=0)")
    return header["loc"], [s["ns"] for s in samples]


def collect_environment() -> dict:
    power = subprocess.run(["powercfg", "/getactivescheme"],
                           capture_output=True, text=True, check=False)
    power_plan = power.stdout.strip().split("(")[-1].rstrip(")").strip() if power.stdout else "unavailable"
    return {
        "cpu_name": platform.processor() or "unavailable",
        "logical_cores": os.cpu_count() or 0,
        "power_plan": power_plan or "unavailable",
        "os_build": platform.version(),
        "clock_control": "not_applicable_cpu",
        "background_note": "desktop dev machine; background load not isolated, "
                           "IQR rejection per protocol; runs interleaved with no deliberate load",
    }


def main() -> int:
    smoke = "--smoke" in sys.argv
    emit = None
    if "--emit" in sys.argv:
        emit = ROOT / sys.argv[sys.argv.index("--emit") + 1]

    ensure_binary()
    target = 256 * 1024 if smoke else TARGET_CORPUS_BYTES
    corpus, problem_size = build_corpus(target)
    with tempfile.TemporaryDirectory() as td:
        corpus_path = Path(td) / "corpus.rx"
        corpus_path.write_text(corpus, encoding="utf-8")

        if smoke:
            loc, ns_list = run_binary(corpus_path, SMOKE_ITERS)
            best = max(kloc_per_s(ns, loc) for ns in ns_list)
            print(f"[parser_bench] smoke PASS ({problem_size}, best {best:.1f} kloc/s, "
                  f"{SMOKE_ITERS} iters, correctness sentinel ok)")
            return 0

        total = WARMUP_MAX + TRIALS * TIMED_ITERATIONS
        loc, ns_list = run_binary(corpus_path, total)

    # --- warmup + 稳态判定(对逐迭代耗时序列做协议判定) ---
    warmup_ms = []
    idx = 0
    while idx < WARMUP_MAX:
        warmup_ms.append(ns_list[idx] / 1e6)
        idx += 1
        if len(warmup_ms) >= WARMUP_MIN and steady_state_reached(warmup_ms, STEADY_WINDOW, STEADY_CV):
            break
    steady_cv_value = cv(warmup_ms[-STEADY_WINDOW:])

    # --- 50 × 3 timed ---
    timed = ns_list[idx: idx + TRIALS * TIMED_ITERATIONS]
    if len(timed) < TRIALS * TIMED_ITERATIONS:
        raise RuntimeError("采样不足(warmup 消耗超出预算)")
    trial_medians: list[float] = []
    all_metrics: list[float] = []
    for t in range(TRIALS):
        chunk = timed[t * TIMED_ITERATIONS:(t + 1) * TIMED_ITERATIONS]
        metrics = sorted(kloc_per_s(ns, loc) for ns in chunk)
        all_metrics.extend(metrics)
        trial_medians.append(metrics[len(metrics) // 2])

    kept, rejected = iqr_filter(all_metrics)
    result_mean = trimmed_mean(trial_medians, TRIM)
    ci_lo, ci_hi = bootstrap_ci(kept, statistic="median")

    doc = {
        "schema_version": 1,
        "evidence_level": "measured_local",
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "bench": {
            "id": "parser_throughput",
            "level": "frontend",
            "problem_size": problem_size,
            "harness_commit": git_commit(),
        },
        "environment": collect_environment(),
        "sampling": {
            "warmup_iterations": len(warmup_ms),
            "steady_state_cv": round(steady_cv_value, 6),
            "timed_iterations": TIMED_ITERATIONS,
            "trials": TRIALS,
            "trimmed_pct": TRIM,
            "timer": "std_instant",
        },
        "results": {
            "metric": "parse_throughput",
            "unit": "kloc/s",
            "trial_medians": [round(v, 2) for v in trial_medians],
            "trimmed_mean": round(result_mean, 2),
            "cv": round(cv(kept), 6),
            "ci95": [round(ci_lo, 2), round(ci_hi, 2)],
            "min": round(min(all_metrics), 2),
            "max": round(max(all_metrics), 2),
            "outliers_rejected_iqr": len(rejected),
            "correctness_check": "pass",
        },
        "notes": "in-process Instant timing covering parse only (tokens pre-lexed, "
                 "cloned outside timed region); corpus = conformance/syntax concatenated "
                 "(inner-attr samples excluded); zero-diagnostic sentinel enforced by "
                 "parse_bench (exit 2 otherwise)",
    }
    if emit:
        emit.parent.mkdir(parents=True, exist_ok=True)
        emit.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[parser_bench] evidence written: {emit.relative_to(ROOT)} "
              f"({doc['results']['trimmed_mean']} kloc/s, level={doc['evidence_level']})")
    else:
        print(json.dumps(doc["results"], ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
