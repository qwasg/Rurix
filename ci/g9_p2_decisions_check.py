#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.7 P2/留档/未触发分项穷举决策门 g9.wave.7.decisions(G9_CONTRACT G-G9-9)。

核验 `milestones/g9/G9_P2_DECISIONS.md`(S6 由 gatekeeper 落盘):
冻结 ID 集合全等、决策枚举合法、单元格非空、go/no-go/defer 义务字段、
defer 必有承接锚字面。只读文档,不代绿实现门;同构 ci/g8_p2_decisions_check.py。

G9.2 开工前本骨架随蜂群基设落盘:此时决策表尚不存在,`--gate` 必须红
(诚实「表未落盘」而非假绿),`--selftest` 必须绿。

用法:
  py -3 ci/g9_p2_decisions_check.py --gate g9.wave.7.decisions
  py -3 ci/g9_p2_decisions_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g9_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g9.wave.7.decisions"
NUMERIC_STEP: int | None = None  # post-interlock actual-next-free;G9.7 门 materialize 时实测回填
SUBJECT = "g9_wave7_decisions"
WAVE = "G9.7"
DECISIONS = ROOT / "milestones" / "g9" / "G9_P2_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_wave7_decisions_evidence_schema.json"
CANDIDATE = ROOT / "milestones" / "g9" / "G9_CANDIDATE_DECISIONS.md"

