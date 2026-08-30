#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G37 W1：check_schemas.py 三处纯追加注册（texel heap 形态纹理判读器双 schema
# 路由——io.open 补丁法 newline="" 字节面保全，既有路由 0-byte）。加性双形态
# 纪律：旧 g31_texture_sampling_{,gate_} 路由与既有 evidence 0-byte 不动，heap
# 件走新前缀 g31_texture_sampling_heap_{,gate_}。**路由序律**：heap 前缀含
# 既有短前缀（"g31_texture_sampling_heap_…".startswith("g31_texture_sampling_")
# == True）⇒ heap 两路由必须插在既有 texture 两路由**之前**（elif 链长前缀
# 先匹配；heap_gate 又先于 heap）。本脚本幂等：已驻留即只做核验。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    g31_texture_sampling_schema = load(
        ROOT / "milestones/g31/g31_texture_sampling_evidence_schema.json"
    )
    g31_texture_sampling_gate_schema = load(
        ROOT / "milestones/g31/g31_texture_sampling_gate_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G37 W1 texel heap 形态判读器双 schema 纯追加（重放幂等面；加性双形态：
    # 旧形态 schema/路由 0-byte，heap 件走 g31_texture_sampling_heap_* 前缀）
    g31_texture_sampling_heap_schema = load(
        ROOT / "milestones/g31/g31_texture_sampling_heap_evidence_schema.json"
    )
    g31_texture_sampling_heap_gate_schema = load(
        ROOT / "milestones/g31/g31_texture_sampling_heap_gate_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_texture_sampling_gate_validator = (
        jsonschema.Draft7Validator(g31_texture_sampling_gate_schema)
        if g31_texture_sampling_gate_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g31_texture_sampling_heap_validator = (
        jsonschema.Draft7Validator(g31_texture_sampling_heap_schema)
        if g31_texture_sampling_heap_schema is not None
        else None
    )
    g31_texture_sampling_heap_gate_validator = (
        jsonschema.Draft7Validator(g31_texture_sampling_heap_gate_schema)
        if g31_texture_sampling_heap_gate_schema is not None
        else None
    )
'''

# 路由 add 前置于既有 texture gate 路由（长前缀先匹配律：heap_gate → heap →
# 既有 gate → 既有 plain）。
ROUTE_ANCHOR = '''        elif (
            f.name.startswith("g31_texture_sampling_gate_")
            and g31_texture_sampling_gate_validator is not None
        ):
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g31_texture_sampling_heap_gate_")
            and g31_texture_sampling_heap_gate_validator is not None
        ):
            # G37 W1 texel heap 形态纹理采样接线门裁决证据 →
            # milestones/g31/g31_texture_sampling_heap_gate_evidence_schema.json
            # （更新后 ci/g31_texture_sampling_smoke.py --gate g31.waveB.texture 产；
            # heap 前缀含旧前缀 ⇒ 本路由必须先于 g31_texture_sampling_{gate_,} 两路由）。
            validator = g31_texture_sampling_heap_gate_validator
        elif (
            f.name.startswith("g31_texture_sampling_heap_")
            and g31_texture_sampling_heap_validator is not None
        ):
            # G37 W1 texel heap 形态纹理采样 harness 证据（--textures on 腿,v2 SPV）→
            # milestones/g31/g31_texture_sampling_heap_evidence_schema.json
            # （harness 侧 schema/gate 字面沿用 rurix.g31.texture_sampling_evidence.v1 /
            # g31.waveB.texture——heap 形态经文件名前缀路由承载）。
            validator = g31_texture_sampling_heap_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    for name, anchor, add, token, before in [
        ("load", LOAD_ANCHOR, LOAD_ADD, "g31_texture_sampling_heap_schema = load(", False),
        ("validator", VALIDATOR_ANCHOR, VALIDATOR_ADD, "g31_texture_sampling_heap_validator = (", False),
        ("route", ROUTE_ANCHOR, ROUTE_ADD, 'f.name.startswith("g31_texture_sampling_heap_gate_")', True),
    ]:
        n = src.count(token)
        if n == 1:
            print(f"[patch_g31_texture_heap] {name} 已驻留（幂等跳过插入）")
            continue
        if n != 0:
            print(f"[patch_g31_texture_heap] FAIL: {name} token 驻留数 {n}（≠0≠1 拒改）", file=sys.stderr)
            return 1
        if src.count(anchor) != 1:
            print(f"[patch_g31_texture_heap] FAIL: {name} 锚出现 {src.count(anchor)} 次（≠1 拒改）", file=sys.stderr)
            return 1
        src = src.replace(anchor, (add + anchor) if before else (anchor + add), 1)
        print(f"[patch_g31_texture_heap] {name} 插入完成")

    # 驻留核验（各 1）+ 路由序律核验（heap_gate < heap < 既有 gate < 既有 plain）。
    misses = []
    for name, token in [
        ("load", "g31_texture_sampling_heap_schema = load("),
        ("load_gate", "g31_texture_sampling_heap_gate_schema = load("),
        ("validator", "g31_texture_sampling_heap_validator = ("),
        ("validator_gate", "g31_texture_sampling_heap_gate_validator = ("),
        ("route_heap_gate", 'f.name.startswith("g31_texture_sampling_heap_gate_")'),
        ("route_heap", 'f.name.startswith("g31_texture_sampling_heap_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    i_hg = src.find('f.name.startswith("g31_texture_sampling_heap_gate_")')
    i_h = src.find('f.name.startswith("g31_texture_sampling_heap_")')
    i_g = src.find('f.name.startswith("g31_texture_sampling_gate_")')
    i_p = src.find('f.name.startswith("g31_texture_sampling_") ')
    if i_p < 0:
        # 既有 plain 路由行尾无空格差异面：退化为 gate 之后的下一次出现。
        i_p = src.find('f.name.startswith("g31_texture_sampling_")', i_g + 1)
    if not (0 <= i_hg < i_h < i_g < i_p):
        misses.append(f"路由序律破: heap_gate={i_hg} heap={i_h} gate={i_g} plain={i_p}（须严格递增）")
    if misses:
        for m in misses:
            print(f"[patch_g31_texture_heap] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    for token in [
        "g31_texture_sampling_heap_schema = load(",
        "g31_texture_sampling_heap_gate_validator = (",
        'f.name.startswith("g31_texture_sampling_heap_gate_")',
    ]:
        if token not in back:
            print(f"[patch_g31_texture_heap] FAIL: 重读未驻留 {token!r}", file=sys.stderr)
            return 1
    import py_compile
    py_compile.compile(P, doraise=True)
    print("[patch_g31_texture_heap] PASS：三处纯追加驻留（load/validator/route×2）+ 路由序律 + 重读核验 + py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
