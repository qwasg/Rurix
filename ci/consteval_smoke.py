# -*- coding: utf-8 -*-
"""const eval 冒烟(M3 CI_GATES §2 步骤 16,契约 G-M3-4)。

用法:
    py -3 ci/consteval_smoke.py compile-run   # 步骤 16:G-M3-4 通道

步骤 16:conformance/consteval/ 的 const 求值程序(const fn / const item 求值链)
经 rurixc 全管线产出 EXE → 运行核对退出码与预期输出(对齐步骤 12 真跑形态)。
const eval 任一环求值错误则常量值不符、输出偏离基线 → 红(CI_GATES §5 第 2 项)。
"""

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "build" / "ci_smoke"

# (源文件, 期待 stdout, 期待退出码)
CASES = [
    ("conformance/consteval/const_eval_run.rx", "consteval-ok", 0),
]


def fail(msg: str) -> None:
    print(f"[consteval_smoke] FAIL: {msg}")
    sys.exit(1)


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def compile_run() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    r = run(["cargo", "build", "-p", "rurixc", "--bin", "rurixc"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"cargo build rurixc 失败:\n{r.stderr}")
    rurixc = ROOT / "target" / "debug" / "rurixc.exe"
    for src, expect_out, expect_code in CASES:
        name = Path(src).stem
        exe = OUT_DIR / f"{name}.exe"
        r = run([str(rurixc), str(ROOT / src), "-o", str(exe)], cwd=ROOT)
        if r.returncode != 0:
            fail(f"rurixc 编译 {src} 失败(exit {r.returncode}):\n{r.stdout}{r.stderr}")
        if not exe.exists():
            fail(f"EXE 未产出: {exe}")
        r = run([str(exe)])
        if r.returncode != expect_code:
            fail(f"{name}.exe 退出码 {r.returncode}(期待 {expect_code})")
        if r.stdout.strip() != expect_out:
            fail(f"{name} stdout 不符: {r.stdout.strip()!r}(期待 {expect_out!r})")
        print(f"[consteval_smoke] {name}: PASS(exit {expect_code} / stdout {expect_out!r})")
    print("[consteval_smoke] compile-run PASS(const eval 全管线真跑,G-M3-4)")


def main() -> None:
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    if mode == "compile-run":
        compile_run()
    else:
        print("usage: py -3 ci/consteval_smoke.py compile-run")
        sys.exit(2)


if __name__ == "__main__":
    main()
