#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.2+G9.6 M122 gameplay_field 硬门冒烟(g9.p0.m122.gameplay_field;
RFC-0024 §4.B + v1.1 章 F2,R-3/R-7/R-10 🔒;spec/physics.md RXS-0375;
判据事实源 = G9_ACCEPTANCE_MAP.md M122 行)。骨架期 --phase g9.2 /
完整期 --phase g9.6。

host 恒跑 / device not_applicable。骨架期 7 checks:
三层解耦 schema 冻结 + 八枚举逐项 accept + 非法枚举 RED +
过滤默认空匹配零影响(真世界逐 tick 对拍)+ persistent journal replay
逐 tick hash + World-Field 唯一出口只读 buffer + 渲染侧零回写静态审计。

完整期 8+2 checks(RXS-0375;G9_ACCEPTANCE_MAP §2 M122 行):
场求值实际驱动力学响应(消费 RXS-0374 耦合面)+ 过滤默认空匹配零影响
完整期重验 + persistent 注册/注销/变更全 journal 化并入 M66 capture 主流
且 replay 逐 tick hash 一致 + World-Field 唯一出口 = GpuScene 只读 buffer
(F2 授权面恰在位 + 唯一提交点)+ 渲染侧写/回写注入 typed Err RED +
旁路提交注入检出 RED + 锚定语料消费 + measured 冻结带对拍 +
门序前置(gate_order_m121_full_passed:RXS-0375 硬约束,M121 完整期未绿
本门不得验收,ci/g9_physics_interlock.py 机器阻断)+ 完整期自证红臂全检出。

双 phase 纪律同 M121:骨架期 evidence phase_g9_6_pass 恒 false;完整期
evidence 同时真跑骨架期回归与完整期门,两者各自实测写入,任一阶段绿不替
另一阶段充绿。

用法:
  py -3 ci/g9_gameplay_field_smoke.py --gate g9.p0.m122.gameplay_field --phase g9.2
  py -3 ci/g9_gameplay_field_smoke.py --gate g9.p0.m122.gameplay_field --phase g9.6
  py -3 ci/g9_gameplay_field_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import subprocess
import sys
from pathlib import Path

# 允许同目录 import(门序机器阻断共享小库,沿 g9_gi_interlock 先例)
sys.path.insert(0, str(Path(__file__).resolve().parent))

import g9_physics_interlock as pi  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
GOLDEN = ROOT / "conformance" / "physics" / "field" / "field_golden.json"
SCHEMA = ROOT / "milestones" / "g9" / "g9_m122_gameplay_field_evidence_schema.json"

GATE_KEY = "g9.p0.m122.gameplay_field"
NUMERIC_STEP = 137
SUBJECT = "g9_m122_gameplay_field"
SOURCE_REF = "RFC-0024 §4.B;G9_ACCEPTANCE_MAP M122;G9.2 骨架期(双 phase:--phase g9.2)"
SOURCE_REF_FULL = (
    "RFC-0024 §4.B + v1.1 章 F2;spec/physics.md RXS-0375;G9_ACCEPTANCE_MAP M122;"
    "G9.6 完整期(双 phase:--phase g9.6)"
)
FULL_FREEZE = ROOT / "milestones" / "g9" / "g9_m122_world_field_readonly_freeze.json"

CHECK_KEYS = [
    "three_layer_schema_frozen",
    "eight_enum_accept_green",
    "illegal_enum_red",
    "filter_default_empty_zero_impact",
    "persistent_journal_replay_hash_equal",
    "world_field_egress_readonly",
    "render_zero_writeback_audit",
]

# 完整期判据键(harness field-full 直出 8 键)+ 门序前置键 + 红臂检出键。
FULL_CHECK_KEYS = [
    "field_drives_dynamic_response_full",
    "filter_default_empty_zero_impact_full",
    "persistent_journal_mainstream_replay_equal",
    "world_field_egress_unique_authorized_f2",
    "render_write_injection_rejected",
    "bypass_submit_detected",
    "conformance_anchor_consumed",
    "measured_freeze_digest_match",
    "gate_order_m121_full_passed",
    "full_selftest_red_arms_detected",
]

