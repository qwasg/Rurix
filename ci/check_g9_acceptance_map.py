#!/usr/bin/env python3
"""G9.1 治理守卫 — 验收映射覆盖 / 空行 / 三向命名空间一致性（G9.3 波起含 §3 P1 面）。

对应 milestones/g9/CI_GATES.md §3 的 `g9.gov.acceptance_coverage`。

事实源三份，必须逐字一致（G9.1 冻结口径）：
  1. milestones/g9/G9_ACCEPTANCE_MAP.md §2（15 P0；G9.1 只映射 P0，go-P1 波次开工前只追加）
  2. milestones/g9/G9_CONTRACT.md §4.2（15 P0 独立断言表）
  3. milestones/g9/CI_GATES.md §4（15 P0）

G9.3 波 P1 全进裁决（G9_CONTRACT §8.1 裁决①，2026-08-11 只追加登记）落地后：
  - MAP §3 登记已 go P1 四行（M92/M105/M106/M107，key 形如 `g9.p1.m##.<slug>`）；
  - CI_GATES §4A 同构登记同四行；P1 行做 **MAP §3 ↔ CI_GATES §4A 双向逐字比对**
    （G9_CONTRACT §4.2 为 15 P0 独立断言表，不载 P1 行，P1 三向比对不适用）；
  - §2 P0 十五行的既有 coverage/no-empty/三向比对 **0-byte 不改弱**；
    后续波次判 go 的 P1 只追加扩 `EXPECTED_P1` 与 §3/§4A 表。
G9.4 波 P1 全进裁决（同一裁决①，2026-08-12 只追加登记）同口径扩三行
（M99/M100/M101；M99 仅屏幕级 / M100 仅低档默认判 go，RD-040 未举证分项
not-triggered 不充绿，G9_CANDIDATE_DECISIONS v1.3 校准注）。
G9.5 波 P1 全进裁决（同一裁决①，2026-08-12 只追加登记）同口径扩九行
（M111/M112/M113/M114/M115/M116/M117/M119/M120；M114 条件 go——strand 档
依赖 M120 精确档 benchmark 数据不足，分项 not-triggered 不充绿，承接锚
「M120 精确档数据落地后重判，兜底 G9.7 穷举」；M115 触 MaterialClosure 32B
经 RFC-0025 §4.L 🔒 显式修订行前置登记；M120 仅测量不定档；D4 伞形 RFC
缺口经起草 RFC-0025 处置，G9_CANDIDATE_DECISIONS v1.4 校准注）。

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

# 已 go P1 精确集合（G9_CONTRACT §8.1 裁决①：
# 逐波经治理流程只追加进 ACCEPTANCE_MAP §3，不静默并入既有 key）。
# 2026-08-11 G9.3 波四行（M92/M105/M106/M107）+ 2026-08-12 G9.4 波三行
# （M99/M100/M101；M99 仅屏幕级、M100 仅低档默认判 go，RD-040 未举证分项
# not-triggered 不充绿）+ 2026-08-12 G9.5 波九行（M111~M120 去 M118〔P0〕；
# M114 条件 go——strand 档依赖 M120 精确档数据不足 not-triggered 不充绿；
# M115 触 MaterialClosure 32B 经 RFC-0025 §4.L 修订行前置；M120 仅测量不定档）。
# 后续波次判 go 的 P1 只追加扩本集合 + MAP §3 + CI_GATES §4A。
EXPECTED_P1 = {
    "M92", "M105", "M106", "M107", "M99", "M100", "M101",
    "M111", "M112", "M113", "M114", "M115", "M116", "M117", "M119", "M120",
}

ALLOWED_WAVES = {"G9.2", "G9.3", "G9.4", "G9.5", "G9.6", "G9.2 + G9.6"}

KEY_RE = re.compile(r"^g9\.p0\.m\d{2,3}\.[a-z0-9_]+$")
KEY_IN_CELL_RE = re.compile(r"`(g9\.p0\.m\d{2,3}\.[a-z0-9_]+)`")
KEY_P1_RE = re.compile(r"^g9\.p1\.m\d{2,3}\.[a-z0-9_]+$")
KEY_P1_IN_CELL_RE = re.compile(r"`(g9\.p1\.m\d{2,3}\.[a-z0-9_]+)`")
KEY_P0_CELL_RE = re.compile(r"`(g9\.p0\.m\d{2,3}\.[a-z0-9_]+)`")
KEY_P1_CELL_RE = re.compile(r"`(g9\.p1\.m\d{2,3}\.[a-z0-9_]+)`")
SECTION_RE = re.compile(r"^## (\d+)\. ")
SCRIPT_RE = re.compile(r"ci/g9_[a-z0-9_]+_smoke\.py")
SCHEMA_RE = re.compile(r"`(milestones/g9/g9_(m\d{2,3})_[a-z0-9_]+_evidence_schema\.json)`")
PLACEHOLDERS = ("TBD", "TODO", "待定", "待补", "待填", "—", "N/A")


class Finding(list):
    """收集失败原因；空 = PASS。"""


def _cells(line: str) -> list[str]:
    return [c.strip() for c in line.strip().strip("|").split("|")]


def section_lines(text: str, section_no: int) -> list[str]:
    """取 `## <section_no>. ` 节首行至下一 `## N. ` 节之间的行（节内作用域）。"""
    out: list[str] = []
    in_sec = False
    for line in text.splitlines():
        m = SECTION_RE.match(line)
        if m:
            if in_sec:
                break
            in_sec = int(m.group(1)) == section_no
            continue
        if in_sec:
            out.append(line)
    return out


def parse_map_rows(lines: list[str], key_in_cell_re: re.Pattern) -> dict[str, dict]:
    """解析节内 `| **M##** | ... ` 行（§2 P0 / §3 P1 共用形态）。"""
    rows: dict[str, dict] = {}
    for line in lines:
        if not line.startswith("| **M"):
            continue
        cells = _cells(line)
        if len(cells) < 5:
            continue
        m = re.match(r"\*\*(M\d{2,3})\*\*", cells[0])
        if not m:
            continue
        keys = key_in_cell_re.findall(cells[1])
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


def parse_key_script_table(
    text: str, key_cell_re: re.Pattern
) -> dict[str, tuple[str, str]]:
    """解析 CONTRACT §4.2 / CI_GATES §4 / §4A 形态的 `key | M## | wave | script` 行。"""
    out: dict[str, tuple[str, str]] = {}
    for line in text.splitlines():
        if not line.startswith("| `g9.p"):
            continue
        cells = _cells(line)
        key_m = key_cell_re.match(cells[0])
        m_m = re.search(r"(M\d{2,3})", cells[1]) if len(cells) > 1 else None
        script_m = SCRIPT_RE.search(line)
        if not (key_m and m_m and script_m):
            continue
        out[m_m.group(1)] = (key_m.group(1), script_m.group(0))
    return out


def check_rows(
    rows: dict[str, dict],
    key_re: re.Pattern,
    key_desc: str,
    bad_ns: str,
    bad_ns_msg: str,
    seen_keys: dict[str, str],
    seen_schemas: dict[str, str],
    findings: Finding,
) -> None:
    """§2 P0 / §3 P1 行共用的 coverage + no-empty 断言组（逐行独立报告）。"""
    for m in sorted(rows):
        row = rows[m]
        if bad_ns in row["raw_key_cell"]:
            findings.append(f"[coverage] {m} 出现 {bad_ns.strip('.')} key：{bad_ns_msg}")
        if len(row["keys"]) != 1:
            findings.append(f"[coverage] {m} 必须恰有一个 canonical symbolic key，实测 {row['keys']}")
            continue
        key = row["keys"][0]
        if not key_re.match(key):
            findings.append(f"[coverage] {m} key `{key}` 不匹配 {key_desc}")
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


def check(map_text: str, contract_text: str, ci_gates_text: str) -> Finding:
    findings = Finding()
    rows = parse_map_rows(section_lines(map_text, 2), KEY_IN_CELL_RE)
    p1_rows = parse_map_rows(section_lines(map_text, 3), KEY_P1_IN_CELL_RE)

    # --- coverage 组 ---
    if set(rows) != EXPECTED_P0:
        findings.append(
            f"[coverage] P0 集合不等于冻结 15 行：缺 {sorted(EXPECTED_P0 - set(rows))}，多 {sorted(set(rows) - EXPECTED_P0)}"
        )
    if set(p1_rows) != EXPECTED_P1:
        findings.append(
            f"[coverage] P1 集合不等于 §1 声明集合：缺 {sorted(EXPECTED_P1 - set(p1_rows))}，多 {sorted(set(p1_rows) - EXPECTED_P1)}"
        )

    seen_keys: dict[str, str] = {}
    seen_schemas: dict[str, str] = {}
    check_rows(
        rows, KEY_RE, "g9.p0.m##.<slug>", ".p1.",
        "§2 只映射 P0（go-P1 只追加进 §3，不混入 P0 冻结面）",
        seen_keys, seen_schemas, findings,
    )
    check_rows(
        p1_rows, KEY_P1_RE, "g9.p1.m##.<slug>", ".p0.",
        "§3 只登记已 go P1（P0 行属 §2 冻结面）",
        seen_keys, seen_schemas, findings,
    )

    # --- 三向一致组（P0：MAP §2 ↔ CONTRACT §4.2 ↔ CI_GATES §4） ---
    contract_rows = parse_key_script_table(contract_text, KEY_P0_CELL_RE)
    ci_rows = parse_key_script_table(ci_gates_text, KEY_P0_CELL_RE)
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

    # --- 双向一致组（P1：MAP §3 ↔ CI_GATES §4A；CONTRACT §4.2 不载 P1 行） ---
    ci_p1_rows = parse_key_script_table(ci_gates_text, KEY_P1_CELL_RE)
    for m in sorted(p1_rows):
        if len(p1_rows[m]["keys"]) != 1 or not p1_rows[m]["scripts"]:
            continue
        key = p1_rows[m]["keys"][0]
        script = p1_rows[m]["scripts"][0]
        if m not in ci_p1_rows:
            findings.append(f"[two-way] CI_GATES §4A 缺 {m} 行")
            continue
        other_key, other_script = ci_p1_rows[m]
        if other_key != key:
            findings.append(f"[two-way] {m} key 漂移：MAP `{key}` vs CI_GATES §4A `{other_key}`")
        if other_script != script:
            findings.append(f"[two-way] {m} script 漂移：MAP `{script}` vs CI_GATES §4A `{other_script}`")
    return findings


# ---------------------------------------------------------------------------
# selftest 合成夹具：15 行 P0 冻结集合 + G9.3 波四行 P1 的正本，不依赖树上文件。
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

CANONICAL_P1_ROWS = [
    ("M92", "gpu_skinning_lod_update", "G9.3"),
    ("M105", "command_build_node", "G9.3"),
    ("M106", "execution_set_pso", "G9.3"),
    ("M107", "shader_library_ir_link", "G9.3"),
    ("M99", "spg_radiance_cache", "G9.4"),
    ("M100", "multi_light_low", "G9.4"),
    ("M101", "if_tier_ladder", "G9.4"),
    ("M111", "hlod_baking", "G9.5"),
    ("M112", "atmosphere_froxel", "G9.5"),
    ("M113", "water_dual_pipeline", "G9.5"),
    ("M114", "hair_marschner", "G9.5"),
    ("M115", "skin_burley_diffusion", "G9.5"),
    ("M116", "terrain_chunk_cell", "G9.5"),
    ("M117", "decal_dbuffer", "G9.5"),
    ("M119", "post_processing_skeleton", "G9.5"),
    ("M120", "oit_benchmark_harness", "G9.5"),
]


def _fixture() -> tuple[str, str, str]:
    map_lines = ["# fixture G9_ACCEPTANCE_MAP", "", "## 2. P0 映射", ""]
    contract_lines = ["# fixture G9_CONTRACT", "", "### 4.2 P0 独立断言表", ""]
    gates_lines = ["# fixture CI_GATES", "", "## 4. P0 门", ""]
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
    map_lines += ["", "## 3. 已 go P1", ""]
    gates_lines += ["", "## 4A. 已 go P1 门", ""]
    for m, slug, wave in CANONICAL_P1_ROWS:
        key = f"g9.p1.{m.lower()}.{slug}"
        script = f"ci/g9_{slug}_smoke.py"
        schema = f"milestones/g9/g9_{m.lower()}_{slug}_evidence_schema.json"
        map_lines.append(
            f"| **{m}** | `{key}` — `py -3 {script} --gate {key}` | `{schema}` | 合成判据 {m} | **{wave}** |"
        )
        gates_lines.append(f"| `{key}` | {m} | {wave} | `{script}` |")
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
        (
            "删除 §3 M107 行 → P1 coverage 必须红",
            "\n".join(l for l in map_text.splitlines() if not l.startswith("| **M107**")),
            contract_text,
            ci_text,
            "P1 集合不等于 §1 声明集合",
        ),
        (
            "§3 P1 行误用 p0 key → coverage 必须红",
            map_text.replace(
                "`g9.p1.m92.gpu_skinning_lod_update`", "`g9.p0.m92.gpu_skinning_lod_update`", 1
            ),
            contract_text,
            ci_text,
            "出现 p0 key",
        ),
        (
            "CI_GATES §4A 单侧改脚本名 → two-way 必须红",
            map_text,
            contract_text,
            ci_text.replace("ci/g9_execution_set_pso_smoke.py", "ci/g9_execution_set_smoke.py"),
            "[two-way] M106 script 漂移",
        ),
        (
            "删除 §3 M99 行（G9.4 波新增）→ P1 coverage 必须红",
            "\n".join(l for l in map_text.splitlines() if not l.startswith("| **M99**")),
            contract_text,
            ci_text,
            "P1 集合不等于 §1 声明集合",
        ),
        (
            "删除 §3 M115 行（G9.5 波新增）→ P1 coverage 必须红",
            "\n".join(l for l in map_text.splitlines() if not l.startswith("| **M115**")),
            contract_text,
            ci_text,
            "P1 集合不等于 §1 声明集合",
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
    print("[check_g9_acceptance_map] SELFTEST PASS (12 RED + 1 GREEN)")
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
        "（15 P0 + 16 已 go P1（G9.3 波四行 + G9.4 波三行 + G9.5 波九行）覆盖齐备；31 key 唯一且同一命名空间；"
        "P0 行 MAP/CONTRACT/CI_GATES 三向逐字一致、P1 行 MAP §3/CI_GATES §4A 双向逐字一致；零空行/占位）"
    )
    print("  注意：本 PASS 只表示映射完整，不表示任何 P0/P1 能力门已实现或已绿。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
