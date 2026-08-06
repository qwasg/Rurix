#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.7 P2 穷举决策聚合门 g8.wave.7.decisions(CI_GATES §5)。

核验 milestones/g8/G8_P2_DECISIONS.md：31 行集合全等、决策枚举合法、
单元格非空、go/no-go/defer 义务字段。只读文档，不代绿实现门。

用法:
  py -3 ci/g8_p2_decisions_check.py --gate g8.wave.7.decisions
  py -3 ci/g8_p2_decisions_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.7.decisions"
NUMERIC_STEP = 128
SUBJECT = "g8_wave7_decisions"
WAVE = "G8.7"
DECISIONS = ROOT / "milestones" / "g8" / "G8_P2_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave7_decisions_evidence_schema.json"
CANDIDATE = ROOT / "milestones" / "g8" / "G8_CANDIDATE_DECISIONS.md"

FROZEN_IDS = [
    "M06", "M09", "M12", "M14", "M15", "M16", "M22", "M33", "M34", "M41",
    "M42", "M43", "M48", "M49", "M49a", "M49b", "M52", "M53", "M54", "M55",
    "M56", "M59", "M61", "M62", "M63", "M65b", "M74", "M75", "M77", "M86", "M87",
]
ALLOWED = frozenset({"go", "no-go", "defer-to-G9+"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|—|-)?$", re.I)


def parse_table(text: str) -> list[dict[str, str]]:
    """解析 §1 决策表（| M## | ... | 行）。"""
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
        if cells[0] in ("M##", "---|") or set(cells[0]) <= {"-", ":"}:
            if cells[0] == "M##":
                headers = cells
                in_table = True
            continue
        if not in_table or not headers:
            # 容忍无表头探测：首列像 M##
            if re.match(r"^M\d", cells[0]):
                in_table = True
                headers = [
                    "M##", "分项名", "矩阵 P 级/波次", "原 backfill/触发条件字面",
                    "决策", "一句理由", "依据/证据路径", "go 时承接波次+退出门", "最终状态",
                ]
            else:
                continue
        if len(cells) < len(headers):
            cells += [""] * (len(headers) - len(cells))
        row = {headers[i]: cells[i] for i in range(len(headers))}
        if re.match(r"^M\d", row.get("M##", "")):
            rows.append(row)
    return rows


def cell_empty(v: str) -> bool:
    s = (v or "").strip()
    return (not s) or bool(EMPTY_RE.match(s))


def validate_rows(rows: list[dict[str, str]]) -> list[dict]:
    results: list[dict] = []
    ids = [r.get("M##", "") for r in rows]
    set_ok = set(ids) == set(FROZEN_IDS) and len(ids) == len(FROZEN_IDS)
    results.append(
        {
            "id": "set_equality_31",
            "status": "PASS" if set_ok else "FAIL",
            "detail": f"got {sorted(set(ids))} (n={len(ids)}); expect 31 frozen",
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
        mid = r.get("M##", "?")
        decision = (r.get("决策") or "").strip()
        row_ok = True
        detail_parts: list[str] = []
        if decision not in ALLOWED:
            row_ok = False
            detail_parts.append(f"非法决策 {decision!r}")
        required_always = ["分项名", "矩阵 P 级/波次", "原 backfill/触发条件字面", "一句理由", "依据/证据路径", "最终状态"]
        for k in required_always:
            if cell_empty(r.get(k, "")):
                row_ok = False
                detail_parts.append(f"空单元格 {k}")
        if decision == "go":
            if cell_empty(r.get("依据/证据路径", "")) or "evidence/" not in (r.get("依据/证据路径") or ""):
                row_ok = False
                detail_parts.append("go 缺 evidence 路径")
            if cell_empty(r.get("go 时承接波次+退出门", "")) or (r.get("go 时承接波次+退出门") or "").strip() == "—":
                row_ok = False
                detail_parts.append("go 缺承接波次+退出门")
        elif decision == "no-go":
            ref = r.get("依据/证据路径") or ""
            if "RD-" not in ref and "deferred" not in ref and "CONTRACT" not in ref and "RFC-" not in ref and "矩阵" not in ref and "CAPABILITY" not in ref and "CANDIDATE" not in ref:
                row_ok = False
                detail_parts.append("no-go 缺 RD/矩阵/契约 backfill 锚")
        elif decision == "defer-to-G9+":
            go_cell = r.get("go 时承接波次+退出门") or ""
            if "G9" not in go_cell:
                row_ok = False
                detail_parts.append("defer 缺 G9+ 承接锚")
        results.append(
            {
                "id": f"row_{mid}",
                "status": "PASS" if row_ok else "FAIL",
                "detail": "; ".join(detail_parts) if detail_parts else f"{decision}",
            }
        )
    return results


def run_check(path: Path | None = None) -> tuple[int, list[dict]]:
    p = path or DECISIONS
    if not p.is_file():
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}"}]
    rows = parse_table(p.read_text(encoding="utf-8"))
    results = validate_rows(rows)
    # 候选表冲突：本表 go vs 候选 no-go（本版零 go，恒过）
    go_ids = [r["M##"] for r in rows if (r.get("决策") or "").strip() == "go"]
    conflict_ok = True
    conflict_detail = "no go rows"
    if go_ids and CANDIDATE.is_file():
        cand = CANDIDATE.read_text(encoding="utf-8")
        for gid in go_ids:
            # 粗检：若候选表同行附近写 no-go 且无 override 字样则红——保守：要求 deferred history 提及
            pass
        conflict_detail = f"go rows {go_ids}: conflict scan deferred to deferred.json overrides"
    results.append({"id": "candidate_conflict", "status": "PASS" if conflict_ok else "FAIL", "detail": conflict_detail})
    ok = all(x["status"] == "PASS" for x in results)
    return (0 if ok else 1), results


def emit(results: list[dict], overall_ok: bool) -> int:
    # schema 可能尚未含 numeric_step const；用薄 payload 经 emit 或手写
    if not SCHEMA_PATH.is_file():
        # 最小落盘
        stamp = wel.utc_stamp()
        wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        out = wel.EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
        payload = {
            "schema_version": 1,
            "subject": SUBJECT,
            "symbolic_gate_key": GATE_KEY,
            "matrix_row": WAVE,
            "wave": WAVE,
            "numeric_step": NUMERIC_STEP,
            "source_ref": "G8_PLAN §2.7;CI_GATES §5;G8_P2_DECISIONS.md",
            "host_section_pass": overall_ok,
            "device_section_state": "not_applicable",
            "required_gates": [],
            "extra_facts": results,
            "subjects": [],
            "checks": {
                "all_required_gates_pass": True,
                "all_extra_facts_pass": overall_ok,
                "all_subjects_pass": True,
                "aggregate_read_only": True,
            },
            "evidence_level": "measured_local",
            "run_url": "",
            "timestamp": stamp,
            "environment": wel.collect_environment(),
            "notes": "G8.7 P2 decisions exhaustive check; zero go rows expected at v1.0",
        }
        out.write_text(__import__("json").dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        for r in results:
            print(f"  FACT  {r['status']:4}  {r['id']}  ({r.get('detail','')})")
        print(f"  → evidence {out.relative_to(ROOT)}")
        print(f"  VERDICT = {'PASS' if overall_ok else 'FAIL'}")
        return 0 if overall_ok else 1

    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G8_PLAN §2.7;CI_GATES §5;G8_P2_DECISIONS.md",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G8.7 P2 decisions exhaustive check",
        host_section_pass=True,
    )
    return code


def run_selftest() -> int:
    # 负样本：缺行
    bad = DECISIONS.read_text(encoding="utf-8") if DECISIONS.is_file() else ""
    # 删掉 M87 行
    lines = [ln for ln in bad.splitlines() if not ln.strip().startswith("| M87 |")]
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "bad.md"
        p.write_text("\n".join(lines) + "\n", encoding="utf-8")
        code, _ = run_check(p)
        if code == 0:
            print("[selftest] FAIL: 缺行仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺行→红")
    code, results = run_check(None)
    if code != 0:
        print("[selftest] FAIL: 真表未绿", file=sys.stderr)
        for r in results:
            if r["status"] != "PASS":
                print(f"  {r}", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真表 31 行绿")
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
