#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.8a soak 聚合门 g9.wave.8a.soak(G9_CONTRACT G-G9-10;G9_PLAN §G9.8a;
G9_ACCEPTANCE_MAP §2/§3/§6;同构 ci/g8_stabilization_soak.py)。

四腿:全量回归(15 P0 + 19 go P1 + wave2~wave6 exit + p2_decisions 波聚合门,
逐门真跑 --gate 后机器核验最新 evidence status==pass〔M90/M91 为 G9.2 既定
无顶层 status 字段形态,按 wel 口径核验并闭集断言〕;M121/M122 双 phase 完整期
按 wave6 聚合口径核验 phase_g9_6_pass==true;携带 base_commit 字段的 32 门
evidence 同值=HEAD 且 40 门 evidence 文件名 stamp ≥ run 起点 = 同一候选
close-out 基线,MAP §6)→ M110 大世界流送长 soak(≥30min 墙钟 且 ≥10000 帧,
全程真实帧循环,禁 sleep 充墙钟)→ budget_eval --strict 非空零
estimated/skip → 纪律日期锚。

诚实语义(沿 G8.8a 2026-08-08 清零假绿后口径,G9 无 legacy 兼容——首份
evidence 即 honesty 格式):
- soak 墙钟=真实帧循环实测(active_frame_seconds),sleep_seconds 恒 0;
  gate 侧用外测墙钟交叉核验,谎报 seconds 判红。
- soak 载体 = g9_m110_world_partition --long-soak(512×512 cell 大世界流送
  调度器逐帧 tick 全工作量 + 逐帧预算一致性机核 + 事件 drain;host 确定性面,
  无 Vulkan validation/device-lost 面,不以字面量 0 充 device 零错门)。
- soak 期间帧计数/hitch p99/流送计数非空(空即红);evidence soak 块
  wall_seconds(seconds)与 frame_count(frames)双字段机器可核。

pr-smoke 默认 --verify-latest(秒级核最新 full-run evidence);
本地/workflow_dispatch 用 --gate 产 full-run。

用法:
  py -3 ci/g9_stabilization_soak.py --gate g9.wave.8a.soak
  py -3 ci/g9_stabilization_soak.py --verify-latest
  py -3 ci/g9_stabilization_soak.py --selftest
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g9_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g9.wave.8a.soak"
NUMERIC_STEP = 171
SUBJECT = "g9_stabilization_soak"
WAVE = "G9.8a"
SOURCE_REF = (
    "G9_CONTRACT G-G9-10;G9_PLAN §G9.8a;G9_ACCEPTANCE_MAP §2/§3/§6;"
    "15 P0 + 19 go P1 + wave2~wave6 exit + p2_decisions 全量回归 + "
    "M110 long soak ≥1800s/≥10000 frames + budget --strict"
)
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_stabilization_soak_evidence_schema.json"

