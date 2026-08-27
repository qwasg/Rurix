#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# SPIKE(RD-027):G31+ 波 C Task C10 — RD-027 毒径参数面扫描(E8)+ 优化档判别腿复测。
# 探针隔离 spike/rd027-pt-poison/,不入 src/ 生产路径;复用 spike_common(proc_guard
# 看门狗 + 金丝雀门 + campaign JSONL,R-606 零裸 launch;G3.1 E0a~E7b 同律)。
"""E8 毒区图(G31+ 波 C Task C10):

- **参数面扫描**:spp ∈ {8,32,64,128,256} × bounces ∈ {1,2,3,4} 全网格 20 组合,
  生产档 720p / N=131072 / REND_FRAMES=1 / SPP_BATCH=min(spp,32),默认工具链
  (ptxas -O3 AOT cubin)逐格 build+guarded_run(60s 判定线;绿档实测 ~1s 量级,
  余量 ≥30×;挂起后金丝雀门)。完成格取 frame_0000.ppm sha256 入图。
- **判别腿复测(归因再确认)**:毒格 (8,3) 三优化档腿——RURIXC_PTXAS_OPT=1
  预期挂(复确认 O0→O1 绿/挂分界)、=0 预期完成;另 (8,4)/(256,2)/(32,2) 的
  =0 腿(护栏下毒格终止 + digest 基线;(32,2) 兼作跨档 digest 对照)。
- **单 artifact 复确认**:全部默认档变体 PTX sha256 去重计数(参数走 launch
  标量,PTX 应与组合无关)。
- **毒区图落盘**:`--write-map` 蒸馏 milestones/g31/g31_rd027_poison_zone_map.json
  (机器可读毒区图;ci/g31_rd027_poison_guard.py 的 fail-closed 分类事实源)。

全程 gpu_device_lock 排他(G9 蜂群纪律);全部 GPU 运行经 bench/proc_guard
guarded_run(超时 = 诚实红 exit 124 + 杀进程树 + 隔离),挂起判定后金丝雀复绿
方采信后续。用法:
  py -3 spike/rd027-pt-poison/run_e8_zone.py [--write-map]
"""
from __future__ import annotations

import datetime
import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "ci"))

from spike_common import (  # noqa: E402
    QUARANTINE, WORK, append_jsonl, build_variant, canary, campaign_header,
    fail, log, nvsmi_sample, sha256_file, GpuSampler,
)
from bench.proc_guard import guarded_run  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

ZONE_TIMEOUT = 60          # 秒:毒格判定线(绿档 ~1s,余量 ≥30×;G3.1 用 120s,本扫加密网格收敛)
CANARY_TIMEOUT = 90        # 秒:金丝雀(已知绿基准秒级复绿)
SPP_AXIS = [8, 32, 64, 128, 256]
B_AXIS = [1, 2, 3, 4]
MAP_OUT = ROOT / "milestones" / "g31" / "g31_rd027_poison_zone_map.json"
RAW_OUT = WORK / "e8" / "zone_map_raw.json"


def patches_for(spp: int, bounces: int) -> list[tuple[str, str, str]]:
    """params.rx 锚定补丁(现值 = STUB(RD-027) 切片 32/32/8f/2;锚缺失即 fail)。"""
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


def opt_env(level: int | None) -> dict | None:
    """RURIXC_PTXAS_OPT 构建期环境(None = 缺省不注入,行为 0-byte;MR-0011)。"""
    if level is None:
        return None
    env = os.environ.copy()
    env["RURIXC_PTXAS_OPT"] = str(level)
    return env


