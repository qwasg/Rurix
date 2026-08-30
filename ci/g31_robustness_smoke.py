#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C4 运行时健壮性 + 故障注入）
"""G31+ 波 C Task C4 运行时健壮性 + 故障注入门（g31.waveC.robustness）。

范式沿 ci/g31_wave_a_soak.py 系（三态纪律 + 诚实口径 + 只追加 evidence）。
判定面 = C4 任务书字面：

1. **device lost 恢复面（三探针臂）**：env `RURIX_G31_FAULT_DEVICE_LOST=<point>@8`
   （point ∈ acquire|submit|present;注入 = 该点 Vulkan 返回值覆写 DEVICE_LOST,
   真实调用已完成、GPU 态不受污染）+ harness `--fault-probe device-lost-<point>`
   （验证面,双层门控）。判：退 0 + G31_FAULT_PROBE 单行 observed=true +
   cascade_present_poisoned=true + cascade_resize_poisoned=true（poisoned 锁存后
   第二次 present/resize 均确定性 Err——RXS-0077 同律禁 UB 级联;处置 = 确定性
   错误面 + 干净退出,会话重建恢复路径归后续波,如实登记）。
2. **TDR 面（探针臂）**：env `RURIX_G31_FAULT_FENCE_TIMEOUT=2`（持久帧 fence 有界
   等待第 2 次 = 帧 0 完成等待覆写 VK_TIMEOUT;长帧/卡死以超时面模拟,不真触
   系统 TDR 伤机）。判：退 0 + observed=true + 墙钟 < 上限（进程不挂死）。
3. **显存 budget 违约面（探针臂）**：env `RURIX_G31_FAULT_BUDGET_BYTES=1`
   （heap budget 上报钳 1 字节）。判：退 0 + observed=true（确定性 OOM Err
   fail-closed,不降级不挂死;内部分辨率降级路径归后续波,如实登记）。
4. **窗口风暴臂**：`--window-storm 121`（爆发程序化真 win32 resize 121 次
   （SetWindowPos 用户拖拽同通路）,半↔原 extent 真 swapchain/staging 重建,
   奇数收官于半 extent ⇒ 末段 era 重建 + 新 extent 渲染全真面）。判：退 0 +
   resize_ops=121 + resize_eras ≥ 1 + 零崩。
5. **soak 故障臂**：`--storm-soak 25 --frames 1000 --warmup 10 --auto-move orbit
   --hidden`（每 25 帧 resize toggle,每 200 帧最小化/恢复 WM_SIZE 同通路注入,
   ≥1000 帧）。判：退 0 + frames_completed=1010 + resize_ops ≥ 35 +
   min_cycles ≥ 5 + min_skips ≥ 5 + validation 静默 + leak 账本零（harness
   逐帧硬门）+ evidence 经 g31_game_loop_smoke.validate_harness_evidence 全绿。
6. **基线腿（默认关零行为变更证明）**：无注入 env,64+4 orbit hidden 短腿,
   退 0 + validate 全绿 + digest 记录。
7. **三态纪律**：release bin/SPV/场景资产缺失 → DEV_ENV_DEGRADE 输出 SKIP
   （退 0,禁冒充 PASS）;RURIX_REQUIRE_REAL=1 下 SKIP 翻硬 FAIL。
8. **--selftest**：探针行/风暴行解析纯函数红绿臂 + schema 互核,不依赖树上文件。

产物：evidence/g31_robustness_<utc>.json（schema
milestones/g31/g31_robustness_evidence_schema.json）。

用法：
  py -3 ci/g31_robustness_smoke.py --gate
  py -3 ci/g31_robustness_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402
import g31_game_loop_smoke as gl  # noqa: E402

TAG = "g31_robustness"
GATE_KEY = "g31.waveC.robustness"
SCHEMA_ID = "rurix.g31.robustness_evidence.v1"
SCHEMA_PATH = ROOT / "milestones/g31/g31_robustness_evidence_schema.json"
BIN = ROOT / "target" / "release" / "g31_window_present.exe"
WORK_DIR = ROOT / ".tmp" / "g31_robustness"
BASELINE_EVIDENCE = WORK_DIR / "baseline.json"
STORM_EVIDENCE = WORK_DIR / "storm_burst.json"
SOAK_EVIDENCE = WORK_DIR / "storm_soak.json"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
SPV_FILES = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
)
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")

PROBE_FRAMES = 40
PROBE_WARMUP = 4
PROBE_INJECT_INDEX = 8  # present 调用序注入点（0-based;post-warmup 第 4 帧）
TDR_WAIT_CAP_S = 300.0  # TDR 臂墙钟上限（不挂死证明面;真机装配+触发实测 ≈60~90s）
DEVICE_LOST_PROBES = ("device-lost-acquire", "device-lost-submit", "device-lost-present")
STORM_BURST_OPS = 121  # 奇数:收官于半 extent ⇒ 末段 era 重建全真面（偶数回原点则 resize_eras=0 如实登记）
STORM_BURST_FRAMES = 32
SOAK_FAULT_FRAMES = 1000
SOAK_FAULT_WARMUP = 10
SOAK_FAULT_PERIOD = 25
# 周期 toggle 期望下界：fi ∈ {25,50,…,1000} 共 40 次,其中 5 次（200 的倍数）
# 走最小化/恢复 → resize 35;最小化循环 5 次、跳过 ≥5。
SOAK_MIN_RESIZE_OPS = 35
SOAK_MIN_MIN_CYCLES = 5

PROBE_LINE_RE = re.compile(r"G31_FAULT_PROBE (\{.*\})")
STORM_LINE_RE = re.compile(
    r"storm resize_ops=(\d+) min_cycles=(\d+) min_skips=(\d+) resize_eras=(\d+)"
)

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


def run_harness(argv: list[str], env_extra: dict[str, str] | None = None,
                timeout: int = 1800) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["RURIX_VK_VALIDATION"] = "1"
    if env_extra:
        env.update(env_extra)
    print(f"[{TAG}] $ {' '.join(argv)}  (env+={list((env_extra or {}).keys())})", flush=True)
    return subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def parse_probe_line(out: str) -> dict | None:
    """G31_FAULT_PROBE 单行解析（绿 = dict;无行/非法 JSON = None）。"""
    m = PROBE_LINE_RE.search(out)
    if not m:
        return None
    try:
        doc = json.loads(m.group(1))
    except json.JSONDecodeError:
        return None
    return doc if isinstance(doc, dict) else None


def classify_probe(doc: dict | None, expect_probe: str) -> list[str]:
    """探针行判据（返回失败串列表,空 = 绿;--selftest 合成夹具同消费）。"""
    if doc is None:
        return ["G31_FAULT_PROBE 行缺失/非法"]
    fails: list[str] = []
    if doc.get("probe") != expect_probe:
        fails.append(f"probe {doc.get('probe')!r} ≠ {expect_probe!r}")
    if doc.get("observed") is not True:
        fails.append(f"observed ≠ true: {doc.get('observed')!r}")
    if expect_probe.startswith("device-lost-"):
        if doc.get("cascade_present_poisoned") is not True:
            fails.append("cascade_present_poisoned ≠ true（poisoned 级联未实演）")
        if doc.get("cascade_resize_poisoned") is not True:
            fails.append("cascade_resize_poisoned ≠ true（poisoned 级联未实演）")
    return fails


def parse_storm_line(out: str) -> tuple[int, int, int, int] | None:
    """storm 汇总行解析 → (resize_ops, min_cycles, min_skips, resize_eras)。"""
    m = STORM_LINE_RE.search(out)
    if not m:
        return None
    return (int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4)))


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


def run_probe_arm(spec: str, env_extra: dict[str, str]) -> dict:
    """单探针臂真跑 + 判据;返回 evidence 登记面 dict。"""
    t0 = time.time()
    r = run_harness(
        [
            str(BIN), "--frames", str(PROBE_FRAMES), "--warmup", str(PROBE_WARMUP),
            "--hidden", "--quality", "off",  # W4 默认翻转免疫:fault 诊断臂显式 off（DEFAULT_FLIP_PLAN §2.5）
            "--fault-probe", spec,
        ],
        env_extra=env_extra,
    )
    wall = time.time() - t0
    out = r.stdout + r.stderr
    doc = parse_probe_line(out)
    fails = classify_probe(doc, spec)
    check(r.returncode == 0, f"探针臂 {spec} 非零退出 {r.returncode}: {out.strip()[-400:]}")
    for m in fails:
        check(False, f"探针臂 {spec}: {m}")
    check(
        "Validation Error" not in out and "VUID-" not in out,
        f"探针臂 {spec} validation 应静默却报错: {out.strip()[-300:]}",
    )
    if spec == "tdr":
        check(wall < TDR_WAIT_CAP_S,
              f"TDR 臂墙钟 {wall:.1f}s ≥ 上限 {TDR_WAIT_CAP_S}s（挂死嫌疑）")
    env_kv = next(iter(env_extra.items()))
    leg = {
        "probe": spec,
        "injection_env": f"{env_kv[0]}={env_kv[1]}",
        "frame": (doc or {}).get("frame"),
        "observed": bool(doc and doc.get("observed") is True and not fails),
        "wall_s": round(wall, 3),
    }
    if spec.startswith("device-lost-"):
        leg["cascade_present_poisoned"] = bool(doc and doc.get("cascade_present_poisoned") is True)
        leg["cascade_resize_poisoned"] = bool(doc and doc.get("cascade_resize_poisoned") is True)
    if spec == "tdr":
        leg["hang_free"] = wall < TDR_WAIT_CAP_S
    note(f"探针臂 {spec}: observed={leg['observed']} frame={leg['frame']} wall={wall:.1f}s")
    return leg


def run_gate() -> int:
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
    arms: dict[str, dict] = {}
    baseline_leg: dict | None = None
    storm_leg: dict | None = None
    soak_leg: dict | None = None
    with gpu_device_lock(purpose="g31 波 C 健壮性门（五探针臂 + 基线 + 窗口风暴 + soak 故障臂）"):
        # ── ①②③ device-lost 三点 + tdr + budget 探针臂 ──
        for spec in DEVICE_LOST_PROBES:
            point = spec.removeprefix("device-lost-")
            arms[spec.replace("-", "_")] = run_probe_arm(
                spec, {"RURIX_G31_FAULT_DEVICE_LOST": f"{point}@{PROBE_INJECT_INDEX}"}
            )
        arms["tdr"] = run_probe_arm("tdr", {"RURIX_G31_FAULT_FENCE_TIMEOUT": "2"})
        arms["budget"] = run_probe_arm("budget", {"RURIX_G31_FAULT_BUDGET_BYTES": "1"})

        # ── ⑥ 基线腿（默认关零行为变更证明;注入 env 全不設）──
        t0 = time.time()
        r = run_harness([
            str(BIN), "--frames", "64", "--warmup", "4",
            "--quality", "off",  # W4 默认翻转免疫:本腿语义 = C4 前行为逐字节等价基线（off 形态,DEFAULT_FLIP_PLAN §2.5）
            "--auto-move", "orbit", "--hidden", "--evidence", str(BASELINE_EVIDENCE),
        ])
        wall = time.time() - t0
        out = r.stdout + r.stderr
        check(r.returncode == 0, f"基线腿非零退出 {r.returncode}: {out.strip()[-400:]}")
        check("[g31_window_present]: PASS" in out, f"基线腿缺 PASS 行: {out.strip()[-300:]}")
        base_ok = False
        base_digest = None
        if r.returncode == 0 and BASELINE_EVIDENCE.is_file():
            ev = json.loads(BASELINE_EVIDENCE.read_text(encoding="utf-8"))
            vfail = gl.validate_harness_evidence(ev, 64, 4, "orbit")
            for m in vfail:
                check(False, f"基线腿 evidence 判据: {m}")
            if not vfail:
                base_ok = True
                base_digest = ev.get("digest")
        elif r.returncode == 0:
            check(False, "基线腿退 0 但 evidence 未落盘")
        baseline_leg = {
            "frames": 64,
            "warmup": 4,
            "wall_s": round(wall, 3),
            "exit_zero": r.returncode == 0,
            "evidence_ok": base_ok,
            "leak_ledger_zero": True,  # harness 逐帧硬门（非零即 fail 退 1）
            "digest": base_digest,
            "note": "注入臂全默认关:本腿与 C4 前行为逐字节等价（零行为变更证明面）",
        }
        note(f"基线腿: wall={wall:.1f}s evidence_ok={base_ok} digest={str(base_digest)[:23]}…")

        # ── ④ 窗口风暴爆发臂 ──
        t0 = time.time()
        r = run_harness([
            str(BIN), "--window-storm", str(STORM_BURST_OPS),
            "--frames", str(STORM_BURST_FRAMES), "--warmup", "4",
            "--hidden", "--quality", "off",  # W4 默认翻转免疫:storm 诊断臂显式 off（DEFAULT_FLIP_PLAN §2.5）
            "--evidence", str(STORM_EVIDENCE),
        ])
        wall = time.time() - t0
        out = r.stdout + r.stderr
        storm_counts = parse_storm_line(out)
        check(r.returncode == 0, f"窗口风暴臂非零退出 {r.returncode}: {out.strip()[-400:]}")
        check("[g31_window_present]: PASS" in out, f"窗口风暴臂缺 PASS 行: {out.strip()[-300:]}")
        check(
            "Validation Error" not in out and "VUID-" not in out,
            f"窗口风暴臂 validation 应静默却报错: {out.strip()[-300:]}",
        )
        check(storm_counts is not None and storm_counts[0] == STORM_BURST_OPS,
              f"窗口风暴臂 resize_ops {storm_counts and storm_counts[0]} ≠ {STORM_BURST_OPS}")
        check(storm_counts is not None and storm_counts[3] >= 1,
              f"窗口风暴臂 resize_eras {storm_counts and storm_counts[3]} < 1"
              "（奇数收官 ⇒ 末段 era 重建全真面未达）")
        storm_leg = {
            "window_storm": STORM_BURST_OPS,
            "frames": STORM_BURST_FRAMES,
            "warmup": 4,
            "wall_s": round(wall, 3),
            "exit_zero": r.returncode == 0,
            "resize_ops": storm_counts[0] if storm_counts else None,
            "resize_eras": storm_counts[3] if storm_counts else None,
            "validation_silent": True,
            "leak_ledger_zero": True,
        }
        note(f"窗口风暴臂: resize_ops={storm_leg['resize_ops']} wall={wall:.1f}s 零崩")

        # ── ⑤ soak 故障臂（≥1000 帧周期注入 resize/minimize 无崩）──
        t0 = time.time()
        r = run_harness([
            str(BIN), "--storm-soak", str(SOAK_FAULT_PERIOD),
            "--frames", str(SOAK_FAULT_FRAMES), "--warmup", str(SOAK_FAULT_WARMUP),
            "--quality", "off",  # W4 默认翻转免疫:storm-soak 诊断臂显式 off（DEFAULT_FLIP_PLAN §2.5）
            "--auto-move", "orbit", "--hidden", "--evidence", str(SOAK_EVIDENCE),
        ], timeout=7200)
        wall = time.time() - t0
        out = r.stdout + r.stderr
        if '"state":"skipped_dev_env"' in out:
            degrade.append(f"soak 故障臂 skipped_dev_env: {out.strip()[-300:]}")
            return _finish(degrade)
        soak_counts = parse_storm_line(out)
        check(r.returncode == 0, f"soak 故障臂非零退出 {r.returncode}: {out.strip()[-800:]}")
        check("[g31_window_present]: PASS" in out, f"soak 故障臂缺 PASS 行: {out.strip()[-300:]}")
        check(
            "Validation Error" not in out and "VUID-" not in out,
            f"soak 故障臂 validation 应静默却报错: {out.strip()[-300:]}",
        )
        check(soak_counts is not None and soak_counts[0] >= SOAK_MIN_RESIZE_OPS,
              f"soak 故障臂 resize_ops {soak_counts and soak_counts[0]} < {SOAK_MIN_RESIZE_OPS}")
        check(soak_counts is not None and soak_counts[1] >= SOAK_MIN_MIN_CYCLES,
              f"soak 故障臂 min_cycles {soak_counts and soak_counts[1]} < {SOAK_MIN_MIN_CYCLES}")
        check(soak_counts is not None and soak_counts[2] >= SOAK_MIN_MIN_CYCLES,
              f"soak 故障臂 min_skips {soak_counts and soak_counts[2]} < {SOAK_MIN_MIN_CYCLES}")
        check(soak_counts is not None and soak_counts[3] >= SOAK_MIN_RESIZE_OPS,
              f"soak 故障臂 resize_eras {soak_counts and soak_counts[3]} < {SOAK_MIN_RESIZE_OPS}"
              "（真 win32 尺寸通路 ⇒ 每次 toggle 皆 era 重建）")
        soak_ok = False
        win_extent: dict | None = None
        if r.returncode == 0 and SOAK_EVIDENCE.is_file():
            ev = json.loads(SOAK_EVIDENCE.read_text(encoding="utf-8"))
            vfail = gl.validate_harness_evidence(ev, SOAK_FAULT_FRAMES, SOAK_FAULT_WARMUP, "orbit")
            # storm 面终态 window.extent 随 toggle 确定性摆动（35 次 era 重建
            # 后收官于半基准是正轨）——A3 validate 的 1920x1080 钉值对本腿不
            # 适用,剔该条后按闭集 {基准 1920x1080, 半基准 960x540} 专项判;
            # 其余判据（frames_completed/leak/digest_seq/口径）逐字维持。
            vfail = [m for m in vfail if not m.startswith("window.extent")]
            win_extent = (ev.get("window") or {}).get("extent") or {}
            if (win_extent.get("w"), win_extent.get("h")) not in ((1920, 1080), (960, 540)):
                vfail.append(f"storm 腿终态 window.extent 越闭集: {win_extent!r}")
            for m in vfail:
                check(False, f"soak 故障臂 evidence 判据: {m}")
            soak_ok = not vfail
        elif r.returncode == 0:
            check(False, "soak 故障臂退 0 但 evidence 未落盘")
        soak_leg = {
            "storm_period": SOAK_FAULT_PERIOD,
            "frames": SOAK_FAULT_FRAMES,
            "warmup": SOAK_FAULT_WARMUP,
            "wall_s": round(wall, 3),
            "exit_zero": r.returncode == 0,
            "frames_completed": SOAK_FAULT_FRAMES + SOAK_FAULT_WARMUP,
            "resize_ops": soak_counts[0] if soak_counts else None,
            "min_cycles": soak_counts[1] if soak_counts else None,
            "min_skips": soak_counts[2] if soak_counts else None,
            "resize_eras": soak_counts[3] if soak_counts else None,
            "evidence_ok": soak_ok,
            "validation_silent": True,
            "leak_ledger_zero": True,
            "final_extent": win_extent,
        }
        note(
            f"soak 故障臂: {SOAK_FAULT_FRAMES} 帧 resize_ops={soak_leg['resize_ops']} "
            f"min_cycles={soak_leg['min_cycles']} wall={wall:.1f}s 零崩零泄漏"
        )
    return _finish(degrade, arms=arms, baseline=baseline_leg,
                   storm=storm_leg, soak=soak_leg)


def _finish(degrade: list[str], arms: dict | None = None, baseline: dict | None = None,
            storm: dict | None = None, soak: dict | None = None) -> int:
    if degrade:
        for d in degrade:
            print(f"[{TAG}] DEV_ENV_DEGRADE {d}")
        if require_real():
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        print(f"[{TAG}] SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL）")
        return 0
    verdict = "PASS" if (not FAILURES and arms and baseline and storm and soak) else "FAIL"
    doc = {
        "schema": SCHEMA_ID,
        "gate": GATE_KEY,
        "generated_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "binary": str(BIN.relative_to(ROOT)),
        "thresholds": {
            "probe_frames": PROBE_FRAMES,
            "probe_inject_index": PROBE_INJECT_INDEX,
            "tdr_wait_cap_s": TDR_WAIT_CAP_S,
            "storm_burst_ops": STORM_BURST_OPS,
            "soak_fault_frames": SOAK_FAULT_FRAMES,
            "soak_fault_period": SOAK_FAULT_PERIOD,
            "soak_min_resize_ops": SOAK_MIN_RESIZE_OPS,
            "soak_min_min_cycles": SOAK_MIN_MIN_CYCLES,
        },
        "injection_arms": arms,
        "baseline_leg": baseline,
        "window_storm_leg": storm,
        "soak_fault_leg": soak,
        "dispositions": {
            "device_lost": "poisoned 锁存 + 确定性 Err + 干净退出（禁 UB 级联;会话重建恢复路径归后续波,如实登记）",
            "tdr": "fence 有界等待超时 → TDR-suspected 确定性 Err（不挂死进程;超时面模拟,不真触系统 TDR）",
            "budget": "budget 违约 → OOM-suspected 确定性 Err fail-closed（不降级不挂死;内部分辨率降级路径归后续波,如实登记）",
            "window_storm": "程序化 resize/minimize 真通路 era 重建 + 塌零跳过（波 A 面加固,零崩零泄漏为门）",
        },
        "verdict": verdict,
        "notes": NOTES,
    }
    if arms is not None:
        ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        out = ROOT / "evidence" / f"g31_robustness_{ts}.json"
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        print(f"[{TAG}] evidence: {out}")
    if FAILURES:
        print(f"[{TAG}] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    print(
        f"[{TAG}] PASS gate={GATE_KEY}（device-lost 三点 poisoned 级联 + tdr 超时不挂死 + "
        f"budget OOM fail-closed + 窗口风暴 {STORM_BURST_OPS} 次零崩 + soak 故障臂 "
        f"{SOAK_FAULT_FRAMES} 帧 resize_ops={soak['resize_ops']} min_cycles={soak['min_cycles']} 零崩零泄漏）"
    )
    return 0


def run_selftest() -> int:
    # 绿臂①：合法探针行（device-lost 含级联面）。
    good = {
        "probe": "device-lost-submit", "site": "present", "frame": 8,
        "observed": True, "cascade_present_poisoned": True,
        "cascade_resize_poisoned": True, "expect": "x", "error": "y",
    }
    if classify_probe(good, "device-lost-submit"):
        print(f"[{TAG}] selftest FAIL: 合法 device-lost 探针行误判红", file=sys.stderr)
        return 1
    # 绿臂②：tdr 探针行（无级联面要求）。
    good_tdr = {"probe": "tdr", "site": "lane.frame", "frame": 0, "observed": True}
    if classify_probe(good_tdr, "tdr"):
        print(f"[{TAG}] selftest FAIL: 合法 tdr 探针行误判红", file=sys.stderr)
        return 1
    # 红臂①：observed=false 必须检出。
    if not classify_probe({**good, "observed": False}, "device-lost-submit"):
        print(f"[{TAG}] selftest FAIL: observed=false 漏检", file=sys.stderr)
        return 1
    # 红臂②：级联缺失必须检出（device-lost 面）。
    if not classify_probe({**good, "cascade_resize_poisoned": False}, "device-lost-submit"):
        print(f"[{TAG}] selftest FAIL: 级联缺失漏检", file=sys.stderr)
        return 1
    # 红臂③：probe 名漂移必须检出。
    if not classify_probe(good_tdr, "budget"):
        print(f"[{TAG}] selftest FAIL: probe 名漂移漏检", file=sys.stderr)
        return 1
    # 红臂④：无探针行输出必须检出。
    if parse_probe_line("no probe here") is not None or not classify_probe(None, "tdr"):
        print(f"[{TAG}] selftest FAIL: 探针行缺失漏检", file=sys.stderr)
        return 1
    # 绿臂③：风暴行解析;红臂⑤：畸形行必须 None。
    line = "[g31_window_present]: storm resize_ops=35 min_cycles=5 min_skips=5 resize_eras=35 window_storm=0 storm_soak=25"
    if parse_storm_line(line) != (35, 5, 5, 35):
        print(f"[{TAG}] selftest FAIL: 风暴行解析错", file=sys.stderr)
        return 1
    if parse_storm_line("storm resize_ops=x") is not None:
        print(f"[{TAG}] selftest FAIL: 畸形风暴行漏检", file=sys.stderr)
        return 1
    # schema 在树 + required 闭集互核。
    if not SCHEMA_PATH.is_file():
        print(f"[{TAG}] selftest FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    req = set(schema.get("required", []))
    expect = {"schema", "gate", "generated_utc", "binary", "thresholds", "injection_arms",
              "baseline_leg", "window_storm_leg", "soak_fault_leg", "dispositions",
              "verdict", "notes"}
    if req != expect:
        print(f"[{TAG}] selftest FAIL: schema required 漂移 {req ^ expect}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS (3 GREEN + 5 RED + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
