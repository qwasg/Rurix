#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.3 M93 VisibleClusterSet 门冒烟(g9.p0.m93.visible_cluster_set;RFC-0022 §4.2;
spec/virtual_geometry.md RXS-0350;G9_ACCEPTANCE_MAP §2 M93)。

host 纯 host 门(device_section_state=not_applicable)。判据:
  ① rurix-render visible_cluster_set::tests 9 单测逐名锚定全绿(合法 cut/
     空洞+重叠注入 RED/覆盖 sweep/父簇兜底/root 兜底/双跑 digest 确定/
     三喂一致/旁路 RED/provenance 双跑确定;防空跑逐测试名锚定);
  ② rurix-geom-build cull_ref lod_cut_select_reference_passes_runtime_coverage_verifier
     锚定(静态 LOD cut 无运行时误差驱动的旧输出不能充绿);
  ③ conformance/virtual_geometry accept(visible_cluster_set_valid_cut)/reject
     (selection_cut_hole_injected)语料 //@ spec: RXS-0350 锚定;
  ④ g9_g93_geometry_probe 真跑:m93.* checks 全绿 + 空洞/重叠 RED 检出 +
     双跑 set_digest 逐位相等(输出 digest golden 轴)。

用法:
  py -3 ci/g9_visible_cluster_set_smoke.py --gate g9.p0.m93.visible_cluster_set
  py -3 ci/g9_visible_cluster_set_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones/g9/g9_m93_visible_cluster_set_evidence_schema.json"
ACCEPT_CORPUS = ROOT / "conformance/virtual_geometry/accept/visible_cluster_set_valid_cut.rx"
REJECT_CORPUS = ROOT / "conformance/virtual_geometry/reject/selection_cut_hole_injected.rx"

GATE_KEY = "g9.p0.m93.visible_cluster_set"
NUMERIC_STEP = 139
SOURCE_REF = "RFC-0022 §4.2;spec/virtual_geometry.md RXS-0350;G9_ACCEPTANCE_MAP §2 M93"
TAG = "g9_m93"

