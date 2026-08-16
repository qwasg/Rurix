#!/usr/bin/env python3
# Assisted-by: Kimi-K3（G10.1 治理波 validator；G10.4b 波 C3/C4 两态校准；G10.8b 波 closed 三态校准）
"""G10.2 实现互锁守卫（milestones/g10/CI_GATES.md §3 `g10.gov.implementation_interlock`）。

读取事实源并逐项输出，最后给出 READY / BLOCKED。两类断言严格分开：

* **事实门**（当前允许为红，红即 BLOCKED，不是脚本失败）：
  ① G10_CONTRACT 在树且 front matter status == active；
  ② G10.1 治理交付五件齐备（G10_PLAN / G10_CAPABILITY_MATRIX / G10_CANDIDATE_DECISIONS /
    G10_ACCEPTANCE_MAP / CI_GATES）；
  ③ g10_budget.json 非空且 budget_eval 可加载（namespace=g10、entries 非空、零 estimated、
    每条 entry 的 evidence_file 在树且可解出 results.trimmed_mean）；
  ④ RFC-0026/0027 均在树且 Agent Approved，且 §9.1 有 ≠ 起草 provenance 的独立评审记录
    （D-409；当前两份均为 Draft，本门必然 RED）；
  ⑤ registry/number_ledger.json reserved_in_flight 存在 owner 以 "G10" 开头的登记行；
  ⑥ check_g10_acceptance_map 三向比对 PASS（MAP §1/§2 ↔ CONTRACT §4.2 ↔ CI_GATES §4/§4A）。
* **一致性门**（红即脚本 FAIL，退出非零）：
  C1 双状态诚实：implementation_status == blocked，或事实门全绿（禁止治理完成冒充实现开工）；
  C2 §8 记录一致：§8 出现 G-G10-3 解锁记录 ⇔ 事实门全绿且 implementation_status != blocked
    （事实未全绿时落解锁记录 / 已解锁但无记录均为 FAIL）；
  C3 数字步骤零预占：milestones/g10 全域无 numeric_step 数字赋值、workflow 无 g10.* key /
    ci/g10_ 脚本引用、ci/ 无 g10_*_smoke.py 预放；
  C4 src/spec/conformance 0-byte（治理期）：三面无任何 g10 字面引用、无 g10 命名文件。

**C3/C4 两态口径（G10.4b 校准，判据语义 0-byte）**：C3/C4 是**治理期口径**——
implementation_status == blocked 时维持原机核（预占/三面命中即 FAIL）；
implementation_status != blocked（已解锁）时 C3/C4 自动不适用（实现波合法
materialize 数字步骤/workflow/ci 脚本与 src/spec/conformance 面，机核必然
命中而非违例），输出行登记 skipped_reason 并按通过处理；blocked 态恢复
原机核。G9 §8.1 TREE 臂两态先例同构（selftest 红绿臂实证两态）。

**closed 三态口径（G10.8b 校准，沿 C3/C4 两态先例，判据语义 0-byte）**：
front matter status == closed（G10.8b close-out READY 后 status flip 的收口
终态）时，本守卫回答的问题「G10.2 可否开工」不再适用——互锁使命完结，
事实门/一致性门整体不适用（skipped_reason 登记），VERDICT=CLOSED、exit=0；
active/blocked 态恢复原机核逐字维持。CLOSED 是终态正确结论，不得被当作
G-G10-3 重新开放凭据（契约 reopen 须新立项治理程序）。

诚实纪律：BLOCKED 是当前正确结论，不得被当作 G-G10-3 PASS；`--require-ready`
供未来 G10.2 实现 PR 作前置 required check（未 READY 即退出非零）。
`--selftest` 用可注入输入的受控负样本证明每组断言都能红/败。

事实门与一致性门的计算全部为纯函数（evaluate_fact_gates / evaluate_consistency_gates），
输入经 TreeInputs 注入，缺文件优雅降级为 RED/FAIL，绝不 traceback。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))

G10_CONTRACT = ROOT / "milestones/g10/G10_CONTRACT.md"
LEDGER = ROOT / "registry/number_ledger.json"
BUDGET = ROOT / "milestones/g10/g10_budget.json"
WORKFLOWS = ROOT / ".github/workflows"

G10_1_DELIVERABLES = [
    "milestones/g10/G10_PLAN.md",
    "milestones/g10/G10_CAPABILITY_MATRIX.md",
    "milestones/g10/G10_CANDIDATE_DECISIONS.md",
    "milestones/g10/G10_ACCEPTANCE_MAP.md",
    "milestones/g10/CI_GATES.md",
]
G10_RFCS = [
    "rfcs/0026-visual-comparison-metrics.md",
    "rfcs/0027-external-reference-harness-license.md",
]

# 数字步骤零预占扫描面：numeric_step 数字赋值 / g10 数据行末列裸数字 / workflow g10 token。
NUMERIC_ASSIGN_RE = re.compile(r"numeric_step[\"']?\s*[:=]\s*[\"']?\d")
NUMERIC_TAIL_CELL_RE = re.compile(r"\|\s*\d{1,4}\s*\|\s*$")
WORKFLOW_G10_RE = re.compile(r"g10\.[pw]|ci/g10_")
IMPL_SURFACE_G10_RE = re.compile(r"\bg10\b", re.IGNORECASE)


@dataclass
class TreeInputs:
    """互锁守卫的全部可注入输入；None / 空表示树上缺失。"""

    g10_contract_text: str | None = None
    deliverables_missing: list[str] = field(default_factory=list)
    budget_doc: dict | None = None
    budget_evidence_ok: dict[str, bool] = field(default_factory=dict)
    rfc_texts: dict[str, str | None] = field(default_factory=dict)
    ledger: dict | None = None
    acceptance_findings: list[str] | None = None
    numeric_step_violations: list[str] = field(default_factory=list)
    workflow_g10_hits: list[str] = field(default_factory=list)
    g10_smoke_scripts: list[str] = field(default_factory=list)
    impl_surface_hits: list[str] = field(default_factory=list)


def _budget_evidence_ok(root: Path, doc: dict | None) -> dict[str, bool]:
    """逐 entry 核验 evidence_file 在树且可解出 results.trimmed_mean（budget_eval 同口径）。"""
    out: dict[str, bool] = {}
    for entry in (doc or {}).get("entries", []):
        ef = entry.get("evidence_file")
        if not ef:
            continue
        ok = False
        path = root / ef
        if path.is_file():
            try:
                results = json.loads(path.read_text(encoding="utf-8")).get("results", {})
                ok = isinstance(results.get("trimmed_mean"), (int, float))
            except (json.JSONDecodeError, OSError):
                ok = False
        out[ef] = ok
    return out


def _scan_numeric_step(root: Path) -> list[str]:
    """milestones/g10 全域数字步骤零预占扫描；返回违例行清单。"""
    hits: list[str] = []
    base = root / "milestones/g10"
    if not base.is_dir():
        return [f"milestones/g10 目录缺失"]
    for path in sorted(base.rglob("*")):
        if not path.is_file() or path.suffix not in (".md", ".json", ".yml", ".yaml"):
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        rel = path.relative_to(root)
        for lineno, line in enumerate(text.splitlines(), 1):
            if NUMERIC_ASSIGN_RE.search(line):
                hits.append(f"{rel}:{lineno} numeric_step 数字赋值：{line.strip()[:80]}")
            elif line.lstrip().startswith(("| **M", "| `g10.")) and NUMERIC_TAIL_CELL_RE.search(line):
                hits.append(f"{rel}:{lineno} 数据行末列裸数字（疑似步骤预占）：{line.strip()[:80]}")
    return hits


def _scan_workflows(root: Path) -> list[str]:
    hits: list[str] = []
    if not WORKFLOWS.is_dir():
        return hits
    for path in sorted(WORKFLOWS.glob("*.yml")) + sorted(WORKFLOWS.glob("*.yaml")):
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for lineno, line in enumerate(text.splitlines(), 1):
            if WORKFLOW_G10_RE.search(line):
                hits.append(f"{path.relative_to(root)}:{lineno} {line.strip()[:80]}")
    return hits


def _scan_impl_surface(root: Path) -> list[str]:
    """src/spec/conformance 治理期 0-byte 扫描：任何 g10 字面引用 / g10 命名文件即违例。"""
    hits: list[str] = []
    for sub in ("src", "spec", "conformance"):
        base = root / sub
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if not path.is_file():
                continue
            rel = path.relative_to(root)
            if "g10" in path.name.lower():
                hits.append(f"{rel}（文件名含 g10）")
                continue
            if path.suffix.lower() not in (".rs", ".md", ".rx", ".toml", ".json", ".txt"):
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                continue
            for lineno, line in enumerate(text.splitlines(), 1):
                if IMPL_SURFACE_G10_RE.search(line):
                    hits.append(f"{rel}:{lineno} {line.strip()[:80]}")
                    break
    return hits


def load_inputs(root: Path) -> TreeInputs:
    """从树上装载输入；缺文件/坏 JSON 一律降级为 None，不 traceback。"""
    def _read(path: Path) -> str | None:
        return path.read_text(encoding="utf-8") if path.exists() else None

    def _read_json(path: Path) -> dict | None:
        try:
            return json.loads(path.read_text(encoding="utf-8")) if path.exists() else None
        except (json.JSONDecodeError, OSError):
            return None

    budget_doc = _read_json(BUDGET)

    acceptance_findings: list[str] | None = None
    map_path = root / "milestones/g10/G10_ACCEPTANCE_MAP.md"
    gates_path = root / "milestones/g10/CI_GATES.md"
    contract_path = root / "milestones/g10/G10_CONTRACT.md"
    if map_path.exists() and gates_path.exists() and contract_path.exists():
        import check_g10_acceptance_map as gam

        acceptance_findings = list(
            gam.check(
                map_path.read_text(encoding="utf-8"),
                contract_path.read_text(encoding="utf-8"),
                gates_path.read_text(encoding="utf-8"),
            )
        )

    return TreeInputs(
        g10_contract_text=_read(G10_CONTRACT),
        deliverables_missing=[p for p in G10_1_DELIVERABLES if not (root / p).exists()],
        budget_doc=budget_doc,
        budget_evidence_ok=_budget_evidence_ok(root, budget_doc),
        rfc_texts={rfc: _read(root / rfc) for rfc in G10_RFCS},
        ledger=_read_json(LEDGER),
        acceptance_findings=acceptance_findings,
        numeric_step_violations=_scan_numeric_step(root),
        workflow_g10_hits=_scan_workflows(root),
        g10_smoke_scripts=[p.name for p in sorted((root / "ci").glob("g10_*_smoke.py"))],
        impl_surface_hits=_scan_impl_surface(root),
    )


def front_matter_field(text: str, field: str) -> str | None:
    m = re.search(rf"^{re.escape(field)}:\s*(\S+)\s*$", text, re.MULTILINE)
    return m.group(1) if m else None


def rfc_review_state_text(text: str | None) -> tuple[bool, str]:
    """(是否 Agent Approved 且有独立 provenance 评审记录, 说明)。同构 G9 逻辑（D-409）。"""
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
    r"^#{2,5}\s*§?8\.\d+[^\n]*(implementation_status 解锁|G-G10-3)", re.MULTILINE
)


def evaluate_budget(doc: dict | None, evidence_ok: dict[str, bool]) -> tuple[bool, str]:
    """事实门③：g10_budget 非空且 budget_eval 可加载（零 estimated）。"""
    if doc is None:
        return False, "g10_budget.json 缺失或不可解析"
    problems: list[str] = []
    if doc.get("namespace") != "g10":
        problems.append(f"namespace={doc.get('namespace')!r}（要求 'g10'）")
    entries = doc.get("entries") or []
    if not entries:
        problems.append("entries 为空")
    for entry in entries:
        eid = entry.get("id", "?")
        if not str(eid).startswith("g10."):
            problems.append(f"{eid} 未带 g10. 前缀")
        if entry.get("evidence") == "estimated":
            problems.append(f"{eid} evidence=estimated（零 estimated 硬约束）")
        if not isinstance(entry.get("threshold"), (int, float)):
            problems.append(f"{eid} 缺数值 threshold")
        if not isinstance(entry.get("measured_value"), (int, float)):
            problems.append(f"{eid} 缺 measured_value")
        ef = entry.get("evidence_file")
        if not ef:
            problems.append(f"{eid} 缺 evidence_file")
        elif not evidence_ok.get(ef, False):
            problems.append(f"{eid} evidence_file 不在树或缺 results.trimmed_mean：{ef}")
    if problems:
        return False, "；".join(problems)
    return True, f"{len(entries)} 条 measured_local 条目全部可加载（零 estimated）"


def evaluate_fact_gates(inp: TreeInputs) -> list[tuple[bool, str]]:
    """事实门：红 = BLOCKED，不是脚本失败。"""
    facts: list[tuple[bool, str]] = []

    status = front_matter_field(inp.g10_contract_text, "status") if inp.g10_contract_text else None
    facts.append((status == "active", f"① G10_CONTRACT status = {status!r}（要求 active）"))

    facts.append(
        (
            not inp.deliverables_missing,
            f"② G10.1 治理交付五件齐备（缺 {inp.deliverables_missing or '无'}）",
        )
    )

    budget_ok, budget_msg = evaluate_budget(inp.budget_doc, inp.budget_evidence_ok)
    facts.append((budget_ok, f"③ g10_budget 非空可加载零 estimated：{budget_msg}"))

    for rfc in G10_RFCS:
        ok, why = rfc_review_state_text(inp.rfc_texts.get(rfc))
        facts.append((ok, f"④ {rfc}：{why}"))

    reserved = (inp.ledger or {}).get("reserved_in_flight", [])
    g10_claim = [r for r in reserved if str(r.get("owner", "")).startswith("G10")]
    facts.append(
        (
            inp.ledger is not None and bool(g10_claim),
            f"⑤ ledger reserved_in_flight 存在 owner 以 \"G10\" 开头的行（实测 {len(g10_claim)} 行）",
        )
    )

    if inp.acceptance_findings is None:
        facts.append((False, "⑥ check_g10_acceptance_map 三向比对：比对源缺失，无法判定"))
    else:
        facts.append(
            (
                not inp.acceptance_findings,
                f"⑥ check_g10_acceptance_map 三向比对 = {'PASS' if not inp.acceptance_findings else f'FAIL（{len(inp.acceptance_findings)} 项）'}",
            )
        )
    return facts


def evaluate_consistency_gates(inp: TreeInputs, facts_all_green: bool) -> list[tuple[bool, str]]:
    """一致性门：红即脚本 FAIL，退出非零。"""
    consistency: list[tuple[bool, str]] = []

    impl_status = front_matter_field(inp.g10_contract_text, "implementation_status") if inp.g10_contract_text else None
    consistency.append(
        (
            impl_status == "blocked" or facts_all_green,
            f"C1 G10_CONTRACT implementation_status = {impl_status!r}；事实门全绿 = {facts_all_green}"
            "（事实未全绿时必须保持 blocked，禁止治理完成冒充实现开工）",
        )
    )

    activated = inp.g10_contract_text is not None and bool(ACTIVATION_RE.search(inp.g10_contract_text))
    consistency.append(
        (
            activated == (facts_all_green and impl_status not in (None, "blocked")),
            f"C2 §8 G-G10-3 解锁记录存在 = {activated}；事实门全绿 = {facts_all_green}、"
            f"implementation_status = {impl_status!r}（双状态与 §8 记录必须一致）",
        )
    )

    preclaim = inp.numeric_step_violations + inp.workflow_g10_hits + inp.g10_smoke_scripts
    if impl_status == "blocked":
        consistency.append(
            (
                not preclaim,
                f"C3 数字步骤零预占：milestones/g10 numeric_step 违例 {len(inp.numeric_step_violations)} 处、"
                f"workflow g10 token {len(inp.workflow_g10_hits)} 处、ci/g10_*_smoke.py 预放 "
                f"{inp.g10_smoke_scripts or '无'}"
                + (f"；首处违例 {preclaim[0]}" if preclaim else ""),
            )
        )

        consistency.append(
            (
                not inp.impl_surface_hits,
                f"C4 src/spec/conformance 治理期 0-byte：g10 字面/命名命中 "
                f"{len(inp.impl_surface_hits)} 处"
                + (f"；首处 {inp.impl_surface_hits[0]}" if inp.impl_surface_hits else ""),
            )
        )
    else:
        # 两态口径（G10.4b 校准，判据语义 0-byte）：已解锁后 C3/C4 治理期口径
        # 自动不适用——实现波合法 materialize 数字步骤/workflow/ci 脚本与
        # src/spec/conformance 面，机核命中非违例；blocked 态恢复原机核。
        consistency.append(
            (
                True,
                f"C3 数字步骤零预占：not_applicable（implementation_status={impl_status!r} 已解锁，"
                f"治理期口径不适用；skipped_reason=实现波合法 materialize，实测 numeric_step 违例 "
                f"{len(inp.numeric_step_violations)} 处 / workflow g10 token {len(inp.workflow_g10_hits)} 处 / "
                f"ci/g10_*_smoke.py {len(inp.g10_smoke_scripts)} 件均为解锁后合法实现面，非预占；"
                "blocked 态恢复原机核，判据语义 0-byte）",
            )
        )
        consistency.append(
            (
                True,
                f"C4 src/spec/conformance 治理期 0-byte：not_applicable（implementation_status={impl_status!r} 已解锁，"
                f"治理期口径不适用；skipped_reason=实现波合法改动三面，实测 g10 字面/命名命中 "
                f"{len(inp.impl_surface_hits)} 处均为解锁后合法实现面，非治理期预放；"
                "blocked 态恢复原机核，判据语义 0-byte）",
            )
        )
    return consistency


def run(inp: TreeInputs, require_ready: bool = False, printer=print) -> tuple[int, str]:
    """执行两类断言并输出；返回 (退出码, VERDICT)。"""
    # closed 三态口径（G10.8b 校准，沿 C3/C4 两态先例，判据语义 0-byte）：
    # status==closed = 收口终态，互锁使命完结，事实门/一致性门整体不适用。
    contract_status = (
        front_matter_field(inp.g10_contract_text, "status") if inp.g10_contract_text else None
    )
    if contract_status == "closed":
        printer(
            "[check_g10_implementation_interlock] 事实门/一致性门：not_applicable"
            "（status='closed' 收口终态，互锁使命完结；skipped_reason=G10.2+ 开工门问题"
            "不再适用——G10.8b close-out READY 后 status flip；active/blocked 态恢复原机核，"
            "判据语义 0-byte）"
        )
        printer("[check_g10_implementation_interlock] VERDICT = CLOSED")
        printer(
            "  CLOSED 是收口终态正确结论：本守卫回答「G10.2 可否开工」，契约 closed 后该问题"
            "不再适用；不得被当作 G-G10-3 重新开放凭据（契约 reopen 须新立项治理程序）。"
        )
        return 0, "CLOSED"
    facts = evaluate_fact_gates(inp)
    facts_all_green = all(ok for ok, _ in facts)
    consistency = evaluate_consistency_gates(inp, facts_all_green)

    printer("[check_g10_implementation_interlock] 事实门（当前可为红）：")
    for ok, msg in facts:
        printer(f"  {'PASS' if ok else 'RED '} {msg}")
    printer("[check_g10_implementation_interlock] 一致性门（红即脚本失败）：")
    for ok, msg in consistency:
        printer(f"  {'PASS' if ok else 'FAIL'} {msg}")

    verdict = "READY" if facts_all_green else "BLOCKED"
    printer(f"[check_g10_implementation_interlock] VERDICT = {verdict}")
    if verdict == "BLOCKED":
        missing = [msg.split("：", 1)[0] for ok, msg in facts if not ok]
        printer(f"  缺项清单：{missing}")
        printer(
            "  BLOCKED 是当前正确结论：G10.2+ 的 src/、spec/、conformance/ 与数字 workflow 步骤保持 0-byte；"
            "本输出不得被当作 G-G10-3 PASS。"
        )

    consistency_failed = [msg for ok, msg in consistency if not ok]
    if consistency_failed:
        printer(f"[check_g10_implementation_interlock] FAIL — {len(consistency_failed)} 项一致性门为红")
        return 1, verdict
    if require_ready and verdict != "READY":
        printer("[check_g10_implementation_interlock] FAIL — --require-ready 模式下互锁未 READY")
        return 1, verdict
    return 0, verdict


# ---------------------------------------------------------------------------
# selftest：可注入输入的受控负样本。
# ---------------------------------------------------------------------------

_RFC_APPROVED = """# fixture RFC

