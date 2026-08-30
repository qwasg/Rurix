#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G37 W1：encode_parity_probe 转正 CI 门（day_0828 A2b 交接项 HANDOVER §B.5）
"""G37 W1 encode device-vs-host 逐像素 parity 硬门（g31.g37w1.encode_parity）。

背景：day_0828 A2b 定案的 ACES 1.3 样条转置 bug（kernels/g31_display_encode.rx
12 处 b1/b2 系数转置）曾长期漏网——夜巡只验确定性 digest（同错自洽恒绿），
不验 host parity。本门将 `artifacts/day_0828/a2b_aces_fix/encode_parity_probe.py`
转正为 encode 改动硬门，防同类 bug 复发。

口径（A2b 探针逐字）：同帧 GPU 切层对拍——
  输入 A = RURIX_G31_DUMP_F32=1 落的末帧 TSR 输出 f32（3 f32/px，encode kernel
    唯一像素输入；固定落点 .tmp/g31_gates/hzb/last_f32.bin）；
  输入 B = --dump-present-raw 落的同帧 presented 字节（w,h u32 LE 头 + 打包px）；
  host 预测 = 输入 A 逐像素经 display::aces13 f64 金标准逐字向量化（矩阵/样条
    单源 = artifacts/day_0828/recon/bluefan_encode_sim.py，A2b 交叉验证在案）
    → BT.1886 γ2.4 逆 EOTF v^(1/2.4) → floor(v·255+0.5) 量化；
  比对 = 预测 8bit vs presented 8bit 逐像素逐通道（BGRA/RGBA 序随 evidence
    window.channel_order）。
臂 = all-off 显式 `--quality off`（dither off / autoexp off / aeg=1.0——parity
  口径前提；显式旗标 = 默认翻转免疫）静态契约相机 --frames 8 --warmup 2。

硬门阈值（A2b 实测 2,073,600 px：exact 99.9891% / p100=1 LSB / >1LSB=0）：
  pixels_gt_1lsb == 0 ∧ diff_p100_lsb ≤ 1 ∧ exact_match_pct ≥ 99.9
  ∧ 0.18 灰设计点 host f64 == kernel f32 正形仿真 == [99,99,99]。

--selftest（纯 CPU，无 GPU/无构建）：
  ① 0.18 灰设计点红绿臂——正形 == host == 99³；**转置 bug 形态 kernel_sim(
    transposed=True) == 47³ ≠ 99³ 必被判红（防复发机核：A2b 根因形态字面复刻）**；
  ② host 向量化 ≡ 标量金标准逐探针互核（fan/wall/gray + 域覆盖点）；
  ③ parity 度量器红绿臂（合成帧：位级同 → exact 100%；注 1 LSB → 带内绿；
    注 2 LSB → gt1 检出必红）；④ raw 头解析 BGRA/RGBA 双序；⑤ 阈值裁决器
    红绿臂；⑥ schema required 互核。

三态：GPU/harness/SPV/bistro/单源件缺 → DEV_ENV_DEGRADE 退 0（不冒充 PASS）；
RURIX_REQUIRE_REAL=1 翻硬 FAIL。evidence = evidence/g31_encode_parity_<utc>.json
（PASS-only；FAIL 诊断件留 .tmp 工作区）。

用法:
  py -3 ci/g31_encode_parity_smoke.py --selftest
  py -3 ci/g31_encode_parity_smoke.py --gate g31.g37w1.encode_parity
"""
from __future__ import annotations

import argparse
import datetime
import json
import math
import os
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

