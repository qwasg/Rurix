#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Day 0829 PT 真值对照战役:PT 真值 EXR vs 光栅线性帧同域对照器。

用途:F0/反射等画质臂的能量方向验证——把 PT 真值(scene-linear HDR EXR)与
光栅 off/on 两臂线性帧放到同一域、同一网格上,登记"哪臂离 PT 更近"。
**登记不裁决**:closer_to_pt/margin 是登记面,pass/fail 归验收脚本;但输入
格式/形状/尺寸错误一律 SystemExit fail-closed。

同域契约:
- PT EXR:ci/g10_exr_lib.decode_exr(expected_end="rurix") 独立解码器读入,
  **未曝光** scene-linear RGB float;
- 光栅 .bin:g31_window_present 的 RURIX_G31_DUMP_F32=1 落盘格式 = 无头裸
  f32 LE、RGB 3 通道行主序(post-TSR pre-encode scene-linear,未施加 AE)。
  无头 ⇒ 必须 --raster-w/--raster-h 给出,长度须精确 == w*h*3*4 字节;
- 光栅臂亦可给 .exr(同 rurix 端解码);
- **曝光域对齐(首跑实证登记)**:光栅 out_color 已含契约静态曝光
  2^(−ev100)(bistro ev100=−4 ⇒ ×16,scene pass 施加;首跑掩码内均值比
  ≈16× 实证),PT EXR 为未曝光——须以 --pt-gain 2^(−ev100) 把 PT 提到同一
  曝光域再比(默认 1.0 = 两端已同域);AE 反馈两端皆无(无 AE 组合契约)。

对齐与度量(全 float64,luma = Rec.709 0.2126/0.7152/0.0722):
1. 三图各自 box 均值降采样到公共最小网格(min 宽 × min 高,因子逐轴独立;
   要求整除,否则 SystemExit 报全部尺寸);
2. fullscreen:mean_luma_{pt,off,on} 与 ratio_on_off / ratio_pt_off;
3. 臂掩码 = |luma_on−luma_off| / max(luma_off,1e-6) > --mask-rel-thr;
   掩码内 mean_{pt,off,on}、dist_off/dist_on(到 PT 均值的距离)、
   closer_to_pt("on"/"off")、margin = (dist_off−dist_on)/max(mean_pt,1e-6)
   (>0 = on 臂更近);掩码为空时统计字段登记 null(不视为错误);
4. 四 ROI(day_0829 口径,1920×1080 基准坐标按公共网格逐轴等比缩放,端点
   round-half-up 纯整数取整):wall/floor/dark_arch/dark_table 的 luma
   mean_{pt,off,on};
5. 掩码内逐像素 |on−pt|/max(pt,1e-6) 的 p50/p95。

用法:
  py -3 compare_pt_raster.py --pt pt.exr --raster-off off.bin --raster-on on.bin \\
      --raster-w 1920 --raster-h 1080 [--mask-rel-thr 0.02] [--arm f0|refl|other] \\
      [--out report.json]
  py -3 compare_pt_raster.py --selftest   # 合成三图全链自测(含 .bin 往返)

输出:JSON(schema=rurix.day0829.ptgt.compare.v1)到 stdout,--out 可另存。
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
from g10_exr_lib import decode_exr  # noqa: E402(依赖上面的 ROOT 路径注入)

SCHEMA = "rurix.day0829.ptgt.compare.v1"
SCHEMA_SELFTEST = "rurix.day0829.ptgt.compare.selftest.v1"
EPS = 1e-6
# ROI 基准坐标系(day_0829 口径,1920×1080)。
ROI_BASE_W, ROI_BASE_H = 1920, 1080
ROIS_BASE = {
    "wall": (1400, 150, 480, 270),
    "floor": (1100, 800, 480, 270),
    "dark_arch": (360, 0, 360, 180),
    "dark_table": (560, 560, 560, 200),
}


def luma_of(rgb: np.ndarray) -> np.ndarray:
    """Rec.709 亮度(day_0828/0829 全部工具同口径)。"""
    return rgb[..., 0] * 0.2126 + rgb[..., 1] * 0.7152 + rgb[..., 2] * 0.0722


# ---------------------------------------------------------------- 输入面 ----

