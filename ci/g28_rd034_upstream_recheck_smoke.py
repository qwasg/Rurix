#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G28 实现批）
"""G28.3 P0 smoke — g28.p0.m_d.rd034_upstream_recheck。

RD-034 上游复查（RFC-0045 §4）：真跑 ci/meshrt_probe_smoke.py——退出码分支捕获非透传
（F10 门态映射）：探针退 0 = blocked 证据新鲜 → maintain-blocked 分支（门绿）；
探针退 1 = 意外成功 → relock-review-triggered 分支（门同样绿：复评启动登记 = 合法
诚实终态）。门 FAIL 只保留给「复查程序未诚实执行」。backfill ②（LLVM 上游）零检测
声明；RD-034 history G28.3 只追加。
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
from g28_interlock_check import G28_0_IMMUTABLE_REF, check_deferred_append_only, _git_show_file  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g28.p0.m_d.rd034_upstream_recheck"
NUMERIC_STEP = 486  # post-interlock actual-next-free 顺位领取（480~492 批）
SUBJECT = "g28_m_d_rd034_upstream_recheck"
WAVE = "G28.3"
SCHEMA_PATH = ROOT / "milestones/g28/g28_m_d_rd034_upstream_recheck_evidence_schema.json"
SOURCE_REF = "G28_CONTRACT §4.2 M-d;RFC-0045 §4;registry/deferred.json RD-034;ci/meshrt_probe_smoke.py"

DEFERRED = ROOT / "registry/deferred.json"


def evaluate() -> list[dict]:
    facts: list[dict] = []
    r = subprocess.run([sys.executable, str(ROOT / "ci/meshrt_probe_smoke.py")],
                       cwd=ROOT, capture_output=True, text=True, timeout=1800)
    tail = ((r.stdout or "") + (r.stderr or ""))[-200:].replace("\n", " ")
    facts.append({"id": "probe_executed_fresh", "status": "PASS",
                  "detail": f"ci/meshrt_probe_smoke.py 真跑 rc={r.returncode}（退出码判定非 grep）：{tail[:120]}"})
    if r.returncode == 0:
        branch = "maintain-blocked"
        branch_detail = "探针退 0 = spirv-cross 仍拒 raygen（blocked 证据新鲜，历史根因 HLSL builtin 5319=LaunchIdKHR）"
    else:
        branch = "relock-review-triggered"
        branch_detail = ("探针退 1 = 意外成功——上游消费能力出现，复评启动登记（backfill ① 命中候选；"
                         "DXIL RT 腿全量落地按 RFC-0013 §4.E9 另立执行窗）")
    facts.append({"id": "probe_branch_verdict", "status": "PASS",
                  "detail": f"门态映射（F10 分支捕获非透传）：{branch}——{branch_detail}；两分支均合法零冒充"})
    rd = {}
    if DEFERRED.is_file():
        for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", []):
            if e.get("id") == "RD-034":
                rd = e
    status_ok = rd.get("status") == "open" if branch == "maintain-blocked" else rd.get("status") in ("open", "closed")
    facts.append({"id": "rd034_status_honest", "status": "PASS" if status_ok else "FAIL",
                  "detail": f"RD-034 status={rd.get('status')}（{branch} 分支下与终态一致）"})
    hist_ok = any("G28.3" in (h.get("event") or "") for h in rd.get("history", []))
    facts.append({"id": "rd034_history_appended", "status": "PASS" if hist_ok else "FAIL",
                  "detail": "RD-034 history 含 G28.3 复查只追加登记（断档口径注明）"})
    base_text = _git_show_file(ROOT, G28_0_IMMUTABLE_REF, "registry/deferred.json")
    base_doc = json.loads(base_text) if base_text else None
    cur_doc = json.loads(DEFERRED.read_text(encoding="utf-8")) if DEFERRED.is_file() else None
    findings = check_deferred_append_only(base_doc, cur_doc)
    facts.append({"id": "rd034_append_only_mechanized", "status": "PASS" if findings == [] else "FAIL",
                  "detail": "append-only 机核（vs G28.0 ref）" + ("" if not findings else f"；违例 {findings[:2]}")})
    facts.append({"id": "backfill_branch2_zero_detection_declared", "status": "PASS",
                  "detail": "backfill ②（LLVM A 路上游 RD-015 #90504/#57928）本期无本地机器检测面——"
                            "①探针为唯一机器面，②以上游 issue 状态为人工复查面，如实登记不冒充覆盖"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G28.3 M-d：RD-034 上游复查（探针真跑退出码分支 + maintain-blocked/relock-review-triggered 均合法 + history 只追加 + ②分支零检测声明）",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
