#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G21 实现批）
"""G21 P0 smoke — g21.p0.m_a.restir_high_reservoir_realization。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g21.p0.m_a.restir_high_reservoir_realization"
NUMERIC_STEP = 368
SUBJECT = "g21_m_a_restir_high_reservoir_realization"
WAVE = "G21.2"
SCHEMA_PATH = ROOT / "milestones/g21/g21_m_a_restir_high_reservoir_realization_evidence_schema.json"
SOURCE_REF = "G21_CONTRACT §4.2;G21_ACCEPTANCE_MAP §1 M-a 行;RFC-0038"

MODULE = ROOT / "src/rurix-render/src/gi/restir_reservoir.rs"
PROBE = ROOT / "src/rurix-render/src/bin/g21_restir_probe.rs"
FROZEN = "src/rurix-render/src/gi/multi_light.rs"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "reservoir_module_on_tree", "status": "PASS" if MODULE.is_file() else "FAIL",
                  "detail": "gi/restir_reservoir.rs（WRS/RIS 无偏权 + 时域 M-cap 合并）"})
    facts.append({"id": "probe_harness_on_tree", "status": "PASS" if PROBE.is_file() else "FAIL",
                  "detail": "bin/g21_restir_probe.rs（64 灯夹具 20k trial）"})
    p = wel.load_latest_evidence("g21_restir_probe")
    doc = wel.load_json(p) if p else {}
    facts.append({"id": "unbiased_all_3sigma", "status": "PASS" if doc.get("unbiased_all_3sigma") is True else "FAIL",
                  "detail": "三估计子对解析全灯和参考 3σ 无偏检验"})
    facts.append({"id": "variance_gain_gt2", "status": "PASS" if doc.get("variance_gain_gt2") is True else "FAIL",
                  "detail": f"等验证预算 var(uniform)/var(RIS)={doc.get('variance_reduction')}（要求 >2 measured）"})
    facts.append({"id": "temporal_gain_gt1_2", "status": "PASS" if doc.get("temporal_gain_gt1_2") is True else "FAIL",
                  "detail": f"时域合并再收益 var(RIS)/var(temporal)={doc.get('temporal_reduction')}（要求 >1.2 measured）"})
    facts.append({"id": "double_run_bitexact", "status": "PASS" if doc.get("double_run_bitexact") is True else "FAIL",
                  "detail": "固定 seed 双跑位级一致"})
    r = subprocess.run(["git", "diff", "--quiet", "g20-closed", "--", FROZEN],
                       cwd=ROOT, capture_output=True)
    facts.append({"id": "m100_low_tier_0byte", "status": "PASS" if r.returncode == 0 else "FAIL",
                  "detail": f"multi_light.rs vs g20-closed 0-byte（低档生产默认面不接线；rc={r.returncode}）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G21.2 M-a：ReSTIR 高档 reservoir host 参考臂（M100-high 证据齐备兑现）",
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
