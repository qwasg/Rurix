#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""edition 机制 + stable API 快照冻结冒烟(G2.5 CI_GATES 步骤 49,RFC-0008;G-G2-5,
纯 host/CPU-only,check_ 守卫风格,反 YAML-only)。

edition 为编译期/host 工具链面,**无 device**(不得 SKIP 充绿)。本冒烟做**真实红绿**:

- **green(edition 解析/校验)**:`cargo test -p rurix-pkg --test edition_corpus` ——
  conformance/edition/accept/*.toml 合法 edition 经 Manifest::parse 接受;
  conformance/edition/reject/*.toml 未知 edition → RX7020 / 类型错误 → RX7005
  strict-only 拒(corpus 测试内部断言期望码,reject 即 red 被正确拦截)。
- **green(stable 快照匹配)**:`py -3 ci/stable_snapshot.py --check` —— 当前 stable 面
  与入库 tests/stable/stable_api.snapshot 一致(RD-008 激活机制)。
- **red→green 闭合(反 YAML-only)**:篡改 tests/stable/stable_api.snapshot 一字 →
  `--check` 翻红(漂移检出)→ 复原原始字节 → `--check` 复绿(红绿闭合)。

证明门真在校验 edition 解析与 stable 面、能区分「一致 vs 漂移」,而非空过。
🔒 不冻结二进制 ABI(RXS-0180 L3);本冒烟纯 host 编译期,无 device/GPU、不 SKIP。
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SNAPSHOT = ROOT / "tests" / "stable" / "stable_api.snapshot"
STABLE_SMOKE = ROOT / "ci" / "stable_snapshot.py"


def fail(msg):
    print(f"[edition] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def edition_corpus_green():
    """green + reject red:edition conformance corpus(accept 解析 OK / reject 拦截)。"""
    print("[edition] cargo test -p rurix-pkg --test edition_corpus")
    r = run(["cargo", "test", "-p", "rurix-pkg", "--test", "edition_corpus", "--quiet"])
    if r.returncode != 0:
        fail(f"edition conformance corpus 未绿(accept/reject 断言失败):\n{r.stdout}\n{r.stderr}")
    print("[edition] OK edition_corpus (accept 解析 OK + reject RX7020/RX7005 strict-only 拦截)")


def edition_unit_green():
    """edition 解析/校验单测(RXS-0177~0180 锚定 + RX7020 红绿)。"""
    print("[edition] cargo test -p rurix-pkg manifest::tests::edition")
    r = run(["cargo", "test", "-p", "rurix-pkg", "manifest::tests::edition", "--quiet"])
    if r.returncode != 0:
        fail(f"edition 解析/校验单测未绿:\n{r.stdout}\n{r.stderr}")
    print("[edition] OK edition unit tests (RXS-0177~0180)")


def stable_check_green():
    """stable 快照匹配(RD-008 激活机制)。"""
    print("[edition] py -3 ci/stable_snapshot.py --check")
    r = run([sys.executable, str(STABLE_SMOKE), "--check"])
    if r.returncode != 0:
        fail(f"stable 快照与入库不一致(应先 RURIX_BLESS=1 重 bless):\n{r.stdout}\n{r.stderr}")
    print("[edition] OK stable snapshot --check (stable 面与入库快照一致)")


def stable_tamper_redgreen():
    """red→green 闭合(反 YAML-only):篡改 stable 快照 → 红 → 复原 → 绿。"""
    if not SNAPSHOT.is_file():
        fail(f"缺 {SNAPSHOT.relative_to(ROOT)}(首份快照未 bless)")
    original = SNAPSHOT.read_bytes()
    try:
        # 篡改:在 edition_anchor 值注入伪 edition,使重算 ≠ 入库 → 必红。
        tampered = original.replace(b'"edition_anchor": "2026"', b'"edition_anchor": "9999"')
        if tampered == original:
            fail("篡改无效(快照缺 edition_anchor 字段,无法构造红路径)")
        SNAPSHOT.write_bytes(tampered)
        r = run([sys.executable, str(STABLE_SMOKE), "--check"])
        if r.returncode == 0:
            fail("red 路径失败:篡改 stable 快照后 --check 仍绿(门空过,反 YAML-only)")
        print("[edition] OK red (篡改 stable 快照 → --check 翻红)")
    finally:
        SNAPSHOT.write_bytes(original)
    # 复原后必绿(红绿闭合)。
    r = run([sys.executable, str(STABLE_SMOKE), "--check"])
    if r.returncode != 0:
        fail(f"复原后未复绿(红绿闭合失败):\n{r.stdout}\n{r.stderr}")
    print("[edition] OK green-restored (复原 stable 快照 → --check 复绿,红绿闭合)")


def main():
    edition_corpus_green()
    edition_unit_green()
    stable_check_green()
    stable_tamper_redgreen()
    print(
        "[edition] PASS (edition 解析/校验真实红绿:accept 解析 OK + 未知 edition RX7020 / "
        "类型错误 RX7005 strict-only 拦截;stable API 快照 RD-008 激活:匹配 + 篡改红绿闭合)"
    )
    sys.exit(0)


if __name__ == "__main__":
    main()
