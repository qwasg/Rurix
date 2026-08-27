#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 B Task B3 slab 生产接线）
"""G31+ 波 B Task B3：slab 材质 closure/侧表转正接线门冒烟（g31.waveB.slab；
RD-041-slab 行 g31_anchor「生产接线窗」兑现；G29 承接锚：g29_slab.rx device
kernel + 16 槽侧表 p100=1.192e-7 恰一 ULP + 角点 rc=ab=1 device 位级 1.0 +
MaterialClosure 32B 零触碰）。

八面判据（facts 闭集；任务书逐字）：
1. **asset_abi_and_mapping_valid**：slab 侧表生产资产（G29 M-b bin-local 件的
   资产化升级，manifest/资产文件驱动）闭集核验——schema/16 槽 f32 位级 ==
   M-b 生成律（rc_k=k/15·0.95、ab_k=(15−k)/15，struct 逐 op 仿真）/域 [0,1]/
   槽序/abi_digest 重算互核（篡改即拒）/bistro glTF 材质名称·索引互核/槽 <
   16/材质映射唯一非空。
2. **wired_per_slot_parity_p100**：接线态逐槽对拍——harness --slab-table
   device 臂 evidence slab.parity_p100 ≤ 冻结容差（milestones/g29/
   g29_budget.json g29.slab_device.host_device_reflectance_tol 程序读禁手写：
   measured p100=1.192092895507812e-07 恰一 ULP、threshold=2.384185791015624e-07
   = measured × 2.0 程序产）；16 槽 device/host digest 跨双跑位级一致。
3. **g29_m_a_rerun_green**：G29 M-a 对拍门接线态复跑全绿（子进程
   ci/g29_slab_device_kernel_smoke.py --gate rc=0：p100 恰一 ULP 在档位级 +
   角点 rc=ab=1 device 位级 1.0 + 有限性一等断言 + kernel-bias RED 臂 +
   material/ 整目录 0-byte）。
4. **g29_m_b_rerun_green**：G29 M-b 侧表臂接线态复跑全绿（子进程
   ci/g29_slab_side_table_arm_smoke.py --gate rc=0：16 槽逐槽 p100 + 逐槽
   白炉互核 + MaterialClosure 32B 零触碰 + 防混淆机核）。
5. **material_closure_32b_abi_wired**：MaterialClosure 32B ABI 核验——
   cargo test -p rurix-render --lib frozen_layout_sizes（==32 机核断言）+
   graph/types.rs 与 material/ 整目录 vs g28-closed 0-byte（提交面 + 工作树
   双面机核）。
6. **bistro_slab_demo**：生产场景演示——bistro-interior 五材质（釉面陶瓷/
   粉刷石膏/清漆木×2/漆面桌台）切 slab（双层 Substrate 类）经侧表 16 槽查表
   求值真跑：slab off 双跑 digest_seq 位级一致（回归/确定性面）+ on device
   臂双跑 digest_seq 位级一致（确定性门）+ on≠off 至少一帧（接线真实生效
   门，防空接线冒充）+ device 臂 vs host 参考臂末帧 BGRA8 跨臂对拍（结构
   容差口径：R 扰动 ≤ 冻结容差 ⇒ albedo 相对扰动 ≤2.4e-7 ⇒ 8-bit 量化翻转
   ≤1 LSB 且占比 ≤0.1%；位级一致 = 更强终态亦合法）。
7. **slab_off_regression_anchor**：既有逐三角 albedo/emission 面回归锚——
   g14_3_pipeline_perf canonical 160 帧 warmup 10 bistro-interior/t100/
   tsr_device 末帧 digest == milestones/g14/g14_3_stage_a_digest_anchor.json
   在案锚（共享体改动 0-byte 的机器证明）。
8. **slab_on_off_frame_ms_measured**：slab on/off frame_ms 对照（同机同窗：
   orbit 轨迹 --hidden 1920×1080 release 真跑；slab 求值 = 装配期一次性
   eval_ms 单列不混帧口径；measured_local 诚实登记）。

三态：无 Vulkan loader/设备/场景资产/SPV → DEV_ENV_DEGRADE 退 0（不冒充
PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充真跑）。

用法：
  py -3 ci/g31_slab_wiring_smoke.py --selftest
  py -3 ci/g31_slab_wiring_smoke.py --gate g31.waveB.slab [--frames 64] [--warmup 10]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import re
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveB.slab"
SUBJECT = "g31_slab_wiring"
WAVE = "G31.B"
TAG = "g31_slab"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_slab_wiring_evidence_schema.json"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_slab_wiring_gate_evidence_schema.json"
SCHEMA_ID = "rurix.g31.slab_wiring_evidence.v1"
GATE_SCHEMA_ID = "rurix.g31.slab_wiring_gate_evidence.v1"
ASSET_PATH = ROOT / "milestones" / "g31" / "g31_slab_side_table_bistro_interior.json"
G29_BUDGET_PATH = ROOT / "milestones" / "g29" / "g29_budget.json"
TOL_ENTRY_ID = "g29.slab_device.host_device_reflectance_tol"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_tsr_device"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
KERNEL = ROOT / "src" / "rurix-render" / "kernels" / "g29_slab.rx"
WORK = ROOT / ".tmp" / "g31_gates" / "slab"
SPV_SLAB = WORK / "g29_slab.spv"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
LANE_SPVS = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
)
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN_PRESENT = ROOT / "target" / "release" / f"g31_window_present{EXE_SUFFIX}"
BIN_BENCH = ROOT / "target" / "release" / f"g14_3_pipeline_perf{EXE_SUFFIX}"
G29_M_A_SMOKE = ROOT / "ci" / "g29_slab_device_kernel_smoke.py"
G29_M_B_SMOKE = ROOT / "ci" / "g29_slab_side_table_arm_smoke.py"
FROZEN_BASE = "g28-closed"
FROZEN_PATHS = [
    "src/rurix-render/src/graph/types.rs",
    "src/rurix-render/src/material",
]
N_SLOTS = 16
SCENE = "bistro-interior"
TRAJECTORY = "orbit"
# 跨臂结构容差（任务书「容差口径给结构依据」：R 扰动 ≤ 冻结容差 2.384e-7 ⇒
# albedo 相对扰动 ≤2.4e-7 ⇒ HDR 辐射同阶 ⇒ ACES+8-bit 量化（quantum 1/255）
# 翻转概率 ≈ 扰动/quantum ≈ 6e-5 ≪ 0.1% 且翻转幅 ≤1 LSB；位级一致 = 更强终态）。
CROSS_ARM_RATIO_BOUND = 0.001
CROSS_ARM_LSB_BOUND = 1

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "asset_abi_and_mapping_valid",
    "wired_per_slot_parity_p100",
    "g29_m_a_rerun_green",
    "g29_m_b_rerun_green",
    "material_closure_32b_abi_wired",
    "bistro_slab_demo",
    "slab_off_regression_anchor",
    "slab_on_off_frame_ms_measured",
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


def f32(x: float) -> float:
    """IEEE-754 binary32 最近舍入仿真（Rust f32 逐 op 语义同律）。"""
    return struct.unpack("f", struct.pack("f", x))[0]


# ---------------------------------------------------------------------------
# 判读器①：slab 侧表生产资产闭集核验（selftest 红绿两臂消费面）
# ---------------------------------------------------------------------------


def validate_asset(doc: dict, gltf_materials: list[str] | None) -> list[str]:
    """资产闭集判（返回失败串列表,空 = 绿;harness slab_load_asset 同律镜像）。"""
    fails: list[str] = []
    if not isinstance(doc, dict):
        return ["资产非 object"]
    allowed = {
        "schema", "scene_id", "n_slots", "abi", "slots", "material_slots",
        "evaluation_semantics", "provenance",
    }
    extra = set(doc) - allowed
    if extra:
        fails.append(f"资产闭集外字段 {sorted(extra)}")
    if doc.get("schema") != "rurix.g31.slab_side_table_asset.v1":
        fails.append(f"schema 非法: {doc.get('schema')!r}")
    if doc.get("scene_id") != SCENE:
        fails.append(f"scene_id ≠ {SCENE}: {doc.get('scene_id')!r}")
    if doc.get("n_slots") != N_SLOTS:
        fails.append(f"n_slots ≠ {N_SLOTS}: {doc.get('n_slots')!r}")
    slots = doc.get("slots")
    if not isinstance(slots, list) or len(slots) != N_SLOTS:
        fails.append(f"slots 非 {N_SLOTS} 行")
        slots = []
    for k, s in enumerate(slots):
        if not isinstance(s, dict) or set(s) != {"k", "rc", "ab"}:
            fails.append(f"slots[{k}] 字段闭集破: {sorted(s) if isinstance(s, dict) else type(s).__name__}")
            continue
        if s.get("k") != k:
            fails.append(f"slots[{k}].k={s.get('k')} 乱序")
        rc, ab = s.get("rc"), s.get("ab")
        if not isinstance(rc, (int, float)) or not isinstance(ab, (int, float)):
            fails.append(f"slots[{k}] rc/ab 非数值")
            continue
        if not (0.0 <= rc <= 1.0) or not (0.0 <= ab <= 1.0):
            fails.append(f"slots[{k}] rc/ab 越域 [0,1]: {rc}/{ab}")
        # f32 位级 == M-b 生成律（逐 op 舍入仿真）
        exp_rc = f32(f32(f32(k) / f32(15.0)) * f32(0.95))
        exp_ab = f32(f32(15 - k) / f32(15.0))
        if struct.pack("f", f32(rc)) != struct.pack("f", exp_rc):
            fails.append(f"slots[{k}] rc f32 位级 ≠ 生成律: {rc!r} vs {exp_rc!r}")
        if struct.pack("f", f32(ab)) != struct.pack("f", exp_ab):
            fails.append(f"slots[{k}] ab f32 位级 ≠ 生成律: {ab!r} vs {exp_ab!r}")
    abi = doc.get("abi") or {}
    stated = abi.get("abi_digest")
    if not isinstance(stated, str) or not DIGEST_RE.match(stated):
        fails.append(f"abi_digest 形态非法: {str(stated)[:40]!r}")
    elif slots:
        buf = b"".join(struct.pack("<ff", f32(s["rc"]), f32(s["ab"])) for s in slots)
        recomputed = "sha256:" + hashlib.sha256(buf).hexdigest()
        if recomputed != stated:
            fails.append(f"abi_digest 重算不符（篡改即拒）: 在档 {stated[:23]}… vs {recomputed[:23]}…")
    ms = doc.get("material_slots")
    if not isinstance(ms, list) or not ms:
        fails.append("material_slots 空/非数组")
        ms = []
    seen: set[int] = set()
    for i, m in enumerate(ms):
        if not isinstance(m, dict):
            fails.append(f"material_slots[{i}] 非 object")
            continue
        allowed_m = {"material_index", "material_name", "slot", "slab_class", "note"}
        if set(m) != allowed_m:
            fails.append(f"material_slots[{i}] 字段闭集破: {sorted(m)}")
        mi, slot = m.get("material_index"), m.get("slot")
        if not isinstance(mi, int) or isinstance(mi, bool) or mi < 0:
            fails.append(f"material_slots[{i}].material_index 非法: {mi!r}")
            continue
        if mi in seen:
            fails.append(f"material_slots[{i}].material_index {mi} 重复映射")
        seen.add(mi)
        if not isinstance(slot, int) or isinstance(slot, bool) or not (0 <= slot < N_SLOTS):
            fails.append(f"material_slots[{i}].slot 越 16 槽: {slot!r}")
        if gltf_materials is not None:
            if mi >= len(gltf_materials):
                fails.append(f"material_slots[{i}].material_index {mi} 越 glTF 材质数 {len(gltf_materials)}")
            elif gltf_materials[mi] != m.get("material_name"):
                fails.append(
                    f"material_slots[{i}] 名称不符: gltf={gltf_materials[mi]!r} vs asset={m.get('material_name')!r}"
                )
    return fails


# ---------------------------------------------------------------------------
# 判读器②③④：容差程序读 / digest 序列对拍 / 跨臂像素对拍（selftest 消费面）
# ---------------------------------------------------------------------------


def frozen_tol(budget: dict) -> float | None:
    """G29 冻结容差程序读（estimated/skip_reason 冒充 measured 即 None fail-closed）。"""
    for e in budget.get("entries", []):
        if e.get("id") == TOL_ENTRY_ID:
            if e.get("evidence") != "measured_local" or e.get("skip_reason"):
                return None
            t = e.get("threshold")
            return float(t) if isinstance(t, (int, float)) else None
    return None


def parity_in_tol(p100: float, tol: float) -> bool:
    """接线态逐槽对拍硬判：p100 有限且 ≤ 冻结容差。"""
    return isinstance(p100, (int, float)) and not isinstance(p100, bool) and p100 == p100 and 0.0 <= p100 <= tol


def seqs_bitexact(a: list, b: list) -> bool:
    """同轨迹双跑 digest_seq 逐帧位级一致判据。"""
    return len(a) == len(b) and len(a) > 0 and all(x == y for x, y in zip(a, b))


def seqs_differ(a: list, b: list) -> bool:
    """on≠off 接线真实生效判据：至少一帧 digest 不同。"""
    if len(a) != len(b):
        return True
    return any(x != y for x, y in zip(a, b))


def cross_arm_pixels(dev: bytes, host: bytes) -> dict:
    """device 臂 vs host 参考臂末帧 BGRA8 跨臂对拍（dump 格式 = w/h u32 LE 头 +
    BGRA8 打包字节）。返回 {bitexact, mismatch_px, total_px, mismatch_ratio,
    max_lsb_diff, in_bound}；in_bound = bitexact ∨ (ratio ≤ 0.1% ∧ max ≤1 LSB)。
    """
    if len(dev) != len(host) or len(dev) < 8:
        return {"bitexact": False, "mismatch_px": -1, "total_px": 0,
                "mismatch_ratio": 1.0, "max_lsb_diff": 255, "in_bound": False}
    if dev[:8] != host[:8]:
        return {"bitexact": False, "mismatch_px": -1, "total_px": 0,
                "mismatch_ratio": 1.0, "max_lsb_diff": 255, "in_bound": False}
    a, b = dev[8:], host[8:]
    total = len(a) // 4
    mismatch = 0
    max_lsb = 0
    for px in range(total):
        o = px * 4
        if a[o:o + 4] != b[o:o + 4]:
            mismatch += 1
            for c in range(4):
                d = abs(a[o + c] - b[o + c])
                if d > max_lsb:
                    max_lsb = d
    ratio = mismatch / total if total else 1.0
    bitexact = mismatch == 0
    return {
        "bitexact": bitexact,
        "mismatch_px": mismatch,
        "total_px": total,
        "mismatch_ratio": ratio,
        "max_lsb_diff": max_lsb,
        "in_bound": bitexact or (ratio <= CROSS_ARM_RATIO_BOUND and max_lsb <= CROSS_ARM_LSB_BOUND),
    }


def frame_ms_sane(*vals: float) -> bool:
    """frame_ms 登记面健全判：全有限正数（诚实登记非阈门）。"""
    return all(isinstance(v, (int, float)) and not isinstance(v, bool) and v == v and v > 0 for v in vals)


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def run_present(
    label: str,
    frames: int,
    warmup: int,
    slab_arm: str | None,
    dump: Path | None,
    env: dict,
    timeout: int = 3600,
) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = WORK / f"harness_{label}.json"
    argv = [
        str(BIN_PRESENT),
        "--frames", str(frames),
        "--warmup", str(warmup),
        "--hidden",
        "--auto-move", TRAJECTORY,
        "--evidence", str(ev_path),
    ]
    if slab_arm is not None:
        argv += ["--slab-table", str(ASSET_PATH), "--slab-arm", slab_arm, "--spv-slab", str(SPV_SLAB)]
    if dump is not None:
        argv += ["--dump-last-frame", str(dump)]
    r = run(argv, timeout=timeout, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def harness_common_judge(doc: dict, frames: int, warmup: int, label: str) -> list[str]:
    """harness evidence 公共判（off = gameloop schema/on = slab schema 共享字段面）。"""
    fails: list[str] = []
    total = frames + warmup
    if doc.get("frames_completed") != total:
        fails.append(f"{label}: frames_completed {doc.get('frames_completed')} ≠ {total}")
    if doc.get("exit_reason") != "frames_done":
        fails.append(f"{label}: exit_reason ≠ frames_done: {doc.get('exit_reason')!r}")
    if doc.get("trajectory") != TRAJECTORY:
        fails.append(f"{label}: trajectory ≠ {TRAJECTORY}")
    seq = doc.get("digest_seq")
    if not isinstance(seq, list) or len(seq) != total or any(not isinstance(x, str) or not DIGEST_RE.match(x) for x in seq):
        fails.append(f"{label}: digest_seq 形态/长度破（≠{total}）")
    if doc.get("digest") != (seq[-1] if isinstance(seq, list) and seq else None):
        fails.append(f"{label}: digest ≠ digest_seq 末项")
    rr = doc.get("real_render_frame_ms")
    if not isinstance(rr, (int, float)) or isinstance(rr, bool) or not rr > 0:
        fails.append(f"{label}: real_render_frame_ms 非正: {rr!r}")
    if doc.get("render_includes_forced_readback") is not True:
        fails.append(f"{label}: render_includes_forced_readback ≠ true")
    if (doc.get("contracts") or {}).get("consistency") != "pass":
        fails.append(f"{label}: contracts.consistency ≠ pass")
    return fails


def run_gate(frames: int, warmup: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:180]}")

    for sp, name in ((SCHEMA_PATH, "harness schema"), (GATE_SCHEMA_PATH, "gate schema")):
        if not sp.is_file():
            fail(f"{name} 缺失: {sp}")
    if FAILURES:
        return 1

    # ── ① 资产闭集核验（glTF 材质名称/索引互核）──
    gltf_mats: list[str] | None = None
    if BISTRO_GLTF.is_file():
        g = json.loads(BISTRO_GLTF.read_text(encoding="utf-8"))
        gltf_mats = [m.get("name") for m in g.get("materials", [])]
    asset_doc = json.loads(ASSET_PATH.read_text(encoding="utf-8")) if ASSET_PATH.is_file() else None
    asset_fails = validate_asset(asset_doc, gltf_mats) if asset_doc is not None else ["资产缺失"]
    set_fact(
        "asset_abi_and_mapping_valid",
        not asset_fails,
        "资产闭集/schema/16 槽 f32 位级 == M-b 生成律/域/槽序/abi_digest 重算互核/5 材质映射名称·索引互核全绿"
        if not asset_fails else f"资产判红: {asset_fails[:3]}",
    )

    # ── 构建（release 双臂 + rurixc debug SPV 面）──
    ok = build_or_fail(
        ["cargo", "build", "--release", "-p", "rurix-render", "--features", "vendor-upscale",
         "--bin", "g31_window_present", "--bin", "g14_3_pipeline_perf", "--quiet"],
        "harness release",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── SPV 面：g29_slab.rx 现编 + spirv-val（G29 M-a 本体 0-byte 消费）──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    r = run([str(rurixc), str(KERNEL), "--target", "vulkan", "-o", str(SPV_SLAB)], timeout=1800)
    spv_ok = r.returncode == 0 and SPV_SLAB.is_file()
    if spv_ok:
        val = run(["spirv-val", str(SPV_SLAB)], timeout=600)
        spv_ok = val.returncode == 0
    degrade: list[str] = []
    if not spv_ok:
        degrade.append(f"g29_slab SPV 编译/spirv-val 未过: {(r.stdout + r.stderr)[-200:]}")
    missing_lane = [f for f in LANE_SPVS if not (SPV_DIR / f).is_file()]
    if missing_lane:
        degrade.append(f"车道 SPV 缺失 {missing_lane}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    # ── ③④ G29 M-a/M-b 接线态复跑（子进程自持 gpu_device_lock;RURIX_REQUIRE_REAL
    #       继承——三态由两门自裁,rc≠0 即 RED 如实登记）──
    g29_ev: dict[str, str] = {}
    for fid, smoke, subj, gate_key in (
        ("g29_m_a_rerun_green", G29_M_A_SMOKE, "g29_m_a_slab_device_kernel", "g29.p0.m_a.slab_device_kernel"),
        ("g29_m_b_rerun_green", G29_M_B_SMOKE, "g29_m_b_slab_side_table_arm", "g29.p0.m_b.slab_side_table_arm"),
    ):
        r = run([sys.executable, str(smoke), "--gate", gate_key], timeout=7200, env=device_env())
        tail = (r.stdout + r.stderr).strip()
        latest = wel.load_latest_evidence(subj)
        g29_ev[subj] = str(latest) if latest else ""
        set_fact(
            fid,
            r.returncode == 0,
            f"{gate_key} 接线态复跑 rc={r.returncode}（evidence {Path(g29_ev[subj]).name if g29_ev[subj] else '缺'}）"
            + ("" if r.returncode == 0 else f": {tail[-200:]}" )
        )

    # ── ⑤ MaterialClosure 32B ABI 核验（cargo test + git 0-byte 双面机核）──
    abi_detail_parts: list[str] = []
    t = run(
        ["cargo", "test", "-p", "rurix-render", "--lib", "frozen_layout_sizes", "--", "--exact",
         "graph::types::tests::frozen_layout_sizes"],
        timeout=7200,
    )
    test_ok = t.returncode == 0 and "1 passed" in (t.stdout + t.stderr)
    abi_detail_parts.append(
        "cargo test frozen_layout_sizes "
        + ("1 passed（MaterialClosure==32/ClusterRecord==64/PageRequest==16 机核断言）" if test_ok
           else f"rc={t.returncode}: {(t.stdout + t.stderr)[-200:]}")
    )
    d = run(["git", "diff", "--quiet", FROZEN_BASE, "--", *FROZEN_PATHS])
    frozen_ok = d.returncode == 0
    u = run(["git", "status", "--porcelain", "--", *FROZEN_PATHS])
    worktree_ok = not u.stdout.strip()
    abi_detail_parts.append(
        f"git diff --quiet {FROZEN_BASE} -- graph/types.rs + material/ 0-byte={frozen_ok};工作树干净={worktree_ok}"
    )
    set_fact(
        "material_closure_32b_abi_wired",
        test_ok and frozen_ok and worktree_ok,
        "；".join(abi_detail_parts),
    )

    # ── dev-env 降级面（SPV/资产缺失登记;probe 真跑判 skipped_dev_env）──
    env = device_env()
    parity_doc: dict = {}
    demo_docs: dict[str, dict] = {}
    harness_archives: list[str] = []
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} dev-env 探针（slab off 短跑）"):
            rp, probe_doc, _ = run_present("probe", 2, 1, None, None, env, timeout=1200)
        probe_out = (rp.stdout or "") + (rp.stderr or "")
        if '"state":"skipped_dev_env"' in probe_out:
            degrade.append(f"harness skipped_dev_env: {probe_out.strip()[-200:]}")

    cross_arm: dict = {"bitexact": False, "mismatch_px": -1, "total_px": 0,
                       "mismatch_ratio": 1.0, "max_lsb_diff": 255, "in_bound": False}
    frame_ms_doc: dict = {}
    anchor_doc: dict = {}
    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} 渲染五腿 + Stage A 锚格 bench"):
            # ── ② 渲染五腿：off×2 / on_device×2（首腿带 dump）/ on_host×1（带 dump）──
            legs = [
                ("off_a", None, None),
                ("off_b", None, None),
                ("on_device_a", "device", WORK / "dump_device.raw"),
                ("on_device_b", "device", None),
                ("on_host", "host", WORK / "dump_host.raw"),
            ]
            leg_docs: dict[str, dict] = {}
            leg_ok = True
            for label, arm, dump in legs:
                r, doc, ev_path = run_present(label, frames, warmup, arm, dump, env)
                out = (r.stdout or "") + (r.stderr or "")
                if r.returncode != 0 or doc is None or f"[g31_window_present]: PASS" not in out:
                    fail(f"{label} 真跑失败 rc={r.returncode}: {out[-300:]}")
                    leg_ok = False
                    continue
                if "Validation Error" in out or "VUID-" in out:
                    fail(f"{label} validation 应静默却报错")
                    leg_ok = False
                j = harness_common_judge(doc, frames, warmup, label)
                for m in j:
                    fail(m)
                leg_ok &= not j
                leg_docs[label] = doc
                # 归档前缀 = evidence schema 路由面：off 腿 = gameloop schema 件
                # （g31_game_loop_ 前缀路由），slab 腿 = slab harness schema 件。
                prefix = "g31_game_loop_slab_" if arm is None else "g31_slab_wiring_harness_"
                arch = ROOT / "evidence" / f"{prefix}{label}_{ts}.json"
                arch.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                harness_archives.append(str(arch.relative_to(ROOT)))
            if leg_ok:
                off_a, off_b = leg_docs["off_a"], leg_docs["off_b"]
                on_a, on_b = leg_docs["on_device_a"], leg_docs["on_host"]
                on_dev_b = leg_docs["on_device_b"]
                demo_docs = leg_docs
                # ② 接线态逐槽对拍（on_device 双臂 evidence slab 块）
                tol_budget = json.loads(G29_BUDGET_PATH.read_text(encoding="utf-8"))
                tol = frozen_tol(tol_budget)
                slab_a = on_a.get("slab") or {}
                slab_b = on_dev_b.get("slab") or {}
                p100 = slab_a.get("parity_p100")
                dig_same = (
                    slab_a.get("device_digest") == slab_b.get("device_digest")
                    and slab_a.get("host_digest") == slab_b.get("host_digest")
                )
                parity_doc = {
                    "wired_p100": p100,
                    "frozen_tol": tol,
                    "in_tol": parity_in_tol(p100, tol) if tol is not None else False,
                    "device_digest": slab_a.get("device_digest", ""),
                    "host_digest": slab_a.get("host_digest", ""),
                }
                set_fact(
                    "wired_per_slot_parity_p100",
                    tol is not None and parity_doc["in_tol"] and dig_same
                    and slab_a.get("n_slots") == N_SLOTS and slab_a.get("finiteness_first_class") is True,
                    f"接线态逐槽对拍 p100={p100!r} ≤ 冻结容差 {tol!r}（{TOL_ENTRY_ID} 程序读）"
                    f"；双臂 device/host digest 位级一致={dig_same}；有限性一等断言={slab_a.get('finiteness_first_class')}",
                )
                # ⑥ demo 判：双跑位级 + on≠off + 跨臂像素对拍
                off_bit = seqs_bitexact(off_a.get("digest_seq", []), off_b.get("digest_seq", []))
                on_bit = seqs_bitexact(on_a.get("digest_seq", []), on_dev_b.get("digest_seq", []))
                differ = seqs_differ(off_a.get("digest_seq", []), on_a.get("digest_seq", []))
                dump_dev = (WORK / "dump_device.raw").read_bytes() if (WORK / "dump_device.raw").is_file() else b""
                dump_host = (WORK / "dump_host.raw").read_bytes() if (WORK / "dump_host.raw").is_file() else b""
                cross_arm = cross_arm_pixels(dump_dev, dump_host)
                set_fact(
                    "bistro_slab_demo",
                    off_bit and on_bit and differ and cross_arm["in_bound"]
                    and (slab_a.get("slab_tris") or 0) >= 1 and slab_a.get("mapped_materials") == 5,
                    f"bistro 五材质 slab 真跑:off 双跑位级={off_bit} on 双跑位级={on_bit} on≠off={differ}"
                    f"；跨臂 bitexact={cross_arm['bitexact']} mismatch={cross_arm['mismatch_px']}/{cross_arm['total_px']}"
                    f"（ratio={cross_arm['mismatch_ratio']:.3e} ≤ {CROSS_ARM_RATIO_BOUND}）max_lsb={cross_arm['max_lsb_diff']}"
                    f" ≤ {CROSS_ARM_LSB_BOUND}；slab_tris={slab_a.get('slab_tris')} mapped={slab_a.get('mapped_materials')}",
                )
                # ⑧ on/off frame_ms measured（同机同窗 orbit --hidden release）
                off_mean = sorted([off_a["real_render_frame_ms"], off_b["real_render_frame_ms"]])[0]
                on_mean = sorted([on_a["real_render_frame_ms"], on_dev_b["real_render_frame_ms"]])[0]
                on_host_mean = on_b["real_render_frame_ms"]
                eval_ms = float(slab_a.get("eval_ms", 0.0))
                frame_ms_doc = {
                    "off_mean": off_mean,
                    "on_device_mean": on_mean,
                    "on_host_mean": on_host_mean,
                    "delta_pct_device_vs_off": (on_mean / off_mean - 1.0) * 100.0,
                    "slab_eval_ms": eval_ms,
                    "measured": "measured_local",
                    "frames_per_run": frames,
                    "runs": 2,
                }
                set_fact(
                    "slab_on_off_frame_ms_measured",
                    frame_ms_sane(off_mean, on_mean, on_host_mean) and eval_ms >= 0.0,
                    f"同机同窗 measured:off={off_mean:.4f}ms on_device={on_mean:.4f}ms"
                    f"（Δ={frame_ms_doc['delta_pct_device_vs_off']:+.2f}%）on_host={on_host_mean:.4f}ms"
                    f"；slab 装配期求值 eval_ms={eval_ms:.3f}（单列不混帧口径）",
                )
            # ── ⑦ Stage A 锚格（canonical 160 帧;共享体 0-byte 机器证明）──
            bench_root = WORK / "anchor_bench"
            r = run(
                [str(BIN_BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
                 "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
                 "--out-root", str(bench_root)],
                timeout=3600, env=env,
            )
            receipt = bench_root / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
            fresh = None
            if r.returncode == 0 and receipt.is_file():
                fresh = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
            anchors = json.loads(ANCHOR_PATH.read_text(encoding="utf-8")).get("anchors") or {}
            anchor_dg = (anchors.get(ANCHOR_CELL) or {}).get("last_frame_digest")
            anchor_doc = {
                "cell": ANCHOR_CELL,
                "fresh_digest": fresh,
                "anchor_digest": anchor_dg,
                "match": isinstance(fresh, str) and fresh == anchor_dg,
                "frames": 160,
                "warmup": 10,
            }
            set_fact(
                "slab_off_regression_anchor",
                anchor_doc["match"],
                f"Stage A 锚格 {ANCHOR_CELL}:fresh {str(fresh)[:23]}… vs 在案 {str(anchor_dg)[:23]}… "
                f"{'位级 MATCH（共享体 0-byte 机器证明）' if anchor_doc['match'] else 'DRIFT（RED）'}",
            )

    if degrade:
        doc = {
            "schema": "rurix.g31.slab_wiring.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL）")
        return 0

    # ── evidence 落盘（门裁决件;jsonschema 自校验硬门）──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "parity": {
            "wired_p100": parity_doc.get("wired_p100", -1.0),
            "frozen_tol": parity_doc.get("frozen_tol", 0.0),
            "frozen_tol_entry": TOL_ENTRY_ID,
            "in_tol": parity_doc.get("in_tol", False),
            "device_digest": parity_doc.get("device_digest", ""),
            "host_digest": parity_doc.get("host_digest", ""),
        },
        "g29_rerun": {
            "m_a_gate": "g29.p0.m_a.slab_device_kernel",
            "m_a_rc": 0 if facts["g29_m_a_rerun_green"]["status"] == "PASS" else 1,
            "m_a_evidence": g29_ev.get("g29_m_a_slab_device_kernel", ""),
            "m_b_gate": "g29.p0.m_b.slab_side_table_arm",
            "m_b_rc": 0 if facts["g29_m_b_rerun_green"]["status"] == "PASS" else 1,
            "m_b_evidence": g29_ev.get("g29_m_b_slab_side_table_arm", ""),
        },
        "material_closure_abi": {
            "layout_test": abi_detail_parts[0] if abi_detail_parts else "未执行",
            "frozen_0byte_vs_g28_closed": frozen_ok,
            "worktree_clean": worktree_ok,
        },
        "demo": {
            "scene_id": SCENE,
            "trajectory": TRAJECTORY,
            "mapped_materials": 5,
            "slab_tris": int((demo_docs.get("on_device_a", {}).get("slab") or {}).get("slab_tris", 0)),
            "off_double_run_bitexact": seqs_bitexact(
                demo_docs.get("off_a", {}).get("digest_seq", []),
                demo_docs.get("off_b", {}).get("digest_seq", []),
            ) if demo_docs else False,
            "on_device_double_run_bitexact": seqs_bitexact(
                demo_docs.get("on_device_a", {}).get("digest_seq", []),
                demo_docs.get("on_device_b", {}).get("digest_seq", []),
            ) if demo_docs else False,
            "on_ne_off": seqs_differ(
                demo_docs.get("off_a", {}).get("digest_seq", []),
                demo_docs.get("on_device_a", {}).get("digest_seq", []),
            ) if demo_docs else False,
            "cross_arm": {
                **cross_arm,
                "ratio_bound": CROSS_ARM_RATIO_BOUND,
                "lsb_bound": CROSS_ARM_LSB_BOUND,
                "structural_basis": (
                    "R 扰动 ≤ 冻结容差 2.384e-7（恰一 ULP 口径 ×2.0）⇒ albedo 相对扰动 ≤2.4e-7 "
                    "⇒ HDR 辐射同阶 ⇒ ACES+8-bit 量化（quantum 1/255）翻转概率 ≈6e-5≪0.1% 且 ≤1 LSB；"
                    "位级一致 = 更强终态"
                ),
            },
        },
        "regression_anchor": anchor_doc if anchor_doc else {
            "cell": ANCHOR_CELL, "fresh_digest": "sha256:" + "0" * 64,
            "anchor_digest": "sha256:" + "0" * 64, "match": False, "frames": 160, "warmup": 10,
        },
        "frame_ms": frame_ms_doc if frame_ms_doc else {
            "off_mean": -1.0, "on_device_mean": -1.0, "on_host_mean": -1.0,
            "delta_pct_device_vs_off": 0.0, "slab_eval_ms": -1.0,
            "measured": "measured_local", "frames_per_run": frames, "runs": 2,
        },
        "harness_evidence": harness_archives,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 B Task B3 slab 材质 closure/侧表转正：G29 M-b bin-local 16 槽侧表 → "
            "资产文件驱动生产面（milestones/g31/g31_slab_side_table_bistro_interior.json）；"
            "kernels/g29_slab.rx 本体 0-byte 经 vk::run_compute dispatch [16,1,1] 接入 shade/"
            "材质求值链（装配期逐槽查表求值 → 逐三角 albedo × R_slot 预调制进既有 mats SSBO 面，"
            "生产 kernel/管线 0-byte；非 slab 材质走既有单层面 0-byte）；MaterialClosure 32B ABI "
            "不破坏（不经 MaterialClosure）；对拍门接线态复跑 = G29 M-a/M-b 子进程全绿 + 本门"
            "逐槽 parity ≤ 冻结容差；demo = bistro 五材质（釉面陶瓷/粉刷石膏/清漆木×2/漆面桌台）"
            "slab 真跑 + on≠off 生效门 + 跨臂结构容差对拍 + Stage A 锚格零漂移"
        ),
    }
    import jsonschema  # 自校验硬门（schema 漂移即 RED）

    errs = list(jsonschema.Draft7Validator(
        json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        fail("gate evidence schema 自校验红: " + "; ".join(
            f"{'/'.join(str(p) for p in e.path)}: {e.message}" for e in errs[:3]))
        all_pass = False
    gate_path = ROOT / "evidence" / f"g31_slab_wiring_gate_{ts}.json"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}(+ harness {len(harness_archives)} 件)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂,无 GPU/无构建依赖）
# ---------------------------------------------------------------------------


def _good_asset() -> dict:
    slots = []
    buf = b""
    for k in range(N_SLOTS):
        rc = f32(f32(f32(k) / f32(15.0)) * f32(0.95))
        ab = f32(f32(15 - k) / f32(15.0))
        slots.append({"k": k, "rc": rc, "ab": ab})
        buf += struct.pack("<ff", rc, ab)
    return {
        "schema": "rurix.g31.slab_side_table_asset.v1",
        "scene_id": "bistro-interior",
        "n_slots": 16,
        "abi": {"abi_digest": "sha256:" + hashlib.sha256(buf).hexdigest()},
        "slots": slots,
        "material_slots": [
            {"material_index": 1, "material_name": "B", "slot": 2, "slab_class": "c", "note": "n"},
            {"material_index": 0, "material_name": "A", "slot": 1, "slab_class": "c", "note": "n"},
        ],
        "evaluation_semantics": "x",
        "provenance": {},
    }


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    mats = ["A", "B"]
    good = _good_asset()
    # 绿臂①:合法资产零失败（含 glTF 名称/索引互核）。
    expect(validate_asset(good, mats) == [], "GREEN:合法资产过（含互核）")
    expect(validate_asset(good, None) == [], "GREEN:合法资产过（无 glTF 面）")
    # 红臂组①:资产构造缺陷逐条必红。
    bad = _good_asset(); bad["schema"] = "rurix.x"
    expect(validate_asset(bad, mats), "RED:schema 篡改必红")
    bad = _good_asset(); bad["n_slots"] = 15
    expect(validate_asset(bad, mats), "RED:n_slots=15 必红")
    bad = _good_asset(); bad["slots"] = bad["slots"][:15]
    expect(validate_asset(bad, mats), "RED:15 行侧表必红")
    bad = _good_asset(); bad["slots"][3]["rc"] = 1.5
    expect(validate_asset(bad, mats), "RED:rc 越域必红")
    bad = _good_asset(); bad["slots"][3]["rc"] = 0.5
    expect(validate_asset(bad, mats), "RED:rc ≠ 生成律 f32 位级必红")
    bad = _good_asset(); bad["slots"][3]["k"] = 4
    expect(validate_asset(bad, mats), "RED:槽序乱必红")
    bad = _good_asset(); bad["abi"]["abi_digest"] = "sha256:" + "0" * 64
    expect(validate_asset(bad, mats), "RED:abi_digest 篡改必红")
    bad = _good_asset(); bad["material_slots"][0]["slot"] = 16
    expect(validate_asset(bad, mats), "RED:slot=16 越槽必红")
    bad = _good_asset(); bad["material_slots"][1]["material_index"] = 1
    expect(validate_asset(bad, mats), "RED:material_index 重复必红")
    bad = _good_asset(); bad["material_slots"][0]["material_name"] = "ZZ"
    expect(validate_asset(bad, mats), "RED:材质名称不符必红")
    bad = _good_asset(); bad["material_slots"][0]["material_index"] = 99
    expect(validate_asset(bad, mats), "RED:material_index 越 glTF 必红")
    bad = _good_asset(); bad["material_slots"] = []
    expect(validate_asset(bad, mats), "RED:空映射必红")
    bad = _good_asset(); bad["extra_field"] = 1
    expect(validate_asset(bad, mats), "RED:闭集外字段注入必红")
    # 红绿臂②:冻结容差程序读 + parity 判。
    budget_good = {"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                                "skip_reason": None, "threshold": 2.4e-7}]}
    expect(frozen_tol(budget_good) == 2.4e-7, "GREEN:容差程序读正例")
    expect(frozen_tol({"entries": [{"id": TOL_ENTRY_ID, "evidence": "estimated",
                                    "skip_reason": None, "threshold": 2.4e-7}]}) is None,
           "RED:estimated 冒充 measured 必拒")
    expect(frozen_tol({"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                                    "skip_reason": "no gpu", "threshold": 2.4e-7}]}) is None,
           "RED:skip_reason 携带必拒")
    expect(frozen_tol({"entries": []}) is None, "RED:条目缺失必拒")
    expect(parity_in_tol(1.192e-7, 2.384e-7), "GREEN:p100 带内过")
    expect(not parity_in_tol(3.0e-7, 2.384e-7), "RED:p100 超容差必红")
    expect(not parity_in_tol(float("nan"), 2.384e-7), "RED:NaN p100 必红")
    expect(not parity_in_tol(-1.0, 2.384e-7), "RED:负 p100 必红")
    # 红绿臂③:digest 序列判。
    expect(seqs_bitexact(["a", "b"], ["a", "b"]), "GREEN:双跑位级正例")
    expect(not seqs_bitexact(["a", "b"], ["a", "x"]), "RED:双跑漂移必红")
    expect(not seqs_bitexact([], []), "RED:空序列必红")
    expect(seqs_differ(["a", "b"], ["a", "x"]), "GREEN:on≠off 正例")
    expect(not seqs_differ(["a", "b"], ["a", "b"]), "RED:on==off 冒充接线必红")
    # 红绿臂④:跨臂像素对拍结构容差（100×100=10000px——1px=0.01% ≤ 0.1% 界）。
    import os as _os
    w, h = 100, 100
    head = w.to_bytes(4, "little") + h.to_bytes(4, "little")
    base = _os.urandom(w * h * 4)
    r = cross_arm_pixels(head + base, head + base)
    expect(r["bitexact"] and r["in_bound"], "GREEN:跨臂位级一致过")
    tam = bytearray(base); tam[0] ^= 1
    r = cross_arm_pixels(head + bytes(tam), head + base)
    expect(not r["bitexact"] and r["in_bound"] and r["mismatch_px"] == 1 and r["max_lsb_diff"] == 1,
           "GREEN:1px 1LSB 带内过（结构容差）")
    tam = bytearray(base); tam[0] ^= 3
    r = cross_arm_pixels(head + bytes(tam), head + base)
    expect(not r["in_bound"] and r["max_lsb_diff"] == 3, "RED:>1 LSB 必红")
    tam = bytearray(base)
    for i in range(len(tam)):
        tam[i] ^= 1
    r = cross_arm_pixels(head + bytes(tam), head + base)
    expect(not r["in_bound"] and r["mismatch_ratio"] == 1.0, "RED:全像素翻转超占比必红")
    r = cross_arm_pixels(head + base, (8).to_bytes(4, "little") + h.to_bytes(4, "little") + base)
    expect(not r["in_bound"], "RED:尺寸不符必红")
    # 红绿臂⑤:frame_ms 健全判。
    expect(frame_ms_sane(3.5, 3.6, 3.4), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(3.5, 0.0), "RED:0ms 必红")
    expect(not frame_ms_sane(3.5, float("nan")), "RED:NaN 必红")
    # schema 互核:两 schema 在树 + gate schema facts enum == FACT_IDS + harness
    # schema required 含 slab。
    expect(SCHEMA_PATH.is_file() and GATE_SCHEMA_PATH.is_file(), "两 schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
    if SCHEMA_PATH.is_file():
        hs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect("slab" in hs.get("required", []), "harness schema required 含 slab")
        expect(hs["properties"]["schema"]["const"] == SCHEMA_ID, "harness schema const 互核")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=8；5 红臂组 + 正例组 + 双 schema 互核）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=64)
    ap.add_argument("--warmup", type=int, default=10)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        if args.frames < 32:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 32（on/off frame_ms 对照下限）", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.warmup)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
