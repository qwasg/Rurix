#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.4 波）
"""G11.4 光照与 GI 修复波 双端 A/B 复跑驱动（G11_CONTRACT §4.2 M153/M154 +
G-G11-6；g10_gap_registry R3/R4 行承接锚字面消费；spec/global_illumination.md
RXS-0394/0395/0396 + spec/visual_comparison.md RXS-0393 修复闭环判据）。

链路面（与 G11.2/G11.3 同构；G10/G11.2/G11.3 帧库只读——本批全部产物落
G11.4 帧区 K:/rurix-ext/g11-frames/g11_4/）：

  契约参数（digest == G10.5 锁定值 0-byte，三面绑定机核）→
  ├─ 派生链：g11_4_light_derive.py（bistro pointLight1~4 + emissive 四件 →
  │   corpus/lighting_bistro_interior.json 只追加修订 + 清单修订行，M133 程序；
  │   幂等复跑）→
  ├─ Rurix 端：g10_5_scene_render --render --exposure-scale 2^(−EV100) +
  │   修复旗标——bistro m153 面 = `--material-pbr --light-seed-set`（R3 隔离
  │   测量面）/ m154 面 = 加 `--gi-multibounce`（R4 世界缓存多反弹）；cornell
  │   = `--smooth-normals --gi-multibounce`（R4 cornell 投影面，灯面 0-byte）；
  │   parity 面 = 双场景无旗标（== G10.5 锁定 digest 回归锚）→ HDR EXR +
  │   stdout JSON 闭集块（lights/world_cache 计数面）
  └─ UE 端：cornell = G11.3 帧 digest 逐位核验复用（UE 臂 0-byte——cornell
      灯面/场景面零改动）；bistro = g10_5_build_scenes.py G11.4 段（契约光照
      JSON 点光源 4 盏 spawn + 读回探针）+ MRQ Phase B 重出（双端同消费
      契约面，RXS-0394 L2）
  双端 HDR → LDR 派生（×1.0 双端统一，C2 对齐后口径）→ HDR 覆盖/亮度统计 +
  FLIP/SSIM/PSNR → g11_4_rerun_report.json（修复前后 delta 对账面：锁定基线
  = g10_gap_registry 0-byte 消费 + G11.2 域统一换算面；R1 g11.5 耦合面复核
  实测登记——为 M155 收敛断言备料）。

G11 不设绝对画质通过线：全部数字 measured_local 登记，收敛判定归门机核。

用法：
  py -3 milestones/g11/harness/g11_4_ab_rerun.py --stage all
  py -3 milestones/g11/harness/g11_4_ab_rerun.py --stage contract|derive_lights|rurix|ue|derive|metrics|closure
  G11_4_UE_COLLECT_ONLY=1 时 ue 段只解码既有帧补登记（不重跑 UE）。
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import re
import shutil
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
FRAMES_G11 = Path(r"K:\rurix-ext\g11-frames\g11_4")
FRAMES_G11_3 = Path(r"K:\rurix-ext\g11-frames\g11_3")
REPORT_PATH = ROOT / "milestones" / "g11" / "g11_4_rerun_report.json"
REPORT_G11_3 = ROOT / "milestones" / "g11" / "g11_3_rerun_report.json"
RUST_RELEASE_BIN = ROOT / "target" / "release" / "g10_5_scene_render.exe"
UE_RUN = ROOT / "milestones" / "g10" / "harness" / "g10_5_ue_run.py"
UE_BUILD = ROOT / "milestones" / "g10" / "harness" / "ue_python" / "g10_5_build_scenes.py"
UE_RENDER = ROOT / "milestones" / "g10" / "harness" / "g10_5_ue_render.py"
GEN_PARAMS = ROOT / "milestones" / "g10" / "harness" / "g10_5_gen_contract_params.py"
LIGHT_DERIVE = ROOT / "milestones" / "g11" / "harness" / "g11_4_light_derive.py"
LIGHTING_BISTRO = CORPUS / "lighting_bistro_interior.json"
LIGHTING_CORNELL = CORPUS / "lighting_cornell_box.json"

# G10.5 锁定契约 digest（机核事实源 = evidence/g10_m130_dual_determinism_contract_
# 20260815T233315Z.json 登记值；RXS-0393 L4 转引字面）。
LOCKED_DIGEST = {
    "cornell-box": "sha256:80305791a68ccc66c5b046efaf193244796b52570494cf00aa1c86efa55be118",
    "bistro-interior": "sha256:ad45951ba641106b24e7d91d49ebf5992fb6a42cb70a3082520e8de19a6cf514",
}

# G10.5a 锁定帧内容 digest（默认面 parity 对账锚；g10_5_ab_preview.md §3 登记面）。
G10_5_FRAME_DIGEST = {
    ("rurix", "cornell-box"): "sha256:c2000ebfbe90359d55e668f8af3b7df24d64c3f72e637904f614821b7ad0d727",
    ("rurix", "bistro-interior"): "sha256:8519cc67c917e7b8c2c5a9bb5633ea5ee9e72deb8cf63b3b187b0d3ac5bb9935",
}

SCENES = {
    "cornell-box": {
        "gltf": Path(r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf"),
        "ev100": 2.0,
        "res": (512, 512),
        # m154 面：cornell 契约 sun+sky 灯面 0-byte（无 --light-seed-set）。
        "rurix_flags": ["--smooth-normals", "--gi-multibounce"],
    },
    "bistro-interior": {
        "gltf": Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf"),
        "ev100": 1.0,
        "res": (1920, 1080),
        "rurix_flags": ["--material-pbr", "--light-seed-set", str(LIGHTING_BISTRO), "--gi-multibounce"],
        # m153 面（R3 隔离测量）：材质 + 灯种子集，不多反弹。
        "rurix_flags_m153": ["--material-pbr", "--light-seed-set", str(LIGHTING_BISTRO)],
    },
}

COMMANDS: list[dict] = []


def log(msg: str) -> None:
    print(f"[g11_4_rerun] {msg}", flush=True)


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


def stage_derive_lights() -> dict:
    """R3 派生链（M133 只追加修订程序；幂等复跑）+ cornell 灯面 0-byte 对账。"""
    cornell_pre = sha256_file(LIGHTING_CORNELL)
    r = run_cmd([sys.executable, str(LIGHT_DERIVE)])
    cornell_post = sha256_file(LIGHTING_CORNELL)
    if cornell_pre != cornell_post:
        raise SystemExit("cornell 契约灯面漂移（bistro 灯面表达回流即 RED）")
    report = json.loads((ROOT / "milestones" / "g11" / "g11_4_light_derivation.json").read_text(encoding="utf-8"))
    out = {
        "lighting_bistro_digest": sha256_file(LIGHTING_BISTRO),
        "lighting_cornell_digest": cornell_post,
        "cornell_light_face_0byte": True,
        "point_lights": len(report["point_lights"]),
        "emissive_surfaces": len(report["emissive_surfaces"]),
        "derivation_report_digest": sha256_file(ROOT / "milestones" / "g11" / "g11_4_light_derivation.json"),
    }
    log(f"R3 派生链: point_lights={out['point_lights']} emissive={out['emissive_surfaces']}；cornell 灯面 0-byte ✓")
    return out


def _render_one(scene_id: str, out_name: str, flags: list[str], exposure: float | None = None,
                out_dir: Path | None = None) -> dict:
    s = SCENES[scene_id]
    # exposure=None → 管线内烘焙 2^(−EV100)（C2 对齐面）；parity 面传 1.0
    #（G10.5 锁定 digest = 无 --exposure-scale 旗标面，G11.3 parity 同口径）。
    scale = 2.0 ** (-s["ev100"]) if exposure is None else exposure
    out_dir = out_dir or (FRAMES_G11 / "rurix")
    out_dir.mkdir(parents=True, exist_ok=True)
    r = run_cmd([
        str(RUST_RELEASE_BIN), "--render",
        "--gltf", str(s["gltf"]),
        "--contract", str(CORPUS / f"contract_params_{scene_id.replace('-', '_')}.json"),
        "--out-dir", str(out_dir),
        "--scene-id", scene_id,
        "--exposure-scale", repr(scale),
        *flags,
    ], timeout=3600)
    frame = out_dir / f"{scene_id}.exr"
    if out_name != scene_id:
        # 多面隔离：异名面移出（防覆盖同名 canonical 帧——同名覆写即 RED 面）。
        frame2 = out_dir / f"{out_name}.exr"
        if frame2.exists():
            frame2.unlink()
        frame.rename(frame2)
        frame = frame2
    d = exr.decode_exr(frame.read_bytes(), "rurix")
    json_lines = [l for l in r.stdout.splitlines() if l.startswith('{"scene_id"')]
    if not json_lines:
        raise SystemExit(f"Rurix 渲染 {scene_id}/{out_name} 缺 stdout JSON 登记行")
    line = json_lines[-1]
    m_anim = re.search(r'"animations":(\{[^}]*\})', line)
    m_mats = re.search(r'"materials":(\{.+\})\}\s*$', line)
    m_lights = re.search(r'"lights":(\{.*?\}),"world_cache"', line)
    m_wc = re.search(r'"world_cache":(\{.*?\}),"materials"', line)
    m_fdigest = re.search(r'"frame_content_digest":"(sha256:[0-9a-f]{64})"', line)
    if not (m_anim and m_mats and m_lights and m_wc and m_fdigest):
        raise SystemExit(f"Rurix 渲染 {scene_id}/{out_name} 登记行闭集块抽取失败")
    return {
        "frame": str(frame),
        "frame_content_digest": frame_content_digest(d["width"], d["height"], 3, d["pixels"]),
        "exposure_scale_in_pipe": scale,
        "rurix_flags": list(flags),
        "param_digest": d["metadata"].get("rurix:capture_params_digest", ""),
        "file_digest": sha256_file(frame),
        "render_json": {
            "animations": json.loads(m_anim.group(1)),
            "materials": json.loads(m_mats.group(1)),
            "lights": json.loads(m_lights.group(1)),
            "world_cache": json.loads(m_wc.group(1)),
            "frame_content_digest": m_fdigest.group(1),
        },
    }


def stage_rurix() -> dict:
    """Rurix 端渲染：bistro 双面（m153 隔离面先渲 + m154 全修复面后渲——异名面
    移出防覆盖）+ cornell m154 面 + parity 双场景无旗标面（独立目录，
    == G10.5 锁定 digest 回归锚）。"""
    out = {}
    # m153 面先渲（R3 隔离测量；bistro 材质+灯种子集不多反弹；移出异名帧）。
    out["bistro-interior-m153"] = _render_one(
        "bistro-interior", "bistro-interior-m153", SCENES["bistro-interior"]["rurix_flags_m153"]
    )
    log(f"Rurix bistro（m153 面）: digest={out['bistro-interior-m153']['frame_content_digest'][:32]}…")
    for scene_id, s in SCENES.items():
        out[scene_id] = _render_one(scene_id, scene_id, s["rurix_flags"])
        log(f"Rurix {scene_id}（m154 面）: digest={out[scene_id]['frame_content_digest'][:32]}…")
    # parity 面（无旗标 + 无曝光烘焙 == G10.5 锁定 digest；独立目录零覆盖）。
    for scene_id in SCENES:
        pf = _render_one(scene_id, scene_id, [], exposure=1.0,
                         out_dir=FRAMES_G11 / "parity")
        ok = pf["frame_content_digest"] == G10_5_FRAME_DIGEST[("rurix", scene_id)]
        out[f"parity:{scene_id}"] = {
            "frame_content_digest": pf["frame_content_digest"],
            "g10_5_locked_digest": G10_5_FRAME_DIGEST[("rurix", scene_id)],
            "parity_bitexact": ok,
        }
        if not ok:
            raise SystemExit(f"默认面 parity 漂移（{scene_id}）——G10.5 锁定 digest 不等，旗标默认关面被破坏")
        log(f"Rurix {scene_id} parity: == G10.5 锁定 digest ✓")
    return out


def stage_ue() -> dict:
    """UE 端：cornell = G11.3 帧 digest 逐位核验复用（UE 臂 0-byte）；bistro =
    G11.4 关卡建设（点光源 spawn 探针）+ MRQ Phase B 重出。"""
    out = {}
    collect_only = os.environ.get("G11_4_UE_COLLECT_ONLY") == "1"
    probe_dir = FRAMES_G11 / "probe"
    probe_dir.mkdir(parents=True, exist_ok=True)
    # cornell：复用核验（G11.3 UE 帧 digest == g11_3 报告登记值）。
    g113_ue_digest = REPORT_G11_3 and json.loads(REPORT_G11_3.read_text(encoding="utf-8"))["results"]["ue"]["cornell-box"]["frame_content_digest"]
    src = FRAMES_G11_3 / "ue" / "cornell-box" / ".0000.exr"
    d = exr.decode_exr(src.read_bytes(), "ue5")
    got = frame_content_digest(d["width"], d["height"], 3, d["pixels"])
    if got != g113_ue_digest:
        raise SystemExit(f"cornell UE 复用帧 digest 漂移: {got} ≠ G11.3 登记 {g113_ue_digest}")
    dst = FRAMES_G11 / "ue" / "cornell-box"
    dst.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src, dst / ".0000.exr")
    out["cornell-box"] = {
        "frame": str(dst / ".0000.exr"),
        "frame_content_digest": got,
        "source_bit_depth": d["source_bit_depth"],
        "file_digest": sha256_file(dst / ".0000.exr"),
        "reuse_from": "g11_3（UE 臂 0-byte：cornell 灯面/场景面零改动，digest 逐位核验复用）",
        "probe": {},
    }
    log(f"UE cornell（复用核验）: digest={got[:32]}… == G11.3 登记 ✓")
    # bistro：G11.4 关卡建设 + MRQ 重出。
    if not collect_only:
        env = {
            "G10_5_SCENE": "bistro-interior",
            "G10_5_CONTRACT": str(CORPUS / "contract_params_bistro_interior.json"),
            "G11_2_OUT_ROOT": "K:/rurix-ext/g11-frames/g11_4/ue",
            "G11_3_PROBE_OUT": str(probe_dir / "bistro-interior_probe.json"),
        }
        run_cmd([sys.executable, str(UE_RUN), str(UE_BUILD)], env=env, timeout=3600)
        log("UE 关卡建设 bistro-interior 完成（G11.4 点光源面）")
        run_cmd([sys.executable, str(UE_RENDER), "bistro-interior", "--timeout", "3600"], timeout=4200)
    frame = FRAMES_G11 / "ue" / "bistro-interior" / ".0000.exr"
    if not frame.is_file():
        raise SystemExit(f"UE 出帧缺失: {frame}")
    d = exr.decode_exr(frame.read_bytes(), "ue5")
    probe_path = probe_dir / "bistro-interior_probe.json"
    probe = json.loads(probe_path.read_text(encoding="utf-8")) if probe_path.is_file() else {}
    out["bistro-interior"] = {
        "frame": str(frame),
        "frame_content_digest": frame_content_digest(d["width"], d["height"], 3, d["pixels"]),
        "source_bit_depth": d["source_bit_depth"],
        "file_digest": sha256_file(frame),
        "probe_path": str(probe_path),
        "probe": probe,
    }
    log(f"UE {'登记' if collect_only else '渲染'} bistro: digest={out['bistro-interior']['frame_content_digest'][:32]}… g11_4_lights={probe.get('g11_4_point_lights_count')}")
    return out


def stage_derive() -> dict:
    """LDR 派生（×1.0 双端统一——C2 对齐后口径；RXS-0386 L2 派生链）。"""
    out = {}
    pairs = [
        ("cornell-box", "rurix", FRAMES_G11 / "rurix" / "cornell-box.exr"),
        ("cornell-box", "ue5", FRAMES_G11 / "ue" / "cornell-box" / ".0000.exr"),
        ("bistro-interior", "rurix", FRAMES_G11 / "rurix" / "bistro-interior.exr"),
        ("bistro-interior", "ue5", FRAMES_G11 / "ue" / "bistro-interior" / ".0000.exr"),
        ("bistro-interior-m153", "rurix", FRAMES_G11 / "rurix" / "bistro-interior-m153.exr"),
    ]
    for scene_id, end, hdr in pairs:
        ldr_path = FRAMES_G11 / "ldr" / f"{scene_id}_{end}_ldr.exr"
        ldr_path.parent.mkdir(parents=True, exist_ok=True)
        digest_scene = "bistro-interior" if "bistro" in scene_id else "cornell-box"
        run_cmd([
            str(RUST_RELEASE_BIN), "--derive-ldr",
            "--hdr", str(hdr),
            "--source-end", end,
            "--out", str(ldr_path),
            "--exposure-scale", "1.0",
            "--params-digest", LOCKED_DIGEST[digest_scene].split(":", 1)[1],
        ], timeout=1800)
        d = exr.decode_exr(ldr_path.read_bytes(), "rurix")
        out[f"{scene_id}:{end}"] = {
            "ldr": str(ldr_path),
            "exposure_scale_host": 1.0,
            "source_frame_digest": d["metadata"].get("rurix:source_frame_digest", ""),
            "file_digest": sha256_file(ldr_path),
        }
        log(f"LDR 派生 {scene_id}/{end}: ×1.0")
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
    0-byte 消费——R3 HDR 亮度中位 / R4 HDR 亮度 p90）+ R1 g11.5 耦合面复核实测。"""
    out = {"scenes": {}, "closure_faces": {}, "commands_count": len(COMMANDS)}
    scene_ends = [
        ("cornell-box", "cornell-box"),
        ("bistro-interior", "bistro-interior"),
        ("bistro-interior-m153", "bistro-interior"),
    ]
    for key, digest_scene in scene_ends:
        _, arr_hr = load_pixels(FRAMES_G11 / "rurix" / f"{key}.exr", "rurix")
        _, arr_hu = load_pixels(FRAMES_G11 / "ue" / digest_scene / ".0000.exr", "ue5")
        _, arr_lr = load_pixels(FRAMES_G11 / "ldr" / f"{key}_rurix_ldr.exr", "rurix")
        _, arr_lu = load_pixels(FRAMES_G11 / "ldr" / f"{digest_scene}_ue5_ldr.exr", "rurix")
        ssim_v = ssim_psnr.ssim_wang2004(arr_lu, arr_lr)
        psnr_v = ssim_psnr.psnr_joint(arr_lu, arr_lr)
        _em, flip_v = flip.flip_ldr(arr_lu, arr_lr)
        sr, su = lum_stats(arr_hr), lum_stats(arr_hu)
        lr, lu = lum_stats(arr_lr), lum_stats(arr_lu)
        out["scenes"][key] = {
            "hdr_stats": {"rurix": sr, "ue5": su},
            "ldr_stats": {"rurix": lr, "ue5": lu},
            "hdr_luminance_median_delta": float(su["median"] - sr["median"]),
            "hdr_luminance_p90_delta": float(su["p90"] - sr["p90"]),
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
            f"{key}: HDR 中位 rurix={sr['median']:.6f} ue5={su['median']:.6f}（delta {out['scenes'][key]['hdr_luminance_median_delta']:+.10f}）"
            f" p90 delta {out['scenes'][key]['hdr_luminance_p90_delta']:+.10f} SSIM={float(ssim_v):.6f}"
        )

    r3 = gap_row("R3")
    r4 = gap_row("R4")
    r1 = gap_row("R1")
    bistro = out["scenes"]["bistro-interior"]
    bistro153 = out["scenes"]["bistro-interior-m153"]
    cornell = out["scenes"]["cornell-box"]
    out["closure_faces"] = {
        "r3": {
            "gap_row_id": r3["gap_id"],
            "metric": "hdr_luminance_median@bistro-interior",
            "baseline_delta": r3["measured_delta"][0]["delta"],
            "baseline_a": r3["measured_delta"][0]["a_value"],
            "baseline_b": r3["measured_delta"][0]["b_value"],
            "baseline_aligned_domain": 2.7314592314362525,
            "retest_rurix_median": bistro153["hdr_stats"]["rurix"]["median"],
            "retest_ue5_median": bistro153["hdr_stats"]["ue5"]["median"],
            "retest_delta": bistro153["hdr_luminance_median_delta"],
            "retest_delta_m154_face": bistro["hdr_luminance_median_delta"],
        },
        "r4": {
            "gap_row_id": r4["gap_id"],
            "metric": "hdr_luminance_p90@bistro-interior",
            "baseline_delta": r4["measured_delta"][0]["delta"],
            "baseline_a": r4["measured_delta"][0]["a_value"],
            "baseline_b": r4["measured_delta"][0]["b_value"],
            "baseline_aligned_domain": 4.8486343559026714,
            "retest_rurix_p90": bistro["hdr_stats"]["rurix"]["p90"],
            "retest_ue5_p90": bistro["hdr_stats"]["ue5"]["p90"],
            "retest_delta": bistro["hdr_luminance_p90_delta"],
            "cornell_p90_delta_residual_face": cornell["hdr_luminance_p90_delta"],
        },
        # R1 的 g11.5 phase 耦合面复核（契约 §8.3a 备料）：G11.4 光照/GI 修复后
        # R1 局部 SSIM delta 是否仍被光照残余主导——实测登记（不冒充收敛断言）。
        "r1_coupling_recheck": {
            "gap_row_id": r1["gap_id"],
            "metric": "ssim@bistro-interior(ldr)",
            "baseline_delta": r1["measured_delta"][0]["delta"],
            "g11_3_retest_delta": 0.9903435577002249,
            "retest_ssim_m154_face": bistro["metrics_ldr"]["ssim"],
            "retest_delta_m154_face": bistro["metrics_ldr"]["ssim_delta_identity"],
            "retest_delta_m153_face": bistro153["metrics_ldr"]["ssim_delta_identity"],
        },
    }
    return out