TAG = "g31_encode_parity"
GATE_KEY = "g31.g37w1.encode_parity"
SCHEMA_ID = "rurix.g31.encode_parity_smoke_evidence.v1"
SCHEMA_PATH = ROOT / "milestones/g31/g31_encode_parity_evidence_schema.json"
WORK = ROOT / ".tmp" / "g31_gates" / "encode_parity"
# RURIX_G31_DUMP_F32=1 的固定落点（g31_window_present.rs 字面；fs::write 不建
# 父目录且错误被吞 ⇒ 本门须预建目录 + 清陈旧件 + 跑后验新鲜）。
F32_DUMP = ROOT / ".tmp" / "g31_gates" / "hzb" / "last_f32.bin"
# host 单源数学件（A2b recon 交叉验证在案：矩阵推导/样条参数/kernel f32 仿真
# 与 aces13.rs 单源同义——不在 ci/ 复制第二份常量表，防双源漂移）。
BLUEFAN_DIR = ROOT / "artifacts" / "day_0828" / "recon"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN_PRESENT = ROOT / "target" / "release" / f"g31_window_present{EXE_SUFFIX}"
# encode v2 字节（A2b 修复编译件,G31_DEFAULT_SPV_ENCODE 指向;G37 W1 起共享
# m_c 路径亦为同字节——本门只核在位性,不锁 sha（重编共享件属合法演进））。
SPV_ENCODE_V2 = ROOT / ".tmp" / "night_0828" / "spv" / "g31_display_encode_v2.spv"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
LANE_SPVS = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
)
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")

# 硬门阈值（A2b 实测锚：exact 99.9891% / p100=1 / >1LSB=0——阈值留裕不放水）。
EXACT_MATCH_PCT_MIN = 99.9
PIXELS_GT_1LSB_MAX = 0
DIFF_P100_LSB_MAX = 1

FAILURES: list[str] = []


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def load_bluefan():
    """加载 host 单源数学件（缺件返回 None，由调用侧三态处置）。"""
    if not (BLUEFAN_DIR / "bluefan_encode_sim.py").is_file():
        return None
    if str(BLUEFAN_DIR) not in sys.path:
        sys.path.insert(0, str(BLUEFAN_DIR))
    import bluefan_encode_sim  # noqa: PLC0415

    return bluefan_encode_sim


# ---------------------------------------------------------------------------
# 纯函数面（--selftest 同消费；host 数学 = A2b 探针逐字移植，f64 向量化）
# ---------------------------------------------------------------------------

D = np.float64


def _spline_fwd_vec(x: np.ndarray, c: dict, n_seg: float, half_min: float) -> np.ndarray:
    """segmented_spline_c5/c9_fwd 逐字向量化（f64；basis = vmul(cf,SPLINE_M) 正形；
    encode_parity_probe.py 逐字移植——样条参数经 bluefan 单源传入）。"""
    logx = np.log10(np.maximum(x, half_min))
    lminx = math.log10(c["min_point"][0])
    lmidx = math.log10(c["mid_point"][0])
    lmaxx = math.log10(c["max_point"][0])
    lminy = math.log10(c["min_point"][1])
    lmaxy = math.log10(c["max_point"][1])
    m_lo = logx <= lminx
    m_s1 = (~m_lo) & (logx < lmidx)
    m_s2 = (~m_lo) & (~m_s1) & (logx < lmaxx)

    lo = np.asarray(c["coefs_low"], dtype=D)
    hi = np.asarray(c["coefs_high"], dtype=D)
    kc1 = n_seg * (logx - lminx) / (lmidx - lminx)
    j1 = np.clip(np.floor(kc1), 0, n_seg - 1).astype(np.int64)
    t1 = kc1 - j1
    kc2 = n_seg * (logx - lmidx) / (lmaxx - lmidx)
    j2 = np.clip(np.floor(kc2), 0, n_seg - 1).astype(np.int64)
    t2 = kc2 - j2
    cf0 = np.where(m_s1, lo[j1], hi[j2])
    cf1 = np.where(m_s1, lo[np.minimum(j1 + 1, len(lo) - 1)], hi[np.minimum(j2 + 1, len(hi) - 1)])
    cf2 = np.where(m_s1, lo[np.minimum(j1 + 2, len(lo) - 1)], hi[np.minimum(j2 + 2, len(hi) - 1)])
    t = np.where(m_s1, t1, t2)
    b0 = 0.5 * cf0 - cf1 + 0.5 * cf2
    b1 = -cf0 + cf1
    b2 = 0.5 * cf0 + 0.5 * cf1
    sp = t * t * b0 + t * b1 + b2
    lin_lo = logx * c["slope_low"] + (lminy - c["slope_low"] * lminx)
    lin_hi = logx * c["slope_high"] + (lmaxy - c["slope_high"] * lmaxx)
    logy = np.where(m_lo, lin_lo, np.where(m_s1 | m_s2, sp, lin_hi))
    return np.power(10.0, logy)


