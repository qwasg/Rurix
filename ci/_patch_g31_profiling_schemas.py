#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G31+ 波 C Task C7：check_schemas.py 三处纯追加注册（io.open 补丁法——
# newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：已驻留即只做核验（C3 _patch_g31_capability_schemas.py 同法；
# 多 agent 并发同窗同文件改写面下,锚 = 当前在树字节文本,拒改条件逐字）。
import io
import sys

P = r"h:\rurix\ci\check_schemas.py"

LOAD_ANCHOR = '''    g31_support_policy_schema = load(
        ROOT / "milestones/g31/g31_support_policy_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G31+ 波 C Task C7 性能剖析与调试工具面前缀纯追加（重放幂等面；与既有
    # g31_* 全族及 gpu fallthrough 互不包含）
    g31_profiling_schema = load(
        ROOT / "milestones/g31/g31_profiling_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_support_policy_validator = (
        jsonschema.Draft7Validator(g31_support_policy_schema)
        if g31_support_policy_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g31_profiling_validator = (
        jsonschema.Draft7Validator(g31_profiling_schema)
        if g31_profiling_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        else:
            validator = gpu_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g31_profiling_")
            and g31_profiling_validator is not None
        ):
            # G31+ 波 C Task C7 性能剖析与调试工具面门裁决证据 →
            # milestones/g31/g31_profiling_evidence_schema.json
            # （ci/g31_profiling_smoke.py --gate g31.waveC.profiling 产）。
            validator = g31_profiling_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # ①②③ 三处插入（幂等：已驻留即跳过插入只做核验；route = add 前置于锚,
    # elif 链单一合法）。
    for name, anchor, add, token, before in [
        ("load", LOAD_ANCHOR, LOAD_ADD, "g31_profiling_schema = load(", False),
        ("validator", VALIDATOR_ANCHOR, VALIDATOR_ADD, "g31_profiling_validator = (", False),
        ("route", ROUTE_ANCHOR, ROUTE_ADD, 'f.name.startswith("g31_profiling_")', True),
    ]:
        n = src.count(token)
        if n == 1:
            print(f"[patch_g31_profiling] {name} 已驻留（幂等跳过插入）")
            continue
        if n != 0:
            print(f"[patch_g31_profiling] FAIL: {name} token 驻留数 {n}（≠0≠1 拒改）",
                  file=sys.stderr)
            return 1
        if src.count(anchor) != 1:
            print(f"[patch_g31_profiling] FAIL: {name} 锚出现 {src.count(anchor)} 次（≠1 拒改）",
                  file=sys.stderr)
            return 1
        src = src.replace(anchor, (add + anchor) if before else (anchor + add), 1)
        print(f"[patch_g31_profiling] {name} 插入完成")

    # 驻留核验（各 1）+ gpu fallthrough 单例 + 语法编译。
    misses = []
    for name, token in [
        ("load", "g31_profiling_schema = load("),
        ("validator", "g31_profiling_validator = ("),
        ("route", 'f.name.startswith("g31_profiling_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    if src.count("            validator = gpu_validator") != 1:
        misses.append("gpu fallthrough 非单例")
    if misses:
        for m in misses:
            print(f"[patch_g31_profiling] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    # 立即重读验证驻留（防字节面漂移;并发同窗改写面下重读为最终裁决）。
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    for token in [
        "g31_profiling_schema = load(",
        "g31_profiling_validator = (",
        'f.name.startswith("g31_profiling_")',
    ]:
        if token not in back:
            print(f"[patch_g31_profiling] FAIL: 重读未驻留 {token!r}", file=sys.stderr)
            return 1
    import py_compile
    py_compile.compile(P, doraise=True)
    print("[patch_g31_profiling] PASS：三处纯追加驻留（load/validator/route）+ 重读核验 + py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