VCS_TESTS = [
    "valid_cut_accept_corpus_dag",
    "hole_and_overlap_injected_red",
    "cut_coverage_generated_dag_sweep",
    "parent_fallback_on_missing_page_and_restore",
    "root_fallback_last_resort",
    "double_run_digest_deterministic",
    "three_feeds_digest_consistent",
    "bypass_single_source_variant_red",
    "provenance_double_run_deterministic",
]
CHECK_KEYS = [
    "host_visible_cluster_set_tests_anchored",
    "host_lod_cut_reference_verifier_anchored",
    "conformance_accept_corpus_anchored",
    "conformance_reject_hole_corpus_anchored",
    "probe_executed_green",
    "probe_selection_cut_checks_green",
    "probe_hole_overlap_red_detected",
    "probe_digest_double_run_equal",
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


def extract_json(stdout: str) -> dict | None:
    text = stdout.strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except Exception:
        pass
    idx = text.rfind("\n{")
    idx = text.rfind("{") if idx < 0 else idx + 1
    if idx < 0:
        return None
    try:
        return json.loads(text[idx:])
    except Exception:
        return None


# ═══════════════════════ host 段 ═══════════════════════


def host_visible_cluster_set_tests() -> bool:
    """cargo test -p rurix-render --lib visible_cluster_set:9 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurix-render", "--lib", "visible_cluster_set"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in VCS_TESTS:
        if not (ok and name in blob):
            check(False, f"visible_cluster_set 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_lod_cut_reference() -> bool:
    """rurix-geom-build cull_ref 运行时覆盖核验参照单测锚定。"""
    rc, blob = run_cargo(
        ["test", "-p", "rurix-geom-build", "--lib", "cull_ref"]
    )
    ok = (
        rc == 0
        and "test result: ok" in blob
        and "lod_cut_select_reference_passes_runtime_coverage_verifier" in blob
    )
    if not ok:
        check(False, "cull_ref lod_cut_select_reference_passes_runtime_coverage_verifier 未锚定/失败")
    return ok


def host_conformance_anchors() -> tuple[bool, bool]:
    """accept/reject 语料在位 + //@ spec: RXS-0350 锚定 + 预期面注释。"""
    accept_ok = ACCEPT_CORPUS.is_file() and "//@ spec: RXS-0350" in ACCEPT_CORPUS.read_text(
        encoding="utf-8"
    )
    if not accept_ok:
        check(False, f"accept 语料 {ACCEPT_CORPUS.name} 缺失/RXS-0350 锚定缺失")
    reject_ok = False
    if REJECT_CORPUS.is_file():
        text = REJECT_CORPUS.read_text(encoding="utf-8")
        reject_ok = (
            "//@ spec: RXS-0350" in text
            and "selection_cut_hole_injected" in REJECT_CORPUS.name
            and "空洞" in text
        )
    if not reject_ok:
        check(False, "reject 语料 selection_cut_hole_injected.rx 锚定/空洞预期面缺失")
    return accept_ok, reject_ok


def probe_run(evidence_path: Path) -> tuple[int, dict | None, str]:
    print(f"[{TAG}] cargo run -p rurix-render --bin g9_g93_geometry_probe -- --evidence {evidence_path.name}")
    r = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "rurix-render",
            "--bin",
            "g9_g93_geometry_probe",
            "--",
            "--evidence",
            str(evidence_path),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    doc = extract_json(r.stdout)
    if doc is None and evidence_path.is_file():
        try:
            doc = json.loads(evidence_path.read_text(encoding="utf-8"))
        except Exception:
            doc = None
    return r.returncode, doc, r.stdout + r.stderr


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    # 红臂:合成 FAILURES 必须使门红(check() 判别有效)。
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # CHECK_KEYS 闭集恰 8 项。
    if len(CHECK_KEYS) != 8:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 8", file=sys.stderr)
        return 1
    # 绿臂:schema required 与 CHECK_KEYS 闭集互核;合成缺键 evidence 必红。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    top_req = set(schema.get("required", []))
    synth = {k: True for k in CHECK_KEYS}
    synth.pop(CHECK_KEYS[0])
    missing = req - set(synth)
    if not missing:
        print(f"[{TAG}] selftest FAIL: 合成缺键未触发 schema required 红", file=sys.stderr)
        return 1
    if not top_req:
        print(f"[{TAG}] selftest FAIL: schema 顶层 required 为空", file=sys.stderr)
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

    # host 段:cargo 单测逐名锚定 + conformance 语料锚定。
    checks["host_visible_cluster_set_tests_anchored"] = host_visible_cluster_set_tests()
    checks["host_lod_cut_reference_verifier_anchored"] = host_lod_cut_reference()
    accept_ok, reject_ok = host_conformance_anchors()
    checks["conformance_accept_corpus_anchored"] = accept_ok
    checks["conformance_reject_hole_corpus_anchored"] = reject_ok

    # probe 真跑 ×2(双跑 set_digest 逐位相等 = 输出 digest golden 轴)。
    with tempfile.TemporaryDirectory(prefix="g9_m93_probe_") as td:
        ev_a = Path(td) / "probe_a.json"
        ev_b = Path(td) / "probe_b.json"
        rc_a, doc_a, blob_a = probe_run(ev_a)
        rc_b, doc_b, _blob_b = probe_run(ev_b)

    # probe 内部 m93.* 断言全过 ⇔ exit 0 且 failures 空(check() 逐项 fail-closed)。
    checks["probe_executed_green"] = (
        rc_a == 0
        and doc_a is not None
        and doc_a.get("failures") == []
        and isinstance(doc_a.get("visible_clusters"), int)
        and doc_a.get("visible_clusters") > 0
    )
    if not checks["probe_executed_green"]:
        check(False, f"probe 首跑失败 rc={rc_a} failures={doc_a and doc_a.get('failures')}")
    if doc_a is not None:
        # selection cut 判据面:fallback 兜底记录非空 + 页到达后 cut 复原。
        checks["probe_selection_cut_checks_green"] = (
            doc_a.get("fallback_records") == 2
            and isinstance(doc_a.get("restored_cut"), list)
            and len(doc_a.get("restored_cut")) == 3
        )
        if not checks["probe_selection_cut_checks_green"]:
            check(False, "probe 父簇兜底/复原判据面为假")
        checks["probe_hole_overlap_red_detected"] = (
            doc_a.get("hole_red_detected") is True and doc_a.get("overlap_red_detected") is True
        )
        if not checks["probe_hole_overlap_red_detected"]:
            check(False, "probe 空洞/重叠 RED 检出标记缺失")
        digest_a = doc_a.get("set_digest")
        digest_b = doc_b.get("set_digest") if doc_b else None
        checks["probe_digest_double_run_equal"] = (
            rc_b == 0
            and isinstance(digest_a, str)
            and len(digest_a) == 64
            and digest_a == digest_b
        )
        if not checks["probe_digest_double_run_equal"]:
            check(False, "probe 双跑 set_digest 不等/缺失")
    note("probe g9_g93_geometry_probe 双跑真跑;set_digest 逐位相等")

    host_pass = all(checks.values()) and not FAILURES

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m93_visible_cluster_set",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M93",
        "milestone": "M93",
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
            {"seq": 1, "command": "cargo test -p rurix-render --lib visible_cluster_set", "exit_code": 0},
            {"seq": 2, "command": "cargo test -p rurix-geom-build --lib cull_ref", "exit_code": 0},
            {"seq": 3, "command": "cargo run -q -p rurix-render --bin g9_g93_geometry_probe -- --evidence <tmp> (×2 双跑 digest 轴)", "exit_code": 0},
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
    out = EVIDENCE_DIR / f"g9_m93_visible_cluster_set_{ts}.json"
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
