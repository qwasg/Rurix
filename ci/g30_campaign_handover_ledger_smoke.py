#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent(G30.3/G30.4 P0 smoke)
"""G30.3 P0 M-d:战役承接锚归档闭集 + 归档完整性机核(gate g30.p0.m_d.campaign_handover_ledger,步骤 518)。

判据法定来源:G30_CONTRACT §4.2 M-d 行 + RFC-0047(v0.2,Agent Approved)§4.1~§4.2。

消费面 = milestones/g30/g30_campaign_handover_registry.json(G31+ 唯一法定输入面;由主控在
M-a/M-b 落档后编写——本脚本只验证不生成,文件缺失时 FAIL 并明示「M-d 前置:登记表未落盘」)。

归档完整性机核分 section 钉死(F5):
- 顶层键闭集 = schema/description/campaign_period_rows/rd_eight/legacy_eleven_source/tail_six/summary;
- campaign_period_rows 行字段 = period/id/final/g31_anchor/source,期集合恰 {G26..G30} 逐期 ≥1 行,
  G30 期 G17-MD-F1 终判行归档槽位点名(final 含 18/18 或 17/18 两态之一——§4.1);
- rd_eight 恰 8 行,行字段 = id/status/g31_anchor,逐行 status 与 registry/deferred.json 实测一致;
- tail_six 恰 6 行,行字段 = id/final/g31_anchor/source(source 含 G30 evidence 文件引用字面);
- legacy_eleven_source 沿 g25 registry 字段名字面(F13),引用 g24 清册不复制;
- 上游锚 0-byte 不回写(g25 registry vs HEAD git diff --quiet)。
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g30.p0.m_d.campaign_handover_ledger"
NUMERIC_STEP = 518
SUBJECT = "g30_m_d_campaign_handover_ledger"
WAVE = "G30.3"
SCHEMA_PATH = ROOT / "milestones/g30/g30_m_d_campaign_handover_ledger_evidence_schema.json"
SOURCE_REF = "G30_CONTRACT §4.2 M-d;RFC-0047 §4.1~§4.2"

REG = ROOT / "milestones/g30/g30_campaign_handover_registry.json"
UPSTREAM_REG = "milestones/g25/g25_campaign_handover_registry.json"
LEGACY_LITERAL = "milestones/g24/g24_legacy_rd_registry.json"
DEFERRED = ROOT / "registry/deferred.json"
M_A_SUBJECT = "g30_m_a_tail_anchor_rejudgment_closure"
MISSING_MSG = ("M-d 前置:登记表未落盘(milestones/g30/g30_campaign_handover_registry.json"
               "——由主控在 M-a/M-b 落档后编写,本脚本只验证不生成)")

# 分 section 字段闭集(RFC-0047 §4.2 F5 字面钉死)
TOP_KEYS = frozenset({"schema", "description", "campaign_period_rows", "rd_eight",
                      "legacy_eleven_source", "tail_six", "summary"})
PERIOD_ROW_FIELDS = frozenset({"period", "id", "final", "g31_anchor", "source"})
PERIODS = frozenset({"G26", "G27", "G28", "G29", "G30"})
RD_ROW_FIELDS = frozenset({"id", "status", "g31_anchor"})
RD_EIGHT_IDS = frozenset({"RD-034", "RD-039", "RD-040", "RD-041",
                          "RD-042", "RD-043", "RD-044", "RD-045"})
TAIL_ROW_FIELDS = frozenset({"id", "final", "g31_anchor", "source"})
TAIL_SIX_IDS = frozenset({"M125-adopt3", "M127", "M114-strand",
                          "M118-hdr-cal", "G10-N6", "SAFE-GPU"})


def fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _load_registry() -> tuple[dict | None, str]:
    if not REG.is_file():
        return None, MISSING_MSG
    try:
        doc = wel.load_json(REG)
    except (OSError, json.JSONDecodeError) as e:
        return None, f"登记表 JSON 不可解析:{e}"
    if not isinstance(doc, dict):
        return None, "登记表顶层非 object"
    return doc, "ok"


def _rows(doc: dict, key: str) -> list:
    v = doc.get(key)
    return v if isinstance(v, list) else []


def _row_field_bad(rows: list, fields: frozenset) -> list[str]:
    bad = []
    for i, r in enumerate(rows):
        if not isinstance(r, dict):
            bad.append(f"#{i}<非 object>")
        elif set(r.keys()) != set(fields):
            bad.append(str(r.get("id", f"#{i}")))
    return bad


def evaluate() -> list[dict]:
    facts: list[dict] = []
    doc, load_detail = _load_registry()
    if doc is None:
        facts.append(fact("registry_exists_and_schema", False, load_detail))
        doc = {}
    else:
        keys = set(doc.keys())
        ok = keys == set(TOP_KEYS)
        facts.append(fact("registry_exists_and_schema", ok,
                          "存在 + JSON 可解析 + 顶层键闭集 ok:" + "/".join(sorted(TOP_KEYS)) if ok
                          else f"顶层键漂移:多={sorted(keys - set(TOP_KEYS))} 缺={sorted(set(TOP_KEYS) - keys)}"))

    rows = _rows(doc, "campaign_period_rows")
    field_bad = _row_field_bad(rows, PERIOD_ROW_FIELDS)
    periods_got = {r.get("period") for r in rows if isinstance(r, dict)}
    per_count = {p: sum(1 for r in rows if isinstance(r, dict) and r.get("period") == p)
                 for p in sorted(PERIODS)}
    ok = bool(rows) and not field_bad and periods_got == set(PERIODS)
    detail = f"{len(rows)} 行;期集合={sorted(periods_got)};逐期行数={per_count}"
    detail += (";行字段闭集 period/id/final/g31_anchor/source ok" if not field_bad
               else f";字段漂移行={field_bad}")
    facts.append(fact("period_rows_closure", ok, detail))

    slot = [r for r in rows if isinstance(r, dict)
            and r.get("period") == "G30" and r.get("id") == "G17-MD-F1"]
    final_ok = any(("18/18" in str(r.get("final", "")) or "17/18" in str(r.get("final", "")))
                   for r in slot)
    if slot and final_ok:
        detail = f"G30 期 G17-MD-F1 行在档,final={slot[0].get('final')!r}(18/18|17/18 两态之一命中)"
    elif not slot:
        detail = "G30 期无 id=G17-MD-F1 行(RFC-0047 §4.1 归档槽位点名)"
    else:
        detail = f"final={slot[0].get('final')!r} 不含 18/18|17/18 两态字面"
    facts.append(fact("period_rows_g17_md_f1", bool(slot) and final_ok, detail))

    rd_rows = _rows(doc, "rd_eight")
    field_bad = _row_field_bad(rd_rows, RD_ROW_FIELDS)
    rd_ids = {r.get("id") for r in rd_rows if isinstance(r, dict)}
    dstat: dict = {}
    if DEFERRED.is_file():
        dstat = {e.get("id"): e.get("status")
                 for e in wel.load_json(DEFERRED).get("entries", [])}
    st_bad = [f"{r.get('id')}:reg={r.get('status')!r}≠deferred={dstat.get(r.get('id'))!r}"
              for r in rd_rows if isinstance(r, dict) and r.get("status") != dstat.get(r.get("id"))]
    ok = (len(rd_rows) == 8 and not field_bad and rd_ids == set(RD_EIGHT_IDS)
          and not st_bad and bool(dstat))
    detail = f"{len(rd_rows)}/8 行;id 集合 ok={rd_ids == set(RD_EIGHT_IDS)}"
    detail += (";行字段闭集 id/status/g31_anchor ok" if not field_bad else f";字段漂移行={field_bad}")
    detail += (";逐行 status 与 registry/deferred.json 实测一致" if not st_bad and dstat
               else f";status 不一致={st_bad or 'deferred.json 缺失'}")
    facts.append(fact("rd_eight_closure", ok, detail))

    t_rows = _rows(doc, "tail_six")
    field_bad = _row_field_bad(t_rows, TAIL_ROW_FIELDS)
    t_ids = {r.get("id") for r in t_rows if isinstance(r, dict)}
    src_bad = [str(r.get("id")) for r in t_rows
               if isinstance(r, dict) and "evidence/g30_" not in str(r.get("source", ""))]
    ok = len(t_rows) == 6 and not field_bad and t_ids == set(TAIL_SIX_IDS) and not src_bad
    detail = f"{len(t_rows)}/6 行;id 集合 ok={t_ids == set(TAIL_SIX_IDS)}"
    detail += (";行字段闭集 id/final/g31_anchor/source ok" if not field_bad
               else f";字段漂移行={field_bad}")
    detail += (";逐行 source 含 evidence/g30_ 引用字面" if not src_bad
               else f";source 缺 G30 evidence 引用行={src_bad}")
    facts.append(fact("tail_six_closure", ok, detail))

    lit = str(doc.get("legacy_eleven_source", ""))
    on_tree = (ROOT / LEGACY_LITERAL).is_file()
    ok = LEGACY_LITERAL in lit and on_tree
    facts.append(fact("legacy_source_literal", ok,
                      f"字面含 {LEGACY_LITERAL}={LEGACY_LITERAL in lit};该文件在树={on_tree}"
                      "(F13 字段名沿 g25 registry;引用不复制)"))

    p = wel.load_latest_evidence(M_A_SUBJECT)
    host = wel.load_json(p).get("host_section_pass") if p else None
    facts.append(fact("m_a_linkage", p is not None and host is True,
                      f"{p.name} host={host}(tail_six 终态的上游绿件链)" if p
                      else f"missing({M_A_SUBJECT}_*.json 未落档——M-a 绿件为 tail_six 终态上游)"))

    r = subprocess.run(["git", "diff", "--quiet", "HEAD", "--", UPSTREAM_REG],
                       cwd=ROOT, capture_output=True, text=True)
    facts.append(fact("upstream_zero_byte", r.returncode == 0,
                      f"git diff --quiet HEAD -- {UPSTREAM_REG} rc={r.returncode}"
                      "(0=vs HEAD 0-byte,上游锚不回写)"))
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G30.3 M-d:战役承接锚归档闭集 + 归档完整性机核分 section 钉死"
              "(五期行 + rd_eight 8 + tail_six 6 + legacy 引用——G31+ 唯一法定输入面;RFC-0047 §4.2 F5)",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def selftest() -> int:
    bad: list[str] = []
    if len(TOP_KEYS) != 7:
        bad.append(f"顶层键闭集应恰 7 键,实为 {len(TOP_KEYS)}")
    if len(PERIODS) != 5:
        bad.append(f"期集合应恰 5 期,实为 {len(PERIODS)}")
    if len(PERIOD_ROW_FIELDS) != 5 or len(RD_ROW_FIELDS) != 3 or len(TAIL_ROW_FIELDS) != 4:
        bad.append("分 section 行字段闭集基数漂移(应 5/3/4——F5)")
    if len(RD_EIGHT_IDS) != 8:
        bad.append(f"rd_eight id 闭集应恰 8 条,实为 {len(RD_EIGHT_IDS)}")
    if len(TAIL_SIX_IDS) != 6:
        bad.append(f"tail_six id 闭集应恰 6 条,实为 {len(TAIL_SIX_IDS)}")
    if not DEFERRED.is_file():
        bad.append("registry/deferred.json 缺失(rd_eight status 对照面)")
    if not (ROOT / UPSTREAM_REG).is_file():
        bad.append(f"上游锚缺失:{UPSTREAM_REG}")
    if not SCHEMA_PATH.is_file():
        bad.append(f"schema 缺失:{SCHEMA_PATH.name}")
    # REG 存在性不入 selftest:消费面由主控落盘,缺表是 --gate 的 FAIL 面而非结构自检面。
    if bad:
        for b in bad:
            print(f"[{SUBJECT}] SELFTEST FAIL: {b}", file=sys.stderr)
        return 1
    print(f"[{SUBJECT}] SELFTEST PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
