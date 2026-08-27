#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 B Task B5 蒙皮/骨骼动画进生产帧）
"""G31+ 波 B Task B5：蒙皮/骨骼动画进生产帧门冒烟（g31.waveB.skinning；
G31_PLUS_COMMERCIAL_RENDERER_TODO §1.3 #10 行；RD-041 backfill「蒙皮 WPO MV
在动态资产面出现时」触发面——本任务即兑现窗）。

判据（任务书逐字）：
- device 蒙皮接生产：骨骼 palette cur/prev 双表逐帧 buffer_uploads 上传，
  device LBS 蒙皮 pass（kernels/g31_skin.rx）写 tris SSBO 角色段 + prev 顶点
  表，FrameUpdate::blas_refit 桥（vkCmdCopyBuffer 蒙皮段 → 角色 BLAS 顶点缓
  冲 + 原地 UPDATE build + consume barrier，创建期 updatable 打标）——蒙皮
  后顶点缓冲 + AS refit 通路（接入面取舍见 evidence notes）。
- 动态角色进画面：3 骨两段臂 + 关节融合套蒙皮角色，脚本化骨骼动画驱动
  （root 三轴正弦 + 肩/肘 z 摆，帧号确定性 f32 函数）--skin-demo 逐帧真跑。
- 蒙皮/WPO MV 通道（RD-041 三类速度设计）：类 3 蒙皮 MV（prev 顶点 bary
  插值 → prev_vp 投影）经 g31_skin_mv 进 TSR 历史链；核验 = 检测像素域
  dev 中位数 vs host 逐顶点中位数逐分量 ≤2px + 窗级聚合真动门（max host
  ≥1.0px;动画冻结必红;低动相位逐帧不误伤）+ 高动帧条件 ratio 门（dev
  ≥0.5×host）+ 静态区无污染门 ≤1.5px；类 2 刚性实例缺口维持登记（不冒充接通）。
- 门维持：M92 双臂对拍门（host 金标准 vs device）接线态复跑 PASS；确定性
  双跑 digest 位级一致；skin vs 静态 digest 必异。
- measured：skin on/off（角色 + 骨骼逐帧更新 vs 同窗静态 bench）frame_ms
  对照 + skin/scene/mv GPU 三分解，真实命令输出。
- 静态场景锚零漂移：bistro 静态契约 bench（160 帧 canonical）digest ==
  milestones/g14/g14_3_stage_a_digest_anchor.json 锚（B5 落地静态面 0-byte）。

三态：无 Vulkan loader/设备/场景资产 → 输出 DEV_ENV_DEGRADE 退 0（不冒充
PASS）；本脚本真跑臂 RURIX_REQUIRE_REAL=1（该态下缺真实面即 FAIL 退 1，
禁 mock 充真跑——g31_dynamic_scene_smoke 同语义）。

用法：
  py -3 ci/g31_skinning_wiring_smoke.py --selftest
  py -3 ci/g31_skinning_wiring_smoke.py --gate g31.waveB.skinning \
      [--runs 2] [--frames 100] [--warmup 10] [--out <evidence.json>]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import math
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveB.skinning"
TAG = "g31_skinning"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_skinning_wiring_evidence_schema.json"
SCHEMA_ID = "rurix.g31.skinning_wiring_evidence.v1"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"
M92_BIN = ROOT / "target" / "release" / "g9_m92_skinning_device.exe"
WORK = ROOT / ".tmp" / "g31_gates" / "skinning"
OUT_ROOT = WORK / "out"
SCENE = "bistro-interior"
TIER = 100
BACKEND = "tsr_device"
TRACE_FRAMES = 24
TRACE_WARMUP = 4
STATIC_ANCHOR_FRAMES = 160  # g14 Stage A 锚 canonical 帧数（锚收割口径）
STATIC_ANCHOR_WARMUP = 10

# 与 bin 侧 g14_3_lane_body.rs G31+ Task B5 常量区逐字同源（动画/容差规格）。
SKIN_TOL_CENTROID_PX = 4.0
SKIN_TOL_AABB_PX = 6.0
SKIN_MV_TOL_MEDIAN_PX = 2.0
SKIN_MV_HOST_MOTION_MIN_PX = 1.0
SKIN_MV_DEV_RATIO_MIN = 0.5
SKIN_MV_STATIC_MAX_PX = 1.5
OBS_MOTION_MIN_PX = 3.0
PALETTE_REDERIVE_TOL = 1e-4
SKIN_VERIFY_SCHEMA = "rurix.g31.skin_verify.v1"

SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
SKIN_SPVS = ("g31_skin", "g31_skin_scene", "g31_skin_mv")

FAILURES: list[str] = []


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


# ---------------------------------------------------------------- 纯函数判据面
def percentile(sorted_v: list[float], q: float) -> float:
    """s 已升序；q∈[0,1] 最近秩（与 A4/A2 分析脚本同一口径）。"""
    if not sorted_v:
        raise ValueError("percentile: 空样本")
    n = len(sorted_v)
    return sorted_v[min(n - 1, int(q * (n - 1) + 0.5))]


def arm_stats(frame_ms: list[float]) -> dict:
    """单轮 frame_ms 全列 → mean/p50/p99/min/max（末一样本含末帧 digest
    tail——两臂同形同价；核验帧回读税计入 tail，两臂同形）。"""
    if not frame_ms:
        raise ValueError("arm_stats: 空 frame_ms")
    s = sorted(frame_ms)
    n = len(frame_ms)
    p50 = s[n // 2] if n % 2 else (s[n // 2 - 1] + s[n // 2]) / 2.0
    return {
        "mean": sum(frame_ms) / n,
        "p50": p50,
        "p99": percentile(s, 0.99),
        "min": s[0],
        "max": s[-1],
    }


def digests_bitexact(digest_lists: list[list[str]]) -> bool:
    """多轮 digest 列：全部展平后集合恰一元（位级一致判据）。"""
    flat = [d for ds in digest_lists for d in ds]
    return bool(flat) and len(set(flat)) == 1


def seqs_bitexact(seqs: list[list[str]]) -> bool:
    """多臂逐帧 digest **序列**逐位对拍：等长且逐下标全同（A2 同律）。"""
    if not seqs or any(len(s) != len(seqs[0]) for s in seqs):
        return False
    return all(len({s[i] for s in seqs}) == 1 for i in range(len(seqs[0])))


def frame_order_ok(trace_rows: list[dict]) -> bool:
    """flip-trace 帧号严格 0..N−1。"""
    return [int(r["frame"]) for r in trace_rows] == list(range(len(trace_rows)))


# ── 骨骼动画 palette 的 python 独立重导（bin 侧 xf_translate/xf_rotz/
# xf_compose/skin_palette 逐字同式——核验 skin_verify.json palette 非伪造的
# 第三臂;compose 累加序 = 平移项先行 + k=0..2 顺加,与 bin 冻结序同）──
def _xf_t(x: float, y: float, z: float) -> list[list[float]]:
    return [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]


def _xf_rz(a: float) -> list[list[float]]:
    s, c = math.sin(a), math.cos(a)
    return [[c, -s, 0.0, 0.0], [s, c, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]]


def _xf_compose(a: list[list[float]], b: list[list[float]]) -> list[list[float]]:
    o = [[0.0] * 4 for _ in range(3)]
    for r in range(3):
        for c in range(4):
            s = a[r][3] if c == 3 else 0.0
            for k in range(3):
                s += a[r][k] * b[k][c]
            o[r][c] = s
    return o


def rederive_palette(frame: int, anim: dict) -> list[float]:
    """动画规格 → 3 骨 palette 展平 36 f32（bin `skin_palette` 的 python 独立
    重导;tol 1e-4——f32 sin/cos 与 python 的 ulp 级分歧吸收面;双谐波面与
    bin 常量区逐字同源）。"""
    t = float(frame)
    amp = [float(x) for x in anim["root_amp"]]
    freq = [float(x) for x in anim["root_freq"]]
    amp2 = [float(x) for x in anim["root_amp2"]]
    freq2 = [float(x) for x in anim["root_freq2"]]
    phase2 = [float(x) for x in anim["root_phase2"]]
    origin = [float(x) for x in anim["origin"]]
    d = [
        amp[0] * math.sin(freq[0] * t) + amp2[0] * math.sin(freq2[0] * t + phase2[0]),
        amp[1] * math.sin(freq[1] * t + 1.0),
        amp[2] * (math.cos(freq[2] * t) - 1.0),
    ]
    a1 = float(anim["swing_amp"]) * math.sin(float(anim["swing_freq"]) * t) + float(
        anim["swing_amp2"]
    ) * math.sin(float(anim["swing_freq2"]) * t + float(anim["swing_phase2"]))
    a2 = float(anim["elbow_amp"]) * math.sin(
        float(anim["elbow_freq"]) * t + 0.5
    ) + float(anim["elbow_amp2"]) * math.sin(
        float(anim["elbow_freq2"]) * t + float(anim["elbow_phase2"])
    )
    root = _xf_t(d[0], d[1], d[2])
    to = _xf_t(origin[0], origin[1], origin[2])
    back = _xf_t(-origin[0], -origin[1], -origin[2])
    m1 = _xf_compose(root, _xf_compose(to, _xf_compose(_xf_rz(a1), back)))
    e = [origin[0], origin[1] + float(anim["upper_len"]), origin[2]]
    te = _xf_t(e[0], e[1], e[2])
    be = _xf_t(-e[0], -e[1], -e[2])
    m2 = _xf_compose(m1, _xf_compose(te, _xf_compose(_xf_rz(a2), be)))
    out: list[float] = []
    for m in (root, m1, m2):
        for r in range(3):
            for c in range(4):
                out.append(m[r][c])
    return out


def validate_skin_verify(doc: dict) -> list[str]:
    """skin_verify.json 逐项判（返回失败串列表,空 = 绿;--selftest 合成夹具
    同消费）：schema/帧数/逐帧 palette 重导/容差/MV 门/obs 运动。"""
    fails: list[str] = []
    if not isinstance(doc, dict):
        return ["skin_verify 非 object"]
    if doc.get("schema") != SKIN_VERIFY_SCHEMA:
        fails.append(f"skin_verify schema 非法: {doc.get('schema')!r}")
    anim = doc.get("animation") or {}
    for k in ("root_amp", "root_freq", "root_amp2", "root_freq2", "root_phase2",
              "swing_amp", "swing_freq", "swing_amp2", "swing_freq2", "swing_phase2",
              "elbow_amp", "elbow_freq", "elbow_amp2", "elbow_freq2", "elbow_phase2",
              "upper_len", "origin", "bone_count", "tri_count",
              "vertex_count", "emission", "albedo"):
        if k not in anim:
            fails.append(f"animation 缺 {k}")
    tol = doc.get("tolerance") or {}
    if abs(float(tol.get("centroid_px", -1)) - SKIN_TOL_CENTROID_PX) > 1e-9:
        fails.append("tolerance.centroid_px 漂移")
    if abs(float(tol.get("aabb_px", -1)) - SKIN_TOL_AABB_PX) > 1e-9:
        fails.append("tolerance.aabb_px 漂移")
    if abs(float(tol.get("mv_median_px", -1)) - SKIN_MV_TOL_MEDIAN_PX) > 1e-9:
        fails.append("tolerance.mv_median_px 漂移")
    frames = doc.get("frames")
    if not isinstance(frames, list) or not frames:
        fails.append("frames 空/非数组")
        frames = []
    if doc.get("frames_verified") != len(frames):
        fails.append(f"frames_verified {doc.get('frames_verified')!r} ≠ len(frames) {len(frames)}")
    if doc.get("all_pass") is not True:
        fails.append("all_pass ≠ true")
    obs_pts: list[tuple[float, float]] = []
    motion_max = 0.0
    for fr in frames:
        tag = f"frame {fr.get('frame')!r}"
        if fr.get("pass") is not True:
            fails.append(f"{tag} pass ≠ true")
        pal = fr.get("palette")
        if not isinstance(pal, list) or len(pal) != 36:
            fails.append(f"{tag} palette 形态非法")
        elif anim:
            exp = rederive_palette(int(fr["frame"]), anim)
            for k in range(36):
                if abs(float(pal[k]) - exp[k]) > PALETTE_REDERIVE_TOL:
                    fails.append(
                        f"{tag} palette[{k}] {float(pal[k]):.9g} 与动画重导 {exp[k]:.9g} 偏差超阈"
                    )
                    break
        cd = float(fr.get("centroid_delta_px", 1e30))
        ad = float(fr.get("aabb_delta_px", 1e30))
        if not (cd <= SKIN_TOL_CENTROID_PX):
            fails.append(f"{tag} 质心偏差 {cd:.3f} > {SKIN_TOL_CENTROID_PX}px")
        if not (ad <= SKIN_TOL_AABB_PX):
            fails.append(f"{tag} AABB 偏差 {ad:.3f} > {SKIN_TOL_AABB_PX}px")
        if int(fr.get("obs_count", 0)) < 200:
            fails.append(f"{tag} obs_count {fr.get('obs_count')!r} < 200（角色丢失面）")
        op = fr.get("obs_px")
        if not isinstance(op, list) or len(op) != 2 or not all(
            isinstance(v, (int, float)) and math.isfinite(float(v)) for v in op
        ):
            fails.append(f"{tag} obs_px 非有限对（角色未检出）")
        else:
            obs_pts.append((float(op[0]), float(op[1])))
        # MV 门（bin 侧 pass 判据逐字同源）：逐分量绝对差（全帧）+ 条件 ratio
        #（host ≥ 1.0px 高动帧才激活——低动相位信噪比低于 jitter 残留,放空）
        # + 静态区无污染；真动判据 = 窗级聚合（循环后,与 bin all_pass 同式）。
        mdd = fr.get("mv_median_delta_px")
        if not isinstance(mdd, list) or len(mdd) != 2 or not all(
            float(v) <= SKIN_MV_TOL_MEDIAN_PX for v in mdd
        ):
            fails.append(f"{tag} mv_median_delta_px {mdd!r} 超阈/形态非法")
        hm = fr.get("mv_host_motion_px")
        dm = fr.get("mv_dev_motion_px")
        if not isinstance(hm, (int, float)) or not math.isfinite(float(hm)):
            fails.append(f"{tag} mv_host_motion_px {hm!r} 形态非法")
            hm = 0.0
        if not isinstance(dm, (int, float)) or not math.isfinite(float(dm)):
            fails.append(f"{tag} mv_dev_motion_px {dm!r} 形态非法")
            dm = 0.0
        host_motion = float(hm)
        dev_motion = float(dm)
        if host_motion >= SKIN_MV_HOST_MOTION_MIN_PX and not (
            dev_motion >= SKIN_MV_DEV_RATIO_MIN * host_motion
        ):
            fails.append(
                f"{tag} 高动帧 dev MV {dev_motion:.3f} < {SKIN_MV_DEV_RATIO_MIN}×host "
                f"{host_motion:.3f}（MV 通道未载蒙皮运动）"
            )
        motion_max = max(motion_max, host_motion)
        if float(fr.get("static_mv_median_abs_px", 1e30)) > SKIN_MV_STATIC_MAX_PX:
            fails.append(f"{tag} 静态区 MV {fr.get('static_mv_median_abs_px')!r} > {SKIN_MV_STATIC_MAX_PX}px（覆盖臂污染）")
    # 窗级聚合真动门（bin all_pass 逐字同源,防「确定性的坏内容」）：max
    # host_motion ≥ 1.0px——动画冻结（palette 未更新）⇒ 全帧 ≈0 必红;双谐波
    # 窗内 max 实测 ~3px 远离阈;低动相位逐帧不误伤。
    if frames and not (motion_max >= SKIN_MV_HOST_MOTION_MIN_PX):
        fails.append(
            f"窗级聚合真动门破缺: max host_motion {motion_max:.3f}px < "
            f"{SKIN_MV_HOST_MOTION_MIN_PX}px（动画未真动/核验窗全低动相位）"
        )
    mg = doc.get("motion_gate")
    if isinstance(mg, dict) and "host_motion_max_px" in mg:
        if abs(float(mg["host_motion_max_px"]) - motion_max) > 1e-6:
            fails.append(
                f"motion_gate.host_motion_max_px {mg['host_motion_max_px']!r} "
                f"与逐帧重算 {motion_max:.6f} 不符（伪造面）"
            )
    # obs 运动判据（防「确定性的坏内容」——画面角色位置必须真动）。
    if len(obs_pts) >= 2:
        motion = max(
            math.hypot(obs_pts[i][0] - obs_pts[j][0], obs_pts[i][1] - obs_pts[j][1])
            for i in range(len(obs_pts))
            for j in range(i + 1, len(obs_pts))
        )
        if motion < OBS_MOTION_MIN_PX:
            fails.append(f"obs 质心最大位移 {motion:.3f}px < {OBS_MOTION_MIN_PX}px（画面未真动）")
    else:
        fails.append("obs 点不足 2（运动判据不可评）")
    return fails


def evidence_required_keys(doc: dict) -> list[str]:
    """schema required 闭集核验（jsonschema 依赖免;check_schemas.py 另作
    形式校验面）。"""
    required = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))["required"]
    return [k for k in required if k not in doc]


# ---------------------------------------------------------------- 真跑驱动
def _rurixc() -> Path | None:
    """rurixc vulkan-backend 构建面（缺则构建一次;g14/A4 smoke 同型）。"""
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if exe.is_file():
        return exe
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"],
        cwd=ROOT, capture_output=True, text=True, timeout=7200,
    )
    return exe if r.returncode == 0 and exe.is_file() else None


def ensure_skin_spv() -> bool:
    """蒙皮三 kernel SPV 存在性保障（缺则编译;.tmp 构建产物不入 git）。"""
    rurixc = _rurixc()
    if rurixc is None:
        return False
    SPV_DIR.mkdir(parents=True, exist_ok=True)
    for name in SKIN_SPVS:
        dst = SPV_DIR / f"{name}.spv"
        if dst.is_file():
            continue
        r = subprocess.run(
            [str(rurixc), str(KERNEL_DIR / f"{name}.rx"), "--target", "vulkan", "-o", str(dst)],
            cwd=ROOT, capture_output=True, text=True, timeout=1800,
        )
        if r.returncode != 0 or not dst.is_file():
            return False
    return True


def run_bench(
    skin: bool,
    frames: int,
    warmup: int,
    out_root: Path,
    flip_trace_dir: Path | None,
    require_real: bool,
    debug_tris: bool = False,
) -> dict:
    """单轮真跑（skin=True = --skin-demo 蒙皮臂;False = 静态 bench 锚/对照臂）。"""
    env = dict(os.environ)
    if require_real:
        env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    if debug_tris:
        env["RURIX_SKIN_DEBUG_TRIS"] = "1"
    elif "RURIX_SKIN_DEBUG_TRIS" in env:
        del env["RURIX_SKIN_DEBUG_TRIS"]
    if flip_trace_dir is not None:
        flip_trace_dir.mkdir(parents=True, exist_ok=True)
        env["RURIX_G14_FLIP_TRACE"] = str(flip_trace_dir)
    elif "RURIX_G14_FLIP_TRACE" in env:
        del env["RURIX_G14_FLIP_TRACE"]
    cmd = [
        str(BIN),
        "--bench",
        "--scene", SCENE,
        "--tier", str(TIER),
        "--backend", BACKEND,
        "--frames", str(frames),
        "--warmup", str(warmup),
        "--out-root", str(out_root),
    ]
    if skin:
        cmd += ["--skin-demo"]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=7200, env=env)
    out = (r.stdout or "") + (r.stderr or "")
    receipt_path = out_root / SCENE / f"tier{TIER}" / BACKEND / "bench_receipt.json"
    verify_path = out_root / SCENE / f"tier{TIER}" / BACKEND / "skin_verify.json"
    receipt = {}
    verify = {}
    fresh = False
    if receipt_path.is_file():
        fresh = receipt_path.stat().st_mtime >= t0 - 5.0
        try:
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            receipt = {}
    if verify_path.is_file() and verify_path.stat().st_mtime >= t0 - 5.0:
        try:
            verify = json.loads(verify_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            verify = {}
    pass_line = re.search(r"BENCH PASS scene=(\S+)", out)
    skin_gpu = re.search(r"SKIN_GPU_MS mean=([0-9.eE+-]+)", out)
    clean = (
        r.returncode == 0
        and pass_line is not None
        and fresh
        and "VALIDATION" not in out
        and "leak" not in out.lower()
    )
    return {
        "rc": r.returncode,
        "clean_shutdown": clean,
        "receipt": receipt,
        "verify": verify,
        "receipt_path": receipt_path,
        "fresh": fresh,
        "tail": out[-400:],
        "skipped_dev_env": "skipped_dev_env" in out,
        "skin_gpu_ms_mean": float(skin_gpu.group(1)) if skin_gpu else None,
        "debug_bitexact_abs": (
            [float(m) for m in re.findall(
                r"SKIN_DEBUG_TRIS 帧 \d+ device vs host max_abs=([0-9.eE+-]+)", out
            )] if debug_tris else None
        ),
    }


def load_trace(trace_dir: Path) -> list[dict]:
    p = trace_dir / f"frame_digests_{SCENE}_t{TIER}_{BACKEND}.jsonl"
    if not p.is_file():
        return []
    return [json.loads(line) for line in p.read_text(encoding="utf-8").splitlines() if line.strip()]


# ---------------------------------------------------------------- selftest
def selftest() -> int:
    note("selftest：判据纯函数红绿臂")
    ok = True

    # ① arm_stats 绿臂：已知向量核对 mean/p50/p99/min/max。
    v = [4.0, 1.0, 2.0, 3.0, 100.0]
    st = arm_stats(v)
    expect = {"mean": 22.0, "p50": 3.0, "p99": 100.0, "min": 1.0, "max": 100.0}
    green = st == expect
    ok &= green
    note(f"  arm_stats 绿臂: {'PASS' if green else 'FAIL'}（{st}）")

    # ② arm_stats 红臂：空样本必拒（ValueError）。
    try:
        arm_stats([])
        red = False
    except ValueError:
        red = True
    ok &= red
    note(f"  arm_stats 红臂（空样本拒）: {'PASS' if red else 'FAIL'}")

    # ③ digests_bitexact 绿臂：双臂全同 → True。
    green = digests_bitexact([["sha256:a", "sha256:a"], ["sha256:a"]])
    ok &= green
    note(f"  digests_bitexact 绿臂: {'PASS' if green else 'FAIL'}")

    # ④ digests_bitexact 红臂：任一漂移 → False（检出即红,不静默）。
    red = not digests_bitexact([["sha256:a"], ["sha256:b"]])
    ok &= red
    note(f"  digests_bitexact 红臂（漂移检出）: {'PASS' if red else 'FAIL'}")

    # ⑤ seqs_bitexact 绿/红臂。
    green = seqs_bitexact([["d0", "d1", "d2"], ["d0", "d1", "d2"]])
    red = not seqs_bitexact([["d0", "d1", "d2"], ["d0", "dX", "d2"]]) and not seqs_bitexact(
        [["d0"], ["d0", "d1"]]
    )
    ok &= green and red
    note(f"  seqs_bitexact 绿臂: {'PASS' if green else 'FAIL'}；红臂: {'PASS' if red else 'FAIL'}")

    # ⑥ frame_order_ok 绿/红臂。
    green = frame_order_ok([{"frame": 0}, {"frame": 1}, {"frame": 2}])
    red = not frame_order_ok([{"frame": 0}, {"frame": 2}, {"frame": 1}])
    ok &= green and red
    note(f"  frame_order_ok 绿臂: {'PASS' if green else 'FAIL'}；红臂: {'PASS' if red else 'FAIL'}")

    # ⑦ rederive_palette 口径锚：frame 0/2/10 与 bin 侧 skin_palette 同式重导
    # 的预收割锚值对拍（容差 1e-6;锚值 = python 重导首轮自收割）。
    anim = {
        "root_amp": [0.05, 0.02, 0.05],
        "root_freq": [0.05, 0.037, 0.043],
        "root_amp2": [0.03, 0.0, 0.0],
        "root_freq2": [0.13, 0.0, 0.0],
        "root_phase2": [0.7, 0.0, 0.0],
        "swing_amp": 0.20,
        "swing_freq": 0.07,
        "swing_amp2": 0.15,
        "swing_freq2": 0.19,
        "swing_phase2": 1.3,
        "elbow_amp": 0.55,
        "elbow_freq": 0.11,
        "elbow_amp2": 0.35,
        "elbow_freq2": 0.23,
        "elbow_phase2": 2.1,
        "upper_len": 0.216,
        "origin": [2.286061, 0.7472407, -1.6825298],
        "bone_count": 3,
        "tri_count": 36,
        "vertex_count": 108,
        "emission": [400.0, 0.0, 400.0],
        "albedo": [0.18, 0.18, 0.2],
    }
    p0 = rederive_palette(0, anim)
    p2 = rederive_palette(2, anim)
    p10 = rederive_palette(10, anim)
    green = (
        abs(p0[3] - 1.932653062e-2) < 1e-6
        and abs(p0[12] - 9.895731711e-1) < 1e-6
        and abs(p0[24] - 7.581395224e-1) < 1e-6
        and abs(p2[12] - 9.843736940e-1) < 1e-6
        and abs(p2[15] - 1.968732971e-1) < 1e-6
        and abs(p10[24] - 9.438195825e-1) < 1e-6
        and abs(p10[31] - (-6.832966003e-1)) < 1e-6
    )
    ok &= green
    note(f"  rederive_palette 口径锚绿臂: {'PASS' if green else 'FAIL'}")

    # ⑧ validate_skin_verify 绿臂：合成合法件（palette 由重导产,obs 随轨迹
    # 真动,MV 全门绿——高动帧 host 1.83 ≥1.0 且 dev 1.7 ≥0.5×host,窗级聚合
    # max=1.83 ≥1.0）必须零失败。
    def synth_verify() -> dict:
        frames = []
        for i, f in enumerate(range(10, 60, 10)):
            frames.append({
                "frame": f,
                "palette": rederive_palette(f, anim),
                "pred_px": [950.0 + 3.0 * i, 760.0 + 1.0 * i],
                "pred_aabb": [920.0 + 3.0 * i, 630.0 + i, 990.0 + 3.0 * i, 890.0 + i],
                "obs_px": [950.4 + 3.0 * i, 760.3 + 1.0 * i],
                "obs_aabb": [920.0 + 3.0 * i, 631.0 + i, 990.0 + 3.0 * i, 890.0 + i],
                "obs_count": 9000,
                "centroid_delta_px": 0.7,
                "aabb_delta_px": 1.0,
                "mv_dev_median_px": [1.5, 0.8],
                "mv_host_median_px": [1.6, 0.9],
                "mv_median_delta_px": [0.1, 0.1],
                "mv_host_motion_px": 1.83,
                "mv_dev_motion_px": 1.70,
                "static_mv_median_abs_px": 0.7,
                "pass": True,
            })
        return {
            "schema": SKIN_VERIFY_SCHEMA,
            "scene_id": SCENE,
            "tier": TIER,
            "backend": BACKEND,
            "animation": dict(anim),
            "tolerance": {
                "centroid_px": SKIN_TOL_CENTROID_PX,
                "aabb_px": SKIN_TOL_AABB_PX,
                "mv_median_px": SKIN_MV_TOL_MEDIAN_PX,
                "min_count_area_ratio": 0.15,
                "mv_host_motion_min_px": SKIN_MV_HOST_MOTION_MIN_PX,
                "mv_dev_ratio_min": SKIN_MV_DEV_RATIO_MIN,
                "static_mv_max_px": SKIN_MV_STATIC_MAX_PX,
            },
            "frames": frames,
            "frames_verified": len(frames),
            "motion_gate": {
                "host_motion_max_px": 1.83,
                "threshold_px": SKIN_MV_HOST_MOTION_MIN_PX,
                "note": "selftest 合成",
            },
            "all_pass": True,
        }

    green = validate_skin_verify(synth_verify()) == []
    ok &= green
    note(f"  validate_skin_verify 绿臂: {'PASS' if green else 'FAIL'}")

    # ⑨ validate_skin_verify 红臂 ×6：palette 篡改 / 质心超阈 / obs 不动 /
    # MV 通道未载运动（高动帧 dev 近零）/ 窗级真动门（动画冻结全帧低动）/
    # all_pass=false 冒充——逐条必须红。
    reds: list[tuple[str, dict]] = []
    bad = synth_verify()
    bad["frames"][2]["palette"][3] += 0.5
    reds.append(("palette 篡改", bad))
    bad = synth_verify()
    bad["frames"][1]["centroid_delta_px"] = 9.9
    reds.append(("质心容差超阈", bad))
    bad = synth_verify()
    for fr in bad["frames"]:
        fr["obs_px"] = [950.4, 760.3]
    reds.append(("obs 不动（坏内容）", bad))
    bad = synth_verify()
    for fr in bad["frames"]:
        fr["mv_dev_motion_px"] = 0.014
    reds.append(("MV 通道未载运动（高动帧 dev 近零）", bad))
    bad = synth_verify()
    for fr in bad["frames"]:
        fr["mv_host_motion_px"] = 0.5
        fr["mv_dev_motion_px"] = 0.4
    bad["motion_gate"]["host_motion_max_px"] = 0.5
    reds.append(("窗级真动门（动画冻结）", bad))
    bad = synth_verify()
    bad["frames"][0]["pass"] = False
    reds.append(("单帧 pass=false", bad))
    missed = [name for name, fx in reds if not validate_skin_verify(fx)]
    red = not missed
    ok &= red
    note(
        f"  validate_skin_verify 红臂（{len(reds)} 类构造缺陷）: "
        f"{'PASS' if red else 'FAIL'}" + (f"（漏检 {missed}）" if missed else "")
    )

    # ⑩ evidence required 键闭集：合成 doc 缺键必列出。
    missing = evidence_required_keys({"schema": SCHEMA_ID})
    red = len(missing) > 0 and "arms" in missing and "static_anchor_match" in missing
    ok &= red
    note(f"  evidence required 键红臂（缺键列出 {len(missing)} 项）: {'PASS' if red else 'FAIL'}")

    # ⑪ percentile 口径锚：n=100 时 int(0.99×99+0.5)=98、int(0.5×99+0.5)=50。
    s100 = list(range(100))
    green = percentile(s100, 0.99) == 98 and percentile(s100, 0.5) == 50
    ok &= green
    note(f"  percentile 口径锚绿臂: {'PASS' if green else 'FAIL'}")

    if ok:
        note("SELFTEST PASS（红绿臂全如预期）")
        return 0
    print(f"[{TAG}] SELFTEST FAIL", file=sys.stderr)
    return 1


# ---------------------------------------------------------------- gate
def gate(runs: int, frames: int, warmup: int, out_path: Path | None) -> int:
    if not SCHEMA_PATH.is_file():
        fail(f"schema 缺失: {SCHEMA_PATH}")
        return 1
    if not ANCHOR_PATH.is_file():
        fail(f"g14 Stage A digest 锚缺失: {ANCHOR_PATH}")
        return 1
    note(
        f"gate {GATE_KEY}: scene={SCENE} tier={TIER} backend={BACKEND} "
        f"runs/skin={runs} frames={frames} warmup={warmup}"
    )

    with gpu_device_lock(purpose=f"{TAG} device 真跑（g31.waveB.skinning）"):
        # 构建（release;g14_3_pipeline_perf 需 vendor-upscale feature + M92
        # 对拍 harness 门维持臂）。
        build = subprocess.run(
            ["cargo", "build", "-p", "rurix-render", "--features", "vulkan,vendor-upscale",
             "--release", "--bin", "g14_3_pipeline_perf", "--bin", "g9_m92_skinning_device"],
            cwd=ROOT, capture_output=True, text=True, timeout=7200,
        )
        if build.returncode != 0 or not BIN.is_file() or not M92_BIN.is_file():
            fail(f"release 构建失败: {(build.stderr or '')[-400:]}")
            return 1
        if not ensure_skin_spv():
            fail("蒙皮 kernel SPV 编译失败（kernels/g31_{skin,skin_scene,skin_mv}.rx）")
            return 1

        # dev-env 探针（不挂 REQUIRE_REAL：缺真实面 → bin 自报 skipped_dev_env 退 0）。
        # 帧窗 = 11+10wu：核验帧 i=10/20——窗级聚合真动门要求窗内含高动帧
        # （双谐波低动相位 frame 1/2 med 0.76/0.48px;短窗探针 2+1 必然全低动
        # 相位误判,11+10 窗 max med 1.53px 远离阈,与主臂同窗同门）。
        probe = run_bench(True, 11, 10, WORK / "probe", None, require_real=False)
        if probe["skipped_dev_env"] or (probe["rc"] == 0 and not probe["fresh"]):
            print(json.dumps({
                "schema": "rurix.g31.skinning_wiring.skip.v1",
                "state": "DEV_ENV_DEGRADE",
                "what": "vulkan_device_or_scene_assets",
                "reason": probe["tail"][-200:],
            }, ensure_ascii=False))
            note("DEV_ENV_DEGRADE（无 Vulkan/设备/场景资产——退 0 不冒充 PASS）")
            return 0
        if probe["rc"] != 0:
            fail(f"dev-env 探针真跑失败: {probe['tail'][-200:]}")
            return 1
        note("dev-env 探针绿（真机真跑面成立）")

        # ── M92 双臂对拍门维持臂（接线态复跑;验证件本体语义零破坏）──
        m92_ev = WORK / "m92_evidence.json"
        m92_ev.parent.mkdir(parents=True, exist_ok=True)
        m92 = subprocess.run(
            [str(M92_BIN), "--evidence", str(m92_ev)],
            cwd=ROOT, capture_output=True, text=True, timeout=7200,
            env={**os.environ, "RURIX_REQUIRE_REAL": "1"},
        )
        m92_out = (m92.stdout or "") + (m92.stderr or "")
        m92_doc = {}
        if m92_ev.is_file():
            try:
                m92_doc = json.loads(m92_ev.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                m92_doc = {}
        m92_pass = (
            m92.returncode == 0
            and "G9_M92_SKIN: PASS" in m92_out
            and isinstance(m92_doc.get("checks"), dict)
            and all(m92_doc["checks"].values())
        )
        if not m92_pass:
            fail(f"M92 双臂对拍门接线态复跑破缺: rc={m92.returncode} tail={m92_out[-300:]}")
            return 1
        m92_run_digest = str(m92_doc.get("digests", {}).get("run_a", ""))
        note(f"  M92 门维持: PASS run_digest={m92_run_digest[:23]}…")

        # ── device/host 蒙皮对拍诊断臂（RURIX_SKIN_DEBUG_TRIS 真跑收割;
        # 蒙皮输出逐顶点 max_abs == 0 判据;帧窗 11+10wu 同窗同门——核验帧
        # i=10/20 挂 debug 回读,窗级聚合真动门 max med 1.53px 绿）──
        dbg = run_bench(True, 11, 10, WORK / "debug_tris", None, require_real=True,
                        debug_tris=True)
        device_host_bitexact = bool(
            dbg["rc"] == 0
            and dbg["debug_bitexact_abs"]
            and all(x == 0.0 for x in dbg["debug_bitexact_abs"])
        )
        if not device_host_bitexact:
            fail(f"device/host 蒙皮对拍破缺: {dbg['debug_bitexact_abs']!r} tail={dbg['tail'][-200:]}")
            return 1
        note(f"  device/host 蒙皮对拍: max_abs=0（{len(dbg['debug_bitexact_abs'])} 核验帧）")

        # ── 主臂：skin_anim × runs 轮（生产口径测量循环）──
        run_stats: list[dict] = []
        run_digests: list[str] = []
        receipts: list[str] = []
        prod_means: list[float] = []
        verify_pass = 0
        verify_total = 0
        centroid_max = 0.0
        aabb_max = 0.0
        mv_median_max = 0.0
        mv_static_max = 0.0
        mv_host_motion_max = 0.0
        skin_gpu_mean = 0.0
        clean_all = True
        traj_doc: dict = {}
        for rep in range(runs):
            r = run_bench(True, frames, warmup, OUT_ROOT, None, require_real=True)
            clean_all &= r["clean_shutdown"]
            if r["rc"] != 0:
                fail(f"skin rep{rep + 1} 真跑 rc={r['rc']}: {r['tail'][-200:]}")
                return 1
            rec = r["receipt"]
            if not rec:
                fail(f"skin rep{rep + 1} receipt 缺失/不新鲜")
                return 1
            vfails = validate_skin_verify(r["verify"])
            if vfails:
                for m in vfails:
                    fail(f"skin rep{rep + 1} skin_verify: {m}")
                return 1
            v = r["verify"]
            if not traj_doc:
                # 动画规格取自真跑产出（禁手写锚——skin_verify 同源转引）。
                traj_doc = dict(v.get("animation") or {})
            verify_pass += sum(1 for fr in v["frames"] if fr.get("pass") is True)
            verify_total += len(v["frames"])
            centroid_max = max(
                centroid_max, max(float(fr["centroid_delta_px"]) for fr in v["frames"])
            )
            aabb_max = max(
                aabb_max, max(float(fr["aabb_delta_px"]) for fr in v["frames"])
            )
            mv_median_max = max(
                mv_median_max,
                max(max(float(x) for x in fr["mv_median_delta_px"]) for fr in v["frames"]),
            )
            mv_static_max = max(
                mv_static_max,
                max(float(fr["static_mv_median_abs_px"]) for fr in v["frames"]),
            )
            mv_host_motion_max = max(
                mv_host_motion_max,
                max(float(fr["mv_host_motion_px"]) for fr in v["frames"]),
            )
            run_stats.append(arm_stats([float(x) for x in rec["frame_ms"]]))
            run_digests.append(str(rec["last_frame_digest"]))
            prod_means.append(float(rec["stats_post_warmup"]["frame_ms_production_mean"]))
            if r["skin_gpu_ms_mean"] is not None:
                skin_gpu_mean = r["skin_gpu_ms_mean"]
            archive_dir = WORK / "receipts"
            archive_dir.mkdir(parents=True, exist_ok=True)
            archive = archive_dir / f"bench_skin_rep{rep + 1}.json"
            archive.write_text(
                json.dumps(rec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )
            receipts.append(str(archive))
            varch = archive_dir / f"skin_verify_rep{rep + 1}.json"
            varch.write_text(
                json.dumps(v, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )
        med = lambda key: sorted(x[key] for x in run_stats)[len(run_stats) // 2]
        skin_arm = {
            "arm": "skin_anim",
            "frame_ms_mean": med("mean"),
            "frame_ms_p50": med("p50"),
            "frame_ms_p99": med("p99"),
            "frame_ms_min": med("min"),
            "frame_ms_max": med("max"),
            "frame_ms_production_mean": sorted(prod_means)[len(prod_means) // 2],
            "last_frame_digest": run_digests[-1],
            "verify_frames_pass": verify_pass,
            "verify_frames_total": verify_total,
            "receipts": receipts,
        }
        note(
            f"  arm skin_anim: mean={med('mean'):.4f} p50={med('p50'):.4f} "
            f"p99={med('p99'):.4f} prod={sorted(prod_means)[len(prod_means) // 2]:.4f} "
            f"digest={run_digests[-1][:23]}… verify={verify_pass}/{verify_total}"
        )

        # ── 判据①：skin 臂双跑位级（runs ≥ 2 ⇒ digest 列恰一元）──
        digest_double = digests_bitexact([run_digests])
        if not digest_double:
            fail(f"skin 双跑 digest 位级一致破缺: {sorted(set(run_digests))}")

        # ── flip-trace 侧跑（逐帧 digest 序列 + 帧序;双趟）──
        trace_seqs: list[list[dict]] = []
        for t in range(2):
            tdir = WORK / f"trace_{t}"
            r = run_bench(True, TRACE_FRAMES, TRACE_WARMUP, WORK / "trace_out",
                          tdir, require_real=True)
            clean_all &= r["clean_shutdown"]
            if r["rc"] != 0:
                fail(f"trace {t} 侧跑 rc={r['rc']}: {r['tail'][-200:]}")
                return 1
            rows = load_trace(tdir)
            if len(rows) != TRACE_FRAMES + TRACE_WARMUP:
                fail(f"trace {t} 行数 {len(rows)} ≠ {TRACE_FRAMES + TRACE_WARMUP}")
                return 1
            trace_seqs.append(rows)
        order_ok = all(frame_order_ok(rows) for rows in trace_seqs)
        if not order_ok:
            fail("flip-trace 帧序破缺（非严格 0..N−1）")
        trace_digests = [[str(r["digest"]) for r in rows] for rows in trace_seqs]
        trace_bitexact = seqs_bitexact(trace_digests)
        if not trace_bitexact:
            fail("flip-trace 逐帧 digest 序列双趟位级一致破缺")
        note(f"  trace 侧跑: 帧序严格={order_ok} 逐帧 digest 双趟位级一致={trace_bitexact}")

        # ── 判据②③：静态回归锚臂（canonical 160 帧;digest == g14 Stage A 锚
        # 且 ≠ skin 臂 digest;skin_off measured 对照同窗）──
        sr = run_bench(False, STATIC_ANCHOR_FRAMES, STATIC_ANCHOR_WARMUP,
                       WORK / "static_out", None, require_real=True)
        clean_all &= sr["clean_shutdown"]
        if sr["rc"] != 0 or not sr["receipt"]:
            fail(f"静态回归锚臂真跑失败: {sr['tail'][-200:]}")
            return 1
        static_rec = sr["receipt"]
        static_digest = str(static_rec["last_frame_digest"])
        anchor_doc = json.loads(ANCHOR_PATH.read_text(encoding="utf-8"))
        anchor_digest = str(
            anchor_doc["anchors"][f"{SCENE}_t{TIER}_{BACKEND}"]["last_frame_digest"]
        )
        static_match = static_digest == anchor_digest
        if not static_match:
            fail(f"静态面 0-byte 回归破缺: {static_digest} ≠ 锚 {anchor_digest}")
        skin_neq_static = all(d != static_digest for d in run_digests)
        if not skin_neq_static:
            fail("动 vs 不动 digest 必异破缺（skin 臂 digest == 静态臂——坏内容面）")
        note(
            f"  静态回归锚: digest={static_digest[:23]}… == g14 锚={static_match}；"
            f"skin ≠ 静={skin_neq_static}"
        )

        if not clean_all:
            fail("clean shutdown 破缺（rc/PASS/receipt 新鲜/validation/leak 字样）")

    # ── measured 对照（skin on/off + 骨骼逐帧更新成本分解）──
    static_stats = arm_stats([float(x) for x in static_rec["frame_ms"]])
    static_prod = float(static_rec["stats_post_warmup"]["frame_ms_production_mean"])
    off_arm = {
        "arm": "static_off",
        "frame_ms_mean": static_stats["mean"],
        "frame_ms_p50": static_stats["p50"],
        "frame_ms_p99": static_stats["p99"],
        "frame_ms_min": static_stats["min"],
        "frame_ms_max": static_stats["max"],
        "frame_ms_production_mean": static_prod,
        "last_frame_digest": static_digest,
        "verify_frames_pass": 0,
        "verify_frames_total": 0,
        "receipts": [str(sr["receipt_path"])],
    }
    # scene/mv GPU 均值自末轮 receipt 列收割（两臂同列名,同语义段）。
    last_skin_rec = json.loads((WORK / "receipts" / f"bench_skin_rep{runs}.json").read_text(encoding="utf-8"))
    scene_gpu_mean = float(last_skin_rec["stats_post_warmup"]["scene_gpu_ns_mean"]) / 1e6
    mv_gpu_mean = float(last_skin_rec["stats_post_warmup"]["mv_ms_mean"])
    delta_ms = skin_arm["frame_ms_mean"] - off_arm["frame_ms_mean"]

    # ── evidence 落盘 ──
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    doc = {
        "schema": SCHEMA_ID,
        "subject": "g31_skinning_wiring",
        "symbolic_gate_key": GATE_KEY,
        "wave": "G31.B",
        "scene_id": SCENE,
        "tier": TIER,
        "backend": BACKEND,
        "seed": int(static_rec["seed"]),
        "frames_measured": frames,
        "warmup": warmup,
        "runs_per_arm": runs,
        "character": {
            "bone_count": int(traj_doc.get("bone_count", 3)),
            "tri_count": int(traj_doc.get("tri_count", 36)),
            "vertex_count": int(traj_doc.get("vertex_count", 108)),
            "emission": [float(x) for x in traj_doc.get("emission", [])],
            "albedo": [float(x) for x in traj_doc.get("albedo", [])],
            "detection": "命中信息通道 inst==1 地面真值（ray query 提交实例下标；非谱近似）",
            "animation": {
                "root_amp": [float(x) for x in traj_doc.get("root_amp", [])],
                "root_freq": [float(x) for x in traj_doc.get("root_freq", [])],
                "root_amp2": [float(x) for x in traj_doc.get("root_amp2", [])],
                "root_freq2": [float(x) for x in traj_doc.get("root_freq2", [])],
                "root_phase2": [float(x) for x in traj_doc.get("root_phase2", [])],
                "swing_amp": float(traj_doc.get("swing_amp", 0.0)),
                "swing_freq": float(traj_doc.get("swing_freq", 0.0)),
                "swing_amp2": float(traj_doc.get("swing_amp2", 0.0)),
                "swing_freq2": float(traj_doc.get("swing_freq2", 0.0)),
                "swing_phase2": float(traj_doc.get("swing_phase2", 0.0)),
                "elbow_amp": float(traj_doc.get("elbow_amp", 0.0)),
                "elbow_freq": float(traj_doc.get("elbow_freq", 0.0)),
                "elbow_amp2": float(traj_doc.get("elbow_amp2", 0.0)),
                "elbow_freq2": float(traj_doc.get("elbow_freq2", 0.0)),
                "elbow_phase2": float(traj_doc.get("elbow_phase2", 0.0)),
                "upper_len": float(traj_doc.get("upper_len", 0.0)),
                "origin": [float(x) for x in traj_doc.get("origin", [])],
            },
            "bone_upload_bytes_per_frame": 352,
        },
        "arms": [skin_arm, off_arm],
        "digest_bitexact_double_run": digest_double,
        "skin_neq_static_digest": skin_neq_static,
        "static_anchor_digest": static_digest,
        "static_anchor_match": static_match,
        "position_verify": {
            "frames_total": verify_total,
            "frames_pass": verify_pass,
            "centroid_max_px": centroid_max,
            "aabb_max_px": aabb_max,
            "mv_median_max_px": mv_median_max,
            "mv_static_max_px": mv_static_max,
            "mv_host_motion_max_px": mv_host_motion_max,
            "tolerance_centroid_px": SKIN_TOL_CENTROID_PX,
            "tolerance_aabb_px": SKIN_TOL_AABB_PX,
            "tolerance_mv_median_px": SKIN_MV_TOL_MEDIAN_PX,
            "tolerance_mv_static_px": SKIN_MV_STATIC_MAX_PX,
        },
        "mv_channel": {
            "class1_camera": "wired（g31_skin_mv = g14_mv 逐字镜像面；静态/天空像素相机 MV 原式不变）",
            "class2_rigid_instance": "gap_registered_A4（刚性实例 MV 缺口维持 A4 登记;本车道不含刚性动态实例,不冒充接通）",
            "class3_skinned": "wired（prev 蒙皮顶点 bary 插值 → prev_vp 投影;RD-041 类 3 逐顶点形变速度）",
            "tsr_history_consumed": True,
            "smearing_mitigation_note": (
                f"蒙皮角色像素 MV = 逐像素 prev 蒙皮位置重投影（进 TSR resolve in_mv 既有消费面,"
                f"TSR kernel 0-byte）——A4 登记的运动物体 TSR 历史拖影缺口在蒙皮类对象上结构性缓解;"
                f"核验证据 = 检测像素域 dev 中位数 vs host 逐顶点中位数逐分量差 ≤{SKIN_MV_TOL_MEDIAN_PX}px"
                f"（实测 max {mv_median_max:.3f}px）+ 窗级聚合真动门（max host 实测 {mv_host_motion_max:.3f}px "
                f"≥{SKIN_MV_HOST_MOTION_MIN_PX}px;高动帧条件 ratio 门 dev ≥{SKIN_MV_DEV_RATIO_MIN}×host）"
                f"+ 静态区无污染（实测 max {mv_static_max:.3f}px ≤{SKIN_MV_STATIC_MAX_PX}px）;"
                f"边界 = 帧间形变小的域（本 demo 核验窗 max host {mv_host_motion_max:.1f}px/帧实测绿）,大形变帧历史信任仍由 TSR 既有"
                f"置信/钳制面承载,未量化改写 TSR 行为——做到多少如实登记"
            ),
        },
        "device_host_skinning_bitexact": device_host_bitexact,
        "m92_gate": {
            "state": "PASS",
            "run_digest": m92_run_digest,
            "double_run_bitexact": bool(m92_doc.get("checks", {}).get("tier_switch_double_run_bitexact")),
            "vertex_bitexact": bool(m92_doc.get("checks", {}).get("vertex_bitexact")),
            "cone_bitexact": bool(m92_doc.get("checks", {}).get("cone_bitexact")),
            "static_frame_zero_as_build": bool(m92_doc.get("checks", {}).get("static_frame_zero_as_build")),
            "as_update_counted": bool(m92_doc.get("checks", {}).get("as_update_counted")),
            "evidence_path": str(m92_ev),
        },
        "measured": {
            "skin_on_frame_ms_mean": skin_arm["frame_ms_mean"],
            "skin_off_frame_ms_mean": off_arm["frame_ms_mean"],
            "delta_frame_ms": delta_ms,
            "skin_pass_gpu_ms_mean": skin_gpu_mean,
            "scene_gpu_ms_mean": scene_gpu_mean,
            "mv_gpu_ms_mean": mv_gpu_mean,
            "note": (
                f"同机同窗对照：skin_on mean={skin_arm['frame_ms_mean']:.4f}ms p50={skin_arm['frame_ms_p50']:.4f}ms "
                f"prod={skin_arm['frame_ms_production_mean']:.4f}ms（{frames}+{warmup}wu ×{runs} 轮跨轮中位数）vs "
                f"skin_off mean={off_arm['frame_ms_mean']:.4f}ms p50={off_arm['frame_ms_p50']:.4f}ms "
                f"prod={off_arm['frame_ms_production_mean']:.4f}ms（静态 canonical {STATIC_ANCHOR_FRAMES}+{STATIC_ANCHOR_WARMUP}wu 锚臂,"
                f"稳态均值与 100 帧窗可比,窗不对称如实登记）；delta={delta_ms:+.4f}ms = 蒙皮角色 + 骨骼逐帧更新全特征成本"
                f"（蒙皮 pass GPU {skin_gpu_mean:.6f}ms + BLAS refit GPU 段（帧墙钟内含,未单列——refit 桥录在 timestamp 区间外,"
                f"如实登记不拆分）+ palette/参数六小件 host 上传 352B/帧 + 角色内容渲染）;"
                f"scene GPU {scene_gpu_mean:.4f}ms / mv GPU {mv_gpu_mean:.4f}ms（含蒙皮 MV 覆盖臂）"
            ),
        },
        "clean_shutdown": clean_all,
        "environment": {
            "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
            "os": "windows",
            "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
            "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                          capture_output=True, text=True).stdout.strip(),
        },
        "timestamp": ts,
        "notes": (
            f"接入面取舍：render_exec 持久车道既有逐帧面 = buffer_uploads + TLAS 实例变换 update（A4,刚体）,"
            f"无 BLAS 顶点更新面;候选「scene kernel 解析求交免 BLAS」违「进 BLAS」字面且角色不进 TLAS 即不投"
            f"形变阴影,弃;选用**蒙皮后顶点缓冲 + BLAS refit（UPDATE）通路**——①单所有者纪律守恒（VkAsManager 独占 AS,"
            f"桥接 = 一条 vkCmdCopyBuffer,无跨所有者显存别名）②全链 GPU 内零 host 回读（生产口径成立）③静态面 0-byte"
            f"（ALLOW_UPDATE 仅角色 BLAS 打标,静态 BLAS flags=0——静态锚零漂移本件机核）④M92 验证件本体语义零触碰"
            f"（本车道消费 geometry::skinning host 参照为核验臂,蒙皮数学同式同序）。"
            f"位置核验 {verify_pass}/{verify_total} 帧（质心 ≤{SKIN_TOL_CENTROID_PX}px AABB ≤{SKIN_TOL_AABB_PX}px,"
            f"pred = host skin_vertex 蒙皮全顶点投影并集掩码 vs device 命中通道 inst==1 地面真值;obs 真动 ≥{OBS_MOTION_MIN_PX}px）;"
            f"M92 门维持接线态复跑 PASS（run_digest={m92_run_digest[:23]}…）;"
            f"device/host 蒙皮逐顶点对拍 max_abs=0（RURIX_SKIN_DEBUG_TRIS 诊断臂真跑收割）;"
            f"flip-trace 双趟逐帧 digest 位级一致 + 帧序严格;"
            f"WPO 登记：RD-041 蒙皮/WPO MV 通道接口 = 本件 prev 逐顶点位置表 + bary 插值面（WPO 资产面在 bistro 缺席,"
            f"顶点级速度通道形态本件冻结,WPO 内容触发时复用同通路）"
        ),
    }
    missing = evidence_required_keys(doc)
    if missing:
        fail(f"evidence 缺 required 键: {missing}")
        return 1
    ev_path = out_path or (ROOT / "evidence" / f"g31_skinning_wiring_{ts}.json")
    ev_path.parent.mkdir(parents=True, exist_ok=True)
    ev_path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {ev_path}")

    ok = (
        digest_double
        and trace_bitexact
        and order_ok
        and skin_neq_static
        and static_match
        and device_host_bitexact
        and clean_all
        and not FAILURES
    )
    note(f"GATE {'PASS' if ok else 'FAIL'} {GATE_KEY}")
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--gate", default="")
    ap.add_argument("--runs", type=int, default=2)
    ap.add_argument("--frames", type=int, default=100)
    ap.add_argument("--warmup", type=int, default=10)
    ap.add_argument("--out", default="")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        if args.frames < 100:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 100（任务书 ≥100 帧硬线）",
                  file=sys.stderr)
            return 1
        if args.runs < 2:
            print(f"[{TAG}] FAIL: --runs {args.runs} < 2（双跑位级判据硬线）",
                  file=sys.stderr)
            return 1
        return gate(args.runs, args.frames, args.warmup,
                    Path(args.out) if args.out else None)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
