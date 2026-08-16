#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.1 治理波 validator）
"""G11 波次聚合门共享库(milestones/g11/CI_GATES.md §5;同构 ci/g10_wave_exit_lib.py)。

只读汇总独立门 evidence,不重跑 smoke、不代绿。任一 required gate 缺失 /
非 PASS / SKIP / DEV_ENV_DEGRADE → 聚合红。供 G11.2~G11.7b 薄壳复用。

与 g10 版差异:docstring 对照说明、selftest 临时目录前缀、示例注释 subject 前缀;
判定逻辑(gate_pass_reason / DEVICE_FAIL_STATES)逐字节同构,不改语义。

`--selftest` 直接核验 gate_pass_reason 红绿两臂(负样本:key 漂移/host 假/
device fail 态/checks 非真各为独立红臂;正样本:合形 evidence 判 PASS)。
"""
from __future__ import annotations

import datetime as _dt
import json
import platform
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
DEFERRED_PATH = ROOT / "registry" / "deferred.json"

# UTC stamp in evidence filenames: g11_<slug>_YYYYMMDDTHHMMSSZ.json
_UTC_STAMP_RE = re.compile(r"_(\d{8}T\d{6}Z)\.json$")

DEVICE_FAIL_STATES = frozenset({"fail", "dev_env_degrade", "SKIP", "skip"})


def utc_stamp() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def load_latest_evidence(subject_prefix: str, evidence_dir: Path | None = None) -> Path | None:
    """evidence/ 内按 UTC 文件名取该 subject 最新一份。

    subject_prefix 例如 ``g11_m154_fix_r4_gi_multibounce_world_cache``(匹配
    ``g11_m154_fix_r4_gi_multibounce_world_cache_<UTC>.json``)。
    """
    base = evidence_dir if evidence_dir is not None else EVIDENCE_DIR
    if not base.is_dir():
        return None
    candidates: list[tuple[str, Path]] = []
    for p in base.glob(f"{subject_prefix}_*.json"):
        m = _UTC_STAMP_RE.search(p.name)
        if m is None:
            continue
        candidates.append((m.group(1), p))
    if not candidates:
        return None
    candidates.sort(key=lambda t: t[0])
    return candidates[-1][1]


def validate_schema(evidence: dict[str, Any], schema_path: Path) -> list[str]:
    """Draft-07 校验;返回错误列表(空=通过)。"""
    try:
        import jsonschema
    except ImportError:
        return ["缺 jsonschema 依赖(pip install -r requirements.txt)"]
    if not schema_path.is_file():
        return [f"schema 缺失: {schema_path}"]
    schema = load_json(schema_path)
    validator = jsonschema.Draft7Validator(schema)
    return [e.message for e in sorted(validator.iter_errors(evidence), key=lambda e: list(e.path))]


def gate_pass_reason(evidence: dict[str, Any], expected_key: str) -> tuple[bool, str]:
    """判定一份 evidence 是否构成该 symbolic key 的 PASS。

    规则(只读、不代绿):
      - symbolic_gate_key == expected_key
      - host_section_pass is True
      - checks 全部为 True(若存在)
      - device_section_state ∉ {fail, dev_env_degrade, skip}
    """
    got = evidence.get("symbolic_gate_key")
    if got != expected_key:
        return False, f"symbolic_gate_key={got!r} ≠ {expected_key!r}"
    if evidence.get("host_section_pass") is not True:
        return False, f"host_section_pass={evidence.get('host_section_pass')!r} (要求 True)"
    device = evidence.get("device_section_state")
    if device in DEVICE_FAIL_STATES:
        return False, f"device_section_state={device!r} (FAIL/SKIP/degrade)"
    checks = evidence.get("checks")
    if isinstance(checks, dict):
        bad = [k for k, v in checks.items() if v is not True]
        if bad:
            return False, f"checks 非真: {bad}"
    return True, "PASS"


def require_gate_pass(
    key: str,
    subject_prefix: str,
    evidence_dir: Path | None = None,
) -> dict[str, Any]:
    """只读核验一门;返回 row(不含重跑)。"""
    path = load_latest_evidence(subject_prefix, evidence_dir=evidence_dir)
    row: dict[str, Any] = {
        "symbolic_gate_key": key,
        "subject_prefix": subject_prefix,
        "evidence_path": None if path is None else str(path.relative_to(ROOT)).replace("\\", "/"),
        "status": "FAIL",
        "detail": "",
    }
    if path is None:
        row["detail"] = f"缺最新 evidence({subject_prefix}_*.json)"
        return row
    try:
        evidence = load_json(path)
    except (OSError, json.JSONDecodeError) as e:
        row["detail"] = f"evidence 不可读: {e}"
        return row
    ok, detail = gate_pass_reason(evidence, key)
    row["status"] = "PASS" if ok else "FAIL"
    row["detail"] = detail
    row["timestamp"] = evidence.get("timestamp")
    row["device_section_state"] = evidence.get("device_section_state")
    row["host_section_pass"] = evidence.get("host_section_pass")
    return row


def load_rd_status(rd_id: str, deferred_path: Path | None = None) -> str | None:
    path = deferred_path if deferred_path is not None else DEFERRED_PATH
    data = load_json(path)
    entries = data.get("entries") or []
    for e in entries:
        if e.get("id") == rd_id:
            return e.get("status")
    return None


def rfc_agent_approved(rfc_path: Path) -> tuple[bool, str]:
    """核验 RFC 正文含 Agent Approved 字面(不代绿实现)。"""
    if not rfc_path.is_file():
        return False, f"RFC 缺失: {rfc_path}"
    text = rfc_path.read_text(encoding="utf-8")
    if "Agent Approved" not in text:
        return False, "正文无 'Agent Approved' 字面"
    # 取状态行旁证
    for line in text.splitlines()[:40]:
        if "Agent Approved" in line and ("状态" in line or "批准" in line or "status" in line.lower()):
            return True, line.strip()
    return True, "正文含 Agent Approved"


