#!/usr/bin/env python3
"""G8.1 measured baseline 守卫（milestones/g8/CI_GATES.md §3 `g8.gov.measured_baseline`）。

判据（G8_PLAN §5 第 3/5 条、G8_CONTRACT D-G8-5、CI_GATES §8）：

1. `milestones/g8/g8_budget.json` 的 `entries` 非空；
2. 每条 id 以 `g8.` 前缀，`evidence` 必须为 `measured_local`，零 `estimated`/`unlocked`；
3. `skip_reason` 必须为 null，`measured_value`/`threshold` 均在位且方向自洽；
4. `evidence_file` 必须真实存在，且其 `evidence_level=measured_local`；
5. budget 的 `measured_value` 必须能在 evidence 中找到同值事实源（禁止 JSON 里手写数字）；
6. 诚实边界：evidence 的 metric 名必须出现在 entry id 中（不得把 host CPU 计时冒充 GPU frame time）。

`--selftest` 用受控负样本证明每条断言都能红。本守卫不判定任何实现门为绿。
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUDGET = ROOT / "milestones/g8/g8_budget.json"


def _numbers(node) -> set[float]:
    out: set[float] = set()
    if isinstance(node, bool):
        return out
    if isinstance(node, (int, float)):
        out.add(round(float(node), 6))
    elif isinstance(node, dict):
        for v in node.values():
            out |= _numbers(v)
    elif isinstance(node, list):
        for v in node:
            out |= _numbers(v)
    return out


def check(budget: dict, load_evidence) -> list[str]:
    findings: list[str] = []
    if budget.get("namespace") != "g8":
        findings.append(f"namespace 必须为 g8，实测 {budget.get('namespace')!r}")
    entries = budget.get("entries") or []
    if not entries:
        findings.append("entries 为空 —— G8.1 首个实现 PR 前必须非空 measured（R-G8-8 预算空壳止损）")
    for entry in entries:
        eid = entry.get("id", "<无 id>")
        if not str(eid).startswith("g8."):
            findings.append(f"{eid}: id 必须以 g8. 前缀")
        level = entry.get("evidence")
        if level != "measured_local":
            findings.append(f"{eid}: evidence = {level!r}，要求 measured_local（零 estimated/unlocked）")
        if entry.get("skip_reason") is not None:
            findings.append(f"{eid}: skip_reason 必须为 null，实测 {entry['skip_reason']!r}")
        measured, threshold = entry.get("measured_value"), entry.get("threshold")
        if measured is None or threshold is None:
            findings.append(f"{eid}: measured_value/threshold 缺一")
            continue
        direction = entry.get("direction")
        if direction == "max" and not measured <= threshold:
            findings.append(f"{eid}: direction=max 但 measured {measured} > threshold {threshold}")
        elif direction == "min" and not measured >= threshold:
            findings.append(f"{eid}: direction=min 但 measured {measured} < threshold {threshold}")
        elif direction not in ("max", "min"):
            findings.append(f"{eid}: direction = {direction!r} 非法")

        ev_rel = entry.get("evidence_file")
        if not ev_rel:
            findings.append(f"{eid}: 缺 evidence_file")
            continue
        ev = load_evidence(ev_rel)
        if ev is None:
            findings.append(f"{eid}: evidence_file {ev_rel} 不存在")
            continue
        if ev.get("evidence_level") != "measured_local":
            findings.append(f"{eid}: evidence {ev_rel} 的 evidence_level = {ev.get('evidence_level')!r}")
        if round(float(measured), 6) not in _numbers(ev):
            findings.append(
                f"{eid}: measured_value {measured} 在 evidence {ev_rel} 中找不到同值事实源（禁手写数字）"
            )
        metric = (ev.get("results") or {}).get("metric")
        if metric and metric not in str(eid):
            findings.append(
                f"{eid}: evidence metric {metric!r} 未出现在 entry id 中（诚实边界：不得改述被测对象）"
            )
    return findings


def _loader(base: Path):
    def load(rel: str):
        path = base / rel
        if not path.exists():
            return None
        return json.loads(path.read_text(encoding="utf-8"))

    return load


def run_selftest() -> int:
    budget = json.loads(BUDGET.read_text(encoding="utf-8"))
    load = _loader(ROOT)
    cases: list[tuple[str, dict, str]] = []

    empty = copy.deepcopy(budget)
    empty["entries"] = []
    cases.append(("entries 清空 → 必须红", empty, "entries 为空"))

    estimated = copy.deepcopy(budget)
    estimated["entries"][0]["evidence"] = "estimated"
    cases.append(("evidence 改 estimated → 必须红", estimated, "要求 measured_local"))

    skipped = copy.deepcopy(budget)
    skipped["entries"][0]["skip_reason"] = "no gpu"
    cases.append(("写入 skip_reason → 必须红", skipped, "skip_reason 必须为 null"))

    handwritten = copy.deepcopy(budget)
    handwritten["entries"][0]["measured_value"] = 123.456
    cases.append(("measured 手写数字 → 必须红", handwritten, "找不到同值事实源"))

    overthreshold = copy.deepcopy(budget)
    overthreshold["entries"][0]["threshold"] = 1.0
    cases.append(("阈值低于实测 → 必须红", overthreshold, "measured"))

    lost_evidence = copy.deepcopy(budget)
    lost_evidence["entries"][0]["evidence_file"] = "evidence/does_not_exist.json"
    cases.append(("evidence 文件缺失 → 必须红", lost_evidence, "不存在"))

    renamed = copy.deepcopy(budget)
    renamed["entries"][0]["id"] = "g8.bench.gpu_frame_p95"
    cases.append(("把 host 计时改述为 GPU 帧 → 必须红", renamed, "未出现在 entry id 中"))

    failures = 0
    for name, mutated, expect in cases:
        got = check(mutated, load)
        hit = [f for f in got if expect in f]
        if hit:
            print(f"  RED ok   — {name}（{hit[0]}）")
        elif got:
            print(f"  RED WRONG— {name}：判红但原因不符，实测 {got[:2]}")
            failures += 1
        else:
            print(f"  RED MISS — {name}：负样本未被判红")
            failures += 1

    green = check(budget, load)
    if green:
        print("  GREEN MISS — 当前树本应 PASS：")
        for f in green:
            print(f"    - {f}")
        failures += 1
    else:
        print("  GREEN ok — 当前树 PASS")
    if failures:
        print(f"[check_g8_budget_baseline] SELFTEST FAIL ({failures})")
        return 1
    print(f"[check_g8_budget_baseline] SELFTEST PASS ({len(cases)} RED + 1 GREEN)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest:
        return run_selftest()

    if not BUDGET.exists():
        print(f"[check_g8_budget_baseline] FAIL — 缺 {BUDGET.relative_to(ROOT)}")
        return 1
    budget = json.loads(BUDGET.read_text(encoding="utf-8"))
    findings = check(budget, _loader(ROOT))
    if findings:
        print(f"[check_g8_budget_baseline] FAIL ({len(findings)} 项)")
        for f in findings:
            print(f"  - {f}")
        return 1
    n = len(budget.get("entries") or [])
    print(f"[check_g8_budget_baseline] PASS（{n} 条 measured_local entry，零 estimated/skip，evidence 可追溯）")
    print("  注意：baseline 只证明测量已建立，不证明任何实现达标或任何能力门已绿。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
