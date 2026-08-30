#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 A Task A5）
"""G31+ 波 A Task A5 FG/MFG 帧生成生产接线门冒烟（g31.waveA.framegen）。

harness = `src/rurix-render/src/bin/g31_window_present.rs --auto-move <orbit|dolly>
--fg <x2|x3>`（G26 device kernel `kernels/g26_framegen.rx` 链接入呈现车道——生产五
pass 0-byte + `kernels/g31_mv_negate.rx` MV 取反 glue + fg kernel + `g31_display_encode`
复用，八/十 pass；present 序 = 生成帧 t 升序 → 真帧；MV = g14_mv 相机 MV 经取反
glue 直通馈入，与 host 金标准逐字同语义；--fg 闭集 = --auto-move + tier=100 +
非 headless）。本冒烟：

1. **构建必绿**：`cargo build --release -p rurix-render --features vendor-upscale
   --bin g31_window_present`（release = bench 同 profile，性能口径可比，如实登记）。
2. **schema 互核**：milestones/g31/g31_framegen_present_evidence_schema.json 在树且
   required 闭集与本脚本校验键集精确互核（防两侧静默漂移）。
3. **device 真跑**（持 gpu_device_lock 串行，RURIX_VK_VALIDATION=1，release 产物）：
   - run A1/A2：`orbit --fg x2` 双跑 → **digest_seq 逐帧位级一致**（确定性门）；
   - run B：`orbit --fg off`（gameloop 面）→ **digest_seq == A1**（FG 不回污染渲染
     车道机核门——真实渲染帧 digest 序列 on/off 位级一致）；
   - run C：`orbit --fg x3` → digest_seq == A1（x2/x3 真渲帧一致门）+ x3 计数面；
   - run D：`dolly --fg x2` → digest_seq ≠ A1（异轨迹门，防"确定性的坏内容"）；
   - evidence 逐项判（字段闭集/类型/双口径恒等式独立重算/计数面/wired_parity
     对拍面/digest 形态与序列）。
4. **G26 对拍门接线态复跑**（同一锁内）：`g26_framegen_device --spv <fg spv>
   --tol <g26_budget 标定 threshold>` 全档验证——state=pass + 三档 in_tol +
   SSIM 全帧胜 frame-hold + device 双跑位级（kernel 本体与 host 金标准面 0-byte
   的接线态维持证据；wired_parity 为生产帧对接面，本复跑为合成 GT 解析对拍面）。
5. **三态纪律**：无 GPU/Vulkan/场景资产/SPV/窗口创建失败 → harness 自报
   `skipped_dev_env`（退 0）→ 本冒烟输出 `DEV_ENV_DEGRADE` 三态之 SKIP（**禁冒充
   PASS**）；`RURIX_REQUIRE_REAL=1` 下 SKIP 翻硬 FAIL。harness 非零退出 = FAIL。
6. **--selftest**（合成夹具红绿自证，不依赖树上文件）：合法 evidence 合成件必须
   绿；八类构造缺陷（缺字段/恒等式旗标翻假/presented 计数脱节/wired_parity 超容差/
   digest 与序列末项脱节/fg_factor 与 fg_mode 不符/frames_presented 公式不符/
   headless 冒充）必须逐条红——证 validate 面真判红非摆设。

用法：
  py -3 ci/g31_framegen_present_smoke.py --gate g31.waveA.framegen
  py -3 ci/g31_framegen_present_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_framegen_present_evidence_schema.json"
G26_BUDGET_PATH = ROOT / "milestones" / "g26" / "g26_budget.json"
G26_TOL_ENTRY_ID = "g26.framegen_device.host_device_maxdiff_tol"
WORK_DIR = ROOT / ".tmp" / "g31_gates" / "waveA_framegen"
BIN = "g31_window_present"
G26_BIN = "g26_framegen_device"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveA.framegen"
TAG = "g31_framegen_present"
SCHEMA_ID = "rurix.g31.framegen_present_evidence.v1"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
KERNEL_SRC = ROOT / "src" / "rurix-render" / "kernels"
SPV_FILES = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
    "g26_framegen.spv",
    "g31_mv_negate.spv",
)
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
# RFC-0043 §1.4 F4 量化兜底：标定容差绝对上界 = RED_BIAS 0.05 × 0.5（冻结面非手写阈）。
TOL_RED_BIAS_BOUND = 0.05 * 0.5
# A5 接线 kernel 两件（g26 本体 0-byte + MV 取反 glue;缺则 rurixc 现编,.tmp 产物）。
A5_KERNELS = ("g26_framegen", "g31_mv_negate")

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
    "fg_mode",
    "fg_factor",
    "inserted_per_pair",
    "real_frames",
    "generated_frames",
    "presented_frames",
    "real_render_frame_ms",
    "real_render_seconds",
    "real_render_fps",
    "present_frame_ms",
    "present_seconds",
    "presented_fps",
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
    "wired_parity",
    "caliber_identities",
    "stats",
    "notes",
]

FG_FACTOR = {"x2": 2, "x3": 3}


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


def ensure_fg_spv() -> bool:
    """FG 接线 kernel SPV 两件存在性保障（缺则经 rurixc --target vulkan 现编；.tmp
    构建产物不入 git,源 = kernels/g26_framegen.rx 0-byte + kernels/g31_mv_negate.rx
    取反 glue）。其余五件 SPV 缺失 = degrade（由各自门保障面,本门不重编）。"""
    ok = True
    for name in A5_KERNELS:
        spv = SPV_DIR / f"{name}.spv"
        if spv.is_file():
            continue
        src = KERNEL_SRC / f"{name}.rx"
        if not src.is_file():
            return False
        rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
        if not rurixc.is_file():
            r = run_cmd(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"], timeout=7200)
            if r.returncode != 0 or not rurixc.is_file():
                return False
        spv.parent.mkdir(parents=True, exist_ok=True)
        r = run_cmd([str(rurixc), str(src), "--target", "vulkan", "-o", str(spv)], timeout=1800)
        ok = ok and r.returncode == 0 and spv.is_file()
    return ok


def g26_frozen_tol() -> float | None:
    """G26 冻结容差程序读（budget 标定条目 threshold;缺失 = None 由调用侧判 FAIL,
    fail-closed 禁手写阈）。"""
    if not G26_BUDGET_PATH.is_file():
        return None
    budget = json.loads(G26_BUDGET_PATH.read_text(encoding="utf-8"))
    for e in budget.get("entries", []):
        if e.get("id") == G26_TOL_ENTRY_ID:
            return float(e["threshold"])
    return None


def compare_digest_seqs_same(a: list, b: list) -> list[str]:
    """确定性/不污染门：digest_seq 必须逐帧位级一致（返回失败串,空 = 绿）。"""
    fails: list[str] = []
    if len(a) != len(b):
        return [f"digest_seq 长度 {len(a)} ≠ {len(b)}"]
    diff = [k for k, (x, y) in enumerate(zip(a, b)) if x != y]
    if diff:
        fails.append(f"digest_seq 位级不一致:首异帧 {diff[0]}（共 {len(diff)} 帧异）")
    return fails


def compare_digest_seqs_diff(a: list, b: list) -> list[str]:
    """异轨迹门：digest_seq 必须至少一帧不同（返回失败串,空 = 绿）。"""
    if len(a) != len(b):
        return []
    if all(x == y for x, y in zip(a, b)):
        return ["digest_seq 全等——异轨迹面疑似冒充（相机未真实生效?）"]
    return []


def validate_harness_evidence(ev: dict, expect_frames: int, expect_warmup: int, trajectory: str, fg_mode: str) -> list[str]:
    """harness evidence 逐项判（返回失败串列表,空 = 绿;--selftest 合成夹具同消费）。
    双口径恒等式独立重算（不信旗标面）：presented == real+generated、real_fps ==
    real/real_seconds、presented_fps 重算、generated 公式面、frames_presented 公式面。"""
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
    if ev.get("tier") != 100:
        fails.append(f"tier ≠ 100（--fg 闭集）: {ev.get('tier')!r}")
    res = ev.get("resolution") or {}
    if (res.get("w"), res.get("h")) != (1920, 1080):
        fails.append(f"resolution ≠ 1920x1080: {res!r}")
    ires = ev.get("internal_resolution") or {}
    if (ires.get("w"), ires.get("h")) != (1920, 1080):
        fails.append(f"internal_resolution ≠ 1920x1080（tier=100 同栅格面）: {ires!r}")
    # ── FG 档闭集与计数面 ──
    factor = FG_FACTOR.get(fg_mode)
    inserted = factor - 1
    if ev.get("fg_mode") != fg_mode:
        fails.append(f"fg_mode ≠ {fg_mode}: {ev.get('fg_mode')!r}")
    if ev.get("fg_factor") != factor:
        fails.append(f"fg_factor {ev.get('fg_factor')} ≠ {factor}（fg_mode={fg_mode}）")
    if ev.get("inserted_per_pair") != inserted:
        fails.append(f"inserted_per_pair {ev.get('inserted_per_pair')} ≠ {inserted}")
    real = ev.get("real_frames")
    gen = ev.get("generated_frames")
    presented = ev.get("presented_frames")
    if real != expect_frames:
        fails.append(f"real_frames {real} ≠ frames {expect_frames}（post-warmup 真渲计数）")
    want_gen = expect_frames * inserted - (inserted if expect_warmup == 0 else 0)
    if gen != want_gen:
        fails.append(f"generated_frames {gen} ≠ {want_gen}（real×inserted 公式面）")
    if not isinstance(presented, int) or presented != (real or -1) + (gen or -1):
        fails.append(f"presented_frames {presented} ≠ real+generated {real}+{gen}（计数脱节）")
    # ── 双口径恒等式独立重算（不信旗标）──
    rr_ms = ev.get("real_render_frame_ms")
    rr_s = ev.get("real_render_seconds")
    rr_fps = ev.get("real_render_fps")
    p_s = ev.get("present_seconds")
    p_fps = ev.get("presented_fps")
    p_ms = ev.get("present_frame_ms")
    for name, v in (("real_render_frame_ms", rr_ms), ("real_render_seconds", rr_s), ("real_render_fps", rr_fps), ("present_frame_ms", p_ms), ("presented_fps", p_fps)):
        if not isinstance(v, (int, float)) or isinstance(v, bool) or not v > 0:
            fails.append(f"{name} 非正数: {v!r}")
    if not isinstance(p_s, (int, float)) or isinstance(p_s, bool) or not p_s >= 0:
        fails.append(f"present_seconds 非 ≥0 数: {p_s!r}")
    if isinstance(real, int) and isinstance(rr_s, (int, float)) and rr_s > 0:
        # 重算容差 1e-6 相对:evidence fps 以 6 位小数落盘(舍入 ≤5e-7),容差盖舍入
        # 而仍足以识别口径混算(生成帧混入 real 口径 = 2~3× 量级偏差)。
        if not isinstance(rr_fps, (int, float)) or abs(rr_fps - real / rr_s) > 1e-6 * max(1.0, real / rr_s):
            fails.append(f"real_render_fps {rr_fps} ≠ real/seconds 重算 {real / rr_s}（独立重算判红）")
    if isinstance(rr_ms, (int, float)) and isinstance(rr_s, (int, float)) and isinstance(real, int) and real > 0:
        if abs(rr_s - rr_ms * real / 1000.0) > 1e-6 * max(1.0, rr_s):
            fails.append(f"real_render_seconds {rr_s} ≠ frame_ms×real/1000 {rr_ms * real / 1000.0}（统计口径脱节）")
    if isinstance(presented, int) and isinstance(rr_s, (int, float)) and isinstance(p_s, (int, float)) and rr_s + p_s > 0:
        want_pfps = presented / (rr_s + p_s)
        if not isinstance(p_fps, (int, float)) or abs(p_fps - want_pfps) > 1e-6 * max(1.0, want_pfps):
            fails.append(f"presented_fps {p_fps} ≠ presented/(render+present) 重算 {want_pfps}（独立重算判红）")
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
    poses = ev.get("camera_poses")
    if not isinstance(poses, list) or len(poses) != total:
        fails.append(f"camera_poses 非数组或长度 ≠ {total}")
    if ev.get("headless") is not False:
        fails.append(f"headless ≠ false（--fg 闭集互斥 headless）: {ev.get('headless')!r}")
    win = ev.get("window")
    if not isinstance(win, dict):
        fails.append("window 非 object（--fg 面必须真窗口）")
        win = {}
    if win.get("channel_order") not in ("bgra8_unorm", "rgba8_unorm"):
        fails.append(f"channel_order 越闭集: {win.get('channel_order')!r}")
    ext = win.get("extent") or {}
    if (ext.get("w"), ext.get("h")) != (1920, 1080):
        fails.append(f"window.extent ≠ 1920x1080: {ext!r}")
    fp = win.get("frames_presented")
    want_fp = 1 + (total - 1) * factor
    if ev.get("resize_eras") == 0 and fp != want_fp:
        fails.append(f"frames_presented {fp} ≠ 1+(total−1)×factor {want_fp}（真帧/生成帧序列计数面）")
    sr = win.get("swapchain_rebuilds")
    if not isinstance(sr, int) or sr < 0:
        fails.append(f"swapchain_rebuilds 非 ≥0 int: {sr!r}")
    contracts = ev.get("contracts") or {}
    if contracts.get("consistency") != "pass":
        fails.append(f"contracts.consistency ≠ pass: {contracts.get('consistency')!r}")
    prod = contracts.get("production") or {}
    if not isinstance(prod.get("digest"), str) or not DIGEST_RE.match(prod["digest"]):
        fails.append(f"production.digest 形态非法: {str(prod.get('digest'))[:40]!r}")
    for gk in ("g10_contract", "g10_camera", "g10_lighting", "encode_spv", "framegen_spv"):
        g = contracts.get(gk) or {}
        if not isinstance(g.get("sha256"), str) or not DIGEST_RE.match(g["sha256"]):
            fails.append(f"{gk}.sha256 形态非法: {str(g.get('sha256'))[:40]!r}")
    if ev.get("render_includes_forced_readback") is not True:
        fails.append("render_includes_forced_readback ≠ true（口径登记缺失）")
    # ── 接线态对拍面（逐像素 ULP 结构界硬门 + SSIM 严格胜 frame-hold;
    #    1080p HDR 生产帧绝对容差物理不适用,登记 p100 实测事实）──
    wp = ev.get("wired_parity")
    if not isinstance(wp, dict):
        fails.append("wired_parity 非 object（接线态对拍登记缺失）")
        wp = {}
    else:
        if not isinstance(wp.get("probe_frame"), int) or wp.get("probe_frame", 0) < 2:
            fails.append(f"wired_parity.probe_frame 非 ≥2 int: {wp.get('probe_frame')!r}")
        ff = wp.get("frozen_floor")
        if not isinstance(ff, (int, float)) or isinstance(ff, bool) or not ff > 0:
            fails.append(f"wired_parity.frozen_floor 非正数: {ff!r}")
        elif ff >= TOL_RED_BIAS_BOUND:
            fails.append(f"wired_parity.frozen_floor {ff} 超 F4 量化兜底上界 {TOL_RED_BIAS_BOUND}")
        p100 = wp.get("p100")
        if not isinstance(p100, (int, float)) or isinstance(p100, bool) or not p100 >= 0:
            fails.append(f"wired_parity.p100 非 ≥0 数: {p100!r}")
        for ck in ("val_ulp_err", "max_bound"):
            if not isinstance(wp.get(ck), (int, float)) or isinstance(wp.get(ck), bool) or not wp[ck] > 0:
                fails.append(f"wired_parity.{ck} 非正数: {wp.get(ck)!r}")
        if wp.get("excess") != 0:
            fails.append(f"wired_parity.excess {wp.get('excess')!r} ≠ 0（逐像素 L1 结构界超界判红）")
        er = wp.get("excess_ratio")
        if not isinstance(er, (int, float)) or isinstance(er, bool) or not 0 <= er <= 1.0:
            fails.append(f"wired_parity.excess_ratio 越 [0,1]: {er!r}")
        if wp.get("in_bound") is not True:
            fails.append("wired_parity.in_bound ≠ true")
        if wp.get("mvn_max_abs_plus_mv") != 0:
            fails.append(f"wired_parity.mvn_max_abs_plus_mv {wp.get('mvn_max_abs_plus_mv')!r} ≠ 0（MV 通路位级硬门判红）")
        if wp.get("ssim_beats_frame_hold") is not True:
            fails.append("wired_parity.ssim_beats_frame_hold ≠ true")
        sd = wp.get("ssim_device_vs_hostref")
        sh = wp.get("ssim_frame_hold_vs_hostref")
        if not isinstance(sd, (int, float)) or not isinstance(sh, (int, float)):
            fails.append("wired_parity.ssim 双面非数值")
        elif not sd > sh:
            fails.append(f"SSIM(device,hostref) {sd} 未严格胜 frame-hold {sh}（独立重算判红）")
        pg = wp.get("per_gen_p100")
        if not isinstance(pg, list) or len(pg) != inserted:
            fails.append(f"wired_parity.per_gen_p100 长度 ≠ {inserted}")
        tv = wp.get("t_values")
        if not isinstance(tv, list) or len(tv) != inserted:
            fails.append(f"wired_parity.t_values 长度 ≠ {inserted}")
        if not isinstance(wp.get("floor_source"), str) or not wp["floor_source"]:
            fails.append("wired_parity.floor_source 空")
    # ── 恒等式旗标面（schema 钉 const true;本脚本不信旗标已独立重算,旗标只核存在性）──
    ci = ev.get("caliber_identities") or {}
    for ck in ("presented_eq_real_plus_generated", "real_fps_recompute_ok", "real_fps_isolated_from_generated_ok", "presented_fps_recompute_ok", "digest_seq_len_eq_real_frames_total"):
        if ci.get(ck) is not True:
            fails.append(f"caliber_identities.{ck} ≠ true")
    stats = ev.get("stats") or {}
    for sk in ("render_cv", "render_min_ms", "render_max_ms", "encode_gpu_ms", "fg_gpu_ms", "render5_gpu_ms"):
        if not isinstance(stats.get(sk), (int, float)) or isinstance(stats.get(sk), bool):
            fails.append(f"stats.{sk} 非数值: {stats.get(sk)!r}")
    if not isinstance(ev.get("notes"), str) or not ev["notes"]:
        fails.append("notes 空（口径注释面缺失）")
    return fails


def good_fixture(frames: int = 8, warmup: int = 2, trajectory: str = "orbit", fg_mode: str = "x2") -> dict:
    """合法 evidence 合成夹具（数字为占位形态值,自证 validate 绿臂——不进任何 evidence）。"""
    d = "sha256:" + "0" * 64
    total = frames + warmup
    factor = FG_FACTOR[fg_mode]
    inserted = factor - 1
    real = frames
    gen = frames * inserted - (inserted if warmup == 0 else 0)
    presented = real + gen
    rr_s = 0.05
    p_s = 0.012 * presented
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
        "fg_mode": fg_mode,
        "fg_factor": factor,
        "inserted_per_pair": inserted,
        "real_frames": real,
        "generated_frames": gen,
        "presented_frames": presented,
        "real_render_frame_ms": rr_s * 1000.0 / real,
        "real_render_seconds": rr_s,
        "real_render_fps": real / rr_s,
        "present_frame_ms": 1.2,
        "present_seconds": p_s,
        "presented_fps": presented / (rr_s + p_s),
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
            "frames_presented": 1 + (total - 1) * factor,
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
            "framegen_spv": {"path": "f.spv", "sha256": d},
        },
        "render_includes_forced_readback": True,
        "spv": {"kind": "tsr_device"},
        "wired_parity": {
            "probe_frame": 3,
            "p100": 9.7e-04,
            "per_gen_p100": [9.7e-04] * inserted,
            "frozen_floor": 7.152557373046876e-07,
            "floor_source": "milestones/g26/g26_budget.json#g26.framegen_device.host_device_maxdiff_tol",
            "g26_measured_anchor": 3.576278686523438e-07,
            "val_ulp_err": 2.0e-06,
            "max_bound": 6.0e-02,
            "excess": 0,
            "excess_ratio": 0.016,
            "in_bound": True,
            "mvn_max_abs_plus_mv": 0,
            "ssim_device_vs_hostref": 0.9999999,
            "ssim_frame_hold_vs_hostref": 0.999,
            "ssim_beats_frame_hold": True,
            "t_values": [i / (inserted + 1) for i in range(1, inserted + 1)],
            "note": "synthetic",
        },
        "caliber_identities": {
            "presented_eq_real_plus_generated": True,
            "real_fps_recompute_ok": True,
            "real_fps_isolated_from_generated_ok": True,
            "presented_fps_recompute_ok": True,
            "digest_seq_len_eq_real_frames_total": True,
        },
        "stats": {
            "render_cv": 0.01,
            "render_min_ms": 4.9,
            "render_max_ms": 5.1,
            "encode_gpu_ms": 0.11,
            "fg_gpu_ms": 0.5,
            "render5_gpu_ms": 3.5,
            "present_cv": 0.02,
            "present_min_ms": 1.1,
            "present_max_ms": 1.3,
        },
        "notes": "synthetic green fixture",
    }
    return ev


def run_selftest() -> int:
    frames, warmup = 8, 2
    green = validate_harness_evidence(good_fixture(frames, warmup), frames, warmup, "orbit", "x2")
    if green:
        print(f"[{TAG}] selftest FAIL: x2 合法夹具误判红 {green}", file=sys.stderr)
        return 1
    green3 = validate_harness_evidence(good_fixture(frames, warmup, fg_mode="x3"), frames, warmup, "orbit", "x3")
    if green3:
        print(f"[{TAG}] selftest FAIL: x3 合法夹具误判红 {green3}", file=sys.stderr)
        return 1
    reds: list[tuple[str, dict]] = []
    bad = good_fixture(frames, warmup)
    del bad["wired_parity"]
    reds.append(("缺顶层字段", bad))
    bad = good_fixture(frames, warmup)
    bad["caliber_identities"]["real_fps_isolated_from_generated_ok"] = False
    reds.append(("恒等式旗标翻假", bad))
    bad = good_fixture(frames, warmup)
    bad["presented_frames"] = bad["real_frames"] + bad["generated_frames"] + 1
    reds.append(("presented 计数脱节", bad))
    bad = good_fixture(frames, warmup)
    bad["wired_parity"]["excess"] = 1.0e-03
    reds.append(("wired_parity 超结构界", bad))
    bad = good_fixture(frames, warmup)
    bad["digest"] = "sha256:" + "1" * 64
    reds.append(("digest 与序列末项脱节", bad))
    bad = good_fixture(frames, warmup)
    bad["fg_factor"] = 3
    reds.append(("fg_factor 与 fg_mode 不符", bad))
    bad = good_fixture(frames, warmup)
    bad["window"]["frames_presented"] += 1
    reds.append(("frames_presented 公式不符", bad))
    bad = good_fixture(frames, warmup)
    bad["headless"] = True
    reds.append(("headless 冒充", bad))
    bad = good_fixture(frames, warmup)
    bad["real_render_fps"] = bad["real_render_fps"] * 1.5
    reds.append(("real_fps 独立重算脱节", bad))
    bad = good_fixture(frames, warmup)
    bad["generated_frames"] = bad["generated_frames"] + 1
    reds.append(("generated 公式脱节", bad))
    missed = []
    for name, fx in reds:
        got = validate_harness_evidence(fx, frames, warmup, "orbit", "x2")
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


def run_harness(exe: Path, trajectory: str, fg_mode: str, frames: int, warmup: int, ev_path: Path, env: dict) -> tuple[subprocess.CompletedProcess, str]:
    argv = [
        str(exe),
        "--frames", str(frames),
        "--warmup", str(warmup),
        "--hidden",
        "--quality", "off",  # W4 默认翻转免疫:A5 门 = fg base 点（两点式闭集之 all-off 基,DEFAULT_FLIP_PLAN §2.5）
        "--auto-move", trajectory,
        "--fg", fg_mode,
        "--evidence", str(ev_path),
    ]
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

    # ① schema 在树 + required 闭集互核 + G26 冻结容差程序读。
    check(SCHEMA_PATH.is_file(), f"schema 文件缺失: {SCHEMA_PATH}")
    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        check(
            set(schema.get("required", [])) == set(REQUIRED_KEYS),
            f"schema required 与校验键集不等: {set(schema.get('required', [])) ^ set(REQUIRED_KEYS)}",
        )
    tol = g26_frozen_tol()
    check(tol is not None, f"G26 冻结容差缺失: {G26_BUDGET_PATH} 无 {G26_TOL_ENTRY_ID}")
    if tol is not None:
        check(tol < TOL_RED_BIAS_BOUND, f"G26 冻结容差 {tol} 超 F4 量化兜底上界 {TOL_RED_BIAS_BOUND}")

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
    if not ensure_fg_spv():
        degrade_reasons.append("g26_framegen/g31_mv_negate SPV 缺失且 rurixc 现编失败（.tmp 构建产物）")
    missing_spv = [f for f in SPV_FILES if not (SPV_DIR / f).is_file()]
    if missing_spv:
        degrade_reasons.append(f"SPV 缺失 {missing_spv}（.tmp 构建产物,CI 需先备 kernel 编译面）")
    if not BISTRO_GLTF.is_file():
        degrade_reasons.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    # ④ device 真跑（持锁;validation 开）：x2 双跑确定性 + fg off 不污染 +
    #    x3 一致 + dolly 异轨迹 + G26 对拍门接线态复跑。
    ran = False
    if not FAILURES and not degrade_reasons:
        WORK_DIR.mkdir(parents=True, exist_ok=True)
        env = dict(os.environ)
        env["RURIX_VK_VALIDATION"] = "1"
        ev_a1 = WORK_DIR / "orbit_x2_run1.json"
        ev_a2 = WORK_DIR / "orbit_x2_run2.json"
        ev_b = WORK_DIR / "orbit_off_run.json"
        ev_c = WORK_DIR / "orbit_x3_run.json"
        ev_d = WORK_DIR / "dolly_x2_run.json"
        with gpu_device_lock(purpose="g31 waveA framegen device 腿 + g26 对拍门复跑"):
            ra1, outa1 = run_harness(exe, "orbit", "x2", args.frames, args.warmup, ev_a1, env)
            ra2 = rb = rc = rd = None
            outa2 = outb = outc = outd = ""
            if '"state":"skipped_dev_env"' not in outa1 and ra1.returncode == 0:
                ra2, outa2 = run_harness(exe, "orbit", "x2", args.frames, args.warmup, ev_a2, env)
                rb, outb = run_harness(exe, "orbit", "off", args.frames, args.warmup, ev_b, env)
                rc, outc = run_harness(exe, "orbit", "x3", args.frames, args.warmup, ev_c, env)
                rd, outd = run_harness(exe, "dolly", "x2", args.frames, args.warmup, ev_d, env)
            # ── G26 对拍门接线态复跑（g26 harness 全档验证;kernel/host 金标准
            #    0-byte 维持证据面;harness 构建失败 = FAIL 非 degrade——本门硬判据）──
            g26_ok = False
            g26_detail = "未执行"
            rb26 = run_cmd([
                "cargo", "build", "-p", "rurix-render", "--features", "vulkan",
                "--bin", G26_BIN, "--quiet",
            ], timeout=7200)
            g26_exe = ROOT / "target" / "debug" / f"{G26_BIN}{EXE_SUFFIX}"
            if rb26.returncode != 0 or not g26_exe.is_file():
                g26_detail = f"g26 harness 构建失败: {(rb26.stdout + rb26.stderr)[-300:]}"
            else:
                rg = run_cmd([
                    str(g26_exe), "--spv", str(SPV_DIR / "g26_framegen.spv"), "--tol", repr(tol),
                ], env=dict(env, RURIX_REQUIRE_REAL="1"), timeout=3600)
                line = ""
                for ln in rg.stdout.splitlines():
                    if "rurix.g26framegen.harness.v1" in ln:
                        line = ln.strip()
                doc = {}
                if line:
                    try:
                        doc = json.loads(line)
                    except json.JSONDecodeError:
                        doc = {}
                tiers = doc.get("tiers") or {}
                rows = [tiers.get(t) or {} for t in ("x2", "x3", "x4")]
                g26_ok = (
                    rg.returncode == 0
                    and doc.get("state") == "pass"
                    and all(t.get("in_tol") is True for t in rows)
                    and all(t.get("ssim_all_beat_frame_hold") is True for t in rows)
                    and all(t.get("bitexact") is True for t in rows)
                    and (doc.get("host") or {}).get("ssim_beats_frame_hold") is True
                )
                g26_detail = (
                    f"state={doc.get('state')};"
                    + ";".join(
                        f"{n} p100={(tiers.get(n) or {}).get('p100_vs_host')} in_tol={(tiers.get(n) or {}).get('in_tol')} ssim={(tiers.get(n) or {}).get('ssim_all_beat_frame_hold')} bitexact={(tiers.get(n) or {}).get('bitexact')}"
                        for n in ("x2", "x3", "x4")
                    )
                    + f";tol={tol:.6e}"
                )
        check(g26_ok, f"G26 对拍门接线态复跑未过: {g26_detail}")
        if g26_ok:
            note(f"G26 对拍门接线态复跑 pass: {g26_detail}")

        if '"state":"skipped_dev_env"' in outa1:
            degrade_reasons.append(f"harness skipped_dev_env: {outa1.strip()[-300:]}")
        else:
            for tag, rr, out in (("orbit_x2#1", ra1, outa1), ("orbit_x2#2", ra2, outa2), ("orbit_off", rb, outb), ("orbit_x3", rc, outc), ("dolly_x2", rd, outd)):
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
            for tag, p in (("orbit_x2#1", ev_a1), ("orbit_x2#2", ev_a2), ("orbit_off", ev_b), ("orbit_x3", ev_c), ("dolly_x2", ev_d)):
                if not p.is_file():
                    check(False, f"{tag} evidence 未落盘: {p}")
                    evs.append(None)
                    continue
                try:
                    evs.append(json.loads(p.read_text(encoding="utf-8")))
                except json.JSONDecodeError as e:
                    check(False, f"{tag} evidence 不可解析: {e}")
                    evs.append(None)
            ea1, ea2, eb, ec, ed = evs
            if ea1 is not None:
                vfail = validate_harness_evidence(ea1, args.frames, args.warmup, "orbit", "x2")
                for m in vfail:
                    check(False, f"orbit_x2#1 evidence 判据: {m}")
                if not vfail:
                    ran = True
            if ea2 is not None:
                for m in validate_harness_evidence(ea2, args.frames, args.warmup, "orbit", "x2"):
                    check(False, f"orbit_x2#2 evidence 判据: {m}")
            if ec is not None:
                for m in validate_harness_evidence(ec, args.frames, args.warmup, "orbit", "x3"):
                    check(False, f"orbit_x3 evidence 判据: {m}")
            if ed is not None:
                for m in validate_harness_evidence(ed, args.frames, args.warmup, "dolly", "x2"):
                    check(False, f"dolly_x2 evidence 判据: {m}")
            if eb is not None:
                check(
                    isinstance(eb.get("digest_seq"), list) and len(eb["digest_seq"]) == args.frames + args.warmup,
                    "orbit_off（gameloop 面）digest_seq 缺失或长度不符",
                )
            # 四面门:双跑位级一致 / fg off 不污染 / x3 一致 / 异轨迹不同。
            if ea1 is not None and ea2 is not None:
                for m in compare_digest_seqs_same(ea1["digest_seq"], ea2["digest_seq"]):
                    check(False, f"确定性门(x2 双跑): {m}")
            if ea1 is not None and eb is not None:
                for m in compare_digest_seqs_same(ea1["digest_seq"], eb["digest_seq"]):
                    check(False, f"FG 不污染门(on/off 真渲帧 digest 位级一致): {m}")
            if ea1 is not None and ec is not None:
                for m in compare_digest_seqs_same(ea1["digest_seq"], ec["digest_seq"]):
                    check(False, f"x2/x3 真渲帧一致门: {m}")
            if ea1 is not None and ed is not None:
                for m in compare_digest_seqs_diff(ea1["digest_seq"], ed["digest_seq"]):
                    check(False, f"异轨迹门: {m}")
            if ran and ea1 is not None and ec is not None:
                note(
                    f"x2 真跑口径: real_render={ea1['real_render_frame_ms']:.3f}ms "
                    f"real_fps={ea1['real_render_fps']:.2f} presented_fps={ea1['presented_fps']:.2f} "
                    f"fg_gpu={ea1['stats']['fg_gpu_ms']:.3f}ms wired_p100={ea1['wired_parity']['p100']:.3e}"
                )
                note(
                    f"x3 真跑口径: real_render={ec['real_render_frame_ms']:.3f}ms "
                    f"real_fps={ec['real_render_fps']:.2f} presented_fps={ec['presented_fps']:.2f} "
                    f"fg_gpu={ec['stats']['fg_gpu_ms']:.3f}ms wired_p100={ec['wired_parity']['p100']:.3e}"
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
        f"[{TAG}] PASS gate={GATE_KEY}（release 构建绿 + schema 互核 + x2 双跑 digest_seq "
        f"位级一致 + fg off 不污染 + x3 一致 + dolly 异轨迹 + G26 对拍门接线态复跑 pass "
        f"+ evidence 闭集/双口径恒等式/接线态对拍判据全绿）"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