# (symbolic_key, subject_prefix, smoke argv relative, require_real, dual_phase_full)
# 顺序即执行序:M110 性能敏感门提首(hitch p99 host 墙钟门,避开回归腿热机段
# 污染——2026-08-14 重负载暂态漂移处置见 g9_budget.json 条目重标定登记);
# M121 先于 M122(RXS-0375 门序 interlock);M96 先于 M97~M101(D2-Q7 门序);
# 波聚合门最后(只读汇总 34 门最新 evidence)。
REGRESSION_GATES: list[tuple[str, str, list[str], bool, bool]] = [
    # ── G9.5 M110(host;性能敏感门提首)──
    ("g9.p0.m110.world_partition", "g9_m110_world_partition",
     ["ci/g9_world_partition_smoke.py", "--gate", "g9.p0.m110.world_partition"], False, False),
    # ── G9.2 波(步骤 131~137;M121/M122 完整期 --phase g9.6 含骨架期回归)──
    ("g9.p0.m102.dgc_abstraction", "g9_m102_dgc_abstraction",
     ["ci/g9_dgc_abstraction_smoke.py", "--gate", "g9.p0.m102.dgc_abstraction"], True, False),
    ("g9.p0.m90.cluster_dag_deepening", "g9_m90_cluster_dag_deepening",
     ["ci/g9_cluster_dag_deepening_smoke.py", "--gate", "g9.p0.m90.cluster_dag_deepening"], False, False),
    ("g9.p0.m91.page_format_v2_abi", "g9_m91_page_format_v2_abi",
     ["ci/g9_page_format_v2_abi_smoke.py", "--gate", "g9.p0.m91.page_format_v2_abi"], True, False),
    ("g9.p0.m103.descriptor_global_table", "g9_m103_descriptor_global_table",
     ["ci/g9_descriptor_global_table_smoke.py", "--gate", "g9.p0.m103.descriptor_global_table"], True, False),
    ("g9.p0.m104.accesskind_indirect_edge", "g9_m104_accesskind_indirect_edge",
     ["ci/g9_accesskind_indirect_edge_smoke.py", "--gate", "g9.p0.m104.accesskind_indirect_edge"], False, False),
    ("g9.p0.m121.physics_particle_view", "g9_m121_physics_particle_view",
     ["ci/g9_physics_particle_view_smoke.py", "--gate", "g9.p0.m121.physics_particle_view",
      "--phase", "g9.6"], False, True),
    ("g9.p0.m122.gameplay_field", "g9_m122_gameplay_field",
     ["ci/g9_gameplay_field_smoke.py", "--gate", "g9.p0.m122.gameplay_field",
      "--phase", "g9.6"], False, True),
    # ── G9.3 波(步骤 139~145)──
    ("g9.p0.m93.visible_cluster_set", "g9_m93_visible_cluster_set",
     ["ci/g9_visible_cluster_set_smoke.py", "--gate", "g9.p0.m93.visible_cluster_set"], False, False),
    ("g9.p0.m94.clas_rt_convergence", "g9_m94_clas_rt_convergence",
     ["ci/g9_clas_rt_convergence_smoke.py", "--gate", "g9.p0.m94.clas_rt_convergence"], True, False),
    ("g9.p0.m95.single_source_truth", "g9_m95_single_source_truth",
     ["ci/g9_single_source_truth_smoke.py", "--gate", "g9.p0.m95.single_source_truth"], True, False),
    ("g9.p1.m92.gpu_skinning_lod_update", "g9_m92_gpu_skinning_lod_update",
     ["ci/g9_gpu_skinning_lod_update_smoke.py", "--gate", "g9.p1.m92.gpu_skinning_lod_update"], True, False),
    ("g9.p1.m105.command_build_node", "g9_m105_command_build_node",
     ["ci/g9_command_build_node_smoke.py", "--gate", "g9.p1.m105.command_build_node"], True, False),
    ("g9.p1.m106.execution_set_pso", "g9_m106_execution_set_pso",
     ["ci/g9_execution_set_pso_smoke.py", "--gate", "g9.p1.m106.execution_set_pso"], True, False),
    ("g9.p1.m107.shader_library_ir_link", "g9_m107_shader_library_ir_link",
     ["ci/g9_shader_library_ir_link_smoke.py", "--gate", "g9.p1.m107.shader_library_ir_link"], False, False),
    # ── G9.4 波(步骤 147~152;M96 门序源先行)──
    ("g9.p0.m96.path_tracer_reference", "g9_m96_path_tracer_reference",
     ["ci/g9_path_tracer_reference_smoke.py", "--gate", "g9.p0.m96.path_tracer_reference"], True, False),
    ("g9.p0.m97.surface_cache", "g9_m97_surface_cache",
     ["ci/g9_surface_cache_smoke.py", "--gate", "g9.p0.m97.surface_cache"], True, False),
    ("g9.p0.m98.tracing_fallback_chain", "g9_m98_tracing_fallback_chain",
     ["ci/g9_tracing_fallback_chain_smoke.py", "--gate", "g9.p0.m98.tracing_fallback_chain"], True, False),
    ("g9.p1.m99.spg_radiance_cache", "g9_m99_spg_radiance_cache",
     ["ci/g9_spg_radiance_cache_smoke.py", "--gate", "g9.p1.m99.spg_radiance_cache"], True, False),
    ("g9.p1.m100.multi_light_low", "g9_m100_multi_light_low",
     ["ci/g9_multi_light_low_smoke.py", "--gate", "g9.p1.m100.multi_light_low"], True, False),
    ("g9.p1.m101.if_tier_ladder", "g9_m101_if_tier_ladder",
     ["ci/g9_if_tier_ladder_smoke.py", "--gate", "g9.p1.m101.if_tier_ladder"], True, False),
    # ── G9.5 波(步骤 155~164,全 host 纯 host 确定性门;M110 已提首)──
    ("g9.p0.m118.display_pipeline_view_transform", "g9_m118_display_pipeline_view_transform",
     ["ci/g9_display_pipeline_view_transform_smoke.py", "--gate",
      "g9.p0.m118.display_pipeline_view_transform"], False, False),
    ("g9.p1.m111.hlod_baking", "g9_m111_hlod_baking",
     ["ci/g9_hlod_baking_smoke.py", "--gate", "g9.p1.m111.hlod_baking"], False, False),
    ("g9.p1.m112.atmosphere_froxel", "g9_m112_atmosphere_froxel",
     ["ci/g9_atmosphere_froxel_smoke.py", "--gate", "g9.p1.m112.atmosphere_froxel"], False, False),
    ("g9.p1.m113.water_dual_pipeline", "g9_m113_water_dual_pipeline",
     ["ci/g9_water_dual_pipeline_smoke.py", "--gate", "g9.p1.m113.water_dual_pipeline"], False, False),
    ("g9.p1.m114.hair_marschner", "g9_m114_hair_marschner",
     ["ci/g9_hair_marschner_smoke.py", "--gate", "g9.p1.m114.hair_marschner"], False, False),
    ("g9.p1.m115.skin_burley_diffusion", "g9_m115_skin_burley_diffusion",
     ["ci/g9_skin_burley_diffusion_smoke.py", "--gate", "g9.p1.m115.skin_burley_diffusion"], False, False),
    ("g9.p1.m116.terrain_chunk_cell", "g9_m116_terrain_chunk_cell",
     ["ci/g9_terrain_chunk_cell_smoke.py", "--gate", "g9.p1.m116.terrain_chunk_cell"], False, False),
    ("g9.p1.m117.decal_dbuffer", "g9_m117_decal_dbuffer",
     ["ci/g9_decal_dbuffer_smoke.py", "--gate", "g9.p1.m117.decal_dbuffer"], False, False),
    ("g9.p1.m119.post_processing_skeleton", "g9_m119_post_processing_skeleton",
     ["ci/g9_post_processing_skeleton_smoke.py", "--gate", "g9.p1.m119.post_processing_skeleton"], False, False),
    ("g9.p1.m120.oit_benchmark_harness", "g9_m120_oit_benchmark_harness",
     ["ci/g9_oit_benchmark_harness_smoke.py", "--gate", "g9.p1.m120.oit_benchmark_harness"], False, False),
    # ── G9.6 波(步骤 166~168,全 host 纯 host 确定性门)──
    ("g9.p1.m124.buoyancy_field_channel", "g9_m124_buoyancy_field_channel",
     ["ci/g9_buoyancy_field_channel_smoke.py", "--gate", "g9.p1.m124.buoyancy_field_channel"], False, False),
    ("g9.p1.m126.rapier_benchmark_ab", "g9_m126_rapier_benchmark_ab",
     ["ci/g9_rapier_benchmark_ab_smoke.py", "--gate", "g9.p1.m126.rapier_benchmark_ab"], False, False),
    ("g9.p1.m125.jolt_56_ab_evaluation", "g9_m125_jolt_56_ab_evaluation",
     ["ci/g9_jolt_56_ab_evaluation_smoke.py", "--gate", "g9.p1.m125.jolt_56_ab_evaluation"], False, False),
    # ── 波聚合门(步骤 138/146/153/165/169/170;只读汇总不重跑子门 smoke)──
    ("g9.wave.2.exit", "g9_wave2_exit",
     ["ci/g9_wave2_exit_check.py", "--gate", "g9.wave.2.exit"], False, False),
    ("g9.wave.3.exit", "g9_wave3_exit",
     ["ci/g9_wave3_exit_check.py", "--gate", "g9.wave.3.exit"], False, False),
    ("g9.wave.4.exit", "g9_wave4_exit",
     ["ci/g9_wave4_exit_check.py", "--gate", "g9.wave.4.exit"], False, False),
    ("g9.wave.5.exit", "g9_wave5_exit",
     ["ci/g9_wave5_exit_check.py", "--gate", "g9.wave.5.exit"], False, False),
    ("g9.wave.6.exit", "g9_wave6_exit",
     ["ci/g9_wave6_exit_check.py", "--gate", "g9.wave.6.exit"], False, False),
    ("g9.wave.7.decisions", "g9_p2_decisions",
     ["ci/g9_p2_decisions_check.py", "--gate", "g9.wave.7.decisions"], False, False),
]

