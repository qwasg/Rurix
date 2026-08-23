#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.6a P2 穷举 + M-e 回归门 + soak 波）
"""G15.6a P2/留档/未触发分项穷举决策门 g15.wave.6a.decisions（G15_CONTRACT G-G15-7；
同构 ci/g14_p2_decisions_check.py〔G14.5a 42 行闭集〕范式）。

核验 `milestones/g15/G15_P2_DECISIONS.md`（2026-08-23 v1.0 落盘）：
冻结 40 行候选闭集全等（§1 G15 候选表 35 行终态裁决——G15_CANDIDATE_DECISIONS
v1.0 闭集 G15 期窗逐行核验：go 行兑现完结核验引 evidence 真跑件转 closed-go 留痕
〔G15.1 §5.5 范式〕，14 行 defer-to-G16+ 维持字面承接锚 0-byte + §2 期内新增 5 行
——G15-MA-F1 closed-caliber-registered 终态 / G15-MC-F1 UE 参照臂黑帧
open-defer-G16+ / G15-MD-F1 DLSS t100 格 open-defer-G16+ 承接锚字面 + G15plus
双延续波留痕行）、裁决枚举合法（go/closed-go/no-go/defer-to-G16+/strategic_override）、
零空行（全列非空）、承接锚「重判条件 + 兜底」字面、defer 行必含 G16+ 重评窗、
closed-go/go 行 evidence 义务；外加四横向机核——
  ① 与 G15_ACCEPTANCE_MAP §1 五 P0（M-a~M-e）互斥：P2 行 ID 不得命中字母行裸 token；
  ② deferred.json 对账：RD-034~RD-045 八条目级 status 全 open 0-byte、零新 RD
    （max=RD-045）、本波零 G15.6a 标记者 history 追加、vs G15.0 base 只追加机核
    （条目四字段 0-byte + history 前缀只追加——RD-045 零检出维持 open 不关闭字面）；
  ③ G15.1 候选决策表对账：§1 35 行 ID 在 G15_CANDIDATE_DECISIONS 在册 + 裁决迁移
    合法（candidate go → P2 closed-go 兑现转留痕；closed-go → closed-go 留痕维持；
    defer-to-G16+ → defer-to-G16+ 维持字面）；
  ④ 差距登记表对账：g15_quality_gap_disposition.json 20 行（gap_id 闭集全等 +
    wave_start 链锚）+ g15_gap_fix_closure_registry.json 20 行三态 tally 重算
    （0/4/16）+ open-defer 行承接锚字面 + G15-MA-F1 定论在案 +
    g14_fps_gap_registry.json 1 行（gap_id 51a150cb4523e8b6）== 最新 G14 M-d
    evidence unmet_count（诚实红登记面一致）+ g13 upscale 8 行 / lumen 2 行 /
    g12 pt 10 行终态 0-byte（git porcelain 空）+ G15-MC-F1/G15-MD-F1 契约登记字面。
只读文档与 registry，不代绿实现门；no-go/defer 如实保持 open 不写进全绿叙述；
双未达标定盘面（商用收口 0/18 + 性能 17/18 单格环境事件面）如实登记不冒充。

materialize：numeric_step=277（落盘前实测 CI_step.next_free=277 顺位领取）。

用法：
  py -3 ci/g15_p2_decisions_check.py --gate g15.wave.6a.decisions
  py -3 ci/g15_p2_decisions_check.py --selftest
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
GATE_KEY = "g15.wave.6a.decisions"
NUMERIC_STEP = 277  # 落盘前实测 registry/number_ledger.json CI_step.next_free=277 顺位领取
SUBJECT = "g15_p2_decisions"
WAVE = "G15.6a"
DECISIONS = ROOT / "milestones" / "g15" / "G15_P2_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_p2_decisions_evidence_schema.json"
ACCEPTANCE_MAP = ROOT / "milestones" / "g15" / "G15_ACCEPTANCE_MAP.md"
CANDIDATE = ROOT / "milestones" / "g15" / "G15_CANDIDATE_DECISIONS.md"
CONTRACT = ROOT / "milestones" / "g15" / "G15_CONTRACT.md"
DEFERRED = DEFERRED_PATH
DISPOSITION = ROOT / "milestones" / "g15" / "g15_quality_gap_disposition.json"
CLOSURE_REGISTRY = ROOT / "milestones" / "g15" / "g15_gap_fix_closure_registry.json"
REG_G14_FPS = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"
REG_G13_UPSCALE = ROOT / "milestones" / "g13" / "g13_ue_upscale_gap_registry.json"
REG_G13_LUMEN = ROOT / "milestones" / "g13" / "g13_ue_lumen_gap_registry.json"
REG_G12 = ROOT / "milestones" / "g12" / "g12_ue_pt_gap_registry.json"
MD_PREFIX = "g14_m_d_dual_end_fps_parity"
G15_0_REF = "f061487efaf7816684de18a6ef86554e5c392a75"  # G15.0 不可变 ref（G14 close-out flip commit）

# 冻结 ID 闭集（40 行）= §1 G15 候选表 35 行 + §2 期内新增 5 行。
SEC1_IDS = [
    "M61", "M52", "M100-high", "SAFE-GPU", "M127", "M98-l4",
    "M114-strand", "M118-hdr-cal", "M125-adopt3", "G10-N6",
    "G10-N8", "G10-N17", "G11-N3", "G11-N5", "G11-N8", "G11-N9",
    "G12-N10", "G12-N12", "G12-N13", "G13-N7", "G13-N8", "G13-N9",
    "G14-N8", "G14-N9", "G14-N10", "G14-N11", "G14-N12", "G14-N13",
    "G14-N14",
    "G15-N1", "G15-N2", "G15-N3", "G15-N4", "G15-N5", "G15-N6",
]
SEC2_IDS = ["G15-MA-F1", "G15-MC-F1", "G15-MD-F1", "G15PLUS-W1", "G15PLUS-W2"]
FROZEN_IDS = SEC1_IDS + SEC2_IDS
ALLOWED = frozenset({"go", "closed-go", "no-go", "defer-to-G16+", "strategic_override"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|待补|—|-)?$", re.I)
ID_RE = re.compile(r"^(M\d|RD-\d|SAFE-GPU|G1[012345]-N\d|G15-(MA|MC|MD)-F1|G15PLUS-W\d)")
HEADERS = [
    "ID", "分项名", "来源波次", "原触发条件字面", "裁决",
    "裁决理由", "依据/证据路径", "承接锚", "登记留痕位置", "最终状态",
]
# 三态分布闭集（G15.6a 定盘字面）。
CLOSED_GO_IDS = frozenset({
    "G11-N3", "G11-N8", "G11-N9", "G12-N10", "G12-N12", "G12-N13",
    "G13-N8", "G13-N9",
    "G14-N8", "G14-N9", "G14-N10", "G14-N11", "G14-N12", "G14-N13", "G14-N14",
    "G15-N1", "G15-N2", "G15-N3", "G15-N4", "G15-N5", "G15-N6",
    "G15-MA-F1", "G15PLUS-W1", "G15PLUS-W2",
})
DEFER_IDS = frozenset({
    "M61", "M52", "M100-high", "SAFE-GPU", "M127", "M98-l4",
    "M114-strand", "M118-hdr-cal", "M125-adopt3", "G10-N6",
    "G10-N8", "G10-N17", "G11-N5", "G13-N7",
    "G15-MC-F1", "G15-MD-F1",
})
NO_GO_IDS: frozenset = frozenset()
# G15.1 候选表 §1/§3 行裁决面（对账字面：go 14 行 → P2 closed-go 兑现转留痕；
# closed-go 7 行 → closed-go 留痕维持；defer-to-G16+ 14 行 → defer-to-G16+ 维持）。
CANDIDATE_GO_IDS = frozenset({
    "G11-N3", "G11-N8", "G11-N9", "G12-N10", "G12-N12", "G12-N13",
    "G13-N8", "G13-N9",
    "G15-N1", "G15-N2", "G15-N3", "G15-N4", "G15-N5", "G15-N6",
})
CANDIDATE_CLOSED_GO_IDS = frozenset({
    "G14-N8", "G14-N9", "G14-N10", "G14-N11", "G14-N12", "G14-N13", "G14-N14",
})
CANDIDATE_DEFER_IDS = frozenset({
    "M61", "M52", "M100-high", "SAFE-GPU", "M127", "M98-l4",
    "M114-strand", "M118-hdr-cal", "M125-adopt3", "G10-N6",
    "G10-N8", "G10-N17", "G11-N5", "G13-N7",
})
# MAP §1 五 P0 字母行裸 token（互斥面）。
MAP_LETTER_TOKENS = frozenset({"M-a", "M-b", "M-c", "M-d", "M-e"})
RD_OPEN_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044", "RD-045"]
FPS_GAP_ID = "51a150cb4523e8b6"
EXPECTED_TALLY = {"closed_resolved": 0, "closed_caliber_registered": 4, "open_defer_g16_plus": 16}
DISPOSITION_WAVE_START = "20260823T084242Z"


def parse_tables(text: str) -> list[dict[str, str]]:
    """解析 §1/§2 两张决策表（| ID | … 十列头；§3 RD 表与 §4 锚清单头集不全等即跳）。"""
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


def _decision_of(raw: str) -> str:
    return (raw or "").strip().strip("*").strip().split("（")[0].split("(")[0].strip().strip("*").strip()


def _git_porcelain(rel: str) -> str:
    r = subprocess.run(
        ["git", "status", "--porcelain", "--", rel],
        cwd=ROOT, capture_output=True, text=True,
    )
    return (r.stdout or "").strip()


def _git_show(ref: str, rel: str) -> str | None:
    r = subprocess.run(
        ["git", "show", f"{ref}:{rel}"], cwd=ROOT, capture_output=True, text=True,
    )
    return r.stdout if r.returncode == 0 else None


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
        decision = _decision_of(decision_full)
        row_ok = True
        detail_parts: list[str] = []
        if decision not in ALLOWED:
            row_ok = False
            detail_parts.append(f"非法裁决 {decision_full!r}")
        elif decision == "closed-go" and rid not in CLOSED_GO_IDS:
            row_ok = False
            detail_parts.append(f"closed-go 行不在兑现闭集: {rid}")
        elif decision == "defer-to-G16+" and rid not in DEFER_IDS:
            row_ok = False
            detail_parts.append(f"defer 行不在闭集: {rid}")
        elif decision == "no-go" and rid not in NO_GO_IDS:
            row_ok = False
            detail_parts.append(f"no-go 行不在闭集: {rid}")
        # 零空行：除 ID 外九列全必填；承接锚全行必填，defer 行再加 G16+ 字面
        for k in HEADERS[1:]:
            if cell_empty(r.get(k, "")):
                row_ok = False
                detail_parts.append(f"空单元格 {k}")
        anchor = r.get("承接锚") or ""
        if "重判条件" not in anchor or "兜底" not in anchor:
            row_ok = False
            detail_parts.append("承接锚缺「重判条件/兜底」字面")
        if decision == "defer-to-G16+" and "G16+" not in anchor and "G16+" not in (r.get("最终状态") or ""):
            row_ok = False
            detail_parts.append("defer 缺 G16+ 重评窗字面")
        if decision in ("go", "closed-go") and "evidence/" not in (r.get("依据/证据路径") or ""):
            row_ok = False
            detail_parts.append(f"{decision} 缺 evidence 路径")
        results.append(
            {
                "id": f"row_{rid}",
                "status": "PASS" if row_ok else "FAIL",
                "detail": "; ".join(detail_parts) if detail_parts else decision_full[:80],
            }
        )

    # 横向机核①：与 G15_ACCEPTANCE_MAP §1 五 P0（M-a~M-e）互斥
    mt = map_text if map_text is not None else (
        ACCEPTANCE_MAP.read_text(encoding="utf-8") if ACCEPTANCE_MAP.is_file() else ""
    )
    p0_rows = re.findall(r"g15\.p0\.m_[a-e]\.", mt)
    hit = sorted(set(ids) & MAP_LETTER_TOKENS)
    mutex_ok = not hit and len(set(p0_rows)) == 5
    results.append(
        {
            "id": "acceptance_map_mutex",
            "status": "PASS" if mutex_ok else "FAIL",
            "detail": f"MAP §1 P0 行={len(set(p0_rows))}（expect 5）；P2 表命中字母行裸 token: {hit or '无'}",
        }
    )

    # 横向机核②：deferred.json 对账（八条 open 0-byte + 零新 RD + 本波零追加 + vs G15.0 base 只追加）
    dd = deferred_data if deferred_data is not None else (
        wel.load_json(DEFERRED) if DEFERRED.is_file() else {"entries": []}
    )
    entries = dd.get("entries") or []
    reconcile_ok = True
    rec_parts: list[str] = []
    status_map = {e.get("id"): e.get("status") for e in entries}
    for rd in RD_OPEN_IDS:
        if status_map.get(rd) != "open":
            reconcile_ok = False
            rec_parts.append(f"{rd} status={status_map.get(rd)!r} ≠ open")
    rd_nums = [int(m.group(1)) for e in entries for m in [re.match(r"RD-(\d+)$", e.get("id") or "")] if m]
    if not rd_nums or max(rd_nums) != 45:
        reconcile_ok = False
        rec_parts.append(f"RD max={max(rd_nums) if rd_nums else None} expect 45（零新 RD）")
    wave_marked = [
        e.get("id")
        for e in entries
        if any("G15.6a" in (h.get("event") or "") for h in e.get("history", []))
    ]
    if wave_marked:
        reconcile_ok = False
        rec_parts.append(f"本波非预期 history 追加: {wave_marked}")
    base_text = _git_show(G15_0_REF, "registry/deferred.json")
    if base_text is None:
        reconcile_ok = False
        rec_parts.append("G15.0 base deferred.json 不可取得")
    else:
        try:
            import json as _json

            base_doc = _json.loads(base_text)
            base_entries = {e.get("id"): e for e in base_doc.get("entries", [])}
            cur_entries = {e.get("id"): e for e in entries}
            removed = sorted(set(base_entries) - set(cur_entries))
            if removed:
                reconcile_ok = False
                rec_parts.append(f"deferred 条目被删除: {removed}")
            for rid, be in base_entries.items():
                ce = cur_entries.get(rid)
                if ce is None:
                    continue
                for f in ("title", "reason", "backfill_condition", "status", "owner_milestone"):
                    if ce.get(f) != be.get(f):
                        reconcile_ok = False
                        rec_parts.append(f"{rid} 字段 {f} 被改写（静默改判/0-byte 违例）")
                bh = be.get("history", [])
                ch = ce.get("history", [])
                if len(ch) < len(bh) or ch[: len(bh)] != bh:
                    reconcile_ok = False
                    rec_parts.append(f"{rid} history 非只追加")
        except ValueError:
            reconcile_ok = False
            rec_parts.append("G15.0 base deferred.json 不可解析")
    if reconcile_ok:
        rec_parts.append("RD-034~045 八条 open 0-byte + 零新 RD（max=RD-045）+ 本波零追加 + vs G15.0 base 只追加（RD-045 零检出维持 open 不关闭字面）")
    results.append(
        {
            "id": "deferred_history_reconcile",
            "status": "PASS" if reconcile_ok else "FAIL",
            "detail": "; ".join(rec_parts),
        }
    )

    # 横向机核③：G15.1 候选决策表对账（§1 35 行 ID 在册 + 裁决迁移合法）
    ct = candidate_text if candidate_text is not None else (
        CANDIDATE.read_text(encoding="utf-8") if CANDIDATE.is_file() else ""
    )
    p2_map = {r.get("ID", ""): r for r in rows}
    cand_ok = True
    cand_parts: list[str] = []
    for rid in SEC1_IDS:
        if rid not in ct:
            cand_ok = False
            cand_parts.append(f"CANDIDATE 缺行 {rid}")
            continue
        pr = p2_map.get(rid)
        if pr is None:
            cand_ok = False
            cand_parts.append(f"P2 表缺承接行 {rid}")
            continue
        verdict = _decision_of(pr.get("裁决") or "")
        if rid in CANDIDATE_GO_IDS:
            want = "closed-go"
        elif rid in CANDIDATE_CLOSED_GO_IDS:
            want = "closed-go"
        else:
            want = "defer-to-G16+"
        if verdict != want:
            cand_ok = False
            cand_parts.append(f"{rid} P2 裁决={verdict!r} ≠ 迁移合法值 {want!r}")
    if not ct:
        cand_ok = False
        cand_parts.append("G15_CANDIDATE_DECISIONS.md 未落盘或不可读")
    cand_parts.append(
        f"承接行对账 n={sum(1 for rid in SEC1_IDS if rid in ct)}/35"
        "（candidate go→closed-go 兑现转留痕 / closed-go→closed-go 维持 / defer→defer 维持字面）"
    )
    results.append(
        {
            "id": "candidate_decisions_reconcile",
            "status": "PASS" if cand_ok else "FAIL",
            "detail": "; ".join(cand_parts),
        }
    )

    # 横向机核④：差距登记表对账（G15 处置/闭环双表 + g14 帧率表 1 行 == 最新 G14 M-d
    # unmet_count + g13/g12 三表终态 0-byte + findings 契约登记字面）。
    reg_ok = True
    reg_parts: list[str] = []
    if not DISPOSITION.is_file():
        reg_ok = False
        reg_parts.append("g15_quality_gap_disposition 缺失")
        disp_ids: set = set()
    else:
        disp = wel.load_json(DISPOSITION)
        d_items = disp.get("items") or []
        disp_ids = {it.get("gap_id") for it in d_items}
        if len(d_items) != 20 or disp.get("wave_start") != DISPOSITION_WAVE_START:
            reg_ok = False
            reg_parts.append(
                f"disposition 行数={len(d_items)}（expect 20）wave_start={disp.get('wave_start')!r}"
            )
    if not CLOSURE_REGISTRY.is_file():
        reg_ok = False
        reg_parts.append("g15_gap_fix_closure_registry 缺失")
    else:
        clo = wel.load_json(CLOSURE_REGISTRY)
        c_items = clo.get("items") or []
        clo_ids = {it.get("gap_id") for it in c_items}
        tally = {
            "closed_resolved": sum(1 for it in c_items if it.get("final_disposition") == "closed-resolved"),
            "closed_caliber_registered": sum(
                1 for it in c_items if it.get("final_disposition") == "closed-caliber-registered"
            ),
            "open_defer_g16_plus": sum(
                1 for it in c_items if it.get("final_disposition") == "open-defer-G16+"
            ),
        }
        summary = clo.get("summary") or {}
        stored_tally = {
            "closed_resolved": summary.get("closed_resolved"),
            "closed_caliber_registered": summary.get("closed_caliber_registered"),
            "open_defer_g16_plus": summary.get("open_defer_g16_plus"),
        }
        anchor_bad = [
            it.get("gap_id")
            for it in c_items
            if it.get("final_disposition") == "open-defer-G16+"
            and ("重判条件 = " not in (it.get("anchor") or "") or "；兜底 = " not in (it.get("anchor") or ""))
        ]
        findings = clo.get("findings_adjudication") or []
        ma_f1 = next((f for f in findings if f.get("id") == "G15-MA-F1"), None)
        if (
            len(c_items) != 20
            or clo_ids != disp_ids
            or tally != EXPECTED_TALLY
            or stored_tally != EXPECTED_TALLY
            or anchor_bad
            or ma_f1 is None
            or ma_f1.get("verdict") != "closed-caliber-registered"
        ):
            reg_ok = False
            reg_parts.append(
                f"closure 行数={len(c_items)} tally={tally} stored={stored_tally}"
                f" gap_id 闭集全等={clo_ids == disp_ids} anchor_bad={anchor_bad[:2]}"
                f" G15-MA-F1 verdict={(ma_f1 or {}).get('verdict')!r}"
            )
        else:
            reg_parts.append("G15 处置/闭环双表 20 行 gap_id 闭集全等 + 三态 tally 0/4/16 重算一致 + open-defer 锚字面 + G15-MA-F1 定论在案")
    if not REG_G14_FPS.is_file():
        reg_ok = False
        reg_parts.append("g14_fps_gap_registry 缺失")
    else:
        g14_doc = wel.load_json(REG_G14_FPS)
        g14_items = g14_doc.get("items") or []
        md_path = wel.load_latest_evidence(MD_PREFIX)
        md_doc = wel.load_json(md_path) if md_path else {}
        unmet = ((md_doc.get("parity") or {}).get("unmet_count"))
        ids14 = {it.get("gap_id") for it in g14_items}
        if not (len(g14_items) == 1 and ids14 == {FPS_GAP_ID} and unmet == 1):
            reg_ok = False
            reg_parts.append(
                f"g14 帧率表行数={len(g14_items)} ids={sorted(ids14)} unmet={unmet}"
                f"（合格面 = 1 行 gap_id {FPS_GAP_ID} == 最新 G14 M-d unmet_count=1，{md_path.name if md_path else '缺件'}）"
            )
        else:
            reg_parts.append(f"g14 帧率表 1 行（{FPS_GAP_ID}）== 最新 G14 M-d unmet_count=1（诚实红登记面一致）")
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
    contract_text = CONTRACT.read_text(encoding="utf-8") if CONTRACT.is_file() else ""
    for token in ("G15-MC-F1", "G15-MD-F1"):
        if token not in contract_text:
            reg_ok = False
            reg_parts.append(f"G15_CONTRACT 缺 {token} 登记字面")
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
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}（G15.6a 决策表未落盘；诚实红，不假绿）"}]
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
        print(f"[g15_p2_decisions] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G15_CONTRACT G-G15-7;G15_P2_DECISIONS.md v1.0;G15_CANDIDATE_DECISIONS v1.0;G15_ACCEPTANCE_MAP §1;registry/deferred.json（RD-034~045 八条 open 0-byte）;g15_quality_gap_disposition/g15_gap_fix_closure_registry（20 行双表）;g14_fps_gap_registry（gap_id 51a150cb4523e8b6 门产登记行）;g13/g12 三表终态只消费不回写;G15_CONTRACT §8.1~§8.7",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G15.6a P2/留档/未触发分项穷举决策（40 行闭集：closed-go 24 + defer-to-G16+ 16，go/no-go/strategic_override 0；维持 open 8 行 RD 映射不重复计入）；defer 必有承接锚（重判条件+兜底+G16+ 重评窗）；与 MAP §1 五 P0 互斥；deferred.json 对账（八条 open 0-byte、零新 RD max=RD-045、本波零追加、vs G15.0 base 只追加——RD-045 零检出维持 open 不关闭字面）；G15.1 候选决策表 35 行承接对账（go→closed-go 兑现转留痕 G15.1 §5.5 范式）；G15 处置/闭环双表 20 行三态 tally 0/4/16 重算 + g14 帧率表 1 行 == 最新 G14 M-d unmet_count（诚实红登记面一致）+ g13/g12 三表终态 0-byte + G15-MA-F1/G15-MC-F1/G15-MD-F1 登记字面在案；双未达标定盘面（商用收口 0/18 + 性能 17/18 单格环境事件面）如实登记不冒充 + G16+ 承接锚三面齐备；no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=True,
    )
    return code


def _synth_row(rid: str) -> str:
    if rid in CLOSED_GO_IDS:
        decision = "closed-go"
        anchor = "重判条件 = 已兑现完结无重判面（异动时按只追加程序新立分项）；兜底 = 既有面维持，门绿 0-byte"
        ref = "evidence/g15_fixture_20260823T000000Z.json"
    elif rid in NO_GO_IDS:
        decision = "no-go"
        anchor = "重判条件 = G16+ 所属会话提交并立项评审后按只追加程序重判；兜底 = 既有面维持"
        ref = "registry/deferred.json RD-041 / G15_CONTRACT / G15_CANDIDATE_DECISIONS"
    else:
        decision = "defer-to-G16+"
        anchor = "重判条件 = G16+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持"
        ref = "registry/deferred.json RD-040 / G15_CANDIDATE_DECISIONS §1"
    return f"| {rid} | 分项 | G15.1 | 触发条件字面 | {decision} | 理由 | {ref} | {anchor} | 留痕位置 | open |\n"


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
            print("[selftest] PASS: 真表 40 行绿")

    with tempfile.TemporaryDirectory(prefix="g15_p2_selftest_") as td:
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
        lines = [ln for ln in full.splitlines() if not ln.strip().startswith("| G15-MD-F1 |")]
        p2 = Path(td) / "bad.md"
        p2.write_text("\n".join(lines) + "\n", encoding="utf-8")
        code, _ = run_check(p2)
        if code == 0:
            print("[selftest] FAIL: 缺行仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 缺行→红")

        # 负样本 2：defer 行承接锚缺 G16+ → 必须红
        bad_defer = full.replace(
            "重判条件 = G16+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持",
            "重判条件 = 触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持",
        )
        p3 = Path(td) / "baddefer.md"
        p3.write_text(bad_defer, encoding="utf-8")
        code, _ = run_check(p3)
        if code == 0:
            print("[selftest] FAIL: defer 缺 G16+ 承接锚仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: defer 缺 G16+ 承接锚→红")

        # 负样本 3：非法裁决枚举 → 必须红
        bad_enum = full.replace("| defer-to-G16+ |", "| maybe |", 1)
        p4 = Path(td) / "badenum.md"
        p4.write_text(bad_enum, encoding="utf-8")
        code, _ = run_check(p4)
        if code == 0:
            print("[selftest] FAIL: 非法枚举仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 非法枚举→红")

        # 负样本 4：closed-go 行缺 evidence → 必须红
        bad_ev = full.replace("evidence/g15_fixture_20260823T000000Z.json", "milestones/g15/无证据.md", 1)
        p5 = Path(td) / "badev.md"
        p5.write_text(bad_ev, encoding="utf-8")
        code, _ = run_check(p5)
        if code == 0:
            print("[selftest] FAIL: closed-go 缺 evidence 仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: closed-go 缺 evidence→红")

    if failures:
        print(f"[selftest] FAILURES={failures}", file=sys.stderr)
        return 1
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G15.6a P2 穷举决策门（g15.wave.6a.decisions）")
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
