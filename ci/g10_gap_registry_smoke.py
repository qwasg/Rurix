#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.5b 波）
"""G10.5b M140 差距清单登记门冒烟（步骤 189；g10.p0.m140.gap_registry；
G10_CONTRACT §4.2 M140 行 / G-G10-7；G10_ACCEPTANCE_MAP §1 M140 行 + §3.2；
RFC-0026 §4.5 + §3.3；spec/visual_comparison.md RXS-0391）。

host 纯 host 门（device_section_state=not_applicable）。判据：preview §5/§6
缺口候选全 11 项（R1~R5/U1~U3/C1~C3）入正式清单 schema——每项 UE5
Renderer 模块归属（RXS-0391 L5 枚举闭集：目录级 23 + 文件级 57 + Other
终值，Other 须 attribution_note 非空）+ measured delta（≥1 项、delta ==
b_value − a_value f64 精确重算、evidence_digest 回溯 M139 最新 evidence
ab_report.artifact_digests 登记集——纯叙述无测量即 RED）+ 建议 P 级
（P0/P1/P2 闭集）+ G11 承接锚（非空）+ kind 两值分列（caliber_diff =
C1~C3 / quality_gap = R1~R5/U1~U3）+ gap_id 冻结字节规则重算 +
场景全集零空行（scene_summary 行集与 M133 冻结清单全等、no_gap_explicit
显式、not_ready_scenes 显式在列）。G10 零通过线纪律：差距全量登记即绿，
不设 FLIP/SSIM 阈值判据。

RED 臂六路（契约 §4.2 M140 字面 + RXS-0391 L2/L5/L6/L8）：缺归属 /
缺承接锚 / 非 measured 叙述充差距（空 measured_delta）/ 枚举闭集外模块 /
场景缺行 / 不可回溯 evidence_digest——篡改副本注入必检出。

用法：
  py -3 ci/g10_gap_registry_smoke.py --gate g10.p0.m140.gap_registry
  py -3 ci/g10_gap_registry_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m140_gap_registry_evidence_schema.json"
SPEC_PATH = ROOT / "spec" / "visual_comparison.md"
REGISTRY_PATH = ROOT / "milestones" / "g10" / "g10_gap_registry.json"
MANIFEST_PATH = ROOT / "milestones" / "g10" / "g10_corpus_scene_manifest.json"

sys.path.insert(0, str(ROOT / "ci"))
import g10_gap_registry_lib as gaplib  # noqa: E402
import g10_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g10.p0.m140.gap_registry"
NUMERIC_STEP = 189
SOURCE_REF = (
    "G10_CONTRACT §4.2 M140 + G-G10-7;G10_ACCEPTANCE_MAP §1 M140 + §3.2;"
    "RFC-0026 §4.5 + §3.3;spec/visual_comparison.md RXS-0391"
)
TAG = "g10_m140"
SUBJECT = "g10_m140_gap_registry"
MATRIX_ROW = "M140"

# preview §5/§6 候选 11 行题目前缀闭集（kind 分列对账面）。
EXPECTED_QUALITY_GAP_IDS = {"R1", "R2", "R3", "R4", "R5", "U1", "U2", "U3"}
EXPECTED_CALIBER_DIFF_IDS = {"C1", "C2", "C3"}

CHECK_KEYS = [
    "spec_rxs0391_clause_on_tree",
    "registry_on_tree_parseable",
    "registry_schema_closed_set_valid",
    "scene_set_matches_m133_manifest",
    "module_attribution_enum_closed_set",
    "measured_delta_traceable_to_m139",
    "kind_split_caliber_vs_quality",
    "g11_anchor_nonempty_all",
    "suggested_priority_enum_all",
    "scene_summary_full_set_zero_empty",
    "gap_id_derivation_recompute",
    "preview_candidate_items_all_present",
    "red_missing_attribution_detected",
    "red_missing_g11_anchor_detected",
    "red_unmeasured_narrative_detected",
    "red_module_outside_enum_detected",
    "red_missing_scene_row_detected",
    "red_untraceable_digest_detected",
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


def item_prefix(title: str) -> str:
    """差距项 title 的 preview 候选编号前缀（R1~R5/U1~U3/C1~C3）。"""
    m = re.match(r"^([RUC]\d+)\s", title)
    return m.group(1) if m else ""


def traceability_problems(doc: dict, digest_set: set[str]) -> list[str]:
    """measured_delta evidence_digest 回溯机核（不在登记集即问题行）。"""
    problems: list[str] = []
    for it in doc.get("items", []):
        for d in it.get("measured_delta", []):
            if d.get("evidence_digest") not in digest_set:
                problems.append(f"{it.get('gap_id')} 不可回溯: {d.get('evidence_digest')}")
    return problems


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂：真树清单 + 真 M139 evidence 登记集回溯通过。
    if not REGISTRY_PATH.is_file():
        print(f"[{TAG}] selftest FAIL: 清单未落盘（先跑 M139 门）", file=sys.stderr)
        return 1
    doc = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))
    if gaplib.validate_registry(doc):
        print(f"[{TAG}] selftest FAIL: 真树清单校验有误", file=sys.stderr)
        return 1
    m139_path = wel.load_latest_evidence("g10_m139_ab_comparison")
    if m139_path is None:
        print(f"[{TAG}] selftest FAIL: 缺 M139 evidence", file=sys.stderr)
        return 1
    m139 = wel.load_json(m139_path)
    digest_set = set(m139.get("ab_report", {}).get("artifact_digests", []))
    if traceability_problems(doc, digest_set):
        print(f"[{TAG}] selftest FAIL: 真树清单回溯面有误", file=sys.stderr)
        return 1
    # 红臂：不可回溯 digest 注入。
    bad = copy.deepcopy(doc)
    bad["items"][0]["measured_delta"][0]["evidence_digest"] = "sha256:" + "f" * 64
    if not traceability_problems(bad, digest_set):
        print(f"[{TAG}] selftest FAIL: 不可回溯 digest 未检出", file=sys.stderr)
        return 1
    # 绿臂：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (1 RED + 3 GREEN)")
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

    # ---- ① spec 条款头在树 ----
    checks["spec_rxs0391_clause_on_tree"] = SPEC_PATH.is_file() and (
        re.search(r"^###\s+RXS-0391\b", SPEC_PATH.read_text(encoding="utf-8"), re.MULTILINE)
        is not None
    )
    check(checks["spec_rxs0391_clause_on_tree"], "spec/visual_comparison.md 缺 RXS-0391 条款头")

    # ---- ② 清单在树可解析 ----
    doc: dict = {}
    if REGISTRY_PATH.is_file():
        try:
            doc = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as e:
            check(False, f"清单不可解析: {e}")
    checks["registry_on_tree_parseable"] = bool(doc)
    check(doc, "差距清单未落盘（M139 门先行）或不可解析")

    # ---- ③ 字段闭集校验（lib 单一事实源） ----
    verrs = gaplib.validate_registry(doc) if doc else ["缺清单"]
    checks["registry_schema_closed_set_valid"] = not verrs
    check(not verrs, f"清单 schema 闭集校验失败: {verrs[:4]}")

    # ---- ④ scene_set 与 M133 冻结清单全等 ----
    manifest_scenes: list[str] = []
    if MANIFEST_PATH.is_file():
        try:
            manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
            manifest_scenes = sorted(
                s["scene_id"] for s in manifest.get("scenes", []) if "scene_id" in s
            )
        except (OSError, json.JSONDecodeError) as e:
            check(False, f"M133 清单不可解析: {e}")
    scene_set_ok = bool(manifest_scenes) and sorted(doc.get("scene_set", [])) == manifest_scenes
    checks["scene_set_matches_m133_manifest"] = scene_set_ok
    check(scene_set_ok, f"scene_set 与 M133 清单不全等: {doc.get('scene_set')} vs {manifest_scenes}")

    # ---- ⑤ 模块归属枚举闭集（含 Other 条件 note；Other 行计数登记防滥用） ----
    items = doc.get("items", [])
    other_rows = [it for it in items if it.get("ue5_module_primary") == gaplib.OTHER_MODULE]
    module_ok = bool(items) and all(
        it.get("ue5_module_primary") in gaplib.UE5_MODULE_ENUM
        and all(s in gaplib.UE5_MODULE_ENUM for s in it.get("ue5_module_secondary", []))
        for it in items
    ) and all(str(it.get("attribution_note", "")).strip() for it in other_rows)
    checks["module_attribution_enum_closed_set"] = module_ok
    check(module_ok, "模块归属枚举闭集/Other note 校验失败")
    note(f"Other 终值行计数 = {len(other_rows)}（防滥用登记，RXS-0391 L5）")

    # ---- ⑥ measured_delta 可回溯（M139 最新 evidence artifact_digests 登记集） ----
    m139_path = wel.load_latest_evidence("g10_m139_ab_comparison")
    m139_doc: dict = {}
    digest_set: set[str] = set()
    if m139_path is not None:
        try:
            m139_doc = wel.load_json(m139_path)
            digest_set = set(m139_doc.get("ab_report", {}).get("artifact_digests", []))
        except (OSError, json.JSONDecodeError) as e:
            check(False, f"M139 evidence 不可解析: {e}")
    m139_ok = (
        m139_doc.get("symbolic_gate_key") == "g10.p0.m139.ab_comparison"
        and m139_doc.get("status") == "pass"
        and bool(digest_set)
    )
    trace_problems = traceability_problems(doc, digest_set) if m139_ok else ["缺 M139 PASS evidence 登记集"]
    checks["measured_delta_traceable_to_m139"] = m139_ok and not trace_problems
    check(checks["measured_delta_traceable_to_m139"], f"measured_delta 回溯失败: {trace_problems[:3]}")

    # ---- ⑦ kind 两值分列（caliber_diff = C1~C3 / quality_gap = R1~R5/U1~U3） ----
    kind_ids = {it.get("kind"): set() for it in items}
    for it in items:
        kind_ids.setdefault(it.get("kind"), set()).add(item_prefix(it.get("title", "")))
    kind_ok = (
        set(kind_ids) == {"quality_gap", "caliber_diff"}
        and kind_ids.get("caliber_diff") == EXPECTED_CALIBER_DIFF_IDS
        and kind_ids.get("quality_gap") == EXPECTED_QUALITY_GAP_IDS
    )
    checks["kind_split_caliber_vs_quality"] = kind_ok
    check(kind_ok, f"kind 分列对账失败: {kind_ids}")

    # ---- ⑧ g11_anchor 全非空 / ⑨ 建议 P 级闭集 ----
    anchor_ok = bool(items) and all(str(it.get("g11_anchor", "")).strip() for it in items)
    checks["g11_anchor_nonempty_all"] = anchor_ok
    check(anchor_ok, "缺承接锚行存在")
    prio_ok = bool(items) and all(it.get("suggested_priority") in gaplib.PRIORITIES for it in items)
    checks["suggested_priority_enum_all"] = prio_ok
    check(prio_ok, "建议 P 级闭集外取值存在")

    # ---- ⑩ 场景全集零空行（scene_summary 全等 + no_gap_explicit 显式 + not_ready 在列） ----
    summary = doc.get("scene_summary", [])
    summary_scenes = sorted(r.get("scene_id", "") for r in summary)
    summary_ok = (
        summary_scenes == sorted(doc.get("scene_set", []))
        and "not_ready_scenes" in doc
        and all(isinstance(r.get("no_gap_explicit"), bool) for r in summary)
        and all(r.get("no_gap_explicit") == (r.get("gap_count") == 0) for r in summary)
    )
    checks["scene_summary_full_set_zero_empty"] = bool(summary) and summary_ok
    check(checks["scene_summary_full_set_zero_empty"], "场景全集零空行对账失败")

    # ---- ⑪ gap_id 冻结字节规则重算 ----
    gid_ok = bool(items) and all(
        it.get("gap_id") == gaplib.derive_gap_id(
            it.get("scene_id", ""), it.get("camera_id", ""),
            it.get("ue5_module_primary", ""), it.get("kind", ""), it.get("title", ""),
        )
        for it in items
    )
    checks["gap_id_derivation_recompute"] = gid_ok
    check(gid_ok, "gap_id 重算不等行存在")

    # ---- ⑫ preview 候选 11 行全在（R1~R5/U1~U3/C1~C3） ----
    present = {item_prefix(it.get("title", "")) for it in items}
    coverage_ok = present == (EXPECTED_QUALITY_GAP_IDS | EXPECTED_CALIBER_DIFF_IDS) and len(items) == 11
    checks["preview_candidate_items_all_present"] = coverage_ok
    check(coverage_ok, f"preview 候选行集不全等: {sorted(present)}")

    # ---- RED 臂六路（篡改副本注入 ⇒ lib/回溯面必检出） ----
    def tamper_errors(mutate) -> list[str]:
        bad = copy.deepcopy(doc)
        mutate(bad)
        return gaplib.validate_registry(bad)

    red_a = bool(tamper_errors(lambda d: d["items"][0].pop("ue5_module_primary")))
    checks["red_missing_attribution_detected"] = red_a
    check(red_a, "RED 臂失效：缺归属未检出")

    red_b = bool(tamper_errors(lambda d: d["items"][0].update(g11_anchor="  ")))
    checks["red_missing_g11_anchor_detected"] = red_b
    check(red_b, "RED 臂失效：缺承接锚未检出")

    red_c = bool(tamper_errors(lambda d: d["items"][0].update(measured_delta=[])))
    checks["red_unmeasured_narrative_detected"] = red_c
    check(red_c, "RED 臂失效：非 measured 叙述未检出")

    red_d = bool(tamper_errors(lambda d: d["items"][0].update(
        ue5_module_primary="Engine/Source/Runtime/Renderer/Private/Evil.cpp")))
    checks["red_module_outside_enum_detected"] = red_d
    check(red_d, "RED 臂失效：闭集外模块未检出")

    red_e = bool(tamper_errors(lambda d: d.update(scene_summary=d["scene_summary"][:1])))
    checks["red_missing_scene_row_detected"] = red_e
    check(red_e, "RED 臂失效：场景缺行未检出")

    def _tamper_digest(d):
        d["items"][0]["measured_delta"][0]["evidence_digest"] = "sha256:" + "f" * 64

    bad_f = copy.deepcopy(doc)
    _tamper_digest(bad_f)
    red_f = bool(traceability_problems(bad_f, digest_set))
    checks["red_untraceable_digest_detected"] = red_f
    check(red_f, "RED 臂失效：不可回溯 digest 未检出")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    kind_split = {
        "quality_gap": sum(1 for it in items if it.get("kind") == "quality_gap"),
        "caliber_diff": sum(1 for it in items if it.get("kind") == "caliber_diff"),
    }
    registry_digest = "sha256:" + hashlib.sha256(REGISTRY_PATH.read_bytes()).hexdigest() if REGISTRY_PATH.is_file() else ""

    gap_report = {
        "registry_path": "milestones/g10/g10_gap_registry.json",
        "registry_digest": registry_digest,
        "item_count": len(items),
        "kind_split": kind_split,
        "other_module_rows": len(other_rows),
        "per_scene": {r.get("scene_id"): r.get("gap_count") for r in summary},
        "not_ready_scenes": doc.get("not_ready_scenes", []),
        "m139_evidence_path": str(m139_path.relative_to(ROOT)).replace("\\", "/") if m139_path else "",
        "traceability": "每 measured_delta.evidence_digest ∈ M139 最新 evidence ab_report.artifact_digests 登记集（机核）",
        "zero_pass_line": "G10 零通过线维持：差距全量 measured 登记即绿，不设 FLIP/SSIM/帧率通过判据（契约 G-G10-7 / 立项裁决 5）",
    }

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G10.5",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "gap_registry_report": gap_report,
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

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if all_pass and not FAILURES:
        print(
            f"[{TAG}] PASS（清单 {len(items)} 项全绿：枚举归属 + measured 回溯 + kind 分列 "
            f"{kind_split} + 场景零空行 + RED 六臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
