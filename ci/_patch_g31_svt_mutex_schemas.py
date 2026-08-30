#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G37 W6 svt mutex_registered：check_schemas.py 三处纯追加注册（SVT 门互斥
# 登记态 evidence 路由——io.open 补丁法 newline="" 字节面保全，既有路由
# 0-byte；打补丁后立即重读验证驻留 + py_compile）。本脚本幂等：已驻留即只做
# 核验（_patch_g31_encode_parity_schemas.py 同法）。锚 = C13 svt 族三处块
# （g31_svt_gate_ load/validator + g31_svt_harness_ 路由——族内锚防多 patch
# 抢锚；新前缀 g31_svt_mutex_registered_ 与 g31_svt_gate_/g31_svt_harness_
# 第九字符分岔 g/h/m 互不包含，与既有 g31_* 全族及 gpu fallthrough 亦互不
# 包含）。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    g31_svt_gate_schema = load(
        ROOT / "milestones/g31/g31_svt_gate_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G37 W6 svt mutex_registered 互斥登记态 schema 纯追加（重放幂等面；
    # ci/g31_svt_smoke.py 真跑四腿前探测 --svt on × day_0828 texel heap
    # fail-closed 互斥字面命中时产，深修归后续波 TODO #33-#36 + day_0828
    # HANDOVER §12）
    g31_svt_mutex_registered_schema = load(
        ROOT / "milestones/g31/g31_svt_mutex_registered_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_svt_gate_validator = (
        jsonschema.Draft7Validator(g31_svt_gate_schema)
        if g31_svt_gate_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g31_svt_mutex_registered_validator = (
        jsonschema.Draft7Validator(g31_svt_mutex_registered_schema)
        if g31_svt_mutex_registered_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif (
            f.name.startswith("g31_svt_harness_")
            and g31_svt_validator is not None
        ):
            # G31+ 波 C Task C13 SVT harness 真跑证据 →
            # milestones/g31/g31_svt_evidence_schema.json。
            validator = g31_svt_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g31_svt_mutex_registered_")
            and g31_svt_mutex_registered_validator is not None
        ):
            # G37 W6 SVT 门互斥登记态证据 →
            # milestones/g31/g31_svt_mutex_registered_schema.json
            # （ci/g31_svt_smoke.py --gate g31.waveC.svt 互斥字面命中时产——
            # 非 PASS 非 FAIL 的登记态件，host 金标准腿全绿前置；前缀与
            # g31_svt_gate_/g31_svt_harness_ 第九字符分岔 g/h/m 互不包含）。
            validator = g31_svt_mutex_registered_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # 三处插入（幂等：已驻留即跳过插入只做核验；全部 add 后置于锚）。
    for name, anchor, add, token in [
        ("load", LOAD_ANCHOR, LOAD_ADD, "g31_svt_mutex_registered_schema = load("),
        ("validator", VALIDATOR_ANCHOR, VALIDATOR_ADD, "g31_svt_mutex_registered_validator = ("),
        ("route", ROUTE_ANCHOR, ROUTE_ADD, 'f.name.startswith("g31_svt_mutex_registered_")'),
    ]:
        n = src.count(token)
        if n == 1:
            print(f"[patch_g31_svt_mutex] {name} 已驻留（幂等跳过插入）")
            continue
        if n != 0:
            print(f"[patch_g31_svt_mutex] FAIL: {name} token 驻留数 {n}（≠0≠1 拒改）", file=sys.stderr)
            return 1
        if src.count(anchor) != 1:
            print(f"[patch_g31_svt_mutex] FAIL: {name} 锚出现 {src.count(anchor)} 次（≠1 拒改）", file=sys.stderr)
            return 1
        src = src.replace(anchor, anchor + add, 1)
        print(f"[patch_g31_svt_mutex] {name} 插入完成")

    # 驻留核验（各 1）。
    misses = []
    for name, token in [
        ("load", "g31_svt_mutex_registered_schema = load("),
        ("validator", "g31_svt_mutex_registered_validator = ("),
        ("route", 'f.name.startswith("g31_svt_mutex_registered_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    if misses:
        for m in misses:
            print(f"[patch_g31_svt_mutex] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    for token in [
        "g31_svt_mutex_registered_schema = load(",
        "g31_svt_mutex_registered_validator = (",
        'f.name.startswith("g31_svt_mutex_registered_")',
    ]:
        if token not in back:
            print(f"[patch_g31_svt_mutex] FAIL: 重读未驻留 {token!r}", file=sys.stderr)
            return 1
    import py_compile
    py_compile.compile(P, doraise=True)
    print("[patch_g31_svt_mutex] PASS：三处纯追加驻留（load/validator/route）+ 重读核验 + py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
