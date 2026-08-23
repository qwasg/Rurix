#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.3 修复闭环波）
"""G15.3 P0 硬门 M-b：measured 主差修复闭环（g15.p0.m_b.gap_fix_closure_loop，
步骤 271；G15_CONTRACT §4.2 M-b 行判据逐字 / G-G15-4；G15_ACCEPTANCE_MAP §1 M-b 行）。

host 消费门（本波零修复立项零 src 变更——门只读核验 M-a fresh evidence/处置表 +
闭环登记表结构化校验 + RED 臂；修复项真跑面 not-triggered 如实登记不充绿）。
判据（契约 §4.2 M-b 行字面）：

1. **处置表 20 行逐行终态处置三态零空行**：闭环登记表
   milestones/g15/g15_gap_fix_closure_registry.json 逐行 final_disposition ∈
   {closed-resolved（修复后 fresh delta 进容差带，RXS-0393 收敛判据两款）/
   closed-caliber-registered（口径差显式登记不拟合，RXS-0392）/
   open-defer-G16+（承接锚字面「重判条件 = …；兜底 = …」）}——逐行 gap_id 与
   M-a 处置表逐字对账（闭集全等 + kind/title/direction/suggestion 标签级一致），
   零空行；closed-caliber-registered 行向上取严 = 仅 kind==caliber_diff 行可判
   （quality_gap 行判 caliber 闭合即 FAIL——防拟合冒充口径闭合）。
2. **修复项 RED 先行**：fix_projects 每项必含 red_first（失败测试先落字面留痕）
   + green_evidence；本波评估结论 = 全部无可 bounded 修复面（逐行 fix_evaluation
   写清修复面在哪、为何触冻结面/为何收益风险不成立）→ 零修复立项 = 合法退出
   形态（修复评估完结 + 零修复立项 + 20 行终态处置全量），RED 先行纪律 vacuous
   成立如实登记；任一修复项无 RED 先行留痕即 FAIL。
3. **触冻结面独立 Full RFC 留痕（D-409 对抗评审）**：summary.frozen_face_touched
   == false + rfc_consumed == 0 + ledger RFC next_free == 31 维持（机核）+
   src/spec/conformance 与 milestones/g5~g14、ci/g5_*~g14_* 已提交面 diff HEAD
   为空（0-byte）；若触冻结面须有 Full RFC 文件 + Agent Approved 字面。
4. **材质链表达面立项评估结论登记**：G11-N8/G11-N9/G12-N10 承接锚命中判定逐字
   （透射/焦散/镜面 IBL 类能量是否成为画质量级 measured 主差）——verdict ∈
   {triggered, not-triggered}；not-triggered = 未命中如实登记不充绿（三锚兜底
   字面逐字登记）；triggered 须 Full RFC Agent Approved。另 G15-MA-F1（vendor
   双臂 converged 输出 scene-linear 域停留）评估定论登记（契约语义内形态 =
   closed-caliber-registered／生产缺陷 = fix-project／open-defer-G16+）。

RED 臂（契约判据字面 + 任务书五臂）：处置缺行 / 三态外值 / open-defer 承接锚缺
「重判条件/兜底」字面 / 材质链评估缺结论字面 / 修复项无 RED 先行留痕——各臂
注入必检出（--selftest + 门内真跑臂）。

pr-smoke 默认 --verify-latest（秒级核最新 full-run evidence）；
本地/workflow_dispatch 用 --gate 产 full-run。

用法：
  py -3 ci/g15_gap_fix_closure_smoke.py --gate g15.p0.m_b.gap_fix_closure_loop
  py -3 ci/g15_gap_fix_closure_smoke.py --verify-latest
  py -3 ci/g15_gap_fix_closure_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import copy
import datetime as _dt
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402
import g15_dual_end_quality_reharvest_smoke as ma  # noqa: E402

GATE_KEY = "g15.p0.m_b.gap_fix_closure_loop"
NUMERIC_STEP = 271  # 落盘前实测 registry/number_ledger.json CI_step.next_free=271 顺位领取
SUBJECT = "g15_m_b_gap_fix_closure_loop"
WAVE = "G15.3"
TAG = "g15_m_b"
MATRIX_ROW = "M-b"
SOURCE_REF = (
    "G15_CONTRACT §4.2 M-b/G-G15-4;G15_ACCEPTANCE_MAP §1 M-b;RXS-0392 不拟合/RXS-0393 修复闭环判据两款;"
    "P-09 程序产阈禁手写;G11-N8/G11-N9/G12-N10 承接锚命中判定;G15-MA-F1 评估定论"
)
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_m_b_gap_fix_closure_loop_evidence_schema.json"
CLOSURE_PATH = ROOT / "milestones" / "g15" / "g15_gap_fix_closure_registry.json"
LEDGER_PATH = ROOT / "registry" / "number_ledger.json"
DISPOSITION_PATH = ma.DISPOSITION_PATH

MA_GATE = "g15.p0.m_a.dual_end_quality_reharvest"
MA_PREFIX = "g15_m_a_dual_end_quality_reharvest"

FINAL_STATES = ("closed-resolved", "closed-caliber-registered", "open-defer-G16+")
GENERATED_BY = (
    "G15.3 修复闭环波（M-b）逐行评估立项裁决面——结构化机核/RED 臂校验 = "
    "ci/g15_gap_fix_closure_smoke.py --gate g15.p0.m_b.gap_fix_closure_loop"
)
RFC_NEXT_FREE_FROZEN = 31  # G15.1 治理波零 RFC 消费定盘面（契约 §7 立项裁决 4）；本波触冻结面零发生维持

CLOSURE_TOP_KEYS = frozenset({
    "schema_version", "registry", "generated_by", "wave", "m_a_evidence",
    "m_a_wave_start", "disposition_source", "items", "fix_projects",
    "material_chain_assessment", "findings_adjudication", "summary",
})
CLOSURE_ITEM_KEYS = frozenset({
    "gap_id", "source_registry", "scene_id", "kind", "title", "m_a_direction",
    "m_a_suggestion", "final_disposition", "fix_evaluation", "anchor",
    "caliber_registration", "fix_id",
})
FIX_PROJECT_KEYS = frozenset({
    "fix_id", "title", "red_first", "green_evidence", "frozen_face_touched", "rfc",
})
MATERIAL_KEYS = frozenset({
    "anchors", "question", "verdict", "verdict_verbatim", "per_family_attribution",
    "anchor_literals", "full_rfc_required", "full_rfc_note",
})
FINDING_KEYS = frozenset({
    "id", "title", "verdict", "verdict_basis", "production_face_status",
    "parity_face_status", "excluded_fix_faces",
})
SUMMARY_KEYS = frozenset({
    "closed_resolved", "closed_caliber_registered", "open_defer_g16_plus",
    "fix_projects_count", "frozen_face_touched", "rfc_consumed",
})
MATERIAL_ANCHORS = ["G11-N8", "G11-N9", "G12-N10"]

CHECK_KEYS = [
    "m_a_evidence_pass_and_chain_anchor",
    "disposition_table_valid_fresh",
    "frozen_contracts_and_registries_0byte",
    "closure_registry_20_rows_zero_empty",
    "gap_id_closed_set_match",
    "final_disposition_three_state_closed_set",
    "open_defer_anchor_verbatim",
    "closed_caliber_registration_rxs0392",
    "closed_resolved_rxs0393_red_first",
    "fix_projects_red_first_discipline",
    "frozen_face_zero_rfc_zero",
    "material_chain_verdict_registered",
    "g15_ma_f1_adjudicated",
    "per_row_fix_evaluation_non_empty",
    "red_arm_missing_row_detected",
    "red_arm_out_of_enum_disposition_detected",
    "red_arm_open_defer_anchor_missing_literal_detected",
    "red_arm_material_chain_verdict_missing_detected",
    "red_arm_fix_project_without_red_first_detected",
]

# 异己并发工作树面（G15_CONTRACT §7 立项裁决 3 + §8.2 登记字面：untracked 未提交、
# 零消费、零混入）——src/ 下 untracked porcelain 闭集允许面。
ALIEN_UNTRACKED_SRC = {
    "src/rurix-asset/src/ktx2_read.rs",
    "src/rurix-render/src/geometry/hzb.rs",
    "src/rurix-render/src/gi/restir.rs",
    "src/rurix-render/src/gi/sdf_trace.rs",
    "src/rurix-render/src/shadow/smrt.rs",
    "src/rurix-render/src/ssr/",
}

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def run(cmd: list[str], timeout: int = 7200) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def base_commit() -> str:
    r = run(["git", "rev-parse", "HEAD"])
    return (r.stdout or "").strip()


def _is_nonempty_str(v) -> bool:
    return isinstance(v, str) and bool(v.strip())


# ---------------------------------------------------------------------------
# ① 闭环登记表校验器（闭集纪律 + 20 行零空行 + 三态枚举 + 逐态字面义务）
# ---------------------------------------------------------------------------


def validate_closure(doc, disp_items: list[dict]) -> list[str]:
    """闭环登记表校验器（顶层/行/修复项/材质链/发现定论/汇总六闭集 + 与 M-a 处置表
    标签级逐字对账 + 三态逐态义务字面机核）。返回错误列表，空 = 通过。"""
    errs: list[str] = []
    if not isinstance(doc, dict):
        return ["闭环登记表顶层非 object"]
    extra = set(doc) - CLOSURE_TOP_KEYS
    missing = CLOSURE_TOP_KEYS - set(doc)
    if extra or missing:
        errs.append(f"顶层闭集漂移: extra={sorted(extra)} missing={sorted(missing)}")
        return errs
    if doc.get("schema_version") != 1:
        errs.append("schema_version ≠ 1")
    if doc.get("registry") != "g15_gap_fix_closure_registry":
        errs.append(f"registry 漂移: {doc.get('registry')!r}")
    if doc.get("generated_by") != GENERATED_BY:
        errs.append("generated_by 非本波字面")
    if doc.get("wave") != WAVE:
        errs.append(f"wave ≠ {WAVE}")
    for k in ("m_a_evidence", "m_a_wave_start", "disposition_source"):
        if not _is_nonempty_str(doc.get(k)):
            errs.append(f"{k} 空（溯源面零空行门）")
    if doc.get("disposition_source") != "milestones/g15/g15_quality_gap_disposition.json":
        errs.append("disposition_source 非 M-a 处置表字面")

    items = doc.get("items")
    if not isinstance(items, list):
        errs.append("items 非数组")
        items = []
    if len(items) != len(disp_items):
        errs.append(f"行数 {len(items)} ≠ M-a 处置表 {len(disp_items)}")
    for idx, it in enumerate(items):
        tag = f"items[{idx}]"
        if not isinstance(it, dict):
            errs.append(f"{tag} 非 object")
            continue
        iextra = set(it) - CLOSURE_ITEM_KEYS
        imissing = CLOSURE_ITEM_KEYS - set(it)
        if iextra or imissing:
            errs.append(f"{tag} 字段闭集漂移: extra={sorted(iextra)} missing={sorted(imissing)}")
            continue
        for k in ("gap_id", "source_registry", "scene_id", "kind", "title",
                  "m_a_direction", "m_a_suggestion", "final_disposition"):
            if not _is_nonempty_str(it.get(k)):
                errs.append(f"{tag}.{k} 空（零空行门）")
        # 与 M-a 处置表标签级逐字对账（f64 数值面不入对账——跨会话带内吸收面；
        # 标签翻转即 FAIL 强制重评估）
        if idx < len(disp_items):
            d = disp_items[idx]
            for k in ("gap_id", "source_registry", "scene_id", "kind", "title"):
                if it.get(k) != d.get(k):
                    errs.append(f"{tag}.{k} 与 M-a 处置表逐字对账不符: {it.get(k)!r} vs {d.get(k)!r}")
            if it.get("m_a_direction") != d.get("direction"):
                errs.append(f"{tag}.m_a_direction 与处置表 direction 不符: {it.get('m_a_direction')!r} vs {d.get('direction')!r}（标签翻转须重评估）")
            if it.get("m_a_suggestion") != d.get("suggestion"):
                errs.append(f"{tag}.m_a_suggestion 与处置表 suggestion 不符: {it.get('m_a_suggestion')!r} vs {d.get('suggestion')!r}")
        if it.get("final_disposition") not in FINAL_STATES:
            errs.append(f"{tag}.final_disposition 三态闭集外: {it.get('final_disposition')!r}")
        fe = it.get("fix_evaluation")
        if not _is_nonempty_str(fe) or "修复面" not in fe:
            errs.append(f"{tag}.fix_evaluation 空/缺「修复面」论证字面（逐行评估义务）")
        state = it.get("final_disposition")
        if state == "open-defer-G16+":
            anchor = it.get("anchor")
            if not _is_nonempty_str(anchor) or "重判条件 = " not in anchor or "兜底 = " not in anchor:
                errs.append(f"{tag}.anchor 缺「重判条件 = …；兜底 = …」承接锚字面")
            if it.get("caliber_registration") is not None:
                errs.append(f"{tag}.caliber_registration 非 null（open-defer 行口径登记互斥）")
            if it.get("fix_id") is not None:
                errs.append(f"{tag}.fix_id 非 null（open-defer 行无修复立项）")
        elif state == "closed-caliber-registered":
            cr = it.get("caliber_registration")
            if not _is_nonempty_str(cr) or "RXS-0392" not in cr:
                errs.append(f"{tag}.caliber_registration 空/缺 RXS-0392 不拟合字面")
            if it.get("kind") != "caliber_diff":
                errs.append(f"{tag} kind={it.get('kind')!r} 非 caliber_diff 判 caliber 闭合（向上取严——防拟合冒充口径闭合）")
            if it.get("anchor") is not None:
                errs.append(f"{tag}.anchor 非 null（closed 行承接锚互斥）")
            if it.get("fix_id") is not None:
                errs.append(f"{tag}.fix_id 非 null（caliber 登记行无修复立项）")
        elif state == "closed-resolved":
            if not _is_nonempty_str(it.get("fix_id")):
                errs.append(f"{tag}.fix_id 空（closed-resolved 必有修复立项引用）")
            if it.get("anchor") is not None or it.get("caliber_registration") is not None:
                errs.append(f"{tag}.anchor/caliber_registration 非 null（closed-resolved 行互斥面）")

    projects = doc.get("fix_projects")
    if not isinstance(projects, list):
        errs.append("fix_projects 非数组")
        projects = []
    proj_ids: set[str] = set()
    for j, p in enumerate(projects):
        tj = f"fix_projects[{j}]"
        if not isinstance(p, dict) or set(p) != FIX_PROJECT_KEYS:
            errs.append(f"{tj} 字段闭集漂移")
            continue
        if not _is_nonempty_str(p.get("fix_id")) or not _is_nonempty_str(p.get("title")):
            errs.append(f"{tj} fix_id/title 空")
        rf = p.get("red_first")
        if not _is_nonempty_str(rf) or "RED" not in rf:
            errs.append(f"{tj}.red_first 空/缺 RED 字面（修复项 RED 先行——失败测试先落留痕义务）")
        if not _is_nonempty_str(p.get("green_evidence")):
            errs.append(f"{tj}.green_evidence 空（修复项绿面 evidence 溯源义务）")
        if not isinstance(p.get("frozen_face_touched"), bool):
            errs.append(f"{tj}.frozen_face_touched 非布尔")
        if p.get("frozen_face_touched") is True and not _is_nonempty_str(p.get("rfc")):
            errs.append(f"{tj} 触冻结面但 rfc 空（触冻结面独立 Full RFC 留痕义务，D-409）")
        if _is_nonempty_str(p.get("fix_id")):
            proj_ids.add(p["fix_id"])
    for idx, it in enumerate(items):
        if isinstance(it, dict) and it.get("final_disposition") == "closed-resolved":
            fid = it.get("fix_id")
            if _is_nonempty_str(fid) and fid not in proj_ids:
                errs.append(f"items[{idx}].fix_id {fid!r} 不在 fix_projects 闭集")

    mc = doc.get("material_chain_assessment")
    if not isinstance(mc, dict) or set(mc) != MATERIAL_KEYS:
        errs.append("material_chain_assessment 字段闭集漂移/缺失")
    else:
        if mc.get("anchors") != MATERIAL_ANCHORS:
            errs.append(f"material_chain anchors 漂移: {mc.get('anchors')!r}")
        q = mc.get("question")
        if not _is_nonempty_str(q) or "透射/焦散/镜面 IBL" not in q:
            errs.append("material_chain question 缺「透射/焦散/镜面 IBL」判据字面")
        verdict = mc.get("verdict")
        if verdict not in ("triggered", "not-triggered"):
            errs.append(f"material_chain verdict 闭集外: {verdict!r}")
        vv = mc.get("verdict_verbatim")
        if not _is_nonempty_str(vv) or "命中" not in vv:
            errs.append("material_chain verdict_verbatim 空/缺命中判定结论字面")
        if verdict == "not-triggered":
            if "未命中" not in (vv or ""):
                errs.append("material_chain not-triggered 但 verdict_verbatim 缺「未命中」字面")
            if mc.get("full_rfc_required") is not False:
                errs.append("material_chain not-triggered 但 full_rfc_required ≠ false")
        if verdict == "triggered" and mc.get("full_rfc_required") is not True:
            errs.append("material_chain triggered 但 full_rfc_required ≠ true（Full RFC 立项义务）")
        al = mc.get("anchor_literals")
        if not isinstance(al, dict) or sorted(al.keys()) != sorted(MATERIAL_ANCHORS):
            errs.append("material_chain anchor_literals 三锚缺行/漂移")
        elif any(not _is_nonempty_str(al.get(a)) for a in MATERIAL_ANCHORS):
            errs.append("material_chain anchor_literals 锚字面空行")
        pfa = mc.get("per_family_attribution")
        if not isinstance(pfa, list) or not pfa:
            errs.append("material_chain per_family_attribution 空（逐族归因核对义务）")
        if not _is_nonempty_str(mc.get("full_rfc_note")):
            errs.append("material_chain full_rfc_note 空")

    findings = doc.get("findings_adjudication")
    if not isinstance(findings, list):
        errs.append("findings_adjudication 非数组")
        findings = []
    f1 = None
    for j, f in enumerate(findings):
        tj = f"findings_adjudication[{j}]"
        if not isinstance(f, dict) or set(f) != FINDING_KEYS:
            errs.append(f"{tj} 字段闭集漂移")
            continue
        if f.get("id") == "G15-MA-F1":
            f1 = f
    if f1 is None:
        errs.append("findings_adjudication 缺 G15-MA-F1 定论行（M-b 门内评估面义务）")
    else:
        if f1.get("verdict") not in ("closed-caliber-registered", "open-defer-G16+", "fix-project"):
            errs.append(f"G15-MA-F1 verdict 闭集外: {f1.get('verdict')!r}")
        vb = f1.get("verdict_basis")
        if not _is_nonempty_str(vb) or "G14.10f" not in vb:
            errs.append("G15-MA-F1 verdict_basis 空/缺 G14.10f 修复面论证字面")
        for k in ("production_face_status", "parity_face_status", "excluded_fix_faces"):
            if not _is_nonempty_str(f1.get(k)):
                errs.append(f"G15-MA-F1.{k} 空")
        if f1.get("verdict") == "fix-project" and not proj_ids:
            errs.append("G15-MA-F1 判 fix-project 但 fix_projects 空（修复立项缺失）")

    summary = doc.get("summary")
    if not isinstance(summary, dict) or set(summary) != SUMMARY_KEYS:
        errs.append("summary 字段闭集漂移/缺失")
    else:
        tally = {s: sum(1 for it in items if isinstance(it, dict) and it.get("final_disposition") == s)
                 for s in FINAL_STATES}
        if summary.get("closed_resolved") != tally["closed-resolved"]:
            errs.append("summary.closed_resolved ≠ 实计")
        if summary.get("closed_caliber_registered") != tally["closed-caliber-registered"]:
            errs.append("summary.closed_caliber_registered ≠ 实计")
        if summary.get("open_defer_g16_plus") != tally["open-defer-G16+"]:
            errs.append("summary.open_defer_g16_plus ≠ 实计")
        if summary.get("fix_projects_count") != len(projects):
            errs.append("summary.fix_projects_count ≠ fix_projects 实计")
        if not isinstance(summary.get("frozen_face_touched"), bool):
            errs.append("summary.frozen_face_touched 非布尔")
        if summary.get("frozen_face_touched") is False and summary.get("rfc_consumed") != 0:
            errs.append("未触冻结面但 rfc_consumed ≠ 0")
        if summary.get("frozen_face_touched") is True and not proj_ids:
            errs.append("触冻结面但无修复立项（Full RFC 留痕面对不上）")
    return errs


# ---------------------------------------------------------------------------
# ② RED 臂（门内真跑：以本门校验器为底，五臂独立）
# ---------------------------------------------------------------------------


def red_arm_missing_row(sample_doc: dict, disp_items: list[dict]) -> bool:
    """处置缺行 → 校验器必检出（行数/闭集面）。"""
    doc = copy.deepcopy(sample_doc)
    doc["items"] = doc["items"][:-1]
    return bool(validate_closure(doc, disp_items))


def red_arm_out_of_enum_disposition(sample_doc: dict, disp_items: list[dict]) -> bool:
    """三态外值 → 枚举闭集面必检出。"""
    doc = copy.deepcopy(sample_doc)
    doc["items"][0]["final_disposition"] = "closed-fake-green"
    return bool(validate_closure(doc, disp_items))


def red_arm_open_defer_anchor_missing_literal(sample_doc: dict, disp_items: list[dict]) -> bool:
    """open-defer 承接锚缺「兜底」字面 → 承接锚机核面必检出。"""
    doc = copy.deepcopy(sample_doc)
    target = None
    for it in doc.get("items") or []:
        if it.get("final_disposition") == "open-defer-G16+":
            target = it
            break
    if target is None:
        return False
    target["anchor"] = "重判条件 = 某条件"
    return bool(validate_closure(doc, disp_items))


def red_arm_material_chain_verdict_missing(sample_doc: dict, disp_items: list[dict]) -> bool:
    """材质链评估缺结论字面（G11-N9 锚字面行删除）→ 三锚闭集面必检出。"""
    doc = copy.deepcopy(sample_doc)
    doc["material_chain_assessment"]["anchor_literals"].pop("G11-N9", None)
    return bool(validate_closure(doc, disp_items))


def red_arm_fix_project_without_red_first(sample_doc: dict, disp_items: list[dict]) -> bool:
    """修复项无 RED 先行留痕 → red_first 字面机核面必检出。"""
    doc = copy.deepcopy(sample_doc)
    doc["fix_projects"] = [{
        "fix_id": "G15-MB-FAKE",
        "title": "合成修复项（无 RED 先行留痕）",
        "red_first": "",
        "green_evidence": "evidence/nowhere.json",
        "frozen_face_touched": False,
        "rfc": None,
    }]
    doc["summary"]["fix_projects_count"] = 1
    return bool(validate_closure(doc, disp_items))


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    started = _dt.datetime.now(_dt.timezone.utc)
    ts = started.strftime("%Y%m%dT%H%M%SZ")
    red_results: dict[str, bool] = {}

    # ── ① M-a 最新 evidence PASS + 链锚（parity.wave_start == 处置表 wave_start） ──
    ma_path = wel.load_latest_evidence(MA_PREFIX)
    ma_doc: dict = {}
    ma_ok = False
    ma_wave_start = ""
    if ma_path is None:
        check(False, f"缺 M-a 最新 evidence（{MA_PREFIX}_*.json）")
    else:
        try:
            ma_doc = wel.load_json(ma_path)
        except (OSError, json.JSONDecodeError) as e:
            check(False, f"M-a evidence 不可读: {e}")
    if ma_doc:
        ma_ok, ma_detail = wel.gate_pass_reason(ma_doc, MA_GATE)
        ma_wave_start = str((ma_doc.get("parity") or {}).get("wave_start") or "")
        check(ma_ok, f"M-a 门非全绿: {ma_detail}")
        note(f"M-a 最新 evidence {ma_path.name}: {'PASS' if ma_ok else ma_detail}（wave_start={ma_wave_start}）")
    checks["m_a_evidence_pass_and_chain_anchor"] = bool(ma_ok and ma_wave_start)

    # ── ② M-a 处置表有效（同族校验 + 方向交叉核验重算绿）＋ 链锚一致 ──
    disp_doc: dict = {}
    disp_items: list[dict] = []
    disp_ok = False
    if not DISPOSITION_PATH.is_file():
        check(False, "g15_quality_gap_disposition.json 缺失")
    else:
        try:
            disp_doc = wel.load_json(DISPOSITION_PATH)
        except (OSError, json.JSONDecodeError) as e:
            check(False, f"处置表不可读: {e}")
    if disp_doc:
        frozen_union: list[tuple[str, str]] = []
        for path, src in ((ma.G13_UPSCALE_REGISTRY, "g13_ue_upscale_gap_registry"),
                          (ma.G13_LUMEN_REGISTRY, "g13_ue_lumen_gap_registry"),
                          (ma.G12_PT_REGISTRY, "g12_ue_pt_gap_registry")):
            rdoc = wel.load_json(path)
            for it in rdoc.get("items") or []:
                frozen_union.append((it.get("gap_id"), src))
        verrs = ma.validate_disposition(disp_doc, frozen_union)
        xerrs = ma.crosscheck_directions(disp_doc)
        chain_ok = str(disp_doc.get("wave_start") or "") == ma_wave_start and bool(ma_wave_start)
        disp_ok = not verrs and not xerrs and chain_ok
        check(not verrs, f"处置表校验: {verrs[:3]}")
        check(not xerrs, f"处置表方向交叉核验: {xerrs[:3]}")
        check(chain_ok, f"链锚不一致: 处置表 wave_start={disp_doc.get('wave_start')!r} vs M-a evidence {ma_wave_start!r}")
        disp_items = disp_doc.get("items") or []
        if disp_ok:
            note(f"M-a 处置表 20 行有效 + 链锚一致（wave_start={ma_wave_start}）")
    checks["disposition_table_valid_fresh"] = bool(disp_ok)

    # ── ③ 三 parity 契约 + 三冻结登记表 0-byte（在树 == HEAD 提交态逐字节 git 机核） ──
    frozen_files = [
        ma.G13_UPSCALE_CONTRACT, ma.G13_LUMEN_CONTRACT, ma.G12_PT_CONTRACT,
        ma.G13_UPSCALE_REGISTRY, ma.G13_LUMEN_REGISTRY, ma.G12_PT_REGISTRY,
    ]
    zero_bad: list[str] = []
    for p in frozen_files:
        rel = p.relative_to(ROOT).as_posix()
        if not p.is_file():
            zero_bad.append(f"{rel} 缺失")
            continue
        committed = run(["git", "show", f"HEAD:{rel}"]).stdout
        if committed.replace("\r\n", "\n") != p.read_text(encoding="utf-8").replace("\r\n", "\n"):
            zero_bad.append(f"{rel} 在树 ≠ HEAD 提交态")
    checks["frozen_contracts_and_registries_0byte"] = not zero_bad
    check(not zero_bad, f"契约/冻结表 0-byte 机核: {zero_bad[:3]}")
    note("三 parity 契约 + 三冻结登记表在树 == HEAD 逐字节（0-byte 门序维持）" if not zero_bad else f"0-byte 越界: {zero_bad[:3]}")

    # ── ④ 闭环登记表装载 + 结构化校验（20 行零空行 + 三态 + 逐态义务字面） ──
    closure_doc: dict = {}
    val_errs: list[str] = ["闭环登记表未装载"]
    if CLOSURE_PATH.is_file():
        try:
            closure_doc = load_json(CLOSURE_PATH)
        except (OSError, json.JSONDecodeError) as e:
            val_errs = [f"闭环登记表不可读: {e}"]
    else:
        val_errs = ["g15_gap_fix_closure_registry.json 缺失"]
    items = closure_doc.get("items") or []
    if closure_doc and disp_items:
        val_errs = validate_closure(closure_doc, disp_items)
    checks["closure_registry_20_rows_zero_empty"] = not val_errs
    check(not val_errs, f"闭环登记表校验: {val_errs[:3]}")
    if not val_errs:
        note("闭环登记表 20 行零空行（三态逐态义务字面机核绿）")

    ids_match = bool(items) and bool(disp_items) and (
        [str(it.get("gap_id")) for it in items] == [str(d.get("gap_id")) for d in disp_items]
    )
    checks["gap_id_closed_set_match"] = ids_match
    check(ids_match, "gap_id 闭集与 M-a 处置表逐字对账不全等")

    state_bad = [str(it.get("gap_id")) for it in items
                 if isinstance(it, dict) and it.get("final_disposition") not in FINAL_STATES]
    checks["final_disposition_three_state_closed_set"] = not state_bad and bool(items)
    check(not state_bad, f"三态闭集外值: {state_bad[:3]}")

    anchor_bad = [str(it.get("gap_id")) for it in items
                  if isinstance(it, dict) and it.get("final_disposition") == "open-defer-G16+"
                  and not (_is_nonempty_str(it.get("anchor"))
                           and "重判条件 = " in str(it.get("anchor"))
                           and "兜底 = " in str(it.get("anchor")))]
    checks["open_defer_anchor_verbatim"] = not anchor_bad
    check(not anchor_bad, f"open-defer 承接锚缺字面: {anchor_bad[:3]}")

    calib_bad = [str(it.get("gap_id")) for it in items
                 if isinstance(it, dict) and it.get("final_disposition") == "closed-caliber-registered"
                 and not (_is_nonempty_str(it.get("caliber_registration"))
                          and "RXS-0392" in str(it.get("caliber_registration"))
                          and it.get("kind") == "caliber_diff")]
    checks["closed_caliber_registration_rxs0392"] = not calib_bad
    check(not calib_bad, f"caliber 登记缺 RXS-0392 字面/kind 非 caliber_diff: {calib_bad[:3]}")

    projects = closure_doc.get("fix_projects") or []
    proj_ids = {p.get("fix_id") for p in projects if isinstance(p, dict)}
    resolved_bad = [str(it.get("gap_id")) for it in items
                    if isinstance(it, dict) and it.get("final_disposition") == "closed-resolved"
                    and not (_is_nonempty_str(it.get("fix_id")) and it.get("fix_id") in proj_ids)]
    checks["closed_resolved_rxs0393_red_first"] = not resolved_bad
    check(not resolved_bad, f"closed-resolved 行缺修复立项引用: {resolved_bad[:3]}")

    redfirst_bad = [str(p.get("fix_id")) for p in projects
                    if isinstance(p, dict)
                    and not (_is_nonempty_str(p.get("red_first")) and "RED" in str(p.get("red_first"))
                             and _is_nonempty_str(p.get("green_evidence")))]
    checks["fix_projects_red_first_discipline"] = not redfirst_bad
    check(not redfirst_bad, f"修复项无 RED 先行留痕: {redfirst_bad[:3]}")
    if not projects:
        note("修复立项 = 零（修复评估完结 + 零修复立项 + 20 行终态处置全量——合法退出形态；RED 先行纪律 vacuous 成立如实登记不充绿）")

    # ── ⑤ 触冻结面独立 Full RFC 留痕面（本波零触面机核） ──
    summary = closure_doc.get("summary") or {}
    ledger = load_json(LEDGER_PATH) if LEDGER_PATH.is_file() else {}
    rfc_next_free = ((ledger.get("namespaces") or {}).get("RFC") or {}).get("next_free")
    rfcs_31_plus = sorted(ROOT.glob("rfcs/0031-*.md")) + sorted(ROOT.glob("rfcs/003[2-9]-*.md"))
    frozen_diff = run(["git", "diff", "--name-only", "HEAD", "--",
                       "src", "spec", "conformance",
                       "milestones/g5", "milestones/g6", "milestones/g7", "milestones/g8",
                       "milestones/g9", "milestones/g10", "milestones/g11", "milestones/g12",
                       "milestones/g13", "milestones/g14",
                       "ci/g5_*.py", "ci/g6_*.py", "ci/g7_*.py", "ci/g8_*.py", "ci/g9_*.py",
                       "ci/g10_*.py", "ci/g11_*.py", "ci/g12_*.py", "ci/g13_*.py", "ci/g14_*.py"])
    frozen_changed = [x for x in (frozen_diff.stdout or "").splitlines() if x.strip()]
    porc = run(["git", "status", "--porcelain", "--", "src", "spec", "conformance"])
    alien_bad: list[str] = []
    for ln in (porc.stdout or "").splitlines():
        if not ln.strip():
            continue
        state, path = ln[:2], ln[3:].strip()
        if state == "??":
            if path not in ALIEN_UNTRACKED_SRC:
                alien_bad.append(f"untracked 越界 {path}")
        else:
            alien_bad.append(f"tracked 修改 {path}")
    rfc_rows_ok = True
    for p in projects:
        if isinstance(p, dict) and p.get("frozen_face_touched") is True:
            rfc_ref = str(p.get("rfc") or "")
            m = re.search(r"rfcs/\d{4}-[^\s；;]+\.md", rfc_ref)
            rfc_path = ROOT / m.group(0) if m else None
            ok, detail = wel.rfc_agent_approved(rfc_path) if rfc_path else (False, "rfc 引用不可解析")
            rfc_rows_ok = rfc_rows_ok and ok
            if not ok:
                note(f"修复项 {p.get('fix_id')} 触冻结面 RFC 留痕异常: {detail}")
    frozen_ok = (
        summary.get("frozen_face_touched") is False
        and summary.get("rfc_consumed") == 0
        and rfc_next_free == RFC_NEXT_FREE_FROZEN
        and not rfcs_31_plus
        and not frozen_changed
        and not alien_bad
        and rfc_rows_ok
    )
    checks["frozen_face_zero_rfc_zero"] = bool(frozen_ok)
    check(frozen_ok,
          f"冻结面/RFC 机核: frozen_face_touched={summary.get('frozen_face_touched')!r} "
          f"rfc_consumed={summary.get('rfc_consumed')!r} rfc_next_free={rfc_next_free!r} "
          f"rfcs≥31={len(rfcs_31_plus)} frozen_diff={frozen_changed[:3]} alien={alien_bad[:3]}")
    note(f"触冻结面零发生：RFC next_free={rfc_next_free} 维持 + src/spec/conformance/G5~G14 面 tracked diff 空 + 异己 untracked 闭集机核绿")

    # ── ⑥ 材质链评估 + G15-MA-F1 定论登记 ──
    mc = closure_doc.get("material_chain_assessment") or {}
    mc_verdict = mc.get("verdict")
    mc_ok = (
        mc.get("anchors") == MATERIAL_ANCHORS
        and mc_verdict in ("triggered", "not-triggered")
        and _is_nonempty_str(mc.get("verdict_verbatim"))
        and "透射/焦散/镜面 IBL" in str(mc.get("question") or "")
        and isinstance(mc.get("anchor_literals"), dict)
        and sorted((mc.get("anchor_literals") or {}).keys()) == sorted(MATERIAL_ANCHORS)
        and all(_is_nonempty_str((mc.get("anchor_literals") or {}).get(a)) for a in MATERIAL_ANCHORS)
    )
    if mc_verdict == "not-triggered":
        mc_ok = mc_ok and "未命中" in str(mc.get("verdict_verbatim") or "") and mc.get("full_rfc_required") is False
    elif mc_verdict == "triggered":
        mc_ok = mc_ok and mc.get("full_rfc_required") is True
    checks["material_chain_verdict_registered"] = bool(mc_ok)
    check(mc_ok, "材质链表达面立项评估结论登记异常（G11-N8/G11-N9/G12-N10 承接锚命中判定逐字义务）")
    if mc_ok:
        note(f"材质链评估 verdict={mc_verdict}（三锚命中判定逐字登记）")

    findings = closure_doc.get("findings_adjudication") or []
    f1 = next((f for f in findings if isinstance(f, dict) and f.get("id") == "G15-MA-F1"), None)
    f1_ok = (
        f1 is not None
        and f1.get("verdict") in ("closed-caliber-registered", "open-defer-G16+", "fix-project")
        and _is_nonempty_str(f1.get("verdict_basis")) and "G14.10f" in str(f1.get("verdict_basis"))
        and _is_nonempty_str(f1.get("production_face_status"))
        and _is_nonempty_str(f1.get("parity_face_status"))
        and _is_nonempty_str(f1.get("excluded_fix_faces"))
    )
    checks["g15_ma_f1_adjudicated"] = bool(f1_ok)
    check(f1_ok, "G15-MA-F1 评估定论登记异常")
    if f1_ok:
        note(f"G15-MA-F1 定论 = {f1.get('verdict')}（vendor 输出域评估面登记）")

    eval_bad = [str(it.get("gap_id")) for it in items
                if not (_is_nonempty_str(it.get("fix_evaluation")) and "修复面" in str(it.get("fix_evaluation")))]
    checks["per_row_fix_evaluation_non_empty"] = not eval_bad and bool(items)
    check(not eval_bad, f"逐行修复评估论证缺行: {eval_bad[:3]}")

    # ── ⑦ RED 臂（门内真跑，五臂独立） ──
    if closure_doc and disp_items and not val_errs:
        red_results["missing_row"] = red_arm_missing_row(closure_doc, disp_items)
        red_results["out_of_enum_disposition"] = red_arm_out_of_enum_disposition(closure_doc, disp_items)
        red_results["open_defer_anchor_missing_literal"] = red_arm_open_defer_anchor_missing_literal(closure_doc, disp_items)
        red_results["material_chain_verdict_missing"] = red_arm_material_chain_verdict_missing(closure_doc, disp_items)
        red_results["fix_project_without_red_first"] = red_arm_fix_project_without_red_first(closure_doc, disp_items)
    checks["red_arm_missing_row_detected"] = red_results.get("missing_row") is True
    checks["red_arm_out_of_enum_disposition_detected"] = red_results.get("out_of_enum_disposition") is True
    checks["red_arm_open_defer_anchor_missing_literal_detected"] = red_results.get("open_defer_anchor_missing_literal") is True
    checks["red_arm_material_chain_verdict_missing_detected"] = red_results.get("material_chain_verdict_missing") is True
    checks["red_arm_fix_project_without_red_first_detected"] = red_results.get("fix_project_without_red_first") is True
    for arm, ok in red_results.items():
        check(ok, f"RED 臂 {arm} 注入未检出")
        note(f"RED 臂 {arm}: {'有效' if ok else '失效'}")

    state_counts = {s: sum(1 for it in items if isinstance(it, dict) and it.get("final_disposition") == s)
                    for s in FINAL_STATES} if items else {}
    closure_sha = ""
    if CLOSURE_PATH.is_file():
        closure_sha = "sha256:" + hashlib.sha256(CLOSURE_PATH.read_bytes()).hexdigest()

    host_pass = all(checks.values()) and not FAILURES
    device_state = "executed" if ma_ok else "fail"

    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if host_pass and device_state == "executed" else "fail",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": base_commit(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": f"M-a 最新 evidence 只读核验（{MA_PREFIX}，链锚 wave_start 对账）",
             "exit_code": 0 if checks["m_a_evidence_pass_and_chain_anchor"] else 1},
            {"seq": 2, "command": "M-a 处置表同族校验 + 方向交叉核验重算（20 行消费面）",
             "exit_code": 0 if checks["disposition_table_valid_fresh"] else 1},
            {"seq": 3, "command": "三 parity 契约 + 三冻结登记表 0-byte git 机核",
             "exit_code": 0 if checks["frozen_contracts_and_registries_0byte"] else 1},
            {"seq": 4, "command": "闭环登记表结构化校验（20 行零空行 + 三态逐态义务 + 材质链 + G15-MA-F1）",
             "exit_code": 0 if checks["closure_registry_20_rows_zero_empty"] else 1},
            {"seq": 5, "command": "冻结面/RFC 机核（ledger RFC next_free + tracked diff 空 + 异己闭集）",
             "exit_code": 0 if checks["frozen_face_zero_rfc_zero"] else 1},
            {"seq": 6, "command": "RED 臂 ×5（missing-row/out-of-enum/anchor-literal/material-chain-verdict/fix-no-red-first）",
             "exit_code": 0 if all(v is True for v in red_results.values()) and len(red_results) == 5 else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "production": {
            "correctness_anchor_unchanged": checks["frozen_contracts_and_registries_0byte"],
            "baseline_anchor_id": "milestones/g15/g15_quality_gap_disposition.json（M-a 重收割 fresh measured 面——本门只消费不回写）",
            "measured_value": (
                f"终态三态分布：closed-resolved={state_counts.get('closed-resolved', 0)} "
                f"closed-caliber-registered={state_counts.get('closed-caliber-registered', 0)} "
                f"open-defer-G16+={state_counts.get('open-defer-G16+', 0)}；修复立项 {len(projects)} 项"
            ) if items else "n/a（闭环登记表未装载）",
            "not_worse_than_anchor": ma_ok,
            "threshold_provenance": "容差带 = M-a 处置表 fresh measured_delta tolerance 面（g13_budget/g12_budget 标定条目双 seed 方差底 p100×2.0 程序产，禁手写 P-09）——本门零新阈零手写",
            "evolution_register": (
                "三冻结表 20 行终态 0-byte 只消费不回写；M-a 处置表只消费不回写；"
                "终态处置面另立 milestones/g15/g15_gap_fix_closure_registry.json（逐行评估论证 + "
                "承接锚字面 + 材质链评估 + G15-MA-F1 定论）；零修复立项零 src 变更零 RFC 消费"
            ),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "closure": {
            "m_a_evidence": None if ma_path is None else str(ma_path.relative_to(ROOT)).replace("\\", "/"),
            "m_a_wave_start": ma_wave_start,
            "disposition_file": "milestones/g15/g15_quality_gap_disposition.json",
            "closure_registry_file": "milestones/g15/g15_gap_fix_closure_registry.json",
            "closure_registry_sha256": closure_sha,
            "final_state_counts": state_counts,
            "fix_projects": [p.get("fix_id") for p in projects if isinstance(p, dict)],
            "material_chain_verdict": mc_verdict,
            "g15_ma_f1_verdict": None if f1 is None else f1.get("verdict"),
            "upstream_m_a": {
                "symbolic_gate_key": MA_GATE,
                "subject_prefix": MA_PREFIX,
                "evidence_path": None if ma_path is None else str(ma_path.relative_to(ROOT)).replace("\\", "/"),
                "status": "PASS" if ma_ok else "FAIL",
                "timestamp": ma_doc.get("timestamp"),
            },
        },
    }
    errs = wel.validate_schema(evidence, SCHEMA_PATH) if SCHEMA_PATH.is_file() else ["schema 缺失"]
    if errs:
        print(f"[{TAG}] schema errors: {errs}", file=sys.stderr)
        evidence["status"] = "fail"
        evidence["host_section_pass"] = False
        host_pass = False
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    print(f"[{TAG}] VERDICT={'PASS' if evidence['status'] == 'pass' else 'FAIL'} "
          f"checks={sum(1 for v in checks.values()) if checks else 0}/{len(checks)}")
    return 0 if evidence["status"] == "pass" else 1


def verify_latest() -> int:
    path = wel.load_latest_evidence(SUBJECT)
    if path is None:
        print(f"[{TAG}] FAIL: 缺最新 evidence（{SUBJECT}_*.json）", file=sys.stderr)
        return 1
    doc = wel.load_json(path)
    checks = doc.get("checks") or {}
    bad = [k for k in CHECK_KEYS if checks.get(k) is not True]
    if bad or doc.get("status") != "pass":
        print(f"[{TAG}] FAIL checks={bad} status={doc.get('status')!r}", file=sys.stderr)
        return 1
    print(f"[{TAG}] verify-latest PASS（{path.name}，checks {len(CHECK_KEYS)} 键全绿）")
    return 0


def run_selftest() -> int:
    """schema 闭集对账 + 五 RED 臂函数面 + GREEN 正例（不依赖 device/UE）。"""
    failures = 0
    schema = wel.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        failures += 1
    # 合成正例闭环登记表（1 行最小面 → 校验器 GREEN；闭集/逐态义务面全绿）
    disp_items = [{
        "gap_id": "a" * 16,
        "source_registry": "g12_ue_pt_gap_registry",
        "scene_id": "cornell-box",
        "kind": "caliber_diff",
        "title": "t",
        "direction": "maintained",
        "suggestion": "closed-caliber-registered",
    }]
    good_item = {
        "gap_id": "a" * 16,
        "source_registry": "g12_ue_pt_gap_registry",
        "scene_id": "cornell-box",
        "kind": "caliber_diff",
        "title": "t",
        "m_a_direction": "maintained",
        "m_a_suggestion": "closed-caliber-registered",
        "final_disposition": "closed-caliber-registered",
        "fix_evaluation": "修复面 = 无（口径差行不拟合面）",
        "anchor": None,
        "caliber_registration": "RXS-0392 不拟合原则登记：合成口径面",
        "fix_id": None,
    }
    good = {
        "schema_version": 1,
        "registry": "g15_gap_fix_closure_registry",
        "generated_by": GENERATED_BY,
        "wave": WAVE,
        "m_a_evidence": "evidence/x.json",
        "m_a_wave_start": "20260823T000000Z",
        "disposition_source": "milestones/g15/g15_quality_gap_disposition.json",
        "items": [good_item],
        "fix_projects": [],
        "material_chain_assessment": {
            "anchors": list(MATERIAL_ANCHORS),
            "question": "透射/焦散/镜面 IBL 类能量是否成为画质量级 measured 主差",
            "verdict": "not-triggered",
            "verdict_verbatim": "未命中——合成面",
            "per_family_attribution": [{"family": "f", "attribution": "a", "material_chain_energy_primary": False}],
            "anchor_literals": {a: f"{a} 兜底字面维持" for a in MATERIAL_ANCHORS},
            "full_rfc_required": False,
            "full_rfc_note": "合成面",
        },
        "findings_adjudication": [{
            "id": "G15-MA-F1",
            "title": "t",
            "verdict": "closed-caliber-registered",
            "verdict_basis": "G14.10f 修复面论证合成字面",
            "production_face_status": "p",
            "parity_face_status": "q",
            "excluded_fix_faces": "r",
        }],
        "summary": {
            "closed_resolved": 0,
            "closed_caliber_registered": 1,
            "open_defer_g16_plus": 0,
            "fix_projects_count": 0,
            "frozen_face_touched": False,
            "rfc_consumed": 0,
        },
    }
    verrs = validate_closure(good, disp_items)
    if verrs:
        print(f"[{TAG}] selftest FAIL: 合形闭环登记表被误拒 {verrs}", file=sys.stderr)
        failures += 1
    if not red_arm_missing_row(good, disp_items):
        print(f"[{TAG}] selftest FAIL: missing-row 臂未检出", file=sys.stderr)
        failures += 1
    if not red_arm_out_of_enum_disposition(good, disp_items):
        print(f"[{TAG}] selftest FAIL: out-of-enum 臂未检出", file=sys.stderr)
        failures += 1
    # anchor-literal 臂需 open-defer 行正例
    defer_doc = copy.deepcopy(good)
    defer_doc["items"][0]["kind"] = "quality_gap"
    defer_doc["items"][0]["m_a_suggestion"] = "open-defer-G16+"
    defer_doc["items"][0]["final_disposition"] = "open-defer-G16+"
    defer_doc["items"][0]["anchor"] = "重判条件 = x；兜底 = y"
    defer_doc["items"][0]["caliber_registration"] = None
    defer_doc["summary"]["closed_caliber_registered"] = 0
    defer_doc["summary"]["open_defer_g16_plus"] = 1
    defer_disp = [dict(disp_items[0], kind="quality_gap", suggestion="open-defer-G16+")]
    if validate_closure(defer_doc, defer_disp):
        print(f"[{TAG}] selftest FAIL: open-defer 合形行被误拒 {validate_closure(defer_doc, defer_disp)}", file=sys.stderr)
        failures += 1
    if not red_arm_open_defer_anchor_missing_literal(defer_doc, defer_disp):
        print(f"[{TAG}] selftest FAIL: anchor-literal 臂未检出", file=sys.stderr)
        failures += 1
    if not red_arm_material_chain_verdict_missing(good, disp_items):
        print(f"[{TAG}] selftest FAIL: material-chain-verdict 臂未检出", file=sys.stderr)
        failures += 1
    if not red_arm_fix_project_without_red_first(good, disp_items):
        print(f"[{TAG}] selftest FAIL: fix-no-red-first 臂未检出", file=sys.stderr)
        failures += 1
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)}（schema 闭集 + 5 RED + 2 GREEN 函数面臂）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G15.3 P0 硬门 M-b 修复闭环")
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return verify_latest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
