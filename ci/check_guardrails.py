"""PR Smoke 步骤 3:guardrail 字节级核对(14 §2 / CI_GATES.md §4,M0 版五项)。

对比基准 ref(优先级:命令行参数 > GITHUB_BASE_REF > tag m2-closed):
  1. 规划文档集(00-14 与 deep-research/)0-byte;
  2. registry/*.json 既有条目只追加;
  3. 预算 JSON:measured_local 条目冻结;estimated 只允许转 measured_local;
  4. evidence/ 只增不删不改;
  5. status: closed 的契约文件只追加;
  6. registry/error_codes.json 含义字段冻结(M1 CI_GATES §4 第 8 项,M1.1 激活);
  7. spec/ 变更必须携带档位标记(修订记录只追加,M1 CI_GATES §4 第 7 项,M1.2 激活);
  8. tests/ui/ 的 .stderr 变更必须经审批 bless(bless_log.md 同 diff 追加且既有行
     0-byte,M1 CI_GATES §4 第 6 项,M1.4 激活)。
  9. tests/mir/ 的 .mir golden 变更必须经审批 bless(tests/mir/bless_log.md 同 diff
     追加且既有行 0-byte,M3 CI_GATES §4 第 2 项,M3.3 WP6 激活)。
"""
from __future__ import annotations

import json
import os
import re
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
    # M3 开工起回退基准切至 m2-closed(M3 CI_GATES §4 第 1 项 / M3_PLAN §1 任务 1)
    return "m2-closed"


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


SPEC_TIER_RE = re.compile(r"\b(Direct|Mini-RFC|Full RFC)\b")


def spec_revision_rows(text: str) -> list[str] | None:
    """提取 spec 文件修订记录表的数据行(版本头与分隔行除外);无修订记录节返回 None。"""
    lines = text.splitlines()
    heading_idx = None
    for i, line in enumerate(lines):
        if line.startswith("#") and "修订记录" in line:
            heading_idx = i
    if heading_idx is None:
        return None
    rows = []
    for line in lines[heading_idx + 1:]:
        s = line.strip()
        if s.startswith("#"):
            break  # 下一节
        if not s.startswith("|"):
            continue
        if "版本" in s or set(s) <= {"|", "-", " "}:
            continue  # 表头/分隔行
        rows.append(s)
    return rows


def check_spec_tier_markers(base: str, diffs: list[tuple[str, str]]) -> None:
    """spec/ 变更必须携带档位标记:修订记录只追加且新行含合法档位(M1 CI_GATES §4 第 7 项)。"""
    for status, path in diffs:
        if not (path.startswith("spec/") and path.endswith(".md")):
            continue
        if status == "D":
            err(f"{path}: spec 文件不得删除(条款永不复用,弃用标注 deprecated,10 §9.5)")
            continue
        cur_rows = spec_revision_rows((ROOT / path).read_text(encoding="utf-8"))
        if cur_rows is None:
            err(f"{path}: 缺修订记录节(spec 体例,spec/README.md §3)")
            continue
        base_text = git_show(base, path)
        base_rows = spec_revision_rows(base_text) or [] if base_text is not None else []
        if cur_rows[: len(base_rows)] != base_rows:
            err(f"{path}: 修订记录既有行被修改(只追加,spec/README.md §3)")
            continue
        new_rows = cur_rows[len(base_rows):]
        if not new_rows:
            err(f"{path}: spec 变更未新增修订行(档位标记缺失即 FAIL,M1 CI_GATES §4.7)")
            continue
        for row in new_rows:
            if not SPEC_TIER_RE.search(row):
                err(f"{path}: 新增修订行缺合法档位标记(Direct / Mini-RFC / Full RFC): {row!r}")


BLESS_LOG = "tests/ui/bless_log.md"


def bless_log_rows(text: str) -> list[str]:
    """提取 bless_log.md 审批表数据行(表头/分隔行除外)。"""
    rows = []
    for line in text.splitlines():
        s = line.strip()
        if not s.startswith("|"):
            continue
        if "日期" in s or set(s) <= {"|", "-", " "}:
            continue
        rows.append(s)
    return rows


