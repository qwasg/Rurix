#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.6/G12.7 收口波）
"""G12.7b close-out 终审 g12.wave.7b.closeout(G12_CONTRACT G-G12-10;G12_PLAN §2
G12.7b;CI_GATES §5 wave7b 行;同构 ci/g11_closeout_check.py)。

只读汇总八 facts:①9 key(8 P0 + 1 go P1)逐门 PASS(wel 口径 + 顶层
status=="pass" 字面——G12 证据形态统一无豁免面)+ ②wave2~7a 六聚合/决策门
(exit×4 + decisions + soak)全 PASS + ③check_g12_acceptance_map 三向 exit=0 +
④P2 决策表 33 行闭集最终状态无漂移(最新 evidence host_section_pass +
FROZEN_IDS 33 行闭集在树)+ ⑤budget --strict 非空零 estimated/skip +
⑥7a full-run 先行(base_commit_7a 留痕;立项裁决 7 同日放行:7a full-run 先行
完成后允许同日 close-out)+ ⑦RD 最终状态逐字一致(deferred.json
RD-034/039~044 七条目级 status 全 open 逐字 + G12_P2_DECISIONS 33 行
FROZEN_IDS 闭集在树——G12 无 defer 重评窗表,G12.1 候选决策表 = 法定输入直
消费,两面一致,全表深对账由 wave6 门承载不重复)+ **⑧生产化差距清单终审
锁定**(g12_ue_pt_gap_registry.json 10 行闭集:gap_id 集 == G12.4 锁定清单
10 id 逐字对账 + 计数重算一致〔total 10/quality_gap 6/caliber_diff 4,
scene_summary cornell-box 4/bistro-interior 6,not_ready_scenes==[]〕+
generated_by == M163 门字面 + quality_gap 6 行超容差项对账齐〔measured_delta
非空 + g11_anchor 锚定 G15 字面〕+ caliber_diff 4 行残余口径归属非空——
残余差距/未闭环行如实登记不冒充全闭环,G-G12-10 字面;**终审锁定面 = G13
法定输入**)+ 最后新绿 UTC 日留痕。

输出 VERDICT = READY|BLOCKED。status flip 可与 READY 同波独立 commit。

用法:
  py -3 ci/g12_closeout_check.py --gate g12.wave.7b.closeout
  py -3 ci/g12_closeout_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g12_wave_exit_lib as wel  # noqa: E402
from g12_p2_decisions_check import FROZEN_IDS  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g12.wave.7b.closeout"
NUMERIC_STEP = 232  # 落盘前实测 registry/number_ledger.json CI_step.next_free=232 顺位领取
SUBJECT = "g12_wave7b_closeout"
WAVE = "G12.7b"
SOURCE_REF = (
    "G12_CONTRACT G-G12-10;G12_PLAN §2 G12.7b;CI_GATES §5 wave7b;"
    "9 key + wave2~7a 聚合/决策 + MAP 三向 + P2 33 行闭集 + budget --strict + 7a 先行"
    "(同日放行立项裁决 7)+ RD 最终状态逐字一致 + 生产化差距清单终审锁定(10 行终态→G13 法定输入)"
)
SCHEMA_PATH = ROOT / "milestones" / "g12" / "g12_wave7b_closeout_evidence_schema.json"
P2_TABLE_PATH = ROOT / "milestones" / "g12" / "G12_P2_DECISIONS.md"
GAP_REGISTRY_PATH = ROOT / "milestones" / "g12" / "g12_ue_pt_gap_registry.json"

# 8 P0 + 1 go P1(G12_ACCEPTANCE_MAP §1/§2 实记;key/prefix 与
# ci/g12_stabilization_soak.py REGRESSION_GATES 前 9 行同一闭集)。
P0_P1_KEYS = [
    ("g12.p1.m166.pt_production_calibration", "g12_pt_production_calibration"),
    ("g12.p0.m158.mis_full_surface", "g12_m158_mis_full_surface"),
    ("g12.p0.m159.russian_roulette_prod", "g12_m159_russian_roulette_prod"),
    ("g12.p0.m160.sampling_lds_upgrade", "g12_m160_sampling_lds_upgrade"),
    ("g12.p0.m161.convergence_criterion_prod", "g12_m161_convergence_criterion_prod"),
    ("g12.p0.m162.denoise_pipeline_tsr", "g12_m162_denoise_pipeline_tsr"),
    ("g12.p0.m163.ue_pt_parity", "g12_m163_ue_pt_parity"),
    ("g12.p0.m164.regression_guard", "g12_m164_regression_guard"),
    ("g12.p0.m165.pt_throughput_baseline", "g12_m165_pt_throughput_baseline"),
]

WAVE_EXITS = [
    ("g12.wave.2.exit", "g12_wave2_exit"),
    ("g12.wave.3.exit", "g12_wave3_exit"),
    ("g12.wave.4.exit", "g12_wave4_exit"),
    ("g12.wave.5.exit", "g12_wave5_exit"),
    ("g12.wave.6.decisions", "g12_p2_decisions"),
    ("g12.wave.7a.soak", "g12_stabilization_soak"),
]

# G12_CONTRACT §6 Deferred 处置表字面:七条目总体 status 全维持 open
# (分项 go/defer 由候选决策表、G12_P2_DECISIONS 与 deferred history 只追加留痕,
# 条目级 0-byte)。
RD_FINAL_OPEN_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044"]

# G12.4 UE PT 对标差距登记表锁定清单 10 id(G12.7b 终审锁定面 = G13 法定输入;
# 多一行/少一行/换一行即漂移)。
FROZEN_GAP_IDS = frozenset(
    {
        "8bb75b6657d6b10c",  # noise_spectrum@cornell-box(quality_gap)
        "525dcf5fe42a5a37",  # energy_conservation@cornell-box(quality_gap)
        "e6796378ebae6108",  # curve_segment_spp1@bistro-interior(quality_gap)
        "000f1899da9f087d",  # curve_segment_spp4@bistro-interior(quality_gap)
        "7ea11b30c1bc7f18",  # noise_spectrum@bistro-interior(quality_gap)
        "3fd88ba1c1b25684",  # energy_conservation@bistro-interior(quality_gap)
        "1cd456377445d16c",  # bistro_material_texture_mean_vs_per_texel(caliber_diff)
        "499b752ce1d25f1d",  # emissive_le_mean_vs_textured_emissive(caliber_diff)
        "21a61c9da0f3122e",  # aa_filter_policy_residual(caliber_diff)
        "802a48548e2e64fe",  # exr_bit_depth_fp16_vs_f32(caliber_diff)
    }
)
GAP_GENERATED_BY = "ci/g12_ue_pt_parity_smoke.py（g12.p0.m163.ue_pt_parity,步骤 225）"
_UTC_STAMP_RE = re.compile(r"_(\d{8}T\d{6}Z)\.json$")


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def verify_key_gate(key: str, prefix: str) -> dict:
    """9 key 最新 evidence 终审核验:wel 口径 + 顶层 status=="pass" 字面
    (G12 证据形态统一,无豁免面)。"""
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
    """对 9 key 取最新 PASS evidence 的 UTC 日期的 max(近似『最后新绿』)。"""
    dates: list[str] = []
    missing: list[str] = []
    for key, prefix in P0_P1_KEYS:
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
    """G-G12-10「验收映射、候选决策、RD 最终状态逐字一致」机器化面:

    - deferred.json 七条 RD 条目级 status 逐字 == "open"(G12_CONTRACT §6
      处置表字面;分项历史只追加,条目级 0-byte);
    - G12_P2_DECISIONS.md 在树且 33 行 FROZEN_IDS 闭集逐 ID 在文——
      两面(deferred.json/P2 决策表)终态一致轻量核验;G12 无 defer 重评窗表
      (G12.1 候选决策表 = 法定输入直消费,无独立重评窗波次),全表深对账由
      g12.wave.6.decisions 门承载,本门不重复对账。
    """
    problems: list[str] = []
    for rd in RD_FINAL_OPEN_IDS:
        st = wel.load_rd_status(rd)
        if st != "open":
            problems.append(f"{rd} status={st!r} ≠ 'open'")
    if not P2_TABLE_PATH.is_file():
        problems.append("G12_P2_DECISIONS.md 缺失")
    else:
        text = P2_TABLE_PATH.read_text(encoding="utf-8")
        absent = [i for i in FROZEN_IDS if i not in text]
        if absent:
            problems.append(f"P2 表缺 FROZEN_IDS: {absent}")
        if len(FROZEN_IDS) != 33:
            problems.append(f"FROZEN_IDS n={len(FROZEN_IDS)} ≠ 33(闭集口径漂移)")
    return (not problems), "; ".join(problems) if problems else (
        "7 RD open 逐字一致 + P2 33 行闭集在树(G12 无重评窗表,两面一致)"
    )


def check_gap_registry_lock() -> tuple[bool, str]:
    """G-G12-10「生产化差距清单终审锁定(残余差距/未闭环行如实登记不冒充全闭环)」
    机器化面(终审锁定面 = G13 法定输入):

    - g12_ue_pt_gap_registry.json 在树且 10 行闭集:gap_id 集 == G12.4 锁定清单
      10 id(FROZEN_GAP_IDS)逐字全等(多一行/少一行/换一行即漂移);
    - 计数重算一致:total==10 == len(items),quality_gap==6,caliber_diff==4;
      scene_summary cornell-box==4/bistro-interior==6 且与各场景行数重算一致;
      not_ready_scenes==[];
    - quality_gap 6 行超容差项对账齐:measured_delta 非空 + delta == b−a 可溯源
      + g11_anchor 锚定 G15 字面非空(残余差距如实登记);
    - caliber_diff 4 行残余口径归属非空:description 非空 + ue5_module_primary
      ∈ {PathTracing.cpp, Other}(RXS-0391 归属枚举口径);
    - generated_by == M163 门字面(清单唯一生成面)。
    """
    problems: list[str] = []
    if not GAP_REGISTRY_PATH.is_file():
        return False, "g12_ue_pt_gap_registry.json 缺失"
    doc = wel.load_json(GAP_REGISTRY_PATH)
    items = doc.get("items") or []
    ids = {it.get("gap_id") for it in items}
    if len(items) != 10 or ids != FROZEN_GAP_IDS:
        problems.append(
            f"gap_id 闭集漂移: n={len(items)} extra={sorted(ids - FROZEN_GAP_IDS)} "
            f"missing={sorted(FROZEN_GAP_IDS - ids)}"
        )
    q_rows = [i for i in items if i.get("kind") == "quality_gap"]
    c_rows = [i for i in items if i.get("kind") == "caliber_diff"]
    if len(q_rows) != 6 or len(c_rows) != 4:
        problems.append(f"kind 计数漂移: quality_gap={len(q_rows)}/caliber_diff={len(c_rows)} ≠ 6/4")
    summary = doc.get("scene_summary") or {}
    for scene, want in (("cornell-box", 4), ("bistro-interior", 6)):
        got = (summary.get(scene) or {}).get("gap_count")
        recount = sum(1 for i in items if i.get("scene_id") == scene)
        if got != want or got != recount:
            problems.append(f"scene_summary.{scene}.gap_count={got} ≠ {want}/重算 {recount}")
        if (summary.get(scene) or {}).get("no_gap_explicit") is not False:
            problems.append(f"scene_summary.{scene}.no_gap_explicit 非 false")
    if doc.get("not_ready_scenes") != []:
        problems.append(f"not_ready_scenes 漂移: {doc.get('not_ready_scenes')}")
    for it in q_rows:
        ds = it.get("measured_delta") or []
        if not ds:
            problems.append(f"quality_gap 行 measured_delta 空: {it.get('gap_id')}")
        for d in ds:
            if abs(d.get("delta", 0.0) - (d.get("b_value", 0.0) - d.get("a_value", 0.0))) > 1e-18:
                problems.append(f"quality_gap 行 delta ≠ b−a 不可溯源: {it.get('gap_id')}")
        if "G15" not in str(it.get("g11_anchor") or ""):
            problems.append(f"quality_gap 行 g11_anchor 未锚定 G15: {it.get('gap_id')}")
    for it in c_rows:
        if not str(it.get("description") or "").strip():
            problems.append(f"caliber_diff 行归属描述空: {it.get('gap_id')}")
        if it.get("ue5_module_primary") not in ("PathTracing.cpp", "Other"):
            problems.append(f"caliber_diff 行归属枚举越闭集: {it.get('gap_id')} {it.get('ue5_module_primary')}")
    if doc.get("generated_by") != GAP_GENERATED_BY:
        problems.append(f"generated_by 漂移: {doc.get('generated_by')!r}")
    return (not problems), "; ".join(problems) if problems else (
        "生产化差距清单 10 行闭集终审锁定(gap_id 集 == G12.4 锁定清单逐字对账 + "
        "计数 10/6/4 重算一致 + quality_gap 超容差项对账齐 + caliber_diff 残余口径归属非空"
        "——残余差距如实登记不冒充全闭环;终审锁定面 = G13 法定输入)"
    )


def run_closeout() -> int:
    if NUMERIC_STEP <= 0:
        print("[7b] NUMERIC_STEP unset → BLOCKED", file=sys.stderr)
        return 1
    today = wel.utc_stamp()[:8]
    facts: list[dict] = []
    gate_rows = [verify_key_gate(k, p) for k, p in P0_P1_KEYS]
    gates_ok = all(r["status"] == "PASS" for r in gate_rows)
    facts.append(_fact("nine_keys_pass", gates_ok, f"pass={sum(1 for r in gate_rows if r['status']=='PASS')}/9"))

    wave_rows = [wel.require_gate_pass(k, p) for k, p in WAVE_EXITS]
    waves_ok = all(r["status"] == "PASS" for r in wave_rows)
    facts.append(_fact("wave_exits_2_to_7a", waves_ok, f"pass={sum(1 for r in wave_rows if r['status']=='PASS')}/{len(WAVE_EXITS)}"))

    # MAP 三向
    map_r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "check_g12_acceptance_map.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    map_ok = map_r.returncode == 0
    facts.append(_fact("acceptance_map_triple", map_ok, f"exit={map_r.returncode}"))

    # P2 33 行闭集最终状态无漂移
    p2 = wel.load_latest_evidence("g12_p2_decisions")
    p2_ok = False
    if p2:
        d = wel.load_json(p2)
        p2_ok = d.get("host_section_pass") is True
    p2_frozen_ok = P2_TABLE_PATH.is_file() and all(
        i in P2_TABLE_PATH.read_text(encoding="utf-8") for i in FROZEN_IDS
    ) and len(FROZEN_IDS) == 33
    facts.append(_fact(
        "p2_decisions_33_frozen",
        p2_ok and p2_frozen_ok,
        f"{str(p2.relative_to(ROOT)) if p2 else 'missing'}; frozen_33_in_tree={p2_frozen_ok}",
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

    # 7a 先行
    e7a = wel.load_latest_evidence("g12_stabilization_soak")
    e7a_ok = False
    e7a_commit = None
    if e7a:
        d7 = wel.load_json(e7a)
        e7a_ok = d7.get("host_section_pass") is True
        e7a_commit = d7.get("base_commit")
    facts.append(_fact(
        "soak_7a_precedes",
        e7a_ok,
        f"{str(e7a.relative_to(ROOT)) if e7a else 'missing'}; base_commit_7a={e7a_commit}"
        "(同日放行立项裁决 7:7a full-run 先行完成后允许同日 close-out)",
    ))

    # RD 最终状态逐字一致(G-G12-10)
    rd_ok, rd_detail = check_rd_final_state()
    facts.append(_fact("rd_final_state_consistent", rd_ok, rd_detail))

    # 生产化差距清单终审锁定 + 最后新绿 UTC 日留痕
    gap_ok, gap_detail = check_gap_registry_lock()
    last_green, missing = max_first_pass_date()
    facts.append(
        _fact(
            "gap_registry_locked_and_green_recorded",
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
        "base_commit_7a": e7a_commit,
        "required_gates": gate_rows + wave_rows,
        "extra_facts": facts,
        "subjects": [],
        "checks": {
            "nine_keys_pass": gates_ok,
            "wave_exits_pass": waves_ok,
            "acceptance_map_ok": map_ok,
            "p2_ok": p2_ok and p2_frozen_ok,
            "budget_strict_ok": bud_ok,
            "soak_7a_ok": e7a_ok,
            "rd_final_state_ok": rd_ok,
            "gap_registry_locked": gap_ok and bool(last_green) and not missing,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": (
            "same-day closeout allowed after 7a full-run(立项裁决 7,沿 G11.7b/G10.8b/G9.8b/G8.8b "
            "先例链); 生产化差距清单终审锁定(g12_ue_pt_gap_registry.json 10 行终态——quality_gap 6 "
            "+ caliber_diff 4,残余差距/未闭环行如实登记不冒充全闭环,G-G12-10 字面;终审锁定面 = "
            "G13 法定输入——G13 期只消费本清单与 G12_P2_DECISIONS 承接锚,不得另起无锚差距面); "
            "status flip is a separate commit after READY"
        ),
    }
    if SCHEMA_PATH.is_file():
        errs = wel.validate_schema(payload, SCHEMA_PATH)
        if errs:
            print(f"[7b] schema: {errs}", file=sys.stderr)
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
