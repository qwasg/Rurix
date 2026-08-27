#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C10 RD-027 PT 毒径定位修复）
"""G31+ 波 C Task C10 RD-027 PT 毒径回归守护门（g31.waveC.rd027）。

承接:RD-027(registry/deferred.json open)——G3.1 spike 定罪 NVIDIA 优化后段
(ptxas -O1+ 与驱动 JIT 同类变换,M1′ 机理:无记账 CALL.REL.NOINC latch 出口 →
barrier 掩码破坏 → BSYNC 死等),护栏 = MR-0011 `RURIXC_PTXAS_OPT=0`(AOT 腿)。
本门 = C10 交付的回归守护 + 毒区参数面 fail-closed 拒绝面:

1. **静态 fail-closed(host 恒跑)**:解析 apps/ruridrop/src/params.rx 生产档
   (SPP/SPP_BATCH/PT_BOUNCES),按毒区图(milestones/g31/
   g31_rd027_poison_zone_map.json,E8 全网格实测蒸馏)分类——green 放行;
   poison/unmapped 一律诚实红(**fail-closed**:未测绘组合按毒处理,禁偷偷
   扩大生产档切片进毒区/盲区)。
2. **边界绿腿(默认档真跑)**:production 切片 (32spp/b2) 与下边界 (8spp/b2)
   各 build+run(REND_FRAMES=1,60s 判定线)——必须终止 + RENDER_OK +
   frame digest == 毒区图基线(digest 漂移 = 工具链/驱动变更诚实红,促
   按生成器全网格重测毒区图)。
3. **毒区护栏腿(RURIXC_PTXAS_OPT=0)**:毒格 (8spp/b3) 与 (256spp/b2)
   护栏构建真跑——必须终止 + digest == 毒区图 O0 基线(MR-0011 护栏效力
   常驻回归;护栏失效即毒径复挂,fail-closed 不静默)。
4. **毒确认腿(默认档有界超时)**:(8spp/b3) 默认 -O3 构建跑 60s 必须
   **hang_timeout**(证毒区仍毒、毒区图未漂移;若完成 = 上游疑修复/漂移,
   诚实红促重测 + RD-027 backfill 评估)+ 挂起判定后金丝雀门(边界绿 exe
   复绿方采信,R-606/G3.1 同律)。
5. **三态纪律**:GPU(nvidia-smi)/ptxas 缺失 → DEV_ENV_DEGRADE 输出 SKIP
   (退 0,禁冒充 PASS);RURIX_REQUIRE_REAL=1 下 SKIP 翻硬 FAIL。全部 GPU
   运行经 bench/proc_guard guarded_run(超时 = 诚实红 124 + 杀进程树 +
   隔离,零裸 launch)。
6. **--selftest**:params 解析/毒区分类/静态 fail-closed/腿判定纯函数红绿臂
   + schema 互核,不依赖树上文件与 GPU。

产物:evidence/g31_rd027_poison_guard_<utc>.json(schema
milestones/g31/g31_rd027_poison_guard_evidence_schema.json)。

用法:
  py -3 ci/g31_rd027_poison_guard.py --gate
  py -3 ci/g31_rd027_poison_guard.py --selftest
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "ci"))

from bench import env_probe  # noqa: E402
from bench.proc_guard import guarded_run  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

TAG = "g31_rd027_poison_guard"
GATE_KEY = "g31.waveC.rd027"
SCHEMA_ID = "rurix.g31.rd027_poison_guard_evidence.v1"
SCHEMA_PATH = ROOT / "milestones/g31/g31_rd027_poison_guard_evidence_schema.json"
ZONE_MAP_PATH = ROOT / "milestones/g31/g31_rd027_poison_zone_map.json"
ZONE_MAP_REL = "milestones/g31/g31_rd027_poison_zone_map.json"
# 红臂测试钩(仅测试):RURIX_G31_RD027_PARAMS_PATH 指向夹具 params 时静态面判夹具;
# 生产/门真跑恒不设 → 缺省路径 0-byte。
PARAMS_PATH = Path(os.environ.get("RURIX_G31_RD027_PARAMS_PATH",
                                  str(ROOT / "apps/ruridrop/src/params.rx")))
SRC = ROOT / "apps/ruridrop/src"
RX = ROOT / "target" / "debug" / ("rx.exe" if os.name == "nt" else "rx")
WORK = ROOT / ".tmp" / "g31_rd027_guard"

RUN_TIMEOUT = 60        # 秒:毒格判定线(绿档实测 ~1s 量级,余量 ≥30×;与毒区图 basis 一致)
BUILD_TIMEOUT = 1800    # 秒:rx build(含 ptxas AOT)
CANARY_TIMEOUT = 90     # 秒:金丝雀(已知绿基准秒级复绿)

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


# ---------------------------------------------------------------------------
# 纯函数面(--selftest 同消费)
# ---------------------------------------------------------------------------
_CONST_RE = {
    "spp": re.compile(r"pub const SPP: usize = (\d+);"),
    "spp_batch": re.compile(r"pub const SPP_BATCH: usize = (\d+);"),
    "bounces": re.compile(r"pub const PT_BOUNCES: u32 = (\d+);"),
    "rend_frames": re.compile(r"pub const REND_FRAMES: usize = (\d+);"),
}


def parse_params_rx(text: str) -> dict | None:
    """params.rx 四项毒区面常量解析;任一缺失/非法 → None(防静默测错档)。"""
    out: dict[str, int] = {}
    for key, rx in _CONST_RE.items():
        m = rx.search(text)
        if not m:
            return None
        out[key] = int(m.group(1))
    return out


def zone_class(zone_doc: dict, spp: int, bounces: int) -> str:
    """毒区分类(实测网格闭集):cells_default_opt 中 completed → green;
    hang_timeout → poison;未测绘组合 → unmapped(fail-closed 按毒处理)。"""
    for cell in zone_doc.get("cells_default_opt", []):
        if cell.get("spp") == spp and cell.get("bounces") == bounces:
            cls = cell.get("classification")
            if cls == "completed":
                return "green"
            if cls == "hang_timeout":
                return "poison"
            return "unmapped"  # error 第三态不入闭集(生成器拒写在案,防御)
    return "unmapped"


def classify_static(params: dict | None, zone_doc: dict, stub_present: bool) -> tuple[str, bool, list[str]]:
    """静态 fail-closed 判定:返回 (zone_class, allowed, fails)。"""
    fails: list[str] = []
    if params is None:
        return "unmapped", False, ["params.rx 四项常量解析失败(防静默测错档)"]
    if params["spp"] % params["spp_batch"] != 0:
        fails.append(f"SPP {params['spp']} % SPP_BATCH {params['spp_batch']} ≠ 0(入口不变式破坏)")
    zc = zone_class(zone_doc, params["spp"], params["bounces"])
    allowed = zc == "green" and not fails
    if zc == "poison":
        fails.append(
            f"生产档 ({params['spp']}spp/b{params['bounces']}) 命中毒区(fail-closed 拒;"
            "须 RURIXC_PTXAS_OPT=0 护栏档或回退绿区——RD-027)")
    elif zc == "unmapped":
        fails.append(
            f"生产档 ({params['spp']}spp/b{params['bounces']}) 未测绘(fail-closed 按毒拒;"
            "须 spike/rd027-pt-poison/run_e8_zone.py 全网格重测扩图)")
    if zc != "green" and stub_present:
        fails.append("STUB(RD-027) 锚在而参数越绿区——锁注释与取值不一致")
    return zc, allowed, fails


def find_cell(zone_doc: dict, spp: int, bounces: int, opt_leg: str) -> dict | None:
    """取毒区图基线格(opt_leg: default → cells_default_opt;O0 → cells_guardrail_O0)。"""
    key = "cells_default_opt" if opt_leg == "default" else "cells_guardrail_O0"
    for cell in zone_doc.get(key, []):
        if cell.get("spp") == spp and cell.get("bounces") == bounces:
            return cell
    return None


def classify_leg(leg: dict, expect: str, expect_digest: str | None) -> list[str]:
    """腿判据(返回失败串列表,空 = 绿;--selftest 合成夹具同消费)。"""
    fails: list[str] = []
    cls = leg.get("classification")
    if cls != expect:
        fails.append(f"classification {cls!r} ≠ 期望 {expect!r}")
    if expect == "completed":
        if leg.get("exit_code") != 0:
            fails.append(f"exit_code {leg.get('exit_code')} ≠ 0")
        if leg.get("render_ok") is not True:
            fails.append("缺 RENDER_OK 见证行")
        if expect_digest is None:
            fails.append("毒区图无本格 digest 基线(防静默测错档)")
        elif leg.get("digest") != expect_digest:
            fails.append(f"digest 漂移: actual={leg.get('digest')} ≠ baseline={expect_digest}"
                         "(工具链/驱动变更 → 按生成器重测毒区图)")
    else:  # hang_timeout 期望腿:完成 = 毒区漂移
        if cls == "completed":
            fails.append("毒格完成——毒区漂移(上游疑修复;须全网格重测 + RD-027 backfill 评估)")
        if leg.get("canary_ok") is not True:
            fails.append("挂起判定后金丝雀未复绿(GPU 态疑污染,不采信)")
    return fails


def validate_zone_map(doc: dict) -> list[str]:
    """毒区图完整性判据(分类事实源不可信即全门不可信)。"""
    fails: list[str] = []
    if doc.get("kind") != "g31_rd027_poison_zone_map":
        fails.append(f"kind {doc.get('kind')!r} ≠ g31_rd027_poison_zone_map")
    if doc.get("schema_version") != 1:
        fails.append(f"schema_version {doc.get('schema_version')!r} ≠ 1")
    if doc.get("rd_ref") != "RD-027":
        fails.append(f"rd_ref {doc.get('rd_ref')!r} ≠ RD-027")
    sa = doc.get("single_artifact") or {}
    if sa.get("distinct_ptx_digests") != 1:
        fails.append(f"distinct_ptx_digests {sa.get('distinct_ptx_digests')!r} ≠ 1(单 artifact 事实漂移)")
    for key in ("cells_default_opt", "cells_guardrail_O0"):
        for cell in doc.get(key, []):
            if cell.get("classification") not in ("completed", "hang_timeout"):
                fails.append(f"{key} 含第三态 {cell.get('classification')!r}(error 不入闭集)")
    if not doc.get("cells_default_opt"):
        fails.append("cells_default_opt 空")
    return fails


# ---------------------------------------------------------------------------
# 构建/运行面(copytree+patch+guarded_run;镜像 spike_common 纪律,guard 自包含)
# ---------------------------------------------------------------------------
def sha256_file(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def patches_for(spp: int, bounces: int) -> list[tuple[str, str, str]]:
    batch = min(spp, 32)
    return [
        ("params.rx", "pub const SPP: usize = 32;", f"pub const SPP: usize = {spp};"),
        ("params.rx", "pub const SPP_BATCH: usize = 32;",
         f"pub const SPP_BATCH: usize = {batch};"),
        ("params.rx", "pub const REND_FRAMES: usize = 8;",
         "pub const REND_FRAMES: usize = 1;"),
        ("params.rx", "pub const PT_BOUNCES: u32 = 2;",
         f"pub const PT_BOUNCES: u32 = {bounces};"),
    ]


def build_leg_exe(name: str, spp: int, bounces: int, opt_level: int | None) -> Path | None:
    """tmp copytree src → 锚定补丁 → rx build(opt_level None=缺省 0-byte;0=护栏)。"""
    vdir = WORK / "variants" / name
    if vdir.exists():
        shutil.rmtree(vdir)
    vdir.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(SRC, vdir)
    for fname, anchor, replacement in patches_for(spp, bounces):
        p = vdir / fname
        text = p.read_bytes().decode("utf-8")
        if anchor not in text:
            check(False, f"腿 {name} 锚点缺失({fname}): {anchor!r}(防静默测错档)")
            return None
        with open(p, "wb") as f:
            f.write(text.replace(anchor, replacement).encode("utf-8"))
    exe = WORK / "bin" / f"{name}.exe"
    exe.parent.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.pop("RURIXC_PTXAS_OPT", None)          # 默认档确定性 0-byte(防外部环境渗入)
    if opt_level is not None:
        env["RURIXC_PTXAS_OPT"] = str(opt_level)
    print(f"[{TAG}] $ rx build {name} (opt={'default' if opt_level is None else f'-O{opt_level}'})",
          flush=True)
    r = guarded_run([RX, "build", vdir / "offline.rx", "-o", exe],
                    timeout=BUILD_TIMEOUT, label=f"rx-build:{name}", env=env)
    if r.returncode != 0 or not exe.is_file() or exe.stat().st_size == 0:
        check(False, f"腿 {name} rx build 失败(exit={r.returncode}): {(r.stdout + r.stderr)[-400:]}")
        return None
    return exe


def run_leg_exe(name: str, exe: Path, timeout: int) -> dict:
    """guarded_run 一条腿(独立 rundir;完成腿取 frame digest + RENDER_OK 见证)。"""
    rundir = WORK / "runs" / f"{name}_{datetime.datetime.now().strftime('%H%M%S')}"
    rundir.mkdir(parents=True, exist_ok=True)
    t0 = time.perf_counter()
    r = guarded_run([exe], timeout=timeout, cwd=rundir,
                    quarantine_exe=exe, quarantine_dir=ROOT / "build" / "quarantine",
                    label=f"{TAG}:{name}")
    wall = time.perf_counter() - t0
    classification = ("hang_timeout" if r.timed_out
                      else ("completed" if r.returncode == 0 else "error"))
    ppm = rundir / "frame_0000.ppm"
    digest = (sha256_file(ppm) if classification == "completed" and ppm.is_file() else None)
    rec = {
        "classification": classification,
        "exit_code": r.returncode,
        "wall_s": round(wall, 3),
        "render_ok": "RENDER_OK frames=8" in r.stdout,  # 入口打印字面恒 frames=8(如实)
        "digest": digest,
        "stdout_tail": r.stdout[-200:],
    }
    note(f"{name}: {classification} exit={r.returncode} wall={wall:.1f}s "
         f"digest={(digest or '—')[:16]}")
    return rec


def ptxas_version() -> str | None:
    try:
        r = subprocess.run(["ptxas", "--version"], capture_output=True, text=True, timeout=15)
        if r.returncode != 0:
            return None
        for line in (r.stdout + r.stderr).splitlines():
            if "release" in line:
                return line.strip()
    except Exception:
        return None
    return "unknown"


def ensure_rx() -> bool:
    if RX.is_file():
        return True
    note(f"rx 缺失,构建: cargo build -p rurixc -p rx")
    r = guarded_run(["cargo", "build", "-p", "rurixc", "-p", "rx", "--quiet"],
                    timeout=BUILD_TIMEOUT, cwd=ROOT, label="cargo-build:rx")
    return r.returncode == 0 and RX.is_file()


# ---------------------------------------------------------------------------
# 门
# ---------------------------------------------------------------------------
LEG_SPECS = {
    # name: (spp, bounces, opt_level(None=default), expect)
    "boundary_prod": (32, 2, None, "completed"),
    "boundary_low": (8, 2, None, "completed"),
    "poison_b3_o0": (8, 3, 0, "completed"),
    "poison_256_o0": (256, 2, 0, "completed"),
    "poison_confirm": (8, 3, None, "hang_timeout"),
}


def run_gate() -> int:
    check(SCHEMA_PATH.is_file(), f"schema 缺失: {SCHEMA_PATH}")
    degrade: list[str] = []

    # ── 毒区图装载 + 完整性 ──
    zone_doc: dict = {}
    if not ZONE_MAP_PATH.is_file():
        check(False, f"毒区图缺失: {ZONE_MAP_PATH}(spike/rd027-pt-poison/run_e8_zone.py --write-map 产)")
    else:
        zone_doc = json.loads(ZONE_MAP_PATH.read_text(encoding="utf-8"))
        for m in validate_zone_map(zone_doc):
            check(False, f"毒区图完整性: {m}")

    # ── ① 静态 fail-closed(host 恒跑,device 降级也照判) ──
    params_text = PARAMS_PATH.read_text(encoding="utf-8") if PARAMS_PATH.is_file() else ""
    params = parse_params_rx(params_text)
    stub_present = "STUB(RD-027)" in params_text
    zc, allowed, sfails = ("unmapped", False, ["params.rx 缺失"]) if not params_text else \
        classify_static(params, zone_doc, stub_present)
    for m in sfails:
        check(False, f"静态 fail-closed: {m}")
    static_rec = {
        "params": params or {"spp": 0, "spp_batch": 1, "bounces": 0, "rend_frames": 1},
        "zone_class": zc,
        "stub_anchor_present": stub_present,
        "allowed": allowed,
    }
    note(f"静态 fail-closed: params={params} zone={zc} allowed={allowed} stub={stub_present}")
    if FAILURES:
        # fail-fast(字面 fail-closed):生产档参数越绿区/毒区图不可信 → 不触 GPU,
        # 诊断出 stderr,不写 evidence(静态红无 device 实测面可报)。
        note("静态面红——fail-closed 拒跑 device 腿(不触 GPU)")
        return _finish([], static_rec, zone_doc, {}, None)

    # ── 三态:device/工具链降级判定 ──
    try:
        env_info = env_probe.collect_environment()
    except Exception as e:
        env_info = {}
        degrade.append(f"env_probe 失败: {e!r}")
    if not env_info.get("gpu_name"):
        degrade.append("GPU 不在位(nvidia-smi 无响应)")
    ptxas_v = ptxas_version()
    if ptxas_v is None:
        degrade.append("ptxas 不在 PATH(护栏 AOT 腿与默认档预编均不可用)")
    if degrade:
        return _finish(degrade, static_rec, zone_doc, env_info, ptxas_v)

    with gpu_device_lock(purpose="g31 波 C RD-027 毒径守护门(边界绿腿+护栏腿+毒确认腿)"):
        if not ensure_rx():
            check(False, "rx 构建失败(cargo build -p rurixc -p rx)")
            return _finish(degrade, static_rec, zone_doc, env_info, ptxas_v)

        legs: dict[str, dict] = {}
        canary_exe: Path | None = None
        for name, (spp, b, lvl, expect) in LEG_SPECS.items():
            exe = build_leg_exe(name, spp, b, lvl)
            if exe is None:
                legs[name] = {
                    "name": name, "spp": spp, "bounces": b,
                    "opt_leg": "default" if lvl is None else "O0",
                    "expect": expect, "classification": "error",
                    "exit_code": -1, "wall_s": 0.001, "timeout_s": RUN_TIMEOUT,
                    "render_ok": False, "digest": None, "digest_match": None,
                    "canary_ok": None,
                }
                continue
            if name == "boundary_prod":
                canary_exe = exe
            rec = run_leg_exe(name, exe, RUN_TIMEOUT)
            opt_leg = "default" if lvl is None else "O0"
            cell = find_cell(zone_doc, spp, b, opt_leg)
            expect_digest = (cell or {}).get("frame_digest")
            leg = {
                "name": name, "spp": spp, "bounces": b, "opt_leg": opt_leg,
                "expect": expect, "classification": rec["classification"],
                "exit_code": rec["exit_code"], "wall_s": rec["wall_s"],
                "timeout_s": RUN_TIMEOUT, "render_ok": rec["render_ok"],
                "digest": rec["digest"],
                "digest_match": (None if expect != "completed" or rec["digest"] is None
                                 or expect_digest is None else rec["digest"] == expect_digest),
                "canary_ok": None,
            }
            # 挂起判定(期望或非期望)后金丝雀门:已知绿基准复绿方采信后续
            if rec["classification"] == "hang_timeout":
                cok = None
                if canary_exe is not None:
                    crec = run_leg_exe("canary", canary_exe, CANARY_TIMEOUT)
                    cok = crec["classification"] == "completed"
                leg["canary_ok"] = cok
                check(cok is True, f"腿 {name} 挂起后金丝雀未复绿(GPU 态疑污染)")
            for m in classify_leg(leg, expect, expect_digest):
                check(False, f"腿 {name}: {m}")
            legs[name] = leg
    return _finish(degrade, static_rec, zone_doc, env_info, ptxas_v, legs=legs)


def _finish(degrade: list[str], static_rec: dict, zone_doc: dict, env_info: dict,
            ptxas_v: str | None, legs: dict | None = None) -> int:
    if degrade:
        for d in degrade:
            print(f"[{TAG}] DEV_ENV_DEGRADE {d}")
        if require_real():
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        print(f"[{TAG}] SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0
    verdict = "PASS" if (not FAILURES and legs) else "FAIL"
    doc = {
        "schema": SCHEMA_ID,
        "gate": GATE_KEY,
        "generated_utc": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "environment": {
            "gpu_name": env_info.get("gpu_name", "unknown"),
            "driver_version": env_info.get("driver_version", "unknown"),
            "cuda_driver_version": env_info.get("cuda_driver_version", "unknown"),
            "ptxas_version": ptxas_v or "unknown",
            "os_build": env_info.get("os_build", "unknown"),
        },
        "zone_map": {
            "path": ZONE_MAP_REL,
            "generated_utc": zone_doc.get("generated_utc", "unknown"),
            "distinct_ptx_digests": (zone_doc.get("single_artifact") or {}).get("distinct_ptx_digests"),
            "green_combos": (zone_doc.get("zone_summary") or {}).get("green_combos", []),
            "poison_combos": (zone_doc.get("zone_summary") or {}).get("poison_combos", []),
        },
        "static_fail_closed": static_rec,
        "legs": legs,
        "verdict": verdict,
        "notes": NOTES,
    }
    if legs is not None:
        ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        out = ROOT / "evidence" / f"g31_rd027_poison_guard_{ts}.json"
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        print(f"[{TAG}] evidence: {out}")
    if FAILURES:
        print(f"[{TAG}] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    print(f"[{TAG}] PASS gate={GATE_KEY}(静态 fail-closed green + 边界双腿 digest 命中 + "
          f"护栏双腿终止 + 毒确认腿 hang_timeout 维持)")
    return 0


# ---------------------------------------------------------------------------
# selftest(纯函数红绿 + schema 互核;不依赖树上文件与 GPU)
# ---------------------------------------------------------------------------
def run_selftest() -> int:
    fixture = """