MIN_SECONDS = 1800
MIN_FRAMES = 10000
# 34 门(15 P0 + 19 go P1)为门 evidence(顶层 status==pass 字面);后 6 门为波
# 聚合 evidence(host_section_pass + checks 全真口径,无顶层 status 字段)。
N_ASSERTION_GATES = 34


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def base_commit() -> str:
    r = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return (r.stdout or "").strip() or "unknown"


def verify_assertion_gate(key: str, prefix: str, dual_phase_full: bool) -> dict:
    """34 门(15 P0 + 19 go P1)最新 evidence 机器核验。

    在 wel.require_gate_pass 口径(symbolic_gate_key/host_section_pass/
    device_section_state/checks 全真)之上叠加:
    - 顶层 status=="pass" 字面(MAP §1 evidence 必备字段,skip/estimated 不充绿);
      M90/M91 为 G9.2 agent A 既定 evidence 形态(无顶层 status 字段,同 wave 门
      wel 口径被 wave2 聚合与 check_schemas 接受)——无字段门按 wel 口径核验并
      如实登记,不改写既有门脚本(0-byte 纪律);
    - dual_phase_full=True(M121/M122)按 wave6 聚合口径叠加
      phase_g9_2_pass==true 且 phase_g9_6_pass==true(骨架期绿不替完整期充绿)。
    """
    row = wel.require_gate_pass(key, prefix)
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return row
    try:
        doc = wel.load_json(path)
    except (OSError, json.JSONDecodeError):
        return row
    problems: list[str] = []
    if "status" in doc:
        if doc.get("status") != "pass":
            problems.append(f"status={doc.get('status')!r} ≠ 'pass'")
    else:
        row["detail"] = (
            f"{row.get('detail', '')}; 无顶层 status 字段(G9.2 既定形态),"
            "按 wel 口径(host/checks/device)核验"
        )
    if dual_phase_full:
        if doc.get("phase_g9_2_pass") is not True:
            problems.append("phase_g9_2_pass≠true")
        if doc.get("phase_g9_6_pass") is not True:
            problems.append("phase_g9_6_pass≠true(骨架期绿不替完整期充绿)")
    if problems:
        row["status"] = "FAIL"
        row["detail"] = f"{row.get('detail', '')}; " + "; ".join(problems)
    return row


