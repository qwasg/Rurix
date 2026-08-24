<!-- Assisted-by: Cursor Agent（G18.1 治理波） -->
# G18 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（309–311）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 309 | g18.wave.1.acceptance_map | ci/g18_acceptance_map_check.py |
| 310 | g18.wave.1.candidate_decisions | ci/g18_candidate_decisions_check.py |
| 311 | g18.gov.implementation_interlock | ci/g18_interlock_check.py |

## 2. P0 九门（312–328）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 312 | g18.p0.m_a.rurix_light_transport_depth | ci/g18_rurix_light_transport_depth_smoke.py |
| 314 | g18.p0.m_b.presentation_pipeline_dual_profile | ci/g18_presentation_pipeline_dual_profile_smoke.py |
| 316 | g18.p0.m_c.ue_arm_lighting_repair_and_render | ci/g18_ue_arm_lighting_repair_and_render_smoke.py |
| 318 | g18.p0.m_d.dual_end_commercial_quality_verdict | ci/g18_dual_end_commercial_quality_verdict_smoke.py |
| 320 | g18.p0.m_e.sl_runtime_upgrade_disposition | ci/g18_sl_runtime_upgrade_disposition_smoke.py |
| 322 | g18.p0.m_f.fps_parity_reeval | ci/g18_fps_parity_reeval_smoke.py |
| 324 | g18.p0.m_g.virtualized_geometry_p3 | ci/g18_virtualized_geometry_p3_smoke.py |
| 326 | g18.p0.m_h.frame_generation_independent_layer | ci/g18_frame_generation_independent_layer_smoke.py |
| 328 | g18.p0.m_i.closed_gate_no_regression | ci/g18_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（313–329）

参数化脚本 `ci/g18_wave_exit_check.py`，gate key `g18.wave.{2..10}.exit`。

## 4. 收口三门（330–332）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 330 | g18.wave.8a.decisions | ci/g18_p2_decisions_check.py |
| 331 | g18.wave.8a.soak | ci/g18_stabilization_soak.py |
| 332 | g18.wave.9b.closeout | ci/g18_closeout_check.py |
