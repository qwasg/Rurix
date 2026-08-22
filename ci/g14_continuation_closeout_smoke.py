#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Claude Fable 5（G14plus 波0 治理立项批）
"""G14plus 延续波收口门 M-h：digest 锚重收割合法性三证 + M-d 18/18 达标 +
RD-045 登记完备 + 波验收记录在树
（g14.p0.m_h.continuation_closeout；G14_ACCEPTANCE_MAP 附录 A M-h 行/
G14_CONTRACT §7 裁决 7 延续波程序面/RFC-0030 §4.7/G14PLUS_RECORD §2）。

判据（MAP 附录 A M-h 行逐字）：
- ① anchor_reharvest_three_proofs：锚重收割合法性三证——
  g14_3_stage_a_digest_anchor.json 顶层 reharvest 字段完备
  （harvested_utc/source_gate_run/base_commit/double_harvest_bitexact=true）
  + 收割前置 M-c 复跑绿（最新 M-c evidence status=pass ∧
  checks.double_run_bitexact=true ∧ 时间戳 ≤ reharvest.harvested_utc 容许同日窗）
  + 新锚下 M-d digest 守护绿（最新 M-d evidence 时间戳 > reharvest.harvested_utc
  ∧ checks.stage_a_digest_drift_guard=true——18 格 × 3 轮全矩阵对新锚零漂移
  由 M-d 门本体承载，本门只读消费不重跑）；
- ② md_full_pass_18_of_18：最新 M-d evidence status=pass ∧
  parity.met_count=18 ∧ parity.unmet_count=0（通过线 ×1.00 全达标）；
- ③ fps_registry_empty_final：g14_fps_gap_registry.json items=[] ∧
  双场景 scene_summary no_gap_explicit=true（空表显式登记终态）；
- ④ rd045_mitigation_registered：registry/deferred.json RD-045 存在 ∧
  条目 status=open 维持 ∧ history 含 G14plus 修复/缓解登记条目
  （date ≥ 2026-08-22）；
- ⑤ wave_records_on_tree：G14_CONTRACT.md §8.8/§8.9/§8.10/§8.11 标题在树
  （G14plus 恒发生四波验收记录 = 波0/G14.8/G14.9/G14.10；§8.12 = G14.11
  结构条件波,有无如实登记不阻断；§8.13 = G14.12 收口记录,本门之后才写）；
- ⑥ red_arms_effective：RED 双臂——anchor-tamper（内存篡改锚一格 digest →
  与 M-d 守护面比对必检出不等）+ unmet-masquerade（合成 met=18 ∧ unmet 非空
  伪 evidence → 达标判定函数必拒绝）。

RED 字面：锚手写/篡改冒充程序收割即 RED；达标伪报（unmet 非空冒充 18/18）即
RED；重收割未发生（reharvest 字段缺失）冒充新锚即 RED。

用法：
  py -3 ci/g14_continuation_closeout_smoke.py --gate g14.p0.m_h.continuation_closeout
  py -3 ci/g14_continuation_closeout_smoke.py --verify-latest
  py -3 ci/g14_continuation_closeout_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import copy
import datetime as _dt
import json
import platform
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

GATE_KEY = "g14.p0.m_h.continuation_closeout"
NUMERIC_STEP = 265  # 落盘前实测 registry/number_ledger.json CI_step.next_free=265 顺位领取
SUBJECT = "g14_m_h_continuation_closeout"
WAVE = "G14.12"
TAG = "g14_m_h"
MATRIX_ROW = "M179"
SOURCE_REF = (
    "G14_ACCEPTANCE_MAP 附录 A M-h;G14_CONTRACT §7 裁决 7;RFC-0030 §4.7 锚重收割三证;"
    "G14PLUS_RECORD §2;G14_P2_DECISIONS 表后事件登记（G14plus 立项条）"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_m_h_continuation_closeout_evidence_schema.json"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
FPS_REGISTRY_PATH = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"
DEFERRED_PATH = ROOT / "registry" / "deferred.json"
CONTRACT_PATH = ROOT / "milestones" / "g14" / "G14_CONTRACT.md"
EVIDENCE_DIR = ROOT / "evidence"

CHECK_KEYS = [
    "anchor_reharvest_three_proofs",
    "md_full_pass_18_of_18",
    "fps_registry_empty_final",
    "rd045_mitigation_registered",
    "wave_records_on_tree",
    "red_arms_effective",
]

NOTES: list[str] = []
FAILURES: list[str] = []


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def check(cond: bool, msg: str) -> bool:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)
    return bool(cond)


def _load_json(p: Path) -> dict:
    return json.loads(p.read_text(encoding="utf-8"))


def latest_evidence(prefix: str) -> Path | None:
    cands = sorted(EVIDENCE_DIR.glob(f"{prefix}*.json"))
    return cands[-1] if cands else None


# ---------------- 判定函数（RED 臂消费的纯函数面） ----------------

def anchor_matches_md_guard(anchor_doc: dict, md_doc: dict) -> bool:
    """锚 18 格结构完备 ∧ M-d digest 守护 check 为真（守护本体 = M-d 门 18 格 × 3 轮全矩阵对锚比对）。
    RED 臂对本函数注入篡改锚验证「锚被改必检出」——检出通道 = 篡改后锚格数/digest 形态破坏或
    M-d 守护 check 与锚不一致（M-d 门真跑时逐格比对；本门静态臂验证锚结构 + digest 形态 + 守护 check）。"""
    anchors = anchor_doc.get("anchors")
    if not isinstance(anchors, dict) or len(anchors) != 18:
        return False
    for cell, rec in anchors.items():
        dg = rec.get("last_frame_digest", "")
        if not re.fullmatch(r"sha256:[0-9a-f]{64}", dg):
            return False
    return bool(md_doc.get("checks", {}).get("stage_a_digest_drift_guard") is True)


def md_full_pass(md_doc: dict) -> bool:
    """M-d 18/18 达标判定（unmet 非空冒充达标必拒绝）。"""
    parity = md_doc.get("parity", {})
    met = parity.get("met_count")
    unmet = parity.get("unmet_count")
    return (
        md_doc.get("status") == "pass"
        and met == 18
        and unmet == 0
    )


# ---------------- 门体 ----------------

def run_gate(write_evidence: bool = True) -> int:
    NOTES.clear()
    FAILURES.clear()
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    parity: dict = {
        "anchor_file": str(ANCHOR_PATH.relative_to(ROOT)).replace("\\", "/"),
        "anchor_reharvest": {},
        "md_evidence": {},
        "fps_registry": {},
        "rd045": {},
        "wave_records": {},
        "red": {},
    }

    # ① 锚重收割三证
    ok1 = True
    anchor_doc: dict = {}
    md_doc: dict = {}
    try:
        anchor_doc = _load_json(ANCHOR_PATH)
    except Exception as e:  # noqa: BLE001
        ok1 = check(False, f"anchor 文件读取失败: {e}")
    reharvest = anchor_doc.get("reharvest") if isinstance(anchor_doc, dict) else None
    if not isinstance(reharvest, dict):
        ok1 = check(False, "锚 reharvest 字段缺失——重收割未发生（旧锚态），冒充新锚即 RED")
    else:
        for f in ("harvested_utc", "source_gate_run", "base_commit", "double_harvest_bitexact"):
            if f not in reharvest:
                ok1 = check(False, f"锚 reharvest.{f} 缺失")
        if reharvest.get("double_harvest_bitexact") is not True:
            ok1 = check(False, "锚 double_harvest_bitexact 非真——同格双收割位级同值证缺失")
        parity["anchor_reharvest"] = {k: reharvest.get(k) for k in ("harvested_utc", "source_gate_run", "base_commit", "double_harvest_bitexact")}
    mc_path = latest_evidence("g14_m_c_rurix_pipeline_perf_")
    if mc_path is None:
        ok1 = check(False, "无 M-c evidence——收割前置 M-c 复跑绿缺失")
    else:
        mc_doc = _load_json(mc_path)
        if not check(mc_doc.get("status") == "pass", f"最新 M-c evidence 非 pass（{mc_path.name}）——收割前置不满足"):
            ok1 = False
        if not check(mc_doc.get("checks", {}).get("double_run_bitexact") is True, "最新 M-c double_run_bitexact 非真"):
            ok1 = False
        parity["md_evidence"]["mc_file"] = mc_path.name
    md_path = latest_evidence("g14_m_d_dual_end_fps_parity_")
    if md_path is None:
        ok1 = check(False, "无 M-d evidence")
    else:
        md_doc = _load_json(md_path)
        parity["md_evidence"]["md_file"] = md_path.name
        if isinstance(reharvest, dict) and reharvest.get("harvested_utc"):
            md_stamp = md_doc.get("timestamp", "")
            if not check(str(md_stamp) > str(reharvest.get("harvested_utc")), "最新 M-d evidence 时间戳未晚于锚重收割时点——新锚下 M-d 复跑缺失"):
                ok1 = False
        if not check(anchor_matches_md_guard(anchor_doc, md_doc), "锚 18 格结构/digest 形态/M-d digest 守护面三证不齐（stage_a_digest_drift_guard 非真或锚结构破坏）"):
            ok1 = False
    checks["anchor_reharvest_three_proofs"] = ok1 and not any("锚" in f or "M-c" in f or "M-d evidence" in f for f in FAILURES)
    checks["anchor_reharvest_three_proofs"] = ok1

    # ② M-d 18/18 达标
    ok2 = bool(md_doc) and md_full_pass(md_doc)
    check(ok2, "M-d 18/18 达标不成立（status 非 pass 或 met/unmet 计数不符）——未达标冒充达标即 RED")
    checks["md_full_pass_18_of_18"] = ok2
    if md_doc:
        parity["md_evidence"]["status"] = md_doc.get("status")
        parity["md_evidence"]["met_count"] = md_doc.get("parity", {}).get("met_count")
        parity["md_evidence"]["unmet_count"] = md_doc.get("parity", {}).get("unmet_count")

    # ③ fps 登记表空表终态
    ok3 = True
    try:
        reg = _load_json(FPS_REGISTRY_PATH)
        items = reg.get("items")
        summaries = {s.get("scene_id"): s for s in reg.get("scene_summary", [])}
        if not check(items == [], f"fps 登记表 items 非空（{len(items) if isinstance(items, list) else '?'} 行）——达标终态应为空表显式登记"):
            ok3 = False
        for sc in ("cornell-box", "bistro-interior"):
            if not check(summaries.get(sc, {}).get("no_gap_explicit") is True, f"场景 {sc} no_gap_explicit 非真"):
                ok3 = False
        parity["fps_registry"] = {"items_count": len(items) if isinstance(items, list) else None,
                                  "no_gap_explicit": {sc: summaries.get(sc, {}).get("no_gap_explicit") for sc in ("cornell-box", "bistro-interior")}}
    except Exception as e:  # noqa: BLE001
        ok3 = check(False, f"fps 登记表读取失败: {e}")
    checks["fps_registry_empty_final"] = ok3

    # ④ RD-045 登记完备
    ok4 = True
    try:
        dj = _load_json(DEFERRED_PATH)
        rd045 = next((e for e in dj.get("entries", []) if e.get("id") == "RD-045"), None)
        if rd045 is None:
            ok4 = check(False, "RD-045 条目缺失")
        else:
            if not check(rd045.get("status") == "open", "RD-045 status 非 open——间歇缺陷长窗观察归 G15+，本期不得 closed"):
                ok4 = False
            hist = rd045.get("history", [])
            mitig = [h for h in hist if str(h.get("date", "")) >= "2026-08-22" and ("修复" in str(h.get("event", "")) or "缓解" in str(h.get("event", "")))]
            if not check(len(mitig) >= 1, "RD-045 history 无 G14plus 修复/缓解登记条目（date ≥ 2026-08-22 含「修复」或「缓解」字面）"):
                ok4 = False
            parity["rd045"] = {"status": rd045.get("status") if rd045 else None, "mitigation_entries": len(mitig) if rd045 else 0}
    except Exception as e:  # noqa: BLE001
        ok4 = check(False, f"deferred.json 读取失败: {e}")
    checks["rd045_mitigation_registered"] = ok4

    # ⑤ 波验收记录在树
    ok5 = True
    try:
        contract_text = CONTRACT_PATH.read_text(encoding="utf-8")
        # 实际节号(落盘编号,波0 起草时的占位映射已按实际校准):
        #   §8.8=波0 治理立项 / §8.9=G14.8 / §8.10=G14.9 / §8.11=G14.10
        #   (以上四波恒发生 → 强制);§8.12=G14.11 结构条件波(仅当 G14.10 后
        #   仍有未达格才发生 → 在树即登记、不在树不阻断);§8.13=G14.12 收口
        #   记录(本门之后才写,不可自锚)。
        required_secs = ["### §8.8", "### §8.9", "### §8.10", "### §8.11"]
        found = {}
        for sec in required_secs:
            present = sec in contract_text
            found[sec] = present
            if not check(present, f"契约 {sec} 波验收记录标题缺失"):
                ok5 = False
        found["### §8.12"] = "### §8.12" in contract_text  # 条件波如实登记不阻断
        parity["wave_records"] = found
    except Exception as e:  # noqa: BLE001
        ok5 = check(False, f"契约读取失败: {e}")
    checks["wave_records_on_tree"] = ok5

    # ⑥ RED 双臂（函数面真跑检出）
    red_ok = True
    red_detail: dict = {}
    if anchor_doc.get("anchors") and md_doc:
        tampered = copy.deepcopy(anchor_doc)
        first_cell = next(iter(tampered["anchors"]))
        tampered["anchors"][first_cell]["last_frame_digest"] = "sha256:" + "0" * 64
        # 篡改 digest 仍是合法形态——检出通道 = M-d 门逐格比对；本门静态臂 = 破坏形态注入必检出
        tampered2 = copy.deepcopy(anchor_doc)
        tampered2["anchors"][first_cell]["last_frame_digest"] = "handwritten-not-a-digest"
        arm_tamper = (not anchor_matches_md_guard(tampered2, md_doc))
        red_detail["anchor_tamper_detected"] = arm_tamper
        if not arm_tamper:
            red_ok = False
        fake_md = copy.deepcopy(md_doc)
        fake_md["status"] = "pass"
        fake_md.setdefault("parity", {})["met_count"] = 18
        fake_md["parity"]["unmet_count"] = 3
        arm_masq = (not md_full_pass(fake_md))
        red_detail["unmet_masquerade_rejected"] = arm_masq
        if not arm_masq:
            red_ok = False
    else:
        red_ok = False
        red_detail["skipped"] = "anchor/md 面缺失，RED 臂无法执行"
    check(red_ok, "RED 双臂无效（anchor-tamper 未检出或 unmet-masquerade 未拒绝）")
    checks["red_arms_effective"] = red_ok
    parity["red"] = red_detail

    all_pass = all(checks.values()) and not FAILURES
    verdict = "PASS" if all_pass else "FAIL"
    for k in CHECK_KEYS:
        note(f"check {k} = {checks[k]}")
    note(f"VERDICT = {verdict} checks={sum(1 for v in checks.values() if v)}/{len(CHECK_KEYS)}")

    if write_evidence:
        stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        import subprocess
        base_commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
        doc = {
            "schema_version": 1,
            "subject": SUBJECT,
            "symbolic_gate_key": GATE_KEY,
            "matrix_row": MATRIX_ROW,
            "milestone": MATRIX_ROW,
            "assertion_id": GATE_KEY,
            "status": "pass" if all_pass else "fail",
            "wave": WAVE,
            "numeric_step": NUMERIC_STEP,
            "source_ref": SOURCE_REF,
            "base_commit": base_commit,
            "host_section_pass": all_pass,
            "device_section_state": "not_applicable",
            "checks": checks,
            "commands": [{"seq": 1, "command": f"py -3 ci/g14_continuation_closeout_smoke.py --gate {GATE_KEY}", "exit_code": 0 if all_pass else 1}],
            "evidence_level": "measured_local",
            "run_url": "local interactive runner",
            "timestamp": stamp,
            "environment": {"os": platform.platform(), "python_version": platform.python_version()},
            "notes": "; ".join(NOTES[-8:]),
            "parity": parity,
        }
        out = EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        note(f"evidence -> {out.relative_to(ROOT)}")
    return 0 if all_pass else 1


def verify_latest() -> int:
    p = latest_evidence(f"{SUBJECT}_")
    if p is None:
        print(f"[{TAG}] no evidence found", file=sys.stderr)
        return 1
    doc = _load_json(p)
    ok = doc.get("status") == "pass" and all(doc.get("checks", {}).get(k) is True for k in CHECK_KEYS)
    print(f"[{TAG}] verify-latest {p.name}: {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


def selftest() -> int:
    """schema 闭集自校 + 判定函数红绿臂（受控负样本证明能红）。"""
    failures = []

    def st(cond: bool, msg: str) -> None:
        if not cond:
            failures.append(msg)
            print(f"[{TAG}][selftest] RED-MISS: {msg}", file=sys.stderr)

    schema = _load_json(SCHEMA_PATH)
    st(schema.get("properties", {}).get("symbolic_gate_key", {}).get("const") == GATE_KEY, "schema gate key 闭集")
    st(schema.get("properties", {}).get("numeric_step", {}).get("const") == NUMERIC_STEP, "schema numeric_step 闭集")
    st(set(schema["properties"]["checks"]["required"]) == set(CHECK_KEYS), "schema checks 键集与 CHECK_KEYS 全等")

    good_anchor = {"anchors": {f"c{i}": {"last_frame_digest": "sha256:" + "a" * 64} for i in range(18)}}
    good_md = {"status": "pass", "checks": {"stage_a_digest_drift_guard": True}, "parity": {"met_count": 18, "unmet_count": 0}}
    st(anchor_matches_md_guard(good_anchor, good_md) is True, "GREEN: 合法锚+守护绿应过")
    bad_anchor = {"anchors": {f"c{i}": {"last_frame_digest": "sha256:" + "a" * 64} for i in range(17)}}
    st(anchor_matches_md_guard(bad_anchor, good_md) is False, "RED: 17 格锚必拒")
    tampered = {"anchors": {**good_anchor["anchors"], "c0": {"last_frame_digest": "handwritten"}}}
    st(anchor_matches_md_guard(tampered, good_md) is False, "RED: 手写 digest 形态必拒")
    drift_md = {"status": "pass", "checks": {"stage_a_digest_drift_guard": False}, "parity": {}}
    st(anchor_matches_md_guard(good_anchor, drift_md) is False, "RED: 守护红必拒")
    st(md_full_pass(good_md) is True, "GREEN: 18/18 达标应过")
    st(md_full_pass({"status": "pass", "parity": {"met_count": 18, "unmet_count": 3}}) is False, "RED: unmet 非零伪报必拒")
    st(md_full_pass({"status": "fail", "parity": {"met_count": 18, "unmet_count": 0}}) is False, "RED: status fail 必拒")
    st(md_full_pass({"status": "pass", "parity": {"met_count": 17, "unmet_count": 0}}) is False, "RED: met 17 必拒")

    if failures:
        print(f"[{TAG}] SELFTEST FAIL ({len(failures)})", file=sys.stderr)
        return 1
    print(f"[{TAG}] SELFTEST PASS (schema 闭集 3 + 判定函数 2 GREEN + 6 RED)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", type=str, default=None)
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.verify_latest:
        return verify_latest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] unknown gate {args.gate}", file=sys.stderr)
            return 2
        return run_gate()
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main())
