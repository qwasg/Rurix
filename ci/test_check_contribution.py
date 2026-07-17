# -*- coding: utf-8 -*-
"""pytest 红绿:check_contribution 规则 4 —— Full RFC 对抗性评审记录门(D-409 Proposed)。

合成 RFC fixture:缺段 / 仅占位 / 评审==起草 provenance → finding;含段且评审≠起草 → 过。
纯函数 `check_rfc_adversarial_review` 直接喂文本,不触 git/网络。

运行: py -3 -m pytest ci/test_check_contribution.py -q
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_contribution as cc  # noqa: E402

# 起草 provenance 落 Header;§9 占位,§9.1 由各用例追加。
HEADER = (
    "# RFC-0099 — 合成 fixture\n\n"
    "| 字段 | 值 |\n|---|---|\n"
    "| Provenance | `Assisted-by: claude-code:claude-opus-4-8` |\n\n"
    "## 9. 未决问题 / 关键裁决\n\n〈占位〉\n"
)

SECTION_DISTINCT = (
    "## 9.1 对抗性评审记录\n\n"
    "| 评审者 provenance | `Assisted-by: codex:gpt-5` |\n"
    "| F1 | 边界情况漏判 | med | 采纳:改 §4 |\n"
)
SECTION_SAME = (
    "## 9.1 对抗性评审记录\n\n"
    "| 评审者 provenance | `Assisted-by: claude-code:claude-opus-4-8` |\n"
    "| F1 | 边界情况 | low | 采纳 |\n"
)
SECTION_PLACEHOLDER = (
    "## 9.1 对抗性评审记录\n\n"
    "| 评审者 provenance | `Assisted-by: <评审 tool>:<评审 model>` |\n"
    "| F1 | 〈…〉 | 〈low〉 | 〈采纳/驳回〉 |\n"
)


def test_missing_section_is_flagged() -> None:
    """缺整段 → 恰一条 finding,内容指明缺段。"""
    problems = cc.check_rfc_adversarial_review(HEADER)
    assert len(problems) == 1
    assert "缺" in problems[0] and "对抗性评审记录" in problems[0]


def test_placeholder_only_is_flagged() -> None:
    """仅模板占位(评审 provenance 未填)→ 视同空段,判 finding。"""
    problems = cc.check_rfc_adversarial_review(HEADER + SECTION_PLACEHOLDER)
    assert problems
    assert "为空/仅占位" in problems[0]


def test_same_provenance_is_flagged() -> None:
    """评审 provenance == 起草 → 未区分,判 finding。"""
    problems = cc.check_rfc_adversarial_review(HEADER + SECTION_SAME)
    assert problems
    assert "未与起草区分" in problems[0]


def test_distinct_reviewer_passes() -> None:
    """评审 provenance ≠ 起草 + 有 finding 行 → 通过(空列表)。"""
    problems = cc.check_rfc_adversarial_review(HEADER + SECTION_DISTINCT)
    assert problems == []


def test_full_rfc_path_regex() -> None:
    """规则 4 只强制 Full RFC(rfcs/NNNN-*.md);mini/README/TEMPLATE 排除。"""
    assert cc.FULL_RFC_RE.match("rfcs/0012-toolchain-real-distribution.md")
    assert not cc.FULL_RFC_RE.match("rfcs/mini-0009-toolchain-frontend.md")
    assert not cc.FULL_RFC_RE.match("rfcs/README.md")
    assert not cc.FULL_RFC_RE.match("rfcs/TEMPLATE-RFC.md")
