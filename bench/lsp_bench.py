# -*- coding: utf-8 -*-
"""LSP 10k 行交互延迟 harness(M6.5,契约 G-M6-2;形态对标 bench/compile_bench.py)。

经 `rurixc --tooling-server` 常驻 query 层(单一前端,07 §9),在程序化生成的
~10k 行样例工程(bench/gen_lsp_workspace.py)上实测三类交互的**客户端墙钟延迟**
(主指标;instructions:u 在 Windows 记 unavailable 趋势参考):
  - completion            : textDocument/completion 请求往返;
  - definition            : textDocument/definition 请求往返;
  - publishDiagnostics    : textDocument/didChange(保存后全文重同步,07 §9 "全量 body
                            重查询")触发到收到 publishDiagnostics 帧的墙钟。

计时为客户端 perf_counter 包裹一次 JSON-RPC 往返;统计复用 BENCH_PROTOCOL §3
(trimmed-mean / trial 内中位数 / IQR / bootstrap CI,bench/stats.py)。廉价交互
(completion/definition,query memoized)用较多迭代;昂贵交互(publishDiagnostics
全量重分析)沿用 compile 风格下调迭代(trials≥3 满足 schema)。CPU 路径无 GPU 锁频,
clock_control 固定 not_applicable_cpu,measured_local 记级(真实硬件 + 三次进程级
独立运行 trimmed mean,见 bench/lsp_latency_triple.py;evidence/ 只增不删不改)。

经 `rx bench lsp [--smoke] [--emit PATH]` 编排(src/rx cmd_bench 泛分发,RD-003)。
"""
from __future__ import annotations

import argparse
import datetime
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import gen_lsp_workspace
from bench.stats import bootstrap_ci, cv, iqr_filter, trimmed_mean

ROOT = Path(__file__).resolve().parent.parent
RURIXC = ROOT / "target" / "debug" / ("rurixc.exe" if os.name == "nt" else "rurixc")
URI = "file:///lsp_latency_workspace.rx"
TRIM = 0.2

# 交互采样参数(廉价交互多迭代;昂贵的 publishDiagnostics 下调,trials≥3 满足 schema)
PARAMS = {
    "completion":         {"warmup": 10, "trials": 3, "timed": 30},
    "definition":         {"warmup": 10, "trials": 3, "timed": 30},
    "publishDiagnostics": {"warmup": 3,  "trials": 5, "timed": 3},
}
SMOKE_PARAMS = {
    "completion":         {"warmup": 1, "trials": 3, "timed": 2},
    "definition":         {"warmup": 1, "trials": 3, "timed": 2},
    "publishDiagnostics": {"warmup": 1, "trials": 3, "timed": 1},
}
SMOKE_LINES = 2_000


def git_commit() -> str:
    out = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT,
                         capture_output=True, text=True, check=False)
    return out.stdout.strip() or "unknown"


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
        "background_note": "desktop dev machine; LSP server is CPU path; background load "
                           "not isolated, IQR rejection per protocol; GPU-bench mutex queue "
                           "discipline reused to avoid interference",
    }


# ---- JSON-RPC over stdio 客户端 ----

