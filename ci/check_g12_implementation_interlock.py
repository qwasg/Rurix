#!/usr/bin/env python3
# Assisted-by: Kimi-K3（G12.1 治理波 validator）
"""G12.2 实现互锁守卫（milestones/g12/CI_GATES.md §3 `g12.gov.implementation_interlock`）。

读取事实源并逐项输出，最后给出 READY / BLOCKED / CLOSED。两类断言严格分开：

* **事实门**（当前允许为红，红即 BLOCKED，不是脚本失败；CI_GATES §1.1 四项）：
  ① G11_CONTRACT §8.8 有效 status == closed（G11.7b close-out flip 终态，§8.8 签署块
    在位），且 G12.0 不可变 ref `5ae83aa7` 已登记（G12_CONTRACT §7 立项裁决字面）；
  ② RFC-0029（G12 路径追踪生产化伞形）在树且 Agent Approved，且 §9.1 有 ≠ 起草
    provenance 的独立评审记录（D-409）；
  ③ G12_CANDIDATE_DECISIONS 无空行（§1 G11 defer 19 行 + §2 open RD 7 行 + §3 新增
    候选 11 行全数在位，全部数据行零空串/占位单元格）+ registry/deferred.json
    history 只追加无静默改判（对 G12.0 不可变 ref 5ae83aa7 base：条目四字段
    id/title/reason/backfill_condition 与 status/owner_milestone 0-byte、history
    前缀只追加、条目零删除）+ G12_ACCEPTANCE_MAP §1 八行 P0（M158~M165 集合全等，
    key/script/schema 路径/判据列无缺）与 §2 一行 P1（M166）无缺行；
  ④ 用户 G12.2 开工指令留痕（2026-08-15 指令全期授权面——「支持 dlss、超分采样、
    路径追踪等前沿技术」字面在 G12_CONTRACT §7）+ workflow 实测末号与 ledger
    namespaces.CI_step on_tree_max 一致且 next_free == on_tree_max + 1。
* **一致性门**（红即脚本 FAIL，退出非零）：
  C1 双状态诚实：implementation_status == blocked，或事实门全绿（禁止治理完成冒充实现开工）；
  C2 §8 记录一致：§8 出现 G-G12-3 解锁记录 ⇔ 事实门全绿且 implementation_status != blocked
    （事实未全绿时落解锁记录 / 已解锁但无记录均为 FAIL）；
  C3 数字步骤零预占：milestones/g12 全域无 numeric_step 数字赋值、workflow 无 g12.* key /
    ci/g12_ 脚本引用、ci/ 无 g12_*_smoke.py 预放；
  C4 src/spec/conformance 0-byte（治理期）：三面无 g12 实现面命中。

**C3/C4 两态口径（沿 G10.4b/G11 先例，判据语义 0-byte）**：C3/C4 是**治理期口径**——
implementation_status == blocked 时维持原机核（预占/三面命中即 FAIL）；
implementation_status != blocked（已解锁）时 C3/C4 自动不适用（实现波合法
materialize 数字步骤/workflow/ci 脚本与 src/spec/conformance 面，机核必然
命中而非违例），输出行登记 skipped_reason 并按通过处理；blocked 态恢复
原机核。G9 §8.1 TREE 臂两态先例同构（selftest 红绿臂实证两态）。

**C4 扫描面校准（G12.1 立项实测登记）**：2026-08-17 实测 spec/ 与 conformance/
全域大小写不敏感 `g12` 零命中、src/ 零 gate-key token 命中，裸字面「G12」当前
无合法存续面；为与 G10.4b/G11 判据语义同构（防未来合法存续面误伤），C4 不扫裸
里程碑名，扫描面 = gate-key/脚本命名 token（`g12.p0.` / `g12.p1.` / `g12.wave.` /
`ci/g12_`）+ g12 命名文件——命中即 G12 实现面预放。

**closed 三态口径（沿 G10.8b/G11 先例，判据语义 0-byte）**：
front matter status == closed（G12.7b close-out READY 后 status flip 的收口
终态）时，本守卫回答的问题「G12.2 可否开工」不再适用——互锁使命完结，
事实门/一致性门整体不适用（skipped_reason 登记），VERDICT=CLOSED、exit=0；
active/blocked 态恢复原机核逐字维持。CLOSED 是终态正确结论，不得被当作
G-G12-3 重新开放凭据（契约 reopen 须新立项治理程序）。

诚实纪律：BLOCKED 是当前正确结论，不得被当作 G-G12-3 PASS；`--require-ready`
供未来 G12.2 实现 PR 作前置 required check（未 READY 即退出非零）。
`--selftest` 用可注入输入的受控负样本证明每组断言都能红/败。

事实门与一致性门的计算全部为纯函数（evaluate_fact_gates / evaluate_consistency_gates），
输入经 TreeInputs 注入，缺文件优雅降级为 RED/FAIL，绝不 traceback。
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))

G12_CONTRACT = ROOT / "milestones/g12/G12_CONTRACT.md"
G11_CONTRACT = ROOT / "milestones/g11/G11_CONTRACT.md"
G12_CANDIDATES = ROOT / "milestones/g12/G12_CANDIDATE_DECISIONS.md"
G12_MAP = ROOT / "milestones/g12/G12_ACCEPTANCE_MAP.md"
LEDGER = ROOT / "registry/number_ledger.json"
DEFERRED = ROOT / "registry/deferred.json"
WORKFLOWS = ROOT / ".github/workflows"

G12_0_IMMUTABLE_REF = "5ae83aa7"  # G12.0 文档集不可变 ref（G11.7a 回归刷新批 HEAD）
G12_USER_INSTRUCTION_LITERAL = "支持 dlss、超分采样、路径追踪等前沿技术"
G12_RFCS = [
    "rfcs/0029-g12-path-tracer-productionization.md",
]

# 候选决策表行闭集（§1 G11 defer 19 行 + §2 open RD 7 行 + §3 新增候选 11 行 = 37 行）。
CANDIDATE_ROW_IDS = [
    # §1：G11 defer-to-G12+ 19 行
    "M61", "M52", "M100-high", "SAFE-GPU", "M127", "M98-l4", "M114-strand",
    "M118-hdr-cal", "M125-adopt3", "G10-N5", "G10-N6", "G10-N8", "G10-N11",
    "G10-N16", "G10-N17", "G11-N3", "G11-N5", "G11-N8", "G11-N9",
    # §2：open RD 7 行
    "RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044",
    # §3：G12 新增候选 11 行
    "G12-N1", "G12-N2", "G12-N3", "G12-N4", "G12-N5", "G12-N6",
    "G12-N7", "G12-N8", "G12-N9", "G12-N10", "G12-N11",
]
# 空行门占位单元格闭集（MAP §5 no-empty 口径）。
PLACEHOLDER_CELLS = {"", "TBD", "TODO", "待定", "待补", "—"}

# 数字步骤零预占扫描面：numeric_step 数字赋值 / g12 数据行末列裸数字 / workflow g12 token。
NUMERIC_ASSIGN_RE = re.compile(r"numeric_step[\"']?\s*[:=]\s*[\"']?\d")
NUMERIC_TAIL_CELL_RE = re.compile(r"\|\s*\d{1,4}\s*\|\s*$")
WORKFLOW_G12_RE = re.compile(r"g12\.[pw]|ci/g12_")
WORKFLOW_STEP_RE = re.compile(r"步骤\s*(\d{1,4})")
# C4 扫描面（G12.1 校准）：gate-key/脚本命名 token + g12 命名文件。
IMPL_SURFACE_G12_RE = re.compile(r"g12\.p[01]\.|g12\.wave\.|ci/g12_")
# MAP §1 P0 行首单元格形态：| **M158** | ...。
MAP_ROW_RE = re.compile(r"^\|\s*\*\*(M\d{3})\*\*")
MAP_KEY_RE = re.compile(r"g12\.p([01])\.m(\d{3})\.[a-z0-9_]+")
MAP_SCRIPT_RE = re.compile(r"ci/g12_[a-z0-9_]+_smoke\.py")
MAP_SCHEMA_RE = re.compile(r"milestones/g12/g12_m\d{3}_[a-z0-9_]+_evidence_schema\.json")


@dataclass
class TreeInputs:
    """互锁守卫的全部可注入输入；None / 空表示树上缺失或无法判定。"""

    g12_contract_text: str | None = None
    g11_contract_text: str | None = None
    rfc_texts: dict[str, str | None] = field(default_factory=dict)
    candidate_findings: list[str] | None = None
    deferred_findings: list[str] | None = None
    map_findings: list[str] | None = None
    ledger: dict | None = None
    workflow_max_step: int | None = None
    numeric_step_violations: list[str] = field(default_factory=list)
    workflow_g12_hits: list[str] = field(default_factory=list)
    g12_smoke_scripts: list[str] = field(default_factory=list)
    impl_surface_hits: list[str] = field(default_factory=list)


def _table_data_rows(text: str) -> list[list[str]]:
    """抽取全部 markdown 表格数据行（跳表头与分隔行），返回单元格列表的行列表。"""
    rows: list[list[str]] = []
    block: list[list[str]] = []

    def flush() -> None:
        nonlocal block
        if len(block) >= 2 and all(re.fullmatch(r":?-{2,}:?", c) for c in block[1]):
            rows.extend(block[2:])
        block = []

    for line in text.splitlines():
        s = line.strip()
        if s.startswith("|"):
            block.append([c.strip() for c in s.strip("|").split("|")])
        else:
            flush()
    flush()
    return rows


def check_candidate_decisions(text: str | None) -> list[str]:
    """G12_CANDIDATE_DECISIONS 无空行：37 行闭集全数在位 + 全部数据行零空串/占位单元格。"""
    if text is None:
        return ["G12_CANDIDATE_DECISIONS.md 缺失"]
    findings: list[str] = []
    rows = _table_data_rows(text)
    first_cells = {r[0] for r in rows if r}
    for rid in CANDIDATE_ROW_IDS:
        if rid not in first_cells:
            findings.append(f"候选决策表缺行 {rid}")
    for r in rows:
        rid = r[0] if r else "?"
        for cell in r:
            if cell in PLACEHOLDER_CELLS:
                findings.append(f"行 {rid} 存在空串/占位单元格")
                break
    return findings


def check_deferred_append_only(base_doc: dict | None, current_doc: dict | None) -> list[str] | None:
    """deferred.json history 只追加核验（vs G12.0 base）：条目零删除、条目级字段 0-byte、
    history 前缀只追加。base 不可取得时返回 None（无法判定，诚实降级为 RED）。"""
    if base_doc is None or current_doc is None:
        return None
    findings: list[str] = []
    base_entries = {e.get("id"): e for e in base_doc.get("entries", [])}
    cur_entries = {e.get("id"): e for e in current_doc.get("entries", [])}
    removed = sorted(set(base_entries) - set(cur_entries))
    if removed:
        findings.append(f"deferred 条目被删除: {removed}")
    for rid, be in base_entries.items():
        ce = cur_entries.get(rid)
        if ce is None:
            continue
        for f in ("title", "reason", "backfill_condition", "status", "owner_milestone"):
            if ce.get(f) != be.get(f):
                findings.append(f"{rid} 字段 {f} 被改写（静默改判/0-byte 违例）")
        bh = be.get("history", [])
        ch = ce.get("history", [])
        if len(ch) < len(bh) or ch[: len(bh)] != bh:
            findings.append(
                f"{rid} history 非只追加（base {len(bh)} 条 vs current {len(ch)} 条前缀不等）"
            )
    return findings


def check_acceptance_map(text: str | None) -> list[str]:
    """G12_ACCEPTANCE_MAP §1 八行 P0（M158~M165 集合全等）+ §2 一行 P1（M166）无缺行。"""
    if text is None:
        return ["G12_ACCEPTANCE_MAP.md 缺失"]
    findings: list[str] = []
    sections = re.split(r"(?m)^## ", text)
    sec1 = next((s for s in sections if s.startswith("1. ")), "")
    sec2 = next((s for s in sections if s.startswith("2. ")), "")
    if not sec1:
        findings.append("MAP §1 缺失")
    if not sec2:
        findings.append("MAP §2 缺失")

    def _rows(section: str) -> dict[str, list[str]]:
        out: dict[str, list[str]] = {}
        for line in section.splitlines():
            m = MAP_ROW_RE.match(line.strip())
            if m:
                out[m.group(1)] = [c.strip() for c in line.strip().strip("|").split("|")]
        return out

    p0_rows = _rows(sec1)
    p1_rows = _rows(sec2)
    expect_p0 = {f"M{n}" for n in range(158, 166)}
    if set(p0_rows) != expect_p0:
        findings.append(
            f"MAP §1 P0 集合 {sorted(p0_rows)} ≠ 闭集 {sorted(expect_p0)}（缺行/多行）"
        )
    if set(p1_rows) != {"M166"}:
        findings.append(f"MAP §2 P1 集合 {sorted(p1_rows)} ≠ {{'M166'}}（缺行/多行）")
    for mid, cells in {**p0_rows, **p1_rows}.items():
        num = mid[1:]
        blob = "|".join(cells)
        km = MAP_KEY_RE.search(blob)
        if not km or km.group(2) != num:
            findings.append(f"MAP {mid} 行 symbolic key 缺失或 M 号不符")
        elif mid == "M166" and km.group(1) != "1":
            findings.append(f"MAP {mid} 行 key 非 g12.p1 命名空间")
        elif mid != "M166" and km.group(1) != "0":
            findings.append(f"MAP {mid} 行 key 非 g12.p0 命名空间")
        if not MAP_SCRIPT_RE.search(blob):
            findings.append(f"MAP {mid} 行缺稳定脚本 ci/g12_*_smoke.py")
        if not MAP_SCHEMA_RE.search(blob):
            findings.append(f"MAP {mid} 行缺 evidence schema 目标路径")
        for cell in cells:
            if cell in PLACEHOLDER_CELLS:
                findings.append(f"MAP {mid} 行存在空串/占位单元格")
                break
    return findings


def _scan_numeric_step(root: Path) -> list[str]:
    """milestones/g12 全域数字步骤零预占扫描；返回违例行清单。"""
    hits: list[str] = []
    base = root / "milestones/g12"
    if not base.is_dir():
        return ["milestones/g12 目录缺失"]
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
            elif line.lstrip().startswith(("| **M", "| `g12.")) and NUMERIC_TAIL_CELL_RE.search(line):
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
            if WORKFLOW_G12_RE.search(line):
                hits.append(f"{path.relative_to(root)}:{lineno} {line.strip()[:80]}")
    return hits


def _scan_workflow_max_step(root: Path) -> int | None:
    """workflow 注释面实测末号（`步骤 NNN` 最大值）；无命中返回 None。"""
    max_step: int | None = None
    if not WORKFLOWS.is_dir():
        return None
    for path in sorted(WORKFLOWS.glob("*.yml")) + sorted(WORKFLOWS.glob("*.yaml")):
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for m in WORKFLOW_STEP_RE.finditer(text):
            n = int(m.group(1))
            max_step = n if max_step is None else max(max_step, n)
    return max_step


def _scan_impl_surface(root: Path) -> list[str]:
    """src/spec/conformance 治理期 0-byte 扫描：g12 实现面 token / g12 命名文件即违例。"""
    hits: list[str] = []
    for sub in ("src", "spec", "conformance"):
        base = root / sub
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if not path.is_file():
                continue
            rel = path.relative_to(root)
            if "g12" in path.name.lower():
                hits.append(f"{rel}（文件名含 g12）")
                continue
            if path.suffix.lower() not in (".rs", ".md", ".rx", ".toml", ".json", ".txt"):
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                continue
            for lineno, line in enumerate(text.splitlines(), 1):
                if IMPL_SURFACE_G12_RE.search(line):
                    hits.append(f"{rel}:{lineno} {line.strip()[:80]}")
                    break
    return hits


def _git_show_file(root: Path, ref: str, rel: str) -> str | None:
    """读取 base ref 上的文件内容；git/ref/文件缺失一律降级 None，不 traceback。"""
    try:
        r = subprocess.run(
            ["git", "show", f"{ref}:{rel}"],
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError:
        return None
    return r.stdout if r.returncode == 0 else None


def load_inputs(root: Path) -> TreeInputs:
    """从树上装载输入；缺文件/坏 JSON 一律降级为 None，不 traceback。"""
    def _read(path: Path) -> str | None:
        return path.read_text(encoding="utf-8") if path.exists() else None

    def _read_json(path: Path) -> dict | None:
        try:
            return json.loads(path.read_text(encoding="utf-8")) if path.exists() else None
        except (json.JSONDecodeError, OSError):
            return None

    deferred_current = _read_json(DEFERRED)
    deferred_base: dict | None = None
    base_text = _git_show_file(root, G12_0_IMMUTABLE_REF, "registry/deferred.json")
    if base_text is not None:
        try:
            deferred_base = json.loads(base_text)
        except json.JSONDecodeError:
            deferred_base = None

    return TreeInputs(
        g12_contract_text=_read(G12_CONTRACT),
        g11_contract_text=_read(G11_CONTRACT),
        rfc_texts={rfc: _read(root / rfc) for rfc in G12_RFCS},
        candidate_findings=check_candidate_decisions(_read(G12_CANDIDATES)),
        deferred_findings=check_deferred_append_only(deferred_base, deferred_current),
        map_findings=check_acceptance_map(_read(G12_MAP)),
        ledger=_read_json(LEDGER),
        workflow_max_step=_scan_workflow_max_step(root),
        numeric_step_violations=_scan_numeric_step(root),
        workflow_g12_hits=_scan_workflows(root),
        g12_smoke_scripts=[p.name for p in sorted((root / "ci").glob("g12_*_smoke.py"))],
        impl_surface_hits=_scan_impl_surface(root),
    )


def front_matter_field(text: str, field: str) -> str | None:
    m = re.search(rf"^{re.escape(field)}:\s*(\S+)\s*$", text, re.MULTILINE)
    return m.group(1) if m else None


def rfc_review_state_text(text: str | None) -> tuple[bool, str]:
    """(是否 Agent Approved 且有独立 provenance 评审记录, 说明)。同构 G10/G11 逻辑（D-409）。"""
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
    r"^#{2,5}\s*§?8\.\d+[^\n]*(implementation_status 解锁|G-G12-3)", re.MULTILINE
)


def evaluate_fact_gates(inp: TreeInputs) -> list[tuple[bool, str]]:
    """事实门：红 = BLOCKED，不是脚本失败。"""
    facts: list[tuple[bool, str]] = []

    g11_status = front_matter_field(inp.g11_contract_text, "status") if inp.g11_contract_text else None
    g11_closeout_section = bool(inp.g11_contract_text and "§8.8" in inp.g11_contract_text)
    g12_ref_registered = bool(inp.g12_contract_text and G12_0_IMMUTABLE_REF in inp.g12_contract_text)
    facts.append(
        (
            g11_status == "closed" and g11_closeout_section and g12_ref_registered,
            f"① G11_CONTRACT status = {g11_status!r}（要求 closed）且 §8.8 签署块在位 = {g11_closeout_section}；"
            f"G12.0 不可变 ref {G12_0_IMMUTABLE_REF} 登记 = {g12_ref_registered}",
        )
    )

    for rfc in G12_RFCS:
        ok, why = rfc_review_state_text(inp.rfc_texts.get(rfc))
        facts.append((ok, f"② {rfc}：{why}"))

    sub3: list[str] = []
    ok3 = True
    if inp.candidate_findings is None:
        ok3 = False
        sub3.append("G12_CANDIDATE_DECISIONS 缺失")
    elif inp.candidate_findings:
        ok3 = False
        sub3.append(f"候选决策表 {len(inp.candidate_findings)} 项缺/空（首项 {inp.candidate_findings[0]}）")
    else:
        sub3.append("候选决策表 37 行零空行")
    if inp.deferred_findings is None:
        ok3 = False
        sub3.append("deferred.json base 不可取得，只追加无法判定")
    elif inp.deferred_findings:
        ok3 = False
        sub3.append(f"deferred history {len(inp.deferred_findings)} 项违例（首项 {inp.deferred_findings[0]}）")
    else:
        sub3.append("deferred history 只追加（vs G12.0 base 四字段 0-byte）")
    if inp.map_findings is None:
        ok3 = False
        sub3.append("G12_ACCEPTANCE_MAP 缺失")
    elif inp.map_findings:
        ok3 = False
        sub3.append(f"验收映射 {len(inp.map_findings)} 项缺行（首项 {inp.map_findings[0]}）")
    else:
        sub3.append("验收映射 §1 八行 P0 + §2 一行 P1 无缺行")
    facts.append((ok3, f"③ 决策表/ deferred/ 验收映射三面：{'；'.join(sub3)}"))

    instruction_recorded = bool(
        inp.g12_contract_text and G12_USER_INSTRUCTION_LITERAL in inp.g12_contract_text
    )
    ci_step = (inp.ledger or {}).get("namespaces", {}).get("CI_step", {})
    on_tree_max = ci_step.get("on_tree_max")
    next_free = ci_step.get("next_free")
    ledger_consistent = (
        inp.ledger is not None
        and isinstance(on_tree_max, int)
        and isinstance(next_free, int)
        and inp.workflow_max_step is not None
        and inp.workflow_max_step == on_tree_max
        and next_free == on_tree_max + 1
    )
    facts.append(
        (
            instruction_recorded and ledger_consistent,
            f"④ 用户 G12.2 开工指令留痕（2026-08-15 全期授权面「{G12_USER_INSTRUCTION_LITERAL}」字面）"
            f" = {instruction_recorded}；workflow 实测末号 = {inp.workflow_max_step}、"
            f"ledger CI_step on_tree_max = {on_tree_max}、next_free = {next_free}"
            f"（一致 = {ledger_consistent}）",
        )
    )
    return facts


def evaluate_consistency_gates(inp: TreeInputs, facts_all_green: bool) -> list[tuple[bool, str]]:
    """一致性门：红即脚本 FAIL，退出非零。"""
    consistency: list[tuple[bool, str]] = []

    impl_status = front_matter_field(inp.g12_contract_text, "implementation_status") if inp.g12_contract_text else None
    consistency.append(
        (
            impl_status == "blocked" or facts_all_green,
            f"C1 G12_CONTRACT implementation_status = {impl_status!r}；事实门全绿 = {facts_all_green}"
            "（事实未全绿时必须保持 blocked，禁止治理完成冒充实现开工）",
        )
    )

    activated = inp.g12_contract_text is not None and bool(ACTIVATION_RE.search(inp.g12_contract_text))
    consistency.append(
        (
            activated == (facts_all_green and impl_status not in (None, "blocked")),
            f"C2 §8 G-G12-3 解锁记录存在 = {activated}；事实门全绿 = {facts_all_green}、"
            f"implementation_status = {impl_status!r}（双状态与 §8 记录必须一致）",
        )
    )

    preclaim = inp.numeric_step_violations + inp.workflow_g12_hits + inp.g12_smoke_scripts
    if impl_status == "blocked":
        consistency.append(
            (
                not preclaim,
                f"C3 数字步骤零预占：milestones/g12 numeric_step 违例 {len(inp.numeric_step_violations)} 处、"
                f"workflow g12 token {len(inp.workflow_g12_hits)} 处、ci/g12_*_smoke.py 预放 "
                f"{inp.g12_smoke_scripts or '无'}"
                + (f"；首处违例 {preclaim[0]}" if preclaim else ""),
            )
        )

        consistency.append(
            (
                not inp.impl_surface_hits,
                f"C4 src/spec/conformance 治理期 0-byte：g12 实现面 token/命名命中 "
                f"{len(inp.impl_surface_hits)} 处"
                + (f"；首处 {inp.impl_surface_hits[0]}" if inp.impl_surface_hits else ""),
            )
        )
    else:
        # 两态口径（沿 G10.4b/G11 先例，判据语义 0-byte）：已解锁后 C3/C4 治理期口径
        # 自动不适用——实现波合法 materialize 数字步骤/workflow/ci 脚本与
        # src/spec/conformance 面，机核命中非违例；blocked 态恢复原机核。
        consistency.append(
            (
                True,
                f"C3 数字步骤零预占：not_applicable（implementation_status={impl_status!r} 已解锁，"
                f"治理期口径不适用；skipped_reason=实现波合法 materialize，实测 numeric_step 违例 "
                f"{len(inp.numeric_step_violations)} 处 / workflow g12 token {len(inp.workflow_g12_hits)} 处 / "
                f"ci/g12_*_smoke.py {len(inp.g12_smoke_scripts)} 件均为解锁后合法实现面，非预占；"
                "blocked 态恢复原机核，判据语义 0-byte）",
            )
        )
        consistency.append(
            (
                True,
                f"C4 src/spec/conformance 治理期 0-byte：not_applicable（implementation_status={impl_status!r} 已解锁，"
                f"治理期口径不适用；skipped_reason=实现波合法改动三面，实测 g12 实现面 token/命名命中 "
                f"{len(inp.impl_surface_hits)} 处均为解锁后合法实现面，非治理期预放；"
                "blocked 态恢复原机核，判据语义 0-byte）",
            )
        )
    return consistency


def run(inp: TreeInputs, require_ready: bool = False, printer=print) -> tuple[int, str]:
    """执行两类断言并输出；返回 (退出码, VERDICT)。"""
    # closed 三态口径（沿 G10.8b/G11 先例，判据语义 0-byte）：
    # status==closed = 收口终态，互锁使命完结，事实门/一致性门整体不适用。
    contract_status = (
        front_matter_field(inp.g12_contract_text, "status") if inp.g12_contract_text else None
    )
    if contract_status == "closed":
        printer(
            "[check_g12_implementation_interlock] 事实门/一致性门：not_applicable"
            "（status='closed' 收口终态，互锁使命完结；skipped_reason=G12.2+ 开工门问题"
            "不再适用——G12.7b close-out READY 后 status flip；active/blocked 态恢复原机核，"
            "判据语义 0-byte）"
        )
        printer("[check_g12_implementation_interlock] VERDICT = CLOSED")
        printer(
            "  CLOSED 是收口终态正确结论：本守卫回答「G12.2 可否开工」，契约 closed 后该问题"
            "不再适用；不得被当作 G-G12-3 重新开放凭据（契约 reopen 须新立项治理程序）。"
        )
        return 0, "CLOSED"
    facts = evaluate_fact_gates(inp)
    facts_all_green = all(ok for ok, _ in facts)
    consistency = evaluate_consistency_gates(inp, facts_all_green)

    printer("[check_g12_implementation_interlock] 事实门（当前可为红）：")
    for ok, msg in facts:
        printer(f"  {'PASS' if ok else 'RED '} {msg}")
    printer("[check_g12_implementation_interlock] 一致性门（红即脚本失败）：")
    for ok, msg in consistency:
        printer(f"  {'PASS' if ok else 'FAIL'} {msg}")

    verdict = "READY" if facts_all_green else "BLOCKED"
    printer(f"[check_g12_implementation_interlock] VERDICT = {verdict}")
    if verdict == "BLOCKED":
        missing = [msg.split("：", 1)[0] for ok, msg in facts if not ok]
        printer(f"  缺项清单：{missing}")
        printer(
            "  BLOCKED 是当前正确结论：G12.2+ 的 src/、spec/、conformance/ 与数字 workflow 步骤保持 0-byte；"
            "本输出不得被当作 G-G12-3 PASS。"
        )

    consistency_failed = [msg for ok, msg in consistency if not ok]
    if consistency_failed:
        printer(f"[check_g12_implementation_interlock] FAIL — {len(consistency_failed)} 项一致性门为红")
        return 1, verdict
    if require_ready and verdict != "READY":
        printer("[check_g12_implementation_interlock] FAIL — --require-ready 模式下互锁未 READY")
        return 1, verdict
    return 0, verdict


# ---------------------------------------------------------------------------
# selftest：可注入输入的受控负样本。
# ---------------------------------------------------------------------------

_RFC_APPROVED = """# fixture RFC