def host_encode_vec(rgb: np.ndarray, bf) -> np.ndarray:
    """aces13.rs to_display_linear f64 逐字向量化 + BT.1886 γ + 量化（A2b 探针
    逐字移植）。rgb: (N,3) f64 scene-linear Rec.709。返回 (N,3) uint8 RGB。"""
    hm = bf.HostAces()
    m = lambda mat: np.asarray(mat, dtype=D)
    r709_ap0 = m(hm.rec709_to_ap0)
    ap0_ap1 = m(hm.ap0_to_ap1)
    ap1_ap0 = m(hm.ap1_to_ap0)
    rrt_sat = m(hm.rrt_sat)
    odt_sat = m(hm.odt_sat)
    ap1_xyz = m(hm.ap1_to_xyz)
    xyz_ap1 = m(hm.xyz_to_ap1)
    d60_d65 = m(hm.d60_to_d65)
    xyz_709 = m(hm.xyz_to_rec709)
    vm = lambda v, mat: v @ mat  # vmul 行向量约定 out[i]=Σ_j v[j]·m[j][i]

    aces = vm(rgb, r709_ap0)
    r, g, b = aces[:, 0], aces[:, 1], aces[:, 2]
    tiny = 1e-10
    maxc = np.max(aces, axis=1)
    minc = np.min(aces, axis=1)
    sat = (np.maximum(maxc, tiny) - np.maximum(minc, tiny)) / np.maximum(maxc, 1e-2)
    chroma = np.sqrt(np.maximum(b * (b - g) + g * (g - r) + r * (r - b), 0.0))
    yc = (b + g + r + 1.75 * chroma) / 3.0
    x = (sat - 0.4) / 0.2
    t = np.maximum(1.0 - np.abs(x / 2.0), 0.0)
    s = (1.0 + np.sign(x) * (1.0 - t * t)) / 2.0
    gg = 0.05 * s
    glow = np.where(
        yc <= 2.0 / 3.0 * 0.08,
        gg,
        np.where(yc >= 2.0 * 0.08, 0.0, gg * (0.08 / np.maximum(yc, 1e-300) - 0.5)),
    )
    aces = aces * (1.0 + glow)[:, None]
    r, g, b = aces[:, 0], aces[:, 1], aces[:, 2]
    neutral = (r == g) & (g == b)
    hue = np.degrees(np.arctan2(math.sqrt(3.0) * (g - b), 2.0 * r - g - b))
    hue = np.where(hue < 0.0, hue + 360.0, hue)
    centered = np.where(hue > 180.0, hue - 360.0, hue)
    w = 135.0
    in_win = (centered > -w / 2.0) & (centered < w / 2.0)
    kc = (centered + w / 2.0) * 4.0 / w
    jf = np.floor(kc)
    tf = kc - jf
    y0 = tf**3 * (1.0 / 6.0)
    y1 = tf**3 * (-3.0 / 6.0) + tf**2 * (3.0 / 6.0) + tf * (3.0 / 6.0) + 1.0 / 6.0
    y2 = tf**3 * (3.0 / 6.0) + tf**2 * (-1.0) + 4.0 / 6.0
    y3 = tf**3 * (-1.0 / 6.0) + tf**2 * (3.0 / 6.0) + tf * (-3.0 / 6.0) + 1.0 / 6.0
    ybs = np.where(jf < 0.5, y0, np.where(jf < 1.5, y1, np.where(jf < 2.5, y2, y3)))
    hw = np.where(in_win & ~neutral, ybs * 1.5, 0.0)
    aces[:, 0] = r + hw * sat * (0.03 - r) * (1.0 - 0.82)
    aces = np.maximum(aces, 0.0)
    pre = np.clip(vm(aces, ap0_ap1), 0.0, 65504.0)
    pre = vm(pre, rrt_sat)
    post = np.stack([_spline_fwd_vec(pre[:, i], bf.C5, 3.0, bf.HALF_MIN) for i in range(3)], axis=1)
    oces = vm(post, ap1_ap0)
    pre2 = vm(oces, ap0_ap1)
    post2 = np.stack([_spline_fwd_vec(pre2[:, i], bf.C9, 7.0, bf.HALF_MIN) for i in range(3)], axis=1)
    cb = 10.0 ** math.log10(0.02)
    lin = (post2 - cb) / (48.0 - cb)
    xyz = vm(lin, ap1_xyz)
    div = xyz.sum(axis=1)
    div = np.where(div == 0.0, 1e-10, div)
    xyx = xyz[:, 0] / div
    xyy = xyz[:, 1] / div
    yv = np.power(np.maximum(xyz[:, 1], 0.0), 0.9811)
    yden = np.maximum(xyy, 1e-10)
    xyz2 = np.stack([xyx * yv / yden, yv, (1.0 - xyx - xyy) * yv / yden], axis=1)
    lin = vm(vm(xyz2, xyz_ap1), odt_sat)
    disp = vm(vm(vm(lin, ap1_xyz), d60_d65), xyz_709)
    disp = np.clip(disp, 0.0, 1.0)
    return np.clip(np.floor(np.power(disp, 1.0 / 2.4) * 255.0 + 0.5), 0.0, 255.0).astype(np.uint8)


