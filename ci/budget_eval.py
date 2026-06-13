"""PR Smoke 步骤 6:预算 evaluator(14 §3 / CI_GATES.md §3.6)。

- 多预算合并加载 + 命名空间前缀与冲突检测;
- estimated 条目自动 skip 并输出 skip_reason 留痕;
- measured_local 条目:读取 evidence_file,断言 results.trimmed_mean 对 threshold;
- --strict:estimated 即 FAIL(close-out / Release 模式,M0 关闭用,契约 G-M0-1)。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ERRORS: list[str] = []
SKIPS: list[str] = []
PASSES: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def load_budgets() -> dict[str, dict]:
    """合并加载全部预算,命名空间冲突即 FAIL。"""
    merged: dict[str, dict] = {}
    for path in sorted(ROOT.glob("milestones/*/m*_budget.json")):
        doc = json.loads(path.read_text(encoding="utf-8"))
        ns = doc.get("namespace", "")
        for group in ("entries", "ratio_assertions", "counter_assertions"):
            for entry in doc.get(group, []):
                eid = entry["id"]
                if not eid.startswith(ns + "."):
                    err(f"{path.name}: {eid} 未带前缀 {ns}.")
                if eid in merged:
                    err(f"命名空间冲突: {eid}(多预算合并检测,14 §3)")
                merged[eid] = {**entry, "_group": group, "_file": path.name}
    return merged


def measured_value(entry: dict) -> float | None:
    ef = entry.get("evidence_file")
    if not ef or not (ROOT / ef).is_file():
        err(f"{entry['id']}: evidence_file 缺失或不存在: {ef!r}")
        return None
    doc = json.loads((ROOT / ef).read_text(encoding="utf-8"))
    return doc["results"]["trimmed_mean"]


def eval_entry(entry: dict, strict: bool) -> None:
    eid = entry["id"]
    ev = entry.get("evidence")
    if ev == "estimated":
        if strict:
            err(f"{eid}: estimated 占位在严格模式下 FAIL(占位存活规则,14 §3)")
        else:
            SKIPS.append(f"{eid}: SKIP — {entry.get('skip_reason', '(无 skip_reason)')}")
        return
    if ev == "unlocked":
        err(f"{eid}: unlocked 证据不得作为预算断言依据(BENCH_PROTOCOL §2.1)")
        return
    value = measured_value(entry)
    if value is None:
        return
    threshold = entry.get("threshold")
    direction = entry.get("direction", "min")
    ok = value >= threshold if direction == "min" else value <= threshold
    if ok:
        PASSES.append(f"{eid}: PASS — {value:.3f} {entry.get('unit', '')} vs {direction} {threshold}")
    else:
        err(f"{eid}: FAIL — {value:.3f} 违反 {direction} {threshold}")


def eval_ratio(entry: dict, merged: dict[str, dict], strict: bool) -> None:
    eid = entry["id"]
    if entry.get("evidence") == "estimated":
        if strict:
            err(f"{eid}: estimated 占位在严格模式下 FAIL")
        else:
            SKIPS.append(f"{eid}: SKIP — {entry.get('skip_reason', '(无 skip_reason)')}")
        return
    num = merged.get(entry["numerator"])
    den = merged.get(entry["denominator"])
    if not num or not den:
        err(f"{eid}: numerator/denominator 条目不存在")
        return
    nv, dv = measured_value(num), measured_value(den)
    if nv is None or dv is None or dv == 0:
        return
    ratio = nv / dv
    threshold = entry.get("threshold")
    direction = entry.get("direction", "min")
    ok = ratio >= threshold if direction == "min" else ratio <= threshold
    (PASSES if ok else ERRORS).append(
        f"{eid}: {'PASS' if ok else 'FAIL'} — ratio {ratio:.4f} vs {direction} {threshold}"
    )


def count_or_gate(eid: str, n: int, required: int, what: str, pending_hint: str, strict: bool) -> None:
    """M1 计数器通用判定:达标 PASS;未达标 → normal skip(建设期)/ strict FAIL(close-out)。"""
    if n >= required:
        PASSES.append(f"{eid}: PASS — {n} {what}(要求 ≥{required})")
    elif strict:
        err(f"{eid}: FAIL — 仅 {n} {what}(要求 ≥{required})")
    else:
        SKIPS.append(f"{eid}: SKIP — 当前 {n} {what}({pending_hint})")


def eval_counter(entry: dict, strict: bool) -> None:
    """计数器断言:已知 id 逐条实现,未知 id 强制 FAIL(逼迫维护,防僵尸计数器,14 §5)。"""
    eid = entry["id"]
    if eid == "m0.counter.env_profile_required_fields":
        # 字段完整性由 check_schemas.py 对证据文件做 JSON Schema 校验兜底
        PASSES.append(f"{eid}: PASS(delegated to check_schemas.py)")
    elif eid == "m0.counter.evidence_files_saxpy_runs":
        n = 0
        for f in (ROOT / "evidence").glob("saxpy_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("evidence_level") == "measured_local":
                n += 1
        if n >= 3:
            PASSES.append(f"{eid}: PASS — {n} 份 measured_local 证据")
        elif strict:
            err(f"{eid}: FAIL — 仅 {n} 份 measured_local 证据(要求 ≥3,契约 G-M0-1)")
        else:
            SKIPS.append(f"{eid}: SKIP — 当前 {n} 份(M0.3 回填前为正常状态)")
    elif eid == "m1.counter.syntax_corpus_size":
        n = len(list((ROOT / "conformance" / "syntax").glob("**/*.rx")))
        count_or_gate(eid, n, 100, "个语法样例", "M1.2/M1.3 建设期为正常状态,契约 G-M1-1", strict)
    elif eid == "m1.counter.ui_golden_path1_snapshots":
        n = len(list((ROOT / "tests" / "ui").glob("**/*.stderr")))
        count_or_gate(eid, n, 10, "条 .stderr snapshot", "M1.4 建设期为正常状态,契约 G-M1-2", strict)
    elif eid == "m2.counter.ui_golden_path2_snapshots":
        typeck_dir = ROOT / "tests" / "ui" / "typeck"
        n = len(list(typeck_dir.glob("**/*.stderr"))) if typeck_dir.is_dir() else 0
        count_or_gate(eid, n, 10, "条 .stderr snapshot", "M2.2 建设期为正常状态,契约 G-M2-3", strict)
    elif eid == "m3.counter.ui_golden_path3_snapshots":
        borrowck_dir = ROOT / "tests" / "ui" / "borrowck"
        n = len(list(borrowck_dir.glob("**/*.stderr"))) if borrowck_dir.is_dir() else 0
        count_or_gate(eid, n, 10, "条 .stderr snapshot", "M3.3 建设期为正常状态,契约 G-M3-2", strict)
    elif eid == "m3.counter.borrowck_conformance_categories":
        reject_dir = ROOT / "conformance" / "borrowck" / "reject"
        n = len([p for p in reject_dir.iterdir() if p.is_dir()]) if reject_dir.is_dir() else 0
        count_or_gate(eid, n, 7, "个预设错误类别目录", "M3.3 建设期为正常状态,契约 G-M3-1", strict)
    elif eid == "m4.counter.launch_conformance_categories":
        reject_dir = ROOT / "conformance" / "launch" / "reject"
        n = len([p for p in reject_dir.iterdir() if p.is_dir()]) if reject_dir.is_dir() else 0
        count_or_gate(eid, n, 4, "个预设错误类别目录", "M4.3 建设期为正常状态,契约 G-M4-2", strict)
    elif eid == "m4.counter.ui_golden_path4_snapshots":
        # 黄金路径 4 = 目标后端错误:3xxx 着色/地址空间(M4.1)+ 6xxx codegen/ptxas
        # (M4.3),契约 G-M4-3 覆盖两段;计数聚合三目录。
        path4_dirs = ["coloring", "addrspace", "codegen"]
        n = sum(
            len(list((ROOT / "tests" / "ui" / d).glob("**/*.stderr")))
            for d in path4_dirs
            if (ROOT / "tests" / "ui" / d).is_dir()
        )
        count_or_gate(eid, n, 10, "条 .stderr snapshot", "M4.1 3xxx 子集已入,6xxx 随 M4.3,契约 G-M4-3", strict)
    elif eid == "m1.counter.spec_clause_test_anchoring":
        # 条款 ↔ 测试锚定由 traceability 矩阵工具核对(M1.4 交付物,契约 G-M1-4);
        # 矩阵产物落地前 normal skip / strict FAIL,落地后委托其自身校验结果。
        matrix = ROOT / "conformance" / "traceability_matrix.json"
        if matrix.is_file():
            doc = json.loads(matrix.read_text(encoding="utf-8"))
            unanchored = [c for c, tests in doc.get("clauses", {}).items() if not tests]
            if unanchored:
                err(f"{eid}: FAIL — 未锚定条款: {', '.join(sorted(unanchored))}(10 §4)")
            else:
                PASSES.append(f"{eid}: PASS — {len(doc.get('clauses', {}))} 条款全部 ≥1 测试锚定")
        elif strict:
            err(f"{eid}: FAIL — traceability 矩阵不存在(契约 G-M1-4)")
        else:
            SKIPS.append(f"{eid}: SKIP — traceability 矩阵未生成(M1.4 交付物,建设期为正常状态)")
    else:
        err(f"{eid}: 未知计数器断言,无对应 evaluator 实现")


def main() -> int:
    strict = "--strict" in sys.argv
    merged = load_budgets()
    for entry in merged.values():
        group = entry["_group"]
        if group == "entries":
            eval_entry(entry, strict)
        elif group == "ratio_assertions":
            eval_ratio(entry, merged, strict)
        else:
            eval_counter(entry, strict)
    for line in PASSES:
        print(f"  PASS {line}")
    for line in SKIPS:
        print(f"  SKIP {line}")
    if ERRORS:
        print(f"[budget_eval] FAIL ({'strict' if strict else 'normal'} mode)")
        for e in ERRORS:
            print(f"  - {e}")
        return 1
    print(f"[budget_eval] PASS ({len(PASSES)} pass, {len(SKIPS)} skip, {'strict' if strict else 'normal'} mode)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
