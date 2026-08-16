#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.5 波）
"""G11.5 波次聚合门 g11.wave.5.exit（步骤 213；milestones/g11/CI_GATES.md §5；
G11_CONTRACT G-G11-7；同构 ci/g11_wave4_exit_check.py + ci/g11_wave_exit_lib.py）。

只读汇总 G11.5 波两门最新 evidence——M155 A/B 复测闭环（步骤 211）/ M156
修复回归门（步骤 212）——+ 六 facts：
① 契约 digest 0-byte（双场景当次重算 == G10.5 锁定值 + 联合值）；
② 两门 RED 臂独立有效（最新 evidence 各含 red_* checks 且全真）；
③ 复测差距清单终态诚实面（11 行闭集对账 + partial 行承接锚非空 + 汇总计数
   重算一致——partial/未收敛行如实登记不充绿）；
④ M156 真跑抽检面（最新 evidence spot_rerun_* 八检全真 + 新鲜度机核在档）；
⑤ 回归面（G10 14 门 + G9 34 门 + G11 已绿门最新 evidence 全绿只读汇总 +
   默认面帧 digest 逐位 parity——g11_5/parity 无旗标复跑帧 == G10.5 锁定 digest）；
⑥ M147 g11.5 phase verdict 诚实登记两态机核（g11.5 phase evidence 在档且
   verdict ∈ {converged, not_converged} 显式登记；**converged ⇔ M155 最新
   evidence PASS**——不一致即遮蔽/冒充判红；not_converged 时本 fact 绿但
   M155 required 行 FAIL ⇒ 聚合 VERDICT=FAIL 如实〔不收敛则整波 FAIL，
   契约 §8.3a 不弱化声明〕）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法：
  py -3 ci/g11_wave5_exit_check.py --gate g11.wave.5.exit
  py -3 ci/g11_wave5_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import g11_5_retest_lib as rl  # noqa: E402
import g11_ab_retest_closure_smoke as m155  # noqa: E402  # 清单校验器单一事实源
import g11_wave_exit_lib as wel  # noqa: E402
from g11_wave3_exit_check import G10_KEYS, G9_KEYS  # noqa: E402  # 48 门清单单一事实源

ROOT = wel.ROOT
GATE_KEY = "g11.wave.5.exit"
NUMERIC_STEP = 213
SUBJECT = "g11_wave5_exit"
WAVE = "G11.5"
SOURCE_REF = (
    "milestones/g11/CI_GATES.md §5;G11_CONTRACT G-G11-7 + §8.3a;G11_ACCEPTANCE_MAP §1;"
    "two gates red arms independently effective;retest registry terminal states honest;"
    "M156 spot reruns green;regression summary green (48 + G11 green gates + parity);"
    "M147 g11.5 phase verdict honest two-state"
)
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_wave5_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g11.p0.m155.ab_retest_closure", "g11_m155_ab_retest_closure"),
    ("g11.p0.m156.regression_guard", "g11_m156_regression_guard"),
]
RED_ARM_GATES = REQUIRED_GATES

G11_GREEN_KEYS = [
    ("g11.p0.m144.caliber_c1_indoor_luminance", "g11_m144_caliber_c1_indoor_luminance"),
    ("g11.p0.m145.caliber_c2_exposure_chain", "g11_m145_caliber_c2_exposure_chain"),
    ("g11.p0.m146.caliber_c3_exr_bit_depth", "g11_m146_caliber_c3_exr_bit_depth"),
    ("g11.p0.m148.fix_r2_geometry_normals", "g11_m148_fix_r2_geometry_normals"),
    ("g11.p0.m149.fix_r5_json_u64_seed", "g11_m149_fix_r5_json_u64_seed"),
    ("g11.p0.m150.fix_u1_cornell_shell_radiance", "g11_m150_fix_u1_cornell_shell_radiance"),
    ("g11.p0.m151.fix_u2_bistro_texture_dds", "g11_m151_fix_u2_bistro_texture_dds"),
    ("g11.p0.m152.fix_u3_bistro_animation", "g11_m152_fix_u3_bistro_animation"),
    ("g11.p0.m153.fix_r3_light_subset", "g11_m153_fix_r3_light_subset"),
    ("g11.p0.m154.fix_r4_gi_multibounce_world_cache", "g11_m154_fix_r4_gi_multibounce_world_cache"),
    ("g11.p1.m157.hdr_flip_calibration", "g11_m157_hdr_flip_calibration"),
    ("g11.wave.2.exit", "g11_wave2_exit"),
    ("g11.wave.3.exit", "g11_wave3_exit"),
    ("g11.wave.4.exit", "g11_wave4_exit"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def m147_g11_5_verdict_fact(m155_latest_status: str | None) -> dict:
    """fact⑥ 两态机核（G11.5 波：B 态断言面——契约 §8.3a 不弱化声明）。"""
    g115 = None
    for p in sorted(wel.EVIDENCE_DIR.glob("g11_m147_fix_r1_material_subset_*.json")):
        doc = wel.load_json(p)
        if doc.get("phase") == "g11.5":
            g115 = (p, doc)
    if g115 is None:
        return _fact("m147_g11_5_verdict_honest_two_state", False, "M147 g11.5 phase evidence 缺档")
    doc = g115[1]
    verdict = (doc.get("closure") or {}).get("verdict")
    converged = (doc.get("closure") or {}).get("converged")
    # 一致性：converged=true ⇔ M155 最新 PASS（不一致 = 遮蔽/冒充判红）。
    consistent = (verdict in ("converged", "not_converged")) and (
        (verdict == "converged") == (m155_latest_status == "PASS")
    ) and (bool(converged) == (verdict == "converged"))
    detail = (
        f"M147 g11.5 phase verdict={verdict}（{g115[0].name}；converged={converged}）⇔ M155 最新 "
        f"{m155_latest_status}——两态一致；not_converged 时本 fact 绿但 M155 required 行 FAIL ⇒ "
        "聚合 VERDICT=FAIL 如实（不收敛则整波 FAIL，§8.3a）"
        if consistent else
        f"两态不一致（遮蔽/冒充面）: verdict={verdict} converged={converged} M155={m155_latest_status}"
    )
    return _fact("m147_g11_5_verdict_honest_two_state", consistent, detail)


def collect_extra_facts(m155_row: dict) -> list[dict]:
    facts: list[dict] = []

    # ① 契约 digest 0-byte（双场景当次重算 + 联合值）。
    drift = []
    for s in rl.SCENES:
        try:
            got = rl.contract_digest_rust(s)
        except Exception as e:  # noqa: BLE001
            got = f"<error {e}>"
        if got != rl.LOCKED_DIGEST[s]:
            drift.append(f"{s}: {got} ≠ {rl.LOCKED_DIGEST[s]}")
    facts.append(_fact(
        "contract_digest_locked_unchanged",
        not drift,
        "双场景契约 digest 当次重算 == G10.5 锁定值（cornell 80305791…/bistro ad45951b…，联合 64fd54df…）"
        if not drift else "; ".join(drift[:2]),
    ))

    # ② 两门 RED 臂独立有效。
    red_bad: list[str] = []
    red_total = 0
    for _key, prefix in RED_ARM_GATES:
        path = wel.load_latest_evidence(prefix)
        if path is None:
            red_bad.append(f"{prefix} 缺最新 evidence")
            continue
        doc = wel.load_json(path)
        red_checks = {k: v for k, v in (doc.get("checks") or {}).items() if k.startswith("red_")}
        red_total += len(red_checks)
        if not red_checks or any(v is not True for v in red_checks.values()):
            red_bad.append(f"{prefix} red_* checks 缺失或非真")
    facts.append(_fact(
        "two_gates_red_arms_independently_effective",
        not red_bad,
        f"两门最新 evidence 各含 red_* checks 且全真（共 {red_total} 臂独立有效）"
        if not red_bad else "; ".join(red_bad[:3]),
    ))

    # ③ 复测差距清单终态诚实面（行集对账 + 终态诚实校验器复用 M155 单一事实源）。
    reg_bad: list[str] = []
    if not rl.RETEST_REGISTRY_PATH.is_file():
        reg_bad.append("复测差距清单缺档")
    else:
        doc = rl.load_retest_registry()
        locked = rl.load_json(rl.GAP_REGISTRY)
        reg_bad += m155.validate_registry_row_set(doc, locked)
        reg_bad += m155.validate_terminal_states(doc)
        partial_rows = (doc.get("summary") or {}).get("partial_rows") or []
    facts.append(_fact(
        "retest_registry_terminal_states_honest",
        not reg_bad,
        (
            f"复测差距清单 11 行闭集对账 + 终态诚实面全绿；partial 行如实登记（{partial_rows}，"
            "带 G12+/G11.6 承接锚不充绿）" if not reg_bad else "; ".join(reg_bad[:3])
        ),
    ))

    # ④ M156 真跑抽检面（最新 evidence spot_rerun_* 全真 + status pass + 新鲜度在档）。
    m156_bad: list[str] = []
    m156_path = wel.load_latest_evidence("g11_m156_regression_guard")
    if m156_path is None:
        m156_bad.append("M156 缺最新 evidence")
    else:
        doc = wel.load_json(m156_path)
        if doc.get("status") != "pass":
            m156_bad.append(f"M156 最新 evidence status={doc.get('status')!r}")
        spot = {k: v for k, v in (doc.get("checks") or {}).items() if k.startswith("spot_rerun_")}
        if len(spot) != 8 or any(v is not True for v in spot.values()):
            m156_bad.append(f"spot_rerun_* 非全真: {spot}")
    facts.append(_fact(
        "m156_spot_reruns_green",
        not m156_bad,
        "M156 最新 evidence PASS + 真跑抽检八面全绿（M130 双 phase/M139/M140/M141/G9 M96/M94/M110）"
        if not m156_bad else "; ".join(m156_bad[:3]),
    ))

    # ⑤ 回归面（48 门 + G11 已绿门最新 evidence 全绿只读汇总 + parity 逐位）。
    reg_bad2: list[str] = []
    for key, prefix in G10_KEYS + G9_KEYS + G11_GREEN_KEYS:
        row = wel.require_gate_pass(key, prefix)
        if row["status"] != "PASS":
            reg_bad2.append(f"{prefix}: {row['detail'][:60]}")
    parity_bad: list[str] = []
    g105_parity = {
        "cornell-box": "sha256:c2000ebfbe90359d55e668f8af3b7df24d64c3f72e637904f614821b7ad0d727",
        "bistro-interior": "sha256:8519cc67c917e7b8c2c5a9bb5633ea5ee9e72deb8cf63b3b187b0d3ac5bb9935",
    }
    import g10_exr_lib as exr  # noqa: E402
    for scene_id, want in g105_parity.items():
        pf = rl.FRAMES_G11_5 / "parity" / f"{scene_id}.exr"
        if not pf.is_file():
            parity_bad.append(f"{scene_id} parity 帧缺失")
            continue
        d = exr.decode_exr(pf.read_bytes(), "rurix")
        dg = exr.frame_content_digest(d["width"], d["height"], 3, d["pixels"])
        if dg != want:
            parity_bad.append(f"{scene_id} 默认面帧 digest 漂移")
    facts.append(_fact(
        "regression_summary_green_with_parity",
        not reg_bad2 and not parity_bad,
        "G10 14 门 + G9 34 门 + G11 已绿门最新 evidence 全绿只读汇总 + 默认面帧 digest 逐位 parity（双场景 == G10.5 锁定值）"
        if not reg_bad2 and not parity_bad else "; ".join((reg_bad2 + parity_bad)[:3]),
    ))

    # ⑥ M147 g11.5 phase verdict 诚实登记两态机核。
    facts.append(m147_g11_5_verdict_fact(m155_row.get("status")))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    m155_row = rows[0] if rows else {"status": None}
    extras = collect_extra_facts(m155_row)
    notes_parts = [
        "implemented: two G11.5 gates (P0 M155 ab_retest_closure step 211 / "
        "P0 M156 regression_guard step 212)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: contract digest 0-byte + red arms + retest registry terminal states honest + "
        "M156 spot reruns + regression summary (48 + G11 green + parity) + M147 g11.5 verdict two-state",
        "aggregate PASS does not mask any child FAIL/SKIP/DEV_ENV_DEGRADE",
    ]
    code, _path = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=rows,
        extra_facts=extras,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="; ".join(notes_parts),
        host_section_pass=True,
    )
    return code


def run_selftest() -> int:
    """① 缺两门 evidence → 红；② 真树聚合 VERDICT 必须 == 子门实测态（遮蔽即自检红）。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g11_wave5_selftest_") as td:
        code = run_gate(evidence_dir=Path(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 真树一致性:聚合 VERDICT == 子门实测态（不遮蔽机核）")
    rows = [wel.require_gate_pass(key, prefix) for key, prefix in REQUIRED_GATES]
    extras = collect_extra_facts(rows[0])
    expected_pass = all(r["status"] == "PASS" for r in rows) and all(f["status"] == "PASS" for f in extras)
    code = run_gate(evidence_dir=None)
    if (code == 0) != expected_pass:
        print(
            f"[selftest] FAIL: 聚合 VERDICT 与子门实测态不一致（遮蔽/代绿面）——expected_pass={expected_pass} exit={code}",
            file=sys.stderr,
        )
        return 1
    print(f"[selftest] PASS: 真树聚合 VERDICT={'PASS' if code == 0 else 'FAIL'} == 子门实测态（不遮蔽）")
    # fact⑥ 红臂单元：converged 而 M155 FAIL（或反之）→ 两态不一致必检出。
    forged = m147_g11_5_verdict_fact("PASS" if rows[0]["status"] != "PASS" else "FAIL")
    if forged["status"] != "FAIL":
        print("[selftest] FAIL: fact⑥ 两态不一致注入未检出", file=sys.stderr)
        return 1
    print("[selftest] PASS: fact⑥ 两态注入 → 红")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G11.5 wave5.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
