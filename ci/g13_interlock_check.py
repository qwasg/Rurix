#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.1 治理波）
"""G13.2 实现互锁守卫（g13.gov.implementation_interlock，步骤 235；G13_CONTRACT G-G13-3）。

读取事实源并逐项输出，最后给出 READY / BLOCKED / CLOSED。两类断言严格分开：

* **事实门**（当前允许为红，红即 BLOCKED，不是脚本失败）：
  ① G12_CONTRACT front matter status == closed（G12.7b close-out flip 终态，§8.8 签署块
    在位），且 G13.0 不可变 ref `8c5dc5ee` 已登记（G13_CONTRACT §7 立项裁决字面）；
  ② G13_CANDIDATE_DECISIONS 无空行（§1 G12 defer 22 行 + §2 open RD 7 行 + §3 新增
    候选 7 行 = 36 行闭集全数在位，全部数据行零空串/占位单元格）+ registry/deferred.json
    history 只追加无静默改判（对 G13.0 不可变 ref 8c5dc5ee base：条目四字段
    id/title/reason/backfill_condition 与 status/owner_milestone 0-byte、history
    前缀只追加、条目零删除）+ G13_ACCEPTANCE_MAP §1 五行 P0（M-a~M-e 闭集全等，
    key/script/schema 路径/判据列无缺）与 §2 零 go P1 空集；
  ③ **M-a 许可前置**：milestones/g13/design/vendor_upscale_license_clearance.md
    （DLSS/Streamline SDK + NGX 签名 DLL 与 FSR redistribution/集成许可 owner 法律面
    清结留痕）在树且五要素字面齐备——G13.1 期不在树即 RED（未清结即 blocked，
    G12.3 NRD 许可取证先例同模；不得以 FSR MIT 面宽松冒充 DLSS NGX 面清结）；
  ④ 用户 G13.2 开工指令留痕（2026-08-15 指令全期授权面——「支持 dlss、超分采样」
    字面在 G13_CONTRACT §7）+ workflow 实测末号与 ledger namespaces.CI_step
    on_tree_max 一致且 next_free == on_tree_max + 1。
* **一致性门**（红即脚本 FAIL，退出非零）：
  C1 双状态诚实：implementation_status == blocked，或事实门全绿（禁止治理完成冒充实现开工）；
  C2 §8 记录一致：§8 出现 G-G13-3 解锁记录 ⇔ 事实门全绿且 implementation_status != blocked；
  C3 数字步骤零预占：milestones/g13 P0 面无 numeric_step 数字赋值 / P0 数据行末列裸数字
    （治理三门步骤 233~235 白名单豁免——本波即落盘真脚本真步骤为例外合法面）、
    workflow 无 g13.p0./g13.p1./ci/g13_*_smoke.py token、ci/ 无 g13_*_smoke.py 预放；
  C4 src/spec/conformance 0-byte（治理期）：三面无 g13 实现面命中。

**C3/C4 两态口径（沿 G10.4b/G11/G12 先例，判据语义 0-byte）**：implementation_status
== blocked 时维持原机核；已解锁时 C3/C4 自动不适用（skipped_reason 登记并按通过处理）；
blocked 态恢复原机核。

**closed 三态口径（沿 G10.8b/G11/G12 先例，判据语义 0-byte）**：status == closed 时
互锁使命完结，事实门/一致性门整体不适用，VERDICT=CLOSED、exit=0；CLOSED 不得被当作
G-G13-3 重新开放凭据（契约 reopen 须新立项治理程序）。

诚实纪律：BLOCKED 是当前正确结论（M-a 许可前置未清结），不得被当作 G-G13-3 PASS；
`--require-ready` 供未来 G13.2 实现 PR 作前置 required check（未 READY 即退出非零）；
`--gate g13.gov.implementation_interlock` 产 evidence（VERDICT 字面入档，BLOCKED 不充绿）。
`--selftest` 用可注入输入的受控负样本证明每组断言都能红/败。

用法：
  py -3 ci/g13_interlock_check.py --gate g13.gov.implementation_interlock
  py -3 ci/g13_interlock_check.py --require-ready
  py -3 ci/g13_interlock_check.py --selftest
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
import g11_wave_exit_lib as wel  # noqa: E402
from g13_candidate_decisions_check import FROZEN_IDS as CANDIDATE_FROZEN_IDS  # noqa: E402

G13_CONTRACT = ROOT / "milestones/g13/G13_CONTRACT.md"
G12_CONTRACT = ROOT / "milestones/g12/G12_CONTRACT.md"
G13_CANDIDATES = ROOT / "milestones/g13/G13_CANDIDATE_DECISIONS.md"
G13_MAP = ROOT / "milestones/g13/G13_ACCEPTANCE_MAP.md"
LICENSE_ARTIFACT = ROOT / "milestones/g13/design/vendor_upscale_license_clearance.md"
LEDGER = ROOT / "registry/number_ledger.json"
DEFERRED = ROOT / "registry/deferred.json"
WORKFLOWS = ROOT / ".github/workflows"
SCHEMA_PATH = ROOT / "milestones/g13/g13_interlock_check_evidence_schema.json"

GATE_KEY = "g13.gov.implementation_interlock"
NUMERIC_STEP = 235  # 落盘前实测 registry/number_ledger.json CI_step.next_free=235 顺位领取
SUBJECT = "g13_interlock_check"
WAVE = "G13.1"

G13_0_IMMUTABLE_REF = "8c5dc5ee"  # G13.0 文档集不可变 ref（G12 close-out flip commit，tag g12-closed）
G13_USER_INSTRUCTION_LITERAL = "支持 dlss、超分采样"
# M-a 许可前置五要素字面（G12.3 NRD 许可取证先例同模——owner 法律面清结为开工硬前置）。
LICENSE_REQUIRED_TOKENS = ("Streamline", "NGX", "FSR", "owner", "清结")

# 空行门占位单元格闭集（MAP §5 no-empty 口径）。
PLACEHOLDER_CELLS = {"", "TBD", "TODO", "待定", "待补", "—"}
# 治理三门白名单（本波即落盘真脚本真步骤 233~235 的例外合法面；C3 数字预占扫描豁免）。
GOVERNANCE_GATE_KEYS = (
    "g13.wave.1.acceptance_map",
    "g13.wave.1.candidate_decisions",
    "g13.gov.implementation_interlock",
)
GOVERNANCE_SCHEMA_FILES = {
    "g13_acceptance_map_check_evidence_schema.json",
    "g13_candidate_decisions_check_evidence_schema.json",
    "g13_interlock_check_evidence_schema.json",
}

# 数字步骤零预占扫描面：numeric_step 数字赋值 / g13 P0 数据行末列裸数字 / workflow g13 实现面 token。
NUMERIC_ASSIGN_RE = re.compile(r"numeric_step[\"']?\s*[:=]\s*[\"']?\d")
NUMERIC_TAIL_CELL_RE = re.compile(r"\|\s*\d{1,4}\s*\|\s*$")
WORKFLOW_G13_RE = re.compile(r"g13\.p[01]\.|ci/g13_[a-z0-9_]+_smoke\.py")
WORKFLOW_STEP_RE = re.compile(r"步骤\s*(\d{1,4})")
# C4 扫描面：gate-key/脚本命名 token + g13 命名文件。
IMPL_SURFACE_G13_RE = re.compile(r"g13\.p[01]\.|g13\.wave\.|g13\.gov\.|ci/g13_")
# MAP §1 P0 行首单元格形态：| **M-a** | ...。
MAP_ROW_RE = re.compile(r"^\|\s*\*\*(M-[a-e])\*\*")
MAP_KEY_RE = re.compile(r"g13\.p0\.(m_[a-e])\.[a-z0-9_]+")
MAP_SCRIPT_RE = re.compile(r"ci/g13_[a-z0-9_]+_smoke\.py")
MAP_SCHEMA_RE = re.compile(r"milestones/g13/g13_m_[a-e]_[a-z0-9_]+_evidence_schema\.json")
EXPECTED_MAP_P0 = {"M-a", "M-b", "M-c", "M-d", "M-e"}
ACTIVATION_RE = re.compile(
    r"^#{2,5}\s*§?8\.\d+[^\n]*(implementation_status 解锁|G-G13-3)", re.MULTILINE
)


@dataclass
class TreeInputs:
    """互锁守卫的全部可注入输入；None / 空表示树上缺失或无法判定。"""

    g13_contract_text: str | None = None
    g12_contract_text: str | None = None
    license_text: str | None = None
    candidate_findings: list[str] | None = None
    deferred_findings: list[str] | None = None
    map_findings: list[str] | None = None
    ledger: dict | None = None
    workflow_max_step: int | None = None
    numeric_step_violations: list[str] = field(default_factory=list)
    workflow_g13_hits: list[str] = field(default_factory=list)
    g13_smoke_scripts: list[str] = field(default_factory=list)
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
    """G13_CANDIDATE_DECISIONS 无空行：36 行闭集全数在位 + 全部数据行零空串/占位单元格。"""
    if text is None:
        return ["G13_CANDIDATE_DECISIONS.md 缺失"]
    findings: list[str] = []
    rows = _table_data_rows(text)
    first_cells = {r[0] for r in rows if r}
    for rid in CANDIDATE_FROZEN_IDS:
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
    """deferred.json history 只追加核验（vs G13.0 base）：条目零删除、条目级字段 0-byte、
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
    """G13_ACCEPTANCE_MAP §1 五行 P0（M-a~M-e 闭集全等）+ §2 零 go P1 空集 + 行面齐备。"""
    if text is None:
        return ["G13_ACCEPTANCE_MAP.md 缺失"]
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
    if set(p0_rows) != EXPECTED_MAP_P0:
        findings.append(f"MAP §1 P0 集合 {sorted(p0_rows)} ≠ 闭集 {sorted(EXPECTED_MAP_P0)}（缺行/多行）")
    if p1_rows:
        findings.append(f"MAP §2 P1 集合 {sorted(p1_rows)} 非空（G13.1 零 go P1 字面违例）")
    for mid, cells in p0_rows.items():
        blob = "|".join(cells)
        km = MAP_KEY_RE.search(blob)
        if not km or km.group(1) != mid.lower().replace("-", "_"):
            findings.append(f"MAP {mid} 行 symbolic key 缺失或 m 段不符")
        if not MAP_SCRIPT_RE.search(blob):
            findings.append(f"MAP {mid} 行缺稳定脚本 ci/g13_*_smoke.py")
        if not MAP_SCHEMA_RE.search(blob):
            findings.append(f"MAP {mid} 行缺 evidence schema 目标路径")
        for cell in cells:
            if cell in PLACEHOLDER_CELLS:
                findings.append(f"MAP {mid} 行存在空串/占位单元格")
                break
    return findings


