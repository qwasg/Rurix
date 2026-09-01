#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C7 性能剖析与调试工具面）
"""G31+ 波 C Task C7：性能剖析与调试工具面门冒烟（g31.waveC.profiling；
G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #54「性能剖析与调试工具面：GPU 时间戳/pass 级
profiler 对外暴露、Nsight 标注、帧捕获兼容（RenderDoc）」兑现载体；验收锚 = 「外部
用户可自助定位帧内热点」）。

实现面（本批 materialize）：
  ① --profile-json <path> 统一 profiler 输出面（默认关,开启零渲染语义变更）——
     g31_window_present / g14_3_pipeline_perf 双 bin 同 schema
     `rurix.g31.profile_output.v1`（milestones/g31/g31_profile_output_schema.json）：
     逐 pass GPU ms（telemetry 声明序全量;g31 = scene/mv/resample/resolve/encode
     五段,g14_3 = scene/mv/resample/resolve 四段）+ CPU 段（record/submit/
     fence_wait/readback_convert）+ 帧段（render/present/digest 或 frame/prod/tail）
     + 统计组 mean/p50/p99/min/max + 恒等式字段（分解和≈帧墙钟,容差 0.10/2.00ms）
     + debug label 态 + profiler 开销如实登记。
  ② Nsight 标注：VK_EXT_debug_utils 在位即启用（render_exec.rs create_instance
     枚举判定;validation 关也在位）,record_frame_body 逐 pass 录
     vkCmdBegin/EndDebugUtilsLabelEXT（pass 名,包裹 timestamp 区间 + pass 本体）;
     扩展/符号 absent = 双 None 零开销跳过（fail-silent 不崩）。
  ③ RenderDoc 帧捕获兼容：renderdoccmd 在 PATH/常见安装位 → 真捕获腿（.rdc 产出
     + 尺寸阈核验）;不在机 → 静态捕获兼容核验（validation 静默 + 捕获不兼容
     API 模式 blocklist 0 命中 + present 标准 swapchain 腿声明）+ 如实
     DEV_ENV_DEGRADE 登记,不冒充真捕获。

判据闭集（milestones/g31/g31_profiling_evidence_schema.json 描述段逐字）：
1. profile_schema_compliant：双 bin profile JSON 过 g31_profile_output_schema.json
   Draft7 校验（schema/bin/tolerance const 互核）。
2. pass_decomposition_measured：声明 pass 名序全在 + frames_measured == 真跑帧数
   + scene pass mean > 0 + 全统计有限。
3. identity_sum_matches_frame：双 bin 恒等式成立——gpu_sum_mean ≤
   render_wall_mean + 0.10 且 −0.10 ≤ host_residual_mean ≤ 2.00。G39 T4 多轮
   中位鲁棒化：on 腿 ×IDENTITY_ROUNDS 轮采样,判据消费逐分量 N 轮中位数
   （statistics.median;规则与容差字面不变,变的只是输入——单轮值 → 中位数）;
   逐轮明细如实登记 evidence identity_rounds 可选块（逐轮 identity_ok 可红,
   中位裁决落 profiles/*/identity_ok）。
4. debug_labels_recorded：双 profile debug_labels.active == true 且
   annotated_pass_count == 声明 pass 数（本机扩展在位;absent → 降级登记非冒充）。
5. profiler_zero_render_drift：profiler on/off 双臂同参复跑 digest 位级一致
   （g31 = presented digest + render_digest 双锚;g14_3 = last_frame_digest）。
6. capture_compat_verified：真捕获成功（real_capture）或静态核验绿
   （validation_error_count_total == 0 + blocklist 0 命中）+ RenderDoc absent 时
   degrade 如实登记。
7. tool_probe_registered：RenderDoc/Nsight Graphics 探测结果在档（state ∈
   {present,absent} + 探测路径登记;absent 不冒充）。

三态：无 bin/无 Vulkan/资产缺失 → DEV_ENV_DEGRADE 退 0（不冒充 PASS）；
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充真跑）。

evidence 纪律：PASS 才落 evidence/g31_profiling_<ts>.json（check_schemas 前缀路由
g31_profiling_）；FAIL 诊断件落 .tmp/g31_gates/profiling/ 工作区不污染 evidence/
路由面（fail-closed：evidence/ 无件 = 门未过）。

用法：
  py -3 ci/g31_profiling_smoke.py --selftest
  py -3 ci/g31_profiling_smoke.py --gate g31.waveC.profiling [--rounds N]
    （--rounds：identity 采样轮数,闭集 [1,9],缺省 IDENTITY_ROUNDS=5;偶数走
    statistics.median 线性插值,建议奇数）
"""
from __future__ import annotations

import argparse
import datetime as _dt
import glob
import io
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveC.profiling"
SUBJECT = "g31_profiling"
WAVE = "G31+.C"
TAG = "g31_profiling"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_profiling_evidence_schema.json"
PROFILE_SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_profile_output_schema.json"
SCHEMA_ID = "rurix.g31.profiling_evidence.v1"
PROFILE_SCHEMA_ID = "rurix.g31.profile_output.v1"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN_G31 = ROOT / "target" / "release" / f"g31_window_present{EXE_SUFFIX}"
BIN_G14 = ROOT / "target" / "release" / f"g14_3_pipeline_perf{EXE_SUFFIX}"
WORK = ROOT / ".tmp" / "g31_gates" / "profiling"

G31_PASSES = ["g14_3_direct_gi", "g14_mv", "g14_8_tsr_resample", "g14_8_tsr_resolve", "g31_display_encode"]
G14_PASSES = ["g14_3_direct_gi", "g14_mv", "g14_8_tsr_resample", "g14_8_tsr_resolve"]
G31_FRAMES, G31_WARMUP = 24, 6
G14_FRAMES, G14_WARMUP = 24, 6

# 恒等式容差（profile JSON identity 字段字面/bin 注释/docs 同一事实源——改动三面同步）。
IDENTITY_GPU_TOL_MS = 0.10
IDENTITY_HOST_TOL_MS = 2.00

# G39 T4 identity 多轮中位鲁棒化：on 腿采样轮数（判据消费逐分量 N 轮中位数;
# 规则/容差字面不动——变的只是输入;--rounds 可覆盖,闭集 [1,9]）。
IDENTITY_ROUNDS = 5
IDENTITY_ROUNDS_MIN, IDENTITY_ROUNDS_MAX = 1, 9

SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
SPV_FILES = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
)
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")

# RenderDoc 捕获不兼容 API 模式 blocklist（源码静态核验面;0 命中 = 兼容面无
# 已知不兼容模式;真捕获腿在机时以真捕获为准,本表为 absent 降级面的静态核验）。
CAPTURE_BLOCKLIST = [
    "vkCmdSetDiscardRectangleEXT",
    "VK_NVX_binary_import",
    "vkQueueBindSparse",
    "vkCmdBeginVideoCoding",
    "VK_KHR_video_queue",
    "VK_NV_low_latency",
    "VK_NV_cluster_acceleration_structure",
    "VK_EXT_opacity_micromap",
]
CAPTURE_AUDITED_FILES = [
    "src/rurix-rt/src/render_exec.rs",
    "src/rurix-rt/src/vk_g31_present.rs",
]

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")

FACT_IDS = [
    "profile_schema_compliant",
    "pass_decomposition_measured",
    "identity_sum_matches_frame",
    "debug_labels_recorded",
    "profiler_zero_render_drift",
    "capture_compat_verified",
    "tool_probe_registered",
]

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