# 冻结 ID 集合 = G9_PLAN §G9.7 候选行集(分项级)——表 materialize 时逐字核对。
FROZEN_IDS = [
    "M52",           # SER(原 no-go;strategic_override→M108 语言层原语,P2 可选)
    "M61",           # mesh shader 第三光栅(原 no-go;strategic_override→M109,P2 可选,顺序硬约束)
    "M99-clipmap",   # 世界辐射缓存世界 clipmap 级(条件制:未 measured 举证只做屏幕级)
    "M100-high",     # ReSTIR 高档(条件制:须多灯 workload 证据,不足只做低档)
    "M114",          # 毛发精确 OIT strand 档(P2,排在 M120 精确档之后)
    "M118-hdr-cal",  # HDR 设备标定层(条件触发;未触发 SKIP=not-triggered 不充绿)
    "M123",          # 双通道 tick 判档(Jolt 单线程成本 measured 硬前置;不足维持 no-go 不充绿)
    "M126",          # Rapier 深造判档(M126 基准报告先行;不成立维持 no-go)
    "M127",          # 神经变形研究子轨(无主线门;NN 权威禁止线;成果另行判档)
    "SAFE-GPU",      # Safe GPU Operator Platform(立项裁决第 2 项 defer 至 G10+)
]
FROZEN_IDS = [s.strip() for s in FROZEN_IDS if s.strip()]
ALLOWED = frozenset({"go", "no-go", "defer-to-G10+", "strategic_override"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|—|-)?$", re.I)


def parse_table(text: str) -> list[dict[str, str]]:
    """解析决策表(| ID | ... | 行)。"""
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
        if cells[0] in ("ID", "---|") or set(cells[0]) <= {"-", ":"}:
            if cells[0] == "ID":
                headers = cells
                in_table = True
            continue
        if not in_table or not headers:
            if re.match(r"^(M\d|SAFE-GPU)", cells[0]):
                in_table = True
                headers = [
                    "ID", "分项名", "P 级/波次", "原触发条件字面",
                    "决策", "一句理由", "依据/证据路径", "承接锚/退出门", "最终状态",
                ]
            else:
                continue
        if len(cells) < len(headers):
            cells += [""] * (len(headers) - len(cells))
        row = {headers[i]: cells[i] for i in range(len(headers))}
        if re.match(r"^(M\d|SAFE-GPU)", row.get("ID", "")):
            rows.append(row)
    return rows


def cell_empty(v: str) -> bool:
    s = (v or "").strip()
    return (not s) or bool(EMPTY_RE.match(s))


def validate_rows(rows: list[dict[str, str]]) -> list[dict]:
    results: list[dict] = []
    ids = [r.get("ID", "") for r in rows]
    set_ok = set(ids) == set(FROZEN_IDS) and len(ids) == len(FROZEN_IDS)
    results.append(
        {
            "id": "set_equality_frozen",
            "status": "PASS" if set_ok else "FAIL",
            "detail": f"got {sorted(set(ids))} (n={len(ids)}); expect frozen {len(FROZEN_IDS)}",
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
        decision = (r.get("决策") or "").strip()
        row_ok = True
        detail_parts: list[str] = []
        if decision not in ALLOWED:
            row_ok = False
            detail_parts.append(f"非法决策 {decision!r}")
        required_always = ["分项名", "P 级/波次", "原触发条件字面", "一句理由", "依据/证据路径", "最终状态"]
        for k in required_always:
            if cell_empty(r.get(k, "")):
                row_ok = False
                detail_parts.append(f"空单元格 {k}")
        if decision == "go":
            if "evidence/" not in (r.get("依据/证据路径") or ""):
                row_ok = False
                detail_parts.append("go 缺 evidence 路径")
            if cell_empty(r.get("承接锚/退出门", "")):
                row_ok = False
                detail_parts.append("go 缺承接锚/退出门")
        elif decision == "no-go":
            ref = r.get("依据/证据路径") or ""
            anchors = ("RD-", "deferred", "CONTRACT", "RFC-", "矩阵", "CAPABILITY", "CANDIDATE", "PLAN")
            if not any(a in ref for a in anchors):
                row_ok = False
                detail_parts.append("no-go 缺 RD/矩阵/契约/计划锚")
        elif decision == "defer-to-G10+":
            anchor = r.get("承接锚/退出门") or ""
            if "G10" not in anchor:
                row_ok = False
                detail_parts.append("defer 缺 G10+ 承接锚")
        elif decision == "strategic_override":
            ref = (r.get("依据/证据路径") or "") + (r.get("一句理由") or "")
            if "override" not in ref.lower() and "deferred" not in ref:
                row_ok = False
                detail_parts.append("strategic_override 缺 deferred history 只追加登记锚")
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
        # 诚实红:表未落盘不是绿(起始正确结论)
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}(G9.7 决策表未落盘;诚实红,不假绿)"}]
    rows = parse_table(p.read_text(encoding="utf-8"))
    results = validate_rows(rows)
    ok = all(x["status"] == "PASS" for x in results)
    return (0 if ok else 1), results


def emit(results: list[dict], overall_ok: bool) -> int:
    if NUMERIC_STEP is None:
        # 门未 materialize:不落 evidence(无数字步骤可填),只打印结论
        for r in results:
            print(f"  FACT  {r['status']:4}  {r['id']}  ({r.get('detail','')})")
        print(f"[g9_p2_decisions] VERDICT = {'PASS' if overall_ok else 'FAIL'}"
              "(numeric_step 未分配:门 materialize 时才落 evidence)")
        return 0 if overall_ok else 1
    if not SCHEMA_PATH.is_file():
        print(f"[g9_p2_decisions] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G9_PLAN §G9.7;CI_GATES;G9_P2_DECISIONS.md",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G9.7 P2/留档/未触发分项穷举决策;defer 必有承接锚;no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=True,
    )
    return code


def run_selftest() -> int:
    # 负样本 1:当前树表未落盘 → --gate 必须红(诚实起始结论)
    code, results = run_check(None)
    if DECISIONS.is_file():
        # 表已落盘(进入 G9.7 后)则本臂语义反转:真表必须绿
        if code != 0:
            print("[selftest] FAIL: 决策表已落盘但核验未绿", file=sys.stderr)
            for r in results:
                if r["status"] != "PASS":
                    print(f"  {r}", file=sys.stderr)
            return 1
        print("[selftest] PASS: 真表绿(表已落盘期)")
    else:
        if code == 0:
            print("[selftest] FAIL: 表未落盘仍绿(假绿)", file=sys.stderr)
            return 1
        print("[selftest] PASS: 表未落盘 → 诚实红(起始正确结论)")

    # 负样本 2:合成表缺行 → 必须红
    good_header = (
        "| ID | 分项名 | P 级/波次 | 原触发条件字面 | 决策 | 一句理由 | 依据/证据路径 | 承接锚/退出门 | 最终状态 |\n"
        "|---|---|---|---|---|---|---|---|---|\n"
    )
    good_row = "| {id} | 分项 | P2/G9.7 | 触发言面 | no-go | 理由 | registry/deferred.json RD-039 | — | open |\n"
    full = good_header + "".join(good_row.format(id=i) for i in FROZEN_IDS)
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "full.md"
        p.write_text(full, encoding="utf-8")
        code, _ = run_check(p)
        if code != 0:
            print("[selftest] FAIL: 合成全表未绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 合成全表绿")
        lines = [ln for ln in full.splitlines() if not ln.strip().startswith("| M127 |")]
        p2 = Path(td) / "bad.md"
        p2.write_text("\n".join(lines) + "\n", encoding="utf-8")
        code, _ = run_check(p2)
        if code == 0:
            print("[selftest] FAIL: 缺行仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺行→红")

        # 负样本 3:defer 无承接锚 → 必须红
        bad_defer = good_header + good_row.format(id="M52") + "".join(
            good_row.format(id=i) for i in FROZEN_IDS[1:-1]
        ) + "| SAFE-GPU | Safe GPU | P2 | 触发言面 | defer-to-G10+ | 理由 | G9_PLAN §5 | — | open-defer |\n"
        p3 = Path(td) / "baddefer.md"
        p3.write_text(bad_defer, encoding="utf-8")
        code, _ = run_check(p3)
        if code == 0:
            print("[selftest] FAIL: defer 无 G10 承接锚仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: defer 无承接锚→红")

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
