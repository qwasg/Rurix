#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.1 治理波）
"""G13.1 治理门 — 候选决策表闭集/锚纪律/横向对账（g13.wave.1.candidate_decisions，步骤 234）。

核验 `milestones/g13/G13_CANDIDATE_DECISIONS.md`（2026-08-18 v1.0 落盘）：
冻结 36 行候选闭集全等（§1 G12 defer-to-G13+ 22 行承接 + §2 open RD 7 行映射 +
§3 G13 新增候选 7 行——G13-N1 vendor 超分接入→M-a / G13-N2 自研 TSR device 化→M-b /
G13-N3 UE5 超分双端对拍→M-c / G13-N4 UE Lumen GI 对照→M-d / G13-N5 回归门+漂移监控→M-e /
G13-N6 异己面 no-go / G13-N7 帧生成 FG/MFG defer-to-G14+）、裁决枚举合法
（go/no-go/defer-to-G14+/strategic_override）、零空行（全列非空）、承接锚纪律
（§1 行承接锚 = G12.6 字面 0-byte 转引，含「→」分节与 G13+ 承接源字面；§3 行含
「重判条件 + 兜底」字面）、defer-to-G14+ 裁决行 G14+ 重评窗字面（裁决/最终状态列
承载，转引列不回写）、go 行验收映射锚义务（登记留痕位置含 G13_ACCEPTANCE_MAP）、
no-go 行 RD/契约锚义务、§2 RD 行条目级 status==open；外加三横向机核——
  ① 与 G13_ACCEPTANCE_MAP 5 key（M-a~M-e 全 P0）互斥：候选行 ID 不得命中已 go 门裸 token；
  ② deferred.json history 对账：G13.1 治理门登记恰好 RD-039 +1（M61）/ RD-040 +3
    （M52 G13.4 重评窗登记 + M100-high 锚定 G14 维持 + RD040-nrd G13 决策窗登记）/
    RD-041 +1（G10-N5 兑现窗 + FSR/DirectSR 分项 M-a 承载登记），零新 RD（max=RD-044），
    RD-039/040/041 条目级 status open 0-byte；
  ③ G12_P2_DECISIONS 对账：§1 22 行 ID 全数在 G12_P2_DECISIONS.md 在树
    （defer-to-G13+ 22 行闭集 = G13 法定输入），且 G13 裁决 = G10-N5 go（G13 兑现窗）+
    其余 21 行 defer-to-G14+ 字面承接。
只读文档与 registry，不代绿实现门；no-go/defer 如实保持 open 不写进全绿叙述。

用法：
  py -3 ci/g13_candidate_decisions_check.py --gate g13.wave.1.candidate_decisions
  py -3 ci/g13_candidate_decisions_check.py --selftest
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

ROOT = wel.ROOT
GATE_KEY = "g13.wave.1.candidate_decisions"
NUMERIC_STEP = 234  # 落盘前实测 registry/number_ledger.json CI_step.next_free=234 顺位领取
SUBJECT = "g13_candidate_decisions_check"
WAVE = "G13.1"
DECISIONS = ROOT / "milestones" / "g13" / "G13_CANDIDATE_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g13" / "g13_candidate_decisions_check_evidence_schema.json"
ACCEPTANCE_MAP = ROOT / "milestones" / "g13" / "G13_ACCEPTANCE_MAP.md"
G12_P2 = ROOT / "milestones" / "g12" / "G12_P2_DECISIONS.md"
DEFERRED = DEFERRED_PATH

# 冻结 ID 闭集（36 行）= §1 G12 defer 22 行 + §2 open RD 7 行 + §3 G13 新增 7 行。
SEC1_IDS = [
    "M61",          # G12 defer 承接：RD-039 mesh shader→M109
    "M52",          # G12 defer 承接：RD-040 SER→M108（G13.4 Lumen 化 workload 重评窗登记）
    "M100-high",    # G12 defer 承接：RD-040 ReSTIR 高档（锚定 G14 字面维持）
    "SAFE-GPU",     # G12 defer 承接：Safe GPU Operator Platform（独立期）
    "M127",         # G12 defer 承接：神经变形研究子轨
    "M98-l4",       # G12 defer 承接：M98 L4 Far Field
    "M114-strand",  # G12 defer 承接：毛发 strand 档（锚定 G14）
    "M118-hdr-cal", # G12 defer 承接：HDR 设备标定层
    "M125-adopt3",  # G12 defer 承接：Jolt 5.6 采纳三件
    "G10-N5",       # G12 defer 承接：DLSS/Streamline 方向（**go：G13 兑现窗 → M-a 承载**）
    "G10-N6",       # G12 defer 承接：BistroExterior 缺口
    "G10-N8",       # G12 defer 承接：-renderoffscreen 未测
    "G10-N11",      # G12 defer 承接：M141 采样形态+MRQ 开销口径（锚定 G14；M-c zero_pass_line 联动）
    "G10-N16",      # G12 defer 承接：Rurix GPU 管线 A/B 面未测（锚定 G14；M-d 边界登记）
    "G10-N17",      # G12 defer 承接：M137 scalars.flip 演进位（G13.4 触发评估登记）
    "G11-N3",       # G12 defer 承接：GPU 管线画质差距面未 measured（锚定 G14）
    "G11-N5",       # G12 defer 承接：锁定度量反向激励旁证（G13.5a 触发评估登记）
    "G11-N8",       # G12 defer 承接：太阳穿玻璃高光尾（锚定 G15）
    "G11-N9",       # G12 defer 承接：c1_ue_specular_ibl 实测上界（锚定 G15）
    "G12-N10",      # G12 defer 承接：材质链（锚定 G15）
    "G12-N12",      # G12 defer 承接：UE PT 差距登记表 10 行处置（锚定 G15；只消费不回写）
    "G12-N13",      # G12 defer 承接：M165 间歇非确定性事件（M-e 漂移监控臂承接）
]
SEC2_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044"]
SEC3_IDS = [
    "G13-N1",  # vendor 超分接入（go → M-a）
    "G13-N2",  # 自研 TSR device 化（go → M-b）
    "G13-N3",  # UE5 超分双端对拍（go → M-c）
    "G13-N4",  # UE Lumen GI 对照（go → M-d）
    "G13-N5",  # 回归门 + M165 漂移监控（go → M-e）
    "G13-N6",  # 异己会话 src/ 未提交面（no-go：维持未提交不混入）
    "G13-N7",  # 帧生成 FG/MFG（defer-to-G14+：独立层另判）
]
FROZEN_IDS = SEC1_IDS + SEC2_IDS + SEC3_IDS
ALLOWED = frozenset({"go", "no-go", "defer-to-G14+", "strategic_override"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|待补|—|-)?$", re.I)
ID_RE = re.compile(r"^(M\d|RD-\d|SAFE-GPU|G1[012]-N\d|G13-N\d|M-[a-e])")
# §1 go 行（G13 兑现窗）；其余 §1 行一律 defer-to-G14+。
SEC1_GO_IDS = frozenset({"G10-N5"})
SEC3_GO_IDS = frozenset({"G13-N1", "G13-N2", "G13-N3", "G13-N4", "G13-N5"})
SEC3_NO_GO_IDS = frozenset({"G13-N6"})
SEC3_DEFER_IDS = frozenset({"G13-N7"})
# deferred.json history 对账期望：G13.1 治理门登记恰好 RD-039 +1（M61）/
# RD-040 +3（M52/M100-high/nrd）/ RD-041 +1（G10-N5）。
EXPECTED_DEFER_HISTORY = {"RD-039": ["M61"], "RD-040": ["M52", "M100-high", "nrd"], "RD-041": ["G10-N5"]}
HISTORY_MARKER = "G13.1"
NO_GO_ANCHORS = ("RD-", "deferred", "CONTRACT", "RFC-", "矩阵", "CAPABILITY", "CANDIDATE", "PLAN", "MAP")


def parse_tables(text: str) -> dict[str, list[list[str]]]:
    """解析三个表（§1 header 首格 `ID` / §2 首格 `RD` / §3 首格 `候选 ID`），返回 {节: 数据行单元格列表}。"""
    out: dict[str, list[list[str]]] = {"sec1": [], "sec2": [], "sec3": []}
    block: list[list[str]] = []

    def flush() -> None:
        nonlocal block
        if len(block) >= 2 and all(re.fullmatch(r":?-{2,}:?", c) for c in block[1]):
            header = block[0]
            head = header[0] if header else ""
            # §5 承接锚清单表头首格同为「ID」——按列数与次列字面区分（§1 = 8 列且次列「分项名」）。
            if head == "ID" and len(header) >= 8 and header[1] == "分项名":
                out["sec1"].extend(block[2:])
            elif head == "RD" and len(header) >= 7:
                out["sec2"].extend(block[2:])
            elif head == "候选 ID" and len(header) >= 9 and header[1] == "分项名":
                out["sec3"].extend(block[2:])
        block = []

    for line in text.splitlines():
        s = line.strip()
        if s.startswith("|"):
            block.append([c.strip() for c in s.strip("|").split("|")])
        else:
            flush()
    flush()
    return out


def cell_empty(v: str) -> bool:
    s = (v or "").strip()
    return (not s) or bool(EMPTY_RE.match(s))


def validate(
    tables: dict[str, list[list[str]]],
    map_text: str | None = None,
    deferred_data: dict | None = None,
    g12_p2_text: str | None = None,
) -> list[dict]:
    """41 facts：set_equality_frozen / no_duplicate_ids / row×36 / 三横向机核。"""
    results: list[dict] = []
    sec1, sec2, sec3 = tables["sec1"], tables["sec2"], tables["sec3"]
    ids = [r[0] for r in sec1 + sec2 + sec3 if r]
    set_ok = set(ids) == set(FROZEN_IDS) and len(ids) == len(FROZEN_IDS)
    results.append({
        "id": "set_equality_frozen",
        "status": "PASS" if set_ok else "FAIL",
        "detail": f"got n={len(ids)} unique={len(set(ids))}; expect frozen {len(FROZEN_IDS)}"
        + ("" if set_ok else f"; diff={sorted(set(FROZEN_IDS) ^ set(ids))}"),
    })
    if len(ids) != len(set(ids)):
        results.append({
            "id": "no_duplicate_ids",
            "status": "FAIL",
            "detail": f"duplicates: {[x for x in ids if ids.count(x) > 1]}",
        })
    else:
        results.append({"id": "no_duplicate_ids", "status": "PASS", "detail": "ok"})

    # --- §1 行级机核（8 列：ID/分项名/承接锚字面/裁决/裁决理由/波次/登记留痕/最终状态） ---
    for r in sec1:
        rid = r[0] if r else "?"
        parts: list[str] = []
        if len(r) < 8:
            parts.append(f"列数不足 8（实测 {len(r)}）")
            results.append({"id": f"row_{rid}", "status": "FAIL", "detail": "; ".join(parts)})
            continue
        decision = r[3].replace("**", "").strip()
        base = (
            "no-go" if decision.startswith("no-go")
            else "go" if decision.startswith("go")
            else "defer-to-G14+" if decision.startswith("defer-to-G14+")
            else "strategic_override" if decision.startswith("strategic_override")
            else None
        )
        if base is None:
            parts.append(f"非法裁决 {decision!r}")
        for i, cell in enumerate(r):
            if cell_empty(cell):
                parts.append(f"空单元格 col{i}")
                break
        anchor = r[2]
        if "→" not in anchor:
            parts.append("承接锚字面缺「→」分节（G12.6 0-byte 转引口径）")
        # §1 锚列 = G12.6 承接锚 0-byte 转引（G13+ 字面）；G14+ 重评窗由裁决列
        # defer-to-G14+ 自身与最终状态列承载（转引列不回写）。
        if base == "defer-to-G14+" and "G13+" not in anchor:
            parts.append("defer 缺承接源 G13+ 重评窗字面（G12.6 0-byte 转引口径）")
        if rid in SEC1_GO_IDS:
            if base != "go":
                parts.append(f"{rid} 应为 go（G13 兑现窗），实测 {decision!r}")
            if "G13_ACCEPTANCE_MAP" not in r[6]:
                parts.append("go 行缺验收映射锚（登记留痕位置须含 G13_ACCEPTANCE_MAP）")
        elif base != "defer-to-G14+":
            parts.append(f"{rid} 应为 defer-to-G14+ 承接，实测 {decision!r}")
        results.append({
            "id": f"row_{rid}",
            "status": "PASS" if not parts else "FAIL",
            "detail": "; ".join(parts) if parts else f"{decision}",
        })

    # --- §2 行级机核（7 列：RD/title/status/处置/联动面/裁决理由/留痕位置） ---
    for r in sec2:
        rid = r[0] if r else "?"
        parts = []
        if len(r) < 7:
            parts.append(f"列数不足 7（实测 {len(r)}）")
        else:
            if r[2] != "open":
                parts.append(f"条目级 status 须为 open，实测 {r[2]!r}")
            for i, cell in enumerate(r):
                if cell_empty(cell):
                    parts.append(f"空单元格 col{i}")
                    break
        results.append({
            "id": f"row_{rid}",
            "status": "PASS" if not parts else "FAIL",
            "detail": "; ".join(parts) if parts else "open 维持",
        })

    # --- §3 行级机核（9 列：候选 ID/分项名/来源/裁决/裁决理由/波次/依据/承接锚/登记留痕） ---
    for r in sec3:
        rid = r[0] if r else "?"
        parts = []
        if len(r) < 9:
            parts.append(f"列数不足 9（实测 {len(r)}）")
            results.append({"id": f"row_{rid}", "status": "FAIL", "detail": "; ".join(parts)})
            continue
        decision = r[3].replace("**", "").strip()
        if decision not in ALLOWED:
            parts.append(f"非法裁决 {decision!r}")
        for i, cell in enumerate(r):
            if cell_empty(cell):
                parts.append(f"空单元格 col{i}")
                break
        anchor = r[7]
        if "重判条件" not in anchor or "兜底" not in anchor:
            parts.append("承接锚缺「重判条件/兜底」字面")
        if decision == "defer-to-G14+" and "G14+" not in anchor:
            parts.append("defer 缺 G14+ 重评窗字面")
        if rid in SEC3_GO_IDS:
            if decision != "go":
                parts.append(f"{rid} 应为 go，实测 {decision!r}")
            if "G13_ACCEPTANCE_MAP" not in r[8]:
                parts.append("go 行缺验收映射锚（登记留痕位置须含 G13_ACCEPTANCE_MAP）")
        elif rid in SEC3_NO_GO_IDS:
            if decision != "no-go":
                parts.append(f"{rid} 应为 no-go，实测 {decision!r}")
            if not any(a in r[6] for a in NO_GO_ANCHORS):
                parts.append("no-go 缺 RD/矩阵/契约/计划/MAP 锚")
        elif rid in SEC3_DEFER_IDS and decision != "defer-to-G14+":
            parts.append(f"{rid} 应为 defer-to-G14+，实测 {decision!r}")
        results.append({
            "id": f"row_{rid}",
            "status": "PASS" if not parts else "FAIL",
            "detail": "; ".join(parts) if parts else f"{decision}",
        })

    # --- 横向机核①：与 G13_ACCEPTANCE_MAP 5 key 互斥 ---
    mt = map_text if map_text is not None else (
        ACCEPTANCE_MAP.read_text(encoding="utf-8") if ACCEPTANCE_MAP.is_file() else ""
    )
    gated = {f"M-{m}" for m in re.findall(r"g13\.p0\.m_([a-e])\.", mt)}
    hit = sorted(set(ids) & gated)
    mutex_ok = not hit and len(gated) == 5
    results.append({
        "id": "acceptance_map_mutex",
        "status": "PASS" if mutex_ok else "FAIL",
        "detail": f"MAP 实解 P0={len(gated)}（expect 5）；候选表命中已 go 门裸 token: {hit or '无'}",
    })

    # --- 横向机核②：deferred.json history 对账（G13.1 治理门登记新增条数） ---
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
    for rd in ("RD-039", "RD-040", "RD-041"):
        if status_map.get(rd) != "open":
            reconcile_ok = False
            rec_parts.append(f"{rd} status 非 open")
    rec_parts.append(f"{HISTORY_MARKER} history: {sorted((r, len(g)) for r, g in holders.items())}")
    results.append({
        "id": "deferred_history_reconcile",
        "status": "PASS" if reconcile_ok else "FAIL",
        "detail": "; ".join(rec_parts),
    })

    # --- 横向机核③：G12_P2_DECISIONS 对账（§1 22 行 = G12 defer-to-G13+ 闭集法定输入） ---
    pt = g12_p2_text if g12_p2_text is not None else (
        G12_P2.read_text(encoding="utf-8") if G12_P2.is_file() else ""
    )
    cand_ok = True
    cand_parts: list[str] = []
    if not pt:
        cand_ok = False
        cand_parts.append("G12_P2_DECISIONS.md 未落盘或不可读")
    else:
        for rid in SEC1_IDS:
            if rid not in pt:
                cand_ok = False
                cand_parts.append(f"G12_P2 缺承接源行 {rid}")
    cand_parts.append(f"承接源行对账 n={sum(1 for rid in SEC1_IDS if rid in pt)}/22")
    results.append({
        "id": "g12_p2_decisions_reconcile",
        "status": "PASS" if cand_ok else "FAIL",
        "detail": "; ".join(cand_parts),
    })
    return results


def run_check(
    path: Path | None = None,
    map_text: str | None = None,
    deferred_data: dict | None = None,
    g12_p2_text: str | None = None,
) -> tuple[int, list[dict]]:
    p = path or DECISIONS
    if not p.is_file():
        # 诚实红：表未落盘不是绿
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}（G13.1 候选决策表未落盘；诚实红，不假绿）"}]
    results = validate(
        parse_tables(p.read_text(encoding="utf-8")),
        map_text=map_text,
        deferred_data=deferred_data,
        g12_p2_text=g12_p2_text,
    )
    ok = all(x["status"] == "PASS" for x in results)
    return (0 if ok else 1), results


def emit(results: list[dict], overall_ok: bool) -> int:
    for r in results:
        print(f"  FACT  {r['status']:4}  {r['id']}  ({r.get('detail','')})")
    if not SCHEMA_PATH.is_file():
        print(f"[g13_candidate_decisions] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G13_CONTRACT G-G13-2/§6/§7;G13_CANDIDATE_DECISIONS.md v1.0;G13_ACCEPTANCE_MAP §1/§2;G12_P2_DECISIONS.md v1.0（22 行承接锚法定输入）;registry/deferred.json（G13.1 治理门登记行）",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G13.1 治理门——候选决策表 36 行闭集（go 6〔§1 G10-N5 G13 兑现窗 + §3 G13-N1~N5〕+ no-go 1〔G13-N6 异己面〕+ defer-to-G14+ 22〔§1 21 + §3 G13-N7 FG/MFG〕+ strategic_override 0；§2 RD 映射 7 行维持 open 不重复计入）：裁决枚举/零空行/承接锚纪律 + MAP 5 key 互斥 + deferred history 对账（RD-039 +1〔M61〕/RD-040 +3〔M52/M100-high/nrd〕/RD-041 +1〔G10-N5〕，零新 RD）+ G12_P2 承接源行对账（n=22/22）；no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=overall_ok,
    )
    return code


# ---------------------------------------------------------------------------
# selftest 合成夹具。
# ---------------------------------------------------------------------------

def _synth_sec1(rid: str) -> str:
    if rid in SEC1_GO_IDS:
        decision = "go（G13 兑现窗 → M-a 承载）"
        reg = "本表 §1 行 + G13_ACCEPTANCE_MAP §1 M-a 行"
    else:
        decision = "defer-to-G14+"
        reg = "本表 §1 行（不新设 RD）"
    return (
        f"| {rid} | 分项 | 「G13+ 重评窗触发条件齐备 → 兜底面维持（字面 0-byte）」 | {decision} | 理由 | —（非 go） | {reg} | open-defer |\n"
    )


def _synth_sec2(rid: str) -> str:
    return f"| {rid} | title | open | 维持 open | 无 | 理由 | 本表 §2 行 |\n"


def _synth_sec3(rid: str) -> str:
    if rid in SEC3_GO_IDS:
        decision = "go"
        anchor = "重判条件 = G14+ 若口径面再发现过严/过松面时按只追加程序重判；兜底 = 现机核维持，门绿 0-byte"
        ref = "用户目标（2026-08-15 会话留痕）"
        reg = "本表 §3 行 + G13_ACCEPTANCE_MAP §1 行"
    elif rid in SEC3_NO_GO_IDS:
        decision = "no-go"
        anchor = "重判条件 = G14+ 触发条件齐备时按只追加程序重判；兜底 = 既有面维持"
        ref = "G13_CONTRACT §7 立项裁决 1"
        reg = "本表 §3 行（不新设 RD）"
    else:
        decision = "defer-to-G14+"
        anchor = "重判条件 = G14+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持"
        ref = "registry/deferred.json RD-041 / rfcs/0016"
        reg = "本表 §3 行 + §2 RD-041 行（不新设 RD）"
    return f"| {rid} | 分项 | 来源 | {decision} | 理由 | 波次 | {ref} | {anchor} | {reg} |\n"


def _full_fixture() -> str:
    sec1_head = "| ID | 分项名 | 承接锚字面 | G13 裁决 | 裁决理由 | 波次/联动面 | 登记留痕位置 | 最终状态 |\n|---|---|---|---|---|---|---|---|\n"
    sec2_head = "| RD | title | 条目级 status | G13 处置 | G13 联动面 | 裁决理由 | 留痕位置 |\n|---|---|---|---|---|---|---|\n"
    sec3_head = "| 候选 ID | 分项名 | 来源 | G13 裁决 | 裁决理由 | 波次归属 | 依据/证据路径 | 承接锚 / 兜底 | 登记留痕位置 |\n|---|---|---|---|---|---|---|---|---|\n"
    return (
        "## 1. §1 表\n\n" + sec1_head + "".join(_synth_sec1(i) for i in SEC1_IDS)
        + "\n## 2. §2 表\n\n" + sec2_head + "".join(_synth_sec2(i) for i in SEC2_IDS)
        + "\n## 3. §3 表\n\n" + sec3_head + "".join(_synth_sec3(i) for i in SEC3_IDS)
    )


def run_selftest() -> int:
    failures = 0
    full = _full_fixture()

    # 正样本 1：真表（已落盘）必须绿
    code, results = run_check()
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
            print("[selftest] PASS: 真表 36 行绿")

    with tempfile.TemporaryDirectory(prefix="g13_cand_selftest_") as td:
        # 正样本 2：合成全表（真树 MAP/deferred/G12_P2 对账）必须绿
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

        def _red(name: str, content: str, expect_fact: str) -> None:
            nonlocal failures
            q = Path(td) / "bad.md"
            q.write_text(content, encoding="utf-8")
            c, rs = run_check(q)
            hit = [r for r in rs if r["id"] == expect_fact and r["status"] == "FAIL"]
            if c != 0 and hit:
                print(f"  RED ok   — {name}（{hit[0]['detail'][:80]}）")
            else:
                print(f"  RED MISS — {name}：负样本未被判红于 {expect_fact}")
                failures += 1

        _red(
            "缺行（删 M127）→ set_equality 红",
            "\n".join(ln for ln in full.splitlines() if not ln.strip().startswith("| M127 |")) + "\n",
            "set_equality_frozen",
        )
        _red(
            "§3 defer 行承接锚缺 G14+ → 红",
            full.replace(
                "重判条件 = G14+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持",
                "重判条件 = 触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持",
            ),
            "row_G13-N7",
        )
        _red(
            "非法裁决枚举（§3 go→maybe）→ 红",
            full.replace("| G13-N1 | 分项 | 来源 | go |", "| G13-N1 | 分项 | 来源 | maybe |", 1),
            "row_G13-N1",
        )
        _red(
            "空单元格（§1 裁决理由置空）→ 红",
            full.replace("| M61 | 分项 |", "| M61 |  |", 1),
            "row_M61",
        )
        _red(
            "互斥违例（已 go 门裸 token M-a 入表）→ 红",
            full.replace("| M61 | 分项 |", "| M-a | 分项 |", 1),
            "acceptance_map_mutex",
        )
        _red(
            "§2 RD 行 status 非 open → 红",
            full.replace("| RD-034 | title | open |", "| RD-034 | title | closed |", 1),
            "row_RD-034",
        )

        # deferred history 缺登记 → 红
        real = wel.load_json(DEFERRED)
        stripped = {
            **real,
            "entries": [
                {**e, "history": [h for h in e.get("history", []) if HISTORY_MARKER not in (h.get("event") or "")]}
                for e in real.get("entries", [])
            ],
        }
        c, rs = run_check(p, deferred_data=stripped)
        hit = [r for r in rs if r["id"] == "deferred_history_reconcile" and r["status"] == "FAIL"]
        if c != 0 and hit:
            print(f"  RED ok   — deferred history 缺登记（{hit[0]['detail'][:80]}）")
        else:
            print("  RED MISS — deferred history 缺登记未被判红")
            failures += 1

        # G12_P2 对账失配（注入缺 G12-N13 的 G12_P2 文本）→ 红
        real_p2 = G12_P2.read_text(encoding="utf-8") if G12_P2.is_file() else ""
        c, rs = run_check(p, g12_p2_text=real_p2.replace("G12-N13", "G12-N99"))
        hit = [r for r in rs if r["id"] == "g12_p2_decisions_reconcile" and r["status"] == "FAIL"]
        if c != 0 and hit:
            print(f"  RED ok   — G12_P2 对账失配（{hit[0]['detail'][:80]}）")
        else:
            print("  RED MISS — G12_P2 对账失配未被判红")
            failures += 1

    if failures:
        print(f"[g13_candidate_decisions] SELFTEST FAIL ({failures})")
        return 1
    print("[g13_candidate_decisions] SELFTEST PASS (8 RED + 真表/合成双臂 GREEN)")
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