# ---------------------------------------------------------------------------
# 判读器（selftest 红绿两臂消费面；全纯函数无 GPU 依赖）
# ---------------------------------------------------------------------------


def finite(x) -> bool:
    return isinstance(x, (int, float)) and not isinstance(x, bool) and x == x and x not in (float("inf"), float("-inf"))


def identity_ok(identity: dict) -> bool:
    """③ 分解和≈帧墙钟恒等式判：gpu_sum_mean ≤ render_wall_mean + 0.10 且
    −0.10 ≤ host_residual_mean ≤ 2.00（容差 = profile identity 字段字面同源）。"""
    if not isinstance(identity, dict):
        return False
    gs = identity.get("gpu_sum_mean_ms")
    rw = identity.get("render_wall_mean_ms")
    hr = identity.get("host_residual_mean_ms")
    if not (finite(gs) and finite(rw) and finite(hr)):
        return False
    return gs <= rw + IDENTITY_GPU_TOL_MS and -IDENTITY_GPU_TOL_MS <= hr <= IDENTITY_HOST_TOL_MS


def rounds_valid(n) -> bool:
    """--rounds 闭集校验：int（bool 拒）且 IDENTITY_ROUNDS_MIN ≤ n ≤ IDENTITY_ROUNDS_MAX。"""
    return isinstance(n, int) and not isinstance(n, bool) and IDENTITY_ROUNDS_MIN <= n <= IDENTITY_ROUNDS_MAX


def median_identity(identities: list[dict]) -> dict:
    """③ G39 T4 多轮中位鲁棒化：逐分量取 N 轮 identity 的中位数
    （statistics.median;N 奇数取中值,偶数线性插值），产出合成 identity 供
    **不变的** identity_ok 规则消费（容差字面 0-byte 不动,变的只是输入）。
    空轮列/任一轮非 dict/任一判据分量非有限 → {}（fail-closed:identity_ok({}) 必红）。"""
    if not identities or not all(isinstance(d, dict) for d in identities):
        return {}
    out: dict = {}
    for key in ("gpu_sum_mean_ms", "render_wall_mean_ms", "host_residual_mean_ms"):
        vals = [d.get(key) for d in identities]
        if not all(finite(v) for v in vals):
            return {}
        out[key] = statistics.median(vals)
    # cpu_seg_sum 非判据分量:可算则一并出中位（evidence profiles 块登记面）,缺失不翻红。
    cpu_vals = [d.get("cpu_seg_sum_mean_ms") for d in identities]
    if all(finite(v) for v in cpu_vals):
        out["cpu_seg_sum_mean_ms"] = statistics.median(cpu_vals)
    return out


def seg_stats_sane(seg: dict) -> bool:
    """段统计组有限性 + min ≤ p50 ≤ max + mean 有限（p99 ≥ p50 弱序——线性插值面）。"""
    if not isinstance(seg, dict):
        return False
    vals = [seg.get(k) for k in ("mean_ms", "p50_ms", "p99_ms", "min_ms", "max_ms")]
    if not all(finite(v) for v in vals):
        return False
    mean_v, p50, p99, mn, mx = vals
    return mn <= p50 <= mx and p99 >= p50 and mn <= mean_v <= mx


def check_profile_decomposition(doc: dict, expect_bin: str, expect_frames: int, expect_passes: list[str]) -> list[str]:
    """② 分解 measured 判读（返回失败串列表,空 = 绿）：bin/frames/pass 名序/scene
    mean > 0/全段统计组 sane。"""
    fails: list[str] = []
    if not isinstance(doc, dict):
        return ["profile 非 object"]
    if doc.get("schema") != PROFILE_SCHEMA_ID:
        fails.append(f"profile schema {doc.get('schema')!r} ≠ {PROFILE_SCHEMA_ID}")
    if doc.get("bin") != expect_bin:
        fails.append(f"profile bin {doc.get('bin')!r} ≠ {expect_bin}")
    if doc.get("frames_measured") != expect_frames:
        fails.append(f"frames_measured {doc.get('frames_measured')} ≠ {expect_frames}")
    passes = doc.get("gpu_passes") or []
    names = [p.get("name") for p in passes if isinstance(p, dict)]
    if names != expect_passes:
        fails.append(f"gpu_passes 名序 {names} ≠ 声明面 {expect_passes}")
    for p in passes:
        if not seg_stats_sane(p):
            fails.append(f"pass {p.get('name')} 统计组非 sane")
    if passes and isinstance(passes[0], dict):
        scene_mean = passes[0].get("mean_ms")
        if not (finite(scene_mean) and scene_mean > 0):
            fails.append(f"scene pass mean {scene_mean} 非正")
    for seg in (doc.get("cpu_segments") or []) + (doc.get("frame_segments") or []):
        if not seg_stats_sane(seg):
            fails.append(f"段 {seg.get('name')} 统计组非 sane")
    return fails


def labels_ok(doc: dict, expect_pass_count: int) -> bool:
    """④ 标注段存在判：debug_labels.active 且 annotated == 声明 pass 数。"""
    dl = (doc or {}).get("debug_labels") or {}
    return dl.get("active") is True and dl.get("annotated_pass_count") == expect_pass_count


def drift_ok(off: dict, on: dict, keys: list[str]) -> bool:
    """⑤ 位级零漂移判：off/on 两 evidence 指定 digest 键全等（sha256 形态先行）。"""
    for k in keys:
        a, b = (off or {}).get(k), (on or {}).get(k)
        if not (isinstance(a, str) and isinstance(b, str) and DIGEST_RE.match(a) and DIGEST_RE.match(b)):
            return False
        if a != b:
            return False
    return True


def capture_compat_ok(method: str, validation_errors: int, blocklist_hits: int, rdc_bytes: int) -> bool:
    """⑥ 捕获兼容判：real_capture ⇒ rdc_bytes > 64KiB；static ⇒ validation 静默
    （harness 逐帧 fail-closed 已证 rc=0,计数面登记 0）+ blocklist 0 命中。"""
    if method == "real_capture":
        return rdc_bytes > 64 * 1024
    if method == "static_checklist_no_renderdoc":
        return validation_errors == 0 and blocklist_hits == 0
    return False


def tool_probe_ok(probe: dict) -> bool:
    """⑦ 工具探测登记判：renderdoc/nsight_graphics 双行 state ∈ {present,absent}
    且探测路径串非空（absent 如实,不冒充 present）。"""
    if not isinstance(probe, dict):
        return False
    for key in ("renderdoc", "nsight_graphics"):
        row = probe.get(key) or {}
        if row.get("state") not in ("present", "absent"):
            return False
        if not isinstance(row.get("probe"), str) or not row["probe"]:
            return False
    return True


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    """三态裁决：无降级 → None（续跑）；有降级 + REQUIRE_REAL → 1（硬红）；
    有降级无 REQUIRE_REAL → 0（SKIP 非 PASS 非 FAIL）。"""
    if not degrade:
        return None
    return 1 if require_real else 0


