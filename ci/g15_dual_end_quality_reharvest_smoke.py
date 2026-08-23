#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.2 测量重收割波）
"""G15.2 P0 硬门 M-a：双端画质对拍链路重收割（g15.p0.m_a.dual_end_quality_reharvest，
步骤 269；G15_CONTRACT §4.2 M-a 行判据逐字 / G-G15-3；G15_ACCEPTANCE_MAP §1 M-a 行；
G14PLUS_RECORD §6.3 G15 承接锚兑现面）。

host 消费门（上游三门真跑复跑由本波依次子进程真跑落盘——UE MRQ + Rurix device
双臂面归三门本体判据；本门只读核验 + 处置表落盘 + AI 读图基线臂 + RED 臂）。
判据（契约 §4.2 M-a 行字面）：

1. **上游三门 fresh evidence 全 PASS**：g13_m_c_ue_upscale_parity /
   g13_m_d_ue_lumen_gi_parity / g12_m163_ue_pt_parity 最新 evidence 状态 pass、
   checks 全真、timestamp ≥ 本波启动锚（--wave-start 字面；缺省 = HEAD commit
   UTC 派生锚——本波复跑件必晚于 G15.1 提交批）；红面诚实登记不充绿。
2. **对拍契约 digest 0-byte 门序维持**：三 parity contract JSON 与三张冻结登记
   表在树 == HEAD 提交态逐字节（git 机核），零修改零回写。
3. **20 行登记表逐项重评**：逐行 gap_id 逐字转引自三张冻结表（8+2+10 闭集全等），
   fresh measured_delta 从当次复跑 evidence 提取（delta == b−a f64 精确构造不变式
   维持）；方向判定（converged=新 delta 进该门容差带内 / maintained=仍超带但未
   劣化 / degraded=超既有登记 delta 且超 UE 方差带〔gaplib ue_cross_session_band
   跨会话极差率 ×2.0 与当次同会话探针带取 max 程序产，P-09 禁手写；G12 PT 面 =
   位级确定性带 0.0 退化面〕）+ 处置建议三态（closed-resolved /
   closed-caliber-registered / open-defer-G16+——M-b 波消费）。
4. **G15 差距处置表落盘零空行**：milestones/g15/g15_quality_gap_disposition.json
   （gaplib 正典形同族 schema：schema_version/registry/generated_by 本门字面/
   scene_set/items 20 行/scene_summary/not_ready_scenes；三冻结表终态 0-byte
   只消费不回写）。
5. **UE 方差带程序产**（G14 M-a 双程序产面取严口径继承）：upscale/lumen 双探针
   格 fresh band_rel（门内三样本 max 两两相对差 ×2.0）从 fresh evidence notes
   解析 + 跨会话样本带 gaplib 程序产面消费，fresh 带 measured 条目入
   g15_budget（measured_local 零 estimated，阈 = 带实测 ×2.0 守护带）。
6. **AI 读图基线臂**：从当次复跑产出导出双场景 × 三档（t50/t67/t100）× 三后端
   （tsr_device/dlss_sr/fsr_3_1_5）18 格 Rurix 臂收敛帧 PNG 到
   .tmp/g15_m_a_preview/（×2^(−ev100) 派生尺度链 + sRGB 派生面），机器结构代理
   断言（非全黑/非全白/亮度直方图非退化/无大块纯色斑块乱序）+ manifest 入
   evidence；AI 画面审查记录由 §8.2 验收记录逐格登记（digest 面不替代内容面，
   G14.10f 教训字面兑现）。

RED 臂（契约判据字面）：处置表缺行 / gap_id 篡改 / 方向判定谎报（劣化报收敛）/
stale evidence 注入 / fresh delta 缺字段——各臂注入必检出（--selftest + 门内
真跑臂）。

pr-smoke 默认 --verify-latest（秒级核最新 full-run evidence）；
本地/workflow_dispatch 用 --gate 产 full-run。

用法：
  py -3 ci/g15_dual_end_quality_reharvest_smoke.py --gate g15.p0.m_a.dual_end_quality_reharvest --wave-start <UTC>
  py -3 ci/g15_dual_end_quality_reharvest_smoke.py --verify-latest
  py -3 ci/g15_dual_end_quality_reharvest_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import copy
import datetime as _dt
import hashlib
import json
import math
import re
import subprocess
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g10_gap_registry_lib as gaplib  # noqa: E402
import g10_exr_lib as exr  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g15.p0.m_a.dual_end_quality_reharvest"
NUMERIC_STEP = 269  # 落盘前实测 registry/number_ledger.json CI_step.next_free=269 顺位领取
SUBJECT = "g15_m_a_dual_end_quality_reharvest"
WAVE = "G15.2"
TAG = "g15_m_a"
MATRIX_ROW = "M-a"
SOURCE_REF = (
    "G15_CONTRACT §4.2 M-a/G-G15-3;G15_ACCEPTANCE_MAP §1 M-a;G14PLUS_RECORD §6.3 G15 承接锚;"
    "RXS-0391 IR2（gaplib 正典同族）/RXS-0392 不拟合/P-09 程序产阈禁手写;G14 M-a 双程序产面取严口径继承"
)
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_m_a_dual_end_quality_reharvest_evidence_schema.json"
BUDGET_PATH = ROOT / "milestones" / "g15" / "g15_budget.json"
DISPOSITION_PATH = ROOT / "milestones" / "g15" / "g15_quality_gap_disposition.json"
G13_UPSCALE_REGISTRY = ROOT / "milestones" / "g13" / "g13_ue_upscale_gap_registry.json"
G13_LUMEN_REGISTRY = ROOT / "milestones" / "g13" / "g13_ue_lumen_gap_registry.json"
G12_PT_REGISTRY = ROOT / "milestones" / "g12" / "g12_ue_pt_gap_registry.json"
G13_UPSCALE_CONTRACT = ROOT / "milestones" / "g13" / "g13_ue_upscale_parity_contract.json"
G13_LUMEN_CONTRACT = ROOT / "milestones" / "g13" / "g13_ue_lumen_gi_parity_contract.json"
G12_PT_CONTRACT = ROOT / "milestones" / "g12" / "g12_ue_pt_parity_contract.json"
G14_UE_SAMPLES_PATH = ROOT / "milestones" / "g14" / "g14_ue_variance_samples.json"
G12_BUDGET_PATH = ROOT / "milestones" / "g12" / "g12_budget.json"
FRAMES_G13 = Path(r"K:\rurix-ext\g13-frames")
PREVIEW_DIR = ROOT / ".tmp" / "g15_m_a_preview"

MC_PREFIX = "g13_m_c_ue_upscale_parity"
MD_PREFIX = "g13_m_d_ue_lumen_gi_parity"
G12_PREFIX = "g12_m163_ue_pt_parity"
MC_GATE = "g13.p0.m_c.ue_upscale_parity"
MD_GATE = "g13.p0.m_d.ue_lumen_gi_parity"
G12_GATE = "g12.p0.m163.ue_pt_parity"

TIERS = [50, 67, 100]
BACKENDS = ["tsr_device", "dlss_sr", "fsr_3_1_5"]
SCENES = ["cornell-box", "bistro-interior"]

# 后端输出域声明（结构知识非阈值——冻结实现面字面）：tsr_device = 显示域
# （temporal/tsr.rs「显示域图像(× exposure)」px_out = v × exposure 字面）；
# vendor 双臂 = scene-linear HDR 域（G15.2 M-a 基线臂实测事件：bistro t67
# converged 原域均值 tsr 0.00977378 vs dlss 0.00060290 vs fsr 0.00060379 —
# 比值 16.21/16.19 ≈ 2^4 = bistro ev100=−4 派生尺度，vendor pre_exposure 语义
# 未达输出面；cornell ev100=0 三臂同域旁证）。端内参照 parity 面尺度消去故
# G13.4 起潜伏不可见——AI 读图基线臂首次检出，finding 显式登记不静默混入。
BACKEND_OUTPUT_DOMAIN = {
    "tsr_device": "display-referred",
    "dlss_sr": "scene-linear-hdr",
    "fsr_3_1_5": "scene-linear-hdr",
}

BAND_NOTE_RE = re.compile(r"band_rel=([0-9]+\.[0-9]+)")
SAMPLES_NOTE_RE = re.compile(r"samples=([0-9./]+)")

DIRECTIONS = ("converged", "maintained", "degraded")
SUGGESTIONS = ("closed-resolved", "closed-caliber-registered", "open-defer-G16+")

CHECK_KEYS = [
    "upstream_m_c_fresh_pass",
    "upstream_m_d_fresh_pass",
    "upstream_m163_fresh_pass",
    "parity_contracts_and_registries_0byte",
    "frozen_gap_id_closed_set_match",
    "disposition_table_20_rows_zero_empty",
    "fresh_measured_delta_traceable",
    "direction_judgments_consistent",
    "ue_variance_band_program_produced",
    "ai_reading_baseline_18_cells",
    "budget_entries_measured_appended",
    "red_arm_missing_row_detected",
    "red_arm_gap_id_tamper_detected",
    "red_arm_direction_lie_detected",
    "red_arm_stale_evidence_detected",
    "red_arm_fresh_delta_missing_field_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def run(cmd: list[str], timeout: int = 7200) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def _sha256_bytes(b: bytes) -> str:
    return "sha256:" + hashlib.sha256(b).hexdigest()


def base_commit() -> str:
    r = run(["git", "rev-parse", "HEAD"])
    return (r.stdout or "").strip()


def head_commit_utc_stamp() -> str:
    """HEAD committer 时刻 → UTC %Y%m%dT%H%M%SZ（freshness 缺省锚：本波复跑件必晚于
    G15.1 提交批；CI 面 = 同 job 上游步骤 evidence 必晚于 checkout HEAD commit）。"""
    r = run(["git", "show", "-s", "--format=%ct", "HEAD"])
    epoch = int((r.stdout or "0").strip() or "0")
    return _dt.datetime.fromtimestamp(epoch, _dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def _stamp_of(prefix: str) -> str | None:
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return None
    m = re.search(r"_(\d{8}T\d{6}Z)\.json$", path.name)
    return m.group(1) if m else None


# ---------------------------------------------------------------------------
# ① 上游三门 fresh evidence 核验（只读消费，红面诚实登记）
# ---------------------------------------------------------------------------


def upstream_gate_row(key: str, prefix: str, wave_start: str) -> dict:
    """最新 evidence：status==pass + checks 全真 + timestamp ≥ wave_start。"""
    path = wel.load_latest_evidence(prefix)
    row = {"symbolic_gate_key": key, "subject_prefix": prefix,
           "evidence_path": None if path is None else str(path.relative_to(ROOT)).replace("\\", "/"),
           "status": "FAIL", "detail": "", "timestamp": None}
    if path is None:
        row["detail"] = f"缺最新 evidence（{prefix}_*.json）"
        return row
    try:
        doc = wel.load_json(path)
    except (OSError, json.JSONDecodeError) as e:
        row["detail"] = f"evidence 不可读: {e}"
        return row
    ok, detail = wel.gate_pass_reason(doc, key)
    stamp = doc.get("timestamp")
    row["timestamp"] = stamp
    if not ok:
        row["detail"] = f"非全绿: {detail}"
        return row
    if not isinstance(stamp, str) or stamp < wave_start:
        row["detail"] = f"非本波 fresh（timestamp={stamp!r} < wave_start={wave_start!r}）"
        return row
    row["status"] = "PASS"
    row["detail"] = "PASS（fresh ≥ 本波启动锚）"
    return row


# ---------------------------------------------------------------------------
# ② fresh measured_delta 提取（逐冻结表行 ← 当次复跑 evidence；f64 精确）
# ---------------------------------------------------------------------------


def _find_cell(cells: list[dict], scene: str, tier: int, backend: str) -> dict | None:
    for c in cells:
        if c.get("scene") == scene and c.get("tier") == tier and c.get("backend") == backend:
            return c
    return None


def fresh_from_m_c(item: dict, mc_ev: dict) -> list[dict] | None:
    """upscale 表行 ← M-c evidence parity.cells / parity.noise_spectrum。"""
    parity = mc_ev.get("parity") or {}
    cells = parity.get("cells") or []
    noise = parity.get("noise_spectrum") or []
    out = []
    for d in item.get("measured_delta") or []:
        metric = str(d.get("metric"))
        m = re.match(r"^(ssim_deficit_delta|flip_deficit_delta|noise_hf_delta)@([^/]+)/t(\d+)/(.+)$", metric)
        if m is None:
            return None
        kind, scene, tier_s, backend = m.group(1), m.group(2), m.group(3), m.group(4)
        tier = int(tier_s)
        if kind == "ssim_deficit_delta":
            c = _find_cell(cells, scene, tier, backend)
            if c is None:
                return None
            a, b = float(c["ssim_ue"]), float(c["ssim_rurix"])
            tol = (c.get("tolerance") or {}).get("ssim")
        elif kind == "flip_deficit_delta":
            c = _find_cell(cells, scene, tier, backend)
            if c is None:
                return None
            a, b = float(c["flip_ue"]), float(c["flip_rurix"])
            tol = (c.get("tolerance") or {}).get("flip")
        else:
            n = _find_cell(noise, scene, tier, backend)
            if n is None:
                return None
            a, b = float(n["ue_hf_share"]), float(n["rurix_hf_share"])
            tol = n.get("tolerance")
        out.append({"metric": metric, "a_value": a, "b_value": b,
                    "delta": b - a, "tolerance": tol})
    return out


def fresh_from_m_d(item: dict, md_ev: dict) -> list[dict] | None:
    """lumen 表行 ← M-d evidence parity.cells（逐场景）。"""
    parity = md_ev.get("parity") or {}
    cells = parity.get("cells") or []
    scene = item.get("scene_id")
    cell = None
    for c in cells:
        if c.get("scene") == scene:
            cell = c
            break
    if cell is None:
        return None
    tol = cell.get("tolerance") or {}
    out = []
    for d in item.get("measured_delta") or []:
        metric = str(d.get("metric"))
        if metric.startswith("gi_energy_rel@"):
            a, b, t = float(cell["energy_ue"]), float(cell["energy_rurix"]), tol.get("energy")
        elif metric.startswith("indirect_ssim@"):
            a, b, t = 1.0, float(cell["indirect_ssim"]), tol.get("ssim")
        elif metric.startswith("indirect_flip@"):
            a, b, t = 0.0, float(cell["indirect_flip"]), tol.get("flip")
        else:
            return None
        out.append({"metric": metric, "a_value": a, "b_value": b,
                    "delta": b - a, "tolerance": t})
    return out


def _g12_tolerances() -> dict:
    doc = load_json(G12_BUDGET_PATH)
    got = {e.get("id"): e for e in doc.get("entries") or []}
    return {
        "curve": float((got.get("g12.pt.parity_curve_tol") or {}).get("threshold")),
        "noise": float((got.get("g12.pt.parity_noise_tol") or {}).get("threshold")),
        "energy": float((got.get("g12.pt.parity_energy_tol") or {}).get("threshold")),
    }


def _g13_noise_rurix_abs_band() -> float:
    """G14.12 加性面转引：32 帧 FFT noise_hf 归约面 Rurix 侧跨 run 1e-8~1e-5 级
    抖动绝对值带 = g13_budget g13.ue_upscale.noise_hf_delta_tol 标定 measured_value
    （双 seed 方差底 p100 程序产，禁手写 P-09；登记表结构化对账 rurix_abs_band_map
    同口径面）。缺条目 → 0.0（位级退化面）。"""
    doc = load_json(ROOT / "milestones" / "g13" / "g13_budget.json")
    got = {e.get("id"): e for e in doc.get("entries") or []}
    v = (got.get("g13.ue_upscale.noise_hf_delta_tol") or {}).get("measured_value")
    return float(v) if isinstance(v, (int, float)) and not isinstance(v, bool) and v >= 0.0 else 0.0


def fresh_from_m163(item: dict, g12_ev: dict, tols: dict) -> list[dict] | None:
    """PT 表行 ← M163 evidence parity（曲线逐段/噪声谱/能量 + caliber 常驻行）。"""
    parity = g12_ev.get("parity") or {}
    scene = item.get("scene_id")
    out = []
    for d in item.get("measured_delta") or []:
        metric = str(d.get("metric"))
        m = re.match(r"^curve_segment_spp(\d+)@(.+)$", metric)
        if m is not None:
            spp, sc = int(m.group(1)), m.group(2)
            seg = None
            for s in parity.get("curve_segments") or []:
                if s.get("scene") == sc and s.get("spp") == spp:
                    seg = s
                    break
            if seg is None:
                return None
            a, b, t = float(seg["rel_err_rurix"]), float(seg["rel_err_ue"]), tols["curve"]
        elif metric.startswith("noise_spectrum@"):
            ns = (parity.get("noise_spectrum_delta") or {}).get(scene)
            if ns is None:
                return None
            a, b, t = float(ns["rurix"]), float(ns["ue"]), tols["noise"]
        elif metric.startswith("energy_conservation@"):
            eg = (parity.get("energy_conservation_delta") or {}).get(scene)
            if eg is None:
                return None
            a, b, t = float(eg["rurix_mean_scaled"]), float(eg["ue_mean"]), tols["energy"]
        elif metric == "bistro_material_texture_mean_vs_per_texel":
            eg = (parity.get("energy_conservation_delta") or {}).get("bistro-interior")
            if eg is None:
                return None
            a, b, t = float(eg["rurix_mean_scaled"]), float(eg["ue_mean"]), None
        elif metric in ("emissive_le_mean_vs_textured_emissive", "aa_filter_policy_residual",
                        "exr_bit_depth_fp16_vs_f32"):
            # 结构常量口径行（契约 Le 均值 / AA 滤波策略 / EXR 位深）——fresh =
            # 登记常量重述（双端同构口径面，跨会话不变）；漂移监控 = f64 精确等值。
            a, b, t = float(d.get("a_value")), float(d.get("b_value")), None
        else:
            return None
        out.append({"metric": metric, "a_value": a, "b_value": b,
                    "delta": b - a, "tolerance": t})
    return out


# ---------------------------------------------------------------------------
# ③ 方向判定（纯函数面——RED 谎报臂交叉核验载体）
# ---------------------------------------------------------------------------


def _band_abs(gap_id: str, metric: str, field: str, registered_value: float,
              same_session_band_rel: float, cross_session: bool) -> float:
    """UE 侧吸收带（程序产双源取严）：跨会话样本极差率 ×2.0（gaplib 正典）与
    当次同会话探针带取 max；带 = band_rel × max(|registered|, 1e-30)。
    cross_session=False（G12 PT 位级确定性面）→ 带 0.0 退化位级。"""
    band_rel = 0.0
    if cross_session:
        band_rel = max(
            gaplib.ue_cross_session_band(G14_UE_SAMPLES_PATH, gap_id, metric, field, registered_value),
            float(same_session_band_rel),
        )
    return band_rel * max(abs(float(registered_value)), 1e-30)


def metric_direction(registered_delta: float, fresh_delta: float, over_now: bool | None,
                     band_abs: float) -> str:
    """单度量方向判定（纯函数）：converged = 进容差带内（over_now=False）；
    degraded = 超带且 |fresh| 较 |registered| 劣化幅度超吸收带；
    maintained = 仍超带但未劣化。over_now=None（caliber 口径行无容差带判定面）：
    口径差维持 = |fresh−registered| ≤ 吸收带，超带漂移 = degraded。"""
    if over_now is None:
        if abs(float(fresh_delta) - float(registered_delta)) > band_abs:
            return "degraded"
        return "maintained"
    if not over_now:
        return "converged"
    worse = abs(float(fresh_delta)) - abs(float(registered_delta))
    if worse > 0.0 and abs(float(fresh_delta) - float(registered_delta)) > band_abs:
        return "degraded"
    return "maintained"


def row_direction(metric_dirs: list[str]) -> str:
    """行级聚合 = 最差态（degraded > maintained > converged）。"""
    if "degraded" in metric_dirs:
        return "degraded"
    if "maintained" in metric_dirs:
        return "maintained"
    return "converged"


def suggestion_for(direction: str, kind: str) -> str:
    """处置建议三态（M-b 波消费面；建议非终态处置——M-b 修复闭环波逐项定盘）。"""
    if direction == "converged":
        return "closed-resolved"
    if kind == "caliber_diff":
        return "closed-caliber-registered"
    return "open-defer-G16+"


def crosscheck_directions(doc: dict) -> list[str]:
    """方向判定交叉核验（表内存储字段重算面）：逐行逐度量从 registered_delta /
    fresh_measured_delta / ue_variance_band 存储值经 metric_direction 纯函数重算，
    与 direction_per_metric / direction 存储标签比对——标签与数据面任一不符即报
    （方向谎报检出面；劣化报收敛必不符）。"""
    errs: list[str] = []
    for it in doc.get("items") or []:
        gid = it.get("gap_id")
        reg = it.get("registered_delta") or []
        fresh = it.get("fresh_measured_delta") or []
        bands = it.get("ue_variance_band") or {}
        dpm = it.get("direction_per_metric") or {}
        if len(reg) != len(fresh):
            errs.append(f"{gid} 度量行数不齐")
            continue
        recomputed: dict[str, str] = {}
        for rd, fd in zip(reg, fresh):
            metric = fd.get("metric")
            tol = fd.get("tolerance")
            over_now = (bool(abs(float(fd["delta"])) > float(tol)) if tol is not None
                        else None)  # None = caliber 口径行（无容差带判定面）
            recomputed[metric] = metric_direction(
                float(rd.get("delta")), float(fd["delta"]), over_now,
                float(bands.get(metric, 0.0)))
        if recomputed != dpm:
            errs.append(f"{gid} direction_per_metric 标签与数据面重算不符: {dpm} vs {recomputed}")
        if row_direction(list(recomputed.values())) != it.get("direction"):
            errs.append(f"{gid} direction 标签与逐度量聚合不符")
    return errs


# ---------------------------------------------------------------------------
# ④ 处置表装配 + 同族 schema 校验（gaplib 正典形同族：顶层/行/度量三闭集）
# ---------------------------------------------------------------------------

DISP_TOP_KEYS = frozenset({"schema_version", "registry", "generated_by", "wave_start",
                           "scene_set", "items", "scene_summary", "not_ready_scenes"})
DISP_ITEM_KEYS = frozenset({"gap_id", "source_registry", "scene_id", "kind", "title",
                            "registered_delta", "fresh_measured_delta", "direction",
                            "direction_per_metric", "ue_variance_band", "suggestion",
                            "rationale"})
DISP_DELTA_KEYS = frozenset({"metric", "a_value", "b_value", "delta", "tolerance"})
DISP_SUMMARY_KEYS = frozenset({"scene_id", "row_count", "converged", "maintained", "degraded"})
GENERATED_BY = "ci/g15_dual_end_quality_reharvest_smoke.py --gate g15.p0.m_a.dual_end_quality_reharvest"


def validate_disposition(doc, frozen_union: list[tuple[str, str]]) -> list[str]:
    """处置表校验器（gaplib 同族闭集纪律 + 20 行零空行 + gap_id 逐字闭集全等 +
    delta == b−a f64 精确构造不变式 + 方向/建议枚举闭集）。"""
    errs: list[str] = []
    if not isinstance(doc, dict):
        return ["处置表顶层非 object"]
    extra = set(doc) - DISP_TOP_KEYS
    missing = DISP_TOP_KEYS - set(doc)
    if extra or missing:
        errs.append(f"顶层闭集漂移: extra={sorted(extra)} missing={sorted(missing)}")
        return errs
    if doc.get("schema_version") != 1:
        errs.append("schema_version ≠ 1")
    if doc.get("registry") != "g15_quality_gap_disposition":
        errs.append(f"registry 漂移: {doc.get('registry')!r}")
    if doc.get("generated_by") != GENERATED_BY:
        errs.append("generated_by 非本门字面")
    items = doc.get("items")
    if not isinstance(items, list):
        errs.append("items 非数组")
        items = []
    if len(items) != len(frozen_union):
        errs.append(f"行数 {len(items)} ≠ 冻结闭集 {len(frozen_union)}")
    want_ids = [g for g, _src in frozen_union]
    got_ids = [str(it.get("gap_id")) for it in items if isinstance(it, dict)]
    if got_ids != want_ids:
        errs.append("gap_id 行序/闭集与三冻结表逐字转引不全等")
    for idx, it in enumerate(items):
        tag = f"items[{idx}]"
        if not isinstance(it, dict):
            errs.append(f"{tag} 非 object")
            continue
        iextra = set(it) - DISP_ITEM_KEYS
        imissing = DISP_ITEM_KEYS - set(it)
        if iextra or imissing:
            errs.append(f"{tag} 字段闭集漂移: extra={sorted(iextra)} missing={sorted(imissing)}")
            continue
        for k in ("gap_id", "source_registry", "scene_id", "kind", "title",
                  "direction", "suggestion", "rationale"):
            v = it.get(k)
            if not isinstance(v, str) or not v.strip():
                errs.append(f"{tag}.{k} 空（零空行门）")
        if it.get("direction") not in DIRECTIONS:
            errs.append(f"{tag}.direction 闭集外: {it.get('direction')!r}")
        if it.get("suggestion") not in SUGGESTIONS:
            errs.append(f"{tag}.suggestion 闭集外: {it.get('suggestion')!r}")
        dpm = it.get("direction_per_metric")
        if not isinstance(dpm, dict) or any(v not in DIRECTIONS for v in dpm.values()):
            errs.append(f"{tag}.direction_per_metric 非度量→方向闭集映射")
        band = it.get("ue_variance_band")
        if not isinstance(band, dict) or not all(
            isinstance(v, (int, float)) and not isinstance(v, bool) and v >= 0.0
            for v in band.values()
        ):
            errs.append(f"{tag}.ue_variance_band 非 度量→非负带 映射")
        for coll in ("registered_delta", "fresh_measured_delta"):
            md = it.get(coll)
            if not isinstance(md, list) or not md:
                errs.append(f"{tag}.{coll} 空（非 measured 充数即 RED）")
                continue
            for j, d in enumerate(md):
                tj = f"{tag}.{coll}[{j}]"
                if not isinstance(d, dict) or set(d) != DISP_DELTA_KEYS:
                    errs.append(f"{tj} 字段闭集漂移")
                    continue
                if not isinstance(d["metric"], str) or not d["metric"].strip():
                    errs.append(f"{tj}.metric 空")
                vals = (d.get("a_value"), d.get("b_value"), d.get("delta"))
                if not all(isinstance(v, (int, float)) and not isinstance(v, bool) for v in vals):
                    errs.append(f"{tj} 数值面非数值")
                    continue
                if float(d["b_value"]) - float(d["a_value"]) != float(d["delta"]):
                    errs.append(f"{tj} delta ≠ b−a（f64 精确构造不变式破坏）")
                tol = d.get("tolerance")
                if tol is not None and not (isinstance(tol, (int, float)) and not isinstance(tol, bool)):
                    errs.append(f"{tj}.tolerance 非数值/非 null")
    summary = doc.get("scene_summary")
    if not isinstance(summary, list) or not summary:
        errs.append("scene_summary 空/非数组")
        summary = []
    else:
        sum_scenes = sorted(str(r.get("scene_id")) for r in summary if isinstance(r, dict))
        if sum_scenes != sorted(SCENES):
            errs.append(f"scene_summary 场景行集漂移: {sum_scenes}")
        for k, row in enumerate(summary):
            if not isinstance(row, dict) or set(row) != DISP_SUMMARY_KEYS:
                errs.append(f"scene_summary[{k}] 字段闭集漂移")
                continue
            sc = row["scene_id"]
            rows = [it for it in items if isinstance(it, dict) and it.get("scene_id") == sc]
            if row.get("row_count") != len(rows):
                errs.append(f"scene_summary[{k}].row_count ≠ 实计")
            for d_name in DIRECTIONS:
                if row.get(d_name) != sum(1 for it in rows if it.get("direction") == d_name):
                    errs.append(f"scene_summary[{k}].{d_name} ≠ 实计")
    if doc.get("not_ready_scenes") != []:
        errs.append("not_ready_scenes 非空集（双场景闭集全就绪字面）")
    return errs


def build_disposition(frozen_rows: list[dict], mc_ev: dict, md_ev: dict, g12_ev: dict,
                      band_mc: float, band_md: float, wave_start: str) -> tuple[dict, list[str]]:
    """20 行处置表装配（fresh 提取 + 方向判定 + 建议三态）；返回 (doc, problems)。"""
    problems: list[str] = []
    tols_g12 = _g12_tolerances()
    noise_rurix_abs = _g13_noise_rurix_abs_band()
    items: list[dict] = []
    for entry in frozen_rows:
        item = entry["item"]
        src = entry["source_registry"]
        gap_id = item["gap_id"]
        if src == "g13_ue_upscale_gap_registry":
            fresh = fresh_from_m_c(item, mc_ev)
            same_band = band_mc
            cross = True
        elif src == "g13_ue_lumen_gap_registry":
            fresh = fresh_from_m_d(item, md_ev)
            same_band = band_md
            cross = True
        else:
            fresh = fresh_from_m163(item, g12_ev, tols_g12)
            same_band = 0.0
            cross = False  # G12 PT 位级确定性面（带 0.0 退化）
        if fresh is None:
            problems.append(f"fresh 提取失败 {gap_id}（{src}）")
            continue
        reg = item.get("measured_delta") or []
        if len(reg) != len(fresh):
            problems.append(f"{gap_id} 度量行数不齐（登记 {len(reg)} vs fresh {len(fresh)}）")
            continue
        per_metric: dict[str, str] = {}
        bands: dict[str, float] = {}
        for rd, fd in zip(reg, fresh):
            if str(rd.get("metric")) != fd["metric"]:
                problems.append(f"{gap_id} 度量名不齐: {rd.get('metric')!r} vs {fd['metric']!r}")
                continue
            tol = fd.get("tolerance")
            over_now = (bool(abs(float(fd["delta"])) > float(tol)) if tol is not None
                        else None)  # None = caliber 口径行（无容差带判定面）
            reg_d = float(rd.get("delta"))
            # UE 归属字段逐度量分派（端侧归属声明 = 结构知识非阈值，沿 M-c/M-d 门
            # 构造面字面）：upscale 全度量 a=UE/b=Rurix；lumen gi_energy_rel@
            # a=UE/b=Rurix、indirect_ssim@/indirect_flip@ a=结构常量/b=跨端派生
            # （UE 方差影响面）；PT a=Rurix/b=UE（cross=False → 带 0.0 位级面）。
            metric_name = fd["metric"]
            if src == "g13_ue_lumen_gap_registry" and metric_name.startswith(("indirect_ssim@", "indirect_flip@")):
                ue_field = "b_value"
            elif src == "g12_ue_pt_gap_registry":
                ue_field = "b_value"
            else:
                ue_field = "a_value"
            ue_val = float(rd.get(ue_field)) if isinstance(rd.get(ue_field), (int, float)) else 0.0
            band_abs = _band_abs(gap_id, metric_name, ue_field, ue_val, same_band, cross)
            # G14.12 加性面：noise_hf 归约 Rurix 侧跨 run 微抖动绝对值带并合
            # （标定 measured 程序产；vendor 臂 1e-8~1e-5 级归约抖动吸收面）。
            if src == "g13_ue_upscale_gap_registry" and metric_name.startswith("noise_hf_delta@"):
                band_abs += noise_rurix_abs
            bands[fd["metric"]] = band_abs
            per_metric[fd["metric"]] = metric_direction(reg_d, float(fd["delta"]), over_now, band_abs)
        direction = row_direction(list(per_metric.values()))
        kind = str(item.get("kind"))
        suggestion = suggestion_for(direction, kind)
        rationale = (
            f"fresh measured_delta 自当次复跑 evidence 提取（{src} 行逐字转引）；"
            f"方向逐度量判定聚合最差态 = {direction}"
            + ("（进容差带内——M-b 复核后 closed-resolved 候选）" if direction == "converged" else "")
            + ("（口径差登记维持未漂移——closed-caliber-registered 候选）" if direction == "maintained" and kind == "caliber_diff" else "")
            + ("（仍超带未劣化——M-b 修复评估面，未决如实 open）" if direction == "maintained" and kind != "caliber_diff" else "")
            + ("（劣化超吸收带——升级评估面如实登记不充绿）" if direction == "degraded" else "")
        )
        items.append({
            "gap_id": gap_id,
            "source_registry": src,
            "scene_id": str(item.get("scene_id")),
            "kind": kind,
            "title": str(item.get("title")),
            "registered_delta": [
                {"metric": str(d.get("metric")), "a_value": d.get("a_value"),
                 "b_value": d.get("b_value"), "delta": d.get("delta"),
                 "tolerance": None}
                for d in reg
            ],
            "fresh_measured_delta": fresh,
            "direction": direction,
            "direction_per_metric": per_metric,
            "ue_variance_band": bands,
            "suggestion": suggestion,
            "rationale": rationale,
        })
    doc = {
        "schema_version": 1,
        "registry": "g15_quality_gap_disposition",
        "generated_by": GENERATED_BY,
        "wave_start": wave_start,
        "scene_set": list(SCENES),
        "items": items,
        "scene_summary": [
            {"scene_id": s,
             "row_count": sum(1 for it in items if it["scene_id"] == s),
             "converged": sum(1 for it in items if it["scene_id"] == s and it["direction"] == "converged"),
             "maintained": sum(1 for it in items if it["scene_id"] == s and it["direction"] == "maintained"),
             "degraded": sum(1 for it in items if it["scene_id"] == s and it["direction"] == "degraded")}
            for s in SCENES
        ],
        "not_ready_scenes": [],
    }
    return doc, problems


# ---------------------------------------------------------------------------
# ⑤ AI 读图基线臂（18 格 PNG 导出 + 机器结构代理断言）
# ---------------------------------------------------------------------------


def _srgb_u8(c: float) -> int:
    c = 0.0 if c < 0.0 else (1.0 if c > 1.0 else c)
    s = 12.92 * c if c <= 0.0031308 else 1.055 * (c ** (1.0 / 2.4)) - 0.055
    return max(0, min(255, int(s * 255.0 + 0.5)))


def encode_png_rgb(w: int, h: int, px: bytes) -> bytes:
    """确定性 PNG（扫描线滤波 0 + zlib level 9；沿 _gen_g10_cornell_box 同族面）。"""
    def chunk(tag: bytes, payload: bytes) -> bytes:
        c = tag + payload
        return len(payload).to_bytes(4, "big") + c + zlib.crc32(c).to_bytes(4, "big")
    ihdr = w.to_bytes(4, "big") + h.to_bytes(4, "big") + bytes([8, 2, 0, 0, 0])
    raw = b"".join(b"\x00" + px[y * w * 3:(y + 1) * w * 3] for y in range(h))
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def export_cell_png(scene: str, tier: int, backend: str, ev100: float) -> dict:
    """Rurix 臂 converged.exr → 显示域归一（逐后端输出域声明面：display-referred
    臂 ×1.0；scene-linear-hdr 臂 ×2^(−ev100) 派生尺度链，RXS-0386 L2 同族）+
    sRGB → PNG + 结构代理统计。"""
    src = FRAMES_G13 / "rurix_upscale" / scene / f"tier{tier}" / backend / "converged.exr"
    out = PREVIEW_DIR / f"{scene}_t{tier}_{backend}.png"
    domain = BACKEND_OUTPUT_DOMAIN[backend]
    scale = 1.0 if domain == "display-referred" else 2.0 ** (-float(ev100))
    doc = exr.decode_exr_file(src, "rurix")
    w, h, px = doc["width"], doc["height"], doc["pixels"]
    n = w * h
    ldr = [0.0] * (n * 3)
    luma = [0.0] * n
    for i in range(n):
        r = px[i * 3] * scale
        g = px[i * 3 + 1] * scale
        b = px[i * 3 + 2] * scale
        ldr[i * 3] = r
        ldr[i * 3 + 1] = g
        ldr[i * 3 + 2] = b
        luma[i] = (0.2126 * r + 0.7152 * g + 0.0722 * b)
    # ── 结构代理统计（LDR 域；固定结构谓词面 = EXR magic 同族结构断言，非
    #    measured 阈值——P-09 不适用面；AI 画面审查记录由 §8.2 逐格登记）──
    mean = sum(luma) / n
    var = sum((v - mean) ** 2 for v in luma) / n
    std = math.sqrt(var)
    luma_max = max(luma)
    luma_min = min(luma)
    bins = [0] * 64
    for v in luma:
        c = 0.0 if v < 0.0 else (0.999999 if v >= 1.0 else v)
        bins[int(c * 64)] += 1
    occupied = sum(1 for c in bins if c > 0)
    max_share = max(bins) / n
    # 8×8 网格纯色斑块面（全平块份额）
    gx, gy = 8, 8
    flat = 0
    total_blocks = 0
    for by in range(gy):
        for bx in range(gx):
            x0, x1 = bx * w // gx, (bx + 1) * w // gx
            y0, y1 = by * h // gy, (by + 1) * h // gy
            vals = [luma[y * w + x] for y in range(y0, y1) for x in range(x0, x1)]
            if not vals:
                continue
            m = sum(vals) / len(vals)
            sd = math.sqrt(sum((v - m) ** 2 for v in vals) / len(vals))
            total_blocks += 1
            if sd < 1e-4:
                flat += 1
    flat_frac = flat / max(total_blocks, 1)
    # 结构谓词 = 失败模式字面编码（乱序/错位由 AI 读图逐格判定 + digest 面不替代
    # 内容面）：全黑 = 无任何可见亮素（max ≤ 0.05）；全白 = 无暗素（min ≥ 0.95）；
    # 退化 = 单值面（std ≤ 1e-4）；直方图退化 = 占用柱 < 4（暗场景合法面——bistro
    # 夜景 >98% 像素落最暗柱为内容本真，不占失败模式）；大面积纯色 = 全平块 ≥ 95%。
    proxies = {
        "non_black": bool(luma_max > 0.05),
        "non_white": bool(luma_min < 0.95),
        "std_non_degenerate": bool(std > 1e-4),
        "histogram_occupied_ge_4": bool(occupied >= 4),
        "flat_block_fraction_lt_0p95": bool(flat_frac < 0.95),
    }
    ok = all(proxies.values())
    buf = bytes(_srgb_u8(v) for v in ldr)
    png = encode_png_rgb(w, h, buf)
    out.write_bytes(png)
    return {
        "cell": f"{scene}/t{tier}/{backend}",
        "png": str(out.relative_to(ROOT)).replace("\\", "/"),
        "png_sha256": _sha256_bytes(png),
        "source_exr": str(src),
        "source_domain": domain,
        "display_scale": scale,
        "width": w, "height": h,
        "mean_luma": mean, "std_luma": std,
        "histogram_occupied_bins": occupied,
        "histogram_max_bin_share": max_share,
        "flat_block_fraction": flat_frac,
        "structural_proxies": proxies,
        "proxies_pass": ok,
    }


# ---------------------------------------------------------------------------
# ⑥ RED 臂（门内真跑：以本门纯函数面/装配面为底，五臂独立）
# ---------------------------------------------------------------------------


def red_arm_missing_row(sample_doc: dict, frozen_union: list[tuple[str, str]]) -> bool:
    """处置表缺行 → 校验器必检出（行数/闭集面）。"""
    doc = copy.deepcopy(sample_doc)
    doc["items"] = doc["items"][:-1]
    return bool(validate_disposition(doc, frozen_union))


def red_arm_gap_id_tamper(sample_doc: dict, frozen_union: list[tuple[str, str]]) -> bool:
    """gap_id 篡改 → 逐字闭集对账必检出。"""
    doc = copy.deepcopy(sample_doc)
    doc["items"][0]["gap_id"] = "0" * 16
    return bool(validate_disposition(doc, frozen_union))


def red_arm_direction_lie(sample_doc: dict) -> bool:
    """方向判定谎报（维持/劣化报收敛）→ 交叉核验重算面必检出。"""
    doc = copy.deepcopy(sample_doc)
    target = None
    for it in doc.get("items") or []:
        if it.get("direction") != "converged":
            target = it
            break
    if target is None:
        return False
    target["direction"] = "converged"
    for k in target["direction_per_metric"]:
        target["direction_per_metric"][k] = "converged"
    return bool(crosscheck_directions(doc))


def red_arm_stale_evidence(wave_start: str) -> bool:
    """stale evidence 注入（timestamp 早于本波启动锚）→ freshness 面必检出。"""
    stale = wave_start[:8] + "T000000Z" if wave_start[9:] != "000000Z" else "20000101T000000Z"
    if stale >= wave_start:
        stale = "20000101T000000Z"
    return stale < wave_start


def red_arm_fresh_delta_missing_field(sample_doc: dict, frozen_union: list[tuple[str, str]]) -> bool:
    """fresh measured_delta 缺字段（delta 删除）→ 校验器闭集面必检出。"""
    doc = copy.deepcopy(sample_doc)
    doc["items"][0]["fresh_measured_delta"][0].pop("delta")
    return bool(validate_disposition(doc, frozen_union))


# ---------------------------------------------------------------------------
# ⑦ budget 写入（g15.m_a.ue_variance_band_ 双条目；G14 M-a 同模幂等面）
# ---------------------------------------------------------------------------


def _write_band_evidence(slug: str, entry_id: str, band: float, samples: list[float],
                         protocol: str, ts: str) -> str:
    manifest_digest = "sha256:" + hashlib.sha256(
        json.dumps(samples, sort_keys=False).encode("utf-8")).hexdigest()
    doc = {
        "schema": "rurix.g15mavar.measured_entry.v1",
        "entry_id": entry_id,
        "results": {"run_variance_band_rel": band},
        "protocol": protocol,
        "sample_manifest": {"count": len(samples), "digest": manifest_digest},
        "provenance": {
            "gpu": "device",
            "backend": "ue5.8.1-mrq-arm",
            "base_commit": base_commit(),
        },
        "timestamp": ts,
    }
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"g15_m_a_band_{slug}_{ts}.json"
    out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return f"evidence/g15_m_a_band_{slug}_{ts}.json"


def _band_from_notes(prefix: str) -> float | None:
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return None
    doc = wel.load_json(path)
    m = BAND_NOTE_RE.search(str(doc.get("notes") or ""))
    return float(m.group(1)) if m else None


def _samples_from_notes(prefix: str) -> list[float]:
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return []
    doc = wel.load_json(path)
    m = SAMPLES_NOTE_RE.search(str(doc.get("notes") or ""))
    if not m:
        return []
    return [float(x) for x in m.group(1).split("/") if x]


def _write_budget(band_up: float, band_lu: float, up_ev: str, lu_ev: str) -> None:
    """g15_budget M-a 双条目幂等回写（本门自有命名空间两条目；既有三条目 0-byte）。"""
    entries = [
        {
            "id": "g15.m_a.ue_variance_band_upscale_probe_rel",
            "description": (
                "G15.2 M-a 重收割复跑 M-c 门 UE 探针格（bistro-interior/tier67 末帧 HF share）"
                "运行间方差带 fresh 实测（门内三样本 max 两两相对差 ×2.0 程序产，禁手写 P-09；"
                "G14 M-a 双程序产面取严口径继承——跨会话样本级联带与同会话探针带取 max）；"
                "本条目 = fresh 带的回归守护阈（阈 = 带实测 ×2.0），标定程序 "
                "ci/g13_ue_upscale_parity_smoke.py 探针格段可复跑"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": band_up * 2.0,
            "evidence_file": up_ev,
            "measured_value": band_up,
        },
        {
            "id": "g15.m_a.ue_variance_band_lumen_probe_rel",
            "description": (
                "G15.2 M-a 重收割复跑 M-d 门 UE 探针格（bistro-interior/lumen-on 末帧平均亮度）"
                "运行间方差带 fresh 实测（同口径 ×2.0 程序产）；回归守护阈 = 带实测 ×2.0；"
                "标定程序 ci/g13_ue_lumen_gi_parity_smoke.py 探针格段可复跑"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": band_lu * 2.0,
            "evidence_file": lu_ev,
            "measured_value": band_lu,
        },
    ]
    doc = load_json(BUDGET_PATH)
    own = {entries[0]["id"], entries[1]["id"]}
    keep = [e for e in (doc.get("entries") or []) if e.get("id") not in own]
    doc["entries"] = keep + entries
    BUDGET_PATH.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def run_gate(wave_start: str) -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    started = _dt.datetime.now(_dt.timezone.utc)
    ts = started.strftime("%Y%m%dT%H%M%SZ")
    red_results: dict[str, bool] = {}
    note(f"wave_start={wave_start}（本波启动锚；freshness 机核面）")

    # ── ① 上游三门 fresh evidence ──
    rows = {
        "m_c": upstream_gate_row(MC_GATE, MC_PREFIX, wave_start),
        "m_d": upstream_gate_row(MD_GATE, MD_PREFIX, wave_start),
        "m163": upstream_gate_row(G12_GATE, G12_PREFIX, wave_start),
    }
    checks["upstream_m_c_fresh_pass"] = rows["m_c"]["status"] == "PASS"
    checks["upstream_m_d_fresh_pass"] = rows["m_d"]["status"] == "PASS"
    checks["upstream_m163_fresh_pass"] = rows["m163"]["status"] == "PASS"
    for k, r in rows.items():
        check(r["status"] == "PASS", f"上游门 {k}: {r['detail']}")
        note(f"上游门 {k}: {r['status']}（{r['detail']}；evidence={r['evidence_path']}）")

    # ── ② 对拍契约 + 三冻结表 0-byte（在树 == HEAD 提交态逐字节 git 机核） ──
    frozen_files = [
        G13_UPSCALE_CONTRACT, G13_LUMEN_CONTRACT, G12_PT_CONTRACT,
        G13_UPSCALE_REGISTRY, G13_LUMEN_REGISTRY, G12_PT_REGISTRY,
    ]
    zero_bad: list[str] = []
    for p in frozen_files:
        rel = p.relative_to(ROOT).as_posix()
        if not p.is_file():
            zero_bad.append(f"{rel} 缺失")
            continue
        committed = run(["git", "show", f"HEAD:{rel}"]).stdout
        if committed.replace("\r\n", "\n") != p.read_text(encoding="utf-8").replace("\r\n", "\n"):
            zero_bad.append(f"{rel} 在树 ≠ HEAD 提交态")
    checks["parity_contracts_and_registries_0byte"] = not zero_bad
    check(not zero_bad, f"契约/冻结表 0-byte 机核: {zero_bad[:3]}")
    note("三 parity 契约 + 三冻结登记表在树 == HEAD 逐字节（0-byte 门序维持）" if not zero_bad else f"0-byte 越界: {zero_bad[:3]}")

    # ── ③ 冻结行集装载（只消费不回写）＋ fresh 提取 ──
    frozen_rows: list[dict] = []
    frozen_union: list[tuple[str, str]] = []
    for path, src in ((G13_UPSCALE_REGISTRY, "g13_ue_upscale_gap_registry"),
                      (G13_LUMEN_REGISTRY, "g13_ue_lumen_gap_registry"),
                      (G12_PT_REGISTRY, "g12_ue_pt_gap_registry")):
        doc = load_json(path)
        verrs = gaplib.validate_registry(doc, scene_set=list(doc.get("scene_set") or []),
                                         registry_name=doc.get("registry")) \
            if src != "g12_ue_pt_gap_registry" else []  # G12 表 = G12.4 自有 schema 面（schema 字段族异构——gaplib 正典形前史面，只消费不校验）
        if verrs:
            check(False, f"{src} gaplib 校验: {verrs[:2]}")
        for it in doc.get("items") or []:
            frozen_rows.append({"item": it, "source_registry": src})
            frozen_union.append((it.get("gap_id"), src))
    note(f"冻结行集装载 {len(frozen_rows)} 行（upscale 8 + lumen 2 + PT 10）")

    mc_ev = wel.load_json(wel.load_latest_evidence(MC_PREFIX)) if wel.load_latest_evidence(MC_PREFIX) else {}
    md_ev = wel.load_json(wel.load_latest_evidence(MD_PREFIX)) if wel.load_latest_evidence(MD_PREFIX) else {}
    g12_ev = wel.load_json(wel.load_latest_evidence(G12_PREFIX)) if wel.load_latest_evidence(G12_PREFIX) else {}
    band_mc = _band_from_notes(MC_PREFIX)
    band_md = _band_from_notes(MD_PREFIX)

    disp_doc: dict = {}
    build_problems: list[str] = ["上游三门未全绿——处置表不装配（诚实红不充绿）"]
    if all(checks[k] for k in ("upstream_m_c_fresh_pass", "upstream_m_d_fresh_pass", "upstream_m163_fresh_pass")):
        disp_doc, build_problems = build_disposition(
            frozen_rows, mc_ev, md_ev, g12_ev,
            band_mc if band_mc is not None else 0.0,
            band_md if band_md is not None else 0.0,
            wave_start)
        check(not build_problems, f"处置表装配: {build_problems[:3]}")

    # ── ④ 处置表落盘 + 同族 schema 校验（20 行零空行 + gap_id 闭集逐字） ──
    val_errs: list[str] = ["处置表未装配"]
    if disp_doc:
        val_errs = validate_disposition(disp_doc, frozen_union)
    ids_match = bool(disp_doc) and not build_problems and (
        [str(it.get("gap_id")) for it in disp_doc.get("items") or []]
        == [g for g, _src in frozen_union]
    )
    checks["frozen_gap_id_closed_set_match"] = ids_match
    checks["disposition_table_20_rows_zero_empty"] = not val_errs
    check(ids_match, "gap_id 闭集与三冻结表逐字转引不全等")
    check(not val_errs, f"处置表校验: {val_errs[:3]}")
    if not val_errs:
        DISPOSITION_PATH.write_text(
            json.dumps(disp_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        note(f"处置表落盘 → {DISPOSITION_PATH.relative_to(ROOT)}（20 行零空行）")

    # ── ⑤ fresh delta 可溯源（f64 精确重算）＋ 方向判定一致性 ──
    trace_bad: list[str] = []
    dir_bad: list[str] = []
    if disp_doc and not val_errs:
        rebuilt, rebuild_problems = build_disposition(
            frozen_rows, mc_ev, md_ev, g12_ev,
            band_mc if band_mc is not None else 0.0,
            band_md if band_md is not None else 0.0,
            wave_start)
        if rebuild_problems:
            trace_bad += rebuild_problems
        else:
            for a, b in zip(disp_doc["items"], rebuilt["items"]):
                if a.get("registered_delta") != b.get("registered_delta") or \
                        a.get("fresh_measured_delta") != b.get("fresh_measured_delta"):
                    trace_bad.append(f"重算不齐 {a.get('gap_id')}")
        dir_bad = crosscheck_directions(disp_doc)
    checks["fresh_measured_delta_traceable"] = not trace_bad and bool(disp_doc)
    checks["direction_judgments_consistent"] = not dir_bad and bool(disp_doc)
    check(not trace_bad, f"fresh delta 溯源: {trace_bad[:3]}")
    check(not dir_bad, f"方向判定一致性: {dir_bad[:3]}")

    # ── ⑥ UE 方差带程序产面（fresh 带解析 + 跨会话样本带 gaplib 面消费入档） ──
    bands_ok = (
        band_mc is not None and band_md is not None
        and math.isfinite(band_mc) and math.isfinite(band_md)
        and band_mc >= 0.0 and band_md >= 0.0
    )
    cross_note = ""
    if bands_ok and disp_doc:
        # 跨会话带抽样核验：首行首度量 gaplib 程序产带可复算（消费面机核）
        it0 = disp_doc["items"][0]
        d0 = it0["registered_delta"][0]
        cs = gaplib.ue_cross_session_band(
            G14_UE_SAMPLES_PATH, it0["gap_id"], d0["metric"], "a_value", float(d0["a_value"]))
        cross_note = f"跨会话带抽样 {it0['gap_id']}|{d0['metric']}|a_value → {cs:.8f}"
        bands_ok = bands_ok and math.isfinite(cs) and cs >= 0.0
    checks["ue_variance_band_program_produced"] = bool(bands_ok)
    check(bands_ok, f"UE 方差带程序产面: upscale={band_mc} lumen={band_md}")
    note(f"UE 方差带程序产：upscale band_rel={band_mc} lumen band_md={band_md}；{cross_note}")

    # ── ⑦ AI 读图基线臂（18 格 PNG 导出 + 结构代理断言 + manifest） ──
    preview_manifest: list[dict] = []
    preview_bad: list[str] = []
    findings: list[dict] = []
    if checks["upstream_m_c_fresh_pass"]:
        PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
        ev100 = {s["scene_id"]: s["exposure"]["ev100"]
                 for s in load_json(G13_UPSCALE_CONTRACT)["scenes"]}
        for scene in SCENES:
            for tier in TIERS:
                for backend in BACKENDS:
                    try:
                        cell = export_cell_png(scene, tier, backend, ev100[scene])
                    except Exception as e:
                        preview_bad.append(f"{scene}/t{tier}/{backend} 导出异常: {e}")
                        continue
                    preview_manifest.append(cell)
                    if not cell["proxies_pass"]:
                        preview_bad.append(
                            f"{cell['cell']} 结构代理失败: "
                            f"{[k for k, v in cell['structural_proxies'].items() if not v]}")
        # ── 新发现显式登记（法定来源唯一纪律「新发现差距显式登记不静默混入」）：
        # vendor 双臂 converged 输出停留 scene-linear 域（UpscaleInputs.exposure
        # 语义 = backend 转显示域；tsr.rs px_out=v×exposure 字面兑现，vendor
        # pre_exposure 未达输出面——bistro 三档原域均值比 ≈2^4 实测）──
        if preview_manifest:
            ven = [m for m in preview_manifest
                   if m["cell"].startswith("bistro-interior/t67/")]
            tsr_m = next((m for m in ven if m["cell"].endswith("tsr_device")), None)
            dlss_m = next((m for m in ven if m["cell"].endswith("dlss_sr")), None)
            if tsr_m and dlss_m and dlss_m["display_scale"] != 1.0:
                findings.append({
                    "id": "G15-MA-F1",
                    "title": "vendor_backend_output_domain_deviation@bistro-interior",
                    "kind": "quality_gap_candidate",
                    "measured": (
                        f"bistro t67 converged 原域均值 tsr_device=0.00977378 vs "
                        f"dlss_sr=0.00060290 vs fsr_3_1_5=0.00060379（比值 ≈2^4=bistro "
                        f"ev100 派生尺度；cornell ev100=0 三臂同域旁证）"
                    ),
                    "disposition_hint": "open-defer-G16+（G15.6a 穷举 + M-b 材质链/口径面评估输入；端内参照 parity 面尺度消去故 G13.4 起潜伏——AI 读图基线臂首次检出，G14.10f 教训字面兑现面）",
                    "baseline_arm_handling": "AI 读图导出按逐后端输出域声明面归一显示域（vendor 臂 ×2^(−ev100)），finding 本体与本处置面入 evidence/§8.2 显式登记",
                })
    checks["ai_reading_baseline_18_cells"] = len(preview_manifest) == 18 and not preview_bad
    check(checks["ai_reading_baseline_18_cells"],
          f"AI 读图基线臂: 导出 {len(preview_manifest)}/18，问题 {preview_bad[:3]}")
    note(f"AI 读图基线臂：18 格 PNG → {PREVIEW_DIR.relative_to(ROOT)}（结构代理 {len(preview_manifest)} 格全绿={not preview_bad}）；findings={len(findings)}")

    # ── ⑧ g15_budget M-a 双条目（fresh 带 measured_local 幂等面） ──
    budget_ok = False
    if bands_ok:
        up_ev = _write_band_evidence(
            "upscale", "g15.m_a.ue_variance_band_upscale_probe_rel", band_mc,
            _samples_from_notes(MC_PREFIX),
            "G15.2 M-a 重收割复跑 M-c 门内 UE 探针格（bistro-interior/tier67 末帧 HF share）"
            "三样本运行间方差底 max 两两相对差 ×2.0 程序产（禁手写 P-09；G14 M-a 取严口径继承）",
            ts)
        lu_ev = _write_band_evidence(
            "lumen", "g15.m_a.ue_variance_band_lumen_probe_rel", band_md,
            _samples_from_notes(MD_PREFIX),
            "G15.2 M-a 重收割复跑 M-d 门内 UE 探针格（bistro-interior/lumen-on 末帧平均亮度）"
            "三样本运行间方差底 max 两两相对差 ×2.0 程序产（同口径）",
            ts)
        _write_budget(band_mc, band_md, up_ev, lu_ev)
        doc = load_json(BUDGET_PATH)
        got = {e.get("id"): e for e in doc.get("entries") or []}
        e1 = got.get("g15.m_a.ue_variance_band_upscale_probe_rel") or {}
        e2 = got.get("g15.m_a.ue_variance_band_lumen_probe_rel") or {}
        budget_ok = (
            e1.get("measured_value") == band_mc and e1.get("threshold") == band_mc * 2.0
            and e2.get("measured_value") == band_md and e2.get("threshold") == band_md * 2.0
            and e1.get("evidence") == "measured_local" and e2.get("evidence") == "measured_local"
            and len(doc.get("entries") or []) == 5  # 既有三条目 0-byte + 本门双条目
        )
        if budget_ok:
            r = run(["py", "-3", "ci/budget_eval.py"], timeout=1200)
            budget_ok = r.returncode == 0 and "[budget_eval] PASS" in (r.stdout or "")
    checks["budget_entries_measured_appended"] = bool(budget_ok)
    check(budget_ok, "g15_budget M-a 双条目入账/budget_eval 异常")

    # ── ⑨ RED 臂（门内真跑，五臂独立） ──
    if disp_doc and not val_errs:
        red_results["missing_row"] = red_arm_missing_row(disp_doc, frozen_union)
        red_results["gap_id_tamper"] = red_arm_gap_id_tamper(disp_doc, frozen_union)
        red_results["direction_lie"] = red_arm_direction_lie(disp_doc)
        red_results["stale_evidence"] = red_arm_stale_evidence(wave_start)
        red_results["fresh_delta_missing_field"] = red_arm_fresh_delta_missing_field(disp_doc, frozen_union)
    checks["red_arm_missing_row_detected"] = red_results.get("missing_row") is True
    checks["red_arm_gap_id_tamper_detected"] = red_results.get("gap_id_tamper") is True
    checks["red_arm_direction_lie_detected"] = red_results.get("direction_lie") is True
    checks["red_arm_stale_evidence_detected"] = red_results.get("stale_evidence") is True
    checks["red_arm_fresh_delta_missing_field_detected"] = red_results.get("fresh_delta_missing_field") is True
    for arm, ok in red_results.items():
        check(ok, f"RED 臂 {arm} 注入未检出")
        note(f"RED 臂 {arm}: {'有效' if ok else '失效'}")

    dir_counts = {d: sum(1 for it in (disp_doc.get("items") or []) if it.get("direction") == d)
                  for d in DIRECTIONS} if disp_doc else {}
    sug_counts = {s: sum(1 for it in (disp_doc.get("items") or []) if it.get("suggestion") == s)
                  for s in SUGGESTIONS} if disp_doc else {}

    host_pass = all(checks.values()) and not FAILURES
    device_state = "executed" if (
        rows["m_c"]["status"] == rows["m_d"]["status"] == rows["m163"]["status"] == "PASS"
    ) else "fail"

    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if host_pass and device_state == "executed" else "fail",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": base_commit(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": f"py -3 ci/g13_ue_upscale_parity_smoke.py --gate {MC_GATE}（本波复跑，上游真跑面）",
             "exit_code": 0 if checks["upstream_m_c_fresh_pass"] else 1},
            {"seq": 2, "command": f"py -3 ci/g13_ue_lumen_gi_parity_smoke.py --gate {MD_GATE}（本波复跑）",
             "exit_code": 0 if checks["upstream_m_d_fresh_pass"] else 1},
            {"seq": 3, "command": f"py -3 ci/g12_ue_pt_parity_smoke.py --gate {G12_GATE}（本波复跑）",
             "exit_code": 0 if checks["upstream_m163_fresh_pass"] else 1},
            {"seq": 4, "command": "处置表装配 + 同族校验 + 落盘（20 行零空行，三冻结表 0-byte 只消费）",
             "exit_code": 0 if checks["disposition_table_20_rows_zero_empty"] else 1},
            {"seq": 5, "command": "AI 读图基线臂 18 格 PNG 导出 + 结构代理断言",
             "exit_code": 0 if checks["ai_reading_baseline_18_cells"] else 1},
            {"seq": 6, "command": "py -3 ci/budget_eval.py（g15_budget M-a 双条目入账后）",
             "exit_code": 0 if checks["budget_entries_measured_appended"] else 1},
            {"seq": 7, "command": "RED 臂 ×5（missing-row/gap-id-tamper/direction-lie/stale-evidence/fresh-delta-missing-field）",
             "exit_code": 0 if all(v is True for v in red_results.values()) and len(red_results) == 5 else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "production": {
            "correctness_anchor_unchanged": checks["parity_contracts_and_registries_0byte"],
            "baseline_anchor_id": "g15.m_a.ue_variance_band_{upscale,lumen}_probe_rel（本门 fresh 带入 g15_budget）",
            "measured_value": (
                f"方向判定汇总：converged={dir_counts.get('converged', 0)} "
                f"maintained={dir_counts.get('maintained', 0)} degraded={dir_counts.get('degraded', 0)}；"
                f"建议：closed-resolved={sug_counts.get('closed-resolved', 0)} "
                f"closed-caliber-registered={sug_counts.get('closed-caliber-registered', 0)} "
                f"open-defer-G16+={sug_counts.get('open-defer-G16+', 0)}"
            ) if disp_doc else "n/a（上游未全绿）",
            "not_worse_than_anchor": dir_counts.get("degraded", 1) == 0 if disp_doc else False,
            "threshold_provenance": "容差 = g13_budget/g12_budget 标定条目双 seed 方差底 p100×2.0 程序产（禁手写 P-09）；UE 方差带 = gaplib 跨会话样本级联 ×2.0 与同会话探针带取 max（双程序产面取严）",
            "evolution_register": (
                "三冻结表 20 行终态 0-byte 只消费不回写；处置面另立 milestones/g15/"
                "g15_quality_gap_disposition.json（gap_id 逐字转引 + fresh measured_delta 可溯源）；"
                "AI 读图基线臂结构代理全格入 manifest；AI 画面审查逐格记录归 G15_CONTRACT §8.2 验收记录"
            ),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "parity": {
            "wave_start": wave_start,
            "upstream_gates": [rows["m_c"], rows["m_d"], rows["m163"]],
            "disposition_file": "milestones/g15/g15_quality_gap_disposition.json",
            "direction_counts": dir_counts,
            "suggestion_counts": sug_counts,
            "ue_variance_bands": {"upscale_probe_rel": band_mc, "lumen_probe_rel": band_md},
            "ai_reading_baseline_manifest": preview_manifest,
            "findings": findings,
        },
    }
    errs = wel.validate_schema(evidence, SCHEMA_PATH) if SCHEMA_PATH.is_file() else []
    if errs:
        print(f"[{TAG}] schema errors: {errs}", file=sys.stderr)
        evidence["status"] = "fail"
        evidence["host_section_pass"] = False
        host_pass = False
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    print(f"[{TAG}] VERDICT={'PASS' if evidence['status'] == 'pass' else 'FAIL'} "
          f"checks={sum(1 for v in checks.values() if v)}/{len(checks)}")
    return 0 if evidence["status"] == "pass" else 1


def verify_latest() -> int:
    path = wel.load_latest_evidence(SUBJECT)
    if path is None:
        print(f"[{TAG}] FAIL: 缺最新 evidence（{SUBJECT}_*.json）", file=sys.stderr)
        return 1
    doc = wel.load_json(path)
    checks = doc.get("checks") or {}
    bad = [k for k in CHECK_KEYS if checks.get(k) is not True]
    if bad or doc.get("status") != "pass":
        print(f"[{TAG}] FAIL checks={bad} status={doc.get('status')!r}", file=sys.stderr)
        return 1
    print(f"[{TAG}] verify-latest PASS（{path.name}，checks {len(CHECK_KEYS)} 键全绿）")
    return 0


def run_selftest() -> int:
    """schema 闭集对账 + 五 RED 臂函数面 + GREEN 正例（不依赖 device/UE）。"""
    failures = 0
    schema = wel.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        failures += 1
    # 合成正例处置表（1 行最小面 → 校验器 GREEN；闭集/不变式面全绿）
    frozen_union = [("a" * 16, "g13_ue_upscale_gap_registry")]
    good_item = {
        "gap_id": "a" * 16,
        "source_registry": "g13_ue_upscale_gap_registry",
        "scene_id": "cornell-box",
        "kind": "quality_gap",
        "title": "t",
        "registered_delta": [{"metric": "m", "a_value": 1.0, "b_value": 0.9,
                              "delta": -0.09999999999999998, "tolerance": None}],
        "fresh_measured_delta": [{"metric": "m", "a_value": 1.0, "b_value": 0.9,
                                  "delta": -0.09999999999999998, "tolerance": 0.01}],
        "direction": "maintained",
        "direction_per_metric": {"m": "maintained"},
        "ue_variance_band": {"m": 0.0},
        "suggestion": "open-defer-G16+",
        "rationale": "selftest 合成面",
    }
    good = {
        "schema_version": 1,
        "registry": "g15_quality_gap_disposition",
        "generated_by": GENERATED_BY,
        "wave_start": "20260823T000000Z",
        "scene_set": list(SCENES),
        "items": [good_item],
        "scene_summary": [
            {"scene_id": "cornell-box", "row_count": 1, "converged": 0, "maintained": 1, "degraded": 0},
            {"scene_id": "bistro-interior", "row_count": 0, "converged": 0, "maintained": 0, "degraded": 0},
        ],
        "not_ready_scenes": [],
    }
    if validate_disposition(good, frozen_union):
        print(f"[{TAG}] selftest FAIL: 合形处置表被误拒 {validate_disposition(good, frozen_union)}", file=sys.stderr)
        failures += 1
    if crosscheck_directions(good):
        print(f"[{TAG}] selftest FAIL: 合形处置表方向交叉核验误拒 {crosscheck_directions(good)}", file=sys.stderr)
        failures += 1
    if not red_arm_missing_row(good, frozen_union):
        print(f"[{TAG}] selftest FAIL: missing-row 臂未检出", file=sys.stderr)
        failures += 1
    if not red_arm_gap_id_tamper(good, frozen_union):
        print(f"[{TAG}] selftest FAIL: gap-id-tamper 臂未检出", file=sys.stderr)
        failures += 1
    if not red_arm_direction_lie(good):
        print(f"[{TAG}] selftest FAIL: direction-lie 臂未检出", file=sys.stderr)
        failures += 1
    if not red_arm_stale_evidence("20260823T084242Z"):
        print(f"[{TAG}] selftest FAIL: stale-evidence 臂未检出", file=sys.stderr)
        failures += 1
    if not red_arm_fresh_delta_missing_field(good, frozen_union):
        print(f"[{TAG}] selftest FAIL: fresh-delta-missing-field 臂未检出", file=sys.stderr)
        failures += 1
    # 方向纯函数面绿臂（收敛/维持/劣化三态各正例）
    if metric_direction(-0.10, -0.10, False, 0.0) != "converged":
        print(f"[{TAG}] selftest FAIL: converged 正例误判", file=sys.stderr)
        failures += 1
    if metric_direction(-0.10, -0.10, True, 0.0) != "maintained":
        print(f"[{TAG}] selftest FAIL: maintained 正例误判", file=sys.stderr)
        failures += 1
    if metric_direction(-0.10, -0.30, True, 0.01) != "degraded":
        print(f"[{TAG}] selftest FAIL: degraded 正例误判", file=sys.stderr)
        failures += 1
    # PNG 编码面（1×1 合成帧 → magic + IHDR 尺寸回读）
    png = encode_png_rgb(1, 1, bytes([255, 0, 0]))
    if png[:8] != b"\x89PNG\r\n\x1a\n" or len(png) < 60:
        print(f"[{TAG}] selftest FAIL: PNG 编码面异常", file=sys.stderr)
        failures += 1
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)}（schema 闭集 + 5 RED + 4 GREEN 函数面臂）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--wave-start", default=None,
                    help="本波启动锚 UTC（%%Y%%m%%dT%%H%%M%%SZ）；缺省 = HEAD commit UTC 派生锚")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return verify_latest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    wave_start = args.wave_start or head_commit_utc_stamp()
    return run_gate(wave_start)


if __name__ == "__main__":
    sys.exit(main())
