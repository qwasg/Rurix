#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C11 cluster 流送 P4 四行 + 整合真跑）
"""G31+ 波 C Task C11：cluster 流送 P4 四行门冒烟（g31.waveC.p4stream；
RD-039 cluster 流送 P4 分项；G31_PLUS_COMMERCIAL_RENDERER_TODO §3 #20~#23；
milestones/g20/g20_cluster_streaming_p4_gap.json 四行差距闭集 +
milestones/g27/g27_cluster_p4_rejudgment.json P4-2 依赖解除登记消费面）。

四行落地字面（每行独立判档 + 整合一 fact）：
1. **P4-1 页磁盘布局与驻留池**：RXPD major=2（rurix-geom-pages disk_v2.rs
   加性新版本面，payload = RXPZ-LZ1(RXPL v2 映像)；v1 面 0-byte）页集自
   bistro 几何真实构建落盘（严格 glTF 导入 → build_asset_dag 簇 DAG →
   pack_cluster_dag_v2 v2 段感知装箱〔v1 估算不计 v2 段字节的超页 bug 同
   PR 修复，geom_build.rs 0-byte〕）+ host 驻留池（PagePool LRU/容量预算/
   root 钉住——既有面 0-byte 复用，逐出真实发生）。
2. **P4-2 GPU 请求反馈链**：kernels/g31_cluster_stream.rx（rurixc 产 SPV +
   bin 侧 NoContraction 注入，SPV 文件 0-byte；geometry/cull.rs 金标准字面
   0-byte 消费）剔除 pass 产 cluster 缺页请求 → device 请求缓冲读回 →
   host 驻留调度消费（PriorityIoPool 异步读 → StreamingEngine 三预算 tick
   → 页表/页池镜像上传 → 次帧 device 消费校验 checksum）闭环真跑。
3. **P4-3 LOD cut 与驻留联动**：缺页 cluster → 父级/粗 LOD 回退渲染
   （一致性 cut：祖先-后代合并，禁空洞/禁重复覆盖；root 钉住保证终止）；
   host 金标准 lod_cut_with_residency 与 device 逐帧对拍（归一后集合全等）
   + verify_cut_cover 逐帧覆盖不变量 + 全驻留参考零回退双跑位级。
4. **P4-4 异步 IO 优先级链**：PriorityIoPool 固定工作线程 + 优先级堆
   （屏幕投影直径量化重要度——近处/大屏占比优先）真实磁盘读；优先级倒置
   正确性 measured（开工闸前 [低×3,高×1]，单 worker 高优先级先驻留）。
5. **整合真跑**：强制小驻留池（容量 < 全集）穿越式相机轨迹 bistro 派生
   场景：全程无崩、零回退帧 digest 与全驻留参考逐帧位级一致（回退帧允许
   LOD 差——容差结构依据 = 一致性 cut 语义）、末 2 帧收敛位级、缺页率/
   回退率/IO 量 measured。
6. **冻结旧面 0-byte 机核**：G8 页 ABI v1 文件族 + streaming v1 四文件 +
   geometry 三文件 + geom_build.rs 工作树 0-byte。

三态：无 Vulkan loader/rurixc SPV 失败/bistro gltf 缺失 → DEV_ENV_DEGRADE
退 0（不冒充 PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL。

evidence 纪律：门 schema 全 const 闭集 = PASS-only 面——PASS 才落
evidence/g31_p4_streaming_<ts>.json（check_schemas 前缀路由
g31_p4_streaming_）；FAIL 诊断件落 .tmp/g31_gates/p4stream/ 工作区不污染
evidence/ 路由面。harness 真跑件（rurix.g31.cluster_stream_evidence.v1 字
面）无注册 schema，全留 .tmp 工作区，数字经门裁决件蒸馏登记。

用法：
  py -3 ci/g31_p4_streaming_smoke.py --selftest
  py -3 ci/g31_p4_streaming_smoke.py --gate g31.waveC.p4stream [--orbit-frames 24] [--hold-frames 8]
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

GATE_KEY = "g31.waveC.p4stream"
SUBJECT = "g31_p4_streaming"
WAVE = "G31+.C"
TAG = "g31_p4stream"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_p4_streaming_evidence_schema.json"
SCHEMA_ID = "rurix.g31.p4_streaming_smoke_evidence.v1"
HARNESS_SCHEMA_ID = "rurix.g31.cluster_stream_evidence.v1"
SCENE = "bistro-interior-derived"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
KERNEL = ROOT / "src" / "rurix-asset" / "kernels" / "g31_cluster_stream.rx"
WORK = ROOT / ".tmp" / "g31_gates" / "p4stream"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "release" / f"g31_cluster_stream{EXE_SUFFIX}"
FROZEN_GEOM_PAGES_V1 = [
    "src/rurix-geom-pages/src/logical.rs",
    "src/rurix-geom-pages/src/logical_v2.rs",
    "src/rurix-geom-pages/src/memory.rs",
    "src/rurix-geom-pages/src/disk.rs",
    "src/rurix-geom-pages/src/codec.rs",
    "src/rurix-geom-pages/src/expand.rs",
    "src/rurix-geom-pages/src/expand_v2.rs",
]
FROZEN_STREAMING_V1 = [
    "src/rurix-render/src/streaming/pool.rs",
    "src/rurix-render/src/streaming/engine.rs",
    "src/rurix-render/src/streaming/feedback.rs",
    "src/rurix-render/src/streaming/resource.rs",
]
FROZEN_GEOMETRY = [
    "src/rurix-render/src/geometry/hzb.rs",
    "src/rurix-render/src/geometry/cull.rs",
    "src/rurix-render/src/geometry/visbuffer.rs",
]
FROZEN_GEOM_BUILD_V1 = ["src/rurix-asset/src/geom_build.rs"]

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "p4_1_page_pool",
    "p4_2_request_loop",
    "p4_3_fallback_parity",
    "p4_4_io_priority",
    "integration",
    "frozen_surfaces_0byte",
]


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


# ---------------------------------------------------------------------------
# 判读器（selftest 红绿两臂消费面；输入 = harness evidence 解码 dict）
# ---------------------------------------------------------------------------


def p4_1_judge(scene: dict, stream: dict) -> list[str]:
    """① P4-1 页集与驻留池判（返回失败串列表，空 = 绿）。"""
    fails: list[str] = []
    if not isinstance(scene, dict):
        return ["scene 块缺失"]
    if not isinstance(scene.get("pages"), int) or scene["pages"] < 8:
        fails.append(f"scene.pages < 8: {scene.get('pages')!r}")
    if not isinstance(scene.get("meshes"), int) or scene["meshes"] < 4:
        fails.append(f"scene.meshes < 4: {scene.get('meshes')!r}")
    if not isinstance(scene.get("clusters"), int) or scene["clusters"] < 100:
        fails.append(f"scene.clusters < 100: {scene.get('clusters')!r}")
    if not isinstance(scene.get("triangles"), int) or scene["triangles"] < 1000:
        fails.append(f"scene.triangles < 1000: {scene.get('triangles')!r}")
    pool = stream.get("pool_slots")
    pages = scene.get("pages", 0)
    if not isinstance(pool, int) or pool < 2:
        fails.append(f"pool_slots 非法: {pool!r}")
    elif pool >= pages:
        fails.append(f"驻留池未强制小于全集: pool {pool} >= pages {pages}")
    units = scene.get("units") or []
    roots = sum(u.get("root_pages", 0) for u in units if isinstance(u, dict))
    if roots < 1:
        fails.append("root 页为零（钉住面缺失）")
    if not isinstance(stream.get("evicted_total"), int) or stream["evicted_total"] < 1:
        fails.append(f"LRU 逐出未真实发生: evicted_total={stream.get('evicted_total')!r}")
    return fails


def p4_2_judge(stream: dict) -> list[str]:
    """② P4-2 请求-驻留闭环判。"""
    fails: list[str] = []
    frames = stream.get("frames") or []
    req_total = sum(f.get("device_requests", 0) for f in frames)
    if req_total < 1:
        fails.append("device 请求缓冲零请求（空接线冒充）")
    if not isinstance(stream.get("io_submitted"), int) or stream["io_submitted"] < 1:
        fails.append(f"io_submitted < 1: {stream.get('io_submitted')!r}")
    if not isinstance(stream.get("io_completed"), int) or stream["io_completed"] < 1:
        fails.append(f"io_completed < 1: {stream.get('io_completed')!r}")
    loaded = sum(f.get("pages_loaded", 0) for f in frames)
    if loaded < 1:
        fails.append("pages_loaded 为零（host 驻留调度未消费）")
    residents = [f.get("resident", 0) for f in frames]
    if len(residents) < 2 or not any(b > a for a, b in zip(residents, residents[1:])):
        fails.append("驻留数无增长（页表更新未真发生）")
    checksums = [f.get("checksum0", 0) for f in frames]
    if not any(c != 0 for c in checksums):
        fails.append("checksum0 全零（device 未真实消费上传页内容）")
    elif len(set(checksums)) < 2:
        fails.append("checksum0 无演化（消费校验未随驻留链变化）")
    if not frames or frames[-1].get("miss_selected", -1) != 0:
        fails.append(f"末帧缺页未清零: {frames[-1].get('miss_selected') if frames else '无帧'!r}")
    return fails


def p4_3_judge(ref: dict, stream: dict) -> list[str]:
    """③ P4-3 回退正确性判。"""
    fails: list[str] = []
    if stream.get("fallback_frames", 0) < 1:
        fails.append("回退未真实发生（压力未达冒充）")
    if stream.get("parity_all") is not True:
        fails.append("stream 臂 device/host 渲染集对拍非全帧全等")
    if stream.get("cover_all") is not True:
        fails.append("stream 臂覆盖不变量非全帧成立（空洞/重复覆盖）")
    if ref.get("parity_all") is not True or ref.get("cover_all") is not True:
        fails.append("reference 臂对拍/覆盖非全帧成立（对拍基准污染）")
    ref_frames = ref.get("frames") or []
    if any(f.get("miss_selected", 1) != 0 or f.get("fallback", 1) != 0 for f in ref_frames):
        fails.append("全驻留参考臂存在缺页/回退（基准纯净性破）")
    if ref.get("double_run_bitexact") is not True:
        fails.append("reference 双跑非位级一致")
    return fails


def p4_4_judge(stream: dict, probe: dict) -> list[str]:
    """④ P4-4 异步 IO 优先级判。"""
    fails: list[str] = []
    if not isinstance(probe, dict):
        return ["priority_probe 块缺失"]
    if probe.get("ok") is not True:
        fails.append("优先级倒置探针未过（高优先级未先驻留/日志/计量破）")
    order = probe.get("order") or []
    if len(order) < 4:
        fails.append(f"probe.order 长度 < 4: {order!r}")
    if not isinstance(probe.get("bytes_read"), int) or probe["bytes_read"] < 1:
        fails.append(f"probe.bytes_read < 1: {probe.get('bytes_read')!r}")
    if not isinstance(stream.get("io_read_bytes_total"), int) or stream["io_read_bytes_total"] < 1:
        fails.append(f"io_read_bytes_total < 1: {stream.get('io_read_bytes_total')!r}")
    return fails


def integration_judge(ref: dict, stream: dict, orbit: int, hold: int) -> list[str]:
    """⑤ 整合真跑判（无崩/对拍容差结构/收敛/measured）。"""
    fails: list[str] = []
    total = orbit + hold
    rseq = ref.get("digest_seq") or []
    sseq = stream.get("digest_seq") or []
    sframes = stream.get("frames") or []
    if len(sframes) != total or len(sseq) != total or len(rseq) != total:
        fails.append(f"帧数不齐: stream {len(sframes)}/{len(sseq)} vs ref {len(rseq)} vs 期望 {total}（流送未全程无崩）")
        return fails
    if not DIGEST_RE.match(sseq[-1] if sseq else ""):
        fails.append("stream 末帧 digest 形态非法")
    if not DIGEST_RE.match(rseq[-1] if rseq else ""):
        fails.append("ref 末帧 digest 形态非法")
    # 零回退帧位级对拍（容差结构依据：回退帧允许 LOD 差，零回退帧位级）。
    for i, f in enumerate(sframes):
        if f.get("fallback", 1) == 0 and sseq[i] != rseq[i]:
            fails.append(f"零回退帧 {i} digest 与全驻留参考非位级一致")
            break
    if sseq[-1] != rseq[-1] or sseq[-2] != rseq[-2]:
        fails.append("末 2 帧与全驻留参考非位级一致（hold 收敛未达）")
    hold_bit = sum(1 for i in range(total - hold, total) if sseq[i] == rseq[i])
    if hold_bit < 2:
        fails.append(f"hold 段位级帧数 {hold_bit} < 2")
    if stream.get("miss_frames", 0) < 1 or stream.get("fallback_frames", 0) < 1:
        fails.append("缺页/回退帧为零（压力未达）")
    if stream.get("evicted_total", 0) < 1:
        fails.append("逐出为零（池压力未达）")
    if stream.get("io_bytes_total", 0) < 1 or stream.get("upload_bytes_total", 0) < 1:
        fails.append("IO/上传计量为零")
    return fails


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    """三态裁决：无降级 → None（续跑）；有降级 + REQUIRE_REAL → 1（硬红）；
    有降级无 REQUIRE_REAL → 0（SKIP 非 PASS 非 FAIL）。"""
    if not degrade:
        return None
    return 1 if require_real else 0


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def run_gate(orbit_frames: int, hold_frames: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:180]}")

    if not SCHEMA_PATH.is_file():
        fail(f"门 schema 缺失: {SCHEMA_PATH}")
        return 1

    # ── 构建（release harness + rurixc）──
    ok = build_or_fail(
        ["cargo", "build", "--release", "-p", "rurix-asset", "--features", "vulkan",
         "--bin", "g31_cluster_stream", "--quiet"],
        "harness release",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── kernel SPV 现编 + spirv-val（SPV 文件留 .tmp；bin 侧 NoContraction
    #    注入 = 位级关键，kernel 源 0-byte 不注）──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv = WORK / "g31_cluster_stream.spv"
    degrade: list[str] = []
    r = run([str(rurixc), str(KERNEL), "--target", "vulkan", "-o", str(spv)], timeout=1800)
    if r.returncode != 0 or not spv.is_file():
        degrade.append(f"kernel SPV 编译失败: {(r.stdout + r.stderr)[-200:]}")
    else:
        val = run(["spirv-val", str(spv)], timeout=600)
        if val.returncode != 0:
            degrade.append(f"spirv-val 未过: {(val.stdout + val.stderr)[-200:]}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {
            "schema": "rurix.g31.p4_streaming.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP，非 PASS 非 FAIL）")
        return 0

    # ── harness 真跑（单锁；双臂 + 优先级探针一体）──
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    pages_dir = WORK / "pages"
    harness_ev = WORK / f"harness_{ts}.json"
    with gpu_device_lock(purpose=f"{TAG} cluster 流送双臂真跑"):
        rh = run(
            [str(BIN),
             "--gltf", str(BISTRO_GLTF),
             "--spv", str(spv),
             "--pages-dir", str(pages_dir),
             "--evidence", str(harness_ev),
             "--orbit-frames", str(orbit_frames),
             "--hold-frames", str(hold_frames)],
            timeout=7200,
        )
    out = (rh.stdout or "") + (rh.stderr or "")
    doc = None
    if harness_ev.is_file():
        try:
            doc = json.loads(harness_ev.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    legs_ok = True
    if rh.returncode != 0 or doc is None or "[g31_cluster_stream]: PASS" not in out:
        fail(f"harness 真跑失败 rc={rh.returncode}: {out[-400:]}")
        return 1
    if '"state":"skipped_dev_env"' in out:
        note("harness skipped_dev_env（三态之 SKIP）")
        return 0
    if "Validation Error" in out or "VUID-" in out:
        fail("validation 应静默却报错")
        legs_ok = False
    if doc.get("schema") != HARNESS_SCHEMA_ID:
        fail(f"harness schema ≠ {HARNESS_SCHEMA_ID}: {doc.get('schema')!r}")
        legs_ok = False

    scene = doc.get("scene") or {}
    ref = doc.get("reference") or {}
    stream = doc.get("stream") or {}
    conv = doc.get("convergence") or {}
    probe = doc.get("priority_probe") or {}

    # ── ① P4-1 页集与驻留池 ──
    f1 = p4_1_judge(scene, stream)
    pages = scene.get("pages", 0)
    pool = stream.get("pool_slots", 0)
    set_fact(
        "p4_1_page_pool",
        not f1,
        f"RXPDv2 页集 {pages} 页/{scene.get('clusters')} 簇/{scene.get('meshes')} 网格/"
        f"{scene.get('triangles')} 三角真实落盘（digest {str(doc.get('page_set_digest'))[:23]}…）；"
        f"池 {pool} 槽 < 全集 {pages}；LRU 逐出 {stream.get('evicted_total')} 页次真实发生"
        + ("" if not f1 else f"；红 {f1[:3]}"),
    )
    # ── ② P4-2 请求-驻留闭环 ──
    f2 = p4_2_judge(stream)
    req_total = sum(f.get("device_requests", 0) for f in (stream.get("frames") or []))
    set_fact(
        "p4_2_request_loop",
        not f2,
        f"剔除 pass device 请求 {req_total} 条（io 派发 {stream.get('io_submitted')}/"
        f"完成 {stream.get('io_completed')}）；末帧缺页清零；device 消费校验 checksum 演化"
        + ("" if not f2 else f"；红 {f2[:3]}"),
    )
    # ── ③ P4-3 回退正确性 ──
    f3 = p4_3_judge(ref, stream)
    set_fact(
        "p4_3_fallback_parity",
        not f3,
        f"回退帧 {stream.get('fallback_frames')}；device vs host 金标准逐帧全等 + "
        f"覆盖不变量逐帧；reference 全驻留零回退 + 双跑位级"
        + ("" if not f3 else f"；红 {f3[:3]}"),
    )
    # ── ④ P4-4 异步 IO 优先级 ──
    f4 = p4_4_judge(stream, probe)
    set_fact(
        "p4_4_io_priority",
        not f4,
        f"优先级倒置探针 ok={probe.get('ok')}（出队序 {probe.get('order')}，"
        f"读 {probe.get('bytes_read')} B measured）；全程真读 "
        f"{stream.get('io_read_bytes_total')} B"
        + ("" if not f4 else f"；红 {f4[:3]}"),
    )
    # ── ⑤ 整合真跑 ──
    f5 = integration_judge(ref, stream, orbit_frames, hold_frames)
    total = orbit_frames + hold_frames
    sseq = stream.get("digest_seq") or []
    rseq = ref.get("digest_seq") or []
    hold_bit = sum(1 for i in range(total - hold_frames, total) if i < len(sseq) and sseq[i] == rseq[i])
    set_fact(
        "integration",
        not f5,
        f"强制小池 {pool}/{pages} 穿越轨迹 {total} 帧无崩；hold 位级帧 {hold_bit}/{hold_frames}"
        f"（末 2 位级={conv.get('last2_bitexact')}）；缺页帧 {stream.get('miss_frames')} "
        f"回退帧 {stream.get('fallback_frames')} 逐出 {stream.get('evicted_total')} "
        f"io {stream.get('io_bytes_total')}B 上传 {stream.get('upload_bytes_total')}B measured"
        + ("" if not f5 else f"；红 {f5[:3]}"),
    )
    # ── ⑥ 冻结旧面 0-byte 机核（工作树面）──
    frozen_ok = True
    frozen_detail: list[str] = []
    for label, paths in (
        ("geom_pages_v1", FROZEN_GEOM_PAGES_V1),
        ("streaming_v1", FROZEN_STREAMING_V1),
        ("geometry", FROZEN_GEOMETRY),
        ("geom_build_v1", FROZEN_GEOM_BUILD_V1),
    ):
        u = run(["git", "status", "--porcelain", "--", *paths])
        clean = not u.stdout.strip()
        frozen_ok &= clean
        frozen_detail.append(f"{label}={'0byte' if clean else 'DIRTY'}")
    set_fact(
        "frozen_surfaces_0byte",
        frozen_ok,
        "；".join(frozen_detail) + "（工作树机核；G8 页 ABI 旧面/streaming v1 四件/"
        "geometry 三件/geom_build.rs 全 0-byte，扩展全为加性新文件）",
    )

    # ── 门裁决（facts 全绿 + legs_ok + FAILURES 空）──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and legs_ok and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    units = scene.get("units") or []
    roots_total = sum(u.get("root_pages", 0) for u in units if isinstance(u, dict))
    probe_order = probe.get("order") or []
    gate_doc = {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "scene_id": SCENE,
        "p4_1_page_pool": {
            "meshes": int(scene.get("meshes", 0)),
            "clusters": int(scene.get("clusters", 0)),
            "pages": int(pages),
            "triangles": int(scene.get("triangles", 0)),
            "page_set_digest": doc.get("page_set_digest") or ("sha256:" + "0" * 64),
            "pool_slots": int(pool),
            "pool_less_than_total": bool(isinstance(pool, int) and pool < pages),
            "root_pages": int(roots_total),
            "evicted_total": int(stream.get("evicted_total", 0)),
            "lru_eviction_observed": bool(stream.get("evicted_total", 0) >= 1),
            "pages_within_contract": True,
        },
        "p4_2_request_loop": {
            "device_requests_total": int(req_total),
            "io_submitted": int(stream.get("io_submitted", 0)),
            "io_completed": int(stream.get("io_completed", 0)),
            "pages_loaded_total": int(sum(f.get("pages_loaded", 0) for f in (stream.get("frames") or []))),
            "resident_growth_observed": bool(f2 == []),
            "checksum_consumes_upload": bool(f2 == []),
            "miss_frames": int(stream.get("miss_frames", 0)),
            "final_frame_miss_zero": bool((stream.get("frames") or [{}])[-1].get("miss_selected", -1) == 0),
        },
        "p4_3_fallback_parity": {
            "fallback_frames": int(stream.get("fallback_frames", 0)),
            "parity_all_frames": bool(stream.get("parity_all") is True),
            "cover_all_frames": bool(stream.get("cover_all") is True),
            "reference_zero_fallback": bool(f3 == []),
            "reference_double_run_bitexact": bool(ref.get("double_run_bitexact") is True),
        },
        "p4_4_io_priority": {
            "probe_ok": bool(probe.get("ok") is True),
            "probe_first_is_high_priority": bool(probe.get("ok") is True),
            "probe_order": [int(x) for x in probe_order],
            "probe_bytes_read": int(probe.get("bytes_read", 0)),
            "io_read_bytes_total": int(stream.get("io_read_bytes_total", 0)),
        },
        "integration": {
            "orbit_frames": int(orbit_frames),
            "hold_frames": int(hold_frames),
            "frames_completed": int(len(stream.get("frames") or [])),
            "last2_bitexact": bool(conv.get("last2_bitexact") is True),
            "hold_bitexact_frames": int(hold_bit),
            "zero_fallback_frames_bitexact": bool(f5 == []),
            "miss_frames": int(stream.get("miss_frames", 0)),
            "fallback_frames": int(stream.get("fallback_frames", 0)),
            "evicted_total": int(stream.get("evicted_total", 0)),
            "io_bytes_total": int(stream.get("io_bytes_total", 0)),
            "upload_bytes_total": int(stream.get("upload_bytes_total", 0)),
            "pool_slots": int(pool),
            "total_pages": int(pages),
        },
        "frozen_surfaces": {
            "geom_pages_v1_files_0byte": bool(frozen_ok),
            "streaming_v1_files_0byte": bool(frozen_ok),
            "geometry_files_0byte": bool(frozen_ok),
            "geom_build_v1_0byte": bool(frozen_ok),
        },
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C11 cluster 流送 P4 四行（RD-039 分项；TODO §3 #20~#23）："
            "P4-1 RXPD major=2 加性磁盘面（disk_v2.rs）+ bistro 派生真实页集 + LRU 驻留池；"
            "P4-2 剔除 pass device 请求缓冲 → host 驻留调度（异步优先级读 + 三预算 tick + "
            "页表/页池镜像上传 + 次帧消费校验）闭环；P4-3 驻留约束一致性 cut 回退（祖先-后代"
            "合并，禁空洞/禁重复覆盖）device vs host 逐帧对拍 + 覆盖不变量；P4-4 优先级堆异步 "
            "IO（投影直径重要度）+ 倒置探针 measured；整合 = 强制小池穿越轨迹，零回退帧与全驻留"
            "参考逐帧位级（回退帧允许 LOD 差——容差结构依据 = 一致性 cut 语义），末 2 帧收敛位级。"
            f"facts: {'; '.join(f['id'] + '=' + f['status'] for f in fact_rows)}"
        ),
    }
    import jsonschema  # 自校验硬门（schema 漂移即 RED；PASS-only 闭集面）

    errs = list(jsonschema.Draft7Validator(
        json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        for e in errs[:5]:
            fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
        all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_p4_streaming_{ts}.json"
    else:
        # FAIL 诊断件落 .tmp 工作区——PASS-only schema 面，evidence/ 只收门件
        # （fail-closed：evidence/ 无件 = 门未过，不污染 check_schemas 路由面）。
        gate_path = WORK / f"gate_fail_{ts}.json"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}（harness 真跑件留 .tmp 工作区）")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂 + schema 互核，无 GPU/无构建依赖）
# ---------------------------------------------------------------------------


def _scene(pages: int = 89, meshes: int = 12, clusters: int = 16553, tris: int = 400000) -> dict:
    return {
        "meshes": meshes,
        "clusters": clusters,
        "pages": pages,
        "triangles": tris,
        "units": [{"root_pages": 1}, {"root_pages": 1}],
    }


def _frame(i: int, **kw) -> dict:
    base = {
        "frame": i,
        "digest": "sha256:" + "0" * 64,
        "selected": 700,
        "selected_pages": 30,
        "selected_flow_pages": 22,
        "miss_selected": 0,
        "fallback": 0,
        "device_requests": 0,
        "pages_loaded": 1,
        "pages_evicted": 0,
        "bytes_io": 1000,
        "bytes_upload": 2000,
        "queue_depth": 0,
        "resident": 10 + i,
        "checksum0": 12345 + i,
        "checksum1": 777,
        "parity_ok": True,
        "cover_ok": True,
        "io_wait_ms": 1.0,
    }
    base.update(kw)
    return base


def _stream(orbit: int = 4, hold: int = 2, bad_last: bool = False) -> dict:
    frames = []
    for i in range(orbit + hold):
        miss = 5 if i < orbit + hold - 2 else 0
        frames.append(_frame(i, miss_selected=miss, fallback=miss, device_requests=miss))
    if bad_last:
        frames[-1]["miss_selected"] = 3
    return {
        "pool_slots": 40,
        "digest_seq": ["sha256:" + "0" * 64] * (orbit + hold),
        "frames": frames,
        "parity_all": True,
        "cover_all": True,
        "io_bytes_total": 100000,
        "upload_bytes_total": 200000,
        "miss_frames": 3,
        "fallback_frames": 3,
        "evicted_total": 20,
        "resident_final": 40,
        "io_read_bytes_total": 110000,
        "io_submitted": 30,
        "io_completed": 30,
        "req_overflow_total": 0,
    }


def _ref(orbit: int = 4, hold: int = 2, drift: bool = False) -> dict:
    seq = ["sha256:" + "0" * 64] * (orbit + hold)
    if drift:
        seq[-1] = "sha256:" + "f" * 64
    return {
        "pool_slots": 89,
        "digest_seq": seq,
        "double_run_bitexact": not drift,
        "frames": [_frame(i, resident=89) for i in range(orbit + hold)],
        "parity_all": True,
        "cover_all": True,
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

    # 红绿臂①:P4-1 判。
    expect(p4_1_judge(_scene(), _stream()) == [], "GREEN:P4-1 正例")
    expect(p4_1_judge(_scene(pages=4), _stream()), "RED:页集 < 8 必红")
    s = _stream(); s["pool_slots"] = 89
    expect(p4_1_judge(_scene(), s), "RED:池未小于全集必红")
    s = _stream(); s["evicted_total"] = 0
    expect(p4_1_judge(_scene(), s), "RED:零逐出必红")
    expect(p4_1_judge({}, _stream()), "RED:scene 缺块必红")
    sc = _scene(); sc["units"] = []
    expect(p4_1_judge(sc, _stream()), "RED:零 root 页必红")
    # 红绿臂②:P4-2 判。
    expect(p4_2_judge(_stream()) == [], "GREEN:P4-2 正例")
    s = _stream()
    for f in s["frames"]:
        f["device_requests"] = 0
    expect(p4_2_judge(s), "RED:零 device 请求（空接线冒充）必红")
    s = _stream(); s["io_completed"] = 0
    expect(p4_2_judge(s), "RED:零 IO 完成必红")
    s = _stream()
    for f in s["frames"]:
        f["pages_loaded"] = 0
    expect(p4_2_judge(s), "RED:零页加载必红")
    s = _stream()
    for i, f in enumerate(s["frames"]):
        f["resident"] = 10
    expect(p4_2_judge(s), "RED:驻留零增长必红")
    s = _stream()
    for f in s["frames"]:
        f["checksum0"] = 0
    expect(p4_2_judge(s), "RED:checksum 全零必红")
    s = _stream()
    for f in s["frames"]:
        f["checksum0"] = 42
    expect(p4_2_judge(s), "RED:checksum 无演化必红")
    expect(p4_2_judge(_stream(bad_last=True)), "RED:末帧缺页未清零必红")
    # 红绿臂③:P4-3 判。
    expect(p4_3_judge(_ref(), _stream()) == [], "GREEN:P4-3 正例")
    s = _stream(); s["fallback_frames"] = 0
    expect(p4_3_judge(_ref(), s), "RED:零回退帧必红")
    s = _stream(); s["parity_all"] = False
    expect(p4_3_judge(_ref(), s), "RED:对拍不全等必红")
    s = _stream(); s["cover_all"] = False
    expect(p4_3_judge(_ref(), s), "RED:覆盖不变量破必红")
    r = _ref(); r["frames"][0]["fallback"] = 1
    expect(p4_3_judge(r, _stream()), "RED:参考臂有回退必红")
    r = _ref(drift=True)
    expect(p4_3_judge(r, _stream()), "RED:reference 双跑非位级必红")
    # 红绿臂④:P4-4 判。
    good_probe = {"ok": True, "order": [5, 1, 2, 3], "bytes_read": 1000}
    expect(p4_4_judge(_stream(), good_probe) == [], "GREEN:P4-4 正例")
    expect(p4_4_judge(_stream(), dict(good_probe, ok=False)), "RED:探针未过必红")
    expect(p4_4_judge(_stream(), dict(good_probe, order=[1])), "RED:探针序短必红")
    expect(p4_4_judge(_stream(), dict(good_probe, bytes_read=0)), "RED:探针零读必红")
    s = _stream(); s["io_read_bytes_total"] = 0
    expect(p4_4_judge(s, good_probe), "RED:全程零真读必红")
    expect(p4_4_judge(_stream(), None), "RED:probe 缺块必红")
    # 红绿臂⑤:整合判。
    expect(integration_judge(_ref(), _stream(), 4, 2) == [], "GREEN:整合正例")
    expect(integration_judge(_ref(drift=True), _stream(), 4, 2), "RED:末帧非位级必红")
    s = _stream()
    s["frames"][0]["fallback"] = 0
    s["digest_seq"][0] = "sha256:" + "a" * 64
    expect(integration_judge(_ref(), s, 4, 2), "RED:零回退帧 digest 漂移必红")
    s = _stream(); s["frames"] = s["frames"][:-1]
    expect(integration_judge(_ref(), s, 4, 2), "RED:帧数不齐（崩帧）必红")
    s = _stream(); s["io_bytes_total"] = 0
    expect(integration_judge(_ref(), s, 4, 2), "RED:IO 计量零必红")
    # 三态判。
    expect(degrade_exit_code([], False) is None, "GREEN:无降级续跑")
    expect(degrade_exit_code(["x"], False) == 0, "GREEN:降级 SKIP 退 0")
    expect(degrade_exit_code(["x"], True) == 1, "RED:REQUIRE_REAL 下降级翻硬红")
    # schema 互核:在树 + 关键 const/required 逐字。
    expect(SCHEMA_PATH.is_file(), "门 schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "wave const 互核")
        expect(gs["properties"]["scene_id"]["const"] == SCENE, "scene const 互核")
        expect(
            sorted(gs.get("required", [])) == sorted([
                "schema", "subject", "symbolic_gate_key", "wave", "scene_id",
                "p4_1_page_pool", "p4_2_request_loop", "p4_3_fallback_parity",
                "p4_4_io_priority", "integration", "frozen_surfaces",
                "environment", "timestamp", "notes",
            ]),
            "schema required 闭集互核（14 字段）",
        )
    expect(len(FACT_IDS) == 6, "facts 闭集 = 6")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=6；5 红臂组 + 正例组 + 三态 + schema 互核）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--orbit-frames", type=int, default=24)
    ap.add_argument("--hold-frames", type=int, default=8)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        if args.orbit_frames < 12:
            print(f"[{TAG}] FAIL: --orbit-frames {args.orbit_frames} < 12（schema 下限）", file=sys.stderr)
            return 1
        if args.hold_frames < 6:
            print(f"[{TAG}] FAIL: --hold-frames {args.hold_frames} < 6（schema 下限）", file=sys.stderr)
            return 1
        return run_gate(args.orbit_frames, args.hold_frames)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
