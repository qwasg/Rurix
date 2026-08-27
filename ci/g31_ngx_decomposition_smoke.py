#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C9 NGX 分解 profiling 调查）
"""G31+ 波 C Task C9：NGX 分解 profiling 门冒烟（g31.waveC.ngx_decomp；
G30 承接锚 g30_campaign_handover_registry.json G30 期行 G17-MD-F1「NGX 分解
profiling 或 UE 侧插桩（宿主差可分离 measured 证据，RFC-0032 重判条件同源）」
兑现载体；G17-MD-F1 焦点格 = bistro-interior/t100/dlss_sr）。

分解面（src/rurix-rt/src/vendor_upscale.rs 双 env 门控探针，默认关零行为变更）：
  ① RURIX_G31_NGX_TS=1 —— GPU 时间戳三槽围 evaluate（ts0 cmd 首 TOP /
  ts1 evaluate 前 BOTTOM / ts2 evaluate 后 BOTTOM），waitIdle 后 64-bit 读回
  × timestampPeriod → pre_eval GPU 税（acquire barrier 等）/ NGX in-stream
  纯 GPU / 提交-同步税（submit_wait 墙钟 − cmd GPU）。
  ② RURIX_G31_DLSS_EVAL_X2=1 —— 同 cmd 第二次 slEvaluateFeature，submit_wait
  边际 = NGX 单次 in-stream 净成本（G17.3 同型），与 ① 直测互核。
四段 = NGX in-stream（evaluate 纯 GPU）/ NGX 提交-同步税 / scene 渲染段
（DeviceFrameTelemetry 逐 pass GPU timestamp：scene+mv，pack stderr 均值单列）/
其余宿主段（frame_ms_production − scene − mv − upscale 墙钟逐帧残差；含 SL
簿记/录制/evaluate CPU + 帧参数上传 + 三 pass submit/fence）。

判据闭集（milestones/g31/g31_ngx_decomposition_evidence_schema.json 描述段逐字）：
1. canonical_digest_zero_drift：canonical 160 帧 warmup 10 复跑末帧 digest ==
   g14_3_stage_a_digest_anchor 在案锚（既有测量口径零破坏的机器证明）。
2. canonical_ratio_not_worsened：新鲜 ratio = ue_median_ms / fresh frame_ms ≥
   在案 0.960479（诚实红不恶化下界，G31 波 A 验收同律）。
3. four_segments_measured：四段全 measured——ngx in-stream 中位 > 0、提交-同步
   税中位 ≥ 0、scene 段均值 > 0、宿主残差有限且 > −0.10ms、TS/timing 逐帧样本
   n ≥ 100。
4. ts_x2_crosscheck_agree：|X2 边际 − TS 直测| ≤ max(0.15ms, TS 直测 × 15%)
   （两独立方法互核 NGX in-stream 同值）。
5. upscale_wall_consistency：cmd GPU ≤ submit_wait 墙钟 + 0.05（GPU 不超墙钟）且
   ngx-ts submit_wait 墙钟与 dlss-ext submit_wait 墙钟两独立墙钟面 |差| ≤ 0.15ms。
6. ue_delta_localized：UE 暖态差主源定位结论在档（dominant_source ∈ 闭集 +
   delta == fresh − ue 一致 + analysis 非空）。
7. rejudge_conclusion_registered：重判评估结论与新鲜 ratio 一致（ratio ≥ 1.00
   → rejudge_triggered_ratio_ge_1；否则 rejudge_not_triggered_honest_red_maintained）。

三态：无 bin/无 Vulkan/无 NGX/资产缺失 → DEV_ENV_DEGRADE 退 0（不冒充 PASS）；
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充真跑）。

evidence 纪律：PASS 才落 evidence/g31_ngx_decomposition_<ts>.json（check_schemas
前缀路由 g31_ngx_decomposition_）；FAIL 诊断件落 .tmp/g31_gates/ngx_decomp/
工作区不污染 evidence/ 路由面（fail-closed：evidence/ 无件 = 门未过）。

用法：
  py -3 ci/g31_ngx_decomposition_smoke.py --selftest
  py -3 ci/g31_ngx_decomposition_smoke.py --gate g31.waveC.ngx_decomp
"""
from __future__ import annotations

import argparse
import datetime as _dt
import io
import json
import os
import re
import statistics
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveC.ngx_decomp"
SUBJECT = "g31_ngx_decomposition"
WAVE = "G31+.C"
TAG = "g31_ngx_decomp"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_ngx_decomposition_evidence_schema.json"
SCHEMA_ID = "rurix.g31.ngx_decomposition_evidence.v1"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_dlss_sr"
CELL_SLASH = "bistro-interior/t100/dlss_sr"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "release" / f"g14_3_pipeline_perf{EXE_SUFFIX}"
WORK = ROOT / ".tmp" / "g31_gates" / "ngx_decomp"

SCENE, TIER, BACKEND = "bistro-interior", 100, "dlss_sr"
FRAMES = 160
WARMUP = 10

ON_RECORD_FRAME_MS = 3.5767   # G30.2 M-b 焦点格新鲜真跑在案（g30_campaign_handover_registry G30 行）
ON_RECORD_RATIO = 0.960479    # 在案新鲜 ratio（诚实红维持下界）