def run_regression(*, skip_rerun: bool = False) -> tuple[bool, list[dict], str, bool]:
    """全量回归(34 门 + 6 波聚合门)。

    skip_rerun=False(--gate):逐门真跑 smoke --gate 后核验其最新 evidence;
    skip_rerun=True(--verify-latest):只读最新 evidence。
    返回 (all_ok, rows, head_commit, base_uniform)。base_uniform 口径(MAP §6
    「同一候选 close-out 基线」):
    - 携带 base_commit 字段的 32 门 evidence 同值且=当前 HEAD;无该字段的门
      恰为 {M90, M91}(G9.2 既定形态,闭集断言——多一门缺字段即红);
    - --gate 模式追加新鲜度机核:40 门最新 evidence 文件名 UTC stamp 均 ≥
      本 run 起点 stamp(证明逐门真跑于本次 close-out 基线工作树)。
    """
    rows: list[dict] = []
    commit = base_commit()
    run_start_stamp = wel.utc_stamp()
    all_ok = True
    bases: list[str] = []
    no_base_field: list[str] = []
    stale: list[str] = []
    stamp_re = re.compile(r"_(\d{8}T\d{6}Z)\.json$")
    for idx, (key, prefix, argv, require_real, dual_phase_full) in enumerate(REGRESSION_GATES):
        is_aggregate = idx >= N_ASSERTION_GATES
        if not skip_rerun:
            env = os.environ.copy()
            if require_real:
                env["RURIX_REQUIRE_REAL"] = "1"
                env["RURIX_VK_VALIDATION"] = "1"
            script = ROOT / argv[0]
            if not script.is_file():
                rows.append(
                    {
                        "symbolic_gate_key": key,
                        "subject_prefix": prefix,
                        "evidence_path": None,
                        "status": "FAIL",
                        "detail": f"smoke missing: {argv[0]}",
                    }
                )
                all_ok = False
                continue
            print(f"[8a] regression {key}", flush=True)
            r = subprocess.run(
                [sys.executable, str(script), *argv[1:]],
                cwd=ROOT,
                env=env,
            )
            if r.returncode != 0:
                rows.append(
                    {
                        "symbolic_gate_key": key,
                        "subject_prefix": prefix,
                        "evidence_path": None,
                        "status": "FAIL",
                        "detail": f"smoke exit={r.returncode}",
                    }
                )
                all_ok = False
                continue
        if is_aggregate:
            row = wel.require_gate_pass(key, prefix)
        else:
            row = verify_assertion_gate(key, prefix, dual_phase_full)
            path = wel.load_latest_evidence(prefix)
            if path is not None:
                try:
                    doc = wel.load_json(path)
                    bc = doc.get("base_commit")
                    if bc is None:
                        no_base_field.append(key)
                    else:
                        bases.append(str(bc))
                except (OSError, json.JSONDecodeError):
                    bases.append("<unreadable>")
        if not skip_rerun and row.get("evidence_path"):
            m = stamp_re.search(str(row["evidence_path"]))
            if m is None or m.group(1) < run_start_stamp:
                stale.append(key)
                row["status"] = "FAIL"
                row["detail"] = (
                    f"{row.get('detail', '')}; evidence 非本 run 新鲜产出"
                    f"(stamp {m.group(1) if m else '?'} < run 起点 {run_start_stamp})"
                )
        rows.append(row)
        if row["status"] != "PASS":
            all_ok = False
    expected_no_base = {"g9.p0.m90.cluster_dag_deepening", "g9.p0.m91.page_format_v2_abi"}
    base_uniform = (
        not stale
        and sorted(no_base_field) == sorted(expected_no_base)
        and len(bases) == N_ASSERTION_GATES - len(expected_no_base)
        and len(set(bases)) == 1
        and bases[0] == commit
        and commit != "unknown"
    )
    return all_ok, rows, commit, base_uniform