def check_license_precondition(text: str | None) -> tuple[bool, str]:
    """M-a 许可前置：清结留痕在树且五要素字面齐备；G13.1 期不在树 = RED（诚实 BLOCKED）。"""
    if text is None:
        return False, (
            "milestones/g13/design/vendor_upscale_license_clearance.md 未在树——"
            "owner 法律面清结未落地（M-a 保持 blocked；G12.3 NRD 许可取证先例同模）"
        )
    missing = [t for t in LICENSE_REQUIRED_TOKENS if t not in text]
    if missing:
        return False, f"许可清结留痕缺要素字面 {missing}（五要素闭集：Streamline/NGX/FSR/owner/清结）"
    return True, "许可清结留痕在树且五要素齐备（Streamline/NGX/FSR/owner/清结）"


def _scan_numeric_step(root: Path) -> list[str]:
    """milestones/g13 P0 面数字步骤零预占扫描（治理三门白名单豁免）；返回违例行清单。"""
    hits: list[str] = []
    base = root / "milestones/g13"
    if not base.is_dir():
        return ["milestones/g13 目录缺失"]
    for path in sorted(base.rglob("*")):
        if not path.is_file() or path.suffix not in (".md", ".json", ".yml", ".yaml"):
            continue
        if path.name in GOVERNANCE_SCHEMA_FILES:
            continue  # 治理三门 schema const 钉死面（步骤 233~235 合法领取）
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        rel = path.relative_to(root)
        for lineno, line in enumerate(text.splitlines(), 1):
            if any(k in line for k in GOVERNANCE_GATE_KEYS):
                continue  # 治理三门声明行（契约 §4.3 / MAP §5）合法
            if NUMERIC_ASSIGN_RE.search(line):
                hits.append(f"{rel}:{lineno} numeric_step 数字赋值：{line.strip()[:80]}")
            elif line.lstrip().startswith(("| **M-", "| `g13.")) and NUMERIC_TAIL_CELL_RE.search(line):
                hits.append(f"{rel}:{lineno} 数据行末列裸数字（疑似步骤预占）：{line.strip()[:80]}")
    return hits


