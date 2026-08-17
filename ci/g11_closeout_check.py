#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.6/G11.7 收口波）
"""G11.7b close-out 终审 g11.wave.7b.closeout(G11_CONTRACT G-G11-10;G11_PLAN §2
G11.7b;CI_GATES §5 wave7b 行;同构 ci/g10_closeout_check.py)。

只读汇总八 facts:①14 key(13 P0 + 1 go P1)逐门 PASS(wel 口径 + 顶层
status=="pass" 字面 + M147 双 phase 机核:最新 phase==g11.3 件 status=="pass"
且最新 g11.5 phase 件 closure.verdict=="converged",契约 §8.3a definitive 收敛
断言面不遮蔽)+ ②wave2~7a 六聚合/决策门(exit×4 + decisions + soak)全 PASS +
③check_g11_acceptance_map 三向 exit=0 + ④P2 决策表 28 行闭集最终状态无漂移
(最新 evidence host_section_pass + FROZEN_IDS 28 行闭集在树)+ ⑤budget
--strict 非空零 estimated/skip + ⑥7a full-run 先行(base_commit_7a 留痕;立项
裁决 7 同日放行:7a full-run 先行完成后允许同日 close-out)+ ⑦RD 最终状态逐字
一致(deferred.json RD-034/039~044 七条目级 status 全 open 逐字 + G11_P2_DECISIONS
28 行 FROZEN_IDS 闭集在树;G11 无 defer 重评窗表——G11.1 如实登记法定输入直
消费无独立重评窗波次,两面一致,全表深对账由 wave6 门承载不重复)+ ⑧复测差距
清单终审锁定(g11_5b_retest_gap_registry.json 11 行闭集:gap_id 集 == G10.8b
锁定清单 11 id 逐字对账 + summary 计数重算一致〔total 11/converged 8/
aligned_closed 3/partial 0/new_items 0〕+ C1 行 attribution 残余归属非空——
残余差距/未闭环行如实登记不冒充全闭环,G-G11-10 字面)+ 最后新绿 UTC 日留痕。

输出 VERDICT = READY|BLOCKED。status flip 可与 READY 同波独立 commit。

用法:
  py -3 ci/g11_closeout_check.py --gate g11.wave.7b.closeout
  py -3 ci/g11_closeout_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
from g11_p2_decisions_check import FROZEN_IDS  # noqa: E402
from g10_closeout_check import FROZEN_GAP_IDS  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g11.wave.7b.closeout"
NUMERIC_STEP = 216  # 落盘前实测 registry/number_ledger.json CI_step.next_free=216 顺位领取
SUBJECT = "g11_wave7b_closeout"
WAVE = "G11.7b"
SOURCE_REF = (
    "G11_CONTRACT G-G11-10;G11_PLAN §2 G11.7b;CI_GATES §5 wave7b;"
    "14 key + wave2~7a 聚合/决策 + MAP 三向 + P2 28 行闭集 + budget --strict + 7a 先行"
    "(同日放行立项裁决 7)+ RD 最终状态逐字一致 + 复测差距清单终审锁定(11 行终态)"
)
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_wave7b_closeout_evidence_schema.json"
P2_TABLE_PATH = ROOT / "milestones" / "g11" / "G11_P2_DECISIONS.md"
RETEST_REGISTRY_PATH = ROOT / "milestones" / "g11" / "g11_5b_retest_gap_registry.json"

# 13 P0 + 1 go P1(G11_ACCEPTANCE_MAP §1/§2 实记;key/prefix 与
# ci/g11_stabilization_soak.py REGRESSION_GATES 前 14 行同一闭集)。
P0_P1_KEYS = [
    ("g11.p0.m144.caliber_c1_indoor_luminance", "g11_m144_caliber_c1_indoor_luminance"),
    ("g11.p0.m145.caliber_c2_exposure_chain", "g11_m145_caliber_c2_exposure_chain"),
    ("g11.p0.m146.caliber_c3_exr_bit_depth", "g11_m146_caliber_c3_exr_bit_depth"),
    ("g11.p1.m157.hdr_flip_calibration", "g11_m157_hdr_flip_calibration"),
    ("g11.p0.m147.fix_r1_material_subset", "g11_m147_fix_r1_material_subset"),
    ("g11.p0.m148.fix_r2_geometry_normals", "g11_m148_fix_r2_geometry_normals"),
    ("g11.p0.m149.fix_r5_json_u64_seed", "g11_m149_fix_r5_json_u64_seed"),
    ("g11.p0.m150.fix_u1_cornell_shell_radiance", "g11_m150_fix_u1_cornell_shell_radiance"),
    ("g11.p0.m151.fix_u2_bistro_texture_dds", "g11_m151_fix_u2_bistro_texture_dds"),
    ("g11.p0.m152.fix_u3_bistro_animation", "g11_m152_fix_u3_bistro_animation"),
    ("g11.p0.m153.fix_r3_light_subset", "g11_m153_fix_r3_light_subset"),
    ("g11.p0.m154.fix_r4_gi_multibounce_world_cache", "g11_m154_fix_r4_gi_multibounce_world_cache"),
    ("g11.p0.m155.ab_retest_closure", "g11_m155_ab_retest_closure"),
    ("g11.p0.m156.regression_guard", "g11_m156_regression_guard"),
]

WAVE_EXITS = [
    ("g11.wave.2.exit", "g11_wave2_exit"),
    ("g11.wave.3.exit", "g11_wave3_exit"),
    ("g11.wave.4.exit", "g11_wave4_exit"),
    ("g11.wave.5.exit", "g11_wave5_exit"),
    ("g11.wave.6.decisions", "g11_p2_decisions"),
    ("g11.wave.7a.soak", "g11_stabilization_soak"),
]

# G11_CONTRACT §6 Deferred 处置表字面:七条目总体 status 全维持 open
# (分项 go/defer 由候选决策表、G11_P2_DECISIONS 与 deferred history 只追加留痕,
# 条目级 0-byte)。
RD_FINAL_OPEN_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044"]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _m147_g11_5_latest() -> Path | None:
    """M147 最新 phase==g11.5 evidence(definitive 收敛断言面)。"""
    best: tuple[str, Path] | None = None
    for p in wel.EVIDENCE_DIR.glob("g11_m147_fix_r1_material_subset_*.json"):
        m = re.search(r"_(\d{8}T\d{6}Z)\.json$", p.name)
        if m is None:
            continue
        try:
            doc = wel.load_json(p)
        except (OSError, json.JSONDecodeError):
            continue
        if doc.get("phase") != "g11.5":
            continue
        if best is None or m.group(1) > best[0]:
            best = (m.group(1), p)
    return best[1] if best else None


def verify_key_gate(key: str, prefix: str) -> dict:
    """14 key 最新 evidence 终审核验:wel 口径 + 顶层 status=="pass" 字面
    (G11 证据形态统一,无豁免面)+ M147 双 phase 机核(最新 phase==g11.3 件
    status=="pass" 且最新 g11.5 phase 件 closure.verdict=="converged",
    契约 §8.3a——g11.3 登记面绿不替 g11.5 收敛断言充绿)。"""
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
    if key == "g11.p0.m147.fix_r1_material_subset":
        if doc.get("phase") != "g11.3":
            problems.append(f"最新 evidence phase={doc.get('phase')!r} ≠ 'g11.3'")
        p5 = _m147_g11_5_latest()
        if p5 is None:
            problems.append("缺 phase==g11.5 evidence(definitive 收敛断言面)")
        else:
            try:
                d5 = wel.load_json(p5)
            except (OSError, ValueError):
                d5 = {}
            verdict = (d5.get("closure") or {}).get("verdict")
            if verdict != "converged":
                problems.append(f"最新 g11.5 phase 件 verdict={verdict!r} ≠ 'converged'")
    if problems:
        row["status"] = "FAIL"
        row["detail"] = f"{row.get('detail', '')}; " + "; ".join(problems)
    return row


def evidence_utc_date(path: Path | None) -> str | None:
    if path is None:
        return None
    m = wel._UTC_STAMP_RE.search(path.name)
    if m:
        return m.group(1)[:8]
    doc = wel.load_json(path)
    ts = doc.get("timestamp") or doc.get("utc_date") or ""
    return str(ts)[:8] if ts else None


def max_first_pass_date() -> tuple[str | None, list[str]]:
    """对 14 key 取最新 PASS evidence 的 UTC 日期的 max(近似『最后新绿』)。"""
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
    """G-G11-10「验收映射、候选决策、RD 最终状态逐字一致」机器化面:

    - deferred.json 七条 RD 条目级 status 逐字 == "open"(G11_CONTRACT §6
      处置表字面;分项历史只追加,条目级 0-byte);
    - G11_P2_DECISIONS.md 在树且 28 行 FROZEN_IDS 闭集逐 ID 在文——
      两面(deferred.json/P2 决策表)终态一致轻量核验;G11 无 defer 重评窗表
      (G11.1 如实登记:法定输入直消费,无独立重评窗波次),全表深对账由
      g11.wave.6.decisions 门承载,本门不重复对账。
    """
    problems: list[str] = []
    for rd in RD_FINAL_OPEN_IDS:
        st = wel.load_rd_status(rd)
        if st != "open":
            problems.append(f"{rd} status={st!r} ≠ 'open'")
    if not P2_TABLE_PATH.is_file():
        problems.append("G11_P2_DECISIONS.md 缺失")
    else:
        text = P2_TABLE_PATH.read_text(encoding="utf-8")
        absent = [i for i in FROZEN_IDS if i not in text]
        if absent:
            problems.append(f"P2 表缺 FROZEN_IDS: {absent}")
        if len(FROZEN_IDS) != 28:
            problems.append(f"FROZEN_IDS n={len(FROZEN_IDS)} ≠ 28(闭集口径漂移)")
    return (not problems), "; ".join(problems) if problems else (
        "7 RD open 逐字一致 + P2 28 行闭集在树(G11 无重评窗表,两面一致)"
    )


def check_retest_registry_lock() -> tuple[bool, str]:
    """G-G11-10「复测差距清单终审锁定(残余差距/未闭环行如实登记不冒充全闭环)」
    机器化面:

    - g11_5b_retest_gap_registry.json 在树且 11 行闭集:gap_id 集 == G10.8b
      锁定清单 11 id(FROZEN_GAP_IDS)逐字全等(多一行/少一行/换一行即漂移);
    - summary 计数重算一致:total==11 且 == len(items),converged==8,
      aligned_closed==3,partial==0 且 partial_rows==[],new_items==0;
    - C1 行 attribution 残余归属非空(残余差距如实登记不冒充全闭环——
      R3/R4 残余 + sky-ibl 落地残余 + c1_ue_specular_ibl ≤0.03% +
      c3_source_bit_depth_quantization + g11_5b_sun_through_glass_tail 五元归属);
    - generated_by == g11_5b harness 字面(清单唯一生成面)。
    """
    problems: list[str] = []
    if not RETEST_REGISTRY_PATH.is_file():
        return False, "g11_5b_retest_gap_registry.json 缺失"
    doc = wel.load_json(RETEST_REGISTRY_PATH)
    items = doc.get("items") or []
    ids = {it.get("gap_id") for it in items}
    if len(items) != 11 or ids != FROZEN_GAP_IDS:
        problems.append(
            f"gap_id 闭集漂移: n={len(items)} extra={sorted(ids - FROZEN_GAP_IDS)} "
            f"missing={sorted(FROZEN_GAP_IDS - ids)}"
        )
    summary = doc.get("summary") or {}
    re_count = {
        "total": len(items),
        "converged": sum(1 for i in items if i.get("closure_status") == "converged"),
        "aligned_closed": sum(1 for i in items if i.get("closure_status") == "aligned_closed"),
        "partial": sum(1 for i in items if i.get("closure_status") == "partial"),
    }
    if summary.get("total") != 11 or summary.get("total") != re_count["total"]:
        problems.append(f"summary.total={summary.get('total')} ≠ 11/重算 {re_count['total']}")
    for k in ("converged", "aligned_closed", "partial"):
        if summary.get(k) != re_count[k]:
            problems.append(f"summary.{k}={summary.get(k)} ≠ 重算 {re_count[k]}")
    if summary.get("converged") != 8 or summary.get("aligned_closed") != 3 or summary.get("partial") != 0:
        problems.append(
            f"summary 终态漂移: converged={summary.get('converged')}/aligned_closed="
            f"{summary.get('aligned_closed')}/partial={summary.get('partial')} ≠ 8/3/0"
        )
    if summary.get("partial_rows") != [] or summary.get("new_items") != 0:
        problems.append(f"partial_rows/new_items 漂移: {summary.get('partial_rows')}/{summary.get('new_items')}")
    c1 = [i for i in items if str(i.get("title") or "").startswith("C1 ")]
    if not c1:
        problems.append("缺 C1 行")
    elif not str(c1[0].get("attribution") or "").strip():
        problems.append("C1 行 attribution 残余归属空(残余差距未如实登记)")
    if doc.get("generated_by") != "milestones/g11/harness/g11_5b_ab_rerun.py --stage registry":
        problems.append(f"generated_by 漂移: {doc.get('generated_by')!r}")
    return (not problems), "; ".join(problems) if problems else (
        "复测差距清单 11 行闭集终审锁定(gap_id 集 == G10.8b 锁定清单逐字对账 + "
        "summary 8/3/0 重算一致 + C1 残余归属非空——残余差距如实登记不冒充全闭环)"
    )


def run_closeout() -> int:
    if NUMERIC_STEP <= 0:
        print("[7b] NUMERIC_STEP unset → BLOCKED", file=sys.stderr)
        return 1
    today = wel.utc_stamp()[:8]
    facts: list[dict] = []
    gate_rows = [verify_key_gate(k, p) for k, p in P0_P1_KEYS]
    gates_ok = all(r["status"] == "PASS" for r in gate_rows)
    facts.append(_fact("fourteen_keys_pass", gates_ok, f"pass={sum(1 for r in gate_rows if r['status']=='PASS')}/14"))

    wave_rows = [wel.require_gate_pass(k, p) for k, p in WAVE_EXITS]
    waves_ok = all(r["status"] == "PASS" for r in wave_rows)
    facts.append(_fact("wave_exits_2_to_7a", waves_ok, f"pass={sum(1 for r in wave_rows if r['status']=='PASS')}/{len(WAVE_EXITS)}"))

    # MAP 三向
    map_r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "check_g11_acceptance_map.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    map_ok = map_r.returncode == 0
    facts.append(_fact("acceptance_map_triple", map_ok, f"exit={map_r.returncode}"))

    # P2 28 行闭集最终状态无漂移
    p2 = wel.load_latest_evidence("g11_p2_decisions")
    p2_ok = False
    if p2:
        d = wel.load_json(p2)
        p2_ok = d.get("host_section_pass") is True
    p2_frozen_ok = P2_TABLE_PATH.is_file() and all(
        i in P2_TABLE_PATH.read_text(encoding="utf-8") for i in FROZEN_IDS
    ) and len(FROZEN_IDS) == 28
    facts.append(_fact(
        "p2_decisions_28_frozen",
        p2_ok and p2_frozen_ok,
        f"{str(p2.relative_to(ROOT)) if p2 else 'missing'}; frozen_28_in_tree={p2_frozen_ok}",
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
    e7a = wel.load_latest_evidence("g11_stabilization_soak")
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

    # RD 最终状态逐字一致(G-G11-10)
    rd_ok, rd_detail = check_rd_final_state()
    facts.append(_fact("rd_final_state_consistent", rd_ok, rd_detail))

    # 复测差距清单终审锁定 + 最后新绿 UTC 日留痕
    gap_ok, gap_detail = check_retest_registry_lock()
    last_green, missing = max_first_pass_date()
    facts.append(
        _fact(
            "retest_registry_locked_and_green_recorded",
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
            "fourteen_keys_pass": gates_ok,
            "wave_exits_pass": waves_ok,
            "acceptance_map_ok": map_ok,
            "p2_ok": p2_ok and p2_frozen_ok,
            "budget_strict_ok": bud_ok,
            "soak_7a_ok": e7a_ok,
            "rd_final_state_ok": rd_ok,
            "retest_registry_locked": gap_ok and bool(last_green) and not missing,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": (
            "same-day closeout allowed after 7a full-run(立项裁决 7,沿 G10.8b/G9.8b/G8.8b "
            "先例链); 复测差距清单终审锁定(g11_5b_retest_gap_registry.json 11 行终态——"
            "converged 8 + aligned_closed 3 + partial 0,残余差距 C1 attribution 五元归属 "
            "如实登记不冒充全闭环,G-G11-10 字面); status flip is a separate commit after READY"
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
