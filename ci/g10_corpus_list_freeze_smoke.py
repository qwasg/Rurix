#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.3 波）
"""G10.3 M133 场景清单冻结门冒烟（步骤 175；g10.p1.m133.corpus_list_freeze；
RFC-0027 §4.4；spec/external_reference.md RXS-0383；
G10_ACCEPTANCE_MAP §2 M133 行判据逐字）。

host 纯 host 门（device_section_state=not_applicable）。判据（MAP §2 逐字）：

  场景清单版本化冻结 + 清单 digest 注册在树 + 后续变更只追加修订行（清单全
  场景与 M131 许可登记、M132 加载门行集闭集对账）。清单变更无只追加修订行
  即 RED；未注册 digest 冒充冻结即 RED；清单行集与许可/加载登记不对账即
  RED。ready 场景数下界 ≥2（首发清单基数），空清单/全 not-ready vacuous
  truth 不构 PASS。

RED 语料（milestones/g10/red_fixtures/m133/）：
  red_manifest_in_place_edit.json      原地改清单行（digest 滞留）
  red_unregistered_digest_masquerade.json  未注册 digest 冒充冻结
  red_rowset_mismatch.json             清单行集与许可登记不对账

用法：
  py -3 ci/g10_corpus_list_freeze_smoke.py --gate g10.p1.m133.corpus_list_freeze
  py -3 ci/g10_corpus_list_freeze_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m133_corpus_list_freeze_evidence_schema.json"
RED_FIXTURE_DIR = ROOT / "milestones" / "g10" / "red_fixtures" / "m133"
M132_SUBJECT_PREFIX = "g10_m132_corpus_loading"

sys.path.insert(0, str(ROOT / "ci"))
import g10_corpus_lib as lib  # noqa: E402
import g10_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g10.p1.m133.corpus_list_freeze"
NUMERIC_STEP = 175
SOURCE_REF = "RFC-0027 §4.4;spec/external_reference.md RXS-0383;G10_ACCEPTANCE_MAP §2 M133"
TAG = "g10_m133"
SUBJECT = "g10_m133_corpus_list_freeze"
MATRIX_ROW = "M133"

CHECK_KEYS = [
    "manifest_schema_valid",
    "manifest_digest_registered_in_tree",
    "append_only_revision_program",
    "rowset_reconciles_license_registry",
    "rowset_reconciles_loading_gate",
    "ready_floor_vacuous_guard",
    "red_fixtures_all_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def latest_m132_loaded_scenes() -> tuple[list[str] | None, str]:
    """读最新 M132 evidence 的已加载场景集（只读对账，不重跑）。"""
    path = wel.load_latest_evidence(M132_SUBJECT_PREFIX)
    if path is None:
        return None, "缺 M132 最新 evidence（门序：M132 先跑）"
    try:
        doc = lib.load_json(path)
    except (OSError, json.JSONDecodeError) as e:
        return None, f"M132 evidence 不可读: {e}"
    if doc.get("status") != "pass":
        return None, f"M132 最新 evidence status={doc.get('status')!r} ≠ pass（加载门未绿不构清单对账）"
    scenes = doc.get("scenes")
    if not isinstance(scenes, list):
        return None, "M132 evidence 缺 scenes 面"
    loaded = sorted(s.get("scene_id") for s in scenes if s.get("ok") is True)
    return loaded, str(path.relative_to(ROOT)).replace("\\", "/")


def leg_red_fixtures(registry: dict) -> bool:
    ok_all = True
    fixtures = sorted(RED_FIXTURE_DIR.glob("*.json"))
    if len(fixtures) < 3:
        check(False, f"M133 RED 语料不足 3 件: {len(fixtures)}")
        return False
    for fp in fixtures:
        doc = lib.load_json(fp)
        name = fp.stem
        fails = lib.validate_manifest(doc, registry)
        if name == "red_manifest_in_place_edit":
            if not any("原地改" in f or "digest 与最新修订注册不符" in f for f in fails):
                check(False, f"{name}: 原地改未被 digest 复算检出: {fails[:2]}")
                ok_all = False
        elif name == "red_unregistered_digest_masquerade":
            if not fails:
                check(False, f"{name}: 未注册 digest 冒充未被检出")
                ok_all = False
        elif name == "red_rowset_mismatch":
            if not any("未在许可注册表登记" in f for f in fails):
                check(False, f"{name}: 行集不对账未被检出: {fails[:2]}")
                ok_all = False
        else:
            check(False, f"未知 RED 语料（闭集外）: {name}")
            ok_all = False
    note("M133 RED 三件全检出（原地改/未注册冒充/行集不对账）" if ok_all else "M133 RED 检出不全")
    return ok_all


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 红臂：原地改（行改了但修订 digest 滞留）必被 manifest_scenes_digest 复算检出。
    scenes = [
        {"scene_id": "a", "asset_id": "x", "camera_ref": "c", "lighting_ref": "l", "status": "ready"},
        {"scene_id": "b", "asset_id": "y", "camera_ref": "c", "lighting_ref": "l", "status": "ready"},
    ]
    d1 = lib.manifest_scenes_digest(scenes)
    scenes[0]["status"] = "not-ready"
    if lib.manifest_scenes_digest(scenes) == d1:
        print(f"[{TAG}] selftest FAIL: 原地改后 digest 未漂移", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (2 RED + 1 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}

    registry = lib.load_json(lib.REGISTRY_PATH) if lib.REGISTRY_PATH.is_file() else {}
    manifest = lib.load_json(lib.MANIFEST_PATH) if lib.MANIFEST_PATH.is_file() else {}

    m_fails = lib.validate_manifest(manifest, registry) if manifest and registry else ["清单/注册表缺失"]
    for f in m_fails:
        check(False, f"清单校验: {f}")
    checks["manifest_schema_valid"] = not m_fails

    # 清单 digest 注册在树：清单文件在 git 树内 + 最新修订行 digest == 全量行复算。
    in_git = subprocess.run(
        ["git", "ls-files", "--error-unmatch", "milestones/g10/g10_corpus_scene_manifest.json"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    ).returncode == 0
    if not in_git:
        check(False, "清单文件未在 git 树（digest 注册载体缺失）")
    digest_match = not any("digest 与最新修订注册不符" in f for f in m_fails)
    checks["manifest_digest_registered_in_tree"] = in_git and digest_match

    revisions = manifest.get("revisions", []) if isinstance(manifest, dict) else []
    ids = [r.get("revision") for r in revisions]
    checks["append_only_revision_program"] = (
        bool(revisions)
        and ids == sorted(ids)
        and len(set(ids)) == len(ids)
        and ids[0] == 1
        and all(all(k in r and r[k] not in (None, "") for k in ("revision", "manifest_digest", "changed_at", "change_note")) for r in revisions)
    )
    if not checks["append_only_revision_program"]:
        check(False, f"只追加修订程序违例: revisions={ids}")

    reg_fails = lib.validate_registry(registry) if registry else ["注册表缺失"]
    man_assets = {r.get("asset_id") for r in manifest.get("scenes", [])} if isinstance(manifest, dict) else set()
    reg_assets = {a.get("asset_id") for a in registry.get("assets", [])} if isinstance(registry, dict) else set()
    if not man_assets <= reg_assets:
        check(False, f"清单 asset_id 越出许可注册表: {sorted(man_assets - reg_assets)}")
    checks["rowset_reconciles_license_registry"] = not reg_fails and man_assets <= reg_assets

    loaded, src = latest_m132_loaded_scenes()
    if loaded is None:
        check(False, f"M132 对账: {src}")
        checks["rowset_reconciles_loading_gate"] = False
    else:
        ready = sorted(r["scene_id"] for r in manifest.get("scenes", []) if r.get("status") == "ready")
        if loaded != ready:
            check(False, f"清单 ready 集 ≠ M132 已加载集: {ready} ≠ {loaded}")
        checks["rowset_reconciles_loading_gate"] = loaded == ready
        note(f"M132 对账源: {src}")

    ready_count = sum(1 for r in manifest.get("scenes", []) if r.get("status") == "ready") if isinstance(manifest, dict) else 0
    checks["ready_floor_vacuous_guard"] = ready_count >= 2
    if ready_count < 2:
        check(False, f"ready 场景数 {ready_count} < 2（vacuous 拦截）")

    checks["red_fixtures_all_detected"] = leg_red_fixtures(registry)

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G10.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（清单冻结 + digest 注册在树 + 只追加修订 + 三方对账 + RED 三件全检出）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
