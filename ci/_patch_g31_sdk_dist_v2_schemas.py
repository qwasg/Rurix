#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G37 W5:check_schemas.py 三处纯追加注册 SDK 分发门 v2 schema(io.open 补丁法——
# newline="" 字节面保全,既有 v1 路由 0-byte;打补丁后立即重读验证驻留 + 路由序)。
# v2 evidence 前缀 g31_sdk_dist_v2_ 含 v1 前缀 g31_sdk_dist_ ⇒ route 分支必须
# 插在 v1 分支**之前**(先例 = g31_texture_sampling_heap_ 系先于 g31_texture_sampling_)。
# 本脚本幂等:已驻留即只做核验(锚唯一性机核,锚不唯一/缺失即 FAIL 拒改不猜)。
# 先例 ci/_patch_g31_sdk_dist_schemas.py。
import io
import sys

P = r"h:\rurix\ci\check_schemas.py"

LOAD_ANCHOR = '''    g31_sdk_dist_schema = load(
        ROOT / "milestones/g31/g31_sdk_dist_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G37 W5 渲染器 SDK 分发打包门 v2 schema 纯追加(重放幂等面;16→24 组件
    # 冻结面版本化,v1 schema/路由 0-byte,v2 件走 g31_sdk_dist_v2_ 前缀)
    g31_sdk_dist_v2_schema = load(
        ROOT / "milestones/g31/g31_sdk_dist_v2_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_sdk_dist_validator = (
        jsonschema.Draft7Validator(g31_sdk_dist_schema)
        if g31_sdk_dist_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g31_sdk_dist_v2_validator = (
        jsonschema.Draft7Validator(g31_sdk_dist_v2_schema)
        if g31_sdk_dist_v2_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif (
            f.name.startswith("g31_sdk_dist_")
            and g31_sdk_dist_validator is not None
        ):
            # G31+ 波 C Task C5 渲染器 SDK 分发打包门证据 →
            # milestones/g31/g31_sdk_dist_evidence_schema.json
            # （ci/g31_sdk_dist_smoke.py --gate g31.waveC.dist 产）。
            validator = g31_sdk_dist_validator
'''
# route 插在 v1 锚**之前**(v2 前缀含 v1 前缀,长前缀先匹配)。
ROUTE_ADD = '''        elif (
            f.name.startswith("g31_sdk_dist_v2_")
            and g31_sdk_dist_v2_validator is not None
        ):
            # G37 W5 渲染器 SDK 分发打包门证据 v2(16→24 组件冻结面版本化)→
            # milestones/g31/g31_sdk_dist_v2_evidence_schema.json
            # (升级后 ci/g31_sdk_dist_smoke.py --gate g31.g37w5.dist 产;
            # v2 前缀含 v1 前缀 ⇒ 本路由必须先于 g31_sdk_dist_ 路由)。
            validator = g31_sdk_dist_v2_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # 幂等面:三处均已驻留 → 只核验不重复插。
    already = all(
        token in src
        for token in (
            "g31_sdk_dist_v2_schema = load(",
            "g31_sdk_dist_v2_validator = (",
            'f.name.startswith("g31_sdk_dist_v2_")',
        )
    )
    if not already:
        # 锚唯一性机核(并发改动下锚不唯一/缺失即拒改)。
        for name, anchor in [
            ("load", LOAD_ANCHOR),
            ("validator", VALIDATOR_ANCHOR),
            ("route", ROUTE_ANCHOR),
        ]:
            n = src.count(anchor)
            if n != 1:
                print(f"[patch_g31_sdk_dist_v2] FAIL: {name} 锚出现 {n} 次（≠1 拒改）",
                      file=sys.stderr)
                return 1
        src = src.replace(LOAD_ANCHOR, LOAD_ANCHOR + LOAD_ADD, 1)
        src = src.replace(VALIDATOR_ANCHOR, VALIDATOR_ANCHOR + VALIDATOR_ADD, 1)
        # route 前插(长前缀 v2 分支先于 v1 分支)。
        src = src.replace(ROUTE_ANCHOR, ROUTE_ADD + ROUTE_ANCHOR, 1)
        print("[patch_g31_sdk_dist_v2] 三处纯追加插入(load/validator 锚后 + route 锚前各 1)")

    # 驻留核验(每 token 恰 1)。
    misses = []
    for name, token in [
        ("load", "g31_sdk_dist_v2_schema = load("),
        ("validator", "g31_sdk_dist_v2_validator = ("),
        ("route", 'f.name.startswith("g31_sdk_dist_v2_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    # 路由序核验:v2 分支必须先于 v1 分支(前缀包含,后置必被 v1 吞)。
    v2_at = src.find('f.name.startswith("g31_sdk_dist_v2_")')
    v1_at = src.find('f.name.startswith("g31_sdk_dist_")')
    if not (0 <= v2_at < v1_at):
        misses.append(f"路由序破缺:v2@{v2_at} 未先于 v1@{v1_at}")
    if misses:
        for m in misses:
            print(f"[patch_g31_sdk_dist_v2] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    import py_compile

    py_compile.compile(P, doraise=True)
    print("[patch_g31_sdk_dist_v2] PASS:三处纯追加驻留(load/validator/route 各 1,"
          "v2 路由先于 v1)+ py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
