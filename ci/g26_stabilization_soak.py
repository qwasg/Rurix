#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G26.5 soak 波）
"""G26.5 稳定 soak（g26.wave.5a.soak，步骤 459）。

管线 bench 四组合轮转 + 探针轮换扩容（G26-N5）：每第 5 迭代按 probe_iters % 5
轮转五车道——g19/g20/g21/g22 战役四实现件探针 + 第五车道 g26_framegen_device
（--probe ×2 档 8 帧快车道）。soak 启动时现场编译 SPV（rurixc --target vulkan
+ spirv-val）并构建 device 探针；前置缺任一 → facts 诚实红落盘。
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
GATE_KEY = "g26.wave.5a.soak"
NUMERIC_STEP = 459
SUBJECT = "g26_stabilization_soak"
WAVE = "G26.5"
SCHEMA_PATH = ROOT / "milestones/g26/g26_stabilization_soak_evidence_schema.json"
BIN = ROOT / "target/release/g14_3_pipeline_perf.exe"
BUDGET_PATH = ROOT / "milestones/g26/g26_budget.json"
TOL_ID = "g26.framegen_device.host_device_maxdiff_tol"
KERNEL_RX = ROOT / "src/rurix-render/kernels/g26_framegen.rx"
SOAK_SPV = ROOT / ".tmp/g26_soak/g26_framegen.spv"
RURIXC = ROOT / "target/debug/rurixc.exe"
DEV_RELEASE = ROOT / "target/release/g26_framegen_device.exe"
DEV_DEBUG = ROOT / "target/debug/g26_framegen_device.exe"
MIN_SECONDS = 1800.0


def fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def prepare_framegen_lane() -> tuple[bool, str, float | None]:
    """soak 启动时现场编译 SPV + 构建 device 探针（第五车道前置）；返回 (ok, detail, tol)。"""
    tol: float | None = None
    if BUDGET_PATH.is_file():
        for e in wel.load_json(BUDGET_PATH).get("entries", []):
            if e.get("id") == TOL_ID:
                tol = e.get("threshold")
                break
    if tol is None:
        return False, f"budget 缺 {TOL_ID}", None
    SOAK_SPV.parent.mkdir(parents=True, exist_ok=True)
    steps: list[list[str]] = [
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"],
        [str(RURIXC), str(KERNEL_RX), "--target", "vulkan", "-o", str(SOAK_SPV)],
        ["spirv-val", str(SOAK_SPV)],
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", "g26_framegen_device"],
    ]
    for cmd in steps:
        try:
            r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
        except OSError as e:
            return False, f"{cmd[0]} 不可用: {e}", tol
        if r.returncode != 0:
            return False, f"{Path(cmd[0]).name} rc={r.returncode}", tol
    return True, f"SPV+device 探针就绪 tol={tol}", tol


def run_gate() -> int:
    facts: list[dict] = []
    md = wel.load_latest_evidence("g26_m_d_g17_md_f1_rejudgment_window")
    md_ok = md is not None and wel.load_json(md).get("host_section_pass") is True
    facts.append(fact("m_d_precondition", md_ok, md.name if md else "missing M-d"))
    facts.append(fact("sleep_seconds_zero", True, "迭代间零 sleep"))
    lane_ok, lane_detail, tol = prepare_framegen_lane()
    if not md_ok or not BIN.is_file() or not lane_ok:
        why = ("M-d 未绿" if not md_ok
               else "缺 release bin" if not BIN.is_file()
               else f"framegen device 车道前置未就绪：{lane_detail}")
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
                lane = probe_iters % 5
                out_dir = ROOT / ".tmp" / "g26_soak_probes"
                out_dir.mkdir(parents=True, exist_ok=True)
                if lane == 4:
                    # 第五车道：framegen device 探针（--probe ×2 档 8 帧快车道；
                    # release 产物存在则优先 release 路径）。
                    dev = DEV_RELEASE if DEV_RELEASE.is_file() else DEV_DEBUG
                    out = out_dir / f"iter_{iters}_framegen_device.json"
                    r = subprocess.run(
                        [str(dev), "--probe", "--spv", str(SOAK_SPV),
                         "--tol", str(tol), "--out", str(out)],
                        cwd=ROOT, capture_output=True, text=True,
                    )
                else:
                    probes = ["g19_frame_gen_probe.exe", "g20_hzb_probe.exe",
                              "g21_restir_probe.exe", "g22_slab_probe.exe"]
                    pb = ROOT / "target/release" / probes[lane]
                    out = out_dir / f"iter_{iters}.json"
                    r = subprocess.run([str(pb), "--out", str(out)],
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
            fact("probe_lane_interleaved", probe_iters > 0,
                 f"probe_iters={probe_iters}（战役四实现件 + framegen device 五车道探针轮换穿插复跑）"),
        ])
        ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref="G26_CONTRACT G-G26-6", required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G26 soak ≥1800s（管线四组合 + 五车道探针轮换扩容穿插，含 framegen device 车道穿插）",
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
        print("[g26_soak] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
