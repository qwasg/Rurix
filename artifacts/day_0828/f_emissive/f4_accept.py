#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase F F4 验收执行器（GPU 锁内顺序跑,任一步失败立即停跑保留现场）。

步骤（--steps 逗号选择,默认全跑）：
  alloff   窗口 all-off 8f == 55e4a92d（零漂移）
  nine     显式九臂 em-off 32f（env ambient 0.004）== 9e5f6300（零漂移）
  bench    bench 默认 160f last_frame_digest == c1d28ad7（零漂移）
  em2      em 臂（--quality full --emissive-tex on）32f ×2 双跑位级 + VUID=0（新锚登记）
  dumps    视觉主证 96f 末帧 dump：off（--quality full）vs on（+--emissive-tex on）
  uhd      4K 展示（uhd 契约 + --headless-smoke + em on）96f dump

evidence/log 落 artifacts/day_0828/f_emissive/{ev,f4_log.jsonl,F4_RUNS.json}。
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
ANCHOR_NINE = "sha256:9e5f6300445748af6cdb9e732c7f35eca7dbe66bbc829a67ca691151d7d12df1"
ANCHOR_BENCH = "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"
UHD_CONTRACT = "artifacts/day_0828/uhd/contract_4k.json"
UHD_DIGEST = "sha256:fd172120efcb1ee055c8c8c7260a625f118d4214ca39887e1a0ff0eeed8b5c5f"
UHD_G10 = "artifacts/day_0828/uhd/g10_corpus"

NINE_EXPLICIT = [
    "--smooth-normals", "on", "--ggx", "on", "--lamp-lights", "on", "--lamp-gain", "4",
    "--textures", "on", "--bloom", "on", "--dither", "on", "--auto-exposure", "on",
    "--gi2", "on", "--gi2-clamp", "0.01", "--tsr-quality", "on",
]


def env_of(ambient: str | None) -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    if ambient is not None:
        env["RURIX_G18_AMBIENT"] = ambient
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


LOG = open(F / "f4_log.jsonl", "a", encoding="utf-8")
RESULTS: list[dict] = []


def rec(row: dict) -> None:
    row["t"] = time.strftime("%H:%M:%S")
    LOG.write(json.dumps(row, ensure_ascii=False) + "\n")
    LOG.flush()
    RESULTS.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)


