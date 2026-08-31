#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G38 T5:RIS/NEE A/B 判读器(消费 run_ab.py 产物,产 ris_nee_ab.json)。

口径链(day_0829 复用,只调用不修改):
  - 度量 = artifacts/day_0829_realism/tools/ab_metrics.py noise 子命令
    (load_raw 跳 8B 头 + BGRA→RGB /255;逐像素跨帧 std → mean/p95)。
  - 帧源 = 每臂 r1 腿 dump 尾段 f0064 起(every=4 ⇒ f0064..f0092 恰 8 张;
    warmup2 后 TSR 帧旋转已收敛,尾段时域噪声即 RIS/NEE 方差口径)。
  - 四 ROI 字面(1920×1080,day_0830 交接单钉死):
      wall=(1400,150,480,270) floor=(1100,800,480,270)
      dark_arch=(360,0,360,180) dark_table=(560,560,560,200)
    dark_* 为 GI2 微光承载区 = RIS/NEE 主战场,verdict 只看 dark 两 ROI。
  - raw 为 display-encode 后 u8 口径:同路 on/off 对照有效;与 bench EXR
    f32 不可跨比(如实登记进 json 注记)。

verdict 规则(阈值口径,写入 json 注记):
  shrink_pct = (base_p95 − arm_p95) / base_p95 × 100(temporal_std_p95)
  取两 dark ROI 收缩的 min(保守口径):min ≥ +10% = effective;
  0 ≤ min < 10% = marginal;min < 0 = worse。

用法:
  py -3 judge_ab.py             # 判读 ab/ 下四臂产物 → ris_nee_ab.json
  py -3 judge_ab.py --selftest  # 伪数据全链(小图 + 代位 ROI,零 GPU)
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
AB_METRICS_DIR = ROOT / "artifacts" / "day_0829_realism" / "tools"
AB_DIR = HERE / "ab"
OUT_JSON = HERE / "ris_nee_ab.json"

ARM_ORDER = ["base", "ris", "nee", "both"]

# 四 ROI 字面(x,y,w,h @1920×1080;交接单钉死,dark_* = 微光承载区)。
ROIS: dict[str, str] = {
    "wall": "1400,150,480,270",
    "floor": "1100,800,480,270",
    "dark_arch": "360,0,360,180",
    "dark_table": "560,560,560,200",
}
DARK_ROIS = ["dark_arch", "dark_table"]
TAIL_FROM = 64          # 尾段起始帧号(f0064 起)
TAIL_MIN = 8            # 尾段最少帧数(不足 fail-closed)
SHRINK_EFFECTIVE_PCT = 10.0   # dark ROI p95 收缩 ≥10% = 有效

CALIBER_NOTE = (
    "raw = presented display-encode 后 u8 口径(8B 头 w/h u32 LE + BGRA8);"
    "同路 on/off 对照有效,与 bench EXR f32 不可跨比。verdict 规则:"
    "shrink_pct=(base_p95−arm_p95)/base_p95×100,取 dark_arch/dark_table 两 ROI "
    "min(保守):≥10%=effective,0..10%=marginal,<0=worse。帧源 = r1 腿 "
    "f0064..f0092 尾段 8 张(every=4,TSR 收敛后时域噪声)。"
)


def _import_ab_metrics():
    """挂载既有度量工具(只调用不修改;经其 build_parser 走 CLI 同语义)。"""
    sys.path.insert(0, str(AB_METRICS_DIR))
    import ab_metrics  # noqa: E402
    return ab_metrics


def pick_tail_frames(run_dir: Path, tail_from: int = TAIL_FROM,
                     tail_min: int = TAIL_MIN) -> list[str]:
    """列出 p.raw.f<帧号:04> 中帧号 ≥ tail_from 者,升序;不足 fail-closed。"""
    pat = re.compile(r"^p\.raw\.f(\d{4})$")
    picked: list[tuple[int, Path]] = []
    for f in run_dir.iterdir():
        m = pat.match(f.name)
        if m and int(m.group(1)) >= tail_from:
            picked.append((int(m.group(1)), f))
    picked.sort()
    if len(picked) < tail_min:
        raise SystemExit(
            f"FAIL: {run_dir} 尾段帧不足(f{tail_from:04} 起需 ≥{tail_min},"
            f"得 {len(picked)};run_ab.py 是否 every=4 全程跑完?)")
    return [str(p) for _, p in picked]


