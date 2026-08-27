#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G34 全特性合流 G34-3 蒙皮角色进真窗口统一车道）
"""G34-3：蒙皮角色进真窗口统一车道门冒烟（g34.wave2.skin；蒙皮×纹理×slab×
动态实例四特性同开——G34Full 27 SSBO 加性扩蒙皮七件 = 36 资源六 pass：
g31_skin 0-byte 复用 → blas_refit 桥（角色 BLAS 2 逐帧 UPDATE）→
kernels/g34_unified_gi_skin.rx（G34-1 统一 kernel + out_hit 命中信息通道 +
角色实例分派）→ kernels/g34_unified_mv.rx（g31_skin_mv 镜像 + 类 2 刚性实例
臂——A4 登记缺口统一车道蒙皮腿顺手接通）→ TSR 双 pass → display_encode;
逐帧 tlas_update refit 3 实例表;顺序入口 inflight=1;harness =
src/rurix-render/src/bin/g34_full_lane.rs --skin on,蒙皮段全量收
src/bin/g34_full_lane/g34_skin_section.rs 独立 include 区段）。

九面判据（facts 闭集）：
1. **kernels_spv_valid**：rurixc 现编 g34_unified_gi_skin/g34_unified_mv/
   g31_skin（0-byte 复用）三 SPV + spirv-val 全绿 + 母版 tracked 双 kernel
   vs HEAD 0-byte + G31/G34 期 untracked 面（g34_unified_gi/g34_unified_shade/
   g31_skin 三件套/g31_texture_gi/g31_dyn_scene/g31_hzb_shade/g31_window_present/
   g14_mv）sha256 快照在档（其门为回归锚）。
2. **skin_vertex_bitexact**：① 蒙皮 device/host 逐顶点对拍 max_abs == 0 位级
   门全核验帧（B5 在案口径;tris 角色段回读 vs host skin_vertex）。
3. **skin_position_verified**：② 位置核验——host 蒙皮投影并集掩码 vs hit
   通道 inst==2 地面真值检测,质心 ≤4.0px/AABB ≤6.0px/计数门全核验帧 pass
   （B5 在案口径）。
4. **skin_mv_wired**：③ 类 3 蒙皮 MV dev/host 逐分量中位差 ≤2.0px + 窗级
   聚合真动门 max host ≥1.0px + 高动帧条件 ratio 门 + 类 1 静态区相机 MV
   一致性 ≤2.0px（auto-move 动相机下 B5 静态绝对门的诚实重述）。
5. **rigid_mv_wired**：类 2 刚性实例 MV（A4 登记缺口本腿顺手接通）——hit
   通道 inst==1 像素 dev/host 逐分量中位差 ≤2.0px,核验激活帧 ≥1。
6. **determinism_double_run**：--skin on 双跑 digest_seq 逐帧位级一致 +
   render_digest 一致（确定性门）。
7. **per_feature_digest_discrimination**：skin ≠ baseline（静态腿）AND
   skin ≠ full_noskin（无蒙皮全特性腿）digest_seq 各至少一帧不同（蒙皮
   贡献真实生效,防暗接线冒充）。
8. **stage_a_anchor_replay**：g14_3_pipeline_perf canonical 160 帧
   bistro-interior/t100/tsr_device 末帧 digest == 在案锚（共享体加性扩展
   对既有面 0-byte 的机器证明）。
9. **frame_ms_measured**：同机同窗 orbit --hidden 1920×1080 release 真跑
   skin on/off 对照 + skin_gpu_ms 分项（measured_local 诚实登记）。

三态：无 Vulkan loader/设备/场景资产/SPV → DEV_ENV_DEGRADE 退 0（不冒充
PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充真跑）。

用法：
  py -3 ci/g34_skin_unified_smoke.py --selftest
  py -3 ci/g34_skin_unified_smoke.py --gate g34.wave2.skin [--frames 64] [--warmup 10]
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

GATE_KEY = "g34.wave2.skin"
SUBJECT = "g34_skin_unified"
WAVE = "G34.2"
TAG = "g34_skin_unified"
SCHEMA_PATH = ROOT / "milestones" / "g34" / "g34_skin_unified_evidence_schema.json"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g34" / "g34_skin_unified_gate_evidence_schema.json"
SCHEMA_ID = "rurix.g34.skin_unified_evidence.v1"
GATE_SCHEMA_ID = "rurix.g34.skin_unified_gate_evidence.v1"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_tsr_device"
SLAB_ASSET = ROOT / "milestones" / "g31" / "g31_slab_side_table_bistro_interior.json"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
KERNEL_GI_SKIN = ROOT / "src" / "rurix-render" / "kernels" / "g34_unified_gi_skin.rx"
KERNEL_MV = ROOT / "src" / "rurix-render" / "kernels" / "g34_unified_mv.rx"
KERNEL_SKIN = ROOT / "src" / "rurix-render" / "kernels" / "g31_skin.rx"
WORK = ROOT / ".tmp" / "g34_gates" / "skin"
SPV_GI_SKIN = WORK / "g34_unified_gi_skin.spv"
SPV_MV = WORK / "g34_unified_mv.spv"
SPV_SKIN = WORK / "g31_skin.spv"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
LANE_SPVS = (
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
    # HEAD（g30-closed 系）已入 git 的母版 kernel——diff vs HEAD 机核 0-byte。
    "src/rurix-render/kernels/g14_3_direct_gi.rx",
    "src/rurix-render/kernels/g14_3_shade_reduce.rx",
]
FROZEN_SNAPSHOT_PATHS = [
    # G31/G34 期 untracked 面（工作树 untracked;diff-vs-HEAD 不适用）——G34-3
    # sha256 快照 = 后续波漂移守护基线;G34-1 统一双 kernel + G31 B5 蒙皮三
    # 件套 + fork A/B 母版 + g31_window_present.rs + g14_mv 0-byte 承诺面。
    "src/rurix-render/kernels/g34_unified_gi.rx",
    "src/rurix-render/kernels/g34_unified_shade.rx",
    "src/rurix-render/kernels/g31_skin.rx",
    "src/rurix-render/kernels/g31_skin_scene.rx",
    "src/rurix-render/kernels/g31_skin_mv.rx",
    "src/rurix-render/kernels/g31_texture_gi.rx",
    "src/rurix-render/kernels/g31_dyn_scene.rx",
    "src/rurix-render/kernels/g31_hzb_shade.rx",
    "src/rurix-render/kernels/g14_mv.rx",
    "src/rurix-render/src/bin/g31_window_present.rs",
]
SCENE = "bistro-interior"
TRAJECTORY = "orbit"

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

# B5 在案口径（harness bin 同源常量;CI 判读器互核面）。
SKIN_TOL_CENTROID_PX = 4.0
SKIN_TOL_AABB_PX = 6.0
SKIN_MV_TOL_MEDIAN_PX = 2.0
SKIN_MV_HOST_MOTION_MIN_PX = 1.0
G34S_STATIC_MV_TOL_PX = 2.0
G34S_RIGID_MV_TOL_PX = 2.0
G34S_RIGID_MIN_COUNT = 50

FACT_IDS = [
    "kernels_spv_valid",
    "skin_vertex_bitexact",
    "skin_position_verified",
    "skin_mv_wired",
    "rigid_mv_wired",
    "determinism_double_run",
    "per_feature_digest_discrimination",
    "stage_a_anchor_replay",
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
# 判读器（selftest 红绿两臂消费面）
# ---------------------------------------------------------------------------


def vertex_bitexact(rows: list[dict]) -> bool:
    """① 逐顶点对拍位级判：核验帧非空且全帧 vertex_max_abs == 0.0。"""
    return len(rows) > 0 and all(
        isinstance(r.get("vertex_max_abs"), (int, float))
        and not isinstance(r.get("vertex_max_abs"), bool)
        and r.get("vertex_max_abs") == 0.0
        for r in rows
    )


def position_verified(rows: list[dict]) -> bool:
    """② 位置核验判：核验帧非空,全帧质心/AABB/计数门 + pass 旗标。"""
    if not rows:
        return False
    for r in rows:
        cd = r.get("centroid_delta_px")
        ad = r.get("aabb_delta_px")
        if not (isinstance(cd, (int, float)) and cd <= SKIN_TOL_CENTROID_PX):
            return False
        if not (isinstance(ad, (int, float)) and ad <= SKIN_TOL_AABB_PX):
            return False
        if not (isinstance(r.get("obs_count"), int) and r["obs_count"] >= 1):
            return False
        if r.get("pass") is not True:
            return False
    return True


def mv_wired(rows: list[dict], motion_max: float) -> bool:
    """③ 类 3 MV + 类 1 静态一致性判：逐帧中位差 ≤ 容差 + 窗级真动门。"""
    if not rows or not (isinstance(motion_max, (int, float)) and motion_max >= SKIN_MV_HOST_MOTION_MIN_PX):
        return False
    for r in rows:
        d = r.get("mv_median_delta_px") or []
        s = r.get("static_mv_delta_px") or []
        if len(d) != 2 or len(s) != 2:
            return False
        if not all(isinstance(x, (int, float)) and x <= SKIN_MV_TOL_MEDIAN_PX for x in d):
            return False
        if not all(isinstance(x, (int, float)) and x <= G34S_STATIC_MV_TOL_PX for x in s):
            return False
        if r.get("pass") is not True:
            return False
    return True


def rigid_wired(rows: list[dict]) -> tuple[bool, int, float]:
    """类 2 刚性 MV 判：激活帧（rigid_count ≥ 阈）≥1 且激活帧逐分量中位差
    ≤ 2.0px;返回 (ok, active_frames, delta_max)。"""
    active = 0
    delta_max = 0.0
    for r in rows:
        if not (isinstance(r.get("rigid_count"), int) and r["rigid_count"] >= G34S_RIGID_MIN_COUNT):
            continue
        d = r.get("rigid_mv_delta_px") or []
        if len(d) != 2 or not all(isinstance(x, (int, float)) for x in d):
            continue
        active += 1
        delta_max = max(delta_max, max(d))
        if not all(x <= G34S_RIGID_MV_TOL_PX for x in d):
            return False, active, delta_max
    return active >= 1, active, delta_max


def seqs_bitexact(a: list, b: list) -> bool:
    """同轨迹双跑 digest_seq 逐帧位级一致判据。"""
    return len(a) == len(b) and len(a) > 0 and all(x == y for x, y in zip(a, b))


def seqs_differ(a: list, b: list) -> bool:
    """逐特性贡献区分判据：至少一帧 digest 不同。"""
    if len(a) != len(b):
        return True
    return any(x != y for x, y in zip(a, b))


def anchor_match(fresh: str | None, anchor: str | None) -> bool:
    """Stage A 锚格位级判据（fresh == anchor；缺 digest 即红）。"""
    return isinstance(fresh, str) and isinstance(anchor, str) and DIGEST_RE.match(fresh) is not None and fresh == anchor


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


def run_full_lane(
    label: str,
    extra: list[str],
    env: dict,
    frames: int = 64,
    warmup: int = 10,
    timeout: int = 3600,
) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = WORK / f"harness_{label}.json"
    argv = [
        str(BIN_FULL),
        "--frames", str(frames),
        "--warmup", str(warmup),
        "--tier", "100",
        "--hidden",
        "--evidence", str(ev_path),
    ] + extra
    r = run(argv, timeout=timeout, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def harness_common_judge(doc: dict, frames: int, warmup: int, label: str) -> list[str]:
    """harness evidence 公共判（蒙皮/非蒙皮腿同面;auto-move 腿 digest_seq）。"""
    fails: list[str] = []
    total = frames + warmup
    if doc.get("frames_completed") != total:
        fails.append(f"{label}: frames_completed {doc.get('frames_completed')} ≠ {total}")
    if doc.get("exit_reason") != "frames_done":
        fails.append(f"{label}: exit_reason ≠ frames_done: {doc.get('exit_reason')!r}")
    seq = doc.get("digest_seq")
    if not isinstance(seq, list) or len(seq) != total or any(not isinstance(x, str) or not DIGEST_RE.match(x) for x in seq):
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
    return fails


def skin_leg_judge(doc: dict, label: str) -> list[str]:
    """蒙皮腿专有判（schema 面 + ①②③ 聚合旗标互核;逐帧门 = bin 内
    fail-closed 已拦,本判 = evidence 面机器复核）。"""
    fails: list[str] = []
    if doc.get("schema") != SCHEMA_ID:
        fails.append(f"{label}: schema ≠ {SCHEMA_ID}: {doc.get('schema')!r}")
    if doc.get("gate") != GATE_KEY:
        fails.append(f"{label}: gate ≠ {GATE_KEY}")
    if doc.get("host_parity") is not None:
        fails.append(f"{label}: host_parity 非 null（蒙皮腿诚实登记面破）")
    sk = doc.get("skin")
    if not isinstance(sk, dict):
        fails.append(f"{label}: skin 块缺失")
        return fails
    vp = sk.get("vertex_parity") or {}
    if vp.get("all_bitexact") is not True or vp.get("max_abs_max") != 0.0:
        fails.append(f"{label}: ① 逐顶点对拍非位级（max_abs_max={vp.get('max_abs_max')!r}）")
    if sk.get("all_pass") is not True:
        fails.append(f"{label}: skin.all_pass ≠ true（bin 内 fail-closed 未拦即红）")
    mg = sk.get("motion_gate") or {}
    if not (isinstance(mg.get("host_motion_max_px"), (int, float)) and mg["host_motion_max_px"] >= SKIN_MV_HOST_MOTION_MIN_PX):
        fails.append(f"{label}: 窗级真动门 host_motion_max={mg.get('host_motion_max_px')!r} < {SKIN_MV_HOST_MOTION_MIN_PX}px")
    gap = sk.get("mv_gap") or {}
    if not (isinstance(gap.get("rigid_active_frames"), int) and gap["rigid_active_frames"] >= 1):
        fails.append(f"{label}: 类 2 刚性 MV 核验激活帧数 {gap.get('rigid_active_frames')!r} < 1")
    dyn = doc.get("dyn") or {}
    if dyn.get("all_pass") is not True:
        fails.append(f"{label}: dyn.all_pass ≠ true（蒙皮×动态同开面 dyn 核验红）")
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

    # ── ① kernel SPV 面：现编三件 + spirv-val + 0-byte 机核 ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for src, dst in ((KERNEL_GI_SKIN, SPV_GI_SKIN), (KERNEL_MV, SPV_MV), (KERNEL_SKIN, SPV_SKIN)):
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(dst)], timeout=1800)
        if r.returncode != 0 or not dst.is_file():
            spv_ok = False
            note(f"rurixc 编译失败 {src.name}: {(r.stdout + r.stderr)[-200:]}")
            continue
        val = run(["spirv-val", str(dst)], timeout=600)
        if val.returncode != 0:
            spv_ok = False
            note(f"spirv-val 未过 {dst.name}: {(val.stdout + val.stderr)[-200:]}")
    d = run(["git", "diff", "--quiet", "HEAD", "--", *MOTHER_TRACKED_PATHS])
    frozen_ok = d.returncode == 0
    u = run(["git", "status", "--porcelain", "--", *MOTHER_TRACKED_PATHS])
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
        f"rurixc 现编 g34_unified_gi_skin/g34_unified_mv/g31_skin（0-byte 复用）三件 + spirv-val={'绿' if spv_ok else '红'}；"
        f"母版 tracked 双 kernel vs HEAD 0-byte={frozen_ok} 工作树干净={worktree_ok}；"
        f"G31/G34 期 untracked 十面 sha256 快照在档={snapshot_ok}（其门为回归锚;共享车道体 = 加性扩展允许面）",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("G34-3 kernel SPV 编译/spirv-val 未过")
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
            rp, probe_doc, _ = run_full_lane(
                "probe",
                ["--auto-move", TRAJECTORY, "--frames", "2", "--warmup", "1", "--host-parity", "off"],
                env, timeout=1800,
            )
        probe_out = (rp.stdout or "") + (rp.stderr or "")
        if '"state":"skipped_dev_env"' in probe_out:
            degrade.append(f"harness skipped_dev_env: {probe_out.strip()[-200:]}")
        elif probe_doc is None:
            degrade.append(f"harness 探针 evidence 缺失: {probe_out.strip()[-200:]}")

    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} 四腿 + Stage A 锚格 bench"):
            legs = [
                # label, extra, skin_leg?, archive_prefix
                ("skin_a", ["--full", "--slab-table", str(SLAB_ASSET), "--skin", "on", "--auto-move", TRAJECTORY], True, "g34_skin_unified_"),
                ("skin_b", ["--full", "--slab-table", str(SLAB_ASSET), "--skin", "on", "--auto-move", TRAJECTORY], True, "g34_skin_unified_"),
                ("full_noskin", ["--full", "--slab-table", str(SLAB_ASSET), "--auto-move", TRAJECTORY], False, "g34_unified_lane_g34skin_"),
                ("baseline", ["--auto-move", TRAJECTORY], False, "g34_unified_lane_g34skin_"),
            ]
            leg_ok = True
            for label, extra, is_skin, arch_prefix in legs:
                r, doc, ev_path = run_full_lane(label, extra, env, frames=frames, warmup=warmup)
                out = (r.stdout or "") + (r.stderr or "")
                pass_marker = "[skin] PASS" if is_skin else "g34_full_lane]: PASS"
                if r.returncode != 0 or doc is None or pass_marker not in out:
                    fail(f"{label} 真跑失败 rc={r.returncode}: {out[-300:]}")
                    leg_ok = False
                    continue
                if "Validation Error" in out or "VUID-" in out:
                    fail(f"{label} validation 应静默却报错")
                    leg_ok = False
                j = harness_common_judge(doc, frames, warmup, label)
                if is_skin:
                    j += skin_leg_judge(doc, label)
                else:
                    hp = doc.get("host_parity")
                    if not isinstance(hp, dict) or hp.get("in_tol") is not True:
                        j.append(f"{label}: host_parity.in_tol ≠ true（非蒙皮腿 G34-1 面 bin 内 fail-closed 未拦即红）")
                for m in j:
                    fail(m)
                leg_ok &= not j
                leg_docs[label] = doc
                arch = ROOT / "evidence" / f"{arch_prefix}{label}_{ts}.json"
                arch.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                harness_archives.append(str(arch.relative_to(ROOT)))
            # ── ⑧ Stage A 锚格 bench 复跑（canonical 160 帧;共享体加性扩展
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
                skin_a = leg_docs["skin_a"]
                rows = skin_a.get("skin", {}).get("verify_frames", [])
                # ── ② ① 逐顶点对拍位级门 ──
                vp = skin_a.get("skin", {}).get("vertex_parity", {})
                set_fact(
                    "skin_vertex_bitexact",
                    vertex_bitexact(rows) and vp.get("all_bitexact") is True,
                    f"① 蒙皮 device/host 逐顶点对拍:全 {len(rows)} 核验帧 max_abs == 0.0 位级"
                    f"（B5 在案口径;窗级 max_abs_max={vp.get('max_abs_max')!r}）",
                )
                # ── ③ ② 位置核验 ──
                pos_ok = position_verified(rows)
                set_fact(
                    "skin_position_verified",
                    pos_ok,
                    f"② 位置核验:host 蒙皮投影掩码 vs hit 通道 inst==2 地面真值——"
                    f"质心 ≤{SKIN_TOL_CENTROID_PX}px/AABB ≤{SKIN_TOL_AABB_PX}px/计数门 全 {len(rows)} 核验帧 "
                    f"{'pass' if pos_ok else 'FAIL'}（B5 在案口径）",
                )
                # ── ④ ③ 类 3 MV + 类 1 静态一致性 ──
                motion_max = (skin_a.get("skin", {}).get("motion_gate") or {}).get("host_motion_max_px", -1.0)
                mv_ok = mv_wired(rows, motion_max)
                char_dmax = max((max(r.get("mv_median_delta_px", [0.0, 0.0])) for r in rows), default=0.0)
                static_dmax = max((max(r.get("static_mv_delta_px", [0.0, 0.0])) for r in rows), default=0.0)
                set_fact(
                    "skin_mv_wired",
                    mv_ok,
                    f"③ 类 3 蒙皮 MV dev/host 逐分量中位差 ≤{SKIN_MV_TOL_MEDIAN_PX}px（窗级 max={char_dmax:.3f}px）"
                    f"+ 窗级真动门 max host={motion_max:.3f}px ≥{SKIN_MV_HOST_MOTION_MIN_PX}px "
                    f"+ 类 1 静态区相机 MV 一致性 ≤{G34S_STATIC_MV_TOL_PX}px（窗级 max={static_dmax:.3f}px;"
                    f"auto-move 动相机下 B5 静态绝对门诚实重述）",
                )
                # ── ⑤ 类 2 刚性 MV（A4 缺口接通面）──
                rig_ok, rig_active, rig_dmax = rigid_wired(rows)
                set_fact(
                    "rigid_mv_wired",
                    rig_ok,
                    f"类 2 刚性实例 MV（A4 登记缺口统一车道蒙皮腿顺手接通）:hit 通道 inst==1 像素 "
                    f"dev/host 逐分量中位差 ≤{G34S_RIGID_MV_TOL_PX}px,核验激活帧={rig_active},窗级 max={rig_dmax:.3f}px",
                )
                # ── ⑥ 确定性双跑 ──
                bit = seqs_bitexact(skin_a.get("digest_seq", []), leg_docs["skin_b"].get("digest_seq", []))
                rd_eq = skin_a.get("render_digest") == leg_docs["skin_b"].get("render_digest")
                set_fact(
                    "determinism_double_run",
                    bit and rd_eq,
                    f"--skin on 双跑 digest_seq 位级一致={bit}（{len(skin_a.get('digest_seq', []))} 帧）render_digest 一致={rd_eq}（确定性门）",
                )
                # ── ⑦ 逐特性贡献 digest 区分（skin≠静态 / skin≠无skin全特性）──
                skin_seq = skin_a.get("digest_seq", [])
                ne_baseline = seqs_differ(skin_seq, leg_docs["baseline"].get("digest_seq", []))
                ne_noskin = seqs_differ(skin_seq, leg_docs["full_noskin"].get("digest_seq", []))
                set_fact(
                    "per_feature_digest_discrimination",
                    ne_baseline and ne_noskin,
                    f"逐特性贡献区分:skin≠baseline（静态）={ne_baseline} skin≠full_noskin（无蒙皮全特性）={ne_noskin}"
                    "（蒙皮贡献真实生效,防暗接线冒充）",
                )
                # ── ⑨ frame_ms measured（skin on/off + skin_gpu_ms 分项）──
                on_mean = skin_a.get("real_render_frame_ms")
                off_mean = leg_docs["full_noskin"].get("real_render_frame_ms")
                base_mean = leg_docs["baseline"].get("real_render_frame_ms")
                sk_gpu = (skin_a.get("stats") or {}).get("skin_gpu_ms", -1.0)
                sk_scene = (skin_a.get("stats") or {}).get("scene_gpu_ms", -1.0)
                set_fact(
                    "frame_ms_measured",
                    frame_ms_sane(on_mean, off_mean, base_mean, sk_gpu, sk_scene),
                    f"同机同窗 measured:skin_on={on_mean:.4f}ms skin_off(full)={off_mean:.4f}ms baseline={base_mean:.4f}ms"
                    f"；skin_gpu={sk_gpu:.4f}ms scene_gpu={sk_scene:.4f}ms（蒙皮逐帧更新成本分项登记）",
                )

    if degrade:
        doc = {
            "schema": "rurix.g34.skin_unified.skip.v1",
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
    skin_a_doc = leg_docs.get("skin_a") or {}
    skin_block = skin_a_doc.get("skin") or {}
    gap_block = skin_block.get("mv_gap") or {}
    rig_ok2, rig_active2, rig_dmax2 = rigid_wired(skin_block.get("verify_frames", []))
    last_digest_of = lambda n: ((leg_docs.get(n) or {}).get("digest_seq") or ["sha256:" + "0" * 64])[-1]
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "kernels": {
            "gi_skin_spv": {
                "path": str(SPV_GI_SKIN.relative_to(ROOT)).replace("\\", "/"),
                "sha256": "sha256:" + hashlib.sha256(SPV_GI_SKIN.read_bytes()).hexdigest() if SPV_GI_SKIN.is_file() else "sha256:" + "0" * 64,
            },
            "mv_spv": {
                "path": str(SPV_MV.relative_to(ROOT)).replace("\\", "/"),
                "sha256": "sha256:" + hashlib.sha256(SPV_MV.read_bytes()).hexdigest() if SPV_MV.is_file() else "sha256:" + "0" * 64,
            },
            "skin_spv": {
                "path": str(SPV_SKIN.relative_to(ROOT)).replace("\\", "/"),
                "sha256": "sha256:" + hashlib.sha256(SPV_SKIN.read_bytes()).hexdigest() if SPV_SKIN.is_file() else "sha256:" + "0" * 64,
            },
            "spirv_val_all": bool(facts["kernels_spv_valid"]["status"] == "PASS"),
            "mother_tracked_0byte": frozen_ok and worktree_ok,
            "frozen_snapshot": frozen_snapshot,
        },
        "skin_vertex_parity": {
            "frames": (skin_block.get("vertex_parity") or {}).get("frames", 0),
            "max_abs_max": (skin_block.get("vertex_parity") or {}).get("max_abs_max", -1.0),
            "all_bitexact": (skin_block.get("vertex_parity") or {}).get("all_bitexact", False),
        },
        "skin_verify": {
            "verify_count": skin_block.get("verify_count", 0),
            "all_pass": skin_block.get("all_pass", False),
            "centroid_tol_px": SKIN_TOL_CENTROID_PX,
            "aabb_tol_px": SKIN_TOL_AABB_PX,
            "motion_max_px": (skin_block.get("motion_gate") or {}).get("host_motion_max_px", -1.0),
            "char_tri_base": (skin_block.get("character") or {}).get("char_tri_base", 0),
            "verify_frames_file": f"harness evidence skin.verify_frames（{harness_archives[0] if harness_archives else 'n/a'}）",
        },
        "mv_gap": {
            "class2_rigid": gap_block.get("class2_rigid", ""),
            "class2_delta_max_px": rig_dmax2,
            "rigid_active_frames": rig_active2,
            "class1_delta_max_px": gap_block.get("class1_delta_max_px", -1.0),
            "class3_delta_max_px": gap_block.get("class3_delta_max_px", -1.0),
            "note": gap_block.get("note", ""),
        },
        "determinism": {
            "double_run_bitexact": seqs_bitexact(
                skin_a_doc.get("digest_seq", []),
                (leg_docs.get("skin_b") or {}).get("digest_seq", []),
            ),
            "frames": len(skin_a_doc.get("digest_seq", [])),
            "render_digest_a": skin_a_doc.get("render_digest", "sha256:" + "0" * 64),
            "render_digest_b": (leg_docs.get("skin_b") or {}).get("render_digest", "sha256:" + "0" * 64),
        },
        "per_feature": {
            "skin_last_digest": last_digest_of("skin_a"),
            "baseline_last_digest": last_digest_of("baseline"),
            "full_noskin_last_digest": last_digest_of("full_noskin"),
            "skin_ne_baseline": seqs_differ(skin_a_doc.get("digest_seq", []), (leg_docs.get("baseline") or {}).get("digest_seq", [])),
            "skin_ne_full_noskin": seqs_differ(skin_a_doc.get("digest_seq", []), (leg_docs.get("full_noskin") or {}).get("digest_seq", [])),
        },
        "regression_anchor": replay_doc if replay_doc else {
            "cell": ANCHOR_CELL, "fresh_digest": "sha256:" + "0" * 64,
            "anchor_digest": "sha256:" + "0" * 64, "match": False, "frames": 160, "warmup": 10,
        },
        "frame_ms": {
            "skin_on_mean": skin_a_doc.get("real_render_frame_ms", -1.0),
            "skin_off_mean": (leg_docs.get("full_noskin") or {}).get("real_render_frame_ms", -1.0),
            "skin_gpu_ms_mean": ((skin_a_doc.get("stats") or {}).get("skin_gpu_ms", -1.0)),
            "skin_scene_gpu_ms_mean": ((skin_a_doc.get("stats") or {}).get("scene_gpu_ms", -1.0)),
            "measured": "measured_local",
            "frames_per_run": frames,
        },
        "harness_evidence": harness_archives,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G34 全特性合流 G34-3 蒙皮角色进真窗口统一车道：蒙皮×纹理×slab×动态实例四特性同开——"
            "G34Full 27 SSBO 加性扩蒙皮七件（hit 通道/REST/WT/palette 双表/PREV/SKIN_PARAMS）= 36 资源六 pass"
            "（g31_skin 0-byte 复用 → blas_refit 桥角色 BLAS 2 逐帧 UPDATE → g34_unified_gi_skin〔G34-1 统一 "
            "kernel + out_hit + 角色实例分派〕→ g34_unified_mv〔g31_skin_mv 镜像 + 类 2 刚性实例臂——A4 登记 "
            "缺口统一车道蒙皮腿顺手接通,hit 通道 inst==1 像素 dev/host ≤2px 核验〕→ TSR 双 pass → display_encode）"
            "+ 逐帧 tlas_update refit 3 实例表;顺序入口 inflight=1。核验三面 = ① 逐顶点对拍 max_abs == 0 位级门"
            "（B5 在案口径）② 位置核验（质心 ≤4px/AABB ≤6px/计数门,B5 在案口径）③ MV 通道（类 3 ≤2px + 窗级真动门;"
            "类 1 静态区相机 MV 一致性 ≤2px——auto-move 动相机下 B5 静态绝对门诚实重述;类 2 ≤2px）;确定性双跑位级 + "
            "skin≠静态/skin≠无skin全特性区分 + frame_ms measured（skin on/off）。host 金标准全场景对拍 = null 诚实登记"
            "（蒙皮腿对拍 = ① 逐顶点臂承载;G34-1 冻结容差标定面 = 非蒙皮腿在案不混口径）。非蒙皮腿维持 g14_mv "
            "0-byte + 类 2 缺口登记（不冒充全局面接通）。g31_window_present.rs/g14_mv/g31_skin_mv/g34_unified_gi.rx "
            "0-byte——其门为回归锚。"
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
    gate_path = ROOT / "evidence" / f"g34_skin_unified_gate_{ts}.json"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}(+ harness {len(harness_archives)} 件)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂,无 GPU/无构建依赖）
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

    # 红绿臂①:逐顶点对拍位级判。
    expect(vertex_bitexact([{"vertex_max_abs": 0.0}, {"vertex_max_abs": 0.0}]),
           "GREEN:① 全帧位级正例")
    expect(not vertex_bitexact([{"vertex_max_abs": 0.0}, {"vertex_max_abs": 1e-7}]),
           "RED:① 非零 max_abs 必红")
    expect(not vertex_bitexact([]), "RED:① 空核验帧必红")
    expect(not vertex_bitexact([{"vertex_max_abs": "0.0"}]), "RED:① 字符串冒充数值必红")
    # 红绿臂②:位置核验判。
    good_row = {"centroid_delta_px": 1.5, "aabb_delta_px": 3.0, "obs_count": 900, "pass": True}
    expect(position_verified([good_row]), "GREEN:② 带内正例")
    expect(not position_verified([{**good_row, "centroid_delta_px": 4.1}]), "RED:② 质心超 4px 必红")
    expect(not position_verified([{**good_row, "aabb_delta_px": 6.1}]), "RED:② AABB 超 6px 必红")
    expect(not position_verified([{**good_row, "obs_count": 0}]), "RED:② 零检测像素必红")
    expect(not position_verified([{**good_row, "pass": False}]), "RED:② pass=false 必红")
    expect(not position_verified([]), "RED:② 空帧列必红")
    # 红绿臂③:MV 三臂判。
    good_mv = {"mv_median_delta_px": [1.2, 0.8], "static_mv_delta_px": [0.4, 0.3], "pass": True,
               "rigid_count": 300, "rigid_mv_delta_px": [0.9, 1.1]}
    expect(mv_wired([good_mv], 3.0), "GREEN:③ 带内正例")
    expect(not mv_wired([{**good_mv, "mv_median_delta_px": [2.1, 0.8]}], 3.0), "RED:③ 类3中位差超 2px 必红")
    expect(not mv_wired([{**good_mv, "static_mv_delta_px": [0.4, 2.1]}], 3.0), "RED:③ 静态一致性超 2px 必红")
    expect(not mv_wired([good_mv], 0.9), "RED:③ 窗级真动不足必红（动画冻结/MV 未载运动检出）")
    expect(not mv_wired([], 3.0), "RED:③ 空帧列必红")
    rig_ok, rig_n, rig_d = rigid_wired([good_mv])
    expect(rig_ok and rig_n == 1 and rig_d == 1.1, "GREEN:类2刚性带内正例（激活帧计数+窗级max）")
    expect(not rigid_wired([{**good_mv, "rigid_mv_delta_px": [2.1, 0.1]}])[0], "RED:类2中位差超 2px 必红")
    expect(not rigid_wired([{**good_mv, "rigid_count": 10}])[0], "RED:类2全帧低像素（<50）零激活必红")
    expect(not rigid_wired([])[0], "RED:类2空帧列必红")
    # 红绿臂④:digest 序列判。
    expect(seqs_bitexact(["a", "b"], ["a", "b"]), "GREEN:双跑位级正例")
    expect(not seqs_bitexact(["a", "b"], ["a", "x"]), "RED:双跑漂移必红")
    expect(not seqs_bitexact([], []), "RED:空序列必红")
    expect(seqs_differ(["a", "b"], ["a", "x"]), "GREEN:区分正例")
    expect(not seqs_differ(["a", "b"], ["a", "b"]), "RED:全同冒充特性生效必红")
    # 红绿臂⑤:Stage A 锚格判。
    d0 = "sha256:" + "a" * 64
    expect(anchor_match(d0, d0), "GREEN:锚位级 MATCH 正例")
    expect(not anchor_match(d0, "sha256:" + "b" * 64), "RED:锚 DRIFT 必红")
    expect(not anchor_match(None, d0), "RED:fresh 缺失必红")
    expect(not anchor_match("not-a-digest", d0), "RED:fresh 形态破必红")
    # 红绿臂⑥:frame_ms 健全判。
    expect(frame_ms_sane(3.5, 41.1, 6.6, 0.004), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(3.5, 0.0), "RED:0ms 必红")
    expect(not frame_ms_sane(3.5, float("nan")), "RED:NaN 必红")
    # schema 互核:双 schema 在树;gate schema facts enum == FACT_IDS;harness
    # schema const/required 互核（skin 块必需键 + features.skin + host_parity null）。
    expect(SCHEMA_PATH.is_file() and GATE_SCHEMA_PATH.is_file(), "双 schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
    if SCHEMA_PATH.is_file():
        hs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        req = hs.get("required", [])
        expect(all(k in req for k in ("features", "textures", "slab", "dyn", "skin", "host_parity")),
               "harness schema required 含 features/textures/slab/dyn/skin/host_parity")
        expect(hs["properties"]["schema"]["const"] == SCHEMA_ID, "harness schema const 互核")
        expect(hs["properties"]["gate"]["const"] == GATE_KEY, "harness schema gate const 互核")
        expect(hs["properties"]["features"]["properties"].get("skin", {}).get("const") is True,
               "harness schema features.skin const true 互核")
        expect(hs["properties"]["host_parity"].get("type") == "null",
               "harness schema host_parity null 诚实登记面互核")
        sk_req = hs["properties"]["skin"].get("required", [])
        expect(all(k in sk_req for k in ("vertex_parity", "verify_frames", "mv_gap", "motion_gate")),
               "harness schema skin 块必需键互核")
        import jsonschema as _js
        _js.Draft7Validator.check_schema(hs)
        _js.Draft7Validator.check_schema(gs)
        note("  ok   — 双 schema Draft7 合法（check_schema 绿）")
    expect(len(FACT_IDS) == 9, "facts 闭集 = 9")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=9；6 红臂组 + 正例组 + 双 schema 互核）")
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
            print(f"[{TAG}] FAIL: --frames {args.frames} < 32（确定性/区分/核验窗面下限）", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.warmup)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
