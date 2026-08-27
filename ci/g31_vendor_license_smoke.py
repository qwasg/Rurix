#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C6 vendor 许可合规终审）
"""G31+ 波 C Task C6：vendor 许可合规终审门冒烟（g31.waveC.license；
G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #53 兑现面；商用分发口径 = 再分发许可合规）。

host 纯 host 门（零 GPU 零网络；device_section_state 恒 not_applicable 落 notes）。
判据闭集（milestones/g31/g31_vendor_license_evidence_schema.json 描述段逐字）：

1. matrix_structure_valid：矩阵 JSON 结构机核——16 项逐项必备字段（id/name/category/
   version/pin/license/license_family/redistribution_status/obligations/sbom_faces/
   evidence_refs/owner_action）+ status 枚举闭集 {cleared, conditional, pending_owner,
   blocked} + 交叉规则（conditional ⇒ conditions+gaps 非空；blocked ⇒ disposition；
   clearance=reference_g13 ⇒ evidence_refs 含 G13 清结档路径）。
2. vendor_coverage_complete：item id 集 == 冻结闭集 16 项精确互核（静默漏项即 RED）。
3. license_texts_on_tree：vendored 项 license_texts_in_tree 逐路径在树非空；
   外部项 external_license_ref 登记非空；仓根双许可 LICENSE-MIT/LICENSE-APACHE 在树。
4. g13_clearance_referenced：G13 清结档在树且含 owner 接受字面（「我接受 DLSS 许可」
   + cleared）；超分三项 clearance=reference_g13 + evidence_refs 含 G13 路径；
   矩阵不复制 owner 接受字面（引用不复制，G13 §3 范式）。
5. sbom_reconciliation：SBOM 生成件 vs 矩阵逐项对账——① release.yml --component
   逐行 5 段且许可段非空（现发行面 v1.0.1-dist.2 组件全有许可登记）；② rurixup
   sbom.rs licenseConcluded + bundle.rs Component.license 字面在树（SBOM 生成机制
   覆盖 bundle 全组件）；③ basis SBOM.md 含 basis_universal + Apache-2.0；
   ④ g13_vendor_sdk_registry streamline/fidelityfx 许可 + DLL digest 登记；
   ⑤ 矩阵每项 sbom_faces 逐面文件在树且含登记字面，not_applicable_* 面如实豁免。
6. obligations_and_gaps_registered：逐项 obligations 非空；分发相关 OSI 项义务含
   声明/保留字面；GAP-01/02/03 三件闭集登记（id/title/description/owner_wave/status）。
7. summary_counts_honest：summary 计数 == items 重算（pending_owner=0/blocked=0
   如实，不冒充 cleared）。

三态：host 恒跑无 device 段；矩阵/许可文本/登记面缺失即 FAIL（不充绿）。
evidence 纪律：PASS 才落 evidence/g31_vendor_license_<ts>.json（check_schemas
前缀路由 g31_vendor_license_）；FAIL 诊断件落 .tmp/g31_gates/vendor_license/
工作区不污染 evidence/ 路由面（fail-closed：evidence/ 无件 = 门未过）。

用法：
  py -3 ci/g31_vendor_license_smoke.py --selftest
  py -3 ci/g31_vendor_license_smoke.py --gate g31.waveC.license
"""
from __future__ import annotations

import argparse
import copy
import datetime as _dt
import hashlib
import json
import platform
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
WORK = ROOT / ".tmp" / "g31_gates" / "vendor_license"
MATRIX_PATH = ROOT / "milestones" / "g31" / "g31_vendor_license_matrix.json"
MATRIX_REL = "milestones/g31/g31_vendor_license_matrix.json"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_vendor_license_evidence_schema.json"
G13_CLEARANCE = ROOT / "milestones" / "g13" / "design" / "vendor_upscale_license_clearance.md"
G13_CLEARANCE_REL = "milestones/g13/design/vendor_upscale_license_clearance.md"
G13_REGISTRY = ROOT / "milestones" / "g13" / "g13_vendor_sdk_registry.json"
RELEASE_YML = ROOT / ".github" / "workflows" / "release.yml"

