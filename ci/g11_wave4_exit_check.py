#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.4 波）
"""G11.4 波次聚合门 g11.wave.4.exit（步骤 210；milestones/g11/CI_GATES.md §5；
G11_CONTRACT G-G11-6；同构 ci/g11_wave3_exit_check.py + ci/g11_wave_exit_lib.py）。

只读汇总 G11.4 波两门最新 evidence——M153 R3 灯种子集（步骤 208）/ M154 R4
多反弹 GI + 世界级辐射缓存（步骤 209）——+ 六 facts：
① 契约 digest 0-byte（双场景当次重算 == G10.5 锁定值 + 联合值）；
② 两门 RED 臂独立有效（最新 evidence 各含 red_* checks 且全真）；
③ 标定值入 g11_budget 且 provenance 齐备（三条 g11.fix.r3/r4 条目
   measured_local + evidence_file 在树可解 results.trimmed_mean +
   threshold == trimmed_mean × k，P-09）；
④ spec-first 面（RXS-0394/0395/0396 条款头在树 + RXS-0396 世界级翻转修订行
   字面 + RXS-0360 既有字面 0-byte + RFC-0028 Agent Approved 字面）；
⑤ 回归前置自检（G10 14 门 + G9 34 门最新 evidence 全绿只读汇总 + 默认面帧
   digest 逐位 parity——g11_4/parity 无旗标复跑帧 == G10.5 锁定 digest）；
⑥ M96 门序（g9.p0.m96 最新 evidence PASS，RXS-0357 L6）+ R1 的 g11.5 phase
   耦合面复核实测登记（g11_4_rerun_report closure_faces.r1_coupling_recheck
   非空——为 M155 收敛断言备料，不冒充收敛断言）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法：
  py -3 ci/g11_wave4_exit_check.py --gate g11.wave.4.exit
  py -3 ci/g11_wave4_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import g11_4_fix_lib as gl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402
from g11_wave3_exit_check import G10_KEYS, G9_KEYS  # noqa: E402  # 48 门清单单一事实源

ROOT = wel.ROOT
GATE_KEY = "g11.wave.4.exit"
NUMERIC_STEP = 210
SUBJECT = "g11_wave4_exit"
WAVE = "G11.4"
SOURCE_REF = (
    "milestones/g11/CI_GATES.md §5;G11_CONTRACT G-G11-6;G11_ACCEPTANCE_MAP §1;"
    "two gates red arms independently effective;calibrated thresholds in g11_budget "
    "with provenance (P-09);spec-first RXS-0394~0396 + RXS-0360 flip revision line;"
    "regression precheck green;M96 gate ordering (RXS-0357 L6)"
)
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_wave4_exit_evidence_schema.json"
BUDGET_PATH = ROOT / "milestones" / "g11" / "g11_budget.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g11.p0.m153.fix_r3_light_subset", "g11_m153_fix_r3_light_subset"),
    ("g11.p0.m154.fix_r4_gi_multibounce_world_cache", "g11_m154_fix_r4_gi_multibounce_world_cache"),
]
RED_ARM_GATES = REQUIRED_GATES

# 标定值入 budget 的三条条目闭集（id → k）。
CALIB_BUDGET_ENTRIES: list[tuple[str, float]] = [
    ("g11.fix.r3_luminance_shrink_tol", 1.0),
    ("g11.fix.r4_p90_shrink_tol", 1.0),
    ("g11.fix.r4_farfield_energy_min", 0.5),
]

_RXS_HEAD_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_extra_facts() -> list[dict]:
    facts: list[dict] = []

    # ① 契约 digest 0-byte（双场景当次重算 + 联合值）。
    drift = []
    for s in gl.SCENES:
        try:
            got = gl.contract_digest_rust(s)
        except Exception as e:  # noqa: BLE001
            got = f"<error {e}>"
        if got != gl.LOCKED_DIGEST[s]:
            drift.append(f"{s}: {got} ≠ {gl.LOCKED_DIGEST[s]}")
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

    # ③ 标定值入 g11_budget 且 provenance 齐备（P-09）。
    budget_bad: list[str] = []
    try:
        budget = wel.load_json(BUDGET_PATH)
    except (OSError, ValueError) as e:
        budget = {"entries": []}
        budget_bad.append(f"budget 不可读: {e}")
    entries = {e.get("id"): e for e in budget.get("entries", [])}
    for eid, k in CALIB_BUDGET_ENTRIES:
        entry = entries.get(eid)
        if entry is None:
            budget_bad.append(f"{eid} 缺条目")
            continue
        if entry.get("evidence") != "measured_local":
            budget_bad.append(f"{eid} evidence={entry.get('evidence')!r}")
        ep = ROOT / (entry.get("evidence_file") or "")
        if not ep.is_file():
            budget_bad.append(f"{eid} evidence_file 不在树")
            continue
        try:
            tm = wel.load_json(ep).get("results", {}).get("trimmed_mean")
        except (OSError, ValueError):
            tm = None
        if not isinstance(tm, (int, float)):
            budget_bad.append(f"{eid} evidence 缺 results.trimmed_mean")
            continue
        if entry.get("measured_value") != tm or entry.get("threshold") != tm * k:
            budget_bad.append(f"{eid} threshold/measured ≠ trimmed_mean×k")
    facts.append(_fact(
        "calibrated_thresholds_in_budget_with_provenance",
        not budget_bad,
        "g11_budget.json 三条 g11.fix.r3/r4 标定条目 measured_local + evidence_file 在树可解 "
        "trimmed_mean + threshold == trimmed_mean × k（P-09）"
        if not budget_bad else "; ".join(budget_bad[:3]),
    ))

    # ④ spec-first 面（条款头 + 翻转修订行 + RXS-0360 0-byte + RFC-0028 Approved）。
    spec_text = gl.SPEC_GI.read_text(encoding="utf-8") if gl.SPEC_GI.is_file() else ""
    heads = {int(m) for m in _RXS_HEAD_RE.findall(spec_text)}
    missing = sorted({394, 395, 396} - heads)
    flat = re.sub(r"\s+", "", spec_text)
    flip_ok = "世界级承接落地（G11.4M154）" in flat
    r360_kept = "世界级clipmap证据不足——未measured举证，登记not-triggered不充绿" in flat
    ok_rfc, detail_rfc = wel.rfc_agent_approved(gl.RFC0028)
    facts.append(_fact(
        "spec_first_rxs_and_flip_revision",
        not missing and flip_ok and r360_kept and ok_rfc,
        "spec/global_illumination.md RXS-0394/0395/0396 条款头在树 + RXS-0396 世界级翻转修订行字面 + "
        f"RXS-0360 既有字面 0-byte + RFC-0028 Agent Approved（{detail_rfc[:40]}…）"
        if not missing and flip_ok and r360_kept and ok_rfc
        else f"缺条款头 {missing} / flip {flip_ok} / rxs0360 0-byte {r360_kept} / rfc {detail_rfc[:60]}",
    ))

    # ⑤ 回归前置自检（48 门最新 evidence 全绿只读汇总 + 默认面帧 digest 逐位 parity）。
    reg_bad: list[str] = []
    for key, prefix in G10_KEYS + G9_KEYS:
        row = wel.require_gate_pass(key, prefix)
        if row["status"] != "PASS":
            reg_bad.append(f"{prefix}: {row['detail'][:60]}")
    parity_bad: list[str] = []
    g105_parity = {
        "cornell-box": "sha256:c2000ebfbe90359d55e668f8af3b7df24d64c3f72e637904f614821b7ad0d727",
        "bistro-interior": "sha256:8519cc67c917e7b8c2c5a9bb5633ea5ee9e72deb8cf63b3b187b0d3ac5bb9935",
    }
    for scene_id, want in g105_parity.items():
        pf = gl.FRAMES_G11_4 / "parity" / f"{scene_id}.exr"
        if not pf.is_file():
            parity_bad.append(f"{scene_id} parity 帧缺失")
            continue
        d = gl.exr.decode_exr(pf.read_bytes(), "rurix")
        dg = gl.exr.frame_content_digest(d["width"], d["height"], 3, d["pixels"])
        if dg != want:
            parity_bad.append(f"{scene_id} 默认面帧 digest 漂移")
    facts.append(_fact(
        "regression_guard_precheck",
        not reg_bad and not parity_bad,
        "G10 14 门 + G9 34 门最新 evidence 全绿只读汇总 + 默认面帧 digest 逐位 parity（双场景 == G10.5 锁定值）"
        if not reg_bad and not parity_bad else "; ".join((reg_bad + parity_bad)[:3]),
    ))

    # ⑥ M96 门序 + R1 耦合面复核登记（M155 备料，不冒充收敛断言）。
    m96_row = wel.require_gate_pass("g9.p0.m96.path_tracer_reference", "g9_m96_path_tracer_reference")
    r1_note: dict = {}
    try:
        r1_note = gl.load_report()["results"]["metrics"]["closure_faces"]["r1_coupling_recheck"]
    except (OSError, ValueError, KeyError):
        r1_note = {}
    r1_ok = bool(r1_note) and r1_note.get("retest_delta_m154_face") is not None
    facts.append(_fact(
        "m96_ordering_and_r1_coupling_recheck",
        m96_row["status"] == "PASS" and r1_ok,
        (
            f"M96 最新 evidence PASS（RXS-0357 L6 门序）；R1 耦合面复核：G11.3 复测 0.9903435577002249 → "
            f"G11.4 m154 面 {r1_note.get('retest_delta_m154_face')}（m153 面 {r1_note.get('retest_delta_m153_face')}）"
            "——实测登记为 M155 收敛断言备料，不冒充收敛"
        )
        if m96_row["status"] == "PASS" and r1_ok
        else f"M96: {m96_row['detail'][:60]} / r1_coupling_recheck 缺失或缺字段",
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_extra_facts()
    notes_parts = [
        "implemented: two G11.4 gates (P0 M153 fix_r3_light_subset step 208 / "
        "P0 M154 fix_r4_gi_multibounce_world_cache step 209)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: contract digest 0-byte + red arms + calibrated thresholds (P-09) + "
        "spec-first RXS-0394~0396 + RXS-0360 flip revision + regression precheck (48 gates + parity) + "
        "M96 ordering + R1 coupling recheck registered (not a convergence claim)",
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
    """① 缺两门 evidence → 红；② 真树两门绿 + 事实核验 → 绿。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g11_wave4_selftest_") as td:
        code = run_gate(evidence_dir=Path(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 正样本:仓库最新两门 evidence")
    import time

    time.sleep(1.1)
    code = run_gate(evidence_dir=None)
    if code != 0:
        print("[selftest] FAIL: 真树聚合未绿（前置两门/事实核验未满足）", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真树聚合绿")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G11.4 wave4.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
