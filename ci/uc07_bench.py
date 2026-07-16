#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-07 生产档性能取证采集器(MS1.4,契约 G-MS1-5/G-MS1-6;RFC-0010 §4.5/§4.6)。

**操作者工具,不进 CI**(实时窗口需交互桌面,镜像 ci/realtime_present_smoke.py
双态先例的 evidence 面;离线/仿真档虽可无窗跑,但三项同属 MS1.4 一次性
measured_local 取证,统一本机人工链路)。

三项 bench(BENCH_PROTOCOL 三次运行规则:三次进程级独立运行,跨 run
trimmed mean;单 run 为端到端进程墙钟,非 50×3 内层协议——采样口径在
evidence sampling 节如实记,timer=wall_clock_process,经
milestones/ms1/uc07_bench_evidence_schema.json 校验):

  sph_step        ms1.bench.uc07_sph_step_ms:sim-only 生产档(N=131072,
                  4 子步/帧)。每 trial 跑 sim_bench_short(8 帧)与
                  sim_bench_long(72 帧)两 EXE,指标 =(t_long − t_short)/64
                  帧 ×1000 ms——进程墙钟差分消 startup/PTX JIT/ctx 建立开销。
  offline_frame   ms1.bench.uc07_offline_frame_s:生产档 offline(1280×720 /
                  N=131072 / 8 帧全序列)**SPP 256→32 + PT_BOUNCES 4→2 切片**
                  全程墙钟 / 8,单位秒——含 sim + PT + PPM 落盘与进程启动摊分
                  (note 如实)。切片依据(2026-07-15 估时/二分实录,毒径工件
                  归档 %TEMP% probe*):完整生产档在本机当前工具链下**不可测**
                  ——存在样本序号/弹射深度相关的毒径(poison path):720p/8spp
                  下 bounces≤2 0.6s 完成、bounces=3 挂起(>300s);bounces=2
                  下 8spp/32spp 秒级完成、256spp 挂起(>590s 零帧);同数据
                  同表 rt_primary 600 帧与 160×120 冒烟档(bounces=4)全绿——
                  源内全部循环编译期有界,疑 PTX 发散重汇聚类工具链问题,
                  按 14 §4 留 RD 跟进,不在 MS1.4 修。切片(32spp/b2/8 帧)
                  经真跑验证确定性完成,在 **tmp 拷贝**打两参补丁(镜像步骤
                  53 数据流红绿的 copytree+patch+同一 rx build 链先例,原树
                  0-byte),分辨率/N/帧数/批宽生产值不动,trials=3 维持
                  BENCH_PROTOCOL 三次运行规则。
  realtime_frame  ms1.bench.uc07_realtime_frame_ms:realtime 入口(present
                  typestate 真窗口)600 帧全程墙钟 / 600——含 present/泵/vsync
                  与进程启动(note 如实)。

  present         G-MS1-5 取证跑(非 bench):realtime 真窗口一次 ≥300 帧,
                  解析 SAMPLE_* 采样对照行 + REALTIME_OK 见证行,写
                  evidence/uc07_present_<date>.json(经
                  milestones/ms1/uc07_present_evidence_schema.json 校验)。

锁频规程(BENCH_PROTOCOL §2.1):采样前置 lock_clocks 读回校验;未锁频 →
evidence_level=unlocked 如实降级(unlocked 不得回填预算,聚合期二道防线拒绝)。
证据 JSON 一律二进制写盘(仓库 LF byte-exact 纪律)。

用法(锁频后):
  py -3 ci/uc07_bench.py sph_step|offline_frame|realtime_frame [--trials N] [--date YYYYMMDD]
  py -3 ci/uc07_bench.py present [--date YYYYMMDD]
