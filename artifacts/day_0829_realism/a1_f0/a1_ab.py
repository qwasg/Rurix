#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""臂① A/B 红修 #1:无 AE 显式组合对照(首版判据在 --quality full 含自动曝光
反馈下失真——F0 修伤令金属高光能量大增,AE 全屏压暗 ⇒ diff 99.9% 全屏 mean
81→15;本版去 AE 定曝光对照,判据 = 掩码占比 ∈(0,40%) + 掩码亮度 on>off +
全屏能量 on>off〔加性能量,无 AE 压制〕)。

显式组合 = full 十臂去 --auto-exposure(env RURIX_G18_AMBIENT=0.004 显式注入
= full 预设同值);双方同组合只差 --metal-f0,公平对照。
"""
from __future__ import annotations

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
A = ROOT / "artifacts" / "day_0829_realism" / "a1_f0"
EV = A / "ev"
PNG = A / "png"

EXPLICIT_NOAE = [
    "--smooth-normals", "on", "--ggx", "on",
    "--lamp-lights", "on", "--lamp-gain", "4",
    "--textures", "on", "--bloom", "on", "--dither", "on",
    "--tsr-quality", "on", "--gi2", "on", "--gi2-clamp", "0.01",
    "--emissive-tex", "on",
]


def env_of() -> dict:
    env = dict(os.environ)
    env["RURIX_G18_AMBIENT"] = "0.004"
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


LOG = open(A / "a1_log.jsonl", "a", encoding="utf-8")
RESULTS: list[dict] = []


def rec(row: dict) -> None:
    row["t"] = time.strftime("%H:%M:%S")
    LOG.write(json.dumps(row, ensure_ascii=False) + "\n")
    LOG.flush()
    RESULTS.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)


def run_win(tag: str, extra: list[str], frames: int = 96) -> tuple[bool, dict]:
    ev = EV / f"{tag}.json"
    cmd = [str(WIN), "--frames", str(frames), "--warmup", "2", "--hidden",
           *EXPLICIT_NOAE, *extra, "--evidence", str(ev)]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       timeout=1800, env=env_of())
    wall = time.time() - t0
    evd: dict = {}
    if r.returncode == 0 and ev.is_file():
        evd = json.loads(ev.read_text(encoding="utf-8"))
    vuid = (r.stderr or "").count("VUID-")
    ok = r.returncode == 0 and vuid == 0 and bool(evd)
    row = {"step": tag, "rc": r.returncode, "digest": evd.get("digest"), "vuid": vuid,
           "wall_s": round(wall, 1),
           "real_render_frame_ms": evd.get("real_render_frame_ms")}
    if not ok:
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-8:]
    return ok, row


def load_raw_luma(path: Path) -> tuple[list[float], int]:
    b = path.read_bytes()
    n = len(b) // 4
    lum = [0.0] * n
    for i in range(n):
        lum[i] = 0.2126 * b[i * 4 + 2] + 0.7152 * b[i * 4 + 1] + 0.0722 * b[i * 4]
    return lum, n


def main() -> int:
    EV.mkdir(parents=True, exist_ok=True)
    PNG.mkdir(parents=True, exist_ok=True)
    fails = 0
    with gpu_device_lock(purpose="day0829 realism a1 AB noAE rerun", timeout_s=7200.0):
        ok, row = run_win("a1_ab_noae_off", [
            "--dump-present-raw", str(PNG / "noae_off.raw"),
            "--dump-present-every", "95",
        ])
        row["pass"] = ok
        fails += 0 if ok else 1
        rec(row)
        if fails == 0:
            ok, row = run_win("a1_ab_noae_on", [
                "--metal-f0", "on",
                "--dump-present-raw", str(PNG / "noae_on.raw"),
                "--dump-present-every", "95",
            ])
            row["pass"] = ok
            fails += 0 if ok else 1
            rec(row)
    if fails == 0:
        lo, n = load_raw_luma(PNG / "noae_off.raw")
        ln, n2 = load_raw_luma(PNG / "noae_on.raw")
        assert n == n2
        thr = 2.0
        changed = [i for i in range(n) if abs(ln[i] - lo[i]) > thr]
        frac = len(changed) / n
        mean_off = sum(lo) / n
        mean_on = sum(ln) / n
        ch_off = sum(lo[i] for i in changed) / max(len(changed), 1)
        ch_on = sum(ln[i] for i in changed) / max(len(changed), 1)
        row = {
            "step": "a1_ab_metrics_noae",
            "changed_frac": round(frac, 6),
            "fullscreen_mean_off": round(mean_off, 4),
            "fullscreen_mean_on": round(mean_on, 4),
            "changed_mean_off": round(ch_off, 4),
            "changed_mean_on": round(ch_on, 4),
            "pass": (0.0 < frac < 0.40) and (ch_on > ch_off) and (mean_on > mean_off),
            "note": "无 AE 定曝光对照:掩码 = F0 修伤面+bloom 光晕;判据 = 掩码 ∈(0,40%) + 掩码亮度恢复 + 全屏能量加性增",
        }
        fails += 0 if row["pass"] else 1
        rec(row)
    summ = json.loads((A / "A1_RUNS.json").read_text(encoding="utf-8")) if (A / "A1_RUNS.json").is_file() else {"rows": []}
    summ["rows"] = summ.get("rows", []) + RESULTS
    summ["ab_rerun_noae_fails"] = fails
    summ["red_fix_1"] = "A/B 判据修正:full 含 AE 反馈失真(F0 能量增 → AE 全屏压暗),改无 AE 显式组合对照;锚/双跑/validation 腿首跑已全绿不重跑"
    (A / "A1_RUNS.json").write_text(json.dumps(summ, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("A1-AB", json.dumps({"fails": fails}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
