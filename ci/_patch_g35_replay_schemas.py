#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude（G35 GPU 粒子系统 G35-9 确定性回放/回滚）
# G35 GPU 粒子系统 G35-9：check_schemas.py 三处纯追加注册（io.open 补丁法——
# newline="" 字节面保全，既有路由 0-byte；打补丁后立即重读验证驻留）。
# 本脚本幂等：已驻留即只做核验（ci/_patch_g35_particle_core_schemas.py 同法）。
#
# 前缀分岔注意（本波特有）：g35_replay_ 与同族 g35_render_ 共享 "g35_re"
# 前缀后于第 7 字符 n/p 分岔（render/replay），互不为对方前缀——startswith
# 路由两键互不包含无遮蔽；与 g35_particle_core_/g35_primitives_/g35_fluids_/
# g35_events_/g35_collision_ 在 g35_ 后首段分岔，与既有 g19_~g30_ 元组/
# g31_/g34_ 各族及 gpu fallthrough 全串互不包含。锚 = g35_collision_ 三块
# （当前链尾）；若并行波（W3 g35_render_ 等）先行落锚同处，锚文本仍在，
# replace(ANCHOR, ANCHOR+ADD, 1) 插在锚后不互斥。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    g35_collision_gate_schema = load(
        ROOT / "milestones/g35/g35_collision_gate_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G35 GPU 粒子系统 G35-9 确定性回放/回滚门前缀纯追加（重放幂等面；仅门
    # 裁决件注册——probe 真跑件〔rurix.g35.replay_record.v1/replay_replay.v1/
    # replay_rollback.v1/红臂 rurix.g35.replay_red_arm.v1〕与 journal/digest
    # 链/检查点文件留 .tmp 不注册，数字经门裁决件蒸馏登记；前缀分岔注意：
    # g35_replay_ 与同族 g35_render_ 共享 "g35_re" 后于 n/p 分岔（render/
    # replay）互不为对方前缀；与 g35_particle_core_/g35_primitives_/
    # g35_fluids_/g35_events_/g35_collision_ 首段分岔及 g19_~g30_ 元组/
    # g31_/g34_ 各族与 gpu fallthrough 全串互不包含）
    g35_replay_gate_schema = load(
        ROOT / "milestones/g35/g35_replay_gate_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g35_collision_gate_validator = (
        jsonschema.Draft7Validator(g35_collision_gate_schema)
        if g35_collision_gate_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g35_replay_gate_validator = (
        jsonschema.Draft7Validator(g35_replay_gate_schema)
        if g35_replay_gate_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''            validator = g35_collision_gate_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g35_replay_")
            and g35_replay_gate_validator is not None
        ):
            # G35 GPU 粒子系统 G35-9 确定性回放/回滚门裁决证据（本门全位级：
            # 整数/digest 域零容差，无 f32 budget 条目——不产 g35_budget
            # 标定件）→ milestones/g35/g35_replay_gate_evidence_schema.json
            # （ci/g35_replay_smoke.py --gate g35.wave9.replay 产；probe 真跑
            # 件〔record/replay/rollback/红臂四腿〕与 journal/digest 链/检查
            # 点文件留 .tmp 不入 evidence/ 不注册；前缀分岔分析：g35_replay_
            # 与同族 g35_render_ 共享 "g35_re" 后于 n/p 分岔（render/replay）
            # 互不包含，与 g35_particle_core_/g35_primitives_/g35_fluids_/
            # g35_events_/g35_collision_ 首段分岔及 g19_~g30_ 元组/g31_/g34_
            # 各族全串互不包含）。
            validator = g35_replay_gate_validator
'''

PROBE = "g35_replay_gate_schema = load"


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        s = f.read()
    if PROBE in s:
        ok = (
            LOAD_ADD in s
            and VALIDATOR_ADD in s
            and ROUTE_ADD in s
        )
        print(f"[patch_g35_replay] 已驻留，核验 {'PASS' if ok else 'FAIL'}")
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
        print(f"[patch_g35_replay] 锚缺失: {missing}", file=sys.stderr)
        return 1
    s = s.replace(LOAD_ANCHOR, LOAD_ANCHOR + LOAD_ADD, 1)
    s = s.replace(VALIDATOR_ANCHOR, VALIDATOR_ANCHOR + VALIDATOR_ADD, 1)
    s = s.replace(ROUTE_ANCHOR, ROUTE_ANCHOR + ROUTE_ADD, 1)
    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(s)
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        s2 = f.read()
    ok = LOAD_ADD in s2 and VALIDATOR_ADD in s2 and ROUTE_ADD in s2
    print(f"[patch_g35_replay] 应用完成，重读核验 {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