def q8_host(disp) -> tuple[int, int, int]:
    """host 标量 8-bit 量化镜像（BT.1886 v^(1/2.4) + floor(v·255+0.5)）。"""
    return tuple(int(min(max(math.floor(c ** (1.0 / 2.4) * 255.0 + 0.5), 0), 255)) for c in disp)


def parse_present_raw(buf: bytes, bgra: bool) -> tuple[int, int, np.ndarray]:
    """--dump-present-raw 布局解析（w,h u32 LE 头 + 4B/px 打包）→ (w,h,(N,3) RGB u8)。"""
    if len(buf) < 8:
        raise ValueError(f"raw 头缺失（{len(buf)}B < 8B）")
    w, h = (int(v) for v in np.frombuffer(buf[:8], dtype="<u4"))
    px = np.frombuffer(buf[8:], dtype=np.uint8)
    if px.size != w * h * 4:
        raise ValueError(f"raw 像素数 {px.size} ≠ {w}x{h}x4")
    px = px.reshape(-1, 4)
    return w, h, px[:, [2, 1, 0]] if bgra else px[:, :3]


def parity_metrics(pred: np.ndarray, meas: np.ndarray, w: int) -> dict:
    """逐像素逐通道 LSB 差分位统计（A2b 探针同口径）。"""
    dch = np.abs(pred.astype(np.int16) - meas.astype(np.int16))
    d = dch.max(axis=1)
    n = int(d.shape[0])
    exact = int(np.sum(d == 0))
    wi = int(np.argmax(d))
    wy, wx = divmod(wi, w)
    return {
        "pixels": n,
        "exact_match": exact,
        "exact_match_pct": round(100.0 * exact / n, 4),
        "diff_p50_lsb": float(np.percentile(d, 50)),
        "diff_p99_lsb": float(np.percentile(d, 99)),
        "diff_p100_lsb": int(d.max()),
        "pixels_gt_1lsb": int(np.sum(d > 1)),
        "pixels_gt_2lsb": int(np.sum(d > 2)),
        "worst_pixel": {
            "x": wx,
            "y": wy,
            "measured_rgb": [int(v) for v in meas[wi]],
            "predicted_rgb": [int(v) for v in pred[wi]],
        },
    }


def parity_verdict(m: dict) -> list[str]:
    """硬门阈值裁决（空列表 = 绿）。"""
    fails: list[str] = []
    if m.get("pixels_gt_1lsb", 1) > PIXELS_GT_1LSB_MAX:
        fails.append(f">1LSB 像素 {m.get('pixels_gt_1lsb')} > {PIXELS_GT_1LSB_MAX}（ACES 类系统性偏差检出）")
    if m.get("diff_p100_lsb", 255) > DIFF_P100_LSB_MAX:
        fails.append(f"p100 {m.get('diff_p100_lsb')} LSB > {DIFF_P100_LSB_MAX}")
    if m.get("exact_match_pct", 0.0) < EXACT_MATCH_PCT_MIN:
        fails.append(f"exact {m.get('exact_match_pct')}% < {EXACT_MATCH_PCT_MIN}%")
    if m.get("pixels", 0) < 1:
        fails.append("像素数 0（空帧冒充）")
    return fails