def judge_soak(
    doc: dict,
    *,
    outer_elapsed: float | None,
    min_seconds: int,
    min_frames: int,
) -> tuple[bool, list[str]]:
    """诚实判定 soak 输出。返回 (ok, problems)。

    反假绿(G8.8a 2026-08-08 口径,G9 无 legacy 兼容):
    - 必须带 honesty 字段(soak_subject=host-soak / sleep_seconds /
      active_frame_seconds);缺字段 = 旧二进制或伪造 → 红。
    - sleep_seconds 必须 == 0(禁 sleep 充墙钟)。
    - active_frame_seconds ≈ soak_seconds(墙钟只能来自真实帧循环)。
    - 外测墙钟(非 None 时)不得小于自称 seconds - 2s(谎报时长 → 红)。
    - 双阈值:frames ≥ min_frames 且 seconds ≥ min_seconds。
    - 帧计数/hitch p99/流送计数非空(total_events/total_cells_streamed ≥1,
      hitch.p99_ms >0)。
    host-soak 无 device 面:不以 validation_messages/device_lost_count 作门。
    """
    problems: list[str] = []
    frames = int(doc.get("soak_frames") or doc.get("frames") or 0)
    # 二进制 raw 输出键为 soak_seconds;gate 落盘的 evidence soak 块键为 seconds——
    # 同一判定函数服务两种来源,秒数取键需兼容。
    seconds = float(doc.get("soak_seconds") or doc.get("seconds") or 0.0)
    if doc.get("soak_subject") != "host-soak":
        problems.append(f"soak_subject={doc.get('soak_subject')!r} ≠ 'host-soak'(缺 honesty 字段)")
    sleep_s = doc.get("sleep_seconds")
    if sleep_s is None:
        problems.append("缺 sleep_seconds 字段(旧二进制/伪造 → 红)")
    elif float(sleep_s) != 0.0:
        problems.append(f"sleep_seconds={sleep_s} ≠ 0(sleep 充墙钟)")
    active = doc.get("active_frame_seconds")
    if active is None:
        problems.append("缺 active_frame_seconds 字段")
    elif abs(float(active) - seconds) > 1.0:
        problems.append(
            f"active_frame_seconds={active} 与 soak_seconds={seconds} 偏差 >1s(墙钟非帧循环产出)"
        )
    if frames < min_frames:
        problems.append(f"frames={frames} < min_frames={min_frames}")
    if seconds < min_seconds:
        problems.append(f"seconds={seconds:.1f} < min_seconds={min_seconds}")
    if outer_elapsed is not None and outer_elapsed + 2.0 < seconds:
        problems.append(
            f"外测墙钟 {outer_elapsed:.1f}s < 自称 seconds={seconds:.1f}s(谎报时长)"
        )
    hitch = doc.get("hitch") or {}
    if float(hitch.get("p99_ms") or 0.0) <= 0.0:
        problems.append("hitch.p99_ms 非正(计数面空)")
    if int(doc.get("total_events") or 0) < 1 or int(doc.get("total_cells_streamed") or 0) < 1:
        problems.append("total_events/total_cells_streamed 计数面空")
    return (not problems), problems