def write_json(path: Path, doc: dict) -> None:
    path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    log(f"落盘 {path}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--stage", default="all",
                    choices=["all", "contract", "derive_lights", "rurix", "ue", "derive", "metrics", "closure"])
    args = ap.parse_args()

    FRAMES_G11.mkdir(parents=True, exist_ok=True)
    if REPORT_PATH.is_file():
        try:
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            report = {}
    else:
        report = {}
    report.update({
        "schema_version": 1,
        "report": "g11_4_ab_rerun",
        "generated_by": "milestones/g11/harness/g11_4_ab_rerun.py",
        "frames_root": str(FRAMES_G11),
        "locked_contract_digest": LOCKED_DIGEST,
        "timestamp_utc": _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    })
    report.setdefault("stages", {})
    prior_results = report.get("results", {}) if isinstance(report.get("results"), dict) else {}
    prior_commands = report.get("commands", []) if isinstance(report.get("commands"), list) else []

    stages = ["contract", "derive_lights", "rurix", "ue", "derive", "metrics"] if args.stage == "all" else (
        ["metrics"] if args.stage == "closure" else [args.stage])
    result: dict = dict(prior_results)
    for st in stages:
        if st == "contract":
            result["contract"] = stage_contract()
        elif st == "derive_lights":
            result["derive_lights"] = stage_derive_lights()
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
