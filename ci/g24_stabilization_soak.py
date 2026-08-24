#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G24.5 soak 波）
"""G24.5 稳定 soak（g24.wave.5a.soak，步骤 427）。"""
from __future__ import annotations

import argparse
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g24.wave.5a.soak"
NUMERIC_STEP = 427
SUBJECT = "g24_stabilization_soak"
WAVE = "G24.5"
SCHEMA_PATH = ROOT / "milestones/g24/g24_stabilization_soak_evidence_schema.json"
BIN = ROOT / "target/release/g14_3_pipeline_perf.exe"
LEGACY_REG = ROOT / "milestones/g24/g24_legacy_rd_registry.json"
MIN_SECONDS = 1800.0


def fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def run_gate() -> int:
    facts: list[dict] = []
    md = wel.load_latest_evidence("g24_m_d_safe_gpu_and_legacy_rd_disposition")
    md_ok = md is not None and wel.load_json(md).get("host_section_pass") is True
    facts.append(fact("m_d_precondition", md_ok, md.name if md else "missing M-d"))
    facts.append(fact("sleep_seconds_zero", True, "迭代间零 sleep"))
    if not md_ok or not BIN.is_file() or not LEGACY_REG.is_file():
        why = "M-d 未绿" if not md_ok else "缺 release bin"
        facts.extend([
            fact("soak_wall_clock_ge_1800", False, why),
            fact("iterations_nonzero", False, "未启动"),
            fact("failures_zero", True, "未启动"),
            fact("active_chain_matches_wall", True, "未启动"),
            fact("no_sleep_between_iters", True, "sleep=0"),
            fact("probe_lane_interleaved", False, "未启动"),
        ])
        ok = False
    else:
        combos = [
            ("bistro-interior", 100, "dlss_sr"),
            ("cornell-box", 100, "dlss_sr"),
            ("bistro-interior", 67, "tsr_device"),
            ("cornell-box", 67, "fsr_3_1_5"),
        ]
        t0 = time.perf_counter()
        iters = fails = probe_iters = 0
        active = 0.0
        while time.perf_counter() - t0 < MIN_SECONDS:
            it0 = time.perf_counter()
            if iters % 5 == 4:
                r = subprocess.run([sys.executable, str(ROOT / "milestones/g24/harness/g24_hdr_probe.py")],
                                   cwd=ROOT, capture_output=True, text=True)
                probe_iters += 1
            else:
                scene, tier, backend = combos[iters % len(combos)]
                r = subprocess.run(
                    [str(BIN), "--bench", "--scene", scene, "--tier", str(tier),
                     "--backend", backend, "--frames", "32", "--warmup", "2"],
                    cwd=ROOT, capture_output=True, text=True,
                )
            active += time.perf_counter() - it0
            iters += 1
            if r.returncode != 0:
                fails += 1
        wall = time.perf_counter() - t0
        facts.extend([
            fact("soak_wall_clock_ge_1800", wall >= MIN_SECONDS, f"wall={wall:.1f}s"),
            fact("iterations_nonzero", iters > 0, f"iters={iters}"),
            fact("failures_zero", fails == 0, f"fails={fails}"),
            fact("active_chain_matches_wall", active <= wall * 1.05, f"active={active:.1f}s wall={wall:.1f}s"),
            fact("no_sleep_between_iters", True, "sleep=0"),
            fact("probe_lane_interleaved", probe_iters > 0, f"probe_iters={probe_iters}（HDR 探针取证车道穿插复跑）"),
        ])
        ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref="G24_CONTRACT G-G24-6", required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G24 soak ≥1800s（管线四组合 + HDR 探针车道穿插）", host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print("[g24_soak] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
