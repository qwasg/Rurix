#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G31+ 波 B Task B2：check_schemas.py 三处纯追加注册（io.open 补丁法——
# newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：已驻留即只做核验；route 面修复「锚后误插」重复链（首版
# patch() helper 的 anchor∉add 分支副作用）为单一合法 elif 链。
import io
import sys

P = r"h:\rurix\ci\check_schemas.py"

LOAD_ADD = '''    # G31+ 波 B Task B2 ReSTIR 车道集成门前缀纯追加（重放幂等面；与既有
    # g31_* 全族及 gpu fallthrough 互不包含）
    g31_restir_wiring_schema = load(
        ROOT / "milestones/g31/g31_restir_wiring_evidence_schema.json"
    )
'''

VALIDATOR_ADD = '''    g31_restir_wiring_validator = (
        jsonschema.Draft7Validator(g31_restir_wiring_schema)
        if g31_restir_wiring_schema is not None
        else None
    )
'''

ROUTE_ADD = '''        elif (
            f.name.startswith("g31_restir_wiring_")
            and g31_restir_wiring_validator is not None
        ):
            # G31+ 波 B Task B2 ReSTIR 高档 reservoir 车道集成证据 →
            # milestones/g31/g31_restir_wiring_evidence_schema.json
            # （ci/g31_restir_wiring_smoke.py --gate g31.waveB.restir 产）。
            validator = g31_restir_wiring_validator
'''

# 修复面：首版 helper 在 route 面走了 anchor+add 误插分支产生的重复链
# （baseline elif → else → gpu → elif baseline → elif restir → else → gpu），
# 收敛为单一链 baseline → restir → else → gpu。
BROKEN_DUP = '''            continue
        else:
            validator = gpu_validator
        elif f.name.startswith("g31_baseline_"):
            # G31 波 A 验收 baseline 快检件（results.trimmed_mean 通用 measured entry，
            # budget_eval eval_entry 通用路消费）——无映射前缀跳过，g19~g30 baseline 同律。
            continue
        elif (
            f.name.startswith("g31_restir_wiring_")'''
FIXED_DUP = '''            continue
        elif (
            f.name.startswith("g31_restir_wiring_")'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # ⓪ 重复链修复（若存在）。
    if BROKEN_DUP in src:
        if src.count(BROKEN_DUP) != 1:
            print(f"[patch_g31_restir] FAIL: 重复链片段出现 {src.count(BROKEN_DUP)} 次（≠1 拒改）",
                  file=sys.stderr)
            return 1
        src = src.replace(BROKEN_DUP, FIXED_DUP, 1)
        print("[patch_g31_restir] route 面重复链修复（baseline→else→gpu 后误插段收敛）")

    # ①②③ 三处驻留核验（本波已打完；缺即 FAIL 不自动补——锚唯一性已人工裁决）。
    misses = []
    for name, token in [
        ("load", "g31_restir_wiring_schema = load("),
        ("validator", "g31_restir_wiring_validator = ("),
        ("route", 'f.name.startswith("g31_restir_wiring_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    if misses:
        for m in misses:
            print(f"[patch_g31_restir] FAIL {m}", file=sys.stderr)
        return 1

    # 合法性：elif 链单一 else 收尾 + 语法编译。
    if src.count('f.name.startswith("g31_baseline_")') != 1:
        print("[patch_g31_restir] FAIL: baseline 路由非单例", file=sys.stderr)
        return 1
    if src.count("validator = gpu_validator") < 1:
        print("[patch_g31_restir] FAIL: gpu fallthrough 缺失", file=sys.stderr)
        return 1
    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    import py_compile
    py_compile.compile(P, doraise=True)
    print("[patch_g31_restir] PASS：三处纯追加驻留（load/validator/route 各 1）+ 链合法 + py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
