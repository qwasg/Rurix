#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Day 0829 真实感战役:六画质臂 A/B 判据单入口指标工具。

输入 = g31_window_present presented raw dump(day_0828 同款,源码确认于
src/rurix-render/src/bin/g31_window_present.rs 写盘段):
  字节布局 = w:u32 LE(4B)+ h:u32 LE(4B)+ BGRA8 打包像素(w*h*4 B,u8)。
  注意:是 display-encode 后的 8bit presented 字节,**不是 f32**
  (f32 仅存在于 RURIX_G31_DUMP_F32 env 门控的 TEMP 归因 dump,无头、非验收面)。
  本工具读入后 BGRA→RGB、/255 归一化到 [0,1] float64 再算指标。

子命令(全部输出 JSON 到 stdout,--out 可选另存):
  luma  <raw>                               全屏亮度统计(linear + log2 域)
  crop  <raw> --rect x,y,w,h [...]          矩形区域统计(luma + RGB 均值/占比)
  diff  <a.raw> <b.raw> [--rect ...]        双 raw 差分(SAD 均值/max/变化像素占比)
  grad  <raw> [--rect ...]                  Sobel 梯度能量(法线/反射细节增量面)
  edge  <raw> --rect x,y,w,h --axis x|y     亮度过渡带 10-90 宽度(软阴影半影面)
  noise <r1> <r2> [r3 ...] [--rect ...]     多帧帧间方差(时域噪声面)
  selftest                                  numpy 合成 8x8 raw 自测全部子命令

用法示例:
  py -3 ab_metrics.py luma on.raw --out luma_on.json
  py -3 ab_metrics.py crop on.raw --rect 1330,270,590,230 --label metal_pot
  py -3 ab_metrics.py diff off.raw on.raw --rect 100,100,200,200
  py -3 ab_metrics.py edge on.raw --rect 700,700,300,120 --axis x
  py -3 ab_metrics.py noise on.raw.f0080 on.raw.f0160 on.raw.f0240
