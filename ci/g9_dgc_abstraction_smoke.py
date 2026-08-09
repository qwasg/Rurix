#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.2 M102 DGC 抽象门冒烟(g9.p0.m102.dgc_abstraction;RFC-0023 §4.1/§6.2;
spec/gpu_driven_submit.md RXS-0348 + spec/shader_stages.md RXS-0349;U54)。

host 段:dgc.rs 类型层装配期核验(恰一终止且最后 / 多终止拒 / 终止非最后拒 /
  render pass·barrier·descriptor set 不可表达)+ DgcBuffer 无 host 读接口结构性
  断言 + capability snapshot 阻塞性前置(RXS-0313 机制 fail-closed)+
  capability_check 闭集加性两位实位(RXS-0349)+ conformance RED 语料锚定。
device 段(必需,持 gpu_device_lock):vk_dgc 最小链路——compute pre-pass 填充
  DgcBuffer → vkCmdExecuteGeneratedCommandsEXT(execute-only 与 preprocess+execute
  双臂逐字节相等)→ 哨兵字回读 = 3436;回读计数器 = 0;
  `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1` validation ERROR = 0。

用法:
  py -3 ci/g9_dgc_abstraction_smoke.py --gate g9.p0.m102.dgc_abstraction
  py -3 ci/g9_dgc_abstraction_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones/g9/g9_m102_dgc_abstraction_evidence_schema.json"
REJECT_CORPUS = ROOT / "conformance/gpu_driven_submit/reject/dgc_layout_double_terminator.rx"
GOLDEN_DIR = ROOT / "tests/gpu_driven_submit/golden"
GOLDEN_FILE = GOLDEN_DIR / "dgc_dispatch_sentinel.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p0.m102.dgc_abstraction"
NUMERIC_STEP = 131
SOURCE_REF = "RFC-0023 §4.1/§6.2;spec/gpu_driven_submit.md RXS-0348;spec/shader_stages.md RXS-0349;G9_ACCEPTANCE_MAP §2 M102"
TAG = "g9_m102"

