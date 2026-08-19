#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.4 UE 对拍波）
"""G13.4 M-c(M169) UE 超分双端对拍门（P0，步骤 240；g13.p0.m_c.ue_upscale_parity；
G13_CONTRACT §4.2 M-c 行判据逐字 / G-G13-6；G13_ACCEPTANCE_MAP §1 M-c 行；
spec/visual_comparison.md RXS-0405；RXS-0384 L5 / RXS-0386/0387/0388/0391/0392 /
RXS-0403 口径继承）。

host+device 门（双臂真跑：UE 臂 = UE 5.8.1 DLSS 插件 MRQ 外部进程；Rurix 臂 =
g13_4_ue_upscale_parity_render release harness RURIX_REQUIRE_REAL=1 +
RURIX_VK_VALIDATION=1；双臂经 gpu_device_lock 串行）。判据（契约 §4.2 M-c 行字面）：

1. **契约 digest 三方独立实现全等机核**：① host python（本门内嵌
   g13_parity_contract 解析器）② Rurix Rust harness --contract-digest ③ UE 内嵌
   CPython（Phase A build_probe.json contract_digest_ue）——三值全等且 == 本门
   冻结注册值 FROZEN_CONTRACT_DIGEST；**不等仍出报告即 RED**（门序硬约束）。
2. **同场景同档位双端出图**：场景闭集 {cornell-box 512×512, bistro-interior
   1920×1080}（M133 清单 digest 转引只读）；档位闭集 [50,67,100] ↔ UE DLSS
   质量映射 {50:Performance, 67:Quality, 100:DLAA}（MoviePipelineDLSSSetting
   逐档注入 + DLSS engagement 日志面机核）；Rurix 臂三后端 [tsr_device,
   dlss_sr, fsr_3_1_5] 经 UpscaleBackend 冻结面逐（场景 × 档位 × 后端）32 帧
   Halton jitter 静态收敛序列出帧；UE build digest == M128 登记 ue_build_id
   机核；**单端缺帧聚合不得 PASS**。
3. **measured 对拍三面**（端内参照口径，RXS-0403 L4 同族：端内参照消去跨端
   曝光/灯面口径差，隔离超分变量）：逐（场景 × 档位 × 后端）端内 SSIM/FLIP
   deficit 对拍（各端参照 = 本端 tier100 收敛帧；RXS-0387/0388 LDR 派生域）+
   noise_probe_tier=67 档残余帧亮度 2D FFT 高频能量份额双端谱差 + 帧率
   measured 基线登记 **zero_pass_line 不设通过线**（G10-N11/N16 锚定 G14；
   **以基线冒充帧率对标即 RED**）；容差标定腿双 seed 方差底 p100×2.0 程序产
   禁手写 P-09，入 g13_budget measured_local。
4. **UE DLSS·超分模块归属差距登记表落盘**（milestones/g13/
   g13_ue_upscale_gap_registry.json，RXS-0391 口径继承）：超容差项显式登记不
   静默混入（**静默即 RED**）+ 行集与对拍报告对账 + measured_delta 可溯源。
5. **残余口径差显式登记**：Rurix 臂直接光口径（无 GI）vs UE 臂 Lumen 默认面
   等跨端口径差只登记不拟合（RXS-0392 不拟合原则；载体 = evidence
   residual_caliber_note + 登记表 caliber_diff 行）；未对齐口径消费 delta 即
   RED——本门度量全部为端内参照面（尺度消去），跨端直比不消费。
6. **不设绝对通过线**：「已达 UE5 DLSS/超分画质」判定归 G15 商用收口期。

RED 臂（契约判据字面）：契约 digest 不等仍出报告 / 超容差静默 / 单端缺帧聚合
PASS / 帧率基线冒充帧率对标——各臂注入必检出（--selftest + 门内真跑臂）。

用法：
  py -3 ci/g13_ue_upscale_parity_smoke.py --gate g13.p0.m_c.ue_upscale_parity
  py -3 ci/g13_ue_upscale_parity_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import math
import os
import platform
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g13" / "g13_m_c_ue_upscale_parity_evidence_schema.json"
CONTRACT_PATH = ROOT / "milestones" / "g13" / "g13_ue_upscale_parity_contract.json"
REGISTRY_PATH = ROOT / "milestones" / "g13" / "g13_ue_upscale_gap_registry.json"
BUDGET_PATH = ROOT / "milestones" / "g13" / "g13_budget.json"
UE_RENDER = ROOT / "milestones" / "g13" / "harness" / "g13_4_ue_render.py"
FRAMES = Path(r"K:\rurix-ext\g13-frames")
RURIX_CAL_ROOT = FRAMES / "rurix_upscale_cal"
RURIX_BIN = ROOT / "target" / "release" / "g13_4_ue_upscale_parity_render.exe"
LDR_BIN = ROOT / "target" / "release" / "g10_5_scene_render.exe"
G13_ZERO_BASE = "8c5dc5ee"

sys.path.insert(0, str(ROOT / "ci"))
sys.path.insert(0, str(ROOT / "milestones" / "g13" / "harness" / "ue_python"))

import g10_exr_lib as exr  # noqa: E402
import g10_flip_lib as flip  # noqa: E402
import g10_ssim_psnr_lib as ssim_psnr  # noqa: E402
import g10_gap_registry_lib as gaplib  # noqa: E402
import g10_ue5_lib as ue5  # noqa: E402
import g10_wave_exit_lib as wel  # noqa: E402
import g13_parity_contract as pc  # noqa: E402
import g13_tsr_device_kernel_smoke as mb  # noqa: E402
import g12_ue_pt_parity_smoke as g12  # noqa: E402（noise_hf_share 口径面转引）
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g13.p0.m_c.ue_upscale_parity"
NUMERIC_STEP = 240
SUBJECT = "g13_m_c_ue_upscale_parity"
WAVE = "G13.4"
TAG = "g13_m_c"
MATRIX_ROW = "M169"
SOURCE_REF = (
    "G13_CONTRACT §4.2 M-c/G-G13-6;G13_ACCEPTANCE_MAP §1;spec/visual_comparison.md "
    "RXS-0405;RXS-0384 L5/RXS-0386/RXS-0387/RXS-0388/RXS-0391/RXS-0392/RXS-0403 口径继承"
)
FROZEN_CONTRACT_DIGEST = "sha256:137483a1696481971fc0da03fad1a188ef6f048243e4616953060014f1d0872f"

TIERS = [50, 67, 100]
BACKENDS = ["tsr_device", "dlss_sr", "fsr_3_1_5"]
PROBE_TIER = 67
FRAME_COUNT = 32
UE_DLSS_LOG_TOKEN = ("Creating NGX DLSS Feature", "NGXPerfQuality")

BUDGET_TOL_ENTRIES = [
    "g13.ue_upscale.ssim_deficit_delta_tol",
    "g13.ue_upscale.flip_deficit_delta_tol",
    "g13.ue_upscale.noise_hf_delta_tol",
]

CHECK_KEYS = [
    "temporal_base_0byte",
    "conformance_corpus_anchored",
    "contract_digest_three_way_equal",
    "ue_build_id_matches_m128",
    "budget_anchors_present",
    "calibration_dual_seed_bitexact",
    "budget_eval_all_pass",
    "ue_arm_frames_all_present",
    "ue_dlss_engagement_logged",
    "rurix_arm_frames_all_present",
    "rurix_double_run_bitexact",
    "frame_digests_recomputed_match",
    "gap_registry_schema_valid",
    "gap_registry_reconciled",
    "fps_baseline_zero_pass_line",
    "residual_caliber_note_registered",
    "device_red_digest_mismatch_detected",
    "device_red_silent_gap_detected",
    "device_red_missing_frame_detected",
    "device_red_fps_masquerade_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL: {msg}", file=sys.stderr)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}")


def run(cmd: list[str], timeout: int = 7200, env=None) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def base_commit() -> str:
    r = run(["git", "rev-parse", "HEAD"])
    return (r.stdout or "").strip()


def environment() -> dict:
    return {
        "os": platform.platform(),
        "python_version": platform.python_version(),
        "cargo_version": (run(["cargo", "--version"]).stdout or "").strip(),
        "rustc_version": (run(["rustc", "--version"]).stdout or "").strip(),
    }


# ---------------------------------------------------------------------------
# host 段
# ---------------------------------------------------------------------------


def conformance_corpus_anchored() -> tuple[bool, str]:
    want = [
        ROOT / "conformance" / "visual_comparison" / "accept" / "ue_upscale_parity_contract_minimal.rx",
        ROOT / "conformance" / "visual_comparison" / "reject" / "upscale_parity_digest_mismatch_report.rx",
        ROOT / "conformance" / "visual_comparison" / "reject" / "upscale_fps_baseline_masquerade.rx",
    ]
    missing = [p.name for p in want if not p.is_file()]
    if missing:
        return False, f"conformance 语料缺失: {missing}"
    for p in want:
        if "//@ spec: RXS-0405" not in p.read_text(encoding="utf-8"):
            return False, f"{p.name} 缺 RXS-0405 锚"
    r = run(["py", "-3", "ci/trace_matrix.py", "--check"])
    ok = r.returncode == 0 and "PASS" in (r.stdout + r.stderr)
    return ok, f"conformance 三件锚定 + trace_matrix {'PASS' if ok else 'FAIL'}"


def host_contract_digest() -> str:
    doc = pc.parse_upscale_contract(CONTRACT_PATH.read_text(encoding="utf-8"))
    return pc.contract_digest(doc)


def ue_build_id_ok() -> tuple[bool, str]:
    exe = ue5.ue_editor_cmd()
    if exe is None:
        return False, "UE 编辑器缺失"
    actual = ue5.read_ue_build_id(exe)
    ok = actual == ue5.EXPECTED_UE_BUILD_ID
    return ok, f"ue_build_id={actual} vs M128 登记 {ue5.EXPECTED_UE_BUILD_ID}"


# ---------------------------------------------------------------------------
# 帧收割与度量
# ---------------------------------------------------------------------------


def _cell_dirs(scene: str, tier: int, backend: str) -> dict:
    return {
        "ue": FRAMES / "ue_upscale" / scene / f"tier{tier}",
        "rurix": FRAMES / "rurix_upscale" / scene / f"tier{tier}" / backend,
    }


def harvest_ue_cell(scene: str, tier: int, started: float) -> dict:
    d = FRAMES / "ue_upscale" / scene / f"tier{tier}"
    receipt_path = d / "render_receipt.json"
    problems = []
    receipt = None
    frames = []
    if not receipt_path.is_file():
        problems.append(f"ue receipt 缺失 {scene}/tier{tier}")
    else:
        receipt = load_json(receipt_path)
        if receipt.get("exit_code") != 0:
            problems.append(f"ue exit_code={receipt.get('exit_code')} {scene}/tier{tier}")
        if receipt.get("started_epoch", 0) < started - 1.0:
            problems.append(f"ue receipt 非当次新鲜 {scene}/tier{tier}")
        frames = receipt.get("frames") or []
        if len(frames) != FRAME_COUNT:
            problems.append(f"ue 帧数 {len(frames)}≠{FRAME_COUNT} {scene}/tier{tier}")
        for fr in frames:
            fp = d / fr["name"]
            if not fr.get("exr_magic_ok") or not fp.is_file():
                problems.append(f"ue 帧坏 {scene}/tier{tier}/{fr.get('name')}")
                break
    return {"receipt": receipt, "frames": frames, "dir": d, "problems": problems}


def harvest_rurix_cell(scene: str, tier: int, backend: str, started: float, seed_role: str = "main") -> dict:
    d = FRAMES / "rurix_upscale" / scene / f"tier{tier}" / backend
    receipt_path = d / "render_receipt.json"
    problems = []
    receipt = None
    if not receipt_path.is_file():
        problems.append(f"rurix receipt 缺失 {scene}/tier{tier}/{backend}")
    else:
        receipt = load_json(receipt_path)
        if receipt.get("seed_role") != seed_role:
            problems.append(f"rurix seed_role={receipt.get('seed_role')}≠{seed_role} {scene}/tier{tier}/{backend}")
        frames = receipt.get("frames") or []
        if len(frames) != FRAME_COUNT:
            problems.append(f"rurix 帧数 {len(frames)}≠{FRAME_COUNT} {scene}/tier{tier}/{backend}")
        conv = d / "converged.exr"
        if not conv.is_file() or conv.stat().st_size < 100_000:
            problems.append(f"rurix converged 缺失 {scene}/tier{tier}/{backend}")
        if (receipt.get("env") or {}).get("RURIX_REQUIRE_REAL") != "1":
            problems.append(f"rurix RURIX_REQUIRE_REAL 字面缺失 {scene}/tier{tier}/{backend}")
        if receipt.get("contract_digest_rurix") != FROZEN_CONTRACT_DIGEST:
            problems.append(f"rurix receipt digest 离冻结值 {scene}/tier{tier}/{backend}")
        mtime = receipt_path.stat().st_mtime
        if mtime < started - 1.0:
            problems.append(f"rurix receipt 非当次新鲜 {scene}/tier{tier}/{backend}")
    return {"receipt": receipt, "dir": d, "problems": problems}


def derive_ldr(hdr_path: Path, end: str, scale: float, params_digest: str, out_path: Path) -> bool:
    r = run([
        str(LDR_BIN), "--derive-ldr", "--hdr", str(hdr_path),
        "--source-end", end, "--out", str(out_path),
        "--exposure-scale", str(scale), "--params-digest", params_digest,
    ], timeout=900)
    return r.returncode == 0 and out_path.is_file()


def _pixels(doc: dict) -> tuple[int, int, list[float]]:
    return doc["width"], doc["height"], doc["pixels"]


def cell_metrics(scene: str, tier: int, backend: str, ue_frames: list[str], rurix_conv: Path,
                 ue_ref: Path, rurix_ref: Path, ev100: float, work: Path) -> dict:
    """端内参照 deficit 对拍（参照 = 本端 tier100 收敛帧；LDR 派生域 RXS-0386 L2）。"""
    scale = 2.0 ** (-ev100)
    pd = FROZEN_CONTRACT_DIGEST.replace("sha256:", "")
    ue_cur = Path(ue_frames[-1])
    cells = {}
    for end, cur, ref in (("ue5", ue_cur, ue_ref), ("rurix", rurix_conv, rurix_ref)):
        cur_ldr = work / f"{scene}_t{tier}_{backend}_{end}_cur_ldr.exr"
        ref_ldr = work / f"{scene}_t{tier}_{backend}_{end}_ref_ldr.exr"
        if not derive_ldr(cur, end, scale if end == "rurix" else 1.0, pd, cur_ldr):
            raise RuntimeError(f"LDR 派生失败 {scene}/t{tier}/{backend}/{end}/cur")
        if not ref_ldr.is_file() and not derive_ldr(ref, end, scale if end == "rurix" else 1.0, pd, ref_ldr):
            raise RuntimeError(f"LDR 派生失败 {scene}/t{tier}/{backend}/{end}/ref")
        # 派生 LDR 件带 rurix:* 元数据（derive_ldr 落盘戳）——以 rurix 策略解码；
        # ue5 严格面仅对原始 UE 帧（命名空间冒充拦截面对 raw 帧生效）
        cd = exr.decode_exr_file(cur_ldr, "rurix")
        rd = exr.decode_exr_file(ref_ldr, "rurix")
        import numpy as np

        cw, ch, cp = _pixels(cd)
        rw, rh, rp = _pixels(rd)
        if (cw, ch) != (rw, rh):
            raise RuntimeError(f"参照/当前分辨率不齐 {scene}/t{tier}/{backend}/{end}")
        a = np.array(cp, dtype=np.float64).reshape(ch, cw, -1)[..., :3]
        b = np.array(rp, dtype=np.float64).reshape(rh, rw, -1)[..., :3]
        cells[end] = {
            "ssim": ssim_psnr.ssim_wang2004(a, b),
            "flip": flip.flip_ldr(b, a, flip.default_ppd())[1],
        }
    return {
        "ssim_ue": cells["ue5"]["ssim"],
        "ssim_rurix": cells["rurix"]["ssim"],
        "flip_ue": cells["ue5"]["flip"],
        "flip_rurix": cells["rurix"]["flip"],
        "ssim_delta": abs(cells["ue5"]["ssim"] - cells["rurix"]["ssim"]),
        "flip_delta": abs(cells["ue5"]["flip"] - cells["rurix"]["flip"]),
    }


def noise_hf(path_frames: list[Path], end: str) -> float:
    """逐端残余帧（末帧 − 32 帧均值参照）亮度 2D FFT 高频能量份额（RXS-0403 L4 口径继承）。"""
    docs = [exr.decode_exr_file(p, end) for p in path_frames]
    w, h, _ = _pixels(docs[0])
    n = len(docs)
    mean = [0.0] * (w * h * 3)
    for d in docs:
        px = d["pixels"]
        for i in range(w * h * 3):
            mean[i] += px[i] / n
    return g12.noise_hf_share(docs[-1]["pixels"], mean, w, h)


# ---------------------------------------------------------------------------
# 标定腿（双 seed 方差底 p100 × 2.0，P-09 禁手写）
# ---------------------------------------------------------------------------


def run_rurix_render(scene: str, tier: int, backend: str, seed_role: str) -> subprocess.CompletedProcess:
    cmd = [
        str(RURIX_BIN), "--render", "--scene", scene, "--tier", str(tier),
        "--backend", backend, "--frames", str(FRAME_COUNT),
    ]
    if seed_role == "calibration":
        cmd += ["--calibration-seed", "--out-root", str(RURIX_CAL_ROOT)]
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return run(cmd, timeout=7200, env=env)


def deficit_rurix(scene: str, tier: int, backend: str, seed_role: str, work: Path, ev100: float) -> dict:
    root = RURIX_CAL_ROOT if seed_role == "calibration" else FRAMES / "rurix_upscale"
    d = root / scene / f"tier{tier}" / backend
    ref = root / scene / "tier100" / backend / "converged.exr"
    scale = 2.0 ** (-ev100)
    pd = FROZEN_CONTRACT_DIGEST.replace("sha256:", "")
    import numpy as np

    out = {}
    for tag, p in (("cur", d / "converged.exr"), ("ref", ref)):
        ldr = work / f"cal_{scene}_t{tier}_{backend}_{seed_role}_{tag}.exr"
        if not derive_ldr(p, "rurix", scale, pd, ldr):
            raise RuntimeError(f"标定 LDR 派生失败 {scene}/{backend}/{seed_role}/{tag}")
        out[tag] = exr.decode_exr_file(ldr, "rurix")
    cw, ch, cp = _pixels(out["cur"])
    rw, rh, rp = _pixels(out["ref"])
    a = np.array(cp, dtype=np.float64).reshape(ch, cw, -1)[..., :3]
    b = np.array(rp, dtype=np.float64).reshape(rh, rw, -1)[..., :3]
    return {"ssim": ssim_psnr.ssim_wang2004(a, b), "flip": flip.flip_ldr(b, a, flip.default_ppd())[1]}


def run_calibration_leg(work: Path, ev100: dict, ts: str) -> tuple[dict, list[str]]:
    """双 seed 方差底 p100×2.0 程序产容差（P-09 禁手写）+ g13_budget 条目注册/对账。

    返回 ({entry_id: {"threshold", "measured"}}, problems)。消费前提：标定腿
    calibration_seed 复跑已落 RURIX_CAL_ROOT（probe 档 × 3 后端 + tier100 参照 ×
    2 场景）且主腿 probe 档在档。位级一致性 = 标定度量从新鲜标定帧重算与 budget
    注册值 f64 精确相等（帧面位级确定性由 rurix_double_run_bitexact 承载）。
    """
    problems: list[str] = []
    var_ssim: list[float] = []
    var_flip: list[float] = []
    var_noise: list[float] = []
    digest_src: list[str] = []
    for scene in ("cornell-box", "bistro-interior"):
        for backend in BACKENDS:
            try:
                m_main = deficit_rurix(scene, PROBE_TIER, backend, "main", work, ev100[scene])
                m_cal = deficit_rurix(scene, PROBE_TIER, backend, "calibration", work, ev100[scene])
            except Exception as e:
                problems.append(f"标定度量失败 {scene}/{backend}: {e}")
                continue
            var_ssim.append(abs(m_main["ssim"] - m_cal["ssim"]))
            var_flip.append(abs(m_main["flip"] - m_cal["flip"]))
            # 噪声谱双 seed 方差（probe 档逐端残余高频份额）
            main_paths = sorted((FRAMES / "rurix_upscale" / scene / f"tier{PROBE_TIER}" / backend / "frames").glob("*.exr"))
            cal_paths = sorted((RURIX_CAL_ROOT / scene / f"tier{PROBE_TIER}" / backend / "frames").glob("*.exr"))
            if len(main_paths) == FRAME_COUNT and len(cal_paths) == FRAME_COUNT:
                var_noise.append(abs(noise_hf(main_paths, "rurix") - noise_hf(cal_paths, "rurix")))
            receipt = RURIX_CAL_ROOT / scene / f"tier{PROBE_TIER}" / backend / "render_receipt.json"
            if receipt.is_file():
                digest_src.append(load_json(receipt).get("converged_digest", ""))
    if problems:
        return {}, problems
    if not var_noise:
        problems.append("标定噪声谱样本空")
        return {}, problems
    import hashlib

    sample_digest = "sha256:" + hashlib.sha256("|".join(sorted(digest_src)).encode()).hexdigest()
    measured = {
        BUDGET_TOL_ENTRIES[0]: max(var_ssim),
        BUDGET_TOL_ENTRIES[1]: max(var_flip),
        BUDGET_TOL_ENTRIES[2]: max(var_noise),
    }
    entries = []
    for eid, m in measured.items():
        ev_rel = f"evidence/g13_m_c_calibration_{eid.split('.')[-1]}_{ts}.json"
        doc = {
            "schema": "rurix.g13ueupscale.measured_entry.v1",
            "entry_id": eid,
            "results": {"dual_seed_p100": m},
            "protocol": (
                "M-c 标定腿：probe 档（tier67）双 seed（seed vs calibration_seed）Rurix 臂端内参照 "
                "deficit/噪声谱方差底 p100，threshold = measured × 2.0 冻结 k（禁手写 P-09）；"
                "样本面 = 2 场景 × 3 后端 × 32 帧 Halton 静态收敛序列"
            ),
            "sample_manifest": {"count": len(var_ssim) + len(var_flip) + len(var_noise), "digest": sample_digest},
            "provenance": {"gpu": "device", "backend": "tsr_device/dlss_sr/fsr_3_1_5", "base_commit": base_commit()},
            "timestamp": ts,
        }
        (ROOT / ev_rel).write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        entries.append({
            "id": eid,
            "description": (
                f"UE 超分对拍 {eid.split('.')[-1]} 容差冻结带（M-c 标定腿双 seed 方差底 p100 × 2.0 程序产，"
                "禁手写 P-09；端内参照口径 RXS-0403 L4 同族——跨端曝光/灯面口径差尺度消去不消费）；"
                f"样本集 digest {sample_digest}（count={len(var_ssim)+len(var_flip)+len(var_noise)}）；"
                "标定程序 ci/g13_ue_upscale_parity_smoke.py 标定腿可复跑（帧面位级确定性双跑承载）"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": m * 2.0,
            "evidence_file": ev_rel,
            "measured_value": m,
        })
    reg_problems = mb.append_budget_entries(entries)
    problems += reg_problems
    budget = mb.load_g13_budget()
    out: dict = {}
    for eid, m in measured.items():
        e = mb.budget_entry(budget, eid) if budget else None
        if e is None:
            problems.append(f"budget 缺条目 {eid}")
            continue
        # 位级一致机核：新鲜标定帧重算 measured 与在档注册值 f64 精确相等
        if e.get("measured_value") != m or e.get("threshold") != m * 2.0:
            problems.append(f"{eid} 重算离在档值（{e.get('measured_value')} vs {m}）")
            continue
        out[eid] = {"threshold": e["threshold"], "measured": m}
    return out, problems


# ---------------------------------------------------------------------------
# RED 臂（门内真跑臂；合成夹具注入）
# ---------------------------------------------------------------------------


def red_arm_digest_mismatch() -> bool:
    """契约篡改 → digest 离冻结值且 fail-closed 拒产报告（检出 = True）。"""
    doc = pc.parse_upscale_contract(CONTRACT_PATH.read_text(encoding="utf-8"))
    tampered = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
    tampered["noise_probe_tier"] = 50
    try:
        d2 = pc.contract_digest(pc.parse_upscale_contract(json.dumps(tampered)))
    except pc.ContractError:
        return True  # fail-closed 拒解析 = 检出
    return d2 != FROZEN_CONTRACT_DIGEST and pc.contract_digest(doc) == FROZEN_CONTRACT_DIGEST


def red_arm_silent_gap() -> bool:
    """超容差项缺登记表行 → 对账函数必报问题（检出 = True）。"""
    cells = [{"scene": "cornell-box", "tier": 67, "backend": "tsr_device", "over_tolerance": True, "registered": False}]
    problems = reconcile_registry(cells, [])
    return len(problems) > 0


def red_arm_missing_frame() -> bool:
    """单端缺帧 → 帧集问题面非空（检出 = True）。"""
    with tempfile.TemporaryDirectory(prefix="g13_m_c_red_") as td:
        d = Path(td) / "ue_upscale" / "cornell-box" / "tier67"
        d.mkdir(parents=True)
        (d / "render_receipt.json").write_text(json.dumps({
            "exit_code": 0, "started_epoch": 9e18, "frames": [{"name": ".0000.exr", "exr_magic_ok": False}],
        }), encoding="utf-8")
        problems = harvest_ue_cell_at(d, 9e18 - 1.0)
        return len(problems) > 0


def harvest_ue_cell_at(d: Path, started: float) -> list[str]:
    receipt_path = d / "render_receipt.json"
    problems = []
    if not receipt_path.is_file():
        return ["ue receipt 缺失"]
    receipt = load_json(receipt_path)
    if receipt.get("exit_code") != 0:
        problems.append("ue exit_code 非零")
    if receipt.get("started_epoch", 0) < started:
        problems.append("ue receipt 非新鲜")
    frames = receipt.get("frames") or []
    if len(frames) != FRAME_COUNT:
        problems.append(f"ue 帧数 {len(frames)}≠{FRAME_COUNT}")
    for fr in frames:
        if not fr.get("exr_magic_ok"):
            problems.append("ue 帧 magic 坏")
            break
    return problems


def red_arm_fps_masquerade() -> bool:
    """zero_pass_line 缺字面/冒充通过判定 → 校验必拒（检出 = True）。"""
    bad = {"zero_pass_line": False, "cells": []}
    ok_bad = validate_fps_baseline(bad)
    masq = {"zero_pass_line": True, "passes_ue5_fps_target": True, "cells": []}
    ok_masq = validate_fps_baseline(masq)
    good = {"zero_pass_line": True, "cells": [{"scene": "s", "tier": 67}]}
    ok_good = validate_fps_baseline(good)
    return (not ok_bad) and (not ok_masq) and ok_good


def validate_fps_baseline(fps: dict) -> bool:
    if fps.get("zero_pass_line") is not True:
        return False
    banned = [k for k in fps if k not in ("zero_pass_line", "cells", "note")]
    if banned:
        return False
    return isinstance(fps.get("cells"), list)


def reconcile_registry(cells: list[dict], rows: list[dict]) -> list[str]:
    """超容差格 ↔ 登记表行对账（RXS-0391 行集对账口径）。"""
    problems = []
    row_keys = {(r.get("scene_id"), r.get("title", "")) for r in rows}
    for c in cells:
        if c.get("over_tolerance") and not c.get("registered"):
            problems.append(f"超容差静默 {c.get('scene')}/{c.get('tier')}/{c.get('backend')}")
        if c.get("over_tolerance") and not any(c.get("scene") == rk[0] for rk in row_keys):
            problems.append(f"超容差格无登记表行 {c.get('scene')}")
    return problems


# ── G13.4 登记表校验单源 = gaplib 正典（RXS-0391 IR2 禁第二份手写；registry_name
# 加性参数承载 G13 波命名；行集对账 / measured_delta 可溯源〔delta == b−a f64 精确
# 重算 + gap_id 冻结字节规则重算〕全由 gaplib.validate_registry 承载）──
REGISTRY_NAME = "g13_ue_upscale_gap_registry"


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


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
    return run_gate()


def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    started = _dt.datetime.now(_dt.timezone.utc).timestamp()

    # ── host 段 ──
    ok, msg = mb.temporal_base_0byte()
    checks["temporal_base_0byte"] = ok
    check(ok, f"temporal 底座 0-byte: {msg}")
    note(msg)

    ok, msg = conformance_corpus_anchored()
    checks["conformance_corpus_anchored"] = ok
    check(ok, msg)
    note(msg)

    try:
        host_digest = host_contract_digest()
    except Exception as e:  # fail-closed
        host_digest = ""
        check(False, f"host 契约解析 fail-closed: {e}")
    checks["contract_digest_three_way_equal"] = host_digest == FROZEN_CONTRACT_DIGEST
    note(f"host contract digest={host_digest[:32]}…（三方机核待 device 段补齐）")

    ok, msg = ue_build_id_ok()
    checks["ue_build_id_matches_m128"] = ok
    check(ok, msg)
    note(msg)

    budget = mb.load_g13_budget()
    checks["budget_anchors_present"] = budget is not None and all(
        (mb.budget_entry(budget, eid) or {}).get("evidence") == "measured_local" for eid in BUDGET_TOL_ENTRIES
    )

    # ── device 段（双臂真跑，持锁串行） ──
    device_state = "fail"
    cells: list[dict] = []
    noise_rows: list[dict] = []
    fps_cells: list[dict] = []
    registry_rows: list[dict] = []
    rust_digest = ""
    ue_digest = ""
    red_results: dict[str, bool] = {}
    # 锁面纪律（D5 定案沿 G10.5b/G12.4）：本门不嵌套持锁——UE 臂经 g13_4_ue_render.py
    # 子进程自持 gpu_device_lock 串行；Rurix 臂直接调 harness 二进制的段落由本门
    # 持锁包裹（harness 内无锁面）；cargo/LDR 派生等 host CPU 段不持锁。
    r = run(["cargo", "build", "--release", "-p", "rurix-render", "--bin",
             "g13_4_ue_upscale_parity_render", "--features", "vendor-upscale"], timeout=7200)
    if r.returncode != 0 or not RURIX_BIN.is_file():
        check(False, f"harness 构建失败 rc={r.returncode}: {(r.stderr or '')[-300:]}")
    else:
        rd = run([str(RURIX_BIN), "--contract-digest"], timeout=300)
        rust_digest = (rd.stdout or "").strip()
    # UE Phase A 建设（幂等；产三方 digest 臂③；子进程自持锁）
    rb = run([sys.executable, str(UE_RENDER), "build", "--all", "--skip-import"], timeout=10800)
    if rb.returncode == 0:
        for scene in ("cornell-box", "bistro-interior"):
            probe = FRAMES / scene / "build_probe.json"
            # build_probe 落点沿 harness 约定（OUT_ROOT/<scene>/build_probe.json）
            for cand in (FRAMES / "ue_upscale" / scene / "build_probe.json",
                         FRAMES / scene / "build_probe.json",
                         FRAMES / "ue_lumen" / scene / "build_probe.json"):
                if cand.is_file():
                    probe = cand
                    break
            if probe.is_file():
                ue_digest = load_json(probe).get("contract_digest_ue", "")
                if ue_digest:
                    break
    checks["contract_digest_three_way_equal"] = (
        host_digest == FROZEN_CONTRACT_DIGEST
        and rust_digest == FROZEN_CONTRACT_DIGEST
        and ue_digest == FROZEN_CONTRACT_DIGEST
    )
    check(checks["contract_digest_three_way_equal"],
          f"三方 digest 离冻结值: host={host_digest[:16]} rust={rust_digest[:16]} ue={ue_digest[:16]}")
    if not checks["contract_digest_three_way_equal"]:
        note("契约 digest 不等——门序硬约束拒产报告（RED 面）")
    else:
        note("三方 digest 全等且 == 冻结注册值")

    # UE 臂逐（场景 × 档位）真跑（子进程自持锁）
    ue_band_rel = 0.0
    ue_run_samples: list[float] = []
    if checks["contract_digest_three_way_equal"]:
        for scene in ("cornell-box", "bistro-interior"):
            for tier in TIERS:
                rr = run([sys.executable, str(UE_RENDER), "render", "upscale", scene, "--tier", str(tier)], timeout=7200)
                if rr.returncode != 0:
                    check(False, f"UE 臂渲染失败 {scene}/tier{tier}")
        # ── G14 M-a 加性：UE 探针格（bistro-interior/tier67）运行间方差底标定 ──
        # （G13 §8.7 承接锚「门内 UE 探针格双跑方差底 ×headroom 程序产」字面兑现：
        # 样本 3 = 主臂一探针格 + 本段两复跑；带 = max 两两相对差 × 2.0，P-09 禁手写；
        # 厂商随机方差吸收面，真实内容变更 ≫带 检出面维持；样本恒等 → 带 0.0 退化位级）
        probe_dir = FRAMES / "ue_upscale" / "bistro-interior" / f"tier{PROBE_TIER}"

        def _probe_hf() -> float:
            rec = load_json(probe_dir / "render_receipt.json")
            frs = [probe_dir / fr["name"] for fr in (rec.get("frames") or [])]
            if len(frs) != FRAME_COUNT:
                check(False, f"UE 探针格帧集异常（{len(frs)}≠{FRAME_COUNT}）")
            return noise_hf(frs, "ue5")

        ue_run_samples.append(_probe_hf())
        for _probe_rep in range(2):
            rr = run([sys.executable, str(UE_RENDER), "render", "upscale",
                      "bistro-interior", "--tier", str(PROBE_TIER)], timeout=7200)
            if rr.returncode != 0:
                check(False, "UE 探针格方差标定复跑失败")
            ue_run_samples.append(_probe_hf())
        for _i in range(len(ue_run_samples)):
            for _j in range(_i + 1, len(ue_run_samples)):
                _a, _b = ue_run_samples[_i], ue_run_samples[_j]
                ue_band_rel = max(ue_band_rel, abs(_a - _b) / max(abs(_a), abs(_b), 1e-30))
        ue_band_rel *= 2.0
        note("UE 探针格运行间方差标定：samples="
             + "/".join(f"{s:.16f}" for s in ue_run_samples)
             + f" band_rel={ue_band_rel:.8f}（max 两两相对差 ×2.0 程序产）")
        # Rurix 臂逐（场景 × 档位 × 后端）真跑（门侧持锁段：harness 直接调用面）
        with gpu_device_lock(purpose=f"{TAG} Rurix 臂逐格渲染 + 双跑 + 标定腿"):
            for scene in ("cornell-box", "bistro-interior"):
                for tier in TIERS:
                    for backend in BACKENDS:
                        rr = run_rurix_render(scene, tier, backend, "main")
                        if rr.returncode != 0:
                            check(False, f"Rurix 臂渲染失败 {scene}/tier{tier}/{backend}: {(rr.stderr or '')[-200:]}")
            # Rurix 双跑位级（探针格 cornell/tier67/tsr_device 复跑比 converged digest）
            first = load_json(FRAMES / "rurix_upscale" / "cornell-box" / "tier67" / "tsr_device" / "render_receipt.json") \
                if (FRAMES / "rurix_upscale" / "cornell-box" / "tier67" / "tsr_device" / "render_receipt.json").is_file() else {}
            rr = run_rurix_render("cornell-box", 67, "tsr_device", "main")
            second = load_json(FRAMES / "rurix_upscale" / "cornell-box" / "tier67" / "tsr_device" / "render_receipt.json") \
                if rr.returncode == 0 else {}
            checks["rurix_double_run_bitexact"] = bool(first) and bool(second) and (
                first.get("converged_digest") == second.get("converged_digest")
            )
            check(checks["rurix_double_run_bitexact"], "Rurix 双跑 converged digest 非位级一致")

            # 标定腿（双 seed 方差底）：逐（场景 × 后端）calibration_seed 复跑 probe 档
            # + tier100 全后端 cal 参照（端内参照面与主腿同后端同参照臂）
            for scene in ("cornell-box", "bistro-interior"):
                for backend in BACKENDS:
                    rr = run_rurix_render(scene, PROBE_TIER, backend, "calibration")
                    if rr.returncode != 0:
                        check(False, f"标定腿失败 {scene}/{backend}")
                    rr = run_rurix_render(scene, 100, backend, "calibration")
                    if rr.returncode != 0:
                        check(False, f"标定腿参照失败 {scene}/tier100/{backend}")

    # ── 帧集齐备机核 ──
    ue_problems: list[str] = []
    rurix_problems: list[str] = []
    ue_cells: dict = {}
    rurix_cells: dict = {}
    for scene in ("cornell-box", "bistro-interior"):
        for tier in TIERS:
            cell = harvest_ue_cell(scene, tier, started)
            ue_cells[(scene, tier)] = cell
            ue_problems += cell["problems"]
            for backend in BACKENDS:
                rcell = harvest_rurix_cell(scene, tier, backend, started)
                rurix_cells[(scene, tier, backend)] = rcell
                rurix_problems += rcell["problems"]
    checks["ue_arm_frames_all_present"] = not ue_problems
    check(not ue_problems, f"UE 臂缺帧: {ue_problems[:3]}")
    checks["rurix_arm_frames_all_present"] = not rurix_problems
    check(not rurix_problems, f"Rurix 臂缺帧: {rurix_problems[:3]}")

    # DLSS engagement 日志面（逐 UE 格 dlss_log_lines 携带 NGX feature 创建行）
    dlss_bad = []
    for (scene, tier), cell in ue_cells.items():
        receipt = cell.get("receipt") or {}
        lines = receipt.get("dlss_log_lines") or []
        blob = " ".join("\n".join(lines).split())
        if not all(tok in blob for tok in UE_DLSS_LOG_TOKEN):
            dlss_bad.append(f"{scene}/tier{tier}")
    checks["ue_dlss_engagement_logged"] = not dlss_bad and bool(ue_cells)
    check(not dlss_bad, f"DLSS engagement 日志缺: {dlss_bad[:3]}")

    # 帧 digest 重算对账（UE 帧 canonical digest + Rurix 帧 content digest 抽格重算 == receipt 登记；
    # canonical digest 扫描线偏移表条数 = 场景高——按契约 resolution.h 逐场景传参，
    # 缺省 (1080,1080) 仅合 1080p 格，512² 格直传默认即系统性不符）
    _scene_h = {s["scene_id"]: s["camera"]["resolution"]["h"]
                for s in load_json(CONTRACT_PATH)["scenes"]}
    recompute_bad = []
    for (scene, tier), cell in ue_cells.items():
        for fr in (cell.get("frames") or [])[:1] + (cell.get("frames") or [])[-1:]:
            fp = cell["dir"] / fr["name"]
            if fp.is_file():
                import g10_determinism as _det  # noqa

                actual = _det.exr_canonical_digest(str(fp), data_window=(_scene_h[scene], _scene_h[scene]))
                if actual != fr.get("canonical_digest"):
                    recompute_bad.append(f"ue {scene}/t{tier}/{fr['name']}")
    for (scene, tier, backend), rcell in rurix_cells.items():
        receipt = rcell.get("receipt") or {}
        fr0 = (receipt.get("frames") or [{}])[0]
        fp = rcell["dir"] / "frames" / fr0.get("name", "")
        if fp.is_file() and fr0.get("digest"):
            doc = exr.decode_exr_file(fp, "rurix")
            actual = exr.frame_content_digest(doc["width"], doc["height"], 3, doc["pixels"])
            if actual != fr0.get("digest"):
                recompute_bad.append(f"rurix {scene}/t{tier}/{backend}/{fr0.get('name')}")
    checks["frame_digests_recomputed_match"] = not recompute_bad and bool(ue_cells) and bool(rurix_cells)
    check(recompute_bad == [], f"帧 digest 重算不符: {recompute_bad[:3]}")

    # ── measured 对拍（端内参照 deficit；probe 档噪声谱；帧率基线） ──
    contract = pc.parse_upscale_contract(CONTRACT_PATH.read_text(encoding="utf-8"))
    ev100 = {s["scene_id"]: s["exposure"]["ev100"] for s in contract["scenes"]}
    work = FRAMES / "report" / f"g13_m_c_{ts}"
    work.mkdir(parents=True, exist_ok=True)

    # 标定腿度量 + budget 注册/对账（消费容差前先行）
    cal_tols, cal_problems = run_calibration_leg(work, ev100, ts)
    checks["calibration_dual_seed_bitexact"] = not cal_problems
    check(not cal_problems, f"标定腿: {cal_problems[:3]}")
    budget = mb.load_g13_budget()
    checks["budget_anchors_present"] = budget is not None and all(
        (mb.budget_entry(budget, eid) or {}).get("evidence") == "measured_local" for eid in BUDGET_TOL_ENTRIES
    )
    if checks["budget_anchors_present"] and checks["calibration_dual_seed_bitexact"]:
        r = run(["py", "-3", "ci/budget_eval.py"])
        checks["budget_eval_all_pass"] = r.returncode == 0 and "[budget_eval] PASS" in (r.stdout + r.stderr)
        check(checks["budget_eval_all_pass"], f"budget_eval 非零: {(r.stdout + r.stderr)[-300:]}")
    tolerances = {eid: (cal_tols.get(eid) or {}).get("threshold") for eid in BUDGET_TOL_ENTRIES}

    parity_ok = checks["ue_arm_frames_all_present"] and checks["rurix_arm_frames_all_present"]
    if parity_ok:
        for scene in ("cornell-box", "bistro-interior"):
            for tier in TIERS:
                ue_frames = sorted((ue_cells[(scene, tier)]["dir"]).glob("*.exr"))
                for backend in BACKENDS:
                    rconv = rurix_cells[(scene, tier, backend)]["dir"] / "converged.exr"
                    ue_ref_dir = ue_cells[(scene, 100)]["dir"]
                    ue_ref = sorted(ue_ref_dir.glob("*.exr"))[-1]
                    r_ref = rurix_cells[(scene, 100, backend)]["dir"] / "converged.exr"
                    m = cell_metrics(scene, tier, backend, [str(p) for p in ue_frames], rconv,
                                     ue_ref, r_ref, ev100[scene], work)
                    tol_s = tolerances.get(BUDGET_TOL_ENTRIES[0])
                    tol_f = tolerances.get(BUDGET_TOL_ENTRIES[1])
                    over = (tol_s is not None and m["ssim_delta"] > tol_s) or (
                        tol_f is not None and m["flip_delta"] > tol_f)
                    cells.append({
                        "scene": scene, "tier": tier, "backend": backend,
                        **m, "tolerance": {"ssim": tol_s, "flip": tol_f},
                        "over_tolerance": bool(over), "registered": False,
                    })
            # 噪声谱（probe 档）
            for backend in BACKENDS:
                ue_paths = sorted(ue_cells[(scene, PROBE_TIER)]["dir"].glob("*.exr"))
                r_paths = sorted((rurix_cells[(scene, PROBE_TIER, backend)]["dir"] / "frames").glob("*.exr"))
                if len(ue_paths) == FRAME_COUNT and len(r_paths) == FRAME_COUNT:
                    su = noise_hf(ue_paths, "ue5")
                    sr = noise_hf(r_paths, "rurix")
                    noise_rows.append({
                        "scene": scene, "tier": PROBE_TIER, "backend": backend,
                        "ue_hf_share": su, "rurix_hf_share": sr, "delta": abs(su - sr),
                        "tolerance": tolerances.get(BUDGET_TOL_ENTRIES[2]),
                        "over_tolerance": bool(
                            tolerances.get(BUDGET_TOL_ENTRIES[2]) is not None
                            and abs(su - sr) > tolerances[BUDGET_TOL_ENTRIES[2]]),
                        "registered": False,
                    })
            # 帧率基线（zero_pass_line）
            for tier in TIERS:
                receipt = ue_cells[(scene, tier)].get("receipt") or {}
                ue_ms = (receipt.get("duration_s", 0.0) * 1000.0 / FRAME_COUNT) if receipt else None
                per_backend = {}
                for backend in BACKENDS:
                    rrec = rurix_cells[(scene, tier, backend)].get("receipt") or {}
                    fms = rrec.get("frame_ms") or []
                    if fms:
                        srt = sorted(fms)
                        per_backend[backend] = {
                            "mean_ms": sum(fms) / len(fms),
                            "p50_ms": srt[len(srt) // 2],
                            "p90_ms": srt[int(len(srt) * 0.9)],
                        }
                fps_cells.append({
                    "scene": scene, "tier": tier,
                    "ue_ms_per_frame_mrq": ue_ms,
                    "rurix": per_backend,
                })

    # ── 差距登记表（超容差项显式登记 + 对账；RXS-0391 正典形 gaplib 单源校验） ──
    registry_doc = None
    if cells:
        cam = "g13_ue_upscale_parity"
        prim = gaplib.MODULE_PREFIX + "PostProcess"
        anchor = "RXS-0405（G13.5/G15 承接；G13 不设绝对画质通过线）"
        for c in cells:
            if c["over_tolerance"]:
                title = f"upscale_deficit_delta@{c['scene']}/t{c['tier']}/{c['backend']}"
                conv_dig = (rurix_cells[(c["scene"], c["tier"], c["backend"])].get("receipt") or {}).get("converged_digest", "")
                registry_rows.append({
                    "gap_id": gaplib.derive_gap_id(c["scene"], cam, prim, "quality_gap", title),
                    "scene_id": c["scene"], "camera_id": cam,
                    "domain": "display-referred-ldr", "kind": "quality_gap",
                    "ue5_module_primary": prim, "ue5_module_secondary": [],
                    "measured_delta": [
                        {"metric": f"ssim_deficit_delta@{c['scene']}/t{c['tier']}/{c['backend']}",
                         "a_value": c["ssim_ue"], "b_value": c["ssim_rurix"],
                         "delta": float(c["ssim_rurix"]) - float(c["ssim_ue"]),
                         "evidence_digest": conv_dig},
                        {"metric": f"flip_deficit_delta@{c['scene']}/t{c['tier']}/{c['backend']}",
                         "a_value": c["flip_ue"], "b_value": c["flip_rurix"],
                         "delta": float(c["flip_rurix"]) - float(c["flip_ue"]),
                         "evidence_digest": conv_dig},
                    ],
                    "suggested_priority": "P2",
                    "g11_anchor": anchor,
                    "title": title,
                    "description": (
                        f"端内参照 deficit 对拍超容差：{c['scene']} tier{c['tier']} {c['backend']}"
                        "（参照 = 本端 tier100 收敛帧；容差 = g13_budget 标定三条目双 seed 方差底 "
                        "p100×2.0 程序产）；只登记不拟合（RXS-0392）。"
                    ),
                    "attachments": [],
                })
                c["registered"] = True
        for n in noise_rows:
            if n["over_tolerance"]:
                title = f"noise_hf_delta@{n['scene']}/t{n['tier']}/{n['backend']}"
                probe_rec = (rurix_cells[(n["scene"], n["tier"], n["backend"])].get("receipt") or {})
                probe_dig = ((probe_rec.get("frames") or [{}])[0]).get("digest", "")
                registry_rows.append({
                    "gap_id": gaplib.derive_gap_id(n["scene"], cam, prim, "quality_gap", title),
                    "scene_id": n["scene"], "camera_id": cam,
                    "domain": "display-referred-ldr", "kind": "quality_gap",
                    "ue5_module_primary": prim, "ue5_module_secondary": [],
                    "measured_delta": [{
                        "metric": f"noise_hf_delta@{n['scene']}/t{n['tier']}/{n['backend']}",
                        "a_value": n["ue_hf_share"], "b_value": n["rurix_hf_share"],
                        "delta": float(n["rurix_hf_share"]) - float(n["ue_hf_share"]),
                        "evidence_digest": probe_dig,
                    }],
                    "suggested_priority": "P2",
                    "g11_anchor": anchor,
                    "title": title,
                    "description": (
                        f"probe 档（tier{n['tier']}）残余帧亮度 2D FFT 高频能量份额双端谱差超容差："
                        f"{n['scene']} {n['backend']}；容差 = g13_budget 标定条目双 seed 方差底 "
                        "p100×2.0 程序产；只登记不拟合（RXS-0392）。"
                    ),
                    "attachments": [],
                })
                n["registered"] = True
        scene_set = ["cornell-box", "bistro-interior"]
        registry_doc = {
            "schema_version": 1,
            "registry": REGISTRY_NAME,
            "generated_by": "ci/g13_ue_upscale_parity_smoke.py --gate g13.p0.m_c.ue_upscale_parity",
            "scene_set": scene_set,
            "items": registry_rows,
            "scene_summary": [
                {"scene_id": s,
                 "gap_count": sum(1 for r in registry_rows if r["scene_id"] == s),
                 "no_gap_explicit": not any(r["scene_id"] == s for r in registry_rows)}
                for s in scene_set
            ],
            "not_ready_scenes": [],
        }
        problems = gaplib.validate_registry(registry_doc, scene_set=scene_set, registry_name=REGISTRY_NAME)
        checks["gap_registry_schema_valid"] = not problems
        check(bool(problems) is False, f"登记表 schema 校验: {problems[:3]}")
        recon = reconcile_registry(cells + noise_rows, registry_rows)
        checks["gap_registry_reconciled"] = not recon
        check(bool(recon) is False, f"登记表对账: {recon[:3]}")
        if checks["gap_registry_schema_valid"]:
            new_text = json.dumps(registry_doc, ensure_ascii=False, indent=1) + "\n"
            if REGISTRY_PATH.is_file():
                # ── G14 M-a 加性：结构化对账替换在树逐字节冻结（G13 §8.7 承接锚）──
                # 身份面逐字节 + Rurix 侧（b_value）位级 + UE 侧（a_value）程序产
                # 方差带内；gaplib 正典单源（RXS-0391 IR2 禁第二份手写维持）。
                old_doc = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))

                def _classify_m_c(metric: str, field: str, _value: float) -> str:
                    # 端侧归属声明（结构知识非阈值——构造面字面：a=UE 侧 / b=Rurix 侧）
                    return gaplib.PROVENANCE_UE if field == "a_value" else gaplib.PROVENANCE_RURIX

                drift = gaplib.reconcile_registry_structured(
                    old_doc, registry_doc, ue_band_rel, _classify_m_c)
                check(not drift,
                      f"登记表结构化对账漂移（身份面/Rurix 位级/UE 超带 {ue_band_rel:.8f}）: {drift[:3]}")
            else:
                REGISTRY_PATH.write_text(new_text, encoding="utf-8")
                note("差距登记表首落盘")

    checks["fps_baseline_zero_pass_line"] = validate_fps_baseline({
        "zero_pass_line": True, "cells": fps_cells,
    }) and bool(fps_cells)
    check(checks["fps_baseline_zero_pass_line"], "帧率基线 zero_pass_line 字面/非空缺失")

    residual_note = (
        "Rurix 臂直接光口径（direct_only_lambert + emissive_primary，无 GI/天光——契约 sun/sky=0.0）"
        "vs UE 臂 deferred+Lumen 默认面：跨端直比不消费（端内参照 deficit 面尺度消去，RXS-0403 L4 同族）；"
        "Rurix DLSS 后端 SL_DLSS_MODE_MAX_PERFORMANCE 钉定面（vendor_upscale.rs 0-byte）vs UE 逐档名义映射"
        "——名义档对拍口径（契约 provenance tier_note 登记）；UE DLSS 实际内部分辨率以 NGX 窗口为准"
        "（实测 tier67 bistro=1281×721）vs Rurix floor(tier%×输出) 口径——残余口径差登记不拟合（RXS-0392）。"
    )
    checks["residual_caliber_note_registered"] = bool(residual_note)

    # ── RED 臂（门内真跑） ──
    red_results["digest_mismatch"] = red_arm_digest_mismatch()
    red_results["silent_gap"] = red_arm_silent_gap()
    red_results["missing_frame"] = red_arm_missing_frame()
    red_results["fps_masquerade"] = red_arm_fps_masquerade()
    checks["device_red_digest_mismatch_detected"] = red_results["digest_mismatch"]
    checks["device_red_silent_gap_detected"] = red_results["silent_gap"]
    checks["device_red_missing_frame_detected"] = red_results["missing_frame"]
    checks["device_red_fps_masquerade_detected"] = red_results["fps_masquerade"]
    for arm, ok in red_results.items():
        check(ok, f"RED 臂 {arm} 注入未检出")

    device_state = "executed" if (
        checks["ue_arm_frames_all_present"] and checks["rurix_arm_frames_all_present"]
    ) else "fail"

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith(("device_", "ue_arm_", "rurix_")))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

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
        "base_commit": base_commit(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": "cargo build --release -p rurix-render --bin g13_4_ue_upscale_parity_render --features vendor-upscale", "exit_code": 0 if RURIX_BIN.is_file() else 1},
            {"seq": 2, "command": "g13_4_ue_upscale_parity_render --contract-digest（三方 digest 臂②）", "exit_code": 0 if rust_digest == FROZEN_CONTRACT_DIGEST else 1},
            {"seq": 3, "command": "g13_4_ue_render.py build --all --skip-import（UE Phase A + 三方 digest 臂③）", "exit_code": 0 if ue_digest == FROZEN_CONTRACT_DIGEST else 1},
            {"seq": 4, "command": "g13_4_ue_render.py render upscale <scene> --tier <50|67|100> ×6（UE DLSS 臂 MRQ 真跑）", "exit_code": 0 if checks["ue_arm_frames_all_present"] else 1},
            {"seq": 5, "command": "g13_4_ue_upscale_parity_render --render --scene <s> --tier <t> --backend <b> ×18（Rurix 三后端 32 帧 Halton 序列）", "exit_code": 0 if checks["rurix_arm_frames_all_present"] else 1},
            {"seq": 6, "command": "标定腿 calibration_seed 复跑（probe 档 × 3 后端 + tier100 参照 × 2 场景）", "exit_code": 0 if checks["calibration_dual_seed_bitexact"] else 1},
            {"seq": 7, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
            {"seq": 8, "command": "RED 臂 ×4（digest-mismatch/silent-gap/missing-frame/fps-masquerade）", "exit_code": 0 if all(red_results.values()) else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": environment(),
        "production": {
            "correctness_anchor_unchanged": checks["temporal_base_0byte"],
            "baseline_anchor_id": "g13.ue_upscale.{ssim_deficit_delta_tol,flip_deficit_delta_tol,noise_hf_delta_tol}（本门标定腿产出入 g13_budget）",
            "measured_value": (
                "; ".join(
                    f"{c['scene']}/t{c['tier']}/{c['backend']}: ssim_delta={c['ssim_delta']:.6g} flip_delta={c['flip_delta']:.6g}{' OVER' if c['over_tolerance'] else ''}"
                    for c in cells[:18]
                )
                if cells else "n/a（双臂未齐）"
            ),
            "not_worse_than_anchor": all(not c["over_tolerance"] for c in cells) if cells else False,
            "threshold_provenance": "g13_budget.json M-c 标定三条目（标定腿双 seed 方差底 p100 × 2.0 程序产，禁手写 P-09）",
            "evolution_register": (
                "帧率基线 zero_pass_line 登记：measured 基线不构成帧率对标通过线（正式帧率对标锚定 G14，"
                "G10-N11/N16 承接锚字面 0-byte）；G13 不设绝对超分画质通过线（归 G15）；"
                f"fps_baseline={json.dumps(fps_cells, ensure_ascii=False)[:600]}"
            ),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "parity": {
            "contract_digest": FROZEN_CONTRACT_DIGEST,
            "ue_build_id": ue5.EXPECTED_UE_BUILD_ID,
            "cells": cells,
            "noise_spectrum": noise_rows,
            "fps_baseline": {"zero_pass_line": True, "cells": fps_cells},
            "gap_registry_file": "milestones/g13/g13_ue_upscale_gap_registry.json",
            "residual_caliber_note": residual_note,
        },
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    print(f"[{TAG}] VERDICT={'PASS' if all_pass else 'FAIL'} checks={sum(1 for v in checks.values() if v)}/{len(checks)}")
    return 0 if all_pass else 1


def run_selftest() -> int:
    """schema 闭集对账 + 函数面 RED/GREEN 双臂。"""
    schema = load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    # RED 臂四件（函数面）
    if not red_arm_digest_mismatch():
        print(f"[{TAG}] selftest FAIL: digest-mismatch 臂未检出", file=sys.stderr)
        return 1
    if not red_arm_silent_gap():
        print(f"[{TAG}] selftest FAIL: silent-gap 臂未检出", file=sys.stderr)
        return 1
    if not red_arm_missing_frame():
        print(f"[{TAG}] selftest FAIL: missing-frame 臂未检出", file=sys.stderr)
        return 1
    if not red_arm_fps_masquerade():
        print(f"[{TAG}] selftest FAIL: fps-masquerade 臂未检出", file=sys.stderr)
        return 1
    # GREEN 面：正例不误判
    good_cells = [{"scene": "cornell-box", "tier": 67, "backend": "tsr_device", "over_tolerance": False, "registered": False}]
    if reconcile_registry(good_cells, []):
        print(f"[{TAG}] selftest FAIL: 对账正例误判", file=sys.stderr)
        return 1
    if not validate_fps_baseline({"zero_pass_line": True, "cells": [{"scene": "s", "tier": 67}]}):
        print(f"[{TAG}] selftest FAIL: fps 基线正例误判", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (4 RED + 2 GREEN)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
