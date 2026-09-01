#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G39 GPU 批 1(主 agent 锁内):T2 skin 验收 + T3 折叠前基线轨迹收割。

分段(--only skin|dynbase|staticbase|negctl|fif|all):
  skin       skin-demo ×{1,2,3} × 双跑(a/b):flip-trace 逐帧 digest 三臂等 +
             各臂双跑位级 + skin_verify all_pass(进程 fail-closed)+ VUID=0;
             x2/x3≠x1 而双跑稳 ⇒ L2a「按槽稳定」降档登记不计 FAIL(refit 非纯预案)。
  dynbase    dyn rebuild/refit ×{1,2,3} 单跑:eq_across 复证(T2 加性 0-byte 实证)
             + 轨迹落 t3_fold/baseline/ 作 T3 折叠前基线;若 G38 旧轨迹在
             (.tmp/g38_dyn_fif)则字节对照(跨役负控,bonus)。
  staticbase 静态 FIF ×{1,2,3} 单跑(120f/warmup10):eq_across + 基线轨迹。
  negctl     bench 缺省 160f == c1d28ad7 锚(T2 后静态缺省面负控)。
  fif        g31_fif_dyn_probe selftest + rebuild/refit 双臂(g39 件)。
产物:t2_skin/gpu/(trace/sv/receipt/evidence)+ t3_fold/baseline/ + B1_SUMMARY.json。
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
G39 = ROOT / "artifacts" / "day_0831_g39"
GPU = G39 / "t2_skin" / "gpu"
BASE = G39 / "t3_fold" / "baseline"
TMP = ROOT / ".tmp" / "g39_b1"
LOG = GPU / "b1_log.jsonl"
REL = ROOT / "target-night" / "release"
PERF = REL / "g14_3_pipeline_perf.exe"
FIFP = REL / "g31_fif_dyn_probe.exe"
BENCH_ANCHOR = "c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"
RCP_SUB = Path("bistro-interior") / "tier100" / "tsr_device"

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


def run(step: str, cmd: list[str], env_extra: dict | None = None,
        timeout: int = 1800) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env.pop("RURIX_G31_LAMP_GRID_M", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    if env_extra:
        env.update(env_extra)
    t0 = time.time()
    p = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                       encoding="utf-8", errors="replace", timeout=timeout, env=env)
    dt = round(time.time() - t0, 1)
    vuid = (p.stderr or "").count("VUID-")
    ok = p.returncode == 0 and vuid == 0
    log(step, status="OK" if ok else f"FAIL rc={p.returncode} vuid={vuid}", wall_s=dt,
        stderr_tail=(p.stderr or "")[-2000:] if not ok else (p.stderr or "")[-400:])
    if not ok:
        FAILS.append(step)
    return p


def blob(p: Path) -> bytes | None:
    if p.is_file():
        return p.read_bytes()
    if p.is_dir():
        parts = [f.read_bytes() for f in sorted(p.rglob("*")) if f.is_file()]
        return b"".join(parts) if parts else None
    return None


def bench_cmd(extra: list[str], frames: int = 120, warmup: int = 10,
              out_root: Path | None = None) -> list[str]:
    cmd = [str(PERF), "--bench", "--backend", "tsr_device", "--scene", "bistro-interior",
           "--tier", "100", "--frames", str(frames), "--warmup", str(warmup), *extra]
    if out_root is not None:
        cmd += ["--out-root", str(out_root)]
    return cmd


def seg_skin() -> None:
    GPU.mkdir(parents=True, exist_ok=True)
    traces: dict[str, Path] = {}
    for x in (1, 2, 3):
        for r in ("a", "b"):
            tag = f"x{x}_{r}"
            ft = GPU / f"ft_{tag}"
            out = TMP / f"skin_out_{tag}"
            traces[tag] = ft
            p = run(f"skin.{tag}", bench_cmd(["--skin-demo", "--inflight", str(x)],
                                             out_root=out),
                    env_extra={"RURIX_G14_FLIP_TRACE": str(ft)})
            # skin_verify / receipt 归档(小件入 artifacts)
            for name, dst in (("skin_verify.json", GPU / f"sv_{tag}.json"),
                              ("bench_receipt.json", GPU / f"receipt_{tag}.json")):
                src = out / RCP_SUB / name
                if src.is_file():
                    shutil.copyfile(src, dst)
            sv = GPU / f"sv_{tag}.json"
            if sv.is_file():
                d = json.loads(sv.read_text(encoding="utf-8"))
                mg = d.get("motion_gate", {})
                log(f"skin.{tag}_verify", status="OK" if d.get("all_pass") else "FAIL",
                    all_pass=d.get("all_pass"),
                    host_motion_max_px=mg.get("host_motion_max_px"))
                if not d.get("all_pass"):
                    FAILS.append(f"skin.{tag}.verify")
            elif p.returncode == 0:
                log(f"skin.{tag}_verify", status="FAIL", note="skin_verify.json 缺失")
                FAILS.append(f"skin.{tag}.sv_missing")
        if FAILS:
            return
    b = {k: blob(v) for k, v in traces.items()}
    if any(v is None for v in b.values()):
        log("skin.judge", status="FAIL", missing=[k for k, v in b.items() if v is None])
        FAILS.append("skin.trace_missing")
        return
    eq_double = all(b[f"x{x}_a"] == b[f"x{x}_b"] for x in (1, 2, 3))
    eq_across = b["x1_a"] == b["x2_a"] == b["x3_a"]
    if eq_across and eq_double:
        log("skin.judge", status="OK", eq_across=True, eq_double=True)
    elif eq_double and not eq_across:
        # L2a 降档预案:refit 非纯,按槽稳定显式登记不充逐字节绿(gpu_batch1 L231 先例)
        note = "skin refit 非纯实测:x 臂间逐字节破而各臂双跑位级稳 ⇒ L2a「按槽稳定」降档显式登记(RFC-0030 v1.1 L99)"
        NOTES.append(note)
        log("skin.judge", status="DOWNGRADE_L2A", eq_across=False, eq_double=True, note=note)
    else:
        log("skin.judge", status="FAIL", eq_across=eq_across, eq_double=eq_double)
        FAILS.append("skin.judge")


