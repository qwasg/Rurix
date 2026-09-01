#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G39 T5 段 2 GPU 验收批(C1-C5;REPORT.md §4 清单的执行器,主 agent 锁内)。

判据:
  C1 device 臂 rc=0(内建判定码逐项全等×16帧×双跑 fail-closed)+ stderr 登记行
     + evidence device 字段非 null。
  C2 host 臂 rc=0 + digest 序列 dev==host 逐帧逐字节(硬门)
     + 对照 G38 t3_incr.json(跨窗参考锚;异则先查驱动/build,如实登记)。
  C3 red-arm rc≠0 + stderr 含「red-arm 模式」与「判定码 mismatch」。
  C4 device × --min-level 1 rc=0(自洽,不与 ml0 比)。
  C5 缺省面零新旗标 rc=0 + digest 序列 == t5_host(0-byte 回归)。
产物:t5_devicecut/ev/*.json + B5_SUMMARY.json + b5_log.jsonl。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
T5 = ROOT / "artifacts" / "day_0831_g39" / "t5_devicecut"
EV = T5 / "ev"
LOG = T5 / "b5_log.jsonl"
FCP = ROOT / "target-night" / "release" / "g31_frame_cut_probe.exe"
KSPV = ROOT / ".tmp" / "g39_gates" / "t5_devicecut" / "g31_cluster_cull.spv"
T3_INCR = ROOT / "artifacts" / "day_0830_g38" / "t3_framecut" / "ev" / "t3_incr.json"
BASE = ["--cluster-pack", ".tmp/g36_gates/wave1_geo_composition/bistro.rxcp",
        "--error-px", "2.0", "--frames", "16", "--step-m", "0.15", "--res", "96x54"]

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

FAILS: list[str] = []
NOTES: list[str] = []


def log(step: str, **kw) -> None:
    rec = {"t": time.strftime("%H:%M:%S"), "step": step, **kw}
    LOG.parent.mkdir(parents=True, exist_ok=True)
    with LOG.open("a", encoding="utf-8") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(f"[{rec['t']}] {step}: {kw.get('status', '')}", flush=True)


def run(step: str, extra: list[str], expect_rc0: bool = True,
        stderr_must: list[str] | None = None) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    t0 = time.time()
    p = subprocess.run([str(FCP), *BASE, *extra], cwd=str(ROOT), capture_output=True,
                       text=True, encoding="utf-8", errors="replace", timeout=1800, env=env)
    dt = round(time.time() - t0, 1)
    vuid = (p.stderr or "").count("VUID-")
    rc_ok = (p.returncode == 0) if expect_rc0 else (p.returncode != 0)
    miss = [s for s in (stderr_must or []) if s not in (p.stderr or "")]
    ok = rc_ok and not miss and (vuid == 0 if expect_rc0 else True)
    log(step, status="OK" if ok else f"FAIL rc={p.returncode} vuid={vuid} miss={miss}",
        wall_s=dt, stderr_tail=(p.stderr or "")[-1500:] if not ok else (p.stderr or "")[-400:])
    if not ok:
        FAILS.append(step)
    return p


def digests(p: Path) -> list[str] | None:
    if not p.is_file():
        return None
    return [f["digest"] for f in json.loads(p.read_text(encoding="utf-8"))["frames_data"]]


def main() -> int:
    for f in (FCP, KSPV):
        if not f.exists():
            print(f"FAIL: 缺件 {f}")
            return 2
    EV.mkdir(parents=True, exist_ok=True)
    with gpu_device_lock(purpose="G39 T5 段2 验收批(C1-C5)"):
        run("C1_device", ["--cut-source", "device", "--cull-spv", str(KSPV),
                          "--evidence", str(EV / "t5_dev.json")],
            stderr_must=["cut_source=device", "device cut 对拍臂就绪"])
        run("C2_host", ["--evidence", str(EV / "t5_host.json")],
            stderr_must=["cut_source=host"])
        if not FAILS:
            da, db = digests(EV / "t5_dev.json"), digests(EV / "t5_host.json")
            ok = da is not None and da == db and len(da) == 16
            log("C2_judge_dev_eq_host", status="OK" if ok else "FAIL", n=len(da or []))
            if not ok:
                FAILS.append("C2.dev_eq_host")
            dt3 = digests(T3_INCR)
            cross = (da == dt3) if (da and dt3) else None
            log("C2_cross_g38_t3incr", status="OK" if cross else "NOTE",
                match=cross,
                note=None if cross else "跨窗参考锚不等——先查驱动/build 面,不误红本臂(REPORT §4 C2 口径)")
            if cross is False:
                NOTES.append("t5_host digest 序列 != G38 t3_incr(跨窗参考锚):待归因登记")
        # C3 red-arm(预期进程死,evidence 不落盘)
        p = run("C3_redarm", ["--cut-source", "device", "--cull-spv", str(KSPV),
                              "--cut-red-arm", "tamper",
                              "--evidence", str(EV / "t5_red.json")],
                expect_rc0=False, stderr_must=["red-arm 模式", "判定码 mismatch"])
        run("C4_device_ml1", ["--cut-source", "device", "--cull-spv", str(KSPV),
                              "--min-level", "1",
                              "--evidence", str(EV / "t5_dev_ml1.json")])
        run("C5_default", ["--evidence", str(EV / "t5_default.json")],
            stderr_must=["cut_source=host"])
        if not any(f.startswith("C5") for f in FAILS):
            d5, dh = digests(EV / "t5_default.json"), digests(EV / "t5_host.json")
            ok = d5 is not None and d5 == dh
            log("C5_judge_default_eq_host", status="OK" if ok else "FAIL")
            if not ok:
                FAILS.append("C5.default_eq_host")
    # device 证据税记账(单列不判读)
    dev = EV / "t5_dev.json"
    probe_ms = None
    if dev.is_file():
        fr = json.loads(dev.read_text(encoding="utf-8"))["frames_data"]
        vals = [f.get("device_cut_probe_ms") for f in fr
                if isinstance(f.get("device_cut_probe_ms"), (int, float))]
        probe_ms = round(sum(vals) / len(vals), 3) if vals else None
    (T5 / "B5_SUMMARY.json").write_text(json.dumps({
        "schema": "rurix.day0831.g39.b5.v1",
        "fails": FAILS, "notes": NOTES,
        "device_cut_probe_ms_mean": probe_ms,
        "verdict": "PASS" if not FAILS else "FAIL",
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(("B5 PASS" if not FAILS else f"B5 FAILS: {FAILS}")
          + (f" NOTES: {NOTES}" if NOTES else "") + f" device_probe_ms={probe_ms}")
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
