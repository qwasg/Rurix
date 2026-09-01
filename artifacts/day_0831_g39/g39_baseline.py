#!/usr/bin/env python3
"""G39 W0 开役基线复证(断言型,零收割——本役预期零重锚)。

链条(锁内,任一步 FAIL 即停役归 owner):
  n1 all-off 8f    == 55e4a92d…(G38_ANCHORS 全串)
  n2 bench 160f    == c1d28ad7…(bench_receipt last_frame_digest)
  n3 Stage A 单格  == stage_a 锚(n2 回执同一口径)
  n4 full19 96f    == a5521e47…(法线 v2 现锚)
产物:artifacts/day_0831_g39/w0_baseline/G39_BASELINE.json + baseline_log.jsonl。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
G39 = ROOT / "artifacts" / "day_0831_g39" / "w0_baseline"
EV = G39 / "ev"
LOG = G39 / "baseline_log.jsonl"
WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
PERF = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_FULL19 = "sha256:a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1"
ANCHOR_BENCH = "c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"
STAGE_A = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

FAILS: list[str] = []
ROWS: list[dict] = []


def rec(step: str, **kw) -> None:
    row = {"t": time.strftime("%H:%M:%S"), "step": step, **kw}
    ROWS.append(row)
    with LOG.open("a", encoding="utf-8") as f:
        f.write(json.dumps(row, ensure_ascii=False) + "\n")
    print(f"[{row['t']}] {step}: {kw.get('status', '')}", flush=True)


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def run_win(step: str, extra: list[str], frames: int, warmup: int = 2) -> str | None:
    ev_p = EV / f"{step}.json"
    cmd = [str(WIN), "--frames", str(frames), "--warmup", str(warmup), "--hidden",
           *extra, "--evidence", str(ev_p)]
    t0 = time.time()
    p = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                       encoding="utf-8", errors="replace", timeout=1800, env=env_of())
    vuid = (p.stderr or "").count("VUID-")
    d = None
    frame_ms = None
    if ev_p.exists():
        e = json.loads(ev_p.read_text(encoding="utf-8"))
        d = e.get("digest")
        frame_ms = e.get("real_render_frame_ms")
    ok = p.returncode == 0 and d is not None and vuid == 0
    rec(step, status="OK" if ok else f"FAIL rc={p.returncode}", digest=d,
        wall_s=round(time.time() - t0, 1), vuid=vuid, real_render_frame_ms=frame_ms,
        stderr_tail=None if ok else (p.stderr or "")[-800:])
    if not ok:
        FAILS.append(step)
    return d


def main() -> int:
    EV.mkdir(parents=True, exist_ok=True)
    for exe in (WIN, PERF):
        if not exe.exists():
            print(f"FAIL: 缺 exe {exe}(先 release 构建)")
            return 2
    with gpu_device_lock(purpose="G39 W0 开役基线复证(三锚断言)"):
        # n1 all-off
        d = run_win("n1_alloff_8f", ["--quality", "off"], 8)
        ok = d == ANCHOR_ALLOFF
        rec("n1_verdict", status="OK" if ok else "FAIL",
            got=(d or "")[:24], expect=ANCHOR_ALLOFF[:24])
        if not ok:
            FAILS.append("n1")
        # n2 bench 160f
        out_root = G39 / "bench_default"
        p = subprocess.run(
            [str(PERF), "--bench", "--scene", "bistro-interior", "--tier", "100",
             "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
             "--out-root", str(out_root)],
            cwd=str(ROOT), capture_output=True, text=True, encoding="utf-8",
            errors="replace", timeout=1800, env=env_of())
        rcp = out_root / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
        got = (json.loads(rcp.read_text(encoding="utf-8"))["last_frame_digest"]
               if rcp.exists() else None)
        ok = p.returncode == 0 and got is not None and got.endswith(ANCHOR_BENCH)
        rec("n2_bench_160f", status="OK" if ok else "FAIL", got=(got or "")[:24])
        if not ok:
            FAILS.append("n2")
        # n3 Stage A 单格(n2 回执同一口径)
        sa = json.loads(STAGE_A.read_text(encoding="utf-8"))["anchors"][
            "bistro-interior_t100_tsr_device"]["last_frame_digest"]
        ok = got is not None and got == sa
        rec("n3_stagea_probe", status="OK" if ok else "FAIL",
            note="bench 默认格 == Stage A bistro/t100/tsr 格同一口径回执")
        if not ok:
            FAILS.append("n3")
        # n4 full19
        d = run_win("n4_full19_96f", [], 96)
        ok = d == ANCHOR_FULL19
        rec("n4_verdict", status="OK" if ok else "FAIL",
            got=(d or "")[:24], expect=ANCHOR_FULL19[:24])
        if not ok:
            FAILS.append("n4")
    out = {
        "schema": "rurix.day0831.g39.baseline.v1",
        "fails": len(FAILS),
        "verdict": "PASS" if not FAILS else "FAIL",
        "anchors_asserted": {
            "alloff_8f": ANCHOR_ALLOFF,
            "full19_default_96f": ANCHOR_FULL19,
            "bench_160f_suffix": ANCHOR_BENCH,
        },
        "rows": ROWS,
    }
    (G39 / "G39_BASELINE.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(("BASELINE PASS" if not FAILS else f"BASELINE FAILS: {FAILS}"))
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
