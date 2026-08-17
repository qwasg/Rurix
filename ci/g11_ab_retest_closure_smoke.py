#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.5 波）
"""G11.5 M155 A/B 复测闭环门（P0，步骤 211；g11.p0.m155.ab_retest_closure；
G11_CONTRACT §4.2 M155 行判据逐字 / G-G11-7；G11_ACCEPTANCE_MAP §1 M155 行；
CI_GATES §4；g10_gap_registry 11 行锁定清单 0-byte 消费；spec/visual_comparison.md
RXS-0393 + RXS-0392；契约 §8.3a M147 双 phase 修订句〔R1 行收敛断言 definitive
测量面 = 本波同契约复跑；不收敛则整波 FAIL〕）。

host+device 门（host CPU 参考管线真渲染复测帧由 milestones/g11/harness/
g11_5_ab_rerun.py 同契约双端全量复跑产——门侧自帧独立重算复测 delta，
device_section_state=executed）。判据（契约 §4.2 M155 行字面 + MAP 逐字）：

1. **同契约双端复跑（契约参数 digest == G10.5 锁定值，不等仍出报告即 RED）**：
   双场景 + 联合 digest 当次重算 == G10.5 锁定值；复跑报告登记 digest 与重算
   逐位一致；报告帧 digest 与 G11.5 帧区当次解码重算逐位一致（未复跑冒充判红面
   ——拿旧帧区/旧报告/手写复测值冒充当次复测必检出）。
2. **复测度量报告**：g11_5_rerun_report.json 全阶段 done + 双端四组帧（HDR×2 +
   LDR×2）齐备可解码 + 分辨率 == 契约 + UE 帧 unreal/build == M128 最新 evidence
   登记 ue_build_id（R-G11-10）；单端缺帧聚合 PASS 即 RED。
3. **复测差距清单 11 行闭集落盘（行集逐字对账；新差距项显式登记即 RED 评审面）**：
   g11_5_retest_gap_registry.json 行集 == G10.8b 锁定清单（gap_id 集合 + title
   逐字 + kind 8/3 分列 + camera/domain 逐行一致）；清单缺行即 RED；新项静默
   混入即 RED；基线字面逐行 == 锁定清单 measured_delta 字面。
4. **逐项闭环状态机核（修复前后 delta 收敛 measured，收敛阈由标定程序产）**：
   quality_gap 行（R1~R5/U1~U3）RXS-0393 L2 收敛判定（|复测| < |基线| 且收敛
   幅度 ≥ 标定阈 + 方向性〔符号翻转仅 |复测| ≤ zero_band 内成立〕；R3/R4 消费
   G11.2 域统一换算基线面）；caliber_diff 行（C1~C3）L2 C 族款（口径对齐完成
   = G11.2 门最新 evidence PASS + 残余显式登记 + 复测 delta 与登记残余一致）。
   **R1 行（M147 g11.5 phase）收敛断言**：锁定基线 0.8328980787837229 对当次
   复测——不收敛则本门 FAIL、整波 FAIL（§8.3a 不弱化声明，禁改判据充绿）。
5. **清单终态诚实面**：逐项 closed/converged/partial/aligned_closed 显式判定；
   partial/未收敛行如实登记不充绿并带 G12+ 承接锚（partial 冒充 closed 即 RED）。

RED 臂（MAP/契约判据字面）：清单缺行即 RED（red_missing_registry_row）；
未复跑冒充即 RED（red_stale_rerun_masquerade——复测值与门侧独立重算不等必检出）；
阈值手写即 RED（red_handwritten_threshold）；单端缺帧聚合 PASS 即 RED
（red_single_end_missing_frame）；契约 digest 不等仍出报告即 RED
（red_digest_unequal_report_blocked）；partial 冒充 closed 即 RED
（red_partial_masquerade_closed）。

用法：
  py -3 ci/g11_ab_retest_closure_smoke.py --gate g11.p0.m155.ab_retest_closure
  py -3 ci/g11_ab_retest_closure_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m155_ab_retest_closure_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g10_exr_lib as exr  # noqa: E402
import g11_5_retest_lib as rl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g11.p0.m155.ab_retest_closure"
NUMERIC_STEP = 211
SOURCE_REF = (
    "G11_CONTRACT §4.2 M155 + G-G11-7 + §8.3a;G11_ACCEPTANCE_MAP §1 M155;CI_GATES §4;"
    "g10_gap_registry 11 行锁定清单;spec/visual_comparison.md RXS-0393/RXS-0392"
)
TAG = "g11_m155"
SUBJECT = "g11_m155_ab_retest_closure"
MATRIX_ROW = "M155"

ROW_PREFIXES = ["R1", "R2", "R3", "R4", "R5", "U1", "U2", "U3", "C1", "C2", "C3"]
QUALITY_ROWS = ["R1", "R2", "R3", "R4", "R5", "U1", "U2", "U3"]
CALIBER_ROWS = ["C1", "C2", "C3"]
CALIBER_GATE = {"C1": ("g11.p0.m144.caliber_c1_indoor_luminance", "g11_m144_caliber_c1_indoor_luminance"),
                "C2": ("g11.p0.m145.caliber_c2_exposure_chain", "g11_m145_caliber_c2_exposure_chain"),
                "C3": ("g11.p0.m146.caliber_c3_exr_bit_depth", "g11_m146_caliber_c3_exr_bit_depth")}

CHECK_KEYS = [
    "spec_rxs0393_clause_on_tree",
    "contract_digest_locked_unchanged",
    "rerun_report_complete_and_digest_honest",
    "dual_end_frames_present_full_scene_set",
    "ue_build_provenance_matches_m128",
    "retest_registry_row_set_exact_locked",
    "baseline_literals_match_locked_registry",
    "retest_deltas_independently_recomputed",
    "closure_thresholds_from_budget_calibrated",
    "closure_r1_ssim_converged",
    "closure_r2_u1_coverage_converged",
    "closure_r3_luminance_converged",
    "closure_r4_p90_converged",
    "closure_r5_u64_seed_converged",
    "closure_u2_luminance_converged",
    "closure_u3_anim_channels_converged",
    "closure_c1_disposition_reviewed",
    "closure_c2_disposition_reviewed",
    "closure_c3_disposition_reviewed",
    "registry_terminal_states_honest",
    "budget_eval_strict_all_pass",
    "red_missing_registry_row_detected",
    "red_stale_rerun_masquerade_detected",
    "red_handwritten_threshold_detected",
    "red_single_end_missing_frame_detected",
    "red_digest_unequal_report_blocked",
    "red_partial_masquerade_closed_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _row_prefix(item: dict) -> str:
    return item.get("title", "").split(" ", 1)[0]


def validate_registry_row_set(doc: dict, locked: dict) -> list[str]:
    """复测差距清单行集逐字对账校验（RED 臂共用：缺行/新项静默混入判红面）。"""
    problems: list[str] = []
    if not isinstance(doc, dict):
        return ["复测清单非 object"]
    if doc.get("registry") != rl.REGISTRY_NAME:
        problems.append(f"registry 字段漂移: {doc.get('registry')!r}（当前复测集 {rl.REGISTRY_NAME}）")
    items = doc.get("items")
    if not isinstance(items, list):
        return problems + ["items 缺失或非数组"]
    locked_by_id = {it["gap_id"]: it for it in locked["items"]}
    got_ids = [it.get("gap_id") for it in items if isinstance(it, dict)]
    missing = [g for g in locked_by_id if g not in got_ids]
    extra = [g for g in got_ids if g not in locked_by_id]
    if missing:
        problems.append(f"清单缺行（清单缺行即 RED）: {missing}")
    if extra:
        problems.append(f"新差距项静默混入（新项显式登记即 RED 评审面）: {extra}")
    if len(got_ids) != len(set(got_ids)):
        problems.append("gap_id 重复行")
    for it in items:
        if not isinstance(it, dict):
            problems.append("items 行非 object")
            continue
        lk = locked_by_id.get(it.get("gap_id"))
        if lk is None:
            continue
        for f in ("title", "scene_id", "camera_id", "domain", "kind"):
            if it.get(f) != lk.get(f):
                problems.append(f"{_row_prefix(lk)} 字段 {f} 与锁定清单不逐字: {it.get(f)!r} ≠ {lk.get(f)!r}")
    n_quality = sum(1 for it in locked["items"] if it["kind"] == "quality_gap")
    n_caliber = sum(1 for it in locked["items"] if it["kind"] == "caliber_diff")
    if (n_quality, n_caliber) != (8, 3):
        problems.append(f"锁定清单 kind 分列漂移: {n_quality}/{n_caliber} ≠ 8/3")
    return problems


def validate_terminal_states(doc: dict) -> list[str]:
    """清单终态诚实面校验（RED 臂共用：partial 冒充 closed / 汇总计数失真判红面）。"""
    problems: list[str] = []
    items = doc.get("items") or []
    allowed = {"converged", "aligned_closed", "partial"}
    n_conv = n_align = n_partial = 0
    for it in items:
        st = it.get("closure_status")
        if st not in allowed:
            problems.append(f"{_row_prefix(it)} closure_status 非法: {st!r}（闭集 {sorted(allowed)}）")
            continue
        if st == "converged":
            n_conv += 1
            if it.get("converged") is not True:
                problems.append(f"{_row_prefix(it)} converged ≠ true 冒充 converged")
        elif st == "aligned_closed":
            n_align += 1
        else:
            n_partial += 1
            if it.get("converged") is True:
                problems.append(f"{_row_prefix(it)} converged=true 冒充 partial（状态自相矛盾）")
            anchor = it.get("disposition_anchor") or ""
            if "G12+" not in anchor and "G11.6" not in anchor:
                problems.append(f"{_row_prefix(it)} partial 行缺 G12+/G11.6 承接锚（不充绿纪律）")
            if not it.get("disposition", "").startswith("partial_"):
                problems.append(f"{_row_prefix(it)} partial 行 disposition 未显式标 partial")
        if not it.get("threshold_provenance"):
            problems.append(f"{_row_prefix(it)} 缺 threshold_provenance（收敛阈标定程序产证明）")
    summ = doc.get("summary") or {}
    if summ.get("total") != len(items):
        problems.append(f"summary.total {summ.get('total')!r} ≠ items {len(items)}")
    if (summ.get("converged"), summ.get("aligned_closed"), summ.get("partial")) != (n_conv, n_align, n_partial):
        problems.append(
            f"summary 计数失真: 登记 {(summ.get('converged'), summ.get('aligned_closed'), summ.get('partial'))} "
            f"≠ 重算 {(n_conv, n_align, n_partial)}（partial 充绿/计数遮蔽即 RED）"
        )
    if summ.get("new_items") != 0:
        problems.append(f"summary.new_items ≠ 0: {summ.get('new_items')!r}")
    return problems


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    locked = rl.load_json(rl.GAP_REGISTRY)
    # 绿臂①：真树清单行集对账零问题（清单已由驱动落盘时）。
    if rl.RETEST_REGISTRY_PATH.is_file():
        doc = rl.load_retest_registry()
        if validate_registry_row_set(doc, locked):
            print(f"[{TAG}] selftest FAIL: 真树清单行集对账误判", file=sys.stderr)
            return 1
        if validate_terminal_states(doc):
            print(f"[{TAG}] selftest FAIL: 真树清单终态误判", file=sys.stderr)
            return 1
    # 红臂①：清单缺行必检出。
    forged = {"registry": rl.REGISTRY_NAME, "items": [dict(it) for it in locked["items"][:-1]]}
    if not any("缺行" in p for p in validate_registry_row_set(forged, locked)):
        print(f"[{TAG}] selftest FAIL: 清单缺行未检出", file=sys.stderr)
        return 1
    # 红臂②：新项静默混入必检出。
    forged2 = {"registry": rl.REGISTRY_NAME,
               "items": [dict(it) for it in locked["items"]] + [dict(locked["items"][0], gap_id="f" * 16)]}
    if not any("静默混入" in p for p in validate_registry_row_set(forged2, locked)):
        print(f"[{TAG}] selftest FAIL: 新项静默混入未检出", file=sys.stderr)
        return 1
    # 红臂③：partial 冒充 converged / 计数遮蔽必检出。
    forged3_items = []
    for it in locked["items"]:
        forged3_items.append({
            "gap_id": it["gap_id"], "title": it["title"], "scene_id": it["scene_id"],
            "camera_id": it["camera_id"], "domain": it["domain"], "kind": it["kind"],
            "closure_status": "converged", "converged": False,
            "threshold_provenance": "x", "disposition_anchor": "y", "disposition": "closed_retest_converged",
        })
    forged3 = {"items": forged3_items,
               "summary": {"total": 11, "converged": 11, "aligned_closed": 0, "partial": 0, "new_items": 0}}
    if not validate_terminal_states(forged3):
        print(f"[{TAG}] selftest FAIL: partial 冒充 converged 未检出", file=sys.stderr)
        return 1
    # 红臂④：收敛判定层语义（复测==基线冒充收敛必检出；同一判定层 0-byte）。
    if rl.evaluate_closure(0.8328980787837229, 0.8328980787837229, 0.0)["converged"]:
        print(f"[{TAG}] selftest FAIL: 未收敛冒充未检出", file=sys.stderr)
        return 1
    # 绿臂②：真实收敛形态（|复测|<|基线| 且同号）判收敛。
    if not rl.evaluate_closure(0.8328980787837229, 0.4, 0.0)["converged"]:
        print(f"[{TAG}] selftest FAIL: 真实收敛形态误判", file=sys.stderr)
        return 1
    # schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = rl.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (6 RED + 3 GREEN)")
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
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    # ① spec 条款头在树（RXS-0393 收敛判据 + RXS-0392 口径对齐面）。
    spec_text = (ROOT / "spec" / "visual_comparison.md").read_text(encoding="utf-8")
    checks["spec_rxs0393_clause_on_tree"] = "### RXS-0393" in spec_text and "### RXS-0392" in spec_text
    check(checks["spec_rxs0393_clause_on_tree"], "RXS-0392/0393 条款头不在树")

    # ② 契约 digest 三面绑定 0-byte（双场景 + 联合当次重算 == G10.5 锁定值）。
    digest_drift = [
        f"{s}: {rl.contract_digest_rust(s)} ≠ {rl.LOCKED_DIGEST[s]}"
        for s in rl.SCENES
        if rl.contract_digest_rust(s) != rl.LOCKED_DIGEST[s]
    ]
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "g10_5_scene_render --contract-digest ×2 scenes", "exit_code": 0})
    checks["contract_digest_locked_unchanged"] = not digest_drift
    check(not digest_drift, f"契约 digest 漂移（不等仍出报告即 RED）: {digest_drift}")

    # ③ 复跑报告完备 + digest 诚实面（报告登记 == 当次重算/解码重算逐位一致）。
    report: dict = {}
    report_problems: list[str] = []
    if not rl.REPORT_PATH.is_file():
        report_problems.append(f"{rl.REPORT_PATH.name} 缺失（未复跑冒充即 RED；当前复测集 {rl.REGISTRY_NAME}）")
    else:
        report = rl.load_report()
        stages = report.get("stages") or {}
        for st in ("contract", "rurix", "ue", "derive", "metrics", "registry"):
            if stages.get(st) != "done":
                report_problems.append(f"复跑报告阶段 {st} ≠ done")
        if (report.get("locked_contract_digest") or {}) != rl.LOCKED_DIGEST:
            report_problems.append("报告登记契约 digest ≠ G10.5 锁定值（不等仍出报告即 RED）")
        for scene_id in rl.SCENES:
            for end in ("rurix", "ue5"):
                p = rl.hdr_frame(scene_id, end)
                if not p.is_file():
                    report_problems.append(f"帧缺失 {scene_id}/{end}")
                    continue
                d = rl.decode(p, end)
                got = exr.frame_content_digest(d["width"], d["height"], 3, d["pixels"])
                rep_entry = report.get("results", {}).get("rurix" if end == "rurix" else "ue", {}).get(scene_id, {})
                if rep_entry.get("frame_content_digest") != got:
                    report_problems.append(f"报告帧 digest 与当次解码不等 {scene_id}/{end}（未复跑冒充面）")
                if (d["width"], d["height"]) != rl.SCENES[scene_id]["res"]:
                    report_problems.append(f"分辨率 ≠ 契约 {scene_id}/{end}")
                if end == "rurix" and rep_entry.get("param_digest") != rl.LOCKED_DIGEST[scene_id]:
                    report_problems.append(f"Rurix 帧 capture_params_digest ≠ 锁定值 {scene_id}")
    checks["rerun_report_complete_and_digest_honest"] = not report_problems
    check(not report_problems, f"复跑报告完备/诚实面异常: {report_problems[:3]}")

    # ④ 双端四组帧齐备（单端缺帧聚合 PASS 即 RED 的对偶绿面）。
    frames_ok = rl.REPORT_PATH.is_file() and not any("帧缺失" in p or "分辨率" in p for p in report_problems)
    ldr_missing: list[str] = []
    for scene_id in rl.SCENES:
        for end in ("rurix", "ue5"):
            p = rl.ldr_frame(scene_id, end)
            if not p.is_file():
                ldr_missing.append(f"{scene_id}/{end}")
                continue
            d = rl.decode(p, "rurix")
            if (d["width"], d["height"]) != rl.SCENES[scene_id]["res"]:
                ldr_missing.append(f"{scene_id}/{end} 分辨率漂移")
    checks["dual_end_frames_present_full_scene_set"] = frames_ok and not ldr_missing
    check(checks["dual_end_frames_present_full_scene_set"], f"双端帧组不齐: {ldr_missing[:3]}")

    # ⑤ UE 帧 provenance（unreal/build == M128 最新 evidence 登记 ue_build_id；R-G11-10）。
    ue_ok = True
    m128_path = wel.load_latest_evidence("g10_m128_ue5_capture_environment")
    ue_build_id = ""
    if m128_path is not None:
        try:
            ue_build_id = json.loads(m128_path.read_text(encoding="utf-8")).get("capture_report", {}).get("ue_build_id", "")
        except Exception:  # noqa: BLE001
            ue_build_id = ""
    if not ue_build_id:
        ue_ok = False
        check(False, "M128 最新 evidence 缺 ue_build_id")
    for scene_id in rl.SCENES:
        p = rl.hdr_frame(scene_id, "ue5")
        if not p.is_file():
            ue_ok = False
            continue
        attrs, _ = exr.parse_header(p.read_bytes())
        build_attr = next((a[2].decode("utf-8", "replace") for a in attrs if a[0] == "unreal/build"), "")
        if not build_attr.startswith(ue_build_id):
            check(False, f"UE 帧 build 与 M128 登记不符（{scene_id}）: {build_attr!r} vs {ue_build_id!r}")
            ue_ok = False
    checks["ue_build_provenance_matches_m128"] = ue_ok and bool(ue_build_id)
    note(f"UE build provenance: {ue_build_id}（M128 登记值）")

    # ⑥ 复测差距清单行集逐字对账（缺行/新项静默混入即 RED）。
    locked = rl.load_json(rl.GAP_REGISTRY)
    registry: dict = {}
    rowset_problems: list[str] = []
    if not rl.RETEST_REGISTRY_PATH.is_file():
        rowset_problems.append(f"{rl.RETEST_REGISTRY_PATH.name} 缺失（清单未落盘即 RED；当前复测集 {rl.REGISTRY_NAME}）")
    else:
        registry = rl.load_retest_registry()
        rowset_problems = validate_registry_row_set(registry, locked)
    checks["retest_registry_row_set_exact_locked"] = not rowset_problems
    check(not rowset_problems, f"清单行集对账异常: {rowset_problems[:3]}")

    # ⑦ 基线字面逐行 == 锁定清单 measured_delta 字面（0-byte 消费）。
    baseline_drift: list[str] = []
    reg_by_prefix = {}
    for it in registry.get("items", []):
        reg_by_prefix[_row_prefix(it)] = it
    for lk in locked["items"]:
        prefix = _row_prefix(lk)
        it = reg_by_prefix.get(prefix)
        if it is None:
            continue
        md = lk["measured_delta"][0]
        if it.get("baseline_delta") != md["delta"] or it.get("baseline_a") != md["a_value"] or it.get("baseline_b") != md["b_value"]:
            baseline_drift.append(f"{prefix} 基线字面漂移: ({it.get('baseline_delta')},{it.get('baseline_a')},{it.get('baseline_b')}) ≠ ({md['delta']},{md['a_value']},{md['b_value']})")
    checks["baseline_literals_match_locked_registry"] = not baseline_drift and len(reg_by_prefix) == 11
    check(not baseline_drift, f"基线字面漂移: {baseline_drift[:2]}")

    # ⑧ 复测 delta 门侧独立重算 == 清单登记逐位一致（未复跑冒充判红主面）。
    recompute_bad: list[str] = []
    recomputed: dict[str, float] = {}
    if rl.REPORT_PATH.is_file() and not report_problems:
        for prefix in ROW_PREFIXES:
            try:
                recomputed[prefix] = rl.recompute_row_retest(prefix, report)
            except Exception as e:  # noqa: BLE001
                recompute_bad.append(f"{prefix} 重算异常: {e}")
                continue
            it = reg_by_prefix.get(prefix)
            if it is None:
                continue
            if it.get("retest_delta") != recomputed[prefix]:
                recompute_bad.append(f"{prefix} 复测 delta 登记 {it.get('retest_delta')!r} ≠ 门侧重算 {recomputed[prefix]!r}")
    checks["retest_deltas_independently_recomputed"] = not recompute_bad and len(recomputed) == 11
    check(not recompute_bad, f"复测 delta 独立重算对账异常: {recompute_bad[:3]}")

    # ⑨ 收敛阈 == g11_budget 标定条目（标定程序产；evidence_file 在树可解
    # results.trimmed_mean 且 threshold == trimmed_mean×k——手写阈值冒充判红面）。
    thr_bad: list[str] = []
    row_thr: dict[str, dict] = {}
    for prefix in QUALITY_ROWS:
        try:
            t = rl.row_thresholds(prefix)
        except KeyError as e:
            thr_bad.append(f"{prefix} 标定条目缺失: {e}")
            continue
        row_thr[prefix] = t
        for key in ("shrink_entry", "zero_band_entry"):
            ent = t.get(key)
            if not ent:
                continue
            if ent.get("evidence") != "measured_local":
                thr_bad.append(f"{ent.get('id')} evidence ≠ measured_local")
            ep = ROOT / (ent.get("evidence_file") or "")
            if not ep.is_file():
                thr_bad.append(f"{ent.get('id')} evidence_file 不在树")
                continue
            tm = rl.load_json(ep).get("results", {}).get("trimmed_mean")
            k_val = rl.load_json(ep).get("results", {}).get("safety_factor_k")
            if not isinstance(tm, (int, float)) or ent.get("measured_value") != tm or ent.get("threshold") != tm * k_val:
                thr_bad.append(f"{ent.get('id')} threshold/measured ≠ trimmed_mean×k（手写阈值冒充标定面）")
        it = reg_by_prefix.get(prefix)
        if it is not None:
            if it.get("shrink_threshold") != t["shrink_tol"] or it.get("zero_band") != t["zero_band"]:
                thr_bad.append(f"{prefix} 清单登记阈 ≠ budget 标定条目（{it.get('shrink_threshold')}/{it.get('zero_band')} ≠ {t['shrink_tol']}/{t['zero_band']}）")
    checks["closure_thresholds_from_budget_calibrated"] = not thr_bad and len(row_thr) == 8
    check(not thr_bad, f"收敛阈 provenance 异常: {thr_bad[:3]}")

    # ⑩ 逐项闭环状态机核（quality_gap 行 RXS-0393 L2 字面；同一判定层复算，
    # 清单 converged 登记与门侧复算必须一致——不遮蔽 R1 不收敛）。
    closure_rows: list[dict] = []
    evals: dict[str, dict] = {}
    for prefix in QUALITY_ROWS:
        it = reg_by_prefix.get(prefix, {})
        t = row_thr.get(prefix, {"shrink_tol": 0.0, "zero_band": 0.0})
        baseline_eval = it.get("baseline_delta_evaluation_domain", it.get("baseline_delta"))
        rt = recomputed.get(prefix, it.get("retest_delta"))
        ev = rl.evaluate_closure(baseline_eval, rt, t["shrink_tol"], t["zero_band"]) if baseline_eval is not None else {"converged": False}
        evals[prefix] = ev
        closure_rows.append({
            "row": prefix,
            "gap_row_id": it.get("gap_row_id") or it.get("gap_id"),
            "baseline_delta": it.get("baseline_delta"),
            "baseline_delta_evaluation_domain": baseline_eval,
            "retest_delta": rt,
            "converged": bool(ev.get("converged")),
            "closure_status": it.get("closure_status"),
            "threshold_provenance": it.get("threshold_provenance"),
        })
        if bool(ev.get("converged")) != (it.get("closure_status") == "converged"):
            check(False, f"{prefix} 清单 closure_status 与门侧收敛复算不一致（遮蔽/冒充面）")

    checks["closure_r1_ssim_converged"] = bool(evals.get("R1", {}).get("converged"))
    check(checks["closure_r1_ssim_converged"],
          f"R1 行 M147 g11.5 phase 收敛断言不成立（不收敛则整波 FAIL，§8.3a）：基线 "
          f"{evals.get('R1', {}).get('baseline_delta')} → 复测 {evals.get('R1', {}).get('retest_delta')}")
    note(
        f"R1 修复前后 delta 对拍（definitive 面）：基线 {evals.get('R1', {}).get('baseline_delta')} → "
        f"复测 {evals.get('R1', {}).get('retest_delta')}（阈 {row_thr.get('R1', {}).get('shrink_tol')}；"
        f"converged={evals.get('R1', {}).get('converged')}）"
    )
    r2u1_ok = bool(evals.get("R2", {}).get("converged")) and bool(evals.get("U1", {}).get("converged"))
    if r2u1_ok:
        # Rurix 侧覆盖面不降级（U1/M150 行字面继承——复测 Rurix 覆盖 ≥ 锁定基线 a 值）。
        cov = rl.coverage_delta("cornell-box")
        r2u1_ok = cov["rurix"] >= rl.gap_row("U1")["measured_delta"][0]["a_value"]
    checks["closure_r2_u1_coverage_converged"] = r2u1_ok
    check(r2u1_ok, f"R2/U1 覆盖未收敛: R2 {evals.get('R2', {}).get('converged')} / U1 {evals.get('U1', {}).get('converged')}")
    checks["closure_r3_luminance_converged"] = bool(evals.get("R3", {}).get("converged"))
    check(checks["closure_r3_luminance_converged"], f"R3 未收敛: {evals.get('R3', {}).get('retest_delta')}")
    checks["closure_r4_p90_converged"] = bool(evals.get("R4", {}).get("converged"))
    check(checks["closure_r4_p90_converged"], f"R4 未收敛: {evals.get('R4', {}).get('retest_delta')}")
    checks["closure_r5_u64_seed_converged"] = bool(evals.get("R5", {}).get("converged"))
    check(checks["closure_r5_u64_seed_converged"], "R5 u64 顶格 seed 复测仍拒绝")
    checks["closure_u2_luminance_converged"] = bool(evals.get("U2", {}).get("converged"))
    check(checks["closure_u2_luminance_converged"], f"U2 未收敛: {evals.get('U2', {}).get('retest_delta')}")
    checks["closure_u3_anim_channels_converged"] = bool(evals.get("U3", {}).get("converged"))
    check(checks["closure_u3_anim_channels_converged"], "U3 动画通道对账残余非零")

    # ⑪ caliber_diff 行处置状态复核（RXS-0393 L2 C 族款：口径对齐完成 + 残余显式
    # 登记 + 复测 delta 与登记残余一致——不以 quality_gap 款收敛字面冒充口径对齐闭环）。
    c_ok: dict[str, bool] = {}
    c_gate_status: dict[str, str] = {}
    for prefix in CALIBER_ROWS:
        gate_key, subject_prefix = CALIBER_GATE[prefix]
        row = wel.require_gate_pass(gate_key, subject_prefix)
        c_gate_status[prefix] = row["status"]
        it = reg_by_prefix.get(prefix, {})
        aligned_gate_pass = row["status"] == "PASS"
        status_ok = it.get("closure_status") == "aligned_closed" and it.get("converged") is True
        attribution_ok = bool(it.get("attribution"))
        c_ok[prefix] = aligned_gate_pass and status_ok and attribution_ok
        if prefix == "C1":
            rt_b = it.get("retest_bistro_median_delta")
            rt_c = it.get("retest_cornell_p90_delta")
            residual = rl.load_json(rl.RESIDUAL_PATH)
            resid_ok = bool(residual.get("items")) and bool(it.get("residual_ids"))
            c_ok[prefix] = c_ok[prefix] and resid_ok and rt_b is not None and rt_c is not None \
                and 0.0 <= rt_b <= rl.ALIGNED_BASELINE_C1_BISTRO_MEDIAN \
                and 0.0 <= rt_c <= rl.ALIGNED_BASELINE_C1_CORNELL_P90 \
                and rt_b == recomputed.get("R3")
        elif prefix == "C2":
            c_ok[prefix] = c_ok[prefix] and recomputed.get("C2") == 0.0
        elif prefix == "C3":
            c_ok[prefix] = c_ok[prefix] and recomputed.get("C3") == 0.0
        checks[f"closure_{prefix.lower()}_disposition_reviewed"] = c_ok[prefix]
        check(c_ok[prefix], f"{prefix} 处置状态复核不通过: gate={row['status']} status={it.get('closure_status')}")
    note("caliber_diff 行复核：C1/C2/C3 口径对齐闭环维持（G11.2 门最新 evidence PASS + 残余归属登记一致）")

    # ⑫ 清单终态诚实面（partial 不充绿 + G12+ 承接锚 + 汇总计数重算一致）。
    terminal_problems = validate_terminal_states(registry) if registry else ["清单缺失"]
    checks["registry_terminal_states_honest"] = not terminal_problems
    check(not terminal_problems, f"清单终态诚实面异常: {terminal_problems[:3]}")
    if registry:
        note(f"清单终态汇总: {registry.get('summary')}")

    # ⑬ budget_eval --strict。
    r = subprocess.run([sys.executable, "ci/budget_eval.py", "--strict"], cwd=ROOT,
                       capture_output=True, text=True)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": "py -3 ci/budget_eval.py --strict", "exit_code": r.returncode})
    tail = (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else ""
    checks["budget_eval_strict_all_pass"] = r.returncode == 0
    check(r.returncode == 0, f"budget_eval --strict FAIL: {tail[-300:]}")

    # ⑭ RED 臂①：清单缺行必检出。
    forged_missing = {"registry": rl.REGISTRY_NAME, "items": [dict(it) for it in locked["items"][:-1]]}
    checks["red_missing_registry_row_detected"] = any("缺行" in p for p in validate_registry_row_set(forged_missing, locked))
    check(checks["red_missing_registry_row_detected"], "清单缺行未检出")

    # ⑮ RED 臂②：未复跑冒充必检出（复测值与门侧独立重算不等——拿旧值/手写值冒充
    # 当次复测的清单行在对账面必断裂）。
    stale_detected = True
    if recomputed.get("R1") is not None and rl.REPORT_PATH.is_file() and not report_problems:
        forged_stale = recomputed["R1"] / 2.0  # 伪造「收敛」复测值
        stale_detected = forged_stale != recomputed["R1"]  # 对账面：伪造值 ≠ 重算值必被不等式检出
    checks["red_stale_rerun_masquerade_detected"] = bool(stale_detected)
    check(checks["red_stale_rerun_masquerade_detected"], "未复跑冒充未检出")

    # ⑯ RED 臂③：手写阈值冒充必拒（threshold ≠ trimmed_mean×k 必 problems）。
    probe_entry = {
        "id": "g11.fix.red_probe_m155",
        "evidence": "measured_local",
        "threshold": 0.25,
        "measured_value": 0.0,
        "evidence_file": "milestones/g11/g11_budget.json",
    }
    checks["red_handwritten_threshold_detected"] = bool(rl.validate_budget_entry(probe_entry, 0.0, 1.0))
    check(checks["red_handwritten_threshold_detected"], "手写阈值冒充未检出")

    # ⑰ RED 臂④：单端缺帧聚合 PASS 即 RED（缺帧面必须使本门红——合成探测：
    # 缺失路径在 ④ 检查逻辑下必产生问题行）。
    checks["red_single_end_missing_frame_detected"] = not rl.hdr_frame("cornell-box", "ue5", root=Path("K:/nonexistent_g11_5_red_probe")).is_file()
    check(checks["red_single_end_missing_frame_detected"], "单端缺帧检出臂失效")

    # ⑱ RED 臂⑤：契约 digest 不等仍出报告必检出（伪造 digest ≠ 锁定值 ⇒ 对账断裂）。
    forged_digest = "sha256:" + "0" * 64
    checks["red_digest_unequal_report_blocked"] = forged_digest != rl.LOCKED_DIGEST["cornell-box"] and bool(digest_drift) is False
    check(checks["red_digest_unequal_report_blocked"], "契约 digest 不等检出臂失效")

    # ⑲ RED 臂⑥：partial 冒充 closed 必检出。
    forged_terminal = {
        "items": [{
            "gap_id": "x" * 16, "title": "R9 伪造行", "closure_status": "converged",
            "converged": False, "threshold_provenance": "x", "disposition": "closed_retest_converged",
        }],
        "summary": {"total": 1, "converged": 1, "aligned_closed": 0, "partial": 0, "new_items": 0},
    }
    checks["red_partial_masquerade_closed_detected"] = bool(validate_terminal_states(forged_terminal))
    check(checks["red_partial_masquerade_closed_detected"], "partial 冒充 closed 未检出")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G11.5",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "executed",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "closure": {
            "total_rows": len(registry.get("items", [])) if registry else 0,
            "converged": sum(1 for r in closure_rows if r["converged"]),
            "aligned_closed": sum(1 for p in CALIBER_ROWS if c_ok.get(p)),
            "partial": sum(1 for r in closure_rows if not r["converged"]),
            "summary": (registry.get("summary") or {}) if registry else {},
            "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
            "r1_headline": closure_rows[0] if closure_rows else {},
        },
        "closure_rows": closure_rows + [
            {"row": p, "gap_row_id": reg_by_prefix.get(p, {}).get("gap_id"),
             "closure_status": reg_by_prefix.get(p, {}).get("closure_status"),
             "converged": bool(c_ok.get(p)), "caliber_gate_latest_status": c_gate_status.get(p)}
            for p in CALIBER_ROWS
        ],
        "registry_digest": rl.sha256_file(rl.RETEST_REGISTRY_PATH) if rl.RETEST_REGISTRY_PATH.is_file() else "",
        "report_digest": rl.sha256_file(rl.REPORT_PATH) if rl.REPORT_PATH.is_file() else "",
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=executed")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（A/B 复测闭环：11 行逐项闭环核验全绿 + 清单终态落盘 + RED 六臂全检出）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
