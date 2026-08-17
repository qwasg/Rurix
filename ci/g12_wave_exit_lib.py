#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.2 生产化核心波续）
"""G12 波次聚合门共享库(milestones/g12/CI_GATES.md §5;同构 ci/g11_wave_exit_lib.py)。

只读汇总独立门 evidence,不重跑 smoke、不代绿。任一 required gate 缺失 /
非 PASS / SKIP / DEV_ENV_DEGRADE → 聚合红。供 G12.2~G12.7b 薄壳复用。

判定逻辑与 g11 版逐字节同构(gate_pass_reason / DEVICE_FAIL_STATES 不改
语义);`--selftest` 直接核验 gate_pass_reason 红绿两臂。
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

# g11_wave_exit_lib 为通用只读聚合实现(命名空间参数化);G12 共享库直接
# 复用其判定面,零语义改动(G12 CI_GATES §5 登记本文件为 G12 共享库——
# 实现形态 = 对 g11 通用库的命名空间转接,判定逻辑 0-byte)。
from g11_wave_exit_lib import (  # noqa: E402,F401
    DEVICE_FAIL_STATES,
    EVIDENCE_DIR,
    ROOT,
    collect_environment,
    emit_wave_evidence,
    gate_pass_reason,
    load_json,
    load_latest_evidence,
    load_rd_status,
    require_gate_pass,
    rfc_agent_approved,
    utc_stamp,
    validate_schema,
)


def run_selftest() -> int:
    """gate_pass_reason 红绿两臂自检(g12 key 面;不依赖树上 evidence)。"""
    key = "g12.p0.m158.mis_full_surface"
    good = {
        "symbolic_gate_key": key,
        "host_section_pass": True,
        "device_section_state": "executed",
        "checks": {"a": True, "b": True},
    }
    failures = 0

    def red(name: str, ev: dict) -> None:
        nonlocal failures
        ok, detail = gate_pass_reason(ev, key)
        if not ok:
            print(f"  RED ok   — {name}({detail})")
        else:
            print(f"  RED MISS — {name}:负样本被判 PASS")
            failures += 1

    red("key 漂移", {**good, "symbolic_gate_key": "g12.p0.m159.russian_roulette_prod"})
    red("host_section_pass 非 True", {**good, "host_section_pass": False})
    red("device fail 态", {**good, "device_section_state": "fail"})
    red("device dev_env_degrade 态", {**good, "device_section_state": "dev_env_degrade"})
    red("device SKIP 态", {**good, "device_section_state": "SKIP"})
    red("checks 含非真", {**good, "checks": {"a": True, "b": False}})

    ok, detail = gate_pass_reason(good, key)
    if ok and detail == "PASS":
        print("  GREEN ok — 合形 evidence 判 PASS")
    else:
        print(f"  GREEN MISS — 合形 evidence 本应 PASS,实测 {detail}")
        failures += 1

    if failures:
        print(f"[g12_wave_exit_lib] SELFTEST FAIL ({failures})")
        return 1
    print("[g12_wave_exit_lib] SELFTEST PASS (6 RED + 1 GREEN)")
    return 0


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        sys.exit(run_selftest())
    print(__doc__)
    sys.exit(0)