def gray018_design_point(bf) -> dict:
    """0.18 灰设计点三口径（host f64 / kernel f32 正形 / 转置 bug 形态复刻）。

    ok = host == fixed == 99³；transposed_detected = 转置形态 ≠ 正形（防复发
    机核——A2b 根因形态在设计点必被检出，47³ vs 99³）。"""
    p = bf.pack_params()
    host8 = list(q8_host(bf.HostAces().run((0.18, 0.18, 0.18))))
    fix8, _ = bf.kernel_sim((0.18, 0.18, 0.18), p, transposed=False)
    bug8, _ = bf.kernel_sim((0.18, 0.18, 0.18), p, transposed=True)
    return {
        "host_f64_rgb8": host8,
        "kernel_f32_sim_rgb8": list(fix8),
        "transposed_bug_sim_rgb8": list(bug8),
        "ok": host8 == [99, 99, 99] and list(fix8) == [99, 99, 99],
        "transposed_detected": list(bug8) != list(fix8),
    }


# ---------------------------------------------------------------------------
# selftest（纯 CPU：无 GPU/无构建/不读 dump）
# ---------------------------------------------------------------------------

REQUIRED_KEYS = [
    "schema", "subject", "symbolic_gate_key", "wave", "caliber", "harness",
    "parity", "gray_018_design_point", "thresholds", "environment", "timestamp", "notes",
]


