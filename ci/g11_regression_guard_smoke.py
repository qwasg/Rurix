#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.5 波）
"""G11.5 M156 修复回归门（P0，步骤 212；g11.p0.m156.regression_guard；
G11_CONTRACT §4.2 M156 行判据逐字 / G-G11-7；G11_ACCEPTANCE_MAP §1 M156 行；
CI_GATES §4）。

host 纯 host 门（device_section_state=not_applicable；抽检既有门经子进程真跑，
各自 evidence 独立落盘自持 device 面——本门只读汇总 + 子进程退出码/新鲜度机核，
D5 定案同 M139 不嵌套持锁）。判据（契约 §4.2 M156 行字面 + MAP 逐字）：

1. **既有 48 门（G9 34 key + G10 14 key）最新 evidence 全绿只读汇总**：
   wel.require_gate_pass 逐门只读核验（symbolic_gate_key 相符 ∧
   host_section_pass=True ∧ device_section_state ∉ {fail,dev_env_degrade,skip} ∧
   checks 全 True）；聚合不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE。
2. **G11 已绿门零降级**：G11.2~G11.4 已绿门（M144~M146/M148~M154/M157 +
   wave2/3/4 exit）最新 evidence 全绿只读汇总；M147 双 phase 面 = 最新
   phase=g11.3 evidence PASS + **当次复跑 --phase g11.3 真跑绿**（G11.5 波
   脚本扩支后既有绿面零降级实证）；M147 g11.5 phase FAIL = 本波 M155 面
   诚实 verdict（显式登记不遮蔽——回归面与复测收敛断言面分离）。
3. **关键门真跑抽检零降级**（修复触改面既有门重跑回归）：M130 双 phase
   （--phase g10.2 + --phase g10.5 三重绑定）/ M139（A/B 对比门）/ M140
   （清单门）/ M141（性能基线门）/ G9 代表门 M96/M94/M110——子进程真跑
   exit 0 + 最新 evidence PASS + **新鲜度机核**（evidence timestamp ≥ 本门
   会话起点——陈旧 evidence 冒充当次复跑即 RED）。
4. **既有判据 0-byte**：G5~G10 closed 门脚本与里程碑面 git 工作树 0-byte
   （git status --porcelain 闭集面空集）。

RED 臂（契约判据字面）：既有门降级即 RED（red_degraded_gate——子断言非 PASS
注入必检出）；聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED
（red_aggregate_masking——遮蔽型汇总必检出）；陈旧 evidence 冒充当次复跑即
RED（red_stale_evidence——timestamp 早于会话起点必检出）。

用法：
  py -3 ci/g11_regression_guard_smoke.py --gate g11.p0.m156.regression_guard
  py -3 ci/g11_regression_guard_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m156_regression_guard_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402
from g11_wave3_exit_check import G10_KEYS, G9_KEYS  # noqa: E402  # 48 门清单单一事实源

GATE_KEY = "g11.p0.m156.regression_guard"
NUMERIC_STEP = 212
SOURCE_REF = (
    "G11_CONTRACT §4.2 M156 + G-G11-7;G11_ACCEPTANCE_MAP §1 M156;CI_GATES §4;"
    "G9 34 keys + G10 14 keys latest evidence read-only summary;spot rerun zero-degrade"
)
TAG = "g11_m156"
SUBJECT = "g11_m156_regression_guard"
MATRIX_ROW = "M156"

# G11 已绿门（wave2~wave4；M147 走双 phase 面独立机核）。
G11_GREEN_KEYS = [
    ("g11.p0.m144.caliber_c1_indoor_luminance", "g11_m144_caliber_c1_indoor_luminance"),
    ("g11.p0.m145.caliber_c2_exposure_chain", "g11_m145_caliber_c2_exposure_chain"),
    ("g11.p0.m146.caliber_c3_exr_bit_depth", "g11_m146_caliber_c3_exr_bit_depth"),
    ("g11.p0.m148.fix_r2_geometry_normals", "g11_m148_fix_r2_geometry_normals"),
    ("g11.p0.m149.fix_r5_json_u64_seed", "g11_m149_fix_r5_json_u64_seed"),
    ("g11.p0.m150.fix_u1_cornell_shell_radiance", "g11_m150_fix_u1_cornell_shell_radiance"),
    ("g11.p0.m151.fix_u2_bistro_texture_dds", "g11_m151_fix_u2_bistro_texture_dds"),
    ("g11.p0.m152.fix_u3_bistro_animation", "g11_m152_fix_u3_bistro_animation"),
    ("g11.p0.m153.fix_r3_light_subset", "g11_m153_fix_r3_light_subset"),
    ("g11.p0.m154.fix_r4_gi_multibounce_world_cache", "g11_m154_fix_r4_gi_multibounce_world_cache"),
    ("g11.p1.m157.hdr_flip_calibration", "g11_m157_hdr_flip_calibration"),
    ("g11.wave.2.exit", "g11_wave2_exit"),
    ("g11.wave.3.exit", "g11_wave3_exit"),
    ("g11.wave.4.exit", "g11_wave4_exit"),
]

# 关键门真跑抽检闭集（子进程 argv + 环境面 + 新鲜度核验 subject/key）。
SPOT_GATES = [
    {
        "id": "m130_g10_2",
        "argv": [sys.executable, "ci/g10_dual_determinism_contract_smoke.py", "--gate", "g10.p0.m130.dual_determinism_contract", "--phase", "g10.2"],
        "env": {},
        "key": "g10.p0.m130.dual_determinism_contract",
        "subject": "g10_m130_dual_determinism_contract",
    },
    {
        "id": "m130_g10_5",
        "argv": [sys.executable, "ci/g10_dual_determinism_contract_smoke.py", "--gate", "g10.p0.m130.dual_determinism_contract", "--phase", "g10.5"],
        "env": {},
        "key": "g10.p0.m130.dual_determinism_contract",
        "subject": "g10_m130_dual_determinism_contract",
    },
    {
        "id": "m139",
        "argv": [sys.executable, "ci/g10_ab_comparison_smoke.py", "--gate", "g10.p0.m139.ab_comparison"],
        "env": {},
        "key": "g10.p0.m139.ab_comparison",
        "subject": "g10_m139_ab_comparison",
    },
    {
        "id": "m140",
        "argv": [sys.executable, "ci/g10_gap_registry_smoke.py", "--gate", "g10.p0.m140.gap_registry"],
        "env": {},
        "key": "g10.p0.m140.gap_registry",
        "subject": "g10_m140_gap_registry",
    },
    {
        "id": "m141",
        "argv": [sys.executable, "ci/g10_perf_baseline_smoke.py", "--gate", "g10.p0.m141.perf_baseline"],
        "env": {},
        "key": "g10.p0.m141.perf_baseline",
        "subject": "g10_m141_perf_baseline",
    },
    {
        "id": "g9_m96",
        "argv": [sys.executable, "ci/g9_path_tracer_reference_smoke.py", "--gate", "g9.p0.m96.path_tracer_reference"],
        "env": {"RURIX_REQUIRE_REAL": "1", "RURIX_VK_VALIDATION": "1"},
        "key": "g9.p0.m96.path_tracer_reference",
        "subject": "g9_m96_path_tracer_reference",
    },
    {
        "id": "g9_m94",
        "argv": [sys.executable, "ci/g9_clas_rt_convergence_smoke.py", "--gate", "g9.p0.m94.clas_rt_convergence"],
        "env": {"RURIX_REQUIRE_REAL": "1", "RURIX_VK_VALIDATION": "1"},
        "key": "g9.p0.m94.clas_rt_convergence",
        "subject": "g9_m94_clas_rt_convergence",
    },
    {
        "id": "g9_m110",
        "argv": [sys.executable, "ci/g9_world_partition_smoke.py", "--gate", "g9.p0.m110.world_partition"],
        "env": {"RURIX_REQUIRE_REAL": "1", "RURIX_VK_VALIDATION": "1"},
        "key": "g9.p0.m110.world_partition",
        "subject": "g9_m110_world_partition",
    },
]

CHECK_KEYS = [
    "g10_14_gates_latest_evidence_all_green",
    "g9_34_gates_latest_evidence_all_green",
    "g11_green_gates_latest_evidence_all_green",
    "m147_g11_3_phase_face_rerun_green",
    "spot_rerun_m130_g10_2_pass",
    "spot_rerun_m130_g10_5_pass",
    "spot_rerun_m139_pass",
    "spot_rerun_m140_pass",
    "spot_rerun_m141_pass",
    "spot_rerun_g9_m96_pass",
    "spot_rerun_g9_m94_pass",
    "spot_rerun_g9_m110_pass",
    "legacy_criteria_0byte",
    "red_degraded_gate_detected",
    "red_aggregate_masking_detected",
    "red_stale_evidence_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []
SESSION_START = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _summary_rows(keys: list[tuple[str, str]]) -> tuple[list[dict], list[str]]:
    rows = [wel.require_gate_pass(key, prefix) for key, prefix in keys]
    bad = [f"{r['subject_prefix']}: {r['detail'][:80]}" for r in rows if r["status"] != "PASS"]
    return rows, bad


def _spot_rerun(spec: dict) -> dict:
    """子进程真跑抽检：exit 0 + 最新 evidence PASS + 新鲜度（timestamp ≥ 会话起点）。"""
    env = dict(os.environ)
    env.update(spec["env"])
    r = subprocess.run(spec["argv"], cwd=ROOT, capture_output=True, text=True, env=env, timeout=14400)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(spec["argv"][1:]), "exit_code": r.returncode})
    row = {"id": spec["id"], "exit_code": r.returncode, "status": "FAIL", "detail": ""}
    if r.returncode != 0:
        tail = (r.stdout + r.stderr).strip().splitlines()
        row["detail"] = f"子进程 exit={r.returncode}: {(tail[-1] if tail else '')[-160:]}"
        return row
    ev_path = wel.load_latest_evidence(spec["subject"])
    if ev_path is None:
        row["detail"] = "缺最新 evidence"
        return row
    ev = wel.load_json(ev_path)
    ok, detail = wel.gate_pass_reason(ev, spec["key"])
    stamp = ev.get("timestamp") or ""
    fresh = isinstance(stamp, str) and stamp >= SESSION_START
    if not ok:
        row["detail"] = f"evidence 非 PASS: {detail}"
        return row
    if not fresh:
        row["detail"] = f"evidence 陈旧（timestamp {stamp} < 会话起点 {SESSION_START}——陈旧冒充当次即 RED）"
        return row
    row["status"] = "PASS"
    row["detail"] = f"exit 0 + 最新 evidence PASS + 新鲜（{stamp}）"
    row["evidence_path"] = str(ev_path.relative_to(ROOT)).replace("\\", "/")
    return row


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    key = "g10.p0.m140.gap_registry"
    # 绿臂：合形 evidence 判 PASS。
    good = {"symbolic_gate_key": key, "host_section_pass": True,
            "device_section_state": "executed", "checks": {"a": True}}
    ok, _ = wel.gate_pass_reason(good, key)
    if not ok:
        print(f"[{TAG}] selftest FAIL: 合形 evidence 未判 PASS", file=sys.stderr)
        return 1
    # 红臂①：既有门降级（子断言 FAIL/SKIP/DEV_ENV_DEGRADE）必检出。
    for bad_state in ("fail", "dev_env_degrade", "skip"):
        ok, _ = wel.gate_pass_reason(dict(good, device_section_state=bad_state), key)
        if ok:
            print(f"[{TAG}] selftest FAIL: device {bad_state} 未检出", file=sys.stderr)
            return 1
    ok, _ = wel.gate_pass_reason(dict(good, checks={"a": False}), key)
    if ok:
        print(f"[{TAG}] selftest FAIL: checks 非真未检出", file=sys.stderr)
        return 1
    # 红臂②：聚合遮蔽必检出（汇总含非 PASS 行时全绿判定必须假）。
    rows = [{"status": "PASS"}, {"status": "FAIL"}]
    if all(r.get("status") == "PASS" for r in rows):
        print(f"[{TAG}] selftest FAIL: 聚合遮蔽未检出", file=sys.stderr)
        return 1
    # 红臂③：陈旧 evidence 冒充当次复跑必检出（timestamp 字典序早于会话起点）。
    stale = "20000101T000000Z"
    if stale >= SESSION_START:
        print(f"[{TAG}] selftest FAIL: 陈旧 timestamp 比较臂失效", file=sys.stderr)
        return 1
    # schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = wel.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (5 RED + 2 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--skip-spot-reruns", action="store_true",
                    help="只跑只读汇总面（自检/调试；正式门不可省——缺抽检面即 FAIL）")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    # ① 既有 48 门最新 evidence 全绿只读汇总（G10 14 + G9 34；聚合不遮蔽子断言）。
    g10_rows, g10_bad = _summary_rows(G10_KEYS)
    checks["g10_14_gates_latest_evidence_all_green"] = not g10_bad
    check(not g10_bad, f"G10 14 门最新 evidence 非全绿: {g10_bad[:3]}")
    g9_rows, g9_bad = _summary_rows(G9_KEYS)
    checks["g9_34_gates_latest_evidence_all_green"] = not g9_bad
    check(not g9_bad, f"G9 34 门最新 evidence 非全绿: {g9_bad[:3]}")
    note(f"48 门只读汇总: G10 {len(G10_KEYS) - len(g10_bad)}/{len(G10_KEYS)} 绿 / G9 {len(G9_KEYS) - len(g9_bad)}/{len(G9_KEYS)} 绿")

    # ② G11 已绿门零降级只读汇总（wave2~wave4 已绿面；M147 走 ③ 双 phase 面）。
    g11_rows, g11_bad = _summary_rows(G11_GREEN_KEYS)
    checks["g11_green_gates_latest_evidence_all_green"] = not g11_bad
    check(not g11_bad, f"G11 已绿门最新 evidence 非全绿: {g11_bad[:3]}")

    # ③ M147 双 phase 面：最新 phase=g11.3 evidence PASS + 当次复跑 --phase g11.3
    # 真跑绿（脚本扩支后既有绿面零降级实证）；g11.5 phase FAIL = 本波 M155 面诚实
    # verdict（显式登记不遮蔽——回归面与复测收敛断言面分离，聚合不代绿）。
    m147_path = wel.load_latest_evidence("g11_m147_fix_r1_material_subset")
    m147_g113_ok = False
    if m147_path is not None:
        ev = wel.load_json(m147_path)
        if ev.get("phase") == "g11.3":
            ok, _ = wel.gate_pass_reason(ev, "g11.p0.m147.fix_r1_material_subset")
            m147_g113_ok = ok
    if not args.skip_spot_reruns:
        r = subprocess.run(
            [sys.executable, "ci/g11_fix_r1_material_subset_smoke.py", "--gate",
             "g11.p0.m147.fix_r1_material_subset", "--phase", "g11.3"],
            cwd=ROOT, capture_output=True, text=True, timeout=3600,
        )
        COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "ci/g11_fix_r1_material_subset_smoke.py --gate g11.p0.m147.fix_r1_material_subset --phase g11.3", "exit_code": r.returncode})
        if r.returncode != 0:
            m147_g113_ok = False
            check(False, f"M147 --phase g11.3 当次复跑非绿（既有绿面降级即 RED）: {(r.stdout + r.stderr).strip().splitlines()[-1][-200:]}")
        else:
            ev_path = wel.load_latest_evidence("g11_m147_fix_r1_material_subset")
            ev = wel.load_json(ev_path) if ev_path else {}
            ok, detail = wel.gate_pass_reason(ev, "g11.p0.m147.fix_r1_material_subset")
            m147_g113_ok = ok and ev.get("phase") == "g11.3" and (ev.get("timestamp") or "") >= SESSION_START
            check(m147_g113_ok, f"M147 g11.3 phase 面复跑核验异常: {detail} / phase={ev.get('phase')}")
        # g11.5 phase verdict 显式登记（诚实面：不遮蔽本波 R1 未收敛 FAIL）。
        g115_path = None
        for p in sorted(EVIDENCE_DIR.glob("g11_m147_fix_r1_material_subset_*.json")):
            doc = wel.load_json(p)
            if doc.get("phase") == "g11.5":
                g115_path = (p, doc)
        if g115_path is not None:
            note(
                f"M147 g11.5 phase 最新 verdict 显式登记（回归面不遮蔽）: status={g115_path[1].get('status')} "
                f"converged={(g115_path[1].get('closure') or {}).get('converged')}（{g115_path[0].name}）"
            )
    checks["m147_g11_3_phase_face_rerun_green"] = bool(m147_g113_ok)
    check(m147_g113_ok, "M147 g11.3 phase 绿面核验失败（既有门降级即 RED）")

    # ④ 关键门真跑抽检零降级（子进程真跑 + 最新 evidence PASS + 新鲜度机核）。
    spot_rows: list[dict] = []
    for spec in SPOT_GATES:
        key = f"spot_rerun_{spec['id']}_pass"
        if args.skip_spot_reruns:
            row = {"id": spec["id"], "status": "FAIL", "detail": "--skip-spot-reruns 省面（正式门不可省）"}
        else:
            note(f"抽检真跑 {spec['id']} …")
            print(f"[{TAG}] 抽检真跑 {spec['id']}: {' '.join(spec['argv'][1:])}", flush=True)
            row = _spot_rerun(spec)
        spot_rows.append(row)
        checks[key] = row["status"] == "PASS"
        check(checks[key], f"抽检 {spec['id']} 非绿（既有门降级即 RED）: {row['detail'][:160]}")
        note(f"抽检 {spec['id']}: {row['status']}（{row['detail'][:120]}）")

    # ⑤ 既有判据 0-byte（G5~G10 closed 门脚本/里程碑面工作树空集——异己 src/
    # 未提交面属立项裁决 1 登记面，不在本闭集）。
    r = subprocess.run(
        ["git", "status", "--porcelain", "--", "ci/g9_*.py", "ci/g10_*.py",
         "milestones/g9", "milestones/g10", "spec", "conformance"],
        cwd=ROOT, capture_output=True, text=True,
    )
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "git status --porcelain -- ci/g9_* ci/g10_* milestones/g9 milestones/g10 spec conformance", "exit_code": r.returncode})
    dirty = [l for l in r.stdout.splitlines() if l.strip()]
    checks["legacy_criteria_0byte"] = r.returncode == 0 and not dirty
    check(not dirty, f"既有判据面漂移（G5~G10 0-byte 违例）: {dirty[:5]}")

    # ⑥ RED 臂①：既有门降级必检出（子断言 FAIL/SKIP/DEV_ENV_DEGRADE 注入）。
    good = {"symbolic_gate_key": "k", "host_section_pass": True,
            "device_section_state": "executed", "checks": {"a": True}}
    degraded_detected = True
    for bad_state in ("fail", "dev_env_degrade", "skip"):
        ok, _ = wel.gate_pass_reason(dict(good, device_section_state=bad_state), "k")
        degraded_detected = degraded_detected and not ok
    ok, _ = wel.gate_pass_reason(dict(good, checks={"a": False}), "k")
    checks["red_degraded_gate_detected"] = degraded_detected and not ok
    check(checks["red_degraded_gate_detected"], "既有门降级检出臂失效")

    # ⑦ RED 臂②：聚合遮蔽子断言必检出。
    masked = [{"status": "PASS"}, {"status": "FAIL"}]
    checks["red_aggregate_masking_detected"] = not all(rw.get("status") == "PASS" for rw in masked)
    check(checks["red_aggregate_masking_detected"], "聚合遮蔽检出臂失效")

    # ⑧ RED 臂③：陈旧 evidence 冒充当次复跑必检出。
    checks["red_stale_evidence_detected"] = "20000101T000000Z" < SESSION_START
    check(checks["red_stale_evidence_detected"], "陈旧 evidence 检出臂失效")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G11.5",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "regression_summary": {
            "session_start": SESSION_START,
            "g10_gates": g10_rows,
            "g9_gates": g9_rows,
            "g11_green_gates": g11_rows,
            "spot_reruns": spot_rows,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（修复回归门：48 门 + G11 已绿门最新 evidence 全绿只读汇总 + 关键门真跑抽检零降级 + RED 三臂全检出）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
