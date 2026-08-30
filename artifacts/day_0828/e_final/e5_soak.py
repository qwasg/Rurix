#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase E1 工作项 5：--quality full 窗口 soak ≥1800s 零失败。

模式照 artifacts/night_0828/soak：GPU 锁内循环迭代——每迭代 = 窗口
--quality full 真跑（32f 静态契约相机）校验 presented digest == 锚
6bd3af63（位级恒值）；每 3 迭代插一次 Stage A 单格探针（bistro t100
tsr bench 160f == c1d28ad7）。任一失败立即停跑保留现场。
落 soak_log.jsonl + soak_summary.json（迭代数/总秒数/零失败/digest 恒值）。
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
E = ROOT / "artifacts" / "day_0828" / "e_final"
SOAK = E / "soak"
ANCHOR_WIN = "sha256:9e5f6300445748af6cdb9e732c7f35eca7dbe66bbc829a67ca691151d7d12df1"
ANCHOR_STAGEA = "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"
BUDGET_S = 1800.0
PROBE_EVERY = 3


def base_env() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)  # 预设自注入面为 soak 对象
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def main() -> int:
    SOAK.mkdir(parents=True, exist_ok=True)
    log = open(SOAK / "soak_log.jsonl", "a", encoding="utf-8")
    t_start = time.time()
    it = 0
    fails = 0
    digests: set[str] = set()
    stagea_runs = 0
    fail_reason = None
    with gpu_device_lock(purpose="day0828 Phase E1 --quality full soak ≥1800s", timeout_s=7200.0):
        while time.time() - t_start < BUDGET_S and fails == 0:
            it += 1
            ev = SOAK / f"win_it{it}.json"
            t0 = time.time()
            r = subprocess.run(
                [str(WIN), "--frames", "32", "--warmup", "2", "--hidden",
                 "--quality", "full", "--evidence", str(ev)],
                cwd=ROOT, capture_output=True, text=True, timeout=1800, env=base_env())
            wall = time.time() - t0
            got = None
            if r.returncode == 0 and ev.is_file():
                got = json.loads(ev.read_text(encoding="utf-8")).get("digest")
            vuid = (r.stderr or "").count("VUID-")
            ok = r.returncode == 0 and got == ANCHOR_WIN and vuid == 0
            if not ok:
                fails += 1
                fail_reason = {"it": it, "rc": r.returncode, "got": got, "vuid": vuid,
                               "stderr_tail": (r.stderr or "").strip().splitlines()[-5:]}
            if got:
                digests.add(got)
            rec = {"it": it, "t_s": round(time.time() - t_start, 1), "win_ok": ok,
                   "win_digest": got, "win_digest_stable": got == ANCHOR_WIN,
                   "vuid_hits": vuid, "win_wall_s": round(wall, 1)}
            if ok and it % PROBE_EVERY == 0 and time.time() - t_start < BUDGET_S:
                t1 = time.time()
                rp = subprocess.run(
                    [str(BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
                     "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
                     "--out-root", str(SOAK / "stagea")],
                    cwd=ROOT, capture_output=True, text=True, timeout=1200, env=base_env())
                receipt = SOAK / "stagea" / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
                pgot = None
                if rp.returncode == 0 and receipt.is_file():
                    pgot = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
                pok = pgot == ANCHOR_STAGEA
                stagea_runs += 1
                if not pok:
                    fails += 1
                    fail_reason = {"it": it, "probe_rc": rp.returncode, "probe_got": pgot}
                rec["stagea_ok"] = pok
                rec["stagea_wall_s"] = round(time.time() - t1, 1)
            log.write(json.dumps(rec, ensure_ascii=False) + "\n")
            log.flush()
            print(json.dumps(rec, ensure_ascii=False), flush=True)
    wall_total = time.time() - t_start
    summary = {"schema": "rurix.day0828.e1.quality_full_soak.v1",
               "arm": "--quality full 窗口 32f 静态（九臂预设展开）",
               "iterations": it, "stagea_probes": stagea_runs, "fails": fails,
               "wall_s": round(wall_total, 1), "budget_s": BUDGET_S,
               "zero_failure": fails == 0,
               "digest_constant": sorted(digests) == [ANCHOR_WIN],
               "anchor": ANCHOR_WIN, "fail_reason": fail_reason}
    (SOAK / "soak_summary.json").write_text(
        json.dumps(summary, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print("SOAK", json.dumps({k: summary[k] for k in
          ("iterations", "stagea_probes", "fails", "wall_s", "zero_failure", "digest_constant")}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
