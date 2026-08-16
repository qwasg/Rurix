#!/usr/bin/env python3
# Assisted-by: Kimi-K3（G11.1 治理波 validator）
"""G11.1 治理守卫 — 验收映射覆盖 / 空行 / 三向命名空间一致性（G11_CONTRACT §4.2 冻结面）。

对应 milestones/g11/CI_GATES.md §3 的 `g11.gov.acceptance_coverage`。

事实源三份，必须逐字一致（G11.1 冻结口径）：
  1. milestones/g11/G11_ACCEPTANCE_MAP.md §1（13 P0）与 §2（1 已 go P1：M157）
  2. milestones/g11/G11_CONTRACT.md §4.2（13 P0 独立断言表；不载 P1 行）
  3. milestones/g11/CI_GATES.md §4（13 P0）与 §4A（1 已 go P1）

三向比对面（MAP §4.1 机器可核声明）：
  - P0 行：MAP §1 ↔ CONTRACT §4.2 ↔ CI_GATES §4 的 symbolic gate key 与稳定脚本名逐字相等；
  - evidence schema 目标路径：MAP §1 ↔ CI_GATES §4 逐行逐字相等，且全部命中 CONTRACT §4.2
    冻结的统一形态 `milestones/g11/g11_m<###>_<slug>_evidence_schema.json`（CONTRACT 只冻结
    形态不逐行载路径，故 CONTRACT 侧以形态机核、逐行字面由 MAP ↔ CI_GATES 双侧比对）；
  - P1 行：MAP §2 ↔ CI_GATES §4A 双向逐字比对（key/脚本/schema；CONTRACT §4.2 不载 P1 行）。
  - slug 同字面机核：key 末段 slug、脚本名 `ci/g11_<slug>_smoke.py`、schema 文件名
    `g11_m<###>_<slug>_evidence_schema.json` 三者同 slug（MAP §1 单一命名空间字面）。

数字步骤纪律：`numeric_step` 列只接受字面 `post-interlock actual-next-free allocation`，
出现任何数字即判预占（MAP §1 编号纪律 / CI_GATES §1.2）。

本守卫属未编号 `check_*` 类，不占 numeric CI step，不判定任何实现门为绿。
`--selftest` 用内置合成夹具的受控负样本证明每组断言都能红（不依赖树上文件）。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

MAP_PATH = ROOT / "milestones/g11/G11_ACCEPTANCE_MAP.md"
CONTRACT_PATH = ROOT / "milestones/g11/G11_CONTRACT.md"
CI_GATES_PATH = ROOT / "milestones/g11/CI_GATES.md"

# 冻结 13 行 P0 集合（2026-08-16 G11.1 立项裁决口径；go-P1 波次开工前只追加）。
EXPECTED_P0 = {
    "M144", "M145", "M146",
    "M147", "M148", "M149", "M150", "M151", "M152",
    "M153", "M154",
    "M155", "M156",
}

# 已 go P1 精确集合（G11_CONTRACT §4.2 末段「M157（HDR-FLIP 独立标定）为 P1，
# 入验收映射随主门核验」）。后续波次判 go 的 P1 只追加扩本集合 + MAP §2 + CI_GATES §4A。
EXPECTED_P1 = {"M157"}

# MAP §5：所有波次属于 G11.2|G11.3|G11.4|G11.5。
ALLOWED_WAVES = {"G11.2", "G11.3", "G11.4", "G11.5"}

# numeric_step 唯一合法字面（CI_GATES §1.2：数字步骤 post-interlock 按 actual next_free 分配）。
NUMERIC_STEP_LITERAL = "post-interlock actual-next-free allocation"

KEY_RE = re.compile(r"^g11\.p0\.m\d{3}\.[a-z0-9_]+$")
KEY_IN_CELL_RE = re.compile(r"`(g11\.p0\.m\d{3}\.[a-z0-9_]+)`")
KEY_P1_RE = re.compile(r"^g11\.p1\.m\d{3}\.[a-z0-9_]+$")
KEY_P1_IN_CELL_RE = re.compile(r"`(g11\.p1\.m\d{3}\.[a-z0-9_]+)`")
KEY_P0_CELL_RE = re.compile(r"`(g11\.p0\.m\d{3}\.[a-z0-9_]+)`")
KEY_P1_CELL_RE = re.compile(r"`(g11\.p1\.m\d{3}\.[a-z0-9_]+)`")
SECTION_RE = re.compile(r"^## (\d+)\. ")
SCRIPT_RE = re.compile(r"ci/g11_[a-z0-9_]+_smoke\.py")
SCHEMA_RE = re.compile(r"`(milestones/g11/g11_(m\d{3})_[a-z0-9_]+_evidence_schema\.json)`")
BOLD_RE = re.compile(r"\*\*([^*]+)\*\*")
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


def _wave_of(cell: str) -> str:
    """最晚波次列取加粗段。"""
    m = BOLD_RE.search(cell)
    return (m.group(1) if m else cell.replace("**", "")).strip()


def parse_map_rows(lines: list[str], key_in_cell_re: re.Pattern) -> dict[str, dict]:
    """解析节内 `| **M###** | ... ` 行（§1 P0 / §2 P1 共用八列形态）。"""
    rows: dict[str, dict] = {}
    for line in lines:
        if not line.startswith("| **M"):
            continue
        cells = _cells(line)
        if len(cells) < 8:
            continue
        m = re.match(r"\*\*(M\d{3})\*\*", cells[0])
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
            "red_arms": cells[4],
            "device_host": cells[5],
            "wave": _wave_of(cells[6]),
            "numeric_step": cells[7].replace("**", "").strip(),
        }
    return rows


def parse_key_script_table(
    text: str, key_cell_re: re.Pattern
) -> dict[str, dict]:
    """解析 CONTRACT §4.2 / CI_GATES §4 / §4A 形态的 `key | M### | ... ` 行。

    返回 {M###: {"key", "script", "schema"}}；CONTRACT §4.2 不载 schema 列（None）。
    """
    out: dict[str, dict] = {}
    for line in text.splitlines():
        if not line.startswith("| `g11.p"):
            continue
        cells = _cells(line)
        key_m = key_cell_re.match(cells[0])
        m_m = re.search(r"(M\d{3})", cells[1]) if len(cells) > 1 else None
        script_m = SCRIPT_RE.search(line)
        if not (key_m and m_m and script_m):
            continue
        schema_m = SCHEMA_RE.search(line)
        out[m_m.group(1)] = {
            "key": key_m.group(1),
            "script": script_m.group(0),
            "schema": schema_m.group(1) if schema_m else None,
        }
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
    """§1 P0 / §2 P1 行共用的 coverage + no-empty 断言组（逐行独立报告）。"""
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
            findings.append(f"[coverage] {m} key `{key}` 的 m### 段与行号不符")
        if key in seen_keys:
            findings.append(f"[coverage] key `{key}` 被 {seen_keys[key]} 与 {m} 共用")
        seen_keys[key] = m
        slug = key.split(".")[3]
        if not row["scripts"]:
            findings.append(f"[coverage] {m} 缺 `ci/g11_*_smoke.py` 脚本命令")
        if len(set(row["scripts"])) > 1:
            findings.append(f"[coverage] {m} 一个 key 只能绑定一个脚本，实测 {sorted(set(row['scripts']))}")
        expected_script = f"ci/g11_{slug}_smoke.py"
        for script in set(row["scripts"]):
            if script != expected_script:
                findings.append(
                    f"[coverage] {m} 脚本名 `{script}` ≠ key slug 同字面形态 `{expected_script}`"
                )
        gates = [g.strip("`") for g in re.findall(r"--gate\s+(\S+)", row["raw_key_cell"])]
        if not gates:
            findings.append(f"[coverage] {m} 脚本命令缺 --gate 参数")
        for gate in gates:
            if gate != key:
                findings.append(f"[coverage] {m} --gate `{gate}` ≠ canonical key `{key}`")

        # --- no-empty 组 ---
        for label, value in (
            ("schema", row["schema"]),
            ("判据", row["criteria"]),
            ("负例 RED 臂", row["red_arms"]),
            ("device/host 性质", row["device_host"]),
            ("波次", row["wave"]),
            ("numeric_step", row["numeric_step"]),
        ):
            if not value.strip() or value in PLACEHOLDERS:
                findings.append(f"[no-empty] {m} 的 {label} 列为空或占位（实测 {value!r}）")
            elif any(p in value for p in PLACEHOLDERS[:5]):
                findings.append(f"[no-empty] {m} 的 {label} 列含占位记号（实测 {value!r}）")
        schema_m = SCHEMA_RE.search(row["schema"])
        if not schema_m:
            findings.append(f"[no-empty] {m} schema 路径不符 g11_m###_<slug>_evidence_schema.json：{row['schema']!r}")
        else:
            path, schema_m_no = schema_m.group(1), schema_m.group(2)
            if schema_m_no != m.lower():
                findings.append(f"[no-empty] {m} schema 路径的 m### 段不符：{path}")
            expected_schema = f"milestones/g11/g11_{m.lower()}_{slug}_evidence_schema.json"
            if path != expected_schema:
                findings.append(
                    f"[no-empty] {m} schema 路径 slug 与 key 末段不同字面：`{path}` ≠ `{expected_schema}`"
                )
            if path in seen_schemas:
                findings.append(f"[no-empty] schema 路径 {path} 被 {seen_schemas[path]} 与 {m} 共用")
            seen_schemas[path] = m
        if row["wave"] not in ALLOWED_WAVES:
            findings.append(f"[no-empty] {m} 波次 {row['wave']!r} 不在允许集合内")
        if row["numeric_step"] != NUMERIC_STEP_LITERAL:
            findings.append(
                f"[no-empty] {m} numeric_step 列必须为字面 `{NUMERIC_STEP_LITERAL}`"
                f"（数字步骤零预占，实测 {row['numeric_step']!r}）"
            )


def check(map_text: str, contract_text: str, ci_gates_text: str) -> Finding:
    findings = Finding()
    rows = parse_map_rows(section_lines(map_text, 1), KEY_IN_CELL_RE)
    p1_rows = parse_map_rows(section_lines(map_text, 2), KEY_P1_IN_CELL_RE)

    # --- coverage 组 ---
    if set(rows) != EXPECTED_P0:
        findings.append(
            f"[coverage] P0 集合不等于冻结 13 行：缺 {sorted(EXPECTED_P0 - set(rows))}，多 {sorted(set(rows) - EXPECTED_P0)}"
        )
    if set(p1_rows) != EXPECTED_P1:
        findings.append(
            f"[coverage] P1 集合不等于 §2 声明集合：缺 {sorted(EXPECTED_P1 - set(p1_rows))}，多 {sorted(set(p1_rows) - EXPECTED_P1)}"
        )

    seen_keys: dict[str, str] = {}
    seen_schemas: dict[str, str] = {}
    check_rows(
        rows, KEY_RE, "g11.p0.m###.<slug>", ".p1.",
        "§1 只映射 P0（go-P1 只追加进 §2，不混入 P0 冻结面）",
        seen_keys, seen_schemas, findings,
    )
    check_rows(
        p1_rows, KEY_P1_RE, "g11.p1.m###.<slug>", ".p0.",
        "§2 只登记已 go P1（P0 行属 §1 冻结面）",
        seen_keys, seen_schemas, findings,
    )

    # --- 三向一致组（P0：MAP §1 ↔ CONTRACT §4.2 ↔ CI_GATES §4） ---
    contract_rows = parse_key_script_table(contract_text, KEY_P0_CELL_RE)
    ci_rows = parse_key_script_table(ci_gates_text, KEY_P0_CELL_RE)
    for m in sorted(rows):
        if len(rows[m]["keys"]) != 1 or not rows[m]["scripts"]:
            continue
        key = rows[m]["keys"][0]
        script = rows[m]["scripts"][0]
        schema_m = SCHEMA_RE.search(rows[m]["schema"])
        schema = schema_m.group(1) if schema_m else None
        for name, table in (("G11_CONTRACT §4.2", contract_rows), ("CI_GATES §4", ci_rows)):
            if m not in table:
                findings.append(f"[three-way] {name} 缺 {m} 行")
                continue
            other = table[m]
            if other["key"] != key:
                findings.append(f"[three-way] {m} key 漂移：MAP `{key}` vs {name} `{other['key']}`")
            if other["script"] != script:
                findings.append(f"[three-way] {m} script 漂移：MAP `{script}` vs {name} `{other['script']}`")
        # schema 逐字比对：MAP §1 ↔ CI_GATES §4（CONTRACT §4.2 侧由形态机核覆盖，
        # 见 check_rows 的 g11_m###_<slug> 同字面断言）。
        if m in ci_rows and schema is not None:
            if ci_rows[m]["schema"] is None:
                findings.append(f"[three-way] CI_GATES §4 {m} 缺 schema 目标路径列")
            elif ci_rows[m]["schema"] != schema:
                findings.append(
                    f"[three-way] {m} schema 漂移：MAP `{schema}` vs CI_GATES §4 `{ci_rows[m]['schema']}`"
                )

    # --- 双向一致组（P1：MAP §2 ↔ CI_GATES §4A；CONTRACT §4.2 不载 P1 行） ---
    ci_p1_rows = parse_key_script_table(ci_gates_text, KEY_P1_CELL_RE)
    for m in sorted(p1_rows):
        if len(p1_rows[m]["keys"]) != 1 or not p1_rows[m]["scripts"]:
            continue
        key = p1_rows[m]["keys"][0]
        script = p1_rows[m]["scripts"][0]
        schema_m = SCHEMA_RE.search(p1_rows[m]["schema"])
        schema = schema_m.group(1) if schema_m else None
        if m not in ci_p1_rows:
            findings.append(f"[two-way] CI_GATES §4A 缺 {m} 行")
            continue
        other = ci_p1_rows[m]
        if other["key"] != key:
            findings.append(f"[two-way] {m} key 漂移：MAP `{key}` vs CI_GATES §4A `{other['key']}`")
        if other["script"] != script:
            findings.append(f"[two-way] {m} script 漂移：MAP `{script}` vs CI_GATES §4A `{other['script']}`")
        if schema is not None:
            if other["schema"] is None:
                findings.append(f"[two-way] CI_GATES §4A {m} 缺 schema 目标路径列")
            elif other["schema"] != schema:
                findings.append(
                    f"[two-way] {m} schema 漂移：MAP `{schema}` vs CI_GATES §4A `{other['schema']}`"
                )
    return findings


# ---------------------------------------------------------------------------
# selftest 合成夹具：13 行 P0 冻结集合 + 1 行已 go P1 的正本，不依赖树上文件。
# ---------------------------------------------------------------------------

CANONICAL_ROWS = [
    ("M144", "caliber_c1_indoor_luminance", "G11.2"),
    ("M145", "caliber_c2_exposure_chain", "G11.2"),
    ("M146", "caliber_c3_exr_bit_depth", "G11.2"),
    ("M147", "fix_r1_material_subset", "G11.3"),
    ("M148", "fix_r2_geometry_normals", "G11.3"),
    ("M149", "fix_r5_json_u64_seed", "G11.3"),
    ("M150", "fix_u1_cornell_shell_radiance", "G11.3"),
    ("M151", "fix_u2_bistro_texture_dds", "G11.3"),
    ("M152", "fix_u3_bistro_animation", "G11.3"),
    ("M153", "fix_r3_light_subset", "G11.4"),
    ("M154", "fix_r4_gi_multibounce_world_cache", "G11.4"),
    ("M155", "ab_retest_closure", "G11.5"),
    ("M156", "regression_guard", "G11.5"),
]

CANONICAL_P1_ROWS = [
    ("M157", "hdr_flip_calibration", "G11.2"),
]


def _fixture() -> tuple[str, str, str]:
    map_lines = ["# fixture G11_ACCEPTANCE_MAP", "", "## 1. P0 硬门", ""]
    contract_lines = ["# fixture G11_CONTRACT", "", "### 4.2 P0 独立断言", ""]
    gates_lines = ["# fixture CI_GATES", "", "## 4. 13 个 P0 独立机器断言", ""]
    for m, slug, wave in CANONICAL_ROWS:
        key = f"g11.p0.{m.lower()}.{slug}"
        script = f"ci/g11_{slug}_smoke.py"
        schema = f"milestones/g11/g11_{m.lower()}_{slug}_evidence_schema.json"
        cmd = f"`py -3 {script} --gate {key}`"
        map_lines.append(
            f"| **{m}** | `{key}`<br>{cmd} | `{schema}` | 合成判据 {m} | 合成 RED 臂 {m} | "
            f"host+device | **{wave}** | {NUMERIC_STEP_LITERAL} |"
        )
        contract_lines.append(f"| `{key}` | {m} | {wave} | `{script}` | 合成判据 {m} |")
        gates_lines.append(f"| `{key}` | {m} | {wave} | `{script}` | `{schema}` | 合成判据摘要 {m} |")
    map_lines += ["", "## 2. 已 go P1 硬门", ""]
    gates_lines += ["", "## 4A. 已 go P1 独立机器断言", ""]
    for m, slug, wave in CANONICAL_P1_ROWS:
        key = f"g11.p1.{m.lower()}.{slug}"
        script = f"ci/g11_{slug}_smoke.py"
        schema = f"milestones/g11/g11_{m.lower()}_{slug}_evidence_schema.json"
        map_lines.append(
            f"| **{m}** | `{key}`<br>`py -3 {script} --gate {key}` | `{schema}` | 合成判据 {m} | "
            f"合成 RED 臂 {m} | host 纯 host | **{wave}** | {NUMERIC_STEP_LITERAL} |"
        )
        gates_lines.append(f"| `{key}` | {m} | {wave} | `{script}` | `{schema}` | 合成判据摘要 {m} |")
    return "\n".join(map_lines), "\n".join(contract_lines), "\n".join(gates_lines)


def run_selftest() -> int:
    map_text, contract_text, ci_text = _fixture()

    cases: list[tuple[str, str, str, str, str]] = [
        (
            "删除 M156 行 → coverage 必须红",
            "\n".join(l for l in map_text.splitlines() if not l.startswith("| **M156**")),
            contract_text,
            ci_text,
            "P0 集合不等于冻结 13 行",
        ),
        (
            "MAP 单侧改写 M154 key → three-way 必须红",
            map_text.replace("g11.p0.m154.fix_r4_gi_multibounce_world_cache", "g11.p0.m154.fix_r4_gi"),
            contract_text,
            ci_text,
            "[three-way] M154 key 漂移",
        ),
        (
            "恢复大写 key 写法 → coverage 必须红",
            map_text.replace("`g11.p0.m144.caliber_c1_indoor_luminance`", "`G11.P0.M144.CALIBER_C1_INDOOR_LUMINANCE`", 1),
            contract_text,
            ci_text,
            "M144 必须恰有一个 canonical symbolic key",
        ),
        (
            "判据列置为待补 → no-empty 必须红",
            map_text.replace("合成判据 M147", "待补", 1),
            contract_text,
            ci_text,
            "[no-empty] M147",
        ),
        (
            "CI_GATES 单侧改脚本名 → three-way 必须红",
            map_text,
            contract_text,
            ci_text.replace("ci/g11_fix_r1_material_subset_smoke.py", "ci/g11_fix_r1_smoke.py"),
            "[three-way] M147 script 漂移",
        ),
        (
            "波次改写为非法 G11.6 → no-empty 必须红",
            map_text.replace("合成 RED 臂 M144 | host+device | **G11.2**", "合成 RED 臂 M144 | host+device | **G11.6**", 1),
            contract_text,
            ci_text,
            "[no-empty] M144 波次",
        ),
        (
            "schema 路径 m### 段改写 → no-empty 必须红",
            map_text.replace(
                "milestones/g11/g11_m148_fix_r2_geometry_normals_evidence_schema.json",
                "milestones/g11/g11_m149_fix_r2_geometry_normals_evidence_schema.json",
                1,
            ),
            contract_text,
            ci_text,
            "[no-empty] M148 schema 路径的 m### 段不符",
        ),
        (
            "numeric_step 列填入数字 → 预占必须红",
            map_text.replace(
                "host+device | **G11.2** | post-interlock actual-next-free allocation |",
                "host+device | **G11.2** | 196 |",
                1,
            ),
            contract_text,
            ci_text,
            "[no-empty] M144 numeric_step",
        ),
        (
            "CI_GATES §4 单侧改 schema 路径 → three-way 必须红",
            map_text,
            contract_text,
            ci_text.replace(
                "milestones/g11/g11_m155_ab_retest_closure_evidence_schema.json",
                "milestones/g11/g11_m155_ab_retest_evidence_schema.json",
            ),
            "[three-way] M155 schema 漂移",
        ),
        (
            "删除 §2 M157 行 → P1 coverage 必须红",
            "\n".join(l for l in map_text.splitlines() if not l.startswith("| **M157**")),
            contract_text,
            ci_text,
            "P1 集合不等于 §2 声明集合",
        ),
        (
            "§2 P1 行误用 p0 key → coverage 必须红",
            map_text.replace(
                "`g11.p1.m157.hdr_flip_calibration`", "`g11.p0.m157.hdr_flip_calibration`", 1
            ),
            contract_text,
            ci_text,
            "出现 p0 key",
        ),
        (
            "CI_GATES §4A 单侧改脚本名 → two-way 必须红",
            map_text,
            contract_text,
            ci_text.replace("ci/g11_hdr_flip_calibration_smoke.py", "ci/g11_hdr_flip_smoke.py"),
            "[two-way] M157 script 漂移",
        ),
        (
            "CI_GATES §4A 单侧改 schema 路径 → two-way 必须红",
            map_text,
            contract_text,
            ci_text.replace(
                "milestones/g11/g11_m157_hdr_flip_calibration_evidence_schema.json",
                "milestones/g11/g11_m157_flip_calibration_evidence_schema.json",
            ),
            "[two-way] M157 schema 漂移",
        ),
        (
            "脚本名与 key slug 不同字面 → coverage 必须红",
            map_text.replace("ci/g11_regression_guard_smoke.py", "ci/g11_regression_smoke.py"),
            contract_text,
            ci_text,
            "M156 脚本名",
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
        print(f"[check_g11_acceptance_map] SELFTEST FAIL ({failures})")
        return 1
    print("[check_g11_acceptance_map] SELFTEST PASS (14 RED + 1 GREEN)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true", help="用受控负样本证明断言能红")
    args = parser.parse_args()
    if args.selftest:
        return run_selftest()

    for path in (MAP_PATH, CONTRACT_PATH, CI_GATES_PATH):
        if not path.exists():
            print(f"[check_g11_acceptance_map] FAIL — 缺事实源 {path.relative_to(ROOT)}")
            return 1

    findings = check(
        MAP_PATH.read_text(encoding="utf-8"),
        CONTRACT_PATH.read_text(encoding="utf-8"),
        CI_GATES_PATH.read_text(encoding="utf-8"),
    )
    if findings:
        print(f"[check_g11_acceptance_map] FAIL ({len(findings)} 项)")
        for f in findings:
            print(f"  - {f}")
        return 1
    print(
        "[check_g11_acceptance_map] PASS"
        "（13 P0 + 1 已 go P1（M157）覆盖齐备；14 key 唯一且同一命名空间；"
        "P0 行 MAP/CONTRACT/CI_GATES 三向逐字一致（schema 路径 MAP ↔ CI_GATES 逐字 + CONTRACT 形态机核）、"
        "P1 行 MAP §2/CI_GATES §4A 双向逐字一致；零空行/占位；numeric_step 全列 post-interlock 字面零预占）"
    )
    print("  注意：本 PASS 只表示映射完整，不表示任何 P0/P1 能力门已实现或已绿。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
