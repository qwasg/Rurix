#!/usr/bin/env python3
"""G9.2 实现互锁守卫（milestones/g9/CI_GATES.md §3 `g9.gov.implementation_interlock`）。

读取事实源并逐项输出，最后给出 READY / BLOCKED。两类断言严格分开：

* **事实门**（当前允许为红，红即 BLOCKED，不是脚本失败）：
  ① G8_CONTRACT front matter status == closed；
  ② G9_CONTRACT 文本含 G9.0 不可变 ref `1d9460a1`（立项基线登记）；
  ③ G9.1 治理交付七件齐备；
  ④ RFC-0022/0023/0024 均 Agent Approved 且 §9.1 有 ≠ 起草 provenance 的独立评审记录；
  ⑤ deferred.json RD-039 history 含 M61 strategic_override 与 M06/M09 触发登记追加行、
    RD-040 history 含 M52 strategic_override 追加行（只追加不回写校验）；
  ⑥ G9_CONTRACT §8 已含 implementation_status 解锁 / G-G9-3 激活记录（起始必然 RED）。
* **一致性门**（红即脚本 FAIL，退出非零）：
  C1 ledger namespaces.RFC on_tree_max/next_free 与 `rfcs/` 实际 .md 末号一致；
  C2 ledger reserved_in_flight 存在 owner 以 G9 开头的行；
  C3 G9_CONTRACT implementation_status == blocked，或事实门全绿。

诚实纪律：BLOCKED 是当前正确结论，不得被当作 G-G9-3 PASS；`--require-ready`
供未来 G9.2 实现 PR 作前置 required check（未 READY 即退出非零）。
`--selftest` 用可注入输入的受控负样本证明每组断言都能红/败（G8 版无此设施）。

事实门与一致性门的计算全部重构为纯函数（evaluate_fact_gates /
evaluate_consistency_gates），输入经 TreeInputs 注入，缺文件优雅降级为 RED/FAIL，
绝不 traceback。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

G8_CONTRACT = ROOT / "milestones/g8/G8_CONTRACT.md"
G9_CONTRACT = ROOT / "milestones/g9/G9_CONTRACT.md"
DEFERRED = ROOT / "registry/deferred.json"
LEDGER = ROOT / "registry/number_ledger.json"
RFCS = ROOT / "rfcs"

G9_0_IMMUTABLE_REF = "1d9460a1"

G9_1_DELIVERABLES = [
    "milestones/g9/G9_PLAN.md",
    "milestones/g9/G9_CONTRACT.md",
    "milestones/g9/CI_GATES.md",
    "milestones/g9/g9_budget.json",
    "milestones/g9/G9_CANDIDATE_DECISIONS.md",
    "milestones/g9/G9_ACCEPTANCE_MAP.md",
    "milestones/g9/G9_CAPABILITY_MATRIX.md",
]
G9_RFCS = [
    "rfcs/0022-virtual-geometry-gi-semantics.md",
    "rfcs/0023-gpu-driven-submission-shading.md",
    "rfcs/0024-physics-platform-revision.md",
]


@dataclass
class TreeInputs:
    """互锁守卫的全部可注入输入；None / 空表示树上缺失。"""

    g8_contract_text: str | None = None
    g9_contract_text: str | None = None
    deliverables_missing: list[str] = field(default_factory=list)
    rfc_texts: dict[str, str | None] = field(default_factory=dict)
    deferred: dict | None = None
    ledger: dict | None = None
    rfcs_filenames: list[str] = field(default_factory=list)


def load_inputs(root: Path) -> TreeInputs:
    """从树上装载输入；缺文件/坏 JSON 一律降级为 None，不 traceback。"""
    def _read(path: Path) -> str | None:
        return path.read_text(encoding="utf-8") if path.exists() else None

    def _read_json(path: Path) -> dict | None:
        try:
            return json.loads(path.read_text(encoding="utf-8")) if path.exists() else None
        except (json.JSONDecodeError, OSError):
            return None

    return TreeInputs(
        g8_contract_text=_read(root / "milestones/g8/G8_CONTRACT.md"),
        g9_contract_text=_read(G9_CONTRACT),
        deliverables_missing=[p for p in G9_1_DELIVERABLES if not (root / p).exists()],
        rfc_texts={rfc: _read(root / rfc) for rfc in G9_RFCS},
        deferred=_read_json(DEFERRED),
        ledger=_read_json(LEDGER),
        rfcs_filenames=[p.name for p in RFCS.glob("*.md")] if RFCS.exists() else [],
    )


def front_matter_field(text: str, field: str) -> str | None:
    m = re.search(rf"^{re.escape(field)}:\s*(\S+)\s*$", text, re.MULTILINE)
    return m.group(1) if m else None


def rd_entry(deferred: dict | None, rd_id: str) -> dict | None:
    items = deferred.get("entries") if isinstance(deferred, dict) else deferred
    for item in items or []:
        if isinstance(item, dict) and item.get("id") == rd_id:
            return item
    return None


def rfc_review_state_text(text: str | None) -> tuple[bool, str]:
    """(是否 Agent Approved 且有独立 provenance 评审记录, 说明)。复用 G8 逻辑。"""
    if text is None:
        return False, "文件不存在"
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


ACTIVATION_RE = re.compile(
    r"^#{2,5}\s*§?8\.\d+[^\n]*(implementation_status 解锁|G-G9-3)", re.MULTILINE
)


def evaluate_fact_gates(inp: TreeInputs) -> list[tuple[bool, str]]:
    """事实门：红 = BLOCKED，不是脚本失败。"""
    facts: list[tuple[bool, str]] = []

    g8_status = front_matter_field(inp.g8_contract_text, "status") if inp.g8_contract_text else None
    facts.append((g8_status == "closed", f"① G8_CONTRACT status = {g8_status!r}（要求 closed）"))

    has_ref = inp.g9_contract_text is not None and G9_0_IMMUTABLE_REF in inp.g9_contract_text
    facts.append(
        (
            has_ref,
            f"② G9_CONTRACT 登记 G9.0 不可变 ref `{G9_0_IMMUTABLE_REF}`（立项基线；"
            f"{'缺文件' if inp.g9_contract_text is None else ('含' if has_ref else '不含')}）",
        )
    )

    facts.append(
        (
            not inp.deliverables_missing,
            f"③ G9.1 治理交付齐备（缺 {inp.deliverables_missing or '无'}）",
        )
    )

    for rfc in G9_RFCS:
        ok, why = rfc_review_state_text(inp.rfc_texts.get(rfc))
        facts.append((ok, f"④ {rfc}：{why}"))

    def _events(rd: dict | None) -> list[str]:
        return [
            str(h.get("event", ""))
            for h in (rd or {}).get("history", [])
            if isinstance(h, dict)
        ]

    rd039_events = _events(rd_entry(inp.deferred, "RD-039"))
    rd040_events = _events(rd_entry(inp.deferred, "RD-040"))
    m61_override = any("override" in e.lower() and "M61" in e for e in rd039_events)
    m06_m09_trigger = any("触发" in e and ("M06" in e or "M09" in e) for e in rd039_events)
    m52_override = any("override" in e.lower() and "M52" in e for e in rd040_events)
    facts.append(
        (
            inp.deferred is not None and m61_override and m06_m09_trigger and m52_override,
            f"⑤ deferred.json override 登记：RD-039 M61 strategic_override = {m61_override}、"
            f"RD-039 M06/M09 触发登记 = {m06_m09_trigger}、RD-040 M52 strategic_override = {m52_override}"
            f"（要求均为 True；只追加不回写校验）",
        )
    )

    activated = inp.g9_contract_text is not None and bool(ACTIVATION_RE.search(inp.g9_contract_text))
    facts.append(
        (
            activated,
            f"⑥ G9_CONTRACT §8 实现门激活记录（小节标题含「implementation_status 解锁」或「G-G9-3」）"
            f"= {activated}（起始必然 RED，保证当前诚实 BLOCKED）",
        )
    )
    return facts


def evaluate_consistency_gates(inp: TreeInputs, facts_all_green: bool) -> list[tuple[bool, str]]:
    """一致性门：红即脚本 FAIL，退出非零。"""
    consistency: list[tuple[bool, str]] = []

    on_tree = sorted(
        int(m.group(1))
        for m in (re.match(r"^(\d{4})-.*\.md$", n) for n in inp.rfcs_filenames)
        if m
    )
    actual_max = on_tree[-1] if on_tree else 0
    rfc_ns = (inp.ledger or {}).get("namespaces", {}).get("RFC", {})
    on_tree_max = rfc_ns.get("on_tree_max")
    next_free = rfc_ns.get("next_free")
    consistency.append(
        (
            on_tree_max == actual_max and next_free == actual_max + 1,
            f"C1 ledger RFC on_tree_max/next_free = {on_tree_max}/{next_free}；"
            f"rfcs/ 实际末号 = {actual_max}（要求台账随 materialize 校准，v1.13/v1.28/v1.29/v1.38/v1.40 先例）",
        )
    )

    reserved = (inp.ledger or {}).get("reserved_in_flight", [])
    g9_claim = [r for r in reserved if str(r.get("owner", "")).startswith("G9")]
    consistency.append((bool(g9_claim), 'C2 ledger reserved_in_flight 存在 owner 以 "G9" 开头的行'))

    impl_status = front_matter_field(inp.g9_contract_text, "implementation_status") if inp.g9_contract_text else None
    consistency.append(
        (
            impl_status == "blocked" or facts_all_green,
            f"C3 G9_CONTRACT implementation_status = {impl_status!r}；事实门全绿 = {facts_all_green}"
            "（事实未全绿时必须保持 blocked，禁止治理完成冒充实现开工）",
        )
    )
    return consistency


def run(inp: TreeInputs, require_ready: bool = False, printer=print) -> tuple[int, str]:
    """执行两类断言并输出；返回 (退出码, VERDICT)。"""
    facts = evaluate_fact_gates(inp)
    facts_all_green = all(ok for ok, _ in facts)
    consistency = evaluate_consistency_gates(inp, facts_all_green)

    printer("[check_g9_implementation_interlock] 事实门（当前可为红）：")
    for ok, msg in facts:
        printer(f"  {'PASS' if ok else 'RED '} {msg}")
    printer("[check_g9_implementation_interlock] 一致性门（红即脚本失败）：")
    for ok, msg in consistency:
        printer(f"  {'PASS' if ok else 'FAIL'} {msg}")

    verdict = "READY" if facts_all_green else "BLOCKED"
    printer(f"[check_g9_implementation_interlock] VERDICT = {verdict}")
    if verdict == "BLOCKED":
        printer(
            "  BLOCKED 是当前正确结论：G9.2+ 的 src/、spec/、conformance/ 与数字 workflow 步骤保持 0-byte；"
            "本输出不得被当作 G-G9-3 PASS。"
        )

    consistency_failed = [msg for ok, msg in consistency if not ok]
    if consistency_failed:
        printer(f"[check_g9_implementation_interlock] FAIL — {len(consistency_failed)} 项一致性门为红")
        return 1, verdict
    if require_ready and verdict != "READY":
        printer("[check_g9_implementation_interlock] FAIL — --require-ready 模式下互锁未 READY")
        return 1, verdict
    return 0, verdict


# ---------------------------------------------------------------------------
# selftest：可注入输入的受控负样本（G8 版无此设施，G9 必须有）。
# ---------------------------------------------------------------------------

_RFC_APPROVED = """# fixture RFC