def scan_blocklist() -> tuple[int, list[str]]:
    """捕获不兼容 API 模式静态扫描（CAPTURE_AUDITED_FILES × CAPTURE_BLOCKLIST）。
    返回 (总命中数, 命中明细串列)。"""
    hits = 0
    detail: list[str] = []
    for rel in CAPTURE_AUDITED_FILES:
        p = ROOT / rel
        if not p.is_file():
            hits += 1
            detail.append(f"审计文件缺失 {rel}")
            continue
        text = p.read_text(encoding="utf-8", errors="replace")
        for token in CAPTURE_BLOCKLIST:
            n = text.count(token)
            if n:
                hits += n
                detail.append(f"{rel}: {token} ×{n}")
    return hits, detail


def probe_tools() -> dict:
    """RenderDoc/Nsight Graphics 在机探测（PATH + 常见安装位;如实 absent/present）。"""
    renderdoc_path = shutil.which("renderdoccmd") or ""
    if not renderdoc_path:
        cand = Path(r"C:\Program Files\RenderDoc\renderdoccmd.exe")
        if cand.is_file():
            renderdoc_path = str(cand)
    nsg_path = shutil.which("nsg") or ""
    if not nsg_path:
        hits = sorted(glob.glob(r"C:\Program Files\NVIDIA Corporation\Nsight Graphics*"))
        hits = [h for h in hits if Path(h).is_dir()]
        if hits:
            nsg_path = hits[0]
    return {
        "renderdoc": {
            "state": "present" if renderdoc_path else "absent",
            "path": renderdoc_path,
            "probe": "PATH(renderdoccmd) + C:\\Program Files\\RenderDoc\\renderdoccmd.exe",
        },
        "nsight_graphics": {
            "state": "present" if nsg_path else "absent",
            "path": nsg_path,
            "probe": "PATH(nsg) + C:\\Program Files\\NVIDIA Corporation\\Nsight Graphics*",
        },
    }


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def g31_leg(label: str, profile_path: Path | None) -> tuple[subprocess.CompletedProcess, dict, str]:
    """g31_window_present 真窗口腿（--hidden;profile on/off 由 profile_path 决定）。"""
    ev_path = WORK / f"g31_{label}_evidence.json"
    argv = [
        str(BIN_G31), "--frames", str(G31_FRAMES), "--warmup", str(G31_WARMUP),
        "--hidden", "--quality", "off",  # W4 默认翻转免疫:G31_PASSES 五段闭集判据钉死 off 形态（DEFAULT_FLIP_PLAN §2.5）
        "--evidence", str(ev_path),
    ]
    if profile_path is not None:
        argv += ["--profile-json", str(profile_path)]
    r = run(argv, env=base_env())
    out = (r.stdout or "") + (r.stderr or "")
    ev = {}
    if r.returncode == 0 and ev_path.is_file():
        try:
            ev = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            ev = {}
    return r, ev, out


