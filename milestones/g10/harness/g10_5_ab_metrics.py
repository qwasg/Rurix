#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.5a 波续）
"""G10.5a A/B 首跑度量预演驱动（FLIP/SSIM/PSNR + diff 报告，design/g10_5_ab_preview.md 数字面）。

消费 G10.4 度量基建单一事实源（ci/g10_flip_lib.py / ci/g10_ssim_psnr_lib.py /
ci/g10_exr_lib.py）+ M137 diff 报告器（g10_m137_diff_report bin）；LDR 帧 =
双端共用 host 侧 sRGB 编码器派生链产物（RXS-0386 L2；g10_5_scene_render
--derive-ldr，Rurix 侧 ×2^(−EV100) 曝光尺度、UE 侧 ×1.0〔pipe 内手动曝光已施〕）。

G10 零通过线：本脚本只测量不定档；全部数字进 preview 文档 + 差距清单候选。

用法：py -3 milestones/g10/harness/g10_5_ab_metrics.py
输出：stdout JSON 行（逐场景 flip/ssim/psnr + 帧统计）+ diff 报告产物目录登记。
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))

import g10_exr_lib as exr  # noqa: E402
import g10_flip_lib as flip  # noqa: E402
import g10_ssim_psnr_lib as ssim_psnr  # noqa: E402

FRAMES = Path(r"K:\rurix-ext\g10-frames\g10_5")
DIFF_BIN = ROOT / "target" / "debug" / "g10_m137_diff_report.exe"

SCENES = {
    "cornell-box": {"ev100": 2.0, "res": (512, 512)},
    "bistro-interior": {"ev100": 1.0, "res": (1920, 1080)},
}


def load_pixels(path: Path, end: str):
    d = exr.decode_exr(path.read_bytes(), end)
    arr = np.asarray(d["pixels"], dtype=np.float64).reshape(d["height"], d["width"], 3)
    return d, arr


def lum_stats(arr):
    lum = 0.2126 * arr[..., 0] + 0.7152 * arr[..., 1] + 0.0722 * arr[..., 2]
    flat = np.sort(lum.ravel())
    n = flat.size
    return {
        "median": float(flat[n // 2]),
        "p90": float(flat[int(n * 0.9)]),
        "max": float(flat[-1]),
        "nonzero_ratio": float(np.count_nonzero(flat > 1e-6) / n),
    }


def main() -> int:
    rc_build = subprocess.run(
        ["cargo", "build", "-p", "rurix-render", "--bin", "g10_m137_diff_report"],
        capture_output=True, text=True, cwd=ROOT,
    )
    if rc_build.returncode != 0:
        print(rc_build.stderr[-2000:], file=sys.stderr)
        return 1

    out = {}
    for scene_id in SCENES:
        hdr_r, arr_hdr_r = load_pixels(FRAMES / "rurix" / f"{scene_id}.exr", "rurix")
        hdr_u, arr_hdr_u = load_pixels(FRAMES / "ue" / scene_id / ".0000.exr", "ue5")
        ldr_r, arr_r = load_pixels(FRAMES / "ldr" / f"{scene_id}_rurix_ldr.exr", "rurix")
        ldr_u, arr_u = load_pixels(FRAMES / "ldr" / f"{scene_id}_ue5_ldr.exr", "rurix")

        ssim_v = ssim_psnr.ssim_wang2004(arr_u, arr_r)
        psnr_v = ssim_psnr.psnr_joint(arr_u, arr_r)
        err_map, flip_v = flip.flip_ldr(arr_u, arr_r)  # reference=UE5（外部参照端）

        diff_dir = FRAMES / "diff" / scene_id
        diff_dir.mkdir(parents=True, exist_ok=True)
        ev_path = diff_dir / "diff_report.json"
        r = subprocess.run(
            [
                str(DIFF_BIN),
                "--frame-a", str(FRAMES / "ldr" / f"{scene_id}_ue5_ldr.exr"),
                "--frame-b", str(FRAMES / "ldr" / f"{scene_id}_rurix_ldr.exr"),
                "--out-dir", str(diff_dir),
                "--evidence", str(ev_path),
                "--scene-id", scene_id,
                "--camera-id", "g10_contract_camera",
                "--frame-index", "0",
                "--threshold", "0.0",
            ],
            capture_output=True, text=True, cwd=ROOT,
        )
        diff_doc = json.loads(ev_path.read_text(encoding="utf-8")) if ev_path.is_file() else {"_diff_exit": r.returncode, "_stderr": r.stderr[-400:]}

        out[scene_id] = {
            "hdr_stats": {"rurix": lum_stats(arr_hdr_r), "ue5": lum_stats(arr_hdr_u)},
            "ldr_stats": {"rurix": lum_stats(arr_r), "ue5": lum_stats(arr_u)},
            "metrics": {
                "flip_ldr": float(flip_v),
                "ssim": float(ssim_v),
                "psnr_db": ssim_psnr.psnr_json_value(psnr_v),
            },
            "diff_report": {
                "dir": str(diff_dir),
                "scalars": diff_doc.get("scalars", diff_doc.get("summary", diff_doc)),
            },
            "frame_digests": {
                "hdr_rurix": hdr_r["metadata"].get("rurix:frame_content_digest") or "",
                "ldr_rurix_source": ldr_r["metadata"].get("rurix:source_frame_digest", ""),
                "ldr_ue5_source": ldr_u["metadata"].get("rurix:source_frame_digest", ""),
            },
        }
    print(json.dumps(out, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
