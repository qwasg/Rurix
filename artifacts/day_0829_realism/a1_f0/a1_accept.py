#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Day 0829 臂① --metal-f0 金属 F0 修伤验收（GPU 锁内,任一步失败停跑）。

序列:
  off_alloff   all-off 8f == 55e4a92d（零漂移证 1）
  off_full     --quality full 96f == de342586（零漂移证 2,realism off 恒载
               night_0828 既有工件;带 presented raw dump + luma sidecar——
               dump 为 present 后 readback 侧车,零扰验证一并承载）
  on_run1/2    --quality full --metal-f0 on 96f 双跑位级一致 + VUID=0
               （run1 带 dump/luma;run2 纯净——若 run1 与 run2 digest 不一,
               即 dump 扰动证据,fail 登记）
  离线判据      diff(on,off):变化像素占比 ∈ (0, 40%)（金属面存在且非全屏）,
               变化像素亮度 on>off（F0 恢复 = 高光能量增加）,全屏 mean 漂移
               记录;帧时增量记录（90fps 预算账本）。

env:RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1;移除 RURIX_G18_AMBIENT。
"""
from __future__ import annotations

import json
import os
import struct
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

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_FULL = "sha256:de342586e452b903a2df7b744b9f67ad5b95b6bc5e3c17e0257def516ffc7211"


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
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


def run_win(tag: str, extra: list[str], frames: int = 96, timeout: int = 1800) -> tuple[bool, str | None, dict]:
    ev = EV / f"{tag}.json"
    cmd = [str(WIN), "--frames", str(frames), "--warmup", "2", "--hidden",
           *extra, "--evidence", str(ev)]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       timeout=timeout, env=env_of())
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


def load_raw_bgra(path: Path) -> tuple[list[float], int]:
    """presented raw = BGRA8(窗口 channel_order bgra8_unorm,1920x1080)。
    返回逐像素 luma(0..255 浮点)列表 + 像素数。"""
    b = path.read_bytes()
    n = len(b) // 4
    lum = [0.0] * n
    for i in range(n):
        bb = b[i * 4]
        gg = b[i * 4 + 1]
        rr = b[i * 4 + 2]
        lum[i] = 0.2126 * rr + 0.7152 * gg + 0.0722 * bb
    return lum, n


def main() -> int:
    EV.mkdir(parents=True, exist_ok=True)
    PNG.mkdir(parents=True, exist_ok=True)
    fails = 0
    with gpu_device_lock(purpose="day0829 realism a1 metal-f0 acceptance", timeout_s=7200.0):
        # ── ① off 零漂移证 1:all-off 8f ──
        ok, got, row = run_win("a1_off_alloff_8f", [], 8)
        row["expect"] = ANCHOR_ALLOFF
        row["pass"] = ok and got == ANCHOR_ALLOFF
        fails += 0 if row["pass"] else 1
        rec(row)
        # ── ② off 零漂移证 2:full 96f(带 dump 侧车 = 零扰验证一并承载)──
        if fails == 0:
            ok, got, row = run_win("a1_off_full_96f", [
                "--quality", "full",
                "--dump-present-raw", str(PNG / "off.raw"),
                "--dump-present-every", "95",
                "--present-luma-out", str(EV / "off_luma.json"),
            ])
            row["expect"] = ANCHOR_FULL
            row["pass"] = ok and got == ANCHOR_FULL
            fails += 0 if row["pass"] else 1
            rec(row)
        # ── ③ on 双跑位级(run1 带 dump,run2 纯净)──
        d1 = None
        if fails == 0:
            ok, d1, row = run_win("a1_on_run1", [
                "--quality", "full", "--metal-f0", "on",
                "--dump-present-raw", str(PNG / "on.raw"),
                "--dump-present-every", "95",
                "--present-luma-out", str(EV / "on_luma.json"),
            ])
            row["pass"] = ok
            fails += 0 if ok else 1
            rec(row)
        if fails == 0:
            ok, d2, row = run_win("a1_on_run2", ["--quality", "full", "--metal-f0", "on"])
            row["pass"] = ok and d2 == d1
            row["double_run_bitexact"] = d2 == d1
            fails += 0 if row["pass"] else 1
            rec(row)
    # ── ④ 离线 A/B 判据(diff 掩码 = 金属面)──
    if fails == 0:
        off_p = PNG / "off.raw"
        on_p = PNG / "on.raw"
        if off_p.is_file() and on_p.is_file():
            lo, n = load_raw_bgra(off_p)
            ln, n2 = load_raw_bgra(on_p)
            assert n == n2, "raw 尺寸不一"
            thr = 2.0  # 8bit luma 变化阈(>2/255 记变化像素)
            changed = [i for i in range(n) if abs(ln[i] - lo[i]) > thr]
            frac = len(changed) / n
            mean_off = sum(lo) / n
            mean_on = sum(ln) / n
            ch_off = sum(lo[i] for i in changed) / max(len(changed), 1)
            ch_on = sum(ln[i] for i in changed) / max(len(changed), 1)
            row = {
                "step": "a1_ab_metrics",
                "changed_frac": round(frac, 6),
                "fullscreen_mean_off": round(mean_off, 4),
                "fullscreen_mean_on": round(mean_on, 4),
                "changed_mean_off": round(ch_off, 4),
                "changed_mean_on": round(ch_on, 4),
                "pass": (0.0 < frac < 0.40) and (ch_on > ch_off),
                "note": "diff 掩码 = F0 修伤面(metal>0 像素);判据 = 掩码存在且非全屏 + 掩码亮度 on>off(高光能量恢复)",
            }
            fails += 0 if row["pass"] else 1
            rec(row)
        else:
            rec({"step": "a1_ab_metrics", "pass": False, "note": "raw dump 缺件"})
            fails += 1
    (A / "A1_RUNS.json").write_text(
        json.dumps({"schema": "rurix.day0829.realism.a1_f0.v1", "fails": fails,
                    "rows": RESULTS}, ensure_ascii=False, indent=1) + "\n",
        encoding="utf-8")
    print("A1", json.dumps({"fails": fails}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
