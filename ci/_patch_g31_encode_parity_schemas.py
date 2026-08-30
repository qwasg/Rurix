#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G37 W1：check_schemas.py 三处纯追加注册（encode parity 硬门 evidence 路由——
# io.open 补丁法 newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证
# 驻留 + py_compile）。本脚本幂等：已驻留即只做核验（B4
# _patch_g31_texture_schemas.py 同法）。锚 = blocked_probes 族三处块（与
# texture 族锚分离，防多 patch 抢锚）。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    g31_blocked_probes_schema = load(
        ROOT / "milestones/g31/g31_blocked_probes_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G37 W1 encode device-vs-host parity 硬门前缀纯追加（重放幂等面；
    # day_0828 A2b encode_parity_probe 转正，HANDOVER §B.5）
    g31_encode_parity_schema = load(
        ROOT / "milestones/g31/g31_encode_parity_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_blocked_probes_validator = (
        jsonschema.Draft7Validator(g31_blocked_probes_schema)
        if g31_blocked_probes_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g31_encode_parity_validator = (
        jsonschema.Draft7Validator(g31_encode_parity_schema)
        if g31_encode_parity_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif (
            f.name.startswith("g31_blocked_probes_")
            and g31_blocked_probes_validator is not None
        ):
            # G31+ 波 C Task C17 阻塞项新鲜探针门裁决证据 →
            # milestones/g31/g31_blocked_probes_evidence_schema.json
            # （ci/g31_blocked_probes_smoke.py --gate g31.waveC.blockedprobes 产）。
            validator = g31_blocked_probes_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g31_encode_parity_")
            and g31_encode_parity_validator is not None
        ):
            # G37 W1 encode device-vs-host parity 硬门裁决证据 →
            # milestones/g31/g31_encode_parity_evidence_schema.json
            # （ci/g31_encode_parity_smoke.py --gate g31.g37w1.encode_parity 产；
            # 与既有 g31_* 全族前缀互不包含）。
            validator = g31_encode_parity_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # 三处插入（幂等：已驻留即跳过插入只做核验；全部 add 后置于锚）。
    for name, anchor, add, token in [
        ("load", LOAD_ANCHOR, LOAD_ADD, "g31_encode_parity_schema = load("),
        ("validator", VALIDATOR_ANCHOR, VALIDATOR_ADD, "g31_encode_parity_validator = ("),
        ("route", ROUTE_ANCHOR, ROUTE_ADD, 'f.name.startswith("g31_encode_parity_")'),
    ]:
        n = src.count(token)
        if n == 1:
            print(f"[patch_g31_encode_parity] {name} 已驻留（幂等跳过插入）")
            continue
        if n != 0:
            print(f"[patch_g31_encode_parity] FAIL: {name} token 驻留数 {n}（≠0≠1 拒改）", file=sys.stderr)
            return 1
        if src.count(anchor) != 1:
            print(f"[patch_g31_encode_parity] FAIL: {name} 锚出现 {src.count(anchor)} 次（≠1 拒改）", file=sys.stderr)
            return 1
        src = src.replace(anchor, anchor + add, 1)
        print(f"[patch_g31_encode_parity] {name} 插入完成")

    # 驻留核验（各 1）。
    misses = []
    for name, token in [
        ("load", "g31_encode_parity_schema = load("),
        ("validator", "g31_encode_parity_validator = ("),
        ("route", 'f.name.startswith("g31_encode_parity_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    if misses:
        for m in misses:
            print(f"[patch_g31_encode_parity] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    for token in [
        "g31_encode_parity_schema = load(",
        "g31_encode_parity_validator = (",
        'f.name.startswith("g31_encode_parity_")',
    ]:
        if token not in back:
            print(f"[patch_g31_encode_parity] FAIL: 重读未驻留 {token!r}", file=sys.stderr)
            return 1
    import py_compile
    py_compile.compile(P, doraise=True)
    print("[patch_g31_encode_parity] PASS：三处纯追加驻留（load/validator/route）+ 重读核验 + py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
