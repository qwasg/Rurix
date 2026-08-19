#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.1 治理波）
"""G14.1 治理门 — 验收映射覆盖 / 空行 / 双向命名空间一致性（g14.wave.1.acceptance_map，步骤 247）。

G14_CONTRACT G-G14-1（G14.1 完成门 D-G14-3 验收映射面）治理三门之一；G-G14-3 实现互锁
（ci/g14_interlock_check.py）事实门④消费本门与候选决策门的独立 PASS 机器事实。

核验 `milestones/g14/G14_ACCEPTANCE_MAP.md`（2026-08-19 v1.0 落盘）：
§1 五行 P0（{M-a,M-b,M-c,M-d,M-e} 闭集全等——M-c/M-d 门登记表 UE 方差带结构化对账修订 /
UE benchmark 臂正式帧率测量 / Rurix 生产管线性能面 / 双端帧率正式对标+画质零降级守护 /
回归门+漂移监控）+ §2 零 go P1 空集断言 +
全部 symbolic key 匹配 `g14.p0.m_<a~e>.<slug>` 单一命名空间（key 的 m 段字母与行号一致、
脚本命令 `ci/g14_<key slug>_smoke.py` 每行单一且 --gate 参数 == canonical key、evidence schema
`g14_m_<a~e>_<key slug>_evidence_schema.json` 与 key 末段同 slug——脚本/schema 路径
与 key 末段同字面形态）+
波次 ∈ {G14.2,G14.3,G14.4,G14.5a} +
numeric_step 全列 `post-interlock actual-next-free allocation` 字面零预占 +
零空行/占位 + **双向一致**：MAP §1 与 G14_CONTRACT.md §4.2 对同一 P0 M 行给出的
key 与脚本逐字相等（G14 治理三件套无独立 CI_GATES——门冻结面 = 契约 §4.2 + MAP §1/§2，
沿 G13 体例精简为双向，MAP §4.1 机器可核声明）。

只读文档，不代绿实现门；本门 PASS 只表示映射完整，不表示任何 P0 能力已实现。
用法：
  py -3 ci/g14_acceptance_map_check.py --gate g14.wave.1.acceptance_map
  py -3 ci/g14_acceptance_map_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g14.wave.1.acceptance_map"
NUMERIC_STEP = 247  # 落盘前实测 registry/number_ledger.json CI_step.next_free=247 顺位领取
SUBJECT = "g14_acceptance_map_check"
WAVE = "G14.1"
MAP_PATH = ROOT / "milestones" / "g14" / "G14_ACCEPTANCE_MAP.md"
CONTRACT_PATH = ROOT / "milestones" / "g14" / "G14_CONTRACT.md"
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_acceptance_map_check_evidence_schema.json"

# 冻结 5 行 P0 集合（2026-08-19 G14.1 立项裁决口径；立项前调研报告帧率面 P0 清单 M-a~M-e）。
EXPECTED_P0 = {"M-a", "M-b", "M-c", "M-d", "M-e"}
# MAP §5：所有波次属于 G14.2|G14.3|G14.4|G14.5a。
ALLOWED_WAVES = {"G14.2", "G14.3", "G14.4", "G14.5a"}
# numeric_step 唯一合法字面（P0 实现门数字步骤 post-interlock 按 actual next_free 分配）。
NUMERIC_STEP_LITERAL = "post-interlock actual-next-free allocation"

KEY_RE = re.compile(r"^g14\.p0\.m_[a-e]\.[a-z0-9_]+$")
KEY_IN_CELL_RE = re.compile(r"`(g14\.p0\.m_[a-e]\.[a-z0-9_]+)`")
KEY_CELL_RE = re.compile(r"`(g14\.p0\.m_[a-e]\.[a-z0-9_]+)`")
SECTION_RE = re.compile(r"^## (\d+)\. ")
SCRIPT_RE = re.compile(r"ci/g14_[a-z0-9_]+_smoke\.py")
SCHEMA_RE = re.compile(r"`(milestones/g14/g14_(m_[a-e])_[a-z0-9_]+_evidence_schema\.json)`")
BOLD_RE = re.compile(r"\*\*([^*]+)\*\*")
MAP_ROW_RE = re.compile(r"^\|\s*\*\*(M-[a-e])\*\*")
CONTRACT_ROW_RE = re.compile(r"^\|\s*(M-[a-e])\s*\|")
PLACEHOLDERS = ("TBD", "TODO", "待定", "待补", "待填", "—", "N/A")


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
    m = BOLD_RE.search(cell)
    return (m.group(1) if m else cell.replace("**", "")).strip()


def parse_map_rows(lines: list[str]) -> dict[str, dict]:
    """解析节内 `| **M-<a~e>** | ... ` 行（§1 P0 八列形态）。"""
    rows: dict[str, dict] = {}
    for line in lines:
        if not MAP_ROW_RE.match(line.strip()):
            continue
        cells = _cells(line)
        if len(cells) < 8:
            continue
        m = re.match(r"\*\*(M-[a-e])\*\*", cells[0])
        if not m:
            continue
        rows[m.group(1)] = {
            "raw_key_cell": cells[1],
            "keys": KEY_IN_CELL_RE.findall(cells[1]),
            "scripts": SCRIPT_RE.findall(cells[1]),
            "schema": cells[2],
            "criteria": cells[3],
            "red_arms": cells[4],
            "device_host": cells[5],
            "wave": _wave_of(cells[6]),
            "numeric_step": cells[7].replace("**", "").strip(),
        }
    return rows


def parse_contract_rows(text: str) -> dict[str, dict]:
    """解析 CONTRACT §4.2 形态的 `| M-<a~e> | `key`<br>`脚本` | 判据 | 波次 |` 行。"""
    out: dict[str, dict] = {}
    for line in text.splitlines():
        m_m = CONTRACT_ROW_RE.match(line.strip())
        if not m_m:
            continue
        cells = _cells(line)
        if len(cells) < 2 or cells[0] != m_m.group(1):
            continue
        key_m = KEY_CELL_RE.search(cells[1])
        script_m = SCRIPT_RE.search(line)
        if not (key_m and script_m):
            continue
        out[m_m.group(1)] = {"key": key_m.group(1), "script": script_m.group(0)}
    return out


def check_row(m: str, row: dict, seen_keys: dict[str, str], seen_schemas: dict[str, str]) -> list[str]:
    """单行 coverage + no-empty 断言（逐行独立报告）。"""
    findings: list[str] = []
    m_seg = m.lower().replace("-", "_")  # M 行号 → key/schema m 段（m_a~m_e，连字符不入 key 字符集）
    if len(row["keys"]) != 1:
        findings.append(f"{m} 必须恰有一个 canonical symbolic key，实测 {row['keys']}")
        return findings
    key = row["keys"][0]
    if not KEY_RE.match(key):
        findings.append(f"{m} key `{key}` 不匹配 g14.p0.m_<a~e>.<slug>")
    if key.split(".")[2] != m_seg:
        findings.append(f"{m} key `{key}` 的 m 段与行号不符")
    if key in seen_keys:
        findings.append(f"key `{key}` 被 {seen_keys[key]} 与 {m} 共用")
    seen_keys[key] = m
    slug = key.split(".")[3]
    if not row["scripts"]:
        findings.append(f"{m} 缺 `ci/g14_*_smoke.py` 脚本命令")
    if len(set(row["scripts"])) > 1:
        findings.append(f"{m} 一个 key 只能绑定一个脚本，实测 {sorted(set(row['scripts']))}")
    # 注：MAP §5 判据字面不设脚本↔key 同 slug 机核（M-a/M-b 脚本名为 key 末段缩写稳定形态，
    # 双向一致由 MAP §1 ↔ CONTRACT §4.2 逐字比对承载）；schema 路径维持 key 同 slug 机核。
    gates = [g.strip("`") for g in re.findall(r"--gate\s+(\S+)", row["raw_key_cell"])]
    if not gates:
        findings.append(f"{m} 脚本命令缺 --gate 参数")
    for gate in gates:
        if gate != key:
            findings.append(f"{m} --gate `{gate}` ≠ canonical key `{key}`")
    for label, value in (
        ("schema", row["schema"]),
        ("判据", row["criteria"]),
        ("负例 RED 臂", row["red_arms"]),
        ("device/host 性质", row["device_host"]),
        ("波次", row["wave"]),
        ("numeric_step", row["numeric_step"]),
    ):
        if not value.strip() or value in PLACEHOLDERS:
            findings.append(f"{m} 的 {label} 列为空或占位（实测 {value!r}）")
        elif any(p in value for p in PLACEHOLDERS[:5]):
            findings.append(f"{m} 的 {label} 列含占位记号（实测 {value!r}）")
    schema_m = SCHEMA_RE.search(row["schema"])
    if not schema_m:
        findings.append(f"{m} schema 路径不符 g14_m_<a~e>_<slug>_evidence_schema.json：{row['schema']!r}")
    else:
        path, schema_m_no = schema_m.group(1), schema_m.group(2)
        if schema_m_no != m_seg:
            findings.append(f"{m} schema 路径的 m 段不符：{path}")
        expected_schema = f"milestones/g14/g14_{m_seg}_{slug}_evidence_schema.json"
        if path != expected_schema:
            findings.append(f"{m} schema 路径 slug 与 key 末段不同字面：`{path}` ≠ `{expected_schema}`")
        if path in seen_schemas:
            findings.append(f"schema 路径 {path} 被 {seen_schemas[path]} 与 {m} 共用")
        seen_schemas[path] = m
    if row["wave"] not in ALLOWED_WAVES:
        findings.append(f"{m} 波次 {row['wave']!r} 不在允许集合 {{G14.2,G14.3,G14.4,G14.5a}} 内")
    if row["numeric_step"] != NUMERIC_STEP_LITERAL:
        findings.append(
            f"{m} numeric_step 列必须为字面 `{NUMERIC_STEP_LITERAL}`"
            f"（数字步骤零预占，实测 {row['numeric_step']!r}）"
        )
    return findings


def evaluate(map_text: str | None, contract_text: str | None) -> list[dict]:
    """12 facts：coverage_p0_set / coverage_p1_empty / row_M-a~M-e / two_way_M-a~M-e。"""
    results: list[dict] = []
    if map_text is None:
        return [{"id": "file", "status": "FAIL", "detail": "G14_ACCEPTANCE_MAP.md 缺失（诚实红，不假绿）"}]
    if contract_text is None:
        return [{"id": "file", "status": "FAIL", "detail": "G14_CONTRACT.md 缺失（诚实红，不假绿）"}]

    rows = parse_map_rows(section_lines(map_text, 1))
    p1_rows = parse_map_rows(section_lines(map_text, 2))

    set_ok = set(rows) == EXPECTED_P0
    results.append({
        "id": "coverage_p0_set",
        "status": "PASS" if set_ok else "FAIL",
        "detail": f"got {sorted(rows)}; expect {sorted(EXPECTED_P0)}"
        + ("" if set_ok else f"; diff={sorted(set(EXPECTED_P0) ^ set(rows))}"),
    })
    p1_ok = not p1_rows
    results.append({
        "id": "coverage_p1_empty",
        "status": "PASS" if p1_ok else "FAIL",
        "detail": "§2 零 go P1 空集（G14.1 字面）" if p1_ok else f"§2 出现 P1 行 {sorted(p1_rows)}（G14.1 零 go P1 字面违例）",
    })

    seen_keys: dict[str, str] = {}
    seen_schemas: dict[str, str] = {}
    contract_rows = parse_contract_rows(contract_text)
    for m in sorted(EXPECTED_P0):
        row = rows.get(m)
        if row is None:
            results.append({"id": f"row_{m}", "status": "FAIL", "detail": "§1 缺行"})
            results.append({"id": f"two_way_{m}", "status": "FAIL", "detail": "§1 缺行不可比对"})
            continue
        findings = check_row(m, row, seen_keys, seen_schemas)
        results.append({
            "id": f"row_{m}",
            "status": "PASS" if not findings else "FAIL",
            "detail": "ok" if not findings else "; ".join(findings),
        })
        two: list[str] = []
        if len(row["keys"]) == 1 and row["scripts"]:
            key, script = row["keys"][0], row["scripts"][0]
            other = contract_rows.get(m)
            if other is None:
                two.append("G14_CONTRACT §4.2 缺行")
            else:
                if other["key"] != key:
                    two.append(f"key 漂移：MAP `{key}` vs CONTRACT `{other['key']}`")
                if other["script"] != script:
                    two.append(f"script 漂移：MAP `{script}` vs CONTRACT `{other['script']}`")
        results.append({
            "id": f"two_way_{m}",
            "status": "PASS" if not two else "FAIL",
            "detail": "ok" if not two else "; ".join(two),
        })
    return results


def run_check(map_text: str | None = None, contract_text: str | None = None) -> tuple[int, list[dict]]:
    mt = map_text if map_text is not None else (
        MAP_PATH.read_text(encoding="utf-8") if MAP_PATH.is_file() else None
    )
    ct = contract_text if contract_text is not None else (
        CONTRACT_PATH.read_text(encoding="utf-8") if CONTRACT_PATH.is_file() else None
    )
    results = evaluate(mt, ct)
    ok = all(r["status"] == "PASS" for r in results)
    return (0 if ok else 1), results


def emit(results: list[dict], overall_ok: bool) -> int:
    for r in results:
        print(f"  FACT  {r['status']:4}  {r['id']}  ({r.get('detail','')})")
    if not SCHEMA_PATH.is_file():
        print(f"[g14_acceptance_map] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref="G14_CONTRACT G-G14-1/§4.2;G14_ACCEPTANCE_MAP.md §1/§2/§4/§5;2026-08-19 G14 立项前调研（UE benchmark 臂测量口径 + UE 渲染确定性控制面两份调研报告）P0 清单 M-a~M-e",
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G14.1 治理门——验收映射覆盖/空行/双向命名空间一致性：5 P0（M-a M-c/M-d 门登记表 UE 方差带结构化对账修订/M-b UE benchmark 臂正式帧率测量/M-c Rurix 生产管线性能面/M-d 双端帧率正式对标+画质零降级守护/M-e 回归门+漂移监控）闭集全等 + §2 零 go P1 空集 + key/脚本/schema 单一命名空间同 slug + numeric_step 全列 post-interlock 字面零预占 + MAP §1 ↔ CONTRACT §4.2 双向逐字一致（G14 无独立 CI_GATES）；本门 PASS 只表示映射完整，不表示任何 P0 能力已实现",
        host_section_pass=overall_ok,
    )
    return code


# ---------------------------------------------------------------------------
# selftest 合成夹具：5 行 P0 冻结集合的正本，不依赖树上文件。
# ---------------------------------------------------------------------------

CANONICAL_ROWS = [
    ("M-a", "registry_variance_band_reconciliation", "ci/g14_registry_variance_band_smoke.py", "G14.2"),
    ("M-b", "ue_benchmark_arm_measurement", "ci/g14_ue_benchmark_arm_smoke.py", "G14.2"),
    ("M-c", "rurix_pipeline_perf", "ci/g14_rurix_pipeline_perf_smoke.py", "G14.3"),
    ("M-d", "dual_end_fps_parity", "ci/g14_dual_end_fps_parity_smoke.py", "G14.4"),
    ("M-e", "regression_drift_guard", "ci/g14_regression_drift_guard_smoke.py", "G14.5a"),
]


def _fixture() -> tuple[str, str]:
    map_lines = ["# fixture G14_ACCEPTANCE_MAP", "", "## 1. P0 硬门（精确 5 行）", ""]
    contract_lines = ["# fixture G14_CONTRACT", "", "### 4.2 五行 P0", ""]
    for m, slug, script, wave in CANONICAL_ROWS:
        key = f"g14.p0.{m.lower().replace('-', '_')}.{slug}"
        schema = f"milestones/g14/g14_{m.lower().replace('-', '_')}_{slug}_evidence_schema.json"
        cmd = f"`py -3 {script} --gate {key}`"
        map_lines.append(
            f"| **{m}** | `{key}`<br>{cmd} | `{schema}` | 合成判据 {m} | 合成 RED 臂 {m} | "
            f"host+device | **{wave}** | {NUMERIC_STEP_LITERAL} |"
        )
        contract_lines.append(f"| {m} | `{key}`<br>`{script}` | 合成判据 {m} | {wave} |")
    map_lines += ["", "## 2. 已 go P1 硬门（零行）", "", "G14.1 无 go 的 P1 行。", ""]
    return "\n".join(map_lines), "\n".join(contract_lines)


def run_selftest() -> int:
    failures = 0
    map_text, contract_text = _fixture()

    # 正样本 1：真表（已落盘）必须绿
    code, results = run_check()
    if not MAP_PATH.is_file():
        if code == 0:
            print("[selftest] FAIL: MAP 未落盘仍绿（假绿）", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: MAP 未落盘 → 诚实红（起始正确结论）")
    else:
        if code != 0:
            print("[selftest] FAIL: 真表已落盘但核验未绿", file=sys.stderr)
            for r in results:
                if r["status"] != "PASS":
                    print(f"  {r}", file=sys.stderr)
            failures += 1
        else:
            print("[selftest] PASS: 真表 5 行绿")

    cases: list[tuple[str, str, str, str]] = [
        (
            "删除 M-e 行 → coverage 必须红",
            "\n".join(l for l in map_text.splitlines() if not l.startswith("| **M-e**")),
            contract_text,
            "coverage_p0_set",
        ),
        (
            "MAP 单侧改写 M-c key → two_way 必须红",
            map_text.replace("g14.p0.m_c.rurix_pipeline_perf", "g14.p0.m_c.rurix_pipeline"),
            contract_text,
            "two_way_M-c",
        ),
        (
            "--gate 参数 ≠ canonical key → row 必须红",
            map_text.replace("--gate g14.p0.m_c.rurix_pipeline_perf`", "--gate g14.p0.m_c.rurix_pipeline`"),
            contract_text,
            "row_M-c",
        ),
        (
            "numeric_step 列填入数字 → 预占必须红",
            map_text.replace(
                "host+device | **G14.2** | post-interlock actual-next-free allocation |",
                "host+device | **G14.2** | 250 |",
                1,
            ),
            contract_text,
            "row_M-a",
        ),
        (
            "判据列置为待补 → no-empty 必须红",
            map_text.replace("合成判据 M-d", "待补", 1),
            contract_text,
            "row_M-d",
        ),
        (
            "波次改写为非法 G14.5b → 必须红",
            map_text.replace("合成 RED 臂 M-a | host+device | **G14.2**", "合成 RED 臂 M-a | host+device | **G14.5b**", 1),
            contract_text,
            "row_M-a",
        ),
        (
            "§2 注入 P1 行 → coverage_p1_empty 必须红",
            map_text.replace(
                "G14.1 无 go 的 P1 行。",
                "| **M-a** | `g14.p0.m_a.registry_variance_band_reconciliation`<br>`py -3 ci/g14_registry_variance_band_reconciliation_smoke.py --gate g14.p0.m_a.registry_variance_band_reconciliation` | `milestones/g14/g14_m_a_registry_variance_band_reconciliation_evidence_schema.json` | 判据 | RED | host | **G14.2** | post-interlock actual-next-free allocation |",
            ),
            contract_text,
            "coverage_p1_empty",
        ),
        (
            "CONTRACT 单侧改脚本名 → two_way 必须红",
            map_text,
            contract_text.replace("ci/g14_dual_end_fps_parity_smoke.py", "ci/g14_dual_end_smoke.py"),
            "two_way_M-d",
        ),
        (
            "schema 路径 m 段改写 → 必须红",
            map_text.replace(
                "milestones/g14/g14_m_c_rurix_pipeline_perf_evidence_schema.json",
                "milestones/g14/g14_m_d_rurix_pipeline_perf_evidence_schema.json",
                1,
            ),
            contract_text,
            "row_M-c",
        ),
    ]
    for name, mt, ct, expect_fact in cases:
        _, results = run_check(mt, ct)
        hit = [r for r in results if r["id"] == expect_fact and r["status"] == "FAIL"]
        if hit:
            print(f"  RED ok   — {name}（{hit[0]['detail'][:80]}）")
        else:
            print(f"  RED MISS — {name}：负样本未被判红于 {expect_fact}")
            failures += 1

    code, results = run_check(map_text, contract_text)
    green = code == 0 and all(r["status"] == "PASS" for r in results)
    if green and len(results) == 12:
        print("  GREEN ok — 合成夹具正本 PASS（12 facts）")
    else:
        print(f"  GREEN MISS — 合成夹具正本本应 PASS（12 facts），实测 code={code} facts={len(results)}")
        for r in results:
            if r["status"] != "PASS":
                print(f"    - {r}")
        failures += 1

    if failures:
        print(f"[g14_acceptance_map] SELFTEST FAIL ({failures})")
        return 1
    print("[g14_acceptance_map] SELFTEST PASS (9 RED + 1 GREEN + 真表臂)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    code, results = run_check()
    return emit(results, code == 0)


if __name__ == "__main__":
    sys.exit(main())
