#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.5b 波）
"""G11.5b LDR 残差分解诊断驱动（g11_5b_ldr_residual_diag.md 数字面；全部输出来自命令实测）。

消费 G10.4 度量基建单一事实源（ci/g10_exr_lib.py 解码）+ G11.5 复测帧区（只读）
+ G11.5b 诊断帧区（--diag-ldr-stages / --diag-aces13-sweep / --diag-sky-vis 产物）：

① HDR→LDR 逐段亮度统计（stage1 曝光后 / stage2 view transform 后 / stage3 sRGB 后）
   双端逐段对比——发散段定位（stage3 与 G11.5 已派生 LDR 逐位一致互证）；
② tone mapping 曲线对拍——aces13+sRGB 组合曲线 sweep（bin 单源实测）+ 双端真实帧
   (hdr,ldr) 像素对经验映射分桶对比（UE 侧实际应用曲线 = 同一 host 派生链实测取证）；
③ diff 热区图——UE 亮度分位三区（高光 p90+ / 中间调 / 阴影 p10−）残差统计 +
   32×18 块区 log10(ue/rurix) 网格 + PNG 可视化落盘；
④ 天空/太阳可见性审计（--diag-sky-vis 产物）双场景汇总。

用法：py -3 milestones/g11/harness/g11_5b_ldr_diag.py
输出：stdout JSON（全部数字）+ K:/rurix-ext/g11-frames/g11_5b/diag/*.png 可视化。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))

import g10_exr_lib as exr  # noqa: E402

G11_5 = Path(r"K:\rurix-ext\g11-frames\g11_5")
DIAG = Path(r"K:\rurix-ext\g11-frames\g11_5b\diag")

SCENES = ["cornell-box", "bistro-interior"]
ENDS = ["rurix", "ue5"]


def load(path: Path, end: str):
    d = exr.decode_exr(path.read_bytes(), end)
    arr = np.asarray(d["pixels"], dtype=np.float64).reshape(d["height"], d["width"], 3)
    return d, arr


def lum(arr: np.ndarray) -> np.ndarray:
    return 0.2126 * arr[..., 0] + 0.7152 * arr[..., 1] + 0.0722 * arr[..., 2]


def stats(l: np.ndarray) -> dict:
    flat = np.sort(l.ravel())
    n = flat.size
    return {
        "median": float(flat[n // 2]),
        "p10": float(flat[int(n * 0.1)]),
        "p90": float(flat[int(n * 0.9)]),
        "mean": float(flat.mean()),
        "max": float(flat[-1]),
        "nonzero_ratio": float(np.count_nonzero(flat > 1e-6) / n),
    }


def gray_png(arr: np.ndarray, path: Path, lo: float, hi: float) -> None:
    v = np.clip((arr - lo) / (hi - lo), 0.0, 1.0)
    Image.fromarray((v * 255.0).astype(np.uint8), mode="L").save(path)


def main() -> int:
    out: dict = {"scenes": {}}

    # ── ① 逐段亮度统计（stage1 曝光后 / stage2 view transform 后 / stage3 sRGB 后）──
    for scene in SCENES:
        tag = "cornell" if scene == "cornell-box" else "bistro"
        per_end: dict = {}
        for end in ENDS:
            _, a1 = load(DIAG / f"{tag}_{end}_stage1_post_exposure.exr", "rurix")
            _, a2 = load(DIAG / f"{tag}_{end}_stage2_view_linear.exr", "rurix")
            _, a3 = load(DIAG / f"{tag}_{end}_stage3_srgb.exr", "rurix")
            # stage3 与 G11.5 已派生 LDR 逐位互证（派生链确定性 + 诊断面零口径漂移）。
            _, ldr_ref = load(G11_5 / "ldr" / f"{scene}_{end}_ldr.exr", "rurix")
            bitexact = bool(
                np.asarray(a3, dtype=np.float32).tobytes()
                == np.asarray(ldr_ref, dtype=np.float32).tobytes()
            )
            per_end[end] = {
                "stage1_post_exposure": stats(lum(a1)),
                "stage2_view_linear": stats(lum(a2)),
                "stage3_srgb": stats(lum(a3)),
                "stage3_vs_g11_5_ldr_bitexact": bitexact,
            }
        ratio = {}
        for st in ("stage1_post_exposure", "stage2_view_linear", "stage3_srgb"):
            ratio[st] = {
                "median_ratio_ue_over_rurix": (
                    per_end["ue5"][st]["median"] / per_end["rurix"][st]["median"]
                    if per_end["rurix"][st]["median"] > 0
                    else None
                ),
                "median_delta": per_end["ue5"][st]["median"] - per_end["rurix"][st]["median"],
                "mean_ratio_ue_over_rurix": (
                    per_end["ue5"][st]["mean"] / per_end["rurix"][st]["mean"]
                    if per_end["rurix"][st]["mean"] > 0
                    else None
                ),
            }
        out["scenes"][scene] = {"per_end": per_end, "stage_ratios": ratio}

    # ── ② tone curve 对拍（sweep 单源 + 真实帧经验映射分桶）──
    sweep = json.loads((DIAG / "aces13_sweep.json").read_text(encoding="utf-8-sig"))
    curve_out = {"sweep_neutral": [
        {"in": s["in"][0], "display_linear": s["display_linear"][0], "srgb": s["srgb"][0]}
        for s in sweep["samples"] if s["tag"].startswith("neutral_")
    ]}
    for scene in SCENES:
        tag = "cornell" if scene == "cornell-box" else "bistro"
        emp = {}
        for end in ENDS:
            _, a1 = load(DIAG / f"{tag}_{end}_stage1_post_exposure.exr", "rurix")
            _, a3 = load(DIAG / f"{tag}_{end}_stage3_srgb.exr", "rurix")
            l1 = lum(a1).ravel()
            l3 = lum(a3).ravel()
            bins = [(-1e-7, 1e-6), (1e-6, 1e-4), (1e-4, 1e-3), (1e-3, 1e-2), (1e-2, 3e-2),
                    (3e-2, 1e-1), (1e-1, 3e-1), (3e-1, 1.0), (1.0, 3.0), (3.0, 1e3)]
            rows = []
            for lo, hi in bins:
                m = (l1 > lo) & (l1 <= hi)
                cnt = int(m.sum())
                rows.append({
                    "bin": f"({lo:g},{hi:g}]",
                    "count": cnt,
                    "hdr_median": float(np.median(l1[m])) if cnt else None,
                    "ldr_srgb_median": float(np.median(l3[m])) if cnt else None,
                })
            emp[end] = rows
        curve_out[scene] = emp

    # sweep 中性曲线关键点（经验映射参照锚）
    out["tone_curve"] = curve_out

    # ── ③ diff 热区（UE 亮度分位三区 + 块区网格 + PNG）──
    diff_out = {}
    for scene in SCENES:
        _, hdr_r = load(G11_5 / "rurix" / f"{scene}.exr", "rurix")
        _, hdr_u = load(G11_5 / "ue" / scene / ".0000.exr", "ue5")
        _, ldr_r = load(G11_5 / "ldr" / f"{scene}_rurix_ldr.exr", "rurix")
        _, ldr_u = load(G11_5 / "ldr" / f"{scene}_ue5_ldr.exr", "rurix")
        lu = lum(ldr_u)
        lr = lum(ldr_r)
        flat_u = np.sort(lu.ravel())
        p10 = float(flat_u[int(flat_u.size * 0.1)])
        p90 = float(flat_u[int(flat_u.size * 0.9)])
        zones = {}
        for name, m in (
            ("shadow_ue_lt_p10", lu <= p10),
            ("midtone", (lu > p10) & (lu <= p90)),
            ("highlight_ue_gt_p90", lu > p90),
        ):
            zones[name] = {
                "px": int(m.sum()),
                "ue_mean": float(lu[m].mean()),
                "rurix_mean": float(lr[m].mean()),
                "mean_abs_diff": float(np.abs(lu - lr)[m].mean()),
                "rurix_over_ue": float(lr[m].mean() / lu[m].mean()) if lu[m].mean() > 0 else None,
            }
        # 32×18 块区 log10(ue/rurix) 网格（HDR 与 LDR 双域）
        h, w = lu.shape
        grid = {"w": 32, "h": 18}
        for dom, aa, bb in (("ldr", lu, lr), ("hdr", lum(hdr_u), lum(hdr_r))):
            cells = []
            for gy in range(18):
                for gx in range(32):
                    ys = slice(gy * h // 18, (gy + 1) * h // 18)
                    xs = slice(gx * w // 32, (gx + 1) * w // 32)
                    mu = float(np.median(aa[ys, xs]))
                    mr = float(np.median(bb[ys, xs]))
                    cells.append(None if mr <= 0 else round(float(np.log10(mu / mr)), 4))
            grid[f"log10_ratio_median_{dom}"] = cells
        diff_out[scene] = {
            "zone_split_points": {"p10": p10, "p90": p90},
            "zones": zones,
            "grid": grid,
        }
        tag = "cornell" if scene == "cornell-box" else "bistro"
        eps = 1e-9
        gray_png(np.log10(lum(hdr_u) + eps), DIAG / f"{tag}_ue_hdr_log10.png", -3.0, 2.0)
        gray_png(np.log10(lum(hdr_r) + eps), DIAG / f"{tag}_rurix_hdr_log10.png", -3.0, 2.0)
        gray_png(np.abs(lu - lr), DIAG / f"{tag}_ldr_absdiff.png", 0.0, 0.5)
        ratio = np.log10((lu + 1e-6) / (lr + 1e-6))
        gray_png(np.clip(ratio, -1.0, 3.0), DIAG / f"{tag}_ldr_log10ratio.png", -1.0, 3.0)
    out["diff_heatmap"] = diff_out

    # ── ③b UE 诊断变体分解（sky0 = SkyLight 关 / nospec = 反射路径关 MRQ 臂）──
    ue_diag = {}
    ue_diag_root = DIAG.parent / "ue_diag"
    p_sky0 = ue_diag_root / "bistro-interior-sky0" / ".0000.exr"
    p_nospec = ue_diag_root / "bistro-interior-nospec" / ".0000.exr"
    if p_sky0.is_file() and p_nospec.is_file():
        _, base = load(G11_5 / "ue" / "bistro-interior" / ".0000.exr", "ue5")
        _, sky0 = load(p_sky0, "ue5")
        _, nospec = load(p_nospec, "ue5")
        lb, l0, ln = lum(base), lum(sky0), lum(nospec)
        sky_total = np.clip(lb - l0, 0.0, None)
        spec_path = np.clip(lb - ln, 0.0, None)
        sky_diff = np.clip(ln - l0, 0.0, None)
        sky_spec = np.clip(sky_total - sky_diff, 0.0, None)
        ue_diag = {
            "base_digest": exr.frame_content_digest(
                base.shape[1], base.shape[0], 3,
                [float(v) for v in np.asarray(base, dtype=np.float32).ravel()]),
            "base_lum": stats(lb),
            "sky0_lum": stats(l0),
            "nospec_lum": stats(ln),
            "sky_total(=base-sky0)": {
                "mean": float(sky_total.mean()), "median": float(np.median(sky_total)),
                "p90": float(np.quantile(sky_total, 0.9)), "max": float(sky_total.max()),
                "share_of_base_mean": float(sky_total.mean() / lb.mean()),
            },
            "spec_path(=base-nospec)": {
                "mean": float(spec_path.mean()), "median": float(np.median(spec_path)),
                "p90": float(np.quantile(spec_path, 0.9)), "max": float(spec_path.max()),
                "share_of_base_mean": float(spec_path.mean() / lb.mean()),
            },
            "sky_diffuse(=nospec-sky0)": {
                "mean": float(sky_diff.mean()), "median": float(np.median(sky_diff)),
                "p90": float(np.quantile(sky_diff, 0.9)),
                "share_of_base_mean": float(sky_diff.mean() / lb.mean()),
            },
            "sky_specular(=sky_total-sky_diffuse)": {
                "mean": float(sky_spec.mean()), "median": float(np.median(sky_spec)),
                "p90": float(np.quantile(sky_spec, 0.9)),
                "share_of_base_mean": float(sky_spec.mean() / lb.mean()),
            },
        }
        gray_png(sky_total, DIAG / "bistro_ue_sky_total.png", 0.0, 1.0)
        gray_png(spec_path, DIAG / "bistro_ue_spec_path.png", 0.0, 1.0)
        gray_png(l0, DIAG / "bistro_ue_sky0_lum.png", 0.0, 1.0)
    out["ue_variant_decomposition"] = ue_diag

    # ── ④ 天空/太阳可见性审计汇总（bin --diag-sky-vis 产物只读消费）──
    skyvis = {}
    for tag in ("bistro", "cornell"):
        p = DIAG / f"skyvis_{tag}.json"
        if p.is_file():
            doc = json.loads(p.read_text(encoding="utf-8-sig"))
            skyvis[tag] = doc
            # 网格 PNG（天空可见率均值）
            g = doc["grid"]
            arr = np.array(
                [np.nan if v is None else v for v in g["sky_vis_mean"]], dtype=np.float64
            ).reshape(g["h"], g["w"])
            arr = np.nan_to_num(arr, nan=0.0)
            gray_png(arr, DIAG / f"{tag}_skyvis_grid.png", 0.0, 1.0)
            garr = np.array(
                [np.nan if v is None else v for v in g["glass_block_frac"]], dtype=np.float64
            ).reshape(g["h"], g["w"])
            gray_png(np.nan_to_num(garr, nan=0.0), DIAG / f"{tag}_glassblock_grid.png", 0.0, 1.0)
    out["sky_visibility_audit"] = {
        t: {
            "covered_points": d["covered_points"],
            "sky_visibility": d["sky_visibility"],
            "glass_blocked_ray_share": d["glass_blocked_ray_share"],
            "sun": d["sun"],
            "hemisphere_blockers": d["hemisphere_blockers"][:8],
            "sun_blockers": d["sun_blockers"][:8],
            "glass_materials": d["glass_materials"],
        }
        for t, d in skyvis.items()
    }

    print(json.dumps(out, ensure_ascii=False, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
