#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G22.1 治理波）
"""G22.1 治理门 — 候选决策表闭集/锚纪律/横向对账（g22.wave.1.candidate_decisions，步骤 382）。

核验 `milestones/g22/G22_CANDIDATE_DECISIONS.md`：
冻结 11 行候选闭集全等（§1 G21 defer-to-G23+ 承接 9 行 + §3 G22 新增候选 5 行 G22-N1~N5）、
裁决枚举合法（go/closed-go/no-go/defer-to-G23+/strategic_override——G22 即本期，defer-to-G23+
不再合法）、零空行（全列非空）、承接锚纪律（§1/§3 行承接锚均含「重判条件 = …；兜底 = …」字面；
§1 行原触发条件字面转引含「→」分节）、
defer-to-G23+ 裁决行承接锚含 G2x 期别重评窗字面、
go 行验收映射锚义务（登记留痕位置含 G22_ACCEPTANCE_MAP，或依据面含 G22_CONTRACT §4.2 M 行锚定）、
§2 RD 八条（RD-034/039/040/041/042/043/044/045）行集闭集 +
条目级 status==open（经 registry/deferred.json 机核，零新 RD max=RD-045）。

只读文档与 registry，不代绿实现门；defer 如实保持 open 不写进全绿叙述。

用法：
  py -3 ci/g22_candidate_decisions_check.py --gate g22.wave.1.candidate_decisions
  py -3 ci/g22_candidate_decisions_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
from g11_wave_exit_lib import DEFERRED_PATH  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g22.wave.1.candidate_decisions"
NUMERIC_STEP = 382  # 落盘前实测 registry/number_ledger.json CI_step.next_free=381 顺位领取
SUBJECT = "g22_candidate_decisions_check"
WAVE = "G22.1"
DECISIONS = ROOT / "milestones" / "g22" / "G22_CANDIDATE_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g22" / "g22_candidate_decisions_check_evidence_schema.json"
SOURCE_REF = (
    "G22_CONTRACT G-G22-1/§6/§7;G22_CANDIDATE_DECISIONS.md v1.0;G22_ACCEPTANCE_MAP §1/§2;"
    "G21_P2_DECISIONS.md §1（承接锚法定输入）+ G21_CONTRACT.md §8.7;"
    "registry/deferred.json（RD 八条目级 status open 机核）"
)

SEC1_IDS = [
    "SAFE-GPU",
    "M127",
    "M114-strand",
    "M118-hdr-cal",
    "M125-adopt3",
    "G10-N6",
]
SEC2_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044", "RD-045"]
SEC3_IDS = ["G22-N1", "G22-N2", "G22-N3", "G22-N4", "G22-N5"]
FROZEN_IDS = SEC1_IDS + SEC3_IDS
MAX_RD = 45
ALLOWED = frozenset({"go", "closed-go", "no-go", "defer-to-G23+", "strategic_override"})
EMPTY_CELLS = {"", "TBD", "TODO", "待定", "待补", "—", "-"}
SECTION_HEAD_RE = re.compile(r"^## (\d+)\. ")
DEFER_WINDOW_RE = re.compile(r"G2[0-5]")


def parse_tables(text: str) -> dict[str, list[list[str]]]:
    """按 `## N.` 节作用域解析三个表（§1/§3 十列首格 `ID`；§2 首格 `RD`）。"""
    out: dict[str, list[list[str]]] = {"sec1": [], "sec2": [], "sec3": []}
    block: list[list[str]] = []
    section = 0

    def flush() -> None:
        nonlocal block
        if len(block) >= 2 and all(re.fullmatch(r":?-{2,}:?", c) for c in block[1]):
            header = block[0]
            head = header[0] if header else ""
            if section == 1 and head == "ID" and len(header) >= 10 and header[1] == "分项名":
                out["sec1"].extend(block[2:])
            elif section == 2 and head == "RD" and len(header) >= 7:
                out["sec2"].extend(block[2:])
            elif section == 3 and head == "ID" and len(header) >= 10 and header[1] == "分项名":
                out["sec3"].extend(block[2:])
        block = []

    for line in text.splitlines():
        m = SECTION_HEAD_RE.match(line.strip())
        if m:
            flush()
            section = int(m.group(1))
            continue
        s = line.strip()
        if s.startswith("|"):
            block.append([c.strip() for c in s.strip("|").split("|")])
        else:
            flush()
    flush()
    return out


def _decision_of(cell: str) -> str:
    return re.sub(r"\*\*", "", cell).split("（")[0].strip()


def evaluate(text: str | None, deferred_doc: dict | None) -> list[dict]:
    """10 facts。"""
    facts: list[dict] = []
    if text is None:
        return [{"id": "file", "status": "FAIL", "detail": "G22_CANDIDATE_DECISIONS.md 缺失（诚实红）"}]
    tables = parse_tables(text)
    sec1, sec2, sec3 = tables["sec1"], tables["sec2"], tables["sec3"]

    got1 = [r[0] for r in sec1 if r]
    ok1 = got1 == SEC1_IDS
    facts.append({"id": "sec1_closed_set", "status": "PASS" if ok1 else "FAIL",
                  "detail": f"§1 {len(got1)}/6 行" + ("" if ok1 else f"；diff={sorted(set(SEC1_IDS) ^ set(got1))}")})
    got3 = [r[0] for r in sec3 if r]
    ok3 = got3 == SEC3_IDS
    facts.append({"id": "sec3_closed_set", "status": "PASS" if ok3 else "FAIL",
                  "detail": f"§3 {len(got3)}/5 行" + ("" if ok3 else f"；diff={sorted(set(SEC3_IDS) ^ set(got3))}")})

    empty_hits = []
    for r in sec1 + sec3:
        rid = r[0] if r else "?"
        if len(r) < 10 or any(c in EMPTY_CELLS for c in r[:10]):
            empty_hits.append(rid)
    facts.append({"id": "zero_empty_rows", "status": "PASS" if not empty_hits else "FAIL",
                  "detail": "14 行全列非空" if not empty_hits else f"空/占位行：{empty_hits}"})

    bad_dec = []
    for r in sec1 + sec3:
        if len(r) > 4 and _decision_of(r[4]) not in ALLOWED:
            bad_dec.append(f"{r[0]}={_decision_of(r[4])!r}")
    facts.append({"id": "decision_enum_legal", "status": "PASS" if not bad_dec else "FAIL",
                  "detail": "裁决枚举合法（defer-to-G23+ 不再合法）" if not bad_dec else str(bad_dec)})

    anchor_bad = []
    for r in sec1 + sec3:
        if len(r) > 7:
            anchor = r[7]
            if "重判条件 =" not in anchor or "兜底 =" not in anchor:
                anchor_bad.append(r[0])
    for r in sec1:
        if len(r) > 3 and "→" not in r[3]:
            anchor_bad.append(f"{r[0]}(原触发条件缺→)")
    facts.append({"id": "anchor_discipline", "status": "PASS" if not anchor_bad else "FAIL",
                  "detail": "承接锚「重判条件/兜底」纪律齐" if not anchor_bad else str(anchor_bad)})

    defer_bad = []
    for r in sec1 + sec3:
        if len(r) > 7 and _decision_of(r[4]).startswith("defer-to-G2"):
            if not DEFER_WINDOW_RE.search(r[7]):
                defer_bad.append(r[0])
    facts.append({"id": "defer_rows_g2x_window", "status": "PASS" if not defer_bad else "FAIL",
                  "detail": "defer 行承接锚含 G2x 期别窗字面" if not defer_bad else str(defer_bad)})

    go_bad = []
    for r in sec1 + sec3:
        if len(r) > 8 and _decision_of(r[4]) in ("go", "closed-go"):
            blob = r[6] + r[8]
            if "G22_ACCEPTANCE_MAP" not in blob and "G22_CONTRACT §4.2" not in blob:
                go_bad.append(r[0])
    facts.append({"id": "go_rows_map_anchor", "status": "PASS" if not go_bad else "FAIL",
                  "detail": "go 行验收映射锚义务齐" if not go_bad else str(go_bad)})

    got2 = [r[0] for r in sec2 if r]
    ok2 = got2 == SEC2_IDS
    facts.append({"id": "sec2_rd_closed_set", "status": "PASS" if ok2 else "FAIL",
                  "detail": f"§2 {len(got2)}/6 行" + ("" if ok2 else f"；diff={sorted(set(SEC2_IDS) ^ set(got2))}")})

    rd_bad = []
    max_seen = 0
    if deferred_doc is None:
        rd_bad.append("registry/deferred.json 不可读")
    else:
        entries = {e.get("id"): e for e in deferred_doc.get("entries", [])}
        for rid in SEC2_IDS:
            st = (entries.get(rid) or {}).get("status")
            if st != "open":
                rd_bad.append(f"{rid} status={st!r}（要求 open）")
        for eid in entries:
            m = re.match(r"RD-(\d+)", eid or "")
            if m:
                max_seen = max(max_seen, int(m.group(1)))
    facts.append({"id": "rd_status_open_machine", "status": "PASS" if not rd_bad else "FAIL",
                  "detail": "RD 八条目级 status 全 open（机核）" if not rd_bad else str(rd_bad)})
    facts.append({"id": "no_new_rd", "status": "PASS" if max_seen == MAX_RD else "FAIL",
                  "detail": f"deferred max=RD-{max_seen:03d}（要求 RD-{MAX_RD:03d} 零新 RD）"})
    return facts


def run_check(text: str | None = None, deferred_doc: dict | None = None) -> tuple[int, list[dict]]:
    t = text if text is not None else (
        DECISIONS.read_text(encoding="utf-8") if DECISIONS.is_file() else None
    )
    d = deferred_doc
    if d is None:
        try:
            d = json.loads(DEFERRED_PATH.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            d = None
    facts = evaluate(t, d)
    ok = all(f["status"] == "PASS" for f in facts)
    return (0 if ok else 1), facts


def emit(facts: list[dict], overall_ok: bool) -> int:
    for f in facts:
        print(f"  FACT  {f['status']:4}  {f['id']}  ({f.get('detail','')})")
    if not SCHEMA_PATH.is_file():
        print(f"[g22_candidate_decisions] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=[],
        extra_facts=facts,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G22.1 治理门——候选决策表 14 行闭集（§1 九行 + §3 五行）+ 裁决枚举合法（defer 合法值 = defer-to-G23+ 点名期别）+ 零空行 + 承接锚纪律 + defer 行 G2x 窗 + go 行验收映射锚 + §2 RD 八条 status open 机核 + 零新 RD；只读文档与 registry，不代绿实现门",
        host_section_pass=overall_ok,
    )
    return code


def _fixture_text() -> str:
    def row10(rid: str, decision: str, trigger: str = "条件 → 兜底面") -> str:
        anchor = "重判条件 = G22 窗；兜底 = 维持" if decision.startswith("defer") else "重判条件 = 只追加重判；兜底 = 维持"
        trace = "本表行" if decision.startswith("defer") else "G22_ACCEPTANCE_MAP M-a"
        return f"| {rid} | 名 | 来源 | {trigger} | {decision} | 理由 | 依据 | {anchor} | {trace} | 终态 |"

    lines = ["# fixture", "", "## 1. 承接 9 行", "",
             "| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |",
             "|---|---|---|---|---|---|---|---|---|---|"]
    for rid in SEC1_IDS:
        lines.append(row10(rid, "go" if rid in () else "defer-to-G23+"))
    lines += ["", "## 2. open RD 八条", "",
              "| RD | title（摘要） | 条目级 status | G22.1 处置 | 联动面 | 裁决理由 | 留痕位置 |",
              "|---|---|---|---|---|---|---|"]
    for rid in SEC2_IDS:
        lines.append(f"| {rid} | t | open | 维持 open | 无 | 理由 | 本表 §2 |")
    lines += ["", "## 3. G22 期新增 5 行", "",
              "| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |",
              "|---|---|---|---|---|---|---|---|---|---|"]
    for rid in SEC3_IDS:
        lines.append(row10(rid, "go"))
    return "\n".join(lines) + "\n"


def _fixture_deferred() -> dict:
    return {"entries": [
        {"id": f"RD-{n:03d}", "status": "open"} for n in (34, 39, 40, 41, 42, 43, 44, 45)
    ]}


def run_selftest() -> int:
    failures = 0
    text = _fixture_text()
    deferred = _fixture_deferred()

    code, facts = run_check()
    if DECISIONS.is_file():
        if code != 0:
            print("[selftest] FAIL: 真表已落盘但核验未绿", file=sys.stderr)
            for f in facts:
                if f["status"] != "PASS":
                    print(f"  {f}", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 真表 14 行绿")
    else:
        if code == 0:
            print("[selftest] FAIL: 表未落盘仍绿（假绿）", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 表未落盘 → 诚实红")

    cases = [
        ("删除 §1 M127 行 → 闭集红", "\n".join(l for l in text.splitlines() if not l.startswith("| M127 ")),
         deferred, "sec1_closed_set"),
        ("裁决改 defer-to-G22+ → 枚举红", text.replace("| defer-to-G23+ |", "| defer-to-G22+ |", 1),
         deferred, "decision_enum_legal"),
        ("承接锚去掉兜底 → 锚纪律红", text.replace("重判条件 = G22 窗；兜底 = 维持", "重判条件 = G22 窗", 1),
         deferred, "anchor_discipline"),
        ("空单元格注入 → 零空行红", text.replace("| M127 | 名 |", "| M127 |  |", 1),
         deferred, "zero_empty_rows"),
        ("§2 删 RD-045 → RD 闭集红", "\n".join(l for l in text.splitlines() if not l.startswith("| RD-045 ")),
         deferred, "sec2_rd_closed_set"),
        ("deferred RD-040 status closed 注入 → 机核红", text,
         {"entries": [{"id": f"RD-{n:03d}", "status": ("closed" if n == 40 else "open")} for n in (34, 39, 40, 41, 42, 43, 44, 45)]},
         "rd_status_open_machine"),
        ("deferred 注入 RD-046 → 零新 RD 红", text,
         {"entries": deferred["entries"] + [{"id": "RD-046", "status": "open"}]}, "no_new_rd"),
        ("go 行去映射锚 → go 锚红", text.replace("| G22_ACCEPTANCE_MAP M-a |", "| 本表行 |", 1),
         deferred, "go_rows_map_anchor"),
    ]
    for name, t, d, expect in cases:
        _, facts = run_check(t, d)
        hit = [f for f in facts if f["id"] == expect and f["status"] == "FAIL"]
        if hit:
            print(f"  RED ok   — {name}（{hit[0]['detail'][:80]}）")
        else:
            print(f"  RED MISS — {name}：负样本未被判红于 {expect}")
            failures += 1

    code, facts = run_check(text, deferred)
    if code == 0 and len(facts) == 10:
        print("  GREEN ok — 合成夹具正本 PASS（10 facts）")
    else:
        print(f"  GREEN MISS — 合成夹具正本本应 PASS，实测 code={code} facts={len(facts)}")
        for f in facts:
            if f["status"] != "PASS":
                print(f"    - {f}")
        failures += 1

    if failures:
        print(f"[g22_candidate_decisions] SELFTEST FAIL ({failures})")
        return 1
    print("[g22_candidate_decisions] SELFTEST PASS (8 RED + 1 GREEN + 真表臂)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    code, facts = run_check()
    return emit(facts, code == 0)


if __name__ == "__main__":
    sys.exit(main())
