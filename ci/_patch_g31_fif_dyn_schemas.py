#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# G38 T2:check_schemas.py 三处纯追加注册 FIF×动态判档 probe v2 schema
# (io.open 补丁法——newline="" 字节面保全,既有路由 0-byte;打补丁后立即重读
# 验证驻留 + py_compile)。前缀 g31_fif_dyn_probe_ 与既有 g31_* 全族互不包含
# (最近邻 g31_frame_pipelining_ / g31_framegen_present 第 5 字符即分叉)⇒ 无
# 路由序约束;v1(rurix.g31.fif_dyn_probe.v1)为 artifacts/ 判档 sidecar 自
# declare 形态,从未注册 schema 文件/路由——本 v2 为首个注册件,如实登记,
# 「v2 先于 v1」序律空适用。本脚本幂等:已驻留即只做核验(锚唯一性机核,
# 锚不唯一/缺失即 FAIL 拒改不猜)。先例 ci/_patch_g31_sdk_dist_v2_schemas.py。
import io
import sys
from pathlib import Path

P = str(Path(__file__).resolve().parent / "check_schemas.py")

LOAD_ANCHOR = '''    g31_dynamic_scene_schema = load(
        ROOT / "milestones/g31/g31_dynamic_scene_evidence_schema.json"
    )
'''
LOAD_ADD = '''    # G38 T2 FIF×动态判档 probe v2 schema 纯追加(重放幂等面;TODO #90 /
    # RFC-0030 §4.3 L2a;v1 为 artifacts sidecar 自 declare 从未注册,v2 首注册)
    g31_fif_dyn_probe_v2_schema = load(
        ROOT / "milestones/g31/g31_fif_dyn_probe_v2_evidence_schema.json"
    )
'''

VALIDATOR_ANCHOR = '''    g31_dynamic_scene_validator = (
        jsonschema.Draft7Validator(g31_dynamic_scene_schema)
        if g31_dynamic_scene_schema is not None
        else None
    )
'''
VALIDATOR_ADD = '''    g31_fif_dyn_probe_v2_validator = (
        jsonschema.Draft7Validator(g31_fif_dyn_probe_v2_schema)
        if g31_fif_dyn_probe_v2_schema is not None
        else None
    )
'''

ROUTE_ANCHOR = '''        elif (
            f.name.startswith("g31_dynamic_scene_")
            and g31_dynamic_scene_validator is not None
        ):
            # G31+ 波 A Task A4 动态场景 refit/rebuild 对照证据 →
            # milestones/g31/g31_dynamic_scene_evidence_schema.json
            # （ci/g31_dynamic_scene_smoke.py --gate g31.waveA.dynscene 产）。
            validator = g31_dynamic_scene_validator
'''
ROUTE_ADD = '''        elif (
            f.name.startswith("g31_fif_dyn_probe_")
            and g31_fif_dyn_probe_v2_validator is not None
        ):
            # G38 T2 FIF×动态共存判档 probe 直出证据 v2(TODO #90;RFC-0030
            # §4.3 L2a 每槽 AS 副本 opt-in)→
            # milestones/g31/g31_fif_dyn_probe_v2_evidence_schema.json
            # (src/rurix-render/src/bin/g31_fif_dyn_probe.rs --out 产;三臂
            # 等价七门 + slot_as_mem 内存账 + results.trimmed_mean 镜像槽供
            # ci/budget_eval.py 通用路判读 g31.fif_dyn.slot_as_group_mem_bytes;
            # 前缀与既有 g31_* 全族互不包含,v1 从未注册无序律)。
            validator = g31_fif_dyn_probe_v2_validator
'''


def main() -> int:
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        src = f.read()

    # 幂等面:三处均已驻留 → 只核验不重复插。
    already = all(
        token in src
        for token in (
            "g31_fif_dyn_probe_v2_schema = load(",
            "g31_fif_dyn_probe_v2_validator = (",
            'f.name.startswith("g31_fif_dyn_probe_")',
        )
    )
    if not already:
        # 锚唯一性机核(并发改动下锚不唯一/缺失即拒改)。
        for name, anchor in [
            ("load", LOAD_ANCHOR),
            ("validator", VALIDATOR_ANCHOR),
            ("route", ROUTE_ANCHOR),
        ]:
            n = src.count(anchor)
            if n != 1:
                print(f"[patch_g31_fif_dyn] FAIL: {name} 锚出现 {n} 次（≠1 拒改）",
                      file=sys.stderr)
                return 1
        src = src.replace(LOAD_ANCHOR, LOAD_ANCHOR + LOAD_ADD, 1)
        src = src.replace(VALIDATOR_ANCHOR, VALIDATOR_ANCHOR + VALIDATOR_ADD, 1)
        src = src.replace(ROUTE_ANCHOR, ROUTE_ANCHOR + ROUTE_ADD, 1)
        print("[patch_g31_fif_dyn] 三处纯追加插入(load/validator/route 各于锚后 1 处)")

    # 驻留核验(每 token 恰 1)。
    misses = []
    for name, token in [
        ("load", "g31_fif_dyn_probe_v2_schema = load("),
        ("validator", "g31_fif_dyn_probe_v2_validator = ("),
        ("route", 'f.name.startswith("g31_fif_dyn_probe_")'),
    ]:
        n = src.count(token)
        if n != 1:
            misses.append(f"{name} 驻留数 {n} ≠ 1")
    if misses:
        for m in misses:
            print(f"[patch_g31_fif_dyn] FAIL {m}", file=sys.stderr)
        return 1

    with io.open(P, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    with io.open(P, "r", encoding="utf-8", newline="") as f:
        back = f.read()
    for token in (
        "g31_fif_dyn_probe_v2_schema = load(",
        "g31_fif_dyn_probe_v2_validator = (",
        'f.name.startswith("g31_fif_dyn_probe_")',
    ):
        if back.count(token) != 1:
            print(f"[patch_g31_fif_dyn] FAIL: 重读驻留数异常 {token!r}", file=sys.stderr)
            return 1
    import py_compile
    py_compile.compile(P, doraise=True)
    print("[patch_g31_fif_dyn] PASS:三处纯追加驻留(load/validator/route 各 1)"
          "+ 重读核验 + py_compile 绿")
    return 0


if __name__ == "__main__":
    sys.exit(main())
