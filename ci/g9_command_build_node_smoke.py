#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.3 M105 command build node 门冒烟(g9.p1.m105.command_build_node;RFC-0023 §4.4;
spec/gpu_driven_submit.md RXS-0354;G9_ACCEPTANCE_MAP §3 M105;G9_CONTRACT §8.1 裁决①;
复用 U54 DGC lane 零新 FFI)。

host 段:rurix-rt command_build::tests 7 单测逐名锚定(indirect 边挂载+barrier
  推导 GREEN/消费声明破坏 RED/host 参照内容流 golden/参照构建确定性+fail-closed/
  零回读全链 GREEN+注入 RED/核验回读记账协议/conformance 锚消费)+
  conformance/gpu_driven_submit/reject/command_build_host_readback.rx(RXS-0354)锚定。
device 段(必需,持 gpu_device_lock):vk_command_build 四腿(dispatch 双构建/draw/
  draw_indexed)device 构建产物与 host build_reference 逐字节一致 + indirect pass
  消费哨兵 0x0D6D 命中 + 生产路径零隐式回读(readback_counter=0)+
  `--red-inject-readback` RED 臂必退 1(注入未记账回读计数面非零必检出)+
  RURIX_VK_VALIDATION=1 validation error=0。`RURIX_REQUIRE_REAL=1` 下 SKIP 翻红。

用法:
  py -3 ci/g9_command_build_node_smoke.py --gate g9.p1.m105.command_build_node
  py -3 ci/g9_command_build_node_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones/g9/g9_m105_command_build_node_evidence_schema.json"
REJECT_CORPUS = ROOT / "conformance/gpu_driven_submit/reject/command_build_host_readback.rx"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p1.m105.command_build_node"
NUMERIC_STEP = 143
SOURCE_REF = "RFC-0023 §4.4;spec/gpu_driven_submit.md RXS-0354;G9_ACCEPTANCE_MAP §3 M105"
TAG = "g9_m105"

COMMAND_BUILD_TESTS = [
    "mounts_indirect_edge_and_derives_barrier_green",
    "broken_consumer_declaration_red",
    "reference_content_stream_golden",
    "reference_build_deterministic_and_fail_closed",
    "zero_readback_full_chain_green_and_injected_red",
    "verification_readback_accounting_protocol",
    "conformance_anchor_consumed",
]

