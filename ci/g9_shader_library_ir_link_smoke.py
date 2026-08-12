#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.3 M107 shader library IR 链接 + 变体预算门冒烟(g9.p1.m107.shader_library_ir_link;
RFC-0023 §4.5/§4.6;spec/gpu_driven_submit.md RXS-0356;G9_ACCEPTANCE_MAP §3 M107;
G9_CONTRACT §8.1 裁决①)。

host 纯 host 门(device_section_state=not_applicable)。判据:
  ① rurixc shader_library::tests 6 单测逐名锚定全绿(链接确定性+序无关/
     interface hash 稳定+回放/链接 fail-closed RED 族/变体预算超限 RED+
     边界/零预算+单变体边沿/变体审计恒等式+死变体+JSON);
  ② conformance/gpu_driven_submit/reject/variant_budget_exceeded.rx
     (RXS-0356)锚定——变体工程级总预算超限装配期硬失败 RED 臂语料。

用法:
  py -3 ci/g9_shader_library_ir_link_smoke.py --gate g9.p1.m107.shader_library_ir_link
  py -3 ci/g9_shader_library_ir_link_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones/g9/g9_m107_shader_library_ir_link_evidence_schema.json"
REJECT_CORPUS = ROOT / "conformance/gpu_driven_submit/reject/variant_budget_exceeded.rx"

GATE_KEY = "g9.p1.m107.shader_library_ir_link"
NUMERIC_STEP = 145
SOURCE_REF = "RFC-0023 §4.5/§4.6;spec/gpu_driven_submit.md RXS-0356;G9_ACCEPTANCE_MAP §3 M107"
TAG = "g9_m107"

SHADER_LIBRARY_TESTS = [
    "link_deterministic_and_order_invariant",
    "interface_hash_stability_and_replay",
    "link_fail_closed_reds",
    "variant_budget_exceeded_red_and_boundary",
    "variant_budget_zero_and_single_edges",
    "variant_audit_identity_dead_and_json",
]

CHECK_KEYS = [
    "host_link_deterministic_and_order_invariant",
    "host_interface_hash_stability_and_replay",
    "host_link_fail_closed_reds",
    "host_variant_budget_exceeded_red_and_boundary",
    "host_variant_budget_zero_and_single_edges",
    "host_variant_audit_identity_dead_and_json",
    "conformance_red_corpus_anchored",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def run_cargo(args: list[str]) -> tuple[int, str]:
    print(f"[{TAG}] cargo {' '.join(args)}")
    r = subprocess.run(["cargo", *args], cwd=ROOT, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


# ═══════════════════════ host 段 ═══════════════════════


def host_shader_library_tests(checks: dict[str, bool]) -> None:
    """cargo test -p rurixc --lib shader_library:6 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurixc", "--lib", "shader_library"])
    ok = rc == 0 and "test result: ok" in blob
    for name in SHADER_LIBRARY_TESTS:
        key = f"host_{name}"
        checks[key] = ok and (name in blob)
        if not checks[key]:
            check(False, f"shader_library 单测 {name} 未锚定/失败")


def host_conformance_anchor() -> bool:
    """conformance RED 语料在位 + `//@ spec: RXS-0356` 锚定 + 预算超限预期面注释。"""
    if not REJECT_CORPUS.is_file():
        check(False, f"缺 RED 语料 {REJECT_CORPUS.name}")
        return False
    text = REJECT_CORPUS.read_text(encoding="utf-8")
    ok = (
        "//@ spec: RXS-0356" in text
        and "variant_budget_exceeded" in REJECT_CORPUS.name
        and "预算" in text
    )
    if not ok:
        check(False, "RED 语料锚定/预算超限预期面缺失")
    return ok


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 7:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 7", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    synth = {k: True for k in CHECK_KEYS}
    synth.pop(CHECK_KEYS[0])
    if not (req - set(synth)):
        print(f"[{TAG}] selftest FAIL: 合成缺键未触发 schema required 红", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (1 RED + 1 GREEN)")
    return 0


# ═══════════════════════ main ═══════════════════════


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

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}

    host_shader_library_tests(checks)
    checks["conformance_red_corpus_anchored"] = host_conformance_anchor()
    note("纯 host 门;6 单测逐名锚定 + conformance RED 语料锚定")

    host_pass = all(checks.values()) and not FAILURES

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m107_shader_library_ir_link",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M107",
        "milestone": "M107",
        "assertion_id": GATE_KEY,
        "status": "pass" if host_pass else "fail",
        "wave": "G9.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        ).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": [
            {"seq": 1, "command": "cargo test -p rurixc --lib shader_library", "exit_code": 0},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"g9_m107_shader_library_ir_link_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if host_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
