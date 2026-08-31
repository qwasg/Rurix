#!/usr/bin/env python3
"""G38 GPU 批次 1 执行器(主 agent 锁内跑;T3/T2/T4 三组验收)。

分段:--only t3|fif|dyn|g34|all(默认 all)。全部直接 exe 命令(锁内);
ci 门脚本(自带锁)不在本脚本内,归批次后段。
日志:artifacts/day_0830_g38/batch1_log.jsonl(逐步追加);FAIL 即该段停,
其余段照跑(段间独立),终态汇总退码 = 任一段 FAIL 则 1。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
G38 = ROOT / "artifacts" / "day_0830_g38"
LOG = G38 / "batch1_log.jsonl"
REL = ROOT / "target-night" / "release"
FCP = REL / "g31_frame_cut_probe.exe"
WIN = REL / "g31_window_present.exe"
PERF = REL / "g14_3_pipeline_perf.exe"
FIFP = REL / "g31_fif_dyn_probe.exe"
G34 = REL / "g34_full_lane.exe"
PACK_V1 = r".tmp/g36_gates/wave1_geo_composition/bistro.rxcp"
WPPK_V1 = r".tmp/g36_gates/wave1_geo_composition/bistro.rxwh"
PACK_V2 = r".tmp/t4_attr96/bistro_v2_attrs.rxcp"
WPPK_V2 = r".tmp/t4_attr96/bistro_v2_attrs.rxwh"
SLAB = r"milestones/g31/g31_slab_side_table_bistro_interior.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

FAILS: list[str] = []


def log(step: str, **kw) -> None:
    rec = {"t": time.strftime("%H:%M:%S"), "step": step, **kw}
    with LOG.open("a", encoding="utf-8") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(f"[{rec['t']}] {step}: {kw.get('status', '')}", flush=True)


def run(step: str, cmd: list[str], env_extra: dict | None = None,
        timeout: int = 1800, expect_rc: int = 0) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    if env_extra:
        env.update(env_extra)
    t0 = time.time()
    p = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                       encoding="utf-8", errors="replace", timeout=timeout, env=env)
    dt = round(time.time() - t0, 1)
    ok = p.returncode == expect_rc
    # stderr 末 2000 字符落日志(装配/digest 行在 stderr)
    log(step, status="OK" if ok else f"FAIL rc={p.returncode}", wall_s=dt,
        cmd=" ".join(cmd[:6]) + (" ..." if len(cmd) > 6 else ""),
        stderr_tail=p.stderr[-2000:] if not ok else p.stderr[-600:])
    if not ok:
        FAILS.append(step)
    return p


def digests_of(ev_path: Path) -> list[str]:
    e = json.loads(ev_path.read_text(encoding="utf-8"))
    return [f["digest"] for f in e["frames_data"]]


# ── T3:frame_cut 增量 refit(B0~B7)──────────────────────────────────


def seg_t3() -> None:
    ev = G38 / "t3_framecut" / "ev"
    ev.mkdir(parents=True, exist_ok=True)
    base = [str(FCP), "--cluster-pack", PACK_V1, "--error-px", "2.0",
            "--frames", "16", "--step-m", "0.15", "--res", "96x54"]
    run("t3.B0_selftest", [str(FCP), "--selftest"])
    if FAILS:
        return
    run("t3.B1_incr", base + ["--refit-copy", "incr", "--evidence", str(ev / "t3_incr.json")])
    run("t3.B2_full", base + ["--refit-copy", "full", "--evidence", str(ev / "t3_full.json")])
    if FAILS:
        return
    da, db = digests_of(ev / "t3_incr.json"), digests_of(ev / "t3_full.json")
    if da == db and len(da) == 16:
        log("t3.B3_incr_eq_full", status="OK", n=16)
    else:
        log("t3.B3_incr_eq_full", status="FAIL", da=da[:3], db=db[:3])
        FAILS.append("t3.B3")
    run("t3.B4_incr_r2", base + ["--refit-copy", "incr", "--evidence", str(ev / "t3_incr_r2.json")])
    if digests_of(ev / "t3_incr_r2.json") == da:
        log("t3.B4_double_run", status="OK")
    else:
        log("t3.B4_double_run", status="FAIL")
        FAILS.append("t3.B4")
    run("t3.B5_minlevel1", base + ["--refit-copy", "incr", "--min-level", "1",
                                   "--evidence", str(ev / "t3_ml1.json")])
    # B6 字段核对 + 帧时汇总
    e1 = json.loads((ev / "t3_incr.json").read_text(encoding="utf-8"))
    f1 = e1["frames_data"][1]
    # 帧 0 全量 = 桥 copy 窗口径 arena_tris×36 = 2,082,603×36 = 74,973,708B
    # (≠ s09 host 上传口径 75,139,596B——那含 passthrough 尾段;首版判读硬编码错,已修)。
    ARENA_BYTES = 74_973_708
    b6_ok = (e1.get("refit_copy_mode") == "incr" and e1.get("min_level") == 0
             and f1.get("copy_regions", 0) >= 1 and 0 < f1.get("copy_bytes", 0) < ARENA_BYTES
             and isinstance(f1.get("bridge_copy_gpu_ms"), (int, float))
             and isinstance(f1.get("bridge_build_gpu_ms"), (int, float))
             and all("cut_tris_promoted" in fr for fr in e1["frames_data"]))
    f0 = e1["frames_data"][0]
    b6_ok = b6_ok and f0.get("copy_regions") == 1 and f0.get("copy_bytes") == ARENA_BYTES

    def mean(ev_p: Path, key: str, skip0: bool = True) -> float | None:
        fr = json.loads(ev_p.read_text(encoding="utf-8"))["frames_data"]
        vals = [f[key] for f in (fr[1:] if skip0 else fr) if isinstance(f.get(key), (int, float))]
        return round(sum(vals) / len(vals), 3) if vals else None

    summary = {
        "incr_exec_ms": mean(ev / "t3_incr.json", "exec_ms"),
        "full_exec_ms": mean(ev / "t3_full.json", "exec_ms"),
        "ml1_exec_ms": mean(ev / "t3_ml1.json", "exec_ms"),
        "incr_copy_gpu_ms": mean(ev / "t3_incr.json", "bridge_copy_gpu_ms"),
        "full_copy_gpu_ms": mean(ev / "t3_full.json", "bridge_copy_gpu_ms"),
        "incr_build_gpu_ms": mean(ev / "t3_incr.json", "bridge_build_gpu_ms"),
        "ml1_build_gpu_ms": mean(ev / "t3_ml1.json", "bridge_build_gpu_ms"),
    }
    log("t3.B6_fields", status="OK" if b6_ok else "FAIL", **summary)
    if not b6_ok:
        FAILS.append("t3.B6")
    # B7 窗口臂加性回归——**W4 s09 原口径**(w4_verify.py L277-285:headless-smoke
    # + frames 24/warmup 2/hidden/evidence;digest 取 evidence 字段对 W4 锚全串)。
    # 首版误用 headless-smoke 缺省 130 帧口径(fc==base 相等但值非锚),已修。
    wcmd = [str(WIN), "--quality", "off", "--headless-smoke", "--auto-move", "dolly",
            "--tier", "100", "--cluster-lod", "on", "--cluster-error-px", "2.0",
            "--cluster-pack", PACK_V1, "--frames", "24", "--warmup", "2", "--hidden"]
    run("t3.B7_win_fc", wcmd + ["--cluster-per-frame-cut", "on",
                                "--frame-cut-out", str(ev / "t3_window_fc.json"),
                                "--evidence", str(ev / "t3_win_fc_ev.json")])
    run("t3.B7_win_base", wcmd + ["--evidence", str(ev / "t3_win_base_ev.json")])
    anchor = "sha256:5540ecaed4fd4c1e3e0abea7f937bbb2e200434096a86a772dbb48238d0e0ea8"

    def ev_digest(p: Path) -> str | None:
        if not p.exists():
            return None
        return json.loads(p.read_text(encoding="utf-8")).get("digest")

    d1 = ev_digest(ev / "t3_win_fc_ev.json")
    d2 = ev_digest(ev / "t3_win_base_ev.json")
    ok7 = d1 is not None and d1 == d2 == anchor and (ev / "t3_window_fc.json").is_file()
    log("t3.B7_digest", status="OK" if ok7 else "FAIL",
        d1=(d1 or "")[:24], d2=(d2 or "")[:24], anchor=anchor[:24])
    if not ok7:
        FAILS.append("t3.B7")


# ── T2a:fif probe 收割(schema v2 slot_as_mem 账)────────────────────


def seg_fif() -> None:
    out = G38 / "t2_fifdyn"
    run("fif.selftest", [str(FIFP), "--selftest"])
    run("fif.rebuild", [str(FIFP), "--frames", "48", "--rays", "96x72",
                        "--out", str(out / "evidence_fif_dyn_rebuild_g38.json")])
    run("fif.refit", [str(FIFP), "--frames", "48", "--rays", "96x72", "--action", "refit",
                      "--out", str(out / "evidence_fif_dyn_refit_g38.json")])
    for tag in ("rebuild", "refit"):
        p = out / f"evidence_fif_dyn_{tag}_g38.json"
        if not p.exists():
            FAILS.append(f"fif.{tag}.missing")
            continue
        e = json.loads(p.read_text(encoding="utf-8"))
        gates = e.get("gates", {})
        mem = e.get("slot_as_mem")
        tm = e.get("results", {}).get("trimmed_mean")
        ok = all(gates.values()) and mem is not None and isinstance(tm, (int, float)) and tm > 0
        log(f"fif.{tag}_judge", status="OK" if ok else "FAIL",
            gates_all=all(gates.values()), trimmed_mean=tm,
            mem_keys=sorted(mem.keys()) if isinstance(mem, dict) else None)
        if not ok:
            FAILS.append(f"fif.{tag}.judge")


# ── T2b:dyn 生产接线三跑等价(rebuild + refit)───────────────────────


def _dyn_arm(action: str) -> None:
    trace_root = ROOT / ".tmp" / "g38_dyn_fif"
    trace_root.mkdir(parents=True, exist_ok=True)
    paths: dict[str, Path] = {}
    for x in (1, 2, 3):
        for r in ("a", "b"):
            tp = trace_root / f"{action}_x{x}_{r}"
            paths[f"x{x}_{r}"] = tp
            run(f"dyn.{action}_x{x}_{r}",
                [str(PERF), "--bench", "--backend", "tsr_device", "--scene", "bistro-interior",
                 "--tier", "100", "--frames", "120", "--warmup", "10",
                 "--dyn-demo", action, "--inflight", str(x)],
                env_extra={"RURIX_G14_FLIP_TRACE": str(tp)})
    # 判读:trace 产物(文件或目录)字节级比对
    def blob(p: Path) -> bytes | None:
        if p.is_file():
            return p.read_bytes()
        cand = [p.with_suffix(s) for s in (".jsonl", ".json")] + \
               sorted(p.parent.glob(p.name + "*")) if not p.is_dir() else []
        if p.is_dir():
            parts = []
            for f in sorted(p.rglob("*")):
                if f.is_file():
                    parts.append(f.read_bytes())
            return b"".join(parts) if parts else None
        for c in cand:
            if isinstance(c, Path) and c.is_file():
                return c.read_bytes()
        return None

    b = {k: blob(v) for k, v in paths.items()}
    if any(v is None for v in b.values()):
        log(f"dyn.{action}_judge", status="FAIL", missing=[k for k, v in b.items() if v is None])
        FAILS.append(f"dyn.{action}.trace_missing")
        return
    eq_across = b["x1_a"] == b["x2_a"] == b["x3_a"]
    eq_double = all(b[f"x{x}_a"] == b[f"x{x}_b"] for x in (1, 2, 3))
    ok = eq_across and eq_double
    log(f"dyn.{action}_judge", status="OK" if ok else "FAIL",
        eq_across=eq_across, eq_double=eq_double,
        note="refit 非纯时按 L2a 降档登记" if (action == "refit" and not eq_across and eq_double) else "")
    if not ok and not (action == "refit" and eq_double):
        FAILS.append(f"dyn.{action}.judge")


def seg_dyn() -> None:
    _dyn_arm("rebuild")
    _dyn_arm("refit")


# ── T4:#96 三验收锚(锚①②主体 + 锚③ a/b 判据)──────────────────────


def seg_g34() -> None:
    out = ROOT / ".tmp" / "t4_attr96"
    g34_base = [str(G34), "--frames", "12", "--warmup", "2", "--tier", "100", "--full",
                "--slab-table", SLAB, "--auto-move", "orbit", "--hidden"]
    run("g34.base", g34_base + ["--evidence", str(out / "g34_base.json")])
    run("g34.leafxfull_v1", g34_base + [
        "--cluster-lod", "leaf", "--cluster-pack", PACK_V1,
        "--wp-hlod", "full", "--wp-pack", WPPK_V1,
        "--evidence", str(out / "g34_leafxfull_v1.json")])
    run("g34.leafxfull_v2", g34_base + [
        "--cluster-lod", "leaf", "--cluster-pack", PACK_V2,
        "--wp-hlod", "full", "--wp-pack", WPPK_V2,
        "--evidence", str(out / "g34_leafxfull_v2.json")])

    def seq(p: Path) -> list | None:
        if not p.exists():
            return None
        e = json.loads(p.read_text(encoding="utf-8"))
        return e.get("digest_seq") or e.get("frames_digests") or [e.get("digest")]

    s0 = seq(out / "g34_base.json")
    s1 = seq(out / "g34_leafxfull_v1.json")
    s2 = seq(out / "g34_leafxfull_v2.json")
    ok12 = s0 is not None and s0 == s1 == s2
    log("g34.anchor12_judge", status="OK" if ok12 else "FAIL",
        base0=str(s0[0])[:16] if s0 else None, eq_v1=s0 == s1, eq_v2=s0 == s2)
    if not ok12:
        FAILS.append("g34.anchor12")
    # 锚③:mixed v1/v2(a 判据 patched 行;b 判据 host 对拍 bin 内建 fail-closed = rc)
    mixed = [str(G34), "--frames", "12", "--warmup", "2", "--tier", "100", "--full",
             "--slab-table", SLAB, "--auto-move", "orbit", "--hidden",
             "--cluster-lod", "on", "--cluster-error-px", "4.0",
             "--wp-hlod", "on", "--wp-threshold-l0", "0.25"]
    p1 = run("g34.mixed_v1", mixed + ["--cluster-pack", PACK_V1, "--wp-pack", WPPK_V1,
                                      "--evidence", str(out / "g34_mixed_v1.json")])
    p2 = run("g34.mixed_v2", mixed + ["--cluster-pack", PACK_V2, "--wp-pack", WPPK_V2,
                                      "--evidence", str(out / "g34_mixed_v2.json")])
    pat = re.compile(r"patched\s*=?\s*(\d+)")
    m1 = pat.findall(p1.stdout + p1.stderr)
    m2 = pat.findall(p2.stdout + p2.stderr)
    # v2 判据 = 无 patched 行(g34_full_lane.rs L1936 `if patched > 0` 才打印)或显式 0
    # ——首版误设"必须抓到 patched=0 行",与打印实现语义不符,已修。
    ok3 = bool(m1) and int(m1[-1]) > 0 and (not m2 or int(m2[-1]) == 0)
    log("g34.anchor3_patched", status="OK" if ok3 else "FAIL",
        v1_patched=m1[-1] if m1 else None, v2_patched=(m2[-1] if m2 else "no_line=0"),
        note="a判据:v1>0 旧语义 / v2 无行=0 退役面兑现;b判据=host对拍bin内建(rc);c判据bbox方差归质量登记面")
    if not ok3:
        FAILS.append("g34.anchor3")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="all", choices=["all", "t3", "fif", "dyn", "g34"])
    a = ap.parse_args()
    segs = {"t3": seg_t3, "fif": seg_fif, "dyn": seg_dyn, "g34": seg_g34}
    todo = list(segs) if a.only == "all" else [a.only]
    for exe in (FCP, WIN, PERF, FIFP, G34):
        if not exe.exists():
            print(f"FAIL: 缺 exe {exe}(先 release 构建)")
            return 2
    with gpu_device_lock(purpose="G38 批次1(T3/T2/T4 验收)"):
        for name in todo:
            log(f"seg.{name}", status="BEGIN")
            fails_before = len(FAILS)
            try:
                segs[name]()
            except Exception as ex:  # 段级兜底:异常如实登记不吞
                log(f"seg.{name}", status=f"EXC {type(ex).__name__}: {ex}")
                FAILS.append(f"{name}.exc")
            log(f"seg.{name}", status="END",
                seg_fails=FAILS[fails_before:])
    log("batch1", status="DONE", fails=FAILS)
    print(("BATCH1 PASS" if not FAILS else f"BATCH1 FAILS: {FAILS}"))
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