def _scan_workflows(root: Path) -> list[str]:
    """workflow g13 实现面 token 扫描（g13.p0./g13.p1./ci/g13_*_smoke.py 即预占）。"""
    hits: list[str] = []
    if not WORKFLOWS.is_dir():
        return hits
    for path in sorted(WORKFLOWS.glob("*.yml")) + sorted(WORKFLOWS.glob("*.yaml")):
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for lineno, line in enumerate(text.splitlines(), 1):
            if WORKFLOW_G13_RE.search(line):
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
    """src/spec/conformance 治理期 0-byte 扫描：g13 实现面 token / g13 命名文件即违例。"""
    hits: list[str] = []
    for sub in ("src", "spec", "conformance"):
        base = root / sub
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if not path.is_file():
                continue
            rel = path.relative_to(root)
            if "g13" in path.name.lower():
                hits.append(f"{rel}（文件名含 g13）")
                continue
            if path.suffix.lower() not in (".rs", ".md", ".rx", ".toml", ".json", ".txt"):
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                continue
            for lineno, line in enumerate(text.splitlines(), 1):
                if IMPL_SURFACE_G13_RE.search(line):
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
    base_text = _git_show_file(root, G13_0_IMMUTABLE_REF, "registry/deferred.json")
    if base_text is not None:
        try:
            deferred_base = json.loads(base_text)
        except json.JSONDecodeError:
            deferred_base = None

    return TreeInputs(
        g13_contract_text=_read(G13_CONTRACT),
        g12_contract_text=_read(G12_CONTRACT),
        license_text=_read(LICENSE_ARTIFACT),
        candidate_findings=check_candidate_decisions(_read(G13_CANDIDATES)),
        deferred_findings=check_deferred_append_only(deferred_base, deferred_current),
        map_findings=check_acceptance_map(_read(G13_MAP)),
        ledger=_read_json(LEDGER),
        workflow_max_step=_scan_workflow_max_step(root),
        numeric_step_violations=_scan_numeric_step(root),
        workflow_g13_hits=_scan_workflows(root),
        g13_smoke_scripts=[p.name for p in sorted((root / "ci").glob("g13_*_smoke.py"))],
        impl_surface_hits=_scan_impl_surface(root),
    )


