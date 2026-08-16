#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.6/G10.7 波）
"""G10.6 defer 重评窗门 g10.wave.6.reevaluation(G10_CONTRACT G-G10-8)。

核验 `milestones/g10/G10_DEFER_REEVALUATION.md`(2026-08-15 v1.0 落盘):
冻结十锚闭集全等(G9_P2_DECISIONS §1 十项 defer-to-G10+ 行,G10_CONTRACT §6
「G9 十项 defer 的逐行重判归 G10.6 重评窗」字面)、零空行(全列非空)、
重判结论枚举合法(maintain-defer / rejudged-go)、G10.5 measured 证据列必含
evidence/ 引用且引用文件在树(法定证据输入)、证据语义为 G10.5 A/B measured 面
(g10_m139_ab_comparison_/g10_m140_gap_registry_/g10_m141_perf_baseline_
前缀闭集至少其一)、未命中行承接锚字面 0-byte 维持(重判后 == 原字面)且最终
状态含 defer、命中行承接锚必含 G11+ 承接波次、deferred history 留痕列必含
「只追加」(禁静默改判);外加两横向机核——
  ① deferred.json history 对账:G10.6 重评窗登记恰好 RD-039 +1(M61)/
    RD-040 +3(M52/M99-clipmap/M100-high),M99-clipmap 行含 rejudged-go 与
    G11 画质修复期字面,零新 RD(max=RD-044),RD-039/040 status open 0-byte;
  ② G10_P2_DECISIONS 行集对账:十锚行齐备、裁决 == defer-to-G11+、
    rejudged-go 仅 M99-clipmap 一行(其余九锚行不得含 rejudged-go 字面)。
只读文档与 registry,不代绿实现门;G10 零实现面——重判 go 只指承接不实现。

materialize:numeric_step=192(落盘前实测 CI_step.next_free=192 顺位领取);
骨架期 FROZEN 行集与行级机核沿用,本版补证据语义 measured 面与两横向对账
(同构 ci/g9_p2_decisions_check.py 横向机核体例)。

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
NUMERIC_STEP = 192  # 落盘前实测 registry/number_ledger.json CI_step.next_free=192 顺位领取
SUBJECT = "g10_wave6_reevaluation"
WAVE = "G10.6"
REEVALUATION = ROOT / "milestones" / "g10" / "G10_DEFER_REEVALUATION.md"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_wave6_reevaluation_evidence_schema.json"
P2_DECISIONS = ROOT / "milestones" / "g10" / "G10_P2_DECISIONS.md"
DEFERRED = wel.DEFERRED_PATH
EVIDENCE_DIR = wel.EVIDENCE_DIR

# G9 十项 defer-to-G10+ 闭集(G9_P2_DECISIONS §1 / G10_CONTRACT §6 法定输入,字面不扩缩)。
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
# G10.6 实测唯一命中行(R4 P0 + C1 P1 measured 举证);其余九锚 maintain-defer。
REJUDGED_GO_IDS = frozenset({"M99-clipmap"})
# G10.5 A/B measured 证据前缀闭集(法定证据输入语义面)。
MEASURED_PREFIXES = (
    "evidence/g10_m139_ab_comparison_",
    "evidence/g10_m140_gap_registry_",
    "evidence/g10_m141_perf_baseline_",
)
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|—|-)?$", re.I)
ID_RE = re.compile(r"^(M\d|RD\d|SAFE-GPU)")
HEADERS = [
    "ID", "分项名", "原承接锚字面", "G10.5 measured 证据", "重判结论",
    "重判理由", "承接锚字面（重判后）", "deferred history 留痕", "最终状态",
]
EVIDENCE_REF_RE = re.compile(r"evidence/[\w.\-]+\.json")
# deferred.json history 对账期望:G10.6 重评窗登记恰好 RD-039 +1 / RD-040 +3。
EXPECTED_DEFER_HISTORY = {"RD-039": ["M61"], "RD-040": ["M52", "M99-clipmap", "M100-high"]}
HISTORY_MARKER = "G10.6 重评窗"


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
    """evidence/ 目录实有文件名集合(G10.5 measured 证据引用在树机核)。"""
    if not EVIDENCE_DIR.is_dir():
        return set()
    return {p.name for p in EVIDENCE_DIR.glob("*.json")}


def validate_rows(
    rows: list[dict[str, str]],
    frozen_ids: list[str] | None = None,
    evidence_on_tree: set[str] | None = None,
    deferred_data: dict | None = None,
    p2_text: str | None = None,
) -> list[dict]:
    """行级机核 + 横向对账;各数据面无注入时读真树。"""
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
        # G10.5 measured 证据 = 法定证据输入:必含 evidence/ 引用且在树,
        # 且至少一条为 G10.5 A/B measured 面前缀(measured 语义机核)。
        evidence_cell = r.get("G10.5 measured 证据") or ""
        refs = EVIDENCE_REF_RE.findall(evidence_cell)
        if not refs:
            row_ok = False
            detail_parts.append("缺 evidence/ 引用(G10.5 measured 法定证据输入)")
        else:
            missing = [f for f in (ref.split("/", 1)[1] for ref in refs) if f not in on_tree]
            if missing:
                row_ok = False
                detail_parts.append(f"证据引用不在树: {missing}")
            if not any(ref.startswith(MEASURED_PREFIXES) for ref in refs):
                row_ok = False
                detail_parts.append("缺 G10.5 A/B measured 面证据引用(m139/m140/m141 前缀闭集)")
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
            detail_parts.append("deferred history 留痕缺「只追加」字面(禁静默改判)")
        results.append(
            {
                "id": f"row_{rid}",
                "status": "PASS" if row_ok else "FAIL",
                "detail": "; ".join(detail_parts) if detail_parts else f"{verdict}",
            }
        )

    # 横向机核①:deferred.json history 对账(G10.6 重评窗登记新增条数)。
    dd = deferred_data if deferred_data is not None else (
        wel.load_json(DEFERRED) if DEFERRED.is_file() else {"entries": []}
    )
    entries = dd.get("entries") or []
    reconcile_ok = True
    rec_parts: list[str] = []
    holders: dict[str, list[dict]] = {}
    for e in entries:
        marked = [h for h in e.get("history", []) if HISTORY_MARKER in (h.get("event") or "")]
        if marked:
            holders[e.get("id")] = marked
    for rd, keys in EXPECTED_DEFER_HISTORY.items():
        held = holders.get(rd)
        if held is None or len(held) != len(keys):
            reconcile_ok = False
            rec_parts.append(f"{rd} {HISTORY_MARKER}行数={0 if held is None else len(held)} expect {len(keys)}")
            continue
        blob = "\n".join(h.get("event") or "" for h in held)
        missing = [k for k in keys if k not in blob]
        if missing:
            reconcile_ok = False
            rec_parts.append(f"{rd} 缺行 key {missing}")
    extra = sorted(r for r in holders if r not in EXPECTED_DEFER_HISTORY)
    if extra:
        reconcile_ok = False
        rec_parts.append(f"非期望 RD 含{HISTORY_MARKER}行: {extra}")
    m99_blob = "\n".join(h.get("event") or "" for h in holders.get("RD-040", []))
    if "rejudged-go" not in m99_blob or "G11 画质修复期" not in m99_blob:
        reconcile_ok = False
        rec_parts.append("RD-040 缺 M99-clipmap rejudged-go/G11 画质修复期字面")
    rd_nums = [int(m.group(1)) for e in entries for m in [re.match(r"RD-(\d+)$", e.get("id") or "")] if m]
    if not rd_nums or max(rd_nums) != 44:
        reconcile_ok = False
        rec_parts.append(f"RD max={max(rd_nums) if rd_nums else None} expect 44(零新 RD)")
    status_map = {e.get("id"): e.get("status") for e in entries}
    if status_map.get("RD-039") != "open" or status_map.get("RD-040") != "open":
        reconcile_ok = False
        rec_parts.append("RD-039/040 status 非 open")
    rec_parts.append(f"{HISTORY_MARKER} history: {sorted((r, len(g)) for r, g in holders.items())}")
    results.append(
        {
            "id": "deferred_history_reconcile",
            "status": "PASS" if reconcile_ok else "FAIL",
            "detail": "; ".join(rec_parts),
        }
    )

    # 横向机核②:G10_P2_DECISIONS 行集对账(十锚行裁决与重判结论一致)。
    p2 = p2_text if p2_text is not None else (
        P2_DECISIONS.read_text(encoding="utf-8") if P2_DECISIONS.is_file() else ""
    )
    p2_rows = parse_table(p2) if p2 else []
    p2_map = {r.get("ID", ""): r for r in p2_rows}
    p2_ok = True
    p2_parts: list[str] = []
    for rid in DEFER_TEN:
        pr = p2_map.get(rid)
        if pr is None:
            p2_ok = False
            p2_parts.append(f"P2 表缺十锚行 {rid}")
            continue
        decision = (pr.get("裁决") or "").strip()
        if decision != "defer-to-G11+":
            p2_ok = False
            p2_parts.append(f"{rid} P2 裁决 {decision!r} ≠ defer-to-G11+")
        row_blob = " | ".join(pr.values())
        has_rejudged = "rejudged-go" in row_blob
        if rid in REJUDGED_GO_IDS and not has_rejudged:
            p2_ok = False
            p2_parts.append(f"{rid} P2 行缺 rejudged-go 字面(G10.6 重判 go 未承接)")
        if rid not in REJUDGED_GO_IDS and has_rejudged:
            p2_ok = False
            p2_parts.append(f"{rid} P2 行含 rejudged-go 字面(非命中行冒充重判 go)")
    if not p2:
        p2_ok = False
        p2_parts.append("G10_P2_DECISIONS.md 未落盘或不可读")
    p2_parts.append(f"十锚行对账 n={sum(1 for rid in DEFER_TEN if rid in p2_map)}/10")
    results.append(
        {
            "id": "p2_decisions_reconcile",
            "status": "PASS" if p2_ok else "FAIL",
            "detail": "; ".join(p2_parts),
        }
    )
    return results


def run_check(
    path: Path | None = None,
    evidence_on_tree: set[str] | None = None,
    deferred_data: dict | None = None,
    p2_text: str | None = None,
) -> tuple[int, list[dict]]:
    p = path or REEVALUATION
    if not p.is_file():
        # 诚实红:十行重判核验表未落盘不是绿
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}(G10.6 重判核验表未落盘;诚实红,不假绿)"}]
    rows = parse_table(p.read_text(encoding="utf-8"))
    results = validate_rows(
        rows,
        evidence_on_tree=evidence_on_tree,
        deferred_data=deferred_data,
        p2_text=p2_text,
    )
    ok = all(x["status"] == "PASS" for x in results)
    return (0 if ok else 1), results


def emit(results: list[dict], overall_ok: bool) -> int:
    for r in results:
        print(f"  FACT  {r['status']:4}  {r['id']}  ({r.get('detail','')})")
    if not SCHEMA_PATH.is_file():
        print(f"[g10_wave6_reevaluation] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G10_CONTRACT G-G10-8;CI_GATES §5 v1.7;G10_DEFER_REEVALUATION.md v1.0;G10_P2_DECISIONS.md v1.0;G9_P2_DECISIONS §1/§3;g10_gap_registry.json R3/R4/C1 行;registry/deferred.json v1.80",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G10.6 defer 重评窗:G9 十项 defer 逐行重判零空行(G10.5 measured 数据为法定证据输入,m139/m140/m141 前缀闭集 measured 语义机核);M99-clipmap rejudged-go(R4 P0+C1 P1 命中,指定 G11 画质修复期承接,G10 零实现面),其余九锚 maintain-defer 承接锚字面 0-byte;deferred history 只追加对账(RD-039 +1/RD-040 +3,零新 RD)+ G10_P2_DECISIONS 十锚行对账;禁静默改判",
        host_section_pass=True,
    )
    return code


_ANCHOR = "重判条件 = G10+ 重评窗触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持"


def _synth_row(rid: str, verdict: str, evidence_ref: str) -> str:
    if verdict == "rejudged-go":
        new_anchor = _ANCHOR + ";G11+ 承接波次 = G11 画质修复期"
        final = "go(G11 承接)"
    else:
        new_anchor = _ANCHOR
        final = "open-defer(G11+)"
    return (
        f"| {rid} | 分项 | {_ANCHOR} | {evidence_ref}（measured） | {verdict} | 理由 | "
        f"{new_anchor} | registry/deferred.json 只追加登记 | {final} |\n"
    )


def run_selftest() -> int:
    failures = 0
    fixture_evidence = {"g10_m140_gap_registry_20260816T022655Z.json"}
    ref = "evidence/g10_m140_gap_registry_20260816T022655Z.json"
    good_header = (
        "| " + " | ".join(HEADERS) + " |\n"
        "|" + "---|" * len(HEADERS) + "\n"
    )
    full = good_header + "".join(
        _synth_row(rid, "rejudged-go" if rid in REJUDGED_GO_IDS else "maintain-defer", ref)
        for rid in DEFER_TEN
    )

    # 正样本 1:真表(已落盘)必须绿
    code, results = run_check(None)
    if not REEVALUATION.is_file():
        if code == 0:
            print("[selftest] FAIL: 表未落盘仍绿(假绿)", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 表未落盘 → 诚实红(起始正确结论)")
    else:
        if code != 0:
            print("[selftest] FAIL: 重判核验表已落盘但核验未绿", file=sys.stderr)
            for r in results:
                if r["status"] != "PASS":
                    print(f"  {r}", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 真表十行绿")

    with tempfile.TemporaryDirectory(prefix="g10_wave6_selftest_") as td:
        # 正样本 2:合成十行全表(注入在树证据集 + 真树 deferred/P2)必须绿
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
        p2bad = Path(td) / "bad.md"
        p2bad.write_text("\n".join(lines) + "\n", encoding="utf-8")
        code, _ = run_check(p2bad, evidence_on_tree=fixture_evidence)
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
        bad_go = full.replace(";G11+ 承接波次 = G11 画质修复期", "", 1)
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

        # 负样本 7:证据引用非 G10.5 A/B measured 面(前缀闭集外)→ 必须红
        bad_measured = full.replace(ref, "evidence/g10_m128_ue5_capture_environment_20260815T163219Z.json")
        fixture_extra = fixture_evidence | {"g10_m128_ue5_capture_environment_20260815T163219Z.json"}
        p7 = Path(td) / "badmeasured.md"
        p7.write_text(bad_measured, encoding="utf-8")
        code, _ = run_check(p7, evidence_on_tree=fixture_extra)
        if code == 0:
            print("[selftest] FAIL: 非 G10.5 measured 面证据仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 非 G10.5 measured 面证据→红")

        # 负样本 8:deferred.json 对账失配(注入缺 G10.6 行的 deferred 数据)→ 必须红
        real = wel.load_json(DEFERRED)
        stripped = {
            **real,
            "entries": [
                {**e, "history": [h for h in e.get("history", []) if HISTORY_MARKER not in (h.get("event") or "")]}
                for e in real.get("entries", [])
            ],
        }
        code, _ = run_check(p, evidence_on_tree=fixture_evidence, deferred_data=stripped)
        if code == 0:
            print("[selftest] FAIL: deferred history 缺登记仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: deferred history 缺登记→红")

        # 负样本 9:P2 行集对账失配(注入 M99-clipmap 缺 rejudged-go 的 P2 文本)→ 必须红
        real_p2 = P2_DECISIONS.read_text(encoding="utf-8") if P2_DECISIONS.is_file() else ""
        bad_p2 = real_p2.replace("rejudged-go", "maintain-defer")
        code, _ = run_check(p, evidence_on_tree=fixture_evidence, p2_text=bad_p2)
        if code == 0:
            print("[selftest] FAIL: P2 对账失配仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: P2 对账失配→红")

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