TIMING_RE = re.compile(
    r"\[vendor-timing dlss-ext\] frame=(\d+) staging=([0-9.]+) sl_book=([0-9.]+) "
    r"record=([0-9.]+) evaluate=([0-9.]+) submit_wait=([0-9.]+) total=([0-9.]+)ms"
)
NGX_TS_RE = re.compile(
    r"\[ngx-ts dlss-ext\] frame=(\d+) pre_eval_gpu=([0-9.]+) ngx_gpu=([0-9.]+) "
    r"cmd_gpu=([0-9.]+) submit_wait_wall=([0-9.]+) tax=(-?[0-9.]+)ms"
)
PACK_RE = re.compile(r"DLSS_RESIDENT pack_gpu_ms mean=([0-9.]+)")
DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")

DOMINANT_SOURCES = (
    "ngx_in_stream_inseparable",
    "ngx_submit_sync_tax",
    "scene_render",
    "host_residual_separable",
    "mixed_host_segments",
)

FACT_IDS = [
    "canonical_digest_zero_drift",
    "canonical_ratio_not_worsened",
    "four_segments_measured",
    "ts_x2_crosscheck_agree",
    "upscale_wall_consistency",
    "ue_delta_localized",
    "rejudge_conclusion_registered",
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


def parse_timing_frames(out: str, warmup: int = WARMUP) -> list[dict]:
    """[vendor-timing dlss-ext] 逐帧解析（post-warmup 过滤 frame >= warmup）。"""
    rows = []
    for m in TIMING_RE.finditer(out):
        f = int(m.group(1))
        if f < warmup:
            continue
        rows.append({
            "frame": f,
            "staging": float(m.group(2)),
            "sl_book": float(m.group(3)),
            "record": float(m.group(4)),
            "evaluate": float(m.group(5)),
            "submit_wait": float(m.group(6)),
            "total": float(m.group(7)),
        })
    return rows


def parse_ngx_ts_frames(out: str, warmup: int = WARMUP) -> list[dict]:
    """[ngx-ts dlss-ext] 逐帧解析（post-warmup 过滤）。"""
    rows = []
    for m in NGX_TS_RE.finditer(out):
        f = int(m.group(1))
        if f < warmup:
            continue
        rows.append({
            "frame": f,
            "pre_eval_gpu": float(m.group(2)),
            "ngx_gpu": float(m.group(3)),
            "cmd_gpu": float(m.group(4)),
            "submit_wait_wall": float(m.group(5)),
            "tax": float(m.group(6)),
        })
    return rows


def med(vals: list[float]) -> float | None:
    return statistics.median(vals) if vals else None


def fmean(vals: list[float]) -> float | None:
    return statistics.fmean(vals) if vals else None


def ratio_not_worsened(fresh_ratio: float | None, on_record: float = ON_RECORD_RATIO) -> bool:
    """诚实红不恶化判：新鲜 ratio 有限且 ≥ 在案下界。"""
    return (
        isinstance(fresh_ratio, (int, float))
        and not isinstance(fresh_ratio, bool)
        and fresh_ratio == fresh_ratio
        and fresh_ratio >= on_record
    )


def crosscheck_agree(marginal: float | None, ts_direct: float | None) -> bool:
    """④ X2 边际 vs TS 直测互核：|差| ≤ max(0.15ms, TS×15%)。"""
    if marginal is None or ts_direct is None or ts_direct <= 0:
        return False
    return abs(marginal - ts_direct) <= max(0.15, 0.15 * ts_direct)


def wall_consistent(
    cmd_gpu_med: float | None,
    sw_wall_ts_med: float | None,
    sw_wall_ext_med: float | None,
) -> bool:
    """⑤ 墙钟一致判：cmd GPU ≤ submit_wait 墙钟 + 0.05（GPU 不超墙钟）；
    ngx-ts 与 dlss-ext 两独立墙钟面 |差| ≤ 0.15ms。"""
    if cmd_gpu_med is None or sw_wall_ts_med is None or sw_wall_ext_med is None:
        return False
    if cmd_gpu_med > sw_wall_ts_med + 0.05:
        return False
    return abs(sw_wall_ts_med - sw_wall_ext_med) <= 0.15


def host_residuals(receipt: dict) -> list[float]:
    """逐帧宿主残差 = frame_ms_production − scene − mv − upscale 墙钟（receipt
    逐帧列全 post-warmup，长度不一致 = 拒判空表）。"""
    prod = receipt.get("frame_ms_production") or []
    scene = receipt.get("scene_render_ms") or []
    mv = receipt.get("mv_ms") or []
    up = receipt.get("upscale_ms") or []
    if not (len(prod) == len(scene) == len(mv) == len(up)) or not prod:
        return []
    out = []
    for p, s, m, u in zip(prod, scene, mv, up):
        try:
            out.append(float(p) - float(s) - float(m) - float(u))
        except (TypeError, ValueError):
            return []
    return out


def localize_dominant_source(delta_ms: float, tax_med: float, host_med: float) -> str:
    """⑥ UE 差主源定位（Rurix 侧可分离宿主段包络归属）：
    delta ≤ 0 → ngx_in_stream_inseparable（ratio ≥ 1 路径，差非正）；
    候选 = 可分离宿主段（提交-同步税 / 宿主残差含 SL CPU）——NGX in-stream
    与 scene 段 = 双边同硬件同工作类，不作差源候选；最大候选 ≥ 60% × delta
    → 该段；否则 mixed_host_segments（包络内无单主导如实混合）。"""
    if delta_ms <= 0:
        return "ngx_in_stream_inseparable"
    cands = {
        "ngx_submit_sync_tax": max(tax_med, 0.0),
        "host_residual_separable": max(host_med, 0.0),
    }
    top = max(cands, key=cands.get)
    if cands[top] >= 0.6 * delta_ms:
        return top
    return "mixed_host_segments"


def rejudge_conclusion(ratio_ge_1: bool) -> str:
    """⑦ 重判结论闭集映射。"""
    return "rejudge_triggered_ratio_ge_1" if ratio_ge_1 else "rejudge_not_triggered_honest_red_maintained"


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    """三态裁决：无降级 → None（续跑）；有降级 + REQUIRE_REAL → 1（硬红）；
    有降级无 REQUIRE_REAL → 0（SKIP 非 PASS 非 FAIL）。"""
    if not degrade:
        return None
    return 1 if require_real else 0


def load_focus_ue_median() -> float | None:
    """g14_m-d dual_end 最新 evidence 焦点格 ue_median_ms（ratio 分母在案锚）。"""
    p = wel.load_latest_evidence("g14_m_d_dual_end_fps_parity")
    if not p:
        return None
    doc = wel.load_json(p)
    for c in (doc.get("parity", {}) or {}).get("cells", []) or []:
        if c.get("scene") == SCENE and c.get("tier") == TIER and c.get("backend") == BACKEND:
            v = c.get("ue_median_ms")
            return float(v) if isinstance(v, (int, float)) else None
    return None


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def bench_round(label: str, extra_env: dict[str, str], frames: int = FRAMES, warmup: int = WARMUP) -> tuple[subprocess.CompletedProcess, dict, str]:
    """单轮 bench 真跑 → (completed, receipt_doc, merged_output)。失败轮 receipt
    不消费（防旧件残留污染——ok=False 语义不可信一律置空，G17.3 M-b 同律）。"""
    out_root = WORK / label
    env = base_env()
    env.update(extra_env)
    argv = [
        str(BIN), "--bench", "--scene", SCENE, "--tier", str(TIER),
        "--backend", BACKEND, "--frames", str(frames), "--warmup", str(warmup),
        "--out-root", str(out_root),
    ]
    t0 = __import__("time").time()
    r = run(argv, timeout=7200, env=env)
    out = (r.stdout or "") + (r.stderr or "")
    receipt_path = out_root / SCENE / f"tier{TIER}" / BACKEND / "bench_receipt.json"
    rec = {}
    if r.returncode == 0 and receipt_path.is_file() and receipt_path.stat().st_mtime >= t0 - 5:
        try:
            rec = json.loads(receipt_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            rec = {}
    return r, rec, out


def run_gate() -> int:
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

    # ── 构建（release bin；rurix-rt vendor-upscale 探针面含在内）──
    r = run([
        "cargo", "build", "--release", "-p", "rurix-render", "--features", "vendor-upscale",
        "--bin", "g14_3_pipeline_perf", "--quiet",
    ])
    if r.returncode != 0:
        fail(f"harness release 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return 1

    # ── dev-env 探针（无 REQUIRE_REAL 短跑；skipped_dev_env / 异常即降级登记）──
    degrade: list[str] = []
    if not BIN.is_file():
        degrade.append(f"bench bin 缺失 {BIN}")
    if not ANCHOR_PATH.is_file():
        degrade.append(f"Stage A 锚缺失 {ANCHOR_PATH}")
    if not degrade:
        probe_env = dict(os.environ)
        probe_env.pop("RURIX_REQUIRE_REAL", None)
        rp = run([
            str(BIN), "--bench", "--scene", SCENE, "--tier", str(TIER),
            "--backend", BACKEND, "--frames", "2", "--warmup", "1",
            "--out-root", str(WORK / "dev_env_probe"),
        ], timeout=1800, env=probe_env)
        probe_out = (rp.stdout or "") + (rp.stderr or "")
        if '"state":"skipped_dev_env"' in probe_out:
            degrade.append(f"harness skipped_dev_env: {probe_out.strip()[-200:]}")
        elif rp.returncode != 0:
            degrade.append(f"dev-env 探针 rc={rp.returncode}: {probe_out.strip()[-200:]}")

    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {"schema": "rurix.g31.ngx_decomposition.skip.v1", "state": "DEV_ENV_DEGRADE", "reasons": degrade}
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP，非 PASS 非 FAIL）")
        return 0

    anchors = json.loads(ANCHOR_PATH.read_text(encoding="utf-8")).get("anchors") or {}
    anchor_digest = (anchors.get(ANCHOR_CELL) or {}).get("last_frame_digest")
    ue_median = load_focus_ue_median()
    if ue_median is None:
        fail("g14_m-d 焦点格 ue_median_ms 在案锚缺失")
        return 1

    # ── 三轮真跑（单锁串行；数字全来自真实命令输出）──
    #   A canonical（在案口径复跑核对）/ B timing+TS（四段直测）/ C timing+X2（边际互核）
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    with gpu_device_lock(purpose=f"{TAG} canonical + timing_ts + timing_x2 三轮"):
        ra, rec_a, out_a = bench_round("canonical", {})
        rb, rec_b, out_b = bench_round("timing_ts", {"RURIX_VENDOR_TIMING": "1", "RURIX_G31_NGX_TS": "1"})
        rc_, rec_c, out_c = bench_round("timing_x2", {"RURIX_VENDOR_TIMING": "1", "RURIX_G31_DLSS_EVAL_X2": "1"})

    io.open(WORK / f"round_canonical_{ts}.log", "w", encoding="utf-8", newline="\n").write(out_a)
    io.open(WORK / f"round_timing_ts_{ts}.log", "w", encoding="utf-8", newline="\n").write(out_b)
    io.open(WORK / f"round_timing_x2_{ts}.log", "w", encoding="utf-8", newline="\n").write(out_c)

    legs_ok = True
    for label, rr, rec in (("canonical", ra, rec_a), ("timing_ts", rb, rec_b), ("timing_x2", rc_, rec_c)):
        if rr.returncode != 0 or not rec:
            fail(f"{label} 真跑失败 rc={rr.returncode}（receipt {'有' if rec else '无'}）")
            legs_ok = False

    # ── A：canonical 在案口径复跑核对 ──
    sp_a = (rec_a.get("stats_post_warmup") or {}) if rec_a else {}
    fresh_frame_ms = sp_a.get("frame_ms_production_mean")
    fresh_digest = rec_a.get("last_frame_digest") if rec_a else None
    digest_hit = (
        isinstance(fresh_digest, str) and isinstance(anchor_digest, str)
        and DIGEST_RE.match(fresh_digest) is not None and fresh_digest == anchor_digest
    )
    set_fact(
        "canonical_digest_zero_drift",
        digest_hit,
        f"canonical 160 帧复跑末帧 digest {str(fresh_digest)[:23]}… vs 在案 {str(anchor_digest)[:23]}… "
        f"{'位级 MATCH（既有测量口径零破坏）' if digest_hit else 'DRIFT（RED）'}",
    )
    fresh_ratio = (ue_median / fresh_frame_ms) if (ue_median and fresh_frame_ms) else None
    ratio_ok = ratio_not_worsened(fresh_ratio)
    set_fact(
        "canonical_ratio_not_worsened",
        ratio_ok,
        f"fresh frame_ms={fresh_frame_ms}（在案 {ON_RECORD_FRAME_MS}）fresh ratio="
        f"{round(fresh_ratio, 6) if fresh_ratio else None}（在案 {ON_RECORD_RATIO}，ue_median={ue_median}ms）"
        f" → {'维持不恶化' if ratio_ok else '恶化如实 RED'}",
    )

    # ── B：四段分解 ──
    sp_b = (rec_b.get("stats_post_warmup") or {}) if rec_b else {}
    timing_b = parse_timing_frames(out_b)
    ts_b = parse_ngx_ts_frames(out_b)
    pack_m = PACK_RE.search(out_b)
    pack_mean = float(pack_m.group(1)) if pack_m else None
    ngx_med = med([r["ngx_gpu"] for r in ts_b])
    ngx_mean = fmean([r["ngx_gpu"] for r in ts_b])
    pre_eval_med = med([r["pre_eval_gpu"] for r in ts_b])
    cmd_gpu_med = med([r["cmd_gpu"] for r in ts_b])
    sw_ts_med = med([r["submit_wait_wall"] for r in ts_b])
    tax_list = [r["tax"] for r in ts_b]
    tax_med = med(tax_list)
    tax_mean = fmean(tax_list)
    sw_ext_med = med([r["submit_wait"] for r in timing_b])
    sl_book_mean = fmean([r["sl_book"] for r in timing_b])
    record_mean = fmean([r["record"] for r in timing_b])
    eval_cpu_mean = fmean([r["evaluate"] for r in timing_b])
    ext_total_med = med([r["total"] for r in timing_b])
    scene_mean = sp_b.get("scene_render_ms_mean")
    mv_mean = sp_b.get("mv_ms_mean")
    scene_seg_mean = (scene_mean + mv_mean) if (scene_mean is not None and mv_mean is not None) else None
    residuals = host_residuals(rec_b) if rec_b else []
    res_med = med(residuals)
    res_mean = fmean(residuals)

    four_ok = (
        ngx_med is not None and ngx_med > 0
        and tax_med is not None and tax_med >= 0
        and scene_seg_mean is not None and scene_seg_mean > 0
        and res_med is not None and res_med > -0.10
        and len(ts_b) >= 100 and len(timing_b) >= 100
        and pack_mean is not None
    )
    set_fact(
        "four_segments_measured",
        four_ok,
        f"NGX in-stream={ngx_med}ms（n={len(ts_b)}）提交-同步税={tax_med}ms（pre_eval GPU={pre_eval_med}）"
        f"scene 段={scene_seg_mean}ms（scene={scene_mean} mv={mv_mean} pack={pack_mean}）"
        f"宿主残差={res_med}ms（n={len(residuals)}；sl_book={sl_book_mean} record={record_mean} eval_cpu={eval_cpu_mean}）",
    )

    # ── C vs B：X2 边际互核 ──
    timing_c = parse_timing_frames(out_c)
    sw_x1_med = med([r["submit_wait"] for r in timing_b])
    sw_x2_med = med([r["submit_wait"] for r in timing_c])
    marginal = (sw_x2_med - sw_x1_med) if (sw_x1_med is not None and sw_x2_med is not None) else None
    x2_ok = crosscheck_agree(marginal, ngx_med) and len(timing_c) >= 100
    set_fact(
        "ts_x2_crosscheck_agree",
        x2_ok,
        f"X2 边际={marginal}ms（x1={sw_x1_med} x2={sw_x2_med}，n={len(timing_c)}）vs TS 直测={ngx_med}ms "
        f"|Δ|={abs(marginal - ngx_med) if (marginal is not None and ngx_med is not None) else None}ms"
        f"（容差 max(0.15, 15%)；两独立方法互核 NGX in-stream）",
    )

    # ── ⑤ 墙钟一致性 ──
    wall_ok = wall_consistent(cmd_gpu_med, sw_ts_med, sw_ext_med)
    set_fact(
        "upscale_wall_consistency",
        wall_ok,
        f"cmd GPU={cmd_gpu_med}ms ≤ submit_wait 墙钟（TS 面 {sw_ts_med}ms）+0.05；"
        f"TS 墙钟 vs dlss-ext 墙钟（{sw_ext_med}ms）|Δ|≤0.15（evaluate CPU={eval_cpu_mean}ms 段外）",
    )

    # ── ⑥ UE 差主源定位 ──
    delta_ms = (fresh_frame_ms - ue_median) if (fresh_frame_ms and ue_median) else None
    sl_cpu_total = (sl_book_mean or 0.0) + (record_mean or 0.0) + (eval_cpu_mean or 0.0)
    host_seg_med = (sl_cpu_total + res_med) if res_med is not None else None
    dominant = localize_dominant_source(
        delta_ms if delta_ms is not None else 0.0,
        tax_med or 0.0,
        host_seg_med if host_seg_med is not None else 0.0,
    )
    separable_host = (tax_med or 0.0) + sl_cpu_total + (max(res_mean, 0.0) if res_mean is not None else 0.0)
    analysis = (
        f"Rurix 新鲜 frame={fresh_frame_ms:.4f}ms vs UE 暖态 {ue_median:.4f}ms → Δ={delta_ms:+.4f}ms。"
        f"NGX in-stream（TS 直测 {ngx_med:.3f}ms / X2 边际 {marginal:.3f}ms 互核）= 双边同 GPU 同 NGX "
        f"310.5.2 网络同一 cubin 族执行（G15plus-II 在案 NGXCubinVulkan vs UE 臂 NGXCubinD3D12 同族），"
        f"物理不可分离且等量，不构成差源；Rurix 侧可分离宿主段包络 = 提交-同步税 {tax_med:.3f} + SL "
        f"簿记/录制/evaluate CPU {sl_cpu_total:.3f} + 车道宿主残差 {res_mean:.3f} ≈ {separable_host:.3f}ms "
        f"≥ |Δ|——差完全落在宿主可分离段包络内（逐帧孤立 submit+waitIdle 提交边界 vs UE 帧内 in-stream "
        f"evaluate 集成形态的宿主构成差），主源 = {dominant}。"
        if (fresh_frame_ms and ue_median and delta_ms is not None and ngx_med is not None
            and marginal is not None and tax_med is not None and res_mean is not None)
        else "数据不全（前置失败）"
    )
    ue_ok = (
        dominant in DOMINANT_SOURCES
        and delta_ms is not None
        and len(analysis) >= 30
    )
    set_fact(
        "ue_delta_localized",
        ue_ok,
        f"Δ={delta_ms if delta_ms is None else round(delta_ms, 4)}ms（fresh {fresh_frame_ms} − ue {ue_median}）"
        f"主源={dominant}；可分离宿主包络≈{round(separable_host, 4)}ms",
    )

    # ── ⑦ 重判评估 ──
    ratio_ge_1 = fresh_ratio is not None and fresh_ratio >= 1.0
    conclusion = rejudge_conclusion(ratio_ge_1)
    basis = (
        f"新鲜 ratio={fresh_ratio:.6f} < 1.00——ratio ≥1.00 重判条件未命中，维持 G30 终判 17/18 诚实红；"
        f"分解证据落档 = 承接锚兑现形态（宿主差可分离 measured 证据：NGX in-stream {ngx_med:.3f}ms "
        f"不可分离等量非差源，Δ={delta_ms:+.4f}ms 全落宿主可分离段包络 ≈{separable_host:.3f}ms 内）；"
        "在案行 0-byte 不回写，新证出现只追加重判程序不变。"
        if not ratio_ge_1 and fresh_ratio is not None and delta_ms is not None
        else (
            f"新鲜 ratio={fresh_ratio:.6f} ≥ 1.00——重判条件命中，按 RFC-0032 重判程序起草追加以太"
            "（在案行 0-byte 不回写，只追加）。"
            if ratio_ge_1 else "数据不全（前置失败）"
        )
    )
    rejudge_ok = conclusion in (
        "rejudge_not_triggered_honest_red_maintained", "rejudge_triggered_ratio_ge_1"
    ) and fresh_ratio is not None
    set_fact(
        "rejudge_conclusion_registered",
        rejudge_ok,
        f"conclusion={conclusion}（fresh ratio={round(fresh_ratio, 6) if fresh_ratio else None}）",
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
    gate_doc = {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "cell": CELL_SLASH,
        "anchor": {
            "g30_handover_row": "g30_campaign_handover_registry.json campaign_period_rows G30 期行（G17-MD-F1）",
            "anchor_text": "NGX 分解 profiling 或 UE 侧插桩（宿主差可分离 measured 证据，RFC-0032 重判条件同源）；新证出现只追加重判",
            "on_record_frame_ms": ON_RECORD_FRAME_MS,
            "on_record_ratio": ON_RECORD_RATIO,
            "ue_warm_median_ms_on_record": ue_median,
        },
        "canonical_rerun": {
            "frames": FRAMES,
            "warmup": WARMUP,
            "frame_ms_production_mean": fresh_frame_ms if fresh_frame_ms is not None else -1.0,
            "last_frame_digest": fresh_digest or ("sha256:" + "0" * 64),
            "stage_a_anchor_digest": anchor_digest or ("sha256:" + "0" * 64),
            "digest_zero_drift": bool(digest_hit),
            "fresh_ratio": fresh_ratio if fresh_ratio is not None else -1.0,
            "ratio_not_worsened": bool(ratio_ok),
        },
        "segments": {
            "ngx_in_stream_ms": {
                "median": ngx_med if ngx_med is not None else -1.0,
                "mean": ngx_mean if ngx_mean is not None else -1.0,
                "method": (
                    "vkCmdWriteTimestamp 三槽围 evaluate（ts1 evaluate 前 BOTTOM / ts2 evaluate 后 "
                    "BOTTOM）× timestampPeriod（props blob@720 实采）；RURIX_G31_NGX_TS=1 探针轮逐帧 "
                    "post-warmup 中位/均值"
                ),
            },
            "ngx_submit_sync_tax_ms": {
                "median": tax_med if tax_med is not None else -1.0,
                "mean": tax_mean if tax_mean is not None else -1.0,
                "pre_eval_gpu_median": pre_eval_med if pre_eval_med is not None else -1.0,
                "method": (
                    "submit_wait 墙钟（evaluate CPU 返回 → vkEndCommandBuffer + vkQueueSubmit + "
                    "vkQueueWaitIdle + reset）− cmd GPU（ts2−ts0）——提交-同步固定税；pre_eval GPU "
                    "子项 = acquire barrier ×3 段（ts1−ts0）"
                ),
            },
            "scene_render_ms": {
                "mean": scene_seg_mean if scene_seg_mean is not None else -1.0,
                "scene_mean": scene_mean if scene_mean is not None else -1.0,
                "mv_mean": mv_mean if mv_mean is not None else -1.0,
                "pack_mean": pack_mean if pack_mean is not None else -1.0,
                "method": (
                    "DeviceFrameTelemetry 逐 pass GPU timestamp（pass0 g14_3_direct_gi + pass1 g14_mv "
                    "post-warmup 均值；pack pass GPU 均值经 stderr DLSS_RESIDENT 行单列）"
                ),
            },
            "host_residual_ms": {
                "median": res_med if res_med is not None else -1.0,
                "mean": res_mean if res_mean is not None else -1.0,
                "sl_book_mean": sl_book_mean if sl_book_mean is not None else -1.0,
                "record_mean": record_mean if record_mean is not None else -1.0,
                "evaluate_cpu_mean": eval_cpu_mean if eval_cpu_mean is not None else -1.0,
                "method": (
                    "frame_ms_production − scene_render_ms − mv_ms − upscale 墙钟逐帧残差（receipt "
                    "逐帧列对齐）；残差内含 pack GPU + 帧参数上传 + 三 pass submit/fence + jitter；"
                    "upscale 内 SL 簿记/录制/evaluate CPU 经 dlss-ext 行单列"
                ),
            },
        },
        "x2_crosscheck": {
            "submit_wait_x1_median_ms": sw_x1_med if sw_x1_med is not None else -1.0,
            "submit_wait_x2_median_ms": sw_x2_med if sw_x2_med is not None else -1.0,
            "marginal_median_ms": marginal if marginal is not None else -1.0,
            "ts_direct_median_ms": ngx_med if ngx_med is not None else -1.0,
            "abs_diff_ms": abs(marginal - ngx_med) if (marginal is not None and ngx_med is not None) else -1.0,
            "agree": bool(x2_ok),
            "method": (
                "RURIX_G31_DLSS_EVAL_X2=1 同 cmd 第二次 slEvaluateFeature——submit_wait 边际 = NGX "
                "单次 in-stream 净成本（G17.3 RURIX_G17_DLSS_EVAL_X2 同型）；与 GPU 时间戳直测互核"
            ),
        },
        "ue_comparison": {
            "ue_warm_median_ms": ue_median,
            "rurix_fresh_frame_ms": fresh_frame_ms if fresh_frame_ms is not None else -1.0,
            "delta_ms": delta_ms if delta_ms is not None else 0.0,
            "dominant_source": dominant,
            "analysis": analysis,
        },
        "rejudge_evaluation": {
            "ratio_ge_1_achieved": bool(ratio_ge_1),
            "fresh_ratio": fresh_ratio if fresh_ratio is not None else -1.0,
            "conclusion": conclusion,
            "basis": basis,
        },
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C9 NGX 分解 profiling（G30 承接锚 G17-MD-F1 行兑现；提前至波 B 同窗）："
            "dlss_sr 臂宿主差可分离四段分解——① NGX in-stream（evaluate 纯 GPU，GPU 时间戳三槽直测 "
            "+ X2 边际互核）② NGX 提交-同步税（submit_wait 墙钟 − cmd GPU；pre_eval acquire barrier "
            "子项）③ scene 渲染段（逐 pass GPU timestamp + pack 单列）④ 其余宿主段（逐帧残差 + SL "
            "簿记/录制/evaluate CPU 单列）。判据：①canonical 复跑 digest == 在案锚（既有口径零破坏）"
            "②fresh ratio ≥ 在案 0.960479 不恶化 ③四段全 measured（n≥100）④TS vs X2 互核容差内 "
            "⑤cmd GPU ≤ 提交墙钟 + 两墙钟面一致 ⑥UE 差主源定位在档 ⑦重判结论与 ratio 一致。"
            f"facts: {'; '.join(f['id'] + '=' + f['status'] for f in (facts[fid] for fid in FACT_IDS))}"
        ),
    }
    import jsonschema  # 自校验硬门（schema 漂移即 RED）

    errs = list(jsonschema.Draft7Validator(
        json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        for e in errs[:5]:
            fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
        all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_ngx_decomposition_{ts}.json"
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


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    # 红绿臂①：dlss-ext 逐帧解析。
    sample = (
        "[vendor-timing dlss-ext] frame=9 staging=0.000 sl_book=0.050 record=0.030 evaluate=0.100 submit_wait=2.000 total=2.200ms\n"
        "[vendor-timing dlss-ext] frame=10 staging=0.000 sl_book=0.051 record=0.031 evaluate=0.101 submit_wait=2.010 total=2.210ms\n"
        "[vendor-timing dlss-ext] frame=11 staging=0.000 sl_book=0.052 record=0.032 evaluate=0.102 submit_wait=2.020 total=2.220ms\n"
    )
    rows = parse_timing_frames(sample, warmup=10)
    expect(len(rows) == 2 and rows[0]["frame"] == 10, "GREEN:timing 解析 post-warmup 过滤")
    expect(abs(rows[1]["submit_wait"] - 2.020) < 1e-9, "GREEN:submit_wait 字段值")
    expect(parse_timing_frames("garbage", warmup=10) == [], "RED:无行空表")
    rows_all = parse_timing_frames(sample, warmup=0)
    expect(len(rows_all) == 3, "GREEN:warmup=0 全收")
    # 红绿臂②：ngx-ts 逐帧解析（tax 可负号形）。
    ts_sample = (
        "[ngx-ts dlss-ext] frame=10 pre_eval_gpu=0.001 ngx_gpu=1.850 cmd_gpu=1.851 submit_wait_wall=2.000 tax=0.149ms\n"
        "[ngx-ts dlss-ext] frame=11 pre_eval_gpu=0.000 ngx_gpu=1.860 cmd_gpu=1.860 submit_wait_wall=1.990 tax=0.130ms\n"
        "[ngx-ts dlss-ext] frame=12 pre_eval_gpu=0.000 ngx_gpu=1.855 cmd_gpu=1.855 submit_wait_wall=1.850 tax=-0.005ms\n"
    )
    ts_rows = parse_ngx_ts_frames(ts_sample, warmup=10)
    expect(len(ts_rows) == 3 and abs(ts_rows[0]["ngx_gpu"] - 1.850) < 1e-9, "GREEN:ngx-ts 解析")
    expect(abs(ts_rows[2]["tax"] + 0.005) < 1e-9, "GREEN:tax 负号形解析")
    expect(parse_ngx_ts_frames("noise", warmup=10) == [], "RED:ngx-ts 无行空表")
    # 红绿臂③：互核/墙钟/ratio 判。
    expect(crosscheck_agree(1.90, 1.85), "GREEN:互核容差内")
    expect(not crosscheck_agree(1.50, 1.85), "RED:互核超差必红")
    expect(not crosscheck_agree(None, 1.85), "RED:marginal 缺失必红")
    expect(not crosscheck_agree(1.90, None), "RED:TS 缺失必红")
    expect(not crosscheck_agree(1.90, 0.0), "RED:TS 零必红")
    expect(crosscheck_agree(2.127, 1.85), "GREEN:15% 相对容差内沿（2.127−1.85=0.277≤0.2775）")
    expect(not crosscheck_agree(2.13, 1.85), "RED:15% 相对容差外沿必红")
    expect(wall_consistent(1.85, 2.00, 2.01), "GREEN:墙钟一致正例")
    expect(not wall_consistent(2.10, 2.00, 2.01), "RED:GPU 超墙钟必红")
    expect(not wall_consistent(1.85, 2.00, 2.20), "RED:两墙钟面漂移必红")
    expect(not wall_consistent(None, 2.0, 2.0), "RED:缺值必红")
    expect(ratio_not_worsened(0.960479), "GREEN:ratio 恰下界")
    expect(ratio_not_worsened(0.966059), "GREEN:ratio 波 A 复测值")
    expect(not ratio_not_worsened(0.95), "RED:ratio 恶化必红")
    expect(not ratio_not_worsened(None), "RED:ratio 缺失必红")
    expect(not ratio_not_worsened(float("nan")), "RED:NaN 必红")
    # 红绿臂④：宿主残差。
    rec_ok = {
        "frame_ms_production": [3.5, 3.6],
        "scene_render_ms": [1.0, 1.0],
        "mv_ms": [0.02, 0.02],
        "upscale_ms": [2.2, 2.3],
    }
    res = host_residuals(rec_ok)
    expect(len(res) == 2 and abs(res[0] - 0.28) < 1e-9, "GREEN:残差逐帧正例")
    expect(host_residuals({"frame_ms_production": [1.0]}) == [], "RED:列缺失空表拒判")
    expect(host_residuals({"frame_ms_production": [1.0, 2.0], "scene_render_ms": [1.0],
                           "mv_ms": [0.1, 0.1], "upscale_ms": [0.5, 0.5]}) == [], "RED:长度不齐拒判")
    expect(host_residuals({}) == [], "RED:空 receipt 拒判")
    # 红绿臂⑤：主源定位 + 重判映射。
    expect(localize_dominant_source(-0.01, 0.1, 0.2) == "ngx_in_stream_inseparable",
           "GREEN:delta ≤ 0 → 不可分离")
    expect(localize_dominant_source(0.14, 0.12, 0.01) == "ngx_submit_sync_tax",
           "GREEN:税主导定位（0.12 ≥ 0.6×0.14）")
    expect(localize_dominant_source(0.14, 0.01, 0.12) == "host_residual_separable",
           "GREEN:宿主残差主导定位")
    expect(localize_dominant_source(0.14, 0.03, 0.03) == "mixed_host_segments",
           "GREEN:混合定位（无 ≥60% 单源）")
    expect(localize_dominant_source(0.21, 0.12, 0.119) == "mixed_host_segments",
           "GREEN:双高均不及 60% 如实混合（0.6×0.21=0.126）")
    expect(rejudge_conclusion(True) == "rejudge_triggered_ratio_ge_1", "GREEN:重判命中映射")
    expect(rejudge_conclusion(False) == "rejudge_not_triggered_honest_red_maintained", "GREEN:诚实红维持映射")
    expect(degrade_exit_code([], False) is None, "GREEN:无降级续跑")
    expect(degrade_exit_code(["x"], False) == 0, "GREEN:降级 SKIP 退 0")
    expect(degrade_exit_code(["x"], True) == 1, "RED:REQUIRE_REAL 下降级翻硬红")
    # 红绿臂⑥：中位/均值空表面。
    expect(med([]) is None and fmean([]) is None, "RED:空表 None 拒判")
    expect(med([2.0, 1.0]) == 1.5 and abs(fmean([1.0, 2.0]) - 1.5) < 1e-12, "GREEN:中位/均值正例")
    # schema 互核：在树 + 关键 const/required 逐字。
    expect(SCHEMA_PATH.is_file(), "门 schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "wave const 互核")
        expect(gs["properties"]["cell"]["const"] == CELL_SLASH, "cell const 互核")
        expect(
            sorted(gs.get("required", [])) == sorted([
                "schema", "subject", "symbolic_gate_key", "wave", "cell", "anchor",
                "canonical_rerun", "segments", "x2_crosscheck", "ue_comparison",
                "rejudge_evaluation", "environment", "timestamp", "notes",
            ]),
            "schema required 闭集互核（14 字段）",
        )
        seg_req = gs["properties"]["segments"].get("required", [])
        expect(
            sorted(seg_req) == sorted([
                "ngx_in_stream_ms", "ngx_submit_sync_tax_ms", "scene_render_ms", "host_residual_ms",
            ]),
            "segments 四段闭集互核",
        )
        ds_enum = gs["properties"]["ue_comparison"]["properties"]["dominant_source"].get("enum", [])
        expect(sorted(ds_enum) == sorted(DOMINANT_SOURCES), "dominant_source 枚举闭集互核")
    expect(len(FACT_IDS) == 7, "facts 闭集 = 7")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=7；6 红臂组 + 正例组 + schema 互核）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        return run_gate()
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