# 完整期自证红臂(harness field-full-selftest --arm):臂失效 = 漏检即门红。
FULL_SELFTEST_ARMS = [
    "render_write_injection",
    "bypass_submit",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def utc_stamp() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True, cwd=ROOT)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def build_gates() -> Path:
    print("[g9_m122] cargo build -p g9-physics-gates")
    r = subprocess.run(
        ["cargo", "build", "-p", "g9-physics-gates", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(r.stdout + r.stderr, file=sys.stderr)
        sys.exit(1)
    name = "g9-physics-gates.exe" if sys.platform == "win32" else "g9-physics-gates"
    exe = ROOT / "target" / "debug" / name
    if not exe.is_file():
        print(f"[g9_m122] missing {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def run_gates(exe: Path, args: list[str]) -> tuple[int, dict | None, str]:
    r = subprocess.run([str(exe), *args], cwd=ROOT, capture_output=True, text=True)
    text = (r.stdout or "").strip().splitlines()
    last = text[-1] if text else ""
    doc = None
    try:
        doc = json.loads(last)
    except Exception:
        pass
    return r.returncode, doc, r.stdout + r.stderr


def run_gate() -> int:
    checks = {k: False for k in CHECK_KEYS}
    exe = build_gates()

    print("[g9_m122] field --golden")
    code, doc, out = run_gates(exe, ["field", "--golden", str(GOLDEN)])
    if code != 0 or doc is None:
        print(f"[g9_m122] harness failed: {out[-600:]}", file=sys.stderr)
        return 1
    for k in CHECK_KEYS:
        checks[k] = bool(doc.get(k))
        check(checks[k], f"{k} not true")

    host_pass = bool(doc.get("ok")) and all(checks.values()) and code == 0 and not FAILURES
    stamp = utc_stamp()
    base_commit = subprocess.run(
        ["git", "rev-parse", "HEAD"], capture_output=True, text=True, cwd=ROOT
    ).stdout.strip()
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "milestone": "M122",
        "assertion_id": GATE_KEY,
        "status": "pass" if host_pass else "fail",
        "matrix_row": "M122",
        "wave": "G9.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "phase_g9_2_pass": host_pass,
        # 双 phase 纪律:骨架期 phase_g9_6_pass 恒 false(完整期未跑,不充绿)。
        "phase_g9_6_pass": False,
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": checks,
        "commands": [
            {
                "seq": 1,
                "command": "cargo build -p g9-physics-gates",
                "exit_code": 0,
            },
            {
                "seq": 2,
                "command": (
                    "g9-physics-gates field --golden "
                    "conformance/physics/field/field_golden.json"
                ),
                "exit_code": code,
            },
        ],
        "base_commit": base_commit,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": tool_version("cargo"),
            "rustc_version": tool_version("rustc"),
        },
        "notes": doc.get("detail") or "M122 gameplay field 骨架期",
    }

    try:
        import jsonschema

        errs = sorted(
            jsonschema.Draft7Validator(
                json.loads(SCHEMA.read_text(encoding="utf-8"))
            ).iter_errors(evidence),
            key=lambda e: list(e.path),
        )
        if errs:
            for e in errs:
                FAILURES.append(f"schema: {e.message}")
            host_pass = False
            evidence["host_section_pass"] = False
            evidence["status"] = "fail"
            evidence["phase_g9_2_pass"] = False
    except ImportError:
        NOTES.append("jsonschema missing; skipped local schema validate")

    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out_path = EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    # LF byte-exact 纪律:text mode 在 Windows 会写出 CRLF——显式 newline 钉死。
    with open(out_path, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(json.dumps(evidence, indent=2, ensure_ascii=False) + "\n")
    print(f"[g9_m122] evidence → {out_path.relative_to(ROOT)}")
    for k, v in checks.items():
        print(f"  check {k}: {'PASS' if v else 'FAIL'}")
    print(f"  phase_g9_2_pass={evidence['phase_g9_2_pass']} phase_g9_6_pass=False (骨架期诚实)")
    if FAILURES:
        print("[g9_m122] FAILURES:", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
    print(f"[g9_m122] VERDICT = {'PASS' if host_pass else 'FAIL'}")
    return 0 if host_pass else 1


def run_gate_full() -> int:
    """G9.6 完整期门(--phase g9.6;RXS-0375)。

    四段真跑:⓪ 门序前置(ci/g9_physics_interlock.py 机器阻断——M121 完整期
    未绿本门不得验收);① 骨架期回归(field,phase_g9_2_pass 实测写入);
    ② 完整期门(field-full --freeze 对拍 measured 冻结带,8 判据);
    ③ 完整期自证红臂(渲染侧写注入/旁路提交注入,臂漏检即门红)。
    """
    checks = {k: False for k in FULL_CHECK_KEYS}
    commands: list[dict] = []

    # —— ⓪ 门序前置(RXS-0375 硬约束;先于 cargo build 短路,阻断理由可见)——
    print("[g9_m122] full: 门序前置核验(M121 完整期绿?)")
    gate_order_ok, gate_order_detail = pi.m121_full_gate_passed()
    print(f"  {gate_order_detail}")
    checks["gate_order_m121_full_passed"] = gate_order_ok
    check(gate_order_ok, gate_order_detail)
    if not gate_order_ok:
        # 门序阻断:不跑后续段,诚实落 fail evidence 退 1。
        return _emit_full_evidence(
            checks=checks,
            commands=commands,
            scaffold_pass=False,
            full_pass=False,
            notes=gate_order_detail,
        )

    exe = build_gates()
    commands.append(
        {"seq": 1, "command": "cargo build -p g9-physics-gates", "exit_code": 0}
    )

    # —— ① 骨架期回归(0-byte 维持:完整期落地不得冲掉 G9.2 面)——
    print("[g9_m122] full: 骨架期回归 field --golden")
    code_s, doc_s, out_s = run_gates(exe, ["field", "--golden", str(GOLDEN)])
    commands.append(
        {
            "seq": 2,
            "command": (
                "g9-physics-gates field --golden "
                "conformance/physics/field/field_golden.json"
            ),
            "exit_code": code_s,
        }
    )
    scaffold_pass = (
        code_s == 0
        and doc_s is not None
        and bool(doc_s.get("ok"))
        and all(bool(doc_s.get(k)) for k in CHECK_KEYS)
    )
    check(scaffold_pass, f"骨架期回归非绿(--phase g9.2 面 0-byte 维持): {out_s[-300:]}")
    print(f"  骨架期回归: {'PASS' if scaffold_pass else 'FAIL'}")

    # —— ② 完整期门(measured 冻结带对拍,禁手写 golden)——
    print("[g9_m122] full: field-full --freeze(measured 冻结带对拍)")
    code, doc, out = run_gates(exe, ["field-full", "--freeze", str(FULL_FREEZE)])
    commands.append(
        {
            "seq": 3,
            "command": (
                "g9-physics-gates field-full --freeze "
                "milestones/g9/g9_m122_world_field_readonly_freeze.json"
            ),
            "exit_code": code,
        }
    )
    if code != 0 or doc is None:
        print(f"[g9_m122] full harness failed: {out[-600:]}", file=sys.stderr)
        return 1
    for k in FULL_CHECK_KEYS[:8]:
        checks[k] = bool(doc.get(k))
        check(checks[k], f"{k} not true")

    # —— ③ 完整期自证红臂实测必红(臂失效 = 漏检即门红)——
    arms_ok = True
    for i, arm in enumerate(FULL_SELFTEST_ARMS):
        code_a, doc_a, out_a = run_gates(exe, ["field-full-selftest", "--arm", arm])
        commands.append(
            {
                "seq": 4 + i,
                "command": f"g9-physics-gates field-full-selftest --arm {arm}",
                "exit_code": code_a,
            }
        )
        arm_red = code_a == 0 and doc_a is not None and bool(doc_a.get("red_detected"))
        check(arm_red, f"full selftest arm {arm} 未检出(red_detected≠true): {out_a[-200:]}")
        arms_ok = arms_ok and arm_red
        print(f"  selftest arm {arm}: {'RED ok' if arm_red else 'MISS'}")
    checks["full_selftest_red_arms_detected"] = arms_ok

    full_pass = (
        code == 0
        and bool(doc.get("ok"))
        and all(checks[k] for k in FULL_CHECK_KEYS[:8])
        and arms_ok
    )
    return _emit_full_evidence(
        checks=checks,
        commands=commands,
        scaffold_pass=scaffold_pass,
        full_pass=full_pass,
        notes=doc.get("detail") or "M122 gameplay field 完整期",
    )


def _emit_full_evidence(
    *,
    checks: dict,
    commands: list[dict],
    scaffold_pass: bool,
    full_pass: bool,
    notes: str,
) -> int:
    """完整期 evidence 落盘 + schema 校验 + VERDICT(骨架/完整期双 phase 各自实测)。"""
    host_pass = (
        scaffold_pass
        and full_pass
        and checks.get("gate_order_m121_full_passed") is True
        and not FAILURES
    )
    stamp = utc_stamp()
    base_commit = subprocess.run(
        ["git", "rev-parse", "HEAD"], capture_output=True, text=True, cwd=ROOT
    ).stdout.strip()
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "milestone": "M122",
        "assertion_id": GATE_KEY,
        "status": "pass" if host_pass else "fail",
        "matrix_row": "M122",
        "wave": "G9.6",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF_FULL,
        # 双 phase 各自实测:任一阶段绿不替另一阶段充绿。
        "phase_g9_2_pass": scaffold_pass,
        "phase_g9_6_pass": full_pass,
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": checks,
        "commands": commands,
        "base_commit": base_commit,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": tool_version("cargo"),
            "rustc_version": tool_version("rustc"),
        },
        "notes": notes,
    }

    try:
        import jsonschema

        errs = sorted(
            jsonschema.Draft7Validator(
                json.loads(SCHEMA.read_text(encoding="utf-8"))
            ).iter_errors(evidence),
            key=lambda e: list(e.path),
        )
        if errs:
            for e in errs:
                FAILURES.append(f"schema: {e.message}")
            host_pass = False
            evidence["host_section_pass"] = False
            evidence["status"] = "fail"
            evidence["phase_g9_6_pass"] = False
    except ImportError:
        NOTES.append("jsonschema missing; skipped local schema validate")

    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out_path = EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    # LF byte-exact 纪律:text mode 在 Windows 会写出 CRLF——显式 newline 钉死。
    with open(out_path, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(json.dumps(evidence, indent=2, ensure_ascii=False) + "\n")
    print(f"[g9_m122] evidence → {out_path.relative_to(ROOT)}")
    for k, v in checks.items():
        print(f"  check {k}: {'PASS' if v else 'FAIL'}")
    print(
        f"  phase_g9_2_pass={evidence['phase_g9_2_pass']} "
        f"phase_g9_6_pass={evidence['phase_g9_6_pass']} (双 phase 各自实测)"
    )
    if FAILURES:
        print("[g9_m122] FAILURES:", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
    print(f"[g9_m122] VERDICT = {'PASS' if host_pass else 'FAIL'}")
    return 0 if host_pass else 1


def run_selftest() -> int:
    """三臂负样本:非法枚举 / 篡改 replay / 非空 filter 反零影响。"""
    exe = build_gates()
    # 臂 1:非法枚举必须 RED。
    code, doc, out = run_gates(exe, ["field-selftest", "--arm", "illegal_enum"])
    if code != 0 or not doc or not doc.get("red_detected"):
        print(f"[selftest] FAIL: illegal_enum arm not red: {out[-300:]}", file=sys.stderr)
        return 1
    print("[selftest] PASS: illegal_enum → red")
    # 臂 2:篡改 replay hash 必须 RED。
    code, doc, out = run_gates(exe, ["field-selftest", "--arm", "tampered_replay"])
    if code != 0 or not doc or not doc.get("red_detected") or not doc.get("baseline_ok"):
        print(f"[selftest] FAIL: tampered_replay arm not red: {out[-300:]}", file=sys.stderr)
        return 1
    print("[selftest] PASS: tampered_replay → red")
    # 臂 3:非空 filter 必须有影响 + 显式 exclude 必须零匹配(过滤机制活)。
    code, doc, out = run_gates(exe, ["field-selftest", "--arm", "nonempty_filter_impact"])
    if (
        code != 0
        or not doc
        or not doc.get("impact_observed")
        or not doc.get("exclude_zero_match")
    ):
        print(
            f"[selftest] FAIL: nonempty_filter_impact arm broken: {out[-300:]}",
            file=sys.stderr,
        )
        return 1
    print("[selftest] PASS: nonempty_filter_impact + exclude_zero_match")
    # 臂 4:golden digest 篡改 → 门必须红。
    tampered = ROOT / "conformance" / "physics" / "field" / "__tampered__.json"
    golden_doc = json.loads(GOLDEN.read_text(encoding="utf-8"))
    golden_doc["journal_digest"] = "0" * 64
    tampered.write_text(
        json.dumps(golden_doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    try:
        code4, doc4, _ = run_gates(exe, ["field", "--golden", str(tampered)])
        if code4 == 0 and doc4 and doc4.get("persistent_journal_replay_hash_equal"):
            print("[selftest] FAIL: tampered journal digest still green", file=sys.stderr)
            return 1
        print("[selftest] PASS: tampered golden digest → red")
    finally:
        tampered.unlink(missing_ok=True)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G9.2+G9.6 M122 gameplay_field smoke")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    ap.add_argument("--phase", choices=["g9.2", "g9.6"], default="g9.2")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.phase == "g9.6":
        # G9.6 完整期(RXS-0375):门序前置 + 骨架期回归 + 完整期门 + 红臂四段真跑。
        return run_gate_full()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
