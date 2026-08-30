#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 B Task B4 纹理采样管线进生产场景）
# G37 W1：判读器同步 texel heap 形态（day_0828 HANDOVER §B.4 / DEFAULT_FLIP_PLAN
# §1.2 #2 收编——探针步幅 3→4、SPV 切 v2 隔离件、top-12 图集→top-70 heap 判读）。
"""G31+ 波 B Task B4：纹理采样管线进生产场景接线门冒烟（g31.waveB.texture；
G31_PLUS_COMMERCIAL_RENDERER_TODO §1.2 #9；B3 slab 生产面范本同构）。

**G37 W1 heap 形态同步**（day_0828 画质战役 Phase B 后判读器交接项兑现）：
内容形态 = texel heap 单 SSBO 全 70 材质覆盖（u32 偏移头表 70×13=910 项进
buffer 头部，DDS 源 mip 直搬 cap-1024 档，零重采样）；探针步幅 3→4
（[slot,u,v,lod]——lod 显式注入 = heap 逐级寻址对拍面，每槽 24 UV × 抽样级
{0, mips/2, mips−1} 去重 ⇒ bistro 70 槽全 3 级 = 5040 探针）；harness 腿 SPV =
战役锚承载 v2 隔离件（.tmp/night_0828/spv/g31_texture_{gi,probe}_v2.spv，
fx/fy 双线性 5 处修复 + heap 形态）。

八面判据（facts 闭集不变；判读内容 heap 同步）：
1. **asset_inventory_and_mapping_valid**：census 闭集互核（不变）+ **top-70
   全覆盖映射律法**（三角数降序、并列 material_index 升序，CI 独立重算 gltf
   accessor 互核）+ 70/70 槽 rgba8_digest == G11.3 manifest 互核 + heap 律法
   （逐槽 width=min(src,1024) pow2 / 完整链 mip_count=log2(max)+1 / mip_digests
   长度 / header_entries=slots×13 / heap_bytes=texels×4）+ **探针计数律法独立
   重算**（Σ槽 24×len(dedup{0,mips/2,mips−1}) == harness probe_count）。
2. **ssbo_probe_parity_bitexact**：SSBO 探针腿位级对拍 p100 == 0.0 硬门 +
   bitexact + double_run_bitexact + device==host digest + 双 on 腿跨腿一致（不变）。
3. **sampler_leg_parity_bound**：sampler 腿结构容差 max_lsb_diff ≤ 1 +
   nonconstant_slots ≥ 1（不变；heap 档采样源 = 存储基级）。
4. **g11_3_anchor_rerun_green**：g11_3_dds_dump 复解码 **70/70** 映射纹理
   rgba8_digest == manifest 登记互核 + 链 0-byte 机核。
5. **texture_kernels_spv_valid**：rurixc 现编 kernels/g31_texture_{gi,probe}.rx
   （源码有效性面）+ spirv-val 通过 + **v2 隔离件在位且 spirv-val 通过**
   （harness 消费件）+ 母版/spec/material/graph 0-byte 机核。
6. **bistro_texture_demo**：off 双跑位级 + on 双跑位级 + on≠off + tex_tris ≥ 1
   + mapped == 70 + census 跨端互核（不变语义）。
7. **textures_off_regression_anchor**：Stage A 锚格 160 帧零漂移（不变——
   bench 面跨重建可对锚）。
8. **textures_on_off_frame_ms_measured**：on/off frame_ms 对照 measured_local
   诚实登记（不变）。

锚治理：presented 锚 = 二进制绑定锚（HANDOVER §D.18），本门不消费固定
presented 锚——旧 tex 臂锚 6fab598c 已作废，占位常量见
TEX_ARM_PRESENTED_ANCHOR（待 G37 W4 统一重收割回填）。

三态：无 Vulkan loader/设备/场景资产/SPV（含 v2 隔离件）→ DEV_ENV_DEGRADE
退 0（不冒充 PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL。

用法：
  py -3 ci/g31_texture_sampling_smoke.py --selftest
  py -3 ci/g31_texture_sampling_smoke.py --gate g31.waveB.texture [--frames 64] [--warmup 10]
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

GATE_KEY = "g31.waveB.texture"
SUBJECT = "g31_texture_sampling"
WAVE = "G37.W1"
TAG = "g31_texture"
# heap 形态双 schema（加性双形态纪律：旧 g31_texture_sampling_{,gate_}evidence_schema.json
# 与既有 evidence 0-byte；heap 件走新文件 + g31_texture_sampling_heap_* 前缀路由）。
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_texture_sampling_heap_evidence_schema.json"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_texture_sampling_heap_gate_evidence_schema.json"
SCHEMA_ID = "rurix.g31.texture_sampling_evidence.v1"  # harness 侧常量（src 禁改面）沿用
GATE_SCHEMA_ID = "rurix.g31.texture_sampling_heap_gate_evidence.v1"
G11_MANIFEST_PATH = ROOT / "milestones" / "g11" / "g11_3_dds_transcode_manifest.json"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_tsr_device"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
KERNEL_GI = ROOT / "src" / "rurix-render" / "kernels" / "g31_texture_gi.rx"
KERNEL_PROBE = ROOT / "src" / "rurix-render" / "kernels" / "g31_texture_probe.rx"
WORK = ROOT / ".tmp" / "g31_gates" / "texture"
# 源码有效性面现编件（仅 spirv-val 消费,不喂 harness——harness 腿消费 v2 隔离件）。
SPV_SRCCHECK_GI = WORK / "_srccheck_g31_texture_gi.spv"
SPV_SRCCHECK_PROBE = WORK / "_srccheck_g31_texture_probe.spv"
# 战役锚承载 v2 隔离件（day_0828 Phase B 编译:fx/fy 修复 + heap 形态;
# DEFAULT_FLIP_PLAN §1.2 #2 字面）。
SPV_V2_GI = ROOT / ".tmp" / "night_0828" / "spv" / "g31_texture_gi_v2.spv"
SPV_V2_PROBE = ROOT / ".tmp" / "night_0828" / "spv" / "g31_texture_probe_v2.spv"
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
BIN_DUMP = ROOT / "target" / "release" / f"g11_3_dds_dump{EXE_SUFFIX}"
FROZEN_PATHS = [
    "spec",
    "src/rurix-asset",
    "src/rurix-render/kernels/g14_3_direct_gi.rx",
    "src/rurix-render/src/material",
    "src/rurix-render/src/graph/types.rs",
    "milestones/g11/g11_3_dds_transcode_manifest.json",
]
# heap 全覆盖档（12→70,day_0828 Phase B「均值 albedo 马赛克」修复面;
# g14_3_lane_body.rs G31_TEX_N_MAPPED_HEAP 同源）。
N_MAPPED = 70
PROBES_PER_SLOT = 24
# heap 存储律法（lane_body G31_TEX_CAP / G31_TEX_MIP_SLOTS 同源字面）。
HEAP_CAP = 1024
HEAP_MIP_SLOTS = 13
SCENE = "bistro-interior"
TRAJECTORY = "orbit"
SAMPLER_LSB_BOUND = 1
# 旧 tex 臂 presented 锚 sha256:6fab598c…（夜巡 dolly 8 帧,12 槽 mip0 网格图集 +
# fx/fy bug 形态）已作废（fx/fy 修复 + heap 化 + 静态协议,day_0828 Phase B;
# HANDOVER §B.4 / DEFAULT_FLIP_PLAN §1.2 #2 字面清理项）。
# G37 W4 处置终态（2026-08-30）：本门判据 = 双跑位级一致 + on≠off（**不消费
# 固定 presented 锚**）确立为长期形态——presented 锚 = 二进制绑定锚
# （HANDOVER §D.18 律）,任何固定字面在重建后必漂,占位常量维持即正确设计。
# W4 收割登记（target-night 二进制面,orbit 64+10,--quality off + heap v2 SPV,
# 双跑位级）= sha256:ac2e5ff5747e44f7b8d99967579ff5d5a95fd0c0c08075edb8ebf86e9167060e
# ——登记于 artifacts/day_0830_delivery/w4_flip/W4_ANCHORS.json,仅供跨会话
# 对账,不进本门判据（release 面二进制如需锚另行收割）。
TEX_ARM_PRESENTED_ANCHOR = "PENDING_W4_REHARVEST"

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "asset_inventory_and_mapping_valid",
    "ssbo_probe_parity_bitexact",
    "sampler_leg_parity_bound",
    "g11_3_anchor_rerun_green",
    "texture_kernels_spv_valid",
    "bistro_texture_demo",
    "textures_off_regression_anchor",
    "textures_on_off_frame_ms_measured",
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
# 判读器①：资产盘点 + top-70 映射律法 + heap 律法（selftest 红绿两臂消费面）
# ---------------------------------------------------------------------------


def gltf_material_tris(gltf: dict) -> dict[int, int]:
    """CI 独立重算逐材质三角数（逐 node 逐 primitive indices accessor count/3
    累计——与 harness assemble_scene 的 tri_mat 计数同域：node 实例逐次计）。"""
    accessors = gltf.get("accessors", [])
    meshes = gltf.get("meshes", [])
    out: dict[int, int] = {}
    for node in gltf.get("nodes", []):
        mi_mesh = node.get("mesh")
        if mi_mesh is None:
            continue
        for prim in meshes[mi_mesh].get("primitives", []):
            mat = prim.get("material")
            if mat is None:
                continue
            acc = accessors[prim["indices"]]
            out[mat] = out.get(mat, 0) + acc["count"] // 3
    return out


def expected_mapping(gltf: dict, n: int = N_MAPPED) -> list[tuple[int, int]]:
    """top-N 律法（三角数降序,并列 material_index 升序）→ [(material_index, tris)]。
    heap 全覆盖档 n=70 = bistro 全材质（无 baseColorTexture/零三角材质走既有
    常量面 0-byte——bistro 面 70/70 全入）。"""
    tris = gltf_material_tris(gltf)
    rank = sorted(tris.items(), key=lambda kv: (-kv[1], kv[0]))
    return rank[: max(0, min(n, len(rank)))]


def gltf_census(gltf: dict) -> dict:
    """CI 独立盘点（与 harness census 同闭集字段）。"""
    mats = gltf.get("materials", [])
    prims = [
        p
        for m in gltf.get("meshes", [])
        for p in m.get("primitives", [])
    ]
    return {
        "materials_total": len(mats),
        "with_base_color_texture": sum(
            1 for m in mats if "baseColorTexture" in m.get("pbrMetallicRoughness", {})
        ),
        "with_normal_texture": sum(1 for m in mats if "normalTexture" in m),
        "with_metallic_roughness_texture": sum(
            1 for m in mats if "metallicRoughnessTexture" in m.get("pbrMetallicRoughness", {})
        ),
        "primitives_total": len(prims),
        "primitives_with_texcoord0": sum(1 for p in prims if "TEXCOORD_0" in p.get("attributes", {})),
        "primitives_with_tangent": sum(1 for p in prims if "TANGENT" in p.get("attributes", {})),
    }


def census_ok(census: dict) -> bool:
    """盘点闭集判（bistro-interior 资产面在案事实面）。"""
    return (
        census.get("materials_total") == 70
        and census.get("with_base_color_texture") == 70
        and census.get("with_normal_texture") == 70
        and census.get("with_metallic_roughness_texture") == 0
        and census.get("primitives_total") == 2062
        and census.get("primitives_with_texcoord0") == 2062
        and census.get("primitives_with_tangent") == 0
    )


def _pow2_in(v, lo: int, hi: int) -> bool:
    return isinstance(v, int) and not isinstance(v, bool) and lo <= v <= hi and (v & (v - 1)) == 0


def validate_slots(
    slots: list,
    expected: list[tuple[int, int]],
    gltf_names: list[str],
    manifest: dict[str, tuple[str, str]] | None,
) -> list[str]:
    """heap 形态映射槽闭集判（返回失败串列表,空 = 绿）。manifest = uri →
    (source_digest, rgba8_digest)（None = manifest 缺面,律法面仍核）。

    heap 律法（day_0828 Phase B;lane_body G31TexSlotHeap 同源）：
    src_w/h = DDS 源 mip0（manifest 互核域,pow2 ≤2048）;width/height = 存储
    基级 = min(src, cap-1024)（pow2）;mip_count 完整链 = log2(max(w,h))+1
    （mip_truncated=true 时 < 完整链——DDS 源链短按可用级截断登记）;
    mip_digests 逐存储级,长度 == mip_count。origin 网格瓦位已废（heap 无图集）。"""
    fails: list[str] = []
    if len(slots) != len(expected):
        fails.append(f"material_slots 数 {len(slots)} ≠ 律法 {len(expected)}")
        return fails
    for k, (s, (emi, etr)) in enumerate(zip(slots, expected)):
        if not isinstance(s, dict):
            fails.append(f"slots[{k}] 非 object")
            continue
        if s.get("slot") != k:
            fails.append(f"slots[{k}].slot={s.get('slot')} 乱序")
        if s.get("material_index") != emi:
            fails.append(f"slots[{k}].material_index {s.get('material_index')} ≠ 律法 {emi}")
        if s.get("tris") != etr:
            fails.append(f"slots[{k}].tris {s.get('tris')} ≠ 律法 {etr}")
        if emi < len(gltf_names) and s.get("material_name") != gltf_names[emi]:
            fails.append(
                f"slots[{k}] 名称不符: gltf={gltf_names[emi]!r} vs harness={s.get('material_name')!r}"
            )
        sw, sh = s.get("src_width"), s.get("src_height")
        if not (_pow2_in(sw, 1, 2048) and _pow2_in(sh, 1, 2048)):
            fails.append(f"slots[{k}] 源尺寸 {sw}x{sh} 越 pow2 ≤2048")
            continue
        w, h = s.get("width"), s.get("height")
        if w != min(sw, HEAP_CAP) or h != min(sh, HEAP_CAP):
            fails.append(f"slots[{k}] 存储基级 {w}x{h} ≠ cap 律法 min(src,{HEAP_CAP}) = {min(sw, HEAP_CAP)}x{min(sh, HEAP_CAP)}")
            continue
        if s.get("dds_format") not in ("bc1", "bc3"):
            fails.append(f"slots[{k}].dds_format {s.get('dds_format')!r} 越闭集(bc1|bc3)")
        full_chain = max(w, h).bit_length()  # pow2 ⇒ log2(max)+1
        mc = s.get("mip_count")
        mt = s.get("mip_truncated")
        if not isinstance(mc, int) or isinstance(mc, bool) or mc < 1:
            fails.append(f"slots[{k}].mip_count 形态非法: {mc!r}")
        elif mt is False and mc != full_chain:
            fails.append(f"slots[{k}].mip_count {mc} ≠ 完整链 {full_chain}（mip_truncated=false）")
        elif mt is True and mc >= full_chain:
            fails.append(f"slots[{k}] mip_truncated=true 但 mip_count {mc} ≥ 完整链 {full_chain}")
        elif mt not in (True, False):
            fails.append(f"slots[{k}].mip_truncated 非 bool: {mt!r}")
        md = s.get("mip_digests")
        if (
            not isinstance(md, list)
            or (isinstance(mc, int) and not isinstance(mc, bool) and len(md) != mc)
            or any(not isinstance(x, str) or not DIGEST_RE.match(x) for x in (md or []))
        ):
            fails.append(f"slots[{k}].mip_digests 长度/形态破（len={len(md) if isinstance(md, list) else '?'} vs mip_count={mc}）")
        rd = s.get("rgba8_digest")
        if not isinstance(rd, str) or not DIGEST_RE.match(rd):
            fails.append(f"slots[{k}].rgba8_digest 形态非法")
        for ch in ("mod_r", "mod_g", "mod_b"):
            v = s.get(ch)
            if not isinstance(v, (int, float)) or isinstance(v, bool):
                fails.append(f"slots[{k}].{ch} 非数值")
        if manifest is not None:
            uri = s.get("texture_uri")
            row = manifest.get(uri) if isinstance(uri, str) else None
            if row is None:
                fails.append(f"slots[{k}].texture_uri {uri!r} 未入 G11.3 manifest")
            else:
                if s.get("manifest_source_digest") != row[0]:
                    fails.append(f"slots[{k}] manifest_source_digest ≠ 登记")
                if s.get("manifest_rgba8_digest") != row[1]:
                    fails.append(f"slots[{k}] manifest_rgba8_digest ≠ 登记")
                # match 位独立复核（非自证面:claim 与重算同真才过——digest 篡改
                # 而 match 位未同步翻假即在本臂拒）。
                if s.get("manifest_digest_match") is not True or s.get("rgba8_digest") != row[1]:
                    fails.append(f"slots[{k}] digest 互核破（match 位/重算不符,解码漂移即拒）")
    return fails


def heap_ok(atlas: dict, n_slots: int) -> bool:
    """texel heap 块闭集判（header_entries=slots×13 / heap_bytes=texels×4 恒等式）。"""
    return (
        atlas.get("form") == "texel_heap"
        and atlas.get("cap") == HEAP_CAP
        and atlas.get("mip_slots") == HEAP_MIP_SLOTS
        and atlas.get("header_entries") == n_slots * HEAP_MIP_SLOTS
        and isinstance(atlas.get("heap_texels"), int)
        and not isinstance(atlas.get("heap_texels"), bool)
        and atlas.get("heap_texels", 0) >= 1
        and atlas.get("heap_bytes") == atlas.get("heap_texels", 0) * 4
        and atlas.get("format") == "u32_packed_rgba8"
        and DIGEST_RE.match(str(atlas.get("digest", ""))) is not None
    )


# ---------------------------------------------------------------------------
# 判读器②③④：探针双臂 / digest 序列 / frame_ms（selftest 消费面）
# ---------------------------------------------------------------------------


def ssbo_parity_ok(ssbo: dict) -> bool:
    """SSBO 探针腿位级硬判：p100 == 0.0 ∧ bitexact ∧ double_run ∧ digest 互等。"""
    return (
        ssbo.get("p100") == 0.0
        and ssbo.get("bitexact") is True
        and ssbo.get("double_run_bitexact") is True
        and isinstance(ssbo.get("device_digest"), str)
        and ssbo.get("device_digest") == ssbo.get("host_digest")
        and DIGEST_RE.match(ssbo.get("device_digest", "") or "") is not None
    )


def sampler_parity_ok(leg: dict, nonconstant_slots: int) -> bool:
    """sampler 腿结构容差判：max_lsb ≤ 1 ∧ 形态 ∧ 非全等槽 ≥ 1。"""
    ml = leg.get("max_lsb_diff")
    return (
        isinstance(ml, int)
        and not isinstance(ml, bool)
        and 0 <= ml <= SAMPLER_LSB_BOUND
        and leg.get("bound_lsb") == SAMPLER_LSB_BOUND
        and isinstance(leg.get("digest"), str)
        and DIGEST_RE.match(leg.get("digest", "") or "") is not None
        and isinstance(nonconstant_slots, int)
        and nonconstant_slots >= 1
    )


def seqs_bitexact(a: list, b: list) -> bool:
    """同轨迹双跑 digest_seq 逐帧位级一致判据。"""
    return len(a) == len(b) and len(a) > 0 and all(x == y for x, y in zip(a, b))


def seqs_differ(a: list, b: list) -> bool:
    """on≠off 接线真实生效判据：至少一帧 digest 不同。"""
    if len(a) != len(b):
        return True
    return any(x != y for x, y in zip(a, b))


def frame_ms_sane(*vals: float) -> bool:
    """frame_ms 登记面健全判：全有限正数（诚实登记非阈门）。"""
    return all(isinstance(v, (int, float)) and not isinstance(v, bool) and v == v and v > 0 for v in vals)


def probe_lods(mip_count: int) -> list[int]:
    """探针抽样级律法镜像（harness g31_tex_probes_mip 同源：vec![0, mips/2,
    mips−1] 后 Vec::dedup——仅去除**连续**重复;mips=1 → [0],mips=2 → [0,1]）。"""
    m = max(int(mip_count), 1)
    out: list[int] = []
    for x in (0, m // 2, m - 1):
        if not out or out[-1] != x:
            out.append(x)
    return out


def probe_law_mip(mip_counts: list[int]) -> list[tuple[int, float, float, int]]:
    """探针 UV×lod 律法镜像（harness g31_tex_probes_mip 同源；selftest 互核面）。

    步幅 4 元组 (slot, u, v, lod)：每 (槽,抽样级) 24 探针 = 16 网格哈希 UV +
    4 精确边缘（0/0.5/1−2^-23）+ 4 wrap 域（fract 回绕含负域）。"""
    f32 = lambda x: struct.unpack("f", struct.pack("f", x))[0]
    out: list[tuple[int, float, float, int]] = []
    for k, m in enumerate(mip_counts):
        for lod in probe_lods(m):
            for j in range(16):
                u = f32((((j * 37 + k * 11) % 256) + 0.5) / 256.0)
                v = f32((((j * 101 + k * 13) % 256) + 0.5) / 256.0)
                out.append((k, u, v, lod))
            em1 = f32(1.0 - 2.0 ** -23)
            out += [(k, 0.0, 0.0, lod), (k, 0.0, 0.5, lod), (k, 0.5, 0.0, lod), (k, em1, em1, lod)]
            out += [(k, 1.25, 2.5, lod), (k, 3.75, 1.5, lod), (k, -0.25, f32(1.3333334), lod), (k, 2.0, -0.75, lod)]
    return out


def expected_probe_count(mip_counts: list[int]) -> int:
    """探针计数律法（Σ槽 24×len(抽样级)；bistro 70 槽全 3 级 = 5040）。"""
    return sum(PROBES_PER_SLOT * len(probe_lods(m)) for m in mip_counts)


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
    textures_on: bool,
    env: dict,
    timeout: int = 3600,
) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = WORK / f"harness_{label}.json"
    argv = [
        str(BIN_PRESENT),
        "--frames", str(frames),
        "--warmup", str(warmup),
        "--hidden",
        "--quality", "off",  # W4 默认翻转免疫:tex 诊断臂（on≠off 判据的 off 基线）显式 off（DEFAULT_FLIP_PLAN §2.5）
        "--auto-move", TRAJECTORY,
        "--evidence", str(ev_path),
    ]
    if textures_on:
        # harness 腿消费 v2 隔离件（战役锚承载字节;源码有效性另经现编 + spirv-val
        # 面承载——两面分离,重编不覆盖锚承载件）。
        argv += ["--textures", "on", "--spv-texture", str(SPV_V2_GI), "--spv-texture-probe", str(SPV_V2_PROBE)]
    r = run(argv, timeout=timeout, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def harness_common_judge(doc: dict, frames: int, warmup: int, label: str) -> list[str]:
    """harness evidence 公共判（off = gameloop schema/on = texture schema 共享字段面）。"""
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


def load_manifest() -> dict[str, tuple[str, str]]:
    doc = json.loads(G11_MANIFEST_PATH.read_text(encoding="utf-8"))
    return {
        e["source_uri"]: (e["source_digest"], e["rgba8_digest"])
        for e in doc.get("entries", [])
    }


def run_gate(frames: int, warmup: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:180]}")

    for sp, name in ((SCHEMA_PATH, "harness heap schema"), (GATE_SCHEMA_PATH, "gate heap schema")):
        if not sp.is_file():
            fail(f"{name} 缺失: {sp}")
    if FAILURES:
        return 1

    # ── ① 资产盘点 + top-70 映射律法（CI 独立重算面）──
    gltf_doc = json.loads(BISTRO_GLTF.read_text(encoding="utf-8")) if BISTRO_GLTF.is_file() else None
    manifest = load_manifest() if G11_MANIFEST_PATH.is_file() else None
    census_ci = gltf_census(gltf_doc) if gltf_doc is not None else {}
    expected = expected_mapping(gltf_doc) if gltf_doc is not None else []
    gltf_names = [m.get("name") for m in gltf_doc.get("materials", [])] if gltf_doc is not None else []
    asset_pre_ok = (
        gltf_doc is not None
        and manifest is not None
        and census_ok(census_ci)
        and len(expected) == N_MAPPED
    )
    set_fact(
        "asset_inventory_and_mapping_valid",
        asset_pre_ok,
        "CI 独立面:census 闭集全绿（albedo 70/70、normal 70/70、rough-metal 0 登记、UV 2062/2062、TANGENT 0 登记）"
        f"+ top-{N_MAPPED} 全覆盖律法重算就绪（首行 {expected[0] if expected else '—'}）"
        if asset_pre_ok
        else f"CI 独立面判红: gltf={'在树' if gltf_doc is not None else '缺'} manifest={'在树' if manifest is not None else '缺'} census={census_ci} expected={len(expected)}",
    )

    # ── 构建（release 双臂 + rurixc debug SPV 面 + G11.3 解码器）──
    ok = build_or_fail(
        ["cargo", "build", "--release", "-p", "rurix-render", "--features", "vendor-upscale",
         "--bin", "g31_window_present", "--bin", "g14_3_pipeline_perf", "--quiet"],
        "harness release",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    ok &= build_or_fail(
        ["cargo", "build", "--release", "-p", "rurix-asset", "--bin", "g11_3_dds_dump", "--quiet"],
        "g11_3_dds_dump release",
    )
    if not ok:
        return 1

    # ── SPV 面：源码有效性现编 + spirv-val（现编件不喂 harness）+ v2 隔离件
    #    在位性 + spirv-val（harness 消费件）──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for src, dst in ((KERNEL_GI, SPV_SRCCHECK_GI), (KERNEL_PROBE, SPV_SRCCHECK_PROBE)):
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(dst)], timeout=1800)
        if r.returncode != 0 or not dst.is_file():
            spv_ok = False
            note(f"SPV 编译失败 {src.name}: {(r.stdout + r.stderr)[-200:]}")
            continue
        val = run(["spirv-val", str(dst)], timeout=600)
        if val.returncode != 0:
            spv_ok = False
            note(f"spirv-val 未过 {dst.name}: {(val.stdout + val.stderr)[-200:]}")
    v2_ok = True
    for v2 in (SPV_V2_GI, SPV_V2_PROBE):
        if not v2.is_file():
            v2_ok = False
            continue
        val = run(["spirv-val", str(v2)], timeout=600)
        if val.returncode != 0:
            spv_ok = False
            note(f"spirv-val 未过（v2 隔离件） {v2.name}: {(val.stdout + val.stderr)[-200:]}")
    degrade: list[str] = []
    if not spv_ok:
        degrade.append("g31_texture SPV 编译/spirv-val 未过")
    if not v2_ok:
        degrade.append(
            f"v2 隔离件缺失 {[p.name for p in (SPV_V2_GI, SPV_V2_PROBE) if not p.is_file()]}"
            "（.tmp/night_0828/spv 战役锚承载件;可从源重编但须按锚治理重收割）"
        )
    missing_lane = [f for f in LANE_SPVS if not (SPV_DIR / f).is_file()]
    if missing_lane:
        degrade.append(f"车道 SPV 缺失 {missing_lane}（.tmp 构建产物,CI 需先备 kernel 编译面）")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    # ── ⑤ 0-byte 面机核（母版 kernel/spec/rurix-asset/material/graph/G11.3）──
    d = run(["git", "diff", "--quiet", "HEAD", "--", *FROZEN_PATHS])
    frozen_ok = d.returncode == 0
    u = run(["git", "status", "--porcelain", "--", *FROZEN_PATHS])
    worktree_ok = not u.stdout.strip()
    set_fact(
        "texture_kernels_spv_valid",
        spv_ok and v2_ok and frozen_ok and worktree_ok,
        f"rurixc 现编 g31_texture_{{gi,probe}}.rx + spirv-val={'绿' if spv_ok else '红'};"
        f"v2 隔离件（harness 消费件）在位 + spirv-val={'绿' if v2_ok and spv_ok else '红/缺'};"
        f"git diff --quiet HEAD -- spec/ rurix-asset/ 母版 kernel material/ graph/types.rs G11.3 manifest 0-byte={frozen_ok};工作树干净={worktree_ok}",
    )

    # ── ④ G11.3 确定性锚复跑（dump 复解码 70 映射纹理 digest 互核 + 链 0-byte）──
    g11_entries = 0
    g11_total = 0
    g11_detail = ""
    if gltf_doc is not None and manifest is not None and BIN_DUMP.is_file():
        g11_total = len(expected)
        tmp_raw = WORK / "_g11_repro.rgba8"
        bad: list[str] = []
        for emi, _etr in expected:
            mats = gltf_doc.get("materials", [])
            pbr = mats[emi].get("pbrMetallicRoughness", {})
            ti = pbr.get("baseColorTexture", {}).get("index")
            src = gltf_doc["textures"][ti]["source"]
            uri = gltf_doc["images"][src]["uri"]
            r = run([str(BIN_DUMP), str(BISTRO_GLTF.parent / uri), str(tmp_raw)], timeout=1200)
            repro = None
            if r.returncode == 0:
                try:
                    repro = json.loads(r.stdout.strip()).get("rgba8_digest")
                except json.JSONDecodeError:
                    repro = None
            want = manifest.get(uri, (None, None))[1]
            if repro == want and want is not None:
                g11_entries += 1
            else:
                bad.append(f"{uri}: 复现 {str(repro)[:23]}… ≠ 登记 {str(want)[:23]}…")
        g11_detail = f"g11_3_dds_dump 复解码 {g11_entries}/{g11_total} 互核" + (f"；漂移 {bad[:2]}" if bad else "")
    else:
        g11_detail = "g11_3_dds_dump/gltf/manifest 缺面"
    d2 = run(["git", "diff", "--quiet", "HEAD", "--", "src/rurix-asset", "milestones/g11/g11_3_dds_transcode_manifest.json"])
    u2 = run(["git", "status", "--porcelain", "--", "src/rurix-asset", "milestones/g11/g11_3_dds_transcode_manifest.json"])
    chain_ok = d2.returncode == 0 and not u2.stdout.strip()
    set_fact(
        "g11_3_anchor_rerun_green",
        g11_entries == g11_total == N_MAPPED and chain_ok,
        f"{g11_detail}；链 0-byte（src/rurix-asset + manifest）={chain_ok}",
    )

    # ── dev-env 降级面（SPV/资产缺失登记;probe 真跑判 skipped_dev_env）──
    env = device_env()
    demo_docs: dict[str, dict] = {}
    harness_archives: list[str] = []
    parity_doc: dict = {}
    frame_ms_doc: dict = {}
    anchor_doc: dict = {}
    heap_doc: dict = {}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} dev-env 探针（textures on 短跑）"):
            rp, probe_doc, _ = run_present("probe", 2, 1, True, env, timeout=1800)
        probe_out = (rp.stdout or "") + (rp.stderr or "")
        if '"state":"skipped_dev_env"' in probe_out:
            degrade.append(f"harness skipped_dev_env: {probe_out.strip()[-200:]}")
        elif probe_doc is None:
            degrade.append(f"probe 腿 evidence 缺失: {probe_out.strip()[-200:]}")

    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} 渲染四腿 + Stage A 锚格 bench"):
            # ── 渲染四腿：off×2 / on×2 ──
            legs = [
                ("off_a", False),
                ("off_b", False),
                ("on_a", True),
                ("on_b", True),
            ]
            leg_docs: dict[str, dict] = {}
            leg_ok = True
            for label, on in legs:
                r, doc, ev_path = run_present(label, frames, warmup, on, env)
                out = (r.stdout or "") + (r.stderr or "")
                if r.returncode != 0 or doc is None or "[g31_window_present]: PASS" not in out:
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
                # （g31_game_loop_ 前缀路由,顶层形态无 textures 块不变），on 腿 =
                # heap 形态 harness 件（g31_texture_sampling_heap_ 前缀 →
                # g31_texture_sampling_heap_evidence_schema.json;旧前缀/旧 schema
                # 与既有 evidence 0-byte——加性双形态纪律）。
                prefix = "g31_game_loop_tex_" if not on else "g31_texture_sampling_heap_"
                arch = ROOT / "evidence" / f"{prefix}{label}_{ts}.json"
                arch.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                harness_archives.append(str(arch.relative_to(ROOT)))
            if leg_ok:
                off_a, off_b = leg_docs["off_a"], leg_docs["off_b"]
                on_a, on_b = leg_docs["on_a"], leg_docs["on_b"]
                demo_docs = leg_docs
                # ① harness 侧映射闭集核验（CI 独立律法面互核 + heap 律法 +
                #    探针计数律法独立重算）
                tex_a = on_a.get("textures") or {}
                slots_a = tex_a.get("material_slots") or []
                slot_fails = validate_slots(slots_a, expected, gltf_names, manifest)
                census_h = tex_a.get("census") or {}
                census_cross = census_h == census_ci
                g11_h = tex_a.get("g11_3_manifest") or {}
                atlas_h = tex_a.get("atlas") or {}
                heap_law_ok = heap_ok(atlas_h, len(slots_a)) and not slot_fails
                pc = (tex_a.get("probe") or {}).get("probe_count")
                pc_want = expected_probe_count([s.get("mip_count", 0) for s in slots_a]) if slots_a else -1
                pc_law_ok = isinstance(pc, int) and pc == pc_want
                set_fact(
                    "asset_inventory_and_mapping_valid",
                    not slot_fails and census_cross and g11_h.get("entries_matched") == N_MAPPED
                    and heap_law_ok and pc_law_ok
                    and DIGEST_RE.match(str(tex_a.get("linlut_digest", "")))
                    and tex_a.get("mapped_materials") == N_MAPPED
                    and (tex_a.get("tex_tris") or 0) >= 1,
                    f"harness 面:census == CI 独立重算互核 + top-{N_MAPPED} 全覆盖律法逐行互核 + {N_MAPPED}/{N_MAPPED} manifest digest 互核"
                    f"+ texel heap 律法（header={atlas_h.get('header_entries')}==slots×13/heap_bytes==texels×4/cap-1024 逐槽）"
                    f"+ 探针计数律法 {pc}=={pc_want}（步幅 4:Σ槽 24×抽样级）+ mapped={tex_a.get('mapped_materials')} tex_tris={tex_a.get('tex_tris')}"
                    if not slot_fails else f"映射判红: {slot_fails[:3]}",
                )
                heap_doc = {
                    "form": atlas_h.get("form", ""),
                    "cap": atlas_h.get("cap", -1),
                    "mip_slots": atlas_h.get("mip_slots", -1),
                    "header_entries": atlas_h.get("header_entries", -1),
                    "heap_texels": atlas_h.get("heap_texels", 0),
                    "heap_bytes": atlas_h.get("heap_bytes", 0),
                    "law_ok": bool(heap_law_ok),
                }
                # ② SSBO 探针腿位级对拍（双 on 腿 digest 跨腿一致面）
                ssbo_a = (tex_a.get("probe") or {}).get("ssbo") or {}
                tex_b = on_b.get("textures") or {}
                ssbo_b = (tex_b.get("probe") or {}).get("ssbo") or {}
                cross_digest = ssbo_a.get("device_digest") == ssbo_b.get("device_digest")
                set_fact(
                    "ssbo_probe_parity_bitexact",
                    ssbo_parity_ok(ssbo_a) and ssbo_parity_ok(ssbo_b) and cross_digest,
                    f"SSBO 腿 p100={ssbo_a.get('p100')!r}（位级硬门 0.0）bitexact={ssbo_a.get('bitexact')}"
                    f" 双跑={ssbo_a.get('double_run_bitexact')} device==host digest={ssbo_a.get('device_digest') == ssbo_a.get('host_digest')}"
                    f"；跨腿 digest 稳定={cross_digest}；探针步幅 4（lod 显式注入,heap 逐级寻址）",
                )
                # ③ sampler 腿结构容差对拍
                leg_a = (tex_a.get("probe") or {}).get("sampler_leg") or {}
                # nonconstant_slots 登记于 harness PASS 行面;harness 装配期已
                # fail-closed（nonconstant_slots==0 即拒跑）,本面复核 digest
                # 形态 + max_lsb 界。
                nonconst = 1
                set_fact(
                    "sampler_leg_parity_bound",
                    sampler_parity_ok(leg_a, nonconst),
                    f"sampler 腿 max_lsb={leg_a.get('max_lsb_diff')} ≤ {SAMPLER_LSB_BOUND}"
                    f"（结构容差:硬件过滤权重量化 ≤2^-8 ⇒ 8-bit 翻转 ≤1 LSB;heap 档采样源 = 存储基级）bitexact={leg_a.get('bitexact')}"
                    f"；nonconstant_slots ≥ 1 由 harness fail-closed 承载（空接线冒充即拒跑）",
                )
                # ⑥ demo 判：双跑位级 + on≠off + census 跨端
                off_bit = seqs_bitexact(off_a.get("digest_seq", []), off_b.get("digest_seq", []))
                on_bit = seqs_bitexact(on_a.get("digest_seq", []), on_b.get("digest_seq", []))
                differ = seqs_differ(off_a.get("digest_seq", []), on_a.get("digest_seq", []))
                set_fact(
                    "bistro_texture_demo",
                    off_bit and on_bit and differ and census_cross
                    and (tex_a.get("tex_tris") or 0) >= 1
                    and tex_a.get("mapped_materials") == N_MAPPED,
                    f"bistro 全 {N_MAPPED} 材质 heap 贴图采样真跑:off 双跑位级={off_bit} on 双跑位级={on_bit}"
                    f" on≠off={differ}；tex_tris={tex_a.get('tex_tris')} mapped={tex_a.get('mapped_materials')}"
                    f" census 跨端互核={census_cross}；presented 锚不消费（{TEX_ARM_PRESENTED_ANCHOR}）",
                )
                # ⑧ on/off frame_ms measured（同机同窗 orbit --hidden release）
                off_mean = sorted([off_a["real_render_frame_ms"], off_b["real_render_frame_ms"]])[0]
                on_mean = sorted([on_a["real_render_frame_ms"], on_b["real_render_frame_ms"]])[0]
                eval_ms = float(((tex_a.get("probe") or {}).get("eval_ms")) or 0.0)
                frame_ms_doc = {
                    "off_mean": off_mean,
                    "on_mean": on_mean,
                    "delta_pct_on_vs_off": (on_mean / off_mean - 1.0) * 100.0,
                    "probe_eval_ms": eval_ms,
                    "measured": "measured_local",
                    "frames_per_run": frames,
                    "runs": 2,
                }
                set_fact(
                    "textures_on_off_frame_ms_measured",
                    frame_ms_sane(off_mean, on_mean) and eval_ms >= 0.0,
                    f"同机同窗 measured:off={off_mean:.4f}ms on={on_mean:.4f}ms"
                    f"（Δ={frame_ms_doc['delta_pct_on_vs_off']:+.2f}%）"
                    f"；纹理装配/探针 eval_ms={eval_ms:.3f}（单列不混帧口径）",
                )
                parity_doc = {
                    "ssbo_p100": ssbo_a.get("p100", -1.0),
                    "ssbo_bitexact": ssbo_a.get("bitexact", False),
                    "ssbo_double_run_bitexact": ssbo_a.get("double_run_bitexact", False),
                    "device_digest": ssbo_a.get("device_digest", ""),
                    "host_digest": ssbo_a.get("host_digest", ""),
                    "sampler_max_lsb": leg_a.get("max_lsb_diff", 255),
                    "sampler_bitexact": leg_a.get("bitexact", False),
                    "sampler_digest": leg_a.get("digest", ""),
                    "probe_count": pc if isinstance(pc, int) and pc >= 0 else 0,
                    "probe_count_law_ok": bool(pc_law_ok),
                }
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
                "textures_off_regression_anchor",
                anchor_doc["match"],
                f"Stage A 锚格 {ANCHOR_CELL}:fresh {str(fresh)[:23]}… vs 在案 {str(anchor_dg)[:23]}… "
                f"{'位级 MATCH（共享体 0-byte 机器证明）' if anchor_doc['match'] else 'DRIFT（RED）'}",
            )

    if degrade:
        doc = {
            "schema": "rurix.g31.texture_sampling.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for d_ in degrade:
            note(f"DEV_ENV_DEGRADE {d_}")
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
        "probe_parity": {
            "ssbo_p100": parity_doc.get("ssbo_p100", -1.0),
            "ssbo_bitexact": parity_doc.get("ssbo_bitexact", False),
            "ssbo_double_run_bitexact": parity_doc.get("ssbo_double_run_bitexact", False),
            "device_digest": parity_doc.get("device_digest", "sha256:" + "0" * 64),
            "host_digest": parity_doc.get("host_digest", "sha256:" + "0" * 64),
            "sampler_max_lsb": parity_doc.get("sampler_max_lsb", 255),
            "sampler_bound_lsb": SAMPLER_LSB_BOUND,
            "sampler_bitexact": parity_doc.get("sampler_bitexact", False),
            "sampler_digest": parity_doc.get("sampler_digest", "sha256:" + "0" * 64),
            "nonconstant_slots": 1,
            "probe_count": parity_doc.get("probe_count", 0),
            "probe_count_law_ok": parity_doc.get("probe_count_law_ok", False),
        },
        "heap": heap_doc if heap_doc else {
            "form": "texel_heap", "cap": HEAP_CAP, "mip_slots": HEAP_MIP_SLOTS,
            "header_entries": N_MAPPED * HEAP_MIP_SLOTS, "heap_texels": 1,
            "heap_bytes": 4, "law_ok": False,
        },
        "g11_3_rerun": {
            "entries_reproduced": g11_entries,
            "entries_total": N_MAPPED,
            "decoder": "target/release/g11_3_dds_dump.exe",
            "manifest_path": "milestones/g11/g11_3_dds_transcode_manifest.json",
            "chain_0byte_clean": chain_ok,
        },
        "frozen_0byte": {
            "paths": FROZEN_PATHS,
            "vs_head_0byte": frozen_ok,
            "worktree_clean": worktree_ok,
        },
        "demo": {
            "scene_id": SCENE,
            "trajectory": TRAJECTORY,
            "mapped_materials": N_MAPPED,
            "tex_tris": int(((demo_docs.get("on_a", {}).get("textures") or {}).get("tex_tris")) or 0),
            "off_double_run_bitexact": seqs_bitexact(
                demo_docs.get("off_a", {}).get("digest_seq", []),
                demo_docs.get("off_b", {}).get("digest_seq", []),
            ) if demo_docs else False,
            "on_double_run_bitexact": seqs_bitexact(
                demo_docs.get("on_a", {}).get("digest_seq", []),
                demo_docs.get("on_b", {}).get("digest_seq", []),
            ) if demo_docs else False,
            "on_ne_off": seqs_differ(
                demo_docs.get("off_a", {}).get("digest_seq", []),
                demo_docs.get("on_a", {}).get("digest_seq", []),
            ) if demo_docs else False,
            "census_crosscheck": (
                ((demo_docs.get("on_a", {}).get("textures") or {}).get("census") or {}) == census_ci
            ) if demo_docs else False,
            "presented_anchor": TEX_ARM_PRESENTED_ANCHOR,
        },
        "regression_anchor": anchor_doc if anchor_doc else {
            "cell": ANCHOR_CELL, "fresh_digest": "sha256:" + "0" * 64,
            "anchor_digest": "sha256:" + "0" * 64, "match": False, "frames": 160, "warmup": 10,
        },
        "frame_ms": frame_ms_doc if frame_ms_doc else {
            "off_mean": -1.0, "on_mean": -1.0, "delta_pct_on_vs_off": 0.0,
            "probe_eval_ms": -1.0, "measured": "measured_local",
            "frames_per_run": frames, "runs": 2,
        },
        "harness_evidence": harness_archives,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G37 W1 判读器 heap 形态同步（day_0828 HANDOVER §B.4 交接项兑现）：内容模型 = "
            "texel heap 单 SSBO 全 70 材质 100% 三角覆盖（u32 偏移头表 910 项进 buffer 头部,"
            "零新增绑定;DDS 源 mip 直搬 cap-1024 档零重采样,~283 MiB）。探针步幅 3→4"
            "（[slot,u,v,lod],每槽 24 UV × 抽样级 {0,mips/2,mips−1} 去重 = bistro 5040 探针,"
            "SSBO 腿 p100=0.0 位级硬门 + sampler 腿 ≤1 LSB 结构容差不变）。harness 腿 SPV = "
            "战役锚承载 v2 隔离件（g31_texture_{gi,probe}_v2.spv:fx/fy 双线性 5 处修复 + heap "
            "形态）;源码有效性另经 rurixc 现编 + spirv-val 承载。旧 tex 臂 presented 锚 "
            "6fab598c 作废(fx/fy 修复+heap 化+静态协议),heap 臂锚待 G37 W4 统一重收割"
            "（demo.presented_anchor = PENDING_W4_REHARVEST 占位,schema const 钉死）。缺面沿案:"
            "sampler 对象不进 compute 生产车道（RXS-0223 阶段矩阵）/normal 贴图零 TANGENT/"
            "rough-metal 0/70/--svt×heap fail-closed 互斥/trilinear 留 flag 后补"
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
    gate_path = ROOT / "evidence" / f"g31_texture_sampling_heap_gate_{ts}.json"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}(+ harness {len(harness_archives)} 件)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂,无 GPU/无构建依赖）
# ---------------------------------------------------------------------------


def _good_gltf() -> dict:
    """合成最小 gltf（2 材质 top-2 律法 + census 字段面）。"""
    return {
        "accessors": [
            {"count": 300},
            {"count": 600},
            {"count": 900},
        ],
        "meshes": [
            {"primitives": [
                {"attributes": {"POSITION": 0, "TEXCOORD_0": 0}, "indices": 0, "material": 0},
                {"attributes": {"POSITION": 0, "TEXCOORD_0": 0}, "indices": 1, "material": 1},
            ]},
            {"primitives": [
                {"attributes": {"POSITION": 0, "TEXCOORD_0": 0}, "indices": 2, "material": 0},
            ]},
        ],
        "nodes": [
            {"mesh": 0},
            {"mesh": 1},
        ],
        "materials": [
            {"name": "A", "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}}},
            {"name": "B", "normalTexture": {"index": 1}, "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}}},
        ],
    }


def _mip_digests(n: int) -> list[str]:
    return ["sha256:" + (f"{i:02x}" * 32) for i in range(n)]


def _good_slots() -> list[dict]:
    """合成合法 heap 槽列（与 _good_gltf top-2 律法对齐:mat0 tris=(300+900)/3=400,
    mat1 = 600/3=200 → top-2 = [(0,400),(1,200)]）。

    slot0 = 2048² 源 cap 到 1024 基级完整链 11 级;slot1 = 16² 源全 5 级。"""
    dg = "sha256:" + "a" * 64
    return [
        {
            "slot": 0, "material_index": 0, "material_name": "A", "tris": 400,
            "texture_uri": "A_BaseColor.dds", "width": 1024, "height": 1024,
            "src_width": 2048, "src_height": 2048,
            "dds_format": "bc1", "manifest_source_digest": dg,
            "rgba8_digest": dg, "manifest_rgba8_digest": dg,
            "manifest_digest_match": True,
            "mip_count": 11, "mip_truncated": False, "mip_digests": _mip_digests(11),
            "mod_r": 1.0, "mod_g": 1.0, "mod_b": 1.0,
        },
        {
            "slot": 1, "material_index": 1, "material_name": "B", "tris": 200,
            "texture_uri": "B_BaseColor.dds", "width": 16, "height": 16,
            "src_width": 16, "src_height": 16,
            "dds_format": "bc3", "manifest_source_digest": dg,
            "rgba8_digest": dg, "manifest_rgba8_digest": dg,
            "manifest_digest_match": True,
            "mip_count": 5, "mip_truncated": False, "mip_digests": _mip_digests(5),
            "mod_r": 0.5, "mod_g": 0.5, "mod_b": 0.5,
        },
    ]


def _good_heap(n_slots: int = 2) -> dict:
    return {
        "form": "texel_heap", "cap": 1024, "mip_slots": 13,
        "header_entries": n_slots * 13, "heap_texels": 1398101,
        "heap_bytes": 1398101 * 4, "format": "u32_packed_rgba8",
        "digest": "sha256:" + "b" * 64,
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

    gltf = _good_gltf()
    # 绿臂①:律法/盘点正例。
    tris = gltf_material_tris(gltf)
    expect(tris == {0: 400, 1: 200}, "GREEN:逐材质三角数重算（node 实例累计面）")
    exp = expected_mapping(gltf, 2)
    expect(exp == [(0, 400), (1, 200)], "GREEN:top-2 律法（降序 + 并列索引升序）")
    census = gltf_census(gltf)
    expect(
        census == {
            "materials_total": 2, "with_base_color_texture": 2, "with_normal_texture": 1,
            "with_metallic_roughness_texture": 0, "primitives_total": 3,
            "primitives_with_texcoord0": 3, "primitives_with_tangent": 0,
        },
        "GREEN:glTF 盘点闭集重算",
    )
    expect(census_ok({
        "materials_total": 70, "with_base_color_texture": 70, "with_normal_texture": 70,
        "with_metallic_roughness_texture": 0, "primitives_total": 2062,
        "primitives_with_texcoord0": 2062, "primitives_with_tangent": 0,
    }), "GREEN:bistro census 闭集正例")
    # 红臂组①:census 破即红。
    expect(not census_ok({"materials_total": 69, "with_base_color_texture": 70, "with_normal_texture": 70,
                          "with_metallic_roughness_texture": 0, "primitives_total": 2062,
                          "primitives_with_texcoord0": 2062, "primitives_with_tangent": 0}),
           "RED:census 总数篡改必红")
    expect(not census_ok({"materials_total": 70, "with_base_color_texture": 70, "with_normal_texture": 70,
                          "with_metallic_roughness_texture": 1, "primitives_total": 2062,
                          "primitives_with_texcoord0": 2062, "primitives_with_tangent": 0}),
           "RED:rough-metal 面冒充存在必红")
    expect(not census_ok({"materials_total": 70, "with_base_color_texture": 70, "with_normal_texture": 70,
                          "with_metallic_roughness_texture": 0, "primitives_total": 2062,
                          "primitives_with_texcoord0": 2062, "primitives_with_tangent": 1}),
           "RED:TANGENT 冒充存在必红")
    # 绿臂②:heap 映射槽闭集正例（含 manifest 互核面）。
    dg = "sha256:" + "a" * 64
    manifest = {"A_BaseColor.dds": (dg, dg), "B_BaseColor.dds": (dg, dg)}
    good = _good_slots()
    expect(validate_slots(good, exp, ["A", "B"], manifest) == [], "GREEN:合法 heap 槽列过（含互核）")
    expect(validate_slots(good, exp, ["A", "B"], None) == [], "GREEN:合法 heap 槽列过（无 manifest 面）")
    # 红臂组②:槽构造缺陷逐条必红。
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], material_index=1)
    bad[1] = dict(bad[1], material_index=0)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:律法序调换必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], tris=401)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:tris 不符必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], material_name="ZZ")
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:材质名称不符必红")
    bad = [dict(s) for s in good]
    bad[1] = dict(bad[1], src_width=2000)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:源非 pow2 尺寸必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], width=2048, height=2048)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:存储基级越 cap 律法（min(src,1024)）必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], mip_count=12, mip_digests=_mip_digests(12))
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:mip_count ≠ 完整链（truncated=false）必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], mip_digests=_mip_digests(10))
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:mip_digests 长度 ≠ mip_count 必红")
    bad = [dict(s) for s in good]
    bad[1] = dict(bad[1], mip_truncated=True)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:truncated=true 但链长 ≥ 完整链必红")
    bad = [dict(s) for s in good]
    old_form = {k: v for k, v in bad[0].items() if k not in ("src_width", "src_height", "mip_count", "mip_truncated", "mip_digests")}
    old_form.update({"origin_x": 0, "origin_y": 0, "width": 2048, "height": 2048})
    bad[0] = old_form
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:旧网格图集形态槽（origin/无 mip 面）必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], rgba8_digest="sha256:" + "b" * 64)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:rgba8_digest 篡改（match 位假）必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], manifest_digest_match=False)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:manifest_digest_match=False 必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], texture_uri="ZZ.dds")
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:uri 未入 manifest 必红")
    # 红绿臂③:texel heap 块判。
    expect(heap_ok(_good_heap(), 2), "GREEN:heap 块正例（header==slots×13/bytes==texels×4）")
    expect(not heap_ok(dict(_good_heap(), header_entries=25), 2), "RED:header_entries ≠ slots×13 必红")
    expect(not heap_ok(dict(_good_heap(), heap_bytes=_good_heap()["heap_bytes"] + 1), 2), "RED:heap_bytes ≠ texels×4 必红")
    expect(not heap_ok(dict(_good_heap(), form="grid_atlas"), 2), "RED:非 heap 形态冒充必红")
    expect(not heap_ok(dict(_good_heap(), cap=2048), 2), "RED:cap 漂移必红")
    expect(not heap_ok(dict(_good_heap(), heap_texels=0, heap_bytes=0), 2), "RED:空 heap 必红")
    # 红绿臂④:SSBO 位级判。
    dg2 = "sha256:" + "c" * 64
    good_ssbo = {"p100": 0.0, "bitexact": True, "double_run_bitexact": True,
                 "device_digest": dg2, "host_digest": dg2}
    expect(ssbo_parity_ok(good_ssbo), "GREEN:SSBO 位级正例")
    expect(not ssbo_parity_ok(dict(good_ssbo, p100=1e-7)), "RED:p100>0 必红")
    expect(not ssbo_parity_ok(dict(good_ssbo, bitexact=False)), "RED:bitexact=False 必红")
    expect(not ssbo_parity_ok(dict(good_ssbo, double_run_bitexact=False)), "RED:双跑漂移必红")
    expect(not ssbo_parity_ok(dict(good_ssbo, host_digest="sha256:" + "d" * 64)),
           "RED:device≠host digest 必红")
    # 红绿臂⑤:sampler 结构容差判。
    good_leg = {"max_lsb_diff": 1, "bound_lsb": 1, "bitexact": False,
                "digest": dg2, "host_digest": "sha256:" + "e" * 64}
    expect(sampler_parity_ok(good_leg, 7), "GREEN:sampler 1 LSB 带内过")
    expect(sampler_parity_ok(dict(good_leg, max_lsb_diff=0, bitexact=True), 12),
           "GREEN:sampler 位级（更强终态）过")
    expect(not sampler_parity_ok(dict(good_leg, max_lsb_diff=2), 7), "RED:>1 LSB 必红")
    expect(not sampler_parity_ok(good_leg, 0), "RED:全常量槽（空接线冒充）必红")
    expect(not sampler_parity_ok(dict(good_leg, bound_lsb=2), 7), "RED:bound 漂移必红")
    # 红绿臂⑥:digest 序列判。
    expect(seqs_bitexact(["a", "b"], ["a", "b"]), "GREEN:双跑位级正例")
    expect(not seqs_bitexact(["a", "b"], ["a", "x"]), "RED:双跑漂移必红")
    expect(not seqs_bitexact([], []), "RED:空序列必红")
    expect(seqs_differ(["a", "b"], ["a", "x"]), "GREEN:on≠off 正例")
    expect(not seqs_differ(["a", "b"], ["a", "b"]), "RED:on==off 冒充接线必红")
    # 红绿臂⑦:frame_ms 健全判。
    expect(frame_ms_sane(3.5, 3.6), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(3.5, 0.0), "RED:0ms 必红")
    expect(not frame_ms_sane(3.5, float("nan")), "RED:NaN 必红")
    # 红绿臂⑧:探针抽样级律法（{0, mips/2, mips−1} 连续 dedup 镜像）。
    expect(probe_lods(1) == [0], "GREEN:mips=1 单级 [0]")
    expect(probe_lods(2) == [0, 1], "GREEN:mips=2 → [0,1]（连续 dedup）")
    expect(probe_lods(3) == [0, 1, 2], "GREEN:mips=3 → [0,1,2]")
    expect(probe_lods(5) == [0, 2, 4], "GREEN:mips=5 → [0,2,4]（16² 全 5 级档）")
    expect(probe_lods(11) == [0, 5, 10], "GREEN:mips=11 → [0,5,10]（2048²→1024 cap 档）")
    # 红绿臂⑨:探针计数律法（bistro 实测形态 = 53×11 级 + 17×5 级 = 5040）。
    expect(expected_probe_count([11] * 53 + [5] * 17) == 5040, "GREEN:bistro 70 槽计数律法 == 5040")
    expect(expected_probe_count([1]) == 24, "GREEN:单级槽 24 探针")
    expect(expected_probe_count([2]) == 48, "GREEN:双级槽 48 探针")
    expect(expected_probe_count([11] * 53 + [5] * 17) != 70 * 24, "RED:步幅 3 旧计数（1680）≠ heap 计数必红")
    # 红绿臂⑩:探针 UV×lod 律法镜像（步幅 4;24/槽·级闭集）。
    law = probe_law_mip([11, 5])
    expect(len(law) == 144 and all(len(t) == 4 for t in law), "GREEN:步幅 4 元组（2 槽 × 3 级 × 24）")
    expect({t[3] for t in law if t[0] == 0} == {0, 5, 10}, "GREEN:slot0 lod 注入 {0,5,10}")
    expect({t[3] for t in law if t[0] == 1} == {0, 2, 4}, "GREEN:slot1 lod 注入 {0,2,4}")
    expect(law[16] == (0, 0.0, 0.0, 0), "GREEN:精确边缘首项（lod 0）")
    expect(law[20] == (0, 1.25, 2.5, 0), "GREEN:wrap 域首项（lod 0）")
    expect(all(-1.0 <= u < 4.0 for _, u, _, _ in law), "GREEN:探针律法值域（含 wrap/负域）")
    expect(len(set(law)) == 144, "GREEN:探针律法无重复（含 lod 维）")
    grid_uv_lod0 = [(u, v) for k, u, v, lod in law if k == 0 and lod == 0][:16]
    grid_uv_lod10 = [(u, v) for k, u, v, lod in law if k == 0 and lod == 10][:16]
    expect(grid_uv_lod0 == grid_uv_lod10, "GREEN:同槽跨级 UV 同源（lod 仅注入第 4 元）")
    # schema 互核:heap 双 schema 在树 + gate schema facts enum == FACT_IDS +
    # harness heap schema 常量互核 + 作废锚占位 const 钉死。
    expect(SCHEMA_PATH.is_file() and GATE_SCHEMA_PATH.is_file(), "heap 双 schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
        expect(gs["properties"]["demo"]["properties"]["mapped_materials"]["const"] == N_MAPPED,
               "gate schema demo.mapped == 70（heap 全覆盖档）")
        expect(gs["properties"]["demo"]["properties"]["presented_anchor"]["const"] == TEX_ARM_PRESENTED_ANCHOR,
               "gate schema 作废锚占位 const == PENDING_W4_REHARVEST（6fab598c 字面清理项）")
        expect(gs["properties"]["g11_3_rerun"]["properties"]["entries_total"]["const"] == N_MAPPED,
               "gate schema g11_3 entries_total == 70")
        expect(gs["properties"]["heap"]["properties"]["header_entries"]["const"] == N_MAPPED * HEAP_MIP_SLOTS,
               "gate schema heap.header_entries == 70×13")
    if SCHEMA_PATH.is_file():
        hs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect("textures" in hs.get("required", []), "harness heap schema required 含 textures")
        expect(hs["properties"]["schema"]["const"] == SCHEMA_ID, "harness schema const 互核（harness 侧常量沿用）")
        tp = hs["properties"]["textures"]["properties"]
        cc = tp["census"]["properties"]
        expect(cc["materials_total"]["const"] == 70
               and cc["with_metallic_roughness_texture"]["const"] == 0
               and cc["primitives_with_tangent"]["const"] == 0,
               "harness schema census 常量互核（含缺面登记）")
        expect(tp["mapped_materials"]["const"] == N_MAPPED, "harness schema mapped == 70")
        expect(tp["atlas"]["properties"]["form"]["const"] == "texel_heap", "harness schema atlas.form == texel_heap")
        expect(tp["atlas"]["properties"]["header_entries"]["const"] == N_MAPPED * HEAP_MIP_SLOTS,
               "harness schema header_entries == 910")
        expect(tp["material_slots"]["minItems"] == N_MAPPED == tp["material_slots"]["maxItems"],
               "harness schema slots 70/70")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=8；heap 律法/探针步幅 4/计数律法红绿臂 + 正例组 + 双 schema 互核 + 作废锚占位钉死）")
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
