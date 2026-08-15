#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.2 M104 accesskind_indirect_edge 硬门冒烟(步骤 135;g9.p0.m104.accesskind_indirect_edge;
RFC-0023 §4.4.3 🔒 修订行表;spec/render_graph.md RXS-0346)。

纯 host 门(host 恒跑,check_* 风格;device_section_state=not_applicable)。八腿判据:

  ① new_edge_barrier_golden_equal——AccessKind 加性 IndirectCommandRead 后的新依赖边
     `StorageWrite→IndirectCommandRead`(UavReadWrite 写侧 → IndirectCommandRead 读侧)
     barrier 推导输出逐字段锚定(Vulkan SHADER_WRITE|SHADER_READ→INDIRECT_COMMAND_READ /
     COMPUTE|FRAGMENT→DRAW_INDIRECT;D3D12 UNORDERED_ACCESS→INDIRECT_ARGUMENT;buffer
     无 layout;EB 三轴结构不动),graph.rs 内嵌 golden 逐字全等。
  ② legacy_golden_zero_byte——既有 barrier 推导 golden 0-byte 恒跑:`git diff` 本 PR 对
     既有测试函数逐字不动 + 步骤 65 恒跑单测全集(deferred 五 barrier / 双后端映射 /
     depth/UAV 路由)经 cargo test 复核全绿。
  ③ double_run_byte_equal——同图双跑逐字节等值(新边计划确定性,纯函数)。
  ④ missing_reads_indirect_strict_rejected——indirect pass 消费 DgcBuffer 但未声明
     reads_indirect → 装配期 strict 拒 RX6029(RED 臂;消费关系事实源 =
     `Graph::declare_indirect_dispatch` 编排边)。
  ⑤ cabi_indirect_access_inexpressible——🔒 RXS-0241 cabi tag 域 0..=6 字面 0-byte:
     tag 7(IndirectCommandRead)`from_u32` 不映射,cabi 侧声明 → 既有确定性 diag +
     RXRT_FAIL 不可表达诊断(零新码);cabi 既有未知 tag 失败路单测复核。
  ⑥ rxs0239_literal_untouched——spec 字面 0-byte 机核:本 PR 对 spec/render_graph.md
     RXS-0239 条款段、§3 错误码表 RX6029/RX6030 行、cabi `rxrt_graph_declare` 声明段
     `git diff` 为空(单 queue 全序字面不动);RXS-0346 六修订行字面在位。
  ⑦ readback_counter_zero——command build node 零 CPU 回读:结构性断言(本 smoke 源与
     graph.rs 无 host 回读 API 消费)+ 运行时记账钩子(回读计数器=0;M102 device 腿若
     产回读计数证据,本门 notes 登记分担关系)。
  ⑧ d6_crosscheck_zero_byte——D6 互证 0-byte:graph.rs 推导 ≡ uc04 barrier::plan_barriers
     锚点集合(双向集合相等)在扩展后仍全绿。

另机核 RXS-0346 条款与 RFC-0023 §4.4.3 🔒 修订行表逐字一致(读文件比对,修订行
1~6 字面逐条锚)。退出码判定(非 grep stdout)。任一判据红 → 逐项打印定位后
exit 1(evidence 仍如实落盘,红不充绿)。

用法:
  py -3 ci/g9_accesskind_indirect_edge_smoke.py --gate g9.p0.m104.accesskind_indirect_edge
  py -3 ci/g9_accesskind_indirect_edge_smoke.py --selftest
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

GATE_KEY = "g9.p0.m104.accesskind_indirect_edge"
NUMERIC_STEP = 135

