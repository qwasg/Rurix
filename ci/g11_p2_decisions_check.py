#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.6/G11.7 收口波）
"""G11.6 P2/留档/未触发分项穷举决策门 g11.wave.6.decisions(G11_CONTRACT G-G11-8)。

核验 `milestones/g11/G11_P2_DECISIONS.md`(2026-08-16 v1.0 落盘):
冻结 28 行候选闭集全等(G11.1 决策表校准后冻结——G11_CANDIDATE_DECISIONS 39 行
实记全集未进 14 key 验收面者 20 行〔§2 defer 15 + §4 G11-N3 + §3 RD 级
RD-034/042/043/044〕+ G11.2~G11.5b 期内新增 not-triggered/no-go/留档/closed-go
登记面 G11-N4~G11-N11 八行去重)、决策枚举合法
(go/no-go/defer-to-G12+/strategic_override)、零空行(全列非空)、承接锚
「重判条件 + 兜底」字面、defer 行必含 G12+ 重评窗、go 行 evidence 义务、
no-go 行 RD/矩阵/契约锚义务;外加三横向机核——
  ① 与 G11_ACCEPTANCE_MAP 14 key(13 P0 + 1 已 go P1)互斥:P2 行 ID 不得命中
    任何已 go M### 裸 token(M98-l4/M100-high 等子项级 key 不互斥);
  ② deferred.json history 对账:G11.6 P2 defer/兑现登记恰好 RD-039 +1(M61)/
    RD-040 +3(M52/M99-clipmap/M100-high——M99-clipmap = G11.4 承接兑现完结
    登记),零新 RD(max=RD-044),status 0-byte;
  ③ G11.1 候选决策表对账:15 行 G10 defer 承接行 + G11-N3 行裁决 ==
    G11_CANDIDATE_DECISIONS §2/§4 行集 defer-to-G12+ 字面逐字承接。
只读文档与 registry,不代绿实现门;no-go/defer 如实保持 open 不写进全绿叙述。

materialize:numeric_step=214(落盘前实测 CI_step.next_free=214 顺位领取);骨架期
(G11.1)行级机核由本版按候选全集口径扩为 28 行闭集(同 G10 先例「骨架期行级机核
→ materialize 期扩闭集」),行 key 逐字对账;同构 ci/g10_p2_decisions_check.py
全量形态。

用法:
  py -3 ci/g11_p2_decisions_check.py --gate g11.wave.6.decisions
  py -3 ci/g11_p2_decisions_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g11.wave.6.decisions"
NUMERIC_STEP = 214  # 落盘前实测 registry/number_ledger.json CI_step.next_free=214 顺位领取
SUBJECT = "g11_p2_decisions"
WAVE = "G11.6"
DECISIONS = ROOT / "milestones" / "g11" / "G11_P2_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_p2_decisions_evidence_schema.json"
ACCEPTANCE_MAP = ROOT / "milestones" / "g11" / "G11_ACCEPTANCE_MAP.md"
CANDIDATE = ROOT / "milestones" / "g11" / "G11_CANDIDATE_DECISIONS.md"
DEFERRED = wel.DEFERRED_PATH

# 冻结 ID 闭集(28 行)= G11.1 决策表校准后冻结(§2 defer 15 + §4 G11-N3 +
# §3 RD 级 4)+ G11.2~G11.5b 期内新增 G11-N4~G11-N11 八行——与
# G11_P2_DECISIONS §1 逐字对账。
FROZEN_IDS = [
    "M61",          # G10 defer 承接:RD-039 mesh shader→M109(G10.6 maintain-defer)
    "M52",          # G10 defer 承接:RD-040 SER→M108(G10.6 maintain-defer,锚定 G12)
    "M100-high",    # G10 defer 承接:RD-040 ReSTIR 高档(G11.6 触发评估兑现=证据未齐备)
    "SAFE-GPU",     # G10 defer 承接:Safe GPU Operator Platform(独立期)
    "M127",         # G10 defer 承接:神经变形研究子轨
    "M98-l4",       # G10 defer 承接:M98 L4 Far Field
    "M114-strand",  # G10 defer 承接:毛发 strand 档(锚定 G14)
    "M118-hdr-cal", # G10 defer 承接:HDR 设备标定层
    "M125-adopt3",  # G10 defer 承接:Jolt 5.6 采纳三件
    "G10-N5",       # G10 defer 承接:DLSS/Streamline 方向登记(锚定 G13)
    "G10-N6",       # G10 defer 承接:BistroExterior 缺口(G11.3 触发评估兑现=未触发)
    "G10-N8",       # G10 defer 承接:-renderoffscreen 未测
    "G10-N11",      # G10 defer 承接:M141 采样形态+MRQ 开销口径(锚定 G14)
    "G10-N16",      # G10 defer 承接:Rurix GPU 管线 A/B 面未测(锚定 G14)
    "G10-N17",      # G10 defer 承接:M137 scalars.flip 演进位(G11.5 触发评估兑现=不成立)
    "G11-N3",       # G11.1 新增:GPU 管线画质差距面未 measured(锚定 G14)
    "RD034",        # RD 级:DXIL RT/mesh 腿(no-go:blocked 维持,Vulkan 主腿)
    "RD042",        # RD 级:可微物理/机器人批仿(no-go:观察维持,红线不动)
    "RD043",        # RD 级:wgrapier GPU 刚体(no-go:观察维持,否决线不动)
    "RD044",        # RD 级:物理 P3+(no-go:maintain_no_go 维持,FLIP 防混淆)
    "G11-N4",       # G11.3 新增:M147 判据双 phase 修订留痕(go,closed-go 留痕)
    "G11-N5",       # G11.3 新增:锁定度量反向激励旁证(defer,G12+ 度量口径修订评估)
    "G11-N6",       # G11.4 新增:串扰标定中间件删除留痕(go,closed-go 留痕)
    "G11-N7",       # G11.5/5b 新增:首跑 FAIL→诊断修复→复测收敛全链留痕(go,closed-go)
    "G11-N8",       # G11.5b 新增:太阳穿玻璃高光尾(defer,锚定 G15)
    "G11-N9",       # G11.2/5b 新增:c1_ue_specular_ibl 实测上界(defer,锚定 G15)
    "G11-N10",      # G11.2 新增:HEAD 预存 fmt/clippy 漂移面(no-go:零修复纪律不回写)
    "G11-N11",      # G11.1 新增:异己会话 src/ 未提交面(no-go:维持未提交不混入)
]
FROZEN_IDS = [s.strip() for s in FROZEN_IDS if s.strip()]
ALLOWED = frozenset({"go", "no-go", "defer-to-G12+", "strategic_override"})
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|—|-)?$", re.I)
ID_RE = re.compile(r"^(M\d|RD\d|SAFE-GPU|G1[01]-N\d)")
HEADERS = [
    "ID", "分项名", "来源波次", "原触发条件字面", "裁决",
    "裁决理由", "依据/证据路径", "承接锚", "登记留痕位置", "最终状态",
]
# deferred.json history 对账期望:G11.6 P2 登记恰好 RD-039 +1(M61)/RD-040 +3
# (M52/M99-clipmap/M100-high——M99-clipmap = G11.4 承接兑现完结登记)。
EXPECTED_DEFER_HISTORY = {"RD-039": ["M61"], "RD-040": ["M52", "M99-clipmap", "M100-high"]}
HISTORY_MARKER = "G11.6 P2"
# G11.1 候选决策表对账闭集:§2 defer 15 行 + §4 G11-N3 行(defer-to-G12+ 字面承接)。
CANDIDATE_DEFER_IDS = [
    "M61", "M52", "M100-high", "SAFE-GPU", "M127", "M98-l4",
    "M114-strand", "M118-hdr-cal", "M125-adopt3", "G10-N5", "G10-N6",
    "G10-N8", "G10-N11", "G10-N16", "G10-N17", "G11-N3",
]
GO_IDS = frozenset({"G11-N4", "G11-N6", "G11-N7"})
NO_GO_IDS = frozenset({"RD034", "RD042", "RD043", "RD044", "G11-N10", "G11-N11"})


def parse_table(text: str) -> list[dict[str, str]]:
    """解析 §1 决策表(| ID | ... | 行;止于表后首个非 | 行,§3 锚清单表不入)。"""
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


def validate_rows(
    rows: list[dict[str, str]],
    map_text: str | None = None,
    deferred_data: dict | None = None,
    candidate_text: str | None = None,
    frozen_ids: list[str] | None = None,
) -> list[dict]:
    """行级机核 + 横向对账;各数据面无注入时读真树。"""
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
        decision = (r.get("裁决") or "").strip()
        row_ok = True
        detail_parts: list[str] = []
        if decision not in ALLOWED:
            row_ok = False
            detail_parts.append(f"非法裁决 {decision!r}")
        # 零空行:除 ID 外九列全必填(承接锚全行必填,defer 行再加 G12+ 字面)
        for k in HEADERS[1:]:
            if cell_empty(r.get(k, "")):
                row_ok = False
                detail_parts.append(f"空单元格 {k}")
        anchor = r.get("承接锚") or ""
        if "重判" not in anchor or "兜底" not in anchor:
            row_ok = False
            detail_parts.append("承接锚缺「重判条件/兜底」字面")
        if decision == "defer-to-G12+" and "G12+" not in anchor:
            row_ok = False
            detail_parts.append("defer 缺 G12+ 重评窗字面")
        if decision == "go":
            if "evidence/" not in (r.get("依据/证据路径") or ""):
                row_ok = False
                detail_parts.append("go 缺 evidence 路径")
        elif decision == "no-go":
            ref = r.get("依据/证据路径") or ""
            anchors = ("RD-", "deferred", "CONTRACT", "RFC-", "矩阵", "CAPABILITY", "CANDIDATE", "PLAN", "MAP")
            if not any(a in ref for a in anchors):
                row_ok = False
                detail_parts.append("no-go 缺 RD/矩阵/契约/计划/MAP 锚")
        results.append(
            {
                "id": f"row_{rid}",
                "status": "PASS" if row_ok else "FAIL",
                "detail": "; ".join(detail_parts) if detail_parts else f"{decision}",
            }
        )

    # 横向机核①:与 G11_ACCEPTANCE_MAP 14 key(13 P0 + 1 已 go P1)互斥
    mt = map_text if map_text is not None else (
        ACCEPTANCE_MAP.read_text(encoding="utf-8") if ACCEPTANCE_MAP.is_file() else ""
    )
    go_p0 = {f"M{m}" for m in re.findall(r"g11\.p0\.m(\d{3})\.", mt)}
    go_p1 = {f"M{m}" for m in re.findall(r"g11\.p1\.m(\d{3})\.", mt)}
    hit = sorted(set(ids) & (go_p0 | go_p1))
    mutex_ok = (
        not hit and len(go_p0) == 13 and len(go_p1) == 1
    )
    results.append(
        {
            "id": "acceptance_map_mutex",
            "status": "PASS" if mutex_ok else "FAIL",
            "detail": f"MAP 实解 P0={len(go_p0)} P1={len(go_p1)}(expect 13/1);P2 表命中已 go 裸 token: {hit or '无'}",
        }
    )

    # 横向机核②:deferred.json history 对账(G11.6 P2 登记新增条数)
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

    # 横向机核③:G11.1 候选决策表对账(15 行 G10 defer 承接 + G11-N3 ==
    # CANDIDATE §2/§4 行集 defer-to-G12+ 字面逐字承接)
    ct = candidate_text if candidate_text is not None else (
        CANDIDATE.read_text(encoding="utf-8") if CANDIDATE.is_file() else ""
    )
    p2_map = {r.get("ID", ""): r for r in rows}
    cand_ok = True
    cand_parts: list[str] = []
    for rid in CANDIDATE_DEFER_IDS:
        if rid not in ct:
            cand_ok = False
            cand_parts.append(f"CANDIDATE 缺行 {rid}")
            continue
        pr = p2_map.get(rid)
        if pr is None:
            cand_ok = False
            cand_parts.append(f"P2 表缺承接行 {rid}")
            continue
        verdict = (pr.get("裁决") or "").strip()
        if not verdict.startswith("defer-to-G12+"):
            cand_ok = False
            cand_parts.append(f"{rid} P2 裁决={verdict!r} ≠ defer-to-G12+ 承接字面")
    if not ct:
        cand_ok = False
        cand_parts.append("G11_CANDIDATE_DECISIONS.md 未落盘或不可读")
    cand_parts.append(f"承接行对账 n={sum(1 for rid in CANDIDATE_DEFER_IDS if rid in ct)}/16")
    results.append(
        {
            "id": "candidate_decisions_reconcile",
            "status": "PASS" if cand_ok else "FAIL",
            "detail": "; ".join(cand_parts),
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
        # 诚实红:表未落盘不是绿
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}(G11.6 决策表未落盘;诚实红,不假绿)"}]
    rows = parse_table(p.read_text(encoding="utf-8"))
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
        print(f"[g11_p2_decisions] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G11_CONTRACT G-G11-8;CI_GATES §5 v1.7;G11_P2_DECISIONS.md v1.0;G11_CANDIDATE_DECISIONS v1.0;G11_ACCEPTANCE_MAP §1/§2;registry/deferred.json(G11.6 P2 行);G11_CONTRACT §8.2~§8.5b",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G11.6 P2/留档/未触发分项穷举决策(28 行闭集:go 3 closed-go 留痕 + no-go 6 + defer-to-G12+ 19,strategic_override 0);defer 必有承接锚(重判条件+兜底+G12+ 重评窗);与 MAP 14 key 互斥;deferred.json history 对账(RD-039 +1/RD-040 +3,M99-clipmap=G11.4 承接兑现完结登记,零新 RD);G11.1 候选决策表对账(15 行 G10 defer 承接 + G11-N3 逐字承接);no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=True,
    )
    return code


def _synth_row(rid: str) -> str:
    if rid in GO_IDS:
        decision = "go"
        anchor = "重判条件 = G12+ 若口径面再发现过严/过松面时按只追加程序重判;兜底 = 现机核维持,门绿 0-byte"
        ref = "evidence/g11_fixture_20260816T000000Z.json"
    elif rid in NO_GO_IDS:
        decision = "no-go"
        anchor = "重判条件 = G12+ 触发条件齐备时按只追加程序重判;兜底 = 既有面维持"
        ref = "registry/deferred.json RD-039 / G11_CANDIDATE_DECISIONS / G11_CONTRACT"
    else:
        decision = "defer-to-G12+"
        anchor = "重判条件 = G12+ 重评窗触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持"
        ref = "registry/deferred.json RD-039 / G11_CANDIDATE_DECISIONS §2"
    return f"| {rid} | 分项 | G11.1 | 触发条件字面 | {decision} | 理由 | {ref} | {anchor} | 留痕位置 | open |\n"


def run_selftest() -> int:
    failures = 0
    good_header = (
        "| " + " | ".join(HEADERS) + " |\n"
        "|" + "---|" * len(HEADERS) + "\n"
    )
    full = good_header + "".join(_synth_row(i) for i in FROZEN_IDS)

    # 正样本 1:真表(已落盘)必须绿
    code, results = run_check(None)
    if not DECISIONS.is_file():
        if code == 0:
            print("[selftest] FAIL: 表未落盘仍绿(假绿)", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 表未落盘 → 诚实红(起始正确结论)")
    else:
        if code != 0:
            print("[selftest] FAIL: 决策表已落盘但核验未绿", file=sys.stderr)
            for r in results:
                if r["status"] != "PASS":
                    print(f"  {r}", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 真表 28 行绿")

    with tempfile.TemporaryDirectory(prefix="g11_p2_selftest_") as td:
        # 正样本 2:合成全表(真树 MAP/deferred/CANDIDATE 对账)必须绿
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

        # 负样本 1:缺行 → 必须红
        lines = [ln for ln in full.splitlines() if not ln.strip().startswith("| M127 |")]
        p2 = Path(td) / "bad.md"
        p2.write_text("\n".join(lines) + "\n", encoding="utf-8")
        code, _ = run_check(p2)
        if code == 0:
            print("[selftest] FAIL: 缺行仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 缺行→红")

        # 负样本 2:defer 行承接锚缺 G12+ → 必须红
        bad_defer = full.replace(
            "重判条件 = G12+ 重评窗触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持",
            "重判条件 = 触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持",
        )
        p3 = Path(td) / "baddefer.md"
        p3.write_text(bad_defer, encoding="utf-8")
        code, _ = run_check(p3)
        if code == 0:
            print("[selftest] FAIL: defer 缺 G12+ 承接锚仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: defer 缺 G12+ 承接锚→红")

        # 负样本 3:非法裁决枚举 → 必须红
        bad_enum = full.replace("| no-go |", "| maybe |", 1)
        p4 = Path(td) / "badenum.md"
        p4.write_text(bad_enum, encoding="utf-8")
        code, _ = run_check(p4)
        if code == 0:
            print("[selftest] FAIL: 非法裁决枚举仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 非法裁决枚举→红")

        # 负样本 4:互斥违例(已 go P0 裸 token M147 入表)→ 必须红
        bad_mutex = full.replace("| M61 |", "| M147 |")
        p5 = Path(td) / "badmutex.md"
        p5.write_text(bad_mutex, encoding="utf-8")
        code, _ = run_check(p5)
        if code == 0:
            print("[selftest] FAIL: 已 go P0 裸 token 入表仍绿(互斥失效)", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 互斥违例→红")

        # 负样本 5:空单元格(裁决理由空)→ 必须红
        bad_empty = full.replace("| 理由 |", "|  |", 1)
        p6 = Path(td) / "badempty.md"
        p6.write_text(bad_empty, encoding="utf-8")
        code, _ = run_check(p6)
        if code == 0:
            print("[selftest] FAIL: 空单元格仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 空单元格→红")

        # 负样本 6:deferred.json 对账失配(注入缺 G11.6 P2 行的 deferred 数据)→ 必须红
        real = wel.load_json(DEFERRED)
        stripped = {
            **real,
            "entries": [
                {**e, "history": [h for h in e.get("history", []) if HISTORY_MARKER not in (h.get("event") or "")]}
                for e in real.get("entries", [])
            ],
        }
        code, _ = run_check(p, deferred_data=stripped)
        if code == 0:
            print("[selftest] FAIL: deferred history 缺登记仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: deferred history 缺登记→红")

        # 负样本 7:G11.1 候选决策表对账失配(P2 行 M61 改 go)→ 必须红
        bad_cand = full.replace(
            "| M61 | 分项 | G11.1 | 触发条件字面 | defer-to-G12+ |",
            "| M61 | 分项 | G11.1 | 触发条件字面 | go |",
            1,
        )
        p7 = Path(td) / "badcand.md"
        p7.write_text(bad_cand, encoding="utf-8")
        code, _ = run_check(p7)
        if code == 0:
            print("[selftest] FAIL: 候选决策表对账失配仍绿", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 候选决策表对账失配→红")

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