def load_exr_rgb(path: str, role: str) -> tuple[np.ndarray, dict]:
    """rurix 端 EXR → scene-linear RGB float64 (H,W,3) + 登记元数据。"""
    try:
        buf = Path(path).read_bytes()
    except OSError as e:
        raise SystemExit(f"FAIL: {role} 读取 {path} 失败: {e}")
    try:
        d = decode_exr(buf, expected_end="rurix")
    except Exception as e:
        raise SystemExit(f"FAIL: {role} EXR 解码失败 {path}: {e}")
    if d["layout"] != "rgb":
        raise SystemExit(f"FAIL: {role} {path} layout={d['layout']!r} 非 rgb")
    w, h = d["width"], d["height"]
    img = np.asarray(d["pixels"], dtype=np.float64).reshape(h, w, 3)
    return img, {"path": path, "format": "exr", "w": w, "h": h}


def load_bin_rgb(path: str, rw: int | None, rh: int | None,
                 role: str) -> tuple[np.ndarray, dict]:
    """无头裸 f32 LE RGB 行主序 .bin → float64 (H,W,3) + 登记元数据。"""
    if not rw or not rh or rw <= 0 or rh <= 0:
        raise SystemExit(f"FAIL: {role} 为 .bin(无头裸 f32),须给 --raster-w/--raster-h 正整数")
    try:
        b = Path(path).read_bytes()
    except OSError as e:
        raise SystemExit(f"FAIL: {role} 读取 {path} 失败: {e}")
    need = rw * rh * 3 * 4
    if len(b) != need:
        raise SystemExit(f"FAIL: {role} {path} 长度 {len(b)} ≠ w*h*3*4 = {need}"
                         f"(w={rw},h={rh};RGB f32 LE 无头行主序契约)")
    img = np.frombuffer(b, dtype="<f4").reshape(rh, rw, 3).astype(np.float64)
    if not np.isfinite(img).all():
        raise SystemExit(f"FAIL: {role} {path} 含 NaN/Inf(scene-linear dump 禁入)")
    return img, {"path": path, "format": "bin", "w": rw, "h": rh}


def load_image(path: str, rw: int | None, rh: int | None,
               role: str) -> tuple[np.ndarray, dict]:
    """按后缀分派 .exr/.bin;闭集外后缀 fail-closed。"""
    sfx = Path(path).suffix.lower()
    if sfx == ".exr":
        return load_exr_rgb(path, role)
    if sfx == ".bin":
        return load_bin_rgb(path, rw, rh, role)
    raise SystemExit(f"FAIL: {role} {path} 后缀 {sfx!r} 非 .exr/.bin")


# ---------------------------------------------------------------- 对齐面 ----

def box_down(img: np.ndarray, cw: int, ch: int) -> np.ndarray:
    """整数因子 box 均值降采样 (H,W,3) → (ch,cw,3);不整除 fail-closed。"""
    h, w = img.shape[:2]
    fy, fx = h // ch, w // cw
    if fy * ch != h or fx * cw != w:
        raise SystemExit(f"FAIL: box 降采样 {w}x{h} → {cw}x{ch} 不整除")
    if fy == 1 and fx == 1:
        return img
    return img.reshape(ch, fy, cw, fx, 3).mean(axis=(1, 3))


def _scale_pos(v: int, grid: int, base: int) -> int:
    """round-half-up(v·grid/base) 纯整数取整(确定性,无浮点边界抖动)。"""
    return (2 * v * grid + base) // (2 * base)


def scale_roi(name: str, rect: tuple[int, int, int, int],
              cw: int, ch: int) -> tuple[int, int, int, int]:
    """1080p 基准 ROI → 公共网格坐标(两端点逐轴等比缩放;退化 fail-closed)。"""
    x, y, w, h = rect
    x0 = _scale_pos(x, cw, ROI_BASE_W)
    y0 = _scale_pos(y, ch, ROI_BASE_H)
    x1 = min(_scale_pos(x + w, cw, ROI_BASE_W), cw)
    y1 = min(_scale_pos(y + h, ch, ROI_BASE_H), ch)
    if x1 <= x0 or y1 <= y0:
        raise SystemExit(f"FAIL: ROI {name} {rect} 缩放到公共网格 {cw}x{ch} 后退化 "
                         f"({x0},{y0})-({x1},{y1})")
    return x0, y0, x1 - x0, y1 - y0


# ---------------------------------------------------------------- 度量面 ----

