#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase F F5 并入预设验收（--quality full 已展开含 --emissive-tex on 的重建面）。

GPU 锁内顺序（任一步失败停跑）：
  conflict  fail-closed 冒烟：--quality full --emissive-tex on = 拒跑（零 GPU）
  alloff    重建面零漂移：all-off 8f == 55e4a92d
  bench     重建面零漂移：bench 默认 160f == c1d28ad7
  full2     --quality full（十臂）32f ×2 双跑位级 + VUID=0 → 新锚收割落 F5_ANCHOR.json
  equiv     显式十臂（九臂 + --emissive-tex on + env ambient 0.004）== 新锚位级
  storm     --quality full --window-storm 3（dolly 30f）rc=0 + VUID=0 + 复验 full == 新锚

soak 另跑 f5_soak.py（读 F5_ANCHOR.json,e5_soak.py 同模式 ≥1800s）。
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
BENCH = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"
F = ROOT / "artifacts" / "day_0828" / "f_emissive"
EV = F / "ev"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_BENCH = "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"

TEN_EXPLICIT = [
    "--smooth-normals", "on", "--ggx", "on", "--lamp-lights", "on", "--lamp-gain", "4",
    "--textures", "on", "--bloom", "on", "--dither", "on", "--auto-exposure", "on",
    "--gi2", "on", "--gi2-clamp", "0.01", "--tsr-quality", "on", "--emissive-tex", "on",
]


def env_of(ambient: str | None) -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    if ambient is not None:
        env["RURIX_G18_AMBIENT"] = ambient
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


LOG = open(F / "f5_log.jsonl", "a", encoding="utf-8")
RESULTS: list[dict] = []


def rec(row: dict) -> None:
    row["t"] = time.strftime("%H:%M:%S")
    LOG.write(json.dumps(row, ensure_ascii=False) + "\n")
    LOG.flush()
    RESULTS.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)


def run_win(tag: str, extra: list[str], frames: int, ambient: str | None,
            timeout: int = 1800) -> tuple[bool, str | None, dict]:
    ev = EV / f"{tag}.json"
    cmd = [str(WIN), "--frames", str(frames), "--warmup", "2", "--hidden",
           *extra, "--evidence", str(ev)]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       timeout=timeout, env=env_of(ambient))
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
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-6:]
        row["stdout_tail"] = (r.stdout or "").strip().splitlines()[-4:]
    return ok, got, row


def main() -> int:
    EV.mkdir(parents=True, exist_ok=True)
    fails = 0
    anchor_full: str | None = None
    # ── 0 fail-closed 冒烟（零 GPU:解析层拒跑）──
    r = subprocess.run(
        [str(WIN), "--frames", "8", "--quality", "full", "--emissive-tex", "on"],
        cwd=ROOT, capture_output=True, text=True, timeout=120, env=env_of(None))
    conflict_ok = r.returncode != 0 and "--emissive-tex" in (r.stdout + r.stderr)
    rec({"step": "conflict_smoke", "rc": r.returncode, "pass": conflict_ok})
    fails += 0 if conflict_ok else 1
    with gpu_device_lock(purpose="day0828 Phase F F5 preset acceptance", timeout_s=7200.0):
        if fails == 0:
            ok, got, row = run_win("f5_alloff_8f", [], 8, None)
            row["expect"] = ANCHOR_ALLOFF
            row["pass"] = ok and got == ANCHOR_ALLOFF
            fails += 0 if row["pass"] else 1
            rec(row)
        if fails == 0:
            t0 = time.time()
            rp = subprocess.run(
                [str(BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
                 "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
                 "--out-root", str(F / "arms" / "bench_default_f5")],
                cwd=ROOT, capture_output=True, text=True, timeout=1800, env=env_of(None))
            receipt = (F / "arms" / "bench_default_f5" / "bistro-interior" / "tier100"
                       / "tsr_device" / "bench_receipt.json")
            got = None
            if rp.returncode == 0 and receipt.is_file():
                got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
            row = {"step": "f5_bench_default_160f", "rc": rp.returncode, "digest": got,
                   "expect": ANCHOR_BENCH, "wall_s": round(time.time() - t0, 1),
                   "pass": got == ANCHOR_BENCH}
            if not row["pass"]:
                row["stderr_tail"] = (rp.stderr or "").strip().splitlines()[-6:]
                fails += 1
            rec(row)
        if fails == 0:
            ok1, g1, row1 = run_win("f5_full_32f_r1", ["--quality", "full"], 32, None)
            row1["pass"] = ok1
            fails += 0 if ok1 else 1
            rec(row1)
            if fails == 0:
                ok2, g2, row2 = run_win("f5_full_32f_r2", ["--quality", "full"], 32, None)
                row2["pass"] = ok2 and g1 == g2
                row2["double_run_bitexact"] = g1 == g2
                fails += 0 if row2["pass"] else 1
                rec(row2)
                if row2["pass"]:
                    anchor_full = g1
        if fails == 0 and anchor_full:
            ok, got, row = run_win("f5_ten_explicit_32f", TEN_EXPLICIT, 32, "0.004")
            row["expect"] = anchor_full
            row["pass"] = ok and got == anchor_full
            row["preset_equiv_bitexact"] = got == anchor_full
            fails += 0 if row["pass"] else 1
            rec(row)
        if fails == 0 and anchor_full:
            ev = EV / "f5_storm.json"
            cmd = [str(WIN), "--frames", "30", "--warmup", "2", "--hidden",
                   "--quality", "full", "--auto-move", "dolly", "--window-storm", "3",
                   "--evidence", str(ev)]
            t0 = time.time()
            r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                               timeout=1800, env=env_of(None))
            vuid = (r.stderr or "").count("VUID-")
            evd = {}
            if r.returncode == 0 and ev.is_file():
                evd = json.loads(ev.read_text(encoding="utf-8"))
            row = {"step": "f5_storm3_dolly30f", "rc": r.returncode, "vuid": vuid,
                   "resize_eras": evd.get("resize_eras"),
                   "exit_reason": evd.get("exit_reason"),
                   "wall_s": round(time.time() - t0, 1),
                   "pass": r.returncode == 0 and vuid == 0}
            if not row["pass"]:
                row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-8:]
                fails += 1
            rec(row)
            if fails == 0:
                ok, got, row = run_win("f5_full_poststorm_32f", ["--quality", "full"], 32, None)
                row["expect"] = anchor_full
                row["pass"] = ok and got == anchor_full
                fails += 0 if row["pass"] else 1
                rec(row)
    if anchor_full:
        (F / "F5_ANCHOR.json").write_text(json.dumps({
            "schema": "rurix.day0828.f_emissive.f5_anchor.v1",
            "window_full_ten_arm_32f": anchor_full,
            "note": "--quality full = 十臂（九臂 + --emissive-tex on）;旧九臂 full 锚 9e5f6300 作废登记",
        }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    (F / "F5_RUNS.json").write_text(
        json.dumps({"schema": "rurix.day0828.f_emissive.f5_runs.v1", "fails": fails,
                    "rows": RESULTS}, ensure_ascii=False, indent=1) + "\n",
        encoding="utf-8")
    print("F5", json.dumps({"fails": fails, "anchor_full": anchor_full}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