def check_ui_bless(base: str, diffs: list[tuple[str, str]]) -> None:
    """UI snapshot 变更必须经审批 bless(14 §6;M1 CI_GATES §4 第 6 项,M1.4 激活)。

    diff 含 tests/ui/**/*.stderr 的新增/修改/删除时:bless_log.md 必须同 diff
    追加新行(既有行 0-byte);bless_log 自身不得删除。
    """
    snapshot_changes = [
        (status, path)
        for status, path in diffs
        if path.startswith("tests/ui/") and path.endswith(".stderr")
    ]
    log_deleted = any(status == "D" and path == BLESS_LOG for status, path in diffs)
    if log_deleted:
        err(f"{BLESS_LOG}: bless 审批记录不得删除(14 §6)")
        return
    if not snapshot_changes:
        return
    log_file = ROOT / BLESS_LOG
    if not log_file.is_file():
        err(f"{BLESS_LOG}: 缺失——.stderr 变更必须携带 bless 审批记录(14 §6)")
        return
    cur_rows = bless_log_rows(log_file.read_text(encoding="utf-8"))
    base_text = git_show(base, BLESS_LOG)
    base_rows = bless_log_rows(base_text) if base_text is not None else []
    if cur_rows[: len(base_rows)] != base_rows:
        err(f"{BLESS_LOG}: 既有审批行被修改(只追加,14 §6)")
        return
    if len(cur_rows) <= len(base_rows):
        changed = ", ".join(p for _, p in snapshot_changes[:5])
        err(
            f"{BLESS_LOG}: .stderr 变更未附 bless 审批行(未审批 bless 即 FAIL,"
            f"M1 CI_GATES §4.6): {changed}"
        )


MIR_BLESS_LOG = "tests/mir/bless_log.md"


def check_mir_bless(base: str, diffs: list[tuple[str, str]]) -> None:
    """MIR 文本 golden 变更必须经审批 bless(14 §2 常驻集;M3 CI_GATES §4 第 2 项,
    M3.3 WP6 激活)。

    diff 含 tests/mir/**/*.mir 的新增/修改/删除时:bless_log.md 必须同 diff
    追加新行(既有行 0-byte);bless_log 自身不得删除。
    """
    golden_changes = [
        (status, path)
        for status, path in diffs
        if path.startswith("tests/mir/") and path.endswith(".mir")
    ]
    log_deleted = any(status == "D" and path == MIR_BLESS_LOG for status, path in diffs)
    if log_deleted:
        err(f"{MIR_BLESS_LOG}: MIR golden bless 审批记录不得删除(14 §2)")
        return
    if not golden_changes:
        return
    log_file = ROOT / MIR_BLESS_LOG
    if not log_file.is_file():
        err(f"{MIR_BLESS_LOG}: 缺失——.mir 变更必须携带 bless 审批记录(14 §2)")
        return
    cur_rows = bless_log_rows(log_file.read_text(encoding="utf-8"))
    base_text = git_show(base, MIR_BLESS_LOG)
    base_rows = bless_log_rows(base_text) if base_text is not None else []
    if cur_rows[: len(base_rows)] != base_rows:
        err(f"{MIR_BLESS_LOG}: 既有审批行被修改(只追加,14 §2)")
        return
    if len(cur_rows) <= len(base_rows):
        changed = ", ".join(p for _, p in golden_changes[:5])
        err(
            f"{MIR_BLESS_LOG}: .mir 变更未附 bless 审批行(未审批 bless 即 FAIL,"
            f"M3 CI_GATES §4.2): {changed}"
        )


def check_error_codes(base: str, path: str) -> None:
    """错误码语义可加不可改(10 §6 稳定面;M1 CI_GATES §4 第 8 项,M1.1 激活)。"""
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
            err(f"{path}: 错误码消失: {eid}(编号永不复用,弃用走 deprecated,10 §9.5)")
            continue
        for field in ("id", "title", "message_key", "introduced_in"):
            if cur_entry.get(field) != base_entry.get(field):
                err(f"{path} {eid}: 含义字段 {field} 被修改(可加不可改,10 §6)")
        if cur_entry.get("status") != base_entry.get("status") and not (
            base_entry.get("status") == "active" and cur_entry.get("status") == "deprecated"
        ):
            err(f"{path} {eid}: status 仅允许 active → deprecated")


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
    check_error_codes(base, "registry/error_codes.json")
    check_spec_tier_markers(base, diffs)
    check_ui_bless(base, diffs)
    check_mir_bless(base, diffs)
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
