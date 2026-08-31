#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G38 T5:RIS/NEE 方差收缩 A/B 四臂跑批(主 agent 批次 2 GPU 锁内执行)。

四臂 = base(显式无 AE 十九臂集减 ris/nee)/ +ris / +nee / +both;
逐臂双跑(r1/r2,digest 位级自证),r1 带 presented raw 周期 dump
(--dump-present-every 4 ⇒ f0000..f0092 批 + 末帧本体),judge_ab.py 取
尾段 f0064 起 8 张喂 ab_metrics noise(TSR 收敛后时域噪声 = 方差口径)。

显式无 AE 集推导(day_0829 红修 #1 纪律,十九臂时代字面):
  = g31_window_present.rs QUALITY_FULL_EXPANSION 赋值区字面(L7850-7874)
    − --auto-exposure(AE 反馈污染 A/B,恒减)
    − --gi2-ris/--gi2-nee(被测臂,归臂旗标)
    + --quality off 打头(G37 W4 默认翻转后缺省=full,显式组合必须显式回退,
      否则与预设 dup fail-closed)
  环境面:RURIX_G18_AMBIENT=0.004 显式注入(预设 OnceLock 字面的 env 等价,
  同字面同 parse 位级同值;run_arm.py env_of 先例)。

用法:
  py -3 run_ab.py              # 真跑(GPU 锁外部由主 agent 持有,本脚本不管锁)
  py -3 run_ab.py --dry-run    # 只打印命令与 env 增量,不执行
  py -3 run_ab.py --selftest   # 零 GPU:伪 raw(8B 头+随机 BGRA)走通 noise 链
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent          # artifacts/day_0830_g38/t5_risnee
ROOT = HERE.parents[2]                          # 仓库根 h:\rurix
WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
# 既有度量工具(只调用不修改;noise 子命令 = 逐像素跨帧 std)。
AB_METRICS_DIR = ROOT / "artifacts" / "day_0829_realism" / "tools"

# ── 显式无 AE 十九臂 base 集(推导见模块 docstring;字面顺序循赋值区)──
EXPLICIT_NOAE_BASE = [
    "--quality", "off",              # W4 默认翻转回退档(显式组合必须)
    "--smooth-normals", "on",
    "--ggx", "on",
    "--lamp-lights", "on",
    "--lamp-gain", "4",              # 赋值区 Some(4.0);"4" parse f32 位级同值(run_arm 先例字面)
    "--textures", "on",
    "--bloom", "on",
    "--dither", "on",
    # --auto-exposure on ← 恒减(无 AE 纪律)
    "--tsr-quality", "on",
    "--gi2", "on",
    "--gi2-clamp", "0.01",           # 赋值区 Some(0.01)
    "--emissive-tex", "on",
    "--metal-f0", "on",
    "--rt-ao", "on",
    "--soft-shadows", "on",
    "--soft-shadow-samples", "1",    # 赋值区 Some(1)(F1 组合定档字面)
    "--rt-reflect", "on",
    "--gi2-tex", "on",
    "--normal-maps", "on",
    "--transparency", "on",          # G37 W2 并入 full(lut/visbuffer 不在赋值区 = 不在集)
    # --gi2-ris/--gi2-nee ← 被测臂,归下方 ARMS
]

# 四臂增量旗标(base 之上叠加;ris/nee 须随 --gi2 on + smooth-normals on
# + textures on,base 集内全有,fail-closed 前提满足)。
ARMS: dict[str, list[str]] = {
    "base": [],
    "ris": ["--gi2-ris", "on"],
    "nee": ["--gi2-nee", "on"],
    "both": ["--gi2-ris", "on", "--gi2-nee", "on"],
}
ARM_ORDER = ["base", "ris", "nee", "both"]

FRAMES = 96
WARMUP = 2
DUMP_EVERY = 4     # ⇒ f0000..f0092 共 24 张 + 末帧本体;尾段 f0064 起 8 张为判读口径
AB_DIR = HERE / "ab"


def env_of() -> dict:
    """A/B 环境注入(run_arm.py env_of L71-78 同律):pop 后显式 set。"""
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_G18_AMBIENT"] = "0.004"     # 无 AE 组合下环境光字面(预设 env 等价)
    env["RURIX_REQUIRE_REAL"] = "1"        # 真设备硬门
    env["RURIX_VK_VALIDATION"] = "1"       # 验证层恒开(VUID=0 为门)
    env.pop("RURIX_G31_LAMP_GRID_M", None)  # 灯簇网格旋钮恒缺席(A/B 不动灯面)
    return env


def cmd_of(arm: str, run_dir: Path) -> list[str]:
    """单跑命令构造(r1/r2 除落盘路径外字面全同 ⇒ digest 对比有效;
    dump/evidence 均为验证面 host 写盘,不入渲染语义不入 digest)。"""
    return [
        str(WIN),
        "--frames", str(FRAMES),
        "--warmup", str(WARMUP),
        "--hidden",
        *EXPLICIT_NOAE_BASE,
        *ARMS[arm],
        "--dump-present-raw", str(run_dir / "p.raw"),
        "--dump-present-every", str(DUMP_EVERY),
        "--evidence", str(run_dir / "ev.json"),
    ]


def run_one(arm: str, leg: str, log_rows: list[dict]) -> dict:
    """单跑执行 + 证据抽取(rc/VUID/digest/帧时;失败留 stderr 尾)。"""
    run_dir = AB_DIR / arm / leg
    run_dir.mkdir(parents=True, exist_ok=True)
    cmd = cmd_of(arm, run_dir)
    t0 = time.time()
    # Windows 控制台 GBK 防线:显式 utf-8 + replace(stderr 有中文/全角字面)。
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       encoding="utf-8", errors="replace",
                       timeout=1800, env=env_of())
    wall = time.time() - t0
    ev_p = run_dir / "ev.json"
    evd: dict = {}
    digest = None
    if r.returncode == 0 and ev_p.is_file():
        evd = json.loads(ev_p.read_text(encoding="utf-8"))
        digest = evd.get("digest")
    vuid = (r.stderr or "").count("VUID-")
    ok = r.returncode == 0 and vuid == 0 and digest is not None
    row = {
        "step": f"{arm}_{leg}",
        "arm": arm,
        "leg": leg,
        "rc": r.returncode,
        "vuid": vuid,
        "digest": digest,
        "real_render_frame_ms": evd.get("real_render_frame_ms"),
        "render_max_ms": (evd.get("stats") or {}).get("render_max_ms"),
        "wall_s": round(wall, 1),
        "ok": ok,
    }
    if not ok:
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-8:]
        row["stdout_tail"] = (r.stdout or "").strip().splitlines()[-4:]
    log_rows.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)
    return row


