#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.1 治理波 validator）
"""G10.7 P2/留档/未触发分项穷举决策门 g10.wave.7.decisions(G10_CONTRACT G-G10-9)——骨架。

骨架形态（G10.1 治理波落盘）：
  - FROZEN_IDS 为空闭集——G10 期 P2/留档/未触发分项候选全集要等 G10.2~G10.6 各波
    真实登记面落地后才可冻结（同 G9 先例：骨架期十行 → materialize 期按候选全集扩闭集）；
  - `milestones/g10/G10_P2_DECISIONS.md` 当前未落盘，`--gate` 诚实红（exit 1），不假绿；
  - FROZEN_IDS 空 = 骨架期硬护栏：任何决策表（即使合形）一律判红「候选闭集未冻结」，
    杜绝骨架冒充全绿；
  - evidence schema 目标路径只冻结不预建（CI_GATES §1.2），骨架期不落 evidence、
    不占 numeric CI step；numeric_step 一律 post-interlock actual-next-free allocation。

行级机核（materialize 期直接复用，selftest 以注入闭集证明红绿两臂）：
  决策枚举合法（go / no-go / defer-to-G11+ / strategic_override）、零空行（全列非空）、
  承接锚必含「重判条件 + 兜底」字面、defer 行必含 G11+ 重评窗、go 行 evidence 义务、
  no-go 行 RD/矩阵/契约锚义务；与 G10_ACCEPTANCE_MAP 14 key 互斥等横向机核随
  FROZEN_IDS 冻结一并落盘（同构 ci/g9_p2_decisions_check.py 全量形态）。

用法:
  py -3 ci/g10_p2_decisions_check.py --gate g10.wave.7.decisions
  py -3 ci/g10_p2_decisions_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g10_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g10.wave.7.decisions"
NUMERIC_STEP = None  # post-interlock actual-next-free allocation；骨架期零数字 claim
SUBJECT = "g10_p2_decisions"
WAVE = "G10.7"
DECISIONS = ROOT / "milestones" / "g10" / "G10_P2_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_p2_decisions_evidence_schema.json"
ACCEPTANCE_MAP = ROOT / "milestones" / "g10" / "G10_ACCEPTANCE_MAP.md"

# 骨架期空闭集：G10.7 穷举时按「G10_CANDIDATE_DECISIONS 实记全集未进 14 key 验收面者
# + G10.2~G10.6 期内新增 not-triggered/no-go 登记面去重」冻结，与 G10_P2_DECISIONS §1 逐字对账。
FROZEN_IDS: list[str] = []
ALLOWED = frozenset({"go", "no-go", "defer-to-G11+", "strategic_override"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|—|-)?$", re.I)
ID_RE = re.compile(r"^(M\d|RD\d|SAFE-GPU|G10-N\d)")
HEADERS = [
    "ID", "分项名", "来源波次", "原触发条件字面", "裁决",
    "裁决理由", "依据/证据路径", "承接锚", "登记留痕位置", "最终状态",
]


def parse_table(text: str) -> list[dict[str, str]]:
    """解析 §1 决策表(| ID | ... | 行;止于表后首个非 | 行)。"""
    rows: list[dict[str, str]] = []
    in_table = False
    headers: list[str] = []
    for line in text.splitlines():
        if not line.strip().startswith("|"):
            if in_table and rows:
                break
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if not cells:
            continue
        if cells[0] == "ID" or set(cells[0]) <= {"-", ":"}:
            if cells[0] == "ID":
                headers = cells
                in_table = True
            continue
        if not in_table or not headers:
            if ID_RE.match(cells[0]):
                in_table = True
                headers = HEADERS
            else:
                continue
        if len(cells) < len(headers):
            cells += [""] * (len(headers) - len(cells))
        row = {headers[i]: cells[i] for i in range(len(headers))}
        if ID_RE.match(row.get("ID", "")):
            rows.append(row)
    return rows


def cell_empty(v: str) -> bool:
    s = (v or "").strip()
    return (not s) or bool(EMPTY_RE.match(s))


def validate_rows(
    rows: list[dict[str, str]],
    frozen_ids: list[str] | None = None,
) -> list[dict]:
    """行级机核；frozen_ids 缺省取模块级 FROZEN_IDS（骨架期为空闭集）。"""
    frozen = FROZEN_IDS if frozen_ids is None else frozen_ids
    results: list[dict] = []
    if not frozen:
        # 骨架期硬护栏：候选闭集未冻结，任何表一律红（杜绝骨架冒充全绿）。
        results.append(
            {
                "id": "set_equality_frozen",
                "status": "FAIL",
                "detail": "FROZEN_IDS 空闭集（G10.7 候选全集未冻结；骨架期诚实红，任何决策表不充绿）",
            }
        )
        return results
    ids = [r.get("ID", "") for r in rows]
    set_ok = set(ids) == set(frozen) and len(ids) == len(frozen)
    results.append(
        {
            "id": "set_equality_frozen",
            "status": "PASS" if set_ok else "FAIL",
            "detail": f"got n={len(ids)} unique={len(set(ids))}; expect frozen {len(frozen)}"
            + ("" if set_ok else f"; diff={sorted(set(frozen) ^ set(ids))}"),
        }
    )
    if len(ids) != len(set(ids)):
        results.append(
            {
                "id": "no_duplicate_ids",
                "status": "FAIL",
                "detail": f"duplicates: {[x for x in ids if ids.count(x) > 1]}",
            }
        )
    else:
        results.append({"id": "no_duplicate_ids", "status": "PASS", "detail": "ok"})

    for r in rows:
        rid = r.get("ID", "?")
        decision = (r.get("裁决") or "").strip()
        row_ok = True
        detail_parts: list[str] = []
        if decision not in ALLOWED:
            row_ok = False
            detail_parts.append(f"非法裁决 {decision!r}")
        # 零空行:除 ID 外九列全必填(承接锚全行必填,defer 行再加 G11+ 字面)
        for k in HEADERS[1:]:
            if cell_empty(r.get(k, "")):
                row_ok = False
                detail_parts.append(f"空单元格 {k}")
        anchor = r.get("承接锚") or ""
        if "重判" not in anchor or "兜底" not in anchor:
            row_ok = False
            detail_parts.append("承接锚缺「重判条件/兜底」字面")
        if decision == "defer-to-G11+" and "G11+" not in anchor:
            row_ok = False
            detail_parts.append("defer 缺 G11+ 重评窗字面")
        if decision == "go":
            if "evidence/" not in (r.get("依据/证据路径") or ""):
                row_ok = False
                detail_parts.append("go 缺 evidence 路径")
        elif decision == "no-go":
            ref = r.get("依据/证据路径") or ""
            anchors = ("RD-", "deferred", "CONTRACT", "RFC-", "矩阵", "CAPABILITY", "CANDIDATE", "PLAN", "MAP")
            if not any(a in ref for a in anchors):
                row_ok = False
                detail_parts.append("no-go 缺 RD/矩阵/契约/计划/MAP 锚")
        results.append(
            {
                "id": f"row_{rid}",
                "status": "PASS" if row_ok else "FAIL",
                "detail": "; ".join(detail_parts) if detail_parts else f"{decision}",
            }
        )
    return results


def run_check(path: Path | None = None) -> tuple[int, list[dict]]:
    p = path or DECISIONS
    if not p.is_file():
        # 诚实红:表未落盘不是绿
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}(G10.7 决策表未落盘;诚实红,不假绿)"}]
    rows = parse_table(p.read_text(encoding="utf-8"))
    results = validate_rows(rows)
    ok = all(x["status"] == "PASS" for x in results)
    return (0 if ok else 1), results


def emit(results: list[dict], overall_ok: bool) -> int:
    """骨架期不落 evidence：schema 只冻结目标路径不预建（CI_GATES §1.2），
    numeric_step 待 post-interlock actual-next-free allocation。"""
    for r in results:
        print(f"  FACT  {r['status']:4}  {r['id']}  ({r.get('detail','')})")
    if not SCHEMA_PATH.is_file():
        print(
            f"[g10_p2_decisions] schema 未落盘（{SCHEMA_PATH.relative_to(ROOT)} 只冻结不预建）；"
            "骨架期不落 evidence、不占数字步骤",
            file=sys.stderr,
        )
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G10_CONTRACT G-G10-9;CI_GATES §5;G10_P2_DECISIONS.md;G10_CANDIDATE_DECISIONS",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G10.7 P2/留档/未触发分项穷举决策；defer 必有承接锚（重判条件+兜底+G11+ 重评窗）；no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=True,
    )
    return code


def _synth_row(rid: str, decision: str) -> str:
    if decision == "defer-to-G11+":
        anchor = "重判条件 = G11+ 重评窗触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持"
        ref = "registry/deferred.json RD-039"
    elif decision == "go":
        anchor = "重判条件 = 回归失败时按只追加程序重判;兜底 = 回退既有面"
        ref = "evidence/g10_fixture_20260815T000000Z.json"
    else:
        anchor = "重判条件 = 触发条件齐备时按只追加程序重判;兜底 = 既有面维持"
        ref = "registry/deferred.json RD-039 / G10_CANDIDATE_DECISIONS"
    return f"| {rid} | 分项 | G10.3 | 触发条件字面 | {decision} | 理由 | {ref} | {anchor} | 留痕位置 | open |\n"


def run_selftest() -> int:
    failures = 0
    fixture_ids = ["M61", "RD042", "G10-N5"]
    good_header = (
        "| " + " | ".join(HEADERS) + " |\n"
        "|" + "---|" * len(HEADERS) + "\n"
    )
    full = good_header + "".join(
        _synth_row(rid, "defer-to-G11+" if rid == "M61" else "no-go") for rid in fixture_ids
    )

    # 正样本 1:真表当前未落盘 → 必须诚实红(起始正确结论)
    code, _ = run_check(None)
    if not DECISIONS.is_file():
        if code == 0:
            print("[selftest] FAIL: 表未落盘仍绿(假绿)", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 表未落盘 → 诚实红(起始正确结论)")
    else:
        # 未来 materialize 后:真表必须绿
        if code != 0:
            print("[selftest] FAIL: 决策表已落盘但核验未绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 真表绿")

    with tempfile.TemporaryDirectory(prefix="g10_p2_selftest_") as td:
        # 负样本 1:骨架期 FROZEN_IDS 空,合成全表经 --gate 路径也必须红
        p = Path(td) / "full.md"
        p.write_text(full, encoding="utf-8")
        code, _ = run_check(p)
        if code == 0:
            print("[selftest] FAIL: 骨架期空闭集下合成表仍绿(骨架冒充全绿)", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 骨架期 FROZEN_IDS 空 → 任何表红")

        # 正样本 2:注入合成闭集后行级机核必须能绿(证明 materialize 期非永红)
        rows = parse_table(full)
        res = validate_rows(rows, frozen_ids=fixture_ids)
        if all(x["status"] == "PASS" for x in res):
            print("[selftest] PASS: 注入闭集 + 合成全表 → 行级机核绿")
        else:
            print("[selftest] FAIL: 注入闭集正本未绿", file=sys.stderr)
            for r in res:
                if r["status"] != "PASS":
                    print(f"  {r}", file=sys.stderr)
            failures += 1

        # 负样本 2:缺行 → 必须红
        lines = [ln for ln in full.splitlines() if not ln.strip().startswith("| RD042 |")]
        rows = parse_table("\n".join(lines) + "\n")
        res = validate_rows(rows, frozen_ids=fixture_ids)
        if any(x["status"] == "FAIL" and x["id"] == "set_equality_frozen" for x in res):
            print("[selftest] PASS: 缺行→红")
        else:
            print("[selftest] FAIL: 缺行仍绿", file=sys.stderr)
            failures += 1

        # 负样本 3:defer 行承接锚缺 G11+ → 必须红
        bad_defer = full.replace(
            "重判条件 = G11+ 重评窗触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持",
            "重判条件 = 触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持",
        )
        res = validate_rows(parse_table(bad_defer), frozen_ids=fixture_ids)
        if any(x["status"] == "FAIL" and "G11+" in x["detail"] for x in res):
            print("[selftest] PASS: defer 缺 G11+ 承接锚→红")
        else:
            print("[selftest] FAIL: defer 缺 G11+ 承接锚仍绿", file=sys.stderr)
            failures += 1

        # 负样本 4:非法裁决枚举 → 必须红
        bad_enum = full.replace("| no-go |", "| maybe |", 1)
        res = validate_rows(parse_table(bad_enum), frozen_ids=fixture_ids)
        if any(x["status"] == "FAIL" and "非法裁决" in x["detail"] for x in res):
            print("[selftest] PASS: 非法裁决枚举→红")
        else:
            print("[selftest] FAIL: 非法裁决枚举仍绿", file=sys.stderr)
            failures += 1

        # 负样本 5:空单元格(裁决理由空)→ 必须红
        bad_empty = full.replace("| 理由 |", "|  |", 1)
        res = validate_rows(parse_table(bad_empty), frozen_ids=fixture_ids)
        if any(x["status"] == "FAIL" and "空单元格" in x["detail"] for x in res):
            print("[selftest] PASS: 空单元格→红")
        else:
            print("[selftest] FAIL: 空单元格仍绿", file=sys.stderr)
            failures += 1

        # 负样本 6:no-go 行缺 RD/矩阵/契约锚 → 必须红
        bad_ref = full.replace("registry/deferred.json RD-039 / G10_CANDIDATE_DECISIONS", "某处留痕", 1)
        res = validate_rows(parse_table(bad_ref), frozen_ids=fixture_ids)
        if any(x["status"] == "FAIL" and "no-go 缺" in x["detail"] for x in res):
            print("[selftest] PASS: no-go 缺锚→红")
        else:
            print("[selftest] FAIL: no-go 缺锚仍绿", file=sys.stderr)
            failures += 1

    if failures:
        print(f"[selftest] FAIL ({failures})")
        return 1
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    code, results = run_check()
    return emit(results, code == 0)


if __name__ == "__main__":
    sys.exit(main())
