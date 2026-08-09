#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.2 M122 gameplay_field 硬门冒烟(g9.p0.m122.gameplay_field;
RFC-0024 §4.B,R-3/R-7/R-10 🔒;判据事实源 = G9_ACCEPTANCE_MAP.md M122 行)。
骨架期 --phase g9.2。

host 恒跑 / device not_applicable。7 checks:
三层解耦 schema 冻结 + 八枚举逐项 accept + 非法枚举 RED +
过滤默认空匹配零影响(真世界逐 tick 对拍)+ persistent journal replay
逐 tick hash + World-Field 唯一出口只读 buffer + 渲染侧零回写静态审计。

双 phase 纪律同 M121:phase_g9_6_pass 骨架期恒 false。

用法:
  py -3 ci/g9_gameplay_field_smoke.py --gate g9.p0.m122.gameplay_field --phase g9.2
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

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
GOLDEN = ROOT / "conformance" / "physics" / "field" / "field_golden.json"
SCHEMA = ROOT / "milestones" / "g9" / "g9_m122_gameplay_field_evidence_schema.json"

GATE_KEY = "g9.p0.m122.gameplay_field"
NUMERIC_STEP = 137
SUBJECT = "g9_m122_gameplay_field"
SOURCE_REF = "RFC-0024 §4.B;G9_ACCEPTANCE_MAP M122;G9.2 骨架期(双 phase:--phase g9.2)"

CHECK_KEYS = [
    "three_layer_schema_frozen",
    "eight_enum_accept_green",
    "illegal_enum_red",
    "filter_default_empty_zero_impact",
    "persistent_journal_replay_hash_equal",
    "world_field_egress_readonly",
    "render_zero_writeback_audit",
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
    ap = argparse.ArgumentParser(description="G9.2 M122 gameplay_field smoke")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    ap.add_argument("--phase", choices=["g9.2", "g9.6"], default="g9.2")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.phase != "g9.2":
        # 完整期未实现:诚实退出非零,不充绿(MAP 双 phase 纪律)。
        print(
            f"[g9_m122] --phase {args.phase} 完整期未落地(G9.6);骨架期绿不替完整期充绿",
            file=sys.stderr,
        )
        return 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
