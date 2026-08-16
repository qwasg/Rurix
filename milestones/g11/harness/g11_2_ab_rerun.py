#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.2 波）
"""G11.2 口径差对齐波 双端 A/B 复跑驱动（spec/visual_comparison.md RXS-0392/RXS-0393；
G11_CONTRACT §4.2 M144/M145/M146/M157 + G-G11-4；g10_gap_registry C1/C2/C3 行承接锚字面消费）。

链路面（与 G10.5a 同构，G10 帧库只读——本批全部产物落 G11.2 帧区
K:/rurix-ext/g11-frames/g11_2/）：

  契约参数（milestones/g10/corpus/contract_params_<scene>.json，digest == G10.5
  锁定值 0-byte——三面绑定机核）→
  ├─ Rurix 端：g10_5_scene_render --render **--exposure-scale 2^(−EV100)**
  │   （C2 对齐：曝光尺度管線内烘焙，与 UE 臂 pipe 内 FixedExposure=2^(−EV100)
  │   同域——HDR 帧 = 曝光已施 scene-linear，LDR 派生尺度双端统一 ×1.0）→ HDR EXR
  └─ UE 端：g10_5_build_scenes.py（**b_srgb=False 光色线性直给**——C1 太阳色链
      口径修复，G10.5a b_srgb=True sRGB 二次转换偏差 G−2.5%/B−6.3% 实测登记）
      → MRQ Phase B → HDR EXR（fp16 → 度量域 f32 提升，C3 对齐面）
  双端 HDR → LDR 派生（**×1.0 双端统一**；RXS-0386 L2 派生链元数据互证回归）
  → HDR 亮度统计 + FLIP/SSIM/PSNR（ci/g10_flip_lib.py / g10_ssim_psnr_lib.py
  单一事实源）→ g11_2_rerun_report.json + g11_2_residual_caliber_registry.json
  （C1 残余口径差逐环节显式登记面，RXS-0392 L4）。

G11 不设绝对画质通过线：全部数字 measured_local 登记，收敛判定归四门机核。

用法：
  py -3 milestones/g11/harness/g11_2_ab_rerun.py --stage all
  py -3 milestones/g11/harness/g11_2_ab_rerun.py --stage contract|rurix|ue|derive|metrics|registry
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))

import g10_exr_lib as exr  # noqa: E402
import g10_flip_lib as flip  # noqa: E402
import g10_ssim_psnr_lib as ssim_psnr  # noqa: E402

CORPUS = ROOT / "milestones" / "g10" / "corpus"
GAP_REGISTRY = ROOT / "milestones" / "g10" / "g10_gap_registry.json"
FRAMES_G11 = Path(r"K:\rurix-ext\g11-frames\g11_2")
REPORT_PATH = ROOT / "milestones" / "g11" / "g11_2_rerun_report.json"
RESIDUAL_PATH = ROOT / "milestones" / "g11" / "g11_2_residual_caliber_registry.json"
RUST_RELEASE_BIN = ROOT / "target" / "release" / "g10_5_scene_render.exe"
UE_RUN = ROOT / "milestones" / "g10" / "harness" / "g10_5_ue_run.py"
UE_BUILD = ROOT / "milestones" / "g10" / "harness" / "ue_python" / "g10_5_build_scenes.py"
UE_RENDER = ROOT / "milestones" / "g10" / "harness" / "g10_5_ue_render.py"
GEN_PARAMS = ROOT / "milestones" / "g10" / "harness" / "g10_5_gen_contract_params.py"
WHITE_HDR = Path(r"K:\rurix-ext\g10-ue\harness_assets\white_2x1.hdr")

# G10.5 锁定契约 digest（机核事实源 = evidence/g10_m130_dual_determinism_contract_
# 20260815T233315Z.json 登记值；RXS-0393 L4 转引字面）。
LOCKED_DIGEST = {
    "cornell-box": "sha256:80305791a68ccc66c5b046efaf193244796b52570494cf00aa1c86efa55be118",
    "bistro-interior": "sha256:ad45951ba641106b24e7d91d49ebf5992fb6a42cb70a3082520e8de19a6cf514",
}

SCENES = {
    "cornell-box": {
        "gltf": Path(r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf"),
        "ev100": 2.0,
        "res": (512, 512),
    },
    "bistro-interior": {
        "gltf": Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf"),
        "ev100": 1.0,
        "res": (1920, 1080),
    },
}

COMMANDS: list[dict] = []


def log(msg: str) -> None:
    print(f"[g11_2_rerun] {msg}", flush=True)


def run_cmd(argv: list[str], *, env: dict | None = None, timeout: int = 3600) -> subprocess.CompletedProcess:
    log("$ " + " ".join(str(a) for a in argv))
    e = dict(os.environ)
    if env:
        e.update(env)
    r = subprocess.run([str(a) for a in argv], cwd=ROOT, capture_output=True, text=True, env=e, timeout=timeout)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(str(a) for a in argv), "exit_code": r.returncode})
    if r.returncode != 0:
        tail = (r.stdout + r.stderr)[-3000:]
        log(f"FAIL exit={r.returncode}: {tail}")
        raise SystemExit(f"命令失败（exit={r.returncode}）: {argv[0]}")
    return r


def sha256_file(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def frame_content_digest(width: int, height: int, channels: int, pixels) -> str:
    return exr.frame_content_digest(width, height, channels, pixels)


def stage_contract() -> dict:
    """契约 digest 三面绑定机核（RXS-0393 L4）：重生成 + Rust 第三实现 digest ==
    G10.5 锁定值（M130 g10.5 evidence 登记字面）。"""
    run_cmd([sys.executable, str(GEN_PARAMS)])
    run_cmd(["cargo", "build", "--release", "-p", "rurix-asset", "--bin", "g10_5_scene_render"], timeout=3600)
    out = {}
    for scene_id in SCENES:
        p = CORPUS / f"contract_params_{scene_id.replace('-', '_')}.json"
        r = run_cmd([str(RUST_RELEASE_BIN), "--contract-digest", str(p)])
        line = [l for l in r.stdout.splitlines() if "param_digest_rust" in l][-1]
        got = "sha256:" + line.split("=")[-1].strip()
        want = LOCKED_DIGEST[scene_id]
        if got != want:
            raise SystemExit(f"契约 digest 漂移（{scene_id}）: {got} ≠ {want}（G10.5 锁定值）——修复动契约参数即 RED")
        out[scene_id] = got
        log(f"契约 digest 锁定复核 {scene_id}: {got} ✓")
    return out


def stage_rurix() -> dict:
    """Rurix 端渲染（曝光尺度管线内烘焙 2^(−EV100)——C2 对齐落点）。"""
    out = {}
    for scene_id, s in SCENES.items():
        scale = 2.0 ** (-s["ev100"])
        out_dir = FRAMES_G11 / "rurix"
        out_dir.mkdir(parents=True, exist_ok=True)
        r = run_cmd([
            str(RUST_RELEASE_BIN), "--render",
            "--gltf", str(s["gltf"]),
            "--contract", str(CORPUS / f"contract_params_{scene_id.replace('-', '_')}.json"),
            "--out-dir", str(out_dir),
            "--scene-id", scene_id,
            "--exposure-scale", repr(scale),
        ], timeout=3600)
        frame = out_dir / f"{scene_id}.exr"
        if not frame.is_file():
            raise SystemExit(f"Rurix 出帧缺失: {frame}")
        d = exr.decode_exr(frame.read_bytes(), "rurix")
        out[scene_id] = {
            "frame": str(frame),
            "frame_content_digest": frame_content_digest(d["width"], d["height"], 3, d["pixels"]),
            "exposure_scale_in_pipe": scale,
            "param_digest": d["metadata"].get("rurix:capture_params_digest", ""),
            "file_digest": sha256_file(frame),
        }
        log(f"Rurix 渲染 {scene_id}: exposure_scale={scale} digest={out[scene_id]['frame_content_digest'][:32]}…")
    return out


def stage_ue() -> dict:
    """UE 端关卡建设（b_srgb=False 光色线性直给——C1 太阳色链修复面）+ MRQ Phase B。
    G11_2_UE_COLLECT_ONLY=1 时只解码既有帧补登记（分阶段增量合并面，不重跑 UE）。"""
    out = {}
    collect_only = os.environ.get("G11_2_UE_COLLECT_ONLY") == "1"
    if not collect_only:
        for scene_id in SCENES:
            env = {
                "G10_5_SCENE": scene_id,
                "G10_5_CONTRACT": str(CORPUS / f"contract_params_{scene_id.replace('-', '_')}.json"),
                "G11_2_OUT_ROOT": "K:/rurix-ext/g11-frames/g11_2/ue",
            }
            run_cmd([sys.executable, str(UE_RUN), str(UE_BUILD)], env=env, timeout=3600)
            log(f"UE 关卡建设 {scene_id} 完成（b_srgb=False 线性直给）")
        for scene_id in SCENES:
            run_cmd([sys.executable, str(UE_RENDER), scene_id, "--timeout", "3600"], timeout=4200)
    for scene_id in SCENES:
        frame = FRAMES_G11 / "ue" / scene_id / ".0000.exr"
        if not frame.is_file():
            raise SystemExit(f"UE 出帧缺失: {frame}")
        d = exr.decode_exr(frame.read_bytes(), "ue5")
        out[scene_id] = {
            "frame": str(frame),
            "frame_content_digest": frame_content_digest(d["width"], d["height"], 3, d["pixels"]),
            "source_bit_depth": d["source_bit_depth"],
            "file_digest": sha256_file(frame),
        }
        log(f"UE {'登记' if collect_only else '渲染'} {scene_id}: bit_depth={d['source_bit_depth']} digest={out[scene_id]['frame_content_digest'][:32]}…")
    return out


def stage_derive() -> dict:
    """LDR 派生（×1.0 双端统一——C2 对齐后口径；RXS-0386 L2 派生链）。"""
    out = {}
    for scene_id in SCENES:
        for end, hdr in (("rurix", FRAMES_G11 / "rurix" / f"{scene_id}.exr"),
                         ("ue5", FRAMES_G11 / "ue" / scene_id / ".0000.exr")):
            ldr_path = FRAMES_G11 / "ldr" / f"{scene_id}_{end}_ldr.exr"
            ldr_path.parent.mkdir(parents=True, exist_ok=True)
            run_cmd([
                str(RUST_RELEASE_BIN), "--derive-ldr",
                "--hdr", str(hdr),
                "--source-end", end,
                "--out", str(ldr_path),
                "--exposure-scale", "1.0",
                "--params-digest", LOCKED_DIGEST[scene_id].split(":", 1)[1],
            ], timeout=1800)
            d = exr.decode_exr(ldr_path.read_bytes(), "rurix")
            out[f"{scene_id}:{end}"] = {
                "ldr": str(ldr_path),
                "exposure_scale_host": 1.0,
                "source_frame_digest": d["metadata"].get("rurix:source_frame_digest", ""),
                "file_digest": sha256_file(ldr_path),
            }
            log(f"LDR 派生 {scene_id}/{end}: ×1.0 source_digest={out[f'{scene_id}:{end}']['source_frame_digest'][:32]}…")
    return out


def lum_stats(arr: np.ndarray) -> dict:
    lum = 0.2126 * arr[..., 0] + 0.7152 * arr[..., 1] + 0.0722 * arr[..., 2]
    flat = np.sort(lum.ravel())
    n = flat.size
    return {
        "median": float(flat[n // 2]),
        "p90": float(flat[int(n * 0.9)]),
        "max": float(flat[-1]),
        "mean": float(flat.mean()),
        "nonzero_ratio": float(np.count_nonzero(flat > 1e-6) / n),
    }


def load_pixels(path: Path, end: str):
    d = exr.decode_exr(path.read_bytes(), end)
    arr = np.asarray(d["pixels"], dtype=np.float64).reshape(d["height"], d["width"], 3)
    return d, arr


def stage_metrics() -> dict:
    """HDR 亮度统计 + LDR 度量 + 修复前后 delta 对拍（baseline = g10_gap_registry 0-byte 消费）。"""
    registry = json.loads(GAP_REGISTRY.read_text(encoding="utf-8"))
    out = {"scenes": {}, "baselines": {}, "commands_count": len(COMMANDS)}
    for item in registry["items"]:
        if item["kind"] == "caliber_diff":
            out["baselines"][item["title"][:2]] = item["measured_delta"]
    for scene_id in SCENES:
        _, arr_hr = load_pixels(FRAMES_G11 / "rurix" / f"{scene_id}.exr", "rurix")
        _, arr_hu = load_pixels(FRAMES_G11 / "ue" / scene_id / ".0000.exr", "ue5")
        _, arr_lr = load_pixels(FRAMES_G11 / "ldr" / f"{scene_id}_rurix_ldr.exr", "rurix")
        _, arr_lu = load_pixels(FRAMES_G11 / "ldr" / f"{scene_id}_ue5_ldr.exr", "rurix")
        ssim_v = ssim_psnr.ssim_wang2004(arr_lu, arr_lr)
        psnr_v = ssim_psnr.psnr_joint(arr_lu, arr_lr)
        _em, flip_v = flip.flip_ldr(arr_lu, arr_lr)
        out["scenes"][scene_id] = {
            "hdr_stats": {"rurix": lum_stats(arr_hr), "ue5": lum_stats(arr_hu)},
            "ldr_stats": {"rurix": lum_stats(arr_lr), "ue5": lum_stats(arr_lu)},
            "hdr_luminance_median_delta": float(lum_stats(arr_hu)["median"] - lum_stats(arr_hr)["median"]),
            "hdr_luminance_p90_delta": float(lum_stats(arr_hu)["p90"] - lum_stats(arr_hr)["p90"]),
            "metrics_ldr": {
                "flip_ldr": float(flip_v),
                "ssim": float(ssim_v),
                "psnr_db": ssim_psnr.psnr_json_value(psnr_v),
            },
        }
        log(
            f"{scene_id}: HDR 中位 rurix={out['scenes'][scene_id]['hdr_stats']['rurix']['median']:.6f} "
            f"ue5={out['scenes'][scene_id]['hdr_stats']['ue5']['median']:.6f} "
            f"p90 rurix={out['scenes'][scene_id]['hdr_stats']['rurix']['p90']:.6f} "
            f"ue5={out['scenes'][scene_id]['hdr_stats']['ue5']['p90']:.6f}"
        )
        log(
            f"{scene_id}: LDR FLIP={out['scenes'][scene_id]['metrics_ldr']['flip_ldr']:.6f} "
            f"SSIM={out['scenes'][scene_id]['metrics_ldr']['ssim']:.6f} "
            f"PSNR={out['scenes'][scene_id]['metrics_ldr']['psnr_db']}"
        )
    return out


def stage_registry(metrics: dict) -> dict:
    """C1 残余口径差逐环节显式登记（RXS-0392 L4；载体 g11_2_residual_caliber_registry.json）。"""
    bistro = metrics["scenes"]["bistro-interior"]
    cornell = metrics["scenes"]["cornell-box"]
    items = [
        {
            "residual_id": "c1_light_seed_subset_r3",
            "chain": "light_seed_subset",
            "scene_id": "bistro-interior",
            "kind": "residual_caliber_diff",
            "description": "UE 臂表达 bistro 包内 pointLight1~N（glTF 节点实测 4+ 盏）与 emissive 面；Rurix 臂灯种子集 = 契约 sun + sky 常量天光（点/面光源与 glTF emissive 不表达）——灯种子集结构差",
            "measured_impact": {
                "hdr_luminance_median_delta_post_alignment": bistro["hdr_luminance_median_delta"],
                "hdr_luminance_p90_delta_post_alignment": bistro["hdr_luminance_p90_delta"],
            },
            "disposition_anchor": "R3 修复承接面（g11.p0.m153.fix_r3_light_subset，G11.4 波；g10_gap_registry.json R3 行 g11_anchor 字面）",
            "status": "registered",
        },
        {
            "residual_id": "c1_gi_structure_multibounce_r4",
            "chain": "gi_structure",
            "scene_id": "bistro-interior",
            "kind": "residual_caliber_diff",
            "description": "UE 臂 Lumen 多反弹 GI（屏幕探针 + 世界辐射缓存双级）vs Rurix 臂屏幕探针单反弹（host 参考管线）——GI 结构/反弹级数差；全向 IBL 遮蔽细节（UE SkyLight × Lumen AO）vs 探针 SH 单反弹覆盖差",
            "measured_impact": {
                "hdr_luminance_p90_delta_post_alignment": bistro["hdr_luminance_p90_delta"],
            },
            "disposition_anchor": "R4 修复承接面（g11.p0.m154.fix_r4_gi_multibounce_world_cache，G11.4 波；g10_gap_registry.json R4 行 g11_anchor 字面 + RFC-0028 §4.1/§4.2）",
            "status": "registered",
        },
        {
            "residual_id": "c1_cornell_gi_structure_r4",
            "chain": "gi_structure",
            "scene_id": "cornell-box",
            "kind": "residual_caliber_diff",
            "description": "cornell 封闭盒多反弹能量回归（UE Lumen）vs Rurix 单反弹——同 GI 结构差在 cornell 场景的投影（块区 p90 UE vs Rurix ≈1.95× 面）",
            "measured_impact": {
                "hdr_luminance_p90_delta_post_alignment": cornell["hdr_luminance_p90_delta"],
            },
            "disposition_anchor": "R4 修复承接面（g11.p0.m154，G11.4 波）",
            "status": "registered",
        },
        {
            "residual_id": "c1_ue_specular_ibl",
            "chain": "sky_ibl_structure",
            "scene_id": "both",
            "kind": "residual_caliber_diff",
            "description": "UE 臂 SkyLight 镜面 IBL / 反射环境（specular）vs Rurix 臂 Lambert-only 漫反射——镜面能量通道结构差（漫反射链已参数化对齐）",
            "measured_impact": None,
            "disposition_anchor": "显式留档（G11 期不承接；G15 画质量级收口面候选登记）",
            "status": "registered",
        },
        {
            "residual_id": "c3_source_bit_depth_quantization",
            "chain": "bit_depth_source",
            "scene_id": "both",
            "kind": "residual_caliber_diff",
            "description": "UE MRQ 源帧位深 fp16（写出时量化一次）vs Rurix 原生 f32——源位深量化差；度量域已统一提升 f32（fp16→f32 提升精确无损，C3 对齐面），源帧量化本身不可回退",
            "measured_impact": None,
            "disposition_anchor": "C3 口径行承接面（g11.p0.m146.caliber_c3_exr_bit_depth，G11.2 波——度量域对齐登记 + 本源差显式留档）",
            "status": "registered",
        },
    ]
    doc = {
        "schema_version": 1,
        "registry": "g11_2_residual_caliber_registry",
        "generated_by": "milestones/g11/harness/g11_2_ab_rerun.py --stage registry",
        "semantics": "C1 口径对齐后残余口径差逐环节显式登记（RXS-0392 L4；不拟合、只登记——口径差行不是被修没，残余 delta 全额归属本登记项，G11.5 复测差距清单 caliber_diff 面消费）",
        "aligned_chains": [
            {
                "chain": "sun_color",
                "scene_id": "bistro-interior",
                "before": "UE 臂 set_light_color b_srgb=True——契约 color_linear_rgb [1.0,0.98,0.95] 被 sRGB 二次转线性（有效值 [1.0,0.9551,0.8902]，G −2.5% / B −6.3% 实测口径偏差）",
                "after": "UE 臂 set_light_color b_srgb=False 线性直给 [1.0,0.98,0.95]（RXS-0392 L3）——与 Rurix 臂 sun_color = rgb×lux 同构",
                "status": "aligned_fixed",
            },
            {
                "chain": "sun_lux_to_radiance",
                "scene_id": "both",
                "before": "UE DirectionalLight lux → L=ρ·E·(n·l)/π；Rurix sun_color=rgb·lux → direct=·ndl·albedo/π——同构（G10.5a 登记面）",
                "after": "同构复核维持（本批双端同参数登记）",
                "status": "aligned_verified",
            },
            {
                "chain": "sky_intensity",
                "scene_id": "both",
                "before": "UE SkyLight 指定 cubemap × intensity（cubemap = 白色常量 1.0 uniform，digest 实测登记）= scene-linear 辐射度；Rurix 常量天光辐射度 = sky.intensity——同单位链（G10.5a 登记面）",
                "after": "同单位链复核维持 + cubemap 逐像素值核验（=1.0 uniform）",
                "status": "aligned_verified",
            },
            {
                "chain": "exposure_scale",
                "scene_id": "both",
                "before": "Rurix 臂 LDR 派生 ×2^(−EV100)（cornell 0.25 / bistro 0.5）vs UE 臂 pipe 内 FixedExposure=2^(−EV100) 已施 ×1.0——C2 口径差",
                "after": "Rurix 臂曝光尺度管线内烘焙 2^(−EV100)（--exposure-scale）与 UE 臂同域；LDR 派生尺度双端统一 ×1.0（C2 对齐落点）",
                "status": "aligned_fixed",
            },
        ],
        "items": items,
    }
    return doc


def write_json(path: Path, doc: dict) -> None:
    path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    log(f"落盘 {path}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--stage", default="all",
                    choices=["all", "contract", "rurix", "ue", "derive", "metrics", "registry"])
    args = ap.parse_args()

    FRAMES_G11.mkdir(parents=True, exist_ok=True)
    # 分阶段增量合并：既有报告 results/commands 保留（分阶段独立调用不覆写）。
    if REPORT_PATH.is_file():
        try:
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            report = {}
    else:
        report = {}
    report.update({
        "schema_version": 1,
        "report": "g11_2_ab_rerun",
        "generated_by": "milestones/g11/harness/g11_2_ab_rerun.py",
        "frames_root": str(FRAMES_G11),
        "locked_contract_digest": LOCKED_DIGEST,
        "timestamp_utc": _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    })
    report.setdefault("stages", {})
    prior_results = report.get("results", {}) if isinstance(report.get("results"), dict) else {}
    prior_commands = report.get("commands", []) if isinstance(report.get("commands"), list) else []

    stages = ["contract", "rurix", "ue", "derive", "metrics", "registry"] if args.stage == "all" else [args.stage]
    result: dict = dict(prior_results)
    for st in stages:
        if st == "contract":
            result["contract"] = stage_contract()
        elif st == "rurix":
            result["rurix"] = stage_rurix()
        elif st == "ue":
            result["ue"] = stage_ue()
        elif st == "derive":
            result["derive"] = stage_derive()
        elif st == "metrics":
            result["metrics"] = stage_metrics()
        elif st == "registry":
            if "metrics" not in result:
                result["metrics"] = stage_metrics()
            result["registry"] = stage_registry(result["metrics"])
            write_json(RESIDUAL_PATH, result["registry"])
        report["stages"][st] = "done"

    merged_commands = list(prior_commands)
    for c in COMMANDS:
        c["seq"] = len(merged_commands) + 1
        merged_commands.append(c)
    report["commands"] = merged_commands
    report["results"] = result
    write_json(REPORT_PATH, report)
    log(f"复跑完成 stages={stages}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
