#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# SPIKE(RD-027):RD-027 毒径判别 spike 共享助手——不入 src/ 生产路径、不随产品编译、
# 探针可弃(G3_CONTRACT §7 / G3_PLAN §2;探针纪律镜像 spike/dxil-path-probe/_common.py)。
"""RD-027 spike 共享层:变体构建(copytree+patch,原树 0-byte)+ 看门狗运行
(bench/proc_guard,禁裸 subprocess,R-606)+ GPU 签名采样(nvidia-smi 伴随线程)
+ 金丝雀门 + campaign JSONL 增量落盘(崩溃不丢已测事实)。

全部工件落 build/spike-rd027/(ASCII 路径,gitignore 区;ptxas 非 ASCII 崩溃先例 r6
+ 本机用户目录含非 ASCII,故不用 %TEMP%)。
"""
from __future__ import annotations

import datetime
import hashlib
import json
import os
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from bench import env_probe  # noqa: E402
from bench.proc_guard import guarded_run  # noqa: E402

RX = ROOT / "target" / "debug" / ("rx.exe" if os.name == "nt" else "rx")
SRC = ROOT / "apps" / "ruridrop" / "src"
WORK = ROOT / "build" / "spike-rd027"          # gitignore 区(build/),ASCII
QUARANTINE = ROOT / "build" / "quarantine"
CAMPAIGN_LOG = WORK / "campaign.jsonl"

POISON_TIMEOUT = 120     # 秒:毒径判定线(绿档实测 ~1s 量级,余量 >100×)
BUILD_TIMEOUT = 1800     # 秒:rx build(含 ptxas AOT)
CANARY_TIMEOUT = 90      # 秒:金丝雀(已知绿基准须秒级复绿)


def log(msg: str) -> None:
    print(f"[rd027-spike] {msg}", flush=True)


