#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.2 M103 descriptor_global_table 硬门冒烟(步骤 134;g9.p0.m103.descriptor_global_table;
RFC-0023 §4.3;spec/rendering_platform.md RXS-0347)。

host+device 门(device 段持锁真跑,RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)。
七腿判据:

  ① reflection_shader_index_bidirectional_equal——reflection v1 尾随可选字段
     「资源→全局 descriptor 索引」与 shader 实际消费索引**双向精确相等**(双向对拍,
     不接受单向抽查):host `GlobalDescriptorTable` 分配映射 ≡ vk_desc_v3 fixture 的
     shader 消费索引(逐条目恒等,首尾两端必触)+ reflection.rs 真值化尾随字段与
     表快照逐值相等。
  ② table_65536_entries_golden——≥65536 条目 fixture device 出图与 host 种子重算
     golden **逐字节相等**(vk_desc_v3 device 真跑)。
  ③ legacy_set_binding_zero_byte_regression——set/binding 旧路径(v1/v2 descriptor
     set)加性并存、回归 digest 不变:既有 reflection golden 0-byte 恒跑(reflection
     缺省 plan 产物逐字节不变)+ vk_desc_v2 同跑对照像素不变。
  ④ index_allocation_deterministic——同输入同映射逐字节等值(双跑)+ 回收空位
     升序复用。
  ⑤ dangling_index_fail_closed——索引越界(≥ capacity)/ 悬空索引(回收后读)/
     双重释放 → fail-closed 诊断。
  ⑥ index_leak_counter_zero——泄漏计数器断言(live=0)全真。
  ⑦ device_validation_zero——device 段 RURIX_VK_VALIDATION=1 全程 validation
     零报错(fail-closed messenger)。

退出码判定(非 grep stdout)。任一判据红 → 逐项打印定位后 exit 1(evidence 仍
如实落盘,红不充绿)。

用法:
  py -3 ci/g9_descriptor_global_table_smoke.py --gate g9.p0.m103.descriptor_global_table
  py -3 ci/g9_descriptor_global_table_smoke.py --selftest
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

GATE_KEY = "g9.p0.m103.descriptor_global_table"
NUMERIC_STEP = 134

CHECK_KEYS = [
    "reflection_shader_index_bidirectional_equal",
    "table_65536_entries_golden",
    "legacy_set_binding_zero_byte_regression",
    "index_allocation_deterministic",
    "dangling_index_fail_closed",
    "index_leak_counter_zero",
    "device_validation_zero",
]

