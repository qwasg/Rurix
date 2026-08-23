#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.6a P2 穷举 + M-e 回归门 + soak 波）
"""G15.6a stabilization soak 聚合门 g15.wave.6a.soak（G15_CONTRACT G-G15-8；
G15_ACCEPTANCE_MAP §7；同构 ci/g14_stabilization_soak.py〔G14.5a〕先例）。

四腿：①全量回归（G15 5 P0 = M-a~M-e 逐门最新 evidence 核验——wel 口径 + 顶层
status=="pass" 字面〔缺字段即红〕；**M-d 诚实红门面特判**（G14 closeout 先例
同型字面）：G15 M-d evidence status=="fail" ∧ 绿键集全真（画质锚带复核/budget
零 estimated/RED 四臂）∧ 红键集全假闭集（复跑 fresh/逐格四面/G14.12 对照——
复跑件红面消费跳过如实红不充绿）∧ 消费的 G14 M-d 复跑件 checks 全绿 ∧
status=="fail" ∧ unmet_count == g14_fps_gap_registry 行数（1 行 gap_id
51a150cb4523e8b6）∧ G15-MD-F1 承接锚在案 = 未达标如实登记不充绿亦不充降级；
wave2/3/4 exit + wave6a decisions 最新 evidence 核验 + **wave5 红面特判同型**
（聚合 FAIL 红面 = M-d 行 FAIL 镜像 + facts ④⑤ 绿 + 红 facts ⊆ M-d 红同源
闭集——如实登记不充绿不充降级）；**soak 门 VERDICT 语义 = 回归腿零降级 +
红面登记完备**）+ ②画质/帧率链路连续复跑 soak ≥1800s（g14_3_pipeline_perf
--bench cornell-box/bistro-interior t67 × tsr_device/dlss_sr/fsr_3_1_5 六组合
轮转 canonical 协议 160 帧 warmup 10，receipt last_frame_digest ==
g14_3_stage_a_digest_anchor 冻结锚位级复现——跨迭代 digest 漂移即 M165 同型
监控面检出登记，RD-045 零检出预期；→ 登记表装配腿 = 最新 G14 M-d evidence
unmet_count 与在树 g14_fps_gap_registry.json 行集对账幂等复核；迭代计数/bench
帧数/装配计数非空零失败，sleep_seconds 恒 0，active_chain_seconds ≈seconds，
gate 外测墙钟交叉核验，谎报判红）+ ③budget_eval --strict 非空零 estimated/skip
+ ④纪律日期锚 + G5~G14 既有判据 0-byte fact（vs G15.0 ref f061487e committed
diff 闭集 ⊆ {g14_budget.json / g14_ue_variance_samples.json〔34f96ac3 归档授权
双面〕/ g14_fps_gap_registry.json〔G14 M-d 门产登记面〕}，工作树闭集空）。

诚实语义（沿 G14.5a/G13.5a/G12.7a/G11.7a/G10.8a/G9.8a 口径）：
- soak 墙钟=真实链路复跑实测（active_chain_seconds 逐迭代计时求和），迭代间
  零 sleep（sleep_seconds 恒 0）；gate 侧用外测墙钟交叉核验，谎报 seconds 判红。
- device 腿持 gpu_device_lock 串行；UE 帧不重复 benchmark（与 G13.5a/G14.5a 同
  口径——UE 臂真跑归 M-d 门本体复跑面）。
- 迭代计数/bench 帧数/登记表装配次数非空（空即红），failures 恒 0；
  evidence soak 块 seconds 与 iterations 双字段机器可核。

pr-smoke 默认 --verify-latest（秒级核最新 full-run evidence）；
本地/workflow_dispatch 用 --gate 产 full-run。

用法：
  py -3 ci/g15_stabilization_soak.py --gate g15.wave.6a.soak
  py -3 ci/g15_stabilization_soak.py --verify-latest
  py -3 ci/g15_stabilization_soak.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

sys.path.insert(0, str(ROOT / "ci"))

import g10_wave_exit_lib as wel  # noqa: E402
from g15_regression_drift_guard_smoke import (  # noqa: E402
    G15_MD_HONEST_GREEN,
    G15_MD_HONEST_RED,
    G15_MD_PREFIX,
    HONEST_RED_PREFIX,
    eval_honest_red_g14_md,
    git_closed_surface,
)
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g15.wave.6a.soak"
NUMERIC_STEP = 279  # 落盘前实测 registry/number_ledger.json CI_step.next_free=279 顺位领取
SUBJECT = "g15_stabilization_soak"
WAVE = "G15.6a"
TAG = "g15_6a_soak"
SOURCE_REF = (
    "G15_CONTRACT G-G15-8;G15_ACCEPTANCE_MAP §7;G15 5 P0（M-a~M-e）最新 evidence 核验（M-d 诚实红特判）+ 聚合/决策门核验（wave5 红面特判同型）;"
    "画质/帧率链路连续复跑 soak ≥1800s（g14_3_pipeline_perf --bench 轮转 digest==冻结锚，G14.5a 继承）;budget_eval --strict;G5~G14 0-byte"
)
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_stabilization_soak_evidence_schema.json"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
FPS_REGISTRY_PATH = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"
G15_CONTRACT = ROOT / "milestones" / "g15" / "G15_CONTRACT.md"
WAVE5_PREFIX = "g15_wave5_exit"
WAVE5_RED_FACTS = frozenset({
    "m_d_gate_pass_red_arms_effective",
    "rerun_real_and_18_cells_all_met",
    "digest_anchor_zero_drift",
    "legacy_closed_zero_src_change_and_namespace",
})
WAVE5_GREEN_FACTS = frozenset({
    "quality_anchor_band_recheck",
    "budgets_zero_estimated_and_eval_pass",
})

P0_GATES = [
    ("g15.p0.m_a.dual_end_quality_reharvest", "g15_m_a_dual_end_quality_reharvest"),
    ("g15.p0.m_b.gap_fix_closure_loop", "g15_m_b_gap_fix_closure_loop"),
    ("g15.p0.m_c.absolute_quality_final_review", "g15_m_c_absolute_quality_final_review"),
    ("g15.p0.m_d.perf_parity_zero_regression", G15_MD_PREFIX),
    ("g15.p0.m_e.regression_drift_guard", "g15_m_e_regression_drift_guard"),
]
AGG_GATES = [
    ("g15.wave.2.exit", "g15_wave2_exit"),
    ("g15.wave.3.exit", "g15_wave3_exit"),
    ("g15.wave.4.exit", "g15_wave4_exit"),
    ("g15.wave.5.exit", WAVE5_PREFIX),
    ("g15.wave.6a.decisions", "g15_p2_decisions"),
]
REGRESSION_GATES = P0_GATES + AGG_GATES

RURIX_BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"
SOAK_ROOT = Path(r"K:\rurix-ext\g15-frames\g15_6a_soak")
SOAK_COMBOS = [
    ("cornell-box", 67, "tsr_device"),
    ("cornell-box", 67, "dlss_sr"),
    ("cornell-box", 67, "fsr_3_1_5"),
    ("bistro-interior", 67, "tsr_device"),
    ("bistro-interior", 67, "dlss_sr"),
    ("bistro-interior", 67, "fsr_3_1_5"),
]
FRAME_COUNT = 160  # canonical 协议（warmup 10 + 150 稳态——冻结锚收割口径同字面）
MIN_SECONDS = 1800
MIN_ITERATIONS = 3

NOTES: list[str] = []


def _note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def base_commit() -> str:
    r = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True)
    return (r.stdout or "").strip() or "unknown"


# ---------------------------------------------------------------- ① 全量回归
def verify_g15_md_honest_red() -> tuple[bool, str]:
    """G15 M-d 门诚实红结构评定（soak 回归腿与 closeout 共用单一事实源）：
    status=="fail" ∧ 绿键集全真 ∧ 红键集全假闭集 ∧ 消费的 G14 M-d 复跑件
    诚实红链（eval_honest_red_g14_md）∧ G15-MD-F1 承接锚在案。"""
    path = wel.load_latest_evidence(G15_MD_PREFIX)
    if path is None:
        return False, "缺最新 g15_m_d evidence"
    doc = wel.load_json(path)
    checks = doc.get("checks") or {}
    green_bad = [k for k in G15_MD_HONEST_GREEN if checks.get(k) is not True]
    red_bad = [k for k in G15_MD_HONEST_RED if checks.get(k) is not False]
    extra = sorted(set(checks) - (G15_MD_HONEST_GREEN | G15_MD_HONEST_RED))
    if doc.get("status") != "fail" or green_bad or red_bad or extra:
        return False, (
            f"G15 M-d 诚实红结构异常: status={doc.get('status')!r} green_bad={green_bad} "
            f"red_bad={red_bad} extra={extra}"
        )
    parity = doc.get("parity") or {}
    consumed_rel = parity.get("g14_m_d_evidence")
    consumed_path = ROOT / str(consumed_rel or "")
    if not consumed_path.is_file():
        return False, f"消费的 G14 M-d 复跑件缺失: {consumed_rel!r}"
    ok, detail = eval_honest_red_g14_md(wel.load_json(consumed_path))
    if not ok:
        return False, f"消费的 G14 M-d 复跑件诚实红链异常: {detail}"
    contract_text = G15_CONTRACT.read_text(encoding="utf-8") if G15_CONTRACT.is_file() else ""
    if "G15-MD-F1" not in contract_text:
        return False, "G15_CONTRACT 缺 G15-MD-F1 承接锚登记字面"
    return True, (
        "G15 M-d 诚实红门面特判合格（status=fail + 绿键 6 全真 + 红键 6 全假闭集 + "
        f"消费的 G14 M-d 复跑件 checks 全绿 ∧ status=fail ∧ unmet==1==登记表行数〔{consumed_rel}〕+ "
        "G15-MD-F1 承接锚在案——未达标如实登记不充绿亦不充降级）"
    )


def eval_honest_red_wave5() -> dict:
    """wave5 聚合红面特判（同型字面）：聚合 FAIL 红面 = M-d 行 FAIL 镜像最新
    M-d 实测态 + facts ④⑤ 绿 + 红 facts ⊆ M-d 红同源闭集——如实登记不充绿不
    充降级；+ G15 M-d 诚实红结构链合格。"""
    key, prefix = ("g15.wave.5.exit", WAVE5_PREFIX)
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return {"symbolic_gate_key": key, "subject_prefix": prefix,
                "evidence_path": None, "status": "FAIL", "detail": "缺最新 evidence"}
    doc = wel.load_json(path)
    facts = doc.get("extra_facts") or []
    checks = doc.get("checks") or {}
    rows = doc.get("required_gates") or []
    md_status = next(
        (r.get("status") for r in rows
         if r.get("subject_prefix") == G15_MD_PREFIX),
        None,
    )
    fact_map = {f.get("id"): f.get("status") for f in facts}
    green_ok = all(fact_map.get(fid) == "PASS" for fid in WAVE5_GREEN_FACTS)
    red_ids = {fid for fid, st in fact_map.items() if st != "PASS"}
    red_ok = red_ids <= WAVE5_RED_FACTS
    md_ok, md_detail = verify_g15_md_honest_red()
    ok = (
        doc.get("host_section_pass") is False
        and checks.get("all_required_gates_pass") is False
        and checks.get("all_extra_facts_pass") is False
        and md_status == "FAIL"
        and green_ok
        and red_ok
        and md_ok
    )
    detail = (
        f"wave5 诚实红聚合面合格（M-d 行 FAIL 镜像 + facts ④⑤ 绿 + 红 facts {sorted(red_ids)} ⊆ "
        f"M-d 红同源闭集 + G15 M-d 诚实红结构链合格——红面如实登记不充绿不充降级；{md_detail}）"
        if ok else
        f"wave5 聚合面异常: hsp={doc.get('host_section_pass')!r} checks={checks} md_row={md_status!r} "
        f"green_ok={green_ok} red_ids={sorted(red_ids)} md_chain={md_detail}"
    )
    return {
        "symbolic_gate_key": key, "subject_prefix": prefix,
        "evidence_path": str(path.relative_to(ROOT).as_posix()),
        "status": "PASS" if ok else "FAIL",
        "detail": detail,
        "timestamp": doc.get("timestamp"),
    }


def verify_assertion_gate(key: str, prefix: str) -> dict:
    """P0 门核验：wel.require_gate_pass 之上叠加顶层 status=="pass" 字面（缺字段即红）。
    M-d 诚实红门面特判：verify_g15_md_honest_red 合格 = 未达标如实登记面——
    不充绿亦不充降级（红即红登记，回归语义 = 判据机核面零劣化）。"""
    row = wel.require_gate_pass(key, prefix)
    path = wel.load_latest_evidence(prefix)
    if path is None:
        row["status"] = "FAIL"
        row["detail"] = f"缺最新 evidence（{prefix}_*.json）"
        return row
    doc = wel.load_json(path)
    if prefix == G15_MD_PREFIX:
        ok, detail = verify_g15_md_honest_red()
        if ok:
            row["status"] = "PASS"
            row["detail"] = f"{row.get('detail','')}; {detail}"
        else:
            row["status"] = "FAIL"
            row["detail"] = detail
        return row
    if doc.get("status") != "pass":
        row["status"] = "FAIL"
        row["detail"] = f"顶层 status={doc.get('status')!r} ≠ 'pass'（缺字段即红面）"
    return row


def run_regression() -> tuple[bool, list[dict], list[str]]:
    """全量回归（5 P0 + 5 聚合/决策门最新 evidence 核验——M-d/wave5 诚实红特判）。"""
    rows: list[dict] = []
    red_faces: list[str] = []
    all_ok = True
    for key, prefix in P0_GATES:
        row = verify_assertion_gate(key, prefix)
        if prefix == G15_MD_PREFIX and row["status"] == "PASS":
            red_faces.append(prefix)
        rows.append(row)
        if row["status"] != "PASS":
            all_ok = False
    for key, prefix in AGG_GATES:
        if prefix == WAVE5_PREFIX:
            row = eval_honest_red_wave5()
            if row["status"] == "PASS":
                red_faces.append(prefix)
        else:
            row = wel.require_gate_pass(key, prefix)
        rows.append(row)
        if row["status"] != "PASS":
            all_ok = False
    return all_ok, rows, red_faces


# ---------------------------------------------------------------- ② 帧率对标链路 soak
def _bench_leg(scene: str, tier: int, backend: str, anchors: dict) -> str | None:
    """bench 腿：六组合轮转 canonical 协议（160 帧 warmup 10）；last_frame_digest
    == g14_3_stage_a_digest_anchor 冻结锚（位级复现——漂移即 M165 同型检出）。"""
    out_root = SOAK_ROOT / backend
    cmd = [
        str(RURIX_BIN), "--bench", "--scene", scene, "--tier", str(tier),
        "--backend", backend, "--frames", str(FRAME_COUNT), "--warmup", "10",
        "--out-root", str(out_root),
    ]
    import os
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env, timeout=7200)
    if r.returncode != 0:
        return f"bench 腿 {scene}/t{tier}/{backend} exit={r.returncode}: {(r.stdout + r.stderr)[-200:]}"
    receipt_p = out_root / scene / f"tier{tier}" / backend / "bench_receipt.json"
    if not receipt_p.is_file():
        return f"bench 腿 {scene}/t{tier}/{backend} receipt 缺失"
    rec = json.loads(receipt_p.read_text(encoding="utf-8"))
    dig = rec.get("last_frame_digest", "")
    if not dig.startswith("sha256:"):
        return f"bench 腿 {scene}/t{tier}/{backend} last_frame_digest 形态异常: {dig!r}"
    key = f"{scene}_t{tier}_{backend}"
    anchor = anchors.get(key)
    if anchor is None:
        return f"bench 腿 {key} 冻结锚缺位（锚表 18 格闭集外组合）"
    if dig != anchor:
        return f"bench 腿 {key} digest 漂移检出（M165 同型监控面）: {dig} ≠ 锚 {anchor}"
    return None


def _registry_leg() -> str | None:
    """登记表装配腿：最新 G14 M-d evidence unmet_count ↔ 在树 g14 帧率登记表行集幂等复核。"""
    path = wel.load_latest_evidence(HONEST_RED_PREFIX)
    if path is None:
        return "登记表装配腿 缺最新 G14 M-d evidence"
    if not FPS_REGISTRY_PATH.is_file():
        return f"登记表装配腿 {FPS_REGISTRY_PATH.name} 在树缺失"
    ev = wel.load_json(path)
    unmet = (ev.get("parity") or {}).get("unmet_count")
    reg = json.loads(FPS_REGISTRY_PATH.read_text(encoding="utf-8"))
    items = reg.get("items") or []
    if unmet is None or len(items) != unmet:
        return f"登记表装配腿 行数={len(items)} ≠ 最新 G14 M-d unmet_count={unmet}"
    bad = [it.get("gap_id") for it in items
           if not it.get("gap_id") or it.get("measured_delta") is None and not it.get("measured")]
    if bad:
        return f"登记表装配腿 measured 面空行: {bad[:2]}"
    return None


def judge_chain_soak(raw: dict, *, min_seconds: int = MIN_SECONDS,
                     min_iterations: int = MIN_ITERATIONS,
                     outer_elapsed: float | None = None) -> tuple[bool, list[str]]:
    """soak raw 判读面（门真跑与 --verify-latest/selftest 同一判据字面）。"""
    problems: list[str] = []
    seconds = float(raw.get("seconds") or 0.0)
    active = float(raw.get("active_chain_seconds") or 0.0)
    if seconds < min_seconds:
        problems.append(f"墙钟不足: {seconds:.1f}s < {min_seconds}s")
    if int(raw.get("iterations") or 0) < min_iterations:
        problems.append(f"迭代计数不足/为空: {raw.get('iterations')}")
    if int(raw.get("failures") or 0) != 0:
        problems.append(f"failures 非 0: {raw.get('failures')}")
    if float(raw.get("sleep_seconds") or 0.0) != 0.0:
        problems.append("sleep_seconds 非 0（谎报面）")
    if seconds > 0 and abs(active - seconds) > max(30.0, seconds * 0.05):
        problems.append(f"active_chain_seconds={active:.1f} 与 seconds={seconds:.1f} 背离（谎报面）")
    if outer_elapsed is not None and abs(outer_elapsed - seconds) > 120.0:
        problems.append(f"外测墙钟 {outer_elapsed:.1f}s 与内测 {seconds:.1f}s 背离（交叉核验谎报面）")
    if int(raw.get("bench_frames_measured") or 0) == 0:
        problems.append("bench 帧计数为空")
    if int(raw.get("registry_assemblies") or 0) == 0:
        problems.append("登记表装配计数为空")
    return not problems, problems


def run_chain_soak() -> tuple[bool, dict]:
    """画质/帧率链路连续复跑 soak：≥1800s 墙钟逐迭代真跑，零 sleep，零失败。"""
    counters = {
        "iterations": 0,
        "failures": 0,
        "bench_frames_measured": 0,
        "registry_assemblies": 0,
    }
    problems: list[str] = []
    rb = subprocess.run(
        ["cargo", "build", "--release", "-p", "rurix-render",
         "--bin", "g14_3_pipeline_perf", "--features", "vendor-upscale"],
        cwd=ROOT, timeout=7200, capture_output=True, text=True,
    )
    if rb.returncode != 0 or not RURIX_BIN.is_file():
        tail = ((rb.stderr or "") + (rb.stdout or "")).strip()[-400:]
        problems.append(f"soak 前置构建失败（release g14_3_pipeline_perf）: rc={rb.returncode} {tail}")
        return False, {"ok": False, "detail": f"problems={problems}", "raw": {}, "counters": counters}
    anchors_doc = json.loads(ANCHOR_PATH.read_text(encoding="utf-8")) if ANCHOR_PATH.is_file() else {}
    anchors = {k: (v or {}).get("last_frame_digest", "") for k, v in (anchors_doc.get("anchors") or {}).items()}
    missing = [f"{s}_t{t}_{b}" for s, t, b in SOAK_COMBOS if not anchors.get(f"{s}_t{t}_{b}")]
    if missing:
        problems.append(f"冻结锚缺位组合: {missing}")
        return False, {"ok": False, "detail": f"problems={problems}", "raw": {}, "counters": counters}
    SOAK_ROOT.mkdir(parents=True, exist_ok=True)
    outer0 = time.time()
    with gpu_device_lock(purpose="g15.6a fps-parity-chain soak"):
        t0 = time.time()
        active = 0.0
        while time.time() - t0 < MIN_SECONDS and not problems:
            it0 = time.time()
            scene, tier, backend = SOAK_COMBOS[counters["iterations"] % len(SOAK_COMBOS)]
            p = _bench_leg(scene, tier, backend, anchors)
            if p:
                problems.append(p)
            else:
                counters["bench_frames_measured"] += FRAME_COUNT
                p = _registry_leg()
                if p:
                    problems.append(p)
                else:
                    counters["registry_assemblies"] += 1
            active += time.time() - it0
            counters["iterations"] += 1
            if problems:
                counters["failures"] += 1
                break
            print(
                f"[{TAG}] soak iter {counters['iterations']} ok "
                f"({scene}/t{tier}/{backend}; elapsed {time.time() - t0:.1f}s/{MIN_SECONDS}s)",
                flush=True,
            )
    seconds = time.time() - t0
    outer = time.time() - outer0
    raw = {
        "soak_subject": "fps-parity-chain-soak",
        "iterations": counters["iterations"],
        "seconds": seconds,
        "min_iterations": MIN_ITERATIONS,
        "min_seconds": MIN_SECONDS,
        "active_chain_seconds": active,
        "sleep_seconds": 0.0,
        "outer_wall_seconds": outer,
        "failures": counters["failures"],
        "bench_frames_measured": counters["bench_frames_measured"],
        "registry_assemblies": counters["registry_assemblies"],
        "backend_digest_anchors": {k: anchors[k] for k in sorted(anchors) if any(
            k == f"{s}_t{t}_{b}" for s, t, b in SOAK_COMBOS)},
    }
    ok, judge_problems = judge_chain_soak(raw, outer_elapsed=outer)
    problems.extend(judge_problems)
    detail = (
        f"iterations={counters['iterations']} seconds={seconds:.1f} active={active:.1f} "
        f"outer={outer:.1f} sleep=0.0 failures={counters['failures']} "
        f"frames={counters['bench_frames_measured']} registries={counters['registry_assemblies']} "
        f"subject='fps-parity-chain-soak'"
    )
    if problems:
        detail += f" problems={problems[:6]}"
    return ok and not problems, {
        "ok": ok and not problems,
        "iterations": counters["iterations"],
        "seconds": seconds,
        "active_chain_seconds": active,
        "detail": detail,
        "raw": raw,
        "counters": counters,
    }


# ---------------------------------------------------------------- ③ budget strict
def run_budget_strict() -> tuple[bool, str]:
    """budget_eval --strict：非空、零 estimated/skip（exit 0 且 PASS 且 0 skip）。"""
    r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "budget_eval.py"), "--strict"],
        cwd=ROOT, capture_output=True, text=True,
    )
    blob = (r.stdout or "") + (r.stderr or "")
    ok = (
        r.returncode == 0
        and "[budget_eval] PASS" in blob
        and "0 skip" in blob
        and "0 pass" not in blob
    )
    tail = " / ".join(blob.strip().splitlines()[-2:])
    return ok, f"exit={r.returncode} {tail}"


# ---------------------------------------------------------------- 门体
def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def run_full_gate() -> int:
    started = _dt.datetime.now(_dt.timezone.utc)
    stamp = started.strftime("%Y%m%dT%H%M%SZ")
    utc_date = started.strftime("%Y%m%d")

    _note("① 全量回归（5 P0 最新 evidence 核验 + 5 聚合/决策门核验——M-d/wave5 诚实红特判）…")
    reg_ok, gate_rows, red_faces = run_regression()
    hr_ok = red_faces == [G15_MD_PREFIX, WAVE5_PREFIX]
    _note(f"回归腿：{sum(1 for r in gate_rows if r['status'] == 'PASS')}/{len(gate_rows)} 合格；登记红面 = {red_faces}")

    _note("② 画质/帧率链路连续复跑 soak（≥1800s，digest == 冻结锚位级复现——RD-045 零检出预期）…")
    soak_ok, soak_info = run_chain_soak()

    _note("③ budget_eval --strict…")
    bud_ok, bud_detail = run_budget_strict()

    _note("④ G5~G14 0-byte + 日期锚…")
    leg_ok, leg_detail = git_closed_surface()

    raw = soak_info.get("raw") or {}
    no_sleep_ok = raw.get("sleep_seconds") == 0.0
    facts = [
        _fact("regression_gates_zero_degrade", reg_ok,
              "5 P0 + 5 聚合/决策门最新 evidence 核验全合格（M-d/wave5 诚实红门面特判合格——回归腿零降级）"
              if reg_ok else "回归面见 required_gates 行集"),
        _fact("honest_red_faces_registered", hr_ok,
              f"红面登记完备：登记红面 == [g15_m_d, g15_wave5_exit]（如实登记不充绿不充降级）"
              if hr_ok else f"红面集 {red_faces} ≠ 登记闭集 [g15_m_d, g15_wave5_exit]"),
        _fact("soak_dual_threshold", soak_ok, soak_info["detail"]),
        _fact("budget_strict", bud_ok, bud_detail),
        _fact("legacy_criteria_0byte", leg_ok, leg_detail),
        _fact("date_anchor", True, f"utc_date={utc_date}"),
    ]
    overall = (
        reg_ok and hr_ok and soak_ok and no_sleep_ok and bud_ok and leg_ok
    )
    payload = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": WAVE,
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": overall,
        "device_section_state": "not_applicable",
        "base_commit": base_commit(),
        "utc_date": utc_date,
        "required_gates": gate_rows,
        "extra_facts": facts,
        "subjects": [],
        "checks": {
            "regression_gates_zero_degrade": reg_ok,
            "honest_red_faces_registered": hr_ok,
            "soak_dual_threshold": soak_ok,
            "soak_no_sleep_padding": no_sleep_ok,
            "budget_strict_pass": bud_ok,
            "legacy_criteria_0byte": leg_ok,
            "date_anchor_recorded": True,
        },
        "soak": raw,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": (
            "G15.6a full soak；四腿 + G5~G14 既有判据 0-byte fact；诚实语义（沿 "
            "G14.5a/G13.5a 口径）：soak 墙钟=真实画质/帧率链路复跑实测（禁 sleep 充时，"
            "sleep_seconds 恒 0，active_chain_seconds 逐迭代计时求和，gate 外测墙钟交叉核验）；"
            "soak 载体=帧率对标链路二面（bench g14_3_pipeline_perf --bench 六组合轮转 "
            "canonical 协议 160 帧，receipt last_frame_digest == g14_3_stage_a_digest_anchor "
            "冻结锚位级复现——跨迭代 digest 漂移即 M165 同型监控面检出，RD-045 零检出预期 → "
            "登记表装配最新 G14 M-d evidence unmet_count 与在树 g14_fps_gap_registry.json "
            "行集对账幂等复核）；M-d/wave5 诚实红门面特判（G14 closeout 先例同型字面：红面"
            "如实登记不充绿不充降级，soak 门 VERDICT 语义 = 回归腿零降级 + 红面登记完备）；"
            "subject=fps-parity-chain-soak 无 validation/device_lost 字面量 0 硬门（device "
            "面归 5 P0 本体）"
        ),
    }
    errs = wel.validate_schema(payload, SCHEMA_PATH)
    if errs:
        print(f"[{TAG}] schema errors: {errs}", file=sys.stderr)
        overall = False
        payload["host_section_pass"] = False
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    out.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    for f in facts:
        print(f"  FACT  {f['status']:4}  {f['id']}  ({f['detail'][:200]})")
    print(f"  → evidence {out.relative_to(ROOT)}")
    print(f"  VERDICT = {'PASS' if overall else 'FAIL'}")
    return 0 if overall else 1


def verify_latest() -> int:
    """pr-smoke 面：秒级核最新 full-run evidence（含 soak 判读面复核）。"""
    path = wel.load_latest_evidence(SUBJECT)
    if path is None:
        print(f"[{TAG}] FAIL: 缺最新 evidence（{SUBJECT}_*.json）", file=sys.stderr)
        return 1
    data = wel.load_json(path)
    checks = data.get("checks") or {}
    need = {
        "regression_gates_zero_degrade", "honest_red_faces_registered", "soak_dual_threshold",
        "soak_no_sleep_padding", "budget_strict_pass", "legacy_criteria_0byte",
        "date_anchor_recorded",
    }
    bad = [k for k in need if checks.get(k) is not True]
    raw = data.get("soak") or {}
    soak_ok, problems = judge_chain_soak(raw)
    if bad or not soak_ok:
        print(f"[{TAG}] FAIL checks={bad} soak={problems[:3]}", file=sys.stderr)
        return 1
    print(f"[{TAG}] verify-latest PASS（{path.name}，checks 7 键全绿 + soak 判读面复核过）")
    return 0


def selftest() -> int:
    """反假绿臂：谎报/sleep 充时/迭代不足/计数面空/外测背离必红 + 诚实样本绿 +
    诚实红冒充必红（wave5/M-d 评定面）。"""
    failures = 0
    good = {
        "soak_subject": "fps-parity-chain-soak",
        "iterations": 60, "seconds": 1801.0, "min_iterations": MIN_ITERATIONS,
        "min_seconds": MIN_SECONDS, "active_chain_seconds": 1798.0,
        "sleep_seconds": 0.0, "outer_wall_seconds": 1803.0, "failures": 0,
        "bench_frames_measured": 9600, "registry_assemblies": 60,
        "backend_digest_anchors": {"cornell-box_t67_dlss_sr": "sha256:" + "0" * 64},
    }
    ok, _ = judge_chain_soak(good, outer_elapsed=1803.0)
    if not ok:
        print(f"[{TAG}] selftest FAIL: 正本误判", file=sys.stderr)
        failures += 1
    for name, mut, outer in (
        ("sleep 谎报", {"sleep_seconds": 0.5}, 1803.0),
        ("迭代不足", {"iterations": 1}, 1803.0),
        ("failures 注入", {"failures": 1}, 1803.0),
        ("墙钟不足", {"seconds": 100.0, "active_chain_seconds": 99.0}, 102.0),
        ("active 背离谎报", {"active_chain_seconds": 100.0}, 1803.0),
        ("外测背离谎报", {}, 100.0),
        ("bench 计数空", {"bench_frames_measured": 0}, 1803.0),
        ("装配计数空", {"registry_assemblies": 0}, 1803.0),
    ):
        bad = dict(good)
        bad.update(mut)
        ok, _ = judge_chain_soak(bad, outer_elapsed=outer)
        if ok:
            print(f"[{TAG}] selftest FAIL: {name} 未检出", file=sys.stderr)
            failures += 1
    # 诚实红冒充臂：G14 M-d 评定面注入非登记红面（checks 非全绿）→ 必拒答
    masq_ok, _ = eval_honest_red_g14_md({
        "status": "fail", "checks": {"a": True, "b": False}, "parity": {"unmet_count": 1},
    })
    if masq_ok:
        print(f"[{TAG}] selftest FAIL: 诚实红冒充（checks 非全绿）未拒答", file=sys.stderr)
        failures += 1
    legit_ok, _ = eval_honest_red_g14_md({
        "status": "fail", "checks": {"a": True, "b": True}, "parity": {"unmet_count": 1},
    })
    if not legit_ok:
        print(f"[{TAG}] selftest FAIL: 真诚实红登记面误拒", file=sys.stderr)
        failures += 1
    if failures:
        return 1
    print(f"[{TAG}] selftest PASS（1 GREEN + 8 RED 判读面 + 2 诚实红评定面臂）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G15.6a stabilization soak 门（g15.wave.6a.soak）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑 full-run soak 门")
    g.add_argument("--verify-latest", action="store_true", help="秒级核最新 full-run evidence")
    g.add_argument("--selftest", action="store_true", help="判读面正/负样本自检")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.verify_latest:
        return verify_latest()
    return run_full_gate()


if __name__ == "__main__":
    sys.exit(main())
