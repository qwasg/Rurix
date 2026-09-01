#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G39 GPU 批 2:T3 slot_as 单源折叠零语义门(全量位级复证 == 折叠前基线)。

基线 = B1 收割件:
  静态 FIF ×{1,2,3}      t3_fold/baseline/static_x*
  dyn rebuild/refit ×{1,2,3} t3_fold/baseline/dyn_{action}_x*
  skin ×{1,2,3}          t2_skin/gpu/ft_x*_a(B1 skin 双跑 a 腿)
  fif probe 双臂          t2_skin/gpu/evidence_fif_dyn_{rebuild,refit}_g39.json(digests 序列)
判据:折叠后单跑轨迹与基线逐字节等(15 跑)+ fif probe gates 全 true 且逐臂
digest 序列 == B1 件。任何不等 = 零语义门红(fail-closed,不降档)。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
G39 = ROOT / "artifacts" / "day_0831_g39"
FOLD = G39 / "t3_fold"
POST = FOLD / "post"
BASE = FOLD / "baseline"
SKIN_BASE = G39 / "t2_skin" / "gpu"
LOG = FOLD / "b2_log.jsonl"
TMP = ROOT / ".tmp" / "g39_b2"
REL = ROOT / "target-night" / "release"
PERF = REL / "g14_3_pipeline_perf.exe"
FIFP = REL / "g31_fif_dyn_probe.exe"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

FAILS: list[str] = []


def log(step: str, **kw) -> None:
    rec = {"t": time.strftime("%H:%M:%S"), "step": step, **kw}
    LOG.parent.mkdir(parents=True, exist_ok=True)
    with LOG.open("a", encoding="utf-8") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(f"[{rec['t']}] {step}: {kw.get('status', '')}", flush=True)


def run(step: str, cmd: list[str], env_extra: dict | None = None) -> None:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env.pop("RURIX_G31_LAMP_GRID_M", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    if env_extra:
        env.update(env_extra)
    t0 = time.time()
    p = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                       encoding="utf-8", errors="replace", timeout=1800, env=env)
    vuid = (p.stderr or "").count("VUID-")
    ok = p.returncode == 0 and vuid == 0
    log(step, status="OK" if ok else f"FAIL rc={p.returncode} vuid={vuid}",
        wall_s=round(time.time() - t0, 1),
        stderr_tail=(p.stderr or "")[-1500:] if not ok else None)
    if not ok:
        FAILS.append(step)


def blob(p: Path) -> bytes | None:
    if p.is_file():
        return p.read_bytes()
    if p.is_dir():
        parts = [f.read_bytes() for f in sorted(p.rglob("*")) if f.is_file()]
        return b"".join(parts) if parts else None
    return None


def bench(extra: list[str], out_tag: str) -> list[str]:
    return [str(PERF), "--bench", "--backend", "tsr_device", "--scene", "bistro-interior",
            "--tier", "100", "--frames", "120", "--warmup", "10", *extra,
            "--out-root", str(TMP / out_tag)]


def replay(tag: str, extra: list[str], baseline: Path) -> None:
    ft = POST / tag
    run(f"replay.{tag}", bench(extra, tag), env_extra={"RURIX_G14_FLIP_TRACE": str(ft)})
    if FAILS and FAILS[-1] == f"replay.{tag}":
        return
    a, b = blob(ft), blob(baseline)
    ok = a is not None and a == b
    log(f"judge.{tag}", status="OK" if ok else "FAIL",
        baseline=str(baseline), post_bytes=len(a or b""), base_bytes=len(b or b""))
    if not ok:
        FAILS.append(f"judge.{tag}")


def fif_digests(p: Path) -> dict | None:
    if not p.is_file():
        return None
    e = json.loads(p.read_text(encoding="utf-8"))
    return {k: v.get("digests") for k, v in (e.get("arms") or {}).items()}


def main() -> int:
    for exe in (PERF, FIFP):
        if not exe.exists():
            print(f"FAIL: 缺 exe {exe}")
            return 2
    POST.mkdir(parents=True, exist_ok=True)
    TMP.mkdir(parents=True, exist_ok=True)
    with gpu_device_lock(purpose="G39 GPU批2(T3 折叠零语义门全量复证)", timeout_s=2 * 3600.0):
        for x in (1, 2, 3):
            replay(f"static_x{x}", ["--inflight", str(x)], BASE / f"static_x{x}")
        for action in ("rebuild", "refit"):
            for x in (1, 2, 3):
                replay(f"dyn_{action}_x{x}", ["--dyn-demo", action, "--inflight", str(x)],
                       BASE / f"dyn_{action}_x{x}")
        for x in (1, 2, 3):
            replay(f"skin_x{x}", ["--skin-demo", "--inflight", str(x)],
                   SKIN_BASE / f"ft_x{x}_a")
        # fif probe 双臂:gates 全 true + digest 序列 == B1 件
        for tag, extra in (("rebuild", []), ("refit", ["--action", "refit"])):
            out = FOLD / f"evidence_fif_dyn_{tag}_post.json"
            run(f"fif.{tag}", [str(FIFP), "--frames", "48", "--rays", "96x72",
                               *extra, "--out", str(out)])
            e = json.loads(out.read_text(encoding="utf-8")) if out.is_file() else {}
            gates_ok = bool(e.get("gates")) and all(e["gates"].values())
            da = fif_digests(out)
            db = fif_digests(SKIN_BASE / f"evidence_fif_dyn_{tag}_g39.json")
            eq = da is not None and da == db
            log(f"fif.{tag}_judge", status="OK" if (gates_ok and eq) else "FAIL",
                gates_all=gates_ok, digests_eq_b1=eq)
            if not (gates_ok and eq):
                FAILS.append(f"fif.{tag}")
    (FOLD / "B2_SUMMARY.json").write_text(json.dumps({
        "schema": "rurix.day0831.g39.b2.v1",
        "fails": FAILS,
        "verdict": "PASS" if not FAILS else "FAIL",
        "caliber": "折叠后 15 跑轨迹逐字节 == 折叠前基线(B1 收割)+ fif probe 双臂 gates+digest 序列恒等",
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("B2 PASS" if not FAILS else f"B2 FAILS: {FAILS}")
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
