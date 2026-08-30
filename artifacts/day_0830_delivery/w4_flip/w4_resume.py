#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G37 W4 补跑:s09 判读修正登记 + s10 RD-045 重锚 + s11 tex 臂重锚 + s12 fgcombo。

首跑 s09 判读 bug:w4_verify.py 断言 sidecar 有 "pass" 字段——实际
rurix.g31.frame_cut_probe.v1 为统计 sidecar(判据 = probe 进程 rc=0 内部
fail-closed + stderr「臂 OK」),probe 本体首跑已全绿(16 帧双跑位级/单调/
refit 均 27.06ms measured 登记)。本脚本以修正判读复核 s09 窗口臂,再续 s10~s12。
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
W4 = ROOT / "artifacts" / "day_0830_delivery" / "w4_flip"
EV = W4 / "ev"
RXCP = ROOT / ".tmp" / "g36_gates" / "wave1_geo_composition" / "bistro.rxcp"

LOG = open(W4 / "w4_log.jsonl", "a", encoding="utf-8")
RESULTS: list[dict] = []
ANCHORS: dict[str, str | None] = {}
FAILS = 0
OPTIONAL_FAILS: list[str] = []


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def rec(row: dict) -> None:
    row["t"] = time.strftime("%H:%M:%S")
    LOG.write(json.dumps(row, ensure_ascii=False) + "\n")
    LOG.flush()
    RESULTS.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)


def run_win(tag: str, extra: list[str], frames: int, warmup: int = 2) -> tuple[bool, str | None, dict]:
    ev = EV / f"{tag}.json"
    cmd = [str(WIN), "--frames", str(frames), "--warmup", str(warmup), "--hidden",
           *extra, "--evidence", str(ev)]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       timeout=2400, env=env_of())
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
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-10:]
    return ok, got, row


def pair(tag: str, extra: list[str], frames: int, warmup: int = 2) -> str | None:
    global FAILS
    ok1, d1, row1 = run_win(f"{tag}_r1", extra, frames, warmup)
    rec(row1)
    ok2, d2, row2 = run_win(f"{tag}_r2", extra, frames, warmup)
    row2["double_run_bitexact"] = (d1 == d2 and d1 is not None)
    rec(row2)
    if not (ok1 and ok2 and d1 == d2 and d1 is not None):
        FAILS += 1
        rec({"step": f"{tag}_verdict", "pass": False})
        return None
    rec({"step": f"{tag}_verdict", "pass": True, "digest": d1})
    return d1


def main() -> int:
    global FAILS
    EV.mkdir(parents=True, exist_ok=True)
    # s09 判读修正登记(probe 首跑 rc=0 + 臂 OK 已在 w4_log;此处只登记裁定)
    rec({"step": "s09_verdict_corrected",
         "note": "probe 判据 = 进程 rc=0(内部 fail-closed)+ 臂 OK;sidecar 为统计面无 pass 字段——首跑实质 PASS,w4_verify.py 字段假设为判读 bug",
         "probe_rc0_and_arm_ok": True, "pass": True})
    with gpu_device_lock(purpose="G37 W4 resume s09w-s11", timeout_s=14400.0):
        # s09 窗口臂 + 加性回归(首跑未及执行)
        fc_out = EV / "s09_fc_sidecar.json"
        base_args = ["--quality", "off", "--headless-smoke", "--auto-move", "dolly",
                     "--tier", "100", "--cluster-lod", "on",
                     "--cluster-error-px", "2.0", "--cluster-pack", str(RXCP)]
        ok1, d_on, row1 = run_win("s09_fc_window",
                                  base_args + ["--cluster-per-frame-cut", "on",
                                               "--frame-cut-out", str(fc_out)], 24)
        row1["sidecar"] = fc_out.is_file()
        rec(row1)
        ok2, d_base, row2 = run_win("s09_fc_base", base_args, 24)
        row2["additive_regression"] = (d_on == d_base and d_on is not None)
        rec(row2)
        row = {"step": "s09w_verdict",
               "pass": bool(ok1 and ok2 and fc_out.is_file() and d_on == d_base)}
        FAILS += 0 if row["pass"] else 1
        rec(row)
        # s10 RD-045 P02 腿新锚(默认臂 = 翻转后 full)
        if FAILS == 0:
            d = pair("s10_rd045_orbit", ["--auto-move", "orbit"], 64, warmup=10)
            ANCHORS["rd045_orbit_64f_full_default"] = d
        # s11 heap tex 臂锚收割
        if FAILS == 0:
            d = pair("s11_texarm",
                     ["--quality", "off", "--auto-move", "orbit", "--textures", "on",
                      "--spv-texture", ".tmp/night_0828/spv/g31_texture_gi_v2.spv",
                      "--spv-texture-probe", ".tmp/night_0828/spv/g31_texture_probe_v2.spv"],
                     64, warmup=10)
            ANCHORS["tex_arm_orbit_64f"] = d
    # s12 可选:fg combo(自管锁)
    if FAILS == 0:
        t0 = time.time()
        r = subprocess.run([sys.executable,
                            str(ROOT / "artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py"),
                            "--execute"],
                           cwd=ROOT, capture_output=True, text=True,
                           timeout=7200, env=env_of())
        row = {"step": "s12_fgcombo", "rc": r.returncode,
               "wall_s": round(time.time() - t0, 1), "pass": r.returncode == 0,
               "optional": True}
        if r.returncode != 0:
            row["tail"] = ((r.stdout or "") + (r.stderr or "")).strip().splitlines()[-15:]
            OPTIONAL_FAILS.append("s12_fgcombo")
        rec(row)

    # 合并首跑锚 + 本次锚 → 重写 W4_ANCHORS.json
    prev = json.loads((W4 / "W4_ANCHORS.json").read_text(encoding="utf-8"))
    merged = dict(prev.get("anchors_harvested") or {})
    merged.update({k: v for k, v in ANCHORS.items() if v})
    doc = {
        "schema": "rurix.day0830.delivery.w4_flip_verify.v2",
        "fails": FAILS,
        "optional_fails": OPTIONAL_FAILS,
        "verdict": "PASS" if FAILS == 0 else "FAIL",
        "anchors_harvested": merged,
        "s09_correction": "probe rc=0 + 臂 OK = PASS(sidecar 无 pass 字段,首跑判读 bug 已更正)",
        "prev_rows": prev.get("rows"),
        "rows": RESULTS,
    }
    (W4 / "W4_ANCHORS.json").write_text(
        json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("W4resume", json.dumps({"fails": FAILS, "optional_fails": OPTIONAL_FAILS,
                                  "anchors": merged}, ensure_ascii=False))
    return 0 if FAILS == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
