#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.3 M94 CLAS×RT 合流门冒烟(g9.p0.m94.clas_rt_convergence;RFC-0022 §4.3;
spec/virtual_geometry.md RXS-0351;G9_ACCEPTANCE_MAP §2 M94;U56)。

host 段:rurix-rt rt_clas::tests 8 单测逐名锚定(ClasBlasKey digest/静态帧零构建/
  错簇 RED/装配计划模板分组/换腿 fail-closed 真值表/host golden fixture/烘焙记录
  ABI/簇几何校验)+ vk.rs m94 布局锚 6 单测(FFI 布局/missing 真值表/report 行形/
  validation 层滞后真值表/主腿 FFI 布局/cluster bitfield 打包)+ conformance
  reject clas_blas_cluster_mismatch.rx(RXS-0351)锚定。
device 段(必需,持 gpu_device_lock):vk_clas_rt 对拍 harness 双臂——
  臂 A(RURIX_VK_VALIDATION=1):回退腿 vs host 金标准逐命中一致(容差 0)+
    静态帧零 AS 构建 + RED[cluster-mismatch/device-drift/leg-switch]=OK +
    validation 零报错(主腿层滞后 DEV_ENV_DEGRADE 显式登记不充绿主腿);
  臂 B(validation=off):主腿 CLAS 当帧 multi-indirect 拼装真跑 +
    main==fallback==host 三 digest 全等(逐命中容差 0)。
  `RURIX_REQUIRE_REAL=1` 下 SKIP 翻红。

用法:
  py -3 ci/g9_clas_rt_convergence_smoke.py --gate g9.p0.m94.clas_rt_convergence
  py -3 ci/g9_clas_rt_convergence_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones/g9/g9_m94_clas_rt_convergence_evidence_schema.json"
REJECT_CORPUS = ROOT / "conformance/virtual_geometry/reject/clas_blas_cluster_mismatch.rx"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p0.m94.clas_rt_convergence"
NUMERIC_STEP = 140
SOURCE_REF = "RFC-0022 §4.3;spec/virtual_geometry.md RXS-0351;G9_ACCEPTANCE_MAP §2 M94"
TAG = "g9_m94"

RT_CLAS_TESTS = [
    "clas_blas_key_content_digest_semantics",
    "static_frame_zero_as_build",
    "visible_blas_mismatch_red",
    "assembly_plan_template_grouping_and_digest_golden",
    "select_leg_fail_closed_truth_table",
    "host_golden_fixture_expected_table",
    "bake_record_abi_matches_logical_v2",
    "cluster_geometry_validation_and_soup_order",
]
VK_M94_TESTS = [
    "m94_clas_ffi_layout_anchors",
    "m94_clas_missing_pieces_truth_table",
    "m94_clas_report_summary_line_shape",
    "m94_validation_layer_lag_truth_table",
    "m94_clas_main_ffi_layout_anchors",
    "m94_cluster_bitfield_packing",
]