GATE_KEY = "g31.waveC.license"
SUBJECT = "g31_vendor_license"
TAG = "g31_vendor_license"

CHECK_KEYS = [
    "matrix_structure_valid",
    "vendor_coverage_complete",
    "license_texts_on_tree",
    "g13_clearance_referenced",
    "sbom_reconciliation",
    "obligations_and_gaps_registered",
    "summary_counts_honest",
]

STATUS_ENUM = {"cleared", "conditional", "pending_owner", "blocked"}

# 冻结覆盖闭集（Task C6 盘点定版；静默漏项即 RED）。
EXPECTED_IDS = {
    "streamline_ngx_dlss",
    "fsr_fidelityfx",
    "nrd",
    "joltc_53",
    "joltphysics_53",
    "joltc_56",
    "joltphysics_56",
    "basis_universal",
    "rurix_basis_shim",
    "taichi_aot_runtime",
    "nvidia_libdevice",
    "nvidia_cublas",
    "rust_rowan",
    "rust_rapier3d",
    "rust_cc_build",
    "rust_cmake_build",
}

EXPECTED_GAP_IDS = {"GAP-01", "GAP-02", "GAP-03"}

ITEM_REQUIRED_FIELDS = [
    "id",
    "name",
    "category",
    "version",
    "pin",
    "license",
    "license_family",
    "redistribution_status",
    "obligations",
    "current_distribution_face",
    "sbom_faces",
    "evidence_refs",
    "owner_action",
]

# 分发相关类（义务须含声明/保留字面）；构建期/信息项豁免。
DISTRIBUTION_RELEVANT_CATEGORIES = {
    "native_vendored",
    "external_sdk_runtime_loaded",
    "external_runtime_loaded_user_provided",
    "rust_crate_embedded_in_distribution",
    "rust_crate_optional_feature_off",
}

G13_REFERENCE_ITEMS = {"streamline_ngx_dlss", "fsr_fidelityfx", "nrd"}

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _sha256_file(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def _git_head() -> str:
    try:
        r = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        )
        return r.stdout.strip()
    except Exception:
        return "unknown"


def validate_matrix(doc: dict) -> list[str]:
    """矩阵结构判定层（gate 腿 1 与 selftest RED 臂共用）。"""
    fails: list[str] = []
    if doc.get("schema") != "rurix.g31.vendor_license_matrix.v1":
        fails.append(f"schema 字面非法: {doc.get('schema')!r}")
    items = doc.get("items")
    if not isinstance(items, list) or not items:
        fails.append("items 缺失或非空数组")
        return fails
    seen: set[str] = set()
    for i, item in enumerate(items):
        iid = item.get("id", f"#{i}")
        for field in ITEM_REQUIRED_FIELDS:
            if field not in item:
                fails.append(f"items[{iid}]: 缺字段 {field}")
        st = item.get("redistribution_status")
        if st not in STATUS_ENUM:
            fails.append(f"items[{iid}]: redistribution_status 非法: {st!r}")
        if iid in seen:
            fails.append(f"items[{iid}]: id 重复")
        seen.add(iid)
        if st == "conditional":
            if not item.get("conditions") or not item.get("gaps"):
                fails.append(f"items[{iid}]: conditional 缺 conditions/gaps 登记")
        if st == "blocked" and not item.get("disposition"):
            fails.append(f"items[{iid}]: blocked 缺 disposition 处置")
        if st == "pending_owner" and not item.get("pending_note"):
            fails.append(f"items[{iid}]: pending_owner 缺 pending_note")
        if item.get("clearance") == "reference_g13":
            refs = item.get("evidence_refs") or []
            if G13_CLEARANCE_REL not in refs:
                fails.append(f"items[{iid}]: reference_g13 但 evidence_refs 缺 G13 清结档")
        if not isinstance(item.get("obligations"), list) or not item.get("obligations"):
            fails.append(f"items[{iid}]: obligations 非空数组缺失")
        if not isinstance(item.get("sbom_faces"), list) or not item.get("sbom_faces"):
            fails.append(f"items[{iid}]: sbom_faces 非空数组缺失")
    gaps = doc.get("gaps")
    if not isinstance(gaps, list) or not gaps:
        fails.append("gaps 缺失或非空数组")
    else:
        for g in gaps:
            for field in ("id", "title", "description", "owner_wave", "status"):
                if not g.get(field):
                    fails.append(f"gap {g.get('id', '?')}: 缺字段 {field}")
    summary = doc.get("summary") or {}
    for field in ("total", "cleared", "conditional", "pending_owner", "blocked"):
        if not isinstance(summary.get(field), int):
            fails.append(f"summary.{field} 非整数")
    return fails


