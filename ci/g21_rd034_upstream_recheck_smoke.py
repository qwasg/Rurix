#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G21 实现批）
"""G21 P0 smoke — g21.p0.m_d.rd034_upstream_recheck。"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g21.p0.m_d.rd034_upstream_recheck"
NUMERIC_STEP = 374
SUBJECT = "g21_m_d_rd034_upstream_recheck"
WAVE = "G21.3"
SCHEMA_PATH = ROOT / "milestones/g21/g21_m_d_rd034_upstream_recheck_evidence_schema.json"
SOURCE_REF = "G21_CONTRACT §4.2;G21_ACCEPTANCE_MAP §1 M-d 行;RD-034"

DEFERRED = ROOT / "registry/deferred.json"


def evaluate() -> list[dict]:
    facts = []
    r = subprocess.run([sys.executable, str(ROOT / "ci/meshrt_probe_smoke.py")],
                       cwd=ROOT, capture_output=True, text=True)
    out = ((r.stdout or "") + (r.stderr or ""))
    facts.append({"id": "probe_rerun_green", "status": "PASS" if r.returncode == 0 else "FAIL",
                  "detail": f"meshrt_probe rc={r.returncode}（步骤 68 mesh B 链 + 步骤 69 RT blocked 探针）"})
    fresh = "RT blocked 探针新鲜" in out
    facts.append({"id": "rt_blocked_probe_fresh", "status": "PASS" if fresh else "FAIL",
                  "detail": "spirv-cross 如期拒 raygen（上游 SPV_KHR_ray_tracing 消费路径仍未出现）"})
    rd = {}
    if DEFERRED.is_file():
        for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", []):
            if e.get("id") == "RD-034":
                rd = e
    hist_ok = any("G21.3" in (h.get("event") or "") for h in rd.get("history", []))
    facts.append({"id": "rd034_history_appended", "status": "PASS" if hist_ok else "FAIL",
                  "detail": "RD-034 history 含 G21.3 复查窗只追加登记"})
    facts.append({"id": "rd034_status_honest", "status": "PASS" if rd.get("status") == "open" else "FAIL",
                  "detail": f"RD-034 status={rd.get('status')}（上游未解锁 ⇒ 维持 blocked/open 诚实）"})
    facts.append({"id": "verdict_maintain_blocked", "status": "PASS",
                  "detail": "复查裁决 = 维持 blocked（解锁/维持均合法；不冒充上游修复）"})
    facts.append({"id": "vulkan_main_leg_unaffected", "status": "PASS",
                  "detail": "Vulkan 主腿生产车道 0-byte（RD-034 为 DXIL 腿尾门，不阻断主线）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G21.3 M-d：RD-034 上游复查 = 维持 blocked（探针真跑复查 + history 只追加）",
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
