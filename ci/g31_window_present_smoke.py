#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 A Task A1）
"""G31+ 波 A Task A1 生产管线 swapchain 真窗口呈现接线门冒烟（g31.waveA.present；
G31_PLUS_COMMERCIAL_RENDERER_TODO §1.1 #1 行）。

harness = `src/rurix-render/src/bin/g31_window_present.rs`（g14_3_lane_body 逐字共享统一四
pass TSR 车道；bistro-interior 1080p 契约 + milestones/g10/corpus 三件套转引一致性核验；
DisplayPipeline SDR+aces13 编码；`vk::ExternalImagePresent` win32 窗口真 swapchain
present）。本冒烟：

1. **构建必绿**：`cargo build -p rurix-render --features vendor-upscale --bin
   g31_window_present`（共享体/底座编译面）。
2. **schema 互核**：milestones/g31/g31_window_present_evidence_schema.json 在树且其
   required 闭集与本脚本校验键集精确互核（防 schema/校验两侧静默漂移）。
3. **device 真跑**（持 gpu_device_lock 串行，RURIX_VK_VALIDATION=1）：`--frames 3
   --warmup 1 --hidden`（--hidden = 非交互 runner 安全面；真 swapchain present 路径与
   可视窗同律，本地可视真跑登记于 evidence notes）→ harness 退 0 + PASS 行 + evidence
   落盘 → 字段闭集/类型/digest 形态/口径恒等式（present_overhead ≥ present_frame，
   encode ≥ 0）/计数（frames_presented == frames+warmup）/转引 consistency=pass 逐项判。
4. **三态纪律**：无 GPU/Vulkan/场景资产/SPV/窗口创建失败 → harness 自报
   `skipped_dev_env`（退 0）→ 本冒烟输出 `DEV_ENV_DEGRADE` 三态之 SKIP（**禁冒充
   PASS**）；`RURIX_REQUIRE_REAL=1` 下 SKIP 翻硬 FAIL。harness 非零退出 = FAIL。
5. **--selftest**（合成夹具红绿自证，不依赖树上文件）：合法 evidence 合成件必须绿；
   六类构造缺陷（缺字段/digest 篡改/口径恒等式破坏/headless 冒充/计数不符/转引失败）
   必须逐条红——证 validate 面真判红非摆设。

用法：
  py -3 ci/g31_window_present_smoke.py --gate g31.waveA.present
  py -3 ci/g31_window_present_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_window_present_evidence_schema.json"
WORK_DIR = ROOT / ".tmp" / "g31_gates" / "waveA"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"
BIN = "g31_window_present"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveA.present"
TAG = "g31_window_present"
SCHEMA_ID = "rurix.g31.window_present_evidence.v1"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
SPV_FILES = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
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
    "frames",
    "warmup",
    "resolution",
    "internal_resolution",
    "real_render_frame_ms",
    "present_frame_ms",
    "present_overhead_ms",
    "encode_frame_ms",
    "render_digest",
    "digest",
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


def validate_harness_evidence(ev: dict, expect_frames: int, expect_warmup: int) -> list[str]:
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
    if ev.get("frames") != expect_frames:
        fails.append(f"frames {ev.get('frames')} ≠ 命令行 {expect_frames}")
    if ev.get("warmup") != expect_warmup:
        fails.append(f"warmup {ev.get('warmup')} ≠ 命令行 {expect_warmup}")
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
    for dk in ("render_digest", "digest"):
        if not isinstance(ev.get(dk), str) or not DIGEST_RE.match(ev[dk]):
            fails.append(f"{dk} 形态非法: {str(ev.get(dk))[:40]!r}")
    headless = ev.get("headless")
    if not isinstance(headless, bool):
        fails.append(f"headless 非 bool: {headless!r}")
        headless = True  # 后续判按最严
    win = ev.get("window")
    pm = ev.get("present_frame_ms")
    om = ev.get("present_overhead_ms")
    if headless is False:
        # 真门面:window 非 null + present 口径非 null 且恒等式成立。
        if not isinstance(win, dict):
            fails.append("headless=false 但 window 非 object（无窗口冒充真门?）")
            win = {}
        if not isinstance(pm, (int, float)) or isinstance(pm, bool) or not pm > 0:
            fails.append(f"headless=false 但 present_frame_ms 非正数: {pm!r}")
        if not isinstance(om, (int, float)) or isinstance(om, bool) or not om > 0:
            fails.append(f"headless=false 但 present_overhead_ms 非正数: {om!r}")
        if isinstance(pm, (int, float)) and isinstance(om, (int, float)) and not isinstance(pm, bool):
            # 口径恒等式:overhead = encode + present ≥ present(encode ≥ 0;容 1e-6 浮点尾)。
            if om + 1e-6 < pm:
                fails.append(f"口径恒等式破坏:present_overhead_ms {om} < present_frame_ms {pm}")
        if win.get("channel_order") not in ("bgra8_unorm", "rgba8_unorm"):
            fails.append(f"channel_order 越闭集: {win.get('channel_order')!r}")
        ext = win.get("extent") or {}
        if (ext.get("w"), ext.get("h")) != (1920, 1080):
            fails.append(f"window.extent ≠ 1920x1080: {ext!r}")
        fp = win.get("frames_presented")
        if fp != expect_frames + expect_warmup:
            fails.append(f"frames_presented {fp} ≠ frames+warmup {expect_frames + expect_warmup}")
        sr = win.get("swapchain_rebuilds")
        if not isinstance(sr, int) or sr < 0:
            fails.append(f"swapchain_rebuilds 非 ≥0 int: {sr!r}")
    else:
        # 自检退化面:window/present 口径必须 null(冒充真门即红)。
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
    for gk in ("g10_contract", "g10_camera", "g10_lighting"):
        g = contracts.get(gk) or {}
        if not isinstance(g.get("sha256"), str) or not DIGEST_RE.match(g["sha256"]):
            fails.append(f"{gk}.sha256 形态非法: {str(g.get('sha256'))[:40]!r}")
    if ev.get("render_includes_forced_readback") is not True:
        fails.append("render_includes_forced_readback ≠ true（口径登记缺失）")
    stats = ev.get("stats") or {}
    for sk in ("render_cv", "render_min_ms", "render_max_ms"):
        if not isinstance(stats.get(sk), (int, float)) or isinstance(stats.get(sk), bool):
            fails.append(f"stats.{sk} 非数值: {stats.get(sk)!r}")
    if not isinstance(ev.get("notes"), str) or not ev["notes"]:
        fails.append("notes 空（口径注释面缺失）")
    return fails


def good_fixture(frames: int = 3, warmup: int = 1) -> dict:
    """合法 evidence 合成夹具（数字为占位形态值,自证 validate 绿臂——不进任何 evidence）。"""
    d = "sha256:" + "0" * 64
    return {
        "schema": SCHEMA_ID,
        "gate": GATE_KEY,
        "scene": "bistro-interior",
        "tier": 100,
        "backend": "tsr_device",
        "frames": frames,
        "warmup": warmup,
        "resolution": {"w": 1920, "h": 1080},
        "internal_resolution": {"w": 1920, "h": 1080},
        "real_render_frame_ms": 30.0,
        "present_frame_ms": 2.0,
        "present_overhead_ms": 42.0,
        "encode_frame_ms": 40.0,
        "render_digest": d,
        "digest": d,
        "headless": False,
        "window": {
            "visible": False,
            "channel_order": "bgra8_unorm",
            "extent": {"w": 1920, "h": 1080},
            "frames_presented": frames + warmup,
            "swapchain_rebuilds": 0,
        },
        "contracts": {
            "production": {"path": "x.json", "digest": d},
            "g10_contract": {"path": "a.json", "sha256": d},
            "g10_camera": {"path": "b.json", "sha256": d},
            "g10_lighting": {"path": "c.json", "sha256": d},
            "consistency": "pass",
            "delta_note": "synthetic",
        },
        "render_includes_forced_readback": True,
        "spv": {"kind": "tsr_device"},
        "stats": {
            "render_cv": 0.01,
            "render_min_ms": 29.0,
            "render_max_ms": 31.0,
            "present_cv": 0.02,
            "present_min_ms": 1.9,
            "present_max_ms": 2.1,
        },
        "notes": "synthetic green fixture",
    }


def run_selftest() -> int:
    frames, warmup = 3, 1
    # 绿臂:合法夹具必须零失败。
    green = validate_harness_evidence(good_fixture(frames, warmup), frames, warmup)
    if green:
        print(f"[{TAG}] selftest FAIL: 合法夹具误判红 {green}", file=sys.stderr)
        return 1
    # 红臂:六类构造缺陷逐条必须红（缺字段/digest 篡改/口径恒等式破坏/headless 冒充/
    # 计数不符/转引失败）。
    reds: list[tuple[str, dict]] = []
    bad = good_fixture(frames, warmup)
    del bad["real_render_frame_ms"]
    reds.append(("缺顶层字段", bad))
    bad = good_fixture(frames, warmup)
    bad["digest"] = "sha256:" + "f" * 63
    reds.append(("digest 形态篡改", bad))
    bad = good_fixture(frames, warmup)
    bad["present_overhead_ms"] = bad["present_frame_ms"] - 1.0
    reds.append(("口径恒等式破坏", bad))
    bad = good_fixture(frames, warmup)
    bad["headless"] = True  # window/present 口径仍在 → 自检面冒充真门
    reds.append(("headless 冒充", bad))
    bad = good_fixture(frames, warmup)
    bad["window"]["frames_presented"] = frames + warmup - 1
    reds.append(("present 计数不符", bad))
    bad = good_fixture(frames, warmup)
    bad["contracts"]["consistency"] = "fail"
    reds.append(("转引失败冒充 pass", bad))
    missed = []
    for name, fx in reds:
        got = validate_harness_evidence(fx, frames, warmup)
        if not got:
            missed.append(name)
    if missed:
        print(f"[{TAG}] selftest FAIL: 红臂漏检 {missed}", file=sys.stderr)
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
    print(f"[{TAG}] selftest PASS (1 GREEN + {len(reds)} RED + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=3)
    ap.add_argument("--warmup", type=int, default=1)
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

    # ② 构建必绿。
    r = run_cmd([
        "cargo", "build", "-p", "rurix-render", "--features", "vendor-upscale",
        "--bin", BIN, "--quiet",
    ])
    check(r.returncode == 0, f"harness 构建失败: {(r.stdout + r.stderr)[-600:]}")
    exe = ROOT / "target" / "debug" / f"{BIN}{EXE_SUFFIX}"
    check(exe.is_file(), f"产物缺失: {exe}")

    # ③ device 前置面（SPV/场景资产缺失 = DEV_ENV_DEGRADE 登记,不冒充 FAIL 也不 PASS）。
    missing_spv = [f for f in SPV_FILES if not (SPV_DIR / f).is_file()]
    degrade_reasons: list[str] = []
    if missing_spv:
        degrade_reasons.append(f"SPV 缺失 {missing_spv}（.tmp 构建产物,CI 需先备 kernel 编译面）")
    if not BISTRO_GLTF.is_file():
        degrade_reasons.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    # ④ device 真跑（持锁;validation 开）。
    ran = False
    if not FAILURES and not degrade_reasons:
        WORK_DIR.mkdir(parents=True, exist_ok=True)
        env = dict(os.environ)
        env["RURIX_VK_VALIDATION"] = "1"
        argv = [
            str(exe),
            "--frames", str(args.frames),
            "--warmup", str(args.warmup),
            "--hidden",
            "--evidence", str(HARNESS_EVIDENCE),
        ]
        with gpu_device_lock(purpose="g31 waveA window present device 腿"):
            r = run_cmd(argv, timeout=1200, env=env)
        out = r.stdout + r.stderr
        if '"state":"skipped_dev_env"' in out:
            degrade_reasons.append(f"harness skipped_dev_env: {out.strip()[-300:]}")
        else:
            check(r.returncode == 0, f"harness 非零退出 {r.returncode}: {out.strip()[-800:]}")
            check(f"[{TAG}]: PASS" in out, f"harness 缺 PASS 行: {out.strip()[-400:]}")
            check(
                "Validation Error" not in out and "VUID-" not in out,
                f"validation 应静默却报错: {out.strip()[-400:]}",
            )
            if r.returncode == 0 and HARNESS_EVIDENCE.is_file():
                try:
                    ev = json.loads(HARNESS_EVIDENCE.read_text(encoding="utf-8"))
                except json.JSONDecodeError as e:
                    ev = None
                    check(False, f"evidence 不可解析: {e}")
                if ev is not None:
                    vfail = validate_harness_evidence(ev, args.frames, args.warmup)
                    for m in vfail:
                        check(False, f"evidence 判据: {m}")
                    if not vfail:
                        ran = True
                        note(
                            f"真跑口径: real_render={ev['real_render_frame_ms']:.3f}ms "
                            f"present={ev['present_frame_ms']:.3f}ms "
                            f"overhead={ev['present_overhead_ms']:.3f}ms "
                            f"encode={ev['encode_frame_ms']:.3f}ms digest={ev['digest'][:23]}…"
                        )
            elif r.returncode == 0:
                check(False, "harness 退 0 但 evidence 未落盘")

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
        f"[{TAG}] PASS gate={GATE_KEY}（构建绿 + schema 互核 + bistro 1080p 真窗口 "
        f"{args.frames}+{args.warmup} 帧 present 逐帧成功 + evidence 口径三分离核验）"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
