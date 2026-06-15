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


def parse_message_keys(path: Path) -> set[str] | None:
    """解析 rurixc 消息表行格式(key = 模板;# 注释),返回 key 集。"""
    if not path.is_file():
        return None
    keys: set[str] = set()
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            err(f"messages: 第 {lineno} 行缺 '=': {line!r}")
            continue
        key = line.split("=", 1)[0].strip()
        if not key or any(c.isspace() for c in key):
            err(f"messages: 第 {lineno} 行 key 非法: {key!r}")
            continue
        if key in keys:
            err(f"messages: key 重复: {key}")
        keys.add(key)
    return keys


def check_error_codes(path: Path) -> None:
    """错误码注册表校验(07 §5 分配制;M1 CI_GATES §2 步骤 11)。"""
    if not path.is_file():
        return  # M1.1 落地前不存在,放行
    data = load(path)
    if data is None:
        return
    message_keys = parse_message_keys(ROOT / "src/rurixc/src/messages/en.messages")
    seen: set[str] = set()
    for entry in data.get("entries", []):
        eid = entry.get("id", "")
        if not re.fullmatch(r"RX\d{4}", eid):
            err(f"error_codes: 编号格式非法: {eid!r}")
        elif eid[2] not in "01234567":
            err(f"error_codes {eid}: 段位非法(0-7,07 §5)")
        if eid in seen:
            err(f"error_codes: 编号重复: {eid}(编号永不复用,10 §9.5)")
        seen.add(eid)
        for field in ("title", "message_key", "status", "introduced_in"):
            if not entry.get(field):
                err(f"error_codes {eid}: 缺字段 {field}")
        if entry.get("status") not in ("active", "deprecated"):
            err(f"error_codes {eid}: status 非法: {entry.get('status')!r}")
        mk = entry.get("message_key")
        if mk and message_keys is not None and mk not in message_keys:
            err(f"error_codes {eid}: message_key 未在 en.messages 注册: {mk!r}")


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
    gpu_schema = load(ROOT / "milestones/m0/evidence_schema.json")
    frontend_schema = load(ROOT / "milestones/m1/frontend_evidence_schema.json")
    compile_schema = load(ROOT / "milestones/m3/compile_evidence_schema.json")
    sanitizer_schema = load(ROOT / "milestones/m5/compute_sanitizer_evidence_schema.json")
    redistribution_schema = load(ROOT / "milestones/m5/redistribution_audit_evidence_schema.json")
    rx_cli_smoke_schema = load(ROOT / "milestones/m6/rx_cli_smoke_evidence_schema.json")
    if (gpu_schema is None or frontend_schema is None or compile_schema is None
            or sanitizer_schema is None or redistribution_schema is None
            or rx_cli_smoke_schema is None):
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
    gpu_validator = jsonschema.Draft7Validator(gpu_schema)
    frontend_validator = jsonschema.Draft7Validator(frontend_schema)
    compile_validator = jsonschema.Draft7Validator(compile_schema)
    sanitizer_validator = jsonschema.Draft7Validator(sanitizer_schema)
    redistribution_validator = jsonschema.Draft7Validator(redistribution_schema)
    rx_cli_smoke_validator = jsonschema.Draft7Validator(rx_cli_smoke_schema)
    for f in evidence_files:
        doc = load(f)
        if doc is None:
            continue
        # 路由(按文件名前缀):frontend_ → m1 前端 schema;compile_ → m3 编译
        # schema(G-M3-3 配套);compute_sanitizer_ → m5 Sanitizer schema
        # (G-M5-4 配套);redistribution_audit_ → m5 再分发审计 schema
        # (CI_GATES §4 第 2 项配套);rx_cli_smoke_ → m6 rx CLI 子命令冒烟 schema
        # (G-M6-3 配套);其余 → m0 GPU schema
        if f.name.startswith("frontend_"):
            validator = frontend_validator
        elif f.name.startswith("compile_"):
            validator = compile_validator
        elif f.name.startswith("compute_sanitizer_"):
            validator = sanitizer_validator
        elif f.name.startswith("redistribution_audit_"):
            validator = redistribution_validator
        elif f.name.startswith("rx_cli_smoke_"):
            validator = rx_cli_smoke_validator
        else:
            validator = gpu_validator
        for v in validator.iter_errors(doc):
            err(f"evidence/{f.name}: {'/'.join(str(p) for p in v.path)}: {v.message}")


def main() -> int:
    check_deferred(ROOT / "registry/deferred.json")
    check_gating(ROOT / "registry/spike_gating.json")
    check_error_codes(ROOT / "registry/error_codes.json")
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
