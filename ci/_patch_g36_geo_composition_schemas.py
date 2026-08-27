#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude(G36 全特性合流 W1-W4 互斥项修复)
# G36 W1-W4:check_schemas.py 三处纯追加注册(io.open 补丁法——newline=""
# 字节面保全,既有路由 0-byte;打补丁后立即重读验证驻留)。
# 本脚本幂等:已驻留即只做核验(ci/_patch_g35_sort_oit_schemas.py 同法)。
#
# 前缀分岔分析(注册前缀 "g36_geo_composition_gate_"):g36_ 族全仓首件——
# 与 g19_~g30_ 元组(第 2 字符分岔)、g31_/g34_/g35_ 各族(第 3 字符 6 ≠
# 1/4/5)及 gpu fallthrough(首字符 g 后 3 ≠ p)全串互不包含,既有路由 0-byte。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    # G31+ 波 C Task C3 设备兼容矩阵与能力降级链门前缀纯追加（重放幂等面；与既有
    # g31_* 全族及 gpu fallthrough 互不包含）
    g31_capability_fallback_schema = load(
        ROOT / "milestones/g31/g31_capability_fallback_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G36 W1-W4 互斥项修复（geo 组合面）门前缀纯追加（重放幂等面；仅门裁决件
    # 注册——lane 真跑件（rurix.g36.unified_geo_evidence.v1 / g35 lane run）留
    # .tmp 不注册，数字经门裁决件蒸馏登记；前缀 g36_geo_composition_gate_ 为
    # 全仓唯一 g36_ 族首件,与 g19_~g35_ 各族及 gpu fallthrough 全串互不包含）
    g36_geo_composition_gate_schema = load(
        ROOT / "milestones/g36/g36_geo_composition_gate_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_wp_hlod_validator = (
        jsonschema.Draft7Validator(g31_wp_hlod_schema)
        if g31_wp_hlod_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g36_geo_composition_gate_validator = (
        jsonschema.Draft7Validator(g36_geo_composition_gate_schema)
        if g36_geo_composition_gate_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif (
            f.name.startswith("g31_wp_hlod_")
            and g31_wp_hlod_validator is not None
        ):
            # G31+ #95/#68/#99 WP cell + HLOD 生产接线门裁决证据 →
            # milestones/g31/g31_wp_hlod_evidence_schema.json
            # （ci/g31_wp_hlod_smoke.py --gate g31.wave95.wp_hlod 产）。
            validator = g31_wp_hlod_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g36_geo_composition_gate_")
            and g36_geo_composition_gate_validator is not None
        ):
            # G36 W1-W4 互斥项修复（geo 组合面）门裁决证据 →
            # milestones/g36/g36_geo_composition_gate_evidence_schema.json
            # （ci/g36_geo_composition_smoke.py --gate g36.wave1.geo_composition
            # 产；lane 真跑件〔rurix.g36.unified_geo_evidence.v1 与 g35 lane
            # run〕留 .tmp 不入 evidence/ 不注册；前缀分岔分析：g36_ 族全仓
            # 首件,与 g19_~g35_ 各族及 gpu fallthrough 全串互不包含）。
            validator = g36_geo_composition_gate_validator
'''

PROBE = "g36_geo_composition_gate_schema = load"


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        s = f.read()
    if PROBE in s:
        ok = LOAD_ADD in s and VALIDATOR_ADD in s and ROUTE_ADD in s
        print(f"[patch_g36_geo_composition] 已驻留，核验 {'PASS' if ok else 'FAIL'}")
        return 0 if ok else 1
    missing = [
        name
        for name, anchor in (
            ("LOAD_ANCHOR", LOAD_ANCHOR),
            ("VALIDATOR_ANCHOR", VALIDATOR_ANCHOR),
            ("ROUTE_ANCHOR", ROUTE_ANCHOR),
        )
        if anchor not in s
    ]
    if missing:
        print(f"[patch_g36_geo_composition] 锚缺失: {missing}", file=sys.stderr)
        return 1
    s = s.replace(LOAD_ANCHOR, LOAD_ADD + LOAD_ANCHOR, 1)
    s = s.replace(VALIDATOR_ANCHOR, VALIDATOR_ANCHOR + VALIDATOR_ADD, 1)
    s = s.replace(ROUTE_ANCHOR, ROUTE_ANCHOR + ROUTE_ADD, 1)
    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(s)
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        s2 = f.read()
    ok = LOAD_ADD in s2 and VALIDATOR_ADD in s2 and ROUTE_ADD in s2
    print(f"[patch_g36_geo_composition] 应用完成，重读核验 {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
