"""G-M1-5 / G-M6-4 判据脚本:rx fmt 在语法样例集上幂等(fmt(fmt(x)) == fmt(x),字节级)。

用法:py -3 ci/check_fmt_idempotent.py
机制:构建 release rx(M6.1 收编 RD-005:雏形 rx_fmt 退役,幂等门路由到 rx fmt),
对 conformance/syntax/**/*.rx 逐文件跑 `rx fmt --check-idempotent`(rx 子命令内
完成两次 fmt 与字节比较),任一失败即 FAIL。

收编(RXS-0087):rx fmt 复用 rurixc::fmt::format_source 单一事实源;tests/ui 含
词法错误样例(format 源不洁)不入幂等门语料,语料维持 well-formed 的 conformance/syntax。
"""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORPUS_DIR = ROOT / "conformance" / "syntax"
BIN = ROOT / "target" / "release" / ("rx.exe" if os.name == "nt" else "rx")


def main() -> int:
    subprocess.run(
        ["cargo", "build", "--release", "-p", "rx"],
        cwd=ROOT, check=True, capture_output=True,
    )
    if not BIN.is_file():
        print(f"[check_fmt_idempotent] FAIL: 构建产物不存在: {BIN}")
        return 1
    files = sorted(CORPUS_DIR.glob("**/*.rx"))
    if len(files) < 100:
        print(f"[check_fmt_idempotent] FAIL: 语料过小({len(files)} 个)")
        return 1
    failures = []
    for f in files:
        proc = subprocess.run(
            [str(BIN), "fmt", "--check-idempotent", str(f)],
            capture_output=True, text=True, check=False,
        )
        if proc.returncode != 0:
            failures.append(f"{f.relative_to(ROOT)}: {proc.stderr.strip()}")
    if failures:
        print(f"[check_fmt_idempotent] FAIL ({len(failures)}/{len(files)})")
        for line in failures:
            print(f"  - {line}")
        return 1
    print(f"[check_fmt_idempotent] PASS ({len(files)} files, rx fmt(fmt(x)) == fmt(x) byte-exact)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
