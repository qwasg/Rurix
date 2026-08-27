#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude(G35 GPU 粒子系统 G35-5 碰撞与力场)
"""G35-5:粒子碰撞与力场门冒烟(g35.wave5.collision;同帧 TLAS ray query
碰撞〔第 k 帧查询消费第 k 帧场景状态,反打 Niagara GPU RT 碰撞异步一帧
延迟〕+ 深度缓冲碰撞对照臂 + 显式降级链 --collision ray_query|depth_buffer
|off〔fail-closed 禁静默换臂〕+ 力场 gravity/wind/drag v1 闭集——host 金
标准 = src/rurix-render/src/particles/collision.rs〔apply_fields/collide_step
〔TriBvh〕/depth_collide_step〕,device 面 = kernels/g35_sim_collide.rx
〔AccelStruct 首形参,run_ray_query_effects 车道〕+ g35_sim_collide_depth.rx
〔run_compute 车道;res=0 = off 档〕,probe = src/rurix-render/src/bin/
g35_collision_device.rs;RFC-0049 §4.7)。

八面判据(facts 闭集):
1. **kernels_spv_valid**:rurixc 现编 2 新 kernel + spirv-val 全绿 + 冻结
   消费面(g35_sim.rx 变体参照/rt/bvh.rs TriBvh 金标准/particles mod.rs+
   core.rs)sha256 快照在档(G35-5 消费不修改承诺,漂移守护基线)。
2. **collision_parity_vs_host**:ray_query 绿臂 7 f32 流(pos/vel/age)
   device↔host 逐帧 max abs diff 聚合 p100 ≤ 冻结容差(milestones/g35/
   g35_budget.json g35.collision.parity_p100 程序读禁手写:threshold =
   measured × 2.0 标定冻结——ray query t 值 RT core vs host 有 ULP 级差,
   g34 先例;缺条目时标定腿先跑 probe --report-max-diff 程序写入)+
   flags 整数流零容差位级 + 命中样本量非零(防判据空转)。
3. **same_frame_semantics_witness**:障碍方块第 32 帧突移——host gold
   (k 帧查 k 帧场景)vs static(方块不动)首异帧 == 32(突移当帧即响应)
   + late(k 帧查 k−1 帧场景 = Niagara 延迟模型)在突移帧与 static 位级
   一致(延迟模型当帧无响应,判别器双向有效)+ device 方块顶命中登记于
   突移帧非零(device 同帧响应机器事实)。
4. **fallback_chain_explicit**:CLI 三档闭集(闭集外 typed 退 2)+
   ray_query 档 TLAS 能力缺失(--force-no-tlas 注入)→ typed 错误
   E_G35_COLLISION_NO_TLAS_CAPABILITY 退 3(fail-closed 禁静默换臂)+
   三臂 digest 两两互异(档位真实非别名)。
5. **force_fields_parity**:off 臂(纯力场+积分)device↔host p100 ≤ 同
   容差 + host 解析语义判(wind_x/z > 0 漂移为正;drag 速率低于无阻尼
   对照;off 档命中恒零)——与 rurix-physics 方向/单位对齐,不依赖该 crate。
6. **determinism_double_run**:ray_query 绿臂同 seed 全链双跑 digest 位级
   一致(digest = 7 f32 流‖flags‖hit 逐帧 sha256 链式)。
7. **red_arm_effective**:--red-arm tamper-e(device 红链 e×1.5 篡改注入
   vs host 冻结 e)digest 必异 + 红链 measured 必须 > 冻结容差(篡改必被
   parity 判据检出,防镂空 digest 冒充)。
8. **frame_ms_measured**:device 逐帧派发墙钟均值 measured_local 诚实登记
   (含 run_ray_query_effects 每帧重建 instance/device/AS 会话开销——
   同帧语义 probe 形态,非帧率对标)。

三态:无 Vulkan loader/设备/SPV → DEV_ENV_DEGRADE 退 0(不冒充 PASS);
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL(禁 mock 充真跑)。

用法:
  py -3 ci/g35_collision_smoke.py --selftest
  py -3 ci/g35_collision_smoke.py --gate g35.wave5.collision [--frames 64] [--cap 512] [--seed 42]
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

GATE_KEY = "g35.wave5.collision"
SUBJECT = "g35_collision"
WAVE = "G35.5"
TAG = "g35_collision"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g35" / "g35_collision_gate_evidence_schema.json"
GATE_SCHEMA_ID = "rurix.g35.collision_gate_evidence.v1"
BUDGET_PATH = ROOT / "milestones" / "g35" / "g35_budget.json"
TOL_ENTRY_ID = "g35.collision.parity_p100"
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
# 2 新 kernel(本波交付;g35_sim.rx 为变体参照,0-byte 不动)。
NEW_KERNELS = ("g35_sim_collide", "g35_sim_collide_depth")
FROZEN_CONSUMED_PATHS = [
    # G35-5 消费不修改承诺面(sim 变体参照 + TriBvh host 金标准 + 粒子池
    # 契约面)——sha256 快照在档 = 漂移守护基线(g35_particle_core 同律)。
    "src/rurix-render/kernels/g35_sim.rx",
    "src/rurix-render/src/rt/bvh.rs",
    "src/rurix-render/src/particles/mod.rs",
    "src/rurix-render/src/particles/core.rs",
]
WORK = ROOT / ".tmp" / "g35_gates" / "collision"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "debug" / f"g35_collision_device{EXE_SUFFIX}"
# 方块突移帧(probe 冻结脚本 MOVE_FRAME;见证判据锚)。
MOVE_FRAME = 32
# typed 错误码(bin 冻结字面;降级链 fail-closed 面)。
E_NO_TLAS = "E_G35_COLLISION_NO_TLAS_CAPABILITY"

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "kernels_spv_valid",
    "collision_parity_vs_host",
    "same_frame_semantics_witness",
    "fallback_chain_explicit",
    "force_fields_parity",
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


def build_env() -> dict[str, str]:
    # 构建产物钉到仓内 target(BIN 路径契约;沙箱/异构 shell 下 env 漂移防御)。
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(ROOT / "target")
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
                "G35 预算面。G35-5 碰撞与力场:7 f32 流 device↔host 对拍容差条目由本波"
                "标定真跑程序产(threshold = measured × 2.0 冻结 k,禁手写;"
                "measured = 0 时 threshold = 0 零容差零条目)。"
            ),
            "source_docs": ["milestones/g35/g35_collision_gate_evidence_schema.json"],
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


def _int(v) -> bool:
    return isinstance(v, int) and not isinstance(v, bool)


def f32_within(measured, tol) -> bool:
    """f32 对拍硬判:measured 有限非负且 ≤ 冻结容差。"""
    return _num(measured) and _num(tol) and 0.0 <= measured <= tol


def parity_ok(doc: dict, tol) -> bool:
    """② 碰撞对拍判:f32 p100 带内 + flags 位级 + 命中样本量非零合取。"""
    return (
        f32_within(doc.get("f32_max_abs_diff"), tol)
        and doc.get("flags_bitexact") is True
        and _int(doc.get("hits_total_host"))
        and doc["hits_total_host"] >= 1
    )


def witness_ok(doc: dict) -> bool:
    """③ 同帧见证判:gold/static 首异帧 == 突移帧(== MOVE_FRAME)+ late
    在突移帧与 static 位级一致 + gold/late 可分辨 + host/device 方块顶命中
    于突移帧非零,五判合取。"""
    return (
        doc.get("witness_applicable") is True
        and doc.get("box_move_frame") == MOVE_FRAME
        and doc.get("host_div_static_frame") == MOVE_FRAME
        and doc.get("late_same_at_move_frame") is True
        and doc.get("gold_late_differ_at_move_frame") is True
        and _int(doc.get("box_hits_frame32_host"))
        and doc["box_hits_frame32_host"] >= 1
        and _int(doc.get("box_hits_frame32_device"))
        and doc["box_hits_frame32_device"] >= 1
    )


def fallback_ok(
    no_tlas_rc, no_tlas_doc, unknown_rc, digest_rq, digest_depth, digest_off
) -> bool:
    """④ 显式降级链判:no-tlas typed 退 3 + typed 错误码字面 + 闭集外退 2
    + 三臂 digest 形态合法且两两互异。"""
    return (
        no_tlas_rc == 3
        and isinstance(no_tlas_doc, dict)
        and no_tlas_doc.get("state") == "typed_error"
        and no_tlas_doc.get("typed_error") == E_NO_TLAS
        and unknown_rc == 2
        and all(
            isinstance(d, str) and DIGEST_RE.match(d) is not None
            for d in (digest_rq, digest_depth, digest_off)
        )
        and len({digest_rq, digest_depth, digest_off}) == 3
    )


def fields_ok(doc: dict, tol) -> bool:
    """⑤ 力场判:off 臂 p100 带内 + 风漂移方向 + 阻尼速降 + off 档命中恒零。"""
    return (
        doc.get("arm") == "off"
        and f32_within(doc.get("f32_max_abs_diff"), tol)
        and doc.get("wind_dx_positive") is True
        and doc.get("wind_dz_positive") is True
        and doc.get("drag_speed_decay") is True
        and _int(doc.get("hits_total_host"))
        and doc["hits_total_host"] == 0
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


def red_ok(doc: dict, tol) -> bool:
    """⑦ RED 臂判:tamper-e 检出 + 双 digest 形态合法且必异 + 红链 measured
    必须溢出冻结容差(篡改必被 parity 判据检出)。"""
    g, r = doc.get("digest_green"), doc.get("digest_red")
    return (
        doc.get("arm") == "tamper-e"
        and doc.get("detected") is True
        and isinstance(g, str)
        and isinstance(r, str)
        and DIGEST_RE.match(g) is not None
        and DIGEST_RE.match(r) is not None
        and g != r
        and _num(doc.get("red_f32_max_abs_diff"))
        and _num(tol)
        and doc["red_f32_max_abs_diff"] > tol
    )


def frame_ms_sane(v) -> bool:
    """⑧ frame_ms 登记面健全判:有限正数(诚实登记非阈门)。"""
    return _num(v) and v > 0


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv, env=build_env())
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def sha256_of(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


def spv_args() -> list[str]:
    return [
        "--spv-collide", str(WORK / "g35_sim_collide.spv"),
        "--spv-collide-depth", str(WORK / "g35_sim_collide_depth.spv"),
    ]


def run_probe(
    label: str,
    arm: str,
    frames: int,
    cap: int,
    seed: int,
    extra: list[str],
    env: dict,
) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = WORK / f"probe_{label}.json"
    argv = [str(BIN), *spv_args(), "--collision", arm, "--frames", str(frames),
            "--cap", str(cap), "--seed", str(seed), "--evidence-out", str(ev_path), *extra]
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
         "--bin", "g35_collision_device", "--quiet"],
        "probe bin",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── ① kernel SPV 面:现编 2 件 + spirv-val + 冻结消费面 sha256 快照 ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for name in NEW_KERNELS:
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
        f"rurixc 现编 2 kernel(g35_sim_collide〔AccelStruct 同帧 ray query〕/"
        f"g35_sim_collide_depth〔深度对照臂/off 档〕)+ spirv-val={'绿' if spv_ok else '红'};"
        f"冻结消费面(g35_sim.rx 变体参照/rt/bvh.rs/particles mod.rs+core.rs)sha256 快照"
        f"在档={snapshot_ok}(G35-5 消费不修改承诺,漂移守护基线)",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("G35-5 kernel SPV 编译/spirv-val 未过")

    # ── ④ 降级链静态腿(零 GPU:typed 退出码机验;--force-no-tlas 在
    #    loader 探测前判,闭集外在 CLI 解析期判)──
    no_tlas_rc, no_tlas_doc = -1, None
    unknown_rc = -1
    if not degrade:
        ev_nt = WORK / "probe_no_tlas.json"
        r_nt = run([str(BIN), "--collision", "ray_query", "--force-no-tlas",
                    "--evidence-out", str(ev_nt)], timeout=600)
        no_tlas_rc = r_nt.returncode
        if ev_nt.is_file():
            try:
                no_tlas_doc = json.loads(ev_nt.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                no_tlas_doc = None
        r_uk = run([str(BIN), "--collision", "bogus"], timeout=600)
        unknown_rc = r_uk.returncode

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    gate_path = ROOT / "evidence" / f"g35_collision_gate_{ts}.json"
    gate_rel = str(gate_path.relative_to(ROOT)).replace("\\", "/")
    doc_green: dict | None = None
    doc_depth: dict | None = None
    doc_off: dict | None = None
    doc_red: dict | None = None
    probe_evidence: list[str] = []
    tol: float | None = None
    calibrated = False
    pending_entry: dict | None = None

    def skipped(doc: dict | None, out: str) -> bool:
        return (doc or {}).get("state") == "skipped_dev_env" or '"skipped_dev_env"' in out

    if not degrade:
        env = device_env()
        with gpu_device_lock(purpose=f"{TAG} 标定腿 + 绿/depth/off/红臂 device 真跑"):
            # ── 标定腿(缺条目才跑;threshold = measured × 2.0 程序产)──
            budget = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
            tol = frozen_tol(budget)
            if tol is None:
                rc, doc_cal, ev_cal = run_probe(
                    "calibration", "ray_query", frames, cap, seed, ["--report-max-diff"], env)
                out = (rc.stdout or "") + (rc.stderr or "")
                if skipped(doc_cal, out):
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
                            "G35-5 碰撞与力场 7 f32 流(pos_x/y/z、vel_x/y/z、age)device↔host 对拍"
                            "容差冻结带(ray_query 臂逐帧 max abs diff 聚合全帧 p100;flags 整数流走"
                            "零容差位级不入本条目;两 kernel SPV 装载期注入 NoContraction 禁驱动 FMA "
                            "收缩后标定;ray query t 值 RT core vs host ULP 级差为本条目根因,g34 先例;"
                            "threshold = measured × 2.0 协议冻结 k,measured = 0 时 threshold = 0 零容差"
                            "零条目,方向 max;标定真跑 = ci/g35_collision_smoke.py --gate "
                            "g35.wave5.collision 标定腿〔g35_collision_device --collision ray_query "
                            "--report-max-diff〕;evidence_file = 门裁决件 results.trimmed_mean 镜像槽,"
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

            # ── 绿臂(ray_query;同帧见证 + 双跑位级 + 对拍)──
            if not degrade:
                rc, doc_green, ev_green = run_probe(
                    "green", "ray_query", frames, cap, seed, ["--report-max-diff"], env)
                out = (rc.stdout or "") + (rc.stderr or "")
                if skipped(doc_green, out):
                    degrade.append(f"probe skipped_dev_env(绿臂): {out.strip()[-200:]}")
                    doc_green = None
                else:
                    if rc.returncode != 0 or doc_green is None:
                        fail(f"绿臂真跑失败 rc={rc.returncode}: {out[-300:]}")
                    if "Validation Error" in out or "VUID-" in out:
                        fail("绿臂 validation 应静默却报错")
                    if doc_green is not None:
                        probe_evidence.append(str(ev_green.relative_to(ROOT)).replace("\\", "/"))

            # ── depth 对照臂 + off 臂(降级链档位真实性 + 力场判)──
            if not degrade:
                rc, doc_depth, ev_depth = run_probe("depth", "depth_buffer", frames, cap, seed, [], env)
                out = (rc.stdout or "") + (rc.stderr or "")
                if skipped(doc_depth, out):
                    degrade.append(f"probe skipped_dev_env(depth 臂): {out.strip()[-200:]}")
                    doc_depth = None
                else:
                    if rc.returncode != 0 or doc_depth is None:
                        fail(f"depth 臂真跑失败 rc={rc.returncode}: {out[-300:]}")
                    if doc_depth is not None:
                        probe_evidence.append(str(ev_depth.relative_to(ROOT)).replace("\\", "/"))
            if not degrade:
                rc, doc_off, ev_off = run_probe("off", "off", frames, cap, seed, [], env)
                out = (rc.stdout or "") + (rc.stderr or "")
                if skipped(doc_off, out):
                    degrade.append(f"probe skipped_dev_env(off 臂): {out.strip()[-200:]}")
                    doc_off = None
                else:
                    if rc.returncode != 0 or doc_off is None:
                        fail(f"off 臂真跑失败 rc={rc.returncode}: {out[-300:]}")
                    if doc_off is not None:
                        probe_evidence.append(str(ev_off.relative_to(ROOT)).replace("\\", "/"))

            # ── 红臂(tamper-e:篡改 e 注入必检出)──
            if not degrade:
                rc, doc_red, ev_red = run_probe(
                    "red", "ray_query", frames, cap, seed, ["--red-arm", "tamper-e"], env)
                out = (rc.stdout or "") + (rc.stderr or "")
                if skipped(doc_red, out):
                    degrade.append(f"probe skipped_dev_env(红臂): {out.strip()[-200:]}")
                    doc_red = None
                else:
                    if rc.returncode != 0 or doc_red is None:
                        fail(f"红臂真跑失败 rc={rc.returncode}: {out[-300:]}")
                    if doc_red is not None:
                        probe_evidence.append(str(ev_red.relative_to(ROOT)).replace("\\", "/"))

    if degrade:
        doc = {
            "schema": "rurix.g35.collision.skip.v1",
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

    # ── ②~⑧ facts(各臂 evidence 判读)──
    g = doc_green or {}
    dp = doc_depth or {}
    off = doc_off or {}
    rd = doc_red or {}
    zero_d = "sha256:" + "0" * 64
    measured_green = g.get("f32_max_abs_diff")
    set_fact(
        "collision_parity_vs_host",
        tol is not None and parity_ok(g, tol),
        f"ray_query 绿臂 7 f32 流全帧 p100 measured={measured_green!r} ≤ 冻结容差 {tol!r}"
        f"({TOL_ENTRY_ID} {'本次标定腿程序产' if calibrated else '程序读'};threshold = measured × 2.0;"
        f"RT core t 值 ULP 级差协议)+ flags 位级={g.get('flags_bitexact')!r} + hit 失配诚实登记"
        f"={g.get('hit_mismatch_total')!r} + 命中样本={g.get('hits_total_host')!r}(≥1 防空转);"
        f"逐流 max={g.get('f32_stream_max')!r}",
    )
    set_fact(
        "same_frame_semantics_witness",
        witness_ok(g),
        f"同帧见证:host gold vs static 首异帧={g.get('host_div_static_frame')!r}(须 == 突移帧 "
        f"{MOVE_FRAME});late(Niagara 一帧延迟模型)突移帧与 static 位级一致="
        f"{g.get('late_same_at_move_frame')!r};gold/late 可分辨={g.get('gold_late_differ_at_move_frame')!r};"
        f"方块顶命中 host={g.get('box_hits_frame32_host')!r} device={g.get('box_hits_frame32_device')!r}"
        f"(device hit 流 1+primitive_index ∈ 方块三角段,同帧响应机器事实)",
    )
    d_rq = g.get("digest_a", zero_d)
    d_dp = dp.get("digest_a", zero_d)
    d_off = off.get("digest_a", zero_d)
    set_fact(
        "fallback_chain_explicit",
        fallback_ok(no_tlas_rc, no_tlas_doc, unknown_rc, d_rq, d_dp, d_off),
        f"F12 显式降级链:--force-no-tlas typed 退出码={no_tlas_rc}(须 3)typed_error="
        f"{(no_tlas_doc or {}).get('typed_error')!r}(须 {E_NO_TLAS});闭集外 --collision bogus "
        f"退出码={unknown_rc}(须 2);三臂 digest 两两互异="
        f"{len({d_rq, d_dp, d_off}) == 3}(禁静默换臂 + 档位真实非别名)",
    )
    set_fact(
        "force_fields_parity",
        tol is not None and fields_ok(off, tol),
        f"力场 v1 闭集(gravity/wind/drag):off 臂 p100={off.get('f32_max_abs_diff')!r} ≤ {tol!r};"
        f"wind_dx_positive={off.get('wind_dx_positive')!r} wind_dz_positive={off.get('wind_dz_positive')!r} "
        f"drag_speed_decay={off.get('drag_speed_decay')!r};off 档命中={off.get('hits_total_host')!r}(须 0);"
        f"方向/单位与 rurix-physics WorldDesc.gravity y-up 约定对齐(只对齐不依赖)",
    )
    set_fact(
        "determinism_double_run",
        determinism_ok(g),
        f"ray_query 绿臂同 seed 全链双跑 digest 位级一致={g.get('determinism_double_run')!r}"
        f"(digest_a={str(g.get('digest_a'))[:23]}…;7 f32 流‖flags‖hit 逐帧 sha256 链式)",
    )
    set_fact(
        "red_arm_effective",
        tol is not None and red_ok(rd, tol),
        f"RED 臂 tamper-e:device 红链 e×1.5 篡改注入 detected={rd.get('detected')!r};红链对拍 "
        f"measured={rd.get('red_f32_max_abs_diff')!r} > 冻结容差 {tol!r}(篡改必被 parity 判据检出);"
        f"digest green/red 必异={rd.get('digest_green') != rd.get('digest_red')}",
    )
    fm = g.get("frame_ms_mean")
    set_fact(
        "frame_ms_measured",
        frame_ms_sane(fm),
        f"device 逐帧派发墙钟均值 {fm!r} ms(measured_local 诚实登记;含 run_ray_query_effects "
        f"每帧重建 instance/device/AS 的会话开销——同帧语义 probe 形态,登记语义非帧率对标)",
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
    red_measured = float(rd["red_f32_max_abs_diff"]) if _num(rd.get("red_f32_max_abs_diff")) else -1.0
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "kernels": {
            "collide_spv": spv_entry("g35_sim_collide"),
            "collide_depth_spv": spv_entry("g35_sim_collide_depth"),
            "spirv_val_all": bool(facts["kernels_spv_valid"]["status"] == "PASS"),
            "frozen_consumed_snapshot": frozen_snapshot,
        },
        "scene": {
            "tri_count": 12,
            "floor_tris": 2,
            "slope_tris": 2,
            "box_tris": 8,
            "box_move_frame": MOVE_FRAME,
            "frames": g.get("frames", frames),
            "cap": g.get("cap", cap),
            "seed": g.get("seed", seed),
            "dt": g.get("dt", 1.0 / 60.0),
        },
        "collision_parity": {
            "measured_p100": measured_num,
            "threshold": tol if tol is not None else -1.0,
            "budget_entry": TOL_ENTRY_ID,
            "calibrated_this_run": calibrated,
            "within": bool(tol is not None and f32_within(measured_green, tol)),
            "flags_bitexact": g.get("flags_bitexact", False),
            "hit_mismatch_total": g.get("hit_mismatch_total", 0),
            "hits_total_host": g.get("hits_total_host", 0),
            "stream_max": g.get("f32_stream_max") or {},
        },
        "results": {"trimmed_mean": measured_num},
        "same_frame_witness": {
            "applicable": g.get("witness_applicable", False),
            "host_div_static_frame": g.get("host_div_static_frame", -1),
            "late_same_at_move_frame": g.get("late_same_at_move_frame", False),
            "gold_late_differ_at_move_frame": g.get("gold_late_differ_at_move_frame", False),
            "box_hits_move_frame_host": g.get("box_hits_frame32_host", 0),
            "box_hits_move_frame_device": g.get("box_hits_frame32_device", 0),
            "niagara_contrast_note": (
                "对照:Niagara GPU RT 碰撞读上一帧末加速结构(异步一帧延迟)——本波同帧语义 = "
                "第 k 帧查询消费第 k 帧障碍变换重建的 TLAS;late 腿(k 帧查 k−1 帧场景)即该延迟"
                "模型的 host 复刻,在突移帧与 static 位级一致 = 延迟模型当帧无响应,判别器双向有效"
            ),
        },
        "fallback_chain": {
            "cli_closed_set": ["ray_query", "depth_buffer", "off"],
            "no_tlas_typed_error": (no_tlas_doc or {}).get("typed_error", "(缺失)"),
            "no_tlas_exit_code": no_tlas_rc,
            "unknown_arm_exit_code": unknown_rc,
            "arms_digest_distinct": len({d_rq, d_dp, d_off}) == 3,
            "digest_ray_query": d_rq,
            "digest_depth_buffer": d_dp,
            "digest_off": d_off,
        },
        "force_fields": {
            "parity_measured_p100": float(off["f32_max_abs_diff"]) if _num(off.get("f32_max_abs_diff")) else -1.0,
            "within": bool(tol is not None and f32_within(off.get("f32_max_abs_diff"), tol)),
            "wind_dx_positive": off.get("wind_dx_positive", False),
            "wind_dz_positive": off.get("wind_dz_positive", False),
            "drag_speed_decay": off.get("drag_speed_decay", False),
            "off_arm_hits": off.get("hits_total_host", 0),
            "semantics_note": (
                "力场 v1 闭集顺序冻结:vel += (g+wind)·dt;vel ×= (1−drag·dt);再碰撞查询。y-up "
                "右手系,gravity_y 与 rurix_physics::types::WorldDesc::gravity=[0,−9.81,0] 同向同"
                "单位(m/s²),wind = 常量加速度(LinearForce 语义),drag = 线性阻尼(1/s)——"
                "语义对齐登记,渲染 crate 不引物理 crate"
            ),
        },
        "determinism": {
            "double_run_bitexact": g.get("determinism_double_run", False),
            "digest_a": g.get("digest_a", zero_d),
            "digest_b": g.get("digest_b", zero_d),
        },
        "red_arm": {
            "arm": "tamper-e",
            "detected": rd.get("detected", False),
            "e_frozen": rd.get("e_frozen", -1.0),
            "e_tampered": rd.get("e_tampered", -1.0),
            "red_f32_max_abs_diff": red_measured if red_measured >= 0 else 0.0,
            "exceeds_threshold": bool(tol is not None and _num(rd.get("red_f32_max_abs_diff"))
                                      and rd["red_f32_max_abs_diff"] > tol),
            "digest_green": rd.get("digest_green", zero_d),
            "digest_red": rd.get("digest_red", zero_d),
        },
        "frame_ms": {
            "device_chain_mean_ms": fm if frame_ms_sane(fm) else 1e-9,
            "frames_per_run": g.get("frames", frames),
            "measured": "measured_local",
            "note": (
                "device 逐帧派发墙钟均值(ray_query 臂 run_ray_query_effects 每帧重建 instance/"
                "device/BLAS/TLAS = 同帧语义 probe 形态,该会话开销如实计入;登记语义非帧率对标,"
                "生产车道届时走持久场景 + TLAS refit/update)"
            ),
        },
        "probe_evidence": probe_evidence or ["(probe evidence 缺失)", "(probe evidence 缺失)"],
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G35-5 碰撞与力场:同帧 TLAS ray query 碰撞(第 k 帧粒子碰撞查询的 TLAS = 第 k 帧场景"
            "状态,probe 每帧以当帧障碍变换合成三角汤经 run_ray_query_effects 重建场景;反打 "
            "Niagara GPU RT 碰撞异步一帧延迟)+ 深度缓冲对照臂(固定正交俯视深度图 SSBO host 合成"
            "上传;法线 +y 简化,屏幕空间局限〔垂直侧面/悬空底面/多层几何不可表达〕头注如实登记 = "
            "对照教育臂非生产档)+ 显式降级链 --collision ray_query|depth_buffer|off(fail-closed "
            "禁静默换臂:能力缺失 typed 退 3,闭集外 typed 退 2)+ 力场 v1 闭集(gravity_y + "
            "wind_xyz 常量 + 线性阻尼 drag;顺序冻结 vel += (g+wind)·dt → vel ×= (1−drag·dt) → "
            "碰撞查询)。碰撞响应冻结式(RFC-0049 §4.7,host/device 逐字同源):射线 = pos → "
            "pos+vel·dt(dir = vel 不归一,t 域 (0,dt));c = pos+vel·t;n = committed primitive "
            "顶点叉积(TriBvh face_normal 同式同序,命中面 SSBO 镜像按 committed_primitive_index "
            "取顶点;朝向翻转使 n·vel<0);pos' = c + n·eps(eps=1e-3 冻结);v_n = dot(vel,n)·n;"
            "v_t = vel−v_n;vel' = mu_t·v_t − e·v_n(e=0.5/mu_t=0.8 冻结缺省 params 可调);age "
            "照常;未命中/零方向守卫走原 sim 积分。host 金标准 = particles/collision.rs"
            "(apply_fields/collide_step〔TriBvh〕/depth_collide_step 与两 kernel 逐字同源);probe "
            "= bin/g35_collision_device.rs(两 SPV 装载期注入 NoContraction 禁驱动 FMA 收缩,"
            "g35_particle_core 同律)。7 f32 流标定容差(threshold = measured×2.0 程序产禁手写,"
            "RT core t 值 ULP 级差根因)+ flags 整数流零容差位级 + hit 流失配诚实登记 + 同 seed "
            "双跑位级 + tamper-e RED 臂 + frame_ms measured_local 登记。results.trimmed_mean = "
            "绿臂 measured p100 镜像(budget_eval 通用路 evidence_file 消费面)。"
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
    d2 = "sha256:" + "c" * 64
    # 红绿臂①:冻结容差程序读 + threshold = measured × 2.0 协议。
    good_budget = {"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                                "skip_reason": None, "threshold": 1.0e-5, "measured_value": 5.0e-6}]}
    expect(frozen_tol(good_budget) == 1.0e-5, "GREEN:容差程序读正例")
    expect(budget_measured(good_budget) == 5.0e-6, "GREEN:measured_value 程序读正例")
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
    # 红绿臂③:碰撞对拍判。
    good_g = {"f32_max_abs_diff": 5.0e-6, "flags_bitexact": True, "hits_total_host": 986}
    expect(parity_ok(good_g, 1.0e-5), "GREEN:对拍正例(带内 + flags 位级 + 样本非零)")
    expect(parity_ok({"f32_max_abs_diff": 0.0, "flags_bitexact": True, "hits_total_host": 1}, 0.0),
           "GREEN:measured = 0 vs threshold = 0 边界过(零容差零条目)")
    expect(not parity_ok({**good_g, "f32_max_abs_diff": 2.0e-5}, 1.0e-5), "RED:f32 超容差必红")
    expect(not parity_ok({**good_g, "flags_bitexact": False}, 1.0e-5), "RED:flags 非位级必红")
    expect(not parity_ok({**good_g, "hits_total_host": 0}, 1.0e-5), "RED:命中样本零(空转)必红")
    expect(not parity_ok({**good_g, "hits_total_host": True}, 1.0e-5), "RED:bool 冒充命中计数必红")
    expect(not parity_ok({**good_g, "f32_max_abs_diff": float("nan")}, 1.0e-5), "RED:NaN measured 必红")
    expect(not parity_ok(good_g, None), "RED:容差缺失(未标定)必红")
    # 红绿臂④:同帧见证判。
    good_w = {"witness_applicable": True, "box_move_frame": 32, "host_div_static_frame": 32,
              "late_same_at_move_frame": True, "gold_late_differ_at_move_frame": True,
              "box_hits_frame32_host": 9, "box_hits_frame32_device": 9}
    expect(witness_ok(good_w), "GREEN:同帧见证正例")
    expect(not witness_ok({**good_w, "host_div_static_frame": 33}),
           "RED:首异帧 ≠ 突移帧(迟一帧 = Niagara 形态)必红")
    expect(not witness_ok({**good_w, "late_same_at_move_frame": False}),
           "RED:延迟模型突移帧竟有响应(判别器失效)必红")
    expect(not witness_ok({**good_w, "gold_late_differ_at_move_frame": False}),
           "RED:gold/late 不可分辨必红")
    expect(not witness_ok({**good_w, "box_hits_frame32_device": 0}), "RED:device 突移帧零命中必红")
    expect(not witness_ok({**good_w, "box_hits_frame32_host": 0}), "RED:host 突移帧零命中必红")
    expect(not witness_ok({**good_w, "witness_applicable": False}), "RED:见证不适用必红")
    expect(not witness_ok({**good_w, "box_hits_frame32_device": True}), "RED:bool 冒充命中计数必红")
    # 红绿臂⑤:显式降级链判。
    nt = {"state": "typed_error", "typed_error": E_NO_TLAS}
    expect(fallback_ok(3, nt, 2, d0, d1, d2), "GREEN:降级链正例(typed 退 3/2 + 三臂互异)")
    expect(not fallback_ok(0, nt, 2, d0, d1, d2), "RED:no-tlas 退 0(静默换臂嫌疑)必红")
    expect(not fallback_ok(3, {"state": "typed_error", "typed_error": "E_OTHER"}, 2, d0, d1, d2),
           "RED:typed 错误码不符必红")
    expect(not fallback_ok(3, None, 2, d0, d1, d2), "RED:typed evidence 缺失必红")
    expect(not fallback_ok(3, nt, 0, d0, d1, d2), "RED:闭集外取值退 0 必红")
    expect(not fallback_ok(3, nt, 2, d0, d0, d2), "RED:两臂 digest 同(档位别名)必红")
    expect(not fallback_ok(3, nt, 2, "xx", d1, d2), "RED:digest 形态破必红")
    # 红绿臂⑥:力场判。
    good_f = {"arm": "off", "f32_max_abs_diff": 0.0, "wind_dx_positive": True,
              "wind_dz_positive": True, "drag_speed_decay": True, "hits_total_host": 0}
    expect(fields_ok(good_f, 1.0e-5), "GREEN:力场正例")
    expect(not fields_ok({**good_f, "arm": "ray_query"}, 1.0e-5), "RED:臂名不符必红")
    expect(not fields_ok({**good_f, "wind_dx_positive": False}, 1.0e-5), "RED:风 x 漂移不符必红")
    expect(not fields_ok({**good_f, "wind_dz_positive": False}, 1.0e-5), "RED:风 z 漂移不符必红")
    expect(not fields_ok({**good_f, "drag_speed_decay": False}, 1.0e-5), "RED:阻尼未速降必红")
    expect(not fields_ok({**good_f, "hits_total_host": 3}, 1.0e-5), "RED:off 档竟有命中必红")
    expect(not fields_ok({**good_f, "f32_max_abs_diff": 2.0e-5}, 1.0e-5), "RED:off 臂超容差必红")
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
    good_red = {"arm": "tamper-e", "detected": True, "digest_green": d0, "digest_red": d1,
                "red_f32_max_abs_diff": 9.96}
    expect(red_ok(good_red, 1.0e-5), "GREEN:RED 臂正例(digest 异 + measured 溢出容差)")
    expect(not red_ok({**good_red, "detected": False}, 1.0e-5), "RED:漏检必红")
    expect(not red_ok({**good_red, "digest_red": d0}, 1.0e-5), "RED:digest 未变(镂空 digest)必红")
    expect(not red_ok({**good_red, "red_f32_max_abs_diff": 5.0e-6}, 1.0e-5),
           "RED:篡改后 measured 未溢出容差(parity 判据检不出)必红")
    expect(not red_ok({**good_red, "arm": "seed-change"}, 1.0e-5), "RED:臂名不符必红")
    expect(not red_ok({**good_red, "digest_green": "bad"}, 1.0e-5), "RED:digest 形态破必红")
    expect(not red_ok(good_red, None), "RED:容差缺失必红")
    # 红绿臂⑨:frame_ms 健全判。
    expect(frame_ms_sane(92.7), "GREEN:frame_ms 正例")
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
        expect(gs["properties"]["collision_parity"]["properties"]["budget_entry"]["const"] == TOL_ENTRY_ID,
               "gate schema budget_entry const 互核")
        expect(gs["properties"]["fallback_chain"]["properties"]["no_tlas_typed_error"]["const"] == E_NO_TLAS,
               "gate schema no_tlas typed 错误码 const 互核")
        expect(gs["properties"]["red_arm"]["properties"]["arm"]["const"] == "tamper-e",
               "gate schema RED 臂名 const 互核")
        expect(gs["properties"]["scene"]["properties"]["box_move_frame"]["const"] == MOVE_FRAME,
               "gate schema 突移帧 const 互核")
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
    ap.add_argument("--cap", type=int, default=512)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        if args.frames < 64:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 64(方块突移帧 = {MOVE_FRAME} 同帧见证"
                  f" + 反弹/沉降覆盖需 ≥64;64 = 冻结默认窗)", file=sys.stderr)
            return 1
        if args.cap <= 0 or args.cap % 256 != 0:
            print(f"[{TAG}] FAIL: --cap {args.cap} 须为 SEG=256 正整倍数", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.cap, args.seed)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
