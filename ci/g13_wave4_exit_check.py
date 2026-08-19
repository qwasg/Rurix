#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.4 UE 对拍波）
"""G13.4 波次聚合门 g13.wave.4.exit（步骤 242；G13_CONTRACT G-G13-6/§2.2；
G13_ACCEPTANCE_MAP §1；spec/visual_comparison.md RXS-0405/RXS-0406；同构
ci/g13_wave3_exit_check.py + ci/g10_wave_exit_lib.py）。

只读汇总 G13.4 波 M-c(M169)/M-d(M170) 双门最新 evidence——UE5 超分双端对拍
（步骤 240，三方 digest 门序 + 双臂同场景同档位出图 + 端内参照 deficit/噪声谱
measured + 帧率 zero_pass_line 基线 + DLSS·超分模块归属差距登记）+ UE Lumen
GI 对照（步骤 241，三方 digest 门序 + 双臂 GI on/off 出图 + GI 能量/间接光
measured + Lumen 模块归属差距登记 + G11 GI 面 0-byte）——+ 六 facts：
① temporal 底座 0-byte（UpscaleBackend trait 签名面与 temporal 底座历史接口
   面 vs G13.0 不可变 ref 8c5dc5ee 目录级 git diff + 工作树双面机核）；
② M-c/M-d 门 RED 臂独立有效（最新 evidence red 面 checks 全真——M-c 四臂
   digest-mismatch/silent-gap/missing-frame/fps-masquerade + M-d 四臂
   digest-mismatch/silent-gap/missing-frame/gi-gate-drift）；
③ g13_budget M-c/M-d 标定六条目齐备 measured_local 零 estimated +
   budget_eval 全 PASS（P-09 禁手写）；
④ spec RXS-0405/0406 条款锚定 + conformance accept/reject 语料六件齐备 +
   trace_matrix 全锚定 PASS；
⑤ M-c/M-d 门最新 evidence 关键判据全真（三方 digest 全等/双臂帧齐备/登记表
   schema+对账/标定位级一致/UE build == M128）；
⑥ 帧率基线 zero_pass_line 登记（M-c evidence parity.fps_baseline.zero_pass_line
   == true 字面 + M-d evidence 不设绝对通过线字面——正式帧率对标锚定 G14，
   绝对画质判定归 G15，以基线冒充帧率对标即 RED）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法：
  py -3 ci/g13_wave4_exit_check.py --gate g13.wave.4.exit
  py -3 ci/g13_wave4_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g13_tsr_device_kernel_smoke as mb  # noqa: E402
import g10_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g13.wave.4.exit"
NUMERIC_STEP = 242
SUBJECT = "g13_wave4_exit"
WAVE = "G13.4"
SOURCE_REF = (
    "G13_CONTRACT G-G13-6/§2.2;G13_ACCEPTANCE_MAP §1;spec/visual_comparison.md RXS-0405/RXS-0406;"
    "M-c/M-d gate red arms independently effective;temporal base 0-byte;g13_budget M-c/M-d six entries "
    "measured_local;RXS-0405/0406 anchors + conformance corpus;both gates key criteria all true;"
    "fps baseline zero_pass_line registered"
)
SCHEMA_PATH = ROOT / "milestones/g13/g13_wave4_exit_evidence_schema.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g13.p0.m_c.ue_upscale_parity", "g13_m_c_ue_upscale_parity"),
    ("g13.p0.m_d.ue_lumen_gi_parity", "g13_m_d_ue_lumen_gi_parity"),
]

BUDGET_TOL_ENTRIES = [
    "g13.ue_upscale.ssim_deficit_delta_tol",
    "g13.ue_upscale.flip_deficit_delta_tol",
    "g13.ue_upscale.noise_hf_delta_tol",
    "g13.ue_lumen.gi_energy_rel_tol",
    "g13.ue_lumen.indirect_ssim_delta_tol",
    "g13.ue_lumen.indirect_flip_delta_tol",
]

MC_RED = [
    "device_red_digest_mismatch_detected",
    "device_red_silent_gap_detected",
    "device_red_missing_frame_detected",
    "device_red_fps_masquerade_detected",
]
MD_RED = [
    "device_red_digest_mismatch_detected",
    "device_red_silent_gap_detected",
    "device_red_missing_frame_detected",
    "device_red_gi_gate_drift_detected",
]
MC_KEY = [
    "contract_digest_three_way_equal",
    "ue_arm_frames_all_present",
    "rurix_arm_frames_all_present",
    "gap_registry_schema_valid",
    "gap_registry_reconciled",
    "calibration_dual_seed_bitexact",
    "ue_build_id_matches_m128",
]
MD_KEY = [
    "contract_digest_three_way_equal",
    "ue_arm_frames_all_present",
    "rurix_arm_frames_all_present",
    "gap_registry_schema_valid",
    "gap_registry_reconciled",
    "calibration_dual_seed_bitexact",
    "ue_build_id_matches_m128",
    "g11_gi_surface_0byte",
    "g10_5_default_path_bitexact",
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def collect_facts() -> list[dict]:
    facts: list[dict] = []

    ok, msg = mb.temporal_base_0byte()
    facts.append(_fact("temporal_base_0byte", ok, msg))

    red_bad: list[str] = []
    red_total = 0
    docs: dict[str, dict] = {}
    for key, prefix in REQUIRED_GATES:
        path = wel.load_latest_evidence(prefix)
        if path is None:
            red_bad.append(f"{prefix} 缺最新 evidence")
            continue
        doc = wel.load_json(path)
        docs[prefix] = doc
        want = MC_RED if "m_c" in prefix else MD_RED
        red_checks = {k: v for k, v in (doc.get("checks") or {}).items() if k in want}
        red_total += len(red_checks)
        if len(red_checks) != len(want) or any(v is not True for v in red_checks.values()):
            red_bad.append(f"{prefix} red 面 checks 缺失或非真")
    facts.append(_fact(
        "m_c_m_d_red_arms_independently_effective",
        not red_bad,
        f"M-c/M-d 门最新 evidence red 面 checks 全真（共 {red_total} 臂独立有效）"
        if not red_bad else "; ".join(red_bad[:3]),
    ))

    bud_bad: list[str] = []
    budget = mb.load_g13_budget()
    if budget is None:
        bud_bad.append("g13_budget.json 缺失")
    else:
        for eid in BUDGET_TOL_ENTRIES:
            e = mb.budget_entry(budget, eid)
            if e is None:
                bud_bad.append(f"缺条目 {eid}")
            elif e.get("evidence") != "measured_local":
                bud_bad.append(f"{eid} 非 measured_local")
    import subprocess

    r = subprocess.run(["py", "-3", "ci/budget_eval.py"], cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        bud_bad.append(f"budget_eval rc={r.returncode}")
    facts.append(_fact(
        "budget_calibration_entries_measured",
        not bud_bad,
        f"g13_budget M-c/M-d 六条目齐备 measured_local 零 estimated + budget_eval 全 PASS（P-09；共 {len(BUDGET_TOL_ENTRIES)} 条目）"
        if not bud_bad else "; ".join(bud_bad[:3]),
    ))

    spec_bad: list[str] = []
    spec = ROOT / "spec" / "visual_comparison.md"
    spec_text = spec.read_text(encoding="utf-8") if spec.is_file() else ""
    for rxs in ("RXS-0405", "RXS-0406"):
        if f"### {rxs} " not in spec_text:
            spec_bad.append(f"spec 缺 {rxs} 条款头")
    corpus = [
        "conformance/visual_comparison/accept/ue_upscale_parity_contract_minimal.rx",
        "conformance/visual_comparison/accept/ue_lumen_gi_parity_contract_minimal.rx",
        "conformance/visual_comparison/reject/upscale_parity_digest_mismatch_report.rx",
        "conformance/visual_comparison/reject/upscale_fps_baseline_masquerade.rx",
        "conformance/visual_comparison/reject/lumen_parity_digest_mismatch_report.rx",
        "conformance/visual_comparison/reject/lumen_gap_silent.rx",
    ]
    for rel in corpus:
        p = ROOT / rel
        if not p.is_file():
            spec_bad.append(f"缺语料 {p.name}")
    r = subprocess.run(["py", "-3", "ci/trace_matrix.py", "--check"], cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0 or "PASS" not in (r.stdout + r.stderr):
        spec_bad.append("trace_matrix 非 PASS")
    facts.append(_fact(
        "spec_clauses_and_corpus_anchored",
        not spec_bad,
        "RXS-0405/0406 条款锚定 + conformance 六件齐备 + trace_matrix 全锚定 PASS"
        if not spec_bad else "; ".join(spec_bad[:3]),
    ))

    key_bad: list[str] = []
    for prefix, want in (("g13_m_c_ue_upscale_parity", MC_KEY), ("g13_m_d_ue_lumen_gi_parity", MD_KEY)):
        doc = docs.get(prefix)
        if doc is None:
            key_bad.append(f"{prefix} 缺 evidence")
            continue
        for k in want:
            if (doc.get("checks") or {}).get(k) is not True:
                key_bad.append(f"{prefix}.checks.{k} 非真")
    facts.append(_fact(
        "both_gates_key_criteria_true",
        not key_bad,
        "M-c/M-d 门最新 evidence 关键判据全真（三方 digest/双臂帧齐/登记表对账/标定位级/ue_build/0-byte 面）"
        if not key_bad else "; ".join(key_bad[:3]),
    ))

    zpl_bad: list[str] = []
    mc = docs.get("g13_m_c_ue_upscale_parity")
    if mc is None:
        zpl_bad.append("M-c 缺 evidence")
    else:
        fps = (mc.get("parity") or {}).get("fps_baseline") or {}
        if fps.get("zero_pass_line") is not True:
            zpl_bad.append("M-c fps_baseline.zero_pass_line 非 true 字面")
        if not fps.get("cells"):
            zpl_bad.append("M-c fps_baseline.cells 空")
    md = docs.get("g13_m_d_ue_lumen_gi_parity")
    if md is None:
        zpl_bad.append("M-d 缺 evidence")
    else:
        evo = str((md.get("production") or {}).get("evolution_register") or "")
        if "不设绝对" not in evo:
            zpl_bad.append("M-d evolution_register 缺不设绝对通过线字面")
    facts.append(_fact(
        "fps_baseline_zero_pass_line_registered",
        not zpl_bad,
        "帧率基线 zero_pass_line 登记（M-c fps_baseline 字面 + M-d 不设绝对通过线字面；锚定 G14/G15）"
        if not zpl_bad else "; ".join(zpl_bad[:3]),
    ))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_facts() if evidence_dir is None else []
    if evidence_dir is not None:
        extras = [
            _fact("temporal_base_0byte", False, "selftest 空目录"),
            _fact("m_c_m_d_red_arms_independently_effective", False, "selftest 空目录"),
            _fact("budget_calibration_entries_measured", False, "selftest 空目录"),
            _fact("spec_clauses_and_corpus_anchored", False, "selftest 空目录"),
            _fact("both_gates_key_criteria_true", False, "selftest 空目录"),
            _fact("fps_baseline_zero_pass_line_registered", False, "selftest 空目录"),
        ]
    notes_parts = [
        "implemented: G13.4 M-c(M169) UE upscale parity gate (step 240) + M-d(M170) UE Lumen GI parity gate (step 241)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: temporal base 0-byte + red arms + g13_budget six entries measured + RXS-0405/0406 anchors + both gates key criteria + fps zero_pass_line",
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
    """① 缺双门 evidence → 红；② 真树聚合 VERDICT == 子门实测态（遮蔽即自检红）。"""
    print("[selftest] 负样本：空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g13_wave4_selftest_") as td:
        code = run_gate(evidence_dir=Path(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 真树一致性：聚合 VERDICT == 子门实测态（不遮蔽机核）")
    rows = [wel.require_gate_pass(key, prefix) for key, prefix in REQUIRED_GATES]
    extras = collect_facts()
    expected_pass = all(r["status"] == "PASS" for r in rows) and all(f["status"] == "PASS" for f in extras)
    code = run_gate(evidence_dir=None)
    if (code == 0) != expected_pass:
        print(
            f"[selftest] FAIL: 聚合 VERDICT 与子门实测态不一致——expected_pass={expected_pass} exit={code}",
            file=sys.stderr,
        )
        return 1
    print(f"[selftest] PASS: 真树聚合 VERDICT={'PASS' if code == 0 else 'FAIL'} == 子门实测态（不遮蔽）")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G13.4 wave4.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