def do_runs() -> int:
    """四臂×双跑序贯执行(任一步 fail 即停,fail-closed;GPU 锁不在本脚本面)。"""
    if not WIN.is_file():
        raise SystemExit(f"FAIL: 窗口 bin 不存在 {WIN}(先建 target-night release)")
    rows: list[dict] = []
    fails = 0
    for arm in ARM_ORDER:
        d1 = run_one(arm, "r1", rows)
        if not d1["ok"]:
            fails += 1
            break
        d2 = run_one(arm, "r2", rows)
        bit = d2["ok"] and d2["digest"] == d1["digest"]
        rows.append({"step": f"{arm}_double_run", "arm": arm,
                     "double_run_bitexact": bit,
                     "digest_r1": d1["digest"], "digest_r2": d2["digest"]})
        print(json.dumps(rows[-1], ensure_ascii=False), flush=True)
        if not bit:
            fails += 1
            break
    out = AB_DIR / "runs_ab.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps({
        "schema": "rurix.day0830.g38.t5.run_ab.v1",
        "fails": fails,
        "frames": FRAMES, "warmup": WARMUP, "dump_every": DUMP_EVERY,
        "explicit_noae_base": EXPLICIT_NOAE_BASE,
        "arms": {a: ARMS[a] for a in ARM_ORDER},
        "env_literal": {"RURIX_G18_AMBIENT": "0.004", "RURIX_REQUIRE_REAL": "1",
                        "RURIX_VK_VALIDATION": "1"},
        "rows": rows,
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"RUN_AB {'PASS' if fails == 0 else 'FAIL'} fails={fails} → {out}")
    return 0 if fails == 0 else 1


def do_dry_run() -> int:
    """打印全部 8 条命令(4 臂×2 跑)与 env 增量字面,零执行。"""
    print("# env 增量(恒注): RURIX_G18_AMBIENT=0.004 RURIX_REQUIRE_REAL=1 "
          "RURIX_VK_VALIDATION=1(RURIX_G31_LAMP_GRID_M 恒缺席)")
    n = 0
    for arm in ARM_ORDER:
        for leg in ("r1", "r2"):
            n += 1
            print(f"[{n}] " + subprocess.list2cmdline(cmd_of(arm, AB_DIR / arm / leg)))
    print(f"# 共 {n} 跑;每跑 {FRAMES}f/warmup{WARMUP},dump every {DUMP_EVERY}")
    return 0


