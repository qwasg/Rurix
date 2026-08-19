#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.5a P2 穷举决策波）
"""G13.5a P2/留档/未触发分项穷举决策门 g13.wave.5.decisions（G13_CONTRACT G-G13-7；
同构 ci/g12_p2_decisions_check.py 全量形态）。

核验 `milestones/g13/G13_P2_DECISIONS.md`（2026-08-19 v1.0 落盘）：
冻结 31 行候选闭集全等（§1 G12 defer 承接 22 行终态裁决 + §2 G13 新增候选 7 行
终态裁决 + §3 G13 期内新增 G13-N8/N9 双差距登记表处置行）、决策枚举合法
（go/no-go/defer-to-G14+/strategic_override）、零空行（全列非空）、承接锚
「重判条件 + 兜底」字面、defer 行必含 G14+ 重评窗、go 行 evidence 义务、
no-go 行 RD/矩阵/契约锚义务；外加四横向机核——
  ① 与 G13_ACCEPTANCE_MAP §1 五 P0（M-a~M-e）互斥：P2 行 ID 不得命中任何已 go
    M### 裸 token（M167~M171 闭集；G13-N1~N5 分项级 closed-go 留痕行不互斥）；
  ② deferred.json history 对账：G13.5a P2 登记恰好 RD-040 +1（M52 G13.4 Lumen
    化 workload 重评窗结论 = 未命中 maintain-defer）/ RD-041 +1（G10-N5 FSR/
    DirectSR 分项 G13.2 M-a 兑现完结 closed-go 登记），零新 RD（max=RD-044），
    status 全 open 0-byte；
  ③ G13.1 候选决策表对账：§1 22 行 G12 defer 承接行 ID 在 G13_CANDIDATE_DECISIONS
    在册 + 裁决面字面承接（go 行 = G10-N5 唯一，其余 21 行 defer-to-G14+）；
  ④ G13.4 门产双差距登记表对账：g13_ue_upscale_gap_registry.json 8 行 +
    g13_ue_lumen_gap_registry.json 2 行全 P2（suggested_priority 机核）——
    §3 G13-N8/N9 行穷举覆盖；g12_ue_pt_gap_registry.json 10 行终态 0-byte
    （git 工作树 porcelain 空 + 行数 10 维持，只消费不回写）。
只读文档与 registry，不代绿实现门；no-go/defer 如实保持 open 不写进全绿叙述。

materialize：numeric_step=243（落盘前实测 CI_step.next_free=243 顺位领取）。

用法：
  py -3 ci/g13_p2_decisions_check.py --gate g13.wave.5.decisions
  py -3 ci/g13_p2_decisions_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g10_wave_exit_lib as wel  # noqa: E402
from g11_wave_exit_lib import DEFERRED_PATH  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g13.wave.5.decisions"
NUMERIC_STEP = 243  # 落盘前实测 registry/number_ledger.json CI_step.next_free=243 顺位领取
SUBJECT = "g13_p2_decisions"
WAVE = "G13.5a"
DECISIONS = ROOT / "milestones" / "g13" / "G13_P2_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g13" / "g13_p2_decisions_evidence_schema.json"
ACCEPTANCE_MAP = ROOT / "milestones" / "g13" / "G13_ACCEPTANCE_MAP.md"
CANDIDATE = ROOT / "milestones" / "g13" / "G13_CANDIDATE_DECISIONS.md"
DEFERRED = DEFERRED_PATH
REG_UPSCALE = ROOT / "milestones" / "g13" / "g13_ue_upscale_gap_registry.json"
REG_LUMEN = ROOT / "milestones" / "g13" / "g13_ue_lumen_gap_registry.json"
REG_G12 = ROOT / "milestones" / "g12" / "g12_ue_pt_gap_registry.json"

# 冻结 ID 闭集（31 行）= §1 G12 defer 承接 22 + §2 G13 新增候选 7 + §3 期内新增 2。
FROZEN_IDS = [
    # §1 G12 defer 承接 22 行
    "M61", "M52", "M100-high", "SAFE-GPU", "M127", "M98-l4",
    "M114-strand", "M118-hdr-cal", "M125-adopt3", "G10-N5", "G10-N6",
    "G10-N8", "G10-N11", "G10-N16", "G10-N17", "G11-N3", "G11-N5",
    "G11-N8", "G11-N9", "G12-N10", "G12-N12", "G12-N13",
    # §2 G13 新增候选 7 行
    "G13-N1", "G13-N2", "G13-N3", "G13-N4", "G13-N5", "G13-N6", "G13-N7",
    # §3 G13 期内新增 2 行（M-c/M-d 门产差距登记表处置）
    "G13-N8", "G13-N9",
]
FROZEN_IDS = [s.strip() for s in FROZEN_IDS if s.strip()]
ALLOWED = frozenset({"go", "no-go", "defer-to-G14+", "strategic_override"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|—|-)?$", re.I)
ID_RE = re.compile(r"^(M\d|RD\d|SAFE-GPU|G1[0123]-N\d)")
HEADERS = [
    "ID", "分项名", "来源波次", "原触发条件字面", "裁决",
    "裁决理由", "依据/证据路径", "承接锚", "登记留痕位置", "最终状态",
]
# deferred.json history 对账期望：G13.5a P2 登记恰好 RD-040 +1（M52 重评窗结论）/
# RD-041 +1（G10-N5 兑现完结 closed-go 登记）。
EXPECTED_DEFER_HISTORY = {"RD-040": ["M52"], "RD-041": ["G10-N5"]}
HISTORY_MARKER = "G13.5a"
# G13.1 候选决策表 §1 承接行 22 闭集（对账字面：唯一 go = G10-N5，其余 defer-to-G14+）。
CANDIDATE_CARRY_IDS = FROZEN_IDS[:22]
CANDIDATE_GO_IDS = frozenset({"G10-N5"})
GO_IDS = frozenset({"G10-N5", "G13-N1", "G13-N2", "G13-N3", "G13-N4", "G13-N5"})
NO_GO_IDS = frozenset({"G13-N6"})
# MAP §1 五 P0 已 go M### 裸 token 闭集（M167=M-a…M170=M-d 已 materialize；
# M171=M-e 本波步骤 244 materialize——决策门序先于 M-e，闭集字面含之）。
MAP_GO_TOKENS = frozenset({"M167", "M168", "M169", "M170", "M171"})


def parse_tables(text: str) -> list[dict[str, str]]:
    """解析 §1/§2/§3 三张决策表（| ID | … 十列头；§5 锚清单三列表不入——头集不全等即跳）。"""
    rows: list[dict[str, str]] = []
    headers: list[str] = []
    in_table = False
    for line in text.splitlines():
        if not line.strip().startswith("|"):
            if in_table:
                in_table = False
                headers = []
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if not cells:
            continue
        if cells[0] == "ID":
            headers = cells
            in_table = headers == HEADERS
            continue
        if set(cells[0]) <= {"-", ":"}:
            continue
        if not in_table or headers != HEADERS:
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


def _git_porcelain(rel: str) -> str:
    r = subprocess.run(
        ["git", "status", "--porcelain", "--", rel],
        cwd=ROOT, capture_output=True, text=True,
    )
    return (r.stdout or "").strip()


def validate_rows(
    rows: list[dict[str, str]],
    map_text: str | None = None,
    deferred_data: dict | None = None,
    candidate_text: str | None = None,
    frozen_ids: list[str] | None = None,
) -> list[dict]:
    """行级机核 + 横向对账；各数据面无注入时读真树。"""
    frozen = FROZEN_IDS if frozen_ids is None else frozen_ids
    results: list[dict] = []
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
        decision_full = (r.get("裁决") or "").strip()
        decision = decision_full.strip("*").strip().split("（")[0].split("(")[0].strip().strip("*").strip()
        row_ok = True
        detail_parts: list[str] = []
        if decision not in ALLOWED:
            row_ok = False
            detail_parts.append(f"非法裁决 {decision_full!r}")
        # 零空行：除 ID 外九列全必填；承接锚全行必填，defer 行再加 G14+ 字面
        for k in HEADERS[1:]:
            if cell_empty(r.get(k, "")):
                row_ok = False
                detail_parts.append(f"空单元格 {k}")
        anchor = r.get("承接锚") or ""
        if "重判条件" not in anchor or "兜底" not in anchor:
            row_ok = False
            detail_parts.append("承接锚缺「重判条件/兜底」字面")
        if decision == "defer-to-G14+" and "G14+" not in anchor:
            row_ok = False
            detail_parts.append("defer 缺 G14+ 重评窗字面")
        if decision == "go":
            if rid not in GO_IDS:
                row_ok = False
                detail_parts.append(f"go 行不在已兑现闭集: {rid}")
            if "evidence/" not in (r.get("依据/证据路径") or ""):
                row_ok = False
                detail_parts.append("go 缺 evidence 路径")
        elif decision == "no-go":
            if rid not in NO_GO_IDS:
                row_ok = False
                detail_parts.append(f"no-go 行不在闭集: {rid}")
            ref = r.get("依据/证据路径") or ""
            anchors = ("RD-", "deferred", "CONTRACT", "RFC-", "矩阵", "CAPABILITY", "CANDIDATE", "PLAN", "MAP", "log")
            if not any(a in ref for a in anchors):
                row_ok = False
                detail_parts.append("no-go 缺 RD/矩阵/契约/计划/MAP 锚")
        results.append(
            {
                "id": f"row_{rid}",
                "status": "PASS" if row_ok else "FAIL",
                "detail": "; ".join(detail_parts) if detail_parts else decision_full[:80],
            }
        )

    # 横向机核①：与 G13_ACCEPTANCE_MAP §1 五 P0（M-a~M-e）互斥
    mt = map_text if map_text is not None else (
        ACCEPTANCE_MAP.read_text(encoding="utf-8") if ACCEPTANCE_MAP.is_file() else ""
    )
    p0_rows = re.findall(r"g13\.p0\.m_[a-e]\.", mt)
    hit = sorted(set(ids) & MAP_GO_TOKENS)
    mutex_ok = not hit and len(set(p0_rows)) == 5
    results.append(
        {
            "id": "acceptance_map_mutex",
            "status": "PASS" if mutex_ok else "FAIL",
            "detail": f"MAP §1 P0 行={len(set(p0_rows))}（expect 5）；P2 表命中已 go M### 裸 token: {hit or '无'}",
        }
    )

    # 横向机核②：deferred.json history 对账（G13.5a P2 登记新增条数）
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
    rd_nums = [int(m.group(1)) for e in entries for m in [re.match(r"RD-(\d+)$", e.get("id") or "")] if m]
    if not rd_nums or max(rd_nums) != 44:
        reconcile_ok = False
        rec_parts.append(f"RD max={max(rd_nums) if rd_nums else None} expect 44（零新 RD）")
    status_map = {e.get("id"): e.get("status") for e in entries}
    if any(status_map.get(f"RD-0{n}") != "open" for n in ("34", "39", "40", "41", "42", "43", "44")):
        reconcile_ok = False
        rec_parts.append("RD-034/039/040/041/042/043/044 status 非全 open")
    rec_parts.append(f"{HISTORY_MARKER} history: {sorted((r, len(g)) for r, g in holders.items())}")
    results.append(
        {
            "id": "deferred_history_reconcile",
            "status": "PASS" if reconcile_ok else "FAIL",
            "detail": "; ".join(rec_parts),
        }
    )

    # 横向机核③：G13.1 候选决策表对账（§1 22 行承接 ID 在册 + 裁决字面承接）
    ct = candidate_text if candidate_text is not None else (
        CANDIDATE.read_text(encoding="utf-8") if CANDIDATE.is_file() else ""
    )
    p2_map = {r.get("ID", ""): r for r in rows}
    cand_ok = True
    cand_parts: list[str] = []
    for rid in CANDIDATE_CARRY_IDS:
        if rid not in ct:
            cand_ok = False
            cand_parts.append(f"CANDIDATE 缺行 {rid}")
            continue
        pr = p2_map.get(rid)
        if pr is None:
            cand_ok = False
            cand_parts.append(f"P2 表缺承接行 {rid}")
            continue
        verdict = (pr.get("裁决") or "").strip().strip("*").strip().split("（")[0].split("(")[0].strip().strip("*").strip()
        want = "go" if rid in CANDIDATE_GO_IDS else "defer-to-G14+"
        if verdict != want:
            cand_ok = False
            cand_parts.append(f"{rid} P2 裁决={verdict!r} ≠ 承接字面 {want!r}")
    if not ct:
        cand_ok = False
        cand_parts.append("G13_CANDIDATE_DECISIONS.md 未落盘或不可读")
    cand_parts.append(f"承接行对账 n={sum(1 for rid in CANDIDATE_CARRY_IDS if rid in ct)}/22")
    results.append(
        {
            "id": "candidate_decisions_reconcile",
            "status": "PASS" if cand_ok else "FAIL",
            "detail": "; ".join(cand_parts),
        }
    )

    # 横向机核④：G13.4 门产双差距登记表对账（8+2 全 P2 机核）+ G12 表 10 行 0-byte
    reg_ok = True
    reg_parts: list[str] = []
    for path, want_n, name in (
        (REG_UPSCALE, 8, "upscale"),
        (REG_LUMEN, 2, "lumen"),
    ):
        if not path.is_file():
            reg_ok = False
            reg_parts.append(f"{name} 登记表缺失 {path.name}")
            continue
        doc = wel.load_json(path)
        items = doc.get("items") or []
        bad_pri = [it.get("title") for it in items if it.get("suggested_priority") != "P2"]
        if len(items) != want_n or bad_pri:
            reg_ok = False
            reg_parts.append(f"{name} 行数={len(items)}（expect {want_n}）非 P2 行={bad_pri or '无'}")
        else:
            reg_parts.append(f"{name} {want_n} 行全 P2")
    if REG_G12.is_file():
        g12_doc = wel.load_json(REG_G12)
        g12_items = g12_doc.get("items") or []
        porcelain = _git_porcelain("milestones/g12/g12_ue_pt_gap_registry.json")
        if len(g12_items) != 10 or porcelain:
            reg_ok = False
            reg_parts.append(f"g12 表行数={len(g12_items)}（expect 10）porcelain={porcelain or '空'}")
        else:
            reg_parts.append("g12 表 10 行终态 0-byte（porcelain 空）")
    else:
        reg_ok = False
        reg_parts.append("g12 登记表缺失")
    results.append(
        {
            "id": "gap_registries_reconcile",
            "status": "PASS" if reg_ok else "FAIL",
            "detail": "; ".join(reg_parts),
        }
    )
    return results


def run_check(
    path: Path | None = None,
    map_text: str | None = None,
    deferred_data: dict | None = None,
    candidate_text: str | None = None,
    frozen_ids: list[str] | None = None,
) -> tuple[int, list[dict]]:
    p = path or DECISIONS
    if not p.is_file():
        # 诚实红：表未落盘不是绿
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}（G13.5a 决策表未落盘；诚实红，不假绿）"}]
    rows = parse_tables(p.read_text(encoding="utf-8"))
    results = validate_rows(
        rows,
        map_text=map_text,
        deferred_data=deferred_data,
        candidate_text=candidate_text,
        frozen_ids=frozen_ids,
    )
    ok = all(x["status"] == "PASS" for x in results)
    return (0 if ok else 1), results


def emit(results: list[dict], overall_ok: bool) -> int:
    for r in results:
        print(f"  FACT  {r['status']:4}  {r['id']}  ({r.get('detail','')})")
    if not SCHEMA_PATH.is_file():
        print(f"[g13_p2_decisions] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G13_CONTRACT G-G13-7;G13_P2_DECISIONS.md v1.0;G13_CANDIDATE_DECISIONS v1.0;G13_ACCEPTANCE_MAP §1/§3.3;registry/deferred.json（G13.5a P2 行）;g13_ue_upscale_gap_registry/g13_ue_lumen_gap_registry（G13.4 门产 P2 行集）;g12_ue_pt_gap_registry（10 行终态只消费不回写）;G13_CONTRACT §8.3~§8.5",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G13.5a P2/留档/未触发分项穷举决策（31 行闭集：go 6 closed-go 留痕 + no-go 1 + defer-to-G14+ 24，strategic_override 0）；defer 必有承接锚（重判条件+兜底+G14+ 重评窗）；与 MAP §1 五 P0 互斥；deferred.json history 对账（RD-040 +1〔M52 G13.4 重评窗未命中〕/RD-041 +1〔G10-N5 兑现完结 closed-go〕，零新 RD，status 全 open）；G13.1 候选决策表 22 行承接对账；G13.4 双差距登记表 8+2 行全 P2 机核 + G12 表 10 行 0-byte；触发评估窗三结论如实登记（M52 未命中/G10-N17 未消费/G11-N5 未齐备）；no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=True,
    )
    return code


def _synth_row(rid: str) -> str:
    if rid in GO_IDS:
        decision = "go"
        anchor = "重判条件 = 已兑现完结无重判面（异动时按只追加程序新立分项）；兜底 = 既有面维持，门绿 0-byte"
        ref = "evidence/g13_fixture_20260819T000000Z.json"
    elif rid in NO_GO_IDS:
        decision = "no-go"
        anchor = "重判条件 = G14+ 所属会话提交并立项评审后按只追加程序重判；兜底 = 既有面维持"
        ref = "registry/deferred.json RD-041 / G13_CONTRACT / G13_CANDIDATE_DECISIONS"
    else:
        decision = "defer-to-G14+"
        anchor = "重判条件 = G14+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持"
        ref = "registry/deferred.json RD-040 / G13_CANDIDATE_DECISIONS §1"
    return f"| {rid} | 分项 | G13.1 | 触发条件字面 | {decision} | 理由 | {ref} | {anchor} | 留痕位置 | open |\n"


def run_selftest() -> int:
    failures = 0
    good_header = (
        "| " + " | ".join(HEADERS) + " |\n"
        "|" + "---|" * len(HEADERS) + "\n"
    )
    full = good_header + "".join(_synth_row(i) for i in FROZEN_IDS)

    # 正样本 1：真表（已落盘）必须绿
    code, results = run_check(None)
    if not DECISIONS.is_file():
        if code == 0:
            print("[selftest] FAIL: 表未落盘仍绿（假绿）", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 表未落盘 → 诚实红（起始正确结论）")
    else:
        if code != 0:
            print("[selftest] FAIL: 决策表已落盘但核验未绿", file=sys.stderr)
            for r in results:
                if r["status"] != "PASS":
                    print(f"  {r}", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 真表 31 行绿")

    with tempfile.TemporaryDirectory(prefix="g13_p2_selftest_") as td:
        # 正样本 2：合成全表（真树 MAP/deferred/CANDIDATE/登记表对账）必须绿
        p = Path(td) / "full.md"
        p.write_text(full, encoding="utf-8")
        code, res = run_check(p)
        if code != 0:
            print("[selftest] FAIL: 合成全表未绿", file=sys.stderr)
            for r in res:
                if r["status"] != "PASS":
                    print(f"  {r}", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 合成全表绿")

        # 负样本 1：缺行 → 必须红
        lines = [ln for ln in full.splitlines() if not ln.strip().startswith("| G13-N8 |")]
        p2 = Path(td) / "bad.md"
        p2.write_text("\n".join(lines) + "\n", encoding="utf-8")
        code, _ = run_check(p2)
        if code == 0:
            print("[selftest] FAIL: 缺行仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 缺行→红")

        # 负样本 2：defer 行承接锚缺 G14+ → 必须红
        bad_defer = full.replace(
            "重判条件 = G14+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持",
            "重判条件 = 触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持",
        )
        p3 = Path(td) / "baddefer.md"
        p3.write_text(bad_defer, encoding="utf-8")
        code, _ = run_check(p3)
        if code == 0:
            print("[selftest] FAIL: defer 缺 G14+ 承接锚仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: defer 缺 G14+ 承接锚→红")

        # 负样本 3：非法裁决枚举 → 必须红
        bad_enum = full.replace("| no-go |", "| maybe |", 1)
        p4 = Path(td) / "badenum.md"
        p4.write_text(bad_enum, encoding="utf-8")
        code, _ = run_check(p4)
        if code == 0:
            print("[selftest] FAIL: 非法裁决仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 非法裁决→红")

        # 负样本 4：已 go M### 裸 token 混入 → 必须红（MAP 互斥）
        bad_mutex = full + _synth_row("X").replace("| X |", "| M171 |", 1)
        p5 = Path(td) / "badmutex.md"
        p5.write_text(bad_mutex, encoding="utf-8")
        code, _ = run_check(p5)
        if code == 0:
            print("[selftest] FAIL: M171 裸 token 混入仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 已 go M### 裸 token 混入→红")

    if failures:
        print(f"[g13_p2_decisions] SELFTEST FAIL ({failures})", file=sys.stderr)
        return 1
    print("[g13_p2_decisions] SELFTEST PASS（2 GREEN + 4 RED）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    code, results = run_check(None)
    return emit(results, code == 0)


if __name__ == "__main__":
    sys.exit(main())
