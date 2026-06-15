# -*- coding: utf-8 -*-
"""rx CLI 核心子命令端到端冒烟(M6 CI_GATES 步骤 25,契约 G-M6-3)。

用法:
    py -3 ci/rx_cli_smoke.py            # 真跑 build/run/check/fmt/bench --smoke
    py -3 ci/rx_cli_smoke.py --no-emit  # 同上但不写 evidence(本地快速核对)

机制:构建 debug rx,在样例工程上逐子命令端到端真跑(退出码符合 RXS-0083 约定):
- rx check conformance/toolchain/check_ok.rx        → 0(仅前端,RXS-0086)
- rx build conformance/toolchain/hello.rx -o ...     → 0 + EXE 落盘(RXS-0084)
- rx run   conformance/toolchain/exit_code.rx -o ... → 0(产物退出码透传,RXS-0085)
- rx fmt --check-idempotent conformance/syntax/...   → 0(收编 RD-005,RXS-0087)
- rx bench saxpy --smoke                              → 0(收编 RD-003,RXS-0088,GPU)

任一子命令端到端失败(非零退出)即整体 FAIL(反 YAML-only)。成功子命令去重集
写 evidence/rx_cli_smoke_<yyyymmdd>.json(subcommands_passed),计入
m6.counter.rx_cli_core_subcommands(ci/budget_eval.py)。
"""
from __future__ import annotations

import datetime
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RX = ROOT / "target" / "debug" / ("rx.exe" if os.name == "nt" else "rx")
OUT_DIR = ROOT / "build" / "rx_cli_smoke"


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def main() -> int:
    emit = "--no-emit" not in sys.argv
    build = subprocess.run(
        ["cargo", "build", "-p", "rx"], cwd=ROOT, capture_output=True, text=True
    )
    if build.returncode != 0:
        print(f"[rx_cli_smoke] FAIL: cargo build -p rx 失败:\n{build.stderr}")
        return 1
    if not RX.is_file():
        print(f"[rx_cli_smoke] FAIL: rx 产物不存在: {RX}")
        return 1
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    # 子命令端到端用例:(子命令名, 命令 argv, 期望退出码, 是否需要 GPU/工具链)
    cases = [
        ("check", [str(RX), "check", "conformance/toolchain/check_ok.rx"], 0),
        ("build", [str(RX), "build", "conformance/toolchain/hello.rx",
                   "-o", str(OUT_DIR / "hello.exe")], 0),
        ("run", [str(RX), "run", "conformance/toolchain/exit_code.rx",
                 "-o", str(OUT_DIR / "exit_code.exe")], 0),
        ("fmt", [str(RX), "fmt", "--check-idempotent",
                 "conformance/syntax/hello_world.rx"], 0),
        ("bench", [str(RX), "bench", "saxpy", "--smoke"], 0),
    ]

    passed: list[str] = []
    facts: list[dict] = []
    failures: list[str] = []
    for name, argv, want in cases:
        proc = run(argv)
        ok = proc.returncode == want
        facts.append({
            "subcommand": name,
            "command": " ".join(argv[1:]) if argv[0] == str(RX) else " ".join(argv),
            "exit_code": proc.returncode,
            "note": "PASS" if ok else proc.stderr.strip()[:300],
        })
        if ok:
            passed.append(name)
        else:
            failures.append(
                f"rx {name}: 期望退出 {want} 实际 {proc.returncode}\n"
                f"  stdout: {proc.stdout.strip()[:200]}\n  stderr: {proc.stderr.strip()[:200]}"
            )

    if emit:
        doc = {
            "schema_version": 1,
            "subject": "rx_cli_core_subcommands",
            "subcommands_passed": sorted(set(passed)),
            "rx_binary": str(RX.relative_to(ROOT)).replace("\\", "/"),
            "facts": facts,
            "timestamp": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
        }
        out = ROOT / "evidence" / f"rx_cli_smoke_{datetime.date.today():%Y%m%d}.json"
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[rx_cli_smoke] evidence 写入 {out.relative_to(ROOT)}")

    if failures:
        print(f"[rx_cli_smoke] FAIL ({len(failures)}/{len(cases)})")
        for line in failures:
            print(f"  - {line}")
        return 1
    print(f"[rx_cli_smoke] PASS ({len(passed)}/{len(cases)} 子命令端到端真跑: {', '.join(passed)})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
