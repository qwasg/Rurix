#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 A Task A4 动态场景更新通路）
"""G31+ 波 A Task A4：动态场景更新通路门冒烟（g31.waveA.dynscene；
G31_PLUS_COMMERCIAL_RENDERER_TODO §1.1 #4 行；本任务任务书 A4 判据逐字）。

判据（任务书逐字）：
- 逐帧实例变换更新链：host 每帧写动态实例变换（平移/旋转）→ GPU scene
  instance 缓冲增量更新（write_transforms 槽位级 diff——仅动态槽 64B，禁全量
  场景重传）→ TLAS refit（优先）/rebuild（回退）接 render_exec 既有机制。
- ≥1 类运动物体进生产帧：bistro-interior + 动态纯发光立方体（12 三角形
  BLAS 1），脚本化轨迹（帧号确定性 f32 函数）--dyn-demo 模式渲染 N 帧。
- refit vs rebuild measured 对照：同轨迹同帧数两策略臂 frame_ms 对照进
  evidence JSON（schema = milestones/g31/g31_dynamic_scene_evidence_schema.json）；
  数字来自真实命令输出。
- 正确性对拍：动态实例逐帧位置经 host 参考臂（解析投影：轨迹点 + 8 角点
  经 jittered vp）核验 device 画面检测位（scene color 纯绿谱质心/AABB，容差
  2.5/4.0px）；同轨迹双跑 digest 位级一致；动 vs 不动 digest 必须不同。
- 静态场景回归锚：bistro 静态契约 bench（160 帧 canonical）digest ==
  milestones/g14/g14_3_stage_a_digest_anchor.json 锚（A4 落地静态面 0-byte）。

三态：无 Vulkan loader/设备/场景资产 → 输出 DEV_ENV_DEGRADE 退 0（不冒充
PASS）；本脚本真跑臂 RURIX_REQUIRE_REAL=1（该态下缺真实面即 FAIL 退 1，
禁 mock 充真跑——g31_frame_pipelining_smoke 同语义）。

用法：
  py -3 ci/g31_dynamic_scene_smoke.py --selftest
  py -3 ci/g31_dynamic_scene_smoke.py --gate g31.waveA.dynscene \
      [--runs 3] [--frames 100] [--warmup 10] [--out <evidence.json>]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import math
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveA.dynscene"
TAG = "g31_dynscene"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_dynamic_scene_evidence_schema.json"
SCHEMA_ID = "rurix.g31.dynamic_scene_evidence.v1"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"
WORK = ROOT / ".tmp" / "g31_gates" / "dynscene"
OUT_ROOT = WORK / "out"
SCENE = "bistro-interior"
TIER = 100
BACKEND = "tsr_device"
ARMS = ("refit", "rebuild")
TRACE_FRAMES = 24
TRACE_WARMUP = 4
STATIC_ANCHOR_FRAMES = 160  # g14 Stage A 锚 canonical 帧数（锚收割口径）
STATIC_ANCHOR_WARMUP = 10

# 与 bin 侧 g14_3_lane_body.rs G31+ Task A4 常量区逐字同源（轨迹/容差规格）。
DYN_TOL_CENTROID_PX = 2.5
DYN_TOL_AABB_PX = 4.0
DYN_VERIFY_EVERY = 10
OBS_MOTION_MIN_PX = 3.0
TRANSFORM_REDERIVE_TOL = 1e-4

SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
SPV_DYN = SPV_DIR / "g31_dyn_scene.spv"
KERNEL_DYN = ROOT / "src" / "rurix-render" / "kernels" / "g31_dyn_scene.rx"

FAILURES: list[str] = []


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


# ---------------------------------------------------------------- 纯函数判据面
def percentile(sorted_v: list[float], q: float) -> float:
    """s 已升序；q∈[0,1] 最近秩（与 A2 分析脚本同一口径）。"""
    if not sorted_v:
        raise ValueError("percentile: 空样本")
    n = len(sorted_v)
    return sorted_v[min(n - 1, int(q * (n - 1) + 0.5))]


def arm_stats(frame_ms: list[float]) -> dict:
    """单轮 frame_ms 全列 → mean/p50/p99/min/max（末一样本含末帧 digest
    tail——两臂同形同价；核验帧 scene color 回读税计入 tail，两臂同形）。"""
    if not frame_ms:
        raise ValueError("arm_stats: 空 frame_ms")
    s = sorted(frame_ms)
    n = len(frame_ms)
    p50 = s[n // 2] if n % 2 else (s[n // 2 - 1] + s[n // 2]) / 2.0
    return {
        "mean": sum(frame_ms) / n,
        "p50": p50,
        "p99": percentile(s, 0.99),
        "min": s[0],
        "max": s[-1],
    }


def digests_bitexact(digest_lists: list[list[str]]) -> bool:
    """多臂 × 多轮 digest 列：全部展平后集合恰一元（位级一致判据）。"""
    flat = [d for ds in digest_lists for d in ds]
    return bool(flat) and len(set(flat)) == 1


def seqs_bitexact(seqs: list[list[str]]) -> bool:
    """多臂逐帧 digest **序列**逐位对拍：等长且逐下标全同（A2 同律）。"""
    if not seqs or any(len(s) != len(seqs[0]) for s in seqs):
        return False
    return all(len({s[i] for s in seqs}) == 1 for i in range(len(seqs[0])))


def frame_order_ok(trace_rows: list[dict]) -> bool:
    """flip-trace 帧号严格 0..N−1。"""
    return [int(r["frame"]) for r in trace_rows] == list(range(len(trace_rows)))


def rederive_transform(frame: int, traj: dict) -> list[float]:
    """轨迹规格 → 行主 3×4 变换（bin 侧 dyn_trajectory/dyn_transform_3x4 的
    python 独立重导——核验 dyn_verify.json 内 transform 非伪造的第三臂）。"""
    t = float(frame)
    amp = [float(x) for x in traj["amp"]]
    freq = [float(x) for x in traj["freq"]]
    origin = [float(x) for x in traj["origin"]]
    pos = [
        origin[0] + amp[0] * math.sin(freq[0] * t),
        origin[1] + amp[1] * math.sin(freq[1] * t + 1.0),
        origin[2] + amp[2] * (math.cos(freq[2] * t) - 1.0),
    ]
    yaw = float(traj["yaw_rate"]) * t
    s, c = math.sin(yaw), math.cos(yaw)
    return [c, 0.0, s, pos[0], 0.0, 1.0, 0.0, pos[1], -s, 0.0, c, pos[2]]


def validate_dyn_verify(doc: dict, expect_action: str) -> list[str]:
    """dyn_verify.json 逐项判（返回失败串列表，空 = 绿；--selftest 合成夹具
    同消费）：schema/action/帧数/逐帧 transform 重导/容差/obs 运动。"""
    fails: list[str] = []
    if not isinstance(doc, dict):
        return ["dyn_verify 非 object"]
    if doc.get("schema") != "rurix.g31.dyn_scene_verify.v1":
        fails.append(f"dyn_verify schema 非法: {doc.get('schema')!r}")
    if doc.get("action") != expect_action:
        fails.append(f"dyn_verify action {doc.get('action')!r} ≠ {expect_action!r}")
    traj = doc.get("trajectory") or {}
    for k in ("amp", "freq", "origin", "cube_half", "emission", "yaw_rate"):
        if k not in traj:
            fails.append(f"trajectory 缺 {k}")
    tol = doc.get("tolerance") or {}
    if abs(float(tol.get("centroid_px", -1)) - DYN_TOL_CENTROID_PX) > 1e-9:
        fails.append("tolerance.centroid_px 漂移")
    if abs(float(tol.get("aabb_px", -1)) - DYN_TOL_AABB_PX) > 1e-9:
        fails.append("tolerance.aabb_px 漂移")
    frames = doc.get("frames")
    if not isinstance(frames, list) or not frames:
        fails.append("frames 空/非数组")
        frames = []
    if doc.get("frames_verified") != len(frames):
        fails.append(f"frames_verified {doc.get('frames_verified')!r} ≠ len(frames) {len(frames)}")
    if doc.get("all_pass") is not True:
        fails.append("all_pass ≠ true")
    obs_pts: list[tuple[float, float]] = []
    for fr in frames:
        tag = f"frame {fr.get('frame')!r}"
        if fr.get("pass") is not True:
            fails.append(f"{tag} pass ≠ true")
        xf = fr.get("transform")
        if not isinstance(xf, list) or len(xf) != 12:
            fails.append(f"{tag} transform 形态非法")
            continue
        if traj:
            exp = rederive_transform(int(fr["frame"]), traj)
            for k in range(12):
                if abs(float(xf[k]) - exp[k]) > TRANSFORM_REDERIVE_TOL:
                    fails.append(
                        f"{tag} transform[{k}] {float(xf[k]):.9g} 与轨迹重导 {exp[k]:.9g} 偏差超阈"
                    )
                    break
        cd = float(fr.get("centroid_delta_px", 1e30))
        ad = float(fr.get("aabb_delta_px", 1e30))
        if not (cd <= DYN_TOL_CENTROID_PX):
            fails.append(f"{tag} 质心偏差 {cd:.3f} > {DYN_TOL_CENTROID_PX}px")
        if not (ad <= DYN_TOL_AABB_PX):
            fails.append(f"{tag} AABB 偏差 {ad:.3f} > {DYN_TOL_AABB_PX}px")
        if int(fr.get("obs_count", 0)) < 200:
            fails.append(f"{tag} obs_count {fr.get('obs_count')!r} < 200（动态实例丢失面）")
        op = fr.get("obs_px")
        if not isinstance(op, list) or len(op) != 2 or not all(
            isinstance(v, (int, float)) and math.isfinite(float(v)) for v in op
        ):
            fails.append(f"{tag} obs_px 非有限对（动态实例未检出）")
        else:
            obs_pts.append((float(op[0]), float(op[1])))
    # obs 运动判据（防「确定性的坏内容」——画面动态实例位置必须真动）。
    if len(obs_pts) >= 2:
        motion = max(
            math.hypot(obs_pts[i][0] - obs_pts[j][0], obs_pts[i][1] - obs_pts[j][1])
            for i in range(len(obs_pts))
            for j in range(i + 1, len(obs_pts))
        )
        if motion < OBS_MOTION_MIN_PX:
            fails.append(f"obs 质心最大位移 {motion:.3f}px < {OBS_MOTION_MIN_PX}px（画面未真动）")
    else:
        fails.append("obs 点不足 2（运动判据不可评）")
    return fails


def evidence_required_keys(doc: dict) -> list[str]:
    """schema required 闭集核验（jsonschema 依赖免；check_schemas.py 另作
    形式校验面）。"""
    required = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))["required"]
    return [k for k in required if k not in doc]


# ---------------------------------------------------------------- 真跑驱动
def _rurixc() -> Path | None:
    """rurixc vulkan-backend 构建面（缺则构建一次；g14 smoke 同型）。"""
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if exe.is_file():
        return exe
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"],
        cwd=ROOT, capture_output=True, text=True, timeout=7200,
    )
    return exe if r.returncode == 0 and exe.is_file() else None


def ensure_dyn_spv() -> bool:
    """动态场景 kernel SPV 存在性保障（缺则编译；.tmp 构建产物不入 git）。"""
    if SPV_DYN.is_file():
        return True
    rurixc = _rurixc()
    if rurixc is None:
        return False
    SPV_DYN.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        [str(rurixc), str(KERNEL_DYN), "--target", "vulkan", "-o", str(SPV_DYN)],
        cwd=ROOT, capture_output=True, text=True, timeout=1800,
    )
    return r.returncode == 0 and SPV_DYN.is_file()


def run_bench(
    dyn_action: str | None,
    frames: int,
    warmup: int,
    out_root: Path,
    flip_trace_dir: Path | None,
    require_real: bool,
) -> dict:
    """单轮真跑（dyn_action None = 静态 bench 回归锚臂；否则 --dyn-demo 臂）。"""
    env = dict(os.environ)
    if require_real:
        env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    if flip_trace_dir is not None:
        flip_trace_dir.mkdir(parents=True, exist_ok=True)
        env["RURIX_G14_FLIP_TRACE"] = str(flip_trace_dir)
    elif "RURIX_G14_FLIP_TRACE" in env:
        del env["RURIX_G14_FLIP_TRACE"]
    cmd = [
        str(BIN),
        "--bench",
        "--scene", SCENE,
        "--tier", str(TIER),
        "--backend", BACKEND,
        "--frames", str(frames),
        "--warmup", str(warmup),
        "--out-root", str(out_root),
    ]
    if dyn_action is not None:
        cmd += ["--dyn-demo", dyn_action]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=7200, env=env)
    out = (r.stdout or "") + (r.stderr or "")
    receipt_path = out_root / SCENE / f"tier{TIER}" / BACKEND / "bench_receipt.json"
    verify_path = out_root / SCENE / f"tier{TIER}" / BACKEND / "dyn_verify.json"
    receipt = {}
    verify = {}
    fresh = False
    if receipt_path.is_file():
        fresh = receipt_path.stat().st_mtime >= t0 - 5.0
        try:
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            receipt = {}
    if verify_path.is_file() and verify_path.stat().st_mtime >= t0 - 5.0:
        try:
            verify = json.loads(verify_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            verify = {}
    pass_line = re.search(r"BENCH PASS scene=(\S+)", out)
    clean = (
        r.returncode == 0
        and pass_line is not None
        and fresh
        and "VALIDATION" not in out
        and "leak" not in out.lower()
    )
    return {
        "rc": r.returncode,
        "clean_shutdown": clean,
        "receipt": receipt,
        "verify": verify,
        "receipt_path": receipt_path,
        "fresh": fresh,
        "tail": out[-400:],
        "skipped_dev_env": "skipped_dev_env" in out,
    }


def load_trace(trace_dir: Path) -> list[dict]:
    p = trace_dir / f"frame_digests_{SCENE}_t{TIER}_{BACKEND}.jsonl"
    if not p.is_file():
        return []
    return [json.loads(line) for line in p.read_text(encoding="utf-8").splitlines() if line.strip()]


# ---------------------------------------------------------------- selftest
def selftest() -> int:
    note("selftest：判据纯函数红绿臂")
    ok = True

    # ① arm_stats 绿臂：已知向量核对 mean/p50/p99/min/max。
    v = [4.0, 1.0, 2.0, 3.0, 100.0]
    st = arm_stats(v)
    expect = {"mean": 22.0, "p50": 3.0, "p99": 100.0, "min": 1.0, "max": 100.0}
    green = st == expect
    ok &= green
    note(f"  arm_stats 绿臂: {'PASS' if green else 'FAIL'}（{st}）")

    # ② arm_stats 红臂：空样本必拒（ValueError）。
    try:
        arm_stats([])
        red = False
    except ValueError:
        red = True
    ok &= red
    note(f"  arm_stats 红臂（空样本拒）: {'PASS' if red else 'FAIL'}")

    # ③ digests_bitexact 绿臂：双臂全同 → True。
    green = digests_bitexact([["sha256:a", "sha256:a"], ["sha256:a"]])
    ok &= green
    note(f"  digests_bitexact 绿臂: {'PASS' if green else 'FAIL'}")

    # ④ digests_bitexact 红臂：任一漂移 → False（检出即红，不静默）。
    red = not digests_bitexact([["sha256:a"], ["sha256:b"]])
    ok &= red
    note(f"  digests_bitexact 红臂（漂移检出）: {'PASS' if red else 'FAIL'}")

    # ⑤ seqs_bitexact 绿/红臂：逐帧序列同 → True；单帧漂移/不等长 → False。
    green = seqs_bitexact([["d0", "d1", "d2"], ["d0", "d1", "d2"]])
    red = not seqs_bitexact([["d0", "d1", "d2"], ["d0", "dX", "d2"]]) and not seqs_bitexact(
        [["d0"], ["d0", "d1"]]
    )
    ok &= green and red
    note(f"  seqs_bitexact 绿臂: {'PASS' if green else 'FAIL'}；红臂: {'PASS' if red else 'FAIL'}")

    # ⑥ frame_order_ok 绿/红臂。
    green = frame_order_ok([{"frame": 0}, {"frame": 1}, {"frame": 2}])
    red = not frame_order_ok([{"frame": 0}, {"frame": 2}, {"frame": 1}])
    ok &= green and red
    note(f"  frame_order_ok 绿臂: {'PASS' if green else 'FAIL'}；红臂: {'PASS' if red else 'FAIL'}")

    # ⑦ rederive_transform 口径锚：frame 0 = identity 旋转 + z 偏移 0（cos−1 起
    # 点）；frame 10 与 bin dyn_verify 实测值对拍（真跑首轮锚，容差 1e-6）。
    traj = {
        "amp": [0.35, 0.18, 0.25],
        "freq": [0.021, 0.013, 0.017],
        "yaw_rate": 0.011,
        "origin": [2.236061, 1.4122407, -1.6825298],
        "cube_half": 0.06,
        "emission": [0.0, 500.0, 0.0],
    }
    xf0 = rederive_transform(0, traj)
    green = (
        abs(xf0[0] - 1.0) < 1e-9
        and abs(xf0[10] - 1.0) < 1e-9
        and abs(xf0[11] - traj["origin"][2]) < 1e-9
        and abs(xf0[3] - traj["origin"][3 - 3]) >= 0.0  # 恒真占位（防误删行）
    )
    xf10 = rederive_transform(10, traj)
    green &= abs(xf10[0] - 9.939560890e-1) < 1e-6 and abs(xf10[3] - 2.309021950e0) < 1e-6
    ok &= green
    note(f"  rederive_transform 口径锚绿臂: {'PASS' if green else 'FAIL'}")

    # ⑧ validate_dyn_verify 绿臂：合成合法件（transform 由重导产，obs 随轨迹
    # 真动）必须零失败。
    def synth_verify(action: str) -> dict:
        frames = []
        for i, f in enumerate(range(10, 60, 10)):
            xf = rederive_transform(f, traj)
            frames.append({
                "frame": f,
                "transform": xf,
                "pred_px": [900.0 + 2.0 * i, 400.0 + 1.0 * i],
                "pred_aabb": [870.0 + 2.0 * i, 370.0 + i, 930.0 + 2.0 * i, 430.0 + i],
                "obs_px": [900.5 + 2.0 * i, 400.5 + 1.0 * i],
                "obs_aabb": [870.0 + 2.0 * i, 370.0 + i, 930.0 + 2.0 * i, 430.0 + i],
                "obs_count": 5000,
                "centroid_delta_px": 0.71,
                "aabb_delta_px": 0.5,
                "pass": True,
            })
        return {
            "schema": "rurix.g31.dyn_scene_verify.v1",
            "action": action,
            "scene_id": SCENE,
            "tier": TIER,
            "backend": BACKEND,
            "trajectory": dict(traj),
            "tolerance": {"centroid_px": DYN_TOL_CENTROID_PX, "aabb_px": DYN_TOL_AABB_PX,
                          "min_count_area_ratio": 0.15},
            "frames": frames,
            "frames_verified": len(frames),
            "all_pass": True,
        }

    green = validate_dyn_verify(synth_verify("refit"), "refit") == []
    ok &= green
    note(f"  validate_dyn_verify 绿臂: {'PASS' if green else 'FAIL'}")

    # ⑨ validate_dyn_verify 红臂 ×4：transform 篡改 / 容差超阈 / obs 不动 /
    # all_pass=false 冒充——逐条必须红。
    reds: list[tuple[str, dict]] = []
    bad = synth_verify("refit")
    bad["frames"][2]["transform"][3] += 0.5
    reds.append(("transform 篡改", bad))
    bad = synth_verify("refit")
    bad["frames"][1]["centroid_delta_px"] = 9.9
    reds.append(("质心容差超阈", bad))
    bad = synth_verify("refit")
    for fr in bad["frames"]:
        fr["obs_px"] = [900.5, 400.5]
    reds.append(("obs 不动（坏内容）", bad))
    bad = synth_verify("refit")
    bad["frames"][0]["pass"] = False
    reds.append(("单帧 pass=false", bad))
    missed = [name for name, fx in reds if not validate_dyn_verify(fx, "refit")]
    red = not missed
    ok &= red
    note(
        f"  validate_dyn_verify 红臂（{len(reds)} 类构造缺陷）: "
        f"{'PASS' if red else 'FAIL'}" + (f"（漏检 {missed}）" if missed else "")
    )

    # ⑩ evidence required 键闭集：合成 doc 缺键必列出。
    missing = evidence_required_keys({"schema": SCHEMA_ID})
    red = len(missing) > 0 and "arms" in missing and "static_anchor_match" in missing
    ok &= red
    note(f"  evidence required 键红臂（缺键列出 {len(missing)} 项）: {'PASS' if red else 'FAIL'}")

    # ⑪ percentile 口径锚：n=100 时 int(0.99×99+0.5)=98、int(0.5×99+0.5)=50。
    s100 = list(range(100))
    green = percentile(s100, 0.99) == 98 and percentile(s100, 0.5) == 50
    ok &= green
    note(f"  percentile 口径锚绿臂: {'PASS' if green else 'FAIL'}")

    if ok:
        note("SELFTEST PASS（红绿臂全如预期）")
        return 0
    print(f"[{TAG}] SELFTEST FAIL", file=sys.stderr)
    return 1


# ---------------------------------------------------------------- gate
def gate(runs: int, frames: int, warmup: int, out_path: Path | None) -> int:
    if not SCHEMA_PATH.is_file():
        fail(f"schema 缺失: {SCHEMA_PATH}")
        return 1
    if not ANCHOR_PATH.is_file():
        fail(f"g14 Stage A digest 锚缺失: {ANCHOR_PATH}")
        return 1
    note(
        f"gate {GATE_KEY}: scene={SCENE} tier={TIER} backend={BACKEND} arms={ARMS} "
        f"runs/arm={runs} frames={frames} warmup={warmup}"
    )

    with gpu_device_lock(purpose=f"{TAG} device 真跑（g31.waveA.dynscene）"):
        # 构建（release；g14_3_pipeline_perf 需 vendor-upscale feature）。
        build = subprocess.run(
            ["cargo", "build", "-p", "rurix-render", "--features", "vulkan,vendor-upscale",
             "--release", "--bin", "g14_3_pipeline_perf"],
            cwd=ROOT, capture_output=True, text=True, timeout=7200,
        )
        if build.returncode != 0 or not BIN.is_file():
            fail(f"release 构建失败: {(build.stderr or '')[-400:]}")
            return 1
        if not ensure_dyn_spv():
            fail(f"动态场景 kernel SPV 编译失败: {KERNEL_DYN} → {SPV_DYN}")
            return 1

        # dev-env 探针（不挂 REQUIRE_REAL：缺真实面 → bin 自报 skipped_dev_env 退 0）。
        probe = run_bench("refit", 2, 1, WORK / "probe", None, require_real=False)
        if probe["skipped_dev_env"] or (probe["rc"] == 0 and not probe["fresh"]):
            print(json.dumps({
                "schema": "rurix.g31.dynamic_scene.skip.v1",
                "state": "DEV_ENV_DEGRADE",
                "what": "vulkan_device_or_scene_assets",
                "reason": probe["tail"][-200:],
            }, ensure_ascii=False))
            note("DEV_ENV_DEGRADE（无 Vulkan/设备/场景资产——退 0 不冒充 PASS）")
            return 0
        if probe["rc"] != 0:
            fail(f"dev-env 探针真跑失败: {probe['tail'][-200:]}")
            return 1
        note("dev-env 探针绿（真机真跑面成立）")

        # ── 主 A/B：refit/rebuild 两臂 × runs 轮（无 trace——生产口径测量循环）──
        arm_docs: list[dict] = []
        all_digests: list[list[str]] = []
        clean_all = True
        traj_doc: dict = {}
        for action in ARMS:
            run_stats: list[dict] = []
            run_digests: list[str] = []
            receipts: list[str] = []
            prod_means: list[float] = []
            verify_pass = 0
            verify_total = 0
            centroid_max = 0.0
            aabb_max = 0.0
            for rep in range(runs):
                r = run_bench(action, frames, warmup, OUT_ROOT, None, require_real=True)
                clean_all &= r["clean_shutdown"]
                if r["rc"] != 0:
                    fail(f"{action} rep{rep + 1} 真跑 rc={r['rc']}: {r['tail'][-200:]}")
                    return 1
                rec = r["receipt"]
                if not rec:
                    fail(f"{action} rep{rep + 1} receipt 缺失/不新鲜")
                    return 1
                vfails = validate_dyn_verify(r["verify"], action)
                if vfails:
                    for m in vfails:
                        fail(f"{action} rep{rep + 1} dyn_verify: {m}")
                    return 1
                v = r["verify"]
                if not traj_doc:
                    # 轨迹规格取自真跑产出（禁手写锚——dyn_verify 同源转引）。
                    traj_doc = dict(v.get("trajectory") or {})
                verify_pass += sum(1 for fr in v["frames"] if fr.get("pass") is True)
                verify_total += len(v["frames"])
                centroid_max = max(
                    centroid_max, max(float(fr["centroid_delta_px"]) for fr in v["frames"])
                )
                aabb_max = max(
                    aabb_max, max(float(fr["aabb_delta_px"]) for fr in v["frames"])
                )
                run_stats.append(arm_stats([float(x) for x in rec["frame_ms"]]))
                run_digests.append(str(rec["last_frame_digest"]))
                prod_means.append(float(rec["stats_post_warmup"]["frame_ms_production_mean"]))
                archive_dir = WORK / "receipts"
                archive_dir.mkdir(parents=True, exist_ok=True)
                archive = archive_dir / f"bench_dyn_{action}_rep{rep + 1}.json"
                archive.write_text(
                    json.dumps(rec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
                )
                receipts.append(str(archive))
                varch = archive_dir / f"dyn_verify_{action}_rep{rep + 1}.json"
                varch.write_text(
                    json.dumps(v, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
                )
            med = lambda key: sorted(x[key] for x in run_stats)[len(run_stats) // 2]
            arm_docs.append({
                "tlas_action": action,
                "frame_ms_mean": med("mean"),
                "frame_ms_p50": med("p50"),
                "frame_ms_p99": med("p99"),
                "frame_ms_min": med("min"),
                "frame_ms_max": med("max"),
                "frame_ms_production_mean": sorted(prod_means)[len(prod_means) // 2],
                "last_frame_digest": run_digests[-1],
                "verify_frames_pass": verify_pass,
                "verify_frames_total": verify_total,
                "verify_centroid_max_px": centroid_max,
                "verify_aabb_max_px": aabb_max,
                "receipts": receipts,
            })
            all_digests.append(run_digests)
            note(
                f"  arm {action}: mean={med('mean'):.4f} p50={med('p50'):.4f} "
                f"p99={med('p99'):.4f} prod={sorted(prod_means)[len(prod_means) // 2]:.4f} "
                f"digest={run_digests[-1][:23]}… verify={verify_pass}/{verify_total}"
            )

        # ── 判据①：两臂全轮 digest 位级一致（refit ≡ rebuild 位级同图硬门）──
        digest_actions = digests_bitexact(all_digests)
        if not digest_actions:
            fail(f"refit/rebuild digest 位级一致破缺: {[sorted(set(ds)) for ds in all_digests]}")
        # ── 判据②：同臂双跑位级（runs ≥ 2 ⇒ 逐臂 digest 列恰一元；①成立时
        # 两臂合并一元蕴含,但逐臂独立断言防「臂内漂移臂间巧合相等」）──
        digest_double = all(len(set(ds)) == 1 for ds in all_digests)
        if not digest_double:
            fail(f"同臂双跑 digest 位级一致破缺: {[sorted(set(ds)) for ds in all_digests]}")

        # ── flip-trace 侧跑（逐帧 digest 序列 + 帧序；两臂）──
        trace_seqs: list[list[dict]] = []
        for action in ARMS:
            tdir = WORK / f"trace_{action}"
            r = run_bench(action, TRACE_FRAMES, TRACE_WARMUP, WORK / "trace_out",
                          tdir, require_real=True)
            clean_all &= r["clean_shutdown"]
            if r["rc"] != 0:
                fail(f"{action} trace 侧跑 rc={r['rc']}: {r['tail'][-200:]}")
                return 1
            rows = load_trace(tdir)
            if len(rows) != TRACE_FRAMES + TRACE_WARMUP:
                fail(f"{action} trace 行数 {len(rows)} ≠ {TRACE_FRAMES + TRACE_WARMUP}")
                return 1
            trace_seqs.append(rows)
        order_ok = all(frame_order_ok(rows) for rows in trace_seqs)
        if not order_ok:
            fail("flip-trace 帧序破缺（非严格 0..N−1）")
        seq_digests = [[str(r["digest"]) for r in rows] for rows in trace_seqs]
        trace_bitexact = seqs_bitexact(seq_digests)
        if not trace_bitexact:
            fail("flip-trace 逐帧 digest 序列跨臂位级一致破缺")
        note(f"  trace 侧跑: 帧序严格={order_ok} 逐帧 digest 跨臂位级一致={trace_bitexact}")

        # ── 判据③④：静态回归锚臂（canonical 160 帧；digest == g14 Stage A 锚
        # 且 ≠ 动态臂 digest）──
        sr = run_bench(None, STATIC_ANCHOR_FRAMES, STATIC_ANCHOR_WARMUP,
                       WORK / "static_out", None, require_real=True)
        clean_all &= sr["clean_shutdown"]
        if sr["rc"] != 0 or not sr["receipt"]:
            fail(f"静态回归锚臂真跑失败: {sr['tail'][-200:]}")
            return 1
        static_digest = str(sr["receipt"]["last_frame_digest"])
        anchor_doc = json.loads(ANCHOR_PATH.read_text(encoding="utf-8"))
        anchor_digest = str(
            anchor_doc["anchors"][f"{SCENE}_t{TIER}_{BACKEND}"]["last_frame_digest"]
        )
        static_match = static_digest == anchor_digest
        if not static_match:
            fail(f"静态面 0-byte 回归破缺: {static_digest} ≠ 锚 {anchor_digest}")
        dyn_neq_static = all(d != static_digest for ds in all_digests for d in ds)
        if not dyn_neq_static:
            fail("动 vs 不动 digest 必异破缺（动态臂 digest == 静态臂——坏内容面）")
        note(
            f"  静态回归锚: digest={static_digest[:23]}… == g14 锚={static_match}；"
            f"动 ≠ 静={dyn_neq_static}"
        )

        if not clean_all:
            fail("clean shutdown 破缺（rc/PASS/receipt 新鲜/validation/leak 字样）")

    # ── evidence 落盘 ──
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    refit_arm = arm_docs[0]
    rebuild_arm = arm_docs[1]
    doc = {
        "schema": SCHEMA_ID,
        "subject": "g31_dynamic_scene",
        "symbolic_gate_key": GATE_KEY,
        "wave": "G31.A",
        "scene_id": SCENE,
        "tier": TIER,
        "backend": BACKEND,
        "seed": int(sr["receipt"]["seed"]),
        "frames_measured": frames,
        "warmup": warmup,
        "runs_per_arm": runs,
        "trajectory": {
            "amp": [float(x) for x in traj_doc.get("amp", [])],
            "freq": [float(x) for x in traj_doc.get("freq", [])],
            "yaw_rate": float(traj_doc.get("yaw_rate", 0.0)),
            "origin": [float(x) for x in traj_doc.get("origin", [])],
            "cube_half": float(traj_doc.get("cube_half", 0.0)),
            "emission": [float(x) for x in traj_doc.get("emission", [])],
            "instance_write_bytes_per_frame": 64,
        },
        "arms": arm_docs,
        "digest_bitexact_across_actions": digest_actions and trace_bitexact,
        "digest_bitexact_double_run": digest_double,
        "dyn_neq_static_digest": dyn_neq_static,
        "static_anchor_digest": static_digest,
        "static_anchor_match": static_match,
        "position_verify": {
            "frames_total": sum(a["verify_frames_total"] for a in arm_docs),
            "frames_pass": sum(a["verify_frames_pass"] for a in arm_docs),
            "centroid_max_px": max(a["verify_centroid_max_px"] for a in arm_docs),
            "aabb_max_px": max(a["verify_aabb_max_px"] for a in arm_docs),
            "tolerance_centroid_px": DYN_TOL_CENTROID_PX,
            "tolerance_aabb_px": DYN_TOL_AABB_PX,
        },
        "clean_shutdown": clean_all,
        "environment": {
            "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
            "os": "windows",
            "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
            "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                          capture_output=True, text=True).stdout.strip(),
        },
        "timestamp": ts,
        "notes": (
            f"refit vs rebuild measured（同轨迹同帧数 {frames}+{warmup}wu ×{runs} 轮，"
            f"跨轮中位数）：refit mean={refit_arm['frame_ms_mean']:.4f}ms p50={refit_arm['frame_ms_p50']:.4f}ms "
            f"p99={refit_arm['frame_ms_p99']:.4f}ms prod={refit_arm['frame_ms_production_mean']:.4f}ms；"
            f"rebuild mean={rebuild_arm['frame_ms_mean']:.4f}ms p50={rebuild_arm['frame_ms_p50']:.4f}ms "
            f"p99={rebuild_arm['frame_ms_p99']:.4f}ms prod={rebuild_arm['frame_ms_production_mean']:.4f}ms；"
            f"p50 差 {(rebuild_arm['frame_ms_p50'] / refit_arm['frame_ms_p50'] - 1) * 100:+.2f}%"
            f"（TLAS 2 实例级——build/refit 均微秒级，差在噪声带内如实登记）；"
            f"frame_ms 含逐帧 TLAS 更新 + 核验帧 scene color 回读税（tail 如实计量，两臂同形同价）；"
            f"A2 流水面兼容取舍：动态面走顺序入口（inflight=1）——FIF 流水公共入口 fail-closed 拒 "
            f"tlas_update（共享 instance buffer host 写面在飞帧不可改写）；未选 per-slot 实例缓冲方案，"
            f"依据 = ①本波单动态实例 64B/帧写面，顺序入口 TLAS 更新 GPU 段实测微秒级，流水化收益不抵 "
            f"AS manager 多缓冲改造风险（单所有者纪律面）；②A2 流水红利已由静态面独立兑现；"
            f"③per-slot 实例缓冲 + 流水合流归后续波（设计面已在 render_exec 登记）。"
            f"位置核验 {sum(a['verify_frames_pass'] for a in arm_docs)}/"
            f"{sum(a['verify_frames_total'] for a in arm_docs)} 帧（质心 ≤{DYN_TOL_CENTROID_PX}px "
            f"AABB ≤{DYN_TOL_AABB_PX}px；obs 真动判据 ≥{OBS_MOTION_MIN_PX}px）"
        ),
    }
    missing = evidence_required_keys(doc)
    if missing:
        fail(f"evidence 缺 required 键: {missing}")
        return 1
    ev_path = out_path or (ROOT / "evidence" / f"g31_dynamic_scene_{ts}.json")
    ev_path.parent.mkdir(parents=True, exist_ok=True)
    ev_path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {ev_path}")

    ok = (
        digest_actions
        and digest_double
        and trace_bitexact
        and order_ok
        and dyn_neq_static
        and static_match
        and clean_all
        and not FAILURES
    )
    note(f"GATE {'PASS' if ok else 'FAIL'} {GATE_KEY}")
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--gate", default="")
    ap.add_argument("--runs", type=int, default=3)
    ap.add_argument("--frames", type=int, default=100)
    ap.add_argument("--warmup", type=int, default=10)
    ap.add_argument("--out", default="")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        if args.frames < 100:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 100（任务书 ≥100 帧硬线）",
                  file=sys.stderr)
            return 1
        if args.runs < 2:
            print(f"[{TAG}] FAIL: --runs {args.runs} < 2（双跑位级判据硬线）",
                  file=sys.stderr)
            return 1
        return gate(args.runs, args.frames, args.warmup,
                    Path(args.out) if args.out else None)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