def run_m110_long_soak(
    *,
    min_seconds: int = MIN_SECONDS,
    min_frames: int = MIN_FRAMES,
) -> tuple[bool, dict]:
    """驱动 g9_m110_world_partition --long-soak;双阈值同时满足且墙钟诚实。"""
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "rurix-render",
        "--bin",
        "g9_m110_world_partition",
        "--",
        "--long-soak",
        "--min-seconds",
        str(min_seconds),
        "--min-frames",
        str(min_frames),
    ]
    print(f"[8a] M110 long soak: {' '.join(cmd)}", flush=True)
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    elapsed = time.time() - t0
    out = (r.stdout or "").strip().splitlines()
    doc: dict = {}
    for line in reversed(out):
        line = line.strip()
        if line.startswith("{") and line.endswith("}"):
            try:
                doc = json.loads(line)
                break
            except json.JSONDecodeError:
                continue
    frames = int(doc.get("soak_frames") or doc.get("frames") or 0)
    seconds = float(doc.get("soak_seconds") or 0.0)
    ok, problems = judge_soak(
        doc, outer_elapsed=elapsed, min_seconds=min_seconds, min_frames=min_frames
    )
    if r.returncode != 0:
        ok = False
        problems.append(f"exit={r.returncode} ≠ 0")
    detail = (
        f"exit={r.returncode} frames={frames} seconds={seconds:.1f} "
        f"outer_wall={elapsed:.1f} sleep={doc.get('sleep_seconds')} "
        f"subject={doc.get('soak_subject')!r}"
    )
    if problems:
        detail += f" problems={problems}"
    if r.returncode != 0:
        err = (r.stderr or "")[-500:]
        detail += f" stderr={err!r}"
    return ok, {
        "ok": ok,
        "frames": frames,
        "seconds": seconds,
        "outer_elapsed": elapsed,
        "detail": detail,
        "raw": doc,
    }


def run_budget_strict() -> tuple[bool, str]:
    """budget_eval --strict:非空、零 estimated/skip(exit 0 且 PASS 且 0 skip)。"""
    r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "budget_eval.py"), "--strict"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    text = (r.stdout or "") + (r.stderr or "")
    m = re.search(r"\[budget_eval\] PASS \((\d+) pass, (\d+) skip, strict mode\)", text)
    ok = r.returncode == 0 and m is not None and int(m.group(1)) >= 1 and int(m.group(2)) == 0
    tail = (text.strip().splitlines() or [""])[-1]
    return ok, f"exit={r.returncode}; {tail}"


def verify_latest() -> int:
    path = wel.load_latest_evidence(SUBJECT)
    if path is None:
        print("[8a] FAIL: missing soak evidence", file=sys.stderr)
        return 1
    data = wel.load_json(path)
    errs = wel.validate_schema(data, SCHEMA_PATH) if SCHEMA_PATH.is_file() else []
    if errs:
        print(f"[8a] schema FAIL: {errs}", file=sys.stderr)
        return 1
    if data.get("host_section_pass") is not True:
        print("[8a] FAIL: host_section_pass≠true", file=sys.stderr)
        return 1
    checks = data.get("checks") or {}
    need = [
        "regression_all_pass",
        "base_commit_uniform",
        "soak_dual_threshold",
        "soak_no_sleep_padding",
        "budget_strict_pass",
        "date_anchor_recorded",
    ]
    bad = [k for k in need if checks.get(k) is not True]
    if bad:
        print(f"[8a] FAIL checks: {bad}", file=sys.stderr)
        return 1
    soak = data.get("soak") or {}
    ok, problems = judge_soak(
        soak, outer_elapsed=None, min_seconds=MIN_SECONDS, min_frames=MIN_FRAMES
    )
    if not ok:
        print(f"[8a] FAIL soak honesty: {problems}", file=sys.stderr)
        return 1
    print(f"[8a] verify-latest PASS(honest soak)← {path.relative_to(ROOT)}")
    return 0


