#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""影子/off-tree 编号工作流登记守卫(MR-0010;CPU-only,纯 stdlib,check_ 守卫风格)。

兑现 10 §9.5「编号永不复用」的**跨分支执行面**:一个 off-main 分支(如 GRX 影子
分支 codex/grx-godot-dxil-workspace,closed)可能消费共享编号命名空间(MR-0006/0007、
RXS-0181~0184 等);main 侧只能靠人工跳号避撞,无结构化登记、无机器门。本守卫把
registry/number_ledger.json 的登记落为可机核事实源,并强制两条**可靠判定**。

三查:
1. **树内同号异义碰撞(blocking)**:扫 spec/**/*.md 的 `### RXS-####` 条款头——同一
   RXS 号出现 ≥2 个 heading 定义即红;扫 registry/{deferred,spike_gating,error_codes}.json
   的 entry `id`——单文件内 id 重复即红。
2. **保留号被尊重(blocking,仅可靠判定项)**:
   - (2a) number_ledger 中 `shadow_reserved` 标注的号,若在当前树**新出现**为条款定义
     (如 shadow-reserved RXS-0181 突然获得 `### RXS-0181` 头)即红——有人复用了对 main
     永久 burned 的号(10 §9.5);
   - (2b) number_ledger 内部一致性:每命名空间 `next_free` 必须 > max(`shadow_reserved`)
     且 > `on_tree_max`(声明的下一个自由号须跳过树内已用与影子保留两者)。
3. **台账引用存在性(advisory,打印不阻断)**:对 `off_tree_workflows` 的分支/commit ref
   做 `git rev-parse --verify`,打印 exists/missing,**绝不 exit 非零**(CI 可能是浅 clone /
   不含该分支,不可因此误红)。

**能力诚实边界(反虚门,14 §5 证据分级)**:CI 只见当前分支树,**无法枚举/扫描其它未合
分支**——故本守卫**不能**「自动发现 untracked 编号工作流」。新影子工作流的登记仍需一次
**人工/agent 前置动作**录入 number_ledger.json。守卫只强制(a)树内同号异义碰撞 +(b)
**已登记**保留号被当前树尊重。**不宣称完全自动化发现**。

check_ 守卫风格:**不分配错误码**(07 §5)、**不写 evidence**、**不接 budget counter**;
内置 `red_self_test()` 反 YAML-only。**blocking(exit 1)**——区别于 check_contribution 的
advisory:编号永不复用(10 §9.5)是抗混乱**硬不变式**,非 autonomy 可放松的软约定。

用法: py -3 ci/check_number_ledger.py
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER_PATH = ROOT / "registry/number_ledger.json"

# spec 条款头:`### RXS-####`(host_orchestration.md 等既有体例)。
RXS_HEADING_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)
# 带 entry id 的 registry 事实源(单文件内 id 唯一)。
REGISTRY_ID_FILES = ("deferred.json", "spike_gating.json", "error_codes.json")
ENTRY_ID_RE = re.compile(r'"id"\s*:\s*"([A-Z]{1,3}-?\d{3,4})"')


# ————————————————————— 纯判定层(red 自检直接喂合成数据)—————————————————————


def detect_heading_collisions(heading_ids: list[int], label: str = "spec RXS") -> list[str]:
    """同一编号出现 ≥2 个条款头定义 = 同号异义碰撞(10 §9.5)。纯函数。"""
    problems: list[str] = []
    for num, cnt in sorted(Counter(heading_ids).items()):
        if cnt > 1:
            problems.append(
                f"{label} 同号异义碰撞:RXS-{num:04d} 出现 {cnt} 个 `### RXS-` 条款头定义"
                "(编号永不复用,10 §9.5;同一号只能定义一次)"
            )
    return problems


def detect_id_dups(ids: list[str], label: str) -> list[str]:
    """registry 单文件内 entry id 重复。纯函数。"""
    problems: list[str] = []
    for eid, cnt in sorted(Counter(ids).items()):
        if cnt > 1:
            problems.append(f"{label}: entry id 重复 {eid}(×{cnt};编号永不复用,10 §9.5)")
    return problems