def leg_coverage(doc: dict) -> bool:
    ids = [it.get("id") for it in doc.get("items", [])]
    ok = set(ids) == EXPECTED_IDS and len(ids) == len(EXPECTED_IDS)
    check(ok, f"vendor 覆盖闭集不符: 多出 {sorted(set(ids) - EXPECTED_IDS)} 缺 {sorted(EXPECTED_IDS - set(ids))}")
    return ok


def leg_license_texts(doc: dict) -> bool:
    ok = True
    for item in doc.get("items", []):
        iid = item.get("id", "?")
        paths = item.get("license_texts_in_tree") or []
        if paths:
            for rel in paths:
                p = ROOT / rel
                if not p.is_file() or p.stat().st_size == 0:
                    check(False, f"{iid}: 许可文本缺失或空: {rel}")
                    ok = False
        else:
            if not item.get("external_license_ref"):
                check(False, f"{iid}: 树内无许可文本且 external_license_ref 未登记")
                ok = False
    for rel in ("LICENSE-MIT", "LICENSE-APACHE"):
        if not (ROOT / rel).is_file():
            check(False, f"仓根双许可文本缺失: {rel}")
            ok = False
    return ok


def leg_g13_reference(doc: dict) -> bool:
    ok = True
    if not G13_CLEARANCE.is_file():
        check(False, f"G13 清结档缺失: {G13_CLEARANCE_REL}")
        return False
    text = G13_CLEARANCE.read_text(encoding="utf-8")
    for literal in ("我接受 DLSS 许可", "cleared"):
        if literal not in text:
            check(False, f"G13 清结档缺 owner 接受字面: {literal!r}")
            ok = False
    matrix_text = MATRIX_PATH.read_text(encoding="utf-8")
    if "我接受 DLSS 许可" in matrix_text:
        check(False, "矩阵复制了 owner 接受字面（引用不复制，G13 §3 范式）")
        ok = False
    for item in doc.get("items", []):
        if item.get("id") in G13_REFERENCE_ITEMS:
            if item.get("clearance") != "reference_g13":
                check(False, f"{item['id']}: clearance != reference_g13")
                ok = False
            if G13_CLEARANCE_REL not in (item.get("evidence_refs") or []):
                check(False, f"{item['id']}: evidence_refs 缺 G13 清结档")
                ok = False
    return ok


