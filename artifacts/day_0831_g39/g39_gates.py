#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G39 收役门全量回归执行器(gpu_batch1「ci 门自带锁不在本脚本内」同律)。

段(--only gates|framecut|guards|all):
  gates    四 ci 门(各自自带 gpu_device_lock,本脚本不持锁):
             g31.waveC.profiling(= B4,T4 多轮中位后首真跑)
             g31.wave95.wp_hlod / g36.wave1.geo_composition
             g31.waveB.restir(T1 冻结面「维持」证明)
  framecut frame_cut probe 回归(本段持锁):incr/full/ml1 + incr==full 16 帧
           位级 + incr 跨进程双跑(gpu_batch1 seg_t3 判据同律)。
  guards   CPU 守卫 7/7(零 GPU)。
fif probe 回归已由 B2 在终态树承载(digest 序列 == B1 + gates 全 true),不重复。
产物:W_GATES.json + gates_log.jsonl。soak 归 g39_soak.py 单独跑。
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
G39 = ROOT / "artifacts" / "day_0831_g39"
OUT = G39 / "closeout"
LOG = OUT / "gates_log.jsonl"
FCP = ROOT / "target-night" / "release" / "g31_frame_cut_probe.exe"

GATES = [
    ("profiling", ["py", "-3", "ci/g31_profiling_smoke.py", "--gate", "g31.waveC.profiling"], 5400),
    ("wp_hlod", ["py", "-3", "ci/g31_wp_hlod_smoke.py", "--gate", "g31.wave95.wp_hlod"], 3600),
    ("g36_geo", ["py", "-3", "ci/g36_geo_composition_smoke.py", "--gate", "g36.wave1.geo_composition"], 3600),
    ("restir_wiring", ["py", "-3", "ci/g31_restir_wiring_smoke.py", "--gate", "g31.waveB.restir"], 3600),
]
GUARDS = [
    ("check_schemas", ["py", "-3", "ci/check_schemas.py"]),
    ("budget_eval", ["py", "-3", "ci/budget_eval.py"]),
    ("gpu_device_lock", ["py", "-3", "ci/gpu_device_lock.py", "--selftest"]),
    ("encode_parity", ["py", "-3", "ci/g31_encode_parity_smoke.py", "--selftest"]),
    ("texture_sampling", ["py", "-3", "ci/g31_texture_sampling_smoke.py", "--selftest"]),
    ("vendor_license", ["py", "-3", "ci/g31_vendor_license_smoke.py", "--selftest"]),
    ("blocked_probes", ["py", "-3", "ci/g31_blocked_probes_smoke.py", "--selftest"]),
]

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

ROWS: list[dict] = []
FAILS: list[str] = []


def log(step: str, **kw) -> None:
    rec = {"t": time.strftime("%H:%M:%S"), "step": step, **kw}
    ROWS.append(rec)
    LOG.parent.mkdir(parents=True, exist_ok=True)
    with LOG.open("a", encoding="utf-8") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(f"[{rec['t']}] {step}: {kw.get('status', '')}", flush=True)


def run(step: str, cmd: list[str], timeout: int = 1800,
        env_extra: dict | None = None) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    env["CARGO_TARGET_DIR"] = r"H:\rurix\target-night"
    if env_extra:
        env.update(env_extra)
    t0 = time.time()
    p = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                       encoding="utf-8", errors="replace", timeout=timeout, env=env)
    ok = p.returncode == 0
    tail = ((p.stdout or "") + "\n" + (p.stderr or ""))[-2500:]
    log(step, status="OK" if ok else f"FAIL rc={p.returncode}",
        argv=" ".join(cmd), wall_s=round(time.time() - t0, 1),
        tail=tail if not ok else tail[-700:])
    if not ok:
        FAILS.append(step)
    return p


def digests_of(ev_path: Path) -> list[str] | None:
    if not ev_path.is_file():
        return None
    return [f["digest"] for f in json.loads(ev_path.read_text(encoding="utf-8"))["frames_data"]]


def seg_gates() -> None:
    for name, cmd, to in GATES:
        run(f"gate.{name}", cmd, timeout=to)


def seg_framecut() -> None:
    ev = OUT / "framecut_ev"
    ev.mkdir(parents=True, exist_ok=True)
    base = [str(FCP), "--cluster-pack", ".tmp/g36_gates/wave1_geo_composition/bistro.rxcp",
            "--error-px", "2.0", "--frames", "16", "--step-m", "0.15", "--res", "96x54"]
    with gpu_device_lock(purpose="G39 收役 framecut probe 回归"):
        run("fc.selftest", [str(FCP), "--selftest"])
        run("fc.incr", base + ["--refit-copy", "incr", "--evidence", str(ev / "t3_incr.json")])
        run("fc.full", base + ["--refit-copy", "full", "--evidence", str(ev / "t3_full.json")])
        run("fc.incr_r2", base + ["--refit-copy", "incr", "--evidence", str(ev / "t3_incr_r2.json")])
        run("fc.ml1", base + ["--refit-copy", "incr", "--min-level", "1",
                              "--evidence", str(ev / "t3_ml1.json")])
    da, db, dr = digests_of(ev / "t3_incr.json"), digests_of(ev / "t3_full.json"), digests_of(ev / "t3_incr_r2.json")
    ok = da is not None and da == db and da == dr and len(da) == 16
    log("fc.judge", status="OK" if ok else "FAIL",
        incr_eq_full=da == db, double_run=da == dr)
    if not ok:
        FAILS.append("fc.judge")
    # 跨役锚:与 G38 t3_incr 序列对照(登记面)
    g38 = digests_of(ROOT / "artifacts/day_0830_g38/t3_framecut/ev/t3_incr.json")
    log("fc.cross_g38", status="OK" if da == g38 else "NOTE", match=da == g38)


def seg_guards() -> None:
    for name, cmd in GUARDS:
        run(f"guard.{name}", cmd, timeout=1200)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="all", choices=["all", "gates", "framecut", "guards"])
    a = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    todo = {"gates": seg_gates, "framecut": seg_framecut, "guards": seg_guards}
    names = list(todo) if a.only == "all" else [a.only]
    for n in names:
        log(f"seg.{n}", status="BEGIN")
        k0 = len(FAILS)
        try:
            todo[n]()
        except Exception as ex:
            log(f"seg.{n}", status=f"EXC {type(ex).__name__}: {ex}")
            FAILS.append(f"{n}.exc")
        log(f"seg.{n}", status="END", seg_fails=FAILS[k0:])
    (OUT / "W_GATES.json").write_text(json.dumps({
        "schema": "rurix.day0831.g39.w_gates.v1",
        "fails": FAILS,
        "verdict": "PASS" if not FAILS else "FAIL",
        "rows": ROWS,
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("W_GATES PASS" if not FAILS else f"W_GATES FAILS: {FAILS}")
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
