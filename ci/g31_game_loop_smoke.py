#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 A Task A3）
"""G31+ 波 A Task A3 游戏循环最小面 + device 侧显示编码门冒烟（g31.waveA.gameloop）。

harness = `src/rurix-render/src/bin/g31_window_present.rs --auto-move <orbit|dolly>`
（g14_3_lane_body 逐字共享统一四 pass TSR 车道 0-byte；第五 pass
`kernels/g31_display_encode.rx` device 侧 ACES1.3 RRT+ODT + BT.1886 编码——A1 host
逐像素 f64 编码瓶颈（实测 645ms/帧 release / 2864ms/帧 debug @1080p）与逐帧 24.9MB
f32 回读瓶颈的消除位：TSR 输出驻留 device，host 仅回读 BGRA8 8.3MB 供 present
拷贝/digest；确定性脚本轨迹经 192B 帧参数 + 128B TSR 参数逐帧 uniform 通路驱动
相机/曝光）。本冒烟：

1. **构建必绿**：`cargo build --release -p rurix-render --features vendor-upscale
   --bin g31_window_present`（release = bench 同 profile，性能口径与 bench 可比；
   构建面如实登记）。
2. **schema 互核**：milestones/g31/g31_game_loop_evidence_schema.json 在树且其
   required 闭集与本脚本校验键集精确互核（防 schema/校验两侧静默漂移）。
3. **device 真跑**（持 gpu_device_lock 串行，RURIX_VK_VALIDATION=1，release 产物）：
   - run A/B：`--auto-move orbit --frames 8 --warmup 2 --hidden` 双跑 →
     **digest_seq 逐帧位级一致**（确定性门）；
   - run C：`--auto-move dolly` 同参 → **digest_seq ≠ A**（异轨迹不同——相机真实
     生效门，防"确定性的坏内容"，G14.10f 教训面）；
   - run D：`--auto-move orbit --ev100-ramp -4.0 -2.0` → **digest_seq ≠ A** 且
     ev100_seq 逐帧 == 坡值（逐帧曝光 uniform 通路真实工作门）；
   - run A evidence 逐项判（字段闭集/类型/digest 形态/序列长度与末项一致/口径
     恒等式/计数/转引 consistency=pass）。
4. **三态纪律**：无 GPU/Vulkan/场景资产/SPV/窗口创建失败 → harness 自报
   `skipped_dev_env`（退 0）→ 本冒烟输出 `DEV_ENV_DEGRADE` 三态之 SKIP（**禁冒充
   PASS**）；`RURIX_REQUIRE_REAL=1` 下 SKIP 翻硬 FAIL。harness 非零退出 = FAIL。
5. **--selftest**（合成夹具红绿自证，不依赖树上文件）：合法 evidence 合成件必须
   绿；七类构造缺陷（缺字段/digest 篡改/序列长度不符/末项不等/poses 长度不符/
   ev100 坡不符/确定性比较器漏判）必须逐条红——证 validate 面真判红非摆设。

用法：
  py -3 ci/g31_game_loop_smoke.py --gate g31.waveA.gameloop
  py -3 ci/g31_game_loop_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_game_loop_evidence_schema.json"
WORK_DIR = ROOT / ".tmp" / "g31_gates" / "waveA_gameloop"
BIN = "g31_window_present"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveA.gameloop"
TAG = "g31_game_loop"
SCHEMA_ID = "rurix.g31.game_loop_evidence.v1"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
KERNEL_SRC = ROOT / "src" / "rurix-render" / "kernels"
SPV_FILES = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
)
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")

# 顶层 required 键闭集（与 schema 文件 properties/required 互核面）。
REQUIRED_KEYS = [
    "schema",
    "gate",
    "scene",
    "tier",
    "backend",
    "trajectory",
    "frames",
    "warmup",
    "frames_completed",
    "exit_reason",
    "resize_eras",
    "resolution",
    "internal_resolution",
    "real_render_frame_ms",
    "present_frame_ms",
    "present_overhead_ms",
    "encode_frame_ms",
    "digest_frame_ms",
    "render_digest",
    "digest",
    "digest_seq",
    "ev100_seq",
    "camera_poses",
    "ev100_ramp",
    "headless",
    "window",
    "contracts",
    "render_includes_forced_readback",
    "spv",
    "stats",
    "notes",
]


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def run_cmd(argv: list[str], timeout: int = 3600, env: dict | None = None) -> subprocess.CompletedProcess:
    print(f"[{TAG}] $ {' '.join(argv)}")
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(argv), "exit_code": r.returncode})
    return r


def ensure_encode_spv() -> bool:
    """encode kernel SPV 存在性保障（缺则经 rurixc --target vulkan 现编；.tmp 构建产物
    不入 git,源 = kernels/g31_display_encode.rx）。四车道 SPV 缺失 = degrade（由
    g14_rurix_pipeline_perf_smoke 门保障面,本门不重编）。"""
    enc = SPV_DIR / "g31_display_encode.spv"
    if enc.is_file():
        return True
    src = KERNEL_SRC / "g31_display_encode.rx"
    if not src.is_file():
        return False
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    if not rurixc.is_file():
        r = run_cmd(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"], timeout=7200)
        if r.returncode != 0 or not rurixc.is_file():
            return False
    enc.parent.mkdir(parents=True, exist_ok=True)
    r = run_cmd([str(rurixc), str(src), "--target", "vulkan", "-o", str(enc)], timeout=1800)
    return r.returncode == 0 and enc.is_file()


def compare_digest_seqs_same(a: list, b: list) -> list[str]:
    """确定性门：同轨迹双跑 digest_seq 必须逐帧位级一致（返回失败串,空 = 绿）。"""
    fails: list[str] = []
    if len(a) != len(b):
        return [f"digest_seq 长度 {len(a)} ≠ {len(b)}"]
    diff = [k for k, (x, y) in enumerate(zip(a, b)) if x != y]
    if diff:
        fails.append(f"digest_seq 双跑位级不一致:首异帧 {diff[0]}（共 {len(diff)} 帧异）")
    return fails


def compare_digest_seqs_diff(a: list, b: list) -> list[str]:
    """异轨迹/异曝光门：digest_seq 必须至少一帧不同（返回失败串,空 = 绿）。"""
    if len(a) != len(b):
        return []  # 长度不同即内容不同,门成立
    if all(x == y for x, y in zip(a, b)):
        return ["digest_seq 全等——异轨迹/异曝光面疑似冒充（相机/曝光未真实生效?）"]
    return []


def validate_harness_evidence(ev: dict, expect_frames: int, expect_warmup: int, trajectory: str) -> list[str]:
    """harness evidence 逐项判（返回失败串列表,空 = 绿;--selftest 合成夹具同消费）。"""
    fails: list[str] = []
    if not isinstance(ev, dict):
        return ["evidence 非 object"]
    for k in REQUIRED_KEYS:
        if k not in ev:
            fails.append(f"缺顶层字段 {k}")
    if fails:
        return fails
    extra = set(ev) - set(REQUIRED_KEYS)
    if extra:
        fails.append(f"闭集外字段 {sorted(extra)}")
    if ev.get("schema") != SCHEMA_ID:
        fails.append(f"schema ≠ {SCHEMA_ID}: {ev.get('schema')!r}")
    if ev.get("gate") != GATE_KEY:
        fails.append(f"gate ≠ {GATE_KEY}: {ev.get('gate')!r}")
    if ev.get("scene") != "bistro-interior":
        fails.append(f"scene ≠ bistro-interior: {ev.get('scene')!r}")
    if ev.get("backend") != "tsr_device":
        fails.append(f"backend ≠ tsr_device: {ev.get('backend')!r}")
    if ev.get("trajectory") != trajectory:
        fails.append(f"trajectory ≠ {trajectory}: {ev.get('trajectory')!r}")
    if ev.get("frames") != expect_frames:
        fails.append(f"frames {ev.get('frames')} ≠ 命令行 {expect_frames}")
    if ev.get("warmup") != expect_warmup:
        fails.append(f"warmup {ev.get('warmup')} ≠ 命令行 {expect_warmup}")
    total = expect_frames + expect_warmup
    if ev.get("frames_completed") != total:
        fails.append(f"frames_completed {ev.get('frames_completed')} ≠ frames+warmup {total}")
    if ev.get("exit_reason") != "frames_done":
        fails.append(f"exit_reason ≠ frames_done: {ev.get('exit_reason')!r}")
    if not isinstance(ev.get("resize_eras"), int) or ev.get("resize_eras", -1) < 0:
        fails.append(f"resize_eras 非 ≥0 int: {ev.get('resize_eras')!r}")
    if ev.get("tier") not in (50, 67, 100):
        fails.append(f"tier 越闭集: {ev.get('tier')!r}")
    res = ev.get("resolution") or {}
    if (res.get("w"), res.get("h")) != (1920, 1080):
        fails.append(f"resolution ≠ 1920x1080: {res!r}")
    ires = ev.get("internal_resolution") or {}
    tier = ev.get("tier")
    if isinstance(tier, int) and (ires.get("w"), ires.get("h")) != (1920 * tier // 100, 1080 * tier // 100):
        fails.append(f"internal_resolution {ires!r} ≠ floor(输出×tier%)")
    rr = ev.get("real_render_frame_ms")
    if not isinstance(rr, (int, float)) or isinstance(rr, bool) or not rr > 0:
        fails.append(f"real_render_frame_ms 非正数: {rr!r}")
    em = ev.get("encode_frame_ms")
    if not isinstance(em, (int, float)) or isinstance(em, bool) or not em >= 0:
        fails.append(f"encode_frame_ms 非 ≥0 数: {em!r}")
    dm = ev.get("digest_frame_ms")
    if not isinstance(dm, (int, float)) or isinstance(dm, bool) or not dm >= 0:
        fails.append(f"digest_frame_ms 非 ≥0 数: {dm!r}")
    for dk in ("render_digest", "digest"):
        if not isinstance(ev.get(dk), str) or not DIGEST_RE.match(ev[dk]):
            fails.append(f"{dk} 形态非法: {str(ev.get(dk))[:40]!r}")
    seq = ev.get("digest_seq")
    if not isinstance(seq, list) or len(seq) != total:
        fails.append(f"digest_seq 非数组或长度 ≠ {total}: {type(seq).__name__}")
        seq = []
    elif any(not isinstance(x, str) or not DIGEST_RE.match(x) for x in seq):
        fails.append("digest_seq 含非法 digest 形态项")
    if seq and ev.get("digest") != seq[-1]:
        fails.append("digest ≠ digest_seq 末项（末帧 digest 与序列脱节）")
    ev100 = ev.get("ev100_seq")
    if not isinstance(ev100, list) or len(ev100) != total:
        fails.append(f"ev100_seq 非数组或长度 ≠ {total}")
        ev100 = []
    elif any(not isinstance(x, (int, float)) or isinstance(x, bool) for x in ev100):
        fails.append("ev100_seq 含非数值项")
    poses = ev.get("camera_poses")
    if not isinstance(poses, list) or len(poses) != total:
        fails.append(f"camera_poses 非数组或长度 ≠ {total}")
    elif any(
        not isinstance(p, list) or len(p) != 5 or any(not isinstance(v, (int, float)) or isinstance(v, bool) for v in p)
        for p in poses
    ):
        fails.append("camera_poses 含非 [f64;5] 项")
    ramp = ev.get("ev100_ramp")
    if ramp is not None:
        if not isinstance(ramp, dict) or set(ramp) != {"a", "b"}:
            fails.append(f"ev100_ramp 形态非法: {ramp!r}")
        elif ev100:
            a, b = ramp["a"], ramp["b"]
            for k, v in enumerate(ev100):
                want = a + (b - a) * (k / total)
                if abs(v - want) > 1e-9:
                    fails.append(f"ev100_seq[{k}]={v} ≠ 坡值 {want}（逐帧曝光 uniform 面判红）")
                    break
    headless = ev.get("headless")
    if not isinstance(headless, bool):
        fails.append(f"headless 非 bool: {headless!r}")
        headless = True
    win = ev.get("window")
    pm = ev.get("present_frame_ms")
    om = ev.get("present_overhead_ms")
    if headless is False:
        if not isinstance(win, dict):
            fails.append("headless=false 但 window 非 object（无窗口冒充真门?）")
            win = {}
        if not isinstance(pm, (int, float)) or isinstance(pm, bool) or not pm > 0:
            fails.append(f"headless=false 但 present_frame_ms 非正数: {pm!r}")
        if not isinstance(om, (int, float)) or isinstance(om, bool) or not om > 0:
            fails.append(f"headless=false 但 present_overhead_ms 非正数: {om!r}")
        if isinstance(pm, (int, float)) and isinstance(om, (int, float)) and not isinstance(pm, bool):
            if om + 1e-6 < pm:
                fails.append(f"口径恒等式破坏:present_overhead_ms {om} < present_frame_ms {pm}")
        if win.get("channel_order") not in ("bgra8_unorm", "rgba8_unorm"):
            fails.append(f"channel_order 越闭集: {win.get('channel_order')!r}")
        ext = win.get("extent") or {}
        if (ext.get("w"), ext.get("h")) != (1920, 1080):
            fails.append(f"window.extent ≠ 1920x1080: {ext!r}")
        fp = win.get("frames_presented")
        if fp != total:
            fails.append(f"frames_presented {fp} ≠ frames+warmup {total}")
        sr = win.get("swapchain_rebuilds")
        if not isinstance(sr, int) or sr < 0:
            fails.append(f"swapchain_rebuilds 非 ≥0 int: {sr!r}")
    else:
        if win is not None:
            fails.append("headless=true 但 window 非 null（自检面冒充真窗?）")
        if pm is not None or om is not None:
            fails.append("headless=true 但 present 口径非 null（自检面冒充真门?）")
    contracts = ev.get("contracts") or {}
    if contracts.get("consistency") != "pass":
        fails.append(f"contracts.consistency ≠ pass: {contracts.get('consistency')!r}")
    prod = contracts.get("production") or {}
    if not isinstance(prod.get("digest"), str) or not DIGEST_RE.match(prod["digest"]):
        fails.append(f"production.digest 形态非法: {str(prod.get('digest'))[:40]!r}")
    for gk in ("g10_contract", "g10_camera", "g10_lighting", "encode_spv"):
        g = contracts.get(gk) or {}
        if not isinstance(g.get("sha256"), str) or not DIGEST_RE.match(g["sha256"]):
            fails.append(f"{gk}.sha256 形态非法: {str(g.get('sha256'))[:40]!r}")
    if ev.get("render_includes_forced_readback") is not True:
        fails.append("render_includes_forced_readback ≠ true（口径登记缺失）")
    stats = ev.get("stats") or {}
    for sk in ("render_cv", "render_min_ms", "render_max_ms", "encode_gpu_ms"):
        if not isinstance(stats.get(sk), (int, float)) or isinstance(stats.get(sk), bool):
            fails.append(f"stats.{sk} 非数值: {stats.get(sk)!r}")
    if not isinstance(ev.get("notes"), str) or not ev["notes"]:
        fails.append("notes 空（口径注释面缺失）")
    return fails


def good_fixture(frames: int = 8, warmup: int = 2, trajectory: str = "orbit", ramp: bool = False) -> dict:
    """合法 evidence 合成夹具（数字为占位形态值,自证 validate 绿臂——不进任何 evidence）。"""
    d = "sha256:" + "0" * 64
    total = frames + warmup
    ev = {
        "schema": SCHEMA_ID,
        "gate": GATE_KEY,
        "scene": "bistro-interior",
        "tier": 100,
        "backend": "tsr_device",
        "trajectory": trajectory,
        "frames": frames,
        "warmup": warmup,
        "frames_completed": total,
        "exit_reason": "frames_done",
        "resize_eras": 0,
        "resolution": {"w": 1920, "h": 1080},
        "internal_resolution": {"w": 1920, "h": 1080},
        "real_render_frame_ms": 5.0,
        "present_frame_ms": 2.0,
        "present_overhead_ms": 2.0,
        "encode_frame_ms": 0.0,
        "digest_frame_ms": 30.0,
        "render_digest": d,
        "digest": d,
        "digest_seq": [d] * total,
        "ev100_seq": [-4.0] * total,
        "camera_poses": [[0.0, 0.0, 0.0, 0.0, 0.0]] * total,
        "ev100_ramp": None,
        "headless": False,
        "window": {
            "visible": False,
            "channel_order": "bgra8_unorm",
            "extent": {"w": 1920, "h": 1080},
            "frames_presented": total,
            "swapchain_rebuilds": 0,
        },
        "contracts": {
            "production": {"path": "x.json", "digest": d},
            "g10_contract": {"path": "a.json", "sha256": d},
            "g10_camera": {"path": "b.json", "sha256": d},
            "g10_lighting": {"path": "c.json", "sha256": d},
            "consistency": "pass",
            "delta_note": "synthetic",
            "encode_spv": {"path": "e.spv", "sha256": d},
        },
        "render_includes_forced_readback": True,
        "spv": {"kind": "tsr_device"},
        "stats": {
            "render_cv": 0.01,
            "render_min_ms": 4.9,
            "render_max_ms": 5.1,
            "encode_gpu_ms": 0.11,
            "present_cv": 0.02,
            "present_min_ms": 1.9,
            "present_max_ms": 2.1,
        },
        "notes": "synthetic green fixture",
    }
    if ramp:
        ev["ev100_ramp"] = {"a": -4.0, "b": -2.0}
        ev["ev100_seq"] = [-4.0 + 2.0 * (k / total) for k in range(total)]
    return ev


def run_selftest() -> int:
    frames, warmup = 8, 2
    green = validate_harness_evidence(good_fixture(frames, warmup), frames, warmup, "orbit")
    if green:
        print(f"[{TAG}] selftest FAIL: 合法夹具误判红 {green}", file=sys.stderr)
        return 1
    green_ramp = validate_harness_evidence(
        good_fixture(frames, warmup, ramp=True), frames, warmup, "orbit"
    )
    if green_ramp:
        print(f"[{TAG}] selftest FAIL: ramp 合法夹具误判红 {green_ramp}", file=sys.stderr)
        return 1
    reds: list[tuple[str, dict]] = []
    bad = good_fixture(frames, warmup)
    del bad["digest_seq"]
    reds.append(("缺顶层字段", bad))
    bad = good_fixture(frames, warmup)
    bad["digest"] = "sha256:" + "f" * 63
    reds.append(("digest 形态篡改", bad))
    bad = good_fixture(frames, warmup)
    bad["digest_seq"] = bad["digest_seq"][:-1]
    reds.append(("digest_seq 长度不符", bad))
    bad = good_fixture(frames, warmup)
    bad["digest_seq"][-1] = "sha256:" + "1" * 64
    reds.append(("digest 与序列末项脱节", bad))
    bad = good_fixture(frames, warmup)
    bad["camera_poses"] = bad["camera_poses"][:-1]
    reds.append(("poses 长度不符", bad))
    bad = good_fixture(frames, warmup, ramp=True)
    bad["ev100_seq"][3] = 99.0
    reds.append(("ev100 坡值不符", bad))
    bad = good_fixture(frames, warmup)
    bad["headless"] = True
    reds.append(("headless 冒充", bad))
    missed = []
    for name, fx in reds:
        got = validate_harness_evidence(fx, frames, warmup, "orbit")
        if not got:
            missed.append(name)
    if missed:
        print(f"[{TAG}] selftest FAIL: 红臂漏检 {missed}", file=sys.stderr)
        return 1
    # 确定性/异轨迹比较器红绿（绿:同序列 same 门过、异序列 diff 门过;红:反之）。
    d0 = "sha256:" + "0" * 64
    d1 = "sha256:" + "1" * 64
    if compare_digest_seqs_same([d0, d0], [d0, d0]):
        print(f"[{TAG}] selftest FAIL: same 门绿臂误判红", file=sys.stderr)
        return 1
    if not compare_digest_seqs_same([d0, d1], [d0, d0]):
        print(f"[{TAG}] selftest FAIL: same 门红臂漏检", file=sys.stderr)
        return 1
    if compare_digest_seqs_diff([d0, d1], [d0, d0]):
        print(f"[{TAG}] selftest FAIL: diff 门绿臂误判红", file=sys.stderr)
        return 1
    if not compare_digest_seqs_diff([d0, d0], [d0, d0]):
        print(f"[{TAG}] selftest FAIL: diff 门红臂漏检（全等序列冒充异轨迹）", file=sys.stderr)
        return 1
    # schema 文件互核:required 闭集 == REQUIRED_KEYS。
    if not SCHEMA_PATH.is_file():
        print(f"[{TAG}] selftest FAIL: schema 文件缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    req = set(schema.get("required", []))
    if req != set(REQUIRED_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与校验键集不等 {req ^ set(REQUIRED_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS (2 GREEN + {len(reds)} RED + 比较器 4 象限 + schema 互核)")
    return 0


def run_harness(exe: Path, trajectory: str, frames: int, warmup: int, ev_path: Path, ramp: tuple[float, float] | None, env: dict) -> tuple[subprocess.CompletedProcess, str]:
    argv = [
        str(exe),
        "--frames", str(frames),
        "--warmup", str(warmup),
        "--hidden",
        "--auto-move", trajectory,
        "--evidence", str(ev_path),
    ]
    if ramp is not None:
        argv += ["--ev100-ramp", str(ramp[0]), str(ramp[1])]
    r = run_cmd(argv, timeout=1800, env=env)
    return r, r.stdout + r.stderr


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=8)
    ap.add_argument("--warmup", type=int, default=2)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    # ① schema 在树 + required 闭集互核。
    check(SCHEMA_PATH.is_file(), f"schema 文件缺失: {SCHEMA_PATH}")
    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        check(
            set(schema.get("required", [])) == set(REQUIRED_KEYS),
            f"schema required 与校验键集不等: {set(schema.get('required', [])) ^ set(REQUIRED_KEYS)}",
        )

    # ② 构建必绿（release = bench 同 profile,性能口径与 bench 可比,如实登记）。
    r = run_cmd([
        "cargo", "build", "--release", "-p", "rurix-render", "--features", "vendor-upscale",
        "--bin", BIN, "--quiet",
    ])
    check(r.returncode == 0, f"harness 构建失败: {(r.stdout + r.stderr)[-600:]}")
    exe = ROOT / "target" / "release" / f"{BIN}{EXE_SUFFIX}"
    check(exe.is_file(), f"产物缺失: {exe}")

    # ③ device 前置面（SPV/场景资产缺失 = DEV_ENV_DEGRADE 登记,不冒充 FAIL 也不 PASS）。
    degrade_reasons: list[str] = []
    if not ensure_encode_spv():
        degrade_reasons.append("g31_display_encode.spv 缺失且 rurixc 现编失败（.tmp 构建产物）")
    missing_spv = [f for f in SPV_FILES if not (SPV_DIR / f).is_file()]
    if missing_spv:
        degrade_reasons.append(f"SPV 缺失 {missing_spv}（.tmp 构建产物,CI 需先备 kernel 编译面）")
    if not BISTRO_GLTF.is_file():
        degrade_reasons.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    # ④ device 真跑（持锁;validation 开）：orbit 双跑确定性 + dolly 异轨迹 +
    #    orbit+ramp 异曝光三面。
    ran = False
    if not FAILURES and not degrade_reasons:
        WORK_DIR.mkdir(parents=True, exist_ok=True)
        env = dict(os.environ)
        env["RURIX_VK_VALIDATION"] = "1"
        ev_a = WORK_DIR / "orbit_run1.json"
        ev_b = WORK_DIR / "orbit_run2.json"
        ev_c = WORK_DIR / "dolly_run.json"
        ev_d = WORK_DIR / "orbit_ramp_run.json"
        with gpu_device_lock(purpose="g31 waveA gameloop device 腿"):
            ra, outa = run_harness(exe, "orbit", args.frames, args.warmup, ev_a, None, env)
            rb, outb = (None, "")
            rc, outc = (None, "")
            rd, outd = (None, "")
            if '"state":"skipped_dev_env"' not in outa and ra.returncode == 0:
                rb, outb = run_harness(exe, "orbit", args.frames, args.warmup, ev_b, None, env)
                rc, outc = run_harness(exe, "dolly", args.frames, args.warmup, ev_c, None, env)
                rd, outd = run_harness(exe, "orbit", args.frames, args.warmup, ev_d, (-4.0, -2.0), env)
        outs = [outa, outb or "", outc or "", outd or ""]
        if '"state":"skipped_dev_env"' in outa:
            degrade_reasons.append(f"harness skipped_dev_env: {outa.strip()[-300:]}")
        else:
            for tag, rr, out in (("orbit#1", ra, outa), ("orbit#2", rb, outb), ("dolly", rc, outc), ("orbit+ramp", rd, outd)):
                if rr is None:
                    check(False, f"{tag} 未执行（前序运行失败）")
                    continue
                check(rr.returncode == 0, f"{tag} harness 非零退出 {rr.returncode}: {out.strip()[-800:]}")
                check("[g31_window_present]: PASS" in out, f"{tag} 缺 PASS 行: {out.strip()[-400:]}")
                check(
                    "Validation Error" not in out and "VUID-" not in out,
                    f"{tag} validation 应静默却报错: {out.strip()[-400:]}",
                )
            evs: list[dict | None] = []
            for tag, p in (("orbit#1", ev_a), ("orbit#2", ev_b), ("dolly", ev_c), ("orbit+ramp", ev_d)):
                if not p.is_file():
                    check(False, f"{tag} evidence 未落盘: {p}")
                    evs.append(None)
                    continue
                try:
                    evs.append(json.loads(p.read_text(encoding="utf-8")))
                except json.JSONDecodeError as e:
                    check(False, f"{tag} evidence 不可解析: {e}")
                    evs.append(None)
            ea, eb, ec, ed = evs
            if ea is not None:
                vfail = validate_harness_evidence(ea, args.frames, args.warmup, "orbit")
                for m in vfail:
                    check(False, f"orbit#1 evidence 判据: {m}")
                if not vfail:
                    ran = True
            if eb is not None:
                for m in validate_harness_evidence(eb, args.frames, args.warmup, "orbit"):
                    check(False, f"orbit#2 evidence 判据: {m}")
            if ec is not None:
                for m in validate_harness_evidence(ec, args.frames, args.warmup, "dolly"):
                    check(False, f"dolly evidence 判据: {m}")
            if ed is not None:
                for m in validate_harness_evidence(ed, args.frames, args.warmup, "orbit"):
                    check(False, f"orbit+ramp evidence 判据: {m}")
            # 三面门:双跑位级一致 / 异轨迹不同 / 异曝光不同。
            if ea is not None and eb is not None:
                for m in compare_digest_seqs_same(ea["digest_seq"], eb["digest_seq"]):
                    check(False, f"确定性门: {m}")
            if ea is not None and ec is not None:
                for m in compare_digest_seqs_diff(ea["digest_seq"], ec["digest_seq"]):
                    check(False, f"异轨迹门: {m}")
            if ea is not None and ed is not None:
                for m in compare_digest_seqs_diff(ea["digest_seq"], ed["digest_seq"]):
                    check(False, f"异曝光门: {m}")
            if ran and ea is not None:
                note(
                    f"真跑口径: real_render={ea['real_render_frame_ms']:.3f}ms "
                    f"present={ea['present_frame_ms']:.3f}ms "
                    f"encode_gpu={ea['stats']['encode_gpu_ms']:.3f}ms "
                    f"digest={ea['digest_frame_ms']:.3f}ms seq[0]={ea['digest_seq'][0][:23]}…"
                )

    for m in NOTES:
        print(f"[{TAG}] NOTE {m}")
    if degrade_reasons:
        for d in degrade_reasons:
            print(f"[{TAG}] DEV_ENV_DEGRADE {d}")
        if require_real():
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        print(f"[{TAG}] SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL;构建/selftest 面仍真跑）")
        return 0
    if FAILURES:
        print(f"[{TAG}] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    if not ran:
        print(f"[{TAG}] FAIL: device 腿未真跑（无 degrade 原因但无真跑证据）", file=sys.stderr)
        return 1
    print(
        f"[{TAG}] PASS gate={GATE_KEY}（release 构建绿 + schema 互核 + orbit 双跑 digest_seq "
        f"位级一致 + dolly 异轨迹不同 + orbit+ramp 异曝光不同 + evidence 闭集判据全绿）"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
