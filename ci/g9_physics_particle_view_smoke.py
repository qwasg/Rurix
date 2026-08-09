#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.2 M121 physics_particle_view 硬门冒烟(g9.p0.m121.physics_particle_view;
RFC-0024 §4.A;判据事实源 = G9_ACCEPTANCE_MAP.md M121 行)。骨架期 --phase g9.2。

host 恒跑 / device not_applicable。7 checks:
五域 adapter + 写路径仅 impulse/force 结构性断言 + 旁路写注入 RED +
名义类型隔离 + M68 迁移 digest golden + journal 全消费 + 单向事实源 0-byte。

双 phase 纪律:骨架期 phase_g9_2_pass 由本门写入;phase_g9_6_pass 恒 false
(完整期未跑,骨架期绿不替完整期充绿)。

用法:
  py -3 ci/g9_physics_particle_view_smoke.py --gate g9.p0.m121.physics_particle_view --phase g9.2
  py -3 ci/g9_physics_particle_view_smoke.py --selftest
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
SOURCE = ROOT / "conformance" / "physics" / "fracture" / "pillar_prefracture" / "source.json"
GOLDEN = ROOT / "conformance" / "physics" / "particle_view" / "m68_migration_golden.json"
SCHEMA = ROOT / "milestones" / "g9" / "g9_m121_physics_particle_view_evidence_schema.json"

GATE_KEY = "g9.p0.m121.physics_particle_view"
NUMERIC_STEP = 136
SUBJECT = "g9_m121_physics_particle_view"
SOURCE_REF = "RFC-0024 §4.A;G9_ACCEPTANCE_MAP M121;G9.2 骨架期(双 phase:--phase g9.2)"

CHECK_KEYS = [
    "five_domain_adapters_implemented",
    "write_path_impulse_only_structural",
    "bypass_write_injection_rejected",
    "nominal_type_isolation",
    "m68_migration_digest_equal",
    "journal_fully_consumed",
    "one_way_fact_source_zero_byte",
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
    print("[g9_m121] cargo build -p g9-physics-gates")
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
        print(f"[g9_m121] missing {exe}", file=sys.stderr)
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

    print("[g9_m121] particle-view --source/--golden")
    code, doc, out = run_gates(
        exe, ["particle-view", "--source", str(SOURCE), "--golden", str(GOLDEN)]
    )
    if code != 0 or doc is None:
        print(f"[g9_m121] harness failed: {out[-600:]}", file=sys.stderr)
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
        "milestone": "M121",
        "assertion_id": GATE_KEY,
        "status": "pass" if host_pass else "fail",
        "matrix_row": "M121",
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
                    "g9-physics-gates particle-view --source "
                    "conformance/physics/fracture/pillar_prefracture/source.json "
                    "--golden conformance/physics/particle_view/m68_migration_golden.json"
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
        "notes": doc.get("detail") or "M121 particle view 骨架期",
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
    out_path.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"[g9_m121] evidence → {out_path.relative_to(ROOT)}")
    for k, v in checks.items():
        print(f"  check {k}: {'PASS' if v else 'FAIL'}")
    print(f"  phase_g9_2_pass={evidence['phase_g9_2_pass']} phase_g9_6_pass=False (骨架期诚实)")
    if FAILURES:
        print("[g9_m121] FAILURES:", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
    print(f"[g9_m121] VERDICT = {'PASS' if host_pass else 'FAIL'}")
    return 0 if host_pass else 1


def run_selftest() -> int:
    """负样本:缺 golden( digest 漂移)→ 门必须红。"""
    exe = build_gates()
    missing = ROOT / "conformance" / "physics" / "particle_view" / "__missing__.json"
    code, doc, _ = run_gates(
        exe, ["particle-view", "--source", str(SOURCE), "--golden", str(missing)]
    )
    if code == 0 and doc and doc.get("ok"):
        print("[selftest] FAIL: missing golden still green", file=sys.stderr)
        return 1
    print("[selftest] PASS: missing golden → red")
    # 负样本 2:golden digest 篡改 → m68_migration_digest_equal 必须红。
    tampered = ROOT / "conformance" / "physics" / "particle_view" / "__tampered__.json"
    golden_doc = json.loads(GOLDEN.read_text(encoding="utf-8"))
    golden_doc["migration_digest"] = "0" * 64
    tampered.write_text(
        json.dumps(golden_doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    try:
        code2, doc2, _ = run_gates(
            exe, ["particle-view", "--source", str(SOURCE), "--golden", str(tampered)]
        )
        if code2 == 0 and doc2 and doc2.get("m68_migration_digest_equal"):
            print("[selftest] FAIL: tampered digest still green", file=sys.stderr)
            return 1
        print("[selftest] PASS: tampered golden digest → red")
    finally:
        tampered.unlink(missing_ok=True)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G9.2 M121 physics_particle_view smoke")
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
            f"[g9_m121] --phase {args.phase} 完整期未落地(G9.6);骨架期绿不替完整期充绿",
            file=sys.stderr,
        )
        return 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
