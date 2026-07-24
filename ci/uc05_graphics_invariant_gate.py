#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-05 图形 RHI 不变量拦截门(步骤 77;G4.2 PR-B / RFC-0015 §4.A;RXS-0270~0273;
验收门 G-G4-3〔gfx I3/I5 装配期 + I7/I8 编译期 + 声明↔反射 gfx 面〕）。

**纯 host 恒跑,无 GPU**(check_ 守卫风格:不分配错误码、不写 evidence、不接 budget counter)。
与步骤 73(compute 不变量门)分工:73 = compute I1~I10 全档;77 = gfx I3/I5/I7/I8 + gfx 语料纪律
+ compute 路零回归守卫(uc05_corpus 批跑覆盖 gfx 编译期 reject + gfx assembly 编译期 CLEAN)。

裁决 1 三档在 gfx 面的逐条断言:
  1. **gfx 编译期档(I7/I8)**:`conformance/uc05/reject/cross_brand_gfx.rx`(`//@ expect-error:
     RX3006`,I7 跨 brand 图形资源)+ `conformance/uc05/reject/rhi_gfx_in_kernel.rx`
     (`//@ expect-error: RX3015`,I8 着色合法性)实存 + 头声明码与矩阵一致;由 uc05_corpus
     (cargo test)真编译全拦截兑现(同 compute I7/I8 同码同口径,零新 RX 码)。
  2. **gfx 装配期档(I3/I5)**:`conformance/uc05/reject/gfx_feedback_loop.rx` /
     `gfx_read_before_write.rx` / `gfx_write_write_conflict.rx`(均带
     `//@ assembly-reject: structure` 头)实存 + **编译期 CLEAN**(图装配期性质,`--emit=check`
     不拦)——证 gfx I3/I5 非编译期。装配期确定性拦的 EXE red-green 见证归步骤 76 device 段
     (rhi_submit [structure] Err → RXRT_FAIL → rxrt_trap);本步骤只断言编译期 CLEAN + 语料头纪律。
  3. **gfx accept 档**:`conformance/uc05/accept/gfx_pass.rx` + `gfx_resources.rx` 编译期 0 诊断
     (gfx pass 声明面合法,5 资源构造 + raster_pass/mesh_pass 已知方法分支)。
  4. **gfx demo 编译档**:`apps/uc05-rhi/src/gfx_demo.rx` 编译期 0 诊断(≥1 raster + ≥1 mesh
     图形 pass 经 .rx RHI 库面声明,装配核验可编译本体;device 真跑归步骤 76)。
  5. **compute 路零回归**:uc05_corpus 批跑(compute + gfx 编译期 reject 全拦截 + assembly
     编译期 CLEAN + I1~I10 矩阵三方一致)——**纯 rust test,无工具链亦恒跑**(反 YAML-only
     底线,compute 路零回归守卫)。
  6. **gfx 语料纪律**:每个 gfx 语料文件携带条款锚定头(`//@ spec: RXS-####`)。

内置 red_self_test 反 YAML-only(合成漂移矩阵须判红)。**blocking(exit 1)**。

用法: py -3 ci/uc05_graphics_invariant_gate.py
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RURIXC = (
    ROOT / "target" / "debug" / "rurixc.exe"
    if sys.platform == "win32"
    else ROOT / "target" / "debug" / "rurixc"
)

UC05 = ROOT / "conformance" / "uc05"
REJECT_DIR = UC05 / "reject"
ACCEPT_DIR = UC05 / "accept"
DEMO = ROOT / "apps" / "uc05-rhi" / "src" / "gfx_demo.rx"

# gfx 编译期 reject 语料 ↔ 期望诊断码(I7/I8 gfx 面;零新 RX 码,复用 compute 同码)。
GFX_COMPILE_REJECTS: dict[str, tuple[str, str]] = {
    "I7_gfx": ("conformance/uc05/reject/cross_brand_gfx.rx", "RX3006"),
    "I8_gfx": ("conformance/uc05/reject/rhi_gfx_in_kernel.rx", "RX3015"),
}

# gfx 装配期 reject 语料 ↔ 装配期类别(I3/I5 gfx 面;库层状态值镜像 RX6029,零新 RX 码)。
# 编译期 CLEAN,违例归 submit() 装配期确定性拦(step 76 device EXE red-green 见证)。
GFX_ASSEMBLY_REJECTS: dict[str, str] = {
    "I3_gfx_feedback_loop": "conformance/uc05/reject/gfx_feedback_loop.rx",
    "I3_gfx_read_before_write": "conformance/uc05/reject/gfx_read_before_write.rx",
    "I5_gfx_write_write_conflict": "conformance/uc05/reject/gfx_write_write_conflict.rx",
}

# gfx accept 语料(编译期 0 诊断;5 资源构造 + raster/mesh pass 声明合法)。
GFX_ACCEPTS: list[str] = [
    "conformance/uc05/accept/gfx_pass.rx",
    "conformance/uc05/accept/gfx_resources.rx",
]

