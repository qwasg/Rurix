#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude(G35 GPU 粒子系统 G35-6 事件/数据通道 + particle_view 双向桥)
"""G35-6:事件/数据通道 + particle_view GPU↔host 双向桥门冒烟
(g35.wave6.events;host→GPU 事件队列〔Niagara Data Channels 等价物,
EVENT_CAP=1024 冻结,溢出 = (producer_id, slot, kind, payload_bits) 全序
稳定裁剪保留前 capacity 项 + overflow 如实登记禁静默丢〕+ GPU 事件驱动
二次发射〔死亡粒子 death_flags → 复用 W2 三 scan kernel 稳定槽位 →
g35_event_collect scatter 入 GPU 事件缓冲;下一帧 g35_event_spawn 读
SSBO 计数零回读 + host 队列双源合并发射,次序冻结 host 先 GPU 后,
pid_base 三段涵盖〕+ 统一粒子视图桥〔GPU 九流 readback →
GpuParticleSnapshot → pid 定址读位级 roundtrip;物理侧
ExternalParticlesAdapter plain 数据适配,反打 Niagara GPU↔CPU 互读静默
失败〕——host 金标准 = src/rurix-render/src/particles/events.rs,device
面 = 2 新 kernel〔g35_event_collect/g35_event_spawn〕+ 7 W2 消费面
〔g35_sim/g35_particle_compact/g35_emit/g35_indirect_args + 3 scan〕,
probe = src/rurix-render/src/bin/g35_events_device.rs 经 vk::run_compute
逐 kernel 真跑;RFC-0049 §4.9/评审 F15 修订后基线)。

八面判据(facts 闭集):
1. **kernels_spv_valid**:rurixc 现编 9 kernel(2 新 + 7 W2 消费面)+
   spirv-val 全绿 + 冻结消费面(W2 七 kernel/particles/mod.rs/scan.rs/
   core.rs)sha256 快照在档(G35-6 消费不修改承诺,漂移守护基线)。
2. **event_overflow_payload_stable**:host 队列溢出裁剪稳定(同帧事件集
   正/逆序装配 trim 位级同果)+ 溢出帧 pushed == kept + overflow 如实
   登记(kept ≤ 1024,禁静默丢)+ GPU 死亡侧溢出钳制腿非空转
   (death_overflow_frames ≥ 1,ev_count 双槽 kept/total 如实登记)。
3. **event_spawn_parity**:双源合并发射 device↔host 对拍——src_meta 消费
   见证位级(host 先 GPU 后次序冻结)+ spawn_counts == host 平行推得 +
   发射段 pid 精确区间(pid_base 三段涵盖)+ 全整数流位级 + f32 面
   (发射段/全流 p100)≤ 冻结容差(milestones/g35/g35_budget.json
   g35.events.parity_p100 程序读禁手写:threshold = measured × 2.0 标定
   冻结;缺条目时标定腿先跑 probe --report-max-diff 程序写入)。
4. **gpu_secondary_emission_zero_readback**:死亡→二次发射链零回读——
   device 端 alive/事件计数一律 SSBO 直读(seg_offsets 总和槽/ev_count),
   host 只平行金标准推进不读回 device 计数;ev_count == host〔kept/
   total〕+ spawn_counts == host + secondary 帧数 ≥ 1 且 gpu_accepted
   总量 ≥ 1(样本量门,防空转)。
5. **particle_view_bridge_roundtrip**:方向 A 桥 roundtrip——末帧 device
   九流 readback → GpuParticleSnapshot → pid 定址读 == readback 原值
   位级(probe 腿)+ 物理侧 ExternalParticlesAdapter 单测全绿(cargo
   test -p rurix-physics --lib --features physics-particle-view
   particle_view::external_adapter;plain 数据适配,物理 crate 不依赖
   渲染 device 面)。
6. **determinism_double_run**:同 seed 全链双跑 digest 位级一致(digest =
   所有流字节 sha256 逐帧链式:B 9 流 + 双 scan 中间流 + 事件缓冲 +
   计数 + src_meta + args)。
7. **red_arm_effective**:--red-arm payload-tamper 帧 12 host 事件
   payload 词 0 上传件篡改 +1.0(host 金标准不动)⇒ 与绿链 digest 必异
   (事件 payload 篡改必检出,防镂空 digest 冒充)。
8. **frame_ms_measured**:device 13 dispatch 链逐帧墙钟均值 measured_local
   诚实登记(含 run_compute 逐 dispatch 会话重建开销,非帧率对标)。

三态:无 Vulkan loader/设备/SPV → DEV_ENV_DEGRADE 退 0(不冒充 PASS);
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL(禁 mock 充真跑)。

用法:
  py -3 ci/g35_events_smoke.py --selftest
  py -3 ci/g35_events_smoke.py --gate g35.wave6.events [--frames 64] [--cap 16384] [--seed 42]
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

GATE_KEY = "g35.wave6.events"
SUBJECT = "g35_events"
WAVE = "G35.6"
TAG = "g35_events"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g35" / "g35_events_gate_evidence_schema.json"
GATE_SCHEMA_ID = "rurix.g35.events_gate_evidence.v1"
BUDGET_PATH = ROOT / "milestones" / "g35" / "g35_budget.json"
TOL_ENTRY_ID = "g35.events.parity_p100"
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
EVENT_CAP = 1024
# 2 新 kernel(本波交付)+ 7 W2 消费面(冻结,消费不修改)。
NEW_KERNELS = ("g35_event_collect", "g35_event_spawn")
CONSUMED_KERNELS = (
    "g35_sim",
    "g35_particle_compact",
    "g35_emit",
    "g35_indirect_args",
    "g35_scan_seg_sum",
    "g35_scan_spine",
    "g35_scan_seg_apply",
)
FROZEN_CONSUMED_PATHS = [
    # G35-6 消费不修改承诺面(W2 七 kernel + host core/scan/契约头)——
    # sha256 快照在档 = 漂移守护基线(g35_particle_core FROZEN 同律)。
    "src/rurix-render/kernels/g35_sim.rx",
    "src/rurix-render/kernels/g35_particle_compact.rx",
    "src/rurix-render/kernels/g35_emit.rx",
    "src/rurix-render/kernels/g35_indirect_args.rx",
    "src/rurix-render/kernels/g35_scan_seg_sum.rx",
    "src/rurix-render/kernels/g35_scan_spine.rx",
    "src/rurix-render/kernels/g35_scan_seg_apply.rx",
    "src/rurix-render/src/particles/mod.rs",
    "src/rurix-render/src/particles/scan.rs",
    "src/rurix-render/src/particles/core.rs",
]
ADAPTER_TEST_CMD = [
    "cargo",
    "test",
    "-p",
    "rurix-physics",
    "--lib",
    "--features",
    "physics-particle-view",
    "particle_view::external_adapter",
    "--quiet",
]
WORK = ROOT / ".tmp" / "g35_gates" / "events"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "debug" / f"g35_events_device{EXE_SUFFIX}"

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "kernels_spv_valid",
    "event_overflow_payload_stable",
    "event_spawn_parity",
    "gpu_secondary_emission_zero_readback",
    "particle_view_bridge_roundtrip",
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


def _num(v) -> bool:
    return isinstance(v, (int, float)) and not isinstance(v, bool) and v == v


def _int(v) -> bool:
    return isinstance(v, int) and not isinstance(v, bool)


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
            return float(t) if _num(t) else None
    return None


def budget_measured(budget: dict | None) -> float | None:
    """标定 measured_value 程序读(threshold == measured × 2.0 关系互核面)。"""
    if not isinstance(budget, dict):
        return None
    for e in budget.get("entries", []):
        if e.get("id") == TOL_ENTRY_ID:
            m = e.get("measured_value")
            return float(m) if _num(m) else None
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
                "G35 预算面。G35-6 事件通道:发射段/事件 payload f32 流 device↔host "
                "对拍容差条目由本波标定真跑程序产(threshold = measured × 2.0 冻结 k,"
                "禁手写;measured = 0 时 threshold = 0 零容差零条目)。"
            ),
            "source_docs": ["milestones/g35/g35_events_gate_evidence_schema.json"],
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


def overflow_ok(doc: dict) -> bool:
    """② 溢出裁剪判:溢出帧非空、逐帧 pushed == kept + overflow 且 kept ≤
    EVENT_CAP(如实登记禁静默丢)、乱序 push trim 同果、死亡侧钳制腿
    非空转。"""
    ev = doc.get("event_overflow")
    if not isinstance(ev, dict):
        return False
    frames = ev.get("frames")
    if not isinstance(frames, list) or not frames:
        return False
    for row in frames:
        if not isinstance(row, dict):
            return False
        p, k, o = row.get("pushed"), row.get("kept"), row.get("overflow")
        if not (_int(p) and _int(k) and _int(o)):
            return False
        if p != k + o or k > EVENT_CAP or o < 1 or p <= EVENT_CAP:
            return False
    return (
        ev.get("trim_dual_order_stable") is True
        and _int(ev.get("death_overflow_frames"))
        and ev["death_overflow_frames"] >= 1
    )


def spawn_parity_ok(doc: dict, tol) -> bool:
    """③ 双源发射对拍判:src_meta 位级(次序见证)+ spawn_counts 对拍 +
    发射段 pid 精确区间 + 全整数流位级 + f32 面(发射段与全流 p100)
    ≤ 冻结容差。"""
    spw = doc.get("spawn_parity")
    if not isinstance(spw, dict):
        return False
    seg = doc.get("spawn_seg_f32_max")
    p100 = doc.get("f32_max_abs_diff")
    return (
        spw.get("src_meta_bitexact") is True
        and spw.get("counts_match") is True
        and spw.get("pid_range_exact") is True
        and doc.get("integer_streams_bitexact") is True
        and _num(seg)
        and _num(p100)
        and _num(tol)
        and 0.0 <= seg <= tol
        and 0.0 <= p100 <= tol
    )


def zero_readback_ok(doc: dict) -> bool:
    """④ 零回读二次发射判:ev_count/spawn_counts device↔host 对拍 +
    secondary 帧数与 gpu_accepted 总量样本量门(防空转)。"""
    z = doc.get("zero_readback")
    if not isinstance(z, dict):
        return False
    return (
        z.get("ev_count_match") is True
        and z.get("spawn_counts_match") is True
        and _int(z.get("secondary_frames"))
        and z["secondary_frames"] >= 1
        and _int(z.get("gpu_accepted_total"))
        and z["gpu_accepted_total"] >= 1
    )


def roundtrip_ok(doc: dict, adapter_rc) -> bool:
    """⑤ 双向桥 roundtrip 判:probe 快照 pid 定址位级 + 非空样本 +
    物理侧 adapter 单测 rc == 0。"""
    s = doc.get("snapshot_roundtrip")
    if not isinstance(s, dict):
        return False
    return (
        s.get("ok") is True
        and _int(s.get("checked"))
        and s["checked"] >= 1
        and adapter_rc == 0
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
    """⑦ RED 臂判:payload-tamper 检出 + 双 digest 形态合法且必异。"""
    g, r = doc.get("digest_green"), doc.get("digest_red")
    return (
        doc.get("arm") == "payload-tamper"
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
        "--spv-event-collect", str(WORK / "g35_event_collect.spv"),
        "--spv-event-spawn", str(WORK / "g35_event_spawn.spv"),
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
         "--bin", "g35_events_device", "--quiet"],
        "probe bin",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── ① kernel SPV 面:现编 9 件(2 新 + 7 W2 消费面)+ spirv-val +
    #    冻结消费面 sha256 快照 ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for name in NEW_KERNELS + CONSUMED_KERNELS:
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
        f"rurixc 现编 9 kernel(2 新 g35_event_collect/g35_event_spawn + 7 W2 消费面)+ "
        f"spirv-val={'绿' if spv_ok else '红'};冻结消费面(W2 七 kernel/particles mod.rs/"
        f"scan.rs/core.rs)sha256 快照在档={snapshot_ok}(G35-6 消费不修改承诺,漂移守护基线)",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("G35-6 kernel SPV 编译/spirv-val 未过")

    # ── ⑤ 物理侧 adapter 单测腿(纯 CPU,GPU 锁外)──
    adapter_r = run(ADAPTER_TEST_CMD, timeout=3600)
    adapter_rc = adapter_r.returncode
    adapter_tail = (adapter_r.stdout + adapter_r.stderr).strip()[-300:]
    if adapter_rc != 0:
        fail(f"ExternalParticlesAdapter 单测红 rc={adapter_rc}: {adapter_tail}")

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    gate_path = ROOT / "evidence" / f"g35_events_gate_{ts}.json"
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
                    seg = doc_cal.get("spawn_seg_f32_max")
                    measured = max(float(doc_cal["f32_max_abs_diff"]), float(seg) if _num(seg) else 0.0)
                    tol = calib_threshold(measured)
                    calibrated = True
                    probe_evidence.append(str(ev_cal.relative_to(ROOT)).replace("\\", "/"))
                    pending_entry = {
                        "id": TOL_ENTRY_ID,
                        "description": (
                            "G35-6 事件通道 f32 流 device↔host 对拍容差冻结带(B 组 pos/vel/age/"
                            "life 八流全帧 p100 + GPU 死亡事件 payload〔死亡帧积分后 pos.xyz+vel.xy〕"
                            "+ 双源发射段单独聚合,取三面 max;整数流 flags/scan/death_flags/"
                            "death_scan/ev_meta/ev_count/src_meta/spawn_counts/pid/args 走零容差位级"
                            "不入本条目;sim/emit/event_spawn SPV 装载期注入 NoContraction 禁驱动 "
                            "FMA 收缩后标定;threshold = measured × 2.0 协议冻结 k,measured = 0 时 "
                            "threshold = 0 零容差零条目,方向 max;标定真跑 = ci/g35_events_smoke.py "
                            "--gate g35.wave6.events 标定腿〔g35_events_device 同 seed 双跑位级一致"
                            "面〕;evidence_file = 门裁决件 results.trimmed_mean 镜像槽,budget_eval "
                            "通用路消费;标定程序可复跑)"
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

            # ── 红臂(payload-tamper:事件净荷篡改必检出)──
            if not degrade:
                rc, doc_red, ev_red = run_probe("red", frames, cap, seed, ["--red-arm", "payload-tamper"], env)
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
            "schema": "rurix.g35.events.skip.v1",
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
    ev_overflow = g.get("event_overflow") or {}
    set_fact(
        "event_overflow_payload_stable",
        overflow_ok(g),
        f"host 队列溢出裁剪:溢出帧={ev_overflow.get('frames')!r} 逐帧 pushed == kept + overflow "
        f"如实登记 + 同帧事件集正/逆序装配 trim 位级同果={ev_overflow.get('trim_dual_order_stable')!r}"
        f"(全序键 (producer_id, slot, kind, payload_bits) 冻结)+ GPU 死亡侧溢出钳制腿"
        f"death_overflow_frames={ev_overflow.get('death_overflow_frames')!r} ≥ 1(ev_count 双槽如实登记)",
    )
    spw = g.get("spawn_parity") or {}
    set_fact(
        "event_spawn_parity",
        tol is not None and spawn_parity_ok(g, tol),
        f"双源合并发射对拍:src_meta 消费见证位级={spw.get('src_meta_bitexact')!r}(host 先 GPU 后"
        f"次序冻结)+ spawn_counts == host={spw.get('counts_match')!r} + 发射段 pid 精确区间="
        f"{spw.get('pid_range_exact')!r} + 全整数流位级={g.get('integer_streams_bitexact')!r} + "
        f"f32 面 p100={g.get('f32_max_abs_diff')!r}/发射段={g.get('spawn_seg_f32_max')!r} ≤ 冻结容差 "
        f"{tol!r}({TOL_ENTRY_ID} {'本次标定腿程序产' if calibrated else '程序读'};threshold = measured × 2.0)",
    )
    z = g.get("zero_readback") or {}
    set_fact(
        "gpu_secondary_emission_zero_readback",
        zero_readback_ok(g),
        f"死亡→二次发射链零回读:ev_count(kept/total)== host={z.get('ev_count_match')!r} + "
        f"spawn_counts == host={z.get('spawn_counts_match')!r} + secondary_frames="
        f"{z.get('secondary_frames')!r} ≥ 1,gpu_accepted_total={z.get('gpu_accepted_total')!r} ≥ 1"
        f"(样本量门;device 计数一律 SSBO 直读,host 只平行推进不读回)",
    )
    snap = g.get("snapshot_roundtrip") or {}
    set_fact(
        "particle_view_bridge_roundtrip",
        roundtrip_ok(g, adapter_rc),
        f"方向 A 桥 roundtrip:末帧 device 九流 readback → GpuParticleSnapshot → pid 定址读位级"
        f"={snap.get('ok')!r}(checked={snap.get('checked')!r})+ 物理侧 ExternalParticlesAdapter "
        f"单测 rc={adapter_rc}(plain 数据适配;{adapter_tail[-120:]!r})",
    )
    set_fact(
        "determinism_double_run",
        determinism_ok(g),
        f"同 seed 全链双跑 digest 位级一致={g.get('determinism_double_run')!r}"
        f"(digest_a={str(g.get('digest_a'))[:23]}…;B 9 流+双 scan 中间流+事件缓冲+计数+src_meta"
        f"+args sha256 逐帧链式)",
    )
    set_fact(
        "red_arm_effective",
        red_ok(r_),
        f"RED 臂 payload-tamper:帧 {r_.get('tamper_frame')!r} host 事件 payload 词 0 上传件篡改 "
        f"+1.0 ⇒ digest 必异 detected={r_.get('detected')!r}"
        f"(green={str(r_.get('digest_green'))[:23]}… red={str(r_.get('digest_red'))[:23]}…)",
    )
    fm = g.get("frame_ms_mean")
    set_fact(
        "frame_ms_measured",
        frame_ms_sane(fm),
        f"device 13 dispatch 链逐帧墙钟均值 {fm!r} ms(measured_local 诚实登记;含 run_compute "
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
    seg_num = g.get("spawn_seg_f32_max")
    measured_num = float(g["f32_max_abs_diff"]) if _num(g.get("f32_max_abs_diff")) else -1.0
    if _num(seg_num):
        measured_num = max(measured_num, float(seg_num))
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "kernels": {
            "event_collect_spv": spv_entry("g35_event_collect"),
            "event_spawn_spv": spv_entry("g35_event_spawn"),
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
            "dt": g.get("dt", 1.0 / 15.0),
            "burst": g.get("burst", 16000),
            "emit_schedule": "f0: min(16000, cap-n); else min(32 + f*11 % 96, cap - n_curr)",
            "host_event_schedule": "f12: 1200; f30: 1100; else (f*7) % 5",
            "n_final": g.get("n_final", 0),
            "alive_final": g.get("alive_final", 0),
            "pids_issued": g.get("pids_issued", 0),
        },
        "event_protocol": {
            "event_cap": EVENT_CAP,
            "layout": "meta u32x3 {producer_id, slot, kind} + payload f32x5;死亡事件 payload = 积分后 pos.xyz + vel.xy(kind=1);host 合成事件 kind=2",
            "trim_order": "(producer_id, slot, kind, payload_bits) 全序稳定裁剪保留前 capacity 项",
            "merge_order": "host_first_gpu_after",
        },
        "overflow": {
            "frames": ev_overflow.get("frames") or [],
            "trim_dual_order_stable": bool(ev_overflow.get("trim_dual_order_stable") is True),
            "death_overflow_frames": ev_overflow.get("death_overflow_frames", 0)
            if _int(ev_overflow.get("death_overflow_frames"))
            else 0,
        },
        "spawn_parity": {
            "src_meta_bitexact": bool(spw.get("src_meta_bitexact") is True),
            "counts_match": bool(spw.get("counts_match") is True),
            "pid_range_exact": bool(spw.get("pid_range_exact") is True),
            "integer_streams_bitexact": bool(g.get("integer_streams_bitexact") is True),
        },
        "zero_readback": {
            "ev_count_match": bool(z.get("ev_count_match") is True),
            "spawn_counts_match": bool(z.get("spawn_counts_match") is True),
            "secondary_frames": z.get("secondary_frames", 0) if _int(z.get("secondary_frames")) else 0,
            "gpu_accepted_total": z.get("gpu_accepted_total", 0) if _int(z.get("gpu_accepted_total")) else 0,
            "host_accepted_total": z.get("host_accepted_total", 0) if _int(z.get("host_accepted_total")) else 0,
        },
        "bridge_roundtrip": {
            "snapshot_ok": bool(snap.get("ok") is True),
            "snapshot_checked": snap.get("checked", 0) if _int(snap.get("checked")) else 0,
            "adapter_tests": " ".join(ADAPTER_TEST_CMD),
            "adapter_tests_rc": adapter_rc,
        },
        "f32_parity": {
            "measured_p100": measured_num,
            "spawn_seg_max": float(seg_num) if _num(seg_num) else -1.0,
            "threshold": tol if tol is not None else -1.0,
            "budget_entry": TOL_ENTRY_ID,
            "calibrated_this_run": calibrated,
            "within": bool(tol is not None and spawn_parity_ok(g, tol)),
            "stream_max": g.get("f32_stream_max") or {},
        },
        "results": {"trimmed_mean": measured_num},
        "determinism": {
            "double_run_bitexact": bool(g.get("determinism_double_run") is True),
            "digest_a": g.get("digest_a", "sha256:" + "0" * 64),
            "digest_b": g.get("digest_b", "sha256:" + "0" * 64),
        },
        "red_arm": {
            "arm": "payload-tamper",
            "detected": bool(r_.get("detected") is True),
            "tamper_frame": r_.get("tamper_frame", -1) if _int(r_.get("tamper_frame")) else -1,
            "digest_green": r_.get("digest_green", "sha256:" + "0" * 64),
            "digest_red": r_.get("digest_red", "sha256:" + "0" * 64),
        },
        "frame_ms": {
            "device_chain_mean_ms": fm if frame_ms_sane(fm) else 1e-9,
            "frames_per_run": g.get("frames", frames),
            "measured": "measured_local",
            "note": (
                "device 13 dispatch 链逐帧墙钟均值(vk::run_compute 每 dispatch 重建 "
                "instance/device,该会话开销如实计入;登记语义非帧率对标,生产车道届时走 "
                "DeviceFrameSession 持久车道 + DispatchSpec::Indirect)"
            ),
        },
        "probe_evidence": probe_evidence or ["(probe evidence 缺失)", "(probe evidence 缺失)"],
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G35-6 事件/数据通道 + particle_view 双向桥:host→GPU 事件队列(EVENT_CAP=1024 冻结;"
            "事件 32B = meta u32×3 + payload f32×5;溢出 = (producer_id, slot, kind, payload_bits) "
            "全序稳定裁剪保留前 capacity 项 + overflow 如实登记禁静默丢;EventQueue push/trim 唯一"
            "装配面,每帧整队列上传)+ GPU 事件驱动二次发射零回读(死亡粒子 death_flags = 1 − "
            "alive_flags → 复用 W2 三 scan kernel 稳定死亡槽 → g35_event_collect 两相 scatter 入 "
            "GPU 事件缓冲,ev_count 双槽 kept/total;下一帧 g35_event_spawn 读 SSBO 计数 + host "
            "队列双源合并,次序冻结 host 先 GPU 后,槽位 = alive + scripted + j,pid_base 三段涵盖,"
            "accepted = min(host+gpu, cap − alive − scripted) 与 core 同律;发射随机走 rand_table "
            "单源)+ 统一粒子视图桥(方向 A:device 九流 readback → GpuParticleSnapshot → pid 定址"
            "读位级 roundtrip + 物理侧 ExternalParticlesAdapter plain 数据适配〔v1 演示域 = "
            "ClothVertex 名义寻址,element_index = 快照下标;mass 诚实 Err;impulse = 记账台账〕;"
            "方向 B:host 合成事件 → EventQueue → GPU 发射,v1 演示域不真接物理世界)。host 金标准 "
            "= particles/events.rs(event_collect_step/event_spawn_step/event_frame 与 2 kernel 逐字"
            "同源);probe = bin/g35_events_device.rs(13 dispatch/帧;sim/emit/event_spawn 注入 "
            "NoContraction)。整数流零容差位级 + f32 流标定容差(threshold = measured×2.0 程序产禁"
            "手写)+ 同 seed 双跑位级 + payload-tamper RED 臂 + frame_ms measured_local 登记。"
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
    #    读-改-写保序只增改自己条目)──
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
    # 红绿臂②:budget 读-改-写保序。
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
    expect(skel.get("namespace") == "g35" and skel["entries"] == [mine],
           "GREEN:budget 缺失建 g35 命名空间骨架")
    # 红绿臂③:溢出裁剪判。
    good_ovf = {"event_overflow": {
        "frames": [{"frame": 12, "pushed": 1200, "kept": 1024, "overflow": 176},
                   {"frame": 30, "pushed": 1100, "kept": 1024, "overflow": 76}],
        "trim_dual_order_stable": True, "death_overflow_frames": 11}}
    expect(overflow_ok(good_ovf), "GREEN:溢出裁剪正例(f12/f30 双溢出帧)")
    expect(not overflow_ok({"event_overflow": {**good_ovf["event_overflow"], "frames": []}}),
           "RED:零溢出帧(裁剪腿空转)必红")
    bad_row = dict(good_ovf["event_overflow"])
    bad_row["frames"] = [{"frame": 12, "pushed": 1200, "kept": 1024, "overflow": 100}]
    expect(not overflow_ok({"event_overflow": bad_row}), "RED:pushed ≠ kept + overflow(静默丢)必红")
    bad_kept = dict(good_ovf["event_overflow"])
    bad_kept["frames"] = [{"frame": 12, "pushed": 1200, "kept": 1100, "overflow": 100}]
    expect(not overflow_ok({"event_overflow": bad_kept}), "RED:kept > EVENT_CAP(容量破)必红")
    expect(not overflow_ok({"event_overflow": {**good_ovf["event_overflow"],
                                               "trim_dual_order_stable": False}}),
           "RED:乱序 push 不同果必红")
    expect(not overflow_ok({"event_overflow": {**good_ovf["event_overflow"],
                                               "death_overflow_frames": 0}}),
           "RED:死亡侧钳制腿空转必红")
    expect(not overflow_ok({}), "RED:溢出面缺失必红")
    # 红绿臂④:双源发射对拍判。
    good_spawn = {"spawn_parity": {"src_meta_bitexact": True, "counts_match": True,
                                   "pid_range_exact": True},
                  "integer_streams_bitexact": True,
                  "spawn_seg_f32_max": 1.0e-6, "f32_max_abs_diff": 1.5e-6}
    expect(spawn_parity_ok(good_spawn, 2.0e-6), "GREEN:双源发射正例")
    expect(spawn_parity_ok({**good_spawn, "spawn_seg_f32_max": 0.0, "f32_max_abs_diff": 0.0}, 0.0),
           "GREEN:measured = 0 vs threshold = 0 边界过(零容差零条目)")
    expect(not spawn_parity_ok({**good_spawn,
                                "spawn_parity": {**good_spawn["spawn_parity"],
                                                 "src_meta_bitexact": False}}, 2.0e-6),
           "RED:src_meta 非位级(双源次序破)必红")
    expect(not spawn_parity_ok({**good_spawn,
                                "spawn_parity": {**good_spawn["spawn_parity"],
                                                 "counts_match": False}}, 2.0e-6),
           "RED:spawn_counts 不对拍必红")
    expect(not spawn_parity_ok({**good_spawn,
                                "spawn_parity": {**good_spawn["spawn_parity"],
                                                 "pid_range_exact": False}}, 2.0e-6),
           "RED:pid 区间不精确(三段涵盖破)必红")
    expect(not spawn_parity_ok({**good_spawn, "integer_streams_bitexact": False}, 2.0e-6),
           "RED:整数流非位级必红")
    expect(not spawn_parity_ok({**good_spawn, "f32_max_abs_diff": 3.0e-6}, 2.0e-6),
           "RED:f32 超容差必红")
    expect(not spawn_parity_ok({**good_spawn, "spawn_seg_f32_max": float("nan")}, 2.0e-6),
           "RED:NaN measured 必红")
    expect(not spawn_parity_ok(good_spawn, None), "RED:容差缺失(未标定)必红")
    # 红绿臂⑤:零回读二次发射判。
    good_zero = {"zero_readback": {"ev_count_match": True, "spawn_counts_match": True,
                                   "secondary_frames": 51, "gpu_accepted_total": 47052}}
    expect(zero_readback_ok(good_zero), "GREEN:零回读正例")
    expect(not zero_readback_ok({"zero_readback": {**good_zero["zero_readback"],
                                                   "ev_count_match": False}}),
           "RED:ev_count 不对拍必红")
    expect(not zero_readback_ok({"zero_readback": {**good_zero["zero_readback"],
                                                   "secondary_frames": 0}}),
           "RED:二次发射空转(样本量门)必红")
    expect(not zero_readback_ok({"zero_readback": {**good_zero["zero_readback"],
                                                   "gpu_accepted_total": 0}}),
           "RED:gpu_accepted 总量零必红")
    expect(not zero_readback_ok({"zero_readback": {**good_zero["zero_readback"],
                                                   "secondary_frames": True}}),
           "RED:bool 冒充计数必红")
    expect(not zero_readback_ok({}), "RED:零回读面缺失必红")
    # 红绿臂⑥:双向桥 roundtrip 判。
    good_rt = {"snapshot_roundtrip": {"ok": True, "checked": 12000}}
    expect(roundtrip_ok(good_rt, 0), "GREEN:roundtrip 正例")
    expect(not roundtrip_ok({"snapshot_roundtrip": {"ok": False, "checked": 12000}}, 0),
           "RED:快照定址非位级必红")
    expect(not roundtrip_ok({"snapshot_roundtrip": {"ok": True, "checked": 0}}, 0),
           "RED:零样本(空转)必红")
    expect(not roundtrip_ok(good_rt, 101), "RED:adapter 单测红必红")
    expect(not roundtrip_ok({}, 0), "RED:快照面缺失必红")
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
    good_red = {"arm": "payload-tamper", "detected": True, "digest_green": d0, "digest_red": d1}
    expect(red_ok(good_red), "GREEN:RED 臂正例")
    expect(not red_ok({**good_red, "detected": False}), "RED:漏检必红")
    expect(not red_ok({**good_red, "digest_red": d0}), "RED:digest 未变(镂空 digest)必红")
    expect(not red_ok({**good_red, "arm": "seed-change"}), "RED:臂名不符必红")
    expect(not red_ok({**good_red, "digest_green": "bad"}), "RED:digest 形态破必红")
    # 红绿臂⑨:frame_ms 健全判。
    expect(frame_ms_sane(1531.2), "GREEN:frame_ms 正例")
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
        expect(gs["properties"]["red_arm"]["properties"]["arm"]["const"] == "payload-tamper",
               "gate schema RED 臂名 const 互核")
        expect(gs["properties"]["event_protocol"]["properties"]["event_cap"]["const"] == EVENT_CAP,
               "gate schema EVENT_CAP const 互核")
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
    ap.add_argument("--cap", type=int, default=16384)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        if args.frames < 64:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 64(冻结脚本溢出帧 12/30 + 爆发死亡窗"
                  f"〔帧 13..24〕+ 二次死亡链覆盖需 64 = 冻结默认窗)", file=sys.stderr)
            return 1
        if args.cap < 16384 or args.cap % 256 != 0:
            print(f"[{TAG}] FAIL: --cap {args.cap} 须为 SEG=256 整倍数且 ≥ 16384(帧 0 爆发 "
                  f"16000 + 死亡溢出钳制腿〔单帧死亡 > EVENT_CAP〕需该容量)", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.cap, args.seed)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