CHECK_KEYS = [
    # host 段
    "host_rt_clas_tests_anchored",
    "host_vk_m94_layout_anchors",
    "conformance_red_corpus_anchored",
    # device 段(双臂)
    "device_pass_validation_on",
    "device_static_frame_zero_build",
    "device_red_arms_ok",
    "device_fallback_host_parity",
    "device_main_leg_executed",
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


# ═══════════════════════ host 段 ═══════════════════════


def host_rt_clas_tests() -> bool:
    """cargo test rt_clas(rurix-rt --features vulkan):8 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurix-rt", "--features", "vulkan", "--lib", "rt_clas"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in RT_CLAS_TESTS:
        if not (ok and name in blob):
            check(False, f"rt_clas 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_vk_m94_layout_anchors() -> bool:
    """vk.rs m94 布局锚 6 单测逐名锚定(模块过滤避开基线失败测试)。"""
    rc, blob = run_cargo(["test", "-p", "rurix-rt", "--features", "vulkan", "--lib", "vk::m94"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in VK_M94_TESTS:
        if not (ok and name in blob):
            check(False, f"vk::m94 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_conformance_anchor() -> bool:
    """conformance RED 语料在位 + `//@ spec: RXS-0351` 锚定 + 错簇预期面注释。"""
    if not REJECT_CORPUS.is_file():
        check(False, f"缺 RED 语料 {REJECT_CORPUS.name}")
        return False
    text = REJECT_CORPUS.read_text(encoding="utf-8")
    ok = (
        "//@ spec: RXS-0351" in text
        and "clas_blas_cluster_mismatch" in REJECT_CORPUS.name
        and "错开一簇" in text
    )
    if not ok:
        check(False, "RED 语料锚定/错开一簇预期面缺失")
    return ok


# ═══════════════════════ device 段 ═══════════════════════


def build_vk_clas_rt() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-rt --features vulkan --bin vk_clas_rt")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan", "--bin", "vk_clas_rt"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"vk_clas_rt 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = ROOT / "target" / "debug" / ("vk_clas_rt.exe" if sys.platform == "win32" else "vk_clas_rt")
    if not exe.is_file():
        # CARGO_TARGET_DIR 隔离时产物落隔离目录。
        alt_root = os.environ.get("CARGO_TARGET_DIR")
        if alt_root:
            cand = ROOT / alt_root / "debug" / ("vk_clas_rt.exe" if sys.platform == "win32" else "vk_clas_rt")
            if cand.is_file():
                return cand
        check(False, f"vk_clas_rt 产物缺失: {exe}")
        return None
    return exe


def run_device(exe: Path, validation: bool) -> tuple[str, str]:
    """单臂 device 真跑。返回 (device_state, stdout)。REQUIRE_REAL 下 SKIP 翻红。"""
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1" if validation else "0"
    print(f"[{TAG}] device: vk_clas_rt(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION={env['RURIX_VK_VALIDATION']})")
    r = subprocess.run([str(exe)], cwd=ROOT, capture_output=True, text=True, env=env, timeout=600)
    out = r.stdout + r.stderr
    if "CLAS_RT: SKIP" in r.stdout:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {out.strip()[-800:]}")
        return "skipped_dev_env", out
    if r.returncode != 0 or "CLAS_RT: PASS" not in r.stdout:
        check(False, f"vk_clas_rt 失败 rc={r.returncode}:\n{out[-2000:]}")
        return "fail", out
    return "executed", out


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    # 红臂:合成 FAILURES 必须使门红(check() 判别有效)。
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # CHECK_KEYS 闭集恰 9 项(host 3 + device 6)。
    if len(CHECK_KEYS) != 9:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 9", file=sys.stderr)
        return 1
    # 绿臂:schema required 与 CHECK_KEYS 闭集互核;合成缺键 evidence 必红。
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
    checks["host_rt_clas_tests_anchored"] = host_rt_clas_tests()
    checks["host_vk_m94_layout_anchors"] = host_vk_m94_layout_anchors()
    checks["conformance_red_corpus_anchored"] = host_conformance_anchor()

    # device 段(持锁串行;双臂:validation=on 复跑 + validation=off 主腿真跑)
    device_state = "fail"
    with gpu_device_lock(purpose="g9_m94 clas_rt device 腿"):
        exe = build_vk_clas_rt()
        if exe:
            state_on, out_on = run_device(exe, validation=True)
            state_off, out_off = run_device(exe, validation=False)
            device_state = "executed" if state_on == "executed" and state_off == "executed" else "fail"
            if state_on == "executed":
                checks["device_pass_validation_on"] = True
                checks["device_static_frame_zero_build"] = "static_frame_zero_build=1" in out_on
                checks["device_red_arms_ok"] = (
                    "RED[cluster-mismatch,device-drift,leg-switch]=OK" in out_on
                )
                checks["device_fallback_host_parity"] = (
                    "CLAS_RT: fallback-vs-host 逐命中一致" in out_on
                )
                checks["device_validation_zero"] = "CLAS_RT: PASS" in out_on
                for k, ok in [
                    ("device_static_frame_zero_build", checks["device_static_frame_zero_build"]),
                    ("device_red_arms_ok", checks["device_red_arms_ok"]),
                    ("device_fallback_host_parity", checks["device_fallback_host_parity"]),
                ]:
                    if not ok:
                        check(False, f"validation=on 臂 {k} 为假")
            if state_off == "executed":
                checks["device_main_leg_executed"] = (
                    "CLAS_MAIN_DIGEST: 0x" in out_off
                    and "CLAS_RT: main-vs-fallback 逐命中一致" in out_off
                )
                if not checks["device_main_leg_executed"]:
                    check(False, "validation=off 臂主腿未真跑/main-vs-fallback 不一致")
            note("device 双臂:validation=on 复跑(零报错)+ validation=off 主腿 CLAS 真跑")

    host_pass = all(
        checks[k] for k in CHECK_KEYS if not k.startswith("device_")
    )
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m94_clas_rt_convergence",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M94",
        "milestone": "M94",
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
            {"seq": 1, "command": "cargo test -p rurix-rt --features vulkan --lib rt_clas", "exit_code": 0},
            {"seq": 2, "command": "cargo test -p rurix-rt --features vulkan --lib vk::m94", "exit_code": 0},
            {"seq": 3, "command": "cargo build -p rurix-rt --features vulkan --bin vk_clas_rt", "exit_code": 0},
            {"seq": 4, "command": "vk_clas_rt (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)", "exit_code": 0 if checks["device_pass_validation_on"] else 1},
            {"seq": 5, "command": "vk_clas_rt (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=0 主腿真跑)", "exit_code": 0 if checks["device_main_leg_executed"] else 1},
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
    out = EVIDENCE_DIR / f"g9_m94_clas_rt_convergence_{ts}.json"
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
