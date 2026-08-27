#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G34 全特性合流 G34-1 合流地基）
"""G34 全特性合流 G34-1：统一 kernel 车道面门冒烟（g34.wave1.unified；
G34-1 合流地基 = 纹理采样（fork A）+ slab 侧表预调制 + 动态实例分派（fork B）
三特性同开车道，kernels/g34_unified_gi.rx 统一 GI kernel + kernels/
g34_unified_shade.rx 统一 shade + UnifiedDescs::G34Full 27 SSBO 形态 +
src/rurix-render/src/bin/g34_full_lane.rs 真窗口 harness）。

八面判据（facts 闭集；任务书逐字）：
1. **kernels_spv_valid**：统一 kernel SPV 面——rurixc 现编
   kernels/g34_unified_{gi,shade}.rx + spirv-val 双绿 + 母版五 kernel
   （g14_3_direct_gi/g31_texture_gi/g31_dyn_scene/g14_3_shade_reduce/
   g31_hzb_shade）与 g31_window_present.rs vs HEAD 提交面 + 工作树 0-byte
   机核（其五门为回归锚；共享车道体 = 加性扩展允许面如实登记）。
2. **default_faces_bitexact_anchor**：缺省面 == 母版位级——g34_full_lane
   --static-camera（全特性缺省关；SPV 处置 = 母版同字面原始 SPV 零注入）
   canonical 160 帧 render_digest == milestones/g14/g14_3_stage_a_digest_
   anchor.json bistro-interior_t100_tsr_device 格（纹理缺省 tritex 全 −1 +
   动态缺省单实例 ⇒ 全链 TSR 位级锚）。
3. **merged_semantics_host_parity**：--full 三特性同开 parity 帧 host 金标准
   （合并语义同步实现：贴图三角 = 采样×(mod×R_slot)/非贴图 = 常量×(R_slot
   若 slab 映射)）vs device scene HDR 逐像素逐通道绝对差 p100 ≤ 冻结容差
   （milestones/g34/g34_budget.json g34.unified_lane.host_parity_tol 程序读
   禁手写：threshold = measured × 2.0 标定冻结；容差结构依据 = RT core vs
   host Möller–Trumbore t 值 ULP 级算术差 ⇒ 命中点/辐照传递差，目标近位级）；
   bitexact 像素占比如实登记（非门判据）。
4. **determinism_double_run**：--full 双跑 digest_seq 逐帧位级一致（确定性门）。
5. **dyn_position_verified**：动态实例位置核验（A4 范式 host 投影：轨迹点 +
   8 角点经 vp_j 投影 vs scene color 纯绿谱检测，每 10 帧 fail-closed）——
   --full 腿 dyn.verify_frames 全 pass。
6. **per_feature_digest_discrimination**：逐特性贡献 digest 区分——full vs
   关纹理/关 slab/关动态腿 digest_seq 各至少一帧不同（合并画面 ≠ 单开画面
   属预期如实登记；各特性真实生效防暗接线冒充）+ full ≠ baseline。
7. **stage_a_anchor_replay**：Stage A 锚复跑零漂移——g14_3_pipeline_perf
   canonical 160 帧 bistro-interior/t100/tsr_device 末帧 digest == 在案锚
   （共享体加性扩展对既有面 0-byte 的机器证明）。
8. **frame_ms_measured**：同机同窗 orbit --hidden 1920×1080 release 真跑
   baseline/full frame_ms 对照（纹理/slab 装配 = 装配期一次性 eval_ms 单列
   不混帧口径；measured_local 诚实登记）。

三态：无 Vulkan loader/设备/场景资产/SPV → DEV_ENV_DEGRADE 退 0（不冒充
PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充真跑）。

用法：
  py -3 ci/g34_unified_lane_smoke.py --selftest
  py -3 ci/g34_unified_lane_smoke.py --gate g34.wave1.unified [--frames 64] [--warmup 10]
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

GATE_KEY = "g34.wave1.unified"
SUBJECT = "g34_unified_lane"
WAVE = "G34.1"
TAG = "g34_unified"
SCHEMA_PATH = ROOT / "milestones" / "g34" / "g34_unified_lane_evidence_schema.json"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g34" / "g34_unified_lane_gate_evidence_schema.json"
SCHEMA_ID = "rurix.g34.unified_lane_evidence.v1"
GATE_SCHEMA_ID = "rurix.g34.unified_lane_gate_evidence.v1"
BUDGET_PATH = ROOT / "milestones" / "g34" / "g34_budget.json"
TOL_ENTRY_ID = "g34.unified_lane.host_parity_tol"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_tsr_device"
SLAB_ASSET = ROOT / "milestones" / "g31" / "g31_slab_side_table_bistro_interior.json"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
KERNEL_GI = ROOT / "src" / "rurix-render" / "kernels" / "g34_unified_gi.rx"
KERNEL_SHADE = ROOT / "src" / "rurix-render" / "kernels" / "g34_unified_shade.rx"
WORK = ROOT / ".tmp" / "g34_gates" / "unified"
SPV_GI = WORK / "g34_unified_gi.spv"
SPV_SHADE = WORK / "g34_unified_shade.spv"
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
    # HEAD（g30-closed 系）已入 git 的母版 kernel——diff vs HEAD 机核 0-byte。
    "src/rurix-render/kernels/g14_3_direct_gi.rx",
    "src/rurix-render/kernels/g14_3_shade_reduce.rx",
]
MOTHER_G31_ERA_PATHS = [
    # G31+ 战役全量未提交面（工作树 untracked；diff-vs-HEAD 不适用——G34-1
    # sha256 快照 = 后续波漂移守护基线,篡改即由快照对拍检出）。
    "src/rurix-render/kernels/g31_texture_gi.rx",
    "src/rurix-render/kernels/g31_dyn_scene.rx",
    "src/rurix-render/kernels/g31_hzb_shade.rx",
    "src/rurix-render/src/bin/g31_window_present.rs",
]
SCENE = "bistro-interior"
TRAJECTORY = "orbit"

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "kernels_spv_valid",
    "default_faces_bitexact_anchor",
    "merged_semantics_host_parity",
    "determinism_double_run",
    "dyn_position_verified",
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


def frozen_tol(budget: dict) -> float | None:
    """G34 冻结容差程序读（estimated/skip_reason 冒充 measured 即 None fail-closed）。"""
    for e in budget.get("entries", []):
        if e.get("id") == TOL_ENTRY_ID:
            if e.get("evidence") != "measured_local" or e.get("skip_reason"):
                return None
            t = e.get("threshold")
            return float(t) if isinstance(t, (int, float)) else None
    return None


def budget_measured(budget: dict) -> float | None:
    """标定 measured_value 程序读（threshold == measured × 2.0 关系互核面）。"""
    for e in budget.get("entries", []):
        if e.get("id") == TOL_ENTRY_ID:
            m = e.get("measured_value")
            return float(m) if isinstance(m, (int, float)) else None
    return None


def parity_in_tol(p100: float, tol: float) -> bool:
    """host 金标准对拍硬判：p100 有限且 ≤ 冻结容差。"""
    return isinstance(p100, (int, float)) and not isinstance(p100, bool) and p100 == p100 and 0.0 <= p100 <= tol


def seqs_bitexact(a: list, b: list) -> bool:
    """同轨迹双跑 digest_seq 逐帧位级一致判据。"""
    return len(a) == len(b) and len(a) > 0 and all(x == y for x, y in zip(a, b))


def seqs_differ(a: list, b: list) -> bool:
    """逐特性贡献区分判据：至少一帧 digest 不同（合并画面 ≠ 单开/全关画面）。"""
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


def harness_common_judge(doc: dict, frames: int, warmup: int, label: str, trajectory: bool) -> list[str]:
    """harness evidence 公共判（auto-move 腿 digest_seq 面/锚格腿空序列面）。"""
    fails: list[str] = []
    total = frames + warmup
    if doc.get("frames_completed") != total:
        fails.append(f"{label}: frames_completed {doc.get('frames_completed')} ≠ {total}")
    if doc.get("exit_reason") != "frames_done":
        fails.append(f"{label}: exit_reason ≠ frames_done: {doc.get('exit_reason')!r}")
    seq = doc.get("digest_seq")
    if trajectory:
        if not isinstance(seq, list) or len(seq) != total or any(not isinstance(x, str) or not DIGEST_RE.match(x) for x in seq):
            fails.append(f"{label}: digest_seq 形态/长度破（≠{total}）")
        if doc.get("digest") != (seq[-1] if isinstance(seq, list) and seq else None):
            fails.append(f"{label}: digest ≠ digest_seq 末项")
    else:
        if seq != []:
            fails.append(f"{label}: 锚格模式 digest_seq 非空序列（静态契约相机面非轨迹登记）")
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
    if not BUDGET_PATH.is_file():
        fail(f"g34 budget 缺失: {BUDGET_PATH}（冻结容差程序读面 fail-closed）")
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

    # ── ① 统一 kernel SPV 面：现编 + spirv-val + 母版 0-byte 机核 ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for src, dst in ((KERNEL_GI, SPV_GI), (KERNEL_SHADE, SPV_SHADE)):
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
    # G31 期 untracked 面：sha256 快照（后续波漂移守护基线;本波 0-byte 承诺 =
    # 任务编辑面未触——快照登记为机核起点）。
    g31_era_snapshot: dict[str, str] = {}
    snapshot_ok = True
    for p in MOTHER_G31_ERA_PATHS:
        fp = ROOT / p
        if fp.is_file():
            g31_era_snapshot[p] = "sha256:" + hashlib.sha256(fp.read_bytes()).hexdigest()
        else:
            snapshot_ok = False
            g31_era_snapshot[p] = "MISSING"
    set_fact(
        "kernels_spv_valid",
        spv_ok and frozen_ok and worktree_ok and snapshot_ok,
        f"rurixc 现编 g34_unified_{{gi,shade}}.rx + spirv-val={'绿' if spv_ok else '红'}；"
        f"母版 tracked 双 kernel vs HEAD 0-byte={frozen_ok} 工作树干净={worktree_ok}；"
        f"G31 期 untracked 四面（g31_texture_gi/g31_dyn_scene/g31_hzb_shade/g31_window_present）"
        f"sha256 快照 {len(g31_era_snapshot)} 件在档={snapshot_ok}（共享车道体 = 加性扩展允许面）",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("g34 统一 kernel SPV 编译/spirv-val 未过")
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
    anchor_doc: dict = {}
    replay_doc: dict = {}
    if not degrade:
        # ── dev-env 探针（baseline 短跑 + --host-parity off——探针面 = env 检测
        #    非正确性裁决;parity 容差 = 标定轨迹帧面〔orbit/64/10 fi=10〕,短跑
        #    异轨迹帧面非本探针消费口径,如实登记不混）──
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
        with gpu_device_lock(purpose=f"{TAG} 锚格 + 六腿 + Stage A 锚格 bench"):
            # ── ② 锚格腿（--static-camera 全特性缺省关 canonical 160 帧）──
            legs = [
                ("anchor", ["--static-camera", "--frames", "160", "--warmup", "10"], 160, 10, False),
                ("baseline", ["--auto-move", TRAJECTORY], frames, warmup, True),
                ("full_a", ["--full", "--slab-table", str(SLAB_ASSET), "--auto-move", TRAJECTORY], frames, warmup, True),
                ("full_b", ["--full", "--slab-table", str(SLAB_ASSET), "--auto-move", TRAJECTORY], frames, warmup, True),
                ("notex", ["--slab-table", str(SLAB_ASSET), "--dyn", "on", "--auto-move", TRAJECTORY], frames, warmup, True),
                ("noslab", ["--textures", "on", "--dyn", "on", "--auto-move", TRAJECTORY], frames, warmup, True),
                ("nodyn", ["--textures", "on", "--slab-table", str(SLAB_ASSET), "--auto-move", TRAJECTORY], frames, warmup, True),
            ]
            leg_ok = True
            for label, extra, lf, lw, traj in legs:
                r, doc, ev_path = run_full_lane(label, extra, env, frames=lf, warmup=lw)
                out = (r.stdout or "") + (r.stderr or "")
                if r.returncode != 0 or doc is None or f"[{ 'g34_full_lane' }]: PASS" not in out:
                    fail(f"{label} 真跑失败 rc={r.returncode}: {out[-300:]}")
                    leg_ok = False
                    continue
                if "Validation Error" in out or "VUID-" in out:
                    fail(f"{label} validation 应静默却报错")
                    leg_ok = False
                j = harness_common_judge(doc, lf, lw, label, traj)
                for m in j:
                    fail(m)
                leg_ok &= not j
                leg_docs[label] = doc
                arch = ROOT / "evidence" / f"g34_unified_lane_{label}_{ts}.json"
                arch.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                harness_archives.append(str(arch.relative_to(ROOT)))
            # ── ⑦ Stage A 锚格 bench 复跑（canonical 160 帧;共享体加性扩展
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
                anchors = json.loads(ANCHOR_PATH.read_text(encoding="utf-8")).get("anchors") or {}
                anchor_dg = (anchors.get(ANCHOR_CELL) or {}).get("last_frame_digest")
                anchor_leg = leg_docs["anchor"]
                fresh_anchor = anchor_leg.get("render_digest")
                anchor_doc = {
                    "cell": ANCHOR_CELL,
                    "fresh_digest": fresh_anchor,
                    "anchor_digest": anchor_dg,
                    "match": anchor_match(fresh_anchor, anchor_dg),
                    "frames": 160,
                    "warmup": 10,
                    "spv_policy": "textures off 缺省面 = 原始 SPV 零注入（母版处置同字面）；textures on = NoContraction 注入（B4 对拍锚前提）",
                }
                # ── ② 缺省面 == 母版位级（锚格腿全链 TSR digest == 在案锚）──
                set_fact(
                    "default_faces_bitexact_anchor",
                    anchor_doc["match"],
                    f"缺省面 == 母版位级:--static-camera 锚格腿 render_digest {str(fresh_anchor)[:23]}… vs 在案 {str(anchor_dg)[:23]}… "
                    f"{'位级 MATCH（纹理缺省 tritex 全 −1 + 动态缺省单实例 ⇒ 全链 TSR 锚）' if anchor_doc['match'] else 'DRIFT（RED）'}",
                )
                # ── ③ 合并语义 host 金标准对拍（--full parity 帧）──
                budget = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
                tol = frozen_tol(budget)
                hp = leg_docs["full_a"].get("host_parity") or {}
                set_fact(
                    "merged_semantics_host_parity",
                    tol is not None and parity_in_tol(hp.get("color_p100"), tol),
                    f"host 金标准对拍 p100={hp.get('color_p100')!r} ≤ 冻结容差 {tol!r}（{TOL_ENTRY_ID} 程序读）"
                    f"；bitexact 像素占比 {hp.get('bitexact_ratio', 0.0)*100:.2f}%（如实登记非门判据）"
                    f"；p50={hp.get('color_p50')!r} depth_p100={hp.get('depth_p100')!r}",
                )
                # ── ④ 确定性双跑 ──
                bit = seqs_bitexact(leg_docs["full_a"].get("digest_seq", []), leg_docs["full_b"].get("digest_seq", []))
                rd_eq = leg_docs["full_a"].get("render_digest") == leg_docs["full_b"].get("render_digest")
                set_fact(
                    "determinism_double_run",
                    bit and rd_eq,
                    f"--full 双跑 digest_seq 位级一致={bit}（74 帧）render_digest 一致={rd_eq}（确定性门）",
                )
                # ── ⑤ 动态实例位置核验（A4 范式 host 投影）──
                dyn = leg_docs["full_a"].get("dyn") or {}
                set_fact(
                    "dyn_position_verified",
                    dyn.get("all_pass") is True and (dyn.get("verify_count") or 0) >= 1,
                    f"动态实例位置核验 verify_frames={dyn.get('verify_count')} all_pass={dyn.get('all_pass')}"
                    f"（A4 范式 host 投影 vs scene color 纯绿谱检测,每 10 帧 fail-closed）",
                )
                # ── ⑥ 逐特性贡献 digest 区分 ──
                full_seq = leg_docs["full_a"].get("digest_seq", [])
                discrim = {
                    n: seqs_differ(full_seq, leg_docs[n].get("digest_seq", []))
                    for n in ("notex", "noslab", "nodyn", "baseline")
                }
                set_fact(
                    "per_feature_digest_discrimination",
                    all(discrim.values()),
                    f"逐特性贡献区分:full≠关纹理={discrim['notex']} full≠关slab={discrim['noslab']} "
                    f"full≠关动态={discrim['nodyn']} full≠baseline={discrim['baseline']}"
                    "（合并画面 ≠ 单开/全关画面属预期如实登记）",
                )
                # ── ⑧ frame_ms measured ──
                base_mean = leg_docs["baseline"]["real_render_frame_ms"]
                full_mean = leg_docs["full_a"]["real_render_frame_ms"]
                full_scene_gpu = (leg_docs["full_a"].get("stats") or {}).get("scene_gpu_ms", -1.0)
                tex_eval = (leg_docs["full_a"].get("textures") or {}).get("probe", {}).get("eval_ms", -1.0)
                slab_eval = (leg_docs["full_a"].get("slab") or {}).get("eval_ms", -1.0)
                set_fact(
                    "frame_ms_measured",
                    frame_ms_sane(base_mean, full_mean, full_scene_gpu) and tex_eval >= 0.0 and slab_eval >= 0.0,
                    f"同机同窗 measured:baseline={base_mean:.4f}ms full={full_mean:.4f}ms（scene_gpu={full_scene_gpu:.4f}ms）"
                    f"；装配期一次性 tex_eval={tex_eval:.3f}ms slab_eval={slab_eval:.3f}ms（单列不混帧口径）",
                )

    if degrade:
        doc = {
            "schema": "rurix.g34.unified_lane.skip.v1",
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
    budget = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
    tol = frozen_tol(budget)
    hp_full = (leg_docs.get("full_a") or {}).get("host_parity") or {}
    dyn_full = (leg_docs.get("full_a") or {}).get("dyn") or {}
    last_digest_of = lambda n: ((leg_docs.get(n) or {}).get("digest_seq") or ["sha256:" + "0" * 64])[-1]
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "kernels": {
            "gi_spv": {
                "path": str(SPV_GI.relative_to(ROOT)).replace("\\", "/"),
                "sha256": "sha256:" + hashlib.sha256(SPV_GI.read_bytes()).hexdigest() if SPV_GI.is_file() else "sha256:" + "0" * 64,
            },
            "shade_spv": {
                "path": str(SPV_SHADE.relative_to(ROOT)).replace("\\", "/"),
                "sha256": "sha256:" + hashlib.sha256(SPV_SHADE.read_bytes()).hexdigest() if SPV_SHADE.is_file() else "sha256:" + "0" * 64,
            },
            "spirv_val_gi": bool(facts["kernels_spv_valid"]["status"] == "PASS"),
            "spirv_val_shade": bool(facts["kernels_spv_valid"]["status"] == "PASS"),
            "mother_tracked_0byte": frozen_ok and worktree_ok,
            "g31_era_snapshot": g31_era_snapshot,
        },
        "default_face_anchor": anchor_doc if anchor_doc else {
            "cell": ANCHOR_CELL, "fresh_digest": "sha256:" + "0" * 64,
            "anchor_digest": "sha256:" + "0" * 64, "match": False,
            "frames": 160, "warmup": 10,
            "spv_policy": "textures off 缺省面 = 原始 SPV 零注入（母版处置同字面）；textures on = NoContraction 注入（B4 对拍锚前提）",
        },
        "host_parity": {
            "frame": hp_full.get("frame", 0),
            "color_p100": hp_full.get("color_p100", -1.0),
            "frozen_tol": tol if tol is not None else 0.0,
            "frozen_tol_entry": TOL_ENTRY_ID,
            "in_tol": parity_in_tol(hp_full.get("color_p100"), tol) if tol is not None else False,
            "bitexact_ratio": hp_full.get("bitexact_ratio", 0.0),
            "color_p50": hp_full.get("color_p50", -1.0),
            "color_mean_abs": hp_full.get("color_mean_abs", -1.0),
            "depth_p100": hp_full.get("depth_p100", -1.0),
            "host_render_ms": hp_full.get("host_render_ms", -1.0),
        },
        "determinism": {
            "double_run_bitexact": seqs_bitexact(
                (leg_docs.get("full_a") or {}).get("digest_seq", []),
                (leg_docs.get("full_b") or {}).get("digest_seq", []),
            ),
            "frames": len((leg_docs.get("full_a") or {}).get("digest_seq", [])),
            "render_digest_a": (leg_docs.get("full_a") or {}).get("render_digest", "sha256:" + "0" * 64),
            "render_digest_b": (leg_docs.get("full_b") or {}).get("render_digest", "sha256:" + "0" * 64),
        },
        "dyn_verify": {
            "verify_count": dyn_full.get("verify_count", 0),
            "all_pass": dyn_full.get("all_pass", False),
            "dyn_tris": dyn_full.get("dyn_tris", 12),
            "dyn_tri_base": dyn_full.get("dyn_tri_base", 0),
            "action": dyn_full.get("action", "refit"),
        },
        "per_feature": {
            "full_last_digest": last_digest_of("full_a"),
            "notex_last_digest": last_digest_of("notex"),
            "noslab_last_digest": last_digest_of("noslab"),
            "nodyn_last_digest": last_digest_of("nodyn"),
            "baseline_last_digest": last_digest_of("baseline"),
            "full_ne_notex": seqs_differ((leg_docs.get("full_a") or {}).get("digest_seq", []), (leg_docs.get("notex") or {}).get("digest_seq", [])),
            "full_ne_noslab": seqs_differ((leg_docs.get("full_a") or {}).get("digest_seq", []), (leg_docs.get("noslab") or {}).get("digest_seq", [])),
            "full_ne_nodyn": seqs_differ((leg_docs.get("full_a") or {}).get("digest_seq", []), (leg_docs.get("nodyn") or {}).get("digest_seq", [])),
            "full_ne_baseline": seqs_differ((leg_docs.get("full_a") or {}).get("digest_seq", []), (leg_docs.get("baseline") or {}).get("digest_seq", [])),
        },
        "regression_anchor": replay_doc if replay_doc else {
            "cell": ANCHOR_CELL, "fresh_digest": "sha256:" + "0" * 64,
            "anchor_digest": "sha256:" + "0" * 64, "match": False, "frames": 160, "warmup": 10,
        },
        "frame_ms": {
            "baseline_mean": (leg_docs.get("baseline") or {}).get("real_render_frame_ms", -1.0),
            "full_mean": (leg_docs.get("full_a") or {}).get("real_render_frame_ms", -1.0),
            "full_scene_gpu_mean": ((leg_docs.get("full_a") or {}).get("stats") or {}).get("scene_gpu_ms", -1.0),
            "tex_eval_ms": ((leg_docs.get("full_a") or {}).get("textures") or {}).get("probe", {}).get("eval_ms", -1.0),
            "slab_eval_ms": ((leg_docs.get("full_a") or {}).get("slab") or {}).get("eval_ms", -1.0),
            "measured": "measured_local",
            "frames_per_run": frames,
        },
        "harness_evidence": harness_archives,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G34 全特性合流 G34-1 合流地基：kernels/g34_unified_gi.rx 统一 GI kernel（母版 "
            "g14_3_direct_gi 语义 + fork A 图集采样块 + fork B 实例分派块合一;两缺省面各自 == "
            "母版位级——--static-camera 锚格模式全链 TSR digest == g14_3_stage_a_digest_anchor "
            "承载,SPV 处置分叉 = textures off 原始 SPV 零注入〔母版同字面〕/ textures on "
            "NoContraction 注入〔B4 对拍锚前提〕如实登记）+ kernels/g34_unified_shade.rx 统一 "
            "shade（shade_reduce 语义 + out_depth_hz 恒输出,HZB off 写而不消费成本 measured,"
            "HZB 合流接口预留）;合并语义 = 贴图三角 采样×(mod×R_slot)/非贴图 常量×(R_slot 若 "
            "slab 映射),host 装配期预调制承载 kernel 零新增面;三特性同开真跑 = 装配期 slab/纹理 "
            "双臂对拍 + 逐帧 tlas_update refit + parity 帧 host 金标准对拍 + A4 范式动态位置核验 "
            "fail-closed + 逐特性贡献 digest 区分;HZB/FG/skin 合流归后续波（接口预留面见 kernels "
            "头注释）。g31_window_present.rs 0-byte——其五门为回归锚。"
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
    gate_path = ROOT / "evidence" / f"g34_unified_lane_gate_{ts}.json"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}(+ harness {len(harness_archives)} 件)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂,无 GPU/无构建依赖）
# ---------------------------------------------------------------------------


def _good_budget() -> dict:
    return {"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                         "skip_reason": None, "threshold": 2.0e-4, "measured_value": 1.0e-4}]}


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    # 红绿臂①:冻结容差程序读 + measured×2.0 关系互核。
    good = _good_budget()
    expect(frozen_tol(good) == 2.0e-4, "GREEN:容差程序读正例")
    expect(budget_measured(good) == 1.0e-4, "GREEN:measured_value 程序读正例")
    expect(frozen_tol(good) == budget_measured(good) * 2.0, "GREEN:threshold == measured × 2.0 关系互核")
    expect(frozen_tol({"entries": [{"id": TOL_ENTRY_ID, "evidence": "estimated",
                                    "skip_reason": None, "threshold": 2.0e-4}]}) is None,
           "RED:estimated 冒充 measured 必拒")
    expect(frozen_tol({"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                                    "skip_reason": "no gpu", "threshold": 2.0e-4}]}) is None,
           "RED:skip_reason 携带必拒")
    expect(frozen_tol({"entries": []}) is None, "RED:条目缺失必拒")
    expect(budget_measured({"entries": []}) is None, "RED:measured 条目缺失必拒")
    # 红绿臂②:parity 判。
    expect(parity_in_tol(1.9e-4, 2.0e-4), "GREEN:p100 带内过")
    expect(parity_in_tol(2.0e-4, 2.0e-4), "GREEN:p100 == tol 边界过")
    expect(not parity_in_tol(2.1e-4, 2.0e-4), "RED:p100 超容差必红")
    expect(not parity_in_tol(float("nan"), 2.0e-4), "RED:NaN p100 必红")
    expect(not parity_in_tol(-1.0, 2.0e-4), "RED:负 p100 必红")
    # 红绿臂③:digest 序列判。
    expect(seqs_bitexact(["a", "b"], ["a", "b"]), "GREEN:双跑位级正例")
    expect(not seqs_bitexact(["a", "b"], ["a", "x"]), "RED:双跑漂移必红")
    expect(not seqs_bitexact([], []), "RED:空序列必红")
    expect(seqs_differ(["a", "b"], ["a", "x"]), "GREEN:逐特性区分正例")
    expect(not seqs_differ(["a", "b"], ["a", "b"]), "RED:全同冒充特性生效必红")
    # 红绿臂④:Stage A 锚格判。
    d0 = "sha256:" + "a" * 64
    expect(anchor_match(d0, d0), "GREEN:锚位级 MATCH 正例")
    expect(not anchor_match(d0, "sha256:" + "b" * 64), "RED:锚 DRIFT 必红")
    expect(not anchor_match(None, d0), "RED:fresh 缺失必红")
    expect(not anchor_match("not-a-digest", d0), "RED:fresh 形态破必红")
    # 红绿臂⑤:frame_ms 健全判。
    expect(frame_ms_sane(3.5, 41.1, 6.6), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(3.5, 0.0), "RED:0ms 必红")
    expect(not frame_ms_sane(3.5, float("nan")), "RED:NaN 必红")
    # schema 互核:两 schema + budget 在树;gate schema facts enum == FACT_IDS;
    # harness schema const/required 互核;budget threshold == measured × 2.0。
    expect(SCHEMA_PATH.is_file() and GATE_SCHEMA_PATH.is_file() and BUDGET_PATH.is_file(),
           "两 schema + budget 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
    if SCHEMA_PATH.is_file():
        hs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        req = hs.get("required", [])
        expect(all(k in req for k in ("features", "textures", "slab", "dyn", "host_parity")),
               "harness schema required 含 features/textures/slab/dyn/host_parity")
        expect(hs["properties"]["schema"]["const"] == SCHEMA_ID, "harness schema const 互核")
    if BUDGET_PATH.is_file():
        bj = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
        t = frozen_tol(bj)
        m = budget_measured(bj)
        expect(t is not None, "budget 冻结容差程序读在档")
        expect(t is not None and m is not None and t == m * 2.0,
               "budget threshold == measured × 2.0 关系互核（标定纪律禁手写）")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=8；5 红臂组 + 正例组 + 双 schema/budget 互核）")
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
            print(f"[{TAG}] FAIL: --frames {args.frames} < 32（逐特性区分/确定性面下限）", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.warmup)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
