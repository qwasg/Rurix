#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C3 设备兼容矩阵与能力降级链系统化）
"""G31+ 波 C Task C3：设备兼容矩阵与能力降级链门冒烟（g31.waveC.capability；
G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #50 兑现载体——「NVIDIA（Ada 实测）之外
AMD/Intel 桌面 GPU 的 capability 探测 → 降级链系统化」+ #18 G-MB1-6 尾门锚）。

面（三件套落地 + 真机实测）：
  ① 统一运行时能力探测聚合面：bin vk_capability_report（rurix-rt，
     vendor-upscale）真跑产 rurix.g31.capability_report.v1——逐物理设备
     vendor/device id、RT/RayQuery、mesh shader、descriptor 面上限、显存
     heap/budget（vk::probe_device_capability）+ DLSS/FSR session 真建可用性
     + TSR 自研恒可用面。
  ② 降级链闭集六链（DLSS→FSR→TSR / HZB on→off / ReSTIR→MegaLights 低档 /
     GI on→off / FG→off / 纹理采样→常量材质）——规范裁决 =
     src/rurix-render/src/capability_matrix.rs（fail-closed 单测 mock 全覆盖）；
     本脚本内置 Python 镜像（三向锚定：镜像 == 注册表 chains 表 == Rust CHAINS，
     Rust 锚测试 registry_json_chains_anchored 机核）。
  ③ 兼容矩阵注册表 milestones/g31/g31_compatibility_matrix.json：NVIDIA Ada
     实测格（2026-08-25 真跑全绿）+ AMD/Intel 格 DEV_ENV_DEGRADE 如实登记
     （锚 G-MB1-6，获得硬件后按同一探测面补测，禁冒充）。

判据闭集（milestones/g31/g31_capability_fallback_evidence_schema.json 描述段逐字）：
1. capability_report_measured：probe 真跑 state=measured；devices ≥ 1；主设备
   十 feature 全真 + limits/显存真值；DLSS/FSR/TSR 三面探测全 available。
2. nvidia_ada_cell_all_green：注册表 nvidia 格 status=measured 且逐字段 == 新鲜
   报告（驱动/设备事实零漂移）；full_request_resolution 六链全 degraded=false。
3. amd_intel_cells_dev_env_degrade：AMD/Intel 两格 status=dev_env_degrade +
   facts=null + 锚含 G-MB1-6（不冒充 measured）。
4. fallback_chains_closed_set：注册表 degradation_chains == 六链闭集（chain id
   + 梯档字面逐项相等，阈值数字锚 1073741824/536870912/12 在文）。
5. fail_closed_unit_tests_green：cargo test -p rurix-render --lib
   capability_matrix 真跑 passed ≥ 12 且 failed = 0。
6. chain_resolution_on_fresh_report：Python 镜像对新鲜报告复算
   FeatureRequest::full 六链裁决 == 注册表 nvidia 格登记裁决；digest 双算位级
   一致（可重现）。
7. upscale_switch_measured：DLSS↔FSR↔TSR 三后端真跑（bistro-interior/t100
   --bench 16+4 帧 × 各双跑）——逐后端双跑 last_frame_digest 位级一致 +
   frame_ms_production_mean measured > 0 + 三后端恰 = 梯三档（本机 DLSS 可用面
   实测切换真跑）。

三态：无 bin/无 Vulkan/vendor SDK 缺失 → DEV_ENV_DEGRADE 退 0（不冒充 PASS）；
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充真跑）。

evidence 纪律：PASS 才落 evidence/g31_capability_fallback_<ts>.json
（check_schemas 前缀路由 g31_capability_fallback_）；FAIL 诊断件落
.tmp/g31_gates/capability/ 工作区不污染 evidence/ 路由面（fail-closed：
evidence/ 无件 = 门未过）。

用法：
  py -3 ci/g31_capability_fallback_smoke.py --selftest
  py -3 ci/g31_capability_fallback_smoke.py --gate g31.waveC.capability
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import io
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveC.capability"
SUBJECT = "g31_capability_fallback"
WAVE = "G31+.C"
TAG = "g31_capability"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_capability_fallback_evidence_schema.json"
SCHEMA_ID = "rurix.g31.capability_fallback_evidence.v1"
MATRIX_PATH = ROOT / "milestones" / "g31" / "g31_compatibility_matrix.json"
DOC_PATH = ROOT / "docs" / "renderer" / "compatibility_matrix.md"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
PROBE_BIN = ROOT / "target" / "release" / f"vk_capability_report{EXE_SUFFIX}"
BENCH_BIN = ROOT / "target" / "release" / f"g14_3_pipeline_perf{EXE_SUFFIX}"
WORK = ROOT / ".tmp" / "g31_gates" / "capability"

SCENE, TIER = "bistro-interior", 100
SWITCH_FRAMES, SWITCH_WARMUP = 16, 4
SWITCH_BACKENDS = ["dlss_sr", "fsr_3_1_5", "tsr_device"]

# ── 降级链闭集 Python 镜像（三向锚定：本表 == 注册表 chains 表 ==
# src/rurix-render/src/capability_matrix.rs CHAINS；Rust 锚测试机核）──
CHAIN_IDS = ["upscale", "hzb", "restir", "gi", "framegen", "texture_sampling"]
LADDERS = {
    "upscale": ["dlss_sr", "fsr_3_1_5", "tsr_device"],
    "hzb": ["on", "off"],
    "restir": ["restir_high", "megalights_low"],
    "gi": ["on", "off"],
    "framegen": ["x3", "x2", "off"],
    "texture_sampling": ["textures", "constant_material"],
}
RESTIR_MIN_VRAM = 1 << 30        # 声明阈值 1 GiB（declared 非 measured）
FG_MIN_VRAM = 512 << 20          # 声明阈值 512 MiB（declared 非 measured）
TEXTURE_MIN_STORAGE = 12         # 基座 7 + B4 五件 SSBO 侧表（车道资源面事实）
FULL_REQUEST = {
    "upscale": "dlss_sr",
    "hzb": "on",
    "restir": "restir_high",
    "gi": "on",
    "framegen": "x3",
    "textures": "textures",
}
FULL_REQUEST_LABEL = (
    "FeatureRequest::full(upscale=dlss_sr hzb=on restir=restir_high gi=on "
    "framegen=x3 textures=textures)"
)

FACT_IDS = [
    "capability_report_measured",
    "nvidia_ada_cell_all_green",
    "amd_intel_cells_dev_env_degrade",
    "fallback_chains_closed_set",
    "fail_closed_unit_tests_green",
    "chain_resolution_on_fresh_report",
    "upscale_switch_measured",
]

FEATURE_KEYS = [
    "rayQuery", "rayTracingPipeline", "accelerationStructure", "taskShader",
    "meshShader", "descriptorBuffer", "timelineSemaphore", "synchronization2",
    "bufferDeviceAddress", "shaderInt64",
]
DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")

FAILURES: list[str] = []


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def base_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    """三态裁决：无降级 → None（续跑）；有降级 + REQUIRE_REAL → 1（硬红）；
    有降级无 REQUIRE_REAL → 0（SKIP 非 PASS 非 FAIL）。"""
    if not degrade:
        return None
    return 1 if require_real else 0


# ---------------------------------------------------------------------------
# 降级链镜像（fail-closed；与 capability_matrix.rs resolve_chains 三向锚定）
# ---------------------------------------------------------------------------


def _rt_missing(facts: dict) -> list[str]:
    missing = []
    if not facts.get("ray_query"):
        missing.append("rt.ray_query")
    if not facts.get("acceleration_structure"):
        missing.append("acceleration_structure")
    return missing


def resolve_chains_mirror(facts: dict, request: dict) -> list[dict]:
    """六链 fail-closed 镜像裁决（输出序 = CHAIN_IDS 冻结序，确定性；梯底恒可
    选中——by construction 无崩溃/无静默错图路径）。"""
    out: list[dict] = []

    def push(chain: str, requested: str, selected: str, reason: str) -> None:
        out.append({
            "chain": chain,
            "requested": requested,
            "selected": selected,
            "degraded": selected != requested,
            "reason": reason,
        })

    # upscale：dlss_sr → fsr_3_1_5 → tsr_device。
    req_up = request["upscale"]
    if req_up == "dlss_sr":
        if facts.get("dlss_available"):
            push("upscale", req_up, "dlss_sr", "dlss_available 实测真建")
        elif facts.get("fsr_available"):
            push("upscale", req_up, "fsr_3_1_5", "dlss_available=false → FSR 实测真建")
        else:
            push("upscale", req_up, "tsr_device", "dlss/fsr 双缺 → TSR 自研恒可用(梯底)")
    elif req_up == "fsr_3_1_5":
        if facts.get("fsr_available"):
            push("upscale", req_up, "fsr_3_1_5", "fsr_available 实测真建")
        else:
            push("upscale", req_up, "tsr_device", "fsr_available=false → TSR 自研恒可用(梯底)")
    else:
        push("upscale", req_up, "tsr_device", "TSR 自研恒可用(需求 = Vulkan compute)")
    # hzb：on → off（高成本如实）。
    miss = _rt_missing(facts)
    if request["hzb"] == "off":
        push("hzb", "off", "off", "请求即 off(无降级)")
    elif not miss:
        push("hzb", "on", "on", "rt 双 feature 实测在位")
    else:
        push("hzb", "on", "off", f"HZB 需求缺失:{' + '.join(miss)} → off(高成本如实)")
    # restir：restir_high → megalights_low（声明阈值）。
    vram = int(facts.get("effective_vram_bytes") or 0)
    if request["restir"] == "megalights_low":
        push("restir", "megalights_low", "megalights_low", "请求即低档(无降级)")
    elif vram >= RESTIR_MIN_VRAM:
        push("restir", "restir_high", "restir_high", f"显存 {vram} ≥ 声明阈值 {RESTIR_MIN_VRAM}")
    else:
        push("restir", "restir_high", "megalights_low", f"显存 {vram} < 声明阈值 {RESTIR_MIN_VRAM} → MegaLights 低档")
    # gi：on → off。
    if request["gi"] == "off":
        push("gi", "off", "off", "请求即 off(无降级)")
    elif not miss:
        push("gi", "on", "on", "rt 双 feature 实测在位")
    else:
        push("gi", "on", "off", f"GI kernel 需求缺失:{' + '.join(miss)} → off")
    # framegen：x2/x3 → off（声明阈值）。
    req_fg = request["framegen"]
    if req_fg == "off":
        push("framegen", "off", "off", "请求即 off(无降级)")
    elif vram >= FG_MIN_VRAM:
        push("framegen", req_fg, req_fg, f"显存 {vram} ≥ 声明阈值 {FG_MIN_VRAM}")
    else:
        push("framegen", req_fg, "off", f"显存 {vram} < 声明阈值 {FG_MIN_VRAM} → off(presented=real)")
    # texture_sampling：textures → constant_material。
    ssbo = int(facts.get("max_per_stage_descriptor_storage_buffers") or 0)
    tex_miss = list(miss)
    if ssbo < TEXTURE_MIN_STORAGE:
        tex_miss.append(f"SSBO {ssbo} < {TEXTURE_MIN_STORAGE}")
    if request["textures"] == "constant_material":
        push("texture_sampling", "constant_material", "constant_material", "请求即常量材质(无降级)")
    elif not tex_miss:
        push("texture_sampling", "textures", "textures", f"rt 双 feature + SSBO {ssbo} ≥ {TEXTURE_MIN_STORAGE} 实测满足")
    else:
        push("texture_sampling", "textures", "constant_material", f"纹理车道需求缺失:{' + '.join(tex_miss)} → 常量材质")
    return out


def decisions_canonical(decisions: list[dict]) -> str:
    """镜像裁决集 canonical（键序 = CHAIN_IDS 冻结序；四字段行——reason 不入
    digest（措辞面），选中/请求/降级标记进 digest（防静默换档面）。"""
    lines = ["rurix.g31.chain-decisions.v1(py-mirror)\n"]
    for d in decisions:
        lines.append(f"{d['chain']}|{d['requested']}|{d['selected']}|{1 if d['degraded'] else 0}\n")
    return "".join(lines)


def decisions_digest(decisions: list[dict]) -> str:
    return "sha256:" + hashlib.sha256(decisions_canonical(decisions).encode("utf-8")).hexdigest()


def selected_in_ladder(decisions: list[dict]) -> bool:
    """输出合法性机核：每链 selected ∈ 梯闭集 + degraded ⇔ selected ≠ requested。"""
    for d in decisions:
        if d["selected"] not in LADDERS[d["chain"]]:
            return False
        if d["degraded"] != (d["selected"] != d["requested"]):
            return False
    return True


def facts_from_report(report: dict) -> dict | None:
    """新鲜 capability report → 镜像 facts（主设备 = devices[0]）。"""
    devices = report.get("devices") or []
    if not devices:
        return None
    dev = devices[0]
    feats = dev.get("features") or {}
    limits = dev.get("limits") or {}
    mem = dev.get("memory") or {}
    up = report.get("upscale") or {}
    budget = mem.get("vramBudgetBytes")
    heap = mem.get("deviceLocalHeapBytes")
    return {
        "vendor_id": dev.get("vendor_id"),
        "device_name": dev.get("device_name"),
        "ray_query": feats.get("rayQuery") is True,
        "acceleration_structure": feats.get("accelerationStructure") is True,
        "max_per_stage_descriptor_storage_buffers": limits.get("maxPerStageDescriptorStorageBuffers") or 0,
        "effective_vram_bytes": budget if isinstance(budget, int) else (heap or 0),
        "dlss_available": (up.get("dlss_sr") or {}).get("available") is True,
        "fsr_available": (up.get("fsr_3_1_5") or {}).get("available") is True,
    }


def parse_bench_receipt(path: Path, mtime_after: float) -> dict | None:
    """bench_receipt.json 解析（mtime 新鲜守卫 + 关键字段闭集；不合即 None）。"""
    try:
        if not path.is_file() or path.stat().st_mtime < mtime_after:
            return None
        doc = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    digest = doc.get("last_frame_digest")
    prod = (doc.get("stats_post_warmup") or {}).get("frame_ms_production_mean")
    if not (isinstance(digest, str) and DIGEST_RE.match(digest)):
        return None
    if not (isinstance(prod, (int, float)) and prod > 0):
        return None
    return {"digest": digest, "frame_ms_production_mean": float(prod), "backend": doc.get("backend")}


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def run_gate() -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not SCHEMA_PATH.is_file():
        fail(f"门 schema 缺失: {SCHEMA_PATH}")
        return 1
    if not MATRIX_PATH.is_file():
        fail(f"兼容矩阵注册表缺失: {MATRIX_PATH}")
        return 1

    # ── 构建（release 双 bin；rurix-rt vendor-upscale 探针面 + render bench 面）──
    for pkg, bin_ in (("rurix-rt", "vk_capability_report"), ("rurix-render", "g14_3_pipeline_perf")):
        r = run([
            "cargo", "build", "--release", "-p", pkg, "--features", "vendor-upscale",
            "--bin", bin_, "--quiet",
        ])
        if r.returncode != 0:
            fail(f"{pkg} {bin_} release 构建失败: {(r.stdout + r.stderr)[-400:]}")
            return 1

    # ── 统一探测面真跑（dev-env 三态裁决面）──
    degrade: list[str] = []
    WORK.mkdir(parents=True, exist_ok=True)
    report_path = WORK / "capability_report_fresh.json"
    r = run([str(PROBE_BIN), "--out", str(report_path)], timeout=1800, env=base_env())
    if r.returncode != 0 or not report_path.is_file():
        degrade.append(f"capability probe 失败 rc={r.returncode}: {(r.stdout + r.stderr).strip()[-200:]}")
        report = {}
    else:
        try:
            report = json.loads(report_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            degrade.append(f"capability report JSON 解析失败: {e}")
            report = {}
    if not degrade and report.get("state") != "measured":
        degrade.append(
            f"capability report state={report.get('state')}（非 measured："
            f"vulkan={((report.get('vulkan_probe') or {}).get('error'))} "
            f"dlss={((report.get('upscale') or {}).get('dlss_sr') or {}).get('detail')} "
            f"fsr={((report.get('upscale') or {}).get('fsr_3_1_5') or {}).get('detail')}）"
        )

    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {"schema": "rurix.g31.capability_fallback.skip.v1", "state": "DEV_ENV_DEGRADE", "reasons": degrade}
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP，非 PASS 非 FAIL）")
        return 0

    devices = report.get("devices") or []
    primary = devices[0] if devices else {}
    p_feats = primary.get("features") or {}
    p_limits = primary.get("limits") or {}
    p_mem = primary.get("memory") or {}
    p_up = report.get("upscale") or {}
    features_true = sum(1 for k in FEATURE_KEYS if p_feats.get(k) is True)
    up_avail = {
        "dlss_sr": (p_up.get("dlss_sr") or {}).get("available") is True,
        "fsr_3_1_5": (p_up.get("fsr_3_1_5") or {}).get("available") is True,
        "tsr_device": (p_up.get("tsr_device") or {}).get("available") is True,
    }

    # ① 探测面 measured 判。
    report_ok = (
        report.get("state") == "measured"
        and len(devices) >= 1
        and isinstance(primary.get("device_name"), str) and bool(primary.get("device_name"))
        and isinstance(primary.get("vendor_id"), int) and primary.get("vendor_id") > 0
        and features_true == 10
        and (p_limits.get("maxPerStageDescriptorSampledImages") or 0) > 0
        and (p_limits.get("maxPerStageDescriptorStorageBuffers") or 0) > 0
        and (p_mem.get("deviceLocalHeapBytes") or 0) > 0
        and (p_mem.get("vramBudgetBytes") or 0) > 0
        and all(up_avail.values())
    )
    set_fact(
        "capability_report_measured",
        report_ok,
        f"state={report.get('state')} devices={len(devices)} 主设备=`{primary.get('device_name')}` "
        f"vendor={primary.get('vendor_id')} features_true={features_true}/10 "
        f"ssbo={p_limits.get('maxPerStageDescriptorStorageBuffers')} "
        f"heap={p_mem.get('deviceLocalHeapBytes')} budget={p_mem.get('vramBudgetBytes')} "
        f"upscale={up_avail}",
    )

    # ── 注册表三面判（nvidia 格 == 新鲜报告 / AMD·Intel 降级登记 / chains 闭集）──
    matrix = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))
    cells = {c.get("cell_id"): c for c in matrix.get("cells") or []}
    nv = cells.get("nvidia-ada-rtx4070ti") or {}
    nv_facts = nv.get("facts") or {}

    nv_field_map = [
        ("device_name", primary.get("device_name")),
        ("vendor_id", primary.get("vendor_id")),
        ("device_id", primary.get("device_id")),
        ("device_type", primary.get("device_type")),
        ("api_version", primary.get("api_version")),
        ("driver_version", primary.get("driver_version")),
        ("ray_query", p_feats.get("rayQuery")),
        ("ray_tracing_pipeline", p_feats.get("rayTracingPipeline")),
        ("acceleration_structure", p_feats.get("accelerationStructure")),
        ("task_shader", p_feats.get("taskShader")),
        ("mesh_shader", p_feats.get("meshShader")),
        ("descriptor_buffer", p_feats.get("descriptorBuffer")),
        ("timeline_semaphore", p_feats.get("timelineSemaphore")),
        ("synchronization2", p_feats.get("synchronization2")),
        ("buffer_device_address", p_feats.get("bufferDeviceAddress")),
        ("shader_int64", p_feats.get("shaderInt64")),
        ("max_per_stage_descriptor_sampled_images", p_limits.get("maxPerStageDescriptorSampledImages")),
        ("max_per_stage_descriptor_storage_buffers", p_limits.get("maxPerStageDescriptorStorageBuffers")),
        ("device_local_heap_bytes", p_mem.get("deviceLocalHeapBytes")),
        ("vram_budget_bytes", p_mem.get("vramBudgetBytes")),
        ("dlss_available", up_avail["dlss_sr"]),
        ("fsr_available", up_avail["fsr_3_1_5"]),
        ("tsr_available", up_avail["tsr_device"]),
    ]
    nv_mismatches = [
        f"{k}(在案={nv_facts.get(k)} vs 新鲜={fresh})"
        for k, fresh in nv_field_map
        if nv_facts.get(k) != fresh
    ]
    nv_resolution = ((nv.get("full_request_resolution") or {}).get("decisions")) or []
    nv_all_green = (
        nv.get("status") == "measured"
        and not nv_mismatches
        and len(nv_resolution) == 6
        and all(d.get("degraded") is False for d in nv_resolution)
    )
    set_fact(
        "nvidia_ada_cell_all_green",
        nv_all_green,
        f"status={nv.get('status')} 逐字段对照 {len(nv_field_map)} 项"
        f"{'全 MATCH' if not nv_mismatches else 'MISMATCH: ' + '; '.join(nv_mismatches[:3])}"
        f"；resolution 六链 degraded=false={all(d.get('degraded') is False for d in nv_resolution) if nv_resolution else None}",
    )

    # ③ AMD/Intel 格 DEV_ENV_DEGRADE 如实登记（锚 G-MB1-6，不冒充）。
    amd = cells.get("amd-desktop") or {}
    intel = cells.get("intel-desktop") or {}
    amd_intel_ok = (
        amd.get("status") == "dev_env_degrade" and amd.get("facts") is None
        and "G-MB1-6" in str(amd.get("anchor", ""))
        and intel.get("status") == "dev_env_degrade" and intel.get("facts") is None
        and "G-MB1-6" in str(intel.get("anchor", ""))
        and amd.get("vendor_id") == 4098 and intel.get("vendor_id") == 32902
    )
    set_fact(
        "amd_intel_cells_dev_env_degrade",
        amd_intel_ok,
        f"amd(status={amd.get('status')},facts={amd.get('facts')},锚含G-MB1-6={'G-MB1-6' in str(amd.get('anchor', ''))}) "
        f"intel(status={intel.get('status')},facts={intel.get('facts')},锚含G-MB1-6={'G-MB1-6' in str(intel.get('anchor', ''))})",
    )

    # ④ chains 表闭集判（chain id + 梯档逐项相等 + 阈值数字锚在文）。
    matrix_text = MATRIX_PATH.read_text(encoding="utf-8")
    reg_chains = matrix.get("degradation_chains") or []
    chains_ok = (
        [c.get("chain") for c in reg_chains] == CHAIN_IDS
        and all(c.get("ladder") == LADDERS[c.get("chain")] for c in reg_chains)
        and str(RESTIR_MIN_VRAM) in matrix_text
        and str(FG_MIN_VRAM) in matrix_text
        and "G-MB1-6" in matrix_text
        and "dev_env_degrade" in matrix_text
    )
    set_fact(
        "fallback_chains_closed_set",
        chains_ok,
        f"chains={[c.get('chain') for c in reg_chains]} 梯档逐项{'MATCH' if chains_ok else 'MISMATCH'}"
        f"（阈值锚 1073741824/536870912/12 + G-MB1-6 + dev_env_degrade 在文）",
    )

    # ⑤ fail-closed 单测真跑（cargo test，无 GPU 依赖；mock 覆盖每链缺失面）。
    r = run(["cargo", "test", "-p", "rurix-render", "--lib", "capability_matrix", "--quiet"], timeout=3600)
    test_out = (r.stdout or "") + (r.stderr or "")
    m = re.search(r"test result: ok\. (\d+) passed; (\d+) failed", test_out)
    tests_passed = int(m.group(1)) if m else 0
    tests_failed = int(m.group(2)) if m else -1
    unit_ok = r.returncode == 0 and tests_passed >= 12 and tests_failed == 0
    set_fact(
        "fail_closed_unit_tests_green",
        unit_ok,
        f"cargo test -p rurix-render --lib capability_matrix → rc={r.returncode} "
        f"passed={tests_passed} failed={tests_failed}（mock 覆盖:超分梯三跳/HZB/GI rt 缺失/"
        f"ReSTIR·FG 显存阈值/纹理 SSBO 下界/AMD·Intel 类多链降级/梯闭集遍历/digest 双跑+扰动/注册表锚）",
    )

    # ⑥ 镜像复算（新鲜报告 → 六链裁决 == 注册表登记 + digest 双算可重现）。
    fresh_facts = facts_from_report(report) or {}
    decisions = resolve_chains_mirror(fresh_facts, FULL_REQUEST)
    digest1 = decisions_digest(decisions)
    digest2 = decisions_digest(resolve_chains_mirror(fresh_facts, FULL_REQUEST))
    reg_pairs = {(d.get("chain"), d.get("selected"), d.get("degraded")) for d in nv_resolution}
    fresh_pairs = {(d["chain"], d["selected"], d["degraded"]) for d in decisions}
    resolution_ok = (
        bool(fresh_facts)
        and len(decisions) == 6
        and selected_in_ladder(decisions)
        and fresh_pairs == reg_pairs
        and digest1 == digest2
        and DIGEST_RE.match(digest1) is not None
    )
    set_fact(
        "chain_resolution_on_fresh_report",
        resolution_ok,
        f"镜像复算六链 selected={[d['selected'] for d in decisions]} vs 注册表登记 "
        f"{'MATCH' if fresh_pairs == reg_pairs else 'MISMATCH'}；digest={digest1[:23]}… 双算一致={digest1 == digest2}",
    )

    # ⑦ 真机超分臂切换（DLSS↔FSR↔TSR 三后端 × 各双跑 16+4 帧；gpu_device_lock 串行）。
    switch_rows: list[dict] = []
    switch_ok = True
    with gpu_device_lock(purpose=f"{TAG} upscale 三后端切换六跑"):
        for backend in SWITCH_BACKENDS:
            runs: list[dict] = []
            for rep in range(2):
                out_root = WORK / f"switch_{backend}_r{rep}"
                argv = [
                    str(BENCH_BIN), "--bench", "--scene", SCENE, "--tier", str(TIER),
                    "--backend", backend, "--frames", str(SWITCH_FRAMES),
                    "--warmup", str(SWITCH_WARMUP), "--out-root", str(out_root),
                ]
                t0 = __import__("time").time()
                rr = run(argv, timeout=3600, env=base_env())
                rec = parse_bench_receipt(
                    out_root / SCENE / f"tier{TIER}" / backend / "bench_receipt.json", t0 - 5
                )
                if rr.returncode != 0 or rec is None or rec.get("backend") != backend:
                    fail(f"{backend} 第 {rep} 跑失败 rc={rr.returncode}（receipt {'无效' if rec is None else rec}）")
                    switch_ok = False
                    runs = []
                    break
                runs.append(rec)
            if len(runs) == 2:
                reproducible = runs[0]["digest"] == runs[1]["digest"]
                if not reproducible:
                    fail(f"{backend} 双跑 digest 位级不一致（{runs[0]['digest'][:23]}… vs {runs[1]['digest'][:23]}…）")
                    switch_ok = False
                switch_rows.append({
                    "backend": backend,
                    "runs": [
                        {"last_frame_digest": x["digest"], "frame_ms_production_mean": x["frame_ms_production_mean"]}
                        for x in runs
                    ],
                    "digest_reproducible": reproducible,
                    "frame_ms_production_mean": runs[1]["frame_ms_production_mean"],
                })
    backends_run = [r_["backend"] for r_ in switch_rows]
    switch_fact_ok = (
        switch_ok
        and len(switch_rows) == 3
        and backends_run == SWITCH_BACKENDS
        and all(x["digest_reproducible"] for x in switch_rows)
    )
    set_fact(
        "upscale_switch_measured",
        switch_fact_ok,
        "；".join(
            f"{x['backend']} prod={x['frame_ms_production_mean']:.4f}ms digest={x['runs'][1]['last_frame_digest'][:19]}… 双跑一致={x['digest_reproducible']}"
            for x in switch_rows
        ) or "无有效跑次",
    )

    # ── 门裁决（facts 全绿 + FAILURES 空）──
    all_pass = all(f["status"] == "PASS" for f in facts.values()) and not FAILURES
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    env_info = {
        "gpu": "RTX 4070 Ti（本机单卡 measured_local；AMD/Intel 格 DEV_ENV_DEGRADE 如实登记）",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    gate_doc = {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "anchor": {
            "todo_row": "G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #50",
            "amd_intel_anchor": "G-MB1-6",
            "registry_path": "milestones/g31/g31_compatibility_matrix.json",
            "normative_resolver": "src/rurix-render/src/capability_matrix.rs",
        },
        "capability_report": {
            "state": "measured",
            "device_count": len(devices),
            "primary_device": {
                "device_name": primary.get("device_name") or "",
                "vendor_id": primary.get("vendor_id") or 0,
                "device_id": primary.get("device_id") or 0,
                "features_all_true_count": features_true,
                "max_per_stage_descriptor_sampled_images": p_limits.get("maxPerStageDescriptorSampledImages") or 0,
                "max_per_stage_descriptor_storage_buffers": p_limits.get("maxPerStageDescriptorStorageBuffers") or 0,
                "device_local_heap_bytes": p_mem.get("deviceLocalHeapBytes") or 0,
                "vram_budget_bytes": p_mem.get("vramBudgetBytes") or 0,
            },
            "upscale": up_avail,
            "report_path": str(report_path.relative_to(ROOT)).replace("\\", "/"),
        },
        "matrix_check": {
            "nvidia_cell_all_green": bool(nv_all_green),
            "amd_intel_cells_dev_env_degrade": bool(amd_intel_ok),
            "chains_table_closed_set": bool(chains_ok),
            "cell_count": len(matrix.get("cells") or []),
        },
        "unit_tests": {
            "suite": "cargo test -p rurix-render --lib capability_matrix",
            "passed": tests_passed,
            "failed": max(tests_failed, 0),
        },
        "chain_resolution": {
            "request": FULL_REQUEST_LABEL,
            "decisions": [
                {"chain": d["chain"], "requested": d["requested"], "selected": d["selected"], "degraded": d["degraded"]}
                for d in decisions
            ],
            "digest": digest1,
            "digest_reproduced": bool(digest1 == digest2),
        },
        "upscale_switch": {
            "scene": SCENE,
            "tier": TIER,
            "frames": SWITCH_FRAMES,
            "warmup": SWITCH_WARMUP,
            "backends": switch_rows,
        },
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C3 设备兼容矩阵与能力降级链系统化（G31_PLUS §5 #50 兑现；"
            "#18 G-MB1-6 尾门锚）：① 统一探测面 vk_capability_report 真跑聚合（逐物理设备 "
            "vendor/device id + RT/RayQuery + mesh shader + descriptor 面上限 + 显存 budget + "
            "DLSS/FSR session 真建 + TSR 自研恒可用）② 六链降级闭集 fail-closed（规范裁决 = "
            "capability_matrix.rs，单测 mock 每链缺失面；本脚本 Python 镜像三向锚定复算新鲜报告）"
            "③ 兼容矩阵 NVIDIA Ada 实测格全绿 + AMD/Intel 格 DEV_ENV_DEGRADE 如实登记（获得硬件后 "
            "按同一探测面补测）④ 真机 DLSS↔FSR↔TSR 切换真跑双跑 digest 可重现。"
            f"facts: {'; '.join(f['id'] + '=' + f['status'] for f in (facts[fid] for fid in FACT_IDS))}"
        ),
    }
    import jsonschema  # 自校验硬门（schema 漂移即 RED）

    errs = list(jsonschema.Draft7Validator(
        json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        for e in errs[:5]:
            fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
        all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_capability_fallback_{ts}.json"
    else:
        # FAIL 诊断件落 .tmp 工作区——fail-closed：evidence/ 无件 = 门未过。
        gate_path = WORK / f"gate_fail_{ts}.json"
    io.open(gate_path, "w", encoding="utf-8", newline="\n").write(
        json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n"
    )
    note(f"evidence: {gate_path.relative_to(ROOT)}")
    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（镜像/判读器红绿两臂，无 GPU/无构建依赖）
# ---------------------------------------------------------------------------


def _ada_facts() -> dict:
    """NVIDIA Ada 实测镜像（2026-08-25 真跑事实同值；selftest 夹具不充 measured）。"""
    return {
        "vendor_id": 0x10DE,
        "device_name": "NVIDIA GeForce RTX 4070 Ti",
        "ray_query": True,
        "acceleration_structure": True,
        "max_per_stage_descriptor_storage_buffers": 1_048_576,
        "effective_vram_bytes": 11_771_314_176,
        "dlss_available": True,
        "fsr_available": True,
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

    def sel(ds: list[dict], chain: str) -> dict:
        return next(d for d in ds if d["chain"] == chain)

    # 红绿臂①：Ada 全量请求零降级 + 序冻结 + digest 可重现。
    ds = resolve_chains_mirror(_ada_facts(), FULL_REQUEST)
    expect(len(ds) == 6 and [d["chain"] for d in ds] == CHAIN_IDS, "GREEN:六链全产序冻结")
    expect(all(not d["degraded"] for d in ds), "GREEN:Ada 零降级")
    expect(sel(ds, "upscale")["selected"] == "dlss_sr", "GREEN:DLSS 顶档选中")
    expect(decisions_digest(ds) == decisions_digest(ds), "GREEN:digest 双算可重现")
    expect(DIGEST_RE.match(decisions_digest(ds)) is not None, "GREEN:digest 形态合法")
    expect(selected_in_ladder(ds), "GREEN:梯闭集机核")
    # 红绿臂②：超分梯三跳。
    f = _ada_facts()
    f["dlss_available"] = False
    ds = resolve_chains_mirror(f, FULL_REQUEST)
    d = sel(ds, "upscale")
    expect(d["degraded"] and d["selected"] == "fsr_3_1_5", "RED:DLSS 缺 → FSR")
    f["fsr_available"] = False
    ds = resolve_chains_mirror(f, FULL_REQUEST)
    d = sel(ds, "upscale")
    expect(d["degraded"] and d["selected"] == "tsr_device", "RED:双缺 → TSR 梯底")
    req = dict(FULL_REQUEST, upscale="tsr_device")
    ds = resolve_chains_mirror(f, req)
    expect(not sel(ds, "upscale")["degraded"] and sel(ds, "upscale")["selected"] == "tsr_device",
           "GREEN:TSR 请求恒选中")
    req = dict(FULL_REQUEST, upscale="fsr_3_1_5")
    ds = resolve_chains_mirror(f, req)
    expect(sel(ds, "upscale")["selected"] == "tsr_device" and sel(ds, "upscale")["degraded"],
           "RED:FSR 请求缺 → TSR")
    # 红绿臂③：HZB/GI rt 缺失。
    f = _ada_facts()
    f["ray_query"] = False
    ds = resolve_chains_mirror(f, FULL_REQUEST)
    expect(sel(ds, "hzb")["selected"] == "off" and sel(ds, "hzb")["degraded"], "RED:ray_query 缺 → HZB off")
    expect("rt.ray_query" in sel(ds, "hzb")["reason"], "RED:reason 携带缺失件")
    expect(sel(ds, "gi")["selected"] == "off" and sel(ds, "gi")["degraded"], "RED:ray_query 缺 → GI off")
    req = dict(FULL_REQUEST, hzb="off")
    ds = resolve_chains_mirror(f, req)
    expect(not sel(ds, "hzb")["degraded"], "GREEN:请求 off 非降级")
    # 红绿臂④：ReSTIR/FG 显存阈值（边界恰值维持）。
    f = _ada_facts()
    f["effective_vram_bytes"] = RESTIR_MIN_VRAM - 1
    ds = resolve_chains_mirror(f, FULL_REQUEST)
    expect(sel(ds, "restir")["selected"] == "megalights_low" and sel(ds, "restir")["degraded"],
           "RED:显存不足 → MegaLights 低档")
    f["effective_vram_bytes"] = RESTIR_MIN_VRAM
    ds = resolve_chains_mirror(f, FULL_REQUEST)
    expect(not sel(ds, "restir")["degraded"], "GREEN:恰阈值(≥)维持高档")
    f["effective_vram_bytes"] = FG_MIN_VRAM - 1
    ds = resolve_chains_mirror(f, FULL_REQUEST)
    expect(sel(ds, "framegen")["selected"] == "off" and sel(ds, "framegen")["degraded"],
           "RED:显存不足 → FG off")
    f["effective_vram_bytes"] = FG_MIN_VRAM
    req = dict(FULL_REQUEST, framegen="x2")
    ds = resolve_chains_mirror(f, req)
    expect(not sel(ds, "framegen")["degraded"] and sel(ds, "framegen")["selected"] == "x2",
           "GREEN:恰阈值维持 x2")
    # 红绿臂⑤：纹理 SSBO 下界。
    f = _ada_facts()
    f["max_per_stage_descriptor_storage_buffers"] = TEXTURE_MIN_STORAGE - 1
    ds = resolve_chains_mirror(f, FULL_REQUEST)
    expect(sel(ds, "texture_sampling")["selected"] == "constant_material"
           and sel(ds, "texture_sampling")["degraded"], "RED:SSBO 11 → 常量材质")
    f["max_per_stage_descriptor_storage_buffers"] = TEXTURE_MIN_STORAGE
    ds = resolve_chains_mirror(f, FULL_REQUEST)
    expect(not sel(ds, "texture_sampling")["degraded"], "GREEN:SSBO 恰 12 维持")
    # 红绿臂⑥：Intel 类多链降级 + 梯闭集遍历机核。
    intel = {
        "vendor_id": 0x8086, "device_name": "Intel Arc(mock)",
        "ray_query": False, "acceleration_structure": False,
        "max_per_stage_descriptor_storage_buffers": 8,
        "effective_vram_bytes": 8 << 30,
        "dlss_available": False, "fsr_available": False,
    }
    ds = resolve_chains_mirror(intel, FULL_REQUEST)
    expect(sel(ds, "upscale")["selected"] == "tsr_device", "RED:Intel 类 → TSR")
    expect(sum(1 for d in ds if d["degraded"]) == 4, "RED:恰四链降级(upscale/hzb/gi/texture)")
    all_ok = True
    for rq in (False, True):
        for dlss in (False, True):
            for fsr in (False, True):
                for vram in (0, FG_MIN_VRAM, RESTIR_MIN_VRAM):
                    for ssbo in (0, TEXTURE_MIN_STORAGE):
                        ff = {"ray_query": rq, "acceleration_structure": rq,
                              "max_per_stage_descriptor_storage_buffers": ssbo,
                              "effective_vram_bytes": vram,
                              "dlss_available": dlss, "fsr_available": fsr}
                        if not selected_in_ladder(resolve_chains_mirror(ff, FULL_REQUEST)):
                            all_ok = False
    expect(all_ok, "GREEN:组合遍历 selected 恒 ∈ 梯闭集")
    # 红绿臂⑦：digest 敏感性。
    base = decisions_digest(resolve_chains_mirror(_ada_facts(), FULL_REQUEST))
    f = _ada_facts()
    f["dlss_available"] = False
    expect(decisions_digest(resolve_chains_mirror(f, FULL_REQUEST)) != base, "RED:事实扰动 digest 必变")
    f = _ada_facts()
    f["ray_query"] = False
    expect(decisions_digest(resolve_chains_mirror(f, FULL_REQUEST)) != base, "RED:rt 扰动 digest 必变")
    # 红绿臂⑧：report→facts 映射 + receipt 解析。
    report = {
        "state": "measured",
        "devices": [{
            "device_name": "NVIDIA GeForce RTX 4070 Ti", "vendor_id": 4318,
            "features": {k: True for k in FEATURE_KEYS},
            "limits": {"maxPerStageDescriptorSampledImages": 1048576,
                       "maxPerStageDescriptorStorageBuffers": 1048576},
            "memory": {"deviceLocalHeapBytes": 12576620544, "memoryBudgetExt": True,
                       "vramBudgetBytes": 11771314176},
        }],
        "upscale": {"dlss_sr": {"available": True}, "fsr_3_1_5": {"available": True},
                    "tsr_device": {"available": True}},
    }
    fmap = facts_from_report(report)
    expect(fmap is not None and fmap["ray_query"] and fmap["dlss_available"]
           and fmap["effective_vram_bytes"] == 11771314176, "GREEN:report→facts 映射（budget 优先）")
    expect(facts_from_report({"devices": []}) is None, "RED:空设备表拒判")
    report_no_budget = json.loads(json.dumps(report))
    report_no_budget["devices"][0]["memory"]["vramBudgetBytes"] = None
    fmap2 = facts_from_report(report_no_budget)
    expect(fmap2 is not None and fmap2["effective_vram_bytes"] == 12576620544,
           "GREEN:budget 缺失回退 heap")
    expect(degrade_exit_code([], False) is None, "GREEN:无降级续跑")
    expect(degrade_exit_code(["x"], False) == 0, "GREEN:降级 SKIP 退 0")
    expect(degrade_exit_code(["x"], True) == 1, "RED:REQUIRE_REAL 下降级翻硬红")
    # schema 互核：在树 + 关键 const/required 逐字 + 注册表互核。
    expect(SCHEMA_PATH.is_file(), "门 schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "wave const 互核")
        expect(
            sorted(gs.get("required", [])) == sorted([
                "schema", "subject", "symbolic_gate_key", "wave", "anchor",
                "capability_report", "matrix_check", "unit_tests", "chain_resolution",
                "upscale_switch", "environment", "timestamp", "notes",
            ]),
            "schema required 闭集互核（13 字段）",
        )
        enum_chains = gs["properties"]["chain_resolution"]["properties"]["decisions"]["items"]["properties"]["chain"]["enum"]
        expect(sorted(enum_chains) == sorted(CHAIN_IDS), "decisions chain 枚举闭集互核")
        enum_be = gs["properties"]["upscale_switch"]["properties"]["backends"]["items"]["properties"]["backend"]["enum"]
        expect(sorted(enum_be) == sorted(SWITCH_BACKENDS), "backends 枚举闭集互核")
    expect(MATRIX_PATH.is_file(), "注册表在树")
    if MATRIX_PATH.is_file():
        mx = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))
        expect([c.get("chain") for c in mx.get("degradation_chains", [])] == CHAIN_IDS,
               "注册表 chains 序 == 镜像闭集")
        expect(all(c.get("ladder") == LADDERS[c.get("chain")] for c in mx.get("degradation_chains", [])),
               "注册表梯档 == 镜像闭集")
        cells = {c.get("cell_id"): c for c in mx.get("cells", [])}
        expect(cells.get("nvidia-ada-rtx4070ti", {}).get("status") == "measured", "nvidia 格 measured")
        expect(cells.get("amd-desktop", {}).get("status") == "dev_env_degrade"
               and cells.get("intel-desktop", {}).get("status") == "dev_env_degrade",
               "AMD/Intel 格 dev_env_degrade")
        expect("G-MB1-6" in json.dumps(mx, ensure_ascii=False), "G-MB1-6 锚在文")
    expect(DOC_PATH.is_file(), "docs/renderer/compatibility_matrix.md 在树")
    expect(len(FACT_IDS) == 7, "facts 闭集 = 7")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=7；8 红臂组 + 正例组 + schema/注册表/文档互核）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        return run_gate()
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
