#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G31+ 波 B Task B4：check_schemas.py 三处纯追加注册（io.open 补丁法——
# newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：已驻留即只做核验（B2 _patch_g31_restir_schemas.py 同法）。
import io
import sys

P = r"h:\rurix\ci\check_schemas.py"

LOAD_ANCHOR = '''    g31_skinning_wiring_schema = load(
        ROOT / "milestones/g31/g31_skinning_wiring_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G31+ 波 B Task B4 纹理采样管线进生产场景门前缀纯追加（重放幂等面；gate
    # 长前缀先匹配）
    g31_texture_sampling_schema = load(
        ROOT / "milestones/g31/g31_texture_sampling_evidence_schema.json"
    )
    g31_texture_sampling_gate_schema = load(
        ROOT / "milestones/g31/g31_texture_sampling_gate_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_skinning_wiring_validator = (
        jsonschema.Draft7Validator(g31_skinning_wiring_schema)
        if g31_skinning_wiring_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g31_texture_sampling_validator = (
        jsonschema.Draft7Validator(g31_texture_sampling_schema)
        if g31_texture_sampling_schema is not None
        else None
    )
    g31_texture_sampling_gate_validator = (
        jsonschema.Draft7Validator(g31_texture_sampling_gate_schema)
        if g31_texture_sampling_gate_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif f.name.startswith("g31_baseline_"):
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g31_texture_sampling_gate_")
            and g31_texture_sampling_gate_validator is not None
        ):
            # G31+ 波 B Task B4 纹理采样接线门裁决证据 →
            # milestones/g31/g31_texture_sampling_gate_evidence_schema.json
            # （ci/g31_texture_sampling_smoke.py --gate g31.waveB.texture 产；长前缀先匹配）。
            validator = g31_texture_sampling_gate_validator
        elif (
            f.name.startswith("g31_texture_sampling_")
            and g31_texture_sampling_validator is not None
        ):
            # G31+ 波 B Task B4 纹理采样生产接线 harness 证据（--textures on 腿）→
            # milestones/g31/g31_texture_sampling_evidence_schema.json
            # （ci/g31_texture_sampling_smoke.py --gate g31.waveB.texture 产）。
            validator = g31_texture_sampling_validator
'''


# 修复面：首版本脚本 route 面误走 anchor+add 顺序（贴到 baseline elif 行后，
# elif 链破）——收敛为 add 在 baseline elif 前的合法单一链。
BROKEN_ROUTE_HEAD = '''        elif f.name.startswith("g31_baseline_"):
        elif (
            f.name.startswith("g31_texture_sampling_gate_")
'''
FIXED_ROUTE_HEAD = '''        elif (
            f.name.startswith("g31_texture_sampling_gate_")
'''
BROKEN_ROUTE_TAIL = '''            validator = g31_texture_sampling_validator
            # G31 波 A 验收 baseline 快检件'''
FIXED_ROUTE_TAIL = '''            validator = g31_texture_sampling_validator
        elif f.name.startswith("g31_baseline_"):
            # G31 波 A 验收 baseline 快检件'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # ⓪ route 面破链修复（若存在）。
    if BROKEN_ROUTE_HEAD in src:
        if src.count(BROKEN_ROUTE_HEAD) != 1 or src.count(BROKEN_ROUTE_TAIL) != 1:
            print("[patch_g31_texture] FAIL: 破链片段计数异常（拒改）", file=sys.stderr)
            return 1
        src = src.replace(BROKEN_ROUTE_HEAD, FIXED_ROUTE_HEAD, 1)
        src = src.replace(BROKEN_ROUTE_TAIL, FIXED_ROUTE_TAIL, 1)
        print("[patch_g31_texture] route 面破链修复（baseline elif 归位）")

    # ①②③ 三处插入（幂等：已驻留即跳过插入只做核验；route = add 前置于锚）。
    for name, anchor, add, token, before in [
        ("load", LOAD_ANCHOR, LOAD_ADD, "g31_texture_sampling_schema = load(", False),
        ("validator", VALIDATOR_ANCHOR, VALIDATOR_ADD, "g31_texture_sampling_validator = (", False),
        ("route", ROUTE_ANCHOR, ROUTE_ADD, 'f.name.startswith("g31_texture_sampling_gate_")', True),
    ]:
        n = src.count(token)
        if n == 1:
            print(f"[patch_g31_texture] {name} 已驻留（幂等跳过插入）")
            continue
        if n != 0:
            print(f"[patch_g31_texture] FAIL: {name} token 驻留数 {n}（≠0≠1 拒改）",
                  file=sys.stderr)
            return 1
        if src.count(anchor) != 1:
            print(f"[patch_g31_texture] FAIL: {name} 锚出现 {src.count(anchor)} 次（≠1 拒改）",
                  file=sys.stderr)
            return 1
        src = src.replace(anchor, (add + anchor) if before else (anchor + add), 1)
        print(f"[patch_g31_texture] {name} 插入完成")

    # 驻留核验（各 1）+ 链合法 + 语法编译。
    misses = []
    for name, token in [
        ("load", "g31_texture_sampling_schema = load("),
        ("load_gate", "g31_texture_sampling_gate_schema = load("),
        ("validator", "g31_texture_sampling_validator = ("),
        ("validator_gate", "g31_texture_sampling_gate_validator = ("),
        ("route_gate", 'f.name.startswith("g31_texture_sampling_gate_")'),
        ("route", 'f.name.startswith("g31_texture_sampling_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    if src.count('f.name.startswith("g31_baseline_")') != 1:
        misses.append("baseline 路由非单例")
    if misses:
        for m in misses:
            print(f"[patch_g31_texture] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    # 立即重读验证驻留（防字节面漂移）。
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    for token in [
        "g31_texture_sampling_schema = load(",
        "g31_texture_sampling_gate_validator = (",
        'f.name.startswith("g31_texture_sampling_gate_")',
    ]:
        if token not in back:
            print(f"[patch_g31_texture] FAIL: 重读未驻留 {token!r}", file=sys.stderr)
            return 1
    import py_compile
    py_compile.compile(P, doraise=True)
    print("[patch_g31_texture] PASS：三处纯追加驻留（load/validator/route）+ 重读核验 + py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
