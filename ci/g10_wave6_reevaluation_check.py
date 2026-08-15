#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.1 治理波 validator）
"""G10.6 defer 重评窗门 g10.wave.6.reevaluation(G10_CONTRACT G-G10-8)——骨架。

骨架形态（G10.1 治理波落盘）：
  - 十行重判核验表 `milestones/g10/G10_WAVE6_REEVALUATION.md` 当前未落盘，
    `--gate` 诚实红（exit 1），不假绿；
  - DEFER_TEN 为已冻结事实（G9_P2_DECISIONS §1 十项 defer-to-G10+ 行，G10_CONTRACT §6
    「G9 十项 defer 的逐行重判归 G10.6 重评窗」字面），集合全等机核本骨架即生效；
  - evidence schema 目标路径只冻结不预建（CI_GATES §1.2），骨架期不落 evidence、
    不占 numeric CI step；numeric_step 一律 post-interlock actual-next-free allocation。

行级机核（G-G10-8 字面展开，selftest 以合成夹具证明红绿两臂）：
  十行闭集全等、零空行（全列非空）、重判结论枚举合法（maintain-defer / rejudged-go）、
  G10.5 measured 证据列必含 evidence/ 引用且引用文件在树（法定证据输入）、
  未命中行承接锚字面 0-byte 维持（重判后 == 原字面）、命中行承接锚必含 G11+ 承接波次、
  deferred history 只追加留痕列必填（禁静默改判）。
G10.6 materialize 期加固项（本骨架不含）：证据语义必须为 G10.5 A/B measured 面、
deferred.json history 实对账、与 G10_P2_DECISIONS 行集对账。

用法:
  py -3 ci/g10_wave6_reevaluation_check.py --gate g10.wave.6.reevaluation
  py -3 ci/g10_wave6_reevaluation_check.py --selftest
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
GATE_KEY = "g10.wave.6.reevaluation"
NUMERIC_STEP = None  # post-interlock actual-next-free allocation；骨架期零数字 claim
SUBJECT = "g10_wave6_reevaluation"
WAVE = "G10.6"
REEVALUATION = ROOT / "milestones" / "g10" / "G10_WAVE6_REEVALUATION.md"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_wave6_reevaluation_evidence_schema.json"
EVIDENCE_DIR = wel.EVIDENCE_DIR

# G9 十项 defer-to-G10+ 闭集（G9_P2_DECISIONS §1 / G10_CONTRACT §6 法定输入，字面不扩缩）。
DEFER_TEN = [
    "M61",
    "M52",
    "M99-clipmap",
    "M100-high",
    "SAFE-GPU",
    "M127",
    "M98-l4",
    "M114-strand",
    "M118-hdr-cal",
    "M125-adopt3",
]
ALLOWED_VERDICTS = frozenset({"maintain-defer", "rejudged-go"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|—|-)?$", re.I)
ID_RE = re.compile(r"^(M\d|RD\d|SAFE-GPU)")
HEADERS = [
    "ID", "分项名", "原承接锚字面", "G10.5 measured 证据", "重判结论",
    "重判理由", "承接锚字面（重判后）", "deferred history 留痕", "最终状态",
]
EVIDENCE_REF_RE = re.compile(r"evidence/[\w.\-]+\.json")


def parse_table(text: str) -> list[dict[str, str]]:
    """解析 §1 重判核验表(| ID | ... | 行;止于表后首个非 | 行)。"""
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


def evidence_files_on_tree() -> set[str]:
    """evidence/ 目录实有文件名集合（G10.5 measured 证据引用在树机核）。"""
    if not EVIDENCE_DIR.is_dir():
        return set()
    return {p.name for p in EVIDENCE_DIR.glob("*.json")}


def validate_rows(
    rows: list[dict[str, str]],
    frozen_ids: list[str] | None = None,
    evidence_on_tree: set[str] | None = None,
) -> list[dict]:
    """行级机核；frozen_ids 缺省取 DEFER_TEN，evidence_on_tree 缺省读树。"""
    frozen = DEFER_TEN if frozen_ids is None else frozen_ids
    on_tree = evidence_files_on_tree() if evidence_on_tree is None else evidence_on_tree
    results: list[dict] = []
    ids = [r.get("ID", "") for r in rows]
    set_ok = set(ids) == set(frozen) and len(ids) == len(frozen)
    results.append(
        {
            "id": "set_equality_defer_ten",
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
        verdict = (r.get("重判结论") or "").strip()
        row_ok = True
        detail_parts: list[str] = []
        if verdict not in ALLOWED_VERDICTS:
            row_ok = False
            detail_parts.append(f"非法重判结论 {verdict!r}")
        for k in HEADERS[1:]:
            if cell_empty(r.get(k, "")):
                row_ok = False
                detail_parts.append(f"空单元格 {k}")
        # G10.5 measured 证据 = 法定证据输入：必含 evidence/ 引用且在树
        evidence_cell = r.get("G10.5 measured 证据") or ""
        refs = EVIDENCE_REF_RE.findall(evidence_cell)
        if not refs:
            row_ok = False
            detail_parts.append("缺 evidence/ 引用（G10.5 measured 法定证据输入）")
        else:
            missing = [f for f in (ref.split("/", 1)[1] for ref in refs) if f not in on_tree]
            if missing:
                row_ok = False
                detail_parts.append(f"证据引用不在树: {missing}")
        original_anchor = (r.get("原承接锚字面") or "").strip()
        new_anchor = (r.get("承接锚字面（重判后）") or "").strip()
        if verdict == "maintain-defer":
            # 未命中者承接锚字面 0-byte 维持
            if new_anchor != original_anchor:
                row_ok = False
                detail_parts.append("maintain-defer 行承接锚字面非 0-byte 维持")
            if "defer" not in (r.get("最终状态") or ""):
                row_ok = False
                detail_parts.append("maintain-defer 行最终状态缺 defer 字面")
        elif verdict == "rejudged-go":
            # 命中者按只追加程序重判 go 并指定 G11+ 承接波次
            if "G11" not in new_anchor:
                row_ok = False
                detail_parts.append("rejudged-go 行承接锚缺 G11+ 承接波次字面")
        history_cell = r.get("deferred history 留痕") or ""
        if "只追加" not in history_cell:
            row_ok = False
            detail_parts.append("deferred history 留痕缺「只追加」字面（禁静默改判）")
        results.append(
            {
                "id": f"row_{rid}",
                "status": "PASS" if row_ok else "FAIL",
                "detail": "; ".join(detail_parts) if detail_parts else f"{verdict}",
            }
        )
    return results


def run_check(
    path: Path | None = None,
    evidence_on_tree: set[str] | None = None,
) -> tuple[int, list[dict]]:
    p = path or REEVALUATION
    if not p.is_file():
        # 诚实红:十行重判核验表未落盘不是绿
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}(G10.6 重判核验表未落盘;诚实红,不假绿)"}]
    rows = parse_table(p.read_text(encoding="utf-8"))
    results = validate_rows(rows, evidence_on_tree=evidence_on_tree)
    ok = all(x["status"] == "PASS" for x in results)
    return (0 if ok else 1), results


def emit(results: list[dict], overall_ok: bool) -> int:
    """骨架期不落 evidence：schema 只冻结目标路径不预建（CI_GATES §1.2），
    numeric_step 待 post-interlock actual-next-free allocation。"""
    for r in results:
        print(f"  FACT  {r['status']:4}  {r['id']}  ({r.get('detail','')})")
    if not SCHEMA_PATH.is_file():
        print(
            f"[g10_wave6_reevaluation] schema 未落盘（{SCHEMA_PATH.relative_to(ROOT)} 只冻结不预建）；"
            "骨架期不落 evidence、不占数字步骤",
            file=sys.stderr,
        )
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G10_CONTRACT G-G10-8;CI_GATES §5;G10_WAVE6_REEVALUATION.md;G9_P2_DECISIONS §1;registry/deferred.json",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G10.6 defer 重评窗：G9 十项 defer 逐行重判零空行（G10.5 measured 数据为法定证据输入）；命中者只追加程序重判 go 并指定 G11+ 承接波次，未命中者维持 defer 承接锚字面 0-byte；deferred history 只追加禁静默改判",
        host_section_pass=True,
    )
    return code


_ANCHOR = "重判条件 = G10+ 重评窗触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持"


def _synth_row(rid: str, verdict: str, evidence_ref: str) -> str:
    if verdict == "rejudged-go":
        new_anchor = _ANCHOR + ";G11+ 承接波次 = G11 修复期"
        final = "closed(go)"
    else:
        new_anchor = _ANCHOR
        final = "open(defer-to-G11+)"
    return (
        f"| {rid} | 分项 | {_ANCHOR} | {evidence_ref}（measured） | {verdict} | 理由 | "
        f"{new_anchor} | registry/deferred.json 只追加登记 | {final} |\n"
    )


def run_selftest() -> int:
    failures = 0
    fixture_evidence = {"g10_ab_fixture_20260815T000000Z.json"}
    ref = "evidence/g10_ab_fixture_20260815T000000Z.json"
    good_header = (
        "| " + " | ".join(HEADERS) + " |\n"
        "|" + "---|" * len(HEADERS) + "\n"
    )
    full = good_header + "".join(
        _synth_row(rid, "rejudged-go" if rid == "M98-l4" else "maintain-defer", ref)
        for rid in DEFER_TEN
    )

    # 正样本 1:真表当前未落盘 → 必须诚实红(起始正确结论)
    code, _ = run_check(None)
    if not REEVALUATION.is_file():
        if code == 0:
            print("[selftest] FAIL: 表未落盘仍绿(假绿)", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 表未落盘 → 诚实红(起始正确结论)")
    else:
        if code != 0:
            print("[selftest] FAIL: 重判核验表已落盘但核验未绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 真表绿")

    with tempfile.TemporaryDirectory(prefix="g10_wave6_selftest_") as td:
        # 正样本 2:合成十行全表(注入在树证据集)必须绿
        p = Path(td) / "full.md"
        p.write_text(full, encoding="utf-8")
        code, res = run_check(p, evidence_on_tree=fixture_evidence)
        if code != 0:
            print("[selftest] FAIL: 合成十行全表未绿", file=sys.stderr)
            for r in res:
                if r["status"] != "PASS":
                    print(f"  {r}", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 合成十行全表绿")

        # 负样本 1:缺行 → 必须红
        lines = [ln for ln in full.splitlines() if not ln.strip().startswith("| M127 |")]
        p2 = Path(td) / "bad.md"
        p2.write_text("\n".join(lines) + "\n", encoding="utf-8")
        code, _ = run_check(p2, evidence_on_tree=fixture_evidence)
        if code == 0:
            print("[selftest] FAIL: 缺行仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 缺行→红")

        # 负样本 2:maintain-defer 行承接锚被改写(非 0-byte 维持)→ 必须红
        bad_anchor = full.replace(_ANCHOR + " | " + ref, _ANCHOR + "（已放宽） | " + ref, 1)
        p3 = Path(td) / "badanchor.md"
        p3.write_text(bad_anchor, encoding="utf-8")
        code, _ = run_check(p3, evidence_on_tree=fixture_evidence)
        if code == 0:
            print("[selftest] FAIL: 承接锚非 0-byte 维持仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: maintain-defer 承接锚改写→红")

        # 负样本 3:非法重判结论枚举 → 必须红
        bad_enum = full.replace("| maintain-defer |", "| keep |", 1)
        p4 = Path(td) / "badenum.md"
        p4.write_text(bad_enum, encoding="utf-8")
        code, _ = run_check(p4, evidence_on_tree=fixture_evidence)
        if code == 0:
            print("[selftest] FAIL: 非法重判结论仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 非法重判结论→红")

        # 负样本 4:证据引用不在树 → 必须红
        code, _ = run_check(p, evidence_on_tree=set())
        if code == 0:
            print("[selftest] FAIL: 证据引用不在树仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 证据引用不在树→红")

        # 负样本 5:rejudged-go 行承接锚缺 G11+ 承接波次 → 必须红
        bad_go = full.replace(";G11+ 承接波次 = G11 修复期", "", 1)
        p5 = Path(td) / "badgo.md"
        p5.write_text(bad_go, encoding="utf-8")
        code, _ = run_check(p5, evidence_on_tree=fixture_evidence)
        if code == 0:
            print("[selftest] FAIL: rejudged-go 缺 G11+ 承接波次仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: rejudged-go 缺 G11+ 承接波次→红")

        # 负样本 6:deferred history 留痕缺「只追加」字面 → 必须红
        bad_hist = full.replace("registry/deferred.json 只追加登记", "registry/deferred.json 原地改判", 1)
        p6 = Path(td) / "badhist.md"
        p6.write_text(bad_hist, encoding="utf-8")
        code, _ = run_check(p6, evidence_on_tree=fixture_evidence)
        if code == 0:
            print("[selftest] FAIL: history 缺只追加字面仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: history 缺只追加字面→红")

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
