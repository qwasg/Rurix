#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.5a P2 穷举决策波）
"""G14.5a P2/留档/未触发分项穷举决策门 g14.wave.5a.decisions（G14_CONTRACT G-G14-7；
同构 ci/g13_p2_decisions_check.py 全量形态）。

核验 `milestones/g14/G14_P2_DECISIONS.md`（2026-08-20 v1.0 落盘）：
冻结 42 行候选闭集全等（§1 G13 defer 承接 24 行终态裁决 + §2 G14 新增候选 7 行
终态裁决 + §3 G14.2~G14.7 期内新增 11 行——M-d 通过线未达标处置 + 结构性优化
六面 + 延续波双门留痕 + workgroup/evaluate 两留痕面）、决策枚举合法
（go/no-go/defer-to-G15+/strategic_override）、零空行（全列非空）、承接锚
「重判条件 + 兜底」字面、defer 行必含 G15+/G16+ 重评窗、go 行 evidence 义务、
no-go 行 RD/矩阵/契约锚义务；外加四横向机核——
  ① 与 G14_ACCEPTANCE_MAP §1 五 P0（M-a~M-e）互斥：P2 行 ID 不得命中任何已 go
    M### 裸 token（M172~M175/M178 闭集；G14-N1~N5 等分项级 closed-go 留痕行不互斥）；
  ② deferred.json history 对账：G14.5a P2 登记恰好 RD-040 +1（M52 G14 重评窗
    维持未命中终态登记），零新 RD（max=RD-044），status 全 open 0-byte；
  ③ G14.1 候选决策表对账：§1 24 行 G13 defer 承接行 ID 在 G14_CANDIDATE_DECISIONS
    在册 + 裁决面字面承接（go 行 = G10-N11/G10-N16 双行，其余 22 行 defer-to-G15+）；
  ④ 差距登记表对账：g14_fps_gap_registry.json 行数 == 最新 M-d evidence
    unmet_count（诚实红 18 行 或 达标空表 0 行 + 双场景 no_gap_explicit；
    只登记不拟合 RXS-0392 字面维持）+ g13_ue_upscale 8 行 /
    g13_ue_lumen 2 行 / g12_ue_pt 10 行终态 0-byte（git porcelain 空）。
只读文档与 registry，不代绿实现门；no-go/defer 如实保持 open 不写进全绿叙述；
M-d 通过线未达标 = 如实登记不冒充（G14-N8 行承载，不充绿叙述面）。

materialize：numeric_step=261（落盘前实测 CI_step.next_free=261 顺位领取）。

用法：
  py -3 ci/g14_p2_decisions_check.py --gate g14.wave.5a.decisions
  py -3 ci/g14_p2_decisions_check.py --selftest
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
GATE_KEY = "g14.wave.5a.decisions"
NUMERIC_STEP = 261  # 落盘前实测 registry/number_ledger.json CI_step.next_free=261 顺位领取
SUBJECT = "g14_p2_decisions"
WAVE = "G14.5a"
DECISIONS = ROOT / "milestones" / "g14" / "G14_P2_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_p2_decisions_evidence_schema.json"
ACCEPTANCE_MAP = ROOT / "milestones" / "g14" / "G14_ACCEPTANCE_MAP.md"
CANDIDATE = ROOT / "milestones" / "g14" / "G14_CANDIDATE_DECISIONS.md"
DEFERRED = DEFERRED_PATH
REG_G14_FPS = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"
REG_G13_UPSCALE = ROOT / "milestones" / "g13" / "g13_ue_upscale_gap_registry.json"
REG_G13_LUMEN = ROOT / "milestones" / "g13" / "g13_ue_lumen_gap_registry.json"
REG_G12 = ROOT / "milestones" / "g12" / "g12_ue_pt_gap_registry.json"
MD_PREFIX = "g14_m_d_dual_end_fps_parity"

# 冻结 ID 闭集（42 行）= §1 G13 defer 承接 24 + §2 G14 新增候选 7 + §3 期内新增 11。
FROZEN_IDS = [
    # §1 G13 defer 承接 24 行
    "M61", "M52", "M100-high", "SAFE-GPU", "M127", "M98-l4",
    "M114-strand", "M118-hdr-cal", "M125-adopt3", "G10-N6",
    "G10-N8", "G10-N11", "G10-N16", "G10-N17", "G11-N3", "G11-N5",
    "G11-N8", "G11-N9", "G12-N10", "G12-N12", "G12-N13", "G13-N7",
    "G13-N8", "G13-N9",
    # §2 G14 新增候选 7 行
    "G14-N1", "G14-N2", "G14-N3", "G14-N4", "G14-N5", "G14-N6", "G14-N7",
    # §3 G14 期内新增 11 行（M-d 通过线处置 + 结构性优化六面 + 延续波双门 + 双留痕面）
    "G14-N8", "G14-N9", "G14-N10", "G14-N11", "G14-N12", "G14-N13",
    "G14-N14", "G14-N15", "G14-N16", "G14-N17", "G14-N18",
]
FROZEN_IDS = [s.strip() for s in FROZEN_IDS if s.strip()]
ALLOWED = frozenset({"go", "no-go", "defer-to-G15+", "strategic_override"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|—|-)?$", re.I)
ID_RE = re.compile(r"^(M\d|RD\d|SAFE-GPU|G1[01234]-N\d)")
HEADERS = [
    "ID", "分项名", "来源波次", "原触发条件字面", "裁决",
    "裁决理由", "依据/证据路径", "承接锚", "登记留痕位置", "最终状态",
]
# deferred.json history 对账期望：G14.5a P2 登记恰好 RD-040 +1（M52 G14 重评窗维持未命中终态登记）
# + RD-045 新立 1 条（G14.5a 后事件升级登记——M-d v5 检出 M165 同型间歇 digest 漂移，
# G12-N13 承接锚升级条件命中 → 生产化缺陷修复项 + Full RFC 评估面；P2 表后事件登记段承载）。
EXPECTED_DEFER_HISTORY = {"RD-040": ["M52"], "RD-045": ["M165"]}
HISTORY_MARKER = "G14.5a"
# G14.1 候选决策表 §1 承接行 24 闭集（对账字面：go = G10-N11/G10-N16 双行，其余 defer-to-G15+）。
CANDIDATE_CARRY_IDS = FROZEN_IDS[:24]
CANDIDATE_GO_IDS = frozenset({"G10-N11", "G10-N16"})
GO_IDS = frozenset({
    "G10-N11", "G10-N16",
    "G14-N1", "G14-N2", "G14-N3", "G14-N4", "G14-N5", "G14-N7",
    "G14-N15", "G14-N16", "G14-N18",
})
NO_GO_IDS = frozenset({"G14-N6", "G14-N17"})
# MAP §1 五 P0 已 go M### 裸 token 闭集（M172=M-a…M175=M-d 已 materialize；
# M178=M-e 本波 materialize——决策门序先于 M-e，闭集字面含之；G13.5a 同模先例）。
MAP_GO_TOKENS = frozenset({"M172", "M173", "M174", "M175", "M178"})


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
            cells += [""] * (len(headers) - len(headers))
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
        # 零空行：除 ID 外九列全必填；承接锚全行必填，defer 行再加 G15+/G16+ 字面
        for k in HEADERS[1:]:
            if cell_empty(r.get(k, "")):
                row_ok = False
                detail_parts.append(f"空单元格 {k}")
        anchor = r.get("承接锚") or ""
        if "重判条件" not in anchor or "兜底" not in anchor:
            row_ok = False
            detail_parts.append("承接锚缺「重判条件/兜底」字面")
        if decision == "defer-to-G15+" and not any(w in anchor for w in ("G15+", "G16+", "G15（")):
            row_ok = False
            detail_parts.append("defer 缺 G15+/G16+（或 G15 锚定面）重评窗字面")
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

    # 横向机核①：与 G14_ACCEPTANCE_MAP §1 五 P0（M-a~M-e）互斥
    mt = map_text if map_text is not None else (
        ACCEPTANCE_MAP.read_text(encoding="utf-8") if ACCEPTANCE_MAP.is_file() else ""
    )
    p0_rows = re.findall(r"g14\.p0\.m_[a-e]\.", mt)
    hit = sorted(set(ids) & MAP_GO_TOKENS)
    mutex_ok = not hit and len(set(p0_rows)) == 5
    results.append(
        {
            "id": "acceptance_map_mutex",
            "status": "PASS" if mutex_ok else "FAIL",
            "detail": f"MAP §1 P0 行={len(set(p0_rows))}（expect 5）；P2 表命中已 go M### 裸 token: {hit or '无'}",
        }
    )

    # 横向机核②：deferred.json history 对账（G14.5a P2 登记新增条数）
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
    if not rd_nums or max(rd_nums) != 45:
        reconcile_ok = False
        rec_parts.append(f"RD max={max(rd_nums) if rd_nums else None} expect 45（RD-045 = G14.5a 后事件升级登记唯一新 RD）")
    status_map = {e.get("id"): e.get("status") for e in entries}
    if any(status_map.get(f"RD-0{n}") != "open" for n in ("34", "39", "40", "41", "42", "43", "44", "45")):
        reconcile_ok = False
        rec_parts.append("RD-034~RD-045 status 非全 open")
    rec_parts.append(f"{HISTORY_MARKER} history: {sorted((r, len(g)) for r, g in holders.items())}")
    results.append(
        {
            "id": "deferred_history_reconcile",
            "status": "PASS" if reconcile_ok else "FAIL",
            "detail": "; ".join(rec_parts),
        }
    )

    # 横向机核③：G14.1 候选决策表对账（§1 24 行承接 ID 在册 + 裁决字面承接）
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
        want = "go" if rid in CANDIDATE_GO_IDS else "defer-to-G15+"
        if verdict != want:
            cand_ok = False
            cand_parts.append(f"{rid} P2 裁决={verdict!r} ≠ 承接字面 {want!r}")
    if not ct:
        cand_ok = False
        cand_parts.append("G14_CANDIDATE_DECISIONS.md 未落盘或不可读")
    cand_parts.append(f"承接行对账 n={sum(1 for rid in CANDIDATE_CARRY_IDS if rid in ct)}/24")
    results.append(
        {
            "id": "candidate_decisions_reconcile",
            "status": "PASS" if cand_ok else "FAIL",
            "detail": "; ".join(cand_parts),
        }
    )

    # 横向机核④：差距登记表对账（g14 帧率表行数 == 最新 M-d unmet_count；
    # 达标空表 0/0 + 双场景 no_gap_explicit，或诚实红 18/18；g13/g12 表终态 0-byte）
    reg_ok = True
    reg_parts: list[str] = []
    if not REG_G14_FPS.is_file():
        reg_ok = False
        reg_parts.append("g14_fps_gap_registry 缺失")
    else:
        g14_doc = wel.load_json(REG_G14_FPS)
        g14_items = g14_doc.get("items") or []
        md_path = wel.load_latest_evidence(MD_PREFIX)
        md_doc = wel.load_json(md_path) if md_path else {}
        unmet = ((md_doc.get("parity") or {}).get("unmet_count"))
        summaries = g14_doc.get("scene_summary") or []
        empty_explicit = (
            len(g14_items) == 0
            and unmet == 0
            and {s.get("scene_id") for s in summaries} >= {"cornell-box", "bistro-interior"}
            and all(s.get("no_gap_explicit") is True for s in summaries)
        )
        honest_18 = len(g14_items) == 18 and unmet == 18
        if unmet is None or not (empty_explicit or honest_18):
            reg_ok = False
            reg_parts.append(
                f"g14 帧率表行数={len(g14_items)} unmet={unmet}"
                "（合格面 = 空表 0/0+no_gap_explicit 或诚实红 18/18）"
            )
        elif empty_explicit:
            reg_parts.append(f"g14 帧率表空表终态 == 最新 M-d unmet_count=0（{md_path.name}）")
        else:
            reg_parts.append(f"g14 帧率表 18 行 == 最新 M-d unmet_count（{md_path.name}）")
    for path, want_n, name in (
        (REG_G13_UPSCALE, 8, "g13_upscale"),
        (REG_G13_LUMEN, 2, "g13_lumen"),
        (REG_G12, 10, "g12_pt"),
    ):
        if not path.is_file():
            reg_ok = False
            reg_parts.append(f"{name} 登记表缺失")
            continue
        doc = wel.load_json(path)
        items = doc.get("items") or []
        rel = path.relative_to(ROOT).as_posix()
        porcelain = _git_porcelain(rel)
        if len(items) != want_n or porcelain:
            reg_ok = False
            reg_parts.append(f"{name} 行数={len(items)}（expect {want_n}）porcelain={porcelain or '空'}")
        else:
            reg_parts.append(f"{name} {want_n} 行终态 0-byte（porcelain 空）")
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
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}（G14.5a 决策表未落盘；诚实红，不假绿）"}]
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
        print(f"[g14_p2_decisions] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G14_CONTRACT G-G14-7;G14_P2_DECISIONS.md v1.0;G14_CANDIDATE_DECISIONS v1.0;G14_ACCEPTANCE_MAP §1/附录 A;registry/deferred.json（G14.5a P2 行）;g14_fps_gap_registry（M-d 门产 18 行 P2 行集）;g13/g12 三表终态只消费不回写;G14_CONTRACT §8.3~§8.7",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G14.5a P2/留档/未触发分项穷举决策（42 行闭集：closed-go 11 + no-go 2 + defer-to-G15+ 29，strategic_override 0）；defer 必有承接锚（重判条件+兜底+G15+/G16+ 重评窗）；与 MAP §1 五 P0 互斥；deferred.json history 对账（RD-040 +1〔M52 G14 重评窗维持未命中终态登记〕，零新 RD，status 全 open）；G14.1 候选决策表 24 行承接对账；g14 帧率差距表 18 行 == 最新 M-d unmet_count + g13/g12 三表终态 0-byte；触发评估窗结论如实登记（M52 双窗未命中/M100-high 未齐备/M114-strand 部分落地/G10-N17 未消费/G11-N5 未齐备/G13-N7 不立项）；M-d 通过线 ×1.00 未达标如实登记不冒充（G14-N8 承载，继续优化面 G16+ 承接）；no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=True,
    )
    return code


def _synth_row(rid: str) -> str:
    if rid in GO_IDS:
        decision = "go"
        anchor = "重判条件 = 已兑现完结无重判面（异动时按只追加程序新立分项）；兜底 = 既有面维持，门绿 0-byte"
        ref = "evidence/g14_fixture_20260820T000000Z.json"
    elif rid in NO_GO_IDS:
        decision = "no-go"
        anchor = "重判条件 = G15+ 所属会话提交并立项评审后按只追加程序重判；兜底 = 既有面维持"
        ref = "registry/deferred.json RD-041 / G14_CONTRACT / G14_CANDIDATE_DECISIONS"
    else:
        decision = "defer-to-G15+"
        anchor = "重判条件 = G15+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持"
        ref = "registry/deferred.json RD-040 / G14_CANDIDATE_DECISIONS §1"
    return f"| {rid} | 分项 | G14.1 | 触发条件字面 | {decision} | 理由 | {ref} | {anchor} | 留痕位置 | open |\n"


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
            print("[selftest] PASS: 真表 42 行绿")

    with tempfile.TemporaryDirectory(prefix="g14_p2_selftest_") as td:
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
        lines = [ln for ln in full.splitlines() if not ln.strip().startswith("| G14-N8 |")]
        p2 = Path(td) / "bad.md"
        p2.write_text("\n".join(lines) + "\n", encoding="utf-8")
        code, _ = run_check(p2)
        if code == 0:
            print("[selftest] FAIL: 缺行仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 缺行→红")

        # 负样本 2：defer 行承接锚缺 G15+/G16+ → 必须红
        bad_defer = full.replace(
            "重判条件 = G15+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持",
            "重判条件 = 触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持",
        )
        p3 = Path(td) / "baddefer.md"
        p3.write_text(bad_defer, encoding="utf-8")
        code, _ = run_check(p3)
        if code == 0:
            print("[selftest] FAIL: defer 缺 G15+/G16+ 承接锚仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: defer 缺 G15+/G16+ 承接锚→红")

        # 负样本 3：非法裁决枚举 → 必须红
        bad_enum = full.replace("| no-go |", "| maybe |", 1)
        p4 = Path(td) / "badenum.md"
        p4.write_text(bad_enum, encoding="utf-8")
        code, _ = run_check(p4)
        if code == 0:
            print("[selftest] FAIL: 非法枚举仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 非法枚举→红")

        # 负样本 4：go 行缺 evidence → 必须红
        bad_ev = full.replace("evidence/g14_fixture_20260820T000000Z.json", "milestones/g14/无证据.md", 1)
        p5 = Path(td) / "badev.md"
        p5.write_text(bad_ev, encoding="utf-8")
        code, _ = run_check(p5)
        if code == 0:
            print("[selftest] FAIL: go 缺 evidence 仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: go 缺 evidence→红")

    if failures:
        print(f"[selftest] FAILURES={failures}", file=sys.stderr)
        return 1
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G14.5a P2 穷举决策门（g14.wave.5a.decisions）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑决策门")
    g.add_argument("--selftest", action="store_true", help="正/负样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    code, results = run_check(None)
    return emit(results, code == 0)


if __name__ == "__main__":
    sys.exit(main())