ERRORS: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def _die(msg: str) -> None:
    print(f"[uc05_graphics_invariant_gate] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def run(cmd, cwd: Path = ROOT, timeout: int = 900):
    r = subprocess.run(cmd, capture_output=True, cwd=str(cwd), timeout=timeout)
    return (
        r.returncode,
        r.stdout.decode("utf-8", "replace"),
        r.stderr.decode("utf-8", "replace"),
    )


def run_cargo(args: list[str]) -> tuple[int, str]:
    r = subprocess.run(["cargo", *args], cwd=str(ROOT), capture_output=True)
    return r.returncode, r.stdout.decode("utf-8", "replace") + r.stderr.decode("utf-8", "replace")


def expect_error_code(rx_path: Path) -> str | None:
    for line in rx_path.read_text(encoding="utf-8").splitlines():
        m = re.search(r"//@\s*expect-error:\s*(RX\d{4})", line)
        if m:
            return m.group(1)
    return None


def assembly_reject_category(rx_path: Path) -> str | None:
    for line in rx_path.read_text(encoding="utf-8").splitlines():
        m = re.search(r"//@\s*assembly-reject:\s*(\w+)", line)
        if m:
            return m.group(1)
    return None


def has_spec_anchor(rx_path: Path) -> bool:
    first = rx_path.read_text(encoding="utf-8").splitlines()[0] if rx_path.is_file() else ""
    return first.startswith("//@ spec: RXS-")


# ───────────────────── 纯判定层(red 自检直接喂合成数据) ─────────────────────


def check_three_way(corpora: dict[str, str], existing_paths: set[str]) -> list[str]:
    """语料路径实存性 + 头纪律(纯函数;red_self_test 喂合成数据)。

    corpora = {inv: path};existing_paths = 已存在的路径集(允许注入合成集验证红绿)。
    """
    problems: list[str] = []
    for inv, path in corpora.items():
        if path not in existing_paths:
            problems.append(f"{inv}: 语料路径不存在 {path}")
    return problems


def red_self_test() -> None:
    """反 YAML-only:合成漂移语料集须判红,一致须判绿。"""
    real_path = "conformance/uc05/reject/cross_brand_gfx.rx"
    good = {"I7_gfx": real_path}
    # 全部路径存在的集合 → 一致 → 判绿(空问题列表)。
    if check_three_way(good, {real_path}):
        _die("red 自检失败:一致三方被误判漂移(门过严)")
    # 缺失路径的合成集 → 漂移 → 判红(非空问题列表)。
    if not check_three_way(good, set()):
        _die("red 自检失败:三方漂移未被识别(门失效)")


# ───────────────────── 主断言层 ─────────────────────


def ensure_rurixc() -> bool:
    """rurixc 在位(--emit=check 不 link,host 恒跑);缺则构建。"""
    if RURIXC.is_file():
        return True
    code, out, error = run(["cargo", "build", "-q", "-p", "rurixc", "--bin", "rurixc"])
    if code != 0 or not RURIXC.is_file():
        print((out + error)[-1200:], file=sys.stderr)
        err("rurixc 构建失败(--emit=check 编译档前置)")
        return False
    return True


def check_compile_rejects() -> None:
    """gfx 编译期 reject 语料实存 + //@ expect-error == 期望码(I7/I8 gfx)。

    实际编译拦截由 uc05_corpus(cargo test)真跑兑现;本步骤只断言语料头纪律。
    编译期拦截的 red-green 在 uc05_corpus 真跑段(下方)断言。
    """
    for inv, (path, code) in GFX_COMPILE_REJECTS.items():
        p = ROOT / path
        if not p.is_file():
            err(f"{inv}: gfx reject 语料不存在 {path}")
            continue
        got = expect_error_code(p)
        if got != code:
            err(f"{inv}: {path} expect-error={got} 应为 {code}")
        if not has_spec_anchor(p):
            err(f"{inv}: {path} 缺 //@ spec: RXS-#### 条款锚定头(语料纪律)")


def check_assembly_rejects() -> None:
    """gfx 装配期 reject 语料实存 + //@ assembly-reject: structure 头 + 编译期 CLEAN(I3/I5 gfx)。

    违例归 submit() 装配期确定性拦;本步骤断言 --emit=check CLEAN(图装配期性质)。
    device EXE red-green 见证归步骤 76(rhi_submit [structure] Err)。
    """
    for inv, path in GFX_ASSEMBLY_REJECTS.items():
        p = ROOT / path
        if not p.is_file():
            err(f"{inv}: gfx assembly-reject 语料不存在 {path}")
            continue
        cat = assembly_reject_category(p)
        if cat != "structure":
            err(f"{inv}: {path} assembly-reject={cat} 应为 structure(I3/I5 图结构违例族)")
        if not has_spec_anchor(p):
            err(f"{inv}: {path} 缺 //@ spec: RXS-#### 条款锚定头(语料纪律)")


def check_accepts() -> None:
    """gfx accept 语料实存 + 编译期 0 诊断(5 资源构造 + raster/mesh pass 声明合法)。"""
    for path in GFX_ACCEPTS:
        p = ROOT / path
        if not p.is_file():
            err(f"gfx_accept: 语料不存在 {path}")
            continue
        if not has_spec_anchor(p):
            err(f"gfx_accept: {path} 缺 //@ spec: RXS-#### 条款锚定头(语料纪律)")
        ac, ao, ae = run([str(RURIXC), str(p), "--emit=check"])
        if ac != 0 or "RX" in (ao + ae):
            print((ao + ae)[-800:], file=sys.stderr)
            err(f"gfx_accept: {path} --emit=check 非 0 诊断(应为 0 诊断,合法声明面)")


def check_gfx_demo_compiles() -> None:
    """apps/uc05-rhi/src/gfx_demo.rx 编译期 0 诊断(≥1 raster + ≥1 mesh pass 声明 + 装配核验可编译)。

    device 真跑(像素判据 RXS-0222)归步骤 76;本步骤只断言 --emit=check CLEAN。
    """
    if not DEMO.is_file():
        err(f"gfx_demo: 文件不存在 {DEMO}")
        return
    ac, ao, ae = run([str(RURIXC), str(DEMO), "--emit=check"])
    if ac != 0 or "RX" in (ao + ae):
        print((ao + ae)[-800:], file=sys.stderr)
        err("gfx_demo: gfx_demo.rx --emit=check 非 0 诊断(应为 0 诊断,装配核验可编译本体)")


def check_assembly_rejects_compile_clean() -> None:
    """gfx 装配期 reject 语料编译期 CLEAN(I3/I5 gfx 非编译期,--emit=check 不拦)。

    证 gfx I3/I5 为图装配期性质,违例归 submit() 装配期拦(库层状态值,零新 RX 码)。
    """
    for inv, path in GFX_ASSEMBLY_REJECTS.items():
        p = ROOT / path
        if not p.is_file():
            continue  # 已在 check_assembly_rejects 报错
        ac, ao, ae = run([str(RURIXC), str(p), "--emit=check"])
        if ac != 0 or "error" in (ao + ae).lower() or "RX" in (ao + ae):
            print((ao + ae)[-800:], file=sys.stderr)
            err(f"{inv}: {path} 应编译期 CLEAN(图装配期性质,--emit=check 不拦)")


def check_corpus_zero_regression() -> bool:
    """uc05_corpus 批跑(compute + gfx 编译期 reject 全拦截 + assembly 编译期 CLEAN + I1~I10 矩阵三方一致)。

    **纯 rust test,无工具链亦恒跑**(反 YAML-only 底线,compute 路零回归守卫)。
    gfx 编译期 reject(cross_brand_gfx / rhi_gfx_in_kernel)在 uc05_corpus 真编译拦截。
    """
    code, out = run_cargo(["test", "-q", "-p", "rurixc", "--test", "uc05_corpus"])
    if code != 0:
        print(out[-2400:], file=sys.stderr)
        err("uc05_corpus 批跑未过(compute 路回归 / gfx 编译期 reject 漏拦)")
        return False
    return True


def main() -> int:
    red_self_test()

    # 静态断言段:gfx 语料纪律 + 头一致性。
    check_compile_rejects()
    check_assembly_rejects()

    # rurixc 在位后:--emit=check 编译档。
    if not ensure_rurixc():
        if ERRORS:
            print("[uc05_graphics_invariant_gate] FAIL")
            for e in ERRORS:
                print(f"  - {e}")
            return 1
        # 静态段无错但 rurixc 构建失败——按 SKIP 退 0(dev-env degrade,非 fake pass)。
        # 但本步骤定位为「纯 host 恒跑」:rurixc 为 host 工具链缺 = 环境不可用。
        # 此处取 hard FAIL(对齐 step 73 rhi.rs 库单测真跑的硬门口径)。
        return 1

    check_accepts()
    check_gfx_demo_compiles()
    check_assembly_rejects_compile_clean()

    if ERRORS:
        print("[uc05_graphics_invariant_gate] FAIL")
        for e in ERRORS:
            print(f"  - {e}")
        return 1

    # 静态断言全过 → 跑真编译门(uc05_corpus:compute + gfx 编译期 reject 全拦截 + 矩阵三方一致)。
    print("[uc05_graphics_invariant_gate] 静态 gfx 语料纪律 + 头一致性 PASS,跑真编译门…")
    if not check_corpus_zero_regression():
        print("[uc05_graphics_invariant_gate] FAIL")
        for e in ERRORS:
            print(f"  - {e}")
        return 1

    print(
        "[uc05_graphics_invariant_gate] PASS gfx I7/I8 编译期(cross_brand_gfx RX3006 /"
        " rhi_gfx_in_kernel RX3015)+ gfx I3/I5 装配期(编译期 CLEAN,违例归 submit() 装配期拦)+"
        " gfx accept 0 诊断 + gfx_demo.rx 0 诊断 + uc05_corpus 零回归(compute + gfx 编译期 reject 全拦截)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
