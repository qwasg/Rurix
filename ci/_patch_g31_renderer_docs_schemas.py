#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G31+ 波 C Task C2：check_schemas.py 三处纯追加注册（io.open 补丁法——
# newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：已驻留即只做核验（B2 _patch_g31_restir_schemas.py /
# B4 _patch_g31_texture_schemas.py 同法）。
# 留痕：2026-08-26 本批 Edit 直改曾遭并发回退（同文件多 agent 补丁面），
# 故收敛为幂等补丁脚本随批留存，被覆写时可确定性重放。
import io
import sys

P = r"h:\rurix\ci\check_schemas.py"

LOAD_ANCHOR = '''    g31_texture_sampling_gate_schema = load(
        ROOT / "milestones/g31/g31_texture_sampling_gate_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G31+ 波 C Task C2 渲染器文档与示例门前缀纯追加（重放幂等面；与既有
    # g31_* 全族及 gpu fallthrough 互不包含）
    g31_renderer_docs_schema = load(
        ROOT / "milestones/g31/g31_renderer_docs_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_texture_sampling_gate_validator = (
        jsonschema.Draft7Validator(g31_texture_sampling_gate_schema)
        if g31_texture_sampling_gate_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g31_renderer_docs_validator = (
        jsonschema.Draft7Validator(g31_renderer_docs_schema)
        if g31_renderer_docs_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''            validator = g31_ngx_decomposition_validator
        else:
            validator = gpu_validator
'''
ROUTE_ADD = '''            validator = g31_ngx_decomposition_validator
        elif (
            f.name.startswith("g31_renderer_docs_")
            and g31_renderer_docs_validator is not None
        ):
            # G31+ 波 C Task C2 渲染器文档与示例门证据 →
            # milestones/g31/g31_renderer_docs_evidence_schema.json
            # （ci/g31_renderer_docs_smoke.py --gate g31.waveC.docs 产）。
            validator = g31_renderer_docs_validator
        else:
            validator = gpu_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # ①②③ 三处插入（幂等：已驻留即跳过插入只做核验）。
    for name, anchor, add, token in [
        ("load", LOAD_ANCHOR, LOAD_ADD, "g31_renderer_docs_schema = load("),
        ("validator", VALIDATOR_ANCHOR, VALIDATOR_ADD, "g31_renderer_docs_validator = ("),
        ("route", ROUTE_ANCHOR, ROUTE_ADD, 'f.name.startswith("g31_renderer_docs_")'),
    ]:
        n = src.count(token)
        if n == 1:
            print(f"[patch_g31_renderer_docs] {name} 已驻留（幂等跳过插入）")
            continue
        if n != 0:
            print(f"[patch_g31_renderer_docs] FAIL: {name} token 驻留数 {n}（≠0≠1 拒改）",
                  file=sys.stderr)
            return 1
        if src.count(anchor) != 1:
            print(f"[patch_g31_renderer_docs] FAIL: {name} 锚出现 {src.count(anchor)} 次（≠1 拒改）",
                  file=sys.stderr)
            return 1
        src = src.replace(anchor, anchor + add if name != "route" else add, 1)
        print(f"[patch_g31_renderer_docs] {name} 插入完成")

    # 驻留核验（各 1）+ gpu fallthrough 单例 + 语法编译。
    misses = []
    for name, token in [
        ("load", "g31_renderer_docs_schema = load("),
        ("validator", "g31_renderer_docs_validator = ("),
        ("route", 'f.name.startswith("g31_renderer_docs_")'),
    ]:
        if src.count(token) != 1:
            misses.append(f"{name} 驻留数 {src.count(token)} ≠ 1")
    if src.count("validator = gpu_validator") != 1:
        misses.append("gpu fallthrough 非单例")
    if misses:
        for m in misses:
            print(f"[patch_g31_renderer_docs] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    # 立即重读验证驻留（防字节面漂移/并发回退）。
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    for token in [
        "g31_renderer_docs_schema = load(",
        "g31_renderer_docs_validator = (",
        'f.name.startswith("g31_renderer_docs_")',
    ]:
        if token not in back:
            print(f"[patch_g31_renderer_docs] FAIL: 重读未驻留 {token!r}", file=sys.stderr)
            return 1
    import py_compile
    py_compile.compile(P, doraise=True)
    print("[patch_g31_renderer_docs] PASS：三处纯追加驻留（load/validator/route）+ 重读核验 + py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