def collect_environment() -> dict[str, str]:
    env = {
        "os": platform.platform(),
        "python_version": sys.version.split()[0],
        "cargo_version": "",
        "rustc_version": "",
    }
    for tool, key in (("cargo", "cargo_version"), ("rustc", "rustc_version")):
        try:
            r = subprocess.run(
                [tool, "--version"],
                cwd=ROOT,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if r.returncode == 0 and r.stdout.strip():
                env[key] = r.stdout.strip().splitlines()[0]
        except (OSError, subprocess.TimeoutExpired):
            pass
    return env


def emit_wave_evidence(
    *,
    wave: str,
    subject: str,
    symbolic_gate_key: str,
    numeric_step: int,
    source_ref: str,
    required_gate_rows: list[dict[str, Any]],
    extra_facts: list[dict[str, Any]],
    subjects: list[dict[str, Any]] | None,
    schema_path: Path,
    evidence_basename: str,
    notes: str,
    host_section_pass: bool,
) -> tuple[int, Path]:
    """落盘 wave evidence + 逐行打印;返回 (exit_code, path)。"""
    stamp = utc_stamp()
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out_path = EVIDENCE_DIR / f"{evidence_basename}_{stamp}.json"

    gate_ok = all(r.get("status") == "PASS" for r in required_gate_rows)
    fact_ok = all(f.get("status") == "PASS" for f in extra_facts)
    subj_ok = True if not subjects else all(s.get("status") == "PASS" for s in subjects)
    overall = gate_ok and fact_ok and subj_ok and host_section_pass

    payload: dict[str, Any] = {
        "schema_version": 1,
        "subject": subject,
        "symbolic_gate_key": symbolic_gate_key,
        "matrix_row": wave,
        "wave": wave,
        "numeric_step": numeric_step,
        "source_ref": source_ref,
        "host_section_pass": overall,
        "device_section_state": "not_applicable",
        "required_gates": required_gate_rows,
        "extra_facts": extra_facts,
        "subjects": subjects or [],
        "checks": {
            "all_required_gates_pass": gate_ok,
            "all_extra_facts_pass": fact_ok,
            "all_subjects_pass": subj_ok,
            "aggregate_read_only": True,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": collect_environment(),
        "notes": notes,
    }

    errs = validate_schema(payload, schema_path)
    if errs:
        print(f"[wave_exit] schema FAIL ({schema_path.name}):", file=sys.stderr)
        for e in errs:
            print(f"  - {e}", file=sys.stderr)
        # 仍落盘以便调试,但 exit 1
        out_path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"[wave_exit] evidence (schema-invalid) → {out_path}")
        return 1, out_path

    out_path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    print(f"[wave_exit] {symbolic_gate_key} (step {numeric_step})")
    for r in required_gate_rows:
        print(f"  GATE  {r['status']:4}  {r['symbolic_gate_key']}  ({r.get('detail','')})")
    for f in extra_facts:
        print(f"  FACT  {f['status']:4}  {f.get('id', f.get('name', '?'))}  ({f.get('detail','')})")
    for s in subjects or []:
        print(f"  SUBJ  {s['status']:4}  {s.get('id', s.get('name', '?'))}  ({s.get('detail','')})")
    print(f"  → evidence {out_path.relative_to(ROOT)}")
    print(f"  VERDICT = {'PASS' if overall else 'FAIL'}")
    return (0 if overall else 1), out_path


def run_selftest_missing_gate(
    *,
    make_runner: Callable[[Path], Callable[[], int]],
) -> int:
    """负样本:临时空 evidence 目录 → 聚合必须红。"""
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g11_wave_exit_selftest_") as td:
        empty = Path(td)
        runner = make_runner(empty)
        code = runner()
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 时聚合仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 聚合红")
        return 0


def run_selftest() -> int:
    """gate_pass_reason 红绿两臂自检(不依赖树上 evidence)。"""
    key = "g11.p0.m154.fix_r4_gi_multibounce_world_cache"
    good = {
        "symbolic_gate_key": key,
        "host_section_pass": True,
        "device_section_state": "executed",
        "checks": {"a": True, "b": True},
    }
    failures = 0

    def red(name: str, ev: dict) -> None:
        nonlocal failures
        ok, detail = gate_pass_reason(ev, key)
        if not ok:
            print(f"  RED ok   — {name}（{detail}）")
        else:
            print(f"  RED MISS — {name}：负样本被判 PASS")
            failures += 1

    red("key 漂移", {**good, "symbolic_gate_key": "g11.p0.m155.ab_retest_closure"})
    red("host_section_pass 非 True", {**good, "host_section_pass": False})
    red("device fail 态", {**good, "device_section_state": "fail"})
    red("device dev_env_degrade 态", {**good, "device_section_state": "dev_env_degrade"})
    red("device SKIP 态", {**good, "device_section_state": "SKIP"})
    red("checks 含非真", {**good, "checks": {"a": True, "b": False}})

    ok, detail = gate_pass_reason(good, key)
    if ok and detail == "PASS":
        print("  GREEN ok — 合形 evidence 判 PASS")
    else:
        print(f"  GREEN MISS — 合形 evidence 本应 PASS，实测 {detail}")
        failures += 1

    if failures:
        print(f"[g11_wave_exit_lib] SELFTEST FAIL ({failures})")
        return 1
    print("[g11_wave_exit_lib] SELFTEST PASS (6 RED + 1 GREEN)")
    return 0


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        sys.exit(run_selftest())
    print(__doc__)
    sys.exit(0)
