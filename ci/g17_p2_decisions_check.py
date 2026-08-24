#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Claude Fable 5（G17.7a P2 穷举决策波）
"""G17.7a P2 穷举决策门（g17.wave.7a.decisions，步骤 306；G17_CONTRACT G-G17-8）。

核验 `milestones/g17/G17_P2_DECISIONS.md`：
§1 = 候选表 15 行终态裁决（十四行 defer-to-G18+ 维持 + G15-MD-F1 期窗终态）；
§2 = open RD 八条映射（条目级 status 全 open）；
§3 = G17 期内行终态（G17-N1~N4 必在 + 期内新增 finding 行 G17-M*-F* 允许追加）。
裁决枚举 = go/closed-go/no-go/defer-to-G18+/strategic_override（defer-to-G17+ 不再合法）；
零空行；承接锚「重判条件 = …；兜底 = …」；defer 行 G18+ 重评窗字面；
closed-go 行 evidence/ 真跑件引用；deferred.json 对账（RD 八条 open 零新 RD）。

用法：
  py -3 ci/g17_p2_decisions_check.py --gate g17.wave.7a.decisions
  py -3 ci/g17_p2_decisions_check.py --verify-latest
  py -3 ci/g17_p2_decisions_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
from g11_wave_exit_lib import DEFERRED_PATH  # noqa: E402
from g17_candidate_decisions_check import (  # noqa: E402
    SEC1_IDS, SEC2_IDS, SEC3_IDS, parse_tables, cell_empty, _decision_base,
)

ROOT = wel.ROOT
GATE_KEY = "g17.wave.7a.decisions"
NUMERIC_STEP = 306  # post-interlock 实测顺位领取
SUBJECT = "g17_p2_decisions_check"
WAVE = "G17.7a"
P2_PATH = ROOT / "milestones/g17/G17_P2_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones/g17/g17_p2_decisions_check_evidence_schema.json"
SOURCE_REF = (
    "G17_CONTRACT G-G17-8;G17_P2_DECISIONS.md v1.0;G17_CANDIDATE_DECISIONS.md v1.0"
    "（19 行闭集法定输入）;registry/deferred.json（RD 八条 open 机核）"
)
NEW_FINDING_RE = re.compile(r"^G17-M[A-E]-[FN]\d+$")


def _row_findings_p2(r: list[str]) -> list[str]:
    parts: list[str] = []
    if len(r) < 10:
        return [f"列数不足 10（实测 {len(r)}）"]
    decision = r[4].replace("**", "").strip()
    base = _decision_base(decision)
    if base is None:
        parts.append(f"非法裁决 {decision!r}")
    for i, cell in enumerate(r):
        if cell_empty(cell):
            parts.append(f"空单元格 col{i}")
            break
    anchor = r[7]
    if "重判条件" not in anchor or "兜底" not in anchor:
        parts.append("承接锚缺「重判条件 = …；兜底 = …」字面")
    if base == "defer-to-G18+" and "G18+" not in anchor:
        parts.append("defer 缺 G18+ 重评窗字面")
    if base == "closed-go" and "evidence/" not in r[6]:
        parts.append("closed-go 行依据/证据路径缺 evidence/ 真跑件")
    return parts


def validate(tables: dict[str, list[list[str]]], deferred_data: dict | None = None) -> list[dict]:
    results: list[dict] = []
    sec1, sec2, sec3 = tables["sec1"], tables["sec2"], tables["sec3"]
    sec1_ids = [r[0] for r in sec1 if r]
    set1_ok = set(sec1_ids) == set(SEC1_IDS) and len(sec1_ids) == len(SEC1_IDS)
    results.append({
        "id": "sec1_set_equality",
        "status": "PASS" if set1_ok else "FAIL",
        "detail": f"§1 终态 {len(sec1_ids)} 行 vs 冻结 15 行"
        + ("" if set1_ok else f"; diff={sorted(set(SEC1_IDS) ^ set(sec1_ids))}"),
    })
    sec3_ids = [r[0] for r in sec3 if r]
    base_ok = set(SEC3_IDS).issubset(set(sec3_ids))
    extra = [i for i in sec3_ids if i not in SEC3_IDS]
    extra_ok = all(NEW_FINDING_RE.match(i) for i in extra)
    results.append({
        "id": "sec3_new_rows_closed",
        "status": "PASS" if base_ok and extra_ok and len(sec3_ids) == len(set(sec3_ids)) else "FAIL",
        "detail": f"§3 = G17-N1~N4 必在（{base_ok}）+ 期内新增 {extra or '零行'}"
                  f"（形态 G17-M*-F*/N* 合法 = {extra_ok}）",
    })
    for r in sec1 + sec3:
        rid = r[0] if r else "?"
        parts = _row_findings_p2(r)
        results.append({
            "id": f"row_{rid}",
            "status": "PASS" if not parts else "FAIL",
            "detail": "; ".join(parts) if parts else f"{r[4].replace('**', '').strip()}",
        })
    sec2_ids = [r[0] for r in sec2 if r]
    sec2_ok = set(sec2_ids) == set(SEC2_IDS)
    bad2 = [r[0] for r in sec2 if len(r) >= 3 and r[2] != "open"]
    results.append({
        "id": "sec2_rd_open",
        "status": "PASS" if sec2_ok and not bad2 else "FAIL",
        "detail": f"§2 RD 八条闭集 = {sec2_ok}；非 open 行 = {bad2 or '无'}",
    })
    dd = deferred_data if deferred_data is not None else (
        wel.load_json(DEFERRED_PATH) if DEFERRED_PATH.is_file() else {"entries": []}
    )
    status_map = {e.get("id"): e.get("status") for e in dd.get("entries") or []}
    rd_bad = [rd for rd in SEC2_IDS if status_map.get(rd) != "open"]
    rd_nums = [int(m.group(1)) for e in dd.get("entries") or []
               for m in [re.match(r"RD-(\d+)$", e.get("id") or "")] if m]
    new_rd_note = f"max=RD-{max(rd_nums):03d}" if rd_nums else "空"
    results.append({
        "id": "deferred_rd_open_reconcile",
        "status": "PASS" if not rd_bad else "FAIL",
        "detail": f"deferred.json RD 八条 status 全 open（{new_rd_note}——战后新增 RD 按只追加程序合法）"
        if not rd_bad else f"非 open: {rd_bad}",
    })
    return results


def run_check(path: Path | None = None, deferred_data: dict | None = None) -> tuple[int, list[dict]]:
    p = path or P2_PATH
    if not p.is_file():
        return 1, [{"id": "file", "status": "FAIL",
                    "detail": f"missing {p}（P2 穷举表未落盘；诚实红不假绿）"}]
    results = validate(parse_tables(p.read_text(encoding="utf-8")), deferred_data=deferred_data)
    ok = all(x["status"] == "PASS" for x in results)
    return (0 if ok else 1), results


def run_gate() -> int:
    code, results = run_check()
    overall = code == 0
    if not SCHEMA_PATH.is_file():
        print(f"[g17_p2] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    for r in results:
        print(f"  FACT  {r['status']:4}  {r['id']}  ({r.get('detail','')[:120]})")
    ecode, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G17.7a P2 穷举决策门：§1 15 行终态（十四行 defer-to-G18+ 维持 + G15-MD-F1 期窗终态）"
              "+ §2 RD 八条 open + §3 G17-N1~N4 终态 + 期内新增 finding 行；裁决枚举合法/零空行/"
              "承接锚纪律/defer G18+ 字面/closed-go evidence 引用/deferred 对账；"
              "no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=overall,
    )
    return 0 if (overall and ecode == 0) else 1


def verify_latest() -> int:
    p = wel.load_latest_evidence(SUBJECT)
    if p is None:
        print(f"[g17_p2] verify-latest FAIL: 无 {SUBJECT} evidence", file=sys.stderr)
        return 1
    doc = wel.load_json(p)
    ok = doc.get("host_section_pass") is True and all(
        f.get("status") == "PASS" for f in doc.get("extra_facts", [])
    )
    print(f"[g17_p2] verify-latest {'PASS' if ok else 'FAIL'}: {p.name}")
    return 0 if ok else 1


def _synth_p2(*, drop: str | None = None, bad_enum: bool = False) -> str:
    head = "| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |\n|---|---|---|---|---|---|---|---|---|---|\n"
    sec1 = ""
    for rid in SEC1_IDS:
        if rid == drop:
            continue
        if rid == "G15-MD-F1":
            dec = "closed-go（G17 期窗兑现完结）" if not bad_enum else "defer-to-G17+"
            ref = "evidence/g17_m_d_t100_final_verdict_20260824T000000Z.json"
        else:
            dec = "defer-to-G18+"
            ref = "milestones/g17/G17_CANDIDATE_DECISIONS.md §1 行"
        sec1 += (f"| {rid} | 分项 | G17.1 | 「x → y」 | {dec} | 理由 | {ref} | "
                 f"重判条件 = G18+ 窗重判；兜底 = 维持 | 本表 §1 行 | 终态 |\n")
    sec2 = "".join(f"| {rid} | t | open | 维持 open | 无 | 理由 | 本表 §2 行 |\n" for rid in SEC2_IDS)
    sec2_head = "| RD | title（摘要） | 条目级 status | G17.7a 处置 | 联动面 | 裁决理由 | 留痕位置 |\n|---|---|---|---|---|---|---|\n"
    sec3 = "".join(
        f"| {rid} | 分项 | G17 | 「x → y」 | closed-go（兑现完结） | 理由 | "
        f"evidence/g17_x_20260824T000000Z.json | 重判条件 = G18+ 窗重判；兜底 = 维持 | 本表 §3 行 | closed-go |\n"
        for rid in SEC3_IDS
    )
    return ("## 1. 候选表 15 行终态裁决\n\n" + head + sec1
            + "\n## 2. open RD 逐条映射\n\n" + sec2_head + sec2
            + "\n## 3. G17 期内行终态\n\n" + head + sec3)


def run_selftest() -> int:
    failures = 0
    with tempfile.TemporaryDirectory(prefix="g17_p2_selftest_") as td:
        good = Path(td) / "good.md"
        good.write_text(_synth_p2(), encoding="utf-8")
        code, res = run_check(good)
        if code == 0:
            print("  GREEN ok — 合成正本全绿")
        else:
            print(f"  GREEN MISS — {[r for r in res if r['status'] != 'PASS'][:3]}")
            failures += 1
        bad1 = Path(td) / "bad1.md"
        bad1.write_text(_synth_p2(drop="M61"), encoding="utf-8")
        c1, r1 = run_check(bad1)
        if c1 != 0 and any(r["id"] == "sec1_set_equality" and r["status"] == "FAIL" for r in r1):
            print("  RED ok   — 缺行 M61 → sec1 闭集红")
        else:
            print("  RED MISS — 缺行未检出")
            failures += 1
        bad2 = Path(td) / "bad2.md"
        bad2.write_text(_synth_p2(bad_enum=True), encoding="utf-8")
        c2, r2 = run_check(bad2)
        if c2 != 0 and any(r["id"] == "row_G15-MD-F1" and r["status"] == "FAIL" for r in r2):
            print("  RED ok   — defer-to-G17+ 非法枚举红")
        else:
            print("  RED MISS — 非法枚举未检出")
            failures += 1
    if failures:
        print(f"[g17_p2] SELFTEST FAIL ({failures})")
        return 1
    print("[g17_p2] SELFTEST PASS (2 RED + 1 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return verify_latest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
