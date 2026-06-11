"""PR Smoke 步骤 3:guardrail 字节级核对(14 §2 / CI_GATES.md §4,M0 版五项)。

对比基准 ref(优先级:命令行参数 > GITHUB_BASE_REF > tag m0-baseline):
  1. 规划文档集(00-14 与 deep-research/)0-byte;
  2. registry/*.json 既有条目只追加;
  3. 预算 JSON:measured_local 条目冻结;estimated 只允许转 measured_local;
  4. evidence/ 只增不删不改;
  5. status: closed 的契约文件只追加。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ERRORS: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", check=False
    ).stdout


def git_show(ref: str, path: str) -> str | None:
    proc = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8", check=False,
    )
    return proc.stdout if proc.returncode == 0 else None


def resolve_base() -> str:
    if len(sys.argv) > 1:
        return sys.argv[1]
    gh_base = os.environ.get("GITHUB_BASE_REF")
    if gh_base:
        return f"origin/{gh_base}"
    return "m0-baseline"


def changed_paths(base: str) -> list[str]:
    out = git("diff", "--name-status", base, "--", ".")
    rows = []
    for line in out.splitlines():
        parts = line.split("\t")
        if len(parts) >= 2:
            rows.append((parts[0], parts[-1]))
    return rows


def check_planning_docs(diffs: list[tuple[str, str]]) -> None:
    for status, path in diffs:
        top = path.split("/")[0]
        if top == "deep-research" or (top.endswith(".md") and top[:2].isdigit() and "/" not in path):
            err(f"规划文档在执行 PR 中被改动({status}): {path}(勘误须独立 PR,00 §6.3)")


def entry_key_fields(entry: dict, kind: str) -> dict:
    """既有条目的不可变字段子集。"""
    if kind == "deferred":
        immutable = ("id", "title", "reason", "backfill_condition")
        appendable = "history"
    else:
        immutable = ("id", "direction", "trigger_condition", "permanence")
        appendable = "decisions"
    return {f: entry.get(f) for f in immutable}, entry.get(appendable, [])


def check_registry(base: str, path: str, kind: str) -> None:
    base_text = git_show(base, path)
    if base_text is None:
        return  # 基准中不存在 → 新文件,放行
    base_doc = json.loads(base_text)
    cur_doc = json.loads((ROOT / path).read_text(encoding="utf-8"))
    cur_by_id = {e["id"]: e for e in cur_doc.get("entries", [])}
    for base_entry in base_doc.get("entries", []):
        eid = base_entry["id"]
        cur_entry = cur_by_id.get(eid)
        if cur_entry is None:
            err(f"{path}: 既有条目消失: {eid}(deferred/gating 不能消失,14 §4 §7)")
            continue
        b_imm, b_log = entry_key_fields(base_entry, kind)
        c_imm, c_log = entry_key_fields(cur_entry, kind)
        if b_imm != c_imm:
            err(f"{path} {eid}: 不可变字段被修改(只追加,需人工审查)")
        if c_log[: len(b_log)] != b_log:
            err(f"{path} {eid}: 留痕数组被改写(只允许追加)")


def check_budget(base: str, path: str) -> None:
    base_text = git_show(base, path)
    if base_text is None:
        return
    base_doc = json.loads(base_text)
    cur_doc = json.loads((ROOT / path).read_text(encoding="utf-8"))
    for group in ("entries", "ratio_assertions"):
        cur_by_id = {e["id"]: e for e in cur_doc.get(group, [])}
        for base_entry in base_doc.get(group, []):
            eid = base_entry["id"]
            cur_entry = cur_by_id.get(eid)
            if cur_entry is None:
                err(f"{path}: 预算条目消失: {eid}")
                continue
            if base_entry.get("evidence") == "measured_local":
                if cur_entry != base_entry:
                    err(f"{path} {eid}: measured_local 条目被修改(历史预算 0-byte,14 §2)")
            elif base_entry.get("evidence") == "estimated":
                if cur_entry != base_entry and cur_entry.get("evidence") != "measured_local":
                    err(f"{path} {eid}: estimated 条目只允许回填为 measured_local")


def check_evidence(base: str, diffs: list[tuple[str, str]]) -> None:
    for status, path in diffs:
        if path.startswith("evidence/") and path != "evidence/README.md":
            if git_show(base, path) is not None:
                err(f"evidence/ 既有文件被{ '删除' if status == 'D' else '修改' }: {path}(证据不可篡改)")


def check_closed_contracts(base: str) -> None:
    for contract in sorted(ROOT.glob("milestones/*/M*_CONTRACT.md")):
        rel = contract.relative_to(ROOT).as_posix()
        base_text = git_show(base, rel)
        if base_text is None:
            continue
        if "status: closed" not in base_text.splitlines()[1:20].__str__():
            continue
        cur_text = contract.read_text(encoding="utf-8")
        if not cur_text.startswith(base_text):
            err(f"{rel}: 已关闭契约的既有内容被修改(close-out 只追加,14 §1)")


def main() -> int:
    base = resolve_base()
    if not git("rev-parse", "--verify", base).strip():
        print(f"[check_guardrails] FAIL: 基准 ref 不存在: {base}")
        return 1
    diffs = changed_paths(base)
    check_planning_docs(diffs)
    check_registry(base, "registry/deferred.json", "deferred")
    check_registry(base, "registry/spike_gating.json", "gating")
    for budget in sorted(ROOT.glob("milestones/*/m*_budget.json")):
        check_budget(base, budget.relative_to(ROOT).as_posix())
    check_evidence(base, diffs)
    check_closed_contracts(base)
    if ERRORS:
        print(f"[check_guardrails] FAIL (base={base})")
        for e in ERRORS:
            print(f"  - {e}")
        return 1
    print(f"[check_guardrails] PASS (base={base}, {len(diffs)} changed paths)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
