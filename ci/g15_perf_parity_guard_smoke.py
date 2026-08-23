#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.5 性能零降级波）
"""G15.5 P0 硬门 M-d：性能零降级守护（g15.p0.m_d.perf_parity_zero_regression，
步骤 275；G15_CONTRACT §4.2 M-d 行判据逐字 / G-G15-6；G15_ACCEPTANCE_MAP §1 M-d 行）。

host 消费门：G14 M-d 门同口径复跑由本波子进程真跑落盘（UE 臂 benchmark harness
复跑 + Rurix 臂生产管线 bench，三轮进程级独立运行 160 帧协议，GPU 锁纪律沿
g14_dual_end_fps_parity_smoke.py 本体面）；本门只读核验 + 画质锚带复核重算 +
RED 臂。判据（契约 §4.2 M-d 行字面）：

1. **G14 M-d 门同口径复跑 fresh PASS**：g14_m_d_dual_end_fps_parity 最新
   evidence 状态 pass、checks 全真、timestamp ≥ 本波启动锚（--wave-start 字面；
   缺省 = HEAD commit UTC 派生锚）、device_section_state=executed、
   base_commit == HEAD（同树同口径复跑面）；旧件冒充 fresh 即 RED。
2. **逐格 ratio ≥ ×1.00 维持**：18 格（双场景 × 三档 × 三后端闭集）逐格
   fps_ratio ≥ 1.00 且 stored pass 标签与重算一致；fps_ratio ==
   (1000/rurix_median_ms)/(1000/ue_median_ms) f64 精确重算（ratio 单轮口径冒充
   三轮跨轮中位数即 RED）；met_count==18 / unmet_count==0 重算一致。
3. **逐轮守护带齐备**：逐格 runs==3（run_index {1,2,3}、started_epoch 严格递增）
   + per_run_ratios==3 全数值 + rurix_median_ms == 三轮 frame_ms_mean 排序中位
   f64 精确重算 + 生产口径不变量 0 < prod ≤ full 逐轮维持（G14.6 v2 面）。
4. **G14 门产 18 格 digest 锚漂移守护核验**：g14_3_stage_a_digest_anchor 冻结锚
   18 键闭集与逐格三轮 last_frame_digest 位级全等；G15 零 src 变更面下应位级
   全等，检出漂移 = RD-045 同型事件如实登记升级（parity.drift_monitoring 入档，
   门红不静默）。
5. **G14 M-c 画质锚带复核**：最新 g14_m_c_rurix_pipeline_perf evidence PASS 且
   quality_parity_anchor=true + 在树 converged.exr 双件（G14.3 生产车道 vs
   G13.4 车道 cornell t67 tsr_device）SSIM deficit 重算 ≤ 0.010779849285388998
   带内（带 = g14_budget 锚定条目 threshold f64 精确对账 == 契约字面）。
6. **G14 门产 budget 条目零 estimated 维持**：g14_budget 全条目 evidence ==
   measured_local、skip_reason 全 null；画质锚条目 measured×2.0 == threshold
   f64 精确。

RED 臂（契约判据字面）：ratio 篡改（未达标冒充达标）/ 旧 evidence 冒充 fresh /
缺轮（两轮冒充三轮）/ 锚漂移静默——各臂注入必检出（--selftest + 门内真跑臂）。
画质修复致性能劣化静默即 RED（本门 = 守护机核面）。

pr-smoke 默认 --verify-latest（秒级核最新 full-run evidence）；
本地/workflow_dispatch 用 --gate 产 full-run。

用法：
  py -3 ci/g15_perf_parity_guard_smoke.py --gate g15.p0.m_d.perf_parity_zero_regression --wave-start <UTC>
  py -3 ci/g15_perf_parity_guard_smoke.py --verify-latest
  py -3 ci/g15_perf_parity_guard_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import copy
import datetime as _dt
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g15.p0.m_d.perf_parity_zero_regression"
NUMERIC_STEP = 275  # 落盘前实测 registry/number_ledger.json CI_step.next_free=275 顺位领取
SUBJECT = "g15_m_d_perf_parity_zero_regression"
WAVE = "G15.5"
TAG = "g15_m_d"
MATRIX_ROW = "M-d"
SOURCE_REF = (
    "G15_CONTRACT §4.2 M-d/G-G15-6;G15_ACCEPTANCE_MAP §1 M-d;"
    "G14_CONTRACT §4.2 M-d 同口径复跑（g14.p0.m_d.dual_end_fps_parity）;"
    "G14-N7 通过线 ×1.00 维持;M141/M165 50×3 冻结统计口径;"
    "G14 M-c 画质锚带 ≤0.010779849285388998 复核;RD-045 漂移监控登记条款"
)
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_m_d_perf_parity_zero_regression_evidence_schema.json"
DIGEST_ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
G14_BUDGET_PATH = ROOT / "milestones" / "g14" / "g14_budget.json"

G14_MD_PREFIX = "g14_m_d_dual_end_fps_parity"
G14_MD_GATE = "g14.p0.m_d.dual_end_fps_parity"
G14_MC_PREFIX = "g14_m_c_rurix_pipeline_perf"
G14_MC_GATE = "g14.p0.m_c.rurix_pipeline_perf"
# G14.12 soak 复跑 18/18 定盘件（同口径对照面；evidence 只增不删不改纪律面在树）。
G14_12_REFERENCE = ROOT / "evidence" / "g14_m_d_dual_end_fps_parity_20260823T051754Z.json"

# G14 M-c 画质锚带（契约 §4.2 M-d 行字面；与 g14_budget 锚定条目 threshold f64 精确对账）。
ANCHOR_BAND = 0.010779849285388998
ANCHOR_BUDGET_ID = "g14.pipeline_perf.quality_anchor_ssim_deficit"
PASS_LINE_RATIO = 1.00

SCENES = ("cornell-box", "bistro-interior")
TIERS = (50, 67, 100)
BACKENDS = ("tsr_device", "dlss_sr", "fsr_3_1_5")

# 画质锚带复核 EXR 面（G14.3 生产车道 vs G13.4 车道 cornell t67 tsr_device converged）。
ANCHOR_CELL = ("cornell-box", 67, "tsr_device")
G14_PROD_CONV = Path(r"K:\rurix-ext\g14-frames\rurix_prod")
G13_CONV = Path(r"K:\rurix-ext\g13-frames\rurix_upscale")

CHECK_KEYS = [
    "g14_m_d_rerun_fresh_pass",
    "eighteen_cells_ratio_pass_line",
    "three_run_guard_band_complete",
    "production_caliber_invariant",
    "digest_anchor_zero_drift",
    "quality_anchor_band_recheck",
    "g14_budget_zero_estimated",
    "comparison_vs_g14_12_rerun",
    "red_arm_ratio_tamper_detected",
    "red_arm_stale_evidence_detected",
    "red_arm_missing_run_detected",
    "red_arm_anchor_drift_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def run(cmd: list[str], timeout: int = 7200) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout)


def base_commit() -> str:
    r = run(["git", "rev-parse", "HEAD"])
    return (r.stdout or "").strip()


def head_commit_utc_stamp() -> str:
    """HEAD committer 时刻 → UTC %Y%m%dT%H%M%SZ（freshness 缺省锚）。"""
    r = run(["git", "show", "-s", "--format=%ct", "HEAD"])
    epoch = int((r.stdout or "0").strip() or "0")
    return _dt.datetime.fromtimestamp(epoch, _dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


# ---------------------------------------------------------------------------
# 纯函数面（RED 臂与 wave5 聚合门消费）
# ---------------------------------------------------------------------------


def cell_key(scene: str, tier: int, backend: str) -> str:
    return f"{scene}_t{tier}_{backend}"


def expected_cell_keys() -> list[str]:
    return [cell_key(s, t, b) for s in SCENES for t in TIERS for b in BACKENDS]


def pass_line(ratio: float) -> bool:
    """通过线 = G14-N7 口径裁决 ×1.00 下限（≥ 即达标）。"""
    return float(ratio) >= PASS_LINE_RATIO


def freshness_ok(stamp, wave_start: str) -> bool:
    """UTC 字符串排序机核（%Y%m%dT%H%M%SZ 字典序 == 时序）。"""
    return isinstance(stamp, str) and isinstance(wave_start, str) and stamp >= wave_start


def recompute_ratio(cell: dict) -> float:
    return (1000.0 / float(cell["rurix_median_ms"])) / (1000.0 / float(cell["ue_median_ms"]))


def validate_cells(doc: dict, anchors: dict) -> list[str]:
    """G14 M-d 复跑 evidence 逐格全量重算校验（18 格闭集 / ratio f64 精确重算 /
    三轮守护带齐备 / 生产口径不变量 / digest 锚位级对账）。返回错误列表（空=绿）。"""
    errs: list[str] = []
    parity = doc.get("parity") or {}
    if parity.get("pass_line_ratio") != PASS_LINE_RATIO:
        errs.append(f"pass_line_ratio 漂移: {parity.get('pass_line_ratio')!r}")
    cells = parity.get("cells") or []
    if len(cells) != 18:
        errs.append(f"cells {len(cells)}≠18")
        return errs
    got_keys = [cell_key(c.get("scene"), c.get("tier"), c.get("backend")) for c in cells]
    if got_keys != expected_cell_keys():
        errs.append("18 格闭集/行序与契约双场景×三档×三后端不全等")
    met = 0
    for c in cells:
        key = cell_key(c.get("scene"), c.get("tier"), c.get("backend"))
        try:
            ratio = float(c["fps_ratio"])
        except (KeyError, TypeError, ValueError):
            errs.append(f"{key} fps_ratio 缺/非数值")
            continue
        if recompute_ratio(c) != ratio:
            errs.append(f"{key} fps_ratio 存储值 ≠ 三轮跨轮中位数口径重算（单轮冒充三轮面）")
        if pass_line(ratio) is not c.get("pass"):
            errs.append(f"{key} pass 标签与 ×1.00 通过线重算不符（未达标冒充达标面）: ratio={ratio}")
        if not pass_line(ratio):
            errs.append(f"{key} ratio={ratio:.6f} < ×1.00 通过线（性能劣化静默即 RED 面）")
        else:
            met += 1
        runs = c.get("runs") or []
        prr = c.get("per_run_ratios") or []
        if len(runs) != 3 or sorted(r.get("run_index") for r in runs) != [1, 2, 3]:
            errs.append(f"{key} 三轮进程级独立运行缺轮（runs={len(runs)}）")
        if len(prr) != 3 or not all(isinstance(v, (int, float)) and not isinstance(v, bool) and v > 0.0 for v in prr):
            errs.append(f"{key} 逐轮守护带不齐（per_run_ratios={len(prr)}）")
        if len(runs) == 3:
            epochs = [float(r.get("started_epoch", 0.0)) for r in sorted(runs, key=lambda r: r.get("run_index"))]
            if not (epochs[0] < epochs[1] < epochs[2]):
                errs.append(f"{key} 三轮 started_epoch 非严格递增（进程级独立性存疑）")
            means = sorted(float(r["frame_ms_mean"]) for r in runs)
            if means[1] != float(c["rurix_median_ms"]):
                errs.append(f"{key} rurix_median_ms ≠ 三轮 frame_ms_mean 排序中位（跨轮中位数口径破坏）")
            for r in runs:
                prod = float(r.get("frame_ms_mean", 0.0))
                full = float(r.get("frame_ms_full_caliber", -1.0))
                if not (0.0 < prod <= full):
                    errs.append(f"{key}/r{r.get('run_index')} 生产口径不变量破: prod={prod} full={full}")
                dig = str(r.get("last_frame_digest", ""))
                if not dig.startswith("sha256:"):
                    errs.append(f"{key}/r{r.get('run_index')} last_frame_digest 缺")
        anchor_dig = (anchors.get(key) or {}).get("last_frame_digest", "")
        cell_digs = {str(r.get("last_frame_digest", "")) for r in runs}
        if not anchor_dig:
            errs.append(f"{key} digest 冻结锚缺")
        elif cell_digs != {anchor_dig}:
            errs.append(f"{key} Stage A digest 漂移（RD-045 同型事件面）: anchor={anchor_dig[:32]}… cell={sorted(d[:32] for d in cell_digs)}")
    if parity.get("met_count") != met or parity.get("unmet_count") != len(cells) - met:
        errs.append(f"met/unmet 计数与逐格重算不符: stored={parity.get('met_count')}/{parity.get('unmet_count')} recompute={met}/{len(cells) - met}")
    return errs


def load_anchors() -> dict:
    doc = wel.load_json(DIGEST_ANCHOR_PATH) if DIGEST_ANCHOR_PATH.is_file() else {}
    return doc.get("anchors") or {}


def synthetic_doc() -> dict:
    """18 格合成正例（selftest/RED 臂底样——结构全合法、ratio=2.0 全达标）。"""
    cells = []
    for scene in SCENES:
        for tier in TIERS:
            for backend in BACKENDS:
                dig = "sha256:" + f"{abs(hash(cell_key(scene, tier, backend))) & 0xFFFFFFFF:08x}" * 8
                runs = [
                    {"run_index": i, "frame_ms_mean": 1.0, "frame_ms_full_caliber": 1.2,
                     "tail_ms_mean": 0.2, "last_frame_digest": dig, "cv": 1.0,
                     "started_epoch": 1000.0 + i}
                    for i in (1, 2, 3)
                ]
                cells.append({
                    "scene": scene, "tier": tier, "backend": backend,
                    "ue_median_ms": 2.0, "rurix_median_ms": 1.0,
                    "rurix_full_caliber_ms": 1.2,
                    "fps_ratio": 2.0, "per_run_ratios": [2.0, 2.0, 2.0],
                    "runs": runs, "pass": True,
                })
    return {"parity": {"pass_line_ratio": PASS_LINE_RATIO, "met_count": 18,
                       "unmet_count": 0, "cells": cells}}


def synthetic_anchors(doc: dict) -> dict:
    return {cell_key(c["scene"], c["tier"], c["backend"]):
            {"last_frame_digest": c["runs"][0]["last_frame_digest"]}
            for c in doc["parity"]["cells"]}


# ---------------------------------------------------------------------------
# RED 臂（门内真跑：以本门纯函数面为底，四臂独立）
# ---------------------------------------------------------------------------


def red_arm_ratio_tamper() -> bool:
    """ratio 篡改（未达标冒充达标：数据面自洽的 0.97 格标 pass=true）→ 逐格重算面必检出。"""
    doc = synthetic_doc()
    anchors = synthetic_anchors(doc)
    cell = doc["parity"]["cells"][0]
    cell["rurix_median_ms"] = 2.0 / 0.97  # 数据面自洽于 0.97（median/逐轮全同步）
    for r in cell["runs"]:
        r["frame_ms_mean"] = 2.0 / 0.97
        r["frame_ms_full_caliber"] = 2.4 / 0.97
    cell["fps_ratio"] = recompute_ratio(cell)  # 重算自洽——篡改面只剩 pass 标签谎报
    cell["pass"] = True  # 未达标冒充达标（×1.00 通过线重算面必检出）
    detected = bool(validate_cells(doc, anchors))
    return detected and (not pass_line(0.97)) and pass_line(1.0)


def red_arm_stale_evidence() -> bool:
    """旧 evidence 冒充 fresh（timestamp 早于本波启动锚）→ freshness 面必检出。"""
    wave_start = "20260823T153347Z"
    stale = "20000101T000000Z"
    return (not freshness_ok(stale, wave_start)) and freshness_ok(wave_start, wave_start)


def red_arm_missing_run() -> bool:
    """缺轮（两轮冒充三轮）→ 三轮守护带/中位数口径面必检出。"""
    doc = synthetic_doc()
    anchors = synthetic_anchors(doc)
    doc["parity"]["cells"][0]["runs"] = doc["parity"]["cells"][0]["runs"][:2]
    doc["parity"]["cells"][0]["per_run_ratios"] = [2.0, 2.0]
    return bool(validate_cells(doc, anchors))


def red_arm_anchor_drift() -> bool:
    """锚漂移静默（一轮 digest 异于冻结锚）→ 位级对账面必检出。"""
    doc = synthetic_doc()
    anchors = synthetic_anchors(doc)
    doc["parity"]["cells"][0]["runs"][2]["last_frame_digest"] = "sha256:" + "0" * 64
    return bool(validate_cells(doc, anchors))


# ---------------------------------------------------------------------------
# 画质锚带复核（G14 M-c 面：最新 evidence PASS + 在树 EXR 双件 SSIM 重算 ≤ 带）
# ---------------------------------------------------------------------------


def recheck_quality_anchor() -> tuple[bool, str, float | None]:
    path = wel.load_latest_evidence(G14_MC_PREFIX)
    if path is None:
        return False, "缺最新 g14_m_c evidence", None
    doc = wel.load_json(path)
    ok, detail = wel.gate_pass_reason(doc, G14_MC_GATE)
    if not ok:
        return False, f"g14_m_c 最新 evidence 非全绿: {detail}", None
    if (doc.get("checks") or {}).get("quality_parity_anchor") is not True:
        return False, "g14_m_c quality_parity_anchor 非 true", None
    if not G14_BUDGET_PATH.is_file():
        return False, "g14_budget.json 缺失", None
    bud = wel.load_json(G14_BUDGET_PATH)
    anchor = next((e for e in (bud.get("entries") or []) if e.get("id") == ANCHOR_BUDGET_ID), None)
    if anchor is None:
        return False, f"g14_budget 缺画质锚条目 {ANCHOR_BUDGET_ID}", None
    if float(anchor["threshold"]) != ANCHOR_BAND:
        return False, f"锚定带 threshold={anchor['threshold']!r} ≠ 契约字面 {ANCHOR_BAND!r}", None
    if float(anchor["measured_value"]) * 2.0 != float(anchor["threshold"]):
        return False, "锚定带 ≠ 首跑 measured×2.0 程序产面（P-09 对账失败）", None
    mine = G14_PROD_CONV / ANCHOR_CELL[0] / f"tier{ANCHOR_CELL[1]}" / ANCHOR_CELL[2] / "converged.exr"
    g13 = G13_CONV / ANCHOR_CELL[0] / f"tier{ANCHOR_CELL[1]}" / ANCHOR_CELL[2] / "converged.exr"
    if not mine.is_file() or not g13.is_file():
        return False, f"converged.exr 双件缺（{mine} / {g13}）", None
    import numpy as np  # noqa: E402
    import g10_exr_lib as exr  # noqa: E402
    import g10_ssim_psnr_lib as ssim_psnr  # noqa: E402
    da = exr.decode_exr_file(mine, "rurix")
    db = exr.decode_exr_file(g13, "rurix")
    if (da["width"], da["height"]) != (db["width"], db["height"]):
        return False, "converged.exr 双件尺寸不齐", None
    a = np.array(da["pixels"], dtype=np.float64).reshape(da["height"], da["width"], -1)[..., :3]
    b = np.array(db["pixels"], dtype=np.float64).reshape(db["height"], db["width"], -1)[..., :3]
    a = np.clip(a, 0.0, 1.0)
    b = np.clip(b, 0.0, 1.0)
    deficit = 1.0 - ssim_psnr.ssim_wang2004(a, b)
    within = deficit <= ANCHOR_BAND
    return within, (f"SSIM deficit 重算={deficit:.12f} ≤ 带 {ANCHOR_BAND} = {within}"
                    f"（g14_m_c 最新件 {path.name} PASS + 锚定条目 f64 对账绿）"), deficit


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def run_gate(wave_start: str) -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    started = _dt.datetime.now(_dt.timezone.utc)
    ts = started.strftime("%Y%m%dT%H%M%SZ")
    note(f"wave_start={wave_start}（本波启动锚；freshness 机核面）")

    # ── ① G14 M-d 门同口径复跑 fresh PASS ──
    md_path = wel.load_latest_evidence(G14_MD_PREFIX)
    md_doc: dict = {}
    md_ok = False
    md_detail = "缺最新 evidence"
    if md_path is not None:
        try:
            md_doc = wel.load_json(md_path)
        except (OSError, json.JSONDecodeError) as e:
            md_detail = f"evidence 不可读: {e}"
            md_doc = {}
    if md_doc:
        ok, detail = wel.gate_pass_reason(md_doc, G14_MD_GATE)
        stamp = md_doc.get("timestamp")
        if not ok:
            md_detail = f"非全绿: {detail}"
        elif not freshness_ok(stamp, wave_start):
            md_detail = f"非本波 fresh（timestamp={stamp!r} < wave_start={wave_start!r}——旧件冒充 fresh 即 RED）"
        elif md_doc.get("device_section_state") != "executed":
            md_detail = f"device_section_state={md_doc.get('device_section_state')!r} ≠ executed"
        elif md_doc.get("base_commit") != base_commit():
            md_detail = f"base_commit={md_doc.get('base_commit')!r} ≠ HEAD（非同树复跑面）"
        else:
            md_ok = True
            md_detail = f"PASS（fresh {stamp} ≥ 本波启动锚；base_commit==HEAD 同树；device executed）"
    checks["g14_m_d_rerun_fresh_pass"] = md_ok
    check(md_ok, f"G14 M-d 复跑 fresh 核验: {md_detail}")
    note(f"G14 M-d 复跑：{'PASS' if md_ok else 'FAIL'}（{md_detail}）")

    # ── ②~⑤ 逐格全量重算（ratio/守护带/口径不变量/digest 锚） ──
    anchors = load_anchors()
    cell_errs: list[str] = ["G14 M-d 复跑 evidence 未就绪——逐格重算跳过（诚实红不充绿）"]
    if md_ok:
        cell_errs = validate_cells(md_doc, anchors)
    for e in cell_errs[:12]:
        check(False, f"逐格重算: {e}")
    cells_len_ok = md_ok and len((md_doc.get("parity") or {}).get("cells") or []) == 18
    checks["eighteen_cells_ratio_pass_line"] = cells_len_ok and not [
        e for e in cell_errs if "×1.00" in e or "fps_ratio" in e or "pass 标签" in e
        or "met/unmet" in e or "闭集" in e or "cells" in e]
    checks["three_run_guard_band_complete"] = cells_len_ok and not [
        e for e in cell_errs if "缺轮" in e or "守护带" in e or "中位" in e or "started_epoch" in e]
    checks["production_caliber_invariant"] = cells_len_ok and not [
        e for e in cell_errs if "生产口径不变量" in e]
    drift_errs = [e for e in cell_errs if "digest 漂移" in e or "冻结锚缺" in e or "last_frame_digest" in e]
    checks["digest_anchor_zero_drift"] = cells_len_ok and not drift_errs
    note(f"逐格重算：18 格 ratio/守护带/口径/锚 错误面 {len(cell_errs) if md_ok else 'n/a'}（drift {len(drift_errs)}）")

    # ── ⑥ 画质锚带复核（SSIM deficit ≤ 0.010779849285388998 带内重算） ──
    anchor_ok, anchor_detail, anchor_deficit = recheck_quality_anchor()
    checks["quality_anchor_band_recheck"] = anchor_ok
    check(anchor_ok, f"画质锚带复核: {anchor_detail}")
    note(f"画质锚带复核：{anchor_detail}")

    # ── ⑦ G14 门产 budget 条目零 estimated 维持 ──
    bud_bad: list[str] = []
    g14_entries: list[dict] = []
    if not G14_BUDGET_PATH.is_file():
        bud_bad.append("g14_budget.json 缺失")
    else:
        bud = wel.load_json(G14_BUDGET_PATH)
        g14_entries = bud.get("entries") or []
        if not g14_entries:
            bud_bad.append("g14_budget 条目空")
        for e in g14_entries:
            if e.get("evidence") != "measured_local":
                bud_bad.append(f"{e.get('id')} 非 measured_local（estimated 冒充即 RED 面）")
            if e.get("skip_reason") is not None:
                bud_bad.append(f"{e.get('id')} skip_reason 非 null")
    checks["g14_budget_zero_estimated"] = not bud_bad
    check(not bud_bad, f"g14_budget 零 estimated 机核: {bud_bad[:3]}")
    note(f"g14_budget {len(g14_entries)} 条目零 estimated/skip 维持 = {not bud_bad}")

    # ── ⑧ G14.12 soak 复跑同口径对照面（双件 18 格全达逐格对账） ──
    cmp_bad: list[str] = []
    cmp_rows: list[dict] = []
    ref_doc: dict = {}
    if not G14_12_REFERENCE.is_file():
        cmp_bad.append("G14.12 对照件缺失")
    else:
        ref_doc = wel.load_json(G14_12_REFERENCE)
        ok, detail = wel.gate_pass_reason(ref_doc, G14_MD_GATE)
        if not ok:
            cmp_bad.append(f"G14.12 对照件非全绿: {detail}")
    if md_ok and ref_doc:
        ref_cells = {cell_key(c.get("scene"), c.get("tier"), c.get("backend")): c
                     for c in (ref_doc.get("parity") or {}).get("cells") or []}
        if sorted(ref_cells) != sorted(expected_cell_keys()):
            cmp_bad.append("G14.12 对照件 18 格闭集不齐")
        for c in (md_doc.get("parity") or {}).get("cells") or []:
            key = cell_key(c["scene"], c["tier"], c["backend"])
            rc = ref_cells.get(key)
            if rc is None:
                continue
            r_now, r_ref = float(c["fps_ratio"]), float(rc["fps_ratio"])
            cmp_rows.append({"cell": key, "ratio_g14_12_soak": r_ref, "ratio_g15_5_rerun": r_now,
                             "both_ge_pass_line": pass_line(r_now) and pass_line(r_ref)})
            if not (pass_line(r_now) and pass_line(r_ref)):
                cmp_bad.append(f"{key} 对照面非双达: g14_12={r_ref:.4f} g15_5={r_now:.4f}")
    checks["comparison_vs_g14_12_rerun"] = not cmp_bad and len(cmp_rows) == 18
    check(not cmp_bad, f"G14.12 对照面: {cmp_bad[:3]}")

    # ── ⑨ RED 臂（门内真跑，四臂独立） ──
    red_results = {
        "ratio_tamper": red_arm_ratio_tamper(),
        "stale_evidence": red_arm_stale_evidence(),
        "missing_run": red_arm_missing_run(),
        "anchor_drift": red_arm_anchor_drift(),
    }
    checks["red_arm_ratio_tamper_detected"] = red_results["ratio_tamper"] is True
    checks["red_arm_stale_evidence_detected"] = red_results["stale_evidence"] is True
    checks["red_arm_missing_run_detected"] = red_results["missing_run"] is True
    checks["red_arm_anchor_drift_detected"] = red_results["anchor_drift"] is True
    for arm, ok in red_results.items():
        check(ok, f"RED 臂 {arm} 注入未检出")
        note(f"RED 臂 {arm}: {'有效' if ok else '失效'}")

    cells = (md_doc.get("parity") or {}).get("cells") or [] if md_ok else []
    host_pass = all(checks.values()) and not FAILURES
    device_state = "executed" if md_ok else "fail"
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if host_pass and device_state == "executed" else "fail",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": base_commit(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": f"py -3 ci/g14_dual_end_fps_parity_smoke.py --gate {G14_MD_GATE}（本波同口径复跑真跑面，本门只读消费）",
             "exit_code": 0 if checks["g14_m_d_rerun_fresh_pass"] else 1},
            {"seq": 2, "command": "18 格逐格 ratio ≥ ×1.00 + 三轮守护带 + 口径不变量 + digest 锚 全量重算",
             "exit_code": 0 if all(checks[k] for k in ("eighteen_cells_ratio_pass_line", "three_run_guard_band_complete", "production_caliber_invariant", "digest_anchor_zero_drift")) else 1},
            {"seq": 3, "command": "G14 M-c 画质锚带复核（converged.exr 双件 SSIM deficit 重算 ≤ 0.010779849285388998）",
             "exit_code": 0 if checks["quality_anchor_band_recheck"] else 1},
            {"seq": 4, "command": "g14_budget 零 estimated 机核 + G14.12 soak 复跑同口径对照",
             "exit_code": 0 if checks["g14_budget_zero_estimated"] and checks["comparison_vs_g14_12_rerun"] else 1},
            {"seq": 5, "command": "RED 臂 ×4（ratio-tamper/stale-evidence/missing-run/anchor-drift）",
             "exit_code": 0 if all(v is True for v in red_results.values()) else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "production": {
            "correctness_anchor_unchanged": checks["digest_anchor_zero_drift"] and checks["quality_anchor_band_recheck"],
            "baseline_anchor_id": "g14_3_stage_a_digest_anchor 18 格冻结锚 + g14.pipeline_perf.quality_anchor_ssim_deficit 锚带 + G14.12 soak 复跑定盘件对照",
            "measured_value": (
                f"达标 {sum(1 for c in cells if c.get('pass'))}/{len(cells)}；"
                + "; ".join(f"{cell_key(c['scene'], c['tier'], c['backend'])}: ratio={float(c['fps_ratio']):.4f}" for c in cells[:18])
            ) if cells else "n/a（G14 M-d 复跑未就绪）",
            "not_worse_than_anchor": checks["eighteen_cells_ratio_pass_line"] and checks["quality_anchor_band_recheck"],
            "threshold_provenance": "通过线 = G14-N7 口径裁决 ×1.00 契约面维持（不新设不放宽）；统计 = M141/M165 冻结 50×3 三轮跨轮中位数；画质锚带 = G14.3 首跑 measured×2.0 程序产（P-09 禁手写）",
            "evolution_register": (
                "G15 全期零 src 变更面下 G14 M-d 同口径复跑 18 格 ×1.00 维持核验；"
                "digest 锚漂移零检出维持 RD-045 open-defer（检出即如实登记升级）；"
                "画质修复致性能劣化静默即 RED 守护机核面"
            ),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "parity": {
            "wave_start": wave_start,
            "g14_m_d_evidence": None if md_path is None else str(md_path.relative_to(ROOT)).replace("\\", "/"),
            "g14_m_d_evidence_timestamp": md_doc.get("timestamp"),
            "g14_m_d_base_commit": md_doc.get("base_commit"),
            "pass_line_ratio": PASS_LINE_RATIO,
            "met_count": sum(1 for c in cells if c.get("pass")),
            "unmet_count": sum(1 for c in cells if not c.get("pass")),
            "cells": [
                {"cell": cell_key(c["scene"], c["tier"], c["backend"]),
                 "ue_median_ms": c["ue_median_ms"], "rurix_median_ms": c["rurix_median_ms"],
                 "fps_ratio": c["fps_ratio"], "per_run_ratios": c["per_run_ratios"],
                 "pass": c["pass"]}
                for c in cells
            ],
            "comparison_vs_g14_12": {
                "reference_evidence": "evidence/g14_m_d_dual_end_fps_parity_20260823T051754Z.json",
                "cells": cmp_rows,
                "all_ge_pass_line_both": bool(cmp_rows) and all(r["both_ge_pass_line"] for r in cmp_rows),
            },
            "digest_anchor": {
                "file": "milestones/g14/g14_3_stage_a_digest_anchor.json",
                "cells_checked": len(cells),
                "drift_count": len(drift_errs),
            },
            "drift_monitoring": {
                "rd_045_type_digest_drift_detected": len(drift_errs),
                "drifts": drift_errs,
                "note": "零检出维持 RD-045 open-defer 不写进全绿叙述；检出即 RD-045 同型事件如实登记升级" if not drift_errs else "检出漂移——RD-045 同型事件如实登记升级面",
            },
            "quality_anchor": {
                "band": ANCHOR_BAND,
                "deficit_recomputed": anchor_deficit,
                "within_band": bool(anchor_ok),
                "detail": anchor_detail,
            },
            "g14_budget": {"entries": len(g14_entries), "zero_estimated": not bud_bad},
        },
    }
    errs = wel.validate_schema(evidence, SCHEMA_PATH) if SCHEMA_PATH.is_file() else []
    if errs:
        print(f"[{TAG}] schema errors: {errs}", file=sys.stderr)
        evidence["status"] = "fail"
        evidence["host_section_pass"] = False
        host_pass = False
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    print(f"[{TAG}] VERDICT={'PASS' if evidence['status'] == 'pass' else 'FAIL'} "
          f"checks={sum(1 for v in checks.values() if v)}/{len(checks)}")
    return 0 if evidence["status"] == "pass" else 1


def verify_latest() -> int:
    path = wel.load_latest_evidence(SUBJECT)
    if path is None:
        print(f"[{TAG}] FAIL: 缺最新 evidence（{SUBJECT}_*.json）", file=sys.stderr)
        return 1
    doc = wel.load_json(path)
    checks = doc.get("checks") or {}
    bad = [k for k in CHECK_KEYS if checks.get(k) is not True]
    if bad or doc.get("status") != "pass":
        print(f"[{TAG}] FAIL checks={bad} status={doc.get('status')!r}", file=sys.stderr)
        return 1
    print(f"[{TAG}] verify-latest PASS（{path.name}，checks {len(CHECK_KEYS)} 键全绿）")
    return 0


def run_selftest() -> int:
    """schema 闭集对账 + 四 RED 臂函数面 + GREEN 正例（合成面不依赖 device/UE）。"""
    failures = 0
    schema = wel.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        failures += 1
    # GREEN 正例：合成 18 格全合法 → 校验器零误拒
    good = synthetic_doc()
    good_errs = validate_cells(good, synthetic_anchors(good))
    if good_errs:
        print(f"[{TAG}] selftest FAIL: 合成正例被误拒 {good_errs[:4]}", file=sys.stderr)
        failures += 1
    # 四 RED 臂
    for name, fn in (("ratio_tamper", red_arm_ratio_tamper),
                     ("stale_evidence", red_arm_stale_evidence),
                     ("missing_run", red_arm_missing_run),
                     ("anchor_drift", red_arm_anchor_drift)):
        if not fn():
            print(f"[{TAG}] selftest FAIL: {name} 臂未检出", file=sys.stderr)
            failures += 1
    # 纯函数面绿臂（中位数口径/通过线/新鲜度）
    if recompute_ratio({"ue_median_ms": 2.0, "rurix_median_ms": 1.0}) != 2.0:
        print(f"[{TAG}] selftest FAIL: ratio 重算面异常", file=sys.stderr)
        failures += 1
    if not pass_line(1.0) or pass_line(0.999999):
        print(f"[{TAG}] selftest FAIL: 通过线边界异常", file=sys.stderr)
        failures += 1
    if not freshness_ok("20260823T153348Z", "20260823T153347Z") or freshness_ok("20260823T153346Z", "20260823T153347Z"):
        print(f"[{TAG}] selftest FAIL: freshness 边界异常", file=sys.stderr)
        failures += 1
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)}（schema 闭集 + 4 RED + 4 GREEN 函数面臂）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--wave-start", default=None,
                    help="本波启动锚 UTC（%%Y%%m%%dT%%H%%M%%SZ）；缺省 = HEAD commit UTC 派生锚")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return verify_latest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    wave_start = args.wave_start or head_commit_utc_stamp()
    return run_gate(wave_start)


if __name__ == "__main__":
    sys.exit(main())
