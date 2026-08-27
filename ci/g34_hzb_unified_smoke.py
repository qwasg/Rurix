#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude（G34 全特性合流收口批 G34-2）
"""G34-2：HZB 接统一车道门冒烟（g34.wave2.hzb；G31 波 B Task B1 生产接线面
逐字同律 + G34 统一车道合流——剔除对象粒度 = TLAS 实例〔bistro 逐 mesh 节点
BLAS 分解 + 动态实例尾槽恒可见不参剔〕，消费点 = 主射线 pass 的 TLAS 实例
mask〔kernels/g34_unified_primary.rx 相机射线走初剔后表 0，g34_unified_shade
阴影射线走全量表 1——被剔实例仍投阴影〕，双 TLAS 逐帧 refit + 帧内金字塔
轮换〔上帧金字塔初剔 p1 → 本帧真深度 out_depth_hz 重建 → 上帧被剔集重测
p2〕+ RFC-0044 §5.8 两阶段闭环第二段〔≤4 迭代未收敛全掩码兜底 = 零剔除精确
收敛〕；harness = target/release/g34_full_lane --hzb on，HZB 段全量收
src/rurix-render/src/bin/g34_full_lane/g34_2_hzb.rs 独立 include 区段）。

九类判据（判据面 = ci/g31_hzb_wiring_smoke.py HZB 族先例逐字同律；蒸馏为
六 facts 闭集，标注承载 fact）：
1. **统一/冻结 kernel SPV 面**：rurixc 现编五件（g34_unified_primary〔G34-2
   加性主射线〕/g34_unified_shade〔④b 段 out_depth_hz〕/g31_hzb_pack〔平铺
   打包 glue〕/g27_hzb_reduce/g27_hzb_test〔G27 M-a 本体冻结消费〕）+
   spirv-val 全绿（fact kernels_spv_valid）。
2. **冻结 tracked 0-byte 机核**：g27 双 kernel（g27_hzb_reduce/g27_hzb_test
   .rx）vs HEAD 提交面 + 工作树 0-byte（fact kernels_spv_valid；
   g31_hzb_pack.rx 实为 G31 期 untracked 冻结源——diff-vs-HEAD 不适用，
   归第 3 类快照面承载，如实登记不冒充）。
3. **untracked 冻结面 sha256 快照**：G31/G34 期未提交七面（g34_unified_gi/
   g34_unified_shade/g34_unified_primary/g31_hzb_pack 四 kernel +
   g34_2_hzb.rs/g34_full_lane.rs/g31_window_present.rs）快照在档 = 后续波
   漂移守护基线（fact kernels_spv_valid）。
4. **剔除像素中性**：hzb_a vs RURIX_HZB_ALL_VISIBLE=1 全集渲染实验臂
   digest_seq 逐帧位级一致（剔除不改变可见像素——两阶段闭环正确性的结构
   判据；fact culling_pixel_neutral）。
5. **host 金标准对拍**：probe 帧车道平铺金字塔 vs HzbPyramid::build 逐级
   位级 mips_bitexact + p1 判定序列 vs test_rect 逐字节 verdict_equal +
   exact_rect_occluded 独立复核零假阳性 false_positives=0（geometry/
   {hzb,cull}.rs 冻结面只读消费；fact hzb_host_parity）。
6. **确定性双跑**：hzb_a vs hzb_b digest_seq 逐帧位级 + render_digest 一致
   （fact determinism_double_run）。
7. **剔除真实发生**：occluded_p1 合计 ≥1（零剔除即空接线冒充判红）+
   tested/flipped_p2/closure 计数如实登记（fact culling_effective_measured）。
8. **frame_ms 对照**：同机同窗 orbit --hidden 1920×1080 release 真跑 hzb on
   vs baseline（--full 无 --hzb 统一车道腿）real_render_frame_ms 如实登记
   **不设通过线**（G6 无硬门纪律，measured_local；fact
   culling_effective_measured）。
9. **Stage A 锚复跑零漂移**：g14_3_pipeline_perf canonical 160 帧
   bistro-interior/t100/tsr_device 末帧 digest == 在案锚——共享体加性扩展
   对既有面 0-byte 的机器证明（fact stage_a_anchor_replay）。

三态：无 Vulkan loader/设备/场景资产/SPV 编译失败 → DEV_ENV_DEGRADE 退 0
（不冒充 PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充
真跑）。

evidence 纪律：门 schema PASS-only 闭集——PASS 才落 evidence/
g34_hzb_unified_gate_<ts>.json（check_schemas 前缀路由 g34_hzb_unified_gate_
仅门裁决件）；FAIL 诊断件落 .tmp/g34_gates/hzb/ 工作区不污染 evidence/ 路由
面（fail-closed：evidence/ 无件 = 门未过，不冒充）。HZB 腿 harness 真跑件
（rurix.g34.hzb_unified_evidence.v1 字面）留 .tmp——harness 真跑件不注册
check_schemas,数字经门裁决件蒸馏登记；baseline 对照腿归档用
g34_unified_lane_g34hzb_ 前缀复用 g34_unified_lane_ 既有路由。

用法：
  py -3 ci/g34_hzb_unified_smoke.py --selftest
  py -3 ci/g34_hzb_unified_smoke.py --gate g34.wave2.hzb [--frames 64] [--warmup 10]
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

GATE_KEY = "g34.wave2.hzb"
SUBJECT = "g34_hzb_unified"
WAVE = "G34.2"
TAG = "g34_hzb_unified"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g34" / "g34_hzb_unified_gate_evidence_schema.json"
GATE_SCHEMA_ID = "rurix.g34.hzb_unified_gate_evidence.v1"
# harness 真跑件 schema 字面（.tmp 工作区件——不注册 check_schemas，无 schema 文件）。
HARNESS_SCHEMA_ID = "rurix.g34.hzb_unified_evidence.v1"
BASELINE_SCHEMA_ID = "rurix.g34.unified_lane_evidence.v1"
BASELINE_GATE_KEY = "g34.wave1.unified"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_tsr_device"
SLAB_ASSET = ROOT / "milestones" / "g31" / "g31_slab_side_table_bistro_interior.json"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
WORK = ROOT / ".tmp" / "g34_gates" / "hzb"
# 五 kernel 现编面（源 → WORK 内 SPV;g27 两件本体 0-byte 冻结消费——bin 侧
# NoContraction 注入 = mips 位级全等关键,SPV 文件本体 0-byte 不动）。
KERNEL_SPECS = (
    ("g34_unified_primary.rx", "g34_unified_primary.spv"),
    ("g34_unified_shade.rx", "g34_unified_shade.spv"),
    ("g31_hzb_pack.rx", "g31_hzb_pack.spv"),
    ("g27_hzb_reduce.rx", "g27_hzb_reduce.spv"),
    ("g27_hzb_test.rx", "g27_hzb_test.spv"),
)
SPV_PRIMARY = WORK / "g34_unified_primary.spv"
SPV_SHADE = WORK / "g34_unified_shade.spv"
SPV_PACK = WORK / "g31_hzb_pack.spv"
SPV_REDUCE = WORK / "g27_hzb_reduce.spv"
SPV_TEST = WORK / "g27_hzb_test.spv"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
LANE_SPVS = (
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
    "g29_slab.spv",
)
SPV_PROBE = ROOT / ".tmp" / "g31_gates" / "texture" / "g31_texture_probe.spv"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN_FULL = ROOT / "target" / "release" / f"g34_full_lane{EXE_SUFFIX}"
BIN_BENCH = ROOT / "target" / "release" / f"g14_3_pipeline_perf{EXE_SUFFIX}"
MOTHER_TRACKED_PATHS = [
    # HEAD 已入 git 的冻结 kernel——diff vs HEAD 机核 0-byte（G27 M-a 本体）。
    # 注：g31_hzb_pack.rx 实为 G31 期 untracked 冻结源（?? 状态,diff-vs-HEAD
    # 不适用）——0-byte 机核只覆 g27 两件 tracked 面，pack 冻结守护 =
    # FROZEN_SNAPSHOT_PATHS sha256 快照面承载（unified 脚本 MOTHER_G31_ERA_PATHS
    # 同律，如实登记不冒充）。
    "src/rurix-render/kernels/g27_hzb_reduce.rx",
    "src/rurix-render/kernels/g27_hzb_test.rx",
]
FROZEN_SNAPSHOT_PATHS = [
    # G31/G34 期 untracked 面（工作树 untracked;diff-vs-HEAD 不适用）——G34-2
    # sha256 快照 = 后续波漂移守护基线;g34_unified_shade.rx 经 G34-2 生产接线
    # 扩展（fork A 采样块 + inline 阴影 + out_depth_hz）后快照即本波冻结起点。
    "src/rurix-render/kernels/g34_unified_gi.rx",
    "src/rurix-render/kernels/g34_unified_shade.rx",
    "src/rurix-render/kernels/g34_unified_primary.rx",
    "src/rurix-render/kernels/g31_hzb_pack.rx",
    "src/rurix-render/src/bin/g34_full_lane/g34_2_hzb.rs",
    "src/rurix-render/src/bin/g34_full_lane.rs",
    "src/rurix-render/src/bin/g31_window_present.rs",
]
SCENE = "bistro-interior"
TRAJECTORY = "orbit"

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "kernels_spv_valid",
    "culling_pixel_neutral",
    "hzb_host_parity",
    "determinism_double_run",
    "culling_effective_measured",
    "stage_a_anchor_replay",
]

# harness evidence 契约键闭集（与 harness 并行任务约定;容错读取但
# fail-closed——缺键即 FAIL 不静默）。
HZB_LEG_REQUIRED = (
    "schema", "gate", "frames", "warmup", "frames_completed", "exit_reason",
    "digest_seq", "render_digest", "real_render_frame_ms", "present_frame_ms",
    "stats", "hzb", "environment",
)
HZB_BLOCK_REQUIRED = (
    "all_visible_arm", "instances", "mips", "tested", "occluded_p1",
    "flipped_p2", "closure_extra_submits", "closure_full_fallback_frames",
    "parity",
)
PARITY_REQUIRED = (
    "mips", "n_rects", "mips_bitexact", "verdict_equal", "false_positives",
    "occluded", "pyramid_digest", "host_pyramid_digest", "verdict_digest",
    "host_verdict_digest",
)


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
# harness 调用契约（与 harness 并行任务约定;对齐微调只动本函数）
# ---------------------------------------------------------------------------


def leg_evidence_path(label: str) -> Path:
    return WORK / f"leg_{label}.json"


def build_harness_argv(
    label: str,
    frames: int,
    warmup: int,
    hzb_on: bool,
    extra: list[str] | None = None,
) -> list[str]:
    """G34-2 harness 调用契约集中面（字面顺序 = 任务书契约）：

    g34_full_lane --hzb on --full --slab-table <SLAB> --frames N --warmup W
      --auto-move orbit --tier 100 --hidden --spv-hzb-{primary,shade,pack,
      reduce,test} <WORK 五 SPV> --evidence <WORK>/leg_<label>.json

    baseline 腿 = --full 无 --hzb（统一车道腿,rurix.g34.unified_lane_evidence
    .v1 面,供 frame_ms 对照）;allvis 腿同 hzb on 契约,差异只在环境变量
    RURIX_HZB_ALL_VISIBLE=1（调用方注入）。
    """
    argv: list[str] = [str(BIN_FULL)]
    if hzb_on:
        argv += ["--hzb", "on"]
    argv += [
        "--full", "--slab-table", str(SLAB_ASSET),
        "--frames", str(frames), "--warmup", str(warmup),
        "--auto-move", TRAJECTORY, "--tier", "100", "--hidden",
    ]
    if hzb_on:
        argv += [
            "--spv-hzb-primary", str(SPV_PRIMARY),
            "--spv-hzb-shade", str(SPV_SHADE),
            "--spv-hzb-pack", str(SPV_PACK),
            "--spv-hzb-reduce", str(SPV_REDUCE),
            "--spv-hzb-test", str(SPV_TEST),
        ]
    argv += ["--evidence", str(leg_evidence_path(label))]
    if extra:
        argv += extra
    return argv


# ---------------------------------------------------------------------------
# 判读器（selftest 红绿两臂消费面）
# ---------------------------------------------------------------------------


def seqs_bitexact(a: list, b: list) -> bool:
    """同轨迹双臂 digest_seq 逐帧位级一致判据（非空 + 等长 + 逐项全等）。"""
    return len(a) == len(b) and len(a) > 0 and all(x == y for x, y in zip(a, b))


def seq_mismatch_count(a: list, b: list) -> int:
    """逐帧 mismatch 计数（长度不齐 = -1 拒判）。"""
    if len(a) != len(b):
        return -1
    return sum(1 for x, y in zip(a, b) if x != y)


def anchor_match(fresh: str | None, anchor: str | None) -> bool:
    """Stage A 锚格位级判据（fresh == anchor；缺 digest/形态破即红）。"""
    return (
        isinstance(fresh, str) and isinstance(anchor, str)
        and DIGEST_RE.match(fresh) is not None and DIGEST_RE.match(anchor) is not None
        and fresh == anchor
    )


def frame_ms_sane(*vals: float) -> bool:
    """frame_ms 登记面健全判：全有限正数（诚实登记非阈门——G6 无硬门纪律）。"""
    return all(isinstance(v, (int, float)) and not isinstance(v, bool) and v == v and v > 0 for v in vals)


def freeze_0byte(diff_rc: int, porcelain: str) -> bool:
    """冻结 tracked 面 0-byte 机核判：diff vs HEAD 空 + 工作树 porcelain 空。"""
    return diff_rc == 0 and not porcelain.strip()


def _nonneg_int(v) -> bool:
    return isinstance(v, int) and not isinstance(v, bool) and v >= 0


def _pos_int(v) -> bool:
    return isinstance(v, int) and not isinstance(v, bool) and v >= 1


def counts_effective(hz: dict) -> bool:
    """⑦ 剔除真实发生判（hzb_a 生产腿窗口合计;occluded_p1≥1——零剔除即空
    接线冒充硬红,G31 B1 occlusion_culling_active 同律）。"""
    if not isinstance(hz, dict):
        return False
    if not _pos_int(hz.get("occluded_p1")) or not _pos_int(hz.get("tested")):
        return False
    return all(_nonneg_int(hz.get(k)) for k in (
        "flipped_p2", "closure_extra_submits", "closure_full_fallback_frames"))


def parity_judge(parity) -> list[str]:
    """⑤ host 金标准对拍判（hzb.parity 块;返回失败串列表,空 = 绿）：
    mips 逐级位级 + 判定序列逐字节 + 零假阳性三面 + 双 digest 对 vs host
    互核（位级/全等 ⇒ digest 必同——机器交叉验证）。"""
    if not isinstance(parity, dict):
        return ["hzb.parity 非 object"]
    missing = [k for k in PARITY_REQUIRED if k not in parity]
    if missing:
        return [f"hzb.parity 缺键 {missing}（fail-closed 不静默）"]
    fails: list[str] = []
    if parity.get("mips_bitexact") is not True:
        fails.append(f"hzb.parity.mips_bitexact ≠ true: {parity.get('mips_bitexact')!r}")
    if parity.get("verdict_equal") is not True:
        fails.append(f"hzb.parity.verdict_equal ≠ true: {parity.get('verdict_equal')!r}")
    if parity.get("false_positives") != 0:
        fails.append(f"hzb.parity.false_positives ≠ 0: {parity.get('false_positives')!r}")
    if not _pos_int(parity.get("mips")):
        fails.append(f"hzb.parity.mips < 1: {parity.get('mips')!r}")
    if not _pos_int(parity.get("n_rects")):
        fails.append(f"hzb.parity.n_rects < 1: {parity.get('n_rects')!r}")
    if not _nonneg_int(parity.get("occluded")):
        fails.append(f"hzb.parity.occluded 非负整数破: {parity.get('occluded')!r}")
    for k in ("pyramid_digest", "host_pyramid_digest", "verdict_digest", "host_verdict_digest"):
        d = parity.get(k)
        if not isinstance(d, str) or not DIGEST_RE.match(d):
            fails.append(f"hzb.parity.{k} 形态非法: {str(d)[:40]!r}")
    if not fails:
        if parity["pyramid_digest"] != parity["host_pyramid_digest"]:
            fails.append("hzb.parity 金字塔 digest ≠ host（mips_bitexact 旗标与 digest 互核破）")
        if parity["verdict_digest"] != parity["host_verdict_digest"]:
            fails.append("hzb.parity 判定 digest ≠ host（verdict_equal 旗标与 digest 互核破）")
    return fails


def hzb_leg_judge(doc: dict, frames: int, warmup: int, label: str, expect_allvis: bool) -> list[str]:
    """HZB 腿 harness evidence 判（契约键闭集容错读取但 fail-closed——缺键即
    FAIL 不静默;深判 parity/剔除计数归 fact ⑤⑦ 消费 hzb_a）。"""
    fails: list[str] = []
    missing = [k for k in HZB_LEG_REQUIRED if k not in doc]
    if missing:
        return [f"{label}: harness evidence 缺键 {missing}（fail-closed 不静默）"]
    if doc["schema"] != HARNESS_SCHEMA_ID:
        fails.append(f"{label}: schema ≠ {HARNESS_SCHEMA_ID}: {doc['schema']!r}")
    if doc["gate"] != GATE_KEY:
        fails.append(f"{label}: gate ≠ {GATE_KEY}: {doc['gate']!r}")
    total = frames + warmup
    if doc["frames_completed"] != total:
        fails.append(f"{label}: frames_completed {doc['frames_completed']} ≠ {total}")
    if doc["exit_reason"] != "frames_done":
        fails.append(f"{label}: exit_reason ≠ frames_done: {doc['exit_reason']!r}")
    seq = doc["digest_seq"]
    if not isinstance(seq, list) or len(seq) != total or any(
        not isinstance(x, str) or not DIGEST_RE.match(x) for x in seq
    ):
        fails.append(f"{label}: digest_seq 形态/长度破（≠{total}）")
    if not isinstance(doc["render_digest"], str) or not DIGEST_RE.match(doc["render_digest"]):
        fails.append(f"{label}: render_digest 形态破")
    rr = doc["real_render_frame_ms"]
    if not isinstance(rr, (int, float)) or isinstance(rr, bool) or not rr > 0:
        fails.append(f"{label}: real_render_frame_ms 非正: {rr!r}")
    pm = doc["present_frame_ms"]
    if not isinstance(pm, (int, float)) or isinstance(pm, bool) or not pm > 0:
        fails.append(f"{label}: present_frame_ms 非正: {pm!r}")
    if not isinstance(doc["stats"], dict):
        fails.append(f"{label}: stats 非 object")
    hz = doc["hzb"]
    if not isinstance(hz, dict):
        fails.append(f"{label}: hzb 块非 object")
        return fails
    hz_missing = [k for k in HZB_BLOCK_REQUIRED if k not in hz]
    if hz_missing:
        fails.append(f"{label}: hzb 块缺键 {hz_missing}（fail-closed 不静默）")
        return fails
    if hz["all_visible_arm"] is not expect_allvis:
        fails.append(f"{label}: hzb.all_visible_arm ≠ {expect_allvis}（实验臂标记面破）")
    if not _pos_int(hz["instances"]):
        fails.append(f"{label}: hzb.instances < 1: {hz['instances']!r}")
    if not _pos_int(hz["mips"]):
        fails.append(f"{label}: hzb.mips < 1: {hz['mips']!r}")
    for k in ("tested", "occluded_p1", "flipped_p2", "closure_extra_submits", "closure_full_fallback_frames"):
        if not _nonneg_int(hz[k]):
            fails.append(f"{label}: hzb.{k} 非负整数破: {hz[k]!r}")
    if not isinstance(hz["parity"], dict):
        fails.append(f"{label}: hzb.parity 非 object")
    return fails


def baseline_leg_judge(doc: dict, frames: int, warmup: int, label: str) -> list[str]:
    """baseline 腿判（--full 无 --hzb 统一车道腿——rurix.g34.unified_lane_
    evidence.v1 面;G34-1 harness 公共判同字面）。"""
    fails: list[str] = []
    if doc.get("schema") != BASELINE_SCHEMA_ID:
        fails.append(f"{label}: schema ≠ {BASELINE_SCHEMA_ID}: {doc.get('schema')!r}")
    if doc.get("gate") != BASELINE_GATE_KEY:
        fails.append(f"{label}: gate ≠ {BASELINE_GATE_KEY}: {doc.get('gate')!r}")
    total = frames + warmup
    if doc.get("frames_completed") != total:
        fails.append(f"{label}: frames_completed {doc.get('frames_completed')} ≠ {total}")
    if doc.get("exit_reason") != "frames_done":
        fails.append(f"{label}: exit_reason ≠ frames_done: {doc.get('exit_reason')!r}")
    seq = doc.get("digest_seq")
    if not isinstance(seq, list) or len(seq) != total or any(
        not isinstance(x, str) or not DIGEST_RE.match(x) for x in seq
    ):
        fails.append(f"{label}: digest_seq 形态/长度破（≠{total}）")
    if doc.get("digest") != (seq[-1] if isinstance(seq, list) and seq else None):
        fails.append(f"{label}: digest ≠ digest_seq 末项")
    if not isinstance(doc.get("render_digest"), str) or not DIGEST_RE.match(doc["render_digest"]):
        fails.append(f"{label}: render_digest 形态破")
    rr = doc.get("real_render_frame_ms")
    if not isinstance(rr, (int, float)) or isinstance(rr, bool) or not rr > 0:
        fails.append(f"{label}: real_render_frame_ms 非正: {rr!r}")
    if doc.get("render_includes_forced_readback") is not True:
        fails.append(f"{label}: render_includes_forced_readback ≠ true")
    if (doc.get("contracts") or {}).get("consistency") != "pass":
        fails.append(f"{label}: contracts.consistency ≠ pass")
    hp = doc.get("host_parity")
    if not isinstance(hp, dict) or hp.get("in_tol") is not True:
        fails.append(f"{label}: host_parity.in_tol ≠ true（bin 内 fail-closed 未拦即红）")
    return fails


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def run_leg(label: str, argv: list[str], env: dict, timeout: int = 3600) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = leg_evidence_path(label)
    r = run(argv, timeout=timeout, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def run_gate(frames: int, warmup: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:180]}")

    if not GATE_SCHEMA_PATH.is_file():
        fail(f"gate schema 缺失: {GATE_SCHEMA_PATH}")
        return 1

    # ── 构建（release 双臂 + rurixc debug SPV 面）──
    ok = build_or_fail(
        ["cargo", "build", "--release", "-p", "rurix-render", "--features", "vendor-upscale",
         "--bin", "g34_full_lane", "--bin", "g14_3_pipeline_perf", "--quiet"],
        "harness release",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── ①②③ kernel SPV 面：现编五件 + spirv-val + 冻结 0-byte 机核 + 快照 ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for src_name, dst_name in KERNEL_SPECS:
        src = KERNEL_DIR / src_name
        dst = WORK / dst_name
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(dst)], timeout=1800)
        if r.returncode != 0 or not dst.is_file():
            spv_ok = False
            note(f"rurixc 编译失败 {src_name}: {(r.stdout + r.stderr)[-200:]}")
            continue
        val = run(["spirv-val", str(dst)], timeout=600)
        if val.returncode != 0:
            spv_ok = False
            note(f"spirv-val 未过 {dst_name}: {(val.stdout + val.stderr)[-200:]}")
    d = run(["git", "diff", "--quiet", "HEAD", "--", *MOTHER_TRACKED_PATHS])
    u = run(["git", "status", "--porcelain", "--", *MOTHER_TRACKED_PATHS])
    frozen_ok = d.returncode == 0
    worktree_ok = not u.stdout.strip()
    frozen_snapshot: dict[str, str] = {}
    snapshot_ok = True
    for p in FROZEN_SNAPSHOT_PATHS:
        fp = ROOT / p
        if fp.is_file():
            frozen_snapshot[p] = "sha256:" + hashlib.sha256(fp.read_bytes()).hexdigest()
        else:
            snapshot_ok = False
            frozen_snapshot[p] = "MISSING"
    set_fact(
        "kernels_spv_valid",
        spv_ok and frozen_ok and worktree_ok and snapshot_ok,
        f"rurixc 现编五件（g34_unified_primary/g34_unified_shade/g31_hzb_pack/g27_hzb_reduce/g27_hzb_test）"
        f"+ spirv-val={'绿' if spv_ok else '红'}；冻结 tracked 双 kernel（g27 两件）vs HEAD 0-byte={frozen_ok} "
        f"工作树干净={worktree_ok}；G31/G34 期 untracked 七面 sha256 快照在档={snapshot_ok}"
        "（g31_hzb_pack.rx 为 untracked 冻结源——diff-vs-HEAD 不适用,快照面承载;如实登记不冒充）",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("G34-2 kernel SPV 编译/spirv-val 未过")
    missing_lane = [f for f in LANE_SPVS if not (SPV_DIR / f).is_file()]
    if missing_lane:
        degrade.append(f"车道 SPV 缺失 {missing_lane}")
    if not SPV_PROBE.is_file():
        degrade.append(f"纹理探针 SPV 缺失 {SPV_PROBE}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")
    if not SLAB_ASSET.is_file():
        degrade.append(f"slab 侧表资产缺失 {SLAB_ASSET}")

    env = device_env()
    leg_docs: dict[str, dict] = {}
    harness_archives: list[str] = []
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    replay_doc: dict = {}
    if not degrade:
        # ── dev-env 探针（baseline 短跑 + --host-parity off——探针面 = env 检测
        #    非正确性裁决;G34-1 同字面）──
        with gpu_device_lock(purpose=f"{TAG} dev-env 探针（baseline 短跑）"):
            rp, probe_doc, _ = run_leg(
                "probe",
                build_harness_argv("probe", 2, 1, hzb_on=False, extra=["--host-parity", "off"]),
                env, timeout=1800,
            )
        probe_out = (rp.stdout or "") + (rp.stderr or "")
        if '"state":"skipped_dev_env"' in probe_out:
            degrade.append(f"harness skipped_dev_env: {probe_out.strip()[-200:]}")
        elif probe_doc is None:
            degrade.append(f"harness 探针 evidence 缺失: {probe_out.strip()[-200:]}")

    leg_ok = True
    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} 四腿 + Stage A 锚格 bench"):
            # ── ④⑤⑥⑦⑧ 四腿（gpu_device_lock 内串行）：hzb_a（对拍 + 计数）/
            #    hzb_b（确定性双跑）/ allvis（RURIX_HZB_ALL_VISIBLE=1 全集渲染
            #    实验臂）/ baseline（--full 无 --hzb 统一车道腿,frame_ms 对照）。
            legs = [
                # label, hzb_on, allvis_env, archive_prefix
                ("hzb_a", True, False, None),
                ("hzb_b", True, False, None),
                ("allvis", True, True, None),
                ("baseline", False, False, "g34_unified_lane_g34hzb_"),
            ]
            for label, hzb_on, allvis, arch_prefix in legs:
                leg_env = dict(env)
                if allvis:
                    leg_env["RURIX_HZB_ALL_VISIBLE"] = "1"
                r, doc, ev_path = run_leg(label, build_harness_argv(label, frames, warmup, hzb_on), leg_env)
                out = (r.stdout or "") + (r.stderr or "")
                # PASS 标记双形态：HZB 腿 = "[g34_full_lane]: [hzb] PASS …"（区段
                # 驱动出口字面）；baseline 腿 = "[g34_full_lane]: PASS …"（bin 总
                # 出口字面）——按腿别取对应标记，缺即败（不接受跨腿标记冒充）。
                pass_marker = "g34_full_lane]: [hzb] PASS" if hzb_on else "g34_full_lane]: PASS"
                if r.returncode != 0 or doc is None or pass_marker not in out:
                    fail(f"{label} 真跑失败 rc={r.returncode}: {out[-300:]}")
                    leg_ok = False
                    continue
                if "Validation Error" in out or "VUID-" in out:
                    fail(f"{label} validation 应静默却报错")
                    leg_ok = False
                j = (
                    hzb_leg_judge(doc, frames, warmup, label, expect_allvis=allvis)
                    if hzb_on
                    else baseline_leg_judge(doc, frames, warmup, label)
                )
                for m in j:
                    fail(m)
                leg_ok &= not j
                leg_docs[label] = doc
                if arch_prefix:
                    # baseline 腿 = 统一车道面（rurix.g34.unified_lane_evidence.v1）
                    # → 归档 evidence/ 用 g34_unified_lane_g34hzb_ 前缀复用
                    # g34_unified_lane_ 既有路由（skin 门对照腿归档同法）。
                    arch = ROOT / "evidence" / f"{arch_prefix}{label}_{ts}.json"
                    arch.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                    harness_archives.append(str(arch.relative_to(ROOT)))
                # HZB 腿 harness 真跑件留 .tmp 工作区不归档 evidence/——
                # harness 真跑件不注册 check_schemas,数字经门裁决件蒸馏登记。
            # ── ⑨ Stage A 锚格 bench 复跑（canonical 160 帧;共享体加性扩展
            #       0-byte 机器证明）──
            bench_root = WORK / "anchor_bench"
            r = run(
                [str(BIN_BENCH), "--bench", "--scene", SCENE, "--tier", "100",
                 "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
                 "--out-root", str(bench_root)],
                timeout=3600, env=env,
            )
            receipt = bench_root / SCENE / "tier100" / "tsr_device" / "bench_receipt.json"
            fresh_replay = None
            if r.returncode == 0 and receipt.is_file():
                fresh_replay = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
            anchors = json.loads(ANCHOR_PATH.read_text(encoding="utf-8")).get("anchors") or {}
            anchor_dg = (anchors.get(ANCHOR_CELL) or {}).get("last_frame_digest")
            replay_doc = {
                "cell": ANCHOR_CELL,
                "fresh_digest": fresh_replay,
                "anchor_digest": anchor_dg,
                "match": anchor_match(fresh_replay, anchor_dg),
                "frames": 160,
                "warmup": 10,
            }
            set_fact(
                "stage_a_anchor_replay",
                replay_doc["match"],
                f"Stage A 锚格 {ANCHOR_CELL} bench 复跑:fresh {str(fresh_replay)[:23]}… vs 在案 {str(anchor_dg)[:23]}… "
                f"{'位级 MATCH（共享体加性扩展 0-byte 机器证明）' if replay_doc['match'] else 'DRIFT（RED）'}",
            )

            if leg_ok:
                hzb_a = leg_docs["hzb_a"]
                hz = hzb_a.get("hzb") or {}
                # ── ④ 剔除像素中性（hzb_a vs allvis 全集渲染实验臂）──
                seq_on = hzb_a.get("digest_seq", [])
                seq_allvis = leg_docs["allvis"].get("digest_seq", [])
                neutral_bit = seqs_bitexact(seq_on, seq_allvis)
                neutral_mismatch = seq_mismatch_count(seq_on, seq_allvis)
                set_fact(
                    "culling_pixel_neutral",
                    neutral_bit and neutral_mismatch == 0,
                    f"--hzb on vs RURIX_HZB_ALL_VISIBLE=1 全集渲染实验臂 digest_seq 逐帧位级一致={neutral_bit}"
                    f"（mismatch={neutral_mismatch}/{len(seq_on)}；剔除不改变可见像素 = 两阶段闭环正确性结构判据）",
                )
                # ── ⑤ host 金标准对拍（probe 帧 parity 三面 + digest 互核）──
                parity = hz.get("parity") or {}
                parity_fails = parity_judge(parity)
                set_fact(
                    "hzb_host_parity",
                    not parity_fails,
                    f"probe 帧 hzb.parity:mips 位级={parity.get('mips_bitexact')} "
                    f"verdict 全等={parity.get('verdict_equal')} fp={parity.get('false_positives')} "
                    f"（mips={parity.get('mips')} n_rects={parity.get('n_rects')} occluded={parity.get('occluded')}；"
                    f"pyramid/verdict digest vs host 互核）"
                    + ("" if not parity_fails else f"；红 {parity_fails[:2]}"),
                )
                # ── ⑥ 确定性双跑 ──
                bit = seqs_bitexact(seq_on, leg_docs["hzb_b"].get("digest_seq", []))
                rd_eq = hzb_a.get("render_digest") == leg_docs["hzb_b"].get("render_digest")
                set_fact(
                    "determinism_double_run",
                    bit and rd_eq,
                    f"--hzb on 双跑 digest_seq 位级一致={bit}（{len(seq_on)} 帧）render_digest 一致={rd_eq}（确定性门）",
                )
                # ── ⑦⑧ 剔除真实发生 + frame_ms 对照（不设通过线 G6 无硬门）──
                on_mean = hzb_a.get("real_render_frame_ms", -1.0)
                base_mean = leg_docs["baseline"].get("real_render_frame_ms", -1.0)
                eff_ok = counts_effective(hz)
                ms_ok = frame_ms_sane(on_mean, base_mean)
                ratio = (on_mean / base_mean) if ms_ok else -1.0
                set_fact(
                    "culling_effective_measured",
                    eff_ok and ms_ok,
                    f"剔除真实发生:tested={hz.get('tested')} occluded_p1={hz.get('occluded_p1')}"
                    f"（零剔除即空接线冒充判红）flipped_p2={hz.get('flipped_p2')} 闭环(额外提交/全量兜底帧)="
                    f"{hz.get('closure_extra_submits')}/{hz.get('closure_full_fallback_frames')}"
                    f"；frame_ms measured:hzb_on={on_mean:.4f}ms baseline={base_mean:.4f}ms"
                    f"（on/baseline={ratio:.4f}；如实登记不设通过线,G6 无硬门纪律,measured_local）",
                )

    if degrade:
        doc = {
            "schema": "rurix.g34.hzb_unified.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for dg in degrade:
            note(f"DEV_ENV_DEGRADE {dg}")
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL）")
        return 0

    # ── evidence 落盘（门裁决件;PASS-only schema——PASS 件 jsonschema 自校验
    #    后落 evidence/,FAIL 诊断件落 .tmp 工作区不污染路由面）──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and leg_ok and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    hzb_a_doc = leg_docs.get("hzb_a") or {}
    hz_block = hzb_a_doc.get("hzb") or {}
    parity_block = hz_block.get("parity") or {}
    seq_on = hzb_a_doc.get("digest_seq", [])
    seq_allvis = (leg_docs.get("allvis") or {}).get("digest_seq", [])
    nm = seq_mismatch_count(seq_on, seq_allvis)
    on_mean = hzb_a_doc.get("real_render_frame_ms", -1.0)
    base_mean = (leg_docs.get("baseline") or {}).get("real_render_frame_ms", -1.0)
    stats_on = hzb_a_doc.get("stats") or {}
    d0 = "sha256:" + "0" * 64

    def spv_entry(p: Path) -> dict:
        return {
            "path": str(p.relative_to(ROOT)).replace("\\", "/"),
            "sha256": "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else d0,
        }

    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "kernels": {
            "primary_spv": spv_entry(SPV_PRIMARY),
            "shade_spv": spv_entry(SPV_SHADE),
            "pack_spv": spv_entry(SPV_PACK),
            "reduce_spv": spv_entry(SPV_REDUCE),
            "test_spv": spv_entry(SPV_TEST),
            "spirv_val_all": bool(spv_ok),
            "mother_tracked_0byte": bool(frozen_ok and worktree_ok),
            "frozen_snapshot": frozen_snapshot,
        },
        "culling": {
            "neutrality": {
                "trajectory": TRAJECTORY,
                "frames": frames,
                "warmup": warmup,
                "seq_len": len(seq_on),
                "hzb_vs_allvis_seq_bitexact": bool(seqs_bitexact(seq_on, seq_allvis)),
                "mismatch_count": int(nm) if nm >= 0 else -1,
            },
            "occlusion": {
                "instances": int(hz_block.get("instances", 0)),
                "mips": int(hz_block.get("mips", 0)),
                "tested": int(hz_block.get("tested", 0)),
                "occluded_p1": int(hz_block.get("occluded_p1", 0)),
                "flipped_p2": int(hz_block.get("flipped_p2", 0)),
                "closure_extra_submits": int(hz_block.get("closure_extra_submits", 0)),
                "closure_full_fallback_frames": int(hz_block.get("closure_full_fallback_frames", 0)),
                "all_visible_arm": bool(hz_block.get("all_visible_arm", False)),
            },
        },
        "parity": {
            "mips": int(parity_block.get("mips", 0)),
            "n_rects": int(parity_block.get("n_rects", 0)),
            "mips_bitexact": parity_block.get("mips_bitexact") is True,
            "verdict_equal": parity_block.get("verdict_equal") is True,
            "false_positives": int(parity_block.get("false_positives", -1)),
            "occluded": int(parity_block.get("occluded", 0)),
            "pyramid_digest": parity_block.get("pyramid_digest") or d0,
            "host_pyramid_digest": parity_block.get("host_pyramid_digest") or d0,
            "verdict_digest": parity_block.get("verdict_digest") or d0,
            "host_verdict_digest": parity_block.get("host_verdict_digest") or d0,
        },
        "determinism": {
            "double_run_bitexact": bool(seqs_bitexact(seq_on, (leg_docs.get("hzb_b") or {}).get("digest_seq", []))),
            "frames": len(seq_on),
            "render_digest_a": hzb_a_doc.get("render_digest") or d0,
            "render_digest_b": (leg_docs.get("hzb_b") or {}).get("render_digest") or d0,
        },
        "frame_ms": {
            "hzb_on_mean": on_mean,
            "baseline_mean": base_mean,
            "on_over_baseline": (on_mean / base_mean) if frame_ms_sane(on_mean, base_mean) else -1.0,
            "scene_gpu_ms_mean": stats_on.get("scene_gpu_ms", -1.0),
            "hzb_gpu_ms_mean": stats_on.get("hzb_gpu_ms", -1.0),
            "measured": "measured_local",
            "frames_per_run": frames,
            "note": (
                f"bistro-interior 1080p orbit --hidden release 同机同窗 measured_local:"
                f"hzb_on={on_mean:.4f}ms baseline(--full 无 --hzb)={base_mean:.4f}ms；"
                "如实登记不设通过线（G6 无硬门纪律）;hzb_gpu_ms_mean=-1.0 表 harness "
                "stats 面未登记该分项（如实登记）"
            ),
        },
        "regression_anchor": replay_doc if replay_doc else {
            "cell": ANCHOR_CELL, "fresh_digest": d0,
            "anchor_digest": d0, "match": False, "frames": 160, "warmup": 10,
        },
        "harness_evidence": harness_archives,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G34 全特性合流 G34-2 HZB 接统一车道（G31 波 B Task B1 生产接线面逐字同律 + G34 统一"
            "车道合流）：剔除对象粒度 = TLAS 实例（bistro 逐 mesh 节点 BLAS 分解 + 动态实例尾槽"
            "恒可见不参剔——A4 核验对象,剔除计数面 = 静态节点如实登记）;消费点 = 主射线 pass 的 "
            "TLAS 实例 mask（g34_unified_primary 相机射线走初剔后表 0,g34_unified_shade 阴影射线"
            "走全量表 1——被剔实例仍投阴影,RXS-0297 单 TLAS 签名纪律 ⇒ 拆 pass）;双 TLAS 逐帧 "
            "refit + 帧内金字塔轮换（上帧金字塔初剔 p1 → 本帧真深度〔g34_unified_shade ④b 段 "
            "out_depth_hz,vp 行 2/3 另算真 ZO NDC,与 U_SCENE_DEPTH 两路并存互不染指〕逐级归约平铺"
            "重建 → 上帧被剔集重测 p2）+ RFC-0044 §5.8 两阶段闭环第二段（应见集 = p1 可见 ∪ p2 "
            "翻回,未渲者掩码并集同帧重渲,≤4 迭代未收敛全掩码兜底 = 零剔除精确收敛）;g31_hzb_pack/"
            "g27_hzb_reduce/g27_hzb_test 冻结消费 + geometry/{hzb,cull}.rs host 金标准只读 0-byte。"
            "九类判据蒸馏六 facts：SPV 面（五件现编 + g27 双 kernel tracked 0-byte + untracked "
            "七面快照——g31_hzb_pack.rx untracked 现实快照面承载如实登记）/ 剔除像素中性（hzb_a "
            "vs RURIX_HZB_ALL_VISIBLE=1 位级——剔除零假阳性 ⇒ 闭环后画面与全集渲染位级一致）/ "
            "host 金标准对拍（mips 逐级位级 + 判定序列逐字节 + 零假阳性独立复核）/ 确定性双跑位级 / "
            "剔除真实发生（occluded_p1≥1 零剔除即空接线冒充判红）+ frame_ms 对照（不设通过线 G6 "
            "无硬门纪律 measured_local）/ Stage A 锚复跑零漂移。HZB 腿 harness 真跑件留 .tmp——"
            "harness 真跑件不注册 check_schemas,数字经门裁决件蒸馏登记;baseline 对照腿归档 "
            "g34_unified_lane_g34hzb_ 前缀复用既有路由。"
            f"facts: {'; '.join(f['id'] + '=' + f['status'] for f in fact_rows)}"
        ),
    }
    if all_pass:
        import jsonschema  # 自校验硬门（schema 漂移即 RED;PASS-only 闭集面）

        errs = list(jsonschema.Draft7Validator(
            json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        ).iter_errors(gate_doc))
        if errs:
            fail("gate evidence schema 自校验红: " + "; ".join(
                f"{'/'.join(str(p) for p in e.path)}: {e.message}" for e in errs[:3]))
            all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g34_hzb_unified_gate_{ts}.json"
    else:
        # FAIL 诊断件落 .tmp 工作区——PASS-only schema 面,evidence/ 只收门件
        # （fail-closed：evidence/ 无件 = 门未过,不冒充）。
        gate_path = WORK / f"gate_fail_{ts}.json"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}（HZB 腿 harness 真跑件留 .tmp 工作区;baseline 归档 {len(harness_archives)} 件）")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂,无 GPU/无构建依赖）
# ---------------------------------------------------------------------------


def _dg(ch: str) -> str:
    return "sha256:" + ch * 64


def _good_parity() -> dict:
    return {
        "mips": 11, "n_rects": 800, "mips_bitexact": True, "verdict_equal": True,
        "false_positives": 0, "occluded": 120,
        "pyramid_digest": _dg("e"), "host_pyramid_digest": _dg("e"),
        "verdict_digest": _dg("f"), "host_verdict_digest": _dg("f"),
    }


def _good_hzb_leg(allvis: bool = False) -> dict:
    return {
        "schema": HARNESS_SCHEMA_ID,
        "gate": GATE_KEY,
        "frames": 2,
        "warmup": 1,
        "frames_completed": 3,
        "exit_reason": "frames_done",
        "digest_seq": [_dg("a"), _dg("b"), _dg("c")],
        "render_digest": _dg("d"),
        "real_render_frame_ms": 5.5,
        "present_frame_ms": 1.2,
        "stats": {"scene_gpu_ms": 3.3},
        "hzb": {
            "all_visible_arm": allvis, "instances": 1186, "mips": 11,
            "tested": 9000, "occluded_p1": 120, "flipped_p2": 2,
            "closure_extra_submits": 1, "closure_full_fallback_frames": 0,
            "parity": _good_parity(),
        },
        "environment": {"gpu": "x"},
    }


def _good_baseline_leg() -> dict:
    seq = [_dg("a"), _dg("b"), _dg("c")]
    return {
        "schema": BASELINE_SCHEMA_ID,
        "gate": BASELINE_GATE_KEY,
        "frames": 2,
        "warmup": 1,
        "frames_completed": 3,
        "exit_reason": "frames_done",
        "digest_seq": seq,
        "digest": seq[-1],
        "render_digest": _dg("d"),
        "real_render_frame_ms": 4.4,
        "render_includes_forced_readback": True,
        "contracts": {"consistency": "pass"},
        "host_parity": {"in_tol": True},
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

    # 红绿臂①:digest 序列判（④ 剔除像素中性 / ⑥ 确定性双跑共用面）。
    expect(seqs_bitexact(["a", "b"], ["a", "b"]), "GREEN:双臂位级正例")
    expect(not seqs_bitexact(["a", "b"], ["a", "x"]), "RED:漂移必红")
    expect(not seqs_bitexact(["a"], ["a", "b"]), "RED:长度不齐必红")
    expect(not seqs_bitexact([], []), "RED:空序列必红")
    expect(seq_mismatch_count(["a", "b", "c"], ["a", "x", "c"]) == 1, "GREEN:mismatch 计数正例")
    expect(seq_mismatch_count(["a", "b"], ["a", "b"]) == 0, "GREEN:零 mismatch 正例")
    expect(seq_mismatch_count(["a"], ["a", "b"]) == -1, "RED:长度不齐 -1 拒判")
    # 红绿臂②:⑤ host 金标准对拍判。
    expect(parity_judge(_good_parity()) == [], "GREEN:parity 正例")
    expect(parity_judge(dict(_good_parity(), mips_bitexact=False)), "RED:mips 非位级必红")
    expect(parity_judge(dict(_good_parity(), verdict_equal=False)), "RED:判定序列异必红")
    expect(parity_judge(dict(_good_parity(), false_positives=1)), "RED:假阳性 1 必红")
    expect(parity_judge(dict(_good_parity(), host_pyramid_digest=_dg("9"))),
           "RED:金字塔 digest vs host 不符（旗标互核）必红")
    expect(parity_judge(dict(_good_parity(), verdict_digest="sha256:zz")), "RED:digest 形态非法必红")
    expect(parity_judge({k: v for k, v in _good_parity().items() if k != "false_positives"}),
           "RED:缺键 fail-closed 必红")
    expect(parity_judge(None), "RED:parity 非 object 必红")
    # 红绿臂③:HZB 腿契约键 fail-closed 判。
    expect(hzb_leg_judge(_good_hzb_leg(), 2, 1, "t", False) == [], "GREEN:HZB 腿契约正例")
    bad = {k: v for k, v in _good_hzb_leg().items() if k != "render_digest"}
    expect(hzb_leg_judge(bad, 2, 1, "t", False), "RED:顶层缺键（render_digest）fail-closed 必红")
    bad = _good_hzb_leg()
    bad["hzb"] = {k: v for k, v in bad["hzb"].items() if k != "closure_full_fallback_frames"}
    expect(hzb_leg_judge(bad, 2, 1, "t", False), "RED:hzb 块缺键 fail-closed 必红")
    expect(hzb_leg_judge(_good_hzb_leg(allvis=True), 2, 1, "t", False),
           "RED:all_visible_arm 与腿别不符必红（实验臂标记面）")
    expect(hzb_leg_judge(_good_hzb_leg(allvis=True), 2, 1, "t", True) == [],
           "GREEN:allvis 腿标记正例")
    expect(hzb_leg_judge(dict(_good_hzb_leg(), frames_completed=2), 2, 1, "t", False),
           "RED:frames_completed 缺帧必红")
    expect(hzb_leg_judge(dict(_good_hzb_leg(), present_frame_ms=0.0), 2, 1, "t", False),
           "RED:present_frame_ms 非正必红")
    expect(hzb_leg_judge(dict(_good_hzb_leg(), schema="rurix.wrong.v1"), 2, 1, "t", False),
           "RED:schema 字面不符必红")
    # 红绿臂④:⑦ 剔除真实发生判。
    good_hz = _good_hzb_leg()["hzb"]
    expect(counts_effective(good_hz), "GREEN:剔除活跃正例")
    expect(not counts_effective(dict(good_hz, occluded_p1=0)), "RED:零剔除（空接线冒充）必红")
    expect(not counts_effective(dict(good_hz, tested=0)), "RED:零测试必红")
    expect(not counts_effective(dict(good_hz, flipped_p2=-1)), "RED:负计数必红")
    expect(not counts_effective(dict(good_hz, closure_full_fallback_frames="0")),
           "RED:字符串冒充计数必红")
    expect(not counts_effective({}), "RED:空 hzb 块必红")
    # 红绿臂⑤:baseline 腿判（统一车道面）。
    expect(baseline_leg_judge(_good_baseline_leg(), 2, 1, "b") == [], "GREEN:baseline 腿正例")
    expect(baseline_leg_judge(dict(_good_baseline_leg(), host_parity=None), 2, 1, "b"),
           "RED:baseline host_parity 缺失必红（bin 内 fail-closed 未拦即红）")
    expect(baseline_leg_judge(dict(_good_baseline_leg(), schema=HARNESS_SCHEMA_ID), 2, 1, "b"),
           "RED:baseline schema 混用 HZB 面必红")
    expect(baseline_leg_judge(dict(_good_baseline_leg(), digest=_dg("9")), 2, 1, "b"),
           "RED:digest ≠ digest_seq 末项必红")
    # 红绿臂⑥:⑨ Stage A 锚格判。
    expect(anchor_match(_dg("a"), _dg("a")), "GREEN:锚位级 MATCH 正例")
    expect(not anchor_match(_dg("a"), _dg("b")), "RED:锚 DRIFT 必红")
    expect(not anchor_match(None, _dg("a")), "RED:fresh 缺失必红")
    expect(not anchor_match("not-a-digest", _dg("a")), "RED:fresh 形态破必红")
    # 红绿臂⑦:⑧ frame_ms 健全判 + 冻结 0-byte 机核判。
    expect(frame_ms_sane(3.5, 41.1), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(3.5, 0.0), "RED:0ms 必红")
    expect(not frame_ms_sane(3.5, float("nan")), "RED:NaN 必红")
    expect(freeze_0byte(0, ""), "GREEN:冻结面 0-byte 正例")
    expect(not freeze_0byte(1, ""), "RED:diff vs HEAD 非空必红")
    expect(not freeze_0byte(0, " M src/x.rx\n"), "RED:工作树脏必红")
    # schema 互核:gate schema 在树 + PASS-only 闭集 + facts enum == FACT_IDS +
    # 关键 const 逐字 + required 闭集 + Draft7 合法。
    expect(GATE_SCHEMA_PATH.is_file(), "gate schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "wave const 互核")
        expect(gs["properties"]["verdict"]["enum"] == ["PASS"], "verdict 枚举 PASS-only 互核")
        fi = gs["properties"]["facts"]["items"]
        expect(sorted(fi["properties"]["id"]["enum"]) == sorted(FACT_IDS),
               f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(fi["properties"]["status"]["enum"] == ["PASS"], "facts.status 枚举 PASS-only 互核")
        expect(gs["properties"]["facts"]["minItems"] == 6 and gs["properties"]["facts"]["maxItems"] == 6,
               "facts 数组恰 6 项互核")
        expect(
            sorted(gs.get("required", [])) == sorted([
                "schema", "subject", "symbolic_gate_key", "wave", "facts", "verdict",
                "kernels", "culling", "parity", "determinism", "frame_ms",
                "regression_anchor", "harness_evidence", "environment", "timestamp", "notes",
            ]),
            "gate schema required 闭集互核（16 键）",
        )
        snap_req = gs["properties"]["kernels"]["properties"]["frozen_snapshot"].get("required", [])
        expect(sorted(snap_req) == sorted(FROZEN_SNAPSHOT_PATHS),
               "frozen_snapshot required == FROZEN_SNAPSHOT_PATHS 闭集互核")
        par_req = gs["properties"]["parity"].get("required", [])
        expect(sorted(par_req) == sorted(PARITY_REQUIRED),
               "parity required == 契约 PARITY_REQUIRED 闭集互核")
        occ_req = gs["properties"]["culling"]["properties"]["occlusion"].get("required", [])
        expect(sorted(occ_req) == sorted([k for k in HZB_BLOCK_REQUIRED if k != "parity"]),
               "culling.occlusion required == 契约 hzb 计数键闭集互核")
        import jsonschema as _js
        _js.Draft7Validator.check_schema(gs)
        print("  ok   — gate schema Draft7 合法（check_schema 绿）")
    expect(len(FACT_IDS) == 6, "facts 闭集 = 6")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=6；7 红臂组 + 正例组 + gate schema 互核）")
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
            print(f"[{TAG}] FAIL: --frames {args.frames} < 32（确定性/中性对拍面下限）", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.warmup)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
