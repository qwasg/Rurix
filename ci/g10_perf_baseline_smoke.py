#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.5b 波）
"""G10.5b M141 性能对标基线门冒烟（步骤 190；g10.p0.m141.perf_baseline；
G10_CONTRACT §4.2 M141 行 / G-G10-7；G10_ACCEPTANCE_MAP §1 M141 行；14 §5
证据分级与基准纪律；milestones/m0/BENCH_PROTOCOL.md §2/§3；G10.1 baseline
clock_lock_note 诚实存档先例 evidence/g10_baseline_sr_pipeline_l3_20260815T094538Z.json）。

host+device 门（device_section_state=executed——UE 真跑经
g10_5_ue_bench.py 子进程自持 gpu_device_lock 串行，本门不嵌套持锁 D5 定案）。
判据：双端同场景帧率采样（14 §5 协议：L0 环境验证 → warmup/稳态 → 50×3
trimmed mean → IQR）+ 环境画像随证据存档 + 双端交替采样顺序登记；未锁频
登记缺失/环境画像缺字段/采样轮数不足冒充即 RED。**只建基线数据，不设帧率
通过线**（契约 G-G10-7 / 立项裁决 5）。

采样面：
- Rurix 端 = `g10_5_scene_render --benchmark`（release profile；host CPU
  GI 管线同 A/B 渲染路径；进程内 warmup 10 + timed 150，逐帧 Instant 墙钟；
  首帧内容 digest == A/B 库帧 digest 机核锚 + distinct_frame_digests==1
  确定性断言）。
- UE 端 = MRQ benchmark（g10_5_build_bench.py 建 Bench 资产 + 本门
  g10_5_ue_bench.py Phase B 真跑；engine_warm_up_count=64 引擎预热 +
  输出 160 帧 = 前 10 帧 warmup 弃计 + 150 timed = 3 trial 块 × 50；逐帧
  时长取 EXR 头 unreal/frameRenderDuration——5.8 源树 MoviePipeline.cpp
  RenderTimeFrameStatistics → EXR FileMetadata 实证面，不解析日志）。
- 双端交替采样顺序：场景粒度 [rurix@cornell → ue5@cornell → rurix@bistro →
  ue5@bistro]，逐腿 UTC 起止随证据登记。

统计口径（冻结登记）：逐 trial 块 IQR 去离群（Q1/Q3 = numpy linear 百分位，
拒 [Q1−1.5·IQR, Q3+1.5·IQR] 外）→ 块内中位数 → 3 块中位数的均值
（trimmed mean(0.2) 于 3 块 = 均值）；cv = 全留样本 stdev/mean；ci95 =
bootstrap 2000 次（np.random.default_rng(42) 确定性）重采样三块中位数均值
的 2.5/97.5 百分位。timer：rurix=host Instant（release profile）/
ue5=MRQ frameRenderDuration（EXR 头）。

用法：
  py -3 ci/g10_perf_baseline_smoke.py --gate g10.p0.m141.perf_baseline
  py -3 ci/g10_perf_baseline_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import platform
import re
import statistics
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m141_perf_baseline_evidence_schema.json"
CORPUS = ROOT / "milestones" / "g10" / "corpus"
FRAMES = Path(r"K:\rurix-ext\g10-frames\g10_5")
BENCH_FRAMES = FRAMES / "bench"
RUST_RELEASE_BIN = ROOT / "target" / "release" / "g10_5_scene_render.exe"
UE_BENCH_BUILD = ROOT / "milestones" / "g10" / "harness" / "ue_python" / "g10_5_build_bench.py"
UE_BENCH_RUN = ROOT / "milestones" / "g10" / "harness" / "g10_5_ue_bench.py"
UE_RUN = ROOT / "milestones" / "g10" / "harness" / "g10_5_ue_run.py"

sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "ci"))
from bench import env_probe  # noqa: E402

GATE_KEY = "g10.p0.m141.perf_baseline"
NUMERIC_STEP = 190
SOURCE_REF = (
    "G10_CONTRACT §4.2 M141 + G-G10-7;G10_ACCEPTANCE_MAP §1 M141;"
    "14 §5;milestones/m0/BENCH_PROTOCOL.md §2/§3;G10.1 baseline clock_lock_note 先例"
)
TAG = "g10_m141"
SUBJECT = "g10_m141_perf_baseline"
MATRIX_ROW = "M141"

SCENES = ("cornell-box", "bistro-interior")
GLTF = {
    "cornell-box": Path(r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf"),
    "bistro-interior": Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf"),
}
LIB_HDR_DIGEST = {
    "cornell-box": "sha256:c2000ebfbe90359d55e668f8af3b7df24d64c3f72e637904f614821b7ad0d727",
    "bistro-interior": "sha256:8519cc67c917e7b8c2c5a9bb5633ea5ee9e72deb8cf63b3b187b0d3ac5bb9935",
}
WARMUP = 10
TIMED = 150
BLOCKS = 3
BLOCK_SIZE = 50
UE_TOTAL_FRAMES = 160  # 10 warmup 弃计 + 150 timed

REQUIRED_ENV_FIELDS = [
    "gpu_name", "driver_version", "nvml_version", "cuda_driver_version",
    "driver_model", "hags_enabled", "tdr", "clocks", "thermal",
    "isolation_check", "os_build",
]

CHECK_KEYS = [
    "l0_environment_profile_complete",
    "clock_lock_state_registered",
    "rurix_benchmark_real_run_release",
    "ue_benchmark_real_run_mrq",
    "sampling_protocol_rounds_complete",
    "alternating_sampling_order_registered",
    "stats_recompute_match",
    "baseline_values_registered",
    "red_unlocked_registration_missing_detected",
    "red_profile_missing_field_detected",
    "red_round_shortfall_masquerade_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def run_cmd(argv: list[str], timeout: int = 7200, env_extra: dict | None = None) -> subprocess.CompletedProcess:
    import os

    print(f"[{TAG}] $ {' '.join(str(a) for a in argv)}", flush=True)
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(str(a) for a in argv), "exit_code": r.returncode})
    return r


# ---------------------------------------------------------------------------
# 统计口径（冻结：块 IQR 去离群 → 块中位数 → 3 块均值；bootstrap ci95 定种）
# ---------------------------------------------------------------------------

def block_stats(samples: list[float]) -> dict:
    """150 样本 → 3 块 × 50 → 逐块 IQR 去离群 + 中位数 → 汇总面。"""
    if len(samples) != TIMED:
        raise ValueError(f"timed 样本数 ≠ {TIMED}: {len(samples)}")
    blocks = []
    kept_all: list[float] = []
    for b in range(BLOCKS):
        seg = samples[b * BLOCK_SIZE:(b + 1) * BLOCK_SIZE]
        q1, q3 = np.percentile(np.asarray(seg, dtype=np.float64), [25.0, 75.0])
        iqr = q3 - q1
        lo, hi = q1 - 1.5 * iqr, q3 + 1.5 * iqr
        kept = [v for v in seg if lo <= v <= hi]
        rejected = len(seg) - len(kept)
        blocks.append({
            "block": b,
            "median_ms": statistics.median(kept),
            "kept": len(kept),
            "rejected_iqr": rejected,
        })
        kept_all.extend(kept)
    medians = [b["median_ms"] for b in blocks]
    trimmed_mean = sum(medians) / len(medians)
    arr = np.asarray(kept_all, dtype=np.float64)
    cv = float(arr.std(ddof=1) / arr.mean()) if arr.size > 1 else 0.0
    rng = np.random.default_rng(42)
    kept_arr = np.asarray(kept_all, dtype=np.float64)
    reps = []
    for _ in range(2000):
        draw = rng.choice(kept_arr, size=TIMED, replace=True)
        bm = [float(np.median(draw[i * BLOCK_SIZE:(i + 1) * BLOCK_SIZE])) for i in range(BLOCKS)]
        reps.append(sum(bm) / len(bm))
    lo, hi = np.percentile(np.asarray(reps), [2.5, 97.5])
    return {
        "blocks": blocks,
        "trimmed_mean_ms": trimmed_mean,
        "fps": 1000.0 / trimmed_mean,
        "cv": cv,
        "ci95_ms": [float(lo), float(hi)],
        "min_ms": float(arr.min()),
        "max_ms": float(arr.max()),
        "outliers_rejected_iqr": sum(b["rejected_iqr"] for b in blocks),
        "kept_total": len(kept_all),
    }


def recompute_check(samples: list[float], reported: dict) -> bool:
    """独立重算核验（第二实现路径：手工排序中位数 + sum 均值 + 同口径 IQR 围栏）。"""
    if len(samples) != TIMED:
        return False
    medians = []
    for b in range(BLOCKS):
        seg = sorted(samples[b * BLOCK_SIZE:(b + 1) * BLOCK_SIZE])
        n = len(seg)
        # numpy linear 百分位同口径手工面：idx = p/100*(n-1)，线性插值。
        def pct(p: float) -> float:
            idx = p / 100.0 * (n - 1)
            lo = int(idx)
            hi = min(lo + 1, n - 1)
            frac = idx - lo
            return seg[lo] + (seg[hi] - seg[lo]) * frac

        q1, q3 = pct(25.0), pct(75.0)
        iqr = q3 - q1
        kept = [v for v in seg if (q1 - 1.5 * iqr) <= v <= (q3 + 1.5 * iqr)]
        m = len(kept)
        med = kept[m // 2] if m % 2 == 1 else (kept[m // 2 - 1] + kept[m // 2]) / 2.0
        medians.append(med)
    tm = sum(medians) / 3.0
    return (
        abs(tm - reported["trimmed_mean_ms"]) <= 1e-9
        and all(abs(medians[i] - reported["blocks"][i]["median_ms"]) <= 1e-9 for i in range(BLOCKS))
        and abs(1000.0 / tm - reported["fps"]) <= 1e-6
    )


def parse_frame_render_duration_s(text: str) -> float:
    """UE EXR 头 unreal/frameRenderDuration（FTimespan ToString 形态
    `[+|-][d.]HH:MM:SS.fff`）→ 秒。"""
    m = re.match(r"^([+-]?)(?:(\d+)\.)?(\d+):(\d+):(\d+(?:\.\d+)?)$", text.strip())
    if not m:
        raise ValueError(f"frameRenderDuration 形态非法: {text!r}")
    sign = -1.0 if m.group(1) == "-" else 1.0
    days = int(m.group(2) or 0)
    hours = int(m.group(3))
    minutes = int(m.group(4))
    secs = float(m.group(5))
    return sign * (days * 86400.0 + hours * 3600.0 + minutes * 60.0 + secs)


def ue_frame_durations_ms(scene: str) -> list[float]:
    """从 MRQ bench 出帧目录逐帧 EXR 头解析 frameRenderDuration（帧号升序）。"""
    import g10_exr_lib as exr  # 延迟导入（ci 路径已插入）

    out_dir = BENCH_FRAMES / "ue" / scene
    frames = sorted(out_dir.glob(".*.exr"), key=lambda p: int(p.stem.lstrip(".")))
    if len(frames) != UE_TOTAL_FRAMES:
        raise RuntimeError(f"{scene} UE bench 帧数 ≠ {UE_TOTAL_FRAMES}: {len(frames)}")
    durations = []
    for p in frames:
        attrs, _ = exr.parse_header(p.read_bytes())
        raw = next((a[2].decode("utf-8") for a in attrs if a[0] == "unreal/frameRenderDuration"), None)
        if raw is None:
            raise RuntimeError(f"{p.name} 缺 unreal/frameRenderDuration")
        durations.append(parse_frame_render_duration_s(raw) * 1000.0)
    return durations


def env_profile_problems(env: dict) -> list[str]:
    """环境画像缺字段机核（14 §5：画像随证据存档，字段闭集缺一即问题行）。"""
    problems = [f"缺字段 {k}" for k in REQUIRED_ENV_FIELDS if k not in env]
    clocks = env.get("clocks", {})
    for k in ("locked", "sm_clock_mhz", "mem_clock_mhz", "lock_method"):
        if k not in clocks:
            problems.append(f"clocks 缺字段 {k}")
    tdr = env.get("tdr", {})
    for k in ("tdr_delay", "tdr_level"):
        if k not in tdr:
            problems.append(f"tdr 缺字段 {k}")
    return problems


def sampling_rounds_problems(leg: dict) -> list[str]:
    """采样轮数机核（warmup ≥10 ∧ timed == 150 ∧ 3×50 块；不足冒充即问题行）。"""
    problems = []
    if leg.get("warmup_count", 0) < WARMUP:
        problems.append(f"warmup {leg.get('warmup_count')} < {WARMUP}")
    if leg.get("timed_count") != TIMED:
        problems.append(f"timed {leg.get('timed_count')} ≠ {TIMED}")
    samples = leg.get("samples_ms", [])
    if len(samples) != TIMED:
        problems.append(f"原始样本数 {len(samples)} ≠ {TIMED}")
    return problems


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂①：统计面——确定性合成样本自洽（recompute 第二实现核验通过）。
    rng = np.random.default_rng(7)
    samples = [float(v) for v in rng.normal(16.6, 0.3, TIMED)]
    rep = block_stats(samples)
    if not recompute_check(samples, rep):
        print(f"[{TAG}] selftest FAIL: 统计面重算自洽失效", file=sys.stderr)
        return 1
    # 绿臂②：FTimespan 解析（实证形态 + 带天/负号形态）。
    if abs(parse_frame_render_duration_s("+00:00:00.012") - 0.012) > 1e-12:
        print(f"[{TAG}] selftest FAIL: timespan 解析失效", file=sys.stderr)
        return 1
    if abs(parse_frame_render_duration_s("1.01:02:03.5") - 90123.5) > 1e-9:
        print(f"[{TAG}] selftest FAIL: timespan 带天形态解析失效", file=sys.stderr)
        return 1
    # 红臂①：轮数不足冒充必检出。
    short = {"warmup_count": 10, "timed_count": 150, "samples_ms": samples[:100]}
    if not sampling_rounds_problems(short):
        print(f"[{TAG}] selftest FAIL: 轮数不足冒充未检出", file=sys.stderr)
        return 1
    # 红臂②：画像缺字段必检出。
    env = {k: "x" for k in REQUIRED_ENV_FIELDS}
    env["clocks"] = {"locked": False, "sm_clock_mhz": 240, "mem_clock_mhz": 405, "lock_method": "m"}
    env["tdr"] = {"tdr_delay": "not_set", "tdr_level": "not_set"}
    if env_profile_problems(env):
        print(f"[{TAG}] selftest FAIL: 全字段画像被误拒", file=sys.stderr)
        return 1
    env2 = dict(env)
    del env2["driver_version"]
    if not env_profile_problems(env2):
        print(f"[{TAG}] selftest FAIL: 画像缺字段未检出", file=sys.stderr)
        return 1
    # 绿臂③：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (2 RED + 4 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    base_commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                 capture_output=True, text=True).stdout.strip()

    # ---- ① L0 环境验证 + 环境画像（14 §5；env_probe 单一事实源） ----
    env: dict = {}
    try:
        env = env_probe.collect_environment()
    except Exception as e:  # noqa: BLE001 — NVML 不可达即 dev_env_degrade 面
        check(False, f"env_probe 环境画像采集失败: {e}")
    profile_problems = env_profile_problems(env) if env else ["画像空"]
    checks["l0_environment_profile_complete"] = not profile_problems
    check(not profile_problems, f"环境画像缺字段: {profile_problems}")

    # ---- ② 锁频状态实测登记（未锁频 → clock_lock_note 诚实存档，G10.1 先例） ----
    clocks = env.get("clocks", {})
    locked = bool(clocks.get("locked"))
    clock_lock_note = ""
    if not locked:
        clock_lock_note = (
            f"GPU 未锁频（NVML 采样探测 locked=false，sm {clocks.get('sm_clock_mhz')}MHz / "
            f"mem {clocks.get('mem_clock_mhz')}MHz；锁频需提权 nvidia-smi -lgc/-lmc，本门未执行）"
            "——沿 G10.1 baseline 先例以 measured_local 登记并将未锁频边界诚实存档；"
            "本门只建基线数据，不设帧率通过线，未锁频漂移风险如实登记不掩盖。"
        )
    checks["clock_lock_state_registered"] = ("locked" in clocks) and (locked or bool(clock_lock_note))
    check(checks["clock_lock_state_registered"], "锁频状态登记缺失（未锁频登记缺失即 RED）")

    # ---- ③ release 构建 ----
    r = run_cmd(["cargo", "build", "--release", "-p", "rurix-asset", "--bin", "g10_5_scene_render"], timeout=3600)
    build_ok = r.returncode == 0 and RUST_RELEASE_BIN.is_file()

    # ---- ④ 双端交替采样（场景粒度 [R,U]×场景；逐腿 UTC 起止登记） ----
    legs: dict[tuple[str, str], dict] = {}
    sampling_order: list[dict] = []
    seq = 0
    for scene in SCENES:
        # Rurix 腿（release --benchmark；进程内 warmup 10 + timed 150）。
        seq += 1
        leg_key = (scene, "rurix")
        t0 = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        leg: dict = {"end": "rurix", "scene_id": scene, "timer": "host Instant (release profile)"}
        if build_ok:
            rb = run_cmd([
                str(RUST_RELEASE_BIN), "--benchmark", "--gltf", str(GLTF[scene]),
                "--contract", str(CORPUS / f"contract_params_{scene.replace('-', '_')}.json"),
                "--scene-id", scene, "--warmup", str(WARMUP), "--frames", str(TIMED),
            ], timeout=7200)
            doc = {}
            if rb.returncode == 0:
                try:
                    doc = json.loads((rb.stdout or "").strip().splitlines()[-1])
                except (json.JSONDecodeError, IndexError) as e:
                    check(False, f"rurix benchmark 输出解析失败（{scene}）: {e}")
            if doc:
                digest_ok = doc.get("first_frame_digest") == LIB_HDR_DIGEST[scene]
                if not digest_ok:
                    check(False, f"rurix benchmark 首帧 digest ≠ A/B 库帧（{scene}）: {doc.get('first_frame_digest')}")
                if doc.get("distinct_frame_digests") != 1:
                    check(False, f"rurix benchmark 逐帧 digest 非单值（{scene}，确定性破缺）: {doc.get('distinct_frame_digests')}")
                leg.update({
                    "profile": doc.get("profile"),
                    "warmup_count": doc.get("warmup_count"),
                    "timed_count": doc.get("timed_count"),
                    "samples_ms": doc.get("frame_ms", []),
                    "warmup_ms": doc.get("warmup_ms", []),
                    "first_frame_digest": doc.get("first_frame_digest"),
                    "distinct_frame_digests": doc.get("distinct_frame_digests"),
                    "digest_binds_ab_library": digest_ok,
                })
        t1 = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        leg["utc_start"], leg["utc_end"] = t0, t1
        legs[leg_key] = leg
        sampling_order.append({"seq": seq, "end": "rurix", "scene_id": scene, "utc_start": t0, "utc_end": t1})
        note(f"采样腿 {seq} rurix@{scene} 完成（{t0}~{t1}）")

        # UE 腿（Bench 资产建设 + MRQ Phase B 真跑 + EXR 头逐帧时长解析）。
        seq += 1
        leg_key = (scene, "ue5")
        t0 = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        leg = {"end": "ue5", "scene_id": scene, "timer": "MRQ frameRenderDuration (EXR header, UE 5.8 source-anchored)"}
        ub = run_cmd(
            [sys.executable, str(UE_RUN), str(UE_BENCH_BUILD)],
            timeout=3600,
            env_extra={
                "G10_5_SCENE": scene,
                "G10_5_CONTRACT": str(CORPUS / f"contract_params_{scene.replace('-', '_')}.json"),
                "G10_5_BENCH_FRAMES": str(UE_TOTAL_FRAMES),
            },
        )
        durations: list[float] = []
        if ub.returncode == 0:
            ur = run_cmd([sys.executable, str(UE_BENCH_RUN), scene], timeout=7200)
            if ur.returncode == 0:
                try:
                    all_ms = ue_frame_durations_ms(scene)
                    durations = all_ms[WARMUP:]
                    leg["engine_warmup_frames"] = 64
                    leg["warmup_frames_dropped"] = all_ms[:WARMUP]
                except RuntimeError as e:
                    check(False, f"UE bench 帧时长解析失败（{scene}）: {e}")
            else:
                check(False, f"UE bench MRQ 非零退出（{scene}）")
        else:
            check(False, f"UE bench 资产建设非零退出（{scene}）")
        if durations:
            leg.update({
                "warmup_count": WARMUP,
                "timed_count": len(durations),
                "samples_ms": durations,
                "frames_dir": f"K:/rurix-ext/g10-frames/g10_5/bench/ue/{scene}",
            })
        t1 = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        leg["utc_start"], leg["utc_end"] = t0, t1
        legs[leg_key] = leg
        sampling_order.append({"seq": seq, "end": "ue5", "scene_id": scene, "utc_start": t0, "utc_end": t1})
        note(f"采样腿 {seq} ue5@{scene} 完成（{t0}~{t1}）")

    # ---- ⑤ 真跑判据：双腿双场景样本齐备 + 锚断言 ----
    rurix_ok = all(
        legs.get((s, "rurix"), {}).get("timed_count") == TIMED
        and legs[(s, "rurix")].get("digest_binds_ab_library") is True
        and legs[(s, "rurix")].get("profile") == "release"
        and legs[(s, "rurix")].get("distinct_frame_digests") == 1
        for s in SCENES
    )
    checks["rurix_benchmark_real_run_release"] = rurix_ok
    check(rurix_ok, "Rurix benchmark 真跑面不全（release/digest 锚/确定性/轮数）")
    ue_ok = all(legs.get((s, "ue5"), {}).get("timed_count") == TIMED for s in SCENES)
    checks["ue_benchmark_real_run_mrq"] = ue_ok
    check(ue_ok, "UE benchmark 真跑面不全（MRQ 160 帧/弃计 10/timed 150）")

    # ---- ⑥ 采样轮数机核（不足冒充即 RED 的判据面） ----
    rounds_problems = [
        f"{s}/{e}: {p}"
        for s in SCENES for e in ("rurix", "ue5")
        for p in sampling_rounds_problems(legs.get((s, e), {}))
    ]
    checks["sampling_protocol_rounds_complete"] = not rounds_problems
    check(not rounds_problems, f"采样轮数不足: {rounds_problems[:3]}")

    # ---- ⑦ 双端交替采样顺序登记 ----
    order_ok = (
        [(o["end"], o["scene_id"]) for o in sampling_order]
        == [("rurix", "cornell-box"), ("ue5", "cornell-box"), ("rurix", "bistro-interior"), ("ue5", "bistro-interior")]
        and all(o["utc_start"] <= o["utc_end"] for o in sampling_order)
        and [o["seq"] for o in sampling_order] == [1, 2, 3, 4]
    )
    checks["alternating_sampling_order_registered"] = order_ok
    check(order_ok, f"交替采样顺序登记异常: {[(o['end'], o['scene_id']) for o in sampling_order]}")

    # ---- ⑧ 统计 + 独立重算核验 ----
    stats_ok = True
    for s in SCENES:
        for e in ("rurix", "ue5"):
            leg = legs.get((s, e), {})
            samples = leg.get("samples_ms", [])
            if len(samples) != TIMED:
                stats_ok = False
                continue
            st = block_stats(samples)
            leg["stats"] = st
            if not recompute_check(samples, st):
                check(False, f"统计独立重算不一致（{s}/{e}）")
                stats_ok = False
    checks["stats_recompute_match"] = stats_ok
    check(stats_ok, "统计面独立重算核验失败")

    # ---- ⑨ 基线值登记（只建数据，不设通过线） ----
    baseline_ok = all(
        legs.get((s, e), {}).get("stats", {}).get("trimmed_mean_ms", 0) > 0
        and legs[(s, e)]["stats"].get("fps", 0) > 0
        for s in SCENES for e in ("rurix", "ue5")
    )
    checks["baseline_values_registered"] = baseline_ok
    check(baseline_ok, "基线值登记缺失（trimmed_mean/fps 非正）")

    # ---- RED 臂①：未锁频登记缺失 ⇒ 画像核验必拒（clocks 键缺失检出 ∧
    # 当次未锁频时 clock_lock_note 诚实存档非空） ----
    env_no_clocks = {k: v for k, v in env.items() if k != "clocks"}
    red1 = bool(env_profile_problems(env_no_clocks)) and (locked or bool(clock_lock_note))
    checks["red_unlocked_registration_missing_detected"] = red1
    check(red1, "RED 臂失效：锁频登记缺失面未检出")

    # ---- RED 臂②：画像缺字段注入 ⇒ 必拒 ----
    env_missing = dict(env)
    env_missing.pop("driver_version", None)
    red2 = bool(env_profile_problems(env_missing))
    checks["red_profile_missing_field_detected"] = red2
    check(red2, "RED 臂失效：画像缺字段未检出")

    # ---- RED 臂③：采样轮数不足冒充 ⇒ 必拒 ----
    synth_leg = {"warmup_count": WARMUP, "timed_count": TIMED, "samples_ms": [1.0] * 100}
    red3 = bool(sampling_rounds_problems(synth_leg))
    checks["red_round_shortfall_masquerade_detected"] = red3
    check(red3, "RED 臂失效：轮数不足冒充未检出")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    perf_report = {
        "sampling_order": sampling_order,
        "legs": {
            f"{s}::{e}": {
                k: v for k, v in legs.get((s, e), {}).items()
                if k not in ("samples_ms", "warmup_ms", "warmup_frames_dropped")
            }
            | {
                "samples_ms": legs.get((s, e), {}).get("samples_ms", []),
                "warmup_ms": legs.get((s, e), {}).get("warmup_ms", legs.get((s, e), {}).get("warmup_frames_dropped", [])),
            }
            for s in SCENES for e in ("rurix", "ue5")
        },
        "stats_caliber": "逐 trial 块 IQR 去离群（numpy linear Q1/Q3，1.5·IQR 围栏）→ 块中位数 → 3 块均值；cv=stdev/mean（留样）；ci95=bootstrap 2000（default_rng(42)）三块中位数均值 2.5/97.5 百分位",
        "clock_lock_note": clock_lock_note,
        "zero_pass_line": "G10 零通过线维持：本基线只建数据，不设帧率通过判定（契约 G-G10-7 / 立项裁决 5；G14 性能优化期承接面）",
    }

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G10.5",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": base_commit,
        "host_section_pass": host_pass,
        "device_section_state": "executed",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "perf_report": perf_report,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
            "gpu": env,
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=executed")
    for s in SCENES:
        for e in ("rurix", "ue5"):
            st = legs.get((s, e), {}).get("stats")
            if st:
                print(f"[{TAG}] baseline {s}/{e}: trimmed_mean={st['trimmed_mean_ms']:.4f} ms → {st['fps']:.3f} fps（cv={st['cv']:.4f}）")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（双端双场景 14 §5 采样 + 交替顺序登记 + 画像齐备 + RED 三臂全检出；未锁频诚实存档）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
