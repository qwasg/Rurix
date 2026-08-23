#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.5b 收口波）
"""G14.5b close-out 终审 g14.wave.5b.closeout（G14_CONTRACT G-G14-9；同构
ci/g13_closeout_check.py〔G13.5b〕先例）。

只读汇总八 facts：①5 P0 key（M-a~M-e）逐门 PASS（wel 口径 + 顶层 status=="pass"
字面）——**M-d 诚实红门面特判**：checks 全绿 ∧ status=="fail" ∧ unmet_count ==
g14_fps_gap_registry 行数 = 通过线未达标如实登记面（G-G14-6/G-G14-9 字面——
红不充绿亦不充降级，终审定盘如实登记不冒充）+ ②wave2/3/4/6/7 exit + wave5a
decisions + wave5a soak 七聚合/决策门全 PASS + ③g14_acceptance_map_check 双向
exit=0 + ④P2 决策表 42 行闭集最终状态无漂移（最新 evidence host_section_pass
+ FROZEN_IDS 42 行闭集在树）+ ⑤budget --strict 非空零 estimated/skip +
⑥5a full-run 先行（base_commit_5a 留痕；立项裁决 3 同日放行：5a full-run 先行
完成后允许同日 close-out）+ ⑦RD 最终状态逐字一致（deferred.json
RD-034/039/040/041/042/043/044 七条目级 status 全 open 逐字 + G14_P2_DECISIONS
42 行 FROZEN_IDS 闭集在树）+ **⑧帧率对标结果终审定盘 + g14 帧率差距登记表
终审锁定**（g14_fps_gap_registry.json 18 行闭集：gap_id 集 == 本门冻结清单逐字
对账 + 计数重算一致〔cornell-box 9/bistro-interior 9〕+ generated_by == M-d 门
字面 + 全行 kind=quality_gap + measured 面非空 + 最新 M-d evidence unmet_count
== 18 行一致 + **画质零降级守护面终态锁定**〔G13 锁定双门最新 evidence PASS
消费——M-d 门画质守护面 checks 真〕+ **通过线 ×1.00 未达标如实登记不冒充**
〔G14-N8 行承载，继续优化面 G16+ 承接——G-G14-9 字面；商用收口判定归 G15+〕）
+ 最后新绿 UTC 日留痕。

输出 VERDICT = READY|BLOCKED。status flip 可与 READY 同波独立 commit。

用法：
  py -3 ci/g14_closeout_check.py --gate g14.wave.5b.closeout
  py -3 ci/g14_closeout_check.py --selftest
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
from g14_p2_decisions_check import FROZEN_IDS  # noqa: E402
from g14_stabilization_soak import HONEST_RED_AGGREGATES, eval_honest_red_aggregate  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g14.wave.5b.closeout"
NUMERIC_STEP = 264  # 落盘前实测 registry/number_ledger.json CI_step.next_free=264 顺位领取
SUBJECT = "g14_wave5b_closeout"
WAVE = "G14.5b"
SOURCE_REF = (
    "G14_CONTRACT G-G14-9;G14_PLAN §2 G14.5b;"
    "5 P0（M-d 诚实红特判）+ wave2/3/4/6/7/5a 聚合/决策 + MAP 双向 + P2 42 行闭集 + budget --strict + 5a 先行"
    "（同日放行立项裁决 3）+ RD 最终状态逐字一致 + 帧率对标终审定盘与 g14 帧率差距登记表终审锁定"
    "（18 行终态→G15+/G16+ 法定输入——未达标如实登记不冒充）"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_wave5b_closeout_evidence_schema.json"
P2_TABLE_PATH = ROOT / "milestones" / "g14" / "G14_P2_DECISIONS.md"
FPS_REGISTRY_PATH = ROOT / "milestones" / "g14" / "g14_fps_gap_registry.json"

# 5 P0（G14_ACCEPTANCE_MAP §1 实记；key/prefix 与
# ci/g14_stabilization_soak.py P0_GATES 同一闭集）。
P0_KEYS = [
    ("g14.p0.m_a.registry_variance_band_reconciliation", "g14_m_a_registry_variance_band_reconciliation"),
    ("g14.p0.m_b.ue_benchmark_arm_measurement", "g14_m_b_ue_benchmark_arm_measurement"),
    ("g14.p0.m_c.rurix_pipeline_perf", "g14_m_c_rurix_pipeline_perf"),
    ("g14.p0.m_d.dual_end_fps_parity", "g14_m_d_dual_end_fps_parity"),
    ("g14.p0.m_e.regression_drift_guard", "g14_m_e_regression_drift_guard"),
]

WAVE_GATES = [
    ("g14.wave.2.exit", "g14_wave2_exit"),
    ("g14.wave.3.exit", "g14_wave3_exit"),
    ("g14.wave.4.exit", "g14_wave4_exit"),
    ("g14.wave.6.exit", "g14_wave6_exit"),
    ("g14.wave.7.exit", "g14_wave7_exit"),
    ("g14.wave.5a.decisions", "g14_p2_decisions"),
    ("g14.wave.5a.soak", "g14_stabilization_soak"),
]

# G14_CONTRACT §6 Deferred 处置表字面：七条目总体 status 全维持 open
# （分项 go/defer 由候选决策表、G14_P2_DECISIONS 与 deferred history 只追加留痕，
# 条目级 0-byte）+ RD-045（G14.5a 后事件升级登记——M165 同型间歇漂移修复项，open）。
RD_FINAL_OPEN_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044", "RD-045"]

# G14.4 门产帧率差距登记表终审锁定清单（G14.5b 终审锁定面 = G15+/G16+ 法定输入；
# 多一行/少一行/换一行即漂移；gap_id = 身份面派生，与测量值再锚定无关）。
FROZEN_FPS_GAP_IDS = frozenset(
    {
        "871cfe1abd90715e", "22a838e2e33d6043", "b94ae169f856c115",
        "967b7ecdf14f226c", "61bb90a0c954861b", "1fb5468095485fcf",
        "d01607e698c1e017", "800d53792a920393", "cd685fe61d66a881",
        "f491f4444796f105", "4737182a0741d03a", "a9201d54384e20ae",
        "135c105f3d08505e", "f74fc1cba6306b0b", "ff5bdbea51a590e9",
        "2ee7a38ddd037c74", "51a150cb4523e8b6", "1d0205d958e01739",
    }
)
FPS_GENERATED_BY = "ci/g14_dual_end_fps_parity_smoke.py --gate g14.p0.m_d.dual_end_fps_parity"
MD_PREFIX = "g14_m_d_dual_end_fps_parity"
G13_QUALITY_GATES = [
    ("g13.p0.m_c.ue_upscale_parity", "g13_m_c_ue_upscale_parity"),
    ("g13.p0.m_d.ue_lumen_gi_parity", "g13_m_d_ue_lumen_gi_parity"),
]
_UTC_STAMP_RE = re.compile(r"_(\d{8}T\d{6}Z)\.json$")


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def verify_key_gate(key: str, prefix: str) -> dict:
    """5 P0 最新 evidence 终审核验：wel 口径 + 顶层 status=="pass" 字面。

    M-d 诚实红门面特判（G-G14-6/G-G14-9 字面）：status=="fail" 且 checks 全绿
    且 unmet_count == g14_fps_gap_registry 行数 = 通过线未达标如实登记面——
    红不充绿亦不充降级（终审定盘如实登记不冒充）。"""
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
    if prefix == MD_PREFIX:
        checks = doc.get("checks") or {}
        bad = [k for k, v in checks.items() if v is not True]
        status = doc.get("status")
        unmet = (doc.get("parity") or {}).get("unmet_count")
        reg_rows = -1
        if FPS_REGISTRY_PATH.is_file():
            reg_rows = len((wel.load_json(FPS_REGISTRY_PATH)).get("items") or [])
        if status == "pass":
            return row  # 达标面（未来延续波可能翻转）——wel 口径已绿
        if status == "fail" and not bad and unmet is not None and unmet == reg_rows:
            # 诚实红登记面——终审核验合格，行置 PASS 并在 detail 承载诚实红字面
            #（不充绿：M-d 门自身 evidence status=fail 0-byte 维持，终审如实登记不冒充）
            row["status"] = "PASS"
            row["detail"] = (f"{row.get('detail','')}; M-d 诚实红门面特判合格"
                             f"（checks 全绿 + unmet={unmet} == 登记表 {reg_rows} 行——"
                             f"未达标如实登记不冒充）")
            return row
        row["status"] = "FAIL"
        row["detail"] = (f"M-d 面异常: status={status!r} checks_bad={bad[:3]} "
                         f"unmet={unmet} reg={reg_rows}（诚实红登记面不一致即 RED）")
        return row
    problems: list[str] = []
    if doc.get("status") != "pass":
        problems.append(f"status={doc.get('status')!r} ≠ 'pass'")
    if problems:
        row["status"] = "FAIL"
        row["detail"] = f"{row.get('detail', '')}; " + "; ".join(problems)
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

    M-d 双态：诚实红期无 PASS 态跳过（终审定盘归 fact⑧）；G14plus 18/18 达标
    后（status=="pass"）纳入日期面（达标态即新绿事件）。"""
    dates: list[str] = []
    missing: list[str] = []
    for key, prefix in P0_KEYS:
        p = wel.load_latest_evidence(prefix)
        if prefix == MD_PREFIX:
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
    """G-G14-9「验收映射、候选决策、RD 最终状态逐字一致」机器化面。"""
    problems: list[str] = []
    for rd in RD_FINAL_OPEN_IDS:
        st = wel.load_rd_status(rd)
        if st != "open":
            problems.append(f"{rd} status={st!r} ≠ 'open'")
    if not P2_TABLE_PATH.is_file():
        problems.append("G14_P2_DECISIONS.md 缺失")
    else:
        text = P2_TABLE_PATH.read_text(encoding="utf-8")
        absent = [i for i in FROZEN_IDS if i not in text]
        if absent:
            problems.append(f"P2 表缺 FROZEN_IDS: {absent}")
        if len(FROZEN_IDS) != 42:
            problems.append(f"FROZEN_IDS n={len(FROZEN_IDS)} ≠ 42（闭集口径漂移）")
    return (not problems), "; ".join(problems) if problems else (
        "8 RD open 逐字一致（含 RD-045 升级登记）+ P2 42 行闭集在树（全表深对账由 wave5a decisions 门承载）"
    )