def detect_reserved_reuse(
    shadow_reserved: list[int], on_tree_ids: set[int], ns: str
) -> list[str]:
    """shadow-reserved(对 main 永久 burned)的号在树内新出现为定义 = 红。纯函数。"""
    problems: list[str] = []
    for n in sorted(set(shadow_reserved)):
        if n in on_tree_ids:
            problems.append(
                f"{ns}: shadow-reserved 号 {ns}-{n:04d} 在当前树新出现为条款定义"
                "——该号已被 off-tree 影子分支 claim、对 main 永久 burned(10 §9.5),"
                "不得复用;若确需该语义请取下一个自由号"
            )
    return problems


def check_ledger_internal(namespaces: dict) -> list[str]:
    """number_ledger 内部一致性(2b):next_free > max(shadow_reserved) 且 > on_tree_max。纯函数。"""
    problems: list[str] = []
    for ns, meta in namespaces.items():
        nf = meta.get("next_free")
        if not isinstance(nf, int):
            continue  # 无数字化 next_free 的命名空间(如 G 门为里程碑分段,非全局序)跳过
        reserved = [r for r in meta.get("shadow_reserved", []) if isinstance(r, int)]
        otm = meta.get("on_tree_max")
        floor = max([*reserved, otm if isinstance(otm, int) else -1], default=-1)
        if reserved and nf <= max(reserved):
            problems.append(
                f"{ns}: next_free={nf} 未跳过 shadow_reserved 最大 {max(reserved)}"
                "(下一个自由号须 > 影子保留号,防复用 burned 号)"
            )
        if isinstance(otm, int) and nf <= otm:
            problems.append(
                f"{ns}: next_free={nf} 未跳过 on_tree_max={otm}(下一个自由号须 > 树内已用)"
            )
        if floor >= 0 and nf <= floor:
            # 冗余兜底(上两条已覆盖),保留以显式化 max 语义。
            pass
    return problems


# ————————————————————— IO 采集层 —————————————————————


def scan_spec_rxs_headings() -> list[int]:
    ids: list[int] = []
    spec_dir = ROOT / "spec"
    if not spec_dir.is_dir():
        return ids
    for md in sorted(spec_dir.glob("*.md")):
        text = md.read_text(encoding="utf-8")
        ids.extend(int(m) for m in RXS_HEADING_RE.findall(text))
    return ids


def scan_registry_id_dups() -> list[str]:
    problems: list[str] = []
    for name in REGISTRY_ID_FILES:
        p = ROOT / "registry" / name
        if not p.is_file():
            continue
        ids = ENTRY_ID_RE.findall(p.read_text(encoding="utf-8"))
        problems.extend(detect_id_dups(ids, f"registry/{name}"))
    return problems


def git_ref_exists(ref: str) -> bool:
    return (
        subprocess.run(
            ["git", "rev-parse", "--verify", "--quiet", ref],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        ).returncode
        == 0
    )


def load_ledger() -> dict | None:
    if not LEDGER_PATH.is_file():
        return None
    return json.loads(LEDGER_PATH.read_text(encoding="utf-8"))


# ————————————————————— red 自检 —————————————————————


