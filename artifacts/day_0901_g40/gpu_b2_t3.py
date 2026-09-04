#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G40 T3 GPU 验收批 B2(AS 副本内存 evidence 补登;主 agent 锁内)。

判据(任务书 B2 两条 + B1f 附录):
  ① dyn/skin × inflight 2|3 各一跑:bench receipt 加性 slot_as_mem 字段在档
     (per_slot_bytes 长度 == inflight、全 >0、生产规模数百 MB 级 measured
     登记)+ skin_verify all_pass + VUID=0。
  ② flip-trace 轨迹对 G39 B1/B2 在案件位级不漂(dyn_rebuild_x2|x3 对
     t3_fold/baseline;skin x2|x3 对 t2_skin/gpu/ft_x{2,3}_a)。
  ③ B1f 附录:窗口臂 5540ecae 锚以 T2+T3 树新建二进制复验(B1 首验用 W0
     二进制,证明力不足如实登记——本附录闭合)。
  ④ inflight=1(非 slot_as)receipt 0-byte 负控:receipt 无 slot_as_mem 键。
产物:t3_asmem/ + B2_SUMMARY.json + b2_log.jsonl。
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
T3 = ROOT / "artifacts" / "day_0901_g40" / "t3_asmem"
TMP = ROOT / ".tmp" / "g40_b2"
LOG = T3 / "b2_log.jsonl"
REL = ROOT / "target-night" / "release"
PERF = REL / "g14_3_pipeline_perf.exe"
WIN = REL / "g31_window_present.exe"
RCP_SUB = Path("bistro-interior") / "tier100" / "tsr_device"
G39_BASE = ROOT / "artifacts" / "day_0831_g39" / "t3_fold" / "baseline"
G39_SKIN = ROOT / "artifacts" / "day_0831_g39" / "t2_skin" / "gpu"
PACK = ".tmp/g36_gates/wave1_geo_composition/bistro.rxcp"
WIN_ANCHOR = "sha256:5540ecaed4fd4c1e3e0abea7f937bbb2e200434096a86a772dbb48238d0e0ea8"

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
        stderr_tail=(p.stderr or "")[-1600:] if not ok else (p.stderr or "")[-300:])
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


def bench_cmd(extra: list[str], out_root: Path) -> list[str]:
    return [str(PERF), "--bench", "--backend", "tsr_device", "--scene", "bistro-interior",
            "--tier", "100", "--frames", "120", "--warmup", "10", *extra,
            "--out-root", str(out_root)]


def check_mem(tag: str, inflight: int) -> dict | None:
    rcp = T3 / f"receipt_{tag}.json"
    if not rcp.is_file():
        FAILS.append(f"{tag}.receipt_missing")
        return None
    d = json.loads(rcp.read_text(encoding="utf-8"))
    mem = d.get("slot_as_mem")
    if inflight == 1:
        ok = mem is None
        log(f"mem.{tag}", status="OK" if ok else "FAIL",
            note="inflight=1 非 slot_as:receipt 应无 slot_as_mem 键(0-byte 负控)")
        if not ok:
            FAILS.append(f"{tag}.mem_leak")
        return None
    ok = (isinstance(mem, dict)
          and isinstance(mem.get("per_slot_bytes"), list)
          and len(mem["per_slot_bytes"]) == inflight
          and all(isinstance(b, int) and b > 0 for b in mem["per_slot_bytes"])
          and mem.get("group_total_bytes") == sum(mem["per_slot_bytes"]))
    log(f"mem.{tag}", status="OK" if ok else "FAIL",
        per_slot=mem.get("per_slot_bytes") if isinstance(mem, dict) else None,
        group_total=mem.get("group_total_bytes") if isinstance(mem, dict) else None)
    if not ok:
        FAILS.append(f"{tag}.mem_fields")
    return mem if ok else None


