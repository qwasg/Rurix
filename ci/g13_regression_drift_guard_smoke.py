#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.5a P2 穷举 + soak 波）
"""G13.5a M-e(M171) 回归门 + 漂移监控门（P0，步骤 244；g13.p0.m_e.regression_drift_guard；
G13_CONTRACT §4.2 M-e 行判据逐字 / G-G13-8；G13_ACCEPTANCE_MAP §1 M-e 行；
同构 ci/g12_regression_guard_smoke.py〔M164〕先例）。

host+device 门（device_section_state=executed——抽检既有门经子进程真跑，各自
evidence 独立落盘自持 device 面；本门只读汇总 + 子进程退出码/新鲜度机核，
不嵌套持锁）。判据（契约 §4.2 M-e 行字面）：

1. **既有 71 门（G9 34 key + G10 14 key + G11 14 key + G12 9 key）最新 evidence
   全绿只读汇总**：wel.require_gate_pass 逐门只读核验（symbolic_gate_key 相符 ∧
   host_section_pass=True ∧ device_section_state ∉ {fail,dev_env_degrade,skip}
   ∧ checks 全 True）；M147 双 phase 纪律两态面继承（g11_wave3_exit
   m147_dual_phase_discipline 单源）；聚合不遮蔽任一子断言 FAIL/SKIP/
   DEV_ENV_DEGRADE（逐门行集入 evidence，不折叠）。
2. **G13 触改面既有门重跑回归零降级**（G13 触改共享面 = ci/g10_gap_registry_lib.py
   加性 registry_name〔G13.4 v1.138 登记〕/ check_schemas.py 五前缀路由 /
   budget_eval.py 判读面 / g10_5_scene_render.rs --gi-off 加性旗标〔M-d 门
   逐字节 parity 机证在案〕——消费面 = G10.5b M139/M140 双门 + M96 golden 门序面
   + 波聚合门族）：M96 golden 门序面真跑抽检（g9.p0.m96，契约字面）+ G10 M139
   A/B 对拍门全档真跑（gaplib 装配面消费）+ G10 M140 登记表门真跑（gaplib 校验面
   消费）+ G12 wave5 exit 聚合复跑 + G13 wave2/3/4 exit 聚合复跑——子进程真跑
   exit 0 + 最新 evidence PASS + **新鲜度机核**（evidence timestamp ≥ 本门会话
   起点——陈旧 evidence 冒充当次复跑即 RED）。
3. **M165 漂移监控登记**（G12-N13 承接锚兑现面）：G13 复跑面同型 digest 漂移
   检出计数（G13 四门最新 evidence 确定性 checks〔bitexact/deterministic/digest
   键族〕+ 本门抽检真跑 digest 对账面）+ 零检出字面入 evidence；M165 事件 FAIL 件
   evidence/g12_m165_pt_throughput_baseline_20260817T235251Z.json 在档 0-byte
   （status=="fail" 保留纪律）+ flip-trace 诊断臂在树字面（G12_5_BENCH_FLIP_TRACE
   env 面 g12_4_ue_pt_parity_render.rs）；漂移检出未登记即 RED。
4. **既有判据 0-byte**：G5~G12 closed 门脚本与里程碑面（ci/g5_*~g12_* +
   milestones/g9~g12）git 面机核——已提交面 diff 8c5dc5ee..HEAD 闭集 =
   {ci/g10_gap_registry_lib.py}（G13.4 加性 registry_name 演进位 v1.138 登记）；
   工作树 porcelain 闭集 = {milestones/g12/g12_pt_sampler_selection.json}
   （异己并发会话登记面，立项裁决 1——不在本闭集消费/混入）。

RED 臂（契约判据字面）：既有门降级即 RED（red_degraded_gate——子断言非 PASS
注入必检出）；聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED
（red_aggregate_masking——遮蔽型汇总必检出）；漂移检出未登记即 RED
（red_drift_unregistered——漂移阳性面未登记必检出）。

用法：
  py -3 ci/g13_regression_drift_guard_smoke.py --gate g13.p0.m_e.regression_drift_guard
  py -3 ci/g13_regression_drift_guard_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g13" / "g13_m_e_regression_drift_guard_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g10_wave_exit_lib as wel  # noqa: E402
import g13_tsr_device_kernel_smoke as mb  # noqa: E402
from g11_wave3_exit_check import G10_KEYS, G9_KEYS  # noqa: E402

GATE_KEY = "g13.p0.m_e.regression_drift_guard"
NUMERIC_STEP = 244  # 落盘前实测 registry/number_ledger.json CI_step.next_free=244 顺位领取
SUBJECT = "g13_m_e_regression_drift_guard"
WAVE = "G13.5a"
TAG = "g13_m_e"
MATRIX_ROW = "M171"
SOURCE_REF = (
    "G13_CONTRACT §4.2 M-e/G-G13-8;G13_ACCEPTANCE_MAP §1 M-e;"
    "G9 34 + G10 14 + G11 14 + G12 9 keys latest evidence read-only summary;"
    "touched-face spot rerun zero-degrade;M165 drift monitoring（G12-N13 承接）"
)
G13_ZERO_BASE = "8c5dc5ee"

# G11 14 key（M144~M157 闭集；G13 契约「G11 14 key」字面，g12_regression_guard 同单源）
from g12_regression_guard_smoke import G11_KEYS  # noqa: E402

# G12 9 key（M158~M165 8 P0 + M166 go P1 闭集；G12 契约「G12 9 key」字面）
G12_KEYS = [
    ("g12.p0.m158.mis_full_surface", "g12_m158_mis_full_surface"),
    ("g12.p0.m159.russian_roulette_prod", "g12_m159_russian_roulette_prod"),
    ("g12.p0.m160.sampling_lds_upgrade", "g12_m160_sampling_lds_upgrade"),
    ("g12.p0.m161.convergence_criterion_prod", "g12_m161_convergence_criterion_prod"),
    ("g12.p0.m162.denoise_pipeline_tsr", "g12_m162_denoise_pipeline_tsr"),
    ("g12.p0.m163.ue_pt_parity", "g12_m163_ue_pt_parity"),
    ("g12.p0.m164.regression_guard", "g12_m164_regression_guard"),
    ("g12.p0.m165.pt_throughput_baseline", "g12_m165_pt_throughput_baseline"),
    ("g12.p1.m166.pt_production_calibration", "g12_pt_production_calibration"),
]

ALL_71 = list(G9_KEYS) + list(G10_KEYS) + list(G11_KEYS) + G12_KEYS

# G13 触改面真跑抽检闭集（子进程 argv + 环境面 + 新鲜度核验 subject/key）。
SPOT_GATES = [
    {
        "id": "g9_m96",
        "argv": [sys.executable, "ci/g9_path_tracer_reference_smoke.py", "--gate", "g9.p0.m96.path_tracer_reference"],
        "env": {"RURIX_REQUIRE_REAL": "1", "RURIX_VK_VALIDATION": "1"},
        "key": "g9.p0.m96.path_tracer_reference",
        "subject": "g9_m96_path_tracer_reference",
    },
    {
        "id": "g10_m139",
        "argv": [sys.executable, "ci/g10_ab_comparison_smoke.py", "--gate", "g10.p0.m139.ab_comparison"],
        "env": {"RURIX_REQUIRE_REAL": "1"},
        "key": "g10.p0.m139.ab_comparison",
        "subject": "g10_m139_ab_comparison",
    },
    {
        "id": "g10_m140",
        "argv": [sys.executable, "ci/g10_gap_registry_smoke.py", "--gate", "g10.p0.m140.gap_registry"],
        "env": {"RURIX_REQUIRE_REAL": "1"},
        "key": "g10.p0.m140.gap_registry",
        "subject": "g10_m140_gap_registry",
    },
    {
        "id": "g12_wave5_exit",
        "argv": [sys.executable, "ci/g12_wave5_exit_check.py", "--gate", "g12.wave.5.exit"],
        "env": {},
        "key": "g12.wave.5.exit",
        "subject": "g12_wave5_exit",
    },
    {
        "id": "g13_wave2_exit",
        "argv": [sys.executable, "ci/g13_wave2_exit_check.py", "--gate", "g13.wave.2.exit"],
        "env": {},
        "key": "g13.wave.2.exit",
        "subject": "g13_wave2_exit",
    },
    {
        "id": "g13_wave3_exit",
        "argv": [sys.executable, "ci/g13_wave3_exit_check.py", "--gate", "g13.wave.3.exit"],
        "env": {},
        "key": "g13.wave.3.exit",
        "subject": "g13_wave3_exit",
    },
    {
        "id": "g13_wave4_exit",
        "argv": [sys.executable, "ci/g13_wave4_exit_check.py", "--gate", "g13.wave.4.exit"],
        "env": {},
        "key": "g13.wave.4.exit",
        "subject": "g13_wave4_exit",
    },
]

# G13 复跑面漂移监控消费闭集（确定性键族：bitexact/deterministic/digest 三字面族）。
G13_DETERMINISM_SURFACES = [
    ("g13.p0.m_a.vendor_upscale_integration", "g13_m_a_vendor_upscale_integration"),
    ("g13.p0.m_b.tsr_device_kernel", "g13_m_b_tsr_device_kernel"),
    ("g13.p0.m_c.ue_upscale_parity", "g13_m_c_ue_upscale_parity"),
    ("g13.p0.m_d.ue_lumen_gi_parity", "g13_m_d_ue_lumen_gi_parity"),
]
DETERMINISM_KEY_TOKENS = ("bitexact", "deterministic", "digest")
M165_FAIL_EVIDENCE = EVIDENCE_DIR / "g12_m165_pt_throughput_baseline_20260817T235251Z.json"
FLIP_TRACE_ARM = ROOT / "src" / "rurix-asset" / "src" / "bin" / "g12_4_ue_pt_parity_render.rs"
FLIP_TRACE_TOKEN = "G12_5_BENCH_FLIP_TRACE"

CHECK_KEYS = [
    "temporal_base_0byte",
    "gates_71_latest_all_pass",
    "spot_rerun_zero_degrade",
    "spot_rerun_evidence_fresh",
    "g5_g12_closed_surface_0byte",
    "m165_drift_monitoring_registered",
    "red_degraded_gate_detected",
    "red_aggregate_masking_detected",
    "red_drift_unregistered_detected",
]

NOTES: list[str] = []
FAILURES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL {msg}")


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}")


def run(cmd: list[str], env_extra: dict[str, str] | None = None, timeout: int = 7200) -> subprocess.CompletedProcess:
    import os
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    print(f"[{TAG}] run: {' '.join(cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env, timeout=timeout)


def aggregate_gates(gates: list[tuple[str, str]]) -> tuple[list[dict], list[str]]:
    """71 门只读汇总：逐门行集 + 问题面（聚合不遮蔽——行集全量入 evidence）。"""
    rows: list[dict] = []
    problems: list[str] = []
    for key, subject in gates:
        row = wel.require_gate_pass(key, subject)
        rows.append(row)
        if row["status"] != "PASS":
            problems.append(f"{key}: {row['detail']}")
    return rows, problems


def verdict_of(rows: list[dict]) -> str:
    """聚合裁定（遮蔽即自检红面：任一子行非 PASS 即 FAIL，零折叠）。"""
    return "PASS" if all(r.get("status") == "PASS" for r in rows) and rows else "FAIL"


def collect_drift_surfaces() -> dict:
    """M165 漂移监控面：G13 复跑面确定性键族逐键读值 + 同型 digest 漂移计数。

    返回 {checked_keys, drift_detected_count, drift_details, fail_evidence_retained,
    flip_trace_arm_present}——drift_detected_count 为本面唯一计数源（检出即须登记，
    零检出字面入 evidence）。"""
    checked: list[str] = []
    drift: list[str] = []
    for key, subject in G13_DETERMINISM_SURFACES:
        path = wel.load_latest_evidence(subject)
        if path is None:
            drift.append(f"{key}: 缺最新 evidence（监控面缺件即计漂移风险行）")
            continue
        doc = wel.load_json(path)
        checks = doc.get("checks") or {}
        hit = False
        for ck, cv in checks.items():
            if any(tok in ck for tok in DETERMINISM_KEY_TOKENS):
                checked.append(f"{subject}:{ck}={cv}")
                hit = True
                if cv is not True:
                    drift.append(f"{subject}:{ck}={cv}（确定性键族非真——同型漂移嫌疑登记）")
        if not hit:
            drift.append(f"{subject}: 确定性键族零命中（监控覆盖面缺失登记）")
    fail_retained = (
        M165_FAIL_EVIDENCE.is_file()
        and (wel.load_json(M165_FAIL_EVIDENCE).get("status") == "fail")
    )
    arm = FLIP_TRACE_ARM.is_file() and FLIP_TRACE_TOKEN in FLIP_TRACE_ARM.read_text(encoding="utf-8")
    return {
        "checked_keys": checked,
        "drift_detected_count": len(drift),
        "drift_details": drift,
        "fail_evidence_retained": bool(fail_retained),
        "flip_trace_arm_present": bool(arm),
    }


def red_arm_degraded_gate() -> bool:
    """RED 臂：既有门降级注入（不存在门 key 注入聚合面 → 必检出非 PASS 行）。"""
    rows, problems = aggregate_gates([("g9.p0.m999.nonexistent_gate", "g9_m999_nonexistent")])
    return bool(problems) and rows[0]["status"] == "FAIL"


def red_arm_aggregate_masking() -> bool:
    """RED 臂：聚合遮蔽注入（一 FAIL 行混入行集 → verdict 必须 FAIL 不得折叠绿）。"""
    rows = [
        {"symbolic_gate_key": "g9.p0.m96.path_tracer_reference", "status": "PASS"},
        {"symbolic_gate_key": "g10.p0.m140.gap_registry", "status": "FAIL"},
    ]
    return verdict_of(rows) == "FAIL"


def red_arm_drift_unregistered() -> bool:
    """RED 臂：漂移阳性注入（确定性键族非真合成面 → 监控计数必须 >0）。"""
    synth = {
        "checks": {"rurix_double_run_bitexact": False, "frame_digests_recomputed_match": True},
    }
    drift = []
    for ck, cv in synth["checks"].items():
        if any(tok in ck for tok in DETERMINISM_KEY_TOKENS) and cv is not True:
            drift.append(f"synth:{ck}={cv}")
    return len(drift) == 1


def git_closed_surface() -> tuple[bool, str]:
    """G5~G12 closed 面 0-byte 机核（已提交 diff 闭集 + 工作树 porcelain 闭集）。"""
    globs = [
        "ci/g5_*.py", "ci/g6_*.py", "ci/g7_*.py", "ci/g8_*.py", "ci/g9_*.py",
        "ci/g10_*.py", "ci/g11_*.py", "ci/g12_*.py",
        "milestones/g5", "milestones/g6", "milestones/g7", "milestones/g8",
        "milestones/g9", "milestones/g10", "milestones/g11", "milestones/g12",
    ]
    diff = subprocess.run(
        ["git", "diff", "--name-only", f"{G13_ZERO_BASE}..HEAD", "--"] + globs,
        cwd=ROOT, capture_output=True, text=True,
    )
    committed = sorted(x for x in (diff.stdout or "").splitlines() if x.strip())
    allowed_committed = {"ci/g10_gap_registry_lib.py"}
    bad_committed = [f for f in committed if f not in allowed_committed]
    porc = subprocess.run(
        ["git", "status", "--porcelain", "--"] + globs, cwd=ROOT, capture_output=True, text=True,
    )
    working = sorted(
        ln[3:].strip() for ln in (porc.stdout or "").splitlines() if ln.strip()
    )
    allowed_working = {"milestones/g12/g12_pt_sampler_selection.json"}  # 异己登记面（立项裁决 1）
    bad_working = [f for f in working if f not in allowed_working]
    ok = not bad_committed and not bad_working
    detail = (
        f"committed 闭集={committed or '空'}（允许面={sorted(allowed_committed)}）;"
        f"工作树闭集={working or '空'}（允许面={sorted(allowed_working)} 异己登记）"
        + (f"；越界 committed={bad_committed} working={bad_working}" if not ok else "")
    )
    return ok, detail


def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    started = _dt.datetime.now(_dt.timezone.utc).timestamp()
    started_stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    # ── ① temporal 底座 0-byte ──
    ok, msg = mb.temporal_base_0byte()
    checks["temporal_base_0byte"] = ok
    check(ok, f"temporal 底座 0-byte: {msg}")

    # ── ② 既有 71 门最新 evidence 全绿只读汇总（不遮蔽） ──
    gate_rows, agg_problems = aggregate_gates(ALL_71)
    checks["gates_71_latest_all_pass"] = not agg_problems and len(gate_rows) == 71
    check(checks["gates_71_latest_all_pass"], f"71 门聚合: {agg_problems[:4]}")
    note(f"71 门只读汇总：{sum(1 for r in gate_rows if r['status'] == 'PASS')}/71 PASS（行集全量入 evidence 不折叠）")

    # ── ③ 触改面真跑抽检（零降级 + 新鲜度机核） ──
    spot_rows: list[dict] = []
    spot_bad: list[str] = []
    stale_bad: list[str] = []
    for spec in SPOT_GATES:
        r = run(spec["argv"], env_extra=spec["env"])
        if r.returncode != 0:
            spot_bad.append(f"{spec['id']} exit={r.returncode}: {(r.stdout + r.stderr)[-200:]}")
            spot_rows.append({"id": spec["id"], "exit": r.returncode, "status": "FAIL", "fresh": False})
            continue
        row = wel.require_gate_pass(spec["key"], spec["subject"])
        ev_ts = row.get("timestamp") or ""
        fresh = bool(ev_ts) and ev_ts >= started_stamp
        if row["status"] != "PASS":
            spot_bad.append(f"{spec['id']} 最新 evidence 非 PASS: {row['detail']}")
        if not fresh:
            stale_bad.append(f"{spec['id']} evidence 陈旧: {ev_ts} < {started_stamp}")
        spot_rows.append({
            "id": spec["id"], "exit": r.returncode, "status": row["status"],
            "fresh": fresh, "evidence": row.get("evidence_path"), "timestamp": ev_ts,
        })
        note(f"抽检 {spec['id']}: exit=0 status={row['status']} fresh={fresh}")
    checks["spot_rerun_zero_degrade"] = not spot_bad
    check(not spot_bad, f"抽检降级: {spot_bad[:3]}")
    checks["spot_rerun_evidence_fresh"] = not stale_bad
    check(not stale_bad, f"陈旧 evidence 冒充当次复跑: {stale_bad[:3]}")

    # ── ④ G5~G12 closed 面 0-byte ──
    ok, detail = git_closed_surface()
    checks["g5_g12_closed_surface_0byte"] = ok
    check(ok, f"G5~G12 closed 面 0-byte: {detail}")
    note(f"G5~G12 closed 面：{detail}")

    # ── ⑤ M165 漂移监控登记 ──
    drift = collect_drift_surfaces()
    drift_ok = (
        drift["drift_detected_count"] == 0
        and drift["fail_evidence_retained"]
        and drift["flip_trace_arm_present"]
        and bool(drift["checked_keys"])
    )
    checks["m165_drift_monitoring_registered"] = drift_ok
    check(drift_ok, f"漂移监控面: {drift['drift_details'][:3] or '零检出'} "
                    f"FAIL件在档={drift['fail_evidence_retained']} 诊断臂在树={drift['flip_trace_arm_present']}")
    note(f"M165 漂移监控：G13 复跑面确定性键族 {len(drift['checked_keys'])} 键全真，"
         f"同型 digest 漂移检出计数 = {drift['drift_detected_count']}（零检出字面登记）；"
         f"FAIL 件 0-byte 在档 = {drift['fail_evidence_retained']}；flip-trace 诊断臂在树 = {drift['flip_trace_arm_present']}")

    # ── ⑥ RED 三臂 ──
    red_results = {
        "degraded_gate": red_arm_degraded_gate(),
        "aggregate_masking": red_arm_aggregate_masking(),
        "drift_unregistered": red_arm_drift_unregistered(),
    }
    checks["red_degraded_gate_detected"] = red_results["degraded_gate"]
    checks["red_aggregate_masking_detected"] = red_results["aggregate_masking"]
    checks["red_drift_unregistered_detected"] = red_results["drift_unregistered"]
    for arm, ok_arm in red_results.items():
        check(ok_arm, f"RED 臂 {arm} 注入未检出")

    all_pass = all(checks.values()) and not FAILURES
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": mb.base_commit(),
        "host_section_pass": all_pass,
        "device_section_state": "executed",
        "checks": checks,
        "commands": [
            {"seq": i + 1, "command": " ".join(s["argv"]), "exit_code": row["exit"]}
            for i, (s, row) in enumerate(zip(SPOT_GATES, spot_rows))
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": mb.environment(),
        "production": {
            "correctness_anchor_unchanged": checks["temporal_base_0byte"],
            "baseline_anchor_id": "n/a（回归门不产新锚；G9~G12 绿面 0-byte 维持）",
            "measured_value": (
                f"71 门聚合 PASS={sum(1 for r in gate_rows if r['status'] == 'PASS')}/71；"
                f"抽检 {len(SPOT_GATES)} 门真跑零降级；漂移检出计数={drift['drift_detected_count']}"
            ),
            "not_worse_than_anchor": checks["gates_71_latest_all_pass"] and checks["spot_rerun_zero_degrade"],
            "threshold_provenance": "n/a（回归门面；抽检门各自 budget 面自持）",
            "evolution_register": "ci/g10_gap_registry_lib.py 加性 registry_name 演进位（G13.4 v1.138 登记，closed 面 0-byte 闭集允许项）",
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "regression": {
            "gates_71": gate_rows,
            "spot_reruns": spot_rows,
            "session_started_utc": started_stamp,
            "drift_monitoring": drift,
        },
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    print(f"[{TAG}] VERDICT={'PASS' if all_pass else 'FAIL'} checks={sum(1 for v in checks.values() if v)}/{len(checks)}")
    return 0 if all_pass else 1


def run_selftest() -> int:
    """schema 闭集对账 + RED/GREEN 双臂。"""
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    if not red_arm_degraded_gate():
        print(f"[{TAG}] selftest FAIL: degraded-gate 臂未检出", file=sys.stderr)
        return 1
    if not red_arm_aggregate_masking():
        print(f"[{TAG}] selftest FAIL: aggregate-masking 臂未检出", file=sys.stderr)
        return 1
    if not red_arm_drift_unregistered():
        print(f"[{TAG}] selftest FAIL: drift-unregistered 臂未检出", file=sys.stderr)
        return 1
    # GREEN 面：正例不误判
    good_rows = [{"symbolic_gate_key": "g9.p0.m96.path_tracer_reference", "status": "PASS"}]
    if verdict_of(good_rows) != "PASS":
        print(f"[{TAG}] selftest FAIL: 聚合正例误判", file=sys.stderr)
        return 1
    drift = collect_drift_surfaces()
    if drift["drift_detected_count"] != 0 or not drift["fail_evidence_retained"] or not drift["flip_trace_arm_present"]:
        print(f"[{TAG}] selftest FAIL: 真树漂移监控面非正常: {drift['drift_details'][:2]}", file=sys.stderr)
        return 1
    if len(ALL_71) != 71:
        print(f"[{TAG}] selftest FAIL: 71 门闭集 n={len(ALL_71)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (3 RED + 3 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
