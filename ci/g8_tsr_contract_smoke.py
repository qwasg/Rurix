#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.5b M24 tsr_contract 硬门冒烟(g8.p0.m24.tsr_contract)。

host:temporal::contract 单测 + 五 case 语义。
device:uc06-renderer --m24-tsr-contract(真实 GPU 序列对拍)。
tolerance 两段式:本脚本可 --write-freeze 产 measured_local_freeze;
RFC-0019 修订行 + g8_budget 由 Gov 接线(本 agent 禁改)。

用法:
  py -3 ci/g8_tsr_contract_smoke.py --gate g8.p0.m24.tsr_contract
  py -3 ci/g8_tsr_contract_smoke.py --selftest
  py -3 ci/g8_tsr_contract_smoke.py --write-freeze
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
FREEZE_PATH = ROOT / "tests" / "tsr_contract" / "freeze.json"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_m24_tsr_contract_evidence_schema.json"
KERNEL_CONTRACT = ROOT / "apps" / "uc06-renderer" / "kernels" / "tsr_contract.rx"
KERNEL_RETIRE = ROOT / "apps" / "uc06-renderer" / "kernels" / "tsr_retire.rx"

GATE_KEY = "g8.p0.m24.tsr_contract"
NUMERIC_STEP = 117
SOURCE_REF = (
    "G8_ACCEPTANCE_MAP §2 M24;G8.5_RENDERING_COMPLETION_DESIGN §3;"
    "RFC-0019 §4.6(tolerance 两段式:local freeze → Gov RFC/budget)"
)
TAG = "g8_m24"
WAVE = "G8.5b"

CASE_SET = [
    "history_resurrection",
    "pixel_animation_velocity",
    "thin_geometry",
    "dynamic_resolution",
    "transparent_velocity",
]

CHECK_KEYS = [
    "host_oracle_regression",
    "case_set_exact",
    "case_history_resurrection",
    "case_pixel_animation_velocity",
    "case_thin_geometry",
    "case_dynamic_resolution",
    "case_transparent_velocity",
    "tolerance_frozen",
    "red_wrong_history_identity",
    "red_cross_cut_resurrection",
    "red_missing_previous_zero_motion",
    "not_satisfiable_by_taa",
    "validation_zero",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def extract_json(stdout: str) -> dict | None:
    text = stdout.strip()
    if not text:
        return None
    for line in reversed(text.splitlines()):
        line = line.strip()
        if line.startswith("{") and ("subject" in line or "checks" in line):
            try:
                return json.loads(line)
            except Exception:
                continue
    try:
        return json.loads(text)
    except Exception:
        return None


def run_device() -> tuple[str, dict | None]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    # render_exec:REQUIRE_REAL=1 强制 VALIDATION=1(ERROR count 不可 unavailable)
    env["RURIX_VK_VALIDATION"] = "1"
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "uc06-renderer",
        "--features",
        "vulkan",
        "--",
        "--m24-tsr-contract",
    ]
    print(f"[{TAG}] device: --m24-tsr-contract")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env, timeout=1200)
    doc = extract_json(r.stdout)
    merged = (r.stdout or "") + (r.stderr or "")
    if doc is None and "SKIP" in merged:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1):\n{merged[-2000:]}")
        return "skipped_dev_env", None
    if doc is None:
        check(
            False,
            f"device JSON 缺失 rc={r.returncode}\n{r.stderr[-2000:]}\n{r.stdout[-2000:]}",
        )
        return "fail", None
    if r.returncode != 0 and not doc.get("pass"):
        check(False, f"device 失败 rc={r.returncode} pass=false")
        return "fail", doc
    return "executed", doc


