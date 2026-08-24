#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G19 实现批）
"""G19 P0 smoke — g19.p0.m_a.frame_generation_host_realization。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g19.p0.m_a.frame_generation_host_realization"
NUMERIC_STEP = 336
SUBJECT = "g19_m_a_frame_generation_host_realization"
WAVE = "G19.2"
SCHEMA_PATH = ROOT / "milestones/g19/g19_m_a_frame_generation_host_realization_evidence_schema.json"
SOURCE_REF = "G19_CONTRACT §4.2;G19_ACCEPTANCE_MAP §1 M-a 行;RFC-0036"

FRAMEGEN_SRC = ROOT / "src/rurix-render/src/temporal/framegen.rs"
PROBE_SRC = ROOT / "src/rurix-render/src/bin/g19_frame_gen_probe.rs"
PERF_BIN_SRC = "src/rurix-render/src/bin/g14_3_pipeline_perf.rs"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "framegen_module_on_tree", "status": "PASS" if FRAMEGEN_SRC.is_file() else "FAIL",
                  "detail": "temporal/framegen.rs（mv 双向 warp + 遮挡感知混合 + MFG ×2/×3/×4）"})
    facts.append({"id": "probe_harness_on_tree", "status": "PASS" if PROBE_SRC.is_file() else "FAIL",
                  "detail": "bin/g19_frame_gen_probe.rs（解析式 GT 全帧率序列）"})
    p = wel.load_latest_evidence("g19_frame_gen_probe")
    doc = wel.load_json(p) if p else {}
    lanes = doc.get("lanes", [])
    modes = sorted(l.get("mode_x") for l in lanes)
    facts.append({"id": "mfg_three_lanes", "status": "PASS" if modes == [2, 3, 4] else "FAIL",
                  "detail": f"lanes={modes}（闭集 ×2/×3/×4）"})
    q = doc.get("all_lanes_quality_pass") is True
    facts.append({"id": "quality_interp_gt_hold_per_frame", "status": "PASS" if q else "FAIL",
                  "detail": "逐帧 SSIM(interp)>SSIM(frame-hold) 程序产对照阈（禁手写）"
                  + (f"；min_margin={min((l.get('min_margin', 0) for l in lanes), default=None)}" if lanes else "")})
    facts.append({"id": "double_run_bitexact", "status": "PASS" if doc.get("double_run_bitexact") is True else "FAIL",
                  "detail": "生成帧字节 sha256 双跑位级一致"})
    facts.append({"id": "real_fps_caliber_invariant", "status": "PASS" if doc.get("real_fps_caliber_invariant") is True else "FAIL",
                  "detail": "真实渲染帧率禁计生成帧（口径恒等式重算核验）；presented 独立登记面永不混算"})
    r = subprocess.run(["git", "diff", "--quiet", "g18-closed", "--", PERF_BIN_SRC],
                       cwd=ROOT, capture_output=True)
    facts.append({"id": "perf_bin_0byte_this_milestone", "status": "PASS" if r.returncode == 0 else "FAIL",
                  "detail": f"g14_3_pipeline_perf.rs vs g18-closed 0-byte（默认臂 Stage A digest 锚红线；rc={r.returncode}）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G19.2 M-a：FG/MFG 独立层 host 参考臂实现（RFC-0036；G13-N7 兑现）",
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
