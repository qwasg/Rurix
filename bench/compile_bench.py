# -*- coding: utf-8 -*-
"""编译性能基准 harness(M3 契约 G-M3-3;形态对标 bench/lexer_bench.py)。

用法:
  py -3 bench/compile_bench.py --bench cold_compile  --emit evidence/compile_cold_x.json
  py -3 bench/compile_bench.py --bench check_latency  --emit evidence/compile_check_x.json
  py -3 bench/compile_bench.py --bench cold_compile  --smoke   # 冒烟(少迭代,正确性哨兵)

计时:每次"编译" = 独立 rurixc 子进程 spawn(inherent process-level cold),
wall = time.perf_counter 包裹子进程;统计复用 BENCH_PROTOCOL §3 trimmed-mean,
迭代规模因单次编译昂贵下调(timer=process_wall)。
- cold_compile:`rurixc hello.rx -o out.exe`(全管线 → EXE,含 clang+link.exe);
- check_latency:`rurixc hello.rx --emit=check`(跑到 borrowck 止,不产物)。
预算回填(G-M3-3)须三次进程级独立运行,见 bench/compile_triple_run.py。
"""
from __future__ import annotations

import argparse
import datetime
import json
import os
import platform
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench.stats import bootstrap_ci, cv, iqr_filter, trimmed_mean

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "conformance" / "syntax" / "hello_world.rx"
BIN = ROOT / "target" / "release" / ("rurixc.exe" if os.name == "nt" else "rurixc")

# 协议参数(单次编译昂贵,迭代规模下调;trials≥3 满足 schema)
WARMUP = 2
TRIALS = 5
SMOKE_WARMUP = 1
SMOKE_TRIALS = 3
TRIM = 0.2

BENCHES = {
    "cold_compile": {
        "id": "cold_compile_hello_world",
        "metric": "cold_compile_ms",
        "problem_size": "hello_world.rx (full pipeline -> EXE, incl clang+link)",
        "emit": None,
    },
    "check_latency": {
        "id": "check_latency",
        "metric": "check_latency_ms",
        "problem_size": "hello_world.rx (--emit=check, no codegen/link)",
        "emit": "check",
    },
}


def git_commit() -> str:
    out = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT,
                         capture_output=True, text=True, check=False)
    return out.stdout.strip() or "unknown"


def ensure_binary() -> None:
    print("[compile_bench] cargo build --release --bin rurixc ...")
    subprocess.run(["cargo", "build", "--release", "--bin", "rurixc"], cwd=ROOT, check=True)
    if not BIN.is_file():
        raise FileNotFoundError(f"构建产物不存在: {BIN}")


def clang_version() -> str:
    for cand in (os.environ.get("RURIXC_CLANG"), "C:/Program Files/LLVM/bin/clang.exe", "clang"):
        if not cand:
            continue
        r = subprocess.run([cand, "--version"], capture_output=True, text=True, check=False)
        if r.returncode == 0:
            return r.stdout.splitlines()[0].strip() if r.stdout else "unavailable"
    return "unavailable"


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
                           "IQR rejection per protocol; process-level cold compiles",
        "toolchain": clang_version(),
    }


def one_compile(emit: str | None, out_dir: Path) -> float:
    """单次编译子进程,返回 wall 毫秒;失败即抛。"""
    cmd = [str(BIN), str(SRC)]
    if emit is None:
        cmd += ["-o", str(out_dir / "hello_world.exe")]
    else:
        cmd += [f"--emit={emit}"]
    t0 = time.perf_counter()
    r = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
    wall_ms = (time.perf_counter() - t0) * 1e3
    if r.returncode != 0:
        raise RuntimeError(f"rurixc 编译失败(exit {r.returncode}): {r.stdout}{r.stderr}")
    return wall_ms


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--bench", required=True, choices=list(BENCHES))
    ap.add_argument("--emit")
    ap.add_argument("--smoke", action="store_true")
    args = ap.parse_args()
    spec = BENCHES[args.bench]

    ensure_binary()
    warmup = SMOKE_WARMUP if args.smoke else WARMUP
    trials = SMOKE_TRIALS if args.smoke else TRIALS

    with tempfile.TemporaryDirectory() as td:
        out_dir = Path(td)
        for _ in range(warmup):
            one_compile(spec["emit"], out_dir)
        trial_medians = [one_compile(spec["emit"], out_dir) for _ in range(trials)]

    if args.smoke:
        best = min(trial_medians)
        print(f"[compile_bench] {args.bench} smoke PASS "
              f"({trials} compiles, best {best:.1f} ms, correctness ok)")
        return 0

    kept, rejected = iqr_filter(trial_medians)
    result_mean = trimmed_mean(trial_medians, TRIM)
    ci_lo, ci_hi = bootstrap_ci(kept if kept else trial_medians, statistic="median")

    doc = {
        "schema_version": 1,
        "evidence_level": "measured_local",
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "bench": {
            "id": spec["id"],
            "level": "compile",
            "problem_size": spec["problem_size"],
            "harness_commit": git_commit(),
        },
        "environment": collect_environment(),
        "sampling": {
            "warmup_iterations": warmup,
            "timed_iterations": 1,
            "trials": trials,
            "trimmed_pct": TRIM,
            "timer": "process_wall",
        },
        "results": {
            "metric": spec["metric"],
            "unit": "ms",
            "trial_medians": [round(v, 4) for v in trial_medians],
            "trimmed_mean": round(result_mean, 4),
            "cv": round(cv(trial_medians), 6),
            "ci95": [round(ci_lo, 4), round(ci_hi, 4)],
            "min": round(min(trial_medians), 4),
            "max": round(max(trial_medians), 4),
            "outliers_rejected_iqr": len(rejected),
            "correctness_check": "pass",
        },
        "notes": "process-level wall timing (perf_counter around rurixc subprocess); "
                 "each compile is a fresh process (inherent cold); single-iteration trials.",
    }
    if args.emit:
        emit_path = ROOT / args.emit
        emit_path.parent.mkdir(parents=True, exist_ok=True)
        emit_path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[compile_bench] evidence written: {args.emit} "
              f"({doc['results']['trimmed_mean']} ms, level={doc['evidence_level']})")
    else:
        print(json.dumps(doc["results"], ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