def run_full_gate() -> int:
    if NUMERIC_STEP <= 0:
        print("[8a] NUMERIC_STEP unset (Gov 回填前草稿 → 红)", file=sys.stderr)
        return 1
    if not SCHEMA_PATH.is_file():
        print(f"[8a] schema missing: {SCHEMA_PATH}", file=sys.stderr)
        return 1

    reg_ok, reg_rows, commit, base_uniform = run_regression(skip_rerun=False)
    soak_ok, soak_info = run_m110_long_soak()
    bud_ok, bud_detail = run_budget_strict()
    stamp = wel.utc_stamp()
    utc_date = stamp[:8]

    overall = reg_ok and base_uniform and soak_ok and bud_ok
    raw_soak = soak_info.get("raw") or {}
    no_sleep_ok, _ = judge_soak(
        raw_soak, outer_elapsed=None, min_seconds=0, min_frames=0
    )
    facts = [
        _fact(
            "regression_15p0_19p1_6wave",
            reg_ok,
            f"gates={len(reg_rows)} base_commit={commit}",
        ),
        _fact(
            "base_commit_uniform",
            base_uniform,
            "34 门 evidence base_commit 同值且=HEAD(同一候选 close-out 基线,MAP §6)",
        ),
        _fact("soak_dual_threshold", soak_ok, soak_info["detail"]),
        _fact("budget_strict", bud_ok, bud_detail),
        _fact("date_anchor", True, f"utc_date={utc_date}"),
    ]
    # host-soak 无 device 面,validation/device_lost 不作门亦不写实(沿 G8.8a 语义)。
    hitch = raw_soak.get("hitch") or {}
    soak_block = {
        "frames": soak_info.get("frames", 0),
        "seconds": soak_info.get("seconds", 0.0),
        "min_frames": MIN_FRAMES,
        "min_seconds": MIN_SECONDS,
        "soak_subject": "host-soak",
        "outer_wall_seconds": round(float(soak_info.get("outer_elapsed") or 0.0), 3),
        "hitch_p99_ms": float(hitch.get("p99_ms") or 0.0),
        "total_events": int(raw_soak.get("total_events") or 0),
        "total_cells_streamed": int(raw_soak.get("total_cells_streamed") or 0),
    }
    for k in ("active_frame_seconds", "sleep_seconds"):
        if k in raw_soak:
            soak_block[k] = raw_soak[k]
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
        "base_commit": commit,
        "utc_date": utc_date,
        "required_gates": reg_rows,
        "extra_facts": facts,
        "subjects": [],
        "checks": {
            "regression_all_pass": reg_ok,
            "base_commit_uniform": base_uniform,
            "soak_dual_threshold": soak_ok,
            "soak_no_sleep_padding": no_sleep_ok,
            "budget_strict_pass": bud_ok,
            "date_anchor_recorded": True,
        },
        "soak": soak_block,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": (
            "G9.8a full soak; four legs; honest semantics(G8.8a 2026-08-08 口径): "
            "soak 墙钟=真实帧循环实测(禁 sleep 充时,sleep_seconds 恒 0, "
            "gate 外测墙钟交叉核验);soak 载体=g9_m110_world_partition --long-soak "
            "(512×512 cell 大世界流送,3072 帧闭式路径周期);subject=host-soak 无 "
            "device 零错字面量门;34 门 evidence base_commit 同值一致(MAP §6 同一 "
            "候选 close-out 基线)"
        ),
    }
    errs = wel.validate_schema(payload, SCHEMA_PATH)
    if errs:
        print(f"[8a] schema errors: {errs}", file=sys.stderr)
        overall = False
        payload["host_section_pass"] = False
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    out.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    for f in facts:
        print(f"  FACT  {f['status']:4}  {f['id']}  ({f['detail']})")
    print(f"  → evidence {out.relative_to(ROOT)}")
    print(f"  VERDICT = {'PASS' if overall else 'FAIL'}")
    return 0 if overall else 1