"""
from __future__ import annotations

import argparse
import json
import math
import struct
import sys
from pathlib import Path

import numpy as np

# 亮度域下界(log2 域防 -inf;u8/255 最小非零 ≈ 0.0039,eps 远低于它)。
EPS = 1e-6
# diff 变化判定默认阈值:源数据均为 u8/255 ⇒ 差值只能是 k/255;
# 0.5/255 精确等价于"u8 域 ≥1 级差",高于任何浮点噪声。
DIFF_THRESH = 0.5 / 255.0


# ---------------------------------------------------------------- 基础 IO ----

def load_raw(path: str) -> np.ndarray:
    """读 presented raw dump → RGB float64 [0,1],shape (H, W, 3)。"""
    b = Path(path).read_bytes()
    if len(b) < 8:
        raise SystemExit(f"FAIL: {path} 不足 8B 头(w/h u32 LE)")
    w, h = struct.unpack_from("<II", b, 0)
    need = 8 + w * h * 4
    if len(b) < need:
        raise SystemExit(f"FAIL: {path} 长度 {len(b)} < 头声明 {need}(w={w},h={h})")
    a = np.frombuffer(b, dtype=np.uint8, offset=8, count=w * h * 4).reshape(h, w, 4)
    # swapchain bgra8_unorm ⇒ 字节序 [b,g,r,a] → 取 RGB。
    return a[:, :, [2, 1, 0]].astype(np.float64) / 255.0


def luma_of(rgb: np.ndarray) -> np.ndarray:
    """Rec.709 亮度(day_0828 全部工具同口径)。"""
    return rgb[..., 0] * 0.2126 + rgb[..., 1] * 0.7152 + rgb[..., 2] * 0.0722


def parse_rect(s: str, w: int, h: int) -> tuple[int, int, int, int]:
    """解析 'x,y,w,h' 并做边界校验(越界 fail-closed,不静默裁剪)。"""
    try:
        x, y, rw, rh = (int(v) for v in s.split(","))
    except ValueError:
        raise SystemExit(f"FAIL: --rect 须 'x,y,w,h' 四整数,得到 {s!r}")
    if rw <= 0 or rh <= 0 or x < 0 or y < 0 or x + rw > w or y + rh > h:
        raise SystemExit(f"FAIL: --rect {s} 越界(图像 {w}x{h})")
    return x, y, rw, rh


def rect_labels(rects: list[str], labels: list[str] | None) -> list[str]:
    """--label 逐 rect 命名;缺省 rect0/rect1/...;数量不匹配 fail-closed。"""
    if not labels:
        return [f"rect{i}" for i in range(len(rects))]
    if len(labels) != len(rects):
        raise SystemExit(f"FAIL: --label 数 {len(labels)} != --rect 数 {len(rects)}")
    return labels


def stats_pair(v: np.ndarray) -> dict:
    """mean/p1/p50/p99/max 双域:linear + log2(max(v,EPS))。"""
    lg = np.log2(np.maximum(v, EPS))

    def block(a: np.ndarray) -> dict:
        return {
            "mean": float(a.mean()),
            "p1": float(np.percentile(a, 1)),
            "p50": float(np.percentile(a, 50)),
            "p99": float(np.percentile(a, 99)),
            "max": float(a.max()),
        }

    return {"linear": block(v), "log2": block(lg)}


def emit(res: dict, out: str | None) -> None:
    txt = json.dumps(res, indent=1, ensure_ascii=False)
    print(txt)
    if out:
        Path(out).write_text(txt + "\n", encoding="utf-8")


# ------------------------------------------------------------ 指标核心面 ----

def sobel_mag(l: np.ndarray) -> np.ndarray:
    """Sobel 3x3 幅值(valid 区,输入 (H,W) 须 ≥3x3;不依赖 scipy)。"""
    if l.shape[0] < 3 or l.shape[1] < 3:
        raise SystemExit(f"FAIL: grad 区域须 ≥3x3,得到 {l.shape}")
    gx = (l[:-2, 2:] + 2.0 * l[1:-1, 2:] + l[2:, 2:]) \
       - (l[:-2, :-2] + 2.0 * l[1:-1, :-2] + l[2:, :-2])
    gy = (l[2:, :-2] + 2.0 * l[2:, 1:-1] + l[2:, 2:]) \
       - (l[:-2, :-2] + 2.0 * l[:-2, 1:-1] + l[:-2, 2:])
    return np.hypot(gx, gy)


def grad_stats(l: np.ndarray) -> dict:
    g = sobel_mag(l)
    return {
        "grad_mean": float(g.mean()),
        "grad_p95": float(np.percentile(g, 95)),
        "grad_max": float(g.max()),
        "valid_px": int(g.size),
    }


def cross_pos(p: np.ndarray, level: float) -> float | None:
    """1D 归一化剖面首次跨 level 的线性插值位置(未跨返回 None)。"""
    above = p >= level
    if above[0]:
        return 0.0
    idx = np.argmax(above)  # 首个 True;全 False 时 argmax=0 且 above[0]=False
    if not above[idx]:
        return None
    lo, hi = p[idx - 1], p[idx]
    if hi <= lo:  # 数值防御(理论上 hi>=level>lo)
        return float(idx)
    return float(idx - 1) + (level - lo) / (hi - lo)


def width_10_90(profile: np.ndarray, min_contrast: float) -> float | None:
    """亮度剖面 10%→90% 过渡带宽度(px);对比度不足或未形成过渡返回 None。

    方向归一:剖面按"首尾均值比较"翻转成暗→亮再找首跨(宽度对翻转不变)。
    """
    if profile.size < 2:
        return None
    vmin, vmax = float(profile.min()), float(profile.max())
    if vmax - vmin < min_contrast:
        return None
    k = max(2, profile.size // 4)
    if profile[:k].mean() > profile[-k:].mean():
        profile = profile[::-1]
    p = (profile - vmin) / (vmax - vmin)
    x10 = cross_pos(p, 0.1)
    x90 = cross_pos(p, 0.9)
    if x10 is None or x90 is None:
        return None
    return abs(x90 - x10)


# -------------------------------------------------------------- 子命令面 ----

def cmd_luma(args) -> dict:
    rgb = load_raw(args.raw)
    return {
        "schema": "rurix.day0829.ab_metrics.luma.v1",
        "raw": args.raw,
        "width": rgb.shape[1],
        "height": rgb.shape[0],
        "luma": stats_pair(luma_of(rgb)),
    }


def cmd_crop(args) -> dict:
    rgb = load_raw(args.raw)
    h, w = rgb.shape[:2]
    labels = rect_labels(args.rect, args.label)
    crops: dict = {}
    for s, name in zip(args.rect, labels):
        x, y, rw, rh = parse_rect(s, w, h)
        c = rgb[y : y + rh, x : x + rw]
        mean_rgb = [float(v) for v in c.reshape(-1, 3).mean(0)]
        tot = sum(mean_rgb)
        crops[name] = {
            "window": [x, y, rw, rh],
            "luma": stats_pair(luma_of(c)),
            "mean_rgb": mean_rgb,
            # 通道占比(和归一;GI2tex 臂色偏判据:向反弹面贴图色偏移)。
            "rgb_frac": [v / tot for v in mean_rgb] if tot > EPS else None,
        }
    return {
        "schema": "rurix.day0829.ab_metrics.crop.v1",
        "raw": args.raw,
        "width": w,
        "height": h,
        "crops": crops,
    }


def cmd_diff(args) -> dict:
    a = load_raw(args.raw_a)
    b = load_raw(args.raw_b)
    if a.shape != b.shape:
        raise SystemExit(f"FAIL: 两 raw 尺寸不一 {a.shape} vs {b.shape}")
    h, w = a.shape[:2]

    def block(pa: np.ndarray, pb: np.ndarray) -> dict:
        d = np.abs(pa - pb)                      # (h,w,3) 逐通道绝对差
        changed = (d > args.thresh).any(axis=2)  # 任一通道超阈 = 变化像素
        return {
            "sad_mean": float(d.mean()),
            "sad_max": float(d.max()),
            "changed_frac": float(changed.mean()),
            "changed_px": int(changed.sum()),
            "total_px": int(changed.size),
        }

    res: dict = {
        "schema": "rurix.day0829.ab_metrics.diff.v1",
        "raw_a": args.raw_a,
        "raw_b": args.raw_b,
        "width": w,
        "height": h,
        "thresh": args.thresh,
        "full": block(a, b),
    }
    if args.rect:
        labels = rect_labels(args.rect, args.label)
        res["crops"] = {}
        for s, name in zip(args.rect, labels):
            x, y, rw, rh = parse_rect(s, w, h)
            res["crops"][name] = {
                "window": [x, y, rw, rh],
                **block(a[y : y + rh, x : x + rw], b[y : y + rh, x : x + rw]),
            }
    return res


def cmd_grad(args) -> dict:
    rgb = load_raw(args.raw)
    l = luma_of(rgb)
    h, w = l.shape
    res: dict = {
        "schema": "rurix.day0829.ab_metrics.grad.v1",
        "raw": args.raw,
        "width": w,
        "height": h,
        "full": grad_stats(l),
    }
    if args.rect:
        labels = rect_labels(args.rect, args.label)
        res["crops"] = {}
        for s, name in zip(args.rect, labels):
            x, y, rw, rh = parse_rect(s, w, h)
            res["crops"][name] = {
                "window": [x, y, rw, rh],
                **grad_stats(l[y : y + rh, x : x + rw]),
            }
    return res


def cmd_edge(args) -> dict:
    rgb = load_raw(args.raw)
    l = luma_of(rgb)
    h, w = l.shape
    x, y, rw, rh = parse_rect(args.rect[0], w, h)
    c = l[y : y + rh, x : x + rw]
    # axis x = 过渡沿水平方向展开(扫描线为行);axis y = 沿垂直(扫描线为列)。
    lines = c if args.axis == "x" else c.T
    # 平均剖面(垂直于过渡方向平均,抗噪)。
    profile = lines.mean(axis=0)
    pw = width_10_90(profile, args.min_contrast)
    # 逐线宽度(对斜置边缘更稳;对比度不足的线跳过)。
    per = [width_10_90(ln, args.min_contrast) for ln in lines]
    valid = [v for v in per if v is not None]
    return {
        "schema": "rurix.day0829.ab_metrics.edge.v1",
        "raw": args.raw,
        "window": [x, y, rw, rh],
        "axis": args.axis,
        "min_contrast": args.min_contrast,
        "profile_min": float(profile.min()),
        "profile_max": float(profile.max()),
        "profile_contrast": float(profile.max() - profile.min()),
        # 平均剖面的 10-90 宽度(px;软影臂主判据:on 臂展宽 ↑)。
        "profile_width_10_90_px": pw,
        "line_width_mean_px": float(np.mean(valid)) if valid else None,
        "line_width_p50_px": float(np.median(valid)) if valid else None,
        "lines_used": len(valid),
        "lines_total": int(lines.shape[0]),
    }


def cmd_noise(args) -> dict:
    if len(args.raws) < 2:
        raise SystemExit("FAIL: noise 须 ≥2 帧 raw")
    stack = np.stack([luma_of(load_raw(p)) for p in args.raws], axis=0)
    h, w = stack.shape[1:]

    def block(s: np.ndarray) -> dict:
        tstd = s.std(axis=0)   # 逐像素跨帧 std(d_metrics/c_noise 同口径)
        tmean = s.mean(axis=0)
        rel = tstd / np.maximum(tmean, 1e-4)
        return {
            "temporal_std_mean": float(tstd.mean()),
            "temporal_std_p95": float(np.percentile(tstd, 95)),
            "temporal_rel_mean": float(rel.mean()),
            "temporal_rel_p95": float(np.percentile(rel, 95)),
            "mean_luma": float(tmean.mean()),
        }

    res: dict = {
        "schema": "rurix.day0829.ab_metrics.noise.v1",
        "raws": list(args.raws),
        "frames_used": len(args.raws),
        "width": w,
        "height": h,
        "full": block(stack),
    }
    if args.rect:
        labels = rect_labels(args.rect, args.label)
        res["crops"] = {}
        for s, name in zip(args.rect, labels):
            x, y, rw, rh = parse_rect(s, w, h)
            res["crops"][name] = {
                "window": [x, y, rw, rh],
                **block(stack[:, y : y + rh, x : x + rw]),
            }
    return res


# ---------------------------------------------------------------- 自测面 ----

def write_raw(path: Path, rgb01: np.ndarray) -> None:
    """RGB float [0,1] (H,W,3) → dump 同款字节(w/h u32 LE 头 + BGRA8)。"""
    h, w = rgb01.shape[:2]
    u8 = (np.clip(rgb01, 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8)
    bgra = np.zeros((h, w, 4), dtype=np.uint8)
    bgra[:, :, 0] = u8[:, :, 2]
    bgra[:, :, 1] = u8[:, :, 1]
    bgra[:, :, 2] = u8[:, :, 0]
    bgra[:, :, 3] = 255
    path.write_bytes(struct.pack("<II", w, h) + bgra.tobytes())


def assert_finite(node, trail: str = "") -> None:
    """递归断言输出树中全部数值有限(自测判据:跑通不 NaN/Inf)。"""
    if isinstance(node, dict):
        for k, v in node.items():
            assert_finite(v, f"{trail}.{k}")
    elif isinstance(node, (list, tuple)):
        for i, v in enumerate(node):
            assert_finite(v, f"{trail}[{i}]")
    elif isinstance(node, float):
        if not math.isfinite(node):
            raise SystemExit(f"FAIL: 自测输出含非有限值 {trail}={node}")


def cmd_selftest(_args) -> dict:
    import tempfile

    summary: dict = {"schema": "rurix.day0829.ab_metrics.selftest.v1", "cases": {}}
    with tempfile.TemporaryDirectory() as td:
        d = Path(td)
        n = 8
        # 合成集:恒定色 / 更亮恒定色 / 水平渐变 / 两帧确定性噪声。
        flat = np.tile(np.array([0.25, 0.5, 0.75]), (n, n, 1))
        bright = np.clip(flat + 0.1, 0, 1)
        ramp = np.zeros((n, n, 3))
        ramp[:, :, :] = (np.arange(n) / (n - 1))[None, :, None]  # x 向 0→1
        rng = np.random.default_rng(829)
        noisy_a = np.clip(flat + rng.normal(0, 0.03, (n, n, 3)), 0, 1)
        noisy_b = np.clip(flat + rng.normal(0, 0.03, (n, n, 3)), 0, 1)
        paths = {}
        for name, img in [("flat", flat), ("bright", bright), ("ramp", ramp),
                          ("noisy_a", noisy_a), ("noisy_b", noisy_b)]:
            p = d / f"{name}.raw"
            write_raw(p, img)
            paths[name] = str(p)

        ap = build_parser()

        def run(argv: list[str]) -> dict:
            a = ap.parse_args(argv)
            r = a.func(a)
            assert_finite(r)
            return r

        # ① luma:恒定色全屏统计(mean == p50)。
        r = run(["luma", paths["flat"]])
        assert abs(r["luma"]["linear"]["mean"] - r["luma"]["linear"]["p50"]) < 1e-9
        summary["cases"]["luma"] = {"mean": r["luma"]["linear"]["mean"],
                                    "log2_mean": r["luma"]["log2"]["mean"]}

        # ② crop:双 rect + rgb_frac(恒定色占比 = 分量/和)。
        r = run(["crop", paths["flat"], "--rect", "2,2,4,4", "--rect", "0,0,3,3",
                 "--label", "a", "--label", "b"])
        assert r["crops"]["a"]["rgb_frac"] is not None
        summary["cases"]["crop"] = {"a_mean": r["crops"]["a"]["luma"]["linear"]["mean"],
                                    "a_rgb_frac": r["crops"]["a"]["rgb_frac"]}

        # ③ diff:自差全零;flat vs bright 全像素变化。
        r0 = run(["diff", paths["flat"], paths["flat"]])
        assert r0["full"]["sad_mean"] == 0.0 and r0["full"]["changed_frac"] == 0.0
        r1 = run(["diff", paths["flat"], paths["bright"], "--rect", "2,2,4,4"])
        assert r1["full"]["changed_frac"] == 1.0
        summary["cases"]["diff"] = {"self_sad": r0["full"]["sad_mean"],
                                    "bright_changed_frac": r1["full"]["changed_frac"],
                                    "bright_sad_mean": r1["full"]["sad_mean"]}

        # ④ grad:恒定色 =0,渐变 >0。
        rf = run(["grad", paths["flat"]])
        rr = run(["grad", paths["ramp"], "--rect", "1,1,6,6"])
        assert rf["full"]["grad_mean"] == 0.0 and rr["full"]["grad_mean"] > 0.0
        summary["cases"]["grad"] = {"flat_mean": rf["full"]["grad_mean"],
                                    "ramp_mean": rr["full"]["grad_mean"]}

        # ⑤ edge:x 向 0→1 线性渐变,10-90 宽 ≈ 0.8×(n-1) = 5.6 px。
        r = run(["edge", paths["ramp"], "--rect", "0,0,8,8", "--axis", "x"])
        assert r["profile_width_10_90_px"] is not None
        assert 4.0 < r["profile_width_10_90_px"] < 7.0
        summary["cases"]["edge"] = {"profile_width_10_90_px": r["profile_width_10_90_px"],
                                    "line_width_mean_px": r["line_width_mean_px"],
                                    "lines_used": r["lines_used"]}

        # ⑥ noise:同帧×2 =0;两噪声帧 >0。
        r0 = run(["noise", paths["flat"], paths["flat"]])
        assert r0["full"]["temporal_std_mean"] == 0.0
        r1 = run(["noise", paths["noisy_a"], paths["noisy_b"], "--rect", "2,2,4,4"])
        assert r1["full"]["temporal_std_mean"] > 0.0
        summary["cases"]["noise"] = {"flat_std": r0["full"]["temporal_std_mean"],
                                     "noisy_std": r1["full"]["temporal_std_mean"],
                                     "noisy_rel_p95": r1["full"]["temporal_rel_p95"]}

    summary["all_pass"] = True
    return summary


# ---------------------------------------------------------------- CLI 面 ----

def add_common(p: argparse.ArgumentParser, rects: bool = True) -> None:
    if rects:
        p.add_argument("--rect", action="append", default=[],
                       help="矩形区域 'x,y,w,h',可多次")
        p.add_argument("--label", action="append", default=[],
                       help="逐 --rect 命名(次数须匹配),缺省 rect0/1/...")
    p.add_argument("--out", default=None, help="JSON 另存路径(stdout 恒打印)")


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("luma", help="全屏亮度统计(linear+log2)")
    p.add_argument("raw")
    add_common(p, rects=False)
    p.set_defaults(func=cmd_luma)

    p = sub.add_parser("crop", help="矩形区域统计(luma + RGB 占比)")
    p.add_argument("raw")
    add_common(p)
    p.set_defaults(func=cmd_crop)

    p = sub.add_parser("diff", help="双 raw 差分(SAD/变化像素占比)")
    p.add_argument("raw_a")
    p.add_argument("raw_b")
    p.add_argument("--thresh", type=float, default=DIFF_THRESH,
                   help=f"变化像素阈值([0,1] 域,默认 {DIFF_THRESH:.6f} = u8 半级)")
    add_common(p)
    p.set_defaults(func=cmd_diff)

    p = sub.add_parser("grad", help="Sobel 梯度能量(细节增量)")
    p.add_argument("raw")
    add_common(p)
    p.set_defaults(func=cmd_grad)

    p = sub.add_parser("edge", help="亮度过渡带 10-90 宽度(软影半影)")
    p.add_argument("raw")
    p.add_argument("--axis", choices=("x", "y"), required=True,
                   help="过渡带展开方向(x=水平扫描线为行)")
    p.add_argument("--min-contrast", type=float, default=0.05,
                   help="剖面最小对比度(低于视为无过渡,默认 0.05)")
    add_common(p)
    p.set_defaults(func=cmd_edge)

    p = sub.add_parser("noise", help="多帧帧间方差(时域噪声)")
    p.add_argument("raws", nargs="+")
    add_common(p)
    p.set_defaults(func=cmd_noise)

    p = sub.add_parser("selftest", help="合成 8x8 raw 自测全部子命令")
    add_common(p, rects=False)
    p.set_defaults(func=cmd_selftest)
    return ap


def main() -> int:
    # Windows GBK console 防线:统一 stdout 为 UTF-8(JSON 含中文注记)。
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    args = build_parser().parse_args()
    # edge 单 rect 契约(多 rect 语义歧义,fail-closed)。
    if args.cmd == "edge" and len(args.rect) != 1:
        raise SystemExit("FAIL: edge 须给恰好一个 --rect")
    res = args.func(args)
    emit(res, args.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
