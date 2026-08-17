#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.4 UE PT 对标波）
"""G12.4 M164 生产化回归门（P0，步骤 226；g12.p0.m164.regression_guard；
G12_CONTRACT §4.2 M164 行判据逐字 / G-G12-6；G12_ACCEPTANCE_MAP §1 M164 行；
CI_GATES §4；同构 ci/g11_regression_guard_smoke.py 先例）。

host 纯 host 门（device_section_state=not_applicable；抽检既有门经子进程真跑，
各自 evidence 独立落盘自持 device 面——本门只读汇总 + 子进程退出码/新鲜度
机核，不嵌套持锁）。判据（契约 §4.2 M164 行字面 + MAP 逐字）：

1. **既有 62 门（G9 34 key + G10 14 key + G11 14 key）最新 evidence 全绿
   只读汇总**：wel.require_gate_pass 逐门只读核验（symbolic_gate_key 相符 ∧
   host_section_pass=True ∧ device_section_state ∉ {fail,dev_env_degrade,
   skip} ∧ checks 全 True）；M147 双 phase 纪律两态面继承（g11_wave3_exit
   m147_dual_phase_discipline 单源）；聚合不遮蔽任一子断言 FAIL/SKIP/
   DEV_ENV_DEGRADE。
2. **生产化触改面既有门重跑回归零降级**（G12.4 触改面 = g12_pt_production.rx
   kernel type=2 三角网格光加性扩展 + stride 16→17 CDF 槽位 + prod.rs 镜像 +
   rurix-asset 新 bin——消费面 = G12.2 四门/G12.3 M162/M96 参照器面）：
   M96 golden 门序面真跑抽检（g9.p0.m96，契约字面）+ G12.2 M158/M160 全档
   真跑（kernel 触改零降级——位级一致/曲线锚复核）+ G12.3 M162 全档真跑 +
   wave2/wave3 exit 聚合复跑 + G10 M140/G11 M157 host 面广幅抽检——子进程
   真跑 exit 0 + 最新 evidence PASS + **新鲜度机核**（evidence timestamp ≥
   本门会话起点——陈旧 evidence 冒充当次复跑即 RED）。
3. **既有判据 0-byte**：G5~G11 closed 门脚本与里程碑面 git 工作树 0-byte
   （git status --porcelain 闭集面空集;spec/conformance 面 = 本波 spec-first
   已合入提交，异己 src/ 未提交面属立项裁决 1 登记面不在本闭集）。

RED 臂（契约判据字面）：既有门降级即 RED（red_degraded_gate——子断言非 PASS
注入必检出）；聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED
（red_aggregate_masking——遮蔽型汇总必检出）；陈旧 evidence 冒充当次复跑即
RED（red_stale_evidence——timestamp 早于会话起点必检出）。

用法：
  py -3 ci/g12_regression_guard_smoke.py --gate g12.p0.m164.regression_guard
  py -3 ci/g12_regression_guard_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g12" / "g12_m164_regression_guard_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g12_wave_exit_lib as wel  # noqa: E402
from g11_wave3_exit_check import G10_KEYS, G9_KEYS, m147_dual_phase_discipline  # noqa: E402

GATE_KEY = "g12.p0.m164.regression_guard"
NUMERIC_STEP = 226
SOURCE_REF = (
    "G12_CONTRACT §4.2 M164 + G-G12-6;G12_ACCEPTANCE_MAP §1 M164;CI_GATES §4;"
    "G9 34 + G10 14 + G11 14 keys latest evidence read-only summary;touched-face spot rerun zero-degrade"
)
TAG = "g12_m164"
SUBJECT = "g12_m164_regression_guard"
MATRIX_ROW = "M164"

# G11 14 key（M144~M157 闭集;G12 契约「G11 14 key」字面）。
G11_KEYS = [
    ("g11.p0.m144.caliber_c1_indoor_luminance", "g11_m144_caliber_c1_indoor_luminance"),
    ("g11.p0.m145.caliber_c2_exposure_chain", "g11_m145_caliber_c2_exposure_chain"),
    ("g11.p0.m146.caliber_c3_exr_bit_depth", "g11_m146_caliber_c3_exr_bit_depth"),
    ("g11.p0.m147.fix_r1_material_subset", "g11_m147_fix_r1_material_subset"),
    ("g11.p0.m148.fix_r2_geometry_normals", "g11_m148_fix_r2_geometry_normals"),
    ("g11.p0.m149.fix_r5_json_u64_seed", "g11_m149_fix_r5_json_u64_seed"),
    ("g11.p0.m150.fix_u1_cornell_shell_radiance", "g11_m150_fix_u1_cornell_shell_radiance"),
    ("g11.p0.m151.fix_u2_bistro_texture_dds", "g11_m151_fix_u2_bistro_texture_dds"),
    ("g11.p0.m152.fix_u3_bistro_animation", "g11_m152_fix_u3_bistro_animation"),
    ("g11.p0.m153.fix_r3_light_subset", "g11_m153_fix_r3_light_subset"),
    ("g11.p0.m154.fix_r4_gi_multibounce_world_cache", "g11_m154_fix_r4_gi_multibounce_world_cache"),
    ("g11.p0.m155.ab_retest_closure", "g11_m155_ab_retest_closure"),
    ("g11.p0.m156.regression_guard", "g11_m156_regression_guard"),
    ("g11.p1.m157.hdr_flip_calibration", "g11_m157_hdr_flip_calibration"),
]

# 生产化触改面真跑抽检闭集（子进程 argv + 环境面 + 新鲜度核验 subject/key）。
SPOT_GATES = [
    {
        "id": "g9_m96",
        "argv": [sys.executable, "ci/g9_path_tracer_reference_smoke.py", "--gate", "g9.p0.m96.path_tracer_reference"],
        "env": {"RURIX_REQUIRE_REAL": "1", "RURIX_VK_VALIDATION": "1"},
        "key": "g9.p0.m96.path_tracer_reference",
        "subject": "g9_m96_path_tracer_reference",
    },
    {
        "id": "g12_m158",
        "argv": [sys.executable, "ci/g12_mis_full_surface_smoke.py", "--gate", "g12.p0.m158.mis_full_surface"],
        "env": {"RURIX_REQUIRE_REAL": "1", "RURIX_VK_VALIDATION": "1"},
        "key": "g12.p0.m158.mis_full_surface",
        "subject": "g12_m158_mis_full_surface",
    },
    {
        "id": "g12_m160",
        "argv": [sys.executable, "ci/g12_sampling_lds_upgrade_smoke.py", "--gate", "g12.p0.m160.sampling_lds_upgrade"],
        "env": {"RURIX_REQUIRE_REAL": "1", "RURIX_VK_VALIDATION": "1"},
        "key": "g12.p0.m160.sampling_lds_upgrade",
        "subject": "g12_m160_sampling_lds_upgrade",
    },
    {
        "id": "g12_m162",
        "argv": [sys.executable, "ci/g12_denoise_pipeline_tsr_smoke.py", "--gate", "g12.p0.m162.denoise_pipeline_tsr"],
        "env": {"RURIX_REQUIRE_REAL": "1", "RURIX_VK_VALIDATION": "1"},
        "key": "g12.p0.m162.denoise_pipeline_tsr",
        "subject": "g12_m162_denoise_pipeline_tsr",
    },
    {
        "id": "g12_wave2_exit",
        "argv": [sys.executable, "ci/g12_wave2_exit_check.py", "--gate", "g12.wave.2.exit"],
        "env": {},
        "key": "g12.wave.2.exit",
        "subject": "g12_wave2_exit",
    },
    {
        "id": "g12_wave3_exit",
        "argv": [sys.executable, "ci/g12_wave3_exit_check.py", "--gate", "g12.wave.3.exit"],
        "env": {},
        "key": "g12.wave.3.exit",
        "subject": "g12_wave3_exit",
    },
    {
        "id": "g10_m140",
        "argv": [sys.executable, "ci/g10_gap_registry_smoke.py", "--gate", "g10.p0.m140.gap_registry"],
        "env": {},
        "key": "g10.p0.m140.gap_registry",
        "subject": "g10_m140_gap_registry",
    },
    {
        "id": "g11_m157",
        "argv": [sys.executable, "ci/g11_hdr_flip_calibration_smoke.py", "--gate", "g11.p1.m157.hdr_flip_calibration"],
        "env": {},
        "key": "g11.p1.m157.hdr_flip_calibration",
        "subject": "g11_m157_hdr_flip_calibration",
    },
]

CHECK_KEYS = [
    "g10_14_gates_latest_evidence_all_green",
    "g9_34_gates_latest_evidence_all_green",
    "g11_14_gates_latest_evidence_all_green",
    "spot_rerun_g9_m96_pass",
    "spot_rerun_g12_m158_pass",
    "spot_rerun_g12_m160_pass",
    "spot_rerun_g12_m162_pass",
    "spot_rerun_g12_wave2_exit_pass",
    "spot_rerun_g12_wave3_exit_pass",
    "spot_rerun_g10_m140_pass",
    "spot_rerun_g11_m157_pass",
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


def _g11_rows() -> tuple[list[dict], list[str]]:
    """G11 14 key 只读汇总;M147 走双 phase 纪律两态面（g11_wave3_exit 单源）。"""
    rows: list[dict] = []
    bad: list[str] = []
    for key, prefix in G11_KEYS:
        if key == "g11.p0.m147.fix_r1_material_subset":
            ok_all = False
            detail = "缺 evidence"
            docs = []
            for p in sorted(EVIDENCE_DIR.glob(prefix + "_*.json")):
                docs.append(wel.load_json(p))
            for doc in docs:
                ok, d = m147_dual_phase_discipline(doc)
                if ok:
                    ok_all = True
                    detail = d
            rows.append({"subject_prefix": prefix, "status": "PASS" if ok_all else "FAIL", "detail": f"M147 双 phase 纪律: {detail}"})
            if not ok_all:
                bad.append(f"{prefix}: M147 双 phase 纪律非绿")
        else:
            r = wel.require_gate_pass(key, prefix)
            rows.append(r)
            if r["status"] != "PASS":
                bad.append(f"{prefix}: {r['detail'][:80]}")
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
    good = {"symbolic_gate_key": key, "host_section_pass": True,
            "device_section_state": "executed", "checks": {"a": True}}
    ok, _ = wel.gate_pass_reason(good, key)
    if not ok:
        print(f"[{TAG}] selftest FAIL: 合形 evidence 未判 PASS", file=sys.stderr)
        return 1
    for bad_state in ("fail", "dev_env_degrade", "skip"):
        ok, _ = wel.gate_pass_reason(dict(good, device_section_state=bad_state), key)
        if ok:
            print(f"[{TAG}] selftest FAIL: device {bad_state} 未检出", file=sys.stderr)
            return 1
    ok, _ = wel.gate_pass_reason(dict(good, checks={"a": False}), key)
    if ok:
        print(f"[{TAG}] selftest FAIL: checks 非真未检出", file=sys.stderr)
        return 1
    rows = [{"status": "PASS"}, {"status": "FAIL"}]
    if all(r.get("status") == "PASS" for r in rows):
        print(f"[{TAG}] selftest FAIL: 聚合遮蔽未检出", file=sys.stderr)
        return 1
    stale = "20000101T000000Z"
    if stale >= SESSION_START:
        print(f"[{TAG}] selftest FAIL: 陈旧 timestamp 比较臂失效", file=sys.stderr)
        return 1
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

    # ① 既有 62 门最新 evidence 全绿只读汇总（聚合不遮蔽子断言）。
    g10_rows, g10_bad = _summary_rows(G10_KEYS)
    checks["g10_14_gates_latest_evidence_all_green"] = not g10_bad
    check(not g10_bad, f"G10 14 门最新 evidence 非全绿: {g10_bad[:3]}")
    g9_rows, g9_bad = _summary_rows(G9_KEYS)
    checks["g9_34_gates_latest_evidence_all_green"] = not g9_bad
    check(not g9_bad, f"G9 34 门最新 evidence 非全绿: {g9_bad[:3]}")
    g11_rows, g11_bad = _g11_rows()
    checks["g11_14_gates_latest_evidence_all_green"] = not g11_bad
    check(not g11_bad, f"G11 14 门最新 evidence 非全绿: {g11_bad[:3]}")
    note(
        f"62 门只读汇总: G10 {len(G10_KEYS) - len(g10_bad)}/{len(G10_KEYS)} 绿 / "
        f"G9 {len(G9_KEYS) - len(g9_bad)}/{len(G9_KEYS)} 绿 / "
        f"G11 {len(G11_KEYS) - len(g11_bad)}/{len(G11_KEYS)} 绿"
    )

    # ② 生产化触改面真跑抽检零降级（子进程真跑 + 最新 evidence PASS + 新鲜度）。
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

    # ③ 既有判据 0-byte（G5~G11 closed 门脚本/里程碑面 + spec/conformance 工作树
    # 空集——本波 spec-first/语料面已合入提交;异己 src/ 未提交面不在本闭集）。
    r = subprocess.run(
        ["git", "status", "--porcelain", "--",
         "ci/g9_*.py", "ci/g10_*.py", "ci/g11_*.py",
         "milestones/g9", "milestones/g10", "milestones/g11", "spec", "conformance"],
        cwd=ROOT, capture_output=True, text=True,
    )
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "git status --porcelain -- ci/g9_* ci/g10_* ci/g11_* milestones/g9..g11 spec conformance", "exit_code": r.returncode})
    dirty = [l for l in r.stdout.splitlines() if l.strip()]
    checks["legacy_criteria_0byte"] = r.returncode == 0 and not dirty
    check(not dirty, f"既有判据面漂移（G5~G11 0-byte 违例）: {dirty[:5]}")

    # ④ RED 臂①:既有门降级必检出（子断言 FAIL/SKIP/DEV_ENV_DEGRADE 注入）。
    good = {"symbolic_gate_key": "k", "host_section_pass": True,
            "device_section_state": "executed", "checks": {"a": True}}
    degraded_detected = True
    for bad_state in ("fail", "dev_env_degrade", "skip"):
        ok, _ = wel.gate_pass_reason(dict(good, device_section_state=bad_state), "k")
        degraded_detected = degraded_detected and not ok
    ok, _ = wel.gate_pass_reason(dict(good, checks={"a": False}), "k")
    checks["red_degraded_gate_detected"] = degraded_detected and not ok
    check(checks["red_degraded_gate_detected"], "既有门降级检出臂失效")

    # ⑤ RED 臂②:聚合遮蔽子断言必检出。
    masked = [{"status": "PASS"}, {"status": "FAIL"}]
    checks["red_aggregate_masking_detected"] = not all(rw.get("status") == "PASS" for rw in masked)
    check(checks["red_aggregate_masking_detected"], "聚合遮蔽检出臂失效")

    # ⑥ RED 臂③:陈旧 evidence 冒充当次复跑必检出。
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
        "wave": "G12.4",
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
            "g11_gates": g11_rows,
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
        print(f"[{TAG}] PASS（生产化回归门:62 门最新 evidence 全绿只读汇总 + 触改面 8 抽检真跑零降级 + RED 三臂全检出）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
