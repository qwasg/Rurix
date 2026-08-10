#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.2 M90 cluster_dag_deepening 硬门冒烟(g9.p0.m90.cluster_dag_deepening;
RFC-0022 §4.1;spec/virtual_geometry.md RXS-0345;G9_ACCEPTANCE_MAP §2 M90)。

host 纯 host 门(device_section_state=not_applicable)。6 腿判据:

  ① double_build_byte_equal(固定 mesh 语料两次独立构建 canonical 字节相等)
  ② monotonic_edge_check(DAG 每条 parent→child 边误差单调不增逐边机器核验)
  ③ monotonic_break_fixture_rejected(破坏单调性 fixture 构建期 fail-closed typed Err 拒录)
  ④ skin_metadata_roundtrip(蒙皮元数据三字段按冻结 schema 完整 roundtrip,含缺字段 RED)
  ⑤ clas_bake_input_roundtrip(CLAS 离线烘焙输入字段按冻结 schema 完整 roundtrip)
  ⑥ not_substituted_by_m01(仅 G8 M01 静态 DAG 输出为绿不能满足本门)

用法:
  py -3 ci/g9_cluster_dag_deepening_smoke.py --gate g9.p0.m90.cluster_dag_deepening
  py -3 ci/g9_cluster_dag_deepening_smoke.py --selftest
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
GOLDEN_DIR = ROOT / "tests" / "virtual_geometry" / "golden"
REJECT_DIR = ROOT / "conformance" / "virtual_geometry" / "reject"
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_m90_cluster_dag_deepening_evidence_schema.json"

GATE_KEY = "g9.p0.m90.cluster_dag_deepening"
NUMERIC_STEP = 131
SOURCE_REF = "RFC-0022 §4.1;spec/virtual_geometry.md RXS-0345;G9_ACCEPTANCE_MAP §2 M90"
TAG = "g9_m90"

CHECK_KEYS = [
    "double_build_byte_equal",
    "monotonic_edge_check",
    "monotonic_break_fixture_rejected",
    "skin_metadata_roundtrip",
    "clas_bake_input_roundtrip",
    "not_substituted_by_m01",
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


def extract_json(stdout: str) -> dict | None:
    text = stdout.strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except Exception:
        pass
    idx = text.rfind("\n{")
    if idx < 0:
        idx = text.rfind("{")
    else:
        idx += 1
    if idx < 0:
        return None
    try:
        return json.loads(text[idx:])
    except Exception:
        return None


def cargo_test(package: str, extra: list[str] | None = None) -> bool:
    cmd = ["cargo", "test", "-p", package, "--quiet"]
    if extra:
        cmd.extend(extra)
    print(f"[{TAG}] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        check(False, f"cargo test -p {package} 失败:\n{r.stdout}\n{r.stderr}")
        return False
    return True


def run_probe() -> dict | None:
    print(f"[{TAG}] cargo run -p rurix-asset --bin g9_m90_probe")
    r = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "rurix-asset",
            "--bin",
            "g9_m90_probe",
            "--",
            "--golden-dir",
            str(GOLDEN_DIR),
            "--reject-dir",
            str(REJECT_DIR),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"g9_m90_probe 失败 rc={r.returncode}\n{r.stderr}")
    return extract_json(r.stdout)


def verify_fixtures_present() -> None:
    check(
        (GOLDEN_DIR / "m90_dag_digest_manifest.json").is_file(),
        "缺少 tests/virtual_geometry/golden/m90_dag_digest_manifest.json",
    )
    check(
        (REJECT_DIR / "dag_error_nonmonotonic.rx").is_file(),
        "缺少 conformance/virtual_geometry/reject/dag_error_nonmonotonic.rx",
    )


def run_selftest() -> int:
    missing = [k for k in CHECK_KEYS if k not in CHECK_KEYS]
    assert not missing
    assert len(CHECK_KEYS) == 6
    assert SCHEMA_PATH.is_file()
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)}")
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

    verify_fixtures_present()

    geom_build_ok = cargo_test("rurix-geom-build")
    asset_ok = cargo_test("rurix-asset")

    probe = run_probe()
    checks = {k: False for k in CHECK_KEYS}
    if probe:
        probe_checks = probe.get("checks") or {}
        for k in CHECK_KEYS:
            ok = bool(probe_checks.get(k))
            checks[k] = ok
            if not ok:
                check(False, f"probe.{k} 为假")
        if not probe.get("golden_manifest_match"):
            check(False, "golden m90_dag_digest_manifest.json 与双构建 digest 不等")
        if not probe.get("skin_missing_rejected"):
            check(False, "缺蒙皮字段 fixture 未 typed Err 拒录")
        if not probe.get("ok"):
            check(False, "probe ok=false")
    else:
        check(False, "probe JSON 缺失")

    host_pass = (
        geom_build_ok
        and asset_ok
        and len(FAILURES) == 0
        and all(checks.values())
    )

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m90_cluster_dag_deepening",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M90",
        "wave": "G9.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
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
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"g9_m90_cluster_dag_deepening_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    # schema soft validate required keys
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    for k in schema.get("required", []):
        check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if host_pass and not FAILURES:
        print(f"[{TAG}] PASS (host 纯 host 门;6 腿全绿;cargo test geom-build/asset 全绿)")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
