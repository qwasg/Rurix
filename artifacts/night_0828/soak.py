#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""夜间巡航稳定性 soak：全特性栈反复真跑，零失败/零漂移 honest 口径。

每迭代：①窗口车道全五臂（smooth-normals+ggx+bloom+dither + 环境光 env）
--auto-move dolly 12 帧 → PASS + validation 静默 + digest 稳定（首迭代定锚）；
②每 N 迭代替跑一次 Stage A 单格（bistro t100 tsr 默认臂）→ digest == 在案锚
（证明画质臂全程零污染默认面）。

用法: py -3 soak.py [--budget-s 1800] [--probe-every 5]
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
BENCH = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"
OUT = ROOT / "artifacts" / "night_0828" / "soak"
STAGE_A_BISTRO_T100_TSR = "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"


def run(cmd, env_extra, timeout=300):
    env = dict(os.environ)
    env.update(env_extra)
    env["RURIX_VK_VALIDATION"] = "1"
    t0 = time.monotonic()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    return r, time.monotonic() - t0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--budget-s", type=int, default=1800)
    ap.add_argument("--probe-every", type=int, default=5)
    args = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    log = OUT / "soak_log.jsonl"

    t_end = time.monotonic() + args.budget_s
    it = 0
    fails = 0
    anchor_digest = None
    t0 = time.monotonic()
    while time.monotonic() < t_end:
        it += 1
        rec = {"it": it, "t_s": round(time.monotonic() - t0, 1)}
        with gpu_device_lock(purpose=f"night0828 soak it{it}"):
            # ① 全特性栈窗口车道
            ev = OUT / f"win_it{it}.json"
            r, dt = run([str(WIN), "--frames", "12", "--warmup", "4", "--hidden",
                         "--auto-move", "dolly", "--smooth-normals", "on", "--ggx", "on",
                         "--bloom", "on", "--dither", "on", "--evidence", str(ev)],
                        {"RURIX_G18_AMBIENT": "0.004"})
            out = (r.stdout or "") + (r.stderr or "")
            ok = r.returncode == 0 and "PASS" in out and "Validation Error" not in out and "VUID-" not in out
            dig = None
            if ev.is_file():
                dig = json.loads(ev.read_text(encoding="utf-8")).get("digest")
            if it == 1:
                anchor_digest = dig
            dig_stable = (dig == anchor_digest)
            rec.update({"win_ok": ok, "win_digest": dig, "win_digest_stable": dig_stable, "win_wall_s": round(dt, 1)})
            if not ok or not dig_stable:
                fails += 1
                rec["fail_reason"] = f"win ok={ok} dig_stable={dig_stable} tail={out[-200:]}"
            # ② 周期性 Stage A 单格
            if it % args.probe_every == 0:
                r2, _ = run([str(BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
                             "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
                             "--out-root", str(OUT / "stagea")], {"RURIX_REQUIRE_REAL": "1"}, timeout=600)
                receipt = OUT / "stagea" / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
                got = None
                if r2.returncode == 0 and receipt.is_file():
                    got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
                stagea_ok = got == STAGE_A_BISTRO_T100_TSR
                rec["stagea_ok"] = stagea_ok
                if not stagea_ok:
                    fails += 1
                    rec["fail_reason"] = f"stagea got={got}"
        with open(log, "a", encoding="utf-8") as f:
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")
        print(f"[soak it{it}] win_ok={rec['win_ok']} dig_stable={rec['win_digest_stable']} "
              f"stagea={rec.get('stagea_ok', '-')} fails={fails} t={rec['t_s']}s", flush=True)

    total = time.monotonic() - t0
    summary = {"iterations": it, "fails": fails, "wall_s": round(total, 1),
               "zero_failure": fails == 0, "budget_s": args.budget_s}
    (OUT / "soak_summary.json").write_text(json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[soak] DONE it={it} fails={fails} wall={total:.0f}s zero_failure={fails == 0}")
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