def red_self_test() -> None:
    """反 YAML-only:合成碰撞 / 复用 / 干净数据,断言门能区分红绿。门失效即红。"""
    # (a) 同号异义碰撞 → 应判红。
    if not detect_heading_collisions([9990, 9991, 9990]):
        _fail("red 自检失败:重复条款头(RXS-9990 ×2)未被识别为碰撞(门失效)")
    # (b) 干净 heading → 应判绿。
    if detect_heading_collisions([9990, 9991, 9992]):
        _fail("red 自检失败:无重复的 heading 被误判为碰撞(门过严)")
    # (c) shadow-reserved 号新出现为树内定义 → 应判红。
    if not detect_reserved_reuse([9990], {9990, 9991}, "RXS"):
        _fail("red 自检失败:shadow-reserved 号 9990 树内复用未被识别(门失效)")
    # (d) shadow-reserved 号未出现在树内 → 应判绿。
    if detect_reserved_reuse([9990], {9991, 9992}, "RXS"):
        _fail("red 自检失败:未被复用的保留号被误判(门过严)")
    # (e) registry id 重复 → 应判红。
    if not detect_id_dups(["RD-001", "RD-002", "RD-001"], "synthetic"):
        _fail("red 自检失败:重复 entry id 未被识别(门失效)")
    # (f) 内部一致性:next_free 未跳过 shadow_reserved → 应判红。
    if not check_ledger_internal({"RXS": {"next_free": 184, "shadow_reserved": [181, 184]}}):
        _fail("red 自检失败:next_free 未跳过 shadow_reserved 未被识别(门失效)")
    if check_ledger_internal({"RXS": {"next_free": 214, "shadow_reserved": [181, 184], "on_tree_max": 213}}):
        _fail("red 自检失败:合规 next_free 被误判(门过严)")


def _fail(msg: str) -> None:
    print(f"[check_number_ledger] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


# ————————————————————— 主流程 —————————————————————


def main() -> int:
    red_self_test()

    errors: list[str] = []
    advisories: list[str] = []

    # 查 1:树内同号异义碰撞(spec 条款头 + registry id),始终跑(不依赖 ledger)。
    heading_ids = scan_spec_rxs_headings()
    errors.extend(detect_heading_collisions(heading_ids))
    errors.extend(scan_registry_id_dups())

    ledger = load_ledger()
    if ledger is None:
        # ledger 未落地(本 MR 前):查 1 仍有意义,查 2/3 跳过。
        print("[check_number_ledger] registry/number_ledger.json 不存在,跳过保留号/引用校验(查 1 已跑)")
    else:
        namespaces = ledger.get("namespaces", {})
        on_tree_rxs = set(heading_ids)
        # 查 2a:shadow-reserved 号被树内复用(仅对 spec 可机检的 RXS 命名空间强制)。
        rxs_meta = namespaces.get("RXS", {})
        rxs_reserved = [r for r in rxs_meta.get("shadow_reserved", []) if isinstance(r, int)]
        errors.extend(detect_reserved_reuse(rxs_reserved, on_tree_rxs, "RXS"))
        # 查 2b:ledger 内部一致性(全命名空间)。
        errors.extend(check_ledger_internal(namespaces))
        # 查 2c(advisory):ledger.RXS.on_tree_max 与实测 spec 最大值漂移提示(不阻断)。
        if heading_ids:
            actual_max = max(heading_ids)
            declared = rxs_meta.get("on_tree_max")
            if isinstance(declared, int) and declared != actual_max:
                advisories.append(
                    f"RXS: ledger on_tree_max={declared} 与实测 spec `### RXS-` 最大 {actual_max} 漂移"
                    "(台账可能滞后;新增条款后请同步 number_ledger.json 的 on_tree_max/next_free)"
                )
        # 查 3(advisory):off_tree_workflows 分支/commit ref 存在性。
        for wf in ledger.get("off_tree_workflows", []):
            wid = wf.get("id", "<?>")
            for key in ("branch", "closeout_commit"):
                ref = wf.get(key)
                if not ref:
                    continue
                status = "exists" if git_ref_exists(ref) else "MISSING(浅 clone 或未 fetch,非错误)"
                advisories.append(f"off_tree_workflows[{wid}].{key} = {ref} → {status}")

    if advisories:
        print("[check_number_ledger] ADVISORY(不阻断):")
        for a in advisories:
            print(f"  - {a}")

    if errors:
        print("[check_number_ledger] FAIL")
        for e in errors:
            print(f"  - {e}")
        return 1

    n_ns = len(ledger.get("namespaces", {})) if ledger else 0
    print(
        f"[check_number_ledger] PASS(spec RXS 头 {len(heading_ids)} 个零同号碰撞;"
        f"ledger {n_ns} 命名空间保留号被尊重;red 自检已过)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
