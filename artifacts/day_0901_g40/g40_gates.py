#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G40 收役门全量回归(g39_gates.py 同型扩段)。

段(--only build|gates|framecut|fif|guards|all):
  build    终态树 CPU 构建(g31_frame_cut_probe/g31_fif_dyn_probe 重建——
           T3 lane_body 改动进 probe include 面;锁外)。
  gates    四 ci 门(各自自带 gpu_device_lock,本脚本不持锁):
             g31.waveC.profiling(若红按 G39 t4_profiling/REPORT §5 预案:
             如实维持红 + 重标定提案归 budget 程序窗,禁改判据凑绿)
             g31.wave95.wp_hlod / g36.wave1.geo_composition
             g31.waveB.restir(T1 冻结面〔g28/gi 金标准〕维持证明)
  framecut 本段持锁:selftest + host incr/full/ml1 + incr==full 16f 位级 +
           跨进程双跑 + 跨 G38 t3_incr 锚 + **device 臂终态树回归**
           (P2 生产 dispatch;dev==host 位级)。
  fif      本段持锁:g31_fif_dyn_probe selftest + rebuild/refit 双臂
           (gates 全 true + trimmed_mean)+ calibrate_fif_budget.py --check。
  guards   CPU 守卫 7/7(零 GPU;budget_eval 期望 330 pass 0 skip)。
产物:closeout/W_GATES.json + gates_log.jsonl。soak 归 g40_soak.py 单独跑。
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
OUT = ROOT / "artifacts" / "day_0901_g40" / "closeout"
LOG = OUT / "gates_log.jsonl"
FCP = ROOT / "target-night" / "release" / "g31_frame_cut_probe.exe"
FIFP = ROOT / "target-night" / "release" / "g31_fif_dyn_probe.exe"
KSPV = ROOT / ".tmp" / "g40_gates" / "t2_devicecut" / "g31_cluster_cull.spv"
PACK = ".tmp/g36_gates/wave1_geo_composition/bistro.rxcp"

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


def seg_build() -> None:
    run("build.frame_cut_probe",
        ["cargo", "build", "--release", "-p", "rurix-render",
         "--features", "vendor-upscale", "--bin", "g31_frame_cut_probe"],
        timeout=1200)
    run("build.fif_dyn_probe",
        ["cargo", "build", "--release", "-p", "rurix-render",
         "--features", "vendor-upscale", "--bin", "g31_fif_dyn_probe"],
        timeout=1200)


def seg_gates() -> None:
    for name, cmd, to in GATES:
        run(f"gate.{name}", cmd, timeout=to)


def seg_framecut() -> None:
    ev = OUT / "framecut_ev"
    ev.mkdir(parents=True, exist_ok=True)
    base = [str(FCP), "--cluster-pack", PACK,
            "--error-px", "2.0", "--frames", "16", "--step-m", "0.15", "--res", "96x54"]
    with gpu_device_lock(purpose="G40 收役 framecut probe 回归"):
        run("fc.selftest", [str(FCP), "--selftest"])
        run("fc.incr", base + ["--refit-copy", "incr", "--evidence", str(ev / "t3_incr.json")])
        run("fc.full", base + ["--refit-copy", "full", "--evidence", str(ev / "t3_full.json")])
        run("fc.incr_r2", base + ["--refit-copy", "incr", "--evidence", str(ev / "t3_incr_r2.json")])
        run("fc.ml1", base + ["--refit-copy", "incr", "--min-level", "1",
                              "--evidence", str(ev / "t3_ml1.json")])
        # G40 T2:device 臂终态树回归(P2 生产 dispatch;dev==host 位级)。
        run("fc.dev", base + ["--cut-source", "device", "--cull-spv", str(KSPV),
                              "--evidence", str(ev / "t5_dev.json")])
    da, db, dr = digests_of(ev / "t3_incr.json"), digests_of(ev / "t3_full.json"), digests_of(ev / "t3_incr_r2.json")
    ok = da is not None and da == db and da == dr and len(da) == 16
    log("fc.judge", status="OK" if ok else "FAIL",
        incr_eq_full=da == db, double_run=da == dr)
    if not ok:
        FAILS.append("fc.judge")
    dd = digests_of(ev / "t5_dev.json")
    okd = da is not None and dd == da
    log("fc.judge_dev", status="OK" if okd else "FAIL", dev_eq_host=dd == da)
    if not okd:
        FAILS.append("fc.judge_dev")
    g38 = digests_of(ROOT / "artifacts/day_0830_g38/t3_framecut/ev/t3_incr.json")
    log("fc.cross_g38", status="OK" if da == g38 else "NOTE", match=da == g38)


def seg_fif() -> None:
    ev = OUT / "fif_ev"
    ev.mkdir(parents=True, exist_ok=True)
    with gpu_device_lock(purpose="G40 收役 fif probe 回归"):
        run("fif.selftest", [str(FIFP), "--selftest"])
        run("fif.rebuild", [str(FIFP), "--frames", "48", "--rays", "96x72",
                            "--out", str(ev / "evidence_fif_dyn_rebuild_g40.json")])
        run("fif.refit", [str(FIFP), "--frames", "48", "--rays", "96x72", "--action", "refit",
                          "--out", str(ev / "evidence_fif_dyn_refit_g40.json")])
    for tag in ("rebuild", "refit"):
        p = ev / f"evidence_fif_dyn_{tag}_g40.json"
        if not p.exists():
            FAILS.append(f"fif.{tag}.missing")
            continue
        e = json.loads(p.read_text(encoding="utf-8"))
        gates = e.get("gates", {})
        tm = e.get("results", {}).get("trimmed_mean")
        ok = all(gates.values()) and isinstance(tm, (int, float)) and tm > 0
        log(f"fif.{tag}_judge", status="OK" if ok else "FAIL",
            gates_all=all(gates.values()), trimmed_mean=tm)
        if not ok:
            FAILS.append(f"fif.{tag}.judge")
    run("fif.calibrate_check", ["py", "-3", "ci/calibrate_fif_budget.py", "--check"],
        timeout=600)


def seg_guards() -> None:
    for name, cmd in GUARDS:
        run(f"guard.{name}", cmd, timeout=1200)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="all",
                    choices=["all", "build", "gates", "framecut", "fif", "guards"])
    a = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    todo = {"build": seg_build, "gates": seg_gates, "framecut": seg_framecut,
            "fif": seg_fif, "guards": seg_guards}
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
        "schema": "rurix.day0901.g40.gates.v1",
        "fails": FAILS,
        "verdict": "PASS" if not FAILS else "FAIL",
        "rows": ROWS,
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(("W_GATES PASS" if not FAILS else f"W_GATES FAILS: {FAILS}"))
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
