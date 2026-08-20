#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.5a P2 穷举 + soak 波）
"""G14.5a stabilization soak 聚合门 g14.wave.5a.soak（G14_CONTRACT G-G14-8；
G14_PLAN §2 G14.5a；同构 ci/g13_stabilization_soak.py〔G13.5a〕先例）。

四腿：①全量回归（G14 5 P0 = M-a~M-e 逐门真跑 --gate——机器核验最新 evidence
顶层 status=="pass" 字面〔缺字段即红〕；**M-d 诚实红门面特判**：checks 全绿 ∧
status=="fail" ∧ unmet_count == g14_fps_gap_registry 行数 = 通过线未达标如实
登记面（G-G14-6 诚实红不充绿亦不充降级）；wave2/3/4/6/7 exit + wave5a
decisions 六聚合/决策门真跑核验；5 门 evidence base_commit 同值=HEAD 且 11 门
evidence 文件名 UTC stamp ≥ run 起点 = 同一候选 close-out 基线，沿 G13.5a/
G12.7a/G11.7a/G10.8a MAP §7 口径）+ ②帧率对标链路（生产管线 bench → 帧率
差距登记表装配复核）连续复跑 soak（≥1800s 墙钟沿 G13.5a/G12.7a/G11.7a/G10.8a/
G9.8a 继承；真实链路逐迭代连续复跑，迭代计数与各环节计数非空、零失败；
sleep_seconds 恒 0，active_chain_seconds ≈seconds，gate 外测墙钟交叉核验，
谎报判红）+ ③budget_eval --strict 非空零 estimated/skip + ④纪律日期锚 +
G5~G13 既有判据 0-byte fact（M-e 门同闭集字面：已提交 diff ⊆ G14.2 M-a 授权
四面，工作树 porcelain ⊆ {g12_pt_sampler_selection.json 异己登记面}）。

诚实语义（沿 G13.5a/G12.7a/G11.7a/G10.8a/G9.8a 口径，G14 无 legacy 兼容）：
- soak 墙钟=真实链路复跑实测（active_chain_seconds 逐迭代计时求和），迭代间
  零 sleep（sleep_seconds 恒 0）；gate 侧用外测墙钟交叉核验，谎报 seconds 判红。
- soak 载体 = G14 帧率对标链路二面（与 M-c/M-d 门数据面同构）：
  bench 腿 = g14_3_pipeline_perf --bench（cornell-box/bistro-interior t67 ×
    tsr_device/dlss_sr/fsr_3_1_5 六组合轮转，canonical 协议 160 帧 warmup 10）
    device 真跑，receipt last_frame_digest == g14_3_stage_a_digest_anchor
    冻结锚逐字一致（固定 seed 位级复现——跨迭代 digest 漂移即 M165 同型
    监控面检出登记）；
  登记表装配腿 = 最新 M-d evidence unmet_count 与在树 g14_fps_gap_registry.json
    行集对账幂等复核（行数全等 + 全行 measured 面非空）。
  device 腿持 gpu_device_lock 串行；UE 帧不重复 benchmark（与 G13.5a 同口径——
  UE 臂真跑归 ①回归腿 M-b/M-d 门本体）。
- 迭代计数/bench 帧数/登记表装配次数非空（空即红），failures 恒 0；
  evidence soak 块 seconds 与 iterations 双字段机器可核；chain-soak 无
  validation/device_lost 字面量 0 硬门（device 面归回归腿 5 门本体）。

pr-smoke 默认 --verify-latest（秒级核最新 full-run evidence）；
本地/workflow_dispatch 用 --gate 产 full-run。

用法：
  py -3 ci/g14_stabilization_soak.py --gate g14.wave.5a.soak
  py -3 ci/g14_stabilization_soak.py --verify-latest
  py -3 ci/g14_stabilization_soak.py --selftest
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
from g14_regression_drift_guard_smoke import git_closed_surface  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g14.wave.5a.soak"
NUMERIC_STEP = 263  # 落盘前实测 registry/number_ledger.json CI_step.next_free=263 顺位领取
SUBJECT = "g14_stabilization_soak"
WAVE = "G14.5a"
TAG = "g14_5a_soak"
SOURCE_REF = (
    "G14_CONTRACT G-G14-8/§2.2;G14 5 P0（M-a~M-e）逐门真跑 + 聚合/决策门核验;"
    "帧率对标链路连续复跑 soak ≥1800s（G13.5a 继承）;budget_eval --strict;G5~G13 0-byte"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_stabilization_soak_evidence_schema.json"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
FPS_REGISTRY_PATH = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"
MD_PREFIX = "g14_m_d_dual_end_fps_parity"

P0_GATES = [
    ("g14.p0.m_a.registry_variance_band_reconciliation", "g14_m_a_registry_variance_band_reconciliation",
     "ci/g14_registry_variance_band_reconciliation_smoke.py"),
    ("g14.p0.m_b.ue_benchmark_arm_measurement", "g14_m_b_ue_benchmark_arm_measurement",
     "ci/g14_ue_benchmark_arm_measurement_smoke.py"),
    ("g14.p0.m_c.rurix_pipeline_perf", "g14_m_c_rurix_pipeline_perf",
     "ci/g14_rurix_pipeline_perf_smoke.py"),
    ("g14.p0.m_d.dual_end_fps_parity", "g14_m_d_dual_end_fps_parity",
     "ci/g14_dual_end_fps_parity_smoke.py"),
    ("g14.p0.m_e.regression_drift_guard", "g14_m_e_regression_drift_guard",
     "ci/g14_regression_drift_guard_smoke.py"),
]
AGG_GATES = [
    ("g14.wave.2.exit", "g14_wave2_exit", "ci/g14_wave2_exit_check.py"),
    ("g14.wave.3.exit", "g14_wave3_exit", "ci/g14_wave3_exit_check.py"),
    ("g14.wave.4.exit", "g14_wave4_exit", "ci/g14_wave4_exit_check.py"),
    ("g14.wave.6.exit", "g14_wave6_exit", "ci/g14_wave6_exit_check.py"),
    ("g14.wave.7.exit", "g14_wave7_exit", "ci/g14_wave7_exit_check.py"),
    ("g14.wave.5a.decisions", "g14_p2_decisions", "ci/g14_p2_decisions_check.py"),
]
REGRESSION_GATES = P0_GATES + AGG_GATES
N_ASSERTION_GATES = len(P0_GATES)

# 诚实红聚合门闭集（M-d 通过线未达期间 wave4 聚合 VERDICT=FAIL 为正确诚实态——
# 红不充绿亦不充降级；合格面 = 最新 evidence 六 facts 全绿 + required M-d 行 FAIL
# 镜像最新 M-d evidence 实测态，聚合不遮蔽机核维持）。
HONEST_RED_AGGREGATES = frozenset({"g14_wave4_exit"})


def eval_honest_red_aggregate(key: str, prefix: str) -> dict:
    """诚实红聚合门评定（soak 回归腿与 closeout 共用单一事实源）。"""
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return {"symbolic_gate_key": key, "subject_prefix": prefix,
                "evidence_path": None, "status": "FAIL", "detail": "缺最新 evidence"}
    doc = wel.load_json(path)
    facts = doc.get("extra_facts") or []
    checks = doc.get("checks") or {}
    rows = doc.get("required_gates") or []
    md_row_fail = any(
        r.get("subject_prefix") == "g14_m_d_dual_end_fps_parity" and r.get("status") == "FAIL"
        for r in rows
    )
    ok = (
        bool(facts)
        and all(f.get("status") == "PASS" for f in facts)
        and checks.get("all_required_gates_pass") is False
        and all(v is True for k, v in checks.items() if k != "all_required_gates_pass")
        and md_row_fail
    )
    return {
        "symbolic_gate_key": key, "subject_prefix": prefix,
        "evidence_path": str(path.relative_to(ROOT).as_posix()),
        "status": "PASS" if ok else "FAIL",
        "detail": ("诚实红聚合面合格（facts 全绿 + M-d 行 FAIL 镜像维持）" if ok
                   else "诚实红聚合面异常（合格面 = facts 绿 + M-d 行 FAIL 镜像）"),
        "timestamp": doc.get("timestamp"),
    }

RURIX_BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"
SOAK_ROOT = Path(r"K:\rurix-ext\g14-frames\g14_5a_soak")
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
def verify_assertion_gate(key: str, prefix: str) -> dict:
    """P0 门核验：wel.require_gate_pass 之上叠加顶层 status=="pass" 字面（缺字段即红）。

    M-d 诚实红门面特判（G-G14-6 字面）：status=="fail" 且 checks 全绿且
    unmet_count == g14_fps_gap_registry 行数 = 通过线未达标如实登记面——
    不充绿亦不充降级（红即红登记，回归语义 = 判据机核面零劣化）。"""
    row = wel.require_gate_pass(key, prefix)
    path = wel.load_latest_evidence(prefix)
    if path is None:
        row["status"] = "FAIL"
        row["detail"] = f"缺最新 evidence（{prefix}_*.json）"
        return row
    doc = wel.load_json(path)
    if prefix == MD_PREFIX:
        checks = doc.get("checks") or {}
        bad = [k for k, v in checks.items() if v is not True]
        status = doc.get("status")
        unmet = (doc.get("parity") or {}).get("unmet_count")
        reg_rows = -1
        if FPS_REGISTRY_PATH.is_file():
            reg_rows = len((wel.load_json(FPS_REGISTRY_PATH)).get("items") or [])
        if status == "pass":
            pass  # 达标面（未来延续波可能翻转）——wel 口径已绿
        elif status == "fail" and not bad and unmet is not None and unmet == reg_rows:
            # 诚实红登记面（checks 全绿 + 红如实登记一致）——回归合格，行置 PASS
            # 并在 detail 承载诚实红字面（不充绿：M-d 门自身 evidence status=fail 0-byte 维持）
            row["status"] = "PASS"
            row["detail"] = (f"{row.get('detail','')}; M-d 诚实红门面特判合格"
                             f"（checks 全绿 + unmet={unmet} == 登记表 {reg_rows} 行——"
                             f"通过线未达标如实登记不冒充）")
        else:
            row["status"] = "FAIL"
            row["detail"] = (f"M-d 面异常: status={status!r} checks_bad={bad[:3]} "
                             f"unmet={unmet} reg={reg_rows}（诚实红登记面不一致即回归降级）")
        return row
    if doc.get("status") != "pass":
        row["status"] = "FAIL"
        row["detail"] = f"顶层 status={doc.get('status')!r} ≠ 'pass'（缺字段即红面）"
    return row


def run_regression(*, skip_rerun: bool = False) -> tuple[bool, list[dict], str, bool]:
    """全量回归（5 P0 + 6 聚合/决策门）。口径沿 G13.5a run_regression 同构。"""
    rows: list[dict] = []
    commit = base_commit()
    run_start_stamp = wel.utc_stamp()
    all_ok = True
    bases: list[str] = []
    no_base_field: list[str] = []
    stale: list[str] = []
    stamp_re = re.compile(r"_(\d{8}T\d{6}Z)\.json$")
    for idx, (key, prefix, script_rel) in enumerate(REGRESSION_GATES):
        is_aggregate = idx >= N_ASSERTION_GATES
        if not skip_rerun:
            script = ROOT / script_rel
            if not script.is_file():
                rows.append({
                    "symbolic_gate_key": key, "subject_prefix": prefix,
                    "evidence_path": None, "status": "FAIL",
                    "detail": f"smoke missing: {script_rel}",
                })
                all_ok = False
                continue
            print(f"[{TAG}] regression {key}", flush=True)
            r = subprocess.run(
                [sys.executable, str(script), "--gate", key],
                cwd=ROOT,
            )
            expected_rc = 1 if (prefix in HONEST_RED_AGGREGATES or prefix == MD_PREFIX) else 0
            if r.returncode != expected_rc:
                rows.append({
                    "symbolic_gate_key": key, "subject_prefix": prefix,
                    "evidence_path": None, "status": "FAIL",
                    "detail": f"smoke exit={r.returncode}（expect {expected_rc}）",
                })
                all_ok = False
                continue
        if is_aggregate:
            row = (eval_honest_red_aggregate(key, prefix) if prefix in HONEST_RED_AGGREGATES
                   else wel.require_gate_pass(key, prefix))
        else:
            row = verify_assertion_gate(key, prefix)
            path = wel.load_latest_evidence(prefix)
            if path is not None:
                try:
                    doc = wel.load_json(path)
                    bc = doc.get("base_commit")
                    if bc is None:
                        no_base_field.append(key)
                    else:
                        bases.append(str(bc))
                except (OSError, json.JSONDecodeError):
                    bases.append("<unreadable>")
        if not skip_rerun and row.get("evidence_path"):
            m = stamp_re.search(str(row["evidence_path"]))
            if m is None or m.group(1) < run_start_stamp:
                stale.append(key)
                row["status"] = "FAIL"
                row["detail"] = (
                    f"{row.get('detail', '')}; evidence 非本 run 新鲜产出"
                    f"（stamp {m.group(1) if m else '?'} < run 起点 {run_start_stamp}）"
                )
        rows.append(row)
        if row["status"] != "PASS":
            all_ok = False
    base_uniform = (
        not stale
        and not no_base_field
        and len(bases) == N_ASSERTION_GATES
        and len(set(bases)) == 1
        and bases[0] == commit
        and commit != "unknown"
    )
    return all_ok, rows, commit, base_uniform


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
    """登记表装配腿：最新 M-d evidence unmet_count ↔ 在树 g14 帧率登记表行集幂等复核。"""
    path = wel.load_latest_evidence(MD_PREFIX)
    if path is None:
        return "登记表装配腿 缺最新 M-d evidence"
    if not FPS_REGISTRY_PATH.is_file():
        return f"登记表装配腿 {FPS_REGISTRY_PATH.name} 在树缺失"
    ev = wel.load_json(path)
    unmet = (ev.get("parity") or {}).get("unmet_count")
    reg = json.loads(FPS_REGISTRY_PATH.read_text(encoding="utf-8"))
    items = reg.get("items") or []
    if unmet is None or len(items) != unmet:
        return f"登记表装配腿 行数={len(items)} ≠ 最新 M-d unmet_count={unmet}"
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
    """帧率对标链路连续复跑 soak：≥1800s 墙钟逐迭代真跑，零 sleep，零失败。"""
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
    with gpu_device_lock(purpose="g14.5a fps-parity-chain soak"):
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

    _note("① 全量回归（5 P0 真跑 + 6 聚合/决策门核验）…")
    reg_ok, gate_rows, commit, base_uniform = run_regression()

    _note("② 帧率对标链路连续复跑 soak（≥1800s）…")
    soak_ok, soak_info = run_chain_soak()

    _note("③ budget_eval --strict…")
    bud_ok, bud_detail = run_budget_strict()

    _note("④ G5~G13 0-byte + 日期锚…")
    leg_ok, leg_detail = git_closed_surface()

    raw = soak_info.get("raw") or {}
    no_sleep_ok = raw.get("sleep_seconds") == 0.0
    facts = [
        _fact("regression_gates_all_pass", reg_ok,
              "5 P0 真跑 + 6 聚合/决策门核验全绿（M-d 诚实红门面特判合格）" if reg_ok else "回归面见 required_gates 行集"),
        _fact("base_commit_uniform", base_uniform,
              f"5 门 evidence base_commit 同值={commit[:12]}（同一候选 close-out 基线）"),
        _fact("soak_dual_threshold", soak_ok, soak_info["detail"]),
        _fact("budget_strict", bud_ok, bud_detail),
        _fact("legacy_criteria_0byte", leg_ok, leg_detail),
        _fact("date_anchor", True, f"utc_date={utc_date}"),
    ]
    overall = (
        reg_ok and base_uniform and soak_ok and no_sleep_ok and bud_ok and leg_ok
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
        "base_commit": commit,
        "utc_date": utc_date,
        "required_gates": gate_rows,
        "extra_facts": facts,
        "subjects": [],
        "checks": {
            "regression_all_pass": reg_ok,
            "base_commit_uniform": base_uniform,
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
            "G14.5a full soak；四腿 + G5~G13 既有判据 0-byte fact；诚实语义（沿 "
            "G13.5a/G12.7a/G11.7a/G10.8a/G9.8a 口径）：soak 墙钟=真实帧率对标链路"
            "复跑实测（禁 sleep 充时，sleep_seconds 恒 0，active_chain_seconds 逐迭代"
            "计时求和，gate 外测墙钟交叉核验）；soak 载体=G14 帧率对标链路二面"
            "（bench g14_3_pipeline_perf --bench 六组合轮转 canonical 协议 160 帧，"
            "receipt last_frame_digest == g14_3_stage_a_digest_anchor 冻结锚位级复现"
            "——跨迭代 digest 漂移即 M165 同型监控面检出 → 登记表装配最新 M-d "
            "evidence unmet_count 与在树 g14_fps_gap_registry.json 行集对账幂等复核）；"
            "M-d 诚实红门面特判（checks 全绿 + status=fail + unmet==登记表行数 = "
            "通过线未达标如实登记，不充绿亦不充降级）；subject=fps-parity-chain-soak "
            "无 validation/device_lost 字面量 0 硬门（device 面归回归腿 5 门本体）；"
            "5 门 evidence base_commit 同值一致（同一候选 close-out 基线）"
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
        "regression_all_pass", "base_commit_uniform", "soak_dual_threshold",
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
    """反假绿臂：谎报/sleep 充时/迭代不足/计数面空/外测背离必红 + 诚实样本绿。"""
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
    if failures:
        return 1
    print(f"[{TAG}] selftest PASS（1 GREEN + 8 RED 判读面）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G14.5a stabilization soak 门（g14.wave.5a.soak）")
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