def selftest() -> int:
    """反假绿臂:复现 G8.8a 基线假绿样式与 G9 新增面,必须判红。"""
    arms: list[tuple[str, bool]] = []

    # A1 基线假绿:100 帧 + sleep 凑 1800s + 字面量 0(旧二进制输出样式)。
    baseline_fake = {
        "ok": True, "soak": True, "soak_frames": 100, "frames": 100,
        "soak_seconds": 1800.0, "validation_messages": 0,
        "device_lost_count": 0, "rss_samples": 20, "rss_final": 0,
        "hitch": {"p99_ms": 5.0}, "total_events": 10, "total_cells_streamed": 10,
    }
    ok, probs = judge_soak(
        baseline_fake, outer_elapsed=20.0, min_seconds=1800, min_frames=10000
    )
    arms.append(("A1 baseline_fake(100帧/sleep凑1800s/字面量0)→红", not ok))
    print(f"[selftest] A1 judge ok={ok} problems={probs}")

    # A2 honesty 字段齐但 sleep_seconds>0(sleep 充墙钟)→ 红。
    sleep_padded = {
        "soak_subject": "host-soak", "soak_frames": 10000, "frames": 10000,
        "soak_seconds": 1800.0, "active_frame_seconds": 1030.0,
        "sleep_seconds": 770.0,
        "hitch": {"p99_ms": 5.0}, "total_events": 10, "total_cells_streamed": 10,
    }
    ok, probs = judge_soak(
        sleep_padded, outer_elapsed=1801.0, min_seconds=1800, min_frames=10000
    )
    arms.append(("A2 sleep_seconds>0(sleep充墙钟)→红", not ok))
    print(f"[selftest] A2 judge ok={ok} problems={probs}")

    # A3 外测墙钟戳穿谎报:自称 2079s 但外测只有 25s → 红。
    wall_lie = {
        "soak_subject": "host-soak", "soak_frames": 10000, "frames": 10000,
        "soak_seconds": 2079.5, "active_frame_seconds": 2079.5,
        "sleep_seconds": 0.0,
        "hitch": {"p99_ms": 5.0}, "total_events": 10, "total_cells_streamed": 10,
    }
    ok, probs = judge_soak(
        wall_lie, outer_elapsed=25.0, min_seconds=1800, min_frames=10000
    )
    arms.append(("A3 外测墙钟25s<自称2079s(谎报)→红", not ok))
    print(f"[selftest] A3 judge ok={ok} problems={probs}")

    # A4 帧数不足:9999 帧诚实墙钟 → 红。
    frames_short = {
        "soak_subject": "host-soak", "soak_frames": 9999, "frames": 9999,
        "soak_seconds": 2079.5, "active_frame_seconds": 2079.5,
        "sleep_seconds": 0.0,
        "hitch": {"p99_ms": 5.0}, "total_events": 10, "total_cells_streamed": 10,
    }
    ok, probs = judge_soak(
        frames_short, outer_elapsed=2082.0, min_seconds=1800, min_frames=10000
    )
    arms.append(("A4 9999帧<10000(双阈值缺帧)→红", not ok))
    print(f"[selftest] A4 judge ok={ok} problems={probs}")

    # A5 计数面空:hitch p99=0 / 流送计数 0 → 红。
    empty_counters = {
        "soak_subject": "host-soak", "soak_frames": 10000, "frames": 10000,
        "soak_seconds": 2079.5, "active_frame_seconds": 2079.5,
        "sleep_seconds": 0.0,
        "hitch": {"p99_ms": 0.0}, "total_events": 0, "total_cells_streamed": 0,
    }
    ok, probs = judge_soak(
        empty_counters, outer_elapsed=2082.0, min_seconds=1800, min_frames=10000
    )
    arms.append(("A5 hitch p99/流送计数空→红", not ok))
    print(f"[selftest] A5 judge ok={ok} problems={probs}")

    # A6 诚实样本:10000+ 真实帧 / 实测 2079.5s / sleep=0 / 计数非空 → 绿(正臂)。
    honest = {
        "soak_subject": "host-soak", "soak_frames": 520000, "frames": 520000,
        "soak_seconds": 2079.5, "active_frame_seconds": 2079.5,
        "sleep_seconds": 0.0,
        "hitch": {"p99_ms": 9.2219}, "total_events": 8123456, "total_cells_streamed": 2345678,
    }
    ok, probs = judge_soak(
        honest, outer_elapsed=2082.0, min_seconds=1800, min_frames=10000
    )
    arms.append(("A6 诚实 soak(520000帧/2079.5s/sleep=0/计数非空)→绿", ok))
    print(f"[selftest] A6 judge ok={ok} problems={probs}")

    failed = [name for name, good in arms if not good]
    for name, good in arms:
        print(f"[selftest] {'PASS' if good else 'FAIL'}  {name}")
    if failed:
        print(f"[selftest] FAIL arms: {failed}", file=sys.stderr)
        return 1
    print("[selftest] PASS: 反假绿臂全部符合预期(5 红臂 + 1 绿臂)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G9.8a stabilization soak")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        if NUMERIC_STEP <= 0:
            print("[8a] selftest: NUMERIC_STEP=0 draft → expect red on --gate")
            code = run_full_gate()
            if code == 0:
                print("[selftest] FAIL: draft still green", file=sys.stderr)
                return 1
            print("[selftest] PASS: draft NUMERIC_STEP=0 → red")
            return 0
        return selftest()
    if args.verify_latest:
        return verify_latest()
    return run_full_gate()


if __name__ == "__main__":
    sys.exit(main())
