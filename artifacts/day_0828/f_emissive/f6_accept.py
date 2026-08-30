#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase F F6 双形态回正验收（Phase B 共享装配 API 就地改形 → 原形态恢复 + heap 另名）。

GPU 锁内顺序（任一步失败停跑）：
  alloff    我们的锚零漂移：window all-off 8f == 55e4a92d
  bench     我们的锚零漂移：bench 默认 160f == c1d28ad7
  full      我们的锚零漂移：--quality full（十臂）32f == 78113d56（F5_ANCHOR）
  g34_r1    g34 运行时健康：--full orbit 74f rc=0 + VUID=0（原形态消费面真跑）
  g34_r2    g34 双跑位级（f39e9808 系在案锚为 night_baseline EXR 逐帧 digest，
            与 orbit 74f 窗口跑不可比对 → 登记双跑一致性）

env：RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1（f5_accept.py 同律）。
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
G34 = ROOT / "target-night" / "release" / "g34_full_lane.exe"
F = ROOT / "artifacts" / "day_0828" / "f_emissive"
EV = F / "ev"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_BENCH = "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"
ANCHOR_FULL = "sha256:78113d56c6ed6d50dc50e68c8c02448b5fa1452c819246492d055b46d21600a7"


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


LOG = open(F / "f6_log.jsonl", "a", encoding="utf-8")
RESULTS: list[dict] = []


def rec(row: dict) -> None:
    row["t"] = time.strftime("%H:%M:%S")
    LOG.write(json.dumps(row, ensure_ascii=False) + "\n")
    LOG.flush()
    RESULTS.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)


def run_win(tag: str, extra: list[str], frames: int, timeout: int = 1800) -> tuple[bool, str | None, dict]:
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
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-6:]
        row["stdout_tail"] = (r.stdout or "").strip().splitlines()[-4:]
    return ok, got, row


def run_g34(tag: str, ev_path: Path, timeout: int = 3600) -> tuple[bool, dict, dict]:
    cmd = [str(G34), "--full", "--slab-table",
           "milestones/g31/g31_slab_side_table_bistro_interior.json",
           "--frames", "74", "--warmup", "10", "--auto-move", "orbit",
           "--hidden", "--tier", "100", "--evidence", str(ev_path)]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       timeout=timeout, env=env_of())
    wall = time.time() - t0
    vuid = (r.stderr or "").count("VUID-")
    evd: dict = {}
    if r.returncode == 0 and ev_path.is_file():
        evd = json.loads(ev_path.read_text(encoding="utf-8"))
    ok = r.returncode == 0 and vuid == 0 and bool(evd)
    dig = {k: evd.get(k) for k in ("digest", "render_digest") if evd.get(k)}
    seq = evd.get("digest_seq") or []
    row = {"step": tag, "rc": r.returncode, "vuid": vuid, "wall_s": round(wall, 1),
           "digest": evd.get("digest"), "render_digest": evd.get("render_digest"),
           "digest_seq_len": len(seq),
           "real_render_frame_ms": evd.get("real_render_frame_ms"),
           "exit_reason": evd.get("exit_reason")}
    if not ok:
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-8:]
        row["stdout_tail"] = (r.stdout or "").strip().splitlines()[-4:]
    dig["digest_seq"] = seq
    return ok, dig, row


def main() -> int:
    EV.mkdir(parents=True, exist_ok=True)
    fails = 0
    with gpu_device_lock(purpose="day0828 Phase F F6 dual-form acceptance", timeout_s=7200.0):
        # ── ① window all-off 8f 锚 ──
        ok, got, row = run_win("f6_alloff_8f", [], 8)
        row["expect"] = ANCHOR_ALLOFF
        row["pass"] = ok and got == ANCHOR_ALLOFF
        fails += 0 if row["pass"] else 1
        rec(row)
        # ── ② bench 默认 160f 锚 ──
        if fails == 0:
            t0 = time.time()
            rp = subprocess.run(
                [str(BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
                 "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
                 "--out-root", str(F / "arms" / "bench_default_f6")],
                cwd=ROOT, capture_output=True, text=True, timeout=1800, env=env_of())
            receipt = (F / "arms" / "bench_default_f6" / "bistro-interior" / "tier100"
                       / "tsr_device" / "bench_receipt.json")
            got = None
            if rp.returncode == 0 and receipt.is_file():
                got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
            row = {"step": "f6_bench_default_160f", "rc": rp.returncode, "digest": got,
                   "expect": ANCHOR_BENCH, "wall_s": round(time.time() - t0, 1),
                   "pass": got == ANCHOR_BENCH}
            if not row["pass"]:
                row["stderr_tail"] = (rp.stderr or "").strip().splitlines()[-6:]
                fails += 1
            rec(row)
        # ── ③ --quality full 32f 锚 ──
        if fails == 0:
            ok, got, row = run_win("f6_full_32f", ["--quality", "full"], 32)
            row["expect"] = ANCHOR_FULL
            row["pass"] = ok and got == ANCHOR_FULL
            fails += 0 if row["pass"] else 1
            rec(row)
        # ── ④ g34 运行时健康双跑（原形态消费面真跑 + 位级一致）──
        g34_digests: list[dict] = []
        if fails == 0:
            for i in (1, 2):
                ev_path = F / (f"f6_g34_probe.json" if i == 1 else "f6_g34_probe_r2.json")
                ok, dig, row = run_g34(f"f6_g34_probe_r{i}", ev_path)
                row["pass"] = ok
                fails += 0 if ok else 1
                rec(row)
                g34_digests.append(dig)
                if not ok:
                    break
        if fails == 0 and len(g34_digests) == 2:
            same = g34_digests[0] == g34_digests[1]
            rec({"step": "f6_g34_double_run", "pass": same,
                 "double_run_bitexact": same,
                 "note": "f39e9808 系在案锚 = night_baseline EXR 逐帧 digest（g14_3 render 面），与 g34 orbit 74f 窗口跑不可比对 → 双跑位级登记"})
            fails += 0 if same else 1
    (F / "F6_RUNS.json").write_text(
        json.dumps({"schema": "rurix.day0828.f_emissive.f6_runs.v1", "fails": fails,
                    "rows": RESULTS}, ensure_ascii=False, indent=1) + "\n",
        encoding="utf-8")
    print("F6", json.dumps({"fails": fails}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
