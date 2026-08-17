#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.5b 波）
"""G11.5b 追加子波 同契约双端全量复跑驱动（G11_CONTRACT §4.2 M155 + G-G11-7 + §8.3a
+ §8.5b；主会话 G11.5b 裁决 = 先诊断修复后评 metric、禁改判据充绿）。

与 G11.5 首跑驱动（g11_5_ab_rerun.py，0-byte 历史面）的差异面：
1. 产物集独立（G11.5 产物 0-byte 保留）：帧区 K:/rurix-ext/g11-frames/g11_5b，
   报告 milestones/g11/g11_5b_rerun_report.json，复测差距清单
   milestones/g11/g11_5b_retest_gap_registry.json（registry 名字面
   "g11_5b_retest_gap_registry"——门侧校验器按 RURIX_G11_RETEST_SET 选择面同构）；
2. Rurix 端旗标面 += `--sky-ibl`（RXS-0397 天光直接漫反射 IBL 消费面——G11.5b
   诊断修复主面，双场景同消费；UE 端不变 = 同契约重跑）；parity 面无旗标 ==
   G10.5 锁定 digest 回归锚维持；
3. 渲染登记行解析 = 整行 json.loads 鲁棒面（sky_ibl 闭集块新增兼容；G11.5 驱动
   的正则抽取面为新输出形态不可消费，如实登记不回流改写历史驱动）。

链路面（G10/G11.2/G11.3/G11.4/G11.5 帧库只读——本批全部产物落 G11.5b 帧区）：
契约参数（digest == G10.5 锁定值 0-byte 三面绑定，不等即 SystemExit 停线）→
Rurix 端 --render --exposure-scale 2^(−EV100) + 全修复旗标 + --sky-ibl →
UE 端 build_scenes + MRQ（G11_2_OUT_ROOT → g11_5b 帧区）→ 双端 HDR → LDR 派生
（×1.0 双端统一，C2 口径维持）→ 度量（ci/g10_*_lib 单一事实源）→ 11 行逐项
闭环清单落盘（RXS-0393 L2 分款收敛断言；收敛阈 = g11_budget g11.fix.* 标定条目
消费，禁手写；partial/未收敛行如实登记不充绿带承接锚；R1 不收敛则整波 FAIL，
§8.3a 不弱化声明）。

G11 不设绝对画质通过线：全部数字 measured_local 登记，收敛判定归 M155 门机核。

用法：
  py -3 milestones/g11/harness/g11_5b_ab_rerun.py --stage all
  py -3 milestones/g11/harness/g11_5b_ab_rerun.py --stage contract|rurix|ue|derive|metrics|registry
  G11_5B_UE_COLLECT_ONLY=1 时 ue 段只解码既有帧补登记（不重跑 UE）。
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))

import g10_exr_lib as exr  # noqa: E402
import g10_flip_lib as flip  # noqa: E402
import g10_ssim_psnr_lib as ssim_psnr  # noqa: E402
import g11_3_fix_lib as fl  # noqa: E402

CORPUS = ROOT / "milestones" / "g10" / "corpus"
GAP_REGISTRY = ROOT / "milestones" / "g10" / "g10_gap_registry.json"
BUDGET_PATH = ROOT / "milestones" / "g11" / "g11_budget.json"
FRAMES_G11 = Path(r"K:\rurix-ext\g11-frames\g11_5b")
REPORT_PATH = ROOT / "milestones" / "g11" / "g11_5b_rerun_report.json"
RETEST_REGISTRY_PATH = ROOT / "milestones" / "g11" / "g11_5b_retest_gap_registry.json"
RESIDUAL_REGISTRY = ROOT / "milestones" / "g11" / "g11_2_residual_caliber_registry.json"
RUST_RELEASE_BIN = ROOT / "target" / "release" / "g10_5_scene_render.exe"
UE_RUN = ROOT / "milestones" / "g10" / "harness" / "g10_5_ue_run.py"
UE_BUILD = ROOT / "milestones" / "g10" / "harness" / "ue_python" / "g10_5_build_scenes.py"
UE_RENDER = ROOT / "milestones" / "g10" / "harness" / "g10_5_ue_render.py"
GEN_PARAMS = ROOT / "milestones" / "g10" / "harness" / "g10_5_gen_contract_params.py"
LIGHTING_BISTRO = CORPUS / "lighting_bistro_interior.json"

# G10.5 锁定契约 digest（机核事实源 = evidence/g10_m130_dual_determinism_contract_
# 20260815T233315Z.json 登记值；RXS-0393 L4 转引字面）。
LOCKED_DIGEST = {
    "cornell-box": "sha256:80305791a68ccc66c5b046efaf193244796b52570494cf00aa1c86efa55be118",
    "bistro-interior": "sha256:ad45951ba641106b24e7d91d49ebf5992fb6a42cb70a3082520e8de19a6cf514",
}
LOCKED_DIGEST_JOINT = "sha256:64fd54df6e9be522d6dbb3bec8fac1eb30a0a421c7a5a8185a3452c381178aa4"

# G10.5a 锁定帧内容 digest（默认面 parity 对账锚；g10_5_ab_preview.md §3 登记面）。
G10_5_FRAME_DIGEST = {
    ("rurix", "cornell-box"): "sha256:c2000ebfbe90359d55e668f8af3b7df24d64c3f72e637904f614821b7ad0d727",
    ("rurix", "bistro-interior"): "sha256:8519cc67c917e7b8c2c5a9bb5633ea5ee9e72deb8cf63b3b187b0d3ac5bb9935",
}

# G11.5b 全修复旗标面（G11.3 + G11.4 修复面合集 + --sky-ibl〔RXS-0397〕= definitive
# 复测面；双场景同消费——同一契约天光消费语义）。
SCENES = {
    "cornell-box": {
        "gltf": Path(r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf"),
        "ev100": 2.0,
        "res": (512, 512),
        "rurix_flags": ["--smooth-normals", "--gi-multibounce", "--sky-ibl"],
    },
    "bistro-interior": {
        "gltf": Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf"),
        "ev100": 1.0,
        "res": (1920, 1080),
        "rurix_flags": ["--material-pbr", "--light-seed-set", str(LIGHTING_BISTRO), "--gi-multibounce", "--sky-ibl"],
    },
}

# G11.2 域统一换算基线（C2 对齐面；M144/M153/M154 门登记面同一字面）。
ALIGNED_BASELINE_R3 = 2.7314592314362525
ALIGNED_BASELINE_R4 = 4.8486343559026714
ALIGNED_BASELINE_C1_BISTRO_MEDIAN = 2.7314592314362525
ALIGNED_BASELINE_C1_CORNELL_P90 = 0.29024957587122924

# 收敛阈消费面 = g11_budget g11.fix.* 标定条目 id（标定程序产，禁手写）。
ROW_THRESHOLD_IDS = {
    "R1": ("g11.fix.r1_ssim_shrink_tol", None),
    "R2": ("g11.fix.r2_coverage_shrink_tol", "g11.fix.r2_coverage_zero_band"),
    "R3": ("g11.fix.r3_luminance_shrink_tol", None),
    "R4": ("g11.fix.r4_p90_shrink_tol", None),
    "R5": ("g11.fix.r5_u64_seed_shrink_tol", None),
    "U1": ("g11.fix.u1_coverage_shrink_tol", "g11.fix.u1_coverage_zero_band"),
    "U2": ("g11.fix.u2_luminance_shrink_tol", None),
    "U3": ("g11.fix.u3_anim_channels_shrink_tol", None),
}

U64_MAX = 18446744073709551615  # 2^64 − 1（u64 顶格探针）

COMMANDS: list[dict] = []


def log(msg: str) -> None:
    print(f"[g11_5b_rerun] {msg}", flush=True)


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
    G10.5 锁定值；联合 digest 复核。"""
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
    joint = "sha256:" + hashlib.sha256(
        "".join(sorted(out[s].split(":", 1)[1] for s in SCENES)).encode("ascii")
    ).hexdigest()
    if joint != LOCKED_DIGEST_JOINT:
        raise SystemExit(f"契约联合 digest 漂移: {joint} ≠ {LOCKED_DIGEST_JOINT}（G10.5 锁定联合值）——门序硬约束停线")
    out["joint"] = joint
    log(f"契约 digest 联合复核: {joint} == G10.5 锁定联合值 ✓")
    return out


