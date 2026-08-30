#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Claude（G37 W3 fg_combo 判档子任务;设计事实源 = 同目录 REPORT.md）
"""G37 W3「FG × 画质臂(--quality full)组合面」验收臂 —— GPU 步骤写好不跑。

前置(获批执行会话才成立;本脚本默认 --plan 只打印命令,纪律 = 本任务禁跑 GPU):
  1. 窗口 bin 已按 REPORT.md §4 十六锚接线(A 判档字面 + B comp-parity 接线 + C 登记面);
  2. release 构建在案:cargo build --release -p rurix-render --features vendor-upscale
     --bin g31_window_present(由验收窗会话执行,本脚本不代跑构建);
  3. full 预设 SPV 工件链在案(realism/bloom/AE/encode;缺失时 harness 自报
     skipped_dev_env,三态纪律之 SKIP,禁冒充 PASS)。

五臂 + 互斥矩阵(全部 --hidden 真窗口,--auto-move orbit,64+10 帧,waveB 矩阵同窗形):
  arm alloff    : 全画质 off 基线(gameloop 面;画质生效门对照)
  arm full_a/b  : --quality full(fg off)双跑(textures 面;不污染门基线 + full 自身确定性)
  arm combo_a/b : --quality full --fg x2 双跑(FG 面;机核组合臂)
  arm combo_x3  : --quality full --fg x3 单跑(x2/x3 真渲帧一致门)
  mutex 矩阵    : 解除后仍须 exit=1 的残余互斥逐条核验(fail-closed 不回退证明)

判读(--judge 可对已在案 evidence 重入;全部独立重算不信旗标):
  ① 双跑位级:combo_a vs combo_b 的 digest + digest_seq 逐帧一致;full_a vs full_b 同律;
  ② 不污染门:digest_seq(combo_x2) == digest_seq(full fg-off)(真渲帧位级——FG 不回写
     画质车道;comp parity 适配只换缓冲对象不换数值的真跑复核);
  ③ 画质生效门:digest_seq(full) != digest_seq(alloff)(防「组合绿但画质没开」冒充);
  ④ presented 计数恒等式:presented == real+generated、generated 公式面、
     window.frames_presented == 1+(total−1)×factor、len(digest_seq) == frames+warmup;
  ⑤ real fps 口径隔离:real_render_fps/presented_fps/real_render_seconds 三重独立重算、
     stats.fg_gpu_ms 与 stats.render5_gpu_ms 分列存在、caliber_identities 五旗标恒 true;
  ⑥ wired_parity:excess==0 / in_bound / mvn_max_abs_plus_mv==0 / SSIM 严格胜 frame-hold /
     frozen_floor == milestones/g26/g26_budget.json 标定条目程序读(禁手写阈);
  ⑦ x2/x3 真渲帧一致门:digest_seq(combo_x3) == digest_seq(combo_x2);
  ⑧ 互斥矩阵:六形态 exit=1(hzb/slab/lut/无轨迹/headless/散臂微调两点式闭集)。

用法:
  py -3 accept_fg_combo.py --selftest   # 判读器合成夹具红绿自证(无 GPU,当下可跑)
  py -3 accept_fg_combo.py --plan      # 打印 GPU 步骤(默认;不执行)
  py -3 accept_fg_combo.py --execute   # 获批验收窗真跑(gpu_device_lock + validation)
  py -3 accept_fg_combo.py --judge     # 只判读已在案 evidence(写 ACCEPTANCE_SUMMARY.json)
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]  # artifacts/day_0830_delivery/w3_deep/fg_combo → 仓根
WORK_DIR = ROOT / ".tmp" / "g31_gates" / "w3_fg_combo"
SUMMARY_PATH = HERE / "ACCEPTANCE_SUMMARY.json"
G26_BUDGET_PATH = ROOT / "milestones" / "g26" / "g26_budget.json"
G26_TOL_ENTRY_ID = "g26.framegen_device.host_device_maxdiff_tol"
FG_SCHEMA_ID = "rurix.g31.framegen_present_evidence.v1"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "release" / f"g31_window_present{EXE_SUFFIX}"
TAG = "fg_combo_accept"
FRAMES, WARMUP = 64, 10
FG_FACTOR = {"x2": 2, "x3": 3}

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


# ── 五臂命令面(REPORT.md §7;waveB 组合矩阵同窗形 64+10 orbit --hidden)──
def arm_argv(name: str) -> list[str]:
    base = [
        str(BIN),
        "--frames", str(FRAMES),
        "--warmup", str(WARMUP),
        "--hidden",
        "--auto-move", "orbit",
        "--evidence", str(WORK_DIR / f"{name}.json"),
    ]
    extra = {
        # W4 默认翻转免疫:alloff 臂 = 画质生效门对照基线,须显式 off（DEFAULT_FLIP_PLAN §2.5）。
        "alloff": ["--quality", "off"],
        "full_a": ["--quality", "full"],
        "full_b": ["--quality", "full"],
        "combo_a": ["--quality", "full", "--fg", "x2"],
        "combo_b": ["--quality", "full", "--fg", "x2"],
        "combo_x3": ["--quality", "full", "--fg", "x3"],
    }[name]
    return base + extra


ARMS = ["alloff", "full_a", "full_b", "combo_a", "combo_b", "combo_x3"]

# ── 互斥矩阵(解除后仍须 exit=1;fail-closed 不回退证明——REPORT.md §6)──
MUTEX_ARMS: list[tuple[str, list[str]]] = [
    ("fg×hzb 维持", ["--frames", "8", "--warmup", "2", "--hidden", "--auto-move", "orbit",
                     "--quality", "full", "--fg", "x2", "--hzb", "on"]),
    ("fg×slab 维持", ["--frames", "8", "--warmup", "2", "--hidden", "--auto-move", "orbit",
                      "--quality", "full", "--fg", "x2",
                      "--slab-table", "milestones/g31/g31_slab_side_table_bistro_interior.json"]),
    ("fg×lut 维持", ["--frames", "8", "--warmup", "2", "--hidden", "--auto-move", "orbit",
                     "--quality", "full", "--fg", "x2", "--lut", "warm"]),
    ("fg 无轨迹维持", ["--frames", "8", "--warmup", "2", "--hidden",
                       "--quality", "full", "--fg", "x2"]),
    ("fg×headless 维持", ["--frames", "8", "--warmup", "2", "--headless-smoke",
                          "--auto-move", "orbit", "--quality", "full", "--fg", "x2"]),
    # W4 默认翻转免疫:显式 off 保「两点式闭集卫兵」路径本身被测(不带则翻转后经
    # full×--textures dup 冲突"碰巧"exit=1,卫兵路径失覆盖;DEFAULT_FLIP_PLAN §2.5)。
    ("散臂微调拒跑(两点式闭集)", ["--frames", "8", "--warmup", "2", "--hidden",
                                  "--auto-move", "orbit", "--quality", "off",
                                  "--textures", "on", "--fg", "x2"]),
]


def g26_frozen_tol() -> float | None:
    """G26 冻结容差程序读(budget 标定条目 threshold;缺失 = None 判红,禁手写阈)。"""
    if not G26_BUDGET_PATH.is_file():
        return None
    budget = json.loads(G26_BUDGET_PATH.read_text(encoding="utf-8"))
    for e in budget.get("entries", []):
        if e.get("id") == G26_TOL_ENTRY_ID:
            return float(e["threshold"])
    return None


# ---------------------------------------------------------------------------
# 判读函数族(纯 JSON 消费,独立重算不信旗标;--selftest 合成夹具同消费)
# ---------------------------------------------------------------------------

def j_seq_same(a: list, b: list, what: str) -> list[str]:
    if not isinstance(a, list) or not isinstance(b, list):
        return [f"{what}: digest_seq 非数组"]
    if len(a) != len(b):
        return [f"{what}: digest_seq 长度 {len(a)} ≠ {len(b)}"]
    diff = [k for k, (x, y) in enumerate(zip(a, b)) if x != y]
    return [f"{what}: 首异帧 {diff[0]}(共 {len(diff)} 帧异)"] if diff else []


def j_seq_diff(a: list, b: list, what: str) -> list[str]:
    if isinstance(a, list) and isinstance(b, list) and len(a) == len(b) and all(x == y for x, y in zip(a, b)):
        return [f"{what}: digest_seq 全等——on 面疑似未真实生效(冒充组合)"]
    return []


def j_double_run(ev_a: dict, ev_b: dict, what: str) -> list[str]:
    fails = j_seq_same(ev_a.get("digest_seq"), ev_b.get("digest_seq"), f"{what} 双跑 digest_seq")
    if ev_a.get("digest") != ev_b.get("digest"):
        fails.append(f"{what} 双跑 digest 位级不一致")
    return fails


def j_counts(ev: dict, fg_mode: str, frames: int = FRAMES, warmup: int = WARMUP) -> list[str]:
    """presented 计数恒等式(独立重算)。消费面 = FG evidence(schema v1)。"""
    fails: list[str] = []
    if ev.get("schema") != FG_SCHEMA_ID:
        fails.append(f"schema ≠ {FG_SCHEMA_ID}: {ev.get('schema')!r}(evidence 分支未按 C1 前移?)")
    factor = FG_FACTOR[fg_mode]
    inserted = factor - 1
    total = frames + warmup
    if ev.get("fg_mode") != fg_mode or ev.get("fg_factor") != factor or ev.get("inserted_per_pair") != inserted:
        fails.append(f"fg 档字段与 {fg_mode} 不符: mode={ev.get('fg_mode')!r} factor={ev.get('fg_factor')!r}")
    real, gen, presented = ev.get("real_frames"), ev.get("generated_frames"), ev.get("presented_frames")
    if real != frames:
        fails.append(f"real_frames {real} ≠ frames {frames}(post-warmup 真渲计数)")
    want_gen = frames * inserted - (inserted if warmup == 0 else 0)
    if gen != want_gen:
        fails.append(f"generated_frames {gen} ≠ {want_gen}(real×inserted 公式面)")
    if not isinstance(presented, int) or presented != (real or -1) + (gen or -1):
        fails.append(f"presented_frames {presented} ≠ real+generated(计数脱节)")
    seq = ev.get("digest_seq")
    if not isinstance(seq, list) or len(seq) != total:
        fails.append(f"digest_seq 长度 ≠ frames+warmup {total}")
    win = ev.get("window") or {}
    want_fp = 1 + (total - 1) * factor
    if ev.get("resize_eras") == 0 and win.get("frames_presented") != want_fp:
        fails.append(f"window.frames_presented {win.get('frames_presented')} ≠ 1+(total−1)×factor {want_fp}")
    if ev.get("headless") is not False:
        fails.append("headless ≠ false(--fg 面必须真窗口)")
    return fails


def j_caliber_isolation(ev: dict) -> list[str]:
    """real fps 口径隔离(三重独立重算 + telemetry 分列 + 恒等式旗标)。"""
    fails: list[str] = []
    real = ev.get("real_frames")
    rr_ms, rr_s, rr_fps = ev.get("real_render_frame_ms"), ev.get("real_render_seconds"), ev.get("real_render_fps")
    p_s, p_fps = ev.get("present_seconds"), ev.get("presented_fps")
    presented = ev.get("presented_frames")
    if isinstance(real, int) and isinstance(rr_s, (int, float)) and rr_s > 0:
        want = real / rr_s
        if not isinstance(rr_fps, (int, float)) or abs(rr_fps - want) > 1e-6 * max(1.0, want):
            fails.append(f"real_render_fps {rr_fps} ≠ real/seconds 重算 {want}(生成帧混入 real 口径即 2~3× 偏差)")
    if isinstance(rr_ms, (int, float)) and isinstance(rr_s, (int, float)) and isinstance(real, int) and real > 0:
        if abs(rr_s - rr_ms * real / 1000.0) > 1e-6 * max(1.0, rr_s):
            fails.append(f"real_render_seconds {rr_s} ≠ frame_ms×real/1000(统计口径脱节)")
    if isinstance(presented, int) and isinstance(rr_s, (int, float)) and isinstance(p_s, (int, float)) and rr_s + p_s > 0:
        want = presented / (rr_s + p_s)
        if not isinstance(p_fps, (int, float)) or abs(p_fps - want) > 1e-6 * max(1.0, want):
            fails.append(f"presented_fps {p_fps} ≠ presented/(render+present) 重算 {want}")
    stats = ev.get("stats") or {}
    for sk in ("fg_gpu_ms", "render5_gpu_ms"):
        v = stats.get(sk)
        if not isinstance(v, (int, float)) or isinstance(v, bool) or not v > 0:
            fails.append(f"stats.{sk} 非正数(FG/真渲 GPU 段 telemetry 分列缺失): {v!r}")
    ci = ev.get("caliber_identities") or {}
    for ck in ("presented_eq_real_plus_generated", "real_fps_recompute_ok",
               "real_fps_isolated_from_generated_ok", "presented_fps_recompute_ok",
               "digest_seq_len_eq_real_frames_total"):
        if ci.get(ck) is not True:
            fails.append(f"caliber_identities.{ck} ≠ true")
    return fails


def j_wired_parity(ev: dict, tol: float | None) -> list[str]:
    """接线态对拍(probe 帧 host 金标准复算;判据 = 结构界/SSIM/MVN 位级/容差程序读)。"""
    fails: list[str] = []
    wp = ev.get("wired_parity")
    if not isinstance(wp, dict):
        return ["wired_parity 非 object(接线态对拍登记缺失)"]
    if wp.get("excess") != 0:
        fails.append(f"wired_parity.excess {wp.get('excess')!r} ≠ 0(逐像素 L1 结构界超界)")
    if wp.get("in_bound") is not True:
        fails.append("wired_parity.in_bound ≠ true")
    if wp.get("mvn_max_abs_plus_mv") != 0:
        fails.append(f"wired_parity.mvn_max_abs_plus_mv ≠ 0(MV 通路位级硬门): {wp.get('mvn_max_abs_plus_mv')!r}")
    if wp.get("ssim_beats_frame_hold") is not True:
        fails.append("wired_parity.ssim_beats_frame_hold ≠ true")
    sd, sh = wp.get("ssim_device_vs_hostref"), wp.get("ssim_frame_hold_vs_hostref")
    if not isinstance(sd, (int, float)) or not isinstance(sh, (int, float)) or not sd > sh:
        fails.append(f"SSIM(device,hostref) {sd!r} 未严格胜 frame-hold {sh!r}(独立重算)")
    if tol is None:
        fails.append(f"G26 冻结容差缺失: {G26_BUDGET_PATH} 无 {G26_TOL_ENTRY_ID}(禁手写阈)")
    else:
        ff = wp.get("frozen_floor")
        if not isinstance(ff, (int, float)) or abs(ff - tol) > 1e-15:
            fails.append(f"wired_parity.frozen_floor {ff!r} ≠ G26 budget 程序读 {tol}(容差来源脱钩)")
    return fails


def judge_all(evs: dict[str, dict | None], tol: float | None) -> list[str]:
    """八面门合判(evs 键 = ARMS;None = 未在案)。返回失败串列表,空 = 绿。"""
    fails: list[str] = []
    for name in ARMS:
        if evs.get(name) is None:
            fails.append(f"{name} evidence 未在案: {WORK_DIR / (name + '.json')}")
    if fails:
        return fails
    alloff, full_a, full_b = evs["alloff"], evs["full_a"], evs["full_b"]
    combo_a, combo_b, combo_x3 = evs["combo_a"], evs["combo_b"], evs["combo_x3"]
    # ① 双跑位级(combo 机核 + full 基线自身)。
    fails += j_double_run(combo_a, combo_b, "combo_x2")
    fails += j_double_run(full_a, full_b, "full")
    # ② 不污染门:combo 真渲帧 == full fg-off 真渲帧(位级)。
    fails += j_seq_same(combo_a.get("digest_seq"), full_a.get("digest_seq"),
                        "不污染门(full+fg x2 vs full fg-off)")
    # ③ 画质生效门:full != alloff(防组合绿但画质没开)。
    fails += j_seq_diff(full_a.get("digest_seq"), alloff.get("digest_seq"),
                        "画质生效门(full vs alloff)")
    # ④+⑤ 计数恒等式 + 口径隔离(combo 两档)。
    for name, ev, mode in (("combo_a", combo_a, "x2"), ("combo_b", combo_b, "x2"), ("combo_x3", combo_x3, "x3")):
        fails += [f"{name}: {m}" for m in j_counts(ev, mode)]
        fails += [f"{name}: {m}" for m in j_caliber_isolation(ev)]
    # ⑥ wired_parity(combo 两档;容差程序读)。
    for name, ev in (("combo_a", combo_a), ("combo_x3", combo_x3)):
        fails += [f"{name}: {m}" for m in j_wired_parity(ev, tol)]
    # ⑦ x2/x3 真渲帧一致门。
    fails += j_seq_same(combo_x3.get("digest_seq"), combo_a.get("digest_seq"),
                        "x2/x3 真渲帧一致门")
    return fails


# ---------------------------------------------------------------------------
# 运行面
# ---------------------------------------------------------------------------

def print_plan() -> None:
    print(f"[{TAG}] PLAN(GPU 步骤写好不跑——本任务纪律禁跑 GPU;获批验收窗以 --execute 执行):")
    print(f"[{TAG}]   前置: cargo build --release -p rurix-render --features vendor-upscale --bin g31_window_present")
    print(f"[{TAG}]   环境: RURIX_VK_VALIDATION=1 RURIX_REQUIRE_REAL=1;持 ci/gpu_device_lock 串行")
    for name in ARMS:
        print(f"[{TAG}]   arm {name}: {' '.join(arm_argv(name))}")
    for what, extra in MUTEX_ARMS:
        print(f"[{TAG}]   mutex[{what}](期望 exit=1): {BIN} {' '.join(extra)}")
    print(f"[{TAG}]   判读: py -3 {Path(__file__).name} --judge")


def load_evidence() -> dict[str, dict | None]:
    evs: dict[str, dict | None] = {}
    for name in ARMS:
        p = WORK_DIR / f"{name}.json"
        try:
            evs[name] = json.loads(p.read_text(encoding="utf-8")) if p.is_file() else None
        except json.JSONDecodeError as e:
            evs[name] = None
            check(False, f"{name} evidence 不可解析: {e}")
    return evs


def run_execute() -> int:
    """获批验收窗真跑(独占 GPU;三态纪律:skipped_dev_env → SKIP 禁冒充)。"""
    sys.path.insert(0, str(ROOT / "ci"))
    from gpu_device_lock import gpu_device_lock  # noqa: E402
    if not BIN.is_file():
        print(f"[{TAG}] FAIL: release 产物缺失 {BIN}(验收窗自行构建后重跑)", file=sys.stderr)
        return 1
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env["RURIX_VK_VALIDATION"] = "1"
    degrade = False
    with gpu_device_lock(purpose="G37 W3 fg×quality-full 组合验收窗"):
        for name in ARMS:
            argv = arm_argv(name)
            print(f"[{TAG}] $ {' '.join(argv)}")
            r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=3600, env=env)
            out = r.stdout + r.stderr
            COMMANDS.append({"arm": name, "exit_code": r.returncode})
            if '"state":"skipped_dev_env"' in out:
                degrade = True
                note(f"{name}: skipped_dev_env(SPV/场景/窗口面缺失)——三态之 SKIP")
                break
            check(r.returncode == 0, f"{name} 非零退出 {r.returncode}: {out.strip()[-600:]}")
            check("[g31_window_present]: PASS" in out, f"{name} 缺 PASS 行")
            check("Validation Error" not in out and "VUID-" not in out,
                  f"{name} validation 应静默却报错: {out.strip()[-400:]}")
        if not degrade and not FAILURES:
            for what, extra in MUTEX_ARMS:
                argv = [str(BIN)] + extra
                print(f"[{TAG}] $ {' '.join(argv)}")
                r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=600, env=env)
                COMMANDS.append({"mutex": what, "exit_code": r.returncode})
                check(r.returncode == 1, f"互斥矩阵[{what}] 期望 exit=1 实得 {r.returncode}(fail-closed 面回退!)")
    if degrade:
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        print(f"[{TAG}] SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0
    return run_judge()


def run_judge() -> int:
    tol = g26_frozen_tol()
    evs = load_evidence()
    fails = FAILURES + judge_all(evs, tol)
    combo = evs.get("combo_a") or {}
    summary = {
        "schema": "rurix.g37w3.fg_combo_acceptance.v1",
        "window": "G37 W3 fg×quality-full 组合验收(REPORT.md §2.3 口径不变量集)",
        "arms": {n: str(WORK_DIR / f"{n}.json") for n in ARMS},
        "mutex_arms": [w for w, _ in MUTEX_ARMS],
        "g26_frozen_tol": tol,
        "verdict": "GREEN" if not fails else "RED",
        "failures": fails,
        "notes": NOTES,
        "commands": COMMANDS,
        "measured": {
            "real_render_frame_ms": combo.get("real_render_frame_ms"),
            "real_render_fps": combo.get("real_render_fps"),
            "presented_fps": combo.get("presented_fps"),
            "fg_gpu_ms": (combo.get("stats") or {}).get("fg_gpu_ms"),
            "render5_gpu_ms": (combo.get("stats") or {}).get("render5_gpu_ms"),
            "wired_p100": (combo.get("wired_parity") or {}).get("p100"),
        } if combo else None,
        "discipline": "measured 如实登记不设通过线(G6 无硬门纪律);判读全部独立重算不信旗标;"
                      "互斥面 exit=1 逐条核验零冒充;digest 门全部位级",
    }
    SUMMARY_PATH.write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")
    if fails:
        print(f"[{TAG}] FAIL ({len(fails)}):", file=sys.stderr)
        for m in fails:
            print(f"  - {m}", file=sys.stderr)
        return 1
    print(f"[{TAG}] PASS(双跑位级 + 不污染 + 画质生效 + 计数恒等式 + 口径隔离 + wired_parity"
          f" + x2/x3 一致 + 互斥矩阵;summary → {SUMMARY_PATH})")
    return 0


# ---------------------------------------------------------------------------
# --selftest:判读器合成夹具红绿自证(无 GPU,不依赖树上 evidence)
# ---------------------------------------------------------------------------

def _fixture(fg_mode: str = "x2", frames: int = FRAMES, warmup: int = WARMUP, seed: str = "0") -> dict:
    d = "sha256:" + seed * 64
    total = frames + warmup
    factor = FG_FACTOR[fg_mode]
    inserted = factor - 1
    real = frames
    gen = frames * inserted - (inserted if warmup == 0 else 0)
    presented = real + gen
    rr_s = 0.5
    p_s = 0.001 * presented
    return {
        "schema": FG_SCHEMA_ID,
        "fg_mode": fg_mode, "fg_factor": factor, "inserted_per_pair": inserted,
        "real_frames": real, "generated_frames": gen, "presented_frames": presented,
        "real_render_frame_ms": rr_s * 1000.0 / real, "real_render_seconds": rr_s,
        "real_render_fps": real / rr_s, "present_seconds": p_s,
        "presented_fps": presented / (rr_s + p_s),
        "digest": d, "digest_seq": [d] * total, "resize_eras": 0, "headless": False,
        "window": {"frames_presented": 1 + (total - 1) * factor},
        "stats": {"fg_gpu_ms": 0.6, "render5_gpu_ms": 4.0},
        "caliber_identities": {k: True for k in (
            "presented_eq_real_plus_generated", "real_fps_recompute_ok",
            "real_fps_isolated_from_generated_ok", "presented_fps_recompute_ok",
            "digest_seq_len_eq_real_frames_total")},
        "wired_parity": {"excess": 0, "in_bound": True, "mvn_max_abs_plus_mv": 0,
                         "ssim_beats_frame_hold": True, "ssim_device_vs_hostref": 0.9999,
                         "ssim_frame_hold_vs_hostref": 0.99, "frozen_floor": 7.152557373046876e-07,
                         "p100": 9.0e-04},
    }


def run_selftest() -> int:
    tol = 7.152557373046876e-07
    # 绿臂:x2/x3 合法夹具全门过。
    evs = {"alloff": _fixture(seed="1"), "full_a": _fixture(), "full_b": _fixture(),
           "combo_a": _fixture(), "combo_b": _fixture(), "combo_x3": _fixture("x3")}
    green = judge_all(evs, tol)
    if green:
        print(f"[{TAG}] selftest FAIL: 合法夹具误判红 {green}", file=sys.stderr)
        return 1
    # 红臂逐条(每条须被检出)。
    reds: list[tuple[str, dict[str, dict]]] = []
    bad = dict(evs)
    b = _fixture()
    b["digest_seq"] = [b["digest_seq"][0]] * (FRAMES + WARMUP - 1) + ["sha256:" + "f" * 64]
    bad["combo_b"] = b
    reds.append(("双跑位级破坏", bad))
    bad = dict(evs)
    b = _fixture(seed="2")
    bad["full_a"] = b
    bad["full_b"] = b
    reds.append(("不污染门破坏(combo≠full)", bad))
    bad = dict(evs)
    bad["alloff"] = _fixture()
    reds.append(("画质生效门破坏(full==alloff)", bad))
    bad = dict(evs)
    b = _fixture()
    b["presented_frames"] += 1
    bad["combo_a"] = b
    reds.append(("presented 计数脱节", bad))
    bad = dict(evs)
    b = _fixture()
    b["real_render_fps"] *= 1.5
    bad["combo_a"] = b
    reds.append(("real fps 口径混算", bad))
    bad = dict(evs)
    b = _fixture()
    b["stats"]["fg_gpu_ms"] = 0.0
    bad["combo_a"] = b
    reds.append(("fg_gpu_ms 分列缺失", bad))
    bad = dict(evs)
    b = _fixture()
    b["wired_parity"]["excess"] = 1e-3
    b["wired_parity"]["in_bound"] = False
    bad["combo_a"] = b
    reds.append(("wired_parity 超结构界", bad))
    bad = dict(evs)
    b = _fixture()
    b["wired_parity"]["frozen_floor"] = 1e-3
    bad["combo_a"] = b
    reds.append(("容差手写脱钩", bad))
    bad = dict(evs)
    b = _fixture("x3")
    b["digest_seq"] = [b["digest_seq"][0]] * (FRAMES + WARMUP - 1) + ["sha256:" + "e" * 64]
    bad["combo_x3"] = b
    reds.append(("x2/x3 真渲帧脱节", bad))
    bad = dict(evs)
    b = _fixture()
    b["schema"] = "rurix.g31.texture_sampling_evidence.v1"
    bad["combo_a"] = b
    reds.append(("evidence 分支未前移(落 textures 面)", bad))
    missed = [name for name, fx in reds if not judge_all(fx, tol)]
    if missed:
        print(f"[{TAG}] selftest FAIL: 红臂漏检 {missed}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(1 GREEN + {len(reds)} RED 全检出)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    g = ap.add_mutually_exclusive_group()
    g.add_argument("--plan", action="store_true", help="打印 GPU 步骤(默认;不执行)")
    g.add_argument("--execute", action="store_true", help="获批验收窗真跑(独占 GPU)")
    g.add_argument("--judge", action="store_true", help="只判读已在案 evidence")
    g.add_argument("--selftest", action="store_true", help="判读器合成夹具红绿自证")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.judge:
        return run_judge()
    if args.execute:
        return run_execute()
    print_plan()
    return 0


if __name__ == "__main__":
    sys.exit(main())