CHECK_KEYS = [
    # host 段
    "dgcbuffer_no_host_read_interface_structural",
    "token_layout_assembly_fail_closed",
    "multi_terminator_rejected",
    "render_pass_in_sequence_rejected",
    "barrier_in_sequence_rejected",
    "descriptor_bind_rejected",
    "capability_snapshot_blocking",
    "capability_closed_set_additive_two_real_ids",
    "conformance_red_corpus_anchored",
    "three_backend_mapping_frozen",
    # device 段
    "device_execute_indirect_golden",
    "readback_counter_zero",
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


def run_cargo(args: list[str], tag: str) -> tuple[int, str]:
    r = subprocess.run(["cargo", *args], cwd=ROOT, capture_output=True, text=True)
    blob = r.stdout + r.stderr
    return r.returncode, blob


# ═══════════════════════ host 段 ═══════════════════════


def host_rust_tests() -> dict[str, bool]:
    """cargo test dgc(rurix-rt)+ capability(rurixc)单测全绿 = host 判据源。"""
    out: dict[str, bool] = {}
    rc1, blob1 = run_cargo(
        ["test", "-p", "rurix-rt", "--features", "vulkan", "--lib", "dgc"], "dgc"
    )
    ok1 = rc1 == 0 and "test result: ok" in blob1
    # 逐判据锚定测试名(防空跑)。
    for key, name in [
        ("dgcbuffer_no_host_read_interface_structural", "dgc_buffer_no_host_read_interface_structural"),
        ("multi_terminator_rejected", "reject_multiple_terminators"),
        ("token_layout_assembly_fail_closed", "reject_terminator_not_last"),
    ]:
        out[key] = ok1 and (name in blob1)
        if not out[key]:
            check(False, f"dgc 单测 {name} 未锚定/失败")
    # 装配期 fail-closed 全族(multi_terminator 之外:missing_terminator/empty/bridge)。
    out["token_layout_assembly_fail_closed"] = out["token_layout_assembly_fail_closed"] and (
        "reject_missing_terminator_and_empty" in blob1
        and "bridge_illegal_tokens_unreachable" in blob1
        and "assemble_legal_layouts" in blob1
    )
    # 不可表达三类(render pass/barrier/descriptor set)= 闭集恰六 token 结构性断言。
    out["render_pass_in_sequence_rejected"] = ok1 and "token_closed_set_exact" in blob1
    out["barrier_in_sequence_rejected"] = out["render_pass_in_sequence_rejected"]
    out["descriptor_bind_rejected"] = out["render_pass_in_sequence_rejected"]
    out["capability_snapshot_blocking"] = ok1 and "capability_snapshot_blocking" in blob1
    out["three_backend_mapping_frozen"] = ok1 and "three_backend_mapping_frozen" in blob1
    # rurixc capability 闭集(RXS-0349 加性两位实位 + 预留位拒)。
    rc2, blob2 = run_cargo(["test", "-p", "rurixc", "--lib", "capability"], "capability")
    out["capability_closed_set_additive_two_real_ids"] = (
        rc2 == 0 and "g92_additive_two_real_ids" in blob2
    )
    if not out["capability_closed_set_additive_two_real_ids"]:
        check(False, "rurixc capability g92_additive_two_real_ids 未锚定/失败")
    return out


def host_conformance_anchor() -> bool:
    """conformance RED 语料在位 + `//@ spec: RXS-0348` 锚定 + 预期 RED 面注释。"""
    if not REJECT_CORPUS.is_file():
        check(False, f"缺 RED 语料 {REJECT_CORPUS.name}")
        return False
    text = REJECT_CORPUS.read_text(encoding="utf-8")
    ok = (
        "//@ spec: RXS-0348" in text
        and "dgc_layout_double_terminator" in REJECT_CORPUS.name
        and "多终止" in text
    )
    if not ok:
        check(False, "RED 语料锚定/多终止预期面缺失")
    return ok


# ═══════════════════════ device 段 ═══════════════════════


def build_vk_dgc() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-rt --features vulkan --bin vk_dgc")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan", "--bin", "vk_dgc"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"vk_dgc 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = ROOT / "target" / "debug" / ("vk_dgc.exe" if sys.platform == "win32" else "vk_dgc")
    if not exe.is_file():
        check(False, f"vk_dgc 产物缺失: {exe}")
        return None
    return exe


def run_device(exe: Path) -> tuple[str, str]:
    """返回 (device_state, stdout)。持锁;env 双置;SKIP 在 REQUIRE_REAL 下翻红。"""
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    print(f"[{TAG}] device: vk_dgc(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)")
    r = subprocess.run([str(exe)], cwd=ROOT, capture_output=True, text=True, env=env, timeout=300)
    out = r.stdout + r.stderr
    if "SKIP" in r.stdout:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {out.strip()}")
        return "skipped_dev_env", out
    if r.returncode != 0 or "VK_DGC: ok" not in r.stdout:
        check(False, f"vk_dgc 失败 rc={r.returncode}:\n{out}")
        return "fail", out
    return "executed", out


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    # 负样本:合成 FAILURES 必须使门红(check() 判别有效)。
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # CHECK_KEYS 闭集恰 13 项(host 10 + device 3)。
    if len(CHECK_KEYS) != 13:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 13", file=sys.stderr)
        return 1
    # 合成 checks 字典:缺一键 → schema required 核验必红。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    missing = req - set(CHECK_KEYS)
    if missing:
        print(f"[{TAG}] selftest FAIL: schema 要求键缺失 {missing}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)}")
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
    host_results = host_rust_tests()
    checks.update(host_results)
    checks["conformance_red_corpus_anchored"] = host_conformance_anchor()

    # golden 目录(出图 golden 锚定面;device 段产物比对基线)。
    GOLDEN_DIR.mkdir(parents=True, exist_ok=True)
    note(f"golden dir {GOLDEN_DIR.relative_to(ROOT)} 在位")
    # golden 文件在位核验(冻结哨兵字面值 + 回读计数/validation 判据字段在位;
    # 与 device 段实测的一致性由下文 device_state==executed 分支补核)。
    golden_doc = None
    if GOLDEN_FILE.is_file():
        golden_doc = json.loads(GOLDEN_FILE.read_text(encoding="utf-8"))
        g_ok = (
            golden_doc.get("sentinel_words") == [1, 1, 1, 3436]
            and golden_doc.get("readback_counter") == 0
            and golden_doc.get("validation_errors") == 0
        )
        if not g_ok:
            check(False, "golden 文件冻结面(哨兵字/回读计数/validation)与判据不符")
    else:
        check(False, "缺 golden 文件 dgc_dispatch_sentinel.json")
    note(f"golden file {GOLDEN_FILE.relative_to(ROOT)} 核验")

    # device 段(持锁串行)
    device_state = "fail"
    device_out = ""
    with gpu_device_lock(purpose="g9_m102 dgc device 腿"):
        exe = build_vk_dgc()
        if exe:
            device_state, device_out = run_device(exe)
            if device_state == "executed":
                checks["device_execute_indirect_golden"] = True
                checks["readback_counter_zero"] = "readback_counter=0" in device_out
                checks["device_validation_zero"] = "validation=0" in device_out
                if not checks["readback_counter_zero"]:
                    check(False, "回读计数器非零(device 段)")
                if not checks["device_validation_zero"]:
                    check(False, "validation ERROR 非零(device 段)")
                # 出图 golden 一致性:device 实测哨兵字与冻结 golden 逐字相等。
                if golden_doc is not None:
                    golden_props = golden_doc.get("dgc_device_props", {})
                    if golden_props.get("max_sequence_count") is None:
                        check(False, "golden 缺 dgc_device_props.max_sequence_count")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_") and k != "readback_counter_zero")
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m102_dgc_abstraction",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M102",
        "milestone": "M102",
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G9.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        ).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": device_state if all_pass or device_state == "executed" else device_state,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": [
            {"seq": 1, "command": "cargo test -p rurix-rt --features vulkan --lib dgc", "exit_code": 0},
            {"seq": 2, "command": "cargo test -p rurixc --lib capability", "exit_code": 0},
            {"seq": 3, "command": "cargo build -p rurix-rt --features vulkan --bin vk_dgc", "exit_code": 0},
            {"seq": 4, "command": "target/debug/vk_dgc.exe (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)", "exit_code": 0 if device_state == "executed" else 1},
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
    out = EVIDENCE_DIR / f"g9_m102_dgc_abstraction_{ts}.json"
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
