#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波）
"""G11.3 资产与场景面修复波 双端 A/B 复跑驱动（G11_CONTRACT §4.2 M147~M152 + G-G11-5；
g10_gap_registry R1/R2/R5/U1/U2/U3 行承接锚字面消费；spec/visual_comparison.md
RXS-0393 修复闭环判据）。

链路面（与 G11.2 同构，G10/G11.2 帧库只读——本批全部产物落 G11.3 帧区
K:/rurix-ext/g11-frames/g11_3/）：

  契约参数（milestones/g10/corpus/contract_params_<scene>.json，digest == G10.5
  锁定值 0-byte——三面绑定机核）→
  ├─ Rurix 端：g10_5_scene_render --render --exposure-scale 2^(−EV100)（C2 对齐面
  │   维持）+ **修复旗标**——cornell `--smooth-normals`（R2：顶点平滑法线重心
  │   插值 + 逆矩阵转置世界化 + 双面翻转消费）/ bistro `--material-pbr`（R1：
  │   baseColorTexture〔bcdec DDS 真实解码〕× factor ×(1−metallic) 漫反射 +
  │   太阳 GGX + 法线贴图切线空间扰动；U2 Rurix 侧纹理消费面同旗标落地）→ HDR EXR
  │   + stdout JSON 闭集块（animations 显式剥离声明 / materials 消费登记）
  └─ UE 端：g10_5_build_scenes.py——cornell **壳体双面化**（U1：单面片外向绕向
      × UE 背面剔除口径对齐，two_sided 父材质 + 逐 actor MIC 置换，语料 0-byte）
      / bistro **DDS→PNG 派生链导入**（U2：UE Interchange 不消费 .dds 的绕行面，
      G10-N7 承接锚兑现；g11_3_dds_transcode_manifest.json 逐文件 digest 机核）
      → MRQ Phase B → HDR EXR + G11_3_PROBE_OUT 探针（双面置换 provenance /
      材质实例 texture_parameter_values 非空回归）
  双端 HDR → LDR 派生（×1.0 双端统一，C2 对齐后口径）→ HDR 覆盖/亮度统计 +
  FLIP/SSIM/PSNR（ci/g10_flip_lib.py / g10_ssim_psnr_lib.py 单一事实源）→
  g11_3_rerun_report.json（修复前后 delta 对账面：锁定基线 = g10_gap_registry
  0-byte 消费）。

G11 不设绝对画质通过线：全部数字 measured_local 登记，收敛判定归六门机核。

用法：
  py -3 milestones/g11/harness/g11_3_ab_rerun.py --stage all
  py -3 milestones/g11/harness/g11_3_ab_rerun.py --stage contract|rurix|ue|derive|metrics|closure
  G11_3_UE_COLLECT_ONLY=1 时 ue 段只解码既有帧补登记（不重跑 UE）。
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import re
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
FRAMES_G11 = Path(r"K:\rurix-ext\g11-frames\g11_3")
REPORT_PATH = ROOT / "milestones" / "g11" / "g11_3_rerun_report.json"
RUST_RELEASE_BIN = ROOT / "target" / "release" / "g10_5_scene_render.exe"
UE_RUN = ROOT / "milestones" / "g10" / "harness" / "g10_5_ue_run.py"
UE_BUILD = ROOT / "milestones" / "g10" / "harness" / "ue_python" / "g10_5_build_scenes.py"
UE_RENDER = ROOT / "milestones" / "g10" / "harness" / "g10_5_ue_render.py"
GEN_PARAMS = ROOT / "milestones" / "g10" / "harness" / "g10_5_gen_contract_params.py"
TRANSCODE_MANIFEST = ROOT / "milestones" / "g11" / "g11_3_dds_transcode_manifest.json"

# G10.5 锁定契约 digest（机核事实源 = evidence/g10_m130_dual_determinism_contract_
# 20260815T233315Z.json 登记值；RXS-0393 L4 转引字面）。
LOCKED_DIGEST = {
    "cornell-box": "sha256:80305791a68ccc66c5b046efaf193244796b52570494cf00aa1c86efa55be118",
    "bistro-interior": "sha256:ad45951ba641106b24e7d91d49ebf5992fb6a42cb70a3082520e8de19a6cf514",
}

# G11.3 修复旗标面（默认关 = G10.5 逐字节口径；复跑帧由旗标显式开启驱动——
# 承接锚字面：R2 = cornell 几何法线/双面；R1+U2 Rurix 侧 = bistro 材质/纹理）。
SCENES = {
    "cornell-box": {
        "gltf": Path(r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf"),
        "ev100": 2.0,
        "res": (512, 512),
        "rurix_flags": ["--smooth-normals"],
    },
    "bistro-interior": {
        "gltf": Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf"),
        "ev100": 1.0,
        "res": (1920, 1080),
        "rurix_flags": ["--material-pbr"],
    },
}

COMMANDS: list[dict] = []


def log(msg: str) -> None:
    print(f"[g11_3_rerun] {msg}", flush=True)


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
    """契约 digest 三面绑定机核（RXS-0393 L4）：重生成 + Rust 默认面 digest ==
    G10.5 锁定值（修复旗标默认关——默认面与 G10.5 逐字节一致）。"""
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
    """Rurix 端渲染（修复旗标面 + 曝光尺度管线内烘焙 2^(−EV100)——C2 对齐维持）。
    stdout 末行 JSON 闭集块（animations / materials）随帧登记进报告。"""
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
            *s["rurix_flags"],
        ], timeout=3600)
        frame = out_dir / f"{scene_id}.exr"
        if not frame.is_file():
            raise SystemExit(f"Rurix 出帧缺失: {frame}")
        # stdout 登记行消费面（G10 门序同口径）："frame" 字段含 Windows 反斜杠
        # 路径非严格 JSON——既有门（g10_ab_comparison/g10_stabilization_soak）
        # 均以正则抽取；本驱动同构抽取 animations/materials 闭集块（块内无
        # 反斜杠面：uri 取自 gltf 正斜杠，reason 为中文登记文本）。
        json_lines = [l for l in r.stdout.splitlines() if l.startswith('{"scene_id"')]
        if not json_lines:
            raise SystemExit(f"Rurix 渲染 {scene_id} 缺 stdout JSON 登记行")
        line = json_lines[-1]
        m_anim = re.search(r'"animations":(\{[^}]*\})', line)
        # materials 为登记行末块——贪婪取到行尾后剥掉根对象收尾 }。
        m_mats = re.search(r'"materials":(\{.+\})\}\s*$', line)
        m_fdigest = re.search(r'"frame_content_digest":"(sha256:[0-9a-f]{64})"', line)
        if not (m_anim and m_mats and m_fdigest):
            raise SystemExit(f"Rurix 渲染 {scene_id} 登记行闭集块抽取失败")
        render_json = {
            "animations": json.loads(m_anim.group(1)),
            "materials": json.loads(m_mats.group(1)),
            "frame_content_digest": m_fdigest.group(1),
        }
        d = exr.decode_exr(frame.read_bytes(), "rurix")
        out[scene_id] = {
            "frame": str(frame),
            "frame_content_digest": frame_content_digest(d["width"], d["height"], 3, d["pixels"]),
            "exposure_scale_in_pipe": scale,
            "rurix_flags": list(s["rurix_flags"]),
            "param_digest": d["metadata"].get("rurix:capture_params_digest", ""),
            "file_digest": sha256_file(frame),
            "render_json": render_json,
        }
        mats = render_json.get("materials", {})
        anims = render_json.get("animations", {})
        log(
            f"Rurix 渲染 {scene_id}: flags={s['rurix_flags']} digest={out[scene_id]['frame_content_digest'][:32]}… "
            f"tex_consumed={mats.get('textures_consumed')} anims={anims.get('package_count')}/{anims.get('channels')}"
        )
    return out


def stage_ue() -> dict:
    """UE 端关卡建设（U1 cornell 壳体双面化 / U2 bistro DDS→PNG 派生链导入）
    + MRQ Phase B + G11_3_PROBE_OUT 探针（双面置换 provenance / 材质纹理参数回归）。
    G11_3_UE_COLLECT_ONLY=1 时只解码既有帧与探针补登记（分阶段增量合并面）。"""
    out = {}
    collect_only = os.environ.get("G11_3_UE_COLLECT_ONLY") == "1"
    probe_dir = FRAMES_G11 / "probe"
    probe_dir.mkdir(parents=True, exist_ok=True)
    if not collect_only:
        for scene_id in SCENES:
            env = {
                "G10_5_SCENE": scene_id,
                "G10_5_CONTRACT": str(CORPUS / f"contract_params_{scene_id.replace('-', '_')}.json"),
                "G11_2_OUT_ROOT": "K:/rurix-ext/g11-frames/g11_3/ue",
                "G11_3_PROBE_OUT": str(probe_dir / f"{scene_id}_probe.json"),
            }
            run_cmd([sys.executable, str(UE_RUN), str(UE_BUILD)], env=env, timeout=3600)
            log(f"UE 关卡建设 {scene_id} 完成（G11.3 修复面）")
        for scene_id in SCENES:
            run_cmd([sys.executable, str(UE_RENDER), scene_id, "--timeout", "3600"], timeout=4200)
    for scene_id in SCENES:
        frame = FRAMES_G11 / "ue" / scene_id / ".0000.exr"
        if not frame.is_file():
            raise SystemExit(f"UE 出帧缺失: {frame}")
        d = exr.decode_exr(frame.read_bytes(), "ue5")
        probe_path = probe_dir / f"{scene_id}_probe.json"
        probe = json.loads(probe_path.read_text(encoding="utf-8")) if probe_path.is_file() else {}
        out[scene_id] = {
            "frame": str(frame),
            "frame_content_digest": frame_content_digest(d["width"], d["height"], 3, d["pixels"]),
            "source_bit_depth": d["source_bit_depth"],
            "file_digest": sha256_file(frame),
            "probe_path": str(probe_path),
            "probe": probe,
        }
        log(f"UE {'登记' if collect_only else '渲染'} {scene_id}: bit_depth={d['source_bit_depth']} digest={out[scene_id]['frame_content_digest'][:32]}… probe_keys={sorted(probe.keys())}")
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


def gap_row(title_prefix: str) -> dict:
    reg = json.loads(GAP_REGISTRY.read_text(encoding="utf-8"))
    for item in reg["items"]:
        if item["title"].startswith(title_prefix):
            return item
    raise KeyError(f"gap registry 缺行: {title_prefix}")


def stage_metrics() -> dict:
    """HDR 覆盖/亮度统计 + LDR 度量 + 修复前后 delta 对拍（baseline = g10_gap_registry
    0-byte 消费——R1 SSIM / R2·U1 HDR 覆盖 / U2 LDR 亮度中位 / U3 动画通道）。"""
    out = {"scenes": {}, "closure_faces": {}, "commands_count": len(COMMANDS)}
    for scene_id in SCENES:
        _, arr_hr = load_pixels(FRAMES_G11 / "rurix" / f"{scene_id}.exr", "rurix")
        _, arr_hu = load_pixels(FRAMES_G11 / "ue" / scene_id / ".0000.exr", "ue5")
        _, arr_lr = load_pixels(FRAMES_G11 / "ldr" / f"{scene_id}_rurix_ldr.exr", "rurix")
        _, arr_lu = load_pixels(FRAMES_G11 / "ldr" / f"{scene_id}_ue5_ldr.exr", "rurix")
        ssim_v = ssim_psnr.ssim_wang2004(arr_lu, arr_lr)
        psnr_v = ssim_psnr.psnr_joint(arr_lu, arr_lr)
        _em, flip_v = flip.flip_ldr(arr_lu, arr_lr)
        sr, su = lum_stats(arr_hr), lum_stats(arr_hu)
        lr, lu = lum_stats(arr_lr), lum_stats(arr_lu)
        out["scenes"][scene_id] = {
            "hdr_stats": {"rurix": sr, "ue5": su},
            "ldr_stats": {"rurix": lr, "ue5": lu},
            "hdr_nonzero_ratio_delta": float(su["nonzero_ratio"] - sr["nonzero_ratio"]),
            "ldr_luminance_median_delta": float(lu["median"] - lr["median"]),
            "metrics_ldr": {
                "flip_ldr": float(flip_v),
                "ssim": float(ssim_v),
                "ssim_delta_identity": float(1.0 - ssim_v),
                "psnr_db": ssim_psnr.psnr_json_value(psnr_v),
            },
        }
        log(
            f"{scene_id}: HDR 覆盖 rurix={sr['nonzero_ratio']:.6f} ue5={su['nonzero_ratio']:.6f} "
            f"（delta {out['scenes'][scene_id]['hdr_nonzero_ratio_delta']:+.10f}） "
            f"LDR 中位 rurix={lr['median']:.6f} ue5={lu['median']:.6f} SSIM={float(ssim_v):.6f}"
        )

    # 修复前后 delta 对账面（锁定基线 0-byte 转引；收敛判定归六门机核）。
    r1 = gap_row("R1")
    r2 = gap_row("R2")
    u1 = gap_row("U1")
    u2 = gap_row("U2")
    u3 = gap_row("U3")
    bistro = out["scenes"]["bistro-interior"]
    cornell = out["scenes"]["cornell-box"]
    out["closure_faces"] = {
        "r1": {
            "gap_row_id": r1["gap_id"],
            "metric": "ssim@bistro-interior(ldr)",
            "baseline_delta": r1["measured_delta"][0]["delta"],
            "baseline_a": r1["measured_delta"][0]["a_value"],
            "retest_ssim": bistro["metrics_ldr"]["ssim"],
            "retest_delta": bistro["metrics_ldr"]["ssim_delta_identity"],
        },
        "r2": {
            "gap_row_id": r2["gap_id"],
            "metric": "hdr_nonzero_ratio@cornell-box",
            "baseline_delta": r2["measured_delta"][0]["delta"],
            "baseline_a": r2["measured_delta"][0]["a_value"],
            "retest_rurix_nonzero": cornell["hdr_stats"]["rurix"]["nonzero_ratio"],
            "retest_ue5_nonzero": cornell["hdr_stats"]["ue5"]["nonzero_ratio"],
            "retest_delta": cornell["hdr_nonzero_ratio_delta"],
        },
        "u1": {
            "gap_row_id": u1["gap_id"],
            "metric": "hdr_nonzero_ratio@cornell-box",
            "baseline_delta": u1["measured_delta"][0]["delta"],
            "baseline_b": u1["measured_delta"][0]["b_value"],
            "retest_rurix_nonzero": cornell["hdr_stats"]["rurix"]["nonzero_ratio"],
            "retest_ue5_nonzero": cornell["hdr_stats"]["ue5"]["nonzero_ratio"],
            "retest_delta": cornell["hdr_nonzero_ratio_delta"],
        },
        "u2": {
            "gap_row_id": u2["gap_id"],
            "metric": "ldr_luminance_median@bistro-interior",
            "baseline_delta": u2["measured_delta"][0]["delta"],
            "baseline_a": u2["measured_delta"][0]["a_value"],
            "baseline_b": u2["measured_delta"][0]["b_value"],
            "retest_rurix_median": bistro["ldr_stats"]["rurix"]["median"],
            "retest_ue5_median": bistro["ldr_stats"]["ue5"]["median"],
            "retest_delta": bistro["ldr_luminance_median_delta"],
        },
        "u3": {
            "gap_row_id": u3["gap_id"],
            "metric": "gltf_animation_channels_unconsumed@bistro-interior",
            "baseline_delta": u3["measured_delta"][0]["delta"],
            "baseline_b": u3["measured_delta"][0]["b_value"],
        },
    }
    return out


def write_json(path: Path, doc: dict) -> None:
    path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    log(f"落盘 {path}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--stage", default="all",
                    choices=["all", "contract", "rurix", "ue", "derive", "metrics", "closure"])
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
        "report": "g11_3_ab_rerun",
        "generated_by": "milestones/g11/harness/g11_3_ab_rerun.py",
        "frames_root": str(FRAMES_G11),
        "locked_contract_digest": LOCKED_DIGEST,
        "transcode_manifest": str(TRANSCODE_MANIFEST),
        "timestamp_utc": _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    })
    report.setdefault("stages", {})
    prior_results = report.get("results", {}) if isinstance(report.get("results"), dict) else {}
    prior_commands = report.get("commands", []) if isinstance(report.get("commands"), list) else []

    stages = ["contract", "rurix", "ue", "derive", "metrics"] if args.stage == "all" else (
        ["metrics"] if args.stage == "closure" else [args.stage])
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
