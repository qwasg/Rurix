#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-05 全包 `--emit=check` 双口径采纳判据取证器(EI1.5;RXS-0265;验收门 G-EI1-5)。

**操作者工具,不进 CI**(RXS-0265:「evidence 面**不进 CI 硬门**(计时波动,EA1 冷启动先例),
SKIP 不充绿」;镜像 ci/uc07_bench.py 双态先例)。产出两份 evidence,经
milestones/ei1/uc05_check_bench_evidence_schema.json 校验,回填 milestones/ei1/ei1_budget.json
的 `ei1.bench.uc05_check_cold_ms` / `ei1.bench.uc05_check_warm_ms`(阈 5000ms,direction=max),
由 ci/budget_eval.py **既有 `eval_entry` 通用路**(读 `results.trimmed_mean`)判读,零新 evaluator 分支。

两口径(RXS-0265 锁死措辞,**warm ≠ LSP 增量**):

  cold  `apps/uc05-rhi` 全包 `--emit=check` **冷全检**(含磁盘 `mod` 解析)。每 trial:
        把包源与 `rurixc.exe` 各拷到**全新临时路径**(该路径此前未被本会话触碰,消 path 级
        镜像/源文件缓存复用)→ **全新进程内唯一一次** check,无任何进程内预热 → 端到端墙钟。
        单 trial 恰好一次计时迭代(冷路按定义只发生一次),trial 值即该次测量。

  warm  **进程 / 缓存预热后的全包 `--emit=check` 重跑**——**诚实标注:全量重析,非 LSP 增量**。
        现 tooling session(`src/rurixc/src/tooling/session.rs::analyze`)只对单个内存文件
        lex + parse + check_crate、**无 `mod` 解析 / 磁盘加载**,无法「增量」检全包
        `apps/uc05-rhi`,故本口径**不用** didChange → publishDiagnostics 增量路(RFC-0014
        §9.1 I-EI1-IMPL-04 disposition,RXS-0265 锁)。每 trial:仓库原址同一 `rurixc.exe`
        先跑 WARMUP 次预热(BENCH_PROTOCOL §3 warmup ≥10),再跑 TIMED 次计时迭代取**中位数**
        (§3 trial 内统计 = 中位数)。

BENCH_PROTOCOL 三次运行规则(§3):上述为**一次 trial** 的内部协议;回填值 = **三次进程级独立
运行**各自 trial 值的 **trimmed mean(去头尾 20%)**;任一 trial 非 measured_local 则整组作废。

**与 m0 evidence_schema 的差异如实声明**(schema description 同步):本 bench 为**纯 host 编译器
墙钟**,全程**零 GPU 参与**——无 CUDA Event 内层 50×3 协议、无 L2 清缓存、无锁频(BENCH_PROTOCOL
§2.1 锁频规程与 unlocked 降级规则**对本 bench 不适用**,理由入 evidence
`environment.clock_lock_applicability` 字段,不静默省略)。

**诚实缺口(硬规则 3,schema 必填 `uncheckable_roots` by-construction 强制披露)**:
`apps/uc05-rhi/src/embed.rx` 为 `#[export(c)]` cdylib 根、**无 `main`**,而 `--emit=check`
的免 `main` 豁免只覆盖 `nvptx-ir` / `ptx` / `pyd` / `dll`(driver.rs `device_emit`;RXS-0252 的
导出根免 main 只落 `--emit=dll`)→ `rurixc embed.rx --emit=check` 报 **RX6002**。故「全包冷检」
**实测覆盖 demo.rx + graph.rx**(`mod graph` 磁盘解析真发生),**embed.rx 不可 check**;该事实由
本工具**真实探测**(非断言)写入 evidence `uncheckable_roots[].probe`,并入对照报告 §5。

用法:
  py -3 ci/uc05_check_bench.py cold|warm|both [--trials N] [--date YYYYMMDD]
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from bench.stats import trimmed_mean  # noqa: E402

PKG_SRC = ROOT / "apps" / "uc05-rhi" / "src"
# 全包 check 的**可 check 根**:demo.rx(经 `mod graph;` 磁盘解析连带 graph.rx)。
# embed.rx 见模块 docstring「诚实缺口」——无 main,--emit=check 报 RX6002,不可 check。
CHECK_ROOTS = ["demo.rx"]
COVERED_FILES = ["apps/uc05-rhi/src/demo.rx", "apps/uc05-rhi/src/graph.rx"]
UNCHECKABLE = ["embed.rx"]
# 编译器二进制取 **release**(rurixup 分发档 = release;采纳判据面向用户可得工具链)。
RURIXC = ROOT / "target" / "release" / ("rurixc.exe" if os.name == "nt" else "rurixc")

TRIALS_DEFAULT = 3
TRIM_PCT = 0.2
WARMUP = 10  # BENCH_PROTOCOL §3:warmup ≥10 次迭代
TIMED = 5  # BENCH_PROTOCOL §3:trial 内统计 = 中位数(warm 口径 5 次计时迭代)
EVIDENCE_DIR = ROOT / "evidence"


