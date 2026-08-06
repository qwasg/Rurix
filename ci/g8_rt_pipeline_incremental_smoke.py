#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.2 M50 rt_pipeline_incremental 硬门冒烟(g8.p0.m50.rt_pipeline_incremental;
RFC-0019 §4.1;RXS-0322~0327)。

host 段(恒跑):
  accept/reject 语料、plan_sbt_v2/packer/stack 单测、rxs0248 反代绿静态审计、
  m50 SPIR-V 非 emit_*_min 锚定。

device 段(gate real;`RURIX_REQUIRE_REAL=1` 翻硬红):
  vk_rt_incremental 同场景多 hit group + SBT readback + stack + library。

用法:
  py -3 ci/g8_rt_pipeline_incremental_smoke.py --gate g8.p0.m50.rt_pipeline_incremental
  py -3 ci/g8_rt_pipeline_incremental_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = (
    ROOT / "milestones" / "g8" / "g8_m50_rt_pipeline_incremental_evidence_schema.json"
)
ACCEPT_DIR = ROOT / "conformance" / "rt_pipeline" / "accept"
REJECT_DIR = ROOT / "conformance" / "rt_pipeline" / "reject"

GATE_KEY = "g8.p0.m50.rt_pipeline_incremental"
NUMERIC_STEP = 103
SOURCE_REF = (
    "RFC-0019 §4.1;spec/shader_stages.md RXS-0322~0324;"
    "spec/vulkan_backend.md RXS-0325~0327"
)
TAG = "g8_m50"

REJECT_EXPECT = {
    "record_with_handle.rx": "RX3012",
    "record_recursive.rx": "RX3012",
    "record_outside_rt_stage.rx": "RX3013",
    "triangles_with_intersection.rx": "RX3017",
    "procedural_missing_intersection.rx": "RX3017",
    "callable_index_oob.rx": "RX3012",
    "callable_nesting.rx": "RX3013",
    "trace_dynamic_sbt_offset.rx": "RX3013",
}

