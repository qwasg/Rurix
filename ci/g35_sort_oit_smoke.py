#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: cursor:claude-fable-5(G35 GPU 粒子系统 G35-4 半透明双臂)
"""G35-4:半透明双臂门冒烟(g35.wave4.sort_oit;RFC-0049 §4.8 评审修订后
冻结协议——臂 A tile 排序 back-to-front〔复合键 tile_id·4096+(4095−depth12)
反键,radix 稳定序 + 每像素区间升序固定序串行合成 = 位级确定基准〕+ 臂 B
WBOIT 定点整数累加〔u32 Q12 SCALE=4096 floor 舍入 + 加前 clamp 饱和 +
cap ≤ 65536 结构性防回绕,可交换 ⇒ 顺序无关位级〕;host 金标准 =
src/rurix-render/src/particles/oit_arms.rs;车道承载 = bin/g35_particle_lane.rs
加性扩展 --oit off|sorted|wboit 三档闭集,off = G35-3 现面零追加)。

九面判据(facts 闭集):
1. **kernels_spv_valid**:rurixc 现编 24 kernel(车道五件 + 粒子七件〔W1/W2
   冻结消费面〕+ 渲染三件〔G35-3 冻结消费面〕+ sort 三件〔W1 冻结消费面〕+
   OIT 六件〔g35_hash_clear W7 0-byte 消费 + 本波五件 g35_oit_tilekey/
   g35_oit_tilerange/g35_oit_blend_sorted/g35_oit_wboit_accum/
   g35_oit_wboit_resolve〕)+ spirv-val 全绿 + 冻结消费面 sha256 快照在档。
2. **sorted_arm_bitexact**:--oit sorted 同参数双跑 render_digest +
   digest_seq_sha 双通道位级一致(radix 稳定序 = tile 段 × 反深度段 × slot
   段内下标序 + 每像素固定序串行 over,与线程调度无关)。
3. **wboit_fixedpoint_saturation**:--oit wboit 双跑双通道位级一致(整数
   fetch_add 可交换)∧ 见证腿 device acc(Q12 u32×4px)vs host 定点累加
   金标准最大整数差 ≤ 冻结容差(milestones/g35/g35_budget.json
   g35.oit.wboit_acc_tol 程序读禁手写:threshold = measured × 2.0 标定冻结;
   缺条目时本腿即标定腿程序写入)∧ 饱和事件计数(device/host)如实登记
   ≥ 0(饱和语义 = 加前 clamp delta ≤ 65535 = 2^16−1,oit_arms.rs 单测
   wboit_saturation_clamp_and_no_overflow 覆盖触发面)。
4. **near_far_order_witness**:--oit-witness 近远两粒子视轴夹具(帧 0/30 各
   发 1,纯前向 0.4 m/s ⇒ 先发者更远且 age 大偏红):sorted 臂末帧全帧
   scene_color vs host 金标准期望(进程内 --oit off 影面基底 + oit_arms
   far→near painter's 串行合成)p100 ≤ 冻结容差(g35.oit.parity_p100 程序
   读/标定同上)∧ 合成改动像素 ≥ 1(远者先画的机器证明经 host 期望承载:
   oit_arms 单测 near_far_order_witness_and_red_arm_flip 证期望 = 远先近后
   over 链位级)。
5. **oit_arms_digest_discrimination**:--oit off 显式档 render_digest 位级
   == 缺省(不带 --oit)面(加性 0 破坏机器证明)∧ off/sorted/wboit 三臂
   digest 两两互异(同轨迹 orbit 同参数,真接线判别防镂空 pass)。
6. **oit_retrigger_registered**:milestones/g31/g31_oit_evaluation_window.json
   含 retriggers 记录(consumer = g35.wave4.sort_oit + decision 引 RFC-0049
   §4.8 与 M120 冻结测量——#13 评估窗 conditional_wiring_sketch ① 选型提交
   引 benchmark 数据纪律)∧ 既有字段(schema/frozen_at_utc/
   trigger_evaluation/conclusion)0-byte 在档。
7. **determinism_double_run**:双臂四跑(sorted×2 + wboit×2)digest +
   digest_seq_sha 全链一致汇总判(fact 2/3 的双通道并集)。
8. **red_arm_effective**:--red-arm key-invert(tilekey 去掉 4095− 反键翻转
   = 协议破坏注入):红臂 p100 vs 正协议 host 期望 > max(冻结容差, 1e-3
   名义底)∧ 红臂 digest ≠ 正臂 digest(近远翻序必检出)。
9. **frame_ms_measured**:off/sorted/wboit 三臂逐帧墙钟 + OIT pass 组 GPU
   段和 measured_local 诚实登记(非帧率对标门)。

OIT 腿构型 = **--tier 50**(bistro 内部 960×540 ⇒ tile_cnt = 60×34 = 2040
≤ 4095 键域守卫:溢出键 ≤ 4095·4096 = 16773120 < 2^24;t100 1920×1080 ⇒
8160 越域 lane 拒跑如实登记)。results.trimmed_mean = sorted 见证 p100 镜像
(ci/budget_eval.py 通用路 evidence_file 消费面;g35.oit.wboit_acc_tol 条目
共享本件,其实测承载 = wboit_witness.acc_max_int_diff,双条目零容差预期面)。

三态:无 Vulkan loader/设备/资产 → DEV_ENV_DEGRADE 退 0(不冒充 PASS);
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL(禁 mock 充真跑)。

用法:
  py -3 ci/g35_sort_oit_smoke.py --selftest
  py -3 ci/g35_sort_oit_smoke.py --gate g35.wave4.sort_oit [--frames 48] [--cap 65536] [--seed 42]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g35.wave4.sort_oit"
SUBJECT = "g35_sort_oit"
WAVE = "G35.4"
TAG = "g35_sort_oit"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g35" / "g35_sort_oit_gate_evidence_schema.json"
GATE_SCHEMA_ID = "rurix.g35.sort_oit_gate_evidence.v1"
BUDGET_PATH = ROOT / "milestones" / "g35" / "g35_budget.json"
TOL_SORTED_ID = "g35.oit.parity_p100"
TOL_WBOIT_ID = "g35.oit.wboit_acc_tol"
# 红臂检出名义底(近远夹具两粒子色差量级 ≫ 1e-3;防 threshold 过小空转)。
RED_FLOOR = 1e-3
WINDOW_PATH = ROOT / "milestones" / "g31" / "g31_oit_evaluation_window.json"
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
# OIT 腿 tier(键域守卫构型;docstring 论证)。
OIT_TIER = 50
LANE_KERNELS = (
    "g14_3_direct_gi",
    "g14_mv",
    "g14_8_tsr_resample",
    "g14_8_tsr_resolve",
    "g31_display_encode",
)
PARTICLE_KERNELS = (
    "g35_sim",
    "g35_scan_seg_sum",
    "g35_scan_spine",
    "g35_scan_seg_apply",
    "g35_particle_compact",
    "g35_emit",
    "g35_indirect_args",
)
RENDER_KERNELS = ("g35_splat_clear", "g35_render_splat", "g35_render_resolve")
SORT_KERNELS = ("g35_sort_hist", "g35_sort_spine", "g35_sort_scatter")
OIT_KERNELS = (
    "g35_hash_clear",
    "g35_oit_tilekey",
    "g35_oit_tilerange",
    "g35_oit_blend_sorted",
    "g35_oit_wboit_accum",
    "g35_oit_wboit_resolve",
)
FROZEN_CONSUMED_PATHS = [
    # G35-4 消费不修改承诺面(W1 sort 三 kernel + W7 hash_clear + G35-3 渲染
    # 三 kernel + 粒子七 kernel + 共享车道体 + host 金标准 + M120 参照臂)——
    # sha256 快照在档 = 0-byte 纪律漂移守护基线(g35_render_wiring 同律)。
    "src/rurix-render/kernels/g35_sort_hist.rx",
    "src/rurix-render/kernels/g35_sort_spine.rx",
    "src/rurix-render/kernels/g35_sort_scatter.rx",
    "src/rurix-render/kernels/g35_hash_clear.rx",
    "src/rurix-render/kernels/g35_splat_clear.rx",
    "src/rurix-render/kernels/g35_render_splat.rx",
    "src/rurix-render/kernels/g35_render_resolve.rx",
    "src/rurix-render/kernels/g35_sim.rx",
    "src/rurix-render/kernels/g35_scan_seg_sum.rx",
    "src/rurix-render/kernels/g35_scan_spine.rx",
    "src/rurix-render/kernels/g35_scan_seg_apply.rx",
    "src/rurix-render/kernels/g35_particle_compact.rx",
    "src/rurix-render/kernels/g35_emit.rx",
    "src/rurix-render/kernels/g35_indirect_args.rx",
    "src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs",
    "src/rurix-render/src/particles/mod.rs",
    "src/rurix-render/src/particles/core.rs",
    "src/rurix-render/src/particles/primitives.rs",
    "src/rurix-render/src/oit/algorithms.rs",
]
WORK = ROOT / ".tmp" / "g35_gates" / "sort_oit"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "debug" / f"g35_particle_lane{EXE_SUFFIX}"

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "kernels_spv_valid",
    "sorted_arm_bitexact",
    "wboit_fixedpoint_saturation",
    "near_far_order_witness",
    "oit_arms_digest_discrimination",
    "oit_retrigger_registered",
    "determinism_double_run",
    "red_arm_effective",
    "frame_ms_measured",
]


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def _num(v) -> bool:
    return isinstance(v, (int, float)) and not isinstance(v, bool) and v == v


def _digest(v) -> bool:
    return isinstance(v, str) and DIGEST_RE.match(v) is not None


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面;全纯函数零 GPU)
# ---------------------------------------------------------------------------


def frozen_tol(budget: dict | None, entry_id: str) -> float | None:
    """冻结容差程序读(estimated/skip_reason 冒充 measured 即 None
    fail-closed;g35_render_wiring frozen_tol 同律,带条目 id 双条目消费)。"""
    if not isinstance(budget, dict):
        return None
    for e in budget.get("entries", []):
        if e.get("id") == entry_id:
            if e.get("evidence") != "measured_local" or e.get("skip_reason"):
                return None
            t = e.get("threshold")
            return float(t) if isinstance(t, (int, float)) and not isinstance(t, bool) else None
    return None


def calib_threshold(measured: float) -> float:
    """标定协议冻结 k:threshold = measured × 2.0(measured = 0 时 = 0.0;
    程序产禁手写)。"""
    return measured * 2.0


def upsert_budget_entry(doc: dict | None, entry: dict) -> dict:
    """budget 读-改-写保序:只增改自己 id 条目,他人条目 0-byte 序不动
    (g35_render_wiring 同律)。"""
    if doc is None:
        doc = {
            "schema_version": 1,
            "namespace": "g35",
            "description": "G35 预算面(G35-4 OIT 双臂容差条目由本波标定真跑程序产)。",
            "source_docs": ["milestones/g35/g35_sort_oit_gate_evidence_schema.json"],
            "entries": [],
            "ratio_assertions": [],
            "counter_assertions": [],
        }
    entries = list(doc.get("entries") or [])
    for i, e in enumerate(entries):
        if e.get("id") == entry["id"]:
            entries[i] = entry
            break
    else:
        entries.append(entry)
    doc["entries"] = entries
    return doc


def digest_matrix_ok(d_default: dict, d_off: dict, d_sorted: dict, d_wboit: dict) -> bool:
    """⑤ 三臂判别:--oit off == 缺省位级(加性 0 破坏)∧ off/sorted/wboit
    两两互异 ∧ 同轨迹面。"""
    dd = d_default.get("render_digest")
    do = d_off.get("render_digest")
    ds = d_sorted.get("render_digest")
    dw = d_wboit.get("render_digest")
    if not (_digest(dd) and _digest(do) and _digest(ds) and _digest(dw)):
        return False
    if not (dd == do and do != ds and do != dw and ds != dw):
        return False
    trajs = {d.get("trajectory") for d in (d_default, d_off, d_sorted, d_wboit)}
    modes = [
        (d_default.get("oit") or {}).get("mode"),
        (d_off.get("oit") or {}).get("mode"),
        (d_sorted.get("oit") or {}).get("mode"),
        (d_wboit.get("oit") or {}).get("mode"),
    ]
    return len(trajs) == 1 and modes == ["off", "off", "sorted", "wboit"]


def arm_bitexact_ok(doc_a: dict, doc_b: dict) -> bool:
    """②③⑦ 双跑位级判:render_digest + digest_seq_sha 双通道一致。"""
    a, b = doc_a.get("render_digest"), doc_b.get("render_digest")
    sa, sb = doc_a.get("digest_seq_sha"), doc_b.get("digest_seq_sha")
    return _digest(a) and a == b and _digest(sa) and sa == sb


def sorted_witness_ok(w: dict | None, tol: float | None) -> bool:
    """④ 近远见证判:sorted 正臂 p100 ≤ 冻结容差 ∧ 合成改动像素 ≥ 1。"""
    if not isinstance(w, dict) or tol is None:
        return False
    p = w.get("p100_vs_host")
    return (
        w.get("arm") == "sorted"
        and w.get("red_arm") is False
        and _num(p)
        and math.isfinite(p)
        and 0.0 <= p <= tol
        and isinstance(w.get("changed_px"), int)
        and w["changed_px"] >= 1
    )


def wboit_witness_ok(w: dict | None, tol: float | None) -> bool:
    """③ WBOIT 见证判:acc 整数差 ≤ 冻结容差 ∧ 饱和计数(device/host)
    登记 ≥ 0。"""
    if not isinstance(w, dict) or tol is None:
        return False
    d = w.get("acc_max_int_diff")
    return (
        w.get("arm") == "wboit"
        and isinstance(d, int)
        and 0 <= d <= tol
        and isinstance(w.get("sat_device"), int)
        and w["sat_device"] >= 0
        and isinstance(w.get("sat_host"), int)
        and w["sat_host"] >= 0
    )


def red_arm_ok(w_red: dict | None, doc_red: dict, doc_ok: dict, tol: float | None) -> bool:
    """⑧ 红臂检出判:键反转翻序 ⇒ p100_red > max(tol, RED_FLOOR) ∧ 红臂
    digest ≠ 正臂 digest。"""
    if not isinstance(w_red, dict) or tol is None:
        return False
    p = w_red.get("p100_red") if "p100_red" in w_red else w_red.get("p100_vs_host")
    dr, dk = doc_red.get("render_digest"), doc_ok.get("render_digest")
    return (
        w_red.get("arm") == "sorted"
        and w_red.get("red_arm") is True
        and _num(p)
        and math.isfinite(p)
        and p > max(tol, RED_FLOOR)
        and _digest(dr)
        and _digest(dk)
        and dr != dk
    )


def retrigger_ok(win: dict | None) -> bool:
    """⑥ 评估窗 re-trigger 机核判:retriggers 含 consumer = 本门记录 +
    decision 引 RFC-0049 与 M120(选型提交引 benchmark 数据纪律)+ 既有
    字段(schema/frozen_at_utc/trigger_evaluation/conclusion)在档。"""
    if not isinstance(win, dict):
        return False
    if win.get("schema") != "rurix.g31.oit_evaluation_window.v1":
        return False
    for key in ("frozen_at_utc", "trigger_evaluation", "conclusion", "oit_harness_inventory"):
        if key not in win:
            return False
    rts = win.get("retriggers")
    if not isinstance(rts, list):
        return False
    for r in rts:
        if not isinstance(r, dict) or r.get("consumer") != GATE_KEY:
            continue
        dec = str(r.get("decision") or "")
        if isinstance(r.get("date"), str) and r["date"] and "RFC-0049" in dec and "M120" in dec:
            return True
    return False


def frame_ms_ok(fm: dict | None) -> bool:
    """⑨ 单臂 frame_ms 健全判(off 臂 oit_gpu_mean_ms 恒 0 合法)。"""
    if not isinstance(fm, dict):
        return False
    r = fm.get("real_render_frame_ms")
    o = fm.get("oit_gpu_mean_ms")
    return _num(r) and r > 0 and _num(o) and o >= 0


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def sha256_of(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


ALL_KERNELS = LANE_KERNELS + PARTICLE_KERNELS + RENDER_KERNELS + SORT_KERNELS + OIT_KERNELS


def spv_args() -> list[str]:
    """全 24 件 SPV 路径旗标(粒子/渲染件亦指本波 WORK 现编产物——独立
    编译域,不依赖 G35-3 门残留)。"""
    w = lambda name: str(WORK / f"{name}.spv")
    return [
        "--spv-scene", w("g14_3_direct_gi"),
        "--spv-mv", w("g14_mv"),
        "--spv-resample", w("g14_8_tsr_resample"),
        "--spv-resolve", w("g14_8_tsr_resolve"),
        "--spv-encode", w("g31_display_encode"),
        "--spv-p-sim", w("g35_sim"),
        "--spv-p-scan-seg-sum", w("g35_scan_seg_sum"),
        "--spv-p-scan-spine", w("g35_scan_spine"),
        "--spv-p-scan-seg-apply", w("g35_scan_seg_apply"),
        "--spv-p-compact", w("g35_particle_compact"),
        "--spv-p-emit", w("g35_emit"),
        "--spv-p-indirect-args", w("g35_indirect_args"),
        "--spv-splat-clear", w("g35_splat_clear"),
        "--spv-splat", w("g35_render_splat"),
        "--spv-presolve", w("g35_render_resolve"),
        "--spv-oit-sort-hist", w("g35_sort_hist"),
        "--spv-oit-sort-spine", w("g35_sort_spine"),
        "--spv-oit-sort-scatter", w("g35_sort_scatter"),
        "--spv-oit-hash-clear", w("g35_hash_clear"),
        "--spv-oit-tilekey", w("g35_oit_tilekey"),
        "--spv-oit-tilerange", w("g35_oit_tilerange"),
        "--spv-oit-blend", w("g35_oit_blend_sorted"),
        "--spv-oit-accum", w("g35_oit_wboit_accum"),
        "--spv-oit-wresolve", w("g35_oit_wboit_resolve"),
    ]


def run_lane(label: str, extra: list[str], cap: int, seed: int, env: dict) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = WORK / f"lane_{label}.json"
    argv = [str(BIN), *spv_args(), "--cap", str(cap), "--seed", str(seed),
            "--tier", str(OIT_TIER), "--evidence", str(ev_path), *extra]
    r = run(argv, timeout=3600, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def run_gate(frames: int, cap: int, seed: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not GATE_SCHEMA_PATH.is_file():
        fail(f"gate schema 缺失: {GATE_SCHEMA_PATH}")
        return 1
    if not WINDOW_PATH.is_file():
        fail(f"OIT 评估窗登记件缺失: {WINDOW_PATH}")
        return 1

    # ── 构建(车道 bin〔vendor-upscale〕 + rurixc)──
    ok = build_or_fail(
        ["cargo", "build", "-p", "rurix-render", "--features", "vendor-upscale",
         "--bin", "g35_particle_lane", "--quiet"],
        "g35_particle_lane bin",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── ① kernel SPV 面:现编 24 件 + spirv-val + 冻结消费面 sha256 快照 ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for name in ALL_KERNELS:
        src = KERNEL_DIR / f"{name}.rx"
        dst = WORK / f"{name}.spv"
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(dst)], timeout=1800)
        if r.returncode != 0 or not dst.is_file():
            spv_ok = False
            note(f"rurixc 编译失败 {src.name}: {(r.stdout + r.stderr)[-200:]}")
            continue
        val = run(["spirv-val", str(dst)], timeout=600)
        if val.returncode != 0:
            spv_ok = False
            note(f"spirv-val 未过 {dst.name}: {(val.stdout + val.stderr)[-200:]}")
    frozen_snapshot: dict[str, str] = {}
    snapshot_ok = True
    for p in FROZEN_CONSUMED_PATHS:
        fp = ROOT / p
        if fp.is_file():
            frozen_snapshot[p] = sha256_of(fp)
        else:
            snapshot_ok = False
            frozen_snapshot[p] = "MISSING"
    set_fact(
        "kernels_spv_valid",
        spv_ok and snapshot_ok,
        f"rurixc 现编 24 kernel(车道五 + 粒子七 + 渲染三 + sort 三 + OIT 六"
        f"〔hash_clear W7 0-byte 消费 + 本波五件〕)+ spirv-val={'绿' if spv_ok else '红'};"
        f"冻结消费面 sha256 快照在档={snapshot_ok}(G35-4 消费不修改纪律漂移守护基线)",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("G35-4 kernel SPV 编译/spirv-val 未过")

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    gate_path = ROOT / "evidence" / f"g35_sort_oit_gate_{ts}.json"
    gate_rel = str(gate_path.relative_to(ROOT)).replace("\\", "/")
    doc_default: dict | None = None
    doc_off: dict | None = None
    doc_sorted_a: dict | None = None
    doc_sorted_b: dict | None = None
    doc_wboit_a: dict | None = None
    doc_wboit_b: dict | None = None
    doc_wit_sorted: dict | None = None
    doc_wit_red: dict | None = None
    doc_wit_wboit: dict | None = None
    run_evidence: list[str] = []

    def leg(label: str, extra: list[str], env: dict) -> dict | None:
        """一腿真跑 + 三态检出(skipped_dev_env → degrade 登记返 None)。"""
        rc, doc, ev = run_lane(label, extra, cap, seed, env)
        out = (rc.stdout or "") + (rc.stderr or "")
        if '"skipped_dev_env"' in out:
            degrade.append(f"lane skipped_dev_env({label}): {out.strip()[-200:]}")
            return None
        if rc.returncode != 0 or doc is None:
            fail(f"{label} 腿真跑失败 rc={rc.returncode}: {out[-300:]}")
            return None
        if "Validation Error" in out or "VUID-" in out:
            fail(f"{label} 腿 validation 应静默却报错")
        run_evidence.append(str(ev.relative_to(ROOT)).replace("\\", "/"))
        return doc

    orbit = ["--particles", "on", "--auto-move", "orbit",
             "--frames", str(frames), "--warmup", "6", "--headless"]
    witness = ["--particles", "on", "--oit-witness",
               "--frames", "40", "--warmup", "4", "--headless"]
    if not degrade:
        env = device_env()
        with gpu_device_lock(purpose=f"{TAG} 三臂 digest 判别 + 双臂双跑 + 近远见证/红臂 device 真跑"):
            # ── ⑤ 缺省面(不带 --oit)+ --oit off 显式档(加性 0 破坏)──
            doc_default = leg("default", orbit, env)
            if not degrade:
                doc_off = leg("oit_off", [*orbit, "--oit", "off"], env)
            # ── ②⑤⑦ sorted 臂双跑 ──
            if not degrade:
                doc_sorted_a = leg("sorted_a", [*orbit, "--oit", "sorted"], env)
            if not degrade:
                doc_sorted_b = leg("sorted_b", [*orbit, "--oit", "sorted"], env)
            # ── ③⑤⑦ wboit 臂双跑 ──
            if not degrade:
                doc_wboit_a = leg("wboit_a", [*orbit, "--oit", "wboit"], env)
            if not degrade:
                doc_wboit_b = leg("wboit_b", [*orbit, "--oit", "wboit"], env)
            # ── ④ 近远见证腿(sorted;静态相机 OitPair 夹具)──
            if not degrade:
                doc_wit_sorted = leg("wit_sorted", [*witness, "--oit", "sorted"], env)
            # ── ⑧ 红臂腿(key-invert 键反转篡改)──
            if not degrade:
                doc_wit_red = leg(
                    "wit_red", [*witness, "--oit", "sorted", "--red-arm", "key-invert"], env
                )
            # ── ③ WBOIT 见证腿(acc 整数差对拍)──
            if not degrade:
                doc_wit_wboit = leg("wit_wboit", [*witness, "--oit", "wboit"], env)

    if degrade:
        doc = {
            "schema": "rurix.g35.sort_oit.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for dg in degrade:
            note(f"DEV_ENV_DEGRADE {dg}")
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0

    # ── 标定/程序读(双条目;measured = 见证腿实测)──
    ws = (doc_wit_sorted or {}).get("oit_witness") if isinstance((doc_wit_sorted or {}).get("oit_witness"), dict) else None
    ww = (doc_wit_wboit or {}).get("oit_witness") if isinstance((doc_wit_wboit or {}).get("oit_witness"), dict) else None
    wr = (doc_wit_red or {}).get("oit_witness") if isinstance((doc_wit_red or {}).get("oit_witness"), dict) else None
    budget = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
    tol_sorted = frozen_tol(budget, TOL_SORTED_ID)
    tol_wboit = frozen_tol(budget, TOL_WBOIT_ID)
    calibrated_sorted = False
    calibrated_wboit = False
    pending_entries: list[dict] = []
    if tol_sorted is None and ws is not None and _num(ws.get("p100_vs_host")):
        measured = float(ws["p100_vs_host"])
        tol_sorted = calib_threshold(measured)
        calibrated_sorted = True
        pending_entries.append({
            "id": TOL_SORTED_ID,
            "description": (
                "G35-4 排序臂近远见证容差冻结带(--oit-witness --oit sorted 两粒子视轴"
                "夹具:sorted 臂末帧全帧 scene_color vs host 金标准期望〔进程内 --oit off "
                "影面基底 + particles/oit_arms.rs far→near painter's 串行合成〕p100 max "
                "abs diff;tilekey/blend SPV 注入 NoContraction 后标定;threshold = "
                "measured × 2.0 协议冻结 k,measured = 0 时 threshold = 0 零容差零条目,"
                "方向 max;标定真跑 = ci/g35_sort_oit_smoke.py --gate g35.wave4.sort_oit "
                "近远见证腿;evidence_file = 门裁决件 results.trimmed_mean 镜像槽,"
                "budget_eval 通用路消费;标定程序可复跑)"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "f32_absdiff",
            "threshold": tol_sorted,
            "evidence_file": gate_rel,
            "measured_value": measured,
        })
        note(f"sorted 标定:measured={measured:e} → threshold={tol_sorted:e}(×2.0 程序产)")
    elif tol_sorted is not None:
        note(f"冻结容差程序读:{TOL_SORTED_ID} threshold={tol_sorted:e}(在档跳过标定)")
    if tol_wboit is None and ww is not None and isinstance(ww.get("acc_max_int_diff"), int):
        measured_w = float(ww["acc_max_int_diff"])
        tol_wboit = calib_threshold(measured_w)
        calibrated_wboit = True
        pending_entries.append({
            "id": TOL_WBOIT_ID,
            "description": (
                "G35-4 WBOIT 臂定点累加器整数差容差冻结带(--oit-witness --oit wboit 两"
                "粒子夹具:device acc〔Q12 u32×4px,g35_oit_wboit_accum 原子和〕vs host "
                "定点累加金标准〔particles/oit_arms.rs wboit_frame〕最大整数差;accum SPV "
                "注入 NoContraction 后标定;threshold = measured × 2.0 协议冻结 k,"
                "measured = 0 时 threshold = 0 零容差零条目,方向 max;标定真跑 = "
                "ci/g35_sort_oit_smoke.py --gate g35.wave4.sort_oit WBOIT 见证腿;"
                "evidence_file = 门裁决件〔与 g35.oit.parity_p100 共享,results."
                "trimmed_mean 槽为 sorted p100 镜像——本条目实测承载 = 门裁决件 "
                "wboit_witness.acc_max_int_diff,双条目零容差预期面登记;budget_eval "
                "通用路以 trimmed_mean 判读时双零等价,非零漂移以门 facts "
                "wboit_fixedpoint_saturation 直判为准〕;标定程序可复跑)"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "u32_absdiff",
            "threshold": tol_wboit,
            "evidence_file": gate_rel,
            "measured_value": measured_w,
        })
        note(f"wboit 标定:measured={measured_w:e} → threshold={tol_wboit:e}(×2.0 程序产)")
    elif tol_wboit is not None:
        note(f"冻结容差程序读:{TOL_WBOIT_ID} threshold={tol_wboit:e}(在档跳过标定)")

    # ── ②~⑨ facts 判读 ──
    dd = doc_default or {}
    do = doc_off or {}
    sa = doc_sorted_a or {}
    sb = doc_sorted_b or {}
    wa = doc_wboit_a or {}
    wb = doc_wboit_b or {}
    dws = doc_wit_sorted or {}
    dwr = doc_wit_red or {}
    win_doc = json.loads(WINDOW_PATH.read_text(encoding="utf-8"))
    set_fact(
        "sorted_arm_bitexact",
        arm_bitexact_ok(sa, sb),
        f"sorted 臂同参数双跑位级:digest 等={sa.get('render_digest') == sb.get('render_digest')} "
        f"digest_seq_sha 等={sa.get('digest_seq_sha') == sb.get('digest_seq_sha')}"
        f"(复合键 radix 稳定序 + 每像素区间升序固定序串行合成 = 位级确定基准臂)",
    )
    set_fact(
        "wboit_fixedpoint_saturation",
        arm_bitexact_ok(wa, wb) and wboit_witness_ok(ww, tol_wboit),
        f"wboit 臂双跑位级={arm_bitexact_ok(wa, wb)}(u32 Q12 fetch_add 可交换 + delta 加前 "
        f"clamp 65535 + cap ≤ 65536 结构性防回绕);acc_max_int_diff={(ww or {}).get('acc_max_int_diff')!r} "
        f"≤ 冻结容差 {tol_wboit!r}({TOL_WBOIT_ID} {'本次标定' if calibrated_wboit else '程序读'});"
        f"饱和事件登记 device={(ww or {}).get('sat_device')!r} host={(ww or {}).get('sat_host')!r}",
    )
    set_fact(
        "near_far_order_witness",
        sorted_witness_ok(ws, tol_sorted),
        f"近远见证(帧 0/30 两粒子视轴夹具,先发者更远 age 大偏红):sorted 臂 p100_vs_host="
        f"{(ws or {}).get('p100_vs_host')!r} ≤ 冻结容差 {tol_sorted!r}({TOL_SORTED_ID} "
        f"{'本次标定' if calibrated_sorted else '程序读'})∧ changed_px={(ws or {}).get('changed_px')!r} ≥ 1"
        f"(host 期望 = far→near painter's 串行链,oit_arms 单测证远者先画)",
    )
    set_fact(
        "oit_arms_digest_discrimination",
        digest_matrix_ok(dd, do, sa, wa),
        f"--oit off == 缺省位级={dd.get('render_digest') == do.get('render_digest')}(加性 0 破坏"
        f"机器证明)∧ off/sorted/wboit 两两互异:off={str(do.get('render_digest'))[:23]}… "
        f"sorted={str(sa.get('render_digest'))[:23]}… wboit={str(wa.get('render_digest'))[:23]}…",
    )
    set_fact(
        "oit_retrigger_registered",
        retrigger_ok(win_doc),
        f"评估窗 {WINDOW_PATH.name} retriggers 含 consumer={GATE_KEY} 记录(decision 引 "
        f"RFC-0049 §4.8 + M120 冻结测量 = #13 conditional_wiring_sketch ① 选型提交引 "
        f"benchmark 数据纪律)∧ 既有字段(schema/frozen_at_utc/trigger_evaluation/conclusion)在档",
    )
    set_fact(
        "determinism_double_run",
        arm_bitexact_ok(sa, sb) and arm_bitexact_ok(wa, wb),
        f"双臂四跑全链位级汇总:sorted={arm_bitexact_ok(sa, sb)} wboit={arm_bitexact_ok(wa, wb)}"
        f"(digest + digest_seq_sha 双通道;臂 A 固定序串行/臂 B 整数可交换,与调度无关)",
    )
    set_fact(
        "red_arm_effective",
        red_arm_ok(wr, dwr, dws, tol_sorted),
        f"红臂 key-invert(tilekey 去 4095− 反键翻转):p100_red={(wr or {}).get('p100_vs_host')!r} "
        f"> max(冻结容差 {tol_sorted!r}, 名义底 {RED_FLOOR}) ∧ 红臂 digest ≠ 正臂 digest"
        f"(近远翻序必检出)",
    )
    fm_off = do.get("frame_ms") if isinstance(do.get("frame_ms"), dict) else None
    fm_s = sa.get("frame_ms") if isinstance(sa.get("frame_ms"), dict) else None
    fm_w = wa.get("frame_ms") if isinstance(wa.get("frame_ms"), dict) else None
    set_fact(
        "frame_ms_measured",
        frame_ms_ok(fm_off) and frame_ms_ok(fm_s) and frame_ms_ok(fm_w),
        f"三臂逐帧墙钟 off={(fm_off or {}).get('real_render_frame_ms')!r} "
        f"sorted={(fm_s or {}).get('real_render_frame_ms')!r} "
        f"wboit={(fm_w or {}).get('real_render_frame_ms')!r} ms + OIT pass 组 GPU 段和 "
        f"sorted={(fm_s or {}).get('oit_gpu_mean_ms')!r} wboit={(fm_w or {}).get('oit_gpu_mean_ms')!r} ms"
        f"(measured_local 诚实登记,非帧率对标门)",
    )

    # ── evidence 落盘(门裁决件;jsonschema 自校验硬门)──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti(本机单卡 measured_local)",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    zero_d = "sha256:" + "0" * 64
    spv_entry = lambda name: {
        "path": str((WORK / f"{name}.spv").relative_to(ROOT)).replace("\\", "/"),
        "sha256": sha256_of(WORK / f"{name}.spv") if (WORK / f"{name}.spv").is_file() else zero_d,
    }
    p100_sorted = float(ws["p100_vs_host"]) if ws is not None and _num(ws.get("p100_vs_host")) else -1.0
    acc_diff = int(ww["acc_max_int_diff"]) if ww is not None and isinstance(ww.get("acc_max_int_diff"), int) else -1
    rt_row = next(
        (r for r in (win_doc.get("retriggers") or [])
         if isinstance(r, dict) and r.get("consumer") == GATE_KEY),
        {},
    )
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "kernels": {
            "lane": {name: spv_entry(name) for name in LANE_KERNELS},
            "particle": {name: spv_entry(name) for name in PARTICLE_KERNELS},
            "render": {name: spv_entry(name) for name in RENDER_KERNELS},
            "sort": {name: spv_entry(name) for name in SORT_KERNELS},
            "oit": {name: spv_entry(name) for name in OIT_KERNELS},
            "spirv_val_all": bool(facts["kernels_spv_valid"]["status"] == "PASS"),
            "frozen_consumed_snapshot": frozen_snapshot,
        },
        "digest_matrix": {
            "trajectory": "orbit",
            "default_digest": dd.get("render_digest", zero_d),
            "off_digest": do.get("render_digest", zero_d),
            "sorted_digest": sa.get("render_digest", zero_d),
            "wboit_digest": wa.get("render_digest", zero_d),
            "off_equals_default": bool(
                _digest(dd.get("render_digest"))
                and dd.get("render_digest") == do.get("render_digest")
            ),
            "pairwise_distinct": bool(digest_matrix_ok(dd, do, sa, wa)),
        },
        "determinism": {
            "sorted_bitexact": bool(arm_bitexact_ok(sa, sb)),
            "wboit_bitexact": bool(arm_bitexact_ok(wa, wb)),
            "sorted_digest_a": sa.get("render_digest", zero_d),
            "sorted_digest_b": sb.get("render_digest", zero_d),
            "sorted_seq_sha_a": sa.get("digest_seq_sha") or zero_d,
            "sorted_seq_sha_b": sb.get("digest_seq_sha") or zero_d,
            "wboit_digest_a": wa.get("render_digest", zero_d),
            "wboit_digest_b": wb.get("render_digest", zero_d),
            "wboit_seq_sha_a": wa.get("digest_seq_sha") or zero_d,
            "wboit_seq_sha_b": wb.get("digest_seq_sha") or zero_d,
        },
        "sorted_witness": {
            "p100_vs_host": p100_sorted,
            "changed_px": int((ws or {}).get("changed_px") or 0),
            "threshold": tol_sorted if tol_sorted is not None else -1.0,
            "budget_entry": TOL_SORTED_ID,
            "calibrated_this_run": calibrated_sorted,
            "within": bool(sorted_witness_ok(ws, tol_sorted)),
        },
        "wboit_witness": {
            "acc_max_int_diff": acc_diff,
            "sat_device": int((ww or {}).get("sat_device") if isinstance((ww or {}).get("sat_device"), int) else -1),
            "sat_host": int((ww or {}).get("sat_host") if isinstance((ww or {}).get("sat_host"), int) else -1),
            "p100_vs_host": float((ww or {}).get("p100_vs_host")) if _num((ww or {}).get("p100_vs_host")) else -1.0,
            "threshold": tol_wboit if tol_wboit is not None else -1.0,
            "budget_entry": TOL_WBOIT_ID,
            "calibrated_this_run": calibrated_wboit,
            "within": bool(wboit_witness_ok(ww, tol_wboit)),
        },
        "red_arm": {
            "arm": "key-invert",
            "p100_red": float((wr or {}).get("p100_vs_host")) if _num((wr or {}).get("p100_vs_host")) else -1.0,
            "digest_red": dwr.get("render_digest", zero_d),
            "digest_ok": dws.get("render_digest", zero_d),
            "red_floor": RED_FLOOR,
            "effective": bool(red_arm_ok(wr, dwr, dws, tol_sorted)),
        },
        "retrigger": {
            "path": str(WINDOW_PATH.relative_to(ROOT)).replace("\\", "/"),
            "found": bool(retrigger_ok(win_doc)),
            "consumer": GATE_KEY,
            "date": str(rt_row.get("date") or ""),
            "decision": str(rt_row.get("decision") or ""),
        },
        "results": {"trimmed_mean": p100_sorted},
        "frame_ms": {
            "off_frame_ms": (fm_off or {}).get("real_render_frame_ms") if frame_ms_ok(fm_off) else 1e-9,
            "sorted_frame_ms": (fm_s or {}).get("real_render_frame_ms") if frame_ms_ok(fm_s) else 1e-9,
            "wboit_frame_ms": (fm_w or {}).get("real_render_frame_ms") if frame_ms_ok(fm_w) else 1e-9,
            "sorted_oit_gpu_ms": (fm_s or {}).get("oit_gpu_mean_ms") if frame_ms_ok(fm_s) else 0.0,
            "wboit_oit_gpu_ms": (fm_w or {}).get("oit_gpu_mean_ms") if frame_ms_ok(fm_w) else 0.0,
            "frames_measured": int((fm_s or {}).get("frames_measured") or 0),
            "measured": "measured_local",
            "note": (
                "三臂逐帧墙钟均值(prepare+execute+回读)+ OIT pass 组 GPU timestamp 段和"
                "(sorted = hash_clear+tilekey+sort 9 dispatch+tilerange+blend;wboit = "
                "accum+resolve,acc 清零段归 splat_clear 名下不重计);登记语义非帧率对标,"
                "OIT 腿构型 = --tier 50(键域守卫,内部 960×540)"
            ),
        },
        "run_evidence": run_evidence or ["(run evidence 缺失)"],
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G35-4 半透明双臂(RFC-0049 §4.8 评审修订后冻结协议;host 金标准 = particles/"
            "oit_arms.rs 四函数与 kernel 逐字同源):臂 A sorted = 复合键 tile_id·4096 + "
            "(4095−depth12)(tile = 16px 网格,中心像素单键归属 v1 冻结,rpx ≤ 3 < 16 跨 tile "
            "截断带如实登记;屏外/被剔 = tile_cnt·4096 溢出 tile;键域论证 tile_cnt ≤ 4095 ⇒ "
            "溢出键 ≤ 16773120 < 2^24)→ W1 sort 三 kernel 3-pass 9 dispatch(payload = slot,"
            "键/payload A→B→A→B)→ g35_oit_tilerange(按 key/4096 tile 段分组——g35_hash_"
            "cellrange 完整键分组不可直用故自写同形;哨兵 = g35_hash_clear 0-byte 消费)→ "
            "g35_oit_blend_sorted 每像素区间升序 far→near painter's 固定序串行 C = C·(1−α) + "
            "c·α(投影/调色/软深度三式 = g35_render_splat/g35_render_resolve 逐字同式;循环体 "
            "gate 代数化 = SPIR-V 结构化控制流承载)= 位级确定基准。臂 B wboit = g35_oit_wboit_"
            "accum u32 Q12 定点原子累加(SCALE = 4096 floor 舍入;饱和 = 加前 clamp delta ≤ "
            "65535 + 事件累计计数;cap ≤ 65536 ⇒ 累加和 ≤ 2^32−2^16 < u32::MAX 结构性防回绕 = "
            "clamp 到 u32::MAX 语义不可达顶证明;w(z) = aw²·dist_w/3000 参照 oit/algorithms.rs "
            "run_weighted nvpro 权重式冻结)→ g35_oit_wboit_resolve(sum_w = acc_w/4096;c = "
            "(acc/4096)/max(sum_w,1e-5);α_out = min(1,sum_w) 替代 reveal 连乘诚实登记)⇒ 整数"
            "可交换双跑位级。车道 = bin/g35_particle_lane.rs 加性扩展:--oit off|sorted|wboit "
            "三档闭集(off = G35-3 现面零追加,digest 位级 == 缺省结构性保证);sorted 档 "
            "presolve(11)后插 13 pass(28 pass)/wboit 档插 3 pass(18 pass),资源 54..=70 "
            "追加,FrameUpdate 重映射 TSR/encode 下标 +Δ;屏障计划全 StorageWrite 形态同律"
            "(全 RW 同态去重零屏障竞争教训)。#13 OIT 评估窗 re-trigger 消费登记于 "
            "milestones/g31/g31_oit_evaluation_window.json retriggers(只追加,既有字段 "
            "0-byte)。results.trimmed_mean = sorted 见证 p100 镜像(budget_eval 通用路)。"
        ),
    }
    import jsonschema  # 自校验硬门(schema 漂移即 RED)

    errs = list(jsonschema.Draft7Validator(
        json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        fail("gate evidence schema 自校验红: " + "; ".join(
            f"{'/'.join(str(p) for p in e.path)}: {e.message}" for e in errs[:3]))
        all_pass = False
        gate_doc["verdict"] = "FAIL"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_rel}(lane 真跑件 {len(run_evidence)} 份留 .tmp 工作区)")

    # ── budget 程序写(标定腿产;gate 裁决件已落盘 ⇒ evidence_file 不悬空)──
    if pending_entries:
        budget_doc = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
        for entry in pending_entries:
            budget_doc = upsert_budget_entry(budget_doc, entry)
        BUDGET_PATH.parent.mkdir(parents=True, exist_ok=True)
        BUDGET_PATH.write_text(json.dumps(budget_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        back = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
        for entry in pending_entries:
            if frozen_tol(back, entry["id"]) != entry["threshold"]:
                fail(f"budget 回读互核失败({entry['id']}:写入后 frozen_tol ≠ 待写 threshold)")
                all_pass = False
            else:
                note(f"g35_budget.json 程序写入 {entry['id']}(threshold={entry['threshold']:e};重读核验绿)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest(判读器红绿穷举 + schema 校验 + FACT_IDS 互核;零 GPU 零构建)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    d0 = "sha256:" + "a" * 64
    d1 = "sha256:" + "b" * 64
    d2 = "sha256:" + "c" * 64
    d3 = "sha256:" + "d" * 64
    s0 = "sha256:" + "e" * 64
    # 红绿臂①:冻结容差程序读(双条目 id)+ ×2.0 协议。
    good_budget = {"entries": [
        {"id": TOL_SORTED_ID, "evidence": "measured_local", "skip_reason": None, "threshold": 0.5},
        {"id": TOL_WBOIT_ID, "evidence": "measured_local", "skip_reason": None, "threshold": 2.0},
    ]}
    expect(frozen_tol(good_budget, TOL_SORTED_ID) == 0.5, "GREEN:sorted 容差程序读")
    expect(frozen_tol(good_budget, TOL_WBOIT_ID) == 2.0, "GREEN:wboit 容差程序读")
    expect(calib_threshold(0.25) == 0.5, "GREEN:×2.0 冻结 k")
    expect(calib_threshold(0.0) == 0.0, "GREEN:measured = 0 ⇒ threshold = 0(零容差零条目)")
    expect(frozen_tol({"entries": [{"id": TOL_SORTED_ID, "evidence": "estimated",
                                    "skip_reason": None, "threshold": 1.0}]}, TOL_SORTED_ID) is None,
           "RED:estimated 冒充 measured 必拒")
    expect(frozen_tol({"entries": [{"id": TOL_WBOIT_ID, "evidence": "measured_local",
                                    "skip_reason": "no gpu", "threshold": 1.0}]}, TOL_WBOIT_ID) is None,
           "RED:skip_reason 携带必拒")
    expect(frozen_tol({"entries": []}, TOL_SORTED_ID) is None, "RED:条目缺失必拒")
    expect(frozen_tol(None, TOL_SORTED_ID) is None, "RED:budget 文件缺失必拒")
    expect(frozen_tol({"entries": [{"id": TOL_WBOIT_ID, "evidence": "measured_local",
                                    "skip_reason": None, "threshold": True}]}, TOL_WBOIT_ID) is None,
           "RED:bool 冒充数值阈必拒")
    # 红绿臂②:budget 读-改-写保序。
    foreign = {"id": "g35.render.mv_parity_px", "threshold": 0.0}
    mine = {"id": TOL_SORTED_ID, "evidence": "measured_local", "skip_reason": None, "threshold": 0.5}
    up = upsert_budget_entry({"namespace": "g35", "entries": [foreign]}, dict(mine))
    expect(up["entries"][0] == foreign and up["entries"][1]["id"] == TOL_SORTED_ID,
           "GREEN:upsert 追加保序(他人条目 0-byte 序不动)")
    up2 = upsert_budget_entry(up, {**mine, "threshold": 1.0})
    expect(len(up2["entries"]) == 2 and up2["entries"][1]["threshold"] == 1.0
           and up2["entries"][0] == foreign,
           "GREEN:upsert 原位替换自己条目(幂等面)")
    # 红绿臂③:三臂 digest 判别。
    mk = lambda dg, mode, traj="orbit": {"render_digest": dg, "trajectory": traj,
                                         "oit": {"mode": mode}}
    expect(digest_matrix_ok(mk(d0, "off"), mk(d0, "off"), mk(d1, "sorted"), mk(d2, "wboit")),
           "GREEN:三臂判别正例(off == 缺省 + 两两互异)")
    expect(not digest_matrix_ok(mk(d0, "off"), mk(d3, "off"), mk(d1, "sorted"), mk(d2, "wboit")),
           "RED:--oit off ≠ 缺省(加性破坏)必红")
    expect(not digest_matrix_ok(mk(d0, "off"), mk(d0, "off"), mk(d0, "sorted"), mk(d2, "wboit")),
           "RED:sorted == off(镂空 pass 冒充)必红")
    expect(not digest_matrix_ok(mk(d0, "off"), mk(d0, "off"), mk(d1, "sorted"), mk(d1, "wboit")),
           "RED:sorted == wboit(双臂未判别)必红")
    expect(not digest_matrix_ok(mk(d0, "off"), mk(d0, "off"), mk(d1, "sorted", "dolly"), mk(d2, "wboit")),
           "RED:轨迹不同面必红")
    expect(not digest_matrix_ok(mk(d0, "off"), mk(d0, "off"), mk(d1, "off"), mk(d2, "wboit")),
           "RED:oit.mode 档漂移必红")
    # 红绿臂④:双跑位级判。
    ga = {"render_digest": d0, "digest_seq_sha": s0}
    expect(arm_bitexact_ok(ga, dict(ga)), "GREEN:双跑位级正例")
    expect(not arm_bitexact_ok(ga, {**ga, "render_digest": d1}), "RED:末帧 digest 异必红")
    expect(not arm_bitexact_ok(ga, {**ga, "digest_seq_sha": d1}),
           "RED:digest_seq_sha 异(逐帧链敏感)必红")
    expect(not arm_bitexact_ok({"render_digest": "xx", "digest_seq_sha": s0},
                               {"render_digest": "xx", "digest_seq_sha": s0}),
           "RED:digest 形态破必红")
    # 红绿臂⑤:sorted 近远见证判。
    good_ws = {"arm": "sorted", "red_arm": False, "p100_vs_host": 0.0, "changed_px": 12}
    expect(sorted_witness_ok(good_ws, 0.0), "GREEN:近远见证正例(measured=0 vs threshold=0 边界)")
    expect(sorted_witness_ok({**good_ws, "p100_vs_host": 0.25}, 0.5), "GREEN:带内正例")
    expect(not sorted_witness_ok({**good_ws, "p100_vs_host": 0.6}, 0.5), "RED:超冻结容差必红")
    expect(not sorted_witness_ok({**good_ws, "changed_px": 0}, 0.5),
           "RED:零合成像素(粒子未上屏)必红")
    expect(not sorted_witness_ok({**good_ws, "red_arm": True}, 0.5), "RED:红臂冒充正臂必红")
    expect(not sorted_witness_ok({**good_ws, "arm": "wboit"}, 0.5), "RED:臂名漂移必红")
    expect(not sorted_witness_ok({**good_ws, "p100_vs_host": float("nan")}, 0.5), "RED:NaN 必红")
    expect(not sorted_witness_ok(good_ws, None), "RED:容差缺失(未标定)必红")
    expect(not sorted_witness_ok(None, 0.5), "RED:见证块缺失必红")
    # 红绿臂⑥:wboit 见证判。
    good_ww = {"arm": "wboit", "acc_max_int_diff": 0, "sat_device": 0, "sat_host": 0}
    expect(wboit_witness_ok(good_ww, 0.0), "GREEN:wboit 见证正例(整数差 0 vs threshold 0)")
    expect(wboit_witness_ok({**good_ww, "acc_max_int_diff": 1}, 2.0), "GREEN:带内正例")
    expect(not wboit_witness_ok({**good_ww, "acc_max_int_diff": 3}, 2.0), "RED:整数差超容差必红")
    expect(not wboit_witness_ok({**good_ww, "acc_max_int_diff": -1}, 2.0),
           "RED:见证缺失哨兵(-1)必红")
    expect(not wboit_witness_ok({**good_ww, "sat_device": -1}, 2.0), "RED:饱和计数缺登记必红")
    expect(not wboit_witness_ok({**good_ww, "acc_max_int_diff": 0.5}, 2.0),
           "RED:非整数 acc diff(形态破)必红")
    expect(not wboit_witness_ok(good_ww, None), "RED:容差缺失必红")
    expect(not wboit_witness_ok(None, 2.0), "RED:见证块缺失必红")
    # 红绿臂⑦:红臂检出判。
    good_wr = {"arm": "sorted", "red_arm": True, "p100_vs_host": 0.08}
    dr = {"render_digest": d1}
    dk = {"render_digest": d0}
    expect(red_arm_ok(good_wr, dr, dk, 1e-6), "GREEN:红臂检出正例(p100 ≫ 容差与名义底)")
    expect(not red_arm_ok({**good_wr, "p100_vs_host": 5e-4}, dr, dk, 1e-6),
           "RED:红臂 p100 未破 1e-3 名义底(翻序未检出)必红")
    expect(not red_arm_ok({**good_wr, "p100_vs_host": 0.05}, dr, dk, 0.06),
           "RED:红臂 p100 未破冻结容差必红")
    expect(not red_arm_ok(good_wr, {"render_digest": d0}, dk, 1e-6),
           "RED:红臂 digest == 正臂(篡改无效)必红")
    expect(not red_arm_ok({**good_wr, "red_arm": False}, dr, dk, 1e-6),
           "RED:red_arm 旗标假(正臂冒充)必红")
    expect(not red_arm_ok(good_wr, dr, dk, None), "RED:容差缺失必红")
    expect(not red_arm_ok(None, dr, dk, 1e-6), "RED:见证块缺失必红")
    # 红绿臂⑧:评估窗 re-trigger 机核判。
    good_win = {
        "schema": "rurix.g31.oit_evaluation_window.v1",
        "frozen_at_utc": "2026-08-26T06:16:26Z",
        "trigger_evaluation": {"verdict": "not_triggered"},
        "conclusion": "…",
        "oit_harness_inventory": {},
        "retriggers": [{
            "date": "2026-08-27",
            "consumer": GATE_KEY,
            "decision": "WBOIT 定点 + 排序双臂,依 RFC-0049 §4.8 + M120 冻结测量",
        }],
    }
    expect(retrigger_ok(good_win), "GREEN:re-trigger 记录正例")
    expect(not retrigger_ok({**good_win, "retriggers": []}), "RED:记录缺失必红")
    expect(not retrigger_ok({**good_win, "retriggers": [
        {**good_win["retriggers"][0], "consumer": "g35.wave5.collision"}]}),
        "RED:consumer 漂移必红")
    expect(not retrigger_ok({**good_win, "retriggers": [
        {**good_win["retriggers"][0], "decision": "拍脑袋选型"}]}),
        "RED:decision 未引 RFC-0049/M120(无数据提交)必红")
    win_broken = dict(good_win)
    del win_broken["frozen_at_utc"]
    expect(not retrigger_ok(win_broken), "RED:既有字段缺失(0-byte 破)必红")
    expect(not retrigger_ok({**good_win, "schema": "rurix.g31.other.v1"}), "RED:schema 漂移必红")
    expect(not retrigger_ok(None), "RED:登记件缺失必红")
    # 红绿臂⑨:frame_ms 健全判。
    expect(frame_ms_ok({"real_render_frame_ms": 12.5, "oit_gpu_mean_ms": 0.0}),
           "GREEN:frame_ms 正例(off 臂 oit 段 0 合法)")
    expect(frame_ms_ok({"real_render_frame_ms": 12.5, "oit_gpu_mean_ms": 3.1}), "GREEN:OIT 臂正例")
    expect(not frame_ms_ok({"real_render_frame_ms": 0.0, "oit_gpu_mean_ms": 1.0}), "RED:0ms 必红")
    expect(not frame_ms_ok({"real_render_frame_ms": float("nan"), "oit_gpu_mean_ms": 1.0}),
           "RED:NaN 必红")
    expect(not frame_ms_ok({"real_render_frame_ms": 12.5}), "RED:oit 段缺失必红")
    expect(not frame_ms_ok(None), "RED:缺失必红")
    # schema 互核。
    expect(GATE_SCHEMA_PATH.is_file(), "gate schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(enum == FACT_IDS, f"gate schema facts enum == FACT_IDS({len(FACT_IDS)},序同)")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate schema 门键 const 互核")
        expect(gs["properties"]["sorted_witness"]["properties"]["budget_entry"]["const"] == TOL_SORTED_ID,
               "gate schema sorted budget_entry const 互核")
        expect(gs["properties"]["wboit_witness"]["properties"]["budget_entry"]["const"] == TOL_WBOIT_ID,
               "gate schema wboit budget_entry const 互核")
        expect(gs["properties"]["red_arm"]["properties"]["red_floor"]["const"] == RED_FLOOR,
               "gate schema red_floor const 互核")
        expect(gs["properties"]["retrigger"]["properties"]["consumer"]["const"] == GATE_KEY,
               "gate schema retrigger consumer const 互核")
        expect("results" in gs.get("required", [])
               and gs["properties"]["results"]["properties"]["trimmed_mean"]["type"] == "number",
               "gate schema results.trimmed_mean 通用消费面互核(budget_eval evidence_file 路)")
        oit_req = gs["properties"]["kernels"]["properties"]["oit"]["required"]
        expect(sorted(oit_req) == sorted(OIT_KERNELS), "gate schema OIT kernel 六件闭集互核")
        import jsonschema as _js
        _js.Draft7Validator.check_schema(gs)
        print("  ok   — gate schema Draft7 合法(check_schema 绿)")
    expect(len(FACT_IDS) == 9, "facts 闭集 = 9")
    expect(WINDOW_PATH.is_file(), "OIT 评估窗登记件在树")
    if WINDOW_PATH.is_file():
        win = json.loads(WINDOW_PATH.read_text(encoding="utf-8"))
        expect(retrigger_ok(win), "评估窗 re-trigger 记录在档(树上实件机核)")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts=9;9 红绿臂组 + budget 读改写保序 + schema/评估窗互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=48)
    ap.add_argument("--cap", type=int, default=65536)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        if args.frames < 16:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 16(TSR 历史收敛 + 粒子换血最小窗)",
                  file=sys.stderr)
            return 1
        if args.cap <= 0 or args.cap % 256 != 0 or args.cap > 65536:
            print(f"[{TAG}] FAIL: --cap {args.cap} 须为 SEG=256 正整倍数且 ≤ 65536"
                  f"(wboit 定点累加结构性防回绕域)", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.cap, args.seed)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
