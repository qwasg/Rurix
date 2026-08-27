#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: cursor:claude-fable-5(G35 GPU 粒子系统 G35-4 半透明双臂)
# G35 GPU 粒子系统 G35-4:check_schemas.py 三处纯追加注册(io.open 补丁法——
# newline="" 字节面保全,既有路由 0-byte;打补丁后立即重读验证驻留)。
# 本脚本幂等:已驻留即只做核验(ci/_patch_g35_render_schemas.py 同法)。
#
# 前缀分岔分析(注册前缀 "g35_sort_oit_"):族内九支 = g35_primitives_ /
# g35_particle_core_ / g35_render_ / g35_sort_oit_ / g35_collision_ /
# g35_events_ / g35_fluids_ / g35_authoring_ / g35_replay_(CI_GATES.md §
# 前缀分岔分析字面)。g35_sort_oit_ 首段 "sort_oit" 与其余八支首段两两全串
# 互不包含——注意:**族内无 "g35_sort_" 独立注册前缀**(W1 排序基元门前缀
# = g35_primitives_,evidence 名 g35_primitives_gate_*),故 g35_sort_oit_
# 不遮蔽亦不被遮蔽;与 g35_render_/g35_replay_ 在 g35_ 后首字符分岔
# (s ≠ r),与 g19_~g30_ 元组/g31_/g34_ 各族(第 3 字符 5 ≠ 1/4)及 gpu
# fallthrough 全串互不包含,既有路由 0-byte。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    g35_replay_gate_schema = load(
        ROOT / "milestones/g35/g35_replay_gate_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G35 GPU 粒子系统 G35-4 半透明双臂门前缀纯追加（重放幂等面；仅门裁决件
    # 注册——lane 真跑件（rurix.g35.particle_lane_run.v1）留 .tmp 不注册，
    # 数字经门裁决件蒸馏登记；前缀 g35_sort_oit_ 首段 sort_oit 与族内其余
    # 八支（primitives/particle_core/render/collision/events/fluids/
    # authoring/replay）两两互不包含——族内无 g35_sort_ 独立前缀（W1 排序
    # 基元门 = g35_primitives_）无遮蔽；与 g35_render_/g35_replay_ 在 g35_
    # 后 s≠r 分岔，与 g19_~g30_ 元组/g31_/g34_ 各族及 gpu fallthrough 全串
    # 互不包含）
    g35_sort_oit_gate_schema = load(
        ROOT / "milestones/g35/g35_sort_oit_gate_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g35_replay_gate_validator = (
        jsonschema.Draft7Validator(g35_replay_gate_schema)
        if g35_replay_gate_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g35_sort_oit_gate_validator = (
        jsonschema.Draft7Validator(g35_sort_oit_gate_schema)
        if g35_sort_oit_gate_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif (
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
ROUTE_ADD = '''        elif (
            f.name.startswith("g35_sort_oit_")
            and g35_sort_oit_gate_validator is not None
        ):
            # G35 GPU 粒子系统 G35-4 半透明双臂门裁决证据（含双标定纪元件——
            # g35_budget g35.oit.parity_p100 / g35.oit.wboit_acc_tol 双条目
            # evidence_file 指向门裁决件，results.trimmed_mean = sorted 见证
            # p100 镜像槽，budget_eval 通用路消费；wboit 条目实测承载 =
            # wboit_witness.acc_max_int_diff，双条目零容差预期面登记）→
            # milestones/g35/g35_sort_oit_gate_evidence_schema.json
            # （ci/g35_sort_oit_smoke.py --gate g35.wave4.sort_oit 产；lane
            # 真跑件〔rurix.g35.particle_lane_run.v1〕留 .tmp 不入 evidence/
            # 不注册；前缀分岔分析：g35_sort_oit_ 首段 sort_oit 与族内其余
            # 八支两两互不包含——族内无 g35_sort_ 独立前缀（W1 排序基元门 =
            # g35_primitives_）无遮蔽；与 g35_render_/g35_replay_ 在 g35_ 后
            # s≠r 分岔，与 g19_~g30_ 元组/g31_/g34_ 各族及 gpu fallthrough
            # 全串互不包含）。
            validator = g35_sort_oit_gate_validator
'''

PROBE = "g35_sort_oit_gate_schema = load"


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        s = f.read()
    if PROBE in s:
        ok = (
            LOAD_ADD in s
            and VALIDATOR_ADD in s
            and ROUTE_ADD in s
        )
        print(f"[patch_g35_sort_oit] 已驻留，核验 {'PASS' if ok else 'FAIL'}")
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
        print(f"[patch_g35_sort_oit] 锚缺失: {missing}", file=sys.stderr)
        return 1
    s = s.replace(LOAD_ANCHOR, LOAD_ANCHOR + LOAD_ADD, 1)
    s = s.replace(VALIDATOR_ANCHOR, VALIDATOR_ANCHOR + VALIDATOR_ADD, 1)
    s = s.replace(ROUTE_ANCHOR, ROUTE_ANCHOR + ROUTE_ADD, 1)
    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(s)
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        s2 = f.read()
    ok = LOAD_ADD in s2 and VALIDATOR_ADD in s2 and ROUTE_ADD in s2
    print(f"[patch_g35_sort_oit] 应用完成，重读核验 {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
