#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.3 波）
"""G10.3 M131 许可登记硬门冒烟（步骤 173；g10.p0.m131.asset_license_registry；
RFC-0027 §4.2/§4.3；spec/external_reference.md RXS-0381/RXS-0382；
G10_ACCEPTANCE_MAP §1 M131 行判据逐字）。

host 纯 host 门（正常态 device_section_state=not_applicable；缓存根不可达时
dev_env_degrade 诚实登记不充绿，R-G10-9）。判据：

  逐资产 license 白名单闭集 {CC0-1.0, CC-BY-3.0, CC-BY-4.0} + SPDX id +
  来源 URL + attribution 结构化子字段闭集 + 资产 digest（清单级 canonical，
  获取时实测）；未登记资产混入即 RED；白名单外许可注入即 RED；按类登记缺
  字段即 RED；两类互冒充即 RED；登记 digest 与缓存实算不符即 RED；git 零
  二进制守卫闭集命中即 RED（扩展名闭集 + measured 体积阈值 + magic-bytes，
  白名单路径闭集豁免留痕）。

RED 语料（milestones/g10/red_fixtures/m131/，逐件必须由判定层独立检出）：
  red_whitelist_injection.json   白名单外许可注入（CC-BY-NC-SA-3.0 反例）
  red_missing_source_url.json    external 五元组缺 source_url
  red_class_masquerade.json      generated/external 互冒充
  red_attribution_missing.json   attribution 子字段缺失
  red_digest_tampered.json       登记 digest 篡改（与缓存实算不符）

用法：
  py -3 ci/g10_asset_license_registry_smoke.py --gate g10.p0.m131.asset_license_registry
  py -3 ci/g10_asset_license_registry_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m131_asset_license_registry_evidence_schema.json"
RED_FIXTURE_DIR = ROOT / "milestones" / "g10" / "red_fixtures" / "m131"

sys.path.insert(0, str(ROOT / "ci"))
import g10_corpus_lib as lib  # noqa: E402

GATE_KEY = "g10.p0.m131.asset_license_registry"
NUMERIC_STEP = 173
SOURCE_REF = "RFC-0027 §4.2/§4.3;spec/external_reference.md RXS-0381/RXS-0382;G10_ACCEPTANCE_MAP §1 M131"
TAG = "g10_m131"
SUBJECT = "g10_m131_asset_license_registry"
MATRIX_ROW = "M131"

CHECK_KEYS = [
    "registry_structure_valid",
    "license_snapshots_on_tree",
    "cache_root_resolved",
    "cache_digest_match",
    "cache_dirs_all_registered",
    "git_binary_guard_clean",
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


def leg_structure(doc: dict) -> bool:
    fails = lib.validate_registry(doc)
    for f in fails:
        check(False, f"注册表结构: {f}")
    return not fails


def leg_snapshots(doc: dict) -> bool:
    ok = True
    for row in doc.get("assets", []):
        snap = row.get("license_snapshot")
        aid = row.get("asset_id", "?")
        if snap == "NONE":
            if row.get("class") != "generated":
                check(False, f"{aid} external 类 license_snapshot=NONE")
                ok = False
            continue
        p = ROOT / "milestones" / "g10" / str(snap)
        if not p.is_file():
            check(False, f"{aid} license_snapshot 缺失: {snap}")
            ok = False
        elif p.stat().st_size < 100:
            check(False, f"{aid} license_snapshot 过小（疑似空壳）: {snap}")
            ok = False
        elif row.get("class") == "external":
            text = p.read_text(encoding="utf-8")
            spdx = str(row.get("spdx_id", ""))
            # 快照须含一手页面 License 行摘录锚（creativecommons.org legalcode 锚点）。
            if "creativecommons.org" not in text:
                check(False, f"{aid} 快照缺 creativecommons.org 法律文本锚点")
                ok = False
            if spdx.startswith("CC-BY-4.0") and "Attribution 4.0 International" not in text:
                check(False, f"{aid} 快照缺 CC-BY-4.0 legalcode 标题字面")
                ok = False
    return ok


def leg_cache(doc: dict) -> tuple[bool, bool, bool]:
    """返回 (root_ok, digest_ok, dirs_ok)。"""
    root, src = lib.resolve_cache_root()
    note(f"缓存根解析: {src}")
    if root is None:
        check(False, f"缓存根不可达（fail-closed，禁静默回退）: {src}")
        return False, False, False
    digest_ok = True
    for row in doc.get("assets", []):
        for f in lib.verify_asset_cache(row, root):
            check(False, f"缓存核验: {f}")
            digest_ok = False
    registered = {r.get("asset_id") for r in doc.get("assets", [])}
    dirs_ok = True
    skip_dirs = {"tools"}
    for child in sorted(root.iterdir()):
        if child.is_dir() and child.name not in skip_dirs and child.name not in registered:
            check(False, f"未登记资产混入缓存: {child.name}（未登记资产混入即 RED）")
            dirs_ok = False
    return True, digest_ok, dirs_ok


def leg_git_guard(doc: dict) -> bool:
    fails = lib.git_binary_guard(doc)
    for f in fails:
        check(False, f"git 零二进制守卫: {f}")
    return not fails


def leg_red_fixtures() -> bool:
    """逐件 RED 语料必须由判定层独立检出（RED 先行，红臂有效才谈绿）。"""
    ok_all = True
    root, _ = lib.resolve_cache_root()
    real = lib.load_json(lib.REGISTRY_PATH)
    real_by_id = {r.get("asset_id"): r for r in real.get("assets", [])}
    fixtures = sorted(RED_FIXTURE_DIR.glob("*.json"))
    if len(fixtures) < 5:
        check(False, f"RED 语料不足 5 件: {len(fixtures)}")
        return False
    for fp in fixtures:
        doc = lib.load_json(fp)
        name = fp.stem
        fails = lib.validate_registry(doc)
        if name == "red_digest_tampered":
            # digest 篡改臂：结构合法，须由缓存复算层检出。
            if fails:
                check(False, f"{name}: 结构层误报（本臂应由缓存复算层检出）: {fails[:2]}")
                ok_all = False
                continue
            if root is None:
                check(False, f"{name}: 缓存根不可达，无法复算检出")
                ok_all = False
                continue
            tampered = doc["assets"][0]
            real_row = real_by_id.get(tampered.get("asset_id"))
            if real_row is None or real_row.get("digest") == tampered.get("digest"):
                check(False, f"{name}: 语料构造失效（digest 未相对真实登记篡改）")
                ok_all = False
                continue
            vfails = lib.verify_asset_cache(tampered, root)
            if not vfails:
                check(False, f"{name}: digest 篡改未被缓存复算层检出")
                ok_all = False
            continue
        if not fails:
            check(False, f"{name}: RED 语料未被结构判定层检出（假绿口）")
            ok_all = False
        else:
            note(f"RED 检出 {name}: {fails[0]}")
    return ok_all


def run_selftest() -> int:
    # 红臂①：合成 FAILURES 必须使门红。
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 红臂②：白名单外 SPDX 注入必被 validate_spdx_expr 拒。
    if lib.validate_spdx_expr("CC-BY-NC-SA-3.0", "snap.txt") is None:
        print(f"[{TAG}] selftest FAIL: 白名单外 SPDX 未拒", file=sys.stderr)
        return 1
    # 红臂③：两类互冒充必被检。
    bogus = {
        "class": "generated",
        "asset_id": "x",
        "spdx_id": "NONE",
        "source_url": "https://evil.example/x.zip",
        "generator_script": "ci/_gen_g10_cornell_box.py",
        "generator_script_digest": "sha256:" + "0" * 64,
        "generator_params_digest": "sha256:" + "0" * 64,
        "digest": "sha256:" + "0" * 64,
        "license_snapshot": "NONE",
        "checked_at": "2026-08-15",
        "upstream_ref": "NONE",
        "cache_rel": "x/v1/",
        "file_count": 1,
        "byte_len": 1,
    }
    if not lib.validate_asset_row(bogus, index=0):
        print(f"[{TAG}] selftest FAIL: generated 谎报外部来源未检出", file=sys.stderr)
        return 1
    # 绿臂：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (3 RED + 1 GREEN)")
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
    device_state = "not_applicable"

    if not lib.REGISTRY_PATH.is_file():
        check(False, f"缺许可注册表 {lib.REGISTRY_PATH.name}")
        doc = {}
    else:
        doc = lib.load_json(lib.REGISTRY_PATH)

    if doc:
        checks["registry_structure_valid"] = leg_structure(doc)
        checks["license_snapshots_on_tree"] = leg_snapshots(doc)
        root_ok, digest_ok, dirs_ok = leg_cache(doc)
        checks["cache_root_resolved"] = root_ok
        checks["cache_digest_match"] = digest_ok
        checks["cache_dirs_all_registered"] = dirs_ok
        checks["git_binary_guard_clean"] = leg_git_guard(doc)
        checks["red_fixtures_all_detected"] = leg_red_fixtures()
        if not root_ok:
            device_state = "dev_env_degrade"
            note("缓存根不可达 → dev_env_degrade 诚实登记（不充绿，R-G10-9）")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES and device_state == "not_applicable"

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
        "device_section_state": device_state,
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
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（许可登记零缺行 + 白名单闭集 + digest 复算 + git 零二进制 + RED 五件全检出）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
