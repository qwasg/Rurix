#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.5b 波）
"""G11.5b 修复预览度量（preview 帧 vs G11.5 UE 帧；正式复测面 = g11_5b_ab_rerun.py 驱动）。

用法：py -3 milestones/g11/harness/g11_5b_preview_metrics.py <preview_dir>
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))

import numpy as np  # noqa: E402
import g10_exr_lib as exr  # noqa: E402
import g10_flip_lib as flip  # noqa: E402
import g10_ssim_psnr_lib as sp  # noqa: E402

G = Path(r"K:\rurix-ext\g11-frames\g11_5")


def load(p: Path, e: str) -> np.ndarray:
    d = exr.decode_exr(p.read_bytes(), e)
    return np.asarray(d["pixels"], dtype=np.float64).reshape(d["height"], d["width"], 3)


def lum(a: np.ndarray) -> np.ndarray:
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def st(l: np.ndarray) -> dict:
    f = np.sort(l.ravel())
    n = f.size
    return {"median": float(f[n // 2]), "p90": float(f[int(n * 0.9)]),
            "mean": float(f.mean()), "max": float(f[-1]),
            "nonzero": float(np.count_nonzero(f > 1e-6) / n)}


def main() -> int:
    p = Path(sys.argv[1] if len(sys.argv) > 1 else r"K:\rurix-ext\g11-frames\g11_5b\preview")
    for scene in ("bistro-interior", "cornell-box"):
        ue_h = load(G / "ue" / scene / ".0000.exr", "ue5")
        ru_h = load(p / f"{scene}.exr", "rurix")
        ue_l = load(G / "ldr" / f"{scene}_ue5_ldr.exr", "rurix")
        ru_l = load(p / f"{scene}_rurix_ldr.exr", "rurix")
        uh, rh, ul, rl = st(lum(ue_h)), st(lum(ru_h)), st(lum(ue_l)), st(lum(ru_l))
        ssim = sp.ssim_wang2004(ue_l, ru_l)
        fl = flip.flip_ldr(ue_l, ru_l)
        psnr = sp.psnr_json_value(sp.psnr_joint(ue_l, ru_l))
        print(f"== {scene}")
        print(f"  HDR median  rurix={rh['median']:.8f} ue={uh['median']:.8f} delta={uh['median'] - rh['median']:.8f}")
        print(f"  HDR p90     rurix={rh['p90']:.8f} ue={uh['p90']:.8f} delta={uh['p90'] - rh['p90']:.8f}")
        print(f"  HDR max     rurix={rh['max']:.6f} ue={uh['max']:.6f}")
        print(f"  HDR nonzero rurix={rh['nonzero']:.8f} ue={uh['nonzero']:.8f} delta={uh['nonzero'] - rh['nonzero']:.8f}")
        print(f"  LDR median  rurix={rl['median']:.8f} ue={ul['median']:.8f} delta={ul['median'] - rl['median']:.8f}")
        print(f"  SSIM={ssim!r}（R1 收敛阈：复测 delta < 基线 0.8328980787837229 ⇔ ssim > 0.16710192121627712；G11.5 复测 0.010847362392386794）")
        print(f"  FLIP={fl[1]!r} PSNR={psnr!r}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
