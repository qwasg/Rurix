#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude（G34 全特性合流收口批 G34-3）
# G34 全特性合流 G34-3：check_schemas.py 三处纯追加注册（io.open 补丁法——
# newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：已驻留即只做核验（ci/_patch_g34_unified_lane_schemas.py 同法）。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    g34_unified_lane_schema = load(
        ROOT / "milestones/g34/g34_unified_lane_evidence_schema.json"
    )
    g34_unified_lane_gate_schema = load(
        ROOT / "milestones/g34/g34_unified_lane_gate_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G34 全特性合流 G34-3 蒙皮统一车道门前缀纯追加（重放幂等面；gate 长前缀
    # 先匹配——g34_skin_unified_gate_ 先于 g34_skin_unified_；与 g34_unified_lane_
    # 族首段 s/u 分岔全串互不包含）
    g34_skin_unified_schema = load(
        ROOT / "milestones/g34/g34_skin_unified_evidence_schema.json"
    )
    g34_skin_unified_gate_schema = load(
        ROOT / "milestones/g34/g34_skin_unified_gate_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g34_unified_lane_validator = (
        jsonschema.Draft7Validator(g34_unified_lane_schema)
        if g34_unified_lane_schema is not None
        else None
    )
    g34_unified_lane_gate_validator = (
        jsonschema.Draft7Validator(g34_unified_lane_gate_schema)
        if g34_unified_lane_gate_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g34_skin_unified_validator = (
        jsonschema.Draft7Validator(g34_skin_unified_schema)
        if g34_skin_unified_schema is not None
        else None
    )
    g34_skin_unified_gate_validator = (
        jsonschema.Draft7Validator(g34_skin_unified_gate_schema)
        if g34_skin_unified_gate_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif (
            f.name.startswith("g34_unified_lane_")
            and g34_unified_lane_validator is not None
        ):
            # G34 全特性合流 G34-1 统一车道 harness 证据（g34_full_lane 真跑腿）→
            # milestones/g34/g34_unified_lane_evidence_schema.json
            # （ci/g34_unified_lane_smoke.py --gate g34.wave1.unified 产）。
            validator = g34_unified_lane_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g34_skin_unified_gate_")
            and g34_skin_unified_gate_validator is not None
        ):
            # G34 全特性合流 G34-3 蒙皮统一车道门裁决证据 →
            # milestones/g34/g34_skin_unified_gate_evidence_schema.json
            # （ci/g34_skin_unified_smoke.py --gate g34.wave2.skin 产；长前缀先匹配）。
            validator = g34_skin_unified_gate_validator
        elif (
            f.name.startswith("g34_skin_unified_")
            and g34_skin_unified_validator is not None
        ):
            # G34 全特性合流 G34-3 蒙皮统一车道 harness 证据（g34_full_lane --skin on
            # 真跑腿归档）→ milestones/g34/g34_skin_unified_evidence_schema.json
            # （ci/g34_skin_unified_smoke.py --gate g34.wave2.skin 产；skin 门对照腿
            # baseline/full_noskin 归档用 g34_unified_lane_g34skin_ 前缀走上方
            # g34_unified_lane_ 既有路由——s/u 首段分岔全串互不包含）。
            validator = g34_skin_unified_validator
'''

PROBE = "g34_skin_unified_gate_schema = load"


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        s = f.read()
    if PROBE in s:
        ok = (
            LOAD_ADD in s
            and VALIDATOR_ADD in s
            and ROUTE_ADD in s
        )
        print(f"[patch_g34_skin] 已驻留，核验 {'PASS' if ok else 'FAIL'}")
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
        print(f"[patch_g34_skin] 锚缺失: {missing}", file=sys.stderr)
        return 1
    s = s.replace(LOAD_ANCHOR, LOAD_ANCHOR + LOAD_ADD, 1)
    s = s.replace(VALIDATOR_ANCHOR, VALIDATOR_ANCHOR + VALIDATOR_ADD, 1)
    s = s.replace(ROUTE_ANCHOR, ROUTE_ANCHOR + ROUTE_ADD, 1)
    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(s)
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        s2 = f.read()
    ok = LOAD_ADD in s2 and VALIDATOR_ADD in s2 and ROUTE_ADD in s2
    print(f"[patch_g34_skin] 应用完成，重读核验 {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