def leg_sbom_reconciliation(doc: dict) -> bool:
    ok = True
    # ① 现发行面：release.yml --component 逐行 5 段且许可段非空。
    if not RELEASE_YML.is_file():
        check(False, "release.yml 缺失（无法核现发行面 SBOM 源）")
        ok = False
    else:
        text = RELEASE_YML.read_text(encoding="utf-8")
        specs = re.findall(r'--component "([^"]+)"', text)
        if not specs:
            check(False, "release.yml 无 --component 行")
            ok = False
        for spec in specs:
            segs = spec.split("|")
            if len(segs) != 5 or not segs[2].strip():
                check(False, f"--component 段数或许可段为空: {spec!r}")
                ok = False
    # ② SBOM 生成机制字面（SPDX licenseConcluded + Component.license 字段）。
    sbom_rs = ROOT / "src" / "rurixup" / "src" / "sbom.rs"
    bundle_rs = ROOT / "src" / "rurixup" / "src" / "bundle.rs"
    if not sbom_rs.is_file() or "licenseConcluded" not in sbom_rs.read_text(encoding="utf-8"):
        check(False, "sbom.rs 缺 licenseConcluded（SPDX 许可登记机制）")
        ok = False
    if not bundle_rs.is_file() or "pub license: String" not in bundle_rs.read_text(encoding="utf-8"):
        check(False, "bundle.rs 缺 Component.license 字段")
        ok = False
    # ③ basis SBOM.md 登记面。
    basis_sbom = ROOT / "src" / "rurix-basis-sys" / "SBOM.md"
    if not basis_sbom.is_file():
        check(False, "src/rurix-basis-sys/SBOM.md 缺失")
        ok = False
    else:
        btext = basis_sbom.read_text(encoding="utf-8")
        for literal in ("basis_universal", "Apache-2.0"):
            if literal not in btext:
                check(False, f"basis SBOM.md 缺登记字面: {literal!r}")
                ok = False
    # ④ G13 外部 SDK 登记面（许可 + 逐 DLL digest）。
    if not G13_REGISTRY.is_file():
        check(False, "g13_vendor_sdk_registry.json 缺失")
        ok = False
    else:
        reg = json.loads(G13_REGISTRY.read_text(encoding="utf-8"))
        for sdk in ("streamline", "fidelityfx"):
            entry = (reg.get("sdks") or {}).get(sdk) or {}
            if not entry.get("license"):
                check(False, f"g13 登记 {sdk}: license 缺")
                ok = False
            if not entry.get("dlls"):
                check(False, f"g13 登记 {sdk}: dlls digest 缺")
                ok = False
    # ⑤ 矩阵每项 sbom_faces 逐面对账（not_applicable_* 面如实豁免）。
    for item in doc.get("items", []):
        iid = item.get("id", "?")
        for face in item.get("sbom_faces") or []:
            kind = face.get("kind", "")
            if kind.startswith("not_applicable"):
                continue
            rel = face.get("path") or ""
            p = ROOT / rel
            if not rel or not p.is_file():
                check(False, f"{iid}: sbom_faces[{kind}] 文件缺失: {rel!r}")
                ok = False
                continue
            needle = face.get("contains") or ""
            if needle and needle not in p.read_text(encoding="utf-8"):
                check(False, f"{iid}: sbom_faces[{kind}] {rel} 缺登记字面 {needle!r}")
                ok = False
    return ok


def leg_obligations_gaps(doc: dict) -> bool:
    ok = True
    for item in doc.get("items", []):
        iid = item.get("id", "?")
        obs = " ".join(item.get("obligations") or [])
        if not obs.strip():
            check(False, f"{iid}: obligations 空")
            ok = False
        if item.get("category") in DISTRIBUTION_RELEVANT_CATEGORIES:
            if "声明" not in obs and "保留" not in obs:
                check(False, f"{iid}: 分发相关 OSI/自定义项义务缺声明/保留字面")
                ok = False
        if item.get("redistribution_status") == "conditional":
            if not item.get("conditions") or not item.get("gaps"):
                check(False, f"{iid}: conditional 缺 conditions/gaps")
                ok = False
    gap_ids = {g.get("id") for g in doc.get("gaps") or []}
    if gap_ids != EXPECTED_GAP_IDS:
        check(False, f"gap 闭集不符: {sorted(gap_ids)}")
        ok = False
    return ok


def leg_summary_honest(doc: dict) -> bool:
    items = doc.get("items", [])
    recount = {
        "total": len(items),
        "cleared": sum(1 for it in items if it.get("redistribution_status") == "cleared"),
        "conditional": sum(1 for it in items if it.get("redistribution_status") == "conditional"),
        "pending_owner": sum(1 for it in items if it.get("redistribution_status") == "pending_owner"),
        "blocked": sum(1 for it in items if it.get("redistribution_status") == "blocked"),
    }
    summary = doc.get("summary") or {}
    ok = all(summary.get(k) == v for k, v in recount.items())
    check(ok, f"summary 计数与 items 重算不符: summary={ {k: summary.get(k) for k in recount} } recount={recount}")
    return ok