def write_freeze(doc: dict) -> None:
    FREEZE_PATH.parent.mkdir(parents=True, exist_ok=True)
    cases = doc.get("cases") or []
    by_name = {c["name"]: c for c in cases}
    freeze = {
        "schema_version": 1,
        "stage": "measured_local_freeze",
        "resurrection_age_max": 6,
        "device_name": doc.get("device_name", ""),
        "cases": [],
        "notes": (
            "measured → local freeze;Gov 须将逐 case tolerance/digest 写入 "
            "RFC-0019 加性修订行 + milestones/g8/g8_budget.json measured 条目后 "
            "stage 才升为 rfc_budget_frozen"
        ),
    }
    for name in CASE_SET:
        c = by_name.get(name)
        if not c:
            raise SystemExit(f"freeze 缺 case {name}")
        # 余量:measured * 2 + 1e-4 下限,避免 FMA 抖动假红
        measured = float(c.get("measured_max_abs") or 0.0)
        tol = max(measured * 2.0, 1e-4)
        freeze["cases"].append(
            {
                "name": name,
                "digest": c.get("digest", ""),
                "measured_max_abs": measured,
                "tolerance": tol,
            }
        )
    FREEZE_PATH.write_text(
        json.dumps(freeze, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(f"[{TAG}] freeze → {FREEZE_PATH}")


def load_freeze() -> dict | None:
    if not FREEZE_PATH.is_file():
        return None
    return json.loads(FREEZE_PATH.read_text(encoding="utf-8"))


def run_selftest() -> int:
    assert SCHEMA_PATH.is_file(), "缺 M24 evidence schema"
    assert KERNEL_CONTRACT.is_file(), "缺 tsr_contract.rx"
    assert KERNEL_RETIRE.is_file(), "缺 tsr_retire.rx"
    assert len(CHECK_KEYS) == 13
    assert len(CASE_SET) == 5
    src = KERNEL_CONTRACT.read_text(encoding="utf-8")
    assert "retired" in src and "coverage" in src
    fake = {k: False for k in CHECK_KEYS}
    fake["host_oracle_regression"] = True
    assert not all(fake.values())
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--write-freeze", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    checks = {k: False for k in CHECK_KEYS}

    print(f"[{TAG}] cargo test -p rurix-render temporal::contract::")
    tr = subprocess.run(
        ["cargo", "test", "-q", "-p", "rurix-render", "temporal::contract::"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    checks["host_oracle_regression"] = tr.returncode == 0
    if tr.returncode != 0:
        check(False, f"contract tests 失败:\n{tr.stdout}\n{tr.stderr}")

    device_state, doc = run_device()
    if args.write_freeze and doc:
        write_freeze(doc)

    freeze = load_freeze()
    freeze_stage = (freeze or {}).get("stage")
    checks["tolerance_frozen"] = bool(
        freeze
        and freeze_stage in ("measured_local_freeze", "rfc_budget_frozen")
        and len(freeze.get("cases") or []) == 5
    )
    if not checks["tolerance_frozen"]:
        check(False, f"缺 freeze({FREEZE_PATH});先 --write-freeze 或 Gov rfc_budget_frozen")
        note("tolerance unfrozen")
    elif freeze_stage == "rfc_budget_frozen":
        note("tolerance stage=rfc_budget_frozen (RFC-0019 §4.6.4 + g8_budget)")
    else:
        note("tolerance stage1=local freeze;RFC/budget Gov pending")

    if device_state == "executed" and doc:
        case_names = [c.get("name") for c in (doc.get("cases") or [])]
        checks["case_set_exact"] = case_names == CASE_SET
        if not checks["case_set_exact"]:
            check(False, f"case_set 非恰好五元组: {case_names}")

        by = {c["name"]: c for c in (doc.get("cases") or [])}
        mapping = {
            "history_resurrection": "case_history_resurrection",
            "pixel_animation_velocity": "case_pixel_animation_velocity",
            "thin_geometry": "case_thin_geometry",
            "dynamic_resolution": "case_dynamic_resolution",
            "transparent_velocity": "case_transparent_velocity",
        }
        for cname, ckey in mapping.items():
            c = by.get(cname) or {}
            ok = bool(c.get("pass"))
            if freeze:
                fc = next((x for x in freeze["cases"] if x["name"] == cname), None)
                if fc:
                    err = float(c.get("measured_max_abs") or 1e9)
                    ok = ok and err <= float(fc["tolerance"])
                    # digest:device 末帧;允许与 freeze 一致(同机重跑)
                    if fc.get("digest") and c.get("digest"):
                        if c["digest"] != fc["digest"] and err > float(fc["tolerance"]):
                            ok = False
            checks[ckey] = ok
            if not ok:
                check(False, f"{ckey} 未过 measured/tolerance")

        checks["red_wrong_history_identity"] = bool(doc.get("red_wrong_history_identity"))
        checks["red_cross_cut_resurrection"] = bool(doc.get("red_cross_cut_resurrection"))
        checks["red_missing_previous_zero_motion"] = bool(
            doc.get("red_missing_previous_zero_motion")
        )
        checks["not_satisfiable_by_taa"] = bool(doc.get("not_satisfiable_by_taa"))
        checks["validation_zero"] = int(doc.get("validation_errors") or 0) == 0
        if not doc.get("pass"):
            note("device pass=false(见 cases/RED)")
        note(f"device={doc.get('device_name')}")
    elif device_state == "skipped_dev_env":
        checks["validation_zero"] = False

    host_pass = checks["host_oracle_regression"] and checks.get("case_set_exact", False)
    all_pass = all(checks.values()) and not FAILURES
    if device_state == "skipped_dev_env" and require_real():
        all_pass = False
        device_section = "skipped_dev_env"
    elif all_pass and device_state == "executed":
        device_section = "pass"
    else:
        device_section = device_state if device_state else "fail"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g8_m24_tsr_contract",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M24",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_pass,
        "device_section_state": device_section,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "case_set": list(CASE_SET),
        "tolerance_stage": {
            "stage": (freeze or {}).get("stage", "unfrozen")
            if freeze
            else "unfrozen",
            "freeze_path": str(FREEZE_PATH.relative_to(ROOT)).replace("\\", "/"),
            "rfc_budget_gov_pending": freeze_stage != "rfc_budget_frozen",
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"g8_m24_tsr_contract_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    for k in schema.get("required", []):
        check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_section}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        print(f"[{TAG}] GOV_WIRING: tests/tsr_contract/GOV_WIRING.md")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