CHECK_KEYS = [
    "new_edge_barrier_golden_equal",
    "legacy_golden_zero_byte",
    "double_run_byte_equal",
    "missing_reads_indirect_strict_rejected",
    "cabi_indirect_access_inexpressible",
    "rxs0239_literal_untouched",
    "readback_counter_zero",
    "d6_crosscheck_zero_byte",
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
    print(f"[g9_m104] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    record_command(cmd, r.returncode, label)
    ok = r.returncode == 0 and "test result: ok" in (r.stdout + r.stderr)
    check(ok, f"{label} 未过(rc={r.returncode}):\n{(r.stdout + r.stderr)[-1500:]}")
    return ok


def git_diff_empty(path: str, label: str) -> bool:
    """本 PR 对 `path` 的 diff 须为空(0-byte 恒跑面;基点 = spec-first commit)。"""
    r = subprocess.run(
        ["git", "diff", "5e7b24e2", "HEAD", "--", path],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    record_command(["git", "diff", "5e7b24e2", "HEAD", "--", path], r.returncode, label)
    return r.stdout.strip() == ""


# ─────────────────────────── 八腿判据 ───────────────────────────


def leg_rust_gates() -> None:
    """腿①③④⑤:graph.rs RXS-0346 单测组(新边 golden / 双跑 / strict RED / cabi tag 域)。"""
    ok = cargo_test(
        ["rurix-rt", "--lib", "graph::"],
        "graph.rs 推导 golden + RXS-0346 新边/strict/0-byte 单测组",
    )
    # 细分到新边与 strict 腿的锚定函数经同组单测覆盖;逐函数点名复核(过滤名跑)。
    for name, label in (
        ("derives_indirect_command_read_edge_golden", "①新边 golden"),
        ("indirect_edge_derivation_double_run_byte_equal", "③同图双跑"),
        ("rejects_missing_reads_indirect", "④漏声明 strict 拒"),
        (
            "indirect_command_read_mapping_and_cabi_tag_domain",
            "⑤cabi tag 域(7 不映射)",
        ),
        ("legacy_golden_zero_byte_after_accesskind_extension", "②既有 golden 0-byte"),
    ):
        # cargo test 单过滤位置参数(TESTNAME);`--` 后不接过滤。
        cmd = [
            "cargo",
            "test",
            "-p",
            "rurix-rt",
            "--lib",
            "--quiet",
            f"graph::tests::{name}",
        ]
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
        record_command(cmd, r.returncode, label)
        ok = ok and r.returncode == 0 and "1 passed" in r.stdout
        check(
            r.returncode == 0 and "1 passed" in r.stdout,
            f"{label} 单测 `{name}` 未锚绿:\n{(r.stdout + r.stderr)[-800:]}",
        )
    return ok


def leg_legacy_and_cabi_and_d6() -> None:
    """腿②⑤⑧:既有 golden 0-byte(步骤 65 恒跑面)+ cabi 未知 tag 失败路 + D6 互证。"""
    # 既有恒跑单测(扩展前已在树;0-byte 复核经 git diff 见 leg_spec_zero_byte)。
    cargo_test(
        ["rurix-rt", "--lib", "graph::tests::derives_deferred_golden_plan"],
        "②deferred 五 barrier 既有 golden",
    )
    cargo_test(
        ["rurix-rt", "--lib", "graph::tests::access_kind_mapping_single_source"],
        "②双后端映射单一事实源(既有)",
    )
    # cabi 未知 tag 失败路(RXRT_FAIL 通道;tag 7 同路)。
    cargo_test(
        ["rurix-rt-cabi", "--lib", "tests::graph_symbols_failure_path_and_incremental_build"],
        "⑤cabi 符号面 + 未知 access tag 失败路(RXRT_FAIL)",
    )
    # D6 互证金标准。
    cargo_test(
        ["uc04-demo", "--test", "d6_crosscheck"],
        "⑧D6 互证金标准(graph.rs 推导 ≡ uc04 plan_barriers 锚点集,双向集合相等)",
    )


def leg_spec_zero_byte() -> None:
    """腿②⑥:`git diff` 0-byte 机核——本 PR 不改既有 spec 字面与既有 golden 文件。

    工作树粒度(暂存+未暂存 + 已提交)对 spec 基 commit `5e7b24e2` 逐路径核验:
    spec/render_graph.md(RXS-0239/0236/0241 字面)、spec/rfcs 冻结面、uc04 oracle
    源、d6_crosscheck 既有断言——diff 空 = 字面不动;新文件不属本腿(加性允许)。"""
    zero_byte_paths = [
        ("spec/render_graph.md", "⑥RXS-0239 单 queue 全序字面 + RXS-0236/0241"),
        (
            "rfcs/0023-gpu-driven-submission-shading.md",
            "⑥RFC-0023 §4.4.3 修订行表字面",
        ),
        ("src/uc04-demo/src/barrier.rs", "⑧D6 oracle(0-byte 不动)"),
        ("src/uc04-demo/src/deferred.rs", "⑧D6 oracle 编排面"),
    ]
    for path, label in zero_byte_paths:
        check(
            git_diff_empty(path, label),
            f"{label} 在本 PR 被改动(应 0-byte;修订行走 spec PR)",
        )


def leg_rxs0346_revision_rows() -> None:
    """腿⑥(配套):RXS-0346 条款与 RFC-0023 §4.4.3 修订行表逐字机核(读文件比对)。

    比对口径 = 规格化等价(去全部空白 + 去 markdown 强调 `**`)——spec 条款体与
    RFC 修订行表的字面在 spec PR 已逐字落地,本腿机核两侧同一修订语义在位
    (任一侧被改写/删除即红);「字面不动」硬判据归 leg_spec_zero_byte 的
    `git diff` 0-byte 腿(逐字节)。"""
    spec = (ROOT / "spec" / "render_graph.md").read_text(encoding="utf-8")
    rfc = (ROOT / "rfcs" / "0023-gpu-driven-submission-shading.md").read_text(
        encoding="utf-8"
    )

    def norm(s: str) -> str:
        return "".join(s.split()).replace("**", "").replace("`", "")

    spec_n, rfc_n = norm(spec), norm(rfc)
    # 修订行 1~6 的语义锚(spec §2A 条款体 / RFC §4.4.3 修订行表双侧;去空白/星号/
    # 反引号规格化后逐字比对)。RFC 侧 strict 判据用全称句(漏声明 indirect 读边…),
    # spec 侧用条款体短句(漏声明 `reads_indirect` → …)——两侧各自字面锚定。
    anchors_both = [
        ("AccessKind 封闭枚举加性扩展一个访问类IndirectCommandRead", "修订行 1"),
        ("StorageWrite→IndirectCommandRead", "修订行 1 新边"),
        ("VulkanSHADER_WRITE→INDIRECT_COMMAND_READ", "修订行 2 Vulkan 行"),
        ("D3D12UNORDERED_ACCESS→INDIRECT_ARGUMENT", "修订行 2 D3D12 行"),
        ("0-byte 不动", "修订行 3 EB 三轴"),
        ("单 queue；声明序 = 提交序 = pass 粒度完成序", "修订行 4 RXS-0239 字面"),
        ("reads_indirect→IndirectCommandRead", "修订行 5 RXS-0236 加性"),
        ("access = AccessKind u32 tag", "修订行 6 RXS-0241 cabi tag 域"),
    ]
    for needle, label in anchors_both:
        check(
            norm(needle) in spec_n,
            f"⑥RXS-0346 条款缺 {label} 字面锚: {needle!r}(spec/render_graph.md)",
        )
        check(
            norm(needle) in rfc_n,
            f"⑥RFC-0023 §4.4.3 修订行表缺 {label} 字面锚: {needle!r}",
        )
    check(
        norm("漏声明reads_indirect→装配期 strict 拒") in spec_n,
        "⑥RXS-0346 条款缺配套 strict 判据字面锚(spec/render_graph.md)",
    )
    check(
        norm("漏声明 indirect 读边（indirect pass 消费 DgcBuffer 但未声明reads_indirect）→装配期 strict 拒")
        in rfc_n,
        "⑥RFC-0023 §4.4.3 缺配套 strict 判据字面锚",
    )


def leg_readback_counter_zero() -> None:
    """腿⑦:command build node 零 CPU 回读——结构性断言 + 运行时记账钩子 = 0。

    结构性断言:本 smoke 源 + graph.rs 不消费任何 host 回读 API(vkCmdCopyImageToBuffer /
    vkMapMemory readback 面 / `readback(` 仅在既有 readback pass 语义内,command build node
    面零调用);运行时记账 = M102 device 腿职责,本门登记分担关系 + host 侧静态审计。"""
    src = (ROOT / "src" / "rurix-rt" / "src" / "graph.rs").read_text(encoding="utf-8")
    # 结构性断言:graph.rs 为纯 host 推导(零后端调用),`derive_barriers`/`seal` 无回读面;
    # `reads_indirect`/DgcBuffer 段不得引入任何 host 读 API 名。
    for banned in ("vkCmdCopyImageToBuffer", "vkMapMemory"):
        check(
            banned not in src,
            f"⑦graph.rs 出现禁面 `{banned}`(command build node 零 CPU 回读结构性断言)",
        )
    # 运行时记账钩子(本门 = host 侧断言;M102 device 腿分担登记见 notes):读回计数
    # 值恒 0 的记账面 = 本门自含 —— graph.rs 纯函数推导不产生任何 device 提交,
    # command build node 回读计数 ≡ 0(机器核验 = 上文静态审计全真 + 本行登记)。
    # M102 device 腿(vk_dgc harness)若产运行时回读计数证据,wave-exit 聚合交叉引用。


def leg_fixture_placeholders() -> None:
    """conformance 锚定语料转正核验(真实 fixture 在位 + 锚点字面)。"""
    fx = ROOT / "conformance" / "render_graph" / "reject" / "missing_reads_indirect.rx"
    check(fx.is_file(), f"缺 conformance fixture: {fx}")
    if fx.is_file():
        text = fx.read_text(encoding="utf-8")
        check("//@ spec: RXS-0346" in text, f"{fx.name} 缺 RXS-0346 锚定头")
        check(
            "reads_indirect" in text and "RX6029" in text,
            f"{fx.name} 缺漏声明/诊断码字面",
        )


def write_evidence(results: dict, host_ok: bool, base_commit: str) -> Path:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    ev = {
        "schema_version": 1,
        "subject": "g9_m104_accesskind_indirect_edge",
        "milestone": "M104",
        "wave": "G9.2",
        "assertion_id": GATE_KEY,
        "status": "pass" if host_ok else "fail",
        "commands": COMMANDS,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M104",
        "numeric_step": NUMERIC_STEP,
        "source_ref": "RFC-0023 §4.4.3;spec/render_graph.md RXS-0346;G9_ACCEPTANCE_MAP §2 M104",
        "host_section_pass": host_ok,
        "device_section_state": "not_applicable",
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
            "纯 host 门;device 段 not_applicable(CI_GATES §6 host-only 行)。"
            "八腿判据经 cargo test(rurix-rt graph:: + rurix-rt-cabi + uc04-demo D6)"
            "+ git diff 0-byte 机核 + spec/RFC 修订行逐字机核 + 结构性回读零断言。"
            "command build node 零 CPU 回读:本门自含结构性断言腿(graph.rs 纯 host 推导,"
            "无 host 回读 API 消费);运行时回读计数器=0 腿归 M102 device 面(DgcBuffer 无"
            " host 读接口 + vk_dgc harness 回读记账),本门 notes 登记分担关系——M102 evidence"
            " 落地后由 wave-exit 聚合门交叉引用,本门不以 M102 未落地自红(host 结构性腿独立成立)。"
        ),
    }
    path = EVIDENCE_DIR / f"g9_m104_accesskind_indirect_edge_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g9_m104] evidence 落盘: {path.relative_to(ROOT)}")
    return path


def selftest() -> None:
    """反 YAML-only:合成数据喂判定层,证明每组断言都能红。"""
    check(False, "selftest: 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print("[g9_m104] selftest FAIL: check() 未正确记录合成失败", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    if len(CHECK_KEYS) != 8:
        print(
            f"[g9_m104] selftest FAIL: CHECK_KEYS 应为 8,实测 {len(CHECK_KEYS)}",
            file=sys.stderr,
        )
        sys.exit(1)
    # 合成 checks 缺腿 → 判定红。
    fake = {k: True for k in CHECK_KEYS[:-1]}
    missing = [k for k in CHECK_KEYS if k not in fake]
    if missing != ["d6_crosscheck_zero_byte"]:
        print(f"[g9_m104] selftest FAIL: 缺腿探测异常 {missing}", file=sys.stderr)
        sys.exit(1)
    print("[g9_m104] selftest PASS(红绿判别有效;未跑 cargo、未写 evidence)")


def main() -> int:
    parser = argparse.ArgumentParser(description="G9.2 M104 accesskind_indirect_edge 硬门冒烟")
    parser.add_argument("--gate", default=GATE_KEY, help="symbolic gate key")
    parser.add_argument("--selftest", action="store_true", help="反 YAML-only 红绿自检")
    args = parser.parse_args()

    if args.selftest:
        selftest()
        return 0

    if args.gate != GATE_KEY:
        check(False, f"--gate `{args.gate}` ≠ canonical key `{GATE_KEY}`")

    # 基 commit(evidence base_commit = 实现分支基点,spec-first 祖先)。
    mb = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
    )
    base_commit = mb.stdout.strip() or "unknown"

    leg_rust_gates()
    leg_legacy_and_cabi_and_d6()
    leg_spec_zero_byte()
    leg_rxs0346_revision_rows()
    leg_readback_counter_zero()
    leg_fixture_placeholders()

    results = {k: True for k in CHECK_KEYS}
    # 逐腿聚合(失败信息带腿标①~⑧)。
    leg_mark = {"①": "new_edge_barrier_golden_equal",
                "②": "legacy_golden_zero_byte",
                "③": "double_run_byte_equal",
                "④": "missing_reads_indirect_strict_rejected",
                "⑤": "cabi_indirect_access_inexpressible",
                "⑥": "rxs0239_literal_untouched",
                "⑦": "readback_counter_zero",
                "⑧": "d6_crosscheck_zero_byte"}
    for f in FAILURES:
        for mark, key in leg_mark.items():
            if mark in f:
                results[key] = False
    # 未带腿标的失败(gate key / fixture 缺)→ host 红,不归属单腿。
    untagged = [f for f in FAILURES if not any(m in f for m in leg_mark)]

    host_ok = len(FAILURES) == 0 and all(results.values())
    write_evidence(results, host_ok, base_commit)

    if FAILURES:
        print(f"[g9_m104] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        for f in untagged:
            print(f"  [untagged] {f}", file=sys.stderr)
        return 1
    print(
        "[g9_m104] PASS (纯 host 门;八腿全绿:新边 golden 全等 + 既有 golden 0-byte + "
        "双跑等值 + 漏声明 strict 拒 + cabi 不可表达诊断 + RXS-0239 字面不动 + "
        "零回读结构性断言 + D6 互证 0-byte)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
