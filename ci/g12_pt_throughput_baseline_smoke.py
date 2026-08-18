#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.5 吞吐基线波）
"""G12.5 M165 PT 吞吐优化基线门（P0，步骤 228；g12.p0.m165.pt_throughput_baseline；
G12_CONTRACT §4.2 M165 行判据逐字 / G-G12-7；G12_ACCEPTANCE_MAP §1 M165 行；
CI_GATES §4/§7；milestones/m0/BENCH_PROTOCOL.md §3；RFC-0029 §4.7 基线锚消费面；
G10.5 M141 50×3 trimmed mean 同协议继承）。

host+device 门（device_section_state=executed——生产化 PT megakernel device 真跑，
release harness + RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1，采样腿持
gpu_device_lock 串行防并发污染计时面）。判据（契约 §4.2 M165 行字面）：

1. **吞吐基线 measured**：rays/sec + 帧时 at 固定 spp × 场景集——场景 = M133
   双场景闭集 {cornell-box, bistro-interior}（清单 digest 转引只读），固定 spp
   档位 {16, 64}（自 G12.4 对标序列 [1,4,16,64,256,1024] 取，登记进 evidence）；
   50×3 trimmed mean 协议（BENCH_PROTOCOL §3 同族：warmup 10 + timed 150 =
   3 块 × 50，逐块 IQR 去离群 → 块中位数 → 3 块均值；cv / bootstrap ci95 定种
   ——M141 冻结统计口径同字面继承）。计时口径 = host Instant 墙钟 around
   run_device 全帧（G12.4 生产化出图路径逐帧全链路：host RNG 流生成 + 打包 +
   Vulkan 初始化/BLAS 构建/dispatch/回读同步）；rays/sec 口径 = 主射线（像素数
   × spp，次级射线未计——口径显式登记不冒充全光线计数）。
2. **入 g12_budget provenance 齐备**：8 条目（frame_ms ×4 + primary_rays_sec
   ×4，场景 × spp 闭集）measured_local 零 estimated，逐条目 evidence 落盘
   （results.trimmed_mean 通用路判读，零新 evaluator 分支）+ budget_eval 全
   PASS；阈 = 实测 ×1.5（min 向 ÷1.5，沿 G9.1/G10.1/G11.1/G12.1 measured 冻结
   先例覆盖频率漂移）——**回归守护语义，非通过线**。
3. **不设通过线登记**：evidence zero_pass_line 字面 + 8 条目描述逐个携带
   「不构成帧率对标通过线」字面——正式帧率对标锚定 G14（G10-N11/N16 承接锚
   字面维持）；以基线冒充帧率对标即 RED。
4. **优化前后正确性锚**：固定 seed digest 0-byte——逐 cell 基准帧内容 digest
   == M163 Rurix 臂 receipt 冻结锚（G12.4 对标核验面，seed=9182346301 固定
   seed 确定性协议 RXS-0357 L2/RXS-0400 继承）+ 全 160 帧 distinct digest==1
   + 演进位 null（本波零优化演进——digest 漂移未登记即 RED）。

RED 臂（契约判据字面）：基线冒充帧率对标 / digest 漂移未登记 / estimated 冒充
measured——各臂注入必检出（--selftest 合成面 + 门内真跑篡改臂）。

用法：
  py -3 ci/g12_pt_throughput_baseline_smoke.py --gate g12.p0.m165.pt_throughput_baseline
  py -3 ci/g12_pt_throughput_baseline_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import re
import statistics
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g12" / "g12_m165_pt_throughput_baseline_evidence_schema.json"
ENTRY_SCHEMA_PATH = ROOT / "milestones" / "g12" / "g12_m165_baseline_entry_evidence_schema.json"
CONTRACT_PATH = ROOT / "milestones" / "g12" / "g12_ue_pt_parity_contract.json"
WORK_DIR = ROOT / ".tmp" / "g12_gates" / "pt_throughput"

sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "ci"))
from bench import env_probe  # noqa: E402

import g12_pt_prod_lib as gl  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402,F401

GATE_KEY = "g12.p0.m165.pt_throughput_baseline"
NUMERIC_STEP = 228
SUBJECT = "g12_m165_pt_throughput_baseline"
MATRIX_ROW = "M165"
WAVE = "G12.5"
TAG = "g12_m165"
SOURCE_REF = (
    "G12_CONTRACT §4.2 M165 + G-G12-7;G12_ACCEPTANCE_MAP §1 M165;CI_GATES §4/§7;"
    "milestones/m0/BENCH_PROTOCOL.md §3;G10.5 M141 50×3 trimmed mean 同协议继承;"
    "RFC-0029 §4.7 基线锚消费面;正确性锚 = M163 Rurix 臂 receipt 冻结 digest"
)
FROZEN_CONTRACT_DIGEST = (
    "sha256:4515625e0797e500c95e9903bcced286976902327166155e4f75bf4804ac77b4"
)
SCENES = ("cornell-box", "bistro-interior")
SPP_CELLS = (16, 64)  # 固定 spp 档位（自 G12.4 对标序列 [1,4,16,64,256,1024] 取）
SEED = 9182346301  # 契约固定 seed（G12.4 parity contract seed 字面）
GLTF_PATHS = {
    "cornell-box": r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf",
    "bistro-interior": r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf",
}
# 正确性锚冻结注册面：M163 Rurix 臂 receipt frame_content_digest（G12.4 对标
# 核验面真跑件 K:\rurix-ext\g12-frames\rurix_pt\<scene>\main\<scene>_spp<N>_
# receipt.json；seed=9182346301 固定 seed 确定性协议面确定函数输出；本波
# g12_4_ue_pt_parity_render --benchmark 同配置复算逐字复核——digest 漂移未
# 登记即 RED 的比对基准）。
FROZEN_FRAME_DIGESTS = {
    ("cornell-box", 16): "sha256:668cc664af7caab7a2eae817a9241b7424c180ab080c284059afcee63250a143",
    ("cornell-box", 64): "sha256:3abd3122beef0fee96abda9ccf9e9c4572d1c54ff4f85d24ea394a76ae0f7398",
    ("bistro-interior", 16): "sha256:bb803d8ec77936354fcfeff3eaf821aea8b761ed8e371d37de4778ef4baf5d39",
    ("bistro-interior", 64): "sha256:02609bbe88789ed20fe94fc000c50f936ab57da911025ddeef8861b939a29529",
}
WARMUP = 10
TIMED = 150
BLOCKS = 3
BLOCK_SIZE = 50

REQUIRED_ENV_FIELDS = [
    "gpu_name", "driver_version", "nvml_version", "cuda_driver_version",
    "driver_model", "hags_enabled", "tdr", "clocks", "thermal",
    "isolation_check", "os_build",
]

NO_PASS_LINE_LITERAL = "不构成帧率对标通过线"
ZERO_PASS_LINE = (
    "本基线不构成帧率对标通过线——只建 PT 吞吐基线数据（rays/sec + 帧时，G14 备料）；"
    "正式帧率对标锚定 G14（G10-N11/N16 承接锚字面维持）；budget 8 条目阈 = 实测 ×1.5"
    "（min 向 ÷1.5）为生产化回归守护语义（吞吐不得降级），非帧率对标通过线。"
)
FORBIDDEN_PASS_LINE_CLAIMS = [
    "帧率对标通过线：", "帧率对标通过线:", "fps 对标通过", "FPS 对标通过",
    "帧率对标 UE5 通过", "帧率对标即 PASS", "帧率对标即PASS", "ue5 帧率通过",
]

CHECK_KEYS = [
    "l0_environment_profile_complete",
    "clock_lock_state_registered",
    "benchmark_real_run_release",
    "sampling_protocol_rounds_complete",
    "sampling_order_registered",
    "stats_recompute_match",
    "throughput_baseline_measured",
    "correctness_anchor_digest_match",
    "within_run_digest_deterministic",
    "budget_entries_measured_local",
    "no_pass_line_registered",
    "red_fps_parity_masquerade_detected",
    "red_digest_drift_unregistered_detected",
    "red_estimated_masquerade_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def run_cmd(argv: list[str], timeout: int = 7200, env: dict | None = None) -> subprocess.CompletedProcess:
    print(f"[{TAG}] $ {' '.join(str(a) for a in argv)[:220]}", flush=True)
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(str(a) for a in argv)[-400:], "exit_code": r.returncode})
    return r


# ---------------------------------------------------------------------------
# 统计口径（M141 冻结同字面：块 IQR 去离群 → 块中位数 → 3 块均值；bootstrap
# ci95 定种 default_rng(42)）
# ---------------------------------------------------------------------------

def block_stats(samples: list[float]) -> dict:
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
    )


# ---------------------------------------------------------------------------
# 环境画像 / 采样轮数 / 锚比对 / budget 登记校验（判据机器求值面）
# ---------------------------------------------------------------------------

def env_profile_problems(env: dict) -> list[str]:
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
    """采样轮数机核（warmup ≥10 ∧ timed == 150 = 3×50 块；不足冒充即问题行）。"""
    problems = []
    if leg.get("warmup_count", 0) < WARMUP:
        problems.append(f"warmup {leg.get('warmup_count')} < {WARMUP}")
    if leg.get("timed_count") != TIMED:
        problems.append(f"timed {leg.get('timed_count')} ≠ {TIMED}")
    if len(leg.get("samples_ms", [])) != TIMED:
        problems.append(f"原始样本数 {len(leg.get('samples_ms', []))} ≠ {TIMED}")
    return problems


def digest_anchor_problems(scene: str, spp: int, doc: dict) -> list[str]:
    """正确性锚比对：固定 seed digest 0-byte（== M163 冻结锚）+ 全帧 digest
    单值（确定性协议）；漂移未登记即问题行。"""
    problems = []
    anchor = FROZEN_FRAME_DIGESTS.get((scene, spp))
    if anchor is None:
        return [f"({scene}, spp{spp}) 无冻结锚注册"]
    if doc.get("first_frame_digest") != anchor:
        problems.append(
            f"{scene} spp{spp} 帧 digest {doc.get('first_frame_digest')} ≠ M163 冻结锚 {anchor}（digest 漂移未登记即 RED）"
        )
    if doc.get("distinct_frame_digests") != 1:
        problems.append(
            f"{scene} spp{spp} 全帧 digest 非单值（{doc.get('distinct_frame_digests')}，确定性协议破缺）"
        )
    return problems


def baseline_registration_problems(entry: dict) -> list[str]:
    """budget 基线条目登记校验：measured_local + 不设通过线字面 + 零帧率对标
    通过线声明（冒充即问题行）。"""
    problems = []
    if entry.get("evidence") != "measured_local":
        problems.append(f"evidence={entry.get('evidence')} 非 measured_local（estimated 冒充即 RED）")
    desc = entry.get("description") or ""
    if NO_PASS_LINE_LITERAL not in desc:
        problems.append("缺不设通过线登记字面（不构成帧率对标通过线）")
    for bad in FORBIDDEN_PASS_LINE_CLAIMS:
        if bad in desc:
            problems.append(f"帧率对标通过线声明注入检出面: {bad}")
    if entry.get("threshold") is None or entry.get("measured_value") is None:
        problems.append("threshold/measured_value 缺失")
    return problems


def scene_slug(scene: str) -> str:
    return scene.replace("-", "_")


def budget_entry_ids() -> list[str]:
    ids = []
    for scene in SCENES:
        for spp in SPP_CELLS:
            ids.append(f"g12.pt.throughput_frame_ms_{scene_slug(scene)}_spp{spp}")
    for scene in SCENES:
        for spp in SPP_CELLS:
            ids.append(f"g12.pt.throughput_primary_rays_sec_{scene_slug(scene)}_spp{spp}")
    return ids


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
    # 红臂①：轮数不足冒充必检出。
    short = {"warmup_count": 10, "timed_count": 150, "samples_ms": samples[:100]}
    if not sampling_rounds_problems(short):
        print(f"[{TAG}] selftest FAIL: 轮数不足冒充未检出", file=sys.stderr)
        return 1
    # 红臂②：digest 漂移未登记必检出（锚比对篡改面）。
    drift_doc = {"first_frame_digest": "sha256:" + "0" * 64, "distinct_frame_digests": 1}
    if not digest_anchor_problems("cornell-box", 16, drift_doc):
        print(f"[{TAG}] selftest FAIL: digest 漂移未检出", file=sys.stderr)
        return 1
    nondet_doc = {
        "first_frame_digest": FROZEN_FRAME_DIGESTS[("cornell-box", 16)],
        "distinct_frame_digests": 2,
    }
    if not digest_anchor_problems("cornell-box", 16, nondet_doc):
        print(f"[{TAG}] selftest FAIL: 全帧 digest 非单值未检出", file=sys.stderr)
        return 1
    if digest_anchor_problems(
        "cornell-box", 16,
        {"first_frame_digest": FROZEN_FRAME_DIGESTS[("cornell-box", 16)], "distinct_frame_digests": 1},
    ):
        print(f"[{TAG}] selftest FAIL: 合锚样本被误拒", file=sys.stderr)
        return 1
    # 红臂③：基线冒充帧率对标必检出 + estimated 冒充必检出；绿臂②：合形条目不误拒。
    good_entry = {
        "evidence": "measured_local",
        "description": f"……回归守护语义，{NO_PASS_LINE_LITERAL}（正式帧率对标锚定 G14）……",
        "threshold": 1.0,
        "measured_value": 0.5,
    }
    if baseline_registration_problems(good_entry):
        print(f"[{TAG}] selftest FAIL: 合形条目被误拒 {baseline_registration_problems(good_entry)}", file=sys.stderr)
        return 1
    masquerade = dict(good_entry)
    masquerade["description"] = "吞吐基线 = 帧率对标通过线： rurix fps ≥ ue5 fps 即 PASS"
    if not baseline_registration_problems(masquerade):
        print(f"[{TAG}] selftest FAIL: 基线冒充帧率对标未检出", file=sys.stderr)
        return 1
    estimated = dict(good_entry)
    estimated["evidence"] = "estimated"
    if not baseline_registration_problems(estimated):
        print(f"[{TAG}] selftest FAIL: estimated 冒充未检出", file=sys.stderr)
        return 1
    # 绿臂③：环境画像校验面（全字段不误拒 + 缺字段必拒）。
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
    # 绿臂④：schema checks.required 与 CHECK_KEYS 闭集精确互核 + budget 条目 id 闭集 8 行。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    if len(budget_entry_ids()) != 8 or len(set(budget_entry_ids())) != 8:
        print(f"[{TAG}] selftest FAIL: budget 条目 id 闭集非 8 行唯一", file=sys.stderr)
        return 1
    entry_schema = json.loads(ENTRY_SCHEMA_PATH.read_text(encoding="utf-8")) if ENTRY_SCHEMA_PATH.is_file() else {}
    enum = set(entry_schema.get("properties", {}).get("entry_id", {}).get("enum", []))
    if enum != set(budget_entry_ids()):
        print(f"[{TAG}] selftest FAIL: entry schema entry_id 枚举与 budget id 闭集不等", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (4 RED + 6 GREEN)")
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

    # ---- ② 锁频状态实测登记（未锁频 → clock_lock_note 诚实存档，G10.1/G12.1 先例） ----
    clocks = env.get("clocks", {})
    locked = bool(clocks.get("locked"))
    clock_lock_note = ""
    if not locked:
        clock_lock_note = (
            f"GPU 未锁频（NVML 采样探测 locked=false，sm {clocks.get('sm_clock_mhz')}MHz / "
            f"mem {clocks.get('mem_clock_mhz')}MHz；锁频需提权 nvidia-smi -lgc/-lmc，本门未执行）"
            "——沿 G10.1/G12.1 baseline 先例以 measured_local 登记并将未锁频边界诚实存档；"
            "本门只建基线数据，不设帧率对标通过线，未锁频漂移风险如实登记不掩盖；"
            "×1.5/÷1.5 回归守护余量沿 G9.1/G10.1/G11.1/G12.1 measured 冻结先例覆盖频率漂移。"
        )
    checks["clock_lock_state_registered"] = ("locked" in clocks) and (locked or bool(clock_lock_note))
    check(checks["clock_lock_state_registered"], "锁频状态登记缺失（未锁频登记缺失即 RED）")

    # ---- ③ 产线：rurixc + SPV + release harness（既有面 0-byte 消费） ----
    with gpu_device_lock(purpose=f"{TAG} 采样腿（计时面串行）"):
        rurixc = gl.build_rurixc()
        spv = WORK_DIR / "g12_pt_production.spv"
        spv_ok = bool(rurixc) and gl.compile_spv(rurixc, spv)
        rb = run_cmd(["cargo", "build", "--release", "-p", "rurix-asset", "--features", "vulkan",
                      "--bin", "g12_4_ue_pt_parity_render"], timeout=3600)
        bench_bin = gl.target_dir() / "release" / "g12_4_ue_pt_parity_render.exe"
        build_ok = rb.returncode == 0 and bench_bin.is_file() and spv_ok
        check(build_ok, f"产线失败（rurixc/SPV/harness）rc={rb.returncode} spv={spv_ok}")

        budget = gl.load_budget()
        tau_entry = gl.budget_entry(budget, "g12.pt.rr_tau")
        tau = float(tau_entry["measured_value"]) if tau_entry else 0.0
        check(tau > 0.0, "g12.pt.rr_tau 标定条目缺失（P-09 禁手写——自 g12_budget 读出）")

        # ---- ④ 采样腿：场景 × spp 闭集 4 腿（顺序登记逐腿 UTC 起止） ----
        legs: dict[tuple[str, int], dict] = {}
        sampling_order: list[dict] = []
        seq = 0
        for scene in SCENES:
            for spp in SPP_CELLS:
                seq += 1
                t0 = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
                leg: dict = {"scene_id": scene, "spp": spp, "timer": "host Instant (release profile)"}
                if build_ok and tau > 0.0:
                    r = run_cmd([
                        str(bench_bin), "--benchmark", "--scene", scene, "--spp", str(spp),
                        "--seed", str(SEED), "--tau", repr(tau),
                        "--contract", str(CONTRACT_PATH), "--gltf", GLTF_PATHS[scene],
                        "--spv", str(spv), "--warmup", str(WARMUP), "--frames", str(TIMED),
                        "--expect-digest", FROZEN_CONTRACT_DIGEST,
                    ], timeout=7200, env=gl.device_env())
                    out = r.stdout or ""
                    if "G12_4_PT: SKIP" in out:
                        check(False, f"benchmark 腿 SKIP（{scene} spp{spp}；DEV_ENV_DEGRADE 不充绿）")
                    elif r.returncode != 0:
                        check(False, f"benchmark 腿非零退出（{scene} spp{spp}）: {(r.stderr or out).strip()[-300:]}")
                    else:
                        try:
                            doc = json.loads(out.strip().splitlines()[-1])
                        except (json.JSONDecodeError, IndexError) as e:
                            doc = None
                            check(False, f"benchmark 输出解析失败（{scene} spp{spp}）: {e}")
                        if doc:
                            if doc.get("schema") != "rurix.g12.pt_throughput_bench.v1":
                                check(False, f"benchmark 输出 schema 字面不符（{scene} spp{spp}）")
                            if doc.get("contract_digest") != FROZEN_CONTRACT_DIGEST:
                                check(False, f"benchmark 契约 digest 不等（{scene} spp{spp}）")
                            if doc.get("tamper_injected") is not False:
                                check(False, f"benchmark tamper 标志异常（{scene} spp{spp}）")
                            leg.update({
                                "profile": doc.get("profile"),
                                "width": doc.get("width"),
                                "height": doc.get("height"),
                                "warmup_count": doc.get("warmup_count"),
                                "timed_count": doc.get("timed_count"),
                                "samples_ms": doc.get("frame_ms", []),
                                "warmup_ms": doc.get("warmup_ms", []),
                                "first_frame_digest": doc.get("first_frame_digest"),
                                "distinct_frame_digests": doc.get("distinct_frame_digests"),
                                "rays_per_frame": doc.get("rays_per_frame"),
                                "timer_caliber": doc.get("timer"),
                                "rays_caliber": doc.get("rays_caliber"),
                            })
                t1 = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
                leg["utc_start"], leg["utc_end"] = t0, t1
                legs[(scene, spp)] = leg
                sampling_order.append({"seq": seq, "scene_id": scene, "spp": spp, "utc_start": t0, "utc_end": t1})
                note(f"采样腿 {seq} {scene} spp{spp} 完成（{t0}~{t1}）")

        # ---- ④b RED 臂②真跑面：digest 漂移注入（tamper 子进程）锚比对必检出 ----
        tamper_doc = None
        if build_ok and tau > 0.0:
            rt = run_cmd([
                str(bench_bin), "--benchmark", "--scene", "cornell-box", "--spp", "16",
                "--seed", str(SEED), "--tau", repr(tau),
                "--contract", str(CONTRACT_PATH), "--gltf", GLTF_PATHS["cornell-box"],
                "--spv", str(spv), "--warmup", "1", "--frames", "2",
                "--expect-digest", FROZEN_CONTRACT_DIGEST,
            ], timeout=1800, env=gl.device_env() | {"G12_5_BENCH_TAMPER": "1"})
            try:
                tamper_doc = json.loads((rt.stdout or "").strip().splitlines()[-1]) if rt.returncode == 0 else None
            except (json.JSONDecodeError, IndexError):
                tamper_doc = None

    # ---- ⑤ 真跑判据：4 腿样本齐备 + release profile ----
    cells = [(scene, spp) for scene in SCENES for spp in SPP_CELLS]
    real_run_ok = all(
        legs.get(c, {}).get("timed_count") == TIMED
        and legs[c].get("profile") == "release"
        and legs[c].get("rays_per_frame") == 128 * 128 * c[1]
        for c in cells
    )
    checks["benchmark_real_run_release"] = bool(build_ok and real_run_ok)
    check(checks["benchmark_real_run_release"], "benchmark 真跑面不全（release/轮数/rays_per_frame）")

    rounds_problems = [f"{c}: {p}" for c in cells for p in sampling_rounds_problems(legs.get(c, {}))]
    checks["sampling_protocol_rounds_complete"] = not rounds_problems
    check(not rounds_problems, f"采样轮数不足: {rounds_problems[:3]}")

    order_ok = (
        [(o["scene_id"], o["spp"]) for o in sampling_order]
        == [(scene, spp) for scene in SCENES for spp in SPP_CELLS]
        and all(o["utc_start"] <= o["utc_end"] for o in sampling_order)
        and [o["seq"] for o in sampling_order] == [1, 2, 3, 4]
    )
    checks["sampling_order_registered"] = order_ok
    check(order_ok, "采样顺序登记异常")

    # ---- ⑥ 统计 + 独立重算核验（M141 冻结口径） ----
    stats_ok = True
    for c in cells:
        leg = legs.get(c, {})
        samples = leg.get("samples_ms", [])
        if len(samples) != TIMED:
            stats_ok = False
            continue
        st = block_stats(samples)
        leg["stats"] = st
        leg["primary_rays_per_sec"] = leg["rays_per_frame"] / (st["trimmed_mean_ms"] / 1000.0)
        if not recompute_check(samples, st):
            check(False, f"统计独立重算不一致（{c}）")
            stats_ok = False
    checks["stats_recompute_match"] = stats_ok
    check(stats_ok, "统计面独立重算核验失败")

    measured_ok = all(
        legs.get(c, {}).get("stats", {}).get("trimmed_mean_ms", 0) > 0
        and legs.get(c, {}).get("primary_rays_per_sec", 0) > 0
        for c in cells
    )
    checks["throughput_baseline_measured"] = measured_ok
    check(measured_ok, "吞吐基线 measured 值缺失（trimmed_mean/rays_sec 非正）")

    # ---- ⑦ 正确性锚：固定 seed digest 0-byte + 全帧确定性 + 演进位 null ----
    anchor_problems = [
        p for c in cells for p in digest_anchor_problems(c[0], c[1], legs.get(c, {}))
    ]
    checks["correctness_anchor_digest_match"] = not anchor_problems
    check(not anchor_problems, f"正确性锚 digest 漂移未登记: {anchor_problems[:2]}")
    checks["within_run_digest_deterministic"] = all(
        legs.get(c, {}).get("distinct_frame_digests") == 1 for c in cells
    )
    check(checks["within_run_digest_deterministic"], "全帧 digest 非单值（确定性协议破缺）")

    # ---- ⑧ budget 8 条目登记（缺失 → 字节级纯追加；在档 → 回归守护复检） ----
    entry_files: dict[str, str] = {}
    if measured_ok:
        budget = gl.load_budget()
        entries = budget.get("entries", [])
        changed = False
        ts_now = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        metric_map: dict[str, tuple[str, float, str, str]] = {}
        for c in cells:
            scene, spp = c
            slug = scene_slug(scene)
            fm = legs[c]["stats"]["trimmed_mean_ms"]
            rps = legs[c]["primary_rays_per_sec"]
            metric_map[f"g12.pt.throughput_frame_ms_{slug}_spp{spp}"] = (
                "ms", fm, "max",
                f"PT 吞吐基线帧时（{scene} spp{spp}，128×128，固定 seed {SEED}，生产化出图路径逐帧全链路墙钟；"
                f"50×3 trimmed mean 协议〔BENCH_PROTOCOL §3 同族，M141 同口径〕）",
            )
            metric_map[f"g12.pt.throughput_primary_rays_sec_{slug}_spp{spp}"] = (
                "rays/s", rps, "min",
                f"PT 吞吐基线主射线吞吐（{scene} spp{spp}，128×128，主射线口径 = 像素数 × spp〔次级射线未计，"
                f"显式登记不冒充全光线计数〕；50×3 trimmed mean 协议帧时派生）",
            )
        for eid, (unit, measured, direction, desc_head) in metric_map.items():
            thr = measured * 1.5 if direction == "max" else measured / 1.5
            existing = gl.budget_entry(budget, eid)
            if existing is None:
                samples = legs[(next(s for s in SCENES if scene_slug(s) in eid),
                                int(re.search(r"_spp(\d+)$", eid).group(1)))]["samples_ms"]
                sample_digest = hashlib.sha256(
                    json.dumps(samples, sort_keys=True).encode("utf-8")
                ).hexdigest()
                suffix = eid.split("g12.pt.throughput_")[1]
                ev_path = EVIDENCE_DIR / f"g12_m165_baseline_{suffix}_{ts_now}.json"
                entry_ev = {
                    "schema": "rurix.g12pt.throughput_baseline_entry.v1",
                    "entry_id": eid,
                    "results": {"trimmed_mean": measured},
                    "protocol": desc_head + "；统计口径 = 逐块 IQR 去离群（numpy linear Q1/Q3，1.5·IQR 围栏）→ 块中位数 → 3 块均值（M141 冻结口径）",
                    "sample_manifest": {
                        "count": TIMED,
                        "digest": "sha256:" + sample_digest,
                        "lower_bound": TIMED,
                    },
                    "provenance": {
                        "seed": str(SEED),
                        "host": "g12_4_ue_pt_parity_render --benchmark release device 真跑（持锁串行）",
                        "timer": "host Instant 墙钟 around run_device 全帧（生产化出图路径逐帧全链路）",
                    },
                    "no_pass_line_declaration": ZERO_PASS_LINE,
                    "timestamp": ts_now,
                }
                ev_path.write_text(json.dumps(entry_ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                entries.append(
                    {
                        "id": eid,
                        "description": (
                            f"{desc_head}。G12.5 M165 吞吐优化基线 measured（G14 备料）——回归守护阈 = 实测 "
                            f"{'×1.5' if direction == 'max' else '÷1.5'}（沿 G9.1/G10.1/G11.1/G12.1 measured 冻结先例覆盖频率漂移），"
                            f"{NO_PASS_LINE_LITERAL}（正式帧率对标锚定 G14，G10-N11/N16 承接锚字面维持）；"
                            f"样本集 digest sha256:{sample_digest}(count={TIMED} ≥ {TIMED})；测量程序 "
                            f"ci/g12_pt_throughput_baseline_smoke.py 可复跑"
                        ),
                        "direction": direction,
                        "evidence": "measured_local",
                        "skip_reason": None,
                        "unit": unit,
                        "threshold": thr,
                        "evidence_file": str(ev_path.relative_to(ROOT)).replace("\\", "/"),
                        "measured_value": measured,
                    }
                )
                entry_files[eid] = str(ev_path.relative_to(ROOT)).replace("\\", "/")
                changed = True
                note(f"基线条目追加: {eid} = {measured:.6e} {unit}（守护阈 {thr:.6e}）")
            else:
                # 在档复跑 = 回归守护复检（零降级语义：帧时 ≤ 在档阈 ∧ 吞吐 ≥ 在档阈）；
                # 条目与首跑 evidence 0-byte（基线锚不回写——G12.7a/后续复测重登记另波裁决）。
                entry_files[eid] = existing.get("evidence_file", "")
                if existing.get("evidence") != "measured_local":
                    check(False, f"基线条目非 measured_local: {eid}")
                elif direction == "max" and not (measured <= float(existing["threshold"])):
                    check(False, f"基线回归守护复检失败: {eid} 复测 {measured:.6e} > 在档阈 {existing['threshold']}")
                elif direction == "min" and not (measured >= float(existing["threshold"])):
                    check(False, f"基线回归守护复检失败: {eid} 复测 {measured:.6e} < 在档阈 {existing['threshold']}")
                else:
                    note(f"基线条目在档守护复检 PASS: {eid} 复测 {measured:.6e} vs 阈 {float(existing['threshold']):.6e}")
        if changed:
            budget["entries"] = entries
            gl.BUDGET_PATH.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        r = run_cmd(["py", "-3", "ci/budget_eval.py"], timeout=1200)
        budget_now = gl.load_budget()
        reg_ok = r.returncode == 0
        for eid in budget_entry_ids():
            e = gl.budget_entry(budget_now, eid)
            if e is None:
                reg_ok = False
                check(False, f"budget 缺基线条目 {eid}")
                continue
            probs = baseline_registration_problems(e)
            if probs:
                reg_ok = False
                check(False, f"基线条目登记面问题 {eid}: {probs}")
            ef = e.get("evidence_file", "")
            if not (ROOT / ef).is_file():
                reg_ok = False
                check(False, f"基线条目 evidence_file 缺失: {eid} -> {ef}")
        checks["budget_entries_measured_local"] = reg_ok
        check(reg_ok, "budget 8 基线条目非 measured_local/登记面问题/budget_eval 非 PASS")

    # ---- ⑨ 不设通过线登记（evidence 字面 + 条目字面 + 演进位 null） ----
    checks["no_pass_line_registered"] = (
        bool(ZERO_PASS_LINE)
        and NO_PASS_LINE_LITERAL in ZERO_PASS_LINE
        and "G14" in ZERO_PASS_LINE
        and all(
            NO_PASS_LINE_LITERAL in (gl.budget_entry(gl.load_budget(), eid) or {}).get("description", "")
            for eid in budget_entry_ids()
        )
    )
    check(checks["no_pass_line_registered"], "不设通过线登记缺失（evidence/条目字面）")

    # ---- RED 臂①：基线冒充帧率对标 ⇒ 登记校验必拒 ----
    synth_masquerade = {
        "evidence": "measured_local",
        "description": "吞吐基线 = 帧率对标通过线： rurix fps ≥ ue5 fps 即 PASS",
        "threshold": 1.0,
        "measured_value": 0.5,
    }
    red1 = bool(baseline_registration_problems(synth_masquerade))
    checks["red_fps_parity_masquerade_detected"] = red1
    check(red1, "RED 臂失效：基线冒充帧率对标未检出")

    # ---- RED 臂②：digest 漂移未登记 ⇒ 锚比对必拒（真跑 tamper 子进程 + 合成面双保险） ----
    red2 = tamper_doc is not None and tamper_doc.get("tamper_injected") is True
    if red2:
        red2 = bool(digest_anchor_problems("cornell-box", 16, tamper_doc))
    red2 = red2 and bool(
        digest_anchor_problems("cornell-box", 16, {"first_frame_digest": "sha256:" + "0" * 64, "distinct_frame_digests": 1})
    )
    checks["red_digest_drift_unregistered_detected"] = red2
    check(red2, "RED 臂失效：digest 漂移未登记未检出（tamper 真跑/合成面）")

    # ---- RED 臂③：estimated 冒充 measured ⇒ 登记校验必拒 ----
    synth_estimated = {
        "evidence": "estimated",
        "description": f"……{NO_PASS_LINE_LITERAL}……",
        "threshold": 1.0,
        "measured_value": 0.5,
    }
    red3 = bool(baseline_registration_problems(synth_estimated))
    checks["red_estimated_masquerade_detected"] = red3
    check(red3, "RED 臂失效：estimated 冒充未检出")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    baseline_section = {
        "cells": [
            {
                "scene_id": c[0],
                "spp": c[1],
                "width": legs.get(c, {}).get("width"),
                "height": legs.get(c, {}).get("height"),
                "warmup_count": legs.get(c, {}).get("warmup_count"),
                "timed_count": legs.get(c, {}).get("timed_count"),
                "stats": legs.get(c, {}).get("stats"),
                "primary_rays_per_sec": legs.get(c, {}).get("primary_rays_per_sec"),
                "rays_per_frame": legs.get(c, {}).get("rays_per_frame"),
                "first_frame_digest": legs.get(c, {}).get("first_frame_digest"),
                "anchor_digest": FROZEN_FRAME_DIGESTS[c],
                "digest_match": legs.get(c, {}).get("first_frame_digest") == FROZEN_FRAME_DIGESTS[c],
                "distinct_frame_digests": legs.get(c, {}).get("distinct_frame_digests"),
                "utc_start": legs.get(c, {}).get("utc_start"),
                "utc_end": legs.get(c, {}).get("utc_end"),
                "samples_ms": legs.get(c, {}).get("samples_ms", []),
                "warmup_ms": legs.get(c, {}).get("warmup_ms", []),
            }
            for c in cells
        ],
        "sampling_order": sampling_order,
        "stats_caliber": "逐 trial 块 IQR 去离群（numpy linear Q1/Q3，1.5·IQR 围栏）→ 块中位数 → 3 块均值；cv=stdev/mean（留样）；ci95=bootstrap 2000（default_rng(42)）三块中位数均值 2.5/97.5 百分位（M141 冻结口径同字面继承）",
        "timer_caliber": "host Instant 墙钟 around run_device 全帧（G12.4 生产化出图路径逐帧全链路：host RNG 流生成 + 打包 + Vulkan 初始化/BLAS 构建/dispatch/回读同步）",
        "rays_caliber": "主射线口径 = 像素数 × spp（次级射线未计——显式登记不冒充全光线计数）",
        "fixed_spp_cells": list(SPP_CELLS),
        "scene_set": list(SCENES),
        "clock_lock_note": clock_lock_note,
        "zero_pass_line": ZERO_PASS_LINE,
        "correctness_anchor": {
            "kind": "固定 seed digest 0-byte",
            "source": "M163 Rurix 臂 receipt frame_content_digest 冻结锚（G12.4 对标核验面真跑件；seed=9182346301 固定 seed 确定性协议 RXS-0357 L2/RXS-0400 继承）",
            "frozen_digests": {f"{s}|spp{n}": d for (s, n), d in FROZEN_FRAME_DIGESTS.items()},
            "evolution_register": None,
        },
        "budget_entry_ids": budget_entry_ids(),
        "budget_entry_evidence_files": entry_files,
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
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": base_commit,
        "host_section_pass": host_pass,
        "device_section_state": "executed",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "baseline": baseline_section,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": __import__("platform").platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": gl.tool_version("cargo"),
            "rustc_version": gl.tool_version("rustc"),
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
    for c in cells:
        st = legs.get(c, {}).get("stats")
        if st:
            rps = legs[c]["primary_rays_per_sec"]
            print(f"[{TAG}] baseline {c[0]} spp{c[1]}: frame={st['trimmed_mean_ms']:.3f} ms → {rps:.3f} primary rays/s（cv={st['cv']:.4f}）")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（4 cell 50×3 采样 + digest 锚 0-byte + budget 8 条目 measured_local + 不设通过线登记 + RED 三臂全检出）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
