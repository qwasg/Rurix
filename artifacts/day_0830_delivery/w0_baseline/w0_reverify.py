#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G37 W0:开役基线定盘(GPU 锁内,任一步失败停跑,只验不收割)。

序列:
  alloff      all-off 8f == 55e4a92d(跨重建稳定锚)
  full16      --quality full(十六臂)96f == 5db2e7d7(day_0829 F2 现锚)
  bench       bench 默认 160f == c1d28ad7(bench 面永不动)
  stagea18    6 tsr 隔离单跑 + 12 vendor 批跑(全 18 格锚)
本脚本 = day_0829 f2_reanchor.py 同构复验形态;落盘 day_0830_delivery,零覆盖前役交付件。
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
W0 = ROOT / "artifacts" / "day_0830_delivery" / "w0_baseline"
EV = W0 / "ev"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_BENCH = "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"
ANCHOR_FULL16 = "sha256:5db2e7d72e6b4f3c961d1acdd48d05c60df24e8803a26f4dfdb37665b79bf673"
STAGEA = json.loads(
    (ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json").read_text(encoding="utf-8")
)["anchors"]

SCENES = ["cornell-box", "bistro-interior"]
TIERS = ["50", "67", "100"]
TSR = [(s, t, "tsr_device") for s in SCENES for t in TIERS]
VENDOR = [(s, t, b) for s in SCENES for t in TIERS for b in ("dlss_sr", "fsr_3_1_5")]


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


LOG = open(W0 / "w0_log.jsonl", "a", encoding="utf-8")
RESULTS: list[dict] = []


def rec(row: dict) -> None:
    row["t"] = time.strftime("%H:%M:%S")
    LOG.write(json.dumps(row, ensure_ascii=False) + "\n")
    LOG.flush()
    RESULTS.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)


def run_win(tag: str, extra: list[str], frames: int, expect: str) -> bool:
    ev = EV / f"{tag}.json"
    cmd = [str(WIN), "--frames", str(frames), "--warmup", "2", "--hidden",
           *extra, "--evidence", str(ev)]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       timeout=1800, env=env_of())
    wall = time.time() - t0
    got = None
    evd: dict = {}
    if r.returncode == 0 and ev.is_file():
        evd = json.loads(ev.read_text(encoding="utf-8"))
        got = evd.get("digest")
    vuid = (r.stderr or "").count("VUID-")
    ok = r.returncode == 0 and vuid == 0 and got == expect
    row = {"step": tag, "rc": r.returncode, "digest": got, "expect": expect,
           "vuid": vuid, "wall_s": round(wall, 1), "pass": ok,
           "real_render_frame_ms": evd.get("real_render_frame_ms")}
    if not ok:
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-8:]
    rec(row)
    return ok


def run_cell(scene: str, tier: str, backend: str) -> dict:
    key = f"{scene}_t{tier}_{backend}"
    anchor = STAGEA[key]["last_frame_digest"]
    argv = [str(BENCH), "--bench", "--scene", scene, "--tier", tier,
            "--backend", backend, "--frames", "160", "--warmup", "10",
            "--out-root", str(W0 / "stagea18")]
    t0 = time.time()
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=1200, env=env_of())
    wall = time.time() - t0
    receipt = W0 / "stagea18" / scene / f"tier{tier}" / backend / "bench_receipt.json"
    got = None
    if r.returncode == 0 and receipt.is_file():
        got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
    ok = got == anchor
    print(f"[{key}] rc={r.returncode} {'MATCH' if ok else 'DRIFT/ERR'} ({wall:.0f}s)", flush=True)
    return {"cell": key, "fresh": got, "anchor": anchor, "match": ok,
            "rc": r.returncode, "wall_s": round(wall, 1)}


def main() -> int:
    EV.mkdir(parents=True, exist_ok=True)
    fails = 0
    stagea_rows: list[dict] = []
    with gpu_device_lock(purpose="G37 W0 baseline reverify", timeout_s=10800.0):
        # W4 默认翻转免疫:alloff 步语义 = 显式 off（55e4a92d 跨重建稳定锚,DEFAULT_FLIP_PLAN §2.5）。
        if not run_win("w0_alloff_8f", ["--quality", "off"], 8, ANCHOR_ALLOFF):
            fails += 1
        if fails == 0 and not run_win("w0_full16_96f", ["--quality", "full"], 96, ANCHOR_FULL16):
            fails += 1
        if fails == 0:
            t0 = time.time()
            rp = subprocess.run(
                [str(BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
                 "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
                 "--out-root", str(W0 / "bench_default")],
                cwd=ROOT, capture_output=True, text=True, timeout=1800, env=env_of())
            receipt = (W0 / "bench_default" / "bistro-interior" / "tier100"
                       / "tsr_device" / "bench_receipt.json")
            got = None
            if rp.returncode == 0 and receipt.is_file():
                got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
            row = {"step": "w0_bench_default_160f", "rc": rp.returncode, "digest": got,
                   "expect": ANCHOR_BENCH, "wall_s": round(time.time() - t0, 1),
                   "pass": got == ANCHOR_BENCH}
            fails += 0 if row["pass"] else 1
            rec(row)
        if fails == 0:
            print("== Stage A 1/2:6 tsr 格隔离单跑 ==", flush=True)
            for cell in TSR:
                stagea_rows.append(run_cell(*cell))
            print("== Stage A 2/2:12 vendor 格批跑 ==", flush=True)
            for cell in VENDOR:
                stagea_rows.append(run_cell(*cell))
            matched = sum(1 for x in stagea_rows if x["match"])
            rec({"step": "w0_stagea18", "matched": matched, "total": len(stagea_rows),
                 "pass": matched == len(stagea_rows)})
            fails += 0 if matched == len(stagea_rows) else 1
    doc = {
        "schema": "rurix.day0830.delivery.w0_baseline.v1",
        "fails": fails,
        "verdict": "PASS" if fails == 0 else "FAIL",
        "anchors": {"alloff": ANCHOR_ALLOFF, "full16": ANCHOR_FULL16, "bench": ANCHOR_BENCH},
        "rows": RESULTS,
        "stagea18": stagea_rows,
    }
    (W0 / "W0_BASELINE.json").write_text(
        json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("W0", json.dumps({"fails": fails}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