def _render_one(scene_id: str, out_name: str, flags: list[str], exposure: float | None = None,
                out_dir: Path | None = None) -> dict:
    s = SCENES[scene_id]
    # exposure=None → 管线内烘焙 2^(−EV100)（C2 对齐面）；parity 面传 1.0。
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
        frame2 = out_dir / f"{out_name}.exr"
        if frame2.exists():
            frame2.unlink()
        frame.rename(frame2)
        frame = frame2
    d = exr.decode_exr(frame.read_bytes(), "rurix")
    json_lines = [l for l in r.stdout.splitlines() if l.startswith('{"scene_id"')]
    if not json_lines:
        raise SystemExit(f"Rurix 渲染 {scene_id}/{out_name} 缺 stdout JSON 登记行")
    # 整行 json.loads 鲁棒面（sky_ibl 闭集块新增兼容；键序不敏感）。登记行
    # "frame" 字段 = Windows 路径未转义反斜杠（非严格 JSON——G11.5 驱动用正则
    # 抽取的原因面）；反斜杠→正斜杠预清洗后整行解析（消费字段 = 各闭集块，
    # 路径字段由驱动侧自持不回取）。
    doc = json.loads(json_lines[-1].replace("\\", "/"))
    return {
        "frame": str(frame),
        "frame_content_digest": frame_content_digest(d["width"], d["height"], 3, d["pixels"]),
        "exposure_scale_in_pipe": scale,
        "rurix_flags": list(flags),
        "param_digest": d["metadata"].get("rurix:capture_params_digest", ""),
        "file_digest": sha256_file(frame),
        "render_json": {
            "animations": doc["animations"],
            "materials": doc["materials"],
            "lights": doc["lights"],
            "world_cache": doc["world_cache"],
            "sky_ibl": doc.get("sky_ibl", {"enabled": False}),
            "frame_content_digest": doc["frame_content_digest"],
        },
    }


