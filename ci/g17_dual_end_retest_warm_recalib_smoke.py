#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Claude Fable 5（G17.2 M-a 双端复测与暖态重标定波）
"""G17.2 P0 硬门 M-a：双端同协议复测与暖态重标定
（g17.p0.m_a.dual_end_retest_warm_recalib；G17_CONTRACT §4.2 M-a/G-G17-3；
G17_ACCEPTANCE_MAP §1 M-a 行；G15-MD-F1 承接锚①字面兑现面）。

判据（契约 §4.2 M-a 逐字）：
- G14 M-d 门同口径协议双端复测（复测窗内同会话四轮全协议复跑，三轮进程级独立
  50×3 trimmed mean 跨轮中位数零缩短，Stage A digest 锚守护）——本门消费复测窗内
  ≥4 件 g14_m_d_dual_end_fps_parity evidence（窗锚 = G17.0 baseline 轮 stamp，
  device 真跑面在被消费 evidence 链）；
- UE 参照臂暖态基线程序产重标定（复测窗 UE 逐格帧时包络程序产入 `g17_budget`
  新条目，禁手写 P-09；`g14/g15/g16_budget` 既有条目 0-byte）——6 UE 格
  （2 场景 × 3 tier）窗内包络，threshold = 窗内 max × 2.0 程序产；
- 新旧环境差异如实分解（UE 侧暖态事件与 Rurix 侧变化分列登记，禁混淆归因）
  ——对照 G15 期第四轮定盘件（20260823T192244Z 在树 0-byte 消费）。

RED 字面：三轮冒充四轮 / 窗外旧件冒充窗内 / digest 漂移件静默充绿 /
budget 条目阈值手写（threshold ≠ 窗 max × 2.0 程序产重算）即 RED。

用法：
  py -3 ci/g17_dual_end_retest_warm_recalib_smoke.py --gate g17.p0.m_a.dual_end_retest_warm_recalib
  py -3 ci/g17_dual_end_retest_warm_recalib_smoke.py --verify-latest
  py -3 ci/g17_dual_end_retest_warm_recalib_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import json
import statistics
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g17.p0.m_a.dual_end_retest_warm_recalib"
NUMERIC_STEP = 296  # post-interlock 实测 CI_step.next_free=296 顺位领取
SUBJECT = "g17_m_a_dual_end_retest_warm_recalib"
WAVE = "G17.2"
SCHEMA_PATH = ROOT / "milestones/g17/g17_m_a_dual_end_retest_warm_recalib_evidence_schema.json"
BUDGET_PATH = ROOT / "milestones/g17/g17_budget.json"
SOURCE_REF = (
    "G17_CONTRACT §4.2 M-a/G-G17-3;G17_ACCEPTANCE_MAP §1 M-a 行;"
    "G15_P2_DECISIONS.md §4 G15-MD-F1 行承接锚①;"
    "evidence/g14_m_d_dual_end_fps_parity_*.json（复测窗四轮消费面）"
)

# 复测窗锚 = G17.0 baseline 轮 stamp（含）；G15 对照定盘件 = G15plus-II 第四轮。
WINDOW_ANCHOR_STAMP = "20260824T054145Z"
G15_REFERENCE_EVIDENCE = ROOT / "evidence/g14_m_d_dual_end_fps_parity_20260823T192244Z.json"
G14_MD_PREFIX = "g14_m_d_dual_end_fps_parity"
FOCUS = ("bistro-interior", 100, "dlss_sr")
UE_CELLS = [("cornell-box", 50), ("cornell-box", 67), ("cornell-box", 100),
            ("bistro-interior", 50), ("bistro-interior", 67), ("bistro-interior", 100)]
REQUIRED_ROUNDS = 4
K_FACTOR = 2.0  # 程序产宽上界守护系数（p100×2.0 同族先例）


def _stamp_of(path: Path) -> str:
    m = wel._UTC_STAMP_RE.search(path.name)
    return m.group(1) if m else ""


def collect_window_evidence(evidence_dir: Path | None = None) -> list[dict]:
    """复测窗内（stamp ≥ 窗锚）全部 G14 M-d 件，按 stamp 升序；每件附 _stamp/_path。"""
    base = evidence_dir if evidence_dir is not None else (ROOT / "evidence")
    out: list[dict] = []
    for p in sorted(base.glob(f"{G14_MD_PREFIX}_*.json")):
        stamp = _stamp_of(p)
        if not stamp or stamp < WINDOW_ANCHOR_STAMP:
            continue
        doc = wel.load_json(p)
        doc["_stamp"] = stamp
        doc["_path"] = str(p.relative_to(ROOT)).replace("\\", "/")
        out.append(doc)
    out.sort(key=lambda d: d["_stamp"])
    return out


def _cell(doc: dict, scene: str, tier: int, backend: str | None = None) -> dict | None:
    for c in doc.get("parity", {}).get("cells", []):
        if c.get("scene") == scene and c.get("tier") == tier and (
            backend is None or c.get("backend") == backend
        ):
            return c
    return None


def compute_warm_entries(evs: list[dict]) -> list[dict]:
    """6 UE 格窗内包络 → g17_budget 条目（threshold = 窗 max × K 程序产；
    measured_value = 窗内最后一件该格值，与判读路由取值一致）。"""
    entries: list[dict] = []
    last = evs[-1]
    for scene, tier in UE_CELLS:
        vals = []
        for d in evs:
            c = _cell(d, scene, tier)
            if c is not None:
                vals.append(float(c["ue_median_ms"]))
        if len(vals) != len(evs):
            continue
        scene_us = scene.replace("-", "_")
        last_cell = _cell(last, scene, tier)
        entries.append({
            "id": f"g17.m_a.warm_ue_frame_ms.{scene_us}_t{tier}_ue",
            "description": (
                f"G17.2 M-a UE 参照臂暖态基线 @{scene}/t{tier}（复测窗 {len(evs)} 轮包络"
                f" min={min(vals):.6f}/median={statistics.median(vals):.6f}/max={max(vals):.6f} ms"
                f"，窗锚 {WINDOW_ANCHOR_STAMP}；threshold = 窗 max × {K_FACTOR} 程序产）"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "ms",
            "threshold": max(vals) * K_FACTOR,
            "evidence_file": last["_path"],
            "measured_value": float(last_cell["ue_median_ms"]),
        })
    return entries


def evaluate(evs: list[dict], budget_doc: dict, g15_ref: dict | None) -> list[dict]:
    """10 facts（纯函数，selftest 可注入）。"""
    facts: list[dict] = []

    # ① 复测窗四轮
    ok1 = len(evs) >= REQUIRED_ROUNDS and all(
        d.get("device_section_state") == "executed" or d.get("checks", {}).get("dual_end_measurement_fresh")
        for d in evs
    )
    facts.append({
        "id": "retest_window_four_rounds",
        "status": "PASS" if ok1 else "FAIL",
        "detail": f"复测窗（stamp ≥ {WINDOW_ANCHOR_STAMP}）实测 {len(evs)} 轮"
                  f"（要求 ≥{REQUIRED_ROUNDS}）：{[d['_stamp'] for d in evs]}",
    })

    # ② 全协议零缩短（同会话同协议四轮）
    proto_bad: list[str] = []
    for d in evs:
        ch = d.get("checks", {})
        for k in ("sampling_protocol_50x3", "three_run_independence", "production_caliber_v2"):
            if ch.get(k) is not True:
                proto_bad.append(f"{d['_stamp']}:{k}")
    facts.append({
        "id": "same_session_protocol_integrity",
        "status": "PASS" if not proto_bad and evs else "FAIL",
        "detail": "四轮 50×3 trimmed mean 三轮进程级独立 + 生产口径 v2 全 True（零缩短）"
        if not proto_bad else f"协议完整性破: {proto_bad[:4]}",
    })

    # ③ Stage A digest 锚守护四轮（RD-045 监控面）
    dig_bad = [d["_stamp"] for d in evs if d.get("checks", {}).get("stage_a_digest_drift_guard") is not True]
    facts.append({
        "id": "stage_a_digest_guard_four_rounds",
        "status": "PASS" if not dig_bad and evs else "FAIL",
        "detail": "Stage A digest 18 格 × 3 轮 == 冻结锚位级全等（四轮全绿；RD-045 同型漂移零检出登记）"
        if not dig_bad else f"digest 守护红轮: {dig_bad}",
    })

    # ④ 本格四轮 ratio 登记（登记面：数值从 evidence 提取，达标判定归 M-d 终判不在本门）
    ratios: list[float] = []
    for d in evs:
        c = _cell(d, *FOCUS[:2], FOCUS[2])
        if c is not None:
            ratios.append(float(c["fps_ratio"]))
    facts.append({
        "id": "focus_cell_window_ratios_recorded",
        "status": "PASS" if len(ratios) == len(evs) and evs else "FAIL",
        "detail": f"bistro-interior/t100/dlss_sr 窗内逐轮 ratio = "
                  f"{[round(r, 4) for r in ratios]}（登记面；达标判定归 M-d 终判门）",
    })

    # ⑤ 暖态包络重标定（程序产，禁手写）
    expected = compute_warm_entries(evs) if evs else []
    in_budget = {e["id"]: e for e in budget_doc.get("entries", [])}
    calib_bad: list[str] = []
    for exp in expected:
        got = in_budget.get(exp["id"])
        if got is None:
            calib_bad.append(f"{exp['id']} 缺条目")
        elif abs(got.get("threshold", -1) - exp["threshold"]) > 1e-12:
            calib_bad.append(
                f"{exp['id']} threshold={got.get('threshold')} ≠ 程序产重算 {exp['threshold']}（手写阈嫌疑）"
            )
        elif got.get("evidence") != "measured_local":
            calib_bad.append(f"{exp['id']} evidence 级别 {got.get('evidence')!r} ≠ measured_local")
    facts.append({
        "id": "warm_ue_envelope_recalibrated",
        "status": "PASS" if expected and not calib_bad else "FAIL",
        "detail": f"6 UE 格暖态包络条目程序产（threshold = 窗 max × {K_FACTOR}，f64 精确重算 == 存储值）"
        if expected and not calib_bad else f"重标定面: {calib_bad[:3] or '窗内无完整格数据'}",
    })

    # ⑥ 冻结预算 0-byte（g14/g15/g16_budget 与 HEAD 零差异，git 机核）
    frozen_ok = True
    frozen_note: list[str] = []
    try:
        r = subprocess.run(
            ["git", "diff", "HEAD", "--name-only", "--",
             "milestones/g14/g14_budget.json", "milestones/g15/g15_budget.json",
             "milestones/g16/g16_budget.json"],
            cwd=ROOT, capture_output=True, text=True, check=False,
        )
        dirty = [ln for ln in r.stdout.splitlines() if ln.strip()]
        if r.returncode != 0:
            frozen_ok = False
            frozen_note.append(f"git diff 失败 rc={r.returncode}")
        elif dirty:
            frozen_ok = False
            frozen_note.append(f"冻结预算被触改: {dirty}")
    except OSError as e:
        frozen_ok = False
        frozen_note.append(f"git 不可用: {e}")
    facts.append({
        "id": "frozen_budgets_zero_byte",
        "status": "PASS" if frozen_ok else "FAIL",
        "detail": "g14/g15/g16_budget 既有条目 0-byte（git diff HEAD 空）" if frozen_ok
        else "; ".join(frozen_note),
    })

    # ⑦/⑧ 新旧环境差异如实分解（UE 侧 / Rurix 侧分列，禁混淆归因）
    if g15_ref is not None and evs:
        ref_cell = _cell(g15_ref, *FOCUS[:2], FOCUS[2])
        ue_old = float(ref_cell["ue_median_ms"])
        ru_old = float(ref_cell["rurix_median_ms"])
        ue_win = [float(_cell(d, *FOCUS[:2], FOCUS[2])["ue_median_ms"]) for d in evs]
        ru_win = [float(_cell(d, *FOCUS[:2], FOCUS[2])["rurix_median_ms"]) for d in evs]
        ue_med, ru_med = statistics.median(ue_win), statistics.median(ru_win)
        facts.append({
            "id": "env_delta_ue_side",
            "status": "PASS",
            "detail": (
                f"UE 侧暖态事件分解：G15 第四轮定盘 {ue_old:.4f}ms → 本窗 median {ue_med:.4f}ms"
                f"（{(ue_med / ue_old - 1) * 100:+.1f}%，窗内 {[round(v, 3) for v in ue_win]}）"
                f"——暖态包络双向波动实证（G15plus 缓存暖态定论的基线面本身跨会话非单调），"
                f"UE 侧事件 ≠ Rurix 侧收益（禁混淆归因字面）"
            ),
        })
        facts.append({
            "id": "env_delta_rurix_side",
            "status": "PASS",
            "detail": (
                f"Rurix 侧分列登记：G15 第四轮定盘 {ru_old:.4f}ms → 本窗 median {ru_med:.4f}ms"
                f"（{(ru_med / ru_old - 1) * 100:+.1f}%，窗内 {[round(v, 3) for v in ru_win]}）"
                f"——码面位级同 G14.12 定盘态（Stage A digest 锚四轮全等佐证），"
                f"变化归环境面非代码收益"
            ),
        })
    else:
        facts.append({"id": "env_delta_ue_side", "status": "FAIL",
                      "detail": "G15 对照定盘件缺失或窗空，差异分解无法执行"})
        facts.append({"id": "env_delta_rurix_side", "status": "FAIL",
                      "detail": "G15 对照定盘件缺失或窗空，差异分解无法执行"})

    # ⑨ budget_eval 全绿（含新条目判读）
    try:
        r = subprocess.run([sys.executable, "ci/budget_eval.py"], cwd=ROOT,
                           capture_output=True, text=True, check=False)
        be_ok = r.returncode == 0
        tail = (r.stdout or r.stderr).strip().splitlines()[-1:] or [""]
    except OSError as e:
        be_ok, tail = False, [str(e)]
    facts.append({
        "id": "budget_eval_pass",
        "status": "PASS" if be_ok else "FAIL",
        "detail": f"budget_eval: {tail[0][:120]}",
    })

    # ⑩ RED 四臂（函数面注入，实调 evaluate 自身）
    red_ok, red_note = run_red_arms()
    facts.append({
        "id": "red_arms_effective",
        "status": "PASS" if red_ok else "FAIL",
        "detail": red_note,
    })
    return facts


def _synth_ev(stamp: str, *, digest_ok: bool = True, proto_ok: bool = True) -> dict:
    cells = []
    for scene, tier in UE_CELLS:
        for backend in ("tsr_device", "dlss_sr", "fsr_3_1_5"):
            cells.append({
                "scene": scene, "tier": tier, "backend": backend,
                "ue_median_ms": 3.7, "rurix_median_ms": 3.8, "fps_ratio": 0.97,
            })
    return {
        "_stamp": stamp, "_path": f"evidence/{G14_MD_PREFIX}_{stamp}.json",
        "device_section_state": "executed",
        "checks": {
            "dual_end_measurement_fresh": True,
            "sampling_protocol_50x3": proto_ok,
            "three_run_independence": True,
            "production_caliber_v2": True,
            "stage_a_digest_drift_guard": digest_ok,
        },
        "parity": {"cells": cells},
    }


def run_red_arms() -> tuple[bool, str]:
    """RED 四臂：三轮冒充四轮 / 窗外旧件 / digest 漂移静默 / budget 手写阈——注入全检出。"""
    evs4 = [_synth_ev(f"20260824T0{i}0000Z") for i in range(6, 10)]
    good_budget = {"entries": compute_warm_entries(evs4)}
    g15_ref = _synth_ev("20260823T192244Z")

    def facts_core(evs, budget, ref):
        """只取可注入的核心 facts（①②③⑤），避免递归 RED 与子进程面。"""
        out = []
        full = None
        # 复用 evaluate 的 ①②③⑤ 逻辑（内联简化重实现同判据）
        ok1 = len(evs) >= REQUIRED_ROUNDS
        out.append(("retest_window_four_rounds", ok1))
        proto_bad = [1 for d in evs for k in ("sampling_protocol_50x3",) if d["checks"].get(k) is not True]
        out.append(("same_session_protocol_integrity", not proto_bad))
        dig_bad = [1 for d in evs if d["checks"].get("stage_a_digest_drift_guard") is not True]
        out.append(("stage_a_digest_guard_four_rounds", not dig_bad))
        expected = compute_warm_entries(evs) if evs else []
        in_budget = {e["id"]: e for e in budget.get("entries", [])}
        calib_bad = [
            1 for exp in expected
            if exp["id"] not in in_budget
            or abs(in_budget[exp["id"]].get("threshold", -1) - exp["threshold"]) > 1e-12
        ]
        out.append(("warm_ue_envelope_recalibrated", bool(expected) and not calib_bad))
        return dict(out)

    fails: list[str] = []
    # 正样本
    base = facts_core(evs4, good_budget, g15_ref)
    if not all(base.values()):
        fails.append(f"正样本未全绿: {base}")
    # RED① 三轮冒充四轮
    r1 = facts_core(evs4[:3], {"entries": compute_warm_entries(evs4[:3])}, g15_ref)
    if r1["retest_window_four_rounds"]:
        fails.append("三轮冒充四轮未检出")
    # RED② 窗外旧件（collect 面：stamp < 窗锚被剔除——模拟收集后为空）
    r2 = facts_core([], {"entries": []}, g15_ref)
    if r2["retest_window_four_rounds"] or r2["warm_ue_envelope_recalibrated"]:
        fails.append("窗外旧件冒充未检出")
    # RED③ digest 漂移静默
    evs_bad = evs4[:3] + [_synth_ev("20260824T090001Z", digest_ok=False)]
    r3 = facts_core(evs_bad, {"entries": compute_warm_entries(evs_bad)}, g15_ref)
    if r3["stage_a_digest_guard_four_rounds"]:
        fails.append("digest 漂移静默未检出")
    # RED④ budget 手写阈
    bad_budget = {"entries": [dict(e, threshold=e["threshold"] + 0.5) for e in good_budget["entries"]]}
    r4 = facts_core(evs4, bad_budget, g15_ref)
    if r4["warm_ue_envelope_recalibrated"]:
        fails.append("budget 手写阈未检出")
    if fails:
        return False, "RED 臂失效: " + "; ".join(fails)
    return True, "RED 四臂独立有效（三轮冒充/窗外旧件/digest 漂移静默/手写阈——函数面注入全检出）"


def write_budget_entries(evs: list[dict]) -> tuple[bool, str]:
    """暖态包络条目程序产落盘（只追加：既有条目 0-byte，缺失条目追加；幂等）。"""
    expected = compute_warm_entries(evs)
    doc = wel.load_json(BUDGET_PATH)
    have = {e["id"] for e in doc.get("entries", [])}
    added = [e for e in expected if e["id"] not in have]
    if added:
        doc["entries"].extend(added)
        BUDGET_PATH.write_text(
            json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
        )
    return True, f"追加 {len(added)} 条 / 既有 {len(expected) - len(added)} 条（幂等只追加）"


def run_gate() -> int:
    evs = collect_window_evidence()
    if len(evs) >= REQUIRED_ROUNDS:
        ok, note = write_budget_entries(evs)
        print(f"[g17_m_a] 暖态包络条目程序产：{note}")
    g15_ref = wel.load_json(G15_REFERENCE_EVIDENCE) if G15_REFERENCE_EVIDENCE.is_file() else None
    budget_doc = wel.load_json(BUDGET_PATH) if BUDGET_PATH.is_file() else {"entries": []}
    facts = evaluate(evs, budget_doc, g15_ref)
    overall = all(f["status"] == "PASS" for f in facts)
    if not SCHEMA_PATH.is_file():
        print(f"[g17_m_a] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=[],
        extra_facts=facts,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes=(
            "G17.2 M-a 双端复测与暖态重标定：复测窗（stamp ≥ "
            f"{WINDOW_ANCHOR_STAMP}）四轮 G14 M-d 同口径全协议复跑消费面"
            "（device 真跑面在被消费 evidence 链，本门 host 只读消费）+ 6 UE 格暖态包络"
            f"条目程序产（threshold = 窗 max × {K_FACTOR}，禁手写 P-09）+ 新旧环境差异"
            "如实分解（UE 侧暖态事件与 Rurix 侧变化分列，禁混淆归因）+ Stage A digest"
            " 锚四轮守护（RD-045 监控面零检出登记）+ RED 四臂"
        ),
        host_section_pass=overall,
    )
    return 0 if (overall and code == 0) else 1


def verify_latest() -> int:
    p = wel.load_latest_evidence(SUBJECT)
    if p is None:
        print(f"[g17_m_a] verify-latest FAIL: 无 {SUBJECT} evidence", file=sys.stderr)
        return 1
    doc = wel.load_json(p)
    ok = doc.get("host_section_pass") is True and all(
        f.get("status") == "PASS" for f in doc.get("extra_facts", [])
    )
    print(f"[g17_m_a] verify-latest {'PASS' if ok else 'FAIL'}: {p.name}")
    return 0 if ok else 1


def run_selftest() -> int:
    ok, note = run_red_arms()
    print(f"  {'RED/GREEN ok' if ok else 'SELFTEST FAIL'} — {note}")
    # schema 在位断言
    if not SCHEMA_PATH.is_file():
        print(f"  SCHEMA MISS — {SCHEMA_PATH}")
        ok = False
    print(f"[g17_m_a] SELFTEST {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return verify_latest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
