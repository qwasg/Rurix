#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.5b 收口波）
"""G13.5b close-out 终审 g13.wave.5b.closeout（G13_CONTRACT G-G13-9；G13_PLAN §2
G13.5b；CI_GATES §5 wave5b 行；同构 ci/g12_closeout_check.py〔G12.7b〕先例）。

只读汇总八 facts：①5 P0 key（M-a~M-e）逐门 PASS（wel 口径 + 顶层 status=="pass"
字面——G13 证据形态统一无豁免面）+ ②wave2/3/4 exit + wave5 decisions + wave5a
soak 五聚合/决策门全 PASS + ③g13_acceptance_map_check 双向 exit=0 +
④P2 决策表 31 行闭集最终状态无漂移（最新 evidence host_section_pass +
FROZEN_IDS 31 行闭集在树）+ ⑤budget --strict 非空零 estimated/skip +
⑥5a full-run 先行（base_commit_5a 留痕；立项裁决 7 同日放行：5a full-run 先行
完成后允许同日 close-out）+ ⑦RD 最终状态逐字一致（deferred.json
RD-034/039/040/041/042/043/044 七条目级 status 全 open 逐字 + G13_P2_DECISIONS
31 行 FROZEN_IDS 闭集在树——G13 无 defer 重评窗表，G13.1 候选决策表 = 法定输入
直消费，两面一致，全表深对账由 wave5 decisions 门承载不重复）+ **⑧超分/Lumen
差距登记表双表终审锁定**（g13_ue_upscale_gap_registry.json 8 行闭集 +
g13_ue_lumen_gap_registry.json 2 行闭集：gap_id 集 == 本门冻结清单逐字对账 +
计数重算一致〔upscale total 8/cornell-box 5/bistro-interior 3，lumen total 2/
cornell-box 1/bistro-interior 1，not_ready_scenes==[]〕+ generated_by ==
M-c/M-d 门字面 + 全行 kind=quality_gap + suggested_priority=P2 + measured_delta
非空 + delta==b−a 可溯源 + evidence_digest 溯源 receipt + g11_anchor 锚定 G15
字面——残余差距/未闭环行如实登记不冒充全闭环，G-G13-9 字面；**终审锁定面 =
G14/G15 法定输入**）+ 最后新绿 UTC 日留痕。

输出 VERDICT = READY|BLOCKED。status flip 可与 READY 同波独立 commit。

用法：
  py -3 ci/g13_closeout_check.py --gate g13.wave.5b.closeout
  py -3 ci/g13_closeout_check.py --selftest
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
from g13_p2_decisions_check import FROZEN_IDS  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g13.wave.5b.closeout"
NUMERIC_STEP = 246  # 落盘前实测 registry/number_ledger.json CI_step.next_free=246 顺位领取
SUBJECT = "g13_wave5b_closeout"
WAVE = "G13.5b"
SOURCE_REF = (
    "G13_CONTRACT G-G13-9;G13_PLAN §2 G13.5b;CI_GATES §5 wave5b;"
    "5 P0 + wave2/3/4/5/5a 聚合/决策 + MAP 三向 + P2 31 行闭集 + budget --strict + 5a 先行"
    "（同日放行立项裁决 7）+ RD 最终状态逐字一致 + 超分/Lumen 双差距登记表终审锁定"
    "（8+2 行终态→G14/G15 法定输入）"
)
SCHEMA_PATH = ROOT / "milestones" / "g13" / "g13_wave5b_closeout_evidence_schema.json"
P2_TABLE_PATH = ROOT / "milestones" / "g13" / "G13_P2_DECISIONS.md"
UPSCALE_REGISTRY_PATH = ROOT / "milestones" / "g13" / "g13_ue_upscale_gap_registry.json"
LUMEN_REGISTRY_PATH = ROOT / "milestones" / "g13" / "g13_ue_lumen_gap_registry.json"

# 5 P0（G13_ACCEPTANCE_MAP §1 实记；key/prefix 与
# ci/g13_stabilization_soak.py P0_GATES 同一闭集）。
P0_KEYS = [
    ("g13.p0.m_a.vendor_upscale_integration", "g13_m_a_vendor_upscale_integration"),
    ("g13.p0.m_b.tsr_device_kernel", "g13_m_b_tsr_device_kernel"),
    ("g13.p0.m_c.ue_upscale_parity", "g13_m_c_ue_upscale_parity"),
    ("g13.p0.m_d.ue_lumen_gi_parity", "g13_m_d_ue_lumen_gi_parity"),
    ("g13.p0.m_e.regression_drift_guard", "g13_m_e_regression_drift_guard"),
]

WAVE_GATES = [
    ("g13.wave.2.exit", "g13_wave2_exit"),
    ("g13.wave.3.exit", "g13_wave3_exit"),
    ("g13.wave.4.exit", "g13_wave4_exit"),
    ("g13.wave.5.decisions", "g13_p2_decisions"),
    ("g13.wave.5a.soak", "g13_stabilization_soak"),
]

# G13_CONTRACT §6 Deferred 处置表字面：七条目总体 status 全维持 open
# （分项 go/defer 由候选决策表、G13_P2_DECISIONS 与 deferred history 只追加留痕，
# 条目级 0-byte）。
RD_FINAL_OPEN_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044"]

# G13.4 门产差距登记表终审锁定清单（G13.5b 终审锁定面 = G14/G15 法定输入；
# 多一行/少一行/换一行即漂移；gap_id = 身份五节派生，与测量值再锚定无关）。
FROZEN_UPSCALE_GAP_IDS = frozenset(
    {
        "fda2892b148edc2f",  # upscale_deficit_delta@cornell-box/t50/dlss_sr
        "20f125548f145335",  # upscale_deficit_delta@cornell-box/t67/dlss_sr
        "58fd4c7e2ef98efe",  # noise_hf_delta@cornell-box/t67/tsr_device
        "2631811751d63e0a",  # noise_hf_delta@cornell-box/t67/dlss_sr
        "d36e8cb107d579d9",  # noise_hf_delta@cornell-box/t67/fsr_3_1_5
        "5b65327b903ac6bc",  # noise_hf_delta@bistro-interior/t67/tsr_device
        "bdf94acf4691fd74",  # noise_hf_delta@bistro-interior/t67/dlss_sr
        "20e8950296211aae",  # noise_hf_delta@bistro-interior/t67/fsr_3_1_5
    }
)
FROZEN_LUMEN_GAP_IDS = frozenset(
    {
        "2f6331a41404dfcd",  # lumen_gi_parity@cornell-box
        "b7527c980cdd1d46",  # lumen_gi_parity@bistro-interior
    }
)
UPSCALE_GENERATED_BY = "ci/g13_ue_upscale_parity_smoke.py --gate g13.p0.m_c.ue_upscale_parity"
LUMEN_GENERATED_BY = "ci/g13_ue_lumen_gi_parity_smoke.py --gate g13.p0.m_d.ue_lumen_gi_parity"
_UTC_STAMP_RE = re.compile(r"_(\d{8}T\d{6}Z)\.json$")


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def verify_key_gate(key: str, prefix: str) -> dict:
    """5 P0 最新 evidence 终审核验：wel 口径 + 顶层 status=="pass" 字面
    （G13 证据形态统一，无豁免面）。"""
    row = wel.require_gate_pass(key, prefix)
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return row
    try:
        doc = wel.load_json(path)
    except (OSError, ValueError):
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
    """对 5 P0 取最新 PASS evidence 的 UTC 日期的 max（近似『最后新绿』）。"""
    dates: list[str] = []
    missing: list[str] = []
    for key, prefix in P0_KEYS:
        p = wel.load_latest_evidence(prefix)
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
    """G-G13-9「验收映射、候选决策、RD 最终状态逐字一致」机器化面：

    - deferred.json 七条 RD 条目级 status 逐字 == "open"（G13_CONTRACT §6
      处置表字面；分项历史只追加，条目级 0-byte）；
    - G13_P2_DECISIONS.md 在树且 31 行 FROZEN_IDS 闭集逐 ID 在文——
      两面（deferred.json/P2 决策表）终态一致轻量核验；G13 无 defer 重评窗表
      （G13.1 候选决策表 = 法定输入直消费，无独立重评窗波次），全表深对账由
      g13.wave.5.decisions 门承载，本门不重复对账。
    """
    problems: list[str] = []
    for rd in RD_FINAL_OPEN_IDS:
        st = wel.load_rd_status(rd)
        if st != "open":
            problems.append(f"{rd} status={st!r} ≠ 'open'")
    if not P2_TABLE_PATH.is_file():
        problems.append("G13_P2_DECISIONS.md 缺失")
    else:
        text = P2_TABLE_PATH.read_text(encoding="utf-8")
        absent = [i for i in FROZEN_IDS if i not in text]
        if absent:
            problems.append(f"P2 表缺 FROZEN_IDS: {absent}")
        if len(FROZEN_IDS) != 31:
            problems.append(f"FROZEN_IDS n={len(FROZEN_IDS)} ≠ 31（闭集口径漂移）")
    return (not problems), "; ".join(problems) if problems else (
        "7 RD open 逐字一致 + P2 31 行闭集在树（G13 无重评窗表，两面一致）"
    )


def _check_one_registry(path: Path, name: str, frozen_ids: frozenset,
                        generated_by: str, scene_counts: dict[str, int]) -> list[str]:
    """单表终审锁定机器化面（gap_id 闭集 + 计数重算 + 行质对账 + generated_by 字面）。"""
    problems: list[str] = []
    if not path.is_file():
        return [f"{name} 缺失"]
    doc = wel.load_json(path)
    items = doc.get("items") or []
    ids = {it.get("gap_id") for it in items}
    if len(items) != len(frozen_ids) or ids != frozen_ids:
        problems.append(
            f"{name} gap_id 闭集漂移: n={len(items)} extra={sorted(ids - frozen_ids)} "
            f"missing={sorted(frozen_ids - ids)}"
        )
    if doc.get("registry") != name:
        problems.append(f"{name} registry 名字面漂移: {doc.get('registry')!r}")
    if doc.get("generated_by") != generated_by:
        problems.append(f"{name} generated_by 漂移: {doc.get('generated_by')!r}")
    for it in items:
        if it.get("kind") != "quality_gap":
            problems.append(f"{name} 行 kind 漂移: {it.get('gap_id')} {it.get('kind')!r}")
        if it.get("suggested_priority") != "P2":
            problems.append(f"{name} 行 suggested_priority 漂移: {it.get('gap_id')}")
        ds = it.get("measured_delta") or []
        if not ds:
            problems.append(f"{name} 行 measured_delta 空: {it.get('gap_id')}")
        for d in ds:
            if abs(d.get("delta", 0.0) - (d.get("b_value", 0.0) - d.get("a_value", 0.0))) > 1e-18:
                problems.append(f"{name} 行 delta ≠ b−a 不可溯源: {it.get('gap_id')}")
            if not str(d.get("evidence_digest") or "").startswith("sha256:"):
                problems.append(f"{name} 行 evidence_digest 溯源缺失: {it.get('gap_id')}")
        if "G15" not in str(it.get("g11_anchor") or ""):
            problems.append(f"{name} 行 g11_anchor 未锚定 G15: {it.get('gap_id')}")
    summary = doc.get("scene_summary") or []
    summary_map = {s.get("scene_id"): s for s in summary} if isinstance(summary, list) else summary
    for scene, want in scene_counts.items():
        got = (summary_map.get(scene) or {}).get("gap_count")
        recount = sum(1 for i in items if i.get("scene_id") == scene)
        if got != want or got != recount:
            problems.append(f"{name} scene_summary.{scene}.gap_count={got} ≠ {want}/重算 {recount}")
        if (summary_map.get(scene) or {}).get("no_gap_explicit") is not False:
            problems.append(f"{name} scene_summary.{scene}.no_gap_explicit 非 false")
    if doc.get("not_ready_scenes") != []:
        problems.append(f"{name} not_ready_scenes 漂移: {doc.get('not_ready_scenes')}")
    return problems


def check_gap_registries_lock() -> tuple[bool, str]:
    """G-G13-9「Lumen/超分差距清单终审锁定（残余差距/未闭环行如实登记不冒充全闭环）」
    机器化面（终审锁定面 = G14/G15 法定输入）：双表 8+2 行闭集逐字对账。"""
    problems: list[str] = []
    problems += _check_one_registry(
        UPSCALE_REGISTRY_PATH, "g13_ue_upscale_gap_registry", FROZEN_UPSCALE_GAP_IDS,
        UPSCALE_GENERATED_BY, {"cornell-box": 5, "bistro-interior": 3},
    )
    problems += _check_one_registry(
        LUMEN_REGISTRY_PATH, "g13_ue_lumen_gap_registry", FROZEN_LUMEN_GAP_IDS,
        LUMEN_GENERATED_BY, {"cornell-box": 1, "bistro-interior": 1},
    )
    return (not problems), "; ".join(problems) if problems else (
        "超分/Lumen 双差距登记表 8+2 行闭集终审锁定（gap_id 集逐字对账 + 计数 8/5/3 "
        "与 2/1/1 重算一致 + 全行 quality_gap/P2 + measured_delta 对账齐 + 锚定 G15 "
        "——残余差距如实登记不冒充全闭环；终审锁定面 = G14/G15 法定输入）"
    )


def run_closeout() -> int:
    if NUMERIC_STEP <= 0:
        print("[5b] NUMERIC_STEP unset → BLOCKED", file=sys.stderr)
        return 1
    today = wel.utc_stamp()[:8]
    facts: list[dict] = []
    gate_rows = [verify_key_gate(k, p) for k, p in P0_KEYS]
    gates_ok = all(r["status"] == "PASS" for r in gate_rows)
    facts.append(_fact("five_p0_pass", gates_ok, f"pass={sum(1 for r in gate_rows if r['status']=='PASS')}/5"))

    wave_rows = [wel.require_gate_pass(k, p) for k, p in WAVE_GATES]
    waves_ok = all(r["status"] == "PASS" for r in wave_rows)
    facts.append(_fact("wave_gates_2_to_5a", waves_ok, f"pass={sum(1 for r in wave_rows if r['status']=='PASS')}/{len(WAVE_GATES)}"))

    # MAP 双向机核（G13.1 治理门面，host 只读快检；--gate 真跑产新鲜 evidence）
    map_r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "g13_acceptance_map_check.py"),
         "--gate", "g13.wave.1.acceptance_map"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    map_ok = map_r.returncode == 0
    facts.append(_fact("acceptance_map_check", map_ok, f"g13.wave.1.acceptance_map exit={map_r.returncode}"))

    # P2 31 行闭集最终状态无漂移
    p2 = wel.load_latest_evidence("g13_p2_decisions")
    p2_ok = False
    if p2:
        d = wel.load_json(p2)
        p2_ok = d.get("host_section_pass") is True
    p2_frozen_ok = P2_TABLE_PATH.is_file() and all(
        i in P2_TABLE_PATH.read_text(encoding="utf-8") for i in FROZEN_IDS
    ) and len(FROZEN_IDS) == 31
    facts.append(_fact(
        "p2_decisions_31_frozen",
        p2_ok and p2_frozen_ok,
        f"{str(p2.relative_to(ROOT)) if p2 else 'missing'}; frozen_31_in_tree={p2_frozen_ok}",
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
    e5a = wel.load_latest_evidence("g13_stabilization_soak")
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
        "（同日放行立项裁决 7：5a full-run 先行完成后允许同日 close-out）",
    ))

    # RD 最终状态逐字一致（G-G13-9）
    rd_ok, rd_detail = check_rd_final_state()
    facts.append(_fact("rd_final_state_consistent", rd_ok, rd_detail))

    # 超分/Lumen 双差距登记表终审锁定 + 最后新绿 UTC 日留痕
    gap_ok, gap_detail = check_gap_registries_lock()
    last_green, missing = max_first_pass_date()
    facts.append(
        _fact(
            "gap_registries_locked_and_green_recorded",
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
            "gap_registries_locked": gap_ok and bool(last_green) and not missing,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": (
            "same-day closeout allowed after 5a full-run（立项裁决 7，沿 G12.7b/G11.7b/"
            "G10.8b/G9.8b 先例链）；超分/Lumen 双差距登记表终审锁定"
            "（g13_ue_upscale_gap_registry.json 8 行 + g13_ue_lumen_gap_registry.json 2 行终态"
            "——全 quality_gap/P2，残余差距/未闭环行如实登记不冒充全闭环，G-G13-9 字面；"
            "终审锁定面 = G14/G15 法定输入——G14 帧率对标期与 G15 画质收口期只消费本双表与 "
            "G13_P2_DECISIONS 承接锚，不得另起无锚差距面）；status flip is a separate "
            "commit after READY"
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
