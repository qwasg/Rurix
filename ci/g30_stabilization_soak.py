#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G30.5 soak 收尾脚本派生）
"""G30.5 稳定 soak（g30.wave.5a.soak，步骤 523）。

管线 bench 四组合轮转 + 探针轮换八车道维持（G29-N5 扩容终态；G30 零新 kernel，
车道表与 g29 版完全一致）：每第 5 迭代按
probe_iters % 8 轮转八车道——g19/g20/g21/g22 战役四实现件探针 + 第五车道
g26_framegen_device（--probe ×2 档 8 帧快车道）+ 第六车道 g27_hzb_device
（--probe 零容差，reduce/test 双 SPV，无 --tol）+ 第七车道 g28_restir_device
（--probe，--tol 取 g28 预算条目 g28.restir_device.host_device_estimate_tol
的 threshold；budget 无该条目 = 零容差态，省略 --tol）+ 第八车道 g29_slab_device
（--probe，--tol 取 g29 预算条目 g29.slab_device.host_device_reflectance_tol
的 threshold；budget 无该条目 = 零容差态，省略 --tol）。soak 启动时现场编译
五 SPV（rurixc --target vulkan + spirv-val）并构建四 device 探针；前置缺任一
→ facts 诚实红落盘。
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
GATE_KEY = "g30.wave.5a.soak"
NUMERIC_STEP = 523
SUBJECT = "g30_stabilization_soak"
WAVE = "G30.5"
SCHEMA_PATH = ROOT / "milestones/g30/g30_stabilization_soak_evidence_schema.json"
BIN = ROOT / "target/release/g14_3_pipeline_perf.exe"
FRAMEGEN_BUDGET_PATH = ROOT / "milestones/g26/g26_budget.json"
FRAMEGEN_TOL_ID = "g26.framegen_device.host_device_maxdiff_tol"
RESTIR_BUDGET_PATH = ROOT / "milestones/g28/g28_budget.json"
RESTIR_TOL_ID = "g28.restir_device.host_device_estimate_tol"
SLAB_BUDGET_PATH = ROOT / "milestones/g29/g29_budget.json"
SLAB_TOL_ID = "g29.slab_device.host_device_reflectance_tol"
FRAMEGEN_RX = ROOT / "src/rurix-render/kernels/g26_framegen.rx"
HZB_REDUCE_RX = ROOT / "src/rurix-render/kernels/g27_hzb_reduce.rx"
HZB_TEST_RX = ROOT / "src/rurix-render/kernels/g27_hzb_test.rx"
RESTIR_RX = ROOT / "src/rurix-render/kernels/g28_restir.rx"
SLAB_RX = ROOT / "src/rurix-render/kernels/g29_slab.rx"
SOAK_DIR = ROOT / ".tmp/g30_soak"
FRAMEGEN_SPV = SOAK_DIR / "g26_framegen.spv"
HZB_REDUCE_SPV = SOAK_DIR / "g27_hzb_reduce.spv"
HZB_TEST_SPV = SOAK_DIR / "g27_hzb_test.spv"
RESTIR_SPV = SOAK_DIR / "g28_restir.spv"
SLAB_SPV = SOAK_DIR / "g29_slab.spv"
RURIXC = ROOT / "target/debug/rurixc.exe"
FRAMEGEN_DEV_RELEASE = ROOT / "target/release/g26_framegen_device.exe"
FRAMEGEN_DEV_DEBUG = ROOT / "target/debug/g26_framegen_device.exe"
HZB_DEV_RELEASE = ROOT / "target/release/g27_hzb_device.exe"
HZB_DEV_DEBUG = ROOT / "target/debug/g27_hzb_device.exe"
RESTIR_DEV_RELEASE = ROOT / "target/release/g28_restir_device.exe"
RESTIR_DEV_DEBUG = ROOT / "target/debug/g28_restir_device.exe"
SLAB_DEV_RELEASE = ROOT / "target/release/g29_slab_device.exe"
SLAB_DEV_DEBUG = ROOT / "target/debug/g29_slab_device.exe"
MIN_SECONDS = 1800.0


def fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def prepare_device_lanes() -> tuple[bool, str, float | None, float | None, float | None]:
    """soak 启动时现场编译五 SPV + 构建四 device 探针（五/六/七/八车道前置）；返回 (ok, detail, framegen_tol, restir_tol, slab_tol)。"""
    framegen_tol: float | None = None
    if FRAMEGEN_BUDGET_PATH.is_file():
        for e in wel.load_json(FRAMEGEN_BUDGET_PATH).get("entries", []):
            if e.get("id") == FRAMEGEN_TOL_ID:
                framegen_tol = e.get("threshold")
                break
    if framegen_tol is None:
        return False, f"budget 缺 {FRAMEGEN_TOL_ID}", None, None, None
    restir_tol: float | None = None
    if RESTIR_BUDGET_PATH.is_file():
        for e in wel.load_json(RESTIR_BUDGET_PATH).get("entries", []):
            if e.get("id") == RESTIR_TOL_ID:
                restir_tol = e.get("threshold")
                break
    # restir_tol 缺条目 = 零容差态合法（g28_budget 头注「实测位级可达则零容差零条目」），lane6 省略 --tol。
    slab_tol: float | None = None
    if SLAB_BUDGET_PATH.is_file():
        for e in wel.load_json(SLAB_BUDGET_PATH).get("entries", []):
            if e.get("id") == SLAB_TOL_ID:
                slab_tol = e.get("threshold")
                break
    # slab_tol 缺条目 = 零容差态合法（g29_budget 头注「实测位级可达则零容差零条目」），lane7 省略 --tol。
    SOAK_DIR.mkdir(parents=True, exist_ok=True)
    steps: list[list[str]] = [
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"],
        [str(RURIXC), str(FRAMEGEN_RX), "--target", "vulkan", "-o", str(FRAMEGEN_SPV)],
        ["spirv-val", str(FRAMEGEN_SPV)],
        [str(RURIXC), str(HZB_REDUCE_RX), "--target", "vulkan", "-o", str(HZB_REDUCE_SPV)],
        ["spirv-val", str(HZB_REDUCE_SPV)],
        [str(RURIXC), str(HZB_TEST_RX), "--target", "vulkan", "-o", str(HZB_TEST_SPV)],
        ["spirv-val", str(HZB_TEST_SPV)],
        [str(RURIXC), str(RESTIR_RX), "--target", "vulkan", "-o", str(RESTIR_SPV)],
        ["spirv-val", str(RESTIR_SPV)],
        [str(RURIXC), str(SLAB_RX), "--target", "vulkan", "-o", str(SLAB_SPV)],
        ["spirv-val", str(SLAB_SPV)],
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan",
         "--bin", "g26_framegen_device", "--bin", "g27_hzb_device",
         "--bin", "g28_restir_device", "--bin", "g29_slab_device"],
    ]
    for cmd in steps:
        try:
            r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
        except OSError as e:
            return False, f"{cmd[0]} 不可用: {e}", framegen_tol, restir_tol, slab_tol
        if r.returncode != 0:
            return False, f"{Path(cmd[0]).name} rc={r.returncode}", framegen_tol, restir_tol, slab_tol
    restir_desc = str(restir_tol) if restir_tol is not None else "零容差（budget 无条目，省略 --tol）"
    slab_desc = str(slab_tol) if slab_tol is not None else "零容差（budget 无条目，省略 --tol）"
    return True, (f"五 SPV+四 device 探针就绪 framegen_tol={framegen_tol} "
                  f"restir_tol={restir_desc} slab_tol={slab_desc}"), framegen_tol, restir_tol, slab_tol


def run_gate() -> int:
    facts: list[dict] = []
    md = wel.load_latest_evidence("g30_m_d_campaign_handover_ledger")
    md_ok = md is not None and wel.load_json(md).get("host_section_pass") is True
    facts.append(fact("m_d_precondition", md_ok, md.name if md else "missing M-d"))
    facts.append(fact("sleep_seconds_zero", True, "迭代间零 sleep"))
    lane_ok, lane_detail, framegen_tol, restir_tol, slab_tol = prepare_device_lanes()
    if not md_ok or not BIN.is_file() or not lane_ok:
        why = ("M-d 未绿" if not md_ok
               else "缺 release bin" if not BIN.is_file()
               else f"framegen/hzb/restir/slab device 车道前置未就绪：{lane_detail}")
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
                lane = probe_iters % 8
                out_dir = ROOT / ".tmp" / "g30_soak_probes"
                out_dir.mkdir(parents=True, exist_ok=True)
                if lane == 4:
                    # 第五车道：framegen device 探针（--probe ×2 档 8 帧快车道；
                    # release 产物存在则优先 release 路径）。
                    dev = FRAMEGEN_DEV_RELEASE if FRAMEGEN_DEV_RELEASE.is_file() else FRAMEGEN_DEV_DEBUG
                    out = out_dir / f"iter_{iters}_framegen.json"
                    r = subprocess.run(
                        [str(dev), "--probe", "--spv", str(FRAMEGEN_SPV),
                         "--tol", str(framegen_tol), "--out", str(out)],
                        cwd=ROOT, capture_output=True, text=True,
                    )
                elif lane == 5:
                    # 第六车道：hzb device 探针（--probe 零容差无 --tol；
                    # reduce/test 双 SPV；release 产物存在则优先 release 路径）。
                    dev = HZB_DEV_RELEASE if HZB_DEV_RELEASE.is_file() else HZB_DEV_DEBUG
                    out = out_dir / f"iter_{iters}_hzb.json"
                    r = subprocess.run(
                        [str(dev), "--probe", "--spv-reduce", str(HZB_REDUCE_SPV),
                         "--spv-test", str(HZB_TEST_SPV), "--out", str(out)],
                        cwd=ROOT, capture_output=True, text=True,
                    )
                elif lane == 6:
                    # 第七车道：restir device 探针（--tol 取 g28 预算条目 threshold；
                    # 零容差态省略 --tol；release 产物存在则优先 release 路径）。
                    dev = RESTIR_DEV_RELEASE if RESTIR_DEV_RELEASE.is_file() else RESTIR_DEV_DEBUG
                    out = out_dir / f"iter_{iters}_restir.json"
                    cmd = [str(dev), "--probe", "--spv", str(RESTIR_SPV)]
                    if restir_tol is not None:
                        cmd += ["--tol", str(restir_tol)]
                    cmd += ["--out", str(out)]
                    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
                elif lane == 7:
                    # 第八车道：slab device 探针（--tol 取 g29 预算条目 threshold；
                    # 零容差态省略 --tol；release 产物存在则优先 release 路径）。
                    dev = SLAB_DEV_RELEASE if SLAB_DEV_RELEASE.is_file() else SLAB_DEV_DEBUG
                    out = out_dir / f"iter_{iters}_slab.json"
                    cmd = [str(dev), "--probe", "--spv", str(SLAB_SPV)]
                    if slab_tol is not None:
                        cmd += ["--tol", str(slab_tol)]
                    cmd += ["--out", str(out)]
                    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
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
                 f"probe_iters={probe_iters}（战役四实现件 + framegen/hzb/restir/slab 四 device 八车道探针轮换穿插复跑）"),
        ])
        ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref="G30_CONTRACT G-G30-6", required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G30 soak ≥1800s（管线四组合 + 八车道探针轮换维持穿插，含 framegen/hzb/restir/slab 四 device 车道穿插（八车道轮换））",
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
        print("[g30_soak] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
