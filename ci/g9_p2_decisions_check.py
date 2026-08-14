#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.7 P2/留档/未触发分项穷举决策门 g9.wave.7.decisions(G9_CONTRACT G-G9-9)。

核验 `milestones/g9/G9_P2_DECISIONS.md`(2026-08-14 v1.0 落盘):
冻结 33 行候选闭集全等(候选全集基数对账)、决策枚举合法、零空行(全列非空)、
承接锚「重判条件 + 兜底」字面、defer 行必含 G10+ 重评窗、go 行 evidence 义务、
no-go 行 RD/矩阵/契约锚义务;外加两横向机核——
  ① 与 G9_ACCEPTANCE_MAP 34 key(15 P0 + 19 已 go P1)互斥:P2 行 ID 不得命中
    任何已 go M## 裸 token(子项级 key 如 M114-strand/M126-rd044 不互斥);
  ② deferred.json history 对账:G9.7 P2 defer 登记恰好 RD-039 +1(M61)/
    RD-040 +3(M52/M99-clipmap/M100-high),零新 RD(max=RD-044),status 0-byte。
只读文档与 registry,不代绿实现门;同构 ci/g8_p2_decisions_check.py。

materialize:numeric_step=170(落盘前实测 CI_step.next_free=170 顺位领取);
G9.1 骨架期 FROZEN_IDS 十行(G9_PLAN §G9.7 候选行集)由本版按候选全集口径
扩为 33 行闭集(CANDIDATE 47 行实记全集未进 34 key 者 + G9.3~G9.6 新增
not-triggered/no-go 登记面去重),行 key 逐字对账。

用法:
  py -3 ci/g9_p2_decisions_check.py --gate g9.wave.7.decisions
  py -3 ci/g9_p2_decisions_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g9_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g9.wave.7.decisions"
NUMERIC_STEP = 170  # 落盘前实测 registry/number_ledger.json CI_step.next_free=170 顺位领取
SUBJECT = "g9_p2_decisions"
WAVE = "G9.7"
DECISIONS = ROOT / "milestones" / "g9" / "G9_P2_DECISIONS.md"
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_p2_decisions_evidence_schema.json"
CANDIDATE = ROOT / "milestones" / "g9" / "G9_CANDIDATE_DECISIONS.md"
ACCEPTANCE_MAP = ROOT / "milestones" / "g9" / "G9_ACCEPTANCE_MAP.md"
DEFERRED = wel.DEFERRED_PATH