"""
from __future__ import annotations

import datetime
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from bench import env_probe  # noqa: E402
from bench.stats import bootstrap_ci, cv, trimmed_mean  # noqa: E402

RX = ROOT / "target" / "debug" / ("rx.exe" if os.name == "nt" else "rx")
SRC = ROOT / "apps" / "ruridrop" / "src"
EVIDENCE_DIR = ROOT / "evidence"
# present-real cabi lib(独立 target-dir,防污染默认 stub 缓存;RX7021 定位序:
# env RURIX_RT_CABI_LIB 优先,realtime 构建时指向真 shim 版)
REAL_CABI_LIB = ROOT / "target" / "crt-static-real" / "release" / "rurix_rt_cabi.lib"
TRIALS_DEFAULT = 3

BENCHES = {
    "sph_step": {
        "file_id": "uc07_sph_step_ms",
        "bench_id": "uc07_sph_step",
        "metric": "sph_step_time",
        "unit": "ms",
        "problem_size": (
            "N=131072 particles, 64^3 grid, 18-bit key radix sort, 4 substeps/frame; "
            "differential (72-frame - 8-frame) / 64 frames, sim-only production profile"
        ),
    },
    "offline_frame": {
        "file_id": "uc07_offline_frame_s",
        "bench_id": "uc07_offline_frame",
        "metric": "offline_frame_time",
        "unit": "s",
        "problem_size": (
            "1280x720, N=131072, full 8-frame production sequence, 32spp (SPP "
            "256->32) and 2 bounces (PT_BOUNCES 4->2) slice patched in a tmp copy of "
            "apps/ruridrop/src, same rx build chain; resolution/N/frames/batch width "
            "at production values; whole-process wall clock / 8 (includes startup + "
            "PTX JIT + sim + PT + tonemap + PPM writes, amortized). Slice rationale: "
            "full 256spp/4-bounce profile is unmeasurable on current toolchain - "
            "sample-index/bounce-depth dependent poison paths hang pt_render "
            "(720p/8spp: bounces<=2 0.6s vs bounces=3 hang >300s; bounces=2: 32spp "
            "seconds vs 256spp hang >590s with zero frames; same data/tables fine "
            "under rt_primary 600 frames and 160x120 smoke at bounces=4; all kernel "
            "loops are compile-time bounded -> suspected toolchain-level divergence/"
            "reconvergence issue, RD follow-up)"
        ),
    },
    "realtime_frame": {
        "file_id": "uc07_realtime_frame_ms",
        "bench_id": "uc07_realtime_frame",
        "metric": "realtime_frame_time",
        "unit": "ms",
        "problem_size": (
            "1280x720, 1spp ray cast, N=131072, 4 substeps/frame, 600 frames "
            "presented to real D3D12 window; whole-process wall clock / 600 "
            "(includes present/pump/vsync + startup)"
        ),
    },
}


def fail(msg: str) -> None:
    print(f"[uc07_bench] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def run(cmd, cwd=ROOT, env=None, timeout=3600):
    r = subprocess.run(cmd, capture_output=True, cwd=cwd, env=env, timeout=timeout)
    out = r.stdout.decode("utf-8", errors="replace")
    err_text = r.stderr.decode("utf-8", errors="replace")
    return r.returncode, out, err_text


def harness_commit() -> str:
    code, out, _ = run(["git", "rev-parse", "--short", "HEAD"])
    return out.strip() if code == 0 else "unknown"


def write_json(path: Path, doc: dict) -> None:
    """二进制写盘(仓库 LF byte-exact:禁 Python 文本模式)。"""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "wb") as f:
        f.write((json.dumps(doc, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))


def rx_build(entry: str, exe: Path, env_extra: dict | None = None) -> None:
    if not RX.is_file():
        fail(f"rx 不存在({RX});先 cargo build -p rurixc -p rx")
    env = os.environ.copy()
    if env_extra:
        env.update(env_extra)
    code, out, err_text = run([str(RX), "build", str(SRC / f"{entry}.rx"), "-o", str(exe)],
                              env=env)
    if code != 0:
        fail(f"rx build {entry}.rx 失败(exit={code}):\n{(out + err_text)[-800:]}")
    if not exe.is_file() or exe.stat().st_size == 0:
        fail(f"{entry} EXE 缺失或为空: {exe}")


def ensure_real_cabi() -> None:
    """present-real cabi staticlib(真 D3D12/DXGI shim;crt-static,独立 target-dir)。"""
    if REAL_CABI_LIB.is_file():
        return
    print("[uc07_bench] building rurix-rt-cabi --features present-real (crt-static, "
          "target/crt-static-real)…")
    env = os.environ.copy()
    env["RUSTFLAGS"] = "-C target-feature=+crt-static"
    code, out, err_text = run(
        ["cargo", "build", "-p", "rurix-rt-cabi", "--release",
         "--features", "present-real", "--target-dir", "target/crt-static-real"],
        env=env,
    )
    if code != 0 or not REAL_CABI_LIB.is_file():
        fail(f"present-real cabi 构建失败(exit={code}):\n{err_text[-800:]}")


def prepare_offline_slice(td: Path) -> None:
    """offline 32spp/2 弹射切片:tmp 拷贝 src → 锚定补丁 → 同一 rx build 链
    (镜像 ci/uc07_offline_golden_smoke.py dataflow_red 的 copytree+patch 先例;
    原树 0-byte 不动)。params.rx 生产档已内联切片值(STUB(RD-027)),补丁
    锚点缺失时核验切片值在位即通过;RD-027 回填 256/4 后本函数恢复补丁语义。"""
    import shutil
    variant = td / "offline_slice_src"
    shutil.copytree(SRC, variant)
    patches = [
        ("params.rx", "pub const SPP: usize = 256;", "pub const SPP: usize = 32;"),
        ("params.rx", "pub const PT_BOUNCES: u32 = 4;", "pub const PT_BOUNCES: u32 = 2;"),
    ]
    for fname, anchor, replacement in patches:
        p = variant / fname
        text = p.read_bytes().decode("utf-8")
        if anchor in text:
            with open(p, "wb") as f:
                f.write(text.replace(anchor, replacement).encode("utf-8"))
        elif replacement in text:
            print(f"[uc07_bench] 切片值已内联(STUB(RD-027)): {replacement!r}")
        else:
            fail(f"offline 切片锚点与切片值均缺失({fname}): {anchor!r}(防静默测错档)")
    exe = td / "offline_bench_slice.exe"
    code, out, err_text = run([str(RX), "build", str(variant / "offline.rx"), "-o", str(exe)])
    if code != 0:
        fail(f"offline 切片 rx build 失败(exit={code}):\n{(out + err_text)[-800:]}")
    if not exe.is_file() or exe.stat().st_size == 0:
        fail(f"offline 切片 EXE 缺失或为空: {exe}")


def probe_env() -> dict:
    env = env_probe.collect_environment()
    env["os_build"] = env["os_build"].replace("Windows", "Windows").strip()
    return env


def timed_run(exe: Path, cwd: Path, want: str, timeout=3600) -> tuple[float, str]:
    """端到端进程墙钟(含启动;各 bench 口径注记如实)。校验见证行。"""
    t0 = time.perf_counter()
    code, out, err_text = run([str(exe)], cwd=cwd, timeout=timeout)
    wall = time.perf_counter() - t0
    if code != 0:
        fail(f"{exe.name} 运行失败(exit={code}):\n{(out + err_text)[-600:]}")
    if want not in out:
        fail(f"{exe.name} stdout 缺见证行 {want!r}:\n{out[-400:]}")
    return wall, out


def make_doc(spec: dict, level: str, env_start: dict, temp_end, value: float,
             sampling: dict, results_extra: dict, notes: str) -> dict:
    env = json.loads(json.dumps(env_start))
    temp_start = env["thermal"]["temp_start_c"]
    env["thermal"] = {
        "temp_start_c": temp_start,
        "temp_end_c": temp_end,
        "steady_state": isinstance(temp_end, int) and abs(temp_end - temp_start) <= 4,
    }
    results = {
        "metric": spec["metric"],
        "unit": spec["unit"],
        "trimmed_mean": round(value, 4),
        "correctness_check": "pass",
    }
    results.update(results_extra)
    return {
        "schema_version": 1,
        "evidence_level": level,
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "bench": {
            "id": spec["bench_id"],
            "level": "L4",
            "problem_size": spec["problem_size"],
            "harness_commit": harness_commit(),
        },
        "environment": env,
        "sampling": sampling,
        "results": results,
        "notes": notes,
    }


def evidence_level(env: dict) -> str:
    """BENCH_PROTOCOL §2.1:锁频降级规则——未锁频照常采样但标 unlocked,
    不得回填预算(聚合期二道防线再拒)。"""
    return "measured_local" if env["clocks"]["locked"] else "unlocked"


def bench_trial(kind: str, seq: int, td: Path, date: str) -> float:
    spec = BENCHES[kind]
    env_start = probe_env()
    level = evidence_level(env_start)
    if level != "measured_local":
        print(f"[uc07_bench] WARN 未锁频(SM {env_start['clocks']['sm_clock_mhz']} MHz)"
              f"→ 本 trial evidence_level=unlocked(不得回填,BENCH_PROTOCOL §2.1)")

    if kind == "sph_step":
        short_exe, long_exe = td / "sim_bench_short.exe", td / "sim_bench_long.exe"
        t_short, _ = timed_run(short_exe, td, "SIM_BENCH_OK frames=8")
        t_long, _ = timed_run(long_exe, td, "SIM_BENCH_OK frames=72")
        value = (t_long - t_short) / 64.0 * 1000.0
        extra = {"raw": {"t_short_s": round(t_short, 4), "t_long_s": round(t_long, 4)}}
        sampling_method = (
            "process-level wall-clock differential: (t_long(72f) - t_short(8f)) / 64 frames; "
            "startup/PTX-JIT/context overhead cancels in subtraction; per-frame stream sync"
        )
        notes = (
            "sim-only production profile (no rendering): 4 substeps/frame x (cell_key + "
            "18-bit bit-split radix sort + reorder + cell_bounds + density + forces + "
            "integrate); NaN/in-box check in both entries (cancels in differential). "
            "RFC-0010 §4.6 ms1.bench.uc07_sph_step_ms"
        )
        witness = f"t_short={t_short:.3f}s t_long={t_long:.3f}s"
    elif kind == "offline_frame":
        exe = td / "offline_bench_slice.exe"
        workdir = td / f"offline_run{seq}"
        workdir.mkdir(parents=True, exist_ok=True)
        wall, _ = timed_run(exe, workdir, "RENDER_OK frames=8", timeout=1800)
        value = wall / 8.0
        extra = {"raw": {"wall_s": round(wall, 4), "frames": 8, "spp": 32, "bounces": 2}}
        sampling_method = (
            "process-level wall clock of one full 8-frame production-sequence run / 8; "
            "includes process startup + PTX JIT + sim (4 substeps/frame) + PT (32spp, "
            "one 32-wide batch) + tonemap + 8 PPM disk writes (recorded honestly, "
            "amortized, not frame-isolated); SPP 256->32 / PT_BOUNCES 4->2 slice "
            "patched in tmp source copy, same rx build chain (mirror of step-53 "
            "dataflow red/green copytree+patch precedent); slice verified to complete "
            "deterministically - full 256spp/4-bounce profile hangs on poison paths "
            "(see bench.problem_size)"
        )
        notes = (
            "production offline path tracing 1280x720/N=131072/8 frames "
            "(frame_0000..0007.ppm written to trial workdir under %TEMP%), 32spp + "
            "2-bounce measurable slice of the 256spp/4-bounce production profile; "
            "wall/8 as honest end-to-end per-frame cost of the offline path "
            "(sim + camera-jitter PT DDA + NEE shadow walks + finalize + IO all "
            "exercised). RFC-0010 §4.6 ms1.bench.uc07_offline_frame_s"
        )
        witness = f"wall={wall:.3f}s"
    else:  # realtime_frame
        exe = td / "realtime_real.exe"
        wall, out = timed_run(exe, td, "sample_ok=true", timeout=1800)
        m = re.search(r"REALTIME_OK frames=(\d+) sample_ok=true", out)
        if m is None or int(m.group(1)) < 300:
            fail(f"realtime 见证行缺失或帧数不足:\n{out[-300:]}")
        frames = int(m.group(1))
        value = wall / frames * 1000.0
        extra = {"raw": {"wall_s": round(wall, 4), "frames": frames}}
        sampling_method = (
            "process-level wall clock of one 600-frame real-window presented run / 600; "
            "includes present typestate loop (wait/signal/pump/present via "
            "d3d12-interop-real shim), window pump and any vsync/present pacing + "
            "process startup + PTX JIT (recorded honestly, not frame-isolated)"
        )
        notes = (
            "realtime ray-cast production profile presented to a real D3D12 window "
            "(RXS-0197/0198 typestate loop); end-of-run sample verification "
            "(sky/water regions) passed inside the EXE (REALTIME_OK sample_ok=true). "
            "RFC-0010 §4.6 ms1.bench.uc07_realtime_frame_ms"
        )
        witness = f"wall={wall:.3f}s frames={frames}"

    temp_end = probe_env()["thermal"]["temp_start_c"]
    sampling = {
        "trials": 1,
        "trimmed_pct": 0.0,
        "timer": "wall_clock_process",
        "method": sampling_method,
    }
    doc = make_doc(spec, level, env_start, temp_end, value, sampling, extra, notes)
    out_path = EVIDENCE_DIR / f"{spec['file_id']}_{date}_{seq}.json"
    write_json(out_path, doc)
    print(f"[uc07_bench] {kind} trial {seq}: {value:.4f} {spec['unit']} ({witness}) "
          f"→ {out_path.relative_to(ROOT)} [{level}]")
    return value


def aggregate(kind: str, date: str) -> None:
    spec = BENCHES[kind]
    docs = []
    for seq in range(1, TRIALS_DEFAULT + 1):
        p = EVIDENCE_DIR / f"{spec['file_id']}_{date}_{seq}.json"
        if not p.is_file():
            fail(f"聚合缺 trial 文件: {p}")
        docs.append(json.loads(p.read_text(encoding="utf-8")))
    levels = {d["evidence_level"] for d in docs}
    if levels != {"measured_local"}:
        fail(f"存在非 measured_local trial({levels}),整组作废"
             "(BENCH_PROTOCOL §2.1,unlocked 不得回填)")
    values = [d["results"]["trimmed_mean"] for d in docs]
    agg_value = trimmed_mean(values, 0.2)
    ci_lo, ci_hi = bootstrap_ci(values, statistic="mean")
    agg = json.loads(json.dumps(docs[0]))
    agg["timestamp"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    agg["sampling"] = {
        "trials": TRIALS_DEFAULT,
        "trimmed_pct": 0.2,
        "timer": "wall_clock_process",
        "method": docs[0]["sampling"]["method"],
    }
    agg["results"]["trial_values"] = [round(v, 4) for v in values]
    agg["results"]["trimmed_mean"] = round(agg_value, 4)
    agg["results"]["cv"] = round(cv(values), 6)
    agg["results"]["ci95"] = [round(ci_lo, 4), round(ci_hi, 4)]
    agg["results"]["min"] = round(min(values), 4)
    agg["results"]["max"] = round(max(values), 4)
    agg["results"].pop("raw", None)
    agg["notes"] = (
        f"aggregate of {TRIALS_DEFAULT} process-level independent runs "
        f"({spec['file_id']}_{date}_1..{TRIALS_DEFAULT}.json); trial_values = per-run "
        "end-to-end wall-clock values. " + agg.get("notes", "")
    )
    out_path = EVIDENCE_DIR / f"{spec['file_id']}_{date}_agg.json"
    write_json(out_path, agg)
    print(f"[uc07_bench] {kind} aggregate: {agg_value:.4f} {spec['unit']} "
          f"(trials={values}) → {out_path.relative_to(ROOT)}")


def run_bench(kind: str, date: str, trials: int) -> None:
    spec = BENCHES[kind]
    with tempfile.TemporaryDirectory(prefix=f"uc07_bench_{kind}_") as tdname:
        td = Path(tdname)
        if kind == "sph_step":
            rx_build("sim_bench_short", td / "sim_bench_short.exe")
            rx_build("sim_bench_long", td / "sim_bench_long.exe")
        elif kind == "offline_frame":
            prepare_offline_slice(td)
        else:
            ensure_real_cabi()
            rx_build("realtime", td / "realtime_real.exe",
                     {"RURIX_RT_CABI_LIB": str(REAL_CABI_LIB)})
        for seq in range(1, trials + 1):
            bench_trial(kind, seq, td, date)
    if trials == TRIALS_DEFAULT:
        aggregate(kind, date)
    else:
        print(f"[uc07_bench] trials={trials} ≠ {TRIALS_DEFAULT},跳过聚合(如实留单 trial)")


def run_present(date: str) -> None:
    """G-MS1-5 取证跑:realtime 真窗口 ≥300 帧 + 采样对照 + 环境画像。"""
    ensure_real_cabi()
    env_start = probe_env()
    with tempfile.TemporaryDirectory(prefix="uc07_present_") as tdname:
        td = Path(tdname)
        exe = td / "realtime_real.exe"
        rx_build("realtime", exe, {"RURIX_RT_CABI_LIB": str(REAL_CABI_LIB)})
        t0 = time.perf_counter()
        code, out, err_text = run([str(exe)], cwd=td, timeout=1800)
        wall = time.perf_counter() - t0
    print(out.strip())
    if code != 0:
        fail(f"realtime 取证跑失败(exit={code}):\n{(out + err_text)[-600:]}")
    m_ok = re.search(r"REALTIME_OK frames=(\d+) sample_ok=true", out)
    m_stats = re.search(r"SAMPLE_STATS min=(\d+) max=(\d+) mean=(\d+)", out)
    m_sky = re.search(r"SAMPLE_SKY (\d+) (\d+) (\d+) \| (\d+) (\d+) (\d+)", out)
    m_water = re.search(r"SAMPLE_WATER (\d+) (\d+) (\d+) \| (\d+) (\d+) (\d+)", out)
    if not (m_ok and m_stats and m_sky and m_water):
        fail(f"取证输出缺见证/采样行:\n{out[-500:]}")
    frames = int(m_ok.group(1))
    if frames < 300:
        fail(f"帧数 {frames} < 300(G-MS1-5)")
    temp_end = probe_env()["thermal"]["temp_start_c"]
    sky = [int(x) for x in m_sky.groups()]
    water = [int(x) for x in m_water.groups()]
    doc = {
        "schema_version": 1,
        "kind": "uc07_present",
        "date": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
        "frames": frames,
        "sample_ok": True,
        "present_path": "d3d12-interop-real",
        "run_seconds": round(wall, 3),
        "run_command": (
            "cargo build -p rurix-rt-cabi --release --features present-real "
            "--target-dir target/crt-static-real (RUSTFLAGS=-C target-feature=+crt-static); "
            "RURIX_RT_CABI_LIB=target/crt-static-real/release/rurix_rt_cabi.lib "
            "rx build apps/ruridrop/src/realtime.rx -o <tmp>; run <tmp> (opens window)"
        ),
        "samples": {
            "stats": {
                "min": int(m_stats.group(1)),
                "max": int(m_stats.group(2)),
                "mean": int(m_stats.group(3)),
            },
            "sky": [sky[0:3], sky[3:6]],
            "water": [water[0:3], water[3:6]],
        },
        "environment": {
            "gpu_name": env_start["gpu_name"],
            "driver_version": env_start["driver_version"],
            "cuda_driver_version": env_start["cuda_driver_version"],
            "driver_model": env_start["driver_model"],
            "os_build": env_start["os_build"],
            "clocks_locked": env_start["clocks"]["locked"],
            "temp_start_c": env_start["thermal"]["temp_start_c"],
            "temp_end_c": temp_end,
        },
        "notes": (
            "G-MS1-5 realtime present 取证(RFC-0010 §4.5):ruridrop realtime 入口经 "
            "RXS-0197/0198 present typestate 帧循环(wait→backbuffer→rt_primary 直写→"
            "signal→pump→present,RFC-0001 fence 协议面经 rxp_* C ABI/d3d12-interop-real "
            "C++ shim,D-130 0-byte)真窗口跑满 MAX_FRAMES;循环结束后同一末帧排序态 "
            "rt_primary 重渲进普通 Buffer(非 backbuffer)download 采样:全域 min/max/"
            "mean 粗测 + 天空区顶行两点(确定性天空梯度,蓝>红)+ 水体区两点(初始"
            "水柱脚印内世界点投影,速度基色蓝主导)范围核验全过(EXE 内自校验,"
            "失败退出非 0)。evidence 面不进 CI 硬门(SKIP 不充绿,双态先例)"
        ),
    }
    out_path = EVIDENCE_DIR / f"uc07_present_{date}.json"
    write_json(out_path, doc)
    print(f"[uc07_bench] present evidence → {out_path.relative_to(ROOT)}"
          f"(frames={frames} sample_ok=true wall={wall:.3f}s)")


def main() -> int:
    args = sys.argv[1:]
    if not args or args[0] not in (*BENCHES, "present"):
        print(__doc__)
        return 2
    kind = args[0]
    date = datetime.date.today().strftime("%Y%m%d")
    trials = TRIALS_DEFAULT
    i = 1
    while i < len(args):
        if args[i] == "--date" and i + 1 < len(args):
            date = args[i + 1]
            i += 2
        elif args[i] == "--trials" and i + 1 < len(args):
            trials = int(args[i + 1])
            i += 2
        else:
            fail(f"未知参数: {args[i]}")
    if kind == "present":
        run_present(date)
    else:
        run_bench(kind, date, trials)
    return 0


if __name__ == "__main__":
    sys.exit(main())