def run_selftest() -> int:
    fails: list[str] = []

    def expect(cond: bool, name: str) -> None:
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            fails.append(name)

    # ① 单源数学件 + 0.18 灰设计点红绿臂（防复发机核）。
    bf = load_bluefan()
    expect(bf is not None, "host 单源数学件在树（artifacts/day_0828/recon/bluefan_encode_sim.py）")
    if bf is None:
        print(f"[{TAG}] selftest FAIL（单源件缺失,后续臂不可执行）", file=sys.stderr)
        return 1
    g18 = gray018_design_point(bf)
    expect(g18["ok"], "GREEN:0.18 灰设计点 host f64 == kernel f32 正形 == 99³")
    expect(g18["transposed_detected"], "RED:ACES 样条转置 bug 形态（A2b 根因复刻）在设计点必被检出")
    expect(g18["transposed_bug_sim_rgb8"] != [99, 99, 99], "RED:转置形态 ≠ 设计点值（47³ 档漂移必红）")

    # ② host 向量化 ≡ 标量金标准（fan/wall/gray + 域覆盖点逐探针互核）。
    probes = [
        (0.35686296, 0.25368458, 0.10629952),  # fan(1500,12) TSR-out（A2b 地标）
        (0.00186373, 0.00055717, 0.00041163),  # wall(1700,430) 暗部
        (0.18, 0.18, 0.18),                    # 灰设计点（中性 red-mod 分支）
        (0.0, 0.0, 0.0),                       # 黑（样条 lo 段）
        (4.2, 0.31, 0.02),                     # 高饱和亮部（glow/red-mod 窗内）
        (0.9, 0.9, 0.05),                      # 黄色高光
    ]
    vec = host_encode_vec(np.asarray(probes, dtype=D), bf)
    scal = [q8_host(bf.HostAces().run(p)) for p in probes]
    expect(
        all(tuple(int(v) for v in vec[i]) == scal[i] for i in range(len(probes))),
        "GREEN:host 向量化 == 标量金标准（6 探针逐通道）",
    )
    expect(list(vec[0]) == [144, 122, 77], "GREEN:fan 地标预测 == A2b 在案 [144,122,77]")

    # ③ parity 度量器红绿臂（合成帧 4×2）。
    w0 = 4
    base = np.arange(24, dtype=np.uint8).reshape(8, 3)
    m_eq = parity_metrics(base, base.copy(), w0)
    expect(
        m_eq["exact_match_pct"] == 100.0 and m_eq["pixels_gt_1lsb"] == 0 and m_eq["diff_p100_lsb"] == 0,
        "GREEN:位级同帧 → exact 100%/p100=0",
    )
    expect(parity_verdict(m_eq) == [], "GREEN:位级同帧过阈值裁决")
    # 1 LSB 带内绿臂用 10,000 px 帧（单像素差 → exact 99.99% ≥ 99.9% 阈内）。
    base_big = (np.arange(30000, dtype=np.int64) % 251).astype(np.uint8).reshape(10000, 3)
    off1 = base_big.copy()
    off1[3, 1] += 1  # 单像素 +1 LSB（带内）
    m1 = parity_metrics(base_big, off1, 100)
    expect(
        m1["pixels_gt_1lsb"] == 0 and m1["diff_p100_lsb"] == 1
        and m1["exact_match_pct"] == 99.99 and parity_verdict(m1) == [],
        "GREEN:1 LSB 带内绿（结构容差;单像素差 @10k px）",
    )
    off2 = base.copy()
    off2[5, 2] += 2  # 单像素 +2 LSB（越带）
    m2 = parity_metrics(base, off2, w0)
    expect(m2["pixels_gt_1lsb"] == 1 and parity_verdict(m2) != [], "RED:2 LSB 越带必红（gt1 检出）")
    expect(m2["worst_pixel"]["x"] == 1 and m2["worst_pixel"]["y"] == 1, "GREEN:worst 座标定位（行主序）")
    low = dict(m_eq, exact_match_pct=99.0)
    expect(parity_verdict(low) != [], "RED:exact 99.0% < 99.9% 必红")
    expect(parity_verdict(dict(m_eq, pixels=0)) != [], "RED:零像素冒充必红")

    # ④ raw 头解析双序。
    px = bytes([10, 20, 30, 255, 40, 50, 60, 255])  # 2 px
    raw = (2).to_bytes(4, "little") + (1).to_bytes(4, "little") + px
    w_, h_, rgb_b = parse_present_raw(raw, bgra=True)
    expect((w_, h_) == (2, 1) and list(rgb_b[0]) == [30, 20, 10], "GREEN:BGRA 序解析（B/R 换位）")
    _, _, rgb_r = parse_present_raw(raw, bgra=False)
    expect(list(rgb_r[0]) == [10, 20, 30], "GREEN:RGBA 序解析（直通）")
    try:
        parse_present_raw(raw[:11], bgra=True)
        expect(False, "RED:截断 raw 必抛")
    except ValueError:
        expect(True, "RED:截断 raw 必抛")

    # ⑤ schema 互核。
    expect(SCHEMA_PATH.is_file(), f"schema 在树 {SCHEMA_PATH.name}")
    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(set(schema.get("required", [])) == set(REQUIRED_KEYS), "schema required == REQUIRED_KEYS 互核")
        expect(schema.get("properties", {}).get("schema", {}).get("const") == SCHEMA_ID, "schema const 互核")
        th = schema.get("properties", {}).get("thresholds", {}).get("properties", {})
        expect(
            th.get("exact_match_pct_min", {}).get("const") == EXACT_MATCH_PCT_MIN
            and th.get("pixels_gt_1lsb_max", {}).get("const") == PIXELS_GT_1LSB_MAX
            and th.get("diff_p100_lsb_max", {}).get("const") == DIFF_P100_LSB_MAX,
            "schema 阈值 const 与脚本常量互核",
        )
    if fails:
        print(f"[{TAG}] selftest FAIL ({len(fails)})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（设计点红绿臂含转置防复发 + 向量化互核 + 度量器红绿臂 + raw 双序 + schema 互核）")
    return 0


# ---------------------------------------------------------------------------
# gate（GPU 腿：all-off 显式 --quality off 同帧切层 + host parity）
# ---------------------------------------------------------------------------

