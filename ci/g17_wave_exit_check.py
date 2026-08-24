#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Claude Fable 5（G17.2 M-a 双端复测与暖态重标定波）
"""G17 波聚合门（参数化单脚本多 gate key——MAP 纪律：脚本可复用，workflow 按
--gate <symbolic-key> 独立调用、独立产 evidence、独立给结论）。

g17.wave.{2..6}.exit：只读汇总，不重跑子门、不代绿、不遮蔽子断言。四 facts：
① 对应 M 门最新 evidence host_section_pass + extra_facts 全 PASS（红即红不遮蔽）；
② 守卫套件绿（check_structure / check_schemas / check_number_ledger 子进程实测）；
③ budget_eval 全绿；
④ 聚合只读声明（本门零重跑零改写）。

用法：
  py -3 ci/g17_wave_exit_check.py --gate g17.wave.2.exit
  py -3 ci/g17_wave_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
SCHEMA_PATH = ROOT / "milestones/g17/g17_wave_exit_evidence_schema.json"

# gate key → (波次, 对应 M 门 evidence subject 前缀, 数字步骤)。
# 步骤 = post-interlock 实测顺位领取（296=M-a 后：297 wave2；298=M-b 后：299 wave3；
# 300=M-c 后：301 wave4；302=M-d 后：303 wave5；304=M-e 后：305 wave6）。
WAVES: dict[str, tuple[str, str, int]] = {
    "g17.wave.2.exit": ("G17.2", "g17_m_a_dual_end_retest_warm_recalib", 297),
    "g17.wave.3.exit": ("G17.3", "g17_m_b_ngx_evolution_alignment", 299),
    "g17.wave.4.exit": ("G17.4", "g17_m_c_d3d12_host_lane_disposition", 301),
    "g17.wave.5.exit": ("G17.5", "g17_m_d_t100_final_verdict", 303),
    "g17.wave.6.exit": ("G17.6", "g17_m_e_closed_gate_no_regression", 305),
}
GUARDS = ("check_structure.py", "check_schemas.py", "check_number_ledger.py")


def evaluate(gate_key: str, m_doc: dict | None, guard_codes: list[int], budget_code: int) -> list[dict]:
    """四 facts（可注入）。"""
    wave, subject, _ = WAVES[gate_key]
    facts: list[dict] = []
    if m_doc is None:
        facts.append({"id": "required_m_gate_latest_pass", "status": "FAIL",
                      "detail": f"{subject} 无 evidence（M 门未跑；诚实红不假绿）"})
    else:
        m_ok = m_doc.get("host_section_pass") is True and all(
            f.get("status") == "PASS" for f in m_doc.get("extra_facts", [])
        )
        bad = [f["id"] for f in m_doc.get("extra_facts", []) if f.get("status") != "PASS"]
        facts.append({
            "id": "required_m_gate_latest_pass",
            "status": "PASS" if m_ok else "FAIL",
            "detail": f"{subject} 最新 evidence host_section_pass={m_doc.get('host_section_pass')}"
                      + (f"; 红 facts: {bad[:4]}" if bad else "; extra_facts 全 PASS"),
        })
    guards_ok = all(c == 0 for c in guard_codes)
    facts.append({
        "id": "guards_pass",
        "status": "PASS" if guards_ok else "FAIL",
        "detail": f"守卫套件 exit codes = {dict(zip(GUARDS, guard_codes))}",
    })
    facts.append({
        "id": "budget_eval_pass",
        "status": "PASS" if budget_code == 0 else "FAIL",
        "detail": f"budget_eval exit = {budget_code}",
    })
    facts.append({
        "id": "aggregate_read_only",
        "status": "PASS",
        "detail": "聚合只读：零重跑子门、零改写、红树下不充绿（子断言红即本门红）",
    })
    return facts


def run_gate(gate_key: str) -> int:
    wave, subject, step = WAVES[gate_key]
    p = wel.load_latest_evidence(subject)
    m_doc = wel.load_json(p) if p else None
    guard_codes = [
        subprocess.run([sys.executable, f"ci/{g}"], cwd=ROOT, capture_output=True).returncode
        for g in GUARDS
    ]
    budget_code = subprocess.run(
        [sys.executable, "ci/budget_eval.py"], cwd=ROOT, capture_output=True
    ).returncode
    facts = evaluate(gate_key, m_doc, guard_codes, budget_code)
    overall = all(f["status"] == "PASS" for f in facts)
    if not SCHEMA_PATH.is_file():
        print(f"[g17_wave_exit] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    n = gate_key.split(".")[2]
    code, _ = wel.emit_wave_evidence(
        wave=wave,
        subject=f"g17_wave{n}_exit",
        symbolic_gate_key=gate_key,
        numeric_step=step,
        source_ref="G17_CONTRACT §2 波次条款;G17 CI_GATES §3;对应 M 门最新 evidence",
        required_gate_rows=[],
        extra_facts=facts,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=f"g17_wave{n}_exit",
        notes=f"G17 波聚合门 {gate_key}：只读汇总四 facts（M 门最新 evidence + 守卫套件 + "
              "budget_eval + 聚合只读声明）；不重跑不代绿不遮蔽，红树下聚合红",
        host_section_pass=overall,
    )
    verdict = "PASS" if (overall and code == 0) else "FAIL"
    print(f"[g17_wave_exit] {gate_key} VERDICT = {verdict}")
    return 0 if (overall and code == 0) else 1


def run_selftest() -> int:
    failures = 0
    good_doc = {"host_section_pass": True, "extra_facts": [{"id": "x", "status": "PASS"}]}
    cases = [
        ("M 门 evidence 缺失 → 红", None, [0, 0, 0], 0, "required_m_gate_latest_pass"),
        ("M 门红 facts 遮蔽拒绝 → 红",
         {"host_section_pass": True, "extra_facts": [{"id": "x", "status": "FAIL"}]},
         [0, 0, 0], 0, "required_m_gate_latest_pass"),
        ("守卫红 → 红", good_doc, [0, 1, 0], 0, "guards_pass"),
        ("budget 红 → 红", good_doc, [0, 0, 0], 1, "budget_eval_pass"),
    ]
    for name, doc, gc, bc, expect in cases:
        facts = evaluate("g17.wave.2.exit", doc, gc, bc)
        hit = [f for f in facts if f["id"] == expect and f["status"] == "FAIL"]
        if hit:
            print(f"  RED ok   — {name}")
        else:
            print(f"  RED MISS — {name}")
            failures += 1
    facts = evaluate("g17.wave.2.exit", good_doc, [0, 0, 0], 0)
    if all(f["status"] == "PASS" for f in facts) and len(facts) == 4:
        print("  GREEN ok — 合成正本 4 facts 全 PASS")
    else:
        print("  GREEN MISS — 合成正本未全绿")
        failures += 1
    if failures:
        print(f"[g17_wave_exit] SELFTEST FAIL ({failures})")
        return 1
    print("[g17_wave_exit] SELFTEST PASS (4 RED + 1 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=sorted(WAVES))
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate(args.gate)


if __name__ == "__main__":
    sys.exit(main())
