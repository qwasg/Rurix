#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G34 全特性合流 G34-1：check_schemas.py 三处纯追加注册（io.open 补丁法——
# newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：已驻留即只做核验（ci/_patch_g31_texture_schemas.py 同法）。
import io
import sys

P = r"h:\rurix\ci\check_schemas.py"

LOAD_ANCHOR = '''    g31_texture_sampling_gate_schema = load(
        ROOT / "milestones/g31/g31_texture_sampling_gate_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G34 全特性合流 G34-1 统一车道门前缀纯追加（重放幂等面；gate 长前缀
    # 先匹配——g34_unified_lane_gate_ 先于 g34_unified_lane_）
    g34_unified_lane_schema = load(
        ROOT / "milestones/g34/g34_unified_lane_evidence_schema.json"
    )
    g34_unified_lane_gate_schema = load(
        ROOT / "milestones/g34/g34_unified_lane_gate_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_texture_sampling_gate_validator = (
        jsonschema.Draft7Validator(g31_texture_sampling_gate_schema)
        if g31_texture_sampling_gate_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g34_unified_lane_validator = (
        jsonschema.Draft7Validator(g34_unified_lane_schema)
        if g34_unified_lane_schema is not None
        else None
    )
    g34_unified_lane_gate_validator = (
        jsonschema.Draft7Validator(g34_unified_lane_gate_schema)
        if g34_unified_lane_gate_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif f.name.startswith("g31_baseline_"):
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g34_unified_lane_gate_")
            and g34_unified_lane_gate_validator is not None
        ):
            # G34 全特性合流 G34-1 统一车道门裁决证据 →
            # milestones/g34/g34_unified_lane_gate_evidence_schema.json
            # （ci/g34_unified_lane_smoke.py --gate g34.wave1.unified 产；长前缀先匹配）。
            validator = g34_unified_lane_gate_validator
        elif (
            f.name.startswith("g34_unified_lane_")
            and g34_unified_lane_validator is not None
        ):
            # G34 全特性合流 G34-1 统一车道 harness 证据（g34_full_lane 真跑腿）→
            # milestones/g34/g34_unified_lane_evidence_schema.json
            # （ci/g34_unified_lane_smoke.py --gate g34.wave1.unified 产）。
            validator = g34_unified_lane_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # ①②③ 三处插入（幂等：已驻留即跳过插入只做核验；route = add 前置于锚）。
    for name, anchor, add, token, before in [
        ("load", LOAD_ANCHOR, LOAD_ADD, "g34_unified_lane_schema = load(", False),
        ("validator", VALIDATOR_ANCHOR, VALIDATOR_ADD, "g34_unified_lane_validator = (", False),
        ("route", ROUTE_ANCHOR, ROUTE_ADD, 'f.name.startswith("g34_unified_lane_gate_")', True),
    ]:
        n = src.count(token)
        if n == 1:
            print(f"[patch_g34_unified] {name} 已驻留（幂等跳过插入）")
            continue
        if n != 0:
            print(f"[patch_g34_unified] FAIL: {name} token 驻留数 {n}（≠0≠1 拒改）",
                  file=sys.stderr)
            return 1
        if src.count(anchor) != 1:
            print(f"[patch_g34_unified] FAIL: {name} 锚出现 {src.count(anchor)} 次（≠1 拒改）",
                  file=sys.stderr)
            return 1
        src = src.replace(anchor, (add + anchor) if before else (anchor + add), 1)
        print(f"[patch_g34_unified] {name} 插入完成")

    # 驻留核验（各 1）+ 链合法 + 语法编译。
    misses = []
    for name, token in [
        ("load", "g34_unified_lane_schema = load("),
        ("load_gate", "g34_unified_lane_gate_schema = load("),
        ("validator", "g34_unified_lane_validator = ("),
        ("validator_gate", "g34_unified_lane_gate_validator = ("),
        ("route_gate", 'f.name.startswith("g34_unified_lane_gate_")'),
        ("route", 'f.name.startswith("g34_unified_lane_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    if src.count('f.name.startswith("g31_baseline_")') != 1:
        misses.append("g31_baseline 路由非单例")
    if misses:
        for m in misses:
            print(f"[patch_g34_unified] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    # 立即重读验证驻留（防字节面漂移）。
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    for token in [
        "g34_unified_lane_schema = load(",
        "g34_unified_lane_gate_validator = (",
        'f.name.startswith("g34_unified_lane_gate_")',
    ]:
        if token not in back:
            print(f"[patch_g34_unified] FAIL: 重读未驻留 {token!r}", file=sys.stderr)
            return 1
    import py_compile
    py_compile.compile(P, doraise=True)
    print("[patch_g34_unified] PASS：三处纯追加驻留（load/validator/route）+ 重读核验 + py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
