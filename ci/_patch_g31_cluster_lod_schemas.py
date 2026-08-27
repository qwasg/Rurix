#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G31+ #58 簇 DAG LOD 生产接线门：check_schemas.py 三处纯追加注册
# （io.open 补丁法——newline="" 字节面保全，既有路由 0-byte；打补丁后立即
# 重读验证驻留）。本脚本幂等：已驻留即只做核验（_patch_g31_capability_schemas
# 同法；多 agent 并发同窗同文件改写面下,锚 = 当前在树字节文本,拒改条件逐字）。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    g31_renderer_docs_schema = load(
        ROOT / "milestones/g31/g31_renderer_docs_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G31+ #58 簇 DAG LOD 生产接线门前缀纯追加（重放幂等面；与既有 g31_* 全族
    # 及 gpu fallthrough 互不包含）
    g31_cluster_lod_schema = load(
        ROOT / "milestones/g31/g31_cluster_lod_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_renderer_docs_validator = (
        jsonschema.Draft7Validator(g31_renderer_docs_schema)
        if g31_renderer_docs_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g31_cluster_lod_validator = (
        jsonschema.Draft7Validator(g31_cluster_lod_schema)
        if g31_cluster_lod_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        else:
            validator = gpu_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g31_cluster_lod_")
            and g31_cluster_lod_validator is not None
        ):
            # G31+ #58 簇 DAG LOD 生产接线门裁决证据 →
            # milestones/g31/g31_cluster_lod_evidence_schema.json
            # （ci/g31_cluster_lod_smoke.py --gate g31.wave58.cluster_lod 产）。
            validator = g31_cluster_lod_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    for name, anchor, add, token, before in [
        ("load", LOAD_ANCHOR, LOAD_ADD, "g31_cluster_lod_schema = load(", False),
        ("validator", VALIDATOR_ANCHOR, VALIDATOR_ADD, "g31_cluster_lod_validator = (", False),
        ("route", ROUTE_ANCHOR, ROUTE_ADD, 'f.name.startswith("g31_cluster_lod_")', True),
    ]:
        n = src.count(token)
        if n == 1:
            print(f"[patch_g31_cluster_lod] {name} 已驻留（幂等跳过插入）")
            continue
        if n != 0:
            print(
                f"[patch_g31_cluster_lod] FAIL: {name} token 驻留数 {n}（≠0≠1 拒改）",
                file=sys.stderr,
            )
            return 1
        if src.count(anchor) != 1:
            print(
                f"[patch_g31_cluster_lod] FAIL: {name} 锚出现 {src.count(anchor)} 次（≠1 拒改）",
                file=sys.stderr,
            )
            return 1
        src = src.replace(anchor, (add + anchor) if before else (anchor + add), 1)
        print(f"[patch_g31_cluster_lod] {name} 插入完成")

    misses = []
    for name, token in [
        ("load", "g31_cluster_lod_schema = load("),
        ("validator", "g31_cluster_lod_validator = ("),
        ("route", 'f.name.startswith("g31_cluster_lod_")'),
    ]:
        if src.count(token) != 1:
            misses.append(name)
    if misses:
        print(f"[patch_g31_cluster_lod] FAIL: 驻留核验缺 {misses}", file=sys.stderr)
        return 1
    try:
        compile(src, P, "exec")
    except SyntaxError as e:
        print(f"[patch_g31_cluster_lod] FAIL: 语法编译 {e}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    print("[patch_g31_cluster_lod] OK: 三处注册驻留 + 语法编译通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