def run_leg(name: str, exe: Path, timeout: int, expect: str) -> dict:
    """guarded_run 一条腿(保留 rundir;完成腿取 frame_0000.ppm digest;伴随 GPU 采样)。"""
    rundir = WORK / "runs" / f"{name}_{datetime.datetime.now().strftime('%H%M%S')}"
    rundir.mkdir(parents=True, exist_ok=True)
    pre = nvsmi_sample()
    t0 = time.perf_counter()
    with GpuSampler(interval=10.0) as sampler:
        r = guarded_run([exe], timeout=timeout, cwd=rundir,
                        quarantine_exe=exe, quarantine_dir=QUARANTINE,
                        label=f"e8:{name}")
    wall = time.perf_counter() - t0
    post = nvsmi_sample()
    classification = ("hang_timeout" if r.timed_out
                      else ("completed" if r.returncode == 0 else "error"))
    ppm = rundir / "frame_0000.ppm"
    digest = (sha256_file(ppm) if classification == "completed" and ppm.is_file()
              else None)
    render_ok = "RENDER_OK frames=8" in r.stdout  # 入口打印字面恒 frames=8(帧数参数不改打印)
    rec = {
        "kind": "run",
        "campaign": "e8_zone",
        "name": name,
        "expect": expect,
        "classification": classification,
        "exit_code": r.returncode,
        "wall_s": round(wall, 2),
        "timeout_s": timeout,
        "timed_out": r.timed_out,
        "quarantined": r.quarantined,
        "render_ok": render_ok,
        "frame_digest": digest,
        "gpu_pre": pre,
        "gpu_during": sampler.samples,
        "gpu_post": post,
        "stdout_tail": r.stdout[-200:],
        "stderr_tail": r.stderr[-200:],
    }
    append_jsonl(rec)
    log(f"{name}: {classification} exit={r.returncode} wall={wall:.1f}s "
        f"digest={(digest or '—')[:16]} (expect={expect})")
    if classification == "completed" and not render_ok:
        fail(f"{name} 完成但缺 RENDER_OK 见证行: {r.stdout[-200:]!r}")
    if classification == "completed" and digest is None:
        fail(f"{name} 完成但 frame_0000.ppm 缺失(防静默测错档)")
    return rec


def ptxas_version() -> str:
    try:
        r = subprocess.run(["ptxas", "--version"], capture_output=True, text=True,
                           timeout=15)
        for line in (r.stdout + r.stderr).splitlines():
            if "release" in line:
                return line.strip()
    except Exception:
        pass
    return "unknown"


def remap() -> int:
    """--remap:从 raw 汇总重写毒区图(纯 CPU,GPU 零占;蒸馏层修正用——cells/digest
    原样透传 raw,仅 rule_observed 等派生串重算;measured 面 0-byte)。"""
    raw = json.loads(RAW_OUT.read_text(encoding="utf-8"))
    ptx_candidates = sorted((WORK / "bin").glob("z_s*.ptx"))
    if not ptx_candidates:
        fail(f"--remap 无 PTX 可复核单 artifact sha: {WORK / 'bin'}")
    ptx_sha = sha256_file(ptx_candidates[0])
    for extra in ptx_candidates[1:]:
        if sha256_file(extra) != ptx_sha:
            fail(f"--remap PTX 去重复核失败: {extra.name} sha 漂移")
    write_zone_map(raw, raw["cells"], raw["o0_cells"], {}, raw.get("hang_signature"),
                   raw.get("cross_opt_digest_check"), raw["distinct_ptx_digests"], ptx_sha)
    log(f"--remap 完成(PTX 复核 {len(ptx_candidates)} 份同一)")
    return 0


