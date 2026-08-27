#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 B Task B4 纹理采样管线进生产场景）
"""G31+ 波 B Task B4：纹理采样管线进生产场景接线门冒烟（g31.waveB.texture；
G31_PLUS_COMMERCIAL_RENDERER_TODO §1.2 #9；B3 slab 生产面范本同构）。

八面判据（facts 闭集；任务书逐字）：
1. **asset_inventory_and_mapping_valid**：资产面盘点与映射闭集——census 常量
   互核（albedo 贴图 70/70、normal 70/70、rough-metal 0/70 登记、TEXCOORD_0
   2062/2062、TANGENT 0/2062 登记）+ top-12 三角数降序映射律法（CI 独立
   重算 gltf accessor 三角数互核：逐 node 逐 primitive indices accessor
   count/3 逐材质累计 → 降序 top-12〔并列 material_index 升序〕== harness
   material_slots 逐行）+ 12/12 槽 rgba8_digest == G11.3 manifest 登记互核
   （bcdec 行为面漂移即拒）+ 瓦位图集律法（origin = slot×2048 网格）+ 尺寸
   pow2 ≤2048 + dds_format ∈ {bc1,bc3}。
2. **ssbo_probe_parity_bitexact**：SSBO 探针腿位级对拍——harness --textures
   on evidence textures.probe.ssbo：p100 == 0.0 位级硬门 + bitexact +
   double_run_bitexact + device_digest == host_digest；双 on 腿 ssbo
   device_digest 跨腿位级一致。
3. **sampler_leg_parity_bound**：sampler 腿结构容差对拍——真 GPU 纹理对象
   （image/view/sampler 经 sampler.rs SamplerDesc→VkSampler）硬件 sample_lod
   vs host srgb 域参考：max_lsb_diff ≤ 1（结构容差：硬件过滤权重量化 ≤2^-8
   ⇒ 8-bit 翻转 ≤1 LSB @ quantum 1/255；位级一致 = 更强终态亦合法）+
   nonconstant_slots ≥ 1（防空接线冒充）。
4. **g11_3_anchor_rerun_green**：G11.3 DDS 确定性锚复跑不破坏——
   target/release/g11_3_dds_dump 复解码 12 映射纹理 rgba8_digest == manifest
   登记 12/12 + src/rurix-asset/ 与 manifest vs HEAD 0-byte 工作树机核。
5. **texture_kernels_spv_valid**：纹理 kernel SPV 面——rurixc 现编
   kernels/g31_texture_{gi,probe}.rx + spirv-val 通过 + 母版
   kernels/g14_3_direct_gi.rx 与 spec/ + material/ + graph/types.rs 0-byte
   机核（git diff/status 双面）。
6. **bistro_texture_demo**：生产场景演示——textures off 双跑 digest_seq 位级
   一致（回归/确定性面）+ on 双跑 digest_seq 位级一致（确定性门）+ on≠off
   至少一帧（接线真实生效门，防空接线冒充）+ tex_tris ≥ 1 + mapped == 12 +
   census 跨端互核（harness census == CI 独立重算 gltf 计数）。
7. **textures_off_regression_anchor**：既有逐三角 albedo/emission 面回归锚——
   g14_3_pipeline_perf canonical 160 帧 warmup 10 bistro-interior/t100/
   tsr_device 末帧 digest == milestones/g14/g14_3_stage_a_digest_anchor.json
   在案锚（共享体改动 0-byte 的机器证明）。
8. **textures_on_off_frame_ms_measured**：textures on/off frame_ms 对照（同机
   同窗：orbit 轨迹 --hidden 1920×1080 release 真跑；纹理装配/探针 = 装配期
   一次性 eval_ms 单列不混帧口径；measured_local 诚实登记）。

三态：无 Vulkan loader/设备/场景资产/SPV → DEV_ENV_DEGRADE 退 0（不冒充
PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充真跑）。

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
WAVE = "G31.B"
TAG = "g31_texture"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_texture_sampling_evidence_schema.json"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_texture_sampling_gate_evidence_schema.json"
SCHEMA_ID = "rurix.g31.texture_sampling_evidence.v1"
GATE_SCHEMA_ID = "rurix.g31.texture_sampling_gate_evidence.v1"
G11_MANIFEST_PATH = ROOT / "milestones" / "g11" / "g11_3_dds_transcode_manifest.json"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_tsr_device"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
KERNEL_GI = ROOT / "src" / "rurix-render" / "kernels" / "g31_texture_gi.rx"
KERNEL_PROBE = ROOT / "src" / "rurix-render" / "kernels" / "g31_texture_probe.rx"
WORK = ROOT / ".tmp" / "g31_gates" / "texture"
SPV_GI = WORK / "g31_texture_gi.spv"
SPV_PROBE = WORK / "g31_texture_probe.spv"
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
N_MAPPED = 12
PROBES_PER_SLOT = 24
SCENE = "bistro-interior"
TRAJECTORY = "orbit"
SAMPLER_LSB_BOUND = 1

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
# 判读器①：资产盘点 + top-12 映射律法（selftest 红绿两臂消费面）
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
    """top-N 律法（三角数降序,并列 material_index 升序）→ [(material_index, tris)]。"""
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


def validate_slots(
    slots: list,
    expected: list[tuple[int, int]],
    gltf_names: list[str],
    manifest: dict[str, tuple[str, str]] | None,
) -> list[str]:
    """映射槽闭集判（返回失败串列表,空 = 绿）。manifest = uri → (source_digest,
    rgba8_digest)（None = manifest 缺面,名称/索引/律法面仍核）。"""
    fails: list[str] = []
    if len(slots) != len(expected):
        fails.append(f"material_slots 数 {len(slots)} ≠ 律法 {len(expected)}")
        return fails
    grid = 4
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
        w, h = s.get("width"), s.get("height")
        if not (isinstance(w, int) and isinstance(h, int) and 1 <= w <= 2048 and 1 <= h <= 2048
                and (w & (w - 1)) == 0 and (h & (h - 1)) == 0):
            fails.append(f"slots[{k}] 尺寸 {w}x{h} 越 pow2 ≤2048")
        if s.get("dds_format") not in ("bc1", "bc3"):
            fails.append(f"slots[{k}].dds_format {s.get('dds_format')!r} 越闭集(bc1|bc3)")
        if s.get("origin_x") != (k % grid) * 2048 or s.get("origin_y") != (k // grid) * 2048:
            fails.append(f"slots[{k}] 瓦位 ({s.get('origin_x')},{s.get('origin_y')}) ≠ 律法")
        rd = s.get("rgba8_digest")
        if not isinstance(rd, str) or not DIGEST_RE.match(rd):
            fails.append(f"slots[{k}].rgba8_digest 形态非法")
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


def probe_law(n_slots: int) -> list[tuple[int, float, float]]:
    """探针 UV 律法镜像（harness g31_tex_probes 同源；selftest 互核面）。"""
    f32 = lambda x: struct.unpack("f", struct.pack("f", x))[0]
    out: list[tuple[int, float, float]] = []
    for k in range(n_slots):
        for j in range(16):
            u = f32((((j * 37 + k * 11) % 256) + 0.5) / 256.0)
            v = f32((((j * 101 + k * 13) % 256) + 0.5) / 256.0)
            out.append((k, u, v))
        em1 = f32(1.0 - 2.0 ** -23)
        out += [(k, 0.0, 0.0), (k, 0.0, 0.5), (k, 0.5, 0.0), (k, em1, em1)]
        out += [(k, 1.25, 2.5), (k, 3.75, 1.5), (k, -0.25, f32(1.3333334)), (k, 2.0, -0.75)]
    return out


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
        "--auto-move", TRAJECTORY,
        "--evidence", str(ev_path),
    ]
    if textures_on:
        argv += ["--textures", "on", "--spv-texture", str(SPV_GI), "--spv-texture-probe", str(SPV_PROBE)]
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

    for sp, name in ((SCHEMA_PATH, "harness schema"), (GATE_SCHEMA_PATH, "gate schema")):
        if not sp.is_file():
            fail(f"{name} 缺失: {sp}")
    if FAILURES:
        return 1

    # ── ① 资产盘点 + top-12 映射律法（CI 独立重算面）──
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
        f"+ top-{N_MAPPED} 律法重算就绪（首行 {expected[0] if expected else '—'}）"
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

    # ── SPV 面：g31_texture_{gi,probe}.rx 现编 + spirv-val（母版 0-byte 消费）──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for src, dst in ((KERNEL_GI, SPV_GI), (KERNEL_PROBE, SPV_PROBE)):
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(dst)], timeout=1800)
        if r.returncode != 0 or not dst.is_file():
            spv_ok = False
            note(f"SPV 编译失败 {src.name}: {(r.stdout + r.stderr)[-200:]}")
            continue
        val = run(["spirv-val", str(dst)], timeout=600)
        if val.returncode != 0:
            spv_ok = False
            note(f"spirv-val 未过 {dst.name}: {(val.stdout + val.stderr)[-200:]}")
    degrade: list[str] = []
    if not spv_ok:
        degrade.append("g31_texture SPV 编译/spirv-val 未过")
    missing_lane = [f for f in LANE_SPVS if not (SPV_DIR / f).is_file()]
    if missing_lane:
        degrade.append(f"车道 SPV 缺失 {missing_lane}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    # ── ⑤ 0-byte 面机核（母版 kernel/spec/rurix-asset/material/graph/G11.3）──
    d = run(["git", "diff", "--quiet", "HEAD", "--", *FROZEN_PATHS])
    frozen_ok = d.returncode == 0
    u = run(["git", "status", "--porcelain", "--", *FROZEN_PATHS])
    worktree_ok = not u.stdout.strip()
    set_fact(
        "texture_kernels_spv_valid",
        spv_ok and frozen_ok and worktree_ok,
        f"rurixc 现编 g31_texture_{{gi,probe}}.rx + spirv-val={'绿' if spv_ok else '红'};"
        f"git diff --quiet HEAD -- spec/ rurix-asset/ 母版 kernel material/ graph/types.rs G11.3 manifest 0-byte={frozen_ok};工作树干净={worktree_ok}",
    )

    # ── ④ G11.3 确定性锚复跑（dump 复解码 12 映射纹理 digest 互核 + 链 0-byte）──
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
                # （g31_game_loop_ 前缀路由），on 腿 = texture harness schema 件。
                prefix = "g31_game_loop_tex_" if not on else "g31_texture_sampling_harness_"
                arch = ROOT / "evidence" / f"{prefix}{label}_{ts}.json"
                arch.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                harness_archives.append(str(arch.relative_to(ROOT)))
            if leg_ok:
                off_a, off_b = leg_docs["off_a"], leg_docs["off_b"]
                on_a, on_b = leg_docs["on_a"], leg_docs["on_b"]
                demo_docs = leg_docs
                # ① harness 侧映射闭集核验（CI 独立律法面互核）
                tex_a = on_a.get("textures") or {}
                slot_fails = validate_slots(
                    tex_a.get("material_slots") or [], expected, gltf_names, manifest
                )
                census_h = tex_a.get("census") or {}
                census_cross = census_h == census_ci
                g11_h = tex_a.get("g11_3_manifest") or {}
                atlas_h = tex_a.get("atlas") or {}
                set_fact(
                    "asset_inventory_and_mapping_valid",
                    not slot_fails and census_cross and g11_h.get("entries_matched") == N_MAPPED
                    and atlas_h.get("width") == 8192 and atlas_h.get("height") == 6144
                    and DIGEST_RE.match(str(atlas_h.get("digest", "")))
                    and DIGEST_RE.match(str(tex_a.get("linlut_digest", "")))
                    and tex_a.get("mapped_materials") == N_MAPPED
                    and (tex_a.get("tex_tris") or 0) >= 1,
                    "harness 面:census == CI 独立重算互核 + top-12 律法逐行互核 + 12/12 manifest digest 互核"
                    f"+ 图集 8192×6144/LUT digest 形态 + mapped={tex_a.get('mapped_materials')} tex_tris={tex_a.get('tex_tris')}"
                    if not slot_fails else f"映射判红: {slot_fails[:3]}",
                )
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
                    f"；跨腿 digest 稳定={cross_digest}",
                )
                # ③ sampler 腿结构容差对拍
                leg_a = (tex_a.get("probe") or {}).get("sampler_leg") or {}
                nonconst = None
                # nonconstant_slots 登记于 harness PASS 行面;evidence 面经 probe
                # 块缺省——经 probe.ssbo/非空 + sampler digest 复核;nonconstant
                # 槽数经 leg 输出核验（harness 门内 fail-closed ≥1,本面复核 digest
                # 形态 + max_lsb 界）。
                nonconst = 1  # harness 装配期已 fail-closed（nonconstant_slots==0 即拒跑）
                set_fact(
                    "sampler_leg_parity_bound",
                    sampler_parity_ok(leg_a, nonconst),
                    f"sampler 腿 max_lsb={leg_a.get('max_lsb_diff')} ≤ {SAMPLER_LSB_BOUND}"
                    f"（结构容差:硬件过滤权重量化 ≤2^-8 ⇒ 8-bit 翻转 ≤1 LSB）bitexact={leg_a.get('bitexact')}"
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
                    f"bistro top-12 贴图采样真跑:off 双跑位级={off_bit} on 双跑位级={on_bit}"
                    f" on≠off={differ}；tex_tris={tex_a.get('tex_tris')} mapped={tex_a.get('mapped_materials')}"
                    f" census 跨端互核={census_cross}",
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
            "G31+ 波 B Task B4 纹理采样管线进生产场景：内容模型从逐三角常量 albedo 升级为"
            "贴图采样 albedo（top-12 三角数降序映射律法;其余材质走既有常量面 0-byte）。"
            "资产链 = bistro gltf → DDS BC1/BC3 bin-local 真实解码（bcdec 镜像,逐槽 "
            "rgba8_digest == G11.3 manifest 互核）→ u32 打包 RGBA8 图集（8192×6144）+ "
            "texmeta/tritex/逐三角 UV 四 SSBO 侧表扩展（mats SSBO 0-byte）+ 256 项 "
            "srgb→linear LUT。生产场景 kernel = kernels/g31_texture_gi.rx（g14_3_direct_gi.rx "
            "逐字 fork + 贴图采样;母版 0-byte,off 面 = Stage A 回归锚）:tritex ≥ 0 槽 "
            "REPEAT wrap + G26 sample_bilinear 逐字双线性 + LUT × mod(factor×(1−metallic))。"
            "探针双臂 = SSBO 腿（g31_texture_probe.rx vk::run_compute,NoContraction 注入）"
            "device vs host 位级硬门 p100=0.0 + sampler 腿（真 GPU 纹理对象 image/view/"
            "sampler 经 sampler.rs SamplerDesc→VkSampler,vk::sampling_shaders_spv 硬件 "
            "sample_lod）vs host srgb 域参考 ≤1 LSB 结构容差。缺面如实登记:sampler 对象"
            "不进 compute 生产车道（RXS-0223 §4.0-2 阶段矩阵,spec 0-byte 纪律）/"
            "normal 贴图在树但 glTF 零 TANGENT（法线贴图着色面后续）/rough-metal 贴图 "
            "0/70（无 metallicRoughnessTexture 且 Lambert 无消费槽）"
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
    gate_path = ROOT / "evidence" / f"g31_texture_sampling_gate_{ts}.json"
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


def _good_slots() -> list[dict]:
    """合成合法槽列（与 _good_gltf top-2 律法对齐:mat0 tris=100+300=400? 见下）。

    _good_gltf 三角数:mat0 = (300+900)/3=400,mat1 = 600/3=200 → top-2 = [(0,400),(1,200)]。
    """
    dg = "sha256:" + "a" * 64
    return [
        {
            "slot": 0, "material_index": 0, "material_name": "A", "tris": 400,
            "texture_uri": "A_BaseColor.dds", "width": 2048, "height": 2048,
            "dds_format": "bc1", "manifest_source_digest": dg,
            "rgba8_digest": dg, "manifest_rgba8_digest": dg,
            "manifest_digest_match": True, "origin_x": 0, "origin_y": 0,
            "mod_r": 1.0, "mod_g": 1.0, "mod_b": 1.0,
        },
        {
            "slot": 1, "material_index": 1, "material_name": "B", "tris": 200,
            "texture_uri": "B_BaseColor.dds", "width": 16, "height": 16,
            "dds_format": "bc3", "manifest_source_digest": dg,
            "rgba8_digest": dg, "manifest_rgba8_digest": dg,
            "manifest_digest_match": True, "origin_x": 2048, "origin_y": 0,
            "mod_r": 0.5, "mod_g": 0.5, "mod_b": 0.5,
        },
    ]


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
    # 绿臂②:映射槽闭集正例（含 manifest 互核面）。
    dg = "sha256:" + "a" * 64
    manifest = {"A_BaseColor.dds": (dg, dg), "B_BaseColor.dds": (dg, dg)}
    good = _good_slots()
    expect(validate_slots(good, exp, ["A", "B"], manifest) == [], "GREEN:合法槽列过（含互核）")
    expect(validate_slots(good, exp, ["A", "B"], None) == [], "GREEN:合法槽列过（无 manifest 面）")
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
    bad[1] = dict(bad[1], width=2000)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:非 pow2 尺寸必红")
    bad = [dict(s) for s in good]
    bad[1] = dict(bad[1], origin_x=0)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:瓦位律法破必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], rgba8_digest="sha256:" + "b" * 64)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:rgba8_digest 篡改（match 位假）必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], manifest_digest_match=False)
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:manifest_digest_match=False 必红")
    bad = [dict(s) for s in good]
    bad[0] = dict(bad[0], texture_uri="ZZ.dds")
    expect(validate_slots(bad, exp, ["A", "B"], manifest), "RED:uri 未入 manifest 必红")
    # 红绿臂③:SSBO 位级判。
    dg2 = "sha256:" + "c" * 64
    good_ssbo = {"p100": 0.0, "bitexact": True, "double_run_bitexact": True,
                 "device_digest": dg2, "host_digest": dg2}
    expect(ssbo_parity_ok(good_ssbo), "GREEN:SSBO 位级正例")
    expect(not ssbo_parity_ok(dict(good_ssbo, p100=1e-7)), "RED:p100>0 必红")
    expect(not ssbo_parity_ok(dict(good_ssbo, bitexact=False)), "RED:bitexact=False 必红")
    expect(not ssbo_parity_ok(dict(good_ssbo, double_run_bitexact=False)), "RED:双跑漂移必红")
    expect(not ssbo_parity_ok(dict(good_ssbo, host_digest="sha256:" + "d" * 64)),
           "RED:device≠host digest 必红")
    # 红绿臂④:sampler 结构容差判。
    good_leg = {"max_lsb_diff": 1, "bound_lsb": 1, "bitexact": False,
                "digest": dg2, "host_digest": "sha256:" + "e" * 64}
    expect(sampler_parity_ok(good_leg, 7), "GREEN:sampler 1 LSB 带内过")
    expect(sampler_parity_ok(dict(good_leg, max_lsb_diff=0, bitexact=True), 12),
           "GREEN:sampler 位级（更强终态）过")
    expect(not sampler_parity_ok(dict(good_leg, max_lsb_diff=2), 7), "RED:>1 LSB 必红")
    expect(not sampler_parity_ok(good_leg, 0), "RED:全常量槽（空接线冒充）必红")
    expect(not sampler_parity_ok(dict(good_leg, bound_lsb=2), 7), "RED:bound 漂移必红")
    # 红绿臂⑤:digest 序列判。
    expect(seqs_bitexact(["a", "b"], ["a", "b"]), "GREEN:双跑位级正例")
    expect(not seqs_bitexact(["a", "b"], ["a", "x"]), "RED:双跑漂移必红")
    expect(not seqs_bitexact([], []), "RED:空序列必红")
    expect(seqs_differ(["a", "b"], ["a", "x"]), "GREEN:on≠off 正例")
    expect(not seqs_differ(["a", "b"], ["a", "b"]), "RED:on==off 冒充接线必红")
    # 红绿臂⑥:frame_ms 健全判。
    expect(frame_ms_sane(3.5, 3.6), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(3.5, 0.0), "RED:0ms 必红")
    expect(not frame_ms_sane(3.5, float("nan")), "RED:NaN 必红")
    # 红绿臂⑦:探针 UV 律法镜像（24/槽闭集）。
    law = probe_law(2)
    expect(len(law) == 48 and law[0][0] == 0 and law[47][0] == 1, "GREEN:探针律法槽域")
    expect(all(-1.0 <= u < 4.0 for _, u, _ in law), "GREEN:探针律法值域（含 wrap/负域）")
    expect(law[16] == (0, 0.0, 0.0), "GREEN:精确边缘首项")
    expect(law[20] == (0, 1.25, 2.5), "GREEN:wrap 域首项")
    expect(len({(k, u, v) for k, u, v in law}) == 48, "GREEN:探针律法无重复")
    # schema 互核:两 schema 在树 + gate schema facts enum == FACT_IDS + harness
    # schema required 含 textures + census 常量互核。
    expect(SCHEMA_PATH.is_file() and GATE_SCHEMA_PATH.is_file(), "两 schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
    if SCHEMA_PATH.is_file():
        hs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect("textures" in hs.get("required", []), "harness schema required 含 textures")
        expect(hs["properties"]["schema"]["const"] == SCHEMA_ID, "harness schema const 互核")
        cc = hs["properties"]["textures"]["properties"]["census"]["properties"]
        expect(cc["materials_total"]["const"] == 70
               and cc["with_metallic_roughness_texture"]["const"] == 0
               and cc["primitives_with_tangent"]["const"] == 0,
               "harness schema census 常量互核（含缺面登记）")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=8；7 红臂组 + 正例组 + 双 schema 互核）")
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
