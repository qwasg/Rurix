"""编号台账守卫单测(MR-0010 配套;合成 fixture 红绿 + 真实仓库 PASS)。

失败测试先行(10 §3 Mini 硬性):合成同号异义 / 保留号复用输入断言守卫判红;
干净输入断言判绿;当前真实 main 树 + registry/number_ledger.json 断言守卫整体 PASS
(RXS-0181~0184 在 main 树内零条款定义 = 无同树碰撞;历史碰撞是跨分支事实,登记在
ledger 的 known_collisions,守卫对其不红)。
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from ci.check_number_ledger import (  # noqa: E402
    check_ledger_internal,
    detect_heading_collisions,
    detect_id_dups,
    detect_reserved_reuse,
    load_ledger,
    main,
    scan_registry_id_dups,
    scan_spec_rxs_headings,
)


# —— 查 1:树内同号异义碰撞 ——


def test_heading_collision_detected():
    # 同一 RXS 号出现两个条款头 → 碰撞。
    assert detect_heading_collisions([1, 2, 1])
    assert "RXS-0001" in detect_heading_collisions([1, 2, 1])[0]


def test_heading_no_collision_green():
    assert detect_heading_collisions([1, 2, 3]) == []


def test_registry_id_dup_detected():
    assert detect_id_dups(["RD-001", "RD-002", "RD-001"], "synthetic")
    assert detect_id_dups(["RD-001", "RD-002"], "synthetic") == []


# —— 查 2a:shadow-reserved 号被树内复用 ——


def test_reserved_reuse_detected():
    # 181 是 shadow-reserved(GRX claim);若树内新出现 RXS-0181 定义 → 红。
    problems = detect_reserved_reuse([181, 184], {181, 200}, "RXS")
    assert problems
    assert "RXS-0181" in problems[0]


def test_reserved_not_reused_green():
    # 181/184 未出现在树内 heading 集 → 绿。
    assert detect_reserved_reuse([181, 184], {200, 213}, "RXS") == []


# —— 查 2b:ledger 内部一致性 ——


def test_internal_next_free_must_skip_reserved():
    # next_free 未跳过 shadow_reserved 最大 → 红。
    assert check_ledger_internal({"RXS": {"next_free": 184, "shadow_reserved": [181, 184]}})
    # next_free 未跳过 on_tree_max → 红。
    assert check_ledger_internal({"RXS": {"next_free": 213, "on_tree_max": 213}})


def test_internal_consistent_green():
    ok = {"RXS": {"next_free": 214, "shadow_reserved": [181, 184], "on_tree_max": 213}}
    assert check_ledger_internal(ok) == []
    # 无整数 next_free 的命名空间(如里程碑分段 G 门)跳过,不误红。
    assert check_ledger_internal({"G": {"next_free": None, "shadow_reserved": []}}) == []


# —— 真实仓库 ——


def test_real_main_tree_has_no_rxs_heading_collision():
    # main 树 spec 条款头零同号异义(RXS-0181~0184 在 main 零定义,是跳号非复用)。
    assert detect_heading_collisions(scan_spec_rxs_headings()) == []
    assert scan_registry_id_dups() == []


def test_real_ledger_reserved_numbers_not_on_tree():
    ledger = load_ledger()
    assert ledger is not None, "registry/number_ledger.json 应已落地"
    on_tree = set(scan_spec_rxs_headings())
    reserved = [
        r for r in ledger["namespaces"]["RXS"].get("shadow_reserved", []) if isinstance(r, int)
    ]
    assert reserved, "RXS 命名空间应登记 GRX 影子保留号"
    # 每个 GRX-claim 的 RXS 保留号都不得在 main 树出现为条款定义。
    assert detect_reserved_reuse(reserved, on_tree, "RXS") == []


def test_real_ledger_internal_consistent():
    ledger = load_ledger()
    assert ledger is not None
    assert check_ledger_internal(ledger["namespaces"]) == []


def test_guard_main_passes_on_current_tree():
    # 守卫整体在当前 main 树 + 已落地 ledger 上 PASS(exit 0)。
    assert main() == 0