def _write_message(stdin, msg: dict) -> None:
    body = json.dumps(msg, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii") + body)
    stdin.flush()


def _read_message(stdout) -> dict:
    # 服务器以 `writeln!` 写头(LF 终止),帧分隔为 `\n\n`;同时兼容 `\r\n\r\n`。
    headers = b""
    while not (headers.endswith(b"\n\n") or headers.endswith(b"\r\n\r\n")):
        ch = stdout.read(1)
        if not ch:
            raise EOFError("tooling-server closed stdout unexpectedly")
        headers += ch
    header_text = headers.decode("ascii", "replace")
    length = None
    for line in header_text.splitlines():
        if line.lower().startswith("content-length:"):
            length = int(line.split(":", 1)[1].strip())
            break
    if length is None:
        raise ValueError(f"frame missing Content-Length: {header_text!r}")
    body = b""
    while len(body) < length:
        chunk = stdout.read(length - len(body))
        if not chunk:
            raise EOFError("eof while reading frame body")
        body += chunk
    return json.loads(body.decode("utf-8"))


class LspClient:
    """单进程 rurixc --tooling-server 会话;请求往返计时(客户端墙钟 ms)。"""

    def __init__(self, base_text: str, anchors: dict) -> None:
        self.proc = subprocess.Popen(
            [str(RURIXC), "--tooling-server"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        self.base_text = base_text
        self.anchors = anchors
        self.version = 1
        self._toggle = False

    def initialize(self) -> None:
        self._request("init", "initialize", {})
        # didOpen → 服务器立即推一帧 publishDiagnostics,先排空
        _write_message(self.proc.stdin, {
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": URI, "version": self.version, "text": self.base_text}},
        })
        self._drain_until(lambda m: m.get("method") == "textDocument/publishDiagnostics")

    def _drain_until(self, pred) -> dict:
        while True:
            msg = _read_message(self.proc.stdout)
            if pred(msg):
                return msg

    def _request(self, msg_id: str, method: str, params: dict) -> tuple[float, dict]:
        params = {"jsonrpc": "2.0", "id": msg_id, "method": method, "params": params}
        t0 = time.perf_counter()
        _write_message(self.proc.stdin, params)
        resp = self._drain_until(lambda m: str(m.get("id")) == str(msg_id))
        return (time.perf_counter() - t0) * 1e3, resp

    def completion_once(self) -> tuple[float, bool]:
        ms, resp = self._request("c", "textDocument/completion", {
            "textDocument": {"uri": URI}, "position": self.anchors["completion"],
        })
        ok = isinstance(resp.get("result"), dict)
        return ms, ok

    def definition_once(self) -> tuple[float, bool]:
        ms, resp = self._request("d", "textDocument/definition", {
            "textDocument": {"uri": URI}, "position": self.anchors["definition"],
        })
        result = resp.get("result")
        ok = bool(result) and isinstance(result, dict) and "range" in result
        return ms, ok

    def diagnostics_once(self) -> tuple[float, bool]:
        # 保存后全文重同步:切换尾部注释强制文本变化 → 全量重分析。
        self._toggle = not self._toggle
        self.version += 1
        text = self.base_text + ("// save-tick\n" if self._toggle else "")
        msg = {"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": URI, "version": self.version},
            "contentChanges": [{"text": text}],
        }}
        t0 = time.perf_counter()
        _write_message(self.proc.stdin, msg)
        resp = self._drain_until(lambda m: m.get("method") == "textDocument/publishDiagnostics")
        ms = (time.perf_counter() - t0) * 1e3
        return ms, "params" in resp

    def close(self) -> None:
        try:
            self.proc.stdin.close()
        except OSError:
            pass
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.proc.kill()


def sample_interaction(once, warmup: int, trials: int, timed: int) -> dict:
    """warmup → trials×timed;trial 内中位数 → 跨 trial trimmed mean(BENCH_PROTOCOL §3)。"""
    for _ in range(warmup):
        once()
    trial_medians: list[float] = []
    all_samples: list[float] = []
    correctness = True
    for _ in range(trials):
        samples: list[float] = []
        for _ in range(timed):
            ms, ok = once()
            correctness = correctness and ok
            samples.append(ms)
        samples.sort()
        all_samples.extend(samples)
        trial_medians.append(samples[len(samples) // 2])
    kept, rejected = iqr_filter(all_samples)
    result_mean = trimmed_mean(trial_medians, TRIM)
    ci_lo, ci_hi = bootstrap_ci(kept if kept else all_samples, statistic="median")
    return {
        "warmup_iterations": warmup,
        "timed_iterations": timed,
        "trials": trials,
        "trimmed_mean": round(result_mean, 4),
        "trial_medians": [round(v, 4) for v in trial_medians],
        "cv": round(cv(all_samples), 6),
        "ci95": [round(ci_lo, 4), round(ci_hi, 4)],
        "min": round(min(all_samples), 4),
        "max": round(max(all_samples), 4),
        "outliers_rejected_iqr": len(rejected),
        "correctness_check": "pass" if correctness else "fail",
    }


def ensure_binary() -> None:
    print("[lsp_bench] cargo build -p rurixc --bin rurixc ...")
    subprocess.run(["cargo", "build", "-p", "rurixc", "--bin", "rurixc"], cwd=ROOT, check=True)
    if not RURIXC.is_file():
        raise FileNotFoundError(f"构建产物不存在: {RURIXC}")


def run(lines: int, smoke: bool) -> dict:
    ensure_binary()
    params = SMOKE_PARAMS if smoke else PARAMS
    ws = ROOT / "build" / "lsp_latency" / "workspace.rx"
    meta = gen_lsp_workspace.generate(lines, ws)
    base_text = ws.read_text(encoding="utf-8")

    client = LspClient(base_text, meta["anchors"])
    try:
        client.initialize()
        per_interaction = {
            "completion": sample_interaction(client.completion_once, **params["completion"]),
            "definition": sample_interaction(client.definition_once, **params["definition"]),
            "publishDiagnostics": sample_interaction(client.diagnostics_once, **params["publishDiagnostics"]),
        }
    finally:
        client.close()

    worst = max(v["trimmed_mean"] for v in per_interaction.values())
    correctness = "pass" if all(v["correctness_check"] == "pass" for v in per_interaction.values()) else "fail"
    return {
        "schema_version": 1,
        "evidence_level": "measured_local",
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "bench": {
            "id": "lsp_interaction_latency",
            "level": "lsp",
            "problem_size": f"generated {meta['lines']}-line workspace ({meta['n_functions']} fns)",
            "harness_commit": git_commit(),
            "sample_lines": meta["lines"],
            "generator": meta["generator"],
        },
        "environment": collect_environment(),
        "sampling": {
            "timer": "lsp_roundtrip_wall",
            "trimmed_pct": TRIM,
            "instructions_u": "unavailable (windows; wall-clock latency is primary metric, instructions:u trend reference only)",
        },
        "results": {
            "metric": "lsp_interaction_latency_ms",
            "unit": "ms",
            "trimmed_mean": round(worst, 4),
            "worst_interaction": max(per_interaction, key=lambda k: per_interaction[k]["trimmed_mean"]),
            "per_interaction": per_interaction,
            "correctness_check": correctness,
        },
        "notes": "client-side wall latency around one JSON-RPC round-trip via rurixc "
                 "--tooling-server; completion/definition are request-response, "
                 "publishDiagnostics measures didChange (full-text resync, 07 §9) to "
                 "publishDiagnostics frame; trimmed_mean = worst-case across the three "
                 "interactions (per_interaction holds each).",
    }


def main() -> int:
    ap = argparse.ArgumentParser(description="LSP 10k 行交互延迟 harness")
    ap.add_argument("--lines", type=int, default=None, help="样例行数(默认 10000;--smoke 默认 2000)")
    ap.add_argument("--smoke", action="store_true", help="冒烟:少迭代 + 小样例 + 正确性哨兵")
    ap.add_argument("--emit", help="写证据 JSON 到该路径(省略则打印 results)")
    args = ap.parse_args()

    lines = args.lines if args.lines is not None else (SMOKE_LINES if args.smoke else gen_lsp_workspace.DEFAULT_LINES)
    doc = run(lines, args.smoke)
    per = doc["results"]["per_interaction"]

    if args.smoke:
        if doc["results"]["correctness_check"] != "pass":
            bad = {k: v["correctness_check"] for k, v in per.items()}
            print(f"[lsp_bench] smoke FAIL: correctness {bad}")
            return 1
        summary = ", ".join(f"{k} {v['trimmed_mean']:.2f}ms" for k, v in per.items())
        print(f"[lsp_bench] smoke PASS ({lines} lines): {summary}")

    if args.emit:
        out = ROOT / args.emit
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[lsp_bench] evidence written: {args.emit} "
              f"(worst {doc['results']['trimmed_mean']} ms, level={doc['evidence_level']})")
    elif not args.smoke:
        print(json.dumps(doc["results"], ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