def main() -> int:
    if "--remap" in sys.argv:
        return remap()
    write_map = "--write-map" in sys.argv
    t_start = time.perf_counter()
    with gpu_device_lock(purpose="rd027 E8 毒区图扫描(G31+ 波C C10)", timeout_s=10800.0):
        campaign_header("e8_zone", "G31+ 波C Task C10:毒径参数面 20 格扫描 + O0/O1 判别腿复测")

        # ── ① 全网格默认档构建 ──
        grid: list[tuple[int, int, str, dict]] = []  # (spp, b, name, build_info)
        ptx_digests: set[str] = set()
        for b in B_AXIS:
            for spp in SPP_AXIS:
                name = f"z_s{spp}_b{b}"
                info = build_variant(name, patches_for(spp, b))
                grid.append((spp, b, name, info))
                if info["ptx_sha256"]:
                    ptx_digests.add(info["ptx_sha256"])
        log(f"默认档 20 变体构建完成;distinct PTX digests = {len(ptx_digests)}"
            f"(单 artifact 复确认:挂/不挂应纯由运行期实参定)")

        # ── ② 判别腿构建((8,3)@O1/(8,3)@O0/(8,4)@O0/(256,2)@O0/(32,2)@O0) ──
        disc_builds: dict[str, dict] = {}
        for name, spp, b, lvl in [
            ("z_s8_b3_O1", 8, 3, 1),
            ("z_s8_b3_O0", 8, 3, 0),
            ("z_s8_b4_O0", 8, 4, 0),
            ("z_s256_b2_O0", 256, 2, 0),
            ("z_s32_b2_O0", 32, 2, 0),
        ]:
            disc_builds[name] = build_variant(name, patches_for(spp, b), env=opt_env(lvl))

        # ── ③ 网格真跑(按 b 升序;绿档先行打底,挂起后金丝雀门) ──
        results: dict[str, dict] = {}
        canary_exe = next(info["exe"] for spp, b, _n, info in grid if (spp, b) == (8, 2))
        for spp, b, name, info in grid:
            expect = "green" if (b <= 2 and spp <= 32) else "unknown"
            rec = run_leg(name, info["exe"], ZONE_TIMEOUT, expect)
            rec["spp"], rec["bounces"], rec["opt_leg"] = spp, b, "default"
            results[name] = rec
            if rec["classification"] == "hang_timeout":
                if not canary(canary_exe):
                    fail("金丝雀门失败——GPU 态疑污染,中止扫描(不带污染态采信后续)")

        # ── ④ 判别腿真跑 ──
        disc_runs: dict[str, dict] = {}
        for name, expect, timeout in [
            ("z_s8_b3_O1", "hang", ZONE_TIMEOUT),
            ("z_s8_b3_O0", "green", ZONE_TIMEOUT),
            ("z_s8_b4_O0", "green", ZONE_TIMEOUT),
            ("z_s256_b2_O0", "green", ZONE_TIMEOUT),
            ("z_s32_b2_O0", "green", ZONE_TIMEOUT),
        ]:
            rec = run_leg(name, disc_builds[name]["exe"], timeout, expect)
            lvl = name.rsplit("_", 1)[1]
            rec["opt_leg"] = lvl
            disc_runs[name] = rec
            if rec["classification"] == "hang_timeout":
                if not canary(canary_exe):
                    fail("金丝雀门失败——GPU 态疑污染,中止判别腿")

        # ── ⑤ 汇总 ──
        env = None
        for line in _read_campaign_header():
            env = line
        cells = []
        for spp, b, name, _info in grid:
            rec = results[name]
            cells.append({
                "spp": spp,
                "bounces": b,
                "opt_leg": "default",
                "classification": rec["classification"],
                "exit_code": rec["exit_code"],
                "wall_s": rec["wall_s"],
                "frame_digest": rec["frame_digest"],
            })
        o0_cells = []
        for name in ("z_s8_b3_O0", "z_s8_b4_O0", "z_s256_b2_O0", "z_s32_b2_O0"):
            rec = disc_runs[name]
            spp_b = name.removeprefix("z_s").removesuffix("_O0").split("_b")
            o0_cells.append({
                "spp": int(spp_b[0]),
                "bounces": int(spp_b[1]),
                "opt_leg": "O0",
                "classification": rec["classification"],
                "exit_code": rec["exit_code"],
                "wall_s": rec["wall_s"],
                "frame_digest": rec["frame_digest"],
            })
        hang_sigs = [
            s for rec in results.values() if rec["classification"] == "hang_timeout"
            for s in rec["gpu_during"]
        ]
        sig = None
        if hang_sigs:
            sig = {
                "samples": len(hang_sigs),
                "util_pct_mean": round(sum(s["util_pct"] for s in hang_sigs) / len(hang_sigs), 1),
                "power_w_mean": round(sum(s["power_w"] for s in hang_sigs) / len(hang_sigs), 1),
                "sm_clock_mhz_mean": round(sum(s["sm_clock_mhz"] for s in hang_sigs) / len(hang_sigs), 0),
            }
        cross_opt = None
        d3 = results["z_s32_b2"]["frame_digest"]
        d0 = disc_runs["z_s32_b2_O0"]["frame_digest"]
        if d3 and d0:
            cross_opt = {"combo": [32, 2], "default_digest": d3, "O0_digest": d0,
                         "identical": d3 == d0}
        raw = {
            "kind": "e8_zone_raw",
            "generated_utc": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "total_wall_s": round(time.perf_counter() - t_start, 1),
            "environment": env,
            "ptxas_version": ptxas_version(),
            "distinct_ptx_digests": len(ptx_digests),
            "cells": cells,
            "o0_cells": o0_cells,
            "discriminator_O1": {
                "combo": [8, 3],
                "classification": disc_runs["z_s8_b3_O1"]["classification"],
                "exit_code": disc_runs["z_s8_b3_O1"]["exit_code"],
                "wall_s": disc_runs["z_s8_b3_O1"]["wall_s"],
            },
            "cross_opt_digest_check": cross_opt,
            "hang_signature": sig,
            "grid": {"spp_axis": SPP_AXIS, "bounces_axis": B_AXIS,
                     "basis": "1280x720 / N=131072 / REND_FRAMES=1 / SPP_BATCH=min(spp,32) / "
                              "SUBSTEPS=4 sim 后数据;默认档 = ptxas -O3 AOT cubin"},
            "timeout_s": ZONE_TIMEOUT,
        }
        RAW_OUT.parent.mkdir(parents=True, exist_ok=True)
        with open(RAW_OUT, "wb") as f:
            f.write((json.dumps(raw, ensure_ascii=False, indent=1) + "\n").encode("utf-8"))
        log(f"raw 汇总 → {RAW_OUT}")

        # ── ⑥ 诚实判定:与既有事实矩阵对账 ──
        bad = []
        expect_map = {(8, 2): "completed", (32, 2): "completed",
                      (8, 3): "hang_timeout", (8, 4): "hang_timeout",
                      (256, 2): "hang_timeout"}
        for (spp, b), want in expect_map.items():
            got = results[f"z_s{spp}_b{b}"]["classification"]
            if got != want:
                bad.append(f"({spp},{b}) 期望 {want} 实测 {got}——与 G3.1 事实矩阵漂移")
        if disc_runs["z_s8_b3_O1"]["classification"] != "hang_timeout":
            bad.append("判别腿 (8,3)@O1 未挂——O0→O1 分界漂移")
        for n in ("z_s8_b3_O0", "z_s8_b4_O0", "z_s256_b2_O0"):
            if disc_runs[n]["classification"] != "completed":
                bad.append(f"护栏腿 {n} 未完成——MR-0011 护栏效力存疑")
        if len(ptx_digests) != 1:
            bad.append(f"distinct PTX digests = {len(ptx_digests)} ≠ 1——单 artifact 事实漂移")
        for c in cells + o0_cells:
            if c["classification"] == "error":
                bad.append(f"格 {c['spp']}/{c['bounces']}/{c['opt_leg']} error——"
                           "非挂起非完成的第三类判定,毒区图拒写")
        if bad:
            for m in bad:
                log(f"FAIL-INFORM {m}")
            append_jsonl({"kind": "e8_verdict", "ok": False, "drift": bad})
            fail(f"E8 对账漂移 {len(bad)} 条(毒区图未写;先人工裁决再 --write-map)")
        append_jsonl({"kind": "e8_verdict", "ok": True})
        log("E8 对账全绿:G3.1 五格复现 + O0/O1 分界 + 护栏 + 单 artifact 全维持")

        if write_map:
            write_zone_map(raw, cells, o0_cells, disc_runs, sig, cross_opt,
                           len(ptx_digests), next(iter(ptx_digests)))
        log(f"E8 全程 wall={time.perf_counter() - t_start:.0f}s")
    return 0


