#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.5a P2 穷举 + soak 波）
"""G13.5a stabilization soak 聚合门 g13.wave.5a.soak（G13_CONTRACT G-G13-8；
G13_PLAN §2 G13.5a；同构 ci/g12_stabilization_soak.py〔G12.7a〕先例）。

四腿：①全量回归（G13 5 P0 = M-a~M-e 逐门真跑 --gate——机器核验最新 evidence
顶层 status=="pass" 字面〔缺字段即红〕；wave2/3/4 exit + wave5 decisions 四
聚合/决策门真跑核验；5 门 evidence base_commit 同值=HEAD 且 9 门 evidence
文件名 UTC stamp ≥ run 起点 = 同一候选 close-out 基线，沿 G12.7a/G11.7a/
G10.8a MAP §7 口径）+ ②超分链路（三后端出图→差距登记表装配复核）连续复跑
soak（≥1800s 墙钟沿 G12.7a/G11.7a/G10.8a/G9.8a 继承；真实链路逐迭代连续复跑，
迭代计数与各环节计数非空、零失败；sleep_seconds 恒 0，active_chain_seconds
≈seconds，gate 外测墙钟交叉核验，谎报判红）+ ③budget_eval --strict 非空零
estimated/skip + ④纪律日期锚 + G5~G12 既有判据 0-byte fact（M-e 门同闭集字面：
已提交 diff ⊆ {ci/g10_gap_registry_lib.py 加性演进位 v1.138 登记}，工作树
porcelain ⊆ {g12_pt_sampler_selection.json 异己登记面}）。

诚实语义（沿 G12.7a/G11.7a/G10.8a/G9.8a 口径，G13 无 legacy 兼容）：
- soak 墙钟=真实链路复跑实测（active_chain_seconds 逐迭代计时求和），迭代间
  零 sleep（sleep_seconds 恒 0）；gate 侧用外测墙钟交叉核验，谎报 seconds 判红。
- soak 载体 = G13 超分链路二面（与 M-a/M-b/M-c 门数据面同构）：
  出图腿 = g13_4_ue_upscale_parity_render --render cornell-box tier67 三后端
    逐迭代轮转（tsr_device/dlss_sr/fsr_3_1_5）device 真跑，receipt
    converged_digest == 本 soak 首轮锚（固定 seed 位级复现——跨迭代 digest
    漂移即 M165 同型监控面检出登记）；
  登记表装配腿 = 最新 M-c/M-d evidence 超容差格与在树 g13_ue_{upscale,lumen}_
    gap_registry.json 行集对账幂等复核（行数全等 + 登记双面 checks 真）。
  device 腿持 gpu_device_lock 串行；UE 帧不重复 MRQ（与 G12.7a 同口径——
  UE 臂真跑归 ①回归腿 M-c/M-d 门本体）。
- 迭代计数/出图帧数/登记表装配次数非空（空即红），failures 恒 0；
  evidence soak 块 seconds 与 iterations 双字段机器可核；chain-soak 无
  validation/device_lost 字面量 0 硬门（device 面归回归腿 5 门本体）。

pr-smoke 默认 --verify-latest（秒级核最新 full-run evidence）；
本地/workflow_dispatch 用 --gate 产 full-run。

用法：
  py -3 ci/g13_stabilization_soak.py --gate g13.wave.5a.soak
  py -3 ci/g13_stabilization_soak.py --verify-latest
  py -3 ci/g13_stabilization_soak.py --selftest
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

sys.path.insert(0, str(ROOT / "ci"))

import g10_wave_exit_lib as wel  # noqa: E402
from g13_regression_drift_guard_smoke import git_closed_surface  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g13.wave.5a.soak"
NUMERIC_STEP = 245  # 落盘前实测 registry/number_ledger.json CI_step.next_free=245 顺位领取
SUBJECT = "g13_stabilization_soak"
WAVE = "G13.5a"
TAG = "g13_5a_soak"
SOURCE_REF = (
    "G13_CONTRACT G-G13-8/§2.2;G13 5 P0（M-a~M-e）逐门真跑 + 聚合/决策门核验;"
    "超分链路连续复跑 soak ≥1800s（G12.7a 继承）;budget_eval --strict;G5~G12 0-byte"
)
SCHEMA_PATH = ROOT / "milestones" / "g13" / "g13_stabilization_soak_evidence_schema.json"

P0_GATES = [
    ("g13.p0.m_a.vendor_upscale_integration", "g13_m_a_vendor_upscale_integration",
     "ci/g13_vendor_upscale_integration_smoke.py"),
    ("g13.p0.m_b.tsr_device_kernel", "g13_m_b_tsr_device_kernel",
     "ci/g13_tsr_device_kernel_smoke.py"),
    ("g13.p0.m_c.ue_upscale_parity", "g13_m_c_ue_upscale_parity",
     "ci/g13_ue_upscale_parity_smoke.py"),
    ("g13.p0.m_d.ue_lumen_gi_parity", "g13_m_d_ue_lumen_gi_parity",
     "ci/g13_ue_lumen_gi_parity_smoke.py"),
    ("g13.p0.m_e.regression_drift_guard", "g13_m_e_regression_drift_guard",
     "ci/g13_regression_drift_guard_smoke.py"),
]
AGG_GATES = [
    ("g13.wave.2.exit", "g13_wave2_exit", "ci/g13_wave2_exit_check.py"),
    ("g13.wave.3.exit", "g13_wave3_exit", "ci/g13_wave3_exit_check.py"),
    ("g13.wave.4.exit", "g13_wave4_exit", "ci/g13_wave4_exit_check.py"),
    ("g13.wave.5.decisions", "g13_p2_decisions", "ci/g13_p2_decisions_check.py"),
]
REGRESSION_GATES = P0_GATES + AGG_GATES
N_ASSERTION_GATES = len(P0_GATES)

RURIX_BIN = ROOT / "target" / "release" / "g13_4_ue_upscale_parity_render.exe"
SOAK_ROOT = Path(r"K:\rurix-ext\g13-frames\g13_5_soak")
BACKENDS = ["tsr_device", "dlss_sr", "fsr_3_1_5"]
SOAK_SCENE = "cornell-box"
SOAK_TIER = 67
FRAME_COUNT = 32
MIN_SECONDS = 1800
MIN_ITERATIONS = 3

REGISTRY_TARGETS = [
    ("g13_m_c_ue_upscale_parity", ROOT / "milestones" / "g13" / "g13_ue_upscale_gap_registry.json"),
    ("g13_m_d_ue_lumen_gi_parity", ROOT / "milestones" / "g13" / "g13_ue_lumen_gap_registry.json"),
]

NOTES: list[str] = []


def _note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def base_commit() -> str:
    r = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True)
    return (r.stdout or "").strip() or "unknown"


# ---------------------------------------------------------------- ① 全量回归
def verify_assertion_gate(key: str, prefix: str) -> dict:
    """P0 门核验：wel.require_gate_pass 之上叠加顶层 status=="pass" 字面（缺字段即红）。"""
    row = wel.require_gate_pass(key, prefix)
    path = wel.load_latest_evidence(prefix)
    if path is None:
        row["status"] = "FAIL"
        row["detail"] = f"缺最新 evidence（{prefix}_*.json）"
        return row
    doc = wel.load_json(path)
    if doc.get("status") != "pass":
        row["status"] = "FAIL"
        row["detail"] = f"顶层 status={doc.get('status')!r} ≠ 'pass'（缺字段即红面）"
    return row


def run_regression(*, skip_rerun: bool = False) -> tuple[bool, list[dict], str, bool]:
    """全量回归（5 P0 + 4 聚合/决策门）。口径沿 G12.7a run_regression 同构。"""
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
                [sys.executable, str(script), "--gate", gate_arg],
                cwd=ROOT,
            )
            if r.returncode != 0:
                rows.append({
                    "symbolic_gate_key": key, "subject_prefix": prefix,
                    "evidence_path": None, "status": "FAIL",
                    "detail": f"smoke exit={r.returncode}",
                })
                all_ok = False
                continue
        if is_aggregate:
            row = wel.require_gate_pass(key, prefix)
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


# ---------------------------------------------------------------- ② 超分链路 soak
def _render_leg(backend: str, anchors: dict) -> str | None:
    """出图腿：三后端轮转 32 帧 Halton 序列；converged_digest == 首轮锚（位级复现）。"""
    out_root = SOAK_ROOT / backend
    cmd = [
        str(RURIX_BIN), "--render", "--scene", SOAK_SCENE, "--tier", str(SOAK_TIER),
        "--backend", backend, "--frames", str(FRAME_COUNT),
        "--out-root", str(out_root),
    ]
    import os
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env, timeout=7200)
    if r.returncode != 0:
        return f"出图腿 {backend} exit={r.returncode}: {(r.stdout + r.stderr)[-200:]}"
    receipt_p = out_root / SOAK_SCENE / f"tier{SOAK_TIER}" / backend / "render_receipt.json"
    if not receipt_p.is_file():
        return f"出图腿 {backend} receipt 缺失"
    rec = json.loads(receipt_p.read_text(encoding="utf-8"))
    dig = rec.get("converged_digest", "")
    if not dig.startswith("sha256:"):
        return f"出图腿 {backend} converged_digest 形态异常: {dig!r}"
    anchor = anchors.setdefault(backend, dig)
    if dig != anchor:
        return f"出图腿 {backend} digest 漂移检出（M165 同型监控面）: {dig} ≠ {anchor}"
    return None


def _registry_leg(idx: int) -> str | None:
    """登记表装配腿：最新 evidence 超容差格 ↔ 在树登记表行集幂等复核。"""
    subject, reg_path = REGISTRY_TARGETS[idx % len(REGISTRY_TARGETS)]
    path = wel.load_latest_evidence(subject)
    if path is None:
        return f"登记表装配腿 {subject} 缺最新 evidence"
    if not reg_path.is_file():
        return f"登记表装配腿 {reg_path.name} 在树缺失"
    ev = wel.load_json(path)
    checks = ev.get("checks") or {}
    if not (checks.get("gap_registry_schema_valid") and checks.get("gap_registry_reconciled")):
        return f"登记表装配腿 {subject} 登记双面非真"
    reg = json.loads(reg_path.read_text(encoding="utf-8"))
    items = reg.get("items") or []
    over = 0
    parity = ev.get("parity") or {}
    for c in parity.get("cells") or []:
        over += 1 if c.get("over_tolerance") else 0
    for n in parity.get("noise_spectrum") or []:
        over += 1 if n.get("over_tolerance") else 0
    if len(items) != over:
        return f"登记表装配腿 {subject} 行数={len(items)} ≠ evidence 超容差格={over}"
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
    if int(raw.get("upscale_frames_rendered") or 0) == 0:
        problems.append("出图帧计数为空")
    if int(raw.get("registry_assemblies") or 0) == 0:
        problems.append("登记表装配计数为空")
    return not problems, problems


def run_chain_soak() -> tuple[bool, dict]:
    """超分链路连续复跑 soak：≥1800s 墙钟逐迭代真跑，零 sleep，零失败。"""
    counters = {
        "iterations": 0,
        "failures": 0,
        "upscale_frames_rendered": 0,
        "registry_assemblies": 0,
    }
    problems: list[str] = []
    rb = subprocess.run(
        ["cargo", "build", "--release", "-p", "rurix-render",
         "--bin", "g13_4_ue_upscale_parity_render", "--features", "vendor-upscale"],
        cwd=ROOT, timeout=7200, capture_output=True, text=True,
    )
    if rb.returncode != 0 or not RURIX_BIN.is_file():
        tail = ((rb.stderr or "") + (rb.stdout or "")).strip()[-400:]
        problems.append(f"soak 前置构建失败（release g13_4_ue_upscale_parity_render）: rc={rb.returncode} {tail}")
        return False, {"ok": False, "detail": f"problems={problems}", "raw": {}, "counters": counters}
    anchors: dict = {}
    SOAK_ROOT.mkdir(parents=True, exist_ok=True)
    outer0 = time.time()
    with gpu_device_lock(purpose="g13.5a upscale-chain soak"):
        t0 = time.time()
        active = 0.0
        while time.time() - t0 < MIN_SECONDS and not problems:
            it0 = time.time()
            backend = BACKENDS[counters["iterations"] % len(BACKENDS)]
            p = _render_leg(backend, anchors)
            if p:
                problems.append(p)
            else:
                counters["upscale_frames_rendered"] += FRAME_COUNT
                p = _registry_leg(counters["iterations"])
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
                f"(elapsed {time.time() - t0:.1f}s/{MIN_SECONDS}s)",
                flush=True,
            )
    seconds = time.time() - t0
    outer = time.time() - outer0
    raw = {
        "soak_subject": "upscale-chain-soak",
        "iterations": counters["iterations"],
        "seconds": seconds,
        "min_iterations": MIN_ITERATIONS,
        "min_seconds": MIN_SECONDS,
        "active_chain_seconds": active,
        "sleep_seconds": 0.0,
        "outer_wall_seconds": outer,
        "failures": counters["failures"],
        "upscale_frames_rendered": counters["upscale_frames_rendered"],
        "registry_assemblies": counters["registry_assemblies"],
        "backend_digest_anchors": anchors,
    }
    ok, judge_problems = judge_chain_soak(raw, outer_elapsed=outer)
    problems.extend(judge_problems)
    detail = (
        f"iterations={counters['iterations']} seconds={seconds:.1f} active={active:.1f} "
        f"outer={outer:.1f} sleep=0.0 failures={counters['failures']} "
        f"frames={counters['upscale_frames_rendered']} registries={counters['registry_assemblies']} "
        f"subject='upscale-chain-soak'"
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

    _note("① 全量回归（5 P0 真跑 + 4 聚合/决策门核验）…")
    reg_ok, gate_rows, commit, base_uniform = run_regression()

    _note("② 超分链路连续复跑 soak（≥1800s）…")
    soak_ok, soak_info = run_chain_soak()

    _note("③ budget_eval --strict…")
    bud_ok, bud_detail = run_budget_strict()

    _note("④ G5~G12 0-byte + 日期锚…")
    leg_ok, leg_detail = git_closed_surface()

    raw = soak_info.get("raw") or {}
    no_sleep_ok = raw.get("sleep_seconds") == 0.0
    facts = [
        _fact("regression_gates_all_pass", reg_ok,
              "5 P0 真跑 + 4 聚合/决策门核验全绿" if reg_ok else "回归面见 required_gates 行集"),
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
            "G13.5a full soak；四腿 + G5~G12 既有判据 0-byte fact；诚实语义（沿 "
            "G12.7a/G11.7a/G10.8a/G9.8a 口径）：soak 墙钟=真实超分链路复跑实测"
            "（禁 sleep 充时，sleep_seconds 恒 0，active_chain_seconds 逐迭代计时求和，"
            "gate 外测墙钟交叉核验）；soak 载体=G13 超分链路二面（出图 "
            "g13_4_ue_upscale_parity_render --render cornell-box tier67 三后端轮转，"
            "receipt converged_digest == 首轮锚位级复现——跨迭代 digest 漂移即 M165 "
            "同型监控面检出 → 登记表装配最新 M-c/M-d evidence 超容差格与在树 "
            "g13_ue_{upscale,lumen}_gap_registry.json 行集对账幂等复核）；"
            "subject=upscale-chain-soak 无 validation/device_lost 字面量 0 硬门"
            "（device 面归回归腿 5 门本体）；5 门 evidence base_commit 同值一致"
            "（同一候选 close-out 基线）"
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
        "soak_subject": "upscale-chain-soak",
        "iterations": 200, "seconds": 1801.0, "min_iterations": MIN_ITERATIONS,
        "min_seconds": MIN_SECONDS, "active_chain_seconds": 1798.0,
        "sleep_seconds": 0.0, "outer_wall_seconds": 1803.0, "failures": 0,
        "upscale_frames_rendered": 6400, "registry_assemblies": 200,
        "backend_digest_anchors": {"tsr_device": "sha256:" + "0" * 64},
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
        ("出图计数空", {"upscale_frames_rendered": 0}, 1803.0),
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
    return run_full_gate()


if __name__ == "__main__":
    sys.exit(main())
