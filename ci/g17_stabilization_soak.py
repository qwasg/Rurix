#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Claude Fable 5（G17.7a soak 稳定波）
"""G17.7a 稳定 soak（g17.wave.7a.soak，步骤 307；G17_CONTRACT G-G17-8）。

仅当 M-d 终判 evidence 在档（两态均合法——达标 18/18 或维持未达标如实登记，
G17 与 G16「M-g 18/18 才 soak」不同：G-G17-9 字面两种结局均允许 close）才跑
≥1800s 连续复跑零失败；谎报秒数判红（active_chain ≈ wall 交叉核验，零 sleep）。

迭代体 = g14_3_pipeline_perf --bench 32 帧真跑（dlss_sr/tsr_device/fsr_3_1_5
三 backend 轮换 × 双场景——G17 主题 DLSS 车道默认臂，零 --gi 显式参数）。

用法：
  py -3 ci/g17_stabilization_soak.py --gate g17.wave.7a.soak
  py -3 ci/g17_stabilization_soak.py --verify-latest
  py -3 ci/g17_stabilization_soak.py --selftest
"""
from __future__ import annotations

import argparse
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g17.wave.7a.soak"
NUMERIC_STEP = 307  # post-interlock 实测顺位领取
SUBJECT = "g17_stabilization_soak"
WAVE = "G17.7a"
SOURCE_REF = "G17_CONTRACT G-G17-8;G17_PLAN §2 阶段⑦"
SCHEMA_PATH = ROOT / "milestones/g17/g17_stabilization_soak_evidence_schema.json"
BIN = ROOT / "target/release/g14_3_pipeline_perf.exe"
MIN_SECONDS = 1800.0


def fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _md_verdict_archived() -> tuple[bool, str]:
    p = wel.load_latest_evidence("g17_m_d_t100_final_verdict")
    if p is None:
        return False, "缺 M-d 终判 evidence"
    doc = wel.load_json(p)
    facts = {f.get("id"): f for f in doc.get("extra_facts") or []}
    v = facts.get("verdict_two_state_honest", {})
    ok = bool(doc.get("host_section_pass")) and v.get("status") == "PASS"
    return ok, f"{p.name} host={doc.get('host_section_pass')} verdict_fact={v.get('detail', '')[:120]}"


def emit(facts: list[dict], notes: str) -> int:
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT, notes=notes,
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def run_gate() -> int:
    facts: list[dict] = []
    md_ok, md_d = _md_verdict_archived()
    facts.append(fact("m_d_verdict_precondition", md_ok, md_d))
    facts.append(fact("sleep_seconds_zero", True, "迭代间零 sleep 字面"))
    if not md_ok or not BIN.is_file():
        why = "M-d 终判未档" if not md_ok else "缺 release bin"
        facts.append(fact("soak_wall_clock_ge_1800", False, f"{why}：禁跑 soak 不谎报"))
        facts.append(fact("iterations_nonzero", False, "未启动"))
        facts.append(fact("failures_zero", True, "未启动故零失败"))
        facts.append(fact("active_chain_matches_wall", True, "未启动"))
        facts.append(fact("no_sleep_between_iters", True, "sleep=0"))
        facts.append(fact("dlss_lane_used", False, "未启动"))
        return emit(facts, f"G17 soak blocked：{why}")
    combos = [
        ("bistro-interior", 100, "dlss_sr"),
        ("cornell-box", 100, "dlss_sr"),
        ("bistro-interior", 67, "tsr_device"),
        ("cornell-box", 67, "fsr_3_1_5"),
    ]
    t0 = time.perf_counter()
    iters = 0
    fails = 0
    active = 0.0
    while True:
        scene, tier, backend = combos[iters % len(combos)]
        it0 = time.perf_counter()
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
        if wall >= MIN_SECONDS:
            break
    wall = time.perf_counter() - t0
    drift = abs(active - wall)
    facts.append(fact("soak_wall_clock_ge_1800", wall >= MIN_SECONDS, f"wall={wall:.3f}s"))
    facts.append(fact("iterations_nonzero", iters > 0, f"iters={iters}"))
    facts.append(fact("failures_zero", fails == 0, f"fails={fails}/{iters}"))
    facts.append(fact("active_chain_matches_wall", drift < 5.0,
                      f"active={active:.3f} wall={wall:.3f} drift={drift:.3f}（谎报秒数交叉核验）"))
    facts.append(fact("no_sleep_between_iters", True, "sleep=0"))
    facts.append(fact("dlss_lane_used", True,
                      "迭代体含 dlss_sr 车道（bistro/cornell t100）+ tsr/fsr 轮换，默认臂零 --gi 参数"))
    return emit(facts, f"G17 soak wall={wall:.3f}s iters={iters} fails={fails}（≥1800s 零失败连续复跑）")


def verify_latest() -> int:
    p = wel.load_latest_evidence(SUBJECT)
    if p is None:
        print(f"[g17_soak] verify-latest FAIL: 无 {SUBJECT} evidence", file=sys.stderr)
        return 1
    doc = wel.load_json(p)
    ok = doc.get("host_section_pass") is True and all(
        f.get("status") == "PASS" for f in doc.get("extra_facts", [])
    )
    print(f"[g17_soak] verify-latest {'PASS' if ok else 'FAIL'}: {p.name}")
    return 0 if ok else 1


def run_selftest() -> int:
    ok = NUMERIC_STEP == 307 and SCHEMA_PATH.is_file() and BIN.parent.is_dir()
    # 前置红臂：M-d 未档时禁跑不谎报（函数面）
    print(f"  {'CLOSURE ok' if ok else 'CLOSURE FAIL'} — 步骤/schema/bin 目录在位")
    print(f"[g17_soak] SELFTEST {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


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
        return verify_latest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
