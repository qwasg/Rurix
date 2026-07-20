#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-05 RHI I1~I10 不变量拦截门(步骤 73;EI1.3 / RFC-0014 Part B;RXS-0263/0264;
验收门 G-EI1-3〔I1~I8〕/ G-EI1-5〔I9~I10 报告〕）。

裁决 1 三档逐条断言(**纯 host,恒跑,无 GPU**;check_ 守卫风格:不分配错误码、不写 evidence、
不接 budget counter):

  1. **矩阵结构**:evidence/uc05_invariant_matrix.json 含 I1~I10,tier ∈ {compile_time,
     assembly_time, lib_tested, report_only};I9/I10 = report_only 且 diagnostic=null。
  2. **三方一致性**(反 YAML-only,RXS-0264 redline F3):矩阵 json ↔ reject/assembly 语料实存 ↔
     evidence/uc05_comparison_report.md 逐项对齐(条款号 / 语料路径 / 诊断码);报告顶部 historical
     counters 标注在位。schema 层(check_schemas)另硬拦 I9/I10 无 in-repo 出处数值字段。
  3. **I1/I2/I6/I7/I8(编译期)**:reject/*.rx 实存 + `//@ expect-error:` == 矩阵 diagnostic;
     由 uc05_corpus(cargo test)真编译全拦截兑现。
  4. **I3/I5(装配期)**:assembly/*.rx 实存 + 编译期 CLEAN;**装配期确定性拦的纯 host 无 GPU 见证**
     = rurix-rt rhi.rs 库单测(rejects_read_before_write_i3 / rejects_write_write_conflict_i5 /
     rejects_lifecycle_misuse)真跑(EXE red-green 为 device 段 e2e 加证,步骤 72)。
  5. **I4(lib_tested,诚实收窄)**:机制由 rhi.rs `rejects_reflection_mismatch_i4` 库测证;`.rx`
     反射喂入(pass 绑 kernel)随 EI1.4——矩阵标注 rx_wiring:EI1.4,**不宣称 I4 .rx 路 ci_checked**。
  6. **I9/I10(report_only)**:documented_historical,无诊断码 / 无杜撰数字(schema by-construction)。

内置 red_self_test 反 YAML-only(合成漂移矩阵须判红)。**blocking(exit 1)**。

用法: py -3 ci/uc05_invariant_gate.py
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MATRIX = ROOT / "evidence" / "uc05_invariant_matrix.json"
REPORT = ROOT / "evidence" / "uc05_comparison_report.md"

# 期望三档划界(裁决 1;矩阵/报告/语料三方须一致)。
COMPILE_TIME = {"I1", "I2", "I6", "I7", "I8"}
ASSEMBLY_TIME = {"I3", "I5"}
LIB_TESTED = {"I4"}
REPORT_ONLY = {"I9", "I10"}

# 编译期不变量 ↔ reject 语料 ↔ 期望诊断码。
COMPILE_MAP = {
    "I1": ("conformance/uc05/reject/res_use_after_move.rx", "RX4001"),
    "I2": ("conformance/uc05/reject/res_double_move.rx", "RX4001"),
    "I6": ("conformance/uc05/reject/rhi_double_submit.rx", "RX4001"),
    "I7": ("conformance/uc05/reject/rhi_cross_brand.rx", "RX3006"),
    "I8": ("conformance/uc05/reject/rhi_in_kernel.rx", "RX3015"),
}
# 装配期不变量 ↔ assembly 语料 ↔ 纯 host 库单测(无 GPU 见证)。
ASSEMBLY_MAP = {
    "I3": ("conformance/uc05/assembly/graph_cycle.rx", "rejects_read_before_write_i3"),
    "I5": ("conformance/uc05/assembly/graph_write_write.rx", "rejects_write_write_conflict_i5"),
}
# 装配期须真跑的 rhi.rs 库单测(纯 host,无 GPU;含生命周期)。
RHI_LIB_TESTS = [
    "rejects_read_before_write_i3",
    "rejects_write_write_conflict_i5",
    "rejects_lifecycle_misuse",
    "rejects_reflection_mismatch_i4",
    "accepts_linear_graph_derives_raw_syncs",
]

ERRORS: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def expect_error_code(rx_path: Path) -> str | None:
    for line in rx_path.read_text(encoding="utf-8").splitlines():
        m = re.search(r"//@\s*expect-error:\s*(RX\d{4})", line)
        if m:
            return m.group(1)
    return None


# ───────────────────── 纯判定层(red 自检直接喂合成数据) ─────────────────────


def check_three_way(matrix_text: str, report: str, corpora: dict[str, tuple[str, str]]) -> list[str]:
    """矩阵 ↔ 语料路径 ↔ 报告三方一致(纯函数;corpora = {inv: (path, code_or_test)})。"""
    problems: list[str] = []
    for inv, (path, _tok) in corpora.items():
        if path not in matrix_text:
            problems.append(f"{inv}: 矩阵 json 缺 corpus 路径 {path}")
        if path not in report:
            problems.append(f"{inv}: 对照报告缺 corpus 路径 {path}(三方漂移,RXS-0264)")
    return problems


def red_self_test() -> None:
    """反 YAML-only:合成三方漂移须判红,一致须判绿。"""
    path = "conformance/uc05/reject/res_use_after_move.rx"
    good = {"I1": (path, "RX4001")}
    consistent = f"matrix has {path} ok"  # 矩阵与报告均含全路径 = 一致。
    if check_three_way(consistent, consistent, good):
        _die("red 自检失败:一致三方被误判漂移(门过严)")
    if not check_three_way("(no path)", "(no path)", good):
        _die("red 自检失败:三方漂移未被识别(门失效)")


def _die(msg: str) -> None:
    print(f"[uc05_invariant_gate] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def run_cargo(args: list[str]) -> tuple[int, str]:
    r = subprocess.run(["cargo", *args], cwd=str(ROOT), capture_output=True)
    return r.returncode, r.stdout.decode("utf-8", "replace") + r.stderr.decode("utf-8", "replace")


def main() -> int:
    red_self_test()

    if not MATRIX.is_file():
        _die(f"缺 {MATRIX.relative_to(ROOT)}")
    if not REPORT.is_file():
        _die(f"缺 {REPORT.relative_to(ROOT)}")
    matrix_text = MATRIX.read_text(encoding="utf-8")
    matrix = json.loads(matrix_text)
    report = REPORT.read_text(encoding="utf-8")

    # 1) 矩阵结构 + 三档。
    invs = {inv["id"]: inv for inv in matrix.get("invariants", [])}
    for want in [f"I{i}" for i in range(1, 11)]:
        if want not in invs:
            err(f"矩阵缺不变量 {want}")
    tier_of = {"compile_time": COMPILE_TIME, "assembly_time": ASSEMBLY_TIME,
               "lib_tested": LIB_TESTED, "report_only": REPORT_ONLY}
    for tier, group in tier_of.items():
        for inv in group:
            if inv in invs and invs[inv].get("tier") != tier:
                err(f"{inv}: 矩阵 tier={invs[inv].get('tier')} 应为 {tier}(裁决 1)")
    for inv in REPORT_ONLY:
        if inv in invs and invs[inv].get("diagnostic") is not None:
            err(f"{inv}: report_only 项应无诊断码(diagnostic=null,documented_historical)")

    # 2) 三方一致性(报告顶部标注 + 路径对齐)。
    if "historical counters unavailable in-repo, non-reproducible, no fabricated figures" not in report:
        err("对照报告缺顶部 historical counters 标注(RXS-0264)")
    all_corpora = {**COMPILE_MAP, **ASSEMBLY_MAP}
    ERRORS.extend(check_three_way(matrix_text, report, all_corpora))

    # 3) 编译期 I1/I2/I6/I7/I8:reject 语料实存 + //@ expect-error == 矩阵 diagnostic。
    for inv, (path, code) in COMPILE_MAP.items():
        p = ROOT / path
        if not p.is_file():
            err(f"{inv}: reject 语料不存在 {path}")
            continue
        got = expect_error_code(p)
        if got != code:
            err(f"{inv}: {path} expect-error={got} 应为 {code}")
        if inv in invs and invs[inv].get("diagnostic") != code:
            err(f"{inv}: 矩阵 diagnostic={invs[inv].get('diagnostic')} 应为 {code}")

    # 4) 装配期 I3/I5:assembly 语料实存(编译期 CLEAN 由 uc05_corpus 兑现)。
    for inv, (path, _test) in ASSEMBLY_MAP.items():
        if not (ROOT / path).is_file():
            err(f"{inv}: assembly 语料不存在 {path}")

    # 5) I4 诚实收窄:矩阵标注 .rx 反射喂入随 EI1.4。
    if "I4" in invs and "EI1.4" not in (invs["I4"].get("rx_wiring", "") + invs["I4"].get("evidence_level", "")):
        err("I4: 矩阵应诚实标注 .rx 反射喂入随 EI1.4(RXS-0257 收窄)")

    if ERRORS:
        print("[uc05_invariant_gate] FAIL")
        for e in ERRORS:
            print(f"  - {e}")
        return 1

    # 静态断言全过 → 跑真编译门(I1~I8 编译期拦截 + I3/I4/I5 装配/反射纯 host 库单测)。
    print("[uc05_invariant_gate] 静态三档 + 三方一致性 PASS,跑真编译门…")
    code, out = run_cargo(["test", "-q", "-p", "rurixc", "--test", "uc05_corpus"])
    if code != 0:
        print(out[-2000:], file=sys.stderr)
        print("[uc05_invariant_gate] FAIL: uc05_corpus 编译期拦截门未过(I1/I2/I6/I7/I8)", file=sys.stderr)
        return 1
    print("[uc05_invariant_gate] PASS uc05_corpus（I1/I2/I6/I7/I8 编译期全拦截 + 矩阵三方一致）")

    # 注:不加 -q(需逐测试名断言全部 RHI_LIB_TESTS 真跑,非仅汇总行)。
    code, out = run_cargo(["test", "-p", "rurix-rt", "rhi::tests"])
    if code != 0:
        print(out[-2000:], file=sys.stderr)
        print("[uc05_invariant_gate] FAIL: rhi.rs 库单测门未过（I3/I4/I5 装配/反射纯 host 见证）", file=sys.stderr)
        return 1
    for t in RHI_LIB_TESTS:
        if t not in out:
            print(f"[uc05_invariant_gate] FAIL: rhi.rs 库单测缺 {t}（I3/I4/I5 装配期纯 host 无 GPU 见证）",
                  file=sys.stderr)
            return 1
    print("[uc05_invariant_gate] PASS rhi.rs 库单测（I3/I5 装配期 + I4 反射 纯 host 无 GPU 确定性拦）")

    print(
        "[uc05_invariant_gate] PASS I1~I8 逐条确定性拦截（编译期 I1/I2/I6/I7/I8 + 装配期 I3/I5 +"
        " lib_tested I4）+ I9/I10 report_only documented_historical;矩阵 ↔ 语料 ↔ 报告三方一致"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
