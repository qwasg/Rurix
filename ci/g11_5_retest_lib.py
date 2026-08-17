#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.5 波）
"""G11.5 A/B 复测波门共享库（milestones/g11/CI_GATES.md §4 M155/M156 + §5 wave5 消费面）。

单一事实源面（禁第二份手写）：
- G11.5 帧区路径（K:/rurix-ext/g11-frames/g11_5/——G10/G11.2/G11.3/G11.4 帧库只读
  分区隔离）与复跑报告 / 复测差距清单装载（g11_5_rerun_report.json /
  g11_5_retest_gap_registry.json，milestones/g11/harness/g11_5_ab_rerun.py 产）；
- 11 行复测 delta 独立重算面（门侧自帧独立重算 == 清单/报告登记值——未复跑冒充
  判红面：拿 G11.3/G11.4 帧区或锁定基线值冒充 G11.5 当次复测必检出）；
- 收敛阈消费面 = g11_budget g11.fix.* 标定条目（标定程序产 p100×k，P-09 禁手写；
  门核 evidence_file 在树可解 results.trimmed_mean 且 threshold == trimmed_mean×k）；
- 收敛判定层 = ci/g11_3_fix_lib.evaluate_closure（RXS-0393 L2 quality_gap 款
  0-byte 语义复用——同一判定层，禁第二实现）。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g10_exr_lib as exr  # noqa: E402
import g10_ssim_psnr_lib as ssim_psnr  # noqa: E402
import g11_2_caliber_lib as cl  # noqa: E402
import g11_3_fix_lib as fl  # noqa: E402

CORPUS = cl.CORPUS
GAP_REGISTRY = cl.GAP_REGISTRY
BUDGET_PATH = cl.BUDGET_PATH
LOCKED_DIGEST = cl.LOCKED_DIGEST
LOCKED_DIGEST_JOINT = cl.LOCKED_DIGEST_JOINT
# G11.5b 复测集选择面（G11.5b 追加子波同构接线，CI_GATES v1.6 注记）：环境变量
# RURIX_G11_RETEST_SET ∈ {g11_5b（缺省——当前复测闭环面 = 诊断修复后复测集）,
# g11_5（G11.5 首跑集，0-byte 历史面回访用）}，闭集外值 fail-closed；门禁检集 /
# 锁定基线 / 标定阈消费面 0-byte 不动——仅复测输入集（帧区/报告/清单）随集切换，
# evidence 面经 registry_digest/report_digest 字段自证消费集（G11.5 首跑 FAIL
# evidence 0-byte 保留）。
_RETEST_SET = os.environ.get("RURIX_G11_RETEST_SET", "g11_5b")
if _RETEST_SET == "g11_5b":
    REPORT_PATH = ROOT / "milestones" / "g11" / "g11_5b_rerun_report.json"
    RETEST_REGISTRY_PATH = ROOT / "milestones" / "g11" / "g11_5b_retest_gap_registry.json"
    FRAMES_G11_5 = Path(r"K:\rurix-ext\g11-frames\g11_5b")
    REGISTRY_NAME = "g11_5b_retest_gap_registry"
elif _RETEST_SET == "g11_5":
    REPORT_PATH = ROOT / "milestones" / "g11" / "g11_5_rerun_report.json"
    RETEST_REGISTRY_PATH = ROOT / "milestones" / "g11" / "g11_5_retest_gap_registry.json"
    FRAMES_G11_5 = Path(r"K:\rurix-ext\g11-frames\g11_5")
    REGISTRY_NAME = "g11_5_retest_gap_registry"
else:
    raise ValueError(f"RURIX_G11_RETEST_SET 闭集外: {_RETEST_SET!r}（g11_5|g11_5b）")
RESIDUAL_PATH = cl.RESIDUAL_PATH
FRAMES_G10_5 = fl.FRAMES_G10_5
RUST_RELEASE_BIN = cl.RUST_RELEASE_BIN

SCENES = cl.SCENES

# G11.2 域统一换算基线（C2 对齐面；g11_4_fix_lib / M144 门登记面同一字面）。
ALIGNED_BASELINE_R3 = 2.7314592314362525
ALIGNED_BASELINE_R4 = 4.8486343559026714
ALIGNED_BASELINE_C1_BISTRO_MEDIAN = 2.7314592314362525
ALIGNED_BASELINE_C1_CORNELL_P90 = 0.29024957587122924

evaluate_closure = fl.evaluate_closure
load_json = fl.load_json
sha256_file = fl.sha256_file
gap_row = fl.gap_row
contract_digest_rust = fl.contract_digest_rust
validate_budget_entry = fl.validate_budget_entry


def load_report() -> dict:
    return load_json(REPORT_PATH)


def load_retest_registry() -> dict:
    return load_json(RETEST_REGISTRY_PATH)


def hdr_frame(scene_id: str, end: str, root: Path = FRAMES_G11_5) -> Path:
    if end == "rurix":
        return root / "rurix" / f"{scene_id}.exr"
    return root / "ue" / scene_id / ".0000.exr"


def ldr_frame(scene_id: str, end: str, root: Path = FRAMES_G11_5) -> Path:
    return root / "ldr" / f"{scene_id}_{end}_ldr.exr"


def decode(path: Path, end: str) -> dict:
    return exr.decode_exr(path.read_bytes(), end)


def pixels_of(d: dict) -> np.ndarray:
    return np.asarray(d["pixels"], dtype=np.float64).reshape(d["height"], d["width"], 3)


def lum_stats(arr: np.ndarray) -> dict:
    return cl.lum_stats(arr)


def budget_entries() -> dict:
    return {e.get("id"): e for e in load_json(BUDGET_PATH).get("entries", [])}


def row_thresholds(row_prefix: str) -> dict:
    """收敛阈消费面（g11_budget g11.fix.* 标定条目；缺失即 KeyError——标定未产时
    闭环断言不成立，RXS-0393 L3 字面）。"""
    ids = {
        "R1": ("g11.fix.r1_ssim_shrink_tol", None),
        "R2": ("g11.fix.r2_coverage_shrink_tol", "g11.fix.r2_coverage_zero_band"),
        "R3": ("g11.fix.r3_luminance_shrink_tol", None),
        "R4": ("g11.fix.r4_p90_shrink_tol", None),
        "R5": ("g11.fix.r5_u64_seed_shrink_tol", None),
        "U1": ("g11.fix.u1_coverage_shrink_tol", "g11.fix.u1_coverage_zero_band"),
        "U2": ("g11.fix.u2_luminance_shrink_tol", None),
        "U3": ("g11.fix.u3_anim_channels_shrink_tol", None),
    }
    sid, zid = ids[row_prefix]
    entries = budget_entries()
    out = {"shrink_id": sid, "shrink_tol": entries[sid]["threshold"], "shrink_entry": entries[sid]}
    if zid:
        out["zero_band_id"] = zid
        out["zero_band"] = entries[zid]["threshold"]
        out["zero_band_entry"] = entries[zid]
    else:
        out["zero_band_id"] = None
        out["zero_band"] = 0.0
    return out


def ssim_ldr(scene_id: str, root: Path = FRAMES_G11_5) -> float:
    """LDR 臂 SSIM（参考端 = UE5；g10_ssim_psnr_lib 单一事实源）。"""
    a = pixels_of(decode(ldr_frame(scene_id, "ue5", root), "rurix"))
    b = pixels_of(decode(ldr_frame(scene_id, "rurix", root), "rurix"))
    return float(ssim_psnr.ssim_wang2004(a, b))


def coverage_delta(scene_id: str, root: Path = FRAMES_G11_5) -> dict:
    """HDR nonzero 覆盖比双端实测（delta = ue − rurix，锁定基线同口径）。"""
    r = fl.nonzero_mask(pixels_of(decode(hdr_frame(scene_id, "rurix", root), "rurix")))
    u = fl.nonzero_mask(pixels_of(decode(hdr_frame(scene_id, "ue5", root), "ue5")))
    rn = float(r.sum() / r.size)
    un = float(u.sum() / u.size)
    return {"rurix": rn, "ue5": un, "delta": un - rn}


def hdr_lum(scene_id: str, end: str, root: Path = FRAMES_G11_5) -> dict:
    return lum_stats(pixels_of(decode(hdr_frame(scene_id, end, root), end)))


def ldr_lum(scene_id: str, end: str, root: Path = FRAMES_G11_5) -> dict:
    return lum_stats(pixels_of(decode(ldr_frame(scene_id, end, root), "rurix")))


def recompute_row_retest(prefix: str, report: dict) -> float:
    """门侧独立重算 11 行复测 delta（自 G11.5 帧/探针当次实测——未复跑冒充判红面：
    清单/报告登记值必须与本重算逐位一致）。host 面行（R5/U3）自报告闭集块重算。
    C 族行返回主度量面 delta（C1=bistro HDR 中位；C2=派生尺度差；C3=度量域位深差）。"""
    metrics = report["results"]["metrics"]
    scenes = metrics["scenes"]
    if prefix == "R1":
        return 1.0 - ssim_ldr("bistro-interior")
    if prefix == "R2" or prefix == "U1":
        return coverage_delta("cornell-box")["delta"]
    if prefix == "R3":
        return hdr_lum("bistro-interior", "ue5")["median"] - hdr_lum("bistro-interior", "rurix")["median"]
    if prefix == "R4":
        return hdr_lum("bistro-interior", "ue5")["p90"] - hdr_lum("bistro-interior", "rurix")["p90"]
    if prefix == "U2":
        return ldr_lum("bistro-interior", "ue5")["median"] - ldr_lum("bistro-interior", "rurix")["median"]
    if prefix == "C1":
        return scenes["bistro-interior"]["hdr_luminance_median_delta"]
    if prefix == "R5":
        face = metrics["closure_faces"]["r5"]
        return 0.0 if face["retest_u64_max_consumed"] else face["baseline_delta"]
    if prefix == "U3":
        anim = report["results"]["rurix"]["bistro-interior"]["render_json"]["animations"]
        ok = (
            anim.get("package_count") == 1 and anim.get("channels") == 2
            and anim.get("consumed_channels") == 0 and anim.get("policy") == "strip_static_contract"
        )
        return 0.0 if ok else float(anim.get("channels", 0))
    if prefix == "C2":
        derive = report["results"]["derive"]
        scales = [v["exposure_scale_host"] for v in derive.values()]
        in_pipe = [report["results"]["rurix"][s]["exposure_scale_in_pipe"] for s in SCENES]
        unified = len(scales) == 4 and all(v == 1.0 for v in scales) and in_pipe == [0.25, 0.5]
        return 0.0 if unified else gap_row("C2")["measured_delta"][0]["delta"]
    if prefix == "C3":
        ue_depth = report["results"]["ue"]["bistro-interior"]["source_bit_depth"]
        return 0.0 if ue_depth == "float16" else gap_row("C3")["measured_delta"][0]["delta"]
    raise KeyError(f"未知行前缀: {prefix}")


def run_rust_digest(scene_id: str, extra_flags: list[str] | None = None) -> tuple[int, str, str]:
    return fl.run_rust_digest(scene_id, extra_flags)