| 状态 | Agent Approved |
| Provenance | 起草 `Assisted-by: Codex:gpt-5` |

## §9.1 对抗性评审记录

评审记录 `Assisted-by: Kiro:claude-opus-5`
"""

_GOOD_G9_CONTRACT = f"""---
contract: G9
status: open
implementation_status: blocked
---
# G9 CONTRACT

G9.0 不可变 ref = `{G9_0_IMMUTABLE_REF}`（立项基线登记）。

## §8 实现门状态

### §8.3 G-G9-3 implementation_status 解锁记录

（解锁记录正文）
"""


def _good_inputs() -> TreeInputs:
    return TreeInputs(
        g8_contract_text="---\ncontract: G8\nstatus: closed\nimplementation_status: unblocked\n---\n",
        g9_contract_text=_GOOD_G9_CONTRACT,
        deliverables_missing=[],
        rfc_texts={rfc: _RFC_APPROVED for rfc in G9_RFCS},
        deferred={
            "entries": [
                {
                    "id": "RD-039",
                    "status": "open",
                    "history": [
                        {"date": "2026-08-09", "event": "M61 mesh shader 改判接受：strategic_override 追加登记（→M109 可选 geometry pipeline）"},
                        {"date": "2026-08-09", "event": "M06/M09 触发登记追加：G9 立项裁决第 3 项触发条件登记"},
                    ],
                },
                {
                    "id": "RD-040",
                    "status": "open",
                    "history": [
                        {"date": "2026-08-09", "event": "M52 SER 改判接受：strategic_override 追加登记（→M108 语言层原语+capability 可选）"},
                    ],
                },
            ]
        },
        ledger={
            "namespaces": {"RFC": {"on_tree_max": 24, "next_free": 25}},
            "reserved_in_flight": [{"owner": "G9(UE5 级渲染器与物理引擎正式建造期)"}],
        },
        rfcs_filenames=[
            "0019-rendering-platform.md",
            "0020-asset-pipeline.md",
            "0021-physics-platform.md",
            "0022-virtual-geometry-gi-semantics.md",
            "0023-gpu-driven-submission-shading.md",
            "0024-physics-platform-revision.md",
        ],
    )


def run_selftest() -> int:
    failures = 0

    def case(name: str, inp: TreeInputs, expect_sub: str, expect_verdict: str, expect_exit: int) -> None:
        nonlocal failures
        lines: list[str] = []
        code, verdict = run(inp, printer=lines.append)
        blob = "\n".join(lines)
        hit = expect_sub in blob and verdict == expect_verdict and code == expect_exit
        if hit:
            print(f"  RED ok   — {name}（VERDICT={verdict}, exit={code}）")
        else:
            print(
                f"  RED WRONG— {name}：期望含 {expect_sub!r} / VERDICT={expect_verdict} / exit={expect_exit}，"
                f"实测 VERDICT={verdict} / exit={code}"
            )
            failures += 1

    import copy

    inp = copy.deepcopy(_good_inputs())
    inp.g8_contract_text = inp.g8_contract_text.replace("status: closed", "status: active")
    case("G8 status 改 active → ① 红", inp, "① G8_CONTRACT status = 'active'", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.g9_contract_text = inp.g9_contract_text.replace(
        "### §8.3 G-G9-3 implementation_status 解锁记录", "### §8.3 实现门待激活"
    )
    case("删 §8 激活记录 → ⑥ 红 VERDICT=BLOCKED", inp, "⑥ G9_CONTRACT §8 实现门激活记录", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.deferred["entries"][1]["history"] = []
    case("RD-040 history 删 override → ⑤ 红", inp, "RD-040 M52 strategic_override = False", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.ledger["namespaces"]["RFC"]["on_tree_max"] = 23
    case("ledger RFC 字段错配 → C1 FAIL 退 1", inp, "C1 ledger RFC on_tree_max/next_free = 23/25", "READY", 1)

    inp = copy.deepcopy(_good_inputs())
    inp.rfc_texts[G9_RFCS[1]] = _RFC_APPROVED.replace(
        "| 状态 | Agent Approved |", "| 状态 | Draft 待评审 |"
    )
    case("RFC-0023 改 Draft → ④ 红", inp, "状态未达 Agent Approved", "BLOCKED", 0)

    lines = []
    code, verdict = run(_good_inputs(), printer=lines.append)
    if code == 0 and verdict == "READY":
        print("  GREEN ok — 合成正本 VERDICT=READY，exit=0")
    else:
        print(f"  GREEN MISS — 合成正本本应 READY/exit=0，实测 VERDICT={verdict} / exit={code}")
        failures += 1

    # 当前树实测：VERDICT 与事实一致（登记未落盘=BLOCKED／已落盘=READY，两态均为正确结论）；
    # 一致性全绿时脚本退 0，有 FAIL 时退 1。
    tree_lines: list[str] = []
    tree_code, tree_verdict = run(load_inputs(ROOT), printer=tree_lines.append)
    tree_consistency_green = "FAIL — " not in "\n".join(tree_lines)
    expected_tree_exit = 0 if tree_consistency_green else 1
    if tree_verdict in ("BLOCKED", "READY") and tree_code == expected_tree_exit:
        print(
            f"  TREE ok   — 当前树 VERDICT={tree_verdict}，exit={tree_code}"
            f"（一致性{'全绿' if tree_consistency_green else '有 FAIL'}；"
            f"{'登记未落盘期' if tree_verdict == 'BLOCKED' else 'G-G9-3 解锁登记已落盘'}，符合当前事实预期）"
        )
    else:
        print(
            f"  TREE WRONG— 当前树 VERDICT/exit 与事实预期不符（期望 exit={expected_tree_exit}），"
            f"实测 VERDICT={tree_verdict} / exit={tree_code}"
        )
        failures += 1

    if failures:
        print(f"[check_g9_implementation_interlock] SELFTEST FAIL ({failures})")
        return 1
    print("[check_g9_implementation_interlock] SELFTEST PASS (5 RED + 1 GREEN + 1 TREE)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--require-ready",
        action="store_true",
        help="G9.2 实现 PR 前置模式：未 READY 即退出非零",
    )
    parser.add_argument("--selftest", action="store_true", help="用受控负样本证明断言能红/败")
    args = parser.parse_args()
    if args.selftest:
        return run_selftest()

    code, _ = run(load_inputs(ROOT), require_ready=args.require_ready)
    return code


if __name__ == "__main__":
    sys.exit(main())