def do_selftest() -> int:
    """零 GPU 自测:自造 8B 头+随机 BGRA 伪 raw 序列(dump 同款字节布局),
    经既有 ab_metrics noise 子命令走通全链(读头/跳头/BGRA→RGB/跨帧 std),
    断言:同帧序列 std=0、噪声序列 std>0、rect 裁切生效。"""
    import struct
    import tempfile

    import numpy as np
    sys.path.insert(0, str(AB_METRICS_DIR))
    import ab_metrics  # noqa: E402(只调用不修改)

    def write_fake_raw(p: Path, rgb01: np.ndarray) -> None:
        # dump 同款字节布局:w/h u32 LE 8B 头 + BGRA8(源码写盘段同构)。
        h, w = rgb01.shape[:2]
        u8 = (np.clip(rgb01, 0, 1) * 255.0 + 0.5).astype(np.uint8)
        bgra = np.zeros((h, w, 4), dtype=np.uint8)
        bgra[:, :, 0] = u8[:, :, 2]
        bgra[:, :, 1] = u8[:, :, 1]
        bgra[:, :, 2] = u8[:, :, 0]
        bgra[:, :, 3] = 255
        p.write_bytes(struct.pack("<II", w, h) + bgra.tobytes())

    rng = np.random.default_rng(830)
    with tempfile.TemporaryDirectory() as td:
        d = Path(td)
        n = 64
        base_img = np.tile(np.array([0.3, 0.4, 0.5]), (n, n, 1))
        # 噪声序列(模拟 8 张尾段帧 f0064..f0092)与恒定序列各 8 张。
        noisy, flat = [], []
        for k in range(8):
            pn = d / f"p.raw.f{64 + 4 * k:04}"
            write_fake_raw(pn, np.clip(base_img + rng.normal(0, 0.03, (n, n, 3)), 0, 1))
            noisy.append(str(pn))
            pf = d / f"flat.raw.f{64 + 4 * k:04}"
            write_fake_raw(pf, base_img)
            flat.append(str(pf))

        ap = ab_metrics.build_parser()

        def noise_of(paths: list[str], rect: str) -> dict:
            a = ap.parse_args(["noise", *paths, "--rect", rect, "--label", "roi"])
            return a.func(a)

        rect = "8,8,32,24"  # 小图代位 rect(生产四 ROI 字面归 judge_ab.py)
        r_noisy = noise_of(noisy, rect)
        r_flat = noise_of(flat, rect)
        # 恒定序列 std ≈ 0(np.std 对常数序列存 ~1e-17 浮点尾数,非逻辑噪声;
        # 容差 1e-12 远低于 u8 量化级 1/255≈3.9e-3,判别力零损)。
        assert r_flat["crops"]["roi"]["temporal_std_mean"] < 1e-12, "恒定序列 std 应≈0"
        assert r_noisy["crops"]["roi"]["temporal_std_p95"] > 1e-3, "噪声序列 p95 应显著 >0"
        assert r_noisy["frames_used"] == 8, "帧数应为 8"
        # 命令构造面自证:8 条命令、臂旗标正确、无 AE 字面不在集内。
        for arm in ARM_ORDER:
            c = cmd_of(arm, d)
            assert "--auto-exposure" not in c, "无 AE 纪律:集内不得出现 --auto-exposure"
            assert c[:2] == [str(WIN), "--frames"], "命令头应为 exe --frames"
            assert ("--gi2-ris" in c) == (arm in ("ris", "both"))
            assert ("--gi2-nee" in c) == (arm in ("nee", "both"))
        print(json.dumps({
            "selftest": "run_ab", "pass": True,
            "noisy_std_p95": r_noisy["crops"]["roi"]["temporal_std_p95"],
            "flat_std_mean": r_flat["crops"]["roi"]["temporal_std_mean"],
            "cmd_count": len(ARM_ORDER) * 2,
        }, ensure_ascii=False))
    return 0


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")   # Windows GBK 控制台防线
    ap = argparse.ArgumentParser(description=__doc__)
    g = ap.add_mutually_exclusive_group()
    g.add_argument("--dry-run", action="store_true", help="打印命令不执行")
    g.add_argument("--selftest", action="store_true", help="伪 raw 走通 noise 链(零 GPU)")
    args = ap.parse_args()
    if args.selftest:
        return do_selftest()
    if args.dry_run:
        return do_dry_run()
    return do_runs()


if __name__ == "__main__":
    sys.exit(main())
