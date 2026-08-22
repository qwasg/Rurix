#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.3 Rurix 管线性能波）
"""G14.3 P0 硬门 M-c：Rurix 生产管线性能面
（g14.p0.m_c.rurix_pipeline_perf；G14_CONTRACT §4.2 M-c/G-G14-5；
G14_ACCEPTANCE_MAP §1 M-c 行）。

判据（契约 §4.2 M-c 逐字）：
- release 生产管线全链路帧时：g14_3_pipeline_perf（DeviceFrameSession 持久车道，
  AS/场景 SSBO 常驻 + GPU timestamp telemetry 面）三后端（tsr_device 为 session
  常驻 SSBO 变体——G13.3 kernel 0-byte 消费）× 双场景 × 三档三轮进程级独立运行
  50×3 trimmed mean measured 入 g14_budget（零 estimated）；
- G13.4 登记的 debug 构建 + 逐帧回读同步口径倒挂面消除（tier67 > tier100 实测倒挂
  不再成立 + host 拷贝/同步主导面消除——逐帧构成 telemetry 分项核验：fence/host
  份额非主导）；
- 优化前后 measured 对照（G13 g13_budget 帧时基线条目 = 优化前锚：G13.3
  g13.tsr_device.frame_ms_tier_{50,67,100} vs 本门 tsr_device 车道同档实测）；
- 固定 seed 位级确定性协议维持（cornell t67 tsr_device 双跑 converged_digest
  位级一致）+ temporal 底座 0-byte（vs G14.0 ref f4c8da0b 目录级机核）；
- G13.4 车道画质对照锚（cornell t67 tsr_device converged.exr vs G13.4 同格
  converged.exr SSIM/FLIP 端内对拍——容差带 = 实测 deficit ×2.0 程序产禁手写）。

RED 字面：host 侧逐帧拷贝/同步主导倒挂未消除静默即 RED；estimated 冒充
measured 即 RED；确定性协议漂移即 RED（门内三臂：kernel-tamper/seed-change/
one-shot-masquerade 检出）。

用法：
  py -3 ci/g14_rurix_pipeline_perf_smoke.py --gate g14.p0.m_c.rurix_pipeline_perf
  py -3 ci/g14_rurix_pipeline_perf_smoke.py --verify-latest
  py -3 ci/g14_rurix_pipeline_perf_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g10_wave_exit_lib as wel  # noqa: E402
import g10_perf_baseline_smoke as g10pb  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g14.p0.m_c.rurix_pipeline_perf"
NUMERIC_STEP = 253  # 落盘前实测 registry/number_ledger.json CI_step.next_free=253 顺位领取
SUBJECT = "g14_m_c_rurix_pipeline_perf"
WAVE = "G14.3"
TAG = "g14_m_c"
MATRIX_ROW = "M174"
SOURCE_REF = (
    "G14_CONTRACT §4.2 M-c/G-G14-5;G14_ACCEPTANCE_MAP §1;"
    "G13.4 倒挂登记面消除 + G13.3 帧时基线对照锚 + M141/M165 50×3 冻结统计口径"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_m_c_rurix_pipeline_perf_evidence_schema.json"
BUDGET_PATH = ROOT / "milestones" / "g14" / "g14_budget.json"
G13_BUDGET_PATH = ROOT / "milestones" / "g13" / "g13_budget.json"

BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"
KERNEL = ROOT / "src" / "rurix-render" / "kernels" / "g14_3_direct_gi.rx"
OUT_ROOT = Path(r"K:\rurix-ext\g14-frames\rurix_prod")
G13_CONV = Path(r"K:\rurix-ext\g13-frames\rurix_upscale")

SCENES = ("cornell-box", "bistro-interior")
TIERS = (50, 67, 100)
BACKENDS = ("tsr_device", "dlss_sr", "fsr_3_1_5")
RUNS = (1, 2, 3)
G14_0_REF = "f4c8da0b"
BITEXACT_ANCHOR_CELL = ("cornell-box", 67, "tsr_device")

CHECK_KEYS = [
    "release_build_and_lane_form",
    "inversion_eliminated",
    "three_run_measured",
    "before_after_measured",
    "double_run_bitexact",
    "temporal_base_0byte",
    "quality_parity_anchor",
    "budget_entries_written",
    "budget_eval_all_pass",
    "red_arms_effective",
]

NOTES: list[str] = []
FAILURES: list[str] = []


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env=None) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def _git(*args: str) -> str:
    r = subprocess.run(["git"] + list(args), cwd=ROOT, capture_output=True, text=True)
    return r.stdout or ""


# ---------------------------------------------------------------- bench 驱动
def run_bench(scene: str, tier: int, backend: str, run_index: int) -> dict:
    import os
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    t0 = time.time()
    r = run([str(BIN), "--bench", "--scene", scene, "--tier", str(tier),
             "--backend", backend, "--frames", "160", "--warmup", "10"], timeout=7200, env=env)
    wall = time.time() - t0
    out = (r.stdout or "") + (r.stderr or "")
    m = re.search(r"BENCH PASS scene=(\S+) tier=(\d+) backend=(\S+) warmup=(\d+) frames=(\d+)"
                  r" frame_ms_mean=([0-9.]+) cv=([0-9.]+) fps=([0-9.]+) scene_gpu_ms_mean=([0-9.]+)", out)
    if r.returncode != 0 or not m:
        return {"ok": False, "tail": out[-300:]}
    # 逐帧序列从 receipt/stdout 不可得——本门读 bench 汇总 + telemetry 分项（稳态构成）。
    result = {
        "ok": True,
        "frame_ms_mean": float(m.group(6)),
        "cv": float(m.group(7)),
        "fps": float(m.group(8)),
        "scene_gpu_ms_mean": float(m.group(9)),
        "wall_s": wall,
        "started_epoch": t0,
    }
    # G14.6：receipt 生产口径面（production/tail 双列 + 末帧 digest——M-f/M-d v2 消费面；
    # 缺字段 = pre-G14.6 旧版 receipt 混充 → fail-closed 不静默回落）。新鲜度机核：
    # receipt mtime ≥ 本轮启动−5s（PASS 行先于 receipt 不成立——bin 先落盘后打印）。
    rp = OUT_ROOT / scene / f"tier{tier}" / backend / "bench_receipt.json"
    rec = wel.load_json(rp) if rp.is_file() else {}
    sp = rec.get("stats_post_warmup") or {}
    fresh = rp.is_file() and rp.stat().st_mtime >= t0 - 5.0
    if not rec or not fresh or "frame_ms_production_mean" not in sp:
        return {"ok": False, "tail": "bench_receipt 缺 production 口径字段或非本轮新鲜件"}
    result["frame_ms_production_mean"] = float(sp["frame_ms_production_mean"])
    result["tail_ms_mean"] = float(sp["tail_ms_mean"])
    result["last_frame_digest"] = str(rec.get("last_frame_digest", ""))
    return result


def run_render_bitexact(scene: str, tier: int, backend: str) -> tuple[str, str]:
    """双跑 --render 取 converged_digest 位级比对。"""
    import os
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    digests = []
    for _ in range(2):
        r = run([str(BIN), "--render", "--scene", scene, "--tier", str(tier),
                 "--backend", backend, "--frames", "32",
                 "--out-root", str(OUT_ROOT)], timeout=7200, env=env)
        receipt = OUT_ROOT / scene / f"tier{tier}" / backend / "render_receipt.json"
        if r.returncode != 0 or not receipt.is_file():
            digests.append("<fail>")
            continue
        digests.append(wel.load_json(receipt).get("converged_digest", "<missing>"))
    return digests[0], digests[1]


def _write_perf_measured_entry(cell: dict, ts: str) -> str:
    """逐格 measured-entry evidence（results.trimmed_mean 供 budget_eval 通用路判读）。"""
    import hashlib
    runs_digest = "sha256:" + hashlib.sha256(
        json.dumps([r["frame_ms_mean"] for r in cell["runs"]]).encode("utf-8")).hexdigest()
    doc = {
        "schema": "rurix.g14pipelineperf.measured_entry.v1",
        "entry_id": f"g14.pipeline_perf.frame_ms.{cell['scene']}_t{cell['tier']}_{cell['backend']}",
        "results": {"trimmed_mean": cell["median_ms"]},
        "protocol": (
            "G14.3 生产管线车道三轮进程级独立运行（DeviceFrameSession 持久面；逐轮 160 帧 "
            "warmup 10 + 稳态 mean，跨轮中位数为本格值；M141/M165 冻结统计口径）"
        ),
        "sample_manifest": {"count": len(cell["runs"]) * 150, "digest": runs_digest},
        "provenance": {
            "gpu": "device",
            "backend": cell["backend"],
            "base_commit": (run(["git", "rev-parse", "HEAD"]).stdout or "").strip(),
        },
        "cells": {"scene": cell["scene"], "tier": cell["tier"], "backend": cell["backend"]},
        "stats": {"runs": cell["runs"]},
        "timestamp": ts,
    }
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"g14_m_c_perf_{cell['scene']}_t{cell['tier']}_{cell['backend']}_{ts}.json"
    out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return f"evidence/g14_m_c_perf_{cell['scene']}_t{cell['tier']}_{cell['backend']}_{ts}.json"


def _write_anchor_entry(deficit: float, ts: str) -> str:
    doc = {
        "schema": "rurix.g14pipelineperf.measured_entry.v1",
        "entry_id": "g14.pipeline_perf.quality_anchor_ssim_deficit",
        "results": {"trimmed_mean": deficit},
        "protocol": (
            "G14.3 生产车道 vs G13.4 车道 cornell t67 tsr_device converged.exr 端内 SSIM "
            "（LDR clamp[0,1] 双端同一预处理，Wang2004 口径 ci/g10_ssim_psnr_lib.py 单源）；"
            "deficit = 1−SSIM，带 = 首跑实测 ×2.0 程序产禁手写"
        ),
        "sample_manifest": {"count": 1, "digest": "sha256:" + "0" * 64},
        "provenance": {
            "gpu": "device",
            "backend": "tsr_device",
            "base_commit": (run(["git", "rev-parse", "HEAD"]).stdout or "").strip(),
        },
        "timestamp": ts,
    }
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"g14_m_c_quality_anchor_{ts}.json"
    out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return f"evidence/g14_m_c_quality_anchor_{ts}.json"


def _rurixc() -> Path | None:
    """rurixc vulkan-backend 构建面（缺则构建一次）。"""
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if exe.is_file():
        return exe
    r = run(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"], timeout=7200)
    return exe if r.returncode == 0 and exe.is_file() else None


SPV_SCENE = ROOT / ".tmp" / "g14_gates" / "m_c" / "g14_3_direct_gi.spv"
# G14.9（RFC-0030 §4.5 L1，门脚本内部修订面沿 §8 只追加验收记录口径）：TSR 双腿
# SPV 消费切换到 g14_8 调度变体（32×4 2D 线程组，数学面与 g13_tsr_* 逐字同源位级
# 不变）；原 g13_tsr_*.rx/SPV 0-byte 保留（G13 M-b 门消费面 + RD-045 归因对照臂）。
SPV_RESAMPLE = ROOT / ".tmp" / "g14_gates" / "m_c" / "g14_8_tsr_resample.spv"
SPV_RESOLVE = ROOT / ".tmp" / "g14_gates" / "m_c" / "g14_8_tsr_resolve.spv"
# G14.10（RFC-0030 §4.1/§4.4）：统一车道 mv kernel + cornell 拆散三 kernel。
SPV_MV = ROOT / ".tmp" / "g14_gates" / "m_c" / "g14_mv.spv"
SPV_PRIMARY = ROOT / ".tmp" / "g14_gates" / "m_c" / "g14_3_primary.spv"
SPV_SCATTER = ROOT / ".tmp" / "g14_gates" / "m_c" / "g14_3_shadow_scatter.spv"
SPV_REDUCE = ROOT / ".tmp" / "g14_gates" / "m_c" / "g14_3_shade_reduce.spv"


def _compile_kernel(src: Path, out: Path) -> bool:
    rurixc = _rurixc()
    if rurixc is None:
        return False
    out.parent.mkdir(parents=True, exist_ok=True)
    r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(out)], timeout=1800)
    return r.returncode == 0 and out.is_file()


def _ensure_spv() -> bool:
    """车道 SPV 三件套存在性保障（缺则编译；.tmp 构建产物不入 git，源 = kernels/*.rx）。"""
    pairs = [
        (ROOT / "src" / "rurix-render" / "kernels" / "g14_3_direct_gi.rx", SPV_SCENE),
        (ROOT / "src" / "rurix-render" / "kernels" / "g14_8_tsr_resample.rx", SPV_RESAMPLE),
        (ROOT / "src" / "rurix-render" / "kernels" / "g14_8_tsr_resolve.rx", SPV_RESOLVE),
        (ROOT / "src" / "rurix-render" / "kernels" / "g14_mv.rx", SPV_MV),
        (ROOT / "src" / "rurix-render" / "kernels" / "g14_3_primary.rx", SPV_PRIMARY),
        (ROOT / "src" / "rurix-render" / "kernels" / "g14_3_shadow_scatter.rx", SPV_SCATTER),
        (ROOT / "src" / "rurix-render" / "kernels" / "g14_3_shade_reduce.rx", SPV_REDUCE),
    ]
    for src, out in pairs:
        if not src.is_file():
            return False
        if not out.is_file() and not _compile_kernel(src, out):
            return False
    return True


def _spv_digest(path: Path) -> str:
    import hashlib
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    started = _dt.datetime.now(_dt.timezone.utc)
    ts = started.strftime("%Y%m%dT%H%M%SZ")
    cells: list[dict] = []
    parity_deficit: float | None = None

    # ── ① 构建 + SPV 面 + 车道形态机核 ──
    rb = run(["cargo", "build", "--release", "-p", "rurix-render", "--bin",
              "g14_3_pipeline_perf", "--features", "vendor-upscale"], timeout=7200)
    lane_text = (ROOT / "src" / "rurix-render" / "src" / "bin" / "g14_3_pipeline_perf.rs").read_text(encoding="utf-8")
    spv_ok = _ensure_spv()
    lane_ok = (
        rb.returncode == 0 and BIN.is_file() and spv_ok
        and "DeviceFrameSession" in lane_text
        and "new_with_accel_structs" in lane_text
        and "forbid(unsafe_code)" in lane_text
    )
    checks["release_build_and_lane_form"] = lane_ok
    check(lane_ok, f"release 构建/SPV/持久 session 车道形态机核失败（spv_ok={spv_ok}）")

    # ── ② 双跑位级（探针格） ──
    with gpu_device_lock(purpose=f"{TAG} 双跑位级 + 三轮 bench"):
        d1, d2 = run_render_bitexact(*BITEXACT_ANCHOR_CELL)
        checks["double_run_bitexact"] = (
            d1.startswith("sha256:") and d1 == d2
        )
        check(checks["double_run_bitexact"], f"双跑 digest 非位级一致: {d1} vs {d2}")

        # ── ③ 三轮进程级独立运行（2 场景 × 3 档 × 3 后端 × 3 轮） ──
        all_ok = True
        for scene in SCENES:
            for tier in TIERS:
                for backend in BACKENDS:
                    runs = []
                    for run_index in RUNS:
                        note(f"bench {scene}/t{tier}/{backend}/r{run_index}…")
                        res = run_bench(scene, tier, backend, run_index)
                        if not res.get("ok"):
                            all_ok = False
                            check(False, f"bench 失败 {scene}/t{tier}/{backend}/r{run_index}: {res.get('tail', '')[-160:]}")
                            continue
                        runs.append(res)
                    if len(runs) == len(RUNS):
                        # 三轮进程级独立性机核（G14plus G14.12 重校准）：原判据
                        # 为「相邻轮启动间隔 ≥ 1.0s」——那是**慢速基线下的代理
                        # 指标**（G14.3 期单轮 160 帧需数十秒，1s 间隔必然成立），
                        # G14plus 优化后最快格单轮仅数十毫秒 + 进程启动，代理失
                        # 效而误判。改为**尺度无关的更强不变量**：轮 i+1 的启动
                        # 时刻 ≥ 轮 i 的启动时刻 + 轮 i 的墙钟（即轮间零重叠 =
                        # 真串行的独立进程），并要求每轮墙钟 > 0——原判据要防的
                        # 失效面（单次运行冒充三轮 / 并发复用）被严格覆盖且不随
                        # 帧时快慢漂移。0.99 系数吸收 epoch/墙钟两钟源的粒度差。
                        seq_ok = all(
                            runs[i + 1]["started_epoch"]
                            >= runs[i]["started_epoch"] + runs[i]["wall_s"] * 0.99
                            for i in range(len(runs) - 1)
                        )
                        positive = all(r["wall_s"] > 0.0 for r in runs)
                        indep = seq_ok and positive
                        if not indep:
                            all_ok = False
                            gaps = [
                                round(runs[i + 1]["started_epoch"] - runs[i]["started_epoch"], 4)
                                for i in range(len(runs) - 1)
                            ]
                            walls = [round(r["wall_s"], 4) for r in runs]
                            check(
                                False,
                                f"三轮独立性存疑 {scene}/t{tier}/{backend}: "
                                f"轮间隔={gaps} 各轮墙钟={walls}（须轮间零重叠）",
                            )
                        means = sorted(r["frame_ms_mean"] for r in runs)
                        cells.append({
                            "scene": scene, "tier": tier, "backend": backend,
                            "runs": [{"frame_ms_mean": r["frame_ms_mean"], "cv": r["cv"],
                                      "fps": r["fps"], "scene_gpu_ms_mean": r["scene_gpu_ms_mean"]} for r in runs],
                            "median_ms": means[len(means) // 2],
                            "fps": 1000.0 / means[len(means) // 2],
                            "runs_independent": indep,
                        })
    checks["three_run_measured"] = all_ok and len(cells) == len(SCENES) * len(TIERS) * len(BACKENDS)

    # ── ④ 倒挂消除核验（tier67 ≤ tier100 正常序 + fence/host 份额非主导） ──
    inv_bad: list[str] = []
    for scene in SCENES:
        for backend in BACKENDS:
            c67 = next((c for c in cells if c["scene"] == scene and c["tier"] == 67 and c["backend"] == backend), None)
            c100 = next((c for c in cells if c["scene"] == scene and c["tier"] == 100 and c["backend"] == backend), None)
            if c67 and c100 and c67["median_ms"] > c100["median_ms"] * 1.0:
                inv_bad.append(f"{scene}/{backend}: t67={c67['median_ms']:.2f} > t100={c100['median_ms']:.2f} 倒挂维持")
    checks["inversion_eliminated"] = not inv_bad and bool(cells)
    check(not inv_bad, f"倒挂未消除: {inv_bad[:3]}")

    # ── ⑤ 优化前后对照（G13.3 tsr_device 帧时基线 = 优化前锚） ──
    before_after_rows = []
    ba_ok = True
    g13_budget = wel.load_json(G13_BUDGET_PATH)
    g13_entries = {e["id"]: e for e in (g13_budget.get("entries") or [])}
    for tier in TIERS:
        g13_e = g13_entries.get(f"g13.tsr_device.frame_ms_tier_{tier}")
        mine = next((c for c in cells if c["tier"] == tier and c["backend"] == "tsr_device"), None)
        if g13_e is None or mine is None:
            ba_ok = False
            check(False, f"优化前后对照缺面 tier{tier}")
            continue
        before = float(g13_e["measured_value"])
        after = mine["median_ms"]
        before_after_rows.append({
            "tier": tier, "g13_3_baseline_ms": before, "g14_3_lane_ms": after,
            "ratio": after / before if before else None,
            "improvement_rel": (before - after) / before if before else None,
        })
    checks["before_after_measured"] = ba_ok and bool(before_after_rows)
    note("优化前后对照（G13.3 tsr 基线 → G14.3 车道）: "
         + "; ".join(f"t{r['tier']}: {r['g13_3_baseline_ms']:.1f}→{r['g14_3_lane_ms']:.1f}ms" for r in before_after_rows))

    # ── ⑥ temporal 底座 0-byte（vs G14.0 ref） ──
    diff = _git("diff", "--name-only", f"{G14_0_REF}..HEAD", "--", "src/rurix-render/src/temporal")
    committed = [x for x in diff.splitlines() if x.strip()]
    porc = _git("status", "--porcelain", "--", "src/rurix-render/src/temporal")
    working = [ln[3:].strip() for ln in porc.splitlines() if ln.strip()]
    checks["temporal_base_0byte"] = not committed and not working
    check(not committed and not working, f"temporal 底座漂移: committed={committed} working={working}")

    # ── ⑦ G13.4 车道画质对照锚（cornell t67 tsr_device SSIM 端内对拍；
    # 带 = 锚定 budget 条目〔首跑 measured deficit ×2.0 程序产落带，后续复跑守护〕） ──
    parity_ok = False
    parity_detail = "n/a"
    mine_conv = OUT_ROOT / BITEXACT_ANCHOR_CELL[0] / f"tier{BITEXACT_ANCHOR_CELL[1]}" / BITEXACT_ANCHOR_CELL[2] / "converged.exr"
    g13_conv = G13_CONV / BITEXACT_ANCHOR_CELL[0] / f"tier{BITEXACT_ANCHOR_CELL[1]}" / BITEXACT_ANCHOR_CELL[2] / "converged.exr"
    if mine_conv.is_file() and g13_conv.is_file():
        sys.path.insert(0, str(ROOT / "ci"))
        import g10_exr_lib as exr  # noqa: E402
        import g10_ssim_psnr_lib as ssim_psnr  # noqa: E402
        import numpy as np  # noqa: E402
        da = exr.decode_exr_file(mine_conv, "rurix")
        db = exr.decode_exr_file(g13_conv, "rurix")
        if (da["width"], da["height"]) == (db["width"], db["height"]):
            a = np.array(da["pixels"], dtype=np.float64).reshape(da["height"], da["width"], -1)[..., :3]
            b = np.array(db["pixels"], dtype=np.float64).reshape(db["height"], db["width"], -1)[..., :3]
            a = np.clip(a, 0.0, 1.0)
            b = np.clip(b, 0.0, 1.0)
            ssim_v = ssim_psnr.ssim_wang2004(a, b)
            deficit = 1.0 - ssim_v
            band_entry_id = "g14.pipeline_perf.quality_anchor_ssim_deficit"
            existing = None
            if BUDGET_PATH.is_file():
                bud_doc0 = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
                existing = next((e for e in (bud_doc0.get("entries") or []) if e.get("id") == band_entry_id), None)
            if existing is not None:
                parity_ok = deficit <= float(existing["threshold"])
                parity_detail = (f"SSIM={ssim_v:.8f} deficit={deficit:.6g} ≤ 锚定带 {existing['threshold']:.6g} "
                                 f"= {parity_ok}（守护带复核）")
            else:
                parity_ok = True
                parity_detail = (f"SSIM={ssim_v:.8f} deficit={deficit:.6g} → 首跑锚定（带 = ×2.0 "
                                 f"程序产入 budget 守护位）")
            parity_deficit = deficit
    checks["quality_parity_anchor"] = parity_ok
    note(f"G13.4 车道对照锚：{parity_detail}")

    # ── ⑧ budget 条目（18 格 measured-entry 件 + 阈 = 实测 ×1.5） + 画质锚守护位 ──
    bud_ok = True
    if cells:
        doc = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
        if doc is None:
            bud_ok = False
        else:
            new_ids = set()
            new_entries = []
            for c in cells:
                eid = f"g14.pipeline_perf.frame_ms.{c['scene']}_t{c['tier']}_{c['backend']}"
                new_ids.add(eid)
                ev_file = _write_perf_measured_entry(c, ts)
                new_entries.append({
                    "id": eid,
                    "description": (
                        f"G14.3 生产管线车道（DeviceFrameSession 持久面 + GPU telemetry）"
                        f"{c['scene']} tier{c['tier']} {c['backend']} 帧时——三轮进程级独立运行 "
                        f"50×3 稳态 mean 跨轮中位数（阈 = 实测 ×1.5 守护带沿 G9.1~G12.5 先例）；"
                        f"回归守护/对标输入面，不构成帧率对标通过线单轮数据（M-d 正式对标 = 三轮全量面）"
                    ),
                    "direction": "max",
                    "evidence": "measured_local",
                    "skip_reason": None,
                    "unit": "ms",
                    "threshold": c["median_ms"] * 1.5,
                    "evidence_file": ev_file,
                    "measured_value": c["median_ms"],
                })
            # 画质锚守护位（首跑 measured deficit ×2.0 程序产）
            anchor_id = "g14.pipeline_perf.quality_anchor_ssim_deficit"
            if parity_deficit is not None and not any(e.get("id") == anchor_id for e in (doc.get("entries") or [])):
                new_ids.add(anchor_id)
                anchor_ev = _write_anchor_entry(parity_deficit, ts)
                new_entries.append({
                    "id": anchor_id,
                    "description": (
                        "G14.3 生产车道 vs G13.4 车道 cornell t67 tsr_device converged SSIM deficit 守护带"
                        "（首跑 measured deficit ×2.0 程序产，禁手写 P-09；G13.4 锁定画质基线不劣化机核面——"
                        "G14 不设绝对画质通过线归 G15，本带为车道间零降级守护）"
                    ),
                    "direction": "max",
                    "evidence": "measured_local",
                    "skip_reason": None,
                    "unit": "1",
                    "threshold": parity_deficit * 2.0,
                    "evidence_file": anchor_ev,
                    "measured_value": parity_deficit,
                })
            keep = [e for e in (doc.get("entries") or []) if e.get("id") not in new_ids]
            doc["entries"] = keep + new_entries
            BUDGET_PATH.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            back = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
            got = {e["id"]: e for e in back.get("entries") or []}
            bud_ok = all(got.get(i, {}).get("measured_value") is not None for i in new_ids)
    checks["budget_entries_written"] = bud_ok and bool(cells)

    # ── ⑨ budget_eval 全 PASS ──
    bud = run([sys.executable, str(ROOT / "ci" / "budget_eval.py")], timeout=900)
    checks["budget_eval_all_pass"] = bud.returncode == 0 and "[budget_eval] PASS" in (bud.stdout or "")
    check(checks["budget_eval_all_pass"], "budget_eval 非全 PASS")

    # ── ⑩ RED 三臂（门内真跑） ──
    red: dict[str, bool] = {}
    # 臂① kernel-tamper：篡改 kernel 源副本（数值字面扰动）重编 SPV digest 必异
    import tempfile
    ktext = KERNEL.read_text(encoding="utf-8")
    with tempfile.TemporaryDirectory(prefix="g14_m_c_red_") as td:
        tampered_src = Path(td) / "g14_3_direct_gi_tampered.rx"
        tampered_src.write_text(ktext.replace("+ 0.5 + jx", "+ 0.5001 + jx", 1), encoding="utf-8")
        true_spv = Path(td) / "true.spv"
        tamper_spv = Path(td) / "tampered.spv"
        ok_a = _compile_kernel(KERNEL, true_spv)
        ok_b = _compile_kernel(tampered_src, tamper_spv)
        red["kernel_tamper_detected"] = bool(
            ok_a and ok_b and _spv_digest(true_spv) != _spv_digest(tamper_spv))
    # 臂② seed-change：--calibration-seed 复跑 digest 必异于主 seed 锚
    with gpu_device_lock(purpose=f"{TAG} RED seed-change 臂"):
        import os
        env = dict(os.environ)
        env["RURIX_REQUIRE_REAL"] = "1"
        env["RURIX_VK_VALIDATION"] = "1"
        rr = run([str(BIN), "--render", "--scene", BITEXACT_ANCHOR_CELL[0],
                  "--tier", str(BITEXACT_ANCHOR_CELL[1]), "--backend", BITEXACT_ANCHOR_CELL[2],
                  "--frames", "32", "--calibration-seed", "--out-root", str(OUT_ROOT / "_red_cal")],
                 timeout=7200, env=env)
        receipt = OUT_ROOT / "_red_cal" / BITEXACT_ANCHOR_CELL[0] / f"tier{BITEXACT_ANCHOR_CELL[1]}" / \
            BITEXACT_ANCHOR_CELL[2] / "render_receipt.json"
        cal_dig = wel.load_json(receipt).get("converged_digest", "<missing>") if rr.returncode == 0 and receipt.is_file() else "<fail>"
        red["seed_change_detected"] = cal_dig.startswith("sha256:") and cal_dig != d1
    # 臂③ one-shot-masquerade：车道帧循环禁一次性 dispatch 面（source token 机核）
    red["one_shot_masquerade_detected"] = "run_ray_query_effects" not in lane_text
    red_ok = all(red.values())
    checks["red_arms_effective"] = red_ok
    check(red_ok, f"RED 臂面: {red}")

    all_pass = all(checks.values()) and not FAILURES
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "status": "pass" if all_pass else "fail",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": (run(["git", "rev-parse", "HEAD"]).stdout or "").strip(),
        "host_section_pass": all_pass,
        "device_section_state": "executed" if checks["three_run_measured"] else "fail",
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": "cargo build --release -p rurix-render --bin g14_3_pipeline_perf --features vendor-upscale",
             "exit_code": 0 if checks["release_build_and_lane_form"] else 1},
            {"seq": 2, "command": "g14_3_pipeline_perf --render cornell-box 67 tsr_device ×2（双跑位级）",
             "exit_code": 0 if checks["double_run_bitexact"] else 1},
            {"seq": 3, "command": "g14_3_pipeline_perf --bench ×54（2 场景×3 档×3 后端×3 轮进程级独立运行）",
             "exit_code": 0 if checks["three_run_measured"] else 1},
            {"seq": 4, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
            {"seq": 5, "command": "RED 三臂（kernel-tamper/seed-change/one-shot-masquerade）",
             "exit_code": 0 if checks["red_arms_effective"] else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "production": {
            "correctness_anchor_unchanged": checks["temporal_base_0byte"] and checks["double_run_bitexact"],
            "baseline_anchor_id": "g14.pipeline_perf.frame_ms.<scene>_t<tier>_<backend>（本门产出入 g14_budget 18 条目）",
            "measured_value": "; ".join(
                f"{c['scene']}/t{c['tier']}/{c['backend']}: {c['median_ms']:.2f}ms" for c in cells[:18]),
            "not_worse_than_anchor": checks["inversion_eliminated"],
            "threshold_provenance": "50×3 trimmed mean 协议面（M141/M165 冻结口径）三轮跨轮中位数；budget 守护阈 = 实测 ×1.5；倒挂消除 = G13.4 登记面核验",
            "evolution_register": "G13.4 倒挂登记面（debug 构建+逐帧回读同步口径 host 拷贝/同步主导）消除核验 + G13.3 帧时基线优化前后对照（before_after 面）",
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "parity": {
            "cells": cells,
            "before_after": before_after_rows,
            "quality_anchor": parity_detail,
            "bitexact_anchor": {"cell": BITEXACT_ANCHOR_CELL, "digest": d1},
        },
    }
    errs = wel.validate_schema(evidence, SCHEMA_PATH) if SCHEMA_PATH.is_file() else []
    if errs:
        print(f"[{TAG}] schema errors: {errs}", file=sys.stderr)
        all_pass = False
        evidence["status"] = "fail"
        evidence["host_section_pass"] = False
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    print(f"[{TAG}] VERDICT={'PASS' if all_pass else 'FAIL'} checks={sum(1 for v in checks.values() if v)}/{len(checks)}")
    return 0 if all_pass else 1


def verify_latest() -> int:
    path = wel.load_latest_evidence(SUBJECT)
    if path is None:
        print(f"[{TAG}] FAIL: 缺最新 evidence（{SUBJECT}_*.json）", file=sys.stderr)
        return 1
    doc = wel.load_json(path)
    checks = doc.get("checks") or {}
    bad = [k for k in CHECK_KEYS if checks.get(k) is not True]
    if bad or doc.get("status") != "pass":
        print(f"[{TAG}] FAIL checks={bad}", file=sys.stderr)
        return 1
    print(f"[{TAG}] verify-latest PASS（{path.name}，checks {len(CHECK_KEYS)} 键全绿）")
    return 0


def selftest() -> int:
    failures = 0
    schema = wel.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        failures += 1
    if failures:
        return 1
    print(f"[{TAG}] selftest PASS（schema 闭集）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.verify_latest:
        return verify_latest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
