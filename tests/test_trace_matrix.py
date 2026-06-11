"""traceability 矩阵工具单测(G-M1-4 配套;合成数据复算 + 真实仓库全锚定)。"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from ci.trace_matrix import build_matrix, collect_anchors, gather_repo, parse_clauses


def test_parse_clauses_synthetic():
    spec = {"spec/a.md": "### RXS-0001 标题\n正文\n### RXS-0002 另一条\n"}
    clauses = parse_clauses(spec)
    assert clauses == {"RXS-0001": "spec/a.md", "RXS-0002": "spec/a.md"}


def test_parse_clauses_rejects_duplicates():
    spec = {
        "spec/a.md": "### RXS-0001 甲\n",
        "spec/b.md": "### RXS-0001 乙\n",
    }
    with pytest.raises(ValueError):
        parse_clauses(spec)


def test_collect_anchors_multi_clause_line_and_dedupe():
    tests = {
        "conformance/x.rx": "//@ spec: RXS-0001, RXS-0002\nfn f() {}\n//@ spec: RXS-0001\n",
        "src/t.rs": "    //@ spec: RXS-0002\n    #[test]\n    fn t() {}\n",
    }
    anchors = collect_anchors(tests)
    assert anchors["RXS-0001"] == ["conformance/x.rx"]
    assert anchors["RXS-0002"] == ["conformance/x.rx", "src/t.rs"]


def test_build_matrix_flags_unanchored_and_ghosts():
    clauses = {"RXS-0001": "spec/a.md", "RXS-0002": "spec/a.md"}
    anchors = {"RXS-0001": ["t.rx"], "RXS-9999": ["ghost.rx"]}
    matrix, unanchored, ghosts = build_matrix(clauses, anchors)
    assert matrix["clauses"]["RXS-0001"] == ["t.rx"]
    assert matrix["clauses"]["RXS-0002"] == []
    assert unanchored == ["RXS-0002"]
    assert ghosts == ["RXS-9999"]


def test_real_repo_all_clauses_anchored():
    spec_texts, test_texts = gather_repo()
    clauses = parse_clauses(spec_texts)
    anchors = collect_anchors(test_texts)
    _, unanchored, ghosts = build_matrix(clauses, anchors)
    assert not unanchored, f"未锚定条款: {unanchored}"
    assert not ghosts, f"幽灵锚定: {ghosts}"
    assert len(clauses) >= 31  # RXS-0001 ~ RXS-0031
