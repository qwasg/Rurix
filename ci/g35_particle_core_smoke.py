#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude(G35 GPU 粒子系统 G35-2 粒子核心运行时)
"""G35-2:粒子核心运行时门冒烟(g35.wave2.particle_core;SoA 粒子池
ping-pong + 确定性发射〔随机带单源 + persistent ID〕+ 半隐式 Euler 积分 +
稳定压缩〔禁原子抢槽,分段稳定 scan 槽位〕+ indirect args 零回读链——
host 金标准 = src/rurix-render/src/particles/core.rs,device 面 = 4 新
kernel〔g35_sim/g35_particle_compact/g35_emit/g35_indirect_args〕+ 3 scan
kernel 消费面,probe = src/rurix-render/src/bin/g35_particle_core_device.rs
经 vk::run_compute 逐 kernel 真跑;G35-P v1 帧序冻结,RFC-0049 §4.3)。

八面判据(facts 闭集):
1. **kernels_spv_valid**:rurixc 现编 7 kernel(4 新 + 3 scan 消费面)+
   spirv-val 全绿 + 冻结消费面(scan 三 kernel/particles/mod.rs/scan.rs)
   sha256 快照在档(G35-2 消费不修改承诺,漂移守护基线)。
2. **integer_streams_bitexact**:pid/flags/scan_out/seg_offsets/args 五整数
   流 device↔host 逐帧 memcmp 零容差位级(mod.rs 整数域协议)。
3. **f32_parity_within_budget**:pos/vel/age/life 八 f32 流逐帧 max abs diff
   聚合 p100 ≤ 冻结容差(milestones/g35/g35_budget.json
   g35.particle_core.f32_parity_p100 程序读禁手写:threshold = measured ×
   2.0 标定冻结,measured = 0 时 threshold = 0;缺条目时标定腿先跑
   probe --report-max-diff 取 measured 程序写入)。
4. **pid_persistent_unique**:每帧无重复 pid + 幸存段 ⊆ 上帧集 + 新发射段
   == [pid_base, pid_base+emit) 精确区间(persistent ID 机器事实)。
5. **indirect_args_zero_readback**:device args 8 槽 == host 平行金标准推得
   + args[7] == alive_total+emit_count 恒等式(host 不读回 device 计数只
   对拍验证——零回读链)。
6. **determinism_double_run**:同 seed 全链双跑 digest 位级一致(digest =
   所有流字节 sha256 逐帧链式)。
7. **red_arm_effective**:--red-arm seed-change 换 seed 双跑 digest 必异
   (digest 判据对流内容敏感性证明,防镂空 digest 冒充)。
8. **frame_ms_measured**:device 7 dispatch 链逐帧墙钟均值 measured_local
   诚实登记(含 run_compute 逐 dispatch 会话重建开销,非帧率对标)。

三态:无 Vulkan loader/设备/SPV → DEV_ENV_DEGRADE 退 0(不冒充 PASS);
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL(禁 mock 充真跑)。

用法:
  py -3 ci/g35_particle_core_smoke.py --selftest
  py -3 ci/g35_particle_core_smoke.py --gate g35.wave2.particle_core [--frames 64] [--cap 65536] [--seed 42]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g35.wave2.particle_core"
SUBJECT = "g35_particle_core"
WAVE = "G35.2"
TAG = "g35_particle_core"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g35" / "g35_particle_core_gate_evidence_schema.json"
GATE_SCHEMA_ID = "rurix.g35.particle_core_gate_evidence.v1"
BUDGET_PATH = ROOT / "milestones" / "g35" / "g35_budget.json"
TOL_ENTRY_ID = "g35.particle_core.f32_parity_p100"
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
# 4 新 kernel(本波交付)+ 3 scan kernel(G35-1 冻结面,消费不修改)。
NEW_KERNELS = ("g35_sim", "g35_particle_compact", "g35_emit", "g35_indirect_args")
SCAN_KERNELS = ("g35_scan_seg_sum", "g35_scan_spine", "g35_scan_seg_apply")
FROZEN_CONSUMED_PATHS = [
    # G35-2 消费不修改承诺面(scan 三 kernel + host scan/契约头)——sha256
    # 快照在档 = 漂移守护基线(g34 FROZEN_SNAPSHOT 同律;untracked 期无
    # diff-vs-HEAD 可用)。
    "src/rurix-render/kernels/g35_scan_seg_sum.rx",
    "src/rurix-render/kernels/g35_scan_spine.rx",
    "src/rurix-render/kernels/g35_scan_seg_apply.rx",
    "src/rurix-render/src/particles/mod.rs",
    "src/rurix-render/src/particles/scan.rs",
]
WORK = ROOT / ".tmp" / "g35_gates" / "particle_core"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "debug" / f"g35_particle_core_device{EXE_SUFFIX}"

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "kernels_spv_valid",
    "integer_streams_bitexact",
    "f32_parity_within_budget",
    "pid_persistent_unique",
    "indirect_args_zero_readback",
    "determinism_double_run",
    "red_arm_effective",
    "frame_ms_measured",
]


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面;全纯函数零 GPU)
# ---------------------------------------------------------------------------


def frozen_tol(budget: dict | None) -> float | None:
    """冻结容差程序读(estimated/skip_reason 冒充 measured 即 None fail-closed;
    g34_unified_lane frozen_tol 同律)。"""
    if not isinstance(budget, dict):
        return None
    for e in budget.get("entries", []):
        if e.get("id") == TOL_ENTRY_ID:
            if e.get("evidence") != "measured_local" or e.get("skip_reason"):
                return None
            t = e.get("threshold")
            return float(t) if isinstance(t, (int, float)) and not isinstance(t, bool) else None
    return None


def budget_measured(budget: dict | None) -> float | None:
    """标定 measured_value 程序读(threshold == measured × 2.0 关系互核面)。"""
    if not isinstance(budget, dict):
        return None
    for e in budget.get("entries", []):
        if e.get("id") == TOL_ENTRY_ID:
            m = e.get("measured_value")
            return float(m) if isinstance(m, (int, float)) and not isinstance(m, bool) else None
    return None


def calib_threshold(measured: float) -> float:
    """标定协议冻结 k:threshold = measured × 2.0(measured = 0 时 = 0.0
    零容差零条目;程序产禁手写)。"""
    return measured * 2.0


def upsert_budget_entry(doc: dict | None, entry: dict) -> dict:
    """budget 读-改-写保序:只增改自己 id 条目,他人条目 0-byte 序不动;
    文件缺失时建 g35 命名空间骨架(g34_budget.json 字段格式)。"""
    if doc is None:
        doc = {
            "schema_version": 1,
            "namespace": "g35",
            "description": (
                "G35 预算面。G35-2 粒子核心:f32 流 device↔host 对拍容差条目由本波"
                "标定真跑程序产(threshold = measured × 2.0 冻结 k,禁手写;"
                "measured = 0 时 threshold = 0 零容差零条目)。"
            ),
            "source_docs": ["milestones/g35/g35_particle_core_gate_evidence_schema.json"],
            "entries": [],
            "ratio_assertions": [],
            "counter_assertions": [],
        }
    entries = list(doc.get("entries") or [])
    for i, e in enumerate(entries):
        if e.get("id") == entry["id"]:
            entries[i] = entry
            break
    else:
        entries.append(entry)
    doc["entries"] = entries
    return doc


def _num(v) -> bool:
    return isinstance(v, (int, float)) and not isinstance(v, bool) and v == v


def integer_ok(doc: dict) -> bool:
    """② 整数流零容差判:probe 逐帧 memcmp 全等旗标 + 帧数非零。"""
    return (
        doc.get("integer_streams_bitexact") is True
        and isinstance(doc.get("frames"), int)
        and doc["frames"] >= 1
    )


def f32_within(measured, tol) -> bool:
    """③ f32 对拍硬判:measured 有限非负且 ≤ 冻结容差。"""
    return _num(measured) and _num(tol) and 0.0 <= measured <= tol


def pid_ok(doc: dict) -> bool:
    """④ pid 持久唯一判:唯一 + 幸存子集 + 发射区间精确三旗标合取。"""
    return (
        doc.get("pid_persistent_unique") is True
        and doc.get("pid_unique") is True
        and doc.get("pid_survivor_subset") is True
        and doc.get("pid_emit_range_exact") is True
    )


def args_ok(doc: dict) -> bool:
    """⑤ indirect args 零回读链判:8 槽全等 + args[7] 恒等式合取。"""
    return (
        doc.get("indirect_args_device_match") is True
        and doc.get("args_match") is True
        and doc.get("args_identity") is True
    )


def determinism_ok(doc: dict) -> bool:
    """⑥ 双跑位级判:旗标 + digest 形态 + digest_a == digest_b 互核。"""
    a, b = doc.get("digest_a"), doc.get("digest_b")
    return (
        doc.get("determinism_double_run") is True
        and isinstance(a, str)
        and isinstance(b, str)
        and DIGEST_RE.match(a) is not None
        and a == b
    )


def red_ok(doc: dict) -> bool:
    """⑦ RED 臂判:seed-change 检出 + 双 digest 形态合法且必异。"""
    g, r = doc.get("digest_green"), doc.get("digest_red")
    return (
        doc.get("arm") == "seed-change"
        and doc.get("detected") is True
        and isinstance(g, str)
        and isinstance(r, str)
        and DIGEST_RE.match(g) is not None
        and DIGEST_RE.match(r) is not None
        and g != r
    )


def frame_ms_sane(v) -> bool:
    """⑧ frame_ms 登记面健全判:有限正数(诚实登记非阈门)。"""
    return _num(v) and v > 0


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def sha256_of(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


def spv_args() -> list[str]:
    return [
        "--spv-sim", str(WORK / "g35_sim.spv"),
        "--spv-compact", str(WORK / "g35_particle_compact.spv"),
        "--spv-emit", str(WORK / "g35_emit.spv"),
        "--spv-indirect-args", str(WORK / "g35_indirect_args.spv"),
        "--spv-scan-seg-sum", str(WORK / "g35_scan_seg_sum.spv"),
        "--spv-scan-spine", str(WORK / "g35_scan_spine.spv"),
        "--spv-scan-seg-apply", str(WORK / "g35_scan_seg_apply.spv"),
    ]


def run_probe(
    label: str,
    frames: int,
    cap: int,
    seed: int,
    extra: list[str],
    env: dict,
) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = WORK / f"probe_{label}.json"
    argv = [str(BIN), *spv_args(), "--frames", str(frames), "--cap", str(cap),
            "--seed", str(seed), "--evidence-out", str(ev_path), *extra]
    r = run(argv, timeout=3600, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def run_gate(frames: int, cap: int, seed: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not GATE_SCHEMA_PATH.is_file():
        fail(f"gate schema 缺失: {GATE_SCHEMA_PATH}")
        return 1

    # ── 构建(probe vulkan bin + rurixc SPV 面)──
    ok = build_or_fail(
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan",
         "--bin", "g35_particle_core_device", "--quiet"],
        "probe bin",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── ① kernel SPV 面:现编 7 件(4 新 + 3 scan 消费面)+ spirv-val +
    #    冻结消费面 sha256 快照 ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for name in NEW_KERNELS + SCAN_KERNELS:
        src = KERNEL_DIR / f"{name}.rx"
        dst = WORK / f"{name}.spv"
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(dst)], timeout=1800)
        if r.returncode != 0 or not dst.is_file():
            spv_ok = False
            note(f"rurixc 编译失败 {src.name}: {(r.stdout + r.stderr)[-200:]}")
            continue
        val = run(["spirv-val", str(dst)], timeout=600)
        if val.returncode != 0:
            spv_ok = False
            note(f"spirv-val 未过 {dst.name}: {(val.stdout + val.stderr)[-200:]}")
    frozen_snapshot: dict[str, str] = {}
    snapshot_ok = True
    for p in FROZEN_CONSUMED_PATHS:
        fp = ROOT / p
        if fp.is_file():
            frozen_snapshot[p] = sha256_of(fp)
        else:
            snapshot_ok = False
            frozen_snapshot[p] = "MISSING"
    set_fact(
        "kernels_spv_valid",
        spv_ok and snapshot_ok,
        f"rurixc 现编 7 kernel(4 新 g35_sim/g35_particle_compact/g35_emit/g35_indirect_args + "
        f"3 scan 消费面)+ spirv-val={'绿' if spv_ok else '红'};冻结消费面(scan 三 kernel/"
        f"particles/mod.rs/scan.rs)sha256 快照在档={snapshot_ok}(G35-2 消费不修改承诺,漂移守护基线)",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("G35-2 kernel SPV 编译/spirv-val 未过")

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    gate_path = ROOT / "evidence" / f"g35_particle_core_gate_{ts}.json"
    gate_rel = str(gate_path.relative_to(ROOT)).replace("\\", "/")
    doc_green: dict | None = None
    doc_red: dict | None = None
    probe_evidence: list[str] = []
    tol: float | None = None
    calibrated = False
    pending_entry: dict | None = None

    if not degrade:
        env = device_env()
        with gpu_device_lock(purpose=f"{TAG} 标定腿 + 绿臂 + 红臂 device 真跑"):
            # ── ③ 标定腿(缺条目才跑;threshold = measured × 2.0 程序产)──
            budget = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
            tol = frozen_tol(budget)
            if tol is None:
                rc, doc_cal, ev_cal = run_probe("calibration", frames, cap, seed, ["--report-max-diff"], env)
                out = (rc.stdout or "") + (rc.stderr or "")
                if (doc_cal or {}).get("state") == "skipped_dev_env" or '"skipped_dev_env"' in out:
                    degrade.append(f"probe skipped_dev_env(标定腿): {out.strip()[-200:]}")
                elif rc.returncode != 0 or doc_cal is None or not _num(doc_cal.get("f32_max_abs_diff")):
                    fail(f"标定腿真跑失败 rc={rc.returncode}: {out[-300:]}")
                else:
                    measured = float(doc_cal["f32_max_abs_diff"])
                    tol = calib_threshold(measured)
                    calibrated = True
                    probe_evidence.append(str(ev_cal.relative_to(ROOT)).replace("\\", "/"))
                    pending_entry = {
                        "id": TOL_ENTRY_ID,
                        "description": (
                            "G35-2 粒子核心 f32 流 device↔host 对拍容差冻结带(pos_x/y/z、vel_x/y/z、"
                            "age、life 八 f32 流逐帧 max abs diff 聚合全帧 p100;整数流 pid/flags/scan/"
                            "args 走零容差位级不入本条目;sim/emit SPV 装载期注入 NoContraction 禁驱动 "
                            "FMA 收缩后标定;threshold = measured × 2.0 协议冻结 k,measured = 0 时 "
                            "threshold = 0 零容差零条目,方向 max;标定真跑 = ci/g35_particle_core_smoke.py "
                            "--gate g35.wave2.particle_core 标定腿〔g35_particle_core_device 同 seed 双跑"
                            "位级一致面〕;evidence_file = 门裁决件 results.trimmed_mean 镜像槽,"
                            "budget_eval 通用路消费;标定程序可复跑)"
                        ),
                        "direction": "max",
                        "evidence": "measured_local",
                        "skip_reason": None,
                        "unit": "f32_absdiff",
                        "threshold": tol,
                        "evidence_file": gate_rel,
                        "measured_value": measured,
                    }
                    note(f"标定腿:measured={measured:e} → threshold={tol:e}(×2.0 程序产,gate 评后写入 budget)")
            else:
                m = budget_measured(budget)
                note(f"冻结容差程序读:threshold={tol:e}(measured={m!r};{TOL_ENTRY_ID} 在档跳过标定)")

            # ── 绿臂(默认模式 = 同 seed 双跑 + 逐帧对拍)──
            if not degrade:
                rc, doc_green, ev_green = run_probe("green", frames, cap, seed, ["--report-max-diff"], env)
                out = (rc.stdout or "") + (rc.stderr or "")
                if (doc_green or {}).get("state") == "skipped_dev_env" or '"skipped_dev_env"' in out:
                    degrade.append(f"probe skipped_dev_env(绿臂): {out.strip()[-200:]}")
                    doc_green = None
                else:
                    if rc.returncode != 0 or doc_green is None:
                        fail(f"绿臂真跑失败 rc={rc.returncode}: {out[-300:]}")
                    if "Validation Error" in out or "VUID-" in out:
                        fail("绿臂 validation 应静默却报错")
                    if doc_green is not None:
                        probe_evidence.append(str(ev_green.relative_to(ROOT)).replace("\\", "/"))

            # ── 红臂(seed-change:digest 判据敏感性证明)──
            if not degrade:
                rc, doc_red, ev_red = run_probe("red", frames, cap, seed, ["--red-arm", "seed-change"], env)
                out = (rc.stdout or "") + (rc.stderr or "")
                if (doc_red or {}).get("state") == "skipped_dev_env" or '"skipped_dev_env"' in out:
                    degrade.append(f"probe skipped_dev_env(红臂): {out.strip()[-200:]}")
                    doc_red = None
                else:
                    if rc.returncode != 0 or doc_red is None:
                        fail(f"红臂真跑失败 rc={rc.returncode}: {out[-300:]}")
                    if doc_red is not None:
                        probe_evidence.append(str(ev_red.relative_to(ROOT)).replace("\\", "/"))

    if degrade:
        doc = {
            "schema": "rurix.g35.particle_core.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for dg in degrade:
            note(f"DEV_ENV_DEGRADE {dg}")
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0

    # ── ②~⑧ facts(绿臂/红臂 evidence 判读)──
    g = doc_green or {}
    r_ = doc_red or {}
    measured_green = g.get("f32_max_abs_diff")
    set_fact(
        "integer_streams_bitexact",
        integer_ok(g),
        f"五整数流(flags/scan_out/seg_offsets/pid/args)device↔host 逐帧 memcmp 零容差位级 "
        f"= {g.get('integer_streams_bitexact')!r}({g.get('frames')!r} 帧,cap={g.get('cap')!r};"
        f"problems={g.get('problems') or []})",
    )
    set_fact(
        "f32_parity_within_budget",
        tol is not None and f32_within(measured_green, tol),
        f"八 f32 流全帧 p100 measured={measured_green!r} ≤ 冻结容差 {tol!r}"
        f"({TOL_ENTRY_ID} {'本次标定腿程序产' if calibrated else '程序读'};threshold = measured × 2.0;"
        f"逐流 max={g.get('f32_stream_max')!r};NoContraction 注入面 = {g.get('nocontraction_injected')!r})",
    )
    set_fact(
        "pid_persistent_unique",
        pid_ok(g),
        f"persistent ID 机器事实:每帧唯一={g.get('pid_unique')!r} 幸存段 ⊆ 上帧集="
        f"{g.get('pid_survivor_subset')!r} 新发射段精确区间={g.get('pid_emit_range_exact')!r}"
        f"(pids_issued={g.get('pids_issued')!r})",
    )
    set_fact(
        "indirect_args_zero_readback",
        args_ok(g),
        f"indirect args 零回读链:device args 8 槽 == host 平行推得={g.get('args_match')!r} + "
        f"args[7] == alive+emit 恒等式={g.get('args_identity')!r}(末帧 args={g.get('args_last')!r};"
        f"host 不读回 device 计数只对拍验证)",
    )
    set_fact(
        "determinism_double_run",
        determinism_ok(g),
        f"同 seed 全链双跑 digest 位级一致={g.get('determinism_double_run')!r}"
        f"(digest_a={str(g.get('digest_a'))[:23]}…;所有流字节 sha256 逐帧链式)",
    )
    set_fact(
        "red_arm_effective",
        red_ok(r_),
        f"RED 臂 seed-change:换 seed 双跑 digest 必异 detected={r_.get('detected')!r}"
        f"(green={str(r_.get('digest_green'))[:23]}… red={str(r_.get('digest_red'))[:23]}…)",
    )
    fm = g.get("frame_ms_mean")
    set_fact(
        "frame_ms_measured",
        frame_ms_sane(fm),
        f"device 7 dispatch 链逐帧墙钟均值 {fm!r} ms(measured_local 诚实登记;含 run_compute "
        f"逐 dispatch instance/device 会话重建开销,登记语义非帧率对标)",
    )

    # ── evidence 落盘(门裁决件;jsonschema 自校验硬门)──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti(本机单卡 measured_local)",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    spv_entry = lambda name: {
        "path": str((WORK / f"{name}.spv").relative_to(ROOT)).replace("\\", "/"),
        "sha256": sha256_of(WORK / f"{name}.spv") if (WORK / f"{name}.spv").is_file() else "sha256:" + "0" * 64,
    }
    measured_num = float(measured_green) if _num(measured_green) else -1.0
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "kernels": {
            "sim_spv": spv_entry("g35_sim"),
            "compact_spv": spv_entry("g35_particle_compact"),
            "emit_spv": spv_entry("g35_emit"),
            "indirect_args_spv": spv_entry("g35_indirect_args"),
            "scan_seg_sum_spv": spv_entry("g35_scan_seg_sum"),
            "scan_spine_spv": spv_entry("g35_scan_spine"),
            "scan_seg_apply_spv": spv_entry("g35_scan_seg_apply"),
            "spirv_val_all": bool(facts["kernels_spv_valid"]["status"] == "PASS"),
            "frozen_consumed_snapshot": frozen_snapshot,
        },
        "frame_protocol": {
            "frames": g.get("frames", frames),
            "cap": g.get("cap", cap),
            "seed": g.get("seed", seed),
            "dt": g.get("dt", 1.0 / 60.0),
            "emit_schedule": "min(64 + frame*17 % 192, cap - n_curr)",
            "n_final": g.get("n_final", 0),
            "alive_final": g.get("alive_final", 0),
            "pids_issued": g.get("pids_issued", 0),
        },
        "integer_parity": {
            "bitexact": g.get("integer_streams_bitexact", False),
            "streams": ["flags", "scan_out", "seg_offsets", "pid", "args"],
        },
        "f32_parity": {
            "measured_p100": measured_num,
            "threshold": tol if tol is not None else -1.0,
            "budget_entry": TOL_ENTRY_ID,
            "calibrated_this_run": calibrated,
            "within": bool(tol is not None and f32_within(measured_green, tol)),
            "stream_max": g.get("f32_stream_max") or {},
        },
        "results": {"trimmed_mean": measured_num},
        "pid_persistence": {
            "unique": g.get("pid_unique", False),
            "survivor_subset": g.get("pid_survivor_subset", False),
            "emit_range_exact": g.get("pid_emit_range_exact", False),
        },
        "indirect_args": {
            "device_match": g.get("args_match", False),
            "identity_alive_plus_emit": g.get("args_identity", False),
            "args_last": g.get("args_last", [0] * 8),
        },
        "determinism": {
            "double_run_bitexact": g.get("determinism_double_run", False),
            "digest_a": g.get("digest_a", "sha256:" + "0" * 64),
            "digest_b": g.get("digest_b", "sha256:" + "0" * 64),
        },
        "red_arm": {
            "arm": "seed-change",
            "detected": r_.get("detected", False),
            "digest_green": r_.get("digest_green", "sha256:" + "0" * 64),
            "digest_red": r_.get("digest_red", "sha256:" + "0" * 64),
        },
        "frame_ms": {
            "device_chain_mean_ms": fm if frame_ms_sane(fm) else 1e-9,
            "frames_per_run": g.get("frames", frames),
            "measured": "measured_local",
            "note": (
                "device 7 dispatch 链逐帧墙钟均值(vk::run_compute 每 dispatch 重建 "
                "instance/device,该会话开销如实计入;登记语义非帧率对标,生产车道"
                "届时走 DeviceFrameSession 持久车道 + DispatchSpec::Indirect)"
            ),
        },
        "probe_evidence": probe_evidence or ["(probe evidence 缺失)", "(probe evidence 缺失)"],
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G35-2 粒子核心运行时:SoA 9 流粒子池 ping-pong(读 A 写 B 帧末交换)+ 确定性发射"
            "(随机带单源 rand_table(seed) device 只读消费 rand_table[(pid·7919+slot)%65536],"
            "persistent ID = pid_base+j host u32 递增维护,f32 参数面精确域 < 2^24)+ 半隐式 "
            "Euler 积分(运算序逐字冻结 vy→px→py〔新 vy〕→pz→age;drag v1 恒 0 登记)+ 稳定压缩"
            "(禁原子抢槽,槽位一律经 G35-1 分段稳定 scan 三 kernel〔消费不修改〕,稳定序 = 下标序)"
            "+ indirect args 零回读链(device 直读 scan 总和槽合成 dispatch{total,1,1}/draw{6·total,"
            "1,0,0}/meta 槽,host 平行金标准 n_next = alive+emit 只对拍验证)。host 金标准 = "
            "particles/core.rs(sim_step/compact_step/emit_step/indirect_args/frame 与 4 kernel 逐字"
            "同源);probe = bin/g35_particle_core_device.rs(vk::run_compute 逐 kernel,buffers "
            "Vec<Vec<u8>> 跨 kernel 复用;sim/emit SPV 装载期注入 NoContraction〔g14_3_lane_body "
            "bin-local 同律复制〕禁驱动 FMA 收缩,compact 纯搬运/indirect_args 纯整数不注入)。"
            "整数流零容差位级 + f32 流标定容差(threshold = measured×2.0 程序产禁手写)+ 同 seed "
            "双跑位级 + seed-change RED 臂 + frame_ms measured_local 登记。results.trimmed_mean = "
            "f32 measured p100 镜像(budget_eval 通用路 evidence_file 消费面)。"
        ),
    }
    import jsonschema  # 自校验硬门(schema 漂移即 RED)

    errs = list(jsonschema.Draft7Validator(
        json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        fail("gate evidence schema 自校验红: " + "; ".join(
            f"{'/'.join(str(p) for p in e.path)}: {e.message}" for e in errs[:3]))
        all_pass = False
        gate_doc["verdict"] = "FAIL"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_rel}(probe 件 {len(probe_evidence)} 份留 .tmp 工作区)")

    # ── budget 程序写(标定腿产;gate 裁决件已落盘 ⇒ evidence_file 不悬空;
    #    读-改-写保序只增改自己前缀条目)──
    if pending_entry is not None:
        budget_doc = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
        budget_doc = upsert_budget_entry(budget_doc, pending_entry)
        BUDGET_PATH.parent.mkdir(parents=True, exist_ok=True)
        BUDGET_PATH.write_text(json.dumps(budget_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        back = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
        if frozen_tol(back) != pending_entry["threshold"]:
            fail("budget 回读互核失败(写入后 frozen_tol ≠ 待写 threshold)")
            all_pass = False
        else:
            note(f"g35_budget.json 程序写入 {TOL_ENTRY_ID}(threshold={pending_entry['threshold']:e};重读核验绿)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest(判读器红绿穷举 + schema 校验 + FACT_IDS 互核;零 GPU 零构建)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    d0 = "sha256:" + "a" * 64
    d1 = "sha256:" + "b" * 64
    # 红绿臂①:冻结容差程序读 + threshold = measured × 2.0 协议。
    good_budget = {"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                                "skip_reason": None, "threshold": 2.0e-6, "measured_value": 1.0e-6}]}
    expect(frozen_tol(good_budget) == 2.0e-6, "GREEN:容差程序读正例")
    expect(budget_measured(good_budget) == 1.0e-6, "GREEN:measured_value 程序读正例")
    expect(frozen_tol(good_budget) == calib_threshold(budget_measured(good_budget)),
           "GREEN:threshold == measured × 2.0 关系互核")
    expect(calib_threshold(0.0) == 0.0, "GREEN:measured = 0 ⇒ threshold = 0(零容差零条目)")
    expect(calib_threshold(3.0e-7) == 6.0e-7, "GREEN:×2.0 冻结 k")
    expect(frozen_tol({"entries": [{"id": TOL_ENTRY_ID, "evidence": "estimated",
                                    "skip_reason": None, "threshold": 1.0}]}) is None,
           "RED:estimated 冒充 measured 必拒")
    expect(frozen_tol({"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                                    "skip_reason": "no gpu", "threshold": 1.0}]}) is None,
           "RED:skip_reason 携带必拒")
    expect(frozen_tol({"entries": []}) is None, "RED:条目缺失必拒")
    expect(frozen_tol(None) is None, "RED:budget 文件缺失必拒")
    expect(frozen_tol({"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                                    "skip_reason": None, "threshold": True}]}) is None,
           "RED:bool 冒充数值阈必拒")
    # 红绿臂②:budget 读-改-写保序(只增改自己条目)。
    foreign = {"id": "g35.primitives.other_tol", "threshold": 1.0}
    mine = {"id": TOL_ENTRY_ID, "evidence": "measured_local", "skip_reason": None,
            "threshold": 4.0e-6, "measured_value": 2.0e-6}
    up = upsert_budget_entry({"namespace": "g35", "entries": [foreign]}, dict(mine))
    expect(up["entries"][0] == foreign and up["entries"][1]["id"] == TOL_ENTRY_ID,
           "GREEN:upsert 追加保序(他人条目 0-byte 序不动)")
    up2 = upsert_budget_entry(up, {**mine, "threshold": 8.0e-6})
    expect(len(up2["entries"]) == 2 and up2["entries"][1]["threshold"] == 8.0e-6
           and up2["entries"][0] == foreign,
           "GREEN:upsert 原位替换自己条目(幂等面)")
    skel = upsert_budget_entry(None, dict(mine))
    expect(skel.get("namespace") == "g35" and skel["entries"] == [mine]
           and skel.get("ratio_assertions") == [] and skel.get("counter_assertions") == [],
           "GREEN:budget 缺失建 g35 命名空间骨架")
    # 红绿臂③:整数流判。
    expect(integer_ok({"integer_streams_bitexact": True, "frames": 64}), "GREEN:整数流位级正例")
    expect(not integer_ok({"integer_streams_bitexact": False, "frames": 64}), "RED:整数流非位级必红")
    expect(not integer_ok({"integer_streams_bitexact": True, "frames": 0}), "RED:零帧必红")
    expect(not integer_ok({"frames": 64}), "RED:旗标缺失必红")
    # 红绿臂④:f32 对拍判。
    expect(f32_within(1.9e-6, 2.0e-6), "GREEN:f32 带内过")
    expect(f32_within(0.0, 0.0), "GREEN:measured = 0 vs threshold = 0 边界过(零容差零条目)")
    expect(not f32_within(2.1e-6, 2.0e-6), "RED:f32 超容差必红")
    expect(not f32_within(float("nan"), 2.0e-6), "RED:NaN measured 必红")
    expect(not f32_within(-1.0, 2.0e-6), "RED:负 measured 必红")
    expect(not f32_within(1.0e-6, None), "RED:容差缺失(未标定)必红")
    expect(not f32_within(True, 2.0e-6), "RED:bool 冒充数值必红")
    # 红绿臂⑤:pid 持久唯一判。
    good_pid = {"pid_persistent_unique": True, "pid_unique": True,
                "pid_survivor_subset": True, "pid_emit_range_exact": True}
    expect(pid_ok(good_pid), "GREEN:pid 三旗标正例")
    expect(not pid_ok({**good_pid, "pid_unique": False}), "RED:pid 重复必红")
    expect(not pid_ok({**good_pid, "pid_survivor_subset": False}), "RED:幸存段非子集必红")
    expect(not pid_ok({**good_pid, "pid_emit_range_exact": False}), "RED:发射区间不精确必红")
    expect(not pid_ok({**good_pid, "pid_persistent_unique": "true"}), "RED:字符串冒充 bool 必红")
    # 红绿臂⑥:indirect args 零回读链判。
    good_args = {"indirect_args_device_match": True, "args_match": True, "args_identity": True}
    expect(args_ok(good_args), "GREEN:args 正例")
    expect(not args_ok({**good_args, "args_match": False}), "RED:args 8 槽非全等必红")
    expect(not args_ok({**good_args, "args_identity": False}), "RED:args[7] 恒等式破必红")
    expect(not args_ok({}), "RED:旗标缺失必红")
    # 红绿臂⑦:双跑位级判。
    expect(determinism_ok({"determinism_double_run": True, "digest_a": d0, "digest_b": d0}),
           "GREEN:双跑位级正例")
    expect(not determinism_ok({"determinism_double_run": True, "digest_a": d0, "digest_b": d1}),
           "RED:旗标真但 digest 异(自相矛盾)必红")
    expect(not determinism_ok({"determinism_double_run": False, "digest_a": d0, "digest_b": d0}),
           "RED:旗标假必红")
    expect(not determinism_ok({"determinism_double_run": True, "digest_a": "xx", "digest_b": "xx"}),
           "RED:digest 形态破必红")
    # 红绿臂⑧:RED 臂判。
    good_red = {"arm": "seed-change", "detected": True, "digest_green": d0, "digest_red": d1}
    expect(red_ok(good_red), "GREEN:RED 臂正例")
    expect(not red_ok({**good_red, "detected": False}), "RED:漏检必红")
    expect(not red_ok({**good_red, "digest_red": d0}), "RED:digest 未变(镂空 digest)必红")
    expect(not red_ok({**good_red, "arm": "tamper"}), "RED:臂名不符必红")
    expect(not red_ok({**good_red, "digest_green": "bad"}), "RED:digest 形态破必红")
    # 红绿臂⑨:frame_ms 健全判。
    expect(frame_ms_sane(845.8), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(0.0), "RED:0ms 必红")
    expect(not frame_ms_sane(float("nan")), "RED:NaN 必红")
    expect(not frame_ms_sane(None), "RED:缺失必红")
    expect(not frame_ms_sane(True), "RED:bool 冒充数值必红")
    # schema 互核:gate schema 在树 + Draft7 合法 + facts enum == FACT_IDS +
    # const 互核 + results.trimmed_mean 通用消费面互核。
    expect(GATE_SCHEMA_PATH.is_file(), "gate schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate schema 门键 const 互核")
        expect(gs["properties"]["f32_parity"]["properties"]["budget_entry"]["const"] == TOL_ENTRY_ID,
               "gate schema budget_entry const 互核")
        expect("results" in gs.get("required", [])
               and gs["properties"]["results"]["properties"]["trimmed_mean"]["type"] == "number",
               "gate schema results.trimmed_mean 通用消费面互核(budget_eval evidence_file 路)")
        import jsonschema as _js
        _js.Draft7Validator.check_schema(gs)
        print("  ok   — gate schema Draft7 合法(check_schema 绿)")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts=8;9 红绿臂组 + budget 读改写保序 + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=64)
    ap.add_argument("--cap", type=int, default=65536)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        if args.frames < 64:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 64(冻结夹具 life ∈ [0.8,1.6)s"
                  f"/dt=1/60 下寿命死亡覆盖需 ≥49 帧;64 = 冻结默认窗)", file=sys.stderr)
            return 1
        if args.cap <= 0 or args.cap % 256 != 0:
            print(f"[{TAG}] FAIL: --cap {args.cap} 须为 SEG=256 正整倍数", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.cap, args.seed)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
