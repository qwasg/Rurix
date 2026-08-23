#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G15.6b close-out 波）
"""G15.6b close-out 终审 g15.wave.6b.closeout（G15_CONTRACT G-G15-9；同构
ci/g14_closeout_check.py〔G14.5b〕先例）。

只读汇总八 facts：①5 P0 key（M-a~M-e）逐门 PASS（wel 口径 + 顶层 status=="pass"
字面）——**M-d 诚实红门面特判**（G14 closeout 先例同型字面）：G15 M-d
status=="fail" ∧ 绿键 6 全真（画质锚带复核/budget 零 estimated/RED 四臂）∧
红键 6 全假闭集 ∧ 消费的 G14 M-d 复跑件 checks 全绿 ∧ status=="fail" ∧
unmet_count == g14_fps_gap_registry 行数（1 行 gap_id 51a150cb4523e8b6）∧
G15-MD-F1 承接锚在案 = 未达标如实登记不充绿亦不充降级（终审定盘如实登记不
冒充）+ ②wave2/3/4/5 exit + wave6a decisions 五聚合/决策门全 PASS（wave5 红面
特判同型：M-d 行 FAIL 镜像 + facts ④⑤ 绿 + 红 facts ⊆ M-d 红同源闭集）+
③g15_acceptance_map_check 双向 exit=0 + ④P2 决策表 40 行闭集最终状态无漂移
（最新 evidence host_section_pass + FROZEN_IDS 40 行闭集在树）+ ⑤budget
--strict 非空零 estimated/skip + ⑥6a full-run 先行（base_commit_6a 留痕；
同日放行沿 G14 立项裁决先例：6a full-run 先行完成后允许同日 close-out）+
⑦RD 最终状态逐字一致（deferred.json RD-034/039/040/041/042/043/044/045 八条
目级 status 全 open 逐字 + P2 40 行 FROZEN_IDS 闭集在树——RD-045 零检出维持
open 不关闭字面）+ **⑧双未达标终审定盘**（如实登记不冒充——商用收口 0/18
画质面：M-c 最新 evidence PASS ∧ commercial_closure verdict==未达标 ∧
met_count==0/18 ∧ 18 格 unmet_attribution 非空 ∧ g16_anchor 授权面字面 ∧
G15-MC-F1 参照死黑退化归因在案；性能 17/18 单格环境事件面：G14 M-d 最新
evidence status==fail ∧ met_count==17/unmet==1 ∧ g14_fps_gap_registry 1 行
gap_id 51a150cb4523e8b6 ∧ G15-MD-F1 承接锚字面在案〔UE 参照臂缓存暖态跨会话
位移 + NGX 物理不可达终版〕；**G16+ 承接锚三面齐备**：① GI 表达面 + UE 参照臂
修复（g15_gap_fix_closure_registry lumen 2 行 GI 表达面承接锚 + G15-MC-F1 行
UE 参照臂修复承接锚）② DLSS NGX 版本与宿主车道（G15-MD-F1 承接锚——NGX
版本演进面/D3D12 宿主车道架构面/税源分解实测约束）③ 绝对画质 deficit 收口
（M-c commercial_closure.g16_anchor + G15-P2 §5 汇总字面 + 16 行 open-defer
承接锚）；用户 2026-08-19 授权面逐字承接，性能零降级守护面终态锁定）+
最后新绿 UTC 日留痕。

输出 VERDICT = READY|BLOCKED。status flip 可与 READY 同波独立 commit。

用法：
  py -3 ci/g15_closeout_check.py --gate g15.wave.6b.closeout
  py -3 ci/g15_closeout_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g10_wave_exit_lib as wel  # noqa: E402
from g15_p2_decisions_check import FROZEN_IDS  # noqa: E402
from g15_regression_drift_guard_smoke import (  # noqa: E402
    FPS_GAP_ID,
    G15_MD_PREFIX,
    HONEST_RED_PREFIX,
)
from g15_stabilization_soak import (  # noqa: E402
    eval_honest_red_wave5,
    verify_g15_md_honest_red,
)

ROOT = wel.ROOT
GATE_KEY = "g15.wave.6b.closeout"
NUMERIC_STEP = 280  # 落盘前实测 registry/number_ledger.json CI_step.next_free=280 顺位领取
SUBJECT = "g15_wave6b_closeout"
WAVE = "G15.6b"
SOURCE_REF = (
    "G15_CONTRACT G-G15-9;G15_ACCEPTANCE_MAP §7;"
    "5 P0（M-d 诚实红特判）+ wave2/3/4/5/6a 聚合/决策（wave5 红面特判同型）+ MAP 双向 + P2 40 行闭集 + budget --strict + 6a 先行"
    "（同日放行沿 G14 立项裁决先例）+ RD 最终状态逐字一致（八条 open——RD-045 零检出维持 open 不关闭）+ 双未达标终审定盘"
    "（商用收口 0/18 画质面 + 性能 17/18 单格环境事件面——如实登记不冒充 + G16+ 承接锚三面齐备）"
)
SCHEMA_PATH = ROOT / "milestones" / "g15" / "g15_wave6b_closeout_evidence_schema.json"
P2_TABLE_PATH = ROOT / "milestones" / "g15" / "G15_P2_DECISIONS.md"
FPS_REGISTRY_PATH = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"
G15_CONTRACT = ROOT / "milestones" / "g15" / "G15_CONTRACT.md"
CLOSURE_REGISTRY = ROOT / "milestones" / "g15" / "g15_gap_fix_closure_registry.json"
MC_PREFIX = "g15_m_c_absolute_quality_final_review"

# 5 P0（G15_ACCEPTANCE_MAP §1 实记；key/prefix 与
# ci/g15_stabilization_soak.py P0_GATES 同一闭集）。
P0_KEYS = [
    ("g15.p0.m_a.dual_end_quality_reharvest", "g15_m_a_dual_end_quality_reharvest"),
    ("g15.p0.m_b.gap_fix_closure_loop", "g15_m_b_gap_fix_closure_loop"),
    ("g15.p0.m_c.absolute_quality_final_review", MC_PREFIX),
    ("g15.p0.m_d.perf_parity_zero_regression", G15_MD_PREFIX),
    ("g15.p0.m_e.regression_drift_guard", "g15_m_e_regression_drift_guard"),
]

WAVE_GATES = [
    ("g15.wave.2.exit", "g15_wave2_exit"),
    ("g15.wave.3.exit", "g15_wave3_exit"),
    ("g15.wave.4.exit", "g15_wave4_exit"),
    ("g15.wave.5.exit", "g15_wave5_exit"),
    ("g15.wave.6a.decisions", "g15_p2_decisions"),
]
WAVE5_PREFIX = "g15_wave5_exit"

# G15_CONTRACT §6 Deferred 处置表字面：八条目级 status 全维持 open
# （RD-045 = G15 M-e 监控臂承接——零检出维持 open 不关闭字面）。
RD_FINAL_OPEN_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044", "RD-045"]

G16_AUTH_LITERAL = "允许在G15后无限制新建里程碑继续优化"
_UTC_STAMP_RE = re.compile(r"_(\d{8}T\d{6}Z)\.json$")


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def verify_key_gate(key: str, prefix: str) -> dict:
    """5 P0 最新 evidence 终审核验：wel 口径 + 顶层 status=="pass" 字面。
    M-d 诚实红门面特判（G14 closeout 先例同型字面）：verify_g15_md_honest_red
    合格 = 未达标如实登记面——红不充绿亦不充降级（终审定盘如实登记不冒充）。"""
    row = wel.require_gate_pass(key, prefix)
    path = wel.load_latest_evidence(prefix)
    if path is None:
        row["status"] = "FAIL"
        row["detail"] = f"缺最新 evidence（{prefix}_*.json）"
        return row
    try:
        doc = wel.load_json(path)
    except (OSError, ValueError):
        return row
    if prefix == G15_MD_PREFIX:
        ok, detail = verify_g15_md_honest_red()
        if ok:
            row["status"] = "PASS"
            row["detail"] = f"{row.get('detail','')}; {detail}"
        else:
            row["status"] = "FAIL"
            row["detail"] = detail
        return row
    if doc.get("status") != "pass":
        row["status"] = "FAIL"
        row["detail"] = f"{row.get('detail', '')}; status={doc.get('status')!r} ≠ 'pass'"
    return row


def evidence_utc_date(path: Path | None) -> str | None:
    if path is None:
        return None
    m = _UTC_STAMP_RE.search(path.name)
    if m:
        return m.group(1)[:8]
    doc = wel.load_json(path)
    ts = doc.get("timestamp") or doc.get("utc_date") or ""
    return str(ts)[:8] if ts else None


def max_first_pass_date() -> tuple[str | None, list[str]]:
    """对 P0 取最新 PASS evidence 的 UTC 日期的 max（近似『最后新绿』）。
    M-d 双态：诚实红期无 PASS 态跳过（终审定盘归 fact⑧）；达标翻转后
    （status=="pass"）纳入日期面。"""
    dates: list[str] = []
    missing: list[str] = []
    for key, prefix in P0_KEYS:
        p = wel.load_latest_evidence(prefix)
        if prefix == G15_MD_PREFIX:
            if p is not None and wel.load_json(p).get("status") == "pass":
                d = evidence_utc_date(p)
                if d:
                    dates.append(d)
            continue  # 诚实红期无 PASS 态——终审定盘归 fact⑧,不计缺失
        if p is None:
            missing.append(key)
            continue
        doc = wel.load_json(p)
        ok, _ = wel.gate_pass_reason(doc, key)
        if not ok or doc.get("status") != "pass":
            missing.append(key)
            continue
        d = evidence_utc_date(p)
        if d:
            dates.append(d)
    if not dates:
        return None, missing
    return max(dates), missing


def check_rd_final_state() -> tuple[bool, str]:
    """G-G15-9「验收映射、候选决策、RD 最终状态逐字一致」机器化面。"""
    problems: list[str] = []
    for rd in RD_FINAL_OPEN_IDS:
        st = wel.load_rd_status(rd)
        if st != "open":
            problems.append(f"{rd} status={st!r} ≠ 'open'")
    if not P2_TABLE_PATH.is_file():
        problems.append("G15_P2_DECISIONS.md 缺失")
    else:
        text = P2_TABLE_PATH.read_text(encoding="utf-8")
        absent = [i for i in FROZEN_IDS if i not in text]
        if absent:
            problems.append(f"P2 表缺 FROZEN_IDS: {absent}")
        if len(FROZEN_IDS) != 40:
            problems.append(f"FROZEN_IDS n={len(FROZEN_IDS)} ≠ 40（闭集口径漂移）")
    return (not problems), "; ".join(problems) if problems else (
        "8 RD open 逐字一致（RD-045 零检出维持 open 不关闭字面）+ P2 40 行闭集在树（全表深对账由 wave6a decisions 门承载）"
    )


def check_dual_unmet_finalized() -> tuple[bool, str]:
    """G-G15-9「商用收口终审定盘」机器化面——双未达标如实登记不冒充 +
    G16+ 承接锚三面齐备 + 性能零降级守护面终态锁定。"""
    problems: list[str] = []
    # ── 画质面：商用收口 0/18 定盘 ──
    mc_path = wel.load_latest_evidence(MC_PREFIX)
    if mc_path is None:
        problems.append("缺最新 M-c evidence")
    else:
        mc = wel.load_json(mc_path)
        cc = (mc.get("parity") or {}).get("commercial_closure") or {}
        attribution = cc.get("unmet_attribution") or {}
        if mc.get("status") != "pass":
            problems.append(f"M-c 最新 evidence status={mc.get('status')!r} ≠ pass")
        if cc.get("verdict") != "未达标" or cc.get("met_count") != 0 or cc.get("total") != 18:
            problems.append(
                f"commercial_closure 定盘异常: verdict={cc.get('verdict')!r} met={cc.get('met_count')}/{cc.get('total')}"
            )
        if len(cc.get("unmet_cells") or []) != 18 or len(attribution) != 18 or not all(attribution.values()):
            problems.append("unmet_cells/attribution 非 18 格全量（未达格逐格归因缺失即冒充面）")
        if "ue_reference_degenerate" not in json.dumps(attribution, ensure_ascii=False):
            problems.append("cornell 格 ue_reference_degenerate（G15-MC-F1）归因缺失")
        g16_anchor = cc.get("g16_anchor") or ""
        if G16_AUTH_LITERAL not in g16_anchor or "重判条件" not in g16_anchor or "兜底" not in g16_anchor:
            problems.append("g16_anchor 授权面/承接锚字面缺失")
    # ── 性能面：17/18 单格环境事件面定盘 ──
    md_path = wel.load_latest_evidence(HONEST_RED_PREFIX)
    if md_path is None:
        problems.append("缺最新 G14 M-d evidence")
    else:
        md_doc = wel.load_json(md_path)
        parity = md_doc.get("parity") or {}
        if md_doc.get("status") != "fail" or parity.get("met_count") != 17 or parity.get("unmet_count") != 1:
            problems.append(
                f"G14 M-d 定盘异常: status={md_doc.get('status')!r} met={parity.get('met_count')} unmet={parity.get('unmet_count')}"
            )
    if not FPS_REGISTRY_PATH.is_file():
        problems.append("g14_fps_gap_registry 缺失")
    else:
        reg = wel.load_json(FPS_REGISTRY_PATH)
        items = reg.get("items") or []
        ids = {it.get("gap_id") for it in items}
        if len(items) != 1 or ids != {FPS_GAP_ID}:
            problems.append(f"g14 帧率表行集漂移: n={len(items)} ids={sorted(ids)}")
    contract_text = G15_CONTRACT.read_text(encoding="utf-8") if G15_CONTRACT.is_file() else ""
    if "G15-MD-F1" not in contract_text:
        problems.append("G15_CONTRACT 缺 G15-MD-F1 承接锚字面")
    # ── G16+ 承接锚三面齐备 ──
    clo = wel.load_json(CLOSURE_REGISTRY) if CLOSURE_REGISTRY.is_file() else {}
    clo_items = clo.get("items") or []
    lumen_anchor = " ".join(
        (it.get("anchor") or "") for it in clo_items if it.get("source_registry") == "g13_ue_lumen_gap_registry"
    )
    if "GI 多级反弹/表面缓存表达面立项" not in lumen_anchor:
        problems.append("承接锚面① GI 表达面承接锚缺失（lumen 2 行）")
    if "UE 项目侧 cornell 出图链诊断/修复" not in contract_text:
        problems.append("承接锚面① UE 参照臂修复承接锚缺失（G15-MC-F1）")
    if "310.5.2→310.6.0" not in contract_text or "D3D12" not in contract_text:
        problems.append("承接锚面② DLSS NGX 版本与宿主车道承接锚缺失（G15-MD-F1）")
    p2_text = P2_TABLE_PATH.read_text(encoding="utf-8") if P2_TABLE_PATH.is_file() else ""
    if "绝对画质 deficit 收口" not in p2_text or "G16+ 承接锚三面齐备" not in p2_text:
        problems.append("承接锚面③ 绝对画质 deficit 收口汇总字面缺失（G15-P2 §5）")
    open_defer_rows = [it for it in clo_items if it.get("final_disposition") == "open-defer-G16+"]
    if len(open_defer_rows) != 16:
        problems.append(f"open-defer-G16+ 行数={len(open_defer_rows)} ≠ 16（绝对画质 deficit 收口承接锚面漂移）")
    if problems:
        return False, "; ".join(problems)
    return True, (
        "双未达标终审定盘：商用收口 0/18（M-c commercial_closure 定盘 + 18 格逐格归因 + "
        "G15-MC-F1 参照死黑退化面 + g16_anchor 授权面字面）+ 性能 17/18 单格环境事件面"
        "（G14 M-d fail 17/1 + 帧率表 1 行 51a150cb4523e8b6 + G15-MD-F1 承接锚 §8.6/§8.7——"
        "UE 参照臂缓存暖态跨会话位移 + NGX 物理不可达终版）——如实登记不冒充；G16+ 承接锚"
        "三面齐备（① GI 表达面 + UE 参照臂修复 ② DLSS NGX 版本与宿主车道 ③ 绝对画质 deficit "
        "收口〔16 行 open-defer 锚字面 + P2 §5 汇总〕）；用户 2026-08-19 授权面逐字承接，"
        "性能零降级守护面终态锁定"
    )


def run_closeout() -> int:
    if NUMERIC_STEP <= 0:
        print("[6b] NUMERIC_STEP unset → BLOCKED", file=sys.stderr)
        return 1
    today = wel.utc_stamp()[:8]
    facts: list[dict] = []
    gate_rows = [verify_key_gate(k, p) for k, p in P0_KEYS]
    gates_ok = all(r["status"] == "PASS" for r in gate_rows)
    facts.append(_fact("five_p0_pass", gates_ok,
                       f"pass={sum(1 for r in gate_rows if r['status']=='PASS')}/5（M-d 诚实红特判面）"))

    wave_rows = [
        (eval_honest_red_wave5() if p == WAVE5_PREFIX else wel.require_gate_pass(k, p))
        for k, p in WAVE_GATES
    ]
    waves_ok = all(r["status"] == "PASS" for r in wave_rows)
    facts.append(_fact("wave_gates_2_to_6a", waves_ok,
                       f"pass={sum(1 for r in wave_rows if r['status']=='PASS')}/{len(WAVE_GATES)}（wave5 红面特判同型）"))

    # MAP 双向机核（G15.1 治理门面，host 只读快检；--gate 真跑产新鲜 evidence）
    map_r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "g15_acceptance_map_check.py"),
         "--gate", "g15.wave.1.acceptance_map"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    map_ok = map_r.returncode == 0
    facts.append(_fact("acceptance_map_check", map_ok, f"g15.wave.1.acceptance_map exit={map_r.returncode}"))

    # P2 40 行闭集最终状态无漂移
    p2 = wel.load_latest_evidence("g15_p2_decisions")
    p2_ok = False
    if p2:
        d = wel.load_json(p2)
        p2_ok = d.get("host_section_pass") is True
    p2_frozen_ok = P2_TABLE_PATH.is_file() and all(
        i in P2_TABLE_PATH.read_text(encoding="utf-8") for i in FROZEN_IDS
    ) and len(FROZEN_IDS) == 40
    facts.append(_fact(
        "p2_decisions_40_frozen",
        p2_ok and p2_frozen_ok,
        f"{str(p2.relative_to(ROOT)) if p2 else 'missing'}; frozen_40_in_tree={p2_frozen_ok}",
    ))

    # budget strict
    bud = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "budget_eval.py"), "--strict"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    bud_text = (bud.stdout or "") + (bud.stderr or "")
    bud_ok = bud.returncode == 0 and "[budget_eval] PASS" in bud_text and ", 0 skip" in bud_text
    facts.append(_fact("budget_strict", bud_ok, f"exit={bud.returncode}"))

    # 6a 先行
    e6a = wel.load_latest_evidence("g15_stabilization_soak")
    e6a_ok = False
    e6a_commit = None
    if e6a:
        d6 = wel.load_json(e6a)
        e6a_ok = d6.get("host_section_pass") is True
        e6a_commit = d6.get("base_commit")
    facts.append(_fact(
        "soak_6a_precedes",
        e6a_ok,
        f"{str(e6a.relative_to(ROOT)) if e6a else 'missing'}; base_commit_6a={e6a_commit}"
        "（同日放行沿 G14 立项裁决先例：6a full-run 先行完成后允许同日 close-out）",
    ))

    # RD 最终状态逐字一致（G-G15-9）
    rd_ok, rd_detail = check_rd_final_state()
    facts.append(_fact("rd_final_state_consistent", rd_ok, rd_detail))

    # 双未达标终审定盘 + G16+ 承接锚三面齐备 + 最后新绿 UTC 日留痕
    dual_ok, dual_detail = check_dual_unmet_finalized()
    last_green, missing = max_first_pass_date()
    facts.append(
        _fact(
            "dual_unmet_finalized",
            dual_ok and bool(last_green) and not missing,
            f"{dual_detail}; last_green_utc={last_green} today={today} missing={missing[:3]}",
        )
    )

    overall = all(f["status"] == "PASS" for f in facts)
    verdict = "READY" if overall else "BLOCKED"
    stamp = wel.utc_stamp()
    payload = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": WAVE,
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": overall,
        "device_section_state": "not_applicable",
        "verdict": verdict,
        "utc_date": today,
        "last_new_green_utc_date": last_green,
        "base_commit_6a": e6a_commit,
        "required_gates": gate_rows + wave_rows,
        "extra_facts": facts,
        "subjects": [],
        "checks": {
            "five_p0_pass": gates_ok,
            "wave_gates_pass": waves_ok,
            "acceptance_map_ok": map_ok,
            "p2_ok": p2_ok and p2_frozen_ok,
            "budget_strict_ok": bud_ok,
            "soak_6a_ok": e6a_ok,
            "rd_final_state_ok": rd_ok,
            "dual_unmet_finalized": dual_ok and bool(last_green) and not missing,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": (
            "same-day closeout allowed after 6a full-run（同日放行沿 G14 立项裁决先例，"
            "G9.8b~G14.5b 先例链）；双未达标终审定盘（商用收口 0/18 画质面 + 性能 17/18 "
            "单格环境事件面——如实登记不冒充，未达标按用户 2026-08-19 授权新建 G16+ 里程碑"
            "继续优化，性能零降级守护面终态锁定）；G16+ 承接锚三面齐备（GI 表达面 + UE 参照臂"
            "修复 / DLSS NGX 版本与宿主车道 / 绝对画质 deficit 收口）；M-d/wave5 诚实红门面"
            "特判（G14 closeout 先例同型字面：checks/结构面合格 + status=fail + unmet==登记表"
            "行数 + 承接锚在案 = 红不充绿亦不充降级）；RD-045 零检出维持 open 不关闭字面；"
            "status flip is a separate commit after READY"
        ),
    }
    if SCHEMA_PATH.is_file():
        errs = wel.validate_schema(payload, SCHEMA_PATH)
        if errs:
            print(f"[6b] schema: {errs}", file=sys.stderr)
            overall = False
            payload["host_section_pass"] = False
            payload["verdict"] = "BLOCKED"
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    out.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    for f in facts:
        print(f"  FACT  {f['status']:4}  {f['id']}  ({f['detail']})")
    print(f"  → evidence {out.relative_to(ROOT)}")
    print(f"  VERDICT = {payload['verdict']}")
    return 0 if overall else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        if NUMERIC_STEP <= 0:
            code = run_closeout()
            if code == 0:
                print("[selftest] FAIL: draft green", file=sys.stderr)
                return 1
            print("[selftest] PASS: draft → BLOCKED")
            return 0
        print("[selftest] OK materialized step", NUMERIC_STEP)
        return 0
    return run_closeout()


if __name__ == "__main__":
    sys.exit(main())
