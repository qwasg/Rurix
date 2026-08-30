#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Day 0829 真实感战役 Phase 0:开工三锚复验 + Stage A 18 格 + full 基线帧时。

GPU 锁内顺序(任一步失败停跑,不带伤开工):
  alloff     window all-off 8f  == 55e4a92d(跨重建稳定锚)
  bench      bench 默认 160f    == c1d28ad7(Stage A 冻结锚 bistro t100 tsr 格)
  full       --quality full 96f == de342586(γ2.5 现行锚,f7 口径 g25_96_ev)
             + 记录 real_render_frame_ms 基线(90fps=11.11ms 预算分母)
  stagea18   6 tsr 格隔离单跑先行 + 12 vendor 格批跑(e3_stagea18 测序纪律)
  assets     γ2.5 烘焙件 4 PNG 在位检查

env:RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1;移除 RURIX_G18_AMBIENT。
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
A = ROOT / "artifacts" / "day_0829_realism" / "anchors"
EV = A / "ev"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_BENCH = "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"
ANCHOR_FULL = "sha256:de342586e452b903a2df7b744b9f67ad5b95b6bc5e3c17e0257def516ffc7211"
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


LOG = open(A / "p0_log.jsonl", "a", encoding="utf-8")
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


def run_cell(scene: str, tier: str, backend: str) -> dict:
    key = f"{scene}_t{tier}_{backend}"
    anchor = STAGEA[key]["last_frame_digest"]
    argv = [str(BENCH), "--bench", "--scene", scene, "--tier", tier,
            "--backend", backend, "--frames", "160", "--warmup", "10",
            "--out-root", str(A / "stagea18")]
    t0 = time.time()
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=1200, env=env_of())
    wall = time.time() - t0
    receipt = A / "stagea18" / scene / f"tier{tier}" / backend / "bench_receipt.json"
    got = None
    if r.returncode == 0 and receipt.is_file():
        got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
    ok = got == anchor
    vuid = (r.stderr or "").count("VUID-")
    print(f"[{key}] rc={r.returncode} {'MATCH' if ok else 'DRIFT/ERR'} vuid={vuid} ({wall:.0f}s)",
          flush=True)
    return {"cell": key, "fresh": got, "anchor": anchor, "match": ok,
            "rc": r.returncode, "vuid_hits": vuid, "wall_s": round(wall, 1)}


def main() -> int:
    EV.mkdir(parents=True, exist_ok=True)
    fails = 0
    stagea_rows: list[dict] = []
    with gpu_device_lock(purpose="day0829 realism Phase 0 anchors", timeout_s=7200.0):
        # ── ① window all-off 8f 锚 ──
        ok, got, row = run_win("p0_alloff_8f", [], 8)
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
                 "--out-root", str(A / "bench_default")],
                cwd=ROOT, capture_output=True, text=True, timeout=1800, env=env_of())
            receipt = (A / "bench_default" / "bistro-interior" / "tier100"
                       / "tsr_device" / "bench_receipt.json")
            got = None
            if rp.returncode == 0 and receipt.is_file():
                got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
            row = {"step": "p0_bench_default_160f", "rc": rp.returncode, "digest": got,
                   "expect": ANCHOR_BENCH, "wall_s": round(time.time() - t0, 1),
                   "pass": got == ANCHOR_BENCH}
            if not row["pass"]:
                row["stderr_tail"] = (rp.stderr or "").strip().splitlines()[-6:]
                fails += 1
            rec(row)
        # ── ③ --quality full 96f 现行锚(γ2.5,f7 口径)+ 基线帧时 ──
        if fails == 0:
            ok, got, row = run_win("p0_full_96f", ["--quality", "full"], 96)
            row["expect"] = ANCHOR_FULL
            row["pass"] = ok and got == ANCHOR_FULL
            fails += 0 if row["pass"] else 1
            rec(row)
        # ── ④ Stage A 18 格(6 tsr 隔离单跑先行 + 12 vendor 批跑)──
        if fails == 0:
            print("== Stage A 1/2:6 tsr 格隔离单跑 ==", flush=True)
            for cell in TSR:
                stagea_rows.append(run_cell(*cell))
            print("== Stage A 2/2:12 vendor 格批跑 ==", flush=True)
            for cell in VENDOR:
                stagea_rows.append(run_cell(*cell))
            matched = sum(1 for x in stagea_rows if x["match"])
            rec({"step": "p0_stagea18", "matched": matched, "total": len(stagea_rows),
                 "pass": matched == len(stagea_rows)})
            fails += 0 if matched == len(stagea_rows) else 1
    # ── ⑤ 资产在位(γ2.5 烘焙件 = 4 件 rgba8bin + manifest.json——Phase F
    #    烘焙侧车真实形状;HANDOVER「4 张 PNG」为口语,如实修正)──
    baked = ROOT / "artifacts" / "day_0828" / "f_emissive" / "baked"
    bins = sorted(p.name for p in baked.glob("*.rgba8bin")) if baked.is_dir() else []
    row = {"step": "p0_assets_baked", "baked_dir": str(baked), "rgba8bins": bins,
           "manifest": (baked / "manifest.json").is_file(),
           "pass": len(bins) >= 4 and (baked / "manifest.json").is_file()}
    fails += 0 if row["pass"] else 1
    rec(row)
    (A / "P0_SUMMARY.json").write_text(
        json.dumps({"schema": "rurix.day0829.realism.p0_anchors.v1", "fails": fails,
                    "rows": RESULTS, "stagea18": stagea_rows},
                   ensure_ascii=False, indent=1) + "\n",
        encoding="utf-8")
    print("P0", json.dumps({"fails": fails}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
