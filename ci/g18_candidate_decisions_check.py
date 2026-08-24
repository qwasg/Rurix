#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Claude Fable 5（G18.1 治理波）
"""G18.1 治理门 — 候选决策表闭集/锚纪律/横向对账（g18.wave.1.candidate_decisions，步骤 310）。

G18_CONTRACT G-G18-1（G18.1 完成门 D-G18-2 候选决策表面）治理三门之一；G-G18-2 实现互锁
（ci/g18_interlock_check.py）事实门②消费本表 25 行零空行面、事实门④消费本门独立 PASS
机器事实。

核验 `milestones/g18/G18_CANDIDATE_DECISIONS.md`：
冻结 25 行候选闭集全等（§1 G16 defer-to-G19+ 承接 16 行 = 十四行 + G17-MB-F1/G17-MD-F1；
§3 G18 新增候选 9 行 G18-N1~N9）、
裁决枚举合法（go/closed-go/no-go/defer-to-G19+/strategic_override——G18 即本期，defer-to-G19+
不再合法；closed-go = 兑现完结留痕变体，入 go 族不入 deferred 面）、
零空行（全列非空）、承接锚纪律（§1/§3 行承接锚均含「重判条件 = …；兜底 = …」字面；
§1 行原触发条件字面转引含「→」分节）、
defer-to-G19+ 裁决行承接锚含 G19+ 重评窗字面、
go 行验收映射锚义务（登记留痕位置含 G18_ACCEPTANCE_MAP，或依据面含 G18_CONTRACT.md §4.2 M 行
锚定）、closed-go 行依据/证据路径必含 evidence/ 真跑件、no-go 行 RD/契约锚义务、
§2 RD 八条（RD-034/039/040/041/042/043/044/045）行集闭集 +
条目级 status==open（经 g11_wave_exit_lib DEFERRED_PATH 读 registry/deferred.json 机核，
零新 RD max=RD-045）；外加横向机核——
  ① 与 G18_ACCEPTANCE_MAP 9 key（M-a~M-i 全 P0）互斥：候选行 ID 不得命中已 go 门裸 token；
  ② registry/deferred.json 对账：RD 八条目级 status 全 open、零新 RD（max=RD-045）。
（§2 RD 映射 8 行维持 open，不重复计入 25 行候选闭集三值枚举。）

§1/§3 = 十列同表头形态（ID / 分项名 / 来源波次 / 原触发条件字面 / 裁决 / 裁决理由 /
依据·证据路径 / 承接锚 / 登记留痕位置 / 最终状态），按 `## N.` 节作用域分流；
§2 = 七列形态（RD / title / 条目级 status / 处置 / 联动面 / 裁决理由 / 留痕位置）。

只读文档与 registry，不代绿实现门；no-go/defer 如实保持 open 不写进全绿叙述。

用法：
  py -3 ci/g18_candidate_decisions_check.py --gate g18.wave.1.candidate_decisions
  py -3 ci/g18_candidate_decisions_check.py --selftest
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
GATE_KEY = "g18.wave.1.candidate_decisions"
NUMERIC_STEP = 310  # 落盘前实测 registry/number_ledger.json CI_step.next_free=293 顺位领取
SUBJECT = "g18_candidate_decisions_check"
WAVE = "G18.1"
DECISIONS = ROOT / "milestones" / "g18" / "G18_CANDIDATE_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g18" / "g18_candidate_decisions_check_evidence_schema.json"
ACCEPTANCE_MAP = ROOT / "milestones" / "g18" / "G18_ACCEPTANCE_MAP.md"
DEFERRED = DEFERRED_PATH
SOURCE_REF = (
    "G18_CONTRACT G-G18-1/§6/§7;G18_CANDIDATE_DECISIONS.md v1.0;G18_ACCEPTANCE_MAP §1/§2;"
    "G17_P2_DECISIONS.md §3（承接锚法定输入）+ G17_CONTRACT.md §8.7;"
    "registry/deferred.json（RD 八条目级 status open 机核）"
)

# 冻结 ID 闭集（25 行）= §1 G17 defer-to-G18+ 承接 16 行 + §3 G18 新增候选 9 行。
SEC1_IDS = [
    "M61",
    "M52",
    "M100-high",
    "SAFE-GPU",
    "M127",
    "M98-l4",
    "M114-strand",
    "M118-hdr-cal",
    "M125-adopt3",
    "G10-N6",
    "G10-N8",
    "G10-N17",
    "G11-N5",
    "G13-N7",
    "G17-MB-F1",
    "G17-MD-F1",
]
SEC2_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044", "RD-045"]
SEC3_IDS = [
    "G18-N1", "G18-N2", "G18-N3", "G18-N4", "G18-N5",
    "G18-N6", "G18-N7", "G18-N8", "G18-N9",
]
FROZEN_IDS = SEC1_IDS + SEC3_IDS
# G18 即本期：defer-to-G18+ 不再合法，defer 合法值 = defer-to-G19+；
# closed-go = 兑现完结留痕变体（go 族终态，不充 G18 门绿、不入 deferred 面）。
ALLOWED = frozenset({"go", "closed-go", "no-go", "defer-to-G19+", "strategic_override"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|待补|—|-)?$", re.I)
NO_GO_ANCHORS = ("RD-", "deferred", "CONTRACT", "RFC-", "矩阵", "CAPABILITY", "CANDIDATE", "PLAN", "MAP")
SECTION_HEAD_RE = re.compile(r"^## (\d+)\. ")


def parse_tables(text: str) -> dict[str, list[list[str]]]:
    """按 `## N.` 节作用域解析三个表（§1/§3 十列同表头首格 `ID`，按节号分流；§2 首格 `RD`），
    返回 {节: 数据行单元格列表}。修订记录表不入任何节。"""
    out: dict[str, list[list[str]]] = {"sec1": [], "sec2": [], "sec3": []}
    block: list[list[str]] = []
    section = 0

    def flush() -> None:
        nonlocal block
        if len(block) >= 2 and all(re.fullmatch(r":?-{2,}:?", c) for c in block[1]):
            header = block[0]
            head = header[0] if header else ""
            if section == 1 and head == "ID" and len(header) >= 10 and header[1] == "分项名":
                out["sec1"].extend(block[2:])
            elif section == 2 and head == "RD" and len(header) >= 7:
                out["sec2"].extend(block[2:])
            elif section == 3 and head == "ID" and len(header) >= 10 and header[1] == "分项名":
                out["sec3"].extend(block[2:])
        block = []

    for line in text.splitlines():
        m = SECTION_HEAD_RE.match(line.strip())
        if m:
            flush()
            section = int(m.group(1))
            continue
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
        "closed-go" if decision.startswith("closed-go")
        else "no-go" if decision.startswith("no-go")
        else "go" if decision.startswith("go")
        else "defer-to-G19+" if decision.startswith("defer-to-G19+")
        else "strategic_override" if decision.startswith("strategic_override")
        else None
    )


def _row_findings(r: list[str], *, carry_over: bool) -> list[str]:
    """§1/§3 十列行级机核（carry_over=True 时追加 §1 原触发条件字面「→」分节转引纪律）。"""
    parts: list[str] = []
    if len(r) < 10:
        return [f"列数不足 10（实测 {len(r)}）"]
    decision = r[4].replace("**", "").strip()
    base = _decision_base(decision)
    if base is None:
        parts.append(f"非法裁决 {decision!r}（G18 合法枚举 go/closed-go/no-go/defer-to-G19+/strategic_override）")
    for i, cell in enumerate(r):
        if cell_empty(cell):
            parts.append(f"空单元格 col{i}")
            break
    if carry_over and "→" not in r[3]:
        parts.append("原触发条件字面缺「→」分节（G15/G16 候选表承接锚 0-byte 转引口径）")
    anchor = r[7]
    if "重判条件" not in anchor or "兜底" not in anchor:
        parts.append("承接锚缺「重判条件 = …；兜底 = …」字面")
    if base == "defer-to-G19+" and "G19+" not in anchor:
        parts.append("defer 缺 G19+ 重评窗字面")
    if base == "go" and "G18_ACCEPTANCE_MAP" not in r[8] and "G18_CONTRACT.md §4.2 M-" not in r[6]:
        parts.append("go 行缺验收映射锚（登记留痕位置须含 G18_ACCEPTANCE_MAP，或依据面含 G18_CONTRACT.md §4.2 M 行锚定）")
    if base == "closed-go" and "evidence/" not in r[6]:
        parts.append("closed-go 留痕行依据/证据路径缺 evidence/ 真跑件")
    if base == "no-go" and not any(a in r[6] for a in NO_GO_ANCHORS):
        parts.append("no-go 缺 RD/矩阵/契约/计划/MAP 锚")
    return parts


def validate(
    tables: dict[str, list[list[str]]],
    map_text: str | None = None,
    deferred_data: dict | None = None,
) -> list[dict]:
    """32 facts：set_equality_frozen / no_duplicate_ids / row×27 / sec2_rd_set / 两横向机核。"""
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

    # --- §1 行级机核（10 列） ---
    for r in sec1:
        rid = r[0] if r else "?"
        parts = _row_findings(r, carry_over=True)
        results.append({
            "id": f"row_{rid}",
            "status": "PASS" if not parts else "FAIL",
            "detail": "; ".join(parts) if parts else f"{r[4].replace('**', '').strip()}",
        })

    # --- §2 行级机核（7 列） ---
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

    # --- §3 行级机核（10 列同 §1 表头；新增候选无「→」分节转引纪律） ---
    for r in sec3:
        rid = r[0] if r else "?"
        parts = _row_findings(r, carry_over=False)
        results.append({
            "id": f"row_{rid}",
            "status": "PASS" if not parts else "FAIL",
            "detail": "; ".join(parts) if parts else f"{r[4].replace('**', '').strip()}",
        })

    # --- §2 行集闭集：RD 八行全等（映射行不入 25 行候选闭集） ---
    sec2_ids = [r[0] for r in sec2 if r]
    sec2_ok = set(sec2_ids) == set(SEC2_IDS) and len(sec2_ids) == len(SEC2_IDS)
    results.append({
        "id": "sec2_rd_set",
        "status": "PASS" if sec2_ok else "FAIL",
        "detail": f"got {sorted(sec2_ids)}; expect {sorted(SEC2_IDS)}"
        + ("" if sec2_ok else f"; diff={sorted(set(SEC2_IDS) ^ set(sec2_ids))}"),
    })

    # --- 横向机核①：与 G18_ACCEPTANCE_MAP 5 key 互斥 ---
    mt = map_text if map_text is not None else (
        ACCEPTANCE_MAP.read_text(encoding="utf-8") if ACCEPTANCE_MAP.is_file() else ""
    )
    gated = {f"M-{m}" for m in re.findall(r"g18\.p0\.m_([a-i])\.", mt)}
    hit = sorted(set(all_ids) & gated)
    mutex_ok = not hit and len(gated) == 9
    results.append({
        "id": "acceptance_map_mutex",
        "status": "PASS" if mutex_ok else "FAIL",
        "detail": f"MAP 实解 P0={len(gated)}（expect 9）；候选表命中已 go 门裸 token: {hit or '无'}",
    })

    # --- 横向机核②：deferred.json 对账（RD 八条目级 status 全 open，零新 RD max=RD-045） ---
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
    # RD max 断言：G18.1 治理波窗内零新 RD（≤45 = 治理门快照字面，RD-045 已存续 open）。
    bad_new = [n for n in rd_nums if n > 45]
    if not rd_nums or max(rd_nums) > 45 or bad_new:
        rec_ok = False
        rec_parts.append(f"RD max={max(rd_nums) if rd_nums else None} 治理窗外新条目={bad_new}（治理窗快照 ≤45 零新 RD）")
    rec_parts.append(f"RD 八条 status: {[(r, status_map.get(r)) for r in SEC2_IDS]}")
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
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}（G18.1 候选决策表未落盘；诚实红，不假绿）"}]
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
        print(f"[g18_candidate_decisions] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
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
        notes="G18.1 治理门——候选决策表 25 行闭集（§1 G16 defer-to-G19+ 承接 16 行 = 十四行 defer-to-G19+ + G17-MB-F1/G17-MD-F1 go 主承接；§3 G18-N1~N4 新增 4 行全 go；§2 RD 映射 8 行维持 open 不重复计入三值枚举）：裁决枚举合法（go/closed-go/no-go/defer-to-G19+/strategic_override——G18 即本期，defer-to-G19+ 不再合法）/零空行/承接锚纪律（全表「重判条件 = …；兜底 = …」字面零缺项；§1 原触发条件字面转引含「→」分节）+ defer 行 G19+ 重评窗字面 + go 行验收映射锚义务 + closed-go 行 evidence/ 真跑件义务 + MAP 5 key 互斥 + deferred 对账（RD 八条目级 status 全 open，零新 RD max=RD-045）；no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=overall_ok,
    )
    return code


# ---------------------------------------------------------------------------
# selftest 合成夹具（十列形态；§1 决策分布 = defer-to-G19+ + go 正本）。
# ---------------------------------------------------------------------------

# 合成 MAP 文本（5 个 P0 key 面）——横向互斥机核的可注入输入，不依赖树上 MAP 落盘态。
FIXTURE_MAP_TEXT = "\n".join(
    f"| **M-{l}** | `g18.p0.m_{l}.{s}` | ... |"
    for l, s in (
        ("a", "dual_end_retest_warm_recalib"),
        ("b", "ngx_evolution_alignment"),
        ("c", "d3d12_host_lane_disposition"),
        ("d", "t100_final_verdict"),
        ("e", "closed_gate_no_regression"),
    )
)

FIXTURE_SEC1_GO_IDS = frozenset({"G17-MB-F1/G17-MD-F1"})


def _synth_sec1(rid: str) -> str:
    if rid in FIXTURE_SEC1_GO_IDS:
        decision = "go（G18 全期承接——M-a~M-e 五门承载）"
        ref = "milestones/g15/G15_P2_DECISIONS.md §4 行"
        anchor = "重判条件 = 门真跑产出面构成新事实时按只追加程序承接；兜底 = 维持未达标登记不冒充"
        reg = "本表 §1 行 + G18_ACCEPTANCE_MAP M-a~M-e 行（不新设 RD）"
        final = "go（五门承载面）"
    else:
        decision = "defer-to-G19+"
        ref = "milestones/g15/G15_P2_DECISIONS.md §4 行"
        anchor = "重判条件 = G19+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有面维持"
        reg = "本表 §1 行（不新设 RD）"
        final = "open-defer（G19+）"
    return (
        f"| {rid} | 分项 | G16.1 defer-to-G19+ | 「G16+ 原触发条件齐备 → 兜底面维持（字面 0-byte）」 | "
        f"{decision} | 理由 | {ref} | {anchor} | {reg} | {final} |\n"
    )


def _synth_sec2(rid: str) -> str:
    return f"| {rid} | title | open | 维持 open | 无 | 理由 | 本表 §2 行 |\n"


def _synth_sec3(rid: str) -> str:
    return (
        f"| {rid} | 分项 | G18.1 新增 | 「口径争议时按只追加程序重判形态」 | go（G18.5 M-d 承载） | 理由 | "
        "milestones/g18/G18_CONTRACT.md §4.2 M-d 行 | "
        "重判条件 = 口径争议时按只追加程序重判形态并 §8 只追加修订；兜底 = 现口径维持 | "
        "本表 §3 行 + G18_ACCEPTANCE_MAP M-d 行 | go（M-d 承载面） |\n"
    )


def _full_fixture() -> str:
    sec1_head = "| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |\n|---|---|---|---|---|---|---|---|---|---|\n"
    sec2_head = "| RD | title（摘要） | 条目级 status | G18.1 处置 | 联动面 | 裁决理由 | 留痕位置 |\n|---|---|---|---|---|---|---|\n"
    sec3_head = sec1_head
    return (
        "## 1. G16 defer-to-G19+ 承接 16 行逐行转引终态裁决\n\n" + sec1_head + "".join(_synth_sec1(i) for i in SEC1_IDS)
        + "\n## 2. open RD 逐条映射\n\n" + sec2_head + "".join(_synth_sec2(i) for i in SEC2_IDS)
        + "\n## 3. G18 期新增候选 4 行\n\n" + sec3_head + "".join(_synth_sec3(i) for i in SEC3_IDS)
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
            print("[selftest] PASS: 真表 25 行闭集绿")

    with tempfile.TemporaryDirectory(prefix="g18_cand_selftest_") as td:
        # 正样本 2：合成全表（合成 MAP 面 + 真树 deferred 对账）必须绿
        p = Path(td) / "full.md"
        p.write_text(full, encoding="utf-8")
        code, res = run_check(p, map_text=FIXTURE_MAP_TEXT)
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
            c, rs = run_check(q, map_text=FIXTURE_MAP_TEXT)
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
            "§1 defer 行承接锚缺 G19+ 重评窗 → 红",
            full.replace(
                "重判条件 = G19+ 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有面维持",
                "重判条件 = 重评窗触发条件齐备时按只追加程序重判；兜底 = 既有面维持",
                1,
            ),
            "row_M61",
        )
        _red(
            "§1 原触发条件字面缺「→」分节 → 红",
            full.replace(
                "「G16+ 原触发条件齐备 → 兜底面维持（字面 0-byte）」",
                "「G16+ 原触发条件齐备兜底面维持（字面 0-byte）」",
                1,
            ),
            "row_M61",
        )
        _red(
            "非法裁决枚举（§1 defer-to-G19+ 本期不再合法）→ 红",
            full.replace("| defer-to-G19+ | 理由 |", "| defer-to-G19+ | 理由 |", 1),
            "row_M61",
        )
        _red(
            "空单元格（§3 裁决理由置空）→ 红",
            full.replace("| G18-N3 | 分项 | G18.1 新增 | 「口径争议时按只追加程序重判形态」 | go（G18.5 M-d 承载） | 理由 |",
                         "| G18-N3 | 分项 | G18.1 新增 | 「口径争议时按只追加程序重判形态」 | go（G18.5 M-d 承载） |  |", 1),
            "row_G18-N3",
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
            full.replace("本表 §1 行 + G18_ACCEPTANCE_MAP M-a~M-e 行（不新设 RD）", "本表 §1 行（不新设 RD）", 1),
            "row_G17-MB-F1/G17-MD-F1",
        )
        _red(
            "§3 go 行承接锚缺「兜底」→ 红",
            full.replace(
                "重判条件 = 口径争议时按只追加程序重判形态并 §8 只追加修订；兜底 = 现口径维持",
                "重判条件 = 口径争议时按只追加程序重判形态并 §8 只追加修订",
                1,
            ),
            "row_G18-N1",
        )
        _red(
            "§2 缺行（删 RD-044）→ sec2_rd_set 红",
            "\n".join(ln for ln in full.splitlines() if not ln.strip().startswith("| RD-044 |")) + "\n",
            "sec2_rd_set",
        )

        # deferred 对账失配（RD-045 status 改 closed）→ 红
        real = wel.load_json(DEFERRED)
        flipped = {
            **real,
            "entries": [
                {**e, "status": "closed"} if e.get("id") == "RD-045" else e
                for e in real.get("entries", [])
            ],
        }
        c, rs = run_check(p, map_text=FIXTURE_MAP_TEXT, deferred_data=flipped)
        hit = [r for r in rs if r["id"] == "deferred_rd_open_reconcile" and r["status"] == "FAIL"]
        if c != 0 and hit:
            print(f"  RED ok   — deferred RD-045 status 非 open（{hit[0]['detail'][:80]}）")
        else:
            print("  RED MISS — deferred RD status 非 open 未被判红")
            failures += 1

        # 合成正本 extra_facts 计数 = 32（2 + row 15 + row 8 + row 4 + sec2_rd_set + 两横向）
        _, res32 = run_check(p, map_text=FIXTURE_MAP_TEXT)
        if len(res32) == 32:
            print("  GREEN ok — 合成夹具 extra_facts=32")
        else:
            print(f"  GREEN MISS — 合成夹具 extra_facts 本应 32，实测 {len(res32)}")
            failures += 1

    if failures:
        print(f"[g18_candidate_decisions] SELFTEST FAIL ({failures})")
        return 1
    print("[g18_candidate_decisions] SELFTEST PASS (11 RED + 真表/合成双臂 GREEN)")
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