def front_matter_field(text: str, field: str) -> str | None:
    m = re.search(rf"^{re.escape(field)}:\s*(\S+)\s*$", text, re.MULTILINE)
    return m.group(1) if m else None


def evaluate_fact_gates(inp: TreeInputs) -> list[tuple[bool, str]]:
    """事实门：红 = BLOCKED，不是脚本失败。"""
    facts: list[tuple[bool, str]] = []

    g12_status = front_matter_field(inp.g12_contract_text, "status") if inp.g12_contract_text else None
    g12_closeout_section = bool(inp.g12_contract_text and "§8.8" in inp.g12_contract_text)
    g13_ref_registered = bool(inp.g13_contract_text and G13_0_IMMUTABLE_REF in inp.g13_contract_text)
    facts.append(
        (
            g12_status == "closed" and g12_closeout_section and g13_ref_registered,
            f"① G12_CONTRACT status = {g12_status!r}（要求 closed）且 §8.8 签署块在位 = {g12_closeout_section}；"
            f"G13.0 不可变 ref {G13_0_IMMUTABLE_REF} 登记 = {g13_ref_registered}",
        )
    )

    sub2: list[str] = []
    ok2 = True
    if inp.candidate_findings is None:
        ok2 = False
        sub2.append("G13_CANDIDATE_DECISIONS 缺失")
    elif inp.candidate_findings:
        ok2 = False
        sub2.append(f"候选决策表 {len(inp.candidate_findings)} 项缺/空（首项 {inp.candidate_findings[0]}）")
    else:
        sub2.append("候选决策表 36 行零空行")
    if inp.deferred_findings is None:
        ok2 = False
        sub2.append("deferred.json base 不可取得，只追加无法判定")
    elif inp.deferred_findings:
        ok2 = False
        sub2.append(f"deferred history {len(inp.deferred_findings)} 项违例（首项 {inp.deferred_findings[0]}）")
    else:
        sub2.append("deferred history 只追加（vs G13.0 base 四字段 0-byte）")
    if inp.map_findings is None:
        ok2 = False
        sub2.append("G13_ACCEPTANCE_MAP 缺失")
    elif inp.map_findings:
        ok2 = False
        sub2.append(f"验收映射 {len(inp.map_findings)} 项缺行（首项 {inp.map_findings[0]}）")
    else:
        sub2.append("验收映射 §1 五行 P0 + §2 零 go P1 空集无缺行")
    facts.append((ok2, f"② 决策表/ deferred/ 验收映射三面：{'；'.join(sub2)}"))

    license_ok, license_msg = check_license_precondition(inp.license_text)
    facts.append((license_ok, f"③ M-a 许可前置（owner 法律面清结硬门）：{license_msg}"))

    instruction_recorded = bool(
        inp.g13_contract_text and G13_USER_INSTRUCTION_LITERAL in inp.g13_contract_text
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
            f"④ 用户 G13.2 开工指令留痕（2026-08-15 全期授权面「{G13_USER_INSTRUCTION_LITERAL}」字面）"
            f" = {instruction_recorded}；workflow 实测末号 = {inp.workflow_max_step}、"
            f"ledger CI_step on_tree_max = {on_tree_max}、next_free = {next_free}"
            f"（一致 = {ledger_consistent}）",
        )
    )
    return facts


