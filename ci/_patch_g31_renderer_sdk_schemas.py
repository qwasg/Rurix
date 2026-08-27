#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G31+ 波 C Task C1：check_schemas.py 三处纯追加注册（io.open 补丁法——
# newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：已驻留即只做核验（多 agent 并发改该文件——锚唯一性机核，
# 锚不唯一/缺失即 FAIL 拒改不猜）。先例 ci/_patch_g31_restir_schemas.py。
import io
import sys

P = r"h:\rurix\ci\check_schemas.py"

LOAD_ANCHOR = '''    g31_ngx_decomposition_schema = load(
        ROOT / "milestones/g31/g31_ngx_decomposition_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G31+ 波 C Task C1 渲染器 SDK 稳定 API 面前缀纯追加（重放幂等面；与既有
    # g31_* 全族及 gpu fallthrough 互不包含）
    g31_renderer_sdk_schema = load(
        ROOT / "milestones/g31/g31_renderer_sdk_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_ngx_decomposition_validator = (
        jsonschema.Draft7Validator(g31_ngx_decomposition_schema)
        if g31_ngx_decomposition_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g31_renderer_sdk_validator = (
        jsonschema.Draft7Validator(g31_renderer_sdk_schema)
        if g31_renderer_sdk_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif (
            f.name.startswith("g31_ngx_decomposition_")
            and g31_ngx_decomposition_validator is not None
        ):
            # G31+ 波 C Task C9 NGX 分解 profiling 证据 →
            # milestones/g31/g31_ngx_decomposition_evidence_schema.json
            # （ci/g31_ngx_decomposition_smoke.py --gate g31.waveC.ngx_decomp 产）。
            validator = g31_ngx_decomposition_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g31_renderer_sdk_")
            and g31_renderer_sdk_validator is not None
        ):
            # G31+ 波 C Task C1 渲染器 SDK 稳定 API 面证据 →
            # milestones/g31/g31_renderer_sdk_evidence_schema.json
            # （ci/g31_renderer_sdk_smoke.py --gate g31.waveC.sdk 产）。
            validator = g31_renderer_sdk_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # 幂等面：三处均已驻留 → 只核验不重复插。
    already = all(
        token in src
        for token in (
            "g31_renderer_sdk_schema = load(",
            "g31_renderer_sdk_validator = (",
            'f.name.startswith("g31_renderer_sdk_")',
        )
    )
    if not already:
        # 锚唯一性机核（并发改动下锚不唯一/缺失即拒改）。
        for name, anchor in [
            ("load", LOAD_ANCHOR),
            ("validator", VALIDATOR_ANCHOR),
            ("route", ROUTE_ANCHOR),
        ]:
            n = src.count(anchor)
            if n != 1:
                print(f"[patch_g31_renderer_sdk] FAIL: {name} 锚出现 {n} 次（≠1 拒改）",
                      file=sys.stderr)
                return 1
        src = src.replace(LOAD_ANCHOR, LOAD_ANCHOR + LOAD_ADD, 1)
        src = src.replace(VALIDATOR_ANCHOR, VALIDATOR_ANCHOR + VALIDATOR_ADD, 1)
        src = src.replace(ROUTE_ANCHOR, ROUTE_ANCHOR + ROUTE_ADD, 1)
        print("[patch_g31_renderer_sdk] 三处纯追加插入（load/validator/route 各 1）")

    # 驻留核验（每 token 恰 1）。
    misses = []
    for name, token in [
        ("load", "g31_renderer_sdk_schema = load("),
        ("validator", "g31_renderer_sdk_validator = ("),
        ("route", 'f.name.startswith("g31_renderer_sdk_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    if misses:
        for m in misses:
            print(f"[patch_g31_renderer_sdk] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    import py_compile

    py_compile.compile(P, doraise=True)
    print("[patch_g31_renderer_sdk] PASS：三处纯追加驻留（load/validator/route 各 1）+ py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