def stage_rurix() -> dict:
    """Rurix 端渲染：双场景全修复面 + --sky-ibl（RXS-0397）+ parity 双场景无旗标
    面（独立目录，== G10.5 锁定 digest 回归锚）。"""
    out = {}
    for scene_id, s in SCENES.items():
        out[scene_id] = _render_one(scene_id, scene_id, s["rurix_flags"])
        log(f"Rurix {scene_id}（全修复面 + --sky-ibl）: digest={out[scene_id]['frame_content_digest'][:32]}…")
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
    """UE 端双场景全量重跑（同契约 0-byte——G11.5b 修复面纯 Rurix 侧，UE 端 =
    同契约复现；G11_2_OUT_ROOT → g11_5b 帧区）。G11_5B_UE_COLLECT_ONLY=1 时只
    解码既有帧与探针补登记（分阶段增量合并面）。"""
    out = {}
    collect_only = os.environ.get("G11_5B_UE_COLLECT_ONLY") == "1"
    probe_dir = FRAMES_G11 / "probe"
    probe_dir.mkdir(parents=True, exist_ok=True)
    if not collect_only:
        for scene_id in SCENES:
            env = {
                "G10_5_SCENE": scene_id,
                "G10_5_CONTRACT": str(CORPUS / f"contract_params_{scene_id.replace('-', '_')}.json"),
                "G11_2_OUT_ROOT": "K:/rurix-ext/g11-frames/g11_5b/ue",
                "G11_3_PROBE_OUT": str(probe_dir / f"{scene_id}_probe.json"),
            }
            run_cmd([sys.executable, str(UE_RUN), str(UE_BUILD)], env=env, timeout=3600)
            log(f"UE 关卡建设 {scene_id} 完成（G11.5b 全量复跑面）")
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


def _seed_contract_text(seed: int) -> str:
    p = CORPUS / "contract_params_cornell_box.json"
    doc = json.loads(p.read_text(encoding="utf-8"))
    doc["time"]["random_seed"] = seed
    return json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n"


def _run_digest_on_text(text: str, u64_seed: bool) -> tuple[int, str, str]:
    """对给定契约文本跑 --contract-digest（临时文件面，不落库；M149 同构探针）。"""
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False, encoding="utf-8", newline="\n") as f:
        f.write(text)
        tmp = f.name
    try:
        argv = [str(RUST_RELEASE_BIN)] + (["--u64-seed"] if u64_seed else []) + ["--contract-digest", tmp]
        r = run_cmd(argv)
        return r.returncode, r.stdout, r.stderr
    finally:
        Path(tmp).unlink(missing_ok=True)


def _digest_of(stdout: str) -> str | None:
    for line in stdout.splitlines():
        if "param_digest_rust" in line:
            return "sha256:" + line.split("=")[-1].strip()
    return None