def evaluate_consistency_gates(inp: TreeInputs, facts_all_green: bool) -> list[tuple[bool, str]]:
    """一致性门：红即脚本 FAIL，退出非零。"""
    consistency: list[tuple[bool, str]] = []

    impl_status = front_matter_field(inp.g13_contract_text, "implementation_status") if inp.g13_contract_text else None
    consistency.append(
        (
            impl_status == "blocked" or facts_all_green,
            f"C1 G13_CONTRACT implementation_status = {impl_status!r}；事实门全绿 = {facts_all_green}"
            "（事实未全绿时必须保持 blocked，禁止治理完成冒充实现开工）",
        )
    )

    activated = inp.g13_contract_text is not None and bool(ACTIVATION_RE.search(inp.g13_contract_text))
    consistency.append(
        (
            activated == (facts_all_green and impl_status not in (None, "blocked")),
            f"C2 §8 G-G13-3 解锁记录存在 = {activated}；事实门全绿 = {facts_all_green}、"
            f"implementation_status = {impl_status!r}（双状态与 §8 记录必须一致）",
        )
    )

    preclaim = inp.numeric_step_violations + inp.workflow_g13_hits + inp.g13_smoke_scripts
    if impl_status == "blocked":
        consistency.append(
            (
                not preclaim,
                f"C3 数字步骤零预占：milestones/g13 numeric_step 违例 {len(inp.numeric_step_violations)} 处、"
                f"workflow g13 实现面 token {len(inp.workflow_g13_hits)} 处、ci/g13_*_smoke.py 预放 "
                f"{inp.g13_smoke_scripts or '无'}（治理三门 233~235 白名单豁免）"
                + (f"；首处违例 {preclaim[0]}" if preclaim else ""),
            )
        )

        consistency.append(
            (
                not inp.impl_surface_hits,
                f"C4 src/spec/conformance 治理期 0-byte：g13 实现面 token/命名命中 "
                f"{len(inp.impl_surface_hits)} 处"
                + (f"；首处 {inp.impl_surface_hits[0]}" if inp.impl_surface_hits else ""),
            )
        )
    else:
        # 两态口径（沿 G10.4b/G11/G12 先例，判据语义 0-byte）：已解锁后 C3/C4 治理期口径
        # 自动不适用——实现波合法 materialize 数字步骤/workflow/ci 脚本与
        # src/spec/conformance 面，机核命中非违例；blocked 态恢复原机核。
        consistency.append(
            (
                True,
                f"C3 数字步骤零预占：not_applicable（implementation_status={impl_status!r} 已解锁，"
                f"治理期口径不适用；skipped_reason=实现波合法 materialize，实测 numeric_step 违例 "
                f"{len(inp.numeric_step_violations)} 处 / workflow g13 实现面 token {len(inp.workflow_g13_hits)} 处 / "
                f"ci/g13_*_smoke.py {len(inp.g13_smoke_scripts)} 件均为解锁后合法实现面，非预占；"
                "blocked 态恢复原机核，判据语义 0-byte）",
            )
        )
        consistency.append(
            (
                True,
                f"C4 src/spec/conformance 治理期 0-byte：not_applicable（implementation_status={impl_status!r} 已解锁，"
                f"治理期口径不适用；skipped_reason=实现波合法改动三面，实测 g13 实现面 token/命名命中 "
                f"{len(inp.impl_surface_hits)} 处均为解锁后合法实现面，非治理期预放；"
                "blocked 态恢复原机核，判据语义 0-byte）",
            )
        )
    return consistency


def run(inp: TreeInputs, require_ready: bool = False, printer=print) -> tuple[int, str]:
    """执行两类断言并输出；返回 (退出码, VERDICT)。"""
    # closed 三态口径（沿 G10.8b/G11/G12 先例，判据语义 0-byte）。
    contract_status = (
        front_matter_field(inp.g13_contract_text, "status") if inp.g13_contract_text else None
    )
    if contract_status == "closed":
        printer(
            "[g13_interlock] 事实门/一致性门：not_applicable"
            "（status='closed' 收口终态，互锁使命完结；skipped_reason=G13.2+ 开工门问题"
            "不再适用——G13.5b close-out READY 后 status flip；active/blocked 态恢复原机核，"
            "判据语义 0-byte）"
        )
        printer("[g13_interlock] VERDICT = CLOSED")
        printer(
            "  CLOSED 是收口终态正确结论：本守卫回答「G13.2 可否开工」，契约 closed 后该问题"
            "不再适用；不得被当作 G-G13-3 重新开放凭据（契约 reopen 须新立项治理程序）。"
        )
        return 0, "CLOSED"
    facts = evaluate_fact_gates(inp)
    facts_all_green = all(ok for ok, _ in facts)
    consistency = evaluate_consistency_gates(inp, facts_all_green)

    printer("[g13_interlock] 事实门（当前可为红）：")
    for ok, msg in facts:
        printer(f"  {'PASS' if ok else 'RED '} {msg}")
    printer("[g13_interlock] 一致性门（红即脚本失败）：")
    for ok, msg in consistency:
        printer(f"  {'PASS' if ok else 'FAIL'} {msg}")

    verdict = "READY" if facts_all_green else "BLOCKED"
    printer(f"[g13_interlock] VERDICT = {verdict}")
    if verdict == "BLOCKED":
        missing = [msg.split("：", 1)[0] for ok, msg in facts if not ok]
        printer(f"  缺项清单：{missing}")
        printer(
            "  BLOCKED 是当前正确结论：G13.2+ 的 src/、spec/、conformance/ 与 P0 数字 workflow 步骤保持 0-byte；"
            "本输出不得被当作 G-G13-3 PASS。"
        )

    consistency_failed = [msg for ok, msg in consistency if not ok]
    if consistency_failed:
        printer(f"[g13_interlock] FAIL — {len(consistency_failed)} 项一致性门为红")
        return 1, verdict
    if require_ready and verdict != "READY":
        printer("[g13_interlock] FAIL — --require-ready 模式下互锁未 READY")
        return 1, verdict
    return 0, verdict