def check_fps_registry_lock() -> tuple[bool, str]:
    """G-G14-9「帧率对标结果终审定盘 + 画质零降级守护面终态锁定」机器化面
    （终审锁定面 = G15+/G16+ 法定输入）。

    双分支（互斥,均要求 registry ↔ 最新 M-d evidence 镜像一致 + G13 双门绿）：
    - **达标分支**（G14plus 18/18,RFC-0030/G14.12）：M-d status=="pass" +
      unmet_count==0 + registry 空表显式登记（items n=0 + 双场景
      no_gap_explicit=true）。
    - **诚实红分支**（G14.5b 原字面）：M-d status=="fail" + unmet==18 +
      registry 18 行闭集逐字对账（FROZEN_FPS_GAP_IDS）。"""
    problems: list[str] = []
    md_path = wel.load_latest_evidence(MD_PREFIX)
    md_doc: dict = {}
    md_status = None
    unmet = None
    if md_path is None:
        problems.append("缺最新 M-d evidence")
    else:
        md_doc = wel.load_json(md_path)
        md_status = md_doc.get("status")
        unmet = (md_doc.get("parity") or {}).get("unmet_count")
        md_checks = md_doc.get("checks") or {}
        qbad = [k for k, v in md_checks.items() if "quality" in k and v is not True]
        if qbad:
            problems.append(f"M-d 画质零降级守护面 checks 非真: {qbad}")
    met_branch = md_status == "pass" and unmet == 0
    if not FPS_REGISTRY_PATH.is_file():
        problems.append("g14_fps_gap_registry 缺失")
    else:
        doc = wel.load_json(FPS_REGISTRY_PATH)
        items = doc.get("items") or []
        ids = {it.get("gap_id") for it in items}
        if met_branch:
            # 达标分支：空表显式登记 + 双场景 no_gap_explicit 全真。
            if items:
                problems.append(f"达标态 registry 非空: n={len(items)}（空表显式登记面漂移）")
            summ = {s.get("scene_id"): s for s in (doc.get("scene_summary") or [])}
            for scene in ("cornell-box", "bistro-interior"):
                if not (summ.get(scene) or {}).get("no_gap_explicit"):
                    problems.append(f"达标态 {scene} no_gap_explicit 非真")
        else:
            if len(items) != 18 or ids != FROZEN_FPS_GAP_IDS:
                problems.append(
                    f"g14 帧率表 gap_id 闭集漂移: n={len(items)} extra={sorted(ids - FROZEN_FPS_GAP_IDS)} "
                    f"missing={sorted(FROZEN_FPS_GAP_IDS - ids)}"
                )
            for it in items:
                if it.get("kind") != "quality_gap":
                    problems.append(f"行 kind 漂移: {it.get('gap_id')} {it.get('kind')!r}")
                ds = it.get("measured_delta") or []
                if not ds and not it.get("measured"):
                    problems.append(f"行 measured 面空: {it.get('gap_id')}")
            for scene, want in (("cornell-box", 9), ("bistro-interior", 9)):
                recount = sum(1 for i in items if i.get("scene_id") == scene)
                if recount != want:
                    problems.append(f"scene 计数重算 {scene}: {recount} ≠ {want}")
        if doc.get("registry") != "g14_fps_gap_registry":
            problems.append(f"registry 名字面漂移: {doc.get('registry')!r}")
        if doc.get("generated_by") != FPS_GENERATED_BY:
            problems.append(f"generated_by 漂移: {doc.get('generated_by')!r}")
    if md_path is not None and not met_branch and unmet != 18:
        problems.append(
            f"最新 M-d unmet_count={unmet} status={md_status!r} 非达标(0/pass)亦非诚实红(18/fail)"
            "（终审定盘面漂移）"
        )
    for key, prefix in G13_QUALITY_GATES:
        row = wel.require_gate_pass(key, prefix)
        if row["status"] != "PASS":
            problems.append(f"G13 画质守护消费面非绿: {key}: {row['detail']}")
    if problems:
        return False, "; ".join(problems)
    if met_branch:
        return True, (
            "g14 帧率对标 18/18 达标终审锁定（最新 M-d status=pass + unmet==0 + registry "
            "空表显式登记 + 双场景 no_gap_explicit + G13 双门画质守护消费面绿）——"
            "G14plus 延续波达标定盘（RFC-0030），画质零降级守护面终态锁定"
        )
    return True, (
        "g14 帧率差距登记表 18 行闭集终审锁定（gap_id 集逐字对账 + 计数 9/9 重算一致 + "
        "generated_by 字面 + 最新 M-d unmet==18 + G13 双门画质守护消费面绿）——"
        "通过线 ×1.00 未达标如实登记不冒充（继续优化面 G16+ 承接），画质零降级守护面终态锁定"
    )