def fail(msg: str) -> None:
    print(f"[rd027-spike] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    h.update(p.read_bytes())
    return h.hexdigest()


def append_jsonl(record: dict) -> None:
    """增量落盘(二进制写,LF;崩溃/断电不丢已测事实)。"""
    WORK.mkdir(parents=True, exist_ok=True)
    record = dict(record)
    record.setdefault("ts", datetime.datetime.now().astimezone().isoformat())
    with open(CAMPAIGN_LOG, "ab") as f:
        f.write((json.dumps(record, ensure_ascii=False) + "\n").encode("utf-8"))


def nvsmi_sample() -> dict | None:
    """单次 nvidia-smi 采样(util%/功耗W/温度C/SM时钟MHz);失败返回 None 不抛。"""
    try:
        r = subprocess.run(
            ["nvidia-smi", "--query-gpu=utilization.gpu,power.draw,temperature.gpu,clocks.sm",
             "--format=csv,noheader,nounits"],
            capture_output=True, timeout=15)
        parts = r.stdout.decode("utf-8", errors="replace").strip().split(",")
        if r.returncode != 0 or len(parts) < 4:
            return None
        return {
            "util_pct": float(parts[0]),
            "power_w": float(parts[1]),
            "temp_c": float(parts[2]),
            "sm_clock_mhz": float(parts[3]),
        }
    except Exception:
        return None


class GpuSampler:
    """伴随采样线程:运行期间每 interval 秒采一次,保留全序列(判 54W 自旋签名)。"""

    def __init__(self, interval: float = 10.0):
        self.interval = interval
        self.samples: list[dict] = []
        self._stop = threading.Event()
        self._t = threading.Thread(target=self._loop, daemon=True)

    def _loop(self):
        while not self._stop.wait(self.interval):
            s = nvsmi_sample()
            if s is not None:
                s["t_offset_s"] = round(time.perf_counter() - self._t0, 1)
                self.samples.append(s)

    def __enter__(self):
        self._t0 = time.perf_counter()
        self._t.start()
        return self

    def __exit__(self, *exc):
        self._stop.set()
        self._t.join(timeout=5)
        return False


def no_ptxas_env() -> dict:
    """构建期强制 PTX-only 的环境(E0b JIT 腿):locate_ptxas 三候选全灭——
    RURIXC_PTXAS 指向不存在路径 + 删 CUDA_PATH + PATH 清洗掉含 CUDA 的目录。"""
    env = os.environ.copy()
    env["RURIXC_PTXAS"] = str(WORK / "nonexistent_ptxas.exe")
    env.pop("CUDA_PATH", None)
    env["PATH"] = os.pathsep.join(
        p for p in env.get("PATH", "").split(os.pathsep)
        if "cuda" not in p.lower())
    return env


def build_variant(name: str, patches: list[tuple[str, str, str]],
                  entry: str = "offline", env: dict | None = None) -> dict:
    """copytree apps/ruridrop/src → build/spike-rd027/variants/<name>/,打锚定补丁,
    rx build <entry>.rx。锚点缺失即 fail(防静默测错档,uc07_bench 先例)。
    返回 {exe, ptx_sha256, ll_sha256, build_wall_s, build_stderr_tail}。"""
    vdir = WORK / "variants" / name
    if vdir.exists():
        shutil.rmtree(vdir)
    vdir.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(SRC, vdir)
    for fname, anchor, replacement in patches:
        p = vdir / fname
        text = p.read_bytes().decode("utf-8")
        if anchor not in text:
            fail(f"变体 {name} 锚点缺失({fname}): {anchor!r}")
        with open(p, "wb") as f:
            f.write(text.replace(anchor, replacement).encode("utf-8"))
    exe = WORK / "bin" / f"{name}.exe"
    exe.parent.mkdir(parents=True, exist_ok=True)
    if not RX.is_file():
        fail(f"rx 不存在({RX});先 cargo build -p rurixc -p rx")
    t0 = time.perf_counter()
    r = guarded_run([RX, "build", vdir / f"{entry}.rx", "-o", exe],
                    timeout=BUILD_TIMEOUT, label=f"rx-build:{name}", env=env)
    wall = time.perf_counter() - t0
    if r.returncode != 0:
        fail(f"变体 {name} rx build 失败(exit={r.returncode}):\n{(r.stdout + r.stderr)[-800:]}")
    if not exe.is_file() or exe.stat().st_size == 0:
        fail(f"变体 {name} EXE 缺失或为空: {exe}")
    ptx = exe.with_suffix(".ptx")
    ll = exe.with_suffix(".dev.ll")
    return {
        "exe": exe,
        "ptx_sha256": sha256_file(ptx) if ptx.is_file() else None,
        "ll_sha256": sha256_file(ll) if ll.is_file() else None,
        "build_wall_s": round(wall, 2),
        "build_stderr_tail": (r.stdout + r.stderr)[-400:],
    }


def run_variant(name: str, exe: Path, timeout: int = POISON_TIMEOUT,
                expect: str = "unknown") -> dict:
    """看门狗跑一个变体 exe(cwd=独立 rundir,PPM 落该处);伴随 GPU 采样。
    返回记录 dict(已 append 进 campaign JSONL)。"""
    rundir = WORK / "runs" / f"{name}_{datetime.datetime.now().strftime('%H%M%S')}"
    rundir.mkdir(parents=True, exist_ok=True)
    pre = nvsmi_sample()
    t0 = time.perf_counter()
    with GpuSampler(interval=10.0) as sampler:
        r = guarded_run([exe], timeout=timeout, cwd=rundir,
                        quarantine_exe=exe, quarantine_dir=QUARANTINE,
                        label=f"run:{name}")
    wall = time.perf_counter() - t0
    post = nvsmi_sample()
    classification = ("hang_timeout" if r.timed_out
                      else ("completed" if r.returncode == 0 else "error"))
    rec = {
        "kind": "run",
        "name": name,
        "expect": expect,
        "classification": classification,
        "exit_code": r.returncode,
        "wall_s": round(wall, 2),
        "timeout_s": timeout,
        "timed_out": r.timed_out,
        "quarantined": r.quarantined,
        "gpu_pre": pre,
        "gpu_during": sampler.samples,
        "gpu_post": post,
        "stdout_tail": r.stdout[-300:],
        "stderr_tail": r.stderr[-300:],
    }
    append_jsonl(rec)
    log(f"{name}: {classification} exit={r.returncode} wall={wall:.1f}s "
        f"(expect={expect}; during_samples={len(sampler.samples)})")
    return rec


def canary(green_exe: Path) -> bool:
    """金丝雀门:毒径挂起判定后必跑——已知绿基准须在 CANARY_TIMEOUT 内复绿,
    否则 GPU 态疑污染,campaign 必须中止(Godot 连环污染先例)。"""
    smi_ok = nvsmi_sample() is not None
    rec = run_variant("canary", green_exe, timeout=CANARY_TIMEOUT, expect="green")
    ok = smi_ok and rec["classification"] == "completed"
    append_jsonl({"kind": "canary_verdict", "nvsmi_responsive": smi_ok, "ok": ok})
    if not ok:
        log("CANARY FAILED — GPU 态疑污染,中止 campaign(不带污染态采信后续结果)")
    return ok


def campaign_header(round_id: str, note: str) -> None:
    env = env_probe.collect_environment()
    append_jsonl({"kind": "header", "round": round_id, "note": note, "environment": env})
    log(f"campaign round={round_id} env: {env.get('gpu_name')} driver="
        f"{env.get('driver_version')} cuda={env.get('cuda_driver_version')}")
