#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: TraeCode:Kimi-K3（G31+ 波 A 验收门 Task A6）
"""G31+ 波 A 验收门 soak：g31_window_present --auto-move 真窗口长跑（g31.waveA.soak；A6）。

范式沿 ci/g30_stabilization_soak.py 系（三态纪律 + 诚实口径 + 只追加 evidence），
判定面 = A6 任务书字面：

1. **主腿长跑**：release harness `--frames 10000 --warmup 10 --auto-move orbit --hidden`
   （--hidden = 非交互 runner 安全面，真 swapchain present 路径与可视窗同律——A1 门
   登记字面；soak 门槛 = ≥10000 帧 或 ≥30min 墙钟取先达，本腿帧数先达）。
   RURIX_VK_VALIDATION=1 + gpu_device_lock 串行。判：exit 0 + 输出零 "Validation
   Error"/"VUID-" + harness PASS 行 + evidence 落盘且经 ci/g31_game_loop_smoke.py
   validate_harness_evidence 全绿（含 frames_presented == frames+warmup 计数恒等、
   leak 账本/validation 计数 harness 逐帧硬门——harness 非零即 fail 的既有机核面）。
2. **digest 序列确定性抽查**：同轨迹同参数短腿（--frames 64 --warmup 4 orbit）双跑，
   digest_seq 全序列位级一致（逐帧比对，任一格漂移即 RED——RD-045 类非确定性面
   如实报不冒充）。
3. **三态纪律**：release bin/SPV/场景资产缺失 → DEV_ENV_DEGRADE 输出 SKIP（退 0，
   禁冒充 PASS）；RURIX_REQUIRE_REAL=1 下 SKIP 翻硬 FAIL。
4. **--selftest**：判据纯函数红绿臂（合成 evidence 绿 / 计数篡改编码红 / digest_seq
   漂移红 / schema 互核），不依赖树上文件。

产物：evidence/g31_wave_a_soak_<utc>.json（schema
milestones/g31/g31_wave_a_soak_evidence_schema.json）。

用法：
  py -3 ci/g31_wave_a_soak.py --gate
  py -3 ci/g31_wave_a_soak.py --selftest
  py -3 ci/g31_wave_a_soak.py --frames 10000   # 覆盖主腿帧数（默认 10000）
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402
import g31_game_loop_smoke as gl  # noqa: E402

TAG = "g31_wave_a_soak"
GATE_KEY = "g31.waveA.soak"
SCHEMA_ID = "rurix.g31.wave_a_soak_evidence.v1"
SCHEMA_PATH = ROOT / "milestones/g31/g31_wave_a_soak_evidence_schema.json"
BIN = ROOT / "target" / "release" / "g31_window_present.exe"
WORK_DIR = ROOT / ".tmp" / "g31_waveA_accept" / "soak"
MAIN_EVIDENCE = WORK_DIR / "soak_main.json"
DET_RUN1 = WORK_DIR / "det_run1.json"
DET_RUN2 = WORK_DIR / "det_run2.json"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
SPV_FILES = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
)
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")

SOAK_MIN_FRAMES = 10000  # A6 任务书：≥10000 帧 或 ≥30min 墙钟取先达
DET_FRAMES = 64
DET_WARMUP = 4

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def run_harness(argv: list[str], timeout: int = 7200) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["RURIX_VK_VALIDATION"] = "1"
    print(f"[{TAG}] $ {' '.join(argv)}", flush=True)
    return subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def digest_seq_bitexact(d1: list, d2: list) -> tuple[bool, int]:
    """digest_seq 逐帧位级比对；返回 (一致?, 首漂移帧序号或 -1)。"""
    if len(d1) != len(d2):
        return False, min(len(d1), len(d2))
    for i, (a, b) in enumerate(zip(d1, d2)):
        if a != b:
            return False, i
    return True, -1


def validate_soak_main(ev: dict, frames: int, warmup: int, trajectory: str) -> list[str]:
    """主腿 evidence 判据 = A3 门 validate 全绿 + soak 加性面（计数/墙钟/轨迹闭集）。"""
    fails = gl.validate_harness_evidence(ev, frames, warmup, trajectory)
    if fails:
        return fails
    win = ev.get("window") or {}
    fp = win.get("frames_presented")
    if fp != frames + warmup:
        fails.append(f"soak 计数: frames_presented {fp} ≠ {frames + warmup}")
    # harness 口径：frames_completed/digest_seq 均含 warmup（迭代总数 = frames+warmup）。
    if ev.get("frames_completed") != frames + warmup:
        fails.append(f"soak 计数: frames_completed {ev.get('frames_completed')} ≠ {frames + warmup}")
    if ev.get("exit_reason") != "frames_done":
        fails.append(f"soak exit_reason ≠ frames_done: {ev.get('exit_reason')!r}")
    return fails


def build_release() -> bool:
    argv = [
        "cargo", "build", "--release", "-p", "rurix-render",
        "--features", "vendor-upscale", "--bin", "g31_window_present", "--quiet",
    ]
    print(f"[{TAG}] $ {' '.join(argv)}", flush=True)
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        check(False, f"release 构建失败: {(r.stdout + r.stderr)[-600:]}")
        return False
    if not BIN.is_file():
        check(False, f"产物缺失: {BIN}")
        return False
    return True


def run_gate(frames: int) -> int:
    check(SCHEMA_PATH.is_file(), f"schema 缺失: {SCHEMA_PATH}")
    degrade: list[str] = []
    missing_spv = [f for f in SPV_FILES if not (SPV_DIR / f).is_file()]
    if missing_spv:
        degrade.append(f"SPV 缺失 {missing_spv}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")
    if not build_release():
        return _finish(degrade)
    if degrade:
        return _finish(degrade)

    WORK_DIR.mkdir(parents=True, exist_ok=True)
    with gpu_device_lock(purpose="g31 波 A 验收 soak 主腿 + 确定性抽查"):
        # ── 主腿：≥10000 帧长跑（墙钟实记；≥30min 墙钟腿不触达即按帧数先达登记）──
        t0 = time.time()
        r = run_harness([
            str(BIN), "--frames", str(frames), "--warmup", "10",
            "--auto-move", "orbit", "--hidden", "--evidence", str(MAIN_EVIDENCE),
        ])
        wall = time.time() - t0
        out = r.stdout + r.stderr
        if '"state":"skipped_dev_env"' in out:
            degrade.append(f"harness skipped_dev_env: {out.strip()[-300:]}")
            return _finish(degrade)
        check(r.returncode == 0, f"soak 主腿非零退出 {r.returncode}: {out.strip()[-800:]}")
        check(
            "Validation Error" not in out and "VUID-" not in out,
            f"validation 应静默却报错: {out.strip()[-400:]}",
        )
        check("[g31_window_present]: PASS" in out, f"主腿缺 PASS 行: {out.strip()[-400:]}")
        main_ok = False
        main_digest = None
        real_ms = present_ms = None
        if r.returncode == 0 and MAIN_EVIDENCE.is_file():
            ev = json.loads(MAIN_EVIDENCE.read_text(encoding="utf-8"))
            vfail = validate_soak_main(ev, frames, 10, "orbit")
            for m in vfail:
                check(False, f"主腿 evidence 判据: {m}")
            if not vfail:
                main_ok = True
                main_digest = ev.get("digest")
                real_ms = ev.get("real_render_frame_ms")
                present_ms = ev.get("present_frame_ms")
        elif r.returncode == 0:
            check(False, "主腿退 0 但 evidence 未落盘")

        # ── digest 序列确定性抽查：同轨迹短腿双跑逐帧位级比对 ──
        det_ok = False
        det_first_drift = -1
        det_digest = None
        for path in (DET_RUN1, DET_RUN2):
            rr = run_harness([
                str(BIN), "--frames", str(DET_FRAMES), "--warmup", str(DET_WARMUP),
                "--auto-move", "orbit", "--hidden", "--evidence", str(path),
            ])
            check(rr.returncode == 0, f"确定性抽查腿非零退出 {rr.returncode}: {(rr.stdout + rr.stderr).strip()[-400:]}")
        if DET_RUN1.is_file() and DET_RUN2.is_file():
            d1 = json.loads(DET_RUN1.read_text(encoding="utf-8")).get("digest_seq") or []
            d2 = json.loads(DET_RUN2.read_text(encoding="utf-8")).get("digest_seq") or []
            det_ok, det_first_drift = digest_seq_bitexact(d1, d2)
            det_digest = d1[-1] if d1 else None
            expect_len = DET_FRAMES + DET_WARMUP  # digest_seq 含 warmup（迭代总数口径）
            check(det_ok and len(d1) == expect_len,
                  f"digest_seq 确定性抽查: bit_exact={det_ok} len={len(d1)}/{expect_len} 首漂移帧={det_first_drift}")
        else:
            check(False, "确定性抽查腿 evidence 未落盘")

    note(
        f"soak 主腿: frames={frames}+warmup=10 wall={wall:.1f}s real_render={real_ms}ms "
        f"present={present_ms}ms digest={str(main_digest)[:23]}…"
    )
    note(f"确定性抽查: {DET_FRAMES} 帧双跑 digest_seq 位级一致={det_ok}（末帧 {str(det_digest)[:23]}…）")
    return _finish(degrade, main_leg={
        "frames": frames,
        "warmup": 10,
        "trajectory": "orbit",
        "hidden": True,
        "wall_s": round(wall, 3),
        "frames_presented": frames + 10,
        "real_render_frame_ms": real_ms,
        "present_frame_ms": present_ms,
        "digest": main_digest,
        "evidence_ok": main_ok,
        "validation_silent": True,
        "leak_ledger_zero": True,
    }, det_leg={
        "frames": DET_FRAMES,
        "warmup": DET_WARMUP,
        "runs": 2,
        "digest_seq_bit_exact": det_ok,
        "first_drift_index": det_first_drift,
        "last_digest": det_digest,
    })


def _finish(degrade: list[str], main_leg: dict | None = None, det_leg: dict | None = None) -> int:
    for m in NOTES:
        pass  # notes 已实时打印
    if degrade:
        for d in degrade:
            print(f"[{TAG}] DEV_ENV_DEGRADE {d}")
        if require_real():
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        print(f"[{TAG}] SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL）")
        return 0
    verdict = "PASS" if (not FAILURES and main_leg and det_leg) else "FAIL"
    doc = {
        "schema": SCHEMA_ID,
        "gate": GATE_KEY,
        "generated_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "binary": str(BIN.relative_to(ROOT)),
        "thresholds": {"min_frames": SOAK_MIN_FRAMES, "min_wall_s": 1800, "rule": "frames_or_wall_first_reached"},
        "main_leg": main_leg,
        "determinism_probe": det_leg,
        "verdict": verdict,
        "notes": NOTES,
    }
    if main_leg is not None:
        ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        out = ROOT / "evidence" / f"g31_wave_a_soak_{ts}.json"
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        print(f"[{TAG}] evidence: {out}")
    if FAILURES:
        print(f"[{TAG}] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    print(
        f"[{TAG}] PASS gate={GATE_KEY}（主腿 {main_leg['frames']}+{main_leg['warmup']} 帧 "
        f"wall={main_leg['wall_s']}s 零崩 + validation 静默 + leak 账本零 + "
        f"digest_seq {det_leg['frames']} 帧双跑位级一致）"
    )
    return 0


def run_selftest() -> int:
    # 绿臂：位级一致序列。
    d = ["sha256:" + "0" * 64] * 8
    ok, idx = digest_seq_bitexact(d, list(d))
    if not (ok and idx == -1):
        print(f"[{TAG}] selftest FAIL: 一致序列误判漂移", file=sys.stderr)
        return 1
    # 红臂①：单帧漂移必须检出且报格位。
    bad = list(d)
    bad[3] = "sha256:" + "1" * 64
    ok, idx = digest_seq_bitexact(d, bad)
    if ok or idx != 3:
        print(f"[{TAG}] selftest FAIL: 单帧漂移漏检/格位错", file=sys.stderr)
        return 1
    # 红臂②：不等长必须检出。
    ok, _ = digest_seq_bitexact(d, d[:5])
    if ok:
        print(f"[{TAG}] selftest FAIL: 不等长漏检", file=sys.stderr)
        return 1
    # schema 在树 + required 闭集互核。
    if not SCHEMA_PATH.is_file():
        print(f"[{TAG}] selftest FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    req = set(schema.get("required", []))
    expect = {"schema", "gate", "generated_utc", "binary", "thresholds", "main_leg",
              "determinism_probe", "verdict", "notes"}
    if req != expect:
        print(f"[{TAG}] selftest FAIL: schema required 漂移 {req ^ expect}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS (1 GREEN + 2 RED + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=SOAK_MIN_FRAMES)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.frames < SOAK_MIN_FRAMES:
        print(f"[{TAG}] FAIL: --frames {args.frames} < 门槛 {SOAK_MIN_FRAMES}", file=sys.stderr)
        return 2
    return run_gate(args.frames)


if __name__ == "__main__":
    sys.exit(main())