def run_closeout() -> int:
    if NUMERIC_STEP <= 0:
        print("[5b] NUMERIC_STEP unset → BLOCKED", file=sys.stderr)
        return 1
    today = wel.utc_stamp()[:8]
    facts: list[dict] = []
    gate_rows = [verify_key_gate(k, p) for k, p in P0_KEYS]
    gates_ok = all(r["status"] == "PASS" for r in gate_rows)
    facts.append(_fact("five_p0_pass", gates_ok,
                       f"pass={sum(1 for r in gate_rows if r['status']=='PASS')}/5（M-d 诚实红特判面）"))

    wave_rows = [
        (eval_honest_red_aggregate(k, p) if p in HONEST_RED_AGGREGATES else wel.require_gate_pass(k, p))
        for k, p in WAVE_GATES
    ]
    waves_ok = all(r["status"] == "PASS" for r in wave_rows)
    facts.append(_fact("wave_gates_2_to_5a", waves_ok,
                       f"pass={sum(1 for r in wave_rows if r['status']=='PASS')}/{len(WAVE_GATES)}"))

    # MAP 双向机核（G14.1 治理门面，host 只读快检；--gate 真跑产新鲜 evidence）
    map_r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "g14_acceptance_map_check.py"),
         "--gate", "g14.wave.1.acceptance_map"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    map_ok = map_r.returncode == 0
    facts.append(_fact("acceptance_map_check", map_ok, f"g14.wave.1.acceptance_map exit={map_r.returncode}"))

    # P2 42 行闭集最终状态无漂移
    p2 = wel.load_latest_evidence("g14_p2_decisions")
    p2_ok = False
    if p2:
        d = wel.load_json(p2)
        p2_ok = d.get("host_section_pass") is True
    p2_frozen_ok = P2_TABLE_PATH.is_file() and all(
        i in P2_TABLE_PATH.read_text(encoding="utf-8") for i in FROZEN_IDS
    ) and len(FROZEN_IDS) == 42
    facts.append(_fact(
        "p2_decisions_42_frozen",
        p2_ok and p2_frozen_ok,
        f"{str(p2.relative_to(ROOT)) if p2 else 'missing'}; frozen_42_in_tree={p2_frozen_ok}",
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

    # 5a 先行
    e5a = wel.load_latest_evidence("g14_stabilization_soak")
    e5a_ok = False
    e5a_commit = None
    if e5a:
        d5 = wel.load_json(e5a)
        e5a_ok = d5.get("host_section_pass") is True
        e5a_commit = d5.get("base_commit")
    facts.append(_fact(
        "soak_5a_precedes",
        e5a_ok,
        f"{str(e5a.relative_to(ROOT)) if e5a else 'missing'}; base_commit_5a={e5a_commit}"
        "（同日放行立项裁决 3：5a full-run 先行完成后允许同日 close-out）",
    ))

    # RD 最终状态逐字一致（G-G14-9）
    rd_ok, rd_detail = check_rd_final_state()
    facts.append(_fact("rd_final_state_consistent", rd_ok, rd_detail))

    # 帧率对标终审定盘 + g14 帧率差距登记表终审锁定 + 最后新绿 UTC 日留痕
    gap_ok, gap_detail = check_fps_registry_lock()
    last_green, missing = max_first_pass_date()
    facts.append(
        _fact(
            "fps_parity_finalized_and_registry_locked",
            gap_ok and bool(last_green) and not missing,
            f"{gap_detail}; last_green_utc={last_green} today={today} missing={missing[:3]}",
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
        "base_commit_5a": e5a_commit,
        "required_gates": gate_rows + wave_rows,
        "extra_facts": facts,
        "subjects": [],
        "checks": {
            "five_p0_pass": gates_ok,
            "wave_gates_pass": waves_ok,
            "acceptance_map_ok": map_ok,
            "p2_ok": p2_ok and p2_frozen_ok,
            "budget_strict_ok": bud_ok,
            "soak_5a_ok": e5a_ok,
            "rd_final_state_ok": rd_ok,
            "fps_parity_finalized_registry_locked": gap_ok and bool(last_green) and not missing,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": (
            "same-day closeout allowed after 5a full-run（立项裁决 3，沿 G9.8b/G10.8b/"
            "G11.7b/G12.7b/G13.5b 先例链）；g14 帧率对标终审定盘双分支"
            "（达标分支 = G14plus 18/18〔RFC-0030 延续波〕M-d pass + unmet==0 + registry "
            "空表显式登记；诚实红分支 = 18 行闭集如实登记不冒充〔G-G14-9 字面，"
            "继续优化面 G16+ 承接〕——fact⑧ detail 承载实测分支）；画质零降级守护面终态锁定"
            "〔G13 锁定双门最新 evidence PASS 消费〕；终审锁定面 = G15+/G16+ 法定输入——"
            "G15 画质收口期与 G16+ 结构性优化面只消费本表与 G14_P2_DECISIONS 承接锚，"
            "不得另起无锚差距面；M-d 门面特判双态（达标 status=pass 直通/诚实红 checks 全绿 + "
            "status=fail + unmet==登记表行数 = 红不充绿亦不充降级）；"
            "status flip is a separate commit after READY"
        ),
    }
    if SCHEMA_PATH.is_file():
        errs = wel.validate_schema(payload, SCHEMA_PATH)
        if errs:
            print(f"[5b] schema: {errs}", file=sys.stderr)
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
