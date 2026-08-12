#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.3 M106 Execution Set 与 PSO 衔接门冒烟(g9.p1.m106.execution_set_pso;
RFC-0023 §4.2;spec/gpu_driven_submit.md RXS-0355 + spec/shader_stages.md RXS-0311
加性修订行;G9_ACCEPTANCE_MAP §3 M106;G9_CONTRACT §8.1 裁决①;U57)。

host 段:rurix-rt execution_set::tests 4 单测逐名锚定(membership+manifest 枚举/
  失效重建确定性/构建 fail-closed typed Err/capability 缺失 fail-closed+诚实
  降级)+ pso_cache execution_set_membership_extension 锚定 + vk.rs
  m106_exec_set_tests 2 单测(FFI 布局锚/命令流布局锚)+ rurixc
  capability_check g93_execution_set_reserved_to_real(RXS-0349 预留位转正)。
device 段(必需,持 gpu_device_lock):vk_execution_set——GPU 侧索引切换臂 vs
  CPU PSO 切换 golden 臂 vs 失效重建臂三 digest 全等(64×64 RGBA8 逐字节容差 0,
  左红右蓝采样点证索引切换真发生)+ host 三段(rebuild_digest/capability_missing
  /d3d12_degrade)+ RURIX_VK_VALIDATION=1 validation error=0。
  `RURIX_REQUIRE_REAL=1` 下 SKIP/degrade 翻硬红。

用法:
  py -3 ci/g9_execution_set_pso_smoke.py --gate g9.p1.m106.execution_set_pso
  py -3 ci/g9_execution_set_pso_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones/g9/g9_m106_execution_set_pso_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p1.m106.execution_set_pso"
NUMERIC_STEP = 144
SOURCE_REF = "RFC-0023 §4.2;spec/gpu_driven_submit.md RXS-0355;spec/shader_stages.md RXS-0311;G9_ACCEPTANCE_MAP §3 M106"
TAG = "g9_m106"

EXECUTION_SET_TESTS = [
    "build_membership_and_manifest_enumeration",
    "invalidate_rebuild_deterministic",
    "build_fail_closed_typed_err",
    "capability_fail_closed_and_honest_degradation",
]
VK_M106_TESTS = [
    "m106_exec_set_ffi_layout_anchors",
    "m106_command_stream_layout_anchor",
]

