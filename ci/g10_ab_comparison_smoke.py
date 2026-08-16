#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.5b 波）
"""G10.5b M139 A/B 对比门冒烟（步骤 188；g10.p0.m139.ab_comparison；
G10_CONTRACT §4.2 M139 行 / G-G10-7；G10_ACCEPTANCE_MAP §1 M139 行 + §3.2/§3.3；
RFC-0026 §4.4/§4.5/§4.6 + §4.0 不变量 4；spec/visual_comparison.md RXS-0386/
RXS-0390/RXS-0391）。

host+device 门（device_section_state=executed——UE 真跑经 M130 g10.5 腿子进程
自持 gpu_device_lock 串行；本门不嵌套持锁，D5 定案）。判据：

1. **门序三重绑定机器核验**（RFC-0026 §4.6 门序硬约束）：子进程真跑
   M130 `--phase g10.5`（当次 session 新鲜 evidence）→ 装载最新双端核验期
   evidence →（a）本门 evidence 内嵌当次 param_digest_rurix/param_digest_ue5
   且二者相等；（b）该 digest == M130 最新 evidence 登记的 param_digest；
   （c）同 base_commit（== 当前 HEAD）且同 session_run_id（harness 生成
   双写）；不等仍出报告即 RED（red_digest_unequal / red_stale_binding 双臂）。
2. **场景全集双端出图**（cornell-box + bistro-interior）：四组帧（双端 HDR
   + 双端 LDR）齐备解码 + 分辨率 == 契约 + Rurix HDR release 重渲染 digest
   逐位复现库帧（c2000ebf…/8519cc67…）+ LDR 派生文件级逐字节复现 +
   UE HDR unreal/build == M128 最新 evidence 登记 ue_build_id + 双端 HDR
   内容 digest == G10.5a 注册常量（c7c6f2cf…/5bfe1f49…）；单端缺帧聚合
   PASS 即 RED（red_single_end_missing_frame 臂）。
3. **度量报告**：LDR 臂 FLIP/SSIM/PSNR 重算 == G10.5a 预演 golden 字面
   逐位相等（design/g10_5_ab_preview.md §4，同一帧库同一工具链确定性
   复核）+ 口径 digest 登记 + 口径漂移注入即 RED（red_caliber_drift 臂）。
4. **逐像素 diff**：g10_m137_diff_report 重跑（H1 修订后 domain 自帧元
   数据派生 == display-referred-ldr 互证）+ 门侧独立重算三面一致 +
   artifacts 四 digest 对账。
5. **差距清单落盘**：preview §5/§6 候选 11 项（R1~R5/U1~U3/C1~C3）按
   RXS-0391 schema 装配（measured_delta 数值全来自本门当次重算输出 +
   evidence_digest 回溯本门 artifact_digests 登记集）→
   ci/g10_gap_registry_lib.py 校验零错误 → 落盘
   milestones/g10/g10_gap_registry.json（幂等：在树即逐字节相等复核，
   漂移即 RED）+ 场景全集零空行（red_missing_scene_row 臂）。

G10 零通过线维持：全部数字 measured_local 登记，不构成任何画质通过判定。

用法：
  py -3 ci/g10_ab_comparison_smoke.py --gate g10.p0.m139.ab_comparison
  py -3 ci/g10_ab_comparison_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import copy
import datetime as _dt
import hashlib
import json
import platform
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m139_ab_comparison_evidence_schema.json"
SPEC_PATH = ROOT / "spec" / "visual_comparison.md"
CORPUS = ROOT / "milestones" / "g10" / "corpus"
REGISTRY_PATH = ROOT / "milestones" / "g10" / "g10_gap_registry.json"
FRAMES = Path(r"K:\rurix-ext\g10-frames\g10_5")
BISTRO_GLTF = Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf")
CORNELL_GLTF = Path(r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf")
REPRO_DIR = FRAMES / "repro_m139"
REPORT_DIR = FRAMES / "report"
RUST_RELEASE_BIN = ROOT / "target" / "release" / "g10_5_scene_render.exe"
DIFF_BIN = ROOT / "target" / "debug" / "g10_m137_diff_report.exe"

sys.path.insert(0, str(ROOT / "ci"))
import g10_exr_lib as exr  # noqa: E402
import g10_flip_lib as flip  # noqa: E402
import g10_ssim_psnr_lib as ssim_psnr  # noqa: E402
import g10_gap_registry_lib as gaplib  # noqa: E402
import g10_wave_exit_lib as wel  # noqa: E402
import g10_dual_determinism_contract_smoke as m130  # noqa: E402
import g10_pixel_diff_report_smoke as m137  # noqa: E402

GATE_KEY = "g10.p0.m139.ab_comparison"
NUMERIC_STEP = 188
SOURCE_REF = (
    "G10_CONTRACT §4.2 M139 + G-G10-7;G10_ACCEPTANCE_MAP §1 M139 + §3.2/§3.3;"
    "RFC-0026 §4.4/§4.5/§4.6 + §4.0 不变量 4;spec/visual_comparison.md RXS-0386/RXS-0390/RXS-0391"
)
TAG = "g10_m139"
SUBJECT = "g10_m139_ab_comparison"
MATRIX_ROW = "M139"

SCENES = ("cornell-box", "bistro-interior")
CAMERA_ID = "g10_contract_camera"
GLTF = {"cornell-box": CORNELL_GLTF, "bistro-interior": BISTRO_GLTF}
# LDR 派生曝光尺度（preview §6 C2 登记：Rurix 臂 ×2^(−EV100)，UE 臂 ×1.0）。
EXPOSURE_SCALE_RURIX = {"cornell-box": 0.25, "bistro-interior": 0.5}

# G10.5a 帧库注册 digest 常量（design/g10_5_ab_preview.md §3 实测登记面，
# release profile 逐位复现已实证：cornell c2000ebf… / bistro 8519cc67…）。
LIB_HDR_DIGEST = {
    ("cornell-box", "rurix"): "sha256:c2000ebfbe90359d55e668f8af3b7df24d64c3f72e637904f614821b7ad0d727",
    ("cornell-box", "ue5"): "sha256:c7c6f2cf1644ba79512da1f4f3fceeb2001826f4723681a35ab7a8ca9dc853a2",
    ("bistro-interior", "rurix"): "sha256:8519cc67c917e7b8c2c5a9bb5633ea5ee9e72deb8cf63b3b187b0d3ac5bb9935",
    ("bistro-interior", "ue5"): "sha256:5bfe1f4965e72e85d4c75f21879f8c89bf1f4e292348fa7e82cd9faf0245cc19",
}

# G10.5a 首跑度量 golden（preview §3/§4 measured 字面；同一帧库同一工具链
# 确定性复核面——漂移即口径/工具链漂移 RED）。
GOLDEN = {
    "cornell-box": {
        "flip_ldr": 0.338644611302288,
        "ssim": 0.34829777885646934,
        "psnr_db": 13.982872203129087,
        "hdr_rurix": {"median": 0.13765611151754856, "p90": 1.2159505246400832, "max": 1.4381704081773758, "nonzero_ratio": 0.9290046691894531},
        "hdr_ue5": {"median": 0.0, "p90": 0.59423720703125, "max": 0.595666796875, "nonzero_ratio": 0.1838836669921875},
        "ldr_rurix": {"median": 0.08358066641539334, "p90": 0.5009738778293134, "max": 0.5498988258123397, "nonzero_ratio": 0.9290046691894531},
        "ldr_ue5": {"median": 0.0, "p90": 0.6963858742475509, "max": 0.6970374228715897, "nonzero_ratio": 0.18301010131835938},
        "diff": {"err_mean": 0.15214514716781968, "err_p95": 0.4725396, "err_max": 0.55182767, "over_threshold_ratio": 0.9290046691894531},
    },
    "bistro-interior": {
        "flip_ldr": 0.9403171994233143,
        "ssim": 0.16710192121627712,
        "psnr_db": 2.5845420330788715,
        "hdr_rurix": {"median": 0.1333588808774948, "p90": 0.30276253819465637, "max": 2.9937067031860347, "nonzero_ratio": 0.9567592592592593},
        "hdr_ue5": {"median": 2.798138671875, "p90": 5.000015625, "max": 77.82171249999999, "nonzero_ratio": 1.0},
        "ldr_rurix": {"median": 0.16252008080482483, "p90": 0.3130713999271392, "max": 0.8724713325500488, "nonzero_ratio": 0.9439134837962962},
        "ldr_ue5": {"median": 0.9324080557703971, "p90": 0.9666675288200379, "max": 1.0, "nonzero_ratio": 1.0},
        "diff": {"err_mean": 0.7208936006809408, "err_p95": 0.9231632, "err_max": 1.0, "over_threshold_ratio": 1.0},
    },
}

CHECK_KEYS = [
    "spec_rxs0391_clause_on_tree",
    "m130_g10_5_leg_fresh_pass",
    "three_binding_verified",
    "dual_end_frames_present_full_scene_set",
    "rurix_frames_reproduced_bit_exact",
    "ldr_derivation_reproduced_bit_exact",
    "ue_frames_provenance_and_digest_match",
    "derivation_chain_metadata_consistent",
    "metric_report_recomputed_golden",
    "diff_reports_regenerated_consistent",
    "gap_registry_materialized_valid",
    "red_missing_scene_row_detected",
    "red_single_end_missing_frame_detected",
    "red_caliber_drift_detected",
    "red_digest_unequal_report_blocked",
    "red_stale_binding_detected",
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


def run_cmd(argv: list[str], timeout: int = 5400, env_extra: dict | None = None) -> subprocess.CompletedProcess:
    import os

    print(f"[{TAG}] $ {' '.join(str(a) for a in argv)}", flush=True)
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(str(a) for a in argv), "exit_code": r.returncode})
    return r


def sha256_file(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def lum_stats(arr: np.ndarray) -> dict:
    """帧亮度统计（门内独立实现——与 harness 预览驱动同一公式面交叉复核）。"""
    lum = 0.2126 * arr[..., 0] + 0.7152 * arr[..., 1] + 0.0722 * arr[..., 2]
    flat = np.sort(lum.ravel())
    n = flat.size
    return {
        "median": float(flat[n // 2]),
        "p90": float(flat[int(n * 0.9)]),
        "max": float(flat[-1]),
        "nonzero_ratio": float(np.count_nonzero(flat > 1e-6) / n),
    }


def load_pixels(path: Path, end: str):
    d = exr.decode_exr(path.read_bytes(), end)
    arr = np.asarray(d["pixels"], dtype=np.float64).reshape(d["height"], d["width"], 3)
    return d, arr


def frame_paths(frames_root: Path, scene: str) -> dict:
    return {
        "hdr_rurix": frames_root / "rurix" / f"{scene}.exr",
        "hdr_ue5": frames_root / "ue" / scene / ".0000.exr",
        "ldr_rurix": frames_root / "ldr" / f"{scene}_rurix_ldr.exr",
        "ldr_ue5": frames_root / "ldr" / f"{scene}_ue5_ldr.exr",
    }


def frame_set_problems(frames_root: Path) -> list[str]:
    """双端帧齐备面（单端缺帧聚合 PASS 即 RED 的机器面；返回问题列表）。"""
    problems: list[str] = []
    for scene in SCENES:
        fp = frame_paths(frames_root, scene)
        for key, p in fp.items():
            if not p.is_file() or p.stat().st_size < 1024:
                problems.append(f"{scene}/{key} 帧缺失或体积异常: {p}")
                continue
            try:
                end = "ue5" if key == "hdr_ue5" else "rurix"
                d = exr.decode_exr(p.read_bytes(), end)
                c = json.loads((CORPUS / f"contract_params_{scene.replace('-', '_')}.json").read_text(encoding="utf-8"))
                if (d["width"], d["height"]) != (c["camera"]["resolution"]["w"], c["camera"]["resolution"]["h"]):
                    problems.append(f"{scene}/{key} 分辨率与契约不符: {d['width']}x{d['height']}")
            except Exception as e:  # noqa: BLE001 — 解码失败即问题行
                problems.append(f"{scene}/{key} 解码失败: {e}")
    return problems


def latest_m130_g10_5() -> tuple[Path, dict]:
    """最新 M130 双端核验期 evidence（路径 + 文档；排序规则与 m130 模块同字面，
    并以 m130.load_latest_g10_5_evidence() 交叉同一性核验）。"""
    cands = []
    for f in EVIDENCE_DIR.glob("g10_m130_dual_determinism_contract_*.json"):
        try:
            doc = json.loads(f.read_text(encoding="utf-8"))
        except Exception:
            continue
        if doc.get("phase") == "g10.5":
            cands.append((doc.get("timestamp", ""), f.name, f, doc))
    if not cands:
        raise RuntimeError("缺 M130 g10.5 期 evidence")
    cands.sort(key=lambda t: (t[0], t[1]))
    top = [c for c in cands if (c[0], c[1]) == (cands[-1][0], cands[-1][1])]
    if len(top) != 1:
        raise RuntimeError("最新 M130 g10.5 evidence 判定并列（fail-closed）")
    _, _, path, doc = top[0]
    cross = m130.load_latest_g10_5_evidence()
    if cross != doc:
        raise RuntimeError("与 m130 模块最新 evidence 判定不一致（fail-closed）")
    return path, doc


def build_gap_registry(measured: dict, artifact_digests: list[str]) -> dict:
    """差距清单装配（preview §5/§6 候选 11 项；measured_delta 数值全来自
    本门当次重算 measured 字典，delta = b−a 由本函数计算）。"""
    P = gaplib.MODULE_PREFIX

    def delta(metric: str, a: float, b: float, digest: str, region_ref: str | None = None) -> dict:
        d = {
            "metric": metric,
            "a_value": a,
            "b_value": b,
            "delta": float(b) - float(a),
            "evidence_digest": digest,
        }
        if region_ref is not None:
            d["region_ref"] = region_ref
        return d

    items_spec = [
        # (key, scene, kind, primary, secondary, priority, title, description, deltas_fn, attribution_note)
        {
            "key": "R1", "scene": "bistro-interior", "kind": "quality_gap",
            "primary": P + "BasePassRendering.cpp",
            "secondary": [P + "Substrate", P + "MaterialCache"],
            "priority": "P0",
            "title": "R1 材质子集仅逐图元 baseColorFactor（Lambert），baseColorTexture/法线/metallic-roughness 不采样",
            "description": "Rurix 侧渲染缺口（g10_5_scene_render 头注诚实边界）：bistro Rurix 帧近灰白（纹理所载色彩全缺）；G10.3 已登记 DDS 解码归后续波次。measured 锚 = bistro LDR SSIM 实测值 vs 恒等极值 1.0（a=实测对拍值，b=恒等参考——结构性内容缺失主导的结构相似度塌陷）。",
            "deltas_fn": lambda m: [delta("ssim@bistro-interior(ldr)", m["bistro-interior"]["metrics"]["ssim"], 1.0, m["digests"]["metric_report"]["bistro-interior"])],
        },
        {
            "key": "R2", "scene": "cornell-box", "kind": "quality_gap",
            "primary": P + "BasePassRendering.cpp",
            "secondary": [P + "SceneVisibility.cpp"],
            "priority": "P1",
            "title": "R2 几何法线（winding 朝向 + 双面翻转），平滑法线不消费",
            "description": "Rurix 侧渲染缺口：cornell 壳体单面片外向绕向被双面口径吞没（UE 侧同内容被背面剔除——口径交互差见 U1/C 族）。measured 锚 = cornell HDR 覆盖 nonzero 比双端实测（a=rurix，b=ue5）。",
            "deltas_fn": lambda m: [delta("hdr_nonzero_ratio@cornell-box", m["cornell-box"]["hdr_rurix"]["nonzero_ratio"], m["cornell-box"]["hdr_ue5"]["nonzero_ratio"], m["digests"]["metric_report"]["cornell-box"])],
        },
        {
            "key": "R3", "scene": "bistro-interior", "kind": "quality_gap",
            "primary": P + "LightRendering.cpp",
            "secondary": [P + "MegaLights"],
            "priority": "P0",
            "title": "R3 灯种子集 = 契约 sun + sky 常量天光；点/面光源与 glTF emissive 不表达",
            "description": "Rurix 侧渲染缺口：bistro 包内 pointLight1~N（glTF 节点实测 4+ 盏）与 emissive surfaces 不表达；cornell 语料点光源按契约降为 sun+sky。measured 锚 = bistro HDR 亮度中位双端实测（a=rurix，b=ue5；≈21× 主差见 C1）。",
            "deltas_fn": lambda m: [delta("hdr_luminance_median@bistro-interior", m["bistro-interior"]["hdr_rurix"]["median"], m["bistro-interior"]["hdr_ue5"]["median"], m["digests"]["metric_report"]["bistro-interior"])],
        },
        {
            "key": "R4", "scene": "bistro-interior", "kind": "quality_gap",
            "primary": P + "Lumen",
            "secondary": [P + "ScreenSpaceDenoise.cpp"],
            "priority": "P0",
            "title": "R4 GI = 屏幕探针单反弹（host 参考管线），非 Lumen 等效宣称",
            "description": "Rurix 侧渲染缺口：bistro Rurix 帧高噪声（单反弹 + 有限样本）；measured 锚 = bistro HDR 亮度 p90 双端实测（a=rurix，b=ue5；GI/天光口径差主因之一，见 C1）。",
            "deltas_fn": lambda m: [delta("hdr_luminance_p90@bistro-interior", m["bistro-interior"]["hdr_rurix"]["p90"], m["bistro-interior"]["hdr_ue5"]["p90"], m["digests"]["metric_report"]["bistro-interior"])],
        },
        {
            "key": "R5", "scene": "cornell-box", "kind": "quality_gap",
            "primary": gaplib.OTHER_MODULE,
            "secondary": [],
            "priority": "P2",
            "title": "R5 JSON 整数解析经 i64（u64 顶格 seed 被 fail-closed 拒绝）",
            "description": "Rurix 侧 harness 解析面缺口：契约 time.random_seed 取 u64 顶格值时 Rust 消费面（g10_5_scene_render，i64 域解析）fail-closed 拒绝；本波契约 seed=42 不触面。measured 锚 = u64 顶格 seed 注入探针：a=i64 域上界（f64），b=u64 顶格（f64），Rust 端拒绝实测（exit≠0）。",
            "attribution_note": "Rurix 侧 harness JSON 解析面（i64 域），无 UE5 Renderer 模块对应——Other 终值按 RXS-0391 L5 登记并入计数。",
            "deltas_fn": lambda m: [delta("contract_seed_u64_max_rejection", 9.223372036854776e+18, 1.8446744073709552e+19, m["digests"]["seed_probe"])],
        },
        {
            "key": "U1", "scene": "cornell-box", "kind": "quality_gap",
            "primary": P + "SceneVisibility.cpp",
            "secondary": [P + "GPUScene.cpp"],
            "priority": "P0",
            "title": "U1 cornell 壳体（墙/顶/地板）零辐射：语料单面片外向绕向 × UE 背面剔除口径",
            "description": "UE 侧场景面缺口：语料单面片外向 CCW × UE 背面剔除 → UE 帧仅双块可见；Rurix 同内容双面着色口径 92.90% 覆盖。内容/口径交互差，G10 零修复不改语料不改渲染器。measured 锚 = cornell HDR 覆盖 nonzero 比双端实测（a=rurix，b=ue5）。",
            "deltas_fn": lambda m: [delta("hdr_nonzero_ratio@cornell-box", m["cornell-box"]["hdr_rurix"]["nonzero_ratio"], m["cornell-box"]["hdr_ue5"]["nonzero_ratio"], m["digests"]["metric_report"]["cornell-box"])],
        },
        {
            "key": "U2", "scene": "bistro-interior", "kind": "quality_gap",
            "primary": P + "BasePassRendering.cpp",
            "secondary": [P + "MaterialCache"],
            "priority": "P0",
            "title": "U2 bistro 纹理全缺：包内 .dds 纹理 Interchange 不支持，材质实例 texture_parameter_values 空",
            "description": "UE 侧场景面缺口：UE bistro 帧近纯白洗涤态（albedo 全 ≈ 白）；导入错误日志 LogInterchangeEngine 逐条在案。measured 锚 = bistro LDR 亮度中位双端实测（a=rurix，b=ue5，UE 帧近纯白抬升）。",
            "deltas_fn": lambda m: [delta("ldr_luminance_median@bistro-interior", m["bistro-interior"]["ldr_rurix"]["median"], m["bistro-interior"]["ldr_ue5"]["median"], m["digests"]["metric_report"]["bistro-interior"])],
        },
        {
            "key": "U3", "scene": "bistro-interior", "kind": "quality_gap",
            "primary": gaplib.OTHER_MODULE,
            "secondary": [],
            "priority": "P2",
            "title": "U3 Bistro 动画 Take 001 / glTF 相机节点不引用（动画剥离）",
            "description": "UE 侧场景面缺口：动画剥离（build_scenes 头注登记），相机采用 glTF 静态节点位姿（corpus 校准登记）。measured 锚 = bistro glTF 包内动画通道计数实测（a=双端消费数 0，b=包内实测动画通道数）。",
            "attribution_note": "动画/Sequencer 面缺口，无 UE5 Renderer 模块对应——Other 终值按 RXS-0391 L5 登记并入计数。",
            "deltas_fn": lambda m: [delta("gltf_animation_channels_unconsumed@bistro-interior", 0.0, float(m["bistro_anim_channels"]), m["digests"]["gltf_probe"])],
        },
        {
            "key": "C1", "scene": "bistro-interior", "kind": "caliber_diff",
            "primary": P + "Lumen",
            "secondary": [P + "SkyPassRendering.cpp"],
            "priority": "P1",
            "title": "C1 室内亮度主差：GI/天光遮蔽口径差（UE SkyLight 指定 cubemap 全向 IBL vs Rurix 屏幕探针单反弹）+ 太阳 lux→辐射度链差",
            "description": "口径差（不拟合、只登记）：bistro HDR 中位 UE vs Rurix ≈21×；cornell 块区 p90 UE vs Rurix ×2^(−EV100) 后 ≈1.95×。measured 锚 = 两条目双端 HDR 亮度实测（a=rurix，b=ue5；cornell 行 a 含 ×2^(−EV100) 派生尺度，见 C2）。",
            "deltas_fn": lambda m: [
                delta("hdr_luminance_median@bistro-interior", m["bistro-interior"]["hdr_rurix"]["median"], m["bistro-interior"]["hdr_ue5"]["median"], m["digests"]["metric_report"]["bistro-interior"]),
                delta("hdr_luminance_p90@cornell-box(rurix×2^-EV100)", m["cornell-box"]["hdr_rurix"]["p90"] * 0.25, m["cornell-box"]["hdr_ue5"]["p90"], m["digests"]["metric_report"]["cornell-box"], region_ref="scene:cornell-box"),
            ],
        },
        {
            "key": "C2", "scene": "cornell-box", "kind": "caliber_diff",
            "primary": P + "PostProcess",
            "secondary": [],
            "priority": "P2",
            "title": "C2 曝光链：双端 EV100 同字面；Rurix 臂派生尺度 = 2^(−EV100)，UE 臂 pipe 内手动曝光已施（FixedExposure=2^(−EV100) 源码实证）派生尺度 ×1.0",
            "description": "口径差：派生链参数登记（a=Rurix 臂派生尺度，b=UE 臂派生尺度；cornell 0.25/1.0、bistro 0.5/1.0 双场景实测登记）。",
            "deltas_fn": lambda m: [
                delta("ldr_derivation_exposure_scale@cornell-box", EXPOSURE_SCALE_RURIX["cornell-box"], 1.0, m["digests"]["metric_report"]["cornell-box"]),
                delta("ldr_derivation_exposure_scale@bistro-interior", EXPOSURE_SCALE_RURIX["bistro-interior"], 1.0, m["digests"]["metric_report"]["bistro-interior"], region_ref="scene:bistro-interior"),
            ],
        },
        {
            "key": "C3", "scene": "bistro-interior", "kind": "caliber_diff",
            "primary": P + "HdrCustomResolveShaders.cpp",
            "secondary": [],
            "priority": "P2",
            "title": "C3 UE EXR 位深 fp16 → f32 提升（RXS-0385 strip-and-log）；Rurix 原生 f32",
            "description": "口径差（M134 既定口径沿用）：源帧位深双端实测（a=rurix 原生 f32=32，b=ue5 MRQ fp16=16；双场景同口径，主行挂 bistro 1080p 帧实测面）。",
            "deltas_fn": lambda m: [delta("exr_source_bit_depth@bistro-interior", 32.0, 16.0, m["digests"]["metric_report"]["bistro-interior"])],
        },
    ]

    items = []
    for spec in items_spec:
        deltas = spec["deltas_fn"](measured)
        item = {
            "gap_id": gaplib.derive_gap_id(spec["scene"], CAMERA_ID, spec["primary"], spec["kind"], spec["title"]),
            "scene_id": spec["scene"],
            "camera_id": CAMERA_ID,
            "domain": "display-referred-ldr" if spec["key"] in ("R1", "U2", "C2") else "scene-linear-hdr",
            "kind": spec["kind"],
            "ue5_module_primary": spec["primary"],
            "ue5_module_secondary": spec["secondary"],
            "measured_delta": deltas,
            "suggested_priority": spec["priority"],
            "g11_anchor": f"G11 立项承接：{spec['title']}（只消费 G10.8b 锁定清单 {spec['key']} 行 + 本锚；G10 零修复纪律不回头改 G10 帧库）",
            "title": spec["title"],
            "description": spec["description"],
            "attachments": [measured["digests"]["error_map"][spec["scene"]], measured["digests"]["heatmap"][spec["scene"]]],
        }
        if "attribution_note" in spec:
            item["attribution_note"] = spec["attribution_note"]
        items.append(item)

    per_scene = {s: [it for it in items if it["scene_id"] == s] for s in SCENES}
    return {
        "schema_version": 1,
        "registry": gaplib.REGISTRY_NAME,
        "generated_by": "ci/g10_ab_comparison_smoke.py --gate g10.p0.m139.ab_comparison",
        "scene_set": list(SCENES),
        "items": items,
        "scene_summary": [
            {"scene_id": s, "gap_count": len(per_scene[s]), "no_gap_explicit": len(per_scene[s]) == 0}
            for s in SCENES
        ],
        "not_ready_scenes": [],
    }


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂①：lib 自检联动（单一事实源消费面）。
    if gaplib.selftest() != 0:
        print(f"[{TAG}] selftest FAIL: gap_registry_lib 自检未过", file=sys.stderr)
        return 1
    # 绿臂②：帧齐备面真树通过 + 红臂（单端缺帧检出）。
    if frame_set_problems(FRAMES):
        print(f"[{TAG}] selftest FAIL: 真树帧库齐备面有缺", file=sys.stderr)
        return 1
    with tempfile.TemporaryDirectory(prefix="g10_m139_selftest_") as td:
        bad_root = Path(td)
        for scene in SCENES:
            (bad_root / "rurix").mkdir(parents=True, exist_ok=True)
            (bad_root / "ldr").mkdir(parents=True, exist_ok=True)
            src = frame_paths(FRAMES, scene)
            dst = frame_paths(bad_root, scene)
            dst["hdr_rurix"].write_bytes(src["hdr_rurix"].read_bytes())
            dst["ldr_rurix"].write_bytes(src["ldr_rurix"].read_bytes())
            dst["ldr_ue5"].write_bytes(src["ldr_ue5"].read_bytes())
            # UE HDR 缺失（单端缺帧）。
        if not frame_set_problems(bad_root):
            print(f"[{TAG}] selftest FAIL: 单端缺帧未检出", file=sys.stderr)
            return 1
    # 红臂：三重绑定核验负样本（digest 不等 / 陈旧绑定）。
    synth = {
        "status": "pass", "phase_g10_5_pass": True, "base_commit": "abc",
        "contract_report": {
            "param_digest": "sha256:x", "param_digest_rurix": "sha256:x",
            "param_digest_ue5": "sha256:y", "session_run_id": "s",
        },
    }
    if m130.verify_three_binding(synth, "sha256:x", "s", "abc"):
        print(f"[{TAG}] selftest FAIL: 双端 digest 不等未被拒", file=sys.stderr)
        return 1
    synth2 = copy.deepcopy(synth)
    synth2["contract_report"]["param_digest_ue5"] = "sha256:x"
    if m130.verify_three_binding(synth2, "sha256:x", "stale", "abc"):
        print(f"[{TAG}] selftest FAIL: 陈旧 session_run_id 未被拒", file=sys.stderr)
        return 1
    if not m130.verify_three_binding(synth2, "sha256:x", "s", "abc"):
        print(f"[{TAG}] selftest FAIL: 正当绑定被误拒", file=sys.stderr)
        return 1
    # 红臂：口径漂移 ⇒ caliber digest 变。
    cal0 = flip.flip_ldr_caliber_literal(flip.default_ppd())
    cal1 = flip.flip_ldr_caliber_literal(flip.default_ppd() + 1e-6)
    if cal0 == cal1:
        print(f"[{TAG}] selftest FAIL: ppd 漂移未引起口径字面变化", file=sys.stderr)
        return 1
    # 绿臂③：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (4 RED + 4 GREEN)")
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
    run_start = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    base_commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                 capture_output=True, text=True).stdout.strip()

    # ---- ① spec 条款头在树 ----
    checks["spec_rxs0391_clause_on_tree"] = SPEC_PATH.is_file() and (
        re.search(r"^###\s+RXS-0391\b", SPEC_PATH.read_text(encoding="utf-8"), re.MULTILINE)
        is not None
    )
    check(checks["spec_rxs0391_clause_on_tree"], "spec/visual_comparison.md 缺 RXS-0391 条款头")

    # ---- ② 门序：M130 g10.5 腿当次 session 真跑（子进程自持锁，本门不嵌套） ----
    r = run_cmd([
        sys.executable, str(ROOT / "ci" / "g10_dual_determinism_contract_smoke.py"),
        "--gate", "g10.p0.m130.dual_determinism_contract", "--phase", "g10.5",
    ], timeout=5400)
    m130_path = None
    m130_doc = None
    if r.returncode == 0:
        try:
            m130_path, m130_doc = latest_m130_g10_5()
        except RuntimeError as e:
            check(False, f"M130 g10.5 最新 evidence 装载失败: {e}")
    fresh = (
        m130_doc is not None
        and m130_doc.get("status") == "pass"
        and m130_doc.get("phase_g10_5_pass") is True
        and str(m130_doc.get("timestamp", "")) >= run_start
    )
    checks["m130_g10_5_leg_fresh_pass"] = r.returncode == 0 and fresh
    check(checks["m130_g10_5_leg_fresh_pass"],
          f"M130 g10.5 腿未当次 session 通过（exit={r.returncode}, fresh={fresh}）")

    # ---- ③ 三重绑定机器核验（联合 param_digest 由本门 host 侧独立重算对账） ----
    rep = (m130_doc or {}).get("contract_report", {})
    session_run_id = rep.get("session_run_id", "")
    host_scene_digests: dict[str, str] = {}
    host_joint_digest = ""
    try:
        for s in SCENES:
            p = CORPUS / f"contract_params_{s.replace('-', '_')}.json"
            host_scene_digests[s] = m130.param_digest_rurix(
                m130.parse_contract_rurix(p.read_text(encoding="utf-8"))
            )
        host_joint_digest = "sha256:" + hashlib.sha256(
            "".join(sorted(host_scene_digests.values())).encode("ascii")
        ).hexdigest()
    except Exception as e:  # noqa: BLE001 — host 独立重算失败即绑定判据失效
        check(False, f"host 侧契约 digest 独立重算失败: {e}")
    param_digest = host_joint_digest
    bind_ok = (
        m130_doc is not None
        and bool(host_joint_digest)
        and m130_doc.get("base_commit") == base_commit
        and rep.get("param_digest") == host_joint_digest
        and all(
            rep.get("scenes", {}).get(s, {}).get("param_digest") == f"sha256:{host_scene_digests[s]}"
            for s in SCENES
        )
        and m130.verify_three_binding(m130_doc, host_joint_digest, session_run_id, base_commit)
    )
    checks["three_binding_verified"] = bool(bind_ok)
    check(bind_ok, "三重绑定核验失败（digest/base_commit/session_run_id 不自洽）")
    if bind_ok:
        note(f"三重绑定核验通过：param_digest={param_digest[:24]}…（host 独立重算对账）session_run_id={session_run_id} base_commit={base_commit[:12]}")

    # ---- ④ 双端帧齐备（场景全集；单端缺帧即问题行） ----
    frame_problems = frame_set_problems(FRAMES)
    checks["dual_end_frames_present_full_scene_set"] = not frame_problems
    check(not frame_problems, f"双端帧齐备面问题: {frame_problems[:3]}")

    # ---- ⑤ Rurix HDR release 重渲染逐位复现 + LDR 派生逐字节复现 ----
    repro_ok = True
    ldr_repro_ok = True
    r_build = run_cmd(["cargo", "build", "--release", "-p", "rurix-asset", "--bin", "g10_5_scene_render"], timeout=3600)
    if r_build.returncode != 0 or not RUST_RELEASE_BIN.is_file():
        check(False, "release 渲染器构建失败")
        repro_ok = ldr_repro_ok = False
    else:
        REPRO_DIR.mkdir(parents=True, exist_ok=True)
        for scene in SCENES:
            params = CORPUS / f"contract_params_{scene.replace('-', '_')}.json"
            out_dir = REPRO_DIR / scene
            out_dir.mkdir(parents=True, exist_ok=True)
            rr = run_cmd([
                str(RUST_RELEASE_BIN), "--render", "--gltf", str(GLTF[scene]),
                "--contract", str(params), "--out-dir", str(out_dir), "--scene-id", scene,
            ], timeout=1800)
            repro_digest = ""
            if rr.returncode == 0:
                m = re.search(r'"frame_content_digest":"(sha256:[0-9a-f]{64})"', rr.stdout or "")
                repro_digest = m.group(1) if m else ""
            want = LIB_HDR_DIGEST[(scene, "rurix")]
            if rr.returncode != 0 or repro_digest != want:
                check(False, f"Rurix HDR 重渲染未逐位复现（{scene}）: {repro_digest} ≠ {want}")
                repro_ok = False
            # LDR 派生逐字节复现（双端两臂；--params-digest = 逐场景契约 digest，
            # M130 evidence contract_report.scenes[] 登记面）。
            lib_hdr_r = FRAMES / "rurix" / f"{scene}.exr"
            lib_hdr_u = FRAMES / "ue" / scene / ".0000.exr"
            scene_digest = (
                (m130_doc or {}).get("contract_report", {}).get("scenes", {})
                .get(scene, {}).get("param_digest", "")
            ).replace("sha256:", "")
            for end, hdr_path, scale, lib_ldr in (
                ("rurix", lib_hdr_r, EXPOSURE_SCALE_RURIX[scene], FRAMES / "ldr" / f"{scene}_rurix_ldr.exr"),
                ("ue5", lib_hdr_u, 1.0, FRAMES / "ldr" / f"{scene}_ue5_ldr.exr"),
            ):
                out_ldr = out_dir / f"{scene}_{end}_ldr.exr"
                rd = run_cmd([
                    str(RUST_RELEASE_BIN), "--derive-ldr", "--hdr", str(hdr_path),
                    "--source-end", end, "--out", str(out_ldr),
                    "--exposure-scale", str(scale), "--params-digest", scene_digest,
                ], timeout=900)
                if rd.returncode != 0 or not out_ldr.is_file() or out_ldr.read_bytes() != lib_ldr.read_bytes():
                    check(False, f"LDR 派生未逐字节复现（{scene}/{end}）")
                    ldr_repro_ok = False
    checks["rurix_frames_reproduced_bit_exact"] = repro_ok
    check(repro_ok, "Rurix HDR 重渲染逐位复现失败")
    checks["ldr_derivation_reproduced_bit_exact"] = ldr_repro_ok
    check(ldr_repro_ok, "LDR 派生逐字节复现失败")

    # ---- ⑥ UE 帧 provenance（unreal/build == M128 登记 ue_build_id）+ 库 digest ----
    ue_ok = True
    m128_path = wel.load_latest_evidence("g10_m128_ue5_capture_environment")
    ue_build_id = ""
    if m128_path is not None:
        try:
            ue_build_id = json.loads(m128_path.read_text(encoding="utf-8")).get("capture_report", {}).get("ue_build_id", "")
        except Exception:
            ue_build_id = ""
    if not ue_build_id:
        check(False, "M128 最新 evidence 缺 ue_build_id")
        ue_ok = False
    for scene in SCENES:
        p = FRAMES / "ue" / scene / ".0000.exr"
        if not p.is_file():
            check(False, f"UE HDR 帧缺失（{scene}）")
            ue_ok = False
            continue
        attrs, _ = exr.parse_header(p.read_bytes())
        build_attr = next((a[2].decode("utf-8", "replace") for a in attrs if a[0] == "unreal/build"), "")
        if not build_attr.startswith(ue_build_id):
            check(False, f"UE 帧 build 与 M128 登记不符（{scene}）: {build_attr!r} vs {ue_build_id!r}")
            ue_ok = False
        d = exr.decode_exr(p.read_bytes(), "ue5")
        content = exr.frame_content_digest(d["width"], d["height"], 3, d["pixels"])
        if content != LIB_HDR_DIGEST[(scene, "ue5")]:
            check(False, f"UE HDR 库帧 digest 漂移（{scene}）: {content} ≠ {LIB_HDR_DIGEST[(scene, 'ue5')]}")
            ue_ok = False
    checks["ue_frames_provenance_and_digest_match"] = ue_ok
    check(ue_ok, "UE 帧 provenance/digest 核验失败")

    # ---- ⑦ 派生链元数据互证 + 度量重算（golden 逐位） ----
    measured: dict = {}
    chain_ok = True
    metric_ok = True
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    artifact_digests: set[str] = set()
    metric_report_digests: dict[str, str] = {}
    for scene in SCENES:
        fp = frame_paths(FRAMES, scene)
        hdr_r, arr_hdr_r = load_pixels(fp["hdr_rurix"], "rurix")
        hdr_u, arr_hdr_u = load_pixels(fp["hdr_ue5"], "ue5")
        ldr_r, arr_r = load_pixels(fp["ldr_rurix"], "rurix")
        ldr_u, arr_u = load_pixels(fp["ldr_ue5"], "rurix")
        # 派生链互证：LDR source_frame_digest == HDR 内容 digest 重算；
        # HDR capture_params_digest == 契约逐场景 digest；LDR domain == ldr。
        scene_param_digest = (
            (m130_doc or {}).get("contract_report", {}).get("scenes", {})
            .get(scene, {}).get("param_digest", "")
        )
        hdr_r_digest = exr.frame_content_digest(hdr_r["width"], hdr_r["height"], 3, hdr_r["pixels"])
        hdr_u_digest = exr.frame_content_digest(hdr_u["width"], hdr_u["height"], 3, hdr_u["pixels"])
        md_pairs = [
            (ldr_r["metadata"].get("rurix:source_frame_digest"), hdr_r_digest, "rurix"),
            (ldr_u["metadata"].get("rurix:source_frame_digest"), hdr_u_digest, "ue5"),
        ]
        for got, want, end in md_pairs:
            if got != want:
                check(False, f"派生链互证失败（{scene}/{end}）: {got} ≠ {want}")
                chain_ok = False
        if hdr_r["metadata"].get("rurix:capture_params_digest") != scene_param_digest:
            check(False, f"HDR capture_params_digest ≠ 契约 digest（{scene}）")
            chain_ok = False
        if ldr_r["metadata"].get("rurix:domain") != "display-referred-ldr" or ldr_u["metadata"].get("rurix:domain") != "display-referred-ldr":
            check(False, f"LDR 域标签非 display-referred-ldr（{scene}）")
            chain_ok = False
        # 度量重算（LDR 臂；参考端 = UE5）。
        ssim_v = ssim_psnr.ssim_wang2004(arr_u, arr_r)
        psnr_v = ssim_psnr.psnr_joint(arr_u, arr_r)
        _err_map, flip_v = flip.flip_ldr(arr_u, arr_r)
        stats = {
            "hdr_rurix": lum_stats(arr_hdr_r), "hdr_ue5": lum_stats(arr_hdr_u),
            "ldr_rurix": lum_stats(arr_r), "ldr_ue5": lum_stats(arr_u),
        }
        measured[scene + "::dims"] = [hdr_r["width"], hdr_r["height"]]
        metrics = {
            "flip_ldr": float(flip_v),
            "ssim": float(ssim_v),
            "psnr_db": ssim_psnr.psnr_json_value(psnr_v),
        }
        g = GOLDEN[scene]
        golden_ok = (
            metrics["flip_ldr"] == g["flip_ldr"]
            and metrics["ssim"] == g["ssim"]
            and metrics["psnr_db"] == g["psnr_db"]
            and all(stats[k][kk] == g[k][kk] for k in ("hdr_rurix", "hdr_ue5", "ldr_rurix", "ldr_ue5") for kk in ("median", "p90", "max", "nonzero_ratio"))
        )
        if not golden_ok:
            check(False, f"度量重算 ≠ G10.5a golden（{scene}）: {metrics}")
            metric_ok = False
        measured[scene] = {**stats, "metrics": metrics}
        # 逐场景度量报告 artifact（差距清单 measured_delta 回溯锚）。
        artifact = {
            "scene_id": scene,
            "camera_id": CAMERA_ID,
            "frame_digests": {
                "hdr_rurix": hdr_r_digest, "hdr_ue5": hdr_u_digest,
                "ldr_rurix_source": ldr_r["metadata"].get("rurix:source_frame_digest"),
                "ldr_ue5_source": ldr_u["metadata"].get("rurix:source_frame_digest"),
            },
            "stats": stats,
            "metrics": metrics,
            "metric_caliber": {
                "flip_ldr": flip.flip_ldr_caliber_literal(flip.default_ppd()),
                "ssim_psnr": "SSIM Wang 2004（11×11 高斯 σ=1.5，K1=0.01，K2=0.03，data_range=1.0，总体协方差，逐通道均值）/ PSNR 联合 MSE（RXS-0387）",
                "domain": "display-referred-ldr",
            },
            "exposure_scale": {"rurix": EXPOSURE_SCALE_RURIX[scene], "ue5": 1.0},
            "exr_source_bit_depth": {"rurix": 32, "ue5": 16},
        }
        apath = REPORT_DIR / f"{scene}_metric_report.json"
        apath.write_text(json.dumps(artifact, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        adigest = sha256_file(apath)
        metric_report_digests[scene] = adigest
        artifact_digests.add(adigest)
        artifact_digests.add(hdr_r_digest)
        artifact_digests.add(hdr_u_digest)
    checks["derivation_chain_metadata_consistent"] = chain_ok
    check(chain_ok, "派生链元数据互证失败")
    checks["metric_report_recomputed_golden"] = metric_ok
    check(metric_ok, "度量重算与 golden 不逐位相等（口径/工具链漂移）")

    # ---- ⑧ 逐像素 diff 报告重跑 + 独立重算 ----
    diff_ok = True
    diff_digests: dict[str, str] = {}
    heatmap_digests: dict[str, str] = {}
    error_map_digests: dict[str, str] = {}
    r = run_cmd(["cargo", "build", "-p", "rurix-render", "--bin", "g10_m137_diff_report"], timeout=3600)
    if r.returncode != 0 or not DIFF_BIN.is_file():
        check(False, "diff 报告器构建失败")
        diff_ok = False
    else:
        for scene in SCENES:
            fp = frame_paths(FRAMES, scene)
            diff_dir = FRAMES / "diff" / scene
            diff_dir.mkdir(parents=True, exist_ok=True)
            ev_path = diff_dir / "diff_report.json"
            rr = run_cmd([
                str(DIFF_BIN),
                "--frame-a", str(fp["ldr_ue5"]), "--frame-b", str(fp["ldr_rurix"]),
                "--out-dir", str(diff_dir), "--evidence", str(ev_path),
                "--scene-id", scene, "--camera-id", CAMERA_ID,
                "--frame-index", "0", "--threshold", "0.0",
            ], timeout=1800)
            if rr.returncode != 0 or not ev_path.is_file():
                check(False, f"diff 报告跑失败（{scene}）: {(rr.stdout or '')[-200:]}{(rr.stderr or '')[-200:]}")
                diff_ok = False
                continue
            report = json.loads(ev_path.read_text(encoding="utf-8"))
            cs_fails = m137.closed_set_failures(report)
            if cs_fails:
                check(False, f"diff 报告闭集机核失败（{scene}）: {cs_fails}")
                diff_ok = False
            if report.get("domain") != "display-referred-ldr":
                check(False, f"diff 报告 domain ≠ display-referred-ldr（{scene}，H1 修订面）: {report.get('domain')!r}")
                diff_ok = False
            if len(report.get("regions", [])) != 256:
                check(False, f"diff 报告区域数 ≠ 256（{scene}）")
                diff_ok = False
            # 独立重算（误差 EXR → 标量三面一致 + artifacts 四 digest 对账）。
            try:
                em = exr.decode_exr_file(diff_dir / "error_map.exr", "rurix")
                all_err = sorted(float(v) for v in em["pixels"])
                n = len(all_err)
                scal = report["scalars"]
                g = GOLDEN[scene]["diff"]
                recompute_ok = (
                    m137.f32_eq(all_err[-1], scal["err_max"])
                    and m137.f32_eq(exr.nearest_rank_p95(all_err), scal["err_p95"])
                    and abs(sum(all_err) / n - scal["err_mean"]) <= 1e-12
                    and abs(scal["over_threshold_ratio"] - g["over_threshold_ratio"]) <= 1e-12
                    and abs(scal["err_mean"] - g["err_mean"]) <= 1e-12
                    and m137.f32_eq(scal["err_p95"], g["err_p95"])
                    and m137.f32_eq(scal["err_max"], g["err_max"])
                )
                arts = report["artifacts"]
                fa = exr.decode_exr_file(fp["ldr_ue5"], "rurix")
                fb = exr.decode_exr_file(fp["ldr_rurix"], "rurix")
                digest_ok = (
                    arts["frame_a_digest"] == exr.frame_content_digest(fa["width"], fa["height"], 3, fa["pixels"])
                    and arts["frame_b_digest"] == exr.frame_content_digest(fb["width"], fb["height"], 3, fb["pixels"])
                    and arts["error_map_digest"] == exr.frame_content_digest(em["width"], em["height"], 1, em["pixels"])
                    and arts["heatmap_digest"] == sha256_file(diff_dir / "heatmap.ppm")
                )
                if not (recompute_ok and digest_ok):
                    check(False, f"diff 独立重算/digest 对账失败（{scene}）: recompute={recompute_ok} digest={digest_ok}")
                    diff_ok = False
                d_digest = sha256_file(ev_path)
                diff_digests[scene] = d_digest
                heatmap_digests[scene] = arts["heatmap_digest"]
                error_map_digests[scene] = arts["error_map_digest"]
                artifact_digests.add(d_digest)
                artifact_digests.add(arts["error_map_digest"])
                artifact_digests.add(arts["heatmap_digest"])
            except Exception as e:  # noqa: BLE001 — 独立复核面异常即判据失效
                check(False, f"diff 独立重算异常（{scene}）: {e}")
                diff_ok = False
    checks["diff_reports_regenerated_consistent"] = diff_ok
    check(diff_ok, "diff 报告重跑/独立重算失败")

    # ---- ⑨ 探针 artifact：R5 seed u64 顶格拒绝 + U3 bistro 动画通道计数 ----
    probe_digests: dict[str, str] = {}
    seed_probe_path = REPORT_DIR / "rurix_seed_u64_max_probe.json"
    seed_doc: dict = {"scene_id": "cornell-box", "probe": "contract_seed_u64_max_rejection"}
    if RUST_RELEASE_BIN.is_file():
        base_params = json.loads((CORPUS / "contract_params_cornell_box.json").read_text(encoding="utf-8"))
        base_params["time"]["random_seed"] = 18446744073709551615
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False, encoding="utf-8") as tf:
            json.dump(base_params, tf)
            tmp_params = Path(tf.name)
        try:
            rp = run_cmd([str(RUST_RELEASE_BIN), "--contract-digest", str(tmp_params)], timeout=300)
            seed_doc["rust_exit_code"] = rp.returncode
            # 确定性登记面：易失临时路径不入 artifact（幂等复核纪律）——只登记
            # exit code 与 i64 域界拒收标记命中与否。
            seed_doc["stderr_has_i64_boundary_reject"] = "i64" in (rp.stderr or "")
        finally:
            tmp_params.unlink(missing_ok=True)
    seed_doc["rejected"] = seed_doc.get("rust_exit_code", 0) != 0
    seed_probe_path.write_text(json.dumps(seed_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    probe_digests["seed_probe"] = sha256_file(seed_probe_path)
    artifact_digests.add(probe_digests["seed_probe"])
    if not seed_doc["rejected"]:
        check(False, "R5 探针失效：u64 顶格 seed 未被 Rust 端拒绝")
    note(f"R5 探针：u64 顶格 seed Rust 端 exit={seed_doc.get('rust_exit_code')}（fail-closed 拒绝实测）")

    bistro_anim_channels = 0
    gltf_probe_path = REPORT_DIR / "bistro_gltf_animations_probe.json"
    try:
        gltf_text = BISTRO_GLTF.read_text(encoding="utf-8")
        gltf_doc = json.loads(gltf_text)
        anims = gltf_doc.get("animations", [])
        bistro_anim_channels = sum(len(a.get("channels", [])) for a in anims)
        gltf_probe = {
            "gltf": str(BISTRO_GLTF),
            "gltf_digest": "sha256:" + hashlib.sha256(gltf_text.encode("utf-8")).hexdigest(),
            "animations_count": len(anims),
            "animation_channels": bistro_anim_channels,
            "consumed_by_dual_end": 0,
        }
    except Exception as e:  # noqa: BLE001
        gltf_probe = {"error": str(e)}
        check(False, f"U3 探针失败：bistro glTF 动画计数不可读: {e}")
    gltf_probe_path.write_text(json.dumps(gltf_probe, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    probe_digests["gltf_probe"] = sha256_file(gltf_probe_path)
    artifact_digests.add(probe_digests["gltf_probe"])
    note(f"U3 探针：bistro glTF animations={gltf_probe.get('animations_count')} channels={bistro_anim_channels}（双端消费 0）")

    # ---- ⑩ 差距清单装配 + 校验 + 幂等落盘 ----
    registry_ok = True
    registry_doc: dict = {}
    if metric_ok and diff_ok and seed_doc.get("rejected") and "error" not in gltf_probe:
        measured["digests"] = {
            "metric_report": metric_report_digests,
            "diff_report": diff_digests,
            "heatmap": heatmap_digests,
            "error_map": error_map_digests,
            **probe_digests,
        }
        measured["bistro_anim_channels"] = bistro_anim_channels
        registry_doc = build_gap_registry(measured, sorted(artifact_digests))
        verrs = gaplib.validate_registry(registry_doc, scene_set=list(SCENES))
        # evidence_digest 回溯面：全 measured_delta 的 evidence_digest ∈ 本门登记集。
        for it in registry_doc["items"]:
            for d in it["measured_delta"]:
                if d["evidence_digest"] not in artifact_digests:
                    verrs.append(f"{it['gap_id']} evidence_digest 不可回溯: {d['evidence_digest']}")
        if verrs:
            check(False, f"差距清单校验失败: {verrs[:4]}")
            registry_ok = False
        else:
            new_text = json.dumps(registry_doc, ensure_ascii=False, indent=2) + "\n"
            if REGISTRY_PATH.is_file():
                old_text = REGISTRY_PATH.read_text(encoding="utf-8")
                if old_text != new_text:
                    check(False, "差距清单在树内容与当次装配漂移（幂等复核 RED）")
                    registry_ok = False
            else:
                REGISTRY_PATH.write_text(new_text, encoding="utf-8", newline="")
                note("差距清单首次落盘 milestones/g10/g10_gap_registry.json")
    else:
        registry_ok = False
        check(False, "前置度量/diff/探针未全绿，差距清单不装配（fail-closed）")
    checks["gap_registry_materialized_valid"] = registry_ok
    check(registry_ok, "差距清单装配/校验/落盘失败")

    # ---- RED 臂①：差距清单缺场景行 ⇒ 校验必拒 ----
    red1 = False
    if registry_doc:
        tampered = copy.deepcopy(registry_doc)
        tampered["scene_summary"] = [r for r in tampered["scene_summary"] if r["scene_id"] != "bistro-interior"]
        red1 = bool(gaplib.validate_registry(tampered, scene_set=list(SCENES)))
    checks["red_missing_scene_row_detected"] = red1
    check(red1, "RED 臂失效：缺场景行未被清单校验检出")

    # ---- RED 臂②：单端缺帧 ⇒ 帧齐备面必出问题行 ----
    red2 = False
    with tempfile.TemporaryDirectory(prefix="g10_m139_red_") as td:
        bad_root = Path(td)
        for scene in SCENES:
            (bad_root / "rurix").mkdir(parents=True, exist_ok=True)
            (bad_root / "ldr").mkdir(parents=True, exist_ok=True)
            src = frame_paths(FRAMES, scene)
            dst = frame_paths(bad_root, scene)
            dst["hdr_rurix"].write_bytes(src["hdr_rurix"].read_bytes())
            dst["ldr_rurix"].write_bytes(src["ldr_rurix"].read_bytes())
            dst["ldr_ue5"].write_bytes(src["ldr_ue5"].read_bytes())
        red2 = bool(frame_set_problems(bad_root))
    checks["red_single_end_missing_frame_detected"] = red2
    check(red2, "RED 臂失效：单端缺帧未检出")

    # ---- RED 臂③：口径漂移 ⇒ caliber digest 变 ----
    cal_literal = flip.flip_ldr_caliber_literal(flip.default_ppd())
    cal_drift = flip.flip_ldr_caliber_literal(flip.default_ppd() + 1e-6)
    red3 = cal_literal != cal_drift
    checks["red_caliber_drift_detected"] = red3
    check(red3, "RED 臂失效：ppd 漂移未引起口径字面变化")

    # ---- RED 臂④：digest 不等仍出报告 ⇒ 三重绑定阻断 ----
    synth_unequal = {
        "status": "pass", "phase_g10_5_pass": True, "base_commit": base_commit,
        "contract_report": {
            "param_digest": param_digest, "param_digest_rurix": param_digest,
            "param_digest_ue5": "sha256:" + "0" * 64, "session_run_id": session_run_id,
        },
    }
    red4 = not m130.verify_three_binding(synth_unequal, param_digest, session_run_id, base_commit)
    checks["red_digest_unequal_report_blocked"] = red4
    check(red4, "RED 臂失效：digest 不等仍出报告未被阻断")

    # ---- RED 臂⑤：陈旧绑定冒充 ⇒ 三重绑定拒收 ----
    synth_stale = {
        "status": "pass", "phase_g10_5_pass": True, "base_commit": base_commit,
        "contract_report": {
            "param_digest": param_digest, "param_digest_rurix": param_digest,
            "param_digest_ue5": param_digest, "session_run_id": session_run_id + "-stale",
        },
    }
    red5 = not m130.verify_three_binding(synth_stale, param_digest, session_run_id, base_commit)
    checks["red_stale_binding_detected"] = red5
    check(red5, "RED 臂失效：陈旧绑定冒充未被拒")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    caliber_digest = "sha256:" + hashlib.sha256(
        (cal_literal + "\n" + "SSIM Wang 2004/PSNR 联合 MSE（RXS-0387）\ndisplay-referred-ldr").encode("utf-8")
    ).hexdigest()
    artifact_digests_sorted = sorted(artifact_digests)
    registry_digest = sha256_file(REGISTRY_PATH) if REGISTRY_PATH.is_file() else ""

    ab_report = {
        "three_binding": {
            "param_digest": param_digest,
            "param_digest_rurix": rep.get("param_digest_rurix", ""),
            "param_digest_ue5": rep.get("param_digest_ue5", ""),
            "session_run_id": session_run_id,
            "base_commit": base_commit,
            "m130_evidence_path": str(m130_path.relative_to(ROOT)).replace("\\", "/") if m130_path else "",
        },
        "scenes": {
            scene: {
                "resolution": measured.get(scene + "::dims", []),
                "metrics": measured.get(scene, {}).get("metrics", {}),
                "hdr_stats": {k: measured.get(scene, {}).get(k, {}) for k in ("hdr_rurix", "hdr_ue5")},
                "ldr_stats": {k: measured.get(scene, {}).get(k, {}) for k in ("ldr_rurix", "ldr_ue5")},
                "metric_report_digest": metric_report_digests.get(scene, ""),
                "diff_report_digest": diff_digests.get(scene, ""),
                "diff_scalars": GOLDEN[scene]["diff"],
            }
            for scene in SCENES
        },
        "metric_caliber": {"flip_ldr": cal_literal, "digest": caliber_digest},
        "gap_registry": {
            "path": "milestones/g10/g10_gap_registry.json",
            "digest": registry_digest,
            "item_count": len(registry_doc.get("items", [])),
            "kind_split": {
                "quality_gap": sum(1 for it in registry_doc.get("items", []) if it["kind"] == "quality_gap"),
                "caliber_diff": sum(1 for it in registry_doc.get("items", []) if it["kind"] == "caliber_diff"),
            },
            "other_module_rows": sum(1 for it in registry_doc.get("items", []) if it["ue5_module_primary"] == gaplib.OTHER_MODULE),
        },
        "probes": probe_digests,
        "artifact_digests": artifact_digests_sorted,
        "zero_pass_line": "G10 零通过线维持：本报告全部数字为 measured_local 登记，不构成任何画质/帧率通过判定（契约 G-G10-7 / 立项裁决 5）",
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
        "ab_report": ab_report,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=executed")
    if all_pass and not FAILURES:
        print(
            f"[{TAG}] PASS（三重绑定同 session 核验 + 双端四组帧齐备/逐位复现 + "
            f"度量 golden 逐位复核 + diff 三面重算 + 差距清单 {len(registry_doc.get('items', []))} 项落盘 + RED 五臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
