#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.4 波）
"""G11.4 光照与 GI 修复波门共享库（milestones/g11/CI_GATES.md §4 M153/M154 消费面）。

单一事实源面（禁第二份手写）：
- G11.4 帧区路径（K:/rurix-ext/g11-frames/g11_4/——G10/G11.2/G11.3 帧库只读
  分区隔离）与复跑报告装载（g11_4_rerun_report.json）；
- 修复闭环收敛判定（RXS-0393 L2 quality_gap 款）与标定程序纪律复用
  g11_3_fix_lib/g11_2_caliber_lib 单一事实源；
- R3/R4 度量面（HDR 亮度中位/p90 双端实测）与 G10.5 基线复现（G11.2 域统一
  换算面：基线原域 a 未施曝光 ⇒ b − a×2^(−EV100) 对齐域换算）；
- 世界缓存计数面/远场探针能量回归/M96 fixture 对拍带校验器（RED 臂共用：
  世界级未落地/屏幕级冒充/单反弹换皮判红面）。
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_2_caliber_lib as cl  # noqa: E402
import g11_3_fix_lib as fl  # noqa: E402
import g10_exr_lib as exr  # noqa: E402

CORPUS = cl.CORPUS
GAP_REGISTRY = cl.GAP_REGISTRY
BUDGET_PATH = cl.BUDGET_PATH
LOCKED_DIGEST = cl.LOCKED_DIGEST
REPORT_PATH = ROOT / "milestones" / "g11" / "g11_4_rerun_report.json"
REPORT_G11_3_PATH = fl.REPORT_PATH
DERIVATION_REPORT = ROOT / "milestones" / "g11" / "g11_4_light_derivation.json"
SCENE_MANIFEST = ROOT / "milestones" / "g10" / "g10_corpus_scene_manifest.json"
LIGHTING_BISTRO = CORPUS / "lighting_bistro_interior.json"
LIGHTING_CORNELL = CORPUS / "lighting_cornell_box.json"
BAND_PATH = ROOT / "milestones" / "g11" / "g11_m154_world_cache_band.json"
FRAMES_G11_4 = Path(r"K:\rurix-ext\g11-frames\g11_4")
FRAMES_G10_5 = fl.FRAMES_G10_5
RUST_RELEASE_BIN = cl.RUST_RELEASE_BIN
SPEC_GI = ROOT / "spec" / "global_illumination.md"
RFC0028 = ROOT / "rfcs" / "0028-g11-gi-quality-closure.md"

SCENES = cl.SCENES

# G11.2 域统一换算基线（M144 门登记面；残余登记 measured_impact 同值）。
ALIGNED_BASELINE_R3 = 2.7314592314362525
ALIGNED_BASELINE_R4 = 4.8486343559026714

evaluate_closure = fl.evaluate_closure
validate_budget_entry = fl.validate_budget_entry
append_budget_entries = fl.append_budget_entries
contract_digest_rust = fl.contract_digest_rust
load_json = fl.load_json
sha256_file = fl.sha256_file
gap_row = fl.gap_row


def calib_evidence_payload(*args, **kwargs) -> dict:
    """G11.4 标定 evidence 闭集（g11_3 共享件 0-byte 复用 + wave 字段改写 G11.4）。"""
    doc = fl.calib_evidence_payload(*args, **kwargs)
    doc["wave"] = "G11.4"
    return doc


def load_report() -> dict:
    return load_json(REPORT_PATH)


def hdr_frame(scene_key: str, end: str, root: Path = FRAMES_G11_4) -> Path:
    if end == "rurix":
        return root / "rurix" / f"{scene_key}.exr"
    return root / "ue" / scene_key / ".0000.exr"


def hdr_lum(scene_key: str, end: str, root: Path = FRAMES_G11_4) -> dict:
    d = exr.decode_exr(hdr_frame(scene_key, end, root).read_bytes(), end)
    arr = np.asarray(d["pixels"], dtype=np.float64).reshape(d["height"], d["width"], 3)
    return cl.lum_stats(arr)


def baseline_reproduction_r3() -> dict:
    """R3 锁定基线复现（G10.5 帧只读重算 f64 + G11.2 域统一换算面）。"""
    a = cl.lum_stats_from_path if hasattr(cl, "lum_stats_from_path") else None
    del a
    d_r = exr.decode_exr((FRAMES_G10_5 / "rurix" / "bistro-interior.exr").read_bytes(), "rurix")
    d_u = exr.decode_exr((FRAMES_G10_5 / "ue" / "bistro-interior" / ".0000.exr").read_bytes(), "ue5")
    arr_r = np.asarray(d_r["pixels"], dtype=np.float64).reshape(d_r["height"], d_r["width"], 3)
    arr_u = np.asarray(d_u["pixels"], dtype=np.float64).reshape(d_u["height"], d_u["width"], 3)
    a_med = cl.lum_stats(arr_r)["median"]
    b_med = cl.lum_stats(arr_u)["median"]
    return {
        "a": a_med,
        "b": b_med,
        "delta_locked": b_med - a_med,
        # 域统一换算（C2 对齐面：Rurix 原域未施曝光 ⇒ a×2^(−EV100=1)=a×0.5）。
        "delta_aligned": b_med - a_med * 0.5,
    }


def baseline_reproduction_r4() -> dict:
    """R4 锁定基线复现（p90 面，同 R3 口径）。"""
    d_r = exr.decode_exr((FRAMES_G10_5 / "rurix" / "bistro-interior.exr").read_bytes(), "rurix")
    d_u = exr.decode_exr((FRAMES_G10_5 / "ue" / "bistro-interior" / ".0000.exr").read_bytes(), "ue5")
    arr_r = np.asarray(d_r["pixels"], dtype=np.float64).reshape(d_r["height"], d_r["width"], 3)
    arr_u = np.asarray(d_u["pixels"], dtype=np.float64).reshape(d_u["height"], d_u["width"], 3)
    a_p90 = cl.lum_stats(arr_r)["p90"]
    b_p90 = cl.lum_stats(arr_u)["p90"]
    return {
        "a": a_p90,
        "b": b_p90,
        "delta_locked": b_p90 - a_p90,
        "delta_aligned": b_p90 - a_p90 * 0.5,
    }


def shrink_calibration(metric_fn, k: float = 1.0) -> dict:
    """收敛幅度阈标定：样本 = 度量双跑噪声（确定性帧同一对两跑逐位一致 ⇒ p100=0）。"""
    a = metric_fn()
    b = metric_fn()
    return {"p100": abs(a - b), "sample_count": 1, "estimator": "p100", "k": k, "value": a}


def lights_block_problems(lights: dict) -> list[str]:
    """R3 灯种子集消费登记校验（RED 臂共用：未表达冒充修复判红面）。"""
    problems: list[str] = []
    if lights.get("enabled") is not True:
        problems.append("lights.enabled ≠ true（--light-seed-set 消费链断裂）")
    if (lights.get("point_lights_consumed") or 0) < 4:
        problems.append(f"point_lights_consumed={lights.get('point_lights_consumed')!r} < 4（点光源未表达冒充修复即 RED）")
    if lights.get("emissive_materials_consumed") != 4:
        problems.append(f"emissive_materials_consumed={lights.get('emissive_materials_consumed')!r} ≠ 4")
    if lights.get("area_lights_declared_absent") is not True:
        problems.append("area_lights 缺类未显式登记（不得以缺类冒充空集）")
    if not str(lights.get("source_digest", "")).startswith("sha256:"):
        problems.append("lights.source_digest 缺失（契约面 provenance 断裂）")
    pls = lights.get("point_lights") or []
    if pls:
        for p in pls:
            for k in ("position", "color_linear_rgb", "intensity_cd", "emit_direction", "area_m2", "derived_from"):
                if k not in p:
                    problems.append(f"point_lights 缺 provenance 字段 {k}")
                    break
    return problems


def world_cache_block_problems(wc: dict) -> list[str]:
    """世界级缓存落地校验（RED 臂共用：世界级未落地/单反弹换皮判红面）。"""
    problems: list[str] = []
    if wc.get("enabled") is not True:
        problems.append("world_cache.enabled ≠ true")
        return problems
    if wc.get("levels") != 4:
        problems.append(f"levels={wc.get('levels')!r} ≠ 4（辐射 LOD 层级漂移）")
    if (wc.get("bounce_iters") or 0) < 2:
        problems.append(f"bounce_iters={wc.get('bounce_iters')!r} < 2（单反弹换皮冒充多反弹即 RED）")
    deposits = wc.get("deposits") or []
    # 沉积总量 > 0 即落地（按距离分级语义：全远场场景集中粗级属正确物理——
    # cornell 全内容超出 d_ref×2² 集中 level 3 实测登记；禁以层级分布冒充）。
    if len(deposits) != 4 or sum(int(d or 0) for d in deposits) <= 0:
        problems.append(f"deposits={deposits!r} 总量非正（世界缓存未落地冒充承接即 RED）")
    queries = wc.get("queries") or []
    hits = wc.get("hits") or []
    if not queries or sum(queries) <= 0:
        problems.append("queries 全零（回落路径无计数即 RED）")
    if not hits or sum(hits) <= 0:
        problems.append("hits 全零（缓存命中为零——世界级未落地）")
    epi = wc.get("energy_per_iter") or []
    if len(epi) < 2:
        problems.append("energy_per_iter 不足两级（每级能量计数缺失即 RED）")
    else:
        totals = [sum(e) for e in epi]
        deltas = [totals[i + 1] - totals[i] for i in range(len(totals) - 1)]
        # 多弹收敛口径（RXS-0395 L3）：增量绝对值递减趋于零（采样噪声下允许
        # 小幅负增量——收敛至均衡下方面）；|Δ| 递增 = 能量发散即 RED。
        if len(deltas) >= 2 and abs(deltas[-1]) > abs(deltas[0]) * (1 + 1e-9):
            problems.append(f"能量增量绝对值非递减（多弹收敛口径违例）: {deltas}")
    if (wc.get("farfield_probe_count") or 0) < 1:
        problems.append("farfield_probe_count = 0（远场探针集未登记即 RED）")
    if (wc.get("farfield_energy_mean") or 0.0) <= 0.0:
        problems.append("farfield_energy_mean ≤ 0（远场能量回归为零冒充世界级即 RED）")
    return problems


def run_fixture() -> dict:
    """M96 fixture 对拍（RXS-0396 L5 锚②机核面）：真跑 --world-cache-fixture。"""
    r = subprocess.run(
        [str(RUST_RELEASE_BIN), "--world-cache-fixture"],
        cwd=ROOT, capture_output=True, text=True, timeout=1800,
    )
    if r.returncode != 0:
        raise RuntimeError(f"fixture 探针失败: {r.stderr[-400:]}")
    line = [l for l in r.stdout.splitlines() if l.startswith('{"fixture"')][-1]
    return json.loads(line)


def band_check(fx: dict, band: dict) -> list[str]:
    """冻结带比对（fail-closed；M99 同构：双 digest 全等 + rel_dev ≤ 带）。"""
    problems: list[str] = []
    e = band["entries"][0]
    if fx.get("product_digest") != e["product_digest"]:
        problems.append(f"product_digest 漂移: {fx.get('product_digest')} ≠ golden {e['product_digest']}")
    if fx.get("m96_host_digest") != e["m96_digest"]:
        problems.append(f"m96_host_digest 漂移: {fx.get('m96_host_digest')} ≠ golden {e['m96_digest']}")
    if not (fx.get("rel_dev", 1e30) <= e["band_rel_dev"]):
        problems.append(f"rel_dev {fx.get('rel_dev')} > 带 {e['band_rel_dev']}（越带即 RED）")
    if fx.get("matched_depth") != band.get("matched_depth"):
        problems.append("matched_depth 漂移")
    return problems