def measure(pt: np.ndarray, off: np.ndarray, on: np.ndarray,
            mask_rel_thr: float) -> dict:
    """三图对齐 + 全部度量(登记面,不裁决)。返回报告主体 dict。"""
    imgs = {"pt": pt, "raster_off": off, "raster_on": on}
    cw = min(i.shape[1] for i in imgs.values())
    ch = min(i.shape[0] for i in imgs.values())
    sizes = ", ".join(f"{k}={v.shape[1]}x{v.shape[0]}" for k, v in imgs.items())
    for k, v in imgs.items():
        h, w = v.shape[:2]
        if w % cw or h % ch:
            raise SystemExit(f"FAIL: {k} {w}x{h} 不能整除降采样到公共网格 "
                             f"{cw}x{ch}({sizes})")
    l = {k: luma_of(box_down(v, cw, ch)) for k, v in imgs.items()}

    mean_pt = float(l["pt"].mean())
    mean_off = float(l["raster_off"].mean())
    mean_on = float(l["raster_on"].mean())
    fullscreen = {
        "mean_luma_pt": mean_pt,
        "mean_luma_off": mean_off,
        "mean_luma_on": mean_on,
        "ratio_on_off": mean_on / max(mean_off, EPS),
        "ratio_pt_off": mean_pt / max(mean_off, EPS),
    }

    # 臂掩码:on/off 相对亮度差超阈的像素(= 臂实际触达区)。
    rel = np.abs(l["raster_on"] - l["raster_off"]) / np.maximum(l["raster_off"], EPS)
    m = rel > mask_rel_thr
    mask_px = int(m.sum())
    mask: dict = {"mask_frac": float(m.mean()), "mask_px": mask_px,
                  "total_px": int(m.size)}
    if mask_px == 0:
        # 臂无可见差异:登记 null,不视为错误(裁决归验收脚本)。
        mask.update({"mean_pt": None, "mean_off": None, "mean_on": None,
                     "dist_off": None, "dist_on": None,
                     "closer_to_pt": None, "margin": None})
        percentiles: dict = {"metric": "|on-pt|/max(pt,1e-6) 掩码内",
                             "p50": None, "p95": None}
    else:
        mp = float(l["pt"][m].mean())
        mo = float(l["raster_off"][m].mean())
        mn = float(l["raster_on"][m].mean())
        dist_off = abs(mo - mp)
        dist_on = abs(mn - mp)
        mask.update({
            "mean_pt": mp, "mean_off": mo, "mean_on": mn,
            "dist_off": dist_off, "dist_on": dist_on,
            "closer_to_pt": "on" if dist_on < dist_off else "off",
            "margin": (dist_off - dist_on) / max(mp, EPS),
        })
        relpix = np.abs(l["raster_on"][m] - l["pt"][m]) / np.maximum(l["pt"][m], EPS)
        percentiles = {
            "metric": "|on-pt|/max(pt,1e-6) 掩码内",
            "p50": float(np.percentile(relpix, 50)),
            "p95": float(np.percentile(relpix, 95)),
        }

    rois: dict = {}
    for name, rect in ROIS_BASE.items():
        x, y, rw, rh = scale_roi(name, rect, cw, ch)
        rois[name] = {
            "base_window_1080p": list(rect),
            "window": [x, y, rw, rh],
            "mean_pt": float(l["pt"][y:y + rh, x:x + rw].mean()),
            "mean_off": float(l["raster_off"][y:y + rh, x:x + rw].mean()),
            "mean_on": float(l["raster_on"][y:y + rh, x:x + rw].mean()),
        }

    return {
        "common_grid": [cw, ch],
        "mask_rel_thr": mask_rel_thr,
        "fullscreen": fullscreen,
        "mask": mask,
        "rois": rois,
        "percentiles": percentiles,
    }


# ---------------------------------------------------------------- 自测面 ----

def expect_systemexit(fn, what: str) -> None:
    try:
        fn()
    except SystemExit:
        return
    raise SystemExit(f"FAIL: 自测预期 SystemExit 未发生: {what}")


def assert_finite_or_null(node, trail: str = "") -> None:
    """递归断言输出树数值全有限(None/str/bool/int 放行)。"""
    if isinstance(node, dict):
        for k, v in node.items():
            assert_finite_or_null(v, f"{trail}.{k}")
    elif isinstance(node, (list, tuple)):
        for i, v in enumerate(node):
            assert_finite_or_null(v, f"{trail}[{i}]")
    elif isinstance(node, float):
        if not math.isfinite(node):
            raise SystemExit(f"FAIL: 输出含非有限值 {trail}={node}")


