#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.1 治理波 validator）
"""G11 P2 决策门骨架（milestones/g11/CI_GATES.md §5 `g11.wave.6.decisions` 骨架期）。

骨架期定位（G11.1 governance-only）：本脚本为 G11.6 P2 穷举决策门的**骨架**——
对 `milestones/g11/G11_CANDIDATE_DECISIONS.md`（法定输入 11 差距行 + G10 defer
18 行 + 新增候选 3 行）做行级机核：行集闭集对账、裁决枚举合法、零空行、defer
行承接锚「重判条件 + 兜底」字面、承接锚清单与 defer 行集对账。
G11.6 时按候选全集（G11.1 决策表校准后冻结 + G11.2~G11.5 期内新增分项）扩闭集
materialize 并领取数字步骤（同 G10 先例「骨架期行级机核 → materialize 期扩闭集」，
post-interlock actual-next-free allocation）。

只读文档，不代绿实现门；本骨架属 `check_*` 未编号守卫，不占 numeric CI step。

用法:
  py -3 ci/g11_p2_decisions_check.py --gate g11.wave.6.decisions   # 骨架期行级机核
  py -3 ci/g11_p2_decisions_check.py --selftest                    # 受控负样本红绿自检
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DECISIONS = ROOT / "milestones/g11/G11_CANDIDATE_DECISIONS.md"

GATE_KEY = "g11.wave.6.decisions"

# 骨架期行集闭集（G11.1 候选决策表三节）：
# §1 法定输入 11 差距行 / §2 G10 defer-to-G11+ 18 行 / §4 G11 新增候选 3 行。
EXPECTED_GAP_IDS = ["R1", "R2", "R3", "R4", "R5", "U1", "U2", "U3", "C1", "C2", "C3"]
EXPECTED_DEFER_IDS = [
    "M61", "M52", "M99-clipmap", "M100-high", "SAFE-GPU",
    "M127", "M98-l4", "M114-strand", "M118-hdr-cal", "M125-adopt3",
    "G10-N5", "G10-N6", "G10-N7", "G10-N8", "G10-N10",
    "G10-N11", "G10-N16", "G10-N17",
]
EXPECTED_NEW_IDS = ["G11-N1", "G11-N2", "G11-N3"]

ALLOWED = frozenset({"go", "no-go", "defer-to-G12+", "strategic_override"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|—|-)?$", re.I)
GAP_ID_RE = re.compile(r"^(R[1-5]|U[1-3]|C[1-3])$")
DEFER_ID_RE = re.compile(r"^(M\d|RD\d|SAFE-GPU|G10-N\d)")


def parse_section_rows(text: str, section_heading: str, id_pattern: re.Pattern) -> list[dict[str, str]]:
    """解析指定 `## N. xxx` 节内首个 markdown 表的 ID 行（列名映射到 dict）。"""
    lines = text.splitlines()
    start = None
    for i, line in enumerate(lines):
        if line.startswith("## ") and section_heading in line:
            start = i + 1
            break
    if start is None:
        return []
    rows: list[dict[str, str]] = []
    headers: list[str] = []
    in_table = False
    for line in lines[start:]:
        if line.startswith("## "):
            break
        if not line.strip().startswith("|"):
            if in_table and rows:
                break
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if not cells:
            continue
        if cells[0] == "ID" or cells[0] == "候选 ID":
            headers = cells
            in_table = True
            continue
        if set(cells[0]) <= {"-", ":"}:
            continue
        if not in_table or not headers:
            continue
        if len(cells) < len(headers):
            cells += [""] * (len(headers) - len(cells))
        row = {headers[i]: cells[i] for i in range(len(headers))}
        first = cells[0]
        if id_pattern.match(first):
            rows.append(row)
    return rows


def cell_empty(v: str) -> bool:
    s = (v or "").strip()
    return (not s) or bool(EMPTY_RE.match(s))


def check(text: str) -> list[str]:
    """骨架期行级机核；返回违例清单（空 = PASS）。"""
    findings: list[str] = []

    gap_rows = parse_section_rows(text, "法定输入 11 差距行", GAP_ID_RE)
    defer_rows = parse_section_rows(text, "G10 defer-to-G11+ 18 行", DEFER_ID_RE)
    new_rows = parse_section_rows(text, "G11 新增候选行", re.compile(r"^G11-N\d"))

    # --- 行集闭集对账 ---
    gap_ids = [r.get("ID", "") for r in gap_rows]
    if gap_ids != EXPECTED_GAP_IDS:
        findings.append(
            f"[closure] §1 差距行集 ≠ 11 行闭集：缺 {sorted(set(EXPECTED_GAP_IDS) - set(gap_ids))}，"
            f"多 {sorted(set(gap_ids) - set(EXPECTED_GAP_IDS))}，顺序漂移={gap_ids != EXPECTED_GAP_IDS and set(gap_ids) == set(EXPECTED_GAP_IDS)}"
        )
    defer_ids = [r.get("ID", "") for r in defer_rows]
    if defer_ids != EXPECTED_DEFER_IDS:
        findings.append(
            f"[closure] §2 defer 行集 ≠ 18 行闭集：缺 {sorted(set(EXPECTED_DEFER_IDS) - set(defer_ids))}，"
            f"多 {sorted(set(defer_ids) - set(EXPECTED_DEFER_IDS))}"
        )
    new_ids = [r.get("候选 ID", r.get("ID", "")) for r in new_rows]
    if new_ids != EXPECTED_NEW_IDS:
        findings.append(
            f"[closure] §4 新增候选行集 ≠ 3 行闭集：缺 {sorted(set(EXPECTED_NEW_IDS) - set(new_ids))}，"
            f"多 {sorted(set(new_ids) - set(EXPECTED_NEW_IDS))}"
        )

    # --- 行级机核：裁决枚举合法 + 零空行 ---
    all_rows = gap_rows + defer_rows + new_rows
    for row in all_rows:
        rid = row.get("ID", row.get("候选 ID", "?"))
        verdict = row.get("G11 裁决", row.get("裁决", ""))
        verdict_head = verdict.split("（")[0].strip().strip("*").strip()
        if verdict_head not in ALLOWED:
            findings.append(f"[verdict] {rid} 裁决枚举非法：{verdict!r}（允许 {sorted(ALLOWED)}）")
        for col, val in row.items():
            if cell_empty(val):
                findings.append(f"[no-empty] {rid} 的 {col} 列为空或占位")

    # --- defer 行承接锚「重判条件 + 兜底」字面（列名含「承接锚」即锚列，兼容各表形态） ---
    for row in defer_rows + new_rows:
        rid = row.get("ID", row.get("候选 ID", "?"))
        verdict = row.get("G11 裁决", row.get("裁决", ""))
        if not verdict.startswith("defer-to-G12+"):
            continue
        anchor = ""
        for k, v in row.items():
            if "承接锚" in k:
                anchor = v
                break
        if "重判条件" not in anchor or "兜底" not in anchor:
            findings.append(f"[anchor] {rid} defer 行承接锚缺「重判条件/兜底」字面：{anchor[:60]!r}")

    # --- 承接锚清单（§6）与 defer 行集对账 ---
    anchor_ids: list[str] = []
    in_anchor_section = False
    for line in text.splitlines():
        if line.startswith("## 6. "):
            in_anchor_section = True
            continue
        if in_anchor_section:
            if line.startswith("## "):
                break
            if line.strip().startswith("|") and not line.strip().startswith("| ID") and not set(line.strip().strip("|").split("|")[0].strip()) <= {"-", ":"}:
                cells = [c.strip() for c in line.strip().strip("|").split("|")]
                if cells and re.match(r"^(M\d|SAFE-GPU|G10-N\d|G11-N\d)", cells[0]):
                    anchor_ids.append(cells[0])
    defer_verdict_ids = [
        r.get("ID", r.get("候选 ID", ""))
        for r in defer_rows + new_rows
        if r.get("G11 裁决", r.get("裁决", "")).startswith("defer-to-G12+")
    ]
    if anchor_ids != defer_verdict_ids:
        findings.append(
            f"[anchor-list] §6 承接锚清单行集 ≠ defer 行集：清单 {len(anchor_ids)} 行 vs defer {len(defer_verdict_ids)} 行；"
            f"缺 {sorted(set(defer_verdict_ids) - set(anchor_ids))}，多 {sorted(set(anchor_ids) - set(defer_verdict_ids))}"
        )
    return findings


# ---------------------------------------------------------------------------
# selftest 合成夹具：三节最小正本，不依赖树上文件。
# ---------------------------------------------------------------------------

def _fixture() -> str:
    gap_header = "| ID | 分项名 | 承接锚字面 | measured 基线 | G11 裁决 | 波次/P 级/M### | 裁决理由 | 登记留痕位置 | 最终状态 |"
    defer_header = "| ID | 分项名 | G10.7 承接锚字面 | G11 裁决 | 裁决理由 | 波次/联动面 | 登记留痕位置 | 最终状态 |"
    new_header = "| 候选 ID | 分项名 | 来源 | G11 裁决 | 裁决理由 | 波次归属 | 依据/证据路径 | 承接锚 / 兜底 | 登记留痕位置 |"
    lines = ["# fixture G11_CANDIDATE_DECISIONS", "", "## 1. 法定输入 11 差距行逐行裁决", "", gap_header, "|---|"]
    for rid in EXPECTED_GAP_IDS:
        lines.append(f"| {rid} | x | 锚{rid} | 0.5 | go | G11.3 / P0 / M147 | 理由 | 位置 | go |")
    lines += ["", "## 2. G10 defer-to-G11+ 18 行逐行处置", "", defer_header, "|---|"]
    for rid in EXPECTED_DEFER_IDS:
        if rid in ("M99-clipmap", "G10-N7", "G10-N10"):
            lines.append(f"| {rid} | x | 重判条件已命中 → 兜底 = y | go（G11.3 兑现） | 理由 | G11.3 | 位置 | go |")
        else:
            lines.append(f"| {rid} | x | 重判条件 = x → 兜底 = y | defer-to-G12+ | 理由 | —（非 go） | 位置 | open-defer |")
    lines += ["", "## 4. G11 新增候选行", "", new_header, "|---|"]
    lines.append("| G11-N1 | x | 来源 | go | 理由 | G11.5 | 路径 | 重判条件 = x；兜底 = y | 位置 |")
    lines.append("| G11-N2 | x | 来源 | go | 理由 | G11.5 | 路径 | 重判条件 = x；兜底 = y | 位置 |")
    lines.append("| G11-N3 | x | 来源 | defer-to-G12+（锚定 G14） | 理由 | —（非 go） | 路径 | 重判条件 = x；兜底 = y | 位置 |")
    lines += ["", "## 6. 承接锚清单（defer-to-G12+ 十六行）", "", "| ID | 承接锚（重判条件 → 兜底） | 目标重评期 |", "|---|"]
    for rid in defer_ids_fixture():
        lines.append(f"| {rid} | 重判条件 → 兜底 | G12+ |")
    return "\n".join(lines)


def defer_ids_fixture() -> list[str]:
    out = [r for r in EXPECTED_DEFER_IDS if r not in ("M99-clipmap", "G10-N7", "G10-N10")]
    out.append("G11-N3")
    return out


def run_selftest() -> int:
    good = _fixture()
    defer_header = "| ID | 分项名 | G10.7 承接锚字面 | G11 裁决 | 裁决理由 | 波次/联动面 | 登记留痕位置 | 最终状态 |"

    cases: list[tuple[str, str, str]] = [
        (
            "删除 §1 C3 行 → closure 红",
            "\n".join(l for l in good.splitlines() if not l.startswith("| C3 |")),
            "§1 差距行集 ≠ 11 行闭集",
        ),
        (
            "删除 §2 G10-N17 行 → closure 红",
            "\n".join(l for l in good.splitlines() if not l.startswith("| G10-N17 |")),
            "§2 defer 行集 ≠ 18 行闭集",
        ),
        (
            "§2 M61 裁决改写非法枚举 → verdict 红",
            good.replace("| M61 | x | 重判条件 = x → 兜底 = y | defer-to-G12+ |", "| M61 | x | 重判条件 = x → 兜底 = y | defer-to-G13+ |", 1),
            "裁决枚举非法",
        ),
        (
            "§1 R4 行某列置空 → no-empty 红",
            good.replace("| R4 | x | 锚R4 | 0.5 | go | G11.3 / P0 / M147 | 理由 | 位置 | go |",
                         "| R4 | x | 锚R4 | 0.5 | go |  | 理由 | 位置 | go |", 1),
            "[no-empty] R4",
        ),
        (
            "§2 M52 defer 行承接锚削掉兜底 → anchor 红",
            good.replace("| M52 | x | 重判条件 = x → 兜底 = y | defer-to-G12+ |",
                         "| M52 | x | 重判条件 = x | defer-to-G12+ |", 1),
            "[anchor] M52",
        ),
        (
            "§6 承接锚清单删 G11-N3 行 → anchor-list 红",
            "\n".join(l for l in good.splitlines() if not l.startswith("| G11-N3 | 重判条件")),
            "§6 承接锚清单行集 ≠ defer 行集",
        ),
    ]

    failures = 0
    for name, text, expect in cases:
        got = check(text)
        hit = [f for f in got if expect in f]
        if hit:
            print(f"  RED ok   — {name}（{hit[0]}）")
        elif got:
            print(f"  RED WRONG— {name}：判红但原因不符，期望含 {expect!r}，实测 {got[:2]}")
            failures += 1
        else:
            print(f"  RED MISS — {name}：负样本未被判红")
            failures += 1
    green = check(good)
    if green:
        print("  GREEN MISS — 合成夹具正本本应 PASS：")
        for f in green:
            print(f"    - {f}")
        failures += 1
    else:
        print("  GREEN ok — 合成夹具正本 PASS")
    if failures:
        print(f"[g11_p2_decisions_check] SELFTEST FAIL ({failures})")
        return 1
    print("[g11_p2_decisions_check] SELFTEST PASS (6 RED + 1 GREEN)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", type=str, default=None, help="骨架期 symbolic gate key（机核字面）")
    parser.add_argument("--selftest", action="store_true", help="用受控负样本证明断言能红")
    args = parser.parse_args()
    if args.selftest:
        return run_selftest()

    if args.gate is not None and args.gate != GATE_KEY:
        print(f"[g11_p2_decisions_check] FAIL — gate key 漂移：{args.gate!r} ≠ {GATE_KEY!r}")
        return 1
    if not DECISIONS.exists():
        print(f"[g11_p2_decisions_check] FAIL — 缺事实源 {DECISIONS.relative_to(ROOT)}")
        return 1
    findings = check(DECISIONS.read_text(encoding="utf-8"))
    if findings:
        print(f"[g11_p2_decisions_check] FAIL ({len(findings)} 项)")
        for f in findings:
            print(f"  - {f}")
        return 1
    print(
        "[g11_p2_decisions_check] PASS（骨架期行级机核：§1 法定输入 11 行 + §2 G10 defer 18 行 + §4 新增候选 3 行闭集对账；"
        "裁决枚举合法；零空行；defer 行承接锚「重判条件 + 兜底」齐备；§6 承接锚清单 16 行与 defer 行集对账一致。"
        "G11.6 时按候选全集扩闭集 materialize 并领取数字步骤——post-interlock actual-next-free allocation）"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