def run_win(tag: str, extra: list[str], frames: int, ambient: str | None,
            dump: str | None = None, timeout: int = 1800) -> tuple[bool, str | None, dict]:
    ev = EV / f"{tag}.json"
    cmd = [str(WIN), "--frames", str(frames), "--warmup", "2", "--hidden",
           *extra, "--evidence", str(ev)]
    if dump:
        cmd += ["--dump-present-raw", dump]
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
    steps = set((sys.argv[sys.argv.index("--steps") + 1] if "--steps" in sys.argv
                 else "alloff,nine,bench,em2,dumps,uhd").split(","))
    EV.mkdir(parents=True, exist_ok=True)
    (F / "png").mkdir(exist_ok=True)
    fails = 0
    with gpu_device_lock(purpose="day0828 Phase F emissive F4 acceptance", timeout_s=7200.0):
        # ── 1 零漂移:all-off ──
        if "alloff" in steps and fails == 0:
            ok, got, row = run_win("alloff_8f", [], 8, None)
            row["expect"] = ANCHOR_ALLOFF
            row["pass"] = ok and got == ANCHOR_ALLOFF
            fails += 0 if row["pass"] else 1
            rec(row)
        # ── 2 零漂移:显式九臂 em-off ──
        if "nine" in steps and fails == 0:
            ok, got, row = run_win("nine_explicit_emoff_32f", NINE_EXPLICIT, 32, "0.004")
            row["expect"] = ANCHOR_NINE
            row["pass"] = ok and got == ANCHOR_NINE
            fails += 0 if row["pass"] else 1
            rec(row)
        # ── 3 零漂移:bench 默认 ──
        if "bench" in steps and fails == 0:
            t0 = time.time()
            rp = subprocess.run(
                [str(BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
                 "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
                 "--out-root", str(F / "arms" / "bench_default")],
                cwd=ROOT, capture_output=True, text=True, timeout=1800, env=env_of(None))
            receipt = (F / "arms" / "bench_default" / "bistro-interior" / "tier100"
                       / "tsr_device" / "bench_receipt.json")
            got = None
            if rp.returncode == 0 and receipt.is_file():
                got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
            row = {"step": "bench_default_160f", "rc": rp.returncode, "digest": got,
                   "expect": ANCHOR_BENCH, "wall_s": round(time.time() - t0, 1),
                   "pass": got == ANCHOR_BENCH}
            if not row["pass"]:
                row["stderr_tail"] = (rp.stderr or "").strip().splitlines()[-6:]
                fails += 1
            rec(row)
        # ── 4 em 臂双跑位级 + validation 静默 ──
        if "em2" in steps and fails == 0:
            ok1, g1, row1 = run_win(
                "em_full_32f_r1", ["--quality", "full", "--emissive-tex", "on"], 32, None)
            row1["pass"] = ok1
            fails += 0 if ok1 else 1
            rec(row1)
            if fails == 0:
                ok2, g2, row2 = run_win(
                    "em_full_32f_r2", ["--quality", "full", "--emissive-tex", "on"], 32, None)
                row2["pass"] = ok2 and g1 == g2
                row2["double_run_bitexact"] = g1 == g2
                fails += 0 if row2["pass"] else 1
                rec(row2)
        # ── 5 视觉主证 dump（96f 收敛末帧;off vs on）──
        if "dumps" in steps and fails == 0:
            ok, got, row = run_win("dump_full_emoff_96f", ["--quality", "full"], 96, None,
                                   dump=str(F / "png" / "off_96.raw"))
            row["pass"] = ok
            fails += 0 if ok else 1
            rec(row)
            if fails == 0:
                ok, got, row = run_win(
                    "dump_full_emon_96f", ["--quality", "full", "--emissive-tex", "on"], 96,
                    None, dump=str(F / "png" / "on_96.raw"))
                row["pass"] = ok
                fails += 0 if ok else 1
                rec(row)
        # ── 6 4K 展示（uhd 契约 headless + em on）──
        if "uhd" in steps and fails == 0:
            ev = EV / "uhd_emon.json"
            cmd = [str(WIN), "--frames", "96", "--warmup", "2", "--headless-smoke",
                   "--quality", "full", "--emissive-tex", "on",
                   "--contract", UHD_CONTRACT, "--expect-digest", UHD_DIGEST,
                   "--g10-dir", UHD_G10,
                   "--dump-present-raw", str(F / "png" / "on_4k.raw"),
                   "--evidence", str(ev)]
            t0 = time.time()
            r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                               timeout=3600, env=env_of(None))
            got = None
            if r.returncode == 0 and ev.is_file():
                got = json.loads(ev.read_text(encoding="utf-8")).get("digest")
            vuid = (r.stderr or "").count("VUID-")
            row = {"step": "uhd_emon_96f", "rc": r.returncode, "digest": got, "vuid": vuid,
                   "wall_s": round(time.time() - t0, 1),
                   "pass": r.returncode == 0 and vuid == 0 and got is not None}
            if not row["pass"]:
                row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-6:]
                row["stdout_tail"] = (r.stdout or "").strip().splitlines()[-4:]
                fails += 1
            rec(row)
    (F / "F4_RUNS.json").write_text(
        json.dumps({"schema": "rurix.day0828.f_emissive.f4_runs.v1", "fails": fails,
                    "rows": RESULTS}, ensure_ascii=False, indent=1) + "\n",
        encoding="utf-8")
    print("F4", json.dumps({"fails": fails, "steps": len(RESULTS)}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
