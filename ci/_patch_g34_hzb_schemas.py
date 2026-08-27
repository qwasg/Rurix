#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude（G34 全特性合流收口批 G34-2）
# G34 全特性合流 G34-2：check_schemas.py 三处纯追加注册（io.open 补丁法——
# newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：已驻留即只做核验（ci/_patch_g34_unified_lane_schemas.py 同法）。
# 注册面 = 仅门裁决件（g34_hzb_unified_gate_ 前缀单路由）：HZB harness 真跑件
# 不注册 check_schemas,数字经门裁决件蒸馏登记；baseline 对照腿归档用
# g34_unified_lane_g34hzb_ 前缀走 g34_unified_lane_ 既有路由。
# 锚选择与 ci/_patch_g34_skin_schemas.py 同块（均为锚后纯追加）——主 agent
# 串行应用任意先后互不破坏：应用后锚字面仍完整驻留，token 各自独立。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    g34_unified_lane_schema = load(
        ROOT / "milestones/g34/g34_unified_lane_evidence_schema.json"
    )
    g34_unified_lane_gate_schema = load(
        ROOT / "milestones/g34/g34_unified_lane_gate_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G34 全特性合流 G34-2 HZB 统一车道门前缀纯追加（重放幂等面；仅门裁决件
    # 注册——HZB harness 真跑件不注册 check_schemas,数字经门裁决件蒸馏登记；
    # g34_ 族前缀分岔:unified_lane/skin_unified/hzb_unified 首段 u/s/h 分岔
    # 互不包含）
    g34_hzb_unified_gate_schema = load(
        ROOT / "milestones/g34/g34_hzb_unified_gate_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g34_unified_lane_gate_validator = (
        jsonschema.Draft7Validator(g34_unified_lane_gate_schema)
        if g34_unified_lane_gate_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g34_hzb_unified_gate_validator = (
        jsonschema.Draft7Validator(g34_hzb_unified_gate_schema)
        if g34_hzb_unified_gate_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif (
            f.name.startswith("g34_unified_lane_")
            and g34_unified_lane_validator is not None
        ):
            # G34 全特性合流 G34-1 统一车道 harness 证据（g34_full_lane 真跑腿）→
            # milestones/g34/g34_unified_lane_evidence_schema.json
            # （ci/g34_unified_lane_smoke.py --gate g34.wave1.unified 产）。
            validator = g34_unified_lane_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g34_hzb_unified_gate_")
            and g34_hzb_unified_gate_validator is not None
        ):
            # G34 全特性合流 G34-2 HZB 统一车道门裁决证据 →
            # milestones/g34/g34_hzb_unified_gate_evidence_schema.json
            # （ci/g34_hzb_unified_smoke.py --gate g34.wave2.hzb 产）。仅门裁决件
            # 路由：HZB harness 真跑件（rurix.g34.hzb_unified_evidence.v1）留
            # .tmp 不注册,数字经门裁决件蒸馏登记；baseline 对照腿归档用
            # g34_unified_lane_g34hzb_ 前缀走上方 g34_unified_lane_ 既有路由。
            # 前缀分岔分析：g34_ 族 unified_lane/skin_unified/hzb_unified 首段
            # u/s/h 分岔互不包含,与既有 g31_hzb_wiring_ 族亦互不包含。
            validator = g34_hzb_unified_gate_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # ①②③ 三处插入（幂等：已驻留即跳过插入只做核验；均为锚后纯追加）。
    for name, anchor, add, token in [
        ("load", LOAD_ANCHOR, LOAD_ADD, "g34_hzb_unified_gate_schema = load("),
        ("validator", VALIDATOR_ANCHOR, VALIDATOR_ADD, "g34_hzb_unified_gate_validator = ("),
        ("route", ROUTE_ANCHOR, ROUTE_ADD, 'f.name.startswith("g34_hzb_unified_gate_")'),
    ]:
        n = src.count(token)
        if n == 1:
            print(f"[patch_g34_hzb] {name} 已驻留（幂等跳过插入）")
            continue
        if n != 0:
            print(f"[patch_g34_hzb] FAIL: {name} token 驻留数 {n}（≠0≠1 拒改）",
                  file=sys.stderr)
            return 1
        if src.count(anchor) != 1:
            print(f"[patch_g34_hzb] FAIL: {name} 锚出现 {src.count(anchor)} 次（≠1 拒改）",
                  file=sys.stderr)
            return 1
        src = src.replace(anchor, anchor + add, 1)
        print(f"[patch_g34_hzb] {name} 插入完成")

    # 驻留核验（各 1）+ 既有路由面不动 + 语法编译。
    misses = []
    for name, token in [
        ("load_gate", "g34_hzb_unified_gate_schema = load("),
        ("validator_gate", "g34_hzb_unified_gate_validator = ("),
        ("route_gate", 'f.name.startswith("g34_hzb_unified_gate_")'),
        ("route_unified_gate", 'f.name.startswith("g34_unified_lane_gate_")'),
        ("route_unified", 'f.name.startswith("g34_unified_lane_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    if src.count('f.name.startswith("g31_baseline_")') != 1:
        misses.append("g31_baseline 路由非单例")
    if src.count('f.name.startswith("g31_hzb_wiring_")') != 1:
        misses.append("g31_hzb_wiring 路由非单例（前缀分岔既有面破）")
    if misses:
        for m in misses:
            print(f"[patch_g34_hzb] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    # 立即重读验证驻留（防字节面漂移）。
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    for token in [
        "g34_hzb_unified_gate_schema = load(",
        "g34_hzb_unified_gate_validator = (",
        'f.name.startswith("g34_hzb_unified_gate_")',
    ]:
        if token not in back:
            print(f"[patch_g34_hzb] FAIL: 重读未驻留 {token!r}", file=sys.stderr)
            return 1
    import py_compile
    py_compile.compile(P, doraise=True)
    print("[patch_g34_hzb] PASS：三处纯追加驻留（load/validator/route 仅门裁决件）+ 重读核验 + py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
