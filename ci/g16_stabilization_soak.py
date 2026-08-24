#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16plus soak）
"""G16plus soak（g16.wave.6a.soak，步骤 291）。仅当 M-g 已绿才跑 ≥1800s；谎报秒数判红。"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g16_p0_lib as g16  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g16.wave.6a.soak"
NUMERIC_STEP = 291
SUBJECT = "g16_stabilization_soak"
WAVE = "G16.6a"
SOURCE_REF = "G16_CONTRACT G-G16-11;G16PLUS_RECORD §2 soak"
SCHEMA = g16.ROOT / "milestones" / "g16" / "g16_stabilization_soak_evidence_schema.json"
BIN = g16.ROOT / "target" / "release" / ("g14_3_pipeline_perf.exe" if sys.platform == "win32" else "g14_3_pipeline_perf")
MIN_SECONDS = 1800.0


def _mg_closed() -> tuple[bool, str]:
    p = wel.load_latest_evidence("g16_m_g_absolute_quality_closure")
    if p is None:
        return False, "缺 M-g evidence"
    doc = wel.load_json(p)
    facts = {f.get("id"): f.get("status") for f in doc.get("extra_facts") or []}
    ok = bool(doc.get("host_section_pass")) and facts.get("met_count_18") == "PASS"
    return ok, f"{p.name} host={doc.get('host_section_pass')} met18={facts.get('met_count_18')}"


def run_gate() -> int:
    facts = []
    mg_ok, mg_d = _mg_closed()
    facts.append(g16.fact("m_g_green_precondition", mg_ok, mg_d))
    facts.append(g16.fact("sleep_seconds_zero", True, "迭代间零 sleep 字面"))
    if not mg_ok:
        facts.append(g16.fact("soak_wall_clock_ge_1800", False, "M-g 未绿：禁跑 soak 不谎报"))
        facts.append(g16.fact("iterations_nonzero", False, "未启动"))
        facts.append(g16.fact("failures_zero", True, "未启动故零失败"))
        facts.append(g16.fact("active_chain_matches_wall", True, "未启动"))
        facts.append(g16.fact("no_sleep_between_iters", True, "sleep=0"))
        facts.append(g16.fact("gi_on_lane_used", False, "未启动"))
        return g16.emit(WAVE, SUBJECT, GATE_KEY, NUMERIC_STEP, SOURCE_REF, SCHEMA, facts, "G16plus soak blocked")
    if not BIN.is_file():
        facts.append(g16.fact("soak_wall_clock_ge_1800", False, "缺 release bin"))
        facts.append(g16.fact("iterations_nonzero", False, "缺 bin"))
        facts.append(g16.fact("failures_zero", False, "缺 bin"))
        facts.append(g16.fact("active_chain_matches_wall", False, "缺 bin"))
        facts.append(g16.fact("no_sleep_between_iters", True, "sleep=0"))
        facts.append(g16.fact("gi_on_lane_used", False, "缺 bin"))
        return g16.emit(WAVE, SUBJECT, GATE_KEY, NUMERIC_STEP, SOURCE_REF, SCHEMA, facts, "G16plus soak no bin")
    combos = [
        ("cornell-box", 67, "tsr_device"),
        ("bistro-interior", 67, "tsr_device"),
    ]
    t0 = time.perf_counter()
    iters = 0
    fails = 0
    active = 0.0
    while True:
        scene, tier, backend = combos[iters % len(combos)]
        it0 = time.perf_counter()
        r = subprocess.run(
            [
                str(BIN), "--bench", "--scene", scene, "--tier", str(tier),
                "--backend", backend, "--frames", "32", "--warmup", "2", "--gi", "on",
            ],
            cwd=g16.ROOT, capture_output=True, text=True,
        )
        active += time.perf_counter() - it0
        iters += 1
        if r.returncode != 0:
            fails += 1
        wall = time.perf_counter() - t0
        if wall >= MIN_SECONDS:
            break
    wall = time.perf_counter() - t0
    drift = abs(active - wall)
    facts.append(g16.fact("soak_wall_clock_ge_1800", wall >= MIN_SECONDS, f"wall={wall:.3f}"))
    facts.append(g16.fact("iterations_nonzero", iters > 0, f"iters={iters}"))
    facts.append(g16.fact("failures_zero", fails == 0, f"fails={fails}"))
    facts.append(g16.fact("active_chain_matches_wall", drift < 5.0, f"active={active:.3f} wall={wall:.3f}"))
    facts.append(g16.fact("no_sleep_between_iters", True, "sleep=0"))
    facts.append(g16.fact("gi_on_lane_used", True, "--gi on"))
    return g16.emit(WAVE, SUBJECT, GATE_KEY, NUMERIC_STEP, SOURCE_REF, SCHEMA, facts, f"soak wall={wall:.3f}")


def run_selftest() -> int:
    if NUMERIC_STEP != 291 or not SCHEMA.is_file():
        print("[g16_soak] SELFTEST FAIL")
        return 1
    print("[g16_soak] SELFTEST PASS")
    return 0


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
        return g16.verify_latest_wave(SUBJECT, 8)
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