def run_gate() -> int:
    check(SCHEMA_PATH.is_file(), f"schema 缺失: {SCHEMA_PATH}")
    bf = load_bluefan()
    check(bf is not None, "host 单源数学件缺失（artifacts/day_0828/recon/bluefan_encode_sim.py）")
    if FAILURES:
        return 1
    # 纯 CPU 前置：设计点自洽（host 数学损坏则先红，不上 GPU）。
    g18 = gray018_design_point(bf)
    check(g18["ok"], f"0.18 灰设计点前置红: {g18}")
    check(g18["transposed_detected"], "转置防复发臂失效（bug 形态未被设计点区分）")
    if FAILURES:
        return 1

    # 三态前置（dev-env 缺面 = DEGRADE 不冒充）。
    degrade: list[str] = []
    if not BIN_PRESENT.is_file():
        degrade.append(f"harness 缺失 {BIN_PRESENT.relative_to(ROOT)}")
    if not SPV_ENCODE_V2.is_file():
        degrade.append(f"encode v2 SPV 缺失 {SPV_ENCODE_V2.relative_to(ROOT)}（.tmp 构建产物）")
    missing = [f for f in LANE_SPVS if not (SPV_DIR / f).is_file()]
    if missing:
        degrade.append(f"车道 SPV 缺失 {missing}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    harness_doc: dict | None = None
    metrics: dict = {}
    validation_silent = False
    raw_path = WORK / "present_last.raw"
    if not degrade:
        WORK.mkdir(parents=True, exist_ok=True)
        F32_DUMP.parent.mkdir(parents=True, exist_ok=True)
        for stale in (F32_DUMP, raw_path):
            if stale.is_file():
                stale.unlink()
        ev_path = WORK / "harness_alloff.json"
        env = dict(os.environ)
        env["RURIX_REQUIRE_REAL"] = "1"
        env["RURIX_VK_VALIDATION"] = "1"
        env["RURIX_G31_DUMP_F32"] = "1"
        argv = [
            str(BIN_PRESENT),
            "--frames", "8", "--warmup", "2", "--hidden",
            "--quality", "off",  # 显式 all-off：parity 口径（aeg=1.0/dither off）+ 默认翻转免疫
            "--dump-present-raw", str(raw_path),
            "--evidence", str(ev_path),
        ]
        note(f"$ {' '.join(argv)}")
        with gpu_device_lock(purpose=f"{TAG} all-off 同帧切层（encode parity 硬门）"):
            r = subprocess.run(argv, capture_output=True, text=True, timeout=1800, env=env, cwd=str(ROOT))
        out = (r.stdout or "") + (r.stderr or "")
        if '"state":"skipped_dev_env"' in out:
            degrade.append(f"harness skipped_dev_env: {out.strip()[-200:]}")
        else:
            check(r.returncode == 0, f"harness 非零退出 {r.returncode}: {out.strip()[-300:]}")
            validation_silent = "Validation Error" not in out and "VUID-" not in out
            check(validation_silent, "validation 应静默却报错")
            check(ev_path.is_file(), "harness evidence 缺失")
            check(raw_path.is_file(), "--dump-present-raw 未落盘")
            check(F32_DUMP.is_file(), f"RURIX_G31_DUMP_F32 未落盘 {F32_DUMP.relative_to(ROOT)}（目录预建后仍缺 = harness 面异常）")
            if ev_path.is_file():
                harness_doc = json.loads(ev_path.read_text(encoding="utf-8"))
                # G37 批次 0 首跑修正:evidence 字面字段 = "frames"(主帧数,
                # warmup 独立字段)——初版误设 "frames_completed"==10 必红。
                check(harness_doc.get("frames") == 8, f"frames {harness_doc.get('frames')} ≠ 8")

    if degrade:
        doc = {"schema": "rurix.g31.encode_parity.skip.v1", "state": "DEV_ENV_DEGRADE", "reasons": degrade}
        print(json.dumps(doc, ensure_ascii=False))
        if require_real():
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL）")
        return 0

    # ── host parity 复算（纯 CPU）──
    if not FAILURES:
        window = (harness_doc or {}).get("window") or {}
        bgra = window.get("channel_order", "bgra8_unorm") == "bgra8_unorm"
        w, h, meas = parse_present_raw(raw_path.read_bytes(), bgra=bgra)
        tsr = np.fromfile(F32_DUMP, dtype="<f4").astype(D)
        check(
            tsr.size == w * h * 3,
            f"TSR f32 数 {tsr.size} ≠ {w}x{h}x3（encode in_color 3 f32/px 布局破）",
        )
        if not FAILURES:
            pred = host_encode_vec(tsr.reshape(-1, 3), bf)
            metrics = parity_metrics(pred, meas, w)
            for msg in parity_verdict(metrics):
                check(False, f"parity 硬门: {msg}")
            note(
                f"parity: pixels={metrics['pixels']} exact={metrics['exact_match_pct']}% "
                f"p100={metrics['diff_p100_lsb']} gt1={metrics['pixels_gt_1lsb']}"
            )

    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    verdict_doc = {
        "schema": SCHEMA_ID,
        "subject": TAG,
        "symbolic_gate_key": GATE_KEY,
        "wave": "G37.W1",
        "caliber": (
            "同帧 GPU 切层：RURIX_G31_DUMP_F32=1 末帧 TSR f32 + --dump-present-raw 同帧 presented 字节;"
            "host = display::aces13 f64 金标准逐字向量化（单源 artifacts/day_0828/recon/bluefan_encode_sim.py）"
            "+ BT.1886 v^(1/2.4) + floor(v·255+0.5);臂 = 显式 --quality off 静态契约相机 8+2（aeg=1.0/dither off）"
        ),
        "harness": {
            "bin": str(BIN_PRESENT.relative_to(ROOT)).replace("\\", "/"),
            "frames_completed": (harness_doc or {}).get("frames", -1),
            "digest": (harness_doc or {}).get("digest", ""),
            "validation_silent": validation_silent,
            "encode_spv": ((harness_doc or {}).get("contracts") or {}).get("encode_spv") or {},
        },
        "parity": metrics or {
            "pixels": 0, "exact_match": 0, "exact_match_pct": 0.0,
            "diff_p50_lsb": -1.0, "diff_p99_lsb": -1.0, "diff_p100_lsb": 255,
            "pixels_gt_1lsb": -1, "pixels_gt_2lsb": -1,
            "worst_pixel": {"x": -1, "y": -1, "measured_rgb": [], "predicted_rgb": []},
        },
        "gray_018_design_point": g18,
        "thresholds": {
            "exact_match_pct_min": EXACT_MATCH_PCT_MIN,
            "pixels_gt_1lsb_max": PIXELS_GT_1LSB_MAX,
            "diff_p100_lsb_max": DIFF_P100_LSB_MAX,
        },
        "environment": {
            "os": f"{os.name}-{sys.platform}",
            "python_version": sys.version.split()[0],
            "host": "RTX 4070 Ti + Vulkan",
        },
        "timestamp": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "notes": (
            "G37 W1:encode_parity_probe.py（day_0828 A2b）转正 encode 改动硬门——夜巡确定性 digest"
            "不验 host parity,ACES 样条转置 bug 即此漏网通道;本门 device-vs-host 逐像素逐通道对拍"
            "（>1LSB=0 硬门）+ 0.18 灰设计点 + 转置形态防复发臂（selftest 纯 CPU 消费）"
        ),
    }
    if FAILURES:
        WORK.mkdir(parents=True, exist_ok=True)
        diag = WORK / f"g31_encode_parity_FAIL_{ts}.json"
        diag.write_text(
            json.dumps({"failures": FAILURES, "verdict_doc": verdict_doc}, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        print(f"[{TAG}] FAIL gate={GATE_KEY}（{len(FAILURES)} 违例;诊断件 {diag}）", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    ev_out = ROOT / "evidence" / f"g31_encode_parity_{ts}.json"
    ev_out.write_text(json.dumps(verdict_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[{TAG}] PASS gate={GATE_KEY}（pixels={metrics.get('pixels')} exact={metrics.get('exact_match_pct')}% "
        f"p100={metrics.get('diff_p100_lsb')} gt1={metrics.get('pixels_gt_1lsb')};evidence {ev_out}）"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        return run_gate()
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