pub const SPP: usize = 32;
pub const SPP_BATCH: usize = 32;
// STUB(RD-027): 切片锁定
pub const REND_FRAMES: usize = 8;
pub const PT_BOUNCES: u32 = 2;
"""
    synthetic_map = {
        "kind": "g31_rd027_poison_zone_map",
        "schema_version": 1,
        "rd_ref": "RD-027",
        "single_artifact": {"distinct_ptx_digests": 1},
        "cells_default_opt": [
            {"spp": 32, "bounces": 2, "classification": "completed", "frame_digest": "a" * 64},
            {"spp": 8, "bounces": 3, "classification": "hang_timeout", "frame_digest": None},
        ],
        "cells_guardrail_O0": [
            {"spp": 8, "bounces": 3, "classification": "completed", "frame_digest": "b" * 64},
        ],
    }
    # 绿臂①:合法 params 解析。
    p = parse_params_rx(fixture)
    if p != {"spp": 32, "spp_batch": 32, "rend_frames": 8, "bounces": 2}:
        print(f"[{TAG}] selftest FAIL: params 解析错 {p}", file=sys.stderr)
        return 1
    # 红臂①:缺常量必须 None。
    if parse_params_rx("pub const SPP: usize = 32;") is not None:
        print(f"[{TAG}] selftest FAIL: 残缺 params 漏检", file=sys.stderr)
        return 1
    # 绿臂②:毒区分类三态。
    if zone_class(synthetic_map, 32, 2) != "green" or \
       zone_class(synthetic_map, 8, 3) != "poison" or \
       zone_class(synthetic_map, 64, 1) != "unmapped":
        print(f"[{TAG}] selftest FAIL: 毒区分类错", file=sys.stderr)
        return 1
    # 红臂②:毒区参数必须 fail-closed 拒;未测绘同拒。
    zc, allowed, fails = classify_static(p, synthetic_map, True)
    if zc != "green" or not allowed or fails:
        print(f"[{TAG}] selftest FAIL: 绿区参数误判红 {fails}", file=sys.stderr)
        return 1
    bad = {"spp": 8, "spp_batch": 8, "rend_frames": 1, "bounces": 3}
    zc, allowed, fails = classify_static(bad, synthetic_map, True)
    if zc != "poison" or allowed or not any("毒区" in m for m in fails):
        print(f"[{TAG}] selftest FAIL: 毒区参数漏拒", file=sys.stderr)
        return 1
    zc, allowed, fails = classify_static({"spp": 64, "spp_batch": 32, "rend_frames": 1, "bounces": 1},
                                         synthetic_map, False)
    if zc != "unmapped" or allowed or not any("未测绘" in m for m in fails):
        print(f"[{TAG}] selftest FAIL: 未测绘组合漏拒", file=sys.stderr)
        return 1
    # 绿臂③:完成腿 digest 命中判据。
    good_leg = {"classification": "completed", "exit_code": 0, "render_ok": True,
                "digest": "a" * 64}
    if classify_leg(good_leg, "completed", "a" * 64):
        print(f"[{TAG}] selftest FAIL: 合法完成腿误判红", file=sys.stderr)
        return 1
    # 红臂③:digest 漂移必须检出。
    if not classify_leg({**good_leg, "digest": "c" * 64}, "completed", "a" * 64):
        print(f"[{TAG}] selftest FAIL: digest 漂移漏检", file=sys.stderr)
        return 1
    # 红臂④:期望挂起腿完成必须检出(毒区漂移)。
    drift = {"classification": "completed", "exit_code": 0, "render_ok": True,
             "digest": "a" * 64, "canary_ok": True}
    if not classify_leg(drift, "hang_timeout", None):
        print(f"[{TAG}] selftest FAIL: 毒区漂移漏检", file=sys.stderr)
        return 1
    # 红臂⑤:挂起后金丝雀未复绿必须检出。
    hung = {"classification": "hang_timeout", "exit_code": 124, "render_ok": False,
            "canary_ok": False}
    if not classify_leg(hung, "hang_timeout", None):
        print(f"[{TAG}] selftest FAIL: 金丝雀失败漏检", file=sys.stderr)
        return 1
    # 绿臂④:合法挂起腿(金丝雀复绿)。
    if classify_leg({**hung, "canary_ok": True}, "hang_timeout", None):
        print(f"[{TAG}] selftest FAIL: 合法挂起腿误判红", file=sys.stderr)
        return 1
    # 绿臂⑤:毒区图完整性判定;红臂⑥:第三态/error 格必须检出。
    if validate_zone_map(synthetic_map):
        print(f"[{TAG}] selftest FAIL: 合法毒区图误判红", file=sys.stderr)
        return 1
    dirty = json.loads(json.dumps(synthetic_map))
    dirty["cells_default_opt"][0]["classification"] = "error"
    if not validate_zone_map(dirty):
        print(f"[{TAG}] selftest FAIL: 第三态格漏检", file=sys.stderr)
        return 1
    # schema 在树 + required 闭集互核。
    if not SCHEMA_PATH.is_file():
        print(f"[{TAG}] selftest FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    req = set(schema.get("required", []))
    expect = {"schema", "gate", "generated_utc", "environment", "zone_map",
              "static_fail_closed", "legs", "verdict", "notes"}
    if req != expect:
        print(f"[{TAG}] selftest FAIL: schema required 漂移 {req ^ expect}", file=sys.stderr)
        return 1
    legs_req = set((schema.get("properties", {}).get("legs", {})).get("required", []))
    if legs_req != set(LEG_SPECS):
        print(f"[{TAG}] selftest FAIL: schema legs 漂移 {legs_req ^ set(LEG_SPECS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS (5 GREEN + 6 RED + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
