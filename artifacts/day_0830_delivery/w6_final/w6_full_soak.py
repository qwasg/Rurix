#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G37 W6:@新默认(full 十九臂)soak ≥1800s(DEFAULT_FLIP_PLAN §2.5 字面承载)。

形态 = day_0829 f3_storm_soak 同构:32f 迭代循环,首迭代自举 32f 口径锚,
后续逐迭代 digest ==(位级恒值);穿插 Stage A 单格探针 ×2;全程 VUID=0;
帧时逐迭代记账(90fps 预算 11.11ms 对照)。风暴腿由 W4 s07(full×storm3
pso_runtime_creates=0)在案承载,本脚本不重复。
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
W6 = ROOT / "artifacts" / "day_0830_delivery" / "w6_final"
EV = W6 / "soak_ev"
MIN_WALL_S = 1800.0
BUDGET_MS = 11.111

STAGEA = json.loads(
    (ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json").read_text(encoding="utf-8")
)["anchors"]


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def stage_a_probe(tag: str) -> dict:
    key = "bistro-interior_t100_tsr_device"
    anchor = STAGEA[key]["last_frame_digest"]
    r = subprocess.run(
        [str(BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
         "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
         "--out-root", str(EV / f"stagea_{tag}")],
        cwd=ROOT, capture_output=True, text=True, timeout=1200, env=env_of())
    receipt = EV / f"stagea_{tag}" / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
    got = None
    if r.returncode == 0 and receipt.is_file():
        got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
    return {"probe": tag, "rc": r.returncode, "match": got == anchor}


def main() -> int:
    EV.mkdir(parents=True, exist_ok=True)
    rows: list[dict] = []
    fails = 0
    anchor: str | None = None
    frame_ms_max = 0.0
    t_start = time.time()
    it = 0
    with gpu_device_lock(purpose="G37 W6 full19 soak >=1800s", timeout_s=4 * 3600.0):
        while time.time() - t_start < MIN_WALL_S:
            it += 1
            ev = EV / f"it{it:03d}.json"
            r = subprocess.run(
                [str(WIN), "--frames", "32", "--warmup", "2", "--hidden",
                 "--evidence", str(ev)],
                cwd=ROOT, capture_output=True, text=True, timeout=2400, env=env_of())
            vuid = (r.stderr or "").count("VUID-")
            d = None
            fms = None
            if r.returncode == 0 and ev.is_file():
                doc = json.loads(ev.read_text(encoding="utf-8"))
                d = doc.get("digest")
                fms = doc.get("real_render_frame_ms")
            ok = r.returncode == 0 and vuid == 0 and d is not None
            if anchor is None and ok:
                anchor = d
            stable = (d == anchor) if ok else False
            if isinstance(fms, (int, float)):
                frame_ms_max = max(frame_ms_max, float(fms))
            row = {"it": it, "rc": r.returncode, "vuid": vuid, "digest_stable": stable,
                   "frame_ms": fms, "wall_s": round(time.time() - t_start, 1)}
            if not (ok and stable):
                fails += 1
                row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-6:]
            rows.append(row)
            print(json.dumps(row, ensure_ascii=False), flush=True)
            if it in (4, 9):
                pr = stage_a_probe(f"it{it}")
                rows.append(pr)
                print(json.dumps(pr, ensure_ascii=False), flush=True)
                if not pr["match"]:
                    fails += 1
    wall = time.time() - t_start
    doc = {
        "schema": "rurix.day0830.delivery.w6_full_soak.v1",
        "caliber": "--quality 缺省(=full 十九臂)32f/warmup2 迭代;首迭代自举锚后续位级恒值;Stage A 单格探针 it4/it9",
        "iterations": it,
        "wall_s": round(wall, 1),
        "min_wall_s": MIN_WALL_S,
        "fails": fails,
        "anchor_32f": anchor,
        "frame_ms_max": frame_ms_max,
        "budget_ms": BUDGET_MS,
        "frame_ms_within_budget": frame_ms_max <= BUDGET_MS,
        "verdict": "PASS" if (fails == 0 and wall >= MIN_WALL_S) else "FAIL",
        "rows": rows,
    }
    (W6 / "W6_FULL_SOAK.json").write_text(
        json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("W6SOAK", json.dumps({"verdict": doc["verdict"], "iterations": it,
                                "wall_s": doc["wall_s"], "frame_ms_max": frame_ms_max},
                               ensure_ascii=False))
    return 0 if doc["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
