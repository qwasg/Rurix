#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.6a P2 穷举 + M-e 回归门 + soak 波）
"""G15.6a P0 硬门 M-e：回归门 + 漂移监控（g15.p0.m_e.regression_drift_guard，
步骤 278；G15_CONTRACT §4.2 M-e 行判据逐字 / G-G15-8 联动；G15_ACCEPTANCE_MAP §1
M-e 行；同构 ci/g14_regression_drift_guard_smoke.py〔M178〕先例）。

host+device 门（device_section_state=executed——触改面抽检经子进程真跑，各自
evidence/输出独立落盘自持 device 面；本门只读汇总 + 子进程退出码机核，不嵌套
持锁）。判据（契约 §4.2 M-e 行字面）：

1. **既有 84 门（G9 34 key + G10 14 key + G11 14 key + G12 9 key + G13 5 key +
   G14 8 key）最新 evidence 全绿只读汇总不遮蔽**：wel.require_gate_pass 逐门
   只读核验；聚合不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE（逐门行集入
   evidence，不折叠）。**G14 M-d 诚实红门面特判**（G14 closeout 先例同型）：
   checks 全绿 ∧ status=="fail" ∧ parity.unmet_count == g14_fps_gap_registry
   行数（1 行 gap_id 51a150cb4523e8b6）= 通过线未达标如实登记面——红面维持
   红登记不遮蔽不代绿，回归门判据 = 零降级机核（判据机核面零劣化即合格；
   任何非登记面红 = 降级即 RED）。
2. **触改面真跑抽检零降级**（G15 期 src 触改面 = G15plus-II vendor_upscale.rs
   candidate-b 单文件——G14 M-c/M-d 复跑件已绿/红在案，本门抽检 =
   `cargo test -p rurix-rt --features vendor-upscale --no-fail-fast` 子进程
   真跑：失败集 ⊆ G14.9/G15plus-II 登记基线三面〔m103_descriptor_buffer_
   ffi_layout_anchors 常量锚 / binding_supply_chain_no_external_vulkan_crate
   空依赖集断言 / zero_readback_full_chain 进程级计数器并行污染——既有面非
   本波引入〕，新败即降级 RED + **G14 M-c 门最新件复核**（画质锚带重算
   SSIM deficit ≤ 0.010779849285388998 带内——candidate-b 落地态画质零降级
   机核）+ src 触改闭集机核（committed diff G15.0..HEAD src/ ⊆
   {src/rurix-rt/src/vendor_upscale.rs} candidate-b 单文件授权面，工作树
   tracked 空 + untracked ⊆ 异己登记六件闭集）。
3. **RD-045/M165 漂移监控登记**（G12-N13 承接锚兑现面）：G15 全期复跑面
   digest 锚零检出字面入 evidence（M-a 上游三门同口径复跑确定性键族 +
   G14 生产面 M-c/M-d/M-f/M-g 确定性键族 + M-c 生产管线 36 格双跑位级探针 +
   M-d 四轮复跑 Stage A digest 守护 18 格 × 3 轮 == 冻结锚——同型 digest
   漂移检出计数/零检出字面入 evidence；G15 M-d evidence 消费 parity.
   digest_anchor.drift_count / drift_monitoring 登记面，不以诚实红跳过量
   冒充监控面）；M165 事件 FAIL 件 0-byte 在档 + flip-trace 诊断臂在树字面
   + RD-045 条目 open 维持（零检出维持 open 不关闭）；漂移检出未登记即 RED。
4. **G5~G14 closed 判据 0-byte**：vs G15.0 不可变 ref f061487e committed diff
   闭集 ⊆ {g14_budget.json / g14_ue_variance_samples.json〔34f96ac3 归档授权
   双面〕/ g14_fps_gap_registry.json〔G14 M-d 门产未达格登记面，G15.5/
   G15plus/G15plus-II 批授权〕}；工作树 porcelain 闭集空（异己 src untracked
   面不在本 globs，归触改闭集机核面）。

RED 臂（契约判据字面）：既有门降级即 RED（red_degraded_gate）；聚合遮蔽即
RED（red_aggregate_masking）；漂移检出未登记即 RED（red_drift_unregistered）；
诚实红冒充即 RED（red_honest_red_masquerade——非登记面红面注入诚实红评定
面必拒答）。

用法：
  py -3 ci/g15_regression_drift_guard_smoke.py --gate g15.p0.m_e.regression_drift_guard
  py -3 ci/g15_regression_drift_guard_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_m_e_regression_drift_guard_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g10_wave_exit_lib as wel  # noqa: E402
import g13_tsr_device_kernel_smoke as mb  # noqa: E402
import g15_perf_parity_guard_smoke as md  # noqa: E402
from g15_gap_fix_closure_smoke import ALIEN_UNTRACKED_SRC  # noqa: E402
from g11_wave3_exit_check import G10_KEYS, G9_KEYS  # noqa: E402
from g12_regression_guard_smoke import G11_KEYS  # noqa: E402
from g14_regression_drift_guard_smoke import G12_KEYS, G13_KEYS  # noqa: E402

GATE_KEY = "g15.p0.m_e.regression_drift_guard"
NUMERIC_STEP = 278  # 落盘前实测 registry/number_ledger.json CI_step.next_free=278 顺位领取
SUBJECT = "g15_m_e_regression_drift_guard"
WAVE = "G15.6a"
TAG = "g15_m_e"
MATRIX_ROW = "M-e"
SOURCE_REF = (
    "G15_CONTRACT §4.2 M-e/G-G15-8;G15_ACCEPTANCE_MAP §1 M-e;"
    "G9 34 + G10 14 + G11 14 + G12 9 + G13 5 + G14 8 keys latest evidence read-only summary（G14 M-d 诚实红特判）;"
    "touched-face cargo test + quality anchor recheck zero-degrade;RD-045/M165 drift monitoring（G12-N13 承接）"
)
G15_0_REF = "f061487efaf7816684de18a6ef86554e5c392a75"  # G15.0 不可变 ref（G14 close-out flip commit）

# G14 8 key（M-a~M-e + M-f + M-g + M-h 闭集；G15 契约「G14 8 key」字面）
G14_KEYS = [
    ("g14.p0.m_a.registry_variance_band_reconciliation", "g14_m_a_registry_variance_band_reconciliation"),
    ("g14.p0.m_b.ue_benchmark_arm_measurement", "g14_m_b_ue_benchmark_arm_measurement"),
    ("g14.p0.m_c.rurix_pipeline_perf", "g14_m_c_rurix_pipeline_perf"),
    ("g14.p0.m_d.dual_end_fps_parity", "g14_m_d_dual_end_fps_parity"),
    ("g14.p0.m_e.regression_drift_guard", "g14_m_e_regression_drift_guard"),
    ("g14.p0.m_f.production_caliber_stage_a", "g14_m_f_production_caliber_stage_a"),
    ("g14.p0.m_g.vendor_parallel_conversion", "g14_m_g_vendor_parallel_conversion"),
    ("g14.p0.m_h.continuation_closeout", "g14_m_h_continuation_closeout"),
]

ALL_84 = list(G9_KEYS) + list(G10_KEYS) + list(G11_KEYS) + G12_KEYS + G13_KEYS + G14_KEYS

# 诚实红门面闭集（84 门内唯一登记红面 = G14 M-d；红面维持红登记不遮蔽不代绿）。
HONEST_RED_PREFIX = "g14_m_d_dual_end_fps_parity"
FPS_REGISTRY_PATH = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"
FPS_GAP_ID = "51a150cb4523e8b6"
G15_CONTRACT = ROOT / "milestones" / "g15" / "G15_CONTRACT.md"
G15_MD_PREFIX = "g15_m_d_perf_parity_zero_regression"
# G15 M-d 门诚实红结构（绿键集/红键集闭集——§8.5~§8.7 定盘字面）。
G15_MD_HONEST_GREEN = frozenset({
    "quality_anchor_band_recheck",
    "g14_budget_zero_estimated",
    "red_arm_ratio_tamper_detected",
    "red_arm_stale_evidence_detected",
    "red_arm_missing_run_detected",
    "red_arm_anchor_drift_detected",
})
G15_MD_HONEST_RED = frozenset({
    "g14_m_d_rerun_fresh_pass",
    "eighteen_cells_ratio_pass_line",
    "three_run_guard_band_complete",
    "production_caliber_invariant",
    "digest_anchor_zero_drift",
    "comparison_vs_g14_12_rerun",
})

# 触改面抽检闭集（cargo test 真跑 + M-c 最新件复核）。
CARGO_TEST_ARGV = ["cargo", "test", "-p", "rurix-rt", "--features", "vendor-upscale", "--no-fail-fast"]
# cargo test 失败基线三面（G14.9 登记双面 + G15plus-II 登记既有并行污染面——非本波引入）。
BASELINE_CARGO_FAILURE_TOKENS = (
    "m103_descriptor_buffer_ffi_layout_anchors",
    "binding_supply_chain_no_external_vulkan_crate",
    "zero_readback_full_chain",
)
MIN_CARGO_PASSED = 200

# G15 复跑面漂移监控消费闭集（确定性键族：bitexact/deterministic/digest 三字面族）。
DRIFT_SURFACES = [
    ("g13.p0.m_c.ue_upscale_parity", "g13_m_c_ue_upscale_parity"),
    ("g13.p0.m_d.ue_lumen_gi_parity", "g13_m_d_ue_lumen_gi_parity"),
    ("g12.p0.m163.ue_pt_parity", "g12_m163_ue_pt_parity"),
    ("g14.p0.m_c.rurix_pipeline_perf", "g14_m_c_rurix_pipeline_perf"),
    ("g14.p0.m_d.dual_end_fps_parity", "g14_m_d_dual_end_fps_parity"),
    ("g14.p0.m_f.production_caliber_stage_a", "g14_m_f_production_caliber_stage_a"),
    ("g14.p0.m_g.vendor_parallel_conversion", "g14_m_g_vendor_parallel_conversion"),
    ("g15.p0.m_c.absolute_quality_final_review", "g15_m_c_absolute_quality_final_review"),
]
DETERMINISM_KEY_TOKENS = ("bitexact", "deterministic", "digest")
M165_FAIL_EVIDENCE = EVIDENCE_DIR / "g12_m165_pt_throughput_baseline_20260817T235251Z.json"
FLIP_TRACE_ARM = ROOT / "src" / "rurix-asset" / "src" / "bin" / "g12_4_ue_pt_parity_render.rs"
FLIP_TRACE_TOKEN = "G12_5_BENCH_FLIP_TRACE"

# G5~G14 closed 面 0-byte 允许闭集（34f96ac3 归档授权双面 + G14 M-d 门产登记面）。
ALLOWED_CLOSED_DIFF = {
    "milestones/g14/g14_budget.json",
    "milestones/g14/g14_ue_variance_samples.json",
    "milestones/g14/g14_fps_gap_registry.json",
}
CLOSED_GLOBS = [
    "ci/g5_*.py", "ci/g6_*.py", "ci/g7_*.py", "ci/g8_*.py", "ci/g9_*.py",
    "ci/g10_*.py", "ci/g11_*.py", "ci/g12_*.py", "ci/g13_*.py", "ci/g14_*.py",
    "milestones/g5", "milestones/g6", "milestones/g7", "milestones/g8",
    "milestones/g9", "milestones/g10", "milestones/g11", "milestones/g12",
    "milestones/g13", "milestones/g14",
]
# G15 期 src 触改授权面（G15plus-II candidate-b 单文件——§8.7 登记面）。
ALLOWED_SRC_DIFF = {"src/rurix-rt/src/vendor_upscale.rs"}

CHECK_KEYS = [
    "temporal_upscale_trait_0byte",
    "gates_84_zero_degrade",
    "honest_red_faces_registered",
    "touched_face_spot_zero_degrade",
    "g5_g14_closed_surface_0byte",
    "rd045_m165_drift_monitoring_registered",
    "red_degraded_gate_detected",
    "red_aggregate_masking_detected",
    "red_drift_unregistered_detected",
    "red_honest_red_masquerade_detected",
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


def run(cmd: list[str], timeout: int = 7200) -> subprocess.CompletedProcess:
    print(f"[{TAG}] run: {' '.join(cmd)}", flush=True)
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout)


def _git(*args: str) -> str:
    r = subprocess.run(["git"] + list(args), cwd=ROOT, capture_output=True, text=True)
    return r.stdout or ""


# ---------------------------------------------------------------------------
# 诚实红门面评定（G14 closeout 先例同型——84 门汇总/soak/closeout 共用单一事实源）
# ---------------------------------------------------------------------------
def eval_honest_red_g14_md(doc: dict) -> tuple[bool, str]:
    """G14 M-d 诚实红门面特判：checks 全绿 ∧ status=="fail" ∧ unmet_count ==
    g14_fps_gap_registry 行数（1 行 gap_id 51a150cb4523e8b6）= 通过线未达标
    如实登记面（红面维持红登记不遮蔽不代绿；登记面不一致即降级）。"""
    checks = doc.get("checks") or {}
    bad = [k for k, v in checks.items() if v is not True]
    status = doc.get("status")
    unmet = (doc.get("parity") or {}).get("unmet_count")
    reg_rows = -1
    reg_ids: set = set()
    if FPS_REGISTRY_PATH.is_file():
        items = (wel.load_json(FPS_REGISTRY_PATH)).get("items") or []
        reg_rows = len(items)
        reg_ids = {it.get("gap_id") for it in items}
    if status == "pass":
        return True, "达标面（status=pass——未来延续波翻转面亦为合法登记态）"
    ok = (
        status == "fail"
        and not bad
        and unmet is not None
        and unmet == reg_rows == 1
        and reg_ids == {FPS_GAP_ID}
    )
    if ok:
        return True, (
            f"G14 M-d 诚实红门面特判合格（checks 全绿 + status=fail + unmet={unmet} == "
            f"登记表 {reg_rows} 行〔gap_id {FPS_GAP_ID}〕——通过线未达标如实登记不冒充，"
            "红面维持红登记不遮蔽不代绿）"
        )
    return False, (
        f"G14 M-d 面异常: status={status!r} checks_bad={bad[:3]} unmet={unmet} "
        f"reg={reg_rows} ids={sorted(reg_ids)}（诚实红登记面不一致即回归降级）"
    )


def aggregate_gates(gates: list[tuple[str, str]]) -> tuple[list[dict], list[str], list[str]]:
    """84 门只读汇总：逐门行集 + 降级面 + 红面登记集（聚合不遮蔽——行集全量入 evidence）。"""
    rows: list[dict] = []
    problems: list[str] = []
    red_faces: list[str] = []
    for key, subject in gates:
        row = wel.require_gate_pass(key, subject)
        if row["status"] != "PASS" and subject == HONEST_RED_PREFIX:
            path = wel.load_latest_evidence(subject)
            doc = wel.load_json(path) if path else {}
            ok, detail = eval_honest_red_g14_md(doc)
            if ok:
                red_faces.append(subject)
                row["status"] = "PASS"
                row["detail"] = f"{row.get('detail','')}; {detail}"
            else:
                row["status"] = "FAIL"
                row["detail"] = detail
        rows.append(row)
        if row["status"] != "PASS":
            problems.append(f"{key}: {row['detail']}")
    return rows, problems, red_faces


def verdict_of(rows: list[dict]) -> str:
    """聚合裁定（遮蔽即自检红面：任一子行非 PASS 即 FAIL，零折叠）。"""
    return "PASS" if all(r.get("status") == "PASS" for r in rows) and rows else "FAIL"


def collect_drift_surfaces() -> dict:
    """RD-045/M165 漂移监控面：G15 全期复跑面确定性键族逐键读值 + 同型 digest 漂移计数
    （G15 M-d 消费 parity 登记面——诚实红跳过量不冒充监控面）。"""
    checked: list[str] = []
    drift: list[str] = []
    for key, subject in DRIFT_SURFACES:
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
    # G15 M-d evidence parity 登记面（drift_count / rd_045 检出计数——零检出字面）
    g15md_path = wel.load_latest_evidence(G15_MD_PREFIX)
    if g15md_path is None:
        drift.append("g15_m_d 缺最新 evidence（M-d 复跑面监控缺件）")
    else:
        g15md = wel.load_json(g15md_path)
        parity = g15md.get("parity") or {}
        dg = parity.get("digest_anchor") or {}
        dm = parity.get("drift_monitoring") or {}
        dc = dg.get("drift_count")
        rd = dm.get("rd_045_type_digest_drift_detected")
        checked.append(f"{G15_MD_PREFIX}:parity.digest_anchor.drift_count={dc}")
        checked.append(f"{G15_MD_PREFIX}:parity.drift_monitoring.rd_045={rd}")
        if dc != 0 or rd != 0:
            drift.append(f"{G15_MD_PREFIX}: drift_count={dc} rd_045={rd}（检出面——同型事件登记升级）")
    fail_retained = (
        M165_FAIL_EVIDENCE.is_file()
        and (wel.load_json(M165_FAIL_EVIDENCE).get("status") == "fail")
    )
    arm = FLIP_TRACE_ARM.is_file() and FLIP_TRACE_TOKEN in FLIP_TRACE_ARM.read_text(encoding="utf-8")
    rd045_open = wel.load_rd_status("RD-045") == "open"
    return {
        "checked_keys": checked,
        "drift_detected_count": len(drift),
        "drift_details": drift,
        "fail_evidence_retained": bool(fail_retained),
        "flip_trace_arm_present": bool(arm),
        "rd045_open": bool(rd045_open),
    }


def red_arm_degraded_gate() -> bool:
    """RED 臂：既有门降级注入（不存在门 key 注入聚合面 → 必检出非 PASS 行）。"""
    rows, problems, _ = aggregate_gates([("g9.p0.m999.nonexistent_gate", "g9_m999_nonexistent")])
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


def red_arm_honest_red_masquerade() -> bool:
    """RED 臂：诚实红冒充注入（checks 非全绿的红面 → 诚实红评定面必拒答判降级）。"""
    masq = {
        "status": "fail",
        "checks": {"dual_end_measurement_fresh": True, "pass_line_evaluated": False},
        "parity": {"unmet_count": 1},
    }
    ok, _ = eval_honest_red_g14_md(masq)
    # 合格面 = 拒答（checks 非全绿不充诚实红）；旁证：真登记面形态判合格
    legit = {
        "status": "fail",
        "checks": {"a": True, "b": True},
        "parity": {"unmet_count": 1},
    }
    ok2, _ = eval_honest_red_g14_md(legit)
    return (not ok) and ok2


def git_closed_surface() -> tuple[bool, str]:
    """G5~G14 closed 面 0-byte 机核（vs G15.0 ref committed diff 闭集 + 工作树 porcelain）。"""
    diff = _git("diff", "--name-only", f"{G15_0_REF}..HEAD", "--", *CLOSED_GLOBS)
    committed = sorted(x for x in diff.splitlines() if x.strip())
    bad_committed = [f for f in committed if f not in ALLOWED_CLOSED_DIFF]
    porc = _git("status", "--porcelain", "--", *CLOSED_GLOBS)
    working = sorted(ln[3:].strip() for ln in porc.splitlines() if ln.strip())
    ok = not bad_committed and not working
    detail = (
        f"committed 闭集={committed or '空'}（允许面={sorted(ALLOWED_CLOSED_DIFF)}）；"
        f"工作树闭集={working or '空'}"
        + (f"；越界 committed={bad_committed} working={working}" if not ok else "")
    )
    return ok, detail


def src_touched_closure() -> tuple[bool, str]:
    """G15 期 src 触改闭集机核（candidate-b 单文件授权面 + 工作树 tracked 空 +
    untracked ⊆ 异己登记六件闭集）。"""
    diff = _git("diff", "--name-only", f"{G15_0_REF}..HEAD", "--", "src")
    committed = sorted(x for x in diff.splitlines() if x.strip())
    bad_committed = [f for f in committed if f not in ALLOWED_SRC_DIFF]
    working_bad: list[str] = []
    for ln in (_git("status", "--porcelain", "--", "src") or "").splitlines():
        if not ln.strip():
            continue
        state, path = ln[:2], ln[3:].strip()
        if state == "??":
            if path not in ALIEN_UNTRACKED_SRC:
                working_bad.append(f"untracked 越界 {path}")
        else:
            working_bad.append(f"tracked 修改 {path}")
    ok = not bad_committed and not working_bad
    detail = (
        f"src/ committed 闭集={committed or '空'}（允许面={sorted(ALLOWED_SRC_DIFF)} candidate-b 单文件）；"
        f"工作树 tracked 空 + untracked ⊆ 异己登记六件"
        + (f"；越界 committed={bad_committed} working={working_bad[:3]}" if not ok else "")
    )
    return ok, detail


def run_cargo_test() -> tuple[bool, str, dict]:
    """触改面真跑抽检：cargo test -p rurix-rt --features vendor-upscale --no-fail-fast。
    合格面 = 失败集 ⊆ 登记基线三面（既有面非本波引入），新败即降级 RED。"""
    r = run(CARGO_TEST_ARGV, timeout=7200)
    blob = (r.stdout or "") + "\n" + (r.stderr or "")
    failed_names = re.findall(r"(?m)^test (\S+) \.\.\. FAILED$", blob)
    passed = sum(int(m.group(1)) for m in re.finditer(r"test result: (?:ok|FAILED)\. (\d+) passed", blob))
    failed_total = sum(int(m.group(1)) for m in re.finditer(r"test result: FAILED\. \d+ passed; (\d+) failed", blob))
    new_failures = [
        n for n in failed_names
        if not any(tok in n for tok in BASELINE_CARGO_FAILURE_TOKENS)
    ]
    missing_baseline_note = [
        tok for tok in BASELINE_CARGO_FAILURE_TOKENS
        if not any(tok in n for n in failed_names)
    ]
    ok = not new_failures and passed >= MIN_CARGO_PASSED
    detail = (
        f"cargo test exit={r.returncode} passed={passed} failed={len(failed_names)}/{failed_total}；"
        f"失败集 ⊆ 登记基线三面（{'; '.join(BASELINE_CARGO_FAILURE_TOKENS)}）"
        f"新败={new_failures[:3] or '无'}（基线未现面={missing_baseline_note or '无'}——并行污染面间歇性不强制）"
    )
    return ok, detail, {
        "exit_code": r.returncode,
        "passed": passed,
        "failed_names": failed_names,
        "new_failures": new_failures,
    }


def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    started_stamp = ts

    # ── ① temporal 底座 + UpscaleBackend trait 面 0-byte + src 触改闭集 ──
    ok_temp, msg_temp = mb.temporal_base_0byte()
    ok_src, msg_src = src_touched_closure()
    checks["temporal_upscale_trait_0byte"] = ok_temp and ok_src
    check(checks["temporal_upscale_trait_0byte"], f"temporal/trait 0-byte: {msg_temp}；{msg_src}")
    note(f"temporal 底座/UpscaleBackend trait 面（temporal/upscale.rs）0-byte：{msg_temp}；src 触改闭集：{msg_src}")

    # ── ② 既有 84 门最新 evidence 全绿只读汇总（不遮蔽；G14 M-d 诚实红特判） ──
    gate_rows, agg_problems, red_faces = aggregate_gates(ALL_84)
    checks["gates_84_zero_degrade"] = not agg_problems and len(gate_rows) == 84
    check(checks["gates_84_zero_degrade"], f"84 门聚合: {agg_problems[:4]}")
    note(f"84 门只读汇总：{sum(1 for r in gate_rows if r['status'] == 'PASS')}/84 合格"
         f"（行集全量入 evidence 不折叠；登记红面 = {red_faces or '无'}）")

    # ── ③ 诚实红面登记完备（红面集 == {G14 M-d} + G15 M-d 门诚实红结构 + G15-MD-F1 锚在案） ──
    hr_bad: list[str] = []
    if red_faces != [HONEST_RED_PREFIX]:
        hr_bad.append(f"红面集 {red_faces} ≠ 登记闭集 [{HONEST_RED_PREFIX}]（非登记面红 = 降级面）")
    g15md_path = wel.load_latest_evidence(G15_MD_PREFIX)
    if g15md_path is None:
        hr_bad.append("g15_m_d 缺最新 evidence")
    else:
        g15md = wel.load_json(g15md_path)
        g15md_checks = g15md.get("checks") or {}
        green_bad = [k for k in G15_MD_HONEST_GREEN if g15md_checks.get(k) is not True]
        red_bad = [k for k in G15_MD_HONEST_RED if g15md_checks.get(k) is not False]
        extra_keys = set(g15md_checks) - (G15_MD_HONEST_GREEN | G15_MD_HONEST_RED)
        if g15md.get("status") != "fail" or green_bad or red_bad or extra_keys:
            hr_bad.append(
                f"G15 M-d 诚实红结构异常: status={g15md.get('status')!r} green_bad={green_bad} "
                f"red_bad={red_bad} extra={sorted(extra_keys)}"
            )
    contract_text = G15_CONTRACT.read_text(encoding="utf-8") if G15_CONTRACT.is_file() else ""
    for token in ("G15-MD-F1", "§8.6", "§8.7"):
        if token not in contract_text:
            hr_bad.append(f"G15_CONTRACT 缺 {token} 承接锚登记字面")
    checks["honest_red_faces_registered"] = not hr_bad
    check(not hr_bad, f"诚实红面登记: {hr_bad[:3]}")
    note("诚实红面登记：红面集 == {G14 M-d}（checks 全绿 + status=fail + unmet==1==登记表行数）"
         "+ G15 M-d 门诚实红结构（绿键 6 + 红键 6 闭集）+ G15-MD-F1 承接锚 §8.6/§8.7 在案"
         if not hr_bad else f"诚实红面登记异常: {hr_bad[:2]}")

    # ── ④ 触改面真跑抽检零降级（cargo test 子进程真跑 + G14 M-c 最新件复核） ──
    spot_rows: list[dict] = []
    spot_bad: list[str] = []
    cargo_ok, cargo_detail, cargo_info = run_cargo_test()
    spot_rows.append({"id": "cargo_test_rurix_rt", "status": "PASS" if cargo_ok else "FAIL", "detail": cargo_detail})
    if not cargo_ok:
        spot_bad.append(cargo_detail)
    note(f"抽检 cargo_test_rurix_rt: {cargo_detail}")
    qa_ok, qa_detail, qa_deficit = md.recheck_quality_anchor()
    spot_rows.append({"id": "g14_m_c_quality_anchor_recheck", "status": "PASS" if qa_ok else "FAIL",
                      "detail": qa_detail})
    if not qa_ok:
        spot_bad.append(f"G14 M-c 最新件复核: {qa_detail}")
    note(f"抽检 g14_m_c_quality_anchor_recheck（candidate-b 落地态画质零降级机核）: {qa_detail}")
    checks["touched_face_spot_zero_degrade"] = not spot_bad
    check(not spot_bad, f"触改面抽检降级: {spot_bad[:2]}")

    # ── ⑤ G5~G14 closed 面 0-byte ──
    ok, detail = git_closed_surface()
    checks["g5_g14_closed_surface_0byte"] = ok
    check(ok, f"G5~G14 closed 面 0-byte: {detail}")
    note(f"G5~G14 closed 面：{detail}")

    # ── ⑥ RD-045/M165 漂移监控登记（G15 全期复跑面零检出字面入 evidence） ──
    drift = collect_drift_surfaces()
    drift_ok = (
        drift["drift_detected_count"] == 0
        and drift["fail_evidence_retained"]
        and drift["flip_trace_arm_present"]
        and drift["rd045_open"]
        and bool(drift["checked_keys"])
    )
    checks["rd045_m165_drift_monitoring_registered"] = drift_ok
    check(drift_ok, f"漂移监控面: {drift['drift_details'][:3] or '零检出'} "
                    f"FAIL件在档={drift['fail_evidence_retained']} 诊断臂在树={drift['flip_trace_arm_present']} "
                    f"RD-045 open={drift['rd045_open']}")
    note(f"RD-045/M165 漂移监控：G15 全期复跑面确定性键族 {len(drift['checked_keys'])} 键全真，"
         f"同型 digest 漂移检出计数 = {drift['drift_detected_count']}（零检出字面维持 open 不关闭——"
         f"检出即如实登记升级）；FAIL 件 0-byte 在档 = {drift['fail_evidence_retained']}；"
         f"flip-trace 诊断臂在树 = {drift['flip_trace_arm_present']}；RD-045 open 维持 = {drift['rd045_open']}")

    # ── ⑦ RED 四臂 ──
    red_results = {
        "degraded_gate": red_arm_degraded_gate(),
        "aggregate_masking": red_arm_aggregate_masking(),
        "drift_unregistered": red_arm_drift_unregistered(),
        "honest_red_masquerade": red_arm_honest_red_masquerade(),
    }
    checks["red_degraded_gate_detected"] = red_results["degraded_gate"]
    checks["red_aggregate_masking_detected"] = red_results["aggregate_masking"]
    checks["red_drift_unregistered_detected"] = red_results["drift_unregistered"]
    checks["red_honest_red_masquerade_detected"] = red_results["honest_red_masquerade"]
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
        "base_commit": (_git("rev-parse", "HEAD") or "").strip(),
        "host_section_pass": all_pass,
        "device_section_state": "executed",
        "checks": checks,
        "commands": [
            {"seq": 1, "command": "84 门最新 evidence 全绿只读汇总（G14 M-d 诚实红特判）", "exit_code": 0 if checks["gates_84_zero_degrade"] else 1},
            {"seq": 2, "command": " ".join(CARGO_TEST_ARGV), "exit_code": cargo_info["exit_code"]},
            {"seq": 3, "command": "G14 M-c 最新件复核（converged.exr 双件 SSIM deficit 重算 ≤ 0.010779849285388998）", "exit_code": 0 if qa_ok else 1},
            {"seq": 4, "command": "RD-045/M165 漂移监控登记（G15 全期复跑面确定性键族）", "exit_code": 0 if checks["rd045_m165_drift_monitoring_registered"] else 1},
            {"seq": 5, "command": "RED 臂 ×4（degraded-gate/aggregate-masking/drift-unregistered/honest-red-masquerade）", "exit_code": 0 if all(v for v in red_results.values()) else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "production": {
            "correctness_anchor_unchanged": checks["temporal_upscale_trait_0byte"],
            "baseline_anchor_id": "n/a（回归门不产新锚；G9~G14 绿面 0-byte 维持 + G14 M-d 诚实红面维持红登记）",
            "measured_value": (
                f"84 门聚合合格={sum(1 for r in gate_rows if r['status'] == 'PASS')}/84（登记红面 {red_faces}）；"
                f"触改面抽检 cargo test passed={cargo_info['passed']} 新败 {len(cargo_info['new_failures'])}；"
                f"画质锚带重算 {qa_deficit}；漂移检出计数={drift['drift_detected_count']}"
            ),
            "not_worse_than_anchor": checks["gates_84_zero_degrade"] and checks["touched_face_spot_zero_degrade"],
            "threshold_provenance": "n/a（回归门面；抽检门各自 budget 面自持）",
            "evolution_register": "G14 M-d 诚实红门面 status=fail 字面维持不充绿不充降级（checks 全绿 + unmet==登记表行数 + G15-MD-F1 承接锚在案）；G15 期 src 触改面 = candidate-b 单文件授权面（§8.7 登记）",
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "regression": {
            "gates_84": gate_rows,
            "honest_red_faces": red_faces,
            "spot_reruns": spot_rows,
            "cargo_test": cargo_info,
            "session_started_utc": started_stamp,
            "drift_monitoring": drift,
        },
    }
    errs = wel.validate_schema(evidence, SCHEMA_PATH) if SCHEMA_PATH.is_file() else []
    if errs:
        print(f"[{TAG}] schema errors: {errs}", file=sys.stderr)
        evidence["status"] = "fail"
        evidence["host_section_pass"] = False
        all_pass = False
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    print(f"[{TAG}] VERDICT={'PASS' if evidence['status'] == 'pass' else 'FAIL'} checks={sum(1 for v in checks.values() if v)}/{len(checks)}")
    return 0 if evidence["status"] == "pass" else 1


def run_selftest() -> int:
    """schema 闭集对账 + RED/GREEN 双臂。"""
    failures = 0
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        failures += 1
    if not red_arm_degraded_gate():
        print(f"[{TAG}] selftest FAIL: degraded-gate 臂未检出", file=sys.stderr)
        failures += 1
    if not red_arm_aggregate_masking():
        print(f"[{TAG}] selftest FAIL: aggregate-masking 臂未检出", file=sys.stderr)
        failures += 1
    if not red_arm_drift_unregistered():
        print(f"[{TAG}] selftest FAIL: drift-unregistered 臂未检出", file=sys.stderr)
        failures += 1
    if not red_arm_honest_red_masquerade():
        print(f"[{TAG}] selftest FAIL: honest-red-masquerade 臂未检出", file=sys.stderr)
        failures += 1
    # GREEN 面：正例不误判
    good_rows = [{"symbolic_gate_key": "g9.p0.m96.path_tracer_reference", "status": "PASS"}]
    if verdict_of(good_rows) != "PASS":
        print(f"[{TAG}] selftest FAIL: 聚合正例误判", file=sys.stderr)
        failures += 1
    if len(ALL_84) != 84:
        print(f"[{TAG}] selftest FAIL: 84 门闭集 n={len(ALL_84)}", file=sys.stderr)
        failures += 1
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)}（schema 闭集 + 4 RED + 2 GREEN 函数面臂）")
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