# 冻结 ID 闭集(33 行)= G9_CANDIDATE_DECISIONS 实记全集未进 34 key 验收面者
# + G9.3~G9.6 期内新增 not-triggered/no-go 登记面去重——与 G9_P2_DECISIONS §1 逐字对账。
FROZEN_IDS = [
    "M61",             # RD-039 mesh shader→M109(strategic_override 维持,可选实现 defer)
    "M03-hzb",         # RD-039 HZB 两阶段遮挡剔除(no-go:measured 瓶颈证据零)
    "M44-p4",          # RD-039 cluster 流送 P4 运行时(no-go:超显存场景证据零)
    "M05-tess",        # RD-039 曲面细分位移(no-go:真实位移资产证据零)
    "M06-asm",         # RD-039 Assemblies 全功能(no-go:真实资产/residency 证据零)
    "M52",             # RD-040 SER→M108(strategic_override 维持,可选实现 defer)
    "M99-clipmap",     # RD-040 世界辐射缓存世界 clipmap 级(条件未触发 defer)
    "M100-high",       # RD-040 ReSTIR 高档(条件未触发 defer)
    "M20-smrt",        # RD-040 SMRT 软阴影完整版(no-go:无需求方/workload 举证)
    "RD040-probe",     # RD-040 自适应探针(no-go:measured 对照零)
    "M13-sdf",         # RD-040 SDF 软追踪(no-go:builder/预算证据零)
    "M53-omm",         # RD-040 OMM(no-go:真实资产/收益证据零)
    "RD040-nrd",       # RD-040 NRD 类 vendor 降噪(no-go:需求证据零)
    "M28",             # RD-041 多层材质 slab/closure IR(no-go 实现;语义面留 RFC-0022)
    "M40-svt",         # RD-041 SVT 虚拟纹理(no-go:独立判档不搭 D4 便车)
    "M26-fg",          # RD-041 帧生成 FG/MFG(no-go:独立层另判)
    "M05-mv",          # RD-041 蒙皮/WPO MV 通道资产验证(no-go:独立 MV 证据零)
    "M56-wg",          # RD-041 Work Graphs(no-go:双条件未满足,reserved_ 不接线)
    "M126-rd044",      # RD-044 Rapier 深造判档(M126 基准 verdict=maintain_no_go 维持)
    "RD044-continuum", # RD-044 Continuum 软体/MPM(no-go:观察维持,三条禁止不动)
    "RD044-fluid",     # RD-044 Fluid 生产面(no-go:观察维持)
    "M123",            # 双通道 tick 判档(no-go 维持:Jolt 单线程成本 measured 前置未满足)
    "SAFE-GPU",        # Safe GPU Operator Platform(立项裁决 defer 至 G10+)
    "M127",            # 神经变形研究子轨(登记维持,成果判档 defer G10+)
    "M59",             # async compute 第二腿(no-go:RXS-0239 字面不动)
    "M62",             # task shader 开放(no-go:RXS-0270 字面不动)
    "RD034",           # DXIL RT/mesh 腿(no-go:blocked 维持,Vulkan 主腿)
    "RD042",           # 可微物理/机器人批仿(no-go:观察维持,红线不动)
    "RD043",           # wgrapier GPU 刚体(no-go:观察维持,否决线不动)
    "M98-l4",          # M98 L4 Far Field(HLOD 接口未就绪 not-triggered → defer)
    "M114-strand",     # M114 strand 档强制精确 OIT(M120 仅测量不定档 → defer)
    "M118-hdr-cal",    # M118 HDR 设备标定层(设备未触发 not-triggered → defer)
    "M125-adopt3",     # M125 采纳臂⑦三件(verdict=maintain_5_3_default 未触发 → defer)
]
FROZEN_IDS = [s.strip() for s in FROZEN_IDS if s.strip()]
ALLOWED = frozenset({"go", "no-go", "defer-to-G10+", "strategic_override"})
DEFER_IDS = frozenset(
    {"M61", "M52", "M99-clipmap", "M100-high", "SAFE-GPU", "M127",
     "M98-l4", "M114-strand", "M118-hdr-cal", "M125-adopt3"}
)
EMPTY_RE = re.compile(r"^(TBD|TODO|待定|—|-)?$", re.I)
ID_RE = re.compile(r"^(M\d|RD\d|SAFE-GPU)")
HEADERS = [
    "ID", "分项名", "来源波次", "原触发条件字面", "裁决",
    "裁决理由", "依据/证据路径", "承接锚", "登记留痕位置", "最终状态",
]
# deferred.json history 对账期望:G9.7 P2 defer 登记恰好 RD-039 +1 / RD-040 +3。
EXPECTED_DEFER_HISTORY = {"RD-039": ["M61"], "RD-040": ["M52", "M99-clipmap", "M100-high"]}


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
) -> list[dict]:
    results: list[dict] = []
    ids = [r.get("ID", "") for r in rows]
    set_ok = set(ids) == set(FROZEN_IDS) and len(ids) == len(FROZEN_IDS)
    results.append(
        {
            "id": "set_equality_frozen",
            "status": "PASS" if set_ok else "FAIL",
            "detail": f"got n={len(ids)} unique={len(set(ids))}; expect frozen {len(FROZEN_IDS)}"
            + ("" if set_ok else f"; diff={sorted(set(FROZEN_IDS) ^ set(ids))}"),
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
        # 零空行:除 ID 外九列全必填(承接锚全行必填,defer 行再加 G10+ 字面)
        for k in HEADERS[1:]:
            if cell_empty(r.get(k, "")):
                row_ok = False
                detail_parts.append(f"空单元格 {k}")
        anchor = r.get("承接锚") or ""
        if "重判" not in anchor or "兜底" not in anchor:
            row_ok = False
            detail_parts.append("承接锚缺「重判条件/兜底」字面")
        if decision == "defer-to-G10+" and "G10+" not in anchor:
            row_ok = False
            detail_parts.append("defer 缺 G10+ 重评窗字面")
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

    # 横向机核①:与 G9_ACCEPTANCE_MAP 34 key(15 P0 + 19 已 go P1)互斥
    mt = map_text if map_text is not None else (
        ACCEPTANCE_MAP.read_text(encoding="utf-8") if ACCEPTANCE_MAP.is_file() else ""
    )
    go_p0 = {f"M{m}" for m in re.findall(r"g9\.p0\.m(\d{2,3})\.", mt)}
    go_p1 = {f"M{m}" for m in re.findall(r"g9\.p1\.m(\d{2,3})\.", mt)}
    hit = sorted(set(ids) & (go_p0 | go_p1))
    mutex_ok = (
        not hit and len(go_p0) == 15 and len(go_p1) == 19
    )
    results.append(
        {
            "id": "acceptance_map_mutex",
            "status": "PASS" if mutex_ok else "FAIL",
            "detail": f"MAP 实解 P0={len(go_p0)} P1={len(go_p1)}(expect 15/19);P2 表命中已 go 裸 token: {hit or '无'}",
        }
    )

    # 横向机核②:deferred.json history 对账(G9.7 P2 defer 登记新增条数)
    dd = deferred_data if deferred_data is not None else (
        wel.load_json(DEFERRED) if DEFERRED.is_file() else {"entries": []}
    )
    entries = dd.get("entries") or []
    reconcile_ok = True
    rec_parts: list[str] = []
    g97_holders = []
    for e in entries:
        g97 = [h for h in e.get("history", []) if "G9.7 P2" in (h.get("event") or "")]
        if g97:
            g97_holders.append((e.get("id"), g97))
    for rd, keys in EXPECTED_DEFER_HISTORY.items():
        held = dict(g97_holders).get(rd)
        if held is None or len(held) != len(keys):
            reconcile_ok = False
            rec_parts.append(f"{rd} G9.7 P2 行数={0 if held is None else len(held)} expect {len(keys)}")
            continue
        blob = "\n".join(h.get("event") or "" for h in held)
        missing = [k for k in keys if k not in blob]
        if missing:
            reconcile_ok = False
            rec_parts.append(f"{rd} 缺行 key {missing}")
    extra = sorted(r for r, _ in g97_holders if r not in EXPECTED_DEFER_HISTORY)
    if extra:
        reconcile_ok = False
        rec_parts.append(f"非期望 RD 含 G9.7 P2 行: {extra}")
    rd_nums = [int(m.group(1)) for e in entries for m in [re.match(r"RD-(\d+)$", e.get("id") or "")] if m]
    if not rd_nums or max(rd_nums) != 44:
        reconcile_ok = False
        rec_parts.append(f"RD max={max(rd_nums) if rd_nums else None} expect 44(零新 RD)")
    status_map = {e.get("id"): e.get("status") for e in entries}
    if status_map.get("RD-039") != "open" or status_map.get("RD-040") != "open":
        reconcile_ok = False
        rec_parts.append("RD-039/040 status 非 open")
    rec_parts.append(f"G9.7 P2 history: {sorted((r, len(g)) for r, g in g97_holders)}")
    results.append(
        {
            "id": "deferred_history_reconcile",
            "status": "PASS" if reconcile_ok else "FAIL",
            "detail": "; ".join(rec_parts),
        }
    )
    return results


def run_check(
    path: Path | None = None,
    map_text: str | None = None,
    deferred_data: dict | None = None,
) -> tuple[int, list[dict]]:
    p = path or DECISIONS
    if not p.is_file():
        # 诚实红:表未落盘不是绿
        return 1, [{"id": "file", "status": "FAIL", "detail": f"missing {p}(G9.7 决策表未落盘;诚实红,不假绿)"}]
    rows = parse_table(p.read_text(encoding="utf-8"))
    results = validate_rows(rows, map_text=map_text, deferred_data=deferred_data)
    ok = all(x["status"] == "PASS" for x in results)
    return (0 if ok else 1), results


def emit(results: list[dict], overall_ok: bool) -> int:
    if not SCHEMA_PATH.is_file():
        print(f"[g9_p2_decisions] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G9_PLAN §G9.7;CI_GATES §6 v1.19;G9_P2_DECISIONS.md v1.0;G9_CANDIDATE_DECISIONS v1.0~v1.6;G9_ACCEPTANCE_MAP §1~§3;registry/deferred.json v1.79",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G9.7 P2/留档/未触发分项穷举决策(33 行闭集:no-go 23 + defer-to-G10+ 10 + go 0);defer 必有承接锚(重判条件+兜底+G10+ 重评窗);与 MAP 34 key 互斥;deferred.json history 对账(RD-039 +1/RD-040 +3,零新 RD);no-go/defer 如实保持 open 不写进全绿叙述",
        host_section_pass=True,
    )
    return code


def _synth_row(rid: str) -> str:
    decision = "defer-to-G10+" if rid in DEFER_IDS else "no-go"
    if decision == "defer-to-G10+":
        anchor = "重判条件 = G10+ 重评窗触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持"
        ref = "registry/deferred.json RD-039"
    else:
        anchor = "重判条件 = 触发条件齐备时按只追加程序重判;兜底 = 既有面维持"
        ref = "registry/deferred.json RD-039 / G9_CANDIDATE_DECISIONS"
    return f"| {rid} | 分项 | G9.1 | 触发条件字面 | {decision} | 理由 | {ref} | {anchor} | 留痕位置 | open |\n"


def run_selftest() -> int:
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
            return 1
        print("[selftest] PASS: 表未落盘 → 诚实红(起始正确结论)")
    else:
        if code != 0:
            print("[selftest] FAIL: 决策表已落盘但核验未绿", file=sys.stderr)
            for r in results:
                if r["status"] != "PASS":
                    print(f"  {r}", file=sys.stderr)
            return 1
        print("[selftest] PASS: 真表 33 行绿")

    with tempfile.TemporaryDirectory(prefix="g9_p2_selftest_") as td:
        # 正样本 2:合成全表(真树 MAP/deferred 对账)必须绿
        p = Path(td) / "full.md"
        p.write_text(full, encoding="utf-8")
        code, res = run_check(p)
        if code != 0:
            print("[selftest] FAIL: 合成全表未绿", file=sys.stderr)
            for r in res:
                if r["status"] != "PASS":
                    print(f"  {r}", file=sys.stderr)
            return 1
        print("[selftest] PASS: 合成全表绿")

        # 负样本 1:缺行 → 必须红
        lines = [ln for ln in full.splitlines() if not ln.strip().startswith("| M127 |")]
        p2 = Path(td) / "bad.md"
        p2.write_text("\n".join(lines) + "\n", encoding="utf-8")
        code, _ = run_check(p2)
        if code == 0:
            print("[selftest] FAIL: 缺行仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺行→红")

        # 负样本 2:defer 行承接锚缺 G10+ → 必须红
        bad_defer = full.replace(
            "重判条件 = G10+ 重评窗触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持",
            "重判条件 = 触发条件齐备时按只追加程序重判;兜底 = 既有已验收面维持",
        )
        p3 = Path(td) / "baddefer.md"
        p3.write_text(bad_defer, encoding="utf-8")
        code, _ = run_check(p3)
        if code == 0:
            print("[selftest] FAIL: defer 缺 G10+ 承接锚仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: defer 缺 G10+ 承接锚→红")

        # 负样本 3:非法裁决枚举 → 必须红
        bad_enum = full.replace("| no-go |", "| maybe |", 1)
        p4 = Path(td) / "badenum.md"
        p4.write_text(bad_enum, encoding="utf-8")
        code, _ = run_check(p4)
        if code == 0:
            print("[selftest] FAIL: 非法裁决枚举仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 非法裁决枚举→红")

        # 负样本 4:互斥违例(已 go P1 裸 token M114 入表)→ 必须红
        bad_mutex = full.replace("| M114-strand |", "| M114 |")
        p5 = Path(td) / "badmutex.md"
        p5.write_text(bad_mutex, encoding="utf-8")
        code, _ = run_check(p5)
        if code == 0:
            print("[selftest] FAIL: 已 go P1 裸 token 入表仍绿(互斥失效)", file=sys.stderr)
            return 1
        print("[selftest] PASS: 互斥违例→红")

        # 负样本 5:空单元格(裁决理由空)→ 必须红
        bad_empty = full.replace("| 理由 |", "|  |", 1)
        p6 = Path(td) / "badempty.md"
        p6.write_text(bad_empty, encoding="utf-8")
        code, _ = run_check(p6)
        if code == 0:
            print("[selftest] FAIL: 空单元格仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 空单元格→红")

        # 负样本 6:deferred.json 对账失配(注入缺 G9.7 P2 行的 deferred 数据)→ 必须红
        real = wel.load_json(DEFERRED)
        stripped = {
            **real,
            "entries": [
                {**e, "history": [h for h in e.get("history", []) if "G9.7 P2" not in (h.get("event") or "")]}
                for e in real.get("entries", [])
            ],
        }
        code, _ = run_check(p, deferred_data=stripped)
        if code == 0:
            print("[selftest] FAIL: deferred history 缺登记仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: deferred history 缺登记→红")

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
