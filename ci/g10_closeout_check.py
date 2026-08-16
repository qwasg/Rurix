#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.8 收口波）
"""G10.8b close-out 终审 g10.wave.8b.closeout(G10_CONTRACT G-G10-11;G10_PLAN §G10.8b;
G10_ACCEPTANCE_MAP §7;CI_GATES §5 wave8b 行;同构 ci/g9_closeout_check.py)。

只读汇总八 facts:①14 key(12 P0 + 2 go P1)逐门 PASS(wel 口径 + 顶层
status=="pass" 字面 + M130 双 phase 完整期核验)+ ②wave2~8a 七聚合/决策门
(exit×4 + 重评窗 + 决策 + soak)全 PASS + ③check_g10_acceptance_map 三向
exit=0 + ④P2 决策表 27 行闭集最终状态无漂移(最新 evidence host_section_pass
+ FROZEN_IDS 闭集在树)+ ⑤budget --strict 非空零 estimated/skip + ⑥8a
full-run 先行(base_commit 留痕;立项裁决 8 同日放行:8a full-run 先行完成后
允许同日 close-out)+ ⑦RD 最终状态逐字一致(deferred.json 七条目级 status 全
open + G10_P2_DECISIONS FROZEN_IDS 27 行在树 + G10_DEFER_REEVALUATION 十锚
终态在树,三面一致)+ ⑧差距清单终审锁定(g10_gap_registry.json 11 行闭集 +
每项 G11 承接锚非空——G11 法定输入)+ 最后新绿 UTC 日留痕。

输出 VERDICT = READY|BLOCKED。status flip 可与 READY 同波独立 commit。

用法:
  py -3 ci/g10_closeout_check.py --gate g10.wave.8b.closeout
  py -3 ci/g10_closeout_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g10_wave_exit_lib as wel  # noqa: E402
import g10_gap_registry_lib as gaplib  # noqa: E402
from g10_p2_decisions_check import DEFER_TEN, FROZEN_IDS  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g10.wave.8b.closeout"
NUMERIC_STEP = 195
SUBJECT = "g10_wave8b_closeout"
WAVE = "G10.8b"
SOURCE_REF = (
    "G10_CONTRACT G-G10-11;G10_PLAN §G10.8b;G10_ACCEPTANCE_MAP §7;CI_GATES §5 wave8b;"
    "14 key + wave2~8a 聚合 + MAP 三向 + P2 27 行闭集 + budget --strict + 8a 先行"
    "(同日放行立项裁决 8)+ RD 最终状态逐字一致 + 差距清单终审锁定(G11 法定输入)"
)
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_wave8b_closeout_evidence_schema.json"
P2_TABLE_PATH = ROOT / "milestones" / "g10" / "G10_P2_DECISIONS.md"
REEVALUATION_PATH = ROOT / "milestones" / "g10" / "G10_DEFER_REEVALUATION.md"
GAP_REGISTRY_PATH = ROOT / "milestones" / "g10" / "g10_gap_registry.json"

# 12 P0 + 2 go P1(G10_ACCEPTANCE_MAP §1/§2 实记;key/prefix 与
# ci/g10_stabilization_soak.py REGRESSION_GATES 前 14 行同一闭集)。
P0_P1_KEYS = [
    ("g10.p0.m128.ue5_capture_environment", "g10_m128_ue5_capture_environment"),
    ("g10.p0.m129.ue5_reference_frames", "g10_m129_ue5_reference_frames"),
    ("g10.p0.m130.dual_determinism_contract", "g10_m130_dual_determinism_contract"),
    ("g10.p0.m131.asset_license_registry", "g10_m131_asset_license_registry"),
    ("g10.p0.m132.corpus_loading", "g10_m132_corpus_loading"),
    ("g10.p1.m133.corpus_list_freeze", "g10_m133_corpus_list_freeze"),
    ("g10.p0.m134.frame_capture_pipeline", "g10_m134_frame_capture_pipeline"),
    ("g10.p0.m135.flip_metric", "g10_m135_flip_metric"),
    ("g10.p0.m136.ssim_psnr_metric", "g10_m136_ssim_psnr_metric"),
    ("g10.p0.m137.pixel_diff_report", "g10_m137_pixel_diff_report"),
    ("g10.p1.m138.metric_threshold_calibration", "g10_m138_metric_threshold_calibration"),
    ("g10.p0.m139.ab_comparison", "g10_m139_ab_comparison"),
    ("g10.p0.m140.gap_registry", "g10_m140_gap_registry"),
    ("g10.p0.m141.perf_baseline", "g10_m141_perf_baseline"),
]

WAVE_EXITS = [
    ("g10.wave.2.exit", "g10_wave2_exit"),
    ("g10.wave.3.exit", "g10_wave3_exit"),
    ("g10.wave.4.exit", "g10_wave4_exit"),
    ("g10.wave.5.exit", "g10_wave5_exit"),
    ("g10.wave.6.reevaluation", "g10_wave6_reevaluation"),
    ("g10.wave.7.decisions", "g10_p2_decisions"),
    ("g10.wave.8a.soak", "g10_stabilization_soak"),
]

# G10_CONTRACT §6 Deferred 处置表字面:七条目总体 status 全维持 open
# (分项 go/defer 由候选决策表、G10_P2_DECISIONS 与 deferred history 只追加留痕,
# 条目级 0-byte)。
RD_FINAL_OPEN_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044"]

# 差距清单终审锁定闭集(11 行;gap_id 按 RXS-0391 L3 冻结字节规则派生,
# 与 milestones/g10/g10_gap_registry.json 在树行集逐字对账——G11 法定输入,
# G11 修复范围只能消费该清单 + 其承接锚)。
FROZEN_GAP_IDS = frozenset({
    "ea68ebb265cb2bd5",  # R1 材质子集(baseColorFactor Lambert)
    "aa64a7a22dc16a0e",  # R2 几何法线(winding/双面翻转)
    "60a3ac2d1711912a",  # R3 灯种子集(sun+sky)
    "865d452e76fbaa45",  # R4 GI 屏幕探针单反弹
    "4fc5507c595f6c35",  # R5 JSON u64 顶格 seed 拒绝
    "03cac13af56a53cf",  # U1 cornell 壳体零辐射
    "6311bc89b610e019",  # U2 bistro 纹理全缺
    "0b7c50151f24137e",  # U3 Bistro 动画通道不消费
    "62692bbd57d731c3",  # C1 室内亮度主差(GI/天光口径差)
    "afba0e189a607ce2",  # C2 曝光链派生尺度
    "cc2b9ec7f19f0dad",  # C3 EXR 位深 fp16→f32
})


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def verify_key_gate(key: str, prefix: str) -> dict:
    """14 key 最新 evidence 终审核验:wel 口径 + 顶层 status=="pass" 字面
    (G10 证据形态统一,无豁免面)+ M130 双 phase 完整期(phase_g10_2_pass 且
    phase_g10_5_pass 同真,骨架期绿不替双端核验期充绿,MAP §3.3)。"""
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
    if key == "g10.p0.m130.dual_determinism_contract":
        if doc.get("phase_g10_2_pass") is not True or doc.get("phase_g10_5_pass") is not True:
            problems.append("M130 双 phase 未同真(phase_g10_2_pass/phase_g10_5_pass)")
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
    """G-G10-11「验收映射、候选决策、RD 最终状态逐字一致」机器化面:

    - deferred.json 七条 RD 条目级 status 逐字 == "open"(G10_CONTRACT §6
      处置表字面;分项历史只追加,条目级 0-byte);
    - G10_P2_DECISIONS.md 在树且 27 行 FROZEN_IDS 闭集逐 ID 在文;
    - G10_DEFER_REEVALUATION.md 在树且十锚 DEFER_TEN 闭集逐 ID 在文——
      三面(P2 决策表/重评窗表/deferred.json)终态一致的轻量核验(全表深
      对账由 g10.wave.6.reevaluation 与 g10.wave.7.decisions 门承载,本门
      不重复对账)。
    """
    problems: list[str] = []
    for rd in RD_FINAL_OPEN_IDS:
        st = wel.load_rd_status(rd)
        if st != "open":
            problems.append(f"{rd} status={st!r} ≠ 'open'")
    if not P2_TABLE_PATH.is_file():
        problems.append("G10_P2_DECISIONS.md 缺失")
    else:
        text = P2_TABLE_PATH.read_text(encoding="utf-8")
        absent = [i for i in FROZEN_IDS if i not in text]
        if absent:
            problems.append(f"P2 表缺 FROZEN_IDS: {absent}")
        if len(FROZEN_IDS) != 27:
            problems.append(f"FROZEN_IDS n={len(FROZEN_IDS)} ≠ 27(闭集口径漂移)")
        ten_absent = [i for i in DEFER_TEN if i not in text]
        if ten_absent:
            problems.append(f"P2 表缺十锚行: {ten_absent}")
    if not REEVALUATION_PATH.is_file():
        problems.append("G10_DEFER_REEVALUATION.md 缺失")
    else:
        rtext = REEVALUATION_PATH.read_text(encoding="utf-8")
        ten_absent = [i for i in DEFER_TEN if i not in rtext]
        if ten_absent:
            problems.append(f"重评窗表缺十锚行: {ten_absent}")
        if len(DEFER_TEN) != 10:
            problems.append(f"DEFER_TEN n={len(DEFER_TEN)} ≠ 10(闭集口径漂移)")
    return (not problems), "; ".join(problems) if problems else (
        "7 RD open 逐字一致 + P2 27 行闭集在树 + 重评窗十锚终态在树(三面一致)"
    )


def check_gap_registry_lock() -> tuple[bool, str]:
    """G-G10-11「差距清单终审锁定为 G11 法定输入」机器化面:

    - g10_gap_registry.json 在树且 gaplib.validate_registry 零错误(RXS-0391
      L1~L9;scene_set == M133 冻结清单双场景);
    - 11 行闭集:gap_id 集 == FROZEN_GAP_IDS 逐字全等(多一行/少一行/换一行
      即漂移);
    - 每项 g11_anchor 非空(G11 承接锚——G11 修复范围只能消费该清单 + 其
      承接锚);
    - kind 两值分列计数(quality_gap 8 / caliber_diff 3)防冒充;
    - generated_by == M139 门字面(清单唯一生成面)。
    """
    problems: list[str] = []
    if not GAP_REGISTRY_PATH.is_file():
        return False, "g10_gap_registry.json 缺失"
    doc = wel.load_json(GAP_REGISTRY_PATH)
    errs = gaplib.validate_registry(doc, scene_set=["cornell-box", "bistro-interior"])
    if errs:
        problems.append(f"清单校验非零错误: {errs[:3]}")
        return False, "; ".join(problems)
    items = doc.get("items") or []
    ids = {it.get("gap_id") for it in items}
    if len(items) != 11 or ids != FROZEN_GAP_IDS:
        problems.append(
            f"gap_id 闭集漂移: n={len(items)} extra={sorted(ids - FROZEN_GAP_IDS)} "
            f"missing={sorted(FROZEN_GAP_IDS - ids)}"
        )
    no_anchor = [it.get("gap_id") for it in items if not str(it.get("g11_anchor") or "").strip()]
    if no_anchor:
        problems.append(f"缺 G11 承接锚行: {no_anchor}")
    kinds = {}
    for it in items:
        kinds[it.get("kind")] = kinds.get(it.get("kind"), 0) + 1
    if kinds.get("quality_gap") != 8 or kinds.get("caliber_diff") != 3:
        problems.append(f"kind 计数漂移: {kinds} ≠ quality_gap 8 / caliber_diff 3")
    if doc.get("generated_by") != "ci/g10_ab_comparison_smoke.py --gate g10.p0.m139.ab_comparison":
        problems.append(f"generated_by 漂移: {doc.get('generated_by')!r}")
    return (not problems), "; ".join(problems) if problems else (
        "差距清单 11 行闭集锁定(R1~R5/U1~U3/C1~C3)+ 每项 G11 承接锚非空 + "
        "校验零错误——G11 法定输入"
    )


def run_closeout() -> int:
    if NUMERIC_STEP <= 0:
        print("[8b] NUMERIC_STEP unset → BLOCKED", file=sys.stderr)
        return 1
    today = wel.utc_stamp()[:8]
    facts: list[dict] = []
    gate_rows = [verify_key_gate(k, p) for k, p in P0_P1_KEYS]
    gates_ok = all(r["status"] == "PASS" for r in gate_rows)
    facts.append(_fact("fourteen_keys_pass", gates_ok, f"pass={sum(1 for r in gate_rows if r['status']=='PASS')}/14"))

    wave_rows = [wel.require_gate_pass(k, p) for k, p in WAVE_EXITS]
    waves_ok = all(r["status"] == "PASS" for r in wave_rows)
    facts.append(_fact("wave_exits_2_to_8a", waves_ok, f"pass={sum(1 for r in wave_rows if r['status']=='PASS')}/{len(WAVE_EXITS)}"))

    # MAP 三向
    map_r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "check_g10_acceptance_map.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    map_ok = map_r.returncode == 0
    facts.append(_fact("acceptance_map_triple", map_ok, f"exit={map_r.returncode}"))

    # P2 27 行闭集最终状态无漂移
    p2 = wel.load_latest_evidence("g10_p2_decisions")
    p2_ok = False
    if p2:
        d = wel.load_json(p2)
        p2_ok = d.get("host_section_pass") is True
    p2_frozen_ok = P2_TABLE_PATH.is_file() and all(
        i in P2_TABLE_PATH.read_text(encoding="utf-8") for i in FROZEN_IDS
    ) and len(FROZEN_IDS) == 27
    facts.append(_fact(
        "p2_decisions_27_frozen",
        p2_ok and p2_frozen_ok,
        f"{str(p2.relative_to(ROOT)) if p2 else 'missing'}; frozen_27_in_tree={p2_frozen_ok}",
    ))

    # budget strict
    bud = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "budget_eval.py"), "--strict"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    bud_ok = bud.returncode == 0 and "[budget_eval] PASS" in ((bud.stdout or "") + (bud.stderr or ""))
    facts.append(_fact("budget_strict", bud_ok, f"exit={bud.returncode}"))

    # 8a 先行
    e8a = wel.load_latest_evidence("g10_stabilization_soak")
    e8a_ok = False
    e8a_commit = None
    if e8a:
        d8 = wel.load_json(e8a)
        e8a_ok = d8.get("host_section_pass") is True
        e8a_commit = d8.get("base_commit")
    facts.append(_fact("soak_8a_precedes", e8a_ok, str(e8a.relative_to(ROOT)) if e8a else "missing"))

    # RD 最终状态逐字一致(G-G10-11)
    rd_ok, rd_detail = check_rd_final_state()
    facts.append(_fact("rd_final_state_consistent", rd_ok, rd_detail))

    # 差距清单终审锁定(G11 法定输入)+ 最后新绿 UTC 日留痕
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
        "base_commit_8a": e8a_commit,
        "required_gates": gate_rows + wave_rows,
        "extra_facts": facts,
        "subjects": [],
        "checks": {
            "fourteen_keys_pass": gates_ok,
            "wave_exits_pass": waves_ok,
            "acceptance_map_ok": map_ok,
            "p2_ok": p2_ok and p2_frozen_ok,
            "budget_strict_ok": bud_ok,
            "soak_8a_ok": e8a_ok,
            "rd_final_state_ok": rd_ok,
            "gap_registry_locked": gap_ok and bool(last_green) and not missing,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": (
            "same-day closeout allowed after 8a full-run(立项裁决 8 继承 G9.8b/G8.8b 先例链); "
            "差距清单终审锁定为 G11 法定输入(G11 修复范围只能消费 g10_gap_registry.json 11 行 "
            "闭集 + 其承接锚); status flip is a separate commit after READY"
        ),
    }
    if SCHEMA_PATH.is_file():
        errs = wel.validate_schema(payload, SCHEMA_PATH)
        if errs:
            print(f"[8b] schema: {errs}", file=sys.stderr)
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