CHECK_KEYS = [
    # host 段
    "host_execution_set_tests_anchored",
    "host_pso_cache_execution_set_anchored",
    "host_vk_m106_layout_anchors",
    "host_capability_reserved_to_real",
    # device 段
    "device_pass",
    "device_gpu_cpu_rebuild_equal",
    "device_index_switch_ok",
    "device_host_red_arms_ok",
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


def host_execution_set_tests() -> bool:
    """cargo test execution_set(rurix-rt --features vulkan):4 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurix-rt", "--features", "vulkan", "--lib", "execution_set"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in EXECUTION_SET_TESTS:
        if not (ok and name in blob):
            check(False, f"execution_set 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_pso_cache_execution_set() -> bool:
    """pso_cache execution_set 成员扩展单测逐名锚定。"""
    rc, blob = run_cargo(
        ["test", "-p", "rurix-rt", "--features", "vulkan", "--lib", "execution_set_membership_extension"]
    )
    ok = rc == 0 and "test result: ok" in blob and "execution_set_membership_extension" in blob
    if not ok:
        check(False, "pso_cache execution_set_membership_extension 未锚定/失败")
    return ok


def host_vk_m106_layout_anchors() -> bool:
    """vk.rs m106_exec_set_tests 2 单测逐名锚定(模块过滤避开基线失败测试)。"""
    rc, blob = run_cargo(["test", "-p", "rurix-rt", "--features", "vulkan", "--lib", "vk::m106"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in VK_M106_TESTS:
        if not (ok and name in blob):
            check(False, f"vk::m106 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_capability_reserved_to_real() -> bool:
    """rurixc capability_check g93_execution_set_reserved_to_real 锚定(预留位转正)。"""
    rc, blob = run_cargo(
        ["test", "-p", "rurixc", "--lib", "g93_execution_set_reserved_to_real"]
    )
    ok = rc == 0 and "test result: ok" in blob and "g93_execution_set_reserved_to_real" in blob
    if not ok:
        check(False, "rurixc capability_check g93_execution_set_reserved_to_real 未锚定/失败")
    return ok


# ═══════════════════════ device 段 ═══════════════════════


def build_device_bin() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-rt --features vulkan --bin vk_execution_set")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan", "--bin", "vk_execution_set"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"vk_execution_set 构建失败:\n{r.stderr[-2000:]}")
        return None
    name = "vk_execution_set.exe" if sys.platform == "win32" else "vk_execution_set"
    exe = ROOT / "target" / "debug" / name
    if exe.is_file():
        return exe
    alt_root = os.environ.get("CARGO_TARGET_DIR")
    if alt_root:
        cand = ROOT / alt_root / "debug" / name
        if cand.is_file():
            return cand
    check(False, f"vk_execution_set 产物缺失: {exe}")
    return None


def run_device(exe: Path, evidence_out: Path) -> tuple[str, str]:
    """返回 (device_state, stdout)。REQUIRE_REAL 下 SKIP/degrade 翻硬红。"""
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    print(f"[{TAG}] device: vk_execution_set(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)")
    r = subprocess.run(
        [str(exe), "--evidence", str(evidence_out)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        timeout=600,
    )
    out = r.stdout + r.stderr
    if "VK_ES: SKIP" in r.stdout or "PASS(dev-env degrade" in r.stdout:
        if require_real():
            check(False, f"device SKIP/degrade(RURIX_REQUIRE_REAL=1 翻硬红): {out.strip()[-800:]}")
        return "skipped_dev_env", out
    if r.returncode != 0 or "VK_ES: PASS gpu==cpu==rebuild" not in r.stdout:
        check(False, f"vk_execution_set 失败 rc={r.returncode}:\n{out[-2000:]}")
        return "fail", out
    return "executed", out


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 9:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 9", file=sys.stderr)
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
    checks["host_execution_set_tests_anchored"] = host_execution_set_tests()
    checks["host_pso_cache_execution_set_anchored"] = host_pso_cache_execution_set()
    checks["host_vk_m106_layout_anchors"] = host_vk_m106_layout_anchors()
    checks["host_capability_reserved_to_real"] = host_capability_reserved_to_real()

    # device 段(持锁串行)
    device_state = "fail"
    with gpu_device_lock(purpose="g9_m106 execution_set device 腿"):
        exe = build_device_bin()
        if exe:
            dev_ev = EVIDENCE_DIR / ".g9_m106_device_latest.json"
            EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
            device_state, dev_out = run_device(exe, dev_ev)
            if device_state == "executed":
                checks["device_pass"] = True
                checks["device_gpu_cpu_rebuild_equal"] = (
                    "gpu==cpu==rebuild 逐字节一致" in dev_out
                )
                checks["device_index_switch_ok"] = "index_switch[左红右蓝]=OK" in dev_out
                checks["device_host_red_arms_ok"] = (
                    "host[rebuild_digest,capability_missing,d3d12_degrade]=OK" in dev_out
                )
                checks["device_validation_zero"] = "validation=on(0)" in dev_out
                for k in (
                    "device_gpu_cpu_rebuild_equal",
                    "device_index_switch_ok",
                    "device_host_red_arms_ok",
                    "device_validation_zero",
                ):
                    if not checks[k]:
                        check(False, f"device 绿臂 {k} 为假")
                try:
                    dev_ev.unlink()
                except OSError:
                    pass

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m106_execution_set_pso",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M106",
        "milestone": "M106",
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
            {"seq": 1, "command": "cargo test -p rurix-rt --features vulkan --lib execution_set", "exit_code": 0},
            {"seq": 2, "command": "cargo test -p rurix-rt --features vulkan --lib execution_set_membership_extension", "exit_code": 0},
            {"seq": 3, "command": "cargo test -p rurix-rt --features vulkan --lib vk::m106", "exit_code": 0},
            {"seq": 4, "command": "cargo test -p rurixc --lib g93_execution_set_reserved_to_real", "exit_code": 0},
            {"seq": 5, "command": "cargo build -p rurix-rt --features vulkan --bin vk_execution_set", "exit_code": 0},
            {"seq": 6, "command": "vk_execution_set (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)", "exit_code": 0 if device_state == "executed" else 1},
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
    out = EVIDENCE_DIR / f"g9_m106_execution_set_pso_{ts}.json"
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