def noise_of(ab_metrics, paths: list[str], rois: dict[str, str]) -> dict:
    """调 ab_metrics noise(多 --rect/--label)→ {roi: 指标块}。"""
    argv = ["noise", *paths]
    for name, rect in rois.items():
        argv += ["--rect", rect, "--label", name]
    a = ab_metrics.build_parser().parse_args(argv)
    r = a.func(a)
    return {name: r["crops"][name] for name in rois}


def load_arm_evidence(arm_dir: Path) -> dict:
    """读双跑 evidence:digest 一致布尔 + r1/r2 帧时(缺文件 fail-closed)。"""
    out: dict = {}
    for leg in ("r1", "r2"):
        p = arm_dir / leg / "ev.json"
        if not p.is_file():
            raise SystemExit(f"FAIL: 缺 evidence {p}(run_ab.py 未跑完?)")
        ev = json.loads(p.read_text(encoding="utf-8"))
        out[leg] = {
            "digest": ev.get("digest"),
            "real_render_frame_ms": ev.get("real_render_frame_ms"),
            "render_max_ms": (ev.get("stats") or {}).get("render_max_ms"),
        }
    out["double_run_bitexact"] = (
        out["r1"]["digest"] is not None
        and out["r1"]["digest"] == out["r2"]["digest"])
    return out


def shrink_pct(base_v: float, arm_v: float) -> float:
    """收缩百分比(正 = 噪声降;base=0 时退化 0.0 并由调用方注记)。"""
    if base_v <= 0.0:
        return 0.0
    return (base_v - arm_v) / base_v * 100.0


def verdict_of(shrinks: dict[str, float], dark_rois: list[str]) -> tuple[str, float]:
    """dark ROI p95 收缩 min 判档:≥10 effective / [0,10) marginal / <0 worse。"""
    m = min(shrinks[r] for r in dark_rois)
    if m >= SHRINK_EFFECTIVE_PCT:
        return "effective", m
    if m >= 0.0:
        return "marginal", m
    return "worse", m


def judge(ab_dir: Path, rois: dict[str, str], dark_rois: list[str],
          with_evidence: bool = True) -> dict:
    """全链判读(参数化 rects/目录以复用于 selftest)。"""
    ab_metrics = _import_ab_metrics()
    arms: dict = {}
    for arm in ARM_ORDER:
        run_dir = ab_dir / arm / "r1"
        frames = pick_tail_frames(run_dir)
        crops = noise_of(ab_metrics, frames, rois)
        arms[arm] = {
            "frames_used": [Path(f).name for f in frames],
            "noise": {
                name: {
                    "temporal_std_mean": c["temporal_std_mean"],
                    "temporal_std_p95": c["temporal_std_p95"],
                    "temporal_rel_mean": c["temporal_rel_mean"],
                    "temporal_rel_p95": c["temporal_rel_p95"],
                    "mean_luma": c["mean_luma"],
                } for name, c in crops.items()
            },
        }
        if with_evidence:
            arms[arm]["evidence"] = load_arm_evidence(ab_dir / arm)

    base_noise = arms["base"]["noise"]
    verdicts: dict = {}
    for arm in ARM_ORDER[1:]:
        shr_p95 = {name: shrink_pct(base_noise[name]["temporal_std_p95"],
                                    arms[arm]["noise"][name]["temporal_std_p95"])
                   for name in rois}
        shr_mean = {name: shrink_pct(base_noise[name]["temporal_std_mean"],
                                     arms[arm]["noise"][name]["temporal_std_mean"])
                    for name in rois}
        arms[arm]["shrink_vs_base_pct"] = {
            "temporal_std_p95": {k: round(v, 2) for k, v in shr_p95.items()},
            "temporal_std_mean": {k: round(v, 2) for k, v in shr_mean.items()},
        }
        v, m = verdict_of(shr_p95, dark_rois)
        verdicts[arm] = {"verdict": v, "dark_min_shrink_p95_pct": round(m, 2)}
        if with_evidence:
            b = arms["base"]["evidence"]["r1"]["real_render_frame_ms"]
            a = arms[arm]["evidence"]["r1"]["real_render_frame_ms"]
            if b is not None and a is not None:
                verdicts[arm]["frame_ms_delta_vs_base"] = round(a - b, 3)

    return {
        "schema": "rurix.day0830.g38.t5.ris_nee_ab.v1",
        "caliber_note": CALIBER_NOTE,
        "rois": rois,
        "dark_rois": dark_rois,
        "tail_frames": {"from": TAIL_FROM, "min": TAIL_MIN},
        "shrink_effective_pct": SHRINK_EFFECTIVE_PCT,
        "arms": arms,
        "variance_verdict": verdicts,
    }