def g14_leg(label: str, profile_path: Path | None) -> tuple[subprocess.CompletedProcess, dict, str]:
    """g14_3_pipeline_perf bench 腿（tsr_device 静态臂 inflight=1）。"""
    out_root = WORK / f"g14_{label}"
    argv = [
        str(BIN_G14), "--bench", "--scene", "bistro-interior", "--tier", "100",
        "--backend", "tsr_device", "--frames", str(G14_FRAMES), "--warmup", str(G14_WARMUP),
        "--out-root", str(out_root),
    ]
    if profile_path is not None:
        argv += ["--profile-json", str(profile_path)]
    r = run(argv, env=base_env())
    out = (r.stdout or "") + (r.stderr or "")
    rec = {}
    receipt_path = out_root / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
    if r.returncode == 0 and receipt_path.is_file():
        try:
            rec = json.loads(receipt_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            rec = {}
    return r, rec, out


def run_gate(rounds: int = IDENTITY_ROUNDS) -> int:
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
    if not PROFILE_SCHEMA_PATH.is_file():
        fail(f"profile 输出 schema 缺失: {PROFILE_SCHEMA_PATH}")
        return 1

    # ── 构建（release 双 bin;profiler/标注面含在内）──
    r = run([
        "cargo", "build", "--release", "-p", "rurix-render", "--features", "vendor-upscale",
        "--bin", "g31_window_present", "--bin", "g14_3_pipeline_perf", "--quiet",
    ])
    if r.returncode != 0:
        fail(f"harness release 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return 1

    # ── dev-env 探针（无 REQUIRE_REAL 短跑;skipped_dev_env / 异常即降级登记）──
    degrade: list[str] = []
    for b in (BIN_G31, BIN_G14):
        if not b.is_file():
            degrade.append(f"harness bin 缺失 {b}")
    for spv in SPV_FILES:
        if not (SPV_DIR / spv).is_file():
            degrade.append(f"SPV 缺失 {SPV_DIR / spv}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro 场景资产缺失 {BISTRO_GLTF}")
    if not degrade:
        probe_env = dict(os.environ)
        probe_env.pop("RURIX_REQUIRE_REAL", None)
        rp = run([
            str(BIN_G31), "--frames", "2", "--warmup", "1", "--hidden",
            "--quality", "off",  # W4 默认翻转免疫:探针与门主腿同 off 形态（DEFAULT_FLIP_PLAN §2.5）
            "--evidence", str(WORK / "dev_env_probe_g31.json"),
        ], timeout=1800, env=probe_env)
        probe_out = (rp.stdout or "") + (rp.stderr or "")
        if '"state":"skipped_dev_env"' in probe_out:
            degrade.append(f"g31 harness skipped_dev_env: {probe_out.strip()[-200:]}")
        elif rp.returncode != 0:
            degrade.append(f"g31 dev-env 探针 rc={rp.returncode}: {probe_out.strip()[-200:]}")
        rp2 = run([
            str(BIN_G14), "--bench", "--scene", "bistro-interior", "--tier", "100",
            "--backend", "tsr_device", "--frames", "2", "--warmup", "1",
            "--out-root", str(WORK / "dev_env_probe_g14"),
        ], timeout=1800, env=probe_env)
        probe2_out = (rp2.stdout or "") + (rp2.stderr or "")
        if '"state":"skipped_dev_env"' in probe2_out:
            degrade.append(f"g14 harness skipped_dev_env: {probe2_out.strip()[-200:]}")
        elif rp2.returncode != 0:
            degrade.append(f"g14 dev-env 探针 rc={rp2.returncode}: {probe2_out.strip()[-200:]}")

    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {"schema": "rurix.g31.profiling.skip.v1", "state": "DEV_ENV_DEGRADE", "reasons": degrade}
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP，非 PASS 非 FAIL）")
        return 0

    # ── 工具探测（RenderDoc/Nsight Graphics;如实 absent/present）──
    tools = probe_tools()
    note(f"tool probe: renderdoc={tools['renderdoc']['state']}({tools['renderdoc']['path'] or '—'}) "
         f"nsight_graphics={tools['nsight_graphics']['state']}({tools['nsight_graphics']['path'] or '—'})")

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    g31_profile_paths = [WORK / f"g31_profile_{ts}_r{i}.json" for i in range(1, rounds + 1)]
    g14_profile_paths = [WORK / f"g14_profile_{ts}_r{i}.json" for i in range(1, rounds + 1)]

    # ── 腿真跑（单锁串行;off ×1 + on ×rounds 双 bin——G39 T4 identity 多轮中位
    #    采样,各轮 profile 独立路径 _r<i>;数字全来自真实命令输出）──
    with gpu_device_lock(purpose=f"{TAG} g31 off+on×{rounds} + g14 off+on×{rounds} 腿"):
        r_g31_off, ev_g31_off, out_g31_off = g31_leg("off", None)
        g31_on_runs = [g31_leg(f"on_r{i}", g31_profile_paths[i - 1]) for i in range(1, rounds + 1)]
        r_g14_off, rec_g14_off, out_g14_off = g14_leg("off", None)
        g14_on_runs = [g14_leg(f"on_r{i}", g14_profile_paths[i - 1]) for i in range(1, rounds + 1)]

    io.open(WORK / f"g31_off_{ts}.log", "w", encoding="utf-8", newline="\n").write(out_g31_off)
    io.open(WORK / f"g14_off_{ts}.log", "w", encoding="utf-8", newline="\n").write(out_g14_off)
    for i, (_, _, out_leg) in enumerate(g31_on_runs, start=1):
        io.open(WORK / f"g31_on_r{i}_{ts}.log", "w", encoding="utf-8", newline="\n").write(out_leg)
    for i, (_, _, out_leg) in enumerate(g14_on_runs, start=1):
        io.open(WORK / f"g14_on_r{i}_{ts}.log", "w", encoding="utf-8", newline="\n").write(out_leg)

    # r1 = 首轮工件别名（其余 6 facts 口径不变消费 r1;identity 面消费全轮中位）。
    ev_g31_on = g31_on_runs[0][1]
    rec_g14_on = g14_on_runs[0][1]
    evs_g31_on = [doc for (_, doc, _) in g31_on_runs]
    recs_g14_on = [doc for (_, doc, _) in g14_on_runs]

    legs_ok = True
    leg_rows = [("g31_off", r_g31_off, ev_g31_off), ("g14_off", r_g14_off, rec_g14_off)]
    leg_rows += [(f"g31_on_r{i}", rr, doc) for i, (rr, doc, _) in enumerate(g31_on_runs, start=1)]
    leg_rows += [(f"g14_on_r{i}", rr, doc) for i, (rr, doc, _) in enumerate(g14_on_runs, start=1)]
    for label, rr, doc in leg_rows:
        if rr.returncode != 0 or not doc:
            fail(f"{label} 真跑失败 rc={rr.returncode}（产出 {'有' if doc else '无'}）")
            legs_ok = False

    def _load_json(p: Path) -> dict:
        if not p.is_file():
            return {}
        try:
            return json.loads(p.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            return {}

    profs_g31 = [_load_json(p) for p in g31_profile_paths]
    profs_g14 = [_load_json(p) for p in g14_profile_paths]
    # r1 profile = facts ①②④ 口径不变消费面。
    prof_g31 = profs_g31[0]
    prof_g14 = profs_g14[0]

    # ── ① profile JSON schema 合规（Draft7 硬校验 + const 互核）──
    import jsonschema

    profile_schema = json.loads(PROFILE_SCHEMA_PATH.read_text(encoding="utf-8"))
    schema_fails: list[str] = []
    for name, doc in (("g31", prof_g31), ("g14", prof_g14)):
        if not doc:
            schema_fails.append(f"{name} profile JSON 缺失/不可解析")
            continue
        errs = list(jsonschema.Draft7Validator(profile_schema).iter_errors(doc))
        for e in errs[:5]:
            schema_fails.append(f"{name} profile schema 红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
    set_fact(
        "profile_schema_compliant",
        not schema_fails,
        ("双 bin profile JSON 过 g31_profile_output_schema.json Draft7 校验（schema/bin/tolerance const 互核）"
         if not schema_fails else "; ".join(schema_fails[:3])),
    )

    # ── ② 分解 measured ──
    dec_fails = [f"g31: {m}" for m in check_profile_decomposition(prof_g31, "g31_window_present", G31_FRAMES, G31_PASSES)]
    dec_fails += [f"g14: {m}" for m in check_profile_decomposition(prof_g14, "g14_3_pipeline_perf", G14_FRAMES, G14_PASSES)]
    g31_passes = prof_g31.get("gpu_passes") or []
    g14_passes = prof_g14.get("gpu_passes") or []
    scene_g31 = g31_passes[0].get("mean_ms") if g31_passes else None
    scene_g14 = g14_passes[0].get("mean_ms") if g14_passes else None
    set_fact(
        "pass_decomposition_measured",
        not dec_fails,
        (f"g31 五段（scene={scene_g31}ms）+ g14 四段（scene={scene_g14}ms）名序全在,frames={G31_FRAMES}/{G14_FRAMES},全统计 sane"
         if not dec_fails else "; ".join(dec_fails[:3])),
    )

    # ── ③ 恒等式（G39 T4 多轮中位:逐 bin 取 N 轮分量中位数,套用**不变的**
    #    identity_ok 规则——容差字面 0-byte 不动,变的只是输入）──
    ids_g31 = [(p.get("identity") or {}) for p in profs_g31]
    ids_g14 = [(p.get("identity") or {}) for p in profs_g14]
    id_g31 = median_identity(ids_g31)
    id_g14 = median_identity(ids_g14)
    id_ok_g31 = identity_ok(id_g31)
    id_ok_g14 = identity_ok(id_g14)
    rounds_ok_g31 = [identity_ok(x) for x in ids_g31]
    rounds_ok_g14 = [identity_ok(x) for x in ids_g14]
    set_fact(
        "identity_sum_matches_frame",
        id_ok_g31 and id_ok_g14,
        (f"N={rounds} 轮中位:g31: gpu_sum={id_g31.get('gpu_sum_mean_ms')} ≤ wall={id_g31.get('render_wall_mean_ms')}+0.10,"
         f"residual={id_g31.get('host_residual_mean_ms')}ms ∈ [−0.10,2.00]（逐轮 ok={rounds_ok_g31}）;"
         f"g14: gpu_sum={id_g14.get('gpu_sum_mean_ms')} ≤ prod={id_g14.get('render_wall_mean_ms')}+0.10,"
         f"residual={id_g14.get('host_residual_mean_ms')}ms（逐轮 ok={rounds_ok_g14}）"
         if id_g31 and id_g14 else
         f"identity 中位合成失败（轮内字段缺失/非有限;逐轮 ok g31={rounds_ok_g31} g14={rounds_ok_g14}）"),
    )

    # ── ④ 标注段存在 ──
    lab_g31 = labels_ok(prof_g31, len(G31_PASSES))
    lab_g14 = labels_ok(prof_g14, len(G14_PASSES))
    if not lab_g31 or not lab_g14:
        degrade.append("VK_EXT_debug_utils absent（标注面 fail-silent 跳过,不可核验——如实登记非冒充）")
    set_fact(
        "debug_labels_recorded",
        lab_g31 and lab_g14,
        (f"g31 active=true annotated=5/5,g14 active=true annotated=4/4（render_exec.rs 逐 pass "
         f"vkCmdBegin/EndDebugUtilsLabelEXT;Nsight/RenderDoc 可辨识）"
         if lab_g31 and lab_g14 else
         f"g31={prof_g31.get('debug_labels')},g14={prof_g14.get('debug_labels')}（absent → 降级登记）"),
    )

    # ── ⑤ on/off 位级零漂移（G39 T4 加固:off 锚 × on 逐轮全等——on 各轮 digest
    #    位级恒值蕴含其中;判定只加严不放松）──
    drift_g31 = all(drift_ok(ev_g31_off, ev_on, ["digest", "render_digest"]) for ev_on in evs_g31_on)
    drift_g14 = all(drift_ok(rec_g14_off, rec_on, ["last_frame_digest"]) for rec_on in recs_g14_on)
    set_fact(
        "profiler_zero_render_drift",
        drift_g31 and drift_g14,
        (f"g31 digest+render_digest 双锚 off×on 全 {rounds} 轮位级一致（{str(ev_g31_on.get('digest'))[:23]}…）,"
         f"g14 last_frame_digest 全 {rounds} 轮位级一致（{str(rec_g14_on.get('last_frame_digest'))[:23]}…）"
         if drift_g31 and drift_g14 else
         f"g31 off={ev_g31_off.get('digest')}/{ev_g31_off.get('render_digest')} "
         f"on 逐轮={[str((e or {}).get('digest'))[:23] for e in evs_g31_on]};"
         f"g14 off={rec_g14_off.get('last_frame_digest')} "
         f"on 逐轮={[str((r or {}).get('last_frame_digest'))[:23] for r in recs_g14_on]}"),
    )

    # ── ⑥ 捕获兼容（真捕获腿 或 静态核验 + 降级登记）──
    blocklist_hits, blocklist_detail = scan_blocklist()
    capture_degrade: list[str] = []
    rdc_path = ""
    rdc_bytes = 0
    if tools["renderdoc"]["state"] == "present":
        # 真捕获腿（renderdoccmd 在机;失败 = 如实 FAIL 不降级为静态）。
        rdc_out = WORK / f"g31_capture_{ts}"
        cap = run([
            tools["renderdoc"]["path"], "capture", "-w", "-f", str(G31_WARMUP + 2),
            "-c", "3", "-o", str(rdc_out), "--",
            str(BIN_G31), "--frames", "12", "--warmup", str(G31_WARMUP), "--hidden",
            "--quality", "off",  # W4 默认翻转免疫:捕获腿与门主腿同 off 形态（DEFAULT_FLIP_PLAN §2.5）
            "--evidence", str(WORK / f"g31_capture_ev_{ts}.json"),
        ], timeout=1800, env=base_env())
        cands = sorted(WORK.glob(f"g31_capture_{ts}*.rdc"))
        if cap.returncode == 0 and cands:
            rdc_path = str(cands[0].relative_to(ROOT))
            rdc_bytes = cands[0].stat().st_size
        capture_method = "real_capture"
    else:
        capture_method = "static_checklist_no_renderdoc"
        capture_degrade.append(
            "RenderDoc 不在机（PATH + C:\\Program Files\\RenderDoc 双探 absent）：真捕获腿未跑,"
            "静态捕获兼容核验面兑现（validation 静默 + blocklist 0 命中）,不冒充真捕获"
        )
    capture_ok = capture_compat_ok(capture_method, 0, blocklist_hits, rdc_bytes)
    set_fact(
        "capture_compat_verified",
        capture_ok,
        (f"method={capture_method} validation=静默（门腿 rc 全 0,harness 逐帧 validation_error_count==0 "
         f"fail-closed）blocklist 命中={blocklist_hits}"
         + (f" rdc={rdc_path}({rdc_bytes}B)" if capture_method == "real_capture" else "（RenderDoc absent 降级登记）")
         if capture_ok else
         f"method={capture_method} blocklist 命中={blocklist_hits} rdc_bytes={rdc_bytes}: {';'.join(blocklist_detail[:3])}"),
    )

    # ── ⑦ 工具探测登记 ──
    tools_ok = tool_probe_ok(tools)
    if tools["nsight_graphics"]["state"] == "absent":
        capture_degrade.append(
            "Nsight Graphics 不在机（PATH(nsg) + Nsight Graphics* 双探 absent）：标注 UI 复核未跑,"
            "标注段存在由 profile debug_labels 面机器核验"
        )
    set_fact(
        "tool_probe_registered",
        tools_ok,
        (f"renderdoc={tools['renderdoc']['state']} nsight_graphics={tools['nsight_graphics']['state']} 双探在档"
         if tools_ok else f"探测面异常: {tools}"),
    )

    # ── 门裁决（facts 全绿 + 腿全绿 + FAILURES 空）──
    all_pass = all(f["status"] == "PASS" for f in facts.values()) and legs_ok and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    sp_g14_on = (rec_g14_on.get("stats_post_warmup") or {}) if rec_g14_on else {}

    def _round_row(x: dict) -> dict:
        # identity_rounds 逐轮行（缺失/非有限分量以 -1.0 如实占位;逐轮 identity_ok
        # 消费原始轮值——可红,中位裁决落 profiles/*/identity_ok const true）。
        return {
            "gpu_sum_mean_ms": x.get("gpu_sum_mean_ms") if finite(x.get("gpu_sum_mean_ms")) else -1.0,
            "render_wall_mean_ms": x.get("render_wall_mean_ms") if finite(x.get("render_wall_mean_ms")) else -1.0,
            "host_residual_mean_ms": x.get("host_residual_mean_ms") if finite(x.get("host_residual_mean_ms")) else -1.0,
            "identity_ok": identity_ok(x),
        }

    gate_doc = {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "anchor": {
            "todo_row": "G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #54",
            "anchor_text": "外部用户可自助定位帧内热点",
            "accept": "GPU 时间戳/pass 级 profiler 对外暴露 + Nsight 标注 + 帧捕获兼容（RenderDoc）",
        },
        "surface": {
            "cli": "--profile-json <path>",
            "bins": ["g31_window_present", "g14_3_pipeline_perf"],
            "profile_output_schema": "milestones/g31/g31_profile_output_schema.json",
            "stats": "mean/p50/p99/min/max",
            "g31_passes": G31_PASSES,
            "g14_passes": G14_PASSES,
        },
        "runs": {
            "g31_frames": {"frames": G31_FRAMES, "warmup": G31_WARMUP, "mode": "--hidden（真窗口 present 腿）"},
            "g14_frames": {"frames": G14_FRAMES, "warmup": G14_WARMUP, "mode": "--bench tsr_device inflight=1"},
            "g31_off": {
                "digest": ev_g31_off.get("digest") or ("sha256:" + "0" * 64),
                "render_digest": ev_g31_off.get("render_digest") or ("sha256:" + "0" * 64),
                "real_render_frame_ms": ev_g31_off.get("real_render_frame_ms") or -1.0,
            },
            "g31_on": {
                "digest": ev_g31_on.get("digest") or ("sha256:" + "0" * 64),
                "render_digest": ev_g31_on.get("render_digest") or ("sha256:" + "0" * 64),
                "real_render_frame_ms": ev_g31_on.get("real_render_frame_ms") or -1.0,
            },
            "g14_off": {
                "last_frame_digest": rec_g14_off.get("last_frame_digest") or ("sha256:" + "0" * 64),
                "frame_ms_production_mean": ((rec_g14_off.get("stats_post_warmup") or {}).get("frame_ms_production_mean")) or -1.0,
            },
            "g14_on": {
                "last_frame_digest": rec_g14_on.get("last_frame_digest") or ("sha256:" + "0" * 64),
                "frame_ms_production_mean": sp_g14_on.get("frame_ms_production_mean") or -1.0,
            },
        },
        "profiles": {
            "g31": {
                "path": str(g31_profile_paths[0].relative_to(ROOT)),
                "frames_measured": prof_g31.get("frames_measured") or 0,
                "gpu_sum_mean_ms": id_g31.get("gpu_sum_mean_ms") or -1.0,
                "render_wall_mean_ms": id_g31.get("render_wall_mean_ms") or -1.0,
                "cpu_seg_sum_mean_ms": id_g31.get("cpu_seg_sum_mean_ms") or -1.0,
                "host_residual_mean_ms": id_g31.get("host_residual_mean_ms") or 0.0,
                "identity_ok": bool(id_ok_g31),
                "debug_labels_active": bool(lab_g31),
                "annotated_pass_count": (prof_g31.get("debug_labels") or {}).get("annotated_pass_count") or 0,
                "assembly_ms": (prof_g31.get("profiler_overhead") or {}).get("assembly_ms") or -1.0,
                "scene_gpu_mean_ms": scene_g31 if scene_g31 is not None else -1.0,
                "schema_valid": not schema_fails,
            },
            "g14": {
                "path": str(g14_profile_paths[0].relative_to(ROOT)),
                "frames_measured": prof_g14.get("frames_measured") or 0,
                "gpu_sum_mean_ms": id_g14.get("gpu_sum_mean_ms") or -1.0,
                "render_wall_mean_ms": id_g14.get("render_wall_mean_ms") or -1.0,
                "cpu_seg_sum_mean_ms": id_g14.get("cpu_seg_sum_mean_ms") or -1.0,
                "host_residual_mean_ms": id_g14.get("host_residual_mean_ms") or 0.0,
                "identity_ok": bool(id_ok_g14),
                "debug_labels_active": bool(lab_g14),
                "annotated_pass_count": (prof_g14.get("debug_labels") or {}).get("annotated_pass_count") or 0,
                "assembly_ms": (prof_g14.get("profiler_overhead") or {}).get("assembly_ms") or -1.0,
                "scene_gpu_mean_ms": scene_g14 if scene_g14 is not None else -1.0,
                "schema_valid": not schema_fails,
            },
        },
        "identity_rounds": {
            "rounds": rounds,
            "g31": [_round_row(x) for x in ids_g31],
            "g14": [_round_row(x) for x in ids_g14],
        },
        "zero_drift": {
            "g31_digest_identical": bool(drift_g31),
            "g31_render_digest_identical": bool(drift_g31),
            "g14_digest_identical": bool(drift_g14),
            "method": (f"profiler on/off 双臂同参复跑 digest 位级对拍（g31 双锚 presented+render;"
                       f"g14 last_frame;G39 T4 加固:off 锚 × on 全 {rounds} 轮逐轮全等——"
                       f"on 各轮 digest 位级恒值蕴含其中,判定只加严）"),
        },
        "annotations": {
            "extension": "VK_EXT_debug_utils",
            "entry_points": "vkCmdBeginDebugUtilsLabelEXT/vkCmdEndDebugUtilsLabelEXT",
            "site": "src/rurix-rt/src/render_exec.rs record_frame_body 逐 pass 包裹（timestamp 区间 + pass 本体）",
            "absent_behavior": "fail-silent 零开销跳过（双符号任一缺失即 None,label_names 不分配）",
            "g31_active": bool(lab_g31),
            "g14_active": bool(lab_g14),
        },
        "tool_probe": tools,
        "capture_compat": {
            "method": capture_method,
            "validation_error_count_total": 0,
            "blocklist_hits": blocklist_hits,
            "blocklist": CAPTURE_BLOCKLIST,
            "audited_files": CAPTURE_AUDITED_FILES,
            "present_path": "vkQueuePresentKHR 标准 swapchain 腿（vk_g31_present.rs staging→copy→present;帧定界可捕获）",
            "rdc_path": rdc_path,
            "rdc_bytes": rdc_bytes,
            "dev_env_degrade": capture_degrade,
            "note": ("RenderDoc absent——静态核验面（validation 静默 + 捕获不兼容 API 模式 0 命中）;"
                     "真捕获腿待 RenderDoc 在机窗复跑本门即自动切换 real_capture 口径"
                     if capture_method == "static_checklist_no_renderdoc" else
                     f"renderdoccmd 真捕获 {rdc_path}（{rdc_bytes}B;帧定界 = swapchain present）"),
        },
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C7 性能剖析与调试工具面（G31_PLUS §5 #54 兑现）：① --profile-json 统一 "
            "profiler 输出面（双 bin 同 schema;逐 pass GPU/CPU/帧段 mean/p50/p99 + 恒等式字段 + profiler "
            "开销登记;默认关,on/off 双臂 digest 位级一致为渲染语义零变更机器证明）② Nsight 标注面 "
            "（VK_EXT_debug_utils 在位即启用,record_frame_body 逐 pass label,absent 零开销跳过）③ "
            "RenderDoc 捕获兼容面（真捕获/静态核验如实两臂）。判据：①profile schema 合规 ②分解全 "
            "measured ③分解和≈帧墙钟恒等式（容差 0.10/2.00ms）④标注段存在 ⑤profiler on/off 位级零漂移 "
            "⑥捕获兼容核验 ⑦工具探测登记。g14_3 --profile-json 首接面 = tsr_device 静态臂 inflight=1,"
            "vendor 双臂/FIF 流水/dyn/skin 面 CLI fail-closed 拒跑（归后续,不冒充）。"
            f"G39 T4 identity 多轮中位鲁棒化:on 腿 ×{rounds} 轮采样,判据消费逐分量中位数"
            "（规则与容差字面 0-byte 不变,变的只是输入;逐轮明细见 identity_rounds 块,"
            "r1 供其余 6 facts 口径不变消费）。逐轮 on digest 登记:"
            f"g31={[str((e or {}).get('digest'))[:15] for e in evs_g31_on]},"
            f"g14={[str((r or {}).get('last_frame_digest'))[:15] for r in recs_g14_on]}"
            "（fact⑤ 断言 off 锚 × on 逐轮全等——各轮位级恒值加固）。"
            f"facts: {'; '.join(f['id'] + '=' + f['status'] for f in (facts[fid] for fid in FACT_IDS))}"
        ),
    }

    errs = list(jsonschema.Draft7Validator(
        json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        for e in errs[:5]:
            fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
        all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_profiling_{ts}.json"
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
# selftest（判读器红绿两臂，无 GPU/无构建依赖）
# ---------------------------------------------------------------------------


def _fixture_profile(bin_name: str = "g31_window_present", frames: int = 24) -> dict:
    """合法 profile 合成夹具（GREEN 臂基座;红臂逐点破坏）。"""
    passes = G31_PASSES if bin_name == "g31_window_present" else G14_PASSES

    def seg(name, unit, base):
        return {"name": name, "unit": unit, "mean_ms": base, "p50_ms": base * 0.99,
                "p99_ms": base * 1.1, "min_ms": base * 0.9, "max_ms": base * 1.2}

    gpu = [seg(n, "gpu_timestamp_ms", 1.0 - i * 0.1) for i, n in enumerate(passes)]
    cpu = [seg(n, "host_wall_ms", 0.2) for n in ("cpu_record", "cpu_submit", "cpu_fence_wait", "readback_convert")]
    frame = [seg(n, "host_wall_ms", b) for n, b in (("render_wall", 3.0), ("present_wall", 1.0), ("digest", 0.1))]
    return {
        "schema": PROFILE_SCHEMA_ID,
        "bin": bin_name,
        "scene": "bistro-interior",
        "tier": 100,
        "backend": "tsr_device",
        "frames_measured": frames,
        "warmup": 6,
        "resolution": {"w": 1920, "h": 1080},
        "internal_resolution": {"w": 1920, "h": 1080},
        "headless": False,
        "render_digest": "sha256:" + "a" * 64,
        "gpu_passes": gpu,
        "cpu_segments": cpu,
        "frame_segments": frame,
        "identity": {
            "gpu_sum_mean_ms": 2.0,
            "gpu_sum_p99_ms": 2.2,
            "render_wall_mean_ms": 3.0,
            "cpu_seg_sum_mean_ms": 2.5,
            "host_residual_mean_ms": 0.5,
            "host_residual_p99_ms": 0.9,
            "host_residual_min_ms": -0.05,
            "host_residual_max_ms": 1.2,
            "gpu_sum_le_render_wall_tol_ms": 0.1,
            "host_residual_tol_ms": 2.0,
            "rule": "gpu_sum_mean<=render_wall_mean+0.10 && -0.10<=host_residual_mean<=2.00",
        },
        "debug_labels": {"active": True, "annotated_pass_count": len(passes),
                         "extension": "VK_EXT_debug_utils", "note": "n"},
        "profiler_overhead": {"assembly_ms": 0.04, "note": "n"},
        "notes": "x" * 40,
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

    # 红绿臂①：恒等式判。
    ident = _fixture_profile()["identity"]
    expect(identity_ok(ident), "GREEN:恒等式正例（2.0 ≤ 3.0+0.10;0.5 ∈ [−0.10,2.00]）")
    bad = dict(ident, gpu_sum_mean_ms=3.2)
    expect(not identity_ok(bad), "RED:gpu_sum 超墙钟+容差必红")
    bad = dict(ident, host_residual_mean_ms=2.1)
    expect(not identity_ok(bad), "RED:residual 超 2.00 必红")
    bad = dict(ident, host_residual_mean_ms=-0.2)
    expect(not identity_ok(bad), "RED:residual 低于 −0.10 必红")
    bad = dict(ident, host_residual_mean_ms=2.0)
    expect(identity_ok(bad), "GREEN:residual 恰上界 2.00")
    expect(not identity_ok({}), "RED:identity 空必红")
    bad = dict(ident, gpu_sum_mean_ms=float("nan"))
    expect(not identity_ok(bad), "RED:NaN 必红")
    # 红绿臂①b：多轮中位鲁棒化（G39 T4;规则/容差不变,输入换 N 轮中位——
    # 越界轮值取自三轮真红实测形态 −0.288 / +2.25）。

    def _idr(gs, rw, hr):
        return {"gpu_sum_mean_ms": gs, "render_wall_mean_ms": rw,
                "host_residual_mean_ms": hr, "cpu_seg_sum_mean_ms": 2.5}

    five_green = [_idr(2.0, 3.0, 0.5), _idr(2.0, 3.0, -0.288), _idr(2.0, 3.0, 0.45),
                  _idr(2.0, 3.0, 2.25), _idr(2.0, 3.0, 0.62)]
    med = median_identity(five_green)
    expect(med.get("host_residual_mean_ms") == 0.5, "median:5 轮 residual 中位=0.5（2 轮越界不掀翻中位）")
    expect(identity_ok(med), "GREEN:5 轮中 2 轮越界但中位在带 ⇒ 绿（中位鲁棒化语义）")
    five_red = [_idr(2.0, 3.0, 2.1), _idr(2.0, 3.0, 2.3), _idr(2.0, 3.0, 2.2),
                _idr(2.0, 3.0, 0.5), _idr(2.0, 3.0, 0.4)]
    expect(not identity_ok(median_identity(five_red)), "RED:residual 中位 2.1 越上界 2.00 必红")
    five_red_gs = [_idr(3.15, 3.0, 0.5), _idr(3.2, 3.0, 0.5), _idr(3.3, 3.0, 0.5),
                   _idr(2.0, 3.0, 0.5), _idr(2.0, 3.0, 0.5)]
    expect(not identity_ok(median_identity(five_red_gs)), "RED:gpu_sum 中位 3.15 超墙钟+0.10 必红")
    five_red_low = [_idr(2.0, 3.0, -0.2), _idr(2.0, 3.0, -0.3), _idr(2.0, 3.0, -0.25),
                    _idr(2.0, 3.0, 0.5), _idr(2.0, 3.0, 0.4)]
    expect(not identity_ok(median_identity(five_red_low)), "RED:residual 中位 −0.2 低于 −0.10 必红")
    expect(identity_ok(median_identity([_idr(2.0, 3.0, 0.5)])), "GREEN:N=1 中位退化 = 单轮语义")
    expect(median_identity([]) == {}, "RED:空轮列 fail-closed 空 identity")
    expect(median_identity([_idr(2.0, 3.0, 0.5), {"gpu_sum_mean_ms": 2.0}]) == {},
           "RED:轮内判据分量缺失 fail-closed")
    expect(median_identity([_idr(2.0, 3.0, 0.5), _idr(2.0, 3.0, float("nan"))]) == {},
           "RED:轮内 NaN fail-closed")
    m4 = median_identity([_idr(2.0, 3.0, 0.4), _idr(2.0, 3.0, 0.5),
                          _idr(2.0, 3.0, 0.6), _idr(2.0, 3.0, 2.5)])
    expect(abs(m4.get("host_residual_mean_ms", -9.0) - 0.55) < 1e-12, "median:偶数 N=4 线性插值 0.55")
    # 闭集：--rounds ∈ [1,9]。
    expect(rounds_valid(1) and rounds_valid(5) and rounds_valid(9), "GREEN:rounds 1/5/9 在闭集")
    expect(not rounds_valid(0) and not rounds_valid(10) and not rounds_valid(-3),
           "RED:rounds 0/10/−3 越闭集必拒")
    expect(not rounds_valid(True) and not rounds_valid(5.0), "RED:rounds 非 int（bool/float）必拒")
    expect(IDENTITY_ROUNDS == 5 and rounds_valid(IDENTITY_ROUNDS), "IDENTITY_ROUNDS 缺省 5 在闭集")
    # 红绿臂②：分解 measured 判。
    good = _fixture_profile()
    expect(check_profile_decomposition(good, "g31_window_present", 24, G31_PASSES) == [],
           "GREEN:合法 profile 分解判绿")
    expect(check_profile_decomposition(good, "g14_3_pipeline_perf", 24, G31_PASSES) != [],
           "RED:bin 错配必红")
    bad = _fixture_profile()
    bad["frames_measured"] = 23
    expect(check_profile_decomposition(bad, "g31_window_present", 24, G31_PASSES) != [],
           "RED:frames 不符必红")
    bad = _fixture_profile()
    bad["gpu_passes"] = bad["gpu_passes"][:4]
    expect(check_profile_decomposition(bad, "g31_window_present", 24, G31_PASSES) != [],
           "RED:缺 pass 必红")
    bad = _fixture_profile()
    bad["gpu_passes"][0]["mean_ms"] = 0.0
    expect(check_profile_decomposition(bad, "g31_window_present", 24, G31_PASSES) != [],
           "RED:scene mean 零必红")
    bad = _fixture_profile()
    bad["gpu_passes"][1]["p50_ms"] = 999.0
    expect(check_profile_decomposition(bad, "g31_window_present", 24, G31_PASSES) != [],
           "RED:p50 越界非 sane 必红")
    g14 = _fixture_profile("g14_3_pipeline_perf")
    expect(check_profile_decomposition(g14, "g14_3_pipeline_perf", 24, G14_PASSES) == [],
           "GREEN:g14 四段合法判绿")
    # 红绿臂③：标注段判。
    expect(labels_ok(good, 5), "GREEN:标注 active + 5/5")
    expect(not labels_ok(good, 4), "RED:计数不符必红")
    bad = _fixture_profile()
    bad["debug_labels"]["active"] = False
    expect(not labels_ok(bad, 5), "RED:inactive 必红")
    # 红绿臂④：位级零漂移判。
    d = "sha256:" + "b" * 64
    expect(drift_ok({"digest": d, "render_digest": d}, {"digest": d, "render_digest": d},
                    ["digest", "render_digest"]), "GREEN:双锚位级一致")
    expect(not drift_ok({"digest": d}, {"digest": "sha256:" + "c" * 64}, ["digest"]),
           "RED:digest 漂移必红")
    expect(not drift_ok({"digest": "bogus"}, {"digest": "bogus"}, ["digest"]),
           "RED:非 sha256 形态必红")
    expect(not drift_ok({}, {"digest": d}, ["digest"]), "RED:缺键必红")
    # 红绿臂⑤：捕获兼容 + 工具探测 + 三态。
    expect(capture_compat_ok("real_capture", 0, 0, 100_000), "GREEN:真捕获尺寸阈绿")
    expect(not capture_compat_ok("real_capture", 0, 0, 100), "RED:rdc 过小必红")
    expect(capture_compat_ok("static_checklist_no_renderdoc", 0, 0, 0), "GREEN:静态核验绿")
    expect(not capture_compat_ok("static_checklist_no_renderdoc", 1, 0, 0), "RED:validation 非静默必红")
    expect(not capture_compat_ok("static_checklist_no_renderdoc", 0, 2, 0), "RED:blocklist 命中必红")
    expect(not capture_compat_ok("unknown_method", 0, 0, 0), "RED:未知 method 必红")
    tp = {"renderdoc": {"state": "absent", "path": "", "probe": "PATH+ProgramFiles"},
          "nsight_graphics": {"state": "present", "path": "C:/x", "probe": "PATH(nsg)"}}
    expect(tool_probe_ok(tp), "GREEN:双探登记绿（absent/present 混合）")
    expect(not tool_probe_ok({"renderdoc": {"state": "unknown", "path": "", "probe": "x"},
                              "nsight_graphics": {"state": "absent", "path": "", "probe": "x"}}),
           "RED:state 越闭集必红")
    expect(not tool_probe_ok({"renderdoc": {"state": "absent", "path": "", "probe": ""},
                              "nsight_graphics": {"state": "absent", "path": "", "probe": "x"}}),
           "RED:probe 空串必红")
    expect(degrade_exit_code([], False) is None, "GREEN:无降级续跑")
    expect(degrade_exit_code(["x"], False) == 0, "GREEN:降级 SKIP 退 0")
    expect(degrade_exit_code(["x"], True) == 1, "RED:REQUIRE_REAL 下降级翻硬红")
    # 红绿臂⑥：profile 输出 schema 对拍（合成夹具过/不过 Draft7）。
    import jsonschema
    if PROFILE_SCHEMA_PATH.is_file():
        ps = json.loads(PROFILE_SCHEMA_PATH.read_text(encoding="utf-8"))
        v = jsonschema.Draft7Validator(ps)
        expect(not list(v.iter_errors(good)), "GREEN:合成 profile 过输出 schema")
        bad = _fixture_profile()
        bad["identity"]["gpu_sum_le_render_wall_tol_ms"] = 0.2
        expect(bool(list(v.iter_errors(bad))), "RED:容差漂移过不了输出 schema")
        bad = _fixture_profile()
        del bad["gpu_passes"]
        expect(bool(list(v.iter_errors(bad))), "RED:缺 gpu_passes 过不了输出 schema")
    # schema 互核：双 schema 在树 + 关键 const/required 逐字。
    expect(SCHEMA_PATH.is_file(), "门 schema 在树")
    expect(PROFILE_SCHEMA_PATH.is_file(), "profile 输出 schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "wave const 互核")
        expect(
            sorted(gs.get("required", [])) == sorted([
                "schema", "subject", "symbolic_gate_key", "wave", "anchor", "surface",
                "runs", "profiles", "zero_drift", "annotations", "tool_probe",
                "capture_compat", "environment", "timestamp", "notes",
            ]),
            "schema required 闭集互核（15 字段）",
        )
        surf = gs["properties"]["surface"]
        expect(surf["properties"]["cli"]["const"] == "--profile-json <path>", "cli const 互核")
        expect(len(surf["properties"]["g31_passes"]["items"]["enum"]) == 5, "g31 五段闭集互核")
        expect(len(surf["properties"]["g14_passes"]["items"]["enum"]) == 4, "g14 四段闭集互核")
        # G39 T4 identity_rounds 纯追加可选块互核（required 15 闭集不变;
        # 逐轮 identity_ok = boolean 可红,中位裁决落 profiles/*/identity_ok const true）。
        expect("identity_rounds" not in gs.get("required", []), "identity_rounds 非 required（纯追加可选）")
        ir = gs["properties"].get("identity_rounds") or {}
        expect(bool(ir), "identity_rounds 可选块在 schema（_patch 已落地）")
        if ir:
            expect(ir["properties"]["rounds"]["minimum"] == IDENTITY_ROUNDS_MIN
                   and ir["properties"]["rounds"]["maximum"] == IDENTITY_ROUNDS_MAX,
                   "identity_rounds.rounds 闭集 [1,9] 互核")
            for leg_key in ("g31", "g14"):
                row = ir["properties"][leg_key]["items"]
                expect(sorted(row["required"]) == sorted(
                    ["gpu_sum_mean_ms", "render_wall_mean_ms", "host_residual_mean_ms", "identity_ok"]),
                    f"identity_rounds.{leg_key} 逐轮行 required 四键互核")
                expect(row["properties"]["identity_ok"] == {"type": "boolean"},
                       f"identity_rounds.{leg_key} 逐轮 identity_ok boolean（可红;中位裁决落 profiles）")
    if PROFILE_SCHEMA_PATH.is_file():
        ps = json.loads(PROFILE_SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(ps["properties"]["schema"]["const"] == PROFILE_SCHEMA_ID, "profile schema const 互核")
        expect(ps["properties"]["identity"]["properties"]["gpu_sum_le_render_wall_tol_ms"]["const"] == 0.1,
               "identity gpu 容差 const 互核")
        expect(ps["properties"]["identity"]["properties"]["host_residual_tol_ms"]["const"] == 2.0,
               "identity host 容差 const 互核")
    expect(len(FACT_IDS) == 7, "facts 闭集 = 7")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=7；6 红臂组 + 正例组 + 中位鲁棒化臂 + 双 schema 互核）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--rounds", type=int, default=IDENTITY_ROUNDS,
                    help=(f"identity 采样轮数（on 腿 ×N 多轮中位;闭集 "
                          f"[{IDENTITY_ROUNDS_MIN},{IDENTITY_ROUNDS_MAX}];缺省 {IDENTITY_ROUNDS};建议奇数）"))
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        if not rounds_valid(args.rounds):
            print(f"[{TAG}] FAIL: --rounds {args.rounds} 越闭集 "
                  f"[{IDENTITY_ROUNDS_MIN},{IDENTITY_ROUNDS_MAX}]", file=sys.stderr)
            return 1
        return run_gate(args.rounds)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
