#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波；G11.3 收口 M147 双 phase 两态校准）
"""G11.3 波次聚合门 g11.wave.3.exit（步骤 207；milestones/g11/CI_GATES.md §5；
G11_CONTRACT G-G11-5；同构 ci/g11_wave2_exit_check.py + ci/g11_wave_exit_lib.py）。

只读汇总 G11.3 波六门最新 evidence——M147 R1 材质（步骤 201）/ M148 R2 法线
（步骤 202）/ M149 R5 u64 seed（步骤 203）/ M150 U1 壳体（步骤 204）/ M151 U2
DDS 纹理（步骤 205）/ M152 U3 动画（步骤 206）——+ 六 facts：
① 契约 digest 0-byte（双场景当次重算 == G10.5 锁定值 + 联合值）；
② 六门 RED 臂独立有效（最新 evidence 各含 red_* checks 且全真）；
③ 标定值入 g11_budget 且 provenance 齐备（八条 g11.fix.* 条目 measured_local +
   evidence_file 在树可解 results.trimmed_mean + threshold == trimmed_mean × k，P-09）；
④ 资产 provenance 齐备（DDS 转码 manifest 144 条目 + cornell 语料 M131 登记
   digest 复算 0-byte + 派生产物目录零未登记混入）；
⑤ 回归前置自检（G10 14 门 + G9 34 门最新 evidence 全绿只读汇总 + 默认面帧
   digest 逐位 parity——g11_3/parity 无旗标复跑帧 == G10.5 锁定 digest）；
⑥ M147 双 phase 纪律两态口径（G11.3 收口校准，沿 G10.8a wave2 fact④ 两态先例；
   判据语义 0-byte——g11.3 phase 绿不替 g11.5 收敛断言充绿）。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法：
  py -3 ci/g11_wave3_exit_check.py --gate g11.wave.3.exit
  py -3 ci/g11_wave3_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import g10_corpus_lib as corpus_lib  # noqa: E402
import g11_3_fix_lib as fl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g11.wave.3.exit"
NUMERIC_STEP = 207
SUBJECT = "g11_wave3_exit"
WAVE = "G11.3"
SOURCE_REF = (
    "milestones/g11/CI_GATES.md §5;G11_CONTRACT G-G11-5;G11_ACCEPTANCE_MAP §1;"
    "six gates red arms independently effective;calibrated thresholds in g11_budget "
    "with provenance (P-09);asset provenance intact;regression precheck green"
)
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_wave3_exit_evidence_schema.json"
BUDGET_PATH = ROOT / "milestones" / "g11" / "g11_budget.json"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g11.p0.m147.fix_r1_material_subset", "g11_m147_fix_r1_material_subset"),
    ("g11.p0.m148.fix_r2_geometry_normals", "g11_m148_fix_r2_geometry_normals"),
    ("g11.p0.m149.fix_r5_json_u64_seed", "g11_m149_fix_r5_json_u64_seed"),
    ("g11.p0.m150.fix_u1_cornell_shell_radiance", "g11_m150_fix_u1_cornell_shell_radiance"),
    ("g11.p0.m151.fix_u2_bistro_texture_dds", "g11_m151_fix_u2_bistro_texture_dds"),
    ("g11.p0.m152.fix_u3_bistro_animation", "g11_m152_fix_u3_bistro_animation"),
]
RED_ARM_GATES = REQUIRED_GATES

# 标定值入 budget 的八条条目闭集（id → k）。
CALIB_BUDGET_ENTRIES: list[tuple[str, float]] = [
    ("g11.fix.r1_ssim_shrink_tol", 1.0),
    ("g11.fix.r2_coverage_shrink_tol", 1.0),
    ("g11.fix.r2_coverage_zero_band", 2.0),
    ("g11.fix.r5_u64_seed_shrink_tol", 1.0),
    ("g11.fix.u1_coverage_shrink_tol", 1.0),
    ("g11.fix.u1_coverage_zero_band", 2.0),
    ("g11.fix.u2_luminance_shrink_tol", 1.0),
    ("g11.fix.u3_anim_channels_shrink_tol", 1.0),
]

G10_KEYS = [
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

G9_KEYS = [
    ("g9.p0.m90.cluster_dag_deepening", "g9_m90_cluster_dag_deepening"),
    ("g9.p0.m91.page_format_v2_abi", "g9_m91_page_format_v2_abi"),
    ("g9.p0.m102.dgc_abstraction", "g9_m102_dgc_abstraction"),
    ("g9.p0.m103.descriptor_global_table", "g9_m103_descriptor_global_table"),
    ("g9.p0.m104.accesskind_indirect_edge", "g9_m104_accesskind_indirect_edge"),
    ("g9.p0.m121.physics_particle_view", "g9_m121_physics_particle_view"),
    ("g9.p0.m122.gameplay_field", "g9_m122_gameplay_field"),
    ("g9.p0.m93.visible_cluster_set", "g9_m93_visible_cluster_set"),
    ("g9.p0.m94.clas_rt_convergence", "g9_m94_clas_rt_convergence"),
    ("g9.p0.m95.single_source_truth", "g9_m95_single_source_truth"),
    ("g9.p0.m96.path_tracer_reference", "g9_m96_path_tracer_reference"),
    ("g9.p0.m97.surface_cache", "g9_m97_surface_cache"),
    ("g9.p0.m98.tracing_fallback_chain", "g9_m98_tracing_fallback_chain"),
    ("g9.p0.m110.world_partition", "g9_m110_world_partition"),
    ("g9.p0.m118.display_pipeline_view_transform", "g9_m118_display_pipeline_view_transform"),
    ("g9.p1.m92.gpu_skinning_lod_update", "g9_m92_gpu_skinning_lod_update"),
    ("g9.p1.m105.command_build_node", "g9_m105_command_build_node"),
    ("g9.p1.m106.execution_set_pso", "g9_m106_execution_set_pso"),
    ("g9.p1.m107.shader_library_ir_link", "g9_m107_shader_library_ir_link"),
    ("g9.p1.m99.spg_radiance_cache", "g9_m99_spg_radiance_cache"),
    ("g9.p1.m100.multi_light_low", "g9_m100_multi_light_low"),
    ("g9.p1.m101.if_tier_ladder", "g9_m101_if_tier_ladder"),
    ("g9.p1.m111.hlod_baking", "g9_m111_hlod_baking"),
    ("g9.p1.m112.atmosphere_froxel", "g9_m112_atmosphere_froxel"),
    ("g9.p1.m113.water_dual_pipeline", "g9_m113_water_dual_pipeline"),
    ("g9.p1.m114.hair_marschner", "g9_m114_hair_marschner"),
    ("g9.p1.m115.skin_burley_diffusion", "g9_m115_skin_burley_diffusion"),
    ("g9.p1.m116.terrain_chunk_cell", "g9_m116_terrain_chunk_cell"),
    ("g9.p1.m117.decal_dbuffer", "g9_m117_decal_dbuffer"),
    ("g9.p1.m119.post_processing_skeleton", "g9_m119_post_processing_skeleton"),
    ("g9.p1.m120.oit_benchmark_harness", "g9_m120_oit_benchmark_harness"),
    ("g9.p1.m124.buoyancy_field_channel", "g9_m124_buoyancy_field_channel"),
    ("g9.p1.m125.jolt_56_ab_evaluation", "g9_m125_jolt_56_ab_evaluation"),
    ("g9.p1.m126.rapier_benchmark_ab", "g9_m126_rapier_benchmark_ab"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def m147_dual_phase_discipline(ev: dict) -> tuple[bool, str]:
    """M147 双 phase 纪律两态判定（G11.3 收口校准，沿 G10.8a wave2 fact④
    m130_phase_discipline 两态先例；契约 §8.3a 修订句消费面；判据语义 0-byte——
    g11.3 phase 绿不替 g11.5 收敛断言充绿）。

    接受形态：
    - A 态（g11.3 phase 登记面）：status==pass ∧ phase==g11.3 ∧ g11_3_phase_pass==true
      ∧（deferred 如实登记：convergence_pending==true ∧ closure.verdict==deferred_to_g11_5
      〔现状形态〕，或实测收敛：convergence_pending==false ∧ closure.converged==true）；
    - B 态（g11.5 phase 收敛断言面，G11.5 落地后合法形态）：status==pass ∧ phase==g11.5
      ∧ convergence_pending==false ∧ closure.converged==true。
    其余一律红（convergence_pending 缺登记冒充全闭环 / g11_3_phase_pass≠true 而 pass /
    phase=g11.5 而 pending=true / 缺 phase 字段的 legacy 形态等）。
    """
    status = ev.get("status")
    phase = ev.get("phase")
    p3 = ev.get("g11_3_phase_pass")
    pending = ev.get("convergence_pending")
    closure = ev.get("closure") or {}
    converged = closure.get("converged")
    verdict = closure.get("verdict")
    if status == "pass" and phase == "g11.3" and p3 is True:
        if pending is True and verdict == "deferred_to_g11_5" and converged is False:
            return True, "M147 A 态：g11.3 phase 绿 + convergence_pending=true（deferred_to_g11_5 如实登记，不替 g11.5 收敛断言充绿）"
        if pending is False and converged is True and verdict == "converged":
            return True, "M147 A 态变体：g11.3 phase 绿 + 实测收敛（converged=true，登记面提前闭环）"
    if status == "pass" and phase == "g11.5" and pending is False and converged is True:
        return True, "M147 B 态：g11.5 phase 收敛断言绿（definitive 测量面）"
    return False, (
        f"M147 phase 纪律不符: status={status} phase={phase} g11_3_phase_pass={p3} "
        f"convergence_pending={pending} converged={converged} verdict={verdict}"
        "（两态外一律红——convergence_pending 缺登记冒充全闭环即 RED，判据语义 0-byte）"
    )


def collect_extra_facts() -> list[dict]:
    facts: list[dict] = []

    # ① 契约 digest 0-byte（双场景当次重算 + 联合值）。
    drift = []
    for s in fl.SCENES:
        try:
            got = fl.contract_digest_rust(s)
        except Exception as e:  # noqa: BLE001
            got = f"<error {e}>"
        if got != fl.LOCKED_DIGEST[s]:
            drift.append(f"{s}: {got} ≠ {fl.LOCKED_DIGEST[s]}")
    facts.append(_fact(
        "contract_digest_locked_unchanged",
        not drift,
        "双场景契约 digest 当次重算 == G10.5 锁定值（cornell 80305791…/bistro ad45951b…，联合 64fd54df…）"
        if not drift else "; ".join(drift[:2]),
    ))

    # ② 六门 RED 臂独立有效。
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
        "six_gates_red_arms_independently_effective",
        not red_bad,
        f"六门最新 evidence 各含 red_* checks 且全真（共 {red_total} 臂独立有效）"
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
        "g11_budget.json 八条 g11.fix.* 标定条目 measured_local + evidence_file 在树可解 "
        "trimmed_mean + threshold == trimmed_mean × k（P-09）"
        if not budget_bad else "; ".join(budget_bad[:3]),
    ))

    # ④ 资产 provenance 齐备（转码 manifest + cornell 语料 0-byte + 零未登记混入）。
    asset_bad: list[str] = []
    if not fl.TRANSCODE_MANIFEST.is_file():
        asset_bad.append("转码 manifest 缺失")
    else:
        m = fl.load_json(fl.TRANSCODE_MANIFEST)
        if len(m.get("entries", [])) != 144:
            asset_bad.append("manifest 条目数 ≠ 144")
        if m.get("format_histogram") != {"bc1": 54, "bc3": 20, "bc5": 70}:
            asset_bad.append("manifest 格式枚举漂移")
        out_dir = Path(m.get("output_dir", ""))
        if out_dir.is_dir():
            registered = {e["product_png"] for e in m["entries"]} | {"buffer.bin", "BistroInterior.gltf"}
            extra = [f.name for f in out_dir.iterdir() if f.is_file() and f.name not in registered]
            if extra:
                asset_bad.append(f"未登记资产混入: {extra[:3]}")
    try:
        reg = fl.load_json(ROOT / "milestones" / "g10" / "g10_asset_license_registry.json")
        row = next((a for a in reg.get("assets", []) if a.get("asset_id") == "cornell-box-generated"), None)
        croot, _src = corpus_lib.resolve_cache_root()
        if row is None or croot is None:
            asset_bad.append("cornell 语料登记行/缓存根不可达")
        else:
            base = croot / str(row["cache_rel"]).rstrip("/")
            digest, count, byte_len, _ = corpus_lib.manifest_level_digest(base)
            if digest != row["digest"] or count != row["file_count"] or byte_len != row["byte_len"]:
                asset_bad.append("cornell 语料 digest/count/byte_len 漂移（语料静默改写）")
    except Exception as e:  # noqa: BLE001
        asset_bad.append(f"语料机核异常: {e}")
    facts.append(_fact(
        "asset_provenance_intact",
        not asset_bad,
        "DDS 转码 manifest 144 条目齐备 + 产物目录零未登记混入 + cornell 语料 M131 登记 digest 复算 0-byte"
        if not asset_bad else "; ".join(asset_bad[:3]),
    ))

    # ⑤ 回归前置自检（48 门最新 evidence 全绿只读汇总 + 默认面帧 digest 逐位 parity）。
    reg_bad: list[str] = []
    for key, prefix in G10_KEYS + G9_KEYS:
        row = wel.require_gate_pass(key, prefix)
        if row["status"] != "PASS":
            reg_bad.append(f"{prefix}: {row['detail'][:60]}")
    parity_bad: list[str] = []
    for scene_id in fl.SCENES:
        pf = fl.FRAMES_G11_3 / "parity" / f"{scene_id}.exr"
        if not pf.is_file():
            parity_bad.append(f"{scene_id} parity 帧缺失")
            continue
        d = fl.decode(pf, "rurix")
        dg = fl.exr.frame_content_digest(d["width"], d["height"], 3, d["pixels"])
        if dg != fl.G10_5_FRAME_DIGEST[("rurix", scene_id)]:
            parity_bad.append(f"{scene_id} 默认面帧 digest 漂移")
    facts.append(_fact(
        "regression_guard_precheck",
        not reg_bad and not parity_bad,
        f"G10 14 门 + G9 34 门最新 evidence 全绿只读汇总 + 默认面帧 digest 逐位 parity（双场景 == G10.5 锁定值）"
        if not reg_bad and not parity_bad else "; ".join((reg_bad + parity_bad)[:3]),
    ))

    # ⑥ M147 双 phase 纪律（两态口径，G11.3 收口校准，沿 G10.8a wave2 fact④ 先例）：
    #    最新 evidence 要么 g11.3 phase 绿且 deferred/收敛如实登记（A 态——不替 g11.5
    #    收敛断言充绿），要么 g11.5 phase 收敛断言绿（B 态——G11.5 落地后合法形态）；
    #    两态外一律红（convergence_pending 缺登记冒充全闭环即 RED，判据语义 0-byte）。
    m147_path = wel.load_latest_evidence("g11_m147_fix_r1_material_subset")
    if m147_path is None:
        phase_ok, phase_detail = False, "M147 最新 evidence 缺失"
    else:
        try:
            phase_ok, phase_detail = m147_dual_phase_discipline(wel.load_json(m147_path))
            phase_detail = f"{phase_detail}（{m147_path.name}）"
        except (OSError, ValueError) as e:
            phase_ok, phase_detail = False, f"M147 evidence 不可读: {e}"
    facts.append(_fact("m147_dual_phase_discipline", phase_ok, phase_detail))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir) for key, prefix in REQUIRED_GATES]
    extras = collect_extra_facts()
    notes_parts = [
        "implemented: six G11.3 gates (P0 M147 fix_r1_material_subset step 201 / "
        "P0 M148 fix_r2_geometry_normals step 202 / P0 M149 fix_r5_json_u64_seed step 203 / "
        "P0 M150 fix_u1_cornell_shell_radiance step 204 / P0 M151 fix_u2_bistro_texture_dds step 205 / "
        "P0 M152 fix_u3_bistro_animation step 206)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: contract digest 0-byte + six gates red arms + calibrated thresholds in budget (P-09) + "
        "asset provenance intact + regression precheck (48 gates + default-face parity) + "
        "M147 dual-phase discipline (g11.3 phase green does not substitute g11.5 convergence assertion)",
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
    """① 缺六门 evidence → 红；② 真树六门绿 + 事实核验 → 绿；
    ③ m147_dual_phase_discipline 两态单元红绿（G11.3 收口校准面）。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    with tempfile.TemporaryDirectory(prefix="g11_wave3_selftest_") as td:
        code = run_gate(evidence_dir=Path(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 正样本:仓库最新六门 evidence")
    import time

    time.sleep(1.1)
    code = run_gate(evidence_dir=None)
    if code != 0:
        print("[selftest] FAIL: 真树聚合未绿（前置六门/事实核验未满足）", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真树聚合绿")

    # 两态单元红绿（合成 evidence dict，不依赖树）：A/A变体/B 三接受态绿 + 六红臂。
    print("[selftest] m147_dual_phase_discipline 两态单元红绿")
    base = {
        "status": "pass", "phase": "g11.3", "g11_3_phase_pass": True,
        "convergence_pending": True,
        "closure": {"converged": False, "verdict": "deferred_to_g11_5"},
    }
    green_a = m147_dual_phase_discipline(base)[0]
    green_a2 = m147_dual_phase_discipline({
        **base, "convergence_pending": False,
        "closure": {"converged": True, "verdict": "converged"},
    })[0]
    green_b = m147_dual_phase_discipline({
        "status": "pass", "phase": "g11.5", "convergence_pending": False,
        "closure": {"converged": True, "verdict": "converged"},
    })[0]
    red_arms = [
        ("convergence_pending 缺登记冒充全闭环（未收敛而 pending=false）",
         {**base, "convergence_pending": False}),
        ("convergence_pending 字段缺失冒充全闭环",
         {k: v for k, v in base.items() if k != "convergence_pending"}),
        ("g11_3_phase_pass≠true 而 pass（其余检未全绿冒充）",
         {**base, "g11_3_phase_pass": False}),
        ("status≠pass（FAIL 件不充绿）",
         {**base, "status": "fail"}),
        ("phase=g11.5 而 pending=true（收敛断言期未收敛冒充绿）",
         {"status": "pass", "phase": "g11.5", "convergence_pending": True,
          "closure": {"converged": False, "verdict": "deferred_to_g11_5"}}),
        ("缺 phase 字段 legacy 形态（双 phase 校准前形态不充新口径绿）",
         {"status": "pass", "g11_3_phase_pass": True, "convergence_pending": True,
          "closure": {"converged": False, "verdict": "deferred_to_g11_5"}}),
    ]
    failures = 0
    if not green_a:
        print("[selftest] RED MISS — A 态（g11.3 phase deferred 如实登记）被误拒")
        failures += 1
    if not green_a2:
        print("[selftest] RED MISS — A 态变体（g11.3 phase 实测收敛）被误拒")
        failures += 1
    if not green_b:
        print("[selftest] RED MISS — B 态（g11.5 phase 收敛断言绿）被误拒")
        failures += 1
    for name, ev in red_arms:
        ok, detail = m147_dual_phase_discipline(ev)
        if ok:
            print(f"[selftest] RED MISS — {name}:负样本过检")
            failures += 1
        else:
            print(f"[selftest] RED ok   — {name}（{detail[:60]}…）")
    if green_a and green_a2 and green_b:
        print("[selftest] GREEN ok — A/A变体/B 三接受态均判绿")
    if failures:
        print(f"[selftest] FAIL ({failures})", file=sys.stderr)
        return 1
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G11.3 wave3.exit 聚合门（只读汇总）")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