def run_selftest() -> int:
    """合成三图全链自测:pt=真值(12×8),off=真值−掩码区0.2,on=真值−掩码区0.05;
    off/on 以 24×16 供给走降采样路径 + .bin tempfile 往返;含 fail-closed 面。"""
    import tempfile

    cases: dict = {}
    w12, h8 = 12, 8
    xs = np.linspace(0.2, 0.8, w12)                # 灰度渐变真值(三通道相等)
    pt = np.tile(xs[None, :, None], (h8, 1, 3))
    region = (slice(2, 6), slice(6, 12))           # y∈[2,6)×x∈[6,12) = 24 px
    off12 = pt.copy()
    off12[region] -= 0.2
    on12 = pt.copy()
    on12[region] -= 0.05

    def up2(a: np.ndarray) -> np.ndarray:
        return np.repeat(np.repeat(a, 2, axis=0), 2, axis=1)

    off24, on24 = up2(off12), up2(on12)            # 24×16(2×2 像素复制)

    # ① box 降采样均值正确性:手算 2×4 → 1×2;像素复制往返恒等。
    a = np.zeros((2, 4, 3))
    a[..., :] = np.arange(8, dtype=np.float64).reshape(2, 4, 1)
    d = box_down(a, 2, 1)
    assert d.shape == (1, 2, 3)
    assert abs(d[0, 0, 0] - 2.5) < 1e-15 and abs(d[0, 1, 0] - 4.5) < 1e-15
    rt_err = float(np.abs(box_down(off24, w12, h8) - off12).max())
    assert rt_err < 1e-12, f"像素复制 box 往返误差 {rt_err}"
    cases["box_down"] = {"hand_2x4_to_1x2": [2.5, 4.5], "roundtrip_max_err": rt_err}

    # ② .bin 读写往返(tempfile 裸 f32 LE)+ fail-closed 面。
    with tempfile.TemporaryDirectory() as td:
        pb_off = Path(td) / "off.bin"
        pb_on = Path(td) / "on.bin"
        pb_off.write_bytes(off24.astype("<f4").tobytes())
        pb_on.write_bytes(on24.astype("<f4").tobytes())
        off_rt, meta_off = load_image(str(pb_off), 24, 16, "raster_off")
        on_rt, meta_on = load_image(str(pb_on), 24, 16, "raster_on")
        assert meta_off == {"path": str(pb_off), "format": "bin", "w": 24, "h": 16}
        assert np.allclose(off_rt, off24, rtol=0.0, atol=1e-6)
        assert np.allclose(on_rt, on24, rtol=0.0, atol=1e-6)
        bad = Path(td) / "bad.bin"
        bad.write_bytes(b"\x00" * 10)
        expect_systemexit(lambda: load_image(str(bad), 24, 16, "x"), ".bin 长度校验")
        expect_systemexit(lambda: load_image(str(pb_off), None, None, "x"), ".bin 缺 w/h")
        expect_systemexit(lambda: load_image(str(Path(td) / "x.txt"), 1, 1, "x"), "后缀闭集")
        cases["bin_roundtrip"] = {"max_abs_err": float(np.abs(off_rt - off24).max())}

        # ③ 全链:12×8 pt + 24×16 off/on(往返数据)→ 公共网格 12×8。
        body = measure(pt, off_rt, on_rt, mask_rel_thr=0.02)

    assert body["common_grid"] == [12, 8]
    fs = body["fullscreen"]
    assert fs["ratio_on_off"] > 1.0 and fs["ratio_pt_off"] > fs["ratio_on_off"]
    mk = body["mask"]
    assert mk["mask_frac"] == 24.0 / 96.0, f"mask_frac 应精确 0.25,得到 {mk['mask_frac']}"
    assert mk["mask_px"] == 24
    assert mk["closer_to_pt"] == "on", f"closer_to_pt 应 on,得到 {mk['closer_to_pt']}"
    assert abs(mk["dist_off"] - 0.2) < 1e-6 and abs(mk["dist_on"] - 0.05) < 1e-6
    assert mk["margin"] > 0.2
    pc = body["percentiles"]
    assert 0.06 < pc["p50"] < 0.09 and 0.09 < pc["p95"] < 0.10
    for name, r in body["rois"].items():
        x, y, rw, rh = r["window"]
        assert 0 <= x and 0 <= y and x + rw <= 12 and y + rh <= 8 and rw > 0 and rh > 0, name
        assert all(math.isfinite(r[k]) for k in ("mean_pt", "mean_off", "mean_on")), name
    cases["pipeline_24x16_to_12x8"] = {
        "common_grid": body["common_grid"],
        "mask_frac": mk["mask_frac"],
        "closer_to_pt": mk["closer_to_pt"],
        "dist_off": mk["dist_off"],
        "dist_on": mk["dist_on"],
        "margin": mk["margin"],
        "p50": pc["p50"],
        "p95": pc["p95"],
    }

    # ④ 不整除 fail-closed + 空掩码 null 登记。
    expect_systemexit(
        lambda: measure(pt, np.zeros((16, 18, 3)), np.zeros((16, 24, 3)), 0.02),
        "公共网格不整除")
    same = measure(pt, pt, pt, 0.02)
    assert same["mask"]["mask_frac"] == 0.0 and same["mask"]["closer_to_pt"] is None
    assert same["percentiles"]["p50"] is None
    cases["fail_closed"] = ["bin_len", "bin_no_wh", "suffix", "grid_indivisible"]
    cases["empty_mask_null"] = True

    # ⑤ 完整报告可序列化且数值全有限。
    report = {"schema": SCHEMA, "arm": "f0",
              "inputs": {"pt": {"path": "<selftest>", "format": "array", "w": 12, "h": 8},
                         "raster_off": meta_off, "raster_on": meta_on},
              **body}
    assert_finite_or_null(report)
    json.dumps(report, ensure_ascii=False)

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
    ap.add_argument("--pt", help="PT 真值 EXR(scene-linear,rurix 端)")
    ap.add_argument("--raster-off", help="光栅 off 臂(.bin 或 .exr)")
    ap.add_argument("--raster-on", help="光栅 on 臂(.bin 或 .exr)")
    ap.add_argument("--raster-w", type=int, default=None, help=".bin 输入宽(无头契约必填)")
    ap.add_argument("--raster-h", type=int, default=None, help=".bin 输入高(无头契约必填)")
    ap.add_argument("--mask-rel-thr", type=float, default=0.02,
                    help="臂掩码相对亮度差阈值(默认 0.02)")
    ap.add_argument("--pt-gain", type=float, default=1.0,
                    help="PT 侧线性增益(曝光域对齐:光栅帧含 2^(−ev100) 静态"
                         "曝光而 PT 未曝光时传 2^(−ev100);默认 1.0)")
    ap.add_argument("--arm", choices=("f0", "refl", "other"), default="other",
                    help="臂名登记(默认 other)")
    ap.add_argument("--out", default=None, help="报告 JSON 另存路径(stdout 恒打印)")
    ap.add_argument("--selftest", action="store_true",
                    help="合成三图全链自测(含 .bin tempfile 往返)")
    args = ap.parse_args()

    if args.selftest:
        return run_selftest()
    if not (args.pt and args.raster_off and args.raster_on):
        raise SystemExit("FAIL: 须给 --pt/--raster-off/--raster-on(或 --selftest);见 -h")
    if args.mask_rel_thr < 0.0:
        raise SystemExit(f"FAIL: --mask-rel-thr 须 ≥ 0,得到 {args.mask_rel_thr}")
    if Path(args.pt).suffix.lower() != ".exr":
        raise SystemExit(f"FAIL: --pt 须为 .exr(PT 真值契约),得到 {args.pt}")

    if not (math.isfinite(args.pt_gain) and args.pt_gain > 0.0):
        raise SystemExit(f"FAIL: --pt-gain 须为有限正数,得到 {args.pt_gain}")

    pt, meta_pt = load_image(args.pt, None, None, "pt")
    off, meta_off = load_image(args.raster_off, args.raster_w, args.raster_h, "raster_off")
    on, meta_on = load_image(args.raster_on, args.raster_w, args.raster_h, "raster_on")
    pt = pt * args.pt_gain
    meta_pt["pt_gain"] = args.pt_gain
    body = measure(pt, off, on, args.mask_rel_thr)
    report = {"schema": SCHEMA, "arm": args.arm, "pt_gain": args.pt_gain,
              "inputs": {"pt": meta_pt, "raster_off": meta_off, "raster_on": meta_on},
              **body}
    txt = json.dumps(report, ensure_ascii=False, indent=1)
    print(txt)
    if args.out:
        Path(args.out).write_text(txt + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