def do_selftest() -> int:
    """伪数据全链:tempdir 造 base(高噪)/ris(低噪)/nee(等噪)/both(更高噪)
    四臂 r1 dump 序列 + 伪 evidence,走 judge 全链,断言三档 verdict 方向。"""
    import struct
    import tempfile

    import numpy as np

    def write_fake_raw(p: Path, rgb01: np.ndarray) -> None:
        h, w = rgb01.shape[:2]
        u8 = (np.clip(rgb01, 0, 1) * 255.0 + 0.5).astype(np.uint8)
        bgra = np.zeros((h, w, 4), dtype=np.uint8)
        bgra[:, :, 0] = u8[:, :, 2]
        bgra[:, :, 1] = u8[:, :, 1]
        bgra[:, :, 2] = u8[:, :, 0]
        bgra[:, :, 3] = 255
        p.write_bytes(struct.pack("<II", w, h) + bgra.tobytes())

    n = 64
    # 小图代位 ROI(生产 1920×1080 四 ROI 字面越界于 64×64,selftest 换代位;
    # 链路本身〔选帧/noise/收缩/verdict〕与生产同代码路径)。
    rois = {"wall": "0,0,24,24", "floor": "32,32,24,24",
            "dark_arch": "0,32,24,24", "dark_table": "32,0,24,24"}
    dark = ["dark_arch", "dark_table"]
    # 噪声幅度设计:base=0.04;ris=0.02(收缩 ~50% ⇒ effective);
    # nee=0.038(收缩 ~5% ⇒ marginal);both=0.06(负收缩 ⇒ worse)。
    sigma = {"base": 0.04, "ris": 0.02, "nee": 0.038, "both": 0.06}
    with tempfile.TemporaryDirectory() as td:
        ab = Path(td)
        base_img = np.tile(np.array([0.3, 0.4, 0.5]), (n, n, 1))
        for arm in ARM_ORDER:
            rd = ab / arm / "r1"
            rd.mkdir(parents=True)
            rng = np.random.default_rng(830)  # 同种子:臂间只差幅度
            for k in range(8):
                img = np.clip(base_img + rng.normal(0, sigma[arm], (n, n, 3)), 0, 1)
                write_fake_raw(rd / f"p.raw.f{64 + 4 * k:04}", img)
        res = judge(ab, rois, dark, with_evidence=False)
        v = res["variance_verdict"]
        assert v["ris"]["verdict"] == "effective", f"ris 应 effective,得 {v['ris']}"
        assert v["nee"]["verdict"] == "marginal", f"nee 应 marginal,得 {v['nee']}"
        assert v["both"]["verdict"] == "worse", f"both 应 worse,得 {v['both']}"
        assert all(len(res["arms"][a]["frames_used"]) == 8 for a in ARM_ORDER)
        print(json.dumps({
            "selftest": "judge_ab", "pass": True,
            "verdicts": {a: v[a]["verdict"] for a in ARM_ORDER[1:]},
            "dark_min_shrink": {a: v[a]["dark_min_shrink_p95_pct"]
                                for a in ARM_ORDER[1:]},
        }, ensure_ascii=False))
    return 0


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--selftest", action="store_true", help="伪数据全链(零 GPU)")
    args = ap.parse_args()
    if args.selftest:
        return do_selftest()
    res = judge(AB_DIR, ROIS, DARK_ROIS, with_evidence=True)
    txt = json.dumps(res, ensure_ascii=False, indent=1)
    OUT_JSON.write_text(txt + "\n", encoding="utf-8")
    print(txt)
    print(f"JUDGE_AB → {OUT_JSON}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
