#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""终局 F3:--quality full 十六臂 窗口风暴 + soak ≥1800s(GPU 锁内)。

风暴:30f/warmup4/dolly + --window-storm 3;判据 rc=0 + resize_eras ≥1 +
VUID=0(AE resize ~12 帧半衰属预期不判红,A2 在案)。
soak:每迭代 full 32f 静态真跑,首迭代 digest 自举为 32f 口径锚,后续迭代
位级同 + VUID=0 + real_render_frame_ms ≤ 11.11(90fps 预算全程);每 3 迭代
插 Stage A 单格探针(bench 默认 160f == c1d28ad7)。任一失败立即停跑留现场。
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
F = ROOT / "artifacts" / "day_0829_realism" / "final"
SOAK = F / "soak"
ANCHOR_STAGEA = "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"
BUDGET_S = 1800.0
PROBE_EVERY = 3
FPS_BUDGET_MS = 11.11


def base_env() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def main() -> int:
    SOAK.mkdir(parents=True, exist_ok=True)
    log = open(SOAK / "soak_log.jsonl", "a", encoding="utf-8")
    summary: dict = {"schema": "rurix.day0829.realism.f3_storm_soak.v1"}
    with gpu_device_lock(purpose="day0829 realism F3 storm+soak full16", timeout_s=10800.0):
        # ── ① 窗口风暴 ──
        ev = F / "ev" / "f3_storm.json"
        t0 = time.time()
        r = subprocess.run(
            [str(WIN), "--frames", "30", "--warmup", "4", "--hidden",
             "--auto-move", "dolly", "--quality", "full", "--window-storm", "3",
             "--evidence", str(ev)],
            cwd=ROOT, capture_output=True, text=True, timeout=1800, env=base_env())
        wall = time.time() - t0
        eras = None
        exit_reason = None
        if ev.is_file():
            d = json.loads(ev.read_text(encoding="utf-8"))
            eras = d.get("resize_eras")
            exit_reason = d.get("exit_reason")
        vuid = (r.stderr or "").count("VUID-")
        storm_ok = r.returncode == 0 and (eras or 0) >= 1 and vuid == 0
        summary["storm"] = {
            "arm": "--quality full(16臂) --window-storm 3（dolly 30f）",
            "rc": r.returncode, "wall_s": round(wall, 1),
            "resize_eras": eras, "exit_reason": exit_reason,
            "vuid_hits": vuid, "pass": storm_ok,
            "stderr_tail": (r.stderr or "").strip().splitlines()[-3:],
        }
        print(json.dumps(summary["storm"], ensure_ascii=False), flush=True)
        if not storm_ok:
            (F / "F3_SUMMARY.json").write_text(
                json.dumps(summary, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
            print("F3", json.dumps({"fails": 1, "stage": "storm"}))
            return 1
        # ── ② soak ≥1800s ──
        t_start = time.time()
        it = 0
        fails = 0
        anchor32: str | None = None
        stagea_runs = 0
        frame_ms_max = 0.0
        fail_reason = None
        while time.time() - t_start < BUDGET_S and fails == 0:
            it += 1
            evi = SOAK / f"win_it{it}.json"
            t1 = time.time()
            r = subprocess.run(
                [str(WIN), "--frames", "32", "--warmup", "2", "--hidden",
                 "--quality", "full", "--evidence", str(evi)],
                cwd=ROOT, capture_output=True, text=True, timeout=1800, env=base_env())
            wall = time.time() - t1
            got = None
            ms = None
            if r.returncode == 0 and evi.is_file():
                d = json.loads(evi.read_text(encoding="utf-8"))
                got = d.get("digest")
                ms = d.get("real_render_frame_ms")
            vuid = (r.stderr or "").count("VUID-")
            if anchor32 is None and got is not None:
                anchor32 = got
            fps_ok = ms is not None and ms <= FPS_BUDGET_MS
            ok = (r.returncode == 0 and got == anchor32 and vuid == 0 and fps_ok)
            if not ok:
                fails += 1
                fail_reason = {"it": it, "rc": r.returncode, "got": got, "vuid": vuid,
                               "frame_ms": ms,
                               "stderr_tail": (r.stderr or "").strip().splitlines()[-5:]}
            if ms is not None:
                frame_ms_max = max(frame_ms_max, ms)
            rec = {"it": it, "t_s": round(time.time() - t_start, 1), "win_ok": ok,
                   "win_digest_stable": got == anchor32, "frame_ms": ms,
                   "vuid_hits": vuid, "win_wall_s": round(wall, 1)}
            if ok and it % PROBE_EVERY == 0 and time.time() - t_start < BUDGET_S:
                t2 = time.time()
                rp = subprocess.run(
                    [str(BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
                     "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
                     "--out-root", str(SOAK / "stagea")],
                    cwd=ROOT, capture_output=True, text=True, timeout=1800, env=base_env())
                receipt = (SOAK / "stagea" / "bistro-interior" / "tier100"
                           / "tsr_device" / "bench_receipt.json")
                pg = None
                if rp.returncode == 0 and receipt.is_file():
                    pg = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
                probe_ok = pg == ANCHOR_STAGEA
                stagea_runs += 1
                if not probe_ok:
                    fails += 1
                    fail_reason = {"it": it, "probe": pg}
                rec["stagea_ok"] = probe_ok
                rec["stagea_wall_s"] = round(time.time() - t2, 1)
            log.write(json.dumps(rec, ensure_ascii=False) + "\n")
            log.flush()
            print(json.dumps(rec, ensure_ascii=False), flush=True)
    elapsed = time.time() - t_start
    summary["soak"] = {
        "budget_s": BUDGET_S, "elapsed_s": round(elapsed, 1), "iterations": it,
        "anchor32_full16": anchor32, "stagea_probes": stagea_runs,
        "frame_ms_max": round(frame_ms_max, 3), "fps_budget_ms": FPS_BUDGET_MS,
        "fails": fails, "fail_reason": fail_reason,
        "pass": fails == 0 and elapsed >= BUDGET_S,
    }
    (F / "F3_SUMMARY.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("F3", json.dumps({"fails": fails, "elapsed_s": round(elapsed, 1)}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
