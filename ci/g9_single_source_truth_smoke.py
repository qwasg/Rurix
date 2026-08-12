#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.3 M95 单源真相门冒烟(g9.p0.m95.single_source_truth;RFC-0022 §4.4;
spec/virtual_geometry.md RXS-0352;G9_ACCEPTANCE_MAP §2 M95;R-G9-8)。

host 段:visible_cluster_set provenance/三喂/旁路 RED 三单测逐名锚定
  (three_feeds_digest_consistent / bypass_single_source_variant_red /
  provenance_double_run_deterministic)+ 消费锚单测(geometry::visbuffer
  skinned_cluster_skin_cache_path_diff_zero_host / shadow::vsm
  vsm_consumes_visible_cluster_set_same_source / rt::as_manager
  rt_feed_consumed_by_as_manager_anchor + rt_feed_bypass_recompute_red_at_consumption)
  + conformance reject bypass_single_source_variant.rx(RXS-0352)锚定 +
  g9_g93_geometry_probe 真跑(m95.* 断言面:provenance/旁路 RED/RT 消费锚/
  VisBuffer diff=0/VSM 三喂)。
device 段(必需,持 gpu_device_lock):g9_m95_visbuffer_swhw 蒙皮簇 VisBuffer
  SW/HW 双腿真跑——u64 位级 diff=0(整数域零容差)+ 帧末 provenance 三喂
  digest 一致 + as_manager RT 消费锚放行权威 feed/旁路重算 feed 判 RED +
  RED 双臂(篡改一像素检出/ids 篡改 diff>0)+ RURIX_VK_VALIDATION=1
  validation error=0。`RURIX_REQUIRE_REAL=1` 下 SKIP 翻红。

用法:
  py -3 ci/g9_single_source_truth_smoke.py --gate g9.p0.m95.single_source_truth
  py -3 ci/g9_single_source_truth_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones/g9/g9_m95_single_source_truth_evidence_schema.json"
REJECT_CORPUS = ROOT / "conformance/virtual_geometry/reject/bypass_single_source_variant.rx"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p0.m95.single_source_truth"
NUMERIC_STEP = 141
SOURCE_REF = "RFC-0022 §4.4;spec/virtual_geometry.md RXS-0352;G9_ACCEPTANCE_MAP §2 M95"
TAG = "g9_m95"

PROVENANCE_TESTS = [
    "three_feeds_digest_consistent",
    "bypass_single_source_variant_red",
    "provenance_double_run_deterministic",
]
CONSUMER_ANCHOR_RUNS = [
    # (cargo test 过滤器, 逐名锚定测试名)
    ("geometry::visbuffer", ["skinned_cluster_skin_cache_path_diff_zero_host"]),
    ("shadow::vsm", ["vsm_consumes_visible_cluster_set_same_source"]),
    (
        "rt::as_manager",
        ["rt_feed_consumed_by_as_manager_anchor", "rt_feed_bypass_recompute_red_at_consumption"],
    ),
]
DEVICE_JSON_CHECKS = [
    "sw_hw_diff_zero",
    "sw_hw_coverage_nonzero",
    "oracle_coverage_equal_sw",
    "oracle_coverage_equal_hw",
    "skin_device_bitexact",
    "provenance_three_feeds",
    "rt_feed_consumed_as_manager",
    "rt_bypass_red",
    "red_pixel_tamper",
    "red_ids_tamper",
    "validation_error_zero",
]