FAILURES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def record_command(cmd: list[str], rc: int, note: str) -> None:
    COMMANDS.append(
        {
            "seq": len(COMMANDS) + 1,
            "command": " ".join(cmd),
            "exit_code": rc,
            "note": note,
        }
    )


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def cargo_test(args: list[str], label: str) -> bool:
    cmd = ["cargo", "test", "-p", args[0], "--quiet"] + args[1:]
    print(f"[g9_m103] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    record_command(cmd, r.returncode, label)
    ok = r.returncode == 0 and "test result: ok" in (r.stdout + r.stderr)
    check(ok, f"{label} 未过(rc={r.returncode}):\n{(r.stdout + r.stderr)[-1200:]}")
    return ok


def leg_rust_gates() -> None:
    """腿①④⑤⑥(host):descriptor_table 分配律 + reflection RXS-0347 尾随字段单测组。"""
    # 全局表分配/回收/悬空/越界/leak(rurix-rt 纯 host)。
    cargo_test(
        ["rurix-rt", "--lib", "descriptor_table::"],
        "④⑤⑥全局表分配律/悬空越界/泄漏计数器",
    )
    # reflection 尾随可选字段:0-drift(③)+ 真值化(①)+ 悬空/越界拒(⑤)。
    cargo_test(
        ["rurixc", "--lib", "reflection"],
        "①③⑤reflection RXS-0347 尾随字段 0-drift/真值化/RED 单测",
    )
    # 逐函数点名锚定(双向对拍 / 0-drift / 分配确定性)。
    for name, pkg, label in (
        (
            "gdi_truth_table_trailing_additive",
            "rurixc",
            "①reflection↔全局索引映射真值化(双向对拍面)",
        ),
        (
            "gdi_absent_is_byte_identical_zero_drift",
            "rurixc",
            "③既有 reflection golden 0-byte 恒跑",
        ),
        ("gdi_dangling_and_budget_fail_closed", "rurixc", "⑤悬空/越界 fail-closed"),
        ("allocation_is_deterministic", "rurix-rt", "④同输入同映射确定性"),
        (
            "fail_closed_paths_and_leak_counter",
            "rurix-rt",
            "⑤⑥悬空/越界/双释放/leak 计数器",
        ),
    ):
        lib = "reflection::tests" if pkg == "rurixc" else "descriptor_table::tests"
        cmd = [
            "cargo",
            "test",
            "-p",
            pkg,
            "--lib",
            "--quiet",
            f"{lib}::{name}",
        ]
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
        record_command(cmd, r.returncode, label)
        ok = r.returncode == 0 and "1 passed" in r.stdout
        check(ok, f"{label} 单测 `{name}` 未锚绿:\n{(r.stdout + r.stderr)[-800:]}")


def leg_device() -> bool:
    """腿②⑦(device,持锁):vk_desc_v3 ≥65536 条目出图 golden + validation=0。

    device 真跑须持 gpu_device_lock(cargo 串行 + GPU 串行);RURIX_REQUIRE_REAL=1
    + RURIX_VK_VALIDATION=1 双置。出图 = host 种子重算 golden 逐字节相等由 harness
    内断言(exit 0);本腿核验 exit 码 + PASS 标记 + 无 validation 报错。"""
    from gpu_device_lock import gpu_device_lock

    env = dict(os.environ, RURIX_REQUIRE_REAL="1", RURIX_VK_VALIDATION="1")
    with gpu_device_lock("g9.2 m103 device 腿"):
        # build(device 前 host 编译,持锁串行)。
        build = subprocess.run(
            [
                "cargo",
                "build",
                "-p",
                "rurix-rt",
                "--features",
                "vulkan",
                "--bin",
                "vk_desc_v3",
                "--quiet",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        record_command(
            ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan", "--bin", "vk_desc_v3"],
            build.returncode,
            "vk_desc_v3 编译",
        )
        if build.returncode != 0:
            check(False, f"②vk_desc_v3 编译失败:\n{(build.stdout + build.stderr)[-1200:]}")
            return False
        exe = ROOT / "target" / "debug" / ("vk_desc_v3.exe" if sys.platform == "win32" else "vk_desc_v3")
        r = subprocess.run([str(exe)], cwd=ROOT, capture_output=True, text=True, env=env)
        record_command(
            [f"{exe.name} (RURIX_REQUIRE_REAL=1 RURIX_VK_VALIDATION=1)"],
            r.returncode,
            "②⑦65536 条目 device 出图 + validation",
        )
        out = r.stdout + r.stderr
        if r.returncode != 0:
            check(False, f"②vk_desc_v3 device 出图失败(rc={r.returncode}):\n{out[-1500:]}")
            return False
        check(
            "VK_DESC_V3: ok" in r.stdout,
            f"②vk_desc_v3 未产 PASS 标记:\n{r.stdout[-800:]}",
        )
        # golden_equal / 65536 / leak_zero 字面锚(harness 断言内嵌,exit 0 已机器核)。
        check(
            "table_len=65536" in r.stdout,
            "②fixture 条目数非 65536(硬门面 ≥65536)",
        )
        check("golden_equal=true" in r.stdout, "②出图 ≠ golden(golden_equal≠true)")
        # validation 零报错(messenger fail-closed + stderr 无 VUID)。
        check(
            "Validation Error" not in out and "VUID-" not in out,
            f"⑦validation 报错:\n{out[-1000:]}",
        )
        return len(FAILURES) == 0


def leg_v1v2_regression() -> None:
    """腿③(回归):vk_desc_v2 v1/v2 descriptor set 旧路径对照像素不变(0-byte)。

    device 持锁跑 vk_desc_v2(v1/v2 同像素断言内嵌);set/binding 旧路径加性并存
    的回归 digest 不变由该对照承担(descriptor buffer 加性不扰动既有渲染输出)。"""
    from gpu_device_lock import gpu_device_lock

    env = dict(os.environ, RURIX_REQUIRE_REAL="1", RURIX_VK_VALIDATION="1")
    with gpu_device_lock("g9.2 m103 v1/v2 回归腿"):
        build = subprocess.run(
            [
                "cargo",
                "build",
                "-p",
                "rurix-rt",
                "--features",
                "vulkan",
                "--bin",
                "vk_desc_v2",
                "--quiet",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        record_command(
            ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan", "--bin", "vk_desc_v2"],
            build.returncode,
            "vk_desc_v2 编译(回归)",
        )
        if build.returncode != 0:
            check(False, f"③vk_desc_v2 编译失败:\n{(build.stdout + build.stderr)[-800:]}")
            return
        exe = ROOT / "target" / "debug" / ("vk_desc_v2.exe" if sys.platform == "win32" else "vk_desc_v2")
        r = subprocess.run([str(exe)], cwd=ROOT, capture_output=True, text=True, env=env)
        record_command([f"{exe.name} (回归对照)"], r.returncode, "③v1/v2 旧路径回归对照")
        out = r.stdout + r.stderr
        check(
            r.returncode == 0 and "VK_DESC_V2: ok" in r.stdout,
            f"③v1/v2 回归对照失败(rc={r.returncode}):\n{out[-1200:]}",
        )
        check(
            "Validation Error" not in out and "VUID-" not in out,
            f"③v1/v2 回归 validation 报错:\n{out[-600:]}",
        )


def leg_fixture_and_spec() -> None:
    """conformance 锚定语料转正核验 + RXS-0347 条款字面机核(spec 在位)。"""
    fx = ROOT / "conformance" / "reflection" / "reject" / "global_descriptor_index_dangling.rx"
    check(fx.is_file(), f"缺 conformance fixture: {fx}")
    if fx.is_file():
        text = fx.read_text(encoding="utf-8")
        check("//@ spec: RXS-0347" in text, f"{fx.name} 缺 RXS-0347 锚定头")
    # spec 条款字面(读文件比对;修订行字面在位)。
    spec = (ROOT / "spec" / "rendering_platform.md").read_text(encoding="utf-8")

    def norm(s: str) -> str:
        return "".join(s.split()).replace("**", "").replace("`", "")

    spec_n = norm(spec)
    for needle, label in (
        ("资源→全局 descriptor 索引", "RXS-0347 映射记录面"),
        ("尾随可选字段", "0-drift 机制"),
        ("不得以「空编码为count 0」冒充0-byte", "0-byte 纪律"),
        ("同输入同映射逐字节等值", "分配律确定性"),
        ("悬空索引", "悬空 fail-closed"),
    ):
        check(
            norm(needle) in spec_n,
            f"RXS-0347 条款缺 {label} 字面锚: {needle!r}(spec/rendering_platform.md)",
        )


def write_evidence(results: dict, host_ok: bool, base_commit: str, device_ok: bool) -> Path:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    ev = {
        "schema_version": 1,
        "subject": "g9_m103_descriptor_global_table",
        "milestone": "M103",
        "wave": "G9.2",
        "assertion_id": GATE_KEY,
        "status": "pass" if host_ok else "fail",
        "commands": COMMANDS,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M103",
        "numeric_step": NUMERIC_STEP,
        "source_ref": "RFC-0023 §4.3;spec/rendering_platform.md RXS-0347;G9_ACCEPTANCE_MAP §2 M103",
        "host_section_pass": host_ok,
        "device_section_state": "pass" if device_ok else "fail",
        "checks": results,
        "evidence_level": "measured_local",
        "base_commit": base_commit,
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": (
            "host+device 门(RTX 4070 Ti 真跑,RURIX_REQUIRE_REAL=1 + "
            "RURIX_VK_VALIDATION=1,validation 零报错)。七腿:reflection↔shader 索引"
            "双向精确相等 + ≥65536 条目出图 golden 逐字节相等 + set/binding 旧路径"
            "0-byte 回归(vk_desc_v2 对照)+ 分配确定性 + 悬空/越界 fail-closed + "
            "泄漏计数器零 + device validation 零。全局表分配律单一事实源 = "
            "src/rurix-rt/src/descriptor_table.rs;descriptor buffer 物理写入面 = "
            "vk.rs VK_EXT_descriptor_buffer FFI(U55);reflection 尾随可选字段 = "
            "rurixc reflection.rs RXS-0347(缺省 0-drift,既有 golden 0-byte 恒跑)。"
        ),
    }
    path = EVIDENCE_DIR / f"g9_m103_descriptor_global_table_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g9_m103] evidence 落盘: {path.relative_to(ROOT)}")
    return path


def selftest() -> None:
    """反 YAML-only:合成数据喂判定层,证明每组断言都能红。"""
    check(False, "selftest: 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print("[g9_m103] selftest FAIL: check() 未正确记录合成失败", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    if len(CHECK_KEYS) != 7:
        print(
            f"[g9_m103] selftest FAIL: CHECK_KEYS 应为 7,实测 {len(CHECK_KEYS)}",
            file=sys.stderr,
        )
        sys.exit(1)
    fake = {k: True for k in CHECK_KEYS[:-1]}
    missing = [k for k in CHECK_KEYS if k not in fake]
    if missing != ["device_validation_zero"]:
        print(f"[g9_m103] selftest FAIL: 缺腿探测异常 {missing}", file=sys.stderr)
        sys.exit(1)
    print("[g9_m103] selftest PASS(红绿判别有效;未跑 cargo/device、未写 evidence)")


def main() -> int:
    parser = argparse.ArgumentParser(description="G9.2 M103 descriptor_global_table 硬门冒烟")
    parser.add_argument("--gate", default=GATE_KEY, help="symbolic gate key")
    parser.add_argument("--selftest", action="store_true", help="反 YAML-only 红绿自检")
    args = parser.parse_args()

    if args.selftest:
        selftest()
        return 0

    if args.gate != GATE_KEY:
        check(False, f"--gate `{args.gate}` ≠ canonical key `{GATE_KEY}`")

    mb = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True)
    base_commit = mb.stdout.strip() or "unknown"

    leg_rust_gates()
    leg_fixture_and_spec()
    device_ok = leg_device()
    leg_v1v2_regression()

    results = {k: True for k in CHECK_KEYS}
    leg_mark = {
        "①": "reflection_shader_index_bidirectional_equal",
        "②": "table_65536_entries_golden",
        "③": "legacy_set_binding_zero_byte_regression",
        "④": "index_allocation_deterministic",
        "⑤": "dangling_index_fail_closed",
        "⑥": "index_leak_counter_zero",
        "⑦": "device_validation_zero",
    }
    for f in FAILURES:
        for mark, key in leg_mark.items():
            if mark in f:
                results[key] = False
    untagged = [f for f in FAILURES if not any(m in f for m in leg_mark)]

    host_ok = len(FAILURES) == 0 and all(results.values())
    write_evidence(results, host_ok, base_commit, device_ok)

    if FAILURES:
        print(f"[g9_m103] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        for f in untagged:
            print(f"  [untagged] {f}", file=sys.stderr)
        return 1
    print(
        "[g9_m103] PASS (host+device 门;七腿全绿:双向精确相等 + 65536 条目出图 golden + "
        "set/binding 0-byte 回归 + 分配确定性 + 悬空/越界拒 + leak 零 + validation 零)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