def run_selftest() -> int:
    # 红臂①：合成 FAILURES 必须使门红。
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if not MATRIX_PATH.is_file():
        print(f"[{TAG}] selftest FAIL: 矩阵缺失 {MATRIX_REL}", file=sys.stderr)
        return 1
    real = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))
    # 红臂②：status 枚举外注入必被拒。
    bogus = copy.deepcopy(real)
    bogus["items"][0]["redistribution_status"] = "cleared_anyway"
    if not validate_matrix(bogus):
        print(f"[{TAG}] selftest FAIL: status 枚举外注入未检出", file=sys.stderr)
        return 1
    # 红臂③：conditional 缺 conditions/gaps 必被拒。
    bogus2 = copy.deepcopy(real)
    for it in bogus2["items"]:
        if it.get("redistribution_status") == "conditional":
            it.pop("conditions", None)
            it.pop("gaps", None)
            break
    if not validate_matrix(bogus2):
        print(f"[{TAG}] selftest FAIL: conditional 缺条件登记未检出", file=sys.stderr)
        return 1
    # 红臂④：blocked 缺 disposition 必被拒。
    bogus3 = copy.deepcopy(real)
    bogus3["items"][0]["redistribution_status"] = "blocked"
    bogus3["items"][0].pop("disposition", None)
    if not validate_matrix(bogus3):
        print(f"[{TAG}] selftest FAIL: blocked 缺 disposition 未检出", file=sys.stderr)
        return 1
    # 红臂⑤：静默漏项（覆盖闭集少一项）必被 coverage 腿检出。
    bogus4 = copy.deepcopy(real)
    bogus4["items"] = bogus4["items"][:-1]
    if leg_coverage(bogus4):
        print(f"[{TAG}] selftest FAIL: 覆盖闭集漏项未检出", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (5 RED + 1 GREEN)")
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

    if not MATRIX_PATH.is_file():
        check(False, f"缺许可矩阵 {MATRIX_REL}")
        doc = {}
    else:
        doc = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))

    if doc:
        struct_fails = validate_matrix(doc)
        for f in struct_fails:
            check(False, f"矩阵结构: {f}")
        checks["matrix_structure_valid"] = not struct_fails
        checks["vendor_coverage_complete"] = leg_coverage(doc)
        checks["license_texts_on_tree"] = leg_license_texts(doc)
        checks["g13_clearance_referenced"] = leg_g13_reference(doc)
        checks["sbom_reconciliation"] = leg_sbom_reconciliation(doc)
        checks["obligations_and_gaps_registered"] = leg_obligations_gaps(doc)
        checks["summary_counts_honest"] = leg_summary_honest(doc)

    all_pass = all(checks.values()) and not FAILURES
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    summary = doc.get("summary") or {}
    gaps_brief = [{"id": g.get("id"), "status": g.get("status")} for g in doc.get("gaps") or []]

    evidence = {
        "schema": "rurix.g31.vendor_license_evidence.v1",
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": "G31+.C",
        "matrix_path": MATRIX_REL,
        "matrix_sha256": _sha256_file(MATRIX_PATH) if MATRIX_PATH.is_file() else "sha256:" + "0" * 64,
        "summary": {
            "total": summary.get("total", 0),
            "cleared": summary.get("cleared", 0),
            "conditional": summary.get("conditional", 0),
            "pending_owner": summary.get("pending_owner", 0),
            "blocked": summary.get("blocked", 0),
        },
        "gaps": gaps_brief,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "base_commit": _git_head(),
            "device_section_state": "not_applicable（host 纯 host 门：矩阵/SBOM/许可文本全为文件面机核）",
        },
        "timestamp": ts,
        "notes": "; ".join(NOTES + FAILURES[:8]) or "全腿绿",
    }

    if all_pass:
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
        out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[{TAG}] evidence → {out}")
        print(
            f"[{TAG}] PASS（16 项盘点 cleared {summary.get('cleared')}/conditional {summary.get('conditional')}"
            f"/pending_owner {summary.get('pending_owner')}/blocked {summary.get('blocked')}；"
            "SBOM 对账 + 许可文本在树 + G13 引用 + GAP-01~03 如实登记）"
        )
        return 0

    WORK.mkdir(parents=True, exist_ok=True)
    diag = WORK / f"diag_{ts}.json"
    diag.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)}；诊断件 → {diag}", file=sys.stderr)
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
