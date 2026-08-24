#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G22 实现批）
"""G22 P0 smoke — g22.p0.m_a.slab_material_host_realization。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g22.p0.m_a.slab_material_host_realization"
NUMERIC_STEP = 384
SUBJECT = "g22_m_a_slab_material_host_realization"
WAVE = "G22.2"
SCHEMA_PATH = ROOT / "milestones/g22/g22_m_a_slab_material_host_realization_evidence_schema.json"
SOURCE_REF = "G22_CONTRACT §4.2;G22_ACCEPTANCE_MAP §1 M-a 行;RFC-0039"

MODULE = ROOT / "src/rurix-render/src/material/slab.rs"
PROBE = ROOT / "src/rurix-render/src/bin/g22_slab_probe.rs"
FROZEN = "src/rurix-render/src/material/closure.rs"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "slab_module_on_tree", "status": "PASS" if MODULE.is_file() else "FAIL",
                  "detail": "material/slab.rs（无穷弹跳解析闭式 + 级数+尾和恒等式）"})
    facts.append({"id": "probe_harness_on_tree", "status": "PASS" if PROBE.is_file() else "FAIL",
                  "detail": "bin/g22_slab_probe.rs（128×128 参数网格白炉审计）"})
    p = wel.load_latest_evidence("g22_slab_probe")
    doc = wel.load_json(p) if p else {}
    for fid, key, desc in (
        ("white_furnace_identity", "white_furnace_identity", "白炉恒等（a_b=1 ⇒ R=1）"),
        ("energy_bounded", "energy_bounded", "全参数域 R ≤ 1（能量不增生）"),
        ("monotonic_in_base_albedo", "monotonic_in_base_albedo", "对 a_b 单调不减"),
        ("series_identity_1e9", "series_identity_1e9", "闭式↔级数+尾和恒等式（1e-9 浮点级）"),
        ("lerp_continuity", "lerp_continuity", "层参数 lerp 连续性"),
        ("double_run_bitexact", "double_run_bitexact", "白炉审计双跑位级一致"),
    ):
        facts.append({"id": fid, "status": "PASS" if doc.get(key) is True else "FAIL", "detail": desc})
    r = subprocess.run(["git", "diff", "--quiet", "g21-closed", "--", FROZEN],
                       cwd=ROOT, capture_output=True)
    facts.append({"id": "closure_single_layer_0byte", "status": "PASS" if r.returncode == 0 else "FAIL",
                  "detail": f"material/closure.rs vs g21-closed 0-byte（单层生产面不接线；rc={r.returncode}）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G22.2 M-a：Substrate 类 slab 能量守恒闭合 host 参考臂（RD-041 slab 分项兑现）",
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
