#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""stable API 快照冻结机制(RD-008 激活,G2.5 语言 1.0;RFC-0008 §9 Q-RD008 /
spec/edition.md RXS-0180;check_ 守卫风格,CPU-only,反 YAML-only)。

RD-008 经 G2.5 语言 1.0(首个 stable 发布触发点)激活:定义 stable 面 + 落快照比对 +
bless 守卫。本脚本把 Rurix **stable 面**(稳定语言面的**存在性 + 含义**,**非二进制
ABI 保证**,RXS-0180 L3)计算为确定性快照,镜像 UI/MIR/PTX/DXIL golden bless 纪律:

  - edition_anchor : 首个 edition(stable 面版本锚,RXS-0180 L1)
  - editions       : src/rurix-pkg/src/manifest.rs VALID_EDITIONS 合法 edition 值集
  - rx_cli_subcommands : src/rx/src/main.rs USAGE 广告的稳定子命令面
  - spec_clauses   : spec/*.md 全部 `### RXS-####` 条款 ID(排序;稳定语言面)
  - error_codes    : registry/error_codes.json entries 的 id → message_key(排序;
                     错误码 ID/含义冻结,10 §6)

用法: py -3 ci/stable_snapshot.py [--check]
  默认 / --check : 重算当前 stable 面,与入库 tests/stable/stable_api.snapshot 比对;
                  不一致 → FAIL(stable 面变更须经 `RURIX_BLESS=1` 重 bless +
                  tests/stable/bless_log.md 追加,check_guardrails check_stable_snapshot_bless
                  守卫;同一 edition 内 stable 面只增不破坏,RXS-0180 L2)。
  RURIX_BLESS=1 : 写 tests/stable/stable_api.snapshot(agent bless)。

内置 red 自检(反 YAML-only):合成篡改快照,断言比较器判「不一致」——证明门真在比对
stable 面、能区分「一致 vs 漂移」,而非空过。🔒 快照仅锚定 stable 面的存在性 + 含义
(条款 ID / 错误码 ID-含义 / edition 值 / 子命令名),**不冻结 register/字节布局/工具
版本为语言 ABI**(RXS-0180 L3,对齐 RXS-0162 / RXS-0165 先例)。
"""
import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SNAPSHOT = ROOT / "tests" / "stable" / "stable_api.snapshot"
SPEC_DIR = ROOT / "spec"
ERROR_CODES = ROOT / "registry" / "error_codes.json"
MANIFEST_RS = ROOT / "src" / "rurix-pkg" / "src" / "manifest.rs"
RX_MAIN_RS = ROOT / "src" / "rx" / "src" / "main.rs"

CLAUSE_RE = re.compile(r"^###\s+(RXS-\d{4})\b", re.MULTILINE)
VALID_EDITIONS_RE = re.compile(r"VALID_EDITIONS:\s*&\[&str\]\s*=\s*&\[(.*?)\]", re.DOTALL)
EDITION_LIT_RE = re.compile(r'"([^"]+)"')
USAGE_RE = re.compile(r"rx\s+<([a-z|]+)>")


def fail(msg: str) -> None:
    print(f"[stable_snapshot] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def collect_spec_clauses() -> list[str]:
    clauses: set[str] = set()
    for p in sorted(SPEC_DIR.glob("*.md")):
        if p.name == "README.md":
            continue
        for cid in CLAUSE_RE.findall(p.read_text(encoding="utf-8")):
            clauses.add(cid)
    return sorted(clauses)


def collect_error_codes() -> dict[str, str]:
    data = json.loads(ERROR_CODES.read_text(encoding="utf-8"))
    out: dict[str, str] = {}
    for entry in data.get("entries", []):
        eid = entry.get("id", "")
        mk = entry.get("message_key", "")
        if eid:
            out[eid] = mk
    return dict(sorted(out.items()))


def collect_editions() -> list[str]:
    text = MANIFEST_RS.read_text(encoding="utf-8")
    m = VALID_EDITIONS_RE.search(text)
    if not m:
        fail(f"未在 {MANIFEST_RS.relative_to(ROOT)} 找到 VALID_EDITIONS 常量(stable 面 edition 集源)")
    eds = EDITION_LIT_RE.findall(m.group(1))
    if not eds:
        fail("VALID_EDITIONS 为空(stable 面至少含首个 edition)")
    return sorted(eds)


def collect_rx_subcommands() -> list[str]:
    text = RX_MAIN_RS.read_text(encoding="utf-8")
    m = USAGE_RE.search(text)
    if not m:
        fail(f"未在 {RX_MAIN_RS.relative_to(ROOT)} 找到 rx CLI USAGE 子命令面")
    subs = [s for s in m.group(1).split("|") if s]
    if not subs:
        fail("rx CLI USAGE 子命令面为空")
    return sorted(subs)


def compute_surface() -> dict:
    editions = collect_editions()
    return {
        "schema_version": 1,
        "subject": "rurix_stable_api_surface",
        "note": (
            "RD-008 stable 面快照(G2.5 语言 1.0 激活,RFC-0008)。锚定稳定语言面的存在性+含义,"
            "非二进制 ABI 保证(RXS-0180 L3)。变更须经 RURIX_BLESS=1 重 bless + tests/stable/bless_log.md 追加。"
        ),
        "edition_anchor": editions[0],
        "editions": editions,
        "rx_cli_subcommands": collect_rx_subcommands(),
        "spec_clause_count": len(collect_spec_clauses()),
        "spec_clauses": collect_spec_clauses(),
        "error_code_count": len(collect_error_codes()),
        "error_codes": collect_error_codes(),
    }


def render(surface: dict) -> str:
    return json.dumps(surface, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def red_self_test() -> None:
    """red 自检(反 YAML-only):合成基准 + 篡改副本,断言比较器判「漂移」。"""
    base = {"spec_clauses": ["RXS-0001"], "editions": ["2026"]}
    tampered = {"spec_clauses": ["RXS-0001", "RXS-9999"], "editions": ["2026"]}
    if render(base) == render(tampered):
        fail("red 自检失败:比较器未识别 stable 面漂移(门空过失效)")


def main() -> int:
    red_self_test()
    surface = compute_surface()
    rendered = render(surface)

    if os.environ.get("RURIX_BLESS") == "1":
        SNAPSHOT.parent.mkdir(parents=True, exist_ok=True)
        SNAPSHOT.write_text(rendered, encoding="utf-8")
        print(
            f"[stable_snapshot] BLESS 写 {SNAPSHOT.relative_to(ROOT)}"
            f"(spec_clauses={surface['spec_clause_count']},error_codes={surface['error_code_count']},"
            f"editions={surface['editions']},subcommands={surface['rx_cli_subcommands']})"
        )
        return 0

    if not SNAPSHOT.is_file():
        fail(
            f"缺 {SNAPSHOT.relative_to(ROOT)}——首份 stable 快照未 bless"
            f"(RD-008 激活:RURIX_BLESS=1 py -3 ci/stable_snapshot.py)"
        )
    current = SNAPSHOT.read_text(encoding="utf-8")
    if current != rendered:
        # 诊断:列出漂移的 stable 面段。
        try:
            old = json.loads(current)
        except json.JSONDecodeError:
            old = {}
        diffs = []
        for key in ("editions", "rx_cli_subcommands", "spec_clauses", "error_codes", "edition_anchor"):
            if old.get(key) != surface.get(key):
                diffs.append(key)
        fail(
            "stable 面与入库快照不一致(漂移段: "
            + ", ".join(diffs)
            + ")——stable 面变更须经 RURIX_BLESS=1 重 bless + tests/stable/bless_log.md 追加"
            "(RD-008 / RXS-0180;同一 edition 内只增不破坏)"
        )
    print(
        f"[stable_snapshot] PASS(stable 面与入库快照一致:spec_clauses={surface['spec_clause_count']},"
        f"error_codes={surface['error_code_count']},editions={surface['editions']},"
        f"subcommands={surface['rx_cli_subcommands']})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