def die(msg: str) -> None:
    print(f"[uc05_check_bench] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def run_check(exe: Path, src_dir: Path) -> tuple[float, int, str]:
    """跑一遍全包 check(全部可 check 根),返回 (墙钟 ms, 末次 rc, 末次 stderr)。"""
    t0 = time.perf_counter()
    rc, errtxt = 0, ""
    for root in CHECK_ROOTS:
        r = subprocess.run(
            [str(exe), str(src_dir / root), "--emit=check"],
            cwd=str(src_dir),
            capture_output=True,
        )
        rc = r.returncode
        errtxt = r.stderr.decode("utf-8", "replace")
        if rc != 0:
            break
    return (time.perf_counter() - t0) * 1000.0, rc, errtxt


def probe_uncheckable() -> list[dict]:
    """**真实探测**(非断言)不可 check 根:跑 `--emit=check` 记录退出码与首行诊断。"""
    out: list[dict] = []
    for name in UNCHECKABLE:
        r = subprocess.run(
            [str(RURIXC), str(PKG_SRC / name), "--emit=check"],
            cwd=str(PKG_SRC),
            capture_output=True,
        )
        first = r.stderr.decode("utf-8", "replace").splitlines()
        out.append(
            {
                "file": f"apps/uc05-rhi/src/{name}",
                "reason": (
                    "`#[export(c)]` cdylib 根,无 host `main`;`--emit=check` 不在 driver.rs "
                    "`device_emit` 免 main 豁免集内(RXS-0252 导出根免 main 只覆盖 `--emit=dll`)"
                    " → RX6002。全包冷/热检因此**不覆盖本文件**(诚实缺口,不折算进 trimmed_mean)"
                ),
                "probe": {
                    "command": f"rurixc apps/uc05-rhi/src/{name} --emit=check",
                    "exit_code": r.returncode,
                    "stderr_first_line": first[0] if first else "",
                },
            }
        )
    return out


def cold_trial() -> float:
    """冷 trial:包源 + rurixc.exe 各拷全新临时路径 → 全新进程唯一一次 check(零预热)。"""
    td = Path(tempfile.mkdtemp(prefix="uc05_check_cold_"))
    try:
        src = td / "src"
        shutil.copytree(PKG_SRC, src)
        exe = td / RURIXC.name
        shutil.copy2(RURIXC, exe)
        ms, rc, errtxt = run_check(exe, src)
        if rc != 0:
            die(f"冷 trial check 非 0(rc={rc}):{errtxt[:400]}")
        return ms
    finally:
        shutil.rmtree(td, ignore_errors=True)


def warm_trial() -> float:
    """热 trial:仓库原址同一 exe,预热 WARMUP 次后取 TIMED 次计时迭代的中位数。"""
    for _ in range(WARMUP):
        _, rc, errtxt = run_check(RURIXC, PKG_SRC)
        if rc != 0:
            die(f"热 trial 预热 check 非 0(rc={rc}):{errtxt[:400]}")
    vals: list[float] = []
    for _ in range(TIMED):
        ms, rc, errtxt = run_check(RURIXC, PKG_SRC)
        if rc != 0:
            die(f"热 trial 计时 check 非 0(rc={rc}):{errtxt[:400]}")
        vals.append(ms)
    return statistics.median(vals)


def git(*args: str) -> str:
    r = subprocess.run(["git", *args], cwd=str(ROOT), capture_output=True)
    return r.stdout.decode("utf-8", "replace").strip()


def sha256(p: Path) -> str:
    h = hashlib.sha256()
    h.update(p.read_bytes())
    return h.hexdigest()


def environment() -> dict:
    return {
        "host_kind": "pure_host_compiler_wallclock",
        "gpu_involved": False,
        "clock_lock_applicability": (
            "n/a —— 本 bench 零 GPU 参与(`--emit=check` 不 codegen / 不 link / 不建 CUDA "
            "Context),BENCH_PROTOCOL §2.1 锁频规程与 unlocked 降级规则对本 bench 不适用;"
            "不因此降级 evidence_level,理由如实入档不静默省略"
        ),
        "os": platform.platform(),
        "os_build": platform.version(),
        "machine": platform.machine(),
        "processor": platform.processor(),
        "cpu_count_logical": os.cpu_count() or 0,
        "python": platform.python_version(),
        "rurixc_profile": "release",
        "rurixc_path": str(RURIXC.relative_to(ROOT)).replace("\\", "/"),
        "rurixc_sha256": sha256(RURIXC),
        "rurixc_size_bytes": RURIXC.stat().st_size,
        "harness_commit": git("rev-parse", "HEAD"),
        "worktree_dirty": bool(git("status", "--porcelain")),
    }


COLD_METHOD = (
    "冷全检:每 trial 把 apps/uc05-rhi/src 与 rurixc.exe 各拷到**全新临时路径**(消 path 级"
    "镜像与源文件缓存复用)→ **全新进程内唯一一次** `--emit=check`(零进程内预热,含 `mod graph` "
    "磁盘解析)→ 端到端墙钟。单 trial 恰一次计时迭代(冷路按定义只发生一次)。**控制到的**:"
    "进程创建 + PE 镜像装载 + 该路径文件首读 + 全量前端解析/检查;**未控制到的**:操作系统"
    "standby 缓存中刚写入的字节(Windows 无管理员级 drop-cache,如实声明不假装)。"
)
WARM_METHOD = (
    "预热后全包重跑(**全量重析,非 LSP 增量** —— RXS-0265 锁:现 tooling session 单文件、"
    "无 mod 解析/磁盘加载,无法增量检全包,故不走 didChange → publishDiagnostics 路):每 trial "
    "在仓库原址用同一 rurixc.exe 先跑 10 次预热(BENCH_PROTOCOL §3 warmup ≥10),再跑 5 次计时"
    "迭代取**中位数**(§3 trial 内统计 = 中位数)。冷/热差 = 进程启动 + 文件 IO + 镜像装载缓存,"
    "**非**编译器内部增量缓存(rurixc 无跨进程增量,07 §9)。"
)

SPECS = {
    "cold": {
        "bench_id": "uc05_check_cold",
        "entry_id": "ei1.bench.uc05_check_cold_ms",
        "metric": "package_check_cold_wallclock",
        "trial_fn": cold_trial,
        "method": COLD_METHOD,
        "timed_iters_per_trial": 1,
        "warmup_iters_per_trial": 0,
    },
    "warm": {
        "bench_id": "uc05_check_warm",
        "entry_id": "ei1.bench.uc05_check_warm_ms",
        "metric": "package_check_warm_wallclock",
        "trial_fn": warm_trial,
        "method": WARM_METHOD,
        "timed_iters_per_trial": TIMED,
        "warmup_iters_per_trial": WARMUP,
    },
}


def collect(kind: str, trials: int, date: str, uncheckable: list[dict]) -> Path:
    spec = SPECS[kind]
    print(f"[uc05_check_bench] {kind}: {trials} trials …")
    vals: list[float] = []
    for i in range(trials):
        v = spec["trial_fn"]()
        print(f"  trial {i + 1}/{trials}: {v:.3f} ms")
        vals.append(v)
    tm = trimmed_mean(vals, TRIM_PCT)
    doc = {
        "schema_version": 1,
        "evidence_level": "measured_local",
        "timestamp": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
        "bench": {
            "id": spec["bench_id"],
            "budget_entry": spec["entry_id"],
            "package": "apps/uc05-rhi",
            "emit": "check",
            "roots": [f"apps/uc05-rhi/src/{r}" for r in CHECK_ROOTS],
            "covered_files": COVERED_FILES,
            "clause": "RXS-0265",
        },
        "uncheckable_roots": uncheckable,
        "environment": environment(),
        "sampling": {
            "trials": trials,
            "trimmed_pct": TRIM_PCT,
            "timer": "wall_clock_process",
            "warmup_iters_per_trial": spec["warmup_iters_per_trial"],
            "timed_iters_per_trial": spec["timed_iters_per_trial"],
            "trial_statistic": "single_measurement" if kind == "cold" else "median",
            "method": spec["method"],
        },
        "results": {
            "metric": spec["metric"],
            "unit": "ms",
            "trimmed_mean": round(tm, 3),
            "trial_values": [round(v, 3) for v in vals],
            "min": round(min(vals), 3),
            "max": round(max(vals), 3),
        },
        "notes": (
            f"采纳判据 RXS-0265 阈 5000ms(direction=max)。本组 trimmed_mean {tm:.3f} ms。"
            "纯 host 编译器墙钟,零 GPU;evidence 面不进 CI 硬门(计时波动,EA1 冷启动先例)。"
        ),
    }
    out = EVIDENCE_DIR / f"{spec['bench_id']}_{date}.json"
    # 仓库 LF byte-exact:二进制写盘(禁 Python 文本模式)。
    out.write_bytes((json.dumps(doc, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))
    print(f"[uc05_check_bench] {kind}: trimmed_mean {tm:.3f} ms → {out.relative_to(ROOT)}")
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("kind", choices=["cold", "warm", "both"])
    ap.add_argument("--trials", type=int, default=TRIALS_DEFAULT)
    ap.add_argument("--date", default=datetime.date.today().strftime("%Y%m%d"))
    args = ap.parse_args()

    if not RURIXC.is_file():
        die(f"缺 {RURIXC.relative_to(ROOT)}(先跑 `cargo build --release -p rurixc`)")
    if not PKG_SRC.is_dir():
        die(f"缺 {PKG_SRC.relative_to(ROOT)}")

    uncheckable = probe_uncheckable()
    for u in uncheckable:
        print(
            f"[uc05_check_bench] 诚实缺口探测 {u['file']}: exit={u['probe']['exit_code']} "
            f"{u['probe']['stderr_first_line']}"
        )
    kinds = ["cold", "warm"] if args.kind == "both" else [args.kind]
    # 冷口径必须先跑(会话内任何 warm 迭代都会污染冷路的缓存前提)。
    for k in kinds:
        collect(k, args.trials, args.date, uncheckable)
    return 0


if __name__ == "__main__":
    sys.exit(main())
