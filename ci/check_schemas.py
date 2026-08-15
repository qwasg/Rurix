"""PR Smoke 步骤 2:注册表/预算/证据 JSON 的 schema 校验(CI_GATES.md §3.2)。

- registry/deferred.json / spike_gating.json:结构字段与编号格式;
- milestones/*/m*_budget.json:结构 + 命名空间强制前缀(14 §3);
- evidence/*.json:对 milestones/m0/evidence_schema.json 做 JSON Schema 校验。
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ERRORS: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def load(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:  # noqa: BLE001 - 报告而非崩溃
        err(f"{path.relative_to(ROOT)}: 无法解析 JSON: {e}")
        return None


def check_deferred(path: Path) -> None:
    data = load(path)
    if data is None:
        return
    seen: set[str] = set()
    for entry in data.get("entries", []):
        eid = entry.get("id", "")
        if not re.fullmatch(r"RD-\d{3}", eid):
            err(f"deferred: 编号格式非法: {eid!r}")
        if eid in seen:
            err(f"deferred: 编号重复: {eid}")
        seen.add(eid)
        for field in ("title", "reason", "backfill_condition", "owner_milestone", "status", "history"):
            if field not in entry:
                err(f"deferred {eid}: 缺字段 {field}")
        if entry.get("status") not in ("open", "inherited", "closed"):
            err(f"deferred {eid}: status 非法: {entry.get('status')!r}")
        if not entry.get("history"):
            err(f"deferred {eid}: history 不得为空(留痕要求,14 §4)")


def check_gating(path: Path) -> None:
    data = load(path)
    if data is None:
        return
    seen: set[str] = set()
    for entry in data.get("entries", []):
        eid = entry.get("id", "")
        if not re.fullmatch(r"SG-\d{3}", eid):
            err(f"spike_gating: 编号格式非法: {eid!r}")
        if eid in seen:
            err(f"spike_gating: 编号重复: {eid}")
        seen.add(eid)
        for field in ("direction", "trigger_condition", "permanence", "current_verdict", "decisions"):
            if field not in entry:
                err(f"spike_gating {eid}: 缺字段 {field}")
        if entry.get("permanence") not in ("permanent", "conditional"):
            err(f"spike_gating {eid}: permanence 非法")
        if not entry.get("decisions"):
            err(f"spike_gating {eid}: decisions 不得为空(留痕要求,14 §7)")


def parse_message_keys(path: Path) -> set[str] | None:
    """解析 rurixc 消息表行格式(key = 模板;# 注释),返回 key 集。"""
    if not path.is_file():
        return None
    keys: set[str] = set()
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            err(f"messages: 第 {lineno} 行缺 '=': {line!r}")
            continue
        key = line.split("=", 1)[0].strip()
        if not key or any(c.isspace() for c in key):
            err(f"messages: 第 {lineno} 行 key 非法: {key!r}")
            continue
        if key in keys:
            err(f"messages: key 重复: {key}")
        keys.add(key)
    return keys


def check_error_codes(path: Path) -> None:
    """错误码注册表校验(07 §5 分配制;M1 CI_GATES §2 步骤 11)。"""
    if not path.is_file():
        return  # M1.1 落地前不存在,放行
    data = load(path)
    if data is None:
        return
    message_keys = parse_message_keys(ROOT / "src/rurixc/src/messages/en.messages")
    seen: set[str] = set()
    for entry in data.get("entries", []):
        eid = entry.get("id", "")
        if not re.fullmatch(r"RX\d{4}", eid):
            err(f"error_codes: 编号格式非法: {eid!r}")
        elif eid[2] not in "01234567":
            err(f"error_codes {eid}: 段位非法(0-7,07 §5)")
        if eid in seen:
            err(f"error_codes: 编号重复: {eid}(编号永不复用,10 §9.5)")
        seen.add(eid)
        for field in ("title", "message_key", "status", "introduced_in"):
            if not entry.get(field):
                err(f"error_codes {eid}: 缺字段 {field}")
        if entry.get("status") not in ("active", "deprecated"):
            err(f"error_codes {eid}: status 非法: {entry.get('status')!r}")
        mk = entry.get("message_key")
        if mk and message_keys is not None and mk not in message_keys:
            err(f"error_codes {eid}: message_key 未在 en.messages 注册: {mk!r}")


def check_budget(path: Path) -> None:
    data = load(path)
    if data is None:
        return
    ns = data.get("namespace")
    if not ns:
        err(f"{path.name}: 缺 namespace 字段")
        return
    prefix = ns + "."
    ids: set[str] = set()
    groups = ("entries", "ratio_assertions", "counter_assertions")
    for group in groups:
        for entry in data.get(group, []):
            eid = entry.get("id", "")
            if not eid.startswith(prefix):
                err(f"{path.name}: id {eid!r} 未带强制前缀 {prefix!r}(14 §3)")
            if eid in ids:
                err(f"{path.name}: id 重复(命名空间冲突): {eid}")
            ids.add(eid)
    for entry in data.get("entries", []):
        ev = entry.get("evidence")
        if ev not in ("measured_local", "unlocked", "estimated"):
            err(f"{path.name} {entry.get('id')}: evidence 非法: {ev!r}")
        if ev == "estimated" and not entry.get("skip_reason"):
            err(f"{path.name} {entry.get('id')}: estimated 占位必须输出 skip_reason(14 §3)")
        if ev == "measured_local":
            if entry.get("threshold") is None:
                err(f"{path.name} {entry.get('id')}: measured_local 必须有 threshold")
            if not entry.get("evidence_file"):
                err(f"{path.name} {entry.get('id')}: measured_local 必须登记 evidence_file")
            elif not (ROOT / entry["evidence_file"]).is_file():
                err(f"{path.name} {entry.get('id')}: evidence_file 不存在: {entry['evidence_file']}")


def check_evidence_files() -> None:
    gpu_schema = load(ROOT / "milestones/m0/evidence_schema.json")
    frontend_schema = load(ROOT / "milestones/m1/frontend_evidence_schema.json")
    compile_schema = load(ROOT / "milestones/m3/compile_evidence_schema.json")
    sanitizer_schema = load(ROOT / "milestones/m5/compute_sanitizer_evidence_schema.json")
    redistribution_schema = load(ROOT / "milestones/m5/redistribution_audit_evidence_schema.json")
    rx_cli_smoke_schema = load(ROOT / "milestones/m6/rx_cli_smoke_evidence_schema.json")
    offline_rebuild_schema = load(ROOT / "milestones/m6/offline_rebuild_evidence_schema.json")
    lsp_smoke_schema = load(ROOT / "milestones/m6/lsp_smoke_evidence_schema.json")
    lsp_latency_schema = load(ROOT / "milestones/m6/lsp_latency_evidence_schema.json")
    stdlib_math_schema = load(ROOT / "milestones/m7/stdlib_math_evidence_schema.json")
    soft_raster_schema = load(ROOT / "milestones/m7/soft_raster_evidence_schema.json")
    uc03_demo_schema = load(ROOT / "milestones/m7/uc03_demo_evidence_schema.json")
    uc01_interop_schema = load(ROOT / "milestones/m8/uc01_interop_evidence_schema.json")
    cublas_binding_schema = load(ROOT / "milestones/m8/cublas_binding_evidence_schema.json")
    uc02_stream_pipeline_schema = load(
        ROOT / "milestones/m8/uc02_stream_pipeline_evidence_schema.json"
    )
    release_schema = load(ROOT / "milestones/m8/release_evidence_schema.json")
    bilingual_schema = load(
        ROOT / "milestones/m8/bilingual_diagnostic_coverage_evidence_schema.json"
    )
    doc_site_schema = load(ROOT / "milestones/m8/doc_site_smoke_evidence_schema.json")
    d3d12_interop_schema = load(ROOT / "milestones/g1/d3d12_interop_evidence_schema.json")
    realtime_present_schema = load(ROOT / "milestones/g1/realtime_present_evidence_schema.json")
    async_buffer_schema = load(ROOT / "milestones/g1/async_buffer_evidence_schema.json")
    engine_integration_schema = load(ROOT / "milestones/g1/engine_integration_evidence_schema.json")
    fatbin_dist_schema = load(ROOT / "milestones/g1/fatbin_dist_evidence_schema.json")
    dxil_path_spike_schema = load(ROOT / "milestones/g2/dxil_path_spike_evidence_schema.json")
    dxil_b_graphics_sig_schema = load(ROOT / "milestones/g2/dxil_b_graphics_sig_evidence_schema.json")
    dxil_b_strict_only_schema = load(ROOT / "milestones/g2/dxil_b_strict_only_evidence_schema.json")
    dxil_a_graphics_sig_effort_schema = load(
        ROOT / "milestones/g2/dxil_a_graphics_sig_effort_evidence_schema.json"
    )
    rd017_varying_semantic_spike_schema = load(
        ROOT / "milestones/g2/rd017_varying_semantic_spike_evidence_schema.json"
    )
    host_orch_smoke_schema = load(
        ROOT / "milestones/ms1/host_orch_smoke_evidence_schema.json"
    )
    uc07_offline_golden_schema = load(
        ROOT / "milestones/ms1/uc07_offline_golden_evidence_schema.json"
    )
    uc07_present_schema = load(
        ROOT / "milestones/ms1/uc07_present_evidence_schema.json"
    )
    uc07_bench_schema = load(
        ROOT / "milestones/ms1/uc07_bench_evidence_schema.json"
    )
    ea1_real_endpoint_e2e_schema = load(
        ROOT / "milestones/ea1/real_endpoint_e2e_evidence_schema.json"
    )
    ea1_install_e2e_schema = load(
        ROOT / "milestones/ea1/install_e2e_evidence_schema.json"
    )
    rd027_pt_poison_spike_schema = load(
        ROOT / "milestones/g3/rd027_spike_evidence_schema.json"
    )
    uc04_present_schema = load(
        ROOT / "milestones/g3/uc04_present_evidence_schema.json"
    )
    sampling_superset_schema = load(
        ROOT / "milestones/g3/sampling_superset_evidence_schema.json"
    )
    bindless_smoke_schema = load(
        ROOT / "milestones/g3/bindless_descriptor_smoke_evidence_schema.json"
    )
    auto_barrier_hazard_schema = load(
        ROOT / "milestones/g3/auto_barrier_hazard_evidence_schema.json"
    )
    meshrt_stages_schema = load(
        ROOT / "milestones/g3/meshrt_stages_evidence_schema.json"
    )
    uc05_check_bench_schema = load(
        ROOT / "milestones/ei1/uc05_check_bench_evidence_schema.json"
    )
    export_c_smoke_schema = load(
        ROOT / "milestones/ei1/export_c_smoke_evidence_schema.json"
    )
    uc05_invariant_matrix_schema = load(
        ROOT / "milestones/ei1/uc05_invariant_matrix_schema.json"
    )
    uc05_rhi_smoke_schema = load(
        ROOT / "milestones/ei1/uc05_rhi_smoke_evidence_schema.json"
    )
    uc05_engine_embed_schema = load(
        ROOT / "milestones/ei1/uc05_engine_embed_evidence_schema.json"
    )
    uc05_engine_embed_v3_schema = load(
        ROOT / "milestones/g4/uc05_engine_embed_v3_evidence_schema.json"
    )
    uc05_exec_face_gate_schema = load(
        ROOT / "milestones/g4/uc05_exec_face_gate_evidence_schema.json"
    )
    uc05_graphics_rhi_smoke_schema = load(
        ROOT / "milestones/g4/uc05_graphics_rhi_smoke_evidence_schema.json"
    )
    vulkan_rhi_channel_smoke_schema = load(
        ROOT / "milestones/g4/vulkan_rhi_channel_smoke_evidence_schema.json"
    )
    blackhole_realtime_smoke_schema = load(
        ROOT / "milestones/g4/blackhole_realtime_smoke_evidence_schema.json"
    )
    renderer_graph_smoke_schema = load(
        ROOT / "milestones/g5/renderer_graph_smoke_evidence_schema.json"
    )
    renderer_draw_smoke_schema = load(
        ROOT / "milestones/g5/renderer_draw_smoke_evidence_schema.json"
    )
    renderer_visbuffer_smoke_schema = load(
        ROOT / "milestones/g5/renderer_visbuffer_smoke_evidence_schema.json"
    )
    renderer_lighting_smoke_schema = load(
        ROOT / "milestones/g5/renderer_lighting_smoke_evidence_schema.json"
    )
    renderer_temporal_smoke_schema = load(
        ROOT / "milestones/g5/renderer_temporal_smoke_evidence_schema.json"
    )
    uc06_renderer_smoke_schema = load(
        ROOT / "milestones/g5/uc06_renderer_smoke_evidence_schema.json"
    )
    physics_core_smoke_schema = load(
        ROOT / "milestones/g6/physics_core_smoke_evidence_schema.json"
    )
    physics_bridge_smoke_schema = load(
        ROOT / "milestones/g6/physics_bridge_smoke_evidence_schema.json"
    )
    uc08_physics_smoke_schema = load(
        ROOT / "milestones/g6/uc08_physics_smoke_evidence_schema.json"
    )
    physics_rapier_parity_schema = load(
        ROOT / "milestones/g6/physics_rapier_parity_evidence_schema.json"
    )
    taichi_vulkan_spike_schema = load(
        ROOT / "milestones/g6/taichi_vulkan_spike_evidence_schema.json"
    )
    g7_baseline_schema = load(
        ROOT / "milestones/g7/g7_baseline_evidence_schema.json"
    )
    g7_perf_baseline_schema = load(
        ROOT / "milestones/g7/g7_perf_baseline_evidence_schema.json"
    )
    ray_query_codegen_schema = load(
        ROOT / "milestones/g7/ray_query_codegen_evidence_schema.json"
    )
    renderer_raster_diff_schema = load(
        ROOT / "milestones/g7/renderer_raster_diff_evidence_schema.json"
    )
    renderer_w3_schema = load(
        ROOT / "milestones/g7/renderer_w3_evidence_schema.json"
    )
    renderer_device_frame_schema = load(
        ROOT / "milestones/g7/renderer_device_frame_evidence_schema.json"
    )
    renderer_soak_schema = load(
        ROOT / "milestones/g7/renderer_soak_evidence_schema.json"
    )
    g8_perf_baseline_schema = load(
        ROOT / "milestones/g8/g8_perf_baseline_evidence_schema.json"
    )
    g8_m31_reflection_hash_schema = load(
        ROOT / "milestones/g8/g8_m31_reflection_hash_evidence_schema.json"
    )
    g8_m29_shader_permutation_schema = load(
        ROOT / "milestones/g8/g8_m29_shader_permutation_evidence_schema.json"
    )
    g8_m32_capability_profile_schema = load(
        ROOT / "milestones/g8/g8_m32_capability_profile_evidence_schema.json"
    )
    g8_m30_pso_cache_schema = load(
        ROOT / "milestones/g8/g8_m30_pso_cache_evidence_schema.json"
    )
    g8_m85_shader_manifest_ddc_schema = load(
        ROOT / "milestones/g8/g8_m85_shader_manifest_ddc_evidence_schema.json"
    )
    g8_m89_single_source_gfx_submit_schema = load(
        ROOT / "milestones/g8/g8_m89_single_source_gfx_submit_evidence_schema.json"
    )
    g8_m50_rt_pipeline_incremental_schema = load(
        ROOT / "milestones/g8/g8_m50_rt_pipeline_incremental_evidence_schema.json"
    )
    g8_wave2_exit_schema = load(
        ROOT / "milestones/g8/g8_wave2_exit_evidence_schema.json"
    )
    g8_wave3_exit_schema = load(
        ROOT / "milestones/g8/g8_wave3_exit_evidence_schema.json"
    )
    g8_m01_meshlet_page_builder_schema = load(
        ROOT / "milestones/g8/g8_m01_meshlet_page_builder_evidence_schema.json"
    )
    g8_m83_texture_transcode_schema = load(
        ROOT / "milestones/g8/g8_m83_texture_transcode_evidence_schema.json"
    )
    g8_m81_gltf_import_schema = load(
        ROOT / "milestones/g8/g8_m81_gltf_import_evidence_schema.json"
    )
    g8_m79_asset_determinism_schema = load(
        ROOT / "milestones/g8/g8_m79_asset_determinism_evidence_schema.json"
    )
    g8_m04_page_format_abi_schema = load(
        ROOT / "milestones/g8/g8_m04_page_format_abi_evidence_schema.json"
    )
    g8_m80_ddc_content_address_schema = load(
        ROOT / "milestones/g8/g8_m80_ddc_content_address_evidence_schema.json"
    )
    g8_m37_streaming_io_schema = load(
        ROOT / "milestones/g8/g8_m37_streaming_io_evidence_schema.json"
    )
    g8_gate_geom_page_schema = load(
        ROOT / "milestones/g8/g8_gate_geom_page_evidence_schema.json"
    )
    g8_wave4_exit_schema = load(
        ROOT / "milestones/g8/g8_wave4_exit_evidence_schema.json"
    )
    g8_m19_vsm_page_cache_schema = load(
        ROOT / "milestones/g8/g8_m19_vsm_page_cache_evidence_schema.json"
    )
    g8_wave5a_exit_schema = load(
        ROOT / "milestones/g8/g8_wave5a_exit_evidence_schema.json"
    )
    g8_m24_tsr_contract_schema = load(
        ROOT / "milestones/g8/g8_m24_tsr_contract_evidence_schema.json"
    )
    g8_m25_upscaler_input_abi_schema = load(
        ROOT / "milestones/g8/g8_m25_upscaler_input_abi_evidence_schema.json"
    )
    g8_wave5b_exit_schema = load(
        ROOT / "milestones/g8/g8_wave5b_exit_evidence_schema.json"
    )
    g8_m66_physics_replay_schema = load(
        ROOT / "milestones/g8/g8_m66_physics_replay_evidence_schema.json"
    )
    g8_wave6a_exit_schema = load(
        ROOT / "milestones/g8/g8_wave6a_exit_evidence_schema.json"
    )
    g8_m67_network_physics_schema = load(
        ROOT / "milestones/g8/g8_m67_network_physics_evidence_schema.json"
    )
    g8_wave6b_exit_schema = load(
        ROOT / "milestones/g8/g8_wave6b_exit_evidence_schema.json"
    )
    g8_m68_fracture_pipeline_schema = load(
        ROOT / "milestones/g8/g8_m68_fracture_pipeline_evidence_schema.json"
    )
    g8_wave6c_exit_schema = load(
        ROOT / "milestones/g8/g8_wave6c_exit_evidence_schema.json"
    )
    g8_m72_cloth_product_chain_schema = load(
        ROOT / "milestones/g8/g8_m72_cloth_product_chain_evidence_schema.json"
    )
    g8_wave6d_exit_schema = load(
        ROOT / "milestones/g8/g8_wave6d_exit_evidence_schema.json"
    )
    g8_wave8a_soak_schema = load(
        ROOT / "milestones/g8/g8_wave8a_soak_evidence_schema.json"
    )
    g8_wave8b_closeout_schema = load(
        ROOT / "milestones/g8/g8_wave8b_closeout_evidence_schema.json"
    )
    g8_wave7_decisions_schema = load(
        ROOT / "milestones/g8/g8_wave7_decisions_evidence_schema.json"
    )
    g9_vram_as_baseline_schema = load(
        ROOT / "milestones/g9/g9_vram_as_baseline_evidence_schema.json"
    )
    g9_m121_physics_particle_view_schema = load(
        ROOT / "milestones/g9/g9_m121_physics_particle_view_evidence_schema.json"
    )
    g9_m122_gameplay_field_schema = load(
        ROOT / "milestones/g9/g9_m122_gameplay_field_evidence_schema.json"
    )
    g9_wave2_exit_schema = load(
        ROOT / "milestones/g9/g9_wave2_exit_evidence_schema.json"
    )
    g9_wave3_exit_schema = load(
        ROOT / "milestones/g9/g9_wave3_exit_evidence_schema.json"
    )
    g9_wave4_exit_schema = load(
        ROOT / "milestones/g9/g9_wave4_exit_evidence_schema.json"
    )
    g9_wave5_exit_schema = load(
        ROOT / "milestones/g9/g9_wave5_exit_evidence_schema.json"
    )
    g9_wave6_exit_schema = load(
        ROOT / "milestones/g9/g9_wave6_exit_evidence_schema.json"
    )
    g9_p2_decisions_schema = load(
        ROOT / "milestones/g9/g9_p2_decisions_evidence_schema.json"
    )
    g9_stabilization_soak_schema = load(
        ROOT / "milestones/g9/g9_stabilization_soak_evidence_schema.json"
    )
    g9_wave8b_closeout_schema = load(
        ROOT / "milestones/g9/g9_wave8b_closeout_evidence_schema.json"
    )
    g10_m131_asset_license_registry_schema = load(
        ROOT / "milestones/g10/g10_m131_asset_license_registry_evidence_schema.json"
    )
    g10_m132_corpus_loading_schema = load(
        ROOT / "milestones/g10/g10_m132_corpus_loading_evidence_schema.json"
    )
    g10_m133_corpus_list_freeze_schema = load(
        ROOT / "milestones/g10/g10_m133_corpus_list_freeze_evidence_schema.json"
    )
    g10_wave3_exit_schema = load(
        ROOT / "milestones/g10/g10_wave3_exit_evidence_schema.json"
    )
    g10_m128_ue5_capture_environment_schema = load(
        ROOT / "milestones/g10/g10_m128_ue5_capture_environment_evidence_schema.json"
    )
    g10_m129_ue5_reference_frames_schema = load(
        ROOT / "milestones/g10/g10_m129_ue5_reference_frames_evidence_schema.json"
    )
    g10_m130_dual_determinism_contract_schema = load(
        ROOT / "milestones/g10/g10_m130_dual_determinism_contract_evidence_schema.json"
    )
    g10_wave2_exit_schema = load(
        ROOT / "milestones/g10/g10_wave2_exit_evidence_schema.json"
    )
    g9_m102_dgc_abstraction_schema = load(
        ROOT / "milestones/g9/g9_m102_dgc_abstraction_evidence_schema.json"
    )
    g9_m103_descriptor_global_table_schema = load(
        ROOT / "milestones/g9/g9_m103_descriptor_global_table_evidence_schema.json"
    )
    g9_m104_accesskind_indirect_edge_schema = load(
        ROOT / "milestones/g9/g9_m104_accesskind_indirect_edge_evidence_schema.json"
    )
    g9_m90_cluster_dag_deepening_schema = load(
        ROOT / "milestones/g9/g9_m90_cluster_dag_deepening_evidence_schema.json"
    )
    g9_m91_page_format_v2_abi_schema = load(
        ROOT / "milestones/g9/g9_m91_page_format_v2_abi_evidence_schema.json"
    )
    g9_m93_visible_cluster_set_schema = load(
        ROOT / "milestones/g9/g9_m93_visible_cluster_set_evidence_schema.json"
    )
    g9_m94_clas_rt_convergence_schema = load(
        ROOT / "milestones/g9/g9_m94_clas_rt_convergence_evidence_schema.json"
    )
    g9_m95_single_source_truth_schema = load(
        ROOT / "milestones/g9/g9_m95_single_source_truth_evidence_schema.json"
    )
    g9_m92_gpu_skinning_lod_update_schema = load(
        ROOT / "milestones/g9/g9_m92_gpu_skinning_lod_update_evidence_schema.json"
    )
    g9_m105_command_build_node_schema = load(
        ROOT / "milestones/g9/g9_m105_command_build_node_evidence_schema.json"
    )
    g9_m106_execution_set_pso_schema = load(
        ROOT / "milestones/g9/g9_m106_execution_set_pso_evidence_schema.json"
    )
    g9_m107_shader_library_ir_link_schema = load(
        ROOT / "milestones/g9/g9_m107_shader_library_ir_link_evidence_schema.json"
    )
    g9_m96_path_tracer_reference_schema = load(
        ROOT / "milestones/g9/g9_m96_path_tracer_reference_evidence_schema.json"
    )
    g9_m97_surface_cache_schema = load(
        ROOT / "milestones/g9/g9_m97_surface_cache_evidence_schema.json"
    )
    g9_m98_tracing_fallback_chain_schema = load(
        ROOT / "milestones/g9/g9_m98_tracing_fallback_chain_evidence_schema.json"
    )
    g9_m99_spg_radiance_cache_schema = load(
        ROOT / "milestones/g9/g9_m99_spg_radiance_cache_evidence_schema.json"
    )
    g9_m100_multi_light_low_schema = load(
        ROOT / "milestones/g9/g9_m100_multi_light_low_evidence_schema.json"
    )
    g9_m101_if_tier_ladder_schema = load(
        ROOT / "milestones/g9/g9_m101_if_tier_ladder_evidence_schema.json"
    )
    g9_gi_harness_schema = load(
        ROOT / "milestones/g9/g9_gi_harness_evidence_schema.json"
    )
    g9_world_harness_schema = load(
        ROOT / "milestones/g9/g9_world_harness_evidence_schema.json"
    )
    g9_m110_world_partition_schema = load(
        ROOT / "milestones/g9/g9_m110_world_partition_evidence_schema.json"
    )
    g9_m118_display_pipeline_view_transform_schema = load(
        ROOT / "milestones/g9/g9_m118_display_pipeline_view_transform_evidence_schema.json"
    )
    g9_m111_hlod_baking_schema = load(
        ROOT / "milestones/g9/g9_m111_hlod_baking_evidence_schema.json"
    )
    g9_m112_atmosphere_froxel_schema = load(
        ROOT / "milestones/g9/g9_m112_atmosphere_froxel_evidence_schema.json"
    )
    g9_m113_water_dual_pipeline_schema = load(
        ROOT / "milestones/g9/g9_m113_water_dual_pipeline_evidence_schema.json"
    )
    g9_m114_hair_marschner_schema = load(
        ROOT / "milestones/g9/g9_m114_hair_marschner_evidence_schema.json"
    )
    g9_m115_skin_burley_diffusion_schema = load(
        ROOT / "milestones/g9/g9_m115_skin_burley_diffusion_evidence_schema.json"
    )
    g9_m116_terrain_chunk_cell_schema = load(
        ROOT / "milestones/g9/g9_m116_terrain_chunk_cell_evidence_schema.json"
    )
    g9_m117_decal_dbuffer_schema = load(
        ROOT / "milestones/g9/g9_m117_decal_dbuffer_evidence_schema.json"
    )
    g9_m119_post_processing_skeleton_schema = load(
        ROOT / "milestones/g9/g9_m119_post_processing_skeleton_evidence_schema.json"
    )
    g9_m120_oit_benchmark_harness_schema = load(
        ROOT / "milestones/g9/g9_m120_oit_benchmark_harness_evidence_schema.json"
    )
    g9_m124_buoyancy_field_channel_schema = load(
        ROOT / "milestones/g9/g9_m124_buoyancy_field_channel_evidence_schema.json"
    )
    g9_m126_rapier_benchmark_ab_schema = load(
        ROOT / "milestones/g9/g9_m126_rapier_benchmark_ab_evidence_schema.json"
    )
    g9_m125_jolt_56_ab_evaluation_schema = load(
        ROOT / "milestones/g9/g9_m125_jolt_56_ab_evaluation_evidence_schema.json"
    )
    g10_baseline_schema = load(
        ROOT / "milestones/g10/g10_baseline_evidence_schema.json"
    )
    if (gpu_schema is None or frontend_schema is None or compile_schema is None
            or sanitizer_schema is None or redistribution_schema is None
            or rx_cli_smoke_schema is None or offline_rebuild_schema is None
            or lsp_smoke_schema is None or lsp_latency_schema is None
            or stdlib_math_schema is None or soft_raster_schema is None
            or uc03_demo_schema is None or uc01_interop_schema is None
            or cublas_binding_schema is None or uc02_stream_pipeline_schema is None
            or release_schema is None or bilingual_schema is None
            or doc_site_schema is None):
        return
    evidence_files = sorted((ROOT / "evidence").glob("*.json"))
    if not evidence_files:
        print("[check_schemas] evidence/ 暂无证据文件(M0.3 前为正常状态)")
        return
    try:
        import jsonschema
    except ImportError:
        err("缺 jsonschema 依赖(pip install -r requirements.txt)")
        return
    gpu_validator = jsonschema.Draft7Validator(gpu_schema)
    frontend_validator = jsonschema.Draft7Validator(frontend_schema)
    compile_validator = jsonschema.Draft7Validator(compile_schema)
    sanitizer_validator = jsonschema.Draft7Validator(sanitizer_schema)
    redistribution_validator = jsonschema.Draft7Validator(redistribution_schema)
    rx_cli_smoke_validator = jsonschema.Draft7Validator(rx_cli_smoke_schema)
    offline_rebuild_validator = jsonschema.Draft7Validator(offline_rebuild_schema)
    lsp_smoke_validator = jsonschema.Draft7Validator(lsp_smoke_schema)
    lsp_latency_validator = jsonschema.Draft7Validator(lsp_latency_schema)
    stdlib_math_validator = jsonschema.Draft7Validator(stdlib_math_schema)
    soft_raster_validator = jsonschema.Draft7Validator(soft_raster_schema)
    uc03_demo_validator = jsonschema.Draft7Validator(uc03_demo_schema)
    uc01_interop_validator = jsonschema.Draft7Validator(uc01_interop_schema)
    cublas_binding_validator = jsonschema.Draft7Validator(cublas_binding_schema)
    uc02_stream_pipeline_validator = jsonschema.Draft7Validator(uc02_stream_pipeline_schema)
    release_validator = jsonschema.Draft7Validator(release_schema)
    bilingual_validator = jsonschema.Draft7Validator(bilingual_schema)
    doc_site_validator = jsonschema.Draft7Validator(doc_site_schema)
    d3d12_interop_validator = jsonschema.Draft7Validator(d3d12_interop_schema)
    realtime_present_validator = jsonschema.Draft7Validator(realtime_present_schema)
    async_buffer_validator = (
        jsonschema.Draft7Validator(async_buffer_schema) if async_buffer_schema else None
    )
    engine_integration_validator = (
        jsonschema.Draft7Validator(engine_integration_schema)
        if engine_integration_schema
        else None
    )
    fatbin_dist_validator = (
        jsonschema.Draft7Validator(fatbin_dist_schema) if fatbin_dist_schema else None
    )
    dxil_path_spike_validator = (
        jsonschema.Draft7Validator(dxil_path_spike_schema) if dxil_path_spike_schema else None
    )
    dxil_b_graphics_sig_validator = (
        jsonschema.Draft7Validator(dxil_b_graphics_sig_schema)
        if dxil_b_graphics_sig_schema
        else None
    )
    dxil_b_strict_only_validator = (
        jsonschema.Draft7Validator(dxil_b_strict_only_schema)
        if dxil_b_strict_only_schema
        else None
    )
    dxil_a_graphics_sig_effort_validator = (
        jsonschema.Draft7Validator(dxil_a_graphics_sig_effort_schema)
        if dxil_a_graphics_sig_effort_schema
        else None
    )
    rd017_varying_semantic_spike_validator = (
        jsonschema.Draft7Validator(rd017_varying_semantic_spike_schema)
        if rd017_varying_semantic_spike_schema
        else None
    )
    host_orch_smoke_validator = (
        jsonschema.Draft7Validator(host_orch_smoke_schema)
        if host_orch_smoke_schema
        else None
    )
    uc07_offline_golden_validator = (
        jsonschema.Draft7Validator(uc07_offline_golden_schema)
        if uc07_offline_golden_schema
        else None
    )
    uc07_present_validator = (
        jsonschema.Draft7Validator(uc07_present_schema)
        if uc07_present_schema
        else None
    )
    uc07_bench_validator = (
        jsonschema.Draft7Validator(uc07_bench_schema)
        if uc07_bench_schema
        else None
    )
    ea1_real_endpoint_e2e_validator = (
        jsonschema.Draft7Validator(ea1_real_endpoint_e2e_schema)
        if ea1_real_endpoint_e2e_schema
        else None
    )
    rd027_pt_poison_spike_validator = (
        jsonschema.Draft7Validator(rd027_pt_poison_spike_schema)
        if rd027_pt_poison_spike_schema
        else None
    )
    uc04_present_validator = (
        jsonschema.Draft7Validator(uc04_present_schema) if uc04_present_schema else None
    )
    sampling_superset_validator = (
        jsonschema.Draft7Validator(sampling_superset_schema)
        if sampling_superset_schema
        else None
    )
    auto_barrier_hazard_validator = (
        jsonschema.Draft7Validator(auto_barrier_hazard_schema)
        if auto_barrier_hazard_schema
        else None
    )
    meshrt_stages_validator = (
        jsonschema.Draft7Validator(meshrt_stages_schema) if meshrt_stages_schema else None
    )
    export_c_smoke_validator = (
        jsonschema.Draft7Validator(export_c_smoke_schema) if export_c_smoke_schema else None
    )
    uc05_invariant_matrix_validator = (
        jsonschema.Draft7Validator(uc05_invariant_matrix_schema)
        if uc05_invariant_matrix_schema
        else None
    )
    uc05_rhi_smoke_validator = (
        jsonschema.Draft7Validator(uc05_rhi_smoke_schema) if uc05_rhi_smoke_schema else None
    )
    uc05_engine_embed_validator = (
        jsonschema.Draft7Validator(uc05_engine_embed_schema) if uc05_engine_embed_schema else None
    )
    uc05_engine_embed_v3_validator = (
        jsonschema.Draft7Validator(uc05_engine_embed_v3_schema) if uc05_engine_embed_v3_schema else None
    )
    uc05_exec_face_gate_validator = (
        jsonschema.Draft7Validator(uc05_exec_face_gate_schema)
        if uc05_exec_face_gate_schema
        else None
    )
    uc05_graphics_rhi_smoke_validator = (
        jsonschema.Draft7Validator(uc05_graphics_rhi_smoke_schema)
        if uc05_graphics_rhi_smoke_schema
        else None
    )
    vulkan_rhi_channel_smoke_validator = (
        jsonschema.Draft7Validator(vulkan_rhi_channel_smoke_schema)
        if vulkan_rhi_channel_smoke_schema
        else None
    )
    blackhole_realtime_smoke_validator = (
        jsonschema.Draft7Validator(blackhole_realtime_smoke_schema)
        if blackhole_realtime_smoke_schema
        else None
    )
    renderer_graph_smoke_validator = (
        jsonschema.Draft7Validator(renderer_graph_smoke_schema)
        if renderer_graph_smoke_schema
        else None
    )
    renderer_draw_smoke_validator = (
        jsonschema.Draft7Validator(renderer_draw_smoke_schema)
        if renderer_draw_smoke_schema
        else None
    )
    renderer_visbuffer_smoke_validator = (
        jsonschema.Draft7Validator(renderer_visbuffer_smoke_schema)
        if renderer_visbuffer_smoke_schema
        else None
    )
    renderer_lighting_smoke_validator = (
        jsonschema.Draft7Validator(renderer_lighting_smoke_schema)
        if renderer_lighting_smoke_schema
        else None
    )
    renderer_temporal_smoke_validator = (
        jsonschema.Draft7Validator(renderer_temporal_smoke_schema)
        if renderer_temporal_smoke_schema
        else None
    )
    uc06_renderer_smoke_validator = (
        jsonschema.Draft7Validator(uc06_renderer_smoke_schema)
        if uc06_renderer_smoke_schema
        else None
    )
    physics_core_smoke_validator = (
        jsonschema.Draft7Validator(physics_core_smoke_schema)
        if physics_core_smoke_schema
        else None
    )
    physics_bridge_smoke_validator = (
        jsonschema.Draft7Validator(physics_bridge_smoke_schema)
        if physics_bridge_smoke_schema
        else None
    )
    uc08_physics_smoke_validator = (
        jsonschema.Draft7Validator(uc08_physics_smoke_schema)
        if uc08_physics_smoke_schema
        else None
    )
    physics_rapier_parity_validator = (
        jsonschema.Draft7Validator(physics_rapier_parity_schema)
        if physics_rapier_parity_schema
        else None
    )
    taichi_vulkan_spike_validator = (
        jsonschema.Draft7Validator(taichi_vulkan_spike_schema)
        if taichi_vulkan_spike_schema is not None
        else None
    )
    g7_baseline_validator = (
        jsonschema.Draft7Validator(g7_baseline_schema)
        if g7_baseline_schema is not None
        else None
    )
    g7_perf_baseline_validator = (
        jsonschema.Draft7Validator(g7_perf_baseline_schema)
        if g7_perf_baseline_schema is not None
        else None
    )
    renderer_raster_diff_validator = (
        jsonschema.Draft7Validator(renderer_raster_diff_schema)
        if renderer_raster_diff_schema is not None
        else None
    )
    renderer_w3_validator = (
        jsonschema.Draft7Validator(renderer_w3_schema)
        if renderer_w3_schema is not None
        else None
    )
    renderer_device_frame_validator = (
        jsonschema.Draft7Validator(renderer_device_frame_schema)
        if renderer_device_frame_schema is not None
        else None
    )
    renderer_soak_validator = (
        jsonschema.Draft7Validator(renderer_soak_schema)
        if renderer_soak_schema is not None
        else None
    )
    ray_query_codegen_validator = (
        jsonschema.Draft7Validator(ray_query_codegen_schema)
        if ray_query_codegen_schema is not None
        else None
    )
    g8_perf_baseline_validator = (
        jsonschema.Draft7Validator(g8_perf_baseline_schema)
        if g8_perf_baseline_schema is not None
        else None
    )
    g8_m31_reflection_hash_validator = (
        jsonschema.Draft7Validator(g8_m31_reflection_hash_schema)
        if g8_m31_reflection_hash_schema is not None
        else None
    )
    g8_m29_shader_permutation_validator = (
        jsonschema.Draft7Validator(g8_m29_shader_permutation_schema)
        if g8_m29_shader_permutation_schema is not None
        else None
    )
    g8_m32_capability_profile_validator = (
        jsonschema.Draft7Validator(g8_m32_capability_profile_schema)
        if g8_m32_capability_profile_schema is not None
        else None
    )
    g8_m30_pso_cache_validator = (
        jsonschema.Draft7Validator(g8_m30_pso_cache_schema)
        if g8_m30_pso_cache_schema is not None
        else None
    )
    g8_m85_shader_manifest_ddc_validator = (
        jsonschema.Draft7Validator(g8_m85_shader_manifest_ddc_schema)
        if g8_m85_shader_manifest_ddc_schema is not None
        else None
    )
    g8_m89_single_source_gfx_submit_validator = (
        jsonschema.Draft7Validator(g8_m89_single_source_gfx_submit_schema)
        if g8_m89_single_source_gfx_submit_schema is not None
        else None
    )
    g8_m50_rt_pipeline_incremental_validator = (
        jsonschema.Draft7Validator(g8_m50_rt_pipeline_incremental_schema)
        if g8_m50_rt_pipeline_incremental_schema is not None
        else None
    )
    g8_wave2_exit_validator = (
        jsonschema.Draft7Validator(g8_wave2_exit_schema)
        if g8_wave2_exit_schema is not None
        else None
    )
    g8_wave3_exit_validator = (
        jsonschema.Draft7Validator(g8_wave3_exit_schema)
        if g8_wave3_exit_schema is not None
        else None
    )
    g8_m01_meshlet_page_builder_validator = (
        jsonschema.Draft7Validator(g8_m01_meshlet_page_builder_schema)
        if g8_m01_meshlet_page_builder_schema is not None
        else None
    )
    g8_m83_texture_transcode_validator = (
        jsonschema.Draft7Validator(g8_m83_texture_transcode_schema)
        if g8_m83_texture_transcode_schema is not None
        else None
    )
    g8_m81_gltf_import_validator = (
        jsonschema.Draft7Validator(g8_m81_gltf_import_schema)
        if g8_m81_gltf_import_schema is not None
        else None
    )
    g8_m79_asset_determinism_validator = (
        jsonschema.Draft7Validator(g8_m79_asset_determinism_schema)
        if g8_m79_asset_determinism_schema is not None
        else None
    )
    g8_m04_page_format_abi_validator = (
        jsonschema.Draft7Validator(g8_m04_page_format_abi_schema)
        if g8_m04_page_format_abi_schema is not None
        else None
    )
    g8_m80_ddc_content_address_validator = (
        jsonschema.Draft7Validator(g8_m80_ddc_content_address_schema)
        if g8_m80_ddc_content_address_schema is not None
        else None
    )
    g8_m37_streaming_io_validator = (
        jsonschema.Draft7Validator(g8_m37_streaming_io_schema)
        if g8_m37_streaming_io_schema is not None
        else None
    )
    g8_gate_geom_page_validator = (
        jsonschema.Draft7Validator(g8_gate_geom_page_schema)
        if g8_gate_geom_page_schema is not None
        else None
    )
    g8_wave4_exit_validator = (
        jsonschema.Draft7Validator(g8_wave4_exit_schema)
        if g8_wave4_exit_schema is not None
        else None
    )
    g8_m19_vsm_page_cache_validator = (
        jsonschema.Draft7Validator(g8_m19_vsm_page_cache_schema)
        if g8_m19_vsm_page_cache_schema is not None
        else None
    )
    g8_wave5a_exit_validator = (
        jsonschema.Draft7Validator(g8_wave5a_exit_schema)
        if g8_wave5a_exit_schema is not None
        else None
    )
    g8_m24_tsr_contract_validator = (
        jsonschema.Draft7Validator(g8_m24_tsr_contract_schema)
        if g8_m24_tsr_contract_schema is not None
        else None
    )
    g8_m25_upscaler_input_abi_validator = (
        jsonschema.Draft7Validator(g8_m25_upscaler_input_abi_schema)
        if g8_m25_upscaler_input_abi_schema is not None
        else None
    )
    g8_wave5b_exit_validator = (
        jsonschema.Draft7Validator(g8_wave5b_exit_schema)
        if g8_wave5b_exit_schema is not None
        else None
    )
    g8_m66_physics_replay_validator = (
        jsonschema.Draft7Validator(g8_m66_physics_replay_schema)
        if g8_m66_physics_replay_schema is not None
        else None
    )
    g8_wave6a_exit_validator = (
        jsonschema.Draft7Validator(g8_wave6a_exit_schema)
        if g8_wave6a_exit_schema is not None
        else None
    )
    g8_m67_network_physics_validator = (
        jsonschema.Draft7Validator(g8_m67_network_physics_schema)
        if g8_m67_network_physics_schema is not None
        else None
    )
    g8_wave6b_exit_validator = (
        jsonschema.Draft7Validator(g8_wave6b_exit_schema)
        if g8_wave6b_exit_schema is not None
        else None
    )
    g8_m68_fracture_pipeline_validator = (
        jsonschema.Draft7Validator(g8_m68_fracture_pipeline_schema)
        if g8_m68_fracture_pipeline_schema is not None
        else None
    )
    g8_wave6c_exit_validator = (
        jsonschema.Draft7Validator(g8_wave6c_exit_schema)
        if g8_wave6c_exit_schema is not None
        else None
    )
    g8_m72_cloth_product_chain_validator = (
        jsonschema.Draft7Validator(g8_m72_cloth_product_chain_schema)
        if g8_m72_cloth_product_chain_schema is not None
        else None
    )
    g8_wave6d_exit_validator = (
        jsonschema.Draft7Validator(g8_wave6d_exit_schema)
        if g8_wave6d_exit_schema is not None
        else None
    )
    g8_wave8a_soak_validator = (
        jsonschema.Draft7Validator(g8_wave8a_soak_schema)
        if g8_wave8a_soak_schema is not None
        else None
    )
    g8_wave8b_closeout_validator = (
        jsonschema.Draft7Validator(g8_wave8b_closeout_schema)
        if g8_wave8b_closeout_schema is not None
        else None
    )
    g8_wave7_decisions_validator = (
        jsonschema.Draft7Validator(g8_wave7_decisions_schema)
        if g8_wave7_decisions_schema is not None
        else None
    )
    g9_vram_as_baseline_validator = (
        jsonschema.Draft7Validator(g9_vram_as_baseline_schema)
        if g9_vram_as_baseline_schema is not None
        else None
    )
    g9_m121_physics_particle_view_validator = (
        jsonschema.Draft7Validator(g9_m121_physics_particle_view_schema)
        if g9_m121_physics_particle_view_schema is not None
        else None
    )
    g9_m122_gameplay_field_validator = (
        jsonschema.Draft7Validator(g9_m122_gameplay_field_schema)
        if g9_m122_gameplay_field_schema is not None
        else None
    )
    g9_wave2_exit_validator = (
        jsonschema.Draft7Validator(g9_wave2_exit_schema)
        if g9_wave2_exit_schema is not None
        else None
    )
    g9_wave3_exit_validator = (
        jsonschema.Draft7Validator(g9_wave3_exit_schema)
        if g9_wave3_exit_schema is not None
        else None
    )
    g9_wave4_exit_validator = (
        jsonschema.Draft7Validator(g9_wave4_exit_schema)
        if g9_wave4_exit_schema is not None
        else None
    )
    g9_wave5_exit_validator = (
        jsonschema.Draft7Validator(g9_wave5_exit_schema)
        if g9_wave5_exit_schema is not None
        else None
    )
    g9_wave6_exit_validator = (
        jsonschema.Draft7Validator(g9_wave6_exit_schema)
        if g9_wave6_exit_schema is not None
        else None
    )
    g9_p2_decisions_validator = (
        jsonschema.Draft7Validator(g9_p2_decisions_schema)
        if g9_p2_decisions_schema is not None
        else None
    )
    g9_stabilization_soak_validator = (
        jsonschema.Draft7Validator(g9_stabilization_soak_schema)
        if g9_stabilization_soak_schema is not None
        else None
    )
    g9_wave8b_closeout_validator = (
        jsonschema.Draft7Validator(g9_wave8b_closeout_schema)
        if g9_wave8b_closeout_schema is not None
        else None
    )
    g10_m131_asset_license_registry_validator = (
        jsonschema.Draft7Validator(g10_m131_asset_license_registry_schema)
        if g10_m131_asset_license_registry_schema is not None
        else None
    )
    g10_m132_corpus_loading_validator = (
        jsonschema.Draft7Validator(g10_m132_corpus_loading_schema)
        if g10_m132_corpus_loading_schema is not None
        else None
    )
    g10_m133_corpus_list_freeze_validator = (
        jsonschema.Draft7Validator(g10_m133_corpus_list_freeze_schema)
        if g10_m133_corpus_list_freeze_schema is not None
        else None
    )
    g10_wave3_exit_validator = (
        jsonschema.Draft7Validator(g10_wave3_exit_schema)
        if g10_wave3_exit_schema is not None
        else None
    )
    g10_m128_ue5_capture_environment_validator = (
        jsonschema.Draft7Validator(g10_m128_ue5_capture_environment_schema)
        if g10_m128_ue5_capture_environment_schema is not None
        else None
    )
    g10_m129_ue5_reference_frames_validator = (
        jsonschema.Draft7Validator(g10_m129_ue5_reference_frames_schema)
        if g10_m129_ue5_reference_frames_schema is not None
        else None
    )
    g10_m130_dual_determinism_contract_validator = (
        jsonschema.Draft7Validator(g10_m130_dual_determinism_contract_schema)
        if g10_m130_dual_determinism_contract_schema is not None
        else None
    )
    g10_wave2_exit_validator = (
        jsonschema.Draft7Validator(g10_wave2_exit_schema)
        if g10_wave2_exit_schema is not None
        else None
    )
    g9_m102_dgc_abstraction_validator = (
        jsonschema.Draft7Validator(g9_m102_dgc_abstraction_schema)
        if g9_m102_dgc_abstraction_schema is not None
        else None
    )
    g9_m103_descriptor_global_table_validator = (
        jsonschema.Draft7Validator(g9_m103_descriptor_global_table_schema)
        if g9_m103_descriptor_global_table_schema is not None
        else None
    )
    g9_m104_accesskind_indirect_edge_validator = (
        jsonschema.Draft7Validator(g9_m104_accesskind_indirect_edge_schema)
        if g9_m104_accesskind_indirect_edge_schema is not None
        else None
    )
    g9_m90_cluster_dag_deepening_validator = (
        jsonschema.Draft7Validator(g9_m90_cluster_dag_deepening_schema)
        if g9_m90_cluster_dag_deepening_schema is not None
        else None
    )
    g9_m91_page_format_v2_abi_validator = (
        jsonschema.Draft7Validator(g9_m91_page_format_v2_abi_schema)
        if g9_m91_page_format_v2_abi_schema is not None
        else None
    )
    g9_m93_visible_cluster_set_validator = (
        jsonschema.Draft7Validator(g9_m93_visible_cluster_set_schema)
        if g9_m93_visible_cluster_set_schema is not None
        else None
    )
    g9_m94_clas_rt_convergence_validator = (
        jsonschema.Draft7Validator(g9_m94_clas_rt_convergence_schema)
        if g9_m94_clas_rt_convergence_schema is not None
        else None
    )
    g9_m95_single_source_truth_validator = (
        jsonschema.Draft7Validator(g9_m95_single_source_truth_schema)
        if g9_m95_single_source_truth_schema is not None
        else None
    )
    g9_m92_gpu_skinning_lod_update_validator = (
        jsonschema.Draft7Validator(g9_m92_gpu_skinning_lod_update_schema)
        if g9_m92_gpu_skinning_lod_update_schema is not None
        else None
    )
    g9_m105_command_build_node_validator = (
        jsonschema.Draft7Validator(g9_m105_command_build_node_schema)
        if g9_m105_command_build_node_schema is not None
        else None
    )
    g9_m106_execution_set_pso_validator = (
        jsonschema.Draft7Validator(g9_m106_execution_set_pso_schema)
        if g9_m106_execution_set_pso_schema is not None
        else None
    )
    g9_m107_shader_library_ir_link_validator = (
        jsonschema.Draft7Validator(g9_m107_shader_library_ir_link_schema)
        if g9_m107_shader_library_ir_link_schema is not None
        else None
    )
    g9_m96_path_tracer_reference_validator = (
        jsonschema.Draft7Validator(g9_m96_path_tracer_reference_schema)
        if g9_m96_path_tracer_reference_schema is not None
        else None
    )
    g9_m97_surface_cache_validator = (
        jsonschema.Draft7Validator(g9_m97_surface_cache_schema)
        if g9_m97_surface_cache_schema is not None
        else None
    )
    g9_m98_tracing_fallback_chain_validator = (
        jsonschema.Draft7Validator(g9_m98_tracing_fallback_chain_schema)
        if g9_m98_tracing_fallback_chain_schema is not None
        else None
    )
    g9_m99_spg_radiance_cache_validator = (
        jsonschema.Draft7Validator(g9_m99_spg_radiance_cache_schema)
        if g9_m99_spg_radiance_cache_schema is not None
        else None
    )
    g9_m100_multi_light_low_validator = (
        jsonschema.Draft7Validator(g9_m100_multi_light_low_schema)
        if g9_m100_multi_light_low_schema is not None
        else None
    )
    g9_m101_if_tier_ladder_validator = (
        jsonschema.Draft7Validator(g9_m101_if_tier_ladder_schema)
        if g9_m101_if_tier_ladder_schema is not None
        else None
    )
    g9_gi_harness_validator = (
        jsonschema.Draft7Validator(g9_gi_harness_schema)
        if g9_gi_harness_schema is not None
        else None
    )
    g9_world_harness_validator = (
        jsonschema.Draft7Validator(g9_world_harness_schema)
        if g9_world_harness_schema is not None
        else None
    )
    g9_m110_world_partition_validator = (
        jsonschema.Draft7Validator(g9_m110_world_partition_schema)
        if g9_m110_world_partition_schema is not None
        else None
    )
    g9_m118_display_pipeline_view_transform_validator = (
        jsonschema.Draft7Validator(g9_m118_display_pipeline_view_transform_schema)
        if g9_m118_display_pipeline_view_transform_schema is not None
        else None
    )
    g9_m111_hlod_baking_validator = (
        jsonschema.Draft7Validator(g9_m111_hlod_baking_schema)
        if g9_m111_hlod_baking_schema is not None
        else None
    )
    g9_m112_atmosphere_froxel_validator = (
        jsonschema.Draft7Validator(g9_m112_atmosphere_froxel_schema)
        if g9_m112_atmosphere_froxel_schema is not None
        else None
    )
    g9_m113_water_dual_pipeline_validator = (
        jsonschema.Draft7Validator(g9_m113_water_dual_pipeline_schema)
        if g9_m113_water_dual_pipeline_schema is not None
        else None
    )
    g9_m114_hair_marschner_validator = (
        jsonschema.Draft7Validator(g9_m114_hair_marschner_schema)
        if g9_m114_hair_marschner_schema is not None
        else None
    )
    g9_m115_skin_burley_diffusion_validator = (
        jsonschema.Draft7Validator(g9_m115_skin_burley_diffusion_schema)
        if g9_m115_skin_burley_diffusion_schema is not None
        else None
    )
    g9_m116_terrain_chunk_cell_validator = (
        jsonschema.Draft7Validator(g9_m116_terrain_chunk_cell_schema)
        if g9_m116_terrain_chunk_cell_schema is not None
        else None
    )
    g9_m117_decal_dbuffer_validator = (
        jsonschema.Draft7Validator(g9_m117_decal_dbuffer_schema)
        if g9_m117_decal_dbuffer_schema is not None
        else None
    )
    g9_m119_post_processing_skeleton_validator = (
        jsonschema.Draft7Validator(g9_m119_post_processing_skeleton_schema)
        if g9_m119_post_processing_skeleton_schema is not None
        else None
    )
    g9_m120_oit_benchmark_harness_validator = (
        jsonschema.Draft7Validator(g9_m120_oit_benchmark_harness_schema)
        if g9_m120_oit_benchmark_harness_schema is not None
        else None
    )
    g9_m124_buoyancy_field_channel_validator = (
        jsonschema.Draft7Validator(g9_m124_buoyancy_field_channel_schema)
        if g9_m124_buoyancy_field_channel_schema is not None
        else None
    )
    g9_m126_rapier_benchmark_ab_validator = (
        jsonschema.Draft7Validator(g9_m126_rapier_benchmark_ab_schema)
        if g9_m126_rapier_benchmark_ab_schema is not None
        else None
    )
    g9_m125_jolt_56_ab_evaluation_validator = (
        jsonschema.Draft7Validator(g9_m125_jolt_56_ab_evaluation_schema)
        if g9_m125_jolt_56_ab_evaluation_schema is not None
        else None
    )
    g10_baseline_validator = (
        jsonschema.Draft7Validator(g10_baseline_schema)
        if g10_baseline_schema is not None
        else None
    )
    uc05_check_bench_validator = (
        jsonschema.Draft7Validator(uc05_check_bench_schema) if uc05_check_bench_schema else None
    )
    bindless_smoke_validator = (
        jsonschema.Draft7Validator(bindless_smoke_schema)
        if bindless_smoke_schema
        else None
    )
    ea1_install_e2e_validator = (
        jsonschema.Draft7Validator(ea1_install_e2e_schema)
        if ea1_install_e2e_schema
        else None
    )
    for f in evidence_files:
        doc = load(f)
        if doc is None:
            continue
        # 路由(按文件名前缀):frontend_ → m1 前端 schema;compile_ → m3 编译
        # schema(G-M3-3 配套);compute_sanitizer_ → m5 Sanitizer schema
        # (G-M5-4 配套);redistribution_audit_ → m5 再分发审计 schema
        # (CI_GATES §4 第 2 项配套);rx_cli_smoke_ → m6 rx CLI 子命令冒烟 schema
        # (G-M6-3 配套);offline_rebuild_ → m6 离线重建复现 schema
        # (G-M6-1 配套);lsp_smoke_ → m6 LSP 能力面冒烟 schema
        # (G-M6-2/G-M6-5 配套);lsp_latency_ → m6 LSP 10k 行交互延迟 schema
        # (G-M6-2 measured_local 配套);stdlib_math_ → m7 core 数学库原语冒烟
        # schema(G-M7-4 配套,m7.counter.math_primitives);soft_raster_ → m7
        # 软光栅 kernel safe 覆盖 + 确定性帧像素冒烟 schema(G-M7-3 配套,
        # m7.counter.soft_raster_kernels_safe);uc03_demo_ → m7 UC-03 demo 单 EXE +
        # 确定性图像序列冒烟 schema(G-M7-1 配套,m7.counter.uc03_demo_image_sequence);
        # uc01_/cublas_/uc02_ → m8 互操作/cublas/UC-02 流水线 schema;release_ → m8
        # 发布链路签名/SBOM/许可审计冒烟 schema(G-M8-4 配套,m8.counter.release_artifacts_signed);
        # bilingual_ → m8 诊断双语全量覆盖 schema(G-M8-5/RD-006 配套,
        # m8.counter.bilingual_diagnostic_coverage);其余 → m0 GPU schema
        if f.name.startswith("frontend_"):
            validator = frontend_validator
        elif f.name.startswith("compile_"):
            validator = compile_validator
        elif f.name.startswith("compute_sanitizer_"):
            validator = sanitizer_validator
        elif f.name.startswith("redistribution_audit_"):
            validator = redistribution_validator
        elif f.name.startswith("rx_cli_smoke_"):
            validator = rx_cli_smoke_validator
        elif f.name.startswith("offline_rebuild_"):
            validator = offline_rebuild_validator
        elif f.name.startswith("lsp_smoke_"):
            validator = lsp_smoke_validator
        elif f.name.startswith("lsp_latency_"):
            validator = lsp_latency_validator
        elif f.name.startswith("stdlib_math_"):
            validator = stdlib_math_validator
        elif f.name.startswith("soft_raster_"):
            validator = soft_raster_validator
        elif f.name.startswith("uc03_demo_"):
            validator = uc03_demo_validator
        elif f.name.startswith("uc01_"):
            validator = uc01_interop_validator
        elif f.name.startswith("cublas_"):
            validator = cublas_binding_validator
        elif f.name.startswith("uc02_"):
            validator = uc02_stream_pipeline_validator
        elif f.name.startswith("release_"):
            validator = release_validator
        elif f.name.startswith("bilingual_"):
            validator = bilingual_validator
        elif f.name.startswith("doc_"):
            validator = doc_site_validator
        elif f.name.startswith("d3d12_interop_"):
            validator = d3d12_interop_validator
        elif f.name.startswith("realtime_present_"):
            validator = realtime_present_validator
        elif f.name.startswith("async_buffer_") and async_buffer_validator is not None:
            validator = async_buffer_validator
        elif (
            f.name.startswith("engine_integration_")
            and engine_integration_validator is not None
        ):
            validator = engine_integration_validator
        elif (
            f.name.startswith("fatbin_dist_")
            and fatbin_dist_validator is not None
        ):
            validator = fatbin_dist_validator
        elif (
            f.name.startswith("dxil_a_graphics_sig_effort_")
            and dxil_a_graphics_sig_effort_validator is not None
        ):
            # G2.2 A 路图形签名工作量评估 spike 证据(RD-010;RFC-0003 §9 Q-D131=A /
            # issue #90504 / #57928)→ milestones/g2/dxil_a_graphics_sig_effort_evidence_schema.json
            # (measured-first / blocked-honest,纯评估 spike 非性能基准;源码勘察 + 上游状态 +
            # 禁区vs conformance 裁断 + 分档工作量 estimated + carry-patch + PoC 锚定;
            # 不入 budget counter,A/B/混合架构结论留 owner)
            validator = dxil_a_graphics_sig_effort_validator
        elif (
            f.name.startswith("dxil_b_strict_only_")
            and dxil_b_strict_only_validator is not None
        ):
            # G2.2 B 路 strict-only 达标取证证据(RD-014;RFC-0004 §4.4 / 04 P-01 / P-13)→
            # milestones/g2/dxil_b_strict_only_evidence_schema.json(measured-first /
            # blocked-honest,纯取证非性能基准;语义名保持配置 b_keep vs 默认 b_default vs direct
            # 三链签名 part dump 对照,证语言层零静默降级能否不靠 P-01 例外达标;不入 budget
            # counter,P-01 规范线 / A/B / ②③契约线归属裁断留 owner)
            validator = dxil_b_strict_only_validator
        elif (
            f.name.startswith("dxil_b_graphics_sig_")
            and dxil_b_graphics_sig_validator is not None
        ):
            # G2.2 B 路图形签名能力取证证据(RD-010;RFC-0003 §9 Q-D131 / §7 B 路)→
            # milestones/g2/dxil_b_graphics_sig_evidence_schema.json(measured-first /
            # blocked-honest,纯取证非性能基准;ISG1/OSG1 签名 part dump 对照 A elemcount=0,
            # 不入 budget counter,A/B/混合架构结论留 owner)
            validator = dxil_b_graphics_sig_validator
        elif (
            f.name.startswith("dxil_path_spike_")
            and dxil_path_spike_validator is not None
        ):
            # G2.2 Q-D131=C 双路 DXIL spike 取证证据(RD-010;RFC-0003 §9 Q-D131)→
            # milestones/g2/dxil_path_spike_evidence_schema.json(measured-first /
            # blocked-honest,纯取证非性能基准;不入 budget counter,A/B 结论留 owner)
            validator = dxil_path_spike_validator
        elif (
            f.name.startswith("host_orch_smoke")
            and host_orch_smoke_validator is not None
        ):
            # MS1.2 single-source 宿主编排冒烟证据(G-MS1-2;RFC-0009 / RXS-0189~0196)→
            # milestones/ms1/host_orch_smoke_evidence_schema.json(CI 步骤 52
            # ci/host_orch_smoke.py 仅 device 段真跑时写;host .rx 经 std::gpu 编排 +
            # 同源 kernel PTX 嵌入单 EXE,device 真跑数值自校验 + 篡改 PTX/桩化写回
            # 双红绿;single_source=true 且 device_run=true 计入
            # ms1.counter.host_orch_single_source,ci/budget_eval.py)
            validator = host_orch_smoke_validator
        elif (
            f.name.startswith("uc07_offline_golden")
            and uc07_offline_golden_validator is not None
        ):
            # MS1.3 UC-07 离线 golden 冒烟证据(G-MS1-3/G-MS1-4;RFC-0010 §4.1/§4.4)→
            # milestones/ms1/uc07_offline_golden_evidence_schema.json(CI 步骤 53
            # ci/uc07_offline_golden_smoke.py 仅 device 段真跑全绿时写;apps/ruridrop
            # 主语言判据审计(零 .rs + kernel 同包 + rx build 产物链路)+ 三层 golden
            # (确定性两跑一致 / GPU vs refcpu 容差 / blessed manifest)+ 篡改重力常数
            # 数据流红绿;digest_match=true 计入 ms1.counter.uc07_offline_golden_frames,
            # ci/budget_eval.py)
            validator = uc07_offline_golden_validator
        elif (
            f.name.startswith("uc07_present_")
            and uc07_present_validator is not None
        ):
            # MS1.4 UC-07 实时 present 取证证据(G-MS1-5;RFC-0010 §4.5)→
            # milestones/ms1/uc07_present_evidence_schema.json(ci/uc07_bench.py
            # present 子命令,本机交互桌面人工链路写;realtime 入口经 RXS-0197/0198
            # present typestate 真窗口 ≥300 帧 + 末帧普通 Buffer download 采样对照
            # (天空区/水体区)EXE 内自校验;**不进 CI 硬门**,SKIP 不充绿,镜像
            # realtime_present_smoke 双态先例;不入 budget counter)
            validator = uc07_present_validator
        elif (
            (
                f.name.startswith("uc07_sph_step_")
                or f.name.startswith("uc07_offline_frame_")
                or f.name.startswith("uc07_realtime_frame_")
            )
            and uc07_bench_validator is not None
        ):
            # MS1.4 UC-07 生产档端到端性能证据(G-MS1-6;RFC-0010 §4.6)→
            # milestones/ms1/uc07_bench_evidence_schema.json(ci/uc07_bench.py 三项
            # bench,双层:单 trial ×3 + agg;进程级墙钟 timer=wall_clock_process,
            # 与 m0 cuda_event 内层协议的差异在 schema description/sampling.method
            # 如实声明;agg 的 results.trimmed_mean 由 ms1_budget.json entries 经
            # ci/budget_eval.py eval_entry 数据驱动判读,无新 evaluator 分支)
            validator = uc07_bench_validator
        elif (
            f.name.startswith("rd017_varying_semantic_spike_")
            and rd017_varying_semantic_spike_validator is not None
        ):
            # G2.4 RD-017 varying 语义名保名机制 spike 证据(owner ruling 选项① HLSL 边界
            # 改写 / 否决③)→ milestones/g2/rd017_varying_semantic_spike_evidence_schema.json
            # (measured-first / blocked-honest,纯取证非性能基准;输出/片元输入 varying 用户名
            # 经 HLSL 边界改写后 dxc 接受 + signature_gate 不放宽也过 + 物理 ABI 不变 + 确定性,
            # 不入 budget counter;golden bless / device 真跑 / RD-017 状态翻转留 owner,G-G2-4)
            validator = rd017_varying_semantic_spike_validator
        elif (
            f.name.startswith("ea1_real_endpoint_e2e_")
            and ea1_real_endpoint_e2e_validator is not None
        ):
            # EA1.2 真端点闭环 e2e 证据(G-EA1-3:真实 GitHub Releases 端点闭环归 EA1.2
            # e2e evidence,不进 pr-smoke;bootstrap→锚→四级校验→物化→探针 全链 measured_local)
            # → milestones/ea1/real_endpoint_e2e_evidence_schema.json。与冷启动两段
            # (install_e2e_evidence_schema.json,裁决 C)互斥不混用,开发机热环境不冒充冷启动段。
            validator = ea1_real_endpoint_e2e_validator
        elif (
            f.name.startswith("ea1_install_e2e_")
            and ea1_install_e2e_validator is not None
        ):
            # EA1 冷启动两段式取证(G-EA1-6/RXS-0219,裁决 C:vm_rxcheck / gpu_first_kernel,
            # 各 ≤10min measured;重测 ≤3 次全部尝试入 evidence 取 median,失败尝试同样入档)
            # → milestones/ea1/install_e2e_evidence_schema.json。
            validator = ea1_install_e2e_validator
        elif (
            f.name.startswith("rd027_pt_poison_spike_")
            and rd027_pt_poison_spike_validator is not None
        ):
            # G3.1 RD-027 毒径判别 spike 证据(G-G3-1 归因闸门:四层判别矩阵——双装载路
            # 对照/ptxas 优化档扫描/sanitizer 前置排除/源循环封顶插桩 + 单 artifact 事实
            # + 归因 verdict;measured-first / blocked-honest,纯取证非性能基准;探针隔离
            # spike/rd027-pt-poison/ 标 // SPIKE(RD-027),全 GPU 运行经 bench/proc_guard)
            # → milestones/g3/rd027_spike_evidence_schema.json。
            validator = rd027_pt_poison_spike_validator
        elif (
            f.name.startswith("uc04_present_")
            and uc04_present_validator is not None
        ):
            # G3.2 UC-04 可见窗口 present 冒烟证据(G-G3-2;RFC-0013 §4.A / RXS-0220~0222)→
            # milestones/g3/uc04_present_evidence_schema.json(ci/uc04_present_smoke.py device 段
            # 真跑写:可见窗口 flip-model swapchain present N 帧 + resize 重建 + 三点 backbuffer
            # readback 数值断言;present_ok=true 计入 g3.counter.uc04_present_frames,ci/budget_eval.py。
            # present 真跑 = 交互桌面人工链路不进 pr-smoke 硬门,SKIP 不充绿,镜像 realtime_present 双态)
            validator = uc04_present_validator
        elif (
            f.name.startswith("sampling_superset_")
            and sampling_superset_validator is not None
        ):
            # G3.3 采样超集面冒烟证据(G-G3-3;RFC-0013 §4.B / RXS-0223~0230)→
            # milestones/g3/sampling_superset_evidence_schema.json(ci/sampling_superset_smoke.py
            # device 段真跑写:≥6 模式数值判据〔mip/sample_lod/sample_grad/load 越界/wrap-vs-clamp/
            # sample_cmp/gather/storage 唯一写者/多分量〕逐项篡改→像素变 RED 复原 GREEN + 双后端一致性
            # 对照;num_modes>=6 计入 g3.counter.sampling_superset_modes,ci/budget_eval.py。device 真跑 =
            # 交互 GPU 链路不进 pr-smoke 硬门,SKIP 不充绿,镜像 uc04_present 双态)
            validator = sampling_superset_validator
        elif (
            f.name.startswith("bindless_")
            and bindless_smoke_validator is not None
        ):
            # G3.4 bindless 面冒烟证据(G-G3-4;RFC-0013 §4.C / RXS-0231~0235)→
            # milestones/g3/bindless_descriptor_smoke_evidence_schema.json(ci/bindless_smoke.py
            # device 段真跑写:≥4 纹理注册表按屏幕象限动态索引采样==四色 + 篡改注册序→像素换位 RED +
            # feature chain 四 bit 缺失→确定性 Err;smoke_ok=true 计入 g3.counter.bindless_descriptor_smoke,
            # ci/budget_eval.py。device 真跑 = 交互 GPU 链路不进 pr-smoke 硬门,SKIP 不充绿,镜像
            # sampling_superset 双态;harness bin/bindless_modes 判据结构就位,数值阈值 TODO 留 owner 本机)
            validator = bindless_smoke_validator
        elif (
            f.name.startswith("graph_")
            and auto_barrier_hazard_validator is not None
        ):
            # G3.5 render graph 自动 barrier hazard 证据(G-G3-5;RFC-0013 §4.D / RXS-0236~0241)→
            # milestones/g3/auto_barrier_hazard_evidence_schema.json(ci/render_graph_smoke.py device
            # 段真跑写:uc04 deferred 三 pass 图迁 Graph API 经 run_graph 自动状态推导重跑步骤 48 同判据 +
            # 漏声明 read → 装配期 strict 拒 RED + Vulkan 同图 run_graph 对照;hazard_ok=true 计入
            # g3.counter.auto_barrier_hazard_redgreen,ci/budget_eval.py。host 段 D6 互证金标准 + 图合法性
            # reject + 推导 golden 为本面核心恒跑验收;device 真跑 = 交互 GPU 链路不进 pr-smoke 硬门,SKIP
            # 不充绿,镜像 bindless 双态;D3D12 shim 执行器诚实边界,device 首跑先经 Vulkan run_graph)
            validator = auto_barrier_hazard_validator
        elif (
            f.name.startswith("meshrt_")
            and meshrt_stages_validator is not None
        ):
            # G3.6 mesh-task-RT 阶段 device 见证证据(G-G3-6;RFC-0013 §4.E7/E8 / RXS-0248)→
            # milestones/g3/meshrt_stages_evidence_schema.json(ci/meshrt_device_smoke.py device 段
            # 真跑写:bin/vk_mesh mesh 管线出图 covered + 篡改 SetMeshOutputs RED / bin/vk_rt 单三角形
            # TLAS 命中·miss 双色 + 移动顶点 RED;stages_ok 去重并集 ≥3〔mesh/raygen/closesthit〕计入
            # g3.counter.mesh_task_rt_stages,ci/budget_eval.py。device 真跑 = 交互 GPU 链路不进
            # pr-smoke 硬门,SKIP 不充绿,镜像 auto_barrier_hazard 双态;像素判据阈值 owner device 调优)
            validator = meshrt_stages_validator
        elif (
            f.name.startswith("export_c_smoke")
            and export_c_smoke_validator is not None
        ):
            # EI1.2 `#[export(c)]` C ABI 导出 codegen 冒烟证据(G-EI1-2;RFC-0014 Part A §4.A /
            # RXS-0250~0255)→ milestones/ei1/export_c_smoke_evidence_schema.json(ci/export_c_smoke.py
            # host 段恒跑 corpus 批跑 + 空导出集 RX6032 + 头幂等 RXS-0253 + 篡改再生成 RED RXS-0254;
            # 工具链/device 段 --emit=dll + dumpbin 未 mangle + 类型层 ABI 往返哨兵 RXS-0252/0253 步骤 71
            # 硬门 redline F6,缺工具链 dev_env_degrade SKIP 退 0,REQUIRE_REAL 翻硬红)。红绿 reject 语料
            # 基数计入 ei1.counter.export_c_redgreen_cases(计数源 conformance/export_c/reject/*.rx 非本证据)。
            validator = export_c_smoke_validator
        elif (
            f.name.startswith("uc05_invariant_matrix")
            and uc05_invariant_matrix_validator is not None
        ):
            # EI1.3 UC-05 RHI I1~I10 不变量矩阵证据(G-EI1-3 / G-EI1-5;RFC-0014 Part B /
            # RXS-0263/0264)→ milestones/ei1/uc05_invariant_matrix_schema.json。redline F3 硬门:
            # invariant 条目字段全 string/null(additionalProperties:false),**任何 number 值即
            # schema 违例** → by-construction 封死 I9/I10 无 in-repo 出处杜撰数字窗口(裁决 1 三档:
            # 编译期/装配期确定拦 I1~I8,report_only I9~I10 documented_historical)。
            validator = uc05_invariant_matrix_validator
        elif (
            f.name.startswith("uc05_rhi_smoke")
            and uc05_rhi_smoke_validator is not None
        ):
            # EI1.3 UC-05 RHI 冒烟证据(G-EI1-3;RFC-0014 Part B / RXS-0256~0265)→
            # milestones/ei1/uc05_rhi_smoke_evidence_schema.json(ci/uc05_rhi_smoke.py 步骤 72 写:
            # host 恒跑 uc05_corpus 批跑 + 零 .rs 审计 + rx build 编译;device 段 demo EXE green +
            # assembly EXE red-green,需 GPU 运行 Context::create,SKIP=dev-env-degrade,REQUIRE_REAL
            # 翻硬红)。I3/I5 装配期确定性拦由本 smoke red-green + rhi.rs 库单测双证。
            validator = uc05_rhi_smoke_validator
        elif (
            f.name.startswith("uc05_graphics_rhi_smoke")
            and uc05_graphics_rhi_smoke_validator is not None
        ):
            # G4.2 UC-05 图形 RHI 冒烟证据(G-G4-3;RFC-0015 §4.A / RXS-0270~0273/0275)→
            # milestones/g4/uc05_graphics_rhi_smoke_evidence_schema.json(ci/uc05_graphics_rhi_smoke.py
            # 步骤 76 写:host 恒跑 uc05_corpus gfx 语料批跑 + 零 .rs 审计 + --emit=check 编译 gfx
            # demo/assembly-reject;device 段 gfx demo EXE green + assembly EXE red-green,需 GPU + Vulkan,
            # SKIP=dev-env-degrade,RURIX_REQUIRE_REAL=1 翻硬红;像素判据 RXS-0222 归 PR-F/步骤 80)。
            validator = uc05_graphics_rhi_smoke_validator
        elif (
            f.name.startswith("vulkan_rhi_channel_smoke")
            and vulkan_rhi_channel_smoke_validator is not None
        ):
            # G4.4 PR-F Vulkan RHI 通道冒烟证据(G-G4-5;RFC-0015 §4.A / RXS-0293/0294 +
            # RXS-0222 像素判据 device 见证)→
            # milestones/g4/vulkan_rhi_channel_smoke_evidence_schema.json(ci/vulkan_rhi_channel_smoke.py
            # 步骤 80 写:host 恒跑 host_lib_tests + spirv_val;device 段 device_run 真 Vulkan 通道
            # 提交(compute + graphics 双腿),vulkan_channel_ok=true 表示通道完整闭合;SKIP=dev-env-degrade,
            # RURIX_REQUIRE_REAL=1 翻硬红)。
            validator = vulkan_rhi_channel_smoke_validator
        elif (
            f.name.startswith("blackhole_realtime_smoke")
            and blackhole_realtime_smoke_validator is not None
        ):
            # G4.6 PR-H blackhole 实时冒烟证据(G-G4-7;RFC-0015 §1 carve-out / RXS-0197/0198)→
            # milestones/g4/blackhole_realtime_smoke_evidence_schema.json(ci/blackhole_realtime_smoke.py
            # 步骤 81 写:carve-out 期 host_section_pass=false + device_section_rc=1 +
            # blackhole_realtime_ok=false 诚实失败而非降级;carve-out 解除后真实 blackhole 路径冒烟,
            # host_checks + device_run + blackhole_realtime_ok)。
            validator = blackhole_realtime_smoke_validator
        elif (
            f.name.startswith("renderer_graph_smoke")
            and renderer_graph_smoke_validator is not None
        ):
            # G5.2-A 渲染调度 render graph 冒烟证据(G-G5-3;RFC-0016 章 A)→
            # milestones/g5/renderer_graph_smoke_evidence_schema.json(ci/renderer_graph_smoke.py
            # 步骤 82 写:纯 host 门,rurix-render graph:: 四趟编译/EB 屏障 golden/别名峰值/校验
            # RED 自检/异步车道 fence/图 dump 单测全过 + 图 dump 测试在集内)。
            validator = renderer_graph_smoke_validator
        elif (
            f.name.startswith("renderer_draw_smoke")
            and renderer_draw_smoke_validator is not None
        ):
            # G5.2-B 渲染器 draw 派发桥冒烟证据(G-G5-4;RFC-0016 章 B)→
            # milestones/g5/renderer_draw_smoke_evidence_schema.json(ci/renderer_draw_smoke.py
            # 步骤 83 写:host 恒跑 render_exec host 单测;device 段 gate real --features vulkan
            # 含 4 项 device 真跑〔三角形真 draw/compute 写 buffer/raster→compute 混合/能力探测〕,
            # SKIP=dev-env-degrade,RURIX_REQUIRE_REAL=1 翻硬红)。
            validator = renderer_draw_smoke_validator
        elif (
            f.name.startswith("renderer_visbuffer_smoke")
            and renderer_visbuffer_smoke_validator is not None
        ):
            # G5.2-C+G5.3-C 虚拟化几何冒烟证据(G-G5-5;RFC-0016 章 C)→
            # milestones/g5/renderer_visbuffer_smoke_evidence_schema.json(ci/renderer_visbuffer_smoke.py
            # 步骤 84 写:host 恒跑 rurix-geom-build + rurix-render geometry:: 单测;device 段
            # blocked-honest 探针〔RFC-0016 §9.1 R-3 条件臂,RD-038 存续,不伪造 device 绿〕)。
            validator = renderer_visbuffer_smoke_validator
        elif (
            f.name.startswith("renderer_lighting_smoke")
            and renderer_lighting_smoke_validator is not None
        ):
            # G5.3-D/E/F 光照冒烟证据(G-G5-6;RFC-0016 章 D/E/F)→
            # milestones/g5/renderer_lighting_smoke_evidence_schema.json(ci/renderer_lighting_smoke.py
            # 步骤 85 写:host 恒跑 shadow::/gi::/rt:: 单测;device 段 blocked-honest 探针
            # 〔RD-038 存续,不伪造 device 绿〕)。
            validator = renderer_lighting_smoke_validator
        elif (
            f.name.startswith("renderer_temporal_smoke")
            and renderer_temporal_smoke_validator is not None
        ):
            # G5.2-H+G5.3-H 时域重建冒烟证据(G-G5-7;RFC-0016 章 H)→
            # milestones/g5/renderer_temporal_smoke_evidence_schema.json(ci/renderer_temporal_smoke.py
            # 步骤 86 写:host 恒跑 temporal:: 单测 + TAA 静态收敛在集内;device 段 blocked-honest
            # 探针〔RD-038 存续〕)。
            validator = renderer_temporal_smoke_validator
        elif (
            f.name.startswith("uc06_renderer_smoke")
            and uc06_renderer_smoke_validator is not None
        ):
            # G5.4 UC-06 全管线渲染器冒烟证据(G-G5-8;RFC-0016 §1 管线图)→
            # milestones/g5/uc06_renderer_smoke_evidence_schema.json(ci/uc06_renderer_smoke.py
            # 步骤 87 写:host 恒跑 uc06-renderer host 全管线 exit 0 + asserts 全 true + PSO 告警 0
            # + graph alias/fence 结构;device 段 gate real --features vulkan --device 真跑,
            # SKIP=dev-env-degrade,RURIX_REQUIRE_REAL=1 翻硬红)。
            validator = uc06_renderer_smoke_validator
        elif (
            f.name.startswith("physics_core_smoke")
            and physics_core_smoke_validator is not None
        ):
            # G6.2 物理库底座冒烟证据(G-G6-3;RFC-0017 §4.A/§4.C)→
            # milestones/g6/physics_core_smoke_evidence_schema.json(ci/physics_core_smoke.py
            # 步骤 88 写:纯 host 门,cargo 三档单测 exit 0 + §4.A7 单测清单关键字在位 +
            # §4.C4 grep 审计门〔零 sys 引用/零原生类型名/unsafe allow 白名单/SAFETY 注释〕;
            # 性能数字入 checks 不进硬门)。
            validator = physics_core_smoke_validator
        elif (
            f.name.startswith("physics_bridge_smoke")
            and physics_bridge_smoke_validator is not None
        ):
            # G6.3 渲染合流桥冒烟证据(G-G6-4;RFC-0017 §4.B)→
            # milestones/g6/physics_bridge_smoke_evidence_schema.json(ci/physics_bridge_smoke.py
            # 步骤 89 写:host 恒跑 cargo 两档单测 exit 0 + bridge 七行为测试关键字在位 +
            # §4.B 机器可核面审计门四项〔render 零物理回引/零原生类型名、bridge 零 AS·时域
            # API、RemovalReceipt 类型纪律〕;device 段 gate real uc08 --device 真跑
            # 〔像素/运动非平凡对拍〕,SKIP=dev-env-degrade,RURIX_REQUIRE_REAL=1 翻硬红)。
            # 前缀置于 physics_core_smoke 分支之后即可(两前缀互不包含)。
            validator = physics_bridge_smoke_validator
        elif (
            f.name.startswith("physics_rapier_parity")
            and physics_rapier_parity_validator is not None
        ):
            # G6.4 Rapier 第二后端对拍冒烟 + parity 标定双形态证据(G-G6-5;
            # RFC-0017 §4.D)→ milestones/g6/physics_rapier_parity_evidence_schema.json
            # (ci/physics_rapier_parity_smoke.py 步骤 90 写:纯 host 门,cargo metadata
            # 默认 off 机验 + rapier-only 依赖树零 CMake + cargo test/clippy 两腿 +
            # parity 双进程重放一致〔§4.D3 容差/重叠率/RLE/阈值钉定〕+ §4.D4 文档口径
            # 审计)。单前缀同时覆盖 smoke 自身 evidence(physics_rapier_parity_smoke_*)
            # 与 parity 测试侧标定 evidence(physics_rapier_parity_2*)——前者是后者的
            # 前缀延长,单路由天然消解包含关系,schema 内按 subject if/then 双形态分流;
            # 与 physics_core/bridge 前缀互不包含,置于其后安全。
            validator = physics_rapier_parity_validator
        elif (
            f.name.startswith("uc08_physics_smoke")
            and uc08_physics_smoke_validator is not None
        ):
            # G6.3 UC-08 物理合流 demo 冒烟证据(G-G6-7;RFC-0017 §4.B)→
            # milestones/g6/uc08_physics_smoke_evidence_schema.json(ci/uc08_physics_smoke.py
            # 步骤 91 写:host 恒跑 uc08 单测 + 96 帧全跑 JSON 16 断言全 true +
            # physics_step_ms measured 留证〔P-09 不进硬门〕;device 段 gate real
            # uc08 --device 真跑〔像素/运动非平凡对拍〕,SKIP=dev-env-degrade,
            # RURIX_REQUIRE_REAL=1 翻硬红)。前缀须置于任何更通用 uc0 前缀之前
            # (现路由表无 uc08 通用前缀,本分支位于 uc06/uc07 分支之后安全)。
            validator = uc08_physics_smoke_validator
        elif (
            f.name.startswith("taichi_vulkan_spike_")
            and taichi_vulkan_spike_validator is not None
        ):
            # G6.5 Taichi Vulkan AOT spike 冒烟证据(G-G6-6 成功臂;RFC-0017 §4.E)→
            # milestones/g6/taichi_vulkan_spike_evidence_schema.json
            # (ci/taichi_vulkan_spike_smoke.py 步骤 92 写:host 恒跑 AOT 资产核验/
            # feature taichi-tirt 默认 off cargo metadata 机验/§4.E4 三条禁止审计/
            # U43 登记/uc09 单测 + host 腿 --json 8 断言;device 段 gate real
            # --features taichi-tirt 真跑〔五断言 + nonzero==64 + first_values 逐位〕,
            # 缺 taichi_c_api.dll SKIP=dev-env-degrade 退 0 不充绿,
            # RURIX_REQUIRE_REAL=1 翻硬红)。前缀与现有各族互不包含,置于末尾安全。
            validator = taichi_vulkan_spike_validator
        elif (
            f.name.startswith("g7_baseline_")
            and g7_baseline_validator is not None
        ):
            # G7.0 基线门实跑证据(G-G7-1 治理/基线门;D-G7-1)→
            # milestones/g7/g7_baseline_evidence_schema.json(G7.0 波次十条既有守卫
            # 〔fmt/clippy/test/number_ledger/schemas/structure/guardrails/contribution/
            # trace/budget〕逐条实跑的命令+退出码+pass/fail+摘要全量记录,附环境画像与
            # Jolt vendor/license/SBOM 复核结论;纯 host 治理记录,ADVISORY 不阻断项如实
            # 入 advisory_notes;不入 budget counter)。前缀与现有各族互不包含,置于末尾安全。
            validator = g7_baseline_validator
        elif (
            f.name.startswith("g7_perf_baseline_")
            and g7_perf_baseline_validator is not None
        ):
            # G7.1 性能 baseline 实测证据(G-G7-3 预算非空化;CI_GATES §5)→
            # milestones/g7/g7_perf_baseline_evidence_schema.json(UC-06 host 软件参照
            # 管线 1080p 末帧 12 阶段 cpu_ms 求和,release 三 trial trimmed mean,results.
            # trimmed_mean 供 g7.bench.uc06_host_frame_cpu_ms_1080p 经 ci/budget_eval.py
            # eval_entry 通用路判读,零新 entries evaluator 分支;correctness.device_
            # pixel_parity_pass + validation_clean 供 g7.counter.uc06_device_pixel_parity
            # 计数)。前缀置于 g7_baseline_ 分支之后(两前缀互不包含)。
            validator = g7_perf_baseline_validator
        elif (
            f.name.startswith("ray_query_codegen_smoke")
            and ray_query_codegen_validator is not None
        ):
            # G7.2 W3a compute RayQuery codegen 冒烟(步骤 93;G-G7-4)→
            # milestones/g7/ray_query_codegen_evidence_schema.json(host/compile 段六项:
            # 语料 accept/reject、codegen 锚定单测、真实 .rx→.spv 且 spirv-val 双口径
            # 〔vulkan1.2 + spv1.4〕、反汇编 golden 最小集〔per-file + 语料并集〕、
            # W1/W2 五 kernel 对 tests/vulkan/w1w2_spv_manifest.json 的 sha256/版本/
            # capability 零漂移、篡改 .spv 的 RED 反证;device 段最小 hit/miss kernel
            # 真跑 gate real,硬前置 G7.3 W3b 未在树时 device_blocked 记 blocked-honest)。
            validator = ray_query_codegen_validator
        elif (
            f.name.startswith("renderer_device_frame_smoke")
            and renderer_device_frame_validator is not None
        ):
            # G7.6 One True Device Frame 冒烟(步骤 96;G-G7-8)→
            # milestones/g7/renderer_device_frame_evidence_schema.json(host 段:
            # schema 自检 + SCENE_FREEZE 960×540→1080p 锚 + RD-038 行 1/2/4/8 与
            # §6.4 帧链并入留痕 + host oracle 过滤 + 既有 kernel manifest 零漂移 +
            # 6 glue kernel 排放 + 静态 provenance 审计〔禁 execute_frame 单发〕;
            # device 段 gate real:--device-frame 8 帧对拍/非退化/provenance + RED 四轴)。
            # 前缀长于 renderer_w3_smoke / renderer_raster_diff_smoke 互不包含;置于
            # 二者之前以遵守「长前缀先匹配」纪律。
            validator = renderer_device_frame_validator
        elif (
            f.name.startswith("renderer_soak_")
            and renderer_soak_validator is not None
        ):
            # G7.6 soak 取证(不占步骤号;CI_GATES §3)→
            # milestones/g7/renderer_soak_evidence_schema.json(≥30min/≥10000 帧;
            # validation/lost/tdr/leak 全 0;schema 本 PR 预置,真跑归 PR-4)。
            # 前缀 renderer_soak_ 与 renderer_device_frame_smoke_ 互不包含。
            validator = renderer_soak_validator
        elif (
            f.name.startswith("renderer_w3_smoke")
            and renderer_w3_validator is not None
        ):
            # G7.4 W3c renderer W3 三效果核冒烟(步骤 94;G-G7-6)→
            # milestones/g7/renderer_w3_evidence_schema.json(host 段七项:host 三效果
            # oracle 单测〔rt:: + gi::,数值语义 0-byte 回归网〕、AS/lifetime 审计
            # 〔as_manager 单源 + forbid(unsafe) + U30 登记〕、三 kernel 真实 .rx→.spv 且
            # SPIR-V 1.4 + spirv-val 双口径、反汇编 golden 并集〔含 barycentrics 分量真实
            # 消费〕、单 TLAS 纪律静态审计〔AccelStruct 形参恰好一个,RXS-0297〕、W1/W2 五
            # kernel 零漂移、篡改 .spv 的 RED 反证;device 段 gate real:同一 TLAS identity
            # 驱动三 dispatch,零容差量〔hit/miss + instance/primitive/geometry index〕与
            # measured/tol 成对量〔t / barycentric / 辐射度 / AO / 可见性〕逐项机验,
            # RED 三轴〔篡改几何数据流反证 / 过期 TLAS / 错误 barrier〕)。
            # 前缀与 renderer_{graph,draw,visbuffer,lighting,temporal}_ 互不包含。
            validator = renderer_w3_validator
        elif (
            f.name.startswith("renderer_raster_diff_smoke")
            and renderer_raster_diff_validator is not None
        ):
            # G7.5 光栅 diff 与 RD-038 余项冒烟(步骤 95;G-G7-7)→
            # milestones/g7/renderer_raster_diff_evidence_schema.json(host 段七项:
            # RD-038 八行字面矩阵 + 场景/相机冻结锚、host oracle 单测〔shadow:: +
            # temporal:: + geometry::visbuffer,数值语义 0-byte 回归网〕、VisBuffer
            # 位格式冻结面〔depth30|cluster27|tri7 与 SW kernel 位移同源〕、余项三核
            # 真实 .rx→.spv〔SPIR-V 1.0 不误升 + spirv-val + 同源 ×2 确定性 + 零 ray
            # query 声明〕、**HW 光栅 blocked-honest 机验**〔目标形态语料 RX6026 必红
            # + 逐轴隔离探针产 missing_toolchain_caps〕、W1/W2 五 kernel 零漂移、篡改
            # .spv 的 RED 反证;device 段 gate real:VSM 深度/采样 + TSR 的 measured/tol
            # 成对机验〔0/1 二值量零容差〕+ SW 基准侧逐位 + RED 两轴)。schema 对
            # hw_raster_diff 施 if/then:verified-diff-zero 须 diff_pixels==0 且 hw_side
            # 在位;blocked-* 须 missing_toolchain_caps 非空 + 逐轴探针 + spec 锚 + 升级路径。
            # 前缀与 renderer_w3_smoke / renderer_{graph,draw,visbuffer,lighting,temporal}_
            # 互不包含。
            validator = renderer_raster_diff_validator
        elif (
            f.name.startswith("g8_perf_baseline_")
            and g8_perf_baseline_validator is not None
        ):
            # G8.1 governance-only measured baseline：UC-06 host 参考帧三 trial
            # trimmed_mean + RTX 4070 Ti Vulkan device correctness/validation。
            # host 计时与 device 见证在 schema 中分栏，禁止把 host cpu_ms 冒充 GPU
            # frame time；供 g8_budget.json 通用 measured entry 判读。
            validator = g8_perf_baseline_validator
        elif (
            f.name.startswith("g8_m31_reflection_hash_")
            and g8_m31_reflection_hash_validator is not None
        ):
            # G8.2 M31 reflection_hash 硬门(RXS-0304~0307;RFC-0019 §4.4):
            # host/compile 纯 host 门,canonical reflection v1 序列化与 interface hash
            # 稳定性六腿判据。device 段 not_applicable(CI_GATES §6 host-only 行)。
            # 供 g8_budget.json g8.counter.reflection_hash_legs 判读。
            validator = g8_m31_reflection_hash_validator
        elif (
            f.name.startswith("g8_m29_shader_permutation_")
            and g8_m29_shader_permutation_validator is not None
        ):
            # G8.2 M29 shader_permutation 硬门(RXS-0308~0310;RFC-0019 §4.3):
            # host/compile 纯 host 门,permutation 域求解/canonical key/裁剪预算
            # 报告 13 项 checks。device 段 not_applicable(CI_GATES §6 host-only 行)。
            # 供 g8_budget.json g8.counter.shader_permutation_legs 判读。
            validator = g8_m29_shader_permutation_validator
        elif (
            f.name.startswith("g8_m32_capability_profile_")
            and g8_m32_capability_profile_validator is not None
        ):
            # G8.2 M32 capability_profile 硬门(RXS-0311~0313;RFC-0019 §4.5):
            # host/compile 纯 host 门,#[requires]/调用图并集/profile 选择律/
            # fallback/snapshot 原语 14 项 checks(三腿缺一 FAIL)。device 段
            # not_applicable。供 g8.counter.capability_profile_legs 判读。
            validator = g8_m32_capability_profile_validator
        elif (
            f.name.startswith("g8_m30_pso_cache_")
            and g8_m30_pso_cache_validator is not None
        ):
            # G8.2 M30 pso_cache 硬门(RXS-0314~0316;RFC-0019 §4.1.4):
            # driver/device 门,collector/golden + cold/warm/stall/tamper 四轴 +
            # binary/cache 诚实律 14 项 checks。device 段 gate real
            # (RURIX_REQUIRE_REAL=1)。供 g8.counter.pso_cache_legs 判读。
            validator = g8_m30_pso_cache_validator
        elif (
            f.name.startswith("g8_m85_shader_manifest_ddc_")
            and g8_m85_shader_manifest_ddc_validator is not None
        ):
            # G8.2/3 M85 shader_manifest_ddc 硬门(RXS-0317~0318;RFC-0019):
            # host 门,--phase g8.2 merge/dedup/coverage 腿;phase_g8_3_pass
            # 字段位冻结、g8.2 期恒 false。供 g8.counter.shader_manifest_phase_g82_legs。
            validator = g8_m85_shader_manifest_ddc_validator
        elif (
            f.name.startswith("g8_m89_single_source_gfx_submit_")
            and g8_m89_single_source_gfx_submit_validator is not None
        ):
            # G8.2 M89 single_source_gfx_submit 硬门(RXS-0319~0321;RD-037):
            # device 门,单源 gfx VB/IB/draw + artifacts v2 真派发 + golden;
            # 零 Rust 宿主像素替身。供 g8.counter.single_source_gfx_checks。
            validator = g8_m89_single_source_gfx_submit_validator
        elif (
            f.name.startswith("g8_m50_rt_pipeline_incremental_")
            and g8_m50_rt_pipeline_incremental_validator is not None
        ):
            # G8.2 M50 rt_pipeline_incremental 硬门(RXS-0322~0327;RD-040/M50):
            # device 门,多 hit group/SBT user data/stack/library + 冻结子集;
            # RXS-0248 最小见证不得代绿。供 g8.counter.rt_pipeline_incremental_features。
            validator = g8_m50_rt_pipeline_incremental_validator
        elif (
            f.name.startswith("g8_wave2_exit_")
            and g8_wave2_exit_validator is not None
        ):
            # G8.2 波次聚合门 g8.wave.2.exit(CI_GATES §5;步骤 104):
            # 只读汇总七 P0 + RFC-0019 Approved + RD-037 closed + RD-038
            # 本波接入空集;不重跑、不代绿;host 聚合门 device=not_applicable。
            validator = g8_wave2_exit_validator
        elif (
            f.name.startswith("g8_wave3_exit_")
            and g8_wave3_exit_validator is not None
        ):
            # G8.3 波次聚合门 g8.wave.3.exit(步骤 111)。
            validator = g8_wave3_exit_validator
        elif (
            f.name.startswith("g8_m01_meshlet_page_builder_")
            and g8_m01_meshlet_page_builder_validator is not None
        ):
            # G8.3 M01 meshlet_page_builder(RXS-0328~0331):host 门,逻辑页 RXPL。
            validator = g8_m01_meshlet_page_builder_validator
        elif (
            f.name.startswith("g8_m83_texture_transcode_")
            and g8_m83_texture_transcode_validator is not None
        ):
            # G8.3 M83 texture_transcode(RXS-0334):host 门,四腿真实 codec。
            validator = g8_m83_texture_transcode_validator
        elif (
            f.name.startswith("g8_m81_gltf_import_")
            and g8_m81_gltf_import_validator is not None
        ):
            # G8.3 M81 gltf_import(RXS-0332~0333):host 门,严格 glTF 六表。
            validator = g8_m81_gltf_import_validator
        elif (
            f.name.startswith("g8_m79_asset_determinism_")
            and g8_m79_asset_determinism_validator is not None
        ):
            # G8.3 M79 asset_determinism(RXS-0335~0337):host 门,canon+双构建。
            validator = g8_m79_asset_determinism_validator
        elif (
            f.name.startswith("g8_m04_page_format_abi_")
            and g8_m04_page_format_abi_validator is not None
        ):
            # G8.3 M04 page_format_abi(RXS-0338~0342):device 门,双 ABI+LZ1。
            validator = g8_m04_page_format_abi_validator
        elif (
            f.name.startswith("g8_m80_ddc_content_address_")
            and g8_m80_ddc_content_address_validator is not None
        ):
            # G8.3 M80 ddc_content_address(RXS-0343):host 门,九段 CAS。
            validator = g8_m80_ddc_content_address_validator
        elif (
            f.name.startswith("g8_m37_streaming_io_")
            and g8_m37_streaming_io_validator is not None
        ):
            # G8.4 M37 streaming_io(步骤 112):device 门,磁盘→解压→upload→GPU。
            validator = g8_m37_streaming_io_validator
        elif (
            f.name.startswith("g8_gate_geom_page_")
            and g8_gate_geom_page_validator is not None
        ):
            # G8.4 门-GeomPage(步骤 113):独立 evidence,消费冻结 M04 ABI。
            validator = g8_gate_geom_page_validator
        elif (
            f.name.startswith("g8_wave4_exit_")
            and g8_wave4_exit_validator is not None
        ):
            # G8.4 波次聚合门 g8.wave.4.exit(步骤 114)。
            validator = g8_wave4_exit_validator
        elif (
            f.name.startswith("g8_m19_vsm_page_cache_")
            and g8_m19_vsm_page_cache_validator is not None
        ):
            # G8.5a M19 vsm_page_cache(步骤 115):device 门,跨帧页缓存对拍。
            validator = g8_m19_vsm_page_cache_validator
        elif (
            f.name.startswith("g8_wave5a_exit_")
            and g8_wave5a_exit_validator is not None
        ):
            # G8.5a 波次聚合门 g8.wave.5a.exit(步骤 116)。
            validator = g8_wave5a_exit_validator
        elif (
            f.name.startswith("g8_m24_tsr_contract_")
            and g8_m24_tsr_contract_validator is not None
        ):
            # G8.5b M24 tsr_contract(步骤 117)。
            validator = g8_m24_tsr_contract_validator
        elif (
            f.name.startswith("g8_m25_upscaler_input_abi_")
            and g8_m25_upscaler_input_abi_validator is not None
        ):
            # G8.5b M25 upscaler_input_abi(步骤 118)。
            validator = g8_m25_upscaler_input_abi_validator
        elif (
            f.name.startswith("g8_wave5b_exit_")
            and g8_wave5b_exit_validator is not None
        ):
            # G8.5b 波次聚合门 g8.wave.5b.exit(步骤 119)。
            validator = g8_wave5b_exit_validator
        elif (
            f.name.startswith("g8_m66_physics_replay_")
            and g8_m66_physics_replay_validator is not None
        ):
            # G8.6a M66 physics_replay(步骤 120)。
            validator = g8_m66_physics_replay_validator
        elif (
            f.name.startswith("g8_wave6a_exit_")
            and g8_wave6a_exit_validator is not None
        ):
            # G8.6a 波次聚合门 g8.wave.6a.exit(步骤 121)。
            validator = g8_wave6a_exit_validator
        elif (
            f.name.startswith("g8_m67_network_physics_")
            and g8_m67_network_physics_validator is not None
        ):
            # G8.6b M67 network_physics(步骤 122)。
            validator = g8_m67_network_physics_validator
        elif (
            f.name.startswith("g8_wave6b_exit_")
            and g8_wave6b_exit_validator is not None
        ):
            validator = g8_wave6b_exit_validator
        elif (
            f.name.startswith("g8_m68_fracture_pipeline_")
            and g8_m68_fracture_pipeline_validator is not None
        ):
            validator = g8_m68_fracture_pipeline_validator
        elif (
            f.name.startswith("g8_wave6c_exit_")
            and g8_wave6c_exit_validator is not None
        ):
            validator = g8_wave6c_exit_validator
        elif (
            f.name.startswith("g8_m72_cloth_product_chain_")
            and g8_m72_cloth_product_chain_validator is not None
        ):
            validator = g8_m72_cloth_product_chain_validator
        elif (
            f.name.startswith("g8_wave6d_exit_")
            and g8_wave6d_exit_validator is not None
        ):
            validator = g8_wave6d_exit_validator
        elif (
            f.name.startswith("g8_wave8a_soak_")
            and g8_wave8a_soak_validator is not None
        ):
            validator = g8_wave8a_soak_validator
        elif (
            f.name.startswith("g8_wave8b_closeout_")
            and g8_wave8b_closeout_validator is not None
        ):
            validator = g8_wave8b_closeout_validator
        elif (
            f.name.startswith("g8_wave7_decisions_")
            and g8_wave7_decisions_validator is not None
        ):
            validator = g8_wave7_decisions_validator
        elif (
            f.name.startswith("g9_vram_as_baseline_")
            and g9_vram_as_baseline_validator is not None
        ):
            # G9.1 governance-only measured baseline：ctypes 直连 vulkan-1.dll
            # 实测 device-local VRAM heap + 130k 三角 BLAS 构建耗时/存储/scratch，
            # 并登记 DGC/descriptor_buffer 等 G9 阻塞性前置扩展在位性；
            # host 墙钟同步等待口径，不冒充 GPU 异步耗时。供 g9_budget.json
            # g9.bench.* 通用 measured entry 判读。前缀与 g8_* 全族互不包含。
            validator = g9_vram_as_baseline_validator
        elif (
            f.name.startswith("g9_m121_physics_particle_view_")
            and g9_m121_physics_particle_view_validator is not None
        ):
            # G9.2 M121 physics_particle_view(步骤 136;双 phase 骨架期)。
            validator = g9_m121_physics_particle_view_validator
        elif (
            f.name.startswith("g9_m122_gameplay_field_")
            and g9_m122_gameplay_field_validator is not None
        ):
            # G9.2 M122 gameplay_field(步骤 137;双 phase 骨架期)。
            validator = g9_m122_gameplay_field_validator
        elif (
            f.name.startswith("g9_wave2_exit_")
            and g9_wave2_exit_validator is not None
        ):
            validator = g9_wave2_exit_validator
        elif (
            f.name.startswith("g9_wave3_exit_")
            and g9_wave3_exit_validator is not None
        ):
            # G9.3 波聚合门(步骤 146;ci/g9_wave3_exit_check.py 写:七门最新
            # evidence 只读汇总 + RFC-0022/0023 Approved + RXS-0350~0356 条款头
            # + U56/U57 登记;聚合不代绿)。
            validator = g9_wave3_exit_validator
        elif (
            f.name.startswith("g9_wave4_exit_")
            and g9_wave4_exit_validator is not None
        ):
            # G9.4 波聚合门(步骤 153;ci/g9_wave4_exit_check.py 写:六门最新
            # evidence 只读汇总 + RFC-0022 Approved + RXS-0357~0362 条款头
            # + 门序机器阻断留痕 + 六冻结带;聚合不代绿)。
            validator = g9_wave4_exit_validator
        elif (
            f.name.startswith("g9_wave5_exit_")
            and g9_wave5_exit_validator is not None
        ):
            # G9.5 波聚合门(步骤 165;ci/g9_wave5_exit_check.py 写:十一门最新
            # evidence 只读汇总 + RFC-0025 Approved + RXS-0363~0373 条款头
            # + M115 32B 布局 digest 冻结面 + M114 strand 档 not-triggered
            # 登记 + M120 仅测量不定档;聚合不代绿)。
            validator = g9_wave5_exit_validator
        elif (
            f.name.startswith("g9_wave6_exit_")
            and g9_wave6_exit_validator is not None
        ):
            # G9.6 波聚合门(步骤 169;ci/g9_wave6_exit_check.py 写:五门最新
            # evidence 只读汇总——M121/M122 完整期双 phase 核验 + M124/M125/
            # M126 + RXS-0374~0379 条款头 + RFC-0024 v1.1 章 F1/F2 + M123
            # no-go 登记 + M125 verdict/5.3 基线 0-byte + M126 RD-044 verdict
            # + 门序 interlock;聚合不代绿)。
            validator = g9_wave6_exit_validator
        elif (
            f.name.startswith("g9_p2_decisions_")
            and g9_p2_decisions_validator is not None
        ):
            # G9.7 P2 穷举决策门(步骤 170;ci/g9_p2_decisions_check.py 写:33 行
            # 冻结候选闭集全等 + 裁决枚举合法 + 零空行 + 承接锚「重判条件+
            # 兜底+G10+ 重评窗」+ MAP 34 key 互斥 + deferred.json history 对账
            # 〔RD-039 +1/RD-040 +3,零新 RD〕;普通检查门非聚合门,不代绿)。
            validator = g9_p2_decisions_validator
        elif (
            f.name.startswith("g9_stabilization_soak_")
            and g9_stabilization_soak_validator is not None
        ):
            # G9.8a stabilization soak 聚合门(步骤 171;ci/g9_stabilization_soak.py
            # 写:15 P0 + 19 go P1 + wave2~wave6 exit + p2_decisions 全量回归
            # 〔M121/M122 双 phase 完整期核验;34 门 base_commit 同值=同一候选
            # close-out 基线〕+ M110 大世界流送长 soak〔≥1800s 且 ≥10000 帧,
            # sleep_seconds 恒 0/active≈wall 诚实口径〕+ budget --strict 非空零
            # estimated/skip + 日期锚)。
            validator = g9_stabilization_soak_validator
        elif (
            f.name.startswith("g9_wave8b_closeout_")
            and g9_wave8b_closeout_validator is not None
        ):
            # G9.8b close-out 终审门(步骤 172;ci/g9_closeout_check.py 写:
            # 34 key + wave2~8a 七聚合门 + MAP 三向 + P2 + budget --strict
            # + 8a full-run 先行〔立项裁决 6 同日放行〕+ RD 最终状态逐字一致
            # 〔RD-034/039~044 七条目级 status 全 open + P2 33 行闭集在树〕
            # + 最后新绿留痕;VERDICT=READY|BLOCKED,status flip 独立 commit)。
            validator = g9_wave8b_closeout_validator
        elif (
            f.name.startswith("g10_m131_asset_license_registry_")
            and g10_m131_asset_license_registry_validator is not None
        ):
            # G10.3 M131 许可登记门(步骤 173;ci/g10_asset_license_registry_smoke.py
            # 写:白名单闭集 + 按类登记零缺行 + attribution 子字段闭集 + 清单级
            # canonical digest 缓存复算 + git 零二进制守卫 + RED 五件全检出)。
            validator = g10_m131_asset_license_registry_validator
        elif (
            f.name.startswith("g10_m132_corpus_loading_")
            and g10_m132_corpus_loading_validator is not None
        ):
            # G10.3 M132 语料加载门(步骤 174;ci/g10_corpus_loading_smoke.py 写:
            # 逐场景 rxcook 真实加载 + 三角形/材质/纹理计数非空 + 计数与六表
            # count/digest 全等 golden + 加载事件序列 golden + 静默丢场景零 +
            # RED 三件全检出)。
            validator = g10_m132_corpus_loading_validator
        elif (
            f.name.startswith("g10_m133_corpus_list_freeze_")
            and g10_m133_corpus_list_freeze_validator is not None
        ):
            # G10.3 M133 清单冻结门(步骤 175;ci/g10_corpus_list_freeze_smoke.py
            # 写:清单 digest 注册在树 + 只追加修订程序 + M131/M132 行集对账 +
            # ready 下界 vacuous 拦截 + RED 三件全检出)。
            validator = g10_m133_corpus_list_freeze_validator
        elif (
            f.name.startswith("g10_wave3_exit_")
            and g10_wave3_exit_validator is not None
        ):
            # G10.3 波聚合门(步骤 176;ci/g10_wave3_exit_check.py 写:三门最新
            # evidence 只读汇总 + RXS-0380~0383 条款头 + RFC-0027 Approved +
            # 注册表零缺行 + 清单 digest 注册在树;聚合不代绿)。
            validator = g10_wave3_exit_validator
        elif (
            f.name.startswith("g10_m128_ue5_capture_environment_")
            and g10_m128_ue5_capture_environment_validator is not None
        ):
            # G10.2 M128 UE5 出图环境门(步骤 177;ci/g10_ue5_capture_environment_smoke.py
            # 写:UE 5.8.1 Build.version 实测 + MRQ 臂真出帧 + 新鲜度/真帧判据 +
            # 画像七元组 + RED 三夹具 + live 非零退出探针)。
            validator = g10_m128_ue5_capture_environment_validator
        elif (
            f.name.startswith("g10_m129_ue5_reference_frames_")
            and g10_m129_ue5_reference_frames_validator is not None
        ):
            # G10.2 M129 UE5 参考帧门(步骤 178;ci/g10_ue5_reference_frames_smoke.py
            # 写:暂定场景集逐场景参考帧 + 双跑 canonical digest 一致 + provenance
            # 闭集 + RED 三夹具〔不等帧对/缺行/篡改〕)。
            validator = g10_m129_ue5_reference_frames_validator
        elif (
            f.name.startswith("g10_m130_dual_determinism_contract_")
            and g10_m130_dual_determinism_contract_validator is not None
        ):
            # G10.2 M130 双端确定性契约门骨架期(步骤 179;ci/g10_dual_determinism_contract_smoke.py
            # --phase g10.2 写:双端 schema 各一份 + digest 比对面 + 边界浮点语料 +
            # RED 四臂;phase_g10_5_pass=false 不充双端核验期绿)。
            validator = g10_m130_dual_determinism_contract_validator
        elif (
            f.name.startswith("g10_wave2_exit_")
            and g10_wave2_exit_validator is not None
        ):
            # G10.2 波聚合门(步骤 180;ci/g10_wave2_exit_check.py 写:三门最新
            # evidence 只读汇总 + RXS-0380/RXS-0384 条款头 + RFC-0026/0027
            # Approved + 场景集登记 + M130 phase 纪律;聚合不代绿)。
            validator = g10_wave2_exit_validator
        elif (
            f.name.startswith("g9_m102_dgc_abstraction_")
            and g9_m102_dgc_abstraction_validator is not None
        ):
            # G9.2 M102 DGC 抽象门(步骤 131;ci/g9_dgc_abstraction_smoke.py 写:
            # host 段 dgc.rs 装配期核验/结构性断言/capability snapshot 阻塞性前置
            # + device 段 vk_dgc 最小链路真跑〔compute pre-pass 直写 DgcBuffer →
            # vkCmdExecuteGeneratedCommandsEXT → 显式 readback pass 回读哨兵字〕,
            # RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1,回读计数器=0)。
            validator = g9_m102_dgc_abstraction_validator
        elif (
            f.name.startswith("g9_m103_descriptor_global_table_")
            and g9_m103_descriptor_global_table_validator is not None
        ):
            # G9.2 M103 descriptor buffer 全局表(ci/g9_descriptor_global_table_smoke.py
            # 步骤 134 写：host+device 门,reflection↔shader 索引双向精确相等 +
            # 65536 条目出图 golden + set/binding 0-byte 回归 + 分配确定性 +
            # 悬空/越界拒 + leak 零 + validation 零)→
            # milestones/g9/g9_m103_descriptor_global_table_evidence_schema.json。
            validator = g9_m103_descriptor_global_table_validator
        elif (
            f.name.startswith("g9_m104_accesskind_indirect_edge_")
            and g9_m104_accesskind_indirect_edge_validator is not None
        ):
            # G9.2 M104 AccessKind 加性 IndirectCommandRead(ci/g9_accesskind_indirect_edge_smoke.py
            # 步骤 135 写：纯 host 门,新边 golden + 既有 golden 0-byte + strict 拒 +
            # cabi 不可表达诊断 + RXS-0239 字面不动 + D6 互证)→
            # milestones/g9/g9_m104_accesskind_indirect_edge_evidence_schema.json。
            validator = g9_m104_accesskind_indirect_edge_validator
        elif (
            f.name.startswith("g9_m90_cluster_dag_deepening_")
            and g9_m90_cluster_dag_deepening_validator is not None
        ):
            # G9.2 P0 硬门 M90 cluster_dag_deepening（步骤 132，host 纯 host 门）：
            # 固定 mesh 语料 cluster DAG 两次独立构建 canonical 字节相等 + 每条
            # parent→child 边误差单调不增逐边机器核验 + 破坏单调性 fixture 构建期
            # fail-closed typed Err 拒录 + 蒙皮元数据/CLAS 离线烘焙输入字段按冻结
            # schema 完整 roundtrip（RXS-0345；RFC-0022 §4.1）。
            # 前缀与 g9_vram_as_baseline_ / g9_m91_page_format_v2_abi_ 互不包含。
            validator = g9_m90_cluster_dag_deepening_validator
        elif (
            f.name.startswith("g9_m91_page_format_v2_abi_")
            and g9_m91_page_format_v2_abi_validator is not None
        ):
            # G9.2 P0 硬门 M91 page_format_v2_abi（步骤 133，host+device 门）：
            # RXPL major=2 ABI id/version 与 v1 不同且冻结 + checked-in fixtures
            # encode→decode 往返无损 canonical records 与 golden 逐字节相等 +
            # M04 v1 页 ABI 0-byte 兼容（v1 消费路径回归 digest 不变）+ 篡改
            # digest 页 fail-closed（RED 臂）+ device 解码 digest 等于 CPU 解码
            # digest（RXS-0344；RFC-0022 §4.5）。device 腿必需，
            # RURIX_REQUIRE_REAL=1 下 SKIP 不充绿。
            # 前缀与 g9_vram_as_baseline_ / g9_m90_cluster_dag_deepening_ 互不包含。
            validator = g9_m91_page_format_v2_abi_validator
        elif (
            f.name.startswith("g9_m93_visible_cluster_set_")
            and g9_m93_visible_cluster_set_validator is not None
        ):
            # G9.3 P0 硬门 M93 visible_cluster_set（步骤 139，host 纯 host 门）：
            # selection cut 无重叠无空洞 + 父簇兜底/复原 + 空洞注入 RED +
            # 双跑 digest 逐位相等（RXS-0350；RFC-0022 §4.2）。
            # 前缀与 g9_m9x/g9_m1xx 全族互不包含。
            validator = g9_m93_visible_cluster_set_validator
        elif (
            f.name.startswith("g9_m94_clas_rt_convergence_")
            and g9_m94_clas_rt_convergence_validator is not None
        ):
            # G9.3 P0 硬门 M94 clas_rt_convergence（步骤 140，host+device 门）：
            # CLAS 主腿 vs BLAS 回退腿逐命中一致（容差 0）+ 错簇 RED +
            # 静态帧零 AS 构建 + validation=0（RXS-0351；RFC-0022 §4.3；U56）。
            validator = g9_m94_clas_rt_convergence_validator
        elif (
            f.name.startswith("g9_m95_single_source_truth_")
            and g9_m95_single_source_truth_validator is not None
        ):
            # G9.3 P0 硬门 M95 single_source_truth（步骤 141，host+device 门）：
            # 一份三喂 provenance + 旁路 variant RED + 蒙皮簇 VisBuffer
            # SW/HW diff=0（RXS-0352；RFC-0022 §4.4；R-G9-8）。
            validator = g9_m95_single_source_truth_validator
        elif (
            f.name.startswith("g9_m92_gpu_skinning_lod_update_")
            and g9_m92_gpu_skinning_lod_update_validator is not None
        ):
            # G9.3 P1 硬门 M92 gpu_skinning_lod_update（步骤 142，host+device 门）：
            # GPU 蒙皮 vs host 参照逐顶点一致 + 保守包围体包含 + 档位闭集
            # 确定性 + 静态帧零 AS 构建（RXS-0353；G9_CONTRACT §8.1 裁决①）。
            validator = g9_m92_gpu_skinning_lod_update_validator
        elif (
            f.name.startswith("g9_m105_command_build_node_")
            and g9_m105_command_build_node_validator is not None
        ):
            # G9.3 P1 硬门 M105 command_build_node（步骤 143，host+device 门）：
            # 全链路零 CPU 回读 + 构建产物逐字节一致 + RED 注入臂
            # （RXS-0354；RFC-0023 §4.4；复用 U54 lane）。
            validator = g9_m105_command_build_node_validator
        elif (
            f.name.startswith("g9_m106_execution_set_pso_")
            and g9_m106_execution_set_pso_validator is not None
        ):
            # G9.3 P1 硬门 M106 execution_set_pso（步骤 144，host+device 门）：
            # GPU 侧索引切换 vs CPU PSO 切换 vs 失效重建三 digest 全等 +
            # capability 缺失 fail-closed（RXS-0355；RFC-0023 §4.2；U57）。
            validator = g9_m106_execution_set_pso_validator
        elif (
            f.name.startswith("g9_m107_shader_library_ir_link_")
            and g9_m107_shader_library_ir_link_validator is not None
        ):
            # G9.3 P1 硬门 M107 shader_library_ir_link（步骤 145，host 纯 host 门）：
            # IR 链接 interface hash 确定性 + 链接 fail-closed RED 族 +
            # 变体预算超限硬失败 RED（RXS-0356；RFC-0023 §4.5/§4.6）。
            validator = g9_m107_shader_library_ir_link_validator
        elif (
            f.name.startswith("g9_m96_path_tracer_reference_")
            and g9_m96_path_tracer_reference_validator is not None
        ):
            # G9.4 P0 硬门 M96 path_tracer_reference（步骤 147，host+device 门）：
            # 固定 seed 双跑位级一致 + pbrt-v4 冻结容差带 + 三臂 RED + 起步范围
            # 冻结（RXS-0357；RFC-0022 §4.10；D2-Q7 门序源）。门 evidence（含
            # symbolic_gate_key 面）→ 门 schema；harness 直出件（无该面）→
            # g9_gi_harness 共享 schema。前缀与 g9_m9x/g9_m1xx 全族互不包含。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m96_path_tracer_reference_validator
            else:
                validator = g9_gi_harness_validator
        elif (
            f.name.startswith("g9_m97_surface_cache_")
            and g9_m97_surface_cache_validator is not None
        ):
            # G9.4 P0 硬门 M97 surface_cache（步骤 148，host+device 门）：Card
            # 参数化/RXPL v2 图集页/三深度产物 golden + 只丢能量不漏光 + 漏光
            # RED 臂 + M96 golden 深度带（RXS-0358；RFC-0022 §4.6；D2-Q7 门序
            # 机器阻断前置）。门/harness 直出件分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m97_surface_cache_validator
            else:
                validator = g9_gi_harness_validator
        elif (
            f.name.startswith("g9_m98_tracing_fallback_chain_")
            and g9_m98_tracing_fallback_chain_validator is not None
        ):
            # G9.4 P0 硬门 M98 tracing_fallback_chain（步骤 149，host+device 门）：
            # 四级计数逐帧非空 + 逐级强关可检测 + 禁静默回退 + L4 not-triggered
            # 登记（RXS-0359；RFC-0022 §4.7；D2-Q7 门序机器阻断前置）。分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m98_tracing_fallback_chain_validator
            else:
                validator = g9_gi_harness_validator
        elif (
            f.name.startswith("g9_m99_spg_radiance_cache_")
            and g9_m99_spg_radiance_cache_validator is not None
        ):
            # G9.4 P1 硬门 M99 spg_radiance_cache（步骤 150，host+device 门）：
            # 屏幕级 SPG 自适应细分 + Radiance Cache 双级 golden + 世界级
            # clipmap not-triggered 登记（RXS-0360；RFC-0022 §4.8；D2-Q7 前置）。
            # 分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m99_spg_radiance_cache_validator
            else:
                validator = g9_gi_harness_validator
        elif (
            f.name.startswith("g9_m100_multi_light_low_")
            and g9_m100_multi_light_low_validator is not None
        ):
            # G9.4 P1 硬门 M100 multi_light_low（步骤 151，host+device 门）：低档
            # 多灯默认档 golden + 验证射线零跳过契约 + ReSTIR not-triggered 登记
            # （RXS-0361；RFC-0022 §7；D2-Q7 前置）。分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m100_multi_light_low_validator
            else:
                validator = g9_gi_harness_validator
        elif (
            f.name.startswith("g9_m101_if_tier_ladder_")
            and g9_m101_if_tier_ladder_validator is not None
        ):
            # G9.4 P1 硬门 M101 if_tier_ladder（步骤 152，host+device 门）：四档
            # 共享内核单实例 + 每档 AS 预算行消费 AsStats + 超预算强制降档 +
            # SRGB 注入 RED（RXS-0362；RFC-0022 §4.8；D2-Q7 前置）。分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m101_if_tier_ladder_validator
            else:
                validator = g9_gi_harness_validator
        elif (
            f.name.startswith("g9_m110_world_partition_")
            and g9_m110_world_partition_validator is not None
        ):
            # G9.5 P0 硬门 M110 world_partition（步骤 154，host 纯 host 确定性门）：
            # 预算违约注入必排队降级 + soak hitch p99 ≤ g9_budget 实测阈值 +
            # cell 事件序列逐字 golden + HLOD 双构建位等（RXS-0363；RFC-0025 §4.A）。
            # 门 evidence（含 symbolic_gate_key 面）→ 门 schema；harness 直出件
            # （无该面）→ g9_world_harness 共享 schema。前缀与 g9_m1xx 全族互不包含。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m110_world_partition_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m118_display_pipeline_")
            and g9_m118_display_pipeline_view_transform_validator is not None
        ):
            # G9.5 P0 硬门 M118 display_pipeline_view_transform（步骤 155，host
            # 纯 host 确定性门）：四内置插件逐一 golden + 三交换链路径运行时切换 +
            # 非 HDR 携带 PQ 即 RED + HDR 标定 not-triggered 不假绿不否决 SDR 面
            # （RXS-0369；RFC-0025 §4.I）。门/harness 直出件分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m118_display_pipeline_view_transform_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m111_hlod_")
            and g9_m111_hlod_baking_validator is not None
        ):
            # G9.5 P1 硬门 M111 hlod_baking（步骤 156，host 纯 host 确定性门）：
            # 双构建 hash 相等 + 运行时零合并断言 + screen-size 互斥切换 golden
            # （RXS-0364；RFC-0025 §4.B）。门 evidence subject=g9_m111_hlod_baking
            # 与 harness 直出件 subject=g9_m111_hlod_runtime 共享前缀，按
            # symbolic_gate_key 有无分派；命名差（harness assertion_id=
            # g9.p1.m111.hlod_runtime vs 门 key=hlod_baking）如实登记不改写。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m111_hlod_baking_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m112_atmosphere_froxel_")
            and g9_m112_atmosphere_froxel_validator is not None
        ):
            # G9.5 P1 硬门 M112 atmosphere_froxel（步骤 157，host 纯 host 确定性门）：
            # Froxel 统一基础设施云雾共用 + 雾/云前端 golden + weather map 篡改
            # 签名拒录 RED + 预算字段逐帧非空（RXS-0365；RFC-0025 §4.C）。分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m112_atmosphere_froxel_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m113_water_")
            and g9_m113_water_dual_pipeline_validator is not None
        ):
            # G9.5 P1 硬门 M113 water_dual_pipeline（步骤 158，host 纯 host 确定性门）：
            # 大洋 IFFT/浅水波方程双管线几何互斥 + IFFT vs host DFT 对拍 +
            # 非法谱参数拒录 RED + 浮力预留不实现（RXS-0366；RFC-0025 §4.D）。分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m113_water_dual_pipeline_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m114_hair_")
            and g9_m114_hair_marschner_validator is not None
        ):
            # G9.5 P1 硬门 M114 hair_marschner（步骤 159，host 纯 host 确定性门）：
            # Marschner 三瓣逐瓣 golden + 能量守恒 + 股替换烘焙确定性 + strand 档
            # 强制精确 OIT 依赖 M120 数据不足 not-triggered 不充绿
            # （RXS-0372；RFC-0025 §4.E）。分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m114_hair_marschner_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m115_skin_")
            and g9_m115_skin_burley_diffusion_validator is not None
        ):
            # G9.5 P1 硬门 M115 skin_burley_diffusion（步骤 160，host 纯 host 确定性门）：
            # Burley 屏单 pass 双 kernel golden + profile 资产化 + 全零衰减退化
            # 纯漫反射 RED + 触 32B 经 RFC-0025 §4.L 修订行（RXS-0373；§4.F）。分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m115_skin_burley_diffusion_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m116_terrain_")
            and g9_m116_terrain_chunk_cell_validator is not None
        ):
            # G9.5 P1 硬门 M116 terrain_chunk_cell（步骤 161，host 纯 host 确定性门）：
            # chunk≡cell 禁第二套分格 + 全 compute LOD/剔除/缝合 + 零 SVT 依赖 +
            # LOD 差>1 注入裂缝 RED（RXS-0367；RFC-0025 §4.G）。分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m116_terrain_chunk_cell_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m117_decal_dbuffer_")
            and g9_m117_decal_dbuffer_validator is not None
        ):
            # G9.5 P1 硬门 M117 decal_dbuffer（步骤 162，host 纯 host 确定性门）：
            # DBuffer 三通道帧图占位 + cluster 化受界 + 两档语义等价 golden +
            # 超界注入降级 RED（RXS-0368；RFC-0025 §4.H）。分派同上。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m117_decal_dbuffer_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m119_post_")
            and g9_m119_post_processing_skeleton_validator is not None
        ):
            # G9.5 P1 硬门 M119 post_processing_skeleton（步骤 163，host 纯 host
            # 确定性门）：五级显式排序冻结 + 全程 HDR 线性域 + 隐式 clamp 注入 RED +
            # 曝光状态帧间持久（RXS-0370；RFC-0025 §4.J）。门 subject=
            # g9_m119_post_processing_skeleton 与 harness 直出件 g9_m119_post_chain
            # 共享前缀，按 symbolic_gate_key 有无分派；命名差如实登记。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m119_post_processing_skeleton_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m120_oit_benchmark")
            and g9_m120_oit_benchmark_harness_validator is not None
        ):
            # G9.5 P1 硬门 M120 oit_benchmark_harness（步骤 164，host 纯 host 确定性门）：
            # nvpro 七算法 × 4 档 evidence 非空 + 仅测量不定档 + 无数据选型判 RED +
            # 排序 fallback 永保留 + 精确档 diff=0（RXS-0371；RFC-0025 §4.K）。
            # 门 subject=g9_m120_oit_benchmark_harness 与 harness 直出件
            # g9_m120_oit_benchmark 共享前缀，按 symbolic_gate_key 有无分派。
            if isinstance(doc, dict) and "symbolic_gate_key" in doc:
                validator = g9_m120_oit_benchmark_harness_validator
            else:
                validator = g9_world_harness_validator
        elif (
            f.name.startswith("g9_m124_buoyancy_field_channel_")
            and g9_m124_buoyancy_field_channel_validator is not None
        ):
            # G9.6 P1 硬门 M124 buoyancy_field_channel（步骤 166，host 纯 host 确定性门）：
            # 解析浮力走 Field 通道 + 旁路 API 注入即 RED + 细长/翻滚 corpus fixture +
            # capture→replay 逐 tick hash + 变帧率逐位一致（RXS-0376；RFC-0024 §4.D）。
            # harness 直出件落 .tmp 工作区不进 evidence/；同短前缀直出件误入则
            # 落 gpu fallthrough 必红（fail-closed，evidence/ 只收门件）。
            validator = g9_m124_buoyancy_field_channel_validator
        elif (
            f.name.startswith("g9_m126_rapier_benchmark_ab_")
            and g9_m126_rapier_benchmark_ab_validator is not None
        ):
            # G9.6 P1 硬门 M126 rapier_benchmark_ab（步骤 167，host 纯 host 确定性门）：
            # 同场景同输入同 determinism 画像 A/B + measured 报告 + 基准不作 replay
            # oracle + RD-044 字面不变（RXS-0378；RFC-0024 §4.E2）。harness 直出件
            # 落 .tmp 工作区不进 evidence/；同短前缀直出件误入落 gpu fallthrough 必红。
            validator = g9_m126_rapier_benchmark_ab_validator
        elif (
            f.name.startswith("g9_m125_jolt_56_ab_evaluation_")
            and g9_m125_jolt_56_ab_evaluation_validator is not None
        ):
            # G9.6 P1 硬门 M125 jolt_56_ab_evaluation（步骤 168，host 纯 host 确定性门）：
            # RFC-0021 §4.A4 七步程序逐字 + 5.6 独立 vendor 并存不覆盖 5.3 基线 +
            # 新摩擦模型逐字段分类 + GPU compute 只评估不接权威 + 两臂诚实登记禁伪绿
            # （RXS-0377；RFC-0024 §4.E1）。harness 直出件落 .tmp 工作区不进 evidence/；
            # 同短前缀直出件误入落 gpu fallthrough 必红（fail-closed，evidence/ 只收门件）。
            validator = g9_m125_jolt_56_ab_evaluation_validator
        elif (
            f.name.startswith("g10_baseline_")
            and g10_baseline_validator is not None
        ):
            # G10.1 governance-only measured baseline（D-G10-5）→
            # milestones/g10/g10_baseline_evidence_schema.json：复用既有 harness 真跑
            # （sr_pipeline L3 1080p 帧墙钟 / d2h_pinned 读回带宽，BENCH_PROTOCOL §3
            # 50x3 协议）+ 会话环境画像随档；未锁频诚实边界经 clock_lock_note 存档，
            # 阈值 = 实测 ×1.5（min 向 ÷1.5）。results.trimmed_mean 供 g10_budget.json
            # 通用 measured entry 判读（ci/budget_eval.py eval_entry 通用路），零新
            # evaluator 分支。前缀与 g7_*/g8_*/g9_* 全族互不包含，置于 g9 族后安全。
            validator = g10_baseline_validator
        elif (
            f.name.startswith("uc05_engine_embed_v3")
            and uc05_engine_embed_v3_validator is not None
        ):
            validator = uc05_engine_embed_v3_validator
        elif (
            f.name.startswith("uc05_exec_face_gate")
            and uc05_exec_face_gate_validator is not None
        ):
            # G4.3 PR-E UC-05 执行面三项拦截门证据(G-G4-4;RFC-0015 §4.B / RXS-0280~0283)→
            # milestones/g4/uc05_exec_face_gate_evidence_schema.json(ci/uc05_exec_face_gate.py
            # 步骤 79 写:host 恒跑 alias_alloc + scheduler + rhi.rs exec_face 库单测 + uc05_corpus
            # 批跑 + --emit=check 编译档;device 段 gate real rx build const_capacity_graph.rx
            # EXE 真跑 + I10 measured 见证,SKIP=dev-env-degrade,RURIX_REQUIRE_REAL=1 翻硬红)。
            # exec_face_ok = device 段真跑 + I10 measured;i10_measured_local = I10 自 report_only
            # 升 measured_local(host 库测锚 + device EXE 双锚)。host_lib_tests 计入
            # g4.counter.exec_face_gate(ci/budget_eval.py,计数源 = 本证据族)。
            validator = uc05_exec_face_gate_validator
        elif (
            f.name.startswith("uc05_engine_embed")
            and uc05_engine_embed_validator is not None
        ):
            # EI1.4 UC-05 引擎嵌入证据(G-EI1-4;RFC-0014 §4.A+§4.B / RXS-0250~0255 + RXS-0261)→
            # milestones/ei1/uc05_engine_embed_evidence_schema.json(ci/uc05_engine_embed_smoke.py
            # 步骤 74 写:host 恒跑 生成头不手写审计 + 两制共存审计〔v1 手写路 RXS-0149 面 0-byte
            # 保留〕+ 零 .rs 审计,工具链档 --emit=dll GPU 导出面三件 + 生成头幂等 + 篡改再生成
            # byte-diff RED;device 段 cl.exe 编 engine_host v2 链 rurix_rhi.lib 真跑 + 三方数值
            # 对照,SKIP=dev-env-degrade,REQUIRE_REAL 翻硬红)。embed_ok=true 计入
            # ei1.counter.uc05_engine_embed(ci/budget_eval.py,计数源 = 本证据族)。
            validator = uc05_engine_embed_validator
        elif (
            f.name.startswith("uc05_check_")
            and uc05_check_bench_validator is not None
        ):
            # EI1.5 UC-05 全包 `--emit=check` 双口径采纳判据证据(G-EI1-5;RFC-0014 §4.B7 /
            # RXS-0265)→ milestones/ei1/uc05_check_bench_evidence_schema.json
            # (ci/uc05_check_bench.py 操作者工具写,**不进 CI 硬门** —— 计时波动,EA1 冷启动
            # 先例,SKIP 不充绿)。纯 host 编译器墙钟零 GPU:BENCH_PROTOCOL §2.1 锁频规程不适用,
            # 理由 required 落 environment.clock_lock_applicability;三次进程级独立 trial →
            # results.trimmed_mean 由 ei1_budget.json entries 经 ci/budget_eval.py **既有
            # eval_entry 通用路**判读(direction=max,阈 5000ms),零新 evaluator 分支。
            # `uncheckable_roots` 为 required 且逐项须带真实探测(probe.exit_code +
            # stderr_first_line),by-construction 封死「把不可 check 的包成员静默排除在
            # 『全包』口径外」的窗口(RXS-0265 诚实缺口纪律);步骤 75 ci/uc05_report_check.py
            # 另核该披露必须同步出现在 evidence/uc05_comparison_report.md 叙事面。
            validator = uc05_check_bench_validator
        else:
            validator = gpu_validator
        for v in validator.iter_errors(doc):
            err(f"evidence/{f.name}: {'/'.join(str(p) for p in v.path)}: {v.message}")


def main() -> int:
    check_deferred(ROOT / "registry/deferred.json")
    check_gating(ROOT / "registry/spike_gating.json")
    check_error_codes(ROOT / "registry/error_codes.json")
    for budget in sorted(ROOT.glob("milestones/*/*_budget.json")):
        check_budget(budget)
    check_evidence_files()
    if ERRORS:
        print("[check_schemas] FAIL")
        for e in ERRORS:
            print(f"  - {e}")
        return 1
    print("[check_schemas] PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