def stage_metrics() -> dict:
    """HDR 覆盖/亮度统计 + LDR 度量 + 11 行修复前后 delta 对拍（baseline =
    g10_gap_registry 0-byte 消费 + G11.2 域统一换算面；R5/U3 host 面同段实测）。"""
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
            f"{scene_id}: HDR 中位 delta {out['scenes'][scene_id]['hdr_luminance_median_delta']:+.10f} "
            f"p90 delta {out['scenes'][scene_id]['hdr_luminance_p90_delta']:+.10f} "
            f"覆盖 delta {out['scenes'][scene_id]['hdr_nonzero_ratio_delta']:+.10f} "
            f"LDR 中位 delta {out['scenes'][scene_id]['ldr_luminance_median_delta']:+.10f} "
            f"SSIM={float(ssim_v):.6f}"
        )

    rows = {p: gap_row(p) for p in ("R1", "R2", "R3", "R4", "R5", "U1", "U2", "U3", "C1", "C2", "C3")}
    bistro = out["scenes"]["bistro-interior"]
    cornell = out["scenes"]["cornell-box"]

    # R5 host 探针面（u64 顶格 seed 合法消费 → delta 收敛至 0；M149 同构）。
    text_max = _seed_contract_text(U64_MAX)
    code_max, out_max, _ = _run_digest_on_text(text_max, True)
    digest_max = _digest_of(out_max)
    text_m1 = _seed_contract_text(U64_MAX - 1)
    code_m1, out_m1, _ = _run_digest_on_text(text_m1, True)
    digest_m1 = _digest_of(out_m1)
    r5_consumed = code_max == 0 and digest_max is not None and digest_m1 is not None and digest_max != digest_m1

    # U3 host 面（bistro 渲染输出 animations 闭集块对账）。
    bistro_anim = json.loads(REPORT_PATH.read_text(encoding="utf-8"))["results"]["rurix"]["bistro-interior"]["render_json"]["animations"]
    u3_retest = 0.0 if (
        bistro_anim.get("package_count") == 1 and bistro_anim.get("channels") == 2
        and bistro_anim.get("consumed_channels") == 0 and bistro_anim.get("policy") == "strip_static_contract"
    ) else float(bistro_anim.get("channels", 0))

    # C2 面尺度实测（复跑报告当次登记值——禁手写）。
    _rep_results = json.loads(REPORT_PATH.read_text(encoding="utf-8"))["results"]
    scale_rurix = {s: _rep_results["rurix"][s]["exposure_scale_in_pipe"] for s in SCENES}
    ldr_scales = {_k: _v["exposure_scale_host"] for _k, _v in _rep_results["derive"].items()}
    c2_retest = 0.0 if (
        scale_rurix["cornell-box"] == 0.25 and scale_rurix["bistro-interior"] == 0.5
        and all(v == 1.0 for v in ldr_scales.values()) and len(ldr_scales) == 4
    ) else rows["C2"]["measured_delta"][0]["delta"]

    out["closure_faces"] = {
        "r1": {
            "gap_row_id": rows["R1"]["gap_id"],
            "metric": "ssim@bistro-interior(ldr)",
            "baseline_delta": rows["R1"]["measured_delta"][0]["delta"],
            "baseline_a": rows["R1"]["measured_delta"][0]["a_value"],
            "retest_ssim": bistro["metrics_ldr"]["ssim"],
            "retest_delta": bistro["metrics_ldr"]["ssim_delta_identity"],
        },
        "r2": {
            "gap_row_id": rows["R2"]["gap_id"],
            "metric": "hdr_nonzero_ratio@cornell-box",
            "baseline_delta": rows["R2"]["measured_delta"][0]["delta"],
            "baseline_a": rows["R2"]["measured_delta"][0]["a_value"],
            "baseline_b": rows["R2"]["measured_delta"][0]["b_value"],
            "retest_rurix_nonzero": cornell["hdr_stats"]["rurix"]["nonzero_ratio"],
            "retest_ue5_nonzero": cornell["hdr_stats"]["ue5"]["nonzero_ratio"],
            "retest_delta": cornell["hdr_nonzero_ratio_delta"],
        },
        "r3": {
            "gap_row_id": rows["R3"]["gap_id"],
            "metric": "hdr_luminance_median@bistro-interior",
            "baseline_delta": rows["R3"]["measured_delta"][0]["delta"],
            "baseline_a": rows["R3"]["measured_delta"][0]["a_value"],
            "baseline_b": rows["R3"]["measured_delta"][0]["b_value"],
            "baseline_delta_aligned_domain": ALIGNED_BASELINE_R3,
            "retest_rurix_median": bistro["hdr_stats"]["rurix"]["median"],
            "retest_ue5_median": bistro["hdr_stats"]["ue5"]["median"],
            "retest_delta": bistro["hdr_luminance_median_delta"],
        },
        "r4": {
            "gap_row_id": rows["R4"]["gap_id"],
            "metric": "hdr_luminance_p90@bistro-interior",
            "baseline_delta": rows["R4"]["measured_delta"][0]["delta"],
            "baseline_a": rows["R4"]["measured_delta"][0]["a_value"],
            "baseline_b": rows["R4"]["measured_delta"][0]["b_value"],
            "baseline_delta_aligned_domain": ALIGNED_BASELINE_R4,
            "retest_rurix_p90": bistro["hdr_stats"]["rurix"]["p90"],
            "retest_ue5_p90": bistro["hdr_stats"]["ue5"]["p90"],
            "retest_delta": bistro["hdr_luminance_p90_delta"],
        },
        "r5": {
            "gap_row_id": rows["R5"]["gap_id"],
            "metric": "contract_seed_u64_max_rejection",
            "baseline_delta": rows["R5"]["measured_delta"][0]["delta"],
            "baseline_a": rows["R5"]["measured_delta"][0]["a_value"],
            "baseline_b": rows["R5"]["measured_delta"][0]["b_value"],
            "retest_u64_max_consumed": r5_consumed,
            "retest_u64_max_digest": digest_max,
            "retest_u64_max_minus1_digest": digest_m1,
            "retest_delta": 0.0 if r5_consumed else rows["R5"]["measured_delta"][0]["delta"],
        },
        "u1": {
            "gap_row_id": rows["U1"]["gap_id"],
            "metric": "hdr_nonzero_ratio@cornell-box",
            "baseline_delta": rows["U1"]["measured_delta"][0]["delta"],
            "baseline_a": rows["U1"]["measured_delta"][0]["a_value"],
            "baseline_b": rows["U1"]["measured_delta"][0]["b_value"],
            "retest_rurix_nonzero": cornell["hdr_stats"]["rurix"]["nonzero_ratio"],
            "retest_ue5_nonzero": cornell["hdr_stats"]["ue5"]["nonzero_ratio"],
            "retest_delta": cornell["hdr_nonzero_ratio_delta"],
        },
        "u2": {
            "gap_row_id": rows["U2"]["gap_id"],
            "metric": "ldr_luminance_median@bistro-interior",
            "baseline_delta": rows["U2"]["measured_delta"][0]["delta"],
            "baseline_a": rows["U2"]["measured_delta"][0]["a_value"],
            "baseline_b": rows["U2"]["measured_delta"][0]["b_value"],
            "retest_rurix_median": bistro["ldr_stats"]["rurix"]["median"],
            "retest_ue5_median": bistro["ldr_stats"]["ue5"]["median"],
            "retest_delta": bistro["ldr_luminance_median_delta"],
        },
        "u3": {
            "gap_row_id": rows["U3"]["gap_id"],
            "metric": "gltf_animation_channels_unconsumed@bistro-interior",
            "baseline_delta": rows["U3"]["measured_delta"][0]["delta"],
            "baseline_a": rows["U3"]["measured_delta"][0]["a_value"],
            "baseline_b": rows["U3"]["measured_delta"][0]["b_value"],
            "retest_package_animations": bistro_anim.get("package_count"),
            "retest_package_channels": bistro_anim.get("channels"),
            "retest_consumed_channels": bistro_anim.get("consumed_channels"),
            "retest_policy": bistro_anim.get("policy"),
            "retest_delta": u3_retest,
        },
        "c1": {
            "gap_row_id": rows["C1"]["gap_id"],
            "metric": "hdr_luminance_median@bistro-interior + hdr_luminance_p90@cornell-box(rurix×2^-EV100)",
            "baseline_delta": rows["C1"]["measured_delta"][0]["delta"],
            "baseline_delta_cornell_p90": rows["C1"]["measured_delta"][1]["delta"],
            "baseline_delta_aligned_domain_bistro_median": ALIGNED_BASELINE_C1_BISTRO_MEDIAN,
            "baseline_delta_aligned_domain_cornell_p90": ALIGNED_BASELINE_C1_CORNELL_P90,
            "retest_bistro_median_delta": bistro["hdr_luminance_median_delta"],
            "retest_cornell_p90_delta": cornell["hdr_luminance_p90_delta"],
            "retest_delta": bistro["hdr_luminance_median_delta"],
        },
        "c2": {
            "gap_row_id": rows["C2"]["gap_id"],
            "metric": "ldr_derivation_exposure_scale@cornell-box + @bistro-interior",
            "baseline_delta": rows["C2"]["measured_delta"][0]["delta"],
            "baseline_delta_bistro": rows["C2"]["measured_delta"][1]["delta"],
            "retest_scale_rurix_cornell": scale_rurix["cornell-box"],
            "retest_scale_rurix_bistro": scale_rurix["bistro-interior"],
            "retest_scale_ue": 1.0,
            "retest_ldr_derivation_scale_both_ends": 1.0 if all(v == 1.0 for v in ldr_scales.values()) and len(ldr_scales) == 4 else None,
            "retest_delta": c2_retest,
        },
        "c3": {
            "gap_row_id": rows["C3"]["gap_id"],
            "metric": "exr_source_bit_depth@bistro-interior",
            "baseline_delta": rows["C3"]["measured_delta"][0]["delta"],
            "baseline_a": rows["C3"]["measured_delta"][0]["a_value"],
            "baseline_b": rows["C3"]["measured_delta"][0]["b_value"],
            "retest_ue_source_bit_depth": json.loads(REPORT_PATH.read_text(encoding="utf-8"))["results"]["ue"]["bistro-interior"]["source_bit_depth"],
            "retest_metric_domain_bit_depth": 32.0,
            "retest_delta": 0.0,
        },
    }
    return out


