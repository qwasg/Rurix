#!/usr/bin/env python3
"""G8.1 治理守卫 — 验收映射覆盖 / 空行 / 三向命名空间一致性。

对应 milestones/g8/CI_GATES.md §3 的 `g8.gov.acceptance_coverage`，以及
milestones/g8/G8_ACCEPTANCE_MAP.md §4 的 coverage + no-empty 两组断言。

事实源三份，必须逐字一致（v1.1 勘误引入的机器锁）：
  1. milestones/g8/G8_ACCEPTANCE_MAP.md §2（18 P0）/ §3（3 已 go P1）
  2. milestones/g8/G8_CONTRACT.md §4.2（18 P0 独立断言表）
  3. milestones/g8/CI_GATES.md §4（18 P0）/ §4.0（3 已 go P1）

本守卫属未编号 `check_*` 类，不占 numeric CI step，不判定任何实现门为绿。
`--selftest` 用受控负样本证明每组断言都能红。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

MAP_PATH = ROOT / "milestones/g8/G8_ACCEPTANCE_MAP.md"
CONTRACT_PATH = ROOT / "milestones/g8/G8_CONTRACT.md"
CI_GATES_PATH = ROOT / "milestones/g8/CI_GATES.md"

EXPECTED_P0 = {
    "M50", "M89", "M29", "M30", "M31", "M32", "M85",
    "M79", "M80", "M81", "M01", "M04", "M37", "M19", "M24",
    "M66", "M67", "M68",
}
EXPECTED_P1_GO = {"M25", "M72", "M83"}

ALLOWED_WAVES = {
    "G8.2", "G8.3", "G8.4", "G8.5a", "G8.5b",
    "G8.6a", "G8.6b", "G8.6c", "G8.6d",
    "G8.2 + G8.3",
}

KEY_RE = re.compile(r"^g8\.p[01]\.m\d{2}\.[a-z0-9_]+$")
SCRIPT_RE = re.compile(r"ci/g8_[a-z0-9_]+_smoke\.py")
PLACEHOLDERS = ("TBD", "TODO", "待定", "待补", "待填", "—", "N/A")


class Finding(list):
    """收集失败原因；空 = PASS。"""


def _cells(line: str) -> list[str]:
    return [c.strip() for c in line.strip().strip("|").split("|")]


def parse_map(text: str) -> dict[str, dict]:
    """解析 ACCEPTANCE_MAP §2/§3 的 21 行。"""
    rows: dict[str, dict] = {}
    for line in text.splitlines():
        if not line.startswith("| **M"):
            continue
        cells = _cells(line)
        if len(cells) < 5:
            continue
        m = re.match(r"\*\*(M\d{2})\*\*", cells[0])
        if not m:
            continue
        row = cells[0]
        keys = re.findall(r"`(g8\.p[01]\.m\d{2}\.[a-z0-9_]+)`", cells[1])
        scripts = SCRIPT_RE.findall(cells[1])
        rows[m.group(1)] = {
            "raw_key_cell": cells[1],
            "keys": keys,
            "scripts": scripts,
            "schema": cells[2],
            "criteria": cells[3],
            "wave": cells[4].replace("**", "").strip(),
            "tier": "P1" if ".p1." in cells[1] else "P0",
            "row": row,
        }
    return rows


def parse_key_script_table(text: str) -> dict[str, tuple[str, str]]:
    """解析 CONTRACT §4.2 / CI_GATES §4·§4.0 形态的 `key | M## | wave | script` 行。"""
    out: dict[str, tuple[str, str]] = {}
    for line in text.splitlines():
        if not line.startswith("| `g8.p"):
            continue
        cells = _cells(line)
        key_m = re.match(r"`(g8\.p[01]\.m\d{2}\.[a-z0-9_]+)`", cells[0])
        m_m = re.search(r"(M\d{2})", cells[1]) if len(cells) > 1 else None
        script_m = SCRIPT_RE.search(line)
        if not (key_m and m_m and script_m):
            continue
        out[m_m.group(1)] = (key_m.group(1), script_m.group(0))
    return out


def check(map_text: str, contract_text: str, ci_gates_text: str) -> Finding:
    findings = Finding()
    rows = parse_map(map_text)
    p0 = {m for m, r in rows.items() if r["tier"] == "P0"}
    p1 = {m for m, r in rows.items() if r["tier"] == "P1"}

    # --- coverage 组 ---
    if p0 != EXPECTED_P0:
        findings.append(
            f"[coverage] P0 集合不等于冻结 18 行：缺 {sorted(EXPECTED_P0 - p0)}，多 {sorted(p0 - EXPECTED_P0)}"
        )
    if p1 != EXPECTED_P1_GO:
        findings.append(
            f"[coverage] 已 go P1 集合不等于 {sorted(EXPECTED_P1_GO)}：缺 {sorted(EXPECTED_P1_GO - p1)}，多 {sorted(p1 - EXPECTED_P1_GO)}"
        )
    if "M04" in p1:
        findings.append("[coverage] M04 属 P0，不得同时记入 P1")

    seen_keys: dict[str, str] = {}
    seen_schemas: dict[str, str] = {}
    for m in sorted(rows):
        row = rows[m]
        if len(row["keys"]) != 1:
            findings.append(f"[coverage] {m} 必须恰有一个 canonical symbolic key，实测 {row['keys']}")
            continue
        key = row["keys"][0]
        if not KEY_RE.match(key):
            findings.append(f"[coverage] {m} key `{key}` 不匹配 g8.p{{0,1}}.m##.<slug>")
        if key.split(".")[2] != m.lower():
            findings.append(f"[coverage] {m} key `{key}` 的 m## 段与行号不符")
        if key in seen_keys:
            findings.append(f"[coverage] key `{key}` 被 {seen_keys[key]} 与 {m} 共用")
        seen_keys[key] = m
        if not row["scripts"]:
            findings.append(f"[coverage] {m} 缺 `ci/g8_*_smoke.py` 脚本命令")
        if len(set(row["scripts"])) > 1:
            findings.append(f"[coverage] {m} 一个 key 只能绑定一个脚本，实测 {sorted(set(row['scripts']))}")
        gates = [g.strip("`") for g in re.findall(r"--gate\s+(\S+)", row["raw_key_cell"])]
        if not gates:
            findings.append(f"[coverage] {m} 脚本命令缺 --gate 参数")
        for gate in gates:
            if gate != key:
                findings.append(f"[coverage] {m} --gate `{gate}` ≠ canonical key `{key}`")
        if len(row["scripts"]) > 1 and len(set(row["scripts"])) == 1:
            phases = re.findall(r"--phase\s+(\S+)", row["raw_key_cell"])
            if len(phases) != len(row["scripts"]):
                findings.append(f"[coverage] {m} 同脚本多次调用必须以 --phase 区分阶段")

        # --- no-empty 组 ---
        for label, value in (("schema", row["schema"]), ("判据", row["criteria"]), ("波次", row["wave"])):
            if not value or value in PLACEHOLDERS:
                findings.append(f"[no-empty] {m} 的 {label} 列为空或占位（实测 {value!r}）")
            elif any(p in value for p in PLACEHOLDERS[:5]):
                findings.append(f"[no-empty] {m} 的 {label} 列含占位记号（实测 {value!r}）")
        schema_m = re.search(r"`(milestones/g8/g8_(m\d{2})_[a-z0-9_]+_evidence_schema\.json)`", row["schema"])
        if not schema_m:
            findings.append(f"[no-empty] {m} schema 路径不符 g8_m##_<slug>_evidence_schema.json：{row['schema']!r}")
        else:
            path, schema_m_no = schema_m.group(1), schema_m.group(2)
            if schema_m_no != m.lower():
                findings.append(f"[no-empty] {m} schema 路径的 m## 段不符：{path}")
            if path in seen_schemas:
                findings.append(f"[no-empty] schema 路径 {path} 被 {seen_schemas[path]} 与 {m} 共用")
            seen_schemas[path] = m
        if row["wave"] not in ALLOWED_WAVES:
            findings.append(f"[no-empty] {m} 波次 {row['wave']!r} 不在允许集合内")

    # --- 三向一致组 ---
    contract_rows = parse_key_script_table(contract_text)
    ci_rows = parse_key_script_table(ci_gates_text)
    for m in sorted(rows):
        if len(rows[m]["keys"]) != 1 or not rows[m]["scripts"]:
            continue
        key = rows[m]["keys"][0]
        script = rows[m]["scripts"][0]
        for name, table in (("G8_CONTRACT §4.2", contract_rows), ("CI_GATES §4/§4.0", ci_rows)):
            if m not in table:
                if name.startswith("G8_CONTRACT") and rows[m]["tier"] == "P1":
                    continue  # 契约 §4.2 只冻结 18 个 P0 断言
                findings.append(f"[three-way] {name} 缺 {m} 行")
                continue
            other_key, other_script = table[m]
            if other_key != key:
                findings.append(f"[three-way] {m} key 漂移：MAP `{key}` vs {name} `{other_key}`")
            if other_script != script:
                findings.append(f"[three-way] {m} script 漂移：MAP `{script}` vs {name} `{other_script}`")
    return findings


def run_selftest() -> int:
    map_text = MAP_PATH.read_text(encoding="utf-8")
    contract_text = CONTRACT_PATH.read_text(encoding="utf-8")
    ci_text = CI_GATES_PATH.read_text(encoding="utf-8")

    cases: list[tuple[str, str, str, str, str]] = [
        (
            "删除 M50 行 → coverage 必须红",
            "\n".join(l for l in map_text.splitlines() if not l.startswith("| **M50**")),
            contract_text,
            ci_text,
            "P0 集合不等于冻结 18 行",
        ),
        (
            "MAP 单侧改写 M68 key → three-way 必须红",
            map_text.replace("g8.p0.m68.fracture_pipeline", "g8.p0.m68.destruction_chain"),
            contract_text,
            ci_text,
            "[three-way] M68 key 漂移",
        ),
        (
            "恢复大写 key 写法 → coverage 必须红",
            map_text.replace("`g8.p0.m24.tsr_contract`", "`G8.P0.M24.TSR_CONTRACT`", 1),
            contract_text,
            ci_text,
            "M24 必须恰有一个 canonical symbolic key",
        ),
        (
            "判据列置为待补 → no-empty 必须红",
            map_text.replace(
                "只有 page-mark 或单帧 depth 不满足本门。", "待补", 1
            ),
            contract_text,
            ci_text,
            "[no-empty] M19",
        ),
        (
            "CI_GATES 单侧改脚本名 → three-way 必须红",
            map_text,
            contract_text,
            ci_text.replace("ci/g8_vsm_page_cache_smoke.py", "ci/g8_vsm_cache_smoke.py"),
            "[three-way] M19 script 漂移",
        ),
        (
            "P1 行冒充 P0（M83 改 p0 段）→ coverage 必须红",
            map_text.replace("g8.p1.m83.texture_transcode", "g8.p0.m83.texture_transcode"),
            contract_text,
            ci_text,
            "P0 集合不等于冻结 18 行",
        ),
    ]

    failures = 0
    for name, mt, ct, gt, expect in cases:
        got = check(mt, ct, gt)
        hit = [f for f in got if expect in f]
        if hit:
            print(f"  RED ok   — {name}（{hit[0]}）")
        elif got:
            print(f"  RED WRONG— {name}：判红但原因不符，期望含 {expect!r}，实测 {got[:2]}")
            failures += 1
        else:
            print(f"  RED MISS — {name}：负样本未被判红")
            failures += 1
    green = check(map_text, contract_text, ci_text)
    if green:
        print("  GREEN MISS — 当前树本应 PASS：")
        for f in green:
            print(f"    - {f}")
        failures += 1
    else:
        print("  GREEN ok — 当前树 PASS")
    if failures:
        print(f"[check_g8_acceptance_map] SELFTEST FAIL ({failures})")
        return 1
    print("[check_g8_acceptance_map] SELFTEST PASS (6 RED + 1 GREEN)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true", help="用受控负样本证明断言能红")
    args = parser.parse_args()
    if args.selftest:
        return run_selftest()

    for path in (MAP_PATH, CONTRACT_PATH, CI_GATES_PATH):
        if not path.exists():
            print(f"[check_g8_acceptance_map] FAIL — 缺事实源 {path.relative_to(ROOT)}")
            return 1

    findings = check(
        MAP_PATH.read_text(encoding="utf-8"),
        CONTRACT_PATH.read_text(encoding="utf-8"),
        CI_GATES_PATH.read_text(encoding="utf-8"),
    )
    if findings:
        print(f"[check_g8_acceptance_map] FAIL ({len(findings)} 项)")
        for f in findings:
            print(f"  - {f}")
        return 1
    print(
        "[check_g8_acceptance_map] PASS"
        f"（18 P0 + 3 已 go P1 覆盖齐备；21 key 唯一且同一命名空间；"
        f"MAP/CONTRACT/CI_GATES 三向逐字一致；零空行/占位）"
    )
    print("  注意：本 PASS 只表示映射完整，不表示任何 P0/P1 能力门已实现或已绿。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
