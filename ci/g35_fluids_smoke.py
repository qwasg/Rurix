#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude(G35 GPU 粒子系统 G35-7 流体统一物理)
"""G35-7:流体统一物理门冒烟(g35.wave7.fluids;count-sort 空间哈希邻居
搜索〔cell_key → W1 sort 三 kernel 3-pass 稳定排序消费不修改 → 单写者
cellrange,分段三阶段零原子〕+ XPBD/PBF 密度约束求解〔FleX 式:poly6 密度
+ spiky 梯度 + λ 乘子,ITER=3 固定迭代 Jacobi ping-pong,gather-only 零
原子〕——host 金标准 = src/rurix-render/src/particles/fluid.rs,device 面 =
6 新 kernel〔g35_hash_cellkey/g35_hash_clear/g35_hash_cellrange/
g35_xpbd_density/g35_xpbd_apply/g35_xpbd_velocity〕+ 3 sort kernel 消费面,
probe = src/rurix-render/src/bin/g35_fluids_device.rs 经 vk::run_compute
逐 kernel 真跑;冻结协议 = RFC-0049 §4.10 F14 + fluid.rs 契约头)。

八面判据(facts 闭集):
1. **kernels_spv_valid**:rurixc 现编 9 kernel(6 新 + 3 sort 消费面)+
   spirv-val 全绿 + 冻结消费面(sort 三 kernel/particles/mod.rs/
   primitives.rs)sha256 快照在档(G35-7 消费不修改承诺,漂移守护基线)。
2. **neighbor_sets_bitexact**:cell_key/sorted_keys/sorted_idx/cell_start/
   cell_end 五整数流 device↔host 逐帧 memcmp 零容差位级(邻居结构位级 ⇒
   邻居集位级;host 侧另有 grid↔朴素 O(n²) 邻居集相等单测互核,
   cargo test 面)。**单帧对拍协议冻结**(probe 头注字面):每帧 device
   九流 = host 金标准帧首状态注入——device .sqrt() 为 Vulkan 非正确舍入
   语义面,单 ULP 种子在触地混沌域经自由跑跨帧 Lyapunov 放大跨 cell 边界
   ⇒ 整数零容差在自由跑下物理不可达;注入协议下 cellkey 预测路径
   (mul/add+FDiv+floor,NoContraction)逐 op 位级恒成立,f32 = 单帧有界
   发散走标定容差,双跑/RED 臂同协议成立。
3. **hash_cell_floor_semantics**:cell = floor((p−origin)/cs) 逐轴负坐标
   向负无穷 + 越界 clamp 到边界 cell 语义见证——dam-break 触地帧预测位置
   必越下界 ⇒ negative_floor_events ≥ 1 且 clamp_events ≥ 1(host 登记
   计数),且整数流位级(device .floor()/clamp 与 host 同语义的机器事实)。
4. **xpbd_parity_within_budget**:pos_x/y/z、vel_x/y/z、ρ、λ 八 f32 流逐帧
   max abs diff 聚合 p100 ≤ 冻结容差(milestones/g35/g35_budget.json
   g35.fluids.parity_p100 程序读禁手写:threshold = measured × 2.0 标定
   冻结,measured = 0 时 threshold = 0;缺条目时标定腿先跑 probe
   --report-max-diff 取 measured 程序写入)。
5. **density_error_measured**:密度误差 measured 登记(device ρ 流:
   mean |ρ/ρ0−1| 与 mean(max(ρ/ρ0−1,0)) 首/末帧;压缩夹具首帧正约束违反
   必 > 0 = 咬合前提;登记语义不设收敛死值——收敛方向性断言归 fluid.rs
   单测 cargo test 面)。
6. **determinism_double_run**:同 seed 全链双跑 digest 位级一致(digest =
   pos/vel/ρ/λ/cell_key/sorted/cell 区间字节 sha256 逐帧链式)。
7. **red_arm_effective**:--red-arm rho0-tamper ρ0 篡改(×1.05)双跑
   digest 必异(压缩夹具 C>0 恒真 ⇒ λ 必受 ρ0 影响——digest 判据对约束
   求解敏感性证明,防镂空 digest 冒充)。
8. **frame_ms_measured**:device 19 dispatch 链(cellkey + sort 9 + clear +
   cellrange + [density+apply]×3 + velocity)逐帧墙钟均值 measured_local
   诚实登记(含 run_compute 逐 dispatch 会话重建开销,非帧率对标)。

MPM 评估窗:不实现——G2P/P2G 散射需原子或图着色,与确定性协议冲突待裁
(RFC-0049 §4.10 既有登记引用;evidence notes 登记)。

三态:无 Vulkan loader/设备/SPV → DEV_ENV_DEGRADE 退 0(不冒充 PASS);
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL(禁 mock 充真跑)。

用法:
  py -3 ci/g35_fluids_smoke.py --selftest
  py -3 ci/g35_fluids_smoke.py --gate g35.wave7.fluids [--frames 32] [--n 4096] [--seed 42]
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

GATE_KEY = "g35.wave7.fluids"
SUBJECT = "g35_fluids"
WAVE = "G35.7"
TAG = "g35_fluids"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g35" / "g35_fluids_gate_evidence_schema.json"
GATE_SCHEMA_ID = "rurix.g35.fluids_gate_evidence.v1"
BUDGET_PATH = ROOT / "milestones" / "g35" / "g35_budget.json"
TOL_ENTRY_ID = "g35.fluids.parity_p100"
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
# 6 新 kernel(本波交付)+ 3 sort kernel(G35-1 冻结面,消费不修改)。
NEW_KERNELS = (
    "g35_hash_cellkey",
    "g35_hash_clear",
    "g35_hash_cellrange",
    "g35_xpbd_density",
    "g35_xpbd_apply",
    "g35_xpbd_velocity",
)
SORT_KERNELS = ("g35_sort_hist", "g35_sort_spine", "g35_sort_scatter")
FROZEN_CONSUMED_PATHS = [
    # G35-7 消费不修改承诺面(W1 sort 三 kernel + host 排序镜像/契约头)——
    # sha256 快照在档 = 漂移守护基线(g35_particle_core FROZEN 同律)。
    "src/rurix-render/kernels/g35_sort_hist.rx",
    "src/rurix-render/kernels/g35_sort_spine.rx",
    "src/rurix-render/kernels/g35_sort_scatter.rx",
    "src/rurix-render/src/particles/mod.rs",
    "src/rurix-render/src/particles/primitives.rs",
]
WORK = ROOT / ".tmp" / "g35_gates" / "fluids"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "debug" / f"g35_fluids_device{EXE_SUFFIX}"

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "kernels_spv_valid",
    "neighbor_sets_bitexact",
    "hash_cell_floor_semantics",
    "xpbd_parity_within_budget",
    "density_error_measured",
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
    g35_particle_core frozen_tol 同律)。"""
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
    文件缺失时建 g35 命名空间骨架(g35_particle_core 同律)。"""
    if doc is None:
        doc = {
            "schema_version": 1,
            "namespace": "g35",
            "description": (
                "G35 预算面。G35-7 流体:f32 流 device↔host 对拍容差条目由本波"
                "标定真跑程序产(threshold = measured × 2.0 冻结 k,禁手写;"
                "measured = 0 时 threshold = 0 零容差零条目)。"
            ),
            "source_docs": ["milestones/g35/g35_fluids_gate_evidence_schema.json"],
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


def _int_nonneg(v) -> bool:
    return isinstance(v, int) and not isinstance(v, bool) and v >= 0


def neighbor_ok(doc: dict) -> bool:
    """② 邻居结构零容差判:五整数流逐帧 memcmp 全等旗标 + 帧数非零。"""
    return (
        doc.get("integer_streams_bitexact") is True
        and isinstance(doc.get("frames"), int)
        and doc["frames"] >= 1
    )


def floor_ok(doc: dict) -> bool:
    """③ floor/clamp 语义判:负 floor 与 clamp 事件均 ≥ 1(夹具咬合)且
    整数流位级(device 语义 == host 语义的机器事实)。"""
    nf = doc.get("negative_floor_events")
    ce = doc.get("clamp_events")
    return (
        doc.get("integer_streams_bitexact") is True
        and _int_nonneg(nf)
        and nf >= 1
        and _int_nonneg(ce)
        and ce >= 1
    )


def f32_within(measured, tol) -> bool:
    """④ f32 对拍硬判:measured 有限非负且 ≤ 冻结容差。"""
    return _num(measured) and _num(tol) and 0.0 <= measured <= tol


def density_ok(doc: dict) -> bool:
    """⑤ 密度误差登记判:四 measured 有限非负 + 首帧正约束违反 > 0
    (压缩夹具咬合前提;收敛方向性断言归 fluid.rs 单测面)。"""
    af = doc.get("density_mean_abs_err_first")
    al = doc.get("density_mean_abs_err_last")
    pf = doc.get("density_pos_constraint_first")
    pl = doc.get("density_pos_constraint_last")
    return (
        _num(af)
        and _num(al)
        and _num(pf)
        and _num(pl)
        and af >= 0.0
        and al >= 0.0
        and pl >= 0.0
        and pf > 0.0
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
    """⑦ RED 臂判:rho0-tamper 检出 + 双 digest 形态合法且必异 +
    ρ0 双值确实相异(防同值假臂)。"""
    g, r = doc.get("digest_green"), doc.get("digest_red")
    rg, rr = doc.get("rho0_green"), doc.get("rho0_red")
    return (
        doc.get("arm") == "rho0-tamper"
        and doc.get("detected") is True
        and isinstance(g, str)
        and isinstance(r, str)
        and DIGEST_RE.match(g) is not None
        and DIGEST_RE.match(r) is not None
        and g != r
        and _num(rg)
        and _num(rr)
        and rg != rr
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
        "--spv-hash-cellkey", str(WORK / "g35_hash_cellkey.spv"),
        "--spv-hash-clear", str(WORK / "g35_hash_clear.spv"),
        "--spv-hash-cellrange", str(WORK / "g35_hash_cellrange.spv"),
        "--spv-sort-hist", str(WORK / "g35_sort_hist.spv"),
        "--spv-sort-spine", str(WORK / "g35_sort_spine.spv"),
        "--spv-sort-scatter", str(WORK / "g35_sort_scatter.spv"),
        "--spv-xpbd-density", str(WORK / "g35_xpbd_density.spv"),
        "--spv-xpbd-apply", str(WORK / "g35_xpbd_apply.spv"),
        "--spv-xpbd-velocity", str(WORK / "g35_xpbd_velocity.spv"),
    ]


def run_probe(
    label: str,
    frames: int,
    n: int,
    seed: int,
    extra: list[str],
    env: dict,
) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = WORK / f"probe_{label}.json"
    argv = [str(BIN), *spv_args(), "--frames", str(frames), "--n", str(n),
            "--seed", str(seed), "--evidence-out", str(ev_path), *extra]
    r = run(argv, timeout=3600, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def run_gate(frames: int, n: int, seed: int) -> int:
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
         "--bin", "g35_fluids_device", "--quiet"],
        "probe bin",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── ① kernel SPV 面:现编 9 件(6 新 + 3 sort 消费面)+ spirv-val +
    #    冻结消费面 sha256 快照 ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for name in NEW_KERNELS + SORT_KERNELS:
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
        f"rurixc 现编 9 kernel(6 新 g35_hash_cellkey/clear/cellrange + g35_xpbd_density/apply/"
        f"velocity + 3 sort 消费面)+ spirv-val={'绿' if spv_ok else '红'};冻结消费面(sort 三 "
        f"kernel/particles/mod.rs/primitives.rs)sha256 快照在档={snapshot_ok}"
        f"(G35-7 消费不修改承诺,漂移守护基线)",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("G35-7 kernel SPV 编译/spirv-val 未过")

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    gate_path = ROOT / "evidence" / f"g35_fluids_gate_{ts}.json"
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
            # ── ④ 标定腿(缺条目才跑;threshold = measured × 2.0 程序产)──
            budget = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
            tol = frozen_tol(budget)
            if tol is None:
                rc, doc_cal, ev_cal = run_probe("calibration", frames, n, seed, ["--report-max-diff"], env)
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
                            "G35-7 流体 f32 流 device↔host 对拍容差冻结带(pos_x/y/z、vel_x/y/z、"
                            "ρ、λ 八 f32 流逐帧 max abs diff 聚合全帧 p100;整数流 cell_key/"
                            "sorted_keys/sorted_idx/cell_start/cell_end 走零容差位级不入本条目;"
                            "cellkey/density/apply/velocity SPV 装载期注入 NoContraction 禁驱动 "
                            "FMA 收缩后标定;threshold = measured × 2.0 协议冻结 k,measured = 0 "
                            "时 threshold = 0 零容差零条目,方向 max;标定真跑 = "
                            "ci/g35_fluids_smoke.py --gate g35.wave7.fluids 标定腿"
                            "〔g35_fluids_device 同 seed 双跑位级一致面〕;evidence_file = 门裁决"
                            "件 results.trimmed_mean 镜像槽,budget_eval 通用路消费;标定程序可复跑)"
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
                rc, doc_green, ev_green = run_probe("green", frames, n, seed, ["--report-max-diff"], env)
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

            # ── 红臂(rho0-tamper:digest 判据敏感性证明)──
            if not degrade:
                rc, doc_red, ev_red = run_probe("red", frames, n, seed, ["--red-arm", "rho0-tamper"], env)
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
            "schema": "rurix.g35.fluids.skip.v1",
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
        "neighbor_sets_bitexact",
        neighbor_ok(g),
        f"五整数流(cell_key/sorted_keys/sorted_idx/cell_start/cell_end)device↔host 逐帧 "
        f"memcmp 零容差位级 = {g.get('integer_streams_bitexact')!r}({g.get('frames')!r} 帧,"
        f"n={g.get('n')!r};邻居结构位级 ⇒ 27-cell 固定序邻居集位级;host grid↔朴素 O(n²) "
        f"相等单测互核在 cargo test 面;problems={g.get('problems') or []})",
    )
    set_fact(
        "hash_cell_floor_semantics",
        floor_ok(g),
        f"cell floor/clamp 语义见证:negative_floor_events={g.get('negative_floor_events')!r} ≥ 1 "
        f"(负商向负无穷取整域触发)+ clamp_events={g.get('clamp_events')!r} ≥ 1(越界 clamp 到"
        f"边界 cell 登记)+ 整数流位级(device .floor()/clamp 与 host 同语义机器事实;dam-break "
        f"触地帧预测位置必越下界 = 夹具咬合)",
    )
    set_fact(
        "xpbd_parity_within_budget",
        tol is not None and f32_within(measured_green, tol),
        f"八 f32 流(pos/vel/ρ/λ)全帧 p100 measured={measured_green!r} ≤ 冻结容差 {tol!r}"
        f"({TOL_ENTRY_ID} {'本次标定腿程序产' if calibrated else '程序读'};threshold = measured × 2.0;"
        f"逐流 max={g.get('f32_stream_max')!r};NoContraction 注入面 = {g.get('nocontraction_injected')!r})",
    )
    set_fact(
        "density_error_measured",
        density_ok(g),
        f"密度误差 measured 登记:mean|ρ/ρ0−1| 首帧={g.get('density_mean_abs_err_first')!r} "
        f"末帧={g.get('density_mean_abs_err_last')!r};正约束违反 mean(max(C,0)) 首帧="
        f"{g.get('density_pos_constraint_first')!r}(压缩夹具必 > 0 咬合)末帧="
        f"{g.get('density_pos_constraint_last')!r}(登记语义不设收敛死值;方向性断言归 fluid.rs 单测)",
    )
    set_fact(
        "determinism_double_run",
        determinism_ok(g),
        f"同 seed 全链双跑 digest 位级一致={g.get('determinism_double_run')!r}"
        f"(digest_a={str(g.get('digest_a'))[:23]}…;pos/vel/ρ/λ/cell_key/sorted/cell 区间字节 "
        f"sha256 逐帧链式)",
    )
    set_fact(
        "red_arm_effective",
        red_ok(r_),
        f"RED 臂 rho0-tamper:ρ0 {r_.get('rho0_green')!r}→{r_.get('rho0_red')!r} 篡改双跑 digest "
        f"必异 detected={r_.get('detected')!r}(green={str(r_.get('digest_green'))[:23]}… "
        f"red={str(r_.get('digest_red'))[:23]}…)",
    )
    fm = g.get("frame_ms_mean")
    set_fact(
        "frame_ms_measured",
        frame_ms_sane(fm),
        f"device 19 dispatch 链(cellkey + sort 3-pass 9 + clear + cellrange + [density+apply]×3 "
        f"+ velocity)逐帧墙钟均值 {fm!r} ms(measured_local 诚实登记;含 run_compute 逐 "
        f"dispatch instance/device 会话重建开销,登记语义非帧率对标)",
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
            "hash_cellkey_spv": spv_entry("g35_hash_cellkey"),
            "hash_clear_spv": spv_entry("g35_hash_clear"),
            "hash_cellrange_spv": spv_entry("g35_hash_cellrange"),
            "xpbd_density_spv": spv_entry("g35_xpbd_density"),
            "xpbd_apply_spv": spv_entry("g35_xpbd_apply"),
            "xpbd_velocity_spv": spv_entry("g35_xpbd_velocity"),
            "sort_hist_spv": spv_entry("g35_sort_hist"),
            "sort_spine_spv": spv_entry("g35_sort_spine"),
            "sort_scatter_spv": spv_entry("g35_sort_scatter"),
            "spirv_val_all": bool(facts["kernels_spv_valid"]["status"] == "PASS"),
            "frozen_consumed_snapshot": frozen_snapshot,
        },
        "fluid_protocol": {
            "frames": g.get("frames", frames),
            "n": g.get("n", n),
            "seed": g.get("seed", seed),
            "dt": g.get("dt", 1.0 / 60.0),
            "cell_size": g.get("cell_size", 0.2),
            "rho0": g.get("rho0", 1000.0),
            "mass": g.get("mass", 1.0),
            "gravity_y": g.get("gravity_y", -9.8),
            "grid": g.get("grid", 64),
            "iter": g.get("iter", 3),
        },
        "integer_parity": {
            "bitexact": g.get("integer_streams_bitexact", False),
            "streams": ["cell_key", "sorted_keys", "sorted_idx", "cell_start", "cell_end"],
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
        "floor_semantics": {
            "negative_floor_events": g.get("negative_floor_events", 0),
            "clamp_events": g.get("clamp_events", 0),
        },
        "density": {
            "mean_abs_err_first": g.get("density_mean_abs_err_first", -1.0),
            "mean_abs_err_last": g.get("density_mean_abs_err_last", -1.0),
            "pos_constraint_first": g.get("density_pos_constraint_first", -1.0),
            "pos_constraint_last": g.get("density_pos_constraint_last", -1.0),
            "measured": "measured_local",
        },
        "determinism": {
            "double_run_bitexact": g.get("determinism_double_run", False),
            "digest_a": g.get("digest_a", "sha256:" + "0" * 64),
            "digest_b": g.get("digest_b", "sha256:" + "0" * 64),
        },
        "red_arm": {
            "arm": "rho0-tamper",
            "detected": r_.get("detected", False),
            "rho0_green": r_.get("rho0_green", -1.0),
            "rho0_red": r_.get("rho0_red", -1.0),
            "digest_green": r_.get("digest_green", "sha256:" + "0" * 64),
            "digest_red": r_.get("digest_red", "sha256:" + "0" * 64),
        },
        "frame_ms": {
            "device_chain_mean_ms": fm if frame_ms_sane(fm) else 1e-9,
            "frames_per_run": g.get("frames", frames),
            "measured": "measured_local",
            "note": (
                "device 19 dispatch 链逐帧墙钟均值(vk::run_compute 每 dispatch 重建 "
                "instance/device,该会话开销如实计入;登记语义非帧率对标,生产车道"
                "届时走 DeviceFrameSession 持久车道)"
            ),
        },
        "probe_evidence": probe_evidence or ["(probe evidence 缺失)", "(probe evidence 缺失)"],
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G35-7 流体统一物理:count-sort 空间哈希邻居搜索(cell = floor((p−origin)/cs) 逐轴负"
            "坐标向负无穷 + 越界 clamp 到边界 cell〔RFC-0049 §4.10 F14 冻结〕;cell_key < 64³ = "
            "262144 < 2^24 → W1 sort 三 kernel 3-pass 稳定排序消费不修改 → g35_hash_clear 哨兵清扫"
            "〔start=0xFFFFFFFF/end=0〕+ g35_hash_cellrange 单写者边界检测,分段三阶段零原子)+ "
            "XPBD/PBF 密度约束求解(FleX/Macklin–Müller 谱系:poly6 W=315/(64πh⁹)(h²−r²)³ + spiky "
            "∇W=−45/(πh⁶)(h−r)²r̂〔系数 host 单源程序产经 params 传 device〕;C=ρ/ρ0−1 仅 C>0;"
            "λ=−C/((|Σ∇W|²+Σ|∇W|²)/ρ0²+ε) ε=100 冻结;Δp=(1/ρ0)Σ(λᵢ+λⱼ)∇W;h=cell_size 冻结,"
            "27-cell 固定序 gather;ITER=3 固定迭代 Jacobi ping-pong 禁自适应早停;帧末 vel=(pos−"
            "prev)/dt + 边界速度置零分量,独立 g35_xpbd_velocity kernel 承载)。host 金标准 = "
            "particles/fluid.rs(与 6 kernel 逐字同源 + 朴素 O(n²) 邻居集/密度独立参考互核单测);"
            "probe = bin/g35_fluids_device.rs(dam-break 4096 粒子 Pcg32 固定 seed 压缩初态,32 帧"
            "×3 迭代;cellkey/density/apply/velocity SPV 装载期注入 NoContraction)。整数流零容差"
            "位级 + f32 流标定容差(threshold = measured×2.0 程序产禁手写)+ 同 seed 双跑位级 + "
            "rho0-tamper RED 臂 + frame_ms measured_local 登记。MPM 评估窗不实现——G2P/P2G 散射需"
            "原子或图着色,与确定性协议冲突待裁(RFC-0049 §4.10 既有登记引用,不承诺)。"
            "results.trimmed_mean = f32 measured p100 镜像(budget_eval 通用路 evidence_file 消费面)。"
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
                                "skip_reason": None, "threshold": 2.0e-3, "measured_value": 1.0e-3}]}
    expect(frozen_tol(good_budget) == 2.0e-3, "GREEN:容差程序读正例")
    expect(budget_measured(good_budget) == 1.0e-3, "GREEN:measured_value 程序读正例")
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
    foreign = {"id": "g35.particle_core.f32_parity_p100", "threshold": 1.0}
    mine = {"id": TOL_ENTRY_ID, "evidence": "measured_local", "skip_reason": None,
            "threshold": 4.0e-3, "measured_value": 2.0e-3}
    up = upsert_budget_entry({"namespace": "g35", "entries": [foreign]}, dict(mine))
    expect(up["entries"][0] == foreign and up["entries"][1]["id"] == TOL_ENTRY_ID,
           "GREEN:upsert 追加保序(他人条目 0-byte 序不动)")
    up2 = upsert_budget_entry(up, {**mine, "threshold": 8.0e-3})
    expect(len(up2["entries"]) == 2 and up2["entries"][1]["threshold"] == 8.0e-3
           and up2["entries"][0] == foreign,
           "GREEN:upsert 原位替换自己条目(幂等面)")
    skel = upsert_budget_entry(None, dict(mine))
    expect(skel.get("namespace") == "g35" and skel["entries"] == [mine]
           and skel.get("ratio_assertions") == [] and skel.get("counter_assertions") == [],
           "GREEN:budget 缺失建 g35 命名空间骨架")
    # 红绿臂③:邻居结构零容差判。
    expect(neighbor_ok({"integer_streams_bitexact": True, "frames": 32}), "GREEN:整数流位级正例")
    expect(not neighbor_ok({"integer_streams_bitexact": False, "frames": 32}), "RED:整数流非位级必红")
    expect(not neighbor_ok({"integer_streams_bitexact": True, "frames": 0}), "RED:零帧必红")
    expect(not neighbor_ok({"frames": 32}), "RED:旗标缺失必红")
    expect(not neighbor_ok({"integer_streams_bitexact": "true", "frames": 32}),
           "RED:字符串冒充 bool 必红")
    # 红绿臂④:floor/clamp 语义判。
    good_floor = {"integer_streams_bitexact": True, "negative_floor_events": 7, "clamp_events": 9}
    expect(floor_ok(good_floor), "GREEN:floor 语义正例")
    expect(not floor_ok({**good_floor, "negative_floor_events": 0}),
           "RED:零负 floor 事件(语义未见证)必红")
    expect(not floor_ok({**good_floor, "clamp_events": 0}), "RED:零 clamp 事件必红")
    expect(not floor_ok({**good_floor, "integer_streams_bitexact": False}),
           "RED:整数流非位级(device 语义 ≠ host)必红")
    expect(not floor_ok({**good_floor, "negative_floor_events": 7.5}),
           "RED:非整数计数必红")
    expect(not floor_ok({**good_floor, "negative_floor_events": True}),
           "RED:bool 冒充计数必红")
    # 红绿臂⑤:f32 对拍判。
    expect(f32_within(1.9e-3, 2.0e-3), "GREEN:f32 带内过")
    expect(f32_within(0.0, 0.0), "GREEN:measured = 0 vs threshold = 0 边界过(零容差零条目)")
    expect(not f32_within(2.1e-3, 2.0e-3), "RED:f32 超容差必红")
    expect(not f32_within(float("nan"), 2.0e-3), "RED:NaN measured 必红")
    expect(not f32_within(-1.0, 2.0e-3), "RED:负 measured 必红")
    expect(not f32_within(1.0e-3, None), "RED:容差缺失(未标定)必红")
    expect(not f32_within(True, 2.0e-3), "RED:bool 冒充数值必红")
    # 红绿臂⑥:密度误差登记判。
    good_density = {"density_mean_abs_err_first": 0.9, "density_mean_abs_err_last": 0.4,
                    "density_pos_constraint_first": 0.5, "density_pos_constraint_last": 0.01}
    expect(density_ok(good_density), "GREEN:密度登记正例")
    expect(density_ok({**good_density, "density_pos_constraint_last": 0.0}),
           "GREEN:末帧正约束违反归零(收敛完成)合法")
    expect(not density_ok({**good_density, "density_pos_constraint_first": 0.0}),
           "RED:首帧正约束违反 = 0(压缩夹具咬合破)必红")
    expect(not density_ok({**good_density, "density_mean_abs_err_first": float("nan")}),
           "RED:NaN 登记必红")
    expect(not density_ok({**good_density, "density_mean_abs_err_last": -0.1}),
           "RED:负误差必红")
    expect(not density_ok({"density_mean_abs_err_first": 0.9}), "RED:登记缺失必红")
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
    good_red = {"arm": "rho0-tamper", "detected": True, "digest_green": d0, "digest_red": d1,
                "rho0_green": 1000.0, "rho0_red": 1050.0}
    expect(red_ok(good_red), "GREEN:RED 臂正例")
    expect(not red_ok({**good_red, "detected": False}), "RED:漏检必红")
    expect(not red_ok({**good_red, "digest_red": d0}), "RED:digest 未变(镂空 digest)必红")
    expect(not red_ok({**good_red, "arm": "seed-change"}), "RED:臂名不符必红")
    expect(not red_ok({**good_red, "digest_green": "bad"}), "RED:digest 形态破必红")
    expect(not red_ok({**good_red, "rho0_red": 1000.0}), "RED:ρ0 同值假臂必红")
    # 红绿臂⑨:frame_ms 健全判。
    expect(frame_ms_sane(1141.7), "GREEN:frame_ms 正例")
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
        expect(gs["properties"]["red_arm"]["properties"]["arm"]["const"] == "rho0-tamper",
               "gate schema RED 臂名 const 互核")
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
    ap.add_argument("--frames", type=int, default=32)
    ap.add_argument("--n", type=int, default=4096)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        if args.frames < 32:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 32(dam-break 触地 ≈ 帧 21,"
                  f"floor/clamp 语义见证需触地覆盖;32 = 冻结默认窗)", file=sys.stderr)
            return 1
        if args.n < 512 or args.n > 1048576:
            print(f"[{TAG}] FAIL: --n {args.n} 越域(512 ..= 1048576;4096 = 冻结默认夹具)",
                  file=sys.stderr)
            return 1
        return run_gate(args.frames, args.n, args.seed)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
