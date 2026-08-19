#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.1 治理波）
"""G14.1 治理门 — 候选决策表闭集/锚纪律/横向对账（g14.wave.1.candidate_decisions，步骤 248）。

G14_CONTRACT G-G14-1（G14.1 完成门 D-G14-2 候选决策表面）治理三门之一；G-G14-3 实现互锁
（ci/g14_interlock_check.py）事实门②消费本表 31 行零空行面、事实门④消费本门独立 PASS
机器事实。

核验 `milestones/g14/G14_CANDIDATE_DECISIONS.md`（2026-08-19 v1.0 落盘）：
冻结 31 行候选闭集全等（§1 G13 defer-to-G14+ 24 行承接——含 G10-N11/G10-N16 两行
G14 兑现窗 go〔→ M-b/M-d 承载〕+ 22 行 defer-to-G15+；§3 G14 新增候选 7 行——
G14-N1 登记表方差带修订→M-a / G14-N2 UE benchmark 臂测量→M-b / G14-N3 Rurix 管线性能→M-c /
G14-N4 双端帧率对标+画质零降级守护→M-d / G14-N5 回归门+漂移监控→M-e /
G14-N6 异己面 no-go / G14-N7 帧率通过线 ×1.00 口径裁决登记〔go 非实现门，→ M-d 判据承载〕）、
裁决枚举合法（go/no-go/defer-to-G15+/strategic_override）、零空行（全列非空）、承接锚纪律
（§1 行承接锚 = G13.5a 字面 0-byte 转引，含「→」分节与 G14+ 承接源字面；§3 行含
「重判条件 + 兜底」字面）、defer-to-G15+ 裁决行 G15+ 重评窗字面（裁决/最终状态列
承载，转引列不回写）、go 行验收映射锚义务（登记留痕位置含 G14_ACCEPTANCE_MAP——
G14-N7 登记留痕位置含「G14_ACCEPTANCE_MAP §1 M-d 行判据字面」，同满足锚义务）、
no-go 行 RD/契约锚义务、§2 RD 七条（RD-034/039/040/041/042/043/044）行集闭集 +
条目级 status==open（经 g11_wave_exit_lib DEFERRED_PATH 读 registry/deferred.json 机核，
零新 RD max=RD-044）；外加横向机核——
  ① 与 G14_ACCEPTANCE_MAP 5 key（M-a~M-e 全 P0）互斥：候选行 ID 不得命中已 go 门裸 token；
  ② registry/deferred.json 对账：RD-034/039/040/041/042/043/044 七条目级 status 全 open、
    零新 RD（max=RD-044）。
（§2 RD 映射 7 行维持 open，不重复计入 31 行候选闭集三值枚举。）
只读文档与 registry，不代绿实现门；no-go/defer 如实保持 open 不写进全绿叙述。

用法：
  py -3 ci/g14_candidate_decisions_check.py --gate g14.wave.1.candidate_decisions
  py -3 ci/g14_candidate_decisions_check.py --selftest
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
GATE_KEY = "g14.wave.1.candidate_decisions"
NUMERIC_STEP = 248  # 落盘前实测 registry/number_ledger.json CI_step.next_free=247 顺位领取
SUBJECT = "g14_candidate_decisions_check"
WAVE = "G14.1"
DECISIONS = ROOT / "milestones" / "g14" / "G14_CANDIDATE_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_candidate_decisions_check_evidence_schema.json"
ACCEPTANCE_MAP = ROOT / "milestones" / "g14" / "G14_ACCEPTANCE_MAP.md"
DEFERRED = DEFERRED_PATH

# 冻结 ID 闭集（31 行）= §1 G13 defer-to-G14+ 24 行承接 + §3 G14 新增候选 7 行。
# （§2 open RD 7 行 = 映射行维持 open，不重复计入候选闭集；行集与 status 由
# sec2_rd_set / row_RD-* / deferred_rd_open_reconcile 三面独立机核。）
SEC1_IDS = [
    "M61",          # G13 defer 承接：RD-039 mesh shader→M109
    "M52",          # G13 defer 承接：RD-040 SER→M108（G13.4 重评窗未命中 + G14 窗维持未命中在案）
    "M100-high",    # G13 defer 承接：RD-040 ReSTIR 高档（G14 窗登记 = 未齐备）
    "SAFE-GPU",     # G13 defer 承接：Safe GPU Operator Platform（独立期）
    "M127",         # G13 defer 承接：神经变形研究子轨
    "M98-l4",       # G13 defer 承接：M98 L4 Far Field
    "M114-strand",  # G13 defer 承接：毛发 strand 档（G14 窗登记 = 数据面部分落地）
    "M118-hdr-cal", # G13 defer 承接：HDR 设备标定层
    "M125-adopt3",  # G13 defer 承接：Jolt 5.6 采纳三件
    "G10-N6",       # G13 defer 承接：BistroExterior 缺口
    "G10-N8",       # G13 defer 承接：-renderoffscreen 未测（G14 窗登记）
    "G10-N11",      # G13 defer 承接：M141 采样形态+MRQ 开销口径（**go：G14 兑现窗 → M-b 承载**）
    "G10-N16",      # G13 defer 承接：GPU 管线双端 A/B 帧率面（**go：G14 兑现窗 → M-d 承载**）
    "G10-N17",      # G13 defer 承接：M137 scalars.flip 演进位（G14 窗 = 未消费）
    "G11-N3",       # G13 defer 承接：GPU 管线画质差距面（G14 部分兑现：A/B 出图面 M-d 承载，差距清单锚定 G15）
    "G11-N5",       # G13 defer 承接：锁定度量反向激励旁证（G14 窗 = 未齐备）
    "G11-N8",       # G13 defer 承接：太阳穿玻璃高光尾（锚定 G15）
    "G11-N9",       # G13 defer 承接：c1_ue_specular_ibl 实测上界（锚定 G15）
    "G12-N10",      # G13 defer 承接：材质链（锚定 G15）
    "G12-N12",      # G13 defer 承接：UE PT 差距登记表 10 行处置（锚定 G15；只消费不回写）
    "G12-N13",      # G13 defer 承接：M165 间歇非确定性事件（M-e 漂移监控臂承接）
    "G13-N7",       # G13 defer 承接：帧生成 FG/MFG（G14 重评窗结论 = 不立项）
    "G13-N8",       # G13 defer 承接：UE 超分差距登记表 8 行处置（锚定 G15；只消费不回写）
    "G13-N9",       # G13 defer 承接：UE Lumen 差距登记表 2 行处置（锚定 G15；只消费不回写）
]
SEC2_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044"]
SEC3_IDS = [
    "G14-N1",  # M-c/M-d 门登记表 UE 方差带结构化对账修订（go → M-a）
    "G14-N2",  # UE benchmark 臂正式帧率测量（go → M-b）
    "G14-N3",  # Rurix 生产管线性能面（go → M-c）
    "G14-N4",  # 双端帧率正式对标 + 画质零降级守护（go → M-d）
    "G14-N5",  # 回归门 + M165 漂移监控（go → M-e）
    "G14-N6",  # 异己并发会话 src/ 未提交面（no-go：维持未提交不混入严禁消费）
    "G14-N7",  # 帧率通过线「略高」×1.00 量化口径（go：口径裁决登记非实现门 → M-d 判据承载）
]
FROZEN_IDS = SEC1_IDS + SEC3_IDS
ALLOWED = frozenset({"go", "no-go", "defer-to-G15+", "strategic_override"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|待补|—|-)?$", re.I)
ID_RE = re.compile(r"^(M\d|RD-\d|SAFE-GPU|G1[01234]-N\d|M-[a-e])")
# §1 go 行（G14 兑现窗）；其余 §1 行一律 defer-to-G15+。
SEC1_GO_IDS = frozenset({"G10-N11", "G10-N16"})
SEC3_GO_IDS = frozenset({"G14-N1", "G14-N2", "G14-N3", "G14-N4", "G14-N5", "G14-N7"})
SEC3_NO_GO_IDS = frozenset({"G14-N6"})
SEC3_DEFER_IDS = frozenset()  # G14 §3 零 defer 行（G14-N7 = go 口径裁决登记行，非 defer）
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


def _decision_base(decision: str) -> str | None:
    return (
        "no-go" if decision.startswith("no-go")
        else "go" if decision.startswith("go")
        else "defer-to-G15+" if decision.startswith("defer-to-G15+")
        else "strategic_override" if decision.startswith("strategic_override")
        else None
    )


def validate(
    tables: dict[str, list[list[str]]],
    map_text: str | None = None,
    deferred_data: dict | None = None,
) -> list[dict]:
    """43 facts：set_equality_frozen / no_duplicate_ids / row×38 / sec2_rd_set / 两横向机核。"""
    results: list[dict] = []
    sec1, sec2, sec3 = tables["sec1"], tables["sec2"], tables["sec3"]
    cand_ids = [r[0] for r in sec1 + sec3 if r]
    all_ids = [r[0] for r in sec1 + sec2 + sec3 if r]
    set_ok = set(cand_ids) == set(FROZEN_IDS) and len(cand_ids) == len(FROZEN_IDS)
    results.append({
        "id": "set_equality_frozen",
        "status": "PASS" if set_ok else "FAIL",
        "detail": f"got n={len(cand_ids)} unique={len(set(cand_ids))}; expect frozen {len(FROZEN_IDS)}"
        + ("" if set_ok else f"; diff={sorted(set(FROZEN_IDS) ^ set(cand_ids))}"),
    })
    if len(all_ids) != len(set(all_ids)):
        results.append({
            "id": "no_duplicate_ids",
            "status": "FAIL",
            "detail": f"duplicates: {[x for x in all_ids if all_ids.count(x) > 1]}",
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
        base = _decision_base(decision)
        if base is None:
            parts.append(f"非法裁决 {decision!r}")
        for i, cell in enumerate(r):
            if cell_empty(cell):
                parts.append(f"空单元格 col{i}")
                break
        anchor = r[2]
        if "→" not in anchor:
            parts.append("承接锚字面缺「→」分节（G13.5a 0-byte 转引口径）")
        # §1 锚列 = G13.5a 承接锚 0-byte 转引（G14+ 字面）；G15+ 重评窗由裁决列
        # defer-to-G15+ 自身与最终状态列承载（转引列不回写）。
        if base == "defer-to-G15+" and "G14+" not in anchor:
            parts.append("defer 缺承接源 G14+ 重评窗字面（G13.5a 0-byte 转引口径）")
        if rid in SEC1_GO_IDS:
            if base != "go":
                parts.append(f"{rid} 应为 go（G14 兑现窗），实测 {decision!r}")
            if "G14_ACCEPTANCE_MAP" not in r[6]:
                parts.append("go 行缺验收映射锚（登记留痕位置须含 G14_ACCEPTANCE_MAP）")
        elif base != "defer-to-G15+":
            parts.append(f"{rid} 应为 defer-to-G15+ 承接，实测 {decision!r}")
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
        base = _decision_base(decision)
        # G14-N7 = 口径裁决登记行：裁决列 =「go（本行 = 口径裁决登记，非实现门）」，
        # 枚举基底 go 合法（MAP §5 裁决枚举口径；非实现门锚义务同 go 行）。
        if base is None:
            parts.append(f"非法裁决 {decision!r}")
        for i, cell in enumerate(r):
            if cell_empty(cell):
                parts.append(f"空单元格 col{i}")
                break
        anchor = r[7]
        if "重判条件" not in anchor or "兜底" not in anchor:
            parts.append("承接锚缺「重判条件/兜底」字面")
        if base == "defer-to-G15+" and "G15+" not in anchor:
            parts.append("defer 缺 G15+ 重评窗字面")
        if rid in SEC3_GO_IDS:
            if base != "go":
                parts.append(f"{rid} 应为 go，实测 {decision!r}")
            if "G14_ACCEPTANCE_MAP" not in r[8]:
                parts.append("go 行缺验收映射锚（登记留痕位置须含 G14_ACCEPTANCE_MAP）")
        elif rid in SEC3_NO_GO_IDS:
            if base != "no-go":
                parts.append(f"{rid} 应为 no-go，实测 {decision!r}")
            if not any(a in r[6] for a in NO_GO_ANCHORS):
                parts.append("no-go 缺 RD/矩阵/契约/计划/MAP 锚")
        elif rid in SEC3_DEFER_IDS and base != "defer-to-G15+":
            parts.append(f"{rid} 应为 defer-to-G15+，实测 {decision!r}")
        results.append({
            "id": f"row_{rid}",
            "status": "PASS" if not parts else "FAIL",
            "detail": "; ".join(parts) if parts else f"{decision}",
        })

    # --- §2 行集闭集：RD-034/039/040/041/042/043/044 七行全等（映射行不入 31 行候选闭集） ---
    sec2_ids = [r[0] for r in sec2 if r]
    sec2_ok = set(sec2_ids) == set(SEC2_IDS) and len(sec2_ids) == len(SEC2_IDS)
    results.append({
        "id": "sec2_rd_set",
        "status": "PASS" if sec2_ok else "FAIL",
        "detail": f"got {sorted(sec2_ids)}; expect {sorted(SEC2_IDS)}"
        + ("" if sec2_ok else f"; diff={sorted(set(SEC2_IDS) ^ set(sec2_ids))}"),
    })

    # --- 横向机核①：与 G14_ACCEPTANCE_MAP 5 key 互斥 ---
    mt = map_text if map_text is not None else (
        ACCEPTANCE_MAP.read_text(encoding="utf-8") if ACCEPTANCE_MAP.is_file() else ""
    )
    gated = {f"M-{m}" for m in re.findall(r"g14\.p0\.m_([a-e])\.", mt)}
    hit = sorted(set(all_ids) & gated)
    mutex_ok = not hit and len(gated) == 5
    results.append({
        "id": "acceptance_map_mutex",
        "status": "PASS" if mutex_ok else "FAIL",
        "detail": f"MAP 实解 P0={len(gated)}（expect 5）；候选表命中已 go 门裸 token: {hit or '无'}",
    })

    # --- 横向机核②：deferred.json 对账（RD 七条目级 status 全 open，零新 RD max=RD-044） ---
    dd = deferred_data if deferred_data is not None else (
        wel.load_json(DEFERRED) if DEFERRED.is_file() else {"entries": []}
    )
    entries = dd.get("entries") or []
    status_map = {e.get("id"): e.get("status") for e in entries}
    rec_ok = True
    rec_parts: list[str] = []
    for rd in SEC2_IDS:
        st = status_map.get(rd)
        if st != "open":
            rec_ok = False
            rec_parts.append(f"{rd} 条目级 status={st!r}（要求 open）")
    rd_nums = [int(m.group(1)) for e in entries for m in [re.match(r"RD-(\d+)$", e.get("id") or "")] if m]
    if not rd_nums or max(rd_nums) != 44:
        rec_ok = False
        rec_parts.append(f"RD max={max(rd_nums) if rd_nums else None} expect 44（零新 RD）")
    rec_parts.append(f"RD 七条 status: {[(r, status_map.get(r)) for r in SEC2_IDS]}")
    results.append({
        "id": "deferred_rd_open_reconcile",
        "status": "PASS" if rec_ok else "FAIL",
        "detail": "; ".join(rec_parts),
    })
    return results


def run_check(
    path: Path | None = None,
    map_text: str | None = None,
    deferred_data: dict | None = None,
) -> tuple[int, list[dict]]:
    p = path or DECISIONS
    if not p.is_file():
        # 诚实红：表未落盘不是绿
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}（G14.1 候选决策表未落盘；诚实红，不假绿）"}]
    results = validate(
        parse_tables(p.read_text(encoding="utf-8")),
        map_text=map_text,
        deferred_data=deferred_data,
    )
    ok = all(x["status"] == "PASS" for x in results)
    return (0 if ok else 1), results


def emit(results: list[dict], overall_ok: bool) -> int:
    for r in results:
        print(f"  FACT  {r['status']:4}  {r['id']}  ({r.get('detail','')})")
    if not SCHEMA_PATH.is_file():
        print(f"[g14_candidate_decisions] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G14_CONTRACT G-G14-1/§6/§7;G14_CANDIDATE_DECISIONS.md v1.0;G14_ACCEPTANCE_MAP §1/§2;G13_P2_DECISIONS.md v1.0（24 行承接锚法定输入）;registry/deferred.json（RD 七条目级 status open 机核）",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G14.1 治理门——候选决策表 31 行闭集（go 8〔§1 G10-N11/G10-N16 G14 兑现窗 + §3 G14-N1~N5 + G14-N7 口径裁决登记〕+ no-go 1〔G14-N6 异己面〕+ defer-to-G15+ 22〔§1 内〕+ strategic_override 0；§2 RD 映射 7 行维持 open 不重复计入三值枚举）：裁决枚举/零空行/承接锚纪律 + MAP 5 key 互斥 + deferred 对账（RD-034/039/040/041/042/043/044 条目级 status 全 open，零新 RD max=RD-044）；no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=overall_ok,
    )
    return code


# ---------------------------------------------------------------------------
# selftest 合成夹具。
# ---------------------------------------------------------------------------

def _synth_sec1(rid: str) -> str:
    if rid in SEC1_GO_IDS:
        decision = "go（G14 兑现窗 → M-b 承载）"
        wave = "G14.2（M-b）"
        reg = "本表 §1 行 + G14_ACCEPTANCE_MAP §1 M-b 行"
        final = "go（G14.2 M-b 承载）"
    else:
        decision = "defer-to-G15+"
        wave = "—（非 go）"
        reg = "本表 §1 行（不新设 RD）"
        final = "open-defer"
    return (
        f"| {rid} | 分项 | 「G14+ 重评窗触发条件齐备 → 兜底面维持（字面 0-byte）」 | {decision} | 理由 | {wave} | {reg} | {final} |\n"
    )


def _synth_sec2(rid: str) -> str:
    return f"| {rid} | title | open | 维持 open | 无 | 理由 | 本表 §2 行 |\n"


def _synth_sec3(rid: str) -> str:
    if rid in SEC3_GO_IDS:
        decision = "go（本行 = 口径裁决登记，非实现门）" if rid == "G14-N7" else "go"
        anchor = "重判条件 = G15+ 若口径面再发现过严/过松面时按只追加程序重判；兜底 = 现机核维持，门绿 0-byte"
        ref = "用户目标（2026-08-19 会话留痕）"
        reg = "本表 §3 行 + G14_ACCEPTANCE_MAP §1 行"
    elif rid in SEC3_NO_GO_IDS:
        decision = "no-go"
        anchor = "重判条件 = G15+ 触发条件齐备时按只追加程序重判；兜底 = 既有面维持"
        ref = "G14_CONTRACT §7 立项裁决 1"
        reg = "本表 §3 行（不新设 RD）"
    else:
        decision = "defer-to-G15+"
        anchor = "重判条件 = G15+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有已验收面维持"
        ref = "registry/deferred.json RD-041 / rfcs/0016"
        reg = "本表 §3 行 + §2 RD-041 行（不新设 RD）"
    return f"| {rid} | 分项 | 来源 | {decision} | 理由 | 波次 | {ref} | {anchor} | {reg} |\n"


def _full_fixture() -> str:
    sec1_head = "| ID | 分项名 | 承接锚字面 | G14 裁决 | 裁决理由 | 波次/联动面 | 登记留痕位置 | 最终状态 |\n|---|---|---|---|---|---|---|---|\n"
    sec2_head = "| RD | title | 条目级 status | G14 处置 | G14 联动面 | 裁决理由 | 留痕位置 |\n|---|---|---|---|---|---|---|\n"
    sec3_head = "| 候选 ID | 分项名 | 来源 | G14 裁决 | 裁决理由 | 波次归属 | 依据/证据路径 | 承接锚 / 兜底 | 登记留痕位置 |\n|---|---|---|---|---|---|---|---|---|\n"
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
            print("[selftest] PASS: 真表 31 行闭集绿")

    with tempfile.TemporaryDirectory(prefix="g14_cand_selftest_") as td:
        # 正样本 2：合成全表（真树 MAP/deferred 对账）必须绿
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
            "§1 defer 行承接锚缺 G14+ → 红",
            full.replace(
                "「G14+ 重评窗触发条件齐备 → 兜底面维持（字面 0-byte）」",
                "「重评窗触发条件齐备 → 兜底面维持（字面 0-byte）」",
                1,
            ),
            "row_M61",
        )
        _red(
            "非法裁决枚举（§3 go→maybe）→ 红",
            full.replace("| G14-N1 | 分项 | 来源 | go |", "| G14-N1 | 分项 | 来源 | maybe |", 1),
            "row_G14-N1",
        )
        _red(
            "空单元格（§3 裁决理由置空）→ 红",
            full.replace("| G14-N3 | 分项 | 来源 | go | 理由 |", "| G14-N3 | 分项 | 来源 | go |  |", 1),
            "row_G14-N3",
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
        _red(
            "§1 go 行缺验收映射锚 → 红",
            full.replace("本表 §1 行 + G14_ACCEPTANCE_MAP §1 M-b 行", "本表 §1 行（不新设 RD）", 1),
            "row_G10-N11",
        )
        _red(
            "§3 go 行承接锚缺「兜底」→ 红",
            full.replace(
                "重判条件 = G15+ 若口径面再发现过严/过松面时按只追加程序重判；兜底 = 现机核维持，门绿 0-byte",
                "重判条件 = G15+ 若口径面再发现过严/过松面时按只追加程序重判",
                1,
            ),
            "row_G14-N1",
        )
        _red(
            "§2 缺行（删 RD-043）→ sec2_rd_set 红",
            "\n".join(ln for ln in full.splitlines() if not ln.strip().startswith("| RD-043 |")) + "\n",
            "sec2_rd_set",
        )

        # deferred 对账失配（RD-040 status 改 closed）→ 红
        real = wel.load_json(DEFERRED)
        flipped = {
            **real,
            "entries": [
                {**e, "status": "closed"} if e.get("id") == "RD-040" else e
                for e in real.get("entries", [])
            ],
        }
        c, rs = run_check(p, deferred_data=flipped)
        hit = [r for r in rs if r["id"] == "deferred_rd_open_reconcile" and r["status"] == "FAIL"]
        if c != 0 and hit:
            print(f"  RED ok   — deferred RD-040 status 非 open（{hit[0]['detail'][:80]}）")
        else:
            print("  RED MISS — deferred RD status 非 open 未被判红")
            failures += 1

    if failures:
        print(f"[g14_candidate_decisions] SELFTEST FAIL ({failures})")
        return 1
    print("[g14_candidate_decisions] SELFTEST PASS (10 RED + 真表/合成双臂 GREEN)")
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