def main() -> int:
    for exe in (PERF, WIN):
        if not exe.exists():
            print(f"FAIL: 缺 exe {exe}")
            return 2
    T3.mkdir(parents=True, exist_ok=True)
    TMP.mkdir(parents=True, exist_ok=True)
    mems: dict[str, dict | None] = {}
    with gpu_device_lock(purpose="G40 B2 T3 AS 内存补登 + B1f 附录", timeout_s=2 * 3600.0):
        # ③ B1f 附录:T2+T3 树窗口臂锚复验
        wcmd = [str(WIN), "--quality", "off", "--headless-smoke", "--auto-move", "dolly",
                "--tier", "100", "--cluster-lod", "on", "--cluster-error-px", "2.0",
                "--cluster-pack", PACK, "--frames", "24", "--warmup", "2", "--hidden"]
        run("B1fx_win_fc", wcmd + ["--cluster-per-frame-cut", "on",
                                   "--frame-cut-out", str(T3 / "win_fc_sidecar.json"),
                                   "--evidence", str(T3 / "win_fc_ev.json")])
        run("B1fx_win_base", wcmd + ["--evidence", str(T3 / "win_base_ev.json")])
        # ① dyn rebuild ×2|3 + skin ×2|3(+ inflight1 负控各一)
        for x in (1, 2, 3):
            tag = f"dyn_x{x}"
            out = TMP / tag
            run(f"run.{tag}", bench_cmd(["--dyn-demo", "rebuild", "--inflight", str(x)], out),
                env_extra={"RURIX_G14_FLIP_TRACE": str(T3 / f"ft_{tag}")})
            src = out / RCP_SUB / "bench_receipt.json"
            if src.is_file():
                shutil.copyfile(src, T3 / f"receipt_{tag}.json")
        for x in (1, 2, 3):
            tag = f"skin_x{x}"
            out = TMP / tag
            run(f"run.{tag}", bench_cmd(["--skin-demo", "--inflight", str(x)], out),
                env_extra={"RURIX_G14_FLIP_TRACE": str(T3 / f"ft_{tag}")})
            for name, dst in (("bench_receipt.json", T3 / f"receipt_{tag}.json"),
                              ("skin_verify.json", T3 / f"sv_{tag}.json")):
                src = out / RCP_SUB / name
                if src.is_file():
                    shutil.copyfile(src, dst)
            sv = T3 / f"sv_{tag}.json"
            if sv.is_file():
                d = json.loads(sv.read_text(encoding="utf-8"))
                if not d.get("all_pass"):
                    log(f"sv.{tag}", status="FAIL", all_pass=d.get("all_pass"))
                    FAILS.append(f"{tag}.skin_verify")
                else:
                    log(f"sv.{tag}", status="OK")

    # ── 判读(锁外)──
    def ev_digest(p: Path) -> str | None:
        return json.loads(p.read_text(encoding="utf-8")).get("digest") if p.is_file() else None
    w1, w2 = ev_digest(T3 / "win_fc_ev.json"), ev_digest(T3 / "win_base_ev.json")
    ok3 = w1 is not None and w1 == w2 == WIN_ANCHOR
    log("J_B1fx_window_anchor", status="OK" if ok3 else "FAIL",
        fc=(w1 or "")[:24], base=(w2 or "")[:24])
    if not ok3:
        FAILS.append("B1fx.window_anchor")
    # ① 字段在档
    for x in (1, 2, 3):
        mems[f"dyn_x{x}"] = check_mem(f"dyn_x{x}", x)
        mems[f"skin_x{x}"] = check_mem(f"skin_x{x}", x)
    # ② flip-trace 位级对 G39 在案件
    pairs = [
        ("dyn_x2", G39_BASE / "dyn_rebuild_x2"), ("dyn_x3", G39_BASE / "dyn_rebuild_x3"),
        ("dyn_x1", G39_BASE / "dyn_rebuild_x1"),
        ("skin_x2", G39_SKIN / "ft_x2_a"), ("skin_x3", G39_SKIN / "ft_x3_a"),
        ("skin_x1", G39_SKIN / "ft_x1_a"),
    ]
    for tag, ref in pairs:
        a, b = blob(T3 / f"ft_{tag}"), blob(ref)
        ok = a is not None and a == b
        log(f"J_trace.{tag}", status="OK" if ok else "FAIL",
            ref=str(ref.name), bytes=len(a or b""))
        if not ok:
            FAILS.append(f"trace.{tag}")
    (T3 / "B2_SUMMARY.json").write_text(json.dumps({
        "schema": "rurix.day0901.g40.b2.v1",
        "fails": FAILS, "notes": NOTES,
        "slot_as_mem": {k: v for k, v in mems.items() if v},
        "verdict": "PASS" if not FAILS else "FAIL",
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(("B2 PASS" if not FAILS else f"B2 FAILS: {FAILS}")
          + (f" NOTES: {NOTES}" if NOTES else ""))
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