def seg_dynbase() -> None:
    BASE.mkdir(parents=True, exist_ok=True)
    for action in ("rebuild", "refit"):
        paths: dict[int, Path] = {}
        for x in (1, 2, 3):
            ft = BASE / f"dyn_{action}_x{x}"
            paths[x] = ft
            run(f"dynbase.{action}_x{x}",
                bench_cmd(["--dyn-demo", action, "--inflight", str(x)],
                          out_root=TMP / f"dyn_{action}_x{x}"),
                env_extra={"RURIX_G14_FLIP_TRACE": str(ft)})
        if FAILS:
            return
        b = {x: blob(p) for x, p in paths.items()}
        eq_across = b[1] is not None and b[1] == b[2] == b[3]
        # 跨役负控(bonus):G38 旧轨迹若在,x 臂对位字节比
        g38 = ROOT / ".tmp" / "g38_dyn_fif"
        cross = None
        old = blob(g38 / f"{action}_x1_a") if g38.exists() else None
        if old is not None:
            cross = old == b[1]
        ok = eq_across and (cross is not False)
        log(f"dynbase.{action}_judge", status="OK" if ok else "FAIL",
            eq_across=eq_across, cross_g38_x1=cross)
        if not ok:
            FAILS.append(f"dynbase.{action}")


def seg_staticbase() -> None:
    BASE.mkdir(parents=True, exist_ok=True)
    paths: dict[int, Path] = {}
    for x in (1, 2, 3):
        ft = BASE / f"static_x{x}"
        paths[x] = ft
        run(f"staticbase.x{x}", bench_cmd(["--inflight", str(x)],
                                          out_root=TMP / f"static_x{x}"),
            env_extra={"RURIX_G14_FLIP_TRACE": str(ft)})
    if FAILS:
        return
    b = {x: blob(p) for x, p in paths.items()}
    eq_across = b[1] is not None and b[1] == b[2] == b[3]
    log("staticbase.judge", status="OK" if eq_across else "FAIL", eq_across=eq_across)
    if not eq_across:
        FAILS.append("staticbase")


def seg_negctl() -> None:
    out = TMP / "bench_negctl"
    run("negctl.bench160", bench_cmd([], frames=160, warmup=10, out_root=out))
    rcp = out / RCP_SUB / "bench_receipt.json"
    got = json.loads(rcp.read_text(encoding="utf-8"))["last_frame_digest"] if rcp.is_file() else None
    ok = got is not None and got.endswith(BENCH_ANCHOR)
    log("negctl.judge", status="OK" if ok else "FAIL", got=(got or "")[:24])
    if not ok:
        FAILS.append("negctl")


def seg_fif() -> None:
    GPU.mkdir(parents=True, exist_ok=True)
    run("fif.selftest", [str(FIFP), "--selftest"])
    run("fif.rebuild", [str(FIFP), "--frames", "48", "--rays", "96x72",
                        "--out", str(GPU / "evidence_fif_dyn_rebuild_g39.json")])
    run("fif.refit", [str(FIFP), "--frames", "48", "--rays", "96x72", "--action", "refit",
                      "--out", str(GPU / "evidence_fif_dyn_refit_g39.json")])
    for tag in ("rebuild", "refit"):
        p = GPU / f"evidence_fif_dyn_{tag}_g39.json"
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


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="all",
                    choices=["all", "skin", "dynbase", "staticbase", "negctl", "fif"])
    a = ap.parse_args()
    segs = {"skin": seg_skin, "dynbase": seg_dynbase, "staticbase": seg_staticbase,
            "negctl": seg_negctl, "fif": seg_fif}
    todo = list(segs) if a.only == "all" else [a.only]
    for exe in (PERF, FIFP):
        if not exe.exists():
            print(f"FAIL: 缺 exe {exe}")
            return 2
    TMP.mkdir(parents=True, exist_ok=True)
    with gpu_device_lock(purpose="G39 GPU批1(T2 skin 验收 + 折叠前基线)", timeout_s=2 * 3600.0):
        for name in todo:
            log(f"seg.{name}", status="BEGIN")
            n0 = len(FAILS)
            try:
                segs[name]()
            except Exception as ex:
                log(f"seg.{name}", status=f"EXC {type(ex).__name__}: {ex}")
                FAILS.append(f"{name}.exc")
            log(f"seg.{name}", status="END", seg_fails=FAILS[n0:])
    (GPU / "B1_SUMMARY.json").write_text(json.dumps({
        "schema": "rurix.day0831.g39.b1.v1",
        "fails": FAILS, "notes": NOTES,
        "verdict": "PASS" if not FAILS else "FAIL",
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(("B1 PASS" if not FAILS else f"B1 FAILS: {FAILS}") + (f" NOTES: {NOTES}" if NOTES else ""))
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