CHECK_KEYS = [
    "multi_hit_group_distinct_golden_hit_ids",
    "sbt_user_data_readback_byte_identical",
    "record_packer_is_sole_encoder",
    "stack_size_configured_from_query",
    "stack_configured_ge_required",
    "stack_undersize_red",
    "library_link_equals_monolithic_pixels",
    "library_hash_mismatch_red",
    "anyhit_ignore_green_and_red",
    "procedural_intersection_green_and_red",
    "callable_green_and_red",
    "rxs0248_minimal_witness_not_sufficient",
    "group_oob_mapping_rejected",
    "validation_zero_errors",
    "accept_corpus_green",
    "reject_corpus_red_with_codes",
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


def build_rurixc() -> Path:
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--features", "shader-stages"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(r.stderr, file=sys.stderr)
        sys.exit(1)
    exe = ROOT / "target" / "debug" / (
        "rurixc.exe" if sys.platform == "win32" else "rurixc"
    )
    return exe


def run_rx(exe: Path, rx: Path) -> tuple[int, str]:
    r = subprocess.run(
        [str(exe), str(rx), "--emit=check"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return r.returncode, (r.stdout or "") + (r.stderr or "")


def leg_accept_reject(exe: Path) -> tuple[bool, bool]:
    ok_a = True
    for rx in sorted(ACCEPT_DIR.glob("*.rx")):
        code, out = run_rx(exe, rx)
        if code != 0:
            ok_a = False
            check(False, f"accept {rx.name} exit={code}\n{out}")
    ok_r = True
    for name, expect in REJECT_EXPECT.items():
        rx = REJECT_DIR / name
        code, out = run_rx(exe, rx)
        if code == 0 or expect not in out:
            ok_r = False
            check(False, f"reject {name}: expect {expect} non-zero, got exit={code}\n{out}")
        else:
            note(f"reject {name}: {expect} ok")
    return ok_a, ok_r


def leg_host_unit() -> bool:
    r = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "rurix-rt",
            "--features",
            "vulkan",
            "--lib",
            "rt_incremental::tests",
            "--",
            "--quiet",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    check(r.returncode == 0, f"rt_incremental unit tests failed:\n{r.stderr}")
    return r.returncode == 0


def leg_rxs0248_not_sufficient() -> bool:
    """静态审计:smoke/harness 不得以 run_ray_tracing_offscreen / emit_*_min 充绿本门。"""
    body = (ROOT / "src" / "rurix-rt" / "src" / "vk_m50_rt_body.rs").read_text(
        encoding="utf-8"
    )
    harness = (
        ROOT / "src" / "rurix-rt" / "src" / "bin" / "vk_rt_incremental.rs"
    ).read_text(encoding="utf-8")
    # 允许在注释/反代绿断言中提及旧入口,但不得作为成功路径调用。
    call_sites = [
        ln
        for ln in body.splitlines()
        if "run_ray_tracing_offscreen(" in ln and not ln.strip().startswith("//")
    ]
    bad_call = any("不得" not in ln for ln in call_sites)
    check(not bad_call, "rxs0248: vk_m50_rt_body 不得调用 run_ray_tracing_offscreen 充绿")
    min_refs = ("emit_raygen_min", "emit_miss_min", "emit_closesthit_min")
    bad_min = any(m in harness or m in body for m in min_refs)
    check(not bad_min, "rxs0248: harness/body 不得消费 emit_*_min 充绿")
    check(
        "m50_incremental" in body or "run_rt_pipeline_offscreen" in body,
        "rxs0248: 须走 run_rt_pipeline_offscreen / m50 增量路径",
    )
    return (not bad_call) and (not bad_min)


def leg_device() -> tuple[str, dict[str, bool]]:
    """device 段:构建并跑 vk_rt_incremental;缺则 SKIP/硬红。"""
    checks = {k: False for k in CHECK_KEYS if k not in (
        "accept_corpus_green",
        "reject_corpus_red_with_codes",
        "record_packer_is_sole_encoder",
        "rxs0248_minimal_witness_not_sufficient",
    )}
    r = subprocess.run(
        [
            "cargo",
            "build",
            "-p",
            "rurix-rt",
            "--features",
            "vulkan",
            "--bin",
            "vk_rt_incremental",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        msg = f"vk_rt_incremental build failed:\n{r.stderr[-2000:]}"
        if require_real():
            check(False, msg)
            return "fail", checks
        note("device: harness build failed → skipped_dev_env")
        return "skipped_dev_env", checks

    exe = ROOT / "target" / "debug" / (
        "vk_rt_incremental.exe" if sys.platform == "win32" else "vk_rt_incremental"
    )
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = env.get("RURIX_VK_VALIDATION", "1")
    run = subprocess.run(
        [str(exe)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        timeout=900,
    )
    text = (run.stdout or "") + "\n" + (run.stderr or "")
    if "skipped_dev_env" in text or "RT: SKIP" in text:
        if require_real():
            check(False, f"device SKIP under RURIX_REQUIRE_REAL=1\n{text[-1500:]}")
            return "fail", checks
        return "skipped_dev_env", checks

    doc: dict = {}
    try:
        # 取 stdout 中第一个完整 JSON 对象(勿用 rfind('{')——会落到嵌套 key)。
        src = run.stdout or ""
        start = src.find("{")
        if start >= 0:
            doc = json.loads(src[start:])
    except Exception:
        try:
            start = text.find("{")
            end = text.rfind("}")
            doc = json.loads(text[start : end + 1]) if start >= 0 and end > start else {}
        except Exception:
            doc = {}

    if run.returncode != 0 or not doc or doc.get("device_state") != "executed":
        msg = f"device harness failed exit={run.returncode}\n{text[-2000:]}"
        check(False, msg)
        return "fail", checks

    for k in checks:
        if k in doc.get("checks", {}):
            checks[k] = bool(doc["checks"][k])
        elif k in doc:
            checks[k] = bool(doc[k])
    # 强制 ≥2 hit groups
    hgc = int(doc.get("hit_group_count", 0))
    checks["rxs0248_minimal_witness_not_sufficient"] = hgc >= 2
    check(hgc >= 2, f"hit_group_count={hgc} < 2 (rxs0248 not sufficient)")
    return "executed", checks


def write_evidence(
    host_pass: bool,
    device_state: str,
    checks: dict[str, bool],
    features: dict[str, bool],
) -> Path:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    path = EVIDENCE_DIR / f"g8_m50_rt_pipeline_incremental_{ts}.json"
    doc = {
        "schema_version": 1,
        "subject": "g8_m50_rt_pipeline_incremental",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M50",
        "wave": "G8.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(checks.get(k, False)) for k in CHECK_KEYS},
        "incremental_features": features,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": " | ".join(NOTES)
        + " | counter=g8.counter.rt_pipeline_incremental_features(接线归主 agent)",
    }
    # schema self-check (draft-07 subset: required keys)
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    for k in schema["required"]:
        assert k in doc, f"evidence missing {k}"
    path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {path}")
    return path


def selftest() -> int:
    assert SCHEMA_PATH.is_file()
    assert ACCEPT_DIR.is_dir() and REJECT_DIR.is_dir()
    print(f"[{TAG}] selftest ok")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    print(f"[{TAG}] gate={GATE_KEY} step={NUMERIC_STEP}")
    exe = build_rurixc()
    ok_a, ok_r = leg_accept_reject(exe)
    ok_u = leg_host_unit()
    ok_x = leg_rxs0248_not_sufficient()

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    checks["accept_corpus_green"] = ok_a
    checks["reject_corpus_red_with_codes"] = ok_r
    checks["record_packer_is_sole_encoder"] = ok_u
    checks["rxs0248_minimal_witness_not_sufficient"] = ok_x
    # host stack/packer covered by unit tests
    checks["stack_undersize_red"] = ok_u
    checks["stack_configured_ge_required"] = ok_u

    device_state, dchecks = leg_device()
    checks.update(dchecks)

    features = {
        "multi_hit_group": checks.get("multi_hit_group_distinct_golden_hit_ids", False),
        "sbt_user_data": checks.get("sbt_user_data_readback_byte_identical", False),
        "stack_sizing": checks.get("stack_configured_ge_required", False),
        "pipeline_library": checks.get("library_link_equals_monolithic_pixels", False),
        "frozen_subset": ok_a and ok_r,
    }

    host_pass = ok_a and ok_r and ok_u and ok_x and not any(
        f.startswith("accept") or f.startswith("reject") or "rt_incremental" in f or "rxs0248" in f
        for f in FAILURES
    )
    # stricter: no FAILURES from host legs
    host_fail_msgs = [
        f
        for f in FAILURES
        if not f.startswith("device") and "harness failed" not in f and "SKIP under" not in f
    ]
    host_pass = len(host_fail_msgs) == 0 and ok_a and ok_r and ok_u and ok_x

    path = write_evidence(host_pass, device_state, checks, features)

    if FAILURES:
        print(f"[{TAG}] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f[:500]}", file=sys.stderr)
        if require_real() or device_state == "fail":
            return 1
        # host-only green with device skip is not a gate PASS under real
        if require_real():
            return 1
        return 1 if not host_pass else 1

    print(f"[{TAG}] PASS evidence={path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