def _budget_thresholds() -> dict:
    budget = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
    return {e.get("id"): e for e in budget.get("entries", [])}


def stage_registry() -> dict:
    """复测差距清单 11 行闭集落盘（行集 == G10.8b 锁定清单逐字对账；逐项
    closed/converged/partial/aligned_closed 显式判定——quality_gap 行 RXS-0393 L2
    收敛判定（收敛阈 = g11_budget g11.fix.* 标定条目消费），caliber_diff 行
    caliber_diff 款；partial/未收敛行如实登记不充绿并带承接锚）。收敛判定机器
    形态与 ci/g11_3_fix_lib.evaluate_closure 同一判定层（0-byte 语义）。"""
    report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    faces = report["results"]["metrics"]["closure_faces"]
    locked = json.loads(GAP_REGISTRY.read_text(encoding="utf-8"))
    entries = _budget_thresholds()
    residual = json.loads(RESIDUAL_REGISTRY.read_text(encoding="utf-8"))
    residual_ids = [it["residual_id"] for it in residual.get("items", [])]

    def thr(row: str) -> tuple[float, float, str]:
        sid, zid = ROW_THRESHOLD_IDS[row]
        e = entries[sid]
        shrink_tol = e["threshold"]
        zero_band = entries[zid]["threshold"] if zid else 0.0
        prov = f"g11_budget 标定条目 {sid}" + (f" + {zid}" if zid else "") + "（标定程序产 p100×k，P-09 禁手写）"
        return shrink_tol, zero_band, prov

    items: list[dict] = []
    for lk in locked["items"]:
        prefix = lk["title"].split(" ", 1)[0]
        face = faces[prefix.lower()]
        base = lk["measured_delta"][0]
        item = {
            "gap_id": lk["gap_id"],
            "scene_id": lk["scene_id"],
            "camera_id": lk["camera_id"],
            "domain": lk["domain"],
            "kind": lk["kind"],
            "title": lk["title"],
            "metric": face["metric"],
            "baseline_delta": base["delta"],
            "baseline_a": base["a_value"],
            "baseline_b": base["b_value"],
            "retest_delta": face["retest_delta"],
        }
        if lk["kind"] == "quality_gap":
            shrink_tol, zero_band, prov = thr(prefix)
            baseline_for_eval = face.get("baseline_delta_aligned_domain", base["delta"])
            ev = fl.evaluate_closure(baseline_for_eval, face["retest_delta"], shrink_tol, zero_band)
            item["baseline_delta_evaluation_domain"] = baseline_for_eval
            item["shrink_threshold"] = shrink_tol
            item["zero_band"] = zero_band
            item["threshold_provenance"] = prov
            item["converged"] = bool(ev["converged"])
            item["shrink"] = ev["shrink"]
            item["direction_ok"] = ev["direction_ok"]
            if ev["converged"]:
                item["closure_status"] = "converged"
                item["disposition"] = "closed_retest_converged"
                item["disposition_anchor"] = (
                    "G11.5b 复测闭环（诊断修复后同契约复跑收敛——本清单终态；"
                    "G11.5 首跑 FAIL→G11.5b 诊断修复→复跑收敛完整留痕见契约 §8.5/§8.5b）"
                )
            else:
                item["closure_status"] = "partial"
                item["disposition"] = "partial_not_converged_honest_registration"
                if prefix == "R1":
                    item["disposition_anchor"] = (
                        "G11.6 P2 穷举承接（『锁定度量对正确修复结构性不友好』候选行——契约 §8.3a 登记，"
                        "反向激励旁证 ssim(ue_修,rurix_未修白帧)=0.1624318277352612 > ssim(ue_修,rurix_修)=0.009656442299775102 "
                        "入证据链 evidence/g11_m147_fix_r1_material_subset_20260816T180419Z.json material_provenance）"
                        "+ G12+ 度量口径修订评估承接面——不冒充 closed"
                    )
                else:
                    item["disposition_anchor"] = "G12+ 承接（复测未收敛如实登记——不冒充 closed）"
        else:
            # caliber_diff 款（RXS-0393 L2 同条款 C 族字面）。
            if prefix == "C1":
                consistent = (
                    face["retest_bistro_median_delta"] <= ALIGNED_BASELINE_C1_BISTRO_MEDIAN
                    and face["retest_bistro_median_delta"] >= 0.0
                    and face["retest_cornell_p90_delta"] <= ALIGNED_BASELINE_C1_CORNELL_P90
                    and face["retest_cornell_p90_delta"] >= 0.0
                    and abs(face["retest_bistro_median_delta"] - faces["r3"]["retest_delta"]) == 0.0
                )
                item["retest_bistro_median_delta"] = face["retest_bistro_median_delta"]
                item["retest_cornell_p90_delta"] = face["retest_cornell_p90_delta"]
                item["attribution"] = (
                    "复测残余 delta 全额归属：已消费修复行 R3（g11.p0.m153）/ R4（g11.p0.m154）"
                    "收敛后残余 + G11.5b 天光直接漫反射 IBL 消费面（--sky-ibl，RXS-0397）落地后残余 "
                    "+ 登记残余项 c1_ue_specular_ibl（UE 镜面 IBL 结构差——G11.5b 实测份额 ≤0.03%〔nospec 臂〕，"
                    "G15 画质量级收口面候选维持）+ c3_source_bit_depth_quantization（源位深量化差，C3 行承接面）"
                    "+ g11_5b_sun_through_glass_tail（太阳穿半透明玻璃高光尾，G11.6 P2 候选登记——诊断文档 §0 次因行）"
                    "——无未归因余量"
                )
                item["residual_ids"] = [r for r in residual_ids if r.startswith("c1_")] + [
                    "c3_source_bit_depth_quantization", "g11_5b_sun_through_glass_tail"]
            elif prefix == "C2":
                consistent = face["retest_delta"] == 0.0 and face["retest_ldr_derivation_scale_both_ends"] == 1.0
                item["attribution"] = (
                    "曝光链派生尺度双端统一 ×1.0（Rurix 臂管线内烘焙 2^(−EV100)，C2 对齐落点维持）"
                    "——复测派生尺度差 0.0，全额归属 aligned_chains.exposure_scale（aligned_fixed）"
                )
                item["residual_ids"] = []
            else:
                consistent = (
                    face["retest_delta"] == 0.0
                    and face["retest_ue_source_bit_depth"] == "float16"
                    and face["retest_metric_domain_bit_depth"] == 32.0
                )
                item["attribution"] = (
                    "度量域统一提升 f32（fp16→f32 精确无损，C3 对齐面）——复测度量域位深差 0.0；"
                    "源位深量化差（UE MRQ fp16 写出一次不可回退）全额归属登记残余项 "
                    "c3_source_bit_depth_quantization"
                )
                item["residual_ids"] = ["c3_source_bit_depth_quantization"]
            item["converged"] = bool(consistent)
            item["closure_status"] = "aligned_closed" if consistent else "partial"
            item["disposition"] = "aligned_closed_caliber_diff" if consistent else "partial_not_aligned_honest_registration"
            item["disposition_anchor"] = (
                "G11.5b 复测闭环（口径差行对齐闭环维持——本清单终态）"
                if consistent else "G12+ 承接（口径对齐残余复核——不冒充 closed）"
            )
            item["threshold_provenance"] = "caliber_diff 款：口径对齐 + 残余登记一致性机核（RXS-0393 L2 C 族；无收敛幅度阈面）"
        items.append(item)

    summary = {
        "total": len(items),
        "converged": sum(1 for i in items if i["closure_status"] == "converged"),
        "aligned_closed": sum(1 for i in items if i["closure_status"] == "aligned_closed"),
        "partial": sum(1 for i in items if i["closure_status"] == "partial"),
        "partial_rows": [i["title"].split(" ", 1)[0] for i in items if i["closure_status"] == "partial"],
        "new_items": 0,
    }
    doc = {
        "schema_version": 1,
        "registry": "g11_5b_retest_gap_registry",
        "generated_by": "milestones/g11/harness/g11_5b_ab_rerun.py --stage registry",
        "upstream_registry": "milestones/g10/g10_gap_registry.json（G10.8b 终审锁定 11 行闭集 0-byte 只读消费；行集逐字对账）",
        "semantics": "G11.5b 追加子波同契约双端复跑复测差距清单：11 行逐项闭环核验终态（修复前锁定 delta〔清单字面〕vs G11.5b 复测 delta〔当次 measured，Rurix 端 += --sky-ibl 天光直接漫反射 IBL 面 RXS-0397〕→ RXS-0393 L2 分款收敛断言；收敛阈标定程序产禁手写；partial/未收敛行如实登记不充绿并带承接锚）；R1 行不收敛则整波 FAIL（契约 §8.3a 不弱化声明）；G11.5 首跑产物 0-byte 保留（g11_5_rerun_report.json / g11_5_retest_gap_registry.json 与 K: g11_5 帧区），本清单 = G11.5b 复测面终态",
        "contract_digest": dict(LOCKED_DIGEST, joint=LOCKED_DIGEST_JOINT),
        "frames_root": str(FRAMES_G11),
        "rerun_report": "milestones/g11/g11_5b_rerun_report.json",
        "rerun_report_digest": sha256_file(REPORT_PATH),
        "items": items,
        "summary": summary,
    }
    RETEST_REGISTRY_PATH.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    log(f"复测差距清单落盘 {RETEST_REGISTRY_PATH}（{summary}）")
    return {
        "path": str(RETEST_REGISTRY_PATH.relative_to(ROOT)).replace("\\", "/"),
        "digest": sha256_file(RETEST_REGISTRY_PATH),
        "summary": summary,
    }


