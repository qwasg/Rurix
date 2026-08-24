#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Claude Fable 5（G17.7b close-out 终审波）
"""G17.7b close-out 终审门（g17.wave.7b.closeout，步骤 308；G17_CONTRACT G-G17-9）。

八 facts（DoD 逐字）：
① five_p0_evidence_green（M-a~M-e 最新 evidence host_section_pass 全真）
② p2_exhaustive_zero_empty（P2 穷举门最新 evidence PASS）
③ final_verdict_chain_complete（终判 ratio 证据链完整——达标 18/18 或维持未达标
  如实登记，二者均合法收口）
④ rfc_0032_terminal_state_archived（RFC-0032 终态留档 approved/no-go/defer 均可）
⑤ old_gates_no_regression（M-e evidence PASS——旧门零降级）
⑥ rd_eight_open（RD 八条条目级 status 全 open 维持）
⑦ soak_ge_1800_zero_fail（soak evidence PASS + budget_eval --strict 零 estimated）
⑧ closeout_ready（前七 facts 全绿 ⇒ VERDICT=READY；任一红 ⇒ BLOCKED 不充绿）

READY 后 status active→closed 独立洁净 commit + tag g17-closed（本门只判不翻）。

用法：
  py -3 ci/g17_closeout_check.py --gate g17.wave.7b.closeout
  py -3 ci/g17_closeout_check.py --verify-latest
  py -3 ci/g17_closeout_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
from g11_wave_exit_lib import DEFERRED_PATH  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g17.wave.7b.closeout"
NUMERIC_STEP = 308  # post-interlock 实测顺位领取
SUBJECT = "g17_wave7b_closeout"
WAVE = "G17.7b"
SOURCE_REF = "G17_CONTRACT G-G17-9;G17_PLAN §2 阶段⑧;registry/deferred.json"
SCHEMA_PATH = ROOT / "milestones/g17/g17_wave7b_closeout_evidence_schema.json"
RFC_PATH = ROOT / "rfcs/0032-d3d12-host-ngx-lane.md"
P0_SUBJECTS = [
    ("m_a", "g17_m_a_dual_end_retest_warm_recalib"),
    ("m_b", "g17_m_b_ngx_evolution_alignment"),
    ("m_c", "g17_m_c_d3d12_host_lane_disposition"),
    ("m_d", "g17_m_d_t100_final_verdict"),
    ("m_e", "g17_m_e_closed_gate_no_regression"),
]
RD_EIGHT = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044", "RD-045"]


def fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _latest_green(subject: str) -> tuple[bool, str, dict]:
    p = wel.load_latest_evidence(subject)
    if p is None:
        return False, f"{subject}: missing", {}
    doc = wel.load_json(p)
    ok = doc.get("host_section_pass") is True and all(
        f.get("status") == "PASS" for f in doc.get("extra_facts", [])
    )
    return ok, f"{p.name} host={doc.get('host_section_pass')}", doc


def evaluate() -> tuple[list[dict], str]:
    facts: list[dict] = []
    # ①
    p0_bad: list[str] = []
    md_doc: dict = {}
    mc_doc: dict = {}
    for short, subject in P0_SUBJECTS:
        ok, detail, doc = _latest_green(subject)
        if not ok:
            p0_bad.append(f"{short}:{detail}")
        if short == "m_d":
            md_doc = doc
        if short == "m_c":
            mc_doc = doc
    facts.append(fact("five_p0_evidence_green", not p0_bad,
                      "M-a~M-e 五 P0 最新 evidence 全绿" if not p0_bad else "; ".join(p0_bad[:3])))
    # ②
    ok2, d2, _ = _latest_green("g17_p2_decisions_check")
    facts.append(fact("p2_exhaustive_zero_empty", ok2, d2))
    # ③
    vfact = next((f for f in md_doc.get("extra_facts", [])
                  if f.get("id") == "verdict_two_state_honest"), {})
    rfact = next((f for f in md_doc.get("extra_facts", [])
                  if f.get("id") == "ratio_from_evidence"), {})
    ok3 = vfact.get("status") == "PASS" and rfact.get("status") == "PASS"
    facts.append(fact("final_verdict_chain_complete", ok3,
                      f"终判两态如实登记 + ratio 证据链：{vfact.get('detail', '缺')[:160]}"))
    # ④
    rfc_ok = RFC_PATH.is_file() and "Agent Approved" in RFC_PATH.read_text(encoding="utf-8")
    disp = ""
    if mc_doc:
        m = re.search(r"terminal_disposition = (\S+)", mc_doc.get("notes", ""))
        disp = m.group(1) if m else ""
    ok4 = rfc_ok and disp in ("implement", "no-go", "defer")
    facts.append(fact("rfc_0032_terminal_state_archived", ok4,
                      f"RFC-0032 Approved 在树 + M-c 终态 = {disp!r}（approved-implement/no-go/defer 均合法终态）"))
    # ⑤
    ok5, d5, _ = _latest_green("g17_m_e_closed_gate_no_regression")
    facts.append(fact("old_gates_no_regression", ok5, d5))
    # ⑥
    dd = wel.load_json(DEFERRED_PATH) if DEFERRED_PATH.is_file() else {"entries": []}
    status_map = {e.get("id"): e.get("status") for e in dd.get("entries") or []}
    rd_bad = [rd for rd in RD_EIGHT if status_map.get(rd) != "open"]
    facts.append(fact("rd_eight_open", not rd_bad,
                      f"RD 八条 status: {[(r, status_map.get(r)) for r in RD_EIGHT]}"))
    # ⑦
    ok7a, d7a, soak_doc = _latest_green("g17_stabilization_soak")
    wall = next((f.get("detail", "") for f in soak_doc.get("extra_facts", [])
                 if f.get("id") == "soak_wall_clock_ge_1800"), "")
    try:
        r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"],
                           cwd=ROOT, capture_output=True, text=True, check=False)
        ok7b = r.returncode == 0
        tail = (r.stdout or r.stderr).strip().splitlines()[-1:] or [""]
    except OSError as e:
        ok7b, tail = False, [str(e)]
    facts.append(fact("soak_ge_1800_zero_fail", ok7a and ok7b,
                      f"soak {d7a}（{wall}）+ budget --strict {tail[0][:80]}"))
    # ⑧
    ready = all(f["status"] == "PASS" for f in facts)
    verdict = "READY" if ready else "BLOCKED"
    facts.append(fact("closeout_ready", ready,
                      f"VERDICT={verdict}（前七 facts {'全绿' if ready else '有红——BLOCKED 不充绿'}；"
                      f"READY 后 status active→closed 独立洁净 commit + tag g17-closed）"))
    return facts, verdict


def run_gate() -> int:
    facts, verdict = evaluate()
    overall = all(f["status"] == "PASS" for f in facts)
    for f in facts:
        print(f"  FACT  {f['status']:4}  {f['id']}  ({f['detail'][:140]})")
    print(f"[g17_closeout] VERDICT = {verdict}")
    if not SCHEMA_PATH.is_file():
        print(f"[g17_closeout] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes=f"G17.7b close-out 终审八 facts：VERDICT={verdict}"
              "（五 P0 全绿 + P2 穷举零空行 + 终判 ratio 证据链完整〔两态均合法收口〕+ "
              "RFC-0032 终态留档 + 旧门零降级 + RD 八条 open + soak ≥1800s 零失败 + "
              "budget --strict 零 estimated；READY 后 status flip 独立洁净 commit + tag g17-closed）",
        host_section_pass=overall,
    )
    return 0 if (overall and code == 0) else 1


def verify_latest() -> int:
    p = wel.load_latest_evidence(SUBJECT)
    if p is None:
        print(f"[g17_closeout] verify-latest FAIL: 无 {SUBJECT} evidence", file=sys.stderr)
        return 1
    doc = wel.load_json(p)
    ok = doc.get("host_section_pass") is True and all(
        f.get("status") == "PASS" for f in doc.get("extra_facts", [])
    )
    print(f"[g17_closeout] verify-latest {'PASS' if ok else 'FAIL'}: {p.name}")
    return 0 if ok else 1


def run_selftest() -> int:
    ok = NUMERIC_STEP == 308 and SCHEMA_PATH.is_file() and len(P0_SUBJECTS) == 5
    print(f"  {'CLOSURE ok' if ok else 'CLOSURE FAIL'} — 步骤 308 + schema 在位 + 五 P0 闭集")
    print(f"[g17_closeout] SELFTEST {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return verify_latest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
