"""预算 JSON 命名空间 check 单测(14 §3:每新增预算配标准 namespace check;M1 起覆盖全部预算文件)。"""
from __future__ import annotations

import json
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
BUDGETS = sorted(ROOT.glob("milestones/*/*_budget.json"))
GROUPS = ("entries", "ratio_assertions", "counter_assertions")


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_budget_files_discovered():
    # M0 与 M1 预算必须都在自动发现集内(check_schemas.py / budget_eval.py 同一 glob)
    names = {p.parent.name for p in BUDGETS}
    assert {"m0", "m1"} <= names


@pytest.mark.parametrize("path", BUDGETS, ids=lambda p: p.parent.name)
def test_namespace_matches_milestone_dir(path: Path):
    doc = load(path)
    assert doc.get("namespace") == path.parent.name


@pytest.mark.parametrize("path", BUDGETS, ids=lambda p: p.parent.name)
def test_all_ids_carry_namespace_prefix(path: Path):
    doc = load(path)
    prefix = doc["namespace"] + "."
    for group in GROUPS:
        for entry in doc.get(group, []):
            assert entry["id"].startswith(prefix), f"{entry['id']} 未带前缀 {prefix}"


def test_merged_ids_unique_across_budgets():
    # 多预算合并加载时冲突即 FAIL(14 §3)
    seen: set[str] = set()
    for path in BUDGETS:
        doc = load(path)
        for group in GROUPS:
            for entry in doc.get(group, []):
                assert entry["id"] not in seen, f"命名空间冲突: {entry['id']}"
                seen.add(entry["id"])


@pytest.mark.parametrize("path", BUDGETS, ids=lambda p: p.parent.name)
def test_estimated_entries_have_skip_reason(path: Path):
    doc = load(path)
    for group in ("entries", "ratio_assertions"):
        for entry in doc.get(group, []):
            if entry.get("evidence") == "estimated":
                assert entry.get("skip_reason"), f"{entry['id']}: estimated 占位缺 skip_reason(14 §3)"
