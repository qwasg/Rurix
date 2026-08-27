#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude（G35 GPU 粒子系统 G35-5 碰撞与力场）
# G35 GPU 粒子系统 G35-5：check_schemas.py 三处纯追加注册（io.open 补丁法——
# newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：已驻留即只做核验（ci/_patch_g35_particle_core_schemas.py 同法）。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    g35_primitives_gate_schema = load(
        ROOT / "milestones/g35/g35_primitives_gate_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G35 GPU 粒子系统 G35-5 碰撞与力场门前缀纯追加（重放幂等面；仅门裁决
    # 件注册——probe 真跑件〔rurix.g35.collision_probe.v1〕留 .tmp 不注册，
    # 数字经门裁决件蒸馏登记；前缀 g35_collision_ 与既有 g35_particle_core_/
    # g35_primitives_ 同族及 g19_~g30_ 元组/g31_/g34_ 各族与 gpu fallthrough
    # 全串互不包含）
    g35_collision_gate_schema = load(
        ROOT / "milestones/g35/g35_collision_gate_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g35_primitives_gate_validator = (
        jsonschema.Draft7Validator(g35_primitives_gate_schema)
        if g35_primitives_gate_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g35_collision_gate_validator = (
        jsonschema.Draft7Validator(g35_collision_gate_schema)
        if g35_collision_gate_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif (
            f.name.startswith("g35_primitives_")
            and g35_primitives_gate_validator is not None
        ):
            # G35 GPU 粒子系统 G35-1 基元库门裁决证据 →
            # milestones/g35/g35_primitives_gate_evidence_schema.json
            # （ci/g35_primitives_smoke.py --gate g35.wave1.primitives 产）。仅门
            # 裁决件路由：probe 真跑件（rurix.g35.primitives_probe.v1）留 .tmp
            # 不注册,数字经门裁决件蒸馏登记。前缀分岔分析：g35_primitives_ 与
            # 既有 g34_ 族（unified_lane/skin_unified/hzb_unified）及 g31_* 全族
            # 首段分岔互不包含,与 gpu fallthrough 亦互不包含。
            validator = g35_primitives_gate_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g35_collision_")
            and g35_collision_gate_validator is not None
        ):
            # G35 GPU 粒子系统 G35-5 碰撞与力场门裁决证据（含标定纪元件——
            # g35_budget g35.collision.parity_p100 evidence_file 指向门裁决件
            # results.trimmed_mean 镜像槽，budget_eval 通用路消费）→
            # milestones/g35/g35_collision_gate_evidence_schema.json
            # （ci/g35_collision_smoke.py --gate g35.wave5.collision 产；probe
            # 真跑件〔rurix.g35.collision_probe.v1〕留 .tmp 不入 evidence/ 不
            # 注册；前缀 g35_collision_ 与 g35_particle_core_/g35_primitives_
            # 同族及 g19_~g30_ 元组/g31_/g34_ 各族全串互不包含）。
            validator = g35_collision_gate_validator
'''

PROBE = "g35_collision_gate_schema = load"


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        s = f.read()
    if PROBE in s:
        ok = (
            LOAD_ADD in s
            and VALIDATOR_ADD in s
            and ROUTE_ADD in s
        )
        print(f"[patch_g35_collision] 已驻留，核验 {'PASS' if ok else 'FAIL'}")
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
        print(f"[patch_g35_collision] 锚缺失: {missing}", file=sys.stderr)
        return 1
    s = s.replace(LOAD_ANCHOR, LOAD_ANCHOR + LOAD_ADD, 1)
    s = s.replace(VALIDATOR_ANCHOR, VALIDATOR_ANCHOR + VALIDATOR_ADD, 1)
    s = s.replace(ROUTE_ANCHOR, ROUTE_ANCHOR + ROUTE_ADD, 1)
    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(s)
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        s2 = f.read()
    ok = LOAD_ADD in s2 and VALIDATOR_ADD in s2 and ROUTE_ADD in s2
    print(f"[patch_g35_collision] 应用完成，重读核验 {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
