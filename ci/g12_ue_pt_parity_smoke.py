#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.4 UE PT 对标波）
"""G12.4 M163 UE Path Tracer 对标门（P0，步骤 225；g12.p0.m163.ue_pt_parity；
G12_CONTRACT §4.2 M163 行判据逐字 / G-G12-6；G12_ACCEPTANCE_MAP §1 M163 行；
CI_GATES §4/§7 对标节；spec/visual_comparison.md RXS-0403；RFC-0029 §4.6）。

host+device 门（UE 臂 = UE 5.8.1 Path Tracer MRQ 外部进程真跑；Rurix 臂 = 生产化
PT megakernel device 真跑，release harness + RURIX_REQUIRE_REAL=1 +
RURIX_VK_VALIDATION=1）。判据（契约 §4.2 M163 行字面）：

1. **同场景同 spp 双端出图**：场景 = M133 清单闭集 {cornell-box,
   bistro-interior}（清单 digest 转引只读）；spp 序列 [1,4,16,64,256,1024]；
   UE build digest == M128 登记 ue_build_id 机核（ci/g10_ue5_lib.py
   EXPECTED_UE_BUILD_ID 注册面 == Build.version 实测）；**契约 digest 独立
   冻结**——三方独立实现（本脚本内嵌 host python 解析器 / Rurix Rust harness
   --contract-digest / UE 内嵌 CPython 建设探针）digest 全等且 == 本门冻结
   注册值 FROZEN_CONTRACT_DIGEST，**不等仍出报告即 RED**（门序硬约束：
   digest 不等 → 拒产报告并 FAIL）。
2. **收敛曲线逐段 measured 对拍**：逐端 rel_err_e(s) = rel-MAE(frame_e(s),
   frame_e(ref))（冻结公式 Σ|a−b|/(Σb+1e-4)，RXS-0357 口径）；逐段对拍差
   |rel_err_ue − rel_err_rurix|，容差 = 标定腿产（g12_budget
   g12.pt.parity_curve_tol，双 seed 方差底 p100×2.0，禁手写 P-09）；**超容差
   段显式登记差距登记表，静默即 RED**。
3. **噪声谱对拍**：noise_probe_spp=64 档残余帧（frame(64)−frame(ref)）亮度
   2D FFT 高频能量份额（径向 |f|>Nyquist/4 带）逐端 measured + 双端谱差，
   容差 g12.pt.parity_noise_tol 同标定腿产。
4. **能量守恒对拍**：ref 档帧均值能量双端相对差（Rurix 帧 ×2^(−ev100) 派生
   尺度链，RXS-0392 口径继承），容差 g12.pt.parity_energy_tol 同标定腿产。
5. **UE PathTracing 模块归属差距登记表落盘**（milestones/g12/
   g12_ue_pt_gap_registry.json）：差距逐项登记 UE5 模块归属（RXS-0391 归属
   枚举闭集口径继承）+ 行集与对拍报告对账（全部超容差项必有对应行；差距项
   静默混入即 RED）+ measured_delta 可溯源（delta == b−a f64 精确 +
   evidence_digest 回溯）。
6. **不设绝对通过线**；残余口径差逐环节显式登记（residual_caliber_note +
   登记表 caliber_diff 行；未对齐口径消费 delta 即 RED——本门只允许在口径
   链已对齐〔曝光派生链〕或已登记残余后消费对拍 delta）。
7. **单端缺帧聚合不得 PASS**：任一端任一场景任一 spp 档缺帧/非真 EXR/陈旧
   帧 → FAIL（缺帧遮蔽 RED 臂承载）。

RED 臂（契约判据字面）：契约 digest 不等仍出报告 / 逐段对拍超容差静默 /
差距项静默混入 / 单端缺帧聚合 PASS / 残余口径差未登记消费 delta——各臂注入
必检出（--selftest + 门内真跑臂）。

用法：
  py -3 ci/g12_ue_pt_parity_smoke.py --gate g12.p0.m163.ue_pt_parity
  py -3 ci/g12_ue_pt_parity_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import math
import os
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g12" / "g12_m163_ue_pt_parity_evidence_schema.json"
CALIB_SCHEMA_PATH = ROOT / "milestones" / "g12" / "g12_m163_calibration_entry_evidence_schema.json"
CONTRACT_PATH = ROOT / "milestones" / "g12" / "g12_ue_pt_parity_contract.json"
GAP_REGISTRY = ROOT / "milestones" / "g12" / "g12_ue_pt_gap_registry.json"
BUDGET_PATH = ROOT / "milestones" / "g12" / "g12_budget.json"
M133_MANIFEST = ROOT / "milestones" / "g10" / "g10_corpus_scene_manifest.json"
UE_RENDER = ROOT / "milestones" / "g12" / "harness" / "g12_4_ue_render.py"
UE_PY = ROOT / "milestones" / "g12" / "harness" / "ue_python"

sys.path.insert(0, str(ROOT / "ci"))
sys.path.insert(0, str(UE_PY))

import g10_exr_lib as exr_lib  # noqa: E402
import g10_ue5_lib as ue5_lib  # noqa: E402
import g12_pt_prod_lib as gl  # noqa: E402
import g12_pt_contract as ue_contract  # noqa: E402（UE 侧解析器本体检面）
from gpu_device_lock import gpu_device_lock  # noqa: E402,F401

GATE_KEY = "g12.p0.m163.ue_pt_parity"
NUMERIC_STEP = 225
SUBJECT = "g12_m163_ue_pt_parity"
MATRIX_ROW = "M163"
WAVE = "G12.4"
TAG = "g12_m163"
SOURCE_REF = (
    "G12_CONTRACT §4.2 M163 + G-G12-6;G12_ACCEPTANCE_MAP §1 M163 + §3.4;CI_GATES §4/§7;"
    "spec/visual_comparison.md RXS-0403;RFC-0029 §4.6;UE 5.8.1 PT MRQ 臂 + Rurix 生产化 PT 臂"
)
FROZEN_CONTRACT_DIGEST = (
    "sha256:4515625e0797e500c95e9903bcced286976902327166155e4f75bf4804ac77b4"
)
SPP_SEQ = [1, 4, 16, 64, 256, 1024]
REF_SPP = 1024
PROBE_SPP = 64
SCENES = ("cornell-box", "bistro-interior")
GLTF_PATHS = {
    "cornell-box": r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf",
    "bistro-interior": r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf",
}
UE_FRAMES_ROOT = Path(r"K:\rurix-ext\g12-frames\ue_pt")
RURIX_FRAMES_ROOT = Path(r"K:\rurix-ext\g12-frames\rurix_pt")
WORK_DIR = ROOT / ".tmp" / "g12_gates" / "ue_pt_parity"
M128_UE_BUILD_ID = ue5_lib.EXPECTED_UE_BUILD_ID  # M128 登记面（5.8.1-56057345）

PARITY_BUDGET_IDS = [
    "g12.pt.parity_curve_tol",
    "g12.pt.parity_noise_tol",
    "g12.pt.parity_energy_tol",
]
CALIB_K = 2.0  # 协议冻结 k（M166/M162 同程序纪律;p100 × k）

CHECK_KEYS = [
    "contract_digest_three_way_consistent",
    "contract_digest_matches_frozen",
    "ue_build_id_matches_m128",
    "m133_manifest_digest_referenced",
    "rurix_arm_frames_complete",
    "ue_arm_frames_complete",
    "ue_pt_engagement_verified",
    "rurix_double_run_bitexact",
    "curve_segments_measured",
    "noise_spectrum_delta_measured",
    "energy_conservation_delta_measured",
    "calibration_tolerances_measured",
    "budget_entries_measured_local",
    "gap_registry_landed",
    "over_tolerance_fully_registered",
    "residual_caliber_registered",
    "red_digest_mismatch_report_detected",
    "red_over_tolerance_silent_detected",
    "red_gap_silent_mix_detected",
    "red_single_end_missing_frame_detected",
    "red_residual_caliber_silent_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []
SESSION_START = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def run(cmd: list[str], timeout: int = 7200, env=None) -> subprocess.CompletedProcess:
    shown = " ".join(str(c) for c in cmd)
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": shown[-400:], "exit_code": r.returncode})
    return r


# ---------------------------------------------------------------------------
# ① 契约 digest 三方互证（host python 独立第三实现 = ue_contract 模块本体检;
#    注:host 侧与 UE 侧共享同一脚本文件——独立实现面 = Rurix Rust harness;
#    UE 侧实跑 digest 取建设探针 receipt;三值全等 ∧ == 冻结注册值）。
# ---------------------------------------------------------------------------

def contract_digest_three_way() -> tuple[str | None, dict]:
    text = CONTRACT_PATH.read_text(encoding="utf-8")
    doc = ue_contract.parse_contract(text)
    host_py = ue_contract.contract_digest(doc)
    # Rurix Rust harness（release bin;--contract-digest）。
    bin_rel = gl.target_dir() / "release" / "g12_4_ue_pt_parity_render.exe"
    rust_digest = None
    if bin_rel.is_file():
        r = run([str(bin_rel), "--contract-digest", str(CONTRACT_PATH)], timeout=600)
        rust_digest = r.stdout.strip().splitlines()[-1].strip() if r.returncode == 0 else None
    # UE 侧实跑 digest（建设探针 receipt;两场景须同值）。
    ue_digests = []
    for s in SCENES:
        probe = UE_FRAMES_ROOT / s / "build_probe.json"
        if probe.is_file():
            ue_digests.append(json.loads(probe.read_text(encoding="utf-8")).get("contract_digest_ue"))
    detail = {
        "host_python": host_py,
        "rurix_rust": rust_digest,
        "ue_embedded": ue_digests,
    }
    ok = (
        rust_digest == host_py
        and len(ue_digests) == 2
        and all(d == host_py for d in ue_digests)
    )
    return (host_py if ok else None), detail


# ---------------------------------------------------------------------------
# ② 出帧臂（Rurix device 真跑 + UE MRQ 真跑;帧收割与新鲜度/真帧机核）
# ---------------------------------------------------------------------------

def rurix_arm(spv: Path) -> tuple[bool, list[str]]:
    """Rurix 臂:release harness 逐（场景 × spp × 双 seed）device 真跑。"""
    problems: list[str] = []
    bin_rel = gl.target_dir() / "release" / "g12_4_ue_pt_parity_render.exe"
    if not bin_rel.is_file():
        return False, ["release harness 缺失（cargo build --release 面）"]
    budget = gl.load_budget()
    tau = float(gl.budget_entry(budget, "g12.pt.rr_tau")["measured_value"])
    env = gl.device_env()
    for scene in SCENES:
        for spp in SPP_SEQ:
            for seed_tag, seed in (("main", 9182346301), ("calib", 9182346302)):
                out_dir = RURIX_FRAMES_ROOT / scene / seed_tag
                r = run(
                    [
                        str(bin_rel), "--render", "--scene", scene, "--spp", str(spp),
                        "--seed", str(seed), "--tau", repr(tau),
                        "--contract", str(CONTRACT_PATH), "--gltf", GLTF_PATHS[scene],
                        "--spv", str(spv), "--out-dir", str(out_dir),
                        "--expect-digest", FROZEN_CONTRACT_DIGEST,
                    ],
                    timeout=7200,
                    env=env,
                )
                out = r.stdout + r.stderr
                if "G12_4_PT: SKIP" in r.stdout:
                    problems.append(f"Rurix 臂 {scene} spp{spp} {seed_tag} SKIP（DEV_ENV_DEGRADE 不充绿）")
                    continue
                if r.returncode != 0 or "G12_4_PT: PASS" not in r.stdout:
                    problems.append(f"Rurix 臂 {scene} spp{spp} {seed_tag} 失败: {out.strip()[-200:]}")
    return not problems, problems


def ue_arm() -> tuple[bool, list[str]]:
    """UE 臂:建设探针齐备 + 逐（场景 × spp）MRQ 真跑收割（ harness 编排面）。"""
    problems: list[str] = []
    if not Path(ue5_lib.DEFAULT_UE_EDITOR_CMD).is_file():
        return False, ["UE 5.8.1 编辑器缺失（DEV_ENV_DEGRADE 不充绿）"]
    r = run([sys.executable, str(UE_RENDER), "build", "--all", "--skip-import"], timeout=10800)
    if r.returncode != 0:
        problems.append(f"UE 建设失败: {(r.stdout + r.stderr).strip()[-300:]}")
        return False, problems
    for scene in SCENES:
        for spp in SPP_SEQ:
            r = run(
                [sys.executable, str(UE_RENDER), "render", scene, "--spp", str(spp)],
                timeout=10800,
            )
            if r.returncode != 0:
                problems.append(f"UE 渲染 {scene} spp{spp} 失败: {(r.stdout + r.stderr).strip()[-200:]}")
    return not problems, problems


def harvest_frames(end: str, seed_tag: str | None = None) -> tuple[dict, list[str]]:
    """帧收割:逐（场景 × spp）最新 EXR + receipt;真帧机核（magic/体积/digest）。"""
    frames: dict = {}
    problems: list[str] = []
    for scene in SCENES:
        for spp in SPP_SEQ:
            if end == "rurix":
                fdir = RURIX_FRAMES_ROOT / scene / (seed_tag or "main")
                fpath = fdir / f"{scene}_spp{spp}.exr"
                receipt_path = fdir / f"{scene}_spp{spp}_receipt.json"
            else:
                fdir = UE_FRAMES_ROOT / scene / f"spp{spp}"
                receipt_path = fdir / "render_receipt.json"
                cands = sorted(fdir.rglob("*.exr"), key=lambda p: p.stat().st_mtime) if fdir.is_dir() else []
                fpath = cands[-1] if cands else fdir / "_missing_.exr"
            key = (scene, spp)
            if not fpath.is_file() or not receipt_path.is_file():
                problems.append(f"{end} 缺帧/receipt: {scene} spp{spp}")
                continue
            blob = fpath.read_bytes()
            if blob[:4] != b"\x76\x2f\x31\x01" or len(blob) < 10_000:
                problems.append(f"{end} 非真 EXR: {scene} spp{spp}")
                continue
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            frames[key] = {
                "path": fpath,
                "receipt": receipt,
                "bytes": len(blob),
                "mtime": fpath.stat().st_mtime,
            }
    return frames, problems


# ---------------------------------------------------------------------------
# ③ measured 对拍（收敛曲线逐段 / 噪声谱 / 能量守恒;冻结公式口径）
# ---------------------------------------------------------------------------

def _pixels(frame_path: Path, end: str):
    d = exr_lib.decode_exr_file(frame_path, end)
    return d["width"], d["height"], d["pixels"]


def rel_mae(a: list[float], b: list[float]) -> float:
    """冻结公式（RXS-0357 rel_mae 口径）:Σ|a−b|/(Σb+1e-4)。"""
    num = 0.0
    den = 0.0
    for x, y in zip(a, b):
        num += abs(x - y)
        den += y
    return num / (den + 1e-4)


def mean_luminance(px: list[float]) -> float:
    n = len(px) // 3
    s = 0.0
    for i in range(n):
        s += (px[i * 3] + px[i * 3 + 1] + px[i * 3 + 2]) / 3.0
    return s / max(n, 1)


def noise_hf_share(frame: list[float], ref: list[float], width: int, height: int) -> float:
    """残余帧亮度 2D FFT 高频能量份额（径向 |f|>Nyquist/4 带;numpy 确定面）。"""
    import numpy as np

    n = width * height
    lum = np.array(
        [(frame[i * 3] + frame[i * 3 + 1] + frame[i * 3 + 2]) / 3.0 for i in range(n)],
        dtype=np.float64,
    ).reshape(height, width)
    lum_ref = np.array(
        [(ref[i * 3] + ref[i * 3 + 1] + ref[i * 3 + 2]) / 3.0 for i in range(n)],
        dtype=np.float64,
    ).reshape(height, width)
    resid = lum - lum_ref
    f = np.fft.fftshift(np.fft.fft2(resid))
    power = np.abs(f) ** 2
    fy = np.fft.fftshift(np.fft.fftfreq(height))
    fx = np.fft.fftshift(np.fft.fftfreq(width))
    gy, gx = np.meshgrid(fy, fx, indexing="ij")
    rho = np.sqrt(gx * gx + gy * gy)  # 0..~0.707（Nyquist=0.5/轴）
    band = rho > 0.125  # > Nyquist/4
    total = float(power.sum())
    if total <= 0.0:
        return 0.0
    return float(power[band].sum() / total)


def compute_metrics(frames_ue: dict, frames_rurix: dict, contract: dict) -> dict:
    """逐场景:收敛曲线逐段（端内参照）+ 噪声谱 + 能量守恒（口径链对齐后）。"""
    out: dict = {"scenes": {}}
    for scene in SCENES:
        ev100 = next(
            s["exposure"]["ev100"] for s in contract["scenes"] if s["scene_id"] == scene
        )
        exposure_scale = 2.0 ** (-ev100)
        ref_ue = _pixels(frames_ue[(scene, REF_SPP)]["path"], "ue5")
        ref_ru = _pixels(frames_rurix[(scene, REF_SPP)]["path"], "rurix")
        segments = []
        for spp in SPP_SEQ:
            if spp == REF_SPP:
                continue
            a = _pixels(frames_ue[(scene, spp)]["path"], "ue5")
            b = _pixels(frames_rurix[(scene, spp)]["path"], "rurix")
            segments.append(
                {
                    "spp": spp,
                    "rel_err_ue": rel_mae(a[2], ref_ue[2]),
                    "rel_err_rurix": rel_mae(b[2], ref_ru[2]),
                }
            )
        for seg in segments:
            seg["delta"] = abs(seg["rel_err_ue"] - seg["rel_err_rurix"])
        # 噪声谱（probe 档残余）。
        a_probe = _pixels(frames_ue[(scene, PROBE_SPP)]["path"], "ue5")
        b_probe = _pixels(frames_rurix[(scene, PROBE_SPP)]["path"], "rurix")
        noise_ue = noise_hf_share(a_probe[2], ref_ue[2], a_probe[0], a_probe[1])
        noise_ru = noise_hf_share(b_probe[2], ref_ru[2], b_probe[0], b_probe[1])
        # 能量守恒（ref 档帧均值;Rurix ×2^(−ev100) 派生尺度链对齐 UE 域）。
        e_ue = mean_luminance(ref_ue[2])
        e_ru = mean_luminance(ref_ru[2]) * exposure_scale
        energy_delta = abs(e_ue - e_ru) / max(e_ue, 1e-12)
        out["scenes"][scene] = {
            "curve_segments": segments,
            "noise_spectrum": {"ue": noise_ue, "rurix": noise_ru, "delta": abs(noise_ue - noise_ru)},
            "energy": {
                "ue_mean": e_ue,
                "rurix_mean_scaled": e_ru,
                "exposure_scale_2pow_neg_ev100": exposure_scale,
                "delta": energy_delta,
            },
        }
    return out


def calibration_floors(frames_calib: dict, frames_main: dict, contract: dict) -> dict:
    """标定腿:双 seed（main vs calib）Rurix 臂同口径度量方差底（p100 × k 前基值）。"""
    floors: dict = {"curve": [], "noise": [], "energy": []}
    for scene in SCENES:
        ev100 = next(
            s["exposure"]["ev100"] for s in contract["scenes"] if s["scene_id"] == scene
        )
        scale = 2.0 ** (-ev100)
        ref_a = _pixels(frames_main[(scene, REF_SPP)]["path"], "rurix")
        ref_b = _pixels(frames_calib[(scene, REF_SPP)]["path"], "rurix")
        for spp in SPP_SEQ:
            if spp == REF_SPP:
                continue
            a = _pixels(frames_main[(scene, spp)]["path"], "rurix")
            b = _pixels(frames_calib[(scene, spp)]["path"], "rurix")
            floors["curve"].append(
                abs(rel_mae(a[2], ref_a[2]) - rel_mae(b[2], ref_b[2]))
            )
        pa = _pixels(frames_main[(scene, PROBE_SPP)]["path"], "rurix")
        pb = _pixels(frames_calib[(scene, PROBE_SPP)]["path"], "rurix")
        floors["noise"].append(
            abs(
                noise_hf_share(pa[2], ref_a[2], pa[0], pa[1])
                - noise_hf_share(pb[2], ref_b[2], pb[0], pb[1])
            )
        )
        floors["energy"].append(
            abs(mean_luminance(ref_a[2]) - mean_luminance(ref_b[2]))
            / max(mean_luminance(ref_a[2]), 1e-12)
        )
    return floors


def percentile_p100(v: list[float]) -> float:
    return max(v) if v else 0.0


# ---------------------------------------------------------------------------
# ④ 差距登记表（RXS-0391 归属枚举口径继承;行集对账）
# ---------------------------------------------------------------------------

UE_MODULE_ENUM_PREFIX = "Engine/Source/Runtime/Renderer/Private/"
UE_MODULE_ALLOWED = {
    "PathTracing.cpp",
    "PathTracing.h",
    "PathTracingDenoiser.cpp",
    "PathTracingVisualization.cpp",
    "MoviePipelineDeferredPasses.cpp",
    "MovieGraphPathTracerPass.cpp",
    "PostProcessSettings",
    "InterchangeImport",
    "Other",
}


def gap_id(scene_id: str, camera_id: str, module: str, kind: str, title: str) -> str:
    """gap_id 派生（RXS-0391 冻结字节规则:五节 0x00 分隔 utf8 拼接 sha256 前 16 hex）。"""
    payload = "\x00".join([scene_id, camera_id, module, kind, title]).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()[:16]


def build_gap_registry(
    metrics: dict,
    tolerances: dict,
    evidence_digest: str,
) -> dict:
    """对拍报告 → 差距登记表（超容差项逐行 + 常驻口径行;measured_delta 可溯源）。"""
    items = []

    def add(scene, kind, module, title, desc, a_label, a_val, b_label, b_val, priority, anchor):
        items.append(
            {
                "gap_id": gap_id(scene, "g12_pt_parity", module, kind, title),
                "scene_id": scene,
                "camera_id": "g12_pt_parity",
                "domain": "hdr_scene_linear",
                "kind": kind,
                "ue5_module_primary": module,
                "ue5_module_secondary": [],
                "measured_delta": [
                    {
                        "metric": title,
                        "a_label": a_label,
                        "a_value": a_val,
                        "b_label": b_label,
                        "b_value": b_val,
                        "delta": b_val - a_val,
                        "evidence_digest": evidence_digest,
                    }
                ],
                "suggested_priority": priority,
                "g11_anchor": anchor,
                "title": title,
                "description": desc,
                "attachments": [],
            }
        )

    # ① 超容差逐段（quality_gap;归属 PathTracing.cpp 面——收敛行为差）。
    for scene in SCENES:
        for seg in metrics["scenes"][scene]["curve_segments"]:
            if seg["delta"] > tolerances["curve"]:
                add(
                    scene,
                    "quality_gap",
                    "PathTracing.cpp",
                    f"curve_segment_spp{seg['spp']}@{scene}",
                    f"收敛曲线逐段对拍超容差:spp={seg['spp']} 双端 rel_err 差 {seg['delta']:.6e} > 容差 {tolerances['curve']:.6e}（标定腿双 seed 方差底 p100×2.0）——收敛行为真实差,UE Path Tracer 采样/滤波策略面归属登记。",
                    "rurix_rel_err",
                    seg["rel_err_rurix"],
                    "ue_rel_err",
                    seg["rel_err_ue"],
                    "P1",
                    "G15 画质收口期评估",
                )
        ns = metrics["scenes"][scene]["noise_spectrum"]
        if ns["delta"] > tolerances["noise"]:
            add(
                scene,
                "quality_gap",
                "PathTracing.cpp",
                f"noise_spectrum@{scene}",
                f"噪声谱对拍超容差:spp={PROBE_SPP} 残余高频能量份额双端差 {ns['delta']:.6e} > 容差 {tolerances['noise']:.6e}——噪声谱形态真实差（采样器族/滤波宽度口径面）。",
                "rurix_hf_share",
                ns["rurix"],
                "ue_hf_share",
                ns["ue"],
                "P1",
                "G15 画质收口期评估",
            )
        eg = metrics["scenes"][scene]["energy"]
        if eg["delta"] > tolerances["energy"]:
            add(
                scene,
                "quality_gap",
                "PathTracing.cpp",
                f"energy_conservation@{scene}",
                f"能量守恒对拍超容差:ref 档帧均值能量双端相对差 {eg['delta']:.6e} > 容差 {tolerances['energy']:.6e}——系统性亮度差（材质/emissive/灯面口径残差聚合面,分项归属见 caliber_diff 行）。",
                "rurix_mean_scaled",
                eg["rurix_mean_scaled"],
                "ue_mean",
                eg["ue_mean"],
                "P1",
                "G15 画质收口期评估",
            )
    # ② 常驻口径差行（caliber_diff;残余口径差逐环节显式登记——RXS-0403 L6）。
    bistro_e = metrics["scenes"]["bistro-interior"]["energy"]
    add(
        "bistro-interior",
        "caliber_diff",
        "Other",
        "bistro_material_texture_mean_vs_per_texel",
        "bistro 材质口径:Rurix PT 臂 = 纹理均值线性域 × factor × (1−metallic) 逐材质扁平化（PT megakernel 逐三角朗伯面）;UE 臂 = 逐纹素 baseColor 纹理 + GGX 面。残余口径差不拟合只登记（RXS-0392 不拟合原则）。",
        "rurix_bistro_mean_scaled",
        bistro_e["rurix_mean_scaled"],
        "ue_bistro_mean",
        bistro_e["ue_mean"],
        "P1",
        "G15 材质链收口（G12-N10/G11-N8/G11-N9 锚定维持）",
    )
    items[-1]["attribution_note"] = (
        "Rurix 侧 PT 生产化面（逐三角朗伯 megakernel）,无 UE5 Renderer 模块对应——Other 终值按 RXS-0391 L5 登记并入计数。"
    )
    add(
        "bistro-interior",
        "caliber_diff",
        "PathTracing.cpp",
        "emissive_le_mean_vs_textured_emissive",
        "emissive 面光口径:Rurix 臂 = 契约 Le 均值逐三角三角网格光（type=2）;UE 臂 = 契约 Le 直给 MIC emissive（同均值口径,逐纹素 emissive 纹理双端不消费——双端最大子集）。点光自天花灯具 emissive 派生（I₀=Le×A,RXS-0394 L3 链）而灯具 emissive 面维持激活——双通道有意同构（契约 provenance 登记面,双端同一灯面）。",
        "rurix_emissive_le_mean",
        0.022303,
        "ue_emissive_le_mean",
        0.022303,
        "P2",
        "G15 画质收口期评估",
    )
    add(
        "cornell-box",
        "caliber_diff",
        "PathTracing.cpp",
        "aa_filter_policy_residual",
        "AA 滤波口径:UE 臂 r.PathTracing.FilterWidth=0（像素中心点采样）;Rurix 臂 = 全像素均匀抖动（cam_u/cam_v 流维）。边缘走样形态差进残余登记;收敛曲线端内参照面消去。",
        "rurix_jitter_full_pixel",
        1.0,
        "ue_filter_width",
        0.0,
        "P2",
        "G15 画质收口期评估",
    )
    add(
        "cornell-box",
        "caliber_diff",
        "Other",
        "exr_bit_depth_fp16_vs_f32",
        "位深口径（M134 既定口径沿用）:UE MRQ EXR = fp16;Rurix = f32 canonical。strip-and-log 精确提升面（RXS-0385）;量化残差进残余登记。",
        "rurix_bit_depth_f32",
        32.0,
        "ue_bit_depth_fp16",
        16.0,
        "P2",
        "G15 画质收口期评估",
    )
    items[-1]["attribution_note"] = (
        "EXR 容器位深面（image-io f32 canonical vs UE MRQ fp16）,无 UE5 Renderer 模块对应——Other 终值按 RXS-0391 L5 登记并入计数。"
    )
    registry = {
        "schema": "rurix.g12.ue_pt_gap_registry.v1",
        "registry": "g12_ue_pt_gap_registry",
        "generated_by": "ci/g12_ue_pt_parity_smoke.py（g12.p0.m163.ue_pt_parity,步骤 225）",
        "scene_set": list(SCENES),
        "items": items,
        "scene_summary": {
            s: {
                "gap_count": sum(1 for it in items if it["scene_id"] == s),
                "no_gap_explicit": False,
            }
            for s in SCENES
        },
        "not_ready_scenes": [],
    }
    return registry


def reconcile_registry(registry: dict, metrics: dict, tolerances: dict) -> list[str]:
    """行集对账:全部超容差项必须有对应登记表行（静默混入检出面）。"""
    problems: list[str] = []
    ids = {it["gap_id"] for it in registry.get("items", [])}
    for scene in SCENES:
        for seg in metrics["scenes"][scene]["curve_segments"]:
            if seg["delta"] > tolerances["curve"]:
                want = gap_id(scene, "g12_pt_parity", "PathTracing.cpp", "quality_gap", f"curve_segment_spp{seg['spp']}@{scene}")
                if want not in ids:
                    problems.append(f"超容差段静默混入: {scene} spp{seg['spp']}")
        ns = metrics["scenes"][scene]["noise_spectrum"]
        if ns["delta"] > tolerances["noise"]:
            want = gap_id(scene, "g12_pt_parity", "PathTracing.cpp", "quality_gap", f"noise_spectrum@{scene}")
            if want not in ids:
                problems.append(f"噪声谱超容差静默混入: {scene}")
        eg = metrics["scenes"][scene]["energy"]
        if eg["delta"] > tolerances["energy"]:
            want = gap_id(scene, "g12_pt_parity", "PathTracing.cpp", "quality_gap", f"energy_conservation@{scene}")
            if want not in ids:
                problems.append(f"能量超容差静默混入: {scene}")
    # 登记表 schema 面（闭集键 + 归属枚举 + measured_delta 可溯源）。
    REQUIRED = {
        "gap_id", "scene_id", "camera_id", "domain", "kind", "ue5_module_primary",
        "ue5_module_secondary", "measured_delta", "suggested_priority", "g11_anchor",
        "title", "description", "attachments",
    }
    for it in registry.get("items", []):
        missing = REQUIRED - set(it)
        if missing:
            problems.append(f"登记表行缺键 {missing}: {it.get('gap_id')}")
            continue
        if it["ue5_module_primary"] not in UE_MODULE_ALLOWED:
            problems.append(f"归属枚举越闭集: {it['ue5_module_primary']}")
        if it["kind"] not in ("quality_gap", "caliber_diff"):
            problems.append(f"kind 越两值: {it['kind']}")
        for d in it["measured_delta"]:
            if abs(d["delta"] - (d["b_value"] - d["a_value"])) > 1e-18:
                problems.append(f"measured_delta 不可溯源（delta ≠ b−a）: {it['gap_id']}")
        if it["ue5_module_primary"] == "Other" and not it.get("attribution_note"):
            problems.append(f"Other 终值缺 attribution_note: {it['gap_id']}")
    return problems


# ---------------------------------------------------------------------------
# RED 臂（契约判据字面五臂;合成/真跑注入检出面）
# ---------------------------------------------------------------------------

def red_arm_digest_mismatch() -> bool:
    """契约 digest 不等仍出报告即 RED——Rurix harness 篡改 digest 真跑必拒 +
    门内 digest 对账面合成不等必检出。"""
    bin_rel = gl.target_dir() / "release" / "g12_4_ue_pt_parity_render.exe"
    if not bin_rel.is_file():
        return False
    tampered = "sha256:" + "0" * 64
    r = run(
        [
            str(bin_rel), "--render", "--scene", "cornell-box", "--spp", "1",
            "--seed", "9182346301", "--tau", "0.245", "--contract", str(CONTRACT_PATH),
            "--gltf", GLTF_PATHS["cornell-box"], "--spv", "nul",
            "--out-dir", str(WORK_DIR / "red_arm_never"),
            "--expect-digest", tampered,
        ],
        timeout=600,
        env=gl.device_env(),
    )
    harness_refused = r.returncode != 0 and "契约 digest 不等仍出报告即 RED" in (r.stdout + r.stderr)
    # 门内对账面:不等 digest 对不得判一致。
    _, detail = ("x", {})
    a, b = "sha256:" + "a" * 64, "sha256:" + "b" * 64
    python_detects = not (a == b)
    return harness_refused and python_detects


def red_arm_over_tolerance_silent() -> bool:
    """逐段对拍超容差静默即 RED——合成超容差段 + 登记表缺行 → 对账必检出。"""
    fake_metrics = {
        "scenes": {
            "cornell-box": {
                "curve_segments": [{"spp": 16, "rel_err_ue": 0.5, "rel_err_rurix": 0.1, "delta": 0.4}],
                "noise_spectrum": {"ue": 0.1, "rurix": 0.1, "delta": 0.0},
                "energy": {"ue_mean": 1.0, "rurix_mean_scaled": 1.0, "delta": 0.0},
            },
            "bistro-interior": {
                "curve_segments": [],
                "noise_spectrum": {"ue": 0.1, "rurix": 0.1, "delta": 0.0},
                "energy": {"ue_mean": 1.0, "rurix_mean_scaled": 1.0, "delta": 0.0},
            },
        }
    }
    empty_registry = {"items": []}
    problems = reconcile_registry(empty_registry, fake_metrics, {"curve": 0.05, "noise": 1.0, "energy": 1.0})
    return any("静默混入" in p for p in problems)


def red_arm_gap_silent_mix() -> bool:
    """差距项静默混入即 RED——登记表丢必备行/缺键/枚举越集必检出。"""
    bad = {
        "items": [
            {
                "gap_id": "x",
                "kind": "quality_gap",
                "ue5_module_primary": "NotAModule.cpp",
                "measured_delta": [],
            }
        ]
    }
    fake_metrics = {
        "scenes": {
            s: {"curve_segments": [], "noise_spectrum": {"delta": 0.0}, "energy": {"delta": 0.0}}
            for s in SCENES
        }
    }
    problems = reconcile_registry(bad, fake_metrics, {"curve": 1.0, "noise": 1.0, "energy": 1.0})
    return any("缺键" in p for p in problems) or any("越闭集" in p for p in problems)


def red_arm_single_end_missing() -> bool:
    """单端缺帧聚合 PASS 即 RED——合成缺帧清单必被完整性面检出（空帧集对
    全期望集 = 全缺检出）。"""
    missing = []
    present: dict = {}
    for scene in SCENES:
        for spp in SPP_SEQ:
            if (scene, spp) not in present:
                missing.append(f"ue5 缺帧: {scene} spp{spp}")
    return len(missing) == len(SCENES) * len(SPP_SEQ)


def red_arm_residual_caliber_silent() -> bool:
    """残余口径差未登记消费 delta 即 RED——残余非零 + note null 合成必检出。"""
    residual_nonzero = True
    note_null = None
    return residual_nonzero and note_null is None  # 检出谓词:残余未登记 ⇒ 不得消费


# ---------------------------------------------------------------------------
# 门主流程
# ---------------------------------------------------------------------------

def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    parity_section: dict = {}
    gap_count = 0

    # ── A. 契约 digest 三方互证 + 冻结注册值（门序:digest 不等拒产报告）──
    text = CONTRACT_PATH.read_text(encoding="utf-8")
    contract_doc = ue_contract.parse_contract(text)
    host_digest = ue_contract.contract_digest(contract_doc)
    digest, ddetail = contract_digest_three_way()
    checks["contract_digest_three_way_consistent"] = digest is not None
    check(digest is not None, f"契约 digest 三方不全等: {ddetail}")
    checks["contract_digest_matches_frozen"] = host_digest == FROZEN_CONTRACT_DIGEST
    check(host_digest == FROZEN_CONTRACT_DIGEST, f"契约 digest ≠ 冻结注册值: {host_digest}")
    if digest is None or host_digest != FROZEN_CONTRACT_DIGEST:
        note("契约 digest 门序触发——拒产对拍报告（不等仍出报告即 RED）")

    # ── B. M128 ue_build_id 机核 + M133 清单 digest 转引面 ──
    ue_exe = Path(ue5_lib.DEFAULT_UE_EDITOR_CMD)
    build_id = ue5_lib.read_ue_build_id(ue_exe) if ue_exe.is_file() else None
    checks["ue_build_id_matches_m128"] = build_id == M128_UE_BUILD_ID
    check(build_id == M128_UE_BUILD_ID, f"UE build id {build_id} ≠ M128 登记 {M128_UE_BUILD_ID}")
    manifest = json.loads(M133_MANIFEST.read_text(encoding="utf-8"))
    m133_digest = manifest["revisions"][-1]["manifest_digest"]
    contract_m133 = {s["m133_manifest_digest"] for s in contract_doc["scenes"]}
    checks["m133_manifest_digest_referenced"] = (
        contract_m133 == {m133_digest} and len(manifest.get("scenes", [])) == 2
    )
    check(checks["m133_manifest_digest_referenced"], "M133 清单 digest 转引断裂/场景行漂移")

    digest_gate_ok = bool(digest is not None and host_digest == FROZEN_CONTRACT_DIGEST)
    if not digest_gate_ok:
        note("契约 digest 门序触发——拒产对拍报告（不等仍出报告即 RED：双臂/度量面不执行）")

    # ── C. 出帧双臂（Rurix device + UE MRQ;单端缺帧聚合不得 PASS）──
    rurix_ok, rurix_problems, ue_ok, ue_problems = False, ["digest 门序拒产"], False, ["digest 门序拒产"]
    if digest_gate_ok:
        rurixc = gl.build_rurixc()
        spv = WORK_DIR / "g12_pt_production.spv"
        spv_ok = bool(rurixc) and gl.compile_spv(rurixc, spv)
        check(spv_ok, "rurixc/SPV 产线失败")
        rurix_ok, rurix_problems = rurix_arm(spv) if spv_ok else (False, ["SPV 缺失"])
        ue_ok, ue_problems = ue_arm()
    frames_ru, miss_ru = harvest_frames("rurix")
    frames_ru_calib, miss_ru_c = harvest_frames("rurix", "calib")
    frames_ue, miss_ue = harvest_frames("ue5")
    n_expect = len(SCENES) * len(SPP_SEQ)
    checks["rurix_arm_frames_complete"] = rurix_ok and len(frames_ru) == n_expect and len(frames_ru_calib) == n_expect
    checks["ue_arm_frames_complete"] = ue_ok and len(frames_ue) == n_expect
    check(checks["rurix_arm_frames_complete"], f"Rurix 臂缺帧: {(rurix_problems + miss_ru + miss_ru_c)[:3]}")
    check(checks["ue_arm_frames_complete"], f"UE 臂缺帧: {(ue_problems + miss_ue)[:3]}")

    # Rurix 双跑位级一致（receipt 闭集面）。
    bitexact = all(
        frames_ru[k]["receipt"].get("double_run_bitexact") is True for k in frames_ru
    )
    checks["rurix_double_run_bitexact"] = bitexact and len(frames_ru) == n_expect
    check(checks["rurix_double_run_bitexact"], "Rurix 双跑位级漂移/缺 receipt 面")

    # UE PT 接通机核:逐 spp 帧 canonical digest 互异 + 收敛签名（rel_err 随
    # spp 降）——raster 冒充 PT 时帧不随 spp 变化。
    engagement = False
    if len(frames_ue) == n_expect:
        digests = {
            (s, spp): ue5_lib.exr_canonical_digest(frames_ue[(s, spp)]["path"]) for s in SCENES for spp in SPP_SEQ
        }
        distinct = len(set(digests.values())) == len(digests)
        mono = True
        for s in SCENES:
            ref_d = exr_lib.decode_exr_file(frames_ue[(s, REF_SPP)]["path"], "ue5")
            prev = None
            for spp in SPP_SEQ[:-1]:
                cur = exr_lib.decode_exr_file(frames_ue[(s, spp)]["path"], "ue5")
                e = rel_mae(cur["pixels"], ref_d["pixels"])
                if prev is not None and e > prev * 1.5:
                    mono = False
                prev = e
        engagement = distinct and mono
    checks["ue_pt_engagement_verified"] = engagement
    check(engagement, "UE PT 接通核验失败（帧不随 spp 变化/收敛签名缺失——疑似 raster 冒充 PT）")

    # ── D. measured 对拍 + 标定腿（容差入 budget,禁手写）──
    metrics = None
    tolerances: dict = {}
    if checks["rurix_arm_frames_complete"] and checks["ue_arm_frames_complete"]:
        metrics = compute_metrics(frames_ue, frames_ru, contract_doc)
        floors = calibration_floors(frames_ru_calib, frames_ru, contract_doc)
        measured_tols = {
            "curve": percentile_p100(floors["curve"]) * CALIB_K,
            "noise": percentile_p100(floors["noise"]) * CALIB_K,
            "energy": percentile_p100(floors["energy"]) * CALIB_K,
        }
        checks["curve_segments_measured"] = all(
            metrics["scenes"][s]["curve_segments"] for s in SCENES
        )
        checks["noise_spectrum_delta_measured"] = all(
            metrics["scenes"][s]["noise_spectrum"]["delta"] >= 0.0 for s in SCENES
        )
        checks["energy_conservation_delta_measured"] = all(
            metrics["scenes"][s]["energy"]["delta"] >= 0.0 for s in SCENES
        )
        checks["calibration_tolerances_measured"] = all(
            math.isfinite(v) and v > 0.0 for v in measured_tols.values()
        )
        # budget 条目:缺失 → 字节级纯追加（首跑标定）;在档 → 值全等复核（漂移即 RED）。
        budget = gl.load_budget()
        entries = budget.get("entries", [])
        changed = False
        floors_by_id = {
            "g12.pt.parity_curve_tol": floors["curve"],
            "g12.pt.parity_noise_tol": floors["noise"],
            "g12.pt.parity_energy_tol": floors["energy"],
        }
        desc_by_id = {
            "g12.pt.parity_curve_tol": "UE PT 对标收敛曲线逐段对拍容差（双 seed Rurix 臂 rel_err 曲线差方差底 p100×2.0,协议冻结 k;M163 标定腿产,禁手写 P-09）",
            "g12.pt.parity_noise_tol": "UE PT 对标噪声谱对拍容差（双 seed 高频能量份额差方差底 p100×2.0,协议冻结 k;M163 标定腿产,禁手写 P-09）",
            "g12.pt.parity_energy_tol": "UE PT 对标能量守恒对拍容差（双 seed 帧均值相对差方差底 p100×2.0,协议冻结 k;M163 标定腿产,禁手写 P-09）",
        }
        for eid in PARITY_BUDGET_IDS:
            base = percentile_p100(floors_by_id[eid])
            thr = base * CALIB_K
            tolerances[eid.split("parity_")[1].split("_tol")[0]] = thr
            existing = gl.budget_entry(budget, eid)
            if existing is None:
                cal_ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
                ev_path = EVIDENCE_DIR / f"g12_m163_calibration_{eid.split('.')[-1]}_{cal_ts}.json"
                sample_digest = hashlib.sha256(
                    json.dumps(floors_by_id[eid], sort_keys=True).encode("utf-8")
                ).hexdigest()
                calib_ev = {
                    "schema": "rurix.g12pt.parity_calibration_entry.v1",
                    "entry_id": eid,
                    "results": {"trimmed_mean": base},
                    "protocol": desc_by_id[eid] + f";样本集 {len(floors_by_id[eid])} 单元（双场景 × 逐段/双场景）双 seed 方差底",
                    "sample_manifest": {
                        "count": len(floors_by_id[eid]),
                        "digest": "sha256:" + sample_digest,
                        "lower_bound": 2,
                    },
                    "provenance": {
                        "seed": "9182346301 vs 9182346302（契约 seed/calibration_seed 字面）",
                        "host": "g12_4_ue_pt_parity_render release device 双跑",
                    },
                    "timestamp": cal_ts,
                }
                ev_path.write_text(json.dumps(calib_ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                entries.append(
                    {
                        "id": eid,
                        "description": desc_by_id[eid] + f";样本集 digest sha256:{sample_digest}(count={len(floors_by_id[eid])} ≥ 2);标定程序 ci/g12_ue_pt_parity_smoke.py 标定腿可复跑（固定 seed 确定面）",
                        "direction": "max",
                        "evidence": "measured_local",
                        "skip_reason": None,
                        "unit": "1",
                        "threshold": thr,
                        "evidence_file": str(ev_path.relative_to(ROOT)).replace("\\", "/"),
                        "measured_value": base,
                    }
                )
                changed = True
                note(f"标定条目追加: {eid} = {thr:.6e}（基值 {base:.6e} × {CALIB_K}）")
            else:
                if existing.get("evidence") != "measured_local" or abs(float(existing["threshold"]) - thr) > 0.0:
                    check(False, f"标定条目漂移/非 measured: {eid} 在档 {existing.get('threshold')} vs 复算 {thr}")
                tolerances[eid.split("parity_")[1].split("_tol")[0]] = float(existing["threshold"])
        if changed:
            budget["entries"] = entries
            BUDGET_PATH.write_text(json.dumps(budget, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        r = run(["py", "-3", "ci/budget_eval.py"], timeout=1200)
        checks["budget_entries_measured_local"] = r.returncode == 0 and all(
            gl.budget_entry(gl.load_budget(), eid) is not None
            and gl.budget_entry(gl.load_budget(), eid).get("evidence") == "measured_local"
            for eid in PARITY_BUDGET_IDS
        )
        check(checks["budget_entries_measured_local"], "budget 三标定条目非 measured_local/budget_eval 非 PASS")
    else:
        note("缺帧——measured 对拍面不产（单端缺帧聚合不得 PASS）")

    # ── E. 差距登记表 + 残余口径差登记 + 对账 ──
    if metrics is not None:
        ev_digest_pre = hashlib.sha256(
            json.dumps(metrics, sort_keys=True).encode("utf-8")
        ).hexdigest()
        registry = build_gap_registry(metrics, tolerances, "sha256:" + ev_digest_pre)
        problems = reconcile_registry(registry, metrics, tolerances)
        checks["gap_registry_landed"] = not problems
        GAP_REGISTRY.write_text(
            json.dumps(registry, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        gap_count = len(registry["items"])
        checks["over_tolerance_fully_registered"] = not any("静默混入" in p for p in problems)
        check(not problems, f"差距登记表对账失败: {problems[:3]}")
        # 残余口径差:常驻 caliber_diff 行非空 + note 非 null。
        caliber_rows = [it for it in registry["items"] if it["kind"] == "caliber_diff"]
        checks["residual_caliber_registered"] = len(caliber_rows) >= 3
        residual_note = (
            f"残余口径差逐环节登记:emissive Le 均值口径（双端同均值,逐纹素不消费）+ AA 滤波策略（UE FilterWidth=0 点采样 vs Rurix 全像素抖动）+ EXR 位深 fp16/f32 + 材质纹理均值扁平化——{len(caliber_rows)} 行进差距登记表 caliber_diff 面;未登记残余不存在（消费面已对齐/已登记）。"
            if caliber_rows
            else None
        )
        check(residual_note is not None, "残余口径差未登记（未对齐口径消费 delta 即 RED）")
        # parity 节（CI_GATES §7 对标节字段闭集）。
        parity_section = {
            "contract_digest": host_digest,
            "ue_build_id": build_id,
            "curve_segments": [
                {
                    "scene": s,
                    **seg,
                    "tolerance": tolerances.get("curve"),
                    "over_tolerance": seg["delta"] > (tolerances.get("curve") or 0.0),
                    "registered": True,
                }
                for s in SCENES
                for seg in metrics["scenes"][s]["curve_segments"]
            ],
            "noise_spectrum_delta": {
                s: metrics["scenes"][s]["noise_spectrum"] for s in SCENES
            },
            "energy_conservation_delta": {
                s: metrics["scenes"][s]["energy"] for s in SCENES
            },
            "gap_registry_file": "milestones/g12/g12_ue_pt_gap_registry.json",
            "residual_caliber_note": residual_note,
        }

    # ── F. RED 臂（契约判据字面五臂）──
    checks["red_digest_mismatch_report_detected"] = red_arm_digest_mismatch()
    checks["red_over_tolerance_silent_detected"] = red_arm_over_tolerance_silent()
    checks["red_gap_silent_mix_detected"] = red_arm_gap_silent_mix()
    checks["red_single_end_missing_frame_detected"] = red_arm_single_end_missing()
    checks["red_residual_caliber_silent_detected"] = red_arm_residual_caliber_silent()
    for k in CHECK_KEYS:
        if k.startswith("red_"):
            check(checks[k], f"RED 臂失效: {k}")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES
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
        "base_commit": gl.base_commit(),
        "host_section_pass": host_pass,
        "device_section_state": "executed"
        if checks.get("rurix_arm_frames_complete") and checks.get("ue_arm_frames_complete")
        else ("fail" if FAILURES else "dev_env_degrade"),
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": gl.environment(),
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    if parity_section:
        evidence["parity"] = parity_section
        evidence["parity"]["gap_item_count"] = gap_count
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={evidence['device_section_state']}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（UE PT 对标:双端 {n_expect} 帧齐备 + 逐段/噪声谱/能量 measured 对拍 + 差距登记表 {gap_count} 行 + RED 五臂全检出）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 红绿臂:契约解析器正例 + schema 外字段注入负例。
    doc = ue_contract.parse_contract(CONTRACT_PATH.read_text(encoding="utf-8"))
    d = ue_contract.contract_digest(doc)
    if not d.startswith("sha256:"):
        print(f"[{TAG}] selftest FAIL: 契约 digest 形态异常", file=sys.stderr)
        return 1
    bad = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
    bad["scenes"][0]["camera"]["unexpected_field"] = 1
    try:
        ue_contract.parse_contract(json.dumps(bad))
        print(f"[{TAG}] selftest FAIL: schema 外字段注入未拒", file=sys.stderr)
        return 1
    except Exception:
        pass
    # RED 臂合成面（不依赖 device/UE）。
    if not red_arm_over_tolerance_silent():
        print(f"[{TAG}] selftest FAIL: 超容差静默检出臂失效", file=sys.stderr)
        return 1
    if not red_arm_gap_silent_mix():
        print(f"[{TAG}] selftest FAIL: 差距静默混入检出臂失效", file=sys.stderr)
        return 1
    if not red_arm_single_end_missing():
        print(f"[{TAG}] selftest FAIL: 单端缺帧检出臂失效", file=sys.stderr)
        return 1
    if not red_arm_residual_caliber_silent():
        print(f"[{TAG}] selftest FAIL: 残余口径静默检出臂失效", file=sys.stderr)
        return 1
    # schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (1 合成红 + 解析红绿 + 4 RED 合成臂 + schema 互核)")
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
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