def _read_campaign_header() -> list[dict]:
    """从 campaign.jsonl 尾部找本论 header 的 environment(不重读全文件)。"""
    from spike_common import CAMPAIGN_LOG
    envs: list[dict] = []
    if CAMPAIGN_LOG.is_file():
        with open(CAMPAIGN_LOG, "rb") as f:
            for raw in f:
                try:
                    rec = json.loads(raw.decode("utf-8"))
                except Exception:
                    continue
                if rec.get("kind") == "header" and rec.get("round") == "e8_zone":
                    envs.append(rec.get("environment") or {})
    return envs[-1:] if envs else []


def derive_rule(cells: list[dict]) -> str:
    """从实测 cells 逐行蒸馏规则串(机读分类在 cells,本串仅供人读;禁手改)。"""
    by_b: dict[int, dict[str, list[int]]] = {}
    for c in cells:
        row = by_b.setdefault(c["bounces"], {"completed": [], "hang_timeout": []})
        row[c["classification"]].append(c["spp"])
    parts = []
    for b in sorted(by_b):
        g = sorted(by_b[b]["completed"])
        p = sorted(by_b[b]["hang_timeout"])
        if g and p:
            parts.append(f"bounces=={b}: spp≤{max(g)} 绿 / spp≥{min(p)} 毒")
        elif g:
            parts.append(f"bounces=={b}: 全测 spp({min(g)}~{max(g)}) 绿")
        else:
            parts.append(f"bounces=={b}: 全测 spp({min(p)}~{max(p)}) 毒")
    return ";".join(parts) + "(实测网格闭集,未测组合 = unmapped,fail-closed 按毒处理)"