| 状态 | Agent Approved |
| Provenance | 起草 `Assisted-by: Kimi-K3（G10.1 治理波 RFC 起草）` |

## §9.1 对抗性评审记录

评审记录 `Assisted-by: Kiro:claude-opus-5`
"""

_GOOD_G10_CONTRACT = """---
contract: G10
status: active
implementation_status: blocked
---
# G10 CONTRACT

## §8 Implementation activation / Close-out（只追加区）

<!-- 当前不得写 PASS。 -->
"""


def _good_inputs() -> TreeInputs:
    return TreeInputs(
        g10_contract_text=_GOOD_G10_CONTRACT,
        deliverables_missing=[],
        budget_doc={
            "namespace": "g10",
            "entries": [
                {
                    "id": "g10.bench.sr_pipeline_l3_frame_ms",
                    "evidence": "measured_local",
                    "threshold": 1.8102,
                    "measured_value": 1.2068,
                    "evidence_file": "evidence/g10_baseline_sr_pipeline_l3_fixture.json",
                }
            ],
        },
        budget_evidence_ok={"evidence/g10_baseline_sr_pipeline_l3_fixture.json": True},
        rfc_texts={rfc: _RFC_APPROVED for rfc in G10_RFCS},
        ledger={
            "namespaces": {"CI_step": {"on_tree_max": 172, "next_free": 173}},
            "reserved_in_flight": [{"owner": "G10(UE5 画面对标基线期)"}],
        },
        acceptance_findings=[],
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
    inp.g10_contract_text = inp.g10_contract_text.replace("status: active", "status: draft")
    case("G10 status 改 draft → ① 红", inp, "① G10_CONTRACT status = 'draft'", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.deliverables_missing = ["milestones/g10/G10_CAPABILITY_MATRIX.md"]
    case("缺一件治理交付 → ② 红", inp, "② G10.1 治理交付五件齐备（缺 ['milestones/g10/G10_CAPABILITY_MATRIX.md']）", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.budget_doc["entries"][0]["evidence"] = "estimated"
    case("budget 混入 estimated → ③ 红", inp, "evidence=estimated", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.rfc_texts[G10_RFCS[0]] = _RFC_APPROVED.replace(
        "| 状态 | Agent Approved |", "| 状态 | **Draft** 待评审 |"
    )
    case("RFC-0026 改 Draft → ④ 红", inp, "状态未达 Agent Approved", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.ledger["reserved_in_flight"] = [{"owner": "G9(UE5 级渲染器与物理引擎正式建造期)"}]
    case("ledger 无 G10 登记行 → ⑤ 红", inp, '⑤ ledger reserved_in_flight 存在 owner 以 "G10" 开头的行（实测 0 行）', "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.acceptance_findings = ["[three-way] M128 key 漂移"]
    case("三向比对注入漂移 → ⑥ 红", inp, "⑥ check_g10_acceptance_map 三向比对 = FAIL（1 项）", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.numeric_step_violations = ["milestones/g10/CI_GATES.md:70 numeric_step 数字赋值：numeric_step: 173"]
    case("数字步骤预占注入 → C3 FAIL 退 1（VERDICT 不受一致性门影响）", inp, "C3 数字步骤零预占", "READY", 1)

    inp = copy.deepcopy(_good_inputs())
    inp.impl_surface_hits = ["spec/g10_metrics.md（文件名含 g10）"]
    case("spec 面 g10 命中注入 → C4 FAIL 退 1", inp, "C4 src/spec/conformance 治理期 0-byte", "READY", 1)

    inp = copy.deepcopy(_good_inputs())
    inp.g10_contract_text = inp.g10_contract_text.replace(
        "## §8 Implementation activation / Close-out（只追加区）",
        "### §8.1 G-G10-3 implementation_status 解锁记录",
    )
    case("落 §8 解锁记录但 front matter 未翻 → C2 FAIL 退 1", inp, "C2 §8 G-G10-3 解锁记录存在 = True", "READY", 1)

    # 两态口径（G10.4b 校准）：unblocked 态 C3/C4 自动不适用（skipped_reason 登记），
    # 预占/三面命中注入不再构成 FAIL；blocked 态上述两臂已实证原机核维持。
    unblocked_text = _GOOD_G10_CONTRACT.replace(
        "implementation_status: blocked", "implementation_status: unblocked"
    ).replace(
        "## §8 Implementation activation / Close-out（只追加区）",
        "### §8.1 G-G10-3 implementation_status 解锁记录",
    )
    inp = copy.deepcopy(_good_inputs())
    inp.g10_contract_text = unblocked_text
    inp.numeric_step_violations = ["milestones/g10/CI_GATES.md:70 numeric_step 数字赋值：numeric_step: 184"]
    inp.workflow_g10_hits = [".github/workflows/pr-smoke.yml:2000 g10.p0.m135.flip_metric"]
    inp.g10_smoke_scripts = ["g10_flip_metric_smoke.py"]
    inp.impl_surface_hits = ["src/image-io/src/exr.rs:1 G10.4 M134"]
    case(
        "unblocked 态预占/三面命中注入 → C3/C4 not_applicable 退 0（skipped_reason 登记）",
        inp, "skipped_reason", "READY", 0,
    )

    inp = copy.deepcopy(_good_inputs())
    inp.g10_contract_text = unblocked_text
    inp.acceptance_findings = ["[three-way] M135 key 漂移"]
    case("unblocked 态事实门红 → C1 仍 FAIL 退 1（两态校准不遮蔽 C1/C2）", inp, "C1 G10_CONTRACT implementation_status = 'unblocked'", "BLOCKED", 1)

    # closed 三态口径（G10.8b 校准）：status==closed → VERDICT=CLOSED 退 0，
    # 全门 not_applicable + skipped_reason 登记；active/blocked 态原机核维持
    #（上述红绿臂已实证）。
    inp = copy.deepcopy(_good_inputs())
    inp.g10_contract_text = unblocked_text.replace("status: active", "status: closed")
    case(
        "closed 态 → 全门 not_applicable VERDICT=CLOSED 退 0（skipped_reason 登记）",
        inp, "skipped_reason", "CLOSED", 0,
    )

    lines = []
    code, verdict = run(_good_inputs(), printer=lines.append)
    if code == 0 and verdict == "READY":
        print("  GREEN ok — 合成正本 VERDICT=READY，exit=0")
    else:
        print(f"  GREEN MISS — 合成正本本应 READY/exit=0，实测 VERDICT={verdict} / exit={code}")
        failures += 1

    # 当前树实测：VERDICT 与事实一致（RFC Draft/登记未落盘=BLOCKED／全绿=READY／
    # 收口终态=CLOSED，三态均为正确结论）；一致性全绿时脚本退 0，有 FAIL 时退 1。
    tree_lines: list[str] = []
    tree_code, tree_verdict = run(load_inputs(ROOT), printer=tree_lines.append)
    tree_consistency_green = "FAIL — " not in "\n".join(tree_lines)
    expected_tree_exit = 0 if tree_consistency_green else 1
    if tree_verdict in ("BLOCKED", "READY", "CLOSED") and tree_code == expected_tree_exit:
        print(
            f"  TREE ok   — 当前树 VERDICT={tree_verdict}，exit={tree_code}"
            f"（一致性{'全绿' if tree_consistency_green else '有 FAIL'}；"
            f"{'互锁条件未齐期' if tree_verdict == 'BLOCKED' else ('收口终态' if tree_verdict == 'CLOSED' else 'G-G10-3 解锁条件已齐')}，符合当前事实预期）"
        )
    else:
        print(
            f"  TREE WRONG— 当前树 VERDICT/exit 与事实预期不符（期望 exit={expected_tree_exit}），"
            f"实测 VERDICT={tree_verdict} / exit={tree_code}"
        )
        failures += 1

    if failures:
        print(f"[check_g10_implementation_interlock] SELFTEST FAIL ({failures})")
        return 1
    print("[check_g10_implementation_interlock] SELFTEST PASS (12 RED + 1 GREEN + 1 TREE)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--require-ready",
        action="store_true",
        help="G10.2 实现 PR 前置模式：未 READY 即退出非零",
    )
    parser.add_argument("--selftest", action="store_true", help="用受控负样本证明断言能红/败")
    args = parser.parse_args()
    if args.selftest:
        return run_selftest()

    code, _ = run(load_inputs(ROOT), require_ready=args.require_ready)
    return code


if __name__ == "__main__":
    sys.exit(main())
