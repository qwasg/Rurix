#!/usr/bin/env python3
"""G8.2 实现互锁守卫（milestones/g8/CI_GATES.md §3 `g8.gov.implementation_interlock`）。

读取事实源并逐项输出，最后给出 READY / BLOCKED。两类断言严格分开：

* **事实门**（当前允许为红，红即 BLOCKED，不是脚本失败）：
  G7 收口、RD-038 处置、六行接入表终态、RFC-0019~0021 Agent Approved、G8.1 交付齐备。
* **一致性门**（红即脚本 FAIL，退出非零）：契约双状态不得与事实矛盾、
  ledger RFC 命名空间必须与 `rfcs/` 实际文件名一致、`reserved_in_flight[G8]` 必须在位。

诚实纪律：BLOCKED 是当前正确结论，不得被当作 G-G8-3 PASS；`--require-ready`
供未来 G8.2 实现 PR 作前置 required check（未 READY 即退出非零）。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

G7_CONTRACT = ROOT / "milestones/g7/G7_CONTRACT.md"
G8_CONTRACT = ROOT / "milestones/g8/G8_CONTRACT.md"
G8_PLAN = ROOT / "milestones/g8/G8_PLAN.md"
DEFERRED = ROOT / "registry/deferred.json"
LEDGER = ROOT / "registry/number_ledger.json"
RFCS = ROOT / "rfcs"

G8_1_DELIVERABLES = [
    "milestones/g8/G8_PLAN.md",
    "milestones/g8/G8_CONTRACT.md",
    "milestones/g8/CI_GATES.md",
    "milestones/g8/g8_budget.json",
    "milestones/g8/G8_CANDIDATE_DECISIONS.md",
    "milestones/g8/G8_ACCEPTANCE_MAP.md",
    "milestones/g8/G8_CAPABILITY_MATRIX.md",
]
G8_RFCS = [
    "rfcs/0019-rendering-platform.md",
    "rfcs/0020-asset-pipeline.md",
    "rfcs/0021-physics-platform.md",
]
TERMINAL_OK = {"closed", "open"}


def front_matter_field(text: str, field: str) -> str | None:
    m = re.search(rf"^{re.escape(field)}:\s*(\S+)\s*$", text, re.MULTILINE)
    return m.group(1) if m else None


def rd_entry(deferred: dict, rd_id: str) -> dict | None:
    items = deferred.get("entries") if isinstance(deferred, dict) else deferred
    for item in items or []:
        if isinstance(item, dict) and item.get("id") == rd_id:
            return item
    return None


def parse_interlock_table(plan_text: str) -> list[tuple[str, str, str]]:
    """返回 §1.0 六行接入表的 (分项, 启动快照, G8.2 前互锁终态)。"""
    rows: list[tuple[str, str, str]] = []
    in_table = False
    for line in plan_text.splitlines():
        if "RD-038 字面分项" in line and "互锁终态" in line:
            in_table = True
            continue
        if in_table:
            if not line.startswith("|"):
                if rows:
                    break
                continue
            if set(line.replace("|", "").strip()) <= set("-: "):
                continue
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            if len(cells) >= 3:
                rows.append((cells[0], cells[1], cells[2]))
    return rows


def rfc_review_state(path: Path) -> tuple[bool, str]:
    """(是否 Agent Approved 且有独立 provenance 评审记录, 说明)。"""
    if not path.exists():
        return False, "文件不存在"
    text = path.read_text(encoding="utf-8")
    status = ""
    drafter = ""
    for line in text.splitlines():
        if line.startswith("| 状态 |"):
            status = line
        elif line.startswith("| Provenance |"):
            drafter = line
    negatives = ("未批准", "Pending", "pending", "Draft", "尚未", "待评审", "草案")
    approved = "Agent Approved" in status and not any(n in status for n in negatives)
    drafter_prov = ""
    dm = re.search(r"`Assisted-by:\s*([^`]+)`", drafter)
    if dm:
        drafter_prov = dm.group(1).strip()
    section = text.split("对抗性评审记录", 1)[-1]
    reviewers = {
        p.strip()
        for p in re.findall(r"Assisted-by:\s*([^`|\n]+)", section)
    }
    independent = {r for r in reviewers if drafter_prov and r != drafter_prov}
    if not approved:
        return False, f"状态未达 Agent Approved（{status.strip() or '缺状态行'}）"
    if not independent:
        return False, "§9.1 缺与起草 provenance 不同的独立评审记录（D-409）"
    return True, f"Agent Approved；独立评审 provenance {sorted(independent)}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--require-ready",
        action="store_true",
        help="G8.2 实现 PR 前置模式：未 READY 即退出非零",
    )
    args = parser.parse_args()

    facts: list[tuple[bool, str]] = []
    consistency: list[tuple[bool, str]] = []

    g7_text = G7_CONTRACT.read_text(encoding="utf-8")
    g7_status = front_matter_field(g7_text, "status")
    facts.append((g7_status == "closed", f"① G7_CONTRACT status = {g7_status!r}（要求 closed）"))

    deferred = json.loads(DEFERRED.read_text(encoding="utf-8"))
    rd038 = rd_entry(deferred, "RD-038")
    rd038_status = rd038.get("status") if rd038 else None
    has_override = bool(
        rd038
        and any("override" in json.dumps(h, ensure_ascii=False).lower() for h in rd038.get("history", []))
    )
    plan_text = G8_PLAN.read_text(encoding="utf-8")
    table = parse_interlock_table(plan_text)
    terminal = [t for _, _, t in table]
    terminal_filled = bool(terminal) and all(t.lower() in TERMINAL_OK for t in terminal)

    rd038_ok = rd038_status == "closed" or (
        g7_status == "closed" and terminal_filled and has_override
    )
    facts.append(
        (
            rd038_ok,
            f"② RD-038 status = {rd038_status!r}；六行接入表终态 = {terminal or '未解析到表'}；"
            f"history 独立 override = {has_override}（要求 closed，或 G7 closed 后终态全填 + override）",
        )
    )
    facts.append(
        (
            len(table) == 6,
            f"③ G8_PLAN §1.0 接入表行数 = {len(table)}（要求 6 行逐行可判）",
        )
    )

    missing = [p for p in G8_1_DELIVERABLES if not (ROOT / p).exists()]
    facts.append((not missing, f"④ G8.1 治理交付齐备（缺 {missing or '无'}）"))

    for rfc in G8_RFCS:
        ok, why = rfc_review_state(ROOT / rfc)
        facts.append((ok, f"⑤ {rfc}：{why}"))

    # --- 一致性门 ---
    ledger = json.loads(LEDGER.read_text(encoding="utf-8"))
    rfc_ns = ledger["namespaces"]["RFC"]
    on_tree = sorted(
        int(m.group(1))
        for m in (re.match(r"^(\d{4})-.*\.md$", p.name) for p in RFCS.glob("*.md"))
        if m
    )
    actual_max = on_tree[-1] if on_tree else 0
    consistency.append(
        (
            rfc_ns["on_tree_max"] == actual_max and rfc_ns["next_free"] == actual_max + 1,
            f"C1 ledger RFC on_tree_max/next_free = {rfc_ns['on_tree_max']}/{rfc_ns['next_free']}；"
            f"rfcs/ 实际末号 = {actual_max}（要求台账随 materialize 校准，v1.13/v1.28/v1.29/v1.38 先例）",
        )
    )
    g8_claim = [r for r in ledger.get("reserved_in_flight", []) if str(r.get("owner", "")).startswith("G8")]
    consistency.append((bool(g8_claim), "C2 ledger reserved_in_flight[G8] claim 在位"))

    g8_text = G8_CONTRACT.read_text(encoding="utf-8")
    impl_status = front_matter_field(g8_text, "implementation_status")
    facts_all_green = all(ok for ok, _ in facts)
    consistency.append(
        (
            impl_status == "blocked" or facts_all_green,
            f"C3 G8_CONTRACT implementation_status = {impl_status!r}；事实门全绿 = {facts_all_green}"
            "（事实未全绿时必须保持 blocked，禁止治理完成冒充实现开工）",
        )
    )

    print("[check_g8_implementation_interlock] 事实门（当前可为红）：")
    for ok, msg in facts:
        print(f"  {'PASS' if ok else 'RED '} {msg}")
    print("[check_g8_implementation_interlock] 一致性门（红即脚本失败）：")
    for ok, msg in consistency:
        print(f"  {'PASS' if ok else 'FAIL'} {msg}")

    verdict = "READY" if facts_all_green else "BLOCKED"
    print(f"[check_g8_implementation_interlock] VERDICT = {verdict}")
    if verdict == "BLOCKED":
        print(
            "  BLOCKED 是当前正确结论：G8.2+ 的 src/、spec/、conformance/ 与数字 workflow 步骤保持 0-byte；"
            "本输出不得被当作 G-G8-3 PASS。"
        )

    consistency_failed = [msg for ok, msg in consistency if not ok]
    if consistency_failed:
        print(f"[check_g8_implementation_interlock] FAIL — {len(consistency_failed)} 项一致性门为红")
        return 1
    if args.require_ready and verdict != "READY":
        print("[check_g8_implementation_interlock] FAIL — --require-ready 模式下互锁未 READY")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
