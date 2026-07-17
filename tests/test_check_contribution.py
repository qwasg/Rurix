"""check_contribution provenance trailer 双格式单测(冒号机读形 + 括号人写形)。

背景:PROVENANCE_RE 原只认 `Assisted-by: <tool>:<model>` 冒号形,而全仓人写惯例是
`Assisted-by: Claude (Fable 5)` 括号形(已合入 PR #147/#148/#151 即此形),导致
ADVISORY 门对这些 commit 误报「缺 provenance trailer」。本轮放宽正则同时接受括号形
(语义映射 tool=claude-code);本文件锁定:两种格式 + Co-Authored-By + 双行并写
(PR #150+ 过渡先例)均绿,无 trailer / 裸无模型形 / 行中内嵌形仍红。

纳入 pr-smoke 的 `pytest tests/ -q` 门(反 YAML-only)。
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from ci.check_contribution import CommitRecord, check_commit, red_self_test

# 非语义文件:只触发 provenance 规则(不落条款号/验证规则,聚焦规则 1)。
DOC_FILES = ["13_DECISION_LOG.md"]


def _problems(message: str) -> list[str]:
    return check_commit(CommitRecord(sha="", message=message, files=DOC_FILES))


# —— 绿:三种等价 provenance 形 + 过渡期双行并写 ——


@pytest.mark.parametrize(
    "trailer",
    [
        "Assisted-by: claude-code:claude-fable-5",
        "Assisted-by: Claude (Fable 5)",
        "Assisted-by: Claude Code (Fable 5)",
        "assisted-by: claude (fable 5)",
        "Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>",
        "Assisted-by: Claude (Fable 5)\nAssisted-by: claude-code:claude-fable-5",
    ],
    ids=[
        "colon-machine-form",
        "paren-human-form",
        "paren-spaced-name",
        "case-insensitive",
        "co-authored-by",
        "dual-line-transition",
    ],
)
def test_provenance_trailer_accepted(trailer: str):
    assert _problems(f"docs: sample change\n\n{trailer}\n") == []


def test_paren_form_composes_with_clause_and_validation():
    # 括号形与规则 2(条款号)/规则 3(验证标记)正交:src 改动 + 三项齐备应零缺项。
    rec = CommitRecord(
        sha="",
        message=(
            "feat(geometry): add bvh node\n\n"
            "Validation: cargo test -p rurix-geometry PASS\n\n"
            "Assisted-by: Claude (Fable 5)\n"
        ),
        files=["src/rurix-geometry/src/lib.rs"],
        added_text="//@ spec: RXS-0113\n",
    )
    assert check_commit(rec) == []


# —— 红:缺 trailer / 形不完整 / 行中内嵌均仍判缺 ——


@pytest.mark.parametrize(
    "message",
    [
        "docs: sample change\n",
        "docs: sample change\n\nAssisted-by: Claude\n",
        "docs: sample change\n\nAssisted-by: (Fable 5)\n",
        "docs: inline mention of Assisted-by: x:y mid-line only\n",
    ],
    ids=[
        "no-trailer",
        "bare-name-no-model",
        "paren-without-name",
        "inline-not-trailer-line",
    ],
)
def test_missing_or_malformed_provenance_flagged(message: str):
    probs = _problems(message)
    assert any("provenance" in p for p in probs), probs


def test_red_self_test_still_passes():
    # 内置 red 自检含括号形绿例 + 裸无模型红例;失败会 sys.exit(1) → SystemExit。
    red_self_test()
