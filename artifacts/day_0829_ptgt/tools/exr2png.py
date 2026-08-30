#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Day 0829 PT 真值对照战役:scene-linear HDR EXR → 可看 PNG。

用途:把 PT 真值(或任何 rurix 端 scene-linear EXR)烘成 8bit PNG 供人眼对照。
EXR 用 ci/g10_exr_lib.decode_exr(expected_end="rurix") 独立解码器读入
(NONE 压缩 scanline、RGB float、fail-closed);像素为**未施加曝光**的
scene-linear 值。处理链 = 曝光 → tonemap → gamma 编码 → uint8 PNG:

  1. 曝光:linear ×= 2**(-ev100) × gain(默认 ev100=-4 即 ×16,bistro 契约
     口径);负值 clamp 到 0(scene-linear 辐射非负,负值只可能是数值噪声);
  2. tonemap:aces(默认)= Narkowicz 2015 拟合
     x(2.51x+0.03)/(x(2.43x+0.59)+0.14) 后 clamp[0,1];
     reinhard = x/(1+x);none = clamp[0,1];
  3. gamma 编码 y = t**(1/gamma)(默认 2.2)→ ×255+0.5 截断 → uint8。

登记口径:mean_luma_linear = **曝光前** scene-linear 域 Rec.709 亮度均值
(0.2126/0.7152/0.0722;表征 EXR 本身,与 ev100/gain 取值无关)。

用法:
  py -3 exr2png.py <in.exr> [out.png] [--ev100 -4.0] [--gain 1.0]
                   [--tonemap aces|reinhard|none] [--gamma 2.2]
  py -3 exr2png.py --selftest   # 合成数组全链自测(不落盘、不依赖 EXR 编码器)

输出:一行 JSON(schema=rurix.day0829.ptgt.exr2png.v1)到 stdout。
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
from g10_exr_lib import decode_exr  # noqa: E402(依赖上面的 ROOT 路径注入)

SCHEMA = "rurix.day0829.ptgt.exr2png.v1"
SCHEMA_SELFTEST = "rurix.day0829.ptgt.exr2png.selftest.v1"


def luma_of(rgb: np.ndarray) -> np.ndarray:
    """Rec.709 亮度(day_0828/0829 全部工具同口径)。"""
    return rgb[..., 0] * 0.2126 + rgb[..., 1] * 0.7152 + rgb[..., 2] * 0.0722


def load_exr_rgb(path: str) -> np.ndarray:
    """rurix 端 EXR → scene-linear RGB float64 (H, W, 3);解码违例 fail-closed。"""
    try:
        buf = Path(path).read_bytes()
    except OSError as e:
        raise SystemExit(f"FAIL: 读取 {path} 失败: {e}")
    try:
        d = decode_exr(buf, expected_end="rurix")
    except Exception as e:
        raise SystemExit(f"FAIL: EXR 解码失败 {path}: {e}")
    if d["layout"] != "rgb":
        raise SystemExit(f"FAIL: {path} layout={d['layout']!r} 非 rgb(本工具只接 RGB EXR)")
    w, h = d["width"], d["height"]
    return np.asarray(d["pixels"], dtype=np.float64).reshape(h, w, 3)


