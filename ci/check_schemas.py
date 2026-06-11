"""PR Smoke 步骤 2:注册表/预算/证据 JSON 的 schema 校验(CI_GATES.md §3.2)。

- registry/deferred.json / spike_gating.json:结构字段与编号格式;
- milestones/*/m*_budget.json:结构 + 命名空间强制前缀(14 §3);
- evidence/*.json:对 milestones/m0/evidence_schema.json 做 JSON Schema 校验。
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ERRORS: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def load(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:  # noqa: BLE001 - 报告而非崩溃
        err(f"{path.relative_to(ROOT)}: 无法解析 JSON: {e}")
        return None


def check_deferred(path: Path) -> None:
    data = load(path)
    if data is None:
        return
    seen: set[str] = set()
    for entry in data.get("entries", []):
        eid = entry.get("id", "")
        if not re.fullmatch(r"RD-\d{3}", eid):
            err(f"deferred: 编号格式非法: {eid!r}")
        if eid in seen:
            err(f"deferred: 编号重复: {eid}")
        seen.add(eid)
        for field in ("title", "reason", "backfill_condition", "owner_milestone", "status", "history"):
            if field not in entry:
                err(f"deferred {eid}: 缺字段 {field}")
        if entry.get("status") not in ("open", "inherited", "closed"):
            err(f"deferred {eid}: status 非法: {entry.get('status')!r}")
        if not entry.get("history"):
            err(f"deferred {eid}: history 不得为空(留痕要求,14 §4)")


def check_gating(path: Path) -> None:
    data = load(path)
    if data is None:
        return
    seen: set[str] = set()
    for entry in data.get("entries", []):
        eid = entry.get("id", "")
        if not re.fullmatch(r"SG-\d{3}", eid):
            err(f"spike_gating: 编号格式非法: {eid!r}")
        if eid in seen:
            err(f"spike_gating: 编号重复: {eid}")
        seen.add(eid)
        for field in ("direction", "trigger_condition", "permanence", "current_verdict", "decisions"):
            if field not in entry:
                err(f"spike_gating {eid}: 缺字段 {field}")
        if entry.get("permanence") not in ("permanent", "conditional"):
            err(f"spike_gating {eid}: permanence 非法")
        if not entry.get("decisions"):
            err(f"spike_gating {eid}: decisions 不得为空(留痕要求,14 §7)")


def check_budget(path: Path) -> None:
    data = load(path)
    if data is None:
        return
    ns = data.get("namespace")
    if not ns:
        err(f"{path.name}: 缺 namespace 字段")
        return
    prefix = ns + "."
    ids: set[str] = set()
    groups = ("entries", "ratio_assertions", "counter_assertions")
    for group in groups:
        for entry in data.get(group, []):
            eid = entry.get("id", "")
            if not eid.startswith(prefix):
                err(f"{path.name}: id {eid!r} 未带强制前缀 {prefix!r}(14 §3)")
            if eid in ids:
                err(f"{path.name}: id 重复(命名空间冲突): {eid}")
            ids.add(eid)
    for entry in data.get("entries", []):
        ev = entry.get("evidence")
        if ev not in ("measured_local", "unlocked", "estimated"):
            err(f"{path.name} {entry.get('id')}: evidence 非法: {ev!r}")
        if ev == "estimated" and not entry.get("skip_reason"):
            err(f"{path.name} {entry.get('id')}: estimated 占位必须输出 skip_reason(14 §3)")
        if ev == "measured_local":
            if entry.get("threshold") is None:
                err(f"{path.name} {entry.get('id')}: measured_local 必须有 threshold")
            if not entry.get("evidence_file"):
                err(f"{path.name} {entry.get('id')}: measured_local 必须登记 evidence_file")
            elif not (ROOT / entry["evidence_file"]).is_file():
                err(f"{path.name} {entry.get('id')}: evidence_file 不存在: {entry['evidence_file']}")


def check_evidence_files() -> None:
    schema_path = ROOT / "milestones/m0/evidence_schema.json"
    schema = load(schema_path)
    if schema is None:
        return
    evidence_files = sorted((ROOT / "evidence").glob("*.json"))
    if not evidence_files:
        print("[check_schemas] evidence/ 暂无证据文件(M0.3 前为正常状态)")
        return
    try:
        import jsonschema
    except ImportError:
        err("缺 jsonschema 依赖(pip install -r requirements.txt)")
        return
    validator = jsonschema.Draft7Validator(schema)
    for f in evidence_files:
        doc = load(f)
        if doc is None:
            continue
        for v in validator.iter_errors(doc):
            err(f"evidence/{f.name}: {'/'.join(str(p) for p in v.path)}: {v.message}")


def main() -> int:
    check_deferred(ROOT / "registry/deferred.json")
    check_gating(ROOT / "registry/spike_gating.json")
    for budget in sorted(ROOT.glob("milestones/*/m*_budget.json")):
        check_budget(budget)
    check_evidence_files()
    if ERRORS:
        print("[check_schemas] FAIL")
        for e in ERRORS:
            print(f"  - {e}")
        return 1
    print("[check_schemas] PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
