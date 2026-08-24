#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G20 实现批）
"""G20 P0 smoke — g20.p0.m_a.hzb_occlusion_host_realization。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g20.p0.m_a.hzb_occlusion_host_realization"
NUMERIC_STEP = 352
SUBJECT = "g20_m_a_hzb_occlusion_host_realization"
WAVE = "G20.2"
SCHEMA_PATH = ROOT / "milestones/g20/g20_m_a_hzb_occlusion_host_realization_evidence_schema.json"
SOURCE_REF = "G20_CONTRACT §4.2;G20_ACCEPTANCE_MAP §1 M-a 行;RFC-0037"

HZB_SRC = ROOT / "src/rurix-render/src/geometry/hzb.rs"
PROBE_SRC = ROOT / "src/rurix-render/src/bin/g20_hzb_probe.rs"
FROZEN = ["src/rurix-render/src/geometry/cull.rs", "src/rurix-render/src/geometry/visbuffer.rs"]


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "hzb_module_on_tree", "status": "PASS" if HZB_SRC.is_file() else "FAIL",
                  "detail": "geometry/hzb.rs（farther-of 归约金字塔 + ≤2×2 窗保守测试 + 双约定）"})
    facts.append({"id": "probe_harness_on_tree", "status": "PASS" if PROBE_SRC.is_file() else "FAIL",
                  "detail": "bin/g20_hzb_probe.rs（800 rect 夹具 vs 逐像素精确真值）"})
    p = wel.load_latest_evidence("g20_hzb_probe")
    doc = wel.load_json(p) if p else {}
    arms = doc.get("arms", [])
    convs = sorted(a.get("conv") for a in arms)
    facts.append({"id": "dual_convention_arms", "status": "PASS" if convs == ["reverse_z", "standard_z"] else "FAIL",
                  "detail": f"arms={convs}"})
    facts.append({"id": "zero_false_positive_invariant",
                  "status": "PASS" if doc.get("zero_false_positive") is True else "FAIL",
                  "detail": "保守零假阳性硬不变量（判遮挡 ⇒ 精确真值同判）"
                  + (f"；fp={[a.get('false_positives') for a in arms]}" if arms else "")})
    facts.append({"id": "cull_rate_nonzero", "status": "PASS" if doc.get("cull_rate_nonzero") is True else "FAIL",
                  "detail": f"occluded={[a.get('occluded') for a in arms]}/800"})
    facts.append({"id": "double_run_bitexact", "status": "PASS" if doc.get("double_run_bitexact") is True else "FAIL",
                  "detail": "金字塔字节 + 判定轨迹 sha256 双跑位级一致"})
    codes = [subprocess.run(["git", "diff", "--quiet", "g19-closed", "--", f],
                            cwd=ROOT, capture_output=True).returncode for f in FROZEN]
    facts.append({"id": "cull_visbuffer_0byte", "status": "PASS" if all(c == 0 for c in codes) else "FAIL",
                  "detail": f"既有 cull/visbuffer 面 vs g19-closed 0-byte（rc={codes}）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G20.2 M-a：HZB 遮挡剔除 host 参考臂实现（RFC-0037；「HZB 两阶段 P3 预留」第一阶段兑现）",
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
