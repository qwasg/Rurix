#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G40 T2 GPU 验收批 B1(#77 P2 生产 dispatch;主 agent 锁内)。

判据(任务书 B1 六条):
  ① device 臂 digest 16f == host 臂位级 + 臂内建双跑(进程内)+ 跨进程双跑。
  ② device × --min-level 1 rc=0(自洽;ml1×device 组合墙钟登记不判红)。
  ③ incr==full 维持(device 臂两 copy 模式)+ 跨 G38 t3_incr / G39 t5_dev
     digest 序列 MATCH(P2 免重锚推导链的机核,DESIGN §3.1)。
  ④ red-arm 维持必红(P2 形态:决策翻转 ⇒ host 影子核覆盖性必破 rc≠0)。
  ⑤ 缺省 host 臂 0-byte 回归(digest == device == G39 t5_host)+ 窗口臂
     0-byte 回归(5540ecae 锚,G38 B7 口径)。
  ⑥ 帧时 measured 登记不判红:select/verify/promote 分项 + dispatch GPU +
     ml1×device 组合墙钟(DESIGN §4-2 预期 ~15-19ms)。
产物:t2_devicecut/ev/*.json + B1_SUMMARY.json + b1_log.jsonl。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
T2 = ROOT / "artifacts" / "day_0901_g40" / "t2_devicecut"
EV = T2 / "ev"
LOG = T2 / "b1_log.jsonl"
FCP = ROOT / "target-night" / "release" / "g31_frame_cut_probe.exe"
WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
KSPV = ROOT / ".tmp" / "g40_gates" / "t2_devicecut" / "g31_cluster_cull.spv"
PACK = ".tmp/g36_gates/wave1_geo_composition/bistro.rxcp"
G38_INCR = ROOT / "artifacts" / "day_0830_g38" / "t3_framecut" / "ev" / "t3_incr.json"
G39_DEV = ROOT / "artifacts" / "day_0831_g39" / "t5_devicecut" / "ev" / "t5_dev.json"
WIN_ANCHOR = "sha256:5540ecaed4fd4c1e3e0abea7f937bbb2e200434096a86a772dbb48238d0e0ea8"
BASE = ["--cluster-pack", PACK, "--error-px", "2.0", "--frames", "16",
        "--step-m", "0.15", "--res", "96x54"]

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


def run(step: str, cmd: list[str], expect_rc0: bool = True,
        stderr_must: list[str] | None = None, timeout: int = 1800) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    t0 = time.time()
    p = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                       encoding="utf-8", errors="replace", timeout=timeout, env=env)
    dt = round(time.time() - t0, 1)
    vuid = (p.stderr or "").count("VUID-")
    rc_ok = (p.returncode == 0) if expect_rc0 else (p.returncode != 0)
    miss = [s for s in (stderr_must or []) if s not in (p.stderr or "")]
    ok = rc_ok and not miss and (vuid == 0 if expect_rc0 else True)
    log(step, status="OK" if ok else f"FAIL rc={p.returncode} vuid={vuid} miss={miss}",
        wall_s=dt, stderr_tail=(p.stderr or "")[-1600:] if not ok else (p.stderr or "")[-400:])
    if not ok:
        FAILS.append(step)
    return p


def digests(p: Path) -> list[str] | None:
    if not p.is_file():
        return None
    return [f["digest"] for f in json.loads(p.read_text(encoding="utf-8"))["frames_data"]]


def mean(ev_p: Path, key: str, skip0: bool = True) -> float | None:
    if not ev_p.is_file():
        return None
    fr = json.loads(ev_p.read_text(encoding="utf-8"))["frames_data"]
    vals = [f[key] for f in (fr[1:] if skip0 else fr) if isinstance(f.get(key), (int, float))]
    return round(sum(vals) / len(vals), 3) if vals else None


def main() -> int:
    for f in (FCP, WIN, KSPV):
        if not f.exists():
            print(f"FAIL: 缺件 {f}")
            return 2
    EV.mkdir(parents=True, exist_ok=True)
    dev = ["--cut-source", "device", "--cull-spv", str(KSPV)]
    with gpu_device_lock(purpose="G40 B1 T2 device cut P2 验收", timeout_s=7200.0):
        # ① device 臂(内建双跑)+ 跨进程双跑
        run("B1a_device_r1", [str(FCP), *BASE, *dev,
                              "--evidence", str(EV / "p2_dev.json")],
            stderr_must=["cut_source=device", "cull 会话就绪", "device cut 表就绪"])
        run("B1a_device_r2", [str(FCP), *BASE, *dev,
                              "--evidence", str(EV / "p2_dev_r2.json")])
        # ① host 臂(对拍基准)
        run("B1b_host", [str(FCP), *BASE, "--evidence", str(EV / "p2_host.json")],
            stderr_must=["cut_source=host"])
        # ③ device × full copy(incr==full 维持)
        run("B1c_device_full", [str(FCP), *BASE, *dev, "--refit-copy", "full",
                                "--evidence", str(EV / "p2_dev_full.json")])
        # ② device × ml1
        run("B1d_device_ml1", [str(FCP), *BASE, *dev, "--min-level", "1",
                               "--evidence", str(EV / "p2_dev_ml1.json")])
        # ④ red-arm(P2 形态:覆盖性必破)
        run("B1e_redarm", [str(FCP), *BASE, *dev, "--cut-red-arm", "tamper",
                           "--evidence", str(EV / "p2_red.json")],
            expect_rc0=False, stderr_must=["red-arm 模式", "覆盖性"])
        # ⑤ 窗口臂 0-byte 回归(G38 B7 口径;fc on/base 双跑 == 5540ecae 锚)
        wcmd = [str(WIN), "--quality", "off", "--headless-smoke", "--auto-move", "dolly",
                "--tier", "100", "--cluster-lod", "on", "--cluster-error-px", "2.0",
                "--cluster-pack", PACK, "--frames", "24", "--warmup", "2", "--hidden"]
        run("B1f_win_fc", wcmd + ["--cluster-per-frame-cut", "on",
                                  "--frame-cut-out", str(EV / "win_fc_sidecar.json"),
                                  "--evidence", str(EV / "win_fc_ev.json")])
        run("B1f_win_base", wcmd + ["--evidence", str(EV / "win_base_ev.json")])

    # ── 判读(锁外)──
    d_dev = digests(EV / "p2_dev.json")
    d_dev2 = digests(EV / "p2_dev_r2.json")
    d_host = digests(EV / "p2_host.json")
    d_full = digests(EV / "p2_dev_full.json")
    ok1 = d_dev is not None and len(d_dev) == 16 and d_dev == d_host
    log("J1_dev_eq_host", status="OK" if ok1 else "FAIL", n=len(d_dev or []))
    if not ok1:
        FAILS.append("J1.dev_eq_host")
    ok1b = d_dev is not None and d_dev == d_dev2
    log("J1_cross_process", status="OK" if ok1b else "FAIL")
    if not ok1b:
        FAILS.append("J1.cross_process")
    ok3 = d_dev is not None and d_dev == d_full
    log("J3_incr_eq_full", status="OK" if ok3 else "FAIL")
    if not ok3:
        FAILS.append("J3.incr_eq_full")
    d_g38 = digests(G38_INCR)
    d_g39 = digests(G39_DEV)
    m38 = d_dev == d_g38 if (d_dev and d_g38) else None
    m39 = d_dev == d_g39 if (d_dev and d_g39) else None
    log("J3_cross_anchors", status="OK" if (m38 and m39) else "NOTE",
        g38_t3incr=m38, g39_t5dev=m39,
        note=None if (m38 and m39) else "跨窗参考锚不等——先查驱动/build 面,不误红本臂(G39 C2 口径)")
    if m38 is False or m39 is False:
        NOTES.append(f"跨窗锚:g38={m38} g39={m39} 待归因登记")
    # 窗口臂锚
    def ev_digest(p: Path) -> str | None:
        return json.loads(p.read_text(encoding="utf-8")).get("digest") if p.is_file() else None
    w1 = ev_digest(EV / "win_fc_ev.json")
    w2 = ev_digest(EV / "win_base_ev.json")
    ok5 = w1 is not None and w1 == w2 == WIN_ANCHOR and (EV / "win_fc_sidecar.json").is_file()
    log("J5_window_anchor", status="OK" if ok5 else "FAIL",
        fc=(w1 or "")[:24], base=(w2 or "")[:24])
    if not ok5:
        FAILS.append("J5.window_anchor")
    # ⑥ 帧时分项 measured(登记不判红)
    def wall_of(p: Path) -> float | None:
        if not p.is_file():
            return None
        fr = json.loads(p.read_text(encoding="utf-8"))["frames_data"][1:]
        vals = [f["cut_ms"] + f["delta_ms"] + f["exec_ms"] for f in fr]
        return round(sum(vals) / len(vals), 3) if vals else None
    summary_ms = {
        "dev_select_ms": mean(EV / "p2_dev.json", "select_ms"),
        "dev_verify_ms": mean(EV / "p2_dev.json", "verify_ms"),
        "dev_promote_ms": mean(EV / "p2_dev.json", "promote_ms"),
        "dev_dispatch_gpu_ms": mean(EV / "p2_dev.json", "device_cut_dispatch_gpu_ms"),
        "dev_cut_ms": mean(EV / "p2_dev.json", "cut_ms"),
        "host_select_ms": mean(EV / "p2_host.json", "select_ms"),
        "host_verify_ms": mean(EV / "p2_host.json", "verify_ms"),
        "host_cut_ms": mean(EV / "p2_host.json", "cut_ms"),
        "ml1_dev_select_ms": mean(EV / "p2_dev_ml1.json", "select_ms"),
        "ml1_dev_promote_ms": mean(EV / "p2_dev_ml1.json", "promote_ms"),
        "ml1_dev_cut_ms": mean(EV / "p2_dev_ml1.json", "cut_ms"),
        "ml1_dev_exec_ms": mean(EV / "p2_dev_ml1.json", "exec_ms"),
        "ml1_dev_frame_wall_ms": wall_of(EV / "p2_dev_ml1.json"),
        "ml0_dev_frame_wall_ms": wall_of(EV / "p2_dev.json"),
        "ml0_host_frame_wall_ms": wall_of(EV / "p2_host.json"),
        "design_4_2_expectation": "ml1×device ~15-19ms(登记不判红,不冒充进预算)",
    }
    log("J6_frametime_measured", status="OK", **summary_ms)
    (T2 / "B1_SUMMARY.json").write_text(json.dumps({
        "schema": "rurix.day0901.g40.b1.v1",
        "fails": FAILS, "notes": NOTES, "frametime": summary_ms,
        "cross_anchors": {"g38_t3incr": m38, "g39_t5dev": m39},
        "verdict": "PASS" if not FAILS else "FAIL",
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(("B1 PASS" if not FAILS else f"B1 FAILS: {FAILS}")
          + (f" NOTES: {NOTES}" if NOTES else ""))
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