def write_zone_map(raw: dict, cells: list[dict], o0_cells: list[dict],
                   disc_runs: dict, sig: dict | None, cross_opt: dict | None,
                   distinct_ptx: int, ptx_sha: str) -> None:
    """蒸馏机器可读毒区图 → milestones/g31/(guard fail-closed 分类事实源)。"""
    green = sorted((c["spp"], c["bounces"]) for c in cells if c["classification"] == "completed")
    poison = sorted((c["spp"], c["bounces"]) for c in cells if c["classification"] == "hang_timeout")
    doc = {
        "schema_version": 1,
        "kind": "g31_rd027_poison_zone_map",
        "rd_ref": "RD-027",
        "generated_utc": raw["generated_utc"],
        "generator": ("spike/rd027-pt-poison/run_e8_zone.py --write-map "
                      "(G31+ 波C Task C10;E8 全网格实测,非外推)"),
        "environment": {
            **(raw.get("environment") or {}),
            "ptxas_version": raw["ptxas_version"],
        },
        "basis": {
            "entry": "apps/ruridrop/src/offline.rx(tmp copytree + params.rx 锚定补丁,原树 0-byte)",
            "rend_w": 1280, "rend_h": 720, "rend_frames": 1,
            "n_particles": 131072, "grid_dim": 64, "substeps": 4,
            "spp_batch_rule": "min(spp,32)(SPP % SPP_BATCH == 0)",
            "data_dependency": "需 4 子步 sim 后粒子分布(G3.1 E4 SUBSTEPS=0 不触发在案)",
            "opt_default": "ptxas -O3 AOT cubin(驱动 620.02 JIT 腿同类变换同挂,G3.1 E0b)",
            "timeout_s": raw["timeout_s"],
            "judgment_margin": "绿档实测 ~1s 量级,判定线余量 ≥30×",
        },
        "single_artifact": {
            "distinct_ptx_digests": distinct_ptx,
            "ptx_sha256": ptx_sha,
            "confirmed": distinct_ptx == 1,
            "note": "参数经 launch 标量下发(RXS-0191),挂/不挂纯由运行期实参 + sim 后数据定",
        },
        "cells_default_opt": cells,
        "cells_guardrail_O0": o0_cells,
        "zone_summary": {
            "green_combos": [[s, b] for s, b in green],
            "poison_combos": [[s, b] for s, b in poison],
            "rule_observed": derive_rule(cells),
        },
        "discriminator": {
            "combo": [8, 3],
            "O1_leg": raw["discriminator_O1"],
            "O0_leg": next(c for c in o0_cells if c["spp"] == 8 and c["bounces"] == 3),
            "verdict_reconfirmed": (
                "O0 完成 / O1+ 挂 绿挂分界复确认;归因维持 nvidia_optimizing_backends"
                "(G3.1 spike verdict,M1′ 机理)"),
        },
        "cross_opt_digest_check": cross_opt,
        "hang_signature": sig,
        "guardrail": {
            "kind": "RURIXC_PTXAS_OPT=0(MR-0011;AOT cubin 腿)",
            "verified_combos": [[8, 3], [8, 4], [256, 2]],
            "jit_leg_limit": "驱动 JIT fallback 腿无对应优化档开关,护栏不覆盖(如实限界)",
        },
        "rebaseline_discipline": (
            "driver/ptxas/GPU 变更后 digest 漂移 = 门卫诚实红;按本生成器全网格重测蒸馏本图,"
            "禁手改 cells/digest(measured-first)"),
    }
    with open(MAP_OUT, "wb") as f:
        f.write((json.dumps(doc, ensure_ascii=False, indent=1) + "\n").encode("utf-8"))
    log(f"毒区图 → {MAP_OUT}")


if __name__ == "__main__":
    sys.exit(main())
