# -*- coding: utf-8 -*-
"""包管理离线解析 + lock + 内容树 digest 冒烟门(M6 CI_GATES 步骤 27a,契约 D-M6-2)。

用法:
    py -3 ci/pkg_resolve_smoke.py

机制(CPU-only,无 codegen,无网络;反 YAML-only):把 conformance/pkg 样例
workspace 拷到临时目录,经 rx 端到端验证包管理子系统(spec/toolchain.md
RXS-0089~0094):
- rx vendor --offline            → 0,写 rurix.lock + vendor/<dep>(内容树 SHA-256)
- rx vendor --locked --offline   → 0(lock 一致 + vendor digest 一致)
- 二次 vendor 的 rurix.lock 逐字节稳定(RXS-0092 确定性序列化)
- 篡改 vendor 内容 → rx vendor --locked 红(RX7008)→ 复原转绿
- 篡改 rurix.lock  → rx vendor --locked 红(RX7007)→ 复原转绿
- path 源缺失 → rx vendor 红(RX7009)

任一断言不成立(应绿却红 / 应红却绿 / 错误码不符)即整体 FAIL(非零退出)。
这使门在真实 PR 上具备真实红绿路径:包管理逻辑被破坏时 CI 必红(D11.8-2)。
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RX = ROOT / "target" / "debug" / ("rx.exe" if os.name == "nt" else "rx")
SAMPLE = ROOT / "conformance" / "pkg"

FAILURES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def run_vendor(ws: Path, *extra: str) -> subprocess.CompletedProcess:
    argv = [str(RX), "vendor", "--manifest-path", str(ws / "rurix.toml"), "--offline", *extra]
    return subprocess.run(argv, capture_output=True, text=True)


def fresh_copy(dst: Path) -> Path:
    ws = dst / "pkg"
    shutil.copytree(SAMPLE, ws)
    return ws


def main() -> int:
    build = subprocess.run(
        ["cargo", "build", "-p", "rx"], cwd=ROOT, capture_output=True, text=True
    )
    if build.returncode != 0:
        print(f"[pkg_resolve_smoke] FAIL: cargo build -p rx 失败:\n{build.stderr}")
        return 1
    if not RX.is_file():
        print(f"[pkg_resolve_smoke] FAIL: rx 产物不存在: {RX}")
        return 1

    with tempfile.TemporaryDirectory(prefix="rurix_pkg_smoke_") as tmp:
        tmp = Path(tmp)
        ws = fresh_copy(tmp)

        # 1) 离线 vendor → 写 lock + vendor(RXS-0094)。
        proc = run_vendor(ws)
        check(proc.returncode == 0, f"vendor 应成功,实得 {proc.returncode}: {proc.stderr.strip()}")
        check((ws / "rurix.lock").is_file(), "应生成 rurix.lock")
        check((ws / "vendor" / "foo" / "rurix.toml").is_file(), "应 vendor foo")
        check((ws / "vendor" / "util" / "rurix.toml").is_file(), "应 vendor util(菱形单根锁)")

        # 关键前置(离线解析 + vendor 落盘)未成立则提前清晰报错,不进入后续红绿子步骤
        # (例如样例清单被破坏 → rx vendor 失败 → vendor 产物缺失)。
        if FAILURES or not (ws / "vendor" / "foo" / "src" / "lib.rx").is_file():
            print(f"[pkg_resolve_smoke] FAIL ({len(FAILURES)})")
            for f in FAILURES:
                print(f"  - {f}")
            if not (ws / "vendor" / "foo" / "src" / "lib.rx").is_file():
                print("  - vendor 前置失败:vendor/foo/src/lib.rx 未生成(rx vendor 解析未成功)")
            return 1

        # 2) 逐字节确定性(RXS-0092):再 vendor 一次,lock 字节不变。
        lock1 = (ws / "rurix.lock").read_bytes() if (ws / "rurix.lock").is_file() else b""
        run_vendor(ws)
        lock2 = (ws / "rurix.lock").read_bytes() if (ws / "rurix.lock").is_file() else b""
        check(lock1 == lock2 and lock1 != b"", "rurix.lock 二次生成应逐字节稳定(RXS-0092)")

        # 3) locked 校验绿(RXS-0094)。
        proc = run_vendor(ws, "--locked")
        check(proc.returncode == 0, f"locked 校验应绿,实得 {proc.returncode}: {proc.stderr.strip()}")

        # 4) 篡改 vendor 内容 → RX7008 红 → 复原绿(反 YAML-only)。
        #    用二进制读写避免 Windows 文本模式 \n→\r\n 翻译扰动内容树哈希。
        tampered_file = ws / "vendor" / "foo" / "src" / "lib.rx"
        original = tampered_file.read_bytes()
        tampered_file.write_bytes(original + b"// tampered\n")
        red = run_vendor(ws, "--locked")
        check(red.returncode != 0, "篡改 vendor 内容应红(RX7008)")
        check("RX7008" in red.stderr, f"应携带 RX7008,实得 stderr: {red.stderr.strip()[:200]}")
        tampered_file.write_bytes(original)
        green = run_vendor(ws, "--locked")
        check(green.returncode == 0, f"复原 vendor 后应转绿,实得 {green.returncode}: {green.stderr.strip()[:200]}")

        # 5) 篡改 rurix.lock → RX7007 红 → 复原绿。
        lock_path = ws / "rurix.lock"
        lock_bytes = lock_path.read_bytes()
        lock_path.write_bytes(lock_bytes.replace(b'content_sha256 = "', b'content_sha256 = "0000', 1))
        red2 = run_vendor(ws, "--locked")
        check(red2.returncode != 0, "篡改 rurix.lock 应红(RX7007)")
        check("RX7007" in red2.stderr, f"应携带 RX7007,实得 stderr: {red2.stderr.strip()[:200]}")
        lock_path.write_bytes(lock_bytes)
        green2 = run_vendor(ws, "--locked")
        check(green2.returncode == 0, f"复原 lock 后应转绿,实得 {green2.returncode}: {green2.stderr.strip()[:200]}")

        # 6) path 源缺失 → RX7009 红。
        ws2 = tmp / "pkg_missing"
        ws2.mkdir()
        (ws2 / "src").mkdir()
        (ws2 / "rurix.toml").write_text(
            '[package]\nname = "app"\nversion = "0.1.0"\n[dependencies]\ngone = { path = "gone" }\n',
            encoding="utf-8",
        )
        (ws2 / "src" / "main.rx").write_text("fn main() {}\n", encoding="utf-8")
        miss = subprocess.run(
            [str(RX), "vendor", "--manifest-path", str(ws2 / "rurix.toml"), "--offline"],
            capture_output=True,
            text=True,
        )
        check(miss.returncode != 0, "path 源缺失应红(RX7009)")
        check("RX7009" in miss.stderr, f"应携带 RX7009,实得 stderr: {miss.stderr.strip()[:200]}")

    if FAILURES:
        print(f"[pkg_resolve_smoke] FAIL ({len(FAILURES)})")
        for f in FAILURES:
            print(f"  - {f}")
        return 1
    print("[pkg_resolve_smoke] PASS (离线解析 + lock 确定性 + 内容树 digest 红绿 + RX7007/7008/7009)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
