#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波）
"""G11.3 资产与场景面修复波门共享库（milestones/g11/CI_GATES.md §4 M147~M152 消费面）。

单一事实源面（禁第二份手写）：
- G11.3 帧区路径（K:/rurix-ext/g11-frames/g11_3/——G10/G11.2 帧库只读分区隔离）与
  G10.5 锁定帧库路径（基线复现面，只读）；
- 复跑报告装载（g11_3_rerun_report.json，milestones/g11/harness/g11_3_ab_rerun.py 产）；
- 修复闭环收敛判定（RXS-0393 L2 quality_gap 款：|复测 delta| < |基线 delta| 且
  收敛幅度 ≥ 标定阈；方向性注入即 RED——符号翻转仅当 |复测 delta| ≤ zero_band
  〔跨端离散一致性标定带，per-tile XOR p100×k measured 产〕时成立）；
- 标定程序两跑逐位一致 + 标定值入 g11_budget 字节级纯追加（M138/M145 同纪律）。
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g10_exr_lib as exr  # noqa: E402
import g10_ssim_psnr_lib as ssim_psnr  # noqa: E402
import g11_2_caliber_lib as cl  # noqa: E402

CORPUS = cl.CORPUS
GAP_REGISTRY = cl.GAP_REGISTRY
BUDGET_PATH = cl.BUDGET_PATH
LOCKED_DIGEST = cl.LOCKED_DIGEST
REPORT_PATH = ROOT / "milestones" / "g11" / "g11_3_rerun_report.json"
TRANSCODE_MANIFEST = ROOT / "milestones" / "g11" / "g11_3_dds_transcode_manifest.json"
FRAMES_G11_3 = Path(r"K:\rurix-ext\g11-frames\g11_3")
FRAMES_G10_5 = Path(r"K:\rurix-ext\g10-frames\g10_5")
RUST_RELEASE_BIN = cl.RUST_RELEASE_BIN
DDS_DUMP_BIN = ROOT / "target" / "release" / "g11_3_dds_dump.exe"
BUILD_SCENES_PY = cl.BUILD_SCENES_PY
SCENE_RENDER_RS = cl.SCENE_RENDER_RS

# G10.5a 锁定帧内容 digest（g10_5_ab_preview.md §3 登记面——默认面零降级对账锚）。
G10_5_FRAME_DIGEST = {
    ("rurix", "cornell-box"): "sha256:c2000ebfbe90359d55e668f8af3b7df24d64c3f72e637904f614821b7ad0d727",
    ("rurix", "bistro-interior"): "sha256:8519cc67c917e7b8c2c5a9bb5633ea5ee9e72deb8cf63b3b187b0d3ac5bb9935",
    ("ue5", "cornell-box"): "sha256:c7c6f2cf1644ba79512da1f4f3fceeb2001826f4723681a35ab7a8ca9dc853a2",
    ("ue5", "bistro-interior"): "sha256:5bfe1f4965e72e85d4c75f21879f8c89bf1f4e292348fa7e82cd9faf0245cc19",
}

SCENES = cl.SCENES


def load_json(path: Path):
    return cl.load_json(path)


def sha256_file(path: Path) -> str:
    return cl.sha256_file(path)


def load_report() -> dict:
    return load_json(REPORT_PATH)


def gap_row(title_prefix: str) -> dict:
    return cl.gap_row(title_prefix)


def contract_digest_rust(scene_id: str) -> str:
    return cl.contract_digest_rust(scene_id)


def validate_budget_entry(entry: dict, p100: float, k: float) -> list[str]:
    return cl.validate_budget_entry(entry, p100, k)


def append_budget_entries(entries: list[dict]) -> list[str]:
    return cl.append_budget_entries(entries)


def hdr_frame(scene_id: str, end: str, root: Path = FRAMES_G11_3) -> Path:
    if end == "rurix":
        return root / "rurix" / f"{scene_id}.exr"
    return root / "ue" / scene_id / ".0000.exr"


def ldr_frame(scene_id: str, end: str, root: Path = FRAMES_G11_3) -> Path:
    return root / "ldr" / f"{scene_id}_{end}_ldr.exr"


def decode(path: Path, end: str) -> dict:
    return exr.decode_exr(path.read_bytes(), end)


def pixels_of(d: dict) -> np.ndarray:
    return np.asarray(d["pixels"], dtype=np.float64).reshape(d["height"], d["width"], 3)


def lum_stats(arr: np.ndarray) -> dict:
    return cl.lum_stats(arr)


def luminance(arr: np.ndarray) -> np.ndarray:
    return 0.2126 * arr[..., 0] + 0.7152 * arr[..., 1] + 0.0722 * arr[..., 2]


def nonzero_mask(arr: np.ndarray) -> np.ndarray:
    return luminance(arr) > 1e-6


def ssim_ldr(scene_id: str, root: Path = FRAMES_G11_3) -> float:
    """LDR 臂 SSIM（参考端 = UE5；g10_ssim_psnr_lib 单一事实源）。"""
    a = pixels_of(decode(ldr_frame(scene_id, "ue5", root), "rurix"))
    b = pixels_of(decode(ldr_frame(scene_id, "rurix", root), "rurix"))
    return float(ssim_psnr.ssim_wang2004(a, b))


def ssim_ldr_cross(scene_id: str, end_a: str, root_a: Path, end_b: str, root_b: Path) -> float:
    """跨帧区 LDR 臂 SSIM（M147 双 phase 反向激励旁证 measured 面：
    ssim(ue_修复帧, rurix_未修复 G10.5 帧) vs ssim(ue_修复帧, rurix_修复帧)——
    锁定度量对正确修复结构性不友好的证据链，G11.6 P2 候选行消费）。"""
    a = pixels_of(decode(ldr_frame(scene_id, end_a, root_a), "rurix"))
    b = pixels_of(decode(ldr_frame(scene_id, end_b, root_b), "rurix"))
    return float(ssim_psnr.ssim_wang2004(a, b))


def coverage_delta(scene_id: str, root: Path = FRAMES_G11_3) -> dict:
    """HDR nonzero 覆盖比双端实测（delta = ue − rurix，锁定基线同口径）。"""
    r = nonzero_mask(pixels_of(decode(hdr_frame(scene_id, "rurix", root), "rurix")))
    u = nonzero_mask(pixels_of(decode(hdr_frame(scene_id, "ue5", root), "ue5")))
    rn = float(r.sum() / r.size)
    un = float(u.sum() / u.size)
    return {"rurix": rn, "ue5": un, "delta": un - rn, "r_mask": r, "u_mask": u}


def coverage_zero_band_calibration(scene_id: str, k: float = 2.0, tiles: int = 8) -> dict:
    """跨端覆盖离散一致性标定带（RXS-0393 L3 p100×k 程序纪律）：

    样本集 = 复测帧对 per-tile（8×8 确定性剖分）逐像素对称差（XOR）比——
    双端渲染器对同一几何可见性的离散化差异实测；p100 × k 得 zero_band。
    净 delta ≤ zero_band 内的符号翻转 = 跨端一致包络内的近零穿越（非方向性
    注入）；超出即方向性 RED。标定两跑逐位一致（确定性帧）。"""
    cov = coverage_delta(scene_id)
    xor = np.logical_xor(cov["r_mask"], cov["u_mask"])
    h, w = xor.shape
    samples: list[float] = []
    for ty in range(tiles):
        for tx in range(tiles):
            t = xor[ty * h // tiles:(ty + 1) * h // tiles, tx * w // tiles:(tx + 1) * w // tiles]
            samples.append(float(t.sum() / t.size))
    p100 = max(samples)
    return {
        "p100": p100,
        "k": k,
        "zero_band": p100 * k,
        "sample_count": len(samples),
        "estimator": "p100",
        "global_xor_ratio": float(xor.sum() / xor.size),
    }


def evaluate_closure(baseline_delta: float, retest_delta: float, shrink_threshold: float, zero_band: float = 0.0) -> dict:
    """RXS-0393 L2 quality_gap 款收敛判定（机器形态）：

    收敛 = |复测 delta| < |基线 delta| 且 收敛幅度（|基线|−|复测|）≥ 标定阈；
    方向性：符号翻转仅当 |复测 delta| ≤ zero_band（跨端一致性标定带）成立——
    反向过冲冒充收敛 / 绝对值缩小但双端仍实质不一致冒充闭环即 RED。
    """
    shrink = abs(baseline_delta) - abs(retest_delta)
    shrink_ok = abs(retest_delta) < abs(baseline_delta) and shrink >= shrink_threshold
    same_sign = (retest_delta == 0.0) or (baseline_delta == 0.0) or (
        (retest_delta > 0) == (baseline_delta > 0)
    )
    direction_ok = same_sign or abs(retest_delta) <= zero_band
    return {
        "baseline_delta": baseline_delta,
        "retest_delta": retest_delta,
        "shrink": shrink,
        "shrink_threshold": shrink_threshold,
        "zero_band": zero_band,
        "same_sign": bool(same_sign),
        "direction_ok": bool(direction_ok),
        "converged": bool(shrink_ok and direction_ok),
    }


def run_rust_digest(scene_id: str, extra_flags: list[str] | None = None) -> tuple[int, str, str]:
    """契约 digest 探针（默认面 / --u64-seed 面共用）：返回 (exit, stdout, stderr)。"""
    p = CORPUS / f"contract_params_{scene_id.replace('-', '_')}.json"
    argv = [str(RUST_RELEASE_BIN), *(extra_flags or []), "--contract-digest", str(p)]
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def calib_evidence_payload(subject: str, gate_key: str, matrix_row: str, numeric_step: int,
                           p100: float, k: float, sample_count: int, sample_set_digest: str,
                           provenance_measured: str, ts: str, extra_results: dict | None = None) -> dict:
    """标定 evidence 闭集（g11_2_calibration schema 同构消费面）。"""
    results = {
        "trimmed_mean": p100,
        "estimator": "p100",
        "sample_pair_count": sample_count,
        "safety_factor_k": k,
        "threshold": p100 * k,
    }
    if extra_results:
        results.update(extra_results)
    return {
        "schema_version": 1,
        "subject": subject,
        "symbolic_gate_key": gate_key,
        "milestone": matrix_row,
        "wave": "G11.3",
        "numeric_step": numeric_step,
        "results": results,
        "provenance": {
            "estimator_semantics": "p100 × k（RFC-0026 §4.2 F10 / RXS-0393 L3）",
            "sample_set_digest": sample_set_digest,
            "rerun_report": "milestones/g11/g11_3_rerun_report.json",
            "measured": provenance_measured,
        },
        "environment": {},
        "timestamp": ts,
    }