| 状态 | Agent Approved |
| Provenance | 起草 `Assisted-by: Kimi-K3（G12.1 治理波 RFC 起草）` |

## §9.1 对抗性评审记录

评审记录 `Assisted-by: Kimi-K3（D-409 独立评审轮次，与起草轮次隔离）`
"""

_GOOD_G11_CONTRACT = """---
contract: G11
status: closed
implementation_status: unblocked
---
# G11 CONTRACT

### §8.8 Close-out 终审签署块（2026-08-17）
"""

_GOOD_G12_CONTRACT = f"""---
contract: G12
status: active
implementation_status: blocked
---
# G12 CONTRACT

## §7 修订记录与开工裁决

- **用户立项指令**：2026-08-15 主会话下达「/goal ……并{G12_USER_INSTRUCTION_LITERAL}……」。
- **不可变基线**：G12.0 文档集不可变 ref = `{G12_0_IMMUTABLE_REF}`。

## §8 Implementation activation / Close-out（只追加区）

<!-- 当前不得写 PASS。 -->
"""


def _fixture_candidates_text() -> str:
    lines = ["# fixture candidates", "", "## 1. §1 表", "", "| ID | 裁决 | 承接锚 |", "|---|---|---|"]
    for rid in CANDIDATE_ROW_IDS:
        lines.append(f"| {rid} | defer-to-G13+ | 重判条件 = x；兜底 = y |")
    return "\n".join(lines) + "\n"


def _fixture_map_text() -> str:
    lines = [
        "# fixture map",
        "",
        "## 1. P0 硬门（精确 8 行）",
        "",
        "| M 行 | key/script | schema | 判据 | 波次 | numeric_step |",
        "|---|---|---|---|---|---|",
    ]
    for n in range(158, 166):
        slug = f"slug_m{n}"
        lines.append(
            f"| **M{n}** | `g12.p0.m{n}.{slug}` `ci/g12_{slug}_smoke.py` | "
            f"`milestones/g12/g12_m{n}_{slug}_evidence_schema.json` | 判据 | G12.2 | "
            "post-interlock actual-next-free allocation |"
        )
    lines += [
        "",
        "## 2. 已 go P1 硬门（一行：M166）",
        "",
        "| M 行 | key/script | schema | 判据 | 波次 | numeric_step |",
        "|---|---|---|---|---|---|",
        "| **M166** | `g12.p1.m166.pt_production_calibration` `ci/g12_pt_production_calibration_smoke.py` | "
        "`milestones/g12/g12_m166_pt_production_calibration_evidence_schema.json` | 判据 | G12.2 | "
        "post-interlock actual-next-free allocation |",
        "",
        "## 3. 条件型登记面",
    ]
    return "\n".join(lines) + "\n"


def _fixture_deferred_doc() -> dict:
    return {
        "entries": [
            {
                "id": rid,
                "title": f"{rid} title",
                "reason": "r",
                "backfill_condition": "b",
                "owner_milestone": "G9",
                "status": "open",
                "history": [{"event": "e1"}],
            }
            for rid in ("RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044")
        ]
    }


def _good_inputs() -> TreeInputs:
    deferred = _fixture_deferred_doc()
    return TreeInputs(
        g12_contract_text=_GOOD_G12_CONTRACT,
        g11_contract_text=_GOOD_G11_CONTRACT,
        rfc_texts={rfc: _RFC_APPROVED for rfc in G12_RFCS},
        candidate_findings=check_candidate_decisions(_fixture_candidates_text()),
        deferred_findings=check_deferred_append_only(deferred, deferred),
        map_findings=check_acceptance_map(_fixture_map_text()),
        ledger={
            "namespaces": {"CI_step": {"on_tree_max": 216, "next_free": 217}},
            "reserved_in_flight": [{"owner": "G12(路径追踪生产化期)"}],
        },
        workflow_max_step=216,
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
    inp.g11_contract_text = inp.g11_contract_text.replace("status: closed", "status: active")
    case("G11 status 改 active → ① 红", inp, "① G11_CONTRACT status = 'active'", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.g12_contract_text = inp.g12_contract_text.replace(G12_0_IMMUTABLE_REF, "deadbeef")
    case("G12.0 不可变 ref 未登记 → ① 红", inp, "G12.0 不可变 ref", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.rfc_texts[G12_RFCS[0]] = _RFC_APPROVED.replace(
        "| 状态 | Agent Approved |", "| 状态 | **Draft** 待评审 |"
    )
    case("RFC-0029 改 Draft → ② 红", inp, "状态未达 Agent Approved", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.candidate_findings = ["候选决策表缺行 M52"]
    case("候选决策表缺行注入 → ③ 红", inp, "③ 决策表/ deferred/ 验收映射三面", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.deferred_findings = ["RD-040 history 非只追加（base 5 条 vs current 4 条前缀不等）"]
    case("deferred history 非只追加注入 → ③ 红", inp, "deferred history 1 项违例", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.deferred_findings = None
    case("deferred base 不可取得 → ③ 红（无法判定不充绿）", inp, "只追加无法判定", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.map_findings = ["MAP §1 P0 集合 ['M158'] ≠ 闭集"]
    case("验收映射缺行注入 → ③ 红", inp, "验收映射 1 项缺行", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.g12_contract_text = inp.g12_contract_text.replace(G12_USER_INSTRUCTION_LITERAL, "支持路径追踪")
    case("用户开工指令字面缺失 → ④ 红", inp, "④ 用户 G12.2 开工指令留痕", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.workflow_max_step = 217
    case("workflow 实测末号 217 ≠ ledger on_tree_max 216 → ④ 红", inp, "workflow 实测末号 = 217", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.ledger["namespaces"]["CI_step"]["next_free"] = 219
    case("ledger next_free=219 ≠ on_tree_max+1 → ④ 红", inp, "next_free = 219", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.numeric_step_violations = ["milestones/g12/CI_GATES.md:70 numeric_step 数字赋值：numeric_step: 217"]
    case("数字步骤预占注入 → C3 FAIL 退 1（VERDICT 不受一致性门影响）", inp, "C3 数字步骤零预占", "READY", 1)

    inp = copy.deepcopy(_good_inputs())
    inp.impl_surface_hits = ["spec/g12_pt.md（文件名含 g12）"]
    case("spec 面 g12 命中注入 → C4 FAIL 退 1", inp, "C4 src/spec/conformance 治理期 0-byte", "READY", 1)

    inp = copy.deepcopy(_good_inputs())
    inp.g12_contract_text = inp.g12_contract_text.replace(
        "## §8 Implementation activation / Close-out（只追加区）",
        "### §8.1 G-G12-3 implementation_status 解锁记录",
    )
    case("落 §8 解锁记录但 front matter 未翻 → C2 FAIL 退 1", inp, "C2 §8 G-G12-3 解锁记录存在 = True", "READY", 1)

    # 两态口径（沿 G10.4b/G11 先例）：unblocked 态 C3/C4 自动不适用（skipped_reason 登记），
    # 预占/三面命中注入不再构成 FAIL；blocked 态上述两臂已实证原机核维持。
    unblocked_text = _GOOD_G12_CONTRACT.replace(
        "implementation_status: blocked", "implementation_status: unblocked"
    ).replace(
        "## §8 Implementation activation / Close-out（只追加区）",
        "### §8.1 G-G12-3 implementation_status 解锁记录",
    )
    inp = copy.deepcopy(_good_inputs())
    inp.g12_contract_text = unblocked_text
    inp.numeric_step_violations = ["milestones/g12/CI_GATES.md:70 numeric_step 数字赋值：numeric_step: 217"]
    inp.workflow_g12_hits = [".github/workflows/pr-smoke.yml:2000 g12.p0.m158.mis_full_surface"]
    inp.g12_smoke_scripts = ["g12_mis_full_surface_smoke.py"]
    inp.impl_surface_hits = ["spec/global_illumination.md:1 G12.2 新条款"]
    case(
        "unblocked 态预占/三面命中注入 → C3/C4 not_applicable 退 0（skipped_reason 登记）",
        inp, "skipped_reason", "READY", 0,
    )

    inp = copy.deepcopy(_good_inputs())
    inp.g12_contract_text = unblocked_text
    inp.map_findings = ["MAP §1 P0 集合漂移"]
    case("unblocked 态事实门红 → C1 仍 FAIL 退 1（两态校准不遮蔽 C1/C2）", inp, "C1 G12_CONTRACT implementation_status = 'unblocked'", "BLOCKED", 1)

    # closed 三态口径（沿 G10.8b/G11 先例）：status==closed → VERDICT=CLOSED 退 0，
    # 全门 not_applicable + skipped_reason 登记；active/blocked 态原机核维持
    #（上述红绿臂已实证）。
    inp = copy.deepcopy(_good_inputs())
    inp.g12_contract_text = unblocked_text.replace("status: active", "status: closed")
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

    # 当前树实测：VERDICT 与事实一致（互锁条件未齐=BLOCKED／全绿=READY／
    # 收口终态=CLOSED，三态均为正确结论）；一致性全绿时脚本退 0，有 FAIL 时退 1。
    tree_lines: list[str] = []
    tree_code, tree_verdict = run(load_inputs(ROOT), printer=tree_lines.append)
    tree_consistency_green = "FAIL — " not in "\n".join(tree_lines)
    expected_tree_exit = 0 if tree_consistency_green else 1
    if tree_verdict in ("BLOCKED", "READY", "CLOSED") and tree_code == expected_tree_exit:
        print(
            f"  TREE ok   — 当前树 VERDICT={tree_verdict}，exit={tree_code}"
            f"（一致性{'全绿' if tree_consistency_green else '有 FAIL'}；"
            f"{'互锁条件未齐期' if tree_verdict == 'BLOCKED' else ('收口终态' if tree_verdict == 'CLOSED' else 'G-G12-3 解锁条件已齐')}，符合当前事实预期）"
        )
    else:
        print(
            f"  TREE WRONG— 当前树 VERDICT/exit 与事实预期不符（期望 exit={expected_tree_exit}），"
            f"实测 VERDICT={tree_verdict} / exit={tree_code}"
        )
        failures += 1

    if failures:
        print(f"[check_g12_implementation_interlock] SELFTEST FAIL ({failures})")
        return 1
    print("[check_g12_implementation_interlock] SELFTEST PASS (16 RED + 1 GREEN + 1 TREE)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--require-ready",
        action="store_true",
        help="G12.2 实现 PR 前置模式：未 READY 即退出非零",
    )
    parser.add_argument("--selftest", action="store_true", help="用受控负样本证明断言能红/败")
    args = parser.parse_args()
    if args.selftest:
        return run_selftest()

    code, _ = run(load_inputs(ROOT), require_ready=args.require_ready)
    return code


if __name__ == "__main__":
    sys.exit(main())
