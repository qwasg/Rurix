#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G25.1 治理波）
"""G25.1 治理门 — 验收映射覆盖 / 空行 / 双向命名空间一致性（g25.wave.1.acceptance_map，步骤 429）。

核验 `milestones/g25/G25_ACCEPTANCE_MAP.md`：
§1 五行 P0（{M-a..M-e} 闭集全等）+ §2 零 go P1 空集断言 +
全部 symbolic key 匹配 `g25.p0.m_<a~e>.<slug>` 单一命名空间（key 的 m 段字母与行号一致、
脚本命令 `ci/g25_*_smoke.py` 每行单一且 --gate 参数 == canonical key、evidence schema
`g25_m_<a~e>_<key slug>_evidence_schema.json` 与 key 末段同 slug）+
波次 ∈ {G25.2,G25.3,G25.4} +
numeric_step 全列 `post-interlock actual-next-free allocation` 字面零预占 +
零空行/占位 + **双向一致**：MAP §1 与 G25_CONTRACT.md §4.2 对同一 P0 M 行给出的
判据与波次逐字相等（G24 同构体例）。

只读文档，不代绿实现门；本门 PASS 只表示映射完整，不表示任何 P0 能力已实现。
用法：
  py -3 ci/g25_acceptance_map_check.py --gate g25.wave.1.acceptance_map
  py -3 ci/g25_acceptance_map_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g25.wave.1.acceptance_map"
NUMERIC_STEP = 429  # 落盘前实测 registry/number_ledger.json CI_step.next_free=429 顺位领取
SUBJECT = "g25_acceptance_map_check"
WAVE = "G25.1"
MAP_PATH = ROOT / "milestones" / "g25" / "G25_ACCEPTANCE_MAP.md"
CONTRACT_PATH = ROOT / "milestones" / "g25" / "G25_CONTRACT.md"
SCHEMA_PATH = ROOT / "milestones" / "g25" / "g25_acceptance_map_check_evidence_schema.json"
SOURCE_REF = (
    "G25_CONTRACT G-G25-1/§4.2;G25_ACCEPTANCE_MAP.md §1/§2/§4/§5;"
    "G24_P2_DECISIONS.md §1 defer-to-G26+ 九行 + G24_CONTRACT.md §8.7 承接锚 + "
    "用户战役指令（帮我一次性完成G19-G25）P0 清单 M-a~M-e"
)

EXPECTED_P0 = {"M-a", "M-b", "M-c", "M-d", "M-e"}
ALLOWED_WAVES = {"G25.2", "G25.3", "G25.4"}
NUMERIC_STEP_LITERAL = "post-interlock actual-next-free allocation"

KEY_RE = re.compile(r"^g25\.p0\.m_[a-e]\.[a-z0-9_]+$")
KEY_IN_CELL_RE = re.compile(r"`(g25\.p0\.m_[a-e]\.[a-z0-9_]+)`")
SECTION_RE = re.compile(r"^## (\d+)\. ")
SCRIPT_RE = re.compile(r"ci/g25_[a-z0-9_]+_smoke\.py")
SCHEMA_RE = re.compile(r"`(milestones/g25/g25_(m_[a-e])_[a-z0-9_]+_evidence_schema\.json)`")
BOLD_RE = re.compile(r"\*\*([^*]+)\*\*")
MAP_ROW_RE = re.compile(r"^\|\s*\*\*(M-[a-e])\*\*")
CONTRACT_ROW_RE = re.compile(r"^\|\s*\*\*(M-[a-e])\*\*")
PLACEHOLDERS = ("TBD", "TODO", "待定", "待补", "待填", "—", "N/A")


def _cells(line: str) -> list[str]:
    return [c.strip() for c in line.strip().strip("|").split("|")]


def section_lines(text: str, section_no: int) -> list[str]:
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
    rows: dict[str, dict] = {}
    for line in lines:
        if not MAP_ROW_RE.match(line.strip()):
            continue
        cells = _cells(line)
        if len(cells) < 7:
            continue
        m = re.match(r"\*\*(M-[a-e])\*\*", cells[0])
        if not m:
            continue
        rows[m.group(1)] = {
            "raw_key_cell": cells[1],
            "keys": KEY_IN_CELL_RE.findall(cells[1]),
            "script_cell": cells[2],
            "scripts": SCRIPT_RE.findall(cells[2]),
            "schema": cells[3],
            "criteria": cells[4],
            "wave": _wave_of(cells[5]),
            "numeric_step": cells[6].replace("**", "").strip(),
        }
    return rows


def parse_contract_rows(text: str) -> dict[str, dict]:
    out: dict[str, dict] = {}
    for line in text.splitlines():
        m_m = CONTRACT_ROW_RE.match(line.strip())
        if not m_m:
            continue
        cells = _cells(line)
        if len(cells) < 3 or cells[0] != f"**{m_m.group(1)}**":
            continue
        out[m_m.group(1)] = {"criteria": cells[1], "wave": _wave_of(cells[2])}
    return out


def check_row(m: str, row: dict, seen_keys: dict[str, str], seen_schemas: dict[str, str]) -> list[str]:
    findings: list[str] = []
    m_seg = m.lower().replace("-", "_")
    if len(row["keys"]) != 1:
        findings.append(f"{m} 必须恰有一个 canonical symbolic key，实测 {row['keys']}")
        return findings
    key = row["keys"][0]
    if not KEY_RE.match(key):
        findings.append(f"{m} key `{key}` 不匹配 g25.p0.m_<a~e>.<slug>")
    if key.split(".")[2] != m_seg:
        findings.append(f"{m} key `{key}` 的 m 段与行号不符")
    if key in seen_keys:
        findings.append(f"key `{key}` 被 {seen_keys[key]} 与 {m} 共用")
    seen_keys[key] = m
    slug = key.split(".")[3]
    if not row["scripts"]:
        findings.append(f"{m} 缺 `ci/g25_*_smoke.py` 脚本命令")
    if len(set(row["scripts"])) > 1:
        findings.append(f"{m} 一个 key 只能绑定一个脚本，实测 {sorted(set(row['scripts']))}")
    gates = [g.strip("`") for g in re.findall(r"--gate\s+(\S+)", row["script_cell"])]
    if not gates:
        findings.append(f"{m} 脚本命令缺 --gate 参数")
    for gate in gates:
        if gate != key:
            findings.append(f"{m} --gate `{gate}` ≠ canonical key `{key}`")
    for label, value in (
        ("脚本命令", row["script_cell"]),
        ("schema", row["schema"]),
        ("判据", row["criteria"]),
        ("波次", row["wave"]),
        ("numeric_step", row["numeric_step"]),
    ):
        if not value.strip() or value in PLACEHOLDERS:
            findings.append(f"{m} 的 {label} 列为空或占位（实测 {value!r}）")
        elif any(p in value for p in PLACEHOLDERS[:5]):
            findings.append(f"{m} 的 {label} 列含占位记号（实测 {value!r}）")
    schema_m = SCHEMA_RE.search(row["schema"])
    if not schema_m:
        findings.append(f"{m} schema 路径不符 g25_m_<a~e>_<slug>_evidence_schema.json：{row['schema']!r}")
    else:
        path, schema_m_no = schema_m.group(1), schema_m.group(2)
        if schema_m_no != m_seg:
            findings.append(f"{m} schema 路径的 m 段不符：{path}")
        expected_schema = f"milestones/g25/g25_{m_seg}_{slug}_evidence_schema.json"
        if path != expected_schema:
            findings.append(f"{m} schema 路径 slug 与 key 末段不同字面：`{path}` ≠ `{expected_schema}`")
        if path in seen_schemas:
            findings.append(f"schema 路径 {path} 被 {seen_schemas[path]} 与 {m} 共用")
        seen_schemas[path] = m
    if row["wave"] not in ALLOWED_WAVES:
        findings.append(f"{m} 波次 {row['wave']!r} 不在允许集合 {{G25.2,G25.3,G25.4}} 内")
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
        return [{"id": "file", "status": "FAIL", "detail": "G25_ACCEPTANCE_MAP.md 缺失（诚实红，不假绿）"}]
    if contract_text is None:
        return [{"id": "file", "status": "FAIL", "detail": "G25_CONTRACT.md 缺失（诚实红，不假绿）"}]

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
        "detail": "§2 零 go P1 空集（G25.1 字面）" if p1_ok else f"§2 出现 P1 行 {sorted(p1_rows)}（G25.1 零 go P1 字面违例）",
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
        if len(row["keys"]) == 1:
            other = contract_rows.get(m)
            if other is None:
                two.append("G25_CONTRACT §4.2 缺行")
            else:
                if other["criteria"] != row["criteria"]:
                    two.append("判据漂移：MAP §1 判据 ≠ CONTRACT §4.2（逐字不一致）")
                if other["wave"] != row["wave"]:
                    two.append(f"波次漂移：MAP `{row['wave']}` vs CONTRACT `{other['wave']}`")
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
        print(f"[g25_acceptance_map] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=[],
        extra_facts=results,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="G25.1 治理门——验收映射覆盖/空行/双向命名空间一致性：5 P0（M-a~M-e）闭集全等 + §2 零 go P1 空集 + key/脚本/schema 单一命名空间同 slug + numeric_step 全列 post-interlock 字面零预占 + MAP §1 ↔ CONTRACT §4.2 判据/波次双向逐字一致；本门 PASS 只表示映射完整，不表示任何 P0 能力已实现",
        host_section_pass=overall_ok,
    )
    return code


CANONICAL_ROWS = [
    ("M-a", "quality_final_state_verification", "ci/g25_quality_final_state_verification_smoke.py", "G25.2"),
    ("M-b", "fps_parity_final_verdict", "ci/g25_fps_parity_final_verdict_smoke.py", "G25.2"),
    ("M-c", "campaign_full_chain_no_regression", "ci/g25_campaign_full_chain_no_regression_smoke.py", "G25.3"),
    ("M-d", "campaign_handover_ledger", "ci/g25_campaign_handover_ledger_smoke.py", "G25.4"),
    ("M-e", "closed_gate_no_regression", "ci/g25_closed_gate_no_regression_smoke.py", "G25.4"),
]


def _fixture() -> tuple[str, str]:
    map_lines = ["# fixture G25_ACCEPTANCE_MAP", "", "## 1. P0 硬门（精确 5 行）", ""]
    contract_lines = ["# fixture G25_CONTRACT", "", "### 4.2 五行 P0", ""]
    for m, slug, script, wave in CANONICAL_ROWS:
        key = f"g25.p0.{m.lower().replace('-', '_')}.{slug}"
        schema = f"milestones/g25/g25_{m.lower().replace('-', '_')}_{slug}_evidence_schema.json"
        cmd = f"`py -3 {script} --gate {key}`"
        map_lines.append(
            f"| **{m}** | `{key}` | {cmd} | `{schema}` | 合成判据 {m} | **{wave}** | {NUMERIC_STEP_LITERAL} |"
        )
        contract_lines.append(f"| **{m}** | 合成判据 {m} | {wave} |")
    map_lines += ["", "## 2. 已 go P1 硬门（零行）", "", "G25.1 无 go 的 P1 行。", ""]
    return "\n".join(map_lines), "\n".join(contract_lines)


def run_selftest() -> int:
    failures = 0
    map_text, contract_text = _fixture()

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
            "MAP 单侧改写 M-c 判据 → two_way 必须红",
            map_text.replace("合成判据 M-c", "合成判据 M-c 改写", 1),
            contract_text,
            "two_way_M-c",
        ),
        (
            "--gate 参数漂移 → row 必须红",
            map_text.replace("--gate g25.p0.m_c.campaign_full_chain_no_regression`", "--gate g25.p0.m_c.rd045_drift`"),
            contract_text,
            "row_M-c",
        ),
        (
            "numeric_step 列填入数字 → 预占必须红",
            map_text.replace(
                f"**G25.2** | {NUMERIC_STEP_LITERAL} |",
                "**G25.2** | 336 |",
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
            "波次改写为非法 G25.5 → 必须红",
            map_text.replace("合成判据 M-a | **G25.2**", "合成判据 M-a | **G25.5**", 1),
            contract_text,
            "row_M-a",
        ),
        (
            "§2 注入 P1 行 → coverage_p1_empty 必须红",
            map_text.replace(
                "G25.1 无 go 的 P1 行。",
                f"| **M-a** | `g25.p0.m_a.quality_final_state_verification` | `py -3 ci/g25_quality_final_state_verification_smoke.py --gate g25.p0.m_a.quality_final_state_verification` | `milestones/g25/g25_m_a_quality_final_state_verification_evidence_schema.json` | 判据 | **G25.2** | {NUMERIC_STEP_LITERAL} |",
            ),
            contract_text,
            "coverage_p1_empty",
        ),
        (
            "CONTRACT 单侧改判据 → two_way 必须红",
            map_text,
            contract_text.replace("合成判据 M-e", "合成判据 M-e 漂移", 1),
            "two_way_M-e",
        ),
        (
            "schema 路径 m 段改写 → 必须红",
            map_text.replace(
                "milestones/g25/g25_m_c_campaign_full_chain_no_regression_evidence_schema.json",
                "milestones/g25/g25_m_d_campaign_full_chain_no_regression_evidence_schema.json",
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
        print(f"[g25_acceptance_map] SELFTEST FAIL ({failures})")
        return 1
    print("[g25_acceptance_map] SELFTEST PASS (9 RED + 1 GREEN + 真表臂)")
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
