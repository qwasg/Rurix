# -*- coding: utf-8 -*-
"""hello-world 编译闭环冒烟 + cdb 断点核对 + self-profile 核对(M2 CI_GATES §2 步骤 12/13/14)。

用法:
    py -3 ci/hello_smoke.py compile-run    # 步骤 12:G-M2-1 通道
    py -3 ci/hello_smoke.py breakpoint     # 步骤 13:G-M2-2 通道
    py -3 ci/hello_smoke.py self-profile   # 步骤 14:G-M2-4 通道(自 M2.4)

步骤 12:rurixc 全管线产出 EXE → 运行核对退出码/输出 → 同名 PDB 存在。
步骤 13:cdb 源行断点(bp `hello_world!hello_world.rx:6`)+ g + k,
        输出与基线不变量比对(命中行 + 栈顶帧;时间戳/地址等非确定字段不入基线)。
步骤 14:rurixc --self-profile 编译 hello-world → JSON 行可解析 +
        六阶段(parse/resolve/typeck/mir/codegen/link)在场且计数器全非零
        + total 行 memo 计数非零(契约 G-M2-4,D-235 二里程碑非零规则布点)。

工具定位:cdb 经 RURIXC_CDB 环境变量 > WinDbg appx > Windows Kits Debuggers。
"""

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "conformance" / "syntax" / "hello_world.rx"
OUT_DIR = ROOT / "build" / "ci_smoke"
EXE = OUT_DIR / "hello_world.exe"
PDB = OUT_DIR / "hello_world.pdb"
PROFILE = OUT_DIR / "hello_world.profile.json"
EXPECT_STDOUT = "hello, rurix"
# 步骤 13 基线不变量(确定性子集;地址/时间戳不入基线)
BP_BASELINE = [
    "Breakpoint 0 hit",
    "hello_world!main",
    "hello_world.rx @ 6",
]
# 步骤 14 基线不变量:六阶段集(M2_PLAN §4;计数器值非确定,只断言非零)
PROFILE_STAGES = ["parse", "resolve", "typeck", "mir", "codegen", "link"]


def fail(msg: str) -> None:
    print(f"[hello_smoke] FAIL: {msg}")
    sys.exit(1)


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def build_exe(self_profile: bool = False) -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    r = run(["cargo", "build", "-p", "rurixc", "--bin", "rurixc"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"cargo build rurixc 失败:\n{r.stderr}")
    rurixc = ROOT / "target" / "debug" / "rurixc.exe"
    cmd = [str(rurixc), str(SRC), "-o", str(EXE)]
    if self_profile:
        cmd.append(f"--self-profile={PROFILE}")
    r = run(cmd, cwd=ROOT)
    if r.returncode != 0:
        fail(f"rurixc 编译 hello_world.rx 失败(exit {r.returncode}):\n{r.stdout}{r.stderr}")


def compile_run() -> None:
    build_exe()
    if not EXE.exists():
        fail(f"EXE 未产出: {EXE}")
    r = run([str(EXE)])
    if r.returncode != 0:
        fail(f"hello_world.exe 退出码 {r.returncode}(期待 0)")
    if r.stdout.strip() != EXPECT_STDOUT:
        fail(f"stdout 不符: {r.stdout.strip()!r}(期待 {EXPECT_STDOUT!r})")
    if not PDB.exists():
        fail(f"PDB 未产出: {PDB}(G-M2-1 要求同名 .pdb)")
    print(f"[hello_smoke] compile-run PASS(exit 0 / stdout 符合 / {PDB.name} 存在)")


def locate_cdb() -> str:
    env = os.environ.get("RURIXC_CDB")
    if env and Path(env).exists():
        return env
    # WinDbg appx(winget Microsoft.WinDbg)
    r = run(
        [
            "powershell",
            "-NoProfile",
            "-Command",
            "(Get-AppxPackage Microsoft.WinDbg).InstallLocation",
        ]
    )
    loc = r.stdout.strip()
    if loc:
        cdb = Path(loc) / "amd64" / "cdb.exe"
        if cdb.exists():
            return str(cdb)
    # 经典 Debugging Tools for Windows
    classic = Path(
        "C:/Program Files (x86)/Windows Kits/10/Debuggers/x64/cdb.exe"
    )
    if classic.exists():
        return str(classic)
    fail("cdb.exe 未找到(装 WinDbg 或设 RURIXC_CDB)")
    raise AssertionError  # unreachable


def breakpoint_check() -> None:
    if not EXE.exists() or not PDB.exists():
        build_exe()
    cdb = locate_cdb()
    script = "bp `hello_world!hello_world.rx:6`; g; k; q"
    r = run([cdb, "-y", str(OUT_DIR), "-lines", "-c", script, str(EXE)], timeout=120)
    out = r.stdout + r.stderr
    missing = [m for m in BP_BASELINE if m not in out]
    if missing:
        log = OUT_DIR / "cdb_breakpoint.log"
        log.write_text(out, encoding="utf-8")
        fail(f"cdb 输出缺基线不变量 {missing}(全文见 {log})")
    print("[hello_smoke] breakpoint PASS(源行断点命中 + main 栈帧 @ hello_world.rx:6)")


def self_profile_check() -> None:
    if PROFILE.exists():
        PROFILE.unlink()  # 防陈旧产物掩盖本次失败
    build_exe(self_profile=True)
    if not PROFILE.exists():
        fail(f"self-profile 输出未产出: {PROFILE}")
    records: dict[str, dict] = {}
    for lineno, line in enumerate(PROFILE.read_text(encoding="utf-8").splitlines(), 1):
        try:
            rec = json.loads(line)
        except json.JSONDecodeError as e:
            fail(f"self-profile 第 {lineno} 行非合法 JSON(G-M2-4 机器可解析判据): {e}\n{line}")
        if not rec.get("stage") or not isinstance(rec.get("wall_ms"), (int, float)):
            fail(f"self-profile 第 {lineno} 行缺 stage/wall_ms 字段:\n{line}")
        records[rec["stage"]] = rec
    missing = [s for s in PROFILE_STAGES if s not in records]
    if missing:
        fail(f"self-profile 缺阶段 {missing}(要求全集 {PROFILE_STAGES})")
    zeros = [
        f"{stage}.{k}"
        for stage in PROFILE_STAGES + ["total"]
        for k, v in records.get(stage, {}).get("counters", {}).items()
        if not v
    ]
    if "total" not in records:
        fail("self-profile 缺 total 行(memo 汇总)")
    if not records["total"]["counters"]:
        fail("total 行计数器为空(期待 memo_hits/memo_misses)")
    if zeros:
        fail(f"计数器为零: {zeros}(契约 G-M2-4 非零判据,D-235)")
    counts = {s: records[s]["counters"] for s in PROFILE_STAGES}
    print(f"[hello_smoke] self-profile PASS(JSON 行可解析 / 六阶段计数器非零: {counts})")


def main() -> None:
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    if mode == "compile-run":
        compile_run()
    elif mode == "breakpoint":
        breakpoint_check()
    elif mode == "self-profile":
        self_profile_check()
    else:
        print("usage: py -3 ci/hello_smoke.py {compile-run|breakpoint|self-profile}")
        sys.exit(2)


if __name__ == "__main__":
    main()