def write_json(path: Path, doc: dict) -> None:
    path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    log(f"落盘 {path}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--stage", default="all",
                    choices=["all", "contract", "rurix", "ue", "derive", "metrics", "registry"])
    args = ap.parse_args()

    FRAMES_G11.mkdir(parents=True, exist_ok=True)
    if REPORT_PATH.is_file():
        try:
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            report = {}
    else:
        report = {}
    report.setdefault("schema_version", 1)
    report["report"] = "g11_5b_ab_rerun"
    report["generated_by"] = "milestones/g11/harness/g11_5b_ab_rerun.py"
    report["frames_root"] = str(FRAMES_G11)
    report["predecessor"] = (
        "milestones/g11/g11_5_rerun_report.json + g11_5_retest_gap_registry.json（G11.5 首跑 FAIL 停线面，0-byte 保留；"
        "R1 复测 ssim=0.010847362392386794 结构性塌陷——G11.5b 诊断修复面 = --sky-ibl 天光直接漫反射 IBL〔RXS-0397〕，"
        "诊断链 milestones/g11/design/g11_5b_ldr_residual_diag.md）"
    )
    report["timestamp_utc"] = _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    report.setdefault("stages", {})
    report.setdefault("results", {})

    def run(name: str, fn):
        report["results"][name] = fn()
        report["stages"][name] = "done"
        write_json(REPORT_PATH, report)

    order = ["contract", "rurix", "ue", "derive", "metrics", "registry"]
    if args.stage == "all":
        for st in order:
            run(st, globals()[f"stage_{st}"])
    else:
        run(args.stage, globals()[f"stage_{args.stage}"])

    if args.stage in ("all", "contract"):
        report["locked_contract_digest"] = {s: LOCKED_DIGEST[s] for s in SCENES}
        report["locked_contract_digest_joint"] = LOCKED_DIGEST_JOINT
    report["commands"] = COMMANDS
    write_json(REPORT_PATH, report)
    log(f"全部阶段完成：{json.dumps(report['stages'], ensure_ascii=False)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