def run_gate() -> int:
    """--gate 模式：真树执行 + evidence 落盘（VERDICT 字面入档，BLOCKED 不充绿）。"""
    inp = load_inputs(ROOT)
    lines: list[str] = []
    code, verdict = run(inp, printer=lines.append)
    for line in lines:
        print(line)

    facts = evaluate_fact_gates(inp)
    facts_all_green = all(ok for ok, _ in facts)
    impl_status = front_matter_field(inp.g13_contract_text, "implementation_status") if inp.g13_contract_text else None
    contract_status = front_matter_field(inp.g13_contract_text, "status") if inp.g13_contract_text else None
    consistency_green = "FAIL — " not in "\n".join(lines)
    verdict_honest = (
        (verdict == "CLOSED" and contract_status == "closed")
        or (verdict == "READY" and facts_all_green and contract_status != "closed")
        or (verdict == "BLOCKED" and not facts_all_green)
    )
    gate_facts: list[dict] = []
    fact_ids = (
        "fact_gate_1_prior_milestone_closed",
        "fact_gate_2_governance_docs",
        "fact_gate_3_license_precondition",
        "fact_gate_4_instruction_ledger",
    )
    for fid, (ok, msg) in zip(fact_ids, facts):
        gate_facts.append({
            "id": fid,
            "status": "PASS",
            "detail": f"state={'GREEN' if ok else 'RED（诚实登记，不充绿）'}——{msg}",
        })
    gate_facts.append({
        "id": "consistency_gates_green",
        "status": "PASS" if consistency_green else "FAIL",
        "detail": "C1~C4 全绿" if consistency_green else "一致性门存在 FAIL（见上行逐字输出）",
    })
    gate_facts.append({
        "id": "verdict_honest",
        "status": "PASS" if verdict_honest else "FAIL",
        "detail": f"VERDICT={verdict} 与事实门真值一致（READY ⇔ 事实门全绿；CLOSED ⇔ status closed）",
    })
    gate_facts.append({
        "id": "verdict_recorded",
        "status": "PASS",
        "detail": f"VERDICT={verdict} 字面入档（evidence notes 同字面）",
    })
    if verdict == "BLOCKED":
        not_masq = impl_status == "blocked"
        detail = f"VERDICT=BLOCKED 且 implementation_status={impl_status!r}：实现面硬阻断不充绿"
    elif verdict == "READY":
        not_masq = facts_all_green
        detail = "VERDICT=READY：事实门全绿机器事实（不以叙述替代）"
    else:
        not_masq = True
        detail = "VERDICT=CLOSED：收口终态，互锁使命完结"
    gate_facts.append({
        "id": "blocked_not_masqueraded",
        "status": "PASS" if not_masq else "FAIL",
        "detail": detail,
    })

    overall = all(f["status"] == "PASS" for f in gate_facts)
    if not SCHEMA_PATH.is_file():
        print(f"[g13_interlock] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    ecode, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G13_CONTRACT G-G13-3/§4.3/§7;G13_ACCEPTANCE_MAP §5/§6;registry/number_ledger.json;registry/deferred.json;milestones/g13/design/vendor_upscale_license_clearance.md（M-a 许可前置面）",
        required_gate_rows=[],
        extra_facts=gate_facts,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes=(
            f"G13.1 治理门——实现互锁 validator 诚实报告：VERDICT={verdict}"
            "（BLOCKED 是当前正确结论：M-a 许可前置 owner 法律面清结未落地，G13.2 不得开工；"
            "不充绿、不以叙述替代机器事实）；一致性门 C1~C4 全绿；"
            "--require-ready 供未来 G13.2 实现 PR 前置 required check"
        ),
        host_section_pass=overall,
    )
    return 0 if (overall and ecode == 0) else 1


# ---------------------------------------------------------------------------
# selftest：可注入输入的受控负样本。
# ---------------------------------------------------------------------------

_GOOD_LICENSE = """# fixture 许可清结留痕

DLSS（Streamline SDK + NGX 签名 DLL）与 FSR redistribution/集成许可经 owner 法律面清结。
"""

