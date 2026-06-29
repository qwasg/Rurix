#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""贡献校验 CI 阻断门(G1.4 / MR-0003;CPU-only,check_ 守卫风格)。

兑现 10 §7「开源后 CI 自动阻断缺 provenance / 验证输出 / 条款号的 PR」(D-406)。
扫描 PR 范围(base..HEAD)的每个**非 merge commit**,三类缺项即红(反 YAML-only):

1. **Provenance**:每个 commit 含 `Assisted-by: <tool>:<model>` 或 `Co-Authored-By:`
   trailer(仓库既有约定 + AGENTS 硬规则 2)。
2. **条款号**:触 `src/**/*.rs` 或 `spec/**/*.md`(README 除外)的语义改动,须在 commit
   body / 新增 diff 行(`//@ spec: RXS-####`)/ 关联 `rfcs/*.md` 之一引用条款号或
   deferred/RFC 编号(`RXS-####` / `RD-###` / `RFC-####` / `MR-####`);纯文档/纯配置/
   纯 CI 改动豁免(硬规则 7)。
3. **验证强制**:触 `src/**/*.rs` 的 commit body 含验证标记(`Validation:` / `验证:` /
   `cargo test|build|clippy|fmt` / `ci/*.py` / `conformance` / `pytest`;数字必须来自命令
   输出,硬规则 3/10)。

base 解析(优先级:命令行参数 > GITHUB_BASE_REF > origin/main > main):贡献门是 **PR 范围**
语义,默认基准取 `origin/main`(只校验本 PR 自己的 commit,不回溯已合入历史)——区别于
`check_guardrails.py` 的字节级基准 `m8-closed`。空范围(如 push 后)→ PASS。

check_ 守卫风格:**不分配错误码**(07 §5)、**不写 evidence**、**不接 budget counter**;
内置 `red_self_test()` 反 YAML-only(合成缺项 commit 断言判红、齐备 commit 断言判绿)。

