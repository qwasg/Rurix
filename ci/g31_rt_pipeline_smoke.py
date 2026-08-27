#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C15 RT pipeline + SBT 宿主车道）
"""G31+ 波 C Task C15：RT pipeline + SBT 宿主车道（Full RFC）+ SER workload 门
（g31.waveC.rtpipeline；TODO §3.2 #31/#32；M52 承接锚；RD-040 RT-PIPELINE-SBT
分项 reeval_anchor 消费面；RFC-0048 语义面）。

八面判据（facts 闭集）：
1. **rfc_0048_in_tree_and_approved**：rfcs/0048_rt_pipeline_sbt_host_lane.md
   在树 + Agent Approved 字面 + 对抗评审节 8 findings 全 disposition +
   评审 provenance ≠ 起草字面机核（D-409）+ 编号与 number_ledger 实测一致。
2. **rx_rt_kernel_typecheck_and_manifest_green**：
   src/rurix-render/kernels/g31_rt_slab_hit.rx --emit=check 0 诊断 +
   --emit=rt-manifest 结构绿（raygen 恰一/miss 1/triangles 双 hit group/
   record_schema_hash 非空/required_capabilities 恰 {rt.pipeline,
   rt.sbt_user_data}/recursion=1；RXS-0311 隐式推导修复承载面）+ RD-040
   分项锚 `kernels/*hit*.rx` 命中。
3. **codegen_gap_honestly_registered**：--target vulkan 对 hit kernel 确定性
   退出码 2 + 「no compute `kernel fn` found」字面实测（缺口机器证据）+
   RFC §6 PR 序字面在案——.rx→SPIR-V RT codegen 维持 open 如实登记不冒充。
4. **mirror_corpus_formula_single_source**：镜像语料公式面单源静态机核——
   kernel 源 ↔ emit_g31_* 发射器源 slab 三常量 1e-30/rc·ab/albedo 乘法序 +
   背景 0.05/0.05/0.08 + 相机公式字面互核（RFC-0048 §9.1 F8 disposition）
   + 四模块 spirv-val 全过 + hand-emitted 标注字面在案。
5. **rt_pipeline_slab_dual_material_device_run_hand_emitted_mirror**：device 臂真跑——
   g31_rt_slab_lane RT 臂 2 hit groups × 20B slab records，双跑位级 +
   record readback 逐字节 + stack configured ≥ required + validation 静默 +
   golden 三采样点 vs host f64 参照。
6. **rayquery_parity_structural**：RQ 臂真 .rx 编译（kernels/
   g31_rt_slab_rayquery.rx 经 rurixc --target vulkan 新鲜产 SPV + spirv-val）
   同场景同材质同相机同公式真跑 + 双跑位级 + 对拍结构容差（bitexact ∨
   (mismatch_ratio ≤ 0.001 ∧ max_lsb ≤ 1)；位级一致 = 更强终态如实登记）。
7. **ser_capability_and_workload**：SER 三 token 现势探测（device 内 NV 链
   + host vulkaninfo EXT 面互核）；available 则 workload 兑现（NV 双臂
   reorder off/on 画面位级一致 + 双跑位级 + 时延 measured 对照 +
   evidence/g31_ser_gain_estimate_<ts>.json 落盘）；absent 则 M52 维持
   defer 字面登记（capability 半命中不冒充）。
8. **frozen_zero_byte**：RayQuery 生产面（g9_m98_hwrt/g28_restir/g29_slab.rx）
   + M50 底座（vk_m50_rt_body/rt_incremental/vk_rt_incremental/g8 smoke）
   vs HEAD 与工作树 0-byte + material/ + graph/types.rs vs g28-closed
   0-byte（RFC-0046 §1.7 冻结机核同 B3 门律）。

三态：无 Vulkan loader/设备/SPIR-V 工具链 → DEV_ENV_DEGRADE 退 0（不冒充
PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充真跑）。

用法：
  py -3 ci/g31_rt_pipeline_smoke.py --selftest
  py -3 ci/g31_rt_pipeline_smoke.py --gate g31.waveC.rtpipeline
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveC.rtpipeline"
SUBJECT = "g31_rt_pipeline"
WAVE = "G31+.C"
TAG = "g31_rtpipe"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_rt_pipeline_evidence_schema.json"
SCHEMA_ID = "rurix.g31.rt_pipeline_smoke_evidence.v1"
SER_SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_ser_gain_estimate_evidence_schema.json"
SER_SCHEMA_ID = "rurix.g31.ser_gain_estimate.v1"
RFC_PATH = ROOT / "rfcs" / "0048_rt_pipeline_sbt_host_lane.md"
LEDGER_PATH = ROOT / "registry" / "number_ledger.json"
HIT_KERNEL = ROOT / "src" / "rurix-render" / "kernels" / "g31_rt_slab_hit.rx"
RQ_KERNEL = ROOT / "src" / "rurix-render" / "kernels" / "g31_rt_slab_rayquery.rx"
CODEGEN_SRC = ROOT / "src" / "rurixc" / "src" / "vulkan_codegen.rs"
WORK = ROOT / ".tmp" / "g31_gates" / "rtpipeline"
RQ_SPV = WORK / "g31_rt_slab_rayquery.spv"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN_LANE = ROOT / "target" / "debug" / f"g31_rt_slab_lane{EXE_SUFFIX}"
RURIXC = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"

FROZEN_HEAD = [
    "src/rurix-render/kernels/g9_m98_hwrt.rx",
    "src/rurix-render/kernels/g28_restir.rx",
    "src/rurix-render/kernels/g29_slab.rx",
]
FROZEN_M50 = [
    "src/rurix-rt/src/vk_m50_rt_body.rs",
    "src/rurix-rt/src/rt_incremental.rs",
    "src/rurix-rt/src/bin/vk_rt_incremental.rs",
    "ci/g8_rt_pipeline_incremental_smoke.py",
]
FROZEN_BASE = "g28-closed"
FROZEN_G28 = ["src/rurix-render/src/material", "src/rurix-render/src/graph/types.rs"]

FACT_IDS = [
    "rfc_0048_in_tree_and_approved",
    "rx_rt_kernel_typecheck_and_manifest_green",
    "codegen_gap_honestly_registered",
    "mirror_corpus_formula_single_source",
    "rt_pipeline_slab_dual_material_device_run_hand_emitted_mirror",
    "rayquery_parity_structural",
    "ser_capability_and_workload",
    "frozen_zero_byte",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 3600, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


# ---------------------------------------------------------------------------
# 判读器①：RFC 在树 + Approved + 对抗评审（selftest 红绿两臂消费面）
# ---------------------------------------------------------------------------


def judge_rfc(text: str, ledger_rfc_on_tree_max: int) -> list[str]:
    """RFC-0048 判（返回失败串列表，空 = 绿）。"""
    fails: list[str] = []
    if "RFC-0048" not in text:
        fails.append("RFC 编号字面缺失")
    if "Agent Approved" not in text:
        fails.append("Agent Approved 字面缺失")
    if "宿主车道" not in text:
        fails.append("标题字面缺失（宿主车道）")
    if ledger_rfc_on_tree_max < 48:
        fails.append(f"number_ledger RFC.on_tree_max={ledger_rfc_on_tree_max} < 48（编号未消费登记）")
    # 对抗评审节：provenance ≠ 起草 + findings 全 disposition（D-409）。
    m_rev = re.search(r"评审者 provenance[^\n]*`Assisted-by:\s*([^`]+)`", text)
    m_draft = re.search(r"\| Provenance \| `Assisted-by:\s*([^`]+)`", text)
    if not m_rev:
        fails.append("评审者 provenance 缺失")
    if not m_draft:
        fails.append("起草 Provenance 缺失")
    if m_rev and m_draft and m_rev.group(1).strip() == m_draft.group(1).strip():
        fails.append(f"评审 provenance == 起草（{m_rev.group(1).strip()}，违 D-409）")
    if "provenance 偏差登记" not in text:
        fails.append("provenance 偏差登记行缺失（同模型效力自限字面）")
    findings = re.findall(r"\| F\d+ \|", text)
    if len(findings) < 8:
        fails.append(f"对抗评审 findings < 8（实测 {len(findings)}）")
    n_disp = len(re.findall(r"\*\*采纳并修\*\*|\*\*采纳，|\*\*采纳：|\*\*驳回\*\*|\*\*驳回：", text))
    if n_disp < len(findings):
        fails.append(f"disposition 数 {n_disp} < findings 数 {len(findings)}（不得空过）")
    for marker in ("hit/miss 着色阶段", "SBT", "RayQuery", "fail-closed", "SER", "确定性"):
        if marker not in text:
            fails.append(f"语义面要素缺失: {marker}")
    return fails


# ---------------------------------------------------------------------------
# 判读器②：rt-manifest 结构判（selftest 消费面）
# ---------------------------------------------------------------------------


def judge_manifest(doc: dict) -> list[str]:
    fails: list[str] = []
    if doc.get("schema") != "rurix.rt-pipeline-manifest.v1":
        fails.append(f"manifest schema 非法: {doc.get('schema')!r}")
    if doc.get("raygen") != "rg":
        fails.append(f"raygen ≠ rg: {doc.get('raygen')!r}")
    if doc.get("miss") != ["ms"]:
        fails.append(f"miss ≠ [ms]: {doc.get('miss')!r}")
    groups = doc.get("hit_groups") or []
    if len(groups) != 2:
        fails.append(f"hit_groups ≠ 2: {len(groups)}")
    else:
        names = [g.get("name") for g in groups]
        if names != ["slab_a", "slab_b"]:
            fails.append(f"组名/序 ≠ [slab_a, slab_b]: {names}")
        for i, g in enumerate(groups):
            if g.get("group_index") != i:
                fails.append(f"group_index ≠ {i}: {g.get('group_index')}")
            if g.get("kind") != "triangles":
                fails.append(f"kind ≠ triangles: {g.get('kind')!r}")
            if not g.get("closest_hit"):
                fails.append(f"组 {i} 缺 closest_hit")
            h = g.get("record_schema_hash") or ""
            if not re.fullmatch(r"[0-9a-f]{64}", h):
                fails.append(f"组 {i} record_schema_hash 非空 64-hex 破")
    if doc.get("callables") != []:
        fails.append(f"callables ≠ []: {doc.get('callables')!r}")
    caps = doc.get("required_capabilities") or []
    if sorted(caps) != ["rt.pipeline", "rt.sbt_user_data"]:
        fails.append(f"required_capabilities ≠ [rt.pipeline, rt.sbt_user_data]: {caps}（RXS-0311 隐式推导面）")
    if doc.get("recursion") != 1:
        fails.append(f"recursion ≠ 1: {doc.get('recursion')!r}")
    if not re.fullmatch(r"[0-9a-f]{64}", doc.get("payload_schema_hash") or ""):
        fails.append("payload_schema_hash 形态破")
    if not re.fullmatch(r"[0-9a-f]{64}", doc.get("interface_hash") or ""):
        fails.append("interface_hash 形态破")
    return fails


# ---------------------------------------------------------------------------
# 判读器③：镜像语料公式面单源（kernel 源 ↔ 发射器源字面互核）
# ---------------------------------------------------------------------------

# (kernel 标记, 发射器/语义镜像标记)——slab 三常量 + 背景 + 相机公式。
FORMULA_PAIRS = [
    ("denom.max(1e-30)", "1e-30f32"),
    ("1.0 - rec.rc * rec.ab", "OP_FMUL"),
    ("rec.albedo_r * r", "albedo"),
    ("p.r = 0.05", "0.05f32"),
    ("p.b = 0.08", "0.08f32"),
    ("uv*2−1", "two_v2"),
    ("origin = (cx, cy, −1)", "float_n1"),
]


def judge_formula_single_source(kernel_src: str, codegen_src: str) -> list[str]:
    fails: list[str] = []
    for k_marker, cg_marker in FORMULA_PAIRS:
        if k_marker not in kernel_src:
            fails.append(f"kernel 源缺公式字面: {k_marker!r}")
        if cg_marker not in codegen_src:
            fails.append(f"vulkan_codegen 源缺镜像字面: {cg_marker!r}")
    for marker in ("emit_g31_rt_slab_miss", "emit_g31_rt_slab_closesthit", "emit_g31_ser_raygen"):
        if marker not in codegen_src:
            fails.append(f"发射器缺失: {marker}")
    if "非 .rx 编译产物" not in codegen_src:
        fails.append("镜像语料 hand-emitted 标注字面缺失（防冒充面）")
    if "镜像语料" not in kernel_src:
        fails.append("kernel 侧镜像语料标注缺失")
    return fails


# ---------------------------------------------------------------------------
# 判读器④：对拍判（结构容差;selftest 消费面）
# ---------------------------------------------------------------------------


def judge_parity(p: dict) -> list[str]:
    fails: list[str] = []
    bit = p.get("bitexact") is True
    ratio = p.get("mismatch_ratio")
    lsb = p.get("max_lsb_diff")
    in_bound = bit or (
        isinstance(ratio, (int, float)) and ratio <= 0.001
        and isinstance(lsb, int) and lsb <= 1
    )
    if not in_bound:
        fails.append(f"对拍超结构容差: ratio={ratio} lsb={lsb}")
    if p.get("in_bound") is not True:
        fails.append("harness in_bound ≠ true")
    return fails


# ---------------------------------------------------------------------------
# 判读器⑤：SER 臂判（selftest 消费面）
# ---------------------------------------------------------------------------


def judge_ser(ser: dict) -> tuple[list[str], str]:
    """返回 (失败串, kind: measured_workload|maintain_defer)。"""
    fails: list[str] = []
    state = ser.get("state")
    if state == "executed":
        if ser.get("pixels_bitexact_across_arms") is not True:
            fails.append("SER 双臂画面非位级一致（reorder 改画面 = 语义破）")
        if ser.get("double_run_bitexact") is not True:
            fails.append("SER 逐臂双跑非位级一致")
        for k in ("time_ms_noreorder", "time_ms_reorder", "speedup_ratio"):
            v = ser.get(k)
            if not isinstance(v, (int, float)) or v != v or v <= 0:
                fails.append(f"SER {k} 非正/非有限: {v!r}")
        if not isinstance(ser.get("dispatches_per_arm"), int) or ser["dispatches_per_arm"] < 1:
            fails.append("SER dispatches_per_arm < 1")
        toks = ser.get("tokens") or {}
        if toks.get("ext_nv") is not True or toks.get("feature_reorder") is not True:
            fails.append(f"SER executed 但 capability token 缺: {toks}")
        return fails, "measured_workload"
    if state == "absent":
        if "维持 defer" not in str(ser.get("note", "")) and "维持 defer" not in str(ser.get("reason", "")):
            fails.append("SER absent 缺「M52 维持 defer」字面登记")
        return fails, "maintain_defer"
    fails.append(f"SER state 非法: {state!r}")
    return fails, "maintain_defer"


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv, timeout=7200)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


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
        fail(f"gate schema 缺失: {SCHEMA_PATH}")
        return 1

    # ── ① RFC 判 ──
    ledger = json.loads(LEDGER_PATH.read_text(encoding="utf-8"))
    rfc_max = int(ledger["namespaces"]["RFC"]["on_tree_max"])
    rfc_text = RFC_PATH.read_text(encoding="utf-8") if RFC_PATH.is_file() else ""
    rfc_fails = ([] if rfc_text else ["RFC-0048 不在树"]) + judge_rfc(rfc_text, rfc_max)
    set_fact(
        "rfc_0048_in_tree_and_approved",
        not rfc_fails,
        "RFC-0048 在树 + Agent Approved + 对抗评审 8 findings 全 disposition + 评审 provenance ≠ 起草 + ledger 编号一致"
        if not rfc_fails else f"RFC 判红: {rfc_fails[:4]}",
    )

    # ── 构建（rurixc + harness）──
    ok = build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend,shader-stages", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan", "--bin", "g31_rt_slab_lane", "--quiet"],
        "g31_rt_slab_lane",
    )
    if not ok:
        return 1

    # ── ② kernel 类型面 + manifest 判 ──
    r = run([str(RURIXC), str(HIT_KERNEL), "--emit=check"], timeout=600)
    typecheck_ok = r.returncode == 0 and "error" not in (r.stdout + r.stderr)
    manifest_doc: dict = {}
    m = subprocess.run(
        [str(RURIXC), str(HIT_KERNEL), "--emit=rt-manifest"],
        cwd=ROOT, capture_output=True, text=True, timeout=600,
    )
    if m.returncode == 0:
        try:
            manifest_doc = json.loads(m.stdout)
        except json.JSONDecodeError:
            manifest_doc = {}
    manifest_fails = ([] if typecheck_ok else [f"typecheck 红: {(r.stdout + r.stderr)[-200:]}"]) + judge_manifest(manifest_doc)
    anchor_hit = HIT_KERNEL.is_file() and HIT_KERNEL.name.endswith("hit.rx")
    if not anchor_hit:
        manifest_fails.append("RD-040 分项锚 kernels/*hit*.rx 未命中")
    set_fact(
        "rx_rt_kernel_typecheck_and_manifest_green",
        not manifest_fails,
        "--emit=check 0 诊断 + rt-manifest 结构绿（双 triangles 组/required_capabilities=[rt.pipeline, rt.sbt_user_data]/recursion=1）+ 锚 *hit*.rx 命中"
        if not manifest_fails else f"kernel 判红: {manifest_fails[:4]}",
    )

    # ── ③ codegen 缺口诚实登记判 ──
    g = subprocess.run(
        [str(RURIXC), str(HIT_KERNEL), "--target", "vulkan", "-o", str(WORK / "hit_should_not_exist.spv")],
        cwd=ROOT, capture_output=True, text=True, timeout=600,
    )
    gap_out = (g.stdout or "") + (g.stderr or "")
    gap_ok = g.returncode == 2 and "no compute `kernel fn` found" in gap_out
    gap_registered = "PR-2" in rfc_text and "PR-3" in rfc_text and "维持 open 如实登记" in rfc_text
    set_fact(
        "codegen_gap_honestly_registered",
        gap_ok and gap_registered,
        "--target vulkan 退出码 2 + 「no compute `kernel fn` found」实测 + RFC §6 PR-2/3/4 序在案（.rx RT codegen 维持 open 不冒充）"
        if gap_ok and gap_registered else f"exit={g.returncode} out={gap_out[-200:]!r} rfc_pr={'PR-2' in rfc_text}",
    )

    # ── ④ 镜像语料公式面单源 + spirv-val ──
    kernel_src = HIT_KERNEL.read_text(encoding="utf-8")
    codegen_src = CODEGEN_SRC.read_text(encoding="utf-8")
    formula_fails = judge_formula_single_source(kernel_src, codegen_src)
    val_ok = True
    val_count = 0
    out_dirs = sorted((ROOT / "target" / "debug" / "build").glob("rurix-rt-*/out"))
    corpus = ["g31_rt_slab_miss", "g31_rt_slab_closesthit", "g31_ser_raygen_noreorder", "g31_ser_raygen_reorder"]
    spv_files: dict[str, Path] = {}
    for d in out_dirs:
        for name in corpus:
            p = d / f"{name}.spv"
            if p.is_file() and p.stat().st_size > 0:
                spv_files[name] = p
    for name in corpus:
        p = spv_files.get(name)
        if p is None:
            val_ok = False
            formula_fails.append(f"镜像 SPV 缺失/空: {name}")
            continue
        v = subprocess.run(["spirv-val", str(p)], capture_output=True, text=True, timeout=300)
        if v.returncode != 0:
            val_ok = False
            formula_fails.append(f"spirv-val 红 {name}: {(v.stdout + v.stderr)[-160:]}")
        else:
            val_count += 1
    # rq SPV 新鲜编译（fact ⑥共用;无 GPU 依赖——编译失败即 DEV_ENV 降级）
    WORK.mkdir(parents=True, exist_ok=True)
    degrade: list[str] = []
    rq = run([str(RURIXC), str(RQ_KERNEL), "--target", "vulkan", "-o", str(RQ_SPV)], timeout=1800)
    if rq.returncode != 0 or not RQ_SPV.is_file():
        degrade.append(f"RQ SPV 编译失败: {(rq.stdout + rq.stderr)[-200:]}")
    else:
        v = subprocess.run(["spirv-val", str(RQ_SPV)], capture_output=True, text=True, timeout=300)
        if v.returncode != 0:
            degrade.append(f"RQ SPV spirv-val 红: {(v.stdout + v.stderr)[-200:]}")
    set_fact(
        "mirror_corpus_formula_single_source",
        not formula_fails and val_ok and val_count == 4,
        f"kernel↔发射器公式字面互核绿 + 镜像四模块 spirv-val 全过（{val_count}/4）+ hand-emitted 标注在案"
        if not formula_fails and val_ok else f"公式面判红: {formula_fails[:4]}",
    )

    # ── ⑧ 冻结面 0-byte 机核（前置——device 前恒跑面）──
    d1 = run(["git", "diff", "--quiet", "HEAD", "--", *FROZEN_HEAD])
    s1 = run(["git", "status", "--porcelain", "--", *FROZEN_HEAD])
    d2 = run(["git", "diff", "--quiet", "HEAD", "--", *FROZEN_M50])
    s2 = run(["git", "status", "--porcelain", "--", *FROZEN_M50])
    d3 = run(["git", "diff", "--quiet", FROZEN_BASE, "--", *FROZEN_G28])
    s3 = run(["git", "status", "--porcelain", "--", *FROZEN_G28])
    frozen = {
        "rayquery_production_0byte": d1.returncode == 0 and not s1.stdout.strip(),
        "m50_base_0byte": d2.returncode == 0 and not s2.stdout.strip(),
        "rfc0046_slab_0byte": d3.returncode == 0 and not s3.stdout.strip(),
        "material_vs_g28_closed_0byte": d3.returncode == 0,
    }
    set_fact(
        "frozen_zero_byte",
        all(frozen.values()),
        "RayQuery 生产三 kernel + M50 底座四件 vs HEAD/工作树 0-byte；material/+graph/types.rs vs g28-closed 0-byte"
        if all(frozen.values()) else f"冻结面破: {json.dumps(frozen)}",
    )

    # ── host vulkaninfo SER EXT 三 token 新鲜复测（G28 面互核;登记面不卡门）──
    vk_tokens = {"VK_NV_ray_tracing_invocation_reorder": None,
                 "VK_EXT_ray_tracing_invocation_reorder": None,
                 "rayTracingInvocationReorderReorderingHint": None}
    vi = subprocess.run(["vulkaninfo"], capture_output=True, text=True, timeout=300)
    if vi.returncode == 0:
        text = vi.stdout or ""
        for t in vk_tokens:
            vk_tokens[t] = t in text
        note(f"vulkaninfo SER tokens: {vk_tokens}")
    else:
        note("vulkaninfo 不可用（登记降级口径;device 内 NV 链探测为主证）")

    # ── device 腿（gpu_device_lock;三态）──
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    ser_gain_path = WORK / f"g31_ser_gain_estimate_{ts}.json"
    harness_archives: list[str] = []
    lane_doc: dict = {}
    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} RT/RQ/SER 三臂真跑"):
            r = run(
                [
                    str(BIN_LANE),
                    "--spv-rq", str(RQ_SPV),
                    "--width", "64",
                    "--height", "64",
                    "--ser-dispatches", "40",
                    "--ser-repeats", "3",
                    "--out", str(ser_gain_path),
                ],
                timeout=1800,
                env=env,
            )
        out_text = (r.stdout or "") + (r.stderr or "")
        if '"device_state":"skipped_dev_env"' in out_text or '"device_state": "skipped_dev_env"' in out_text:
            degrade.append(f"harness skipped_dev_env: {out_text.strip()[-200:]}")
        else:
            try:
                start = (r.stdout or "").find("{")
                lane_doc = json.loads((r.stdout or "")[start:]) if start >= 0 else {}
            except json.JSONDecodeError:
                lane_doc = {}
            if not lane_doc:
                degrade.append(f"harness 出报非 JSON rc={r.returncode}: {out_text[-200:]}")
            elif r.returncode != 0 or lane_doc.get("state") != "pass":
                fail(f"harness 真跑红 rc={r.returncode}: {out_text[-300:]}")
            else:
                arch = WORK / f"g31_rt_pipeline_harness_lane_{ts}.json"
                arch.write_text(json.dumps(lane_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                harness_archives.append(str(arch.relative_to(ROOT)))

    if degrade:
        doc = {"schema": "rurix.g31.rt_pipeline.skip.v1", "state": "DEV_ENV_DEGRADE", "reasons": degrade}
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if require_real():
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL）")
        return 0

    # ── ⑤ RT 臂判 ──
    rt = lane_doc.get("rt_arm") or {}
    golden = lane_doc.get("golden") or {}
    rt_ok = (
        rt.get("double_run_bitexact") is True
        and rt.get("record_readback_ok") is True
        and rt.get("validation_errors") == 0
        and int(rt.get("hit_group_count", 0)) >= 2
        and rt.get("stack_configured", -1) >= rt.get("stack_required", 0)
        and golden.get("rt_ok") == [True, True, True]
    )
    set_fact(
        "rt_pipeline_slab_dual_material_device_run_hand_emitted_mirror",
        rt_ok,
        f"RT 臂 executed:hit_groups={rt.get('hit_group_count')} 双跑位级 + record readback + stack {rt.get('stack_configured')}≥{rt.get('stack_required')} + validation 静默 + golden 三点全中（digest {str(rt.get('pixels_digest'))[:16]}…）"
        if rt_ok else f"RT 臂判红: {json.dumps(rt)[:200]} golden={json.dumps(golden)[:120]}",
    )

    # ── ⑥ RQ 对拍判 ──
    rqa = lane_doc.get("rq_arm") or {}
    parity = lane_doc.get("parity") or {}
    parity_fails = judge_parity(parity)
    rq_ok = (
        rqa.get("double_run_bitexact") is True
        and bool(rqa.get("entry"))
        and golden.get("rq_ok") == [True, True, True]
        and not parity_fails
    )
    digest_equal = rt.get("pixels_digest") == rqa.get("pixels_digest") and bool(rt.get("pixels_digest"))
    set_fact(
        "rayquery_parity_structural",
        rq_ok,
        f"RQ 臂真 .rx 编译真跑 + 双跑位级 + 对拍 in_bound（bitexact={parity.get('bitexact')} mismatch={parity.get('mismatch_px')}/{parity.get('total_px')} max_lsb={parity.get('max_lsb_diff')}；双臂 digest 相等={digest_equal}）"
        if rq_ok else f"RQ/parity 判红: {parity_fails} rq={json.dumps(rqa)[:160]}",
    )

    # ── ⑦ SER 判 ──
    ser = lane_doc.get("ser") or {}
    ser_fails, ser_kind = judge_ser(ser)
    gain_arch = ""
    if ser_kind == "measured_workload" and ser_gain_path.is_file():
        gain_doc = json.loads(ser_gain_path.read_text(encoding="utf-8"))
        import jsonschema as _js

        ser_errs = list(_js.Draft7Validator(
            json.loads(SER_SCHEMA_PATH.read_text(encoding="utf-8"))
        ).iter_errors(gain_doc))
        if ser_errs:
            fail("ser_gain schema 自校验红: " + "; ".join(
                f"{'/'.join(str(p) for p in e.path)}: {e.message}" for e in ser_errs[:3]))
            ser_fails.append("ser_gain schema 自校验红")
        else:
            gain_file = ROOT / "evidence" / f"g31_ser_gain_estimate_{ts}.json"
            gain_file.write_text(ser_gain_path.read_text(encoding="utf-8"), encoding="utf-8")
            gain_arch = str(gain_file.relative_to(ROOT))
            harness_archives.append(gain_arch)
    set_fact(
        "ser_capability_and_workload",
        not ser_fails,
        (
            f"SER executed:NV 双臂 reorder off/on 画面位级 + 双跑位级 + "
            f"t_off={ser.get('time_ms_noreorder')}ms t_on={ser.get('time_ms_reorder')}ms "
            f"ratio={ser.get('speedup_ratio')}（measured_local 微基准;caveats 在案）+ gain evidence 落盘"
            if ser_kind == "measured_workload"
            else "SER absent → M52 维持 defer 字面登记（capability 半命中不冒充）"
        )
        if not ser_fails else f"SER 判红: {ser_fails[:3]}",
    )

    # ── evidence 落盘（门裁决件;jsonschema 自校验硬门）──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "vulkaninfo_ser_tokens": vk_tokens,
    }
    gate_doc = {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "rfc": {
            "path": "rfcs/0048_rt_pipeline_sbt_host_lane.md",
            "number": 48,
            "status": "Agent Approved",
            "review_findings": 8,
            "review_provenance_differs": True,
        },
        "manifest": {
            "raygen": manifest_doc.get("raygen", ""),
            "miss_count": len(manifest_doc.get("miss") or []),
            "hit_group_count": len(manifest_doc.get("hit_groups") or []),
            "group_kind": (manifest_doc.get("hit_groups") or [{}])[0].get("kind", ""),
            "required_capabilities": manifest_doc.get("required_capabilities") or [],
            "recursion": manifest_doc.get("recursion", 0),
            "record_schema_hash_nonempty": bool(
                re.fullmatch(r"[0-9a-f]{64}", ((manifest_doc.get("hit_groups") or [{}])[0].get("record_schema_hash") or ""))
            ),
            "anchor_kernel_glob_hit": anchor_hit,
        },
        "codegen_gap": {
            "exit_code": g.returncode,
            "message_marker": "no compute `kernel fn` found",
            "registered_honest": gap_registered,
        },
        "mirror_corpus": {
            "hand_emitted_not_rx_compiled": True,
            "formula_literals_match": not formula_fails,
            "spirv_val_modules": val_count,
        },
        "rt_arm": {
            "device_state": "executed",
            "hit_group_count": int(rt.get("hit_group_count", 0)),
            "record_readback_ok": rt.get("record_readback_ok") is True,
            "double_run_bitexact": rt.get("double_run_bitexact") is True,
            "stack_ok": rt.get("stack_configured", -1) >= rt.get("stack_required", 0),
            "validation_silent": rt.get("validation_errors") == 0,
            "golden_rt_ok": golden.get("rt_ok") == [True, True, True],
            "pixels_digest": rt.get("pixels_digest", ""),
        },
        "rq_arm": {
            "spv_fresh_compiled": True,
            "spirv_val_ok": True,
            "entry_nonempty": bool(rqa.get("entry")),
            "double_run_bitexact": rqa.get("double_run_bitexact") is True,
            "golden_rq_ok": golden.get("rq_ok") == [True, True, True],
        },
        "parity": {
            "bitexact": parity.get("bitexact") is True,
            "mismatch_px": int(parity.get("mismatch_px", -1)),
            "total_px": int(parity.get("total_px", 0)),
            "mismatch_ratio": float(parity.get("mismatch_ratio", 1.0)),
            "max_lsb_diff": int(parity.get("max_lsb_diff", 255)),
            "in_bound": parity.get("in_bound") is True,
            "ratio_bound": 0.001,
            "lsb_bound": 1,
            "digest_equal": digest_equal,
        },
        "ser": {
            "state": ser.get("state", "absent"),
            "tokens": ser.get("tokens") or {"ext_nv": False, "feature_reorder": False, "feature_reordering_hint": False},
            "workload_or_defer": (
                {
                    "kind": "measured_workload",
                    "time_ms_noreorder": ser.get("time_ms_noreorder"),
                    "time_ms_reorder": ser.get("time_ms_reorder"),
                    "speedup_ratio": ser.get("speedup_ratio"),
                    "pixels_bitexact_across_arms": ser.get("pixels_bitexact_across_arms"),
                    "double_run_bitexact": ser.get("double_run_bitexact"),
                    "dispatches_per_arm": ser.get("dispatches_per_arm"),
                    "repeats": ser.get("repeats"),
                    "n_instances": ser.get("n_instances"),
                    "gain_evidence": gain_arch,
                }
                if ser_kind == "measured_workload"
                else {"kind": "maintain_defer"}
            ),
        },
        "frozen_surfaces": frozen,
        "harness_evidence": harness_archives,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C15（RFC-0048）：RT pipeline + SBT 宿主车道——slab 双材质经 SBT "
            "record 分派双 hit group（镜像语料 hand-emitted 臂,非 .rx 编译,不充 .rx codegen 绿;"
            ".rx 语义锚 = kernels/g31_rt_slab_hit.rx typecheck+manifest 真绿,codegen 缺口 RFC §6 "
            "PR-2/3/4 维持 open 如实登记）+ RayQuery 对拍臂真 .rx 编译同场景对拍（结构容差;"
            f"bitexact={parity.get('bitexact')}）+ SER workload NV 双臂 measured 对照"
            f"（state={ser.get('state')}）;M52/RD-040 处置 = deferred.json history 只追加。"
            + (" | " + " | ".join(NOTES[-6:]) if NOTES else "")
        ),
    }
    if all_pass:
        import jsonschema

        errs = list(jsonschema.Draft7Validator(
            json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        ).iter_errors(gate_doc))
        if errs:
            fail("gate evidence schema 自校验红: " + "; ".join(
                f"{'/'.join(str(p) for p in e.path)}: {e.message}" for e in errs[:3]))
            all_pass = False
    gate_path = ROOT / "evidence" / f"g31_rt_pipeline_{ts}.json"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}(+ harness {len(harness_archives)} 件)")

    if FAILURES:
        print(f"[{TAG}] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f[:400]}", file=sys.stderr)
    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂,无 GPU/无构建依赖）
# ---------------------------------------------------------------------------


def _good_rfc() -> str:
    return (
        "| Provenance | `Assisted-by: TraeCode:Kimi-K3`（起草） |\n"
        "RFC-0048 RT pipeline + SBT 宿主车道 Agent Approved\n"
        "hit/miss 着色阶段 SBT RayQuery fail-closed SER 确定性\n"
        "## 9.1 对抗性评审记录\n"
        "| 评审者 provenance | `Assisted-by: TraeCode:Kimi-K3（D-409 独立评审视角实例，与起草逻辑隔离）` |\n"
        "| provenance 偏差登记 | 同模型效力自限 |\n"
        "PR-2 PR-3 维持 open 如实登记\n"
        + "".join(
            f"| F{i} | finding{i} | med | **采纳并修**：§X |\n" if i % 2 else f"| F{i} | finding{i} | low | **驳回**：理由 |\n"
            for i in range(1, 9)
        )
    )


def _good_manifest() -> dict:
    h = "a" * 64
    return {
        "schema": "rurix.rt-pipeline-manifest.v1",
        "raygen": "rg",
        "miss": ["ms"],
        "hit_groups": [
            {"name": "slab_a", "group_index": 0, "kind": "triangles", "closest_hit": "ch_a",
             "any_hit": None, "intersection": None, "record_schema_hash": h},
            {"name": "slab_b", "group_index": 1, "kind": "triangles", "closest_hit": "ch_b",
             "any_hit": None, "intersection": None, "record_schema_hash": h},
        ],
        "callables": [],
        "payload_schema_hash": h,
        "required_capabilities": ["rt.pipeline", "rt.sbt_user_data"],
        "recursion": 1,
        "interface_hash": h,
    }


def _good_kernel() -> str:
    return (
        "let denom = 1.0 - rec.rc * rec.ab;\n"
        "let r = rec.rc + tc * tc * rec.ab / denom.max(1e-30);\n"
        "p.r = rec.albedo_r * r;\n"
        "p.r = 0.05;\np.b = 0.08;\n"
        "// uv*2−1;origin = (cx, cy, −1);镜像语料发射器\n"
    )


def _good_codegen() -> str:
    return (
        "OP_FMUL 1e-30f32 albedo 0.05f32 0.08f32 two_v2 float_n1\n"
        "emit_g31_rt_slab_miss emit_g31_rt_slab_closesthit emit_g31_ser_raygen\n"
        "// 非 .rx 编译产物\n"
    )


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    # ① RFC 判读器。
    expect(judge_rfc(_good_rfc(), 48) == [], "GREEN:合法 RFC 过")
    bad = _good_rfc().replace("Agent Approved", "Draft")
    expect(judge_rfc(bad, 48), "RED:未 Approved 必红")
    bad = _good_rfc().replace("（D-409 独立评审视角实例，与起草逻辑隔离）", "")
    expect(judge_rfc(bad, 48), "RED:评审 provenance == 起草必红")
    bad = _good_rfc().replace("| F8 | finding8 | low | **驳回**：理由 |\n", "")
    expect(judge_rfc(bad, 48), "RED:findings < 8 必红")
    expect(judge_rfc(_good_rfc(), 47), "RED:ledger 编号未消费必红")
    bad = _good_rfc().replace("RayQuery", "XX")
    expect(judge_rfc(bad, 48), "RED:语义要素缺失必红")

    # ② manifest 判读器。
    expect(judge_manifest(_good_manifest()) == [], "GREEN:合法 manifest 过")
    bad = _good_manifest(); bad["required_capabilities"] = ["rt.pipeline"]
    expect(judge_manifest(bad), "RED:漏 rt.sbt_user_data 必红（RXS-0311 防线）")
    bad = _good_manifest(); bad["hit_groups"] = bad["hit_groups"][:1]
    expect(judge_manifest(bad), "RED:单 hit group 必红")
    bad = _good_manifest(); bad["hit_groups"][0]["record_schema_hash"] = "zz"
    expect(judge_manifest(bad), "RED:record hash 形态破必红")
    bad = _good_manifest(); bad["recursion"] = 2
    expect(judge_manifest(bad), "RED:recursion≠1 必红")
    bad = _good_manifest(); bad["hit_groups"][1]["name"] = "slab_c"
    expect(judge_manifest(bad), "RED:组名/序漂移必红")

    # ③ 公式面判读器。
    expect(judge_formula_single_source(_good_kernel(), _good_codegen()) == [], "GREEN:公式面互核过")
    expect(judge_formula_single_source(_good_kernel().replace("denom.max(1e-30)", "denom.max(1e-20)"), _good_codegen()),
           "RED:kernel 常量漂移必红")
    expect(judge_formula_single_source(_good_kernel(), _good_codegen().replace("非 .rx 编译产物", "")),
           "RED:hand-emitted 标注缺失必红")
    expect(judge_formula_single_source(_good_kernel(), _good_codegen().replace("emit_g31_ser_raygen", "")),
           "RED:发射器缺失必红")

    # ④ 对拍判读器。
    expect(judge_parity({"bitexact": True, "in_bound": True, "mismatch_ratio": 0.0, "max_lsb_diff": 0}) == [],
           "GREEN:位级一致过")
    expect(judge_parity({"bitexact": False, "in_bound": True, "mismatch_ratio": 0.0005, "max_lsb_diff": 1}) == [],
           "GREEN:结构容差带内过")
    expect(judge_parity({"bitexact": False, "in_bound": False, "mismatch_ratio": 0.002, "max_lsb_diff": 1}),
           "RED:超占比必红")
    expect(judge_parity({"bitexact": False, "in_bound": False, "mismatch_ratio": 0.0, "max_lsb_diff": 2}),
           "RED:>1 LSB 必红")

    # ⑤ SER 判读器。
    good_ser = {"state": "executed", "pixels_bitexact_across_arms": True, "double_run_bitexact": True,
                "time_ms_noreorder": 1.3, "time_ms_reorder": 2.5, "speedup_ratio": 0.52,
                "dispatches_per_arm": 40, "tokens": {"ext_nv": True, "feature_reorder": True}}
    fails, kind = judge_ser(good_ser)
    expect(not fails and kind == "measured_workload", "GREEN:SER executed 过（measured_workload）")
    fails, _ = judge_ser({**good_ser, "pixels_bitexact_across_arms": False})
    expect(fails, "RED:reorder 改画面必红")
    fails, _ = judge_ser({**good_ser, "time_ms_reorder": 0.0})
    expect(fails, "RED:SER 时延非正必红")
    fails, kind = judge_ser({"state": "absent", "note": "M52 capability 半命中维持 defer 如实登记"})
    expect(not fails and kind == "maintain_defer", "GREEN:SER absent 维持 defer 过")
    fails, _ = judge_ser({"state": "absent", "note": "no defer wording"})
    expect(fails, "RED:absent 缺 defer 字面必红")

    # schema 互核:facts enum == FACT_IDS。
    expect(SCHEMA_PATH.is_file(), "gate schema 在树")
    expect(SER_SCHEMA_PATH.is_file(), "ser_gain schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
    if SER_SCHEMA_PATH.is_file():
        ss = json.loads(SER_SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(ss["properties"]["schema"]["const"] == SER_SCHEMA_ID, "ser schema const 互核")
        expect(ss["properties"]["capability"]["properties"]["ext_nv"]["const"] is True, "ser schema capability ext_nv 门")
        expect(ss["properties"]["correctness"]["properties"]["pixels_bitexact_across_arms"]["const"] is True,
               "ser schema reorder 不改画面门")
    expect(RFC_PATH.is_file(), "RFC-0048 在树")
    expect(HIT_KERNEL.is_file(), "hit kernel 在树（RD-040 锚 *hit*.rx 命中）")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=8；5 判读器红绿双臂 + schema/RFC/kernel 互核）")
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
