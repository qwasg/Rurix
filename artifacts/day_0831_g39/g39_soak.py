#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G39 收役 soak ≥1800s @缺省面(g38_soak.py 同构;锚断言钉死非自举)。

差异(G38 版 → G39 版):
  1. 产物隔离 artifacts/day_0831_g39/soak/。
  2. 32f 迭代锚**钉死** = G38 W6_FULL_SOAK anchor_32f 在案值(a8204b3b…)——
     本役全加性 off 缺省,缺省面 32f 口径位级恒值是硬预期,自举改断言。
  3. 前置双锚断言(锁内):all-off 8f == 55e4a92d… + full19 96f == a5521e47…
     (收役「all-off/full19 锚位级恒值」字面承载)。
  4. Stage A 单格探针 it4/it9 不变;帧时对 11.111ms 预算记账不变;VUID=0 门不变。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
BENCH = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"
OUT = ROOT / "artifacts" / "day_0831_g39" / "soak"
EV = OUT / "soak_ev"
MIN_WALL_S = 1800.0
BUDGET_MS = 11.111

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_FULL19 = "sha256:a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1"
# G38 W6_FULL_SOAK.json anchor_32f 在案值(缺省面 32f/warmup2 口径)。
ANCHOR_32F = "sha256:a8204b3b93845f557656231e5b3e2407bcbda030857d8e0d87ace0b48b32ac09"

STAGEA = json.loads(
    (ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json").read_text(encoding="utf-8")
)["anchors"]


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def win_run(tag: str, extra: list[str], frames: int) -> tuple[dict, str | None, int, float]:
    ev = EV / f"{tag}.json"
    r = subprocess.run(
        [str(WIN), "--frames", str(frames), "--warmup", "2", "--hidden",
         *extra, "--evidence", str(ev)],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8",
        errors="replace", timeout=2400, env=env_of())
    vuid = (r.stderr or "").count("VUID-")
    d = None
    fms = None
    if r.returncode == 0 and ev.is_file():
        doc = json.loads(ev.read_text(encoding="utf-8"))
        d = doc.get("digest")
        fms = doc.get("real_render_frame_ms")
    row = {"tag": tag, "rc": r.returncode, "vuid": vuid, "digest": d, "frame_ms": fms}
    if r.returncode != 0 or vuid != 0:
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-6:]
    return row, d, vuid, (fms if isinstance(fms, (int, float)) else 0.0)


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
    frame_ms_max = 0.0
    t_start = time.time()
    it = 0
    with gpu_device_lock(purpose="G39 收役 soak >=1800s(锚钉死)", timeout_s=4 * 3600.0):
        # 前置双锚断言
        row, d, vuid, _ = win_run("pre_alloff_8f", ["--quality", "off"], 8)
        row["anchor_match"] = d == ANCHOR_ALLOFF
        rows.append(row)
        print(json.dumps(row, ensure_ascii=False), flush=True)
        if not row["anchor_match"] or vuid != 0:
            fails += 1
        row, d, vuid, fms = win_run("pre_full19_96f", [], 96)
        row["anchor_match"] = d == ANCHOR_FULL19
        rows.append(row)
        print(json.dumps(row, ensure_ascii=False), flush=True)
        if not row["anchor_match"] or vuid != 0:
            fails += 1
        frame_ms_max = max(frame_ms_max, fms)
        if fails:
            print("SOAK ABORT: 前置锚断言红")
        else:
            while time.time() - t_start < MIN_WALL_S:
                it += 1
                row, d, vuid, fms = win_run(f"it{it:03d}", [], 32)
                stable = d == ANCHOR_32F
                frame_ms_max = max(frame_ms_max, fms)
                row.update({"it": it, "digest_stable": stable,
                            "wall_s": round(time.time() - t_start, 1)})
                if not (row["rc"] == 0 and vuid == 0 and stable):
                    fails += 1
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
        "schema": "rurix.day0831.g39.soak.v1",
        "caliber": "缺省面(=full19)32f/warmup2 迭代锚钉死 a8204b3b(G38 在案)+ 前置 all-off/full19 双锚断言 + Stage A 探针 it4/it9",
        "iterations": it,
        "wall_s": round(wall, 1),
        "min_wall_s": MIN_WALL_S,
        "fails": fails,
        "anchor_32f": ANCHOR_32F,
        "frame_ms_max": frame_ms_max,
        "budget_ms": BUDGET_MS,
        "frame_ms_within_budget": frame_ms_max <= BUDGET_MS,
        "verdict": "PASS" if (fails == 0 and wall >= MIN_WALL_S) else "FAIL",
        "rows": rows,
    }
    (OUT / "G39_SOAK.json").write_text(
        json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("G39SOAK", json.dumps({"verdict": doc["verdict"], "iterations": it,
                                 "wall_s": doc["wall_s"], "frame_ms_max": frame_ms_max},
                                ensure_ascii=False))
    return 0 if doc["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