def apply_chain(linear: np.ndarray, ev100: float, gain: float,
                tonemap: str, gamma: float) -> np.ndarray:
    """曝光 → tonemap → gamma 编码 → uint8(逐元素,任意形状)。"""
    if gain <= 0.0:
        raise SystemExit(f"FAIL: --gain 须 > 0,得到 {gain}")
    if gamma <= 0.0:
        raise SystemExit(f"FAIL: --gamma 须 > 0,得到 {gamma}")
    x = np.maximum(np.asarray(linear, dtype=np.float64) * (2.0 ** (-ev100)) * gain, 0.0)
    if tonemap == "aces":
        # Narkowicz 2015 ACES 拟合;分母 2.43x²+0.59x+0.14 在 x≥0 恒 >0。
        t = np.clip(x * (2.51 * x + 0.03) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0)
    elif tonemap == "reinhard":
        t = x / (1.0 + x)
    elif tonemap == "none":
        t = np.clip(x, 0.0, 1.0)
    else:
        raise SystemExit(f"FAIL: --tonemap 未知 {tonemap!r}")
    enc = np.power(t, 1.0 / gamma)
    return (np.clip(enc, 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8)


# ---------------------------------------------------------------- 自测面 ----

def expect_systemexit(fn, what: str) -> None:
    try:
        fn()
    except SystemExit:
        return
    raise SystemExit(f"FAIL: 自测预期 SystemExit 未发生: {what}")


def run_selftest() -> int:
    """合成数组全链自测:单调性、[0,255] 域、锚点值;不落盘、不依赖编码器。"""
    cases: dict = {}

    # ① 逐 tonemap:x=0→0、大 x→接近 255、升序输入单调、uint8 值域。
    xs = np.concatenate([[0.0], np.geomspace(1e-4, 128.0, 61)])
    for tm in ("aces", "reinhard", "none"):
        u8 = apply_chain(xs, ev100=0.0, gain=1.0, tonemap=tm, gamma=2.2)
        assert u8.dtype == np.uint8, tm
        assert int(u8[0]) == 0, f"{tm}: x=0 应映射 0,得到 {u8[0]}"
        assert int(u8[-1]) >= 254, f"{tm}: x=128 应接近 255,得到 {u8[-1]}"
        assert np.all(np.diff(u8.astype(np.int64)) >= 0), f"{tm}: 非单调"
        assert int(u8.min()) >= 0 and int(u8.max()) <= 255, tm
        cases[f"tm_{tm}"] = {"u8_at_0": int(u8[0]), "u8_at_x128": int(u8[-1])}
    # aces 大 x 渐近 2.51/2.43 > 1 ⇒ clamp 后精确 255。
    assert int(apply_chain(np.array([100.0]), 0.0, 1.0, "aces", 2.2)[0]) == 255

    # ② 锚点:aces(1)=2.54/3.16≈0.80380→205(γ=1);reinhard(1)=0.5→γ2.2→186。
    a1 = int(apply_chain(np.array([1.0]), 0.0, 1.0, "aces", 1.0)[0])
    r1 = int(apply_chain(np.array([1.0]), 0.0, 1.0, "reinhard", 2.2)[0])
    assert a1 == 205, f"aces(1) γ=1 应 205,得到 {a1}"
    assert r1 == 186, f"reinhard(1) γ=2.2 应 186,得到 {r1}"
    cases["anchor"] = {"aces1_gamma1": a1, "reinhard1_gamma22": r1}

    # ③ 曝光:ev100=-4 ⇒ ×16(bistro 契约);gain 纯乘;负输入 clamp 0。
    assert int(apply_chain(np.array([1.0 / 16.0]), -4.0, 1.0, "none", 1.0)[0]) == 255
    assert int(apply_chain(np.array([1.0 / 32.0]), -4.0, 1.0, "none", 1.0)[0]) == 128
    assert int(apply_chain(np.array([0.5]), 0.0, 2.0, "none", 1.0)[0]) == 255
    assert int(apply_chain(np.array([-0.5]), 0.0, 1.0, "none", 1.0)[0]) == 0
    cases["exposure"] = {"x1_16_ev-4_u8": 255, "x1_32_ev-4_u8": 128, "neg_clamp_u8": 0}

    # ④ 全链数组形状/默认参数(aces,ev100=-4,γ2.2)+ 渐变轴单调。
    grad = np.tile(np.linspace(0.0, 4.0, 8)[None, :, None], (4, 1, 3))
    u8 = apply_chain(grad, -4.0, 1.0, "aces", 2.2)
    assert u8.shape == (4, 8, 3) and u8.dtype == np.uint8
    assert np.all(np.diff(u8[..., 0].astype(np.int64), axis=1) >= 0)
    cases["chain_grad_8x4"] = {"first_u8": int(u8[0, 0, 0]), "last_u8": int(u8[0, -1, 0])}

    # ⑤ Rec.709 luma 权重口径。
    assert abs(float(luma_of(np.ones((1, 1, 3)))[0, 0]) - 1.0) < 1e-12
    assert abs(float(luma_of(np.array([[[1.0, 0.0, 0.0]]]))[0, 0]) - 0.2126) < 1e-15
    cases["luma"] = {"white": 1.0, "red_weight": 0.2126}

    # ⑥ 非法参数 fail-closed。
    expect_systemexit(lambda: apply_chain(np.array([1.0]), 0.0, 0.0, "none", 1.0), "gain=0")
    expect_systemexit(lambda: apply_chain(np.array([1.0]), 0.0, 1.0, "none", 0.0), "gamma=0")
    cases["fail_closed"] = ["gain<=0", "gamma<=0"]

    print(json.dumps({"schema": SCHEMA_SELFTEST, "cases": cases, "all_pass": True},
                     ensure_ascii=False, indent=1))
    return 0


# ---------------------------------------------------------------- CLI 面 ----

def main() -> int:
    # Windows GBK console 防线:统一 stdout 为 UTF-8(ab_metrics 同律)。
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("in_exr", nargs="?", help="输入 scene-linear EXR(rurix 端)")
    ap.add_argument("out_png", nargs="?", help="输出 PNG(缺省 = 输入同名 .png)")
    ap.add_argument("--ev100", type=float, default=-4.0,
                    help="曝光 EV100(默认 -4 即 ×16,bistro 契约口径)")
    ap.add_argument("--gain", type=float, default=1.0, help="附加线性增益(默认 1.0)")
    ap.add_argument("--tonemap", choices=("aces", "reinhard", "none"), default="aces",
                    help="色调映射(默认 aces = Narkowicz 2015 拟合)")
    ap.add_argument("--gamma", type=float, default=2.2,
                    help="显示伽马(编码指数 1/γ,默认 2.2)")
    ap.add_argument("--selftest", action="store_true",
                    help="合成数组全链自测(不落盘、不依赖 EXR 编码器)")
    args = ap.parse_args()

    if args.selftest:
        return run_selftest()
    if not args.in_exr:
        raise SystemExit("FAIL: 须给 <in.exr>(或 --selftest);见 -h")

    rgb = load_exr_rgb(args.in_exr)
    h, w = rgb.shape[:2]
    u8 = apply_chain(rgb, args.ev100, args.gain, args.tonemap, args.gamma)
    out = args.out_png or str(Path(args.in_exr).with_suffix(".png"))
    Image.fromarray(np.ascontiguousarray(u8), "RGB").save(out)
    print(json.dumps({
        "schema": SCHEMA,
        "in": args.in_exr,
        "out": out,
        "w": w,
        "h": h,
        "mean_luma_linear": float(luma_of(rgb).mean()),
        "ev100": args.ev100,
        "tonemap": args.tonemap,
        "gain": args.gain,
        "gamma": args.gamma,
    }, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
