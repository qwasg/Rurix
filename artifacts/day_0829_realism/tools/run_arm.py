#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Day 0829 六臂通用验收器(GPU 锁内,任一步失败停跑)。

用法: py -3 run_arm.py --arm ao   (闭集 ao|soft|refl|gitex)

序列(a1 红修 #1 后定形):
  off_alloff   all-off 8f == 55e4a92d(零漂移证 1)
  off_full     --quality full 96f == de342586(零漂移证 2)
  on_run1/2    --quality full + 臂旗标 96f 双跑位级一致 + VUID=0
  ab_off/on    无 AE 显式组合(RURIX_G18_AMBIENT=0.004)off/on + presented
               raw dump → 臂特定 A/B 判据(AE 反馈污染教训固化)
  帧时增量记账(90fps=11.11ms 预算;基线 no-AE off 腿自证)。
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_FULL = "sha256:de342586e452b903a2df7b744b9f67ad5b95b6bc5e3c17e0257def516ffc7211"

EXPLICIT_NOAE = [
    "--smooth-normals", "on", "--ggx", "on",
    "--lamp-lights", "on", "--lamp-gain", "4",
    "--textures", "on", "--bloom", "on", "--dither", "on",
    "--tsr-quality", "on", "--gi2", "on", "--gi2-clamp", "0.01",
    "--emissive-tex", "on",
]

ARMS: dict[str, dict] = {
    "ao": {
        "dir": "a2_ao",
        "flags": ["--rt-ao", "on"],
        "note": "接触遮蔽:掩码变暗 + 全屏 mean 降(遮蔽减能)",
    },
    "soft": {
        "dir": "a5_softshadow",
        "flags": ["--soft-shadows", "on"],
        "note": "半影重分布:掩码存在 + 全屏能量近守恒(|Δmean| 小)",
    },
    "refl": {
        "dir": "a3_reflect",
        "flags": ["--rt-reflect", "on"],
        "note": "反射加性:掩码亮度 on>off + 全屏能量增",
    },
    "gitex": {
        "dir": "a6_gi2tex",
        "flags": ["--gi2-tex", "on"],
        "note": "反弹色彩重分布:掩码存在 + 不爆能(|Δmean| 有界)",
    },
    "nrm": {
        "dir": "a4_normalmap",
        "flags": ["--normal-maps", "on"],
        "note": "法线细节重分布:掩码大面 + 能量近守恒(|Δmean| 有界)",
    },
}


def env_of(ambient: str | None) -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    if ambient is not None:
        env["RURIX_G18_AMBIENT"] = ambient
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def load_raw_luma(path: Path) -> tuple[list[float], int]:
    b = path.read_bytes()
    n = len(b) // 4
    lum = [0.0] * n
    for i in range(n):
        lum[i] = 0.2126 * b[i * 4 + 2] + 0.7152 * b[i * 4 + 1] + 0.0722 * b[i * 4]
    return lum, n


def ab_metrics(off_p: Path, on_p: Path) -> dict:
    lo, n = load_raw_luma(off_p)
    ln, n2 = load_raw_luma(on_p)
    assert n == n2, "raw 尺寸不一"
    thr = 2.0
    changed = [i for i in range(n) if abs(ln[i] - lo[i]) > thr]
    frac = len(changed) / n
    m_off = sum(lo) / n
    m_on = sum(ln) / n
    ch_off = sum(lo[i] for i in changed) / max(len(changed), 1)
    ch_on = sum(ln[i] for i in changed) / max(len(changed), 1)
    return {
        "changed_frac": round(frac, 6),
        "fullscreen_mean_off": round(m_off, 4),
        "fullscreen_mean_on": round(m_on, 4),
        "changed_mean_off": round(ch_off, 4),
        "changed_mean_on": round(ch_on, 4),
    }


def judge(arm: str, m: dict) -> bool:
    frac = m["changed_frac"]
    dm = m["fullscreen_mean_on"] - m["fullscreen_mean_off"]
    dch = m["changed_mean_on"] - m["changed_mean_off"]
    if arm == "ao":
        return 0.0 < frac < 0.90 and dm < 0.0 and dch < 0.0
    if arm == "soft":
        return 0.0 < frac < 0.50 and abs(dm) < 3.0
    if arm == "refl":
        return 0.0 < frac < 0.60 and dch > 0.0 and dm > 0.0
    if arm == "gitex":
        return 0.0 < frac and abs(dm) < 10.0
    if arm == "nrm":
        return 0.0 < frac < 0.98 and abs(dm) < 5.0
    return False


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--arm", required=True, choices=sorted(ARMS))
    args = ap.parse_args()
    spec = ARMS[args.arm]
    A = ROOT / "artifacts" / "day_0829_realism" / spec["dir"]
    ev_dir = A / "ev"
    png = A / "png"
    ev_dir.mkdir(parents=True, exist_ok=True)
    png.mkdir(parents=True, exist_ok=True)
    log = open(A / "arm_log.jsonl", "a", encoding="utf-8")
    results: list[dict] = []

    def rec(row: dict) -> None:
        row["t"] = time.strftime("%H:%M:%S")
        log.write(json.dumps(row, ensure_ascii=False) + "\n")
        log.flush()
        results.append(row)
        print(json.dumps(row, ensure_ascii=False), flush=True)

    def run_win(tag: str, extra: list[str], frames: int, ambient: str | None) -> tuple[bool, str | None, dict]:
        ev = ev_dir / f"{tag}.json"
        cmd = [str(WIN), "--frames", str(frames), "--warmup", "2", "--hidden",
               *extra, "--evidence", str(ev)]
        t0 = time.time()
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                           timeout=1800, env=env_of(ambient))
        wall = time.time() - t0
        got = None
        evd: dict = {}
        if r.returncode == 0 and ev.is_file():
            evd = json.loads(ev.read_text(encoding="utf-8"))
            got = evd.get("digest")
        vuid = (r.stderr or "").count("VUID-")
        ok = r.returncode == 0 and vuid == 0 and got is not None
        row = {"step": tag, "rc": r.returncode, "digest": got, "vuid": vuid,
               "wall_s": round(wall, 1),
               "real_render_frame_ms": evd.get("real_render_frame_ms")}
        if not ok:
            row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-8:]
            row["stdout_tail"] = (r.stdout or "").strip().splitlines()[-4:]
        return ok, got, row

    fails = 0
    with gpu_device_lock(purpose=f"day0829 realism arm {args.arm} acceptance", timeout_s=7200.0):
        ok, got, row = run_win(f"{args.arm}_off_alloff_8f", [], 8, None)
        row["expect"] = ANCHOR_ALLOFF
        row["pass"] = ok and got == ANCHOR_ALLOFF
        fails += 0 if row["pass"] else 1
        rec(row)
        if fails == 0:
            ok, got, row = run_win(f"{args.arm}_off_full_96f", ["--quality", "full"], 96, None)
            row["expect"] = ANCHOR_FULL
            row["pass"] = ok and got == ANCHOR_FULL
            fails += 0 if row["pass"] else 1
            rec(row)
        d1 = None
        if fails == 0:
            ok, d1, row = run_win(f"{args.arm}_on_run1", ["--quality", "full", *spec["flags"]], 96, None)
            row["pass"] = ok
            fails += 0 if ok else 1
            rec(row)
        if fails == 0:
            ok, d2, row = run_win(f"{args.arm}_on_run2", ["--quality", "full", *spec["flags"]], 96, None)
            row["pass"] = ok and d2 == d1
            row["double_run_bitexact"] = d2 == d1
            fails += 0 if row["pass"] else 1
            rec(row)
        if fails == 0:
            ok, _, row = run_win(f"{args.arm}_ab_off", [
                *EXPLICIT_NOAE,
                "--dump-present-raw", str(png / "ab_off.raw"),
                "--dump-present-every", "95",
            ], 96, "0.004")
            row["pass"] = ok
            fails += 0 if ok else 1
            rec(row)
        if fails == 0:
            ok, _, row = run_win(f"{args.arm}_ab_on", [
                *EXPLICIT_NOAE, *spec["flags"],
                "--dump-present-raw", str(png / "ab_on.raw"),
                "--dump-present-every", "95",
            ], 96, "0.004")
            row["pass"] = ok
            fails += 0 if ok else 1
            rec(row)
    if fails == 0:
        m = ab_metrics(png / "ab_off.raw", png / "ab_on.raw")
        m["step"] = f"{args.arm}_ab_metrics"
        m["pass"] = judge(args.arm, m)
        m["note"] = spec["note"]
        fails += 0 if m["pass"] else 1
        rec(m)
    (A / "ARM_RUNS.json").write_text(
        json.dumps({"schema": f"rurix.day0829.realism.{args.arm}.v1", "fails": fails,
                    "rows": results}, ensure_ascii=False, indent=1) + "\n",
        encoding="utf-8")
    print(f"ARM-{args.arm.upper()}", json.dumps({"fails": fails}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
