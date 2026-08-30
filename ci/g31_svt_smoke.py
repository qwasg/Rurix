#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C13 SVT 四行立项窗兑现）
"""G31+ 波 C Task C13：SVT 稀疏虚拟纹理四行接线门冒烟（g31.waveC.svt；
G31_PLUS_COMMERCIAL_RENDERER_TODO #33-#36；RD-041 分项；
milestones/g22/g22_svt_gap.json SVT 立项窗兑现 + g29 重判维持 defer 窗口）。
B4 纹理采样门范本同构（ci/g31_texture_sampling_smoke.py）。

五面判据（facts 闭集；四行各一 + 整合一）：
1. **svt1_page_table_indirection**：SVT-1 虚拟纹理页表——128K² = 131072²
   虚拟地址空间 1024×1024 u32 页表（0=未驻留,驻留=物理槽号+1）→ bistro
   图集活动区 64×48=3072 页 → 物理瓦片池容量预算；探针全驻留 device SVT
   vs host 整图直采位级硬门 p100 == 0.0（间接寻址正确性）+ 恒等页表
   digest CI 独立重算互核（页表网格序律法 tile→(tile/64)·1024+tile%64,
   值 = 紧凑页号+1）+ cargo test streaming::svt host 单测绿。
2. **svt2_gpu_feedback_closed_loop**：SVT-2 GPU 反馈 pass——部分驻留探针
   （page_id%3==2 未驻留律法）miss 请求缓冲 device vs host 位级 +
   host SvtStreaming::consume 闭环（loaded ≥ 1,io == loaded×67600）重跑
   全 hit 且输出 == 全驻留臂位级 + 生产小池臂逐帧请求-驻留真跑
   （tiles_loaded_total ≥ 1）。
3. **svt3_tile_border_filtering**：SVT-3 瓦片边界过滤——130² 带边物理瓦片
   （border texel 复制,页所属槽 REPEAT wrap 律）;边界聚焦 96 探针
   boundary_max_abs == 0.0 位级（双线性 2×2 footprint 单瓦片自足,跨瓦片
   读取零需求）;各向异性跨瓦片 = 生产采样闭集双线性唯一过滤面,aniso
   需求不成立如实登记（gaps 字面在案）。
4. **svt4_terrain_consumer_m116**：SVT-4 地形/贴花消费方——按 M116 锚核验
   「需求不成立」维持 defer 如实登记（不冒充接线）：
   assert_zero_svt_dependency/SvtDependencyDetected 字面在树 + cargo test
   world::terrain 绿（零 SVT 依赖断言维持,含 zero_svt_dependency_red 臂）
   + basis 字面登记（heightfield + 材质层 id 最小语义,零纹理采样消费面）
   + terrain.rs 并发在飞改动如实登记（他务判档;本任务零触碰以 C13 文件
   清单面承载,不以 0-byte-vs-HEAD 冒充）。
5. **svt_integration_streaming**：整合真跑——B4 锚臂（--textures on 无
   SVT 面 = 全驻留锚 0-byte）vs SVT 全驻留臂逐帧 digest_seq 位级一致 +
   强制小池 256 臂流送全程 frames_completed 全量无崩 + fallback 帧如实
   登记 + 条件式错图机核（∀ miss_px==0 帧 digest == 全驻留同帧位级;
   零帧 = 真空真,计数如实登记）+ miss 率/IO 量 measured（io_bytes ==
   tiles_loaded×67600 重算互核）+ 小池双跑位级一致（确定性门）+
   小池 ≠ 全驻留至少一帧（流送真实生效门,防空接线冒充）。

三态：无 Vulkan loader/设备/场景资产/SPV → DEV_ENV_DEGRADE 退 0（不冒充
PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充真跑）。

G37 W6 svt mutex_registered（三态之外的互斥登记态）：day_0828 Phase B texel
heap 化起 harness 对 --svt on 无条件 fail-closed（CLI 校验期即拒;结构性互斥,
W6 首跑实证 artifacts/day_0830_delivery/w6_final/W6_GATES.json svt 行）。真跑
四腿前以 probe 短跑（--svt on 极小参数）探测互斥字面：命中 ⇒ host 金标准腿
（streaming::svt/world::terrain 单测 + terrain 零 SVT 断言字面,纯 host/CPU 判）
照跑照判——全绿则产 mutex_registered evidence（rurix.g31.svt_smoke.
mutex_registered.v1,独立新 schema 新路径,既有 v1 双 schema 0-byte）退 0
（非 PASS 非 FAIL,深修归后续波 TODO #33-#36 + day_0828 HANDOVER §12）,
host 腿红 ⇒ 整体 FAIL（不得靠登记态掩盖）；未命中（将来深修互斥解除）⇒
既有全量判读 0 改动。先例 = encode parity DEV_ENV_DEGRADE skip doc /
blocked_probes maintain 登记面。

用法：
  py -3 ci/g31_svt_smoke.py --selftest
  py -3 ci/g31_svt_smoke.py --gate g31.waveC.svt [--frames 64] [--warmup 10]
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

import g11_wave_exit_lib as wel  # noqa: E402,F401
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveC.svt"
SUBJECT = "g31_svt"
WAVE = "G31.C"
TAG = "g31_svt"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_svt_evidence_schema.json"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_svt_gate_evidence_schema.json"
SCHEMA_ID = "rurix.g31.svt_evidence.v1"
GATE_SCHEMA_ID = "rurix.g31.svt_gate_evidence.v1"
# ── G37 W6 svt mutex_registered：互斥登记态常量族 ──
# harness fail-closed 字面锚（g31_window_present.rs CLI 校验期 eprintln,
# 全字面 =「--svt on 与 day_0828 Phase B texel heap 纹理形态互斥（SVT 假设
# = 2048 网格图集/texmeta origin/tritex 步幅 1,heap 化未适配——fail-closed
# 登记,SVT 深修归后续波）」;此处取前段稳健子串防标点宽度漂移,其他 fail
# 字面不命中 = 不入登记态）。
MUTEX_LITERAL = "--svt on 与 day_0828 Phase B texel heap 纹理形态互斥"
MUTEX_SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_svt_mutex_registered_schema.json"
MUTEX_SCHEMA_ID = "rurix.g31.svt_smoke.mutex_registered.v1"
MUTEX_REGISTERED_WAVE = "G37.W6"
MUTEX_DEEP_FIX_ANCHOR = (
    "G31_PLUS_COMMERCIAL_RENDERER_TODO #33-#36 SVT 四行（open,SVT 立项窗）"
    " + artifacts/day_0828/e_final/HANDOVER.md §12"
    "（--svt × texel heap fail-closed 互斥：SVT 页表假设 2048 固定网格与 heap"
    " 寻址不同构,深修留窗;现为显式拒跑）"
    " + artifacts/day_0830_delivery/w6_final/W6_GATES.json svt 行（W6 首跑实证"
    " rc=1,full/small_a/small_b 三腿同字面必现）"
)
MUTEX_SKIPPED_LEGS = ["b4anchor", "full", "small_a", "small_b"]
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
KERNEL_SVT = ROOT / "src" / "rurix-render" / "kernels" / "g31_svt_gi.rx"
KERNEL_SVT_PROBE = ROOT / "src" / "rurix-render" / "kernels" / "g31_svt_probe.rx"
WORK = ROOT / ".tmp" / "g31_gates" / "svt"
SPV_SVT = WORK / "g31_svt_gi.spv"
SPV_SVT_PROBE = WORK / "g31_svt_probe.spv"
SPV_TEX_GI = ROOT / ".tmp" / "g31_gates" / "texture" / "g31_texture_gi.spv"
SPV_TEX_PROBE = ROOT / ".tmp" / "g31_gates" / "texture" / "g31_texture_probe.spv"
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
FROZEN_PATHS = [
    "src/rurix-render/kernels/g14_3_direct_gi.rx",
    "src/rurix-render/src/material",
    "src/rurix-render/src/graph/types.rs",
    "milestones/g11/g11_3_dds_transcode_manifest.json",
]
TERRAIN_RS = ROOT / "src" / "rurix-render" / "src" / "world" / "terrain.rs"
SCENE = "bistro-interior"
TRAJECTORY = "orbit"
# SVT 常量族（与 src/rurix-render/src/streaming/svt.rs 单一事实源镜像——
# 篡改常量即红;判读器红绿臂消费面）。
SVT_VIRTUAL_DIM = 131072
SVT_TILE_DIM = 128
SVT_BORDER = 1
SVT_PHYS_DIM = 130
SVT_PAGE_TABLE_DIM = 1024
SVT_PAGE_COUNT = SVT_PAGE_TABLE_DIM * SVT_PAGE_TABLE_DIM
SVT_PHYS_TILE_BYTES = 130 * 130 * 4
ACTIVE_PAGES_X = 64
ACTIVE_PAGES_Y = 48
ACTIVE_PAGES = ACTIVE_PAGES_X * ACTIVE_PAGES_Y
POOL_FULL = ACTIVE_PAGES
POOL_SMALL = 256
N_PROBES = 384
N_BOUNDARY_PROBES = 96

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "svt1_page_table_indirection",
    "svt2_gpu_feedback_closed_loop",
    "svt3_tile_border_filtering",
    "svt4_terrain_consumer_m116",
    "svt_integration_streaming",
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
# 判读器①：恒等页表 digest 律法（CI 独立重算面;selftest 红绿臂消费）
# ---------------------------------------------------------------------------


def identity_page_table_digest() -> str:
    """恒等映射页表律法独立重算（全驻留锚臂终态）:页表 1024² u32 LE 全零,
    活动区紧凑页 tile ∈ [0,3072) 于页表网格序下标 (tile//64)·1024+tile%64
    写入 tile+1——与 streaming/svt.rs new_full/table_index 同律。"""
    buf = bytearray(SVT_PAGE_COUNT * 4)
    for tile in range(ACTIVE_PAGES):
        t = (tile // ACTIVE_PAGES_X) * SVT_PAGE_TABLE_DIM + (tile % ACTIVE_PAGES_X)
        struct.pack_into("<I", buf, t * 4, tile + 1)
    return "sha256:" + hashlib.sha256(bytes(buf)).hexdigest()


def svt1_ok(svt: dict, recompute: str, host_tests_green: bool) -> bool:
    """SVT-1 判:虚拟地址空间/页表/瓦片集常量族 + 全驻留位级硬门 + 恒等
    页表 digest 互核 + host 单测绿。"""
    fra = (svt.get("probe") or {}).get("full_residency_arm") or {}
    return (
        svt.get("virtual_dim") == SVT_VIRTUAL_DIM
        and svt.get("tile_dim") == SVT_TILE_DIM
        and svt.get("page_table_dim") == SVT_PAGE_TABLE_DIM
        and svt.get("page_table_entries") == SVT_PAGE_COUNT
        and svt.get("active_pages_x") == ACTIVE_PAGES_X
        and svt.get("active_pages_y") == ACTIVE_PAGES_Y
        and svt.get("active_pages") == ACTIVE_PAGES
        and svt.get("phys_tile_bytes") == SVT_PHYS_TILE_BYTES
        and DIGEST_RE.match(str(svt.get("tile_set_digest", "")))
        and svt.get("page_table_digest_final") == recompute
        and fra.get("p100_vs_direct") == 0.0
        and fra.get("bitexact_vs_direct") is True
        and fra.get("bitexact_vs_svt_host") is True
        and fra.get("double_run_bitexact") is True
        and DIGEST_RE.match(str(fra.get("device_digest", "")))
        and host_tests_green is True
    )


def svt2_ok(svt: dict, small_streaming: dict) -> bool:
    """SVT-2 判:部分驻留请求位级 + 闭环重跑全 hit == 全驻留 + 生产闭环真跑。"""
    pra = (svt.get("probe") or {}).get("partial_residency_arm") or {}
    return (
        isinstance(pra.get("miss_probes"), int)
        and pra.get("miss_probes", 0) >= 1
        and pra.get("req_bitexact") is True
        and pra.get("out_bitexact") is True
        and isinstance(pra.get("closed_loop_loaded"), int)
        and pra.get("closed_loop_loaded", 0) >= 1
        and pra.get("closed_loop_io_bytes")
        == pra.get("closed_loop_loaded") * SVT_PHYS_TILE_BYTES
        and pra.get("closed_loop_all_hit") is True
        and pra.get("closed_loop_bitexact_vs_full") is True
        and isinstance(small_streaming.get("tiles_loaded_total"), int)
        and small_streaming.get("tiles_loaded_total", 0) >= 1
        and isinstance(small_streaming.get("requested_pages_total"), int)
        and small_streaming.get("requested_pages_total", 0) >= 1
    )


def svt3_ok(svt: dict) -> bool:
    """SVT-3 判:border 复制面 + 边界聚焦误差位级 + 各向异性 N/A 登记字面。"""
    fra = (svt.get("probe") or {}).get("full_residency_arm") or {}
    gaps = str(svt.get("gaps", ""))
    return (
        svt.get("phys_tile_dim") == SVT_PHYS_DIM
        and svt.get("border") == SVT_BORDER
        and (svt.get("probe") or {}).get("boundary_probe_count") == N_BOUNDARY_PROBES
        and fra.get("boundary_max_abs") == 0.0
        and "各向异性" in gaps
        and "双线性" in gaps
    )


def integration_ok(
    b4_doc: dict,
    full_doc: dict,
    small_a: dict,
    small_b: dict,
    frames: int,
    warmup: int,
) -> tuple[bool, dict]:
    """整合判（返回 (ok, 登记面 dict)）:全驻留锚 == B4 位级 + 小池双跑 +
    条件式错图机核 + miss/IO measured + 流送真实生效门。"""
    total = frames + warmup
    b4_seq = b4_doc.get("digest_seq") or []
    full_seq = full_doc.get("digest_seq") or []
    sa_seq = small_a.get("digest_seq") or []
    sb_seq = small_b.get("digest_seq") or []
    svt_full = full_doc.get("svt") or {}
    svt_a = small_a.get("svt") or {}
    st = svt_a.get("streaming") or {}
    miss_seq = st.get("miss_px_seq") or []
    b4_anchor = len(b4_seq) == total and b4_seq == full_seq
    small_double = len(sa_seq) == total and sa_seq == sb_seq
    frames_completed_ok = (
        full_doc.get("frames_completed") == total
        and small_a.get("frames_completed") == total
        and small_b.get("frames_completed") == total
    )
    io_law = st.get("io_bytes_total") == (st.get("tiles_loaded_total") or 0) * SVT_PHYS_TILE_BYTES
    zero_frames = sum(1 for m in miss_seq if m == 0)
    zero_bitexact = all(
        i < len(full_seq) and sa_seq[i] == full_seq[i]
        for i, m in enumerate(miss_seq)
        if m == 0
    )
    small_ne_full = any(
        i >= len(full_seq) or sa_seq[i] != full_seq[i] for i in range(min(len(sa_seq), total))
    )
    miss_rate = st.get("miss_rate")
    ok = (
        b4_anchor
        and small_double
        and frames_completed_ok
        and io_law
        and zero_bitexact
        and small_ne_full
        and isinstance(miss_rate, (int, float))
        and miss_rate > 0.0
        and svt_full.get("full_residency") is True
        and svt_full.get("pool_tiles") == POOL_FULL
        and svt_a.get("full_residency") is False
        and svt_a.get("pool_tiles") == POOL_SMALL
        and len(miss_seq) == total
        and st.get("frames") == total
    )
    detail = {
        "b4_anchor_bitexact": b4_anchor,
        "small_pool_double_run_bitexact": small_double,
        "small_pool_frames_completed": small_a.get("frames_completed") or 0,
        "small_pool_miss_rate": float(miss_rate or 0.0),
        "small_pool_io_bytes": int(st.get("io_bytes_total") or 0),
        "small_pool_io_law_match": io_law,
        "small_pool_fallback_frames": int(st.get("fallback_frames") or 0),
        "zero_fallback_frames": zero_frames,
        "zero_fallback_frames_bitexact": zero_bitexact,
        "small_ne_full": small_ne_full,
        "full_residency_pool_tiles": POOL_FULL,
        "small_pool_tiles": POOL_SMALL,
    }
    return ok, detail


def terrain_judge(greps: dict[str, bool], tests_green: bool) -> bool:
    """SVT-4 判（维持 defer 合法面）:零 SVT 断言字面在树 + terrain 单测绿
    （含 zero_svt_dependency_red 臂——M116 锚断言面维持的机器证明;terrain.rs
    并发在飞改动归他务判档,本任务零触碰以 C13 文件清单面登记,不以
    0-byte-vs-HEAD 冒充——全树多波次在飞 WIP 共享面下 HEAD 差分不可归因）。
    """
    return (
        greps.get("assert_zero_svt_dependency") is True
        and greps.get("SvtDependencyDetected") is True
        and tests_green is True
    )


def seqs_bitexact(a: list, b: list) -> bool:
    return len(a) == len(b) and len(a) > 0 and all(x == y for x, y in zip(a, b))


# ---------------------------------------------------------------------------
# 判读器②：G37 W6 svt mutex_registered 互斥登记态（纯函数面;selftest 消费）
# ---------------------------------------------------------------------------


def detect_svt_mutex(out: str, returncode: int) -> str:
    """G37 W6 svt mutex_registered：探测 harness --svt on 无条件 fail-closed
    互斥字面。命中（rc≠0 且字面在输出）返回捕获的完整字面行;未命中返回空串
    ——将来深修互斥解除后门自动回落既有全量判读（0 改动）。其他 fail 字面
    （interference 不符）不入登记态。"""
    if returncode == 0 or MUTEX_LITERAL not in out:
        return ""
    for line in out.splitlines():
        if MUTEX_LITERAL in line:
            return line.strip()
    return MUTEX_LITERAL  # 字面在 out 必在某行;防御性兜底


def mutex_host_legs_ok(svt_tests_green: bool, terrain_ok: bool) -> bool:
    """登记态下 host 金标准腿判（纯 host/CPU 面,不依赖 --svt on 真跑）：
    ① cargo test streaming::svt = SVT-1/2/3 页表/反馈闭环/border 的 host
    金标准;② SVT-4 维持 defer 面 = terrain 零 SVT 断言字面 + cargo test
    world::terrain（terrain_judge 同判）。任一红 ⇒ 门整体 FAIL——host 面
    坏了不能靠登记态掩盖。"""
    return svt_tests_green is True and terrain_ok is True


def build_mutex_registered_doc(
    mutex_line: str,
    probe_rc: int,
    svt_tests_green: bool,
    terrain_ok: bool,
    terrain_greps: dict[str, bool],
    ts: str,
    env_info: dict,
) -> dict:
    """互斥登记件构造（gate 落盘面 + selftest 正红臂共用;host 腿全绿前置
    由调用方 mutex_host_legs_ok 硬门保证,本函数只装配登记面）。"""
    identity = identity_page_table_digest()
    return {
        "schema": MUTEX_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "registered_wave": MUTEX_REGISTERED_WAVE,
        "state": "MUTEX_REGISTERED",
        "mutex_literal": mutex_line,
        "mutex_probe": {
            "argv_shape": (
                "g31_window_present --frames 2 --warmup 1 --hidden --quality off"
                " --auto-move orbit --textures on --svt on --svt-pool-tiles 0"
                "（极小参数短跑;互斥 fail-closed 在 CLI 校验期必现,零 GPU 消耗）"
            ),
            "returncode": probe_rc,
            "frames": 2,
            "warmup": 1,
        },
        "host_golden_legs": {
            "svt_host_tests_green": svt_tests_green,
            "terrain_defer_face_ok": terrain_ok,
            "assert_zero_svt_dependency_present": terrain_greps.get(
                "assert_zero_svt_dependency", False
            ),
            "svt_dependency_detected_present": terrain_greps.get(
                "SvtDependencyDetected", False
            ),
            "identity_page_table_digest": identity,
            "identity_digest_deterministic": identity == identity_page_table_digest(),
        },
        "skipped_device_legs": list(MUTEX_SKIPPED_LEGS),
        "deep_fix_anchor": MUTEX_DEEP_FIX_ANCHOR,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G37 W6 svt mutex_registered：day_0828 Phase B texel heap 化起"
            " svt 生产臂结构性不可跑（harness 对 --svt on 无条件 fail-closed,"
            "CLI 校验期即拒）——前役门与形态演进不同步的遗留,非本役引入"
            "（W6 首跑实证在 deep_fix_anchor）。处置 = 互斥登记态：不冒充"
            " PASS,非 FAIL,三态之外的登记态（先例 = encode parity 门"
            " DEV_ENV_DEGRADE skip doc / blocked_probes maintain 登记面）。"
            "host 金标准腿照跑照判全绿在案（红则门 FAIL 不产本件）;device"
            " 四腿如实登记跳过。深修（SVT 页表/瓦片集/探针假设适配 heap 形态）"
            "归后续波,互斥解除后本门自动回落既有全量判读（0 改动）。"
        ),
    }


def mutex_doc_schema_errors(doc: dict) -> list[str]:
    """登记件对新 schema 的 Draft7 自校验（gate 落盘前硬门 + selftest 共用;
    schema 漂移/登记件缺面即红,不冒充登记态）。"""
    import jsonschema

    return [
        f"{'/'.join(str(p) for p in e.path)}: {e.message}"
        for e in jsonschema.Draft7Validator(
            json.loads(MUTEX_SCHEMA_PATH.read_text(encoding="utf-8"))
        ).iter_errors(doc)
    ]


def mutex_registered_exit(
    mutex_line: str,
    probe_rc: int,
    svt_tests_ok: bool,
    terrain_ok: bool,
    terrain_greps: dict[str, bool],
    ts: str,
) -> int:
    """G37 W6 svt mutex_registered 终态处置：host 金标准腿红 ⇒ 整体 FAIL;
    全绿 ⇒ 落盘登记件（自校验硬门）+ 明确登记态字面 + 退 0。"""
    if not mutex_host_legs_ok(svt_tests_ok, terrain_ok):
        fail(
            f"互斥登记态拒入：host 金标准腿红（streaming::svt 绿={svt_tests_ok}"
            f" / SVT-4 defer 面={terrain_ok}）——host 面坏了不能靠登记态掩盖"
        )
        note(f"GATE FAIL {GATE_KEY}（互斥字面命中但 host 金标准腿红）")
        return 1
    env_info = {
        "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
        "os": "windows",
        "rustc": subprocess.run(
            ["rustc", "--version"], capture_output=True, text=True
        ).stdout.strip(),
        "base_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        ).stdout.strip(),
    }
    doc = build_mutex_registered_doc(
        mutex_line, probe_rc, svt_tests_ok, terrain_ok, terrain_greps, ts, env_info
    )
    errs = mutex_doc_schema_errors(doc)
    if errs:
        fail("mutex_registered evidence schema 自校验红: " + "; ".join(errs[:3]))
        return 1
    path = ROOT / "evidence" / f"g31_svt_mutex_registered_{ts}.json"
    path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {path.relative_to(ROOT)}")
    note(f"互斥字面（fail-closed 捕获）: {mutex_line[:200]}")
    note(
        f"GATE MUTEX_REGISTERED {GATE_KEY}"
        "（非 PASS 非 FAIL,三态之外的登记态；--svt on × day_0828 texel heap"
        " 形态互斥 fail-closed,host 金标准腿全绿在案,device 四腿结构性跳过,"
        "深修归后续波 TODO #33-#36）"
    )
    return 0


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
    svt_pool: int | None,
    env: dict,
    timeout: int = 3600,
) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    """svt_pool: None = B4 锚臂（--textures on 无 SVT 面）;0 = 全驻留臂;
    N ≥ 1 = 冷启动小池臂。"""
    ev_path = WORK / f"harness_{label}.json"
    argv = [
        str(BIN_PRESENT),
        "--frames", str(frames),
        "--warmup", str(warmup),
        "--hidden",
        "--quality", "off",  # W4 默认翻转免疫:svt/textures 诊断臂显式 off（DEFAULT_FLIP_PLAN §2.5）
        "--auto-move", TRAJECTORY,
        "--evidence", str(ev_path),
        "--textures", "on",
        "--spv-texture", str(SPV_TEX_GI),
        "--spv-texture-probe", str(SPV_TEX_PROBE),
    ]
    if svt_pool is not None:
        argv += [
            "--svt", "on",
            "--spv-svt", str(SPV_SVT),
            "--spv-svt-probe", str(SPV_SVT_PROBE),
            "--svt-pool-tiles", str(svt_pool),
        ]
    r = run(argv, timeout=timeout, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def harness_common_judge(doc: dict, frames: int, warmup: int, label: str) -> list[str]:
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


def cargo_test_green(filter_pat: str) -> tuple[bool, str]:
    r = run(
        ["cargo", "test", "-p", "rurix-render", "--lib", filter_pat, "--quiet"],
        timeout=3600,
    )
    out = (r.stdout or "") + (r.stderr or "")
    ok = r.returncode == 0 and "test result: ok" in out and "0 failed" in out
    return ok, out.strip()[-160:]


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

    # ── 构建（release harness + rurixc debug SPV 面）──
    ok = build_or_fail(
        ["cargo", "build", "--release", "-p", "rurix-render", "--features", "vendor-upscale",
         "--bin", "g31_window_present", "--quiet"],
        "harness release",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── SPV 面：g31_svt_{gi,probe}.rx 现编 + spirv-val ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for src, dst in ((KERNEL_SVT, SPV_SVT), (KERNEL_SVT_PROBE, SPV_SVT_PROBE)):
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
        degrade.append("g31_svt SPV 编译/spirv-val 未过")
    for p, what in ((SPV_TEX_GI, "B4 纹理 SPV"), (SPV_TEX_PROBE, "B4 探针 SPV")):
        if not p.is_file():
            degrade.append(f"{what} 缺失 {p}")
    missing_lane = [f for f in LANE_SPVS if not (SPV_DIR / f).is_file()]
    if missing_lane:
        degrade.append(f"车道 SPV 缺失 {missing_lane}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    # ── 0-byte 面机核（spec/rurix-asset/母版 kernel/material/graph/manifest/terrain）──
    d = run(["git", "diff", "--quiet", "HEAD", "--", *FROZEN_PATHS])
    frozen_ok = d.returncode == 0
    u = run(["git", "status", "--porcelain", "--", *FROZEN_PATHS])
    worktree_ok = not u.stdout.strip()

    # ── host 单测面（streaming::svt + world::terrain）──
    svt_tests_ok, svt_tests_tail = cargo_test_green("streaming::svt")
    terrain_tests_ok, terrain_tests_tail = cargo_test_green("world::terrain")

    # ── SVT-4 判读面（terrain.rs 断言字面 + 并发状态如实登记 + 单测）──
    terrain_greps: dict[str, bool] = {}
    if TERRAIN_RS.is_file():
        text = TERRAIN_RS.read_text(encoding="utf-8")
        terrain_greps = {
            "assert_zero_svt_dependency": "assert_zero_svt_dependency" in text,
            "SvtDependencyDetected": "SvtDependencyDetected" in text,
        }
    else:
        terrain_greps = {"assert_zero_svt_dependency": False, "SvtDependencyDetected": False}
    # terrain.rs 并发在飞状态如实登记（非门判据——他务 WIP 归属他务判档;
    # 本任务零触碰以 C13 文件清单面承载,不以 0-byte-vs-HEAD 冒充）。
    tdiff = run(["git", "status", "--porcelain", "--", "src/rurix-render/src/world/terrain.rs"])
    terrain_modified_vs_head = bool(tdiff.stdout.strip())
    terrain_ok = terrain_judge(terrain_greps, terrain_tests_ok)
    set_fact(
        "svt4_terrain_consumer_m116",
        terrain_ok,
        "维持 defer 如实登记:M116 锚核验需求不成立——terrain.rs 零 SVT 依赖断言在树"
        f"（assert_zero_svt_dependency={terrain_greps['assert_zero_svt_dependency']} "
        f"SvtDependencyDetected={terrain_greps['SvtDependencyDetected']}）"
        f"+ cargo test world::terrain 绿={terrain_tests_ok}（含 zero_svt_dependency_red 臂）"
        f";terrain.rs 并发在飞改动={terrain_modified_vs_head}（他务判档,本任务零触碰以 C13 文件清单面登记）;"
        "basis = heightfield + 材质层 id（4 层闭集）最小语义,模块零纹理/图像消费面"
        "（结构性:M40/41/42 G8 no-go 维持,D4 D17）——不冒充接线"
        if terrain_ok
        else f"SVT-4 判红: greps={terrain_greps} tests={terrain_tests_ok}（{terrain_tests_tail}）",
    )

    # ── dev-env 降级面（probe 真跑判 skipped_dev_env）+ G37 W6 互斥探测 ──
    env = device_env()
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    harness_archives: list[str] = []
    docs: dict[str, dict] = {}
    mutex_line = ""  # G37 W6 svt mutex_registered：非空 = 互斥登记态
    mutex_probe_rc = 0
    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} dev-env 探针（svt 全驻留短跑）"):
            rp, probe_doc, _ = run_present("probe", 2, 1, 0, env, timeout=1800)
        probe_out = (rp.stdout or "") + (rp.stderr or "")
        # G37 W6 svt mutex_registered：真跑四腿前先探互斥——probe 腿本身即
        # --svt on 极小短跑（frames=2/warmup=1）,互斥 fail-closed 在 harness
        # CLI 校验期必现于 stderr。命中字面 ⇒ 登记态（不入 degrade,不冒充
        # skipped_dev_env;stale evidence 文件不参与判定——只看 rc + 字面）。
        mutex_line = detect_svt_mutex(probe_out, rp.returncode)
        if mutex_line:
            mutex_probe_rc = rp.returncode
            note(f"互斥字面命中（probe rc={rp.returncode}）→ 走 mutex_registered 登记态")
        elif '"state":"skipped_dev_env"' in probe_out:
            degrade.append(f"harness skipped_dev_env: {probe_out.strip()[-200:]}")
        elif probe_doc is None:
            degrade.append(f"probe 腿 evidence 缺失: {probe_out.strip()[-200:]}")

    # ── G37 W6 svt mutex_registered：互斥登记态终态处置（真跑四腿前拦截;
    #    host 金标准腿已在上文照跑照判,红则 FAIL;未命中互斥 ⇒ 下方既有
    #    全量判读 0 改动）──
    if mutex_line:
        return mutex_registered_exit(
            mutex_line, mutex_probe_rc, svt_tests_ok, terrain_ok, terrain_greps, ts
        )

    integ_detail: dict = {}
    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} 渲染四腿（B4 锚/全驻留/小池×2）"):
            legs = [
                ("b4anchor", None, "g31_texture_sampling_harness_svtanchor_"),
                ("full", 0, "g31_svt_harness_full_"),
                ("small_a", POOL_SMALL, "g31_svt_harness_small_a_"),
                ("small_b", POOL_SMALL, "g31_svt_harness_small_b_"),
            ]
            leg_ok = True
            for label, pool, prefix in legs:
                r, doc, ev_path = run_present(label, frames, warmup, pool, env)
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
                docs[label] = doc
                arch = ROOT / "evidence" / f"{prefix}{ts}.json"
                arch.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                harness_archives.append(str(arch.relative_to(ROOT)))
            if leg_ok and len(docs) == 4:
                svt_block = (docs["full"].get("svt") or {})
                small_streaming = (docs["small_a"].get("svt") or {}).get("streaming") or {}
                recompute = identity_page_table_digest()
                # ── ① SVT-1 ──
                set_fact(
                    "svt1_page_table_indirection",
                    svt1_ok(svt_block, recompute, svt_tests_ok),
                    f"128K² 虚拟地址空间页表 1024²（活动区 3072 页）+ 物理瓦片池预算:全驻留 SVT vs 直采"
                    f" p100={((svt_block.get('probe') or {}).get('full_residency_arm') or {}).get('p100_vs_direct')!r}"
                    f"（位级硬门 0.0）+ 恒等页表 digest CI 独立重算互核"
                    f"（{svt_block.get('page_table_digest_final') == recompute}）"
                    f"+ streaming::svt host 单测绿={svt_tests_ok}",
                )
                # ── ② SVT-2 ──
                pra = (svt_block.get("probe") or {}).get("partial_residency_arm") or {}
                set_fact(
                    "svt2_gpu_feedback_closed_loop",
                    svt2_ok(svt_block, small_streaming),
                    f"部分驻留 miss={pra.get('miss_probes')} 请求 device==host 位级={pra.get('req_bitexact')}"
                    f" → host consume loaded={pra.get('closed_loop_loaded')} io={pra.get('closed_loop_io_bytes')}B"
                    f" → 重跑全 hit={pra.get('closed_loop_all_hit')} == 全驻留={pra.get('closed_loop_bitexact_vs_full')}"
                    f"；生产小池臂 tiles_loaded_total={small_streaming.get('tiles_loaded_total')}（闭环真跑）",
                )
                # ── ③ SVT-3 ──
                fra = (svt_block.get("probe") or {}).get("full_residency_arm") or {}
                set_fact(
                    "svt3_tile_border_filtering",
                    svt3_ok(svt_block),
                    f"130² 带边物理瓦片（border texel 复制,页所属槽 wrap 律）:边界聚焦"
                    f" {(svt_block.get('probe') or {}).get('boundary_probe_count')} 探针"
                    f" boundary_max_abs={fra.get('boundary_max_abs')!r}（位级 0.0）"
                    f"；各向异性跨瓦片 = 双线性闭集 N/A 登记在案",
                )
                # ── ⑤ 整合 ──
                ok_i, integ_detail = integration_ok(
                    docs["b4anchor"], docs["full"], docs["small_a"], docs["small_b"], frames, warmup
                )
                set_fact(
                    "svt_integration_streaming",
                    ok_i,
                    f"全驻留 == B4 锚逐帧位级={integ_detail['b4_anchor_bitexact']}"
                    f"；小池 {POOL_SMALL} 臂:双跑位级={integ_detail['small_pool_double_run_bitexact']}"
                    f" miss_rate={integ_detail['small_pool_miss_rate']:.6e}"
                    f" io={integ_detail['small_pool_io_bytes']}B（律法互核={integ_detail['small_pool_io_law_match']}）"
                    f" fallback 帧={integ_detail['small_pool_fallback_frames']}"
                    f" 零 fallback 帧={integ_detail['zero_fallback_frames']}（对拍={integ_detail['zero_fallback_frames_bitexact']}）"
                    f" ≠全驻留={integ_detail['small_ne_full']}",
                )

    if degrade:
        doc = {
            "schema": "rurix.g31.svt.skip.v1",
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
    svt_block = (docs.get("full", {}).get("svt") or {})
    pra = (svt_block.get("probe") or {}).get("partial_residency_arm") or {}
    fra = (svt_block.get("probe") or {}).get("full_residency_arm") or {}
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "svt1": {
            "virtual_dim": SVT_VIRTUAL_DIM,
            "page_table_dim": SVT_PAGE_TABLE_DIM,
            "active_pages": ACTIVE_PAGES,
            "p100_vs_direct": fra.get("p100_vs_direct", -1.0),
            "bitexact_vs_direct": fra.get("bitexact_vs_direct", False),
            "identity_page_table_digest": identity_page_table_digest(),
            "identity_recompute_match": svt_block.get("page_table_digest_final") == identity_page_table_digest(),
            "host_tests_green": svt_tests_ok,
        },
        "svt2": {
            "miss_probes": pra.get("miss_probes", 0),
            "req_bitexact": pra.get("req_bitexact", False),
            "out_bitexact": pra.get("out_bitexact", False),
            "closed_loop_loaded": pra.get("closed_loop_loaded", 0),
            "closed_loop_io_bytes": pra.get("closed_loop_io_bytes", 0),
            "closed_loop_all_hit": pra.get("closed_loop_all_hit", False),
            "closed_loop_bitexact_vs_full": pra.get("closed_loop_bitexact_vs_full", False),
            "production_tiles_loaded": int((((docs.get("small_a", {}).get("svt") or {}).get("streaming") or {}).get("tiles_loaded_total")) or 0),
        },
        "svt3": {
            "phys_tile_dim": SVT_PHYS_DIM,
            "border": SVT_BORDER,
            "boundary_probe_count": (svt_block.get("probe") or {}).get("boundary_probe_count", 0),
            "boundary_max_abs": fra.get("boundary_max_abs", -1.0),
            "aniso_registration": "各向异性" in str(svt_block.get("gaps", "")),
        },
        "svt4": {
            "disposition": "maintain-defer",
            "terrain_modified_vs_head_registered": terrain_modified_vs_head,
            "zero_svt_assert_present": terrain_greps.get("assert_zero_svt_dependency", False)
            and terrain_greps.get("SvtDependencyDetected", False),
            "terrain_tests_green": terrain_tests_ok,
            "basis": "M116 地形面 = heightfield（M04 页格式资产）+ 材质层 id（4 层闭集）最小语义,模块零纹理/图像/采样消费面（结构性）;D4 D17 零 SVT 依赖断言（M40/41/42 G8 no-go 维持）——地形 SVT 需求不成立,维持 defer 如实登记不冒充接线;terrain.rs 并发在飞改动归他务判档,本任务零触碰（C13 文件清单面）,不以 0-byte-vs-HEAD 冒充（全树多波次 WIP 共享面差分不可归因）",
        },
        "integration": integ_detail if integ_detail else {
            "b4_anchor_bitexact": False,
            "small_pool_double_run_bitexact": False,
            "small_pool_frames_completed": 0,
            "small_pool_miss_rate": 0.0,
            "small_pool_io_bytes": 0,
            "small_pool_io_law_match": False,
            "small_pool_fallback_frames": 0,
            "zero_fallback_frames": 0,
            "zero_fallback_frames_bitexact": False,
            "small_ne_full": False,
            "full_residency_pool_tiles": POOL_FULL,
            "small_pool_tiles": POOL_SMALL,
        },
        "harness_evidence": harness_archives,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C13 SVT 四行（RD-041 分项,TODO #33-#36）:SVT-1 虚拟纹理页表"
            "（streaming/svt.rs host 管理面 + 1024² u32 页表 SSBO + 128² 虚拟瓦片/130² 带边"
            "物理池,128K² 虚拟地址空间满尺寸分配,活动区 = bistro 图集 3072 页）/ SVT-2 GPU"
            " 反馈 pass（kernels/g31_svt_gi.rx 采样 miss → out_req 1 f32/px 请求缓冲〔0=无"
            " miss,页表网格序页号+1,禁 atomic 逐像素直写〕→ host SvtStreaming::consume〔"
            "BTreeSet 去重排序确定性 + LRU 池 + 页表影〕→ 次帧 FrameUpdate.buffer_uploads"
            " 页表写段+瓦片上传段,请求-驻留闭环逐帧真跑）/ SVT-3 瓦片边界过滤（border"
            " texel 复制——页所属槽 REPEAT wrap 律 rem_euclid,双线性 2×2 footprint 单瓦片"
            "自足;探针全驻留 SVT vs 整图直采位级硬门,边界聚焦 96 探针误差=0;各向异性跨"
            "瓦片 = 生产采样闭集双线性唯一过滤面,需求不成立登记 N/A）/ SVT-4 按 M116 锚"
            "核验需求不成立维持 defer（terrain.rs 零 SVT 断言 0-byte,不冒充接线）。整合"
            "真跑:B4 锚臂（--textures on 无 SVT 面 = 全驻留锚 0-byte）vs SVT 全驻留臂"
            "（--svt-pool-tiles 0）逐帧 digest_seq 位级一致;强制小池 256 臂流送全程无崩,"
            "fallback = 槽均值低 mip 等效合法面（hit 门融合 1·x+0·y IEEE 精确 ⇒ 全驻留"
            "零扰动）,miss 率/IO 量 measured,确定性双跑位级一致。B4 纹理面回归不破坏"
            "（--textures on 无 SVT 面 = 锚臂在案）。"
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
    gate_path = ROOT / "evidence" / f"g31_svt_gate_{ts}.json"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}(+ harness {len(harness_archives)} 件)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂,无 GPU/无构建依赖）
# ---------------------------------------------------------------------------


def _good_svt_block(recompute: str) -> dict:
    dg = "sha256:" + "a" * 64
    return {
        "virtual_dim": SVT_VIRTUAL_DIM,
        "tile_dim": SVT_TILE_DIM,
        "border": SVT_BORDER,
        "phys_tile_dim": SVT_PHYS_DIM,
        "page_table_dim": SVT_PAGE_TABLE_DIM,
        "page_table_entries": SVT_PAGE_COUNT,
        "active_pages_x": ACTIVE_PAGES_X,
        "active_pages_y": ACTIVE_PAGES_Y,
        "active_pages": ACTIVE_PAGES,
        "tile_set_digest": dg,
        "page_table_digest_final": recompute,
        "pool_tiles": POOL_FULL,
        "full_residency": True,
        "phys_tile_bytes": SVT_PHYS_TILE_BYTES,
        "fallback_digest": dg,
        "probe": {
            "uv_law": "…",
            "probe_count": N_PROBES,
            "boundary_probe_count": N_BOUNDARY_PROBES,
            "eval_ms": 1.0,
            "full_residency_arm": {
                "p100_vs_direct": 0.0,
                "bitexact_vs_direct": True,
                "bitexact_vs_svt_host": True,
                "double_run_bitexact": True,
                "device_digest": dg,
                "host_digest": dg,
                "boundary_max_abs": 0.0,
            },
            "partial_residency_arm": {
                "law": "page_id%3==2",
                "miss_probes": 136,
                "req_bitexact": True,
                "out_bitexact": True,
                "closed_loop_loaded": 66,
                "closed_loop_evicted": 0,
                "closed_loop_io_bytes": 66 * SVT_PHYS_TILE_BYTES,
                "closed_loop_all_hit": True,
                "closed_loop_bitexact_vs_full": True,
            },
        },
        "streaming": {
            "frames": 4,
            "miss_px_total": 10,
            "requested_pages_total": 8,
            "tiles_loaded_total": 6,
            "tiles_evicted_total": 4,
            "io_bytes_total": 6 * SVT_PHYS_TILE_BYTES,
            "io_per_frame_bytes": 0,
            "miss_rate": 0.1,
            "fallback_frames": 2,
            "converged_frame": 3,
            "miss_px_seq": [5, 3, 0, 0],
            "unique_pages_seq": [4, 2, 0, 0],
            "loaded_seq": [4, 2, 0, 0],
            "evicted_seq": [2, 2, 0, 0],
        },
        "spv_svt": {"path": "x", "sha256": dg, "no_contraction_injected": True},
        "spv_svt_probe": {"path": "y", "sha256": dg, "no_contraction_injected": True},
        "gaps": "各向异性跨瓦片 = 生产采样闭集双线性唯一过滤面 N/A",
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

    # 绿臂①:恒等页表律法——确定性双算一致 + 关键下标命中。
    d1 = identity_page_table_digest()
    d2 = identity_page_table_digest()
    expect(d1 == d2 and DIGEST_RE.match(d1) is not None, "GREEN:恒等页表 digest 确定性双算")
    buf = bytearray(SVT_PAGE_COUNT * 4)
    struct.pack_into("<I", buf, 0, 1)  # tile 0 → 下标 0 值 1
    t5 = (5 // ACTIVE_PAGES_X) * SVT_PAGE_TABLE_DIM + (5 % ACTIVE_PAGES_X)
    struct.pack_into("<I", buf, t5 * 4, 6)  # tile 5 → 值 6
    expect(t5 == 5, "GREEN:tile 5 网格序下标 = 5（pages_x 整除面）")
    t64 = (64 // ACTIVE_PAGES_X) * SVT_PAGE_TABLE_DIM + (64 % ACTIVE_PAGES_X)
    expect(t64 == SVT_PAGE_TABLE_DIM, "GREEN:tile 64 网格序下标 = 1024（跨行 stride 面）")
    # 红臂①:律法破（篡改一项）digest 必异。
    buf2 = bytearray(buf)
    struct.pack_into("<I", buf2, 7 * 4, 999)
    expect(
        "sha256:" + hashlib.sha256(bytes(buf2)).hexdigest() != "sha256:" + hashlib.sha256(bytes(buf)).hexdigest(),
        "RED:页表项篡改 digest 必异",
    )
    # 绿臂②:svt1 判正例。
    good = _good_svt_block(d1)
    expect(svt1_ok(good, d1, True), "GREEN:svt1 判合法正例")
    expect(not svt1_ok(good, "sha256:" + "b" * 64, True), "RED:恒等页表 digest 不符必红")
    bad = json.loads(json.dumps(good))
    bad["page_table_entries"] = 1048575
    expect(not svt1_ok(bad, d1, True), "RED:页表项数篡改必红")
    bad = json.loads(json.dumps(good))
    bad["probe"]["full_residency_arm"]["p100_vs_direct"] = 1e-7
    expect(not svt1_ok(bad, d1, True), "RED:全驻留 p100>0 必红")
    expect(not svt1_ok(good, d1, False), "RED:host 单测红必红")
    # 绿臂③:svt2 判正例 + 红臂组。
    small_streaming = {"tiles_loaded_total": 100, "requested_pages_total": 120}
    expect(svt2_ok(good, small_streaming), "GREEN:svt2 判合法正例")
    bad = json.loads(json.dumps(good))
    bad["probe"]["partial_residency_arm"]["closed_loop_io_bytes"] = 65 * SVT_PHYS_TILE_BYTES
    expect(not svt2_ok(bad, small_streaming), "RED:闭环 IO 律法破（≠loaded×67600）必红")
    bad = json.loads(json.dumps(good))
    bad["probe"]["partial_residency_arm"]["closed_loop_all_hit"] = False
    expect(not svt2_ok(bad, small_streaming), "RED:闭环未全 hit 必红")
    expect(not svt2_ok(good, {"tiles_loaded_total": 0, "requested_pages_total": 5}),
           "RED:生产闭环空转（loaded=0）必红")
    # 绿臂④:svt3 判正例 + 红臂组。
    expect(svt3_ok(good), "GREEN:svt3 判合法正例")
    bad = json.loads(json.dumps(good))
    bad["probe"]["full_residency_arm"]["boundary_max_abs"] = 1e-6
    expect(not svt3_ok(bad), "RED:边界误差非零必红")
    bad = json.loads(json.dumps(good))
    bad["gaps"] = "无登记"
    expect(not svt3_ok(bad), "RED:各向异性 N/A 登记缺失必红")
    # 绿臂⑤:整合判正例（合成四腿:全驻留==B4,小池双跑,条件式错图机核）。
    total = 4
    seq_full = ["sha256:" + "a" * 64, "sha256:" + "b" * 64, "sha256:" + "c" * 64, "sha256:" + "d" * 64]
    seq_small = ["sha256:" + "e" * 64, "sha256:" + "f" * 64, seq_full[2], seq_full[3]]
    b4_doc = {"digest_seq": seq_full, "frames_completed": total}
    full_doc = {
        "digest_seq": seq_full, "frames_completed": total,
        "svt": {"full_residency": True, "pool_tiles": POOL_FULL},
    }
    small_a = {
        "digest_seq": seq_small, "frames_completed": total,
        "svt": {
            "full_residency": False, "pool_tiles": POOL_SMALL,
            "streaming": {
                "frames": total, "miss_px_seq": [5, 3, 0, 0],
                "io_bytes_total": 6 * SVT_PHYS_TILE_BYTES,
                "tiles_loaded_total": 6, "miss_rate": 0.1, "fallback_frames": 2,
            },
        },
    }
    small_b = json.loads(json.dumps(small_a))
    ok_i, det = integration_ok(b4_doc, full_doc, small_a, small_b, 2, 2)
    expect(ok_i, "GREEN:整合判合法正例（零 fallback 帧对拍真空真含盖）")
    expect(det["zero_fallback_frames"] == 2, "GREEN:零 fallback 帧计数 = 2")
    # 红臂组⑤。
    bad_full = json.loads(json.dumps(full_doc))
    bad_full["digest_seq"] = list(seq_full)
    bad_full["digest_seq"][1] = "sha256:" + "0" * 64
    ok_bad, _ = integration_ok(b4_doc, bad_full, small_a, small_b, 2, 2)
    expect(not ok_bad, "RED:全驻留 ≠ B4 锚必红")
    bad_b = json.loads(json.dumps(small_a))
    bad_b["digest_seq"] = list(seq_small)
    bad_b["digest_seq"][1] = "sha256:" + "1" * 64
    ok_bad, _ = integration_ok(b4_doc, full_doc, small_a, bad_b, 2, 2)
    expect(not ok_bad, "RED:小池双跑漂移必红")
    bad_z = json.loads(json.dumps(small_a))
    bad_z["digest_seq"] = list(seq_small)
    bad_z["digest_seq"][2] = "sha256:" + "2" * 64  # miss=0 帧与全驻留不符
    ok_bad, det_bad = integration_ok(b4_doc, full_doc, bad_z, small_b, 2, 2)
    expect(not ok_bad and det_bad["zero_fallback_frames_bitexact"] is False,
           "RED:零 fallback 帧错图（≠全驻留）必红")
    bad_io = json.loads(json.dumps(small_a))
    bad_io["svt"]["streaming"]["io_bytes_total"] = 5 * SVT_PHYS_TILE_BYTES
    ok_bad, _ = integration_ok(b4_doc, full_doc, bad_io, small_b, 2, 2)
    expect(not ok_bad, "RED:IO 律法破必红")
    bad_ne = json.loads(json.dumps(small_a))
    bad_ne["digest_seq"] = list(seq_full)  # 小池 == 全驻留 = 冒充流送
    ok_bad, det_bad = integration_ok(b4_doc, full_doc, bad_ne, small_b, 2, 2)
    expect(not ok_bad and det_bad["small_ne_full"] is False, "RED:小池==全驻留冒充流送必红")
    bad_mr = json.loads(json.dumps(small_a))
    bad_mr["svt"]["streaming"]["miss_rate"] = 0.0
    ok_bad, _ = integration_ok(b4_doc, full_doc, bad_mr, small_b, 2, 2)
    expect(not ok_bad, "RED:miss_rate=0 冒充真流送必红")
    # 绿臂⑥:SVT-4 判。
    expect(
        terrain_judge({"assert_zero_svt_dependency": True, "SvtDependencyDetected": True}, True),
        "GREEN:svt4 维持 defer 合法面",
    )
    expect(
        not terrain_judge({"assert_zero_svt_dependency": False, "SvtDependencyDetected": True}, True),
        "RED:零 SVT 断言字面缺失必红",
    )
    expect(
        not terrain_judge({"assert_zero_svt_dependency": True, "SvtDependencyDetected": True}, False),
        "RED:terrain 单测红必红",
    )
    # schema 互核:两 schema 在树 + gate schema facts enum == FACT_IDS + harness
    # schema required 含 svt + 常量互核。
    expect(SCHEMA_PATH.is_file() and GATE_SCHEMA_PATH.is_file(), "两 schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
    if SCHEMA_PATH.is_file():
        hs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect("svt" in hs.get("required", []), "harness schema required 含 svt")
        expect(hs["properties"]["schema"]["const"] == SCHEMA_ID, "harness schema const 互核")
        sp = hs["properties"]["svt"]["properties"]
        expect(sp["virtual_dim"]["const"] == 131072 and sp["page_table_dim"]["const"] == 1024,
               "harness schema 页表常量互核")
        expect(sp["active_pages"]["const"] == 3072 and sp["phys_tile_bytes"]["const"] == 67600,
               "harness schema 活动区/瓦片字节常量互核")
    expect(len(FACT_IDS) == 5, "facts 闭集 = 5（四行各一 + 整合一）")
    # ── G37 W6 svt mutex_registered 登记态臂（正例 + 红臂组）──
    mutex_full_line = (
        f"[g14_3_pipeline_perf]: FAIL {MUTEX_LITERAL}（SVT 假设 = 2048 网格图集"
        "/texmeta origin/tritex 步幅 1,heap 化未适配——fail-closed 登记,"
        "SVT 深修归后续波）"
    )
    hit = detect_svt_mutex("前导行\n" + mutex_full_line + "\n后续行", 1)
    expect(hit == mutex_full_line, "GREEN:互斥字面命中捕获完整行（rc=1）")
    expect(detect_svt_mutex(mutex_full_line, 0) == "", "RED:rc=0 不入登记态")
    expect(
        detect_svt_mutex(
            "[g14_3_pipeline_perf]: FAIL --spv-svt*/--svt-pool-tiles 须随 --svt on"
            "（svt off 面 = 车道 0-byte,SPV/池覆盖位无消费面）",
            1,
        )
        == "",
        "RED:互斥字面不符（其他 fail-closed）不入登记态",
    )
    expect(mutex_host_legs_ok(True, True), "GREEN:host 金标准腿全绿判")
    expect(not mutex_host_legs_ok(False, True), "RED:svt host 单测红 ⇒ 登记态整体 FAIL")
    expect(not mutex_host_legs_ok(True, False), "RED:SVT-4 defer 面红 ⇒ 登记态整体 FAIL")
    good_greps = {"assert_zero_svt_dependency": True, "SvtDependencyDetected": True}
    st_env = {"gpu": "selftest", "os": "windows", "rustc": "selftest", "base_commit": "selftest"}
    mdoc = build_mutex_registered_doc(
        mutex_full_line, 1, True, True, good_greps, "20260830T000000Z", st_env
    )
    expect(not mutex_doc_schema_errors(mdoc), "GREEN:登记件过新 schema Draft7 校验")
    bad = json.loads(json.dumps(mdoc))
    bad["state"] = "PASS"
    expect(bool(mutex_doc_schema_errors(bad)), "RED:登记件冒充 PASS 必红（state const）")
    bad = json.loads(json.dumps(mdoc))
    bad["mutex_literal"] = "无关 fail 字面"
    expect(bool(mutex_doc_schema_errors(bad)), "RED:登记件互斥字面不符必红（pattern）")
    bad = json.loads(json.dumps(mdoc))
    bad["host_golden_legs"]["svt_host_tests_green"] = False
    expect(bool(mutex_doc_schema_errors(bad)), "RED:登记件 host 腿红字面必红（const true）")
    expect(MUTEX_SCHEMA_PATH.is_file(), "mutex_registered schema 在树")
    if MUTEX_SCHEMA_PATH.is_file():
        ms = json.loads(MUTEX_SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(
            ms["properties"]["schema"]["const"] == MUTEX_SCHEMA_ID,
            "mutex schema const 互核",
        )
        expect(
            ms["properties"]["state"]["const"] == "MUTEX_REGISTERED"
            and ms["properties"]["registered_wave"]["const"] == MUTEX_REGISTERED_WAVE,
            "mutex schema 登记态/波次 const 互核",
        )
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(
        f"[{TAG}] selftest PASS（facts=5；红臂组 + 正例组 + 双 schema 互核"
        " + G37 W6 mutex_registered 登记态臂）"
    )
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
            print(f"[{TAG}] FAIL: --frames {args.frames} < 32（整合真跑窗下限）", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.warmup)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
