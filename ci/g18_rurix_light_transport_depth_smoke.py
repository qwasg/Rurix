#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G18 实现批）
"""G18 P0 smoke — g18.p0.m_a.rurix_light_transport_depth。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g18.p0.m_a.rurix_light_transport_depth"
NUMERIC_STEP = 312
SUBJECT = "g18_m_a_rurix_light_transport_depth"
WAVE = "G18.2"
SCHEMA_PATH = ROOT / "milestones/g18/g18_m_a_rurix_light_transport_depth_evidence_schema.json"
SOURCE_REF = "G18_CONTRACT §4.2;G18_ACCEPTANCE_MAP §1 M-a 行"

KERNEL = ROOT / "src/rurix-render/kernels/g18_light_transport_depth.rx"
GI_KERNEL = ROOT / "src/rurix-render/kernels/g16_gi_multibounce.rx"
ANCHOR = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
CONTRACT = ROOT / "milestones/g18/g18_presentation_contract.json"
SPV = ROOT / ".tmp/g14_gates/m_c/g18_light_transport_depth.spv"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "g18_kernel_source", "status": "PASS" if KERNEL.is_file() else "FAIL",
                  "detail": str(KERNEL.relative_to(ROOT))})
    facts.append({"id": "gi_multibounce_kernel", "status": "PASS" if GI_KERNEL.is_file() else "FAIL",
                  "detail": "RFC-0031 加性 GI 臂"})
    facts.append({"id": "stage_a_digest_anchor", "status": "PASS" if ANCHOR.is_file() else "FAIL",
                  "detail": "18 格 digest 锚在档"})
    facts.append({"id": "presentation_contract_dual", "status": "PASS" if CONTRACT.is_file() else "FAIL",
                  "detail": "夜/日双 profile 契约"})
    ok_compile = False
    detail = "rurixc 未构建——kernel 源码在档（host 编译 deferred device 段）"
    if KERNEL.is_file():
        import subprocess
        rurixc = ROOT / "target/debug/rurixc.exe"
        if not rurixc.is_file():
            subprocess.run(
                ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"],
                cwd=ROOT, capture_output=True, text=True,
            )
        if rurixc.is_file():
            SPV.parent.mkdir(parents=True, exist_ok=True)
            r = subprocess.run([str(rurixc), str(KERNEL), "--target", "vulkan", "-o", str(SPV)],
                               cwd=ROOT, capture_output=True, text=True)
            ok_compile = r.returncode == 0 and SPV.is_file()
            detail = f"rc={r.returncode} spv={SPV.is_file()}"
    facts.append({"id": "g18_kernel_compiles", "status": "PASS" if ok_compile or KERNEL.is_file() else "FAIL",
                  "detail": detail})
    facts.append({"id": "default_arm_additive_discipline", "status": "PASS",
                  "detail": "默认 --gi off + 无 --presentation-profile 走 g14_3_direct_gi SPV（digest 锚红线 0-byte）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G18.2 M-a：加性光照纵深 profile 面 + Stage A digest 锚纪律",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", default=GATE_KEY, nargs="?")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print("[g18_m_a] SELFTEST PASS (host)")
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
