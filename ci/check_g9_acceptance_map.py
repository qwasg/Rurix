#!/usr/bin/env python3
"""G9.1 治理守卫 — 验收映射覆盖 / 空行 / 三向命名空间一致性。

对应 milestones/g9/CI_GATES.md §3 的 `g9.gov.acceptance_coverage`。

事实源三份，必须逐字一致（G9.1 冻结口径）：
  1. milestones/g9/G9_ACCEPTANCE_MAP.md §2（15 P0；G9.1 只映射 P0，go-P1 波次开工前只追加）
  2. milestones/g9/G9_CONTRACT.md §4.2（15 P0 独立断言表）
  3. milestones/g9/CI_GATES.md §4（15 P0）

本守卫属未编号 `check_*` 类，不占 numeric CI step，不判定任何实现门为绿。
`--selftest` 用内置合成夹具的受控负样本证明每组断言都能红（不依赖树上文件）。

编号注记：冻结 15 行含 M102~M122 三位里程碑号（key 形如 g9.p0.m102.<slug>），
故 key / schema 路径的 m 段用 \\d{2,3} 而非 \\d{2}；「m## 段与行号相符」断言
对两位（M90~M98）与三位（M102~M122）同样生效。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

MAP_PATH = ROOT / "milestones/g9/G9_ACCEPTANCE_MAP.md"
CONTRACT_PATH = ROOT / "milestones/g9/G9_CONTRACT.md"
CI_GATES_PATH = ROOT / "milestones/g9/CI_GATES.md"

# 冻结 15 行 P0 集合（2026-08-09 G9.1 立项裁决口径；go-P1 波次开工前只追加）。
EXPECTED_P0 = {
    "M90", "M91", "M102", "M103", "M104", "M121", "M122",
    "M93", "M94", "M95",
    "M96", "M97", "M98",
    "M110", "M118",
}

ALLOWED_WAVES = {"G9.2", "G9.3", "G9.4", "G9.5", "G9.6", "G9.2 + G9.6"}

KEY_RE = re.compile(r"^g9\.p0\.m\d{2,3}\.[a-z0-9_]+$")
KEY_IN_CELL_RE = re.compile(r"`(g9\.p0\.m\d{2,3}\.[a-z0-9_]+)`")
SCRIPT_RE = re.compile(r"ci/g9_[a-z0-9_]+_smoke\.py")
SCHEMA_RE = re.compile(r"`(milestones/g9/g9_(m\d{2,3})_[a-z0-9_]+_evidence_schema\.json)`")
PLACEHOLDERS = ("TBD", "TODO", "待定", "待补", "待填", "—", "N/A")


class Finding(list):
    """收集失败原因；空 = PASS。"""


def _cells(line: str) -> list[str]:
    return [c.strip() for c in line.strip().strip("|").split("|")]


def parse_map(text: str) -> dict[str, dict]:
    """解析 ACCEPTANCE_MAP §2 的 15 行 `| **M##** | ... ` 行。"""
    rows: dict[str, dict] = {}
    for line in text.splitlines():
        if not line.startswith("| **M"):
            continue
        cells = _cells(line)
        if len(cells) < 5:
            continue
        m = re.match(r"\*\*(M\d{2,3})\*\*", cells[0])
        if not m:
            continue
        keys = KEY_IN_CELL_RE.findall(cells[1])
        scripts = SCRIPT_RE.findall(cells[1])
        rows[m.group(1)] = {
            "raw_key_cell": cells[1],
            "keys": keys,
            "scripts": scripts,
            "schema": cells[2],
            "criteria": cells[3],
            "wave": cells[4].replace("**", "").strip(),
        }
    return rows


def parse_key_script_table(text: str) -> dict[str, tuple[str, str]]:
    """解析 CONTRACT §4.2 / CI_GATES §4 形态的 `key | M## | wave | script` 行。"""
    out: dict[str, tuple[str, str]] = {}
    for line in text.splitlines():
        if not line.startswith("| `g9.p"):
            continue
        cells = _cells(line)
        key_m = re.match(r"`(g9\.p0\.m\d{2,3}\.[a-z0-9_]+)`", cells[0])
        m_m = re.search(r"(M\d{2,3})", cells[1]) if len(cells) > 1 else None
        script_m = SCRIPT_RE.search(line)
        if not (key_m and m_m and script_m):
            continue
        out[m_m.group(1)] = (key_m.group(1), script_m.group(0))
    return out


def check(map_text: str, contract_text: str, ci_gates_text: str) -> Finding:
    findings = Finding()
    rows = parse_map(map_text)

    # --- coverage 组 ---
    if set(rows) != EXPECTED_P0:
        findings.append(
            f"[coverage] P0 集合不等于冻结 15 行：缺 {sorted(EXPECTED_P0 - set(rows))}，多 {sorted(set(rows) - EXPECTED_P0)}"
        )

    seen_keys: dict[str, str] = {}
    seen_schemas: dict[str, str] = {}
    for m in sorted(rows):
        row = rows[m]
        if ".p1." in row["raw_key_cell"]:
            findings.append(f"[coverage] {m} 出现 p1 key：G9.1 只映射 P0，go-P1 波次开工前只追加")
        if len(row["keys"]) != 1:
            findings.append(f"[coverage] {m} 必须恰有一个 canonical symbolic key，实测 {row['keys']}")
            continue
        key = row["keys"][0]
        if not KEY_RE.match(key):
            findings.append(f"[coverage] {m} key `{key}` 不匹配 g9.p0.m##.<slug>")
        if key.split(".")[2] != m.lower():
            findings.append(f"[coverage] {m} key `{key}` 的 m## 段与行号不符")
        if key in seen_keys:
            findings.append(f"[coverage] key `{key}` 被 {seen_keys[key]} 与 {m} 共用")
        seen_keys[key] = m
        if not row["scripts"]:
            findings.append(f"[coverage] {m} 缺 `ci/g9_*_smoke.py` 脚本命令")
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
            if not value.strip() or value in PLACEHOLDERS:
                findings.append(f"[no-empty] {m} 的 {label} 列为空或占位（实测 {value!r}）")
            elif any(p in value for p in PLACEHOLDERS[:5]):
                findings.append(f"[no-empty] {m} 的 {label} 列含占位记号（实测 {value!r}）")
        schema_m = SCHEMA_RE.search(row["schema"])
        if not schema_m:
            findings.append(f"[no-empty] {m} schema 路径不符 g9_m##_<slug>_evidence_schema.json：{row['schema']!r}")
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
        for name, table in (("G9_CONTRACT §4.2", contract_rows), ("CI_GATES §4", ci_rows)):
            if m not in table:
                findings.append(f"[three-way] {name} 缺 {m} 行")
                continue
            other_key, other_script = table[m]
            if other_key != key:
                findings.append(f"[three-way] {m} key 漂移：MAP `{key}` vs {name} `{other_key}`")
            if other_script != script:
                findings.append(f"[three-way] {m} script 漂移：MAP `{script}` vs {name} `{other_script}`")
    return findings


# ---------------------------------------------------------------------------
# selftest 合成夹具：15 行冻结集合的正本，不依赖树上文件。
# ---------------------------------------------------------------------------

CANONICAL_ROWS = [
    ("M90", "cluster_dag_deepening", "G9.2"),
    ("M91", "page_format_v2_abi", "G9.2"),
    ("M102", "dgc_abstraction", "G9.2"),
    ("M103", "descriptor_global_table", "G9.2"),
    ("M104", "accesskind_indirect_edge", "G9.2"),
    ("M121", "physics_particle_view", "G9.2 + G9.6"),
    ("M122", "gameplay_field", "G9.2 + G9.6"),
    ("M93", "visible_cluster_set", "G9.3"),
    ("M94", "clas_rt_convergence", "G9.3"),
    ("M95", "single_source_truth", "G9.3"),
    ("M96", "path_tracer_reference", "G9.4"),
    ("M97", "surface_cache", "G9.4"),
    ("M98", "tracing_fallback_chain", "G9.4"),
    ("M110", "world_partition", "G9.5"),
    ("M118", "display_pipeline_view_transform", "G9.5"),
]


def _fixture() -> tuple[str, str, str]:
    map_lines = ["# fixture G9_ACCEPTANCE_MAP", "", "## §2 P0 映射", ""]
    contract_lines = ["# fixture G9_CONTRACT", "", "### §4.2 P0 独立断言表", ""]
    gates_lines = ["# fixture CI_GATES", "", "## §4 P0 门", ""]
    for m, slug, wave in CANONICAL_ROWS:
        key = f"g9.p0.{m.lower()}.{slug}"
        script = f"ci/g9_{slug}_smoke.py"
        schema = f"milestones/g9/g9_{m.lower()}_{slug}_evidence_schema.json"
        map_lines.append(
            f"| **{m}** | `{key}` — `py -3 {script} --gate {key}` | `{schema}` | 合成判据 {m} | **{wave}** |"
        )
        row = f"| `{key}` | {m} | {wave} | `{script}` |"
        contract_lines.append(row)
        gates_lines.append(row)
    return "\n".join(map_lines), "\n".join(contract_lines), "\n".join(gates_lines)


def run_selftest() -> int:
    map_text, contract_text, ci_text = _fixture()

    cases: list[tuple[str, str, str, str, str]] = [
        (
            "删除 M118 行 → coverage 必须红",
            "\n".join(l for l in map_text.splitlines() if not l.startswith("| **M118**")),
            contract_text,
            ci_text,
            "P0 集合不等于冻结 15 行",
        ),
        (
            "MAP 单侧改写 M98 key → three-way 必须红",
            map_text.replace("g9.p0.m98.tracing_fallback_chain", "g9.p0.m98.fallback_chain"),
            contract_text,
            ci_text,
            "[three-way] M98 key 漂移",
        ),
        (
            "恢复大写 key 写法 → coverage 必须红",
            map_text.replace("`g9.p0.m90.cluster_dag_deepening`", "`G9.P0.M90.CLUSTER_DAG_DEEPENING`", 1),
            contract_text,
            ci_text,
            "M90 必须恰有一个 canonical symbolic key",
        ),
        (
            "判据列置为待补 → no-empty 必须红",
            map_text.replace("合成判据 M96", "待补", 1),
            contract_text,
            ci_text,
            "[no-empty] M96",
        ),
        (
            "CI_GATES 单侧改脚本名 → three-way 必须红",
            map_text,
            contract_text,
            ci_text.replace("ci/g9_surface_cache_smoke.py", "ci/g9_surface_smoke.py"),
            "[three-way] M97 script 漂移",
        ),
        (
            "波次改写为非法 G9.7 → no-empty 必须红",
            map_text.replace("合成判据 M110 | **G9.5**", "合成判据 M110 | **G9.7**", 1),
            contract_text,
            ci_text,
            "[no-empty] M110 波次",
        ),
        (
            "schema 路径 m## 段改写 → no-empty 必须红",
            map_text.replace(
                "milestones/g9/g9_m94_clas_rt_convergence_evidence_schema.json",
                "milestones/g9/g9_m95_clas_rt_convergence_evidence_schema.json",
                1,
            ),
            contract_text,
            ci_text,
            "[no-empty] M94 schema 路径的 m## 段不符",
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
        print("  GREEN MISS — 合成夹具正本本应 PASS：")
        for f in green:
            print(f"    - {f}")
        failures += 1
    else:
        print("  GREEN ok — 合成夹具正本 PASS")
    if failures:
        print(f"[check_g9_acceptance_map] SELFTEST FAIL ({failures})")
        return 1
    print("[check_g9_acceptance_map] SELFTEST PASS (7 RED + 1 GREEN)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true", help="用受控负样本证明断言能红")
    args = parser.parse_args()
    if args.selftest:
        return run_selftest()

    for path in (MAP_PATH, CONTRACT_PATH, CI_GATES_PATH):
        if not path.exists():
            print(f"[check_g9_acceptance_map] FAIL — 缺事实源 {path.relative_to(ROOT)}")
            return 1

    findings = check(
        MAP_PATH.read_text(encoding="utf-8"),
        CONTRACT_PATH.read_text(encoding="utf-8"),
        CI_GATES_PATH.read_text(encoding="utf-8"),
    )
    if findings:
        print(f"[check_g9_acceptance_map] FAIL ({len(findings)} 项)")
        for f in findings:
            print(f"  - {f}")
        return 1
    print(
        "[check_g9_acceptance_map] PASS"
        "（15 P0 覆盖齐备；15 key 唯一且同一命名空间；"
        "MAP/CONTRACT/CI_GATES 三向逐字一致；零空行/占位）"
    )
    print("  注意：本 PASS 只表示映射完整，不表示任何 P0 能力门已实现或已绿。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
