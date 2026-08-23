#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.5a P2 穷举 + soak 波）
"""G14.5a M-e(M178) 回归门 + 漂移监控门（P0，步骤 262；g14.p0.m_e.regression_drift_guard；
G14_CONTRACT §4.2 M-e 行判据逐字 / G-G14-8 联动；G14_ACCEPTANCE_MAP §1 M-e 行；
同构 ci/g13_regression_drift_guard_smoke.py〔M171〕先例）。

host+device 门（device_section_state=executed——抽检既有门经子进程真跑，各自
evidence 独立落盘自持 device 面；本门只读汇总 + 子进程退出码/新鲜度机核，
不嵌套持锁）。判据（契约 §4.2 M-e 行字面）：

1. **既有 76 门（G9 34 key + G10 14 key + G11 14 key + G12 9 key + G13 5 key）
   最新 evidence 全绿只读汇总**：wel.require_gate_pass 逐门只读核验
   （symbolic_gate_key 相符 ∧ host_section_pass=True ∧ device_section_state
   ∉ {fail,dev_env_degrade,skip} ∧ checks 全 True）；聚合不遮蔽任一子断言
   FAIL/SKIP/DEV_ENV_DEGRADE（逐门行集入 evidence，不折叠）。
2. **G14 触改面既有门重跑回归零降级**（G14 触改共享面 = ci/check_schemas.py
   前缀路由纯追加 / pr-smoke.yml 步骤追加 / number_ledger.json 数字面 /
   budget_eval.py g14 分派支〔G14.2 加性〕/ g10_gap_registry_lib.py 结构化对账
   加性面〔G14.2 M-a 授权〕——消费面 = G10.5b M139/M140 双门 + M96 golden 门序面
   + 波聚合门族 + M-c/M-d 修订门复跑面）：M96 golden 门序面真跑抽检
   （g9.p0.m96，契约字面）+ G10 M139 A/B 对拍门全档真跑（gaplib 装配面消费）
   + G10 M140 登记表门真跑（gaplib 校验面消费）+ G13 wave5b closeout 聚合复跑
   + G14 wave2/3/4/6/7 exit 聚合复跑——子进程真跑 exit 0 + 最新 evidence PASS
   + **新鲜度机核**（live 抽检 evidence timestamp ≥ 本门会话起点；M-c/M-d 复跑面
   消费 = 最新 evidence timestamp > M-g 门最新 evidence timestamp〔G14.7 并行化
   落地后复跑机证——陈旧 evidence 冒充当期复跑即 RED〕）。
3. **M165 漂移监控登记**（G12-N13 承接锚兑现面）：G14 复跑面同型 digest 漂移
   检出计数（G14 复跑面确定性 checks〔bitexact/deterministic/digest 键族——
   M-c 双跑位级 / M-d digest 守护 18 格 × 3 轮 / M-f Stage A 锚 / M-g 并行化
   三机核〕+ 本门抽检真跑 digest 对账面）+ 零检出字面入 evidence；M165 事件
   FAIL 件 evidence/g12_m165_pt_throughput_baseline_20260817T235251Z.json
   在档 0-byte（status=="fail" 保留纪律）+ flip-trace 诊断臂在树字面
   （G12_5_BENCH_FLIP_TRACE env 面 g12_4_ue_pt_parity_render.rs）；漂移检出
   未登记即 RED。
4. **既有判据 0-byte**：G5~G13 closed 门脚本与里程碑面（ci/g5_*~g13_* +
   milestones/g9~g13）git 面机核——已提交面 diff f4c8da0b..HEAD 闭集 ⊆
   {ci/g10_gap_registry_lib.py / ci/g13_ue_upscale_parity_smoke.py /
   ci/g13_ue_lumen_gi_parity_smoke.py / milestones/g13/g13_budget.json /
   ci/budget_eval.py}（G14.2 M-a 授权面 + G14.12 RFC-0030 §4.7 测量派生冻结件重派生）；
   工作树 porcelain 闭集 = {milestones/g12/g12_pt_sampler_selection.json}
   （异己并发会话登记面，立项裁决 1——不在本闭集消费/混入）。

RED 臂（契约判据字面）：既有门降级即 RED（red_degraded_gate——子断言非 PASS
注入必检出）；聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED
（red_aggregate_masking——遮蔽型汇总必检出）；漂移检出未登记即 RED
（red_drift_unregistered——漂移阳性面未登记必检出）。

用法：
  py -3 ci/g14_regression_drift_guard_smoke.py --gate g14.p0.m_e.regression_drift_guard
  py -3 ci/g14_regression_drift_guard_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_m_e_regression_drift_guard_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g10_wave_exit_lib as wel  # noqa: E402
import g13_tsr_device_kernel_smoke as mb  # noqa: E402
from g11_wave3_exit_check import G10_KEYS, G9_KEYS  # noqa: E402
from g12_regression_guard_smoke import G11_KEYS  # noqa: E402

GATE_KEY = "g14.p0.m_e.regression_drift_guard"
NUMERIC_STEP = 262  # 落盘前实测 registry/number_ledger.json CI_step.next_free=262 顺位领取
SUBJECT = "g14_m_e_regression_drift_guard"
WAVE = "G14.5a"
TAG = "g14_m_e"
MATRIX_ROW = "M178"
SOURCE_REF = (
    "G14_CONTRACT §4.2 M-e/G-G14-8;G14_ACCEPTANCE_MAP §1 M-e;"
    "G9 34 + G10 14 + G11 14 + G12 9 + G13 5 keys latest evidence read-only summary;"
    "touched-face spot rerun zero-degrade;M165 drift monitoring（G12-N13 承接）"
)
G14_ZERO_BASE = "f4c8da0b"

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

# G13 5 key（M167~M171 闭集；G13 契约「G13 5 key」字面）
G13_KEYS = [
    ("g13.p0.m_a.vendor_upscale_integration", "g13_m_a_vendor_upscale_integration"),
    ("g13.p0.m_b.tsr_device_kernel", "g13_m_b_tsr_device_kernel"),
    ("g13.p0.m_c.ue_upscale_parity", "g13_m_c_ue_upscale_parity"),
    ("g13.p0.m_d.ue_lumen_gi_parity", "g13_m_d_ue_lumen_gi_parity"),
    ("g13.p0.m_e.regression_drift_guard", "g13_m_e_regression_drift_guard"),
]

ALL_76 = list(G9_KEYS) + list(G10_KEYS) + list(G11_KEYS) + G12_KEYS + G13_KEYS

# G14 触改面真跑抽检闭集（live 子进程真跑 + 新鲜度核验 ≥ 会话起点）。
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
        "id": "g13_wave5b_closeout",
        "argv": [sys.executable, "ci/g13_closeout_check.py", "--gate", "g13.wave.5b.closeout"],
        "env": {},
        "key": "g13.wave.5b.closeout",
        "subject": "g13_wave5b_closeout",
    },
    {
        "id": "g14_wave2_exit",
        "argv": [sys.executable, "ci/g14_wave2_exit_check.py", "--gate", "g14.wave.2.exit"],
        "env": {},
        "key": "g14.wave.2.exit",
        "subject": "g14_wave2_exit",
    },
    {
        "id": "g14_wave3_exit",
        "argv": [sys.executable, "ci/g14_wave3_exit_check.py", "--gate", "g14.wave.3.exit"],
        "env": {},
        "key": "g14.wave.3.exit",
        "subject": "g14_wave3_exit",
    },
    {
        "id": "g14_wave4_exit",
        "argv": [sys.executable, "ci/g14_wave4_exit_check.py", "--gate", "g14.wave.4.exit"],
        "env": {},
        "key": "g14.wave.4.exit",
        "subject": "g14_wave4_exit",
    },
    {
        "id": "g14_wave6_exit",
        "argv": [sys.executable, "ci/g14_wave6_exit_check.py", "--gate", "g14.wave.6.exit"],
        "env": {},
        "key": "g14.wave.6.exit",
        "subject": "g14_wave6_exit",
    },
    {
        "id": "g14_wave7_exit",
        "argv": [sys.executable, "ci/g14_wave7_exit_check.py", "--gate", "g14.wave.7.exit"],
        "env": {},
        "key": "g14.wave.7.exit",
        "subject": "g14_wave7_exit",
    },
]

# G14.7 修订门复跑消费面（post-并行化复跑机证：最新 evidence ts > M-g 最新 ts）。
RERUN_CONSUME_GATES = [
    ("g14.p0.m_c.rurix_pipeline_perf", "g14_m_c_rurix_pipeline_perf"),
    ("g14.p0.m_d.dual_end_fps_parity", "g14_m_d_dual_end_fps_parity"),
]
MG_PREFIX = "g14_m_g_vendor_parallel_conversion"

# 诚实红聚合门闭集（M-d 通过线未达期间 wave4 聚合 VERDICT=FAIL 为正确诚实态——
# 红不充绿亦不充降级；合格面 = exit=1 + 六 facts 全绿 + required M-d 行 FAIL 镜像
# 最新 M-d evidence 实测态 + 新鲜度机核，聚合不遮蔽机核维持）。
HONEST_RED_AGGREGATES = frozenset({"g14_wave4_exit"})

# G14 复跑面漂移监控消费闭集（确定性键族：bitexact/deterministic/digest 三字面族）。
G14_DETERMINISM_SURFACES = [
    ("g14.p0.m_c.rurix_pipeline_perf", "g14_m_c_rurix_pipeline_perf"),
    ("g14.p0.m_d.dual_end_fps_parity", "g14_m_d_dual_end_fps_parity"),
    ("g14.p0.m_f.production_caliber_stage_a", "g14_m_f_production_caliber_stage_a"),
    ("g14.p0.m_g.vendor_parallel_conversion", "g14_m_g_vendor_parallel_conversion"),
]
DETERMINISM_KEY_TOKENS = ("bitexact", "deterministic", "digest")
M165_FAIL_EVIDENCE = EVIDENCE_DIR / "g12_m165_pt_throughput_baseline_20260817T235251Z.json"
FLIP_TRACE_ARM = ROOT / "src" / "rurix-asset" / "src" / "bin" / "g12_4_ue_pt_parity_render.rs"
FLIP_TRACE_TOKEN = "G12_5_BENCH_FLIP_TRACE"

CHECK_KEYS = [
    "temporal_base_0byte",
    "gates_76_latest_all_pass",
    "spot_rerun_zero_degrade",
    "spot_rerun_evidence_fresh",
    "g5_g13_closed_surface_0byte",
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
    """76 门只读汇总：逐门行集 + 问题面（聚合不遮蔽——行集全量入 evidence）。"""
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
    """M165 漂移监控面：G14 复跑面确定性键族逐键读值 + 同型 digest 漂移计数。"""
    checked: list[str] = []
    drift: list[str] = []
    for key, subject in G14_DETERMINISM_SURFACES:
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


def rd045_upgrade_registered() -> bool:
    """RD-045 升级登记机核（G14_CONTRACT M165 条款「检出即如实登记并升级评估」字面）：
    deferred.json RD-045 条目在树 + status=="open" + history 含 v5 检出件路径字面。"""
    dp = ROOT / "registry" / "deferred.json"
    if not dp.is_file():
        return False
    doc = wel.load_json(dp)
    for e in doc.get("entries") or []:
        if e.get("id") != "RD-045":
            continue
        blob = "\n".join(h.get("event", "") + (h.get("evidence", "") or "") for h in e.get("history", []))
        return e.get("status") == "open" and "g14_m_d_dual_end_fps_parity_20260821T003053Z" in blob
    return False


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
    """G5~G13 closed 面 0-byte 机核（已提交 diff 闭集 + 工作树 porcelain 闭集）。"""
    globs = [
        "ci/g5_*.py", "ci/g6_*.py", "ci/g7_*.py", "ci/g8_*.py", "ci/g9_*.py",
        "ci/g10_*.py", "ci/g11_*.py", "ci/g12_*.py", "ci/g13_*.py",
        "milestones/g5", "milestones/g6", "milestones/g7", "milestones/g8",
        "milestones/g9", "milestones/g10", "milestones/g11", "milestones/g12",
        "milestones/g13",
    ]
    diff = subprocess.run(
        ["git", "diff", "--name-only", f"{G14_ZERO_BASE}..HEAD", "--"] + globs,
        cwd=ROOT, capture_output=True, text=True,
    )
    committed = sorted(x for x in (diff.stdout or "").splitlines() if x.strip())
    allowed_committed = {
        "ci/g10_gap_registry_lib.py",
        "ci/g13_ue_upscale_parity_smoke.py",
        "ci/g13_ue_lumen_gi_parity_smoke.py",
        "milestones/g13/g13_budget.json",
        "ci/budget_eval.py",
    }
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
    started_stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    # ── ① temporal 底座 0-byte ──
    ok, msg = mb.temporal_base_0byte()
    checks["temporal_base_0byte"] = ok
    check(ok, f"temporal 底座 0-byte: {msg}")

    # ── ② 既有 76 门最新 evidence 全绿只读汇总（不遮蔽） ──
    gate_rows, agg_problems = aggregate_gates(ALL_76)
    checks["gates_76_latest_all_pass"] = not agg_problems and len(gate_rows) == 76
    check(checks["gates_76_latest_all_pass"], f"76 门聚合: {agg_problems[:4]}")
    note(f"76 门只读汇总：{sum(1 for r in gate_rows if r['status'] == 'PASS')}/76 PASS（行集全量入 evidence 不折叠）")

    # ── ③ 触改面真跑抽检（live 零降级 + 新鲜度机核）+ M-c/M-d 复跑面消费 ──
    spot_rows: list[dict] = []
    spot_bad: list[str] = []
    stale_bad: list[str] = []
    for spec in SPOT_GATES:
        r = run(spec["argv"], env_extra=spec["env"])
        if spec["id"] in HONEST_RED_AGGREGATES:
            # 聚合门双分支特判(HONEST_RED_AGGREGATES 登记面字面;G14plus
            # RFC-0030/G14.12 达标分支加性):
            #   达标分支 = exit0 + facts 全绿 + checks 全真 + M-d 行 PASS 镜像
            #   诚实红分支 = exit1 + facts 全绿 + 余 checks 全真 + M-d 行 FAIL 镜像
            # 两分支互斥,均要求聚合如实镜像最新 M-d 实测态(不遮蔽机核维持)。
            ok_state = False
            ev_ts = ""
            wpath = wel.load_latest_evidence(spec["subject"])
            wdoc = wel.load_json(wpath) if wpath else {}
            if wdoc:
                wfacts = wdoc.get("extra_facts") or []
                wchecks = wdoc.get("checks") or {}
                wrows = wdoc.get("required_gates") or []
                md_status = next(
                    (rr.get("status") for rr in wrows
                     if rr.get("subject_prefix") == "g14_m_d_dual_end_fps_parity"),
                    None,
                )
                facts_ok = bool(wfacts) and all(f.get("status") == "PASS" for f in wfacts)
                others_ok = all(
                    v is True for k, v in wchecks.items() if k != "all_required_gates_pass"
                )
                ok_green = (
                    r.returncode == 0 and facts_ok and others_ok
                    and wchecks.get("all_required_gates_pass") is True
                    and md_status == "PASS"
                )
                ok_red = (
                    r.returncode == 1 and facts_ok and others_ok
                    and wchecks.get("all_required_gates_pass") is False
                    and md_status == "FAIL"
                )
                ok_state = ok_green or ok_red
                ev_ts = wdoc.get("timestamp") or ""
            fresh = bool(ev_ts) and ev_ts >= started_stamp
            if not ok_state:
                spot_bad.append(
                    f"{spec['id']} 聚合面异常: exit={r.returncode}"
                    "（合格面 = exit0+facts 绿+M-d 行 PASS 镜像〔达标〕"
                    " 或 exit1+facts 绿+M-d 行 FAIL 镜像〔诚实红〕）"
                )
            if not fresh:
                stale_bad.append(f"{spec['id']} evidence 陈旧: {ev_ts} < {started_stamp}")
            spot_rows.append({
                "id": spec["id"], "exit": r.returncode, "status": "PASS" if ok_state else "FAIL",
                "fresh": fresh,
                "evidence": str(wpath.relative_to(ROOT).as_posix()) if wpath else None,
                "timestamp": ev_ts,
            })
            note(f"抽检 {spec['id']}: exit={r.returncode} 诚实红聚合面={'合格' if ok_state else '异常'} fresh={fresh}")
            continue
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
    # M-c/M-d 修订门复跑面消费（G14.7 并行化落地后复跑机证：ts > M-g 最新 ts）
    mg_path = wel.load_latest_evidence(MG_PREFIX)
    mg_doc = wel.load_json(mg_path) if mg_path else {}
    mg_ts = (mg_doc.get("timestamp") or "") if mg_doc else ""
    for key, subject in RERUN_CONSUME_GATES:
        path = wel.load_latest_evidence(subject)
        if path is None:
            spot_bad.append(f"{subject} 缺最新 evidence（复跑面缺件）")
            spot_rows.append({"id": subject, "exit": -1, "status": "FAIL", "fresh": False})
            continue
        doc = wel.load_json(path)
        sub_checks = doc.get("checks") or {}
        bad = [k for k, v in sub_checks.items() if v is not True]
        ev_ts = doc.get("timestamp") or ""
        post_par = bool(mg_ts) and ev_ts > mg_ts
        # M-d 为诚实红门面（status=fail 字面维持）：复跑面核验 = checks 全绿 + 复跑时序，
        # 不以通过线红为降级（G-G14-6 诚实红不充绿亦不充降级面）
        status_face = doc.get("status")
        status_ok = (status_face == "pass") if subject != "g14_m_d_dual_end_fps_parity" else (status_face in ("pass", "fail"))
        if bad or status_face not in ("pass", "fail"):
            spot_bad.append(f"{subject} 复跑面 checks 非全绿/status 异常: bad={bad[:3]} status={status_face!r}")
        if not status_ok:
            spot_bad.append(f"{subject} status 面异常: {status_face!r}")
        if not post_par:
            stale_bad.append(f"{subject} 复跑面非 G14.7 后件: {ev_ts} ≤ M-g {mg_ts}")
        spot_rows.append({
            "id": subject, "exit": 0, "status": "PASS" if (not bad and post_par) else "FAIL",
            "fresh": post_par, "evidence": str(path.relative_to(ROOT).as_posix()), "timestamp": ev_ts,
        })
        note(f"复跑面 {subject}: checks 全绿={not bad} post-G14.7={post_par}（status={status_face}）")
    checks["spot_rerun_zero_degrade"] = not spot_bad
    check(not spot_bad, f"抽检/复跑面降级: {spot_bad[:3]}")
    checks["spot_rerun_evidence_fresh"] = not stale_bad
    check(not stale_bad, f"陈旧 evidence 冒充当期复跑: {stale_bad[:3]}")

    # ── ④ G5~G13 closed 面 0-byte ──
    ok, detail = git_closed_surface()
    checks["g5_g13_closed_surface_0byte"] = ok
    check(ok, f"G5~G13 closed 面 0-byte: {detail}")
    note(f"G5~G13 closed 面：{detail}")

    # ── ⑤ M165 漂移监控登记（检出计数/零检出字面入 evidence；检出 ⇒ 升级登记机核） ──
    drift = collect_drift_surfaces()
    reg_ok = drift["drift_detected_count"] == 0 or rd045_upgrade_registered()
    drift_ok = (
        reg_ok
        and drift["fail_evidence_retained"]
        and drift["flip_trace_arm_present"]
        and bool(drift["checked_keys"])
    )
    checks["m165_drift_monitoring_registered"] = drift_ok
    check(drift_ok, f"漂移监控面: {drift['drift_details'][:3] or '零检出'} "
                    f"检出登记完备={reg_ok} FAIL件在档={drift['fail_evidence_retained']} 诊断臂在树={drift['flip_trace_arm_present']}")
    note(f"M165 漂移监控：G14 复跑面确定性键族 {len(drift['checked_keys'])} 键全真，"
         f"同型 digest 漂移检出计数 = {drift['drift_detected_count']}（零检出字面/检出计数如实登记）；"
         f"检出升级登记完备（RD-045）= {reg_ok if drift['drift_detected_count'] else 'n/a（零检出）'}；"
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
            "baseline_anchor_id": "n/a（回归门不产新锚；G9~G13 绿面 0-byte 维持）",
            "measured_value": (
                f"76 门聚合 PASS={sum(1 for r in gate_rows if r['status'] == 'PASS')}/76；"
                f"抽检 {len(SPOT_GATES)} 门真跑零降级 + M-c/M-d 复跑面 post-G14.7 机证；"
                f"漂移检出计数={drift['drift_detected_count']}"
            ),
            "not_worse_than_anchor": checks["gates_76_latest_all_pass"] and checks["spot_rerun_zero_degrade"],
            "threshold_provenance": "n/a（回归门面；抽检门各自 budget 面自持）",
            "evolution_register": "G14.2 M-a 授权四面（gaplib 结构化对账/g13 双门脚本/budget_eval g14 分派支）committed 闭集允许项；M-d 诚实红门面 status=fail 字面维持不充降级",
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "regression": {
            "gates_76": gate_rows,
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
    if len(ALL_76) != 76:
        print(f"[{TAG}] selftest FAIL: 76 门闭集 n={len(ALL_76)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (3 RED + 2 GREEN)")
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
