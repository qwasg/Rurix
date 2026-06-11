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


def eval_counter(entry: dict, strict: bool) -> None:
    """计数器断言:M0 已知两条,未知 id 强制 FAIL(逼迫维护,防僵尸计数器,14 §5)。"""
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