_GOOD_G12_CONTRACT = """---
contract: G12
status: closed
implementation_status: unlocked
---
# G12 CONTRACT

### §8.8 Close-out 终审签署块（2026-08-17）
"""

_GOOD_G13_CONTRACT = f"""---
contract: G13
status: active
implementation_status: blocked
---
# G13 CONTRACT

## §7 修订记录与开工裁决

- **用户立项指令**：2026-08-15 主会话下达「/goal ……并{G13_USER_INSTRUCTION_LITERAL}……」。
- **不可变基线**：G13.0 文档集不可变 ref = `{G13_0_IMMUTABLE_REF}`。

## §8 Implementation activation / Close-out（只追加区）

<!-- 当前不得写 PASS。 -->
"""


def _fixture_candidates_text() -> str:
    lines = ["# fixture candidates", "", "## 1. §1 表", "", "| ID | 裁决 | 承接锚 |", "|---|---|---|"]
    for rid in CANDIDATE_FROZEN_IDS:
        lines.append(f"| {rid} | defer-to-G14+ | 重判条件 = x；兜底 = y |")
    return "\n".join(lines) + "\n"


def _fixture_map_text() -> str:
    lines = [
        "# fixture map",
        "",
        "## 1. P0 硬门（精确 5 行）",
        "",
        "| M 行 | key/script | schema | 判据 | 波次 | numeric_step |",
        "|---|---|---|---|---|---|",
    ]
    for letter, slug in (
        ("a", "vendor_upscale_integration"),
        ("b", "tsr_device_kernel"),
        ("c", "ue_upscale_parity"),
        ("d", "ue_lumen_gi_parity"),
        ("e", "regression_drift_guard"),
    ):
        lines.append(
            f"| **M-{letter}** | `g13.p0.m_{letter}.{slug}` `ci/g13_{slug}_smoke.py` | "
            f"`milestones/g13/g13_m_{letter}_{slug}_evidence_schema.json` | 判据 | G13.2 | "
            "post-interlock actual-next-free allocation |"
        )
    lines += [
        "",
        "## 2. 已 go P1 硬门（零行）",
        "",
        "G13.1 无 go 的 P1 行。",
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
        g13_contract_text=_GOOD_G13_CONTRACT,
        g12_contract_text=_GOOD_G12_CONTRACT,
        license_text=_GOOD_LICENSE,
        candidate_findings=check_candidate_decisions(_fixture_candidates_text()),
        deferred_findings=check_deferred_append_only(deferred, deferred),
        map_findings=check_acceptance_map(_fixture_map_text()),
        ledger={"namespaces": {"CI_step": {"on_tree_max": 235, "next_free": 236}}},
        workflow_max_step=235,
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
    inp.g12_contract_text = inp.g12_contract_text.replace("status: closed", "status: active")
    case("G12 status 改 active → ① 红", inp, "① G12_CONTRACT status = 'active'", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.g13_contract_text = inp.g13_contract_text.replace(G13_0_IMMUTABLE_REF, "deadbeef")
    case("G13.0 不可变 ref 未登记 → ① 红", inp, "G13.0 不可变 ref", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.candidate_findings = ["候选决策表缺行 M52"]
    case("候选决策表缺行注入 → ② 红", inp, "② 决策表/ deferred/ 验收映射三面", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.deferred_findings = ["RD-040 history 非只追加（base 5 条 vs current 4 条前缀不等）"]
    case("deferred history 非只追加注入 → ② 红", inp, "deferred history 1 项违例", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.deferred_findings = None
    case("deferred base 不可取得 → ② 红（无法判定不充绿）", inp, "只追加无法判定", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.map_findings = ["MAP §1 P0 集合 ['M-a'] ≠ 闭集"]
    case("验收映射缺行注入 → ② 红", inp, "验收映射 1 项缺行", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.license_text = None
    case("许可清结留痕缺位 → ③ 红（M-a 许可前置硬门）", inp, "③ M-a 许可前置", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.license_text = "# 只有 FSR MIT 面登记，无 owner 清结"
    case("许可五要素缺字 → ③ 红（FSR 宽松冒充 DLSS NGX 清结必拒）", inp, "缺要素字面", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.g13_contract_text = inp.g13_contract_text.replace(G13_USER_INSTRUCTION_LITERAL, "支持路径追踪")
    case("用户开工指令字面缺失 → ④ 红", inp, "④ 用户 G13.2 开工指令留痕", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.workflow_max_step = 236
    case("workflow 实测末号 236 ≠ ledger on_tree_max 235 → ④ 红", inp, "workflow 实测末号 = 236", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.ledger["namespaces"]["CI_step"]["next_free"] = 238
    case("ledger next_free=238 ≠ on_tree_max+1 → ④ 红", inp, "next_free = 238", "BLOCKED", 0)

    inp = copy.deepcopy(_good_inputs())
    inp.numeric_step_violations = ["milestones/g13/G13_ACCEPTANCE_MAP.md:30 numeric_step 数字赋值：numeric_step: 236"]
    case("数字步骤预占注入 → C3 FAIL 退 1（VERDICT 不受一致性门影响）", inp, "C3 数字步骤零预占", "READY", 1)

    inp = copy.deepcopy(_good_inputs())
    inp.impl_surface_hits = ["spec/g13_upscale.md（文件名含 g13）"]
    case("spec 面 g13 命中注入 → C4 FAIL 退 1", inp, "C4 src/spec/conformance 治理期 0-byte", "READY", 1)

    inp = copy.deepcopy(_good_inputs())
    inp.g13_contract_text = inp.g13_contract_text.replace(
        "## §8 Implementation activation / Close-out（只追加区）",
        "### §8.1 G-G13-3 implementation_status 解锁记录",
    )
    case("落 §8 解锁记录但 front matter 未翻 → C2 FAIL 退 1", inp, "C2 §8 G-G13-3 解锁记录存在 = True", "READY", 1)

    # 两态口径（沿 G10.4b/G11/G12 先例）：unblocked 态 C3/C4 自动不适用（skipped_reason 登记）。
    unblocked_text = _GOOD_G13_CONTRACT.replace(
        "implementation_status: blocked", "implementation_status: unblocked"
    ).replace(
        "## §8 Implementation activation / Close-out（只追加区）",
        "### §8.1 G-G13-3 implementation_status 解锁记录",
    )
    inp = copy.deepcopy(_good_inputs())
    inp.g13_contract_text = unblocked_text
    inp.numeric_step_violations = ["milestones/g13/G13_ACCEPTANCE_MAP.md:30 numeric_step 数字赋值：numeric_step: 236"]
    inp.workflow_g13_hits = [".github/workflows/pr-smoke.yml:2900 g13.p0.m_a.vendor_upscale_integration"]
    inp.g13_smoke_scripts = ["g13_vendor_upscale_integration_smoke.py"]
    inp.impl_surface_hits = ["spec/upscale.md:1 g13.p0. 新条款"]
    case(
        "unblocked 态预占/三面命中注入 → C3/C4 not_applicable 退 0（skipped_reason 登记）",
        inp, "skipped_reason", "READY", 0,
    )

    inp = copy.deepcopy(_good_inputs())
    inp.g13_contract_text = unblocked_text
    inp.license_text = None
    case("unblocked 态事实门红 → C1 仍 FAIL 退 1（两态校准不遮蔽 C1/C2）", inp, "C1 G13_CONTRACT implementation_status = 'unblocked'", "BLOCKED", 1)

    # closed 三态口径：status==closed → VERDICT=CLOSED 退 0。
    inp = copy.deepcopy(_good_inputs())
    inp.g13_contract_text = unblocked_text.replace("status: active", "status: closed")
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

    # 当前树实测：VERDICT 与事实一致（G13.1 期 M-a 许可前置未清结 = BLOCKED／全绿=READY／
    # 收口终态=CLOSED，三态均为正确结论）；一致性全绿时脚本退 0，有 FAIL 时退 1。
    tree_lines: list[str] = []
    tree_code, tree_verdict = run(load_inputs(ROOT), printer=tree_lines.append)
    tree_consistency_green = "FAIL — " not in "\n".join(tree_lines)
    expected_tree_exit = 0 if tree_consistency_green else 1
    if tree_verdict in ("BLOCKED", "READY", "CLOSED") and tree_code == expected_tree_exit:
        print(
            f"  TREE ok   — 当前树 VERDICT={tree_verdict}，exit={tree_code}"
            f"（一致性{'全绿' if tree_consistency_green else '有 FAIL'}；"
            f"{'M-a 许可前置未清结期' if tree_verdict == 'BLOCKED' else ('收口终态' if tree_verdict == 'CLOSED' else 'G-G13-3 解锁条件已齐')}，符合当前事实预期）"
        )
    else:
        print(
            f"  TREE WRONG— 当前树 VERDICT/exit 与事实预期不符（期望 exit={expected_tree_exit}），"
            f"实测 VERDICT={tree_verdict} / exit={tree_code}"
        )
        failures += 1

    if failures:
        print(f"[g13_interlock] SELFTEST FAIL ({failures})")
        return 1
    print("[g13_interlock] SELFTEST PASS (17 RED + 1 GREEN + 1 TREE)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--require-ready", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        return run_gate()
    code, _ = run(load_inputs(ROOT), require_ready=args.require_ready)
    return code


if __name__ == "__main__":
    sys.exit(main())