CHECK_KEYS = [
    # host 段
    "host_command_build_tests_anchored",
    "conformance_red_corpus_anchored",
    # device 段
    "device_pass",
    "device_byte_exact_legs",
    "device_double_build_digest_equal",
    "device_indirect_consumed_sentinel",
    "device_production_readback_zero",
    "device_red_inject_readback_ok",
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


def host_command_build_tests() -> bool:
    """cargo test command_build(rurix-rt --features vulkan):7 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurix-rt", "--features", "vulkan", "--lib", "command_build"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in COMMAND_BUILD_TESTS:
        if not (ok and name in blob):
            check(False, f"command_build 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_conformance_anchor() -> bool:
    """conformance RED 语料在位 + `//@ spec: RXS-0354` 锚定 + 零回读预期面注释。"""
    if not REJECT_CORPUS.is_file():
        check(False, f"缺 RED 语料 {REJECT_CORPUS.name}")
        return False
    text = REJECT_CORPUS.read_text(encoding="utf-8")
    ok = (
        "//@ spec: RXS-0354" in text
        and "command_build_host_readback" in REJECT_CORPUS.name
        and "回读" in text
    )
    if not ok:
        check(False, "RED 语料锚定/零回读预期面缺失")
    return ok


# ═══════════════════════ device 段 ═══════════════════════


def build_device_bin() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-rt --features vulkan --bin vk_command_build")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan", "--bin", "vk_command_build"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"vk_command_build 构建失败:\n{r.stderr[-2000:]}")
        return None
    name = "vk_command_build.exe" if sys.platform == "win32" else "vk_command_build"
    exe = ROOT / "target" / "debug" / name
    if exe.is_file():
        return exe
    alt_root = os.environ.get("CARGO_TARGET_DIR")
    if alt_root:
        cand = ROOT / alt_root / "debug" / name
        if cand.is_file():
            return cand
    check(False, f"vk_command_build 产物缺失: {exe}")
    return None


def run_device(exe: Path, evidence_out: Path) -> tuple[str, str]:
    """绿臂 device 真跑。返回 (device_state, stdout)。REQUIRE_REAL 下 SKIP 翻红。"""
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    print(f"[{TAG}] device: vk_command_build(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)")
    r = subprocess.run(
        [str(exe), "--evidence", str(evidence_out)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        timeout=600,
    )
    out = r.stdout + r.stderr
    if "VK_CB: SKIP" in r.stdout:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {out.strip()[-800:]}")
        return "skipped_dev_env", out
    if r.returncode != 0 or "VK_CB: PASS" not in r.stdout:
        check(False, f"vk_command_build 失败 rc={r.returncode}:\n{out[-2000:]}")
        return "fail", out
    return "executed", out


def run_red_arm(exe: Path) -> bool:
    """RED 臂独立见证:--red-inject-readback 子进程必须退 1 且诊断点名
    「零 CPU 回读违例 + RED 注入生效」(注入未记账回读必检出翻红;与绿臂主进程
    内部 spawn_leg 编排同臂,门脚本独立复跑证 RED 机制非主进程自说自话)。"""
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    print(f"[{TAG}] device RED 臂: vk_command_build --red-inject-readback(期望退 1)")
    r = subprocess.run(
        [str(exe), "--red-inject-readback"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        timeout=600,
    )
    out = r.stdout + r.stderr
    ok = (
        r.returncode == 1
        and "零 CPU 回读违例" in out
        and "RED 注入生效" in out
    )
    if not ok:
        check(False, f"RED 臂未按预期翻红 rc={r.returncode}:\n{out[-1200:]}")
    return ok


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
    checks["host_command_build_tests_anchored"] = host_command_build_tests()
    checks["conformance_red_corpus_anchored"] = host_conformance_anchor()

    # device 段(持锁串行;绿臂 + RED 注入臂)
    device_state = "fail"
    with gpu_device_lock(purpose="g9_m105 command_build device 腿"):
        exe = build_device_bin()
        if exe:
            dev_ev = EVIDENCE_DIR / ".g9_m105_device_latest.json"
            EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
            device_state, dev_out = run_device(exe, dev_ev)
            if device_state == "executed":
                checks["device_pass"] = True
                checks["device_byte_exact_legs"] = (
                    "byte_exact[dispatch,draw,draw_indexed]" in dev_out
                )
                checks["device_double_build_digest_equal"] = (
                    "double_build_digest_equal" in dev_out
                )
                checks["device_indirect_consumed_sentinel"] = (
                    "indirect_consumed(sentinel=0x0D6D)" in dev_out
                )
                checks["device_production_readback_zero"] = "production_readback=0" in dev_out
                checks["device_validation_zero"] = "validation=on(0)" in dev_out
                # 绿臂主进程内部已编排 RED 子进程(⑤ spawn_leg --red-inject-readback
                # 期待 rc==1 + 诊断点名),PASS 行 RED[injected-readback]=OK 为机器核验面;
                # 门脚本再独立复跑 RED 臂子进程(退 1 + 诊断点名)作独立见证。
                green_red_armed = "RED[injected-readback]=OK" in dev_out
                checks["device_red_inject_readback_ok"] = green_red_armed and run_red_arm(exe)
                for k in (
                    "device_byte_exact_legs",
                    "device_double_build_digest_equal",
                    "device_indirect_consumed_sentinel",
                    "device_production_readback_zero",
                    "device_validation_zero",
                ):
                    if not checks[k]:
                        check(False, f"device 绿臂 {k} 为假")
                if not green_red_armed:
                    check(False, "device 绿臂 RED[injected-readback]=OK 锚定缺失")
                try:
                    dev_ev.unlink()
                except OSError:
                    pass

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m105_command_build_node",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M105",
        "milestone": "M105",
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
            {"seq": 1, "command": "cargo test -p rurix-rt --features vulkan --lib command_build", "exit_code": 0},
            {"seq": 2, "command": "cargo build -p rurix-rt --features vulkan --bin vk_command_build", "exit_code": 0},
            {"seq": 3, "command": "vk_command_build (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)", "exit_code": 0 if device_state == "executed" else 1},
            {"seq": 4, "command": "vk_command_build --red-inject-readback (RED 臂,期望退 1)", "exit_code": 1 if checks["device_red_inject_readback_ok"] else 0},
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
    out = EVIDENCE_DIR / f"g9_m105_command_build_node_{ts}.json"
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