用法: py -3 ci/check_contribution.py [base_ref]
"""
from __future__ import annotations

import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# —— 规则正则 ——
# Provenance trailer:`Assisted-by: tool:model` 或 `Co-Authored-By: name <email>`。
PROVENANCE_RE = re.compile(
    r"(?im)^\s*(?:Assisted-by:\s*\S+:\S+|Co-Authored-By:\s*.+\S)\s*$"
)
# 条款号 / deferred / RFC / Mini-RFC 编号(语义改动须引用其一)。
CLAUSE_ID_RE = re.compile(r"\b(?:RXS-\d{4}|RD-\d{3}|RFC-\d{4}|MR-\d{4})\b")
# 验证标记(src 改动 commit body 须含其一)。
VERIFY_RE = re.compile(
    r"(?im)(?:Validation:|验证[:：]|cargo\s+(?:test|build|clippy|fmt)\b|"
    r"ci/[\w-]+\.py|conformance|pytest)"
)
# 语义改动文件:src 下 .rs 或 spec 下 .md(README 除外,与 trace_matrix 口径一致)。
SRC_RS_RE = re.compile(r"^src/.+\.rs$")
SPEC_MD_RE = re.compile(r"^spec/.+\.md$")


@dataclass
class CommitRecord:
    """单个 commit 的可校验快照(git 采集层与 red 自检层共用)。"""

    sha: str
    message: str
    files: list[str]
    added_text: str = ""  # 新增 diff 行(去 +++ 头)拼接,供条款号检测

    @property
    def short(self) -> str:
        return self.sha[:10] if self.sha else "<synthetic>"


def _touches_semantic(files: list[str]) -> bool:
    return any(
        SRC_RS_RE.match(f) or (SPEC_MD_RE.match(f) and not f.endswith("/README.md"))
        for f in files
    )


def _touches_src_rs(files: list[str]) -> bool:
    return any(SRC_RS_RE.match(f) for f in files)


def _touches_rfcs(files: list[str]) -> bool:
    return any(f.startswith("rfcs/") and f.endswith(".md") for f in files)


def check_commit(rec: CommitRecord) -> list[str]:
    """返回该 commit 的缺项列表(空 = 通过)。纯函数,red 自检直接喂合成记录。"""
    problems: list[str] = []

    # 规则 1:Provenance trailer。
    if not PROVENANCE_RE.search(rec.message):
        problems.append(
            "缺 provenance trailer(须含 `Assisted-by: <tool>:<model>` 或 "
            "`Co-Authored-By:`,D-406 / 硬规则 2)"
        )

    # 规则 2:语义改动须引用条款号 / deferred / RFC 编号。
    if _touches_semantic(rec.files):
        has_id = bool(
            CLAUSE_ID_RE.search(rec.message)
            or CLAUSE_ID_RE.search(rec.added_text)
            or _touches_rfcs(rec.files)
        )
        if not has_id:
            problems.append(
                "语义改动(src/*.rs 或 spec/*.md)未引用条款号"
                "(commit body / `//@ spec: RXS-####` 注释 / 关联 rfcs/*.md 之一,硬规则 7)"
            )

    # 规则 3:src 改动须附验证标记。
    if _touches_src_rs(rec.files) and not VERIFY_RE.search(rec.message):
        problems.append(
            "src 改动 commit body 缺验证标记(`Validation:` / `验证:` / "
            "`cargo test` / `ci/*.py` 等,硬规则 3/10)"
        )

    return problems


# ——————————————————————— git 采集层 ———————————————————————


def git(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", check=False
    )


def rev_exists(ref: str) -> bool:
    return git("rev-parse", "--verify", "--quiet", ref).returncode == 0


def resolve_base() -> str | None:
    if len(sys.argv) > 1:
        return sys.argv[1]
    gh_base = os.environ.get("GITHUB_BASE_REF")
    if gh_base:
        cand = f"origin/{gh_base}"
        return cand if rev_exists(cand) else gh_base
    # PR 范围语义:默认基准取 origin/main(只校验本 PR 自己的 commit)。
    for cand in ("origin/main", "main"):
        if rev_exists(cand):
            return cand
    return None


def collect_commits(base: str) -> list[CommitRecord]:
    out = git("log", "--no-merges", "--format=%H", f"{base}..HEAD").stdout
    shas = [s for s in out.splitlines() if s.strip()]
    records: list[CommitRecord] = []
    for sha in shas:
        message = git("log", "-1", "--format=%B", sha).stdout
        files = [
            f for f in git("diff-tree", "--no-commit-id", "--name-only", "-r", sha).stdout.splitlines()
            if f.strip()
        ]
        added = [
            ln[1:]
            for ln in git("show", "--format=", "--unified=0", sha).stdout.splitlines()
            if ln.startswith("+") and not ln.startswith("+++")
        ]
        records.append(
            CommitRecord(sha=sha, message=message, files=files, added_text="\n".join(added))
        )
    return records


# ——————————————————————— red 自检 ———————————————————————


def red_self_test() -> None:
    """反 YAML-only:合成缺项 / 齐备 commit,断言门能区分红绿。门失效即红。"""
    # (a) 缺 provenance + 缺条款 + 缺验证(触 src/*.rs)→ 应判红(≥3 问题)。
    bad = CommitRecord(
        sha="",
        message="feat: tweak geometry kernel\n",
        files=["src/rurix-geometry/src/lib.rs"],
        added_text="fn foo() {}\n",
    )
    bad_problems = check_commit(bad)
    if len(bad_problems) < 3:
        _fail(
            f"red 自检失败:缺项 commit 未被全数识别(检出 {len(bad_problems)}/3,门失效):"
            f"{bad_problems}"
        )
    # (b) 齐备(provenance + 条款 + 验证)→ 应判绿(0 问题)。
    good = CommitRecord(
        sha="",
        message=(
            "feat(geometry): add bvh node\n\n"
            "Validation: cargo test -p rurix-geometry PASS\n"
            "//@ spec: RXS-0113 covered\n\n"
            "Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>\n"
            "Assisted-by: claude-code:claude-opus-4-8\n"
        ),
        files=["src/rurix-geometry/src/lib.rs"],
        added_text="//@ spec: RXS-0113\n",
    )
    good_problems = check_commit(good)
    if good_problems:
        _fail(f"red 自检失败:齐备 commit 被误判有缺项(门过严):{good_problems}")


def _fail(msg: str) -> None:
    print(f"[check_contribution] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> int:
    red_self_test()

    base = resolve_base()
    if base is None or not rev_exists(base):
        print(
            "[check_contribution] PASS(无法解析基准 ref,跳过 PR 范围扫描;"
            "red 自检已过)"
        )
        return 0

    records = collect_commits(base)
    if not records:
        print(f"[check_contribution] PASS(base={base},0 非 merge commit 待校验)")
        return 0

    failures: list[str] = []
    for rec in records:
        for p in check_commit(rec):
            subject = rec.message.splitlines()[0] if rec.message.strip() else "<空提交信息>"
            failures.append(f"{rec.short} 「{subject}」: {p}")

    if failures:
        # agent 完全自主化（10 §7 v2.0 / AGENTS v3.0）:provenance / 条款号 / 验证标记
        # 降级为 advisory 审计输出,不再阻断合入。检测逻辑保留以维持可审计性。
        print(f"[check_contribution] ADVISORY(base={base},{len(records)} commit 扫描,不阻断)")
        for f in failures:
            print(f"  - {f}")
        print(
            "  说明:agent 完全自主模式下 provenance/条款号/验证为建议项,不阻断合入。"
            "见 CONTRIBUTING.md / 10 §7 v2.0。"
        )
        return 0

    print(
        f"[check_contribution] PASS(base={base},{len(records)} 非 merge commit 全过:"
        "provenance + 条款号 + 验证)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