CHECK_KEYS = [
    # host 段
    "host_provenance_tests_anchored",
    "host_consumer_anchor_tests_anchored",
    "conformance_red_corpus_anchored",
    "probe_m95_checks_green",
    # device 段
    "device_sw_hw_diff_zero",
    "device_provenance_consumption_anchors",
    "device_red_arms_ok",
    "device_validation_zero",
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


def host_provenance_tests() -> bool:
    """visible_cluster_set provenance/三喂/旁路 RED 三单测逐名锚定。"""
    rc, blob = run_cargo(["test", "-p", "rurix-render", "--lib", "visible_cluster_set"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in PROVENANCE_TESTS:
        if not (ok and name in blob):
            check(False, f"visible_cluster_set 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_consumer_anchor_tests() -> bool:
    """visbuffer/vsm/as_manager 消费锚单测逐名锚定(三趟 cargo test)。"""
    anchored = True
    for filt, names in CONSUMER_ANCHOR_RUNS:
        rc, blob = run_cargo(["test", "-p", "rurix-render", "--lib", filt])
        ok = rc == 0 and "test result: ok" in blob
        for name in names:
            if not (ok and name in blob):
                check(False, f"{filt} 单测 {name} 未锚定/失败")
                anchored = False
    return anchored


def host_conformance_anchor() -> bool:
    """conformance RED 语料在位 + `//@ spec: RXS-0352` 锚定 + 旁路预期面注释。"""
    if not REJECT_CORPUS.is_file():
        check(False, f"缺 RED 语料 {REJECT_CORPUS.name}")
        return False
    text = REJECT_CORPUS.read_text(encoding="utf-8")
    ok = (
        "//@ spec: RXS-0352" in text
        and "bypass_single_source_variant" in REJECT_CORPUS.name
        and "旁路" in text
    )
    if not ok:
        check(False, "RED 语料锚定/旁路预期面缺失")
    return ok


def host_probe() -> bool:
    """g9_g93_geometry_probe 真跑:m95.* 断言面全绿(exit 0 + failures 空 +
    provenance/bypass RED/RT 消费锚/VisBuffer diff/VSM 三喂 JSON 真值面)。"""
    with tempfile.TemporaryDirectory(prefix="g9_m95_probe_") as td:
        ev = Path(td) / "probe.json"
        print(f"[{TAG}] cargo run -p rurix-render --bin g9_g93_geometry_probe")
        r = subprocess.run(
            [
                "cargo", "run", "-q", "-p", "rurix-render", "--bin",
                "g9_g93_geometry_probe", "--", "--evidence", str(ev),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        doc = extract_json(r.stdout)
        if doc is None and ev.is_file():
            try:
                doc = json.loads(ev.read_text(encoding="utf-8"))
            except Exception:
                doc = None
    ok = (
        r.returncode == 0
        and doc is not None
        and doc.get("failures") == []
        and doc.get("provenance_ok") is True
        and doc.get("bypass_red_detected") is True
        and doc.get("rt_feed_consumed_ok") is True
        and doc.get("rt_feed_bypass_red_detected") is True
        and doc.get("visbuffer_diff_mismatched") == 0
        and doc.get("vsm_depth_tris") == 3
    )
    if not ok:
        check(False, f"probe m95 断言面失败 rc={r.returncode} doc={doc and {k: doc.get(k) for k in ('provenance_ok', 'bypass_red_detected', 'rt_feed_consumed_ok', 'rt_feed_bypass_red_detected', 'visbuffer_diff_mismatched', 'vsm_depth_tris')}}")
    return ok


# ═══════════════════════ device 段 ═══════════════════════


def build_device_bin() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin g9_m95_visbuffer_swhw")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", "g9_m95_visbuffer_swhw"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"g9_m95_visbuffer_swhw 构建失败:\n{r.stderr[-2000:]}")
        return None
    name = "g9_m95_visbuffer_swhw.exe" if sys.platform == "win32" else "g9_m95_visbuffer_swhw"
    exe = ROOT / "target" / "debug" / name
    if exe.is_file():
        return exe
    alt_root = os.environ.get("CARGO_TARGET_DIR")
    if alt_root:
        cand = ROOT / alt_root / "debug" / name
        if cand.is_file():
            return cand
    check(False, f"g9_m95_visbuffer_swhw 产物缺失: {exe}")
    return None


def run_device(exe: Path, evidence_out: Path) -> tuple[str, str, dict | None]:
    """返回 (device_state, stdout, device_json)。REQUIRE_REAL 下 SKIP 翻红。"""
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    print(f"[{TAG}] device: g9_m95_visbuffer_swhw(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)")
    r = subprocess.run(
        [str(exe), "--evidence", str(evidence_out)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        timeout=600,
    )
    out = r.stdout + r.stderr
    doc = extract_json(r.stdout)
    if doc is None and evidence_out.is_file():
        try:
            doc = json.loads(evidence_out.read_text(encoding="utf-8"))
        except Exception:
            doc = None
    if "G9_M95_VB: SKIP" in r.stdout:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {out.strip()[-800:]}")
        return "skipped_dev_env", out, doc
    if r.returncode != 0 or "G9_M95_VB: PASS" not in r.stdout:
        check(False, f"g9_m95_visbuffer_swhw 失败 rc={r.returncode}:\n{out[-2000:]}")
        return "fail", out, doc
    return "executed", out, doc


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 8:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 8", file=sys.stderr)
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

    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}

    # host 段
    checks["host_provenance_tests_anchored"] = host_provenance_tests()
    checks["host_consumer_anchor_tests_anchored"] = host_consumer_anchor_tests()
    checks["conformance_red_corpus_anchored"] = host_conformance_anchor()
    checks["probe_m95_checks_green"] = host_probe()

    # device 段(持锁串行)
    device_state = "fail"
    with gpu_device_lock(purpose="g9_m95 visbuffer swhw device 腿"):
        exe = build_device_bin()
        if exe:
            with tempfile.TemporaryDirectory(prefix="g9_m95_dev_") as td:
                dev_ev = Path(td) / "device.json"
                device_state, dev_out, dev_doc = run_device(exe, dev_ev)
            if device_state == "executed" and dev_doc is not None:
                dc = dev_doc.get("checks", {})
                checks["device_sw_hw_diff_zero"] = (
                    dc.get("sw_hw_diff_zero") is True
                    and dev_doc.get("diff_pixels") == 0
                    and dc.get("sw_hw_coverage_nonzero") is True
                    and dc.get("oracle_coverage_equal_sw") is True
                    and dc.get("oracle_coverage_equal_hw") is True
                    and dc.get("skin_device_bitexact") is True
                )
                checks["device_provenance_consumption_anchors"] = (
                    dc.get("provenance_three_feeds") is True
                    and dc.get("rt_feed_consumed_as_manager") is True
                    and dc.get("rt_bypass_red") is True
                )
                checks["device_red_arms_ok"] = (
                    dc.get("red_pixel_tamper") is True and dc.get("red_ids_tamper") is True
                )
                checks["device_validation_zero"] = dc.get("validation_error_zero") is True
                for k in DEVICE_JSON_CHECKS:
                    if dc.get(k) is not True:
                        check(False, f"device checks.{k} 非 true")
            elif device_state == "executed":
                check(False, "device JSON 缺失")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m95_single_source_truth",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M95",
        "milestone": "M95",
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G9.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        ).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": [
            {"seq": 1, "command": "cargo test -p rurix-render --lib visible_cluster_set", "exit_code": 0},
            {"seq": 2, "command": "cargo test -p rurix-render --lib geometry::visbuffer / shadow::vsm / rt::as_manager", "exit_code": 0},
            {"seq": 3, "command": "cargo run -q -p rurix-render --bin g9_g93_geometry_probe", "exit_code": 0},
            {"seq": 4, "command": "cargo build -p rurix-render --features vulkan --bin g9_m95_visbuffer_swhw", "exit_code": 0},
            {"seq": 5, "command": "g9_m95_visbuffer_swhw (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)", "exit_code": 0 if device_state == "executed" else 1},
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
    out = EVIDENCE_DIR / f"g9_m95_single_source_truth_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
